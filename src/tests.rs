use crate::rng::{self, Prng, Rt};
use crate::world::World;

fn first_n(seed: &str, n: usize) -> Vec<f64> {
    let mut p = Prng::new(seed);
    (0..n).map(|_| p.next()).collect()
}

#[test]
fn prng_is_deterministic_per_seed() {
    assert_eq!(first_n("123", 8), first_n("123", 8));
    assert_ne!(first_n("123", 8), first_n("124", 8));
}

#[test]
fn prng_stays_in_unit_range() {
    for v in first_n("shanshui", 2000) {
        assert!((0.0..1.0).contains(&v), "{v} out of range");
    }
}

#[test]
fn prng_matches_the_original_js() {
    // Values produced by the upstream Prng under node for seed "123"; they pin
    // down the f64 Blum-Blum-Shub quirks the port has to keep.
    let got: Vec<f64> = first_n("123", 4).iter().map(|v| (v * 1e12).round()).collect();
    assert_eq!(got, vec![563566544187.0, 321631826710.0, 265661911200.0, 977427078879.0]);
}

#[test]
fn noise_is_deterministic_and_bounded() {
    let mut a = Rt::new("seed-a");
    let mut b = Rt::new("seed-a");
    for i in 0..200 {
        let x = i as f64 * 0.37;
        let va = a.noise3(x, x * 0.5, 1.0);
        assert_eq!(va, b.noise3(x, x * 0.5, 1.0));
        assert!((0.0..=1.0).contains(&va), "{va} out of range");
    }
}

#[test]
fn noise_lazy_table_consumes_the_prng() {
    // The original seeds Perlin from Math.random() on first use, which shifts
    // every later draw; the port must do the same.
    let mut a = Rt::new("x");
    let mut b = Rt::new("x");
    a.noise3(1.0, 0.0, 0.0);
    assert_ne!(a.rand(), b.rand());
}

/// (chunks, polylines, finite coordinates, sum of those coordinates) for the
/// same world the original's `chunkloader(0, 1200)` builds.
fn fingerprint(seed: &str) -> (usize, usize, usize, f64) {
    rng::seed(seed);
    let mut w = World::new();
    w.load(0.0, 1200.0);
    let polylines: usize = w.chunks.iter().map(|c| c.prims.len()).sum();
    let mut coords = 0usize;
    let mut sum = 0.0;
    for v in w
        .chunks
        .iter()
        .flat_map(|c| c.prims.iter())
        .flat_map(|p| p.pts.iter())
        .flatten()
    {
        if v.is_finite() {
            coords += 1;
            sum += *v;
        }
    }
    (w.chunks.len(), polylines, coords, sum)
}

#[test]
fn chunks_match_the_original_js() {
    // Reference figures from running the upstream index.html under node.
    for (seed, chunks, polylines, coords, sum) in [
        ("123", 45, 23825, 885172, 713728631.309304),
        ("777", 63, 25580, 947502, 508971564.34038925),
        ("abc", 35, 15068, 431428, 352482129.5187212),
        ("20260101", 77, 36114, 1424160, 611127037.820957),
    ] {
        let got = fingerprint(seed);
        assert_eq!((got.0, got.1, got.2), (chunks, polylines, coords), "seed {seed}");
        assert!(
            ((got.3 - sum) / sum).abs() < 1e-9,
            "seed {seed}: coordinate sum {} vs {sum}",
            got.3
        );
    }
}

#[test]
fn chunks_reproduce_for_a_seed() {
    let a = fingerprint("reproduce-me");
    let b = fingerprint("reproduce-me");
    assert_eq!(a, b);
    assert!(a.1 > 1000, "expected a populated scene, got {} primitives", a.1);
    assert!(a.3.is_finite());
}

#[test]
fn different_seeds_make_different_scenes() {
    assert_ne!(fingerprint("alpha"), fingerprint("beta"));
}
