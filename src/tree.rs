// Port of shan-shui-inf by Lingdong Huang (https://github.com/LingDong-/shan-shui-inf), MIT. See LICENSE.
use crate::draw::*;
use crate::rng::rand;
use std::f64::consts::PI;

pub fn tree01(x: f64, y: f64, hei: f64, wid: f64, col: Col) -> Canv {
    let reso = 10usize;
    let mut nslist = Vec::with_capacity(reso);
    for i in 0..reso {
        nslist.push([n(i as f64 * 0.5), n2(i as f64 * 0.5, 0.5)]);
    }
    let mut canv = Canv::new();
    let mut line1 = Vec::with_capacity(reso);
    let mut line2 = Vec::with_capacity(reso);
    for i in 0..reso {
        let nx = x;
        let ny = y - (i as f64 * hei) / reso as f64;
        if i as f64 >= reso as f64 / 4.0 {
            let m = (reso - i) as f64 / 5.0;
            let mut j = 0.0;
            while j < m {
                canv.extend(blob(
                    nx + (rand() - 0.5) * wid * 1.2 * (reso - i) as f64,
                    ny + (rand() - 0.5) * wid,
                    &BlobArgs {
                        len: rand() * 20.0 * (reso - i) as f64 * 0.2 + 10.0,
                        wid: rand() * 6.0 + 3.0,
                        ang: ((rand() - 0.5) * PI) / 6.0,
                        col: Col { a: fix(rand() * 0.2 + col.a, 1), ..col },
                        ..Default::default()
                    },
                ));
                j += 1.0;
            }
        }
        line1.push([nx + (nslist[i][0] - 0.5) * wid - wid / 2.0, ny]);
        line2.push([nx + (nslist[i][1] - 0.5) * wid + wid / 2.0, ny]);
    }
    canv.extend(poly(&line1, 0.0, 0.0, NONE, col, 1.5));
    canv.extend(poly(&line2, 0.0, 0.0, NONE, col, 1.5));
    canv
}

pub fn tree02(x: f64, y: f64, hei: f64, wid: f64, clu: usize, col: Col) -> Canv {
    let mut canv = Canv::new();
    for _ in 0..clu {
        canv.extend(blob(
            x + rand_gaussian() * clu as f64 * 4.0,
            y + rand_gaussian() * clu as f64 * 4.0,
            &BlobArgs {
                ang: PI / 2.0,
                fun: &blob_leaf_fun,
                wid: rand() * wid * 0.75 + wid * 0.5,
                len: rand() * hei * 0.75 + hei * 0.5,
                col,
                ..Default::default()
            },
        ));
    }
    canv
}

pub fn tree03(x: f64, y: f64, hei: f64, wid: f64, ben: &dyn Fn(f64) -> f64, col: Col) -> Canv {
    let reso = 10usize;
    let mut nslist = Vec::with_capacity(reso);
    for i in 0..reso {
        nslist.push([n(i as f64 * 0.5), n2(i as f64 * 0.5, 0.5)]);
    }
    let mut canv = Canv::new();
    let mut blobs = Canv::new();
    let mut line1 = Vec::with_capacity(reso);
    let mut line2 = Vec::with_capacity(reso);
    for i in 0..reso {
        let nx = x + ben(i as f64 / reso as f64) * 100.0;
        let ny = y - (i as f64 * hei) / reso as f64;
        if i as f64 >= reso as f64 / 5.0 {
            for _ in 0..(reso - i) * 2 {
                let shape = |v: f64| (50.0 * v + 1.0).ln() / 3.95;
                let ox = rand() * wid * 2.0 * shape((reso - i) as f64 / reso as f64);
                blobs.extend(blob(
                    nx + ox * rand_choice(&[-1.0, 1.0]),
                    ny + (rand() - 0.5) * wid * 2.0,
                    &BlobArgs {
                        len: ox * 2.0,
                        wid: rand() * 6.0 + 3.0,
                        ang: ((rand() - 0.5) * PI) / 6.0,
                        col: Col { a: fix(rand() * 0.2 + col.a, 3), ..col },
                        ..Default::default()
                    },
                ));
            }
        }
        line1.push([nx + (((nslist[i][0] - 0.5) * wid - wid / 2.0) * (reso - i) as f64) / reso as f64, ny]);
        line2.push([nx + (((nslist[i][1] - 0.5) * wid + wid / 2.0) * (reso - i) as f64) / reso as f64, ny]);
    }
    let mut lc = line1.clone();
    let mut l2 = line2.clone();
    l2.reverse();
    lc.extend(l2);
    canv.extend(poly(&lc, 0.0, 0.0, WHITE, col, 1.5));
    canv.extend(blobs);
    canv
}

pub struct BranchArgs {
    pub hei: f64,
    pub wid: f64,
    pub ang: f64,
    pub det: f64,
    pub ben: f64,
}

impl Default for BranchArgs {
    fn default() -> Self {
        BranchArgs { hei: 300.0, wid: 6.0, ang: 0.0, det: 10.0, ben: PI * 0.2 }
    }
}

pub fn branch(a: &BranchArgs) -> (Vec<Pt>, Vec<Pt>) {
    let (hei, wid, ang, det, ben) = (a.hei, a.wid, a.ang, a.det, a.ben);
    let mut nx = 0.0;
    let mut ny = 0.0;
    let mut tlist: Vec<Pt> = vec![[nx, ny]];
    let mut a0 = 0.0;
    let g = 3;
    for _ in 0..g {
        a0 += (ben / 2.0 + (rand() * ben) / 2.0) * rand_choice(&[-1.0, 1.0]);
        nx += (a0.cos() * hei) / g as f64;
        ny -= (a0.sin() * hei) / g as f64;
        tlist.push([nx, ny]);
    }
    let last = tlist[tlist.len() - 1];
    let ta = last[1].atan2(last[0]);
    for p in tlist.iter_mut() {
        let an = p[1].atan2(p[0]);
        let d = (p[0] * p[0] + p[1] * p[1]).sqrt();
        p[0] = d * (an - ta + ang).cos();
        p[1] = d * (an - ta + ang).sin();
    }

    let mut trlist1 = Vec::new();
    let mut trlist2 = Vec::new();
    let span = det;
    let tl = (tlist.len() as f64 - 1.0) * span;
    let mut lx = 0.0;
    let mut ly = 0.0;
    let mut i = 0.0f64;
    while i < tl {
        let lastp = tlist[crate::rng::jfloor(i / span) as usize];
        let nextp = tlist[(i / span).ceil() as usize];
        let p = (i % span) / span;
        let nx = lastp[0] * (1.0 - p) + nextp[0] * p;
        let ny = lastp[1] * (1.0 - p) + nextp[1] * p;
        let angl = (ny - ly).atan2(nx - lx);
        let woff = ((n(i * 0.3) - 0.5) * wid * hei) / 80.0;
        let b = if p == 0.0 { rand() * wid } else { 0.0 };
        let nw = wid * (((tl - i) / tl) * 0.5 + 0.5);
        trlist1.push([
            nx + (angl + PI / 2.0).cos() * (nw + woff + b),
            ny + (angl + PI / 2.0).sin() * (nw + woff + b),
        ]);
        trlist2.push([
            nx + (angl - PI / 2.0).cos() * (nw - woff + b),
            ny + (angl - PI / 2.0).sin() * (nw - woff + b),
        ]);
        lx = nx;
        ly = ny;
        i += 1.0;
    }
    (trlist1, trlist2)
}

pub struct TwigArgs {
    pub dir: f64,
    pub sca: f64,
    pub wid: f64,
    pub ang: f64,
    pub lea: (bool, f64),
}

impl Default for TwigArgs {
    fn default() -> Self {
        TwigArgs { dir: 1.0, sca: 1.0, wid: 1.0, ang: 0.0, lea: (true, 12.0) }
    }
}

pub fn twig(tx: f64, ty: f64, dep: i32, a: &TwigArgs) -> Canv {
    let mut canv = Canv::new();
    let mut twlist = Vec::new();
    let tl = 10usize;
    let hs = rand() * 0.5 + 0.5;
    // original picks from a one-element list, consuming one random value
    let _ = rand_choice(&[0]);
    let tfun = |_x: f64, i: f64| -1.0 / (i / tl as f64 + 1.0).powi(5) + 1.0;
    let a0 = ((rand() * PI) / 6.0) * a.dir + a.ang;
    for i in 0..tl {
        let mx = a.dir * tfun(i as f64 / tl as f64, i as f64) * 50.0 * a.sca * hs;
        let my = -(i as f64) * 5.0 * a.sca;
        let an = my.atan2(mx);
        let d = (mx * mx + my * my).powf(0.5);
        let nx = (an + a0).cos() * d;
        let ny = (an + a0).sin() * d;
        twlist.push([nx + tx, ny + ty]);
        if (i == tl / 3 || i == (tl * 2) / 3) && dep > 0 {
            canv.extend(twig(
                nx + tx,
                ny + ty,
                dep - 1,
                &TwigArgs {
                    ang: a.ang,
                    sca: a.sca * 0.8,
                    wid: a.wid,
                    dir: a.dir * rand_choice(&[-1.0, 1.0]),
                    lea: a.lea,
                },
            ));
        }
        if i == tl - 1 && a.lea.0 {
            for j in 0..5 {
                let dj = (j as f64 - 2.5) * 5.0;
                canv.extend(blob(
                    nx + tx + a.ang.cos() * dj * a.wid,
                    ny + ty + (a.ang.sin() * dj - a.lea.1 / (dep as f64 + 1.0)) * a.wid,
                    &BlobArgs {
                        wid: (6.0 + 3.0 * rand()) * a.wid,
                        len: (15.0 + 12.0 * rand()) * a.wid,
                        ang: a.ang / 2.0 + PI / 2.0 + PI * 0.2 * (rand() - 0.5),
                        col: Col::grey(100.0, fix(0.5 + dep as f64 * 0.2, 3)),
                        fun: &blob_leaf_fun,
                        ..Default::default()
                    },
                ));
            }
        }
    }
    canv.extend(stroke(
        &twlist,
        StrokeArgs {
            wid: 1.0,
            fun: &|x: f64| ((x * PI) / 2.0).cos(),
            col: Col::grey(100.0, 0.5),
            ..Default::default()
        },
    ));
    canv
}

fn bark(x: f64, y: f64, wid: f64, ang: f64) -> Canv {
    let len = 10.0 + 10.0 * rand();
    let noi = 0.5;
    let reso = 20.0;
    let cnt = reso as usize + 1;
    let mut lalist = Vec::with_capacity(cnt);
    for i in 0..cnt {
        let p = (i as f64 / reso) * 2.0;
        let xo = len / 2.0 - (p - 1.0).abs() * len;
        let yo = (blob_default_fun(p) * wid) / 2.0;
        lalist.push([yo.atan2(xo), (xo * xo + yo * yo).sqrt()]);
    }
    let mut nslist = Vec::with_capacity(cnt);
    let n0 = rand() * 10.0;
    for i in 0..cnt {
        nslist.push(n2(i as f64 * 0.05, n0));
    }
    loop_noise(&mut nslist);
    let mut brklist = Vec::with_capacity(cnt);
    for i in 0..cnt {
        let ns = nslist[i] * noi + (1.0 - noi);
        brklist.push([
            x + (lalist[i][0] + ang).cos() * lalist[i][1] * ns,
            y + (lalist[i][0] + ang).sin() * lalist[i][1] * ns,
        ]);
    }
    let fr = rand();
    stroke(
        &brklist,
        StrokeArgs {
            wid: 0.8,
            noi: 0.0,
            col: Col::grey(100.0, 0.4),
            out: 0.0,
            fun: &move |x: f64| ((x + fr) * PI * 3.0).sin(),
            ..Default::default()
        },
    )
}

pub fn barkify(x: f64, y: f64, trlist: &mut (Vec<Pt>, Vec<Pt>)) -> Canv {
    let mut canv = Canv::new();
    for i in 2..trlist.0.len().saturating_sub(1) {
        let a0 = (trlist.0[i][1] - trlist.0[i - 1][1]).atan2(trlist.0[i][0] - trlist.0[i - 1][0]);
        let a1 = (trlist.1[i][1] - trlist.1[i - 1][1]).atan2(trlist.1[i][0] - trlist.1[i - 1][0]);
        let p = rand();
        let nx = trlist.0[i][0] * (1.0 - p) + trlist.1[i][0] * p;
        let ny = trlist.0[i][1] * (1.0 - p) + trlist.1[i][1] * p;
        if rand() < 0.2 {
            canv.extend(blob(
                nx + x,
                ny + y,
                &BlobArgs {
                    noi: 1.0,
                    len: 15.0,
                    wid: 6.0 - (p - 0.5).abs() * 10.0,
                    ang: (a0 + a1) / 2.0,
                    col: Col::grey(100.0, 0.6),
                    ..Default::default()
                },
            ));
        } else {
            canv.extend(bark(nx + x, ny + y, 5.0 - (p - 0.5).abs() * 10.0, (a0 + a1) / 2.0));
        }
        if rand() < 0.05 {
            let jl = rand() * 2.0 + 2.0;
            let xya = rand_choice(&[
                [trlist.0[i][0], trlist.0[i][1], a0],
                [trlist.1[i][0], trlist.1[i][1], a1],
            ]);
            let mut j = 0.0;
            while j < jl {
                canv.extend(blob(
                    xya[0] + x + xya[2].cos() * (j - jl / 2.0) * 4.0,
                    xya[1] + y + xya[2].sin() * (j - jl / 2.0) * 4.0,
                    &BlobArgs {
                        wid: 4.0,
                        len: 4.0 + 6.0 * rand(),
                        ang: a0 + PI / 2.0,
                        col: Col::grey(100.0, 0.6),
                        ..Default::default()
                    },
                ));
                j += 1.0;
            }
        }
    }
    // trflist aliases trlist's own points in the original, and div() hands back
    // the last one unchanged, so the jitter below writes through to the trunk.
    let n0 = trlist.0.len();
    let n1 = trlist.1.len();
    let locate = |k: usize| -> (bool, usize) {
        if k < n0 {
            (false, k)
        } else {
            (true, n1 - 1 - (k - n0))
        }
    };
    let mut rglist: Vec<Vec<usize>> = vec![Vec::new()];
    for k in 0..n0 + n1 {
        if rand() < 0.5 {
            rglist.push(Vec::new());
        } else {
            rglist.last_mut().unwrap().push(k);
        }
    }
    for (i, seg) in rglist.iter().enumerate() {
        let pts: Vec<Pt> = seg
            .iter()
            .map(|&k| {
                let (second, j) = locate(k);
                if second {
                    trlist.1[j]
                } else {
                    trlist.0[j]
                }
            })
            .collect();
        let mut d = div(&pts, 4.0);
        for (j, q) in d.iter_mut().enumerate() {
            q[0] += (n3(i as f64, j as f64 * 0.1, 1.0) - 0.5) * (15.0 + 5.0 * rand_gaussian());
            q[1] += (n3(i as f64, j as f64 * 0.1, 2.0) - 0.5) * (15.0 + 5.0 * rand_gaussian());
        }
        if let (Some(&k), Some(&last)) = (seg.last(), d.last()) {
            let (second, j) = locate(k);
            if second {
                trlist.1[j] = last;
            } else {
                trlist.0[j] = last;
            }
        }
        let pts: Vec<Pt> = d.iter().map(|v| [v[0] + x, v[1] + y]).collect();
        canv.extend(stroke(
            &pts,
            StrokeArgs { wid: 1.5, col: Col::grey(100.0, 0.7), out: 0.0, ..Default::default() },
        ));
    }
    canv
}

fn trunk_outline(trmlist: &[Pt], x: f64, y: f64, col: Col, base_alpha: f64) -> Canv {
    let mut canv = poly(trmlist, x, y, WHITE, col, 0.0);
    if trmlist.len() < 3 {
        return canv;
    }
    let inner: Vec<Pt> = trmlist[1..trmlist.len() - 1].iter().map(|v| [v[0] + x, v[1] + y]).collect();
    canv.extend(stroke(
        &inner,
        StrokeArgs {
            col: Col::grey(100.0, fix(base_alpha + rand() * 0.1, 3)),
            wid: 2.5,
            fun: &|_x: f64| 1f64.sin(),
            noi: 0.9,
            out: 0.0,
            ..Default::default()
        },
    ));
    canv
}

pub fn tree04(x: f64, y: f64, hei: f64, wid: f64, col: Col) -> Canv {
    let mut canv = Canv::new();
    let mut txcanv = Canv::new();
    let mut twcanv = Canv::new();

    let mut br = branch(&BranchArgs { hei, wid, ang: -PI / 2.0, ..Default::default() });
    txcanv.extend(barkify(x, y, &mut br));
    let mut trlist = br.0.clone();
    let mut r = br.1.clone();
    r.reverse();
    trlist.extend(r);

    let mut trmlist: Vec<Pt> = Vec::new();
    let tn = trlist.len() as f64;
    for i in 0..trlist.len() {
        let fi = i as f64;
        if (fi >= tn * 0.3 && fi <= tn * 0.7 && rand() < 0.1) || fi == tn / 2.0 - 1.0 {
            let ba = PI * 0.2 - PI * 1.4 * f64::from(fi > tn / 2.0);
            let mut brlist = branch(&BranchArgs {
                hei: hei * (rand() + 1.0) * 0.3,
                wid: wid * 0.5,
                ang: ba,
                ..Default::default()
            });
            brlist.0.remove(0);
            brlist.1.remove(0);
            let off = trlist[i];
            let b0: Vec<Pt> = brlist.0.iter().map(|v| [v[0] + off[0], v[1] + off[1]]).collect();
            let b1: Vec<Pt> = brlist.1.iter().map(|v| [v[0] + off[0], v[1] + off[1]]).collect();
            txcanv.extend(barkify(x, y, &mut (b0, b1)));
            for j in 0..brlist.0.len() {
                if rand() < 0.2 || j == brlist.0.len() - 1 {
                    twcanv.extend(twig(
                        brlist.0[j][0] + off[0] + x,
                        brlist.0[j][1] + off[1] + y,
                        1,
                        &TwigArgs {
                            wid: hei / 300.0,
                            ang: if ba > -PI / 2.0 { ba } else { ba + PI },
                            sca: (0.5 * hei) / 300.0,
                            dir: if ba > -PI / 2.0 { 1.0 } else { -1.0 },
                            ..Default::default()
                        },
                    ));
                }
            }
            let mut merged = brlist.0.clone();
            let mut rr = brlist.1.clone();
            rr.reverse();
            merged.extend(rr);
            trmlist.extend(merged.iter().map(|v| [v[0] + off[0], v[1] + off[1]]));
        } else {
            trmlist.push(trlist[i]);
        }
    }
    canv.extend(trunk_outline(&trmlist, x, y, col, 0.4));
    canv.extend(txcanv);
    canv.extend(twcanv);
    canv
}

pub fn tree05(x: f64, y: f64, hei: f64, wid: f64, col: Col) -> Canv {
    let mut canv = Canv::new();
    let mut txcanv = Canv::new();
    let mut twcanv = Canv::new();

    let mut br = branch(&BranchArgs { hei, wid, ang: -PI / 2.0, ben: 0.0, ..Default::default() });
    txcanv.extend(barkify(x, y, &mut br));
    let mut trlist = br.0.clone();
    let mut r = br.1.clone();
    r.reverse();
    trlist.extend(r);

    let mut trmlist: Vec<Pt> = Vec::new();
    let tn = trlist.len() as f64;
    for i in 0..trlist.len() {
        let fi = i as f64;
        let p = (fi - tn * 0.5).abs() / (tn * 0.5);
        if (fi >= tn * 0.2 && fi <= tn * 0.8 && i % 3 == 0 && rand() > p) || fi == tn / 2.0 - 1.0 {
            let bar = rand() * 0.2;
            let ba = -bar * PI - (1.0 - bar * 2.0) * PI * f64::from(fi > tn / 2.0);
            let mut brlist = branch(&BranchArgs {
                hei: hei * (0.3 * p - rand() * 0.05),
                wid: wid * 0.5,
                ang: ba,
                ben: 0.5,
                ..Default::default()
            });
            brlist.0.remove(0);
            brlist.1.remove(0);
            let off = trlist[i];
            for j in 0..brlist.0.len() {
                if j % 20 == 0 || j == brlist.0.len() - 1 {
                    twcanv.extend(twig(
                        brlist.0[j][0] + off[0] + x,
                        brlist.0[j][1] + off[1] + y,
                        0,
                        &TwigArgs {
                            wid: hei / 300.0,
                            ang: if ba > -PI / 2.0 { ba } else { ba + PI },
                            sca: (0.2 * hei) / 300.0,
                            dir: if ba > -PI / 2.0 { 1.0 } else { -1.0 },
                            lea: (true, 5.0),
                        },
                    ));
                }
            }
            let mut merged = brlist.0.clone();
            let mut rr = brlist.1.clone();
            rr.reverse();
            merged.extend(rr);
            trmlist.extend(merged.iter().map(|v| [v[0] + off[0], v[1] + off[1]]));
        } else {
            trmlist.push(trlist[i]);
        }
    }
    canv.extend(trunk_outline(&trmlist, x, y, col, 0.4));
    canv.extend(txcanv);
    canv.extend(twcanv);
    canv
}

fn frac_tree6(
    xoff: f64,
    yoff: f64,
    dep: i32,
    hei: f64,
    wid: f64,
    ang: f64,
    ben: f64,
    txcanv: &mut Canv,
    twcanv: &mut Canv,
) -> Vec<Pt> {
    let mut br = branch(&BranchArgs { hei, wid, ang, ben, det: hei / 20.0 });
    txcanv.extend(barkify(xoff, yoff, &mut br));
    let mut trlist = br.0.clone();
    let mut r = br.1.clone();
    r.reverse();
    trlist.extend(r);

    let mut trmlist: Vec<Pt> = Vec::new();
    let tn = trlist.len() as f64;
    let half = (trlist.len() / 2) as i64;
    for i in 0..trlist.len() {
        let fi = i as f64;
        if ((rand() < 0.025 && fi >= tn * 0.2 && fi <= tn * 0.8)
            || i as i64 == half - 1
            || i as i64 == half + 1)
            && dep > 0
        {
            let bar = 0.02 + rand() * 0.08;
            let ba = bar * PI - bar * 2.0 * PI * f64::from(fi > tn / 2.0);
            let off = trlist[i];
            let brlist = frac_tree6(
                off[0] + xoff,
                off[1] + yoff,
                dep - 1,
                hei * (0.7 + rand() * 0.2),
                wid * 0.6,
                ang + ba,
                0.55,
                txcanv,
                twcanv,
            );
            for b in &brlist {
                if rand() < 0.03 {
                    twcanv.extend(twig(
                        b[0] + off[0] + xoff,
                        b[1] + off[1] + yoff,
                        2,
                        &TwigArgs {
                            ang: ba * (rand() * 0.5 + 0.75),
                            sca: 0.3,
                            dir: if ba > 0.0 { 1.0 } else { -1.0 },
                            lea: (false, 0.0),
                            ..Default::default()
                        },
                    ));
                }
            }
            trmlist.extend(brlist.iter().map(|v| [v[0] + off[0], v[1] + off[1]]));
        } else {
            trmlist.push(trlist[i]);
        }
    }
    trmlist
}

pub fn tree06(x: f64, y: f64, hei: f64, wid: f64, col: Col) -> Canv {
    let mut canv = Canv::new();
    let mut txcanv = Canv::new();
    let mut twcanv = Canv::new();
    let trmlist = frac_tree6(x, y, 3, hei, wid, -PI / 2.0, 0.0, &mut txcanv, &mut twcanv);
    canv.extend(trunk_outline(&trmlist, x, y, col, 0.4));
    canv.extend(txcanv);
    canv.extend(twcanv);
    canv
}

pub fn tree07(x: f64, y: f64, hei: f64, wid: f64, ben: &dyn Fn(f64) -> f64, col: Col) -> Canv {
    let reso = 10usize;
    let mut nslist = Vec::with_capacity(reso);
    for i in 0..reso {
        nslist.push([n(i as f64 * 0.5), n2(i as f64 * 0.5, 0.5)]);
    }
    let mut canv = Canv::new();
    let mut line1 = Vec::with_capacity(reso);
    let mut line2 = Vec::with_capacity(reso);
    let mut t: Vec<Vec<Pt>> = Vec::new();
    for i in 0..reso {
        let nx = x + ben(i as f64 / reso as f64) * 100.0;
        let ny = y - (i as f64 * hei) / reso as f64;
        if i as f64 >= reso as f64 / 4.0 {
            let bpl = blob_pts(
                nx + (rand() - 0.5) * wid * 1.2 * (reso - i) as f64 * 0.5,
                ny + (rand() - 0.5) * wid * 0.5,
                &BlobArgs {
                    len: rand() * 50.0 + 20.0,
                    wid: rand() * 12.0 + 12.0,
                    ang: (-rand() * PI) / 6.0,
                    col: Col { a: fix(col.a, 3), ..col },
                    fun: &|v: f64| {
                        if v <= 1.0 {
                            2.75 * v * (1.0 - v).powf(1.0 / 1.8)
                        } else {
                            2.75 * (v - 2.0) * (v - 1.0).powf(1.0 / 1.8)
                        }
                    },
                    ..Default::default()
                },
            );
            t.extend(triangulate(&bpl, 50.0, true, false));
        }
        line1.push([nx + (nslist[i][0] - 0.5) * wid - wid / 2.0, ny]);
        line2.push([nx + (nslist[i][1] - 0.5) * wid + wid / 2.0, ny]);
    }
    let mut lc = line1.clone();
    let mut l2 = line2.clone();
    l2.reverse();
    lc.extend(l2);
    let mut tt = triangulate(&lc, 50.0, true, true);
    tt.extend(t);
    for tri in &tt {
        let m = midpt(tri);
        let c = (n2(m[0] * 0.02, m[1] * 0.02) * 200.0 + 50.0) as i64 as f64;
        let co = Col::grey(c, 0.8);
        canv.extend(poly(tri, 0.0, 0.0, co, co, 0.0));
    }
    canv
}

fn frac_tree8(xoff: f64, yoff: f64, dep: i32, ang: f64, len: f64, ben: f64) -> Canv {
    let spt = [xoff, yoff];
    let ept = [xoff + ang.cos() * len, yoff + ang.sin() * len];
    let mut trmlist = div(&[[xoff, yoff], [xoff + len, yoff]], 10.0);
    let bsign = rand_choice(&[1.0, -1.0]);
    let bfun = move |x: f64| bsign * (x * PI).sin();
    let tl = trmlist.len() as f64;
    for (i, p) in trmlist.iter_mut().enumerate() {
        p[1] += bfun(i as f64 / tl) * 2.0;
    }
    for p in trmlist.iter_mut() {
        let d = distance(*p, spt);
        let a = (p[1] - spt[1]).atan2(p[0] - spt[0]);
        p[0] = spt[0] + d * (a + ang).cos();
        p[1] = spt[1] + d * (a + ang).sin();
    }
    let mut tcanv = Canv::new();
    let f0 = |x: f64| (0.5 * PI * x).cos();
    let f1 = |_x: f64| 1.0;
    let fun: &dyn Fn(f64) -> f64 = if dep == 0 { &f0 } else { &f1 };
    tcanv.extend(stroke(
        &trmlist,
        StrokeArgs { fun, wid: 0.8, col: Col::grey(100.0, 0.5), ..Default::default() },
    ));
    if dep != 0 {
        let nben = ben + rand_choice(&[-1.0, 1.0]) * PI * 0.001 * dep as f64 * dep as f64;
        if rand() < 0.5 {
            let c0 = rand_choice(&[norm_rand(-1.0, 0.5), norm_rand(0.5, 1.0)]);
            tcanv.extend(frac_tree8(
                ept[0],
                ept[1],
                dep - 1,
                ang + ben + PI * c0 * 0.2,
                len * norm_rand(0.8, 0.9),
                nben,
            ));
            let c1 = rand_choice(&[norm_rand(-1.0, -0.5), norm_rand(0.5, 1.0)]);
            tcanv.extend(frac_tree8(
                ept[0],
                ept[1],
                dep - 1,
                ang + ben + PI * c1 * 0.2,
                len * norm_rand(0.8, 0.9),
                nben,
            ));
        } else {
            tcanv.extend(frac_tree8(ept[0], ept[1], dep - 1, ang + ben, len * norm_rand(0.8, 0.9), nben));
        }
    }
    tcanv
}

pub fn tree08(x: f64, y: f64, hei: f64, wid: f64, col: Col) -> Canv {
    let mut canv = Canv::new();
    let txcanv = Canv::new();
    let mut twcanv = Canv::new();
    let ang = norm_rand(-1.0, 1.0) * PI * 0.2;
    let br = branch(&BranchArgs { hei, wid, ang: -PI / 2.0 + ang, ben: PI * 0.2, det: hei / 20.0 });
    let mut trlist = br.0.clone();
    let mut r = br.1.clone();
    r.reverse();
    trlist.extend(r);

    for i in 0..trlist.len() {
        if rand() < 0.2 {
            twcanv.extend(frac_tree8(
                x + trlist[i][0],
                y + trlist[i][1],
                crate::rng::jfloor(4.0 * rand()) as i32,
                -PI / 2.0 - ang * rand(),
                15.0,
                0.0,
            ));
        } else if i == trlist.len() / 2 {
            twcanv.extend(frac_tree8(x + trlist[i][0], y + trlist[i][1], 3, -PI / 2.0 + ang, 15.0, 0.0));
        }
    }
    canv.extend(poly(&trlist, x, y, WHITE, col, 0.0));
    let pts: Vec<Pt> = trlist.iter().map(|v| [v[0] + x, v[1] + y]).collect();
    canv.extend(stroke(
        &pts,
        StrokeArgs {
            col: Col::grey(100.0, fix(0.6 + rand() * 0.1, 3)),
            wid: 2.5,
            fun: &|_x: f64| 1f64.sin(),
            noi: 0.9,
            out: 0.0,
            ..Default::default()
        },
    ));
    canv.extend(txcanv);
    canv.extend(twcanv);
    canv
}
