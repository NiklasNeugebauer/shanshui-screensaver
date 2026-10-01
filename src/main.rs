#![allow(
    clippy::collapsible_match,
    clippy::manual_div_ceil,
    clippy::manual_is_multiple_of,
    clippy::needless_range_loop,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::too_many_arguments
)]

mod arch;
mod draw;
mod man;
mod mount;
mod noise;
mod raster;
mod rng;
mod screensaver;
#[cfg(test)]
mod tests;
mod tree;
mod world;

use clap::Parser;

use raster::{Raster, WORLD_H};
use world::World;

/// Procedurally generated infinite landscape scroll, as a Wayland screensaver.
#[derive(Parser, Debug, Clone)]
#[command(name = "shanshui", version, about, long_about = None)]
pub struct Opts {
    /// Wayland app-id for compositor window rules (Omarchy keys its screensaver rules off the default)
    #[arg(long, default_value = "org.omarchy.screensaver")]
    pub class: String,

    /// Scene seed; the same seed reproduces the same landscape
    #[arg(long)]
    pub seed: Option<String>,

    /// Scroll speed in logical pixels per second
    #[arg(long, default_value_t = 24.0)]
    pub speed: f64,

    /// Frame cap
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u32).range(1..=240))]
    pub fps: u32,

    /// Pixels per world unit (default: fit the window height)
    #[arg(long)]
    pub zoom: Option<f64>,

    /// Seconds to fade the scene in over the paper on the first frame; 0 disables
    #[arg(long, default_value_t = 1.5, value_name = "SECS")]
    pub fade: f64,

    /// Open a single normal window instead of one fullscreen window per monitor
    #[arg(long)]
    pub windowed: bool,

    /// Render one frame to this PNG file and exit
    #[arg(long, value_name = "FILE")]
    pub png: Option<String>,

    /// Run the pipeline headlessly for this many seconds and report CPU use
    #[arg(long, value_name = "SECS")]
    pub bench: Option<f64>,

    /// Frame width for --png and --bench
    #[arg(long, default_value_t = 1920)]
    pub width: u32,

    /// Frame height for --png and --bench
    #[arg(long, default_value_t = 1080)]
    pub height: u32,

    /// Horizontal scroll offset in pixels for --png and --bench
    #[arg(long, default_value_t = 0.0)]
    pub x: f64,
}

pub fn time_seed() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_else(|_| "1234".into())
}

pub fn make_raster(seed: &str, width: u32, height: u32, zoom: Option<f64>) -> Raster {
    rng::seed(seed);
    let scale = zoom.unwrap_or(height as f64 / WORLD_H);
    Raster::new(World::new(), scale, width, height, seed)
}

fn main() {
    let o = Opts::parse();
    let seed = o.seed.clone().unwrap_or_else(time_seed);
    if let Some(path) = &o.png {
        let mut r = make_raster(&seed, o.width, o.height, o.zoom);
        let t0 = std::time::Instant::now();
        let mut buf = Vec::new();
        r.render_u32(o.x as i64, &mut buf);
        let dt = t0.elapsed();
        let mut pm = tiny_skia::Pixmap::new(o.width, o.height).unwrap();
        for (px, v) in pm.pixels_mut().iter_mut().zip(&buf) {
            *px = tiny_skia::PremultipliedColorU8::from_rgba(
                (v >> 16) as u8,
                (v >> 8) as u8,
                *v as u8,
                255,
            )
            .unwrap();
        }
        pm.save_png(path).expect("cannot write png");
        eprintln!("rendered {}x{} in {:?} (seed {seed})", o.width, o.height, dt);
        return;
    }
    if let Some(secs) = o.bench {
        screensaver::bench(o, seed, secs);
        return;
    }
    screensaver::run(o, seed);
}
