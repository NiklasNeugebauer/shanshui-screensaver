use crate::draw::Prim;
use crate::rng::Rt;
use crate::world::World;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform,
};

pub const PAPER: usize = 512;
/// World height mapped onto the window: the original's 800px window at 1.142 zoom.
pub const WORLD_H: f64 = 700.5253940455341;

pub fn paper_tex(seed: &str) -> Pixmap {
    let mut rt = Rt::new(seed);
    let mut pm = Pixmap::new(PAPER as u32, PAPER as u32).unwrap();
    let data = pm.pixels_mut();
    let r = PAPER;
    for i in 0..=r / 2 {
        for j in 0..=r / 2 {
            let mut c = 245.0 + rt.noise3(i as f64 * 0.1, j as f64 * 0.1, 0.0) * 10.0;
            c -= rt.rand() * 20.0;
            let px = tiny_skia::PremultipliedColorU8::from_rgba(
                c.round().clamp(0.0, 255.0) as u8,
                (c * 0.95).round().clamp(0.0, 255.0) as u8,
                (c * 0.85).round().clamp(0.0, 255.0) as u8,
                255,
            )
            .unwrap();
            for (a, b) in [(i, j), (r - i, j), (i, r - j), (r - i, r - j)] {
                if a < r && b < r {
                    data[b * r + a] = px;
                }
            }
        }
    }
    pm
}

/// Per-thread scratch for rasterizing one tile. Several of these share one
/// `World` so a batch of tiles can be rendered in parallel.
pub struct TileRaster {
    pub scale: f64,
    pub height: u32,
    pub width: u32,
    paper: Arc<Pixmap>,
    scene: Pixmap,
}

/// Owns the world and generates it; rasterizing goes through `TileRaster`.
pub struct Raster {
    pub world: World,
    pub paper: Arc<Pixmap>,
    pub scale: f64,
    pub width: u32,
    pub height: u32,
    tr: TileRaster,
}

impl Raster {
    pub fn new(world: World, scale: f64, width: u32, height: u32, paper_seed: &str) -> Raster {
        let paper = Arc::new(paper_tex(paper_seed));
        Raster {
            world,
            scale,
            width,
            height,
            tr: TileRaster::new(scale, width, height, paper.clone()),
            paper,
        }
    }

    /// Generate chunks so every tile in `px0..px1` can be rasterized.
    pub fn ensure(&mut self, px0: i64, px1: i64) {
        self.world.load(px0 as f64 / self.scale, px1 as f64 / self.scale + MARGIN);
    }

    pub fn render_u32(&mut self, px0: i64, dst: &mut Vec<u32>) {
        self.ensure(px0, px0 + self.width as i64);
        self.tr.render_u32(&self.world, px0, dst);
    }
}

/// How far past the visible range chunks must exist before a tile is final:
/// a single element can reach this far from its anchor.
const MARGIN: f64 = 1600.0;

impl TileRaster {
    pub fn new(scale: f64, width: u32, height: u32, paper: Arc<Pixmap>) -> TileRaster {
        TileRaster {
            scale,
            height,
            width,
            paper,
            scene: Pixmap::new(width, height).unwrap(),
        }
    }

    fn draw_prim(scene: &mut Pixmap, prim: &Prim, t: Transform) {
        if prim.pts.len() < 2 {
            return;
        }
        let mut pb = PathBuilder::with_capacity(prim.pts.len() + 1, prim.pts.len() + 1);
        let mut started = false;
        for q in &prim.pts {
            if !q[0].is_finite() || !q[1].is_finite() {
                return;
            }
            if started {
                pb.line_to(q[0] as f32, q[1] as f32);
            } else {
                pb.move_to(q[0] as f32, q[1] as f32);
                started = true;
            }
        }
        let path = match pb.finish() {
            Some(p) => p,
            None => return,
        };
        let mut paint = Paint { anti_alias: true, ..Default::default() };
        if prim.fil.a > 0.0 {
            paint.set_color(Color::from_rgba8(
                prim.fil.r.round().clamp(0.0, 255.0) as u8,
                prim.fil.g.round().clamp(0.0, 255.0) as u8,
                prim.fil.b.round().clamp(0.0, 255.0) as u8,
                (prim.fil.a * 255.0).round().clamp(0.0, 255.0) as u8,
            ));
            scene.fill_path(&path, &paint, FillRule::Winding, t, None);
        }
        if prim.wid > 0.0 && prim.str_.a > 0.0 {
            paint.set_color(Color::from_rgba8(
                prim.str_.r.round().clamp(0.0, 255.0) as u8,
                prim.str_.g.round().clamp(0.0, 255.0) as u8,
                prim.str_.b.round().clamp(0.0, 255.0) as u8,
                (prim.str_.a * 255.0).round().clamp(0.0, 255.0) as u8,
            ));
            let stroke = Stroke {
                width: prim.wid as f32,
                line_cap: LineCap::Butt,
                line_join: LineJoin::Miter,
                miter_limit: 4.0,
                ..Default::default()
            };
            scene.stroke_path(&path, &paint, &stroke, t, None);
        }
    }

    /// Draw the ink layer for the pixel column range [px0, px0+width).
    fn draw_scene(&mut self, world: &World, px0: i64) {
        let xw0 = px0 as f64 / self.scale;
        let xw1 = (px0 + self.width as i64) as f64 / self.scale;
        self.scene.fill(Color::TRANSPARENT);
        let t = Transform::from_row(
            self.scale as f32,
            0.0,
            0.0,
            self.scale as f32,
            -(px0 as f32),
            0.0,
        );
        for ch in &world.chunks {
            if ch.x1 < xw0 || ch.x0 > xw1 {
                continue;
            }
            for prim in &ch.prims {
                Self::draw_prim(&mut self.scene, prim, t);
            }
        }
    }

    /// Multiply the ink layer onto the paper straight into the output words.
    /// Doing it here rather than through a second pixmap saves a full-size
    /// buffer per rasterizer and one composite pass per tile.
    pub fn render_u32(&mut self, world: &World, px0: i64, dst: &mut Vec<u32>) {
        self.draw_scene(world, px0);
        let (w, h) = (self.width as usize, self.height as usize);
        dst.clear();
        dst.resize(w * h, 0);
        let src = self.scene.pixels();
        let paper = self.paper.pixels();
        let xo = px0.rem_euclid(PAPER as i64) as usize;
        for y in 0..h {
            let prow = (y & (PAPER - 1)) * PAPER;
            let srow = y * w;
            for x in 0..w {
                let p = paper[prow + ((x + xo) & (PAPER - 1))];
                let s = src[srow + x];
                // multiply with a premultiplied source: Cr = Cb * (1 - a + Cs)
                let a = 255 - s.alpha() as u32;
                let m = |cb: u8, cs: u8| {
                    let t = cb as u32 * (a + cs as u32) + 128;
                    ((t + (t >> 8)) >> 8) & 0xff
                };
                dst[srow + x] =
                    (m(p.red(), s.red()) << 16) | (m(p.green(), s.green()) << 8) | m(p.blue(), s.blue());
            }
        }
    }
}

/// Rasterize several tiles at once, one `TileRaster` per thread over a shared
/// world. Tiles are independent once their chunks exist, and the machine is
/// otherwise idle while the screensaver runs.
pub fn render_batch(
    world: &World,
    paper: &Arc<Pixmap>,
    scale: f64,
    tile_w: u32,
    height: u32,
    px0s: &[i64],
) -> Vec<Vec<u32>> {
    let n = px0s.len();
    if n == 1 {
        let mut tr = TileRaster::new(scale, tile_w, height, paper.clone());
        let mut buf = Vec::new();
        tr.render_u32(world, px0s[0], &mut buf);
        return vec![buf];
    }
    let slots: Vec<Mutex<Vec<u32>>> = (0..n).map(|_| Mutex::new(Vec::new())).collect();
    let cursor = AtomicUsize::new(0);
    // Each thread keeps its own full-height scratch pixmap, so the width of the
    // batch is capped to keep the startup burst's footprint bounded.
    let threads =
        n.min(std::thread::available_parallelism().map(|v| v.get()).unwrap_or(4)).min(6);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let mut tr = TileRaster::new(scale, tile_w, height, paper.clone());
                loop {
                    let i = cursor.fetch_add(1, Ordering::Relaxed);
                    if i >= n {
                        break;
                    }
                    let mut buf = Vec::new();
                    tr.render_u32(world, px0s[i], &mut buf);
                    *slots[i].lock().unwrap() = buf;
                }
            });
        }
    });
    slots.into_iter().map(|m| m.into_inner().unwrap()).collect()
}
