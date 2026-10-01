use crate::arch;
use crate::draw::*;
use crate::rng::rand;
use crate::tree;
use std::f64::consts::PI;

fn foot(ptlist: &[Vec<Pt>], xof: f64, yof: f64) -> Canv {
    let mut ftlist: Vec<Vec<Pt>> = Vec::new();
    let span = 10;
    let mut ni = 0usize;
    for i in 0..ptlist.len().saturating_sub(2) {
        if i != ni {
            continue;
        }
        ni = (ni + rand_choice(&[1usize, 2])).min(ptlist.len() - 1);
        ftlist.push(Vec::new());
        ftlist.push(Vec::new());
        let l = ftlist.len();
        let m = (ptlist[i].len() as f64 / 8.0).min(10.0);
        let mut j = 0usize;
        while (j as f64) < m {
            let a = [ptlist[i][j][0] + n2(j as f64 * 0.1, i as f64) * 10.0, ptlist[i][j][1]];
            ftlist[l - 2].push(a);
            let k = ptlist[i].len() - 1 - j;
            let b = [ptlist[i][k][0] - n2(j as f64 * 0.1, i as f64) * 10.0, ptlist[i][k][1]];
            ftlist[l - 1].push(b);
            j += 1;
        }
        ftlist[l - 2].reverse();
        ftlist[l - 1].reverse();
        for j in 0..span {
            let p = j as f64 / span as f64;
            let mut x1 = ptlist[i][0][0] * (1.0 - p) + ptlist[ni][0][0] * p;
            let mut y1 = ptlist[i][0][1] * (1.0 - p) + ptlist[ni][0][1] * p;
            let last = ptlist[i].len() - 1;
            let mut x2 = ptlist[i][last][0] * (1.0 - p) + ptlist[ni][last][0] * p;
            let mut y2 = ptlist[i][last][1] * (1.0 - p) + ptlist[ni][last][1] * p;
            let vib = -1.7 * (p - 1.0) * p.powf(1.0 / 5.0);
            let nz = n2(xof * 0.05, i as f64) * 5.0;
            y1 += vib * 5.0 + nz;
            y2 += vib * 5.0 + nz;
            let _ = (&mut x1, &mut x2);
            ftlist[l - 2].push([x1, y1]);
            ftlist[l - 1].push([x2, y2]);
        }
    }
    let mut canv = Canv::new();
    for f in &ftlist {
        canv.extend(poly(f, xof, yof, WHITE, NONE, 0.0));
    }
    for f in &ftlist {
        let pts: Vec<Pt> = f.iter().map(|x| [x[0] + xof, x[1] + yof]).collect();
        canv.extend(stroke(
            &pts,
            StrokeArgs {
                col: Col::grey(100.0, fix(0.1 + rand() * 0.1, 3)),
                wid: 1.0,
                ..Default::default()
            },
        ));
    }
    canv
}

fn vegetate(
    ptlist: &[Vec<Pt>],
    canv: &mut Canv,
    tree_func: &mut dyn FnMut(f64, f64) -> Canv,
    growth: &dyn Fn(usize, usize) -> bool,
    proof: &dyn Fn(&[Pt], usize) -> bool,
) {
    let mut veglist: Vec<Pt> = Vec::new();
    for (i, row) in ptlist.iter().enumerate() {
        for (j, p) in row.iter().enumerate() {
            if growth(i, j) {
                veglist.push(*p);
            }
        }
    }
    for i in 0..veglist.len() {
        if proof(&veglist, i) {
            let c = tree_func(veglist[i][0], veglist[i][1]);
            canv.extend(c);
        }
    }
}

fn proof_true(_v: &[Pt], _i: usize) -> bool {
    true
}

pub fn mountain(xoff: f64, yoff: f64, seed: f64, hei: f64, wid: f64, tex: usize, veg: bool) -> Canv {
    let mut canv = Canv::new();
    let h = hei;
    let w = wid;
    let reso = [10usize, 50usize];
    let mut ptlist: Vec<Vec<Pt>> = Vec::with_capacity(reso[0]);
    let mut hoff = 0.0;
    for j in 0..reso[0] {
        hoff += (rand() * yoff) / 100.0;
        let mut row = Vec::with_capacity(reso[1]);
        for i in 0..reso[1] {
            let x = (i as f64 / reso[1] as f64 - 0.5) * PI;
            let mut y = x.cos();
            y *= n3(x + 10.0, j as f64 * 0.15, seed);
            let p = 1.0 - j as f64 / reso[0] as f64;
            row.push([(x / PI) * w * p, -y * h * p + hoff]);
        }
        ptlist.push(row);
    }

    // RIM
    vegetate(
        &ptlist,
        &mut canv,
        &mut |x, y| {
            tree::tree02(
                x + xoff,
                y + yoff - 5.0,
                16.0,
                8.0,
                2,
                Col::grey(100.0, fix(n2(0.01 * x, 0.01 * y) * 0.5 * 0.3 + 0.5, 3)),
            )
        },
        &|i, j| {
            let ns = n2(j as f64 * 0.1, seed);
            i == 0 && ns * ns * ns < 0.1 && ptlist[i][j][1].abs() / h > 0.2
        },
        &proof_true,
    );

    let mut bg = ptlist[0].clone();
    bg.push([0.0, reso[0] as f64 * 4.0]);
    canv.extend(poly(&bg, xoff, yoff, WHITE, NONE, 0.0));
    let outline: Vec<Pt> = ptlist[0].iter().map(|x| [x[0] + xoff, x[1] + yoff]).collect();
    canv.extend(stroke(
        &outline,
        StrokeArgs { col: Col::grey(100.0, 0.3), noi: 1.0, wid: 3.0, ..Default::default() },
    ));
    canv.extend(foot(&ptlist, xoff, yoff));
    let sha = rand_choice(&[0.0, 0.0, 0.0, 0.0, 5.0]);
    canv.extend(texture(&ptlist, &TexArgs { xof: xoff, yof: yoff, tex, sha, ..Default::default() }));

    // TOP
    vegetate(
        &ptlist,
        &mut canv,
        &mut |x, y| {
            tree::tree02(
                x + xoff,
                y + yoff,
                16.0,
                8.0,
                5,
                Col::grey(100.0, fix(n2(0.01 * x, 0.01 * y) * 0.5 * 0.3 + 0.5, 3)),
            )
        },
        &|i, j| {
            let ns = n3(i as f64 * 0.1, j as f64 * 0.1, seed + 2.0);
            ns * ns * ns < 0.1 && ptlist[i][j][1].abs() / h > 0.5
        },
        &proof_true,
    );

    if veg {
        // MIDDLE
        vegetate(
            &ptlist,
            &mut canv,
            &mut |x, y| {
                let mut ht = ((h + y) / h) * 70.0;
                ht = ht * 0.3 + rand() * ht * 0.7;
                tree::tree01(
                    x + xoff,
                    y + yoff,
                    ht,
                    rand() * 3.0 + 1.0,
                    Col::grey(100.0, fix(n2(0.01 * x, 0.01 * y) * 0.5 * 0.3 + 0.3, 3)),
                )
            },
            &|i, j| {
                let ns = n3(i as f64 * 0.2, j as f64 * 0.05, seed);
                j % 2 == 1 && ns * ns * ns * ns < 0.012 && ptlist[i][j][1].abs() / h < 0.3
            },
            &|veglist, i| {
                let mut counter = 0;
                for j in 0..veglist.len() {
                    if i != j
                        && (veglist[i][0] - veglist[j][0]).powi(2)
                            + (veglist[i][1] - veglist[j][1]).powi(2)
                            < 900.0
                    {
                        counter += 1;
                    }
                    if counter > 2 {
                        return true;
                    }
                }
                false
            },
        );

        // BOTTOM
        vegetate(
            &ptlist,
            &mut canv,
            &mut |x, y| {
                let mut ht = ((h + y) / h) * 120.0;
                ht = ht * 0.5 + rand() * ht * 0.5;
                let bc = rand() * 0.1;
                tree::tree03(
                    x + xoff,
                    y + yoff,
                    ht,
                    5.0,
                    &move |v: f64| v * bc,
                    Col::grey(100.0, fix(n2(0.01 * x, 0.01 * y) * 0.5 * 0.3 + 0.3, 3)),
                )
            },
            &|i, j| {
                let ns = n3(i as f64 * 0.2, j as f64 * 0.05, seed);
                (j == 0 || j == ptlist[i].len() - 1) && ns * ns * ns * ns < 0.012
            },
            &proof_true,
        );
    }

    // BOTT ARCH
    vegetate(
        &ptlist,
        &mut canv,
        &mut |x, y| {
            let tt = rand_choice(&[0, 0, 1, 1, 1, 2]);
            if tt == 1 {
                let wid = norm_rand(40.0, 70.0);
                let sto = rand_choice(&[1, 2, 2, 3]);
                let rot = rand();
                let sty = rand_choice(&[1, 2, 3]);
                arch::arch02(x + xoff, y + yoff, seed, 10.0, wid, rot, 5.0, sto, sty, false)
            } else if tt == 2 {
                arch::arch04(x + xoff, y + yoff, 15.0, 30.0, 0.7, 5.0, rand_choice(&[1, 1, 1, 2, 2]))
            } else {
                Canv::new()
            }
        },
        &|i, j| {
            let ns = n3(i as f64 * 0.2, j as f64 * 0.05, seed + 10.0);
            i != 0 && (j == 1 || j == ptlist[i].len() - 2) && ns * ns * ns * ns < 0.008
        },
        &proof_true,
    );

    // TOP ARCH
    vegetate(
        &ptlist,
        &mut canv,
        &mut |x, y| {
            let sto = rand_choice(&[5, 7]);
            let wid = 40.0 + rand() * 20.0;
            arch::arch03(x + xoff, y + yoff, 10.0, wid, 0.7, 5.0, sto)
        },
        &|i, j| i == 1 && (j as f64 - ptlist[i].len() as f64 / 2.0).abs() < 1.0 && rand() < 0.02,
        &proof_true,
    );

    // TRANSMISSION TOWER
    vegetate(
        &ptlist,
        &mut canv,
        &mut |x, y| arch::transmission_tower01(x + xoff, y + yoff, 100.0, 20.0),
        &|i, j| {
            let ns = n3(i as f64 * 0.2, j as f64 * 0.05, seed + 20.0 * PI);
            i % 2 == 0 && (j == 1 || j == ptlist[i].len() - 2) && ns * ns * ns * ns < 0.002
        },
        &proof_true,
    );

    // BOTT ROCK
    vegetate(
        &ptlist,
        &mut canv,
        &mut |x, y| {
            let wid = 20.0 + rand() * 20.0;
            let hei = 20.0 + rand() * 20.0;
            rock(x + xoff, y + yoff, seed, wid, hei, 40, 2.0)
        },
        &|i, j| (j == 0 || j == ptlist[i].len() - 1) && rand() < 0.1,
        &proof_true,
    );

    canv
}

pub fn flat_mount(xoff: f64, yoff: f64, seed: f64, hei: f64, wid: f64, tex: usize, cho: f64) -> Canv {
    let mut canv = Canv::new();
    let reso = [5usize, 50usize];
    let mut hoff = 0.0;
    let mut ptlist: Vec<Vec<Pt>> = Vec::new();
    let mut flat: Vec<Vec<Pt>> = Vec::new();
    for j in 0..reso[0] {
        hoff += (rand() * yoff) / 100.0;
        ptlist.push(Vec::new());
        flat.push(Vec::new());
        for i in 0..reso[1] {
            let x = (i as f64 / reso[1] as f64 - 0.5) * PI;
            let mut y = (x * 2.0).cos() + 1.0;
            y *= n3(x + 10.0, j as f64 * 0.1, seed);
            let p = 1.0 - (j as f64 / reso[0] as f64) * 0.6;
            let nx = (x / PI) * wid * p;
            let mut ny = -y * hei * p + hoff;
            let h = 100.0;
            if ny < -h * cho + hoff {
                ny = -h * cho + hoff;
                let fl = flat.last_mut().unwrap();
                if fl.len() % 2 == 0 {
                    fl.push([nx, ny]);
                }
            } else {
                let lastrow = ptlist.last().unwrap();
                let lastpt = lastrow.last().copied();
                let fl = flat.last_mut().unwrap();
                if fl.len() % 2 == 1 {
                    if let Some(lp) = lastpt {
                        fl.push(lp);
                    }
                }
            }
            ptlist.last_mut().unwrap().push([nx, ny]);
        }
    }

    let mut bg = ptlist[0].clone();
    bg.push([0.0, reso[0] as f64 * 4.0]);
    canv.extend(poly(&bg, xoff, yoff, WHITE, NONE, 0.0));
    let outline: Vec<Pt> = ptlist[0].iter().map(|x| [x[0] + xoff, x[1] + yoff]).collect();
    canv.extend(stroke(
        &outline,
        StrokeArgs { col: Col::grey(100.0, 0.3), noi: 1.0, wid: 3.0, ..Default::default() },
    ));
    canv.extend(texture(
        &ptlist,
        &TexArgs {
            xof: xoff,
            yof: yoff,
            tex,
            wid: 2.0,
            dis: &|| {
                if rand() > 0.5 {
                    0.1 + 0.4 * rand()
                } else {
                    0.9 - 0.4 * rand()
                }
            },
            ..Default::default()
        },
    ));

    let mut grlist1: Vec<Pt> = Vec::new();
    let mut grlist2: Vec<Pt> = Vec::new();
    let mut i = 0;
    while i < flat.len() {
        if flat[i].len() >= 2 {
            grlist1.push(flat[i][0]);
            grlist2.push(flat[i][flat[i].len() - 1]);
        }
        i += 2;
    }
    if grlist1.is_empty() {
        return canv;
    }
    let mut wb = [grlist1[0][0], grlist2[0][0]];
    for i in 0..3 {
        let p = 0.8 - i as f64 * 0.2;
        grlist1.insert(0, [wb[0] * p, grlist1[0][1] - 5.0]);
        grlist2.insert(0, [wb[1] * p, grlist2[0][1] - 5.0]);
    }
    wb = [grlist1[grlist1.len() - 1][0], grlist2[grlist2.len() - 1][0]];
    for i in 0..3 {
        let p = 0.6 - (i * i) as f64 * 0.1;
        grlist1.push([wb[0] * p, grlist1[grlist1.len() - 1][1] + 1.0]);
        grlist2.push([wb[1] * p, grlist2[grlist2.len() - 1][1] + 1.0]);
    }
    let d = 5.0;
    let mut g1 = div(&grlist1, d);
    let g2 = div(&grlist2, d);
    g1.reverse();
    let first = g1[0];
    let mut grlist = g1;
    grlist.extend(g2);
    grlist.push(first);
    for (i, p) in grlist.iter_mut().enumerate() {
        let v = (1.0 - ((i as f64 % d) - d / 2.0).abs() / (d / 2.0)) * 0.12;
        p[0] *= 1.0 - v + n(p[1] * 0.5) * v;
    }
    canv.extend(poly(&grlist, xoff, yoff, WHITE, NONE, 2.0));
    let pts: Vec<Pt> = grlist.iter().map(|x| [x[0] + xoff, x[1] + yoff]).collect();
    canv.extend(stroke(
        &pts,
        StrokeArgs { wid: 3.0, col: Col::grey(100.0, 0.2), ..Default::default() },
    ));

    let (mut xmin, mut xmax, mut ymin, mut ymax) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in &grlist {
        xmin = xmin.min(p[0]);
        xmax = xmax.max(p[0]);
        ymin = ymin.min(p[1]);
        ymax = ymax.max(p[1]);
    }
    canv.extend(flat_dec(xoff, yoff, xmin, xmax, ymin, ymax));
    canv
}

pub fn flat_dec(xoff: f64, yoff: f64, xmin: f64, xmax: f64, ymin: f64, ymax: f64) -> Canv {
    let mut canv = Canv::new();
    let tt = rand_choice(&[0, 0, 1, 2, 3, 4]);
    let ymid = (ymin + ymax) / 2.0;

    let mut j = 0.0;
    let lim = rand() * 5.0;
    while j < lim {
        canv.extend(rock(
            xoff + norm_rand(xmin, xmax),
            yoff + ymid + norm_rand(-10.0, 10.0) + 10.0,
            rand() * 100.0,
            10.0 + rand() * 20.0,
            10.0 + rand() * 20.0,
            40,
            2.0,
        ));
        j += 1.0;
    }
    for _ in 0..rand_choice(&[0, 0, 1, 2]) {
        let xr = xoff + norm_rand(xmin, xmax);
        let yr = yoff + ymid + norm_rand(-5.0, 5.0) + 20.0;
        let mut k = 0.0;
        let kl = 2.0 + rand() * 3.0;
        while k < kl {
            canv.extend(tree::tree08(
                xr + norm_rand(-30.0, 30.0).clamp(xmin, xmax),
                yr,
                60.0 + rand() * 40.0,
                1.0,
                Col::grey(100.0, 0.5),
            ));
            k += 1.0;
        }
    }
    if tt == 0 {
        let mut j = 0.0;
        let lim = rand() * 3.0;
        while j < lim {
            canv.extend(rock(
                xoff + norm_rand(xmin, xmax),
                yoff + ymid + norm_rand(-5.0, 5.0) + 20.0,
                rand() * 100.0,
                50.0 + rand() * 20.0,
                40.0 + rand() * 20.0,
                40,
                5.0,
            ));
            j += 1.0;
        }
    }
    if tt == 1 {
        let pmin = rand() * 0.5;
        let pmax = rand() * 0.5 + 0.5;
        let xa = xmin * (1.0 - pmin) + xmax * pmin;
        let xb = xmin * (1.0 - pmax) + xmax * pmax;
        let mut i = xa;
        while i < xb {
            canv.extend(tree::tree05(
                xoff + i + 20.0 * norm_rand(-1.0, 1.0),
                yoff + ymid + 20.0,
                100.0 + rand() * 200.0,
                5.0,
                Col::grey(100.0, 0.5),
            ));
            i += 30.0;
        }
        let mut j = 0.0;
        let lim = rand() * 4.0;
        while j < lim {
            canv.extend(rock(
                xoff + norm_rand(xmin, xmax),
                yoff + ymid + norm_rand(-5.0, 5.0) + 20.0,
                rand() * 100.0,
                50.0 + rand() * 20.0,
                40.0 + rand() * 20.0,
                40,
                5.0,
            ));
            j += 1.0;
        }
    } else if tt == 2 {
        for i in 0..rand_choice(&[1, 1, 1, 1, 2, 2, 3]) {
            let xr = norm_rand(xmin, xmax);
            let yr = ymid;
            canv.extend(tree::tree04(xoff + xr, yoff + yr + 20.0, 300.0, 6.0, Col::grey(100.0, 0.5)));
            let mut j = 0.0;
            let lim = rand() * 2.0;
            while j < lim {
                canv.extend(rock(
                    xoff + (xr + norm_rand(-50.0, 50.0)).clamp(xmin, xmax),
                    yoff + yr + norm_rand(-5.0, 5.0) + 20.0,
                    j * i as f64 * rand() * 100.0,
                    50.0 + rand() * 20.0,
                    40.0 + rand() * 20.0,
                    40,
                    5.0,
                ));
                j += 1.0;
            }
        }
    } else if tt == 3 {
        for _ in 0..rand_choice(&[1, 1, 1, 1, 2, 2, 3]) {
            canv.extend(tree::tree06(
                xoff + norm_rand(xmin, xmax),
                yoff + ymid,
                60.0 + rand() * 60.0,
                6.0,
                Col::grey(100.0, 0.5),
            ));
        }
    } else if tt == 4 {
        let pmin = rand() * 0.5;
        let pmax = rand() * 0.5 + 0.5;
        let xa = xmin * (1.0 - pmin) + xmax * pmin;
        let xb = xmin * (1.0 - pmax) + xmax * pmax;
        let mut i = xa;
        while i < xb {
            canv.extend(tree::tree07(
                xoff + i + 20.0 * norm_rand(-1.0, 1.0),
                yoff + ymid + norm_rand(-1.0, 1.0),
                norm_rand(40.0, 80.0),
                4.0,
                &|v: f64| v.sqrt() * 0.2,
                Col::grey(100.0, 1.0),
            ));
            i += 20.0;
        }
    }

    let mut i = 0.0;
    let lim = 50.0 * rand();
    while i < lim {
        canv.extend(tree::tree02(
            xoff + norm_rand(xmin, xmax),
            yoff + norm_rand(ymin, ymax),
            16.0,
            8.0,
            5,
            Col::grey(100.0, 0.5),
        ));
        i += 1.0;
    }

    let ts = rand_choice(&[0, 0, 0, 0, 1]);
    if ts == 1 && tt != 4 {
        let ax = xoff + norm_rand(xmin, xmax);
        let seed = rand();
        let wid = norm_rand(160.0, 200.0);
        let hei = norm_rand(80.0, 100.0);
        let per = rand();
        canv.extend(arch::arch01(ax, yoff + ymid + 20.0, seed, wid, hei, per));
    }
    canv
}

pub fn dist_mount(xoff: f64, yoff: f64, seed: f64, hei: f64, len: f64, seg: usize) -> Canv {
    let mut canv = Canv::new();
    let span = 10.0;
    let mut ptlist: Vec<Vec<Pt>> = Vec::new();
    let cnt = (len / span / seg as f64) as usize;
    for i in 0..cnt {
        let mut row: Vec<Pt> = Vec::new();
        for j in 0..seg + 1 {
            let k = (i * seg + j) as f64;
            row.push([
                xoff + k * span,
                yoff - hei * n2(k * 0.05, seed) * ((PI * k) / (len / span)).sin().powf(0.5),
            ]);
        }
        let jb = seg as f64 / 2.0 + 1.0;
        let mut j = 0usize;
        while (j as f64) < jb {
            let k = (i * seg + j * 2) as f64;
            row.insert(
                0,
                [
                    xoff + k * span,
                    yoff + 24.0 * n3(k * 0.05, 2.0, seed) * ((PI * k) / (len / span)).sin(),
                ],
            );
            j += 1;
        }
        ptlist.push(row);
    }
    for row in &ptlist {
        let getcol = |x: f64, y: f64| {
            let c = (n3(x * 0.02, y * 0.02, yoff) * 55.0 + 200.0) as i64 as f64;
            Col::grey(c, 1.0)
        };
        let lastp = row[row.len() - 1];
        canv.extend(poly(row, 0.0, 0.0, getcol(lastp[0], lastp[1]), NONE, 1.0));
        let t = triangulate(row, 100.0, true, false);
        for tri in &t {
            let m = midpt(tri);
            let co = getcol(m[0], m[1]);
            canv.extend(poly(tri, 0.0, 0.0, co, co, 1.0));
        }
    }
    canv
}

pub fn rock(xoff: f64, yoff: f64, seed: f64, wid: f64, hei: f64, tex: usize, sha: f64) -> Canv {
    let mut canv = Canv::new();
    let reso = [10usize, 50usize];
    let mut ptlist: Vec<Vec<Pt>> = Vec::with_capacity(reso[0]);
    for i in 0..reso[0] {
        let mut nslist = Vec::with_capacity(reso[1]);
        for j in 0..reso[1] {
            nslist.push(n3(i as f64, j as f64 * 0.2, seed));
        }
        loop_noise(&mut nslist);
        let mut row = Vec::with_capacity(reso[1]);
        for j in 0..reso[1] {
            let a = (j as f64 / reso[1] as f64) * PI * 2.0 - PI / 2.0;
            let mut l = (wid * hei)
                / ((hei * a.cos()).powi(2) + (wid * a.sin()).powi(2)).sqrt();
            l *= 0.7 + 0.3 * nslist[j];
            let p = 1.0 - i as f64 / reso[0] as f64;
            let nx = a.cos() * l * p;
            let mut ny = -a.sin() * l * p;
            if PI < a || a < 0.0 {
                ny *= 0.2;
            }
            ny += hei * (i as f64 / reso[0] as f64) * 0.2;
            row.push([nx, ny]);
        }
        ptlist.push(row);
    }
    let mut bg = ptlist[0].clone();
    bg.push([0.0, 0.0]);
    canv.extend(poly(&bg, xoff, yoff, WHITE, NONE, 0.0));
    let outline: Vec<Pt> = ptlist[0].iter().map(|x| [x[0] + xoff, x[1] + yoff]).collect();
    canv.extend(stroke(
        &outline,
        StrokeArgs { col: Col::grey(100.0, 0.3), noi: 1.0, wid: 3.0, ..Default::default() },
    ));
    canv.extend(texture(
        &ptlist,
        &TexArgs {
            xof: xoff,
            yof: yoff,
            tex,
            wid: 3.0,
            sha,
            col: &|_x| Col::grey(180.0, fix(0.3 + rand() * 0.3, 3)),
            dis: &|| {
                if rand() > 0.5 {
                    0.15 + 0.15 * rand()
                } else {
                    0.85 - 0.15 * rand()
                }
            },
            ..Default::default()
        },
    ));
    canv
}

pub fn water(xoff: f64, yoff: f64, hei: f64, len: f64, clu: usize) -> Canv {
    let mut canv = Canv::new();
    let mut ptlist: Vec<Vec<Pt>> = Vec::with_capacity(clu);
    let mut yk = 0.0;
    for _ in 0..clu {
        let mut row = Vec::new();
        let xk = (rand() - 0.5) * (len / 8.0);
        yk += rand() * 5.0;
        let lk = len / 4.0 + rand() * (len / 4.0);
        let reso = 5.0;
        let mut j = -lk;
        while j < lk {
            row.push([j + xk, (j * 0.2).sin() * hei * n(j * 0.1) - 20.0 + yk]);
            j += reso;
        }
        ptlist.push(row);
    }
    for row in ptlist.iter().skip(1) {
        let pts: Vec<Pt> = row.iter().map(|x| [x[0] + xoff, x[1] + yoff]).collect();
        canv.extend(stroke(
            &pts,
            StrokeArgs {
                col: Col::grey(100.0, fix(0.3 + rand() * 0.3, 3)),
                wid: 1.0,
                ..Default::default()
            },
        ));
    }
    canv
}
