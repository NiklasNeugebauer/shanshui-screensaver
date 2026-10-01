mod arch;
mod draw;
mod man;
mod mount;
mod raster;
mod rng;
mod screensaver;
mod tree;
mod world;

use raster::{Raster, WORLD_H};
use world::World;

pub struct Opts {
    pub seed: Option<String>,
    pub speed: f64,
    pub fps: u32,
    pub zoom: Option<f64>,
    pub png: Option<String>,
    pub width: u32,
    pub height: u32,
    pub x: f64,
    pub windowed: bool,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            seed: None,
            speed: 24.0,
            fps: 30,
            zoom: None,
            png: None,
            width: 1920,
            height: 1080,
            x: 0.0,
            windowed: false,
        }
    }
}

const USAGE: &str = "\
shanshui - procedurally generated infinite landscape scroll

  --class <name>      ignored, present so pkill -f org.omarchy.screensaver matches
  --seed <string>     scene seed (default: current time)
  --speed <px/s>      scroll speed, default 24
  --fps <n>           frame cap, default 30
  --zoom <f>          pixels per world unit (default: fit window height)
  --windowed          do not request fullscreen
  --png <file>        render one frame to PNG and exit
  --width/--height    size for --png, default 1920x1080
  --x <px>            horizontal scroll offset for --png
  -h, --help          this text
";

fn parse_args() -> Option<Opts> {
    let mut o = Opts::default();
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut next = || it.next().unwrap_or_default();
        match a.as_str() {
            "--class" => {
                next();
            }
            "--seed" => o.seed = Some(next()),
            "--speed" => o.speed = next().parse().unwrap_or(o.speed),
            "--fps" => o.fps = next().parse().unwrap_or(o.fps),
            "--zoom" => o.zoom = next().parse().ok(),
            "--png" => o.png = Some(next()),
            "--width" => o.width = next().parse().unwrap_or(o.width),
            "--height" => o.height = next().parse().unwrap_or(o.height),
            "--x" => o.x = next().parse().unwrap_or(o.x),
            "--windowed" => o.windowed = true,
            "-h" | "--help" => {
                print!("{USAGE}");
                return None;
            }
            s if s.contains("org.omarchy.screensaver") => {}
            s => eprintln!("shanshui: ignoring unknown argument {s}"),
        }
    }
    Some(o)
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
    let o = match parse_args() {
        Some(o) => o,
        None => return,
    };
    let seed = o.seed.clone().unwrap_or_else(time_seed);
    if let Some(path) = &o.png {
        let mut r = make_raster(&seed, o.width, o.height, o.zoom);
        let t0 = std::time::Instant::now();
        let pm = r.render(o.x as i64);
        let dt = t0.elapsed();
        pm.to_owned().save_png(path).expect("cannot write png");
        eprintln!("rendered {}x{} in {:?} (seed {seed})", o.width, o.height, dt);
        return;
    }
    screensaver::run(o, seed);
}
