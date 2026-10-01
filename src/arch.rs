// Port of shan-shui-inf by Lingdong Huang (https://github.com/LingDong-/shan-shui-inf), MIT. See LICENSE.
use crate::draw::*;
use crate::man::{self, ManArgs};
use crate::rng::rand;
use std::f64::consts::PI;

fn flip(ptlist: &mut [Vec<Pt>]) {
    for row in ptlist.iter_mut() {
        for p in row.iter_mut() {
            p[0] = -p[0];
        }
    }
}

fn hut(xoff: f64, yoff: f64, hei: f64, wid: f64, tex: usize) -> Canv {
    let reso = [10usize, 10usize];
    let mut ptlist: Vec<Vec<Pt>> = Vec::with_capacity(reso[0]);
    for i in 0..reso[0] {
        let mut row = Vec::with_capacity(reso[1]);
        let heir = hei + hei * 0.2 * rand();
        for j in 0..reso[1] {
            let nx = wid * (i as f64 / (reso[0] - 1) as f64 - 0.5)
                * (j as f64 / (reso[1] - 1) as f64).powf(0.7);
            let ny = heir * (j as f64 / (reso[1] - 1) as f64);
            row.push([nx, ny]);
        }
        ptlist.push(row);
    }
    let mut canv = Canv::new();
    let mut outline: Vec<Pt> = ptlist[0][..reso[1] - 1].to_vec();
    let mut back: Vec<Pt> = ptlist[reso[0] - 1][..reso[1] - 1].to_vec();
    back.reverse();
    outline.extend(back);
    canv.extend(poly(&outline, xoff, yoff, WHITE, NONE, 0.0));
    canv.extend(poly(&ptlist[0], xoff, yoff, NONE, Col::grey(100.0, 0.3), 2.0));
    canv.extend(poly(&ptlist[reso[0] - 1], xoff, yoff, NONE, Col::grey(100.0, 0.3), 2.0));
    canv.extend(texture(
        &ptlist,
        &TexArgs {
            xof: xoff,
            yof: yoff,
            tex,
            wid: 1.0,
            len: 0.25,
            col: &|_x| Col::grey(120.0, fix(0.3 + rand() * 0.3, 3)),
            dis: &|| wtrand(&|a: f64| a * a),
            noi: &|_x| 5.0,
            ..Default::default()
        },
    ));
    canv
}

pub struct BoxArgs<'a> {
    pub hei: f64,
    pub wid: f64,
    pub rot: f64,
    pub per: f64,
    pub tra: bool,
    pub bot: bool,
    pub wei: f64,
    pub dec: &'a dyn Fn(Pt, Pt, Pt, Pt) -> Vec<Vec<Pt>>,
}

impl Default for BoxArgs<'_> {
    fn default() -> Self {
        BoxArgs {
            hei: 20.0,
            wid: 120.0,
            rot: 0.7,
            per: 4.0,
            tra: true,
            bot: true,
            wei: 3.0,
            dec: &|_a, _b, _c, _d| Vec::new(),
        }
    }
}

fn boxx(xoff: f64, yoff: f64, a: &BoxArgs) -> Canv {
    let (hei, wid, rot, per) = (a.hei, a.wid, a.rot, a.per);
    let mid = -wid * 0.5 + wid * rot;
    let bmid = -wid * 0.5 + wid * (1.0 - rot);
    let mut ptlist: Vec<Vec<Pt>> = Vec::new();
    ptlist.push(div(&[[-wid * 0.5, -hei], [-wid * 0.5, 0.0]], 5.0));
    ptlist.push(div(&[[wid * 0.5, -hei], [wid * 0.5, 0.0]], 5.0));
    if a.bot {
        ptlist.push(div(&[[-wid * 0.5, 0.0], [mid, per]], 5.0));
        ptlist.push(div(&[[wid * 0.5, 0.0], [mid, per]], 5.0));
    }
    ptlist.push(div(&[[mid, -hei], [mid, per]], 5.0));
    if a.tra {
        if a.bot {
            ptlist.push(div(&[[-wid * 0.5, 0.0], [bmid, -per]], 5.0));
            ptlist.push(div(&[[wid * 0.5, 0.0], [bmid, -per]], 5.0));
        }
        ptlist.push(div(&[[bmid, -hei], [bmid, -per]], 5.0));
    }
    let surf = if rot < 0.5 { 1.0 } else { -1.0 };
    ptlist.extend((a.dec)(
        [surf * wid * 0.5, -hei],
        [mid, -hei + per],
        [surf * wid * 0.5, 0.0],
        [mid, per],
    ));
    let polist: Vec<Pt> = vec![
        [-wid * 0.5, -hei],
        [wid * 0.5, -hei],
        [wid * 0.5, 0.0],
        [mid, per],
        [-wid * 0.5, 0.0],
    ];
    let mut canv = Canv::new();
    if !a.tra {
        canv.extend(poly(&polist, xoff, yoff, WHITE, NONE, 0.0));
    }
    for row in &ptlist {
        let pts: Vec<Pt> = row.iter().map(|x| [x[0] + xoff, x[1] + yoff]).collect();
        canv.extend(stroke(
            &pts,
            StrokeArgs {
                col: Col::grey(100.0, 0.4),
                noi: 1.0,
                wid: a.wei,
                fun: &|_x: f64| 1.0,
                ..Default::default()
            },
        ));
    }
    canv
}

fn deco(style: u32, pul: Pt, pur: Pt, pdl: Pt, pdr: Pt, hsp: [usize; 2], vsp: [usize; 2]) -> Vec<Vec<Pt>> {
    let mut plist = Vec::new();
    let dl = div(&[pul, pdl], vsp[1] as f64);
    let dr = div(&[pur, pdr], vsp[1] as f64);
    let du = div(&[pul, pur], hsp[1] as f64);
    let dd = div(&[pdl, pdr], hsp[1] as f64);
    if style == 1 || style == 3 {
        let mlu = du[hsp[0]];
        let mru = du[du.len() - 1 - hsp[0]];
        let mld = dd[hsp[0]];
        let mrd = dd[du.len() - 1 - hsp[0]];
        let mut i = vsp[0];
        while i < dl.len() - vsp[0] {
            let mml = div(&[mlu, mld], vsp[1] as f64)[i];
            let mmr = div(&[mru, mrd], vsp[1] as f64)[i];
            if style == 1 {
                plist.push(div(&[mml, dl[i]], 5.0));
                plist.push(div(&[mmr, dr[i]], 5.0));
            } else {
                let mmu = div(&[mlu, mru], vsp[1] as f64)[i];
                let mmd = div(&[mld, mrd], vsp[1] as f64)[i];
                plist.push(div(&[mml, mmr], 5.0));
                plist.push(div(&[mmu, mmd], 5.0));
            }
            i += vsp[0];
        }
        plist.push(div(&[mlu, mld], 5.0));
        plist.push(div(&[mru, mrd], 5.0));
    } else if style == 2 {
        let mut i = hsp[0];
        while i < du.len() - hsp[0] {
            plist.push(div(&[du[i], dd[i]], 5.0));
            i += hsp[0];
        }
    }
    plist
}

pub struct RailArgs {
    pub hei: f64,
    pub wid: f64,
    pub rot: f64,
    pub per: f64,
    pub seg: f64,
    pub wei: f64,
    pub tra: bool,
    pub fro: bool,
}

impl Default for RailArgs {
    fn default() -> Self {
        RailArgs { hei: 20.0, wid: 180.0, rot: 0.7, per: 4.0, seg: 4.0, wei: 1.0, tra: true, fro: true }
    }
}

fn rail(xoff: f64, yoff: f64, seed: f64, a: &RailArgs) -> Canv {
    let (hei, wid, rot, per, seg) = (a.hei, a.wid, a.rot, a.per, a.seg);
    let mid = -wid * 0.5 + wid * rot;
    let bmid = -wid * 0.5 + wid * (1.0 - rot);
    let mut ptlist: Vec<Vec<Pt>> = Vec::new();
    if a.fro {
        ptlist.push(div(&[[-wid * 0.5, 0.0], [mid, per]], seg));
        ptlist.push(div(&[[mid, per], [wid * 0.5, 0.0]], seg));
    }
    if a.tra {
        ptlist.push(div(&[[-wid * 0.5, 0.0], [bmid, -per]], seg));
        ptlist.push(div(&[[bmid, -per], [wid * 0.5, 0.0]], seg));
    }
    if a.fro {
        ptlist.push(div(&[[-wid * 0.5, -hei], [mid, -hei + per]], seg));
        ptlist.push(div(&[[mid, -hei + per], [wid * 0.5, -hei]], seg));
    }
    if a.tra {
        ptlist.push(div(&[[-wid * 0.5, -hei], [bmid, -hei - per]], seg));
        ptlist.push(div(&[[bmid, -hei - per], [wid * 0.5, -hei]], seg));
    }
    if ptlist.is_empty() {
        return Canv::new();
    }
    if a.tra {
        let open = crate::rng::jfloor(rand() * ptlist.len() as f64) as usize;
        if !ptlist[open].is_empty() {
            ptlist[open].pop();
        }
        if !ptlist[open].is_empty() {
            ptlist[open].pop();
        }
    }
    let mut canv = Canv::new();
    let half = ptlist.len() / 2;
    for i in 0..half {
        for j in 0..ptlist[i].len() {
            ptlist[i][j][1] += (n3(i as f64, j as f64 * 0.5, seed) - 0.5) * hei;
            let k = (half + i) % ptlist.len();
            let jj = j % ptlist[k].len().max(1);
            if ptlist[k].is_empty() {
                continue;
            }
            ptlist[k][jj][1] += (n3(i as f64 + 0.5, j as f64 * 0.5, seed) - 0.5) * hei;
            let mut ln = div(&[ptlist[i][j], ptlist[k][jj]], 2.0);
            ln[0][0] += (rand() - 0.5) * hei * 0.5;
            canv.extend(poly(&ln, xoff, yoff, NONE, Col::grey(100.0, 0.5), 2.0));
        }
    }
    for row in &ptlist {
        let pts: Vec<Pt> = row.iter().map(|x| [x[0] + xoff, x[1] + yoff]).collect();
        canv.extend(stroke(
            &pts,
            StrokeArgs {
                col: Col::grey(100.0, 0.5),
                noi: 0.5,
                wid: a.wei,
                fun: &|_x: f64| 1.0,
                ..Default::default()
            },
        ));
    }
    canv
}

pub struct RoofArgs {
    pub hei: f64,
    pub wid: f64,
    pub rot: f64,
    pub per: f64,
    pub cor: f64,
    pub wei: f64,
}

impl Default for RoofArgs {
    fn default() -> Self {
        RoofArgs { hei: 20.0, wid: 120.0, rot: 0.7, per: 4.0, cor: 5.0, wei: 3.0 }
    }
}

fn roof(xoff: f64, yoff: f64, a: &RoofArgs) -> Canv {
    let (hei, wid, rot, per, cor) = (a.hei, a.wid, a.rot, a.per, a.cor);
    let doflip = rot < 0.5;
    let rrot = if rot < 0.5 { 1.0 - rot } else { rot };
    let mid = -wid * 0.5 + wid * rrot;
    let quat = (mid + wid * 0.5) * 0.5 - mid;
    let mut groups: Vec<Vec<Pt>> = vec![
        vec![
            [-wid * 0.5 + quat, -hei - per / 2.0],
            [-wid * 0.5 + quat * 0.5, -hei / 2.0 - per / 4.0],
            [-wid * 0.5 - cor, 0.0],
        ],
        vec![
            [mid + quat, -hei],
            [(mid + quat + wid * 0.5) / 2.0, -hei / 2.0],
            [wid * 0.5 + cor, 0.0],
        ],
        vec![[mid + quat, -hei], [mid + quat / 2.0, -hei / 2.0 + per / 2.0], [mid + cor, per]],
        vec![[-wid * 0.5 - cor, 0.0], [mid + cor, per]],
        vec![[wid * 0.5 + cor, 0.0], [mid + cor, per]],
        vec![[-wid * 0.5 + quat, -hei - per / 2.0], [mid + quat, -hei]],
    ];
    let mut polist: Vec<Vec<Pt>> = vec![vec![
        [-wid * 0.5, 0.0],
        [-wid * 0.5 + quat, -hei - per / 2.0],
        [mid + quat, -hei],
        [wid * 0.5, 0.0],
        [mid, per],
    ]];
    if doflip {
        flip(&mut groups);
        flip(&mut polist);
    }
    let mut canv = poly(&polist[0], xoff, yoff, WHITE, NONE, 0.0);
    for g in &groups {
        let d = div(g, 5.0);
        let pts: Vec<Pt> = d.iter().map(|x| [x[0] + xoff, x[1] + yoff]).collect();
        canv.extend(stroke(
            &pts,
            StrokeArgs {
                col: Col::grey(100.0, 0.4),
                noi: 1.0,
                wid: a.wei,
                fun: &|_x: f64| 1.0,
                ..Default::default()
            },
        ));
    }
    canv
}

fn pagroof(xoff: f64, yoff: f64, hei: f64, wid: f64, per: f64, cor: f64, sid: usize, wei: f64) -> Canv {
    let mut ptlist: Vec<Vec<Pt>> = Vec::new();
    let mut polist: Vec<Pt> = vec![[0.0, -hei]];
    let mut canv = Canv::new();
    for i in 0..sid {
        let t = i as f64 / (sid - 1) as f64;
        let fx = wid * (t - 0.5);
        let fy = per * (1.0 - (t - 0.5).abs() * 2.0);
        let fxx = (wid + cor) * (t - 0.5);
        if i > 0 {
            let prev = ptlist[ptlist.len() - 1][2];
            ptlist.push(vec![prev, [fxx, fy]]);
        }
        ptlist.push(vec![[0.0, -hei], [fx * 0.5, (-hei + fy) * 0.5], [fxx, fy]]);
        polist.push([fxx, fy]);
    }
    canv.extend(poly(&polist, xoff, yoff, WHITE, NONE, 0.0));
    for row in &ptlist {
        let d = div(row, 5.0);
        let pts: Vec<Pt> = d.iter().map(|x| [x[0] + xoff, x[1] + yoff]).collect();
        canv.extend(stroke(
            &pts,
            StrokeArgs {
                col: Col::grey(100.0, 0.4),
                noi: 1.0,
                wid: wei,
                fun: &|_x: f64| 1.0,
                ..Default::default()
            },
        ));
    }
    canv
}

pub fn arch01(xoff: f64, yoff: f64, seed: f64, wid: f64, hei: f64, per: f64) -> Canv {
    let p = 0.4 + rand() * 0.2;
    let h0 = hei * p;
    let h1 = hei * (1.0 - p);
    let mut canv = Canv::new();
    canv.extend(hut(xoff, yoff - hei, h0, wid, 300));
    canv.extend(boxx(
        xoff,
        yoff,
        &BoxArgs { hei: h1, wid: (wid * 2.0) / 3.0, per, bot: false, ..Default::default() },
    ));
    canv.extend(rail(
        xoff,
        yoff,
        seed,
        &RailArgs {
            tra: true,
            fro: false,
            hei: 10.0,
            wid,
            per: per * 2.0,
            seg: (3.0 + rand() * 3.0) as i64 as f64,
            ..Default::default()
        },
    ));
    let mcnt = rand_choice(&[0, 1, 1, 2]);
    if mcnt == 1 {
        canv.extend(man::man(
            xoff + norm_rand(-wid / 3.0, wid / 3.0),
            yoff,
            &ManArgs { fli: rand_choice(&[true, false]), sca: 0.42, ..Default::default() },
        ));
    } else if mcnt == 2 {
        canv.extend(man::man(
            xoff + norm_rand(-wid / 4.0, -wid / 5.0),
            yoff,
            &ManArgs { fli: false, sca: 0.42, ..Default::default() },
        ));
        canv.extend(man::man(
            xoff + norm_rand(wid / 5.0, wid / 4.0),
            yoff,
            &ManArgs { fli: true, sca: 0.42, ..Default::default() },
        ));
    }
    canv.extend(rail(
        xoff,
        yoff,
        seed,
        &RailArgs {
            tra: false,
            fro: true,
            hei: 10.0,
            wid,
            per: per * 2.0,
            seg: (3.0 + rand() * 3.0) as i64 as f64,
            ..Default::default()
        },
    ));
    canv
}

pub fn arch02(xoff: f64, yoff: f64, _seed: f64, hei: f64, wid: f64, rot: f64, per: f64, sto: usize, sty: u32, rai: bool) -> Canv {
    let mut canv = Canv::new();
    let mut hoff = 0.0;
    let hsp: [usize; 2] = [[1, 5], [1, 5], [1, 4]][(sty as usize).saturating_sub(1).min(2)];
    let vsp: [usize; 2] = [[1, 2], [1, 2], [1, 3]][(sty as usize).saturating_sub(1).min(2)];
    for i in 0..sto {
        canv.extend(boxx(
            xoff,
            yoff - hoff,
            &BoxArgs {
                tra: false,
                hei,
                wid: wid * 0.85f64.powi(i as i32),
                rot,
                wei: 1.5,
                per,
                bot: true,
                dec: &|pul, pur, pdl, pdr| deco(sty, pul, pur, pdl, pdr, hsp, vsp),
            },
        ));
        if rai {
            canv.extend(rail(
                xoff,
                yoff - hoff,
                i as f64 * 0.2,
                &RailArgs {
                    wid: wid * 0.85f64.powi(i as i32) * 1.1,
                    hei: hei / 2.0,
                    per,
                    rot,
                    wei: 0.5,
                    tra: false,
                    ..Default::default()
                },
            ));
        }
        if sto == 1 {
            let _ = rand();
        }
        canv.extend(roof(
            xoff,
            yoff - hoff - hei,
            &RoofArgs { hei, wid: wid * 0.9f64.powi(i as i32), rot, wei: 1.5, per, ..Default::default() },
        ));
        hoff += hei * 1.5;
    }
    canv
}

pub fn arch03(xoff: f64, yoff: f64, hei: f64, wid: f64, rot: f64, per: f64, sto: usize) -> Canv {
    let mut canv = Canv::new();
    let mut hoff = 0.0;
    for i in 0..sto {
        canv.extend(boxx(
            xoff,
            yoff - hoff,
            &BoxArgs {
                tra: false,
                hei,
                wid: wid * 0.85f64.powi(i as i32),
                rot,
                wei: 1.5,
                per: per / 2.0,
                bot: true,
                dec: &|pul, pur, pdl, pdr| deco(1, pul, pur, pdl, pdr, [1, 4], [1, 2]),
            },
        ));
        canv.extend(rail(
            xoff,
            yoff - hoff,
            i as f64 * 0.2,
            &RailArgs {
                seg: 5.0,
                wid: wid * 0.85f64.powi(i as i32) * 1.1,
                hei: hei / 2.0,
                per: per / 2.0,
                rot,
                wei: 0.5,
                tra: false,
                ..Default::default()
            },
        ));
        canv.extend(pagroof(
            xoff,
            yoff - hoff - hei,
            hei * 1.5,
            wid * 0.9f64.powi(i as i32),
            per,
            10.0,
            4,
            1.5,
        ));
        hoff += hei * 1.5;
    }
    canv
}

pub fn arch04(xoff: f64, yoff: f64, hei: f64, wid: f64, rot: f64, per: f64, sto: usize) -> Canv {
    let mut canv = Canv::new();
    let mut hoff = 0.0;
    for i in 0..sto {
        canv.extend(boxx(
            xoff,
            yoff - hoff,
            &BoxArgs {
                tra: true,
                hei,
                wid: wid * 0.85f64.powi(i as i32),
                rot,
                wei: 1.5,
                per: per / 2.0,
                ..Default::default()
            },
        ));
        canv.extend(rail(
            xoff,
            yoff - hoff,
            i as f64 * 0.2,
            &RailArgs {
                seg: 3.0,
                wid: wid * 0.85f64.powi(i as i32) * 1.2,
                hei: hei / 3.0,
                per: per / 2.0,
                rot,
                wei: 0.5,
                tra: true,
                ..Default::default()
            },
        ));
        canv.extend(pagroof(
            xoff,
            yoff - hoff - hei,
            hei,
            wid * 0.9f64.powi(i as i32),
            per,
            10.0,
            4,
            1.5,
        ));
        hoff += hei * 1.2;
    }
    canv
}

pub fn boat01(xoff: f64, yoff: f64, len: f64, sca: f64, fli: bool) -> Canv {
    let mut canv = Canv::new();
    let dir = if fli { -1.0 } else { 1.0 };
    canv.extend(man::man(
        xoff + 20.0 * sca * dir,
        yoff,
        &ManArgs {
            ite: man::stick01,
            hat: man::hat02,
            sca: 0.5 * sca,
            fli: !fli,
            len: Some([0.0, 30.0, 20.0, 30.0, 10.0, 30.0, 30.0, 30.0, 30.0]),
            ..Default::default()
        },
    ));
    let mut plist1 = Vec::new();
    let mut plist2 = Vec::new();
    let f1 = |x: f64| (x * PI).sin().powf(0.5) * 7.0 * sca;
    let f2 = |x: f64| (x * PI).sin().powf(0.5) * 10.0 * sca;
    let mut i = 0.0;
    while i < len * sca {
        plist1.push([i * dir, f1(i / len)]);
        plist2.push([i * dir, f2(i / len)]);
        i += 5.0 * sca;
    }
    plist2.reverse();
    let mut plist = plist1;
    plist.extend(plist2);
    canv.extend(poly(&plist, xoff, yoff, WHITE, WHITE, 0.0));
    let pts: Vec<Pt> = plist.iter().map(|v| [xoff + v[0], yoff + v[1]]).collect();
    canv.extend(stroke(
        &pts,
        StrokeArgs {
            wid: 1.0,
            fun: &|x: f64| (x * PI * 2.0).sin(),
            col: Col::grey(100.0, 0.4),
            ..Default::default()
        },
    ));
    canv
}

pub fn transmission_tower01(xoff: f64, yoff: f64, hei: f64, wid: f64) -> Canv {
    let mut canv = Canv::new();
    let qs = |pl: &[Pt], canv: &mut Canv| {
        let d = div(pl, 5.0);
        let pts: Vec<Pt> = d.iter().map(|v| [v[0] + xoff, v[1] + yoff]).collect();
        canv.extend(stroke(
            &pts,
            StrokeArgs {
                wid: 1.0,
                fun: &|_x: f64| 0.5,
                col: Col::grey(100.0, 0.4),
                ..Default::default()
            },
        ));
    };
    let p00 = [-wid * 0.05, -hei];
    let p01 = [wid * 0.05, -hei];
    let p10 = [-wid * 0.1, -hei * 0.9];
    let p11 = [wid * 0.1, -hei * 0.9];
    let p20 = [-wid * 0.2, -hei * 0.5];
    let p21 = [wid * 0.2, -hei * 0.5];
    let p30 = [-wid * 0.5, 0.0];
    let p31 = [wid * 0.5, 0.0];
    let bch = [[0.7, -0.85], [1.0, -0.675], [0.7, -0.5]];
    for b in &bch {
        qs(&[[-b[0] * wid, b[1] * hei], [b[0] * wid, b[1] * hei]], &mut canv);
        qs(&[[-b[0] * wid, b[1] * hei], [0.0, (b[1] - 0.05) * hei]], &mut canv);
        qs(&[[b[0] * wid, b[1] * hei], [0.0, (b[1] - 0.05) * hei]], &mut canv);
        qs(&[[-b[0] * wid, b[1] * hei], [-b[0] * wid, (b[1] + 0.1) * hei]], &mut canv);
        qs(&[[b[0] * wid, b[1] * hei], [b[0] * wid, (b[1] + 0.1) * hei]], &mut canv);
    }
    let l10 = div(&[p00, p10, p20, p30], 5.0);
    let l11 = div(&[p01, p11, p21, p31], 5.0);
    for i in 0..l10.len() - 1 {
        qs(&[l10[i], l11[i + 1]], &mut canv);
        qs(&[l11[i], l10[i + 1]], &mut canv);
    }
    qs(&[p00, p01], &mut canv);
    qs(&[p10, p11], &mut canv);
    qs(&[p20, p21], &mut canv);
    qs(&[p00, p10, p20, p30], &mut canv);
    qs(&[p01, p11, p21, p31], &mut canv);
    canv
}
