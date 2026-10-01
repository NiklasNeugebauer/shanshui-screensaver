use crate::raster::WORLD_H;
use crate::{world::World, Opts};
use std::collections::VecDeque;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::wayland::WindowAttributesExtWayland;
use winit::window::{Fullscreen, Window, WindowId};

const TILE: u32 = 512;
/// Tiles kept ready beyond the right edge. One tile is ~25 s of scrolling and
/// takes well under a second to make, so a small buffer is plenty.
const LOOKAHEAD: i64 = 2;
/// Tiles rasterized together on the first request, in parallel across cores.
const MAX_BATCH: i64 = 32;
/// How many past presents we remember content for, to skip redundant blits.
const RING: usize = 8;
/// marker for "this buffer holds nothing but paper"
const BLANK: i64 = i64::MIN;
/// marker for "what this buffer holds is unknown"; softbuffer keeps reporting
/// the old age after it reallocates on resize, so the ring has to be poisoned
/// by hand or we skip painting a freshly mapped (black) buffer
const UNKNOWN: i64 = i64::MAX;
const GRACE: Duration = Duration::from_millis(500);
const DEADZONE: f64 = 12.0;
const PAPER_FALLBACK: u32 = 0x00f3ecdd;

pub static T0: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn ms() -> f64 {
    T0.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

fn tracing() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("SHANSHUI_TRACE").is_some())
}

macro_rules! trace {
    ($($a:tt)*) => {
        if tracing() {
            eprintln!("[{:8.1}ms] {}", ms(), format!($($a)*));
        }
    };
}

fn install_signals() -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    for sig in [
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGINT,
        signal_hook::consts::SIGHUP,
    ] {
        let _ = signal_hook::flag::register(sig, flag.clone());
    }
    flag
}

struct Pane {
    window: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    w: u32,
    h: u32,
    scroll: f64,
    tiles: VecDeque<(i64, Vec<u32>)>,
    rx: Receiver<(i64, Vec<u32>)>,
    tx: Sender<i64>,
    want: i64,
    focused: bool,
    /// frame callbacks per 1 px step, derived from the monitor's refresh rate
    k: u32,
    /// nothing is presented before the compositor has sized the surface: the
    /// pre-configure default size would be committed and then reallocated
    /// underneath the open animation
    configured: bool,
    phase: u32,
    frames: u64,
    presents: u64,
    /// x0 of the content presented n frames ago, so a stale buffer that already
    /// holds the right pixels can be presented again without re-blitting
    content: [i64; RING],
    /// the first screenful is held back as plain paper rather than shown in pieces
    ready: bool,
    seed: String,
    zoom: Option<f64>,
    name: String,
    refresh: f64,
    last_present: Option<Instant>,
    intervals: Vec<f64>,
    ages: Vec<u32>,
    acq_us: Vec<f64>,
    commit_us: Vec<f64>,
    blit_us: Vec<f64>,
    present_us: Vec<f64>,
}

impl Pane {
    /// (Re)start generation for the current size and recompute the cadence.
    fn restart(&mut self, speed: f64, fps: u32) {
        let (tx_req, rx_req) = channel();
        let (tx_out, rx_out) = channel();
        let scale = self.zoom.unwrap_or(self.h as f64 / WORLD_H);
        spawn_worker(self.seed.clone(), scale, self.h, rx_req, tx_out);
        self.rx = rx_out;
        self.tx = tx_req;
        self.tiles.clear();
        self.want = -1;
        self.scroll = 0.0;
        self.ready = false;
        self.content = [UNKNOWN; RING];
        let speed_px = speed * self.window.scale_factor();
        self.k = cadence(self.refresh, speed_px, fps);
        // Offset each window's step cycle: both blit on 2 of every k frames, so
        // out-of-phase cycles cut how often they land in the same refresh.
        self.frames = self.phase as u64 % self.k.max(1) as u64;
        trace!(
            "{}: {}x{} @{:.3}Hz, {:.1} px/s requested -> 1 px every {} frames = {:.1} px/s",
            self.name,
            self.w,
            self.h,
            self.refresh,
            speed_px,
            self.k,
            self.refresh / self.k as f64
        );
        self.pump();
    }
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
        loop {
            while next > target {
                match rx_req.recv() {
                    Ok(t) => target = target.max(t),
                    Err(_) => return,
                }
            }
            while let Ok(t) = rx_req.try_recv() {
                target = target.max(t);
            }
            let end = target.min(next + MAX_BATCH - 1);
            let px0s: Vec<i64> = (next..=end).map(|i| i * TILE as i64).collect();
            let t = Instant::now();
            r.ensure(px0s[0], px0s[px0s.len() - 1] + TILE as i64);
            let gen = t.elapsed().as_secs_f64() * 1000.0;
            let t = Instant::now();
            let bufs =
                crate::raster::render_batch(&r.world, &r.paper, scale, TILE, height, &px0s);
            trace!(
                "tiles {next}..={end} ({}px): generate {gen:.0}ms, rasterize {:.0}ms",
                height,
                t.elapsed().as_secs_f64() * 1000.0
            );
            for (i, b) in (next..=end).zip(bufs) {
                if tx_out.send((i, b)).is_err() {
                    return;
                }
            }
            r.world.drop_before((next - 2).max(0) as f64 * TILE as f64 / scale - 1024.0);
            next = end + 1;
        }
    });
}

/// Frame callbacks per one-pixel step. Scrolling by a whole pixel on a fixed
/// number of refreshes is what makes the motion even; the effective speed is
/// snapped to `refresh / k` rather than the cadence being bent to the speed.
fn cadence(refresh_hz: f64, speed_px: f64, fps_cap: u32) -> u32 {
    let k_min = (refresh_hz / fps_cap.max(1) as f64).ceil().max(1.0);
    // ceil, not round: the effective speed is the fastest regular cadence that
    // does not exceed what was asked for, and a bigger k also means fewer blits.
    let k = (refresh_hz / speed_px.max(0.01)).ceil().max(1.0).max(k_min);
    k.min(600.0) as u32
}

impl Pane {
    fn pump(&mut self) {
        while let Ok((idx, buf)) = self.rx.try_recv() {
            self.tiles.push_back((idx, buf));
        }
        let x0 = self.scroll as i64;
        let first = x0.div_euclid(TILE as i64) - 1;
        while self.tiles.front().map(|t| t.0 < first).unwrap_or(false) {
            self.tiles.pop_front();
        }
        let want = (x0 + self.w as i64).div_euclid(TILE as i64) + LOOKAHEAD;
        if want > self.want {
            self.want = want;
            let _ = self.tx.send(want);
        }
        if !self.ready && covers(&self.tiles, x0, self.w as i64) {
            self.ready = true;
            trace!(
                "{} first screenful complete ({} tiles, after {} presents)",
                self.name,
                self.tiles.len(),
                self.presents
            );
        }
    }

    fn draw(&mut self, wakeup: Instant) {
        let (w, h) = (self.w as usize, self.h as usize);
        let want = if self.ready { self.scroll as i64 } else { BLANK };
        let t_acq = Instant::now();
        let mut sb = match self.surface.buffer_mut() {
            Ok(b) => b,
            Err(_) => return,
        };
        let acq = t_acq.elapsed();
        // The buffer we were handed may already hold exactly these pixels from
        // an earlier present; between steps that is the common case.
        let age = sb.age() as u64;
        let fresh = age > 0
            && age <= self.presents
            && self.content[((self.presents - age) % RING as u64) as usize] == want;
        let t_blit = Instant::now();
        if !fresh {
            if want == BLANK {
                sb.fill(PAPER_FALLBACK);
            } else {
                blit(&self.tiles, want, w, h, &mut sb);
            }
        }
        let blit_t = t_blit.elapsed();
        self.content[(self.presents % RING as u64) as usize] = want;
        self.presents += 1;
        let t_pre = Instant::now();
        // Tells winit a frame callback is wanted; without it RedrawRequested is
        // not throttled to the compositor's refresh and the loop free-runs.
        self.window.pre_present_notify();
        let _ = sb.present();
        let pre = t_pre.elapsed();
        let now = Instant::now();
        if let Some(prev) = self.last_present {
            self.intervals.push(now.duration_since(prev).as_secs_f64() * 1000.0);
        } else {
            trace!("{} first present (tiles={})", self.name, self.tiles.len());
        }
        self.last_present = Some(now);
        self.commit_us.push(now.duration_since(wakeup).as_secs_f64() * 1e6);
        self.acq_us.push(acq.as_secs_f64() * 1e6);
        if !fresh {
            self.blit_us.push(blit_t.as_secs_f64() * 1e6);
        }
        self.ages.push(age as u32);
        self.present_us.push(pre.as_secs_f64() * 1e6);
    }

    /// Advance on the frame callback, then present. Presenting on every
    /// callback is what keeps the surface in lockstep with the refresh; the
    /// blit above is skipped on the frames where nothing moved.
    fn tick(&mut self, speed: f64, fps: u32, wakeup: Instant) {
        if !self.configured {
            self.window.request_redraw();
            return;
        }
        self.frames += 1;
        // Hold off until the compositor has finished configuring the surface,
        // otherwise the first size is thrown away and generated twice.
        if self.want < 0 {
            if self.frames < 3 {
                self.draw(wakeup);
                self.window.request_redraw();
                return;
            }
            self.restart(speed, fps);
        }
        if self.ready && self.frames % self.k as u64 == 0 {
            self.scroll += 1.0;
        }
        self.pump();
        self.draw(wakeup);
        self.window.request_redraw();
    }

    fn stats(&self) {
        let pct = |v: &mut Vec<f64>, p: f64| -> f64 {
            if v.is_empty() {
                return 0.0;
            }
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v[((v.len() - 1) as f64 * p) as usize]
        };
        let mut iv = self.intervals.clone();
        let mut a = self.acq_us.clone();
        let mut b = self.blit_us.clone();
        let mut p = self.present_us.clone();
        let n = iv.len();
        let mean = if n > 0 { iv.iter().sum::<f64>() / n as f64 } else { 0.0 };
        let var = if n > 1 {
            iv.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64
        } else {
            0.0
        };
        eprintln!(
            "{}: {}x{} {} presents  interval mean {:.2}ms sd {:.2} p50 {:.2} p95 {:.2} max {:.2}",
            self.name,
            self.w,
            self.h,
            n,
            mean,
            var.sqrt(),
            pct(&mut iv, 0.5),
            pct(&mut iv, 0.95),
            pct(&mut iv, 1.0)
        );
        let mut c = self.commit_us.clone();
        eprintln!(
            "    wakeup->commit p50 {:.2}ms p95 {:.2}ms max {:.2}ms (budget {:.1}ms)",
            pct(&mut c, 0.5) / 1000.0,
            pct(&mut c, 0.95) / 1000.0,
            pct(&mut c, 1.0) / 1000.0,
            1000.0 / self.refresh
        );
        eprintln!(
            "    acquire p50 {:.0}us p95 {:.0}us | blit p50 {:.0}us p95 {:.0}us | present p50 {:.0}us p95 {:.0}us",
            pct(&mut a, 0.5),
            pct(&mut a, 0.95),
            pct(&mut b, 0.5),
            pct(&mut b, 0.95),
            pct(&mut p, 0.5),
            pct(&mut p, 0.95)
        );
        let mut hist = std::collections::BTreeMap::new();
        for v in &self.intervals {
            *hist.entry((v * 2.0).round() as i64).or_insert(0usize) += 1;
        }
        let mut top: Vec<_> = hist.into_iter().collect();
        top.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
        let line: Vec<String> = top
            .iter()
            .take(8)
            .map(|(b, c)| format!("{:.1}ms:{c}", *b as f64 / 2.0))
            .collect();
        eprintln!("    interval histogram: {}", line.join("  "));
        let mut ah = std::collections::BTreeMap::new();
        for a in &self.ages {
            *ah.entry(*a).or_insert(0usize) += 1;
        }
        eprintln!(
            "    buffer ages: {:?}  blits {}/{} presents ({:.0}%)",
            ah,
            self.blit_us.len(),
            self.presents,
            100.0 * self.blit_us.len() as f64 / self.presents.max(1) as f64
        );
    }
}

/// Do the ready tiles span the whole window at this offset, with no gaps?
fn covers(tiles: &VecDeque<(i64, Vec<u32>)>, x0: i64, w: i64) -> bool {
    let tw = TILE as i64;
    match (tiles.front(), tiles.back()) {
        (Some(f), Some(b)) => {
            f.0 * tw <= x0
                && (b.0 + 1) * tw >= x0 + w
                && (b.0 - f.0) as usize + 1 == tiles.len()
        }
        _ => false,
    }
}

/// Copy the visible slice of the ready tiles into the surface. This is a plain
/// row-wise memcpy and is memory-bound: splitting it across threads buys ~20%
/// wall time for 2-3x the CPU, so it deliberately stays on one thread.
fn blit(tiles: &VecDeque<(i64, Vec<u32>)>, x0: i64, w: usize, h: usize, dst: &mut [u32]) {
    if !covers(tiles, x0, w as i64) {
        dst.fill(PAPER_FALLBACK);
    }
    let tw = TILE as i64;
    for (idx, tile) in tiles {
        if tile.len() < TILE as usize * h {
            continue;
        }
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
/// Models the real cadence: one present per refresh, and two blits per 1 px
/// step because the compositor hands back double-buffered surfaces.
pub fn bench(opts: Opts, seed: String, secs: f64) {
    let (w, h) = (opts.width, opts.height);
    let scale = opts.zoom.unwrap_or(h as f64 / WORLD_H);
    let refresh = 60.0f64;
    let k = cadence(refresh, opts.speed, opts.fps);
    let (tx_req, rx_req) = channel();
    let (tx_out, rx_out) = channel();
    spawn_worker(seed, scale, h, rx_req, tx_out);
    let mut tiles: VecDeque<(i64, Vec<u32>)> = VecDeque::new();
    let mut dst = vec![0u32; (w * h) as usize];
    let frame = Duration::from_secs_f64(1.0 / refresh);
    let mut scroll = 0i64;
    let mut want = -1i64;
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
    let (mut frames, mut blits) = (0u64, 0u64);
    let mut blit_us: Vec<f64> = Vec::new();
    let mut pending = 0u32;
    while t0.elapsed().as_secs_f64() < secs {
        let next = Instant::now() + frame;
        frames += 1;
        if frames % k as u64 == 0 {
            scroll += 1;
            pending = 2;
        }
        while let Ok(t) = rx_out.try_recv() {
            tiles.push_back(t);
        }
        let first = scroll.div_euclid(TILE as i64) - 1;
        while tiles.front().map(|t| t.0 < first).unwrap_or(false) {
            tiles.pop_front();
        }
        let n = (scroll + w as i64).div_euclid(TILE as i64) + LOOKAHEAD;
        if n > want {
            want = n;
            let _ = tx_req.send(n);
        }
        if pending > 0 {
            pending -= 1;
            let t = Instant::now();
            blit(&tiles, scroll, w as usize, h as usize, &mut dst);
            blit_us.push(t.elapsed().as_secs_f64() * 1e6);
            blits += 1;
        }
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        }
    }
    let wall = t0.elapsed().as_secs_f64();
    let cpu = cpu_time() - c0;
    blit_us.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pct = |p: f64| blit_us.get(((blit_us.len().max(1) - 1) as f64 * p) as usize).copied().unwrap_or(0.0);
    println!(
        "{w}x{h} {refresh:.0}Hz, 1px every {k} frames = {:.1} px/s: {frames} presents, {blits} blits in {wall:.1}s",
        refresh / k as f64
    );
    println!(
        "  peak RSS {} MB, {} tiles held",
        peak_rss_mb(),
        tiles.len()
    );
    println!(
        "  blit wall p50 {:.2}ms p95 {:.2}ms max {:.2}ms | {cpu:.2}s CPU = {:.1}% of one core",
        pct(0.5) / 1000.0,
        pct(0.95) / 1000.0,
        pct(1.0) / 1000.0,
        100.0 * cpu / wall
    );
}

fn peak_rss_mb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmHWM:"))
                .and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse::<u64>().ok()))
        })
        .map(|kb| kb / 1024)
        .unwrap_or(0)
}

/// Process CPU time, read the way /proc exposes it so no libc binding is needed.
fn cpu_time() -> f64 {
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
    let tail = stat.rsplit(')').next().unwrap_or("");
    let f: Vec<&str> = tail.split_whitespace().collect();
    let ticks: f64 = f.get(11).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let sticks: f64 = f.get(12).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    (ticks + sticks) / 100.0
}

struct App {
    opts: Opts,
    seed: String,
    panes: Vec<Pane>,
    start: Instant,
    pointer: Option<(f64, f64)>,
    quitting: bool,
    ever_focused: bool,
    unfocused_since: Option<Instant>,
    signalled: Arc<AtomicBool>,
    /// start of the current event-loop iteration, i.e. roughly when the
    /// compositor's frame callback landed
    wakeup: Instant,
    /// dev harness: run this long ignoring input, then dump frame stats
    selftest: Option<Duration>,
}

impl App {
    fn quit(&mut self, el: &ActiveEventLoop, why: &str) {
        if self.selftest.is_some() && why != "selftest done" && why != "signal" {
            return;
        }
        if !self.quitting {
            if self.selftest.is_some() {
                for p in &self.panes {
                    p.stats();
                }
            }
            if std::env::var_os("SHANSHUI_DEBUG").is_some() {
                eprintln!("shanshui: exiting ({why})");
            }
            self.quitting = true;
            el.exit();
        }
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, _el: &ActiveEventLoop, _cause: winit::event::StartCause) {
        self.wakeup = Instant::now();
    }

    fn resumed(&mut self, el: &ActiveEventLoop) {
        if !self.panes.is_empty() {
            return;
        }
        let monitors: Vec<_> = el.available_monitors().collect();
        let count = if self.opts.windowed {
            std::env::var("SHANSHUI_WINDOWS").ok().and_then(|v| v.parse().ok()).unwrap_or(1)
        } else {
            monitors.len().max(1)
        };
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
            trace!(
                "window {i} ({}) inner_size {w}x{h} scale_factor {:.4} monitor {:?}",
                monitors.get(i).and_then(|m| m.name()).unwrap_or_default(),
                window.scale_factor(),
                monitors.get(i).map(|m| (m.size(), m.refresh_rate_millihertz()))
            );
            let context = softbuffer::Context::new(window.clone()).expect("softbuffer context");
            let mut surface =
                softbuffer::Surface::new(&context, window.clone()).expect("softbuffer surface");
            surface
                .resize(NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap())
                .expect("surface resize");
            let seed = format!("{}:{}", self.seed, i);
            // Left dangling on purpose: the real worker starts from configure(),
            // once the compositor has told us the final surface size.
            let (tx_req, rx_req) = channel();
            let (tx_out, rx_out) = channel();
            drop(rx_req);
            drop(tx_out);
            let refresh = monitors
                .get(i)
                .and_then(|m| m.refresh_rate_millihertz())
                .or_else(|| {
                    window.current_monitor().and_then(|m| m.refresh_rate_millihertz())
                })
                .map(|v| v as f64 / 1000.0)
                .unwrap_or(60.0);
            let pane = Pane {
                window,
                surface,
                w,
                h,
                scroll: 0.0,
                tiles: VecDeque::new(),
                rx: rx_out,
                tx: tx_req,
                want: -1,
                focused: false,
                k: 2,
                configured: false,
                phase: (i as u32).wrapping_mul(2),
                frames: 0,
                presents: 0,
                content: [UNKNOWN; RING],
                ready: false,
                seed,
                zoom: self.opts.zoom,
                name: monitors
                    .get(i)
                    .and_then(|m| m.name())
                    .unwrap_or_else(|| format!("window{i}")),
                refresh,
                last_present: None,
                intervals: Vec::new(),
                ages: Vec::new(),
                acq_us: Vec::new(),
                commit_us: Vec::new(),
                blit_us: Vec::new(),
                present_us: Vec::new(),
            };
            pane.window.request_redraw();
            self.panes.push(pane);
        }
        self.start = Instant::now();
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
                if std::env::var_os("SHANSHUI_DEBUG").is_some() {
                    eprintln!("shanshui: focus={f} at {:?}", self.start.elapsed());
                }
                for p in self.panes.iter_mut() {
                    if p.window.id() == id {
                        p.focused = f;
                    }
                }
                if f {
                    self.ever_focused = true;
                    self.unfocused_since = None;
                } else if self.unfocused_since.is_none() {
                    // a second monitor's window taking focus unfocuses the first,
                    // so settle before deciding the user came back
                    self.unfocused_since = Some(Instant::now());
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                trace!("scale factor changed -> {scale_factor:.4}");
            }
            WindowEvent::Resized(size) => {
                let (speed, fps) = (self.opts.speed, self.opts.fps);
                trace!(
                    "resized -> {}x{} (scale_factor now {:.4})",
                    size.width,
                    size.height,
                    self.panes
                        .iter()
                        .find(|p| p.window.id() == id)
                        .map(|p| p.window.scale_factor())
                        .unwrap_or(0.0)
                );
                for p in self.panes.iter_mut() {
                    if p.window.id() == id {
                        let h = size.height.max(1);
                        let changed = h != p.h && p.want >= 0;
                        let resized = p.w != size.width.max(1) || p.h != h;
                        p.w = size.width.max(1);
                        p.h = h;
                        let _ = p.surface.resize(
                            NonZeroU32::new(p.w).unwrap(),
                            NonZeroU32::new(p.h).unwrap(),
                        );
                        if resized || !p.configured {
                            // both surface buffers are reallocated lazily and
                            // come back unpainted, whatever age claims
                            p.content = [UNKNOWN; RING];
                        }
                        p.configured = true;
                        // tiles are full-window-height, so a height change
                        // invalidates every one of them
                        if changed {
                            p.restart(speed, fps);
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let (speed, fps) = (self.opts.speed, self.opts.fps);
                let wakeup = self.wakeup;
                for p in self.panes.iter_mut() {
                    if p.window.id() == id {
                        p.tick(speed, fps, wakeup);
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
        if self.signalled.load(Ordering::Relaxed) {
            self.quit(el, "signal");
            return;
        }
        if let Some(d) = self.selftest {
            if self.start.elapsed() > d {
                self.quit(el, "selftest done");
                return;
            }
        }
        if self.quitting {
            return;
        }
        if let Some(t) = self.unfocused_since {
            if self.ever_focused
                && self.start.elapsed() > GRACE
                && t.elapsed() > Duration::from_millis(200)
                && !self.panes.iter().any(|p| p.focused)
            {
                self.quit(el, "focus lost");
                return;
            }
        }
        // Everything is driven by frame callbacks; nothing to schedule here.
        el.set_control_flow(ControlFlow::Wait);
    }
}


pub fn run(mut opts: Opts, seed: String) {
    let signalled = install_signals();
    let el = match EventLoop::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("shanshui: no wayland display: {e}");
            std::process::exit(1);
        }
    };
    el.set_control_flow(ControlFlow::Poll);
    opts.fps = opts.fps.clamp(1, 240);
    let mut app = App {
        opts,
        seed,
        panes: Vec::new(),
        start: Instant::now(),
        pointer: None,
        quitting: false,
        ever_focused: false,
        unfocused_since: None,
        signalled,
        wakeup: Instant::now(),
        selftest: std::env::var("SHANSHUI_SELFTEST")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .map(Duration::from_secs_f64),
    };
    let _ = el.run_app(&mut app);
    std::process::exit(0);
}
