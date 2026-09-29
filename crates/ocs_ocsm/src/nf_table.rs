//! 智能卡片「NF 内花键参数表」（`OCSMCARD`；`CardRenderer::NfTable`）：
//! **13 行 × 2 列表格块几何内建**（照 `~/桌面/GB/参数表/外花键参数表NF.dxf` **同构镜像**，
//! 逐图元反解，不依赖外部 DXF）+ 12 个值属性 + 6 个公差洞位 + CLI/GUI 参数解析。
//!
//! # 为什么是「镜像」而不是另立版面
//!
//! NF E22-141（中文译本扫描 p01–p35）里有内花键的尺寸表（p18）、检查尺寸表（p23–p25）、
//! 槽底圆角（p22/p26）、偏差表（p35），但**没有**“图面参数表”这种版面——用户那份
//! `外花键参数表NF.dxf` 是自定画法。按调研结论，内花键版**同构镜像**外花键模板：
//! 版面几何/图层/行高/行列数全同，只换“量”与标签。
//!
//! 模板实测（调研 §3）：**无块、无 ATTDEF、无 INSERT**；97 图元 = 66 LINE + 25 MTEXT +
//! 6 TEXT；13 行 × 2 列（第 1 行标题跨两列，第 2–13 行 = 左标签 / 右值）；
//! 外框 `x∈[−335.222, 0]`、`y∈[0, −529.901]`，中分隔线 `x=−167.611`；
//! 标签/值 MTEXT 字高 25、attachment=4（左中），公差 TEXT 字高 15、字宽 0.667。
//!
//! 本卡在镜像时的**必要改动**（都被单测锁住）：
//! * **所有文字样式一律 `OCSM_GB`**（用户明确要求；不照抄模板的 `PC_TEXTSTYLE`）；
//! * 模板末行标签 `公法线 W` 起点 `x=−444.95` **出框**（模板自身版面异常）→ 归位 `x=−313.8426`；
//! * 模板无 ATTDEF：12 个值 MTEXT → **12 个值 ATTDEF**（左中，与模板 MTEXT 同位），
//!   标题 + 12 个标签保持静态 MTEXT；6 个公差 TEXT → **6 个公差 ATTDEF**（基线，同位）；
//! * 模板里的 `\C3;` 绿色内联码不再需要（文字本来就在绿色 `6文字层`），文本按 plain text 落图。
//!
//! # 字段与取值（NF 原文符号为准）
//!
//! | 行 | 标签 | 取值 | 出处 |
//! |---|---|---|---|
//! | 3 | 定心方式 | 外径定心（缺省）/ 齿面定心 | p10–p12；p18 表题 |
//! | 4–6 | 模数 m / 齿数 z / 压力角 a | 输入 + 表行核对 | p07 / p18 |
//! | 7 | 齿根样式 | 平齿根（缺省）/ 圆齿根 | p06 / p22 |
//! | 8 | 加工方法 | 拉削 | p18 表题 / p19 |
//! | 9 | 大径 `Az` | 外径定心 `=A`；齿面定心 `=A+0.3m` | p04 / p07 |
//! | 10 | 小径 `D` | `=A−2m` | p04 / p07（与 p18 表 D 列交叉核对）|
//! | 11 | 基准尺寸 Do | `=A` | p07 |
//! | 12 | 量棒直径 `V` | p23–p25 检查表（表外 → `—`） | p25 |
//! | 13 | 跨棒距 `G` | p23–p25 检查表（表外 → `—`） | p25 |
//!
//! **公差（p28 + p29）**：模板里大径/小径/跨棒距三行的上/下偏差洞位均已填值——
//! 大径上/下差 = ISO 286 **R7**（p28 §4：拉削/外径定心的内花键大径公差）；
//! 小径上/下差 = ISO 286 **H7**（p28 §6：内花键小径公差，参考）；
//! 跨棒距上/下差 = p29 检查尺寸的公差值里 **内花键 E** 的偏差（µm→mm）。
//! p29 的 **xm 内花键** 与所选配合（p31/p34：松动/滑动/固定/压，缺省固定）的
//! **外花键 E/xm** 偏差进预览读数；`(m,A)` 不在 p29 或 ISO 档缺时对应格「—」，
//! 不外推——元素（ATTDEF）始终在，可在 CAD 里改写。`ri`（槽底圆角半径，p22）与
//! `x`（变位系数）不进表格（外花键模板也没有这两行），只在预览读数/回执里给。
//!
//! **表外不外推**：`V/V1/G/G1/ri/z` 只从入库表值取（`nf_e22141_dims.csv` /
//! `nf_e22141_check.csv`，逐行带 `source/flags`）；查不到就显示 `—` 并说明原因。
//! `Az/D/Do` 是公式量（p04/p07），任何 `m/A` 都能给（与本卡的标准表口径不冲突）。
//!
//! # 卡面文字随语言（③ 卡面批；`i18n::lang()`）
//!
//! 标题 + 12 个标签 → catalog（`card.nf.title.*` / `card.nf.label.*`），**符号原样**：卡面 = 译名 +
//! （符号非空时）`空格 + 符号`；定心方式/齿根样式/加工方法这类**枚举显示值**随语言（`card.nf.*`），
//! 而**标准号 `NF E22-141`、数值、「—」、ATTDEF tag、块名一律原样**。
//!
//! ★ **标签行是 MTEXT，而 acadrust 的 `MText` 没有实体级 `width_factor`**（只有参考框宽
//! `rectangle_width`，那是折行框不是字宽；宿主渲染 MTEXT 用**文字样式**的字宽，`OCSM_GB` = 0.7）。
//! 英文标签天生比中文长（实测 `Number of teeth z` 真字体宽 291 vs 标签列 146），因此英文（及
//! 模板自带偏窄的 `基准尺寸 Do`）按**内联字宽码** `\W<f>;` 逐条压缩（宿主 MTEXT 解析器支持，
//! 见 `src/entities/text_support.rs` 的 `\W1.5;x` 用例）：有效字宽 = 0.7 × f。
//! **不动坐标/字高/图层/对齐/列宽，也不动全局 `OCSM_GB` 样式**；元素清单照模板不变
//! （66 线 + 13 MTEXT + 18 ATTDEF）。预算表见 [`label_inline_wf`]（脚本
//! `~/桌面/OCSM/review/i18n_检查/card_nf_width_plan.py`，真字体 × 仓库两个保守模型取更紧、留 ≥3%）。
//!
//! 排版口径：主值最多 3 位小数（显示与内部计算分开）；值属性用**实体级**
//! [`VALUE_WIDTH_FACTOR`] 压缩，保证值框不进公差格（几何回归见测试
//! `nf_values_clear_tolerance_column`）。

use ocs_plugin_api::host::acadrust::entities::mtext::AttachmentPoint;
use ocs_plugin_api::host::acadrust::entities::{
    AttributeDefinition, AttributeEntity, Entity as _, EntityCommon, EntityType,
    HorizontalAlignment, Insert, MText, VerticalAlignment,
};
use ocs_plugin_api::host::acadrust::types::{Color, LineWeight, Vector3};

/// 表格块名（本卡只有内花键一种版面）。
pub const BLOCK: &str = "OCSM_NFTABLE_NF_INT";
/// 缺项标记（本仓没有/表外的数据一律显示它；回执与预览点明原因 —— 不臆造）。
pub const MISSING: &str = "—";

/// 值属性**实体级**字宽压缩（不动全局 `OCSM_GB` 样式；值列可用宽 ~78，见几何回归）。
pub const VALUE_WIDTH_FACTOR: f64 = 0.7;
/// 公差属性实体级字宽（照模板 TEXT 的 0.667）。
pub const TOL_WIDTH_FACTOR: f64 = 0.667;

/// 模板标签列 x（模板全部 12 个标签都在这里；末行原为 −444.95，镜像时归位）。
pub const LABEL_X: f64 = -313.8425768498209;
/// 模板值列 x（值 MTEXT 的 attachment=4 插入点）。
pub const VALUE_X: f64 = -146.2317554905194;
/// 标题插入点（attachment=5 正中）。
pub const TITLE_AT: (f64, f64) = (-167.6108213593015, -20.96507352941165);
/// 标题/标签/值字高（模板 25）。
pub const TEXT_H: f64 = 25.0;
/// 公差 TEXT 字高（模板 15）。
pub const TOL_H: f64 = 15.0;
/// 表格外框（左/右/上/下）。
pub const FRAME: (f64, f64, f64, f64) =
    (-335.2216427186031, 0.0, 0.0, -529.9008956621753);
/// 中分隔线 x。
pub const MID_X: f64 = -167.6108213593015;

/// 整表缩放（用户 2026-09-26 截图）：NF 卡用 **INSERT 缩放 0.17**，
/// **不动块内图元几何**；ATTRIB 文字随 INSERT 变换一起缩放（`transform_attribute_entity` 按 X 缩放因子缩放高度）。
pub const TABLE_SCALE: f64 = 0.17;

// ══════════════════════════════════════════════════════════════════════════
// 模板数据（逐图元反解；不依赖外部 DXF）
// ══════════════════════════════════════════════════════════════════════════

/// 模板 `外花键参数表NF.dxf` 的 66 条 LINE（逐图元照录；`(a, b, layer)`）。
/// 内外两卡同版面：本表原样被 `nf_ext_table` 复用（外卡就是模板原版）。
pub(crate) const NF_LINES: &[([f64; 2], [f64; 2], &str)] = &[
    ([-335.2216427186031, 0.0], [-167.6108213593015, 0.0], "1轮廓实线层"),
    ([-167.6108213593015, 0.0], [0.0, 0.0], "1轮廓实线层"),
    ([-335.2216427186031, 0.0], [-335.2216427186031, -41.93014705882336], "1轮廓实线层"),
    ([0.0, 0.0], [0.0, -41.93014705882336], "1轮廓实线层"),
    ([-335.2216427186031, -41.93014705882336], [-167.6108213593015, -41.93014705882336], "2细线层"),
    ([-335.2216427186031, -41.93014705882336], [-335.2216427186031, -82.6214766677619], "1轮廓实线层"),
    ([-167.6108213593015, -41.93014705882336], [0.0, -41.93014705882336], "2细线层"),
    ([-167.6108213593015, -41.93014705882336], [-167.6108213593015, -82.6214766677619], "2细线层"),
    ([0.0, -41.93014705882336], [0.0, -82.6214766677619], "1轮廓实线层"),
    ([-335.2216427186031, -82.6214766677619], [-167.6108213593015, -82.6214766677619], "2细线层"),
    ([-335.2216427186031, -82.6214766677619], [-335.2216427186031, -123.0960018074267], "1轮廓实线层"),
    ([-167.6108213593015, -82.6214766677619], [0.0, -82.6214766677619], "2细线层"),
    ([-167.6108213593015, -82.6214766677619], [-167.6108213593015, -123.0960018074267], "2细线层"),
    ([0.0, -82.6214766677619], [0.0, -123.0960018074267], "1轮廓实线层"),
    ([-335.2216427186031, -123.0960018074267], [-167.6108213593015, -123.0960018074267], "2细线层"),
    ([-335.2216427186031, -123.0960018074267], [-335.2216427186031, -163.7873314163652], "1轮廓实线层"),
    ([-167.6108213593015, -123.0960018074267], [0.0, -123.0960018074267], "2细线层"),
    ([-167.6108213593015, -123.0960018074267], [-167.6108213593015, -163.7873314163652], "2细线层"),
    ([0.0, -123.0960018074267], [0.0, -163.7873314163652], "1轮廓实线层"),
    ([-335.2216427186031, -163.7873314163652], [-167.6108213593015, -163.7873314163652], "2细线层"),
    ([-335.2216427186031, -163.7873314163652], [-335.2216427186031, -204.3702587906671], "1轮廓实线层"),
    ([-167.6108213593015, -163.7873314163652], [0.0, -163.7873314163652], "2细线层"),
    ([-167.6108213593015, -163.7873314163652], [-167.6108213593015, -204.3702587906671], "2细线层"),
    ([0.0, -163.7873314163652], [0.0, -204.3702587906671], "1轮廓实线层"),
    ([-335.2216427186031, -204.3702587906671], [-167.6108213593015, -204.3702587906671], "2细线层"),
    ([-335.2216427186031, -204.3702587906671], [-335.2216427186031, -245.1699906342423], "1轮廓实线层"),
    ([-167.6108213593015, -204.3702587906671], [0.0, -204.3702587906671], "2细线层"),
    ([-167.6108213593015, -204.3702587906671], [-167.6108213593015, -245.1699906342423], "2细线层"),
    ([0.0, -204.3702587906671], [0.0, -245.1699906342423], "1轮廓实线层"),
    ([-335.2216427186031, -245.1699906342423], [-167.6108213593015, -245.1699906342423], "2细线层"),
    ([-335.2216427186031, -245.1699906342423], [-335.2216427186031, -285.7529180085441], "1轮廓实线层"),
    ([-167.6108213593015, -245.1699906342423], [0.0, -245.1699906342423], "2细线层"),
    ([-167.6108213593015, -245.1699906342423], [-167.6108213593015, -285.7529180085441], "2细线层"),
    ([0.0, -245.1699906342423], [0.0, -285.7529180085441], "1轮廓实线层"),
    ([-335.2216427186031, -285.7529180085441], [-167.6108213593015, -285.7529180085441], "2细线层"),
    ([-335.2216427186031, -285.7529180085441], [-335.2216427186031, -326.2274431482089], "1轮廓实线层"),
    ([-167.6108213593015, -285.7529180085441], [0.0, -285.7529180085441], "2细线层"),
    ([-167.6108213593015, -285.7529180085441], [-167.6108213593015, -326.2274431482089], "2细线层"),
    ([0.0, -285.7529180085441], [0.0, -326.2274431482089], "1轮廓实线层"),
    ([-335.2216427186031, -326.2274431482089], [-167.6108213593015, -326.2274431482089], "2细线层"),
    ([-335.2216427186031, -326.2274431482089], [-335.2216427186031, -366.8103705225105], "1轮廓实线层"),
    ([-167.6108213593015, -326.2274431482089], [0.0, -326.2274431482089], "2细线层"),
    ([-167.6108213593015, -326.2274431482089], [-167.6108213593015, -366.8103705225105], "2细线层"),
    ([0.0, -326.2274431482089], [0.0, -366.8103705225105], "1轮廓实线层"),
    ([-335.2216427186031, -366.8103705225105], [-167.6108213593015, -366.8103705225105], "2细线层"),
    ([-335.2216427186031, -366.8103705225105], [-335.2216427186031, -407.1764934275384], "1轮廓实线层"),
    ([-167.6108213593015, -366.8103705225105], [0.0, -366.8103705225105], "2细线层"),
    ([-167.6108213593015, -366.8103705225105], [-167.6108213593015, -407.1764934275384], "2细线层"),
    ([0.0, -366.8103705225105], [0.0, -407.1764934275384], "1轮廓实线层"),
    ([-335.2216427186031, -407.1764934275384], [-167.6108213593015, -407.1764934275384], "2细线层"),
    ([-335.2216427186031, -407.1764934275384], [-335.2216427186031, -447.7594208018401], "1轮廓实线层"),
    ([-167.6108213593015, -407.1764934275384], [0.0, -407.1764934275384], "2细线层"),
    ([-167.6108213593015, -407.1764934275384], [-167.6108213593015, -447.7594208018401], "2细线层"),
    ([0.0, -407.1764934275384], [0.0, -447.7594208018401], "1轮廓实线层"),
    ([-335.2216427186031, -447.7594208018401], [-167.6108213593015, -447.7594208018401], "2细线层"),
    ([-335.2216427186031, -447.7594208018401], [-335.2216427186031, -489.2095660532368], "1轮廓实线层"),
    ([-167.6108213593015, -447.7594208018401], [0.0, -447.7594208018401], "2细线层"),
    ([-167.6108213593015, -447.7594208018401], [-167.6108213593015, -489.2095660532368], "2细线层"),
    ([0.0, -447.7594208018401], [0.0, -489.2095660532368], "1轮廓实线层"),
    ([-335.2216427186031, -489.2095660532368], [-167.6108213593015, -489.2095660532368], "2细线层"),
    ([-335.2216427186031, -489.2095660532368], [-335.2216427186031, -529.9008956621753], "1轮廓实线层"),
    ([-167.6108213593015, -489.2095660532368], [0.0, -489.2095660532368], "2细线层"),
    ([-167.6108213593015, -489.2095660532368], [-167.6108213593015, -529.9008956621753], "2细线层"),
    ([0.0, -489.2095660532368], [0.0, -529.9008956621753], "1轮廓实线层"),
    ([-335.2216427186031, -529.9008956621753], [-167.6108213593015, -529.9008956621753], "1轮廓实线层"),
    ([-167.6108213593015, -529.9008956621753], [0.0, -529.9008956621753], "1轮廓实线层"),
];

/// 12 个内容行：`(标签 catalog key, 符号, 值属性 tag, 行中心 y)`。
///
/// y = 模板各行带（13 行 = 标题 + 12 内容行）的垂直中心，与模板值 MTEXT 逐点一致；
/// 标签 x 一律 [`LABEL_X`]（模板末行 `x=−444.95` 归位）。
/// **标签 = 译名 + 「空格 + 符号」**（符号原样；`card.nf.label.*` 的 zh 值不含空格/符号 ⇒ 与模板逐字相同）。
pub(crate) const NF_ROWS: &[(&str, &str, &'static str, f64)] = &[
    ("card.nf.label.standard", "", "执行标准", -62.27581186329274),
    ("card.nf.label.centering", "", "定心方式", -102.8587392375943),
    ("card.nf.label.module", "m", "模数", -143.4416666118961),
    ("card.nf.label.teeth", "z", "齿数", -184.0787951035162),
    ("card.nf.label.alpha", "a", "压力角", -224.7701247124548),
    ("card.nf.label.root_form", "", "齿根样式", -265.4614543213933),
    ("card.nf.label.machining", "", "加工方法", -305.9901805783765),
    ("card.nf.label.major_dia", "Az", "大径Az", -346.5189068353597),
    ("card.nf.label.minor_dia", "D", "小径D", -386.9934319750245),
    ("card.nf.label.base_size", "Do", "基准尺寸", -427.4679571146893),
    ("card.nf.label.pin_dia", "V", "量棒直径V", -468.4844934275385),
    ("card.nf.label.over_pins", "G", "跨棒距G", -509.5552308577061),
];

/// 6 个公差洞位：`(tag, 插入 x, 基线 y)`，照模板 TEXT 逐点（字高 15、字宽 0.667）。
/// 末对（跨棒距上/下差）在用户截图后**右移 2 个字符宽**，见 [`TOL_X_SHIFT`]。
const NF_TOL_ATTS: &[(&str, f64, f64)] = &[
    ("大径上差", -79.67949597123788, -342.7153021054078),
    ("大径下差", -79.90685200460302, -362.9124102114762),
    ("小径上差", -87.69659398855129, -384.3053261490311),
    ("小径下差", -87.92395002191643, -404.5024342550995),
    ("跨棒距上差", -68.34721762208392, -505.6563921767285),
    ("跨棒距下差", -68.57457365544906, -525.8535002827967),
];

/// 公差实体级右移量（用户 2026-09-26 截图）：NF 末对公差（跨棒距/公法线）右移 2 个字符宽。
/// 口径照既有 `spline_table::TOL_X_SHIFT`：**一个字符宽 = 字高 × 该实体字宽因子**，
/// 这里是 2 个 → `2 × 15 × 0.667 = 20.01`。不动全局 `OCSM_GB` 样式。
pub const TOL_X_SHIFT: f64 = 2.0 * TOL_H * TOL_WIDTH_FACTOR;

/// 12 个值属性 tag（自上而下的行序；`values()` 按本序返回）。
pub const VALUE_TAGS: &[&str] = &[
    "执行标准",
    "定心方式",
    "模数",
    "齿数",
    "压力角",
    "齿根样式",
    "加工方法",
    "大径Az",
    "小径D",
    "基准尺寸",
    "量棒直径V",
    "跨棒距G",
];

/// 6 个公差属性 tag（大径/小径/跨棒距 的 上/下）。
pub const TOL_TAGS: &[&str] = &[
    "大径上差",
    "大径下差",
    "小径上差",
    "小径下差",
    "跨棒距上差",
    "跨棒距下差",
];

// ══════════════════════════════════════════════════════════════════════════
// NF E22-141 公差（p28 直径公差 + p29 E/xm 偏差表）
// ══════════════════════════════════════════════════════════════════════════

/// 配合类别（NF E22-141 中文译本 p31「1=松动配合 2=滑动配合 3=固定配合 4=压配合」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitClass {
    Loose,
    Slide,
    Fixed,
    Press,
}

impl FitClass {
    pub fn id(self) -> &'static str {
        match self {
            FitClass::Loose => "loose",
            FitClass::Slide => "slide",
            FitClass::Fixed => "fixed",
            FitClass::Press => "press",
        }
    }
    /// 显示名（随语言；catalog `gui.fld.nf_table.form.fit.opt.*`，NF 内/外共用）。
    pub fn label(self) -> String {
        crate::i18n::t(match self {
            FitClass::Loose => "gui.fld.nf_table.form.fit.opt.loose",
            FitClass::Slide => "gui.fld.nf_table.form.fit.opt.slide",
            FitClass::Fixed => "gui.fld.nf_table.form.fit.opt.fixed",
            FitClass::Press => "gui.fld.nf_table.form.fit.opt.press",
        })
    }
    pub fn index(self) -> usize {
        match self {
            FitClass::Loose => 0,
            FitClass::Slide => 1,
            FitClass::Fixed => 2,
            FitClass::Press => 3,
        }
    }
    /// 命令/GUI token（宽松收词）。
    pub fn from_token(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "松动" | "松" | "loose" | "l" => Some(FitClass::Loose),
            "滑动" | "滑" | "slide" | "sliding" | "s" => Some(FitClass::Slide),
            "固定" | "固" | "fixed" | "fix" | "f" => Some(FitClass::Fixed),
            "压" | "压配合" | "press" | "p" => Some(FitClass::Press),
            _ => None,
        }
    }
    pub const ALL: [FitClass; 4] = [
        FitClass::Loose,
        FitClass::Slide,
        FitClass::Fixed,
        FitClass::Press,
    ];
}

impl Default for FitClass {
    /// 缺省「固定」：模板公法线公差 +0.055/−0.042 的量级/正负最接近 p29 固定配合列（+42/−42）。
    fn default() -> Self {
        FitClass::Fixed
    }
}

/// 上/下偏差对（微米，NF 原文单位）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DevPair {
    pub upper: f64,
    pub lower: f64,
}

impl DevPair {
    /// `"+25/0"` → DevPair；解析失败 `None`。
    pub fn parse(s: &str) -> Option<Self> {
        let (u, l) = s.trim().split_once('/')?;
        Some(DevPair {
            upper: u.trim().parse().ok()?,
            lower: l.trim().parse().ok()?,
        })
    }
}

/// p29 一行（m + A 范围/列表 + E/xm 内花键 + E/xm 外花键 4 配合，单位微米）。
#[derive(Debug, Clone, PartialEq)]
pub struct NfDevRow {
    pub m: f64,
    /// 原文 A 范围/列表（如 `4~15` / `110,120,130`）。
    pub a_spec: String,
    pub e_int: DevPair,
    pub e_ext: [DevPair; 4],
    pub xm_int: DevPair,
    pub xm_ext: [DevPair; 4],
    pub source: String,
    pub raw: String,
    pub note: String,
}

impl NfDevRow {
    /// A 是否落在本行（范围 `lo~hi` 或逗号列表；容差 1e-9）。
    pub fn contains_a(&self, a: f64) -> bool {
        let s = self.a_spec.replace('~', "-");
        if s.contains('-') {
            let mut it = s.split('-').filter_map(|x| x.trim().parse::<f64>().ok());
            if let (Some(lo), Some(hi)) = (it.next(), it.next()) {
                return a >= lo - 1e-9 && a <= hi + 1e-9;
            }
        }
        s.split(',')
            .filter_map(|x| x.trim().parse::<f64>().ok())
            .any(|v| (v - a).abs() < 1e-9)
    }
    pub fn e_ext_for(&self, fit: FitClass) -> DevPair {
        self.e_ext[fit.index()]
    }
    pub fn xm_ext_for(&self, fit: FitClass) -> DevPair {
        self.xm_ext[fit.index()]
    }
}

/// 解析 `assets/nf_e22141_e_xm_tol.csv`（14 行；逐行 source/raw/note，include_str! 编译进插件）。
fn parse_dev_rows(text: &str) -> Vec<NfDevRow> {
    fn split_csv(line: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut quoted = false;
        for ch in line.chars() {
            match ch {
                '"' => quoted = !quoted,
                ',' if !quoted => out.push(std::mem::take(&mut cur)),
                _ => cur.push(ch),
            }
        }
        out.push(cur);
        out
    }
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() || line.starts_with('#') || line.starts_with("m,") {
            continue;
        }
        let f = split_csv(line);
        if f.len() < 15 {
            continue;
        }
        let pair = |i: usize| DevPair::parse(&f[i]);
        let Some(m) = f[0].trim().parse::<f64>().ok() else {
            continue;
        };
        let (Some(ei), Some(xi)) = (pair(2), pair(7)) else {
            continue;
        };
        let mut e_ext = [DevPair { upper: 0.0, lower: 0.0 }; 4];
        let mut xm_ext = [DevPair { upper: 0.0, lower: 0.0 }; 4];
        let mut ok = true;
        for k in 0..4 {
            match (pair(3 + k), pair(8 + k)) {
                (Some(a), Some(b)) => {
                    e_ext[k] = a;
                    xm_ext[k] = b;
                }
                _ => ok = false,
            }
        }
        if !ok {
            continue;
        }
        rows.push(NfDevRow {
            m,
            a_spec: f[1].trim().to_string(),
            e_int: ei,
            e_ext,
            xm_int: xi,
            xm_ext,
            source: f[12].clone(),
            raw: f[13].clone(),
            note: f[14].clone(),
        });
    }
    rows
}

/// p29 偏差表（惰性解析一次）。
pub fn e_xm_tol_rows() -> &'static [NfDevRow] {
    static ROWS: std::sync::OnceLock<Vec<NfDevRow>> = std::sync::OnceLock::new();
    ROWS.get_or_init(|| {
        parse_dev_rows(include_str!("../assets/nf_e22141_e_xm_tol.csv"))
    })
}

/// 该 (m, A) 命中的 p29 行（表外 `None` → 内花键 E 偏差标缺，不外推）。
pub fn e_xm_tol_row(m: f64, a: f64) -> Option<&'static NfDevRow> {
    e_xm_tol_rows()
        .iter()
        .find(|r| (r.m - m).abs() < 1e-9 && r.contains_a(a))
}

/// 微米偏差（µm）→ mm 显示串（带符号，最多 3 位小数；0 → `0`）。
pub(crate) fn fmt_um_mm(v_um: f64) -> String {
    let v = v_um / 1000.0;
    if v.abs() < 5e-7 {
        return "0".to_string();
    }
    let s = format!("{v:+.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "+" || s == "-" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 文字/属性实体构造
// ══════════════════════════════════════════════════════════════════════════

fn common_of(layer: &str) -> EntityCommon {
    let mut c = EntityCommon::default();
    c.layer = layer.to_string();
    c.color = Color::ByLayer;
    c.linetype = "ByLayer".into();
    c
}

/// 卡面取词：`card.` 前缀 = catalog key（按当前语言）；其余原样（符号/协议记号）。
pub(crate) fn face_text(key: &str) -> String {
    if key.starts_with("card.") {
        crate::i18n::t(key)
    } else {
        key.to_string()
    }
}

/// 卡面标签文字 = 译名 +（符号非空时）`空格 + 符号`；**符号两语原样**。
pub(crate) fn face_label(key: &str, symbol: &str) -> String {
    let text = face_text(key);
    if symbol.is_empty() {
        text
    } else {
        format!("{text} {symbol}")
    }
}

/// 标签/标题 MTEXT 的**内联字宽因子表**（`\W<f>;`；有效字宽 = 0.7 × f）。
///
/// `None` = 不压缩（卡面文字与模板逐字相同）。数字来源：`card_nf_width_plan.py`
/// （真字体 advance × cap 0.637 归一 / 仓库 `char_em` / 仓库 `char_em_ttf` **三模型取更紧者**，
/// 再留 ≥3% 列内余量）；标签列可用 = `MID_X − LABEL_X − 0.5` = 145.73。
/// 压缩只发生在**装不下的那一条**上（中文只有模板本就偏宽的 `基准尺寸 Do`），不动列宽/线位/样式。
pub(crate) fn label_inline_wf(key: &str, lang: crate::i18n::Lang) -> Option<f64> {
    use crate::i18n::Lang;
    let table: &[(&str, f64, f64)] = &[
        // (key, zh 的 W, en 的 W)；1.0 = 不压缩
        ("card.nf.label.teeth", 1.0, 0.66),
        ("card.nf.label.alpha", 1.0, 0.70),
        ("card.nf.label.machining", 1.0, 0.62),
        ("card.nf.label.major_dia", 1.0, 0.60),
        ("card.nf.label.minor_dia", 1.0, 0.60),
        ("card.nf.label.base_size", 0.92, 0.89),
        ("card.nf.label.pin_dia", 1.0, 0.80),
        ("card.nf.label.over_pins", 1.0, 0.45),
        ("card.nf.label.span_teeth", 1.0, 0.95),
        ("card.nf.label.base_tangent", 1.0, 0.52),
    ];
    let (_, zh, en) = table.iter().find(|(k, _, _)| *k == key)?;
    let w = if lang == Lang::En { *en } else { *zh };
    (w < 0.995).then_some(w)
}

/// 标签/标题 MTEXT：模板是 `attachment=4`（标题 5），字高 25，框宽照模板 320.2216。
/// 装不下的标签在**值首**加内联字宽码 `\W<f>;`（见 [`label_inline_wf`]；宿主按 0.7 × f 渲染）。
pub(crate) fn mtext_ent(value: &str, x: f64, y: f64, attach: i16) -> EntityType {
    let mut m = MText::new();
    m.value = value.to_string();
    m.insertion_point = Vector3::new(x, y, 0.0);
    m.height = TEXT_H;
    m.rectangle_width = 320.2216427186032;
    m.style = "OCSM_GB".into();
    m.attachment_point = match attach {
        5 => AttachmentPoint::MiddleCenter,
        _ => AttachmentPoint::MiddleLeft,
    };
    m.common = common_of("6文字层");
    EntityType::MText(m)
}

/// 带内联字宽码的 MTEXT：`\W0.66;Number of teeth z`（`wf` = 内联因子，有效字宽 = 0.7 × wf）。
pub(crate) fn mtext_with_inline_wf(
    text: &str,
    wf: f64,
    x: f64,
    y: f64,
    attach: i16,
) -> EntityType {
    mtext_ent(&format!("\\W{wf:.2};{text}"), x, y, attach)
}

/// 标签 MTEXT（按当前语言取词 + 必要时内联压缩）。
pub(crate) fn label_mtext(key: &str, symbol: &str, x: f64, y: f64) -> EntityType {
    let text = face_label(key, symbol);
    match label_inline_wf(key, crate::i18n::lang()) {
        Some(wf) => mtext_with_inline_wf(&text, wf, x, y, 4),
        None => mtext_ent(&text, x, y, 4),
    }
}

/// 值属性：左中（与模板值 MTEXT 的 attachment=4 同语义），实体级字宽 [`VALUE_WIDTH_FACTOR`]
/// （英文长文字值再按 [`value_width_factor`] 压一档）。
pub(crate) fn value_attdef_for(tag: &str, x: f64, y: f64, base_wf: f64) -> AttributeDefinition {
    value_attdef_wf(
        tag,
        x,
        y,
        value_width_factor(tag, base_wf, crate::i18n::lang()),
    )
}

/// 值属性（指定实体级字宽；外花键卡最长值 `279.795` 要比内卡更窄一档）。
pub(crate) fn value_attdef_wf(
    tag: &str,
    x: f64,
    y: f64,
    width_factor: f64,
) -> AttributeDefinition {
    let mut ad = AttributeDefinition::new(tag.to_string(), String::new(), " ".to_string());
    ad.insertion_point = Vector3::new(x, y, 0.0);
    ad.alignment_point = ad.insertion_point;
    ad.height = TEXT_H;
    ad.width_factor = width_factor;
    ad.text_style = "OCSM_GB".into();
    ad.horizontal_alignment = HorizontalAlignment::Left;
    ad.vertical_alignment = VerticalAlignment::Middle;
    ad.flags.preset = true;
    ad.common = common_of("6文字层");
    ad
}

/// 值 ATTDEF 的**实体级字宽**：默认 [`base`]（内卡 0.7 / 外卡 0.6），英文长文字值再压一档。
///
/// 三个文字值（定心方式 / 齿根样式 / 加工方法）英文比中文长得多（`Major dia. centering`
/// 真字体 328 > 值列 145.73）；预算同标签（`card_nf_width_plan.py`，三模型取更紧、≥3% 余量），
/// 中文侧保持原字宽。
pub(crate) fn value_width_factor(tag: &str, base: f64, lang: crate::i18n::Lang) -> f64 {
    if lang != crate::i18n::Lang::En {
        return base;
    }
    match tag {
        "定心方式" => base.min(0.40),
        _ => base,
    }
}

/// 公差属性：照模板 TEXT 基线左对齐（halign/valign=0），字高 15、字宽 0.667。
pub(crate) fn tol_attdef(tag: &str, x: f64, y: f64) -> AttributeDefinition {
    let mut ad = AttributeDefinition::new(tag.to_string(), String::new(), " ".to_string());
    ad.insertion_point = Vector3::new(x, y, 0.0);
    ad.alignment_point = ad.insertion_point;
    ad.height = TOL_H;
    ad.width_factor = TOL_WIDTH_FACTOR;
    ad.text_style = "OCSM_GB".into();
    ad.horizontal_alignment = HorizontalAlignment::Left;
    ad.vertical_alignment = VerticalAlignment::Baseline;
    ad.flags.preset = true;
    ad.common = common_of("6文字层");
    ad
}

/// 表格块成员：**66 线 + 13 MTEXT（标题 + 12 标签）+ 18 ATTDEF**（12 值 + 6 公差）。
pub fn block_entities() -> Vec<EntityType> {
    let mut out =
        Vec::with_capacity(NF_LINES.len() + 1 + NF_ROWS.len() + VALUE_TAGS.len() + TOL_TAGS.len());
    for (a, b, layer) in NF_LINES {
        out.push(crate::partgen_kit::line(*a, *b, layer));
    }
    out.push(mtext_ent(
        &face_text("card.nf.title.int"),
        TITLE_AT.0,
        TITLE_AT.1,
        5,
    ));
    for (key, symbol, _, y) in NF_ROWS {
        out.push(label_mtext(key, symbol, LABEL_X, *y));
    }
    for ad in attdefs() {
        out.push(EntityType::AttributeDefinition(ad));
    }
    out
}

/// 18 个 ATTDEF：先 12 个值（行序），再 6 个公差（大径/小径/跨棒距 上/下）。
pub fn attdefs() -> Vec<AttributeDefinition> {
    let mut out = Vec::with_capacity(VALUE_TAGS.len() + TOL_TAGS.len());
    for (_, _, tag, y) in NF_ROWS {
        out.push(value_attdef_for(tag, VALUE_X, *y, VALUE_WIDTH_FACTOR));
    }
    for (i, (tag, x, y)) in NF_TOL_ATTS.iter().enumerate() {
        // 末对（跨棒距上/下差）右移 2 个字符宽（用户截图；位置口径 = TOL_H × TOL_WIDTH_FACTOR）。
        let x = if i >= 4 { x + TOL_X_SHIFT } else { *x };
        out.push(tol_attdef(tag, x, *y));
    }
    out
}

// ══════════════════════════════════════════════════════════════════════════
// 字段口径（定心方式 / 齿根样式）
// ══════════════════════════════════════════════════════════════════════════

/// 定心方式：外径定心 `Az=A`（缺省；p18 表口径）/ 拉削+齿面定心 `Az=A+0.3m`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Centering {
    Outer,
    Flank,
}

impl Centering {
    pub fn id(self) -> &'static str {
        match self {
            Centering::Outer => "outer",
            Centering::Flank => "flank",
        }
    }
    /// 卡面取值（枚举显示值**随语言**；键 `card.nf.centering.*`）。
    pub fn label(self) -> String {
        crate::i18n::t(match self {
            Centering::Outer => "card.nf.centering.outer",
            Centering::Flank => "card.nf.centering.flank",
        })
    }
    /// 命令/GUI token（宽松收词；`None` = 不认识）。
    pub fn from_token(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "外径" | "外径定心" | "外" | "outer" | "outside" | "od" | "major" => {
                Some(Centering::Outer)
            }
            "齿面" | "齿面定心" | "齿形" | "齿形定心" | "齿侧" | "flank" | "face" | "side" => {
                Some(Centering::Flank)
            }
            _ => None,
        }
    }
}

/// 齿根样式（行 7 文本；NF 内花键对应槽底圆角 `ri` 在预览读数里给）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootStyle {
    Flat,
    Fillet,
}

impl RootStyle {
    pub fn id(self) -> &'static str {
        match self {
            RootStyle::Flat => "flat",
            RootStyle::Fillet => "fillet",
        }
    }
    /// 卡面取值（枚举显示值**随语言**；键 `card.nf.root.*`）。
    pub fn label(self) -> String {
        crate::i18n::t(match self {
            RootStyle::Flat => "card.nf.root.flat",
            RootStyle::Fillet => "card.nf.root.fillet",
        })
    }
    pub fn from_token(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "平" | "平齿根" | "平根" | "flat" | "flat_root" | "straight" => Some(RootStyle::Flat),
            "圆" | "圆齿根" | "圆根" | "fillet" | "round" | "round_root" => {
                Some(RootStyle::Fillet)
            }
            _ => None,
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 取值（查 `invol_spline` 已入库的 NF E22-141 表；表外不外推）
// ══════════════════════════════════════════════════════════════════════════

/// 「NF 内花键参数表」卡参数（CLI 与 GUI 同一份字段）。
#[derive(Debug, Clone, PartialEq)]
pub struct NfTableSpec {
    /// 公称直径 `A`（NF 主参数）。
    pub a: f64,
    /// 模数 `m`。
    pub m: f64,
    /// 齿数 `z`（选填；给了要和 p18 表行的 N 核对）。
    pub z: Option<u32>,
    /// 定心方式（缺省 = 外径定心）。
    pub centering: Centering,
    /// 齿根样式（缺省 = 平齿根）。
    pub root: RootStyle,
    /// 配合类别（p31/p34 四档；只影响预览里配对外花键的偏差读数）。
    pub fit: FitClass,
    pub at: Option<[f64; 2]>,
    pub rot: f64,
}

/// 查表 + 公式的中间结果（测试与预览共用；不直接落图）。
#[derive(Debug, Clone, PartialEq)]
pub struct NfDerived {
    /// 齿数（输入优先，否则取表值；都无 = `None`）。
    pub z: Option<u32>,
    /// 大径 `Az`（公式）。
    pub az: f64,
    /// 小径 `D = A − 2m`（公式）。
    pub d: f64,
    /// 内花键量棒直径 `V`（p23–p25；表外 `None`）。
    pub v: Option<f64>,
    /// 内花键量棒削边 `V1`（p23–p25）。
    pub v1: Option<f64>,
    /// 内花键量棒跨距 `G`（p23–p25）。
    pub g: Option<f64>,
    /// 内花键量棒跨距 `G1`（p23–p25）。
    pub g1: Option<f64>,
    /// 槽底圆角半径 `ri`（p22/p26 设计尺寸表）。
    pub ri: Option<f64>,
    /// 变位系数 `x`（p21/p22 设计尺寸表）。
    pub x: Option<f64>,
    /// p18 表里的小径 D（用于交叉核对公式值；表外 `None`）。
    pub table_d: Option<f64>,
    /// p18 行来源说明（`—` + 原因 = 表外）。
    pub dims_source: String,
    /// p22/p21 行来源说明（ri/x 的出处）。
    pub detail_source: String,
    /// p23–p25 行来源说明（V/G 的出处）。
    pub check_source: String,
    /// p29 偏差行（E/xm；表外 `None` → 跨棒距公差标缺）。
    pub tol_row: Option<&'static NfDevRow>,
    /// 内花键大径公差（ISO 286 R7，NF E22-141 p28 §4）。
    pub major_tol: Option<crate::tolerance::Limits>,
    /// 内花键小径公差（ISO 286 H7，NF E22-141 p28 §6，参考）。
    pub minor_tol: Option<crate::tolerance::Limits>,
    /// 大径公差来源说明（含失败原因）。
    pub major_tol_note: String,
    /// 小径公差来源说明。
    pub minor_tol_note: String,
}

/// 主参数合法性（`z` 给了就限 3..=1000，与 NF 表量级一致）。
fn validate_basic(a: f64, m: f64, z: Option<u32>) -> Result<(), String> {
    if !(a.is_finite() && a > 0.0) {
        return Err(crate::i18n::t_fmt(
            "cmd.nf.err.a_positive",
            &[("a", &crate::partgen_kit::trim(a))],
        ));
    }
    if !(m.is_finite() && m > 0.0) {
        return Err(crate::i18n::t_fmt(
            "cmd.nf.err.m_positive",
            &[("m", &crate::partgen_kit::trim(m))],
        ));
    }
    if let Some(z) = z {
        if !(3..=1000).contains(&z) {
            return Err(crate::i18n::t_fmt(
                "cmd.nf.err.z_range",
                &[("z", &z.to_string())],
            ));
        }
    }
    Ok(())
}

/// p18 行来源串（`p18R 拉削内花键(外径定心)（source=p18R）`）。
fn dims_source_note(r: &crate::invol_spline::NfE22141Row) -> String {
    format!("p{} {}（source={}）", r.page, r.table_no, r.source)
}

/// 查表 + 公式求值；校验 `z` 与 p18 表值一致（标准件口径）。
pub fn derive(spec: &NfTableSpec) -> Result<NfDerived, String> {
    validate_basic(spec.a, spec.m, spec.z)?;
    let m_tol = 1e-9;
    let a_tol = 1e-3;
    // ① p18：内花键尺寸行（m+A 联合定位，A 在 NF 表里不唯一，必须带 m）。
    let dims = crate::invol_spline::nf_e22141_rows()
        .iter()
        .find(|r| {
            r.page == 18
                && (r.m - spec.m).abs() < m_tol
                && (r.a - spec.a).abs() < a_tol
        })
        .cloned();
    // ② p21/p22：设计尺寸行（x / ri）；齿数也要对上（表里同 (m,A) 只一行）。
    let detail = crate::invol_spline::nf_e22141_rows()
        .iter()
        .find(|r| {
            matches!(r.page, 21 | 22)
                && (r.m - spec.m).abs() < m_tol
                && (r.a - spec.a).abs() < a_tol
                && spec.z.map(|z| r.z == z).unwrap_or(true)
        })
        .cloned();
    // ③ p23–p25：检查表行（内花键 V/V1/G/G1）。按 (A, m) 取，只收检查尺寸表页。
    let check = crate::invol_spline::nf_check_by_a_m(spec.a, spec.m)
        .into_iter()
        .find(|r| matches!(r.page, 23..=25) && r.has_check_dims());
    // ④ z 交叉核对：输入与表值不一致直接拦下（标准件表口径，避免 V/G 取错行）。
    let table_z = dims
        .as_ref()
        .map(|r| r.z)
        .or_else(|| check.as_ref().and_then(|r| r.n));
    if let (Some(z_in), Some(z_tab)) = (spec.z, table_z) {
        if z_in != z_tab {
            return Err(crate::i18n::t_fmt(
                "cmd.nf.err.z_table_mismatch",
                &[
                    ("z_in", &z_in.to_string()),
                    ("a", &crate::partgen_kit::trim(spec.a)),
                    ("m", &crate::partgen_kit::trim(spec.m)),
                    ("z_tab", &z_tab.to_string()),
                ],
            ));
        }
    }
    let z = spec.z.or(table_z);
    let az = match spec.centering {
        Centering::Outer => spec.a,
        Centering::Flank => spec.a + 0.3 * spec.m,
    };
    let d = spec.a - 2.0 * spec.m;
    // ⑤ p28 直径公差（内花键大径 R7、小径 H7 参考）+ p29 E/xm 偏差行。
    let major_tol = crate::tolerance::hole(az, "R7");
    let minor_tol = crate::tolerance::hole(d, "H7");
    let major_tol_note = match major_tol {
        Some(_) => crate::i18n::t("gui.nf.tol.major_note"),
        None => crate::i18n::t_fmt("gui.nf.tol.major_none", &[("d", &fmt_mm(az))]),
    };
    let minor_tol_note = match minor_tol {
        Some(_) => crate::i18n::t("gui.nf.tol.minor_note"),
        None => crate::i18n::t_fmt("gui.nf.tol.minor_none", &[("d", &fmt_mm(d))]),
    };
    let tol_row = e_xm_tol_row(spec.m, spec.a);
    Ok(NfDerived {
        z,
        az,
        d,
        v: check.as_ref().and_then(|r| r.v),
        v1: check.as_ref().and_then(|r| r.v1),
        g: check.as_ref().and_then(|r| r.g),
        g1: check.as_ref().and_then(|r| r.g1),
        ri: detail.as_ref().and_then(|r| r.hub_fillet),
        x: detail.as_ref().and_then(|r| r.x),
        table_d: dims.as_ref().and_then(|r| r.internal_tip),
        dims_source: dims
            .as_ref()
            .map(dims_source_note)
            .unwrap_or_else(|| "—（该 (m, A) 不在 p18 拉削内花键表内）".to_string()),
        detail_source: detail
            .as_ref()
            .map(dims_source_note)
            .unwrap_or_else(|| "—（该 (m, A) 不在 p21/p22 设计尺寸表内）".to_string()),
        check_source: check
            .as_ref()
            .map(|r| r.source_note())
            .unwrap_or_else(|| "—（该 (m, A) 不在 p23–p25 检查尺寸表内）".to_string()),
        tol_row,
        major_tol,
        minor_tol,
        major_tol_note,
        minor_tol_note,
    })
}

/// 最多 3 位小数（显示与内部计算分开）。
pub fn fmt_mm(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// ISO 286 `Limits` → 上/下差显示（缺档 → [`MISSING`]）。
pub(crate) fn tol_display(lim: &Option<crate::tolerance::Limits>, upper: bool) -> String {
    match lim {
        Some(l) => {
            let (u, lo) = l.display();
            if upper {
                u
            } else {
                lo
            }
        }
        None => MISSING.to_string(),
    }
}

/// 18 项取值（顺序 = `attdefs()`：12 值 + 6 公差）。
///
/// 公差口径（NF E22-141）：
/// * 大径上/下差 = ISO 286 **R7**（p28 §4：拉削/外径定心的内花键大径公差）；
/// * 小径上/下差 = ISO 286 **H7**（p28 §6：小径公差，参考）；
/// * 跨棒距上/下差 = p29 **内花键 E 的偏差**（µm→mm；表题「齿公法线长度公差」）。
/// 查不到（(m,A) 不在 p29、ISO 档缺）只对应格「—」，其余照给。
pub fn values(spec: &NfTableSpec) -> Result<Vec<(String, String)>, String> {
    let d = derive(spec)?;
    let out: Vec<(&str, String)> = vec![
        ("执行标准", "NF E22-141".to_string()),
        ("定心方式", spec.centering.label()),
        ("模数", fmt_mm(spec.m)),
        (
            "齿数",
            d.z.map(|z| z.to_string()).unwrap_or_else(|| MISSING.to_string()),
        ),
        ("压力角", "20°".to_string()),
        ("齿根样式", spec.root.label()),
        ("加工方法", crate::i18n::t("card.nf.machining.broach")),
        ("大径Az", fmt_mm(d.az)),
        ("小径D", fmt_mm(d.d)),
        ("基准尺寸", fmt_mm(spec.a)),
        (
            "量棒直径V",
            d.v.map(fmt_mm).unwrap_or_else(|| MISSING.to_string()),
        ),
        (
            "跨棒距G",
            d.g.map(fmt_mm).unwrap_or_else(|| MISSING.to_string()),
        ),
        ("大径上差", tol_display(&d.major_tol, true)),
        ("大径下差", tol_display(&d.major_tol, false)),
        ("小径上差", tol_display(&d.minor_tol, true)),
        ("小径下差", tol_display(&d.minor_tol, false)),
        (
            "跨棒距上差",
            d.tol_row
                .map(|r| fmt_um_mm(r.e_int.upper))
                .unwrap_or_else(|| MISSING.to_string()),
        ),
        (
            "跨棒距下差",
            d.tol_row
                .map(|r| fmt_um_mm(r.e_int.lower))
                .unwrap_or_else(|| MISSING.to_string()),
        ),
    ];
    // 顺序护栏：取值顺序必须与 ATTDEF 表一致（加/改行时先在这里暴露）。
    let got: Vec<&str> = out.iter().map(|(t, _)| *t).collect();
    let want: Vec<String> = attdefs().iter().map(|ad| ad.tag.clone()).collect();
    let want: Vec<&str> = want.iter().map(|s| s.as_str()).collect();
    if got != want {
        return Err(crate::i18n::t_fmt(
            "cmd.nf.err.tag_order",
            &[("got", &format!("{got:?}")), ("want", &format!("{want:?}"))],
        ));
    }
    Ok(out.into_iter().map(|(t, v)| (t.to_string(), v)).collect())
}

/// 建 INSERT（基点在 `at`，旋转 `rot_deg` 度；18 个 ATTRIB 取自 `values()`）。
pub fn build_insert(spec: &NfTableSpec, at: [f64; 2], rot_deg: f64) -> Result<Insert, String> {
    let vals = values(spec)?;
    let mut ins = Insert::new(BLOCK, Vector3::new(at[0], at[1], 0.0));
    ins.rotation = rot_deg.to_radians();
    // 整表缩放（不改块几何；ATTRIB 随 INSERT 一起缩放）。
    ins.set_x_scale(TABLE_SCALE);
    ins.set_y_scale(TABLE_SCALE);
    ins.set_z_scale(TABLE_SCALE);
    {
        let c = &mut ins.common;
        c.layer = crate::partgen::LAYER_MAIN.to_string();
        c.color = Color::ByLayer;
        c.linetype = "ByLayer".to_string();
        c.line_weight = LineWeight::ByLayer;
    }
    for ad in attdefs() {
        let val = vals
            .iter()
            .find(|(tag, _)| tag == &ad.tag)
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        let mut tmpl = ad.clone();
        tmpl.rotation = 0.0; // 旋转由 INSERT 变换施加
        let mut attr = AttributeEntity::from_definition(&tmpl, Some(val));
        attr.apply_transform(&ins.get_transform());
        ins.attributes.push(attr);
    }
    Ok(ins)
}

/// 18 项 Markdown（回执/计算书用；与表同一份取值）。
pub fn markdown_table(spec: &NfTableSpec) -> Result<String, String> {
    let vals = values(spec)?;
    let mut md = String::new();
    md.push_str("| 属性 | 值 |\n|---|---|\n");
    for (tag, v) in vals {
        md.push_str(&format!("| {tag} | {v} |\n"));
    }
    Ok(md)
}

// ══════════════════════════════════════════════════════════════════════════
// GUI 选项表（18 项口径）与表单模型（CLI/GUI/HTTP 同一份字段）
// ══════════════════════════════════════════════════════════════════════════

/// 一项属性的取值口径（进原生 `title=`，不做常显）。
///
/// ★ 本批 GUI 元数据批：`label/formula/source` 由中文字面量改成 catalog key（随语言）；
/// 取词走 [`NfColumnSpec::label`] / [`NfColumnSpec::formula`] / [`NfColumnSpec::source`]。
/// 列名里的符号（`Az`/`D`/`Do`/`V`/`G`/`m`/`z`/`a`）两语原样。
#[derive(Debug, Clone, Copy)]
pub struct NfColumnSpec {
    /// ATTDEF tag（协议记号，不译）。
    pub tag: &'static str,
    /// 列名 key（`gui.nf.col.*`）。
    pub label_key: &'static str,
    pub unit: &'static str,
    /// 取值口径 key（`gui.nf.formula.*`）。
    pub formula_key: &'static str,
    /// 来源 key（`gui.nf.source.*`）。
    pub source_key: &'static str,
}

impl NfColumnSpec {
    /// 列名（按当前语言）。
    pub fn label(&self) -> String {
        crate::i18n::t(self.label_key)
    }
    /// 取值口径（按当前语言）。
    pub fn formula(&self) -> String {
        crate::i18n::t(self.formula_key)
    }
    /// 来源（按当前语言）。
    pub fn source(&self) -> String {
        crate::i18n::t(self.source_key)
    }
}

// ★ 来源串已入 catalog（本批）：`gui.nf.source.*`（dims/check/major_tol/minor_tol/over_pins_tol）——
//   原来的 `SOURCE_*` 中文常量已删（catalog 是唯一来源，避免两份文案漂移）。

/// 18 项口径（顺序 = `attdefs()`）。
pub const NF_COLUMNS: &[NfColumnSpec] = &[
    NfColumnSpec { tag: "执行标准", label_key: "gui.nf.col.standard", unit: "", formula_key: "gui.nf.formula.standard", source_key: "gui.nf.source.standard" },
    NfColumnSpec { tag: "定心方式", label_key: "gui.nf.col.centering", unit: "", formula_key: "gui.nf.formula.centering", source_key: "gui.nf.source.centering_p18" },
    NfColumnSpec { tag: "模数", label_key: "gui.nf.col.module", unit: "mm", formula_key: "gui.nf.formula.module", source_key: "gui.nf.source.module_col_p18" },
    NfColumnSpec { tag: "齿数", label_key: "gui.nf.col.teeth", unit: "", formula_key: "gui.nf.formula.teeth", source_key: "gui.nf.source.dims" },
    NfColumnSpec { tag: "压力角", label_key: "gui.nf.col.alpha", unit: "°", formula_key: "gui.nf.formula.alpha", source_key: "gui.nf.source.p07" },
    NfColumnSpec { tag: "齿根样式", label_key: "gui.nf.col.root_form", unit: "", formula_key: "gui.nf.formula.root_form", source_key: "gui.nf.source.root_form_p06" },
    NfColumnSpec { tag: "加工方法", label_key: "gui.nf.col.machining", unit: "", formula_key: "gui.nf.formula.machining", source_key: "gui.nf.source.machining_figs" },
    NfColumnSpec { tag: "大径Az", label_key: "gui.nf.col.major_dia", unit: "mm", formula_key: "gui.nf.formula.major_dia", source_key: "gui.nf.source.p04_p07" },
    NfColumnSpec { tag: "小径D", label_key: "gui.nf.col.minor_dia", unit: "mm", formula_key: "gui.nf.formula.minor_dia", source_key: "gui.nf.source.p04_p05_p07_p18" },
    NfColumnSpec { tag: "基准尺寸", label_key: "gui.nf.col.base_size", unit: "mm", formula_key: "gui.nf.formula.base_size", source_key: "gui.nf.source.p07" },
    NfColumnSpec { tag: "量棒直径V", label_key: "gui.nf.col.pin_dia", unit: "mm", formula_key: "gui.nf.formula.pin_dia", source_key: "gui.nf.source.check" },
    NfColumnSpec { tag: "跨棒距G", label_key: "gui.nf.col.over_pins", unit: "mm", formula_key: "gui.nf.formula.over_pins", source_key: "gui.nf.source.check" },
    NfColumnSpec { tag: "大径上差", label_key: "gui.nf.col.major_dia_up", unit: "mm", formula_key: "gui.nf.formula.major_dia_up", source_key: "gui.nf.source.major_tol" },
    NfColumnSpec { tag: "大径下差", label_key: "gui.nf.col.major_dia_down", unit: "mm", formula_key: "gui.nf.formula.major_dia_down", source_key: "gui.nf.source.major_tol" },
    NfColumnSpec { tag: "小径上差", label_key: "gui.nf.col.minor_dia_up", unit: "mm", formula_key: "gui.nf.formula.minor_dia_up", source_key: "gui.nf.source.minor_tol" },
    NfColumnSpec { tag: "小径下差", label_key: "gui.nf.col.minor_dia_down", unit: "mm", formula_key: "gui.nf.formula.minor_dia_down", source_key: "gui.nf.source.minor_tol" },
    NfColumnSpec { tag: "跨棒距上差", label_key: "gui.nf.col.over_pins_up", unit: "mm", formula_key: "gui.nf.formula.over_pins_up", source_key: "gui.nf.source.over_pins_tol" },
    NfColumnSpec { tag: "跨棒距下差", label_key: "gui.nf.col.over_pins_down", unit: "mm", formula_key: "gui.nf.formula.over_pins_down", source_key: "gui.nf.source.over_pins_tol" },
];

/// NF 内花键卡的表单字段（表驱动 GUI 骨架）。
/// NF 卡的表达式策略（表驱动；体系要求 SPLINE、方向恒内、α=20、直齿；变位进 A 公式）。
pub const EXPR_POLICY: crate::card_expr::ExprPolicy = crate::card_expr::ExprPolicy {
    card: "NF 内花键参数表",
    mark: crate::card_expr::ExprMark::Spline,
    alphas: &[20.0],
    spur: true,
    allow_shift: true,
    map: &[
        crate::card_expr::ExprRule {
            target: "a",
            label: "公称直径 A",
            op: crate::card_expr::ExprOp::NfBaseA,
        },
        crate::card_expr::ExprRule {
            target: "m",
            label: "模数 m",
            op: crate::card_expr::ExprOp::Module,
        },
        crate::card_expr::ExprRule {
            target: "z",
            label: "齿数 z",
            op: crate::card_expr::ExprOp::Teeth,
        },
    ],
};

pub const FORM: crate::card::CardFormSpec = crate::card::CardFormSpec {
    fields: &[
        crate::card::CardFieldSpec {
            key: "expr",
            label: "gui.fld.nf_table.form.expr.label",
            kind: "textarea",
            placeholder: "SPLINE IN M7.5 Z38 ALPHA20 X0.8 BETA0 H30",
            default: "",
            title: "gui.fld.nf_table.form.expr.title",
            options: &[],
            options_from: "",
            min: 0.0,
            step: 0.0,
            required: false,
        },
        crate::card::CardFieldSpec {
            key: "a",
            label: "gui.fld.nf_table.form.a.label",
            kind: "number",
            placeholder: "gui.fld.nf_table.form.a.placeholder",
            default: "300",
            title: "gui.fld.nf_table.form.a.title",
            options: &[],
            options_from: "",
            min: 0.0,
            step: 0.001,
            required: true,
        },
        crate::card::CardFieldSpec {
            key: "m",
            label: "gui.fld.nf_table.form.m.label",
            kind: "number",
            placeholder: "gui.fld.nf_table.form.m.placeholder",
            default: "7.5",
            title: "gui.fld.nf_table.form.m.title",
            options: &[],
            options_from: "",
            min: 0.0,
            step: 0.001,
            required: true,
        },
        crate::card::CardFieldSpec {
            key: "z",
            label: "gui.fld.nf_table.form.z.label",
            kind: "number",
            placeholder: "gui.fld.nf_table.form.z.placeholder",
            default: "38",
            title: "gui.fld.nf_table.form.z.title",
            options: &[],
            options_from: "",
            min: 3.0,
            step: 1.0,
            required: false,
        },
        crate::card::CardFieldSpec {
            key: "centering",
            label: "gui.fld.nf_table.form.centering.label",
            kind: "select",
            placeholder: "",
            default: "outer",
            title: "gui.fld.nf_table.form.centering.title",
            options: &[],
            options_from: "nf_centering",
            min: 0.0,
            step: 0.0,
            required: false,
        },
        crate::card::CardFieldSpec {
            key: "root",
            label: "gui.fld.nf_table.form.root.label",
            kind: "select",
            placeholder: "",
            default: "flat",
            title: "gui.fld.nf_table.form.root.title",
            options: &[],
            options_from: "nf_roots",
            min: 0.0,
            step: 0.0,
            required: false,
        },
        crate::card::CardFieldSpec {
            key: "fit",
            label: "gui.fld.nf_table.form.fit.label",
            kind: "select",
            placeholder: "",
            default: "fixed",
            title: "gui.fld.nf_table.form.fit.title",
            options: &[
                ("loose", "gui.fld.nf_table.form.fit.opt.loose"),
                ("slide", "gui.fld.nf_table.form.fit.opt.slide"),
                ("fixed", "gui.fld.nf_table.form.fit.opt.fixed"),
                ("press", "gui.fld.nf_table.form.fit.opt.press"),
            ],
            options_from: "",
            min: 0.0,
            step: 0.0,
            required: false,
        },
    ],
    note_key: "gui.form.nf.note",
    missing_note_key: "gui.form.nf.missing_note",
};

/// NF 卡的选项/口径 JSON（随 `/api/spline_options` 下发；页面只渲染）。
pub fn options_json() -> serde_json::Value {
    serde_json::json!({
        "columns": NF_COLUMNS.iter().map(|c| serde_json::json!({
            "tag": c.tag,
            "label": c.label(),
            "unit": c.unit,
            "formula": c.formula(),
            "source": c.source(),
        })).collect::<Vec<_>>(),
        "modules": crate::invol_spline::nf_e22141_modules(),
        "centering": [
            {"id": "outer", "label": crate::i18n::t("gui.nf.centering.outer")},
            {"id": "flank", "label": crate::i18n::t("gui.nf.centering.flank")},
        ],
        "roots": [
            {"id": "flat", "label": RootStyle::Flat.label()},
            {"id": "fillet", "label": RootStyle::Fillet.label()},
        ],
        // p31/p34 配合名（NF E22-141 译本体系；四档按 p29 分列）
        "fits": FitClass::ALL.iter().map(|f| serde_json::json!({
            "id": f.id(),
            "label": f.label(),
        })).collect::<Vec<_>>(),
        "missing_note": crate::i18n::t("gui.nf.options.missing_note"),
        "note": crate::i18n::t("gui.nf.options.note"),
    })
}

/// CLI/HTTP 表单模型（字段与 GUI 控件一一对应）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct NfTableModel {
    /// 卡类型（GUI 回传；后端按 renderer 分派，这里只记不看）。
    #[serde(default)]
    pub card: String,
    /// 九字段统一齿形表达式（可空；给了则覆盖 A/m/z）。
    #[serde(default)]
    pub expr: Option<String>,
    /// 公称直径 `A`。
    pub a: f64,
    /// 模数 `m`。
    pub m: f64,
    /// 齿数 `z`（选填）。
    #[serde(default)]
    pub z: Option<u32>,
    /// 定心方式（`outer`/`flank`/`外径`/`齿面`；缺省 outer）。
    #[serde(default)]
    pub centering: Option<String>,
    /// 齿根样式（`flat`/`fillet`/`平`/`圆`；缺省 flat）。
    #[serde(default)]
    pub root: Option<String>,
    /// 配合类别（`loose`/`slide`/`fixed`/`press` 或中文；缺省 fixed）。
    #[serde(default)]
    pub fit: Option<String>,
    /// 显式落点；缺省 = 待放置件。
    #[serde(default)]
    pub at: Option<[f64; 2]>,
    #[serde(default)]
    pub rot: f64,
}

impl NfTableModel {
    /// →（校验过的 `NfTableSpec`）。
    pub fn spec(&self) -> Result<NfTableSpec, String> {
        let centering = match self.centering.as_deref().map(str::trim) {
            None | Some("") => Centering::Outer,
            Some(t) => Centering::from_token(t).ok_or_else(|| {
                crate::i18n::t_fmt("cmd.nf.err.centering_invalid", &[("t", t)])
            })?,
        };
        let root = match self.root.as_deref().map(str::trim) {
            None | Some("") => RootStyle::Flat,
            Some(t) => RootStyle::from_token(t).ok_or_else(|| {
                crate::i18n::t_fmt("cmd.nf.err.root_invalid", &[("t", t)])
            })?,
        };
        let fit = match self.fit.as_deref().map(str::trim) {
            None | Some("") => FitClass::default(),
            Some(t) => FitClass::from_token(t).ok_or_else(|| {
                crate::i18n::t_fmt("cmd.nf.err.fit_invalid", &[("t", t)])
            })?,
        };
        let (a, m, z) = match self.expr.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(e) => {
                let r = crate::card_expr::resolve(&EXPR_POLICY, e, Some(true))?;
                (
                    r.value("a")
                        .ok_or_else(|| crate::i18n::t("cmd.nf.err.map_missing_a"))?,
                    r.value("m")
                        .ok_or_else(|| crate::i18n::t("cmd.nf.err.map_missing_m"))?,
                    Some(
                        r.value("z")
                            .ok_or_else(|| {
                                crate::i18n::t("cmd.nf.err.map_missing_z")
                            })? as u32,
                    ),
                )
            }
            None => (self.a, self.m, self.z),
        };
        let spec = NfTableSpec {
            a,
            m,
            z,
            centering,
            root,
            fit,
            at: self.at,
            rot: self.rot,
        };
        derive(&spec)?; // 立即校验（A/m/z 与表值）
        Ok(spec)
    }

    /// 预览 JSON（不碰图纸）。
    pub fn preview_json(&self) -> Result<serde_json::Value, String> {
        let spec = self.spec()?;
        let d = derive(&spec)?;
        let vals = values(&spec)?;
        let mut items = Vec::with_capacity(NF_COLUMNS.len());
        let mut missing = Vec::new();
        for (c, (tag, value)) in NF_COLUMNS.iter().zip(vals) {
            debug_assert_eq!(c.tag, tag);
            if value == MISSING {
                missing.push(c.label());
            }
            items.push(serde_json::json!({
                "tag": tag,
                "label": c.label(),
                "unit": c.unit,
                "value": value,
                "formula": c.formula(),
                "source": c.source(),
                "missing": value == MISSING,
            }));
        }
        let opt = |v: Option<f64>| v.map(fmt_mm).unwrap_or_else(|| MISSING.to_string());
        let az_formula = match spec.centering {
            Centering::Outer => "Az = A",
            Centering::Flank => "Az = A + 0.3m",
        };
        // p29 E/xm 偏差读数（内花键 + 所选配合的外花键）；微米原文与 mm 对照。
        let um = |p: DevPair| format!("{:+}/{:+}", p.upper, p.lower);
        let (p29_int, p29_ext) = match d.tol_row {
            Some(r) => (
                format!(
                    "E {}（{}/{} mm）；xm {}（µm）",
                    um(r.e_int),
                    fmt_um_mm(r.e_int.upper),
                    fmt_um_mm(r.e_int.lower),
                    um(r.xm_int)
                ),
                format!(
                    "E {}；xm {}",
                    um(r.e_ext_for(spec.fit)),
                    um(r.xm_ext_for(spec.fit))
                ),
            ),
            None => (MISSING.to_string(), MISSING.to_string()),
        };
        let readout = serde_json::json!([
            {"k": crate::i18n::t("gui.nf.readout.nominal_a"), "v": fmt_mm(spec.a)},
            {"k": crate::i18n::t_fmt("gui.nf.readout.major_dia", &[("az_formula", az_formula)]), "v": fmt_mm(d.az)},
            {"k": crate::i18n::t("gui.nf.readout.minor_dia"), "v": fmt_mm(d.d)},
            {"k": crate::i18n::t("gui.nf.readout.table_minor_dia"), "v": d.table_d.map(fmt_mm).unwrap_or_else(|| MISSING.to_string())},
            {"k": crate::i18n::t("gui.nf.readout.pin_v"), "v": format!("{} / {}", opt(d.v), opt(d.v1))},
            {"k": crate::i18n::t("gui.nf.readout.over_pins_g"), "v": format!("{} / {}", opt(d.g), opt(d.g1))},
            {"k": crate::i18n::t("gui.nf.readout.root_fillet_ri"), "v": opt(d.ri)},
            {"k": crate::i18n::t("gui.nf.readout.shift_x"), "v": opt(d.x)},
            {"k": crate::i18n::t("gui.nf.readout.p29_int_dev"), "v": p29_int, "tol": true},
            {"k": crate::i18n::t_fmt("gui.nf.readout.p29_ext_dev", &[("fit", &spec.fit.label())]), "v": p29_ext, "tol": true},
            {"k": crate::i18n::t("gui.nf.readout.major_tol"), "v": d.major_tol.map(|l| { let (u, lo) = l.display(); format!("{u} / {lo}") }).unwrap_or_else(|| MISSING.to_string()), "tol": true},
            {"k": crate::i18n::t("gui.nf.readout.minor_tol"), "v": d.minor_tol.map(|l| { let (u, lo) = l.display(); format!("{u} / {lo}") }).unwrap_or_else(|| MISSING.to_string()), "tol": true},
            {"k": crate::i18n::t("gui.nf.readout.p18_row"), "v": d.dims_source},
            {"k": crate::i18n::t("gui.nf.readout.p22_row"), "v": d.detail_source},
            {"k": crate::i18n::t("gui.nf.readout.p25_row"), "v": d.check_source},
        ]);
        let expr_echo = self
            .expr
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let fields = if expr_echo.is_some() {
            serde_json::json!({
                "a": spec.a,
                "m": spec.m,
                "z": spec.z,
            })
        } else {
            serde_json::Value::Null
        };
        Ok(serde_json::json!({
            "ok": true,
            "card": "NF内花键参数表",
            "renderer": "nf_table",
            "expr": expr_echo,
            "fields": fields,
            "title": format!(
                "NF E22-141 内花键参数表（A={} m={} z={}，{}配合）",
                fmt_mm(spec.a),
                fmt_mm(spec.m),
                d.z.map(|z| z.to_string()).unwrap_or_else(|| MISSING.to_string()),
                spec.fit.label()
            ),
            "a": spec.a,
            "m": spec.m,
            "z": d.z,
            "centering": spec.centering.id(),
            "root": spec.root.id(),
            "fit": spec.fit.id(),
            "readout": readout,
            "items": items,
            "missing": missing,
            "missing_note": crate::i18n::t("gui.nf.preview.missing_note"),
            // 直径公差读数注（随语言；GUI 读数旁注/API 消费者用）。
            "major_tol_note": d.major_tol_note,
            "minor_tol_note": d.minor_tol_note,
        }))
    }

    /// 待放置件要带的 18 个 ATTRIB（tag → 值 + ATTDEF 模板）。
    pub fn pending_attrs(&self) -> Result<Vec<(AttributeDefinition, String)>, String> {
        let spec = self.spec()?;
        let vals = values(&spec)?;
        let mut out = Vec::with_capacity(vals.len());
        for ad in attdefs() {
            let v = vals
                .iter()
                .find(|(tag, _)| tag == &ad.tag)
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            out.push((ad, v));
        }
        Ok(out)
    }

    /// 直接落点用的 `INSERT`（CLI 与 GUI 同一条路径）。
    pub fn build_insert(&self) -> Result<Insert, String> {
        let spec = self.spec()?;
        let at = self.at.unwrap_or([0.0, 0.0]);
        build_insert(&spec, at, self.rot)
    }

    /// 插入回执里的一段。
    pub fn echo_note(&self) -> Result<String, String> {
        let spec = self.spec()?;
        let d = derive(&spec)?;
        Ok(crate::i18n::t_fmt(
            "cmd.nf.echo",
            &[
                ("a", &fmt_mm(spec.a)),
                ("m", &fmt_mm(spec.m)),
                (
                    "z",
                    &d.z.map(|z| z.to_string())
                        .unwrap_or_else(|| MISSING.to_string()),
                ),
                (
                    "centering",
                    &crate::i18n::t(match spec.centering {
                        Centering::Outer => "cmd.nf.centering.outer",
                        Centering::Flank => "cmd.nf.centering.flank",
                    }),
                ),
                (
                    "fit",
                    &crate::i18n::t(match spec.fit {
                        FitClass::Loose => "cmd.nf.fit.loose",
                        FitClass::Slide => "cmd.nf.fit.sliding",
                        FitClass::Fixed => "cmd.nf.fit.fixed",
                        FitClass::Press => "cmd.nf.fit.press",
                    }),
                ),
                (
                    "vg",
                    &match (d.v, d.g, d.g1) {
                        (Some(v), Some(g), Some(g1)) => {
                            format!("{} / {} / {}", fmt_mm(v), fmt_mm(g), fmt_mm(g1))
                        }
                        _ => MISSING.to_string(),
                    },
                ),
            ],
        ))
    }

    /// 待放置件的 `OCSM_PART` 元数据（薄台账）。
    pub fn part_meta_json(&self) -> Result<String, String> {
        let spec = self.spec()?;
        let d = derive(&spec)?;
        Ok(serde_json::json!({
            "family": "nf_table",
            "card": "NF内花键参数表",
            "a": spec.a,
            "m": spec.m,
            "z": d.z,
            "centering": spec.centering.id(),
            "root": spec.root.id(),
            "fit": spec.fit.id(),
        })
        .to_string())
    }
}

/// 命令用法（`OCSMCARD` 报错指路）。
/// 表达式截取用：NF 卡选项关键字（`A300`/`M7.5`/`Z38`/`中心`/`根`/`配合`/`at`/`rot`…）。
/// 只在表达式第 7 个 token 之后调用（`M7.5`/`Z38` 在表达式里不会被误判）。
fn is_option_token(t: &str) -> bool {
    let l = t.to_ascii_lowercase();
    if t.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return true;
    }
    if matches!(
        l.as_str(),
        "at" | "rot" | "旋转" | "中心" | "定心" | "centering" | "根" | "齿根" | "root" | "配合"
            | "fit" | "直径" | "公称直径" | "模数" | "齿数"
    ) {
        return true;
    }
    for p in ['a', 'm', 'z'] {
        if l == p.to_string() {
            return true;
        }
        if let Some(rest) = l.strip_prefix(p) {
            if rest.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return true;
            }
        }
    }
    false
}

pub fn usage() -> String {
    crate::i18n::t("cmd.nf.usage")
}

impl NfTableSpec {
    /// 解析 `A<公称直径> M<模数> [Z<齿数>] [中心 外径|齿面] [根 平|圆] [at x,y] [rot 度]`。
    pub fn parse(text: &str) -> Result<Self, String> {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        if tokens.is_empty() {
            return Err(usage());
        }
        let mut a: Option<f64> = None;
        let mut m: Option<f64> = None;
        let mut z: Option<u32> = None;
        let mut centering = Centering::Outer;
        let mut root = RootStyle::Flat;
        let mut fit = FitClass::default();
        let mut at: Option<[f64; 2]> = None;
        let mut rot = 0.0f64;
        let mut i = 0usize;
        // 表达式形态：`<九字段表达式> [中心 …] [根 …] [配合 …] [at …] [rot …]`。
        let mut expr: Option<String> = None;
        let tokens = if let Some((e, rest)) = crate::card_expr::split_expr(&tokens, is_option_token) {
            expr = Some(e);
            rest
        } else {
            tokens
        };
        let need = |i: &mut usize, tokens: &[&str], what: &str| -> Result<String, String> {
            *i += 1;
            tokens
                .get(*i)
                .map(|s| s.to_string())
                .ok_or_else(|| {
                    crate::i18n::t_fmt("cmd.nf.err.need", &[("what", what)])
                })
        };
        while i < tokens.len() {
            let t = tokens[i];
            let key = t.to_ascii_lowercase();
            if key == "at" {
                let v = need(&mut i, &tokens, "at")?;
                let (x, y) = v
                    .split_once(',')
                    .ok_or_else(|| {
                        crate::i18n::t_fmt("cmd.nf.err.at_format", &[("v", &v)])
                    })?;
                let x: f64 = x
                    .trim()
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.nf.err.at_x",
                            &[("x", &x), ("e", &e.to_string())],
                        )
                    })?;
                let y: f64 = y
                    .trim()
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.nf.err.at_y",
                            &[("y", &y), ("e", &e.to_string())],
                        )
                    })?;
                at = Some([x, y]);
            } else if key == "rot" || key == "旋转" {
                let v = need(&mut i, &tokens, "rot")?;
                rot = v
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.nf.err.rot",
                            &[("v", &v), ("e", &e.to_string())],
                        )
                    })?;
            } else if key == "中心" || key == "定心" || key == "centering" {
                let v = need(&mut i, &tokens, "中心")?;
                centering = Centering::from_token(&v)
                    .ok_or_else(|| format!("NF 内花键参数表：定心方式「{v}」非法（可用 外径 / 齿面）"))?;
            } else if key == "根" || key == "齿根" || key == "root" {
                let v = need(&mut i, &tokens, "根")?;
                root = RootStyle::from_token(&v)
                    .ok_or_else(|| format!("NF 内花键参数表：齿根样式「{v}」非法（可用 平 / 圆）"))?;
            } else if key == "配合" || key == "fit" {
                let v = need(&mut i, &tokens, "配合")?;
                fit = FitClass::from_token(&v).ok_or_else(|| {
                    format!("NF 内花键参数表：配合类别「{v}」非法（可用 松动 / 滑动 / 固定 / 压）")
                })?;
            } else if key == "直径" || key == "公称直径" || key == "a" {
                let v = need(&mut i, &tokens, "A")?;
                a = Some(
                    v.parse()
                        .map_err(|e: std::num::ParseFloatError| {
                            crate::i18n::t_fmt(
                                "cmd.nf.err.num",
                                &[("name", "A"), ("v", &v), ("e", &e.to_string())],
                            )
                        })?,
                );
            } else if key == "模数" || key == "m" {
                let v = need(&mut i, &tokens, "m")?;
                m = Some(
                    v.parse()
                        .map_err(|e: std::num::ParseFloatError| {
                            crate::i18n::t_fmt(
                                "cmd.nf.err.num",
                                &[("name", "m"), ("v", &v), ("e", &e.to_string())],
                            )
                        })?,
                );
            } else if key == "齿数" || key == "z" {
                let v = need(&mut i, &tokens, "z")?;
                z = Some(
                    v.parse()
                        .map_err(|e: std::num::ParseIntError| {
                            crate::i18n::t_fmt(
                                "cmd.nf.err.z_num",
                                &[("name", "z"), ("v", &v), ("e", &e.to_string())],
                            )
                        })?,
                );
            } else if let Some(v) = strip_prefix_ci(t, "a") {
                let v = if v.is_empty() { need(&mut i, &tokens, "A")? } else { v.to_string() };
                a = Some(
                    v.parse()
                        .map_err(|e: std::num::ParseFloatError| {
                            crate::i18n::t_fmt(
                                "cmd.nf.err.num",
                                &[("name", "A"), ("v", &v), ("e", &e.to_string())],
                            )
                        })?,
                );
            } else if let Some(v) = strip_prefix_ci(t, "m") {
                let v = if v.is_empty() { need(&mut i, &tokens, "M")? } else { v.to_string() };
                m = Some(
                    v.parse()
                        .map_err(|e: std::num::ParseFloatError| {
                            crate::i18n::t_fmt(
                                "cmd.nf.err.num",
                                &[("name", "M"), ("v", &v), ("e", &e.to_string())],
                            )
                        })?,
                );
            } else if let Some(v) = strip_prefix_ci(t, "z") {
                let v = if v.is_empty() { need(&mut i, &tokens, "Z")? } else { v.to_string() };
                z = Some(
                    v.parse()
                        .map_err(|e: std::num::ParseIntError| {
                            crate::i18n::t_fmt(
                                "cmd.nf.err.z_num",
                                &[("name", "Z"), ("v", &v), ("e", &e.to_string())],
                            )
                        })?,
                );
            } else {
                return Err(crate::i18n::t_fmt(
                    "cmd.nf.err.unknown_param",
                    &[("t", t), ("usage", &usage())],
                ));
            }
            i += 1;
        }
        // 表达式 > 旧 A/M/Z；显式值与表达式反解不一致 → 报错（帮排错）。
        let (a, m, z) = match expr {
            Some(e) => {
                let r = crate::card_expr::resolve(&EXPR_POLICY, &e, Some(true))?;
                let ea = r.value("a").ok_or_else(|| {
                    crate::i18n::t("cmd.nf.err.map_missing_a")
                })?;
                let em = r.value("m").ok_or_else(|| {
                    crate::i18n::t("cmd.nf.err.map_missing_m")
                })?;
                let ez = r.value("z").ok_or_else(|| {
                    crate::i18n::t("cmd.nf.err.map_missing_z")
                })? as u32;
                if let Some(given) = a {
                    if (given - ea).abs() > 1e-6 {
                        return Err(crate::i18n::t_fmt(
                            "cmd.nf.err.explicit_mismatch",
                            &[
                                ("name", "A"),
                                ("given", &crate::partgen_kit::trim(given)),
                                ("resolved", &crate::partgen_kit::trim(ea)),
                            ],
                        ));
                    }
                }
                if let Some(given) = m {
                    if (given - em).abs() > 1e-9 {
                        return Err(crate::i18n::t_fmt(
                            "cmd.nf.err.explicit_mismatch",
                            &[
                                ("name", "m"),
                                ("given", &crate::partgen_kit::trim(given)),
                                ("resolved", &crate::partgen_kit::trim(em)),
                            ],
                        ));
                    }
                }
                if let Some(given) = z {
                    if given != ez {
                        return Err(crate::i18n::t_fmt(
                            "cmd.nf.err.explicit_mismatch",
                            &[
                                ("name", "z"),
                                ("given", &given.to_string()),
                                ("resolved", &ez.to_string()),
                            ],
                        ));
                    }
                }
                (ea, em, Some(ez))
            }
            None => {
                let a = a.ok_or_else(|| {
                    crate::i18n::t_fmt("cmd.nf.err.missing_a", &[("usage", &usage())])
                })?;
                let m = m.ok_or_else(|| {
                    crate::i18n::t_fmt("cmd.nf.err.missing_m", &[("usage", &usage())])
                })?;
                (a, m, z)
            }
        };
        let spec = NfTableSpec {
            a,
            m,
            z,
            centering,
            root,
            fit,
            at,
            rot,
        };
        derive(&spec)?; // 立即校验（表值/ z 核对）
        Ok(spec)
    }
}

/// 大小写不敏感地剥前缀（`A300` → `300`；非 ASCII 边界返回 `None`，CJK token 直接落到不认识分支）。
fn strip_prefix_ci<'a>(t: &'a str, p: &str) -> Option<&'a str> {
    t.get(..p.len())
        .filter(|head| head.eq_ignore_ascii_case(p))
        .map(|_| &t[p.len()..])
}

// ══════════════════════════════════════════════════════════════════════════
// 测试
// ══════════════════════════════════════════════════════════════════════════

/// 测试用：拆出标签/标题 MTEXT 头部的**内联字宽码**（`\W<f>;`）→ `(内联因子, 纯文字)`。
/// 无码 = `(1.0, 原文)`；MTEXT 的**有效字宽** = 模板样式 0.7 × 内联因子（见模块文档）。
#[cfg(test)]
pub(crate) fn mtext_inline_wf(value: &str) -> (f64, &str) {
    if let Some(rest) = value.strip_prefix("\\W") {
        if let Some((f, text)) = rest.split_once(';') {
            if let Ok(wf) = f.parse::<f64>() {
                return (wf, text);
            }
        }
    }
    (1.0, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// ⑥ 批：直径公差**读数注**随语言（Some/None 两分支；`ISO 286 R7/H7` 等协议记号两语原样）。
    #[test]
    fn tol_notes_switch_language_and_have_no_cjk_in_english() {
        use crate::i18n::{set_lang, set_lang_auto, Lang};
        let _g = crate::global_state_test_lock();
        let cjk = |s: &str| s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
        let spec = |a: f64| NfTableSpec {
            a,
            m: 3.75,
            z: None,
            centering: Centering::Outer,
            root: RootStyle::Flat,
            fit: FitClass::Fixed,
            at: None,
            rot: 0.0,
        };
        set_lang(Lang::Zh);
        let hit_zh = derive(&spec(80.0)).unwrap();
        let miss_zh = derive(&spec(5000.0)).unwrap();
        set_lang(Lang::En);
        let hit_en = derive(&spec(80.0)).unwrap();
        let miss_en = derive(&spec(5000.0)).unwrap();
        set_lang_auto();
        assert!(hit_zh.major_tol.is_some() && hit_zh.minor_tol.is_some(), "A=80 应在 ISO 286 档内");
        assert!(
            miss_zh.major_tol.is_none() || miss_zh.minor_tol.is_none(),
            "A=5000 应缺 ISO 286 档"
        );
        assert!(cjk(&hit_zh.major_tol_note) && cjk(&hit_zh.minor_tol_note));
        for (z, e) in [
            (&hit_zh.major_tol_note, &hit_en.major_tol_note),
            (&hit_zh.minor_tol_note, &hit_en.minor_tol_note),
            (&miss_zh.major_tol_note, &miss_en.major_tol_note),
            (&miss_zh.minor_tol_note, &miss_en.minor_tol_note),
        ] {
            assert_ne!(z, e, "公差注应随语言：{z}");
            assert!(!cjk(e), "en 公差注仍有汉字：{e}");
            assert!(e.contains("ISO 286"), "协议记号两语原样：{e}");
        }
        if miss_en.major_tol.is_none() {
            assert!(miss_en.major_tol_note.contains("Az="), "{miss_en:?}");
        }
        if miss_en.minor_tol.is_none() {
            assert!(miss_en.minor_tol_note.contains("D="), "{miss_en:?}");
        }
        assert!(hit_en.major_tol_note.contains("R7") && hit_en.minor_tol_note.contains("H7"));
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn near5(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-5
    }

    fn anchor_spec() -> NfTableSpec {
        NfTableSpec {
            a: 300.0,
            m: 7.5,
            z: Some(38),
            centering: Centering::Outer,
            root: RootStyle::Flat,
            fit: FitClass::Fixed,
            at: None,
            rot: 0.0,
        }
    }

    fn value_map(spec: &NfTableSpec, at: [f64; 2]) -> HashMap<String, String> {
        build_insert(spec, at, 0.0)
            .unwrap()
            .attributes
            .iter()
            .map(|a| (a.tag.clone(), a.value.clone()))
            .collect()
    }

    /// 13 行结构 + 逐图元对模板（1e-5）：66 线（外框/中分隔/行分隔）、标题 + 12 标签、
    /// 12 值 ATTDEF、6 公差 ATTDEF；末行标签归位；样式一律 OCSM_GB。
    /// 逐图元对模板（③ 卡面批：**中英各跑一遍** —— 卡面文字随语言，元素清单/位置/图层不变）。
    #[test]
    fn nf_template_geometry_line_by_line() {
        let _g = crate::global_state_test_lock();
        for lang in [crate::i18n::Lang::Zh, crate::i18n::Lang::En] {
            crate::i18n::set_lang(lang);
            nf_template_geometry_one_lang(lang);
        }
        crate::i18n::set_lang_auto();
    }

    fn nf_template_geometry_one_lang(lang: crate::i18n::Lang) {
        let ents = block_entities();
        assert_eq!(
            ents.len(),
            66 + 13 + 18,
            "模板 97 图元（66 线 + 25 MTEXT + 6 TEXT）→ 66 线 + 13 标签 + 18 属性"
        );

        // ── 66 条 LINE：逐条在集合里能按坐标+图层找到（1e-5）──
        let line_hits = |a: [f64; 2], b: [f64; 2], layer: &str| -> bool {
            ents.iter().any(|e| {
                matches!(e, EntityType::Line(l)
                    if l.common.layer == layer
                        && ((near5(l.start.x, a[0]) && near5(l.start.y, a[1])
                            && near5(l.end.x, b[0]) && near5(l.end.y, b[1]))
                            || (near5(l.start.x, b[0]) && near5(l.start.y, b[1])
                                && near5(l.end.x, a[0]) && near5(l.end.y, a[1]))))
            })
        };
        let line_count = NF_LINES.len();
        assert_eq!(line_count, 66);
        for (a, b, layer) in NF_LINES {
            assert!(line_hits(*a, *b, layer), "缺线 {a:?}→{b:?} @{layer}");
        }
        // 13 行 = 14 条边界（标题 + 12 内容行）——先算出来给边框/分隔检查共用
        let mut sep_y: Vec<f64> = NF_LINES
            .iter()
            .filter(|(a, b, _)| near5(a[1], b[1]))
            .map(|(_, b, _)| b[1])
            .collect();
        sep_y.sort_by(|a, b| b.partial_cmp(a).unwrap());
        sep_y.dedup_by(|a, b| near5(*a, *b));
        assert_eq!(sep_y.len(), 14, "13 行 → 14 条水平边界：{sep_y:?}");
        assert!(near5(sep_y[0], 0.0) && near5(sep_y[13], FRAME.3));

        // 四边框：上/下各两半（1轮廓实线层）；左右边框模板是**逐行分段**画的（不是一条通高线）
        assert!(line_hits([FRAME.0, 0.0], [MID_X, 0.0], "1轮廓实线层"), "上框左半");
        assert!(line_hits([MID_X, 0.0], [0.0, 0.0], "1轮廓实线层"), "上框右半");
        assert!(line_hits([FRAME.0, FRAME.3], [MID_X, FRAME.3], "1轮廓实线层"), "下框左半");
        assert!(line_hits([MID_X, FRAME.3], [0.0, FRAME.3], "1轮廓实线层"), "下框右半");
        for w in sep_y.windows(2) {
            assert!(
                line_hits([FRAME.0, w[0]], [FRAME.0, w[1]], "1轮廓实线层"),
                "左框段 {w:?}"
            );
            assert!(
                line_hits([0.0, w[0]], [0.0, w[1]], "1轮廓实线层"),
                "右框段 {w:?}"
            );
        }

        // 每行边界都有左右两半
        for y in &sep_y {
            assert!(
                line_hits([FRAME.0, *y], [MID_X, *y], "2细线层")
                    || line_hits([FRAME.0, *y], [MID_X, *y], "1轮廓实线层"),
                "边界 y={y} 左半缺"
            );
            assert!(
                line_hits([MID_X, *y], [0.0, *y], "2细线层")
                    || line_hits([MID_X, *y], [0.0, *y], "1轮廓实线层"),
                "边界 y={y} 右半缺"
            );
        }
        // 中分隔线（左右两列的竖线，逐行都有）
        assert!(line_hits([MID_X, -41.93014705882336], [MID_X, -82.6214766677619], "2细线层"));
        assert!(line_hits([MID_X, -489.2095660532368], [MID_X, FRAME.3], "2细线层"));

        // ── 13 条 MTEXT：标题 + 12 标签，全部 OCSM_GB / 6文字层 ──
        let mtexts: Vec<&MText> = ents
            .iter()
            .filter_map(|e| match e {
                EntityType::MText(m) => Some(m),
                _ => None,
            })
            .collect();
        assert_eq!(mtexts.len(), 13, "标题 + 12 标签");
        let title_text = face_text("card.nf.title.int");
        let title = mtexts
            .iter()
            .find(|m| m.value == title_text)
            .unwrap_or_else(|| panic!("{lang:?} 缺标题 MTEXT「{title_text}」"));
        assert_eq!(title.attachment_point, AttachmentPoint::MiddleCenter);
        assert!(near5(title.insertion_point.x, TITLE_AT.0) && near5(title.insertion_point.y, TITLE_AT.1));
        for m in &mtexts {
            assert_eq!(m.style, "OCSM_GB", "MTEXT 样式：{}", m.value);
            assert_eq!(m.common.layer, "6文字层");
            assert!(near5(m.height, TEXT_H));
        }
        for (key, symbol, _, y) in NF_ROWS {
            let label = face_label(key, symbol);
            let m = mtexts
                .iter()
                .find(|m| mtext_inline_wf(&m.value).1 == label)
                .unwrap_or_else(|| panic!("{lang:?} 缺标签 MTEXT {label}"));
            assert_eq!(m.attachment_point, AttachmentPoint::MiddleLeft, "{label}");
            assert!(
                near5(m.insertion_point.x, LABEL_X) && near5(m.insertion_point.y, *y),
                "{label} 位置异常：{:?}（末行必须归位 {LABEL_X}）",
                m.insertion_point
            );
        }
        // 模板末行标签原 x=−444.95 出框 → 镜像后不允许任何标签还在那个位置
        assert!(
            mtexts.iter().all(|m| (m.insertion_point.x + 444.9504118793738).abs() > 1.0),
            "末行标签仍在模板的出框位置"
        );

        // ── 18 个 ATTDEF：12 值（左中）+ 6 公差（基线），tag/位置/字高/字宽 ──
        let atts: Vec<AttributeDefinition> = attdefs();
        assert_eq!(atts.len(), 18);
        let want_tags: Vec<&str> = VALUE_TAGS.iter().chain(TOL_TAGS.iter()).copied().collect();
        let got_tags: Vec<&str> = atts.iter().map(|a| a.tag.as_str()).collect();
        assert_eq!(got_tags, want_tags, "ATTDEF tag 顺序");
        for (i, (_, _, tag, y)) in NF_ROWS.iter().enumerate() {
            let ad = &atts[i];
            assert_eq!(ad.tag, *tag);
            assert!(near5(ad.insertion_point.x, VALUE_X) && near5(ad.insertion_point.y, *y));
            assert!(near5(ad.height, TEXT_H));
            assert!(
                near5(ad.width_factor, value_width_factor(tag, VALUE_WIDTH_FACTOR, lang)),
                "{tag} 字宽 {} ≠ {}",
                ad.width_factor,
                value_width_factor(tag, VALUE_WIDTH_FACTOR, lang)
            );
            assert_eq!(ad.horizontal_alignment, HorizontalAlignment::Left);
            assert_eq!(ad.vertical_alignment, VerticalAlignment::Middle, "值行与模板 MTEXT 同为左中");
            assert_eq!(ad.text_style, "OCSM_GB");
            assert_eq!(ad.common.layer, "6文字层");
        }
        for (j, (tag, x, y)) in NF_TOL_ATTS.iter().enumerate() {
            let ad = &atts[12 + j];
            assert_eq!(ad.tag, *tag);
            let want_x = if j >= 4 { *x + TOL_X_SHIFT } else { *x };
            assert!(
                near5(ad.insertion_point.x, want_x) && near5(ad.insertion_point.y, *y),
                "{tag} 位置 {:?} ≠ {want_x}",
                ad.insertion_point
            );
            assert!(j < 4 || near5(TOL_X_SHIFT, 2.0 * TOL_H * TOL_WIDTH_FACTOR), "右移口径");
            assert!(near5(ad.height, TOL_H));
            assert!(near5(ad.width_factor, TOL_WIDTH_FACTOR));
            assert_eq!(ad.vertical_alignment, VerticalAlignment::Baseline, "公差照模板 TEXT 基线");
            assert_eq!(ad.text_style, "OCSM_GB");
            assert_eq!(ad.common.layer, "6文字层");
        }
        // 公差 tag 分组：大径/小径/跨棒距 × 上/下
        for (i, pair) in [("大径上差", "大径下差"), ("小径上差", "小径下差"), ("跨棒距上差", "跨棒距下差")]
            .iter()
            .enumerate()
        {
            assert_eq!(atts[12 + 2 * i].tag, *pair.0);
            assert_eq!(atts[12 + 2 * i + 1].tag, *pair.1);
        }
    }

    /// ★ 锚点正向断言（调研 §4.2）：模板示例 m=7.5 / A=300 / z=38 的每个标准原值，
    /// 与 `assets/nf_internal_card_anchor.csv`（逐行 source）交叉核对。
    #[test]
    fn nf_anchor_values_match_standard_table() {
        let spec = anchor_spec();
        let d = derive(&spec).unwrap();
        let vals = value_map(&spec, [0.0, 0.0]);
        let get = |tag: &str| vals.get(tag).unwrap_or_else(|| panic!("缺 {tag}"));
        assert_eq!(get("执行标准"), "NF E22-141");
        assert_eq!(get("定心方式"), "外径定心");
        assert_eq!(get("模数"), "7.5");
        assert_eq!(get("齿数"), "38");
        assert_eq!(get("压力角"), "20°");
        assert_eq!(get("齿根样式"), "平齿根");
        assert_eq!(get("加工方法"), "拉削");
        assert_eq!(get("大径Az"), "300");
        assert_eq!(get("小径D"), "285");
        assert_eq!(get("基准尺寸"), "300");
        assert_eq!(get("量棒直径V"), "15");
        assert_eq!(get("跨棒距G"), "270.508");
        // 公差（p28 直径公差 + p29 内花键 E 偏差，µm→mm）
        assert_eq!(get("大径上差"), "-0.078", "Az=300 的 ISO 286 R7");
        assert_eq!(get("大径下差"), "-0.130");
        assert_eq!(get("小径上差"), "+0.052", "D=285 的 ISO 286 H7");
        assert_eq!(get("小径下差"), "0");
        assert_eq!(get("跨棒距上差"), "+0.052", "p29 m=7.5/A=300 内花键 E +52/0");
        assert_eq!(get("跨棒距下差"), "0");
        // 表值与公式交叉核对
        assert_eq!(d.table_d, Some(285.0), "p18 表 D = A−2m = 285");
        assert_eq!(d.v, Some(15.0));
        assert_eq!(d.v1, Some(12.6));
        assert_eq!(d.g, Some(270.508));
        assert_eq!(d.g1, Some(270.508));
        assert_eq!(d.ri, Some(1.343));
        assert_eq!(d.x, Some(0.8));
        assert!(near(d.az, 300.0), "外径定心 Az=A=300");
        // 齿面定心 → A+0.3m = 302.25
        let mut flank = spec.clone();
        flank.centering = Centering::Flank;
        let df = derive(&flank).unwrap();
        assert!(near(df.az, 302.25), "齿面定心 Az=A+0.3m=302.25，实为 {}", df.az);
        assert_eq!(value_map(&flank, [0.0, 0.0])["定心方式"], "齿面定心");
        // 外花键对账锚点：模板 W=129.871 ↔ p25 行 E（调研 §4.2）
        let check = crate::invol_spline::nf_check_by_a_m(300.0, 7.5)
            .into_iter()
            .find(|r| r.page == 25 && r.has_check_dims())
            .expect("p25 检查行");
        assert!(near(check.e.unwrap(), 129.871), "模板 W 与 p25 E 逐位一致");

        // ── 逐行对照 assets/nf_internal_card_anchor.csv（field→值；1e-9）──
        let text = include_str!("../assets/nf_internal_card_anchor.csv");
        let mut checked = 0usize;
        for line in text.lines() {
            if line.trim().is_empty() || line.starts_with('#') || line.starts_with("field,") {
                continue;
            }
            let f: Vec<&str> = line.split(',').collect();
            assert_eq!(f.len(), 10, "锚点 CSV 列数：{line}");
            let field = f[0];
            let want: f64 = f[6].parse().unwrap_or_else(|e| panic!("{line}: {e}"));
            let got: f64 = match field {
                "大径Az_外径定心" => d.az,
                "大径Az_齿面定心" => {
                    // 该行锚点值对应齿面定心，单独用 flank spec 复核；
                    // 外径 spec 的 Az 已在上一行核对，这里仅确认公式口径。
                    spec.a + 0.3 * spec.m
                }
                "小径D" => d.d,
                "量棒直径V" => d.v.unwrap(),
                "量棒削边V1" => d.v1.unwrap(),
                "跨棒距G" => d.g.unwrap(),
                "跨棒距G1" => d.g1.unwrap(),
                "槽底圆角ri" => d.ri.unwrap(),
                "变位系数x" => d.x.unwrap(),
                "压力角a" => vals["压力角"].trim_end_matches('°').parse().unwrap(),
                "外花键公法线W_对账锚点" => check.e.unwrap(),
                other => panic!("锚点 CSV 有未识别的 field「{other}」"),
            };
            assert!(near(got, want), "锚点 {field}: 卡取 {got} ≠ 标准 {want}（{line}）");
            checked += 1;
        }
        assert_eq!(checked, 11, "锚点行数（含外花键对账锚点）");
    }

    /// p29 E/xm 偏差资产：14 行、两处 dev7 校正、A 范围/列表命中、四配合列与主控转录一致。
    #[test]
    fn nf_p29_e_xm_tol_asset_matches_standard() {
        let rows = e_xm_tol_rows();
        assert_eq!(rows.len(), 14, "p29 14 行");
        // m=7.5 / A=300：锚点行（E 内 +52/0；xm 内 +76/0；固定 E +42/-42、xm +61/-61）
        let r = e_xm_tol_row(7.5, 300.0).expect("m=7.5 A=300 应在 p29 表内");
        assert_eq!(r.e_int, DevPair { upper: 52.0, lower: 0.0 });
        assert_eq!(r.xm_int, DevPair { upper: 76.0, lower: 0.0 });
        assert_eq!(r.e_ext_for(FitClass::Fixed), DevPair { upper: 42.0, lower: -42.0 });
        assert_eq!(r.xm_ext_for(FitClass::Fixed), DevPair { upper: 61.0, lower: -61.0 });
        assert_eq!(r.e_ext_for(FitClass::Loose), DevPair { upper: -110.0, lower: -194.0 });
        assert_eq!(r.xm_ext_for(FitClass::Press), DevPair { upper: 202.0, lower: 79.0 });
        // m=7.5 / A=200 命中第一表列（110,120,130,140,150,170,180,200,250），不是第二表列
        let r200 = e_xm_tol_row(7.5, 200.0).expect("A=200");
        assert_eq!(r200.e_int, DevPair { upper: 43.0, lower: 0.0 });
        assert_eq!(r200.e_ext_for(FitClass::Press), DevPair { upper: 115.0, lower: 45.0 });
        // m=10 / A=300–400 的压列特异值（用户转录确认）
        let r10 = e_xm_tol_row(10.0, 350.0).expect("A=350");
        assert_eq!(r10.e_ext_for(FitClass::Press), DevPair { upper: 148.0, lower: 64.0 });
        assert_eq!(r10.xm_ext_for(FitClass::Press), DevPair { upper: 217.0, lower: 94.0 });
        // 表外：A=210/m=7.5、A=300/m=3.75 → None（不外推）
        assert!(e_xm_tol_row(7.5, 210.0).is_none());
        assert!(e_xm_tol_row(3.75, 300.0).is_none());
        // 两处 dev7 校正：0.75 / 1.00 行不再有 -30/-146（与 0.50/1.25 同行）
        for m in [0.5, 0.75, 1.0, 1.25] {
            let r = e_xm_tol_row(m, 10.0).or_else(|| e_xm_tol_row(m, 4.0)).unwrap_or_else(|| {
                panic!("m={m} 行")
            });
            assert_eq!(
                r.xm_int,
                DevPair { upper: 37.0, lower: 0.0 },
                "m={m} xm 内"
            );
        }
        assert_eq!(
            e_xm_tol_row(0.75, 10.0).unwrap().e_ext_for(FitClass::Loose),
            DevPair { upper: -60.0, lower: -100.0 }
        );
        assert!(
            e_xm_tol_rows()
                .iter()
                .filter(|r| (r.m - 0.75).abs() < 1e-9 || (r.m - 1.0).abs() < 1e-9)
                .all(|r| r.xm_ext_for(FitClass::Loose) == DevPair { upper: -88.0, lower: -146.0 }),
            "dev7 校正 -88/-146"
        );
        // 逐行 raw 非空（原始读数留档）
        for r in rows {
            assert!(!r.raw.is_empty() && r.source == "p29", "{r:?}");
        }
    }

    /// 标缺行为：6 个公差格一律「—」；表外 (m,A) 的 V/G/ri/z 一律「—」，不外推。
    #[test]
    fn nf_missing_is_dash_and_no_extrapolation() {
        // ① 锚点公差：6 格全有值（p28 R7/H7 + p29 内花键 E）；不再整片标缺
        let spec = anchor_spec();
        let vals = values(&spec).unwrap();
        let g = |t: &str| vals.iter().find(|(x, _)| x == t).unwrap().1.clone();
        assert_eq!(g("大径上差"), "-0.078", "ISO 286 R7（Az=300，280–315 档）");
        assert_eq!(g("大径下差"), "-0.130");
        assert_eq!(g("小径上差"), "+0.052", "ISO 286 H7（D=285，250–315 档）");
        assert_eq!(g("小径下差"), "0");
        assert_eq!(g("跨棒距上差"), "+0.052", "p29 m=7.5/A=300 内花键 E 上差 +52 µm");
        assert_eq!(g("跨棒距下差"), "0");
        for tag in TOL_TAGS {
            assert_ne!(g(tag), MISSING, "{tag} 不应缺");
        }
        let j = NfTableModel {
            card: "NF内花键参数表".into(),
            expr: None,
            a: spec.a,
            m: spec.m,
            z: spec.z,
            centering: None,
            root: None,
            fit: None,
            at: None,
            rot: 0.0,
        }
        .preview_json()
        .unwrap();
        assert_eq!(j["items"].as_array().unwrap().len(), 18);
        assert_eq!(j["missing"].as_array().unwrap().len(), 0, "锚点 18 项齐全");
        assert_eq!(j["fit"], "fixed", "配合缺省固定");
        assert!(
            j["readout"].as_array().unwrap().iter().any(|r| r["v"]
                .as_str()
                .unwrap()
                .contains("+52/+0")),
            "p29 内花键 E 应进读数：{j}"
        );
        // ② 表外：A=210 m=7.5 不在 p18/p25（表内只有 200/220）→ 公式量照给，表量/跨棒距公差标缺；
        //    ISO 286 R7/H7 与 A 无关，D=195/Az=210 仍算得出 → 不连坐。
        let off = NfTableSpec {
            a: 210.0,
            m: 7.5,
            z: None,
            centering: Centering::Outer,
            root: RootStyle::Flat,
            fit: FitClass::Fixed,
            at: None,
            rot: 0.0,
        };
        let d = derive(&off).unwrap();
        assert!(near(d.az, 210.0) && near(d.d, 195.0), "公式量 Az/D 仍给");
        assert_eq!(d.z, None);
        assert_eq!(d.v, None);
        assert_eq!(d.g, None);
        assert_eq!(d.v1, None);
        assert_eq!(d.g1, None);
        assert_eq!(d.ri, None);
        assert!(d.dims_source.contains("不在 p18"));
        assert!(d.check_source.contains("不在 p23–p25"));
        assert!(d.tol_row.is_none(), "A=210 不在 p29 表 → E 公差标缺（不外推）");
        let vals = values(&off).unwrap();
        let g = |t: &str| vals.iter().find(|(x, _)| x == t).unwrap().1.clone();
        assert_eq!(g("齿数"), MISSING);
        assert_eq!(g("量棒直径V"), MISSING);
        assert_eq!(g("跨棒距G"), MISSING);
        assert_eq!(g("大径Az"), "210");
        assert_eq!(g("小径D"), "195");
        assert_eq!(g("跨棒距上差"), MISSING, "p29 表外 → 跨棒距公差标缺");
        assert_eq!(g("跨棒距下差"), MISSING);
        // ③ 配合类别只影响“配对外花键”读数，不影响内花键自身取值
        for fit in FitClass::ALL {
            let mut s = spec.clone();
            s.fit = fit;
            let pj = NfTableModel {
                card: "NF内花键参数表".into(),
                expr: None,
                a: s.a,
                m: s.m,
                z: s.z,
                centering: None,
                root: None,
                fit: Some(fit.label().to_string()),
                at: None,
                rot: 0.0,
            }
            .preview_json()
            .unwrap();
            assert_eq!(pj["fit"], fit.id());
            assert_eq!(pj["items"][16]["value"], "+0.052", "{fit:?} 不影响内花键 E");
            assert!(
                pj["readout"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["k"].as_str().unwrap().contains(&fit.label())),
                "{fit:?} 配对外花键读数应在"
            );
        }
    }

    /// 报错路径：z 与表值不一致 / z 越界 / 缺 m / 不认识的参数 / 非正数。
    #[test]
    fn nf_errors_are_loud() {
        let e = NfTableSpec::parse("A300 M7.5 Z39").unwrap_err();
        assert!(e.contains("不一致") && e.contains("N=38"), "{e}");
        let e = NfTableSpec::parse("A300 M7.5 Z2").unwrap_err();
        assert!(e.contains("超出范围"), "{e}");
        let e = NfTableSpec::parse("A300").unwrap_err();
        assert!(e.contains("缺模数"), "{e}");
        let e = NfTableSpec::parse("M7.5").unwrap_err();
        assert!(e.contains("缺公称直径"), "{e}");
        let e = NfTableSpec::parse("A300 M7.5 铭牌").unwrap_err();
        assert!(e.contains("不认识的参数"), "{e}");
        let e = NfTableSpec::parse("A-1 M7.5").unwrap_err();
        assert!(e.contains("正数"), "{e}");
        let e = NfTableSpec::parse("").unwrap_err();
        assert!(e.contains("OCSMCARD NF内花键参数表"), "空参应给用法：{e}");
        let e = NfTableSpec::parse("A300 M7.5 中心 偏置").unwrap_err();
        assert!(e.contains("定心方式"), "{e}");
        let e = NfTableSpec::parse("A300 M7.5 根 尖").unwrap_err();
        assert!(e.contains("齿根样式"), "{e}");
        let e = NfTableSpec::parse("A300 M7.5 配合 抱").unwrap_err();
        assert!(e.contains("配合类别"), "{e}");
    }

    /// CLI 解析：短记法 + 中文键 + at/rot；数字与别名。
    #[test]
    fn nf_parse_cli_forms() {
        let s = NfTableSpec::parse("A300 M7.5 Z38").unwrap();
        assert_eq!(s, anchor_spec());
        let s = NfTableSpec::parse("直径 300 模数 7.5 齿数 38 中心 齿面 根 圆 at 10,20 rot 30").unwrap();
        assert!(near(s.a, 300.0) && near(s.m, 7.5) && s.z == Some(38));
        assert_eq!(s.centering, Centering::Flank);
        assert_eq!(s.root, RootStyle::Fillet);
        assert_eq!(s.at, Some([10.0, 20.0]));
        assert!(near(s.rot, 30.0));
        let s = NfTableSpec::parse("a300 m7.5 z38").unwrap();
        assert_eq!(s, anchor_spec());
        let s = NfTableSpec::parse("A300 M7.5 中心 outer 根 flat").unwrap();
        assert_eq!(s.centering, Centering::Outer);
        assert_eq!(s.root, RootStyle::Flat);
        let e = strip_prefix_ci("A300", "a").unwrap();
        assert_eq!(e, "300");
        assert!(strip_prefix_ci("M7.5", "a").is_none());
        // 用法文案包含关键口径
        let u = usage();
        assert!(u.contains("A300 M7.5"), "{u}");
        assert!(u.contains("表外"), "{u}");
        assert!(u.contains("R7") && u.contains("H7"), "{u}");
        assert!(u.contains("配合"), "{u}");
        // 配合类别（四个中文名 + 英文 id）
        for (tok, want) in [
            ("松动", FitClass::Loose),
            ("滑动", FitClass::Slide),
            ("固定", FitClass::Fixed),
            ("压", FitClass::Press),
            ("press", FitClass::Press),
        ] {
            assert_eq!(FitClass::from_token(tok), Some(want), "{tok}");
        }
        assert_eq!(FitClass::from_token("抱"), None);
        assert_eq!(FitClass::default(), FitClass::Fixed);
        let s = NfTableSpec::parse("A300 M7.5 Z38 配合 松动").unwrap();
        assert_eq!(s.fit, FitClass::Loose);
        let s = NfTableSpec::parse("A300 M7.5 fit slide").unwrap();
        assert_eq!(s.fit, FitClass::Slide);
    }

    /// 文字干涉几何检查（照 GB 花键 `main_values_clear_tolerance_column` 的口径）：
    /// 值框不进公差格、标签不越中分隔、全部文字在表格行带与外框内；不止锚点，
    /// **全量 p18 表行**（144 行）+ 齿面定心 + 表外长值都过。
    /// ③ 卡面批：**中英各跑一遍**（英文文字值用实体级字宽、标签用内联 `\W` 压缩后的有效宽）。
    #[test]
    fn nf_values_clear_tolerance_column() {
        assert_eq!(VALUE_WIDTH_FACTOR, 0.7, "值列实体级字宽（不动 OCSM_GB 样式）");
        assert_eq!(TOL_WIDTH_FACTOR, 0.667, "公差列照模板 TEXT");
        let _g = crate::global_state_test_lock();
        for lang in [crate::i18n::Lang::Zh, crate::i18n::Lang::En] {
            crate::i18n::set_lang(lang);
            nf_values_clear_tolerance_column_one_lang(lang);
        }
        crate::i18n::set_lang_auto();
    }

    fn nf_values_clear_tolerance_column_one_lang(lang: crate::i18n::Lang) {
        // 行带边界：14 条水平边界（标题 + 12 内容行）
        let mut sep: Vec<f64> = NF_LINES
            .iter()
            .filter(|(a, b, _)| near5(a[1], b[1]))
            .map(|(_, b, _)| b[1])
            .collect();
        sep.sort_by(|a, b| b.partial_cmp(a).unwrap());
        sep.dedup_by(|a, b| near5(*a, *b));
        let band_of = |y: f64| -> (f64, f64) {
            for w in sep.windows(2) {
                if y <= w[0] + 1e-9 && y >= w[1] - 1e-9 {
                    return (w[0], w[1]);
                }
            }
            panic!("y={y} 不在任何行带");
        };
        // 全量用例：每个 p18 内花键表行 + 锚点（含齿面定心）
        let mut specs: Vec<NfTableSpec> = Vec::new();
        for r in crate::invol_spline::nf_e22141_rows() {
            if r.page == 18 && r.internal_tip.is_some() {
                specs.push(NfTableSpec {
                    a: r.a,
                    m: r.m,
                    z: None,
                    centering: Centering::Outer,
                    root: RootStyle::Flat,
                    fit: FitClass::Fixed,
                    at: None,
                    rot: 0.0,
                });
            }
        }
        specs.push(anchor_spec());
        let mut flank = anchor_spec();
        flank.centering = Centering::Flank;
        specs.push(flank);
        // 表外长值：A=210 → “210”/“195”，加 V/G 标缺的「—」
        specs.push(NfTableSpec {
            a: 210.0,
            m: 7.5,
            z: None,
            centering: Centering::Outer,
            root: RootStyle::Flat,
            fit: FitClass::Fixed,
            at: None,
            rot: 0.0,
        });
        assert!(specs.len() > 140, "覆盖全量 p18 表行");

        let overlap = |a: [f64; 4], b: [f64; 4]| a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3];
        // 中点左对齐框：值/标签（MiddleLeft）；公差为基线左对齐
        let mid_box = |x: f64, y: f64, h: f64, wf: f64, v: &str| {
            [x, y - h / 2.0, x + text_extent(v, h, wf), y + h / 2.0]
        };
        let base_box = |x: f64, y: f64, h: f64, wf: f64, v: &str| {
            [x, y, x + text_extent(v, h, wf), y + h]
        };
        for spec in &specs {
            // 干涉检查用 **块局部**几何（attdefs() 模板）；整表 INSERT 缩放 0.17 是均匀缩放，
            // 相对几何不变，另有 `nf_insert_has_table_scale` 单独锁缩放值。
            let attrs: HashMap<String, _> =
                attdefs().into_iter().map(|a| (a.tag.clone(), a)).collect();
            let vals = values(spec).unwrap();
            let vmap: HashMap<&str, &str> = vals.iter().map(|(t, v)| (t.as_str(), v.as_str())).collect();
            // 12 值框：在右列行带内、不出右框；同行的值框与公差框不相交
            for (i, (_, _, tag, y)) in NF_ROWS.iter().enumerate() {
                let v = vmap[*tag];
                let wf = value_width_factor(tag, VALUE_WIDTH_FACTOR, lang);
                let b = mid_box(VALUE_X, *y, TEXT_H, wf, v);
                let (top, bottom) = band_of(*y);
                assert!(
                    b[1] >= bottom - 1e-9 && b[3] <= top + 1e-9,
                    "{tag}={v} 框 {b:?} 出行为 [{bottom}, {top}]"
                );
                assert!(b[2] <= 0.0 - 0.5, "{tag}={v} 右边界 {:.3} 出右框", b[2]);
                assert!(b[0] >= MID_X + 0.5, "{tag}={v} 左边界进标签列");
                // 同行公差洞位（大径行=第 8 行 / 小径=9 / 跨棒距=11，0-based）
                if let Some(tol_pairs) = [
                    (8usize, "大径上差", "大径下差"),
                    (9, "小径上差", "小径下差"),
                    (11, "跨棒距上差", "跨棒距下差"),
                ]
                .iter()
                .find(|(ri, _, _)| *ri == i)
                {
                    for tol_tag in [tol_pairs.1, tol_pairs.2] {
                        let ta = attrs.get(tol_tag).unwrap();
                        let tv = vmap[tol_tag];
                        let tb = base_box(
                            ta.insertion_point.x,
                            ta.insertion_point.y,
                            TOL_H,
                            TOL_WIDTH_FACTOR,
                            tv,
                        );
                        assert!(
                            !overlap(b, tb),
                            "{tag}={v} 与 {tol_tag}={tv} 叠字：值右 {:.3} / 公差左 {:.3}",
                            b[2],
                            tb[0]
                        );
                        let (ttop, tbottom) = band_of(ta.insertion_point.y + TOL_H / 2.0);
                        assert!(
                            tb[1] >= tbottom - 1e-9 && tb[3] <= ttop + 1e-9,
                            "{tol_tag} 框出行为 [{tbottom}, {ttop}]"
                        );
                    }
                }
            }
            // 12 标签框：左列内、不进中分隔、不出左框（英文侧用内联 `\W` 压缩后的有效字宽）
            for (key, symbol, _, y) in NF_ROWS {
                let label = face_label(key, symbol);
                let inline = label_inline_wf(key, lang).unwrap_or(1.0);
                let b = mid_box(LABEL_X, *y, TEXT_H, 0.7 * inline, &label);
                assert!(b[0] >= FRAME.0 + 0.5, "标签 {label} 出左框");
                assert!(b[2] <= MID_X - 0.5, "标签 {label} 越中分隔：右 {:.3}", b[2]);
                let (top, bottom) = band_of(*y);
                assert!(b[1] >= bottom - 1e-9 && b[3] <= top + 1e-9, "标签 {label} 出行为");
            }
        }
        // 标题框：行 1 内、不越外框
        let title = face_text("card.nf.title.int");
        let half = text_extent(&title, TEXT_H, 0.7) / 2.0;
        let tbox = [MID_X - half, TITLE_AT.1 - TEXT_H / 2.0, MID_X + half, TITLE_AT.1 + TEXT_H / 2.0];
        assert!(tbox[0] >= FRAME.0 + 0.5 && tbox[2] <= FRAME.1 - 0.5, "标题 {title} 出框");
        let (ttop, tbottom) = band_of(TITLE_AT.1);
        assert!(tbox[1] >= tbottom - 1e-9 && tbox[3] <= ttop + 1e-9, "标题出行为");
    }

    /// 值/标签文本宽度估计：字符推进宽度（em，朱雀仿宋实测的保守上限，与 GB 花键卡同款）。
    fn char_em(c: char) -> f64 {
        if c.is_ascii_digit() || c == '.' || c == '+' || c == '-' {
            0.55
        } else if c == ' ' {
            0.25
        } else if c.is_ascii() {
            0.75
        } else {
            1.0 // CJK / 全角
        }
    }

    fn text_extent(value: &str, height: f64, width_factor: f64) -> f64 {
        value.chars().map(char_em).sum::<f64>() * height * width_factor
    }

    /// 预览 JSON 形状（GUI 只渲染）+ 出表 ATTRIB 数 + 待放置件元数据。
    #[test]
    fn nf_preview_and_pending_shape() {
        let m = NfTableModel {
            card: "NF内花键参数表".into(),
            expr: None,
            a: 300.0,
            m: 7.5,
            z: Some(38),
            centering: None,
            root: None,
            fit: None,
            at: Some([12.0, 34.0]),
            rot: 0.0,
        };
        let j = m.preview_json().unwrap();
        assert_eq!(j["ok"], true);
        assert_eq!(j["card"], "NF内花键参数表");
        assert_eq!(j["renderer"], "nf_table");
        assert_eq!(j["centering"], "outer");
        assert_eq!(j["root"], "flat");
        assert_eq!(j["items"].as_array().unwrap().len(), 18);
        assert!(
            j["readout"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["v"].as_str().unwrap().contains("270.508")),
            "读数应含 G/G1：{j}"
        );
        assert!(
            j["readout"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["v"].as_str().unwrap().contains("1.343")),
            "读数应含 ri：{j}"
        );
        let ins = m.build_insert().unwrap();
        assert_eq!(ins.block_name, BLOCK);
        assert_eq!(ins.attributes.len(), 18);
        assert!(near(ins.rotation, 0.0));
        // 整表 INSERT 缩放 0.17（不改块几何；ATTRIB 随 INSERT 缩放：高度/位置 ×0.17）
        assert!(near5(ins.x_scale(), TABLE_SCALE) && near5(ins.y_scale(), TABLE_SCALE) && near5(ins.z_scale(), TABLE_SCALE));
        let att0 = &ins.attributes[0];
        assert!(near5(att0.height, TEXT_H * TABLE_SCALE), "ATTRIB 字高随缩放：{}", att0.height);
        assert!(near5(att0.insertion_point.x, 12.0 + VALUE_X * TABLE_SCALE), "ATTRIB 位置随缩放（+ 基点平移）");
        // 多重集断言：preview items 的 tag 集合 == ATTDEF tag 集合（无漏无重、18 项）
        let mut item_tags: Vec<String> = j["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|it| it["tag"].as_str().unwrap().to_string())
            .collect();
        let mut att_tags: Vec<String> = attdefs().iter().map(|ad| ad.tag.clone()).collect();
        item_tags.sort();
        att_tags.sort();
        assert_eq!(item_tags, att_tags, "口径表 ↔ 属性表多重集不一致");
        assert_eq!(att_tags.len(), 18);
        assert_eq!(m.pending_attrs().unwrap().len(), 18);
        let meta = m.part_meta_json().unwrap();
        assert!(meta.contains("\"family\":\"nf_table\""));
        let echo = m.echo_note().unwrap();
        assert!(echo.contains("A=300") && echo.contains("z=38"), "{echo}");
        let md = markdown_table(&m.spec().unwrap()).unwrap();
        assert_eq!(md.lines().count(), 20, "18 行 + 表头 + 分隔");
    }

    fn expr_model(expr: &str) -> NfTableModel {
        NfTableModel {
            card: "NF内花键参数表".into(),
            expr: Some(expr.into()),
            a: 0.0,
            m: 0.0,
            z: None,
            centering: None,
            root: None,
            fit: None,
            at: None,
            rot: 0.0,
        }
    }

    /// ★ 表达式 → 字段 → 卡内值：A=m(z+0.4+2x)、m、z；锚点 A300/M7.5/Z38 逐项。
    #[test]
    fn nf_expr_maps_to_anchor_a_m_z() {
        let spec = expr_model("SPLINE IN M7.5 Z38 ALPHA20 X0.8 BETA0 H30")
            .spec()
            .unwrap();
        assert!((spec.a - 300.0).abs() < 1e-9, "A={}", spec.a);
        assert!((spec.m - 7.5).abs() < 1e-12);
        assert_eq!(spec.z, Some(38));
        let d = derive(&spec).unwrap();
        assert!(near(d.g.unwrap(), 270.508));
        assert!(near(d.v.unwrap(), 15.0));
        assert!(near(d.ri.unwrap(), 1.343));
        // 卡内值（逐项）
        let vals = values(&spec).unwrap();
        let get = |tag: &str| vals.iter().find(|(t, _)| t == tag).unwrap().1.clone();
        assert_eq!(get("模数"), "7.5");
        assert_eq!(get("齿数"), "38");
        assert_eq!(get("压力角"), "20°");
        assert_eq!(get("大径Az"), "300");
        assert_eq!(get("小径D"), "285");
        assert_eq!(get("跨棒距G"), "270.508");
        // 与旧 A/M/Z 输入同值（CLI 两条路）
        let old = NfTableSpec::parse("A300 M7.5 Z38").unwrap();
        assert_eq!(values(&old).unwrap(), values(&spec).unwrap());
        // 预览回填 fields + expr 回显
        let pj = expr_model("SPLINE IN M7.5 Z38 ALPHA20 X0.8 BETA0 H30")
            .preview_json()
            .unwrap();
        assert!(near(pj["fields"]["a"].as_f64().unwrap(), 300.0));
        assert!(near(pj["fields"]["m"].as_f64().unwrap(), 7.5));
        assert_eq!(pj["fields"]["z"], 38);
        assert!(pj["expr"].as_str().unwrap().starts_with("SPLINE IN"));
        // CLI 表达式写法
        let cli = NfTableSpec::parse("SPLINE IN M7.5 Z38 ALPHA20 X0.8 BETA0 H30 中心 齿面").unwrap();
        assert!(near(cli.a, 300.0) && cli.z == Some(38));
        assert_eq!(cli.centering, Centering::Flank);
    }

    /// 表达式错误路径：KIND/MARK/α/显式冲突/缺失/表外。
    #[test]
    fn nf_expr_error_paths_and_off_table() {
        // KIND EX 与 NF 内花键卡不一致
        let e = expr_model("SPLINE EX M7.5 Z38 ALPHA20 X0.8 BETA0 H30")
            .spec()
            .unwrap_err();
        assert!(e.contains("KIND") && e.contains("EX（外）") && e.contains("「内」"), "{e}");
        // MARK 体系不符
        let e = expr_model("GEAR IN M7.5 Z38 ALPHA20 X0.8 BETA0 H30")
            .spec()
            .unwrap_err();
        assert!(e.contains("MARK") && e.contains("GEAR（齿轮）"), "{e}");
        // α 必须 20
        let e = expr_model("SPLINE IN M7.5 Z38 ALPHA30 X0.8 BETA0 H30")
            .spec()
            .unwrap_err();
        assert!(e.contains("压力角 30°") && e.contains("α=20"), "{e}");
        // 模型路径：有表达式时字段以表达式为准（GUI 回填前的旧值不参与校验）
        let mut m = expr_model("SPLINE IN M7.5 Z38 ALPHA20 X0.8 BETA0 H30");
        m.z = Some(40);
        assert_eq!(m.spec().unwrap().z, Some(38), "表达式覆盖显式 z");
        // CLI 显式冲突
        let e = NfTableSpec::parse("SPLINE IN M7.5 Z38 ALPHA20 X0.8 BETA0 H30 Z40").unwrap_err();
        assert!(e.contains("显式 z=40") && e.contains("z=38"), "{e}");
        // 缺表达式且 A=0 → 现有校验路径
        let e = NfTableModel {
            card: "NF内花键参数表".into(),
            expr: None,
            a: 0.0,
            m: 7.5,
            z: None,
            centering: None,
            root: None,
            fit: None,
            at: None,
            rot: 0.0,
        }
        .spec()
        .unwrap_err();
        assert!(e.contains("A=0"), "{e}");
        // 表外（表达式 z=40 → A=309，不在 p18 表）→ V/G/ri 如实标「—」，不报错也不臆造
        let off = expr_model("SPLINE IN M7.5 Z40 ALPHA20 X0.8 BETA0 H30")
            .spec()
            .unwrap();
        let vals = values(&off).unwrap();
        let get = |tag: &str| vals.iter().find(|(t, _)| t == tag).unwrap().1.clone();
        assert_eq!(get("量棒直径V"), MISSING);
        assert_eq!(get("跨棒距G"), MISSING);
    }

    /// ★ 负断言（用户 2026-09-27 判定）：卡底**不得**再出现「p29 四配合全表条带」。
    /// 内/外两卡都回到模板同构：97 图元 = 66 线 + 13 MTEXT + 18 ATTDEF；18 项取值。
    #[test]
    fn nf_cards_have_no_p29_fit_strip() {
        let strip_mark = |value: &str| value.contains("p29 配合偏差") || value == "E（µm）" || value == "xm（µm）";
        let strip_tag = |tag: &str| {
            ["E内花键", "E松动", "E滑动", "E固定", "E压", "xm内花键", "xm松动", "xm滑动", "xm固定", "xm压"]
                .iter()
                .any(|p| tag.starts_with(p))
        };
        // 内卡
        let int_ents = block_entities();
        assert_eq!(int_ents.len(), 66 + 13 + 18, "内卡回到模板 97 图元");
        let int_texts: Vec<&str> = int_ents
            .iter()
            .filter_map(|e| match e {
                EntityType::MText(m) => Some(m.value.as_str()),
                _ => None,
            })
            .collect();
        assert!(int_texts.iter().all(|t| !strip_mark(t)), "内卡仍有条带文字：{int_texts:?}");
        let int_tags: Vec<String> = attdefs().iter().map(|a| a.tag.clone()).collect();
        assert_eq!(int_tags.len(), 18);
        assert!(int_tags.iter().all(|t| !strip_tag(t)), "内卡仍有条带属性：{int_tags:?}");
        let int_vals = values(&anchor_spec()).unwrap();
        assert_eq!(int_vals.len(), 18);
        assert!(int_vals.iter().all(|(t, v)| !strip_tag(t) && !v.contains('µ')));
        // 外卡
        let ext_ents = crate::nf_ext_table::block_entities();
        assert_eq!(ext_ents.len(), 66 + 13 + 18, "外卡回到模板 97 图元");
        let ext_texts: Vec<&str> = ext_ents
            .iter()
            .filter_map(|e| match e {
                EntityType::MText(m) => Some(m.value.as_str()),
                _ => None,
            })
            .collect();
        assert!(ext_texts.iter().all(|t| !strip_mark(t)), "外卡仍有条带文字：{ext_texts:?}");
        let ext_tags: Vec<String> = crate::nf_ext_table::attdefs().iter().map(|a| a.tag.clone()).collect();
        assert_eq!(ext_tags.len(), 18);
        assert!(ext_tags.iter().all(|t| !strip_tag(t)), "外卡仍有条带属性：{ext_tags:?}");
        let ext_spec = crate::nf_ext_table::NfExtTableSpec {
            a: 300.0,
            m: 7.5,
            z: Some(38),
            centering: Centering::Flank,
            root: RootStyle::Flat,
            fit: FitClass::Fixed,
            at: None,
            rot: 0.0,
        };
        let ext_vals = crate::nf_ext_table::values(&ext_spec).unwrap();
        assert_eq!(ext_vals.len(), 18);
        assert!(ext_vals.iter().all(|(t, v)| !strip_tag(t) && !v.contains('µ')));
        // 预览 items 也回到 18（内 6 值 + 6 公差 = 18；无条带 20）
        let j = NfTableModel {
            card: "NF内花键参数表".into(),
            expr: None,
            a: 300.0,
            m: 7.5,
            z: Some(38),
            centering: None,
            root: None,
            fit: None,
            at: None,
            rot: 0.0,
        }
        .preview_json()
        .unwrap();
        assert_eq!(j["items"].as_array().unwrap().len(), 18);
    }

    /// ★ 几何：内/外两卡所有 MTEXT 参考框宽 ≥ 文本宽（不折行）且框不出表；标签互不叠。
    /// ③ 卡面批：**中英各跑一遍**（文本宽 = 模板样式 0.7 × 内联 `\W` 压缩后的**有效宽**）。
    #[test]
    fn nf_cards_mtext_no_wrap_and_no_overlap() {
        let _g = crate::global_state_test_lock();
        for lang in [crate::i18n::Lang::Zh, crate::i18n::Lang::En] {
            crate::i18n::set_lang(lang);
            nf_cards_mtext_no_wrap_and_no_overlap_one_lang(lang);
        }
        crate::i18n::set_lang_auto();
    }

    fn nf_cards_mtext_no_wrap_and_no_overlap_one_lang(lang: crate::i18n::Lang) {
        for (name, ents) in [
            ("NF内", block_entities()),
            ("NF外", crate::nf_ext_table::block_entities()),
        ] {
            let mut boxes: Vec<(&str, [f64; 4])> = Vec::new();
            for e in &ents {
                let EntityType::MText(m) = e else { continue };
                let (inline, plain) = mtext_inline_wf(&m.value);
                let w = text_extent(plain, m.height, 0.7 * inline);
                assert!(
                    m.rectangle_width == 0.0 || m.rectangle_width + 1e-9 >= w,
                    "{lang:?} {name} MTEXT {:?} 参考框宽 {:.2} < 文本宽 {:.2}（会折行）",
                    plain,
                    m.rectangle_width,
                    w
                );
                let (x, y, h) = (m.insertion_point.x, m.insertion_point.y, m.height);
                let b = match m.attachment_point {
                    AttachmentPoint::MiddleCenter => [x - w / 2.0, y - h / 2.0, x + w / 2.0, y + h / 2.0],
                    AttachmentPoint::MiddleLeft => [x, y - h / 2.0, x + w, y + h / 2.0],
                    other => panic!("{lang:?} {name} MTEXT {plain:?} 未覆盖的对齐 {other:?}"),
                };
                assert!(
                    b[0] >= FRAME.0 - 1e-6 && b[2] <= FRAME.1 + 1e-6
                        && b[1] >= FRAME.3 - 1e-6 && b[3] <= FRAME.2 + 1e-6,
                    "{lang:?} {name} MTEXT {plain:?} 出表：{b:?}"
                );
                boxes.push((plain, b));
            }
            for i in 0..boxes.len() {
                for j in i + 1..boxes.len() {
                    let (a, b) = (boxes[i].1, boxes[j].1);
                    assert!(
                        !(a[0] < b[2] - 1e-9 && b[0] < a[2] - 1e-9
                            && a[1] < b[3] - 1e-9 && b[1] < a[3] - 1e-9),
                        "{lang:?} {name} MTEXT {:?} × {:?} 叠字",
                        boxes[i].0,
                        boxes[j].0
                    );
                }
            }
        }
    }

    /// ③ 卡面批：内卡**中英双断言** —— 标题/12 标签/14 条符号随语言切换、英文侧零汉字；
    /// 标准号/数值/「—」/ATTDEF tag 两语逐项相等；`missing_keys()` 为空（无漏 key）。
    #[test]
    fn nf_card_faces_switch_language_keeping_symbols_and_values() {
        use crate::i18n::{set_lang, set_lang_auto, Lang};
        let _g = crate::global_state_test_lock();
        let cjk = |s: &str| s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
        let mtexts = |ents: &[EntityType]| -> Vec<String> {
            ents.iter()
                .filter_map(|e| match e {
                    EntityType::MText(m) => Some(mtext_inline_wf(&m.value).1.to_string()),
                    _ => None,
                })
                .collect()
        };
        let tags = |ents: &[EntityType]| -> Vec<String> {
            ents.iter()
                .filter_map(|e| match e {
                    EntityType::AttributeDefinition(a) => Some(a.tag.clone()),
                    _ => None,
                })
                .collect()
        };
        // 符号集合（两语原样；NF 原文符号）
        let syms: &[&str] = &["m", "z", "a", "Az", "D", "Do", "V", "G"];
        let syms_ext: &[&str] = &["m", "z", "a", "Dee", "Die", "Do", "K", "W"];
        for (name, int_card) in [("NF内", true), ("NF外", false)] {
            let build = |_card: bool| -> Vec<EntityType> {
                if int_card {
                    block_entities()
                } else {
                    crate::nf_ext_table::block_entities()
                }
            };
            let rows: &[(&str, &str, &str, f64)] = if int_card {
                NF_ROWS
            } else {
                crate::nf_ext_table::rows()
            };
            set_lang(Lang::Zh);
            let zh = mtexts(&build(int_card));
            let zh_tags = tags(&build(int_card));
            set_lang(Lang::En);
            let en = mtexts(&build(int_card));
            let en_tags = tags(&build(int_card));
            assert_eq!(zh.len(), 13, "{name} 标题 + 12 标签");
            assert_eq!(en.len(), 13);
            assert_eq!(zh_tags, en_tags, "{name} ATTDEF tag 不随语言");
            assert!(!en.iter().any(|s| cjk(s)), "{name} 英文卡面零汉字：{en:?}");
            // 中文侧：12 条标签原文（不含内联码）逐条在
            set_lang(Lang::Zh);
            for (key, symbol, _, _) in rows {
                let want = face_label(key, symbol);
                assert!(zh.iter().any(|s| s == &want), "{name} zh 卡面缺「{want}」：{zh:?}");
            }
            set_lang(Lang::En);
            for (key, symbol, _, _) in rows {
                let want = face_label(key, symbol);
                assert!(en.iter().any(|s| s == &want), "{name} en 卡面缺「{want}」：{en:?}");
            }
            for s in if int_card { syms } else { syms_ext } {
                assert!(zh.iter().any(|t| t.split_whitespace().last() == Some(*s)),
                        "{name} zh 符号 {s} 原样：{zh:?}");
                assert!(en.iter().any(|t| t.split_whitespace().last() == Some(*s)),
                        "{name} en 符号 {s} 原样：{en:?}");
            }
            // 值：标准号/数值/缺项与枚举显示值（随语言）
            let int_spec = anchor_spec();
            let ext_spec = crate::nf_ext_table::NfExtTableSpec {
                a: 300.0,
                m: 7.5,
                z: Some(38),
                centering: Centering::Flank,
                root: RootStyle::Flat,
                fit: FitClass::Fixed,
                at: None,
                rot: 0.0,
            };
            let vals_of = |card: bool| -> HashMap<String, String> {
                let v = if card {
                    values(&int_spec).unwrap()
                } else {
                    crate::nf_ext_table::values(&ext_spec).unwrap()
                };
                v.into_iter().collect()
            };
            set_lang(Lang::Zh);
            let zh_vals = vals_of(int_card);
            set_lang(Lang::En);
            let en_vals = vals_of(int_card);
            assert_eq!(zh_vals.len(), 18, "{name} 18 项");
            for (tag, zh_v) in &zh_vals {
                let en_v = en_vals.get(tag).expect("同 tag 集合");
                if ["定心方式", "齿根样式", "加工方法"].contains(&tag.as_str()) {
                    assert_ne!(zh_v, en_v, "{name} {tag} 是枚举显示值，应随语言");
                } else {
                    assert_eq!(zh_v, en_v, "{name} {tag} 是数据/标准号，应两语相同");
                }
            }
            assert_eq!(zh_vals.get("执行标准").map(String::as_str), Some("NF E22-141"));
        }
        // ★ 只查**本批 NF 键**未漏（`missing_keys()` 是进程全局，其它并行用例会故意点
        //   `no.such.key.at.all` / `data:…` ⇒ 不能断言全局为空，否则偶发假红）。
        let missed = crate::i18n::missing_keys();
        assert!(
            !missed.iter().any(|k| k.starts_with("card.nf.")),
            "NF 键有漏：{missed:?}"
        );
        set_lang_auto();
    }
}
