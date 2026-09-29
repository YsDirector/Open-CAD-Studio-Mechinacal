//! 第四批 B 组（partgen_b2）：**内六角平端紧定螺钉 GB/T 77-2007**、**圆螺母 GB/T 812-1988**、
//! **圆螺母用止动垫圈 GB/T 858-1988**。
//!
//! 本文件只放这三个族的画法与数据；共用图元/表/校验工具在 `partgen_kit.rs`。
//! **不要改其它模块**——并行开发时会互相踩。
//!
//! | 族 id | 名称 | 模板（用户参数化图 DXF） | 视图 | 数据 |
//! |---|---|---|---|---|
//! | `set_screw_77` | 内六角平端紧定螺钉 | `~/桌面/GB/参数化/内六角平端紧定螺钉_GB-T77-2007/`（主/左） | main/end | 易紧通 info_26503 |
//! | `round_nut_812` | 圆螺母 | `~/桌面/GB/参数化/圆螺母_GB-T812-1988/`（主/俯/剖） | main/top/section | 易紧通 info_13770 |
//! | `lock_washer_858` | 圆螺母用止动垫圈 | `.../圆螺母用止动垫圈(d≤100)_…` 与 `(d≥100)_…` | main/section | 易紧通 info_3349 |
//!
//! 画法 100% 由用户模板 DXF 逐条反解（`ref_dump` → TSV，圆弧角度是度）。各族要点见各函数注释。
//! 模板规格（回归基准）：
//! - 77：M10×25（d=10 dp=7 e=5.723 s=5 t=4 短螺钉值）
//! - 812：M22（表 dk=38→模板 D、表 d1=30→模板 dk、m=10 n=5.3 t=3.1 c=1 c1=0.5）
//! - 858：d≤100 取规格 25（d=25.5 D1=34 dc=45 b=4.8 a=22 s=1 h=4）；
//!        d≥100 取规格 160（d=161 D1=190 dc=216 b=15.5 a=156 s=2 h=8）
//!
//! 说明：模板 DXF 的剖视图带 `5剖面线层` ANSI31 剖面线（ezdxf 复核：`圆螺母`/`止动垫圈`
//! 剖视图各 2 片，`round_nut` angle 0/270°·scale 0.25，`lock_washer` angle 0°·scale 1.0），
//! 本模块按模板逐片补出（边界 = 模板实测的被切材料轮廓）。

use ocs_plugin_api::host::acadrust::entities::EntityType;

use crate::partgen::{GenPart, PartMeta};
use crate::partgen_kit::{
    arc, check_length, circle, hatch_ansi31_scaled, line, polyline, trim, weight_kg, Table,
    LAYER_CENTER, LAYER_HIDDEN, LAYER_MAIN, LAYER_THIN,
};

/// 常量：sin/cos/tan 常用角。
const TAN30: f64 = 0.577_350_269_189_625_7;
const TAN60: f64 = 1.732_050_807_568_877_2;

// ══════════════════════════════════════════════════════════════════════════
// 数据表
// ══════════════════════════════════════════════════════════════════════════

/// GB/T 77 内六角平端紧定螺钉一行。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SetScrewRow {
    pub d: f64,
    #[serde(rename = "P")]
    pub pitch: f64,
    /// 端部直径（表内最大值，模板实测取最大值）
    pub dp: f64,
    /// 内六角对边宽 e（表内最小值）
    pub e: f64,
    /// 内六角对边距 s（公称）
    pub s: f64,
    /// 内六角孔深（短螺钉）
    pub t_short: f64,
    /// 内六角孔深（长螺钉）
    #[allow(dead_code)]
    pub t_long: f64,
    /// L0 公称（表注②）
    #[serde(rename = "L0")]
    #[allow(dead_code)]
    pub l0: f64,
    pub l_min: f64,
    pub l_max: f64,
    pub lengths: Vec<f64>,
}

/// GB/T 812 圆螺母一行。**注意命名互换**：模板 `D` = 本表 `dk`（外径），模板 `dk` = 本表 `d1`。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RoundNutRow {
    /// 螺纹公称直径 D（选择器用的规格）
    pub d: f64,
    /// 外径（模板里的 `D`）
    pub dk: f64,
    /// 端面凹槽/凸台直径（模板里的 `dk`）
    pub d1: f64,
    pub m: f64,
    /// 槽宽（取最大值 = 模板公称）
    pub n: f64,
    /// 槽深（取最大值 = 模板公称）
    pub t: f64,
    pub c: f64,
    pub c1: f64,
    #[serde(default = "default_slots")]
    pub slots: u32,
}

fn default_slots() -> u32 {
    4
}

/// GB/T 858 止动垫圈一行。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LockWasherRow {
    /// 公称规格（10…200，选择器用）
    pub d: f64,
    /// 内孔实际直径（模板 `d`）
    pub d_hole: f64,
    /// 外舌（外径）直径（模板 `D` ← 表 `d c`）
    pub dc: f64,
    /// 垫圈外径（模板 `D1`）
    #[serde(rename = "D1")]
    pub d1: f64,
    /// 厚度
    pub s: f64,
    /// 舌高
    pub h: f64,
    /// 舌宽
    pub b: f64,
    /// 内舌长度基准
    pub a: f64,
}

fn set_screw_table() -> &'static Table<SetScrewRow> {
    static T: std::sync::OnceLock<Table<SetScrewRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        Table::parse(include_str!("tables/partsSetScrew77.json"))
    })
}

fn round_nut_table() -> &'static Table<RoundNutRow> {
    static T: std::sync::OnceLock<Table<RoundNutRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| Table::parse(include_str!("tables/partsRoundNut812.json")))
}

fn lock_washer_table(large: bool) -> &'static Table<LockWasherRow> {
    static S: std::sync::OnceLock<Table<LockWasherRow>> = std::sync::OnceLock::new();
    static L: std::sync::OnceLock<Table<LockWasherRow>> = std::sync::OnceLock::new();
    if large {
        L.get_or_init(|| Table::parse(include_str!("tables/partsLockWasher858l.json")))
    } else {
        S.get_or_init(|| Table::parse(include_str!("tables/partsLockWasher858s.json")))
    }
}

/// 两段模板分界：d≤100 与 d≥100（模板外舌角向不同）。
fn lock_washer_large(d: f64) -> bool {
    d > 100.0 + 1e-9
}

/// 两段合并的规格序列（families_json 用）。
fn lock_washer_all_rows() -> Vec<&'static LockWasherRow> {
    let mut v: Vec<&'static LockWasherRow> = lock_washer_table(false).rows.iter().collect();
    v.extend(lock_washer_table(true).rows.iter());
    v.sort_by(|a, b| a.d.partial_cmp(&b.d).unwrap());
    v
}

// ══════════════════════════════════════════════════════════════════════════
// GB/T 77 内六角平端紧定螺钉
//
// 模板（M10×25）逐条实测：
// - 基点 = 内六角端端面 × 轴线；零件占 x∈[0,l]，x=0 为内六角孔端，x=l 为平端。
// - 端部倒角：端面半径 = 螺纹小径 dm/2 = 0.85d/2，45° 倒到 d/2，轴向长 c=(d−dm)/2=0.075d
//   （M10：端面 r=4.25、c=0.75 ✓；ACM GB77-85 M3 亦为「倒到螺纹小径」的同一规则）。
// - 杆身 d 至 x=l−(d−dp)/2，再 45° 收锥到平端 dp（M10：x=23.5→25，r=5→3.5 ✓）。
// - 螺纹牙底细实线 r=dm/2，自 x=0 画到与平端锥面相交处 x=l−(d−dp)/2+(d−dm)/2（M10：24.25 ✓）。
// - 内六角孔（4虚线层）：外轮廓母线 y=±e/2、内母线 y=±(e/2−(s/2)tan30)（M10：±2.8615 / ±1.4181 ✓），
//   孔底平面竖线 x=t，120° 锥尖 x=t+(e/2)/tan60（M10：4 / 5.6521 ✓）。
//   *模板把孔底竖线画在 3.6（比标准 t=4 浅 0.4），但锥尖仍按 t=4 计算；本实现统一按标准 t 取 x=t，见报告。*
// - 左视图：外圆 d/2、内六角口外圆 e/2、牙底 3/4 细弧 dm/2、六角口（上下顶点 ±e/2、左右 (±s/2,±(e/2−(s/2)tan30))）。
// ══════════════════════════════════════════════════════════════════════════

fn set_screw_row(d: f64) -> Option<&'static SetScrewRow> {
    set_screw_table().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

/// 内六角孔深：模板对短/长螺钉两列，紧定螺钉按短螺钉列画（见文件头说明）。
fn set_screw_t(row: &SetScrewRow) -> f64 {
    row.t_short
}

/// GB/T 77 主视图。
fn set_screw_main(row: &SetScrewRow, l: f64) -> GenPart {
    let (d, dp, e, s) = (row.d, row.dp, row.e, row.s);
    let dm = 0.85 * d;
    let c = (d - dm) / 2.0;
    let t = set_screw_t(row);
    let mut en: Vec<EntityType> = Vec::new();
    // 外形
    en.push(line([0.0, dm / 2.0], [0.0, -dm / 2.0], LAYER_MAIN));
    for sgn in [1.0, -1.0] {
        en.push(line([0.0, sgn * dm / 2.0], [c, sgn * d / 2.0], LAYER_MAIN));
        en.push(line(
            [c, sgn * d / 2.0],
            [l - (d - dp) / 2.0, sgn * d / 2.0],
            LAYER_MAIN,
        ));
        en.push(line(
            [l - (d - dp) / 2.0, sgn * d / 2.0],
            [l, sgn * dp / 2.0],
            LAYER_MAIN,
        ));
    }
    en.push(line([l, dp / 2.0], [l, -dp / 2.0], LAYER_MAIN));
    // 倒角/杆身 与 杆身/平端锥 的相贯圆在侧视中投影成竖线（模板 & ACM GB77-85 均有）
    en.push(line([c, d / 2.0], [c, -d / 2.0], LAYER_MAIN));
    en.push(line(
        [l - (d - dp) / 2.0, d / 2.0],
        [l - (d - dp) / 2.0, -d / 2.0],
        LAYER_MAIN,
    ));
    // 螺纹牙底细实线：止于平端锥面
    let x_thread_end = l - (d - dp) / 2.0 + (d - dm) / 2.0;
    for sgn in [1.0, -1.0] {
        en.push(line([0.0, sgn * dm / 2.0], [x_thread_end, sgn * dm / 2.0], LAYER_THIN));
    }
    // 内六角孔（虚线）
    let y_out = e / 2.0;
    let y_in = e / 2.0 - (s / 2.0) * TAN30;
    let apex = t + (e / 2.0) / TAN60;
    for sgn in [1.0, -1.0] {
        en.push(line([0.0, sgn * y_out], [t, sgn * y_out], LAYER_HIDDEN));
        en.push(line([0.0, sgn * y_in], [t, sgn * y_in], LAYER_HIDDEN));
        en.push(line([t, sgn * y_out], [apex, 0.0], LAYER_HIDDEN));
    }
    en.push(line([t, y_out], [t, -y_out], LAYER_HIDDEN));
    // 中心线
    en.push(line([-2.0, 0.0], [l + 2.0, 0.0], LAYER_CENTER));
    let half = d / 2.0;
    let vol = std::f64::consts::PI / 4.0 * d * d * l
        - std::f64::consts::PI / 4.0 * e * e * t * 0.7;
    GenPart {
        entities: en,
        meta: PartMeta {
            code: "GB/T 77-2007".into(),
            name: crate::i18n::t_data("内六角平端紧定螺钉").into(),
            spec: format!("M{}×{}", trim(d), trim(l)),
            material: String::new(),
            weight: format!("{:.5}", weight_kg(vol, 7.85)),
        },
        bbox: [0.0, -half, l, half],
    }
}

/// GB/T 77 左视图（端视图）。
fn set_screw_end(row: &SetScrewRow, l: f64) -> GenPart {
    let (d, e, s) = (row.d, row.e, row.s);
    let dm = 0.85 * d;
    let y_in = e / 2.0 - (s / 2.0) * TAN30;
    let mut en: Vec<EntityType> = Vec::new();
    en.push(circle([0.0, 0.0], d / 2.0, LAYER_MAIN));
    en.push(circle([0.0, 0.0], e / 2.0, LAYER_MAIN));
    // 牙底 3/4 细弧（模板 0°→270°）
    en.push(arc([0.0, 0.0], dm / 2.0, 0.0, 270.0, LAYER_THIN));
    // 六角口
    let v = [
        [0.0, e / 2.0],
        [s / 2.0, y_in],
        [s / 2.0, -y_in],
        [0.0, -e / 2.0],
        [-s / 2.0, -y_in],
        [-s / 2.0, y_in],
    ];
    for i in 0..6 {
        en.push(line(v[i], v[(i + 1) % 6], LAYER_MAIN));
    }
    let o = d / 2.0 + 2.0;
    en.push(line([-o, 0.0], [o, 0.0], LAYER_CENTER));
    en.push(line([0.0, o], [0.0, -o], LAYER_CENTER));
    let mut p = GenPart {
        entities: en,
        meta: PartMeta {
            code: "GB/T 77-2007".into(),
            name: crate::i18n::t_data("内六角平端紧定螺钉").into(),
            spec: format!("M{}×{}", trim(d), trim(l)),
            material: String::new(),
            weight: String::new(),
        },
        bbox: [-d / 2.0, -d / 2.0, d / 2.0, d / 2.0],
    };
    p.meta.weight = set_screw_main(row, l).meta.weight;
    p
}

// ══════════════════════════════════════════════════════════════════════════
// GB/T 812 圆螺母
//
// 模板（M22：表 dk=38、d1=30）逐条实测：
// - **命名互换**：模板 `D`=表 dk（外径，r=19）、模板 `dk`=表 d1（端面凸台/凹槽，r=15）。
// - 基点 = 端面（d1 凸台面）× 轴线。
// - 主视图（端视图）：内孔小径实线 r=0.45D（模板 9.9，即 0.9D/2）、大径 3/4 细弧 r=D/2、
//   d1 凸台圆 r=dk/2；外径 r=D/2 上开 4 槽：槽宽 n（半角 asin((n/2)/(D/2))）、
//   槽底平面距轴心 x_slot=sqrt((D/2)²−(n/2)²)−t（模板 15.7143）。
// - 俯视图（轴向侧视，轴线竖直）：端面 r=dk/2 → 60° 锥到 r=D/2（轴向长 L=(D/2−dk/2)/tan60=2.3094）
//   → r=D/2 到 y=m−c → c 倒角 → 端面 r=D/2−c；槽为 x=±n/2 全长直线 + 两条倒角边界。
// - 剖视图（轴向剖，轴线水平）：同外形 + 槽底母线 r_slot + 内孔（小径 r=0.45D 实线、大径 D/2 细线、
//   两端 60° 倒角 c1/cR）。
// ══════════════════════════════════════════════════════════════════════════

fn round_nut_row(d: f64) -> Option<&'static RoundNutRow> {
    round_nut_table().rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

/// 圆螺母几何量（模板命名：D=表 dk，dk=表 d1）。
struct Nut812 {
    d_thread: f64,
    rd: f64,     // 模板 D/2 = 表 dk/2（外径半径）
    rdk: f64,    // 模板 dk/2 = 表 d1/2（端面凸台半径）
    m: f64,
    n: f64,
    #[allow(dead_code)]
    t: f64,
    c: f64,
    c1: f64,
    r_minor: f64, // 内孔小径（模板 0.9D/2）
    l_ch: f64,    // d1 锥轴向长
    r_slot: f64,  // 槽底母线半径
    x_slot: f64,  // 槽底平面到轴心距离
    slots: u32,
}

fn nut812_geom(row: &RoundNutRow) -> Nut812 {
    let rd = row.dk / 2.0;
    let rdk = row.d1 / 2.0;
    let r_minor = 0.45 * row.d;
    let l_ch = (rd - rdk) / TAN60;
    let r_slot = ((rd * rd) - (row.n / 2.0).powi(2)).sqrt() - row.t;
    let x_slot = if (rd - rdk).abs() < 1e-12 {
        0.0
    } else {
        l_ch * (r_slot - rdk) / (rd - rdk)
    };
    Nut812 {
        d_thread: row.d,
        rd,
        rdk,
        m: row.m,
        n: row.n,
        t: row.t,
        c: row.c,
        c1: row.c1,
        r_minor,
        l_ch,
        r_slot,
        x_slot,
        slots: row.slots.max(1),
    }
}

fn nut812_meta(g: &Nut812) -> PartMeta {
    let vol = std::f64::consts::PI
        * (g.rd * g.rd - g.rdk * g.rdk) * g.m
        - std::f64::consts::PI / 4.0 * g.d_thread * g.d_thread * g.m;
    PartMeta {
        code: "GB/T 812-1988".into(),
        name: crate::i18n::t_data("圆螺母").into(),
        spec: format!("M{}", trim(g.d_thread)),
        material: String::new(),
        weight: format!("{:.4}", weight_kg(vol.max(0.0), 7.85)),
    }
}

fn round_nut_main(g: &Nut812) -> GenPart {
    let mut en: Vec<EntityType> = Vec::new();
    en.push(circle([0.0, 0.0], g.rdk, LAYER_MAIN));
    en.push(circle([0.0, 0.0], g.r_minor, LAYER_MAIN));
    // 大径 3/4 细弧（模板 270°→180°），大径 = 螺纹公称 D（模板 11）
    en.push(arc([0.0, 0.0], g.d_thread / 2.0, 270.0, 180.0, LAYER_THIN));
    // 4 个槽 + 外圆圆弧
    let alpha = ((g.n / 2.0) / g.rd).asin();
    let x_out = ((g.rd * g.rd) - (g.n / 2.0).powi(2)).sqrt();
    for k in 0..g.slots {
        let th = k as f64 * 90.0;
        // 外圆圆弧（槽之间）
        en.push(arc(
            [0.0, 0.0],
            g.rd,
            th + alpha.to_degrees(),
            th + 90.0 - alpha.to_degrees(),
            LAYER_MAIN,
        ));
        // 槽：两条侧壁 + 底
        let (c0, s0) = (th.to_radians().cos(), th.to_radians().sin());
        // 垂直方向
        let (px, py) = (-s0, c0);
        let wall_a = [x_out * c0 + (g.n / 2.0) * px, x_out * s0 + (g.n / 2.0) * py];
        let wall_b = [g.r_slot * c0 + (g.n / 2.0) * px, g.r_slot * s0 + (g.n / 2.0) * py];
        let wall_a2 = [x_out * c0 - (g.n / 2.0) * px, x_out * s0 - (g.n / 2.0) * py];
        let wall_b2 = [g.r_slot * c0 - (g.n / 2.0) * px, g.r_slot * s0 - (g.n / 2.0) * py];
        en.push(line(wall_a, wall_b, LAYER_MAIN));
        en.push(line(wall_a2, wall_b2, LAYER_MAIN));
        en.push(line(wall_b, wall_b2, LAYER_MAIN));
    }
    let o = g.rd + 3.0;
    en.push(line([-o, 0.0], [o, 0.0], LAYER_CENTER));
    en.push(line([0.0, o], [0.0, -o], LAYER_CENTER));
    GenPart {
        entities: en,
        meta: nut812_meta(g),
        bbox: [-g.rd, -g.rd, g.rd, g.rd],
    }
}

fn round_nut_top(g: &Nut812) -> GenPart {
    let mut en: Vec<EntityType> = Vec::new();
    let (rd, rdk, m, n, c, lch) = (g.rd, g.rdk, g.m, g.n, g.c, g.l_ch);
    // 外轮廓（轴线竖直 y，端面 d1 在 y=0，dk 端在 y=m）
    let prof = [
        [-rdk, 0.0],
        [rdk, 0.0],
        [rd, lch],
        [rd, m - c],
        [rd - c, m],
        [-rd + c, m],
        [-rd, m - c],
        [-rd, lch],
        [-rdk, 0.0],
    ];
    for w in prof.windows(2) {
        en.push(line(w[0], w[1], LAYER_MAIN));
    }
    // 槽壁（全长）+ 倒角边界
    for sgn in [1.0, -1.0] {
        en.push(line([sgn * n / 2.0, 0.0], [sgn * n / 2.0, m], LAYER_MAIN));
        en.push(line([sgn * n / 2.0, lch], [sgn * rd, lch], LAYER_MAIN));
        en.push(line([sgn * n / 2.0, m - c], [sgn * rd, m - c], LAYER_MAIN));
    }
    en.push(line([0.0, -3.0], [0.0, m + 3.0], LAYER_CENTER));
    GenPart {
        entities: en,
        meta: nut812_meta(g),
        bbox: [-rd, 0.0, rd, m],
    }
}

fn round_nut_section(g: &Nut812) -> GenPart {
    let mut en: Vec<EntityType> = Vec::new();
    let (rd, rdk, m, c, c1, r_minor, lch, r_slot, x_slot) =
        (g.rd, g.rdk, g.m, g.c, g.c1, g.r_minor, g.l_ch, g.r_slot, g.x_slot);
    let c_r = (g.d_thread / 2.0 - r_minor) / TAN60; // 右端（d1 面）内孔倒角轴向长
    let r_major = g.d_thread / 2.0;
    let r_bore_l = r_minor + c1 * TAN60; // 左端（dk 面）内孔倒角起点半径
    // 外轮廓（轴线水平 x：d1 面在 x=0，dk 面在 x=-m）
    let prof = [
        [-m, -(rd - c)],
        [-m, rd - c],
        [-(m - c), rd],
        [-lch, rd],
        [0.0, rdk],
        [0.0, -rdk],
        [-lch, -rd],
        [-(m - c), -rd],
        [-m, -(rd - c)],
    ];
    for w in prof.windows(2) {
        en.push(line(w[0], w[1], LAYER_MAIN));
    }
    // 槽底母线
    for sgn in [1.0, -1.0] {
        en.push(line([-m, sgn * r_slot], [-x_slot, sgn * r_slot], LAYER_MAIN));
    }
    // 内孔：小径实线 + 两端 60° 倒角；大径细线全长
    for sgn in [1.0, -1.0] {
        en.push(line([-m, sgn * r_bore_l], [-(m - c1), sgn * r_minor], LAYER_MAIN));
        en.push(line([-(m - c1), sgn * r_minor], [-c_r, sgn * r_minor], LAYER_MAIN));
        en.push(line([-c_r, sgn * r_minor], [0.0, sgn * r_major], LAYER_MAIN));
        en.push(line([-m, sgn * r_major], [0.0, sgn * r_major], LAYER_THIN));
    }
    en.push(line([-(m + 3.0), 0.0], [3.0, 0.0], LAYER_CENTER));
    // ANSI31 剖面线（模板 2 片，scale 0.25，angle 下 0°/上 270°）：边界 = 被切材料轮廓。
    // 模板 M22 实测：(-10,-10.766)-(-9.5,-9.9)-(-0.6351,-9.9)-(0,-11)-(0,-15)-(-0.4124,-15.7143)-(-10,-15.7143)。
    let lower = [
        [-m, -r_bore_l],
        [-(m - c1), -r_minor],
        [-c_r, -r_minor],
        [0.0, -r_major],
        [0.0, -rdk],
        [-x_slot, -r_slot],
        [-m, -r_slot],
    ];
    en.push(hatch_ansi31_scaled(&lower, 0.0, 0.25));
    let upper: Vec<[f64; 2]> = lower.iter().map(|p| [p[0], -p[1]]).collect();
    en.push(hatch_ansi31_scaled(&upper, 270.0, 0.25));
    GenPart {
        entities: en,
        meta: nut812_meta(g),
        bbox: [-m, -rd, 0.0, rd],
    }
}

// ══════════════════════════════════════════════════════════════════════════
// GB/T 858 圆螺母用止动垫圈
//
// 模板逐条实测（d≤100 规格 25、d≥100 规格 160）：
// - 基点 = 垫圈端面中心（d1 端面）× 轴线。
// - 主视图（端视图）：垫圈本体圆 r=D1/2；内孔圆 r=d/2（d 为表内孔实际值），
//   内孔在 270° 处被内舌打断（半角 asin((b/2)/(d/2))）；内舌宽 b、舌根在孔圆上、舌尖到 r=a−d/2；
//   外舌 6 个，每个宽 b、从本体圆 r=D1/2 径向伸到 r=dc/2，舌尖为以原点为心、过两舌尖角的弧
//   （半径 sqrt((dc/2)²+(b/2)²)）；外舌角向：d≤100 为 15/45/75 与 210/240/270，
//   d≥100 为 20/40/60 与 230/250/270（两模板不同 → 拆两族）。
// - 剖视图（轴线竖直，径向水平）：模板为「展开剖」——本体矩形 + 上外舌 + 内舌/下外舌的弯折多边形，
//   外舌轴向错移 Δ=(dc/2−D1/2)·tan25°（两模板实测比值均为 0.46631=tan25°）。
// ══════════════════════════════════════════════════════════════════════════

const TAN25: f64 = 0.466_307_658_411_357;

/// 按公称规格取行（自动按 d 选 d≤100 / d≥100 段）。
fn lock_washer_row_by_d(d: f64) -> Option<&'static LockWasherRow> {
    lock_washer_row(lock_washer_large(d), d)
}

fn lock_washer_row(large: bool, d: f64) -> Option<&'static LockWasherRow> {
    lock_washer_table(large).rows.iter().find(|r| (r.d - d).abs() < 1e-9)
}

fn lock_washer_tab_angles(large: bool) -> &'static [f64] {
    if large {
        &[20.0, 40.0, 60.0, 230.0, 250.0, 270.0]
    } else {
        &[15.0, 45.0, 75.0, 210.0, 240.0, 270.0]
    }
}

fn lock_washer_geom(row: &LockWasherRow) -> (f64, f64, f64, f64, f64) {
    let rh = row.d_hole / 2.0;
    let rb = row.d1 / 2.0;
    let rt = row.dc / 2.0;
    let rit = row.a - rh;
    let y = ((rb * rb) - (row.b / 2.0).powi(2)).sqrt();
    (rh, rb, rt, rit, y)
}

fn lock_washer_meta(row: &LockWasherRow) -> PartMeta {
    let (rh, rb, _, _, _) = lock_washer_geom(row);
    let vol = std::f64::consts::PI * (rb * rb - rh * rh) * row.s;
    PartMeta {
        code: "GB/T 858-1988".into(),
        name: crate::i18n::t_data("圆螺母用止动垫圈").into(),
        spec: if (row.d - row.d.floor()).abs() < 1e-9 {
            format!("Ø{}", trim(row.d))
        } else {
            format!("Ø{}", trim(row.d))
        },
        material: String::new(),
        weight: format!("{:.4}", weight_kg(vol.max(0.0), 7.85)),
    }
}

fn lock_washer_main(row: &LockWasherRow, large: bool) -> GenPart {
    let (rh, rb, rt, rit, _y) = lock_washer_geom(row);
    let b = row.b;
    let mut en: Vec<EntityType> = Vec::new();
    let angles = lock_washer_tab_angles(large);
    let alpha = ((b / 2.0) / rb).asin();
    // 本体圆：槽之间的弧
    for i in 0..angles.len() {
        let a0 = angles[i] + alpha.to_degrees();
        let a1 = angles[(i + 1) % angles.len()] - alpha.to_degrees();
        en.push(arc([0.0, 0.0], rb, a0, a1, LAYER_MAIN));
    }
    // 外舌
    let r_tip = ((rt * rt) + (b / 2.0).powi(2)).sqrt();
    let side_r = ((rb * rb) - (b / 2.0).powi(2)).sqrt();
    for &th_deg in angles {
        let th = th_deg.to_radians();
        let (u, v) = ((th.cos(), th.sin()), (-th.sin(), th.cos()));
        for sgn in [1.0, -1.0] {
            let p0 = [side_r * u.0 + sgn * (b / 2.0) * v.0, side_r * u.1 + sgn * (b / 2.0) * v.1];
            let p1 = [rt * u.0 + sgn * (b / 2.0) * v.0, rt * u.1 + sgn * (b / 2.0) * v.1];
            en.push(line(p0, p1, LAYER_MAIN));
        }
        let c1 = [rt * u.0 - (b / 2.0) * v.0, rt * u.1 - (b / 2.0) * v.1];
        let c2 = [rt * u.0 + (b / 2.0) * v.0, rt * u.1 + (b / 2.0) * v.1];
        let a1 = c1[1].atan2(c1[0]).to_degrees();
        let a2 = c2[1].atan2(c2[0]).to_degrees();
        en.push(arc([0.0, 0.0], r_tip, a1, a2, LAYER_MAIN));
    }
    // 内孔圆（被内舌打断）
    let ah = ((b / 2.0) / rh).asin().to_degrees();
    en.push(arc([0.0, 0.0], rh, 270.0 + ah, 270.0 - ah, LAYER_MAIN));
    // 内舌（开折线）
    let y_hole = ((rh * rh) - (b / 2.0).powi(2)).sqrt();
    en.push(polyline(
        &[
            [-b / 2.0, -y_hole],
            [-b / 2.0, -rit],
            [b / 2.0, -rit],
            [b / 2.0, -y_hole],
        ],
        false,
        LAYER_MAIN,
    ));
    // 中心线（各舌方向 + 十字）
    let o = rt + 3.0;
    for &th in angles.iter().chain([0.0, 90.0, 180.0].iter()) {
        let r = th.to_radians();
        en.push(line([0.0, 0.0], [o * r.cos(), o * r.sin()], LAYER_CENTER));
    }
    GenPart {
        entities: en,
        meta: lock_washer_meta(row),
        bbox: [-r_tip, -r_tip, r_tip, r_tip],
    }
}

fn lock_washer_section(row: &LockWasherRow, large: bool) -> GenPart {
    let (rh, rb, rt, rit, y) = lock_washer_geom(row);
    let (s, h, b) = (row.s, row.h, row.b);
    let delta = (rt - rb) * TAN25;
    let mut en: Vec<EntityType> = Vec::new();
    let _ = b;
    // 本体矩形
    en.push(polyline(
        &[[0.0, rh], [-s, rh], [-s, -rit], [0.0, -rit]],
        true,
        LAYER_MAIN,
    ));
    // 上外舌 + 本体上边
    en.push(polyline(
        &[
            [0.0, rh],
            [0.0, rb],
            [-delta, rt],
            [-delta - s, rt],
            [-s, rb],
            [-s, rh],
        ],
        true,
        LAYER_MAIN,
    ));
    // 内舌（弯折）+ 下外舌
    en.push(polyline(
        &[
            [0.0, -rit],
            [-s, -rit],
            [-h, -rit],
            [-h, -rit - s],
            [-s, -rit - s],
            [-s, -y],
            [-delta - s, -rt],
            [-delta, -rt],
            [0.0, -y],
        ],
        true,
        LAYER_MAIN,
    ));
    let _ = large;
    en.push(line([-h, 0.0], [3.0, 0.0], LAYER_CENTER));
    // ANSI31 剖面线（模板 2 片，angle 0°/scale 1.0）：上片 = 上外舌区域（6 点），
    // 下片 = 内舌 + 下外舌区域（9 点），边界即上面两段材料轮廓。
    en.push(hatch_ansi31_scaled(
        &[[0.0, rh], [0.0, rb], [-delta, rt], [-delta - s, rt], [-s, rb], [-s, rh]],
        0.0,
        1.0,
    ));
    en.push(hatch_ansi31_scaled(
        &[
            [0.0, -rit],
            [-s, -rit],
            [-h, -rit],
            [-h, -rit - s],
            [-s, -rit - s],
            [-s, -y],
            [-delta - s, -rt],
            [-delta, -rt],
            [0.0, -y],
        ],
        0.0,
        1.0,
    ));
    GenPart {
        entities: en,
        meta: lock_washer_meta(row),
        bbox: [-delta - s - 1.0, -rt, s, rt],
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 视图注册 / 目录 JSON / 派发
// ══════════════════════════════════════════════════════════════════════════

/// 本组声明的视图（`partgen_more::family_views` 会先问这里）。
pub fn family_views(family: &str) -> Vec<&'static str> {
    match family {
        "set_screw_77" => vec!["main", "end"],
        "round_nut_812" => vec!["main", "top", "section"],
        "lock_washer_858" => vec!["main", "section"],
        _ => Vec::new(),
    }
}

/// 本组族的目录 JSON 片段（键 = 族 id）。
pub fn families_json() -> serde_json::Map<String, serde_json::Value> {
    use crate::partgen_kit::views_json;
    let mut m = serde_json::Map::new();

    // ── 内六角平端紧定螺钉 ──
    let sizes: Vec<serde_json::Value> = set_screw_table()
        .rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "d": r.d, "label": format!("M{}", trim(r.d)), "pitch": r.pitch,
                "l_min": r.l_min, "l_max": r.l_max, "lengths": r.lengths,
                "extra": format!("dp={} e={} s={} t={}", trim(r.dp), trim(r.e), trim(r.s), trim(r.t_short)),
            })
        })
        .collect();
    m.insert(
        "set_screw_77".into(),
        serde_json::json!({
            "id": "set_screw_77", "name": "内六角平端紧定螺钉",
            "code": "GB/T 77-2007", "iso": "ISO 4027:2003",
            "implemented": true, "views": views_json("set_screw_77"), "sizes": sizes,
            "len_label": "长度 l", "base_hint": "基点 = 内六角端端面 × 轴线",
            "tree_path": "零件库/螺钉/紧定螺钉/内六角平端紧定螺钉 GB/T 77-2007",
        }),
    );

    // ── 圆螺母 ──
    let sizes: Vec<serde_json::Value> = round_nut_table()
        .rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "d": r.d, "label": format!("M{}", trim(r.d)), "pitch": 0.0,
                "l_min": r.m, "l_max": r.m, "lengths": [r.m],
                "extra": format!("外径 {} 端面凹槽 {} n={} t={} c={} c1={}",
                    trim(r.dk), trim(r.d1), trim(r.n), trim(r.t), trim(r.c), trim(r.c1)),
            })
        })
        .collect();
    m.insert(
        "round_nut_812".into(),
        serde_json::json!({
            "id": "round_nut_812", "name": "圆螺母",
            "code": "GB/T 812-1988", "iso": "—",
            "implemented": true, "views": views_json("round_nut_812"), "sizes": sizes,
            "len_label": "高度 m", "base_hint": "基点 = 端面（d1 凸台面）× 轴线",
            "tree_path": "零件库/螺母/圆螺母/圆螺母 GB/T 812-1988",
        }),
    );

    // ── 止动垫圈（一个族；d≤100 / d≥100 两段模板同构，仅外舌角向不同） ──
    {
        let sizes: Vec<serde_json::Value> = lock_washer_all_rows()
            .iter()
            .map(|r| {
                serde_json::json!({
                    "d": r.d, "label": format!("Ø{}", trim(r.d)), "pitch": 0.0,
                    "l_min": r.s, "l_max": r.s, "lengths": [r.s],
                    "extra": format!("内孔 {} 外径 {} 外舌 {} b={} a={} h={}",
                        trim(r.d_hole), trim(r.d1), trim(r.dc), trim(r.b), trim(r.a), trim(r.h)),
                })
            })
            .collect();
        m.insert(
            "lock_washer_858".into(),
            serde_json::json!({
                "id": "lock_washer_858", "name": "圆螺母用止动垫圈",
                "code": "GB/T 858-1988", "iso": "—",
                "implemented": true, "views": views_json("lock_washer_858"), "sizes": sizes,
                "len_label": "厚度 s", "base_hint": "基点 = 垫圈端面中心（轴线）",
                "tree_path": "零件库/垫圈/止动垫圈/圆螺母用止动垫圈 GB/T 858-1988",
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
    Some(match family {
        "set_screw_77" => {
            let Some(row) = set_screw_row(d) else {
                return Some(Err(crate::i18n::t_fmt(
                    "cmd.parts.err.no_m",
                    &[("table", "GB/T 77-2007"), ("d", &trim(d))],
                )));
            };
            if let Err(e) = check_length(row.l_min, row.l_max, d, l) {
                return Some(Err(e));
            }
            match view {
                "main" => Ok(set_screw_main(row, l)),
                "end" => Ok(set_screw_end(row, l)),
                _ => unreachable!(),
            }
        }
        "round_nut_812" => {
            let Some(row) = round_nut_row(d) else {
                return Some(Err(crate::i18n::t_fmt(
                    "cmd.parts.err.no_m",
                    &[("table", "GB/T 812-1988"), ("d", &trim(d))],
                )));
            };
            let g = nut812_geom(row);
            match view {
                "main" => Ok(round_nut_main(&g)),
                "top" => Ok(round_nut_top(&g)),
                "section" => Ok(round_nut_section(&g)),
                _ => unreachable!(),
            }
        }
        "lock_washer_858" => {
            let large = lock_washer_large(d);
            let Some(row) = lock_washer_row_by_d(d) else {
                return Some(Err(crate::i18n::t_fmt(
                    "cmd.parts.err.no_dia",
                    &[("table", "GB/T 858-1988"), ("d", &trim(d))],
                )));
            };
            match view {
                "main" => Ok(lock_washer_main(row, large)),
                "section" => Ok(lock_washer_section(row, large)),
                _ => unreachable!(),
            }
        }
        _ => return None,
    })
}

// ══════════════════════════════════════════════════════════════════════════
// 测试
// ══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::partgen::{generate as gen, BoltView};
    use crate::partgen_kit::LAYER_HATCH;
    use ocs_plugin_api::host::acadrust::entities::EntityCommon;
    use ocs_plugin_api::host::acadrust::types::Color;

    fn common(e: &EntityType) -> &EntityCommon {
        match e {
            EntityType::Line(x) => &x.common,
            EntityType::Arc(x) => &x.common,
            EntityType::Circle(x) => &x.common,
            EntityType::LwPolyline(x) => &x.common,
            EntityType::Hatch(x) => &x.common,
            _ => panic!("未知实体"),
        }
    }

    fn near(p: [f64; 2], q: [f64; 2]) -> bool {
        (p[0] - q[0]).abs() < 1e-3 && (p[1] - q[1]).abs() < 1e-3
    }

    fn seg_match2(p: [f64; 2], q: [f64; 2], a: [f64; 2], b: [f64; 2]) -> bool {
        (near(p, a) && near(q, b)) || (near(p, b) && near(q, a))
    }

    fn seg_match(segs: &[([f64; 2], [f64; 2])], a: [f64; 2], b: [f64; 2]) -> bool {
        segs.iter().any(|(p, q)| seg_match2(*p, *q, a, b))
    }

    fn check_part(fam: &str, d: f64, l: f64, view: &str) -> GenPart {        let p = gen(fam, d, l, view).unwrap_or_else(|e| panic!("{fam} {d}×{l} {view}: {e}"));
        assert!(!p.entities.is_empty(), "{fam} {view} 空");
        for e in &p.entities {
            let lay = common(e).layer.as_str();
            assert!(
                [LAYER_MAIN, LAYER_THIN, LAYER_CENTER, LAYER_HIDDEN, LAYER_HATCH].contains(&lay),
                "{fam} {view} 图层越界: {lay}"
            );
            assert!(matches!(common(e).color, Color::ByLayer), "{fam} {view} 非 ByLayer");
        }
        p
    }

    /// 全部规格 × 全部视图可生成，且图元只在 1/2/3/4 层、无尺寸标注。
    #[test]
    fn all_specs_all_views() {
        for row in &set_screw_table().rows {
            check_part("set_screw_77", row.d, row.lengths[0], "main");
            check_part("set_screw_77", row.d, row.lengths[0], "end");
        }
        for row in &round_nut_table().rows {
            for v in ["main", "top", "section"] {
                check_part("round_nut_812", row.d, row.m, v);
            }
        }
        for large in [false, true] {
            for row in &lock_washer_table(large).rows {
                for v in ["main", "section"] {
                    check_part("lock_washer_858", row.d, row.s, v);
                }
            }
        }
    }

    /// 本组三族的目录/树接线自检（全局护栏的一部分，单独跑不受其它批影响）。
    #[test]
    fn catalog_wiring() {
        let cat: serde_json::Value =
            serde_json::from_str(&crate::partgen::catalog_json()).expect("catalog json");
        let fams = cat["families"].as_object().unwrap();
        for id in ["set_screw_77", "round_nut_812", "lock_washer_858"] {
            let f = &fams[id];
            assert!(f.is_object(), "目录缺族 {id}");
            assert_eq!(f["implemented"], serde_json::json!(true));
            assert!(f["tree_path"].is_string(), "{id} 缺 tree_path");
            let views: Vec<String> = f["views"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v["id"].as_str().unwrap().to_string())
                .collect();
            assert_eq!(views, family_views(id), "{id} views 不一致");
            let sizes = f["sizes"].as_array().unwrap();
            assert!(!sizes.is_empty(), "{id} 无规格");
            let d = sizes[0]["d"].as_f64().unwrap();
            let l = sizes[0]["lengths"][0].as_f64().unwrap();
            for v in &views {
                crate::partgen::generate(id, d, l, v)
                    .unwrap_or_else(|e| panic!("{id} Ø{d}×{l} {v}: {e}"));
            }
        }
        // 树里挂上本组三族
        let tree = cat["tree"].to_string();
        for needle in ["set_screw_77", "round_nut_812", "lock_washer_858"] {
            assert!(tree.contains(needle), "树缺 {needle}");
        }
    }

    /// 不属于本组的族必须返回 None / 空。
    #[test]
    fn no_trespassing() {
        assert!(family_views("hex_bolt_c").is_empty());
        assert!(generate("hex_bolt_c", 10.0, 20.0, "main").is_none());
        assert!(families_json().get("hex_bolt_c").is_none());
    }

    /// 视图校验。
    #[test]
    fn bad_view_rejected() {
        assert!(generate("set_screw_77", 10.0, 25.0, "top").unwrap().is_err());
        assert!(generate("round_nut_812", 22.0, 10.0, "end").unwrap().is_err());
        assert!(generate("lock_washer_858", 25.0, 1.0, "top").unwrap().is_err());
    }

    // ── 模板规格逐条数值回归 ──────────────────────────────────────────────

    /// GB/T 77 M10×25：模板 TSV 实测坐标（M10, l=25）。
    #[test]
    fn regress_set_screw_77_m10() {
        let row = set_screw_row(10.0).unwrap();
        let p = set_screw_main(row, 25.0);
        let segs: Vec<([f64; 2], [f64; 2])> = p
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Line(x) => Some((
                    [x.start.x, x.start.y],
                    [x.end.x, x.end.y],
                )),
                _ => None,
            })
            .collect();
        let has = |a: [f64; 2], b: [f64; 2]| {
            seg_match(&segs, a, b)
        };
        // 端面 / 倒角 / 杆身 / 平端（模板 1轮廓实线层）
        assert!(has([0.0, -4.25], [0.0, 4.25]), "端面");
        assert!(has([0.0, -4.25], [0.75, -5.0]), "端部倒角");
        assert!(has([0.75, -5.0], [23.5, -5.0]), "杆身");
        assert!(has([23.5, -5.0], [25.0, -3.5]), "平端锥");
        assert!(has([25.0, -3.5], [25.0, 3.5]), "平端面");
        // 螺纹牙底细实线（2细线层）0→24.25
        assert!(has([0.0, -4.25], [24.25, -4.25]), "牙底细线");
        // 内六角孔（4虚线层）：母线 y=±2.8615 / ±1.4181、孔底 x=4、锥尖 x=5.6521
        assert!(has([0.0, -2.8615], [4.0, -2.8615]), "孔外母线");
        assert!(has([0.0, -1.4181], [4.0, -1.4181]), "孔内母线");
        assert!(has([4.0, -2.8615], [4.0, 2.8615]), "孔底");
        assert!(has([4.0, -2.8615], [5.6521, 0.0]), "孔锥");
        // 左视图
        let e = set_screw_end(row, 25.0);
        let circles: Vec<f64> = e
            .entities
            .iter()
            .filter_map(|x| match x {
                EntityType::Circle(c) => Some(c.radius),
                _ => None,
            })
            .collect();
        assert!(circles.iter().any(|r| (r - 5.0).abs() < 1e-9), "外圆 r5");
        assert!(circles.iter().any(|r| (r - 2.8615).abs() < 1e-9), "六角口外圆 r=e/2");
        let arc_r: Vec<f64> = e
            .entities
            .iter()
            .filter_map(|x| match x {
                EntityType::Arc(a) => Some(a.radius),
                _ => None,
            })
            .collect();
        assert!(arc_r.iter().any(|r| (r - 4.25).abs() < 1e-9), "牙底 3/4 弧 r=0.85d/2");
        // 六角口左右顶点 (2.5, ±1.4181)
        let pts: Vec<[f64; 2]> = e
            .entities
            .iter()
            .filter_map(|x| match x {
                EntityType::Line(l) => Some([[l.start.x, l.start.y], [l.end.x, l.end.y]]),
                EntityType::LwPolyline(_) => None,
                _ => None,
            })
            .flatten()
            .collect();
        assert!(
            pts.iter().any(|p| (p[0] - 2.5).abs() < 1e-4 && (p[1] - 1.4181).abs() < 1e-4),
            "六角口侧顶点"
        );
    }

    /// GB/T 812 M22：模板 TSV 实测坐标。模板 D=表 dk=38、模板 dk=表 d1=30。
    #[test]
    fn regress_round_nut_812_m22() {
        let row = round_nut_row(22.0).unwrap();
        let g = nut812_geom(row);
        assert!((g.rd - 19.0).abs() < 1e-9, "外径半径");
        assert!((g.rdk - 15.0).abs() < 1e-9, "端面凸台半径");
        assert!((g.r_minor - 9.9).abs() < 1e-9, "内孔小径 0.45D");
        assert!((g.l_ch - 2.309_401_6).abs() < 1e-3, "d1 锥轴向长");
        assert!((g.r_slot - 15.714_285_7).abs() < 1e-3, "槽底母线");
        assert!((g.x_slot - 0.412_41).abs() < 1e-3, "槽底平面 x");
        // 主视图：槽底竖线 x=15.7143、槽宽 5.3、4 条外弧
        let m = round_nut_main(&g);
        let has_line = |a: [f64; 2], b: [f64; 2]| {
            m.entities.iter().any(|e| match e {
                EntityType::Line(l) => {
                    let p = [l.start.x, l.start.y];
                    let q = [l.end.x, l.end.y];
                    (near(p, a) && near(q, b)) || (near(p, b) && near(q, a))
                }
                _ => false,
            })
        };
        assert!(has_line([15.714_285_7, -2.65], [15.714_285_7, 2.65]), "槽底");
        assert!(has_line([18.814_29, -2.65], [15.714_285_7, -2.65]), "槽壁");
        // 剖视图：外形关键点
        let s = round_nut_section(&g);
        let sl: Vec<([f64; 2], [f64; 2])> = s
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Line(l) => Some(([l.start.x, l.start.y], [l.end.x, l.end.y])),
                _ => None,
            })
            .collect();
        let hs = |a: [f64; 2], b: [f64; 2]| {
            sl.iter().any(|(p, q)| seg_match2(*p, *q, a, b))
        };
        assert!(hs([-9.0, 19.0], [-10.0, 18.0]), "c 倒角");
        assert!(hs([-10.0, 15.714_285_7], [-0.412_41, 15.714_285_7]), "槽底母线");
        assert!(hs([-10.0, 10.766], [-9.5, 9.9]), "内孔左倒角");
        // 俯视图（轴向侧视，轴线竖直）：模板 (19,9)-(2.65,9)、(19,2.3094)-(2.65,2.3094)
        let tp = round_nut_top(&g);
        let tl: Vec<([f64; 2], [f64; 2])> = tp
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Line(l) => Some(([l.start.x, l.start.y], [l.end.x, l.end.y])),
                _ => None,
            })
            .collect();
        assert!(seg_match(&tl, [2.65, 9.0], [19.0, 9.0]), "c 倒角边界");
        assert!(seg_match(&tl, [2.65, 2.309_401_6], [19.0, 2.309_401_6]), "d1 锥边界");
    }

    /// GB/T 858 规格 25（d≤100）模板 TSV 实测坐标。
    #[test]
    fn regress_lock_washer_858_s_25() {
        let row = lock_washer_row(false, 25.0).unwrap();
        assert!((row.d_hole - 25.5).abs() < 1e-9);
        let (rh, rb, rt, rit, y) = lock_washer_geom(row);
        assert!((rh - 12.75).abs() < 1e-9);
        assert!((rb - 17.0).abs() < 1e-9);
        assert!((rt - 22.5).abs() < 1e-9);
        assert!((rit - 9.25).abs() < 1e-9);
        assert!((y - 16.829_698_4).abs() < 1e-3, "舌根 y={y}");
        let m = lock_washer_main(row, false);
        let has = |a: [f64; 2], b: [f64; 2]| {
            m.entities.iter().any(|e| match e {
                EntityType::Line(l) => {
                    let p = [l.start.x, l.start.y];
                    let q = [l.end.x, l.end.y];
                    (near(p, a) && near(q, b)) || (near(p, b) && near(q, a))
                }
                _ => false,
            })
        };
        // 内舌竖线 x=±2.4 从 y=-16.8297 到 -22.5（270° 外舌侧壁也是同一组线）
        assert!(has([2.4, -16.829_698_4], [2.4, -22.5]), "270° 外舌侧壁");
        // 内舌（折线）：(-2.4,-12.5221)->(-2.4,-9.25)
        let poly: Vec<(bool, Vec<[f64; 2]>)> = m
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::LwPolyline(pl) => Some((
                    pl.is_closed,
                    pl.vertices.iter().map(|v| [v.location.x, v.location.y]).collect(),
                )),
                _ => None,
            })
            .collect();
        assert!(
            poly.iter().any(|(_, v)| v.iter().any(|p| {
                (p[0] + 2.4).abs() < 1e-3 && (p[1] + 12.522_09).abs() < 1e-3
            })),
            "内舌根 (-2.4,-12.5221)"
        );
        // 剖视图：关键点
        let sec = lock_washer_section(row, false);
        let pts: Vec<[f64; 2]> = sec
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::LwPolyline(pl) => {
                    Some(pl.vertices.iter().map(|v| [v.location.x, v.location.y]).collect::<Vec<_>>())
                }
                _ => None,
            })
            .flatten()
            .collect();
        assert!(pts.iter().any(|p| (p[0] + 2.564_7).abs() < 1e-3 && (p[1] - 22.5).abs() < 1e-3), "上外舌尖 (-2.5647,22.5)");
        assert!(pts.iter().any(|p| (p[0] + 3.564_7).abs() < 1e-3 && (p[1] - 22.5).abs() < 1e-3), "上外舌尖 (-3.5647,22.5)");
    }

    /// GB/T 858 规格 160（d≥100）模板 TSV 实测坐标。
    #[test]
    fn regress_lock_washer_858_l_160() {
        let row = lock_washer_row(true, 160.0).unwrap();
        let (rh, rb, rt, rit, y) = lock_washer_geom(row);
        assert!((rh - 80.5).abs() < 1e-9);
        assert!((rb - 95.0).abs() < 1e-9);
        assert!((rt - 108.0).abs() < 1e-9);
        assert!((rit - 75.5).abs() < 1e-9);
        assert!((y - 94.683_37).abs() < 1e-2, "舌根 y={y}");
        let sec = lock_washer_section(row, true);
        let pts: Vec<[f64; 2]> = sec
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::LwPolyline(pl) => {
                    Some(pl.vertices.iter().map(|v| [v.location.x, v.location.y]).collect::<Vec<_>>())
                }
                _ => None,
            })
            .flatten()
            .collect();
        // 模板 858l：上外舌尖 (-6.062,108)/(-8.062,108)，内舌尖 (-8,-75.5)
        assert!(pts.iter().any(|p| (p[0] + 6.062).abs() < 1e-3 && (p[1] - 108.0).abs() < 1e-3), "上外舌尖");
        assert!(pts.iter().any(|p| (p[0] + 8.0).abs() < 1e-3 && (p[1] + 75.5).abs() < 1e-3), "内舌尖");
    }

    /// 模板剖视图剖面线回归（片数 / angle / scale / 边界逐点）。
    ///
    /// 模板权威读数（ezdxf）：812 剖视 2 片 scale 0.25（下 0°/上 270°）；
    /// 858 两段模板剖视各 2 片，均 angle 0°/scale 1.0。812 的上片在模板里以 OCS(extrusion=0,0,-1)
    /// 存储（存储 x 为负），本库按既有规则换算为 x→−x，即“上片 = 下片关于 y=0 镜像”；
    /// 858 两个 HATCH 都是 extrusion=+z，存储即真 WCS，无需换算。
    #[test]
    fn hatch_sections_match_template() {
        use ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge;
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
        let verts_match = |got: &[[f64; 2]], want: &[[f64; 2]]| -> bool {
            got.len() == want.len()
                && want.iter().all(|w| got.iter().any(|g| (g[0] - w[0]).abs() <= 1e-3 && (g[1] - w[1]).abs() <= 1e-3))
        };

        // ── round_nut_812 / section（M22）──
        let p = gen("round_nut_812", 22.0, 10.0, "section").unwrap();
        let hs = hatches(&p);
        assert_eq!(hs.len(), 2, "812 剖视应 2 片剖面线");
        let lower = [
            [-10.0, -10.766],
            [-9.5, -9.9],
            [-0.6351, -9.9],
            [0.0, -11.0],
            [0.0, -15.0],
            [-0.4124, -15.7143],
            [-10.0, -15.7143],
        ];
        // 模板上片 OCS → 真 WCS（x→−x）
        let upper: Vec<[f64; 2]> = lower.iter().map(|q| [q[0], -q[1]]).collect();
        for (ang, scale, verts) in &hs {
            assert!((*scale - 0.25).abs() < 1e-9, "812 scale={scale}");
            if (*ang - 0.0).abs() < 1e-9 {
                assert!(verts_match(verts, &lower), "812 下片边界 {verts:?}");
            } else if (*ang - 270.0).abs() < 1e-9 {
                assert!(verts_match(verts, &upper), "812 上片边界 {verts:?}");
            } else {
                panic!("812 意外 angle={ang}");
            }
        }

        // ── lock_washer_858 / section（d≤100 规格 25、d≥100 规格 160）──
        //
        // 注：模板与用户 PNG 参数表给 d≥100 的 s 都 = 2.0，但矿采的
        // `partsLockWasher858l.json` 在 d≥150 行写成 2.5（数据 bug，本次不越权改表）。
        // 因此 d=160 的剖面线边界用生成的 s 参数化验证；d=25 直接对模板硬值。
        for d in [25.0, 160.0] {
            let row = lock_washer_row(lock_washer_large(d), d).unwrap();
            let (rh, rb, rt, rit, y) = lock_washer_geom(row);
            let s = row.s;
            let h = row.h;
            let delta = (rt - rb) * TAN25;
            let lower: Vec<[f64; 2]> = vec![
                [0.0, -rit],
                [-s, -rit],
                [-h, -rit],
                [-h, -rit - s],
                [-s, -rit - s],
                [-s, -y],
                [-delta - s, -rt],
                [-delta, -rt],
                [0.0, -y],
            ];
            let upper: Vec<[f64; 2]> = vec![
                [0.0, rh],
                [0.0, rb],
                [-delta, rt],
                [-delta - s, rt],
                [-s, rb],
                [-s, rh],
            ];
            let p = gen("lock_washer_858", d, s, "section").unwrap();
            let hs = hatches(&p);
            assert_eq!(hs.len(), 2, "858 d{d} 剖视应 2 片剖面线");
            for (ang, scale, verts) in &hs {
                assert!((*ang - 0.0).abs() < 1e-9, "858 d{d} angle={ang}");
                assert!((*scale - 1.0).abs() < 1e-9, "858 d{d} scale={scale}");
                assert!(
                    verts_match(verts, &upper) || verts_match(verts, &lower),
                    "858 d{d} 边界 {verts:?}"
                );
            }
            // 两片必须一片上、一片下（不能都是同一片）
            assert!(hs.iter().any(|h| verts_match(&h.2, &upper)), "858 d{d} 缺上片");
            assert!(hs.iter().any(|h| verts_match(&h.2, &lower)), "858 d{d} 缺下片");
        }
        // d=25：模板实测边界逐点 ≤1e-3（模板 s=1.0 = 表 s）
        let p = gen("lock_washer_858", 25.0, 1.0, "section").unwrap();
        let hs = hatches(&p);
        let tpl_upper = [
            [0.0, 12.75],
            [0.0, 17.0],
            [-2.5647, 22.5],
            [-3.5647, 22.5],
            [-1.0, 17.0],
            [-1.0, 12.75],
        ];
        let tpl_lower = [
            [0.0, -9.25],
            [-1.0, -9.25],
            [-4.0, -9.25],
            [-4.0, -10.25],
            [-1.0, -10.25],
            [-1.0, -16.8297],
            [-3.5647, -22.5],
            [-2.5647, -22.5],
            [0.0, -16.8297],
        ];
        assert!(hs.iter().any(|h| verts_match(&h.2, &tpl_upper)), "858 d25 与模板上片不符");
        assert!(hs.iter().any(|h| verts_match(&h.2, &tpl_lower)), "858 d25 与模板下片不符");
        // d=160：表 s 已按**用户参数表 PNG + 模板 DIM** 从 164580 的 2.5 修回 2.0，
        // 所以边界现在应与模板**逐点**一致（不再有 s 推移的容差）。
        let row = lock_washer_row(true, 160.0).unwrap();
        assert!((row.s - 2.0).abs() < 1e-9, "d160 的 s 应为 2（用户 PNG/模板 DIM）");
        let p = gen("lock_washer_858", 160.0, row.s, "section").unwrap();
        let hs = hatches(&p);
        let tpl_upper = [
            [0.0, 80.5],
            [0.0, 95.0],
            [-6.062, 108.0],
            [-8.062, 108.0],
            [-2.0, 95.0],
            [-2.0, 80.5],
        ];
        let got = hs
            .iter()
            .find(|h| (h.0 - 0.0).abs() < 1e-9 && h.2.len() == 6)
            .map(|h| h.2.clone())
            .expect("858 d160 上片");
        for (g, t) in got.iter().zip(tpl_upper.iter()) {
            assert!(
                (g[0] - t[0]).abs() <= 1e-3 && (g[1] - t[1]).abs() <= 1e-3,
                "858 d160 上片顶点 {g:?} vs 模板 {t:?}"
            );
        }
        // 下片 9 点：y 顶点必须落在模板的 {±75.5, ±77.5, ±94.6834, ±108} 上
        let low = hs
            .iter()
            .find(|h| h.2.len() == 9)
            .map(|h| h.2.clone())
            .expect("858 d160 下片");
        for v in &low {
            let y = v[1].abs();
            assert!(
                [75.5, 77.5, 94.6834, 108.0]
                    .iter()
                    .any(|t| (y - t).abs() <= 1e-3),
                "858 d160 下片 y 顶点 {v:?} 不在模板值集内"
            );
        }
    }

    /// 数据表健全性：单调、范围合理。
    #[test]
    fn tables_sane() {
        let t = set_screw_table();
        assert_eq!(t.rows.len(), 13);
        for w in t.rows.windows(2) {
            assert!(w[1].d > w[0].d);
        }
        let n = round_nut_table();
        assert_eq!(n.rows.len(), 48);
        for r in &n.rows {
            assert!(r.n > 0.0 && r.t > 0.0 && r.dk > r.d1);
        }
        let s = lock_washer_table(false);
        let l = lock_washer_table(true);
        assert_eq!(s.rows.len() + l.rows.len(), 48);
        for r in s.rows.iter().chain(l.rows.iter()) {
            assert!(r.d1 > r.d_hole);
            let rit = r.a - r.d_hole / 2.0;
            assert!(rit > 0.0, "内舌尖半径必须为正: {r:?}");
        }
    }

    /// 长度越界报错。
    #[test]
    fn length_check() {
        assert!(generate("set_screw_77", 10.0, 5.0, "main").unwrap().is_err());
        assert!(generate("set_screw_77", 10.0, 25.0, "main").unwrap().is_ok());
    }

    // ── 叠合 / 出图（人工核对，#[ignore]） ────────────────────────────────

    /// 生成段列表（仅 `1轮廓实线层`；线段 + 圆/弧按折线近似）。
    fn outline_segments(p: &GenPart) -> Vec<([f64; 2], [f64; 2])> {
        let mut out = Vec::new();
        for e in &p.entities {
            if common(e).layer != LAYER_MAIN {
                continue;
            }
            match e {
                EntityType::Line(l) => out.push(([l.start.x, l.start.y], [l.end.x, l.end.y])),
                EntityType::Circle(c) => {
                    let n = 64;
                    for i in 0..n {
                        let a0 = i as f64 / n as f64 * std::f64::consts::TAU;
                        let a1 = (i + 1) as f64 / n as f64 * std::f64::consts::TAU;
                        out.push((
                            [c.center.x + c.radius * a0.cos(), c.center.y + c.radius * a0.sin()],
                            [c.center.x + c.radius * a1.cos(), c.center.y + c.radius * a1.sin()],
                        ));
                    }
                }
                EntityType::Arc(a) => {
                    let n = 48;
                    let (s, e2) = (a.start_angle, a.end_angle);
                    let span = if e2 > s { e2 - s } else { e2 - s + std::f64::consts::TAU };
                    for i in 0..n {
                        let t0 = s + span * i as f64 / n as f64;
                        let t1 = s + span * (i + 1) as f64 / n as f64;
                        out.push((
                            [a.center.x + a.radius * t0.cos(), a.center.y + a.radius * t0.sin()],
                            [a.center.x + a.radius * t1.cos(), a.center.y + a.radius * t1.sin()],
                        ));
                    }
                }
                EntityType::LwPolyline(pl) => {
                    let pts: Vec<[f64; 2]> = pl.vertices.iter().map(|v| [v.location.x, v.location.y]).collect();
                    for w in pts.windows(2) {
                        out.push((w[0], w[1]));
                    }
                    if pl.is_closed && pts.len() > 2 {
                        out.push((*pts.last().unwrap(), pts[0]));
                    }
                }
                _ => {}
            }
        }
        out
    }

    fn read_template_outline(path: &str, keep: &dyn Fn(f64, f64) -> bool) -> Vec<([f64; 2], [f64; 2])> {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for ln in text.lines().skip(1) {
            let cols: Vec<&str> = ln.split('\t').collect();
            if cols.len() < 5 || cols[1] != "1轮廓实线层" {
                continue;
            }
            let nums: Vec<f64> = cols[4]
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            match cols[0] {
                "LINE" if nums.len() >= 4 => {
                    if keep(nums[0], nums[1]) && keep(nums[2], nums[3]) {
                        out.push(([nums[0], nums[1]], [nums[2], nums[3]]));
                    }
                }
                "CIRCLE" if nums.len() >= 3 => {
                    let n = 64;
                    for i in 0..n {
                        let a0 = i as f64 / n as f64 * std::f64::consts::TAU;
                        let a1 = (i + 1) as f64 / n as f64 * std::f64::consts::TAU;
                        out.push((
                            [nums[0] + nums[2] * a0.cos(), nums[1] + nums[2] * a0.sin()],
                            [nums[0] + nums[2] * a1.cos(), nums[1] + nums[2] * a1.sin()],
                        ));
                    }
                }
                "ARC" if nums.len() >= 5 => {
                    let n = 48;
                    let (s, e2) = (nums[3].to_radians(), nums[4].to_radians());
                    let span = if e2 > s { e2 - s } else { e2 - s + std::f64::consts::TAU };
                    for i in 0..n {
                        let t0 = s + span * i as f64 / n as f64;
                        let t1 = s + span * (i + 1) as f64 / n as f64;
                        out.push((
                            [nums[0] + nums[2] * t0.cos(), nums[1] + nums[2] * t0.sin()],
                            [nums[0] + nums[2] * t1.cos(), nums[1] + nums[2] * t1.sin()],
                        ));
                    }
                }
                k if k.starts_with("POLY") || k.starts_with("LWPOLY") => {
                    let pts: Vec<[f64; 2]> = nums.chunks(3).filter(|c| c.len() >= 2).map(|c| [c[0], c[1]]).collect();
                    for w in pts.windows(2) {
                        if keep(w[0][0], w[0][1]) && keep(w[1][0], w[1][1]) {
                            out.push((w[0], w[1]));
                        }
                    }
                    if k.contains('C') && pts.len() > 2 {
                        let (a, b) = (pts[pts.len() - 1], pts[0]);
                        if keep(a[0], a[1]) && keep(b[0], b[1]) {
                            out.push((a, b));
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    fn write_overlay(name: &str, tmpl: &[([f64; 2], [f64; 2])], mine: &[([f64; 2], [f64; 2])]) {
        let all: Vec<[f64; 2]> = tmpl.iter().chain(mine.iter()).flat_map(|(a, b)| [*a, *b]).collect();
        if all.is_empty() {
            return;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for p in &all {
            x0 = x0.min(p[0]); y0 = y0.min(p[1]); x1 = x1.max(p[0]); y1 = y1.max(p[1]);
        }
        let pad = ((x1 - x0).max(y1 - y0) * 0.08).max(1.0);
        let px = 1200.0;
        let sc = px / ((x1 - x0) + 2.0 * pad).max(1e-6);
        let py = ((y1 - y0) + 2.0 * pad) * sc;
        let t = |p: [f64; 2]| ((p[0] - x0 + pad) * sc, py - (p[1] - y0 + pad) * sc);
        let mut svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{px:.0}" height="{py:.0}"><rect width="100%" height="100%" fill="#fff"/>"##
        );
        for (a, b) in tmpl {
            let (a, b) = (t(*a), t(*b));
            svg.push_str(&format!(
                r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="#d00" stroke-width="1.2"/>"##,
                a.0, a.1, b.0, b.1
            ));
        }
        for (a, b) in mine {
            let (a, b) = (t(*a), t(*b));
            svg.push_str(&format!(
                r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="#00d" stroke-width="1.0" opacity="0.8"/>"##,
                a.0, a.1, b.0, b.1
            ));
        }
        svg.push_str("</svg>");
        let _ = std::fs::write(format!("/tmp/b2/overlay-{name}.svg"), svg);
    }

    fn home() -> String {
        std::env::var("HOME").unwrap_or_else(|_| "/home/ysdirector".into())
    }

    /// 模板 TSV 与生成几何的 1轮廓实线层叠合图（红=模板、蓝=生成）。
    #[test]
    #[ignore]
    fn overlay_b2() {
        let _ = std::fs::create_dir_all("/tmp/b2");
        let h = home();
        let base = format!("{h}/桌面/GB/参数化");

        // 77 主视图（模板 upper 半是 180° 复制，仅取 x≥0 的下半）
        let tmpl = read_template_outline(
            &format!("{base}/内六角平端紧定螺钉_GB-T77-2007/内六角平端紧定螺钉_GB-T77-2007-主视图.dxf.tsv"),
            &|_, _| true,
        );
        // 若没有 .dxf.tsv，尝试已 dump 的 /tmp/b2 TSV
        let tmpl = if tmpl.is_empty() {
            read_template_outline("/tmp/b2/screw77_main.tsv", &|x, _| x >= -1e-6)
        } else {
            tmpl
        };
        let row = set_screw_row(10.0).unwrap();
        let p = set_screw_main(row, 25.0);
        write_overlay("set_screw_77-main", &tmpl, &outline_segments(&p));
        let tmpl_e = read_template_outline("/tmp/b2/screw77_end.tsv", &|_, _| true);
        let pe = set_screw_end(row, 25.0);
        write_overlay("set_screw_77-end", &tmpl_e, &outline_segments(&pe));

        // 812 三视图
        for (v, file) in [
            ("main", "/tmp/b2/nut812_main.tsv"),
            ("top", "/tmp/b2/nut812_top.tsv"),
            ("section", "/tmp/b2/nut812_section.tsv"),
        ] {
            let tmpl = read_template_outline(file, &|_, _| true);
            let row = round_nut_row(22.0).unwrap();
            let g = nut812_geom(row);
            let p = match v {
                "main" => round_nut_main(&g),
                "top" => round_nut_top(&g),
                _ => round_nut_section(&g),
            };
            write_overlay(&format!("round_nut_812-{v}"), &tmpl, &outline_segments(&p));
        }

        // 858 两模板 × 两视图
        let r25 = lock_washer_row(false, 25.0).unwrap();
        let tmpl = read_template_outline("/tmp/b2/washer858s_main.tsv", &|_, _| true);
        write_overlay("lock_washer_858_s-main", &tmpl, &outline_segments(&lock_washer_main(r25, false)));
        let tmpl = read_template_outline("/tmp/b2/washer858s_section.tsv", &|_, _| true);
        write_overlay("lock_washer_858_s-section", &tmpl, &outline_segments(&lock_washer_section(r25, false)));
        let r160 = lock_washer_row(true, 160.0).unwrap();
        let tmpl = read_template_outline("/tmp/b2/washer858l_main.tsv", &|_, _| true);
        write_overlay("lock_washer_858_l-main", &tmpl, &outline_segments(&lock_washer_main(r160, true)));
        let tmpl = read_template_outline("/tmp/b2/washer858l_section.tsv", &|_, _| true);
        write_overlay("lock_washer_858_l-section", &tmpl, &outline_segments(&lock_washer_section(r160, true)));
    }

    /// 每族 × 每视图落成 /tmp/b2/*.svg（人工看图）。
    #[test]
    #[ignore]
    fn dump_b2_svg() {
        let _ = std::fs::create_dir_all("/tmp/b2");
        let dump = |fam: &str, d: f64, l: f64, view: &str, tag: &str| {
            let p = gen(fam, d, l, view).unwrap();
            let path = format!("/tmp/b2/{tag}-{view}.svg");
            crate::partgen_kit::dump_svg(&p, &path).unwrap();
        };
        dump("set_screw_77", 10.0, 25.0, "main", "set_screw_77-M10x25");
        dump("set_screw_77", 10.0, 25.0, "end", "set_screw_77-M10x25");
        dump("round_nut_812", 22.0, 10.0, "main", "round_nut_812-M22");
        dump("round_nut_812", 22.0, 10.0, "top", "round_nut_812-M22");
        dump("round_nut_812", 22.0, 10.0, "section", "round_nut_812-M22");
        dump("lock_washer_858", 25.0, 1.0, "main", "lock_washer_858-25");
        dump("lock_washer_858", 25.0, 1.0, "section", "lock_washer_858-25");
        dump("lock_washer_858", 160.0, 2.0, "main", "lock_washer_858-160");
        dump("lock_washer_858", 160.0, 2.0, "section", "lock_washer_858-160");
        let _ = BoltView::Main;
    }
}
