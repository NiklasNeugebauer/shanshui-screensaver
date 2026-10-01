use crate::arch;
use crate::draw::*;
use crate::mount;
use crate::rng::{jfloor, rand};
use std::collections::HashMap;

pub const CWID: f64 = 512.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tag {
    Mount,
    FlatMount,
    DistMount,
    Boat,
    Water,
}

pub struct Chunk {
    pub y: f64,
    pub x0: f64,
    pub x1: f64,
    pub prims: Vec<Prim>,
}

pub struct World {
    pub chunks: Vec<Chunk>,
    pub xmin: f64,
    pub xmax: f64,
    planmtx: HashMap<i64, i32>,
}

struct Plan {
    tag: Tag,
    x: f64,
    y: f64,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> World {
        World { chunks: Vec::new(), xmin: 0.0, xmax: 0.0, planmtx: HashMap::new() }
    }

    fn mountplanner(&mut self, xmin: f64, xmax: f64) -> Vec<Plan> {
        let mut reg: Vec<Plan> = Vec::new();
        let samp = 0.03;
        let ns = |x: f64| (n(x * samp) - 0.55).max(0.0) * 2.0;
        let yr = |x: f64| n2(x * 0.01, std::f64::consts::PI);
        let locmax = |x: f64, y: f64, r: i64| -> bool {
            let z0 = ns(x);
            if z0 <= 0.3 {
                return false;
            }
            let mut i = x - r as f64;
            while i < x + r as f64 {
                let mut j = y - r as f64;
                while j < y + r as f64 {
                    if ns(i) > z0 {
                        return false;
                    }
                    j += 1.0;
                }
                i += 1.0;
            }
            true
        };
        fn chadd(reg: &mut Vec<Plan>, r: Plan, mind: f64) -> bool {
            for k in reg.iter() {
                if (k.x - r.x).abs() < mind {
                    return false;
                }
            }
            reg.push(r);
            true
        }

        let xstep = 5.0;
        let mwid = 200.0;
        let mut i = xmin;
        while i < xmax {
            let i1 = jfloor(i / xstep) as i64;
            self.planmtx.entry(i1).or_insert(0);
            i += xstep;
        }

        let mut i = xmin;
        while i < xmax {
            let mut j = 0.0;
            while j < yr(i) * 480.0 {
                if locmax(i, j, 2) {
                    let xof = i + 2.0 * (rand() - 0.5) * 500.0;
                    let yof = j + 300.0;
                    if chadd(&mut reg, Plan { tag: Tag::Mount, x: xof, y: yof }, 10.0) {
                        let mut k = jfloor((xof - mwid) / xstep) as i64;
                        while (k as f64) < (xof + mwid) / xstep {
                            *self.planmtx.entry(k).or_insert(0) += 1;
                            k += 1;
                        }
                    }
                }
                j += 30.0;
            }
            if i.abs() % 1000.0 < (xstep - 1.0).max(1.0) {
                chadd(
                    &mut reg,
                    Plan { tag: Tag::DistMount, x: i, y: 280.0 - rand() * 50.0 },
                    10.0,
                );
            }
            i += xstep;
        }

        let mut i = xmin;
        while i < xmax {
            if self.planmtx.get(&(jfloor(i / xstep) as i64)).copied().unwrap_or(0) == 0
                && rand() < 0.01
            {
                let mut j = 0.0;
                let lim = 4.0 * rand();
                while j < lim {
                    chadd(
                        &mut reg,
                        Plan {
                            tag: Tag::FlatMount,
                            x: i + 2.0 * (rand() - 0.5) * 700.0,
                            y: 700.0 - j * 50.0,
                        },
                        10.0,
                    );
                    j += 1.0;
                }
            }
            i += xstep;
        }

        let mut i = xmin;
        while i < xmax {
            if rand() < 0.2 {
                chadd(&mut reg, Plan { tag: Tag::Boat, x: i, y: 300.0 + rand() * 390.0 }, 400.0);
            }
            i += xstep;
        }
        reg
    }

    fn add(&mut self, y: f64, prims: Vec<Prim>) {
        if prims.is_empty() {
            return;
        }
        let (mut x0, mut x1) = (f64::MAX, f64::MIN);
        for p in &prims {
            for q in &p.pts {
                if q[0].is_finite() {
                    x0 = x0.min(q[0]);
                    x1 = x1.max(q[0]);
                }
            }
        }
        let pad = 4.0;
        let ch = Chunk { y, x0: x0 - pad, x1: x1 + pad, prims };
        let n = self.chunks.len();
        if n == 0 {
            self.chunks.push(ch);
        } else if ch.y <= self.chunks[0].y {
            self.chunks.insert(0, ch);
        } else if ch.y >= self.chunks[n - 1].y {
            self.chunks.push(ch);
        } else {
            let mut at = n;
            for j in 0..n - 1 {
                if self.chunks[j].y <= ch.y && ch.y <= self.chunks[j + 1].y {
                    at = j + 1;
                    break;
                }
            }
            self.chunks.insert(at, ch);
        }
    }

    /// Generate forward until xmax is covered, mirroring the original chunkloader.
    pub fn load(&mut self, xmax: f64) {
        while xmax > self.xmax - CWID {
            let plan = self.mountplanner(self.xmax, self.xmax + CWID);
            self.xmax += CWID;
            for (i, p) in plan.iter().enumerate() {
                match p.tag {
                    Tag::Mount => {
                        let seed = i as f64 * 2.0 * rand();
                        let hei = 100.0 + rand() * 400.0;
                        let wid = 400.0 + rand() * 200.0;
                        let c = mount::mountain(p.x, p.y, seed, hei, wid, 200, true);
                        self.add(p.y, c);
                        let w = mount::water(p.x, p.y, 2.0, 800.0, 10);
                        self.add(p.y - 10000.0, w);
                    }
                    Tag::FlatMount => {
                        let seed = 2.0 * rand() * std::f64::consts::PI;
                        let wid = 600.0 + rand() * 400.0;
                        let cho = 0.5 + rand() * 0.2;
                        let c = mount::flat_mount(p.x, p.y, seed, 100.0, wid, 80, cho);
                        self.add(p.y, c);
                    }
                    Tag::DistMount => {
                        let seed = rand() * 100.0;
                        let len = rand_choice(&[500.0, 1000.0, 1500.0]);
                        let c = mount::dist_mount(p.x, p.y, seed, 150.0, len, 5);
                        self.add(p.y, c);
                    }
                    Tag::Boat => {
                        let _seed = rand();
                        let sca = p.y / 800.0;
                        let fli = rand_choice(&[true, false]);
                        let c = arch::boat01(p.x, p.y, 120.0, sca, fli);
                        self.add(p.y, c);
                    }
                    Tag::Water => {}
                }
            }
        }
    }

    pub fn drop_before(&mut self, x: f64) {
        self.chunks.retain(|c| c.x1 >= x);
    }
}
