//! 平键参数化（GB/T 1096-2003 普通平键 A/B/C 型；GB/T 1097-2003 导向平键 A/B 型）。
//!
//! ## 素材与依据
//!
//! - 几何反解：`~/桌面/OCSM/review/平键_几何反解.md`（逐图元 WCS 坐标 + 尺寸实测 + OCS 镜像坑）。
//! - 数据表：`平键_表格_1096{A,B,C}.csv`（各 20 档 `O.,b,h,L`）、
//!   `平键_表格_1097{A,B}.csv`（各 14 档 `O.,b,h,C,h1,d0,d1,D,C1,L0,L,L1,L2,L3`）。
//! - 本节规则由**用户逐条拍板**（2026-09 平键任务），与反解报告冲突处一律以用户口径为准：
//!
//! ### 用户已定口径（写死在代码里，勿自行"改正"）
//!
//! 1. **1097 键中央 d0 孔**：**通孔**（用户 2026-09-25 裁定：螺钉穿过键身；源图/旧实现拿轴上
//!    孔深 `L0` 当孔底，多档 `L0>h` 会画出键外）——`L0` 数据保留但**不参与键零件几何**；
//!    俯视小径 3/4 圈：源图画在 `4虚线层`，本库改到 **`2细线层`（细实线）**；
//!    其余一律照源图（**不**按 GB/T 4459.1 大改）。⚠️
//! 2. **1097 主视图剖切**：右半 x>0 全剖（材料打剖面线），左半为外形，左端孔用虚线。
//!    **分界线（用户 2026-09 画法修正）**：改用规范 **45° 断裂折线**、画在 **`2细线层`**（细实线）；
//!    不再照抄源图那条 `1轮廓实线层` 的“竖线 + 端部小台阶”（手工痕迹）。
//!    推广规则与形态见 [`BREAK_GAP`]/[`BREAK_AMP`] 与 `break_1097()`。
//! 3. **剖面线间距全局 3.0 mm**（与齿轮/花键/轴生成器统一；**不**照模板的 0.794/3.175）。
//! 4. **1096 表第三列就是 `L`**（源图 PNG 表头"L/2提示"是 OCR 误读）；L 取值按
//!    [`KEY_1096_L_SERIES`] + `L < 10b` 校验，档位默认 L 取表内该档值（PNG 第三列）。
//! 5. **1096 倒角按 b 分档、默认取该档范围下限**（用户 2026-09 新定案）：档位 = GB/T 1096
//!    表 1 `s` 行（见 [`KEY_1096_CHAMFER_RANGES`]），每档 `c_min/c_max` 落在
//!    `tables/partsKey1096{A,B,C}.json` 每行；实画取值由 [`CHAMFER_1096_PICK`] 一处切换
//!    （min/max/mid，默认 min）。与 1097 的 `C` 取表内下限同口径。
//!    ⚠️ specimen（b=2）按新口径 = **0.16**，模板 DXF 画的是 0.2（在该档 0.16~0.25 内，
//!    属模板取中间值）→ **已知 0.04mm 有意偏差**（举证页偏差④），**不要**为对齐模板改回 0.2。
//! 6. **1097 的固定方式 = 螺钉把键固定在轴上（用户 2026-09-25 更正）**：轴上是 2 个**固定螺钉"
//!    "螺纹孔**（d0×L0，自键槽底向下，118° 钻尖）；键上对应 d1 通孔 + D×h1 沉孔（螺钉 GB/T 822/65）。
//!    **“起键螺孔”是 GB/T 1096 附录 A 的概念（普通平键撬出用），不属于 1097**；
//!    旧注释把表列 d0/L0 当“起键螺孔”已作废（L0 真义 = 轴上螺纹孔深，见 `Key1097Row::l0`）。
//! 7. **1097 的 `L1/L2/L3` 由 `L` 查 GB/T 1097-2003 长度系列表派生**（26 档，JSON `length_rows`；
//!    **不插值、不外推**，表外 `L` 明确报错并列出可选系列）。孔位 `±L1/2`、端距 `L3`、断裂线
//!    全部随 `L` 派生；**废除旧行为**：不再沿用 specimen 行固定的 `L1=60/L2=50/L3=20`。
//!    用户模板 `导向平键参数化问题.dxf` 右侧 = L=25 目标（L1=13/L2=12.5/L3=6），逐图元对齐。
//!
//! ### 数据口径备忘
//!
//! - **1096 A 型源图末 4 行 h 印作 32/20/22/25（源图自身误印，已像素级证实）**：
//!   CSV 照录不改，但**库内取标准值 20/22/25/28**；JSON 表里 `h` 用标准值、
//!   `h_printed` 保留印刷值备查（见 `tables/partsKey1096A.json` 的 `source`）。
//! - **1097 的 `L1/L2/L3` 由 `L` 查长度系列表派生**（JSON `length_rows`，26 档；**不插值/不外推**，
//!   表外 `L` 明确报错并列出可选系列）—— 旧行内固定值 `L1=60/L2=50/L3=20`（specimen 行）已废除。
//!   每档默认 `L=100`（specimen 行），b=8/10 违反 `L<10b` 时就近取 70/90；C 取表内范围**下限**。
//! - **1096 的 L 区间已到位**（GB/T 1096-2003 表 1 + 用户素材补 6、8）：
//!   系列见 [`KEY_1096_L_SERIES`]，约束 `L < 10·b`；档位默认 L 必须 ∈ 系列且 < 10b，
//!   不满足时就近取合法值（[`key_1096_legal_default`]，当前 20 档全部合法）。
//! - **1097 的 L 系列已到位**（嘉立创 `new05232.htm` 长度表逐格核对，2026-09）：
//!   系列/派生见 [`key_1097_l_series`] / [`key_1097_length_for`]
//!   （数据源 = `partsKey1097{A,B}.json` 的 `l_series` + `length_rows`，两者一致性有测试护栏）；
//!   沿用标准注③ `L < 10·b`（>400 时按 GB/T 321 的 R20 选取，本库未列）；
//!   `l` 省略/传 0 时用表内该档 L（100；b=8/10 因 100≥10b 就近合法化，见
//!   [`key_1097_legal_default`]/[`key_1097_default_corrections`]）。
//!
//! ## 参数模型（GUI/CLI/HTTP 统一）
//!
//! 族 id 五枚：`key_1096_a` / `key_1096_b` / `key_1096_c` / `key_1097_a` / `key_1097_b`。
//! **自变量 = 键宽 `b` + 型别（编在族 id 里）**；`h` 由数据表派生；`L` 可选。
//! 既有 GUI/CLI 的两参数模型固定为 `<d> <l>`，所以本族 **`d` 槽位承载 `b`**：
//! `OCSMPART key_1096_a 4 8` = b=4、L=8；`l` 传 0/省略 = 用表内该档默认 L。
//! 1097：`OCSMPART key_1097_a 8 25`（L=25 → L1=13/L2=12.5/L3=6，由 L 查长度系列表派生）。
//!
//! ## 视图与基点（照源图坐标，另见各族 `base_hint`）
//!
//! - **1096（三视图）**：主视图 `L×h`（原点 = 左端 × 底面；A 两端 45° 切角、
//!   C 左端切角、B 无；y=c 与 y=h-c 两条倒角棱线）；俯视图 `L×b`（原点 = 左端 × 对称轴；
//!   A 双圆头 R=b/2、B 矩形、C 左圆右方；内缩 c 的内轮廓；中心线两端各伸 3）；
//!   剖视图 `b×h` 八边形（四角切 c）+ ANSI31 剖面线（**无中心线**，照源图）。
//! - **1097（两视图，模板无剖视图 → 不画）**：主视图 = 右半剖 + 左半外形（见口径 2）；
//!   俯视图 A 跑道形 R=b/2 / B 矩形 + 全部孔圈；中心线伸出 3。
//! - **不生成任何尺寸标注**；只用 OCSM 五层。
//!
//! ## OCS 镜像坑（1097 特有）
//!
//! 源图左半图元大量用镜像（`extrusion=(0,0,-1)`，DXF 存 OCS 坐标）。本库按报告
//! §3 的 **WCS 坐标**生成完整对称件，绝不把"镜像对"当重复实体。

use crate::partgen::{GenPart, PartMeta};
use crate::partgen_kit::{
    arc, circle, hatch_ansi31_scaled, line, polyline, trim, views_json, ANSI31_SPACING_MM,
    LAYER_CENTER, LAYER_HIDDEN, LAYER_MAIN, LAYER_THIN,
};
use ocs_plugin_api::host::acadrust::entities::EntityType;

// ══════════════════════════════════════════════════════════════════════════
// 常量（用户口径与源图推导）
// ══════════════════════════════════════════════════════════════════════════

/// GB/T 1096-2003 表 1 `s`（倒角或倒圆）= **按 b 分档的范围**（`(b_lo, b_hi, 范围原文)`）。
/// 每行的 `c_min/c_max` 存在 JSON（`partsKey1096{A,B,C}.json`）；这里是档位边界与
/// 可读范围原文的唯一声明，实画取值策略见 [`CHAMFER_1096_PICK`]。
pub const KEY_1096_CHAMFER_RANGES: &[(f64, f64, &str)] = &[
    (2.0, 4.0, "0.16~0.25"),
    (5.0, 8.0, "0.25~0.40"),
    (10.0, 18.0, "0.40~0.60"),
    (20.0, 32.0, "0.60~0.80"),
    (36.0, 50.0, "1.00~1.20"),
    (56.0, 70.0, "1.60~2.00"),
    (80.0, 100.0, "2.50~3.00"),
];

/// 1096 倒角实画取值策略。
///
/// **用户 2026-09 定案：取该档范围下限**（依据：1097 的 C 表给 `0.25~0.4`、图面就画下限
/// 0.25 → 两族同口径）。将来要改上限/中间值，**只改 [`CHAMFER_1096_PICK`] 一处**。
///
/// `Max`/`Mid` 当前不被生产代码构造（默认 `Min`），仅在测试里验证可切换性；
/// 非测试构建下允许 dead_code，勿删（它们是“一处切换”的入口）。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChamferPick {
    /// 取范围下限（当前默认）
    Min,
    /// 取范围上限
    Max,
    /// 取范围内中点
    Mid,
}

/// 倒角实画取值策略（默认 `Min` = 该档范围下限，与 1097 的 C 取下限同口径）。
pub const CHAMFER_1096_PICK: ChamferPick = ChamferPick::Min;

/// 按指定策略从档位范围取实画倒角值（测试可直接传 `Max`/`Mid` 验证可切换）。
pub fn chamfer_1096_from_range_pick(c_min: f64, c_max: f64, pick: ChamferPick) -> f64 {
    match pick {
        ChamferPick::Min => c_min,
        ChamferPick::Max => c_max,
        ChamferPick::Mid => (c_min + c_max) / 2.0,
    }
}

/// 按当前 [`CHAMFER_1096_PICK`] 取实画倒角值。
pub fn chamfer_1096_from_range(c_min: f64, c_max: f64) -> f64 {
    chamfer_1096_from_range_pick(c_min, c_max, CHAMFER_1096_PICK)
}

/// 1096 数据行 → 实画倒角 `c`（默认该档范围下限）。
pub fn chamfer_1096(row: &Key1096Row) -> f64 {
    // 数据自检：行的 b 必须落在 [`KEY_1096_CHAMFER_RANGES`] 的某一档（JSON 与档位表漂移时 debug 构建报错）。
    debug_assert!(
        chamfer_1096_range_for(row.b).is_some(),
        "{}",
        crate::i18n::t_fmt("cmd.parts.err.corner_b", &[("b", &trim(row.b))])
    );
    chamfer_1096_from_range(row.c_min, row.c_max)
}

/// 某 `b` 的倒角档 `(b_lo, b_hi, 范围原文)`；不在任何档（例：b=9、b=55）→ `None`。
pub fn chamfer_1096_range_for(b: f64) -> Option<(f64, f64, &'static str)> {
    KEY_1096_CHAMFER_RANGES
        .iter()
        .copied()
        .find(|(lo, hi, _)| b >= *lo - 1e-9 && b <= *hi + 1e-9)
}

/// GB/T 1096-2003 表 1 长度 L 标准系列。
///
/// `6、8` 依据用户素材补充（素材 PNG 第三列含 6、8，DXF specimen 就是 L=6；
/// 在线来源的 L 行自 10 起），其余照表 1。`L > 500` 应改按 GB/T 321 R20
/// 选取（本系列最大 400，暂不涉及）。
pub const KEY_1096_L_SERIES: &[f64] = &[
    6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 25.0, 28.0, 32.0, 36.0, 40.0, 45.0,
    50.0, 56.0, 63.0, 70.0, 80.0, 90.0, 100.0, 110.0, 125.0, 140.0, 160.0, 180.0, 200.0, 220.0,
    250.0, 280.0, 320.0, 360.0, 400.0,
];

/// 1097 断裂分界线的 x 基准：距左侧 d1 沉孔外缘（D/2）的间隙。
/// **定案（2026-09 用户画法修正）**：断裂线位于左侧 d1 孔（沉孔外缘）与中央 d0 孔之间，
/// 与沉孔外缘留 [`BREAK_GAP`]；例：specimen L=100（查表 L1=60、D=6）→ x = −(30−3−1) = −26，
/// 短键 L=25（查表 L1=13）→ x = −(6.5−3−1) = −2.5。**L1 由 L 查表派生**，不是行内固定值。
///（原注释称“按 specimen 推导的临时规则”，现按用户定案升格为正式推广规则。）
const BREAK_GAP: f64 = 1.0;

/// 45° 断裂折线的振幅（mm）：4 段斜线以 xb 为中心摆动（xb→xb+a→xb→xb−a→xb），
/// 每段 `|dx| = |dy| = BREAK_AMP`（严格 45°）。取 0.5 使最左峰值距沉孔外缘仍有
/// `BREAK_GAP − BREAK_AMP = 0.5mm` 净距，不压到左侧孔的虚线。
const BREAK_AMP: f64 = 0.5;

/// 螺纹小径简化系数：0.85 × d0/2（报告 §3 实测）。
const MINOR_THREAD_FACTOR: f64 = 0.85;

/// 全局剖面线间距 3.0 mm（用户口径 3）→ ANSI31 pattern_scale。
fn hatch_scale_3mm() -> f64 {
    3.0 / ANSI31_SPACING_MM
}

// ══════════════════════════════════════════════════════════════════════════
// 数据表
// ══════════════════════════════════════════════════════════════════════════

/// JSON 表外壳（比 `partgen_kit::Table` 多 `source`/`note`，供手册与测试引用）。
#[derive(Debug, Clone, serde::Deserialize)]
struct KeyTable<T> {
    #[allow(dead_code)]
    family: String,
    #[allow(dead_code)]
    code: String,
    #[allow(dead_code)]
    iso: String,
    /// 数据来源与口径批注（源图误印等）。
    #[serde(default)]
    #[allow(dead_code)]
    source: String,
    /// 表注（L 系列补充说明等）。
    #[serde(default)]
    #[allow(dead_code)]
    note: String,
    /// 长度 L 标准系列（1097 两表的 `l_series`；1096 用代码常量 [`KEY_1096_L_SERIES`]）。
    #[serde(default)]
    l_series: Vec<f64>,
    /// **1097 专用**：L → (L1, L2, L3) 长度系列表（26 档；1096 无此表 → 空）。
    #[serde(default)]
    length_rows: Vec<Key1097LengthRow>,
    rows: Vec<T>,
}

impl<T: serde::de::DeserializeOwned> KeyTable<T> {
    fn parse(json: &str) -> Self {
        serde_json::from_str(json).expect(&crate::i18n::t("cmd.parts.err.json_parse_keys"))
    }
}

/// GB/T 1096-2003 一行（`O. b h L`）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Key1096Row {
    /// 键宽 b（本族的规格自变量）。
    pub b: f64,
    /// 键高 h（A 型末 4 行已用标准值 20/22/25/28；印刷值见 `h_printed`）。
    pub h: f64,
    /// 源图第三列 = 该档默认 L（"L/2提示"是 OCR 误读）。
    pub l: f64,
    /// 倒角范围下限（GB/T 1096 表 1 `s`，按 b 分档；**实画值默认取它**）。
    pub c_min: f64,
    /// 倒角范围上限（实画改取上限/中点的切换点见 [`CHAMFER_1096_PICK`]）。
    pub c_max: f64,
    /// **仅 A 型**：源图印刷的 h（末 4 行为 32/20/22/25，疑源图误印）；B/C 型无。
    #[serde(default)]
    pub h_printed: Option<f64>,
}

/// GB/T 1097-2003 一行（`O. b h C h1 d0 d1 D C1 L0 L L1 L2 L3`）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Key1097Row {
    /// 键宽 b（规格自变量）。
    pub b: f64,
    /// 键高 h。
    pub h: f64,
    /// 倒角 C（表内给范围，库内取**下限**，照源图 specimen）。
    pub c: f64,
    /// 表内 C 范围原文（留档）。
    #[serde(default)]
    #[allow(dead_code)]
    pub c_range: String,
    /// 沉孔深 h1。
    pub h1: f64,
    /// 固定螺钉螺纹公称 d0（**打在轴上**的螺纹孔；键上对应 d1 通孔 + D×h1 沉孔）。
    pub d0: f64,
    /// 键上通孔直径 d1（固定螺钉过孔）。
    pub d1: f64,
    /// 键上沉孔直径 D。
    #[serde(rename = "D")]
    pub d_sink: f64,
    /// 键上孔口 120° 锪锥深度 C1。
    pub c1: f64,
    /// **轴上固定螺纹孔深度 L0**（自键槽底/轴表面向下量；不是键上孔深）。
    /// ⚠️ **不参与键零件几何**（用户 2026-09-25 裁定：键中央 d0 孔按通孔画，螺钉穿过键身）；
    /// 数据保留备查，禁止再拿它当键上孔底。
    #[serde(rename = "L0")]
    pub l0: f64,
    /// 该档默认长度 L（specimen 行 = 100；b=8/10 不合法 → 就近合法化）。
    pub l: f64,
    /// 固定用螺钉（GB/T 822 或 GB/T 65，表内 `d×L4` 列）——**当前画法不画出**，仅留档。
    #[serde(default)]
    #[allow(dead_code)]
    pub screw: String,
}

/// GB/T 1097-2003 长度系列表一行（「L 与 L1、L2、L3 的对应长度系列」，26 档）。
///
/// **L1/L2/L3 是派生量**：由 [`key_1097_length_for`] 按 `L` 查表；`L` 不在表内 → 报错，
/// **不插值、不外推**。孔位（±L1/2）、端距（L3=(L−L1)/2）与断裂线都由它派生。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Key1097LengthRow {
    /// 键长 L（标准系列 25…450）。
    pub l: f64,
    /// 两端 d1 孔中心距 L1。
    pub l1: f64,
    /// 键中心 → 端面 L2（= L/2）。
    pub l2: f64,
    /// d1 孔中心 → 端面 L3（= (L−L1)/2）。
    pub l3: f64,
}

fn table_1096(ty: KeyType) -> &'static KeyTable<Key1096Row> {
    macro_rules! once {
        ($json:literal) => {{
            static T: std::sync::OnceLock<KeyTable<Key1096Row>> = std::sync::OnceLock::new();
            T.get_or_init(|| KeyTable::parse(include_str!($json)))
        }};
    }
    match ty {
        KeyType::A => once!("tables/partsKey1096A.json"),
        KeyType::B => once!("tables/partsKey1096B.json"),
        KeyType::C => once!("tables/partsKey1096C.json"),
    }
}

fn table_1097(ty: KeyType) -> &'static KeyTable<Key1097Row> {
    macro_rules! once {
        ($json:literal) => {{
            static T: std::sync::OnceLock<KeyTable<Key1097Row>> = std::sync::OnceLock::new();
            T.get_or_init(|| KeyTable::parse(include_str!($json)))
        }};
    }
    match ty {
        KeyType::A => once!("tables/partsKey1097A.json"),
        KeyType::B => once!("tables/partsKey1097B.json"),
        KeyType::C => unreachable!("{}", crate::i18n::t("cmd.parts.err.key1097_no_c")),
    }
}

fn row_1096(ty: KeyType, b: f64) -> Option<&'static Key1096Row> {
    table_1096(ty).rows.iter().find(|r| (r.b - b).abs() < 1e-9)
}

fn row_1097(ty: KeyType, b: f64) -> Option<&'static Key1097Row> {
    table_1097(ty).rows.iter().find(|r| (r.b - b).abs() < 1e-9)
}

/// 1097 行查询（轴生成器轴槽侧的校验/画法用）：按 b 取 A/B 型表行；表外 → `None`。
pub fn key_1097_row(ty: KeyType, b: f64) -> Option<&'static Key1097Row> {
    row_1097(ty, b)
}

// ══════════════════════════════════════════════════════════════════════════
// 族 / 型别 / 视图注册
// ══════════════════════════════════════════════════════════════════════════

/// 平键型别（1096 三型；1097 只有 A/B）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    /// 圆头（双圆头）
    A,
    /// 方头（双平头）
    B,
    /// 单圆头（左圆右方）
    C,
}

/// 平键标准族。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStandard {
    /// GB/T 1096-2003 普通平键（三视图）
    Gb1096,
    /// GB/T 1097-2003 导向平键（两视图，主视图为右半剖 + 左半外形）
    Gb1097,
}

/// 族 id → (标准, 型别)。
pub fn family_parts(family: &str) -> Option<(KeyStandard, KeyType)> {
    match family {
        "key_1096_a" => Some((KeyStandard::Gb1096, KeyType::A)),
        "key_1096_b" => Some((KeyStandard::Gb1096, KeyType::B)),
        "key_1096_c" => Some((KeyStandard::Gb1096, KeyType::C)),
        "key_1097_a" => Some((KeyStandard::Gb1097, KeyType::A)),
        "key_1097_b" => Some((KeyStandard::Gb1097, KeyType::B)),
        _ => None,
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 键型样式表（表驱动；键槽段 GUI 下拉 + DSL/校验的唯一元数据源）
// ══════════════════════════════════════════════════════════════════════════

/// 键型样式：**键型 → 画法（key_type+guided）/ 所属标准 / 可用位置 / 可用选项**。
///
/// 用户 2026-09-25 收尾口径：键槽段的「键型 + 导向复选框」合并为一个键型下拉；
/// 本表是唯一元数据源（经 `/api/parts` 的顶层 `key_styles` 下发给 GUI 渲染）。
/// **以后加「楔形平键」等新键型：本表加一行 + `shaft.rs` 按 `key_type`/`guided` 补画法分支**。
#[derive(Debug, Clone, Copy)]
pub struct KeyStyleSpec {
    /// 稳定 id：GUI 下拉 value + 推荐 DSL 合并记号（例 `导向A`；普通键仍用 `A`/`B`/`C`）。
    pub id: &'static str,
    /// 额外 DSL 别名（大小写不敏感；旧开关 `KEY A 36 导向` 不在此列，由 `guided` 继续兼容）。
    pub aliases: &'static [&'static str],
    /// 界面名（用户口径：`普通平键A型` / `导向平键A型`）。
    pub label: &'static str,
    /// 平键族型别（画法分支）。
    pub key_type: KeyType,
    /// 是否 GB/T 1097 导向平键。
    pub guided: bool,
    /// 所属标准（GUI 信息行显示）。
    pub standard: &'static str,
    /// 允许的槽位置（`"mid"` / `"end"`；表里没有的 → 组装/校验明确报错）。
    pub places: &'static [&'static str],
    /// 是否支持双槽（可用选项）。
    pub allow_double: bool,
    /// 位置不合法时的指路文案（拼在报错尾部；普通键不会用到）。
    pub place_hint: &'static str,
}

/// 键型样式表（顺序 = GUI 下拉顺序；同时被 `key_styles_json()` 下发给 GUI）。
pub const KEY_STYLES: &[KeyStyleSpec] = &[
    KeyStyleSpec {
        id: "A",
        aliases: &[],
        label: "普通平键A型",
        key_type: KeyType::A,
        guided: false,
        standard: "GB/T 1096-2003",
        places: &["mid", "end"],
        allow_double: true,
        place_hint: "",
    },
    KeyStyleSpec {
        id: "B",
        aliases: &[],
        label: "普通平键B型",
        key_type: KeyType::B,
        guided: false,
        standard: "GB/T 1096-2003",
        places: &["mid", "end"],
        allow_double: true,
        place_hint: "",
    },
    KeyStyleSpec {
        id: "C",
        aliases: &[],
        label: "普通平键C型",
        key_type: KeyType::C,
        guided: false,
        standard: "GB/T 1096-2003",
        places: &["mid", "end"],
        allow_double: true,
        place_hint: "",
    },
    KeyStyleSpec {
        id: "导向A",
        aliases: &["導向A", "GUIDED_A"],
        label: "导向平键A型",
        key_type: KeyType::A,
        guided: true,
        standard: "GB/T 1097-2003",
        places: &["mid"],
        allow_double: false,
        place_hint: "固定键固定在轴上；端置是普通平键 KEY 的画法",
    },
    KeyStyleSpec {
        id: "导向B",
        aliases: &["導向B", "GUIDED_B"],
        label: "导向平键B型",
        key_type: KeyType::B,
        guided: true,
        standard: "GB/T 1097-2003",
        places: &["mid"],
        allow_double: false,
        place_hint: "固定键固定在轴上；端置是普通平键 KEY 的画法",
    },
];

/// 键型记号（id/别名）→ 样式（大小写不敏感；含合并记号 `导向A`/`GUIDED_A`）。
pub fn key_style_by_token(text: &str) -> Option<&'static KeyStyleSpec> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    KEY_STYLES.iter().find(|s| {
        s.id.eq_ignore_ascii_case(t) || s.aliases.iter().any(|a| a.eq_ignore_ascii_case(t))
    })
}

/// (平键族型别, 是否导向) → 样式；无此组合（如导向 C）→ `None`。
pub fn key_style_of(ty: KeyType, guided: bool) -> Option<&'static KeyStyleSpec> {
    KEY_STYLES
        .iter()
        .find(|s| s.key_type == ty && s.guided == guided)
}

/// 样式表 JSON（顶层 `key_styles` 下发；GUI 键型下拉/约束/信息行的唯一来源）。
pub fn key_styles_json() -> serde_json::Value {
    serde_json::Value::Array(
        KEY_STYLES
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "label": s.label,
                    "kind": match s.key_type {
                        KeyType::A => "A",
                        KeyType::B => "B",
                        KeyType::C => "C",
                    },
                    "guided": s.guided,
                    "standard": s.standard,
                    "places": s.places,
                    "allow_double": s.allow_double,
                    "place_hint": s.place_hint,
                })
            })
            .collect(),
    )
}

/// 型别的中文后缀（族名/规格文本用）。
#[allow(dead_code)]
fn type_label(ty: KeyType) -> &'static str {
    match ty {
        KeyType::A => "A型（双圆头）",
        KeyType::B => "B型（双平头）",
        KeyType::C => "C型（单圆头）",
    }
}

/// 本模块声明的视图（`partgen_more::family_views` 会先问这里）。
pub fn family_views(family: &str) -> Vec<&'static str> {
    match family_parts(family) {
        // 1096 三视图（源图主/俯/剖三个 DXF）
        Some((KeyStandard::Gb1096, _)) => vec!["main", "top", "section"],
        // 1097 只有主视图 + 俯视图（模板没有剖视图文件 → 不画，报告 §6-A1）
        Some((KeyStandard::Gb1097, _)) => vec!["main", "top"],
        None => Vec::new(),
    }
}

/// 1096 三型的族条目（同一尺寸表，型别只改画法/名称，故共享一份 sizes 组装）。
fn sizes_1096(ty: KeyType) -> Vec<serde_json::Value> {
    table_1096(ty)
        .rows
        .iter()
        .map(|r| {
            let allowed = key_1096_l_allowed(r.b);
            let def = key_1096_legal_default(r);
            let mut lengths = vec![def];
            lengths.extend(allowed.iter().copied().filter(|v| (*v - def).abs() > 1e-9));
            serde_json::json!({
                "d": r.b,
                "b": r.b,
                "h": r.h,
                "label": format!("b={}×h={}", trim(r.b), trim(r.h)),
                "pitch": 0.0,
                "l_min": allowed.first().copied().unwrap_or(def),
                "l_max": allowed.last().copied().unwrap_or(def),
                "lengths": lengths,
                "extra": format!(
                    "h={}；c={}（按 b 分档取范围下限）；默认 L={}",
                    trim(r.h), trim(chamfer_1096(r)), trim(def)),
            })
        })
        .collect()
}

/// 1097 两型的族条目。
fn sizes_1097(ty: KeyType) -> Vec<serde_json::Value> {
    table_1097(ty)
        .rows
        .iter()
        .map(|r| {
            let allowed = key_1097_l_allowed(r.b);
            let def = key_1097_legal_default(r);
            let mut lengths = vec![def];
            lengths.extend(allowed.iter().copied().filter(|v| (*v - def).abs() > 1e-9));
            serde_json::json!({
                "d": r.b,
                "b": r.b,
                "h": r.h,
                "label": format!("b={}（h={}）", trim(r.b), trim(r.h)),
                "pitch": 0.0,
                "l_min": allowed.first().copied().unwrap_or(def),
                "l_max": allowed.last().copied().unwrap_or(def),
                "lengths": lengths,
                "d0": r.d0,
                "l0": r.l0,
                "extra": format!(
                    "h={}；轴上固定螺纹孔 M{}×{}（L0，自槽底向下）、键上 D{}×h{} 沉孔（d1 {}）；L1/L2/L3 由 L 查 GB/T 1097 长度系列表",
                    trim(r.h), trim(r.d0), trim(r.l0), trim(r.d_sink), trim(r.h1), trim(r.d1)),
            })
        })
        .collect()
}

/// 1096 的型别下拉（parts_gui.html 按 `type_group` 渲染）。
fn type_group_1096() -> serde_json::Value {
    serde_json::json!([
        { "id": "key_1096_a", "name": "A型（双圆头）" },
        { "id": "key_1096_b", "name": "B型（双平头）" },
        { "id": "key_1096_c", "name": "C型（单圆头）" },
    ])
}

/// 1097 的型别下拉。
fn type_group_1097() -> serde_json::Value {
    serde_json::json!([
        { "id": "key_1097_a", "name": "A型（双圆头）" },
        { "id": "key_1097_b", "name": "B型（双平头）" },
    ])
}

/// 1096 族条目的 notes（倒角分档范围、h 误印、L 系列口径）。
fn notes_1096() -> Vec<String> {
    // 倒角档位从唯一数据源 `KEY_1096_CHAMFER_RANGES` 拼出；实画取值默认该档下限。
    let ranges: Vec<String> = KEY_1096_CHAMFER_RANGES
        .iter()
        .map(|(lo, hi, r)| format!("b={}~{} {}", trim(*lo), trim(*hi), r))
        .collect();
    let mut notes = vec![
        format!(
            "倒角 c 按 b 分档取**范围下限**（用户 2026-09 定案；档位与范围来自 GB/T 1096 表 1 s 行，\
             每行 c_min/c_max 在 JSON，实画取值切换点 = CHAMFER_1096_PICK）：{}。\
             注：specimen b=2 → 0.16；模板 DXF 画 0.2（在 0.16~0.25 内的中间值），属已知有意偏差。",
            ranges.join("；")
        ),
        "L：GB/T 1096-2003 表 1 系列（6、8 依据用户素材补充；在线 L 行自 10 起）；\
         约束 L < 10·b；下拉里列该 b 的全部合法 L，默认值 = 源图第三列。"
            .to_string(),
    ];
    if table_1096(KeyType::A).rows.iter().any(|r| r.h_printed.is_some()) {
        notes.push(
            "A 型源图末 4 行 h 印作 32/20/22/25，疑源图误印（在线表与 B/C 型均为 20/22/25/28）；\
             库内取标准值 20/22/25/28，印刷值仅留档。"
                .to_string(),
        );
    }
    let fixed = key_1096_default_corrections();
    if !fixed.is_empty() {
        notes.push(format!(
            "以下档位默认 L 不在合法集内，已就近修正：{}",
            fixed
                .iter()
                .map(|(b, was, now)| format!("b={}：{}→{}", trim(*b), trim(*was), trim(*now)))
                .collect::<Vec<_>>()
                .join("；")
        ));
    }
    notes
}

/// 本组族的目录 JSON 片段（键 = 族 id）——`partgen::catalog_json` 会合并。
pub fn families_json() -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    let hint =
        "基点 = 左端面对称轴（俯视图）/ 左端面×底面（主视图）/ 截面左下角（剖视图）；d 槽位承载 b";
    let note_1097 = {
        let mut v = vec![
            "1097 的固定方式 = 螺钉把键固定在轴上（用户 2026-09-25 更正）：轴上是 2 个固定螺钉螺纹孔\
             （d0×L0，自键槽底向下，118° 钻尖）；键上对应 d1 通孔 + D×h1 沉孔。\
             “起键螺孔”是 GB/T 1096 附录 A 的概念（普通平键撬出用），不属于 1097。"
                .to_string(),
            "主视图：右半剖 + 左半外形；分界线 = 2细线层 细实线 + 规范 45° 断裂折线\
             （用户 2026-09 画法修正；源图粗线竖线+端部台阶是手工痕迹，不再照抄）。"
                .to_string(),
            "键中央 d0 孔为通孔（用户 2026-09-25 裁定：螺钉穿过键身）；轴上 L0 不入键零件几何。\
             螺纹小径 3/4 圈在 2细线层（用户拍板；源图在 4虚线层）。"
                .to_string(),
            "L：GB/T 1097-2003 标准系列 25、28、…、450（嘉立创 new05232.htm 长度表 26 档），\
             约束 L < 10·b（注③；>400 按 GB/T 321 R20 选取）；下拉列该 b 的全部合法 L。"
                .to_string(),
            "**L1/L2/L3 由 L 查 GB/T 1097-2003 长度系列表**（26 档，不插值/不外推；表外 L 明确报错）；\
             孔位 ±L1/2、端距 L3 与断裂线随 L 派生，不再用 specimen 固定值。"
                .to_string(),
        ];
        // 源图 specimen 的 C 取表内下限；留档一行。
        if let Some(r) = table_1097(KeyType::A).rows.first() {
            v.push(format!(
                "C 取表内范围下限（b={}：{}）；剖面线间距统一 3.0mm。",
                trim(r.b), r.c_range
            ));
        }
        let fixed = key_1097_default_corrections();
        if !fixed.is_empty() {
            v.push(format!(
                "表内 L=100 在以下档位违反 L<10b，已就近修正：{}",
                fixed
                    .iter()
                    .map(|(b, was, now)| format!("b={}：{}→{}", trim(*b), trim(*was), trim(*now)))
                    .collect::<Vec<_>>()
                    .join("；")
            ));
        }
        v
    };

    for ty in [KeyType::A, KeyType::B, KeyType::C] {
        let id = match ty {
            KeyType::A => "key_1096_a",
            KeyType::B => "key_1096_b",
            KeyType::C => "key_1096_c",
        };
        m.insert(
            id.into(),
            serde_json::json!({
                "id": id,
                "name": format!("{} {}", crate::i18n::t_data("圆头普通平键"), crate::i18n::t_data(if ty == KeyType::A { "A型" } else if ty == KeyType::B { "B型" } else { "C型" })),
                "code": "GB/T 1096-2003",
                "iso": "—",
                "implemented": true,
                "views": views_json(id),
                "sizes": sizes_1096(ty),
                "len_label": "长度 L",
                "base_hint": hint,
                "tree_dir": "零件库/键/平键",
                "type_group": type_group_1096(),
                "notes": notes_1096(),
                "source": table_1096(ty).source,
                // 轴径 → 键宽选型区间（1979 d 列；仅初选；轴生成器平键面板用）。
                "shaft_ranges": key_1096_shaft_ranges()
                    .iter()
                    .map(|(lo, hi, incl, b)| serde_json::json!({
                        "d_lo": lo, "d_hi": hi, "lo_inclusive": incl, "b": b
                    }))
                    .collect::<Vec<_>>(),
            }),
        );
    }
    for ty in [KeyType::A, KeyType::B] {
        let id = match ty {
            KeyType::A => "key_1097_a",
            KeyType::B => "key_1097_b",
            KeyType::C => unreachable!("{}", crate::i18n::t("cmd.parts.err.key1097_no_c")),
        };
        m.insert(
            id.into(),
            serde_json::json!({
                "id": id,
                "name": format!("{} {}", crate::i18n::t_data("导向平键"), crate::i18n::t_data(if ty == KeyType::A { "A型" } else { "B型" })),
                "code": "GB/T 1097-2003",
                "iso": "—",
                "implemented": true,
                "views": views_json(id),
                "sizes": sizes_1097(ty),
                "length_rows": table_1097(ty).length_rows.iter().map(|r| serde_json::json!({
                    "l": r.l, "l1": r.l1, "l2": r.l2, "l3": r.l3
                })).collect::<Vec<_>>(),
                "len_label": "长度 L",
                "base_hint": hint,
                "tree_dir": "零件库/键/平键",
                "type_group": type_group_1097(),
                "notes": note_1097.clone(),
                "source": table_1097(ty).source,
            }),
        );
    }
    m
}

// ══════════════════════════════════════════════════════════════════════════
// 长度口径与校验
// ══════════════════════════════════════════════════════════════════════════

/// 1096 某 b 允许的 L（L 系列 ∩ `L < 10·b`），升序。
pub fn key_1096_l_allowed(b: f64) -> Vec<f64> {
    KEY_1096_L_SERIES
        .iter()
        .copied()
        .filter(|l| *l < 10.0 * b - 1e-9)
        .collect()
}

/// 1096 某 b 的合法 L 区间 `(min, max)`；b 不在表内 → `None`。
pub fn key_1096_l_range(b: f64) -> Option<(f64, f64)> {
    if row_1096(KeyType::A, b).is_none() {
        return None;
    }
    let allowed = key_1096_l_allowed(b);
    Some((*allowed.first()?, *allowed.last()?))
}

/// 档位默认 L（源图第三列）；若它不在合法集内则就近取合法值（用户 2026-09 口径）。
///
/// 当前 20 档默认值全部合法，`key_1096_default_corrections()` 为空。
pub fn key_1096_legal_default(row: &Key1096Row) -> f64 {
    let allowed = key_1096_l_allowed(row.b);
    if allowed.iter().any(|v| (*v - row.l).abs() < 1e-9) {
        return row.l;
    }
    *allowed
        .iter()
        .min_by(|a, b| {
            (*a - row.l)
                .abs()
                .partial_cmp(&(*b - row.l).abs())
                .unwrap()
        })
        .unwrap_or(&row.l)
}

/// (b, 印刷默认 L, 就近合法值) —— 供测试/报告列出被修正的档位；当前应为空。
pub fn key_1096_default_corrections() -> Vec<(f64, f64, f64)> {
    let mut out = Vec::new();
    for ty in [KeyType::A, KeyType::B, KeyType::C] {
        for r in &table_1096(ty).rows {
            let fixed = key_1096_legal_default(r);
            if (fixed - r.l).abs() > 1e-9 {
                out.push((r.b, r.l, fixed));
            }
        }
    }
    out
}

/// 1096 长度校验：必须是 L 系列值，且 `L < 10·b`。
pub fn check_length_1096(b: f64, l: f64) -> Result<(), String> {
    let Some((lo, hi)) = key_1096_l_range(b) else {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.key_no_b",
            &[("table", "GB/T 1096-2003"), ("b", &trim(b))],
        ));
    };
    if !(l > 0.0) || !l.is_finite() {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.key_l_positive",
            &[("std", "1096"), ("b", &trim(b)), ("l", &trim(l))],
        ));
    }
    if l < lo - 1e-9 || l > hi + 1e-9 {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.key_l_range",
            &[
                ("std", "1096"),
                ("b", &trim(b)),
                ("lo", &trim(lo)),
                ("hi", &trim(hi)),
                ("l", &trim(l)),
            ],
        ));
    }
    if !key_1096_l_allowed(b).iter().any(|v| (*v - l).abs() < 1e-9) {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.key_l_series",
            &[("b", &trim(b)), ("l", &trim(l))],
        ));
    }
    Ok(())
}

// ══════════════════════════════════════════════════════════════════════
// 轴径 → 键宽选型（GB/T 1095 的 d 列；**轴槽的 b×h 选型依据**）
// ══════════════════════════════════════════════════════════════════════

/// GB/T 1095-1979「轴的公称直径 d」区间 → 键宽 b（`(d_lo, d_hi, d_lo_inclusive, b)`）。
///
/// **轴槽的 b×h 选型依据**（GB/T 1095 首列 d → b；h 跟 b 走）；
/// 2003 版已取消 d 列，
/// 键槽尺寸（t₁/t₂）数据不依赖它。数据 = `assets/key1096_shaft_ranges.csv`
/// （由已三来源校验的 `review/键槽_GB1095_表_v2.csv` d 列入库）。
const KEY1096_SHAFT_RANGES_CSV: &str = include_str!("../assets/key1096_shaft_ranges.csv");

/// 解析选型表（26 档；生产路径 `key_1096_b_for_shaft` 不返回错误，测试断言正确性）。
pub fn key_1096_shaft_ranges() -> Vec<(f64, f64, bool, f64)> {
    let mut out = Vec::new();
    for line in KEY1096_SHAFT_RANGES_CSV
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .skip(1)
    {
        let v: Vec<&str> = line.split(',').collect();
        if v.len() < 4 {
            continue;
        }
        let num = |i: usize| v[i].trim().parse::<f64>().unwrap_or(f64::NAN);
        out.push((num(0), num(1), v[2].trim() == "1", num(3)));
    }
    out
}

/// 轴径 d → 标准键宽 b（1979 d 列；d 不在区间内 → `None`）。
/// 区间口径：左端仅首行（6~8）含 6（`d_lo_inclusive`）；其余为 `d_lo < d ≤ d_hi`。
pub fn key_1096_b_for_shaft(d: f64) -> Option<f64> {
    key_1096_shaft_ranges().into_iter().find_map(|(lo, hi, incl, b)| {
        let ge = if incl { d >= lo - 1e-9 } else { d > lo + 1e-9 };
        (ge && d <= hi + 1e-9).then_some(b)
    })
}

/// 平键族某型别/键宽 b 的键高 h（b 不在该型别表 → `None`）。
pub fn key_1096_h(ty: KeyType, b: f64) -> Option<f64> {
    row_1096(ty, b).map(|r| r.h)
}

/// 平键族某型别/键宽 b 的该档默认键长 L（源图表内值）。
pub fn key_1096_default_l(ty: KeyType, b: f64) -> Option<f64> {
    row_1096(ty, b).map(key_1096_legal_default)
}

/// 1097 长度系列（数据源 = `partsKey1097{A,B}.json` 的 `l_series`；两表必须一致，测试有护栏）。
pub fn key_1097_l_series() -> &'static [f64] {
    &table_1097(KeyType::A).l_series
}

/// 1097 长度系列表（`L → L1/L2/L3`，26 档；A/B 两表必须一致，测试有护栏）。
pub fn key_1097_length_rows() -> &'static [Key1097LengthRow] {
    &table_1097(KeyType::A).length_rows
}

/// 查表派生：1097 长度 `L` → `(L1, L2, L3)` 行。**不插值、不外推**：
/// `L` 不在 26 档表内（如 25/28 之间的 26、超过 450 等）→ `Err`，错误里列出全部可选 `L`。
pub fn key_1097_length_for(l: f64) -> Result<&'static Key1097LengthRow, String> {
    if let Some(row) = key_1097_length_rows()
        .iter()
        .find(|r| (r.l - l).abs() < 1e-9)
    {
        return Ok(row);
    }
    Err(crate::i18n::t_fmt(
        "cmd.parts.err.key_series_missing",
        &[
            ("l", &trim(l)),
            ("n", &key_1097_length_rows().len().to_string()),
            (
                "list",
                &key_1097_length_rows()
                    .iter()
                    .map(|r| trim(r.l))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        ],
    ))
}

/// 1097 某 b 允许的 L（L 系列 ∩ `L < 10·b`），升序。
///
/// 标准注③：键长应小于 10 倍键宽；>400 时按 GB/T 321 的 R20 系列选取（本库未列）。
pub fn key_1097_l_allowed(b: f64) -> Vec<f64> {
    key_1097_l_series()
        .iter()
        .copied()
        .filter(|l| *l < 10.0 * b - 1e-9)
        .collect()
}

/// 1097 某 b 的合法 L 区间 `(min, max)`；b 不在表内 → `None`。
pub fn key_1097_l_range(b: f64) -> Option<(f64, f64)> {
    if row_1097(KeyType::A, b).is_none() {
        return None;
    }
    let allowed = key_1097_l_allowed(b);
    Some((*allowed.first()?, *allowed.last()?))
}

/// 1097 档位默认 L（表内 L=100；不合法时就近取合法值——b=8/10 会落到 70/90）。
pub fn key_1097_legal_default(row: &Key1097Row) -> f64 {
    let allowed = key_1097_l_allowed(row.b);
    if allowed.iter().any(|v| (*v - row.l).abs() < 1e-9) {
        return row.l;
    }
    *allowed
        .iter()
        .min_by(|a, b| {
            (*a - row.l)
                .abs()
                .partial_cmp(&(*b - row.l).abs())
                .unwrap()
        })
        .unwrap_or(&row.l)
}

/// (b, 表内 L, 就近合法值) —— 供测试/手册列出被修正的档位（b=8→70、b=10→90）。
pub fn key_1097_default_corrections() -> Vec<(f64, f64, f64)> {
    let mut out = Vec::new();
    // A/B 两表逐行相同（系列一致性有测试），只列一份。
    for r in &table_1097(KeyType::A).rows {
        let fixed = key_1097_legal_default(r);
        if (fixed - r.l).abs() > 1e-9 {
            out.push((r.b, r.l, fixed));
        }
    }
    out
}

/// 1097 长度校验 + 派生：`L ∈ 长度系列表`（不插值）且 `L < 10·b`（标准注③）。
///
/// 成功返回派生的 `L1/L2/L3` 行；表外 `L` → `Err`（列出可选系列）。
/// 同时做数据护栏：表内任一行必须满足 `L2=L/2`、`L1+2L3=L`。
pub fn key_1097_length_checked(b: f64, l: f64) -> Result<&'static Key1097LengthRow, String> {
    let Some((lo, hi)) = key_1097_l_range(b) else {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.key_no_b",
            &[("table", "GB/T 1097-2003"), ("b", &trim(b))],
        ));
    };
    if !(l > 0.0) || !l.is_finite() {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.key_l_positive",
            &[("std", "1097"), ("b", &trim(b)), ("l", &trim(l))],
        ));
    }
    let row = key_1097_length_for(l)?;
    if l < lo - 1e-9 || l > hi + 1e-9 {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.key_l_range_1097",
            &[
                ("b", &trim(b)),
                ("lo", &trim(lo)),
                ("hi", &trim(hi)),
                ("l", &trim(l)),
            ],
        ));
    }
    // 尺寸链自检：L2=L/2、L1+2·L3=L（长度表 26 档应恒满足；几何只依赖 L1，L2/L3 供校验/文档）。
    if (row.l2 - row.l / 2.0).abs() > 1e-9 || (row.l1 + 2.0 * row.l3 - row.l).abs() > 1e-9 {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.key_chain",
            &[
                ("l", &trim(row.l)),
                ("l1", &trim(row.l1)),
                ("l2", &trim(row.l2)),
                ("l3", &trim(row.l3)),
            ],
        ));
    }
    Ok(row)
}

/// 1097 长度校验（旧入口，保持返回 `Result<(), String>`）；实现见 [`key_1097_length_checked`]。
pub fn check_length_1097(b: f64, l: f64) -> Result<(), String> {
    key_1097_length_checked(b, l).map(|_| ())
}

// ══════════════════════════════════════════════════════════════════════════
// 生成派发
// ══════════════════════════════════════════════════════════════════════════

/// 本组族的生成派发（`partgen::generate` / `partgen_more::generate` 会先问这里）。
///
/// `d` 槽位承载 **b**（键宽）；`l <= 0` = 用表内该档默认 L。
pub fn generate(family: &str, d: f64, l: f64, view: &str) -> Option<Result<GenPart, String>> {
    let (standard, ty) = family_parts(family)?;
    let allowed = family_views(family);
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
    Some(match standard {
        KeyStandard::Gb1096 => gen_1096(ty, d, l, view),
        KeyStandard::Gb1097 => gen_1097(ty, d, l, view),
    })
}

// ── 1096 普通平键 ─────────────────────────────────────────────────────────

fn gen_1096(ty: KeyType, b: f64, l: f64, view: &str) -> Result<GenPart, String> {
    let row = row_1096(ty, b).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.parts.err.key_no_b",
            &[("table", "GB/T 1096-2003"), ("b", &trim(b))],
        )
    })?;
    let l = if l > 0.0 { l } else { row.l };
    check_length_1096(b, l)?;
    let (h, c) = (row.h, chamfer_1096(row));
    let entities = match view {
        "main" => main_1096(ty, b, h, l, c),
        "top" => top_1096(ty, b, l, c),
        "section" => section_1096(b, h, c),
        other => {
            return Err(crate::i18n::t_fmt(
                "cmd.parts.err.view_absent_bare",
                &[("family", "key_1096"), ("view", other)],
            ))
        }
    };
    let weight = weight_1096(ty, b, h, l);
    Ok(GenPart {
        bbox: bbox_of(&entities),
        entities,
        meta: PartMeta {
            code: "GB/T 1096-2003".into(),
            name: format!("{} {}", crate::i18n::t_data("圆头普通平键"), crate::i18n::t_data(match ty {
                KeyType::A => "A型",
                KeyType::B => "B型",
                KeyType::C => "C型",
            })),
            spec: format!("{}×{}×{}", trim(b), trim(h), trim(l)),
            material: String::new(),
            weight: format!("{weight:.4}"),
        },
    })
}

/// 1096 主视图：`L×h` 带端部 45° 切角（仅圆头端）+ 两条倒角棱线；源图此视图无中心线。
fn main_1096(ty: KeyType, _b: f64, h: f64, l: f64, c: f64) -> Vec<EntityType> {
    // 端点顺序照源图 POLYLINE（A/C 圆头端 c×c 切角；B 方角）。
    let outline: Vec<[f64; 2]> = match ty {
        KeyType::A => vec![
            [0.0, c], [c, 0.0], [l - c, 0.0], [l, c],
            [l, h - c], [l - c, h], [c, h], [0.0, h - c],
        ],
        KeyType::B => vec![
            [0.0, c], [0.0, 0.0], [l, 0.0], [l, c],
            [l, h - c], [l, h], [0.0, h], [0.0, h - c],
        ],
        KeyType::C => vec![
            [0.0, c], [c, 0.0], [l, 0.0], [l, c],
            [l, h - c], [l, h], [c, h], [0.0, h - c],
        ],
    };
    vec![
        polyline(&outline, true, LAYER_MAIN),
        line([0.0, c], [l, c], LAYER_MAIN),
        line([l, h - c], [0.0, h - c], LAYER_MAIN),
    ]
}

/// 1096 俯视图：`L×b` 外形（A 双圆头 R=b/2｜B 双方头｜C 左圆右方）+ 内缩 c 的倒角棱线
/// + 中心线（两端各伸 3，源图实测）。
fn top_1096(ty: KeyType, b: f64, l: f64, c: f64) -> Vec<EntityType> {
    let half = b / 2.0;
    let ri = half - c;
    let mut en = vec![line([-3.0, 0.0], [l + 3.0, 0.0], LAYER_CENTER)];
    match ty {
        KeyType::A => {
            let (cl, cr) = (half, l - half);
            en.push(arc([cl, 0.0], half, 90.0, 270.0, LAYER_MAIN));
            en.push(arc([cr, 0.0], half, 270.0, 90.0, LAYER_MAIN));
            en.push(line([cl, -half], [cr, -half], LAYER_MAIN));
            en.push(line([cr, half], [cl, half], LAYER_MAIN));
            en.push(line([cl, -ri], [cr, -ri], LAYER_MAIN));
            en.push(line([cr, ri], [cl, ri], LAYER_MAIN));
            en.push(arc([cl, 0.0], ri, 90.0, 270.0, LAYER_MAIN));
            en.push(arc([cr, 0.0], ri, 270.0, 90.0, LAYER_MAIN));
        }
        KeyType::B => {
            // 源图 B 型俯视图为 7 条 LINE（非闭合多段线），逐条照抄。
            en.push(line([0.0, -half], [l, -half], LAYER_MAIN));
            en.push(line([l, half], [0.0, half], LAYER_MAIN));
            en.push(line([0.0, -ri], [l, -ri], LAYER_MAIN));
            en.push(line([0.0, ri], [l, ri], LAYER_MAIN));
            en.push(line([l, half], [l, -half], LAYER_MAIN));
            en.push(line([0.0, half], [0.0, -half], LAYER_MAIN));
        }
        KeyType::C => {
            // 左圆右方：左端半圆，右端方角（内轮廓线画到端面）。
            let cl = half;
            en.push(arc([cl, 0.0], half, 90.0, 270.0, LAYER_MAIN));
            en.push(line([cl, -half], [l, -half], LAYER_MAIN));
            en.push(line([l, half], [cl, half], LAYER_MAIN));
            en.push(line([cl, -ri], [l, -ri], LAYER_MAIN));
            en.push(line([cl, ri], [l, ri], LAYER_MAIN));
            en.push(arc([cl, 0.0], ri, 90.0, 270.0, LAYER_MAIN));
            en.push(line([l, half], [l, -half], LAYER_MAIN));
        }
    }
    en
}

/// 1096 剖视图：`b×h` 四角切 c 的八边形 + ANSI31 剖面线（**无中心线**，照源图）。
fn section_1096(b: f64, h: f64, c: f64) -> Vec<EntityType> {
    let verts: Vec<[f64; 2]> = vec![
        [0.0, c], [c, 0.0], [b - c, 0.0], [b, c],
        [b, h - c], [b - c, h], [c, h], [0.0, h - c],
    ];
    vec![
        polyline(&verts, true, LAYER_MAIN),
        hatch_ansi31_scaled(&verts, 0.0, hatch_scale_3mm()),
    ]
}

/// 1096 单件重量估算（钢 7.85 g/cm³；面积按端型取，忽略倒角）。
fn weight_1096(ty: KeyType, b: f64, h: f64, l: f64) -> f64 {
    let area = match ty {
        KeyType::A => (l - b).max(0.0) * b + std::f64::consts::PI * b * b / 4.0,
        KeyType::B => l * b,
        KeyType::C => (l - b / 2.0).max(0.0) * b + std::f64::consts::PI * b * b / 8.0,
    };
    area * h * 7.85e-3 / 1000.0
}

// ── 1097 导向平键 ─────────────────────────────────────────────────────────

fn gen_1097(ty: KeyType, b: f64, l: f64, view: &str) -> Result<GenPart, String> {
    let row = row_1097(ty, b).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.parts.err.key_no_b",
            &[("table", "GB/T 1097-2003"), ("b", &trim(b))],
        )
    })?;
    // `l=0` = 该档默认 L（表内 100；b=8/10 就近合法化为 70/90）。
    let l = if l > 0.0 { l } else { key_1097_legal_default(row) };
    // L → (L1,L2,L3)：查表派生（不插值/不外推）；L 表外/不合法 → 明确报错。
    let length = key_1097_length_checked(b, l)?;
    let entities = match view {
        "main" => main_1097(ty, row, length),
        "top" => top_1097(ty, row, length),
        other => {
            return Err(crate::i18n::t_fmt(
                "cmd.parts.err.view_absent_bare",
                &[("family", "key_1097"), ("view", other)],
            ))
        }
    };
    let weight = weight_1097(ty, row, l);
    Ok(GenPart {
        bbox: bbox_of(&entities),
        entities,
        meta: PartMeta {
            code: "GB/T 1097-2003".into(),
            name: format!("{} {}", crate::i18n::t_data("导向平键"), crate::i18n::t_data(if ty == KeyType::A { "A型" } else { "B型" })),
            spec: format!("{}×{}×{}", trim(row.b), trim(row.h), trim(l)),
            material: String::new(),
            weight: format!("{weight:.4}"),
        },
    })
}

/// 1097 主视图（右半剖 + 左半外形；分界线 = `2细线层` 规范 45° 断裂折线，用户 2026-09 修正）。
///
/// 图元顺序照源图（WCS，已去镜像重复）；坐标详见反解报告 §3.2/§3.4。
/// 1097 主视图断裂分界线（用户 2026-09 定案）：`2细线层` 细实线 + 规范 45° 折线。
///
/// 形态（自底边到顶边）：`(xb,0) → (xb,y0) → [4 段 45° 折线 xb→xb+a→xb→xb−a→xb] → (xb,h)`，
/// 其中 `y0=(h−4a)/2`；两端短竖线接在顶/底边上，中段严格 45°。
/// 推广规则（用户定案）：`xb = −(L1/2 − D/2 − BREAK_GAP)`（位于左 d1 沉孔外缘与中央 d0 孔之间）。
fn break_1097(r: &Key1097Row, l1: f64) -> Vec<[f64; 2]> {
    let xb = -(l1 / 2.0 - r.d_sink / 2.0 - BREAK_GAP);
    let a = BREAK_AMP;
    let y0 = (r.h - 4.0 * a) / 2.0;
    vec![
        [xb, 0.0],
        [xb, y0],
        [xb + a, y0 + a],
        [xb, y0 + 2.0 * a],
        [xb - a, y0 + 3.0 * a],
        [xb, y0 + 4.0 * a],
        [xb, r.h],
    ]
}

fn main_1097(ty: KeyType, r: &Key1097Row, len: &Key1097LengthRow) -> Vec<EntityType> {
    let l = len.l;
    let (h, c) = (r.h, r.c);
    let (xh, d0, d1, d) = (len.l1 / 2.0, r.d0, r.d1, r.d_sink);
    let (h1, c1) = (r.h1, r.c1);
    let rcs = d0 / 2.0 - c1 * 60f64.to_radians().tan(); // 锪锥小端半径
    // 中央 d0 孔：口部 120° 锪锥 + 大径竖线 + 小端竖线（照源图）。
    // **通孔**（用户 2026-09-25 裁定：螺钉穿过键身）——孔壁直达键底面 y=0。
    // ⚠️ `L0` 真义 = 轴上固定螺纹孔深，**不参与键零件几何**（数据保留在表里，勿删）。
    let y0 = 0.0;
    // 断裂分界线：2细线层 + 规范 45° 折线（推广规则 xb = L1/2 − D/2 − BREAK_GAP，见 break_1097）。
    let xb = xh - d / 2.0 - BREAK_GAP;
    let mut en = vec![
        line([0.0, -3.0], [0.0, h + 3.0], LAYER_CENTER),
        line([xh, -3.0], [xh, h + 3.0], LAYER_CENTER),
        line([-xh, -3.0], [-xh, h + 3.0], LAYER_CENTER),
    ];
    en.push(polyline(
        &[[-rcs, y0], [-rcs, h - c1], [0.0, h - c1]],
        false,
        LAYER_MAIN,
    ));
    en.push(polyline(
        &[[-rcs, h - c1], [-d0 / 2.0, h], [-d0 / 2.0, y0]],
        false,
        LAYER_MAIN,
    ));
    // 左端孔：左半为外形 → 近/远两条隐藏线（4虚线层）。
    en.push(polyline(
        &[
            [-(xh - d1 / 2.0), 0.0],
            [-(xh - d1 / 2.0), h - h1],
            [-(xh - d / 2.0), h - h1],
            [-(xh - d / 2.0), h],
        ],
        false,
        LAYER_HIDDEN,
    ));
    // 右端孔：右半剖 → 可见轮廓（1轮廓实线层）。
    en.push(polyline(
        &[
            [xh - d1 / 2.0, 0.0],
            [xh - d1 / 2.0, h - h1],
            [xh - d / 2.0, h - h1],
            [xh - d / 2.0, h],
        ],
        false,
        LAYER_MAIN,
    ));
    en.push(line(
        [xh - d1 / 2.0, h - h1],
        [xh, h - h1],
        LAYER_MAIN,
    ));
    // 外形 + 倒角棱线（左半外形；断裂线止于 -xb）。
    match ty {
        KeyType::A => {
            en.push(polyline(
                &[
                    [0.0, 0.0],
                    [-(l / 2.0 - c), 0.0],
                    [-l / 2.0, c],
                    [-l / 2.0, h - c],
                    [-(l / 2.0 - c), h],
                    [0.0, h],
                ],
                false,
                LAYER_MAIN,
            ));
            en.push(line([-l / 2.0, h - c], [-xb, h - c], LAYER_MAIN));
            en.push(line([-l / 2.0, c], [-xb, c], LAYER_MAIN));
        }
        KeyType::B => {
            en.push(line([-l / 2.0, h - c], [-xb, h - c], LAYER_MAIN));
            en.push(line([-l / 2.0, c], [-xb, c], LAYER_MAIN));
        }
        KeyType::C => unreachable!("{}", crate::i18n::t("cmd.parts.err.key1097_no_c")),
    }
    // 材料剖面多边形（中段 / 右端 / 左中带断裂边界）。
    let mid = vec![
        [d0 / 2.0, 0.0],
        [d0 / 2.0, h],
        [xh - d / 2.0, h],
        [xh - d / 2.0, h - h1],
        [xh - d1 / 2.0, h - h1],
        [xh - d1 / 2.0, 0.0],
    ];
    let right = match ty {
        KeyType::A => vec![
            [xh + d1 / 2.0, 0.0],
            [xh + d1 / 2.0, h - h1],
            [xh + d / 2.0, h - h1],
            [xh + d / 2.0, h],
            [l / 2.0 - c, h],
            [l / 2.0, h - c],
            [l / 2.0, c],
            [l / 2.0 - c, 0.0],
        ],
        KeyType::B => vec![
            [xh + d1 / 2.0, 0.0],
            [xh + d1 / 2.0, h - h1],
            [xh + d / 2.0, h - h1],
            [xh + d / 2.0, h],
            [l / 2.0, h],
            [l / 2.0, 0.0],
        ],
        KeyType::C => unreachable!("{}", crate::i18n::t("cmd.parts.err.key1097_no_c")),
    };
    // 左中材料：左缘 = 45° 断裂折线（细实线，只画线不填充），右缘 = 中央孔壁；
    // 顶/底边已由左半外形多段线覆盖 → 不再重复画闭合粗轮廓（旧实现把三者合成一条 1轮廓实线层 闭合线）。
    let break_pts = break_1097(r, len.l1);
    let mut left_loop = break_pts.clone();
    left_loop.push([-d0 / 2.0, h]);
    left_loop.push([-d0 / 2.0, 0.0]);
    en.push(polyline(&mid, true, LAYER_MAIN));
    en.push(polyline(&right, true, LAYER_MAIN));
    en.push(polyline(&break_pts, false, LAYER_THIN));
    // 中央孔右半 + 左端孔远侧隐藏线 + 右端孔远侧可见轮廓 + 沉孔底（镜像对的 WCS 坐标）。
    en.push(polyline(
        &[[rcs, y0], [rcs, h - c1], [0.0, h - c1]],
        false,
        LAYER_MAIN,
    ));
    en.push(polyline(
        &[[rcs, h - c1], [d0 / 2.0, h], [d0 / 2.0, y0]],
        false,
        LAYER_MAIN,
    ));
    en.push(polyline(
        &[
            [-(xh + d1 / 2.0), 0.0],
            [-(xh + d1 / 2.0), h - h1],
            [-(xh + d / 2.0), h - h1],
            [-(xh + d / 2.0), h],
        ],
        false,
        LAYER_HIDDEN,
    ));
    en.push(polyline(
        &[
            [xh + d1 / 2.0, 0.0],
            [xh + d1 / 2.0, h - h1],
            [xh + d / 2.0, h - h1],
            [xh + d / 2.0, h],
        ],
        false,
        LAYER_MAIN,
    ));
    en.push(line(
        [xh + d1 / 2.0, h - h1],
        [xh, h - h1],
        LAYER_MAIN,
    ));
    match ty {
        KeyType::A => {
            en.push(polyline(
                &[
                    [0.0, 0.0],
                    [l / 2.0 - c, 0.0],
                    [l / 2.0, c],
                    [l / 2.0, h - c],
                    [l / 2.0 - c, h],
                    [0.0, h],
                ],
                false,
                LAYER_MAIN,
            ));
        }
        KeyType::B => {
            en.push(polyline(
                &[[-l / 2.0, 0.0], [-l / 2.0, h], [l / 2.0, h], [l / 2.0, 0.0]],
                true,
                LAYER_MAIN,
            ));
        }
        KeyType::C => unreachable!(),
    }
    // 3 片剖面线（中段 / 右端 / 左中），间距统一 3.0mm。
    en.push(hatch_ansi31_scaled(&mid, 0.0, hatch_scale_3mm()));
    en.push(hatch_ansi31_scaled(&right, 0.0, hatch_scale_3mm()));
    en.push(hatch_ansi31_scaled(&left_loop, 0.0, hatch_scale_3mm()));
    en
}

/// 1097 俯视图：A 跑道形 R=b/2 / B 矩形 + 倒角内轮廓 + 三组中心线 + 全部孔圈。
fn top_1097(ty: KeyType, r: &Key1097Row, len: &Key1097LengthRow) -> Vec<EntityType> {
    let l = len.l;
    let (b, c) = (r.b, r.c);
    let (xh, d0, d1, d) = (len.l1 / 2.0, r.d0, r.d1, r.d_sink);
    let rcs = d0 / 2.0 - r.c1 * 60f64.to_radians().tan();
    let rmin = MINOR_THREAD_FACTOR * d0 / 2.0;
    let half = b / 2.0;
    let ri = half - c;
    let over = half + 3.0;
    let mut en = Vec::new();
    // 中央 d0 螺纹孔三圈：大径实线、锪锥小端实线、小径 3/4 圈（**2细线层**，用户拍板）。
    en.push(circle([0.0, 0.0], d0 / 2.0, LAYER_MAIN));
    en.push(circle([0.0, 0.0], rcs, LAYER_MAIN));
    // 两端 d1 通孔 + D 沉孔圈。
    for x in [-xh, xh] {
        en.push(circle([x, 0.0], d1 / 2.0, LAYER_MAIN));
        en.push(circle([x, 0.0], d / 2.0, LAYER_MAIN));
    }
    // 中心线：纵向轴、两端孔轴；横向轴两端各伸 3。
    en.push(line([0.0, -over], [0.0, over], LAYER_CENTER));
    en.push(line([-(l / 2.0 + 3.0), 0.0], [l / 2.0 + 3.0, 0.0], LAYER_CENTER));
    for x in [-xh, xh] {
        en.push(line([x, -over], [x, over], LAYER_CENTER));
    }
    // 螺纹小径 3/4 圈：缺口在右下象限（0°→270° 逆时针），照源图方位。
    en.push(arc([0.0, 0.0], rmin, 0.0, 270.0, LAYER_THIN));
    // 外形 + 倒角内轮廓。
    match ty {
        KeyType::A => {
            // 左右半各画一段：源图用镜像生成，断裂在 x=0（不是重复实体，是两段共线轮廓）。
            let (cl, cr) = (l / 2.0 - half, -(l / 2.0 - half));
            en.push(arc([cr, 0.0], half, 90.0, 270.0, LAYER_MAIN));
            en.push(arc([cl, 0.0], half, 270.0, 90.0, LAYER_MAIN));
            for x in [cr, cl] {
                en.push(line([x, -half], [0.0, -half], LAYER_MAIN));
                en.push(line([x, half], [0.0, half], LAYER_MAIN));
                en.push(line([x, -ri], [0.0, -ri], LAYER_MAIN));
                en.push(line([x, ri], [0.0, ri], LAYER_MAIN));
            }
            en.push(arc([cr, 0.0], ri, 90.0, 270.0, LAYER_MAIN));
            en.push(arc([cl, 0.0], ri, 270.0, 90.0, LAYER_MAIN));
        }
        KeyType::B => {
            en.push(polyline(
                &[
                    [-l / 2.0, half],
                    [l / 2.0, half],
                    [l / 2.0, -half],
                    [-l / 2.0, -half],
                ],
                true,
                LAYER_MAIN,
            ));
            en.push(line([-l / 2.0, ri], [l / 2.0, ri], LAYER_MAIN));
            en.push(line([-l / 2.0, -ri], [l / 2.0, -ri], LAYER_MAIN));
        }
        KeyType::C => unreachable!("{}", crate::i18n::t("cmd.parts.err.key1097_no_c")),
    }
    en
}

/// 1097 单件重量估算（钢 7.85 g/cm³；跑道/矩形面积 − 孔体积）。
fn weight_1097(ty: KeyType, r: &Key1097Row, l: f64) -> f64 {
    let area = match ty {
        KeyType::A => (l - r.b).max(0.0) * r.b + std::f64::consts::PI * r.b * r.b / 4.0,
        KeyType::B => l * r.b,
        KeyType::C => unreachable!(),
    };
    let mut vol = area * r.h;
    // 键中央孔按通孔（用户裁定：L0 不入键零件几何）；体积按贯穿全高 h 扣。
    vol -= std::f64::consts::PI * (r.d0 / 2.0).powi(2) * r.h;
    for _ in 0..2 {
        vol -= std::f64::consts::PI * (r.d1 / 2.0).powi(2) * r.h;
        vol -= std::f64::consts::PI / 4.0 * (r.d_sink * r.d_sink - r.d1 * r.d1) * r.h1;
    }
    (vol.max(0.0)) * 7.85e-3 / 1000.0
}

// ══════════════════════════════════════════════════════════════════════════
// 包围盒
// ══════════════════════════════════════════════════════════════════════════

/// 包围盒：Line/LwPolyline 取顶点；Circle 取 center±r；Arc 取采样点（16 段）。
fn bbox_of(entities: &[EntityType]) -> [f64; 4] {
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    let mut push = |x: f64, y: f64| {
        b[0] = b[0].min(x);
        b[1] = b[1].min(y);
        b[2] = b[2].max(x);
        b[3] = b[3].max(y);
    };
    for e in entities {
        match e {
            EntityType::Line(l) => {
                push(l.start.x, l.start.y);
                push(l.end.x, l.end.y);
            }
            EntityType::Circle(c) => {
                push(c.center.x - c.radius, c.center.y - c.radius);
                push(c.center.x + c.radius, c.center.y + c.radius);
            }
            EntityType::Arc(a) => {
                let (s, e) = (a.start_angle, a.end_angle);
                let mut sweep = e - s;
                while sweep <= 0.0 {
                    sweep += std::f64::consts::TAU;
                }
                for i in 0..=16 {
                    let ang = s + sweep * (i as f64 / 16.0);
                    push(
                        a.center.x + a.radius * ang.cos(),
                        a.center.y + a.radius * ang.sin(),
                    );
                }
            }
            EntityType::LwPolyline(p) => {
                for v in &p.vertices {
                    push(v.location.x, v.location.y);
                }
            }
            // HATCH 的边界由轮廓多段线覆盖，不另计。
            _ => {}
        }
    }
    b
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

    use crate::partgen::generate as gen_all;
    use crate::partgen_kit::{hatch_perpendicular_spacing, part_svg};
    use ocs_plugin_api::host::acadrust::types::Color;

    const OCSM_LAYERS: [&str; 5] = [
        LAYER_MAIN,
        LAYER_THIN,
        LAYER_CENTER,
        LAYER_HIDDEN,
        crate::partgen_kit::LAYER_HATCH,
    ];

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-4
    }

    /// 实体规范化字符串（Line 端点方向无关；Polyline 保留顶点顺序与闭合位；含图层）。
    fn canon(e: &EntityType) -> String {
        let layer = e.common().layer.clone();
        match e {
            EntityType::Line(l) => {
                let (mut a, mut b) = ((l.start.x, l.start.y), (l.end.x, l.end.y));
                if a > b {
                    std::mem::swap(&mut a, &mut b);
                }
                format!("LINE ({:.4},{:.4})-({:.4},{:.4}) [{layer}]", a.0, a.1, b.0, b.1)
            }
            EntityType::Arc(a) => format!(
                "ARC c=({:.4},{:.4}) r={:.6} {:.3}..{:.3} [{layer}]",
                a.center.x,
                a.center.y,
                a.radius,
                a.start_angle.to_degrees(),
                a.end_angle.to_degrees()
            ),
            EntityType::Circle(c) => format!(
                "CIRCLE c=({:.4},{:.4}) r={:.6} [{layer}]",
                c.center.x, c.center.y, c.radius
            ),
            EntityType::LwPolyline(pl) => {
                let vs: Vec<String> = pl
                    .vertices
                    .iter()
                    .map(|v| format!("({:.4},{:.4})", v.location.x, v.location.y))
                    .collect();
                format!(
                    "POLYLINE{}{} [{layer}]",
                    if pl.is_closed { " closed" } else { "" },
                    vs.join("")
                )
            }
            EntityType::Hatch(h) => format!(
                "HATCH angle={:.3} scale={:.6} [{layer}]",
                h.pattern_angle.to_degrees(),
                h.pattern_scale
            ),
            other => format!("OTHER {:?} [{layer}]", std::mem::discriminant(other)),
        }
    }

    fn canonical(entities: &[EntityType]) -> Vec<String> {
        let mut v: Vec<String> = entities.iter().map(canon).collect();
        v.sort();
        v
    }

    /// 逐图元对模板：实体数 + 规范化集合（类型/坐标/图层）完全一致。
    fn assert_entities(entities: &[EntityType], expected: &[EntityType], what: &str) {
        assert!(
            !entities.iter().any(|e| matches!(e, EntityType::Dimension(_))),
            "{what}: 不应有任何尺寸标注"
        );
        assert_eq!(entities.len(), expected.len(), "{what}: 图元数不一致");
        let (got, want) = (canonical(entities), canonical(expected));
        if got != want {
            let mut msg = format!("{what}: 逐图元不一致\n");
            for c in &want {
                if !got.contains(c) {
                    msg.push_str(&format!("  缺: {c}\n"));
                }
            }
            for c in &got {
                if !want.contains(c) {
                    msg.push_str(&format!("  多: {c}\n"));
                }
            }
            panic!("{msg}");
        }
    }

    fn assert_template(p: &GenPart, expected: &[EntityType], what: &str) {
        assert_entities(&p.entities, expected, what);
    }

    // ── 1096 specimen b=2、h=2、L=6（模板坐标见 平键_几何反解.md §2）──

    fn exp_1096_main(ty: KeyType, c: f64) -> Vec<EntityType> {
        let h = 2.0;
        let hi = h - c;
        let outline: Vec<[f64; 2]> = match ty {
            KeyType::A => vec![
                [0.0, c], [c, 0.0], [6.0 - c, 0.0], [6.0, c],
                [6.0, hi], [6.0 - c, h], [c, h], [0.0, hi],
            ],
            KeyType::B => vec![
                [0.0, c], [0.0, 0.0], [6.0, 0.0], [6.0, c],
                [6.0, hi], [6.0, h], [0.0, h], [0.0, hi],
            ],
            KeyType::C => vec![
                [0.0, c], [c, 0.0], [6.0, 0.0], [6.0, c],
                [6.0, hi], [6.0, h], [c, h], [0.0, hi],
            ],
        };
        vec![
            polyline(&outline, true, LAYER_MAIN),
            line([0.0, c], [6.0, c], LAYER_MAIN),
            line([6.0, hi], [0.0, hi], LAYER_MAIN),
        ]
    }

    fn exp_1096_top(ty: KeyType, c: f64) -> Vec<EntityType> {
        let ri = 1.0 - c; // b=2 → R−c
        let mut en = vec![line([-3.0, 0.0], [9.0, 0.0], LAYER_CENTER)];
        match ty {
            KeyType::A => {
                en.push(arc([1.0, 0.0], 1.0, 90.0, 270.0, LAYER_MAIN));
                en.push(arc([5.0, 0.0], 1.0, 270.0, 90.0, LAYER_MAIN));
                en.push(line([1.0, -1.0], [5.0, -1.0], LAYER_MAIN));
                en.push(line([5.0, 1.0], [1.0, 1.0], LAYER_MAIN));
                en.push(line([1.0, -ri], [5.0, -ri], LAYER_MAIN));
                en.push(line([5.0, ri], [1.0, ri], LAYER_MAIN));
                en.push(arc([1.0, 0.0], ri, 90.0, 270.0, LAYER_MAIN));
                en.push(arc([5.0, 0.0], ri, 270.0, 90.0, LAYER_MAIN));
            }
            KeyType::B => {
                en.push(line([0.0, -1.0], [6.0, -1.0], LAYER_MAIN));
                en.push(line([6.0, 1.0], [0.0, 1.0], LAYER_MAIN));
                en.push(line([0.0, -ri], [6.0, -ri], LAYER_MAIN));
                en.push(line([0.0, ri], [6.0, ri], LAYER_MAIN));
                en.push(line([6.0, 1.0], [6.0, -1.0], LAYER_MAIN));
                en.push(line([0.0, 1.0], [0.0, -1.0], LAYER_MAIN));
            }
            KeyType::C => {
                en.push(arc([1.0, 0.0], 1.0, 90.0, 270.0, LAYER_MAIN));
                en.push(line([1.0, -1.0], [6.0, -1.0], LAYER_MAIN));
                en.push(line([6.0, 1.0], [1.0, 1.0], LAYER_MAIN));
                en.push(line([1.0, -ri], [6.0, -ri], LAYER_MAIN));
                en.push(line([1.0, ri], [6.0, ri], LAYER_MAIN));
                en.push(arc([1.0, 0.0], ri, 90.0, 270.0, LAYER_MAIN));
                en.push(line([6.0, 1.0], [6.0, -1.0], LAYER_MAIN));
            }
        }
        en
    }

    fn exp_1096_section(c: f64) -> Vec<EntityType> {
        let pts = vec![
            [0.0, c], [c, 0.0], [2.0 - c, 0.0], [2.0, c],
            [2.0, 2.0 - c], [2.0 - c, 2.0], [c, 2.0], [0.0, 2.0 - c],
        ];
        vec![
            polyline(&pts, true, LAYER_MAIN),
            // 源图 ANSI31 scale=0.25（≈0.79mm）；用户口径③改为全局 3.0mm。
            hatch_ansi31_scaled(&pts, 0.0, 3.0 / ANSI31_SPACING_MM),
        ]
    }

    #[test]
    fn template_1096_specimen_b2_h2_l6() {
        // b=2 的倒角档 = 0.16~0.25（GB/T 1096 表 1 `s` 行）；用户 2026-09 口径取**下限 0.16**。
        // 注意：源模板 DXF 画的是 0.2（在 0.16~0.25 区间内的中间值）→ 已知 0.04mm 有意偏差（举证页偏差④）。
        let c = 0.16;
        let row = row_1096(KeyType::A, 2.0).unwrap();
        assert!(row.c_min <= c && c <= row.c_max, "specimen 的 c 应落在倒角档区间内");
        assert!((c - row.c_min).abs() < 1e-12, "本库口径：c 取档位下限");
        assert!(0.2 > row.c_min && 0.2 < row.c_max, "模板的 0.2 在区间内但非下限（分歧留档）");
        for (fam, ty) in [
            ("key_1096_a", KeyType::A),
            ("key_1096_b", KeyType::B),
            ("key_1096_c", KeyType::C),
        ] {
            let main = gen_all(fam, 2.0, 6.0, "main").unwrap();
            assert_template(&main, &exp_1096_main(ty, c), &format!("{fam} 主视图"));
            let top = gen_all(fam, 2.0, 6.0, "top").unwrap();
            assert_template(&top, &exp_1096_top(ty, c), &format!("{fam} 俯视图"));
            let sec = gen_all(fam, 2.0, 6.0, "section").unwrap();
            assert_template(&sec, &exp_1096_section(c), &format!("{fam} 剖视图"));
            // 剖视图三型逐图元同构（源图实测）。
            assert_eq!(main.meta.spec, "2×2×6", "{fam} 规格文本");
        }
    }

    /// 倒角按 b 分档 + 默认取档位下限（与 1097 C 同口径）；上下限/中间值可一处切换。
    #[test]
    fn chamfer_1096_bands_lower_bound() {
        // 档位边界与范围原文（GB/T 1096 表 1 s 行）
        assert_eq!(chamfer_1096_range_for(2.0), Some((2.0, 4.0, "0.16~0.25")));
        assert_eq!(chamfer_1096_range_for(8.0), Some((5.0, 8.0, "0.25~0.40")));
        assert_eq!(chamfer_1096_range_for(9.0), None, "b=9 不在任何档");
        assert_eq!(chamfer_1096_range_for(55.0), None, "b=55 不在任何档");
        assert_eq!(chamfer_1096_range_for(100.0), Some((80.0, 100.0, "2.50~3.00")));
        // 默认策略 = 下限；Max/Mid 可切换（改 CHAMFER_1096_PICK 一处）
        assert_eq!(CHAMFER_1096_PICK, ChamferPick::Min);
        assert!((chamfer_1096_from_range(0.16, 0.25) - 0.16).abs() < 1e-12);
        assert!((chamfer_1096_from_range_pick(0.16, 0.25, ChamferPick::Max) - 0.25).abs() < 1e-12);
        assert!((chamfer_1096_from_range_pick(0.16, 0.25, ChamferPick::Mid) - 0.205).abs() < 1e-12);
        // 三型 20 档逐行：JSON 的 c_min/c_max = 档位范围，实画 c = 下限
        for ty in [KeyType::A, KeyType::B, KeyType::C] {
            let table = table_1096(ty);
            assert_eq!(table.rows.len(), 20);
            for r in &table.rows {
                let (lo, hi, range) = chamfer_1096_range_for(r.b)
                    .unwrap_or_else(|| panic!("b={} 无倒角档", r.b));
                assert!(lo <= r.b && r.b <= hi && !range.is_empty());
                let (want_min, want_max) = match r.b as i64 {
                    2..=4 => (0.16, 0.25),
                    5..=8 => (0.25, 0.40),
                    10..=18 => (0.40, 0.60),
                    20..=32 => (0.60, 0.80),
                    36..=50 => (1.00, 1.20),
                    other => panic!("测试未覆盖 b={other}"),
                };
                assert!((r.c_min - want_min).abs() < 1e-12 && (r.c_max - want_max).abs() < 1e-12,
                    "b={} 的 JSON c_min/c_max 与表 1 s 行不符", r.b);
                assert!((chamfer_1096(r) - want_min).abs() < 1e-12, "b={} 应取档位下限", r.b);
            }
        }
        // 指名档位断言（用户点名）：b=20 → 0.60、b=50 → 1.00；几何真的按 c 画
        assert!((chamfer_1096(row_1096(KeyType::A, 20.0).unwrap()) - 0.60).abs() < 1e-12);
        assert!((chamfer_1096(row_1096(KeyType::A, 50.0).unwrap()) - 1.00).abs() < 1e-12);
        let p = gen_all("key_1096_a", 20.0, 0.0, "section").unwrap();
        let pl = p
            .entities
            .iter()
            .find_map(|e| match e {
                EntityType::LwPolyline(pl) => Some(pl),
                _ => None,
            })
            .expect("剖视图应有轮廓多段线");
        assert!((pl.vertices[0].location.y - 0.60).abs() < 1e-12, "b=20 剖视图倒角应为 0.60");
    }

    // ── 1097 specimen b=8、h=7、L=100（模板见 平键_几何反解.md §3）──

    /// specimen 期望（b=8、h=7、L=100、xh=L1/2=30）。
    fn exp_1097_main_a() -> Vec<EntityType> {
        exp_1097_main_a_with(100.0, 30.0)
    }

    /// 参数化期望：只变 `L` 与 `xh=L1/2`（其余照 b=8 specimen 行）。
    /// `exp_1097_main_a_with(25.0, 6.5)` = 用户模板右侧目标（L=25→L1=13）。
    fn exp_1097_main_a_with(l: f64, xh: f64) -> Vec<EntityType> {
        let (h, c) = (7.0, 0.25);
        let (d0, d1, d) = (3.0, 3.4, 6.0);
        let (h1, c1) = (2.4, 0.3);
        let rcs = d0 / 2.0 - c1 * 60f64.to_radians().tan();
        let y0 = 0.0; // 键中央 d0 孔 = 通孔（用户裁定）
        let xb = xh - d / 2.0 - BREAK_GAP;
        // 断裂分界线（本次修正）：2细线层 + 45° 折线（自底边到顶边，与生产规则同式）。
        let ba = BREAK_AMP;
        let by0 = (h - 4.0 * ba) / 2.0;
        let brk = vec![
            [-xb, 0.0],
            [-xb, by0],
            [-(xb - ba), by0 + ba],
            [-xb, by0 + 2.0 * ba],
            [-(xb + ba), by0 + 3.0 * ba],
            [-xb, by0 + 4.0 * ba],
            [-xb, h],
        ];
        let mut left_loop = brk.clone();
        left_loop.push([-d0 / 2.0, h]);
        left_loop.push([-d0 / 2.0, 0.0]);
        let mid = vec![
            [d0 / 2.0, 0.0], [d0 / 2.0, h], [xh - d / 2.0, h],
            [xh - d / 2.0, h - h1], [xh - d1 / 2.0, h - h1], [xh - d1 / 2.0, 0.0],
        ];
        let right = vec![
            [xh + d1 / 2.0, 0.0], [xh + d1 / 2.0, h - h1], [xh + d / 2.0, h - h1],
            [xh + d / 2.0, h], [l / 2.0 - c, h], [l / 2.0, h - c],
            [l / 2.0, c], [l / 2.0 - c, 0.0],
        ];
        vec![
            line([0.0, -3.0], [0.0, h + 3.0], LAYER_CENTER),
            line([xh, -3.0], [xh, h + 3.0], LAYER_CENTER),
            line([-xh, -3.0], [-xh, h + 3.0], LAYER_CENTER),
            polyline(&[[-rcs, y0], [-rcs, h - c1], [0.0, h - c1]], false, LAYER_MAIN),
            polyline(&[[-rcs, h - c1], [-d0 / 2.0, h], [-d0 / 2.0, y0]], false, LAYER_MAIN),
            polyline(
                &[[-(xh - d1 / 2.0), 0.0], [-(xh - d1 / 2.0), h - h1], [-(xh - d / 2.0), h - h1], [-(xh - d / 2.0), h]],
                false, LAYER_HIDDEN,
            ),
            polyline(
                &[[xh - d1 / 2.0, 0.0], [xh - d1 / 2.0, h - h1], [xh - d / 2.0, h - h1], [xh - d / 2.0, h]],
                false, LAYER_MAIN,
            ),
            line([xh - d1 / 2.0, h - h1], [xh, h - h1], LAYER_MAIN),
            polyline(
                &[[0.0, 0.0], [-(l / 2.0 - c), 0.0], [-l / 2.0, c], [-l / 2.0, h - c], [-(l / 2.0 - c), h], [0.0, h]],
                false, LAYER_MAIN,
            ),
            line([-l / 2.0, h - c], [-xb, h - c], LAYER_MAIN),
            line([-l / 2.0, c], [-xb, c], LAYER_MAIN),
            polyline(&mid, true, LAYER_MAIN),
            polyline(&right, true, LAYER_MAIN),
            polyline(&brk, false, LAYER_THIN),
            polyline(&[[rcs, y0], [rcs, h - c1], [0.0, h - c1]], false, LAYER_MAIN),
            polyline(&[[rcs, h - c1], [d0 / 2.0, h], [d0 / 2.0, y0]], false, LAYER_MAIN),
            polyline(
                &[[-(xh + d1 / 2.0), 0.0], [-(xh + d1 / 2.0), h - h1], [-(xh + d / 2.0), h - h1], [-(xh + d / 2.0), h]],
                false, LAYER_HIDDEN,
            ),
            polyline(
                &[[xh + d1 / 2.0, 0.0], [xh + d1 / 2.0, h - h1], [xh + d / 2.0, h - h1], [xh + d / 2.0, h]],
                false, LAYER_MAIN,
            ),
            line([xh + d1 / 2.0, h - h1], [xh, h - h1], LAYER_MAIN),
            polyline(
                &[[0.0, 0.0], [l / 2.0 - c, 0.0], [l / 2.0, c], [l / 2.0, h - c], [l / 2.0 - c, h], [0.0, h]],
                false, LAYER_MAIN,
            ),
            hatch_ansi31_scaled(&mid, 0.0, 3.0 / ANSI31_SPACING_MM),
            hatch_ansi31_scaled(&right, 0.0, 3.0 / ANSI31_SPACING_MM),
            hatch_ansi31_scaled(&left_loop, 0.0, 3.0 / ANSI31_SPACING_MM),
        ]
    }

    fn exp_1097_main_b() -> Vec<EntityType> {
        exp_1097_main_b_with(100.0, 30.0)
    }

    fn exp_1097_main_b_with(l: f64, xh: f64) -> Vec<EntityType> {
        let (h, c) = (7.0, 0.25);
        let (d0, d1, d) = (3.0, 3.4, 6.0);
        let (h1, c1) = (2.4, 0.3);
        let rcs = d0 / 2.0 - c1 * 60f64.to_radians().tan();
        let y0 = 0.0; // 键中央 d0 孔 = 通孔（用户裁定）
        let xb = xh - d / 2.0 - BREAK_GAP;
        // 断裂分界线（本次修正）：2细线层 + 45° 折线（自底边到顶边，与生产规则同式）。
        let ba = BREAK_AMP;
        let by0 = (h - 4.0 * ba) / 2.0;
        let brk = vec![
            [-xb, 0.0],
            [-xb, by0],
            [-(xb - ba), by0 + ba],
            [-xb, by0 + 2.0 * ba],
            [-(xb + ba), by0 + 3.0 * ba],
            [-xb, by0 + 4.0 * ba],
            [-xb, h],
        ];
        let mut left_loop = brk.clone();
        left_loop.push([-d0 / 2.0, h]);
        left_loop.push([-d0 / 2.0, 0.0]);
        let mid = vec![
            [d0 / 2.0, 0.0], [d0 / 2.0, h], [xh - d / 2.0, h],
            [xh - d / 2.0, h - h1], [xh - d1 / 2.0, h - h1], [xh - d1 / 2.0, 0.0],
        ];
        let right = vec![
            [xh + d1 / 2.0, 0.0], [xh + d1 / 2.0, h - h1], [xh + d / 2.0, h - h1],
            [xh + d / 2.0, h], [l / 2.0, h], [l / 2.0, 0.0],
        ];
        vec![
            line([0.0, -3.0], [0.0, h + 3.0], LAYER_CENTER),
            line([xh, -3.0], [xh, h + 3.0], LAYER_CENTER),
            line([-xh, -3.0], [-xh, h + 3.0], LAYER_CENTER),
            polyline(&[[-rcs, y0], [-rcs, h - c1], [0.0, h - c1]], false, LAYER_MAIN),
            polyline(&[[-rcs, h - c1], [-d0 / 2.0, h], [-d0 / 2.0, y0]], false, LAYER_MAIN),
            polyline(
                &[[-(xh - d1 / 2.0), 0.0], [-(xh - d1 / 2.0), h - h1], [-(xh - d / 2.0), h - h1], [-(xh - d / 2.0), h]],
                false, LAYER_HIDDEN,
            ),
            polyline(
                &[[xh - d1 / 2.0, 0.0], [xh - d1 / 2.0, h - h1], [xh - d / 2.0, h - h1], [xh - d / 2.0, h]],
                false, LAYER_MAIN,
            ),
            line([xh - d1 / 2.0, h - h1], [xh, h - h1], LAYER_MAIN),
            line([-l / 2.0, h - c], [-xb, h - c], LAYER_MAIN),
            line([-l / 2.0, c], [-xb, c], LAYER_MAIN),
            polyline(&mid, true, LAYER_MAIN),
            polyline(&right, true, LAYER_MAIN),
            polyline(&brk, false, LAYER_THIN),
            polyline(&[[rcs, y0], [rcs, h - c1], [0.0, h - c1]], false, LAYER_MAIN),
            polyline(&[[rcs, h - c1], [d0 / 2.0, h], [d0 / 2.0, y0]], false, LAYER_MAIN),
            polyline(
                &[[-(xh + d1 / 2.0), 0.0], [-(xh + d1 / 2.0), h - h1], [-(xh + d / 2.0), h - h1], [-(xh + d / 2.0), h]],
                false, LAYER_HIDDEN,
            ),
            polyline(
                &[[xh + d1 / 2.0, 0.0], [xh + d1 / 2.0, h - h1], [xh + d / 2.0, h - h1], [xh + d / 2.0, h]],
                false, LAYER_MAIN,
            ),
            line([xh + d1 / 2.0, h - h1], [xh, h - h1], LAYER_MAIN),
            polyline(&[[-l / 2.0, 0.0], [-l / 2.0, h], [l / 2.0, h], [l / 2.0, 0.0]], true, LAYER_MAIN),
            hatch_ansi31_scaled(&mid, 0.0, 3.0 / ANSI31_SPACING_MM),
            hatch_ansi31_scaled(&right, 0.0, 3.0 / ANSI31_SPACING_MM),
            hatch_ansi31_scaled(&left_loop, 0.0, 3.0 / ANSI31_SPACING_MM),
        ]
    }

    fn exp_1097_top_a() -> Vec<EntityType> {
        exp_1097_top_a_with(100.0, 30.0)
    }

    fn exp_1097_top_a_with(l: f64, xh: f64) -> Vec<EntityType> {
        let (b, c) = (8.0, 0.25);
        let (d0, d1, d) = (3.0, 3.4, 6.0);
        let rcs = d0 / 2.0 - 0.3 * 60f64.to_radians().tan();
        let rmin = 0.85 * d0 / 2.0;
        let half = b / 2.0;
        let ri = half - c;
        let over = half + 3.0;
        let cl = l / 2.0 - half;
        vec![
            circle([0.0, 0.0], d0 / 2.0, LAYER_MAIN),
            circle([0.0, 0.0], rcs, LAYER_MAIN),
            circle([-xh, 0.0], d1 / 2.0, LAYER_MAIN),
            circle([-xh, 0.0], d / 2.0, LAYER_MAIN),
            circle([xh, 0.0], d1 / 2.0, LAYER_MAIN),
            circle([xh, 0.0], d / 2.0, LAYER_MAIN),
            line([0.0, -over], [0.0, over], LAYER_CENTER),
            line([-(l / 2.0 + 3.0), 0.0], [l / 2.0 + 3.0, 0.0], LAYER_CENTER),
            line([-xh, -over], [-xh, over], LAYER_CENTER),
            line([xh, -over], [xh, over], LAYER_CENTER),
            arc([0.0, 0.0], rmin, 0.0, 270.0, LAYER_THIN),
            arc([-cl, 0.0], half, 90.0, 270.0, LAYER_MAIN),
            arc([cl, 0.0], half, 270.0, 90.0, LAYER_MAIN),
            line([-cl, -half], [0.0, -half], LAYER_MAIN),
            line([-cl, half], [0.0, half], LAYER_MAIN),
            line([cl, -half], [0.0, -half], LAYER_MAIN),
            line([cl, half], [0.0, half], LAYER_MAIN),
            line([-cl, -ri], [0.0, -ri], LAYER_MAIN),
            line([-cl, ri], [0.0, ri], LAYER_MAIN),
            line([cl, -ri], [0.0, -ri], LAYER_MAIN),
            line([cl, ri], [0.0, ri], LAYER_MAIN),
            arc([-cl, 0.0], ri, 90.0, 270.0, LAYER_MAIN),
            arc([cl, 0.0], ri, 270.0, 90.0, LAYER_MAIN),
        ]
    }

    fn exp_1097_top_b() -> Vec<EntityType> {
        exp_1097_top_b_with(100.0, 30.0)
    }

    fn exp_1097_top_b_with(l: f64, xh: f64) -> Vec<EntityType> {
        let (b, c) = (8.0, 0.25);
        let (d0, d1, d) = (3.0, 3.4, 6.0);
        let rcs = d0 / 2.0 - 0.3 * 60f64.to_radians().tan();
        let rmin = 0.85 * d0 / 2.0;
        let half = b / 2.0;
        let ri = half - c;
        let over = half + 3.0;
        vec![
            circle([0.0, 0.0], d0 / 2.0, LAYER_MAIN),
            circle([0.0, 0.0], rcs, LAYER_MAIN),
            circle([-xh, 0.0], d1 / 2.0, LAYER_MAIN),
            circle([-xh, 0.0], d / 2.0, LAYER_MAIN),
            circle([xh, 0.0], d1 / 2.0, LAYER_MAIN),
            circle([xh, 0.0], d / 2.0, LAYER_MAIN),
            line([0.0, -over], [0.0, over], LAYER_CENTER),
            line([-(l / 2.0 + 3.0), 0.0], [l / 2.0 + 3.0, 0.0], LAYER_CENTER),
            line([-xh, -over], [-xh, over], LAYER_CENTER),
            line([xh, -over], [xh, over], LAYER_CENTER),
            arc([0.0, 0.0], rmin, 0.0, 270.0, LAYER_THIN),
            polyline(&[[-l / 2.0, half], [l / 2.0, half], [l / 2.0, -half], [-l / 2.0, -half]], true, LAYER_MAIN),
            line([-l / 2.0, ri], [l / 2.0, ri], LAYER_MAIN),
            line([-l / 2.0, -ri], [l / 2.0, -ri], LAYER_MAIN),
        ]
    }

    #[test]
    fn template_1097_specimen_b8_h7_l100() {
        // 源模板 specimen 就是 b=8×h=7×L=100；但 L=100 对 b=8 违反标准注③ `L<10b`
        //（模板自身如此），所以逐图元比对直接走视图函数，公共入口的该校验另行单测。
        let row_a = row_1097(KeyType::A, 8.0).unwrap();
        let row_b = row_1097(KeyType::B, 8.0).unwrap();
        let len = key_1097_length_for(100.0).unwrap();
        assert_entities(
            &main_1097(KeyType::A, row_a, len),
            &exp_1097_main_a(),
            "key_1097_a 主视图（specimen L=100）",
        );
        assert_entities(
            &top_1097(KeyType::A, row_a, len),
            &exp_1097_top_a(),
            "key_1097_a 俯视图（specimen L=100）",
        );
        assert_entities(
            &main_1097(KeyType::B, row_b, len),
            &exp_1097_main_b(),
            "key_1097_b 主视图（specimen L=100）",
        );
        assert_entities(
            &top_1097(KeyType::B, row_b, len),
            &exp_1097_top_b(),
            "key_1097_b 俯视图（specimen L=100）",
        );
        // 公共入口：specimen 尺寸除 L 外都应能出（L=100 被注③挡住，见 length 测试）。
        let p = gen_all("key_1097_a", 8.0, 70.0, "top").unwrap();
        assert_eq!(p.meta.spec, "8×7×70");
    }

    /// HATCH 边界 → 无向线段规范化集合（跨 HATCH 合并；方向/起点无关）。
    /// 模板右侧是 1 个 HATCH 3 环、本库是 3 个 HATCH，故只能比边集合。
    fn hatch_edge_set(entities: &[EntityType]) -> Vec<String> {
        use ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge;
        let mut out = Vec::new();
        for e in entities {
            let EntityType::Hatch(h) = e else { continue };
            for p in &h.paths {
                for edge in &p.edges {
                    if let BoundaryEdge::Line(le) = edge {
                        out.push(undirected_edge((le.start.x, le.start.y), (le.end.x, le.end.y)));
                    }
                }
            }
        }
        out.sort();
        out
    }

    fn edge_set_of_loops(loops: &[Vec<[f64; 2]>]) -> Vec<String> {
        let mut out = Vec::new();
        for lp in loops {
            for i in 0..lp.len() {
                let j = (i + 1) % lp.len();
                out.push(undirected_edge((lp[i][0], lp[i][1]), (lp[j][0], lp[j][1])));
            }
        }
        out.sort();
        out
    }

    fn undirected_edge(a: (f64, f64), b: (f64, f64)) -> String {
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        format!("({:.4},{:.4})-({:.4},{:.4})", a.0, a.1, b.0, b.1)
    }

    /// 短键对模板右侧逐图元：用户模板 `导向平键参数化问题.dxf` 右侧 = b=8×h=7×L=25 目标画法。
    /// 坐标取自 `review/1097长度系列_dump.txt`（相对键中心归一；左侧 INSERT 的孔位错在 specimen L1=60）。
    #[test]
    fn template_1097_short_l25_matches_right_side() {
        // L=25 → (L1,L2,L3)=(13,12.5,6)（GB/T 1097 首档）。
        let len = key_1097_length_for(25.0).unwrap();
        assert_eq!((len.l1, len.l2, len.l3), (13.0, 12.5, 6.0), "L=25 查表");
        let row = row_1097(KeyType::A, 8.0).unwrap();
        let got = main_1097(KeyType::A, row, len);
        // 逐图元（模板右侧坐标；xh=L1/2=6.5，端面 ±L/2=±12.5，断裂线 xb=−2.5）。
        let got_nh: Vec<EntityType> = got
            .iter()
            .filter(|e| !matches!(e, EntityType::Hatch(_)))
            .cloned()
            .collect();
        let want_nh: Vec<EntityType> = exp_1097_main_a_with(25.0, 6.5)
            .into_iter()
            .filter(|e| !matches!(e, EntityType::Hatch(_)))
            .collect();
        assert_entities(&got_nh, &want_nh, "key_1097_a L=25 主视图（specimen 结构 + 查表坐标）");
        // 模板右侧只有 18 个非 HATCH 图元：材料边界 mid/right 在模板里只作剖面线边界，不单独画可见线；
        // 本库照 specimen 源模板仍画这两条（与既有孔壁/外形边完全重合，图形无人眼可见差别）→ 比对时剔除。
        let material_outline = |e: &EntityType| {
            matches!(e, EntityType::LwPolyline(pl)
                if pl.is_closed && pl.common.layer == LAYER_MAIN
                    && (pl.vertices.len() == 6 || pl.vertices.len() == 8)
                    && pl.vertices.iter().any(|v| {
                        near(v.location.x, 4.8) || near(v.location.x, 8.2)
                    }))
        };
        let got_target: Vec<EntityType> = got
            .iter()
            .filter(|e| !matches!(e, EntityType::Hatch(_)) && !material_outline(e))
            .cloned()
            .collect();
        let want_target: Vec<EntityType> = want_nh
            .iter()
            .filter(|e| !material_outline(e))
            .cloned()
            .collect();
        assert_eq!(got_target.len(), 18, "模板右侧 18 个非 HATCH 图元");
        assert_entities(
            &got_target,
            &want_target,
            "key_1097_a L=25 主视图（模板右侧 18 图元）",
        );
        // HATCH：模板右侧 1 个 HATCH 3 环，本库 3 个 HATCH；边界线边集合必须逐段一致。
        let want_loops: Vec<Vec<[f64; 2]>> = vec![
            vec![
                [-1.5, 0.0], [-1.5, 7.0], [-2.5, 7.0], [-2.5, 4.5], [-3.0, 4.0],
                [-2.5, 3.5], [-2.0, 3.0], [-2.5, 2.5], [-2.5, 0.0],
            ],
            vec![
                [4.8, 0.0], [4.8, 4.6], [3.5, 4.6], [3.5, 7.0], [1.5, 7.0], [1.5, 0.0],
            ],
            vec![
                [8.2, 0.0], [12.25, 0.0], [12.5, 0.25], [12.5, 6.75], [12.25, 7.0],
                [9.5, 7.0], [9.5, 4.6], [8.2, 4.6],
            ],
        ];
        assert_eq!(
            hatch_edge_set(&got),
            edge_set_of_loops(&want_loops),
            "L=25 主视图 3 片剖面线边界应与模板右侧一致"
        );
        // 公共入口也能直接出 L=25（b=8：25<80 合法）。
        let p = gen_all("key_1097_a", 8.0, 25.0, "main").unwrap();
        assert_eq!(p.meta.spec, "8×7×25");
        let got_nh2: Vec<EntityType> = p
            .entities
            .iter()
            .filter(|e| !matches!(e, EntityType::Hatch(_)))
            .cloned()
            .collect();
        assert_entities(
            &got_nh2,
            &want_nh,
            "key_1097_a L=25 公共入口",
        );
    }

    /// 短键 L=25/32/50：孔位/断裂线全部随长度表派生（不再用 specimen L1=60）。
    #[test]
    fn short_keys_l25_l32_l50_follow_length_table() {
        for l in [25.0, 32.0, 50.0] {
            let len = key_1097_length_for(l).unwrap();
            let expect_xh = len.l1 / 2.0;
            let d_sink = row_1097(KeyType::A, 8.0).unwrap().d_sink;
            let expect_xb = -(expect_xh - d_sink / 2.0 - BREAK_GAP);
            for fam in ["key_1097_a", "key_1097_b"] {
                // 俯视图：除中央 d0 孔外的圆全部在 ±L1/2。
                let p = gen_all(fam, 8.0, l, "top").unwrap();
                let mut centers: Vec<f64> = p
                    .entities
                    .iter()
                    .filter_map(|e| match e {
                        EntityType::Circle(c) if c.center.x.abs() > 1e-9 => Some(c.center.x),
                        _ => None,
                    })
                    .collect();
                centers.sort_by(|a, b| a.partial_cmp(b).unwrap());
                assert_eq!(centers.len(), 4, "{fam} L={l}：两端各 d1/D 两圈");
                for (i, x) in centers.iter().enumerate() {
                    let side = if i < 2 { -1.0 } else { 1.0 };
                    assert!(
                        (x - side * expect_xh).abs() < 1e-12,
                        "{fam} L={l}：孔心应 ±L1/2={expect_xh}，实得 {x}"
                    );
                }
                // 主视图：断裂线位于 xb（随 L1 派生）。
                let m = gen_all(fam, 8.0, l, "main").unwrap();
                let thin = m
                    .entities
                    .iter()
                    .find_map(|e| match e {
                        EntityType::LwPolyline(pl) if pl.common.layer == LAYER_THIN => Some(pl),
                        _ => None,
                    })
                    .expect("断裂线");
                assert!(
                    (thin.vertices[0].location.x - expect_xb).abs() < 1e-12,
                    "{fam} L={l}：断裂线应在 {expect_xb}（L1={}），实得 {}",
                    len.l1,
                    thin.vertices[0].location.x
                );
            }
        }
    }

    /// 1097 断裂分界线（用户 2026-09 画法修正）：必在 `2细线层`；形态 = 2 端竖线 + 4 段严格 45°
    /// 折线；端点在 xb=−(L1/2−D/2−BREAK_GAP)；其余几何（实体数/棱线端点/剖面线片数）不受影响。
    #[test]
    fn break_line_1097_is_thin_45deg_zigzag() {
        for (fam, ty) in [("key_1097_a", KeyType::A), ("key_1097_b", KeyType::B)] {
            let row = row_1097(ty, 8.0).unwrap();
            for (l, who) in [(100.0, "specimen L=100"), (70.0, "合法 L=70")] {
                let len = key_1097_length_for(l).unwrap();
                let ents = main_1097(ty, row, len);
                // 主视图只有一条 `2细线层` 图元 = 分界线（其余线在 1/3/4/5 层）。
                let thin: Vec<_> = ents
                    .iter()
                    .filter_map(|e| match e {
                        EntityType::LwPolyline(pl) if pl.common.layer == LAYER_THIN => Some(pl),
                        _ => None,
                    })
                    .collect();
                assert_eq!(thin.len(), 1, "{fam} {who}：分界线应唯一且在 2细线层");
                let pl = thin[0];
                assert!(!pl.is_closed, "{fam} {who}：断裂线应为开口折线");
                let pts: Vec<(f64, f64)> = pl
                    .vertices
                    .iter()
                    .map(|v| (v.location.x, v.location.y))
                    .collect();
                assert_eq!(pts.len(), 7, "{fam} {who}：2 端竖线 + 4 段45° → 7 顶点");
                let xb = -(len.l1 / 2.0 - row.d_sink / 2.0 - BREAK_GAP);
                assert!((pts[0].0 - xb).abs() < 1e-12 && pts[0].1.abs() < 1e-12, "起点在底边");
                assert!(
                    (pts[6].0 - xb).abs() < 1e-12 && (pts[6].1 - row.h).abs() < 1e-12,
                    "终点在顶边"
                );
                for i in 0..6 {
                    let (dx, dy) = (pts[i + 1].0 - pts[i].0, pts[i + 1].1 - pts[i].1);
                    if i == 0 || i == 5 {
                        assert!(dx.abs() < 1e-12 && dy.abs() > 0.0, "端段应为竖线（i={i}）");
                    } else {
                        assert!(
                            (dx.abs() - BREAK_AMP).abs() < 1e-12
                                && (dy.abs() - BREAK_AMP).abs() < 1e-12,
                            "斜段应为严格45°：|dx|=|dy|=BREAK_AMP（i={i}，实得 {dx}/{dy}）"
                        );
                    }
                }
                // 倒角棱线仍止于分界线（其余几何不受影响）。
                assert!(
                    ents.iter().any(|e| matches!(e, EntityType::Line(ln)
                        if (ln.end.x - xb).abs() < 1e-12
                            && (ln.start.y - (row.h - row.c)).abs() < 1e-12)),
                    "{fam} {who}：上倒角棱线应止于断裂线 xb"
                );
                let want = if ty == KeyType::A { 23 } else { 22 };
                assert_eq!(ents.len(), want, "{fam} {who}：实体数应与修正前一致");
                let hatches = ents.iter().filter(|e| matches!(e, EntityType::Hatch(_))).count();
                assert_eq!(hatches, 3, "{fam}：3 片剖面线不变");
            }
        }
    }

    /// 3/4 螺纹小径圈必须在 `2细线层`（用户口径①；源图在 4虚线层），缺口右下 0°→270°。
    #[test]
    fn thread_minor_arc_in_thin_layer() {
        for fam in ["key_1097_a", "key_1097_b"] {
            // 用小径圈无关的合法 L=70（specimen 的 L=100 违反注③；逐图元比对另测）。
            let p = gen_all(fam, 8.0, 70.0, "top").unwrap();
            let arcs: Vec<_> = p
                .entities
                .iter()
                .filter_map(|e| match e {
                    EntityType::Arc(a) => Some(a),
                    _ => None,
                })
                .collect();
            let expect_arcs = if fam == "key_1097_a" { 5 } else { 1 }; // B 型方头无端部半圆
            assert_eq!(arcs.len(), expect_arcs, "{fam} 俯视图弧段数（3/4 小径 + 端部半圆）");
            let thin: Vec<_> = arcs
                .iter()
                .filter(|a| a.common.layer == LAYER_THIN)
                .collect();
            assert_eq!(thin.len(), 1, "{fam}：只有 1 段弧在 2细线层");
            let a = thin[0];
            assert!(near(a.center.x, 0.0) && near(a.center.y, 0.0), "小径圈圆心在键中心");
            assert!(near(a.radius, 0.85 * 3.0 / 2.0), "小径半径 = 0.85·d0/2");
            assert!(near(a.start_angle.to_degrees(), 0.0) && near(a.end_angle.to_degrees(), 270.0), "3/4 圈缺口在右下（0→270°）");
            assert!(
                arcs.iter().all(|a| a.common.layer != LAYER_HIDDEN),
                "{fam}：4虚线层不应再有螺纹圈（源图的 4虚线已按用户口径移走）"
            );
        }
    }

    /// 裁定②（用户 2026-09-25）：键中央 d0 孔 = **通孔**（螺钉穿过键身）；
    /// 多档 L0>h 不得再把孔底画到键外；`L0` 保留在数据里但不参与键零件几何。
    #[test]
    fn key_1097_central_hole_is_through_all_rows() {
        let mut checked_beyond_h = 0;
        for ty in [KeyType::A, KeyType::B] {
            let rows = &table_1097(ty).rows;
            assert_eq!(rows.len(), 14, "{ty:?} 1097 表应为 14 档");
            for r in rows {
                // L=25 对所有 b（8…45）都合法：25 < 10b 且 ∈ 系列。
                let len = key_1097_length_for(25.0).unwrap();
                let ents = main_1097(ty, r, len);
                let rcs = r.d0 / 2.0 - r.c1 * 60f64.to_radians().tan();
                let has_pt = |x: f64, y: f64| {
                    ents.iter().any(|e| {
                        matches!(e, EntityType::LwPolyline(pl)
                            if pl.vertices.iter().any(|v| near(v.location.x, x) && near(v.location.y, y)))
                    })
                };
                // 正向：孔壁贯穿全高（小端壁到锪锥根；口部大径壁 0→h）。
                for side in [-1.0, 1.0] {
                    assert!(
                        has_pt(side * rcs, 0.0) && has_pt(side * rcs, r.h - r.c1),
                        "{ty:?} b={}：中央孔小端壁应自底面贯穿到锪锥根",
                        trim(r.b)
                    );
                    assert!(
                        has_pt(side * r.d0 / 2.0, 0.0) && has_pt(side * r.d0 / 2.0, r.h),
                        "{ty:?} b={}：中央孔口部壁应自底面贯穿到顶面",
                        trim(r.b)
                    );
                }
                // 反向：除中心线外不得有键外几何（旧 bug：y0=h−L0<0 时孔壁下探到键底之下）。
                for e in &ents {
                    let (layer, pts): (&str, Vec<[f64; 2]>) = match e {
                        EntityType::LwPolyline(pl) => (
                            &pl.common.layer,
                            pl.vertices.iter().map(|v| [v.location.x, v.location.y]).collect(),
                        ),
                        EntityType::Line(l) => (
                            &l.common.layer,
                            vec![[l.start.x, l.start.y], [l.end.x, l.end.y]],
                        ),
                        _ => continue,
                    };
                    if layer == LAYER_CENTER {
                        continue;
                    }
                    for p in pts {
                        assert!(
                            p[1] >= -1e-9 && p[1] <= r.h + 1e-9,
                            "{ty:?} b={}：非中心线图元越出键厚 [0,{}]（{p:?}）",
                            trim(r.b),
                            trim(r.h)
                        );
                    }
                }
                if r.l0 > r.h + 1e-9 {
                    checked_beyond_h += 1;
                }
            }
        }
        assert!(checked_beyond_h > 0, "应至少覆盖一档 L0>h（旧 bug 触发档）");
    }

    /// 20 档（1096）× 3 视图、14 档（1097）× 2 视图全遍历：能出图、图层合法、无标注、bbox 有效。
    #[test]
    fn all_specs_and_views_generate_clean() {
        for (fam, ty) in [
            ("key_1096_a", KeyType::A),
            ("key_1096_b", KeyType::B),
            ("key_1096_c", KeyType::C),
        ] {
            let table = table_1096(ty);
            assert_eq!(table.rows.len(), 20, "{fam} 应 20 档");
            for r in &table.rows {
                // 倒角：每一档实画值 = 该档范围下限（用户 2026-09 新口径）。
                assert!(
                    (chamfer_1096(r) - r.c_min).abs() < 1e-12,
                    "{fam} b={} 实画倒角应取档位下限",
                    r.b
                );
                for view in family_views(fam) {
                    let p = gen_all(fam, r.b, 0.0, view)
                        .unwrap_or_else(|e| panic!("{fam} b={} {view}: {e}", r.b));
                    assert!(!p.entities.is_empty(), "{fam} b={} {view} 空", r.b);
                    assert_eq!(p.meta.spec, format!("{}×{}×{}", trim(r.b), trim(r.h), trim(key_1096_legal_default(r))));
                    check_clean(&p, &format!("{fam} b={} {view}", r.b));
                }
            }
        }
        for (fam, ty) in [("key_1097_a", KeyType::A), ("key_1097_b", KeyType::B)] {
            let table = table_1097(ty);
            assert_eq!(table.rows.len(), 14, "{fam} 应 14 档");
            for r in &table.rows {
                for view in family_views(fam) {
                    let p = gen_all(fam, r.b, 0.0, view)
                        .unwrap_or_else(|e| panic!("{fam} b={} {view}: {e}", r.b));
                    assert!(!p.entities.is_empty(), "{fam} b={} {view} 空", r.b);
                    assert_eq!(p.meta.spec, format!("{}×{}×{}", trim(r.b), trim(r.h), trim(key_1097_legal_default(r))));
                    check_clean(&p, &format!("{fam} b={} {view}", r.b));
                }
            }
        }
    }

    fn check_clean(p: &GenPart, what: &str) {
        let _g = zh_guard();
        for e in &p.entities {
            let lay = e.common().layer.as_str();
            assert!(OCSM_LAYERS.contains(&lay), "{what} 图层越界: {lay}");
            assert!(matches!(e.common().color, Color::ByLayer), "{what} 颜色非 ByLayer");
            assert!(!matches!(e, EntityType::Dimension(_)), "{what} 出现尺寸标注");
        }
        assert!(p.bbox[2] > p.bbox[0] && p.bbox[3] > p.bbox[1], "{what} bbox 无效");
    }

    /// L 口径：1096 系列 34 值 + `L<10b`；1097 从易紧通取得系列 + `L<10b` + 默认就近合法化。
    #[test]
    fn length_series_and_constraints() {
        // 1096
        assert_eq!(KEY_1096_L_SERIES.len(), 34, "1096 L 系列 34 值");
        assert_eq!(KEY_1096_L_SERIES[0], 6.0);
        assert_eq!(KEY_1096_L_SERIES[33], 400.0);
        assert_eq!(key_1096_l_allowed(2.0), vec![6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0]);
        assert_eq!(key_1096_l_range(2.0), Some((6.0, 18.0)));
        assert!(check_length_1096(2.0, 6.0).is_ok());
        assert!(check_length_1096(2.0, 18.0).is_ok());
        assert!(check_length_1096(2.0, 20.0).is_err(), "b=2 时 L=20 违反 L<10b");
        assert!(check_length_1096(2.0, 7.0).is_err(), "7 不在 L 系列");
        assert!(check_length_1096(50.0, 400.0).is_ok());
        assert!(check_length_1096(50.0, 450.0).is_err());
        assert!(check_length_1096(9.0, 6.0).is_err(), "b=9 不在表内");
        assert!(key_1096_default_corrections().is_empty(), "1096 默认 L 应全部合法");
        for r in &table_1096(KeyType::A).rows {
            let def = key_1096_legal_default(r);
            assert!(check_length_1096(r.b, def).is_ok(), "b={} 默认 L={} 应合法", r.b, def);
        }
        // 1097：两表系列必须一致，且与易紧通 info_42177 长度行对齐。
        assert_eq!(key_1097_l_series(), table_1097(KeyType::B).l_series.as_slice());
        assert_eq!(key_1097_l_series().len(), 26, "含 R20 延伸 450 共 26 值");
        assert_eq!(key_1097_l_series()[0], 25.0);
        assert_eq!(key_1097_l_series()[24], 400.0);
        assert_eq!(key_1097_l_series()[25], 450.0);
        assert_eq!(
            key_1097_l_allowed(8.0),
            vec![25.0, 28.0, 32.0, 36.0, 40.0, 45.0, 50.0, 56.0, 63.0, 70.0]
        );
        assert_eq!(key_1097_l_range(8.0), Some((25.0, 70.0)));
        assert_eq!(key_1097_l_range(45.0), Some((25.0, 400.0)));
        assert!(check_length_1097(8.0, 70.0).is_ok());
        assert!(check_length_1097(8.0, 100.0).is_err(), "b=8 时 L=100 违反 L<10b");
        assert!(check_length_1097(8.0, 22.0).is_err(), "22 不在 1097 L 系列");
        assert!(check_length_1097(45.0, 400.0).is_ok());
        assert!(check_length_1097(45.0, 450.0).is_err(), "450 不满足 L<10b");
        let corrections = key_1097_default_corrections();
        assert_eq!(corrections, vec![(8.0, 100.0, 70.0), (10.0, 100.0, 90.0)], "b=8/10 默认就近化");
        // 传 L 要真正进几何：L=70 的 1097A 主视图左端外形到 x=-35。
        let p = gen_all("key_1097_a", 8.0, 70.0, "main").unwrap();
        assert_eq!(p.meta.spec, "8×7×70");
        // L=70 → L1=40（查表）→ 断裂线 xb=−(20−3−1)=−16；倒角棱线从 −35 起、止于 −16。
        assert!(
            p.entities.iter().any(|e| matches!(e, EntityType::Line(l)
                if near(l.start.x, -35.0) && near(l.start.y, 6.75) && near(l.end.x, -16.0))),
            "L=70 的倒角棱线应从 x=-35 止于断裂线 x=-16"
        );
        let p = gen_all("key_1097_b", 8.0, 0.0, "main").unwrap();
        assert_eq!(p.meta.spec, "8×7×70", "省略 L 时 b=8 默认应为合法值 70");
    }

    /// GB/T 1097-2003「L 与 L1、L2、L3 的对应长度系列」26 档逐档断言
    /// （数据源：嘉立创 new05232.htm，2026-09-24 抓取 /review/1097长度系列_进度.md）+ 表外报错。
    #[test]
    fn length_table_1097_26_rows_and_out_of_table_errors() {
        let _g = zh_guard();
        const WANT: &[(f64, f64, f64, f64)] = &[
            (25.0, 13.0, 12.5, 6.0),
            (28.0, 14.0, 14.0, 7.0),
            (32.0, 16.0, 16.0, 8.0),
            (36.0, 18.0, 18.0, 9.0),
            (40.0, 20.0, 20.0, 10.0),
            (45.0, 23.0, 22.5, 11.0),
            (50.0, 26.0, 25.0, 12.0),
            (56.0, 30.0, 28.0, 13.0),
            (63.0, 35.0, 31.5, 14.0),
            (70.0, 40.0, 35.0, 15.0),
            (80.0, 48.0, 40.0, 16.0),
            (90.0, 54.0, 45.0, 18.0),
            (100.0, 60.0, 50.0, 20.0),
            (110.0, 66.0, 55.0, 22.0),
            (125.0, 75.0, 62.5, 25.0),
            (140.0, 80.0, 70.0, 30.0),
            (160.0, 90.0, 80.0, 35.0),
            (180.0, 100.0, 90.0, 40.0),
            (200.0, 110.0, 100.0, 45.0),
            (220.0, 120.0, 110.0, 50.0),
            (250.0, 140.0, 125.0, 55.0),
            (280.0, 160.0, 140.0, 60.0),
            (320.0, 180.0, 160.0, 70.0),
            (360.0, 200.0, 180.0, 80.0),
            (400.0, 220.0, 200.0, 90.0),
            (450.0, 250.0, 225.0, 100.0),
        ];
        let rows = key_1097_length_rows();
        assert_eq!(rows.len(), WANT.len(), "长度系列应 26 档");
        for (i, (l, l1, l2, l3)) in WANT.iter().copied().enumerate() {
            let r = key_1097_length_for(l).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(
                (r.l, r.l1, r.l2, r.l3),
                (l, l1, l2, l3),
                "长度表第 {} 档 L={} 与 JLC 表不符",
                i + 1,
                trim(l)
            );
            let rb = &table_1097(KeyType::B).length_rows[i];
            assert_eq!(
                (rb.l, rb.l1, rb.l2, rb.l3),
                (l, l1, l2, l3),
                "A/B 两表第 {} 档不一致",
                i + 1
            );
            assert_eq!(key_1097_l_series()[i], l, "l_series 与长度表键不一致");
            assert!(
                (r.l2 - l / 2.0).abs() < 1e-12 && (r.l1 + 2.0 * r.l3 - l).abs() < 1e-12,
                "第 {} 档应满足 L2=L/2、L1+2L3=L",
                i + 1
            );
        }
        // 表外：不插值（25/28 之间的 26、27）、不外推（500）→ 报错并列全系列。
        for bad in [26.0, 27.0, 30.0, 500.0, 0.0, -25.0] {
            let err = key_1097_length_for(bad).unwrap_err();
            assert!(err.contains(&format!("没有 L={}", trim(bad))), "{bad}: {err}");
            assert!(
                err.contains("不插值") && err.contains("25") && err.contains("450"),
                "{bad}: {err}"
            );
        }
        // 公共入口：表外 L 也必须明确报错。
        let err = gen_all("key_1097_a", 8.0, 27.0, "main").unwrap_err();
        assert!(err.contains("没有 L=27") && err.contains("不插值"), "{err}");
        let err = gen_all("key_1097_a", 45.0, 500.0, "main").unwrap_err();
        assert!(err.contains("没有 L=500"), "{err}");
        // 固定用螺钉列（同页 rowspan 对齐解析）：逐行抽检。
        for (b, want) in [(8.0, "M3×8"), (12.0, "M4×10"), (22.0, "M6×16"), (45.0, "M12×25")] {
            assert_eq!(row_1097(KeyType::A, b).unwrap().screw, want, "b={b} 固定螺钉");
        }
    }

    /// 目录集成：五族登记、kind=key、型别下拉数据、视图/规格数、树上挂位。
    #[test]
    fn catalog_registration() {
        let _g = zh_guard();
        let cat: serde_json::Value = serde_json::from_str(&crate::partgen::catalog_json()).unwrap();
        let fams = &cat["families"];
        for (fam, n_sizes, n_views, n_types) in [
            ("key_1096_a", 20, 3, 3),
            ("key_1096_b", 20, 3, 3),
            ("key_1096_c", 20, 3, 3),
            ("key_1097_a", 14, 2, 2),
            ("key_1097_b", 14, 2, 2),
        ] {
            let f = &fams[fam];
            assert!(!f.is_null(), "目录缺 {fam}");
            assert_eq!(f["kind"], "key", "{fam} kind");
            assert_eq!(f["implemented"], true);
            assert_eq!(f["views"].as_array().unwrap().len(), n_views, "{fam} 视图数");
            assert_eq!(f["sizes"].as_array().unwrap().len(), n_sizes, "{fam} 规格数");
            assert_eq!(f["type_group"].as_array().unwrap().len(), n_types, "{fam} 型别下拉项");
            assert_eq!(crate::partgen::family_kind(fam), "key");
        }
        assert_eq!(fams["key_1096_a"]["views"][0]["id"], "main");
        assert_eq!(fams["key_1096_a"]["views"][2]["id"], "section");
        assert_eq!(fams["key_1097_a"]["views"][1]["id"], "top");
        assert_eq!(fams["key_1096_a"]["type_group"][1]["id"], "key_1096_b");
        // 倒角按 b 分档（用户 2026-09）：notes 说“范围下限”，sizes.extra 带实画 c。
        let notes_1096 = fams["key_1096_a"]["notes"].as_array().unwrap();
        assert!(notes_1096.iter().any(|n| n.as_str().unwrap_or("").contains("范围下限")));
        assert!(fams["key_1096_a"]["sizes"][0]["extra"].as_str().unwrap().contains("c=0.16"));
        assert!(fams["key_1096_a"]["sizes"][16]["extra"].as_str().unwrap().contains("c=1"));
        // 1097 下拉：b=8 的首个 lengths 是合法默认 70；全部 < 10b。
        let s8 = &fams["key_1097_a"]["sizes"][0];
        assert_eq!(s8["d"], 8.0);
        assert_eq!(s8["lengths"][0], 70.0);
        assert_eq!(s8["l_min"], 25.0);
        assert_eq!(s8["l_max"], 70.0);
        for fam in ["key_1096_a", "key_1096_b", "key_1096_c", "key_1097_a", "key_1097_b"] {
            for s in fams[fam]["sizes"].as_array().unwrap() {
                let d = s["d"].as_f64().unwrap();
                for l in s["lengths"].as_array().unwrap() {
                    assert!(l.as_f64().unwrap() < 10.0 * d, "{fam} b={d} 含非法 L {l}");
                }
            }
        }
        // 1097 sizes 必须带 h（GUI 信息行 `b8×h7`；收尾前缺字段，信息行显示 `h?`）。
        assert_eq!(s8["h"], 7.0, "1097 sizes 应带 h 字段");
        for (fam, ty) in [("key_1097_a", KeyType::A), ("key_1097_b", KeyType::B)] {
            for s in fams[fam]["sizes"].as_array().unwrap() {
                let b = s["d"].as_f64().unwrap();
                assert_eq!(
                    s["h"].as_f64().unwrap(),
                    row_1097(ty, b).unwrap().h,
                    "{fam} b={b} sizes.h 应与表行一致"
                );
            }
        }
        // 键型样式表（顶层 key_styles；用户 2026-09-25）：GUI 下拉/约束/信息行的唯一来源。
        let styles = cat["key_styles"].as_array().expect("顶层 key_styles");
        let ids: Vec<&str> = styles.iter().map(|s| s["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["A", "B", "C", "导向A", "导向B"], "键型下拉集合/顺序");
        let labels: Vec<&str> = styles.iter().map(|s| s["label"].as_str().unwrap()).collect();
        assert_eq!(
            labels,
            [
                "普通平键A型",
                "普通平键B型",
                "普通平键C型",
                "导向平键A型",
                "导向平键B型"
            ]
        );
        let g = &styles[3];
        assert_eq!(g["kind"], "A");
        assert_eq!(g["guided"], true);
        assert_eq!(g["standard"], "GB/T 1097-2003");
        assert_eq!(g["places"], serde_json::json!(["mid"]));
        assert_eq!(g["allow_double"], false);
        assert_eq!(styles[0]["places"], serde_json::json!(["mid", "end"]));
        assert_eq!(styles[0]["allow_double"], true);
        // 树：五族都挂在「键/平键」下。
        fn walk(node: &serde_json::Value, out: &mut Vec<String>) {
            if let Some(arr) = node.as_array() {
                for n in arr {
                    walk(n, out);
                }
            }
            if let Some(f) = node.get("family").and_then(|x| x.as_str()) {
                out.push(f.to_string());
            }
            if let Some(k) = node.get("children") {
                walk(k, out);
            }
        }
        let mut on_tree = Vec::new();
        walk(&cat["tree"], &mut on_tree);
        for fam in ["key_1096_a", "key_1096_b", "key_1096_c", "key_1097_a", "key_1097_b"] {
            assert!(on_tree.contains(&fam.to_string()), "{fam} 没挂到零件树");
        }
        assert!(cat.to_string().contains("平键"), "树路径应含「平键」");
    }

    /// 键型样式表（用户 2026-09-25）：合并记号解析 + (型别, 导向) 反查 + 表自洽。
    #[test]
    fn key_style_table_covers_merged_types() {
        assert_eq!(key_style_by_token("导向A").unwrap().id, "导向A");
        assert_eq!(key_style_by_token("GUIDED_A").unwrap().label, "导向平键A型");
        assert_eq!(key_style_by_token("導向B").unwrap().label, "导向平键B型");
        assert_eq!(key_style_by_token("a").unwrap().label, "普通平键A型");
        assert!(key_style_by_token("导向C").is_none(), "GB/T 1097 无 C 型");
        assert!(key_style_by_token("楔形A").is_none(), "未登记键型不得被识别");
        assert!(key_style_of(KeyType::A, true).unwrap().guided);
        assert!(!key_style_of(KeyType::A, false).unwrap().guided);
        assert!(key_style_of(KeyType::C, true).is_none(), "导向 C 无样式");
        for s in KEY_STYLES {
            assert_eq!(key_style_by_token(s.id).unwrap().id, s.id);
            assert_eq!(key_style_of(s.key_type, s.guided).unwrap().id, s.id);
        }
    }

    /// 视图/参数校验与错误路径。
    #[test]
    fn view_and_param_errors() {
        let _g = zh_guard();
        assert_eq!(family_views("key_1096_a"), vec!["main", "top", "section"]);
        assert_eq!(family_views("key_1097_a"), vec!["main", "top"]);
        assert!(family_views("key_9999").is_empty());
        assert!(gen_all("key_1096_a", 2.0, 6.0, "end").unwrap_err().contains("不提供视图"));
        assert!(gen_all("key_1097_a", 8.0, 100.0, "section").unwrap_err().contains("不提供视图"));
        assert!(gen_all("key_1096_a", 7.0, 6.0, "main").unwrap_err().contains("没有 b=7"));
        assert!(gen_all("key_1097_a", 9.0, 70.0, "main").unwrap_err().contains("没有 b=9"));
        assert!(gen_all("key_1097_a", 8.0, 100.0, "main").is_err());
        assert!(gen_all("key_1096_a", 4.0, 8.0, "main").is_ok());
        // 1096 A 型 h 误印：库内取标准值 20/22/25/28，印刷值仅留档。
        let a = table_1096(KeyType::A);
        let last4: Vec<_> = a.rows[a.rows.len() - 4..].iter().collect();
        assert_eq!(last4.iter().map(|r| r.h).collect::<Vec<_>>(), vec![20.0, 22.0, 25.0, 28.0]);
        assert_eq!(last4.iter().map(|r| r.h_printed).collect::<Vec<_>>(), vec![Some(32.0), Some(20.0), Some(22.0), Some(25.0)]);
        assert_eq!(table_1096(KeyType::B).rows[16].h_printed, None);
        assert!(KEY_1096_CHAMFER_RANGES.len() == 7 && type_label(KeyType::C).contains("单圆头"));
    }

    /// SVG 冒烟：样本规格可渲染；剖面线间距按用户口径 3.0mm。
    #[test]
    fn svg_smoke() {
        let _g = zh_guard();
        for (fam, d, l, views) in [
            ("key_1096_a", 4.0, 8.0, vec!["main", "top", "section"]),
            ("key_1096_c", 2.0, 6.0, vec!["main", "top", "section"]),
            ("key_1097_a", 8.0, 70.0, vec!["main", "top"]),
            ("key_1097_b", 45.0, 400.0, vec!["main", "top"]),
        ] {
            for view in views {
                let p = gen_all(fam, d, l, view).unwrap();
                let svg = part_svg(&p, 1200.0, 800.0);
                assert!(svg.contains("<svg"), "{fam}/{view} SVG 空");
                assert!(!svg.contains("error"), "{fam}/{view} SVG 报错");
            }
        }
        for h in gen_all("key_1096_a", 2.0, 6.0, "section").unwrap().entities.iter().filter_map(|e| match e {
            EntityType::Hatch(h) => Some(h),
            _ => None,
        }) {
            assert!(near(hatch_perpendicular_spacing(h.pattern_scale), 3.0), "1096 剖面线间距应为 3.0mm");
        }
        for h in gen_all("key_1097_a", 8.0, 70.0, "main").unwrap().entities.iter().filter_map(|e| match e {
            EntityType::Hatch(h) => Some(h),
            _ => None,
        }) {
            assert!(near(hatch_perpendicular_spacing(h.pattern_scale), 3.0), "1097 剖面线间距应为 3.0mm");
        }
    }
}
