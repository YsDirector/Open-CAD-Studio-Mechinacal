//! 智能卡片「ANSI 花键参数表」：**纯中文版 / 纯英文版**两个卡类型（内/外共用渲染器）。
//!
//! 模板（用户做的**中英双语**版，本卡按语种拆开）：
//! `~/桌面/GB/参数表/内花键参数表ANSI.dxf` / `外花键参数表ANSI.dxf`
//! （各 26 LINE + 20 MTEXT + 17 ATTDEF；几何逐图元反解见下方 const）。
//!
//! **拆分口径**：模板每条 MTEXT 是「中文 + 英文」一段（如 `花键类型 FLAT ROOT SIDE FIT`）；
//! 纯中文版只出中文段、纯英文版只出英文段，位置/字高/层/对齐照模板。两处例外：
//! * 模板 `跨棒距`/`公法线长度` 的中英文是**两条 MTEXT**（英文在下一行）——两版各取其一条；
//! * `花键类型` 行的英文段模板写的是样例值 `FLAT ROOT SIDE FIT`，本卡按「行名」译为
//!   `SPLINE TYPE`（值由属性给）；标题/`MIN MINEFFECTIVE` 的明显拼写错误一并更正
//!   （`INTERANL INOLUTE`→`INTERNAL INVOLUTE`、`MIN MINEFFECTIVE`→`MIN EFFECTIVE`）。
//!
//! 取值来源（用户 2026-09-26 口径：**先看仓内已有 ANSI 实现，够用就复用；缺的如实标缺**）：
//! * 径节 `P`/齿数 `N`/压力角/基圆/节圆/大径/小径/有效直径（form dia）：`invol_spline.rs`
//!   的 ANSI B92.1 Table 2 公式（`InvolParams::ansi(profile, P, z)` 同一套）；
//! * 齿形类型（平/圆齿根 + 齿侧/外径配合）：`AnsiColumn::from_profile`；
//! * 配合/公差（Table 4/5）、量棒检验（p30–p32）、公法线跨测表：**本仓未收** → 相关格
//!   一律 `MISSING`（「—」），**不臆造**。
//!
//! 所有文本样式一律 `OCSM_GB`；值属性用**实体级** `VALUE_WIDTH_FACTOR` 压缩防串列。

use crate::invol_spline::{AnsiColumn, InvolParams, SplineStd};
use crate::spline_tol::SplineSide;
use ocs_plugin_api::host::acadrust::entities::mtext::AttachmentPoint;
use ocs_plugin_api::host::acadrust::entities::{
    AttributeDefinition, AttributeEntity, Entity as _, EntityCommon, EntityType, Insert, MText,
};
use ocs_plugin_api::host::acadrust::types::{Color, LineWeight, Vector3};

/// 缺项标记（本仓没有的 ANSI 表数据一律显示它；GUI/回执点明原因 —— 不臆造）。
pub const MISSING: &str = "—";

/// 值属性**实体级**字宽压缩（不动全局 `OCSM_GB` 样式；中列可用宽 ~16.4）。
pub const VALUE_WIDTH_FACTOR: f64 = 0.5;

/// `花键类型` 值的**实体级**字宽：英文全称 `FLAT ROOT MAJOR DIA FIT`（最长档）
/// 在 16.4 宽的中列里必须压到 0.32 才不出格（OCS 出图 + DXF 文本盒检查后定的档）。
pub const TYPE_WIDTH_FACTOR: f64 = 0.32;

/// 语种：纯中文 / 纯英文。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsiLang {
    Cn,
    En,
}

impl AnsiLang {
    pub fn id(self) -> &'static str {
        match self {
            AnsiLang::Cn => "cn",
            AnsiLang::En => "en",
        }
    }
    pub fn label(self) -> String {
        crate::i18n::t(match self {
            AnsiLang::Cn => "card.ansi.lang.cn",
            AnsiLang::En => "card.ansi.lang.en",
        })
    }
    pub fn from_id(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "cn" | "zh" | "中文" => Some(AnsiLang::Cn),
            "en" | "英文" => Some(AnsiLang::En),
            _ => None,
        }
    }
}

/// 4 个块名（内/外 × 中/英）。
pub const BLOCK_INT_CN: &str = "OCSM_ANSI_INT_CN";
pub const BLOCK_INT_EN: &str = "OCSM_ANSI_INT_EN";
pub const BLOCK_EXT_CN: &str = "OCSM_ANSI_EXT_CN";
pub const BLOCK_EXT_EN: &str = "OCSM_ANSI_EXT_EN";

/// 按方向 + 语种取块名。
pub fn block_name(side: SplineSide, lang: AnsiLang) -> &'static str {
    match (side, lang) {
        (SplineSide::Internal, AnsiLang::Cn) => BLOCK_INT_CN,
        (SplineSide::Internal, AnsiLang::En) => BLOCK_INT_EN,
        (SplineSide::External, AnsiLang::Cn) => BLOCK_EXT_CN,
        (SplineSide::External, AnsiLang::En) => BLOCK_EXT_EN,
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 模板数据（逐图元反解；不依赖外部 DXF）
// ══════════════════════════════════════════════════════════════════════════

/// ANSI 模板 26 条 LINE（内/外模板手绘偏差 ≤5e-14，取内花键一份；`(a, b, layer)`）。
const ANSI_LINES: &[([f64; 2], [f64; 2], &str)] = &[
    ([-39.32678800844012, -13.79429999999997], [-39.32678800844012, -82.76579999999993], "2细线层"),
    ([-21.79359502134392, -41.38289999999995], [-21.79359502134392, -68.97149999999992], "2细线层"),
    ([-21.79359502134392, -75.86864999999992], [-21.79359502134392, -82.76579999999991], "2细线层"),
    ([-21.79359502134392, -89.66294999999991], [-21.79359502134392, -124.1486999999999], "2细线层"),
    ([-79.92973028775381, 1.42e-14], [-1.42e-14, 1.42e-14], "1轮廓实线层"),
    ([-79.92973028775381, -20.69144999999997], [-1.42e-14, -20.69144999999997], "2细线层"),
    ([-79.92973028775381, -27.58859999999996], [-1.42e-14, -27.58859999999996], "2细线层"),
    ([-79.92973028775381, -34.48574999999995], [-1.42e-14, -34.48574999999995], "2细线层"),
    ([-79.92973028775381, -41.38289999999995], [-1.42e-14, -41.38289999999995], "2细线层"),
    ([-79.92973028775381, -48.28004999999993], [-1.42e-14, -48.28004999999993], "2细线层"),
    ([-79.92973028775381, -55.17719999999993], [-1.42e-14, -55.17719999999993], "2细线层"),
    ([-21.79359502134392, -62.07434999999993], [-1.42e-14, -62.07434999999993], "2细线层"),
    ([-79.92973028775381, -68.97149999999992], [-1.42e-14, -68.97149999999992], "2细线层"),
    ([-79.92973028775381, -75.86864999999992], [-1.42e-14, -75.86864999999992], "2细线层"),
    ([-79.92973028775381, -82.76579999999991], [-1.42e-14, -82.76579999999991], "2细线层"),
    ([-79.92973028775381, -89.66294999999991], [-1.42e-14, -89.66294999999991], "2细线层"),
    ([-79.92973028775381, -96.56009999999989], [-1.42e-14, -96.56009999999989], "2细线层"),
    ([-79.92973028775381, -103.4572499999999], [-1.42e-14, -103.4572499999999], "2细线层"),
    ([-21.79359502134392, -110.3543999999999], [-1.42e-14, -110.3543999999999], "2细线层"),
    ([-79.92973028775381, -117.2515499999999], [-1.42e-14, -117.2515499999999], "2细线层"),
    ([-39.32678800844012, -103.4572499999999], [-39.32678800844012, -117.2515499999999], "2细线层"),
    ([-79.92973028775381, -124.1486999999999], [-1.42e-14, -124.1486999999999], "1轮廓实线层"),
    ([-1.42e-14, 1.42e-14], [-1.42e-14, -124.1486999999999], "1轮廓实线层"),
    ([-79.92973028775381, 1.42e-14], [-79.92973028775381, -124.1486999999999], "1轮廓实线层"),
    ([-79.92973028775381, -13.79429999999997], [-1.42e-14, -13.79429999999997], "2细线层"),
    ([-79.92973028775381, -6.897149999999979], [-1.42e-14, -6.897149999999979], "2细线层"),
];

/// ANSI 内花键模板的 20 条 MTEXT 卡面标签（**块语种钉死**；`(key, pin, ins_x, ins_y, h, wf, layer, attach, rect_w)`）。
///
/// * `key` = catalog `card.ansi.label.*`（zh = 纯中文块原文、en = 纯英文块原文，两语都在表里）；
/// * `pin`：`b` = 两块都有（同 key 取块语种那一侧） / `c` = 只有中文块 / `e` = 只有英文块
///   —— 英文把「跨棒距 / 公法线长度」换行成第二行，故同一 key 两行、几何各按双语模板原值；
/// * `wf` 列是模板参考值（MTEXT 实际用 `rect_w` 折行框，见 `mtext_ent`），位置/字高/层照模板。
const ANSI_LABELS_INT: &[(&str, char, f64, f64, f64, f64, &str, i16, f64)] = &[
    ("card.ansi.label.spline_type", 'b', -78.66410528775381, -19.42582499999996, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.teeth", 'b', -78.66410528775381, -26.32297499999996, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.base_dia", 'b', -78.66410528775381, -47.01442499999994, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.ref", 'b', -10.89679751067196, -47.01442499999994, 3.274425, 19.2623450213439, "6文字层", 8, 19.2623450213439),
    ("card.ansi.label.pitch_dia", 'b', -78.66410528775381, -53.91157499999994, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.ref", 'b', -10.89679751067196, -53.91157499999994, 3.274425, 19.2623450213439, "6文字层", 8, 19.2623450213439),
    ("card.ansi.label.major_dia", 'b', -78.66410528775381, -62.07434999999993, 3.274425, 38.0716922793137, "6文字层", 4, 38.0716922793137),
    ("card.ansi.label.max_actual", 'b', -78.66410528775381, -95.29447499999989, 3.274425, 55.6048852664099, "6文字层", 7, 55.6048852664099),
    ("card.ansi.label.over_pins", 'c', -78.66410528775381, -109.0887749999999, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.over_pins", 'e', -78.6641052877538, -115.9859249999999, 3.1, 43.69900654265613, "6文字层", 7, 43.69900654265613),
    ("card.ansi.label.pin_dia", 'b', -78.66410528775381, -122.8830749999999, 3.274425, 55.6048852664099, "6文字层", 7, 55.6048852664099),
    ("card.ansi.label.min_effective", 'b', -78.66410528775381, -102.1916249999999, 3.274425, 55.6048852664099, "6文字层", 7, 55.6048852664099),
    ("card.ansi.label.circular_thickness", 'b', -78.66410528775381, -88.39732499999991, 3.274425, 77.3984802877538, "6文字层", 7, 77.3984802877538),
    ("card.ansi.label.min", 'b', -10.89679751067196, -81.50017499999991, 3.274425, 19.2623450213439, "6文字层", 8, 19.2623450213439),
    ("card.ansi.label.minor_dia", 'b', -78.66410528775381, -81.50017499999991, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.form_dia", 'b', -78.66410528775381, -74.60302499999992, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.pressure_angle", 'b', -78.66410528775381, -40.11727499999994, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.pitch", 'b', -78.66410528775381, -33.22012499999996, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.standard", 'b', -39.96486514387692, -12.52867499999997, 3.274425, 77.3984802877538, "6文字层", 8, 77.3984802877538),
    ("card.ansi.label.title_internal", 'b', -39.96486514387692, -5.631524999999979, 3.274425, 77.3984802877538, "6文字层", 8, 77.3984802877538),
];

/// ANSI 内花键模板的 17 个 ATTDEF（**自上而下**排序；`(tag, ins_x, ins_y, h, wf, layer)`）。
const ANSI_ATTDEFS_INT: &[(&str, f64, f64, f64, f64, &str)] = &[
    ("量棒直径", -20.62448762993647, -122.4550306395315, 3.274425, 0.707, "6文字层"),
    ("跨棒距下差", -20.62448762993647, -115.5578806395315, 3.274425, 0.707, "6文字层"),
    ("跨棒距", -38.1576806170327, -111.7010477747637, 3.274425, 0.707, "6文字层"),
    ("跨棒距上差", -20.62448762993647, -108.6607306395315, 3.274425, 0.707, "6文字层"),
    ("作用齿厚最小值", -20.62448762993647, -101.7635806395315, 3.274425, 0.707, "6文字层"),
    ("实际齿厚最大值", -20.62448762993647, -94.86643063953159, 3.274425, 0.707, "6文字层"),
    ("小径", -38.15768061703269, -81.07213063953158, 3.274425, 0.707, "6文字层"),
    ("有效直径", -38.15768061703268, -74.1749806395316, 3.274425, 0.707, "6文字层"),
    ("大径下差", -20.62448762993647, -67.2778306395316, 3.274425, 0.707, "6文字层"),
    ("大径", -38.15768061703268, -63.72427315782444, 3.274425, 0.707, "6文字层"),
    ("大径上差", -20.62448762993647, -60.3806806395316, 3.274425, 0.707, "6文字层"),
    ("节圆直径", -38.15768061703268, -53.48353063953162, 3.274425, 0.707, "6文字层"),
    ("基圆直径", -38.15768061703268, -46.58638063953161, 3.274425, 0.707, "6文字层"),
    ("压力角", -38.15768061703268, -39.68923063953163, 3.274425, 0.707, "6文字层"),
    ("径节", -38.15768061703268, -32.79208063953163, 3.274425, 0.707, "6文字层"),
    ("齿数", -38.15768061703268, -25.89493063953164, 3.274425, 0.707, "6文字层"),
    ("花键类型", -38.15768061703268, -18.99778063953164, 3.274425, 0.707, "6文字层"),
];

/// ANSI 外花键模板的 20 条 MTEXT 卡面标签（**块语种钉死**；`(key, pin, ins_x, ins_y, h, wf, layer, attach, rect_w)`）。
///
/// * `key` = catalog `card.ansi.label.*`（zh = 纯中文块原文、en = 纯英文块原文，两语都在表里）；
/// * `pin`：`b` = 两块都有（同 key 取块语种那一侧） / `c` = 只有中文块 / `e` = 只有英文块
///   —— 英文把「跨棒距 / 公法线长度」换行成第二行，故同一 key 两行、几何各按双语模板原值；
/// * `wf` 列是模板参考值（MTEXT 实际用 `rect_w` 折行框，见 `mtext_ent`），位置/字高/层照模板。
const ANSI_LABELS_EXT: &[(&str, char, f64, f64, f64, f64, &str, i16, f64)] = &[
    ("card.ansi.label.title_external", 'b', -39.96486514387692, -5.63152500000001, 3.274425, 77.3984802877538, "6文字层", 8, 77.3984802877538),
    ("card.ansi.label.standard", 'b', -39.96486514387692, -12.52867499999996, 3.274425, 77.3984802877538, "6文字层", 8, 77.3984802877538),
    ("card.ansi.label.pitch", 'b', -78.66410528775378, -33.22012499999997, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.pressure_angle", 'b', -78.66410528775378, -40.11727499999994, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.form_dia_ext", 'b', -78.66410528775378, -74.60302499999995, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.minor_dia", 'b', -78.66410528775378, -81.50017499999993, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.min", 'b', -10.89679751067194, -81.50017499999993, 3.274425, 19.2623450213439, "6文字层", 8, 19.2623450213439),
    ("card.ansi.label.circular_thickness", 'b', -78.66410528775378, -88.39732499999991, 3.274425, 77.3984802877538, "6文字层", 7, 77.3984802877538),
    ("card.ansi.label.min_actual", 'b', -78.66410528775378, -102.1916249999999, 3.274425, 55.6048852664099, "6文字层", 7, 55.6048852664099),
    ("card.ansi.label.spline_type", 'b', -78.66410528775378, -19.42582499999997, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.teeth", 'b', -78.66410528775378, -26.32297499999999, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.base_dia", 'b', -78.66410528775378, -47.01442499999996, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.ref", 'b', -10.89679751067194, -47.01442499999996, 3.274425, 19.2623450213439, "6文字层", 8, 19.2623450213439),
    ("card.ansi.label.pitch_dia", 'b', -78.66410528775378, -53.91157499999994, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.ref", 'b', -10.89679751067194, -53.91157499999994, 3.274425, 19.2623450213439, "6文字层", 8, 19.2623450213439),
    ("card.ansi.label.major_dia", 'b', -78.66410528775378, -62.07434999999995, 3.274425, 38.0716922793137, "6文字层", 4, 38.0716922793137),
    ("card.ansi.label.max_effective", 'b', -78.66410528775378, -95.29447499999992, 3.274425, 55.6048852664099, "6文字层", 7, 55.6048852664099),
    ("card.ansi.label.base_tangent", 'c', -78.66410528775378, -109.0887749999999, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.base_tangent", 'e', -78.66410528775378, -115.9859249999999, 3.274425, 38.0716922793137, "6文字层", 7, 38.0716922793137),
    ("card.ansi.label.span_teeth", 'b', -78.66410528775378, -122.8830749999999, 3.274425, 55.6048852664099, "6文字层", 7, 55.6048852664099),
];

/// ANSI 外花键模板的 17 个 ATTDEF（**自上而下**排序；`(tag, ins_x, ins_y, h, wf, layer)`）。
const ANSI_ATTDEFS_EXT: &[(&str, f64, f64, f64, f64, &str)] = &[
    ("跨测齿数", -20.62448762993648, -122.4550306395316, 3.274425, 0.707, "6文字层"),
    ("公法线下差", -20.62448762993648, -115.5578806395315, 3.274425, 0.707, "6文字层"),
    ("公法线长度", -38.15768061703267, -111.7010477747637, 3.274425, 0.707, "6文字层"),
    ("公法线上差", -20.62448762993648, -108.6607306395316, 3.274425, 0.707, "6文字层"),
    ("实际齿厚最小值", -20.62448762993648, -101.7635806395316, 3.274425, 0.707, "6文字层"),
    ("作用齿厚最大值", -20.62448762993648, -94.86643063953159, 3.274425, 0.707, "6文字层"),
    ("小径", -38.15768061703267, -81.07213063953162, 3.274425, 0.707, "6文字层"),
    ("渐开线终止圆直径", -38.15768061703267, -74.17498063953161, 3.274425, 0.707, "6文字层"),
    ("大径下差", -20.62448762993648, -67.2778306395316, 3.274425, 0.707, "6文字层"),
    ("大径", -38.15768061703267, -63.72427315782445, 3.274425, 0.707, "6文字层"),
    ("大径上差", -20.62448762993648, -60.38068063953161, 3.274425, 0.707, "6文字层"),
    ("节圆直径", -38.15768061703267, -53.48353063953164, 3.274425, 0.707, "6文字层"),
    ("基圆直径", -38.15768061703267, -46.58638063953162, 3.274425, 0.707, "6文字层"),
    ("压力角", -38.15768061703267, -39.68923063953165, 3.274425, 0.707, "6文字层"),
    ("径节", -38.15768061703267, -32.79208063953166, 3.274425, 0.707, "6文字层"),
    ("齿数", -38.15768061703267, -25.89493063953165, 3.274425, 0.707, "6文字层"),
    ("花键类型", -38.15768061703267, -18.99778063953164, 3.274425, 0.707, "6文字层"),
];



/// 内花键 17 个属性 tag（**自上而下**的行序；模板 ATTDEF 的文档顺序内/外不一致，
/// 改按可视行序，`attdefs()` 用本表取位置）。
pub const TAGS_INT: &[&str] = &[
    "花键类型",
    "齿数",
    "径节",
    "压力角",
    "基圆直径",
    "节圆直径",
    "大径上差",
    "大径",
    "大径下差",
    "有效直径",
    "小径",
    "实际齿厚最大值",
    "作用齿厚最小值",
    "跨棒距上差",
    "跨棒距",
    "跨棒距下差",
    "量棒直径",
];

/// 外花键 17 个属性 tag（自上而下）。
pub const TAGS_EXT: &[&str] = &[
    "花键类型",
    "齿数",
    "径节",
    "压力角",
    "基圆直径",
    "节圆直径",
    "大径上差",
    "大径",
    "大径下差",
    "渐开线终止圆直径",
    "小径",
    "作用齿厚最大值",
    "实际齿厚最小值",
    "公法线上差",
    "公法线长度",
    "公法线下差",
    "跨测齿数",
];

/// 方向 → tag 清单。
pub fn tags(side: SplineSide) -> &'static [&'static str] {
    match side {
        SplineSide::Internal => TAGS_INT,
        SplineSide::External => TAGS_EXT,
    }
}

fn common_of(layer: &str) -> EntityCommon {
    let mut c = EntityCommon::default();
    c.layer = layer.to_string();
    c.color = Color::ByLayer;
    c.linetype = "ByLayer".into();
    c
}

fn mtext_ent(
    value: &str,
    x: f64,
    y: f64,
    h: f64,
    layer: &str,
    attach: i16,
    rect_w: f64,
) -> EntityType {
    let mut m = MText::new();
    m.value = value.to_string();
    m.insertion_point = Vector3::new(x, y, 0.0);
    m.height = h;
    m.rectangle_width = rect_w;
    m.style = "OCSM_GB".into();
    m.attachment_point = match attach {
        4 => AttachmentPoint::MiddleLeft,
        8 => AttachmentPoint::BottomCenter,
        _ => AttachmentPoint::BottomLeft,
    };
    m.common = common_of(layer);
    EntityType::MText(m)
}

fn attdef(tag: &str, x: f64, y: f64, h: f64, layer: &str) -> AttributeDefinition {
    let mut ad = AttributeDefinition::new(tag.to_string(), String::new(), " ".to_string());
    ad.insertion_point = Vector3::new(x, y, 0.0);
    ad.alignment_point = ad.insertion_point;
    ad.height = h;
    ad.width_factor = if tag == "花键类型" {
        TYPE_WIDTH_FACTOR
    } else {
        VALUE_WIDTH_FACTOR
    };
    ad.text_style = "OCSM_GB".into();
    ad.flags.preset = true;
    ad.common = common_of(layer);
    ad
}

/// 17 个 ATTDEF（行序 = `tags()`；位置/字高/层照模板）。
pub fn attdefs(side: SplineSide) -> Vec<AttributeDefinition> {
    let data = match side {
        SplineSide::Internal => ANSI_ATTDEFS_INT,
        SplineSide::External => ANSI_ATTDEFS_EXT,
    };
    tags(side)
        .iter()
        .map(|tag| {
            let (t, x, y, h, _wf, layer) = data
                .iter()
                .find(|(t, ..)| t == tag)
                .unwrap_or_else(|| panic!("ANSI 模板数据缺 tag {tag}"));
            attdef(t, *x, *y, *h, layer)
        })
        .collect()
}

/// 表格块成员：26 线 + 语种标签 MTEXT + 17 ATTDEF。
/// 卡面标签的**语种钉死**标记（`ANSI_LABELS_*` 第 2 字段）。
///
/// ★ 用户 2026-09-27 口径：**`OCSMLANG`（语言开关）与「块语种」是两件事** ——
/// 纯中文块取 catalog 的 `zh`、纯英文块取 `en`，**与当前语言无关**；
/// `c`/`e` = 该条只在中文块 / 只在英文块（英文把「跨棒距 / 公法线长度」换行成第二行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LabelPin {
    Both,
    Cn,
    En,
}

fn label_pin(c: char) -> LabelPin {
    match c {
        'c' => LabelPin::Cn,
        'e' => LabelPin::En,
        _ => LabelPin::Both,
    }
}

/// 块语种 → catalog 取词语言（**不再看 `i18n::lang()`**）。
fn block_lang(lang: AnsiLang) -> crate::i18n::Lang {
    match lang {
        AnsiLang::Cn => crate::i18n::Lang::Zh,
        AnsiLang::En => crate::i18n::Lang::En,
    }
}

/// 表格块成员：26 线 + 语种标签 MTEXT（**catalog 取词，按块语种钉死**）+ 17 ATTDEF。
pub fn block_entities(side: SplineSide, lang: AnsiLang) -> Vec<EntityType> {
    let labels = match side {
        SplineSide::Internal => ANSI_LABELS_INT,
        SplineSide::External => ANSI_LABELS_EXT,
    };
    let mut out = Vec::with_capacity(ANSI_LINES.len() + labels.len() + 17);
    for (a, b, layer) in ANSI_LINES {
        out.push(crate::partgen_kit::line(*a, *b, layer));
    }
    for (key, pin, x, y, h, _wf, layer, attach, rect_w) in labels {
        match (label_pin(*pin), lang) {
            (LabelPin::Both, _) => {}
            (LabelPin::Cn, AnsiLang::Cn) => {}
            (LabelPin::En, AnsiLang::En) => {}
            _ => continue, // 该块语种没有这条（如 跨棒距 的英文续行）
        }
        let v = crate::i18n::t_lang(block_lang(lang), key);
        out.push(mtext_ent(&v, *x, *y, *h, layer, *attach, *rect_w));
    }
    for ad in attdefs(side) {
        out.push(EntityType::AttributeDefinition(ad));
    }
    out
}

// ══════════════════════════════════════════════════════════════════════════
// 取值（与 `invol_spline.rs` 的 ANSI B92.1 实现同源）
// ══════════════════════════════════════════════════════════════════════════

/// 最多 3 位小数（显示与内部计算分开）。
pub fn fmt_mm(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn fmt_deg(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}°", v.round() as i64)
    } else {
        format!("{}°", crate::partgen_kit::trim(v))
    }
}

/// `P/Ps` 成对写法（17 项系列里取标准 label；系列外退化为 `P/2P`）。
pub fn pair_label(p: f64) -> String {
    crate::invol_spline::ansi_pitch_row(p)
        .map(|r| r.label.clone())
        .unwrap_or_else(|| {
            format!(
                "{}/{}",
                crate::partgen_kit::trim(p),
                crate::partgen_kit::trim(2.0 * p)
            )
        })
}

/// 齿形类型（中文/英文；ANSI B92.1 Table 2 的列名）。
///
/// ★ catalog 唯一来源（`card.ansi.type.*`）：**按块语种钉死**取词 —— 与 GUI 语言开关无关
/// （纯中文块永远取 zh 侧，纯英文块永远取 en 侧）。
fn type_text(col: Option<AnsiColumn>, lang: AnsiLang) -> String {
    let Some(c) = col else {
        return MISSING.to_string();
    };
    let key = match c {
        AnsiColumn::A30FlatSide => "card.ansi.type.a30flat_side",
        AnsiColumn::B30FlatMajor => "card.ansi.type.b30flat_major",
        AnsiColumn::C30FilletSide => "card.ansi.type.c30fillet_side",
        AnsiColumn::D375FilletSide => "card.ansi.type.d375fillet_side",
        AnsiColumn::E45FilletSide => "card.ansi.type.e45fillet_side",
    };
    crate::i18n::t_lang(
        match lang {
            AnsiLang::Cn => crate::i18n::Lang::Zh,
            AnsiLang::En => crate::i18n::Lang::En,
        },
        key,
    )
}

/// 「ANSI 花键参数表」卡参数（CLI/GUI 同一份字段）。
#[derive(Debug, Clone, PartialEq)]
pub struct AnsiTableSpec {
    pub side: SplineSide,
    pub lang: AnsiLang,
    /// 齿廓预设名（`ANSI30平齿根齿侧` …；缺省 = 列 A）。
    pub profile: &'static str,
    /// 径节 `P`（每英寸齿数；同时用于显示 `P/Ps`）。
    pub p: f64,
    /// 齿数 `N`。
    pub z: u32,
    pub at: Option<[f64; 2]>,
    pub rot: f64,
}

impl AnsiTableSpec {
    /// 引擎输入（`InvolParams`，与齿轮生成器花键模式同一套）。
    pub fn params(&self) -> Result<InvolParams, String> {
        let q = InvolParams::ansi(self.profile, self.p, self.z)?
            .with_internal(self.side == SplineSide::Internal);
        q.validate()?;
        Ok(q)
    }

    /// 解析 `内|外 P<径节> Z<齿数> [profile 齿廓] [at x,y] [rot 度]`。
    pub fn parse(lang: AnsiLang, text: &str) -> Result<Self, String> {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        if tokens.is_empty() {
            return Err(usage(lang));
        }
        let mut i = 0usize;
        let side = match tokens.get(i).copied() {
            Some("内") | Some("内部") | Some("内花键") | Some("int") | Some("internal") => {
                SplineSide::Internal
            }
            Some("外") | Some("外部") | Some("外花键") | Some("ext") | Some("external") => {
                SplineSide::External
            }
            _ => return Err(usage(lang)),
        };
        i += 1;
        // 表达式形态（用户 2026-09-26）：`内|外 <九字段表达式>`（MARK KIND M Z ALPHA X DA DF BETA H）。
        // 表达式从 `GEAR`/`SPLINE` 截到下一个本卡选项关键字；`P16 Z20` 旧写法保留兼容，
        // 两者都给时以表达式为准，不一致则报错。
        let mut expr: Option<String> = None;
        let tokens = if let Some((e, rest)) = crate::card_expr::split_expr(&tokens, is_option_token) {
            expr = Some(e);
            rest
        } else {
            tokens
        };
        let mut p: Option<f64> = None;
        let mut z: Option<u32> = None;
        let mut profile: Option<&'static str> = None;
        let mut at: Option<[f64; 2]> = None;
        let mut rot = 0.0f64;
        let mut need = |i: &mut usize, what: &str| -> Result<&str, String> {
            *i += 1;
            tokens
                .get(*i)
                .copied()
                .ok_or_else(|| {
                    crate::i18n::t_fmt("cmd.ansi.err.need_value", &[("what", what)])
                })
        };
        while i < tokens.len() {
            let t = tokens[i];
            let key = t.to_ascii_lowercase();
            if key == "profile" || key == "齿廓" || key == "preset" {
                let v = need(&mut i, "profile")?;
                profile = Some(resolve_profile(v)?);
            } else if key == "pitch" || key == "径节" {
                let v = need(&mut i, "径节")?;
                p = Some(crate::invol_spline::parse_ansi_pitch(v)?);
            } else if key == "齿数" {
                let v = need(&mut i, "齿数")?;
                z = Some(
                    v.parse()
                        .map_err(|e: std::num::ParseIntError| {
                            crate::i18n::t_fmt(
                                "cmd.ansi.err.z_num",
                                &[("v", &v), ("e", &e.to_string())],
                            )
                        })?,
                );
            } else if key == "at" {
                let v = need(&mut i, "at")?;
                let (x, y) = v
                    .split_once(',')
                    .ok_or_else(|| {
                        crate::i18n::t_fmt("cmd.ansi.err.at_format", &[("v", &v)])
                    })?;
                let x: f64 = x
                    .trim()
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.ansi.err.at_x",
                            &[("x", &x), ("e", &e.to_string())],
                        )
                    })?;
                let y: f64 = y
                    .trim()
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.ansi.err.at_y",
                            &[("y", &y), ("e", &e.to_string())],
                        )
                    })?;
                at = Some([x, y]);
            } else if key == "rot" || key == "旋转" {
                let v = need(&mut i, "rot")?;
                rot = v
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.ansi.err.rot",
                            &[("v", &v), ("e", &e.to_string())],
                        )
                    })?;
            } else if let Some(v) = strip_prefix_ci(t, "p") {
                let v = if v.is_empty() {
                    need(&mut i, "P")?
                } else {
                    v
                };
                p = Some(crate::invol_spline::parse_ansi_pitch(v)?);
            } else if let Some(v) = strip_prefix_ci(t, "z") {
                let v = if v.is_empty() { need(&mut i, "Z")? } else { v };
                z = Some(
                    v.parse()
                        .map_err(|e| format!("ANSI 花键参数表：Z={v} 不是整数：{e}"))?,
                );
            } else {
                // 裸齿廓名（`ANSI30R` / `30圆齿根齿侧` …）
                match crate::invol_spline::parse_preset_token(t) {
                    Some((SplineStd::ANSI, name)) => profile = Some(name),
                    _ => {
                        return Err(crate::i18n::t_fmt(
                            "cmd.ansi.err.unknown_param",
                            &[("t", t), ("usage", &usage(lang))],
                        ))
                    }
                }
            }
            i += 1;
        }
        // 表达式 > 旧 P/Z；显式值与表达式反解不一致 → 报错（帮排错）。
        let (p, z, profile) = match expr {
            Some(e) => {
                let (ep, ez, prof) = expr_inputs(&e, side, profile)?;
                if let Some(given) = p {
                    if (given - ep).abs() > 1e-9 {
                        return Err(crate::i18n::t_fmt(
                            "cmd.ansi.err.p_mismatch",
                            &[
                                ("p", &pair_label(given)),
                                ("ep", &pair_label(ep)),
                            ],
                        ));
                    }
                }
                if let Some(given) = z {
                    if given != ez {
                        return Err(crate::i18n::t_fmt(
                            "cmd.ansi.err.n_mismatch",
                            &[("given", &given.to_string()), ("ez", &ez.to_string())],
                        ));
                    }
                }
                (ep, ez, prof)
            }
            None => {
                let p = p.ok_or_else(|| {
                    crate::i18n::t_fmt(
                        "cmd.ansi.err.missing_p",
                        &[("usage", &usage(lang))],
                    )
                })?;
                let z = z.ok_or_else(|| {
                    crate::i18n::t_fmt(
                        "cmd.ansi.err.missing_z",
                        &[("usage", &usage(lang))],
                    )
                })?;
                (
                    p,
                    z,
                    profile.unwrap_or(crate::invol_spline::ANSI_DEFAULT_PROFILE),
                )
            }
        };
        let spec = AnsiTableSpec {
            side,
            lang,
            profile,
            p,
            z,
            at,
            rot,
        };
        spec.params()?; // 立即校验（系列/列适用径节/几何）
        Ok(spec)
    }
}

/// 表达式截取用：token 是不是 ANSI 卡的选项关键字（`P16`/`Z20`/`profile`/`at`/`rot`…）。
/// 只在表达式第 7 个 token 之后调用，所以不会误判 `M3`/`Z20`/`ALPHA30`/`X0`。
fn is_option_token(t: &str) -> bool {
    let l = t.to_ascii_lowercase();
    if t.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return true; // 裸数字/配合 token（稳健兜底）
    }
    if matches!(
        l.as_str(),
        "at" | "rot" | "旋转" | "profile" | "齿廓" | "preset" | "pitch" | "径节" | "齿数"
    ) {
        return true;
    }
    if l == "p" || l == "z" {
        return true;
    }
    if let Some(rest) = l.strip_prefix('p').or_else(|| l.strip_prefix('z')) {
        return rest.chars().next().is_some_and(|c| c.is_ascii_digit());
    }
    false
}

fn strip_prefix_ci<'a>(t: &'a str, p: &str) -> Option<&'a str> {
    if t.len() >= p.len() && t[..p.len()].eq_ignore_ascii_case(p) {
        Some(&t[p.len()..])
    } else {
        None
    }
}

/// ANSI 卡的表达式策略（表驱动；体系要求 SPLINE，α 只收 30/37.5/45，直齿，不收变位）。
pub const EXPR_POLICY: crate::card_expr::ExprPolicy = crate::card_expr::ExprPolicy {
    card: "ANSI 花键参数表",
    mark: crate::card_expr::ExprMark::Spline,
    alphas: &[30.0, 37.5, 45.0],
    spur: true,
    allow_shift: false,
    map: &[
        crate::card_expr::ExprRule {
            target: "p",
            label: "径节 P/Ps",
            op: crate::card_expr::ExprOp::Pitch,
        },
        crate::card_expr::ExprRule {
            target: "z",
            label: "齿数 N",
            op: crate::card_expr::ExprOp::Teeth,
        },
    ],
};

/// 齿廓预设 → α（Table 2 五列）。
fn profile_alpha(profile: &str) -> Option<f64> {
    crate::invol_spline::preset(SplineStd::ANSI, profile)
        .ok()
        .map(|p| p.alpha_deg)
}

/// α → 默认齿廓：优先列 A（30° 平齿根齿侧），否则 Table 2 里同 α 的第一列。
pub fn default_profile_for_alpha(alpha: f64) -> Result<&'static str, String> {
    use crate::invol_spline::{ANSI_DEFAULT_PROFILE, ANSI_PRESETS};
    if let Some(p) = ANSI_PRESETS
        .iter()
        .find(|p| (p.alpha_deg - alpha).abs() < 1e-9 && p.profile == ANSI_DEFAULT_PROFILE)
    {
        return Ok(p.profile);
    }
    ANSI_PRESETS
        .iter()
        .find(|p| (p.alpha_deg - alpha).abs() < 1e-9)
        .map(|p| p.profile)
        .ok_or_else(|| {
            crate::i18n::t_fmt(
                "cmd.ansi.err.no_profile",
                &[("alpha", &crate::partgen_kit::trim(alpha))],
            )
        })
}

/// 九字段表达式 → `(P, N, profile)`：方向必查（KIND 与卡片方向一致）；
/// 齿廓保持「当前选择里同 α 的那一列」，否则按 α 取默认列。
pub fn expr_inputs(
    expr: &str,
    side: SplineSide,
    current: Option<&'static str>,
) -> Result<(f64, u32, &'static str), String> {
    let r = crate::card_expr::resolve(&EXPR_POLICY, expr, Some(side == SplineSide::Internal))?;
    let p = r
        .value("p")
        .ok_or_else(|| crate::i18n::t("cmd.ansi.err.map_missing_p"))?;
    let z = r
        .value("z")
        .ok_or_else(|| crate::i18n::t("cmd.ansi.err.map_missing_n"))? as u32;
    let alpha = r.fields.alpha_deg();
    let profile = match current {
        Some(t) if profile_alpha(t).is_some_and(|a| (a - alpha).abs() < 1e-9) => t,
        _ => default_profile_for_alpha(alpha)?,
    };
    Ok((p, z, profile))
}

/// 齿廓 token → 预设全名（收五列代号 `ANSI30R` 与全名 `30圆齿根齿侧`）。
fn resolve_profile(t: &str) -> Result<&'static str, String> {
    if let Some((SplineStd::ANSI, name)) = crate::invol_spline::parse_preset_token(t) {
        return Ok(name);
    }
    crate::invol_spline::preset(SplineStd::ANSI, t).map(|p| p.profile)
}

/// 17 项取值（顺序 = `tags(side)`；未收数据 = `MISSING`）。
pub fn values(spec: &AnsiTableSpec) -> Result<Vec<(String, String)>, String> {
    let q = spec.params()?;
    let side = spec.side;
    let (major, minor, form) = match side {
        SplineSide::Internal => (
            q.internal_major_dia(),
            q.internal_minor_dia(),
            q.ansi_form_dia_internal(),
        ),
        SplineSide::External => (q.da(), q.df(), q.ansi_form_dia_external()),
    };
    let type_desc = type_text(q.ansi_column(), spec.lang);
    let out: Vec<(&str, String)> = tags(side)
        .iter()
        .map(|tag| {
            let v = match *tag {
                "花键类型" => type_desc.clone(),
                "齿数" => q.z.to_string(),
                "径节" => pair_label(q.ansi_p()),
                "压力角" => fmt_deg(q.alpha_deg),
                "基圆直径" => fmt_mm(q.db()),
                "节圆直径" => fmt_mm(q.d()),
                "大径" => fmt_mm(major),
                "有效直径" | "渐开线终止圆直径" => fmt_mm(form),
                "小径" => fmt_mm(minor),
                // ANSI B92.1 配合/公差/量棒/公法线表本仓未收 → 如实标缺。
                _ => MISSING.to_string(),
            };
            (*tag, v)
        })
        .collect();
    Ok(out
        .into_iter()
        .map(|(t, v)| (t.to_string(), v))
        .collect())
}

/// 建 INSERT（基点在 `at`，旋转 `rot_deg` 度；17 个 ATTRIB 取自 `values()`）。
pub fn build_insert(spec: &AnsiTableSpec, at: [f64; 2], rot_deg: f64) -> Result<Insert, String> {
    let values = values(spec)?;
    let block = block_name(spec.side, spec.lang);
    let mut ins = Insert::new(block, Vector3::new(at[0], at[1], 0.0));
    ins.rotation = rot_deg.to_radians();
    {
        let c = &mut ins.common;
        c.layer = crate::partgen::LAYER_MAIN.to_string();
        c.color = Color::ByLayer;
        c.linetype = "ByLayer".to_string();
        c.line_weight = LineWeight::ByLayer;
    }
    for ad in attdefs(spec.side) {
        let val = values
            .iter()
            .find(|(tag, _)| tag == &ad.tag)
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        let mut tmpl = ad.clone();
        tmpl.rotation = 0.0;
        let mut attr = AttributeEntity::from_definition(&tmpl, Some(val));
        attr.apply_transform(&ins.get_transform());
        ins.attributes.push(attr);
    }
    Ok(ins)
}

/// 17 项 Markdown（计算书/回执用）。
pub fn markdown_table(spec: &AnsiTableSpec) -> Result<String, String> {
    let vals = values(spec)?;
    let mut md = String::new();
    md.push_str("| 属性 | 值 |\n|---|---|\n");
    for (tag, v) in vals {
        md.push_str(&format!("| {tag} | {v} |\n"));
    }
    Ok(md)
}

fn usage(lang: AnsiLang) -> String {
    let card = match lang {
        AnsiLang::Cn => "ANSI花键参数表_中文",
        AnsiLang::En => "ANSI花键参数表_英文",
    };
    crate::i18n::t_fmt("cmd.ansi.usage", &[("card", card)])
}

// ══════════════════════════════════════════════════════════════════════════
// GUI 选项表（列口径）与表单模型（CLI/GUI/HTTP 同一份字段）
// ══════════════════════════════════════════════════════════════════════════

/// 一项属性的取值口径（进原生 `title=`，不做常显）。
///
/// ★ GUI 元数据（本批 ②）：文字全进 catalog —— 列名 = [`label_key`](Self::label_key)
/// （`gui.ansi.col.*`）+ [`symbol`](Self::symbol)（两语原样，`N`/`P/Ps`/`φD`/`Dri`…）；
/// 取值口径/来源 = [`formula_key`](Self::formula_key) / [`source_key`](Self::source_key)。
/// 取词走 [`AnsiColumnSpec::label`] / [`AnsiColumnSpec::formula`] / [`AnsiColumnSpec::source`]。
#[derive(Debug, Clone, Copy)]
pub struct AnsiColumnSpec {
    pub tag: &'static str,
    /// 列名 key（`gui.ansi.col.*`）。
    pub label_key: &'static str,
    /// 列名里的符号（两语原样；空串 = 无）。
    pub symbol: &'static str,
    pub unit: &'static str,
    /// 取值口径 key（`gui.ansi.formula.*`）。
    pub formula_key: &'static str,
    /// 来源 key（`gui.ansi.source.*`）。
    pub source_key: &'static str,
}

impl AnsiColumnSpec {
    /// 列名（按当前语言）：译名 + 「空格 + 符号」（符号两语原样）。
    pub fn label(&self) -> String {
        let text = crate::i18n::t(self.label_key);
        if self.symbol.is_empty() {
            text
        } else {
            format!("{text} {}", self.symbol)
        }
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

/// 内花键 17 项口径（顺序 = `TAGS_INT`）。
pub const ANSI_COLUMNS_INT: &[AnsiColumnSpec] = &[
    AnsiColumnSpec { tag: "花键类型", label_key: "gui.ansi.col.spline_type", symbol: "", unit: "", formula_key: "gui.ansi.formula.profile_column", source_key: "gui.ansi.source.table2" },
    AnsiColumnSpec { tag: "齿数", label_key: "gui.ansi.col.teeth", symbol: "N", unit: "", formula_key: "gui.ansi.formula.teeth_input", source_key: "gui.ansi.source.user" },
    AnsiColumnSpec { tag: "径节", label_key: "gui.ansi.col.pitch", symbol: "P/Ps", unit: "", formula_key: "gui.ansi.formula.pitch_pair", source_key: "gui.ansi.source.table3" },
    AnsiColumnSpec { tag: "压力角", label_key: "gui.ansi.col.pressure_angle", symbol: "φD", unit: "°", formula_key: "gui.ansi.formula.alpha_columns", source_key: "gui.ansi.source.table2" },
    AnsiColumnSpec { tag: "基圆直径", label_key: "gui.ansi.col.base_dia", symbol: "Db", unit: "mm", formula_key: "gui.ansi.formula.base_dia", source_key: "gui.ansi.source.table2" },
    AnsiColumnSpec { tag: "节圆直径", label_key: "gui.ansi.col.pitch_dia", symbol: "D", unit: "mm", formula_key: "gui.ansi.formula.pitch_dia", source_key: "gui.ansi.source.table2" },
    AnsiColumnSpec { tag: "大径上差", label_key: "gui.ansi.col.major_dia_up", symbol: "", unit: "", formula_key: "gui.ansi.formula.fit_missing", source_key: "gui.ansi.source.missing" },
    AnsiColumnSpec { tag: "大径", label_key: "gui.ansi.col.major_dia", symbol: "Dri", unit: "mm", formula_key: "gui.ansi.formula.major_dia_int", source_key: "gui.ansi.source.table2" },
    AnsiColumnSpec { tag: "大径下差", label_key: "gui.ansi.col.major_dia_down", symbol: "", unit: "", formula_key: "gui.ansi.formula.fit_missing", source_key: "gui.ansi.source.missing" },
    AnsiColumnSpec { tag: "有效直径", label_key: "gui.ansi.col.form_dia", symbol: "DFi", unit: "mm", formula_key: "gui.ansi.formula.form_dia_int", source_key: "gui.ansi.source.table2" },
    AnsiColumnSpec { tag: "小径", label_key: "gui.ansi.col.minor_dia", symbol: "Di", unit: "mm", formula_key: "gui.ansi.formula.minor_dia_int", source_key: "gui.ansi.source.table2" },
    AnsiColumnSpec { tag: "实际齿厚最大值", label_key: "gui.ansi.col.thick_max_actual", symbol: "", unit: "", formula_key: "gui.ansi.formula.thickness_missing", source_key: "gui.ansi.source.missing" },
    AnsiColumnSpec { tag: "作用齿厚最小值", label_key: "gui.ansi.col.thick_min_effective", symbol: "", unit: "", formula_key: "gui.ansi.formula.thickness_missing", source_key: "gui.ansi.source.missing" },
    AnsiColumnSpec { tag: "跨棒距上差", label_key: "gui.ansi.col.over_pins_up", symbol: "", unit: "", formula_key: "gui.ansi.formula.pin_inspection_missing", source_key: "gui.ansi.source.missing" },
    AnsiColumnSpec { tag: "跨棒距", label_key: "gui.ansi.col.over_pins", symbol: "", unit: "mm", formula_key: "gui.ansi.formula.over_pins_missing", source_key: "gui.ansi.source.missing" },
    AnsiColumnSpec { tag: "跨棒距下差", label_key: "gui.ansi.col.over_pins_down", symbol: "", unit: "", formula_key: "gui.ansi.formula.pin_inspection_missing", source_key: "gui.ansi.source.missing" },
    AnsiColumnSpec { tag: "量棒直径", label_key: "gui.ansi.col.pin_dia", symbol: "", unit: "mm", formula_key: "gui.ansi.formula.pin_dia_missing", source_key: "gui.ansi.source.missing" },
];

/// 外花键 17 项口径（顺序 = `TAGS_EXT`）。
pub const ANSI_COLUMNS_EXT: &[AnsiColumnSpec] = &[
    AnsiColumnSpec { tag: "花键类型", label_key: "gui.ansi.col.spline_type", symbol: "", unit: "", formula_key: "gui.ansi.formula.profile_column", source_key: "gui.ansi.source.table2" },
    AnsiColumnSpec { tag: "齿数", label_key: "gui.ansi.col.teeth", symbol: "N", unit: "", formula_key: "gui.ansi.formula.teeth_input", source_key: "gui.ansi.source.user" },
    AnsiColumnSpec { tag: "径节", label_key: "gui.ansi.col.pitch", symbol: "P/Ps", unit: "", formula_key: "gui.ansi.formula.pitch_pair", source_key: "gui.ansi.source.table3" },
    AnsiColumnSpec { tag: "压力角", label_key: "gui.ansi.col.pressure_angle", symbol: "φD", unit: "°", formula_key: "gui.ansi.formula.alpha_columns", source_key: "gui.ansi.source.table2" },
    AnsiColumnSpec { tag: "基圆直径", label_key: "gui.ansi.col.base_dia", symbol: "Db", unit: "mm", formula_key: "gui.ansi.formula.base_dia", source_key: "gui.ansi.source.table2" },
    AnsiColumnSpec { tag: "节圆直径", label_key: "gui.ansi.col.pitch_dia", symbol: "D", unit: "mm", formula_key: "gui.ansi.formula.pitch_dia", source_key: "gui.ansi.source.table2" },
    AnsiColumnSpec { tag: "大径上差", label_key: "gui.ansi.col.major_dia_up", symbol: "", unit: "", formula_key: "gui.ansi.formula.fit_missing", source_key: "gui.ansi.source.missing" },
    AnsiColumnSpec { tag: "大径", label_key: "gui.ansi.col.major_dia", symbol: "Do", unit: "mm", formula_key: "gui.ansi.formula.major_dia_ext", source_key: "gui.ansi.source.table2" },
    AnsiColumnSpec { tag: "大径下差", label_key: "gui.ansi.col.major_dia_down", symbol: "", unit: "", formula_key: "gui.ansi.formula.fit_missing", source_key: "gui.ansi.source.missing" },
    AnsiColumnSpec { tag: "渐开线终止圆直径", label_key: "gui.ansi.col.form_dia_ext", symbol: "DFe", unit: "mm", formula_key: "gui.ansi.formula.form_dia_ext", source_key: "gui.ansi.source.table2" },
    AnsiColumnSpec { tag: "小径", label_key: "gui.ansi.col.minor_dia", symbol: "Dre", unit: "mm", formula_key: "gui.ansi.formula.minor_dia_ext", source_key: "gui.ansi.source.table2" },
    AnsiColumnSpec { tag: "作用齿厚最大值", label_key: "gui.ansi.col.thick_max_effective", symbol: "", unit: "", formula_key: "gui.ansi.formula.thickness_missing", source_key: "gui.ansi.source.missing" },
    AnsiColumnSpec { tag: "实际齿厚最小值", label_key: "gui.ansi.col.thick_min_actual", symbol: "", unit: "", formula_key: "gui.ansi.formula.thickness_missing", source_key: "gui.ansi.source.missing" },
    AnsiColumnSpec { tag: "公法线上差", label_key: "gui.ansi.col.base_tangent_up", symbol: "", unit: "", formula_key: "gui.ansi.formula.base_tangent_missing", source_key: "gui.ansi.source.missing" },
    AnsiColumnSpec { tag: "公法线长度", label_key: "gui.ansi.col.base_tangent", symbol: "", unit: "mm", formula_key: "gui.ansi.formula.base_tangent_missing", source_key: "gui.ansi.source.missing" },
    AnsiColumnSpec { tag: "公法线下差", label_key: "gui.ansi.col.base_tangent_down", symbol: "", unit: "", formula_key: "gui.ansi.formula.base_tangent_missing", source_key: "gui.ansi.source.missing" },
    AnsiColumnSpec { tag: "跨测齿数", label_key: "gui.ansi.col.span_teeth", symbol: "", unit: "", formula_key: "gui.ansi.formula.base_tangent_missing", source_key: "gui.ansi.source.missing" },
];

/// 方向 → 列口径。
pub fn columns(side: SplineSide) -> &'static [AnsiColumnSpec] {
    match side {
        SplineSide::Internal => ANSI_COLUMNS_INT,
        SplineSide::External => ANSI_COLUMNS_EXT,
    }
}

/// ANSI 卡的表单字段（表驱动 GUI 骨架；中/英两卡共用同一清单，只有语种渲染不同）。
pub const FORM: crate::card::CardFormSpec = crate::card::CardFormSpec {
    fields: &[
        crate::card::CardFieldSpec {
            key: "expr",
            label: "gui.fld.ansi_table.form.expr.label",
            kind: "textarea",
            placeholder: "gui.fld.ansi_table.form.expr.placeholder",
            default: "",
            title: "gui.fld.ansi_table.form.expr.title",
            options: &[],
            options_from: "",
            min: 0.0,
            step: 0.0,
            required: false,
        },
        crate::card::CardFieldSpec {
            key: "profile",
            label: "gui.fld.ansi_table.form.profile.label",
            kind: "select",
            placeholder: "",
            default: "",
            title: "gui.fld.ansi_table.form.profile.title",
            options: &[],
            options_from: "ansi_profiles",
            min: 0.0,
            step: 0.0,
            required: false,
        },
        crate::card::CardFieldSpec {
            key: "p",
            label: "gui.fld.ansi_table.form.p.label",
            kind: "number",
            placeholder: "gui.fld.ansi_table.form.p.placeholder",
            default: "16",
            title: "gui.fld.ansi_table.form.p.title",
            options: &[],
            options_from: "",
            min: 2.5,
            step: 0.5,
            required: true,
        },
        crate::card::CardFieldSpec {
            key: "z",
            label: "gui.fld.ansi_table.form.z.label",
            kind: "number",
            placeholder: "gui.fld.ansi_table.form.z.placeholder",
            default: "20",
            title: "gui.fld.ansi_table.form.z.title",
            options: &[],
            options_from: "",
            min: 3.0,
            step: 1.0,
            required: true,
        },
    ],
    note_key: "gui.form.ansi.note",
    missing_note_key: "gui.form.ansi.missing_note",
};

/// ANSI 卡的选项/口径 JSON（随 `/api/spline_options` 下发；页面只渲染）。
pub fn options_json() -> serde_json::Value {
    let profiles: Vec<serde_json::Value> = crate::invol_spline::ANSI_PRESETS
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": p.profile,
                "code": crate::invol_spline::preset_code(SplineStd::ANSI, p.profile),
                "alpha": p.alpha_deg,
            })
        })
        .collect();
    let cols = |side: SplineSide| -> Vec<serde_json::Value> {
        columns(side)
            .iter()
            .map(|c| {
                serde_json::json!({
                    "tag": c.tag,
                    "label": c.label(),
                    "unit": c.unit,
                    "formula": c.formula(),
                    "source": c.source(),
                })
            })
            .collect()
    };
    serde_json::json!({
        "profiles": profiles,
        "columns": {
            "int": cols(SplineSide::Internal),
            "ext": cols(SplineSide::External),
        },
        "langs": [
            {"id": "cn", "label": crate::i18n::t("card.ansi.lang.cn")},
            {"id": "en", "label": crate::i18n::t("card.ansi.lang.en")},
        ],
        "missing_note": crate::i18n::t("card.ansi.missing_note"),
        "note": crate::i18n::t("card.ansi.note"),
    })
}

/// CLI/HTTP 表单模型（GUI 与命令字段一一对应）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AnsiTableModel {
    /// 卡类型（GUI 回传；后端按 renderer 分派）。
    #[serde(default)]
    pub card: String,
    /// 语种（`cn`/`en`；由卡类型决定，GUI 不必填）。
    #[serde(default)]
    pub lang: Option<String>,
    /// 九字段统一齿形表达式（可空；给了则覆盖 P/Z，齿廓按 α 保持/选默认列）。
    #[serde(default)]
    pub expr: Option<String>,
    /// 方向：`int`/`ext`（由卡类型固定并于后端回填；旧请求显式给也收）。
    #[serde(default)]
    pub side: String,
    /// 径节 P。
    pub p: f64,
    /// 齿数 N。
    pub z: u32,
    /// 齿廓预设（缺省 = 列 A `ANSI30平齿根齿侧`）。
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub at: Option<[f64; 2]>,
    #[serde(default)]
    pub rot: f64,
}

impl AnsiTableModel {
    pub fn side(&self) -> Result<SplineSide, String> {
        match self.side.trim().to_ascii_lowercase().as_str() {
            "int" | "internal" | "内" | "内花键" => Ok(SplineSide::Internal),
            "ext" | "external" | "外" | "外花键" => Ok(SplineSide::External),
            other => Err(crate::i18n::t_fmt(
                "cmd.ansi.err.side_invalid",
                &[("other", other)],
            )),
        }
    }

    pub fn lang(&self) -> AnsiLang {
        self.lang
            .as_deref()
            .and_then(AnsiLang::from_id)
            .unwrap_or(AnsiLang::Cn)
    }

    /// →（校验过的 `AnsiTableSpec`）。
    pub fn spec(&self) -> Result<AnsiTableSpec, String> {
        let side = self.side()?;
        let lang = self.lang();
        let wanted = match self.profile.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(t) => Some(resolve_profile(t)?),
        };
        let (p, z, profile) = match self.expr.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(e) => expr_inputs(e, side, wanted)?,
            None => (
                self.p,
                self.z,
                wanted.unwrap_or(crate::invol_spline::ANSI_DEFAULT_PROFILE),
            ),
        };
        let spec = AnsiTableSpec {
            side,
            lang,
            profile,
            p,
            z,
            at: self.at,
            rot: self.rot,
        };
        spec.params()?;
        Ok(spec)
    }

    /// 预览 JSON（不碰图纸）。
    pub fn preview_json(&self) -> Result<serde_json::Value, String> {
        let spec = self.spec()?;
        let q = spec.params()?;
        let vals = values(&spec)?;
        let mut items = Vec::with_capacity(vals.len());
        let mut missing = Vec::new();
        for (c, (tag, value)) in columns(spec.side).iter().zip(vals) {
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
        let side_label = match spec.side {
            SplineSide::Internal => "内花键",
            SplineSide::External => "外花键",
        };
        let readout = serde_json::json!([
            {"k": crate::i18n::t("gui.ansi.readout.pitch"), "v": pair_label(q.ansi_p())},
            {"k": crate::i18n::t("gui.ansi.readout.teeth"), "v": q.z.to_string()},
            {"k": crate::i18n::t("gui.ansi.readout.alpha"), "v": fmt_deg(q.alpha_deg)},
            {"k": crate::i18n::t("gui.ansi.readout.pitch_dia"), "v": fmt_mm(q.d())},
            {"k": crate::i18n::t("gui.ansi.readout.base_dia"), "v": fmt_mm(q.db())},
        ]);
        // 表达式反解回填：只带本卡字段（p/z/profile）；无表达式 = null（页面不动控件）。
        let expr_echo = self
            .expr
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let fields = if expr_echo.is_some() {
            serde_json::json!({"p": spec.p, "z": spec.z, "profile": spec.profile})
        } else {
            serde_json::Value::Null
        };
        Ok(serde_json::json!({
            "ok": true,
            "card": match spec.lang {
                AnsiLang::Cn => "ANSI花键参数表_中文",
                AnsiLang::En => "ANSI花键参数表_英文",
            },
            "renderer": match spec.lang {
                AnsiLang::Cn => "ansi_table_cn",
                AnsiLang::En => "ansi_table_en",
            },
            "title": format!("ANSI B92.1 {side_label}参数表（{}，{side_label}）", spec.lang.label()),
            "expr": expr_echo,
            "fields": fields,
            "side": match spec.side { SplineSide::Internal => "int", SplineSide::External => "ext" },
            "profile": spec.profile,
            "readout": readout,
            "items": items,
            "missing": missing,
            "missing_note": crate::i18n::t("gui.ansi.preview.missing_note"),
        }))
    }

    /// 待放置件要带的 17 个 ATTRIB。
    pub fn pending_attrs(&self) -> Result<Vec<(AttributeDefinition, String)>, String> {
        let spec = self.spec()?;
        let vals = values(&spec)?;
        let mut out = Vec::with_capacity(vals.len());
        for ad in attdefs(spec.side) {
            let v = vals
                .iter()
                .find(|(tag, _)| tag == &ad.tag)
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            out.push((ad, v));
        }
        Ok(out)
    }

    /// 直接落点用的 `INSERT`。
    pub fn build_insert(&self) -> Result<Insert, String> {
        let spec = self.spec()?;
        let at = self.at.unwrap_or([0.0, 0.0]);
        build_insert(&spec, at, self.rot)
    }

    /// 插入回执里的一段。
    pub fn echo_note(&self) -> Result<String, String> {
        let q = self.spec()?.params()?;
        Ok(crate::i18n::t_fmt(
            "cmd.ansi.echo",
            &[
                (
                    "side",
                    &crate::i18n::t(match self.side()? {
                        SplineSide::Internal => "cmd.ansi.side.internal",
                        SplineSide::External => "cmd.ansi.side.external",
                    }),
                ),
                ("pair", &pair_label(q.ansi_p())),
                ("n", &q.z.to_string()),
                ("alpha", &crate::partgen_kit::trim(q.alpha_deg)),
            ],
        ))
    }

    /// 待放置件的 `OCSM_PART` 元数据（薄台账）。
    pub fn part_meta_json(&self) -> Result<String, String> {
        let spec = self.spec()?;
        Ok(serde_json::json!({
            "family": "ansi_table",
            "card": match spec.lang {
                AnsiLang::Cn => "ANSI花键参数表_中文",
                AnsiLang::En => "ANSI花键参数表_英文",
            },
            "side": match spec.side { SplineSide::Internal => "int", SplineSide::External => "ext" },
            "lang": spec.lang.id(),
            "profile": spec.profile,
            "p": spec.p,
            "z": spec.z,
        })
        .to_string())
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 测试
// ══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spline_tol::SplineSide::{External, Internal};

    /// ⑥ 批：ANSI 齿形类型显示值 catalog 化后，**块语种钉死**（与 GUI 语言开关无关）。
    #[test]
    fn ansi_type_text_is_pinned_by_block_language() {
        use crate::i18n::{set_lang, set_lang_auto, Lang};
        let _g = crate::global_state_test_lock();
        let cols = [
            (AnsiColumn::A30FlatSide, "30°平齿根齿侧配合", "FLAT ROOT SIDE FIT"),
            (AnsiColumn::B30FlatMajor, "30°平齿根外径配合", "FLAT ROOT MAJOR DIA FIT"),
            (AnsiColumn::C30FilletSide, "30°圆齿根齿侧配合", "FILLET ROOT SIDE FIT"),
            (AnsiColumn::D375FilletSide, "37.5°圆齿根齿侧配合", "FILLET ROOT SIDE FIT"),
            (AnsiColumn::E45FilletSide, "45°圆齿根齿侧配合", "FILLET ROOT SIDE FIT"),
        ];
        for gui_lang in [Lang::Zh, Lang::En] {
            set_lang(gui_lang);
            for (c, cn, en) in cols {
                assert_eq!(type_text(Some(c), AnsiLang::Cn), cn, "GUI 语言开关不得影响块语种");
                assert_eq!(type_text(Some(c), AnsiLang::En), en, "GUI 语言开关不得影响块语种");
            }
            assert_eq!(type_text(None, AnsiLang::Cn), MISSING);
        }
        set_lang_auto();
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    /// 逐图元对模板：26 线（边框/横线 6.89715 行距）+ 19 标签 + 17 ATTDEF；样式 OCSM_GB。
    #[test]
    fn ansi_template_geometry_line_by_line() {
        const W: f64 = 79.92973028775381;
        const H: f64 = 124.1486999999999;
        const ROW: f64 = 6.89715;
        for side in [Internal, External] {
            for lang in [AnsiLang::Cn, AnsiLang::En] {
                let ents = block_entities(side, lang);
                assert_eq!(ents.len(), 26 + 19 + 17, "{side:?}/{lang:?} 图元数");
                let near5 = |a: f64, b: f64| (a - b).abs() < 1e-5;
                // 四条外框（1轮廓实线层）
                let frame = |a: [f64; 2], b: [f64; 2]| {
                    ents.iter().any(|e| matches!(e, EntityType::Line(l)
                        if l.common.layer == "1轮廓实线层"
                            && ((near5(l.start.x, a[0]) && near5(l.start.y, a[1])
                                && near5(l.end.x, b[0]) && near5(l.end.y, b[1]))
                                || (near5(l.start.x, b[0]) && near5(l.start.y, b[1])
                                    && near5(l.end.x, a[0]) && near5(l.end.y, a[1])))))
                };
                assert!(frame([-W, 0.0], [0.0, 0.0]), "{side:?} 上框");
                assert!(frame([0.0, 0.0], [0.0, -H]), "{side:?} 右框");
                assert!(frame([-W, -H], [0.0, -H]), "{side:?} 下框");
                assert!(frame([-W, 0.0], [-W, -H]), "{side:?} 左框");
                // 14 条整宽横线 y=-6.89715k（k=1..18），再加下框（k=18）
                for k in 1..=18 {
                    let y = -ROW * k as f64;
                    assert!(
                        ents.iter().any(|e| matches!(e, EntityType::Line(l)
                            if near5(l.start.y, y) && near5(l.end.y, y))),
                        "{side:?} 横线 y={y}"
                    );
                }
                // 竖分栏
                assert!(ents.iter().any(|e| matches!(e, EntityType::Line(l)
                    if near5(l.start.x, -39.32678800844012) && near5(l.start.y, -13.7943))));
                assert!(ents.iter().any(|e| matches!(e, EntityType::Line(l)
                    if near5(l.start.x, -21.79359502134392) && near5(l.start.y, -41.3829))));
                // 标签：全部 6文字层 + OCSM_GB；ATTDEF 同理
                let (mut mtexts, mut atts) = (0usize, 0usize);
                for e in &ents {
                    match e {
                        EntityType::MText(m) => {
                            assert_eq!(m.common.layer, "6文字层");
                            assert_eq!(m.style, "OCSM_GB", "MTEXT 样式：{}", m.value);
                            mtexts += 1;
                        }
                        EntityType::AttributeDefinition(ad) => {
                            assert_eq!(ad.common.layer, "6文字层");
                            assert_eq!(ad.text_style, "OCSM_GB", "ATTDEF 样式：{}", ad.tag);
                            atts += 1;
                        }
                        _ => {}
                    }
                }
                assert_eq!((mtexts, atts), (19, 17), "{side:?}/{lang:?}");
            }
        }
    }

    /// ★ 拆分：纯中文不含英文单词；纯英文不含汉字（标准号 `ANSI B92.1-1996` 除外）。
    ///
    /// ② 卡面批：文案已入 catalog（`card.ansi.label.*`，**同 key 两语**：zh = 纯中文块原文、
    /// en = 纯英文块原文）—— 本用例改成查 catalog + 块实体两侧纯度。
    #[test]
    fn ansi_split_pure_languages() {
        let cjk = |s: &str| s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
        let latin_word = |s: &str| {
            s.split(|c: char| !c.is_ascii_alphabetic())
                .any(|w| w.len() >= 2 && !matches!(w, "ANSI" | "B"))
        };
        let mut both = 0usize;
        for labels in [ANSI_LABELS_INT, ANSI_LABELS_EXT] {
            for (key, pin, ..) in labels {
                let zh = crate::i18n::t_lang(crate::i18n::Lang::Zh, key);
                let en = crate::i18n::t_lang(crate::i18n::Lang::En, key);
                assert!(
                    !zh.is_empty() && !en.is_empty(),
                    "{key} 两语都必须非空（同 key 装中/英两份原稿）"
                );
                assert!(!latin_word(&zh), "纯中文标签含英文：{key}={zh}");
                assert!(!cjk(&en), "纯英文标签含汉字：{key}={en}");
                if *pin == 'b' {
                    both += 1;
                }
            }
        }
        assert!(both > 0, "应有两块共用的标签（pin=b）");
        // 逐块实体纯度（块语种钉死）
        for side in [Internal, External] {
            for (lang, want_cn) in [(AnsiLang::Cn, true), (AnsiLang::En, false)] {
                let texts: Vec<String> = block_entities(side, lang)
                    .iter()
                    .filter_map(|e| match e {
                        EntityType::MText(m) => Some(m.value.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(texts.len(), 19, "{side:?}/{lang:?} 标签条数");
                for t in &texts {
                    if want_cn {
                        assert!(!latin_word(t), "{side:?}/纯中文块含英文：{t}");
                    } else {
                        assert!(!cjk(t), "{side:?}/纯英文块含汉字：{t}");
                    }
                }
                let title = texts
                    .iter()
                    .find(|t| t.contains("参数表") || t.contains("INVOLUTE"))
                    .unwrap();
                if want_cn {
                    assert!(!latin_word(title), "纯中文块标题：{title}");
                } else {
                    assert!(
                        !cjk(title) && !title.contains("INTERANL") && !title.contains("INOLUTE"),
                        "纯英文块标题：{title}"
                    );
                }
            }
        }
    }

    /// ② 卡面批：**块语种**决定卡面文案/几何（catalog 化前后逐字节相同，golden 表钉死）；
    /// ★ 与 `OCSMLANG`（语言开关）**无关** —— 同一块语种在两语环境下出的文字逐条相等。
    #[test]
    fn ansi_block_faces_are_language_pinned_and_switch_independent() {
        use crate::i18n::{set_lang, set_lang_auto, Lang};
        let _g = crate::global_state_test_lock();
        let cjk = |s: &str| s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));

        // 改 catalog 前的卡面 golden（文案 + 几何；用户 2026-09-27 验收基线）
        const INT_CN: &[(&str, f64, f64, f64, f64)] = &[
            ("花键类型", -78.66410528775381, -19.42582499999996, 3.274425, 38.0716922793137),
            ("齿数", -78.66410528775381, -26.32297499999996, 3.274425, 38.0716922793137),
            ("基圆直径", -78.66410528775381, -47.01442499999994, 3.274425, 38.0716922793137),
            ("参考", -10.89679751067196, -47.01442499999994, 3.274425, 19.2623450213439),
            ("节圆直径", -78.66410528775381, -53.91157499999994, 3.274425, 38.0716922793137),
            ("参考", -10.89679751067196, -53.91157499999994, 3.274425, 19.2623450213439),
            ("大径", -78.66410528775381, -62.07434999999993, 3.274425, 38.0716922793137),
            ("实际齿厚最大值", -78.66410528775381, -95.29447499999989, 3.274425, 55.6048852664099),
            ("跨棒距", -78.66410528775381, -109.0887749999999, 3.274425, 38.0716922793137),
            ("量棒直径", -78.66410528775381, -122.8830749999999, 3.274425, 55.6048852664099),
            ("作用齿厚最小值", -78.66410528775381, -102.1916249999999, 3.274425, 55.6048852664099),
            ("圆周齿厚", -78.66410528775381, -88.39732499999991, 3.274425, 77.3984802877538),
            ("最小", -10.89679751067196, -81.50017499999991, 3.274425, 19.2623450213439),
            ("小径", -78.66410528775381, -81.50017499999991, 3.274425, 38.0716922793137),
            ("有效直径", -78.66410528775381, -74.60302499999992, 3.274425, 38.0716922793137),
            ("压力角", -78.66410528775381, -40.11727499999994, 3.274425, 38.0716922793137),
            ("径节", -78.66410528775381, -33.22012499999996, 3.274425, 38.0716922793137),
            ("标准：ANSI B92.1-1996", -39.96486514387692, -12.52867499999997, 3.274425, 77.3984802877538),
            ("内花键参数表", -39.96486514387692, -5.631524999999979, 3.274425, 77.3984802877538),
        ];
        const INT_EN: &[(&str, f64, f64, f64, f64)] = &[
            ("SPLINE TYPE", -78.66410528775381, -19.42582499999996, 3.274425, 38.0716922793137),
            ("NUMBER OF TEETH", -78.66410528775381, -26.32297499999996, 3.274425, 38.0716922793137),
            ("BASE DIAMETER", -78.66410528775381, -47.01442499999994, 3.274425, 38.0716922793137),
            ("REF", -10.89679751067196, -47.01442499999994, 3.274425, 19.2623450213439),
            ("PITCH DIAMETER", -78.66410528775381, -53.91157499999994, 3.274425, 38.0716922793137),
            ("REF", -10.89679751067196, -53.91157499999994, 3.274425, 19.2623450213439),
            ("MAJOR DIAMETER", -78.66410528775381, -62.07434999999993, 3.274425, 38.0716922793137),
            ("MAX ACTUAL", -78.66410528775381, -95.29447499999989, 3.274425, 55.6048852664099),
            ("MEASUREMENT BETWEEN PINS", -78.6641052877538, -115.9859249999999, 3.1, 43.69900654265613),
            ("PIN DIAMETER", -78.66410528775381, -122.8830749999999, 3.274425, 55.6048852664099),
            ("MIN EFFECTIVE", -78.66410528775381, -102.1916249999999, 3.274425, 55.6048852664099),
            ("CIRCULAR TOOTH THICKNESS", -78.66410528775381, -88.39732499999991, 3.274425, 77.3984802877538),
            ("MIN", -10.89679751067196, -81.50017499999991, 3.274425, 19.2623450213439),
            ("MINOR DIAMETER", -78.66410528775381, -81.50017499999991, 3.274425, 38.0716922793137),
            ("FORM DIAMETER", -78.66410528775381, -74.60302499999992, 3.274425, 38.0716922793137),
            ("PRESSURE ANGLE", -78.66410528775381, -40.11727499999994, 3.274425, 38.0716922793137),
            ("SPLINE PITCH", -78.66410528775381, -33.22012499999996, 3.274425, 38.0716922793137),
            ("STANDARD: ANSI B92.1-1996", -39.96486514387692, -12.52867499999997, 3.274425, 77.3984802877538),
            ("INTERNAL INVOLUTE SPLINE DATA", -39.96486514387692, -5.631524999999979, 3.274425, 77.3984802877538),
        ];
        const EXT_CN: &[(&str, f64, f64, f64, f64)] = &[
            ("外花键参数表", -39.96486514387692, -5.63152500000001, 3.274425, 77.3984802877538),
            ("标准：ANSI B92.1-1996", -39.96486514387692, -12.52867499999996, 3.274425, 77.3984802877538),
            ("径节", -78.66410528775378, -33.22012499999997, 3.274425, 38.0716922793137),
            ("压力角", -78.66410528775378, -40.11727499999994, 3.274425, 38.0716922793137),
            ("渐开线终止圆直径", -78.66410528775378, -74.60302499999995, 3.274425, 38.0716922793137),
            ("小径", -78.66410528775378, -81.50017499999993, 3.274425, 38.0716922793137),
            ("最小", -10.89679751067194, -81.50017499999993, 3.274425, 19.2623450213439),
            ("圆周齿厚", -78.66410528775378, -88.39732499999991, 3.274425, 77.3984802877538),
            ("实际齿厚最小值", -78.66410528775378, -102.1916249999999, 3.274425, 55.6048852664099),
            ("花键类型", -78.66410528775378, -19.42582499999997, 3.274425, 38.0716922793137),
            ("齿数", -78.66410528775378, -26.32297499999999, 3.274425, 38.0716922793137),
            ("基圆直径", -78.66410528775378, -47.01442499999996, 3.274425, 38.0716922793137),
            ("参考", -10.89679751067194, -47.01442499999996, 3.274425, 19.2623450213439),
            ("节圆直径", -78.66410528775378, -53.91157499999994, 3.274425, 38.0716922793137),
            ("参考", -10.89679751067194, -53.91157499999994, 3.274425, 19.2623450213439),
            ("大径", -78.66410528775378, -62.07434999999995, 3.274425, 38.0716922793137),
            ("作用齿厚最大值", -78.66410528775378, -95.29447499999992, 3.274425, 55.6048852664099),
            ("公法线长度", -78.66410528775378, -109.0887749999999, 3.274425, 38.0716922793137),
            ("跨测齿数", -78.66410528775378, -122.8830749999999, 3.274425, 55.6048852664099),
        ];
        const EXT_EN: &[(&str, f64, f64, f64, f64)] = &[
            ("EXTERNAL INVOLUTE SPLINE DATA", -39.96486514387692, -5.63152500000001, 3.274425, 77.3984802877538),
            ("STANDARD: ANSI B92.1-1996", -39.96486514387692, -12.52867499999996, 3.274425, 77.3984802877538),
            ("SPLINE PITCH", -78.66410528775378, -33.22012499999997, 3.274425, 38.0716922793137),
            ("PRESSURE ANGLE", -78.66410528775378, -40.11727499999994, 3.274425, 38.0716922793137),
            ("FORM DIAMETER", -78.66410528775378, -74.60302499999995, 3.274425, 38.0716922793137),
            ("MINOR DIAMETER", -78.66410528775378, -81.50017499999993, 3.274425, 38.0716922793137),
            ("MIN", -10.89679751067194, -81.50017499999993, 3.274425, 19.2623450213439),
            ("CIRCULAR TOOTH THICKNESS", -78.66410528775378, -88.39732499999991, 3.274425, 77.3984802877538),
            ("MIN ACTUAL", -78.66410528775378, -102.1916249999999, 3.274425, 55.6048852664099),
            ("SPLINE TYPE", -78.66410528775378, -19.42582499999997, 3.274425, 38.0716922793137),
            ("NUMBER OF TEETH", -78.66410528775378, -26.32297499999999, 3.274425, 38.0716922793137),
            ("BASE DIAMETER", -78.66410528775378, -47.01442499999996, 3.274425, 38.0716922793137),
            ("REF", -10.89679751067194, -47.01442499999996, 3.274425, 19.2623450213439),
            ("PITCH DIAMETER", -78.66410528775378, -53.91157499999994, 3.274425, 38.0716922793137),
            ("REF", -10.89679751067194, -53.91157499999994, 3.274425, 19.2623450213439),
            ("MAJOR DIAMETER", -78.66410528775378, -62.07434999999995, 3.274425, 38.0716922793137),
            ("MAX EFFECTIVE", -78.66410528775378, -95.29447499999992, 3.274425, 55.6048852664099),
            ("COMMON NORMAL LINE", -78.66410528775378, -115.9859249999999, 3.274425, 38.0716922793137),
            ("NUMBER OF TESTING TEETH", -78.66410528775378, -122.8830749999999, 3.274425, 55.6048852664099),
        ];
        let faces = |side: SplineSide, lang: AnsiLang| -> Vec<(String, f64, f64, f64, f64)> {
            block_entities(side, lang)
                .iter()
                .filter_map(|e| match e {
                    EntityType::MText(m) => Some((
                        m.value.clone(),
                        m.insertion_point.x,
                        m.insertion_point.y,
                        m.height,
                        m.rectangle_width,
                    )),
                    _ => None,
                })
                .collect()
        };
        let tags = |side: SplineSide, _lang: AnsiLang| -> Vec<String> {
            attdefs(side).iter().map(|a| a.tag.clone()).collect()
        };
        // 1) golden：4 块卡面文字 + 位置/字高/折行框逐条与改 catalog 前一致
        for (side, lang, want) in [
            (Internal, AnsiLang::Cn, INT_CN),
            (Internal, AnsiLang::En, INT_EN),
            (External, AnsiLang::Cn, EXT_CN),
            (External, AnsiLang::En, EXT_EN),
        ] {
            let got = faces(side, lang);
            assert_eq!(got.len(), want.len(), "{side:?}/{lang:?} 条数");
            for (g, w) in got.iter().zip(want.iter()) {
                assert_eq!(g.0, w.0, "{side:?}/{lang:?} 文案");
                assert_eq!(g.1, w.1, "{side:?}/{lang:?}「{}」x", w.0);
                assert_eq!(g.2, w.2, "{side:?}/{lang:?}「{}」y", w.0);
                assert_eq!(g.3, w.3, "{side:?}/{lang:?}「{}」字高", w.0);
                assert_eq!(g.4, w.4, "{side:?}/{lang:?}「{}」折行框", w.0);
            }
        }
        // 2) ★ 语言开关与块语种是两件事：块内容逐条不随 OCSMLANG 变
        for side in [Internal, External] {
            for blk in [AnsiLang::Cn, AnsiLang::En] {
                set_lang(Lang::Zh);
                let a = faces(side, blk);
                let ta = tags(side, blk);
                set_lang(Lang::En);
                let b = faces(side, blk);
                assert_eq!(a, b, "{side:?}/{blk:?} 卡面随 OCSMLANG 变了（应按块语种钉死）");
                assert_eq!(ta, tags(side, blk), "{side:?}/{blk:?} tag 随 OCSMLANG 变了");
            }
        }
        // 3) ATTDEF tag：中/英块同一套（tag 不译，值按 tag 定位）
        for side in [Internal, External] {
            assert_eq!(
                tags(side, AnsiLang::Cn),
                tags(side, AnsiLang::En),
                "{side:?} 中/英块 tag 应同一套"
            );
            assert!(
                tags(side, AnsiLang::Cn).iter().all(|t| cjk(t)),
                "{side:?} tag 应保持中文原样（不译）"
            );
        }
        assert!(crate::i18n::missing_keys().is_empty(), "{:?}", crate::i18n::missing_keys());
        set_lang_auto();
    }

    /// ② GUI 元数据批：ANSI 卡语种名 + 17 项列名/口径/来源 + `options_json` 的 langs/notes
    /// **随语言且非空**；英文侧**零汉字**；列名里的符号（`N`/`P/Ps`/`φD`/`Dri`…）两语原样。
    #[test]
    fn ansi_gui_metadata_switches_language_and_has_no_cjk() {
        use crate::i18n::{set_lang, set_lang_auto, Lang};
        let _g = crate::global_state_test_lock();
        let cjk = |s: &str| s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
        let snap = |side: SplineSide| -> Vec<(String, String, String, String)> {
            columns(side)
                .iter()
                .map(|c| (c.tag.to_string(), c.label(), c.formula(), c.source()))
                .collect()
        };
        set_lang(Lang::Zh);
        let zh_int = snap(SplineSide::Internal);
        let zh_ext = snap(SplineSide::External);
        let zh_lang = AnsiLang::Cn.label();
        let zh_en_lang = AnsiLang::En.label();
        let zh_opts = options_json();
        set_lang(Lang::En);
        let en_int = snap(SplineSide::Internal);
        let en_ext = snap(SplineSide::External);
        let en_lang = AnsiLang::Cn.label();
        let en_en_lang = AnsiLang::En.label();
        let en_opts = options_json();
        assert_eq!(zh_lang, "纯中文");
        assert_eq!(zh_en_lang, "纯英文");
        assert_eq!(en_lang, "Chinese only");
        assert_eq!(en_en_lang, "English only");
        for (side, zh, en) in [
            ("int", &zh_int, &en_int),
            ("ext", &zh_ext, &en_ext),
        ] {
            assert_eq!(zh.len(), 17, "{side} 17 项");
            for (i, ((zt, zl, zf, zs), (et, el, ef, es))) in zh.iter().zip(en.iter()).enumerate() {
                assert_eq!(zt, et, "{side} 第 {} 项 tag 不译", i + 1);
                for (what, z, e) in [("label", zl, el), ("formula", zf, ef), ("source", zs, es)] {
                    assert!(!z.is_empty() && !e.is_empty(), "{side} 第 {} 项 {what} 空", i + 1);
                    assert!(!cjk(e), "{side} 第 {} 项 {what} 英文含汉字：{e:?}", i + 1);
                    assert_ne!(z, e, "{side} 第 {} 项 {what} 未随语言：{z:?}", i + 1);
                    // 列名必为中文术语（口径/来源可能是纯公式/标准号，如 `Di = (N−1)/P…`）
                    if what == "label" {
                        assert!(cjk(z), "{side} 第 {} 项列名中文应含汉字：{z:?}", i + 1);
                    }
                }
                // 列名 = 译名 +「空格 + 符号」；符号两语原样，且中文列名里的符号不丢
                let spec = if side == "int" {
                    SplineSide::Internal
                } else {
                    SplineSide::External
                };
                let sym = columns(spec)[i].symbol;
                if !sym.is_empty() {
                    assert!(
                        zl.ends_with(sym) && el.ends_with(sym),
                        "{side} 第 {} 项丢符号 {sym}",
                        i + 1
                    );
                    assert_ne!(zl, el, "{side} 第 {} 项列名未随语言", i + 1);
                }
            }
        }
        // options_json：langs / missing_note / note 同源随语言
        assert_eq!(zh_opts["langs"][0]["label"], "纯中文");
        assert_eq!(en_opts["langs"][0]["label"], "Chinese only");
        for f in ["missing_note", "note"] {
            let z = zh_opts[f].as_str().unwrap_or_default();
            let e = en_opts[f].as_str().unwrap_or_default();
            assert!(!z.is_empty() && !e.is_empty(), "options_json.{f} 空");
            assert!(!cjk(e), "options_json.{f} 英文含汉字：{e:?}");
            assert!(cjk(z), "options_json.{f} 中文应含汉字：{z:?}");
        }
        // 列名/口径/来源的 zh 值不得被接口改制搬丢
        assert_eq!(zh_int[0].2, "Table 2 列（30°平/圆齿根 × 齿侧/外径配合；由 profile 预设决定）");
        assert_eq!(zh_int[5].1, "节圆直径 D");
        assert_eq!(zh_ext[9].1, "渐开线终止圆直径 DFe");
        set_lang_auto();
    }

    /// 17 项口径 ↔ ATTDEF 逐项同序（表驱动护栏）。
    #[test]
    fn ansi_columns_cover_all_attdefs() {
        for side in [Internal, External] {
            let atts = attdefs(side);
            let cols = columns(side);
            assert_eq!(atts.len(), 17);
            assert_eq!(cols.len(), 17);
            for (i, (c, ad)) in cols.iter().zip(atts.iter()).enumerate() {
                assert_eq!(c.tag, ad.tag, "{side:?} 第 {} 项", i + 1);
                assert!(
                    !c.label().is_empty() && !c.formula().is_empty() && !c.source().is_empty(),
                    "{side:?} 第 {} 项 GUI 元数据空",
                    i + 1
                );
            }
        }
    }

    /// 取值正确性：P16 N20 列 A 的几何，与 `invol_spline.rs` ANSI 实现同源；
    /// 未收公差/量棒/公法线一律「—」。
    #[test]
    fn ansi_values_from_engine_and_missing() {
        let int = AnsiTableModel {
            card: "ANSI花键参数表_中文".into(),
            expr: None,
            lang: None,
            side: "int".into(),
            p: 16.0,
            z: 20,
            profile: None,
            at: None,
            rot: 0.0,
        }
        .spec()
        .unwrap();
        let q = int.params().unwrap();
        assert_eq!(int.profile, crate::invol_spline::ANSI_DEFAULT_PROFILE);
        assert!(near(q.ansi_p(), 16.0));
        let vals = values(&int).unwrap();
        assert_eq!(vals.len(), 17);
        let get = |tag: &str| vals.iter().find(|(t, _)| t == tag).unwrap().1.clone();
        assert_eq!(get("花键类型"), "30°平齿根齿侧配合");
        assert_eq!(get("齿数"), "20");
        assert_eq!(get("径节"), "16/32");
        assert_eq!(get("压力角"), "30°");
        assert_eq!(get("节圆直径"), crate::ansi_table::fmt_mm(q.d()));
        assert_eq!(get("基圆直径"), crate::ansi_table::fmt_mm(q.db()));
        assert_eq!(get("大径"), crate::ansi_table::fmt_mm(q.internal_major_dia()));
        assert_eq!(get("小径"), crate::ansi_table::fmt_mm(q.internal_minor_dia()));
        assert_eq!(get("有效直径"), crate::ansi_table::fmt_mm(q.ansi_form_dia_internal()));
        for tag in [
            "大径上差",
            "大径下差",
            "实际齿厚最大值",
            "作用齿厚最小值",
            "跨棒距上差",
            "跨棒距",
            "跨棒距下差",
            "量棒直径",
        ] {
            assert_eq!(get(tag), MISSING, "{tag} 应标缺（本仓未收）");
        }
        // 纯英文版：类型值用英文
        let mut en = AnsiTableModel {
            card: "ANSI花键参数表_英文".into(),
            expr: None,
            lang: Some("en".into()),
            side: "int".into(),
            p: 16.0,
            z: 20,
            profile: None,
            at: None,
            rot: 0.0,
        };
        let ven = values(&en.spec().unwrap()).unwrap();
        assert_eq!(
            ven.iter().find(|(t, _)| t == "花键类型").unwrap().1,
            "FLAT ROOT SIDE FIT"
        );
        // 外花键：Do/Dre/DFe 口径
        let ext = AnsiTableModel {
            card: "ANSI花键参数表_中文".into(),
            expr: None,
            lang: Some("cn".into()),
            side: "ext".into(),
            p: 16.0,
            z: 20,
            profile: Some("ANSI30R".into()),
            at: None,
            rot: 0.0,
        }
        .spec()
        .unwrap();
        let qe = ext.params().unwrap();
        let ve = values(&ext).unwrap();
        let gete = |tag: &str| ve.iter().find(|(t, _)| t == tag).unwrap().1.clone();
        assert_eq!(gete("大径"), crate::ansi_table::fmt_mm(qe.da()));
        assert_eq!(gete("小径"), crate::ansi_table::fmt_mm(qe.df()));
        assert_eq!(
            gete("渐开线终止圆直径"),
            crate::ansi_table::fmt_mm(qe.ansi_form_dia_external())
        );
        assert_eq!(gete("花键类型"), "30°圆齿根齿侧配合");
        en.lang = Some("en".into());
        assert_eq!(
            values(&en.spec().unwrap())
                .unwrap()
                .iter()
                .find(|(t, _)| t == "花键类型")
                .unwrap()
                .1,
            "FLAT ROOT SIDE FIT"
        );
    }

    /// CLI 解析 + 参数校验（系列/列范围/齿廓别名/未知参数）。
    #[test]
    fn ansi_spec_parse_and_errors() {
        let s = AnsiTableSpec::parse(AnsiLang::Cn, "内 P16 Z20").unwrap();
        assert_eq!(s.side, Internal);
        assert_eq!(s.profile, crate::invol_spline::ANSI_DEFAULT_PROFILE);
        assert_eq!(s.p, 16.0);
        assert_eq!(s.z, 20);
        // 五列代号与全名
        let s = AnsiTableSpec::parse(AnsiLang::En, "外 P12 Z24 profile ANSI30PM at 10,20 rot 30").unwrap();
        assert_eq!(s.side, External);
        assert_eq!(s.profile, "ANSI30平齿根外径");
        assert_eq!(s.at, Some([10.0, 20.0]));
        assert_eq!(s.rot, 30.0);
        assert_eq!(AnsiTableSpec::parse(AnsiLang::Cn, "内 P12 Z24 ANSI30R").unwrap().profile, "ANSI30圆齿根齿侧");
        assert_eq!(AnsiTableSpec::parse(AnsiLang::Cn, "内 径节 16/32 齿数 20").unwrap().p, 16.0);
        // 非系列径节
        let e = AnsiTableSpec::parse(AnsiLang::Cn, "内 P7 Z20").unwrap_err();
        assert!(e.contains("系列"), "{e}");
        // 列范围：45° 列下限 10（P8 越界）
        let e = AnsiTableSpec::parse(AnsiLang::Cn, "外 P8 Z20 ANSI45R").unwrap_err();
        assert!(e.contains("适用径节") || e.contains("低于下限"), "{e}");
        // 缺 P / 缺 Z / 未知参数
        assert!(AnsiTableSpec::parse(AnsiLang::Cn, "内 Z20").unwrap_err().contains("径节"));
        assert!(AnsiTableSpec::parse(AnsiLang::Cn, "内 P16").unwrap_err().contains("齿数"));
        assert!(AnsiTableSpec::parse(AnsiLang::Cn, "内 P16 Z20 bogus").unwrap_err().contains("bogus"));
    }

    /// 排版/几何断言：值文本不出表边界、中列/右列不串列（**中/英文都要**）。
    #[test]
    fn ansi_text_boxes_stay_in_columns() {
        use crate::spline_table::text_extent;
        for side in [Internal, External] {
            for lang in [AnsiLang::Cn, AnsiLang::En] {
                let spec = AnsiTableSpec {
                    side,
                    lang,
                    profile: crate::invol_spline::ANSI_DEFAULT_PROFILE,
                    p: 16.0,
                    z: 20,
                    at: None,
                    rot: 0.0,
                };
                let vals = values(&spec).unwrap();
                for ad in attdefs(side) {
                    let v = vals.iter().find(|(t, _)| t == &ad.tag).unwrap().1.clone();
                    let w = text_extent(&v, ad.height, ad.width_factor);
                    let left = ad.insertion_point.x;
                    let right = left + w;
                    assert!(
                        left >= -40.0 && right <= 1e-9,
                        "{side:?}/{lang:?} {} 值「{v}」越界 [{left:.3},{right:.3}]",
                        ad.tag
                    );
                    if ad.insertion_point.x > -22.0 {
                        // 右列
                        assert!(left >= -21.7936 - 1e-9, "{} 进中列", ad.tag);
                    } else {
                        // 中列：不得越过 x=−21.7936 的竖分栏
                        assert!(
                            right <= -21.7936 + 1e-6,
                            "{side:?}/{lang:?} {} 值「{v}」越入右列：右 {right:.3}",
                            ad.tag
                        );
                    }
                }
            }
            // 英文最长档（外花键列 B 的 `FLAT ROOT MAJOR DIA FIT`）：单独栅住
            let spec_en = AnsiTableSpec {
                side,
                lang: AnsiLang::En,
                profile: "ANSI30平齿根外径",
                p: 16.0,
                z: 20,
                at: None,
                rot: 0.0,
            };
            let v = values(&spec_en).unwrap();
            let ty = v.iter().find(|(t, _)| t == "花键类型").unwrap().1.clone();
            assert_eq!(ty, "FLAT ROOT MAJOR DIA FIT");
            let ad = attdefs(side).into_iter().find(|a| a.tag == "花键类型").unwrap();
            let w = text_extent(&ty, ad.height, ad.width_factor);
            assert!(
                ad.insertion_point.x + w <= -21.7936 + 1e-6,
                "最长英文类型值越入右列：{:.3}",
                ad.insertion_point.x + w
            );
            assert_eq!(ad.width_factor, TYPE_WIDTH_FACTOR);
        }
    }

    /// CLI/GUI 同源：同一参数两边出同一份 17 项。
    #[test]
    fn ansi_cli_and_model_same_values() {
        let cli = AnsiTableSpec::parse(AnsiLang::Cn, "内 P16 Z20").unwrap();
        let model = AnsiTableModel {
            card: "ANSI花键参数表_中文".into(),
            expr: None,
            lang: Some("cn".into()),
            side: "int".into(),
            p: 16.0,
            z: 20,
            profile: None,
            at: None,
            rot: 0.0,
        }
        .spec()
        .unwrap();
        assert_eq!(values(&cli).unwrap(), values(&model).unwrap());
        let md = markdown_table(&cli).unwrap();
        assert!(md.contains("| 花键类型 | 30°平齿根齿侧配合 |"));
        assert!(md.lines().count() >= 19);
    }

    fn expr_model(side: &str, expr: &str, profile: Option<&str>) -> AnsiTableModel {
        AnsiTableModel {
            card: "ANSI花键参数表_中文".into(),
            expr: Some(expr.into()),
            lang: None,
            side: side.into(),
            p: 0.0,
            z: 0,
            profile: profile.map(str::to_string),
            at: None,
            rot: 0.0,
        }
    }

    /// ★ 表达式 → 字段 → 卡内值：P=25.4/m、N=z、齿廓按 α；同 α 的用户选择保持。
    #[test]
    fn ansi_expr_maps_pitch_teeth_and_profile() {
        // P16 ⇔ m=25.4/16=1.5875；α30 → 列 A（30°平齿根齿侧）
        let spec = expr_model("int", "SPLINE IN M1.5875 Z20 ALPHA30 X0 BETA0 H30", None)
            .spec()
            .unwrap();
        assert!(near(spec.p, 16.0), "P=25.4/m：{}", spec.p);
        assert_eq!(spec.z, 20);
        assert_eq!(spec.profile, crate::invol_spline::ANSI_DEFAULT_PROFILE);
        let vals = values(&spec).unwrap();
        let get = |tag: &str| vals.iter().find(|(t, _)| t == tag).unwrap().1.clone();
        assert_eq!(get("齿数"), "20");
        assert_eq!(get("径节"), "16/32");
        assert_eq!(get("压力角"), "30°");
        // 同 α 的用户选择不被换列
        let keep = expr_model(
            "int",
            "SPLINE IN M1.5875 Z20 ALPHA30 X0 BETA0 H30",
            Some("ANSI30圆齿根齿侧"),
        )
        .spec()
        .unwrap();
        assert_eq!(keep.profile, "ANSI30圆齿根齿侧");
        // α=45 → 45° 圆齿根齿侧（外花键）
        let a45 = expr_model("ext", "SPLINE EX M1.5875 Z20 ALPHA45 X0 BETA0 H30", None)
            .spec()
            .unwrap();
        assert_eq!(a45.profile, "ANSI45圆齿根齿侧");
        // α=37.5 → 37.5° 圆齿根齿侧
        let a375 = expr_model("ext", "SPLINE EX M1.5875 Z20 ALPHA37.5 X0 BETA0 H30", None)
            .spec()
            .unwrap();
        assert_eq!(a375.profile, "ANSI37.5圆齿根齿侧");
        // 预览 JSON 回填 fields（GUI 自动回填用）
        let pj = expr_model("int", "SPLINE IN M1.5875 Z20 ALPHA30 X0 BETA0 H30", None)
            .preview_json()
            .unwrap();
        assert!(near(pj["fields"]["p"].as_f64().unwrap(), 16.0));
        assert_eq!(pj["fields"]["z"], 20);
        assert_eq!(pj["fields"]["profile"], crate::invol_spline::ANSI_DEFAULT_PROFILE);
        assert!(pj["expr"].as_str().unwrap().starts_with("SPLINE IN"));
    }

    /// 表达式错误路径：方向/体系/α/变位/系列/缺失。
    #[test]
    fn ansi_expr_error_paths() {
        // KIND 与卡片方向不一致（照 GB 卡文案）
        let e = expr_model("int", "SPLINE EX M1.5875 Z20 ALPHA30 X0 BETA0 H30", None)
            .spec()
            .unwrap_err();
        assert!(e.contains("KIND") && e.contains("EX（外）") && e.contains("「内」"), "{e}");
        // MARK 体系不符
        let e = expr_model("int", "GEAR IN M1.5875 Z20 ALPHA30 X0 BETA0 H30", None)
            .spec()
            .unwrap_err();
        assert!(e.contains("MARK") && e.contains("GEAR（齿轮）"), "{e}");
        // α 不在 30/37.5/45
        let e = expr_model("int", "SPLINE IN M3 Z20 ALPHA20 X0 BETA0 H30", None)
            .spec()
            .unwrap_err();
        assert!(e.contains("压力角 20°") && e.contains("30/37.5/45"), "{e}");
        // ANSI 不收变位
        let e = expr_model("int", "SPLINE IN M1.5875 Z20 ALPHA30 X0.4 BETA0 H30", None)
            .spec()
            .unwrap_err();
        assert!(e.contains("变位 X=0.4"), "{e}");
        // 径节系列外（m=3 → P=8.4667）
        let e = expr_model("int", "SPLINE IN M3 Z20 ALPHA30 X0 BETA0 H30", None)
            .spec()
            .unwrap_err();
        assert!(e.contains("不在标准系列") || e.contains("径节"), "{e}");
        // 缺表达式且缺 P
        let e = AnsiTableModel {
            card: "ANSI花键参数表_中文".into(),
            expr: None,
            lang: None,
            side: "int".into(),
            p: 0.0,
            z: 20,
            profile: None,
            at: None,
            rot: 0.0,
        }
        .spec()
        .unwrap_err();
        assert!(e.contains("P"), "{e}");
    }

    /// CLI 表达式形态：与模型同值；旧 P/Z 保留；显式冲突报错。
    #[test]
    fn ansi_cli_expr_form() {
        let expr = "内 SPLINE IN M1.5875 Z20 ALPHA30 X0 BETA0 H30";
        let cli = AnsiTableSpec::parse(AnsiLang::Cn, expr).unwrap();
        assert!(near(cli.p, 16.0) && cli.z == 20);
        assert_eq!(cli.profile, crate::invol_spline::ANSI_DEFAULT_PROFILE);
        let cli_en = AnsiTableSpec::parse(AnsiLang::En, "外 SPLINE EX M1.5875 Z20 ALPHA45 X0 BETA0 H30").unwrap();
        assert_eq!(cli_en.side, External);
        assert_eq!(cli_en.profile, "ANSI45圆齿根齿侧");
        // 旧写法不受影响
        let old = AnsiTableSpec::parse(AnsiLang::Cn, "内 P16 Z20").unwrap();
        assert_eq!(values(&cli).unwrap(), values(&old).unwrap());
        // 表达式与显式 P/Z 同值时允许；不同值 → 报错
        assert!(AnsiTableSpec::parse(AnsiLang::Cn, "内 SPLINE IN M1.5875 Z20 ALPHA30 X0 BETA0 H30 P16").is_ok());
        let e = AnsiTableSpec::parse(AnsiLang::Cn, "内 SPLINE IN M1.5875 Z20 ALPHA30 X0 BETA0 H30 Z22").unwrap_err();
        assert!(e.contains("显式齿数 N=22") && e.contains("N=20"), "{e}");
        // 表达式里有不认识的 token → 指路解析错误
        let e = AnsiTableSpec::parse(AnsiLang::Cn, "内 SPLINE IN M1.5875 Z20 ALPHA30 X0 bogus").unwrap_err();
        assert!(e.contains("无法解析"), "{e}");
    }
}
