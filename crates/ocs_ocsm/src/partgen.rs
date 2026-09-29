//! 标准件**参数化生成**（GB/T 4458 简化画法，与库内 ACM 件风格一致）。
//!
//! 为什么参数化：图库缺的恰好是最常用的族（普通六角螺母/平键/轴承/挡圈 =
//! 0 个目录；GB 5782/5783 只有 M5/M6；内六角螺钉只有 M1.6/M2）。
//! 尺寸数据取国标的采标源（GB/T 5782←ISO 4014、GB/T 6170←ISO 4032…），
//! 见 `src/tables/parts*.json`。
//!
//! ## 画法约定（全族统一，便于选择器/BOM 复用）
//! - **基点落在原点**：螺栓 = 头部支承面 × 轴线；螺母 = 底面 × 轴线；垫圈/销 = 端面中心。
//! - **图层**：轮廓 → `1轮廓实线层`；牙底/细线 → `2细线层`；中心线 → `3中心线层`；
//!   颜色/线型/线宽一律 ByLayer。
//! - **螺纹画法（GB/T 4459.1）**：外螺纹 牙顶（大径 d）粗实线、牙底（小径 d1）细实线；
//!   内螺纹反过来（小径粗、大径细）。小径 d1 ≈ d − 1.0825·P。
//! - 螺栓主视图：轴线水平、头在 −x 侧、杆向 +x 伸长；六角头端面画 s 宽，
//!   外轮廓画对角宽 e = s/cos30°，两端 30° 倒角斜线。
//! - 螺母主视图：底面 y=0、顶面 y=m，对边宽 s 居中，四角 30° 倒角。

use ocs_plugin_api::host::acadrust;
use ocs_plugin_api::host::acadrust::entities::{Arc, Circle, EntityType, Line};
use ocs_plugin_api::host::acadrust::types::{Color, LineWeight, Vector3};

/// 轮廓（粗实线）层。
pub const LAYER_MAIN: &str = "1轮廓实线层";
/// 细实线层（牙底、细线）。
pub const LAYER_THIN: &str = "2细线层";
/// 中心线层（点划线）。
pub const LAYER_CENTER: &str = "3中心线层";

const COS30: f64 = 0.866_025_403_784_438_6;
/// 库内 ACM 件实测：六角头倒角弧半径 = 0.72 × 对边宽。
const CHAMFER_R: f64 = 0.72;
/// 库内 ACM 件实测：倒角弧的一半张角（度）。
const CHAMFER_HALF_DEG: f64 = 20.3;

/// 外/内螺纹小径（GB/T 197 简化式）。
pub fn minor_dia(d: f64, pitch: f64) -> f64 {
    d - 1.0825 * pitch
}

/// 六角头对角宽度（公称，CAD 画图用；表里的 e 是公差下限）。
pub fn across_corners(s: f64) -> f64 {
    s / COS30
}

/// 一条线（落在指定图层）。
fn line(a: [f64; 2], b: [f64; 2], layer: &str) -> EntityType {
    let mut l = Line::from_points(Vector3::new(a[0], a[1], 0.0), Vector3::new(b[0], b[1], 0.0));
    set_layer(&mut l, layer);
    EntityType::Line(l)
}

/// 圆弧：入参用**度**（与 DXF/GB 画法一致），内部转弧度（acadrust 的角度字段是弧度）。
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

/// 整圆。
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

/// 让 set_layer 同时适配 Line/Arc/Circle。
trait CommonLayer {
    fn common_mut(&mut self) -> &mut acadrust::entities::EntityCommon;
}

impl CommonLayer for Line {
    fn common_mut(&mut self) -> &mut acadrust::entities::EntityCommon {
        &mut self.common
    }
}

impl CommonLayer for Arc {
    fn common_mut(&mut self) -> &mut acadrust::entities::EntityCommon {
        &mut self.common
    }
}

impl CommonLayer for Circle {
    fn common_mut(&mut self) -> &mut acadrust::entities::EntityCommon {
        &mut self.common
    }
}

// ── 数据表 ─────────────────────────────────────────────────────────────────

/// 六角头螺栓一行（GB/T 5782 ← ISO 4014）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BoltRow {
    /// 公称直径
    pub d: f64,
    /// 螺距
    #[serde(rename = "P")]
    pub pitch: f64,
    /// 对边宽度
    pub s: f64,
    /// 头高
    pub k: f64,
    /// 对角宽度（最小）
    pub e: f64,
    /// 垫圈面直径
    pub dw: f64,
    /// 螺纹长度（l ≤ 125）
    pub b1: f64,
    /// 螺纹长度（125 < l ≤ 200）
    pub b2: f64,
    /// 螺纹长度（l > 200）
    pub b3: f64,
    pub l_min: f64,
    pub l_max: f64,
}

/// 1 型六角螺母一行（GB/T 6170 ← ISO 4032）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct NutRow {
    pub d: f64,
    #[serde(rename = "P")]
    pub pitch: f64,
    pub s: f64,
    pub e: f64,
    /// 螺母高度
    pub m: f64,
    /// 扳手面高度
    pub mw: f64,
    /// 孔口直径上限
    pub damax: f64,
    pub dw: f64,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct Table<T> {
    #[allow(dead_code)]
    family: String,
    #[allow(dead_code)]
    code: String,
    #[allow(dead_code)]
    iso: String,
    rows: Vec<T>,
}

fn hex_bolt_table() -> &'static Table<BoltRow> {
    static T: std::sync::OnceLock<Table<BoltRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("tables/partsHexBolt.json")).expect("partsHexBolt.json")
    })
}

fn hex_nut_table() -> &'static Table<NutRow> {
    static T: std::sync::OnceLock<Table<NutRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("tables/partsHexNut.json")).expect("partsHexNut.json")
    })
}

/// 六角头螺栓可选公称直径。
pub fn bolt_diameters() -> Vec<f64> {
    hex_bolt_table().rows.iter().map(|r| r.d).collect()
}

/// 六角螺母可选公称直径。
pub fn nut_diameters() -> Vec<f64> {
    hex_nut_table().rows.iter().map(|r| r.d).collect()
}

pub fn bolt_row(d: f64) -> Option<&'static BoltRow> {
    hex_bolt_table().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

pub fn nut_row(d: f64) -> Option<&'static NutRow> {
    hex_nut_table().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

/// 标准长度系列（简化规则：≤20 步长 2 / ≤70 步长 5 / ≤160 步长 10 / 其余步长 20），
/// 裁剪到该直径的 l_min…l_max。
pub fn standard_lengths(row: &BoltRow) -> Vec<f64> {
    // 步长按"当前值所在区间"取，保证 70 的下一档是 80（而不是 75），与 ISO/GB 的首选系列一致。
    let step = |l: f64| if l < 20.0 { 2.0 } else if l < 70.0 { 5.0 } else if l < 160.0 { 10.0 } else { 20.0 };
    let mut out = Vec::new();
    let mut l = row.l_min;
    while l <= row.l_max + 1e-9 {
        out.push(l);
        l += step(l);
    }
    if out.last().map(|x| (*x - row.l_max).abs() > 1e-9).unwrap_or(false) {
        out.push(row.l_max);
    }
    out
}

/// 螺纹长度（GB/T 5782 规则：l≤125 → b1；125<l≤200 → b2；l>200 → b3）。
pub fn thread_length(row: &BoltRow, l: f64) -> f64 {
    if l <= 125.0 {
        row.b1
    } else if l <= 200.0 {
        row.b2
    } else {
        row.b3
    }
}

// ── 生成结果 ───────────────────────────────────────────────────────────────

/// 零件的 BOM 元数据（写入 `OCSM_PART`；明细表/序号直接取用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartMeta {
    /// 现行代号，如 `GB/T 5782-2016`
    pub code: String,
    /// 名称，如 `六角头螺栓`
    pub name: String,
    /// 规格，如 `M10x40`
    pub spec: String,
    /// 材料（默认空，由数据表补）
    pub material: String,
    /// 单件重量 kg（估算值；不写 `≈`，用户 2026-09-15）
    pub weight: String,
}

/// 一次生成的插入方案。
#[derive(Debug, Clone)]
pub struct GenPart {
    /// 图元（已落在 OCSM 图层上，ByLayer）
    pub entities: Vec<EntityType>,
    pub meta: PartMeta,
    /// 包围盒 [xmin, ymin, xmax, ymax]
    pub bbox: [f64; 4],
}

/// 规格文本：螺栓 `M10x40`，螺母 `M10`。
pub fn spec_text_bolt(d: f64, l: f64) -> String {
    format!("M{}x{}", trim_num(d), trim_num(l))
}

pub fn spec_text_dia(d: f64) -> String {
    format!("M{}", trim_num(d))
}

fn trim_num(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        let s = format!("{v:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// 六角头螺栓（`GB/T 5782-2016`；`full_thread=true` 时按全螺纹 `GB/T 5783` 画）。
pub fn hex_bolt(d: f64, l: f64, full_thread: bool) -> Result<GenPart, String> {
    let Some(row) = bolt_row(d) else {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.no_m",
            &[("table", "GB/T 5782"), ("d", &trim_num(d))],
        ));
    };
    if l < row.l_min - 1e-9 || l > row.l_max + 1e-9 {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.len_range_std",
            &[
                ("d", &trim_num(d)),
                ("lo", &trim_num(row.l_min)),
                ("hi", &trim_num(row.l_max)),
            ],
        ));
    }
    let (s, k) = (row.s, row.k);
    let d1 = minor_dia(d, row.pitch);
    let b = if full_thread { l } else { thread_length(row, l) };
    let end = (d * 0.1).max(0.5); // 杆端倒角

    let mut entities = Vec::new();
    // 头部（端面在 x=-k，支承面在 y 轴）——按库内 ACM 件实测画法：
    // 外形宽 = 对边 s；端面竖线跨 s；上下轮廓从 x=-k+R(1-cos20.3°) 起；
    // 倒角弧 圆心 (-k+R, ±0.25s)、半径 R=s×0.72、张角 ±20.3°（两弧在轴线相接）。
    let (rr, half) = (s * CHAMFER_R, CHAMFER_HALF_DEG);
    let x_out = -k + rr * (1.0 - half.to_radians().cos());
    entities.push(line([x_out, s / 2.0], [0.0, s / 2.0], LAYER_MAIN));
    entities.push(line([x_out, -s / 2.0], [0.0, -s / 2.0], LAYER_MAIN));
    entities.push(line([-k, s / 2.0], [-k, -s / 2.0], LAYER_MAIN));
    entities.push(arc([-k + rr, s * 0.25], rr, 180.0 - half, 180.0 + half, LAYER_MAIN));
    entities.push(arc([-k + rr, -s * 0.25], rr, 180.0 - half, 180.0 + half, LAYER_MAIN));
    // 头部中棱线（两棱面交线的投影）
    entities.push(line([x_out, 0.0], [0.0, 0.0], LAYER_MAIN));
    // 头/杆交界
    entities.push(line([0.0, s / 2.0], [0.0, -s / 2.0], LAYER_MAIN));
    entities.push(line([0.0, d / 2.0], [0.0, -d / 2.0], LAYER_MAIN));
    // 杆（光杆段轮廓 + 杆端倒角 + 端面）
    entities.push(line([0.0, d / 2.0], [l, d / 2.0], LAYER_MAIN));
    entities.push(line([0.0, -d / 2.0], [l, -d / 2.0], LAYER_MAIN));
    entities.push(line([l - end, d / 2.0], [l, d / 2.0 - end], LAYER_MAIN));
    entities.push(line([l - end, -d / 2.0], [l, -d / 2.0 + end], LAYER_MAIN));
    entities.push(line([l, d / 2.0 - end], [l, -d / 2.0 + end], LAYER_MAIN));
    // 螺纹：牙底细实线（GB/T 4459.1 外螺纹牙底）
    entities.push(line([l - b, d1 / 2.0], [l, d1 / 2.0], LAYER_THIN));
    entities.push(line([l - b, -d1 / 2.0], [l, -d1 / 2.0], LAYER_THIN));
    // 螺纹收尾斜线（细实线，斜接到大径）
    let runout = row.pitch * 1.5;
    if !full_thread && l - b - runout > 0.0 {
        entities.push(line([l - b - runout, d / 2.0], [l - b, d1 / 2.0], LAYER_THIN));
        entities.push(line([l - b - runout, -d / 2.0], [l - b, -d1 / 2.0], LAYER_THIN));
    }
    if !full_thread {
        // 螺纹终止线（粗实线）
        entities.push(line([l - b, d / 2.0], [l - b, -d / 2.0], LAYER_MAIN));
    }
    // 中心线（两端各出头 ~2d）
    let over = (d * 0.5).max(1.0);
    entities.push(line([-k - over, 0.0], [l + over, 0.0], LAYER_CENTER));

    let (code, name) = if full_thread {
        ("GB/T 5783-2016", "六角头螺栓 全螺纹")
    } else {
        ("GB/T 5782-2016", "六角头螺栓")
    };
    Ok(GenPart {
        entities,
        meta: PartMeta {
            code: code.to_string(),
            name: name.to_string(),
            spec: spec_text_bolt(d, l),
            material: String::new(),
            weight: format!("{:.3}", bolt_weight_kg(row, l)),
        },
        bbox: [-k, -s / 2.0, l, s / 2.0],
    })
}

/// 1 型六角螺母（`GB/T 6170-2015`）主视图：底面 y=0、顶面 y=m，对边宽 s。
pub fn hex_nut(d: f64) -> Result<GenPart, String> {
    let Some(row) = nut_row(d) else {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.no_m",
            &[("table", "GB/T 6170"), ("d", &trim_num(d))],
        ));
    };
    let (s, m) = (row.s, row.m);
    let d1 = minor_dia(d, row.pitch);

    let mut entities = Vec::new();
    // 外形
    entities.push(line([-s / 2.0, 0.0], [s / 2.0, 0.0], LAYER_MAIN));
    entities.push(line([-s / 2.0, m], [s / 2.0, m], LAYER_MAIN));
    entities.push(line([-s / 2.0, 0.0], [-s / 2.0, m], LAYER_MAIN));
    entities.push(line([s / 2.0, 0.0], [s / 2.0, m], LAYER_MAIN));
    // 四角 30° 倒角
    // 顶/底面的倒角弧（构造同螺栓头，旋转 90°）：圆心 (±0.25s, y0∓R)、半径 R=0.72s、张角 ±20.3°
    let (rr, half) = (s * CHAMFER_R, CHAMFER_HALF_DEG);
    let y_in_top = m - rr * (1.0 - half.to_radians().cos());
    let y_in_bot = rr * (1.0 - half.to_radians().cos());
    entities.push(arc([s * 0.25, m - rr], rr, 90.0 - half, 90.0 + half, LAYER_MAIN));
    entities.push(arc([-s * 0.25, m - rr], rr, 90.0 - half, 90.0 + half, LAYER_MAIN));
    entities.push(arc([s * 0.25, rr], rr, -90.0 - half, -90.0 + half, LAYER_MAIN));
    entities.push(arc([-s * 0.25, rr], rr, -90.0 - half, -90.0 + half, LAYER_MAIN));
    let _ = (y_in_top, y_in_bot);
    // 内螺纹：牙底（小径）粗实线、牙顶（大径）细实线
    entities.push(line([-d1 / 2.0, 0.0], [-d1 / 2.0, m], LAYER_MAIN));
    entities.push(line([d1 / 2.0, 0.0], [d1 / 2.0, m], LAYER_MAIN));
    entities.push(line([-d / 2.0, 0.0], [-d / 2.0, m], LAYER_THIN));
    entities.push(line([d / 2.0, 0.0], [d / 2.0, m], LAYER_THIN));
    // 中心线（竖）
    let over = (d * 0.5).max(1.0);
    entities.push(line([0.0, -over], [0.0, m + over], LAYER_CENTER));

    Ok(GenPart {
        entities,
        meta: PartMeta {
            code: "GB/T 6170-2015".to_string(),
            name: crate::i18n::t_data("1型六角螺母").to_string(),
            spec: spec_text_dia(d),
            material: String::new(),
            weight: format!("{:.3}", nut_weight_kg(row)),
        },
        bbox: [-s / 2.0, 0.0, s / 2.0, m],
    })
}

/// 单件重量估算（钢 7.85 g/cm³；螺栓 = 头 + 杆 − 螺纹牙底修正）。
pub fn bolt_weight_kg(row: &BoltRow, l: f64) -> f64 {
    let head = COS30 * row.s * row.s * row.k; // 六角棱柱体积
    let shank = std::f64::consts::PI / 4.0 * row.d * row.d * (l - row.k).max(0.0);
    (head + shank) * 7.85e-3 / 1000.0
}

/// 单件重量估算（钢）。
pub fn nut_weight_kg(row: &NutRow) -> f64 {
    let hex = COS30 * row.s * row.s * row.m;
    let hole = std::f64::consts::PI / 4.0 * minor_dia(row.d, row.pitch).powi(2) * row.m;
    (hex - hole).max(0.0) * 7.85e-3 / 1000.0
}

// ── 六角头螺栓 C 级（GB/T 5780-2016）─────────────────────────────────────────
//
// 画法由用户提供的三视图 DXF 反解（M5x25 逐条对齐），三条规则把全部圆弧唯一确定：
//   ① 倒角水平投影  Δ = (e − s)/2 · tan30°
//   ② 主视图（对角朝前，e 宽）：角弧半径 r= (e/8)/sin(θ/2 的两倍)…实际取 θ=2·atan(8Δ/e)、
//      r=(e/8)/sin θ；端面倒角大弧 r1 = u+Δ（u=((e/4)²−Δ²)/(2Δ)），张角 ±atan((e/4)/u)
//   ③ 俯视图（对边朝前，s 宽）：弧 r=(s/4)/sin θ、θ=2·atan(4Δ/s)，弧心 (−k+r, ±s/4)
// 螺纹按 GB/T 4459.1 简化画法：牙底 = 0.85d（细实线），收尾斜线长度取标准表 runout。

/// 螺栓视图。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoltView {
    /// 主视图：轴线水平、六角头对角朝前（外形宽 = 对角 e）
    Main,
    /// 俯视图：轴线水平、六角头对边朝前（外形宽 = 对边 s）
    Top,
    /// 左视图：端视图（六边形 + 端面倒角圆）
    End,
}

/// 六角头螺栓一行（GB/T 5780 C级 与 GB/T 5782 A/B级 共用字段）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BoltCRow {
    pub d: f64,
    #[serde(rename = "P")]
    pub pitch: f64,
    pub s: f64,
    pub k: f64,
    /// 对角宽度（公差下限；**画图一律用公称 e = s/cos30°**，此列只记录）
    pub e: f64,
    pub b1: f64,
    pub b2: f64,
    pub b3: f64,
    pub l_min: f64,
    pub l_max: f64,
    /// 螺纹收尾（退刀）段长度
    pub runout: f64,
    /// 该直径的标准长度系列
    pub lengths: Vec<f64>,
    /// A/B 级头部垫圈面直径（只有 GB/T 5782 表有；C 级无此特征 —— 仅记录不画）
    #[serde(default)]
    pub dw: f64,
    /// A/B 级头部垫圈面高度（仅记录不画）
    #[serde(default)]
    pub c: f64,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct BoltCTable {
    #[allow(dead_code)]
    family: String,
    #[allow(dead_code)]
    grade: String,
    #[allow(dead_code)]
    code: String,
    #[allow(dead_code)]
    iso: String,
    rows: Vec<BoltCRow>,
}

fn hex_bolt_c_table() -> &'static BoltCTable {
    static T: std::sync::OnceLock<BoltCTable> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("tables/partsHexBoltC.json")).expect("partsHexBoltC.json")
    })
}

/// C 级螺栓可选的公称直径。
pub fn hex_bolt_c_diameters() -> Vec<f64> {
    hex_bolt_c_table().rows.iter().map(|r| r.d).collect()
}

/// 某直径的标准长度系列。
pub fn hex_bolt_c_lengths(d: f64) -> Vec<f64> {
    hex_bolt_c_table()
        .rows
        .iter()
        .find(|r| (r.d - d).abs() < 1e-9)
        .map(|r| r.lengths.clone())
        .unwrap_or_default()
}

pub fn hex_bolt_c_row(d: f64) -> Option<&'static BoltCRow> {
    hex_bolt_c_table().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

// ── GB/T 5782-2016 六角头螺栓 A/B级（部分螺纹）────────────────────────────
//
// **画法与 GB/T 5780 C级完全相同**（用户 2026-09-15：「A/B 级与 C 级肉眼看不出来区别，直接复制过来即可」）。
// 两族表的 s/k 逐规格一致（所以头部画法一致），差异只在：
// - `e`（公差下限，仅记录不画 —— 几何一律用公称 e = s/cos30°）；
// - 可用长度区间（如 M10：5782 = 45~100、5780 C级 = 40~100；M36：140~360 vs 110~360）；
// - A/B 级标准图在头部支承面有垫圈面 dw×c（C 级无）—— 本库按用户指示不画，dw/c 仅入表备查。

fn hex_bolt_ab_table() -> &'static BoltCTable {
    static T: std::sync::OnceLock<BoltCTable> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("tables/partsHexBolt5782.json"))
            .expect("partsHexBolt5782.json")
    })
}

/// GB/T 5782 可选的公称直径（29 规格 M1.6…M64）。
pub fn hex_bolt_ab_diameters() -> Vec<f64> {
    hex_bolt_ab_table().rows.iter().map(|r| r.d).collect()
}

/// GB/T 5782 某个直径的长度系列。
pub fn hex_bolt_ab_lengths(d: f64) -> Vec<f64> {
    hex_bolt_ab_table()
        .rows
        .iter()
        .find(|r| (r.d - d).abs() < 1e-9)
        .map(|r| r.lengths.clone())
        .unwrap_or_default()
}

pub fn hex_bolt_ab_row(d: f64) -> Option<&'static BoltCRow> {
    hex_bolt_ab_table().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

/// 螺纹长度（GB/T 5780：l ≤ 125 → b1；125 < l ≤ 200 → b2；l > 200 → b3）。
pub fn thread_length_c(row: &BoltCRow, l: f64) -> f64 {
    if l <= 125.0 {
        row.b1
    } else if l <= 200.0 {
        row.b2
    } else {
        row.b3
    }
}

/// 六角头倒角水平投影 Δ = (e − s)/2 · tan30°（用户图实测：M5 0.357、M16 库件 0.716）。
pub fn chamfer_run(s: f64, e: f64) -> f64 {
    (e - s) / 2.0 * 30f64.to_radians().tan()
}

/// 生成 GB/T 5780-2016（C 级）六角头螺栓的一个视图。
///
/// - 基点：主/俯视图 = 头部支承面 × 轴线（原点）；左视图 = 头部中心（原点）
/// - 图元只落 `1轮廓实线层`/`2细线层`/`3中心线层`，颜色/线型/线宽 ByLayer
/// - **不含任何尺寸标注**（用户要求：调用零件时不显示尺寸）
pub fn hex_bolt_c(d: f64, l: f64, view: BoltView) -> Result<GenPart, String> {
    let Some(row) = hex_bolt_c_row(d) else {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.no_m",
            &[("table", "GB/T 5780"), ("d", &trim_num(d))],
        ));
    };
    if l < row.l_min - 1e-9 || l > row.l_max + 1e-9 {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.len_range",
            &[
                ("d", &trim_num(d)),
                ("lo", &trim_num(row.l_min)),
                ("hi", &trim_num(row.l_max)),
            ],
        ));
    }
    let b = thread_length_c(row, l);
    let meta = PartMeta {
        code: "GB/T 5780-2016".to_string(),
        name: crate::i18n::t_data("六角头螺栓 C级").to_string(),
        spec: spec_text_bolt(d, l),
        material: String::new(),
        weight: format!("{:.3}", hex_bolt_weight_kg(row, l)),
    };
    Ok(hex_bolt_views(view, d, l, row.s, row.k, b, row.runout, None, meta))
}

/// GB/T 5782-2016 六角头螺栓 A/B级（部分螺纹）——**与 C 级同一套画法**（用户确认直接复制）。
///
/// 差异全在数据侧：s/k 与 5780 C级逐规格相同、`e` 是公差下限（仅记录）、可用长度区间不同；
/// A/B 级标准图的头部垫圈面 dw×c 未画（按用户指示，dw/c 仅入表备查）。
pub fn hex_bolt_ab(d: f64, l: f64, view: BoltView) -> Result<GenPart, String> {
    let Some(row) = hex_bolt_ab_row(d) else {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.no_m",
            &[("table", "GB/T 5782"), ("d", &trim_num(d))],
        ));
    };
    if l < row.l_min - 1e-9 || l > row.l_max + 1e-9 {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.len_range_ab",
            &[
                ("d", &trim_num(d)),
                ("lo", &trim_num(row.l_min)),
                ("hi", &trim_num(row.l_max)),
            ],
        ));
    }
    let b = thread_length_c(row, l);
    let runout = 5.0 * row.pitch;
    let meta = PartMeta {
        code: "GB/T 5782-2016".to_string(),
        name: crate::i18n::t_data("六角头螺栓 A/B级").to_string(),
        spec: spec_text_bolt(d, l),
        material: String::new(),
        weight: format!("{:.3}", hex_bolt_weight_kg(row, l)),
    };
    // 头部垫圈面 dw×c：**主视图 + 俯视图**都画（用户 2026-09-15 确认），左视图不画
    Ok(hex_bolt_views(
        view,
        d,
        l,
        row.s,
        row.k,
        b,
        runout,
        Some((row.dw, row.c)),
        meta,
    ))
}

/// 六角头螺栓三视图的**画法本体**（GB/T 5780 C级 与 GB/T 5782 A/B级共用）。
///
/// 传入已解出的对边宽 `s`、头高 `k`、螺纹长 `b`、收尾 `runout`、元数据，
/// 以及**头部垫圈面** `washer_face`（A/B 级特有：`Some((dw, c))`；C 级传 `None`）。
///
/// 垫圈面画法（标准 CAD 图实测，M10 例）：
/// - 它是支承面上一个高 `c`、直径 `dw` 的凸台（垫圈面在最低处，周围是内缩的六角体支承面）；
/// - **头高 `k` 从垫圈面量到顶面**（总高 = k，六角体高 = k − c）；
/// - 主视图：垫圈面竖线 `(0, ±dw/2)` + 凸台母线 `(0,±dw/2)→(−c,±dw/2)`；
///   六角体支承面横线由 `x=0` 改到 `x=−c`（仍跨 ±e/2），头其余几何不动（顶面仍在 `x=−k`）；
/// - 端视图：多一个垫圈面圆 `r=dw/2`（在倒角圆 r=s/2 内部）；附、俯视图不变。
fn hex_bolt_views(
    view: BoltView,
    d: f64,
    l: f64,
    s: f64,
    k: f64,
    b: f64,
    runout: f64,
    washer_face: Option<(f64, f64)>,
    meta: PartMeta,
) -> GenPart {
    // 几何用**公称对角宽** e = s/cos30°（表里的 e 是公差下限：M5 8.63 vs 公称 9.2376，
    // 用户图实测轮廓在 ±4.619 = ±e_公称/2）
    let e = across_corners(s);
    let dm = 0.85 * d; // GB/T 4459.1 简化画法：牙底 ≈ 0.85d
    let delta = chamfer_run(s, e);
    let x_out = -k + delta;
    // 六角体支承面（轴承面）位置：无垫圈面时 = 基点 x=0；有垫圈面时内缩到 x=−c
    let (xj, wf) = match washer_face {
        Some((dw, c)) => (-c, Some((dw, c))),
        None => (0.0, None),
    };
    // 中心线出头固定 3.0（实测：5780 M5 = 3.0、5783 M10 = 3.0、32.1 M18 = 3.0）
    let over = 3.0;

    let mut entities = Vec::new();
    match view {
        BoltView::Main => {
            // 垫圈面（A/B 级）：凸台母线 + 垫圈面线，位于头的最下方（基点 x=0）
            if let Some((dw, c)) = wf {
                entities.push(line([-c, dw / 2.0], [0.0, dw / 2.0], LAYER_MAIN));
                entities.push(line([-c, -dw / 2.0], [0.0, -dw / 2.0], LAYER_MAIN));
                entities.push(line([0.0, dw / 2.0], [0.0, -dw / 2.0], LAYER_MAIN));
            }
            // 头：端面（s 宽）、轮廓（e 宽）、斜线、角弧、端面倒角大弧、棱线、交界线
            entities.push(line([-k, s / 2.0], [-k, -s / 2.0], LAYER_MAIN));
            entities.push(line([x_out, e / 2.0], [xj, e / 2.0], LAYER_MAIN));
            entities.push(line([x_out, -e / 2.0], [xj, -e / 2.0], LAYER_MAIN));
            entities.push(line([-k, s / 2.0], [x_out, e / 2.0], LAYER_MAIN));
            entities.push(line([-k, -s / 2.0], [x_out, -e / 2.0], LAYER_MAIN));
            // 角弧：θ = 2·atan(8Δ/e)，r = (e/8)/sinθ，弧心 (−k+r, ±0.375e)，从 180°−θ 到 180°
            let th = 2.0 * (8.0 * delta / e).atan();
            let r2 = (e / 8.0) / th.sin();
            // 下角弧：棱线 (x_out, −e/4) → 端面 (−k, −0.375e)
            entities.push(arc([-k + r2, -0.375 * e], r2, 180.0 - th.to_degrees(), 180.0, LAYER_MAIN));
            // 上角弧：下角弧关于轴线的**镜像**（端面 (−k, +0.375e) → 棱线/大弧交点 (x_out, +e/4)）
            entities.push(arc([-k + r2, 0.375 * e], r2, 180.0, 180.0 + th.to_degrees(), LAYER_MAIN));
            // 端面倒角大弧：u = ((e/4)²−Δ²)/(2Δ)，r1 = u+Δ，张角 ±atan((e/4)/u)
            let u = ((e / 4.0).powi(2) - delta * delta) / (2.0 * delta);
            let r1 = u + delta;
            let al = (e / 4.0 / u).atan().to_degrees();
            entities.push(arc([-k + r1, 0.0], r1, 180.0 - al, 180.0 + al, LAYER_MAIN));
            // 头内部棱线（六边形侧棱的投影，位于 ±e/4）
            entities.push(line([x_out, e / 4.0], [xj, e / 4.0], LAYER_MAIN));
            entities.push(line([x_out, -e / 4.0], [xj, -e / 4.0], LAYER_MAIN));
            // 六角体支承面（无垫圈面时就是头/杆交界；有垫圈面时是内缩的支承面）
            entities.push(line([xj, e / 2.0], [xj, -e / 2.0], LAYER_MAIN));
        }
        BoltView::Top => {
            // 垫圈面（A/B 级，用户 2026-09-15 确认“主视图和俯视图都按你现在的画法”）：
            // 俯视看下去，凸台是个比头体窄的“小脚”（宽 dw、轴向长 c），位置在支承面之后
            if let Some((dw, c)) = wf {
                entities.push(line([-c, dw / 2.0], [0.0, dw / 2.0], LAYER_MAIN));
                entities.push(line([-c, -dw / 2.0], [0.0, -dw / 2.0], LAYER_MAIN));
                entities.push(line([0.0, dw / 2.0], [0.0, -dw / 2.0], LAYER_MAIN));
            }
            entities.push(line([-k, s / 2.0], [-k, -s / 2.0], LAYER_MAIN));
            // 轮廓线从端面竖线 x=−k 起（用户规范图：穿过倒角弧终点直到底面；有垫圈面时至 x=−c）
            entities.push(line([-k, s / 2.0], [xj, s / 2.0], LAYER_MAIN));
            entities.push(line([-k, -s / 2.0], [xj, -s / 2.0], LAYER_MAIN));
            // 弧：θ = 2·atan(4Δ/s)，r = (s/4)/sinθ，弧心 (−k+r, ±s/4)
            let th = 2.0 * (4.0 * delta / s).atan();
            let r3 = (s / 4.0) / th.sin();
            // 俯视图的弧从"轮廓起点 (x_out, ±s/2)"跨过 (±e/4) 一直画到中棱线端 (x_out, 0)，
            // 即张角对称于 180°（用户图：159.74°→200.26°）
            entities.push(arc([-k + r3, s / 4.0], r3, 180.0 - th.to_degrees(), 180.0 + th.to_degrees(), LAYER_MAIN));
            entities.push(arc([-k + r3, -s / 4.0], r3, 180.0 - th.to_degrees(), 180.0 + th.to_degrees(), LAYER_MAIN));
            // 头内部中棱线
            entities.push(line([x_out, 0.0], [xj, 0.0], LAYER_MAIN));
            // 六角体支承面（无垫圈面时就是头/杆交界）
            entities.push(line([xj, s / 2.0], [xj, -s / 2.0], LAYER_MAIN));
        }
        BoltView::End => {
            // 六边形：上下顶点 ±e/2，左右对边 ±s/2（顶点在 (±s/2, ±e/4)）
            // 注：**垫圈面圆不画**（用户 2026-09-15：“主视图和俯视图按你的画法，左视图不改”）
            let v = [
                [0.0, e / 2.0],
                [s / 2.0, e / 4.0],
                [s / 2.0, -e / 4.0],
                [0.0, -e / 2.0],
                [-s / 2.0, -e / 4.0],
                [-s / 2.0, e / 4.0],
            ];
            for i in 0..6 {
                entities.push(line(v[i], v[(i + 1) % 6], LAYER_MAIN));
            }
            // 端面倒角圆（直径 = 对边 s）
            entities.push(circle([0.0, 0.0], s / 2.0, LAYER_MAIN));
            // 十字中心线：半长 = s/2 + 3（实测 5780 M5 = 7.0、5783 M10 = 11.0）
            let o = s / 2.0 + 3.0;
            entities.push(line([-o, 0.0], [o, 0.0], LAYER_CENTER));
            entities.push(line([0.0, o], [0.0, -o], LAYER_CENTER));
            return GenPart { entities, meta, bbox: [-s / 2.0, -e / 2.0, s / 2.0, e / 2.0] };
        }
    }

    // 杆 + 螺纹（主/俯视图共用）
    let c = 0.075 * d; // 杆端倒角
    entities.push(line([0.0, d / 2.0], [l - c, d / 2.0], LAYER_MAIN));
    entities.push(line([0.0, -d / 2.0], [l - c, -d / 2.0], LAYER_MAIN));
    entities.push(line([l - c, d / 2.0], [l, dm / 2.0], LAYER_MAIN));
    entities.push(line([l - c, -d / 2.0], [l, -dm / 2.0], LAYER_MAIN));
    entities.push(line([l, dm / 2.0], [l, -dm / 2.0], LAYER_MAIN));
    // 杆端倒角起点竖线 + 螺纹起点竖线（用户规范图里两条都在，跨满杆径）
    entities.push(line([l - c, d / 2.0], [l - c, -d / 2.0], LAYER_MAIN));
    entities.push(line([l - b, d / 2.0], [l - b, -d / 2.0], LAYER_MAIN));
    // 牙底细实线 + 收尾斜线
    entities.push(line([l - b, dm / 2.0], [l, dm / 2.0], LAYER_THIN));
    entities.push(line([l - b, -dm / 2.0], [l, -dm / 2.0], LAYER_THIN));
    entities.push(line([l - b - runout, d / 2.0], [l - b, dm / 2.0], LAYER_THIN));
    entities.push(line([l - b - runout, -d / 2.0], [l - b, -dm / 2.0], LAYER_THIN));
    // 中心线
    entities.push(line([-k - over, 0.0], [l + over, 0.0], LAYER_CENTER));

    let half_h = if view == BoltView::Main { e / 2.0 } else { s / 2.0 };
    GenPart { entities, meta, bbox: [-k, -half_h, l, half_h] }
}

/// 六角头螺栓单件重量估算（钢 7.85 g/cm³，六角头按正六边形面积 + 杆体积）。
pub fn hex_bolt_weight_kg(row: &BoltCRow, l: f64) -> f64 {
    let head = COS30 * row.s * row.s * row.k;
    let shank = std::f64::consts::PI / 4.0 * row.d * row.d * (l - row.k).max(0.0);
    (head + shank) * 7.85e-3 / 1000.0
}

// ── 选择器页接口（目录 JSON / 预览 SVG / 生成派发）─────────────────────────

/// 参数化零件目录（选择器页用）：文件树 + 各族可用规格。
///
/// 树形路径与用户约定一致：`零件库/螺栓/六角螺栓/六角头螺栓 C级 GB/T 5780-2016`。
/// 未实现的常用族也列在树里（`implemented:false`），便于按图索骥。
/// 块名清洗（零件库插入与件链装配共用）：只留 ASCII 字母数字和 `-`，
/// 规格里的 `x`/`×`/`Ø` 当分隔符（族名的 x 保留——`hex` 的 x 不能切）。
pub fn ascii_block(family: &str, spec: &str) -> String {
    fn clean(text: &str, separator_chars: &[char], keep_case: bool) -> String {
        let mut out = String::new();
        for ch in text.chars() {
            if (ch.is_ascii_alphanumeric() || ch == '-') && !separator_chars.contains(&ch) {
                out.push(if keep_case { ch } else { ch.to_ascii_uppercase() });
            } else if !out.ends_with('_') {
                out.push('_');
            }
        }
        out.trim_matches('_').to_string()
    }
    match spec.is_empty() {
        true => clean(family, &[], false),
        false => format!("{}_{}", clean(family, &[], false), clean(spec, &['x', 'X', '×', 'Ø', 'ø'], false)),
    }
}

pub fn catalog_json() -> String {
    // ── 已实现族的规格表
    let mut sizes = Vec::new();
    for row in hex_bolt_c_table().rows.iter() {
        sizes.push(serde_json::json!({
            "d": row.d,
            "label": format!("M{}", trim_num(row.d)),
            "pitch": row.pitch,
            "l_min": row.l_min,
            "l_max": row.l_max,
            "lengths": row.lengths,
        }));
    }
    let views = serde_json::json!([
        { "id": "main", "name": "主视图" },
        { "id": "top",  "name": "俯视图" },
        { "id": "end",  "name": "左视图" },
    ]);
    // ── 文件树（path 的每一层；叶子 = 规格表）
    let mut fam_map = serde_json::Map::new();
    fam_map.insert(
        "hex_bolt_c".to_string(),
        serde_json::json!({
            "id": "hex_bolt_c",
            "name": "六角头螺栓 C级",
            "code": "GB/T 5780-2016",
            "iso": "ISO 4016:2011",
            "implemented": true,
            "views": views,
            "sizes": sizes,
            "len_label": "长度 l",
            "base_hint": "基点 = 头部支承面 × 轴线",
        }),
    );
    // GB/T 5782-2016 A/B 级：**画法与 C 级完全相同**（用户确认直接复制），差异只在数据表
    let sizes_ab: Vec<serde_json::Value> = hex_bolt_ab_table()
        .rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "d": r.d,
                "label": format!("M{}", trim_num(r.d)),
                "pitch": r.pitch,
                "l_min": r.l_min,
                "l_max": r.l_max,
                "lengths": r.lengths,
                "extra": format!("s={} k={} e={}（A级）dw={} c={}", trim_num(r.s), trim_num(r.k), trim_num(r.e), trim_num(r.dw), trim_num(r.c)),
            })
        })
        .collect();
    let views_ab = serde_json::json!([
        { "id": "main", "name": "主视图" },
        { "id": "top",  "name": "俯视图" },
        { "id": "end",  "name": "左视图" },
    ]);
    fam_map.insert(
        "hex_bolt_ab".to_string(),
        serde_json::json!({
            "id": "hex_bolt_ab",
            "name": "六角头螺栓 A/B级",
            "code": "GB/T 5782-2016",
            "iso": "ISO 4014:2011",
            "implemented": true,
            "views": views_ab,
            "sizes": sizes_ab,
            "len_label": "长度 l",
            "base_hint": "基点 = 头部支承面 × 轴线（画法同 GB/T 5780 C级）",
        }),
    );
    for (k, v) in crate::partgen_more::families_json() {
        fam_map.insert(k, v);
    }
    // 第四批起：各族模块**自报家门**（id/name/code/views/sizes/tree_path），
    // 本文件不再逐个登记——并行分支各写各的模块，接进来自动出现。
    // 结构要素（磨外圆等）走同一套：`detail::families_json` 用 `tree_dir` 声明
    // 「结构要素/…」路径，会在文件树上自动长出与「零件库」并列的根树。
    for (k, v) in crate::partgen_b1::families_json()
        .into_iter()
        .chain(crate::partgen_b2::families_json())
        .chain(crate::partgen_b3::families_json())
        .chain(crate::partgen_b4::families_json())
        .chain(crate::partgen_b5::families_json())
        .chain(crate::partgen_keys::families_json())
        .chain(crate::detail::families_json())
    {
        fam_map.insert(k, v);
    }
    // 轴槽段类型显示用：给平键族的 `shaft_ranges` 每行补 `t1`（按 b 查 GB/T 1095 表；
    // 唯一数据源 = shaft.rs 的 `assets/keyway_gb1095.csv`），避免 GUI 另写一份 t1 表。
    for id in ["key_1096_a", "key_1096_b", "key_1096_c"] {
        if let Some(ranges) = fam_map
            .get_mut(id)
            .and_then(|v| v.as_object_mut())
            .and_then(|o| o.get_mut("shaft_ranges"))
            .and_then(|v| v.as_array_mut())
        {
            for row in ranges.iter_mut() {
                if let Some(obj) = row.as_object_mut() {
                    if let Some(b) = obj.get("b").and_then(|v| v.as_f64()) {
                        if let Ok(t1) = crate::shaft::keyway_t1(b, None) {
                            obj.insert("t1".to_string(), serde_json::json!(t1));
                        }
                    }
                }
            }
        }
    }
    // 每族补一个 kind 字段（bolt/nut/washer/pin/other）：GUI 与 AI 都靠它分组，
    // 不用去猜族名前缀（`family_kind` 是唯一判据来源）。
    // ★ ⑥ 批：目录 JSON 的显示元数据随语言（族名/基点/长度标签/树目录/来源…）。
    // 在**合并后、建树前**统一取词，保证树叶子名与 families 一致（不留半截中文）。
    let families = crate::i18n::translate_json(&{
        let mut obj = fam_map;
        for (key, value) in obj.iter_mut() {
            let kind = family_kind(key);
            if let Some(map) = value.as_object_mut() {
                map.insert("kind".to_string(), serde_json::json!(kind));
            }
        }
        serde_json::Value::Object(obj)
    });
    let tree = crate::i18n::translate_json(&serde_json::json!([
        { "name": "零件库", "children": [
            { "name": "螺栓", "children": [
                { "name": "六角螺栓", "children": [
                    { "name": "六角头螺栓 C级 GB/T 5780-2016", "family": "hex_bolt_c", "implemented": true },
                    { "name": "六角头螺栓 A/B级 GB/T 5782-2016", "family": "hex_bolt_ab", "implemented": true },
                    { "name": "六角头螺栓 全螺纹 GB/T 5783-2016", "family": "hex_bolt_b_full", "implemented": true },
                    { "name": "六角头头部带孔螺栓 GB/T 32.1-2020", "family": "hex_bolt_hole_a", "implemented": true }
                ]}
            ]},
            { "name": "螺钉", "children": [
                { "name": "内六角", "children": [
                    { "name": "内六角圆柱头螺钉 GB/T 70.1-2008", "family": "socket_head", "implemented": true }
                ]}
            ]},
            { "name": "螺母", "children": [
                { "name": "六角螺母", "children": [
                    { "name": "六角螺母 C级 GB/T 41-2016", "family": "nut_c41", "implemented": true }
                ]},
                { "name": "六角薄螺母", "children": [
                    { "name": "六角薄螺母 GB/T 6172.1-2016", "family": "nut_61721", "implemented": true }
                ]}
            ]},
            { "name": "垫圈", "children": [
                { "name": "平垫圈", "children": [
                    { "name": "平垫圈 A级 GB/T 97.1-2002", "family": "washer_971", "implemented": true }
                ]},
                { "name": "弹性垫圈", "children": [
                    { "name": "标准型弹簧垫圈 GB/T 93-2025", "family": "washer_93", "implemented": true }
                ]}
            ]},
            { "name": "销", "children": [
                { "name": "圆柱销", "children": [
                    { "name": "圆柱销 A型 GB/T 119.1-2000", "family": "pin_1191", "implemented": true },
                    { "name": "内螺纹圆柱销 GB/T 120.1-2000", "family": "pin_1201", "implemented": true }
                ]}
            ]}
        ]}
    ]));

    // 树叶子动态补位：新批次的族在自己的 `families_json` 里声明**目录**，叶子名由代码统一拼
    // （`族名 + 代号`，保证全库标签一致）：
    //   `tree_dir = "零件库/螺钉/紧定螺钉"` → 叶子「内六角平端紧定螺钉 GB/T 77-2007」
    // 目录路径用 `/` 分隔是安全的（目录名里不会出现代号）。
    //
    // 另一种写法 `tree_path`（整条路径）**必须用 `>` 分隔**：`零件库 > 螺钉 > 内六角平端紧定螺钉 GB/T 77-2007`
    // —— 不能用 `/`，否则代号里的 `GB/T` 会被当成路径分隔符切成「…螺钉 GB」「T 77-2007」（踩过）。
    //
    // 注意：**螺钉不是螺栓**（用户 2026-09-16 明确）——螺钉自带头部、直接拧入螺纹孔、不配螺母，
    // 所以树上单独一类「螺钉」；紧定螺钉（GB/T 77）与吊环螺钉（GB/T 825）也走这一支。
    let mut tree = tree;
    let leaves: Vec<(String, String, bool)> = families
        .as_object()
        .map(|obj| {
            obj.iter()
                .filter_map(|(key, val)| {
                    let implemented = val.get("implemented") != Some(&serde_json::json!(false));
                    let name = val.get("name").and_then(|v| v.as_str()).unwrap_or(key);
                    let code = val.get("code").and_then(|v| v.as_str()).unwrap_or("");
                    let leaf = format!("{name} {code}").trim().to_string();
                    let path = if let Some(dir) = val.get("tree_dir").and_then(|v| v.as_str()) {
                        format!("{}/{}", dir.trim_end_matches('/'), leaf)
                    } else {
                        val.get("tree_path")?.as_str()?.replace('>', "/")
                    };
                    Some((path, key.clone(), implemented))
                })
                .collect()
        })
        .unwrap_or_default();
    for (path, family, implemented) in leaves {
        insert_tree_leaf(&mut tree, &path, &family, implemented);
    }
    // 渐开线花键引擎数据（预设/DIN·NF 名义表/检验表/模数候选）：齿轮生成器与轴生成器共用。
    // 它是**引擎数据**、不是上架族——XL 结构要素里的渐开线花键入口已移除，避免两个生成入口。
    serde_json::json!({
        "tree": tree,
        "families": families,
        "spline_engine": crate::invol_spline::catalog_payload(),
        // 轴生成器键槽段的键型下拉元数据（表驱动；加新键型只需 partgen_keys::KEY_STYLES 加一行）。
        "key_styles": crate::i18n::translate_json(&crate::partgen_keys::key_styles_json()),
    })
    .to_string()
}

/// 把 `/`（或 `>`）分隔的路径切成段。**代号里的斜杠不能当分隔符**：
///
/// `GB/T`、`JB/T` 这类代号自带斜杠，直接 `split('/')` 会把「内六角平端紧定螺钉 GB/T 77-2007」
/// 切成「…螺钉 GB」+「T 77-2007」（目录与叶子全错）。这里做**代号回接**：
/// 若上一段以标准代号前缀（GB/JB/ISO/DIN/…）结尾，就把下一段用 `/` 拼回去。
fn split_tree_path(path: &str) -> Vec<String> {
    const CODE_PREFIX: [&str; 12] = [
        "GB", "JB", "Q", "ASME", "ISO", "DIN", "ANSI", "UNI", "CNS", "GOST", "HB", "QJ",
    ];
    let raw = path
        .replace('>', "/")
        .split('/')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let mut out: Vec<String> = Vec::new();
    for seg in raw {
        let merge = out
            .last()
            .map(|prev: &String| {
                let tail = prev.rsplit([' ', '　']).next().unwrap_or(prev.as_str());
                CODE_PREFIX.contains(&tail)
            })
            .unwrap_or(false);
        if merge {
            let prev = out.last_mut().expect("非空");
            prev.push('/');
            prev.push_str(&seg);
        } else {
            out.push(seg);
        }
    }
    out
}

/// 把族插到零件树的指定路径（`零件库/挡圈/孔用弹性挡圈 A型 GB/T 893-2017`）。
///
/// 缺的中间目录自动创建；同名族已存在则幂等跳过。
fn insert_tree_leaf(tree: &mut serde_json::Value, path: &str, family: &str, implemented: bool) {
    let segs = split_tree_path(path);
    if !segs.is_empty() {
        let refs: Vec<&str> = segs.iter().map(|s| s.as_str()).collect();
        insert_leaf_rec(tree, &refs, family, implemented);
    }
}

fn tree_children_mut(node: &mut serde_json::Value) -> Option<&mut Vec<serde_json::Value>> {
    if node.is_array() {
        node.as_array_mut()
    } else {
        node.get_mut("children").and_then(|c| c.as_array_mut())
    }
}

fn insert_leaf_rec(node: &mut serde_json::Value, segs: &[&str], family: &str, implemented: bool) {
    let Some(children) = tree_children_mut(node) else {
        return;
    };
    if segs.len() == 1 {
        if children
            .iter()
            .any(|c| c.get("family").and_then(|f| f.as_str()) == Some(family))
        {
            return;
        }
        children.push(
            serde_json::json!({ "name": segs[0], "family": family, "implemented": implemented }),
        );
        return;
    }
    let idx = match children
        .iter()
        .position(|c| c.get("name").and_then(|n| n.as_str()) == Some(segs[0]))
    {
        Some(i) => i,
        None => {
            children.push(serde_json::json!({ "name": segs[0], "children": [] }));
            children.len() - 1
        }
    };
    insert_leaf_rec(&mut children[idx], &segs[1..], family, implemented);
}

/// 族的规模行（直径/螺距/长度范围/供货长度系列）——从同一份 catalog JSON 取，不另建数据源。
#[derive(Debug, Clone, PartialEq)]
pub struct SizeRow {
    pub d: f64,
    pub pitch: f64,
    pub l_min: f64,
    pub l_max: f64,
    /// 厂家实际可购的供货长度系列（可能略偏离推标公称系列，以表为准）。
    pub lengths: Vec<f64>,
}

/// 族的类型：bolt / nut / washer / pin / detail / other（件链解析用）。
pub fn family_kind(family: &str) -> &'static str {
    if family.starts_with("detail_") {
        // 结构要素（磨外圆/退刀槽/键槽…）：与标准件并列的另一棵树
        // （不进螺栓副/件链，GUI 面板走「自由输入 d」而不是固定规格下拉）
        "detail"
    } else if family.starts_with("hex_bolt") {
        // 真·螺栓：穿孔 + 螺母（件链装配只认这一种）
        "bolt"
    } else if family == "socket_head"
        || family.starts_with("socket_button")
        || family.starts_with("socket_torx")
        || family.starts_with("set_screw")
        || family.starts_with("eye_bolt")
    {
        // 螺钉：自带头部、直接拧入螺纹孔、不配螺母 —— 与螺栓是两类东西
        // （用户 2026-09-16 明确：螺钉从螺栓族里抽出来单独成类；也不进螺栓副装配）
        "screw"
    } else if family.starts_with("nut_") || family.starts_with("round_nut") {
        "nut"
    } else if family.starts_with("washer_") || family.starts_with("lock_washer") {
        "washer"
    } else if family.starts_with("pin_") {
        "pin"
    } else if family.starts_with("key_") {
        // 平键（GB/T 1096/1097）：键不是销、也不是紧固螺纹件，单独一类。
        "key"
    } else if family.starts_with("ring_") {
        "ring"
    } else if family.starts_with("bearing_") {
        "bearing"
    } else if family.starts_with("seal") {
        "seal"
    } else {
        "other"
    }
}

/// 族的显示名与标准号（报告/日志用）。
pub fn family_meta(family: &str) -> (String, String) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&catalog_json()) else {
        return (family.to_string(), String::new());
    };
    let entry = &value["families"][family];
    let name = entry["name"].as_str().unwrap_or(family).to_string();
    let code = entry["code"].as_str().unwrap_or_default().to_string();
    (name, code)
}

/// 族 → 规模行列表。
pub fn family_sizes(family: &str) -> Vec<SizeRow> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&catalog_json()) else {
        return Vec::new();
    };
    let Some(rows) = value["families"][family]["sizes"].as_array() else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            Some(SizeRow {
                d: row["d"].as_f64()?,
                pitch: row["pitch"].as_f64().unwrap_or(0.0),
                l_min: row["l_min"].as_f64().unwrap_or(0.0),
                l_max: row["l_max"].as_f64().unwrap_or(0.0),
                lengths: row["lengths"]
                    .as_array()
                    .map(|list| list.iter().filter_map(|v| v.as_f64()).collect())
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// 族 + 直径 → 规模行（大小写不敏感，1e-9 容差）。
pub fn size_row(family: &str, d: f64) -> Option<SizeRow> {
    family_sizes(family)
        .into_iter()
        .find(|row| (row.d - d).abs() < 1e-9)
}

/// 按族/规格/视图生成零件（预览与插入共用同一入口）。
///
/// 结构要素（`detail_*`）没有长度 l：本入口约定 `l` 槽位承载**可选 b1**
///（`l = 0` → 该 d 档默认行），方便预览/测试用一条统一入口；HTTP/CLI 侧
/// 有显式 `b1` 字段，走 `generate_requested`。外螺纹退刀槽需要 P（不是 b1），
/// 走 `generate_requested_params`（本入口会明确报“需要 P”）。
pub fn generate(family: &str, d: f64, l: f64, view: &str) -> Result<GenPart, String> {
    if let Some(result) = crate::detail::try_generate(family, d, (l > 0.0).then_some(l), view) {
        return result;
    }
    // 第四批起的族：各自模块里带完整实现（画法/数据/校验），先于本文件的历史分支派发
    for f in [
        crate::partgen_b1::generate,
        crate::partgen_b2::generate,
        crate::partgen_b3::generate,
        crate::partgen_b4::generate,
        crate::partgen_b5::generate,
        crate::partgen_keys::generate,
    ] {
        if let Some(r) = f(family, d, l, view) {
            return r;
        }
    }
    if let Some(r) = crate::partgen_more::generate(family, d, l, view) {
        return r;
    }
    match (family, view) {
        ("hex_bolt_c", "main") => hex_bolt_c(d, l, BoltView::Main),
        ("hex_bolt_c", "top") => hex_bolt_c(d, l, BoltView::Top),
        ("hex_bolt_c", "end") => hex_bolt_c(d, l, BoltView::End),
        ("hex_bolt_c", other) => Err(crate::i18n::t_fmt(
            "cmd.parts.err.view_absent_bare",
            &[("family", "hex_bolt_c"), ("view", other)],
        )),
        // GB/T 5782-2016 A/B级：画法同 C 级（用户确认直接复制），数据表不同
        ("hex_bolt_ab", "main") => hex_bolt_ab(d, l, BoltView::Main),
        ("hex_bolt_ab", "top") => hex_bolt_ab(d, l, BoltView::Top),
        ("hex_bolt_ab", "end") => hex_bolt_ab(d, l, BoltView::End),
        ("hex_bolt_ab", other) => Err(crate::i18n::t_fmt(
            "cmd.parts.err.view_absent_bare",
            &[("family", "hex_bolt_ab"), ("view", other)],
        )),
        (other, _) => Err(crate::i18n::t_fmt("cmd.parts.err.family_todo", &[("other", other)])),
    }
}

/// 统一生成入口（HTTP / CLI 调用）：标准件要求 `l`；结构要素不要 `l`，
/// `b1` 可选（`None` = 该 d 档默认行）。
pub fn generate_requested(
    family: &str,
    d: f64,
    l: Option<f64>,
    b1: Option<f64>,
    view: &str,
) -> Result<GenPart, String> {
    generate_requested_params(family, d, l, &crate::detail::DetailParams::from_b1(b1), view)
}

/// 带**通用参数**的统一生成入口（第二期退刀槽的 `P/g1/g2/dg/r/alpha` 走这里；
/// 标准件的参数集恒为空）。`b1` 与 `params` 合并，同名时 `params` 优先。
pub fn generate_requested_params(
    family: &str,
    d: f64,
    l: Option<f64>,
    params: &crate::detail::DetailParams,
    view: &str,
) -> Result<GenPart, String> {
    if crate::detail::is_detail(family) {
        return crate::detail::generate_params(family, d, params, view);
    }
    let l = l.ok_or_else(|| crate::i18n::t_fmt("cmd.parts.err.missing_param", &[("name", "l")]))?;
    generate(family, d, l, view)
}

/// URL 查询串（`family=hex_bolt_c&d=5&l=25&view=main`）→ 预览 SVG。
/// 结构要素形如 `family=detail_grind_od&d=100[&b1=8]&view=main` 或
/// `family=detail_thread_relief&d=20&P=1.5[&g1=…]&view=main`（无 l；参数自由带）。
pub fn preview_svg(query: &str) -> Result<String, String> {
    if let Some(result) = crate::detail::preview_svg(query) {
        return result;
    }
    let get = |k: &str| {
        query
            .split('&')
            .filter_map(|kv| kv.split_once('='))
            .find(|(a, _)| *a == k)
            .map(|(_, v)| v.to_string())
    };
    let family = get("family").unwrap_or_else(|| "hex_bolt_c".to_string());
    let d: f64 = get("d")
        .ok_or_else(|| crate::i18n::t_fmt("cmd.parts.err.missing_param", &[("name", "d")]))?
        .parse()
        .map_err(|_| crate::i18n::t_fmt("cmd.parts.err.param_not_number", &[("name", "d")]))?;
    let l: f64 = get("l")
        .ok_or_else(|| crate::i18n::t_fmt("cmd.parts.err.missing_param", &[("name", "l")]))?
        .parse()
        .map_err(|_| crate::i18n::t_fmt("cmd.parts.err.param_not_number", &[("name", "l")]))?;
    let view = get("view").unwrap_or_else(|| "main".to_string());
    let part = generate(&family, d, l, &view)?;
    let title = format!("{} {} {}", part.meta.code, part.meta.name, part.meta.spec);
    Ok(to_svg(&part, &title, 520.0, 260.0))
}

// ── SVG 预览（选择器页 / 人工核对共用）────────────────────────────────────
//
// 按图层着色（1轮廓实线层=黑、2细线层=青、3中心线层=红点划线、5剖面线层=暗金），自动适配包围盒。

/// 剖面线边界路径 → 多边形顶点（Line / 带 bulge 多段线 / 圆弧按 8 段近似）。
pub fn boundary_polygon(
    path: &ocs_plugin_api::host::acadrust::entities::hatch::BoundaryPath,
) -> Vec<(f64, f64)> {
    use ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge as BE;
    let mut pts: Vec<(f64, f64)> = Vec::new();
    for e in &path.edges {
        match e {
            BE::Line(l) => {
                pts.push((l.start.x, l.start.y));
                pts.push((l.end.x, l.end.y));
            }
            BE::Polyline(pe) => {
                for v in &pe.vertices {
                    pts.push((v.x, v.y));
                }
            }
            BE::CircularArc(a) => {
                for step in 0..=8 {
                    let ang = a.start_angle + (a.end_angle - a.start_angle) * (step as f64 / 8.0);
                    pts.push((a.center.x + a.radius * ang.cos(), a.center.y + a.radius * ang.sin()));
                }
            }
            _ => {}
        }
    }
    pts
}

/// 直线（过 `p0`、方向 `dir`）与闭合多边形的**偶数规则**内外区间裁剪。
pub fn clip_line_to_polygon(
    p0: (f64, f64),
    dir: (f64, f64),
    poly: &[(f64, f64)],
) -> Vec<((f64, f64), (f64, f64))> {
    let n = poly.len();
    if n < 3 {
        return Vec::new();
    }
    let mut ts: Vec<f64> = Vec::new();
    for i in 0..n {
        let (ax, ay) = poly[i];
        let (bx, by) = poly[(i + 1) % n];
        let (ex, ey) = (bx - ax, by - ay);
        let den = dir.0 * ey - dir.1 * ex;
        if den.abs() < 1e-12 {
            continue; // 平行
        }
        // p0 + t·dir = a + s·e
        let (rx, ry) = (ax - p0.0, ay - p0.1);
        let t = (rx * ey - ry * ex) / den;
        // s = cross(r, dir) / cross(dir, e)（与 t 同分母 den）；符号写反会得到错误的裁剪区间
        let s = (rx * dir.1 - ry * dir.0) / den;
        if (-1e-9..=1.0 + 1e-9).contains(&s) {
            ts.push(t);
        }
    }
    if ts.len() < 2 {
        return Vec::new();
    }
    ts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // 去重：线正好穿过多边形顶点时，同一处会被相邻两条边各记一次（否则奇偶配对错位）
    let mut uniq: Vec<f64> = Vec::with_capacity(ts.len());
    for t in ts {
        if uniq.last().is_none_or(|p| (t - p).abs() > 1e-7) {
            uniq.push(t);
        }
    }
    let mut segs = Vec::new();
    let mut i = 0;
    while i + 1 < uniq.len() {
        let (t0, t1) = (uniq[i], uniq[i + 1]);
        if t1 - t0 > 1e-9 {
            segs.push((
                (p0.0 + t0 * dir.0, p0.1 + t0 * dir.1),
                (p0.0 + t1 * dir.0, p0.1 + t1 * dir.1),
            ));
        }
        i += 2;
    }
    segs
}

/// 把 HATCH 的图案线渲染成 SVG 线段（`to_svg` 与 `gear::svg_of` **共用同一个函数**）。
///
/// 与宿主 `scene/entity.rs::family_from_stored_line` **同一读法**：`pattern.lines` 的
/// `angle` 已是最终角度，`offset` 是世界向量 → `(dx, dy) = (沿线位移, 垂直距离)`，
/// `pattern_scale` 只作记录。因此预览与出图共用同一份图案定义，
/// 不再各自写 45°/3.175 的第二套口径。
/// 边界支持直线/折线/圆弧（圆弧按 [`boundary_polygon`] 的 8 段近似）。
pub fn hatch_svg_lines(
    h: &ocs_plugin_api::host::acadrust::entities::Hatch,
    tx: &dyn Fn(f64) -> f64,
    ty: &dyn Fn(f64) -> f64,
    stroke: &str,
    stroke_width: f64,
) -> String {
    let mut out = String::new();
    for path in &h.paths {
        let poly = boundary_polygon(path);
        if poly.len() < 3 {
            continue;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for (x, y) in &poly {
            x0 = x0.min(*x);
            y0 = y0.min(*y);
            x1 = x1.max(*x);
            y1 = y1.max(*y);
        }
        for ln in &h.pattern.lines {
            let (ca, sa) = (ln.angle.cos(), ln.angle.sin());
            let dir = (ca, sa);
            // 图案定义的 offset 本身就是**世界向量**（宿主 `family_from_stored_line` 也这么读）：
            // 线族 = 过基点 + k·offset、方向 dir。直接用它排线时，
            // 线间垂直距离 = offset 的垂直分量 = 宿主读出的 |dy| —— 预览与出图同源。
            // （旧写法把 offset 反旋转成坐标后再当世界向量用，45° 时垂距被少算 √2。）
            let step = (ln.offset.x, ln.offset.y);
            let space = step.0.hypot(step.1);
            if space < 1e-6 {
                continue;
            }
            // 沿线间距 step 排线，覆盖边界包围盒（投影到 step 方向）
            let ext = ((x1 - x0).hypot(y1 - y0)) + (x1 - x0).abs() + (y1 - y0).abs();
            let n = (ext / space).ceil().min(2000.0) as i64;
            let phase = if ln.base_point.x.abs() + ln.base_point.y.abs() > 1e-12 {
                ln.base_point.x * step.0 + ln.base_point.y * step.1
            } else {
                0.0
            };
            let k0 = -n - (phase / space).floor() as i64;
            for k in k0..=(k0 + 2 * n) {
                let (px, py) = (k as f64 * step.0, k as f64 * step.1);
                for (a, b) in clip_line_to_polygon((px, py), dir, &poly) {
                    out.push_str(&format!(
                        "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"/>",
                        tx(a.0),
                        ty(a.1),
                        tx(b.0),
                        ty(b.1)
                    ));
                }
            }
        }
    }
    out
}

/// 单个视图 → 独立 SVG（宽高像素由 `px_w`/`px_h` 控制）。
pub fn to_svg(part: &GenPart, title: &str, px_w: f64, px_h: f64) -> String {
    let [x0, y0, x1, y1] = part.bbox;
    let (w, h) = ((x1 - x0).max(1e-6), (y1 - y0).max(1e-6));
    let pad = 0.08 * w.max(h);
    let k = ((px_w - 2.0 * pad) / w).min((px_h - 2.0 * pad) / h);
    let tx = |x: f64| pad + (x - x0) * k + (px_w - 2.0 * pad - w * k) / 2.0;
    let ty = |y: f64| px_h - pad - (y - y0) * k - (px_h - 2.0 * pad - h * k) / 2.0;
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{px_w}\" height=\"{px_h}\" viewBox=\"0 0 {px_w} {px_h}\">\
         <rect width=\"{px_w}\" height=\"{px_h}\" fill=\"#fff\"/>\
         <text x=\"8\" y=\"18\" font-family=\"sans-serif\" font-size=\"13\" fill=\"#333\">{}</text>",
        title.replace('&', "&amp;").replace('<', "&lt;")
    );
    for e in &part.entities {
        let layer = e.common().layer.as_str();
        let (color, dash) = match layer {
            LAYER_THIN => ("#00b0c0", ""),
            LAYER_CENTER => ("#d02020", "stroke-dasharray=\"10 3 3 3\""),
            // 4虚线层（不可见轮廓）
            "4虚线层" => ("#1040c0", "stroke-dasharray=\"9 4\""),
            // 5剖面线层（Hatch 图形本体在各自分支里画，这里只备用）
            "5剖面线层" => ("#b8860b", ""),
            _ => ("#111111", ""),
        };
        match e {
            EntityType::Line(l) => out.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{color}\" stroke-width=\"1.3\" {dash}/>",
                tx(l.start.x), ty(l.start.y), tx(l.end.x), ty(l.end.y)
            )),
            EntityType::Circle(c) => out.push_str(&format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.3\" {dash}/>",
                tx(c.center.x), ty(c.center.y), c.radius * k
            )),
            EntityType::Arc(a) => {
                // DXF/acadrust 的弧一律 **start→end 逆时针（CCW）**：跨度 = (end − start) mod 360。
                // 屏幕坐标 y 向下 → 同一段弧在屏幕上变成顺时针（sweep-flag = 0）。
                let span = {
                    let d = a.end_angle - a.start_angle;
                    let d = d.rem_euclid(std::f64::consts::TAU);
                    if d == 0.0 {
                        std::f64::consts::TAU
                    } else {
                        d
                    }
                };
                let (a0, a1) = (-a.start_angle.to_degrees(), -(a.start_angle + span).to_degrees());
                let large = if span.to_degrees() > 180.0 { 1 } else { 0 };
                let dir = 0;
                let (sx, sy) = (
                    tx(a.center.x) + a.radius * k * a0.to_radians().cos(),
                    ty(a.center.y) + a.radius * k * a0.to_radians().sin(),
                );
                let (ex, ey) = (
                    tx(a.center.x) + a.radius * k * a1.to_radians().cos(),
                    ty(a.center.y) + a.radius * k * a1.to_radians().sin(),
                );
                out.push_str(&format!(
                    "<path d=\"M {sx:.2} {sy:.2} A {:.2} {:.2} 0 {large} {dir} {ex:.2} {ey:.2}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.3\" {dash}/>",
                    a.radius * k, a.radius * k
                ));
            }
            EntityType::Hatch(h) => out.push_str(&hatch_svg_lines(h, &tx, &ty, "#b8860b", 1.1)),
            EntityType::LwPolyline(pl) => {
                let d: Vec<String> = pl
                    .vertices
                    .iter()
                    .enumerate()
                    .map(|(i, v)| format!("{} {:.2} {:.2}", if i == 0 { "M" } else { "L" }, tx(v.location.x), ty(v.location.y)))
                    .collect();
                out.push_str(&format!(
                    "<path d=\"{}{}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.3\" {dash}/>",
                    d.join(" "),
                    if pl.is_closed { " Z" } else { "" }
                ));
            }
            _ => {}
        }
    }
    out.push_str("</svg>");
    out
}

#[cfg(test)]
mod svg_dump {
    /// 手工核对用：把 C 级螺栓三视图写成 SVG（`cargo test -p ocs_ocsm dump_bolt_svg -- --ignored --nocapture`）。
    #[test]
    #[ignore]
    fn dump_bolt_svg() {
        let dir = std::path::Path::new("/tmp/partgen");
        std::fs::create_dir_all(dir).unwrap();
        let cases: &[(f64, f64, &str, super::BoltView)] = &[
            (5.0, 25.0, "M5x25-主视图", super::BoltView::Main),
            (5.0, 25.0, "M5x25-俯视图", super::BoltView::Top),
            (5.0, 25.0, "M5x25-左视图", super::BoltView::End),
            (10.0, 45.0, "M10x45-主视图", super::BoltView::Main),
            (16.0, 65.0, "M16x65-主视图", super::BoltView::Main),
            (24.0, 100.0, "M24x100-主视图", super::BoltView::Main),
            (16.0, 65.0, "M16x65-左视图", super::BoltView::End),
        ];
        for (d, l, name, v) in cases {
            let p = super::hex_bolt_c(*d, *l, *v).unwrap();
            let svg = super::to_svg(&p, &format!("{} {}", p.meta.code, name), 460.0, 300.0);
            let f = dir.join(format!("boltC-{name}.svg"));
            std::fs::write(&f, svg).unwrap();
            println!("写出 {}", f.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 语言是进程级全局：断言中文文案的用例共用锁并钉死 zh。
    fn zh_guard() -> std::sync::MutexGuard<'static, ()> {
        let g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        g
    }
    /// 剖面线裁剪（预览用）：直线与闭合多边形的偶数规则区间。
    #[test]
    fn hatch_preview_clip_line_to_polygon() {
        // 长方形 0..20 × 0..10
        let poly = vec![(0.0, 0.0), (20.0, 0.0), (20.0, 10.0), (0.0, 10.0)];
        // 水平线 y=2 穿过矩形
        let segs = clip_line_to_polygon((0.0, 2.0), (1.0, 0.0), &poly);
        assert_eq!(segs.len(), 1, "一条区间：{segs:?}");
        let ((x0, y0), (x1, y1)) = segs[0];
        assert!((x0 - 0.0).abs() < 1e-9 && (x1 - 20.0).abs() < 1e-9, "区间应为 0→20：{segs:?}");
        assert!((y0 - 2.0).abs() < 1e-9 && (y1 - 2.0).abs() < 1e-9);
        // 45° 线过原点：在矩形内的区间 = (0,0)→(10,10)
        let segs = clip_line_to_polygon((0.0, 0.0), (std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2), &poly);
        assert_eq!(segs.len(), 1, "{segs:?}");
        let ((x0, y0), (x1, y1)) = segs[0];
        assert!(x0.abs() < 1e-9 && y0.abs() < 1e-9 && (x1 - 10.0).abs() < 1e-9 && (y1 - 10.0).abs() < 1e-9, "{segs:?}");
        // 完全在外的线：无区间
        assert!(clip_line_to_polygon((0.0, 100.0), (1.0, 0.0), &poly).is_empty());
        // 凹多边形（L 形）：横线穿两个区间
        let l = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 4.0), (4.0, 4.0), (4.0, 10.0), (0.0, 10.0)];
        let segs = clip_line_to_polygon((0.0, 6.0), (1.0, 0.0), &l);
        assert_eq!(segs.len(), 1, "L 形上半部只有 x∈[0,4]：{segs:?}");
        let segs = clip_line_to_polygon((0.0, 2.0), (1.0, 0.0), &l);
        assert_eq!(segs.len(), 1, "L 形下半部 x∈[0,10]：{segs:?}");
    }

    /// 与用户提供的规范图逐条对齐（`六角头螺栓 C级 GBT5780-2016_主视图.dxf`，M5x25）。
    #[test]
    fn hex_bolt_c_main_view_matches_user_dxf() {
        let p = hex_bolt_c(5.0, 25.0, BoltView::Main).expect("M5x25 主视图");
        let ls = lines(&p);
        let near = |a: f64, b: f64| (a - b).abs() < 1e-3;
        let has_line = |x1: f64, y1: f64, x2: f64, y2: f64| {
            ls.iter().any(|(a, b, _)| {
                (near(a[0], x1) && near(a[1], y1) && near(b[0], x2) && near(b[1], y2))
                    || (near(a[0], x2) && near(a[1], y2) && near(b[0], x1) && near(b[1], y1))
            })
        };
        // 头：端面 / 轮廓 / 斜线 / 棱线 / 交界
        assert!(has_line(-3.5, 4.0, -3.5, -4.0), "端面 x=-k，跨对边 s");
        assert!(has_line(-3.143, 4.619, 0.0, 4.619), "上轮廓到 x=0（e/2 高）");
        assert!(has_line(-3.143, -4.619, 0.0, -4.619), "下轮廓");
        assert!(has_line(-3.5, 4.0, -3.143, 4.619), "倒角斜线（上）");
        assert!(has_line(-3.5, -4.0, -3.143, -4.619), "倒角斜线（下）");
        assert!(has_line(-3.143, 2.309, 0.0, 2.309), "头内棱线 +e/4");
        assert!(has_line(-3.143, -2.309, 0.0, -2.309), "头内棱线 -e/4");
        assert!(has_line(0.0, 4.619, 0.0, -4.619), "头杆交界 x=0");
        // 杆 / 螺纹 / 中心线
        assert!(has_line(0.0, 2.5, 24.625, 2.5), "杆轮廓（上）");
        assert!(has_line(24.625, 2.5, 25.0, 2.125), "杆端倒角");
        assert!(has_line(25.0, 2.125, 25.0, -2.125), "杆端面");
        assert!(has_line(9.0, 2.125, 25.0, 2.125), "牙底细实线（l−b → l）");
        assert!(has_line(9.0, 2.5, 9.0, -2.5), "螺纹起点竖线（l−b 处满杆径）");
        assert!(has_line(24.625, 2.5, 24.625, -2.5), "杆端倒角起点竖线");
        assert!(has_line(5.0, 2.5, 9.0, 2.125), "螺纹收尾斜线（runout=4）");
        assert!(has_line(-6.5, 0.0, 28.0, 0.0), "中心线出头 0.375s");
        // 圆弧：角弧 / 端面倒角大弧
        let arcs: Vec<(f64, f64, f64, f64, f64, String)> = p
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) => Some((
                    a.center.x,
                    a.center.y,
                    a.radius,
                    a.start_angle.to_degrees(),
                    a.end_angle.to_degrees(),
                    a.common.layer.clone(),
                )),
                _ => None,
            })
            .collect();
        // 角度按 0.05° 容差比较（DXF 里角度只存两位小数）
        let near_ang = |a: f64, b: f64| (a - b).abs() < 0.05;
        let found = |cx: f64, cy: f64, r: f64, a0: f64, a1: f64| {
            arcs.iter().any(|(x, y, rr, s0, s1, _)| {
                near(*x, cx) && near(*y, cy) && near(*rr, r) && near_ang(*s0, a0) && near_ang(*s1, a1)
            })
        };
        // 上角弧 = 下角弧的镜像（180° → 180°+34.38°），终点落在棱线/大弧交点
        assert!(found(-1.455, 3.464, 2.045, 180.0, 214.38), "角弧（上，镜像）");
        assert!(found(-1.455, -3.464, 2.045, 145.62, 180.0), "角弧（下）");
        assert!(found(4.143, 0.0, 7.643, 162.41, 197.59), "端面倒角大弧");
        // 不含任何尺寸标注
        assert!(!p.entities.iter().any(|e| matches!(e, EntityType::Dimension(_))));
        assert_eq!(p.meta.code, "GB/T 5780-2016");
        assert_eq!(p.meta.spec, "M5x25");
    }

    /// 俯视图 / 左视图同样与用户 DXF 对齐。
    #[test]
    fn hex_bolt_c_top_and_end_view_match_user_dxf() {
        let top = hex_bolt_c(5.0, 25.0, BoltView::Top).expect("俯视图");
        let ls = lines(&top);
        let near = |a: f64, b: f64| (a - b).abs() < 1e-3;
        let has = |x1: f64, y1: f64, x2: f64, y2: f64| {
            ls.iter().any(|(a, b, _)| {
                (near(a[0], x1) && near(a[1], y1) && near(b[0], x2) && near(b[1], y2))
                    || (near(a[0], x2) && near(a[1], y2) && near(b[0], x1) && near(b[1], y1))
            })
        };
        assert!(has(-3.5, 4.0, -3.5, -4.0), "俯视图端面跨对边");
        assert!(has(-3.5, 4.0, 0.0, 4.0), "俯视图轮廓从端面 x=−k 起（对边 s）");
        assert!(has(-3.143, 0.0, 0.0, 0.0), "俯视图头内中棱线");
        let arcs: Vec<(f64, f64, f64, f64, f64)> = top
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) => Some((
                    a.center.x,
                    a.center.y,
                    a.radius,
                    a.start_angle.to_degrees(),
                    a.end_angle.to_degrees(),
                )),
                _ => None,
            })
            .collect();
        let near_ang = |a: f64, b: f64| (a - b).abs() < 0.05;
        let found = |cx: f64, cy: f64, r: f64, a0: f64, a1: f64| {
            arcs.iter().any(|(x, y, rr, s0, s1)| {
                near(*x, cx) && near(*y, cy) && near(*rr, r) && near_ang(*s0, a0) && near_ang(*s1, a1)
            })
        };
        assert!(found(2.277, 2.0, 5.777, 159.74, 200.26), "俯视图倒角弧（上）");
        assert!(found(2.277, -2.0, 5.777, 159.74, 200.26), "俯视图倒角弧（下）");

        let end = hex_bolt_c(5.0, 25.0, BoltView::End).expect("左视图");
        let els = lines(&end);
        let has_e = |x1: f64, y1: f64, x2: f64, y2: f64| {
            els.iter().any(|(a, b, _)| {
                (near(a[0], x1) && near(a[1], y1) && near(b[0], x2) && near(b[1], y2))
                    || (near(a[0], x2) && near(a[1], y2) && near(b[0], x1) && near(b[1], y1))
            })
        };
        assert!(has_e(0.0, 4.619, -4.0, 2.309), "六边形边（上-左）");
        assert!(has_e(4.0, -2.309, 4.0, 2.309), "六边形右侧对边");
        assert!(has_e(-7.0, 0.0, 7.0, 0.0), "水平中心线 ±0.875s");
        assert!(has_e(0.0, 7.0, 0.0, -7.0), "竖直中心线");
        let circle = end.entities.iter().find_map(|e| match e {
            EntityType::Circle(c) => Some((c.center.x, c.center.y, c.radius, c.common.layer.clone())),
            _ => None,
        });
        assert_eq!(circle, Some((0.0, 0.0, 4.0, LAYER_MAIN.to_string())), "端面倒角圆 r=s/2");
    }

    /// 画法关键比例（跨直径自洽）：Δ=(e−s)/2·tan30°。
    #[test]
    fn chamfer_run_rule_matches_samples() {
        assert!((chamfer_run(8.0, across_corners(8.0)) - 0.3573).abs() < 1e-3, "M5");
        assert!((chamfer_run(16.0, across_corners(16.0)) - 0.7146).abs() < 1e-3, "M10");
        assert!((chamfer_run(24.0, across_corners(24.0)) - 1.0719).abs() < 1e-3, "M16");
    }

    /// 长度系列与长度校验。
    #[test]
    fn hex_bolt_c_lengths_and_validation() {
        let ls = hex_bolt_c_lengths(10.0);
        assert!(ls.contains(&40.0) && ls.contains(&100.0), "M10 长度系列");
        assert_eq!(ls.first().copied(), Some(40.0));
        assert!(hex_bolt_c(10.0, 30.0, BoltView::Main).is_err(), "低于 l_min");
        assert!(hex_bolt_c(10.0, 200.0, BoltView::Main).is_err(), "超过 l_max");
        assert!(hex_bolt_c(7.0, 40.0, BoltView::Main).is_err(), "无此直径");
        // 螺纹长度规则
        let row = hex_bolt_c_row(10.0).unwrap();
        assert_eq!(thread_length_c(row, 100.0), 26.0);
        assert_eq!(thread_length_c(row, 150.0), 32.0);
        assert_eq!(thread_length_c(row, 220.0), 45.0);
    }

    /// 所有生成的图元都必须是 ByLayer + 三个 OCSM 图层之一（不含尺寸标注）。
    #[test]
    fn hex_bolt_c_output_is_clean() {
        for view in [BoltView::Main, BoltView::Top, BoltView::End] {
            let p = hex_bolt_c(24.0, 100.0, view).unwrap();
            assert!(!p.entities.is_empty());
            for e in &p.entities {
                let c = e.common();
                assert_eq!(c.color, Color::ByLayer);
                assert_eq!(c.linetype, "ByLayer");
                assert_eq!(c.line_weight, LineWeight::ByLayer);
                assert!([LAYER_MAIN, LAYER_THIN, LAYER_CENTER].contains(&c.layer.as_str()));
                assert!(!matches!(e, EntityType::Dimension(_)));
            }
        }
    }

    use ocs_plugin_api::host::acadrust::entities::EntityType;

    fn layer_of(e: &EntityType) -> &str {
        &e.common().layer
    }

    fn lines(part: &GenPart) -> Vec<([f64; 2], [f64; 2], String)> {
        part.entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Line(l) => Some((
                    [l.start.x, l.start.y],
                    [l.end.x, l.end.y],
                    l.common.layer.clone(),
                )),
                _ => None,
            })
            .collect()
    }

    /// GB/T 5782-2016 A/B 级 = **C 级画法 + 头部垫圈面 dw×c**
    /// （用户 2026-09-15 确认：主视图/俯视图按草稿画，**左视图不改**）。
    ///
    /// 强断言：主/俯视图里 C 级的每条图元，要么与 AB 完全重合，要么在 AB 里被整体平移 −c；
    /// 且 AB 恰好比 C 级多 3 条垫圈面线；左视图两族完全相同（不画垫圈面圆）。
    #[test]
    fn hex_bolt_ab_is_c_drawing_plus_washer_face() {
        let ab_row = hex_bolt_ab_row(10.0).unwrap();
        let (dw, c) = (ab_row.dw, ab_row.c);
        fn key(p: &GenPart) -> Vec<String> {
            p.entities
                .iter()
                .map(|e| match e {
                    EntityType::Line(l) => format!(
                        "LINE {} ({:.4},{:.4})-({:.4},{:.4})",
                        l.common.layer, l.start.x, l.start.y, l.end.x, l.end.y
                    ),
                    EntityType::Arc(a) => format!(
                        "ARC {} ({:.4},{:.4}) r={:.4} {:.4}->{:.4}",
                        a.common.layer,
                        a.center.x,
                        a.center.y,
                        a.radius,
                        a.start_angle.to_degrees(),
                        a.end_angle.to_degrees()
                    ),
                    EntityType::Circle(ci) => format!(
                        "CIRCLE {} ({:.4},{:.4}) r={:.4}",
                        ci.common.layer, ci.center.x, ci.center.y, ci.radius
                    ),
                    other => format!("OTHER {}", other.common().layer),
                })
                .collect()
        }
        for (view, name) in [(BoltView::Main, "主视图"), (BoltView::Top, "俯视图")] {
            let pc = hex_bolt_c(10.0, 60.0, view).unwrap();
            let pab = hex_bolt_ab(10.0, 60.0, view).unwrap();
            assert_eq!(pab.entities.len(), pc.entities.len() + 3, "{name}: 只多 3 条垫圈面线");
            let ck = key(&pc);
            let abk = key(&pab);
            for k in &ck {
                // C 级的一条线，在 AB 里应是下列之一：
                // ① 原样保留（杆/螺纹/中心线等与支承面无关的）；② 整体平移 −c（支承面线）；
                // ③ 只有 x=0 那端内缩到 x=−c（轮廓线/内棱线 —— 另一端仍从端面 x_out 起）
                let ok = abk.contains(k)
                    || abk.contains(&shift_key_x(k, -c, false))
                    || abk.contains(&shift_key_x(k, -c, true));
                assert!(ok, "{name}: C 级图元 {k} 在 AB 里既没原样保留、也没平移/端点内缩 −c");
            }
            // AB 里"与 C 级对不上"的图元：只能是 ① 3 条垫圈面线，或 ② C 级某条线经上述变换后的版本
            for k in abk.iter().filter(|k| !ck.contains(k)) {
                let is_washer = k.contains("7.3150") || (k.contains("(0.0000,") && k.contains("(-0.6000,"));
                let moved = ck.iter().any(|c2| {
                    shift_key_x(c2, -c, false) == *k || shift_key_x(c2, -c, true) == *k
                });
                assert!(is_washer || moved, "{name}: AB 多出的图元既不是垫圈面线也不是 C 级线的平移/内缩: {k}");
            }
            // 垫圈面三条（凸台母线 ×2 + 垫圈面线）必须在 AB、且不在 C
            for k in [
                format!("LINE {} ({:.4},{:.4})-({:.4},{:.4})", LAYER_MAIN, -c, dw / 2.0, 0.0, dw / 2.0),
                format!("LINE {} ({:.4},{:.4})-({:.4},{:.4})", LAYER_MAIN, -c, -dw / 2.0, 0.0, -dw / 2.0),
                format!("LINE {} ({:.4},{:.4})-({:.4},{:.4})", LAYER_MAIN, 0.0, dw / 2.0, 0.0, -dw / 2.0),
            ] {
                assert!(abk.contains(&k), "{name}: 缺垫圈面线 {k}");
                assert!(!ck.contains(&k), "{name}: C 级不应有垫圈面线");
            }
        }
        // 左视图：两族完全相同（**不画**垫圈面圆）
        assert_eq!(
            key(&hex_bolt_c(10.0, 60.0, BoltView::End).unwrap()),
            key(&hex_bolt_ab(10.0, 60.0, BoltView::End).unwrap()),
            "左视图应与 C 级完全相同（用户：左视图不改）"
        );
        // 绝对坐标钉死（M10：dw=14.63 → ±7.315；c=0.6 → 支承面在 x=−0.6）
        assert!((dw - 14.63).abs() < 1e-9 && (c - 0.6).abs() < 1e-9);
        for (view, name) in [(BoltView::Main, "主视图"), (BoltView::Top, "俯视图")] {
            let p = hex_bolt_ab(10.0, 60.0, view).unwrap();
            let half = dw / 2.0;
            let has = |x1: f64, y1: f64, x2: f64, y2: f64| {
                p.entities.iter().any(|e| match e {
                    EntityType::Line(l) => {
                        let near = |u: f64, v: f64| (u - v).abs() < 1e-9;
                        (near(l.start.x, x1) && near(l.start.y, y1) && near(l.end.x, x2) && near(l.end.y, y2))
                            || (near(l.start.x, x2) && near(l.start.y, y2) && near(l.end.x, x1) && near(l.end.y, y1))
                    }
                    _ => false,
                })
            };
            assert!(has(-c, half, 0.0, half), "{name}: 凸台母线（上）");
            assert!(has(-c, -half, 0.0, -half), "{name}: 凸台母线（下）");
            assert!(has(0.0, half, 0.0, -half), "{name}: 垫圈面线");
            let want = if view == BoltView::Main {
                across_corners(ab_row.s) / 2.0
            } else {
                ab_row.s / 2.0
            };
            assert!(
                p.entities.iter().any(|e| matches!(e, EntityType::Line(l)
                    if (l.start.x + c).abs() < 1e-9 && (l.end.x + c).abs() < 1e-9
                        && (l.start.y - want).abs() < 1e-9 && (l.end.y + want).abs() < 1e-9)),
                "{name}: 支承面线内缩到 x=−c 且宽 ±{want}"
            );
        }
    }

    /// 把 "LINE 层 (x,y)-(x,y)" 这类 key 里的 x 坐标平移 dx（配合上面那条断言）。
    /// `only_zero = true` 时只平移原本 x≈0 的端点（模拟"支承面侧内缩、另一端不动"）。
    fn shift_key_x(k: &str, dx: f64, only_zero: bool) -> String {
        let mut out = String::new();
        let mut rest = k;
        while let Some(i) = rest.find('(') {
            out.push_str(&rest[..i]);
            let j = match rest[i..].find(')') {
                Some(v) => v + i,
                None => break,
            };
            let inside = &rest[i + 1..j];
            match inside.split_once(',') {
                Some((a, b)) => {
                    let x: f64 = a.trim().parse().unwrap_or(f64::NAN);
                    let nx = if only_zero && x.abs() > 1e-9 { x } else { x + dx };
                    out.push_str(&format!("({:.4},{})", nx, b.trim()));
                }
                None => {
                    out.push('(');
                    out.push_str(inside);
                    out.push(')');
                }
            }
            rest = &rest[j + 1..];
        }
        out.push_str(rest);
        out
    }

    /// 两族差异只在**数据 + 头部垫圈面**：s/k 逐规格相同（所以头部画法一致），
    /// e（公差下限，仅记录）与可用长度区间不同（用户说的「可用尺寸区间有出入」）；
    /// A/B 级另有垫圈面 dw×c（主/俯视图画，左视图不画）。
    #[test]
    fn hex_bolt_ab_table_differs_from_c_only_in_records() {
        assert_eq!(hex_bolt_ab_diameters().len(), 29, "5782 共 29 规格（M1.6…M64）");
        let mut diff_len = 0;
        for d in hex_bolt_ab_diameters() {
            if let Some(cr) = hex_bolt_c_row(d) {
                let ar = hex_bolt_ab_row(d).unwrap();
                assert_eq!(cr.s, ar.s, "M{d} 对边宽 s 应相同（画法一致的前提）");
                assert_eq!(cr.k, ar.k, "M{d} 头高 k 应相同");
                assert!(cr.pitch == ar.pitch, "M{d} 螺距应相同");
                // 螺纹长分档相同（都是 2d+6 / 2d+12 / 2d+25）；b1 在最小长度 > 125 的大直径上
                // 一方可能记 0（不适用）—— 只比"双方都填了"的档
                for (ci, ai, label) in [(cr.b1, ar.b1, "b1"), (cr.b2, ar.b2, "b2"), (cr.b3, ar.b3, "b3")] {
                    if ci > 0.0 && ai > 0.0 {
                        assert_eq!(ci, ai, "M{d} {label} 应相同");
                    }
                }
                if cr.l_min != ar.l_min || cr.l_max != ar.l_max {
                    diff_len += 1;
                }
                assert!(ar.dw > 0.0 && ar.c > 0.0, "M{d} A/B 级应有垫圈面 dw/c（已用于主/俯视图）");
            }
        }
        assert!(diff_len >= 5, "至少若干规格的可用长度区间应与 C 级不同（实际 {diff_len}）");
        // 具体差异例（用户说的「支持的尺寸区间不同」）
        assert_eq!(hex_bolt_c_row(10.0).unwrap().l_min, 40.0);
        assert_eq!(hex_bolt_ab_row(10.0).unwrap().l_min, 45.0);
        assert_eq!(hex_bolt_c_row(36.0).unwrap().l_min, 110.0);
        assert_eq!(hex_bolt_ab_row(36.0).unwrap().l_min, 140.0);
        assert!((hex_bolt_ab_row(42.0).unwrap().e - 71.3).abs() < 1e-9, "M42 e=71.3（官方规格图）");
        // 越界报错自带族名
        assert!(hex_bolt_ab(10.0, 300.0, BoltView::Main).unwrap_err().contains("GB/T 5782"));
    }

    /// 5782 全部 29 规格 × 长度系列 × 三视图都能生成，且长度序列递增。
    #[test]
    fn hex_bolt_ab_all_specs_generate() {
        for d in hex_bolt_ab_diameters() {
            let ls = hex_bolt_ab_lengths(d);
            assert!(!ls.is_empty(), "M{d} 无长度系列");
            assert!(ls.windows(2).all(|w| w[1] > w[0]), "M{d} 长度序列非递增");
            for l in &ls {
                for view in [BoltView::Main, BoltView::Top, BoltView::End] {
                    let p = hex_bolt_ab(d, *l, view)
                        .unwrap_or_else(|e| panic!("5782 M{d}x{l} {view:?} 生成失败: {e}"));
                    assert!(!p.entities.is_empty());
                    assert!(
                        !p.entities.iter().any(|e| matches!(e, EntityType::Dimension(_))),
                        "5782 M{d}x{l} 不含尺寸标注"
                    );
                }
            }
        }
    }

    #[test]
    fn hex_bolt_m10x100_matches_gb_dims() {
        let p = hex_bolt(10.0, 100.0, false).expect("生成 M10x100");
        // 头：对边 16、头高 6.4、对角 16/cos30 = 18.475
        // 对边视图：外形宽 = 对边 s = 16；对角宽 e = 18.475 只在"对角线视图"里出现
        assert!((p.bbox[0] + 6.4).abs() < 1e-9, "头部端面在 x=-k");
        assert!(p.bbox[1] == -8.0 && p.bbox[3] == 8.0, "头部外形宽 = 对边 16");
        assert!((across_corners(16.0) - 18.475).abs() < 1e-3);
        let ls = lines(&p);
        // 端面竖线 x=-6.4，长 16（对边）
        let face = ls
            .iter()
            .find(|(a, b, _)| (a[0] + 6.4).abs() < 1e-9 && (b[0] + 6.4).abs() < 1e-9)
            .expect("端面线");
        assert!((face.0[1].abs() - 8.0).abs() < 1e-9 && (face.1[1].abs() - 8.0).abs() < 1e-9);
        // 螺纹：M10 螺距 1.5 → 小径 8.376；b1 = 26 → 牙底线 74…100
        let d1 = minor_dia(10.0, 1.5);
        // 细实线：2 条牙底线（74→100，d1/2）+ 2 条螺纹收尾斜线
        let thread: Vec<_> = ls
            .iter()
            .filter(|(a, b, lay)| {
                lay == LAYER_THIN
                    && (a[0] - 74.0).abs() < 1e-9
                    && (b[0] - 100.0).abs() < 1e-9
            })
            .collect();
        assert_eq!(thread.len(), 2, "牙底线两条");
        for (a, _, _) in &thread {
            assert!((a[1].abs() - d1 / 2.0).abs() < 1e-9, "牙底细实线在 d1/2");
        }
        let runouts = ls
            .iter()
            .filter(|(_, _, lay)| lay == LAYER_THIN)
            .count();
        assert_eq!(runouts, 4, "2 条牙底线 + 2 条收尾斜线");
        // 螺纹终止线（粗实线）在 x=74
        assert!(ls.iter().any(|(a, b, lay)| lay == LAYER_MAIN
            && (a[0] - 74.0).abs() < 1e-9
            && (b[0] - 74.0).abs() < 1e-9));
        // 中心线在 y=0
        let center: Vec<_> = ls.iter().filter(|(_, _, lay)| lay == LAYER_CENTER).collect();
        assert_eq!(center.len(), 1);
        assert_eq!(center[0].0[1], 0.0);
        assert_eq!(p.meta.code, "GB/T 5782-2016");
        assert_eq!(p.meta.spec, "M10x100");
        // 设计重量是估算值，但**不写 ≈**（用户 2026-09-15）
        assert!(!p.meta.weight.contains('≈'));
        assert!(p.meta.weight.parse::<f64>().unwrap() > 0.0);
    }

    #[test]
    fn hex_bolt_full_thread_runs_whole_length() {
        let p = hex_bolt(10.0, 100.0, true).expect("全螺纹");
        let ls = lines(&p);
        let thread: Vec<_> = ls.iter().filter(|(_, _, lay)| lay == LAYER_THIN).collect();
        assert_eq!(thread.len(), 2, "全螺纹只有 2 条牙底线（无收尾斜线）");
        for (a, b, _) in &thread {
            assert!((a[0] - 0.0).abs() < 1e-9 && (b[0] - 100.0).abs() < 1e-9, "牙底贯通");
        }
        assert_eq!(p.meta.code, "GB/T 5783-2016");
        // 全螺纹没有螺纹终止线（不应有 x=74 处的竖线；杆端面在 x=100 不算）
        let stops: Vec<_> = ls
            .iter()
            .filter(|(a, b, lay)| {
                lay == LAYER_MAIN
                    && (a[0] - b[0]).abs() < 1e-9
                    && (a[0] - 74.0).abs() < 1e-9
            })
            .collect();
        assert!(stops.is_empty(), "全螺纹不应有螺纹终止线");
    }

    #[test]
    fn hex_nut_m10_matches_gb_dims() {
        let p = hex_nut(10.0).expect("生成 M10 螺母");
        // 对边 16、高 8.4；底面 y=0
        assert!((p.bbox[0] + 8.0).abs() < 1e-9 && (p.bbox[2] - 8.0).abs() < 1e-9);
        assert!((p.bbox[1] - 0.0).abs() < 1e-9 && (p.bbox[3] - 8.4).abs() < 1e-9);
        let ls = lines(&p);
        let d1 = minor_dia(10.0, 1.5);
        // 内螺纹：小径粗实线、大径细实线
        assert!(ls.iter().any(|(a, b, lay)| lay == LAYER_MAIN
            && (a[0] - d1 / 2.0).abs() < 1e-9
            && (b[0] - d1 / 2.0).abs() < 1e-9
            && (a[1] - 0.0).abs() < 1e-9
            && (b[1] - 8.4).abs() < 1e-9));
        assert!(ls.iter().any(|(a, b, lay)| lay == LAYER_THIN
            && (a[0] - 5.0).abs() < 1e-9
            && (b[0] - 5.0).abs() < 1e-9));
        assert_eq!(p.meta.code, "GB/T 6170-2015");
        assert_eq!(p.meta.spec, "M10");
    }

    #[test]
    fn generated_entities_are_all_bylayer_on_ocsm_layers() {
        for part in [
            hex_bolt(6.0, 30.0, false).unwrap(),
            hex_bolt(24.0, 80.0, true).unwrap(),
            hex_nut(20.0).unwrap(),
        ] {
            for e in &part.entities {
                let c = e.common();
                assert_eq!(c.color, Color::ByLayer, "颜色应随层");
                assert_eq!(c.linetype, "ByLayer");
                assert_eq!(c.line_weight, LineWeight::ByLayer);
                assert!(
                    [LAYER_MAIN, LAYER_THIN, LAYER_CENTER].contains(&c.layer.as_str()),
                    "图层只允许 1轮廓实线层/2细线层/3中心线层，实际 {}",
                    c.layer
                );
            }
        }
    }

    #[test]
    fn out_of_range_length_is_rejected() {
        assert!(hex_bolt(10.0, 10.0, false).is_err());
        assert!(hex_bolt(99.0, 100.0, false).is_err());
    }

    #[test]
    fn standard_length_series_covers_range() {
        let row = bolt_row(10.0).unwrap();
        let ls = standard_lengths(row);
        assert_eq!(*ls.first().unwrap(), row.l_min);
        assert_eq!(*ls.last().unwrap(), row.l_max);
        assert!(ls.contains(&40.0) && ls.contains(&100.0));
        assert!(ls.windows(2).all(|w| w[1] > w[0]));
    }

    #[test]
    fn thread_length_follows_iso_rule() {
        let row = bolt_row(10.0).unwrap();
        assert_eq!(thread_length(row, 100.0), 26.0);
        assert_eq!(thread_length(row, 150.0), 32.0);
        assert_eq!(thread_length(row, 220.0), 45.0);
    }
    /// 防回归：acadrust 的角度字段是**弧度**（DXF 读取时已 `.to_radians()`），
    /// 曾经把度数直接写进去 → 宿主渲染时弧线跑飞（用户实测）。
    #[test]
    fn arc_angles_are_stored_in_radians() {
        for view in [BoltView::Main, BoltView::Top, BoltView::End] {
            let p = hex_bolt_c(5.0, 25.0, view).unwrap();
            for e in &p.entities {
                if let EntityType::Arc(a) = e {
                    assert!(
                        a.start_angle.abs() <= std::f64::consts::TAU + 1e-9
                            && a.end_angle.abs() <= std::f64::consts::TAU + 1e-9,
                        "弧角度必须是弧度（start {} end {}）",
                        a.start_angle,
                        a.end_angle
                    );
                }
            }
        }
    }

    /// 目录 JSON 的**全局防呆**：每个上架族都必须有视图、有规格、且第一个规格能出图。
    ///
    /// 回归背景（2026-09-15）：`families_json` 曾把已下架族（`nut_6170`）的空视图数组
    /// 错给到 `nut_61721` / `nut_c41` → GUI 视图按钮为空、`pickFamily` 抛异常、
    /// 长度与预览全不加载（面板静默失效）。此测试是那一类错误的语料级护栏。
    #[test]
    fn catalog_all_implemented_families_usable() {
        let cat: serde_json::Value = serde_json::from_str(&catalog_json()).expect("目录 JSON");
        let fams = cat["families"].as_object().expect("families");
        assert!(fams.len() >= 8, "上架族数异常：{}", fams.len());
        for (id, f) in fams {
            if f["implemented"] == serde_json::json!(false) {
                continue;
            }
            let views: Vec<String> = f["views"]
                .as_array()
                .unwrap_or_else(|| panic!("{id} 缺 views"))
                .iter()
                .map(|v| v["id"].as_str().unwrap().to_string())
                .collect();
            assert!(!views.is_empty(), "{id} 上架却没有视图（GUI 会静默失效）");
            assert_eq!(
                views,
                crate::partgen_more::family_views(id),
                "{id} 的 views 与视图注册表不一致"
            );
            // 结构要素（detail_*）：**自由输入 d**、没有固定规格下拉；用目录里
            // 族自报的示例（带 params）或第一档区间中点试生成，不走下面的 sizes/lengths 校验。
            if crate::detail::is_detail(id) {
                let (d, params) = detail_sample(f);
                for v in &views {
                    crate::detail::generate_params(id, d, &params, v)
                        .unwrap_or_else(|e| panic!("{id} d={d} {v} 生成失败: {e}"));
                }
                continue;
            }
            let sizes = f["sizes"].as_array().unwrap_or_else(|| panic!("{id} 缺 sizes"));
            assert!(!sizes.is_empty(), "{id} 没有规格");
            let d = sizes[0]["d"].as_f64().unwrap();
            let l = sizes[0]["lengths"][0].as_f64().unwrap();
            for v in &views {
                generate(id, d, l, v)
                    .unwrap_or_else(|e| panic!("{id} Ø{d}×{l} {v} 生成失败: {e}"));
            }
            // 视图名齐全（GUI 按钮文案）
            for v in f["views"].as_array().unwrap() {
                assert!(!v["name"].as_str().unwrap_or("").is_empty(), "{id} 视图缺 name");
            }
        }
        // 树上：每个已上架族的叶子必须挂上 family 且名字与 families 一致
        // （第四批起树上叶子由各族自己的 `tree_path` 动态插入，见 catalog_json）
        let tree = cat["tree"].to_string();
        for needle in [
            "六角螺母 C级 GB/T 41-2016",
            "六角薄螺母 GB/T 6172.1-2016",
            "1型六角螺母 GB/T 6170-2015",
            "nut_c41",
            "nut_61721",
            "nut_6170",
            // 第四批 11 族
            "ring_893",
            "ring_894",
            "set_screw_77",
            "round_nut_812",
            "lock_washer_858",
            "eye_bolt_825",
            "seal_fb",
            "bearing_276",
            "bearing_297",
            "bearing_288",
            // 结构要素（第一期磨外圆 + 第二期外螺纹退刀槽）
            "结构要素",
            "detail_grind_od",
            "磨外圆 GB/T 6403.5-2008",
            "detail_thread_relief",
            "外螺纹退刀槽 GB/T 3-1997",
        ] {
            assert!(tree.contains(needle), "树里缺 {needle}");
        }
    }

    /// 目录里结构要素的试生成样本：优先用族自报的 `sample`（新要素带参数，
    /// 如退刀槽 `{"d":20,"P":1.5}`）；否则回落到第一档区间中点（磨外圆只有 b1）。
    fn detail_sample(f: &serde_json::Value) -> (f64, crate::detail::DetailParams) {
        if let Some(sample) = f.get("sample").and_then(|s| s.as_object()) {
            let d = sample.get("d").and_then(|d| d.as_f64());
            if let Some(d) = d {
                let mut params = crate::detail::DetailParams::new();
                for (key, value) in sample {
                    if key == "d" {
                        continue;
                    }
                    if key == "spec" {
                        if let Some(spec) = value.as_str() {
                            params.set_spec(spec);
                        }
                        continue;
                    }
                    if let Some(value) = value.as_f64() {
                        params.insert(key, value);
                    }
                }
                return (d, params);
            }
        }
        let band = &f["bands"][0];
        let lo = band["lo"].as_f64().unwrap_or(1.0);
        let d = match band["hi"].as_f64() {
            Some(hi) if hi.is_finite() => (lo.max(0.0) + hi) / 2.0,
            _ => lo.max(1.0) * 2.0,
        };
        (d, crate::detail::DetailParams::new())
    }

    /// 螺栓 ≠ 螺钉：两类东西，各有各的 kind 与树位置（用户 2026-09-16 明确）。
    ///
    /// 螺钉：自带头部、直接拧入螺纹孔、不配螺母 → `screw`，且在树上是独立顶级类「螺钉」；
    /// 螺栓：穿孔 + 螺母 → `bolt`，进件链（`joint`）的只有这一类。
    #[test]
    fn bolts_and_screws_are_different_kinds() {
        for fam in ["hex_bolt_c", "hex_bolt_ab", "hex_bolt_b_full", "hex_bolt_hole_a"] {
            assert_eq!(family_kind(fam), "bolt", "{fam} 应是螺栓");
        }
        for fam in [
            "socket_head",
            "socket_button_702",
            "socket_torx_2671",
            "set_screw_77",
            "eye_bolt_825",
        ] {
            assert_eq!(family_kind(fam), "screw", "{fam} 应是螺钉（与螺栓不同类）");
        }
        // 内六角圆柱头螺钉必须在「螺钉」支下，且不在「螺栓」支下
        let cat: serde_json::Value = serde_json::from_str(&catalog_json()).expect("目录 JSON");
        let mut found: Option<(String, Vec<String>)> = None;
        fn walk(node: &serde_json::Value, path: &mut Vec<String>, found: &mut Option<(String, Vec<String>)>) {
            if let Some(arr) = node.as_array() {
                for n in arr {
                    walk(n, path, found);
                }
                return;
            }
            if let Some(name) = node.get("name").and_then(|n| n.as_str()) {
                path.push(name.to_string());
            }
            if node.get("family").and_then(|f| f.as_str()) == Some("socket_head") {
                *found = Some(("socket_head".to_string(), path.clone()));
            }
            if let Some(kids) = node.get("children") {
                walk(kids, path, found);
            }
            if node.get("name").is_some() {
                path.pop();
            }
        }
        walk(&cat["tree"], &mut Vec::new(), &mut found);
        let (_, path) = found.expect("树上必须有 socket_head");
        let joined = path.join("/");
        assert!(
            path.contains(&"螺钉".to_string()),
            "内六角圆柱头螺钉应在「螺钉」支下，实际: {joined}"
        );
        assert!(
            !path.contains(&"螺栓".to_string()),
            "内六角圆柱头螺钉不该还在「螺栓」支下，实际: {joined}"
        );
    }

    /// 零件树叶子动态插入器：同名目录复用、缺目录新建、重复插幂等。
    ///
    /// 第四批 11 族都由各模块声明 `tree_path` 自动上树；这个测试守住插入器的三个行为。
    #[test]
    fn tree_leaf_insert_is_nested_and_idempotent() {
        let mut tree = serde_json::json!([
            { "name": "零件库", "children": [
                { "name": "挡圈", "children": [] }
            ]}
        ]);
        // 1) 已有目录下插叶子
        insert_tree_leaf(&mut tree, "零件库/挡圈/孔用弹性挡圈 A型 GB/T 893-2017", "ring_893", true);
        // 2) 新目录（密封件）自动建
        insert_tree_leaf(&mut tree, "零件库/密封件/密封圈 FB型 GB/T 13871.1-2007", "seal_fb", true);
        // 3) 幂等：同族再插一次不会重复
        insert_tree_leaf(&mut tree, "零件库/挡圈/孔用弹性挡圈 A型 GB/T 893-2017", "ring_893", true);
        let s = tree.to_string();
        assert!(s.contains("\"family\":\"ring_893\""), "挡圈叶子没插上: {s}");
        assert_eq!(s.matches("ring_893").count(), 1, "重复插入: {s}");
        assert!(s.contains("\"name\":\"密封件\""), "中间目录没自动建: {s}");
        assert!(s.contains("\"family\":\"seal_fb\""), "密封圈叶子没插上: {s}");
        // 4) 树上没有的路径（根不是数组/没有 children）不该 panic
        let mut weird = serde_json::json!({ "name": "零件库" });
        insert_tree_leaf(&mut weird, "零件库/挡圈/X", "ring_893", true);
    }

    /// PART/XL 报错随语言切换，关键数据（M8/表号/视图名）原样保留。
    #[test]
    fn parts_messages_switch_language_keeping_data() {
        let _g = zh_guard();
        crate::i18n::clear_missing_keys();

        let err = hex_bolt(8.0, 5.0, false).unwrap_err();
        assert!(err.contains("M8") && err.contains("长度") && err.contains("30"), "{err}");
        let err = hex_bolt_c(8.0, 5.0, BoltView::Main).unwrap_err();
        assert!(err.contains("M8") && err.contains("长度") && err.contains("80"), "{err}");
        let err = generate("hex_bolt_c", 8.0, 30.0, "bad").unwrap_err();
        assert!(err.contains("hex_bolt_c") && err.contains("bad") && err.contains("视图"), "{err}");
        let err = preview_svg("family=hex_bolt_c&d=8").unwrap_err();
        assert!(err.contains("l"), "{err}");
        let err = preview_svg("family=hex_bolt_c&d=x&l=30").unwrap_err();
        assert!(err.contains("d") && err.contains("数字"), "{err}");
        let err = crate::partgen_kit::check_length(20.0, 200.0, 8.0, 5.0).unwrap_err();
        assert!(err.contains("M8") && err.contains("20") && err.contains("200"), "{err}");

        crate::i18n::set_lang(crate::i18n::Lang::En);
        let err = hex_bolt(8.0, 5.0, false).unwrap_err();
        assert!(err.contains("M8") && err.contains("length") && err.contains("30"), "{err}");
        assert!(!err.contains("长度"), "{err}");
        let err = hex_bolt_c(8.0, 5.0, BoltView::Main).unwrap_err();
        assert!(err.contains("M8") && err.contains("between") && err.contains("80"), "{err}");
        let err = generate("hex_bolt_c", 8.0, 30.0, "bad").unwrap_err();
        assert!(err.contains("hex_bolt_c") && err.contains("bad") && err.contains("does not provide view"), "{err}");
        let err = preview_svg("family=hex_bolt_c&d=8").unwrap_err();
        assert!(err.contains("missing parameter l"), "{err}");
        let err = preview_svg("family=hex_bolt_c&d=x&l=30").unwrap_err();
        assert!(err.contains("d") && err.contains("not a number"), "{err}");
        let err = crate::partgen_kit::check_length(20.0, 200.0, 8.0, 5.0).unwrap_err();
        assert!(err.contains("M8") && err.contains("20") && err.contains("200") && err.contains("range"), "{err}");

        assert!(crate::i18n::missing_keys().is_empty(), "缺词条：{:?}", crate::i18n::missing_keys());
        crate::i18n::set_lang_auto();
    }

    /// ⑥ 批：GUI 目录 JSON 的深层元数据（族名/基点/长度标签/树目录/来源）**随语言**且非空。
    #[test]
    fn catalog_json_metadata_switches_language_and_stays_nonempty() {
        let _g = zh_guard();
        crate::i18n::clear_missing_keys();
        let zh: serde_json::Value = serde_json::from_str(&catalog_json()).expect("zh catalog json");
        crate::i18n::set_lang(crate::i18n::Lang::En);
        let en: serde_json::Value = serde_json::from_str(&catalog_json()).expect("en catalog json");
        crate::i18n::set_lang_auto();
        // 结构要素（detail 族）：四个元数据字段逐个 zh/en 双断言
        for f in ["name", "len_label", "base_hint", "tree_dir"] {
            let z = zh["families"]["detail_grind_od"][f].as_str().unwrap_or_default();
            let e = en["families"]["detail_grind_od"][f].as_str().unwrap_or_default();
            assert!(!z.is_empty() && !e.is_empty(), "{f} 空：{z:?} / {e:?}");
            assert_ne!(z, e, "{f} 未随语言：{z:?}");
            assert!(
                !e.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
                "{f} 英文里仍含汉字：{e:?}"
            );
        }
        // 标准件族：族名 / 长度标签 / 基点
        for f in ["name", "len_label", "base_hint"] {
            let z = zh["families"]["hex_bolt_c"][f].as_str().unwrap_or_default();
            let e = en["families"]["hex_bolt_c"][f].as_str().unwrap_or_default();
            assert!(!z.is_empty() && !e.is_empty() && z != e, "{f}：{z:?} / {e:?}");
        }
        // 树（含 `族名 + 代号` 叶子）也随语言；**代号/族 id 不译**
        let zt = zh["tree"].to_string();
        let et = en["tree"].to_string();
        assert!(zt.contains("六角头螺栓 C级 GB/T 5780-2016"), "{zt}");
        assert!(et.contains("Hex head bolt, grade C GB/T 5780-2016"), "{et}");
        assert!(et.contains("hex_bolt_c") && et.contains("GB/T 5780-2016"));
        // 键型下拉元数据（key_styles）同样随语言；`id`（协议别名）**不译**，只查 label/place_hint
        let zk = zh["key_styles"].to_string();
        let ek = en["key_styles"].to_string();
        assert!(!zk.is_empty() && !ek.is_empty() && zk != ek, "key_styles 未随语言");
        let has_cjk = |s: &str| s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
        for item in en["key_styles"].as_array().expect("key_styles 数组") {
            for f in ["label", "place_hint"] {
                let v = item[f].as_str().unwrap_or_default();
                assert!(!has_cjk(v), "key_styles.{f} 英文里仍含汉字：{v:?}");
            }
        }
        // `id`（协议别名，如 `A`/`导向A`）两语必须**逐项相同**
        let ids = |v: &serde_json::Value| -> Vec<String> {
            v["key_styles"]
                .as_array()
                .unwrap()
                .iter()
                .map(|it| it["id"].as_str().unwrap_or_default().to_string())
                .collect()
        };
        assert_eq!(ids(&zh), ids(&en));
        // ⑥ 收尾：`spline_engine` 引擎说明串（source/din_notes/nf_notes/ansi_notes）随语言，
        // 协议/数据记号（标准代号、行数、文件名、公式符号）两语一致。
        for f in ["source", "din_notes", "nf_notes", "ansi_notes"] {
            let z = zh["spline_engine"][f].as_str().unwrap_or_default();
            let e = en["spline_engine"][f].as_str().unwrap_or_default();
            assert!(!z.is_empty() && !e.is_empty(), "spline_engine.{f} 空：{z:?} / {e:?}");
            assert_ne!(z, e, "spline_engine.{f} 未随语言：{z:?}");
            assert!(!has_cjk(e), "spline_engine.{f} 英文里仍含汉字：{e:?}");
        }
        let esrc = en["spline_engine"]["source"].as_str().unwrap_or_default();
        assert!(
            esrc.contains("GB/T 3478.1-2008") && esrc.contains("DIN 5480-1:2015"),
            "spline_engine.source 丢了标准代号：{esrc}"
        );
        let ean = en["spline_engine"]["ansi_notes"].as_str().unwrap_or_default();
        assert!(
            ean.contains("assets/ansi_b921_notes.md") && ean.contains("m=25.4/P"),
            "spline_engine.ansi_notes 丢了文件名/公式：{ean}"
        );
        assert!(crate::i18n::missing_keys().is_empty(), "{:?}", crate::i18n::missing_keys());
    }

    /// ② GUI 元数据批：**全量**目录 JSON（families + tree）里白名单字段（`DATA_JSON_FIELDS`）
    /// 在英文下**逐条无汉字**且 zh/en 都非空；族 id/代号/文件名（`DATA_KEEP_AS_IS`）例外。
    #[test]
    fn catalog_json_all_whitelisted_metadata_has_no_cjk_in_english() {
        let _g = zh_guard();
        crate::i18n::clear_missing_keys();
        let zh: serde_json::Value = serde_json::from_str(&catalog_json()).expect("zh catalog json");
        crate::i18n::set_lang(crate::i18n::Lang::En);
        let en: serde_json::Value = serde_json::from_str(&catalog_json()).expect("en catalog json");
        crate::i18n::set_lang_auto();
        let has_cjk = |s: &str| s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
        let fields = [
            "name", "base_hint", "len_label", "d_label", "tree_dir", "hint", "place_hint",
            "label", "source", "display", "desc",
        ];
        // `DATA_KEEP_AS_IS`（文件名/路径 —— 翻了就对不上磁盘）允许保留中文
        let keep = ["磨外圆_GB-T6403.png + 磨外圆_GB-T6403.5-2008.dxf"];
        // 译文里**必须原样保留的文件名**（模板 dxf/表格 csv）——查汉字前先剥掉
        let keep_names = ["毂槽-{主视图,侧视图}.dxf", "矩形花键.dxf"];
        let mut checked = 0usize;
        let mut offenders: Vec<String> = Vec::new();
        fn walk(
            z: &serde_json::Value,
            e: &serde_json::Value,
            path: &str,
            on: &mut dyn FnMut(&str, &str, &str),
        ) {
            match (z, e) {
                (serde_json::Value::Object(zm), serde_json::Value::Object(em)) => {
                    for (k, zv) in zm {
                        let Some(ev) = em.get(k) else { continue };
                        let p = format!("{path}.{k}");
                        if let (Some(zs), Some(es)) = (zv.as_str(), ev.as_str()) {
                            on(&p, zs, es);
                        }
                        walk(zv, ev, &p, on);
                    }
                }
                (serde_json::Value::Array(za), serde_json::Value::Array(ea)) => {
                    for (i, (zv, ev)) in za.iter().zip(ea.iter()).enumerate() {
                        walk(zv, ev, &format!("{path}[{i}]"), on);
                    }
                }
                _ => {}
            }
        }
        fn walk_tree(
            z: &serde_json::Value,
            e: &serde_json::Value,
            on: &mut dyn FnMut(&str, &str, &str),
        ) {
            if let (Some(zn), Some(en)) = (z["name"].as_str(), e["name"].as_str()) {
                on("tree", zn, en);
            }
            if let (Some(zc), Some(ec)) = (z["children"].as_array(), e["children"].as_array()) {
                for (zv, ev) in zc.iter().zip(ec.iter()) {
                    walk_tree(zv, ev, on);
                }
            }
        }
        {
            let mut on = |p: &str, zs: &str, es: &str| {
                let f = p.rsplit('.').next().unwrap_or("");
                if !fields.contains(&f) {
                    return;
                }
                // 规格代号（`specs[].label`，如 `中10x72x82x12`）是协议数据，不译
                if p.contains(".specs[") || zs.starts_with("用户 OCR 核对表") {
                    return;
                }
                checked += 1;
                assert!(!zs.is_empty() && !es.is_empty(), "{p} 空：{zs:?} / {es:?}");
                let mut probe = es.to_string();
                for name in keep_names {
                    probe = probe.replace(name, "");
                }
                if has_cjk(&probe) && !keep.contains(&zs) {
                    offenders.push(format!("{p}: {es:?}"));
                }
            };
            for (fam, obj) in zh["families"].as_object().unwrap() {
                let eobj = &en["families"][fam];
                walk(obj, eobj, fam, &mut on);
                // 族 id / 代号不译
                assert_eq!(obj["id"], eobj["id"], "{fam} id 不译");
                assert_eq!(obj["code"], eobj["code"], "{fam} code 不译");
            }
        }
        {
            let mut on = |p: &str, zs: &str, es: &str| {
                checked += 1;
                if has_cjk(es) && !keep.contains(&zs) {
                    offenders.push(format!("{p}: {es:?}"));
                }
            };
            walk_tree(&zh["tree"], &en["tree"], &mut on);
        }
        assert!(checked > 60, "扫过的元数据字段太少（{checked}）——白名单/结构变了？");
        assert!(
            offenders.is_empty(),
            "英文目录仍含汉字 {} 条：\n{}",
            offenders.len(),
            offenders.join("\n")
        );
    }
}

#[cfg(test)]
pub(crate) mod acceptance_dump {
    //! 出"参数化验收图"：用**与插件插入完全相同的生成代码**建块 + INSERT 并写盘，
    //! 供在 OCS 里打开检查画法（这是发现"弧线单位"这类宿主渲染问题最有效的方式）。
    //!
    //! `cargo test -p ocs_ocsm dump_acceptance_drawing -- --ignored --nocapture`

    use super::*;
    use ocs_plugin_api::host::acadrust::entities::{Block, BlockEnd, Insert};
    use ocs_plugin_api::host::acadrust::io::dxf::DxfWriter;
    use ocs_plugin_api::host::acadrust::io::dwg::DwgWriter;
    use ocs_plugin_api::host::acadrust::tables::{BlockRecord, Layer, LineType, LineTypeElement};
    use ocs_plugin_api::host::acadrust::xdata::{ExtendedDataRecord, XDataValue};
    use ocs_plugin_api::host::acadrust::CadDocument;

    /// 建块（与宿主 `AddBlockRecord` 同构：BlockRecord → Block → 成员 → BlockEnd）。
    pub(crate) fn add_block(doc: &mut CadDocument, name: &str, entities: Vec<EntityType>) {
        let mut br = BlockRecord::new(name);
        br.handle = doc.allocate_handle();
        let mut block = Block::new(name, Vector3::new(0.0, 0.0, 0.0));
        block.common.handle = doc.allocate_handle();
        block.common.owner_handle = br.handle;
        br.block_entity_handle = block.common.handle;
        doc.add_entity(EntityType::Block(block)).expect("block");
        let br_handle = br.handle;
        doc.block_records.add(br).expect("block record");
        let mut members = Vec::new();
        for mut e in entities {
            e.common_mut().owner_handle = br_handle;
            members.push(doc.add_entity(e).expect("block member"));
        }
        let mut end = BlockEnd::new();
        end.common.handle = doc.allocate_handle();
        end.common.owner_handle = br_handle;
        let end_handle = end.common.handle;
        doc.add_entity(EntityType::BlockEnd(end)).expect("block end");
        if let Some(b) = doc.block_records.get_mut(name) {
            b.entity_handles = members;
            b.block_end_handle = end_handle;
        }
    }

    /// 图层 + 线型（与插件运行时 `ensure_layers/ensure_linetypes` 同源）。
    ///
    /// 注意：**必须给 LTYPE/LAYER 分配句柄**，否则 DxfWriter 会写出 `handle=0`，
    /// ezdxf 等严格读取器会报 `Invalid handle 0`（验收 DXF 打不开）。
    pub(crate) fn add_ocsm_layers(doc: &mut CadDocument) {
        for def in crate::linetype_defs() {
            let mut lt = LineType::new(&def.name);
            lt.handle = doc.allocate_handle();
            lt.description = def.description.clone();
            lt.elements = def
                .elements
                .iter()
                .map(|v| LineTypeElement { length: *v, complex: None })
                .collect();
            lt.pattern_length = lt.elements.iter().map(|e| e.length.abs()).sum();
            doc.line_types.add_or_replace(lt);
        }
        for def in crate::layer_defs() {
            let mut l = Layer::new(&def.name);
            l.handle = doc.allocate_handle();
            l.color = def.color;
            l.line_type = def.linetype.clone();
            l.line_weight = def.lineweight;
            doc.layers.add_or_replace(l);
        }
    }

    /// GB/T 5782 A/B 级三视图（带头部垫圈面 dw×c）→ SVG + DXF，供看图/开 OCS 核对。
    ///
    /// `cargo test -p ocs_ocsm -- --ignored dump_washer_face_draft --nocapture`
    #[test]
    #[ignore]
    fn dump_washer_face_draft() {
        use ocs_plugin_api::host::acadrust::io::dxf::DxfWriter;
        use ocs_plugin_api::host::acadrust::CadDocument;
        let dir = std::path::Path::new("/home/ysdirector/桌面/OCSM/test/模板草案-5782垫圈面");
        std::fs::create_dir_all(dir).unwrap();
        for (d, l, tag) in [(10.0, 60.0, "M10x60"), (42.0, 200.0, "M42x200")] {
            for (view, vn) in [
                (BoltView::Main, "主视图"),
                (BoltView::Top, "俯视图"),
                (BoltView::End, "左视图"),
            ] {
                let part = hex_bolt_ab(d, l, view).unwrap();
                // SVG（预览）
                let svg = to_svg(&part, &format!("GB/T 5782-2016 {} {} 带垫圈面", part.meta.spec, vn), 760.0, 420.0);
                let f = dir.join(format!("草稿-{tag}-{vn}.svg"));
                std::fs::write(&f, svg).unwrap();
                // DXF（落图实体，供 OCS 打开核图）
                let mut doc = CadDocument::new();
                add_ocsm_layers(&mut doc);
                for e in part.entities.clone() {
                    doc.add_entity(e).expect("图元入库");
                }
                let f2 = dir.join(format!("草稿-{tag}-{vn}.dxf"));
                DxfWriter::new(&doc).write_to_file(&f2).expect("写 DXF");
                println!("写出 {} / {}", f.display(), f2.display());
            }
        }
        // 与 C 级（无垫圈面）同规格对比：主/俯视图各一份，供叠合差异自查
        for (part, suffix) in [
            (hex_bolt_c(10.0, 60.0, BoltView::Main).unwrap(), "C级-主视图"),
            (hex_bolt_c(10.0, 60.0, BoltView::Top).unwrap(), "C级-俯视图"),
            (hex_bolt_ab(10.0, 60.0, BoltView::Main).unwrap(), "A-B级-主视图"),
            (hex_bolt_ab(10.0, 60.0, BoltView::Top).unwrap(), "A-B级-俯视图"),
        ] {
            let mut doc = CadDocument::new();
            add_ocsm_layers(&mut doc);
            for e in part.entities.clone() {
                doc.add_entity(e).expect("图元入库");
            }
            let f = dir.join(format!("对比-M10x60-{suffix}.dxf"));
            DxfWriter::new(&doc).write_to_file(&f).expect("写 DXF");
            println!("写出 {}", f.display());
        }
    }

    #[test]
    #[ignore]
    fn dump_acceptance_drawing() {
        use BoltView::*;
        // (直径, 长度, 视图, 插入点x, 插入点y)
        let cases: &[(f64, f64, BoltView, f64, f64)] = &[
            (5.0, 25.0, Main, 20.0, 200.0),
            (10.0, 45.0, Main, 80.0, 200.0),
            (16.0, 65.0, Main, 150.0, 200.0),
            (10.0, 45.0, Top, 250.0, 200.0),
            (5.0, 25.0, Top, 20.0, 140.0),
            (5.0, 25.0, End, 80.0, 140.0),
            (24.0, 100.0, Main, 140.0, 140.0),
            (8.0, 40.0, Main, 20.0, 80.0),
            (16.0, 65.0, End, 100.0, 80.0),
            (24.0, 100.0, End, 160.0, 80.0),
        ];

        let mut doc = CadDocument::new();
        add_ocsm_layers(&mut doc);
        let mut lines = Vec::new();
        for (d, l, view, x, y) in cases {
            let part = hex_bolt_c(*d, *l, *view).expect("生成");
            let vname = match view {
                Main => "main",
                Top => "top",
                End => "end",
            };
            let block = format!(
                "OCSM_HEX_BOLT_C_{}_{}",
                part.meta.spec.replace(['.', ' ', '/', 'x', 'X'], "_"),
                vname.to_ascii_uppercase()
            );
            add_block(&mut doc, &block, part.entities.clone());
            let mut ins = Insert::new(&block, Vector3::new(*x, *y, 0.0));
            ins.common.layer = LAYER_MAIN.to_string();
            ins.common.color = ocs_plugin_api::host::acadrust::types::Color::ByLayer;
            ins.common.linetype = "ByLayer".to_string();
            ins.common.line_weight = ocs_plugin_api::host::acadrust::types::LineWeight::ByLayer;
            let mut rec = ExtendedDataRecord::new("OCSM_PART");
            rec.values.push(XDataValue::String(
                serde_json::json!({
                    "code": part.meta.code, "name": part.meta.name, "spec": part.meta.spec,
                    "weight": part.meta.weight, "d": d, "l": l, "view": vname,
                })
                .to_string(),
            ));
            ins.common.extended_data.add_record(rec);
            doc.add_entity(EntityType::Insert(ins)).expect("insert");
            lines.push(format!(
                "{:>6} {:>4}  {:>4}  @({x:.0},{y:.0})  块 {block}",
                part.meta.spec,
                vname,
                part.entities.len()
            ));
        }

        let dir = std::path::Path::new("/home/ysdirector/桌面/OCSM/test");
        std::fs::create_dir_all(dir).unwrap();
        let dwg = dir.join("参数化验收-六角头螺栓C级.dwg");
        let dxf = dir.join("参数化验收-六角头螺栓C级.dxf");
        DwgWriter::write_to_file(&dwg, &doc).expect("写 DWG");
        DxfWriter::new(&doc).write_to_file(&dxf).expect("写 DXF");
        println!("已写出：\n  {}\n  {}", dwg.display(), dxf.display());
        println!("内容：{} 个零件（块 + INSERT + OCSM_PART 记录）", lines.len());
        for l in &lines {
            println!("  {l}");
        }
    }
}

#[cfg(test)]
mod json_dump {
    use super::*;
    use ocs_plugin_api::host::acadrust::EntityType;

    /// 把 M5x25 主/俯视图的图元导出成 JSON（供与用户规范 DXF 叠加对比）。
    #[test]
    #[ignore]
    fn dump_bolt_json() {
        let mut all = Vec::new();
        for (name, v) in [("main", super::BoltView::Main), ("top", super::BoltView::Top), ("end", super::BoltView::End)] {
            let p = super::hex_bolt_c(5.0, 25.0, v).unwrap();
            let (mut lines, mut arcs, mut circles) = (Vec::new(), Vec::new(), Vec::new());
            for e in &p.entities {
                match e {
                    EntityType::Line(l) => lines.push(vec![l.start.x, l.start.y, l.end.x, l.end.y]),
                    EntityType::Arc(a) => arcs.push(vec![
                        a.center.x,
                        a.center.y,
                        a.radius,
                        a.start_angle.to_degrees(),
                        a.end_angle.to_degrees(),
                    ]),
                    EntityType::Circle(c) => circles.push(vec![c.center.x, c.center.y, c.radius]),
                _ => {}
                    _ => {}
                }
            }
            all.push(serde_json::json!({
                "name": name, "lines": lines, "arcs": arcs, "circles": circles
            }));
        }
        let out = serde_json::json!({ "views": all }).to_string();
        let f = std::path::Path::new("/tmp/partgen");
        std::fs::create_dir_all(f).unwrap();
        std::fs::write(f.join("mine.json"), out).unwrap();
        println!("已写出 /tmp/partgen/mine.json");
    }


}
