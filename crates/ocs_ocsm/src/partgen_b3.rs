//! 第四批 C 组（partgen_b3）：**吊环螺钉 A型 GB/T 825-1988**、
//! **内包骨架有副唇密封圈 FB型 GB/T 13871.1-2007**、**深沟球轴承 60000型 GB/T 276-2013**。
//!
//! 本文件只放这三个族的画法与数据；共用图元/表/校验工具在 `partgen_kit.rs`。
//! **不要改其它模块**——并行开发时会互相踩。
//!
//! | 族 id | 名称 | 模板（用户参数化图 DXF） | 视图 | 数据 |
//! |---|---|---|---|---|
//! | `eye_bolt_825` | 吊环螺钉 A型 | `~/桌面/GB/参数化/吊环螺钉_A型_GB-T825-1988/`（主/左） | main/end | 用户 PNG 参数表（O. d D1 h1 d4 h r1 a l d2 d1 b） |
//! | `seal_fb` | 内包骨架有副唇密封圈 FB型 | `~/桌面/GB/参数化/内包骨架有副唇密封圈_FB型_GB-T13871.1-2007/`（主） | main | 易紧通 info_399381 |
//! | `bearing_276` | 深沟球轴承 60000型 | `~/桌面/GB/参数化/深沟球轴承_60000型_GB-T276-2013/`（主，参数 d/D/B/r） | main | mechtool.cn GB/T 276—2013 表 + 用户 PNG 核对 |
//!
//! # 画法（全部由模板 DXF 逐条反解，模板规格已作数值回归）
//!
//! 反解工具：`target/debug/examples/ref_dump <dxf> <out.tsv>`（圆弧角度为**度**）。
//! **重要**：模板下半个视/镜像侧的图元在 DXF 里用 OCS（extrusion=(0,0,−1)）存储，
//! ref_dump 只按存储坐标输出（看起来像点对称），本模块统一按**真正的镜像**生成
//! （`mirror_x` 关于轴线、`mirror_y` 关于轴线），叠合测试把模板的对应半视镜像后比对。
//!
//! ## eye_bolt_825（模板 M100：d100 D1 200 d4 350 h175 h1 62 r1 40 a12 l140 d2 204.2 d1 79.2 b88）
//! - 主视图：环外弧 R=d4/2、环孔 D1/2（心 (0,h)）；`r1` 圆角把环接到支承面，圆角心
//!   `(d2/2+r1, h−√((d4/2+r1)²−(d2/2+r1)²))`，切点在环上角 `360°−φ`（φ=atan2(h−cy,cx)）；
//!   支承面 y=0 至 d2/2；杆 d/2、螺纹细线 0.85d/2、收尾 `(d/2,−a/2)→(0.85d/2,−a)`、
//!   端部 45° 倒角 `0.075d`；另有 h1 处与 `(0,(d4−D1)/2)→(0.7d,h1)→(0.8d,0.9h1)` 两条基座线。
//! - 左视图：环以小圆（Ød1）表示，圆心高 `h+(d4/2−d1/2)`（下半 Ø 边虚线）；环侧线
//!   `(d1/2,h+·)→(b/2,0.54b)→(0.6b,h−·)`；基座 `(d2/2,0)→(d2/2−a,h1)`。
//!
//! ## seal_fb（模板 D30×d16×b7）
//! - 主视图 = **轴向剖视**（轴线 x=0，y 为轴向 0…b；两侧对称）。剖面轮廓（金属骨架 +
//!   橡胶体）在归一坐标 `u=(x−r)/(R−r)`、`v=y/b` 下逐条复刻（下表常量），镜像得另一侧；
//!   另画两处弹簧小圆与中心线。**剖面线**（模板 4 片 ANSI31）：金属骨架左半 angle 0°/scale 1.0、
//!   右半 angle 90°/scale 0.25；橡胶体左右各 1 片 angle 0°/scale 0.25（唇口含 r=0.1b 圆弧）。
//!
//! ## bearing_276（模板 61807：d35 D47 B7 r0.3）
//! - 主视图 = **轴向剖视**（轴线 y=0，x 为轴向 0…B、y 为半径；上半个视镜像得下半）。
//!   外径 D/2（两端圆角 r）、内孔 d/2；球心半径 `(d+D)/4`、球半径 **`(D−d)/8`**
//!   （模板实测比例；表里的 Dw 只作记录，不用于画法）；内/外圈挡面半径 `球心 ∓ 球半径/2`，
//!   滚道弧 = 球圆在挡面之间的一段（±30°…±150°）。

use std::sync::OnceLock;

use ocs_plugin_api::host::acadrust::entities::EntityType;

use crate::partgen::{GenPart, PartMeta};
use crate::partgen_kit::{
    arc, circle, hatch_ansi31_edges, hatch_ansi31_scaled, hatch_ansi37_edges, line, polyline, trim,
    views_json,
    HatchEdge, Table, LAYER_CENTER, LAYER_HIDDEN, LAYER_MAIN, LAYER_THIN,
};

// ══════════════════════════════════════════════════════════════════════════
// 镜像工具（模板用 OCS 存的半视，这里统一生成真正的镜像）
// ══════════════════════════════════════════════════════════════════════════

/// 关于 x=0（竖直轴线）镜像：直线 x→−x；圆/圆心 x→−x；弧角 θ→180°−θ 且起止互换。
fn mirror_x(e: &EntityType) -> EntityType {
    match e {
        EntityType::Line(l) => line(
            [-l.start.x, l.start.y],
            [-l.end.x, l.end.y],
            &l.common.layer,
        ),
        EntityType::Arc(a) => arc(
            [-a.center.x, a.center.y],
            a.radius,
            180.0 - a.end_angle.to_degrees(),
            180.0 - a.start_angle.to_degrees(),
            &a.common.layer,
        ),
        EntityType::Circle(c) => circle([-c.center.x, c.center.y], c.radius, &c.common.layer),
        EntityType::LwPolyline(pl) => {
            let pts: Vec<[f64; 2]> = pl.vertices.iter().map(|v| [-v.location.x, v.location.y]).collect();
            polyline(&pts, pl.is_closed, &pl.common.layer)
        }
        other => other.clone(),
    }
}

/// 关于 y=0（水平轴线）镜像：直线/圆 y→−y；弧角 θ→−θ 且起止互换。
fn mirror_y(e: &EntityType) -> EntityType {
    match e {
        EntityType::Line(l) => line(
            [l.start.x, -l.start.y],
            [l.end.x, -l.end.y],
            &l.common.layer,
        ),
        EntityType::Arc(a) => arc(
            [a.center.x, -a.center.y],
            a.radius,
            -a.end_angle.to_degrees(),
            -a.start_angle.to_degrees(),
            &a.common.layer,
        ),
        EntityType::Circle(c) => circle([c.center.x, -c.center.y], c.radius, &c.common.layer),
        EntityType::LwPolyline(pl) => {
            let pts: Vec<[f64; 2]> = pl.vertices.iter().map(|v| [v.location.x, -v.location.y]).collect();
            polyline(&pts, pl.is_closed, &pl.common.layer)
        }
        other => other.clone(),
    }
}

/// 加原件 + 关于 x=0 的镜像。
fn push_x(en: &mut Vec<EntityType>, e: EntityType) {
    let m = mirror_x(&e);
    en.push(e);
    en.push(m);
}

/// 加原件 + 关于 y=0 的镜像。
fn push_y(en: &mut Vec<EntityType>, e: EntityType) {
    let m = mirror_y(&e);
    en.push(e);
    en.push(m);
}

/// 关于 x=0 镜像一条剖面线边界（与 `mirror_x` 同规则；圆弧统一重表为正向区间）。
fn mirror_hatch_x(edges: &[HatchEdge]) -> Vec<HatchEdge> {
    edges
        .iter()
        .map(|e| match *e {
            HatchEdge::Line { a, b } => HatchEdge::Line {
                a: [-a[0], a[1]],
                b: [-b[0], b[1]],
            },
            HatchEdge::Arc {
                c,
                r,
                start_deg,
                end_deg,
                ..
            } => {
                let span = (end_deg - start_deg).abs();
                let s = (180.0 - end_deg).rem_euclid(360.0);
                HatchEdge::Arc {
                    c: [-c[0], c[1]],
                    r,
                    start_deg: s,
                    end_deg: s + span,
                    ccw: true,
                }
            }
        })
        .collect()
}

/// 关于 y=0 镜像一条剖面线边界（与 `mirror_y` 同规则）。
fn mirror_hatch_y(edges: &[HatchEdge]) -> Vec<HatchEdge> {
    edges
        .iter()
        .map(|e| match *e {
            HatchEdge::Line { a, b } => HatchEdge::Line {
                a: [a[0], -a[1]],
                b: [b[0], -b[1]],
            },
            HatchEdge::Arc {
                c,
                r,
                start_deg,
                end_deg,
                ..
            } => {
                let span = (end_deg - start_deg).abs();
                let s = (-end_deg).rem_euclid(360.0);
                HatchEdge::Arc {
                    c: [c[0], -c[1]],
                    r,
                    start_deg: s,
                    end_deg: s + span,
                    ccw: true,
                }
            }
        })
        .collect()
}

// ══════════════════════════════════════════════════════════════════════════
// 数据表
// ══════════════════════════════════════════════════════════════════════════

/// GB/T 825-1988 吊环螺钉 A 型一行（列名 = 用户参数表/模板 MTEXT 参数名）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct EyeBoltRow {
    pub d: f64,
    #[serde(rename = "P")]
    pub pitch: f64,
    /// 环部内径（公称）
    #[serde(rename = "D1")]
    pub d_hole: f64,
    /// 环高（公称）
    pub h1: f64,
    /// 环外径（参考）
    pub d4: f64,
    /// 总高（支承面 → 环心）
    pub h: f64,
    /// 环部圆角
    pub r1: f64,
    /// 基座高
    pub a: f64,
    /// 螺纹长度（= 杆长）
    pub l: f64,
    /// 支承面直径（最大）
    pub d2: f64,
    /// 环部截面直径（最大）
    pub d1: f64,
    /// 基座宽度（对边）
    pub b: f64,
}

/// GB/T 13871.1-2007 FB 型密封圈一行。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SealRow {
    /// 轴径（内孔）
    pub d1: f64,
    #[serde(rename = "D")]
    pub od: f64,
    /// 宽度
    pub b: f64,
}

impl crate::partgen_kit::HasD for SealRow {
    fn d(&self) -> f64 {
        self.d1
    }
}

/// GB/T 276-2013 深沟球轴承 60000 型一行。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BearingRow {
    pub code: String,
    pub d: f64,
    #[serde(rename = "D")]
    pub od: f64,
    #[serde(rename = "B")]
    pub w: f64,
    #[serde(default, deserialize_with = "de_f64_or")]
    pub r: f64,
    /// 球径（表记录值；画法按模板实测比例 (D−d)/8，不直接用 Dw）
    #[serde(default)]
    #[allow(dead_code)]
    pub dw: Option<f64>,
    /// 球数（表记录值，仅供参考）
    #[serde(default)]
    #[allow(dead_code)]
    pub z: Option<f64>,
}

impl crate::partgen_kit::HasD for BearingRow {
    fn d(&self) -> f64 {
        self.d
    }
}

/// null/缺失的数值字段兜底（防别人表里留 null 让整个 crate 反序列化 panic）。
fn de_f64_or<'de, D>(d: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize as _;
    Ok(Option::<f64>::deserialize(d)?.unwrap_or(0.5))
}

fn eye_table() -> &'static Table<EyeBoltRow> {
    static T: OnceLock<Table<EyeBoltRow>> = OnceLock::new();
    T.get_or_init(|| Table::parse(include_str!("tables/partsEyeBolt825.json")))
}
fn seal_table() -> &'static Table<SealRow> {
    static T: OnceLock<Table<SealRow>> = OnceLock::new();
    T.get_or_init(|| Table::parse(include_str!("tables/partsSealFb.json")))
}
fn bearing_table() -> &'static Table<BearingRow> {
    static T: OnceLock<Table<BearingRow>> = OnceLock::new();
    T.get_or_init(|| Table::parse(include_str!("tables/partsBearing276.json")))
}

fn eye_row(d: f64) -> Option<&'static EyeBoltRow> {
    eye_table().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}
fn seal_row(d1: f64, od: f64) -> Option<&'static SealRow> {
    seal_table()
        .rows
        .iter()
        .find(|r| (r.d1 - d1).abs() < 1e-9 && (r.od - od).abs() < 1e-9)
}
fn bearing_row(d: f64, w: f64) -> Option<&'static BearingRow> {
    bearing_table()
        .rows
        .iter()
        .find(|r| (r.d - d).abs() < 1e-9 && (r.w - w).abs() < 1e-9)
}

// ══════════════════════════════════════════════════════════════════════════
// eye_bolt_825
// ══════════════════════════════════════════════════════════════════════════

/// 杆端倒角轴向长（= 大径 − 小径的半差；模板 7.5 = 0.075·d）。
fn eye_end_chamfer(d: f64) -> f64 {
    0.075 * d
}

fn eye_weight(row: &EyeBoltRow) -> String {
    let (r_out, r_in) = (row.d4 / 2.0, row.d_hole / 2.0);
    let rs = row.d / 2.0;
    let ring = std::f64::consts::PI * (r_out * r_out - r_in * r_in) * row.d1; // 环（近似）
    let shank = std::f64::consts::PI * rs * rs * row.l;
    let base = std::f64::consts::PI * (row.d2 / 2.0).powi(2) * (row.d4 / 2.0 - r_in).max(0.0);
    crate::partgen_kit::weight_text(crate::partgen_kit::weight_kg(ring + shank + base, 7.85))
}

/// 吊环螺钉主视图（轴线 x=0；环心 (0,h)；杆向 −y）。
pub fn eye_bolt_main(row: &EyeBoltRow) -> Result<GenPart, String> {
    let (d, d2, d4, h, h1, r1, a, l, d_hole) =
        (row.d, row.d2, row.d4, row.h, row.h1, row.r1, row.a, row.l, row.d_hole);
    let r_out = d4 / 2.0;
    let r_in = d_hole / 2.0;
    let r_base = d2 / 2.0;
    let rs = d / 2.0;
    let rfm = 0.85 * rs;
    // 圆角心：与环外圆外切、且与 x=d2/2 相切
    let cx = r_base + r1;
    let cy = h - ((r_out + r1).powi(2) - cx * cx).max(0.0).sqrt();
    let phi = (h - cy).atan2(cx); // 圆角→环心方向与 +x 的夹角（模板 48.629°）
    let phi_deg = phi.to_degrees();
    let end = l - eye_end_chamfer(d);

    let mut en: Vec<EntityType> = Vec::new();
    // 环：外弧（右半 360−φ → 90）、孔
    push_x(&mut en, arc([0.0, h], r_out, 360.0 - phi_deg, 90.0, LAYER_MAIN));
    en.push(circle([0.0, h], r_in, LAYER_MAIN));
    // 圆角 + 支承面 + 立壁
    push_x(&mut en, arc([cx, cy], r1, 180.0 - phi_deg, 180.0, LAYER_MAIN));
    push_x(&mut en, line([0.0, 0.0], [r_base, 0.0], LAYER_MAIN));
    push_x(&mut en, line([r_base, 0.0], [r_base, cy], LAYER_MAIN));
    // 基座细节（h1 线 + (0,(d4−D1)/2)→(0.7d,h1)→(0.8d,0.9h1)）
    push_x(&mut en, line([0.0, h1], [0.7 * d, h1], LAYER_MAIN));
    push_x(
        &mut en,
        polyline(
            &[[0.0, r_out - r_in], [0.7 * d, h1], [0.8 * d, 0.9 * h1]],
            false,
            LAYER_MAIN,
        ),
    );
    // 杆（模板把 (rs,0)→(rs,−a)→(rs,−end) 分成两段）
    push_x(&mut en, line([rs, 0.0], [rs, -a], LAYER_MAIN));
    push_x(&mut en, line([rs, -a], [rs, -end], LAYER_MAIN));
    push_x(&mut en, line([0.0, -a], [rs, -a], LAYER_MAIN));
    push_x(&mut en, line([rs, -end], [rfm, -l], LAYER_MAIN));
    push_x(&mut en, line([0.0, -l], [rfm, -l], LAYER_MAIN));
    push_x(&mut en, line([0.0, -end], [rs, -end], LAYER_MAIN));
    // 螺纹细线 + 收尾
    push_x(&mut en, line([rs, -a / 2.0], [rfm, -a], LAYER_THIN));
    push_x(&mut en, line([rfm, -a], [rfm, -end], LAYER_THIN));
    // 中心线
    en.push(line([0.0, -l - 2.0], [0.0, h + r_out + 2.0], LAYER_CENTER));
    push_x(&mut en, line([0.0, h], [r_out + 2.0, h], LAYER_CENTER));

    Ok(GenPart {
        entities: en,
        meta: PartMeta {
            code: "GB/T 825-1988".into(),
            name: crate::i18n::t_data("吊环螺钉 A型").into(),
            spec: format!("M{}", trim(d)),
            material: String::new(),
            weight: eye_weight(row),
        },
        bbox: [-r_out, -l - 2.0, r_out, h + r_out],
    })
}

/// 吊环螺钉左视图（环以小圆 Ød1 表示；基座/杆同主视）。
pub fn eye_bolt_end(row: &EyeBoltRow) -> Result<GenPart, String> {
    let (d, d2, d4, d1, h, h1, a, l, b, d_hole) =
        (row.d, row.d2, row.d4, row.d1, row.h, row.h1, row.a, row.l, row.b, row.d_hole);
    let r_out = d4 / 2.0;
    let rl = d1 / 2.0;
    let r_ring = (d4 - d_hole) / 2.0; // 环体径向高度（模板 75）
    let y_up = h + (r_out - rl);
    let y_lo = h - (r_out - rl);
    let rs = d / 2.0;
    let rfm = 0.85 * rs;
    let r_base = d2 / 2.0;
    let end = l - eye_end_chamfer(d);
    // 基座外锥上端 x：模板实测 89.7 = 0.87855·d2/2（M100）。
    let x_base_top = r_base * 0.87855;

    let mut en: Vec<EntityType> = Vec::new();
    // 环截面圆（上：实线上半 + 虚线下半）
    en.push(arc([0.0, y_up], rl, 0.0, 180.0, LAYER_MAIN));
    en.push(arc([0.0, y_up], rl, 180.0, 0.0, LAYER_HIDDEN));
    push_x(&mut en, line([rl, y_up], [b / 2.0, 0.54 * b], LAYER_MAIN));
    push_x(&mut en, line([b / 2.0, 0.54 * b], [0.6 * b, y_lo], LAYER_MAIN));
    // 基座外形（底面 → 外锥上端）
    push_x(&mut en, polyline(&[[0.0, 0.0], [r_base, 0.0], [x_base_top, h1]], false, LAYER_MAIN));
    // 基座上端细节：(b/2,(d4−D1)/2)→(x_base_top,h1)→(b/2,h1)
    push_x(
        &mut en,
        polyline(
            &[[b / 2.0, r_ring], [x_base_top, h1], [b / 2.0, h1]],
            false,
            LAYER_MAIN,
        ),
    );
    // 杆（与主视一致）
    push_x(&mut en, line([rs, 0.0], [rs, -a], LAYER_MAIN));
    push_x(&mut en, line([rs, -a], [rs, -end], LAYER_MAIN));
    push_x(&mut en, line([0.0, -a], [rs, -a], LAYER_MAIN));
    push_x(&mut en, line([rs, -end], [rfm, -l], LAYER_MAIN));
    push_x(&mut en, line([0.0, -l], [rfm, -l], LAYER_MAIN));
    push_x(&mut en, line([0.0, -end], [rs, -end], LAYER_MAIN));
    push_x(&mut en, line([rs, -a / 2.0], [rfm, -a], LAYER_THIN));
    push_x(&mut en, line([rfm, -a], [rfm, -end], LAYER_THIN));
    // 中心线
    en.push(line([0.0, -l - 2.0], [0.0, h + r_out + 2.0], LAYER_CENTER));
    push_x(&mut en, line([0.0, 0.54 * b], [b / 2.0, 0.54 * b], LAYER_CENTER));
    push_x(&mut en, line([0.0, y_up], [rl, y_up], LAYER_CENTER));

    Ok(GenPart {
        entities: en,
        meta: PartMeta {
            code: "GB/T 825-1988".into(),
            name: crate::i18n::t_data("吊环螺钉 A型").into(),
            spec: format!("M{}", trim(d)),
            material: String::new(),
            weight: eye_weight(row),
        },
        bbox: [-r_base, -l - 2.0, r_base, y_up + rl],
    })
}

// ══════════════════════════════════════════════════════════════════════════
// seal_fb
// ══════════════════════════════════════════════════════════════════════════

/// 密封圈剖面右半轮廓（归一坐标 `u=(x−r)/(R−r)`、`v=y/b`），由模板 D30×d16×b7 逐条反解。
/// 每项 `[u1,v1,u2,v2]`；镜像（`mirror_x`）得左半。u 允许 <0（弹簧中心线越过内孔）。
const SEAL_SEGS: [[f64; 4]; 22] = [
    [0.95, 1.0, 1.0, 0.9],          // 外径上倒角
    [1.0, 0.9, 1.0, 0.1],           // 外径柱面
    [1.0, 0.1, 0.95, 0.0],          // 外径下倒角
    [0.95, 0.1, 0.95, 0.9],         // 骨架外壁
    [0.95, 0.9, 0.85, 0.9],         // 骨架顶
    [0.85, 0.9, 0.85, 0.2],         // 骨架内壁
    [0.85, 0.2, 0.333329, 0.2],     // 骨架底
    [0.8, 0.25, 0.347614, 0.25],    // 骨架内台阶
    [0.8, 0.25, 0.835714, 1.0],     // 骨架斜壁
    [0.75, 0.25, 0.75, 0.2],
    [0.383329, 0.753271, 0.383329, 0.9], // 副唇竖边
    [0.433329, 0.25, 0.433329, 0.2],
    [0.141671, 0.0, 0.191671, 0.05],     // 底部小折边
    [0.347614, 0.25, 0.333329, 0.566671],
    [0.1, 0.9, 0.0, 0.8],           // 主唇
    [0.0, 0.8, 0.1, 0.5],
    [0.166671, 0.5, 0.25, 0.3],
    [0.25, 0.3, 0.041671, 0.05],
    [0.041671, 0.05, 0.141671, 0.0],
    [0.191671, 0.05, 0.241671, 0.0],
    [0.333329, 0.1, 0.333329, 0.2],
    [0.333329, 0.1, 0.95, 0.1],     // 骨架底外延
];

/// 全宽水平线（模板里跨过轴线的投影线）：`[v, u_half]`，实际 x 半宽 = r+u_half·(R−r)。
const SEAL_HLINES: [[f64; 2]; 7] = [
    [1.0, 0.95],
    [0.9, 0.383329],
    [0.8, 0.0],
    [0.5, 0.166671],
    [0.3, 0.25],
    [0.05, 0.041671],
    [0.0, 0.95],
];

/// 弹簧小圆：`[u, v, 半径/b]`。
const SEAL_CIRCLES: [[f64; 3]; 2] = [
    [0.333329, 0.666671, 0.08],
    [0.333329, 0.666671, 0.1],
];

/// 金属骨架剖面线边界（归一 `(u,v)`，6 点；模板左半 angle 0°/scale 1.0、右半 angle 90°/scale 0.25）。
const SEAL_METAL_HATCH: [[f64; 2]; 6] = [
    [0.333329, 0.1],
    [0.333329, 0.2],
    [0.85, 0.2],
    [0.85, 0.9],
    [0.95, 0.9],
    [0.95, 0.1],
];

/// 橡胶体剖面线边界（归一 `(u,v)`，29 边）：第 1 条边是唇口圆弧（圆心 (0.333329,0.666671)、
/// 半径 0.1b、270°→480°），后 28 条为直线（`[29]` 收尾回 `[0]`）。模板左右各 1 片。
const SEAL_RUBBER_HATCH: [[f64; 2]; 29] = [
    [0.333329, 0.566671],
    [0.383329, 0.753271],
    [0.383329, 0.9],
    [0.1, 0.9],
    [0.0, 0.8],
    [0.1, 0.5],
    [0.166671, 0.5],
    [0.25, 0.3],
    [0.041671, 0.05],
    [0.141671, 0.0],
    [0.191671, 0.05],
    [0.241671, 0.0],
    [0.95, 0.0],
    [1.0, 0.1],
    [1.0, 0.9],
    [0.95, 1.0],
    [0.835714, 1.0],
    [0.8, 0.25],
    [0.75, 0.25],
    [0.75, 0.2],
    [0.85, 0.2],
    [0.85, 0.9],
    [0.95, 0.9],
    [0.95, 0.1],
    [0.333329, 0.1],
    [0.333329, 0.2],
    [0.433329, 0.2],
    [0.433329, 0.25],
    [0.347614, 0.25],
];

fn seal_weight(row: &SealRow) -> String {
    let v = std::f64::consts::PI / 4.0 * (row.od * row.od - row.d1 * row.d1) * row.b;
    crate::partgen_kit::weight_text(crate::partgen_kit::weight_kg(v, 7.85))
}

/// 内包骨架有副唇密封圈 FB 型主视图（轴向剖视）。
pub fn seal_fb(row: &SealRow) -> Result<GenPart, String> {
    let (d1, od, b) = (row.d1, row.od, row.b);
    let r = d1 / 2.0;
    let rr = od / 2.0;
    let w = rr - r;
    let x = |u: f64| r + u * w;
    let y = |v: f64| v * b;

    let mut en: Vec<EntityType> = Vec::new();
    // 全宽水平线（模板里跨过轴线的投影线，直接画整条）
    for [v, u] in SEAL_HLINES {
        en.push(line([-x(u), y(v)], [x(u), y(v)], LAYER_MAIN));
    }
    // 右半轮廓
    for [u1, v1, u2, v2] in SEAL_SEGS {
        push_x(&mut en, line([x(u1), y(v1)], [x(u2), y(v2)], LAYER_MAIN));
    }
    // 弹簧小圆
    for [u, v, rf] in SEAL_CIRCLES {
        push_x(&mut en, circle([x(u), y(v)], rf * b, LAYER_MAIN));
    }
    // 中心线
    en.push(line([0.0, b + 3.0], [0.0, -3.0], LAYER_CENTER));
    push_x(&mut en, line([x(6.6333 / 7.0 - 8.0 / 7.0), y(2.0 / 3.0)], [x(14.0333 / 7.0 - 8.0 / 7.0), y(2.0 / 3.0)], LAYER_CENTER));
    push_x(&mut en, line([x((10.3333 - 8.0) / 7.0), y(0.9667 / 7.0)], [x((10.3333 - 8.0) / 7.0), y(8.3667 / 7.0)], LAYER_CENTER));

    // ── 剖面线（模板 4 片 ANSI31；左半 = 模板存储的 extrude(+z) 半边，右半 = 其镜像）──
    let xn = |u: f64| -(r + u * w);
    let yv = |v: f64| v * b;
    // 金属骨架：左半 #1（angle 0°/scale 1.0），右半 #3（angle 90°/scale 0.25）
    let metal = |sx: f64| -> Vec<[f64; 2]> {
        SEAL_METAL_HATCH
            .iter()
            .map(|p| [sx * (r + p[0] * w), p[1] * b])
            .collect()
    };
    en.push(hatch_ansi31_scaled(&metal(-1.0), 0.0, 1.0));
    en.push(hatch_ansi31_scaled(&metal(1.0), 90.0, 0.25));
    // 橡胶体：左半 #2、右半 #4，均 angle 0°/scale 0.25；边界含唇口圆弧
    let rubber_neg: Vec<HatchEdge> = {
        let p = |k: usize| [xn(SEAL_RUBBER_HATCH[k][0]), yv(SEAL_RUBBER_HATCH[k][1])];
        let mut edges = Vec::with_capacity(29);
        edges.push(HatchEdge::Arc {
            c: [xn(0.333329), yv(0.666671)],
            r: 0.1 * b,
            start_deg: 270.0,
            end_deg: 480.0,
            ccw: true,
        });
        for k in 1..28 {
            edges.push(HatchEdge::Line { a: p(k), b: p(k + 1) });
        }
        edges.push(HatchEdge::Line { a: p(28), b: p(0) });
        edges
    };
    // 橡胶件（唇口）用 **ANSI37 双向网纹**（GB/T 4457.5 材质剖面线约定）；
    // 模板实测：这两片 pattern_name=ANSI37（金属骨架两片是 ANSI31），同名同边界同角度同 scale，只差图案。
    en.push(hatch_ansi37_edges(&rubber_neg, 0.0, 0.25));
    en.push(hatch_ansi37_edges(&mirror_hatch_x(&rubber_neg), 0.0, 0.25));

    Ok(GenPart {
        entities: en,
        meta: PartMeta {
            code: "GB/T 13871.1-2007".into(),
            name: crate::i18n::t_data("内包骨架有副唇密封圈 FB型").into(),
            spec: format!("{}×{}×{}", trim(d1), trim(od), trim(b)),
            material: String::new(),
            weight: seal_weight(row),
        },
        bbox: [-rr, -3.0, rr, b + 3.0],
    })
}

// ══════════════════════════════════════════════════════════════════════════
// bearing_276
// ══════════════════════════════════════════════════════════════════════════

fn bearing_weight(row: &BearingRow) -> String {
    let v = std::f64::consts::PI / 4.0 * (row.od * row.od - row.d * row.d) * row.w * 0.6;
    crate::partgen_kit::weight_text(crate::partgen_kit::weight_kg(v, 7.85))
}

/// 深沟球轴承 60000 型主视图（轴向剖视；上半个视镜像得下半）。
pub fn bearing_276(row: &BearingRow) -> Result<GenPart, String> {
    let (d, od, bw, rc) = (row.d, row.od, row.w, row.r);
    let r = d / 2.0;
    let rr = od / 2.0;
    let rcen = (r + rr) / 2.0; // 球心半径
    let rb = (rr - r) / 4.0; // 球半径（模板实测比例 (D−d)/8）
    let r_ir = rcen - rb / 2.0; // 内圈挡面半径
    let r_or = rcen + rb / 2.0; // 外圈挡面半径
    let hw = bw / 2.0;
    let gx = rb * 30f64.to_radians().cos(); // 滚道端点轴向偏移 = rb·cos30°
    let xg1 = hw - gx;
    let xg2 = hw + gx;

    let mut en: Vec<EntityType> = Vec::new();
    // 外径 + 两端圆角
    push_y(&mut en, line([rc, rr], [bw - rc, rr], LAYER_MAIN));
    push_y(&mut en, arc([rc, rr - rc], rc, 90.0, 180.0, LAYER_MAIN));
    push_y(&mut en, arc([bw - rc, rr - rc], rc, 0.0, 90.0, LAYER_MAIN));
    // 外圈（挡面 + 左右端面段）
    push_y(&mut en, polyline(&[[xg1, r_or], [0.0, r_or], [0.0, rr - rc]], false, LAYER_MAIN));
    push_y(&mut en, polyline(&[[bw, rr - rc], [bw, r_or]], false, LAYER_MAIN));
    push_y(&mut en, polyline(&[[bw, r_or], [xg2, r_or]], false, LAYER_MAIN));
    // 外圈滚道（球圆上段 30…150）
    push_y(&mut en, arc([hw, rcen], rb, 30.0, 150.0, LAYER_MAIN));
    // 内圈（挡面 + 左右端面段）
    push_y(&mut en, polyline(&[[0.0, r + rc], [0.0, r_ir], [xg1, r_ir]], false, LAYER_MAIN));
    push_y(&mut en, polyline(&[[xg2, r_ir], [bw, r_ir], [bw, r + rc]], false, LAYER_MAIN));
    // 内圈滚道（球圆下段 210…330）
    push_y(&mut en, arc([hw, rcen], rb, 210.0, 330.0, LAYER_MAIN));
    // 内孔 + 两端倒角
    push_y(&mut en, polyline(&[[bw - rc, r], [rc, r]], false, LAYER_MAIN));
    push_y(&mut en, arc([rc, r + rc], rc, 180.0, 270.0, LAYER_MAIN));
    push_y(&mut en, arc([bw - rc, r + rc], rc, 270.0, 360.0, LAYER_MAIN));
    // 钢球（整圆）
    push_y(&mut en, circle([hw, rcen], rb, LAYER_MAIN));
    // 端面从轴线引出的整竖线（模板实测）
    push_y(&mut en, line([0.0, 0.0], [0.0, rr - rc], LAYER_MAIN));
    push_y(&mut en, line([bw, rr - rc], [bw, 0.0], LAYER_MAIN));
    // 中心线
    en.push(line([-2.0, 0.0], [bw + 2.0, 0.0], LAYER_CENTER));
    push_y(&mut en, line([hw, rcen - 3.0 * rb], [hw, rcen + 3.0 * rb], LAYER_CENTER));
    push_y(&mut en, line([-1.0, rcen], [bw + 1.0, rcen], LAYER_CENTER));

    // ── 剖面线（模板 4 片 ANSI31 scale 1.0）：上外圈 0°/上内圈 90°/下外圈 270°/下内圈 180° ──
    // 上半边界 = 模板 piece#0/#1 实测（含滚道 r=b 弧、端圆角 r=rc 弧）；下半 = 关于 y=0 镜像。
    let outer_up: Vec<HatchEdge> = vec![
        HatchEdge::Line { a: [0.0, r_or], b: [xg1, r_or] },
        HatchEdge::Arc { c: [hw, rcen], r: rb, start_deg: 30.0, end_deg: 150.0, ccw: true },
        HatchEdge::Line { a: [xg2, r_or], b: [bw, r_or] },
        HatchEdge::Line { a: [bw, r_or], b: [bw, rr - rc] },
        HatchEdge::Arc { c: [bw - rc, rr - rc], r: rc, start_deg: 0.0, end_deg: 90.0, ccw: true },
        HatchEdge::Line { a: [bw - rc, rr], b: [rc, rr] },
        HatchEdge::Arc { c: [rc, rr - rc], r: rc, start_deg: 90.0, end_deg: 180.0, ccw: true },
        HatchEdge::Line { a: [0.0, rr - rc], b: [0.0, r_or] },
    ];
    let inner_up: Vec<HatchEdge> = vec![
        HatchEdge::Arc { c: [rc, r + rc], r: rc, start_deg: 180.0, end_deg: 270.0, ccw: true },
        HatchEdge::Line { a: [rc, r], b: [bw - rc, r] },
        HatchEdge::Arc { c: [bw - rc, r + rc], r: rc, start_deg: 270.0, end_deg: 360.0, ccw: true },
        HatchEdge::Line { a: [bw, r + rc], b: [bw, r_ir] },
        HatchEdge::Line { a: [bw, r_ir], b: [xg2, r_ir] },
        HatchEdge::Arc { c: [hw, rcen], r: rb, start_deg: 210.0, end_deg: 330.0, ccw: true },
        HatchEdge::Line { a: [xg1, r_ir], b: [0.0, r_ir] },
        HatchEdge::Line { a: [0.0, r_ir], b: [0.0, r + rc] },
    ];
    en.push(hatch_ansi31_edges(&outer_up, 0.0, 1.0));
    en.push(hatch_ansi31_edges(&inner_up, 90.0, 1.0));
    en.push(hatch_ansi31_edges(&mirror_hatch_y(&outer_up), 270.0, 1.0));
    en.push(hatch_ansi31_edges(&mirror_hatch_y(&inner_up), 180.0, 1.0));

    Ok(GenPart {
        entities: en,
        meta: PartMeta {
            code: "GB/T 276-2013".into(),
            name: crate::i18n::t_data("深沟球轴承 60000型").into(),
            spec: row.code.clone(),
            material: String::new(),
            weight: bearing_weight(row),
        },
        bbox: [-2.0, -rr, bw + 2.0, rr],
    })
}

// ══════════════════════════════════════════════════════════════════════════
// 对外 API（partgen_more 会先问这里）
// ══════════════════════════════════════════════════════════════════════════

/// 本组声明的视图。
pub fn family_views(family: &str) -> Vec<&'static str> {
    match family {
        "eye_bolt_825" => vec!["main", "end"],
        "seal_fb" => vec!["main"],
        "bearing_276" => vec!["main"],
        _ => Vec::new(),
    }
}

/// 本组族的目录 JSON 片段（键 = 族 id）。
pub fn families_json() -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();

    // ── 吊环螺钉 ──
    let sizes: Vec<serde_json::Value> = eye_table()
        .rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "d": r.d, "label": format!("M{}", trim(r.d)), "pitch": r.pitch,
                "l_min": r.l, "l_max": r.l, "lengths": [r.l],
                "extra": format!(
                    "D1={} d4={} h={} h1={} r1={} a={} d2={} d1={} b={}",
                    trim(r.d_hole), trim(r.d4), trim(r.h), trim(r.h1), trim(r.r1),
                    trim(r.a), trim(r.d2), trim(r.d1), trim(r.b)),
            })
        })
        .collect();
    m.insert(
        "eye_bolt_825".into(),
        serde_json::json!({
            "id": "eye_bolt_825", "name": "吊环螺钉 A型", "code": "GB/T 825-1988",
            "iso": "—", "implemented": true, "views": views_json("eye_bolt_825"),
            "sizes": sizes, "len_label": "螺纹长度 l",
            "base_hint": "基点 = 支承面 × 轴线（环心在 +y 侧、杆沿 −y）",
            "tree_path": "零件库/螺钉/吊环螺钉/吊环螺钉 A型 GB/T 825-1988",
        }),
    );

    // ── 密封圈（按 d1 唯一化；lengths = 该 d1 的各外径 D）──
    let mut by_d1: Vec<(f64, Vec<f64>, Vec<String>)> = Vec::new();
    for r in &seal_table().rows {
        if let Some(e) = by_d1.iter_mut().find(|e| (e.0 - r.d1).abs() < 1e-9) {
            if !e.1.iter().any(|v| (v - r.od).abs() < 1e-9) {
                e.1.push(r.od);
            }
            e.2.push(format!("D{}×b{}", trim(r.od), trim(r.b)));
        } else {
            by_d1.push((r.d1, vec![r.od], vec![format!("D{}×b{}", trim(r.od), trim(r.b))]));
        }
    }
    by_d1.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    for e in by_d1.iter_mut() {
        e.1.sort_by(|a, b| a.partial_cmp(b).unwrap());
    }
    let seal_sizes: Vec<serde_json::Value> = by_d1
        .iter()
        .map(|(d1, ods, tags)| {
            serde_json::json!({
                "d": d1, "label": format!("{} {}", crate::i18n::t_data("轴径"), trim(*d1)), "pitch": 0.0,
                "l_min": ods.first().copied().unwrap_or(0.0),
                "l_max": ods.last().copied().unwrap_or(0.0),
                "lengths": ods, "extra": tags.join("、"),
            })
        })
        .collect();
    m.insert(
        "seal_fb".into(),
        serde_json::json!({
            "id": "seal_fb", "name": "内包骨架有副唇密封圈 FB型",
            "code": "GB/T 13871.1-2007", "iso": "—", "implemented": true,
            "views": views_json("seal_fb"), "sizes": seal_sizes,
            "len_label": "外径 D（同轴径变体）",
            "base_hint": "基点 = 端面中心 × 轴线（剖面：轴线 x=0、端面 y=0）",
            "tree_path": "零件库/密封件/内包骨架有副唇密封圈 FB型 GB/T 13871.1-2007",
        }),
    );

    // ── 轴承（按内径 d 唯一化；lengths = 该内径的各宽度 B）──
    let mut by_d: Vec<(f64, Vec<f64>, Vec<String>)> = Vec::new();
    for r in &bearing_table().rows {
        if let Some(e) = by_d.iter_mut().find(|e| (e.0 - r.d).abs() < 1e-9) {
            if !e.1.iter().any(|v| (v - r.w).abs() < 1e-9) {
                e.1.push(r.w);
            }
            e.2.push(format!("{} D{}×B{}×r{}", r.code, trim(r.od), trim(r.w), trim(r.r)));
        } else {
            by_d.push((
                r.d,
                vec![r.w],
                vec![format!("{} D{}×B{}×r{}", r.code, trim(r.od), trim(r.w), trim(r.r))],
            ));
        }
    }
    by_d.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    for e in by_d.iter_mut() {
        e.1.sort_by(|a, b| a.partial_cmp(b).unwrap());
    }
    let brg_sizes: Vec<serde_json::Value> = by_d
        .iter()
        .map(|(d, ws, tags)| {
            serde_json::json!({
                "d": d, "label": format!("{} {}", crate::i18n::t_data("内径"), trim(*d)), "pitch": 0.0,
                "l_min": ws.first().copied().unwrap_or(0.0),
                "l_max": ws.last().copied().unwrap_or(0.0),
                "lengths": ws, "extra": tags.join("、"),
            })
        })
        .collect();
    m.insert(
        "bearing_276".into(),
        serde_json::json!({
            "id": "bearing_276", "name": "深沟球轴承 60000型",
            "code": "GB/T 276-2013", "iso": "ISO 15:2017", "implemented": true,
            "views": views_json("bearing_276"), "sizes": brg_sizes,
            "len_label": "宽度 B（同内径变体）",
            "base_hint": "基点 = 端面中心 × 轴线（主视图：轴线 y=0、端面 x=0）",
            "tree_path": "零件库/轴承/深沟球轴承 60000型 GB/T 276-2013",
        }),
    );

    m
}

/// 本组族的生成派发。
pub fn generate(family: &str, d: f64, l: f64, view: &str) -> Option<Result<GenPart, String>> {
    match family {
        "eye_bolt_825" => Some(gen_eye(d, l, view)),
        "seal_fb" => Some(gen_seal(d, l, view)),
        "bearing_276" => Some(gen_bearing(d, l, view)),
        _ => None,
    }
}

fn gen_eye(d: f64, l: f64, view: &str) -> Result<GenPart, String> {
    if !family_views("eye_bolt_825").contains(&view) {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.view_not_offered",
            &[("family", "eye_bolt_825"), ("view", view), ("avail", "main/end")],
        ));
    }
    let row = eye_row(d).ok_or_else(|| {
        crate::i18n::t_fmt("cmd.parts.err.no_m", &[("table", "GB/T 825"), ("d", &trim(d))])
    })?;
    if (l - row.l).abs() > 1e-6 {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.thread_len",
            &[("d", &trim(d)), ("want", &trim(row.l)), ("got", &trim(l))],
        ));
    }
    match view {
        "main" => eye_bolt_main(row),
        _ => eye_bolt_end(row),
    }
}

fn gen_seal(d1: f64, od: f64, view: &str) -> Result<GenPart, String> {
    if view != "main" {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.only_main_received",
            &[("family", "seal_fb"), ("view", view)],
        ));
    }
    let row = seal_row(d1, od).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.parts.err.no_d1_od",
            &[("d1", &trim(d1)), ("od", &trim(od))],
        )
    })?;
    seal_fb(row)
}

fn gen_bearing(d: f64, w: f64, view: &str) -> Result<GenPart, String> {
    if view != "main" {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.only_main_received",
            &[("family", "bearing_276"), ("view", view)],
        ));
    }
    let row = bearing_row(d, w).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.parts.err.no_d_bearing",
            &[("d", &trim(d)), ("b", &trim(w))],
        )
    })?;
    bearing_276(row)
}

// ══════════════════════════════════════════════════════════════════════════
// 测试
// ══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::partgen::generate as gen_all;
    use crate::partgen_kit::{dump_svg, part_svg, LAYER_HATCH};
    use ocs_plugin_api::host::acadrust::types::Color;

    const OCSM_LAYERS: [&str; 5] = [LAYER_MAIN, LAYER_THIN, LAYER_CENTER, LAYER_HIDDEN, LAYER_HATCH];

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-3
    }

    /// 全部规格 × 全部视图都能生成；图层只在 OCSM 五层内、无尺寸标注、ByLayer。
    #[test]
    fn all_specs_generate_and_layers_ok() {
        for (fam, list) in [
            ("eye_bolt_825", eye_table().rows.iter().map(|r| (r.d, r.l)).collect::<Vec<_>>()),
            ("seal_fb", seal_table().rows.iter().map(|r| (r.d1, r.od)).collect::<Vec<_>>()),
            ("bearing_276", bearing_table().rows.iter().map(|r| (r.d, r.w)).collect::<Vec<_>>()),
        ] {
            assert!(list.len() >= 15, "{fam} 规格太少（{}）", list.len());
            // 每个规格只测一次（分组后首值即可），避免 336 次重复
            let mut seen = std::collections::BTreeSet::new();
            for (d, l) in list {
                let key = format!("{d}/{l}");
                if !seen.insert(key) {
                    continue;
                }
                for view in family_views(fam) {
                    let p = gen_all(fam, d, l, view)
                        .unwrap_or_else(|e| panic!("{fam} d{d}×l{l} {view}: {e}"));
                    assert!(!p.entities.is_empty(), "{fam} {view} 空");
                    for e in &p.entities {
                        let lay = e.common().layer.as_str();
                        assert!(OCSM_LAYERS.contains(&lay), "{fam} {view} 图层越界: {lay}");
                        assert!(matches!(e.common().color, Color::ByLayer), "{fam} 颜色非 ByLayer");
                        assert!(!matches!(e, EntityType::Dimension(_)), "{fam} 出现尺寸标注");
                    }
                    assert!(p.bbox[2] > p.bbox[0] && p.bbox[3] > p.bbox[1], "{fam} bbox 无效");
                }
            }
        }
    }

    /// 每个规格分组后的首个长度都能出图（全局护栏用的就是 sizes[0].lengths[0]）。
    #[test]
    fn min_spec_and_first_length_usable() {
        for fam in ["eye_bolt_825", "seal_fb", "bearing_276"] {
            let sizes = crate::partgen::family_sizes(fam);
            assert!(!sizes.is_empty(), "{fam} 无 sizes");
            let s0 = &sizes[0];
            let l = s0.lengths[0];
            for v in family_views(fam) {
                gen_all(fam, s0.d, l, v).unwrap_or_else(|e| panic!("{fam} {v}: {e}"));
            }
        }
    }

    // ── 图元提取工具 ──

    fn lines_of(p: &GenPart) -> Vec<((f64, f64), (f64, f64))> {
        p.entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Line(l) => Some(((l.start.x, l.start.y), (l.end.x, l.end.y))),
                _ => None,
            })
            .collect()
    }
    fn arcs_of(p: &GenPart) -> Vec<((f64, f64), f64, f64, f64)> {
        p.entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) => Some((
                    (a.center.x, a.center.y),
                    a.radius,
                    a.start_angle.to_degrees(),
                    a.end_angle.to_degrees(),
                )),
                _ => None,
            })
            .collect()
    }
    fn circles_of(p: &GenPart) -> Vec<((f64, f64), f64)> {
        p.entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Circle(c) => Some(((c.center.x, c.center.y), c.radius)),
                _ => None,
            })
            .collect()
    }
    fn has_line(p: &GenPart, a: (f64, f64), b: (f64, f64)) -> bool {
        let direct = lines_of(p).iter().any(|(s, e)| {
            (near(s.0, a.0) && near(s.1, a.1) && near(e.0, b.0) && near(e.1, b.1))
                || (near(s.0, b.0) && near(s.1, b.1) && near(e.0, a.0) && near(e.1, a.1))
        });
        if direct {
            return true;
        }
        // 折线的一段也算（模板大量用 LWPOLYLINE 画轮廓）
        p.entities.iter().any(|e| match e {
            EntityType::LwPolyline(pl) => {
                let v: Vec<(f64, f64)> =
                    pl.vertices.iter().map(|x| (x.location.x, x.location.y)).collect();
                let seg = |s: (f64, f64), t: (f64, f64)| {
                    (near(s.0, a.0) && near(s.1, a.1) && near(t.0, b.0) && near(t.1, b.1))
                        || (near(s.0, b.0) && near(s.1, b.1) && near(t.0, a.0) && near(t.1, a.1))
                };
                v.windows(2).any(|w| seg(w[0], w[1]))
                    || (pl.is_closed && v.len() >= 3 && seg(v[v.len() - 1], v[0]))
            }
            _ => false,
        })
    }
    fn has_arc(p: &GenPart, c: (f64, f64), r: f64, a1: f64, a2: f64) -> bool {
        arcs_of(p).iter().any(|(cc, rr, s, e)| {
            near(cc.0, c.0) && near(cc.1, c.1) && near(*rr, r)
                && ((near(*s, a1) && near(*e, a2)) || (near(*s, a2) && near(*e, a1)))
        })
    }

    // ── 模板 M100 主视图逐条数值回归 ──

    #[test]
    fn eye_bolt_template_m100_regression() {
        let row = eye_row(100.0).expect("M100");
        let p = eye_bolt_main(row).unwrap();
        // 环外弧：心 (0,175) r175，右半 311.371°→90°
        assert!(has_arc(&p, (0.0, 175.0), 175.0, 311.371, 90.0), "环外弧");
        assert!(circles_of(&p).iter().any(|(c, r)| near(c.0, 0.0) && near(c.1, 175.0) && near(*r, 100.0)), "环孔 Ø200");
        // 圆角心 (142.1,13.6538) r40，131.371°→180°
        let cx = 204.2 / 2.0 + 40.0;
        let cy = 175.0 - ((175.0 + 40.0f64).powi(2) - cx * cx).sqrt();
        assert!(near(cx, 142.1) && near(cy, 13.6538), "圆角心 ({cx},{cy})");
        assert!(has_arc(&p, (142.1, 13.6538), 40.0, 131.371, 180.0), "圆角弧");
        // 支承面 / 立壁
        assert!(has_line(&p, (102.1, 0.0), (0.0, 0.0)), "支承面");
        assert!(has_line(&p, (102.1, 0.0), (102.1, 13.6538)), "基座立壁");
        // 基座细节
        assert!(has_line(&p, (0.0, 75.0), (70.0, 62.0)), "基座斜线 1");
        assert!(has_line(&p, (70.0, 62.0), (80.0, 55.8)), "基座斜线 2");
        assert!(has_line(&p, (0.0, 62.0), (70.0, 62.0)), "h1 线");
        // 杆
        assert!(has_line(&p, (50.0, 0.0), (50.0, -12.0)), "杆上段");
        assert!(has_line(&p, (0.0, -12.0), (50.0, -12.0)), "a 线");
        assert!(has_line(&p, (50.0, -12.0), (50.0, -132.5)), "杆下段");
        assert!(has_line(&p, (50.0, -132.5), (42.5, -140.0)), "端部倒角");
        assert!(has_line(&p, (0.0, -140.0), (42.5, -140.0)), "端面");
        assert!(has_line(&p, (50.0, -6.0), (42.5, -12.0)), "收尾斜线");
        assert!(has_line(&p, (42.5, -12.0), (42.5, -132.5)), "螺纹细线");
        // 中心线
        assert!(has_line(&p, (0.0, -142.0), (0.0, 352.0)), "纵中心线 −l−2…h+d4/2+2");
        // 镜像
        assert!(has_line(&p, (-102.1, 0.0), (0.0, 0.0)), "支承面镜像");
        assert!(has_arc(&p, (-142.1, 13.6538), 40.0, 0.0, 48.629), "圆角镜像");
        // 无尺寸标注
        assert!(!p.entities.iter().any(|e| matches!(e, EntityType::Dimension(_))));
    }

    /// 模板 M100 左视图关键数值。
    #[test]
    fn eye_bolt_end_template_regression() {
        let row = eye_row(100.0).unwrap();
        let p = eye_bolt_end(row).unwrap();
        let y_up = 175.0 + (175.0 - 79.2 / 2.0);
        assert!(near(y_up, 310.4), "环截面圆心高 {y_up}");
        assert!(has_arc(&p, (0.0, 310.4), 39.6, 0.0, 180.0), "环截面实线半圆");
        assert!(has_arc(&p, (0.0, 310.4), 39.6, 180.0, 0.0), "环截面虚线半圆");
        assert!(has_line(&p, (39.6, 310.4), (44.0, 47.52)), "环侧线 1");
        assert!(has_line(&p, (44.0, 47.52), (52.8, 39.6)), "环侧线 2");
        assert!(has_line(&p, (0.0, 0.0), (102.1, 0.0)), "基座底");
        assert!(has_line(&p, (102.1, 0.0), (89.7, 62.0)), "基座外锥");
        assert!(has_line(&p, (44.0, 75.0), (89.7, 62.0)), "基座上端斜线");
    }

    /// 模板 61807 主视图逐条数值回归。
    #[test]
    fn bearing_276_template_61807_regression() {
        let row = bearing_row(35.0, 7.0).expect("61807");
        assert_eq!(row.code, "61807");
        let p = bearing_276(row).unwrap();
        // 外径 / 内孔 / 圆角
        assert!(has_line(&p, (0.3, 23.5), (6.7, 23.5)), "外径");
        assert!(has_arc(&p, (0.3, 23.2), 0.3, 90.0, 180.0), "外径左圆角");
        assert!(has_arc(&p, (6.7, 23.2), 0.3, 0.0, 90.0), "外径右圆角");
        assert!(has_line(&p, (6.7, 17.5), (0.3, 17.5)), "内孔");
        assert!(has_arc(&p, (0.3, 17.8), 0.3, 180.0, 270.0), "内孔左倒角");
        assert!(has_arc(&p, (6.7, 17.8), 0.3, 270.0, 360.0), "内孔右倒角");
        // 球 + 滚道（球半径 (D−d)/8 = 1.5）
        assert!(circles_of(&p).iter().any(|(c, r)| near(c.0, 3.5) && near(c.1, 20.5) && near(*r, 1.5)), "钢球");
        assert!(has_arc(&p, (3.5, 20.5), 1.5, 30.0, 150.0), "外圈滚道");
        assert!(has_arc(&p, (3.5, 20.5), 1.5, 210.0, 330.0), "内圈滚道");
        assert!(near(1.5 * 30f64.to_radians().cos(), 1.2990), "滚道端点 cos30");
        // 圈挡面
        assert!(has_line(&p, (2.201, 21.25), (0.0, 21.25)), "外圈挡面左");
        assert!(has_line(&p, (2.201, 19.75), (0.0, 19.75)), "内圈挡面左");
        // 端面整竖线
        assert!(has_line(&p, (0.0, 0.0), (0.0, 23.2)), "左端面整线");
        assert!(has_line(&p, (7.0, 23.2), (7.0, 0.0)), "右端面整线");
        // 中心线
        assert!(has_line(&p, (-2.0, 0.0), (9.0, 0.0)), "主轴线");
        // 镜像（下半）
        assert!(has_line(&p, (0.3, -23.5), (6.7, -23.5)), "下半外径镜像");
        assert!(circles_of(&p).iter().any(|(c, r)| near(c.0, 3.5) && near(c.1, -20.5) && near(*r, 1.5)), "下半钢球");
    }

    /// 模板 D30×d16×b7 主视图关键点（归一坐标 u=(x−8)/7、v=y/7）。
    #[test]
    fn seal_template_d30_regression() {
        let row = seal_row(16.0, 30.0).expect("D30×d16");
        let p = seal_fb(row).unwrap();
        // 外径 15、端面 14.65=15−0.35
        assert!(has_line(&p, (14.65, 7.0), (15.0, 6.3)), "外径上倒角");
        assert!(has_line(&p, (15.0, 6.3), (15.0, 0.7)), "外径柱面");
        assert!(has_line(&p, (15.0, 0.7), (14.65, 0.0)), "外径下倒角");
        // 骨架
        assert!(has_line(&p, (14.65, 6.3), (13.95, 6.3)), "骨架顶");
        assert!(has_line(&p, (13.95, 6.3), (13.95, 1.4)), "骨架内壁");
        assert!(has_line(&p, (13.95, 1.4), (10.3333, 1.4)), "骨架底");
        // 主唇
        assert!(has_line(&p, (8.7, 6.3), (8.0, 5.6)), "主唇 1");
        assert!(has_line(&p, (8.0, 5.6), (8.7, 3.5)), "主唇 2");
        assert!(has_line(&p, (9.1667, 3.5), (9.75, 2.1)), "主唇 3");
        assert!(has_line(&p, (9.75, 2.1), (8.2917, 0.35)), "主唇 4");
        // 全宽投影线
        assert!(has_line(&p, (-8.0, 5.6), (8.0, 5.6)), "y=5.6 全宽线");
        assert!(has_line(&p, (-9.1667, 3.5), (9.1667, 3.5)), "y=3.5 全宽线");
        assert!(has_line(&p, (-14.65, 0.0), (14.65, 0.0)), "底面");
        assert!(has_line(&p, (-14.65, 7.0), (14.65, 7.0)), "顶面");
        // 弹簧小圆（两个）
        assert!(circles_of(&p).iter().any(|(c, r)| near(c.0, 10.3333) && near(c.1, 4.6667) && near(*r, 0.7)), "弹簧外圆");
        assert!(circles_of(&p).iter().any(|(c, r)| near(c.0, 10.3333) && near(c.1, 4.6667) && near(*r, 0.56)), "弹簧内圆");
        // 镜像
        assert!(has_line(&p, (-15.0, 6.3), (-15.0, 0.7)), "外径柱面镜像");
        // 剖面线：模板 4 片（金属骨架左右各 1、橡胶体左右各 1）
        let hatches = p.entities.iter().filter(|e| matches!(e, EntityType::Hatch(_))).count();
        assert_eq!(hatches, 4, "密封圈主视图应 4 片剖面线");
    }

    /// 模板剖面线回归（片数 / angle / scale / 边界逐边含圆弧）。
    ///
    /// 模板权威读数（ezdxf）：
    /// - seal_fb：4 片——金属骨架左 angle 0°/scale 1.0、右 angle 90°/scale 0.25；
    ///   橡胶体左右各 1 片 angle 0°/scale 0.25（含唇口 r=0.7 圆弧）。模板右半的 HATCH 以
    ///   OCS(extrusion=0,0,-1) 存储，按族规则换算为 x→−x（即生成件的正中 x 半）。
    /// - bearing_276：4 片 scale 1.0，上外 0°/上内 90°/下外 270°/下内 180°，各 8 边含滚道弧/圆角弧。
    #[test]
    fn seal_bearing_hatches_match_template() {
        #[derive(Debug, Clone)]
        enum HEdge {
            L([f64; 2], [f64; 2]),
            A([f64; 2], f64, f64, f64),
        }
        fn extract_hatches(p: &GenPart) -> Vec<(f64, f64, Vec<HEdge>)> {
            use ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge;
            p.entities
                .iter()
                .filter_map(|e| match e {
                    EntityType::Hatch(h) => {
                        let path = h.paths.first()?;
                        let mut edges = Vec::new();
                        for ed in &path.edges {
                            match ed {
                                BoundaryEdge::Line(l) => {
                                    edges.push(HEdge::L([l.start.x, l.start.y], [l.end.x, l.end.y]))
                                }
                                BoundaryEdge::CircularArc(a) => edges.push(HEdge::A(
                                    [a.center.x, a.center.y],
                                    a.radius,
                                    a.start_angle.to_degrees(),
                                    a.end_angle.to_degrees(),
                                )),
                                _ => return None,
                            }
                        }
                        Some((h.pattern_angle.to_degrees(), h.pattern_scale, edges))
                    }
                    _ => None,
                })
                .collect()
        }
        fn edges_close(got: &[HEdge], want: &[HEdge]) -> bool {
            let ap = |a: f64, b: f64| {
                let d = (a - b).rem_euclid(360.0);
                d.min(360.0 - d)
            };
            got.len() == want.len()
                && got.iter().zip(want).all(|(g, w)| match (g, w) {
                    (HEdge::L(a, b), HEdge::L(c, d)) => {
                        (a[0] - c[0]).abs() < 1e-3
                            && (a[1] - c[1]).abs() < 1e-3
                            && (b[0] - d[0]).abs() < 1e-3
                            && (b[1] - d[1]).abs() < 1e-3
                    }
                    (HEdge::A(c1, r1, s1, e1), HEdge::A(c2, r2, s2, e2)) => {
                        (c1[0] - c2[0]).abs() < 1e-3
                            && (c1[1] - c2[1]).abs() < 1e-3
                            && (r1 - r2).abs() < 1e-3
                            && ap(*s1, *s2) < 1e-3
                            && ap(*e1, *e2) < 1e-3
                    }
                    _ => false,
                })
        }

        // ── seal_fb（D30×d16×b7）──
        let p = gen_all("seal_fb", 16.0, 30.0, "main").unwrap();
        let hs = extract_hatches(&p);
        assert_eq!(hs.len(), 4, "seal_fb 应 4 片剖面线");
        // 图案名：**金属骨架 2 片 = ANSI31（单向 45°）、唇口橡胶 2 片 = ANSI37（双向网纹）**
        // —— GB/T 4457.5 材质剖面线约定；模板实测：片0/2=ANSI31、片1/3=ANSI37。
        let names: Vec<String> = p
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Hatch(h) => Some(h.pattern.name.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            names.iter().filter(|n| n.as_str() == "ANSI31").count(),
            2,
            "seal_fb 金属骨架应为 ANSI31：{names:?}"
        );
        assert_eq!(
            names.iter().filter(|n| n.as_str() == "ANSI37").count(),
            2,
            "seal_fb 唇口橡胶应为 ANSI37（双向网纹）：{names:?}"
        );
        // 模板金属骨架 6 点（左半，负 x）
        let metal_pts: [[f64; 2]; 6] = [
            [-10.3333, 0.7],
            [-10.3333, 1.4],
            [-13.95, 1.4],
            [-13.95, 6.3],
            [-14.65, 6.3],
            [-14.65, 0.7],
        ];
        let metal_neg: Vec<HEdge> = (0..6)
            .map(|i| HEdge::L(metal_pts[i], metal_pts[(i + 1) % 6]))
            .collect();
        let metal_pos: Vec<HEdge> = metal_neg
            .iter()
            .map(|e| match e {
                HEdge::L(a, b) => HEdge::L([-a[0], a[1]], [-b[0], b[1]]),
                _ => unreachable!(),
            })
            .collect();
        // 模板橡胶体 29 边（左半，负 x；首边为唇口弧）
        let rub_pts: [[f64; 2]; 28] = [
            [-10.6833, 5.2729],
            [-10.6833, 6.3],
            [-8.7, 6.3],
            [-8.0, 5.6],
            [-8.7, 3.5],
            [-9.1667, 3.5],
            [-9.75, 2.1],
            [-8.2917, 0.35],
            [-8.9917, 0.0],
            [-9.3417, 0.35],
            [-9.6917, 0.0],
            [-14.65, 0.0],
            [-15.0, 0.7],
            [-15.0, 6.3],
            [-14.65, 7.0],
            [-13.85, 7.0],
            [-13.6, 1.75],
            [-13.25, 1.75],
            [-13.25, 1.4],
            [-13.95, 1.4],
            [-13.95, 6.3],
            [-14.65, 6.3],
            [-14.65, 0.7],
            [-10.3333, 0.7],
            [-10.3333, 1.4],
            [-11.0333, 1.4],
            [-11.0333, 1.75],
            [-10.4333, 1.75],
        ];
        let arc_start = [-10.3333, 3.9667];
        let mut rubber_neg: Vec<HEdge> = vec![HEdge::A([-10.3333, 4.6667], 0.7, 270.0, 480.0)];
        for k in 0..27 {
            rubber_neg.push(HEdge::L(rub_pts[k], rub_pts[k + 1]));
        }
        rubber_neg.push(HEdge::L(rub_pts[27], arc_start));
        let rubber_pos: Vec<HEdge> = rubber_neg
            .iter()
            .map(|e| match e {
                HEdge::L(a, b) => HEdge::L([-a[0], a[1]], [-b[0], b[1]]),
                HEdge::A(c, r, s, en) => {
                    let span = (en - s).abs();
                    let ns = (180.0 - en).rem_euclid(360.0);
                    HEdge::A([-c[0], c[1]], *r, ns, ns + span)
                }
            })
            .collect();
        for (ang, scale, edges) in &hs {
            let (wa, ws) = if edges_close(edges, &metal_neg) {
                (0.0, 1.0)
            } else if edges_close(edges, &metal_pos) {
                (90.0, 0.25)
            } else if edges_close(edges, &rubber_neg) {
                (0.0, 0.25)
            } else if edges_close(edges, &rubber_pos) {
                (0.0, 0.25)
            } else {
                panic!("seal_fb 未知边界片: {edges:?}");
            };
            assert!((ang - wa).abs() < 1e-9, "seal_fb angle {ang} ≠ {wa}");
            assert!((scale - ws).abs() < 1e-9, "seal_fb scale {scale} ≠ {ws}");
        }

        // ── bearing_276（61807：d35 D47 B7 r0.3）──
        let p = gen_all("bearing_276", 35.0, 7.0, "main").unwrap();
        let hs = extract_hatches(&p);
        assert_eq!(hs.len(), 4, "bearing_276 应 4 片剖面线");
        let (r, rr, rcen, rb, hw, rc) = (17.5, 23.5, 20.5, 1.5, 3.5, 0.3);
        let r_or = rcen + rb / 2.0;
        let r_ir = rcen - rb / 2.0;
        let gx = rb * 30f64.to_radians().cos();
        let (xg1, xg2, bw) = (hw - gx, hw + gx, 7.0);
        let outer_up: Vec<HEdge> = vec![
            HEdge::L([0.0, r_or], [xg1, r_or]),
            HEdge::A([hw, rcen], rb, 30.0, 150.0),
            HEdge::L([xg2, r_or], [bw, r_or]),
            HEdge::L([bw, r_or], [bw, rr - rc]),
            HEdge::A([bw - rc, rr - rc], rc, 0.0, 90.0),
            HEdge::L([bw - rc, rr], [rc, rr]),
            HEdge::A([rc, rr - rc], rc, 90.0, 180.0),
            HEdge::L([0.0, rr - rc], [0.0, r_or]),
        ];
        let inner_up: Vec<HEdge> = vec![
            HEdge::A([rc, r + rc], rc, 180.0, 270.0),
            HEdge::L([rc, r], [bw - rc, r]),
            HEdge::A([bw - rc, r + rc], rc, 270.0, 360.0),
            HEdge::L([bw, r + rc], [bw, r_ir]),
            HEdge::L([bw, r_ir], [xg2, r_ir]),
            HEdge::A([hw, rcen], rb, 210.0, 330.0),
            HEdge::L([xg1, r_ir], [0.0, r_ir]),
            HEdge::L([0.0, r_ir], [0.0, r + rc]),
        ];
        let outer_lo: Vec<HEdge> = outer_up
            .iter()
            .map(|e| match e {
                HEdge::L(a, b) => HEdge::L([a[0], -a[1]], [b[0], -b[1]]),
                HEdge::A(c, rr_, s, en) => {
                    let span = (en - s).abs();
                    let ns = (-en).rem_euclid(360.0);
                    HEdge::A([c[0], -c[1]], *rr_, ns, ns + span)
                }
            })
            .collect();
        let inner_lo: Vec<HEdge> = inner_up
            .iter()
            .map(|e| match e {
                HEdge::L(a, b) => HEdge::L([a[0], -a[1]], [b[0], -b[1]]),
                HEdge::A(c, rr_, s, en) => {
                    let span = (en - s).abs();
                    let ns = (-en).rem_euclid(360.0);
                    HEdge::A([c[0], -c[1]], *rr_, ns, ns + span)
                }
            })
            .collect();
        for (ang, scale, edges) in &hs {
            let wa = if edges_close(edges, &outer_up) {
                0.0
            } else if edges_close(edges, &inner_up) {
                90.0
            } else if edges_close(edges, &outer_lo) {
                270.0
            } else if edges_close(edges, &inner_lo) {
                180.0
            } else {
                panic!("bearing_276 未知边界片: {edges:?}");
            };
            assert!((scale - 1.0).abs() < 1e-9, "bearing_276 scale {scale}");
            assert!((ang - wa).abs() < 1e-9, "bearing_276 angle {ang} ≠ {wa}");
        }
    }

    /// 视图校验 / 错误信息。
    #[test]
    fn view_validation() {
        assert!(generate("eye_bolt_825", 100.0, 140.0, "top").unwrap().is_err());
        assert!(generate("seal_fb", 16.0, 30.0, "section").unwrap().is_err());
        assert!(generate("bearing_276", 35.0, 7.0, "end").unwrap().is_err());
        assert!(generate("hex_bolt_c", 5.0, 25.0, "main").is_none());
        assert!(generate("eye_bolt_825", 999.0, 1.0, "main").unwrap().is_err());
    }

    /// 人工核对：落 SVG（`cargo test -p ocs_ocsm --lib partgen_b3:: -- --ignored dump_b3_svg`）。
    #[test]
    #[ignore]
    fn dump_b3_svg() {
        std::fs::create_dir_all("/tmp/b3").unwrap();
        for (fam, d, l) in [
            ("eye_bolt_825", 100.0, 140.0),
            ("eye_bolt_825", 16.0, 28.0),
            ("seal_fb", 16.0, 30.0),
            ("seal_fb", 45.0, 62.0),
            ("bearing_276", 35.0, 7.0),
            ("bearing_276", 0.6, 1.0),
            ("bearing_276", 50.0, 10.0),
        ] {
            for view in family_views(fam) {
                let p = gen_all(fam, d, l, view).unwrap();
                let f = format!("/tmp/b3/{}-{}-{}.svg", fam, trim(d), view);
                dump_svg(&p, &f).unwrap();
                println!("写出 {f}");
            }
        }
    }

    // ── 可视化叠合：模板 1轮廓实线层（红）对生成几何（蓝）──

    struct TplEnt {
        kind: char,
        n: Vec<f64>,
    }
    fn l(x1: f64, y1: f64, x2: f64, y2: f64) -> TplEnt {
        TplEnt { kind: 'L', n: vec![x1, y1, x2, y2] }
    }
    fn a(cx: f64, cy: f64, r: f64, a1: f64, a2: f64) -> TplEnt {
        TplEnt { kind: 'A', n: vec![cx, cy, r, a1, a2] }
    }
    fn pc(v: Vec<f64>) -> TplEnt {
        TplEnt { kind: 'P', n: v }
    }

    /// 吊环螺钉 M100 左视图模板（右半，镜像由 `tpl_pts` 生成）。
    fn tpl_eye_end() -> Vec<TplEnt> {
        vec![
            a(0.0, 310.4, 39.6, 0.0, 180.0),
            l(39.6, 310.4, 44.0, 47.52),
            l(44.0, 47.52, 52.8, 39.6),
            pc(vec![0.0, 0.0, 102.1, 0.0, 89.7, 62.0]),
            pc(vec![43.54, 75.0, 89.7, 62.0, 43.758, 62.0]),
            pc(vec![50.0, 0.0, 50.0, -12.0, 0.0, -12.0]),
            pc(vec![50.0, -12.0, 50.0, -132.5, 0.0, -132.5]),
            pc(vec![50.0, -132.5, 42.5, -140.0, 0.0, -140.0]),
        ]
    }

    /// 吊环螺钉 M100 主视图模板（右半，含镜像侧由 `sgn` 生成）。
    fn tpl_eye_main() -> Vec<TplEnt> {
        vec![
            a(0.0, 175.0, 175.0, 311.3709, 90.0),
            a(142.1, 13.6538, 40.0, 131.371, 180.0),
            l(102.1, 0.0, 0.0, 0.0),
            l(102.1, 0.0, 102.1, 13.6538),
            pc(vec![0.0, 75.0, 70.0, 62.0, 80.0, 55.8]),
            pc(vec![0.0, 62.0, 70.0, 62.0]),
            pc(vec![50.0, 0.0, 50.0, -12.0, 0.0, -12.0]),
            pc(vec![50.0, -12.0, 50.0, -132.5, 0.0, -132.5]),
            pc(vec![50.0, -132.5, 42.5, -140.0, 0.0, -140.0]),
        ]
    }
    /// 密封圈 D30×d16×b7 模板（右半）。
    fn tpl_seal() -> Vec<TplEnt> {
        let mut v = Vec::new();
        for [u, h] in [[1.0, 0.95], [0.9, 0.383329], [0.8, 0.0], [0.5, 0.166671], [0.3, 0.25], [0.05, 0.041671], [0.0, 0.95]] {
            let xx = 8.0 + h * 7.0;
            let yy = u * 7.0;
            v.push(l(-xx, yy, xx, yy));
        }
        for seg in SEAL_SEGS {
            let xx = |u: f64| 8.0 + u * 7.0;
            let yy = |vv: f64| vv * 7.0;
            v.push(l(xx(seg[0]), yy(seg[1]), xx(seg[2]), yy(seg[3])));
        }
        v
    }
    /// 轴承 61807 模板（上半）。
    fn tpl_bearing() -> Vec<TplEnt> {
        vec![
            l(0.0, 0.0, 0.0, 23.2),
            l(7.0, 23.2, 7.0, 0.0),
            l(0.3, 23.5, 6.7, 23.5),
            a(0.3, 23.2, 0.3, 90.0, 180.0),
            a(6.7, 23.2, 0.3, 0.0, 90.0),
            pc(vec![2.201, 21.25, 0.0, 21.25, 0.0, 23.2]),
            pc(vec![7.0, 23.2, 7.0, 21.25]),
            pc(vec![7.0, 21.25, 4.799, 21.25]),
            a(3.5, 20.5, 1.5, 30.0, 150.0),
            a(3.5, 20.5, 1.5, 210.0, 330.0),
            pc(vec![0.0, 17.8, 0.0, 19.75, 2.201, 19.75]),
            pc(vec![4.799, 19.75, 7.0, 19.75, 7.0, 17.8]),
            l(6.7, 17.5, 0.3, 17.5),
            a(0.3, 17.8, 0.3, 180.0, 270.0),
            a(6.7, 17.8, 0.3, 270.0, 360.0),
        ]
    }

    /// 模板图元离散成采样点（含镜像/对称）。
    fn tpl_pts(tpl: &[TplEnt], mirrors: &[char]) -> Vec<(f64, f64)> {
        let mut out = Vec::new();
        // 模板只存了半个视：先放原件（恒等），再加镜像。
        let mut ms: Vec<char> = vec![' '];
        ms.extend_from_slice(mirrors);
        for e in tpl {
            for m in &ms {
                let (fx, fy) = match *m {
                    'x' => (-1.0, 1.0),
                    'y' => (1.0, -1.0),
                    _ => (1.0, 1.0),
                };
                match e.kind {
                    'L' => {
                        let (x1, y1, x2, y2) = (e.n[0] * fx, e.n[1] * fy, e.n[2] * fx, e.n[3] * fy);
                        for i in 0..=24 {
                            let t = i as f64 / 24.0;
                            out.push((x1 + (x2 - x1) * t, y1 + (y2 - y1) * t));
                        }
                    }
                    'A' => {
                        let (cx, cy, r) = (e.n[0] * fx, e.n[1] * fy, e.n[2]);
                        let (mut a1, mut a2) = (e.n[3], e.n[4]);
                        if fx < 0.0 {
                            let (n1, n2) = (180.0 - a2, 180.0 - a1);
                            a1 = n1;
                            a2 = n2;
                        }
                        if fy < 0.0 {
                            let (n1, n2) = (-a2, -a1);
                            a1 = n1;
                            a2 = n2;
                        }
                        for i in 0..=24 {
                            let ang = (a1 + (a2 - a1) * (i as f64 / 24.0)).to_radians();
                            out.push((cx + r * ang.cos(), cy + r * ang.sin()));
                        }
                    }
                    _ => {
                        let pts: Vec<(f64, f64)> =
                            e.n.chunks(2).map(|c| (c[0] * fx, c[1] * fy)).collect();
                        for w in pts.windows(2) {
                            for i in 0..=24 {
                                let t = i as f64 / 24.0;
                                out.push((
                                    w[0].0 + (w[1].0 - w[0].0) * t,
                                    w[0].1 + (w[1].1 - w[0].1) * t,
                                ));
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// 生成件离散成线段集合（用于精确的点到线段距离）。
    fn gen_segs(p: &GenPart) -> Vec<((f64, f64), (f64, f64))> {
        let mut out = Vec::new();
        for e in &p.entities {
            match e {
                EntityType::Line(ln) => {
                    out.push(((ln.start.x, ln.start.y), (ln.end.x, ln.end.y)));
                }
                EntityType::Arc(a) => {
                    let (a1, a2) = (a.start_angle.to_degrees(), a.end_angle.to_degrees());
                    for i in 0..24 {
                        let f = |k: f64| a.center.x + a.radius * k.to_radians().cos();
                        let g = |k: f64| a.center.y + a.radius * k.to_radians().sin();
                        out.push(((f(a1 + (a2 - a1) * i as f64 / 24.0), g(a1 + (a2 - a1) * i as f64 / 24.0)),
                                  (f(a1 + (a2 - a1) * (i + 1) as f64 / 24.0), g(a1 + (a2 - a1) * (i + 1) as f64 / 24.0))));
                    }
                }
                EntityType::Circle(c) => {
                    for i in 0..24 {
                        let f = |k: f64| c.center.x + c.radius * k.to_radians().cos();
                        let g = |k: f64| c.center.y + c.radius * k.to_radians().sin();
                        out.push(((f(360.0 * i as f64 / 24.0), g(360.0 * i as f64 / 24.0)),
                                  (f(360.0 * (i + 1) as f64 / 24.0), g(360.0 * (i + 1) as f64 / 24.0))));
                    }
                }
                EntityType::LwPolyline(pl) => {
                    let v: Vec<(f64, f64)> =
                        pl.vertices.iter().map(|x| (x.location.x, x.location.y)).collect();
                    for w in v.windows(2) {
                        out.push((w[0], w[1]));
                    }
                }
                _ => {}
            }
        }
        out
    }

    fn pt_seg_dist(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = dx * dx + dy * dy;
        let t = if len2 < 1e-18 {
            0.0
        } else {
            (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
        };
        ((p.0 - (a.0 + t * dx)).powi(2) + (p.1 - (a.1 + t * dy)).powi(2)).sqrt()
    }

    fn worst_dev(tpl: &[(f64, f64)], segs: &[((f64, f64), (f64, f64))]) -> f64 {
        tpl.iter()
            .map(|t| {
                segs.iter()
                    .map(|(a, b)| pt_seg_dist(*t, *a, *b))
                    .fold(f64::MAX, f64::min)
            })
            .fold(0.0, f64::max)
    }

    /// 叠合数值断言：模板每条轮廓都能在生成件里找到 ≤0.05 的对应图元。
    #[test]
    fn overlay_geometry_matches_template() {
        // (族, d, l, 视图, 模板, 镜像轴, 容差)
        for (fam, d, l, view, tpl, mirrors, tol) in [
            ("eye_bolt_825", 100.0, 140.0, "main", tpl_eye_main(), vec!['x'], 0.05),
            ("eye_bolt_825", 100.0, 140.0, "end", tpl_eye_end(), vec!['x'], 0.6),
            ("seal_fb", 16.0, 30.0, "main", tpl_seal(), vec!['x'], 0.05),
            ("bearing_276", 35.0, 7.0, "main", tpl_bearing(), vec!['y'], 0.05),
        ] {
            let p = gen_all(fam, d, l, view).unwrap();
            let w = worst_dev(&tpl_pts(&tpl, &mirrors), &gen_segs(&p));
            assert!(w < tol, "{fam} {view} 模板与生成件最大偏离 {w:.4} > {tol}");
        }
    }

    /// 可视化叠合产物（人工看红蓝是否重合）。
    #[test]
    #[ignore]
    fn overlay_b3() {
        std::fs::create_dir_all("/tmp/b3").unwrap();
        for (fam, d, l, view, tpl, mirrors) in [
            ("eye_bolt_825", 100.0, 140.0, "main", tpl_eye_main(), vec!['x']),
            ("eye_bolt_825", 100.0, 140.0, "end", tpl_eye_end(), vec!['x']),
            ("seal_fb", 16.0, 30.0, "main", tpl_seal(), vec!['x']),
            ("bearing_276", 35.0, 7.0, "main", tpl_bearing(), vec!['y']),
        ] {
            let p = gen_all(fam, d, l, view).unwrap();
            let path = format!("/tmp/b3/overlay-{fam}-{view}.svg");
            overlay_svg(
                &format!("{fam} {view} (red=template, blue=generated)"),
                &tpl_pts(&tpl, &mirrors),
                &p,
                &path,
            );
            println!("写出 {path}");
        }
    }

    fn overlay_svg(title: &str, tpl: &[(f64, f64)], part: &GenPart, path: &str) {
        // 模板采样点 → 红点；生成件 → 蓝折线
        let mut pts: Vec<(f64, f64)> = tpl.to_vec();
        let gen = gen_segs(part);
        for (a, b) in &gen {
            pts.push(*a);
            pts.push(*b);
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for p in &pts {
            x0 = x0.min(p.0);
            y0 = y0.min(p.1);
            x1 = x1.max(p.0);
            y1 = y1.max(p.1);
        }
        let (w, h) = ((x1 - x0).max(1e-6), (y1 - y0).max(1e-6));
        let pad = 0.05 * w.max(h);
        let px = 1200.0_f64;
        let sc = px / (w + 2.0 * pad);
        let py = (h + 2.0 * pad) * sc;
        let t = |x: f64, y: f64| ((x - x0 + pad) * sc, py - (y - y0 + pad) * sc);
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{px:.0}\" height=\"{py:.0}\"><rect width=\"100%\" height=\"100%\" fill=\"#fff\"/><text x=\"10\" y=\"20\" font-size=\"16\">{title}</text>"
        );
        for p in tpl {
            let q = t(p.0, p.1);
            out.push_str(&format!(
                "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"1.3\" fill=\"#e00000\"/>",
                q.0, q.1
            ));
        }
        for e in &part.entities {
            let seg = |a: (f64, f64), b: (f64, f64)| {
                let (qa, qb) = (t(a.0, a.1), t(b.0, b.1));
                format!(
                    "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"#0033dd\" stroke-width=\"1\"/>",
                    qa.0, qa.1, qb.0, qb.1
                )
            };
            match e {
                EntityType::Line(ln) => out.push_str(&seg(
                    (ln.start.x, ln.start.y),
                    (ln.end.x, ln.end.y),
                )),
                EntityType::Arc(a) => {
                    let (a1, a2) = (a.start_angle.to_degrees(), a.end_angle.to_degrees());
                    for i in 0..24 {
                        let f = |k: f64| k.to_radians();
                        let p1 = (
                            a.center.x + a.radius * f(a1 + (a2 - a1) * i as f64 / 24.0).cos(),
                            a.center.y + a.radius * f(a1 + (a2 - a1) * i as f64 / 24.0).sin(),
                        );
                        let p2 = (
                            a.center.x + a.radius * f(a1 + (a2 - a1) * (i + 1) as f64 / 24.0).cos(),
                            a.center.y + a.radius * f(a1 + (a2 - a1) * (i + 1) as f64 / 24.0).sin(),
                        );
                        out.push_str(&seg(p1, p2));
                    }
                }
                EntityType::Circle(c) => {
                    for i in 0..24 {
                        let f = |k: f64| k.to_radians();
                        let p1 = (
                            c.center.x + c.radius * f(360.0 * i as f64 / 24.0).cos(),
                            c.center.y + c.radius * f(360.0 * i as f64 / 24.0).sin(),
                        );
                        let p2 = (
                            c.center.x + c.radius * f(360.0 * (i + 1) as f64 / 24.0).cos(),
                            c.center.y + c.radius * f(360.0 * (i + 1) as f64 / 24.0).sin(),
                        );
                        out.push_str(&seg(p1, p2));
                    }
                }
                EntityType::LwPolyline(pl) => {
                    let v: Vec<(f64, f64)> =
                        pl.vertices.iter().map(|x| (x.location.x, x.location.y)).collect();
                    for i in 0..v.len().saturating_sub(1) {
                        out.push_str(&seg(v[i], v[i + 1]));
                    }
                }
                _ => {}
            }
        }
        out.push_str("</svg>");
        std::fs::write(path, out).unwrap();
    }

    /// SVG 冒烟。
    #[test]
    fn svg_smoke() {
        let p = gen_all("bearing_276", 35.0, 7.0, "main").unwrap();
        assert!(part_svg(&p, 400.0, 300.0).contains("<svg"));
    }
}
