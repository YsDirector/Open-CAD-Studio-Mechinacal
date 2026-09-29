//! 第五批（partgen_b5）：两个螺钉族 —— **GB/T 70.2-2015 内六角平圆头螺钉**、
//! **GB/T 2671.1-2017 内六角花形低圆柱头螺钉**。
//!
//! 本文件只放这两个族的画法与数据；共用图元/表/校验工具在 `partgen_kit.rs`。
//! 反解全记录见 `~/桌面/OCSM/review/内六角螺钉_几何反解.md` / `内六角螺钉_校验.md`。
//!
//! | 族 id | 名称 | 模板（用户参数化图 DXF） | 视图 | 数据 |
//! |---|---|---|---|---|
//! | `socket_button_702` | 内六角平圆头螺钉 | `~/桌面/GB/参数化/内六角平圆头螺钉_GB-T702-2015/`（主/左） | main/end | 用户 PNG 表（O. d P a da dk e k s t l/2提示） |
//! | `socket_torx_2671` | 内六角花形低圆柱头螺钉 | `~/桌面/GB/参数化/内六角花形低圆柱头螺钉_GB-T2671.1-2017/`（主/左） | main/end | 用户 PNG 表（O. d b k dk l/2提示 t X A） |
//!
//! **标准号更正**：用户目录名写作 `GB-T702`，但平圆头内六角螺钉的标准号是
//! **GB/T 70.2-2015**（GB/T 702 是热轧钢棒标准）；族与树按 70.2 注册。
//!
//! **螺钉 ≠ 螺栓**（用户 2026-09-16 明确）：两族 `family_kind` = `screw`、上树在
//! 「零件库/螺钉/…」、不进螺栓副（`joint`）装配 —— 与 `partgen.rs:1080` 的
//! `socket_head`/`set_screw_77`/`eye_bolt_825` 同一类（依据先行；吊环螺钉的树路径
//! 见 `partgen_b3.rs:735`，螺钉树支见 `partgen.rs:903`）。
//!
//! # 画法（全部由模板 DXF 逐条反解，样本已作数值回归）
//!
//! ## socket_button_702（模板 M3×6）
//! - 主视图：头 = 顶面平环（半径 `rt`）+ 球冠弧。球冠过 `(0, dk/2)` 与
//!   `(−k, rt)`，半径 `r3`（背离轴线鼓起）。`rt/r3` 由数据表给出：M3 = 模板实测
//!   （1.38 / 4.41827658463912）；其余规格 `rt = e/2 + w`（标准最小壁厚）、
//!   `r3` = 2025 版 r3 上限（2015 表未给 r3，遗留见校验报告）。
//! - 内六角孔（虚线）：与 70.1 同一套公式 `hy=e/2`、`ht=s/2`、`h2=hy−ht·tan30`、
//!   孔底 `−k+t`、120° 锥尖 `−k+t+hy/tan60`。
//! - 杆：轮廓 `d/2` 到 `l−0.075d`、45° 收端倒角；牙底细实线 `0.85d/2` 从
//!   收尾起点 `a` 到 `l−0.075d`；收尾斜线从头部 `(0,d/2)` 到 `(a,0.85d/2)`。
//! - 左视图：圆 `dk/2`、`rt`、`e/2` + 六角（顶点 ±`e/2`、对边 ±`s/2`）+ 十字（出头 2）。
//!
//! ## socket_torx_2671（模板 M2×8）
//! - 主视图头部：5° 锥（标准图 `≤5°`，模板正好 5°）从基面 `(0,dk/2)` 到与顶部
//!   圆角 `r` 的切点；顶面半径解析式 `rt = dk/2 − tan5°·(k−r) − r/cos5°`。
//! - 主视图花形槽（虚线，制图展开表示）：外壁 `±L3`（`L3 = 0.485714·A`）到深度 `t`，
//!   槽底锥到 `-(k-t)+t/2`；内壁 `±L1=L3/3`，中壁 `±L2=2L3/3`；两个圆弧
//!   `Ra=0.3483966·L3`（过 `(-k,±L1)`、`(-(k-t),±L1)` 向右鼓）；口部斜线从
//!   `(-k,±L3)` 到 `(-k+(Ra−δ),±L2)`；中壁止于槽底锥交点 `-(k-t)+t/6`。
//! - 主视图杆：收尾从头部 `(0, 0.85d/2)` 到 `(2P, d/2)`、牙顶到 `l`（**平端、无倒角**）、
//!   牙底细实线 `0.85d/2` 全长；`l > b`（b=25/38）时杆端起长 `b` 为螺纹。
//! - 左视图：圆 `dk/2`、`A/2` + 虚线谷圆 `0.8·A/2` + 6 条谷弧
//!   （`rs=0.1424A`、心半径 `0.5424A`、端点落在 `A/2` 圆上 ±15°）+ 十字（出头 3）。

use std::sync::OnceLock;

use ocs_plugin_api::host::acadrust::entities::EntityType;

use crate::partgen::{GenPart, PartMeta};
use crate::partgen_kit::{
    arc, check_length, circle, line, polyline, trim, views_json, weight_kg, weight_text, Table,
    LAYER_CENTER, LAYER_HIDDEN, LAYER_MAIN, LAYER_THIN,
};

// ══════════════════════════════════════════════════════════════════════════
// 数据表
// ══════════════════════════════════════════════════════════════════════════

/// GB/T 70.2-2015 一行。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Button702Row {
    pub d: f64,
    /// 螺距
    #[serde(rename = "P")]
    pub p: f64,
    /// 螺纹收尾（模板实测 `a = 2P`）
    pub a: f64,
    /// 杆过渡直径（表列，画法不用）
    pub da: f64,
    /// 头径
    pub dk: f64,
    /// 内六角对角（min）
    pub e: f64,
    /// 头高
    pub k: f64,
    /// 内六角对边
    pub s: f64,
    /// 内六角孔深（min）
    pub t: f64,
    /// 顶部边缘圆角（min，2008 手册 r 行）
    pub r1: f64,
    /// 顶部壁厚（min，2008 手册 w 行）
    pub w: f64,
    /// 球冠半径（M3 = 模板实测；其余 = 2025 表 r3 上限）
    pub r3: f64,
    /// 顶面平环半径（M3 = 模板实测 1.38；其余 = e/2 + w）
    pub rt: f64,
    pub l_min: f64,
    pub l_max: f64,
    pub lengths: Vec<f64>,
}

/// GB/T 2671.1-2017 一行。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Torx2671Row {
    pub d: f64,
    #[serde(rename = "P")]
    pub p: f64,
    /// 螺纹长度（d≤3 → 25，d≥3.5 → 38）
    pub b: f64,
    /// 头高（公称=max）
    pub k: f64,
    /// 头径（公称=max）
    pub dk: f64,
    /// 花形槽深（max）
    pub t: f64,
    /// 螺纹收尾上限（≈2.5P；画法用 2P）
    #[serde(rename = "x")]
    pub x: f64,
    /// 花形槽参考外接圆
    #[serde(rename = "A")]
    pub big_a: f64,
    /// 顶部圆角（min）
    pub r: f64,
    /// 顶部壁厚（min）
    pub w: f64,
    /// 杆过渡直径（表列，画法不用）
    pub da: f64,
    /// 收尾上限（表列，画法用 2P）
    pub a: f64,
    pub t_min: f64,
    /// 花形槽号（T6…T50）
    pub slot: String,
    pub l_min: f64,
    pub l_max: f64,
    pub lengths: Vec<f64>,
}

fn t_button() -> &'static Table<Button702Row> {
    static T: OnceLock<Table<Button702Row>> = OnceLock::new();
    T.get_or_init(|| Table::parse(include_str!("tables/partsSocketButton702.json")))
}

fn t_torx() -> &'static Table<Torx2671Row> {
    static T: OnceLock<Table<Torx2671Row>> = OnceLock::new();
    T.get_or_init(|| Table::parse(include_str!("tables/partsSocketTorx2671.json")))
}

/// 可选公称直径（GB/T 70.2）。
pub fn button702_diameters() -> Vec<f64> {
    t_button().rows.iter().map(|r| r.d).collect()
}

/// 可选公称直径（GB/T 2671.1）。
pub fn torx2671_diameters() -> Vec<f64> {
    t_torx().rows.iter().map(|r| r.d).collect()
}

pub fn button702_row(d: f64) -> Option<&'static Button702Row> {
    t_button().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

pub fn torx2671_row(d: f64) -> Option<&'static Torx2671Row> {
    t_torx().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

// ══════════════════════════════════════════════════════════════════════════
// 几何工具
// ══════════════════════════════════════════════════════════════════════════

/// 过两点、半径 `r` 的圆弧（取背离 y=0 轴线鼓起的那一支）。
/// 返回 `(圆心, 起角°, 止角°)`，起止角保证从起角 CCW 到止角就是该支。
fn bulge_arc(p1: [f64; 2], p2: [f64; 2], r: f64) -> ([f64; 2], f64, f64) {
    let pi = std::f64::consts::PI;
    let mid = [(p1[0] + p2[0]) / 2.0, (p1[1] + p2[1]) / 2.0];
    let (dx, dy) = (p2[0] - p1[0], p2[1] - p1[1]);
    let len = (dx * dx + dy * dy).sqrt();
    let (nx, ny) = (-dy / len, dx / len);
    let h = (r * r - 0.25 * len * len).max(0.0).sqrt();
    let mut best: Option<([f64; 2], f64, f64, f64)> = None;
    for c in [
        [mid[0] + nx * h, mid[1] + ny * h],
        [mid[0] - nx * h, mid[1] - ny * h],
    ] {
        let mut a1 = (p1[1] - c[1]).atan2(p1[0] - c[0]);
        let mut a2 = (p2[1] - c[1]).atan2(p2[0] - c[0]);
        let mut span = (a2 - a1).rem_euclid(2.0 * pi);
        if span > pi {
            std::mem::swap(&mut a1, &mut a2);
            span = 2.0 * pi - span;
        }
        let am = a1 + span / 2.0;
        // 取“弧中点离原点更远”的那支（背离轴线的鼓面）
        let mrad = ((c[0] + r * am.cos()).powi(2) + (c[1] + r * am.sin()).powi(2)).sqrt();
        if best.map_or(true, |b| mrad > b.3 + 1e-15) {
            best = Some((c, a1.to_degrees(), a2.to_degrees(), mrad));
        }
    }
    let (c, a1, a2, _) = best.expect("两点+半径必有解");
    (c, a1, a2)
}

/// 已知圆心/半径，过 `pa`、`pb` 的两支 CCW 弧中取“背离轴线”或“朝向轴线”的一支
/// （按弧中点到原点的距离判定），返回 `(起角°, 止角°)`。
fn arc_prefer(c: [f64; 2], r: f64, pa: [f64; 2], pb: [f64; 2], outward: bool) -> (f64, f64) {
    let pi = std::f64::consts::PI;
    let mut opts: Vec<(f64, f64, f64)> = Vec::new();
    for (p0, p1) in [(pa, pb), (pb, pa)] {
        let a1 = (p0[1] - c[1]).atan2(p0[0] - c[0]);
        let a2 = (p1[1] - c[1]).atan2(p1[0] - c[0]);
        let span = (a2 - a1).rem_euclid(2.0 * pi);
        let am = a1 + span / 2.0;
        let mrad = ((c[0] + r * am.cos()).powi(2) + (c[1] + r * am.sin()).powi(2)).sqrt();
        opts.push((mrad, a1.to_degrees(), a2.to_degrees()));
    }
    let pick = if outward {
        opts.iter().cloned().fold(f64::MIN, |m, o| m.max(o.0))
    } else {
        opts.iter().cloned().fold(f64::MAX, |m, o| m.min(o.0))
    };
    let o = opts.iter().find(|o| (o.0 - pick).abs() < 1e-9).unwrap();
    (o.1, o.2)
}

// ── GB/T 70.2-2015 ─────────────────────────────────────────────────────────

/// 生成 GB/T 70.2-2015 的一个视图。
pub fn socket_button(d: f64, l: f64, view: &str) -> Result<GenPart, String> {
    let row = button702_row(d).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.parts.err.no_m",
            &[("table", "GB/T 70.2"), ("d", &trim(d))],
        )
    })?;
    check_length(row.l_min, row.l_max, d, l)?;
    let entities = match view {
        "main" => button_main(row, l),
        "end" => button_end(row),
        other => {
            return Err(crate::i18n::t_fmt(
                "cmd.parts.err.view_not_offered",
                &[("family", "socket_button_702"), ("view", other), ("avail", "main/end")],
            ))
        }
    };
    let bbox = if view == "main" {
        [-row.k, -row.dk / 2.0, l, row.dk / 2.0]
    } else {
        [-row.dk / 2.0, -row.dk / 2.0, row.dk / 2.0, row.dk / 2.0]
    };
    Ok(GenPart {
        entities,
        meta: PartMeta {
            code: "GB/T 70.2-2015".into(),
            name: crate::i18n::t_data("内六角平圆头螺钉").into(),
            spec: format!("M{}×{}", trim(d), trim(l)),
            material: String::new(),
            weight: weight_text(weight_button(row, l)),
        },
        bbox,
    })
}

/// 主视图（轴水平、头在 −x、杆向 +x；两半显式生成，与模板同构）。
fn button_main(row: &Button702Row, l: f64) -> Vec<EntityType> {
    let (d, e, s, k, t, a, rt, r3) = (row.d, row.e, row.s, row.k, row.t, row.a, row.rt, row.r3);
    let (rd, dm, ck) = (d / 2.0, 0.85 * d / 2.0, 0.075 * d);
    let hy = e / 2.0;
    let ht = s / 2.0;
    let h2 = hy - ht * 30f64.to_radians().tan();
    let xb = -k + t;
    let xe = xb + hy / 60f64.to_radians().tan();
    let mut en = Vec::new();
    for sgn in [1.0f64, -1.0] {
        // 头：顶面 + 球冠 + 支承面
        en.push(line([-k, 0.0], [-k, sgn * rt], LAYER_MAIN));
        let (c, a1, a2) = bulge_arc([-k, sgn * rt], [0.0, sgn * row.dk / 2.0], r3);
        en.push(arc(c, r3, a1, a2, LAYER_MAIN));
        en.push(line([0.0, 0.0], [0.0, sgn * row.dk / 2.0], LAYER_MAIN));
        // 杆（模板把各段画成“回到轴线”的折线）
        en.push(polyline(&[[0.0, sgn * rd], [a, sgn * rd], [a, 0.0]], false, LAYER_MAIN));
        en.push(polyline(
            &[[a, sgn * rd], [l - ck, sgn * rd], [l - ck, 0.0]],
            false,
            LAYER_MAIN,
        ));
        en.push(polyline(
            &[[l - ck, sgn * rd], [l, sgn * dm], [l, 0.0]],
            false,
            LAYER_MAIN,
        ));
        // 收尾斜线 + 牙底细实线
        en.push(polyline(
            &[[0.0, sgn * rd], [a, sgn * dm], [l - ck, sgn * dm]],
            false,
            LAYER_THIN,
        ));
        // 内六角孔（虚线）
        en.push(polyline(
            &[[-k, sgn * hy], [xb, sgn * hy], [xb, 0.0]],
            false,
            LAYER_HIDDEN,
        ));
        en.push(polyline(&[[xb, sgn * hy], [xe, 0.0]], false, LAYER_HIDDEN));
        en.push(line([-k, sgn * h2], [xb, sgn * h2], LAYER_HIDDEN));
    }
    en.push(line(
        [-k - AXIS_OVER, 0.0],
        [l + AXIS_OVER, 0.0],
        LAYER_CENTER,
    ));
    en
}

/// 左视图（端视）。
fn button_end(row: &Button702Row) -> Vec<EntityType> {
    let (dk, rt, e, s) = (row.dk, row.rt, row.e, row.s);
    let hy = e / 2.0;
    let ht = s / 2.0;
    let h2 = hy - ht * 30f64.to_radians().tan();
    let mut en = vec![
        circle([0.0, 0.0], dk / 2.0, LAYER_MAIN),
        circle([0.0, 0.0], rt, LAYER_MAIN),
        circle([0.0, 0.0], hy, LAYER_MAIN),
        polyline(
            &[[0.0, -hy], [ht, -h2], [ht, h2], [0.0, hy]],
            false,
            LAYER_MAIN,
        ),
        polyline(
            &[[0.0, -hy], [-ht, -h2], [-ht, h2], [0.0, hy]],
            false,
            LAYER_MAIN,
        ),
    ];
    let o = dk / 2.0 + AXIS_OVER; // 端视出头 2（模板实测）
    en.push(line([-o, 0.0], [o, 0.0], LAYER_CENTER));
    en.push(line([0.0, o], [0.0, -o], LAYER_CENTER));
    en
}

// ── GB/T 2671.1-2017 ───────────────────────────────────────────────────────

/// 花形槽 5° 锥头：顶面平环半径（解析式）。
pub fn torx2671_top_radius(row: &Torx2671Row) -> f64 {
    let a = TORX_HEAD_CONE_DEG.to_radians();
    row.dk / 2.0 - a.tan() * (row.k - row.r) - row.r / a.cos()
}

/// 生成 GB/T 2671.1-2017 的一个视图。
pub fn socket_torx(d: f64, l: f64, view: &str) -> Result<GenPart, String> {
    let row = torx2671_row(d).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.parts.err.no_m",
            &[("table", "GB/T 2671.1"), ("d", &trim(d))],
        )
    })?;
    check_length(row.l_min, row.l_max, d, l)?;
    let entities = match view {
        "main" => torx_main(row, l),
        "end" => torx_end(row),
        other => {
            return Err(crate::i18n::t_fmt(
                "cmd.parts.err.view_not_offered",
                &[("family", "socket_torx_2671"), ("view", other), ("avail", "main/end")],
            ))
        }
    };
    let bbox = if view == "main" {
        [-row.k, -row.dk / 2.0, l, row.dk / 2.0]
    } else {
        [-row.dk / 2.0, -row.dk / 2.0, row.dk / 2.0, row.dk / 2.0]
    };
    Ok(GenPart {
        entities,
        meta: PartMeta {
            code: "GB/T 2671.1-2017".into(),
            name: crate::i18n::t_data("内六角花形低圆柱头螺钉").into(),
            spec: format!("M{}×{}", trim(d), trim(l)),
            material: String::new(),
            weight: weight_text(weight_torx(row, l)),
        },
        bbox,
    })
}

/// 头部 5° 锥角（标准图 `≤5°`，模板实测 5°）。
const TORX_HEAD_CONE_DEG: f64 = 5.0;
/// 中心线出头（主视图，模板实测 2.0）。
const AXIS_OVER: f64 = 2.0;
/// 中心线出头（2671 端视，模板实测 3.0）。
const AXIS_OVER_END: f64 = 3.0;
/// 花形外壁比例（模板 M2：0.85 / A=1.75）。
const TORX_L3_RATIO: f64 = 0.85 / 1.75;
/// 花形槽制图圆弧比例（模板 M2：Ra=0.296137116402381 / 0.85）。
const TORX_RA_RATIO: f64 = 0.296137116402381 / 0.85;
/// 花形谷弧心半径比例（标准花形：0.5424·A）。
const TORX_RC_RATIO: f64 = 0.5424;
/// 花形谷弧半径比例（标准花形：0.1424·A）。
const TORX_RS_RATIO: f64 = 0.1424;

/// 主视图。
fn torx_main(row: &Torx2671Row, l: f64) -> Vec<EntityType> {
    let (d, k, t, r, p) = (row.d, row.k, row.t, row.r, row.p);
    let rt = torx2671_top_radius(row);
    let l3 = row.big_a * TORX_L3_RATIO;
    let l1 = l3 / 3.0;
    let l2 = l3 * 2.0 / 3.0;
    let ra = l3 * TORX_RA_RATIO;
    let dd = (ra * ra - l1 * l1).max(0.0).sqrt();
    let x0 = -k;
    let xt = -k + t;
    let xd = x0 + (ra - dd);
    let xtip = xt + t / 2.0;
    let xl2 = xt + t / 6.0;
    let (rd, dm) = (d / 2.0, 0.85 * d / 2.0);
    let run = 2.0 * p; // 画法收尾 2P（表 X 为上限 ≈2.5P）
    let full = l <= row.b + 1e-9;
    let delta = TORX_HEAD_CONE_DEG.to_radians();
    let (tn, nrm) = (delta.tan(), (1.0 + delta.tan() * delta.tan()).sqrt());
    let mut en = Vec::new();
    for sgn in [1.0f64, -1.0] {
        // ── 头：顶面 + 顶部圆角 + 5° 锥侧 + 基面 ──
        let ftop = [x0, sgn * rt];
        let fc = [x0 + r, sgn * rt];
        // 锥面直线（下半/上半）：tan5°·x − sgn·y + dk/2 = 0
        let dval = tn * fc[0] - sgn * fc[1] + row.dk / 2.0;
        let foot = [
            fc[0] - dval * tn / (nrm * nrm),
            fc[1] + dval * sgn / (nrm * nrm),
        ];
        let (a1, a2) = arc_prefer(fc, r, ftop, foot, true);
        en.push(polyline(&[[x0, 0.0], ftop], false, LAYER_MAIN));
        en.push(arc(fc, r, a1, a2, LAYER_MAIN));
        en.push(polyline(
            &[foot, [0.0, sgn * row.dk / 2.0], [0.0, 0.0]],
            false,
            LAYER_MAIN,
        ));
        // ── 花形槽（虚线制图展开）──
        en.push(polyline(&[[x0, sgn * l3], [xt, sgn * l3]], false, LAYER_HIDDEN));
        en.push(polyline(&[[xt, sgn * l3], [xtip, 0.0]], false, LAYER_HIDDEN));
        en.push(polyline(&[[x0, sgn * l1], [xt, sgn * l1]], false, LAYER_HIDDEN));
        en.push(line([xt, sgn * l1], [xt, sgn * l2], LAYER_HIDDEN));
        en.push(line([x0, sgn * l2], [xl2, sgn * l2], LAYER_HIDDEN));
        en.push(polyline(&[[x0, sgn * l3], [xd, sgn * l2]], false, LAYER_HIDDEN));
        // ── 杆 / 螺纹 ──
        if full {
            en.push(polyline(
                &[[0.0, sgn * dm], [run, sgn * rd], [run, 0.0]],
                false,
                LAYER_MAIN,
            ));
            en.push(polyline(
                &[[run, sgn * rd], [l, sgn * rd], [l, 0.0]],
                false,
                LAYER_MAIN,
            ));
            en.push(polyline(
                &[[0.0, sgn * dm], [run, sgn * dm], [l, sgn * dm]],
                false,
                LAYER_THIN,
            ));
        } else {
            let xls = l - row.b; // 杆端起长 b 为螺纹
            let xs = xls - run; // 收尾起点
            en.push(polyline(
                &[[0.0, sgn * rd], [xs, sgn * rd], [xs, 0.0]],
                false,
                LAYER_MAIN,
            ));
            en.push(polyline(
                &[[xs, sgn * rd], [xls, sgn * dm], [xls, 0.0]],
                false,
                LAYER_MAIN,
            ));
            en.push(polyline(
                &[[xls, sgn * rd], [l, sgn * rd], [l, 0.0]],
                false,
                LAYER_MAIN,
            ));
            en.push(polyline(
                &[[xls, sgn * dm], [l, sgn * dm]],
                false,
                LAYER_THIN,
            ));
        }
    }
    // 花形槽两条“展开圆弧”（跨轴线，模板单实体）
    let alpha = (l1 / ra).clamp(-1.0, 1.0).asin().to_degrees();
    for xc in [x0 - dd, xt - dd] {
        en.push(arc([xc, 0.0], ra, -alpha, alpha, LAYER_HIDDEN));
    }
    en.push(line(
        [x0 - AXIS_OVER, 0.0],
        [l + AXIS_OVER, 0.0],
        LAYER_CENTER,
    ));
    en
}

/// 左视图（花形端视）。
fn torx_end(row: &Torx2671Row) -> Vec<EntityType> {
    let r_major = row.big_a / 2.0;
    let r_valley = 0.8 * r_major;
    let rs = row.big_a * TORX_RS_RATIO;
    let rc = row.big_a * TORX_RC_RATIO;
    let mut en = vec![
        circle([0.0, 0.0], row.dk / 2.0, LAYER_MAIN),
        circle([0.0, 0.0], r_major, LAYER_MAIN),
        circle([0.0, 0.0], r_valley, LAYER_HIDDEN),
    ];
    for i in 0..6 {
        let th = (60.0 * i as f64).to_radians();
        let c = [rc * th.cos(), rc * th.sin()];
        let p1 = [
            r_major * (th - 15f64.to_radians()).cos(),
            r_major * (th - 15f64.to_radians()).sin(),
        ];
        let p2 = [
            r_major * (th + 15f64.to_radians()).cos(),
            r_major * (th + 15f64.to_radians()).sin(),
        ];
        let (a1, a2) = arc_prefer(c, rs, p1, p2, false); // 取朝轴线鼓的谷弧
        en.push(arc(c, rs, a1, a2, LAYER_MAIN));
    }
    let o = row.dk / 2.0 + AXIS_OVER_END;
    en.push(line([-o, 0.0], [o, 0.0], LAYER_CENTER));
    en.push(line([0.0, o], [0.0, -o], LAYER_CENTER));
    en
}

// ── 重量估算（钢 7.85 g/cm³；体积 mm³）────────────────────────────────────

pub fn weight_button(row: &Button702Row, l: f64) -> f64 {
    // 球冠头 ≈ 圆柱 0.85；内六角孔 ≈ 外接圆 0.75
    let head = std::f64::consts::PI / 4.0 * row.dk * row.dk * row.k * 0.85;
    let socket = std::f64::consts::PI / 4.0 * row.e * row.e * row.t * 0.75;
    let shank = std::f64::consts::PI / 4.0 * row.d * row.d * l;
    weight_kg(head - socket + shank, 7.85)
}

pub fn weight_torx(row: &Torx2671Row, l: f64) -> f64 {
    // 5° 锥头 ≈ 圆柱 0.75；花形槽 ≈ 外接圆 0.6
    let head = std::f64::consts::PI / 4.0 * row.dk * row.dk * row.k * 0.75;
    let socket = std::f64::consts::PI / 4.0 * row.big_a * row.big_a * row.t * 0.6;
    let shank = std::f64::consts::PI / 4.0 * row.d * row.d * l;
    weight_kg(head - socket + shank, 7.85)
}

// ══════════════════════════════════════════════════════════════════════════
// 对外 API（partgen_more / partgen 会先问这里）
// ══════════════════════════════════════════════════════════════════════════

/// 本组声明的视图（两族都是模板里的 主视图 + 左视图）。
pub fn family_views(family: &str) -> Vec<&'static str> {
    match family {
        "socket_button_702" | "socket_torx_2671" => vec!["main", "end"],
        _ => Vec::new(),
    }
}

/// 本组族的目录 JSON 片段（键 = 族 id）。
pub fn families_json() -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    let sizes702: Vec<serde_json::Value> = t_button()
        .rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "d": r.d, "label": format!("M{}", trim(r.d)), "pitch": r.p,
                "l_min": r.l_min, "l_max": r.l_max, "lengths": r.lengths,
                "extra": format!(
                    "头 dk{} k{} / 内六角 s{} t{} / 收尾 a{}",
                    trim(r.dk), trim(r.k), trim(r.s), trim(r.t), trim(r.a)),
            })
        })
        .collect();
    m.insert(
        "socket_button_702".into(),
        serde_json::json!({
            "id": "socket_button_702", "name": "内六角平圆头螺钉",
            "code": "GB/T 70.2-2015", "iso": "ISO 7380-1:2011",
            "implemented": true, "views": views_json("socket_button_702"), "sizes": sizes702,
            "len_label": "长度 l", "base_hint": "基点 = 头部支承面 × 轴线",
            "tree_dir": "零件库/螺钉/内六角",
        }),
    );
    let sizes2671: Vec<serde_json::Value> = t_torx()
        .rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "d": r.d, "label": format!("M{}", trim(r.d)), "pitch": r.p,
                "l_min": r.l_min, "l_max": r.l_max, "lengths": r.lengths,
                "extra": format!(
                    "头 dk{} k{} / 花形 {} A{} t{} / 螺纹长 b{}",
                    trim(r.dk), trim(r.k), r.slot, trim(r.big_a), trim(r.t), trim(r.b)),
            })
        })
        .collect();
    m.insert(
        "socket_torx_2671".into(),
        serde_json::json!({
            "id": "socket_torx_2671", "name": "内六角花形低圆柱头螺钉",
            "code": "GB/T 2671.1-2017", "iso": "ISO 14580",
            "implemented": true, "views": views_json("socket_torx_2671"), "sizes": sizes2671,
            "len_label": "长度 l", "base_hint": "基点 = 头部支承面 × 轴线",
            "tree_dir": "零件库/螺钉/内六角",
        }),
    );
    m
}

/// 本组族的生成派发。
pub fn generate(family: &str, d: f64, l: f64, view: &str) -> Option<Result<GenPart, String>> {
    match family {
        "socket_button_702" => Some(socket_button(d, l, view)),
        "socket_torx_2671" => Some(socket_torx(d, l, view)),
        _ => None,
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 测试
// ══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── 模板（WCS；只存非镜像半 + 中心线；mir 指明测试里要补的镜像）──────
    // 元组：(图元种类, 数值, 图层字符, 镜像字符)
    //   L = 直线 (x1,y1,x2,y2)；A = 圆弧 (cx,cy,r,a1°,a2°)；
    //   C = 圆 (cx,cy,r)；P = 折线 (x1,y1,x2,y2,…)
    //   图层：M=1轮廓实线层 T=2细线层 H=4虚线层 C=3中心线层
    //   镜像：'y' = 关于 y=0 轴线镜像、'x' = 关于 x=0 镜像、' ' = 不镜像
    //
    // 说明：模板 DXF 里少数点被源 CAD 取到 5 位小数（例 702 的 h2=0.57265 ——
    // 与解析值 0.57264973 差 2.7e-7）；2671 端视的 6 条谷弧被源 CAD 用了两个近似
    // 半径（0.24918/0.24921，解析应为 0.1424A=0.2492），故该视图容差取 5e-5；
    // 其余三视图逐图元实测 ≤1.5e-6，容差取 1e-5。关键曲线另有解析值断言（见下）。

    const TOL_TPL: f64 = 1e-5;
    /// 2671 端视（模板谷弧是源 CAD 近似；解析构造仍按 0.1424A/0.5424A，见解析断言）。
    const TOL_TPL_TORX_END: f64 = 5e-5;

    type Tpl = (char, &'static [f64], char, char);

    /// 702 主视图（下半 + 轴线）。
    const TPL_702_MAIN: &[Tpl] = &[
        ('L', &[-1.65, 0.0, -1.65, -1.38], 'M', 'y'),
        (
            'A',
            &[
                2.02067634173053,
                1.07912650602406,
                4.41827658463912,
                213.819643515188,
                242.784087833682,
            ],
            'M',
            'y',
        ),
        ('L', &[0.0, 0.0, 0.0, -2.85], 'M', 'y'),
        ('P', &[0.0, -1.5, 1.0, -1.5, 1.0, 0.0], 'M', 'y'),
        ('P', &[1.0, -1.5, 5.775, -1.5, 5.775, 0.0], 'M', 'y'),
        ('P', &[5.775, -1.5, 6.0, -1.275, 6.0, 0.0], 'M', 'y'),
        ('P', &[0.0, -1.5, 1.0, -1.275, 5.775, -1.275], 'T', 'y'),
        ('P', &[-1.65, -1.15, -0.61, -1.15, -0.61, 0.0], 'H', 'y'),
        ('P', &[-0.61, -1.15, 0.0539528, 0.0], 'H', 'y'),
        ('P', &[-1.65, -0.57265, -0.61, -0.57265], 'H', 'y'),
        ('L', &[-3.65, 0.0, 8.0, 0.0], 'C', ' '),
    ];

    /// 702 左视图（右半六角 + 对称圆/十字）。
    const TPL_702_END: &[Tpl] = &[
        ('C', &[0.0, 0.0, 2.85], 'M', ' '),
        ('C', &[0.0, 0.0, 1.38], 'M', ' '),
        ('C', &[0.0, 0.0, 1.15], 'M', ' '),
        ('L', &[0.0, -4.85, 0.0, 4.85], 'C', ' '),
        ('L', &[4.85, 0.0, -4.85, 0.0], 'C', ' '),
        (
            'P',
            &[0.0, -1.15, 1.0, -0.57265, 1.0, 0.57265, 0.0, 1.15],
            'M',
            'x',
        ),
    ];

    /// 2671 主视图（下半；两条花形弧跨轴线不镜像）。
    const TPL_2671_MAIN: &[Tpl] = &[
        ('P', &[-1.55, 0.0, -1.55, -1.67276], 'M', 'y'),
        (
            'A',
            &[-1.45, -1.67276, 0.1, 180.0, 264.997500394289],
            'M',
            'y',
        ),
        ('P', &[-1.45872, -1.77238, 0.0, -1.9, 0.0, 0.0], 'M', 'y'),
        ('P', &[-1.55, -0.85, -0.71, -0.85], 'H', 'y'),
        ('P', &[-0.71, -0.85, -0.29, 0.0], 'H', 'y'),
        ('P', &[0.0, -0.85, 0.8, -1.0, 0.8, 0.0], 'M', 'y'),
        ('P', &[0.8, -1.0, 8.0, -1.0, 8.0, 0.0], 'M', 'y'),
        ('P', &[0.0, -0.85, 0.8, -0.85, 8.0, -0.85], 'T', 'y'),
        (
            'A',
            &[
                -1.63613711640238,
                0.0,
                0.296137116402381,
                286.909990862005,
                73.0900091379955,
            ],
            'H',
            ' ',
        ),
        (
            'A',
            &[
                -0.796137116402381,
                0.0,
                0.296137116402381,
                286.909990862005,
                73.0900091379955,
            ],
            'H',
            ' ',
        ),
        ('L', &[-1.55, -0.283333, -0.71, -0.283333], 'H', 'y'),
        ('L', &[-1.55, -0.566667, -0.57, -0.566667], 'H', 'y'),
        ('L', &[-0.71, -0.283333, -0.71, -0.566667], 'H', 'y'),
        ('P', &[-1.34, -0.566667, -1.55, -0.85], 'H', 'y'),
        ('L', &[-3.55, 0.0, 10.0, 0.0], 'C', ' '),
    ];

    /// 2671 左视图（左半 3 条谷弧；镜像到右半）。
    const TPL_2671_END: &[Tpl] = &[
        ('L', &[0.0, 4.9, 0.0, -4.9], 'C', ' '),
        ('L', &[-4.9, 0.0, 4.9, 0.0], 'C', ' '),
        ('C', &[0.0, 0.0, 1.9], 'M', ' '),
        ('C', &[0.0, 0.0, 0.875], 'M', ' '),
        ('C', &[0.0, 0.0, 0.7], 'H', ' '),
        (
            'A',
            &[
                -0.4746020613,
                0.8220096914,
                0.2491821176,
                234.6700876,
                365.336535365,
            ],
            'M',
            'x',
        ),
        (
            'A',
            &[
                -0.949209167,
                0.0,
                0.249209167,
                294.66783005,
                425.33216995,
            ],
            'M',
            'x',
        ),
        (
            'A',
            &[
                -0.4746020613,
                -0.8220096914,
                0.2491821176,
                354.663464635,
                485.3299124,
            ],
            'M',
            'x',
        ),
    ];

    // ── 模板展开 / 采样 / 距离 ─────────────────────────────────────────

    #[derive(Clone, Debug)]
    struct Ent {
        k: char,
        n: Vec<f64>,
        layer: char,
    }

    fn mirror_ent(e: &Ent, axis: char) -> Ent {
        let n = match e.k {
            'L' | 'P' => {
                let mut v = e.n.clone();
                for i in (0..v.len()).step_by(2) {
                    if axis == 'x' {
                        v[i] = -v[i];
                    } else {
                        v[i + 1] = -v[i + 1];
                    }
                }
                v
            }
            'A' => {
                let (cx, cy, r, a1, a2) = (e.n[0], e.n[1], e.n[2], e.n[3], e.n[4]);
                if axis == 'x' {
                    vec![-cx, cy, r, 180.0 - a2, 180.0 - a1]
                } else {
                    vec![cx, -cy, r, -a2, -a1]
                }
            }
            'C' => {
                if axis == 'x' {
                    vec![-e.n[0], e.n[1], e.n[2]]
                } else {
                    vec![e.n[0], -e.n[1], e.n[2]]
                }
            }
            _ => e.n.clone(),
        };
        Ent {
            k: e.k,
            n,
            layer: e.layer,
        }
    }

    fn tpl_entities(tpl: &[Tpl]) -> Vec<Ent> {
        let mut out = Vec::new();
        for (k, n, layer, mir) in tpl {
            let e = Ent {
                k: *k,
                n: n.to_vec(),
                layer: *layer,
            };
            out.push(e.clone());
            if *mir != ' ' {
                out.push(mirror_ent(&e, *mir));
            }
        }
        out
    }

    fn gen_entities(p: &GenPart) -> Vec<Ent> {
        let mut out = Vec::new();
        for e in &p.entities {
            let lay = match e.common().layer.as_str() {
                LAYER_MAIN => 'M',
                LAYER_THIN => 'T',
                LAYER_HIDDEN => 'H',
                LAYER_CENTER => 'C',
                _ => '?',
            };
            match e {
                EntityType::Line(l) => out.push(Ent {
                    k: 'L',
                    n: vec![l.start.x, l.start.y, l.end.x, l.end.y],
                    layer: lay,
                }),
                EntityType::Arc(a) => out.push(Ent {
                    k: 'A',
                    n: vec![
                        a.center.x,
                        a.center.y,
                        a.radius,
                        a.start_angle.to_degrees(),
                        a.end_angle.to_degrees(),
                    ],
                    layer: lay,
                }),
                EntityType::Circle(c) => out.push(Ent {
                    k: 'C',
                    n: vec![c.center.x, c.center.y, c.radius],
                    layer: lay,
                }),
                EntityType::LwPolyline(pl) => {
                    let mut n = Vec::new();
                    for v in &pl.vertices {
                        n.push(v.location.x);
                        n.push(v.location.y);
                    }
                    out.push(Ent {
                        k: 'P',
                        n,
                        layer: lay,
                    });
                }
                _ => {}
            }
        }
        out
    }

    /// 图元采样点（L/P 端点与折线段；A/C 弧上取 17 点）。
    fn samples(e: &Ent) -> Vec<[f64; 2]> {
        let mut out = Vec::new();
        match e.k {
            'L' => {
                out.push([e.n[0], e.n[1]]);
                out.push([e.n[2], e.n[3]]);
            }
            'P' => {
                for w in e.n.chunks(2) {
                    out.push([w[0], w[1]]);
                }
            }
            'A' => {
                let (cx, cy, r) = (e.n[0], e.n[1], e.n[2]);
                let (a1, a2) = (e.n[3], e.n[4]);
                let mut span = (a2 - a1).rem_euclid(360.0);
                let mut s0 = a1;
                if span <= 1e-12 {
                    span = 360.0;
                    s0 = 0.0;
                }
                for i in 0..=16 {
                    let a = (s0 + span * i as f64 / 16.0).to_radians();
                    out.push([cx + r * a.cos(), cy + r * a.sin()]);
                }
            }
            'C' => {
                let (cx, cy, r) = (e.n[0], e.n[1], e.n[2]);
                for i in 0..=16 {
                    let a = (360.0 * i as f64 / 16.0).to_radians();
                    out.push([cx + r * a.cos(), cy + r * a.sin()]);
                }
            }
            _ => {}
        }
        out
    }

    /// 点到图元的精确距离。
    fn dist(p: [f64; 2], e: &Ent) -> f64 {
        match e.k {
            'L' => dist_seg(p, [e.n[0], e.n[1]], [e.n[2], e.n[3]]),
            'P' => e
                .n
                .chunks(2)
                .collect::<Vec<_>>()
                .windows(2)
                .map(|w| dist_seg(p, [w[0][0], w[0][1]], [w[1][0], w[1][1]]))
                .fold(f64::MAX, f64::min),
            'C' => {
                let (cx, cy, r) = (e.n[0], e.n[1], e.n[2]);
                ((p[0] - cx).hypot(p[1] - cy) - r).abs()
            }
            'A' => {
                let (cx, cy, r, a1, a2) = (e.n[0], e.n[1], e.n[2], e.n[3], e.n[4]);
                let ang = (p[1] - cy).atan2(p[0] - cx).to_degrees();
                let span = (a2 - a1).rem_euclid(360.0);
                let rel = (ang - a1).rem_euclid(360.0);
                if rel <= span + 1e-9 {
                    ((p[0] - cx).hypot(p[1] - cy) - r).abs()
                } else {
                    let pa = [
                        cx + r * a1.to_radians().cos(),
                        cy + r * a1.to_radians().sin(),
                    ];
                    let pb = [
                        cx + r * a2.to_radians().cos(),
                        cy + r * a2.to_radians().sin(),
                    ];
                    dist_seg(p, pa, pa).min(dist_seg(p, pb, pb))
                }
            }
            _ => f64::MAX,
        }
    }

    fn dist_seg(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len2 = dx * dx + dy * dy;
        let t = if len2 < 1e-18 {
            0.0
        } else {
            (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
        };
        ((p[0] - (a[0] + t * dx)).powi(2) + (p[1] - (a[1] + t * dy)).powi(2)).sqrt()
    }

    /// 双向模板比对：模板每条图元的每个采样点都能在生成件同图层里找到 ≤tol 的
    /// 对应，生成件每条图元也都能被模板同图层覆盖。
    fn check_template(tpl: &[Tpl], part: &GenPart, tol: f64, label: &str) {
        let te = tpl_entities(tpl);
        let ge = gen_entities(part);
        let mut worst = 0.0f64;
        for t in &te {
            for p in samples(t) {
                let d = ge
                    .iter()
                    .filter(|g| g.layer == t.layer)
                    .map(|g| dist(p, g))
                    .fold(f64::MAX, f64::min);
                worst = worst.max(d);
                assert!(
                    d <= tol,
                    "{label}: 模板图元 {:?} 采样点 {:?} 在生成件同层找不到（{d:.2e} > {tol:.1e}）",
                    t.k,
                    p
                );
            }
        }
        for g in &ge {
            for p in samples(g) {
                let d = te
                    .iter()
                    .filter(|t| t.layer == g.layer)
                    .map(|t| dist(p, t))
                    .fold(f64::MAX, f64::min);
                worst = worst.max(d);
                assert!(
                    d <= tol,
                    "{label}: 生成件图元 {:?} 采样点 {:?} 在模板同层找不到（{d:.2e} > {tol:.1e}）",
                    g.k,
                    p
                );
            }
        }
        eprintln!("{label}: 逐图元最大偏离 {worst:.3e}（容差 {tol:.1e}）");
    }

    // ── 规格遍历契约 ───────────────────────────────────────────────────

    fn assert_part_ok(fam: &str, d: f64, l: f64, view: &str, p: &GenPart) {
        assert!(!p.meta.code.is_empty(), "{fam} 应有标准号");
        assert!(!p.meta.spec.is_empty(), "{fam} 应有规格文本");
        assert!(p.meta.weight.parse::<f64>().unwrap_or(-1.0) > 0.0, "{fam} 重量应为正");
        assert!(!p.entities.is_empty(), "{fam} {view} 应有图元");
        for e in &p.entities {
            let lay = e.common().layer.as_str();
            assert!(
                [LAYER_MAIN, LAYER_THIN, LAYER_HIDDEN, LAYER_CENTER].contains(&lay),
                "{fam} 只能使用 OCSM 五层，实际 {lay}"
            );
        }
        assert!(
            !p.entities.iter().any(|e| matches!(
                e,
                EntityType::Dimension(_) | EntityType::Hatch(_) | EntityType::Text(_)
            )),
            "{fam} {view} 不应含标注/剖面线/文字"
        );
        for v in [p.bbox[0], p.bbox[1], p.bbox[2], p.bbox[3]] {
            assert!(v.is_finite(), "{fam} bbox 应有限");
        }
        assert!(p.bbox[2] > p.bbox[0] && p.bbox[3] > p.bbox[1], "{fam} bbox 非退化");
    }

    #[test]
    fn button702_all_specs_contract() {
        for row in &t_button().rows {
            for &l in &row.lengths {
                for view in ["main", "end"] {
                    let p = socket_button(row.d, l, view).unwrap_or_else(|e| {
                        panic!("socket_button_702 M{}×{} {view}: {e}", row.d, l)
                    });
                    assert_part_ok("socket_button_702", row.d, l, view, &p);
                }
            }
            assert!(button702_row(row.d).is_some());
        }
        // 表外直径 / 表外长度 / 未知视图都要报错
        assert!(socket_button(2.5, 6.0, "main").is_err(), "M2.5 不在 70.2 表");
        assert!(socket_button(3.0, 5.0, "main").is_err(), "M3 长度下界 6");
        assert!(socket_button(3.0, 13.0, "main").is_err(), "M3 长度上界 12");
        assert!(socket_button(3.0, 6.0, "top").is_err(), "70.2 无俯视图");
        assert!(button702_diameters() == vec![3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 12.0, 16.0]);
    }

    #[test]
    fn torx2671_all_specs_contract() {
        for row in &t_torx().rows {
            for &l in &row.lengths {
                for view in ["main", "end"] {
                    let p = socket_torx(row.d, l, view)
                        .unwrap_or_else(|e| panic!("socket_torx_2671 M{}×{} {view}: {e}", row.d, l));
                    assert_part_ok("socket_torx_2671", row.d, l, view, &p);
                }
            }
        }
        assert!(socket_torx(1.6, 4.0, "main").is_err(), "M1.6 不在 2671.1 表");
        assert!(socket_torx(2.0, 3.0, "main").is_err(), "M2 长度下界 4");
        assert!(socket_torx(2.0, 10.0, "main").is_err(), "M2 长度上界 8（按用户 l/2提示）");
        assert!(socket_torx(2.0, 8.0, "section").is_err(), "2671.1 无剖视图");
        assert!(torx2671_diameters() == vec![2.0, 2.5, 3.0, 3.5, 4.0, 5.0, 6.0, 8.0, 10.0]);
    }

    // ── 逐图元对模板（每视图）──────────────────────────────────────────

    #[test]
    fn button702_sample_matches_template() {
        let p = socket_button(3.0, 6.0, "main").unwrap();
        check_template(TPL_702_MAIN, &p, TOL_TPL, "70.2 M3×6 主视图");
        let p = socket_button(3.0, 6.0, "end").unwrap();
        check_template(TPL_702_END, &p, TOL_TPL, "70.2 M3×6 左视图");
    }

    #[test]
    fn torx2671_sample_matches_template() {
        let p = socket_torx(2.0, 8.0, "main").unwrap();
        check_template(TPL_2671_MAIN, &p, TOL_TPL, "2671.1 M2×8 主视图");
        let p = socket_torx(2.0, 8.0, "end").unwrap();
        check_template(TPL_2671_END, &p, TOL_TPL_TORX_END, "2671.1 M2×8 左视图");
    }

    /// 关键曲线解析值（1e-9）：模板比对用 1e-4 兜住源 CAD 的取整，这里锁死公式。
    #[test]
    fn button702_head_and_socket_analytic() {
        let p = socket_button(3.0, 6.0, "main").unwrap();
        let arcs: Vec<_> = p
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) if a.common.layer == LAYER_MAIN => Some(a.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(arcs.len(), 2, "球冠弧应有两条（镜像）");
        for a in &arcs {
            assert!(
                (a.radius - 4.41827658463912).abs() < 1e-9,
                "球冠 R 应为模板实测 4.41827658463912，实得 {}",
                a.radius
            );
            assert!(
                (a.center.x - 2.02067634173053).abs() < 1e-9,
                "球冠心 x = 2.02067634173053，实得 {}",
                a.center.x
            );
            assert!(
                (a.center.y.abs() - 1.07912650602406).abs() < 1e-9,
                "球冠心 |y| = 1.07912650602406，实得 {}",
                a.center.y
            );
        }
        // 内六角孔公式：xb=-k+t、锥尖=xb+hy/tan60
        let row = button702_row(3.0).unwrap();
        let xe = -row.k + row.t + (row.e / 2.0) / 60f64.to_radians().tan();
        assert!((xe - 0.0539528).abs() < 1e-6, "锥尖解析值");
        let h2 = row.e / 2.0 - (row.s / 2.0) * 30f64.to_radians().tan();
        assert!(
            (h2 - 0.5726497308103742).abs() < 1e-12,
            "h2 = e/2 − s/2·tan30 = 0.5726497308，实得 {h2}"
        );
        assert!((h2 - 0.57265).abs() < 1e-4, "模板取整 0.57265（源 CAD 5 位小数）");
    }

    /// 2671 头部 5° 锥 + 顶部圆角解析值。
    #[test]
    fn torx2671_head_analytic() {
        let p = socket_torx(2.0, 8.0, "main").unwrap();
        let arcs: Vec<_> = p
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) if a.common.layer == LAYER_MAIN => Some(a.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(arcs.len(), 2, "顶部圆角应有两条（镜像）");
        for a in &arcs {
            assert!((a.radius - 0.1).abs() < 1e-12, "圆角 r=0.1");
            assert!((a.center.x + 1.45).abs() < 1e-9, "圆角心 x = -k+r = -1.45");
        }
        let rt = torx2671_top_radius(torx2671_row(2.0).unwrap());
        assert!((rt - 1.672759).abs() < 1e-5, "顶面半径解析值 1.672759，实得 {rt}");
    }

    /// 花形端视：A 定形（谷圆 0.8·A/2、谷弧 rs=0.1424A、心 0.5424A、端点 ±15°）。
    #[test]
    fn torx2671_end_analytic() {
        let p = socket_torx(2.0, 8.0, "end").unwrap();
        let circles: Vec<_> = p
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Circle(c) => Some(c.clone()),
                _ => None,
            })
            .collect();
        let mut rs: Vec<f64> = circles.iter().map(|c| c.radius).collect();
        rs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((rs[0] - 0.7).abs() < 1e-12, "谷圆 0.4A");
        assert!((rs[1] - 0.875).abs() < 1e-12, "A/2");
        assert!((rs[2] - 1.9).abs() < 1e-12, "dk/2");
        let arcs: Vec<_> = p
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) => Some(a.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(arcs.len(), 6);
        for a in &arcs {
            assert!((a.radius - 0.1424 * 1.75).abs() < 1e-12, "谷弧 rs=0.1424A");
            let rc = (a.center.x * a.center.x + a.center.y * a.center.y).sqrt();
            assert!((rc - 0.5424 * 1.75).abs() < 1e-12, "谷弧心半径 0.5424A");
        }
    }

    /// 树/目录注册：螺钉 = 独立顶级类「螺钉」，不是螺栓；目录条目带视图与规格。
    #[test]
    fn catalog_and_tree_registration() {
        let _g = crate::global_state_test_lock();
        let cat: serde_json::Value =
            serde_json::from_str(&crate::partgen::catalog_json()).expect("目录 JSON");
        for (fam, code) in [
            ("socket_button_702", "GB/T 70.2-2015"),
            ("socket_torx_2671", "GB/T 2671.1-2017"),
        ] {
            assert_eq!(crate::partgen::family_kind(fam), "screw", "{fam} 应是螺钉");
            assert_eq!(crate::partgen::family_meta(fam).1, code, "{fam} 标准号");
            let f = &cat["families"][fam];
            assert_eq!(f["implemented"], true);
            assert!(f["views"].as_array().unwrap().len() == 2, "{fam} 两视图");
            assert!(f["sizes"].as_array().unwrap().len() >= 8, "{fam} 规格数");
            // 树路径：零件库/螺钉/…；不得出现「螺栓」
            let mut found: Option<Vec<String>> = None;
            fn walk(
                node: &serde_json::Value,
                path: &mut Vec<String>,
                fam: &str,
                found: &mut Option<Vec<String>>,
            ) {
                if let Some(arr) = node.as_array() {
                    for n in arr {
                        walk(n, path, fam, found);
                    }
                    return;
                }
                if let Some(name) = node.get("name").and_then(|n| n.as_str()) {
                    path.push(name.to_string());
                }
                if node.get("family").and_then(|f| f.as_str()) == Some(fam) {
                    *found = Some(path.clone());
                }
                if let Some(kids) = node.get("children") {
                    walk(kids, path, fam, found);
                }
                if node.get("name").is_some() {
                    path.pop();
                }
            }
            walk(&cat["tree"], &mut Vec::new(), fam, &mut found);
            let path = found.unwrap_or_else(|| panic!("树上应有 {fam}"));
            let joined = path.join("/");
            assert!(path.contains(&"螺钉".to_string()), "{fam} 应在「螺钉」支: {joined}");
            assert!(!path.contains(&"螺栓".to_string()), "{fam} 不该在「螺栓」支: {joined}");
        }
        // 走统一入口也能生成（树/GUI 用的就是它）
        assert!(crate::partgen::generate("socket_button_702", 3.0, 6.0, "main").is_ok());
        assert!(crate::partgen::generate("socket_torx_2671", 2.0, 8.0, "end").is_ok());
    }

    /// 规格序列/尺寸链恒等式（与校验报告同口径，只做正向断言）。
    #[test]
    fn table_invariants() {
        for r in &t_button().rows {
            assert!((r.a - 2.0 * r.p).abs() < 1e-12, "M{}: a=2P", r.d);
            assert!(r.da > r.d, "M{}: da>d", r.d);
            assert!(r.dk > r.s, "M{}: dk>s", r.d);
            assert!(r.e >= r.s && r.e <= r.s / 30f64.to_radians().cos() + 1e-9, "M{}: e 范围", r.d);
            assert!(r.t < r.k && r.k < r.dk, "M{}: t<k<dk", r.d);
            assert!((r.rt - (r.e / 2.0 + r.w)).abs() < 1e-9 || r.d == 3.0, "M{}: rt=e/2+w", r.d);
        }
        for r in &t_torx().rows {
            assert!(r.b == 25.0 || r.b == 38.0, "M{}: b∈{{25,38}}", r.d);
            assert!((r.d <= 3.0) == (r.b == 25.0), "M{}: b 分组", r.d);
            assert!(r.big_a > r.x, "M{}: A>X", r.d);
            assert!(r.t < r.k && r.k < r.dk, "M{}: t<k<dk", r.d);
            let rt = torx2671_top_radius(r);
            assert!(rt > 0.0 && rt < r.dk / 2.0, "M{}: 顶面半径在 (0,dk/2)", r.d);
        }
    }
}
