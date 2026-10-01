use crate::draw::Prim;
use crate::rng::Rt;
use crate::world::World;
use tiny_skia::{
    BlendMode, Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, PixmapPaint,
    PixmapRef, Stroke, Transform,
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

pub struct Raster {
    pub world: World,
    pub scale: f64,
    pub height: u32,
    paper: Pixmap,
    scene: Pixmap,
    out: Pixmap,
    width: u32,
}

impl Raster {
    pub fn new(world: World, scale: f64, width: u32, height: u32, paper_seed: &str) -> Raster {
        Raster {
            world,
            scale,
            height,
            width,
            paper: paper_tex(paper_seed),
            scene: Pixmap::new(width, height).unwrap(),
            out: Pixmap::new(width, height).unwrap(),
        }
    }

    fn fill_paper(&mut self, px0: i64) {
        let paint = PixmapPaint::default();
        let p = PAPER as i64;
        let start = px0.div_euclid(p) * p;
        let mut x = start;
        while x < px0 + self.width as i64 {
            let mut y = 0i64;
            while y < self.height as i64 {
                self.out.draw_pixmap(
                    (x - px0) as i32,
                    y as i32,
                    self.paper.as_ref(),
                    &paint,
                    Transform::identity(),
                    None,
                );
                y += p;
            }
            x += p;
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

    /// Render the pixel column range [px0, px0+width) of the infinite scroll.
    pub fn render(&mut self, px0: i64) -> PixmapRef<'_> {
        let xw0 = px0 as f64 / self.scale;
        let xw1 = (px0 + self.width as i64) as f64 / self.scale;
        self.world.load(xw1 + 1600.0);
        self.scene.fill(Color::TRANSPARENT);
        let t = Transform::from_row(
            self.scale as f32,
            0.0,
            0.0,
            self.scale as f32,
            -(px0 as f32),
            0.0,
        );
        for ch in &self.world.chunks {
            if ch.x1 < xw0 || ch.x0 > xw1 {
                continue;
            }
            for prim in &ch.prims {
                Self::draw_prim(&mut self.scene, prim, t);
            }
        }
        self.fill_paper(px0);
        self.out.draw_pixmap(
            0,
            0,
            self.scene.as_ref(),
            &PixmapPaint { blend_mode: BlendMode::Multiply, ..Default::default() },
            Transform::identity(),
            None,
        );
        self.out.as_ref()
    }

    pub fn render_u32(&mut self, px0: i64, dst: &mut Vec<u32>) {
        let pm = self.render(px0);
        dst.clear();
        dst.reserve(pm.pixels().len());
        for p in pm.pixels() {
            let d = p.demultiply();
            dst.push(((d.red() as u32) << 16) | ((d.green() as u32) << 8) | d.blue() as u32);
        }
    }
}
