//! 第四批 A 组（partgen_b1）：**1型六角螺母 GB/T 6170-2015**、**孔用弹性挡圈 A型 GB/T 893-2017**、
//! **轴用弹性挡圈 A型 GB/T 894-2017**。
//!
//! 本文件只放这三个族的画法与数据；共用图元/表/校验工具在 `partgen_kit.rs`。
//! **不要改其它模块**（partgen.rs / partgen_more.rs / partgen_kit.rs / 其它 partgen_bN.rs）——
//! 并行开发时会互相踩。需要共享的东西在 `partgen_kit` 里已有，或在自己文件内复制一份。
//!
//! | 族 id | 名称 | 模板（用户参数化图 DXF） | 视图 | 数据 |
//! |---|---|---|---|---|
//! | `nut_6170` | 1型六角螺母 | `~/桌面/GB/参数化/1型六角螺母_GB-T6170-2015/`（主/俯/左/剖） | main/top/end/section | `tables/partsNut6170.json`（29 规格） |
//! | `ring_893` | 孔用弹性挡圈 A型 | `~/桌面/GB/参数化/孔用弹性挡圈_A型_GB-T893-2017/`（主/左） | main/end | `tables/partsRing893.json`（88 规格） |
//! | `ring_894` | 轴用弹性挡圈 A型 | `~/桌面/GB/参数化/轴用弹性挡圈_A型_GB-T894-2017/`（主/左） | main/end | `tables/partsRing894.json`（86 规格） |
//!
//! 反解工具：`target/debug/examples/ref_dump <dxf> <out.tsv>`（TSV 精确数值清单，圆弧角度是**度**）。
//!
//! ## 画法来源（模板逐条反解）
//! - **nut_6170**：四视图模板（M8：s=13/e公称=15.0111/m=6.8）。轴向公式与螺栓头同构
//!   Δ=(e−s)/2·tan30°、角弧 r₂=(e/8)/sinθ（θ=2atan(8Δ/e)）、端面大弧 r₁=((e/4)²−Δ²)/(2Δ)+Δ；
//!   剖视图 = 端面 45° 倒角（0.075d）+ 内孔竖线 + 牙底粗/牙顶细 + 上下两片 ANSI31（5剖面线层）。
//!   模板里 3 条“右端弧”其实是把左端弧**平移 +m** 的制图笔误（叠合测试里做反射修正后即重合）；
//!   本实现按对称的正确画法出图。
//! - **ring_893**：规格 50 模板。外圈 r=dc/2；内圈 r=(d−d1_min)/2、圆心沿 −y 偏 e_i=(dc−d)*2/7；
//!   耳孔 r=ear/2、耳部圆弧 r_b=ear、耳孔圆心 (±Bx,−By)（Bx/By 按模板相对外半径比例）；
//!   外圈止于 −67.5°，经切线接耳弧、再接内圈（夹角由两圆求交）。模板为**半视图**（只画右半），
//!   本实现出完整对称件。
//! - **ring_894**：规格 40 模板。内圈 r=d3/2（圆心原点）；外弧 r=(22/18.25)·Ri、圆心沿 +y 偏 ear/2；
//!   耳孔 r=ear/2；耳两侧竖线 x=x1/x2、耳外弧 r=(24.3/18.25)·Ri（均按模板比例于 Ri）；两端弧止角
//!   由 x1/x2 与圆求交。模板同样是半视图（只画右半 + 两耳），本实现出完整对称件。

use crate::partgen::{chamfer_run, GenPart, PartMeta};
use crate::partgen_kit::*;
use ocs_plugin_api::host::acadrust::entities::EntityType;

/// 中心线出头（沿轴/径向），模板实测 3.0。
const AXIS_OVER: f64 = 3.0;

// ══════════════════════════════════════════════════════════════════════════
// 数据表
// ══════════════════════════════════════════════════════════════════════════

/// GB/T 6170-2015 一行（与 `tables/partsNut6170.json` 对应）。
#[derive(Debug, Clone, serde::Deserialize)]
struct NutRow {
    d: f64,
    #[serde(rename = "P")]
    pitch: f64,
    s: f64,
    /// 对角宽度（公差下限；画图用公称 s/cos30°，此字段仅备查）
    #[allow(dead_code)]
    e: f64,
    /// 螺母高度 m
    m: f64,
    #[allow(dead_code)]
    dw: f64,
    #[allow(dead_code)]
    c: f64,
    #[allow(dead_code)]
    m_min: f64,
    #[serde(default)]
    lengths: Vec<f64>,
}

/// 孔用弹性挡圈 GB/T 893-2017 A型 一行。
#[derive(Debug, Clone, serde::Deserialize)]
struct Ring893Row {
    d: f64,
    /// 厚度 s
    s: f64,
    /// 挡圈外径（公称）d_c
    dc: f64,
    #[allow(dead_code)]
    a: f64,
    #[allow(dead_code)]
    b: f64,
    /// d1 最小值（备查）
    #[allow(dead_code)]
    d1_min: f64,
    /// d2 ①（备查）
    #[allow(dead_code)]
    d2: f64,
    /// 耳孔直径（标准“安装工具规格”）
    ear: f64,
    #[serde(default)]
    lengths: Vec<f64>,
}

/// 轴用弹性挡圈 GB/T 894-2017 A型 一行。
#[derive(Debug, Clone, serde::Deserialize)]
struct Ring894Row {
    d: f64,
    /// 厚度 s
    s: f64,
    /// 挡圈自由内径（公称）d3
    d3: f64,
    #[allow(dead_code)]
    a: f64,
    #[allow(dead_code)]
    n: f64,
    /// d5 最小值（备查）
    #[allow(dead_code)]
    d5_min: f64,
    /// 耳孔直径（标准“安装工具规格”）
    ear: f64,
    #[serde(default)]
    lengths: Vec<f64>,
}

impl HasD for NutRow {
    fn d(&self) -> f64 {
        self.d
    }
}
impl HasD for Ring893Row {
    fn d(&self) -> f64 {
        self.d
    }
}
impl HasD for Ring894Row {
    fn d(&self) -> f64 {
        self.d
    }
}

fn nut_table() -> &'static Table<NutRow> {
    static T: std::sync::OnceLock<Table<NutRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| Table::parse(include_str!("tables/partsNut6170.json")))
}

fn ring893_table() -> &'static Table<Ring893Row> {
    static T: std::sync::OnceLock<Table<Ring893Row>> = std::sync::OnceLock::new();
    T.get_or_init(|| Table::parse(include_str!("tables/partsRing893.json")))
}

fn ring894_table() -> &'static Table<Ring894Row> {
    static T: std::sync::OnceLock<Table<Ring894Row>> = std::sync::OnceLock::new();
    T.get_or_init(|| Table::parse(include_str!("tables/partsRing894.json")))
}

#[allow(dead_code)]
fn nut_diameters() -> Vec<f64> {
    nut_table().rows.iter().map(|r| r.d).collect()
}
#[allow(dead_code)]
fn ring893_diameters() -> Vec<f64> {
    ring893_table().rows.iter().map(|r| r.d).collect()
}
#[allow(dead_code)]
fn ring894_diameters() -> Vec<f64> {
    ring894_table().rows.iter().map(|r| r.d).collect()
}

// ══════════════════════════════════════════════════════════════════════════
// 视图注册 / 族目录 / 派发
// ══════════════════════════════════════════════════════════════════════════

/// 本组声明的视图（`partgen_more::family_views` 会先问这里）。
pub fn family_views(family: &str) -> Vec<&'static str> {
    match family {
        "nut_6170" => vec!["main", "top", "end", "section"],
        "ring_893" | "ring_894" => vec!["main", "end"],
        _ => Vec::new(),
    }
}

/// 本组族的目录 JSON 片段（键 = 族 id）。
pub fn families_json() -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();

    // ── nut_6170 ──
    {
        let sizes: Vec<serde_json::Value> = nut_table()
            .rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "d": r.d, "label": format!("M{}", trim(r.d)), "pitch": r.pitch,
                    "l_min": r.m, "l_max": r.m, "lengths": r.lengths,
                    "extra": format!("s={} e={} m={}", trim(r.s), trim(r.e), trim(r.m)),
                })
            })
            .collect();
        m.insert(
            "nut_6170".into(),
            serde_json::json!({
                "id": "nut_6170", "name": "1型六角螺母", "code": "GB/T 6170-2015",
                "iso": "ISO 4032:2012", "implemented": true,
                "views": views_json("nut_6170"), "sizes": sizes,
                "len_label": "高度 m", "base_hint": "基点 = 端面 × 轴线",
                "tree_path": "零件库/螺母/六角螺母/1型六角螺母 GB/T 6170-2015",
            }),
        );
    }

    // ── ring_893 ──
    {
        let sizes: Vec<serde_json::Value> = ring893_table()
            .rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "d": r.d, "label": format!("Ø{}", trim(r.d)), "pitch": 0.0,
                    "l_min": r.s, "l_max": r.s, "lengths": r.lengths,
                    "extra": format!("外径 {} 耳孔 Ø{}", trim(r.dc), trim(r.ear)),
                })
            })
            .collect();
        m.insert(
            "ring_893".into(),
            serde_json::json!({
                "id": "ring_893", "name": "孔用弹性挡圈 A型", "code": "GB/T 893-2017",
                "iso": "—", "implemented": true,
                "views": views_json("ring_893"), "sizes": sizes,
                "len_label": "厚度 s", "base_hint": "基点 = 端面中心（轴线）",
                "tree_path": "零件库/挡圈/孔用弹性挡圈 A型 GB/T 893-2017",
            }),
        );
    }

    // ── ring_894 ──
    {
        let sizes: Vec<serde_json::Value> = ring894_table()
            .rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "d": r.d, "label": format!("Ø{}", trim(r.d)), "pitch": 0.0,
                    "l_min": r.s, "l_max": r.s, "lengths": r.lengths,
                    "extra": format!("内径 {} 耳孔 Ø{}", trim(r.d3), trim(r.ear)),
                })
            })
            .collect();
        m.insert(
            "ring_894".into(),
            serde_json::json!({
                "id": "ring_894", "name": "轴用弹性挡圈 A型", "code": "GB/T 894-2017",
                "iso": "—", "implemented": true,
                "views": views_json("ring_894"), "sizes": sizes,
                "len_label": "厚度 s", "base_hint": "基点 = 端面中心（轴线）",
                "tree_path": "零件库/挡圈/轴用弹性挡圈 A型 GB/T 894-2017",
            }),
        );
    }

    m
}

/// 本组族的生成派发（`partgen_more::generate` 会先问这里）。
pub fn generate(family: &str, d: f64, l: f64, view: &str) -> Option<Result<GenPart, String>> {
    let allowed = family_views(family);
    if allowed.is_empty() {
        return None;
    }
    if !allowed.contains(&view) {
        return Some(Err(crate::i18n::t_fmt(
            "cmd.parts.err.view_not_offered",
            &[
                ("family", family),
                ("view", view),
                ("avail", &allowed.join("/")),
            ],
        )));
    }
    match family {
        "nut_6170" => Some(nut_6170(d, l, view)),
        "ring_893" => Some(ring_893(d, view)),
        "ring_894" => Some(ring_894(d, view)),
        _ => None,
    }
}

// ══════════════════════════════════════════════════════════════════════════
// nut_6170：1型六角螺母（四视图；模板逐条反解）
// ══════════════════════════════════════════════════════════════════════════

fn nut_weight_kg(row: &NutRow, d: f64) -> f64 {
    // 正六边形面积 = (√3/2)s² ≈ 0.8660254·s²，减牙底孔
    let hex = 0.866_025_403_784_438_6 * row.s * row.s;
    let hole = std::f64::consts::PI / 4.0 * (0.85 * d).powi(2);
    (hex - hole).max(0.0) * row.m * 7.85e-3 / 1000.0
}

fn nut_6170(d: f64, _l: f64, view: &str) -> Result<GenPart, String> {
    let row = nut_table()
        .row(d)
        .ok_or_else(|| {
            crate::i18n::t_fmt(
                "cmd.parts.err.no_m",
                &[("table", "GB/T 6170-2015"), ("d", &trim(d))],
            )
        })?;
    let meta = PartMeta {
        code: "GB/T 6170-2015".into(),
        name: crate::i18n::t_data("1型六角螺母").into(),
        spec: format!("M{}", trim(d)),
        material: String::new(),
        weight: format!("{:.4}", nut_weight_kg(row, d)),
    };
    if view == "section" {
        return nut_6170_section(d, row, meta);
    }

    let (s, m) = (row.s, row.m);
    let e = crate::partgen::across_corners(s); // 公称对角宽（模板实测 ±7.5055）
    let dm = 0.85 * d; // 简化画法牙底
    let delta = chamfer_run(s, e);
    let u_big = ((e / 4.0).powi(2) - delta * delta) / (2.0 * delta);
    let r_big = u_big + delta;
    let al_big = (e / 4.0 / u_big).atan().to_degrees();
    let mut en: Vec<EntityType> = Vec::new();
    let th = 2.0 * (8.0 * delta / e).atan();
    let r2 = (e / 8.0) / th.sin();

    match view {
        "main" => {
            en.push(line([0.0, s / 2.0], [0.0, -s / 2.0], LAYER_MAIN));
            en.push(line([m, s / 2.0], [m, -s / 2.0], LAYER_MAIN));
            for sgn in [1.0, -1.0] {
                en.push(line([0.0, sgn * s / 2.0], [delta, sgn * e / 2.0], LAYER_MAIN));
                en.push(line([m, sgn * s / 2.0], [m - delta, sgn * e / 2.0], LAYER_MAIN));
                en.push(line([delta, sgn * e / 2.0], [m - delta, sgn * e / 2.0], LAYER_MAIN));
                en.push(line([delta, sgn * e / 4.0], [m - delta, sgn * e / 4.0], LAYER_MAIN));
            }
            en.push(arc([r2, -0.375 * e], r2, 180.0 - th.to_degrees(), 180.0, LAYER_MAIN));
            en.push(arc([r2, 0.375 * e], r2, 180.0, 180.0 + th.to_degrees(), LAYER_MAIN));
            en.push(arc([m - r2, -0.375 * e], r2, 0.0, th.to_degrees(), LAYER_MAIN));
            en.push(arc([m - r2, 0.375 * e], r2, -th.to_degrees(), 0.0, LAYER_MAIN));
            en.push(arc([r_big, 0.0], r_big, 180.0 - al_big, 180.0 + al_big, LAYER_MAIN));
            en.push(arc([m - r_big, 0.0], r_big, -al_big, al_big, LAYER_MAIN));
            en.push(line([-AXIS_OVER, 0.0], [m + AXIS_OVER, 0.0], LAYER_CENTER));
            Ok(GenPart { entities: en, meta, bbox: [0.0, -e / 2.0, m, e / 2.0] })
        }
        "top" => {
            en.push(line([0.0, s / 2.0], [0.0, -s / 2.0], LAYER_MAIN));
            en.push(line([m, s / 2.0], [m, -s / 2.0], LAYER_MAIN));
            en.push(line([0.0, s / 2.0], [m, s / 2.0], LAYER_MAIN));
            en.push(line([0.0, -s / 2.0], [m, -s / 2.0], LAYER_MAIN));
            en.push(line([delta, 0.0], [m - delta, 0.0], LAYER_MAIN));
            let th2 = 2.0 * (4.0 * delta / s).atan();
            let r3 = (s / 4.0) / th2.sin();
            for sgn in [1.0, -1.0] {
                en.push(arc([r3, sgn * s / 4.0], r3, 180.0 - th2.to_degrees(), 180.0 + th2.to_degrees(), LAYER_MAIN));
                en.push(arc([m - r3, sgn * s / 4.0], r3, -th2.to_degrees(), th2.to_degrees(), LAYER_MAIN));
            }
            en.push(line([-AXIS_OVER, 0.0], [m + AXIS_OVER, 0.0], LAYER_CENTER));
            Ok(GenPart { entities: en, meta, bbox: [0.0, -s / 2.0, m, s / 2.0] })
        }
        "end" => {
            let v = [
                [0.0, e / 2.0],
                [s / 2.0, e / 4.0],
                [s / 2.0, -e / 4.0],
                [0.0, -e / 2.0],
                [-s / 2.0, -e / 4.0],
                [-s / 2.0, e / 4.0],
            ];
            for i in 0..6 {
                en.push(line(v[i], v[(i + 1) % 6], LAYER_MAIN));
            }
            en.push(circle([0.0, 0.0], s / 2.0, LAYER_MAIN));
            en.push(circle([0.0, 0.0], dm / 2.0, LAYER_MAIN));
            en.push(arc([0.0, 0.0], d / 2.0, 270.0, 180.0, LAYER_THIN));
            let o = s / 2.0 + AXIS_OVER;
            en.push(line([-o, 0.0], [o, 0.0], LAYER_CENTER));
            en.push(line([0.0, o], [0.0, -o], LAYER_CENTER));
            Ok(GenPart { entities: en, meta, bbox: [-s / 2.0, -e / 2.0, s / 2.0, e / 2.0] })
        }
        other => Err(crate::i18n::t_fmt(
            "cmd.parts.err.view_absent",
            &[
                ("family", "nut_6170"),
                ("view", other),
                ("avail", "main/top/end/section"),
            ],
        )),
    }
}

fn nut_6170_section(d: f64, row: &NutRow, meta: PartMeta) -> Result<GenPart, String> {
    let (s, m) = (row.s, row.m);
    let e = crate::partgen::across_corners(s);
    let dm = 0.85 * d;
    let delta = chamfer_run(s, e);
    let c45 = (d - dm) / 2.0; // 端面 45° 倒角轴向宽 = 0.075d（模板 M8 = 0.6）
    let mut en: Vec<EntityType> = Vec::new();
    en.push(line([0.0, s / 2.0], [0.0, -s / 2.0], LAYER_MAIN));
    en.push(line([m, s / 2.0], [m, -s / 2.0], LAYER_MAIN));
    for sgn in [1.0, -1.0] {
        en.push(line([0.0, sgn * s / 2.0], [delta, sgn * e / 2.0], LAYER_MAIN));
        en.push(line([m, sgn * s / 2.0], [m - delta, sgn * e / 2.0], LAYER_MAIN));
        en.push(line([delta, sgn * e / 2.0], [m - delta, sgn * e / 2.0], LAYER_MAIN));
    }
    for sgn in [1.0, -1.0] {
        en.push(line([0.0, sgn * d / 2.0], [c45, sgn * dm / 2.0], LAYER_MAIN));
        en.push(line([m, sgn * d / 2.0], [m - c45, sgn * dm / 2.0], LAYER_MAIN));
        en.push(line([0.0, sgn * d / 2.0], [m, sgn * d / 2.0], LAYER_THIN));
    }
    en.push(line([c45, dm / 2.0], [c45, -dm / 2.0], LAYER_MAIN));
    en.push(line([m - c45, dm / 2.0], [m - c45, -dm / 2.0], LAYER_MAIN));
    for sgn in [1.0, -1.0] {
        en.push(line([c45, sgn * dm / 2.0], [m - c45, sgn * dm / 2.0], LAYER_MAIN));
    }
    // 两片 ANSI31 剖面线（5剖面线层）：边界 = 被切材料轮廓（模板 8 顶点/片，角 0°/270°）
    for (sgn, ang) in [(-1.0, 0.0), (1.0, 270.0)] {
        let verts: Vec<[f64; 2]> = vec![
            [0.0, sgn * d / 2.0],
            [c45, sgn * dm / 2.0],
            [m - c45, sgn * dm / 2.0],
            [m, sgn * d / 2.0],
            [m, sgn * s / 2.0],
            [m - delta, sgn * e / 2.0],
            [delta, sgn * e / 2.0],
            [0.0, sgn * s / 2.0],
        ];
        en.push(hatch_ansi31(&verts, ang));
    }
    en.push(line([-AXIS_OVER, 0.0], [m + AXIS_OVER, 0.0], LAYER_CENTER));
    Ok(GenPart { entities: en, meta, bbox: [0.0, -e / 2.0, m, e / 2.0] })
}

// ══════════════════════════════════════════════════════════════════════════
// 挡圈公共几何
// ══════════════════════════════════════════════════════════════════════════

/// 两圆交点（返回两解；无交点 None）。
fn circle_isect(c0: [f64; 2], r0: f64, c1: [f64; 2], r1: f64) -> Option<([f64; 2], [f64; 2])> {
    let dx = c1[0] - c0[0];
    let dy = c1[1] - c0[1];
    let d = (dx * dx + dy * dy).sqrt();
    if d < 1e-9 || d > r0 + r1 || d < (r0 - r1).abs() {
        return None;
    }
    let a = (r0 * r0 - r1 * r1 + d * d) / (2.0 * d);
    let h2 = r0 * r0 - a * a;
    let h = if h2 > 0.0 { h2.sqrt() } else { 0.0 };
    let xm = c0[0] + a * dx / d;
    let ym = c0[1] + a * dy / d;
    let rx = -dy * (h / d);
    let ry = dx * (h / d);
    Some(([xm + rx, ym + ry], [xm - rx, ym - ry]))
}

fn norm360(deg: f64) -> f64 {
    let mut a = deg % 360.0;
    if a < 0.0 {
        a += 360.0;
    }
    a
}

fn ang_at(c: [f64; 2], p: [f64; 2]) -> f64 {
    (p[1] - c[1]).atan2(p[0] - c[0]).to_degrees()
}

/// 从点 `t` 向圆 `(b, r)` 作切线，返回切点相对圆心的角度（度），取模板那一支。
fn tangent_angle(t: [f64; 2], b: [f64; 2], r: f64) -> f64 {
    let tb = [t[0] - b[0], t[1] - b[1]];
    let d = (tb[0] * tb[0] + tb[1] * tb[1]).sqrt();
    let phi = tb[1].atan2(tb[0]);
    let alpha = if d > r { (r / d).acos() } else { 0.0 };
    (phi - alpha).to_degrees()
}

// ══════════════════════════════════════════════════════════════════════════
// ring_893：孔用弹性挡圈 A型
// ══════════════════════════════════════════════════════════════════════════

/// 模板（规格 50, 外半径 27.1）实测比例：耳孔圆心 / 外半径。
const R893_BX_K: f64 = 11.6139 / 27.1;
const R893_BY_K: f64 = 20.1992 / 27.1;
/// 内圈圆心偏移 e_i = (dc − d) · k（模板 4.2×2/7 = 1.2）。
const R893_EI_K: f64 = 2.0 / 7.0;
/// 外圈在开口侧的止角（模板 292.5° = −67.5°）。
const R893_GAP_DEG: f64 = -67.5;

fn ring_893(d: f64, view: &str) -> Result<GenPart, String> {
    let row = ring893_table()
        .row(d)
        .ok_or_else(|| {
            crate::i18n::t_fmt(
                "cmd.parts.err.no_dia",
                &[("table", "GB/T 893-2017 A型"), ("d", &trim(d))],
            )
        })?;
    let meta = PartMeta {
        code: "GB/T 893-2017".into(),
        name: crate::i18n::t_data("孔用弹性挡圈 A型").into(),
        spec: format!("d{}", trim(d)),
        material: String::new(),
        weight: weight_text(ring893_weight_kg(row)),
    };
    match view {
        "main" => Ok(ring893_main(row, meta)),
        "end" => Ok(ring893_end(row, meta)),
        other => Err(crate::i18n::t_fmt(
            "cmd.parts.err.view_absent",
            &[("family", "ring_893"), ("view", other), ("avail", "main/end")],
        )),
    }
}

fn ring893_weight_kg(row: &Ring893Row) -> f64 {
    let ro = row.dc / 2.0;
    let ri = (row.d - row.d1_min) / 2.0;
    let area = std::f64::consts::PI * (ro * ro - ri * ri); // 环面积（开口忽略）
    area * row.s * 7.85e-3 / 1000.0
}

fn ring893_main(row: &Ring893Row, meta: PartMeta) -> GenPart {
    let (d, dc) = (row.d, row.dc);
    let ro = dc / 2.0;
    let ri = (d - row.d1_min) / 2.0;
    let e_i = (dc - d) * R893_EI_K;
    let r_h = row.ear / 2.0;
    let r_b = row.ear;
    let bx = ro * R893_BX_K;
    let by = ro * R893_BY_K;
    let i = [0.0, -e_i];
    let b = [bx, -by];
    let theta_g = R893_GAP_DEG.to_radians();
    let t = [ro * theta_g.cos(), ro * theta_g.sin()];

    let mut en: Vec<EntityType> = Vec::new();
    // 外圈（整圈主体，开口在下）
    en.push(arc([0.0, 0.0], ro, norm360(R893_GAP_DEG), norm360(180.0 - R893_GAP_DEG), LAYER_MAIN));
    en.push(line([-ro - AXIS_OVER, 0.0], [ro + AXIS_OVER, 0.0], LAYER_CENTER));
    en.push(line([0.0, ro + AXIS_OVER], [0.0, -ro - AXIS_OVER], LAYER_CENTER));

    // 内外圈交点 E（取 x 大者）→ 决定耳弧与内圈止角
    if let Some((e1, e2)) = circle_isect(i, ri, b, r_b) {
        let e = if e1[0] >= e2[0] { e1 } else { e2 };
        let ang_e_boss = ang_at(b, e);
        let ang_e_inner = ang_at(i, e);
        en.push(arc(i, ri, norm360(ang_e_inner), norm360(180.0 - ang_e_inner), LAYER_MAIN));
        // 耳弧：E → P（经上方/左方，模板 5.131°→202.501°）
        let ang_p = tangent_angle(t, b, r_b);
        en.push(arc(b, r_b, norm360(ang_e_boss), norm360(ang_p), LAYER_MAIN));
        // 切线：外圈止点 T → 耳弧切点 P
        let p = [b[0] + r_b * ang_p.to_radians().cos(), b[1] + r_b * ang_p.to_radians().sin()];
        en.push(line(t, p, LAYER_MAIN));
        // 左半镜像
        let bm = [-b[0], b[1]];
        let em = [-e[0], e[1]];
        let ang_e_boss_m = ang_at(bm, em);
        let ang_p_m = 180.0 - ang_p;
        en.push(arc(bm, r_b, norm360(ang_p_m), norm360(ang_e_boss_m), LAYER_MAIN));
        en.push(line([-t[0], t[1]], [-p[0], p[1]], LAYER_MAIN));
        en.push(circle(b, r_h, LAYER_MAIN));
        en.push(circle(bm, r_h, LAYER_MAIN));
    } else {
        // 退化保护（极小规格）：只画内外圈
        en.push(arc(i, ri, norm360(R893_GAP_DEG), norm360(180.0 - R893_GAP_DEG), LAYER_MAIN));
    }
    GenPart { entities: en, meta, bbox: [-ro, -ro, ro, ro] }
}

fn ring893_end(row: &Ring893Row, meta: PartMeta) -> GenPart {
    let (d, dc, s) = (row.d, row.dc, row.s);
    let ro = dc / 2.0;
    let ri = (d - row.d1_min) / 2.0;
    let e_i = (dc - d) * R893_EI_K;
    let r_b = row.ear;
    let by = ro * R893_BY_K;
    let y_out = ro;
    let y_in = ri - e_i;
    let y_bottom = ro * R893_GAP_DEG.to_radians().sin();
    let y_boss = -(by - r_b);
    let mut en: Vec<EntityType> = Vec::new();
    en.push(line([0.0, y_out], [s, y_out], LAYER_MAIN));
    en.push(line([0.0, y_in], [s, y_in], LAYER_MAIN));
    en.push(line([0.0, y_bottom], [s, y_bottom], LAYER_MAIN));
    en.push(line([0.0, y_boss], [s, y_boss], LAYER_MAIN));
    en.push(line([0.0, y_out], [0.0, y_bottom], LAYER_MAIN));
    en.push(line([s, y_out], [s, y_bottom], LAYER_MAIN));
    en.push(line([-AXIS_OVER, 0.0], [s + AXIS_OVER, 0.0], LAYER_CENTER));
    // 剖面线（模板 左视图 1 片，ANSI31 angle=0°/scale=1.0）：
    // 边界 = 模板实测矩形 (0, y_in)-(0, y_out)-(s, y_out)-(s, y_in)，
    // 其中 y_in = ri−e_i = 22.55、y_out = ro = 27.1、s = 2.0（规格 50）。
    en.push(hatch_ansi31_scaled(
        &[[0.0, y_in], [0.0, y_out], [s, y_out], [s, y_in]],
        0.0,
        1.0,
    ));
    GenPart { entities: en, meta, bbox: [0.0, y_bottom, s, y_out] }
}

// ══════════════════════════════════════════════════════════════════════════
// ring_894：轴用弹性挡圈 A型
// ══════════════════════════════════════════════════════════════════════════

/// 模板（规格 40, 内半径 18.25）实测比例（均相对内半径 Ri）。
const R894_RA_K: f64 = 22.0 / 18.25; // 外弧半径
const R894_EO_K: f64 = 1.25 / 18.25; // 外弧圆心 y 偏移（模板 = ear/2；按 Ri 比例以免小规格外弧盖不住耳）
const R894_BY_K: f64 = 20.5777 / 18.25; // 耳孔圆心 y
const R894_X1_K: f64 = 1.5 / 18.25; // 耳内侧竖线 x
const R894_X2_K: f64 = 9.5 / 18.25; // 耳外侧竖线 x
const R894_RE_K: f64 = 24.3 / 18.25; // 耳外弧半径

/// 894 主视图半边的派生几何。
struct R894 {
    ri: f64,
    ra: f64,
    e_o: f64,
    x1: f64,
    x2: f64,
    by: f64,
    r_e: f64,
    r_h: f64,
    y_in: f64,
    y_out: f64,
    y_ear1: f64,
    y_ear2: f64,
}

fn r894_geom(row: &Ring894Row) -> R894 {
    let ri = row.d3 / 2.0;
    let ra = ri * R894_RA_K;
    let e_o = ri * R894_EO_K;
    let x1 = ri * R894_X1_K;
    let x2 = ri * R894_X2_K;
    let bx = (x1 + x2) / 2.0;
    let by = ri * R894_BY_K;
    let r_h = row.ear / 2.0;
    // 耳外弧半径：模板比例；小规格时保证耳孔完全落在耳弧内（否则孔会穿出轮廓）
    let hole_r = (bx * bx + by * by).sqrt();
    let r_e = (ri * R894_RE_K).max(hole_r + r_h * 1.05);
    let y_in = -(ri * ri - x1 * x1).max(0.0).sqrt();
    let y_out = e_o - (ra * ra - x2 * x2).max(0.0).sqrt();
    let y_ear1 = -(r_e * r_e - x1 * x1).max(0.0).sqrt();
    let y_ear2 = -(r_e * r_e - x2 * x2).max(0.0).sqrt();
    R894 { ri, ra, e_o, x1, x2, by, r_e, r_h, y_in, y_out, y_ear1, y_ear2 }
}

fn ring_894(d: f64, view: &str) -> Result<GenPart, String> {
    let row = ring894_table()
        .row(d)
        .ok_or_else(|| {
            crate::i18n::t_fmt(
                "cmd.parts.err.no_dia",
                &[("table", "GB/T 894-2017 A型"), ("d", &trim(d))],
            )
        })?;
    let meta = PartMeta {
        code: "GB/T 894-2017".into(),
        name: crate::i18n::t_data("轴用弹性挡圈 A型").into(),
        spec: format!("d{}", trim(d)),
        material: String::new(),
        weight: weight_text(ring894_weight_kg(row)),
    };
    match view {
        "main" => Ok(ring894_main(row, meta)),
        "end" => Ok(ring894_end(row, meta)),
        other => Err(crate::i18n::t_fmt(
            "cmd.parts.err.view_absent",
            &[("family", "ring_894"), ("view", other), ("avail", "main/end")],
        )),
    }
}

fn ring894_weight_kg(row: &Ring894Row) -> f64 {
    let ri = row.d3 / 2.0;
    let ra = ri * R894_RA_K;
    let area = std::f64::consts::PI * (ra * ra - ri * ri);
    area * row.s * 7.85e-3 / 1000.0
}

fn ring894_main(row: &Ring894Row, meta: PartMeta) -> GenPart {
    let g = r894_geom(row);
    let R894 { ri, ra, e_o, x1, x2, by, r_e, r_h, y_in, y_out, y_ear1, y_ear2 } = g;
    let bx = (x1 + x2) / 2.0; // 耳孔圆心 x（模板 5.5 = (1.5+9.5)/2）
    let mut en: Vec<EntityType> = Vec::new();
    // 内圈（整圈主体，开口在下）
    let ai = ang_at([0.0, 0.0], [x1, y_in]);
    en.push(arc([0.0, 0.0], ri, norm360(ai), norm360(180.0 - ai), LAYER_MAIN));
    // 外弧（圆心 (0,e_o)）
    let ao = ang_at([0.0, e_o], [x2, y_out]);
    en.push(arc([0.0, e_o], ra, norm360(ao), norm360(180.0 - ao), LAYER_MAIN));
    // 耳外弧（圆心原点，右耳）与两竖线
    let ae1 = ang_at([0.0, 0.0], [x1, y_ear1]);
    let ae2 = ang_at([0.0, 0.0], [x2, y_ear2]);
    en.push(arc([0.0, 0.0], r_e, norm360(ae1), norm360(ae2), LAYER_MAIN));
    for sgn in [1.0, -1.0] {
        en.push(line([sgn * x1, y_in], [sgn * x1, y_ear1], LAYER_MAIN));
        en.push(line([sgn * x2, y_out], [sgn * x2, y_ear2], LAYER_MAIN));
        en.push(circle([sgn * bx, -by], r_h, LAYER_MAIN));
    }
    // 左耳外弧（镜像）
    en.push(arc([0.0, 0.0], r_e, norm360(180.0 - ae2), norm360(180.0 - ae1), LAYER_MAIN));
    let o = e_o + ra + AXIS_OVER;
    en.push(line([-o, 0.0], [o, 0.0], LAYER_CENTER));
    en.push(line([0.0, o], [0.0, -o], LAYER_CENTER));
    GenPart { entities: en, meta, bbox: [-r_e, -r_e, r_e, o] }
}

fn ring894_end(row: &Ring894Row, meta: PartMeta) -> GenPart {
    let g = r894_geom(row);
    let s = row.s;
    let y_out = g.e_o + g.ra;
    let y_bottom = g.y_ear1.min(g.y_ear2);
    let mut en: Vec<EntityType> = Vec::new();
    en.push(line([0.0, y_out], [s, y_out], LAYER_MAIN));
    en.push(line([0.0, g.ri], [s, g.ri], LAYER_MAIN));
    en.push(line([0.0, y_bottom], [s, y_bottom], LAYER_MAIN));
    en.push(line([0.0, g.y_in], [s, g.y_in], LAYER_MAIN));
    en.push(line([0.0, y_out], [0.0, y_bottom], LAYER_MAIN));
    en.push(line([s, y_out], [s, y_bottom], LAYER_MAIN));
    en.push(line([-AXIS_OVER, 0.0], [s + AXIS_OVER, 0.0], LAYER_CENTER));
    // 剖面线（模板 左视图 1 片，ANSI31 angle=0°/scale=1.5）：
    // 边界 = 矩形 (0, ri)-(0, y_out)-(s, y_out)-(s, ri)；模板用 1986 版 s=1.5，
    // 本库数据表用 2017 版 s=1.75（既有 `R894_END` 回归已按 0.30 容差记该版本差异）。
    en.push(hatch_ansi31_scaled(
        &[[0.0, g.ri], [0.0, y_out], [s, y_out], [s, g.ri]],
        0.0,
        1.5,
    ));
    GenPart { entities: en, meta, bbox: [0.0, y_bottom, s, y_out] }
}

// ══════════════════════════════════════════════════════════════════════════
// 测试
// ══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 语言是进程级全局：断言中文文案的用例共用锁并钉死 zh。
    fn zh_guard() -> std::sync::MutexGuard<'static, ()> {
        let g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        g
    }

    use crate::partgen_kit::{LAYER_CENTER, LAYER_HATCH, LAYER_HIDDEN, LAYER_MAIN, LAYER_THIN};

    /// 每个规格 × 每个视图都能生成；首个长度（= s / m）能出图。
    #[test]
    fn every_spec_and_view_generates() {
        for d in nut_diameters() {
            let row = nut_table().row(d).unwrap();
            let l = *row.lengths.first().unwrap_or(&row.m);
            for v in family_views("nut_6170") {
                let p = generate("nut_6170", d, l, v)
                    .unwrap()
                    .unwrap_or_else(|e| panic!("nut_6170 M{} {v}: {e}", trim(d)));
                assert!(!p.entities.is_empty(), "nut_6170 M{} {v} 空", trim(d));
            }
        }
        for (fam, ds) in [("ring_893", ring893_diameters()), ("ring_894", ring894_diameters())] {
            for d in ds {
                let l = if fam == "ring_893" {
                    ring893_table().row(d).unwrap().s
                } else {
                    ring894_table().row(d).unwrap().s
                };
                for v in family_views(fam) {
                    let p = generate(fam, d, l, v)
                        .unwrap()
                        .unwrap_or_else(|e| panic!("{fam} Ø{} {v}: {e}", trim(d)));
                    assert!(!p.entities.is_empty(), "{fam} Ø{} {v} 空", trim(d));
                }
            }
        }
    }

    /// 非本组族：`generate` 必须返回 None；视图越界必须报错。
    #[test]
    fn dispatch_ownership_and_view_check() {
        let _g = zh_guard();
        assert!(generate("hex_bolt_c", 10.0, 20.0, "main").is_none());
        assert!(generate("washer_971", 10.0, 10.0, "main").is_none());
        let e = generate("nut_6170", 10.0, 8.4, "side").unwrap().unwrap_err();
        assert!(e.contains("不提供视图"), "{e}");
        let e = generate("ring_893", 50.0, 2.0, "top").unwrap().unwrap_err();
        assert!(e.contains("不提供视图"), "{e}");
    }

    /// 目录接线：三族经 `partgen::catalog_json` 上树/带视图，最小规格 × 首长度 × 每视图能出图。
    #[test]
    fn catalog_integration() {
        let cat: serde_json::Value = serde_json::from_str(&crate::partgen::catalog_json()).unwrap();
        for fam in ["nut_6170", "ring_893", "ring_894"] {
            let sizes = crate::partgen::family_sizes(fam);
            assert!(!sizes.is_empty(), "{fam} 无规格");
            let row = &sizes[0];
            let l = *row.lengths.first().unwrap_or(&row.d);
            for v in family_views(fam) {
                let p = crate::partgen::generate(fam, row.d, l, v)
                    .unwrap_or_else(|e| panic!("{fam} Ø{} {v}: {e}", trim(row.d)));
                assert!(!p.entities.is_empty(), "{fam} {v} 空");
            }
            let views = cat["families"][fam]["views"].as_array().unwrap();
            assert_eq!(views.len(), family_views(fam).len(), "{fam} views 不一致");
            assert!(cat["families"][fam]["tree_path"].as_str().is_some(), "{fam} 缺 tree_path");
        }
    }

    /// 全族全视图：图层只落在 5 个 OCSM 层内、颜色 ByLayer、无尺寸标注。
    #[test]
    fn layers_and_no_dimension() {
        let mut check = |fam: &str, d: f64, l: f64| {
            for v in family_views(fam) {
                let p = generate(fam, d, l, v).unwrap().unwrap();
                for e in &p.entities {
                    let lay = e.common().layer.as_str();
                    assert!(
                        [LAYER_MAIN, LAYER_THIN, LAYER_CENTER, LAYER_HIDDEN, LAYER_HATCH].contains(&lay),
                        "{fam} Ø{d} {v} 图层越界: {lay}"
                    );
                    assert!(
                        matches!(e.common().color, ocs_plugin_api::host::acadrust::types::Color::ByLayer),
                        "{fam} Ø{d} {v} 颜色非 ByLayer"
                    );
                    assert!(!matches!(e, EntityType::Dimension(_)), "{fam} Ø{d} {v} 含尺寸标注");
                }
            }
        };
        for d in nut_diameters() {
            check("nut_6170", d, nut_table().row(d).unwrap().m);
        }
        for d in ring893_diameters() {
            check("ring_893", d, ring893_table().row(d).unwrap().s);
        }
        for d in ring894_diameters() {
            check("ring_894", d, ring894_table().row(d).unwrap().s);
        }
    }

    // ── 模板逐条数值回归 ──────────────────────────────────────────────────
    //
    // 模板坐标（ref_dump 精确 TSV，圆弧角度为度）。缺省基准：
    //   * nut 主/俯/剖：模板 x∈[−6.8,0] → 本库基点“端面×轴线” = 左端面 x=0，故整体 +m 平移；
    //     模板里 3 条“右端弧”是把左端弧平移 +m 的制图笔误，叠合时按 x>m 反射修正。
    //   * 挡圈：模板原点 = 挡圈中心 = 本库原点；模板为半视图，叠合时镜像补全。
    // 回归采用“点到几何最近距离”：每个模板采样点到生成件最近图元的距离 < tol。

    #[derive(Clone, Copy)]
    enum Tpl {
        L(&'static str, [f64; 4]),
        A(&'static str, [f64; 5]),
        C(&'static str, [f64; 3]),
    }

const NUT_MAIN: &[Tpl] = &[
    Tpl::A("M",[-3.4774, -5.6292, 3.3226, 145.6158, 180.0]),
    Tpl::A("M",[5.6194, 0.0, 12.4194, 162.412, 197.588]),
    Tpl::L("M",[-6.8, 6.5, -6.8, -6.5]),
    Tpl::L("C",[-9.8, 0.0, 3.0, 0.0]),
    Tpl::L("M",[-6.2194, -7.5055, -0.5806, -7.5055]),
    Tpl::L("M",[-6.2194, -3.7528, -0.5806, -3.7528]),
    Tpl::L("M",[-6.8, -6.5, -6.2194, -7.5055]),
    Tpl::A("M",[3.4774, 5.6292, 3.3226, 325.6158, 360.0]),
    Tpl::L("M",[-6.8, 6.5, -6.2194, 7.5055]),
    Tpl::L("M",[-6.2194, 7.5055, -0.5806, 7.5055]),
    Tpl::L("M",[-6.2194, 3.7528, -0.5806, 3.7528]),
    Tpl::L("M",[0.0, 6.5, 0.0, -6.5]),
    Tpl::A("M",[3.3226, -5.6292, 3.3226, 145.6158, 180.0]),
    Tpl::A("M",[12.4194, 0.0, 12.4194, 162.412, 197.588]),
    Tpl::A("M",[-3.3226, 5.6292, 3.3226, 325.6158, 360.0]),
    Tpl::L("M",[0.0, 6.5, -0.5806, 7.5055]),
    Tpl::L("M",[0.0, -6.5, -0.5806, -7.5055]),
];

const NUT_TOP: &[Tpl] = &[
    Tpl::L("M",[-6.8, 6.5, -6.8, -6.5]),
    Tpl::L("C",[-9.8, 0.0, 3.0, 0.0]),
    Tpl::L("M",[-6.8, 6.5, 0.0, 6.5]),
    Tpl::L("M",[-6.2194, 0.0, -0.5806, 0.0]),
    Tpl::L("M",[-6.8, -6.5, 0.0, -6.5]),
    Tpl::A("M",[2.5871, 3.25, 9.3871, 159.7437, 200.2563]),
    Tpl::A("M",[2.5871, -3.25, 9.3871, 159.7437, 200.2563]),
    Tpl::L("M",[0.0, 6.5, 0.0, -6.5]),
    Tpl::A("M",[9.3871, 3.25, 9.3871, 159.7437, 200.2563]),
    Tpl::A("M",[9.3871, -3.25, 9.3871, 159.7437, 200.2563]),
];

const NUT_END: &[Tpl] = &[
    Tpl::L("M",[0.0, 7.5055, -6.5, 3.7528]),
    Tpl::C("M",[0.0, 0.0, 6.5]),
    Tpl::L("C",[-9.5, 0.0, 9.5, 0.0]),
    Tpl::L("C",[0.0, 9.5, 0.0, -9.5]),
    Tpl::C("M",[0.0, 0.0, 3.4]),
    Tpl::A("T",[0.0, 0.0, 4.0, 270.0, 180.0]),
    Tpl::L("M",[-6.5, 3.7528, -6.5, -3.7528]),
    Tpl::L("M",[-6.5, -3.7528, 0.0, -7.5056]),
    Tpl::L("M",[0.0, -7.5055, 6.5, -3.7528]),
    Tpl::L("M",[6.5, -3.7528, 6.5, 3.7528]),
    Tpl::L("M",[6.5, 3.7528, 0.0, 7.5056]),
];

const NUT_SECTION: &[Tpl] = &[
    Tpl::L("M",[-6.8, -4.0, -6.2, -3.4]),
    Tpl::L("T",[-6.8, -4.0, 0.0, -4.0]),
    Tpl::L("M",[-6.2, -3.4, -0.6, -3.4]),
    Tpl::L("M",[-0.6, -3.4, 0.0, -4.0]),
    Tpl::L("M",[-6.8, -6.5, -6.2194, -7.5055]),
    Tpl::L("M",[-6.2194, -7.5055, -0.5806, -7.5055]),
    Tpl::L("M",[-0.5806, -7.5055, 0.0, -6.5]),
    Tpl::L("M",[-6.8, 6.5, -6.8, -6.5]),
    Tpl::L("M",[0.0, 6.5, 0.0, -6.5]),
    Tpl::L("M",[-6.2, 3.4, -6.2, -3.4]),
    Tpl::L("M",[-0.6, 3.4, -0.6, -3.4]),
    Tpl::L("C",[-9.8, 0.0, 3.0, 0.0]),
    Tpl::L("M",[-6.8, 4.0, -6.2, 3.4]),
    Tpl::L("T",[-6.8, 4.0, 0.0, 4.0]),
    Tpl::L("M",[-6.2, 3.4, -0.6, 3.4]),
    Tpl::L("M",[-0.6, 3.4, 0.0, 4.0]),
    Tpl::L("M",[-6.8, 6.5, -6.2194, 7.5055]),
    Tpl::L("M",[-6.2194, 7.5055, -0.5806, 7.5055]),
    Tpl::L("M",[-0.5806, 7.5055, 0.0, 6.5]),
];

const R893_MAIN: &[Tpl] = &[
    Tpl::L("C",[-30.1, 0.0, 30.1, 0.0]),
    Tpl::L("C",[0.0, 30.1, 0.0, -30.1]),
    Tpl::L("M",[8.8423, -21.3473, 10.3707, -25.0371]),
    Tpl::A("M",[0.0, 0.0, 27.1, 292.5, 90.0]),
    Tpl::A("M",[11.6139, -20.1992, 3.0, 5.131, 202.5009]),
    Tpl::A("M",[0.0, -1.2, 23.75, 307.9386, 90.0]),
    Tpl::C("M",[11.6139, -20.1992, 1.5]),
    Tpl::L("C",[6.0706, -22.4953, 17.1572, -17.9031]),
    Tpl::L("C",[13.91, -25.7425, 9.3178, -14.6559]),
    Tpl::L("M",[-8.8423, -21.3473, -10.3707, -25.0371]),
    Tpl::L("C",[-6.0706, -22.4953, -17.1572, -17.9031]),
    Tpl::L("C",[-13.91, -25.7425, -9.3178, -14.6559]),
];

const R893_END: &[Tpl] = &[
    Tpl::L("C",[-3.0, 0.0, 5.0, 0.0]),
    Tpl::L("M",[2.0, -17.1992, 0.0, -17.1992]),
    Tpl::L("M",[0.0, 22.55, 0.0, 27.1]),
    Tpl::L("M",[0.0, 27.1, 2.0, 27.1]),
    Tpl::L("M",[2.0, 27.1, 2.0, 22.55]),
    Tpl::L("M",[2.0, 22.55, 0.0, 22.55]),
    Tpl::L("M",[0.0, 22.55, 0.0, -25.0371]),
    Tpl::L("M",[0.0, -25.0371, 2.0, -25.0371]),
    Tpl::L("M",[2.0, -25.0371, 2.0, 22.55]),
    Tpl::L("C",[-3.0, -20.1992, 5.0, -20.1992]),
];

const R894_MAIN: &[Tpl] = &[
    Tpl::L("C",[-25.0, 0.0, 25.0, 0.0]),
    Tpl::L("C",[0.0, 26.25, 0.0, -23.75]),
    Tpl::A("M",[0.0, 1.25, 22.0, 295.583, 90.0]),
    Tpl::A("M",[0.0, 0.0, 18.25, 274.7145, 90.0]),
    Tpl::C("M",[5.5, -20.5777, 1.25]),
    Tpl::A("M",[0.0, 0.0, 24.3, 273.539, 293.0134]),
    Tpl::L("M",[1.5, -18.1883, 1.5, -24.2537]),
    Tpl::L("M",[9.5, -18.5931, 9.5, -22.366]),
    Tpl::L("C",[1.25, -20.5777, 9.75, -20.5777]),
    Tpl::L("C",[5.5, -24.8277, 5.5, -16.3277]),
    Tpl::L("C",[-1.25, -20.5777, -9.75, -20.5777]),
    Tpl::L("C",[-5.5, -24.8277, -5.5, -16.3277]),
    Tpl::L("M",[-1.5, -18.1883, -1.5, -24.2537]),
    Tpl::L("M",[-9.5, -18.5931, -9.5, -22.366]),
];

const R894_END: &[Tpl] = &[
    Tpl::L("C",[-3.0, 0.0, 4.5, 0.0]),
    Tpl::L("M",[1.5, -18.1883, 0.0, -18.1883]),
    Tpl::L("M",[0.0, 18.25, 0.0, 23.25]),
    Tpl::L("M",[0.0, 23.25, 1.5, 23.25]),
    Tpl::L("M",[1.5, 23.25, 1.5, 18.25]),
    Tpl::L("M",[1.5, 18.25, 0.0, 18.25]),
    Tpl::L("M",[0.0, 18.25, 0.0, -24.2537]),
    Tpl::L("M",[0.0, -24.2537, 1.5, -24.2537]),
    Tpl::L("M",[1.5, -24.2537, 1.5, 18.25]),
];

    /// 模板图元 → 采样点（模板坐标，未变换）。
    fn sample_tpl(t: &Tpl) -> Vec<[f64; 2]> {
        match *t {
            Tpl::L(_, [x1, y1, x2, y2]) => (0..=12)
                .map(|i| {
                    let t = i as f64 / 12.0;
                    [x1 + (x2 - x1) * t, y1 + (y2 - y1) * t]
                })
                .collect(),
            Tpl::A(_, [cx, cy, r, a1, a2]) => {
                let (a1, a2) = (a1.to_radians(), a2.to_radians());
                let mut span = a2 - a1;
                if span <= 0.0 {
                    span += std::f64::consts::TAU;
                }
                (0..=32)
                    .map(|i| {
                        let a = a1 + span * i as f64 / 32.0;
                        [cx + r * a.cos(), cy + r * a.sin()]
                    })
                    .collect()
            }
            Tpl::C(_, [cx, cy, r]) => (0..48)
                .map(|i| {
                    let a = std::f64::consts::TAU * i as f64 / 48.0;
                    [cx + r * a.cos(), cy + r * a.sin()]
                })
                .collect(),
        }
    }

    fn layer_of(t: &Tpl) -> &'static str {
        match t {
            Tpl::L(l, _) | Tpl::A(l, _) | Tpl::C(l, _) => l,
        }
    }

    /// 模板点 → 本库坐标（nut：+m；x>m 反射；ring：可选镜像）。
    fn to_lib(_t: &Tpl, p: [f64; 2], nut_m: Option<f64>) -> [f64; 2] {
        match nut_m {
            Some(m) => {
                let mut x = p[0] + m;
                if x > m {
                    x = 2.0 * m - x;
                }
                [x, p[1]]
            }
            None => p,
        }
    }

    /// 点到 Line 段 / Arc / Circle 的距离。
    fn dist_to_entity(e: &EntityType, p: [f64; 2]) -> f64 {
        match e {
            EntityType::Line(l) => {
                let (a, b) = ([l.start.x, l.start.y], [l.end.x, l.end.y]);
                let ab = [b[0] - a[0], b[1] - a[1]];
                let len2 = ab[0] * ab[0] + ab[1] * ab[1];
                if len2 < 1e-12 {
                    return ((p[0] - a[0]).powi(2) + (p[1] - a[1]).powi(2)).sqrt();
                }
                let t = (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / len2).clamp(0.0, 1.0);
                let q = [a[0] + t * ab[0], a[1] + t * ab[1]];
                ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt()
            }
            EntityType::Circle(c) => {
                let d = ((p[0] - c.center.x).powi(2) + (p[1] - c.center.y).powi(2)).sqrt();
                (d - c.radius).abs()
            }
            EntityType::Arc(a) => {
                let ang = (p[1] - a.center.y).atan2(p[0] - a.center.x).rem_euclid(std::f64::consts::TAU);
                let (s, en) = (a.start_angle.rem_euclid(std::f64::consts::TAU), a.end_angle.rem_euclid(std::f64::consts::TAU));
                let mut span = en - s;
                if span <= 0.0 {
                    span += std::f64::consts::TAU;
                }
                let rel = (ang - s).rem_euclid(std::f64::consts::TAU);
                let d = ((p[0] - a.center.x).powi(2) + (p[1] - a.center.y).powi(2)).sqrt();
                if rel <= span {
                    (d - a.radius).abs()
                } else {
                    // 到两端点的较小距离
                    let p0 = [a.center.x + a.radius * s.cos(), a.center.y + a.radius * s.sin()];
                    let p1 = [a.center.x + a.radius * en.cos(), a.center.y + a.radius * en.sin()];
                    let d0 = ((p[0] - p0[0]).powi(2) + (p[1] - p0[1]).powi(2)).sqrt();
                    let d1 = ((p[0] - p1[0]).powi(2) + (p[1] - p1[1]).powi(2)).sqrt();
                    d0.min(d1)
                }
            }
            _ => f64::INFINITY,
        }
    }

    fn min_dist(part: &GenPart, p: [f64; 2]) -> f64 {
        part.entities
            .iter()
            .map(|e| dist_to_entity(e, p))
            .fold(f64::INFINITY, f64::min)
    }

    /// 模板那档规格的逐条数值回归（点—几何最近距离 ≤ 2e-3）。
    #[test]
    fn template_regression() {
        let nut_m = 6.8_f64;
        // 每例的容差：模板值四舍五入到 4 位 → 2e-3；ring_894 端视厚度是**版本差异**
        // （模板/用户 PNG 用 1986 版 s=1.5，易紧通 2017 版 s=1.75），该视图放宽到 0.3 并单列存疑。
        let cases: [(&str, &str, &[Tpl], Option<f64>, bool, f64); 8] = [
            ("nut_6170", "main", NUT_MAIN, Some(nut_m), false, 2.0e-3),
            ("nut_6170", "top", NUT_TOP, Some(nut_m), false, 2.0e-3),
            ("nut_6170", "end", NUT_END, None, false, 2.0e-3),
            ("nut_6170", "section", NUT_SECTION, Some(nut_m), false, 2.0e-3),
            ("ring_893", "main", R893_MAIN, None, true, 2.0e-3),
            ("ring_893", "end", R893_END, None, false, 2.0e-3),
            ("ring_894", "main", R894_MAIN, None, true, 2.0e-3),
            ("ring_894", "end", R894_END, None, false, 0.30),
        ];
        for (fam, view, tpl, nut_off, mirror, tol) in cases {
            let (d, l) = match fam {
                "nut_6170" => (8.0, 6.8),
                "ring_893" => (50.0, 2.0),
                _ => (40.0, 1.75),
            };
            let part = generate(fam, d, l, view).unwrap().unwrap();
            for t in tpl {
                // 只比 1轮廓实线层（模板其它层另测）
                if layer_of(t) != "M" {
                    continue;
                }
                for p in sample_tpl(t) {
                    let q = to_lib(t, p, nut_off);
                    let dist = min_dist(&part, q);
                    assert!(
                        dist <= tol,
                        "{fam} {view}: 模板点 {p:?} → {q:?} 最近几何距离 {dist:.5} > {tol}",
                    );
                    if mirror {
                        let qm = [-q[0], q[1]];
                        let dm = min_dist(&part, qm);
                        assert!(dm <= tol, "{fam} {view} 镜像点 {qm:?} 距离 {dm:.5} > {tol}");
                    }
                }
            }
        }
    }

    /// 图上无尺寸标注、且至少一个视图含 5剖面线层（螺母剖视）。
    #[test]
    fn no_annotations_nut_section_hatched() {
        let p = generate("nut_6170", 8.0, 6.8, "section").unwrap().unwrap();
        let hatches = p
            .entities
            .iter()
            .filter(|e| e.common().layer == LAYER_HATCH)
            .count();
        assert_eq!(hatches, 2, "剖视图应有上下两片 ANSI31");
        for fam in ["ring_893", "ring_894"] {
            let p = generate(fam, 40.0, 1.75, "main").unwrap().unwrap();
            assert!(
                p.entities.iter().all(|e| e.common().layer != LAYER_HATCH),
                "{fam} 主视图不应有剖面线"
            );
        }
    }

    /// 模板 `左视图` 剖面线回归（模板片数 / angle / scale / 边界逐点）。
    ///
    /// 模板（ezdxf 权威读数，坐标 = 模板坐标）：
    /// - ring_893/end：1 片 ANSI31 angle 0 scale 1.0，矩形 (0,22.55)-(0,27.1)-(2,27.1)-(2,22.55)
    /// - ring_894/end：1 片 ANSI31 angle 0 scale 1.5，矩形 (0,18.25)-(0,23.25)-(1.5,23.25)-(1.5,18.25)
    ///   （x=1.5 是模板 1986 版厚度；本库数据表用 2017 版 s=1.75，见 `template_regression` 的版本说明）
    #[test]
    fn hatch_end_views_match_template() {
        use ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge;
        // 把生成件的 Hatch 边界取成闭合折线（本组端视剖面线都是直线边）
        let hatches = |p: &GenPart| -> Vec<(f64, f64, Vec<[f64; 2]>)> {
            p.entities
                .iter()
                .filter_map(|e| match e {
                    EntityType::Hatch(h) => {
                        let path = h.paths.first()?;
                        let mut pts = Vec::new();
                        for ed in &path.edges {
                            match ed {
                                BoundaryEdge::Line(l) => pts.push([l.start.x, l.start.y]),
                                _ => return None,
                            }
                        }
                        Some((h.pattern_angle.to_degrees(), h.pattern_scale, pts))
                    }
                    _ => None,
                })
                .collect()
        };
        // 顶点集比对（顺序无关；容差 = tol）。
        let verts_match = |got: &[[f64; 2]], want: &[[f64; 2]], tol: f64| -> bool {
            got.len() == want.len()
                && want.iter().all(|w| got.iter().any(|g| (g[0] - w[0]).abs() <= tol && (g[1] - w[1]).abs() <= tol))
        };

        // ring_893 / end（规格 50：ro=27.1，ri=23.75，e_i=1.2 ⇒ y_in=22.55，s=2.0）
        let p = generate("ring_893", 50.0, 2.0, "end").unwrap().unwrap();
        let hs = hatches(&p);
        assert_eq!(hs.len(), 1, "ring_893 左视图应 1 片剖面线");
        assert!((hs[0].0 - 0.0).abs() < 1e-9, "angle={}", hs[0].0);
        assert!((hs[0].1 - 1.0).abs() < 1e-9, "scale={}", hs[0].1);
        assert!(
            verts_match(&hs[0].2, &[[0.0, 22.55], [0.0, 27.1], [2.0, 27.1], [2.0, 22.55]], 1e-3),
            "ring_893 左视边界 = {:?}",
            hs[0].2
        );

        // ring_894 / end（规格 40：ri=18.25，y_out=23.25）
        let p = generate("ring_894", 40.0, 1.75, "end").unwrap().unwrap();
        let s = ring894_table().row(40.0).unwrap().s;
        let hs = hatches(&p);
        assert_eq!(hs.len(), 1, "ring_894 左视图应 1 片剖面线");
        assert!((hs[0].0 - 0.0).abs() < 1e-9, "angle={}", hs[0].0);
        assert!((hs[0].1 - 1.5).abs() < 1e-9, "scale={}", hs[0].1);
        // 边界 = 本库被切材料矩形（x 用本库 s=1.75）
        assert!(
            verts_match(&hs[0].2, &[[0.0, 18.25], [0.0, 23.25], [s, 23.25], [s, 18.25]], 1e-3),
            "ring_894 左视边界 = {:?}",
            hs[0].2
        );
        // 模板 x=1.5 vs 本库 x=s：差值就是已知的 1986/2017 版本厚度差
        assert!((s - 1.5 - 0.25).abs() < 1e-9, "模板/本库厚度版本差应 = 0.25");
    }

    // ── 导出：SVG + 叠合 ──────────────────────────────────────────────────

    fn sample_entity_points(e: &EntityType) -> Vec<[f64; 2]> {
        match e {
            EntityType::Line(l) => (0..=12)
                .map(|i| {
                    let t = i as f64 / 12.0;
                    [l.start.x + (l.end.x - l.start.x) * t, l.start.y + (l.end.y - l.start.y) * t]
                })
                .collect(),
            EntityType::Circle(c) => (0..48)
                .map(|i| {
                    let a = std::f64::consts::TAU * i as f64 / 48.0;
                    [c.center.x + c.radius * a.cos(), c.center.y + c.radius * a.sin()]
                })
                .collect(),
            EntityType::Arc(a) => {
                let (s, en) = (a.start_angle, a.end_angle);
                let mut span = en - s;
                while span <= 0.0 {
                    span += std::f64::consts::TAU;
                }
                (0..=32)
                    .map(|i| {
                        let ang = s + span * i as f64 / 32.0;
                        [a.center.x + a.radius * ang.cos(), a.center.y + a.radius * ang.sin()]
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    /// 生成 SVG（模板红 / 生成蓝，同坐标系；nut 平移 + 反射修正，ring 镜像补全）。
    fn overlay_svg(fam: &str, view: &str) -> String {
        let nut_m = 6.8_f64;
        let nut_off = if fam == "nut_6170" && view != "end" { Some(nut_m) } else { None };
        // ring 端点：模板坐标 = 本库坐标；**仅主视图是半视图**需镜像补全（端视为厚度方向，不镜像）
        let mirror = fam.starts_with("ring_") && view == "main";
        let (tpl, d, l) = match (fam, view) {
            ("nut_6170", "main") => (NUT_MAIN, 8.0, 6.8),
            ("nut_6170", "top") => (NUT_TOP, 8.0, 6.8),
            ("nut_6170", "end") => (NUT_END, 8.0, 6.8),
            ("nut_6170", "section") => (NUT_SECTION, 8.0, 6.8),
            ("ring_893", "main") => (R893_MAIN, 50.0, 2.0),
            ("ring_893", "end") => (R893_END, 50.0, 2.0),
            ("ring_894", "main") => (R894_MAIN, 40.0, 1.75),
            _ => (R894_END, 40.0, 1.75),
        };
        let part = generate(fam, d, l, view).unwrap().unwrap();

        let mut shapes: Vec<(Vec<[f64; 2]>, &'static str)> = Vec::new();
        for t in tpl {
            if layer_of(t) != "M" {
                continue;
            }
            let tp: Vec<[f64; 2]> = sample_tpl(t)
                .iter()
                .map(|p| to_lib(t, *p, nut_off))
                .collect();
            shapes.push((tp.clone(), "red"));
            if mirror {
                shapes.push((tp.iter().map(|p| [-p[0], p[1]]).collect(), "red"));
            }
        }
        for e in &part.entities {
            if e.common().layer != LAYER_MAIN {
                continue;
            }
            let pts = sample_entity_points(e);
            if !pts.is_empty() {
                shapes.push((pts, "blue"));
            }
        }
        let all: Vec<[f64; 2]> = shapes.iter().flat_map(|(p, _)| p.iter().cloned()).collect();
        let xs: Vec<f64> = all.iter().map(|p| p[0]).collect();
        let ys: Vec<f64> = all.iter().map(|p| p[1]).collect();
        let (xmin, xmax) = (xs.iter().cloned().fold(f64::INFINITY, f64::min), xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
        let (ymin, ymax) = (ys.iter().cloned().fold(f64::INFINITY, f64::min), ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
        let pad = ((xmax - xmin).max(ymax - ymin)) * 0.08 + 2.0;
        let (w, h) = (xmax - xmin + 2.0 * pad, ymax - ymin + 2.0 * pad);
        let scale = 900.0 / w.max(h);
        let mut s = String::new();
        use std::fmt::Write as _;
        let _ = write!(
            s,
            "<svg xmlns='http://www.w3.org/2000/svg' width='{:.0}' height='{:.0}' viewBox='0 0 {:.0} {:.0}'><rect width='100%' height='100%' fill='#fff'/>",
            w * scale, h * scale, w * scale, h * scale
        );
        for (shape, col) in &shapes {
            let pts: Vec<String> = shape
                .iter()
                .map(|p| {
                    let x = (p[0] - xmin + pad) * scale;
                    let y = (ymax - p[1] + pad) * scale;
                    format!("{x:.2},{y:.2}")
                })
                .collect();
            let (stroke, sw, op) = if *col == "red" {
                ("#d00000", 1.6, "0.6")
            } else {
                ("#0040d0", 1.0, "0.85")
            };
            let _ = writeln!(
                s,
                "<polyline points='{}' fill='none' stroke='{stroke}' stroke-width='{sw}' stroke-opacity='{op}'/>",
                pts.join(" ")
            );
        }
        s.push_str("</svg>");
        s
    }

    /// 人工看色：每族每视图落 SVG（生成件，`part_svg`）。
    #[test]
    #[ignore]
    fn dump_b1_svg() {
        let dir = std::path::Path::new("/tmp/b1");
        std::fs::create_dir_all(dir).unwrap();
        let cases = [
            ("nut_6170", 8.0, 6.8),
            ("nut_6170", 48.0, 38.0),
            ("ring_893", 50.0, 2.0),
            ("ring_893", 16.0, 1.0),
            ("ring_893", 8.0, 0.8),
            ("ring_894", 40.0, 1.75),
            ("ring_894", 25.0, 1.2),
            ("ring_894", 3.0, 0.4),
        ];
        for (fam, d, l) in cases {
            for v in family_views(fam) {
                let p = generate(fam, d, l, v).unwrap().unwrap();
                let f = dir.join(format!("gen-{fam}-{}-{v}.svg", trim(d)));
                dump_svg(&p, f.to_str().unwrap()).unwrap();
                println!("写出 {}", f.display());
            }
        }
    }

    /// 人工看色：模板（红）与生成件（蓝）叠合 SVG。
    #[test]
    #[ignore]
    fn overlay_b1() {
        let dir = std::path::Path::new("/tmp/b1");
        std::fs::create_dir_all(dir).unwrap();
        for fam in ["nut_6170", "ring_893", "ring_894"] {
            for v in family_views(fam) {
                let svg = overlay_svg(fam, v);
                let f = dir.join(format!("overlay-{fam}-{v}.svg"));
                std::fs::write(&f, &svg).unwrap();
                println!("写出 {}", f.display());
            }
        }
        // 顺带确认 part_svg 可用（编译期护栏）
        let p = generate("ring_893", 50.0, 2.0, "main").unwrap().unwrap();
        assert!(part_svg(&p, 400.0, 400.0).contains("<svg"));
    }
}
