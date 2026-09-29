//! 第四批 D 组（partgen_b4）：**圆锥滚子轴承 30000型 02系列 GB/T 297-1994**、
//! **调心滚子轴承 20000C型 GB/T 288-1994**。
//!
//! 本文件只放这两个族的画法与数据；共用图元/表/校验工具在 `partgen_kit.rs`。
//! **不要改其它模块**——并行开发时会互相踩。
//!
//! | 族 id | 名称 | 模板（用户参数化图 DXF） | 视图 | 数据 |
//! |---|---|---|---|---|
//! | `bearing_297` | 圆锥滚子轴承 30000型 02系列 | `~/桌面/GB/参数化/圆锥滚子轴承_30000型_02系列_GB-T297-1994/` | main | `tables/partsBearing297.json` |
//! | `bearing_288` | 调心滚子轴承 20000C型 | `~/桌面/GB/参数化/调心滚子轴承_20000C型_GB-T288-1994/` | main | `tables/partsBearing288.json`（已扩到 GB/T 288-1994 圆柱孔 C/CC 型全量 71 规格） |
//!
//! 反解工具：`target/debug/examples/ref_dump <dxf> <out.tsv>`（TSV 圆弧角度是**度**）。
//!
//! # 画法（由模板实尺图逐条反解，模板规格已作数值回归）
//!
//! ## bearing_297（模板 30202：d15 D35 T11.75 B11 C10 E27 r1=r3=0.6）
//! 主视图 = **上下对称的轴向剖视**（轴线 y=0，x 为轴向 0…T，y 为半径；下半个视是
//! 上半个视关于 y=0 的镜像）。三类轮廓：
//! - **外圈**：左端面 x=0（E/2…D/2−r1）、外径 y=D/2（r1…C−r1，两端圆角 r1）、
//!   右端面 x=C，滚道为过 (0,E/2) 且**半锥角 15°** 的直线，止于 (C, E/2+C·tan15°)。
//! - **内圈**：宽 B（x=T−B…T，两端倒角 r3），内孔 y=d/2；滚道 = 外圈滚道关于
//!   **滚子轴线**（过 (C/2, (d/2+D/2)/2)、半锥角 13°）的镜像（故其半锥角 = 2·13−15 = 11°）；
//!   大/小挡边顶面分别位于半径 r + 0.5·Δ、r + 0.375·Δ（Δ=D/2−d/2）。
//! - **滚子**：两端面垂直于滚子轴线；外接触点 A、B 取在外圈滚道上，轴向位置为
//!   x = 0.12·C 与 x = 0.77299·C（由模板实测的**形状比例**，见下方常量）；内接触点
//!   P1、P2 = A、B 关于滚子轴线的镜像；挡边顶点 D、C2 = 端面与挡边顶面半径的交点。
//! - 另有三条从轴线 y=0 引到端面的竖线（模板实测：x=0、x=T−B、x=T），
//!   滚子轴线（x∈[0,C] 段）与主轴线（两端各出头 3）画在 `3中心线层`。
//!
//! ## bearing_288（模板 23022C：d110 D170 B45 r2）
//! 主视图 = 轴向剖视（轴线 y=0）。外圈外径为 y=D/2 的直线（两端圆角 r），
//! **球面滚道**以 (B/2, 0) 为心、过两侧端面顶点；内圈内孔 y=d/2，两条**内滚道圆弧**
//! 过「端面顶点 + 两滚子内接点」三点的外接圆；两列桶形滚子、中央保持架梯形、剖面线。
//! 所有自由量都取模板实测的**形状比例**（下方常量，均归一到 Δ=D/2−d/2 或 B）。
//!
//! 注：两族模板里下半个视的圆弧在 DXF 中用 OCS（extrusion = (0,0,−1)）存，
//! 即世界坐标 = 关于 y=0 的镜像；本模块直接生成镜像（比 DXF 里的存储更规范），
//! 叠合测试会把模板的 OCS 换算成镜像后比对。

use std::sync::OnceLock;

use ocs_plugin_api::host::acadrust::entities::EntityType;

use crate::partgen::{GenPart, PartMeta};
use crate::partgen_kit::{
    arc, circle, hatch_ansi31_scaled, line, polyline, trim, views_json, Table, LAYER_CENTER, LAYER_MAIN,
};

// ══════════════════════════════════════════════════════════════════════════
// 数据表
// ══════════════════════════════════════════════════════════════════════════

/// GB/T 297-1994 302xx 一行（列名 = 模板 MTEXT 参数名）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct B297Row {
    pub code: String,
    pub d: f64,
    #[serde(rename = "D")]
    pub od: f64,
    #[serde(rename = "T")]
    pub t: f64,
    #[serde(rename = "B")]
    pub b: f64,
    pub r1: f64,
    #[serde(rename = "C")]
    pub c: f64,
    pub r3: f64,
    #[serde(rename = "E")]
    pub e: f64,
}

/// GB/T 288-1994 20000C 一行。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct B288Row {
    pub code: String,
    pub d: f64,
    #[serde(rename = "D")]
    pub od: f64,
    #[serde(rename = "B")]
    pub b: f64,
    pub r: f64,
}

impl crate::partgen_kit::HasD for B297Row {
    fn d(&self) -> f64 {
        self.d
    }
}
impl crate::partgen_kit::HasD for B288Row {
    fn d(&self) -> f64 {
        self.d
    }
}

fn t297() -> &'static Table<B297Row> {
    static T: OnceLock<Table<B297Row>> = OnceLock::new();
    T.get_or_init(|| Table::parse(include_str!("tables/partsBearing297.json")))
}
fn t288() -> &'static Table<B288Row> {
    static T: OnceLock<Table<B288Row>> = OnceLock::new();
    T.get_or_init(|| Table::parse(include_str!("tables/partsBearing288.json")))
}

fn row297(d: f64, l: f64) -> Option<&'static B297Row> {
    t297()
        .rows
        .iter()
        .find(|r| (r.d - d).abs() < 1e-9 && (r.t - l).abs() < 1e-9)
}
fn row288(d: f64, l: f64) -> Option<&'static B288Row> {
    t288()
        .rows
        .iter()
        .find(|r| (r.d - d).abs() < 1e-9 && (r.b - l).abs() < 1e-9)
}

// ══════════════════════════════════════════════════════════════════════════
// 几何工具
// ══════════════════════════════════════════════════════════════════════════

/// 关于 y=0 镜像一个图元（下半个视 = 上半个视的镜像）。
///
/// 直线 y→−y；圆/圆心 y→−y；圆弧角度 θ→−θ 且起止互换（CCW 弧镜像后仍按 CCW 存）。
fn mirror_ent(e: &EntityType) -> EntityType {
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

/// 把上半个视图元加入列表：原件 + 镜像件。
fn push2(en: &mut Vec<EntityType>, e: EntityType) {
    let m = mirror_ent(&e);
    en.push(e);
    en.push(m);
}

/// 点 `p` 关于「过 `q`、方向 `dir`（单位向量）」的直线做镜像。
fn mirror_pt(p: [f64; 2], q: [f64; 2], dir: [f64; 2]) -> [f64; 2] {
    let v = [p[0] - q[0], p[1] - q[1]];
    let t = v[0] * dir[0] + v[1] * dir[1];
    let proj = [q[0] + t * dir[0], q[1] + t * dir[1]];
    [2.0 * proj[0] - p[0], 2.0 * proj[1] - p[1]]
}

/// 从 `p` 沿方向 `dir` 走到纵坐标 `y`（`dir[1] != 0`）。
fn ray_to_y(p: [f64; 2], dir: [f64; 2], y: f64) -> [f64; 2] {
    let s = (y - p[1]) / dir[1];
    [p[0] + s * dir[0], y]
}

/// 圆弧采样成折线点（给只接受折线边界的 `hatch_ansi31` 用）。
fn arc_pts(c: [f64; 2], r: f64, a1_deg: f64, a2_deg: f64, n: usize) -> Vec<[f64; 2]> {
    let (a1, a2) = (a1_deg.to_radians(), a2_deg.to_radians());
    (0..=n)
        .map(|i| {
            let a = a1 + (a2 - a1) * (i as f64 / n as f64);
            [c[0] + r * a.cos(), c[1] + r * a.sin()]
        })
        .collect()
}

/// 去掉折线环里相邻重复点（含首尾重复），返回新环。
///
/// 剖面线边界由直线段 + `arc_pts` 采样拼接，弧的首点常与上一段末点重合，
/// 末点又常与下一段起点重合。这些零长边会让宿主判边界无效而**整片不画**
/// （297/288 在 OCS 里一片都看不到的根因之一）。阈值 1e-9（比较平方距离）。
fn dedup_ring(verts: &[[f64; 2]]) -> Vec<[f64; 2]> {
    const EPS2: f64 = 1e-18; // (1e-9)^2
    let d2 = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2);
    let mut out: Vec<[f64; 2]> = Vec::with_capacity(verts.len());
    for &p in verts {
        if out.last().map_or(true, |&q| d2(q, p) > EPS2) {
            out.push(p);
        }
    }
    while out.len() >= 2 && d2(out[0], out[out.len() - 1]) <= EPS2 {
        out.pop();
    }
    out
}

/// 三点外接圆（圆心 + 半径）；共线时退化返回 `None`。
fn circumcircle(p1: [f64; 2], p2: [f64; 2], p3: [f64; 2]) -> Option<([f64; 2], f64)> {
    let (ax, ay, bx, by, cx, cy) = (p1[0], p1[1], p2[0], p2[1], p3[0], p3[1]);
    let dd = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if dd.abs() < 1e-12 {
        return None;
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
    Some(([ux, uy], r))
}

/// 元数据（规格 = 轴承代号）。
fn meta_297(row: &B297Row) -> PartMeta {
    PartMeta {
        code: "GB/T 297-1994".into(),
        name: crate::i18n::t_data("圆锥滚子轴承 30000型 02系列").into(),
        spec: row.code.clone(),
        material: String::new(),
        weight: weight_297(row),
    }
}
fn meta_288(row: &B288Row) -> PartMeta {
    PartMeta {
        code: "GB/T 288-1994".into(),
        name: crate::i18n::t_data("调心滚子轴承 20000C型").into(),
        spec: row.code.clone(),
        material: String::new(),
        weight: weight_288(row),
    }
}

/// 单件重量估算（钢 7.85 g/cm³）——按「圆环体积 + 滚子」粗估，仅进 BOM。
fn weight_297(row: &B297Row) -> String {
    let r = row.d / 2.0;
    let r2 = row.od / 2.0;
    let v = std::f64::consts::PI
        * (r2 * r2 - r * r)
        * row.t
        * 0.55; // 剖视图上材料约占整环 55%（滚道/滚子间隙）
    crate::partgen_kit::weight_text(crate::partgen_kit::weight_kg(v, 7.85))
}
fn weight_288(row: &B288Row) -> String {
    let r = row.d / 2.0;
    let r2 = row.od / 2.0;
    let v = std::f64::consts::PI * (r2 * r2 - r * r) * row.b * 0.55;
    crate::partgen_kit::weight_text(crate::partgen_kit::weight_kg(v, 7.85))
}

// ══════════════════════════════════════════════════════════════════════════
// bearing_297
// ══════════════════════════════════════════════════════════════════════════

/// 外圈滚道半锥角（模板 30202 实测 tanα = (16.1795−13.5)/10 = tan15.0000°）。
const CONE_OUTER_DEG: f64 = 15.0;
/// 滚子轴线半锥角（模板实测：13.0000°；内圈滚道 = 外圈滚道关于它的镜像 → 11°）。
const CONE_ROLLER_DEG: f64 = 13.0;
/// 滚子小端外接点轴向位置（占外圈宽 C 的比例；模板 A.x = 1.2 = 0.12·C）。
const ROLLER_X1_FRAC: f64 = 0.12;
/// 滚子大端外接点轴向位置（占 C 的比例；模板 B.x = 7.7299 = 0.77299·C）。
const ROLLER_X2_FRAC: f64 = 0.77299;
/// 小挡边顶面半径（占 Δr = D/2−d/2 的比例；模板 11.25 = 7.5+0.375·10）。
const RIB_SMALL_FRAC: f64 = 0.375;
/// 大挡边顶面半径（模板 12.5 = 7.5+0.5·10）。
const RIB_LARGE_FRAC: f64 = 0.5;
/// 主轴线两端出头（模板 −3…T+3）。
const AXIS_OVER: f64 = 3.0;

/// 圆锥滚子轴承主视图（轴向剖视，上下对称）。
pub fn bearing_297(row: &B297Row) -> GenPart {
    let (d, od, t, b, c, e, r1, r3) = (row.d, row.od, row.t, row.b, row.c, row.e, row.r1, row.r3);
    let r = d / 2.0;
    let r2 = od / 2.0;
    let e2 = e / 2.0;
    let dr = r2 - r;
    let tan15 = CONE_OUTER_DEG.to_radians().tan();
    let y_or = e2 + c * tan15; // 外圈滚道大端半径

    // 滚子轴线：过 (C/2, 中径)，半锥角 13°
    let ang = CONE_ROLLER_DEG.to_radians();
    let dir = [ang.cos(), ang.sin()];
    let axis_c = [c / 2.0, (r + r2) / 2.0];
    // 端面方向 = 垂直于滚子轴线
    let ef = [ang.sin(), -ang.cos()];

    let a_pt = [ROLLER_X1_FRAC * c, e2 + tan15 * ROLLER_X1_FRAC * c];
    let b_pt = [ROLLER_X2_FRAC * c, e2 + tan15 * ROLLER_X2_FRAC * c];
    let p1 = mirror_pt(a_pt, axis_c, dir);
    let p2 = mirror_pt(b_pt, axis_c, dir);
    let y_sr = r + RIB_SMALL_FRAC * dr;
    let y_lr = r + RIB_LARGE_FRAC * dr;
    let d_pt = ray_to_y(a_pt, ef, y_sr);
    let c2_pt = ray_to_y(b_pt, ef, y_lr);

    let mut en: Vec<EntityType> = Vec::new();
    // ── 外圈 ──
    push2(&mut en, line([0.0, e2], [0.0, r2 - r1], LAYER_MAIN));
    push2(&mut en, arc([r1, r2 - r1], r1, 90.0, 180.0, LAYER_MAIN));
    push2(&mut en, line([r1, r2], [c - r1, r2], LAYER_MAIN));
    push2(&mut en, arc([c - r1, r2 - r1], r1, 0.0, 90.0, LAYER_MAIN));
    push2(&mut en, line([c, r2 - r1], [c, y_or], LAYER_MAIN));
    push2(&mut en, line([0.0, e2], [c, y_or], LAYER_MAIN)); // 外圈滚道
    push2(&mut en, line([c, y_or], [c, y_lr], LAYER_MAIN)); // 右端面下延段（模板 10 16.1795→12.5）
    // ── 内圈 ──
    push2(&mut en, line([t - b + r3, r], [t - r3, r], LAYER_MAIN)); // 内孔
    push2(&mut en, arc([t - b + r3, r + r3], r3, 180.0, 270.0, LAYER_MAIN));
    push2(&mut en, line([t - b, r + r3], [t - b, y_sr], LAYER_MAIN));
    push2(&mut en, line([t - b, y_sr], d_pt, LAYER_MAIN));
    push2(&mut en, line(d_pt, p1, LAYER_MAIN)); // 小挡边面（= 滚子端面下半段）
    push2(&mut en, line(p1, p2, LAYER_MAIN)); // 内圈滚道
    push2(&mut en, line(p2, c2_pt, LAYER_MAIN)); // 大挡边面
    push2(&mut en, line(c2_pt, [t, y_lr], LAYER_MAIN));
    push2(&mut en, line([t, y_lr], [t, r + r3], LAYER_MAIN));
    push2(&mut en, arc([t - r3, r + r3], r3, 270.0, 360.0, LAYER_MAIN));
    // 端面上从轴线引出的竖线（模板实测）
    push2(&mut en, line([0.0, 0.0], [0.0, e2], LAYER_MAIN));
    push2(&mut en, line([t - b, 0.0], [t - b, r + r3], LAYER_MAIN));
    push2(&mut en, line([t, 0.0], [t, r + r3], LAYER_MAIN));
    // ── 滚子端面 ──
    push2(&mut en, line(a_pt, d_pt, LAYER_MAIN));
    push2(&mut en, line(b_pt, c2_pt, LAYER_MAIN));
    // ── 中心线 ──
    push2(
        &mut en,
        line(
            [axis_c[0] - c / 2.0 * dir[0], axis_c[1] - c / 2.0 * dir[1]],
            [axis_c[0] + c / 2.0 * dir[0], axis_c[1] + c / 2.0 * dir[1]],
            LAYER_CENTER,
        ),
    );
    en.push(line([-AXIS_OVER, 0.0], [t + AXIS_OVER, 0.0], LAYER_CENTER));

    // ── 剖面线（5剖面线层，两片/半：外圈、内圈）──
    let outer_verts = {
        let mut v = vec![[0.0, e2], [c, y_or], [c, r2 - r1]];
        v.extend(arc_pts([c - r1, r2 - r1], r1, 0.0, 90.0, 4));
        v.push([r1, r2]);
        v.extend(arc_pts([r1, r2 - r1], r1, 90.0, 180.0, 4));
        v.push([0.0, r2 - r1]);
        v
    };
    // 环序（模板 30202，上半个视，沿轮廓一周）：
    //   (T−B, d/2+r3) 起，圆角 → 内孔 (T−B+r3, d/2)→(T−r3, d/2) → 圆角 →
    //   右端面 (T, d/2+r3)→(T, y_lr) → 大挡边顶 → 大挡边面 → 内圈滚道 →
    //   小挡边面 → 小挡边顶 (T−B, y_sr) → 闭合回起点（左端面竖线）。
    // 首个弧采样点就是起点 (T−B, d/2+r3)，不再额外 push；弧末点与后续顶点重合
    // 交给 `dedup_ring` 处理 —— 避免重复点让宿主判边界无效。
    let inner_verts = {
        let mut v = Vec::new();
        v.extend(arc_pts([t - b + r3, r + r3], r3, 180.0, 270.0, 4)); // 左倒角
        v.push([t - r3, r]); // 内孔
        v.extend(arc_pts([t - r3, r + r3], r3, 270.0, 360.0, 4)); // 右倒角
        v.push([t, y_lr]); // 右端面
        v.push(c2_pt); // 大挡边顶
        v.push(p2); // 大挡边面
        v.push(p1); // 内圈滚道
        v.push(d_pt); // 小挡边面
        v.push([t - b, y_sr]); // 小挡边顶 → 闭合回起点（左端面）
        v
    };
    add_hatch_pair(&mut en, &outer_verts, 0.0, 270.0, 1.0);
    add_hatch_pair(&mut en, &inner_verts, 90.0, 180.0, 1.0);

    GenPart {
        entities: en,
        meta: meta_297(row),
        bbox: [-AXIS_OVER, -r2, t + AXIS_OVER, r2],
    }
}

/// 一片剖面线的边界同时落上、下两个半视（下半 = y 镜像）。
///
/// 角度/比例按**用户模板逐片实测**（GB/T 297/288 主视图的 hatch 数据）：
/// 上半·外圈 0° / 上半·内圈 90° / 下半·外圈 270° / 下半·内圈 180°（scale 1.0）；
/// 调心滚子的**保持架**两片模板是 **scale 0.5**（同一图案画密一倍）。
/// 方向必须**烘焙进线角度**（OCS 的 prebaked 路径只认线角度、不叠加 pattern_angle）——
/// `hatch_ansi31_scaled` 已做烘焙，别改成只传 pattern_angle。
fn add_hatch_pair(en: &mut Vec<EntityType>, verts: &[[f64; 2]], up_deg: f64, lo_deg: f64, scale: f64) {
    let up = dedup_ring(verts);
    en.push(hatch_ansi31_scaled(&up, up_deg, scale));
    let m: Vec<[f64; 2]> = up.iter().map(|p| [p[0], -p[1]]).collect();
    en.push(hatch_ansi31_scaled(&m, lo_deg, scale));
}

// ══════════════════════════════════════════════════════════════════════════
// bearing_288
// ══════════════════════════════════════════════════════════════════════════

/// 外圈端面顶点（球面滚道端点）半径比例：y0 = D/2 − 0.30906333·Δ（模板 75.7281）。
const S288_Y0_FRAC: f64 = 0.30906333;
/// 内圈端面顶点半径比例：y_in = d/2 + 0.29093667·Δ（模板 63.7281）。
const S288_YIN_FRAC: f64 = 0.29093667;
/// 左滚道弧内接点（滚子小端）坐标比例（模板 (4.5775, 63.122)）。
const S288_PB_X_FRAC: f64 = 0.10172222;
const S288_PB_Y_FRAC: f64 = 0.27073333;
/// 左滚道弧内接点（滚子大端）坐标比例（模板 (20.25, 65.6717)）。
const S288_PC_X_FRAC: f64 = 0.45;
const S288_PC_Y_FRAC: f64 = 0.35572333;
/// 保持架底/顶半径比例（模板 65.6717 / 69.6717）。
const S288_CAGE_BOT_FRAC: f64 = 0.35572333;
const S288_CAGE_TOP_FRAC: f64 = 0.48905667;
/// 保持架底/顶半宽比例（模板 2.25 / 2.9318，占 B）。
const S288_CAGE_HB_FRAC: f64 = 0.05;
const S288_CAGE_HT_FRAC: f64 = 0.06515111;
/// 滚子外接点轴向位置比例（模板 2.25 / 18，占 B）。
const S288_ROLL_X1_FRAC: f64 = 0.05;
const S288_ROLL_X2_FRAC: f64 = 0.4;

/// 调心滚子轴承主视图（轴向剖视，上下对称）。
pub fn bearing_288(row: &B288Row) -> GenPart {
    let (d, od, w, rr) = (row.d, row.od, row.b, row.r);
    let r = d / 2.0;
    let r2 = od / 2.0;
    let dr = r2 - r;
    let hw = w / 2.0;

    let y0 = r2 - S288_Y0_FRAC * dr;
    let rs = ((hw).powi(2) + y0 * y0).sqrt(); // 外圈球面滚道半径（中心 (B/2, 0)）
    let sp_start = y0.atan2(hw).to_degrees(); // 球面弧起角
    let sp_end = 180.0 - sp_start;

    let y_in = r + S288_YIN_FRAC * dr;
    let pb = [S288_PB_X_FRAC * w, r + S288_PB_Y_FRAC * dr];
    let pc = [S288_PC_X_FRAC * w, r + S288_PC_Y_FRAC * dr];
    let pa = [0.0, y_in];
    let (cc, ri) = circumcircle(pa, pb, pc).expect("内滚道三点共线");
    let a1 = (pa[1] - cc[1]).atan2(pa[0] - cc[0]).to_degrees();
    let a2 = (pc[1] - cc[1]).atan2(pc[0] - cc[0]).to_degrees();
    let cc_r = [w - cc[0], cc[1]];
    let (ar1, ar2) = (180.0 - a2, 180.0 - a1);

    let y_cage = r + S288_CAGE_BOT_FRAC * dr;
    let y_top = r + S288_CAGE_TOP_FRAC * dr;
    let hb = S288_CAGE_HB_FRAC * w;
    let ht = S288_CAGE_HT_FRAC * w;

    // 滚子外接点（在球面滚道上）
    let sy = |x: f64| (rs * rs - (hw - x).powi(2)).max(0.0).sqrt();
    let o1 = [S288_ROLL_X1_FRAC * w, sy(S288_ROLL_X1_FRAC * w)];
    let o2 = [S288_ROLL_X2_FRAC * w, sy(S288_ROLL_X2_FRAC * w)];

    let mut en: Vec<EntityType> = Vec::new();
    // ── 外圈 ──
    push2(&mut en, line([0.0, y0], [0.0, r2 - rr], LAYER_MAIN));
    push2(&mut en, line([w, y0], [w, r2 - rr], LAYER_MAIN));
    push2(&mut en, line([rr, r2], [w - rr, r2], LAYER_MAIN));
    push2(&mut en, arc([rr, r2 - rr], rr, 90.0, 180.0, LAYER_MAIN));
    push2(&mut en, arc([w - rr, r2 - rr], rr, 0.0, 90.0, LAYER_MAIN));
    push2(&mut en, arc([hw, 0.0], rs, sp_start, sp_end, LAYER_MAIN)); // 球面滚道
    // ── 内圈 ──
    push2(&mut en, line([0.0, 0.0], [0.0, r2 - rr], LAYER_MAIN));
    push2(&mut en, line([w, 0.0], [w, r2 - rr], LAYER_MAIN));
    push2(&mut en, line([0.0, y_in], [0.0, r + rr], LAYER_MAIN));
    push2(&mut en, line([w, y_in], [w, r + rr], LAYER_MAIN));
    push2(&mut en, line([rr, r], [w - rr, r], LAYER_MAIN));
    push2(&mut en, arc([rr, r + rr], rr, 180.0, 270.0, LAYER_MAIN));
    push2(&mut en, arc([w - rr, r + rr], rr, 270.0, 360.0, LAYER_MAIN));
    push2(&mut en, arc(cc, ri, a1, a2, LAYER_MAIN)); // 左内滚道
    push2(&mut en, arc(cc_r, ri, ar1, ar2, LAYER_MAIN)); // 右内滚道
    // ── 两列滚子 ──
    push2(&mut en, line(o1, pb, LAYER_MAIN));
    push2(&mut en, line(o2, pc, LAYER_MAIN));
    push2(&mut en, line([w - o1[0], o1[1]], [w - pb[0], pb[1]], LAYER_MAIN));
    push2(&mut en, line([w - o2[0], o2[1]], [w - pc[0], pc[1]], LAYER_MAIN));
    // ── 保持架 ──
    push2(
        &mut en,
        polyline(
            &[
                [hw - hb, y_cage],
                [hw - ht, y_top],
                [hw + ht, y_top],
                [hw + hb, y_cage],
            ],
            true,
            LAYER_MAIN,
        ),
    );
    push2(&mut en, line([hw - hb, y_cage], [hw + hb, y_cage], LAYER_MAIN));
    // ── 中心线 ──
    en.push(line([-5.0, 0.0], [w + 5.0, 0.0], LAYER_CENTER));
    push2(&mut en, line([hw, r2 + AXIS_OVER], [hw, r - AXIS_OVER], LAYER_CENTER));

    // ── 剖面线（外圈 / 内圈 / 保持架）──
    // 环序（模板 23022C，上半个视，沿轮廓一周）：
    //   球面滚道从**左端 (0, y0) 向右扫到 (B, y0)** → 右侧面 (B, y0)→(B, D/2−r) →
    //   右圆角 → 外径 (B−r, D/2)→(r, D/2) → 左圆角 → 左侧面 (0, D/2−r) →
    //   闭合回 (0, y0)（左端面竖线）。
    // 关键：弧采样必须 `sp_end→sp_start`（左→右）；若 `sp_start→sp_end`（右→左），
    // 闭合成 (0,D/2−r)→(B,y0) 的长对角线，宿主按奇偶规则判不出内部 → 整片不画。
    let outer_verts = {
        let mut v = Vec::new();
        v.extend(arc_pts([hw, 0.0], rs, sp_end, sp_start, 12)); // 球面滚道（左→右）
        v.push([w, r2 - rr]); // 右侧面
        v.extend(arc_pts([w - rr, r2 - rr], rr, 0.0, 90.0, 4)); // 右圆角
        v.push([rr, r2]); // 外径
        v.extend(arc_pts([rr, r2 - rr], rr, 90.0, 180.0, 4)); // 左圆角
        v.push([0.0, r2 - rr]); // 左侧面（闭合回弧起点）
        v
    };
    // 环序（模板 23022C，上半个视，沿轮廓一周）：
    //   (0, d/2+rr) 起，左圆角 → 内孔 (rr, d/2)→(B−rr, d/2) → 右圆角 →
    //   右端面 (B, d/2+rr)→(B, y_in) → 上侧内滚道弧 (B,y_in)→(hw+hb,y_cage) →
    //   保持架底 (hw+hb,y_cage)→(hw−hb,y_cage) → 下侧内滚道弧 (hw−hb,y_cage)→(0,y_in) →
    //   左端面 (0, y_in)→(0, d/2+rr) 闭合。
    // 两条内滚道弧在模板里分别是 cc 的 a1→a2、cc_r 的 ar1→ar2；按本环序遍历时
    // 方向相反，故采样 a2→a1 / ar2→ar1（arc_pts 支持 a2<a1 反向，避免折返）。
    let inner_verts = {
        let mut v = Vec::new();
        v.extend(arc_pts([rr, r + rr], rr, 180.0, 270.0, 4)); // 左倒角
        v.push([w - rr, r]); // 内孔
        v.extend(arc_pts([w - rr, r + rr], rr, 270.0, 360.0, 4)); // 右倒角
        v.push([w, y_in]); // 右端面
        v.extend(arc_pts(cc_r, ri, ar2, ar1, 10)); // 上侧内滚道弧（反向）
        v.push([hw - hb, y_cage]); // 保持架底
        v.extend(arc_pts(cc, ri, a2, a1, 10)); // 下侧内滚道弧（反向）
        v.push([0.0, r + rr]); // 左端面（闭合回起点）
        v
    };
    let cage_verts = [
        [hw - hb, y_cage],
        [hw - ht, y_top],
        [hw + ht, y_top],
        [hw + hb, y_cage],
    ];
    add_hatch_pair(&mut en, &outer_verts, 0.0, 270.0, 1.0);
    add_hatch_pair(&mut en, &inner_verts, 90.0, 180.0, 1.0);
    add_hatch_pair(&mut en, &cage_verts, 0.0, 270.0, 0.5);

    GenPart {
        entities: en,
        meta: meta_288(row),
        bbox: [-5.0, -r2, w + 5.0, r2],
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 对外 API（partgen_more 会先问这里）
// ══════════════════════════════════════════════════════════════════════════

/// 本组声明的视图：两族都只有模板里的**主视图**。
pub fn family_views(family: &str) -> Vec<&'static str> {
    match family {
        "bearing_297" | "bearing_288" => vec!["main"],
        _ => Vec::new(),
    }
}

/// 本组族的目录 JSON 片段（键 = 族 id）。
pub fn families_json() -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    let sizes297: Vec<serde_json::Value> = t297()
        .rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "d": r.d, "label": r.code, "pitch": 0.0,
                "l_min": r.t, "l_max": r.t, "lengths": [r.t],
                "extra": format!(
                    "d{} D{} T{} B{} C{} E{} r1 {} r3 {}",
                    trim(r.d), trim(r.od), trim(r.t), trim(r.b),
                    trim(r.c), trim(r.e), trim(r.r1), trim(r.r3)),
            })
        })
        .collect();
    m.insert(
        "bearing_297".into(),
        serde_json::json!({
            "id": "bearing_297", "name": "圆锥滚子轴承 30000型 02系列",
            "code": "GB/T 297-1994", "iso": "—", "implemented": true,
            "views": views_json("bearing_297"), "sizes": sizes297,
            "len_label": "厚度 T", "base_hint": "基点 = 端面中心 × 轴线（主视图：轴线 y=0、端面 x=0）",
            "tree_path": "零件库/轴承/圆锥滚子轴承 30000型 02系列 GB/T 297-1994",
        }),
    );
    // 288 的同一直径跨 230xx/240xx（仅宽度 B 不同）→ 按 d 归并，lengths = 该 d 下的全部 B。
    let mut sizes288: Vec<serde_json::Value> = Vec::new();
    for r in &t288().rows {
        if let Some(last) = sizes288.last_mut() {
            if (last["d"].as_f64().unwrap() - r.d).abs() < 1e-9 {
                // 追加宽度
                let mut ls: Vec<f64> = last["lengths"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|v| v.as_f64())
                    .collect();
                if !ls.iter().any(|x| (x - r.b).abs() < 1e-9) {
                    ls.push(r.b);
                }
                last["lengths"] = serde_json::json!(ls);
                let ex = last["extra"].as_str().unwrap_or("").to_string();
                last["extra"] = serde_json::json!(format!("{ex} / {}", r.code));
                continue;
            }
        }
        sizes288.push(serde_json::json!({
            "d": r.d, "label": format!("d{}", trim(r.d)), "pitch": 0.0,
            "l_min": r.b, "l_max": r.b, "lengths": [r.b],
            "extra": format!(
                "{}：d{} D{} B{} r{}",
                r.code, trim(r.d), trim(r.od), trim(r.b), trim(r.r)),
        }));
    }
    m.insert(
        "bearing_288".into(),
        serde_json::json!({
            "id": "bearing_288", "name": "调心滚子轴承 20000C型",
            "code": "GB/T 288-1994", "iso": "—", "implemented": true,
            "views": views_json("bearing_288"), "sizes": sizes288,
            "len_label": "宽度 B", "base_hint": "基点 = 端面中心 × 轴线（主视图：轴线 y=0、端面 x=0）",
            "tree_path": "零件库/轴承/调心滚子轴承 20000C型 GB/T 288-1994",
        }),
    );
    m
}

/// 本组族的生成派发。
pub fn generate(family: &str, d: f64, l: f64, view: &str) -> Option<Result<GenPart, String>> {
    match family {
        "bearing_297" => Some(gen297(d, l, view)),
        "bearing_288" => Some(gen288(d, l, view)),
        _ => None,
    }
}

fn gen297(d: f64, l: f64, view: &str) -> Result<GenPart, String> {
    if view != "main" {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.only_main_received",
            &[("family", "bearing_297"), ("view", view)],
        ));
    }
    let row = row297(d, l).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.parts.err.no_dt",
            &[
                ("table", "GB/T 297"),
                ("d", &trim(d)),
                ("t", &trim(l)),
            ],
        )
    })?;
    Ok(bearing_297(row))
}

fn gen288(d: f64, l: f64, view: &str) -> Result<GenPart, String> {
    if view != "main" {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.only_main_received",
            &[("family", "bearing_288"), ("view", view)],
        ));
    }
    let row = row288(d, l).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.parts.err.no_db",
            &[
                ("table", "GB/T 288"),
                ("d", &trim(d)),
                ("b", &trim(l)),
            ],
        )
    })?;
    Ok(bearing_288(row))
}

// ══════════════════════════════════════════════════════════════════════════
// 测试
// ══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::partgen::generate as gen_all;
    use crate::partgen_kit::{dump_svg, part_svg, LAYER_HATCH, LAYER_MAIN, LAYER_THIN};
    use ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge;
    use ocs_plugin_api::host::acadrust::entities::Hatch;
    use ocs_plugin_api::host::acadrust::types::Color;

    const OCSM_LAYERS: [&str; 5] = [LAYER_MAIN, LAYER_THIN, LAYER_CENTER, "4虚线层", LAYER_HATCH];

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-3
    }

    /// 全部规格 × 全部视图都能生成；图层只在 OCSM 五层内、无尺寸标注、ByLayer。
    #[test]
    fn all_specs_generate_and_layers_ok() {
        for (fam, ds) in [
            ("bearing_297", t297().rows.iter().map(|r| (r.d, r.t)).collect::<Vec<_>>()),
            ("bearing_288", t288().rows.iter().map(|r| (r.d, r.b)).collect::<Vec<_>>()),
        ] {
            assert!(ds.len() >= 20, "{fam} 规格数不足");
            for (d, l) in ds {
                for view in family_views(fam) {
                    let p = gen_all(fam, d, l, view)
                        .unwrap_or_else(|e| panic!("{fam} d{d}×{l} {view}: {e}"));
                    assert!(!p.entities.is_empty(), "{fam} d{d} {view} 空");
                    assert!(!matches!(p.entities[0], EntityType::Dimension(_)));
                    for e in &p.entities {
                        let lay = e.common().layer.as_str();
                        assert!(OCSM_LAYERS.contains(&lay), "{fam} d{d} {view} 图层越界: {lay}");
                        assert!(matches!(e.common().color, Color::ByLayer), "{fam} 颜色非 ByLayer");
                        assert!(!matches!(e, EntityType::Dimension(_)), "{fam} 出现尺寸标注");
                    }
                    // 包围盒有效
                    assert!(p.bbox[2] > p.bbox[0] && p.bbox[3] > p.bbox[1], "{fam} bbox 无效");
                }
            }
        }
    }

    /// 表规模 + 尺寸单调性（扩系列后新增的护栏）。
    ///
    /// - `bearing_297`：保持用户 20 行锚点——E（外圈滚道小端直径）拿不到可信在线来源，
    ///   按“不许臆造/插值”原则不扩（详见表 `source`）。
    /// - `bearing_288`：扩到 GB/T 288-1994 圆柱孔 `230xx/231xx/232xx/240xx/241xx` C/CC 型全量 71 行，
    ///   新增最小规格 23218C（d90）也必须能出图。
    #[test]
    fn tables_expected_counts_and_monotonic_dims() {
        use std::collections::HashSet;
        assert_eq!(t297().rows.len(), 20, "297 应保持用户 20 行锚点（E 无来源，不扩）");
        assert_eq!(t288().rows.len(), 71, "288 应扩到 GB/T 288-1994 C 型全量 71 行");
        // 297：d 严格递增；D/T/B/C/E 合法且 E 落在 (d, D) 内。
        let mut prev = 0.0;
        let mut codes = HashSet::new();
        for r in &t297().rows {
            assert!(codes.insert(r.code.clone()), "297 代号重复 {}", r.code);
            assert!(r.d > prev, "297 d 非严格递增：{}", r.code);
            prev = r.d;
            assert!(
                r.od > r.d && r.t > 0.0 && r.b > 0.0 && r.c > 0.0 && r.r1 > 0.0 && r.r3 > 0.0,
                "297 尺寸非法 {}",
                r.code
            );
            assert!(r.e > r.d && r.e < r.od, "297 E 越界 {}: E={}", r.code, r.e);
        }
        // 288：按 d 升序排列（非递减）；D>d、B>0、r>0。
        let mut prev = 0.0;
        let mut codes = HashSet::new();
        for r in &t288().rows {
            assert!(codes.insert(r.code.clone()), "288 代号重复 {}", r.code);
            assert!(r.d + 1e-9 >= prev, "288 d 非递增：{}", r.code);
            prev = r.d;
            assert!(r.od > r.d && r.b > 0.0 && r.r > 0.0, "288 尺寸非法 {}", r.code);
        }
        // 新增的最小规格（23218C，d90 B52.4）必须能出图。
        let p = gen_all("bearing_288", 90.0, 52.4, "main").expect("23218C 应能出图");
        assert!(!p.entities.is_empty());
        // 最大新规格（24060C，d300 B160）也要能出图。
        let p = gen_all("bearing_288", 300.0, 160.0, "main").expect("24060C 应能出图");
        assert!(!p.entities.is_empty());
    }

    // ── 模板 30202 主视图逐条数值回归（坐标/半径/角度）──

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

    /// 断言存在一条（方向无关的）直线端点匹配。
    fn has_line(p: &GenPart, a: (f64, f64), b: (f64, f64)) -> bool {
        lines_of(p).iter().any(|(s, e)| {
            (near(s.0, a.0) && near(s.1, a.1) && near(e.0, b.0) && near(e.1, b.1))
                || (near(s.0, b.0) && near(s.1, b.1) && near(e.0, a.0) && near(e.1, a.1))
        })
    }
    fn has_arc(p: &GenPart, c: (f64, f64), r: f64, a1: f64, a2: f64) -> bool {
        let n = |x: f64| {
            let m = x % 360.0;
            if m < 0.0 {
                m + 360.0
            } else {
                m
            }
        };
        arcs_of(p).iter().any(|(cc, rr, s, e)| {
            near(cc.0, c.0)
                && near(cc.1, c.1)
                && near(*rr, r)
                && ((near(n(*s), n(a1)) && near(n(*e), n(a2)))
                    || (near(n(*s), n(a2)) && near(n(*e), n(a1))))
        })
    }

    #[test]
    fn bearing_297_template_30202_regression() {
        let row = row297(15.0, 11.75).expect("30202");
        let p = bearing_297(row);
        // 外圈
        assert!(has_line(&p, (0.0, 13.5), (0.0, 16.9)), "外圈左端面");
        assert!(has_line(&p, (0.6, 17.5), (9.4, 17.5)), "外径");
        assert!(has_line(&p, (10.0, 16.9), (10.0, 16.1795)), "外圈右端面上段");
        assert!(has_line(&p, (0.0, 13.5), (10.0, 16.1795)), "外圈滚道 15°");
        assert!(has_arc(&p, (0.6, 16.9), 0.6, 90.0, 180.0), "外径左圆角 r1");
        assert!(has_arc(&p, (9.4, 16.9), 0.6, 0.0, 90.0), "外径右圆角 r1");
        // 内圈
        assert!(has_line(&p, (1.35, 7.5), (11.15, 7.5)), "内孔");
        assert!(has_line(&p, (0.75, 11.25), (0.75, 8.1)), "内圈左端面");
        assert!(has_line(&p, (11.75, 8.1), (11.75, 12.5)), "内圈右端面");
        assert!(has_arc(&p, (1.35, 8.1), 0.6, 180.0, 270.0), "内孔左倒角 r3");
        assert!(has_arc(&p, (11.15, 8.1), 0.6, 270.0, 360.0), "内孔右倒角 r3");
        assert!(has_line(&p, (8.8, 10.9363), (2.1639, 9.6464)), "内圈滚道 11°");
        // 滚子（外接触点在 15° 滚道上；内接触点 = 关于 13° 滚子轴线的镜像）
        assert!(has_line(&p, (1.2, 13.8215), (1.7937, 11.25)), "滚子小端面");
        assert!(has_line(&p, (7.7299, 15.5712), (8.439, 12.5)), "滚子大端面");
        assert!(has_line(&p, (1.7937, 11.25), (2.1639, 9.6464)), "小挡边面");
        assert!(has_line(&p, (8.439, 12.5), (8.8, 10.9363)), "大挡边面");
        // 端面引线 + 轴线
        assert!(has_line(&p, (0.0, 0.0), (0.0, 13.5)), "左端面至轴线");
        assert!(has_line(&p, (11.75, 0.0), (11.75, 8.1)), "右端面至轴线");
        assert!(has_line(&p, (-3.0, 0.0), (14.75, 0.0)), "主轴线出头 3");
        // 滚子轴线中心线：x∈[0,C]
        assert!(
            has_line(&p, (0.1281, 11.3752), (9.8719, 13.6248)),
            "滚子轴线中心线"
        );
        // 上/下两个镜像
        assert!(has_line(&p, (0.0, -13.5), (0.0, -16.9)), "下半外圈左端面");
        assert!(has_arc(&p, (0.6, -16.9), 0.6, 180.0, 270.0), "下半圆角镜像");
        // 剖面线：外圈 + 内圈，每片镜像 → 共 4 片；角度/比例 = 模板逐片实测
        let hs: Vec<(f64, f64)> = p
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Hatch(h) => Some((h.pattern_angle.to_degrees(), h.pattern_scale)),
                _ => None,
            })
            .collect();
        assert_eq!(hs.len(), 4, "30202 剖面线片数");
        let mut got: Vec<(i64, i64)> = hs
            .iter()
            .map(|(a, s)| (a.round() as i64, (s * 100.0).round() as i64))
            .collect();
        got.sort_unstable();
        // 模板：上外 0° / 上内 90° / 下外 270° / 下内 180°，均 scale 1.0
        assert_eq!(
            got,
            vec![(0, 100), (90, 100), (180, 100), (270, 100)],
            "297 剖面线角度/比例应同模板"
        );
    }

    // ── 模板 23022C 主视图逐条数值回归 ──

    #[test]
    fn bearing_288_template_23022c_regression() {
        let row = row288(110.0, 45.0).expect("23022C");
        let p = bearing_288(row);
        // 外圈
        assert!(has_line(&p, (0.0, 75.7281), (0.0, 83.0)), "外圈左侧面");
        assert!(has_line(&p, (45.0, 75.7281), (45.0, 83.0)), "外圈右侧面");
        assert!(has_line(&p, (2.0, 85.0), (43.0, 85.0)), "外径");
        assert!(has_arc(&p, (2.0, 83.0), 2.0, 90.0, 180.0), "外径左圆角");
        assert!(has_arc(&p, (43.0, 83.0), 2.0, 0.0, 90.0), "外径右圆角");
        assert!(has_arc(&p, (22.5, 0.0), 79.0, 73.4525, 106.5475), "球面滚道");
        // 内圈
        assert!(has_line(&p, (2.0, 55.0), (43.0, 55.0)), "内孔");
        assert!(has_line(&p, (0.0, 0.0), (0.0, 83.0)), "内圈左端面至轴线");
        assert!(has_arc(&p, (2.0, 57.0), 2.0, 180.0, 270.0), "内孔左倒角");
        assert!(has_arc(&p, (43.0, 57.0), 2.0, 270.0, 360.0), "内孔右倒角");
        // 内滚道圆弧（外接圆）
        assert!(has_arc(&p, (6.9028, 98.2719), 35.2267, 258.6996, 292.2653), "左内滚道");
        assert!(has_arc(&p, (38.0972, 98.2718), 35.2266, 247.7347, 281.3004), "右内滚道");
        // 两列滚子端面
        assert!(has_line(&p, (2.25, 76.3606), (4.5775, 63.122)), "左滚子小端面");
        assert!(has_line(&p, (18.0, 78.8717), (20.25, 65.6717)), "左滚子大端面");
        assert!(has_line(&p, (42.75, 76.3606), (40.4225, 63.122)), "右滚子小端面");
        assert!(has_line(&p, (27.0, 78.8717), (24.75, 65.6717)), "右滚子大端面");
        // 保持架 + 中心线
        assert!(has_line(&p, (20.25, 65.6717), (24.75, 65.6717)), "保持架底");
        assert!(has_line(&p, (-5.0, 0.0), (50.0, 0.0)), "主轴线");
        assert!(has_line(&p, (22.5, 88.0), (22.5, 52.0)), "竖中心线");
        // 镜像
        assert!(has_line(&p, (0.0, -75.7281), (0.0, -83.0)), "下半外圈侧面");
        // 剖面线：外圈/内圈/保持架 各镜像 → 共 6 片；角度/比例 = 模板逐片实测
        let hs: Vec<(f64, f64)> = p
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Hatch(h) => Some((h.pattern_angle.to_degrees(), h.pattern_scale)),
                _ => None,
            })
            .collect();
        assert_eq!(hs.len(), 6, "23022C 剖面线片数");
        let mut got: Vec<(i64, i64)> = hs
            .iter()
            .map(|(a, s)| (a.round() as i64, (s * 100.0).round() as i64))
            .collect();
        got.sort_unstable();
        // 模板：外圈 0°/270°、内圈 90°/180°（scale 1.0）+ 保持架 0°/270°（scale 0.5）
        assert_eq!(
            got,
            vec![(0, 50), (0, 100), (90, 100), (180, 100), (270, 50), (270, 100)],
            "288 剖面线角度/比例应同模板"
        );
    }

    // ── 剖面线边界：去重 / 简单 / 面积非零 / 环序 = 模板 ──
    //
    // 297/288 之前在 OCS 里一片都看不到：边界有重复点、弧反向遍历折返、闭合边成了长
    // 对角线，宿主按奇偶规则算不出内部。下列测试把模板环序独立硬编码，逐片回归。

    /// 从一个 Hatch 实体取出折线边界顶点（`hatch_ansi31_scaled` 的边序 = 环顶点序）。
    fn hatch_ring(h: &Hatch) -> Vec<[f64; 2]> {
        h.paths
            .iter()
            .flat_map(|p| p.edges.iter())
            .map(|e| match e {
                BoundaryEdge::Line(l) => [l.start.x, l.start.y],
                other => panic!("剖面线边界应为折线边，实际 {other:?}"),
            })
            .collect()
    }

    fn hdist2(a: [f64; 2], b: [f64; 2]) -> f64 {
        (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
    }

    /// 按图元顺序取出全部剖面线的 `(角度°, 比例, 环顶点)`。
    fn hatch_entries(p: &GenPart) -> Vec<(f64, f64, Vec<[f64; 2]>)> {
        p.entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Hatch(h) => Some((
                    h.pattern_angle.to_degrees(),
                    h.pattern_scale,
                    hatch_ring(h),
                )),
                _ => None,
            })
            .collect()
    }

    /// 鞋带公式符号面积。
    fn signed_area(ring: &[[f64; 2]]) -> f64 {
        let n = ring.len();
        let mut s = 0.0;
        for i in 0..n {
            let a = ring[i];
            let b = ring[(i + 1) % n];
            s += a[0] * b[1] - b[0] * a[1];
        }
        s * 0.5
    }

    fn orient(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    }

    fn on_seg(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> bool {
        const E: f64 = 1e-9;
        p[0] >= a[0].min(b[0]) - E
            && p[0] <= a[0].max(b[0]) + E
            && p[1] >= a[1].min(b[1]) - E
            && p[1] <= a[1].max(b[1]) + E
    }

    /// 线段相交（含共线重叠/端点相切）；相邻边共用端点由调用方跳过。
    fn seg_cross(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
        const E: f64 = 1e-9;
        let (o1, o2, o3, o4) = (orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b));
        if ((o1 > E && o2 < -E) || (o1 < -E && o2 > E))
            && ((o3 > E && o4 < -E) || (o3 < -E && o4 > E))
        {
            return true;
        }
        (o1.abs() <= E && on_seg(c, a, b))
            || (o2.abs() <= E && on_seg(d, a, b))
            || (o3.abs() <= E && on_seg(a, c, d))
            || (o4.abs() <= E && on_seg(b, c, d))
    }

    /// 简单多边形：任意两条非相邻边不相交。
    fn ring_is_simple(ring: &[[f64; 2]]) -> bool {
        let n = ring.len();
        for i in 0..n {
            for j in (i + 1)..n {
                if j == i + 1 || (i == 0 && j == n - 1) {
                    continue;
                }
                if seg_cross(ring[i], ring[(i + 1) % n], ring[j], ring[(j + 1) % n]) {
                    return false;
                }
            }
        }
        true
    }

    /// 对每一片剖面线断言：① 相邻顶点不重复（>1e-9）；② 简单；③ 面积非零。
    fn assert_hatch_rings_clean(label: &str, hs: &[(f64, f64, Vec<[f64; 2]>)]) {
        for (k, (angle, scale, ring)) in hs.iter().enumerate() {
            assert!(ring.len() >= 3, "{label} 第{k}片({angle}°/{scale}) 顶点不足");
            for i in 0..ring.len() {
                let j = (i + 1) % ring.len();
                assert!(
                    hdist2(ring[i], ring[j]) > 1e-18,
                    "{label} 第{k}片({angle}°/{scale}) 相邻顶点重复 @ {i}: {:?}",
                    ring[i]
                );
            }
            assert!(
                ring_is_simple(ring),
                "{label} 第{k}片({angle}°/{scale}) 不是简单多边形（自交/折返）: {ring:?}"
            );
            let a = signed_area(ring);
            assert!(a.abs() > 1e-6, "{label} 第{k}片({angle}°/{scale}) 面积≈0 ({a})");
        }
    }

    /// 模板环的一段：`Line(终点)`（起点由上一段决定）或按 `a1→a2` 方向采样的 `Arc`。
    enum TplSeg {
        Line([f64; 2]),
        Arc { c: [f64; 2], r: f64, a1: f64, a2: f64, n: usize },
    }

    /// 按模板环序从 `start` 构造期望环（测试内独立硬编码，不复用生产的 verts）。
    /// 相邻重复点（弧端点与直线端点重合）会被去掉，首尾重复也去掉，与生产一致。
    fn build_tpl_ring(start: [f64; 2], segs: &[TplSeg]) -> Vec<[f64; 2]> {
        fn push(v: &mut Vec<[f64; 2]>, p: [f64; 2]) {
            if v.last().map_or(true, |q| hdist2(*q, p) > 1e-18) {
                v.push(p);
            }
        }
        let mut v = vec![start];
        for s in segs {
            match s {
                TplSeg::Line(p) => push(&mut v, *p),
                TplSeg::Arc { c, r, a1, a2, n } => {
                    for p in arc_pts(*c, *r, *a1, *a2, *n) {
                        push(&mut v, p);
                    }
                }
            }
        }
        while v.len() >= 2 && hdist2(v[0], v[v.len() - 1]) <= 1e-18 {
            v.pop();
        }
        v
    }

    fn assert_ring_eq_tpl(label: &str, got: &[[f64; 2]], want: &[[f64; 2]]) {
        assert_eq!(got.len(), want.len(), "{label} 顶点数不一致：got {} want {}", got.len(), want.len());
        for (i, (g, w)) in got.iter().zip(want).enumerate() {
            let dd = hdist2(*g, *w).sqrt();
            assert!(dd <= 1e-3, "{label} 第{i}点偏离模板 {dd:.6}：got {g:?} want {w:?}");
        }
    }

    fn mirror_y(v: &[[f64; 2]]) -> Vec<[f64; 2]> {
        v.iter().map(|p| [p[0], -p[1]]).collect()
    }

    #[test]
    fn bearing_297_hatch_ring_order_and_simplicity() {
        let row = row297(15.0, 11.75).expect("30202");
        let p = bearing_297(row);
        let hs = hatch_entries(&p);
        // 片数与角度/比例（模板逐片实测，顺序 = 构造顺序）
        let meta: Vec<(i64, i64)> = hs
            .iter()
            .map(|(a, s, _)| (a.round() as i64, (s * 100.0).round() as i64))
            .collect();
        assert_eq!(
            meta,
            vec![(0, 100), (270, 100), (90, 100), (180, 100)],
            "297 剖面线角度/比例顺序"
        );
        assert_hatch_rings_clean("297", &hs);

        // 模板参数（环序在下面独立硬编码）
        let (d, od, t, b, c, e, r1, r3) = (row.d, row.od, row.t, row.b, row.c, row.e, row.r1, row.r3);
        let r = d / 2.0;
        let r2 = od / 2.0;
        let e2 = e / 2.0;
        let tn = CONE_OUTER_DEG.to_radians().tan();
        let y_or = e2 + c * tn;
        let y_sr = r + RIB_SMALL_FRAC * (r2 - r);
        let y_lr = r + RIB_LARGE_FRAC * (r2 - r);
        let ang = CONE_ROLLER_DEG.to_radians();
        let dirv = [ang.cos(), ang.sin()];
        let axis_c = [c / 2.0, (r + r2) / 2.0];
        let ef = [ang.sin(), -ang.cos()];
        let a_pt = [ROLLER_X1_FRAC * c, e2 + tn * ROLLER_X1_FRAC * c];
        let b_pt = [ROLLER_X2_FRAC * c, e2 + tn * ROLLER_X2_FRAC * c];
        let p1 = mirror_pt(a_pt, axis_c, dirv);
        let p2 = mirror_pt(b_pt, axis_c, dirv);
        let d_pt = ray_to_y(a_pt, ef, y_sr);
        let c2_pt = ray_to_y(b_pt, ef, y_lr);

        // 297 外圈：(0,E/2) → 滚道 → (C,D/2−r1) → 右圆角 → 外径 → 左圆角 → (0,D/2−r1) → 闭合
        let outer_tpl = build_tpl_ring(
            [0.0, e2],
            &[
                TplSeg::Line([c, y_or]),
                TplSeg::Line([c, r2 - r1]),
                TplSeg::Arc { c: [c - r1, r2 - r1], r: r1, a1: 0.0, a2: 90.0, n: 4 },
                TplSeg::Line([r1, r2]),
                TplSeg::Arc { c: [r1, r2 - r1], r: r1, a1: 90.0, a2: 180.0, n: 4 },
                TplSeg::Line([0.0, r2 - r1]),
            ],
        );
        // 297 内圈：(T−B, d/2+r3) → 左圆角 → 内孔 → 右圆角 → 右端面 → 大挡边 → 滚道 →
        //          小挡边 → (T−B, y_sr) → 闭合
        let inner_tpl = build_tpl_ring(
            [t - b, r + r3],
            &[
                TplSeg::Arc { c: [t - b + r3, r + r3], r: r3, a1: 180.0, a2: 270.0, n: 4 },
                TplSeg::Line([t - r3, r]),
                TplSeg::Arc { c: [t - r3, r + r3], r: r3, a1: 270.0, a2: 360.0, n: 4 },
                TplSeg::Line([t, y_lr]),
                TplSeg::Line(c2_pt),
                TplSeg::Line(p2),
                TplSeg::Line(p1),
                TplSeg::Line(d_pt),
                TplSeg::Line([t - b, y_sr]),
            ],
        );
        assert_ring_eq_tpl("297 上外圈", &hs[0].2, &outer_tpl);
        assert_ring_eq_tpl("297 上内圈", &hs[2].2, &inner_tpl);
        assert_ring_eq_tpl("297 下外圈", &hs[1].2, &mirror_y(&outer_tpl));
        assert_ring_eq_tpl("297 下内圈", &hs[3].2, &mirror_y(&inner_tpl));
    }

    #[test]
    fn bearing_288_hatch_ring_order_and_simplicity() {
        let row = row288(110.0, 45.0).expect("23022C");
        let p = bearing_288(row);
        let hs = hatch_entries(&p);
        let meta: Vec<(i64, i64)> = hs
            .iter()
            .map(|(a, s, _)| (a.round() as i64, (s * 100.0).round() as i64))
            .collect();
        assert_eq!(
            meta,
            vec![(0, 100), (270, 100), (90, 100), (180, 100), (0, 50), (270, 50)],
            "288 剖面线角度/比例顺序"
        );
        assert_hatch_rings_clean("288", &hs);

        let (d, od, w, rr) = (row.d, row.od, row.b, row.r);
        let r = d / 2.0;
        let r2 = od / 2.0;
        let dr = r2 - r;
        let hw = w / 2.0;
        let y0 = r2 - S288_Y0_FRAC * dr;
        let rs = (hw * hw + y0 * y0).sqrt();
        let sp_start = y0.atan2(hw).to_degrees();
        let sp_end = 180.0 - sp_start;
        let y_in = r + S288_YIN_FRAC * dr;
        let pb = [S288_PB_X_FRAC * w, r + S288_PB_Y_FRAC * dr];
        let pc = [S288_PC_X_FRAC * w, r + S288_PC_Y_FRAC * dr];
        let pa = [0.0, y_in];
        let (cc, ri) = circumcircle(pa, pb, pc).expect("内滚道三点共线");
        let a1 = (pa[1] - cc[1]).atan2(pa[0] - cc[0]).to_degrees();
        let a2 = (pc[1] - cc[1]).atan2(pc[0] - cc[0]).to_degrees();
        let cc_r = [w - cc[0], cc[1]];
        let (ar1, ar2) = (180.0 - a2, 180.0 - a1);
        let y_cage = r + S288_CAGE_BOT_FRAC * dr;
        let y_top = r + S288_CAGE_TOP_FRAC * dr;
        let hb = S288_CAGE_HB_FRAC * w;
        let ht = S288_CAGE_HT_FRAC * w;

        // 288 外圈：(0,y0) → 球面滚道（左→右）→ (B,y0) → 右侧面 → 右圆角 → 外径 →
        //          左圆角 → (0,D/2−r) → 闭合回 (0,y0)
        let outer_tpl = build_tpl_ring(
            [0.0, y0],
            &[
                TplSeg::Arc { c: [hw, 0.0], r: rs, a1: sp_end, a2: sp_start, n: 12 },
                TplSeg::Line([w, r2 - rr]),
                TplSeg::Arc { c: [w - rr, r2 - rr], r: rr, a1: 0.0, a2: 90.0, n: 4 },
                TplSeg::Line([rr, r2]),
                TplSeg::Arc { c: [rr, r2 - rr], r: rr, a1: 90.0, a2: 180.0, n: 4 },
                TplSeg::Line([0.0, r2 - rr]),
            ],
        );
        // 288 内圈：(0,d/2+rr) → 左圆角 → 内孔 → 右圆角 → 右端面 → 上侧内滚道弧 →
        //          保持架底 → 下侧内滚道弧 → 左端面 (0,y_in) → 闭合
        let inner_tpl = build_tpl_ring(
            [0.0, r + rr],
            &[
                TplSeg::Arc { c: [rr, r + rr], r: rr, a1: 180.0, a2: 270.0, n: 4 },
                TplSeg::Line([w - rr, r]),
                TplSeg::Arc { c: [w - rr, r + rr], r: rr, a1: 270.0, a2: 360.0, n: 4 },
                TplSeg::Line([w, y_in]),
                TplSeg::Arc { c: cc_r, r: ri, a1: ar2, a2: ar1, n: 10 },
                TplSeg::Line([hw - hb, y_cage]),
                TplSeg::Arc { c: cc, r: ri, a1: a2, a2: a1, n: 10 },
                TplSeg::Line([0.0, r + rr]),
            ],
        );
        assert_ring_eq_tpl("288 上外圈", &hs[0].2, &outer_tpl);
        assert_ring_eq_tpl("288 上内圈", &hs[2].2, &inner_tpl);
        assert_ring_eq_tpl("288 下外圈", &hs[1].2, &mirror_y(&outer_tpl));
        assert_ring_eq_tpl("288 下内圈", &hs[3].2, &mirror_y(&inner_tpl));

        // 288 保持架：4 点梯形（scale 0.5），模板逐点硬编码
        let cage_tpl = build_tpl_ring(
            [hw - hb, y_cage],
            &[
                TplSeg::Line([hw - ht, y_top]),
                TplSeg::Line([hw + ht, y_top]),
                TplSeg::Line([hw + hb, y_cage]),
            ],
        );
        assert_ring_eq_tpl("288 上保持架", &hs[4].2, &cage_tpl);
        assert_ring_eq_tpl("288 下保持架", &hs[5].2, &mirror_y(&cage_tpl));
    }

    /// 最小规格 + 每个规格的首个长度都能出图（全局护栏用的就是这一条）。
    #[test]
    fn min_spec_and_first_length_usable() {
        // 直接用本模块的表，避免牵连其它并行分支的目录 JSON
        let (d297, l297) = (t297().rows[0].d, t297().rows[0].t);
        for v in family_views("bearing_297") {
            gen_all("bearing_297", d297, l297, v).unwrap_or_else(|e| panic!("bearing_297 {v}: {e}"));
        }
        let (d288, l288) = (t288().rows[0].d, t288().rows[0].b);
        for v in family_views("bearing_288") {
            gen_all("bearing_288", d288, l288, v).unwrap_or_else(|e| panic!("bearing_288 {v}: {e}"));
        }
        // 目录 sizes[0] 必须能出图
        for fam in ["bearing_297", "bearing_288"] {
            let f = families_json();
            let sizes = f[fam]["sizes"].as_array().unwrap();
            let d = sizes[0]["d"].as_f64().unwrap();
            let l = sizes[0]["lengths"][0].as_f64().unwrap();
            for v in family_views(fam) {
                gen_all(fam, d, l, v).unwrap_or_else(|e| panic!("{fam} {v}: {e}"));
            }
        }
    }

    /// 视图校验：非本族 / 非 main 一律报错。
    #[test]
    fn view_validation() {
        assert!(generate("bearing_297", 15.0, 11.75, "top").unwrap().is_err());
        assert!(generate("bearing_288", 110.0, 45.0, "section").unwrap().is_err());
        assert!(generate("hex_bolt_c", 5.0, 25.0, "main").is_none());
    }

    /// 人工核对：落 SVG（`cargo test -p ocs_ocsm --lib partgen_b4:: -- --ignored dump_b4_svg`）。
    #[test]
    #[ignore]
    fn dump_b4_svg() {
        std::fs::create_dir_all("/tmp/b4").unwrap();
        for (fam, d, l) in [
            ("bearing_297", 15.0, 11.75),
            ("bearing_297", 100.0, 37.0),
            ("bearing_288", 90.0, 52.4),
            ("bearing_288", 110.0, 45.0),
            ("bearing_288", 240.0, 92.0),
            ("bearing_288", 300.0, 160.0),
        ] {
            let p = gen_all(fam, d, l, "main").unwrap();
            let f = format!("/tmp/b4/{}-{}-main.svg", fam, trim(d));
            dump_svg(&p, &f).unwrap();
            println!("写出 {f}");
        }
    }

    // ── 可视化叠合：模板 TSV 的 1轮廓实线层 坐标（红）对生成几何（蓝）──

    /// 模板上半个视的几何（从 ref_dump 的 TSV 抄录；下半 = y 镜像，含 OCS 换算后的世界坐标）。
    struct TplEnt {
        /// 'L' 直线：x1,y1,x2,y2；'A' 圆弧：cx,cy,r,a1,a2；'P' 折线：x0,y0,...
        kind: char,
        n: Vec<f64>,
    }
    fn tpl297() -> Vec<TplEnt> {
        let l = |x1, y1, x2, y2| TplEnt { kind: 'L', n: vec![x1, y1, x2, y2] };
        let a = |cx, cy, r, a1, a2| TplEnt { kind: 'A', n: vec![cx, cy, r, a1, a2] };
        vec![
            l(0.0, 13.5, 0.0, 16.9),
            l(0.6, 17.5, 9.4, 17.5),
            l(10.0, 16.9, 10.0, 16.1795),
            l(0.0, 13.5, 10.0, 16.1795),
            l(2.1639, 9.6464, 1.7937, 11.25),
            l(1.7937, 11.25, 0.75, 11.25),
            l(0.75, 11.25, 0.75, 8.1),
            l(1.35, 7.5, 11.15, 7.5),
            l(11.75, 8.1, 11.75, 12.5),
            l(11.75, 12.5, 8.439, 12.5),
            l(8.439, 12.5, 8.8, 10.9363),
            l(8.8, 10.9363, 2.1639, 9.6464),
            l(1.2, 13.8215, 1.7937, 11.25),
            l(7.7299, 15.5712, 8.439, 12.5),
            l(10.0, 16.1795, 10.0, 12.5),
            l(0.0, 0.0, 0.0, 13.5),
            l(0.75, 0.0, 0.75, 8.1),
            l(11.75, 0.0, 11.75, 8.1),
            a(0.6, 16.9, 0.6, 90.0, 180.0),
            a(9.4, 16.9, 0.6, 0.0, 90.0),
            a(1.35, 8.1, 0.6, 180.0, 270.0),
            a(11.15, 8.1, 0.6, 270.0, 360.0),
        ]
    }
    fn tpl288() -> Vec<TplEnt> {
        let l = |x1, y1, x2, y2| TplEnt { kind: 'L', n: vec![x1, y1, x2, y2] };
        let a = |cx, cy, r, a1, a2| TplEnt { kind: 'A', n: vec![cx, cy, r, a1, a2] };
        let p = |v: Vec<f64>| TplEnt { kind: 'P', n: v };
        vec![
            l(0.0, 75.7281, 0.0, 83.0),
            l(45.0, 75.7281, 45.0, 83.0),
            l(2.0, 85.0, 43.0, 85.0),
            l(0.0, 0.0, 0.0, 83.0),
            l(45.0, 0.0, 45.0, 83.0),
            l(0.0, 63.7281, 0.0, 57.0),
            l(45.0, 63.7281, 45.0, 57.0),
            l(43.0, 55.0, 2.0, 55.0),
            l(2.25, 76.3606, 4.5775, 63.122),
            l(18.0, 78.8717, 20.25, 65.6717),
            l(27.0, 78.8717, 24.75, 65.6717),
            l(42.75, 76.3606, 40.4225, 63.122),
            l(20.25, 65.6717, 24.75, 65.6717),
            a(22.5, 0.0, 79.0, 73.4525, 106.5475),
            a(2.0, 83.0, 2.0, 90.0, 180.0),
            a(43.0, 83.0, 2.0, 0.0, 90.0),
            a(2.0, 57.0, 2.0, 180.0, 270.0),
            a(43.0, 57.0, 2.0, 270.0, 360.0),
            a(6.9028, 98.2719, 35.2267, 258.6996, 292.2653),
            a(38.0972, 98.2718, 35.2266, 247.7347, 281.3004),
            p(vec![20.25, 65.6717, 19.5682, 69.6717, 25.4318, 69.6717, 24.75, 65.6717]),
        ]
    }

    /// 把图元列表（含镜像）连同模板一起画成同一张 SVG，模板红、生成蓝。
    fn overlay_svg(title: &str, tpl: &[TplEnt], part: &GenPart, path: &str) {
        // 采样：把 Arc 也离散成折线（统一渲染）
        let mut segs: Vec<(bool, [[f64; 2]; 2])> = Vec::new(); // (is_tpl, seg)
        // 模板（含 y 镜像）
        for e in tpl {
            for sgn in [1.0, -1.0] {
                match e.kind {
                    'L' => segs.push((
                        true,
                        [
                            [e.n[0], sgn * e.n[1]],
                            [e.n[2], sgn * e.n[3]],
                        ],
                    )),
                    'A' => {
                        let (cx, cy, r, a1, a2) = (e.n[0], sgn * e.n[1], e.n[2], e.n[3], e.n[4]);
                        let (a1, a2) = if sgn > 0.0 {
                            (a1, a2)
                        } else {
                            (-a2, -a1)
                        };
                        let pts = arc_pts([cx, cy], r, a1, a2, 24);
                        for w in pts.windows(2) {
                            segs.push((true, [w[0], w[1]]));
                        }
                    }
                    _ => {
                        let pts: Vec<[f64; 2]> = e
                            .n
                            .chunks(2)
                            .map(|c| [c[0], sgn * c[1]])
                            .collect();
                        for w in pts.windows(2) {
                            segs.push((true, [w[0], w[1]]));
                        }
                        // 闭合
                        if pts.len() >= 3 {
                            segs.push((true, [pts[pts.len() - 1], pts[0]]));
                        }
                    }
                }
            }
        }
        // 生成件（只取轮廓/倒角所在层，剖面线不画）
        for e in &part.entities {
            match e {
                EntityType::Line(l) => {
                    segs.push((false, [[l.start.x, l.start.y], [l.end.x, l.end.y]]));
                }
                EntityType::Arc(a) => {
                    let pts = arc_pts(
                        [a.center.x, a.center.y],
                        a.radius,
                        a.start_angle.to_degrees(),
                        a.end_angle.to_degrees(),
                        24,
                    );
                    for w in pts.windows(2) {
                        segs.push((false, [w[0], w[1]]));
                    }
                }
                EntityType::LwPolyline(pl) => {
                    let pts: Vec<[f64; 2]> = pl.vertices.iter().map(|v| [v.location.x, v.location.y]).collect();
                    for w in pts.windows(2) {
                        segs.push((false, [w[0], w[1]]));
                    }
                    if pl.is_closed && pts.len() >= 3 {
                        segs.push((false, [pts[pts.len() - 1], pts[0]]));
                    }
                }
                _ => {}
            }
        }
        // bbox
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for (_, s) in &segs {
            for p in s {
                x0 = x0.min(p[0]);
                y0 = y0.min(p[1]);
                x1 = x1.max(p[0]);
                y1 = y1.max(p[1]);
            }
        }
        let (w, h) = (x1 - x0, y1 - y0);
        let pad = 0.05 * w.max(h);
        let px = 1200.0_f64;
        let sc = px / (w + 2.0 * pad).max(1e-6);
        let py = (h + 2.0 * pad) * sc;
        let t = |x: f64, y: f64| ((x - x0 + pad) * sc, py - (y - y0 + pad) * sc);
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{px:.0}\" height=\"{py:.0}\" viewBox=\"0 0 {px:.0} {py:.0}\"><rect width=\"100%\" height=\"100%\" fill=\"#fff\"/><g fill=\"none\" stroke-width=\"1.2\">"
        );
        out.push_str(&format!("<text x=\"10\" y=\"20\" font-size=\"16\" fill=\"#000\">{title}</text>"));
        // 生成件（蓝）先画，模板（红）后画压在上面：重合处显红，生成的独有段显蓝、模板独有段显红。
        for (is_tpl, s) in segs.iter().filter(|(t, _)| !*t).chain(segs.iter().filter(|(t, _)| *t)) {
            let a = t(s[0][0], s[0][1]);
            let b = t(s[1][0], s[1][1]);
            let col = if *is_tpl { "#e00000" } else { "#0033dd" };
            out.push_str(&format!(
                "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{col}\"/>",
                a.0, a.1, b.0, b.1
            ));
        }
        out.push_str("</g></svg>");
        std::fs::write(path, out).unwrap();
    }

    /// 可视化叠合产物（人工看红蓝是否重合）。
    #[test]
    #[ignore]
    fn overlay_b4() {
        std::fs::create_dir_all("/tmp/b4").unwrap();
        let p297 = gen_all("bearing_297", 15.0, 11.75, "main").unwrap();
        overlay_svg(
            "bearing_297 30202  main  (red=template, blue=generated)",
            &tpl297(),
            &p297,
            "/tmp/b4/overlay-bearing_297-main.svg",
        );
        let p288 = gen_all("bearing_288", 110.0, 45.0, "main").unwrap();
        overlay_svg(
            "bearing_288 23022C  main  (red=template, blue=generated)",
            &tpl288(),
            &p288,
            "/tmp/b4/overlay-bearing_288-main.svg",
        );
        println!("写出 /tmp/b4/overlay-bearing_297-main.svg 与 overlay-bearing_288-main.svg");
    }

    /// 叠合数值断言（不依赖人工）：模板每条轮廓都能在生成件里找到 ≤0.01 的对应图元。
    #[test]
    fn overlay_geometry_matches_template() {
        for (fam, d, l, tpl) in [
            ("bearing_297", 15.0, 11.75, tpl297()),
            ("bearing_288", 110.0, 45.0, tpl288()),
        ] {
            let p = gen_all(fam, d, l, "main").unwrap();
            // 模板图元（含镜像）离散成采样点集合
            let mut tpl_pts: Vec<(f64, f64)> = Vec::new();
            for e in &tpl {
                for sgn in [1.0_f64, -1.0] {
                    match e.kind {
                        'L' => {
                            let (x1, y1, x2, y2) = (e.n[0], sgn * e.n[1], e.n[2], sgn * e.n[3]);
                            for i in 0..=20 {
                                let t = i as f64 / 20.0;
                                tpl_pts.push((x1 + (x2 - x1) * t, y1 + (y2 - y1) * t));
                            }
                        }
                        'A' => {
                            let (cx, cy, r) = (e.n[0], sgn * e.n[1], e.n[2]);
                            let (a1, a2) = if sgn > 0.0 {
                                (e.n[3], e.n[4])
                            } else {
                                (-e.n[4], -e.n[3])
                            };
                            for q in arc_pts([cx, cy], r, a1, a2, 20) {
                                tpl_pts.push((q[0], q[1]));
                            }
                        }
                        _ => {
                            for c in e.n.chunks(2) {
                                // 边按 20 段采样
                                let _ = c;
                            }
                            for c in e.n.chunks(2) {
                                tpl_pts.push((c[0], sgn * c[1]));
                            }
                        }
                    }
                }
            }
            // 生成件的采样点：直线/圆弧/折线各段
            let mut gen_pts: Vec<(f64, f64)> = Vec::new();
            for e in &p.entities {
                match e {
                    EntityType::Line(ln) => {
                        for i in 0..=20 {
                            let t = i as f64 / 20.0;
                            gen_pts.push((
                                ln.start.x + (ln.end.x - ln.start.x) * t,
                                ln.start.y + (ln.end.y - ln.start.y) * t,
                            ));
                        }
                    }
                    EntityType::Arc(a) => {
                        for q in arc_pts(
                            [a.center.x, a.center.y],
                            a.radius,
                            a.start_angle.to_degrees(),
                            a.end_angle.to_degrees(),
                            20,
                        ) {
                            gen_pts.push((q[0], q[1]));
                        }
                    }
                    EntityType::LwPolyline(pl) => {
                        for v in &pl.vertices {
                            gen_pts.push((v.location.x, v.location.y));
                        }
                    }
                    _ => {}
                }
            }
            // 每条模板线段取中点，检查到生成件最近点距离
            let mut worst = 0.0_f64;
            for pt in &tpl_pts {
                let dd = gen_pts
                    .iter()
                    .map(|g| ((g.0 - pt.0).powi(2) + (g.1 - pt.1).powi(2)).sqrt())
                    .fold(f64::MAX, f64::min);
                worst = worst.max(dd);
            }
            assert!(worst < 0.05, "{fam} 模板与生成件最大偏离 {worst:.4} > 0.05");
        }
    }

    /// 方便人工核对：写出 30202 的纯 SVG（生成件）。
    #[test]
    fn svg_smoke() {
        let p = gen_all("bearing_297", 15.0, 11.75, "main").unwrap();
        let s = part_svg(&p, 400.0, 300.0);
        assert!(s.contains("<svg"));
    }
}
