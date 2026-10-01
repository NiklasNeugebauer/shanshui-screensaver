// Port of shan-shui-inf's Prng (Blum-Blum-Shub evaluated in f64, quirks included),
// keeping the exact call order of the original. The Perlin noise it feeds lives
// in noise.rs under the LGPL.

use std::cell::RefCell;

const P: f64 = 999979.0;
const Q: f64 = 999983.0;

pub struct Prng {
    s: f64,
    m: f64,
}

const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn btoa(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity((b.len() + 2) / 3 * 4);
    for c in b.chunks(3) {
        let (b0, b1, b2) = (c[0] as u32, *c.get(1).unwrap_or(&0) as u32, *c.get(2).unwrap_or(&0) as u32);
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

// JSON.stringify of a JS string
fn json_str(x: &str) -> String {
    let mut o = String::with_capacity(x.len() + 2);
    o.push('"');
    for c in x.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            _ => o.push(c),
        }
    }
    o.push('"');
    o
}

fn hash(x: &str) -> f64 {
    let y = btoa(&json_str(x));
    let mut z = 0.0f64;
    for (i, c) in y.chars().enumerate() {
        z += (c as u32 as f64) * 128f64.powi(i as i32);
    }
    z
}

impl Prng {
    pub fn new(seed: &str) -> Prng {
        let m = P * Q;
        let mut y = 0.0f64;
        let mut z = 0.0f64;
        let h = hash(seed);
        let mut guard = 0;
        while y % P == 0.0 || y % Q == 0.0 || y == 0.0 || y == 1.0 || !y.is_finite() {
            y = (h + z) % m;
            z += 1.0;
            guard += 1;
            if guard > 1000 {
                y = 1234.0;
                break;
            }
        }
        let mut p = Prng { s: y, m };
        for _ in 0..10 {
            p.next();
        }
        p
    }
    #[inline]
    pub fn next(&mut self) -> f64 {
        self.s = (self.s * self.s) % self.m;
        self.s / self.m
    }
}

use crate::noise::PERLIN_SIZE;

pub struct Rt {
    pub prng: Prng,
    perlin: Option<Vec<f64>>,
}

impl Rt {
    pub fn new(seed: &str) -> Rt {
        Rt { prng: Prng::new(seed), perlin: None }
    }

    #[inline]
    pub fn rand(&mut self) -> f64 {
        self.prng.next()
    }

    pub fn noise3(&mut self, x: f64, y: f64, z: f64) -> f64 {
        if self.perlin.is_none() {
            let mut v = Vec::with_capacity(PERLIN_SIZE as usize + 1);
            for _ in 0..PERLIN_SIZE + 1 {
                v.push(self.prng.next());
            }
            self.perlin = Some(v);
        }
        crate::noise::noise3(self.perlin.as_ref().unwrap(), x, y, z)
    }

}

#[inline]
pub fn jfloor(x: f64) -> f64 {
    if x.is_finite() {
        x.floor()
    } else {
        0.0
    }
}

thread_local! {
    pub static RT: RefCell<Rt> = RefCell::new(Rt::new("0"));
}

pub fn seed(s: &str) {
    RT.with(|r| *r.borrow_mut() = Rt::new(s));
}
#[inline]
pub fn rand() -> f64 {
    RT.with(|r| r.borrow_mut().rand())
}
#[inline]
pub fn noise(x: f64) -> f64 {
    RT.with(|r| r.borrow_mut().noise3(x, 0.0, 0.0))
}
#[inline]
pub fn noise2(x: f64, y: f64) -> f64 {
    RT.with(|r| r.borrow_mut().noise3(x, y, 0.0))
}
#[inline]
pub fn noise3(x: f64, y: f64, z: f64) -> f64 {
    RT.with(|r| r.borrow_mut().noise3(x, y, z))
}
