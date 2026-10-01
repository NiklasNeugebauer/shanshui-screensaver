// Perlin noise, derived from p5.js src/math/noise.js
// (https://github.com/processing/p5.js, Copyright (c) the p5.js contributors),
// which adapted Processing's PApplet.java, itself adapted from toxi and the
// demo group farbrausch. Reached this crate through shan-shui-inf's verbatim
// copy. Unlike the rest of the crate this file is licensed under the GNU
// Lesser General Public License, version 2.1 or (at your option) any later
// version; see LICENSE-LGPL-2.1. The complete source of the program is
// published, which satisfies the LGPL's relinking requirement for the
// statically linked binary.
//
// Call order and float semantics are kept identical to the JS original so
// seeds reproduce the same scene; `jfloor` is JS Math.floor.

use crate::rng::jfloor;

const PERLIN_YWRAPB: i32 = 4;
const PERLIN_YWRAP: i32 = 1 << PERLIN_YWRAPB;
const PERLIN_ZWRAPB: i32 = 8;
const PERLIN_ZWRAP: i32 = 1 << PERLIN_ZWRAPB;
pub const PERLIN_SIZE: i32 = 4095;
const OCTAVES: usize = 4;
const AMP_FALLOFF: f64 = 0.5;

#[inline]
fn scaled_cosine(i: f64) -> f64 {
    0.5 * (1.0 - (i * std::f64::consts::PI).cos())
}

/// `perlin` is the lazily initialised table of `PERLIN_SIZE + 1` uniform draws.
pub fn noise3(perlin: &[f64], x: f64, y: f64, z: f64) -> f64 {
    let (x, y, z) = (x.abs(), y.abs(), z.abs());
    let (mut xi, mut yi, mut zi) = (jfloor(x) as i32, jfloor(y) as i32, jfloor(z) as i32);
    let mut xf = x - jfloor(x);
    let mut yf = y - jfloor(y);
    let mut zf = z - jfloor(z);
    let mut r = 0.0;
    let mut ampl = 0.5;
    for _ in 0..OCTAVES {
        let mut of = xi
            .wrapping_add(yi.wrapping_shl(PERLIN_YWRAPB as u32))
            .wrapping_add(zi.wrapping_shl(PERLIN_ZWRAPB as u32));
        let rxf = scaled_cosine(xf);
        let ryf = scaled_cosine(yf);
        let g = |i: i32| perlin[(i & PERLIN_SIZE) as usize];
        let mut n1 = g(of);
        n1 += rxf * (g(of.wrapping_add(1)) - n1);
        let mut n2 = g(of.wrapping_add(PERLIN_YWRAP));
        n2 += rxf * (g(of.wrapping_add(PERLIN_YWRAP).wrapping_add(1)) - n2);
        n1 += ryf * (n2 - n1);
        of = of.wrapping_add(PERLIN_ZWRAP);
        n2 = g(of);
        n2 += rxf * (g(of.wrapping_add(1)) - n2);
        let mut n3 = g(of.wrapping_add(PERLIN_YWRAP));
        n3 += rxf * (g(of.wrapping_add(PERLIN_YWRAP).wrapping_add(1)) - n3);
        n2 += ryf * (n3 - n2);
        n1 += scaled_cosine(zf) * (n2 - n1);
        r += n1 * ampl;
        ampl *= AMP_FALLOFF;
        xi = xi.wrapping_shl(1);
        xf *= 2.0;
        yi = yi.wrapping_shl(1);
        yf *= 2.0;
        zi = zi.wrapping_shl(1);
        zf *= 2.0;
        if xf >= 1.0 {
            xi = xi.wrapping_add(1);
            xf -= 1.0;
        }
        if yf >= 1.0 {
            yi = yi.wrapping_add(1);
            yf -= 1.0;
        }
        if zf >= 1.0 {
            zi = zi.wrapping_add(1);
            zf -= 1.0;
        }
    }
    r
}
