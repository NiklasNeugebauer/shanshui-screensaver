// Port of shan-shui-inf by Lingdong Huang (https://github.com/LingDong-/shan-shui-inf), MIT. See LICENSE.
use crate::rng::{jfloor, noise, noise2, noise3, rand};
use std::f64::consts::PI;

pub type Pt = [f64; 2];

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Col {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

pub const NONE: Col = Col { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
pub const WHITE: Col = Col { r: 255.0, g: 255.0, b: 255.0, a: 1.0 };

impl Col {
    pub const fn rgba(r: f64, g: f64, b: f64, a: f64) -> Col {
        Col { r, g, b, a }
    }
    pub const fn grey(v: f64, a: f64) -> Col {
        Col { r: v, g: v, b: v, a }
    }
}

#[derive(Clone)]
pub struct Prim {
    pub pts: Vec<Pt>,
    pub fil: Col,
    pub str_: Col,
    pub wid: f64,
}

pub type Canv = Vec<Prim>;

#[inline]
pub fn fix(v: f64, n: u32) -> f64 {
    let m = 10f64.powi(n as i32);
    (v * m).round() / m
}

pub fn midpt2(a: Pt, b: Pt) -> Pt {
    [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
}

pub fn midpt(pl: &[Pt]) -> Pt {
    let n = pl.len() as f64;
    pl.iter().fold([0.0, 0.0], |acc, v| [v[0] / n + acc[0], v[1] / n + acc[1]])
}

pub fn distance(a: Pt, b: Pt) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

pub fn mapval(value: f64, istart: f64, istop: f64, ostart: f64, ostop: f64) -> f64 {
    ostart + (ostop - ostart) * ((value - istart) / (istop - istart))
}

pub fn loop_noise(ns: &mut [f64]) {
    let n = ns.len();
    let dif = ns[n - 1] - ns[0];
    let mut bds = [100.0f64, -100.0f64];
    for (i, v) in ns.iter_mut().enumerate() {
        *v += dif * ((n - 1 - i) as f64) / ((n - 1) as f64);
        if *v < bds[0] {
            bds[0] = *v;
        }
        if *v > bds[1] {
            bds[1] = *v;
        }
    }
    for v in ns.iter_mut() {
        *v = mapval(*v, bds[0], bds[1], 0.0, 1.0);
    }
}

pub fn rand_choice<T: Copy>(arr: &[T]) -> T {
    arr[(arr.len() as f64 * rand()) as usize]
}

pub fn norm_rand(m: f64, mm: f64) -> f64 {
    mapval(rand(), 0.0, 1.0, m, mm)
}

pub fn wtrand(f: &dyn Fn(f64) -> f64) -> f64 {
    loop {
        let x = rand();
        let y = rand();
        if y < f(x) {
            return x;
        }
    }
}

pub fn rand_gaussian() -> f64 {
    wtrand(&|x: f64| std::f64::consts::E.powf(-24.0 * (x - 0.5).powi(2))) * 2.0 - 1.0
}

pub fn bezmh(p: &[Pt], w: f64) -> Vec<Pt> {
    let mut pp: Vec<Pt> = p.to_vec();
    if pp.len() == 2 {
        pp = vec![p[0], midpt2(p[0], p[1]), p[1]];
    }
    let n = pp.len();
    let mut plist = Vec::new();
    for j in 0..n.saturating_sub(2) {
        let p0 = if j == 0 { pp[j] } else { midpt2(pp[j], pp[j + 1]) };
        let p1 = pp[j + 1];
        let p2 = if j == n - 3 { pp[j + 2] } else { midpt2(pp[j + 1], pp[j + 2]) };
        let pl = 20;
        let extra = usize::from(j == n - 3);
        for i in 0..pl + extra {
            let t = i as f64 / pl as f64;
            let u = (1.0 - t).powi(2) + 2.0 * t * (1.0 - t) * w + t * t;
            plist.push([
                ((1.0 - t).powi(2) * p0[0] + 2.0 * t * (1.0 - t) * p1[0] * w + t * t * p2[0]) / u,
                ((1.0 - t).powi(2) * p0[1] + 2.0 * t * (1.0 - t) * p1[1] * w + t * t * p2[1]) / u,
            ]);
        }
    }
    plist
}

pub fn poly(plist: &[Pt], xof: f64, yof: f64, fil: Col, str_: Col, wid: f64) -> Canv {
    if plist.is_empty() {
        return Vec::new();
    }
    vec![Prim {
        pts: plist.iter().map(|p| [p[0] + xof, p[1] + yof]).collect(),
        fil,
        str_,
        wid,
    }]
}

#[inline]
pub fn polyf(plist: &[Pt], fil: Col) -> Canv {
    poly(plist, 0.0, 0.0, fil, fil, 0.0)
}

pub struct StrokeArgs<'a> {
    pub xof: f64,
    pub yof: f64,
    pub wid: f64,
    pub col: Col,
    pub noi: f64,
    pub out: f64,
    pub fun: &'a dyn Fn(f64) -> f64,
}

impl Default for StrokeArgs<'_> {
    fn default() -> Self {
        StrokeArgs {
            xof: 0.0,
            yof: 0.0,
            wid: 2.0,
            col: Col::grey(200.0, 0.9),
            noi: 0.5,
            out: 1.0,
            fun: &|x: f64| (x * PI).sin(),
        }
    }
}

pub fn stroke(ptlist: &[Pt], a: StrokeArgs) -> Canv {
    if ptlist.is_empty() {
        return Vec::new();
    }
    let n = ptlist.len();
    let mut v0: Vec<Pt> = Vec::with_capacity(n);
    let mut v1: Vec<Pt> = Vec::with_capacity(n);
    let n0 = rand() * 10.0;
    for i in 1..n.saturating_sub(1) {
        let mut w = a.wid * (a.fun)(i as f64 / n as f64);
        w = w * (1.0 - a.noi) + w * a.noi * noise2(i as f64 * 0.5, n0);
        let a1 = (ptlist[i][1] - ptlist[i - 1][1]).atan2(ptlist[i][0] - ptlist[i - 1][0]);
        let a2 = (ptlist[i][1] - ptlist[i + 1][1]).atan2(ptlist[i][0] - ptlist[i + 1][0]);
        let mut ang = (a1 + a2) / 2.0;
        if ang < a2 {
            ang += PI;
        }
        v0.push([ptlist[i][0] + w * ang.cos(), ptlist[i][1] + w * ang.sin()]);
        v1.push([ptlist[i][0] - w * ang.cos(), ptlist[i][1] - w * ang.sin()]);
    }
    let mut vtx: Vec<Pt> = Vec::with_capacity(2 * n + 2);
    vtx.push(ptlist[0]);
    vtx.extend_from_slice(&v0);
    v1.push(ptlist[n - 1]);
    v1.reverse();
    vtx.extend_from_slice(&v1);
    vtx.push(ptlist[0]);
    poly(&vtx, a.xof, a.yof, a.col, a.col, a.out)
}

pub struct BlobArgs<'a> {
    pub len: f64,
    pub wid: f64,
    pub ang: f64,
    pub col: Col,
    pub noi: f64,
    pub fun: &'a dyn Fn(f64) -> f64,
}

pub fn blob_default_fun(x: f64) -> f64 {
    if x <= 1.0 {
        (x * PI).sin().powf(0.5)
    } else {
        -(((x + 1.0) * PI).sin()).powf(0.5)
    }
}

// the leaf shape shared by tree02 and twig
pub fn blob_leaf_fun(x: f64) -> f64 {
    if x <= 1.0 {
        ((x * PI).sin() * x).powf(0.5)
    } else {
        -(((x - 2.0) * PI * (x - 2.0)).sin()).powf(0.5)
    }
}

impl Default for BlobArgs<'_> {
    fn default() -> Self {
        BlobArgs {
            len: 20.0,
            wid: 5.0,
            ang: 0.0,
            col: Col::grey(200.0, 0.9),
            noi: 0.5,
            fun: &blob_default_fun,
        }
    }
}

pub fn blob_pts(x: f64, y: f64, a: &BlobArgs) -> Vec<Pt> {
    let reso = 20.0;
    let n = reso as usize + 1;
    let mut lalist = Vec::with_capacity(n);
    for i in 0..n {
        let p = (i as f64 / reso) * 2.0;
        let xo = a.len / 2.0 - (p - 1.0).abs() * a.len;
        let yo = ((a.fun)(p) * a.wid) / 2.0;
        lalist.push([yo.atan2(xo), (xo * xo + yo * yo).sqrt()]);
    }
    let mut nslist = Vec::with_capacity(n);
    let n0 = rand() * 10.0;
    for i in 0..n {
        nslist.push(noise2(i as f64 * 0.05, n0));
    }
    loop_noise(&mut nslist);
    let mut plist = Vec::with_capacity(n);
    for i in 0..n {
        let ns = nslist[i] * a.noi + (1.0 - a.noi);
        let (ang, l) = (lalist[i][0], lalist[i][1]);
        plist.push([x + (ang + a.ang).cos() * l * ns, y + (ang + a.ang).sin() * l * ns]);
    }
    plist
}

pub fn blob(x: f64, y: f64, a: &BlobArgs) -> Canv {
    let plist = blob_pts(x, y, a);
    poly(&plist, 0.0, 0.0, a.col, a.col, 0.0)
}

pub fn div(plist: &[Pt], reso: f64) -> Vec<Pt> {
    let tl = (plist.len() as f64 - 1.0) * reso;
    let mut rlist = Vec::new();
    let mut i = 0.0f64;
    while i < tl {
        let lastp = plist[jfloor(i / reso) as usize];
        let nextp = plist[(i / reso).ceil() as usize];
        let p = (i % reso) / reso;
        rlist.push([lastp[0] * (1.0 - p) + nextp[0] * p, lastp[1] * (1.0 - p) + nextp[1] * p]);
        i += 1.0;
    }
    if !plist.is_empty() {
        rlist.push(plist[plist.len() - 1]);
    }
    rlist
}

pub struct TexArgs<'a> {
    pub xof: f64,
    pub yof: f64,
    pub tex: usize,
    pub wid: f64,
    pub len: f64,
    pub sha: f64,
    pub noi: &'a dyn Fn(f64) -> f64,
    pub col: &'a dyn Fn(f64) -> Col,
    pub dis: &'a dyn Fn() -> f64,
}

pub fn tex_default_col(_x: f64) -> Col {
    Col::grey(100.0, fix(rand() * 0.3, 3))
}
pub fn tex_default_dis() -> f64 {
    if rand() > 0.5 {
        (1.0 / 3.0) * rand()
    } else {
        2.0 / 3.0 + (1.0 / 3.0) * rand()
    }
}
pub fn tex_default_noi(x: f64) -> f64 {
    30.0 / x
}

impl Default for TexArgs<'_> {
    fn default() -> Self {
        TexArgs {
            xof: 0.0,
            yof: 0.0,
            tex: 400,
            wid: 1.5,
            len: 0.2,
            sha: 0.0,
            noi: &tex_default_noi,
            col: &tex_default_col,
            dis: &tex_default_dis,
        }
    }
}

pub fn texture_pts(ptlist: &[Vec<Pt>], a: &TexArgs) -> Vec<Vec<Pt>> {
    let reso = [ptlist.len(), ptlist[0].len()];
    let mut texlist: Vec<Vec<Pt>> = Vec::with_capacity(a.tex);
    for i in 0..a.tex {
        let mid = ((a.dis)() * reso[1] as f64) as i64;
        let hlen = jfloor(rand() * (reso[1] as f64 * a.len)) as i64;
        let start = (mid - hlen).clamp(0, reso[1] as i64);
        let end = (mid + hlen).clamp(0, reso[1] as i64);
        let layer = (i as f64 / a.tex as f64) * (reso[0] as f64 - 1.0);
        let mut cur = Vec::new();
        let (lo, hi) = (jfloor(layer) as usize, layer.ceil() as usize);
        for j in start..end {
            let j = j as usize;
            let p = layer - jfloor(layer);
            let x = ptlist[lo][j][0] * p + ptlist[hi][j][0] * (1.0 - p);
            let y = ptlist[lo][j][1] * p + ptlist[hi][j][1] * (1.0 - p);
            let nx = (a.noi)(layer + 1.0) * (noise2(x, j as f64 * 0.5) - 0.5);
            let ny = (a.noi)(layer + 1.0) * (noise2(y, j as f64 * 0.5) - 0.5);
            cur.push([x + nx, y + ny]);
        }
        texlist.push(cur);
    }
    texlist
}

pub fn texture(ptlist: &[Vec<Pt>], a: &TexArgs) -> Canv {
    let texlist = texture_pts(ptlist, a);
    let mut canv = Canv::new();
    let shaded = a.sha != 0.0;
    if shaded {
        let mut j = 0;
        while j < texlist.len() {
            let pts: Vec<Pt> = texlist[j].iter().map(|x| [x[0] + a.xof, x[1] + a.yof]).collect();
            canv.extend(stroke(
                &pts,
                StrokeArgs { col: Col::grey(100.0, 0.1), wid: a.sha, ..Default::default() },
            ));
            j += 2;
        }
    }
    let step = 1 + a.sha as usize;
    let mut j = a.sha as usize;
    while j < texlist.len() {
        let pts: Vec<Pt> = texlist[j].iter().map(|x| [x[0] + a.xof, x[1] + a.yof]).collect();
        canv.extend(stroke(
            &pts,
            StrokeArgs {
                col: (a.col)(j as f64 / texlist.len() as f64),
                wid: a.wid,
                ..Default::default()
            },
        ));
        j += step;
    }
    canv
}

// ---- PolyTools::triangulate ----

fn line_expr(p0: Pt, p1: Pt) -> [f64; 2] {
    let den = p1[0] - p0[0];
    let m = if den == 0.0 { f64::INFINITY } else { (p1[1] - p0[1]) / den };
    [m, p0[1] - m * p0[0]]
}

fn on_seg(p: Pt, ln: [Pt; 2]) -> bool {
    ln[0][0].min(ln[1][0]) <= p[0]
        && p[0] <= ln[0][0].max(ln[1][0])
        && ln[0][1].min(ln[1][1]) <= p[1]
        && p[1] <= ln[0][1].max(ln[1][1])
}

fn intersect(ln0: [Pt; 2], ln1: [Pt; 2]) -> Option<Pt> {
    let le0 = line_expr(ln0[0], ln0[1]);
    let le1 = line_expr(ln1[0], ln1[1]);
    let den = le0[0] - le1[0];
    if den == 0.0 || den.is_nan() {
        return None;
    }
    let x = (le1[1] - le0[1]) / den;
    let y = le0[0] * x + le0[1];
    if on_seg([x, y], ln0) && on_seg([x, y], ln1) {
        Some([x, y])
    } else {
        None
    }
}

fn pt_in_poly(pt: Pt, plist: &[Pt]) -> bool {
    let mut sc = 0;
    for i in 0..plist.len() {
        let np = plist[if i != plist.len() - 1 { i + 1 } else { 0 }];
        if intersect([plist[i], np], [pt, [pt[0] + 999.0, pt[1] + 999.0]]).is_some() {
            sc += 1;
        }
    }
    sc % 2 == 1
}

fn ln_in_poly(ln: [Pt; 2], plist: &[Pt]) -> bool {
    let ep = 0.01;
    let lnc = [
        [ln[0][0] * (1.0 - ep) + ln[1][0] * ep, ln[0][1] * (1.0 - ep) + ln[1][1] * ep],
        [ln[0][0] * ep + ln[1][0] * (1.0 - ep), ln[0][1] * ep + ln[1][1] * (1.0 - ep)],
    ];
    for i in 0..plist.len() {
        let np = plist[if i != plist.len() - 1 { i + 1 } else { 0 }];
        if intersect(lnc, [plist[i], np]).is_some() {
            return false;
        }
    }
    pt_in_poly(midpt(&ln), plist)
}

fn sides_of(plist: &[Pt]) -> Vec<f64> {
    (0..plist.len())
        .map(|i| {
            let np = plist[if i != plist.len() - 1 { i + 1 } else { 0 }];
            ((np[0] - plist[i][0]).powi(2) + (np[1] - plist[i][1]).powi(2)).sqrt()
        })
        .collect()
}

fn area_of(plist: &[Pt]) -> f64 {
    let s = sides_of(plist);
    if s.len() < 3 {
        return 0.0;
    }
    let (a, b, c) = (s[0], s[1], s[2]);
    let p = (a + b + c) / 2.0;
    (p * (p - a) * (p - b) * (p - c)).sqrt()
}

fn sliver_ratio(plist: &[Pt]) -> f64 {
    area_of(plist) / sides_of(plist).iter().sum::<f64>()
}

fn shatter(plist: &[Pt], a: f64, out: &mut Vec<Vec<Pt>>) {
    if plist.is_empty() {
        return;
    }
    if !(area_of(plist) >= a) {
        out.push(plist.to_vec());
        return;
    }
    let slist = sides_of(plist);
    let mut ind = 0;
    for i in 0..slist.len() {
        if slist[i] > slist[ind] {
            ind = i;
        }
    }
    let nind = (ind + 1) % plist.len();
    let lind = (ind + 2) % plist.len();
    let mid = midpt2(plist[ind], plist[nind]);
    shatter(&[plist[ind], mid, plist[lind]], a, out);
    shatter(&[plist[lind], plist[nind], mid], a, out);
}

pub fn triangulate(plist: &[Pt], area: f64, convex: bool, optimize: bool) -> Vec<Vec<Pt>> {
    let mut out = Vec::new();
    tri_rec(plist, area, convex, optimize, &mut out, 0);
    out
}

fn tri_rec(plist: &[Pt], area: f64, convex: bool, optimize: bool, out: &mut Vec<Vec<Pt>>, depth: u32) {
    if depth > 256 {
        return;
    }
    if plist.len() <= 3 {
        shatter(plist, area, out);
        return;
    }
    // bestEar
    let mut cuts: Vec<(Vec<Pt>, Vec<Pt>)> = Vec::new();
    let mut quick: Option<(Vec<Pt>, Vec<Pt>)> = None;
    for i in 0..plist.len() {
        let pt = plist[i];
        let lp = plist[if i != 0 { i - 1 } else { plist.len() - 1 }];
        let np = plist[if i != plist.len() - 1 { i + 1 } else { 0 }];
        let mut qlist = plist.to_vec();
        qlist.remove(i);
        if convex || ln_in_poly([lp, np], plist) {
            let c = (vec![lp, pt, np], qlist);
            if !optimize {
                quick = Some(c);
                break;
            }
            cuts.push(c);
        }
    }
    let best = if let Some(q) = quick {
        q
    } else {
        let mut best = (plist.to_vec(), Vec::new());
        let mut best_ratio = 0.0;
        for c in cuts {
            let r = sliver_ratio(&c.0);
            if r >= best_ratio {
                best_ratio = r;
                best = c;
            }
        }
        best
    };
    shatter(&best.0, area, out);
    if !best.1.is_empty() {
        tri_rec(&best.1, area, convex, optimize, out, depth + 1);
    }
}

pub fn n(x: f64) -> f64 {
    noise(x)
}
pub fn n2(x: f64, y: f64) -> f64 {
    noise2(x, y)
}
pub fn n3(x: f64, y: f64, z: f64) -> f64 {
    noise3(x, y, z)
}
