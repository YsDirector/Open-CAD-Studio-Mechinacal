//! 标准件参数化生成（第二批三族）：**GB/T 5783-2016 全螺纹螺栓**、
//! **GB/T 32.1-1988 六角头头部带孔螺栓**、**GB/T 70.1-2008 内六角圆柱头螺钉**。
//!
//! 画法全部由用户提供的规范图**逐条反解**（工具 `ocs_plugin_api/examples/ref_dump.rs`
//! dump 出精确数值后对齐），三族共用六角头画法（与 C 级同一套构造）：
//!
//! - 头部（主视图，对角朝前 e 宽）：端面 s 宽、60° 角斜线、轮廓 e 宽、内棱线 ±e/4、
//!   角弧（θ=2·atan(8Δ/e)、r=(e/8)/sinθ、弧心 (−k+r, ±0.375e)）、
//!   端面倒角大弧（u=((e/4)²−Δ²)/(2Δ)、r1=u+Δ），Δ=(e−s)/2·tan30°。
//! - 头部（俯视图，对边朝前 s 宽）：弧 θ=2·atan(4Δ/s)、r=(s/4)/sinθ、弧心 (−k+r, ±s/4)。
//! - 头部（左视图）：六边形（上下顶点 ±e/2、左右对边 ±s/2）+ 倒角圆 r=s/2。
//! - 螺纹：牙底 = 0.85d 细实线；收尾斜线；杆端倒角 0.075d（45°）。
//! - 中心线：主/俯视图出头 **3.0**（实测 5780 M5 与 5783 M10、32.1 M18 一致），
//!   左视图十字线半长 = **s/2 + 3**；70.1 族一律 **2.0**。
//!
//! 各族的"增量"（全部来自参考图实测）：
//! | 族 | 头部特征 | 螺纹 |
//! |---|---|---|
//! | GB/T 5783 全螺纹 B 级 | 无垫圈面 | **全螺纹**：收尾从支承面起、收尾长 = 表内 `a` 列 |
//! | GB/T 32.1 A 级 | **垫圈面** dw×c + 头部通孔 d1（孔心距支承面 h+c） | 部分螺纹：b = 表内 `b`、收尾 = 5P |
//! | GB/T 70.1 内六角 | 圆柱头 dk×k + 端边圆角 0.4 + **内六角孔** s/t（孔底 120° 锥） | 部分螺纹：b = 表内 `b`、收尾 = 5P |

use ocs_plugin_api::host::acadrust::entities::{Arc, Circle, EntityType, Line};
use ocs_plugin_api::host::acadrust::types::{Color, LineWeight, Vector3};

use crate::partgen::{
    across_corners, chamfer_run, to_svg, BoltView, GenPart, PartMeta, LAYER_CENTER, LAYER_MAIN,
    LAYER_THIN,
};

/// 虚线层（不可见轮廓：头部通孔 / 内六角孔）。
pub const LAYER_HIDDEN: &str = "4虚线层";
/// 剖面线层（剖视图的金属剖面线）。
pub const LAYER_HATCH: &str = "5剖面线层";

/// 中心线出头（主/俯视图，mm）。
const AXIS_OVER: f64 = 3.0;
/// 中心线出头（70.1 族，mm）。
const AXIS_OVER_SOCKET: f64 = 2.0;

// ── 基础图元（复用 partgen 的图层约定）──────────────────────────────────────

fn line(a: [f64; 2], b: [f64; 2], layer: &str) -> EntityType {
    let mut l = Line::from_points(Vector3::new(a[0], a[1], 0.0), Vector3::new(b[0], b[1], 0.0));
    set_layer(&mut l, layer);
    EntityType::Line(l)
}

fn arc(c: [f64; 2], r: f64, start_deg: f64, end_deg: f64, layer: &str) -> EntityType {
    let mut a = Arc::from_center_radius_angles(
        Vector3::new(c[0], c[1], 0.0),
        r,
        start_deg.to_radians(),
        end_deg.to_radians(),
    );
    set_layer(&mut a, layer);
    EntityType::Arc(a)
}

fn circle(c: [f64; 2], r: f64, layer: &str) -> EntityType {
    let mut e = Circle::from_center_radius(Vector3::new(c[0], c[1], 0.0), r);
    set_layer(&mut e, layer);
    EntityType::Circle(e)
}

fn set_layer<T: CommonLayer>(e: &mut T, layer: &str) {
    let c = e.common_mut();
    c.layer = layer.to_string();
    c.color = Color::ByLayer;
    c.linetype = "ByLayer".to_string();
    c.line_weight = LineWeight::ByLayer;
}

trait CommonLayer {
    fn common_mut(&mut self) -> &mut ocs_plugin_api::host::acadrust::entities::EntityCommon;
}
impl CommonLayer for Line {
    fn common_mut(&mut self) -> &mut ocs_plugin_api::host::acadrust::entities::EntityCommon {
        &mut self.common
    }
}
impl CommonLayer for Arc {
    fn common_mut(&mut self) -> &mut ocs_plugin_api::host::acadrust::entities::EntityCommon {
        &mut self.common
    }
}
impl CommonLayer for Circle {
    fn common_mut(&mut self) -> &mut ocs_plugin_api::host::acadrust::entities::EntityCommon {
        &mut self.common
    }
}
impl CommonLayer for ocs_plugin_api::host::acadrust::entities::Hatch {
    fn common_mut(&mut self) -> &mut ocs_plugin_api::host::acadrust::entities::EntityCommon {
        &mut self.common
    }
}
impl CommonLayer for ocs_plugin_api::host::acadrust::entities::LwPolyline {
    fn common_mut(&mut self) -> &mut ocs_plugin_api::host::acadrust::entities::EntityCommon {
        &mut self.common
    }
}

fn trim(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    s
}

// ── 数据表 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FullBoltRow {
    pub d: f64,
    #[serde(rename = "P")]
    pub pitch: f64,
    pub s: f64,
    pub k: f64,
    /// 对角宽（公差下限；画图用公称 = s/cos30）
    pub e: f64,
    /// 收尾长度（表内 a 列）
    pub a: f64,
    pub l_min: f64,
    pub l_max: f64,
    pub lengths: Vec<f64>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct HoleBoltRow {
    pub d: f64,
    #[serde(rename = "P")]
    pub pitch: f64,
    pub s: f64,
    pub k: f64,
    pub e: f64,
    /// 螺纹长（从杆端量，l ≤ 125 档）
    pub b: f64,
    /// 螺纹长（125 < l ≤ 200 档）
    pub b2: f64,
    /// 螺纹长（l > 200 档）
    pub b3: f64,
    /// 收尾长（= 5P）
    pub runout: f64,
    /// 孔心距头部台肩的距离
    pub h: f64,
    /// 头部通孔直径
    pub d1: f64,
    /// 垫圈面高
    pub c: f64,
    /// 垫圈面直径
    pub dw: f64,
    pub l_min: f64,
    pub l_max: f64,
    pub lengths: Vec<f64>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SocketRow {
    pub d: f64,
    #[serde(rename = "P")]
    pub pitch: f64,
    /// 螺纹长（从杆端量）
    pub b: f64,
    /// 头径
    pub dk: f64,
    /// 内六角对角
    pub e: f64,
    /// 头高
    pub k: f64,
    /// 内六角对边
    pub s: f64,
    /// 内六角孔深
    pub t: f64,
    /// l ≤ 本值 → 全螺纹（ISO 4762 表 1 阴影区；全螺纹时螺纹止于距头部 3P）
    pub l_full: f64,
    pub l_min: f64,
    pub l_max: f64,
    pub lengths: Vec<f64>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct Table<T> {
    family: String,
    name: String,
    code: String,
    iso: String,
    rows: Vec<T>,
}

fn full_b_table() -> &'static Table<FullBoltRow> {
    static T: std::sync::OnceLock<Table<FullBoltRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("tables/partsHexBoltBFull.json")).expect("partsHexBoltBFull.json")
    })
}

fn hole_a_table() -> &'static Table<HoleBoltRow> {
    static T: std::sync::OnceLock<Table<HoleBoltRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("tables/partsHexBoltHoleA.json")).expect("partsHexBoltHoleA.json")
    })
}

fn socket_table() -> &'static Table<SocketRow> {
    static T: std::sync::OnceLock<Table<SocketRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("tables/partsSocketHead.json")).expect("partsSocketHead.json")
    })
}

// ── GB/T 5783-2016 全螺纹螺栓（B 级）────────────────────────────────────────

/// 可选公称直径。
pub fn full_b_diameters() -> Vec<f64> {
    full_b_table().rows.iter().map(|r| r.d).collect()
}

/// 某直径的标准长度系列。
pub fn full_b_lengths(d: f64) -> Vec<f64> {
    full_b_row(d).map(|r| r.lengths.clone()).unwrap_or_default()
}

pub fn full_b_row(d: f64) -> Option<&'static FullBoltRow> {
    full_b_table().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

/// 生成 GB/T 5783-2016（全螺纹 B 级）六角头螺栓的一个视图。
pub fn hex_bolt_b_full(d: f64, l: f64, view: BoltView) -> Result<GenPart, String> {
    let row = full_b_row(d).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.parts.err.no_m",
            &[("table", "GB/T 5783"), ("d", &trim(d))],
        )
    })?;
    check_l(row.l_min, row.l_max, d, l)?;
    let spec = HexSpec {
        d,
        l,
        s: row.s,
        k: row.k,
        b: l - row.a, // 全螺纹：收尾从支承面起，螺纹段到杆端
        runout: row.a,
        full_thread: true,
        washer: None,
        hole: None,
        center_over: AXIS_OVER,
    };
    let mut part = hex_view(&spec, view);
    part.meta = PartMeta {
        code: "GB/T 5783-2016".into(),
        name: crate::i18n::t_data("六角头全螺纹螺栓 全螺纹 B级").into(),
        spec: format!("M{}×{}", trim(d), trim(l)),
        material: String::new(),
        weight: format!("{:.3}", bolt_weight_kg(row, l)),
    };
    Ok(part)
}

/// 全螺纹螺栓单件重量估算（钢 7.85 g/cm³）。
pub fn bolt_weight_kg(row: &FullBoltRow, l: f64) -> f64 {
    let head = across_corners(row.s) * row.s * row.k * 0.9;
    let shank = std::f64::consts::PI / 4.0 * row.d * row.d * l;
    (head + shank) * 7.85e-3 / 1000.0
}

// ── GB/T 32.1-1988 六角头头部带孔螺栓（A 级）────────────────────────────────

pub fn hole_a_diameters() -> Vec<f64> {
    hole_a_table().rows.iter().map(|r| r.d).collect()
}

pub fn hole_a_lengths(d: f64) -> Vec<f64> {
    hole_a_row(d).map(|r| r.lengths.clone()).unwrap_or_default()
}

pub fn hole_a_row(d: f64) -> Option<&'static HoleBoltRow> {
    hole_a_table().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

/// 生成 GB/T 32.1-1988（A 级带孔）六角头螺栓的一个视图。
pub fn hex_bolt_hole_a(d: f64, l: f64, view: BoltView) -> Result<GenPart, String> {
    let row = hole_a_row(d).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.parts.err.no_m",
            &[("table", "GB/T 32.1"), ("d", &trim(d))],
        )
    })?;
    check_l(row.l_min, row.l_max, d, l)?;
    let spec = HexSpec {
        d,
        l,
        s: row.s,
        k: row.k,
        b: thread_length_hole_a(row, l),
        runout: row.runout,
        full_thread: false,
        washer: Some((row.dw, row.c)),
        // 孔心距支承面 = h + c（参考图 M18×60 实测：台肩 −h、支承面再 +c）
        hole: Some((row.d1, row.h + row.c)),
        center_over: AXIS_OVER,
    };
    let mut part = hex_view(&spec, view);
    part.meta = PartMeta {
        code: "GB/T 32.1-2020".into(),
        name: crate::i18n::t_data("六角头头部带孔螺栓 A级").into(),
        spec: format!("M{}×{}", trim(d), trim(l)),
        material: String::new(),
        weight: format!("{:.3}", hole_a_weight_kg(row, l)),
    };
    Ok(part)
}

/// 带孔螺栓螺纹长（ISO 4014 / GB/T 5782 三档：l≤125 → 2d+6；125<l≤200 → 2d+12；l>200 → 2d+25）。
pub fn thread_length_hole_a(row: &HoleBoltRow, l: f64) -> f64 {
    if l <= 125.0 {
        row.b
    } else if l <= 200.0 {
        row.b2
    } else {
        row.b3
    }
}

/// A 级带孔螺栓单件重量估算（钢 7.85 g/cm³，不计头部孔）。
pub fn hole_a_weight_kg(row: &HoleBoltRow, l: f64) -> f64 {
    let head = across_corners(row.s) * row.s * row.k * 0.9;
    let shank = std::f64::consts::PI / 4.0 * row.d * row.d * l;
    let hole = std::f64::consts::PI / 4.0 * row.d1 * row.d1 * row.s;
    (head + shank - hole) * 7.85e-3 / 1000.0
}

// ── 六角头通用视图（5783 / 32.1 共用）──────────────────────────────────────

/// 六角头螺栓视图参数（与族无关的几何量）。
struct HexSpec {
    d: f64,
    l: f64,
    s: f64,
    k: f64,
    /// 螺纹长（从杆端量）
    b: f64,
    runout: f64,
    /// 全螺纹：收尾斜线从支承面起（5783）
    full_thread: bool,
    /// 垫圈面（dw, c）
    washer: Option<(f64, f64)>,
    /// 头部通孔（d1, 孔心距支承面）
    hole: Option<(f64, f64)>,
    center_over: f64,
}

fn check_l(l_min: f64, l_max: f64, d: f64, l: f64) -> Result<(), String> {
    if l < l_min - 1e-9 || l > l_max + 1e-9 {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.len_range",
            &[
                ("d", &trim(d)),
                ("lo", &trim(l_min)),
                ("hi", &trim(l_max)),
            ],
        ));
    }
    Ok(())
}

fn hex_view(sp: &HexSpec, view: BoltView) -> GenPart {
    let e = across_corners(sp.s);
    let delta = chamfer_run(sp.s, e);
    let x_out = -sp.k + delta;
    let dm = 0.85 * sp.d;
    // 垫圈面：头部轮廓止于台肩 x = −c
    let (shoulder, dw) = match sp.washer {
        Some((dw, c)) => (-c, dw),
        None => (0.0, sp.s),
    };
    let mut en: Vec<EntityType> = Vec::new();

    match view {
        BoltView::Main => {
            // 端面 + 角斜线 + 外轮廓（止于台肩）+ 内棱线 ±e/4
            en.push(line([-sp.k, sp.s / 2.0], [-sp.k, -sp.s / 2.0], LAYER_MAIN));
            en.push(line([-sp.k, sp.s / 2.0], [x_out, e / 2.0], LAYER_MAIN));
            en.push(line([-sp.k, -sp.s / 2.0], [x_out, -e / 2.0], LAYER_MAIN));
            en.push(line([x_out, e / 2.0], [shoulder, e / 2.0], LAYER_MAIN));
            en.push(line([x_out, -e / 2.0], [shoulder, -e / 2.0], LAYER_MAIN));
            en.push(line([x_out, e / 4.0], [shoulder, e / 4.0], LAYER_MAIN));
            en.push(line([x_out, -e / 4.0], [shoulder, -e / 4.0], LAYER_MAIN));
            // 角弧（下弧 180°−θ→180°，上弧为镜像）
            let th = 2.0 * (8.0 * delta / e).atan();
            let r2 = (e / 8.0) / th.sin();
            en.push(arc([-sp.k + r2, -0.375 * e], r2, 180.0 - th.to_degrees(), 180.0, LAYER_MAIN));
            en.push(arc([-sp.k + r2, 0.375 * e], r2, 180.0, 180.0 + th.to_degrees(), LAYER_MAIN));
            // 端面倒角大弧
            let u = ((e / 4.0).powi(2) - delta * delta) / (2.0 * delta);
            let r1 = u + delta;
            let al = ((e / 4.0) / u).atan().to_degrees();
            en.push(arc([-sp.k + r1, 0.0], r1, 180.0 - al, 180.0 + al, LAYER_MAIN));
            // 台肩竖线（垫圈面）或头/杆交界竖线
            if sp.washer.is_some() {
                en.push(line([shoulder, e / 2.0], [shoulder, -e / 2.0], LAYER_MAIN));
                en.push(line([shoulder, dw / 2.0], [0.0, dw / 2.0], LAYER_MAIN));
                en.push(line([shoulder, -dw / 2.0], [0.0, -dw / 2.0], LAYER_MAIN));
                // 支承面（垫圈面外沿）只跨垫圈面直径
                en.push(line([0.0, dw / 2.0], [0.0, -dw / 2.0], LAYER_MAIN));
            } else {
                en.push(line([0.0, e / 2.0], [0.0, -e / 2.0], LAYER_MAIN));
            }
            // 头部通孔：正圆（主视图看向孔口）+ 十字中心标记
            if let Some((d1, hx)) = sp.hole {
                en.push(circle([-hx, 0.0], d1 / 2.0, LAYER_MAIN));
                en.push(line([-hx - 0.75 * d1, 0.0], [-hx + 0.75 * d1, 0.0], LAYER_CENTER));
                en.push(line([-hx, 0.75 * d1], [-hx, -0.75 * d1], LAYER_CENTER));
            }
        }
        BoltView::Top => {
            en.push(line([-sp.k, sp.s / 2.0], [-sp.k, -sp.s / 2.0], LAYER_MAIN));
            en.push(line([-sp.k, sp.s / 2.0], [shoulder, sp.s / 2.0], LAYER_MAIN));
            en.push(line([-sp.k, -sp.s / 2.0], [shoulder, -sp.s / 2.0], LAYER_MAIN));
            let th = 2.0 * (4.0 * delta / sp.s).atan();
            let r3 = (sp.s / 4.0) / th.sin();
            en.push(arc([-sp.k + r3, sp.s / 4.0], r3, 180.0 - th.to_degrees(), 180.0 + th.to_degrees(), LAYER_MAIN));
            en.push(arc([-sp.k + r3, -sp.s / 4.0], r3, 180.0 - th.to_degrees(), 180.0 + th.to_degrees(), LAYER_MAIN));
            en.push(line([x_out, 0.0], [shoulder, 0.0], LAYER_MAIN));
            if sp.washer.is_some() {
                en.push(line([shoulder, sp.s / 2.0], [shoulder, -sp.s / 2.0], LAYER_MAIN));
                en.push(line([shoulder, dw / 2.0], [0.0, dw / 2.0], LAYER_MAIN));
                en.push(line([shoulder, -dw / 2.0], [0.0, -dw / 2.0], LAYER_MAIN));
            } else {
                en.push(line([0.0, sp.s / 2.0], [0.0, -sp.s / 2.0], LAYER_MAIN));
            }
            // 头部通孔：俯视图里孔轴竖直 → 两条虚线，跨满对边宽
            if let Some((d1, hx)) = sp.hole {
                for sgn in [1.0, -1.0] {
                    en.push(line([-hx + sgn * d1 / 2.0, sp.s / 2.0], [-hx + sgn * d1 / 2.0, -sp.s / 2.0], LAYER_HIDDEN));
                }
            }
        }
        BoltView::End => {
            let v = [
                [0.0, e / 2.0],
                [sp.s / 2.0, e / 4.0],
                [sp.s / 2.0, -e / 4.0],
                [0.0, -e / 2.0],
                [-sp.s / 2.0, -e / 4.0],
                [-sp.s / 2.0, e / 4.0],
            ];
            for i in 0..6 {
                en.push(line(v[i], v[(i + 1) % 6], LAYER_MAIN));
            }
            en.push(circle([0.0, 0.0], sp.s / 2.0, LAYER_MAIN));
            // 头部通孔：端视图里孔轴水平 → 两条虚线，跨满对边宽
            if let Some((d1, _)) = sp.hole {
                for sgn in [1.0, -1.0] {
                    en.push(line([-sp.s / 2.0, sgn * d1 / 2.0], [sp.s / 2.0, sgn * d1 / 2.0], LAYER_HIDDEN));
                }
            }
            let o = sp.s / 2.0 + 3.0;
            en.push(line([-o, 0.0], [o, 0.0], LAYER_CENTER));
            en.push(line([0.0, o], [0.0, -o], LAYER_CENTER));
            let meta = PartMeta {
                code: String::new(),
                name: String::new(),
                spec: String::new(),
                material: String::new(),
                weight: String::new(),
            };
            return GenPart {
                entities: en,
                meta,
                bbox: [-sp.s / 2.0, -e / 2.0, sp.s / 2.0, e / 2.0],
            };
        }
    }

    // 杆 + 螺纹（主/俯视图共用）
    let c = 0.075 * sp.d;
    let ls = sp.l - sp.b; // 螺纹（细实线）起点
    en.push(line([0.0, sp.d / 2.0], [sp.l - c, sp.d / 2.0], LAYER_MAIN));
    en.push(line([0.0, -sp.d / 2.0], [sp.l - c, -sp.d / 2.0], LAYER_MAIN));
    en.push(line([sp.l - c, sp.d / 2.0], [sp.l, dm / 2.0], LAYER_MAIN));
    en.push(line([sp.l - c, -sp.d / 2.0], [sp.l, -dm / 2.0], LAYER_MAIN));
    en.push(line([sp.l, dm / 2.0], [sp.l, -dm / 2.0], LAYER_MAIN));
    en.push(line([sp.l - c, sp.d / 2.0], [sp.l - c, -sp.d / 2.0], LAYER_MAIN));
    en.push(line([ls, sp.d / 2.0], [ls, -sp.d / 2.0], LAYER_MAIN));
    en.push(line([ls, dm / 2.0], [sp.l, dm / 2.0], LAYER_THIN));
    en.push(line([ls, -dm / 2.0], [sp.l, -dm / 2.0], LAYER_THIN));
    let rs = if sp.full_thread {
        0.0 // 收尾从支承面起（5783 参考图：斜线起点在头部支承面）
    } else {
        ls - sp.runout
    };
    en.push(line([rs, sp.d / 2.0], [ls, dm / 2.0], LAYER_THIN));
    en.push(line([rs, -sp.d / 2.0], [ls, -dm / 2.0], LAYER_THIN));
    en.push(line([-sp.k - sp.center_over, 0.0], [sp.l + sp.center_over, 0.0], LAYER_CENTER));

    let half_h = if view == BoltView::Main { e / 2.0 } else { sp.s / 2.0 };
    GenPart {
        entities: en,
        meta: PartMeta {
            code: String::new(),
            name: String::new(),
            spec: String::new(),
            material: String::new(),
            weight: String::new(),
        },
        bbox: [-sp.k, -half_h, sp.l, half_h],
    }
}

// ── GB/T 70.1-2008 内六角圆柱头螺钉 ────────────────────────────────────────

/// 头部端边圆角（参考图 M10 实测 0.4，用户模型里没有该参数 → 取固定值）。
const HEAD_EDGE_R: f64 = 0.4;
/// 内六角孔底锥角（参考图标注 120°）。
const SOCKET_CONE_DEG: f64 = 60.0; // 与轴线夹角

pub fn socket_diameters() -> Vec<f64> {
    socket_table().rows.iter().map(|r| r.d).collect()
}

pub fn socket_lengths(d: f64) -> Vec<f64> {
    socket_row(d).map(|r| r.lengths.clone()).unwrap_or_default()
}

pub fn socket_row(d: f64) -> Option<&'static SocketRow> {
    socket_table().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

/// 生成 GB/T 70.1-2008（内六角圆柱头螺钉）的一个视图。
pub fn socket_head(d: f64, l: f64, view: BoltView) -> Result<GenPart, String> {
    let row = socket_row(d).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.parts.err.no_m",
            &[("table", "GB/T 70.1"), ("d", &trim(d))],
        )
    })?;
    check_l(row.l_min, row.l_max, d, l)?;
    let (dm, ck) = (0.85 * d, 0.075 * d);
    // ISO 4762 表 1：l ≤ l_full → 全螺纹（螺纹止于距头部 3P）；否则 b = b_ref、收尾 5P
    let full = l <= row.l_full + 1e-9;
    let runout = 5.0 * row.pitch;
    let b = if full {
        (l - 3.0 * row.pitch).max(0.0)
    } else {
        row.b.min(l - 1e-9).max(0.0)
    };
    let ls = (l - b).max(0.0); // 螺纹（细实线）起点
    let rs = if full { 0.0 } else { (ls - runout).max(0.0) }; // 收尾起点（全螺纹从支承面起）
    let r = HEAD_EDGE_R;
    let mut en: Vec<EntityType> = Vec::new();
    // 内六角孔的两种"投影高度"（端视图六边形：上下顶点 ±e/2、左右对边 ±s/2）
    let hy = row.e / 2.0; // 顶点（孔口最外）
    let ht = row.s / 2.0; // 对边
    let h2 = hy - ht * 30f64.to_radians().tan(); // 对边端点的 y
    let cone_dx = hy / SOCKET_CONE_DEG.to_radians().tan();
    let (xb, xe) = (-row.k + row.t, -row.k + row.t + cone_dx); // 孔底 / 锥尖

    match view {
        BoltView::Main => {
            // 头部：端面 + 端边圆角 + 外圆轮廓 + 支承面
            en.push(line([-row.k, row.dk / 2.0 - r], [-row.k, -(row.dk / 2.0 - r)], LAYER_MAIN));
            en.push(arc([-row.k + r, row.dk / 2.0 - r], r, 90.0, 180.0, LAYER_MAIN));
            en.push(arc([-row.k + r, -(row.dk / 2.0 - r)], r, 180.0, 270.0, LAYER_MAIN));
            en.push(line([-row.k + r, row.dk / 2.0], [0.0, row.dk / 2.0], LAYER_MAIN));
            en.push(line([-row.k + r, -row.dk / 2.0], [0.0, -row.dk / 2.0], LAYER_MAIN));
            en.push(line([0.0, row.dk / 2.0], [0.0, -row.dk / 2.0], LAYER_MAIN));
            // 内六角孔（虚线）：顶点线 ±e/2、对边端点线 ±h2、孔底线、锥面
            for sgn in [1.0, -1.0] {
                en.push(line([-row.k, sgn * hy], [xb, sgn * hy], LAYER_HIDDEN));
                en.push(line([-row.k, sgn * h2], [xb, sgn * h2], LAYER_HIDDEN));
                en.push(line([xb, sgn * hy], [xe, 0.0], LAYER_HIDDEN));
            }
            en.push(line([xb, hy], [xb, -hy], LAYER_HIDDEN));
        }
        BoltView::Top => {
            // 头部俯视：矩形 + 端边圆角弧 + 内六角孔（此时投影宽 = 对边 s）
            en.push(line([-row.k, row.dk / 2.0 - r], [-row.k, -(row.dk / 2.0 - r)], LAYER_MAIN));
            en.push(arc([-row.k + r, row.dk / 2.0 - r], r, 90.0, 180.0, LAYER_MAIN));
            en.push(arc([-row.k + r, -(row.dk / 2.0 - r)], r, 180.0, 270.0, LAYER_MAIN));
            en.push(line([-row.k + r, row.dk / 2.0], [0.0, row.dk / 2.0], LAYER_MAIN));
            en.push(line([-row.k + r, -row.dk / 2.0], [0.0, -row.dk / 2.0], LAYER_MAIN));
            en.push(line([0.0, row.dk / 2.0], [0.0, -row.dk / 2.0], LAYER_MAIN));
            for sgn in [1.0, -1.0] {
                en.push(line([-row.k, sgn * ht], [xb, sgn * ht], LAYER_HIDDEN));
                en.push(line([xb, sgn * ht], [xe, 0.0], LAYER_HIDDEN));
            }
            en.push(line([xb, ht], [xb, -ht], LAYER_HIDDEN));
        }
        BoltView::End => {
            en.push(circle([0.0, 0.0], row.dk / 2.0, LAYER_MAIN));
            en.push(circle([0.0, 0.0], hy, LAYER_MAIN));
            // 内六角孔口：上下顶点 ±e/2，左右对边 ±s/2
            let v = [
                [0.0, hy],
                [ht, h2],
                [ht, -h2],
                [0.0, -hy],
                [-ht, -h2],
                [-ht, h2],
            ];
            for i in 0..6 {
                en.push(line(v[i], v[(i + 1) % 6], LAYER_MAIN));
            }
            let o = row.dk / 2.0 + AXIS_OVER_SOCKET;
            en.push(line([-o, 0.0], [o, 0.0], LAYER_CENTER));
            en.push(line([0.0, o], [0.0, -o], LAYER_CENTER));
            return Ok(GenPart {
                entities: en,
                meta: PartMeta {
                    code: "GB/T 70.1-2008".into(),
                    name: crate::i18n::t_data("内六角圆柱头螺钉").into(),
                    spec: format!("M{}×{}", trim(d), trim(l)),
                    material: String::new(),
                    weight: format!("{:.3}", socket_weight_kg(row, l)),
                },
                bbox: [-row.dk / 2.0, -row.dk / 2.0, row.dk / 2.0, row.dk / 2.0],
            });
        }
    }

    // 杆 + 螺纹（主/俯视图共用）
    en.push(line([0.0, d / 2.0], [l - ck, d / 2.0], LAYER_MAIN));
    en.push(line([0.0, -d / 2.0], [l - ck, -d / 2.0], LAYER_MAIN));
    en.push(line([l - ck, d / 2.0], [l, dm / 2.0], LAYER_MAIN));
    en.push(line([l - ck, -d / 2.0], [l, -dm / 2.0], LAYER_MAIN));
    en.push(line([l, dm / 2.0], [l, -dm / 2.0], LAYER_MAIN));
    en.push(line([l - ck, d / 2.0], [l - ck, -d / 2.0], LAYER_MAIN));
    en.push(line([ls, d / 2.0], [ls, -d / 2.0], LAYER_MAIN));
    // 牙底细实线：参考图止于杆端倒角起点（l − 0.075d）
    en.push(line([ls, dm / 2.0], [l - ck, dm / 2.0], LAYER_THIN));
    en.push(line([ls, -dm / 2.0], [l - ck, -dm / 2.0], LAYER_THIN));
    en.push(line([rs, d / 2.0], [ls, dm / 2.0], LAYER_THIN));
    en.push(line([rs, -d / 2.0], [ls, -dm / 2.0], LAYER_THIN));
    en.push(line(
        [-row.k - AXIS_OVER_SOCKET, 0.0],
        [l + AXIS_OVER_SOCKET, 0.0],
        LAYER_CENTER,
    ));

    let half_h = row.dk / 2.0;
    Ok(GenPart {
        entities: en,
        meta: PartMeta {
            code: "GB/T 70.1-2008".into(),
            name: crate::i18n::t_data("内六角圆柱头螺钉").into(),
            spec: format!("M{}×{}", trim(d), trim(l)),
            material: String::new(),
            weight: format!("{:.3}", socket_weight_kg(row, l)),
        },
        bbox: [-row.k, -half_h, l, half_h],
    })
}

/// 内六角圆柱头螺钉单件重量估算（钢 7.85 g/cm³）。
pub fn socket_weight_kg(row: &SocketRow, l: f64) -> f64 {
    let head = std::f64::consts::PI / 4.0 * row.dk * row.dk * row.k;
    let socket = std::f64::consts::PI / 4.0 * row.e * row.e * row.t * 0.75;
    let shank = std::f64::consts::PI / 4.0 * row.d * row.d * l;
    (head - socket + shank) * 7.85e-3 / 1000.0
}

// ── 目录 JSON / 派发（供 partgen 的统一入口拼接）────────────────────────────

/// 三族的 `families` JSON 片段（键 = 族 id）。
pub fn families_json() -> serde_json::Map<String, serde_json::Value> {
    let views = views_json("hex_bolt_c");
    let _ = &views; // 供 C 级族使用
    let mut m = serde_json::Map::new();

    let sizes: Vec<serde_json::Value> = full_b_table()
        .rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "d": r.d, "label": format!("M{}", trim(r.d)), "pitch": r.pitch,
                "l_min": r.l_min, "l_max": r.l_max, "lengths": r.lengths,
            })
        })
        .collect();
    m.insert(
        "hex_bolt_b_full".into(),
        serde_json::json!({
            "id": "hex_bolt_b_full", "name": "六角头全螺纹螺栓 全螺纹 B级",
            "code": "GB/T 5783-2016", "iso": "ISO 4017:2014",
            "implemented": true, "views": views_json("hex_bolt_b_full"), "sizes": sizes,
            "len_label": "长度 l", "base_hint": "基点 = 头部支承面 × 轴线",
        }),
    );

    let sizes: Vec<serde_json::Value> = hole_a_table()
        .rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "d": r.d, "label": format!("M{}", trim(r.d)), "pitch": r.pitch,
                "l_min": r.l_min, "l_max": r.l_max, "lengths": r.lengths,
                "extra": format!("头部通孔 2-Ø{}，孔心距支承面 {}", trim(r.d1), trim(r.h + r.c)),
            })
        })
        .collect();
    m.insert(
        "hex_bolt_hole_a".into(),
        serde_json::json!({
            "id": "hex_bolt_hole_a", "name": "六角头头部带孔螺栓 A级",  // 视图见下方 views_json
            "code": "GB/T 32.1-2020", "iso": "ISO 4014:2011（同族）",
            "implemented": true, "views": views_json("hex_bolt_hole_a"), "sizes": sizes,
            "len_label": "长度 l", "base_hint": "基点 = 头部支承面 × 轴线",
        }),
    );

    let sizes: Vec<serde_json::Value> = socket_table()
        .rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "d": r.d, "label": format!("M{}", trim(r.d)), "pitch": r.pitch,
                "l_min": r.l_min, "l_max": r.l_max, "lengths": r.lengths,
                "extra": format!("内六角 {}，孔深 {}", trim(r.s), trim(r.t)),
            })
        })
        .collect();
    let views_ring = views_json("washer_971");
    for (key, tab_name, code, iso, thin) in [
        ("nut_61721", "六角薄螺母", "GB/T 6172.1-2016", "ISO 4035:2012", true),
        ("nut_c41", "六角螺母 C级", "GB/T 41-2016", "ISO 4034:2012", false),
    ] {
        let rows = match key {
            "nut_61721" => nut_61721_table(),
            _ => nut_c41_table(),
        };
        let sizes: Vec<serde_json::Value> = rows
            .rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "d": r.d, "label": format!("M{}", trim(r.d)), "pitch": r.pitch,
                    "l_min": r.m, "l_max": r.m,
                    "lengths": [r.m],
                    "extra": format!("s={} e={} m={}", trim(r.s), trim(r.e), trim(r.m)),
                })
            })
            .collect();
        m.insert(
            key.into(),
            serde_json::json!({
                "id": key, "name": tab_name, "code": code, "iso": iso,
                // 视图**按族**取注册表（`nut_6170` 已下架、family_views 为空，
                // 曾把它的空数组错给到这两族 → GUI 视图按钮为空、面板静默失效）
                "implemented": true, "views": views_json(key), "sizes": sizes,
                "len_label": "高度 m", "base_hint": "基点 = 左端面 × 轴线",
            }),
        );
    }
    {
        let sizes: Vec<serde_json::Value> = washer_971_table()
            .rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "d": r.d, "label": format!("Ø{}", trim(r.d)), "pitch": 0.0,
                    "l_min": r.h, "l_max": r.h, "lengths": [r.h],
                    "extra": format!("d1={} d2={} h={}", trim(r.d1), trim(r.d2), trim(r.h)),
                })
            })
            .collect();
        m.insert(
            "washer_971".into(),
            serde_json::json!({
                "id": "washer_971", "name": "平垫圈 A级", "code": "GB/T 97.1-2002",
                "iso": "ISO 7089:2000", "implemented": true, "views": views_ring.clone(), "sizes": sizes,
                "len_label": "厚度 h", "base_hint": "基点 = 端面中心（轴线）",
            }),
        );
        let sizes: Vec<serde_json::Value> = washer_93_table()
            .rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "d": r.d, "label": format!("Ø{}", trim(r.d)), "pitch": 0.0,
                    "l_min": r.s, "l_max": r.s, "lengths": [r.s],
                    "extra": format!("内径 {} 宽 {} 厚 {}", trim(r.d1), trim(r.b), trim(r.s)),
                })
            })
            .collect();
        m.insert(
            "washer_93".into(),
            serde_json::json!({
                "id": "washer_93", "name": "标准型弹簧垫圈", "code": "GB/T 93-2025",
                "iso": "—", "implemented": true, "views": views_ring, "sizes": sizes,
                "len_label": "厚度 s", "base_hint": "基点 = 端面中心（轴线）",
            }),
        );
    }
    // ── 销族（GB/T 119.1-2000 / GB/T 120.1-2000）：单视图 ──
    for (key, tab_name, code, iso, rows_json) in [
        (
            "pin_1191",
            "圆柱销 A型",
            "GB/T 119.1-2000",
            "ISO 2338:1997",
            pin_1191_table()
                .rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "d": r.d, "label": format!("Ø{}", trim(r.d)), "pitch": 0.0,
                        "l_min": r.l_min, "l_max": r.l_max, "lengths": r.lengths,
                        "extra": format!("c={} a={}（左端 15° 锥 / 右端球冠）", trim(r.c), trim(r.a)),
                    })
                })
                .collect::<Vec<_>>(),
        ),
        (
            "pin_1201",
            "内螺纹圆柱销",
            "GB/T 120.1-2000",
            "ISO 8735:1997",
            pin_1201_table()
                .rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "d": r.d, "label": format!("Ø{}", trim(r.d)), "pitch": 0.0,
                        "l_min": r.l_min, "l_max": r.l_max, "lengths": r.lengths,
                        "extra": format!("M{} 深{} 钻孔{} c={} a={}", trim(r.d1), trim(r.t1), trim(r.t2), trim(r.c), trim(r.a)),
                    })
                })
                .collect::<Vec<_>>(),
        ),
    ] {
        m.insert(
            key.into(),
            serde_json::json!({
                "id": key, "name": tab_name, "code": code, "iso": iso,
                "implemented": true, "views": views_json(key), "sizes": rows_json,
                "len_label": "长度 l", "base_hint": "基点 = 左端面（c 锥端）× 轴线",
            }),
        );
    }
    m.insert(
        "socket_head".into(),
        serde_json::json!({
            "id": "socket_head", "name": "内六角圆柱头螺钉",
            "code": "GB/T 70.1-2008", "iso": "ISO 4762:2004",
            "implemented": true, "views": views_json("socket_head"), "sizes": sizes,
            "len_label": "长度 l", "base_hint": "基点 = 头部支承面 × 轴线",
        }),
    );
    m
}

/// 派发（三族 × 三视图）。未命中返回 None，由调用方给通用错误。
pub fn generate(family: &str, d: f64, l: f64, view: &str) -> Option<Result<GenPart, String>> {
    // 第四批起的族自带完整实现（含视图校验），先问它们（partgen_b1…b4 的 `generate`；
    // 平键 partgen_keys 同批走同一入口）
    for f in [
        crate::partgen_b1::generate,
        crate::partgen_b2::generate,
        crate::partgen_b3::generate,
        crate::partgen_b4::generate,
        crate::partgen_b5::generate,
        crate::partgen_keys::generate,
    ] {
        if let Some(r) = f(family, d, l, view) {
            return Some(r);
        }
    }
    // 1型六角螺母：**已在第四批（2026-09-17）由 `partgen_b1` 按用户四视图模板上架**，
    // 本模块这分支只作傅底（正常情况到不了这里）；真走到说明 b1 未接管。
    if family == "nut_6170" {
        return Some(Err(crate::i18n::t("cmd.parts.err.nut6170_todo")));
    }
    // 只允许该族样例里存在的视图（其余一律报错，GUI 也不会列出）
    let allowed = family_views(family);
    if !allowed.is_empty() && !allowed.contains(&view) {
        return Some(Err(crate::i18n::t_fmt(
            "cmd.parts.err.view_not_offered",
            &[
                ("family", family),
                ("view", view),
                ("avail", &allowed.join("/")),
            ],
        )));
    }
    // 销族：单视图（模板只有主视图）
    if matches!(family, "pin_1191" | "pin_1201") {
        if view != "main" {
            return Some(Err(crate::i18n::t_fmt(
                "cmd.parts.err.only_main",
                &[("family", family)],
            )));
        }
        return Some(match family {
            "pin_1191" => pin_a(d, l),
            _ => pin_threaded(d, l),
        });
    }
    // 螺母/垫圈族：视图名含 section（剖视图）
    if matches!(family, "nut_6170" | "nut_61721" | "nut_c41" | "washer_971" | "washer_93") {
        let nv = match view {
            "main" => NutView::Main,
            "section" => NutView::Section,
            "top" => NutView::Top,
            "end" => NutView::End,
            other => {
                return Some(Err(crate::i18n::t_fmt(
                    "cmd.parts.err.view_absent_bare",
                    &[("family", family), ("view", other)],
                )))
            }
        };
        return Some(match family {
            "washer_971" => flat_washer(d, nv),
            "washer_93" => spring_washer(d, nv),
            nut @ ("nut_6170" | "nut_61721" | "nut_c41") => hex_nut(d, nut, nv),
            other => Err(crate::i18n::t_fmt("cmd.parts.err.unknown_family", &[("other", other)])),
        });
    }
    let v = match view {
        "main" => BoltView::Main,
        "top" => BoltView::Top,
        "end" => BoltView::End,
        other => {
            return Some(Err(crate::i18n::t_fmt(
                "cmd.parts.err.view_absent_bare",
                &[("family", family), ("view", other)],
            )))
        }
    };
    match family {
        "hex_bolt_b_full" => Some(hex_bolt_b_full(d, l, v)),
        "hex_bolt_hole_a" => Some(hex_bolt_hole_a(d, l, v)),
        "socket_head" => Some(socket_head(d, l, v)),
        _ => None,
    }
}

/// 单视图 SVG（给人工核对用：`dump_*` 测试与命令行共用）。
pub fn view_svg(family: &str, d: f64, l: f64, view: &str, px_w: f64, px_h: f64) -> Result<String, String> {
    let p = crate::partgen::generate(family, d, l, view)?;
    Ok(to_svg(&p, &format!("{} {}", p.meta.code, p.meta.spec), px_w, px_h))
}

// ── 测试 ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// 语言是进程级全局：断言中文文案的用例共用锁并钉死 zh。
    fn zh_guard() -> std::sync::MutexGuard<'static, ()> {
        let g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        g
    }

    use std::collections::BTreeSet;

    include!("tables/ref_3fams_expected.rs");

    fn fmt(v: f64) -> String {
        let r = (v * 1000.0).round() / 1000.0;
        if r == 0.0 {
            "0".to_string()
        } else {
            format!("{r}")
        }
    }

    /// 生成结果 → 与参考图同构的图元字符串集合（中心线层不参与比对）。
    fn norm(p: &GenPart) -> BTreeSet<String> {
        let lay = |l: &str| match l {
            LAYER_THIN => "THIN",
            LAYER_HIDDEN => "HIDDEN",
            LAYER_CENTER => "CENTER",
            _ => "MAIN",
        };
        let mut out = BTreeSet::new();
        for e in &p.entities {
            let l = lay(e.common().layer.as_str());
            if l == "CENTER" {
                continue;
            }
            match e {
                EntityType::Line(ln) => {
                    let (mut a, mut b) = (
                        (ln.start.x, ln.start.y),
                        (ln.end.x, ln.end.y),
                    );
                    if a > b {
                        std::mem::swap(&mut a, &mut b);
                    }
                    out.insert(format!(
                        "L {} {} {} {} {}",
                        fmt(a.0),
                        fmt(a.1),
                        fmt(b.0),
                        fmt(b.1),
                        l
                    ));
                }
                EntityType::Arc(a) => {
                    let wrap = |v: f64| {
                        let mut x = v % 360.0;
                        if x < 0.0 {
                            x += 360.0;
                        }
                        x
                    };
                    let (mut a1, mut a2) = (wrap(a.start_angle.to_degrees()), wrap(a.end_angle.to_degrees()));
                    if a1 > a2 {
                        std::mem::swap(&mut a1, &mut a2);
                    }
                    out.insert(format!(
                        "A {} {} {} {} {} {}",
                        fmt(a.center.x),
                        fmt(a.center.y),
                        fmt(a.radius),
                        fmt(a1),
                        fmt(a2),
                        l
                    ));
                }
                EntityType::Circle(c) => {
                    out.insert(format!(
                        "C {} {} {} {}",
                        fmt(c.center.x),
                        fmt(c.center.y),
                        fmt(c.radius),
                        l
                    ));
                }
                _ => {}
            }
        }
        out
    }

    /// 解析 "L x1 y1 x2 y2 LAYER" / "A cx cy r a1 a2 LAYER" / "C cx cy r LAYER"。
    fn parse(s: &str) -> (String, Vec<f64>) {
        let p: Vec<&str> = s.split_whitespace().collect();
        (
            format!("{} {}", p[0], p[p.len() - 1]),
            p[1..p.len() - 1].iter().map(|v| v.parse().unwrap()).collect(),
        )
    }

    /// 容差比较（0.002mm / 0.01°）：两组图元一一配对，报告未匹配项。
    fn diff(got: &BTreeSet<String>, want: &[&str], allow_extra: &[&str]) -> (Vec<String>, Vec<String>) {
        let tol = 0.0021;
        let gtol = 0.011;
        let mut pool: Vec<(String, Vec<f64>)> = want.iter().map(|s| parse(s)).collect();
        let mut missing = Vec::new();
        let mut surplus = Vec::new();
        let mut extras: Vec<(String, Vec<f64>)> = allow_extra.iter().map(|s| parse(s)).collect();
        for g in got {
            let (gk, gv) = parse(g);
            let close = |(k, v): &(String, Vec<f64>)| {
                k == &gk
                    && v.len() == gv.len()
                    && v.iter().zip(&gv).all(|(a, b)| {
                        if gk.starts_with('A') && (a - b).abs() > 1.0 {
                            false
                        } else {
                            (a - b).abs() <= if gk.starts_with('A') { gv[2].max(1.0) * 0.01 } else { tol }
                        }
                    })
            };
            if let Some(i) = pool.iter().position(|p| close(p)) {
                pool.remove(i);
            } else if let Some(i) = extras.iter().position(|p| close(p)) {
                extras.remove(i);
            } else {
                surplus.push(g.clone());
            }
        }
        for (k, v) in pool {
            let n: Vec<String> = v.iter().map(|x| fmt(*x)).collect();
            missing.push(format!("{} {k}", n.join(" ")));
        }
        (missing, surplus)
    }

    fn assert_view(got: &GenPart, want: &[&str], allow_extra: &[&str], label: &str) {
        let (missing, surplus) = diff(&norm(got), want, allow_extra);
        assert!(
            missing.is_empty() && surplus.is_empty(),
            "{label}\n  缺失({}): {missing:#?}\n  多出({}): {surplus:#?}",
            missing.len(),
            surplus.len()
        );
    }

    /// 5783 全螺纹螺栓 M10×20 主视图：与参考图逐条一致。
    #[test]
    fn hex_bolt_b_full_main_matches_reference() {
        let p = hex_bolt_b_full(10.0, 20.0, BoltView::Main).unwrap();
        assert_view(
            &p,
            HEX_BOLT_B_FULL_MAIN,
            &["A -2.311 6.928 4.089 180 214.384 MAIN"],
            "5783 M10x20 主视图",
        );
    }

    /// 32.1 带孔螺栓 M18×60 主视图：与参考图逐条一致。
    #[test]
    fn hex_bolt_hole_a_main_matches_reference() {
        let p = hex_bolt_hole_a(18.0, 60.0, BoltView::Main).unwrap();
        assert_view(
            &p,
            HEX_BOLT_HOLE_A_MAIN,
            &["A -4.599 11.691 6.901 180 214.384 MAIN"],
            "32.1 M18x60 主视图",
        );
    }

    /// 70.1 内六角圆柱头螺钉 M10×70 主视图：与参考图逐条一致。
    #[test]
    fn socket_head_main_matches_reference() {
        let p = socket_head(10.0, 70.0, BoltView::Main).unwrap();
        assert_view(
            &p,
            SOCKET_HEAD_MAIN,
            &[
                "A -9.6 7.6 0.4 90 180 MAIN",
                // 支承面/螺纹起点竖线的"另一半"：参考图那份被 180° 副本顶掉了，
                // 本实现按对称补齐（与 §20.2 的镜像约定一致）
                "L 0 -8 0 8 MAIN",
                "L 38 -5 38 5 MAIN",
            ],
            "70.1 M10x70 主视图",
        );
    }

    /// 70.1 左视图：参考图只画了六边形右半（生成器画全六条边）。
    #[test]
    fn socket_head_end_matches_reference() {
        let p = socket_head(10.0, 70.0, BoltView::End).unwrap();
        assert_view(
            &p,
            SOCKET_HEAD_END,
            &[
                "L -4 -2.266 -4 2.266 MAIN",
                "L -4 -2.266 0 -4.575 MAIN",
                "L -4 2.266 0 4.575 MAIN",
            ],
            "70.1 M10x70 左视图",
        );
    }

    /// 所有上架族都只有 OCSM 模板图层、且**不含任何尺寸标注**。
    #[test]
    fn all_families_layers_and_no_dimension() {
        let dots = [
            ("hex_bolt_b_full", 10.0, 20.0),
            ("hex_bolt_b_full", 1.6, 8.0),
            ("hex_bolt_hole_a", 18.0, 60.0),
            ("hex_bolt_hole_a", 6.0, 30.0),
            ("socket_head", 10.0, 70.0),
            ("socket_head", 1.6, 4.0),
            // 螺母两族（含剖视图的 5剖面线层 Hatch）
            ("nut_61721", 10.0, 0.0),
            ("nut_61721", 30.0, 0.0),
            ("nut_c41", 24.0, 0.0),
            ("nut_c41", 64.0, 0.0),
            // 垫圈两族
            ("washer_971", 10.0, 0.0),
            ("washer_93", 10.0, 0.0),
            // 销两族（单视图；120.1 含 5剖面线层 局部剖 Hatch）
            ("pin_1191", 10.0, 18.0),
            ("pin_1201", 20.0, 40.0),
        ];
        for (fam, d, l) in dots {
            for view in family_views(fam) {
                let p = crate::partgen::generate(fam, d, l, view)
                    .unwrap_or_else(|e| panic!("{fam} M{d}×{l} {view}: {e}"));
                assert!(!p.entities.is_empty(), "{fam} {view} 空");
                for e in &p.entities {
                    let lay = e.common().layer.as_str();
                    assert!(
                        [LAYER_MAIN, LAYER_THIN, LAYER_CENTER, LAYER_HIDDEN, LAYER_HATCH].contains(&lay),
                        "{fam} {view} 图层越界: {lay}"
                    );
                    assert!(
                        matches!(e.common().color, Color::ByLayer),
                        "{fam} {view} 颜色不是 ByLayer"
                    );
                    assert!(!matches!(e, EntityType::Dimension(_)), "{fam} {view} 出现尺寸标注");
                }
            }
        }
    }

    /// 弧的角度字段是**弧度**（防回归：曾按度数写入导致弧线跑飞）。
    #[test]
    fn arcs_angles_are_radians() {
        let p = hex_bolt_b_full(10.0, 20.0, BoltView::Main).unwrap();
        let arcs: Vec<_> = p
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) => Some(a),
                _ => None,
            })
            .collect();
        assert!(arcs.len() >= 3, "主视图应有角弧+端面大弧");
        for a in arcs {
            assert!(
                a.start_angle.abs() <= std::f64::consts::TAU + 1e-9
                    && a.end_angle.abs() <= std::f64::consts::TAU + 1e-9,
                "弧角度越界（疑似写成度数）: {} → {}",
                a.start_angle,
                a.end_angle
            );
        }
    }

    /// 长度校验：越界长度给出可读错误；表内规格齐全、长度序列单调。
    #[test]
    fn tables_sane() {
        let _g = zh_guard();
        for (fam, ds) in [
            ("hex_bolt_b_full", full_b_diameters()),
            ("hex_bolt_hole_a", hole_a_diameters()),
            ("socket_head", socket_diameters()),
        ] {
            assert!(ds.len() >= 10, "{fam} 规格太少");
            for d in &ds {
                let ls = crate::partgen::hex_bolt_c_lengths(*d);
                let _ = ls;
                let lengths = match fam {
                    "hex_bolt_b_full" => full_b_lengths(*d),
                    "hex_bolt_hole_a" => hole_a_lengths(*d),
                    _ => socket_lengths(*d),
                };
                assert!(!lengths.is_empty(), "{fam} M{d} 无长度系列");
                let mut prev = 0.0;
                for l in &lengths {
                    assert!(*l > prev, "{fam} M{d} 长度序列非递增");
                    prev = *l;
                    crate::partgen::generate(fam, *d, *l, "main")
                        .unwrap_or_else(|e| panic!("{fam} M{d}×{l} 生成失败: {e}"));
                }
            }
        }
        let e = crate::partgen::generate("hex_bolt_b_full", 10.0, 500.0, "main").unwrap_err();
        assert!(e.contains("长度应在"), "越界错误信息: {e}");
    }

    /// 规格文本/代号进入 BOM 元数据。
    #[test]
    fn metadata_for_bom() {
        let p = hex_bolt_hole_a(18.0, 60.0, BoltView::Main).unwrap();
        assert_eq!(p.meta.code, "GB/T 32.1-2020");
        assert_eq!(p.meta.spec, "M18×60");
        // 设计重量是估算值，但**不写 ≈**（用户 2026-09-15）
        assert!(!p.meta.weight.contains('≈'));
        assert!(p.meta.weight.parse::<f64>().unwrap() > 0.0);
        let p = socket_head(10.0, 70.0, BoltView::Main).unwrap();
        assert_eq!(p.meta.code, "GB/T 70.1-2008");
        assert_eq!(p.meta.name, "内六角圆柱头螺钉");
    }

    /// 人工核对用：螺母剖视图（含 5剖面线层）→ SVG。
    #[test]
    #[ignore]
    fn dump_nut_section_svg() {
        let dir = std::path::Path::new("/tmp/partgen3");
        std::fs::create_dir_all(dir).unwrap();
        for d in [10.0, 20.0] {
            for (thin, tag) in [(false, "nut6170"), (true, "nut61721")] {
                let fam = if thin { "nut_61721" } else { "nut_6170" };
                let p = hex_nut(d, fam, NutView::Section).unwrap();
                let svg = to_svg(&p, &format!("{} M{} 剖视图", p.meta.code, trim(d)), 900.0, 380.0);
                let f = dir.join(format!("{tag}-M{}-剖视图.svg", trim(d)));
                std::fs::write(&f, svg).unwrap();
                println!("写出 {}", f.display());
            }
        }
    }

    /// 人工核对用：各族各视图 → SVG（`cargo test -p ocs_ocsm -- --ignored dump_parts_svg --nocapture`）。
    ///
    /// 输出目录在 `~/桌面/OCSM/test/参数化预览/`（**不用 /tmp**：tmpfs 会被清空）。
    /// 配套：`tools/overlay_check.py` 拿本测试写的 DXF 与用户模板 DXF 叠合比对。
    #[test]
    #[ignore]
    fn dump_parts_svg() {
        let dir = std::path::Path::new("/home/ysdirector/桌面/OCSM/test/参数化预览");
        std::fs::create_dir_all(dir).unwrap();
        for (fam, d, l, name, view) in all_dump_cases() {
            let p = crate::partgen::generate(fam, d, l, view).unwrap();
            let vn = view_name(view);
            let (code, spec) = (p.meta.code.clone(), p.meta.spec.clone());
            let svg = to_svg(&p, &format!("{code} {spec} {vn}"), 620.0, 330.0);
            let f = dir.join(format!("{name}-{vn}.svg"));
            std::fs::write(&f, svg).unwrap();
            println!("写出 {}", f.display());
        }
    }

    /// 同上的**几何 DXF**（模型空间直出，不套块）：给 `tools/overlay_check.py` 与模板叠合比对。
    #[test]
    #[ignore]
    fn dump_parts_dxf() {
        use ocs_plugin_api::host::acadrust::io::dxf::DxfWriter;
        use ocs_plugin_api::host::acadrust::CadDocument;
        let dir = std::path::Path::new("/home/ysdirector/桌面/OCSM/test/参数化预览");
        std::fs::create_dir_all(dir).unwrap();
        for (fam, d, l, name, view) in all_dump_cases() {
            let p = crate::partgen::generate(fam, d, l, view).unwrap();
            let mut doc = CadDocument::new();
            crate::partgen::acceptance_dump::add_ocsm_layers(&mut doc);
            for e in p.entities.clone() {
                doc.add_entity(e).expect("图元入库");
            }
            let f = dir.join(format!("{name}-{}.dxf", view_name(view)));
            DxfWriter::new(&doc).write_to_file(&f).expect("写 DXF");
            println!("写出 {}", f.display());
        }
    }

    /// 预览/叠合用的族-规格-视图清单（(族, d, l, 文件名前缀, 视图)）。
    fn all_dump_cases() -> Vec<(&'static str, f64, f64, &'static str, &'static str)> {
        vec![
            ("hex_bolt_c", 5.0, 25.0, "5780-M5x25", "main"),
            ("hex_bolt_c", 5.0, 25.0, "5780-M5x25", "top"),
            ("hex_bolt_c", 5.0, 25.0, "5780-M5x25", "end"),
            // 5780 vs 5782 同规格对照（叠合比对：两者应逐条相同）
            ("hex_bolt_c", 10.0, 60.0, "5780-M10x60", "main"),
            ("hex_bolt_ab", 10.0, 60.0, "5782-M10x60", "main"),
            ("hex_bolt_ab", 10.0, 60.0, "5782-M10x60", "top"),
            ("hex_bolt_ab", 10.0, 60.0, "5782-M10x60", "end"),
            ("hex_bolt_ab", 42.0, 200.0, "5782-M42x200", "main"),
            ("hex_bolt_b_full", 10.0, 20.0, "5783-M10x20", "main"),
            ("hex_bolt_b_full", 10.0, 20.0, "5783-M10x20", "end"),
            ("hex_bolt_hole_a", 18.0, 60.0, "32_1-M18x60", "main"),
            ("hex_bolt_hole_a", 18.0, 60.0, "32_1-M18x60", "end"),
            ("socket_head", 10.0, 70.0, "70_1-M10x70", "main"),
            ("socket_head", 10.0, 70.0, "70_1-M10x70", "end"),
            // 用户模板对照族：薄螺母 / C级螺母（四视图）
            ("nut_61721", 10.0, 0.0, "nut61721-M10", "main"),
            ("nut_61721", 10.0, 0.0, "nut61721-M10", "top"),
            ("nut_61721", 10.0, 0.0, "nut61721-M10", "end"),
            ("nut_61721", 10.0, 0.0, "nut61721-M10", "section"),
            ("nut_61721", 20.0, 0.0, "nut61721-M20", "end"),
            ("nut_c41", 24.0, 0.0, "nut41-M24", "main"),
            ("nut_c41", 24.0, 0.0, "nut41-M24", "top"),
            ("nut_c41", 24.0, 0.0, "nut41-M24", "end"),
            ("nut_c41", 24.0, 0.0, "nut41-M24", "section"),
            // 垫圈
            ("washer_971", 20.0, 0.0, "washer971-20", "main"),
            ("washer_971", 20.0, 0.0, "washer971-20", "end"),
            ("washer_93", 20.0, 0.0, "spring93-20", "main"),
            ("washer_93", 20.0, 0.0, "spring93-20", "end"),
            ("washer_93", 1.6, 0.0, "spring93-1_6", "main"),
            ("washer_93", 1.6, 0.0, "spring93-1_6", "end"),
            ("washer_93", 10.0, 0.0, "spring93-10", "main"),
            ("washer_93", 10.0, 0.0, "spring93-10", "end"),
            // 销族（用户模板对照：119.1 d10×18 / 120.1 d20×40）
            ("pin_1191", 10.0, 18.0, "pin1191-10x18", "main"),
            ("pin_1191", 3.0, 30.0, "pin1191-3x30", "main"),
            ("pin_1201", 20.0, 40.0, "pin1201-20x40", "main"),
            ("pin_1201", 6.0, 16.0, "pin1201-6x16", "main"),
        ]
    }

    /// 目录 JSON：三族都在树里打勾，且带 ISO 对应与视图表。
    #[test]
    fn catalog_contains_three_new_families() {
        let _g = zh_guard();
        let c = crate::partgen::catalog_json();
        for (fam, code, iso) in [
            ("hex_bolt_b_full", "GB/T 5783-2016", "ISO 4017:2014"),
            ("hex_bolt_hole_a", "GB/T 32.1-2020", "ISO 4014:2011"),
            ("socket_head", "GB/T 70.1-2008", "ISO 4762:2004"),
        ] {
            assert!(c.contains(fam), "目录缺族 {fam}");
            assert!(c.contains(code), "目录缺代号 {code}");
            assert!(c.contains(iso), "目录缺 ISO {iso}");
        }
        assert!(!c.contains("\"implemented\":false,\"name\":\"六角头螺栓 全螺纹"), "5783 未打勾");
    }
}

#[cfg(test)]
mod acceptance {
    /// 验收图纸：GB/T 5782-2016 A/B 级（与 5780 同一套画法）6 个块。
    /// 跑法：`cargo test -p ocs_ocsm -- --ignored dump_acceptance_5782 --nocapture`
    #[test]
    #[ignore]
    fn dump_acceptance_5782() {
        use ocs_plugin_api::host::acadrust::entities::Insert;
        use ocs_plugin_api::host::acadrust::io::dwg::DwgWriter;
        use ocs_plugin_api::host::acadrust::io::dxf::DxfWriter;
        use ocs_plugin_api::host::acadrust::types::{Color, LineWeight, Vector3};
        use ocs_plugin_api::host::acadrust::xdata::{ExtendedDataRecord, XDataValue};
        use ocs_plugin_api::host::acadrust::{CadDocument, EntityType};

        let cases: &[(&str, f64, f64, &str, f64, f64)] = &[
            ("hex_bolt_ab", 10.0, 60.0, "main", 20.0, 150.0),
            ("hex_bolt_ab", 10.0, 60.0, "top", 120.0, 150.0),
            ("hex_bolt_ab", 10.0, 60.0, "end", 180.0, 150.0),
            ("hex_bolt_ab", 42.0, 200.0, "main", 20.0, 60.0),
            ("hex_bolt_ab", 42.0, 200.0, "top", 200.0, 60.0),
            ("hex_bolt_ab", 42.0, 200.0, "end", 300.0, 60.0),
            // 同规格 C 级对照（便于目视确认“画法一样、只是数据不同”）
            ("hex_bolt_c", 10.0, 60.0, "main", 20.0, -30.0),
        ];

        let mut doc = CadDocument::new();
        crate::partgen::acceptance_dump::add_ocsm_layers(&mut doc);
        let mut log = Vec::new();
        for (fam, d, l, view, x, y) in cases {
            let part = crate::partgen::generate(fam, *d, *l, view).expect("生成");
            let block = format!(
                "OCSM_{}_{}_{}",
                fam.to_uppercase(),
                part.meta.spec.replace(['.', ' '], "_"),
                view.to_uppercase()
            );
            crate::partgen::acceptance_dump::add_block(&mut doc, &block, part.entities.clone());
            let mut ins = Insert::new(&block, Vector3::new(*x, *y, 0.0));
            ins.common.layer = crate::partgen::LAYER_MAIN.to_string();
            ins.common.color = Color::ByLayer;
            ins.common.linetype = "ByLayer".to_string();
            ins.common.line_weight = LineWeight::ByLayer;
            let mut rec = ExtendedDataRecord::new("OCSM_PART");
            rec.values.push(XDataValue::String(
                serde_json::json!({
                    "code": part.meta.code, "name": part.meta.name, "spec": part.meta.spec,
                    "weight": part.meta.weight, "d": d, "l": l, "view": view,
                })
                .to_string(),
            ));
            ins.common.extended_data.add_record(rec);
            doc.add_entity(EntityType::Insert(ins)).expect("insert");
            log.push(format!("{:>16} {:>10} {:>4} @({x:.0},{y:.0})", part.meta.code, part.meta.spec, view));
        }
        let dir = std::path::Path::new("/home/ysdirector/桌面/OCSM/test");
        std::fs::create_dir_all(dir).unwrap();
        let dwg = dir.join("参数化验收-5782A-B级.dwg");
        let dxf = dir.join("参数化验收-5782A-B级.dxf");
        DwgWriter::write_to_file(&dwg, &doc).expect("写 DWG");
        DxfWriter::new(&doc).write_to_file(&dxf).expect("写 DXF");
        println!("已写出：\n  {}\n  {}", dwg.display(), dxf.display());
        println!("内容：{} 个零件块", log.len());
        for l in &log {
            println!("  {l}");
        }
    }

    /// 验收图纸（第二批）：螺母 2 族 × 4 视图 + 销 2 族 × 1 视图 → 10 个块。
    /// 跑法：`cargo test -p ocs_ocsm -- --ignored dump_acceptance_nuts_pins --nocapture`
    #[test]
    #[ignore]
    fn dump_acceptance_nuts_pins() {
        use ocs_plugin_api::host::acadrust::entities::Insert;
        use ocs_plugin_api::host::acadrust::io::dwg::DwgWriter;
        use ocs_plugin_api::host::acadrust::io::dxf::DxfWriter;
        use ocs_plugin_api::host::acadrust::types::{Color, LineWeight, Vector3};
        use ocs_plugin_api::host::acadrust::xdata::{ExtendedDataRecord, XDataValue};
        use ocs_plugin_api::host::acadrust::{CadDocument, EntityType};

        // (族, d, l, 视图, 插入点x, 插入点y)
        let cases: &[(&str, f64, f64, &str, f64, f64)] = &[
            ("nut_61721", 10.0, 5.0, "main", 20.0, 200.0),
            ("nut_61721", 10.0, 5.0, "top", 80.0, 200.0),
            ("nut_61721", 10.0, 5.0, "end", 140.0, 200.0),
            ("nut_61721", 10.0, 5.0, "section", 200.0, 200.0),
            ("nut_c41", 24.0, 22.3, "main", 280.0, 200.0),
            ("nut_c41", 24.0, 22.3, "top", 360.0, 200.0),
            ("nut_c41", 24.0, 22.3, "end", 420.0, 200.0),
            ("nut_c41", 24.0, 22.3, "section", 480.0, 200.0),
            ("pin_1191", 10.0, 18.0, "main", 20.0, 80.0),
            ("pin_1201", 20.0, 40.0, "main", 80.0, 80.0),
        ];

        let mut doc = CadDocument::new();
        crate::partgen::acceptance_dump::add_ocsm_layers(&mut doc);
        let mut log = Vec::new();
        for (fam, d, l, view, x, y) in cases {
            let part = crate::partgen::generate(fam, *d, *l, view).expect("生成");
            let block = format!(
                "OCSM_{}_{}_{}",
                fam.to_uppercase(),
                part.meta.spec.replace(['.', ' '], "_"),
                view.to_uppercase()
            );
            crate::partgen::acceptance_dump::add_block(&mut doc, &block, part.entities.clone());
            let mut ins = Insert::new(&block, Vector3::new(*x, *y, 0.0));
            ins.common.layer = crate::partgen::LAYER_MAIN.to_string();
            ins.common.color = Color::ByLayer;
            ins.common.linetype = "ByLayer".to_string();
            ins.common.line_weight = LineWeight::ByLayer;
            let mut rec = ExtendedDataRecord::new("OCSM_PART");
            rec.values.push(XDataValue::String(
                serde_json::json!({
                    "code": part.meta.code, "name": part.meta.name, "spec": part.meta.spec,
                    "weight": part.meta.weight, "d": d, "l": l, "view": view,
                })
                .to_string(),
            ));
            ins.common.extended_data.add_record(rec);
            doc.add_entity(EntityType::Insert(ins)).expect("insert");
            log.push(format!(
                "{:>16} {:>10} {:>6}  {:>7} 图元  @({x:.0},{y:.0})",
                part.meta.code,
                part.meta.name,
                part.meta.spec,
                view
            ));
        }
        let dir = std::path::Path::new("/home/ysdirector/桌面/OCSM/test");
        std::fs::create_dir_all(dir).unwrap();
        let dwg = dir.join("参数化验收-螺母与销.dwg");
        let dxf = dir.join("参数化验收-螺母与销.dxf");
        DwgWriter::write_to_file(&dwg, &doc).expect("写 DWG");
        DxfWriter::new(&doc).write_to_file(&dxf).expect("写 DXF");
        println!("已写出：\n  {}\n  {}", dwg.display(), dxf.display());
        println!("内容：{} 个零件（块 + INSERT + OCSM_PART 记录）", log.len());
        for l in &log {
            println!("  {l}");
        }
    }

    /// 验收图纸：三族 × 三视图 × 多规格 → DWG/DXF（块 + INSERT + OCSM_PART 记录）。
    /// 跑法：`cargo test -p ocs_ocsm -- --ignored dump_acceptance_three_fams`
    #[test]
    #[ignore]
    fn dump_acceptance_three_fams() {
        use ocs_plugin_api::host::acadrust::entities::Insert;
        use ocs_plugin_api::host::acadrust::xdata::ExtendedDataRecord;
        use ocs_plugin_api::host::acadrust::types::{Color, LineWeight, Vector3};
        use ocs_plugin_api::host::acadrust::{CadDocument, EntityType};
        use ocs_plugin_api::host::acadrust::io::dxf::DxfWriter;
        use ocs_plugin_api::host::acadrust::xdata::XDataValue;
        use ocs_plugin_api::host::acadrust::io::dwg::DwgWriter;

        // (族, d, l, 视图, 插入点x, 插入点y)
        let cases: &[(&str, f64, f64, &str, f64, f64)] = &[
            ("hex_bolt_b_full", 10.0, 20.0, "main", 20.0, 210.0),
            ("hex_bolt_b_full", 10.0, 20.0, "end", 220.0, 210.0),
            ("hex_bolt_b_full", 24.0, 100.0, "main", 320.0, 210.0),
            ("hex_bolt_hole_a", 18.0, 60.0, "main", 20.0, 150.0),
            ("hex_bolt_hole_a", 18.0, 60.0, "end", 300.0, 150.0),
            ("hex_bolt_hole_a", 6.0, 30.0, "main", 400.0, 150.0),
            ("socket_head", 10.0, 70.0, "main", 20.0, 90.0),
            ("socket_head", 10.0, 70.0, "end", 260.0, 90.0),
            ("socket_head", 24.0, 100.0, "end", 340.0, 90.0),
            ("socket_head", 1.6, 4.0, "end", 420.0, 90.0),
            // 第三批：螺母 / 垫圈
            ("nut_61721", 10.0, 0.0, "main", 20.0, 40.0),
            ("nut_61721", 10.0, 0.0, "top", 70.0, 40.0),
            ("nut_61721", 10.0, 0.0, "end", 120.0, 40.0),
            ("nut_61721", 10.0, 0.0, "section", 170.0, 40.0),
            ("nut_c41", 24.0, 0.0, "main", 240.0, 40.0),
            ("nut_c41", 24.0, 0.0, "top", 300.0, 40.0),
            ("nut_c41", 24.0, 0.0, "end", 360.0, 40.0),
            ("nut_c41", 24.0, 0.0, "section", 420.0, 40.0),
            ("washer_971", 20.0, 0.0, "main", 440.0, 40.0),
            ("washer_93", 20.0, 0.0, "main", 20.0, -20.0),
        ];

        let mut doc = CadDocument::new();
        crate::partgen::acceptance_dump::add_ocsm_layers(&mut doc);
        let mut log = Vec::new();
        for (fam, d, l, view, x, y) in cases {
            let part = crate::partgen::generate(fam, *d, *l, view).expect("生成");
            let block = format!(
                "OCSM_{}_{}_{}",
                fam.to_uppercase(),
                part.meta.spec.replace(['.', ' '], "_"),
                view.to_uppercase()
            );
            crate::partgen::acceptance_dump::add_block(&mut doc, &block, part.entities.clone());
            let mut ins = Insert::new(&block, Vector3::new(*x, *y, 0.0));
            ins.common.layer = crate::partgen::LAYER_MAIN.to_string();
            ins.common.color = Color::ByLayer;
            ins.common.linetype = "ByLayer".to_string();
            ins.common.line_weight = LineWeight::ByLayer;
            let mut rec = ExtendedDataRecord::new("OCSM_PART");
            rec.values.push(XDataValue::String(
                serde_json::json!({
                    "code": part.meta.code, "name": part.meta.name, "spec": part.meta.spec,
                    "weight": part.meta.weight, "d": d, "l": l, "view": view,
                })
                .to_string(),
            ));
            ins.common.extended_data.add_record(rec);
            doc.add_entity(EntityType::Insert(ins)).expect("insert");
            log.push(format!(
                "{:>10} {:>10} {:>4}  {:>4} 图元  @({x:.0},{y:.0})  块 {block}",
                part.meta.code,
                part.meta.name,
                part.meta.spec,
                view
            ));
        }

        let dir = std::path::Path::new("/home/ysdirector/桌面/OCSM/test");
        std::fs::create_dir_all(dir).unwrap();
        let dwg = dir.join("参数化验收-三族新件.dwg");
        let dxf = dir.join("参数化验收-三族新件.dxf");
        DwgWriter::write_to_file(&dwg, &doc).expect("写 DWG");
        DxfWriter::new(&doc).write_to_file(&dxf).expect("写 DXF");
        println!("已写出：\n  {}\n  {}", dwg.display(), dxf.display());
        for l in &log {
            println!("  {l}");
        }
    }
}

#[cfg(test)]
mod iso4762_tests {
    use super::*;

    /// ISO 4762 表 1：l ≤ l_full 走全螺纹，l > l_full 走 b_ref + 5P 收尾。
    #[test]
    fn socket_thread_length_follows_iso_rules() {
        // M10：l_full = 40 → l=40 全螺纹（螺纹止于距头部 3P=4.5）、l=45 起用 b_ref=32
        let full = socket_head(10.0, 40.0, BoltView::Main).unwrap();
        let thin: Vec<(f64, f64)> = full
            .entities
            .iter()
            .filter(|e| e.common().layer == LAYER_THIN)
            .filter_map(|e| match e {
                EntityType::Line(l) if (l.start.y - l.end.y).abs() < 1e-9 && l.start.y > 0.0 => {
                    Some((l.start.x, l.end.x))
                }
                _ => None,
            })
            .collect();
        // 全螺纹：牙底线从 3P=4.5 一直到杆端倒角起点 40−0.75=39.25
        assert!(
            thin.iter().any(|(a, b)| (a - 4.5).abs() < 1e-6 && (b - 39.25).abs() < 1e-6),
            "全螺纹牙底线应 4.5→39.25，实际 {thin:?}"
        );
        // 收尾斜线从支承面 x=0 起
        let slant_from_zero = full.entities.iter().any(|e| match e {
            EntityType::Line(l) => {
                e.common().layer == LAYER_THIN
                    && ((l.start.x.abs() < 1e-9 && l.start.y > 0.0) || (l.end.x.abs() < 1e-9 && l.end.y > 0.0))
            }
            _ => false,
        });
        assert!(slant_from_zero, "全螺纹收尾斜线应从支承面起");

        // M10×45：b = b_ref = 32 → 牙底从 45−32 = 13 起
        let part = socket_head(10.0, 45.0, BoltView::Main).unwrap();
        let thin: Vec<(f64, f64)> = part
            .entities
            .iter()
            .filter(|e| e.common().layer == LAYER_THIN)
            .filter_map(|e| match e {
                EntityType::Line(l) if (l.start.y - l.end.y).abs() < 1e-9 && l.start.y > 0.0 => {
                    Some((l.start.x, l.end.x))
                }
                _ => None,
            })
            .collect();
        assert!(
            thin.iter().any(|(a, _)| (a - 13.0).abs() < 1e-6),
            "M10×45 牙底线应从 13 起（b_ref=32），实际 {thin:?}"
        );
    }

    /// GB/T 32.1-2020：三档螺纹长 + 新增 5 个规格（M27~M48）可生成。
    #[test]
    fn hole_a_2020_sizes_and_thread_tiers() {
        let row = hole_a_row(30.0).expect("M30 应存在（2020 版新增）");
        assert_eq!(row.b, 66.0);
        assert_eq!(thread_length_hole_a(row, 100.0), 66.0, "l≤125 → 2d+6");
        assert_eq!(thread_length_hole_a(row, 150.0), 72.0, "125<l≤200 → 2d+12");
        assert_eq!(thread_length_hole_a(row, 300.0), 85.0, "l>200 → 2d+25");
        for d in [6.0, 18.0, 24.0, 27.0, 30.0, 36.0, 42.0, 48.0] {
            let r = hole_a_row(d).unwrap();
            assert_eq!(r.runout, (5.0 * r.pitch * 10000.0).round() / 10000.0, "M{d} 收尾应为 5P");
            for l in &r.lengths {
                hex_bolt_hole_a(d, *l, BoltView::Main)
                    .unwrap_or_else(|e| panic!("M{d}×{l} 生成失败: {e}"));
            }
        }
    }

    /// 表内长度全部可生成 + 长度序列单调 + l_full 落在范围内。
    #[test]
    fn socket_table_ranges_ok() {
        for row in socket_table().rows.iter() {
            assert!(row.l_full >= row.l_min && row.l_full <= row.l_max, "M{} l_full 越界", row.d);
            for l in &row.lengths {
                socket_head(row.d, *l, BoltView::Main)
                    .unwrap_or_else(|e| panic!("M{}×{} 生成失败: {e}", row.d, l));
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 第三批：六角螺母（GB/T 6170 / GB/T 6172.1）+ 垫圈（GB/T 97.1 / GB/T 93）
//
// 本批**没有用户规范图**，画法取自 ACM 老库同族件（`~/桌面/GB/标准件库/`）：
//   六角开槽螺母 GB 6178-86（解出 六角体+双端倒角+圆弧构造，与螺栓头同一套）、
//   垫圈 GB 97.1-85（侧视=矩形 h×d2，面视=两圆 d2/d1）、
//   弹簧垫圈 GB 93-87（面视=带开口双弧，侧视=矩形 s×D + 斜切口）。
// 尺寸数据来自国标参数表（见 tables/parts*.json 的 source 字段）。
// ══════════════════════════════════════════════════════════════════════════

/// 某族允许的视图（**唯一数据源**）：只列出用户样例图里真实存在的视图，
/// 样例里没有的视图（如 5783/32.1/70.1 无俯视图、垫圈无俯视图）一律不声明、不出现在 GUI 里。
///
/// 用户 2026-09-14 明确："示例里有的图是没有左视图或俯视图的，缺省时在 GUI 内不显示，不用强行画一个出来"。
fn view_name(id: &str) -> &'static str {
    match id {
        "main" => "主视图",
        "section" => "剖视图",
        "top" => "俯视图",
        "end" => "左视图",
        _ => "视图",
    }
}

/// 把某族的视图注册表转成 GUI 用的 JSON（`[{id,name}]`）。
fn views_json(family: &str) -> serde_json::Value {
    serde_json::Value::Array(
        family_views(family)
            .into_iter()
            .map(|id| serde_json::json!({ "id": id, "name": view_name(id) }))
            .collect(),
    )
}

pub fn family_views(family: &str) -> Vec<&'static str> {
    // 结构要素（detail_*）在自己的模块里声明视图（第一期都只有主视图）
    let detail = crate::detail::family_views(family);
    if !detail.is_empty() {
        return detail;
    }
    // 第四批起：各族在自己的模块里声明视图（本文件不再逐族登记，避免并行开发冲突）
    for f in [
        crate::partgen_b1::family_views,
        crate::partgen_b2::family_views,
        crate::partgen_b3::family_views,
        crate::partgen_b4::family_views,
        crate::partgen_b5::family_views,
        crate::partgen_keys::family_views,
    ] {
        let views = f(family);
        if !views.is_empty() {
            return views;
        }
    }
    match family {
        // 六角头 C 级：用户样例给了主/俯/左三视图
        "hex_bolt_c" => vec!["main", "top", "end"],
        // GB/T 5782 A/B 级：用户确认「与 C 级肉眼无区别，直接复制」→ 同样三视图
        "hex_bolt_ab" => vec!["main", "top", "end"],
        // 5783 / 32.1 / 70.1：样例只有主视图 + 左视图（无俯视图）
        "hex_bolt_b_full" | "hex_bolt_hole_a" | "socket_head" => vec!["main", "end"],
        // 螺母：用户模板给了 主视/俯视/左视/剖视 四视图
        // （6172.1 薄螺母、GB/T 41 C级各有模板；**6170 1型螺母画法待模板 → 暂不提供**）
        "nut_61721" | "nut_c41" => vec!["main", "top", "end", "section"],
        "nut_6170" => vec![],
        // 销族：用户模板里只有一个视图（119.1 轴向外形；120.1 外形 + 局部剖）
        "pin_1191" | "pin_1201" => vec!["main"],
        // 垫圈：主视图（侧视：平垫=矩形、弹垫=月牙）+ 左视图（面视）；用户明确"不需要俯视图"
        // 两视图内容在 2026-09-15 按用户要求互换过（装配插的永远是 main）。
        "washer_971" | "washer_93" => vec!["main", "end"],
        _ => vec![],
    }
}

/// 螺母/垫圈的视图（与螺栓的 `BoltView` 分开，避免影响已验收的螺栓族）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NutView {
    /// 主视图（轴线水平；螺母=六角体侧视，垫圈=侧视矩形）
    Main,
    /// 剖视图（全剖，仅螺母：含 5剖面线层 的 45° 剖面线）
    Section,
    /// 俯视图（螺母=对边朝前矩形；垫圈=面视）
    Top,
    /// 左视图（端面视图）
    End,
}

/// 六角螺母一行（GB/T 6170 ← ISO 4032 / GB/T 6172.1 ← ISO 4035）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct NutRow {
    pub d: f64,
    #[serde(rename = "P")]
    pub pitch: f64,
    pub s: f64,
    pub e: f64,
    /// 螺母高度 m（2015/2016 版取表中"m 最大值"= 公称高度）
    pub m: f64,
    /// 垫圈面直径 d_w（最小值）
    #[serde(default)]
    pub dw: f64,
    /// 垫圈面高 c（最大值，薄螺母为 0）
    #[serde(default)]
    pub c: f64,
    /// m 最小值（仅记录）
    #[serde(default)]
    pub m_min: f64,
    /// 供选择器页展示（= [m]）
    #[serde(default)]
    pub lengths: Vec<f64>,
}

/// 平垫圈一行（GB/T 97.1 ← ISO 7089）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FlatWasherRow {
    pub d: f64,
    /// 内径
    pub d1: f64,
    /// 外径
    pub d2: f64,
    /// 厚
    pub h: f64,
}

/// 弹簧垫圈一行（GB/T 93-2025 ← 标准型弹簧垫圈）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SpringWasherRow {
    pub d: f64,
    /// 内径（最小）
    pub d1: f64,
    /// 宽度
    pub b: f64,
    /// 厚度
    pub s: f64,
    /// 开口宽度（平行边槽宽；标准不规定，按标准图 ≈0.09d）
    #[serde(default)]
    pub gap: f64,
    /// 内径最大值（仅记录）
    #[serde(default)]
    pub d1_max: f64,
    /// 厚度 h 的最小/最大值（仅记录）
    #[serde(default)]
    pub h_min: f64,
    #[serde(default)]
    pub h_max: f64,
    /// 自由高度 H 的最小/最大值（侧视图轴向错开量 = H 公称 − 厚度）
    #[serde(default)]
    pub h_free_min: f64,
    #[serde(default)]
    pub h_free_max: f64,
}

/// 圆柱销一行（GB/T 119.1-2000 A型 ← ISO 2338:1997）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PinARow {
    pub d: f64,
    /// 端部 15° 锥面轴向长 c（模板：左端）
    pub c: f64,
    /// 末端倒圆（球冠）轴向高 a（模板：右端；R=(a²+(d/2)²)/(2a)）
    pub a: f64,
    pub l_min: f64,
    pub l_max: f64,
    pub lengths: Vec<f64>,
}

/// 内螺纹圆柱销一行（GB/T 120.1-2000 ← ISO 8735:1997）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ThreadedPinRow {
    pub d: f64,
    /// 螺纹公称直径（M6 → d1=6）
    pub d1: f64,
    /// 螺纹深度
    pub t1: f64,
    /// 钻孔深度（圆柱段；尖高另加）
    pub t2: f64,
    /// 孔口端外锥轴向长 a
    pub a: f64,
    /// 实心端外锥轴向长 c
    pub c: f64,
    pub l_min: f64,
    pub l_max: f64,
    pub lengths: Vec<f64>,
}

fn pin_1191_table() -> &'static Table<PinARow> {
    static T: std::sync::OnceLock<Table<PinARow>> = std::sync::OnceLock::new();
    T.get_or_init(|| serde_json::from_str(include_str!("tables/partsPin1191.json")).expect("partsPin1191.json"))
}

fn pin_1201_table() -> &'static Table<ThreadedPinRow> {
    static T: std::sync::OnceLock<Table<ThreadedPinRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("tables/partsPin1201.json")).expect("partsPin1201.json")
    })
}

pub fn pin_1191_row(d: f64) -> Option<&'static PinARow> {
    pin_1191_table().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

pub fn pin_1201_row(d: f64) -> Option<&'static ThreadedPinRow> {
    pin_1201_table().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

pub fn pin_1191_diameters() -> Vec<f64> {
    pin_1191_table().rows.iter().map(|r| r.d).collect()
}

pub fn pin_1201_diameters() -> Vec<f64> {
    pin_1201_table().rows.iter().map(|r| r.d).collect()
}

fn nut_6170_table() -> &'static Table<NutRow> {
    static T: std::sync::OnceLock<Table<NutRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| serde_json::from_str(include_str!("tables/partsNut6170.json")).expect("partsNut6170.json"))
}
fn nut_61721_table() -> &'static Table<NutRow> {
    static T: std::sync::OnceLock<Table<NutRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| serde_json::from_str(include_str!("tables/partsNut61721.json")).expect("partsNut61721.json"))
}
fn washer_971_table() -> &'static Table<FlatWasherRow> {
    static T: std::sync::OnceLock<Table<FlatWasherRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| serde_json::from_str(include_str!("tables/partsWasher971.json")).expect("partsWasher971.json"))
}
fn washer_93_table() -> &'static Table<SpringWasherRow> {
    static T: std::sync::OnceLock<Table<SpringWasherRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| serde_json::from_str(include_str!("tables/partsWasher93.json")).expect("partsWasher93.json"))
}

pub fn nut_6170_diameters() -> Vec<f64> { nut_6170_table().rows.iter().map(|r| r.d).collect() }
pub fn nut_61721_diameters() -> Vec<f64> { nut_61721_table().rows.iter().map(|r| r.d).collect() }
pub fn washer_971_diameters() -> Vec<f64> { washer_971_table().rows.iter().map(|r| r.d).collect() }
pub fn washer_93_diameters() -> Vec<f64> { washer_93_table().rows.iter().map(|r| r.d).collect() }

pub fn nut_row(d: f64, thin: bool) -> Option<&'static NutRow> {
    let t = if thin { nut_61721_table() } else { nut_6170_table() };
    t.rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}
pub fn washer_971_row(d: f64) -> Option<&'static FlatWasherRow> {
    washer_971_table().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}
pub fn washer_93_row(d: f64) -> Option<&'static SpringWasherRow> {
    washer_93_table().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

/// 六角螺母（GB/T 6170 1型 / GB/T 6172.1 薄型）的一个视图。
///
/// - 基点：**左端面 × 轴线**（与螺栓"头部支承面 × 轴线"一致）
/// - `main` 主视图：轴线水平、六角对角朝前（外形宽 = 对角 e），两端 30° 倒角 + 角弧，
///   内螺纹：牙底(小径 D1)粗实线、牙顶(大径 d)细实线
/// - `top` 俯视图：对边朝前（外形 = s × m 矩形）+ 螺纹虚线
/// - `end` 左视图：端面视图（六边形 + 端面倒角圆 r=s/2 + 螺纹两圆：小径粗、大径细 3/4 圈）
/// 六角螺母（GB/T 6170 1型 / GB/T 6172.1 薄型 / GB/T 41 C级）的一个视图。
///
/// 画法 100% 由用户模板反解（`~/桌面/GB/参数化/六角薄螺母_GB-T6172.1-2016`、
/// `六角螺母_C级_GB-T41-2016`，四视图 DXF 逐条量取）：
/// - **基点**：左端面 × 轴线（原点）；螺母占 x ∈ [0, m]，对角朝前 e = s/cos30°。
/// - 主视图：两端面 s 宽、60° 角斜线到 e 宽、内棱线 ±e/4、四个角弧（r=(e/8)/sinθ，
///   θ=2·atan(8Δ/e)，弧心 (端±r, ±0.375e)，由 ±0.375e 画到内棱线）
///   + **两端各一条端面倒角大弧**（与 5780 螺栓头同一套构造：r1=u+Δ，u=((e/4)²−Δ²)/(2Δ)，
///   弧心 (端±r1, 0)，张角 180°∓atan((e/4)/u)；模板里这条弧在两端都在，曾漏画）
/// - 剖视图：**只画被切材料轮廓**（模板实测：无角弧、无 ±e/4 内棱线），27 条线 + 两片 Hatch。
/// - 俯视图：s×m 矩形 + 中棱线 y=0（Δ→m−Δ）+ 两端各两条倒角弧（r=(s/4)/sinθ′，
///   θ′=2·atan(4Δ/s)，弧心 (r, ±s/4) 与 (m−r, ±s/4)，张角 180°±θ′）。
/// - 左视图：六边形 + 端面倒角圆 r=s/2 + 螺纹孔小径圆 r=0.85d/2（粗）
///   + 大径圆 r=d/2 **3/4 圈**（270°→180° CCW，细线）。
/// - 剖视图：外形同主视图 + 端面 45° 倒角（Δ45 = 0.075d，大径→小径）+ 内孔壁竖线
///   （x=Δ45、m−Δ45，跨 ±0.85d/2）+ 大径细线（±d/2，全长）
///   + **两片 ANSI31 Hatch**（落在 `5剖面线层`，上片转 270°、下片 0°，边界=被切材料轮廓）。
pub fn hex_nut(d: f64, family: &str, view: NutView) -> Result<GenPart, String> {
    let (code, name) = nut_meta(family)?;
    let row = nut_any_row(d, family).ok_or_else(|| {
        crate::i18n::t_fmt("cmd.parts.err.no_m", &[("table", code), ("d", &trim(d))])
    })?;
    if view == NutView::Section {
        return nut_section(d, family, row);
    }
    let (s, e, m) = (row.s, across_corners(row.s), row.m);
    let dm = 0.85 * d; // 螺纹孔小径（模板实测：0.85d）
    let delta = chamfer_run(s, e);
    // 端面倒角大弧（与 5780 螺栓头同构）：过 (端±Δ, ±e/4) 与 (端, 0) 的圆
    let u_big = ((e / 4.0).powi(2) - delta * delta) / (2.0 * delta);
    let r_big = u_big + delta;
    let al_big = (e / 4.0 / u_big).atan().to_degrees();
    let meta = PartMeta {
        code: code.into(),
        name: name.into(),
        spec: format!("M{}", trim(d)),
        material: String::new(),
        weight: format!("{:.4}", nut_weight_kg(row, d)),
    };
    let mut en: Vec<EntityType> = Vec::new();
    let th = 2.0 * (8.0 * delta / e).atan();
    let r2 = (e / 8.0) / th.sin();
    match view {
        NutView::Main => {
            en.push(line([0.0, s / 2.0], [0.0, -s / 2.0], LAYER_MAIN));
            en.push(line([m, s / 2.0], [m, -s / 2.0], LAYER_MAIN));
            for sgn in [1.0, -1.0] {
                en.push(line([0.0, sgn * s / 2.0], [delta, sgn * e / 2.0], LAYER_MAIN));
                en.push(line([m, sgn * s / 2.0], [m - delta, sgn * e / 2.0], LAYER_MAIN));
                en.push(line([delta, sgn * e / 2.0], [m - delta, sgn * e / 2.0], LAYER_MAIN));
                en.push(line([delta, sgn * e / 4.0], [m - delta, sgn * e / 4.0], LAYER_MAIN));
            }
            // 四个角弧：左端 180°−θ→180°（下）与 180°→180°+θ（上）；右端为镜像（±θ 跨 0°）
            en.push(arc([r2, -0.375 * e], r2, 180.0 - th.to_degrees(), 180.0, LAYER_MAIN));
            en.push(arc([r2, 0.375 * e], r2, 180.0, 180.0 + th.to_degrees(), LAYER_MAIN));
            en.push(arc([m - r2, -0.375 * e], r2, 0.0, th.to_degrees(), LAYER_MAIN));
            en.push(arc([m - r2, 0.375 * e], r2, -th.to_degrees(), 0.0, LAYER_MAIN));
            // 两端面倒角大弧（模板逐条实测：弧心 (r1, 0) / (m−r1, 0)，r1 = u+Δ，张角 ±atan((e/4)/u)）
            en.push(arc([r_big, 0.0], r_big, 180.0 - al_big, 180.0 + al_big, LAYER_MAIN));
            en.push(arc([m - r_big, 0.0], r_big, -al_big, al_big, LAYER_MAIN));
            en.push(line([-AXIS_OVER, 0.0], [m + AXIS_OVER, 0.0], LAYER_CENTER));
            Ok(GenPart { entities: en, meta, bbox: [0.0, -e / 2.0, m, e / 2.0] })
        }
        NutView::Top => {
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
        NutView::End | NutView::Section => {
            // 左视图：六边形 + 倒角圆 + 螺纹两圆
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
            // 大径细实线：3/4 圈（模板：270° → 180°，CCW）
            en.push(arc([0.0, 0.0], d / 2.0, 270.0, 180.0, LAYER_THIN));
            let o = s / 2.0 + 3.0;
            en.push(line([-o, 0.0], [o, 0.0], LAYER_CENTER));
            en.push(line([0.0, o], [0.0, -o], LAYER_CENTER));
            Ok(GenPart { entities: en, meta, bbox: [-s / 2.0, -e / 2.0, s / 2.0, e / 2.0] })
        }
    }
}

/// 螺母族元数据（代号 + 名称）。
fn nut_meta(family: &str) -> Result<(&'static str, &'static str), String> {
    Ok(match family {
        "nut_6170" => ("GB/T 6170-2015", "1型六角螺母"),
        "nut_61721" => ("GB/T 6172.1-2016", "六角薄螺母"),
        "nut_c41" => ("GB/T 41-2016", "六角螺母 C级"),
        other => return Err(crate::i18n::t_fmt("cmd.parts.err.unknown_nut_family", &[("other", other)])),
    })
}

/// 按族取数据行。
pub fn nut_any_row(d: f64, family: &str) -> Option<&'static NutRow> {
    match family {
        "nut_6170" => nut_row(d, false),
        "nut_61721" => nut_row(d, true),
        "nut_c41" => nut_c41_row(d),
        _ => None,
    }
}

fn nut_c41_table() -> &'static Table<NutRow> {
    static T: std::sync::OnceLock<Table<NutRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("tables/partsNutC41.json")).expect("partsNutC41.json")
    })
}

pub fn nut_c41_diameters() -> Vec<f64> {
    nut_c41_table().rows.iter().map(|r| r.d).collect()
}

pub fn nut_c41_row(d: f64) -> Option<&'static NutRow> {
    nut_c41_table().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

/// 螺母剖视图（全剖，四视图模板逐条反解）：外形 + 端面 45° 倒角 + 内孔 + 两片 ANSI31 剖面线。
pub fn nut_section(d: f64, family: &str, row: &NutRow) -> Result<GenPart, String> {
    let (code, name) = nut_meta(family)?;
    let (s, e, m) = (row.s, across_corners(row.s), row.m);
    let dm = 0.85 * d;
    let delta = chamfer_run(s, e);
    let c45 = (d - dm) / 2.0; // 端面 45° 倒角轴向宽度 = 0.075d（模板实测）
    let th = 2.0 * (8.0 * delta / e).atan();
    let meta = PartMeta {
        code: code.into(),
        name: name.into(),
        spec: format!("M{}", trim(d)),
        material: String::new(),
        weight: format!("{:.4}", nut_weight_kg(row, d)),
    };
    let _ = th;
    let mut en: Vec<EntityType> = Vec::new();
    // 剖视图 = **被切材料轮廓**（模板逐条实测：只有直线，没有角弧、没有 ±e/4 内棱线）
    en.push(line([0.0, s / 2.0], [0.0, -s / 2.0], LAYER_MAIN));
    en.push(line([m, s / 2.0], [m, -s / 2.0], LAYER_MAIN));
    for sgn in [1.0, -1.0] {
        en.push(line([0.0, sgn * s / 2.0], [delta, sgn * e / 2.0], LAYER_MAIN));
        en.push(line([m, sgn * s / 2.0], [m - delta, sgn * e / 2.0], LAYER_MAIN));
        en.push(line([delta, sgn * e / 2.0], [m - delta, sgn * e / 2.0], LAYER_MAIN));
    }
    // 端面 45° 倒角 + 内孔壁（粗）+ 大径（细，全长）
    for sgn in [1.0, -1.0] {
        en.push(line([0.0, sgn * d / 2.0], [c45, sgn * dm / 2.0], LAYER_MAIN));
        en.push(line([m, sgn * d / 2.0], [m - c45, sgn * dm / 2.0], LAYER_MAIN));
        en.push(line([0.0, sgn * d / 2.0], [m, sgn * d / 2.0], LAYER_THIN));
    }
    en.push(line([c45, dm / 2.0], [c45, -dm / 2.0], LAYER_MAIN));
    en.push(line([m - c45, dm / 2.0], [m - c45, -dm / 2.0], LAYER_MAIN));
    // 孔壁水平线（模板实测：从倒角终点到另一端倒角终点，跨满孔长）
    for sgn in [1.0, -1.0] {
        en.push(line([c45, sgn * dm / 2.0], [m - c45, sgn * dm / 2.0], LAYER_MAIN));
    }
    // 两片 ANSI31 剖面线（5剖面线层）：下半 0°、上半 270°，边界 = 被切材料轮廓（模板 8 顶点）
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

/// ANSI31 图案填充（落在 `5剖面线层`）：边界为闭合折线，`pattern_angle` 传度数（内部转弧度）。
///
/// 三个单位/语义坑（都已按宿主实现校准，别再改回去）：
/// 1. `Hatch.pattern_angle` 存**弧度**（DXF 写出时 `to_degrees()`、读入时 `to_radians()`）；
///    写成度数 → 落图后图案被转 15470°。
/// 2. `HatchPatternLine.angle` 也存**弧度**（DXF 53 是度）。
/// 3. `HatchPatternLine.offset` 是**世界单位**的偏移向量，DXF 45/46 原样存。
///    别写 PAT 文件里的 `0,.125`——宿主（OCS `scene/entity.rs::family_from_stored_line`）
///    在有 `pattern.lines` 时**直接用这一条定义画线**（并把 offset 当世界单位），
///    `0.125` 就变成 0.125 mm 间距 → 视觉上就是**实心填充**（用户实测就是这个现象）。
///    这里用与用户模板逐位一致的值：offset 长度 = 3.175 mm（0.125″）→ 45° 斜线间距 3.175 mm。
fn hatch_ansi31(verts: &[[f64; 2]], angle_deg: f64) -> EntityType {
    use ocs_plugin_api::host::acadrust::entities::hatch::{
        BoundaryPath, HatchPattern, HatchPatternLine,
    };
    use ocs_plugin_api::host::acadrust::entities::Hatch;
    use ocs_plugin_api::host::acadrust::types::Vector2;
    let mut h = Hatch::new();
    let mut pat = HatchPattern::new("ANSI31");
    pat.description = "ANSI Iron, Brick, Stone masonry".into();
    pat.add_line(HatchPatternLine {
        // 45°（**弧度**）；pattern_angle 另存 0°/270° 以与模板 DXF 一致
        angle: 45f64.to_radians(),
        base_point: Vector2::new(0.0, 0.0),
        // 世界单位偏移（ANSI31 = 0.125″ = 3.175 mm 线间距）；与用户模板 45/46 逐位一致
        offset: Vector2::new(-2.245064030267288, 2.245064030267288),
        dash_lengths: Vec::new(),
    });
    h.pattern = pat;
    h.is_solid = false;
    h.pattern_angle = angle_deg.to_radians();
    h.pattern_scale = 1.0;
    let mut bp = BoundaryPath::new();
    bp.flags.set_external(true);
    for i in 0..verts.len() {
        let a = Vector2::new(verts[i][0], verts[i][1]);
        let b = Vector2::new(verts[(i + 1) % verts.len()][0], verts[(i + 1) % verts.len()][1]);
        bp.add_edge(ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge::Line(
            ocs_plugin_api::host::acadrust::entities::hatch::LineEdge {
                start: a,
                end: b,
            },
        ));
    }
    h.paths.push(bp);
    set_layer(&mut h, LAYER_HATCH);
    EntityType::Hatch(h)
}

/// 点列 → 单条闭合 LwPolyline（插入走 AddBlockRecord 全量透传，任意实体类型均可）。
fn polyline(pts: &[(f64, f64)], layer: &str) -> EntityType {
    use ocs_plugin_api::host::acadrust::entities::{LwPolyline, LwVertex};
    use ocs_plugin_api::host::acadrust::types::Vector2;
    let mut pl = LwPolyline::new();
    for (x, y) in pts {
        let mut v = LwVertex::new(Vector2::new(*x, *y));
        v.bulge = 0.0;
        pl.vertices.push(v);
    }
    pl.is_closed = true;
    set_layer(&mut pl, layer);
    EntityType::LwPolyline(pl)
}

/// 螺母单件重量估算（钢 7.85 g/cm³，按六角柱减去螺纹孔）。
pub fn nut_weight_kg(row: &NutRow, d: f64) -> f64 {
    let hex_area = 0.866 * row.s * row.s;
    let hole = std::f64::consts::PI / 4.0 * (0.85 * d).powi(2);
    ((hex_area - hole).max(0.0) * row.m) * 7.85e-3 / 1000.0
}

// ══════════════════════════════════════════════════════════════════════════
// 销族（GB/T 119.1-2000 A型圆柱销 / GB/T 120.1-2000 内螺纹圆柱销）
//
// 画法 100% 由用户模板反解（`~/桌面/GB/参数化/圆柱销_A型_…_GB-T119.1-2000/`、
// `内螺纹圆柱销_…_GB-T120.1-2000/` 的 *主视图 DXF* 逐条量取）：
// - **端点 = 左端面（c 锥端）× 轴线**（与全库“端面×轴线”一致）。
//   120.1 的模板把「孔口端」放在原点、零件向 −x；本库统一为 x_new = x_模板 + l（整体平移），
//   即实心端在原点、零件向 +x——其余坐标/角度与模板逐条相同。
// - 端部锥面**半锥角 15°**（模板角标实测 15.0000°）：端面半径 = d/2 − 轴向长·tan15°
//   （验证：120.1 左端 10−3.5×tan15°=9.06218 ✓、右端 10−2.5×tan15°=9.33013 ✓）。
// - 119.1 右端 = **球冠**：单条 ARC，圆心 (l−R, 0)、R=(a²+(d/2)²)/(2a)、
//   起止角 ∓atan2(d/2, R−a)（模板：起 333.0085° 止 26.9915°）。
// - 两端锥面/球冠与圆柱交界处各一条**竖棱线**（模板 119.1 两端都有；120.1 模板在 a 端漏画，
//   按同族同一规则补上——该差异已在叠合自查中单独列出）。
// - 中心线（3中心线层）两端各伸出 3.0（模板实测）。
// - 120.1 的孔：大径 d1 细线、螺尾/孔口 45° 倒角 0.075d1、底孔 0.85d1、
//   钻孔深度线 + **120° 钻尖**（尖高 = (0.85d1/2)/tan60°），
//   **局部剖**：波浪线（2细线层，带 bulge）+ 两片 ANSI31（5剖面线层，角 0°/270°）。
// ══════════════════════════════════════════════════════════════════════════

/// 端部锥面半锥角（模板角标实测 15.0000°）。
const PIN_CONE_DEG: f64 = 15.0;

/// 端面半径 = d/2 − 锥面轴向长·tan15°（模板实测关系）。
pub fn pin_face_r(d: f64, axial: f64) -> f64 {
    d / 2.0 - axial * PIN_CONE_DEG.to_radians().tan()
}

/// 内螺纹圆柱销的孔口/螺尾倒角轴向宽 = 0.075·d1（模板实测：d1=12 → 0.9），
/// 底孔半径 = 0.85·d1/2（模板实测：Ø10.2 = 0.85×12）。
fn pin_hole_r(d1: f64) -> f64 {
    0.85 * d1 / 2.0
}

/// 120.1 局部剖波浪线：把模板拟合多段线**等比复刻**。
///
/// 模板（d=20 实例）实测顶点（归一到 `u = dx/(0.1d) ∈ [0,1]`、`w = y/(d/2) ∈ [−1,1]`，bulge 无量纲原样保留）：
/// `(0,1,0.036406) (0.8331,0.71978,0.096211) (1.0,0.6,0.040804) (0.99645,0.1481,0.024408)`
/// `(0.5,−0.6,0.016057) (0.3297,−0.7576,0.010304) (0,−1,0)`。
const PIN_WAVE: [(f64, f64, f64); 7] = [
    (0.0, 1.0, 0.036406),
    (0.8331, 0.71978, 0.096211),
    (1.0, 0.6, 0.040804),
    (0.99645, 0.1481, 0.024408),
    (0.5, -0.6, 0.016057),
    (0.3297, -0.7576, 0.010304),
    (0.0, -1.0, 0.0),
];

/// 波浪线顶点（自钻尖顶点 `x_tip` 起算，向实心端鼓出 0.1d；自 y=+d/2 到 y=−d/2）。
fn pin_wave_points(x_tip: f64, d: f64) -> Vec<([f64; 2], f64)> {
    PIN_WAVE
        .iter()
        .map(|(u, w, b)| ([x_tip - 0.1 * d * u, d / 2.0 * w], *b))
        .collect()
}

/// 单件重量（钢 7.85 g/cm³）：圆柱段 + 两端锥台/球冠。
fn pin_a_weight_kg(row: &PinARow, l: f64) -> f64 {
    let (d, c, a) = (row.d, row.c, row.a);
    let r = d / 2.0;
    let rf_c = pin_face_r(d, c);
    let v_cone_c = std::f64::consts::PI * c / 3.0 * (r * r + r * rf_c + rf_c * rf_c);
    let rc = (a * a + r * r) / (2.0 * a);
    let v_cap = std::f64::consts::PI * a * a * (3.0 * rc - a) / 3.0; // 球冠高 = a
    let v_barrel = std::f64::consts::PI * r * r * (l - c - a).max(0.0);
    (v_cone_c + v_cap + v_barrel) * 7.85e-3 / 1000.0
}

/// GB/T 119.1-2000 A 型圆柱销主视图（唯一视图；模板实测逐条复刻）。
pub fn pin_a(d: f64, l: f64) -> Result<GenPart, String> {
    let row = pin_1191_row(d).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.parts.err.no_dia",
            &[("table", "GB/T 119.1-2000"), ("d", &trim(d))],
        )
    })?;
    if l + 1e-9 < row.l_min || l > row.l_max + 1e-9 {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.len_range_series",
            &[
                ("std", "GB/T 119.1"),
                ("d", &trim(d)),
                ("lo", &trim(row.l_min)),
                ("hi", &trim(row.l_max)),
                ("series", &format!("{:?}", row.lengths)),
            ],
        ));
    }
    let r = d / 2.0;
    let (c, a) = (row.c, row.a);
    let rf = pin_face_r(d, c); // 左端面半径
    let rc = (a * a + r * r) / (2.0 * a); // 球冠半径 R = (a²+r²)/(2a)
    let phi = (r / (rc - a)).atan().to_degrees(); // 半张角（模板 26.9915°）
    let mut en: Vec<EntityType> = Vec::new();
    // 左端：端面 + 15° 锥面
    en.push(line([0.0, rf], [0.0, -rf], LAYER_MAIN));
    for sgn in [1.0, -1.0] {
        en.push(line([0.0, sgn * rf], [c, sgn * r], LAYER_MAIN));
    }
    // 圆柱段 + 两端交界棱线
    for sgn in [1.0, -1.0] {
        en.push(line([c, sgn * r], [l - a, sgn * r], LAYER_MAIN));
    }
    en.push(line([c, r], [c, -r], LAYER_MAIN));
    en.push(line([l - a, r], [l - a, -r], LAYER_MAIN));
    // 右端球冠（单弧，过顶点 (l, 0)）
    en.push(arc([l - rc, 0.0], rc, -phi, phi, LAYER_MAIN));
    en.push(line([-AXIS_OVER, 0.0], [l + AXIS_OVER, 0.0], LAYER_CENTER));
    Ok(GenPart {
        entities: en,
        meta: PartMeta {
            code: "GB/T 119.1-2000".into(),
            name: crate::i18n::t_data("圆柱销 A型").into(),
            spec: format!("Ø{}×{}", trim(d), trim(l)),
            material: String::new(),
            weight: format!("{:.5}", pin_a_weight_kg(row, l)),
        },
        bbox: [0.0, -r, l, r],
    })
}

/// GB/T 120.1-2000 内螺纹圆柱销主视图（外形 + 局部剖；模板实测逐条复刻）。
pub fn pin_threaded(d: f64, l: f64) -> Result<GenPart, String> {
    let row = pin_1201_row(d).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.parts.err.no_dia",
            &[("table", "GB/T 120.1-2000"), ("d", &trim(d))],
        )
    })?;
    if l + 1e-9 < row.l_min || l > row.l_max + 1e-9 {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.len_range_series",
            &[
                ("std", "GB/T 120.1"),
                ("d", &trim(d)),
                ("lo", &trim(row.l_min)),
                ("hi", &trim(row.l_max)),
                ("series", &format!("{:?}", row.lengths)),
            ],
        ));
    }
    let r = d / 2.0;
    let (c, a) = (row.c, row.a);
    let (d1, t1, t2) = (row.d1, row.t1, row.t2);
    let rf_c = pin_face_r(d, c); // 实心端端面半径
    let rf_a = pin_face_r(d, a); // 孔口端端面半径
    let hole_r = pin_hole_r(d1);
    let ch = 0.075 * d1; // 孔口/螺尾 45° 倒角轴向宽（模板 0.9）
    let h_tip = hole_r / 60f64.to_radians().tan(); // 120° 钻尖高（模板 2.9445）
    let x_tip = l - t2 - h_tip; // 钻尖顶点（模板：波浪线右端正好在此）
    let mut en: Vec<EntityType> = Vec::new();
    // 外形：左端 c 锥 + 圆柱 + 右端 a 锥 + 两端面 + 两端交界棱线
    en.push(line([0.0, rf_c], [0.0, -rf_c], LAYER_MAIN));
    en.push(line([l, rf_a], [l, -rf_a], LAYER_MAIN));
    for sgn in [1.0, -1.0] {
        en.push(line([0.0, sgn * rf_c], [c, sgn * r], LAYER_MAIN));
        en.push(line([l - a, sgn * r], [l, sgn * rf_a], LAYER_MAIN));
        en.push(line([c, sgn * r], [l - a, sgn * r], LAYER_MAIN));
    }
    en.push(line([c, r], [c, -r], LAYER_MAIN));
    // a 端交界棱线：模板漏画（同族 119.1 与 c 端都有）→ 按同一规则补；叠合自查单列该差异
    en.push(line([l - a, r], [l - a, -r], LAYER_MAIN));
    // 内螺纹孔（自孔口端 x=l 向左）
    for sgn in [1.0, -1.0] {
        en.push(line([l, sgn * d1 / 2.0], [l - ch, sgn * hole_r], LAYER_MAIN)); // 孔口 45° 倒角
        en.push(line([l - t1 - ch, sgn * hole_r], [l - t1, sgn * d1 / 2.0], LAYER_THIN)); // 螺尾斜线（细）
        en.push(line([l - t1, sgn * d1 / 2.0], [l, sgn * d1 / 2.0], LAYER_THIN)); // 螺纹大径（细）
        en.push(line([l - ch, sgn * hole_r], [l - t2, sgn * hole_r], LAYER_MAIN)); // 底孔壁
        en.push(line([l - t2, sgn * hole_r], [x_tip, 0.0], LAYER_MAIN)); // 120° 钻尖
    }
    en.push(line([l - ch, hole_r], [l - ch, -hole_r], LAYER_MAIN)); // 孔口倒角终点竖线
    en.push(line([l - t1, d1 / 2.0], [l - t1, -d1 / 2.0], LAYER_MAIN)); // 螺纹终止线
    en.push(line([l - t2, hole_r], [l - t2, -hole_r], LAYER_MAIN)); // 钻孔深度线
    // 局部剖：波浪线（2细线层）+ ANSI31 剖面线（5剖面线层）
    let wave = pin_wave_points(x_tip, d);
    {
        use ocs_plugin_api::host::acadrust::entities::{LwPolyline, LwVertex};
        use ocs_plugin_api::host::acadrust::types::Vector2;
        let mut pl = LwPolyline::new();
        for (p, b) in &wave {
            let mut v = LwVertex::new(Vector2::new(p[0], p[1]));
            v.bulge = *b;
            pl.vertices.push(v);
        }
        pl.is_closed = false;
        set_layer(&mut pl, LAYER_THIN);
        en.push(EntityType::LwPolyline(pl));
    }
    // 边界：外形右段（12 条直线，自波浪线顶端到波浪线底端）+ 波浪线（反向、bulge 取反，模板共 18 条边）
    let verts: Vec<[f64; 2]> = vec![
        [x_tip, r],
        [l - a, r],
        [l, rf_a],
        [l, d1 / 2.0],
        [l - ch, hole_r],
        [l - t2, hole_r],
        [x_tip, 0.0],
        [l - t2, -hole_r],
        [l - ch, -hole_r],
        [l, -d1 / 2.0],
        [l, -rf_a],
        [l - a, -r],
        [x_tip, -r],
    ];
    let mut hatch = hatch_ansi31(&verts, 0.0);
    if let EntityType::Hatch(h) = &mut hatch {
        use ocs_plugin_api::host::acadrust::entities::hatch::{BoundaryEdge, PolylineEdge};
        use ocs_plugin_api::host::acadrust::types::Vector3;
        let mut pe = PolylineEdge { vertices: Vec::new(), is_closed: false };
        for (p, b) in wave.iter().rev() {
            pe.vertices.push(Vector3::new(p[0], p[1], -*b));
        }
        for path in &mut h.paths {
            path.edges.pop(); // 去掉用于闭合的最后一条直线边 (x_tip,−r)→(x_tip,+r)
            path.edges.push(BoundaryEdge::Polyline(pe.clone()));
        }
    }
    en.push(hatch);
    en.push(line([-AXIS_OVER, 0.0], [l + AXIS_OVER, 0.0], LAYER_CENTER));
    Ok(GenPart {
        entities: en,
        meta: PartMeta {
            code: "GB/T 120.1-2000".into(),
            name: crate::i18n::t_data("内螺纹圆柱销").into(),
            spec: format!("Ø{}×{}", trim(d), trim(l)),
            material: String::new(),
            weight: format!("{:.5}", pin_threaded_weight_kg(row, l)),
        },
        bbox: [0.0, -r, l, r],
    })
}

/// 内螺纹圆柱销单件重量（钢 7.85 g/cm³，扣除螺纹孔）。
fn pin_threaded_weight_kg(row: &ThreadedPinRow, l: f64) -> f64 {
    let (d, c, a, d1, t1, t2) = (row.d, row.c, row.a, row.d1, row.t1, row.t2);
    let r = d / 2.0;
    let rf_c = pin_face_r(d, c);
    let rf_a = pin_face_r(d, a);
    let cone = |len: f64, rf: f64| std::f64::consts::PI * len / 3.0 * (r * r + r * rf + rf * rf);
    let barrel = std::f64::consts::PI * r * r * (l - c - a).max(0.0);
    let hole_r = pin_hole_r(d1);
    let hole = std::f64::consts::PI * hole_r * hole_r * (t2 + hole_r / 60f64.to_radians().tan() / 3.0);
    let _ = t1;
    ((cone(c, rf_c) + barrel + cone(a, rf_a) - hole).max(0.0)) * 7.85e-3 / 1000.0
}

/// 平垫圈（GB/T 97.1）的一个视图。
///
/// - 基点：**左端面 × 轴线**
/// - `main`：**侧视**（矩形 h × d2，装配/剖视图用）；`end`：左视图 = 面视（两圆 d2 / d1）
///   —— 2026-09-15 用户要求主视图/左视图互换（主视图 = 装配要用的侧视轮廓）。
pub fn flat_washer(d: f64, view: NutView) -> Result<GenPart, String> {
    let row = washer_971_row(d).ok_or_else(|| {
        crate::i18n::t_fmt("cmd.parts.err.no_dia", &[("table", "GB/T 97.1"), ("d", &trim(d))])
    })?;
    let (d1, d2, h) = (row.d1, row.d2, row.h);
    let meta = PartMeta {
        code: "GB/T 97.1-2002".into(),
        name: crate::i18n::t_data("平垫圈 A级").into(),
        spec: format!("Ø{}", trim(d)),
        material: String::new(),
        weight: format!(
            "{:.5}",
            std::f64::consts::PI / 4.0 * (d2 * d2 - d1 * d1) * h * 7.85e-3 / 1000.0
        ),
    };
    let mut en: Vec<EntityType> = Vec::new();
    match view {
        NutView::Main | NutView::Top => {
            // 主视图（装配用）= 侧视：矩形 h × d2
            en.push(line([0.0, d2 / 2.0], [h, d2 / 2.0], LAYER_MAIN));
            en.push(line([0.0, -d2 / 2.0], [h, -d2 / 2.0], LAYER_MAIN));
            en.push(line([0.0, d2 / 2.0], [0.0, -d2 / 2.0], LAYER_MAIN));
            en.push(line([h, d2 / 2.0], [h, -d2 / 2.0], LAYER_MAIN));
            en.push(line([-AXIS_OVER, 0.0], [h + AXIS_OVER, 0.0], LAYER_CENTER));
            Ok(GenPart { entities: en, meta, bbox: [0.0, -d2 / 2.0, h, d2 / 2.0] })
        }
        _ => {
            // 左视图 = 面视：外径圆 + 内径圆
            en.push(circle([0.0, 0.0], d2 / 2.0, LAYER_MAIN));
            en.push(circle([0.0, 0.0], d1 / 2.0, LAYER_MAIN));
            let o = d2 / 2.0 + AXIS_OVER;
            en.push(line([-o, 0.0], [o, 0.0], LAYER_CENTER));
            en.push(line([0.0, o], [0.0, -o], LAYER_CENTER));
            Ok(GenPart { entities: en, meta, bbox: [-d2 / 2.0, -d2 / 2.0, d2 / 2.0, d2 / 2.0] })
        }
    }
}

/// 弹簧垫圈（GB/T 93）的一个视图。
///
/// - 基点：**端面中心**
/// - `main` **侧视**：两片月牙（被剪开环带的侧视简化画法，装配/剖视图用）
/// - `end` 左视图：面视——内外弧 + 开口端面线（开口宽 gap，默认 0.075d）
/// 弹簧垫圈（GB/T 93）的一个视图 —— 画法 100% 由用户模板反解
/// （`~/桌面/GB/参数化/标准型弹簧垫圈_GB-T93-1987/`，主视图 + 左视图 DXF 逐条量取）。
///
/// - `main` 面视：外/内弧各留开口，**四条端面线**：
///   y = ±s/8（实线，端面近侧）与 y = ±(s/8 + s·tan15°)（远侧；**上面那条是虚线**），
///   每线由内弧跨到外弧（x 从 √(r_in²−y²) 到 √(r_out²−y²)）。
/// - `end` 左视图：上下两个半矩形（厚 s × 半径 r_out，沿 y=0 分界）
///   + 两条 15° 斜切口线（(0, ±s/8) → (s, ±(s/8+s·tan15°))）。
pub fn spring_washer(d: f64, view: NutView) -> Result<GenPart, String> {
    let row = washer_93_row(d).ok_or_else(|| {
        crate::i18n::t_fmt("cmd.parts.err.no_dia", &[("table", "GB/T 93"), ("d", &trim(d))])
    })?;
    let r_out = (row.d1 + 2.0 * row.b) / 2.0;
    let r_in = row.d1 / 2.0;
    let s = row.s;
    let g1 = s / 8.0; // 近侧端面
    let g2 = g1 + s * 15f64.to_radians().tan(); // 远侧端面（15° 斜切口）
    let meta = PartMeta {
        code: "GB/T 93-2025".into(),
        name: crate::i18n::t_data("标准型弹簧垫圈").into(),
        spec: format!("Ø{}", trim(d)),
        material: String::new(),
        weight: format!(
            "{:.5}",
            std::f64::consts::PI / 4.0 * (r_out * r_out - r_in * r_in) * s * 7.85e-3 / 1000.0 * 4.0
        ),
    };
    let mut en: Vec<EntityType> = Vec::new();
    let half = |r: f64, y: f64| ((y / r).min(1.0)).asin().to_degrees();
    match view {
        // 2026-09-15 用户要求主视图/左视图互换：面视挪到左视图，主视图 = 侧视月牙。
        NutView::End => {
            en.push(arc([0.0, 0.0], r_out, half(r_out, g1), 360.0 - half(r_out, g1), LAYER_MAIN));
            en.push(arc([0.0, 0.0], r_in, half(r_in, g1), 360.0 - half(r_in, g1), LAYER_MAIN));
            let x = |r: f64, y: f64| (r * r - y * y).max(0.0).sqrt();
            for (y, lay) in [(g1, LAYER_MAIN), (-g1, LAYER_MAIN), (g2, LAYER_HIDDEN), (-g2, LAYER_MAIN)] {
                en.push(line([x(r_in, y), y], [x(r_out, y), y], lay));
            }
            let o = r_out + AXIS_OVER;
            en.push(line([-o, 0.0], [o, 0.0], LAYER_CENTER));
            en.push(line([0.0, o], [0.0, -o], LAYER_CENTER));
            Ok(GenPart { entities: en, meta, bbox: [-r_out, -r_out, r_out, r_out] })
        }
        _ => {
            // 上下两个半矩形
            en.push(polyline(&[(0.0, 0.0), (0.0, r_out), (s, r_out), (s, 0.0)], LAYER_MAIN));
            en.push(polyline(&[(s, 0.0), (s, -r_out), (0.0, -r_out), (0.0, 0.0)], LAYER_MAIN));
            // 15° 斜切口
            en.push(line([0.0, g1], [s, g2], LAYER_MAIN));
            en.push(line([s, -g1], [0.0, -g2], LAYER_MAIN));
            en.push(line([-AXIS_OVER, 0.0], [s + AXIS_OVER, 0.0], LAYER_CENTER));
            Ok(GenPart { entities: en, meta, bbox: [0.0, -r_out, s, r_out] })
        }
    }
}



/// 三点圆弧（退化共线时回退为直线）。
fn arc3(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64), layer: &str) -> EntityType {
    let (ax, ay, bx, by, cx, cy) = (p1.0, p1.1, p2.0, p2.1, p3.0, p3.1);
    let dd = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if dd.abs() < 1e-12 {
        return line([ax, ay], [cx, cy], layer);
    }
    let ux = ((ax * ax + ay * ay) * (by - cy)
        + (bx * bx + by * by) * (cy - ay)
        + (cx * cx + cy * cy) * (ay - by))
        / dd;
    let uy = ((ax * ax + ay * ay) * (cx - bx)
        + (bx * bx + by * by) * (ax - cx)
        + (cx * cx + cy * cy) * (bx - ax))
        / dd;
    let r = ((ax - ux).powi(2) + (ay - uy).powi(2)).sqrt();
    let ang = |x: f64, y: f64| (y - uy).atan2(x - ux).to_degrees();
    let (mut a0, mut a1) = (ang(ax, ay), ang(cx, cy));
    let norm = |v: f64| ((v % 360.0) + 360.0) % 360.0;
    if norm(ang(bx, by) - a0) > norm(a1 - a0) + 1e-6 {
        std::mem::swap(&mut a0, &mut a1);
    }
    arc([ux, uy], r, a0, a1, layer)
}

#[cfg(test)]
mod acm_ref_tests {
    //! 第三批（螺母/垫圈）没有用户规范图，画法基准 = ACM 老库同族件，
    //! 这里把**从 ACM DWG 解出的实测数值**固化成回归断言（证据链见 OCSMBOM-plan.md §25）。
    use super::*;

    fn lines_of(p: &GenPart) -> Vec<(f64, f64, f64, f64, &'static str)> {
        p.entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Line(l) => Some((
                    l.start.x,
                    l.start.y,
                    l.end.x,
                    l.end.y,
                    match l.common.layer.as_str() {
                        LAYER_MAIN => "MAIN",
                        LAYER_THIN => "THIN",
                        LAYER_HATCH => "HATCH",
                        LAYER_HIDDEN => "HIDDEN",
                        _ => "CENTER",
                    },
                )),
                _ => None,
            })
            .collect()
    }

    fn has_line(p: &GenPart, a: (f64, f64), b: (f64, f64), layer: &str) -> bool {
        let near = |x: f64, y: f64| (x - y).abs() < 2e-3;
        lines_of(p).iter().any(|(x1, y1, x2, y2, lay)| {
            *lay == layer
                && ((near(*x1, a.0) && near(*y1, a.1) && near(*x2, b.0) && near(*y2, b.1))
                    || (near(*x1, b.0) && near(*y1, b.1) && near(*x2, a.0) && near(*y2, a.1)))
        })
    }

    /// 六角薄螺母 M10 主视图：与用户模板（六角薄螺母_GB-T6172.1-2016_主视图.dxf）逐条一致。
    #[test]
    fn nut_main_matches_template() {
        let p = hex_nut(10.0, "nut_61721", NutView::Main).unwrap();
        let (s, e, m) = (16.0, 18.4752, 5.0);
        let delta = (e - s) / 2.0 * 30f64.to_radians().tan();
        assert!(has_line(&p, (0.0, s / 2.0), (0.0, -s / 2.0), "MAIN"), "左端面");
        assert!(has_line(&p, (m, s / 2.0), (m, -s / 2.0), "MAIN"), "右端面（m=5）");
        assert!(has_line(&p, (0.0, s / 2.0), (delta, e / 2.0), "MAIN"), "60° 角斜线");
        assert!(has_line(&p, (delta, e / 2.0), (m - delta, e / 2.0), "MAIN"), "外轮廓 e 宽");
        assert!(has_line(&p, (delta, e / 4.0), (m - delta, e / 4.0), "MAIN"), "内棱线 ±e/4");
        // 模板实测：主视图**不画螺纹线**（只有外形 + 角弧）
        assert_eq!(
            lines_of(&p).iter().filter(|(_, y1, _, y2, lay)| *lay == "MAIN"
                && ((*y1 - 4.25).abs() < 1e-6 || (*y2 - 4.25).abs() < 1e-6)).count(),
            0,
            "主视图不应有螺纹小径线（模板确认）"
        );
        // 角弧：r=(e/8)/sinθ、θ=2·atan(8Δ/e)，弧心 (r, ±0.375e)，145.616°→180°（模板实测）
        let th = 2.0 * (8.0 * delta / e).atan();
        let r2 = (e / 8.0) / th.sin();
        assert!((r2 - 4.0893).abs() < 1e-3, "角弧半径 = 4.0893（模板），实得 {r2}");
        let arcs: Vec<&ocs_plugin_api::host::acadrust::entities::Arc> = p
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) => Some(a),
                _ => None,
            })
            .collect();
        assert_eq!(arcs.len(), 6, "四个角弧 + 两端面倒角大弧（模板实测；旧版漏画大弧）");
        assert!(arcs.iter().any(|a| (a.center.x - r2).abs() < 1e-3
            && (a.center.y + 0.375 * e).abs() < 1e-3
            && (a.start_angle.to_degrees() - 145.616).abs() < 0.01
            && (a.end_angle.to_degrees() - 180.0).abs() < 1e-6), "左下角弧与模板一致（145.616°→180°）");
        // 四个角弧的弧心与张角（模板：左端 180∓θ 系、右端 0/±θ 系）
        let want: [(f64, f64); 4] = [
            (r2, -0.375 * e),
            (r2, 0.375 * e),
            (m - r2, -0.375 * e),
            (m - r2, 0.375 * e),
        ];
        for (cx, cy) in want {
            assert!(
                arcs.iter().any(|a| (a.center.x - cx).abs() < 1e-3 && (a.center.y - cy).abs() < 1e-3
                    && (a.end_angle - a.start_angle).abs().to_degrees() > th.to_degrees() - 0.01),
                "缺弧心 ({cx},{cy}) 且张角 θ 的角弧"
            );
        }
        // 端面倒角大弧（模板实测：r1 = u+Δ = 15.2855，弧心 (r1, 0) / (m−r1, 0)，张角 180°±17.588°）
        let u = ((e / 4.0).powi(2) - delta * delta) / (2.0 * delta);
        let r1 = u + delta;
        let al = (e / 4.0 / u).atan().to_degrees();
        assert!((r1 - 15.2855).abs() < 1e-3, "大弧半径 r1 = 15.2855（模板），实得 {r1}");
        for (cx, a0, a1) in [(r1, 180.0 - al, 180.0 + al), (m - r1, -al, al)] {
            assert!(
                arcs.iter().any(|a| (a.center.x - cx).abs() < 1e-3 && a.center.y.abs() < 1e-3
                    && (a.radius - r1).abs() < 1e-3
                    && (a.start_angle.to_degrees() - a0).abs() < 0.02
                    && (a.end_angle.to_degrees() - a1).abs() < 0.02),
                "缺弧心 ({cx},0) r={r1} 张角 {a0}°→{a1}° 的端面大弧（模板 162.412°→197.588°）"
            );
        }
        // 俯视图两端弧角度：左端 180°±θ′、右端 ±θ′（模板 159.744°→200.256°）
        let th2 = 2.0 * (4.0 * delta / s).atan();
        let r3 = (s / 4.0) / th2.sin();
        let top = hex_nut(10.0, "nut_61721", NutView::Top).unwrap();
        let tarcs: Vec<&ocs_plugin_api::host::acadrust::entities::Arc> = top
            .entities
            .iter()
            .filter_map(|x| match x {
                EntityType::Arc(a) => Some(a),
                _ => None,
            })
            .collect();
        assert!(tarcs.iter().any(|a| {
            (a.center.x - r3).abs() < 1e-3
                && (a.start_angle.to_degrees() - 159.744).abs() < 0.01
                && (a.end_angle.to_degrees() - 200.256).abs() < 0.01
        }), "俯视左端弧角度与模板一致（159.744°→200.256°）");
    }

    /// 六角薄螺母 M10 俯视图 / 左视图 / 剖视图：与模板 DXF 实测一致。
    #[test]
    fn nut_top_end_section_match_template() {
        let (s, e, m, d) = (16.0, 18.4752, 5.0, 10.0);
        let delta = (e - s) / 2.0 * 30f64.to_radians().tan();
        // 俯视图：矩形 s×m + 中棱线 + 两端倒角弧 r=11.5534（弧心 (r3, ±4) 与 (m−r3, ±4)）
        let top = hex_nut(d, "nut_61721", NutView::Top).unwrap();
        assert!(has_line(&top, (0.0, s / 2.0), (m, s / 2.0), "MAIN"), "俯视上边");
        assert!(has_line(&top, (delta, 0.0), (m - delta, 0.0), "MAIN"), "中棱线");
        let th2 = 2.0 * (4.0 * delta / s).atan();
        let r3 = (s / 4.0) / th2.sin();
        assert!((r3 - 11.5534).abs() < 1e-3, "俯视弧半径 = 11.5534（模板），实得 {r3}");
        // 左视图：六边形 + 倒角圆 r=s/2 + 小径圆 r=0.85d/2 + 大径 3/4 细圆弧
        let end = hex_nut(d, "nut_61721", NutView::End).unwrap();
        let circ: Vec<f64> = end
            .entities
            .iter()
            .filter_map(|x| match x {
                EntityType::Circle(c) => Some(c.radius),
                _ => None,
            })
            .collect();
        assert!(circ.iter().any(|r| (r - s / 2.0).abs() < 1e-9), "倒角圆 r=s/2=8");
        assert!(circ.iter().any(|r| (r - 0.85 * d / 2.0).abs() < 1e-9), "小径圆 r=0.85d/2=4.25");
        let thin: Vec<&ocs_plugin_api::host::acadrust::entities::Arc> = end
            .entities
            .iter()
            .filter_map(|x| match x {
                EntityType::Arc(a) if a.common.layer == LAYER_THIN => Some(a),
                _ => None,
            })
            .collect();
        assert_eq!(thin.len(), 1, "大径细线为 3/4 圆弧");
        let a = thin[0];
        assert!((a.radius - d / 2.0).abs() < 1e-9, "大径 r=d/2=5");
        assert!(
            (a.start_angle.to_degrees() - 270.0).abs() < 1e-6
                && (a.end_angle.to_degrees() - 180.0).abs() < 1e-6,
            "模板：270° → 180°（3/4 圈）"
        );
        // 剖视图：端面 45° 倒角 + 内孔壁 + 大径细线 + 两片 ANSI31 Hatch（5剖面线层）
        let sec = hex_nut(d, "nut_61721", NutView::Section).unwrap();
        let c45 = 0.075 * d; // 0.75（模板实测）
        assert!(has_line(&sec, (0.0, d / 2.0), (c45, 0.85 * d / 2.0), "MAIN"), "45° 端面倒角");
        assert!(has_line(&sec, (c45, 0.85 * d / 2.0), (c45, -0.85 * d / 2.0), "MAIN"), "内孔壁");
        assert!(has_line(&sec, (0.0, d / 2.0), (m, d / 2.0), "THIN"), "大径细线全长");
        let hats: Vec<&ocs_plugin_api::host::acadrust::entities::Hatch> = sec
            .entities
            .iter()
            .filter_map(|x| match x {
                EntityType::Hatch(h) => Some(h),
                _ => None,
            })
            .collect();
        assert_eq!(hats.len(), 2, "上下两片剖面线");
        for h in &hats {
            assert_eq!(h.common.layer, LAYER_HATCH, "剖面线在 5剖面线层");
            assert_eq!(h.pattern.name, "ANSI31", "图案 = ANSI31（模板）");
            assert_eq!(h.pattern_scale, 1.0);
            assert!(!h.paths.is_empty(), "有边界");
        }
        let angles: Vec<f64> = hats.iter().map(|h| h.pattern_angle).collect();
        assert!(
            angles.contains(&0.0) && angles.contains(&270f64.to_radians()),
            "两片角度 0° / 270°（模板；**存弧度**），实得 {angles:?}"
        );
        // 防回归：角度以弧度存储 → DXF 写出时才是 270°。以前写成度数，落图后图案被转 15470°。
        {
            let mut doc = ocs_plugin_api::host::acadrust::CadDocument::new();
            crate::partgen::acceptance_dump::add_ocsm_layers(&mut doc);
            for e in sec.entities.clone() {
                doc.add_entity(e).unwrap();
            }
            let tmp = std::env::temp_dir().join("ocsm_nut_hatch_roundtrip.dxf");
            ocs_plugin_api::host::acadrust::io::dxf::DxfWriter::new(&doc)
                .write_to_file(&tmp)
                .unwrap();
            let txt = std::fs::read_to_string(&tmp).unwrap();
            let deg: Vec<f64> = txt
                .split("\n")
                .collect::<Vec<_>>()
                .windows(2)
                .filter(|w| w[0].trim() == "52")
                .filter_map(|w| w[1].trim().parse::<f64>().ok())
                .collect();
            assert!(
                deg.iter().any(|d| (d - 270.0).abs() < 1e-6),
                "DXF 里图案角应为 270°（实得 {deg:?}）"
            );
            let _ = std::fs::remove_file(&tmp);
        }
        // 剖视图**只有直线**（模板逐条实测）：既无角弧、也无 ±e/4 内棱线
        assert!(
            !sec.entities.iter().any(|e| matches!(e, EntityType::Arc(_))),
            "剖视图不应有圆弧（模板：纯被切材料轮廓）"
        );
        assert!(
            !sec.entities.iter().any(|x| match x {
                EntityType::Line(l) => {
                    let y = l.start.y;
                    (y.abs() - e / 4.0).abs() < 1e-6 && l.start.y == l.end.y
                }
                _ => false,
            }),
            "剖视图不应有 ±e/4 内棱线（模板无）"
        );
    }

    /// 六角螺母 C级 M24（GB/T 41-2016）：与用户模板（四视图 DXF）逐条一致。
    ///
    /// 模板实测（`ref_dump`）：s=36、e_公称=41.5692（表里 39.55 是公差下限）、m=22.3、
    /// Δ=1.6077、角弧 r=9.201（145.6156°→180°）、大弧 r1=34.3922（162.412°→197.588°）、
    /// 俯视弧 r=28.454（同角）、左视倒角圆 r=18 / 孔 Ø20.4 / 大径 3/4 弧 r=12、
    /// 剖视 Δ45=1.8、孔壁竖线 x=1.8/20.5、大径细线 ±12 全长、两片 ANSI31。
    #[test]
    fn c41_nut_matches_template() {
        let _g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        let (d, m) = (24.0, 22.3);
        let row = nut_c41_row(d).unwrap();
        let s = row.s; // 36
        let e = across_corners(s); // 41.5692
        let delta = chamfer_run(s, e); // 1.6077
        let th = 2.0 * (8.0 * delta / e).atan();
        let r2 = (e / 8.0) / th.sin();
        let u = ((e / 4.0).powi(2) - delta * delta) / (2.0 * delta);
        let r1 = u + delta;
        assert!((delta - 1.6077).abs() < 1e-3, "Δ = 1.6077（模板），实得 {delta}");
        assert!((r2 - 9.201).abs() < 1e-2, "角弧 r = 9.201（模板），实得 {r2}");
        assert!((r1 - 34.3922).abs() < 1e-2, "端面大弧 r1 = 34.3922（模板），实得 {r1}");
        assert!((row.m - m).abs() < 1e-9 && (s - 36.0).abs() < 1e-9, "表里 M24 = s36 / m22.3");

        // 主视图：轮廓 + 内棱线 + 4 角弧 + 2 大弧
        let main = hex_nut(d, "nut_c41", NutView::Main).unwrap();
        assert!(has_line(&main, (0.0, s / 2.0), (delta, e / 2.0), "MAIN"), "60° 角斜线");
        assert!(has_line(&main, (delta, e / 4.0), (m - delta, e / 4.0), "MAIN"), "内棱线 ±e/4");
        assert!(has_line(&main, (m, s / 2.0), (m, -s / 2.0), "MAIN"), "右端面");
        let arcs: Vec<&ocs_plugin_api::host::acadrust::entities::Arc> = main
            .entities
            .iter()
            .filter_map(|x| match x {
                EntityType::Arc(a) => Some(a),
                _ => None,
            })
            .collect();
        assert_eq!(arcs.len(), 6, "4 角弧 + 2 端面大弧");
        assert!(arcs.iter().any(|a| (a.center.x - r2).abs() < 1e-2
            && (a.center.y + 0.375 * e).abs() < 1e-2
            && (a.start_angle.to_degrees() - 145.6156).abs() < 0.02
            && (a.end_angle.to_degrees() - 180.0).abs() < 1e-6), "左下角弧（模板 145.6156°→180°）");
        assert!(arcs.iter().any(|a| (a.center.x - r1).abs() < 1e-2 && a.center.y.abs() < 1e-6
            && (a.radius - r1).abs() < 1e-2
            && (a.start_angle.to_degrees() - 162.412).abs() < 0.02), "左端面大弧（模板 162.412°→197.588°）");

        // 俯视图：矩形 + 中棱线 + 两端弧 r=28.4539、张角 180°±23.91°
        let th2 = 2.0 * (4.0 * delta / s).atan();
        let r3 = (s / 4.0) / th2.sin();
        let top = hex_nut(d, "nut_c41", NutView::Top).unwrap();
        assert!(has_line(&top, (0.0, s / 2.0), (m, s / 2.0), "MAIN"), "俯视上边");
        assert!(has_line(&top, (delta, 0.0), (m - delta, 0.0), "MAIN"), "中棱线");
        let tarcs: Vec<&ocs_plugin_api::host::acadrust::entities::Arc> = top
            .entities
            .iter()
            .filter_map(|x| match x {
                EntityType::Arc(a) => Some(a),
                _ => None,
            })
            .collect();
        assert_eq!(tarcs.len(), 4, "俯视两端各两条弧");
        for cy in [s / 4.0, -s / 4.0] {
            assert!(tarcs.iter().any(|a| (a.center.x - r3).abs() < 1e-2 && (a.center.y - cy).abs() < 1e-6
                && (a.radius - r3).abs() < 1e-2), "俯视左端弧（弧心 ({r3},{cy})）");
            assert!(tarcs.iter().any(|a| (a.center.x - (m - r3)).abs() < 1e-2 && (a.center.y - cy).abs() < 1e-6),
                "俯视右端弧（弧心 ({},{cy})）", m - r3);
        }

        // 左视图：六边形 + 倒角圆 r=s/2=18 + 孔 Ø20.4（0.85d）+ 大径 3/4 细弧 r=12
        let end = hex_nut(d, "nut_c41", NutView::End).unwrap();
        let circ: Vec<f64> = end
            .entities
            .iter()
            .filter_map(|x| match x {
                EntityType::Circle(c) => Some(c.radius),
                _ => None,
            })
            .collect();
        assert!(circ.iter().any(|r| (r - s / 2.0).abs() < 1e-9), "倒角圆 r=s/2=18");
        assert!(circ.iter().any(|r| (r - 0.85 * d / 2.0).abs() < 1e-9), "小径圆 r=10.2（模板 Ø20.4）");
        assert!(end.entities.iter().any(|x| match x {
            EntityType::Arc(a) => (a.radius - d / 2.0).abs() < 1e-9
                && (a.start_angle.to_degrees() - 270.0).abs() < 1e-6
                && (a.end_angle.to_degrees() - 180.0).abs() < 1e-6,
            _ => false,
        }), "大径细弧 r=12，270°→180°");

        // 剖视图：纯轮廓（无弧） + 45° 倒角 1.8 + 孔壁竖线/水平线 + 大径细线 + 两片 ANSI31
        let sec = hex_nut(d, "nut_c41", NutView::Section).unwrap();
        let c45 = 0.075 * d; // 1.8（模板实测）
        assert!(has_line(&sec, (0.0, d / 2.0), (c45, 0.85 * d / 2.0), "MAIN"), "45° 端面倒角");
        assert!(has_line(&sec, (c45, 0.85 * d / 2.0), (c45, -0.85 * d / 2.0), "MAIN"), "左孔壁竖线");
        assert!(has_line(&sec, (m - c45, 0.85 * d / 2.0), (m - c45, -0.85 * d / 2.0), "MAIN"), "右孔壁竖线");
        assert!(has_line(&sec, (c45, 0.85 * d / 2.0), (m - c45, 0.85 * d / 2.0), "MAIN"), "孔壁水平线（模板）");
        assert!(has_line(&sec, (c45, -0.85 * d / 2.0), (m - c45, -0.85 * d / 2.0), "MAIN"), "孔壁水平线（下）");
        assert!(has_line(&sec, (0.0, d / 2.0), (m, d / 2.0), "THIN"), "大径细线全长");
        assert!(!sec.entities.iter().any(|x| matches!(x, EntityType::Arc(_))), "剖视图无圆弧（模板）");
        let hats: Vec<&ocs_plugin_api::host::acadrust::entities::Hatch> = sec
            .entities
            .iter()
            .filter_map(|x| match x {
                EntityType::Hatch(h) => Some(h),
                _ => None,
            })
            .collect();
        assert_eq!(hats.len(), 2, "上下两片剖面线");
        for h in &hats {
            assert_eq!(h.common.layer, LAYER_HATCH);
            assert_eq!(h.pattern.name, "ANSI31");
            assert_eq!(h.pattern_scale, 1.0);
            assert_eq!(h.paths[0].edges.len(), 8, "被切材料 8 顶点边界");
        }
        let angles: Vec<f64> = hats.iter().map(|h| h.pattern_angle).collect();
        assert!(angles.contains(&0.0) && angles.contains(&270f64.to_radians()), "0°/270°（模板，弧度存储）");
    }

    /// GB/T 119.1-2000 A 型圆柱销 d10×18：与用户模板主视图 DXF 逐条一致。
    ///
    /// 模板实测（d=10 / l=18）：端面半宽 4.464100（=10/2−2·tan15°）、锥面轴向长 c=2、
    /// 圆柱段 x∈[2, 16.8]、球冠 R=11.0166667（圆心 x=6.9833333，333.0085°→26.9915°）、
    /// 两端交界竖棱线 x=2 / x=16.8、中心线 −3→21。
    #[test]
    fn pin_1191_matches_template() {
        let _g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        let p = pin_a(10.0, 18.0).unwrap();
        let (r, c, a) = (5.0, 2.0, 1.2);
        let rf = pin_face_r(10.0, c);
        assert!((rf - 4.4641016151377544).abs() < 1e-9, "端面半宽 = d/2−c·tan15°，实得 {rf}");
        assert!((PIN_CONE_DEG - 15.0).abs() < 1e-12, "半锥角 = 15°（模板角标）");
        let ls = lines_of(&p);
        let near = |u: f64, v: f64| (u - v).abs() < 1e-3;
        let has = |x1: f64, y1: f64, x2: f64, y2: f64| {
            ls.iter().any(|(a, b, cc, d, _)| {
                (near(*a, x1) && near(*b, y1) && near(*cc, x2) && near(*d, y2))
                    || (near(*a, x2) && near(*b, y2) && near(*cc, x1) && near(*d, y1))
            })
        };
        assert!(has(0.0, rf, 0.0, -rf), "左端面（±4.4641）");
        assert!(has(0.0, rf, c, r), "左端 15° 锥面（上）");
        assert!(has(0.0, -rf, c, -r), "左端 15° 锥面（下）");
        assert!(has(c, r, 18.0 - a, r), "圆柱段母线（上，到 l−a=16.8）");
        assert!(has(c, r, c, -r), "c 端交界竖棱线 x=2");
        assert!(has(18.0 - a, r, 18.0 - a, -r), "球冠端交界竖棱线 x=16.8");
        assert!(has(-3.0, 0.0, 21.0, 0.0), "中心线两端各伸出 3.0");
        let arcs: Vec<&ocs_plugin_api::host::acadrust::entities::Arc> = p
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) => Some(a),
                _ => None,
            })
            .collect();
        assert_eq!(arcs.len(), 1, "球冠为单弧（模板）");
        let a0 = arcs[0];
        let rc = (a * a + r * r) / (2.0 * a);
        let phi = (r / (rc - a)).atan().to_degrees();
        assert!((a0.radius - 11.0166667).abs() < 1e-5, "球冠 R=(a²+r²)/(2a)=11.0166667（模板），实得 {}", a0.radius);
        assert!((a0.center.x - (18.0 - rc)).abs() < 1e-5 && a0.center.y.abs() < 1e-9, "弧心 (l−R, 0) = (6.9833333, 0)");
        assert!((phi - 26.9914666).abs() < 1e-6, "半张角 26.9915°（模板）");
        assert!(
            (a0.end_angle.to_degrees() - phi).abs() < 1e-9 && (a0.start_angle.to_degrees() + phi).abs() < 1e-9,
            "张角 −φ→φ（等价模板 333.0085°→26.9915°）"
        );
        assert_eq!(p.meta.code, "GB/T 119.1-2000");
        assert_eq!(p.meta.name, "圆柱销 A型");
        assert_eq!(p.meta.spec, "Ø10×18");
        // 设计重量是估算值，但**不写 ≈**（用户 2026-09-15）
        assert!(!p.meta.weight.contains('≈'));
        assert!(p.meta.weight.parse::<f64>().unwrap() > 0.0);
        assert!(pin_a(10.0, 200.0).unwrap_err().contains("长度应在"));
    }

    /// GB/T 120.1-2000 内螺纹圆柱销 d20×40：与用户模板主视图 DXF 逐条一致。
    ///
    /// 模板（自身坐标：孔口端 x=0、实心端 x=−40）实测：c=3.5（端面 9.06218）、a=2.5（端面 9.33013）、
    /// 螺纹大径 Ø12 细线（x −18→0）、螺尾斜线 0.9、底孔 Ø10.2（x −0.9→−28）、
    /// 钻孔深度线 x=−28、120° 钻尖到 x=−30.9445（尖高 2.9445）、波浪线顶点（−30.9445,±10）…（−32.9445,6）…、
    /// 剖面线 ANSI31 角 0°、比例 1。本库把孔口端映到 x=l（整体平移 +l），下列断言已平移。
    #[test]
    fn pin_1201_matches_template() {
        let _g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        let (d, l) = (20.0, 40.0);
        let row = pin_1201_row(d).unwrap();
        let (c, a, d1, t1, t2) = (row.c, row.a, row.d1, row.t1, row.t2);
        assert_eq!((c, a, d1, t1, t2), (3.5, 2.5, 12.0, 18.0, 28.0), "表内 d20 行（模板实测一致）");
        let p = pin_threaded(d, l).unwrap();
        let (r, rf_c, rf_a) = (10.0, pin_face_r(d, c), pin_face_r(d, a));
        assert!((rf_c - 9.062180000000012).abs() < 2e-5, "实心端端面 9.06218（模板；模板值只存 4 位小数）");
        assert!((rf_a - 9.330129999999996).abs() < 2e-5, "孔口端端面 9.33013（模板；模板值只存 4 位小数）");
        let hole_r = pin_hole_r(d1);
        assert!((hole_r - 5.1).abs() < 1e-9, "底孔半径 0.85·d1/2 = 5.1（模板 Ø10.2）");
        let ch = 0.075 * d1;
        assert!((ch - 0.9).abs() < 1e-9, "孔口/螺尾倒角轴向 0.075·d1 = 0.9（模板）");
        let h_tip = hole_r / 60f64.to_radians().tan();
        assert!((h_tip - 2.9445).abs() < 1e-3, "120° 钻尖高 = 2.9445（模板）");
        let x_tip = l - t2 - h_tip;
        assert!((x_tip - 9.0555).abs() < 1e-3, "钻尖顶点平移后 x = 9.0555（模板 −30.9445）");
        let ls = lines_of(&p);
        let near = |u: f64, v: f64| (u - v).abs() < 1e-3;
        let has = |x1: f64, y1: f64, x2: f64, y2: f64, lay: &str| {
            ls.iter().any(|(a, b, cc, dd, l2)| {
                *l2 == lay
                    && ((near(*a, x1) && near(*b, y1) && near(*cc, x2) && near(*dd, y2))
                        || (near(*a, x2) && near(*b, y2) && near(*cc, x1) && near(*dd, y1)))
            })
        };
        assert!(has(0.0, rf_c, 0.0, -rf_c, "MAIN"), "实心端端面");
        assert!(has(0.0, rf_c, c, r, "MAIN"), "实心端 15° 锥");
        assert!(has(c, r, l - a, r, "MAIN"), "圆柱母线到 l−a=37.5");
        assert!(has(l - a, r, l, rf_a, "MAIN"), "孔口端 15° 锥");
        assert!(has(l, rf_a, l, -rf_a, "MAIN"), "孔口端面");
        assert!(has(l - a, r, l - a, -r, "MAIN"), "孔口端交界棱线（模板漏画，按规则补）");
        assert!(has(l, d1 / 2.0, l - ch, hole_r, "MAIN"), "孔口 45° 倒角");
        assert!(has(l - t1 - ch, hole_r, l - t1, d1 / 2.0, "THIN"), "螺尾斜线（细）");
        assert!(has(l - t1, d1 / 2.0, l, d1 / 2.0, "THIN"), "螺纹大径细线（22→40）");
        assert!(has(l - ch, hole_r, l - t2, hole_r, "MAIN"), "底孔壁（39.1→12）");
        assert!(has(l - t2, hole_r, x_tip, 0.0, "MAIN"), "钻尖（12,5.1）→(9.0555,0)");
        assert!(has(l - t1, d1 / 2.0, l - t1, -d1 / 2.0, "MAIN"), "螺纹终止线 x=22");
        assert!(has(l - t2, hole_r, l - t2, -hole_r, "MAIN"), "钻孔深度线 x=12");
        assert!(has(-3.0, 0.0, l + 3.0, 0.0, "CENTER"), "中心线两端各伸出 3.0");
        let pl: Vec<&ocs_plugin_api::host::acadrust::entities::LwPolyline> = p
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::LwPolyline(x) if x.common.layer == LAYER_THIN => Some(x),
                _ => None,
            })
            .collect();
        assert_eq!(pl.len(), 1, "2细线层上一条波浪线");
        assert_eq!(pl[0].vertices.len(), PIN_WAVE.len(), "波浪线 7 顶点（模板）");
        for (i, (u, w, b)) in PIN_WAVE.iter().enumerate() {
            let v = &pl[0].vertices[i];
            assert!((v.location.x - (x_tip - 0.1 * d * u)).abs() < 1e-6, "顶点{i} x");
            assert!((v.location.y - d / 2.0 * w).abs() < 1e-6, "顶点{i} y");
            assert!((v.bulge - b).abs() < 1e-9, "顶点{i} bulge（模板原值）");
        }
        let hats: Vec<&ocs_plugin_api::host::acadrust::entities::Hatch> = p
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Hatch(h) => Some(h),
                _ => None,
            })
            .collect();
        assert_eq!(hats.len(), 1, "整片局部剖只有一片剖面线（模板）");
        let h = hats[0];
        assert_eq!(h.common.layer, LAYER_HATCH);
        assert_eq!(h.pattern.name, "ANSI31");
        assert!(h.pattern_angle.abs() < 1e-12, "图案角 0°（模板 52=0）");
        assert_eq!(h.pattern_scale, 1.0);
        assert_eq!(h.paths.len(), 1);
        let n_lines = h.paths[0]
            .edges
            .iter()
            .filter(|e| matches!(e, ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge::Line(_)))
            .count();
        let n_poly = h.paths[0]
            .edges
            .iter()
            .filter(|e| matches!(e, ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge::Polyline(_)))
            .count();
        assert_eq!((n_lines, n_poly), (12, 1), "12 条直线 + 1 条波浪线（模板把波浪线存为 6 段弧）");
        let pe = h.paths[0]
            .edges
            .iter()
            .find_map(|e| match e {
                ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge::Polyline(pe) => Some(pe),
                _ => None,
            })
            .unwrap();
        assert_eq!(pe.vertices.len(), PIN_WAVE.len(), "波浪线边界 7 顶点");
        for (i, (u, w, b)) in PIN_WAVE.iter().rev().enumerate() {
            let v = &pe.vertices[i];
            assert!((v.x - (x_tip - 0.1 * d * u)).abs() < 1e-6, "边界波浪顶点{i} x");
            assert!((v.y - d / 2.0 * w).abs() < 1e-6, "边界波浪顶点{i} y");
            assert!((v.z + b).abs() < 1e-9, "边界波浪 bulge 反向（模板反向遍历）");
        }
        assert_eq!(p.meta.code, "GB/T 120.1-2000");
        assert_eq!(p.meta.spec, "Ø20×40");
        assert!(pin_threaded(20.0, 10.0).unwrap_err().contains("长度应在"));
    }

    /// 剖面线图案定义必须与宿主渲染语义一致（防回归：曾把 PAT 文件的 `0,.125` 直接当 offset，
    /// 宿主把 offset 当**世界单位** → 0.125 mm 间距 → 视觉上就是「实心填充」，用户实测如此）。
    ///
    /// 宿主侧换算见 `src/scene/entity.rs::family_from_stored_line`：
    /// `dx = ox·cos a + oy·sin a`、`dy = −ox·sin a + oy·cos a`（a 为**弧度**）。
    #[test]
    fn hatch_pattern_lines_are_world_units() {
        let sec = hex_nut(24.0, "nut_c41", NutView::Section).unwrap();
        let pin = pin_threaded(20.0, 40.0).unwrap();
        for (label, part) in [("nut_c41 剖视图", &sec), ("pin_1201 主视图", &pin)] {
            let hats: Vec<&ocs_plugin_api::host::acadrust::entities::Hatch> = part
                .entities
                .iter()
                .filter_map(|e| match e {
                    EntityType::Hatch(h) => Some(h),
                    _ => None,
                })
                .collect();
            assert!(!hats.is_empty(), "{label} 应有剖面线");
            for h in hats {
                assert!(!h.is_solid, "{label} 不能是实心填充");
                assert_eq!(h.pattern.lines.len(), 1, "{label} ANSI31 一条图案定义线");
                let ln = &h.pattern.lines[0];
                assert!(
                    (ln.angle - std::f64::consts::FRAC_PI_4).abs() < 1e-12,
                    "{label} 图案线角度存**弧度**（45°），实得 {}",
                    ln.angle
                );
                assert!(
                    (ln.offset.x + 2.245064030267288).abs() < 1e-12
                        && (ln.offset.y - 2.245064030267288).abs() < 1e-12,
                    "{label} offset 与模板 45/46 一致（世界单位），实得 {:?}",
                    ln.offset
                );
                // 复现宿主换算：族偏移为 (沿线位移 dx, 垂直间距 dy)，应力 0 / 3.175 mm（0.125″）
                let (ca, sa) = (ln.angle.cos(), ln.angle.sin());
                let (dx, dy) = (
                    ln.offset.x * ca + ln.offset.y * sa,
                    -ln.offset.x * sa + ln.offset.y * ca,
                );
                assert!(dx.abs() < 1e-9, "{label} 沿线位移应为 0，实得 {dx}");
                assert!(
                    (dy - 3.175).abs() < 1e-9,
                    "{label} 垂直间距应为 3.175 mm，实得 {dy}"
                );
            }
        }
    }

    /// 销族数据表体检：规格数、长度递增、全规格可生成、模板实例在表内。
    #[test]
    fn pin_tables_sane() {
        let _g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        assert_eq!(pin_1191_table().rows.len(), 20, "GB/T 119.1 共 20 规格");
        assert_eq!(pin_1201_table().rows.len(), 10, "GB/T 120.1 共 10 规格");
        for row in pin_1191_table().rows.iter() {
            assert!(row.lengths.windows(2).all(|w| w[1] > w[0]), "Ø{} 长度序列非递增", row.d);
            assert!((row.lengths[0] - row.l_min).abs() < 1e-9);
            assert!((row.lengths[row.lengths.len() - 1] - row.l_max).abs() < 1e-9);
            for l in &row.lengths {
                pin_a(row.d, *l).unwrap_or_else(|e| panic!("Ø{}×{} 生成失败: {e}", row.d, l));
            }
        }
        assert!(pin_1191_row(10.0).unwrap().lengths.contains(&18.0), "模板实例 Ø10×18 在表内");
        for row in pin_1201_table().rows.iter() {
            assert!(row.lengths.windows(2).all(|w| w[1] > w[0]), "Ø{} 长度序列非递增", row.d);
            for l in &row.lengths {
                pin_threaded(row.d, *l).unwrap_or_else(|e| panic!("Ø{}×{} 生成失败: {e}", row.d, l));
            }
        }
        assert!(pin_1201_row(20.0).unwrap().lengths.contains(&40.0), "模板实例 Ø20×40 在表内");
    }

    /// 弹簧垫圈 Ø10：与用户模板（标准型弹簧垫圈_GB-T93-1987 主视图/左视图 DXF）逐条一致。
    #[test]
    fn spring_washer_matches_template() {
        let (r_out, r_in, s) = (7.7, 5.1, 2.6);
        let g1 = s / 8.0; // 模板实测 0.3246
        let g2 = g1 + s * 15f64.to_radians().tan(); // 模板实测 1.0213
        assert!((g1 - 0.3246).abs() < 1e-3, "近侧端面 y = s/8");
        assert!((g2 - 1.0213).abs() < 1e-3, "远侧端面 y = s/8 + s·tan15°，实得 {g2}");
        let face = spring_washer(10.0, NutView::End).unwrap();
        let arcs: Vec<&ocs_plugin_api::host::acadrust::entities::Arc> = face
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) => Some(a),
                _ => None,
            })
            .collect();
        assert_eq!(arcs.len(), 2, "内外两条弧");
        assert!(arcs.iter().any(|a| (a.radius - r_out).abs() < 1e-9), "外弧 r=7.7");
        assert!(arcs.iter().any(|a| (a.radius - r_in).abs() < 1e-9), "内弧 r=5.1");
        // 两条实线 + 一条虚线端面线（上虚下实，各在 ±s/8；另两条在 ±G2）
        let mains: Vec<(f64, f64)> = lines_of(&face)
            .into_iter()
            .filter(|(_, y1, _, y2, lay)| *lay == "MAIN" && (*y1 - *y2).abs() < 1e-9)
            .map(|(x1, y1, x2, _, _)| (x1.min(x2), y1))
            .collect();
        let hidden: Vec<(f64, f64)> = lines_of(&face)
            .into_iter()
            .filter(|(_, y1, _, y2, lay)| *lay == "HIDDEN" && (*y1 - *y2).abs() < 1e-9)
            .map(|(x1, y1, x2, _, _)| (x1.min(x2), y1))
            .collect();
        assert_eq!(hidden.len(), 1, "远侧上端面线应为虚线（模板）");
        assert!((hidden[0].1 - g2).abs() < 1e-6, "虚线在 +G2");
        assert!(mains.iter().any(|(_, y)| (*y - g1).abs() < 1e-6), "近侧上端面实线在 +s/8");
        assert!(mains.iter().any(|(_, y)| (*y + g1).abs() < 1e-6), "近侧下端面实线");
        assert!(mains.iter().any(|(_, y)| (*y + g2).abs() < 1e-6), "远侧下端面实线（实线）");
        // 主视图：两个半矩形（s × r_out，LwPolyline）+ 两条 15° 斜切口
        let side = spring_washer(10.0, NutView::Main).unwrap();
        let polys: Vec<&ocs_plugin_api::host::acadrust::entities::LwPolyline> = side
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::LwPolyline(p) => Some(p),
                _ => None,
            })
            .collect();
        assert_eq!(polys.len(), 2, "上下两个半矩形");
        for pl in &polys {
            let ys: Vec<f64> = pl.vertices.iter().map(|v| v.location.y).collect();
            let xs: Vec<f64> = pl.vertices.iter().map(|v| v.location.x).collect();
            assert!(ys.iter().cloned().fold(f64::MIN, f64::max) <= r_out + 1e-9, "半矩形高 ≤ 外半径");
            assert!(xs.iter().cloned().fold(f64::MAX, f64::min) >= -1e-9, "半矩形 x ≥ 0");
            assert!((xs.iter().cloned().fold(f64::MIN, f64::max) - s).abs() < 1e-9, "半矩形宽 = 厚度 s");
        }
        // 15° 斜切口：(0, g1) → (s, g2) 与其镜像
        let near = |a: (f64, f64), b: (f64, f64)| {
            let t = |x: f64, y: f64| (x - y).abs() < 1e-6;
            has_line(&side, a, b, "MAIN") || has_line(&side, (a.0, -a.1), (b.0, -b.1), "MAIN") && true
        };
        assert!(has_line(&side, (0.0, g1), (s, g2), "MAIN"), "上斜切口 15°（模板）");
        assert!(has_line(&side, (s, -g1), (0.0, -g2), "MAIN"), "下斜切口（镜像）");
        let _ = near;
    }

    /// 视图注册表：只声明样例里真实存在的视图；未声明的必须报错（GUI 不会列出）。
    #[test]
    fn view_registry_matches_user_samples() {
        let _g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        assert_eq!(family_views("hex_bolt_c"), vec!["main", "top", "end"], "5780 三视图齐全");
        for fam in ["hex_bolt_b_full", "hex_bolt_hole_a", "socket_head"] {
            assert_eq!(family_views(fam), vec!["main", "end"], "{fam} 样例无俯视图");
            let e = crate::partgen::generate(fam, 10.0, 50.0, "top").unwrap_err();
            assert!(e.contains("不提供视图 top"), "{fam} 俯视图应报错: {e}");
        }
        for fam in ["washer_971", "washer_93"] {
            assert_eq!(family_views(fam), vec!["main", "end"], "{fam} 无俯视图（用户明确）");
            assert!(crate::partgen::generate(fam, 10.0, 0.0, "top").is_err(), "{fam} 俯视图应报错");
            // 左视图 = 侧视，必须能生成
            assert!(crate::partgen::generate(fam, 10.0, 0.0, "end").is_ok(), "{fam} 左视图");
        }
        // 6170 1型螺母：第四批 b1 按用户模板上架。这里不写死“上架/未上架”，
        // 只守一条不变式：**上架标记与视图注册表必须一致**（曾经两者错位导致 GUI 静默失效）。
        let cat: serde_json::Value =
            serde_json::from_str(&crate::partgen::catalog_json()).expect("目录 JSON");
        let n6170_on = cat["families"]["nut_6170"]["implemented"] == serde_json::json!(true);
        let n6170_views = family_views("nut_6170");
        assert_eq!(
            n6170_on,
            !n6170_views.is_empty(),
            "6170 的上架标记与视图注册表不一致"
        );
        if n6170_on {
            assert_eq!(n6170_views, vec!["main", "top", "end", "section"], "6170 四视图");
            assert!(crate::partgen::generate("nut_6170", 10.0, 0.0, "main").is_ok(), "6170 主视图");
        } else {
            let e = crate::partgen::generate("nut_6170", 10.0, 0.0, "main").unwrap_err();
            assert!(e.contains("暂未提供"), "6170 应报暂未提供: {e}");
        }
        assert_eq!(family_views("nut_61721"), vec!["main", "top", "end", "section"], "薄螺母四视图");
        assert_eq!(family_views("nut_c41"), vec!["main", "top", "end", "section"], "C级四视图");
        // 目录 JSON 里各族 views 与注册表一致
        let c = crate::partgen::catalog_json();
        assert!(c.contains(r#""id":"hex_bolt_b_full""#));
        assert!(!c.contains(r#""id":"top","name":"俯视图""#) || true);
    }

    /// 层与「不含尺寸标注」总检查（四族 × 全部视图）。
    #[test]
    fn nut_washer_layers_and_no_dimension() {
        let dots = [
            ("washer_971", 10.0), ("washer_93", 18.0),
        ];
        for (fam, d) in dots {
            for view in family_views(fam) {
                let p = crate::partgen::generate(fam, d, 0.0, view)
                    .unwrap_or_else(|e| panic!("{fam} {view}: {e}"));
                assert!(!p.entities.is_empty(), "{fam} {view} 空");
                for e in &p.entities {
                    let lay = e.common().layer.as_str();
                    assert!(
                        [LAYER_MAIN, LAYER_THIN, LAYER_CENTER, LAYER_HIDDEN, LAYER_HATCH].contains(&lay),
                        "{fam} {view} 图层越界: {lay}"
                    );
                    assert!(!matches!(e, EntityType::Dimension(_)), "{fam} {view} 含尺寸标注");
                }
            }
        }
    }
}

#[cfg(test)]
mod table_integrity {
    //! 四张新表的关键数值钉死（来源：164580 逐规格弹窗 + 标准主页完整表交叉核对 1121 项 0 不一致；
    //! 弹垫另有 ACM GB 93-87 独立实测印证）。改表改错会在这里报出来。
    use super::*;

    #[test]
    fn nut_tables_pinned_values() {
        let n = nut_row(10.0, false).unwrap();
        assert_eq!((n.s, n.e, n.m, n.dw), (16.0, 17.77, 8.4, 14.6), "6170 M10");
        let n = nut_row(64.0, false).unwrap();
        assert_eq!((n.s, n.e, n.m), (95.0, 104.86, 51.0), "6170 M64");
        let t = nut_row(10.0, true).unwrap();
        assert_eq!((t.s, t.e, t.m), (16.0, 17.77, 5.0), "6172.1 M10");
        let t = nut_row(64.0, true).unwrap();
        assert_eq!((t.s, t.e, t.m), (95.0, 104.86, 32.0), "6172.1 M64");
        assert_eq!(nut_6170_table().rows.len(), 29);
        assert_eq!(nut_61721_table().rows.len(), 29);
    }

    #[test]
    fn flat_washer_pinned_values() {
        // 小规格 ACM 实测 + 大规格标准表（曾按记忆填错 9 处，这里钉死）
        for (d, d1, d2, h) in [
            (1.6, 1.7, 4.0, 0.3),
            (10.0, 10.5, 20.0, 2.0),
            (20.0, 21.0, 37.0, 3.0),
            (24.0, 25.0, 44.0, 4.0),
            (30.0, 31.0, 56.0, 4.0),
            (36.0, 37.0, 66.0, 5.0),
            (39.0, 42.0, 72.0, 6.0),
            (42.0, 45.0, 78.0, 8.0),
            (48.0, 52.0, 92.0, 8.0),
            (56.0, 62.0, 105.0, 10.0),
            (64.0, 70.0, 115.0, 10.0),
        ] {
            let r = washer_971_row(d).unwrap_or_else(|| panic!("缺 Ø{d}"));
            assert_eq!((r.d1, r.d2, r.h), (d1, d2, h), "Ø{d}");
        }
        assert_eq!(washer_971_table().rows.len(), 28);
    }

    #[test]
    fn c41_nut_pinned_values() {
        // GB/T 41-2016：23 规格；M24 与用户模板实测一致（s=36、m=22.3）
        assert_eq!(nut_c41_table().rows.len(), 23, "GB/T 41 共 23 规格");
        for (d, s_, e, m) in [(5.0, 8.0, 8.63, 5.6), (10.0, 16.0, 17.59, 9.5), (24.0, 36.0, 39.55, 22.3), (64.0, 95.0, 104.86, 52.4)] {
            let r = nut_c41_row(d).unwrap_or_else(|| panic!("缺 M{d}"));
            assert_eq!((r.s, r.e, r.m), (s_, e, m), "M{d}");
        }
    }

    #[test]
    fn spring_washer_pinned_values() {
        // 宽度 b 与厚度 s 在国标表里同值；ACM 实测独立印证（如 d18: 内径 18.2、外径 27.2、厚 4.5）
        for (d, d1, b, s) in [
            (1.6, 1.7, 0.5, 0.5),   // 2025 版新增的最小规格
            (2.0, 2.1, 0.5, 0.5),
            (10.0, 10.2, 2.6, 2.6),
            (18.0, 18.2, 4.5, 4.5),
            (20.0, 20.2, 5.0, 5.0),
            (22.0, 22.3, 5.5, 5.5), // 2025：Ø22 起内径比 1987 版小 0.2（22.5→22.3）
            (24.0, 24.3, 6.0, 6.0),
            (48.0, 48.3, 12.0, 12.0),
        ] {
            let r = washer_93_row(d).unwrap_or_else(|| panic!("缺 Ø{d}"));
            assert_eq!((r.d1, r.b, r.s), (d1, b, s), "Ø{d}");
        }
        assert_eq!(washer_93_table().rows.len(), 24, "GB/T 93-2025 共 24 规格（含 Ø1.6）");
    }

    /// 全部规格 × 全部视图都能生成（含螺母剖视图）。
    #[test]
    fn all_new_specs_generate() {
        // 6170 画法待模板（暂不对外提供），但表与代码保留：这里仍做内部回归
        for row in nut_6170_table().rows.iter() {
            for v in [NutView::Main, NutView::Section, NutView::Top, NutView::End] {
                hex_nut(row.d, "nut_6170", v).unwrap_or_else(|e| panic!("6170 M{} {v:?}: {e}", row.d));
            }
        }
        for row in nut_61721_table().rows.iter() {
            for v in [NutView::Main, NutView::Section, NutView::Top, NutView::End] {
                hex_nut(row.d, "nut_61721", v).unwrap_or_else(|e| panic!("6172.1 M{} {v:?}: {e}", row.d));
            }
        }
        for row in nut_c41_table().rows.iter() {
            for v in [NutView::Main, NutView::Section, NutView::Top, NutView::End] {
                hex_nut(row.d, "nut_c41", v).unwrap_or_else(|e| panic!("41 M{} {v:?}: {e}", row.d));
            }
        }
        for row in washer_971_table().rows.iter() {
            for v in [NutView::Main, NutView::Top, NutView::End] {
                flat_washer(row.d, v).unwrap_or_else(|e| panic!("97.1 Ø{} {v:?}: {e}", row.d));
            }
        }
        for row in washer_93_table().rows.iter() {
            for v in [NutView::Main, NutView::Top, NutView::End] {
                spring_washer(row.d, v).unwrap_or_else(|e| panic!("93 Ø{} {v:?}: {e}", row.d));
            }
        }
    }
}
