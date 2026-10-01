// Port of shan-shui-inf by Lingdong Huang (https://github.com/LingDong-/shan-shui-inf), MIT. See LICENSE.
use crate::draw::*;
use crate::rng::rand;
use std::f64::consts::PI;

fn expand(ptlist: &[Pt], wfun: &dyn Fn(f64) -> f64) -> (Vec<Pt>, Vec<Pt>) {
    // the original draws (and discards) a value here; keep the stream aligned
    let _ = rand();
    let mut v0: Vec<Pt> = Vec::new();
    let mut v1: Vec<Pt> = Vec::new();
    let n = ptlist.len();
    for i in 1..n.saturating_sub(1) {
        let w = wfun(i as f64 / n as f64);
        let a1 = (ptlist[i][1] - ptlist[i - 1][1]).atan2(ptlist[i][0] - ptlist[i - 1][0]);
        let a2 = (ptlist[i][1] - ptlist[i + 1][1]).atan2(ptlist[i][0] - ptlist[i + 1][0]);
        let mut a = (a1 + a2) / 2.0;
        if a < a2 {
            a += PI;
        }
        v0.push([ptlist[i][0] + w * a.cos(), ptlist[i][1] + w * a.sin()]);
        v1.push([ptlist[i][0] - w * a.cos(), ptlist[i][1] - w * a.sin()]);
    }
    let l = n - 1;
    let a0 = (ptlist[1][1] - ptlist[0][1]).atan2(ptlist[1][0] - ptlist[0][0]) - PI / 2.0;
    let a1 = (ptlist[l][1] - ptlist[l - 1][1]).atan2(ptlist[l][0] - ptlist[l - 1][0]) - PI / 2.0;
    let w0 = wfun(0.0);
    let w1 = wfun(1.0);
    v0.insert(0, [ptlist[0][0] + w0 * a0.cos(), ptlist[0][1] + w0 * a0.sin()]);
    v1.insert(0, [ptlist[0][0] - w0 * a0.cos(), ptlist[0][1] - w0 * a0.sin()]);
    v0.push([ptlist[l][0] + w1 * a1.cos(), ptlist[l][1] + w1 * a1.sin()]);
    v1.push([ptlist[l][0] - w1 * a1.cos(), ptlist[l][1] - w1 * a1.sin()]);
    (v0, v1)
}

fn tranpoly(p0: Pt, p1: Pt, ptlist: &[Pt]) -> Vec<Pt> {
    let ang = (p1[1] - p0[1]).atan2(p1[0] - p0[0]) - PI / 2.0;
    let scl = distance(p0, p1);
    ptlist
        .iter()
        .map(|v| {
            let v = [-v[0], v[1]];
            let d = distance(v, [0.0, 0.0]);
            let a = v[1].atan2(v[0]);
            [p0[0] + d * scl * (ang + a).cos(), p0[1] + d * scl * (ang + a).sin()]
        })
        .collect()
}

fn flipper(plist: &[Pt]) -> Vec<Pt> {
    plist.iter().map(|v| [-v[0], v[1]]).collect()
}

fn maybe_flip(plist: &[Pt], fli: bool) -> Vec<Pt> {
    if fli {
        flipper(plist)
    } else {
        plist.to_vec()
    }
}

pub type Deco = fn(Pt, Pt, bool) -> Canv;

pub fn hat01(p0: Pt, p1: Pt, fli: bool) -> Canv {
    let mut canv = Canv::new();
    let seed = rand();
    let base: [Pt; 7] = [
        [-0.3, 0.5],
        [0.3, 0.8],
        [0.2, 1.0],
        [0.0, 1.1],
        [-0.3, 1.15],
        [-0.55, 1.0],
        [-0.65, 0.5],
    ];
    canv.extend(polyf(&tranpoly(p0, p1, &maybe_flip(&base, fli)), Col::grey(100.0, 0.8)));
    let mut qlist1 = Vec::with_capacity(10);
    for i in 0..10 {
        qlist1.push([-0.3 - n2(i as f64 * 0.2, seed) * i as f64 * 0.1, 0.5 - i as f64 * 0.3]);
    }
    canv.extend(poly(
        &tranpoly(p0, p1, &maybe_flip(&qlist1, fli)),
        0.0,
        0.0,
        NONE,
        Col::grey(100.0, 0.8),
        1.0,
    ));
    canv
}

pub fn hat02(p0: Pt, p1: Pt, fli: bool) -> Canv {
    let _seed = rand();
    let base: [Pt; 10] = [
        [-0.3, 0.5],
        [-1.1, 0.5],
        [-1.2, 0.6],
        [-1.1, 0.7],
        [-0.3, 0.8],
        [0.3, 0.8],
        [1.0, 0.7],
        [1.3, 0.6],
        [1.2, 0.5],
        [0.3, 0.5],
    ];
    polyf(&tranpoly(p0, p1, &maybe_flip(&base, fli)), Col::grey(100.0, 0.8))
}

pub fn stick01(p0: Pt, p1: Pt, fli: bool) -> Canv {
    let seed = rand();
    let l = 12;
    let mut qlist1 = Vec::with_capacity(l);
    for i in 0..l {
        qlist1.push([
            -n2(i as f64 * 0.1, seed) * 0.1 * ((i as f64 / l as f64) * PI).sin() * 5.0,
            i as f64 * 0.3,
        ]);
    }
    poly(
        &tranpoly(p0, p1, &maybe_flip(&qlist1, fli)),
        0.0,
        0.0,
        NONE,
        Col::grey(100.0, 0.5),
        1.0,
    )
}

pub fn no_item(_p0: Pt, _p1: Pt, _fli: bool) -> Canv {
    Canv::new()
}

const PAR: [&[usize]; 9] = [
    &[0],
    &[0, 1],
    &[0, 1, 2],
    &[0, 3],
    &[0, 3, 4],
    &[0, 1, 5],
    &[0, 1, 5, 6],
    &[0, 1, 7],
    &[0, 1, 7, 8],
];

pub struct ManArgs {
    pub sca: f64,
    pub hat: Deco,
    pub ite: Deco,
    pub fli: bool,
    pub ang: Option<[f64; 9]>,
    pub len: Option<[f64; 9]>,
}

impl Default for ManArgs {
    fn default() -> Self {
        ManArgs { sca: 0.5, hat: hat01, ite: no_item, fli: true, ang: None, len: None }
    }
}

pub fn man(xoff: f64, mut yoff: f64, a: &ManArgs) -> Canv {
    let ang = a.ang.unwrap_or_else(|| [
        0.0,
        -PI / 2.0,
        norm_rand(0.0, 0.0),
        (PI / 4.0) * rand(),
        ((PI * 3.0) / 4.0) * rand(),
        (PI * 3.0) / 4.0,
        -PI / 4.0,
        (-PI * 3.0) / 4.0 - (PI / 4.0) * rand(),
        -PI / 4.0,
    ]);
    let mut len = a.len.unwrap_or([0.0, 30.0, 20.0, 30.0, 30.0, 30.0, 30.0, 30.0, 30.0]);
    for v in len.iter_mut() {
        *v *= a.sca;
    }
    let sca = a.sca;
    let grot = |ind: usize| -> f64 { PAR[ind].iter().map(|&p| ang[p]).sum() };
    let gpos = |ind: usize| -> Pt {
        let mut pos = [0.0, 0.0];
        for &p in PAR[ind] {
            let r = grot(p);
            pos[0] += len[p] * r.cos();
            pos[1] += len[p] * r.sin();
        }
        pos
    };
    let pts: Vec<Pt> = (0..9).map(gpos).collect();
    yoff -= pts[4][1];
    let fli = a.fli;
    let to_global = move |v: Pt| -> Pt { [(if fli { -1.0 } else { 1.0 }) * v[0] + xoff, v[1] + yoff] };

    let mut canv = Canv::new();
    let fsleeve = |x: f64| {
        sca * 8.0 * ((0.5 * x * PI).sin() * (x * PI).sin().powf(0.1) + (1.0 - x) * 0.4)
    };
    let fbody = |x: f64| {
        sca * 11.0 * ((0.5 * x * PI).sin() * (x * PI).sin().powf(0.1) + (1.0 - x) * 0.5)
    };
    let fhead = |x: f64| sca * 7.0 * (0.25 - (x - 0.5).powi(2)).powf(0.3);

    canv.extend((a.ite)(to_global(pts[8]), to_global(pts[6]), fli));

    let cloth = |plist: &[Pt], fun: &dyn Fn(f64) -> f64, out: &mut Canv| {
        let tlist = bezmh(plist, 2.0);
        let (t1, mut t2) = expand(&tlist, fun);
        // the original reverses tlist2 in place, so the stroke below sees it reversed
        t2.reverse();
        let mut all: Vec<Pt> = t1.iter().map(|v| to_global(*v)).collect();
        let mut r: Vec<Pt> = t2.iter().map(|v| to_global(*v)).collect();
        all.append(&mut r);
        out.extend(polyf(&all, WHITE));
        let g1: Vec<Pt> = t1.iter().map(|v| to_global(*v)).collect();
        out.extend(stroke(
            &g1,
            StrokeArgs { wid: 1.0, col: Col::grey(100.0, 0.5), ..Default::default() },
        ));
        let g2: Vec<Pt> = t2.iter().map(|v| to_global(*v)).collect();
        out.extend(stroke(
            &g2,
            StrokeArgs { wid: 1.0, col: Col::grey(100.0, 0.6), ..Default::default() },
        ));
    };

    let mut c = Canv::new();
    cloth(&[pts[1], pts[7], pts[8]], &fsleeve, &mut c);
    cloth(&[pts[1], pts[0], pts[3], pts[4]], &fbody, &mut c);
    cloth(&[pts[1], pts[5], pts[6]], &fsleeve, &mut c);
    cloth(&[pts[1], pts[2]], &fhead, &mut c);
    canv.extend(c);

    let hlist = bezmh(&[pts[1], pts[2]], 2.0);
    let (mut h1, mut h2) = expand(&hlist, &fhead);
    let d1 = crate::rng::jfloor(h1.len() as f64 * 0.1) as usize;
    h1.drain(0..d1.min(h1.len()));
    let d2 = crate::rng::jfloor(h2.len() as f64 * 0.95) as usize;
    h2.drain(0..d2.min(h2.len()));
    let mut all: Vec<Pt> = h1.iter().map(|v| to_global(*v)).collect();
    let mut r: Vec<Pt> = h2.iter().rev().map(|v| to_global(*v)).collect();
    all.append(&mut r);
    canv.extend(polyf(&all, Col::grey(100.0, 0.6)));
    canv.extend((a.hat)(to_global(pts[1]), to_global(pts[2]), fli));
    canv
}
