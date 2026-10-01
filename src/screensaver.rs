use crate::raster::WORLD_H;
use crate::{world::World, Opts};
use std::collections::VecDeque;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::wayland::WindowAttributesExtWayland;
use winit::window::{Fullscreen, Window, WindowId};

const TILE: u32 = 512;
const LOOKAHEAD: i64 = 5;
const GRACE: Duration = Duration::from_millis(500);
const DEADZONE: f64 = 12.0;
const PAPER_FALLBACK: u32 = 0x00f3ecdd;

static SIGNALLED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_: libc::c_int) {
    SIGNALLED.store(true, Ordering::SeqCst);
}

fn install_signals() {
    unsafe {
        libc::signal(libc::SIGTERM, on_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, on_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGHUP, on_signal as *const () as libc::sighandler_t);
    }
}

struct Pane {
    window: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    w: u32,
    h: u32,
    scroll: f64,
    speed: f64,
    tiles: VecDeque<(i64, Vec<u32>)>,
    rx: Receiver<(i64, Vec<u32>)>,
    tx: Sender<i64>,
    want: i64,
    drawn: i64,
    focused: bool,
}

fn spawn_worker(
    seed: String,
    scale: f64,
    height: u32,
    rx_req: Receiver<i64>,
    tx_out: Sender<(i64, Vec<u32>)>,
) {
    std::thread::spawn(move || {
        let mut r = crate::raster::Raster::new(World::new(), scale, TILE, height, &seed);
        crate::rng::seed(&seed);
        let mut next: i64 = 0;
        let mut target: i64 = 0;
        let mut buf: Vec<u32> = Vec::new();
        loop {
            while next > target {
                match rx_req.recv() {
                    Ok(t) => target = t,
                    Err(_) => return,
                }
            }
            while let Ok(t) = rx_req.try_recv() {
                target = target.max(t);
            }
            r.render_u32(next * TILE as i64, &mut buf);
            if tx_out.send((next, std::mem::take(&mut buf))).is_err() {
                return;
            }
            if next % 4 == 0 {
                let keep = (next - 2) * TILE as i64;
                r.world.drop_before(keep as f64 / scale - 1024.0);
            }
            next += 1;
        }
    });
}

impl Pane {
    fn pump(&mut self) {
        while let Ok((idx, buf)) = self.rx.try_recv() {
            self.tiles.push_back((idx, buf));
        }
        let first = (self.scroll as i64).div_euclid(TILE as i64) - 1;
        while self.tiles.front().map(|t| t.0 < first).unwrap_or(false) {
            self.tiles.pop_front();
        }
        let want = (self.scroll as i64 + self.w as i64).div_euclid(TILE as i64) + LOOKAHEAD;
        if want > self.want {
            self.want = want;
            let _ = self.tx.send(want);
        }
    }

    fn draw(&mut self) {
        let (w, h) = (self.w as usize, self.h as usize);
        let x0 = self.scroll.round() as i64;
        let mut sb = match self.surface.buffer_mut() {
            Ok(b) => b,
            Err(_) => return,
        };
        blit(&self.tiles, x0, w, h, &mut sb);
        let _ = sb.present();
    }
}

fn blit(tiles: &VecDeque<(i64, Vec<u32>)>, x0: i64, w: usize, h: usize, dst: &mut [u32]) {
    let tw = TILE as i64;
    // Only clear when the ready tiles do not span the whole window (startup, or
    // when generation falls behind); the common path overwrites every pixel.
    let covered = match (tiles.front(), tiles.back()) {
        (Some(f), Some(b)) => {
            f.0 * tw <= x0
                && (b.0 + 1) * tw >= x0 + w as i64
                && (b.0 - f.0) as usize + 1 == tiles.len()
        }
        _ => false,
    };
    if !covered {
        dst.fill(PAPER_FALLBACK);
    }
    for (idx, tile) in tiles {
        let t0 = idx * tw;
        let a = t0.max(x0);
        let b = (t0 + tw).min(x0 + w as i64);
        if a >= b {
            continue;
        }
        let dst_off = (a - x0) as usize;
        let src_off = (a - t0) as usize;
        let n = (b - a) as usize;
        for y in 0..h {
            let d = y * w + dst_off;
            let s = y * TILE as usize + src_off;
            dst[d..d + n].copy_from_slice(&tile[s..s + n]);
        }
    }
}

/// Run the generator + blit pipeline without a compositor and report CPU use.
pub fn bench(opts: Opts, seed: String, secs: f64) {
    let (w, h) = (opts.width, opts.height);
    let scale = opts.zoom.unwrap_or(h as f64 / WORLD_H);
    let (tx_req, rx_req) = channel();
    let (tx_out, rx_out) = channel();
    spawn_worker(seed, scale, h, rx_req, tx_out);
    let mut tiles: VecDeque<(i64, Vec<u32>)> = VecDeque::new();
    let mut dst = vec![0u32; (w * h) as usize];
    let frame = Duration::from_secs_f64(1.0 / opts.fps.clamp(1, 240) as f64);
    let mut scroll = 0.0f64;
    let mut want = -1i64;
    let mut drawn = i64::MIN;
    // let the worker fill the first screenful before timing
    let warm = (w as i64).div_euclid(TILE as i64) + LOOKAHEAD + 1;
    let _ = tx_req.send(warm);
    while tiles.len() < warm as usize {
        match rx_out.recv() {
            Ok(t) => tiles.push_back(t),
            Err(_) => return,
        }
    }
    let t0 = Instant::now();
    let c0 = cpu_time();
    let mut frames = 0u64;
    while t0.elapsed().as_secs_f64() < secs {
        let next = Instant::now() + frame;
        scroll += opts.speed * frame.as_secs_f64();
        while let Ok(t) = rx_out.try_recv() {
            tiles.push_back(t);
        }
        let first = (scroll as i64).div_euclid(TILE as i64) - 1;
        while tiles.front().map(|t| t.0 < first).unwrap_or(false) {
            tiles.pop_front();
        }
        let n = (scroll as i64 + w as i64).div_euclid(TILE as i64) + LOOKAHEAD;
        if n > want {
            want = n;
            let _ = tx_req.send(n);
        }
        let x = scroll.round() as i64;
        if x != drawn {
            drawn = x;
            blit(&tiles, x, w as usize, h as usize, &mut dst);
            frames += 1;
        }
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        }
    }
    let wall = t0.elapsed().as_secs_f64();
    let cpu = cpu_time() - c0;
    println!(
        "{w}x{h} @{}fps, {frames} frames in {wall:.1}s: {cpu:.2}s CPU = {:.1}% of one core",
        opts.fps,
        100.0 * cpu / wall
    );
}

fn cpu_time() -> f64 {
    unsafe {
        let mut u: libc::rusage = std::mem::zeroed();
        libc::getrusage(libc::RUSAGE_SELF, &mut u);
        u.ru_utime.tv_sec as f64
            + u.ru_utime.tv_usec as f64 * 1e-6
            + u.ru_stime.tv_sec as f64
            + u.ru_stime.tv_usec as f64 * 1e-6
    }
}

struct App {
    opts: Opts,
    seed: String,
    panes: Vec<Pane>,
    start: Instant,
    last: Instant,
    frame: Duration,
    pointer: Option<(f64, f64)>,
    quitting: bool,
}

impl App {
    fn quit(&mut self, el: &ActiveEventLoop, why: &str) {
        if !self.quitting {
            if std::env::var_os("SHANSHUI_DEBUG").is_some() {
                eprintln!("shanshui: exiting ({why})");
            }
            self.quitting = true;
            el.exit();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if !self.panes.is_empty() {
            return;
        }
        let monitors: Vec<_> = el.available_monitors().collect();
        let count = if self.opts.windowed { 1 } else { monitors.len().max(1) };
        for i in 0..count {
            let mut attrs = Window::default_attributes()
                .with_title("shanshui")
                .with_name(self.opts.class.clone(), self.opts.class.clone())
                .with_decorations(false);
            if self.opts.windowed {
                attrs = attrs
                    .with_inner_size(winit::dpi::LogicalSize::new(960.0, 540.0));
            } else {
                attrs = attrs
                    .with_fullscreen(Some(Fullscreen::Borderless(monitors.get(i).cloned())));
            }
            let window = match el.create_window(attrs) {
                Ok(w) => Rc::new(w),
                Err(e) => {
                    eprintln!("shanshui: cannot create window: {e}");
                    el.exit();
                    return;
                }
            };
            window.set_cursor_visible(false);
            let size = window.inner_size();
            let (w, h) = (size.width.max(1), size.height.max(1));
            let context = softbuffer::Context::new(window.clone()).expect("softbuffer context");
            let mut surface =
                softbuffer::Surface::new(&context, window.clone()).expect("softbuffer surface");
            surface
                .resize(NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap())
                .expect("surface resize");
            let scale = self.opts.zoom.unwrap_or(h as f64 / WORLD_H);
            let seed = format!("{}:{}", self.seed, i);
            let (tx_req, rx_req) = channel();
            let (tx_out, rx_out) = channel();
            spawn_worker(seed, scale, h, rx_req, tx_out);
            let mut pane = Pane {
                window,
                surface,
                w,
                h,
                scroll: 0.0,
                speed: self.opts.speed * window_scale(&monitors, i),
                tiles: VecDeque::new(),
                rx: rx_out,
                tx: tx_req,
                want: -1,
                drawn: i64::MIN,
                focused: false,
            };
            pane.pump();
            self.panes.push(pane);
        }
        self.start = Instant::now();
        self.last = Instant::now();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let armed = self.start.elapsed() > GRACE;
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => self.quit(el, "closed"),
            WindowEvent::KeyboardInput { event, .. } => {
                if armed && event.state == ElementState::Pressed {
                    self.quit(el, "key");
                }
            }
            WindowEvent::MouseInput { .. } | WindowEvent::MouseWheel { .. } => {
                if armed {
                    self.quit(el, "mouse");
                }
            }
            WindowEvent::Touch(_) => {
                if armed {
                    self.quit(el, "touch");
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if armed {
                    match self.pointer {
                        None => self.pointer = Some((position.x, position.y)),
                        Some((x, y)) => {
                            if (position.x - x).abs() > DEADZONE
                                || (position.y - y).abs() > DEADZONE
                            {
                                self.quit(el, "cursor moved");
                            }
                        }
                    }
                }
            }
            WindowEvent::Focused(f) => {
                let mut lost = false;
                for p in self.panes.iter_mut() {
                    if p.window.id() == id {
                        if f {
                            p.focused = true;
                        } else if p.focused {
                            lost = true;
                        }
                    }
                }
                if lost && armed {
                    self.quit(el, "focus lost");
                }
            }
            WindowEvent::Resized(size) => {
                for p in self.panes.iter_mut() {
                    if p.window.id() == id {
                        p.w = size.width.max(1);
                        p.h = size.height.max(1);
                        let _ = p.surface.resize(
                            NonZeroU32::new(p.w).unwrap(),
                            NonZeroU32::new(p.h).unwrap(),
                        );
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                for p in self.panes.iter_mut() {
                    if p.window.id() == id {
                        p.draw();
                    }
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, el: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if self.start.elapsed() < GRACE {
            return;
        }
        match event {
            DeviceEvent::Key(_) | DeviceEvent::Button { .. } => self.quit(el, "device input"),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if SIGNALLED.load(Ordering::SeqCst) {
            self.quit(el, "signal");
            return;
        }
        if self.quitting {
            return;
        }
        let now = Instant::now();
        let dt = now.duration_since(self.last);
        if dt >= self.frame {
            self.last = now;
            let secs = dt.as_secs_f64().min(0.25);
            for p in self.panes.iter_mut() {
                p.scroll += p.speed * secs;
                p.pump();
                let x = p.scroll.round() as i64;
                if x != p.drawn {
                    p.drawn = x;
                    p.window.request_redraw();
                }
            }
        }
        el.set_control_flow(ControlFlow::WaitUntil(self.last + self.frame));
    }
}

fn window_scale(monitors: &[winit::monitor::MonitorHandle], i: usize) -> f64 {
    monitors.get(i).map(|m| m.scale_factor()).unwrap_or(1.0)
}

pub fn run(opts: Opts, seed: String) {
    install_signals();
    let el = match EventLoop::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("shanshui: no wayland display: {e}");
            std::process::exit(1);
        }
    };
    el.set_control_flow(ControlFlow::Poll);
    let fps = opts.fps.clamp(1, 240);
    let mut app = App {
        opts,
        seed,
        panes: Vec::new(),
        start: Instant::now(),
        last: Instant::now(),
        frame: Duration::from_secs_f64(1.0 / fps as f64),
        pointer: None,
        quitting: false,
    };
    let _ = el.run_app(&mut app);
    std::process::exit(0);
}
