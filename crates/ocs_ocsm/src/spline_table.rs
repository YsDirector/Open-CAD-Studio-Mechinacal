//! 智能卡片「花键参数表」（`OCSMCARD`）：**表格块几何内建**（模板逐图元照录，
//! 不依赖外部 DXF）+ 21 属性值映射 + 命令参数解析（吃**九字段齿形表达式**）。
//!
//! 模板：`~/桌面/GB/参数表/内花键参数表GB.dxf` / `外花键参数表GB.dxf`（两份同构，仅
//! 标签/符号/tag 前缀不同）。几何来源与坐标见下方 const（由模板逐图元提取）。
//! 表格块本身**与取值无关**（标签/符号固定），所以每个方向只建一次块
//! （`OCSM_SPTABLE_GB_INT` / `..._EXT`），每次插入用 `INSERT.attributes` 填 21 个值。
//!
//! 排版口径（用户 2026-09-25 截图反馈）：主值显示**最多 3 位小数**（显示与内部计算分开）；
//! 值列/公差列 ATTRIB **实体级**字宽因子 0.7（不动全局 `OCSM_GB` 样式）；
//! 主值右边界不得进入公差列（几何回归见 `spline_gui::tests::main_values_clear_tolerance_column`）。
//!
//! 单位口径：直径/跨棒距/公法线/圆弧半径 = mm；Ff/Fp/λ = μm（与 GB/T 3478.1 表 7~21 同）；
//! 公差 = mm（带符号）；齿形角 = `30°` 形式。

use crate::spline_tol::{
    ExtDev, PressureAngle, RootForm, SplineInput, SplineSide, SplineTable,
};
use ocs_plugin_api::host::acadrust::entities::{
    AttributeDefinition, AttributeEntity, Entity as _, EntityType, Insert, MText, Text,
};
use ocs_plugin_api::host::acadrust::types::{Color, LineWeight, Vector3};

/// 内花键参数表块名。
pub const BLOCK_INT: &str = "OCSM_SPTABLE_GB_INT";
/// 外花键参数表块名。
pub const BLOCK_EXT: &str = "OCSM_SPTABLE_GB_EXT";

/// 按方向取块名。
pub fn block_name(side: SplineSide) -> &'static str {
    match side {
        SplineSide::Internal => BLOCK_INT,
        SplineSide::External => BLOCK_EXT,
    }
}
// ---------- internal ----------
const INTERNAL_LINES: &[([f64; 2], [f64; 2], &str)] = &[
    ([-23.36718963751664, -34.49359974805751], [-23.36718963751664, -110.3795185976579], "2细线层"),
    ([-34.86505612988023, -6.898720166384521], [-34.86505612988023, -110.3795185976579], "2细线层"),
    ([-62.69374286414512, -110.3795185976579], [-62.69374286414512, 0.0], "1轮廓实线层"),
    ([0.0, 0.0], [0.0, -110.3795185976579], "1轮廓实线层"),
    ([-62.69374286414512, -110.3795185976579], [0.0, -110.3795185976579], "1轮廓实线层"),
    ([-62.69374286414512, -89.68335891140339], [0.0, -89.68335891140339], "2细线层"),
    ([-62.69374286414512, -82.7846390159852], [0.0, -82.7846390159852], "2细线层"),
    ([-62.69374286414512, -75.88591912056688], [0.0, -75.88591912056688], "2细线层"),
    ([-62.69374286414512, -68.98719922514871], [0.0, -68.98719922514871], "2细线层"),
    ([-62.69374286414512, -62.08847932973037], [0.0, -62.08847932973037], "2细线层"),
    ([-62.69374286414512, -55.18975943431221], [0.0, -55.18975943431221], "2细线层"),
    ([-62.69374286414512, -48.29103953889386], [0.0, -48.29103953889386], "2细线层"),
    ([-62.69374286414512, -41.3923196434757], [0.0, -41.3923196434757], "2细线层"),
    ([-62.69374286414512, -34.49359974805751], [0.0, -34.49359974805751], "2细线层"),
    ([-62.69374286414512, -27.5948798526392], [0.0, -27.5948798526392], "2细线层"),
    ([-62.69374286414512, -20.696159957221], [0.0, -20.696159957221], "2细线层"),
    ([-62.69374286414512, -13.79744006180269], [0.0, -13.79744006180269], "2细线层"),
    ([-62.69374286414512, -6.898720166384379], [0.0, -6.898720166384379], "2细线层"),
    ([0.0, 0.0], [-62.69374286414512, 0.0], "1轮廓实线层"),
    ([-23.36718963751664, -6.898720166384521], [-23.36718963751664, -27.5948798526392], "2细线层"),
    ([-62.69374286414512, -103.4807987022399], [0.0, -103.4807987022399], "2细线层"),
    ([-62.69374286414512, -96.58207880682149], [0.0, -96.58207880682149], "2细线层"),
];
const INTERNAL_TEXTS: &[(&str, f64, f64, f64, f64, &str)] = &[
    ("card.gb.label.total_composite", -62.69374286414518, -109.5446076115941, 3.6, 1.0, "6文字层"),
    ("card.gb.label.pitch_cumulative", -62.69374286414518, -102.7907789382295, 3.6, 0.8, "6文字层"),
    ("card.gb.label.profile", -62.69374286414518, -95.67305206200419, 3.6, 1.0, "6文字层"),
    ("Fp", -33.81015835284055, -101.8976261214447, 3.6, 1.0, "6文字层"),
    ("Ff", -33.91055722555311, -95.92724097636636, 3.6, 1.0, "6文字层"),
    ("card.gb.label.space_width_min", -62.74036870903419, -74.43506837667576, 3.6, 0.6, "6文字层"),
    ("card.gb.label.minor_dia", -62.18751327897629, -54.26665004362536, 3.6, 1.0, "6文字层"),
    ("card.gb.label.form_dia_max", -62.22825362065162, -47.39724973583915, 3.6, 0.4, "6文字层"),
    ("card.gb.label.major_dia", -61.55193148853519, -40.25802341502523, 3.6, 1.0, "6文字层"),
    ("card.gb.label.grade_fit", -61.744183745011334, -33.50989393471917, 3.6, 0.5, "6文字层"),
    ("card.gb.label.module", -61.70996272295335, -12.74995484307626, 3.6, 1.0, "6文字层"),
    ("card.gb.label.alpha", -61.70996272295335, -26.95440059678791, 3.6, 1.0, "6文字层"),
    ("card.gb.label.teeth", -61.70996272295335, -19.85217771993203, 3.6, 1.0, "6文字层"),
    ("m", -33.43646059027842, -12.26070290589675, 3.6, 1.0, "6文字层"),
    ("a", -32.20014068398604, -25.90460962548872, 3.6, 1.0, "6文字层"),
    ("z", -32.15253967264553, -19.15611735034653, 3.6, 1.0, "6文字层"),
    ("card.gb.title.int", -51.288982774652794, -5.200708803331338, 3.6, 1.0, "6文字层"),
    ("ei", -29.967321891043238, -40.851433265641774, 2.52, 1.0, "6文字层"),
    ("D", -34.17337885715943, -40.90694929205426, 3.6, 1.0, "6文字层"),
    ("D", -34.33504978280976, -47.62255944877526, 3.6, 1.0, "6文字层"),
    ("fimin", -30.446713731506556, -47.90561741469286, 2.52, 0.75, "6文字层"),
    ("D", -34.23964047591943, -54.56862870290547, 3.6, 1.0, "6文字层"),
    ("ii", -29.966867292505743, -54.68120223776848, 2.52, 0.699999988079071, "6文字层"),
    ("E", -34.32122037598592, -82.13962610404641, 3.6, 1.0, "6文字层"),
    ("max", -31.289100542159474, -82.31188950393954, 2.52, 0.75, "6文字层"),
    ("card.gb.label.space_width_max", -62.74036870903419, -81.48833021911646, 3.6, 0.6, "6文字层"),
    ("E", -34.38976126106141, -75.35278522794349, 3.6, 1.0, "6文字层"),
    ("vmin", -31.376086331746045, -75.44501807621538, 2.52, 0.75, "6文字层"),
    ("card.gb.label.root_fillet_radius", -62.55080601699521, -88.28439445877913, 3.6, 0.52, "6文字层"),
    ("R", -33.81343121065558, -89.047570182886, 3.6, 1.0, "6文字层"),
    ("imin", -30.713768025294595, -89.33570563800188, 2.52, 0.75, "6文字层"),
    ("card.gb.label.pin_dia", -62.18751327897629, -68.10668378690454, 3.6, 1.0, "6文字层"),
    ("card.gb.label.over_pins", -62.18751327897629, -60.96129455463037, 3.6, 0.9, "6文字层"),
    ("Dp", -33.65729574447627, -67.5576881494821, 3.6, 1.0, "6文字层"),
    ("Md", -33.31480838014204, -60.81415826218535, 3.6, 1.0, "6文字层"),
];
const INTERNAL_MTEXTS: &[(&str, f64, f64, f64, &str)] = &[
    ("\\Fgreeks.shx;l", -32.74314245831093, -104.4996602307434, 3.6, "6文字层"),
];
const INTERNAL_ATTDEFS: &[(&str, f64, f64, f64, &str)] = &[
    ("(内)齿形角", -21.36718963751662, -25.9948798526392, 3.6, "6文字层"),
    ("(内)齿数", -21.36718963751662, -19.096159957221, 3.6, "6文字层"),
    ("(内)模数", -21.36718963751662, -12.19744006180269, 3.6, "6文字层"),
    ("(内)公差等级和配合类别", -32.86505612988023, -32.89359974805751, 3.6, "6文字层"),
    ("(内)大径", -21.36718963751662, -39.79231964347571, 3.6, "6文字层"),
    ("(内)渐开线终止圆直径最大值", -21.36718963751662, -46.69103953889387, 3.6, "6文字层"),
    ("(内)小径", -21.36718963751662, -53.58975943431222, 3.6, "6文字层"),
    ("(内)测量跨棒距", -21.36718963751662, -60.48847932973038, 3.6, "6文字层"),
    ("(内)量棒直径", -21.36718963751662, -67.38719922514872, 3.6, "6文字层"),
    ("(内)作用齿槽宽最小值", -21.36718963751662, -74.28591912056687, 3.6, "6文字层"),
    ("(内)实际齿槽宽最大值", -21.36718963751662, -81.18463901598523, 3.6, "6文字层"),
    ("(内)齿根圆最小曲率半径", -21.36718963751662, -88.0833589114034, 3.6, "6文字层"),
    ("(内)齿形公差", -21.36718963751662, -94.9820788068215, 3.6, "6文字层"),
    ("(内)齿距累计公差", -21.36718963751662, -101.8807987022399, 3.6, "6文字层"),
    ("(内)综合公差", -21.36718963751662, -108.7795185976579, 3.6, "6文字层"),
    ("(内)小径.下公差", -9.497106487147901, -53.85559879316364, 1.8, "6文字层"),
    ("(内)小径.上公差", -9.504517159625607, -51.90745512108853, 1.8, "6文字层"),
    ("(内)测量跨棒距.下公差", -9.443948160479408, -60.96283257523471, 1.8, "6文字层"),
    ("(内)测量跨棒距.上公差", -9.451358832957112, -59.01468890315958, 1.8, "6文字层"),
    ("(内)大径.下公差", -9.62078383746109, -40.12233343290509, 1.8, "6文字层"),
    ("(内)大径.上公差", -9.628194509938792, -38.17418976082999, 1.8, "6文字层"),
];
// ---------- external ----------
const EXTERNAL_LINES: &[([f64; 2], [f64; 2], &str)] = &[
    ([-62.69374286414518, 2.709661828e-07], [-62.69374286414518, -110.3795183266917], "1轮廓实线层"),
    ([-23.36718963751674, -34.49359947709132], [-23.36718963751674, -110.3795183266917], "2细线层"),
    ([-34.86505612988027, -6.898719895418338], [-34.86505612988027, -110.3795183266917], "2细线层"),
    ([-7.1e-15, 0.0], [-7.1e-15, -110.3795183266917], "1轮廓实线层"),
    ([-62.69374286414518, -110.3795183266917], [-7.1e-15, -110.3795183266917], "1轮廓实线层"),
    ([-62.69374286414518, -89.6833586404372], [-7.1e-15, -89.6833586404372], "2细线层"),
    ([-62.69374286414518, -82.78463874501902], [-7.1e-15, -82.78463874501902], "2细线层"),
    ([-62.69374286414518, -75.8859188496007], [-7.1e-15, -75.8859188496007], "2细线层"),
    ([-62.69374286414518, -68.98719895418253], [-7.1e-15, -68.98719895418253], "2细线层"),
    ([-62.69374286414518, -62.08847905876419], [-7.1e-15, -62.08847905876419], "2细线层"),
    ([-62.69374286414518, -55.18975916334603], [-7.1e-15, -55.18975916334603], "2细线层"),
    ([-62.69374286414518, -48.29103926792768], [-7.1e-15, -48.29103926792768], "2细线层"),
    ([-62.69374286414518, -41.39231937250952], [-7.1e-15, -41.39231937250952], "2细线层"),
    ([-62.69374286414518, -34.49359947709132], [-7.1e-15, -34.49359947709132], "2细线层"),
    ([-62.69374286414518, -27.59487958167301], [-7.1e-15, -27.59487958167301], "2细线层"),
    ([-62.69374286414518, -20.69615968625482], [-7.1e-15, -20.69615968625482], "2细线层"),
    ([-62.69374286414518, -13.7974397908365], [-7.1e-15, -13.7974397908365], "2细线层"),
    ([-62.69374286414518, -6.898719895418196], [-7.1e-15, -6.898719895418196], "2细线层"),
    ([-7.1e-15, 0.0], [-62.69374286414518, 0.0], "1轮廓实线层"),
    ([-23.36718963751674, -6.898719895418338], [-23.36718963751674, -27.59487958167301], "2细线层"),
    ([-62.69374286414518, -103.4807984312737], [-7.1e-15, -103.4807984312737], "2细线层"),
    ([-62.69374286414518, -96.5820785358553], [-7.1e-15, -96.5820785358553], "2细线层"),
];
const EXTERNAL_TEXTS: &[(&str, f64, f64, f64, f64, &str)] = &[
    ("card.gb.label.total_composite", -62.69374286414518, -109.544607340628, 3.6, 1.0, "6文字层"),
    ("card.gb.label.pitch_cumulative", -62.69374286414518, -102.7907786672633, 3.6, 0.8, "6文字层"),
    ("card.gb.label.profile", -62.69374286414518, -95.673051791038, 3.6, 1.0, "6文字层"),
    ("Fp", -33.81015835284053, -101.8976258504785, 3.6, 1.0, "6文字层"),
    ("Ff", -33.91055722555314, -95.92724070540018, 3.6, 1.0, "6文字层"),
    ("card.gb.label.thickness_min", -62.7403687090341, -74.43506810570958, 3.6, 0.7, "6文字层"),
    ("card.gb.label.minor_dia", -62.18751327897621, -54.26664977265918, 3.6, 1.0, "6文字层"),
    ("card.gb.label.form_dia_max", -62.22825362065157, -47.39724946487297, 3.6, 0.4, "6文字层"),
    ("card.gb.label.major_dia", -61.55193148853516, -40.25802314405905, 3.6, 1.0, "6文字层"),
    ("card.gb.label.grade_fit", -61.74418374501124, -33.50989366375299, 3.6, 0.5, "6文字层"),
    ("card.gb.label.module", -61.70996272295326, -12.74995457211008, 3.6, 1.0, "6文字层"),
    ("card.gb.label.alpha", -61.70996272295326, -26.95440032582173, 3.6, 1.0, "6文字层"),
    ("card.gb.label.teeth", -61.70996272295326, -19.85217744896585, 3.6, 1.0, "6文字层"),
    ("m", -33.43646059027845, -12.26070263493056, 3.6, 1.0, "6文字层"),
    ("a", -34.07552393350103, -25.86975741918659, 3.6, 1.0, "6文字层"),
    ("z", -32.15253967264562, -19.15611707938035, 3.6, 1.0, "6文字层"),
    ("card.gb.title.ext", -51.420189287103995, -5.20070873823613, 3.6, 1.0, "6文字层"),
    ("ee", -30.038510405102382, -40.851433912565376, 2.52, 1.0, "6文字层"),
    ("D", -34.1733788571595, -40.90694902108808, 3.6, 1.0, "6文字层"),
    ("D", -34.33504978280986, -47.62255917780907, 3.6, 1.0, "6文字层"),
    ("femax", -30.629927474037416, -47.89476233584241, 2.52, 0.7, "6文字层"),
    ("D", -34.2396404759195, -54.56862843193929, 3.6, 1.0, "6文字层"),
    ("ie", -30.395464829327988, -54.68120288469209, 2.52, 0.699999988079071, "6文字层"),
    ("S", -34.32122037598595, -82.13962583308022, 3.6, 1.0, "6文字层"),
    ("vmax", -31.39445366364856, -82.31559341794187, 2.52, 0.8, "6文字层"),
    ("card.gb.label.thickness_max", -62.7403687090341, -81.4883299481503, 3.6, 0.7, "6文字层"),
    ("S", -34.3897612610615, -75.35278495697732, 3.6, 1.0, "6文字层"),
    ("min", -31.644967794676372, -75.44501947788305, 2.52, 1.0, "6文字层"),
    ("card.gb.label.root_fillet_radius", -62.55080601699513, -88.28439418781295, 3.6, 0.5, "6文字层"),
    ("R", -33.81343121065562, -89.04756991191982, 3.6, 1.0, "6文字层"),
    ("imin", -30.484130177906046, -89.33570703966956, 2.52, 0.9, "6文字层"),
    ("card.gb.label.span_teeth", -62.18751327897621, -68.10668351593836, 3.6, 1.0, "6文字层"),
    ("card.gb.label.base_tangent", -62.18751327897621, -60.96129428366419, 3.6, 0.90, "6文字层"),
    ("Kn", -33.65729574447631, -67.55768787851594, 3.6, 1.0, "6文字层"),
    ("Wn", -33.31480838014205, -60.81415799121916, 3.6, 1.0, "6文字层"),
];
const EXTERNAL_MTEXTS: &[(&str, f64, f64, f64, &str)] = &[
    ("\\Fgreeks.shx;l", -32.74314245831102, -104.4996599597772, 3.6, "6文字层"),
];
const EXTERNAL_ATTDEFS: &[(&str, f64, f64, f64, &str)] = &[
    ("(外)齿形角", -21.36718963751674, -25.9948798526392, 3.6, "6文字层"),
    ("(外)齿数", -21.36718963751674, -19.096159957221, 3.6, "6文字层"),
    ("(外)模数", -21.36718963751674, -12.19744006180269, 3.6, "6文字层"),
    ("(外)公差等级和配合类别", -32.86505612988032, -32.89359974805751, 3.6, "6文字层"),
    ("(外)大径", -21.36718963751674, -39.79231964347571, 3.6, "6文字层"),
    ("(外)渐开线终止圆直径最大值", -21.36718963751674, -46.69103953889387, 3.6, "6文字层"),
    ("(外)小径", -21.36718963751674, -53.58975943431222, 3.6, "6文字层"),
    ("(外)公法线长度", -21.36718963751674, -60.48847932973038, 3.6, "6文字层"),
    ("(外)跨测齿数", -21.36718963751674, -67.38719922514872, 3.6, "6文字层"),
    ("(外)实际齿厚最小值", -21.36718963751674, -74.28591912056687, 3.6, "6文字层"),
    ("(外)作用齿厚最大值", -21.36718963751674, -81.18463901598523, 3.6, "6文字层"),
    ("(外)齿根圆最小曲率半径", -21.36718963751674, -88.0833589114034, 3.6, "6文字层"),
    ("(外)齿形公差", -21.36718963751674, -94.9820788068215, 3.6, "6文字层"),
    ("(外)齿距累计公差", -21.36718963751674, -101.8807987022399, 3.6, "6文字层"),
    ("(外)综合公差", -21.36718963751674, -108.7795185976579, 3.6, "6文字层"),
    ("(外)小径.下公差", -9.497106487148024, -53.85559879316364, 1.8, "6文字层"),
    ("(外)小径.上公差", -9.504517159625728, -51.90745512108853, 1.8, "6文字层"),
    ("(外)公法线长度.下公差", -9.443948160479529, -60.96283257523471, 1.8, "6文字层"),
    ("(外)公法线长度.上公差", -9.451358832957231, -59.01468890315958, 1.8, "6文字层"),
    ("(外)大径.下公差", -9.62078383746121, -40.12233343290509, 1.8, "6文字层"),
    ("(外)大径.上公差", -9.628194509938915, -38.17418976082999, 1.8, "6文字层"),
];

// ══════════════════════════════════════════════════════════════════════════
// 表格块构造（逐图元照录模板；**卡面文字随语言**）
// ══════════════════════════════════════════════════════════════════════════

/// 卡面文字取词：catalog key（`card.` 前缀）→ 当前语言；**符号/代号原样**（`Fp`/`D`/`m`/`z`…）。
fn face_text(key: &str) -> String {
    if key.starts_with("card.") {
        crate::i18n::t(key)
    } else {
        key.to_string()
    }
}

/// 英文卡面文字的**实体级字宽**（`None` = 用模板原值 zh 字宽）。
///
/// 口径：真字体 metrics（宿主把 TTF 轮廓归一化到 cap height；朱雀仿宋 cap = 0.637em ⇒
/// 宽 = Σ(advance/em) × 字号 × 1.57 × width_factor）。英文标签天生比中文长，
/// 逐条按「宽 ≤ (标签列右界 −34.865056 − 插入 x) × 0.97（≥3% 余量）」预算得出，
/// 并同时满足仓库保守模型 `text_extent_ttf()`（预算脚本：`~/桌面/OCSM/review/i18n_检查/card_gb_width_plan.py`）。
/// **不动列宽/线位/全局 `OCSM_GB`**（用户 2026-09-27 口径：装不下就压实体字宽）。
fn en_label_width_factor(key: &str) -> Option<f64> {
    Some(match key {
        "card.gb.title.int" => 0.66,
        "card.gb.title.ext" => 0.65,
        "card.gb.label.total_composite" => 0.44,
        "card.gb.label.pitch_cumulative" => 0.42,
        "card.gb.label.profile" => 0.69,
        "card.gb.label.space_width_min" => 0.43,
        "card.gb.label.space_width_max" => 0.47,
        "card.gb.label.thickness_min" => 0.42,
        "card.gb.label.thickness_max" => 0.38,
        "card.gb.label.minor_dia" => 0.70,
        "card.gb.label.major_dia" => 0.70,
        "card.gb.label.form_dia_max" => 0.55,
        "card.gb.label.grade_fit" => 0.48,
        "card.gb.label.module" => 1.0,
        "card.gb.label.alpha" => 0.75,
        "card.gb.label.teeth" => 0.67,
        "card.gb.label.root_fillet_radius" => 0.52,
        "card.gb.label.pin_dia" => 0.86,
        "card.gb.label.over_pins" => 0.47,
        "card.gb.label.span_teeth" => 1.0,
        "card.gb.label.base_tangent" => 0.57,
        _ => return None,
    })
}

/// 卡面文字字宽因子：中文用模板原值；英文用 [`en_label_width_factor`] 覆盖。
fn face_width_factor(key: &str, wf_zh: f64) -> f64 {
    match (crate::i18n::lang(), en_label_width_factor(key)) {
        (crate::i18n::Lang::En, Some(wf)) => wf,
        _ => wf_zh,
    }
}

fn text_ent(value: &str, x: f64, y: f64, h: f64, wf: f64, layer: &str) -> EntityType {
    let mut t = Text::with_value(value, Vector3::new(x, y, 0.0));
    t.height = h;
    t.width_factor = wf;
    t.style = "OCSM_GB".into();
    t.common.layer = layer.to_string();
    t.common.color = Color::ByLayer;
    t.common.linetype = "ByLayer".into();
    EntityType::Text(t)
}

fn mtext_ent(value: &str, x: f64, y: f64, h: f64, layer: &str) -> EntityType {
    let mut m = MText::new();
    m.value = value.to_string();
    m.insertion_point = Vector3::new(x, y, 0.0);
    m.height = h;
    m.rectangle_width = (value.chars().count() as f64 * h * 0.75).max(10.0);
    m.style = "OCSM_GB".into();
    m.common.layer = layer.to_string();
    m.common.color = Color::ByLayer;
    m.common.linetype = "ByLayer".into();
    EntityType::MText(m)
}

/// 值列/公差列的**实体级**字宽压缩（不动全局样式 `OCSM_GB`，只影响本表 ATTRIB）。
///
/// 模板 ATTDEF 原来是 1.0：长主值（未截小数的直径/跨棒距）会一路顶到 x≈−9.5 的公差列，
/// 与上/下公差叠成乱码（用户截图实测）。0.7 与样式 `OCSM_GB` 的宽度因子同口径，
/// 值列还剩 ~30% 余量；换字体/字号也只影响本表。
pub const VALUE_WIDTH_FACTOR: f64 = 0.7;

/// 上/下公差文字**实体级**右移（用户 2026-09-26：公差右移一个字符的位置）。
///
/// 口径 = 一个字符宽 = 该字高（1.8）× 实体字宽因子（[`VALUE_WIDTH_FACTOR`] 0.7）= **1.26**；
/// 只挪公差行 ATTDEF 的插入点 x，不动全局 `OCSM_GB` 样式，也不改字号/字宽。
pub const TOL_X_SHIFT: f64 = 1.8 * VALUE_WIDTH_FACTOR;

/// 公差 tag（`.上公差`/`.下公差`）→ 实体级右移；其余不动。
fn tolerance_shift(tag: &str) -> f64 {
    if tag.ends_with(".上公差") || tag.ends_with(".下公差") {
        TOL_X_SHIFT
    } else {
        0.0
    }
}

fn attdef(tag: &str, x: f64, y: f64, h: f64, layer: &str) -> AttributeDefinition {
    let x = x + tolerance_shift(tag);
    let mut ad = AttributeDefinition::new(tag.to_string(), String::new(), " ".to_string());
    ad.insertion_point = Vector3::new(x, y, 0.0);
    ad.alignment_point = ad.insertion_point;
    ad.height = h;
    ad.width_factor = VALUE_WIDTH_FACTOR;
    ad.text_style = "OCSM_GB".into();
    ad.flags.preset = true;
    ad.common.layer = layer.to_string();
    ad
}

fn side_data(
    side: SplineSide,
) -> (
    &'static [([f64; 2], [f64; 2], &'static str)],
    &'static [(&'static str, f64, f64, f64, f64, &'static str)],
    &'static [(&'static str, f64, f64, f64, &'static str)],
    &'static [(&'static str, f64, f64, f64, &'static str)],
) {
    match side {
        SplineSide::Internal => (
            INTERNAL_LINES,
            INTERNAL_TEXTS,
            INTERNAL_MTEXTS,
            INTERNAL_ATTDEFS,
        ),
        SplineSide::External => (
            EXTERNAL_LINES,
            EXTERNAL_TEXTS,
            EXTERNAL_MTEXTS,
            EXTERNAL_ATTDEFS,
        ),
    }
}

/// 表格块成员（22 线 + 标签/符号 + 21 ATTDEF；ATTDEF default 空）。
pub fn block_entities(side: SplineSide) -> Vec<EntityType> {
    let (lines, texts, mtexts, atts) = side_data(side);
    let mut out = Vec::with_capacity(lines.len() + texts.len() + mtexts.len() + atts.len());
    for (a, b, layer) in lines {
        out.push(crate::partgen_kit::line(*a, *b, layer));
    }
    for (value, x, y, h, wf, layer) in texts {
        out.push(text_ent(
            &face_text(value),
            *x,
            *y,
            *h,
            face_width_factor(value, *wf),
            layer,
        ));
    }
    for (value, x, y, h, layer) in mtexts {
        out.push(mtext_ent(value, *x, *y, *h, layer));
    }
    for (tag, x, y, h, layer) in atts {
        out.push(EntityType::AttributeDefinition(attdef(
            tag, *x, *y, *h, layer,
        )));
    }
    out
}

/// 21 个 ATTDEF（插入时按 tag 与 `values` 对齐）。
pub fn attdefs(side: SplineSide) -> Vec<AttributeDefinition> {
    let (_, _, _, atts) = side_data(side);
    atts.iter()
        .map(|(tag, x, y, h, layer)| attdef(tag, *x, *y, *h, layer))
        .collect()
}

// ══════════════════════════════════════════════════════════════════════════
// 21 属性取值（与 `spline_tol::compute()` 同源）
// ══════════════════════════════════════════════════════════════════════════

/// 主值/尺寸的**显示**格式：最多 3 位小数（截到 0.001 再抹尾零）。
///
/// **显示与内部计算分开**：`spline_tol` 里仍是全精度 f64，只有写 ATTRIB 时才截；
/// 不截的话长小数（如 78.113360…）会把值列顶进公差列（用户截图实测）。
pub(crate) fn fmt_mm(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// 字符推进宽度（em）：朱雀仿宋实测数字 0.354~0.533、`.` 0.305；这里取保守上限。
#[cfg(test)]
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

/// 估算左对齐文本宽度（世界单位）= Σ char_em × 字高 × 字宽因子（几何断言用）。
#[cfg(test)]
pub fn text_extent(value: &str, height: f64, width_factor: f64) -> f64 {
    value.chars().map(char_em).sum::<f64>() * height * width_factor
}

/// 估算左对齐文本的轴对齐框 `[x0, y0, x1, y1]`（基点在左下，与 acadrust
/// `Text::bounding_box` 同口径但按字符分类，比它的 `len×0.6` 更贴近真实字体）。
#[cfg(test)]
pub fn text_box(insertion: [f64; 2], height: f64, width_factor: f64, value: &str) -> [f64; 4] {
    [
        insertion[0],
        insertion[1],
        insertion[0] + text_extent(value, height, width_factor),
        insertion[1] + height,
    ]
}

/// TTF **实际渲染宽**（保守）：宿主把 TTF 轮廓归一化到 cap height=9 再乘 `height/9`
/// （`src/scene/text/ttf_glyph.rs::cap_scale`）；朱雀仿宋 cap = 0.637 em ⇒
/// 实际宽 ≈ Σ(advance_em) × height × 1.57 × width_factor。
///
/// 各类别取实测上限再留裕量（CJK=1.0、字母 0.75、数字 0.55、空格 0.20、
/// 其它 ASCII 0.60、非 CJK 符号 0.60、破折号/省略号 1.0）。
/// 与旧 `char_em()`（未含 cap-height 归一化）并存：用户 2026-09-27 长标签扫描后的
/// 新断言一律用本函数。
#[cfg(test)]
pub(crate) const TTF_EM_PER_HEIGHT: f64 = 1.57;

#[cfg(test)]
fn char_em_ttf(c: char) -> f64 {
    match c {
        ' ' => 0.20,
        '\u{2014}' | '\u{2026}' => 1.0,
        // 朱雀仿宋实测 advance（上取整到 0.01；卡面出现的拉丁字母/符号）
        'G' => 0.70,
        'B' => 0.62,
        'T' => 0.57,
        'D' => 0.72,
        'p' => 0.55,
        'K' => 0.64,
        'n' => 0.56,
        'W' => 0.94,
        'M' => 0.86,
        'd' => 0.55,
        'm' => 0.84,
        'i' => 0.30,
        'e' => 0.46,
        'z' => 0.47,
        'α' => 0.58,
        '°' => 0.33,
        // 其余拉丁字母同口径（朱雀仿宋实测 advance，上取整到 0.01；2026-09-27 精简卡扩体系用）
        'A' => 0.59,
        'C' => 0.63,
        'E' => 0.60,
        'F' => 0.52,
        'H' => 0.76,
        'I' => 0.37,
        'J' => 0.33,
        'L' => 0.51,
        'N' => 0.72,
        'O' => 0.70,
        'P' => 0.56,
        'Q' => 0.70,
        'R' => 0.62,
        'S' => 0.53,
        'U' => 0.69,
        'V' => 0.61,
        'X' => 0.66,
        'Y' => 0.54,
        'Z' => 0.57,
        'a' => 0.47,
        'b' => 0.51,
        'c' => 0.43,
        'f' => 0.33,
        'g' => 0.51,
        'h' => 0.54,
        'j' => 0.25,
        'k' => 0.50,
        'l' => 0.25,
        'o' => 0.50,
        'q' => 0.52,
        'r' => 0.39,
        's' => 0.41,
        't' => 0.33,
        'u' => 0.55,
        'v' => 0.45,
        'w' => 0.71,
        'x' => 0.48,
        'y' => 0.45,
        '_' => 0.46,
        '·' | '×' | '±' | '−' => 0.55,
        _ if c.is_ascii_digit() => 0.54,
        '.' | ',' => 0.35,
        '-' | '+' => 0.55,
        '/' => 0.50,
        '(' | ')' => 0.31,
        _ if c.is_ascii_alphabetic() => 0.95, // 未列字母按上限（W=0.932）
        _ if c.is_ascii() => 0.60,
        _ if ('\u{4E00}'..='\u{9FFF}').contains(&c)
            || ('\u{3000}'..='\u{303F}').contains(&c)
            || ('\u{FF00}'..='\u{FFEF}').contains(&c) => 1.0,
        _ => 1.0, // 未知非 ASCII 按全角保守
    }
}

/// TTF 实际宽的保守估算（几何断言用）。
#[cfg(test)]
pub(crate) fn text_extent_ttf(value: &str, height: f64, width_factor: f64) -> f64 {
    value.chars().map(char_em_ttf).sum::<f64>() * height * TTF_EM_PER_HEIGHT * width_factor
}

/// TTF 实际宽的保守估算框（几何断言用）。
#[cfg(test)]
pub(crate) fn text_box_ttf(
    insertion: [f64; 2],
    height: f64,
    width_factor: f64,
    value: &str,
) -> [f64; 4] {
    [
        insertion[0],
        insertion[1],
        insertion[0] + text_extent_ttf(value, height, width_factor),
        insertion[1] + height,
    ]
}

pub(crate) fn fmt_deg(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}°", v.round() as i64)
    } else {
        format!("{}°", crate::partgen_kit::trim(v))
    }
}

/// 带符号 mm 公差（`+0.12` / `-0.046` / `0`）。
fn fmt_dev(v: f64) -> String {
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

/// tag → 值（去掉 `(内)`/`(外)` 前缀后匹配）。
fn value_for(tag: &str, table: &SplineTable) -> Option<String> {
    let key = tag
        .trim_start_matches("(内)")
        .trim_start_matches("(外)")
        .trim();
    match table {
        SplineTable::Internal(t) => value_internal(key, t),
        SplineTable::External(t) => value_external(key, t),
    }
}

fn value_internal(
    key: &str,
    t: &crate::spline_tol::InternalSplineTable,
) -> Option<String> {
    let s = |v: f64| fmt_mm(v);
    match key {
        "齿形角" => Some(fmt_deg(t.alpha_deg)),
        "齿数" => Some(t.z.to_string()),
        "模数" => Some(fmt_mm(t.m)),
        "公差等级和配合类别" => Some(t.grade_fit.clone()),
        "大径" => Some(s(t.major_dia)),
        "大径.下公差" => Some(fmt_dev(t.major_lower)),
        "大径.上公差" => Some(fmt_dev(t.major_upper)),
        "渐开线终止圆直径最大值" => Some(s(t.dfimin)),
        "小径" => Some(s(t.minor_dia)),
        "小径.下公差" => Some(fmt_dev(t.minor_lower)),
        "小径.上公差" => Some(fmt_dev(t.minor_upper)),
        "测量跨棒距" => Some(s(t.md)),
        "测量跨棒距.下公差" => Some(fmt_dev(t.md_lower - t.md)),
        "测量跨棒距.上公差" => Some(fmt_dev(t.md_upper - t.md)),
        "量棒直径" => Some(s(t.dp)),
        "作用齿槽宽最小值" => Some(s(t.eval_min)),
        "实际齿槽宽最大值" => Some(s(t.e_max)),
        // 卡面口径（用户 2026-09-27 拍板）：表 26 有值用表值；原表「—」/表外 →「—」。
        "齿根圆最小曲率半径" => Some(t.rimin_table.map(s).unwrap_or_else(|| "—".to_string())),
        "齿形公差" => Some(fmt_mm(t.ff)),
        "齿距累计公差" => Some(fmt_mm(t.fp)),
        "综合公差" => Some(fmt_mm(t.lambda)),
        _ => None,
    }
}

fn value_external(
    key: &str,
    t: &crate::spline_tol::ExternalSplineTable,
) -> Option<String> {
    let s = |v: f64| fmt_mm(v);
    match key {
        "齿形角" => Some(fmt_deg(t.alpha_deg)),
        "齿数" => Some(t.z.to_string()),
        "模数" => Some(fmt_mm(t.m)),
        "公差等级和配合类别" => Some(t.grade_fit.clone()),
        "大径" => Some(s(t.major_dia)),
        "大径.下公差" => Some(fmt_dev(t.major_lower)),
        "大径.上公差" => Some(fmt_dev(t.major_upper)),
        "渐开线终止圆直径最大值" => Some(s(t.dfemax)),
        "小径" => Some(s(t.minor_dia)),
        "小径.下公差" => Some(fmt_dev(t.minor_lower)),
        "小径.上公差" => Some(fmt_dev(t.minor_upper)),
        "公法线长度" => Some(s(t.wn)),
        "公法线长度.下公差" => Some(fmt_dev(t.wn_lower - t.wn)),
        "公法线长度.上公差" => Some(fmt_dev(t.wn_upper - t.wn)),
        "跨测齿数" => Some(t.kn.to_string()),
        "实际齿厚最小值" => Some(s(t.s_min)),
        "作用齿厚最大值" => Some(s(t.sv_max)),
        // 卡面口径（用户 2026-09-27 拍板）：表 26 有值用表值；原表「—」/表外 →「—」。
        "齿根圆最小曲率半径" => Some(t.rimin_table.map(s).unwrap_or_else(|| "—".to_string())),
        "齿形公差" => Some(fmt_mm(t.ff)),
        "齿距累计公差" => Some(fmt_mm(t.fp)),
        "综合公差" => Some(fmt_mm(t.lambda)),
        _ => None,
    }
}

/// 21 个 tag 的值（顺序与 `attdefs()` 一致）。
pub fn values(side: SplineSide, table: &SplineTable) -> Result<Vec<(String, String)>, String> {
    let atts = attdefs(side);
    let mut out = Vec::with_capacity(atts.len());
    for ad in &atts {
        let v = value_for(&ad.tag, table).ok_or_else(|| {
            crate::i18n::t_fmt("cmd.spline.err.values_tag_missing", &[("tag", &ad.tag)])
        })?;
        out.push((ad.tag.clone(), v));
    }
    Ok(out)
}

/// 建 INSERT（基点在 `at`，旋转 `rot_deg` 度；21 个 `INSERT.attributes` 取自 `values()`）。
///
/// **CLI（`cmd_spline_table`）与 GUI 导出（`/api/spline_export`）共用这一条路径**——
/// 块几何只建一次（`block_entities`），值只映射一次（`values`），两处不会漂。
pub fn build_insert(
    side: SplineSide,
    table: &SplineTable,
    at: [f64; 2],
    rot_deg: f64,
) -> Result<Insert, String> {
    let values = values(side, table)?;
    let mut ins = Insert::new(block_name(side), Vector3::new(at[0], at[1], 0.0));
    ins.rotation = rot_deg.to_radians();
    {
        let c = &mut ins.common;
        c.layer = crate::partgen::LAYER_MAIN.to_string();
        c.color = Color::ByLayer;
        c.linetype = "ByLayer".to_string();
        c.line_weight = LineWeight::ByLayer;
    }
    for ad in attdefs(side).iter() {
        let val = values
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

/// 21 项 Markdown（计算书新节用；与表同一份 `compute()` 结果）。
pub fn markdown_table(side: SplineSide, table: &SplineTable) -> Result<String, String> {
    let vals = values(side, table)?;
    let mut md = String::new();
    md.push_str("| 属性 | 值 |\n|---|---|\n");
    for (tag, v) in vals {
        md.push_str(&format!("| {} | {} |\n", tag, v));
    }
    Ok(md)
}

/// 从轴段 `Gear` 构造输入（**唯一生成路径 = 齿轮生成器同一 `GearParams`**）。
/// 报告/命令共用；默认 **7 级 + 基孔制 H/h**（`OCSMCARD` 可显式改等级/配合/Dp）。
/// 齿根：先由表达式 `DA/DF` 反解（30° 可分辨），再退到花键齿廓代号 `P/R`，最后按 αD 默认。
pub fn input_from_gear(g: &crate::shaft::Gear) -> Result<SplineInput, String> {
    let gp = g.params();
    let alpha = PressureAngle::parse(&format!("{}", gp.alpha_deg))?;
    let side = if gp.kind.is_internal() {
        SplineSide::Internal
    } else {
        SplineSide::External
    };
    let profile = gp
        .spline
        .as_ref()
        .map(|s| s.profile.to_ascii_uppercase())
        .unwrap_or_default();
    let root = root_from_gear(g, side)
        .or_else(|| {
            if profile.ends_with('P') {
                Some(RootForm::Flat)
            } else if profile.ends_with('R') {
                Some(RootForm::Fillet)
            } else {
                None
            }
        })
        .unwrap_or(match alpha {
            PressureAngle::A30 => RootForm::Flat,
            _ => RootForm::Fillet,
        });
    Ok(SplineInput {
        m: gp.m,
        z: gp.z,
        alpha,
        root,
        side,
        grade: 7,
        ext_dev: ExtDev::H,
        fit_length: None,
        dp: None,
    })
}

// ══════════════════════════════════════════════════════════════════════════
// 命令参数解析（智能卡片 `OCSMCARD` 的「花键参数表」卡）
// ══════════════════════════════════════════════════════════════════════════

/// 「花键参数表」卡参数：
/// `[std GB] 内 6H <九字段表达式> [dp 4.5] [root 平|圆] [at x,y] [rot 度]`。
///
/// **参数来源 = 表达式**（用户 2026-09-25 裁定）：直接吃齿轮/花键/轴生成器 GUI
/// 复制的**九字段统一齿形表达式**（`MARK KIND M Z ALPHA X DA DF BETA H`），
/// 在这里反解出 `m/z/αD/x/Da/Df` 再交给 `spline_tol::compute()`。
/// **不读选中/上一个块**——那条路径会在命令中途要求用户去选对象，放置状态会丢。
#[derive(Debug, Clone)]
pub struct SplineTableSpec {
    /// 体系 id（表驱动；本期只有 `gb3478`）。
    pub system: &'static str,
    pub side: SplineSide,
    pub grade: u32,
    pub ext_dev: ExtDev,
    /// 九字段表达式原文（回显/计算书）。
    pub expr: String,
    /// 表达式反解出的齿形段（`m/z/αD/x/DA/DF/KIND`）。
    pub gear: crate::shaft::Gear,
    /// 齿根形式：显式 `root` > 由表达式 `DA/DF` 反解（30° 平/圆公式可分辨）> 按 αD 默认。
    pub root: Option<RootForm>,
    pub dp: Option<f64>,
    pub at: Option<[f64; 2]>,
    pub rot: f64,
}

/// 九字段表达式 → 齿形段 `Gear`（**唯一实现**；`card` = 报错前缀，如「花键参数表」）。
///
/// 复用既有 `shaft::parse_program`，不另写解析器；五张智能卡片（含齿轮/ANSI/NF/DIN）
/// 都经这里解析，各卡只换前缀。
pub fn parse_gear_expr(expr: &str, card: &str) -> Result<crate::shaft::Gear, String> {
    let card = crate::i18n::card_display(card);
    if expr.trim().is_empty() {
        return Err(crate::i18n::t_fmt(
            "cmd.card.err.expr_missing",
            &[("card", &card)],
        ));
    }
    let program = crate::shaft::parse_program(expr).map_err(|e| {
        crate::i18n::t_fmt("cmd.card.err.expr_parse", &[("card", &card), ("e", &e)])
    })?;
    if program.segments.len() != 1 {
        return Err(crate::i18n::t_fmt(
            "cmd.card.err.expr_multi",
            &[("card", &card), ("n", &program.segments.len().to_string())],
        ));
    }
    program.segments[0].gear.ok_or_else(|| {
        crate::i18n::t_fmt("cmd.card.err.expr_no_gear", &[("card", &card)])
    })
}

/// 九字段表达式 → 齿形段 `Gear`（花键参数表口径的薄包装；其余卡走 [`parse_gear_expr`]）。
pub fn parse_expr_gear(expr: &str) -> Result<crate::shaft::Gear, String> {
    parse_gear_expr(expr, "花键参数表")
}

/// 表达式里是否显式写了 `IN`/`EX`（`true` = 内齿）；缺省 = 外齿（与 `shaft` 口径一致）。
pub fn expr_explicit_kind(expr: &str) -> Option<bool> {
    for t in expr.split_whitespace() {
        if t.eq_ignore_ascii_case("IN") {
            return Some(true);
        }
        if t.eq_ignore_ascii_case("EX") {
            return Some(false);
        }
    }
    None
}

/// 由表达式 `DA/DF` 反解齿根形式（GB/T 3478.1 表 3：**只有 30° 的平/圆公式不同**，
/// 37.5°/45° 推不出来 → `None`，由调用方按 αD 默认）。
///
/// 内花键看大径 `Dei`（平 `m(z+1.5)`、圆 `m(z+1.8)`），外花键看小径 `Die`
/// （平 `m(z−1.5)`、圆 `m(z−1.8)`）——表 3 的平/圆差异就在这两处。
pub fn root_from_gear(g: &crate::shaft::Gear, side: SplineSide) -> Option<RootForm> {
    let alpha = PressureAngle::parse(&format!("{}", g.alpha_deg)).ok()?;
    if alpha != PressureAngle::A30 {
        return None;
    }
    let target = match side {
        SplineSide::Internal => g.da?,
        SplineSide::External => g.df?,
    };
    let (flat, fillet) = match side {
        SplineSide::Internal => (
            crate::spline_tol::internal_major(alpha, RootForm::Flat, g.m, g.z),
            crate::spline_tol::internal_major(alpha, RootForm::Fillet, g.m, g.z),
        ),
        SplineSide::External => (
            crate::spline_tol::external_minor(alpha, RootForm::Flat, g.m, g.z),
            crate::spline_tol::external_minor(alpha, RootForm::Fillet, g.m, g.z),
        ),
    };
    if (target - flat).abs() < 5e-3 {
        Some(RootForm::Flat)
    } else if (target - fillet).abs() < 5e-3 {
        Some(RootForm::Fillet)
    } else {
        None
    }
}

impl SplineTableSpec {
    /// 解析 `[std GB] 内 6H <表达式> [dp 4.5] [root 平|圆] [at x,y] [rot 度]`。
    pub fn parse(text: &str) -> Result<Self, String> {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let mut i = 0usize;
        // 可选体系：`std GB`（体系 = 表驱动的一项；本期只有 gb3478）。
        let mut system = "gb3478";
        if let Some(t) = tokens.get(i) {
            if matches!(t.to_ascii_lowercase().as_str(), "std" | "标准" | "体系") {
                i += 1;
                let v = tokens
                    .get(i)
                    .copied()
                    .ok_or_else(|| crate::i18n::t("cmd.spline.err.std_missing_system"))?;
                system = crate::spline_gui::system_id_by_token(v)?;
                i += 1;
            }
        }
        let side = match tokens.get(i).copied() {
            Some("内") | Some("内部") | Some("内花键") | Some("int") | Some("internal") => {
                SplineSide::Internal
            }
            Some("外") | Some("外部") | Some("外花键") | Some("ext") | Some("external") => {
                SplineSide::External
            }
            Some(other) => {
                return Err(crate::i18n::t_fmt(
                    "cmd.spline.err.side_first",
                    &[("other", other), ("usage", &usage())],
                ))
            }
            None => return Err(usage()),
        };
        i += 1;
        let mark = tokens.get(i).copied().ok_or_else(usage)?;
        i += 1;
        let (grade, fit_text) = split_grade_fit(mark)?;
        let ext_dev = match side {
            SplineSide::Internal => {
                if !fit_text.eq_ignore_ascii_case("h") {
                    return Err(crate::i18n::t_fmt(
                        "cmd.spline.err.internal_h_mark",
                        &[("mark", mark)],
                    ));
                }
                ExtDev::H
            }
            SplineSide::External => {
                if fit_text.eq_ignore_ascii_case("h") {
                    ExtDev::H
                } else {
                    ExtDev::parse(fit_text)?
                }
            }
        };
        // 表达式 = mark 之后一直吃到第一个卡选项关键字（dp/root/at/rot）；
        // 齿形关键字里不会出现这四个词（X/DF 等是带值前缀）。
        let expr_start = i;
        while i < tokens.len() {
            let k = tokens[i].to_ascii_lowercase();
            if matches!(
                k.as_str(),
                "dp" | "root" | "齿根" | "齿根形式" | "at" | "rot" | "旋转"
            ) {
                break;
            }
            i += 1;
        }
        let expr = tokens[expr_start..i].join(" ");
        let gear = parse_expr_gear(&expr)?;
        if let Some(internal) = expr_explicit_kind(&expr) {
            if internal != (side == SplineSide::Internal) {
                return Err(crate::i18n::t_fmt(
                    "cmd.spline.err.kind_mismatch",
                    &[
                        (
                            "kind",
                            &crate::i18n::t(if internal {
                                "cmd.cardexpr.kind.in"
                            } else {
                                "cmd.cardexpr.kind.ex"
                            }),
                        ),
                        (
                            "want",
                            &crate::i18n::t(if side == SplineSide::Internal {
                                "cmd.cardexpr.dir.int"
                            } else {
                                "cmd.cardexpr.dir.ext"
                            }),
                        ),
                    ],
                ));
            }
        }
        let mut spec = SplineTableSpec {
            system,
            side,
            grade,
            ext_dev,
            expr,
            gear,
            root: None,
            dp: None,
            at: None,
            rot: 0.0,
        };
        while i < tokens.len() {
            let key = tokens[i].to_ascii_lowercase();
            let mut need = |what: &str| -> Result<&str, String> {
                i += 1;
                tokens
                    .get(i)
                    .copied()
                    .ok_or_else(|| {
                        crate::i18n::t_fmt("cmd.spline.err.need_value", &[("what", what)])
                    })
            };
            match key.as_str() {
                "dp" => {
                    let v = need("dp")?;
                    let dp = v
                        .parse::<f64>()
                        .map_err(|e| crate::i18n::t_fmt("cmd.spline.err.dp_num", &[("v", v), ("e", &e.to_string())]))?;
                    if !(dp > 0.0) {
                        return Err(crate::i18n::t_fmt(
                            "cmd.spline.err.dp_parse_positive",
                            &[("dp", &dp.to_string())],
                        ));
                    }
                    spec.dp = Some(dp);
                }
                "root" | "齿根" | "齿根形式" => {
                    let v = need("root")?;
                    spec.root = Some(match v {
                        "平" | "平齿根" | "flat" | "FLAT" => RootForm::Flat,
                        "圆" | "圆齿根" | "fillet" | "FILLET" => RootForm::Fillet,
                        other => {
                            return Err(crate::i18n::t_fmt(
                                "cmd.spline.err.root_invalid",
                                &[("other", other)],
                            ))
                        }
                    });
                }
                "at" => {
                    let v = need("at")?;
                    let (x, y) = v.split_once(',').ok_or_else(|| {
                        crate::i18n::t_fmt("cmd.spline.err.at_format", &[("v", v)])
                    })?;
                    let x = x.trim().parse::<f64>().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.spline.err.at_x",
                            &[("x", x), ("e", &e.to_string())],
                        )
                    })?;
                    let y = y.trim().parse::<f64>().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.spline.err.at_y",
                            &[("y", y), ("e", &e.to_string())],
                        )
                    })?;
                    spec.at = Some([x, y]);
                }
                "rot" | "旋转" => {
                    let v = need("rot")?;
                    spec.rot = v
                        .parse::<f64>()
                        .map_err(|e: std::num::ParseFloatError| {
                            crate::i18n::t_fmt(
                                "cmd.spline.err.rot",
                                &[("v", v), ("e", &e.to_string())],
                            )
                        })?;
                }
                other => {
                    return Err(crate::i18n::t_fmt(
                        "cmd.spline.err.unknown_param",
                        &[("other", other), ("usage", &usage())],
                    ))
                }
            }
            i += 1;
        }
        Ok(spec)
    }

    /// 表达式反解 → `SplineInput`（齿根：显式 `root` > DA/DF 反解 > 按 αD 默认）。
    pub fn to_input(&self) -> Result<SplineInput, String> {
        let alpha = PressureAngle::parse(&format!("{}", self.gear.alpha_deg))?;
        let root = self
            .root
            .or_else(|| root_from_gear(&self.gear, self.side))
            .unwrap_or(match alpha {
                PressureAngle::A30 => RootForm::Flat,
                _ => RootForm::Fillet,
            });
        Ok(SplineInput {
            m: self.gear.m,
            z: self.gear.z,
            alpha,
            root,
            side: self.side,
            grade: self.grade,
            ext_dev: self.ext_dev,
            fit_length: None,
            dp: self.dp,
        })
    }
}

fn split_grade_fit(mark: &str) -> Result<(u32, &str), String> {
    let split = mark
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| crate::i18n::t_fmt("cmd.spline.err.mark_format", &[("mark", mark)]))?;
    let (g, f) = mark.split_at(split);
    let grade: u32 = g
        .parse()
        .map_err(|e: std::num::ParseIntError| {
            crate::i18n::t_fmt(
                "cmd.spline.err.grade_num",
                &[("g", g), ("e", &e.to_string())],
            )
        })?;
    if !matches!(grade, 4 | 5 | 6 | 7) {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.grade_invalid_card",
            &[("grade", &grade.to_string())],
        ));
    }
    if f.is_empty() {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.mark_no_fit",
            &[("mark", mark)],
        ));
    }
    Ok((grade, f))
}

fn usage() -> String {
    crate::i18n::t("cmd.spline.usage")
}

// ══════════════════════════════════════════════════════════════════════════
// 测试
// ══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spline_tol;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn sides() -> [SplineSide; 2] {
        [SplineSide::Internal, SplineSide::External]
    }

    /// 逐图元对模板：22 线 + 33 文字 + 21 ATTDEF，坐标/层/样式照模板。
    #[test]
    fn template_geometry_line_by_line() {
        // 模板框架（GB 内/外同构）。
        const W: f64 = 62.69374286414512;
        const H: f64 = 110.3795185976579;
        const ROW: f64 = 6.898720166384521;
        for side in sides() {
            let ents = block_entities(side);
            assert_eq!(ents.len(), 22 + 35 + 1 + 21, "{side:?} 图元数");
            // 4 条外框（1轮廓实线层）
            // 模板容差 1e-5（内/外两份模板的同名线有 ~3e-7 的手绘偏差）。
            let near5 = |a: f64, b: f64| (a - b).abs() < 1e-5;
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
            // 15 条正文横线（2细线层；模板 y = -ROW*k，k=1..15，容差 1e-5）
            for k in 1..=15 {
                let y = -ROW * k as f64;
                assert!(
                    ents.iter().any(|e| matches!(e, EntityType::Line(l)
                        if l.common.layer == "2细线层"
                            && (l.start.y - y).abs() < 1e-5
                            && (l.end.y - y).abs() < 1e-5
                            && (l.start.x + W).abs() < 1e-9
                            && l.end.x.abs() < 1e-9)),
                    "{side:?} 横线 k={k} y={y}"
                );
            }
            // 3 条竖线：名称列通高 + 值列两段
            assert!(ents.iter().any(|e| matches!(e, EntityType::Line(l)
                if l.common.layer == "2细线层"
                    && near5(l.start.x, -34.86505612988023)
                    && near5(l.start.y, -ROW)
                    && near5(l.end.y, -H))), "{side:?} 名称列竖线");
            assert!(ents.iter().any(|e| matches!(e, EntityType::Line(l)
                if l.common.layer == "2细线层"
                    && near5(l.start.x, -23.36718963751664)
                    && near5(l.start.y, -ROW)
                    && (l.end.y + 27.5948798526392).abs() < 1e-5)), "{side:?} 值列上段");
            assert!(ents.iter().any(|e| matches!(e, EntityType::Line(l)
                if l.common.layer == "2细线层"
                    && near5(l.start.x, -23.36718963751664)
                    && (l.start.y + 34.49359974805751).abs() < 1e-5
                    && near5(l.end.y, -H))), "{side:?} 值列下段");
            // 文字：全部 6文字层 + OCSM_GB
            let mut texts = 0usize;
            let mut mtexts = 0usize;
            for e in &ents {
                match e {
                    EntityType::Text(t) => {
                        assert_eq!(t.common.layer, "6文字层", "{side:?} 文字层");
                        assert_eq!(t.style, "OCSM_GB", "{side:?} 文字样式");
                        texts += 1;
                    }
                    EntityType::MText(m) => {
                        assert_eq!(m.common.layer, "6文字层", "{side:?} MTEXT 层");
                        assert_eq!(m.style, "OCSM_GB", "{side:?} MTEXT 样式");
                        mtexts += 1;
                    }
                    _ => {}
                }
            }
            assert_eq!((texts, mtexts), (35, 1), "{side:?} TEXT/MTEXT 数");
            // 21 个 ATTDEF（tag 集合 + 层/样式）
            let atts = attdefs(side);
            assert_eq!(atts.len(), 21, "{side:?} ATTDEF 数");
            let prefix = match side {
                SplineSide::Internal => "(内)",
                SplineSide::External => "(外)",
            };
            for ad in &atts {
                assert!(ad.tag.starts_with(prefix), "{side:?} tag {}", ad.tag);
                assert_eq!(ad.common.layer, "6文字层");
                assert_eq!(ad.text_style, "OCSM_GB");
                assert!(near(ad.insertion_point.y, -108.7795185976579 + 0.0)
                    || ad.insertion_point.y < 0.0);
            }
        }
    }

    /// 21 项取值：内/外各一例，与 `spline_tol` 同一份结果。
    #[test]
    fn values_match_compute_21() {
        let internal = SplineInput {
            m: 2.0,
            z: 20,
            alpha: PressureAngle::A30,
            root: RootForm::Fillet,
            side: SplineSide::Internal,
            grade: 6,
            ext_dev: ExtDev::H,
            fit_length: None,
            dp: None,
        };
        let t = spline_tol::compute(&internal).unwrap();
        let vals = values(SplineSide::Internal, &t).unwrap();
        assert_eq!(vals.len(), 21);
        let get = |tag: &str| vals.iter().find(|(t, _)| t == tag).unwrap().1.clone();
        assert_eq!(get("(内)齿形角"), "30°");
        assert_eq!(get("(内)齿数"), "20");
        assert_eq!(get("(内)模数"), "2");
        assert_eq!(get("(内)公差等级和配合类别"), "6H");
        assert_eq!(get("(内)大径"), "43.6");
        assert_eq!(get("(内)大径.下公差"), "0");
        let md = get("(内)测量跨棒距.下公差");
        assert!(md.starts_with('-'), "Md 下公差应为负：{md}");
        assert!(vals.iter().all(|(_, v)| !v.is_empty()), "不许留空");
        // Dp 不填 → 标准 R40 自动选 + 3 备选可见；填 → Md 按所填重算。
        let t_auto = match &t {
            SplineTable::Internal(x) => x,
            _ => unreachable!(),
        };
        assert!(t_auto.dp_candidates.len() == 3);
        assert!(t_auto.dp_candidates.iter().any(|p| (p - t_auto.dp).abs() < 1e-9));
        let mut with_dp = internal;
        with_dp.dp = Some(4.5);
        let t_dp = match spline_tol::compute(&with_dp).unwrap() {
            SplineTable::Internal(x) => x,
            _ => unreachable!(),
        };
        assert!((t_dp.dp - 4.5).abs() < 1e-9);
        assert!(
            (t_dp.md - t_auto.md).abs() > 1e-9,
            "填 Dp 后 Md 必须按所填 Dp 重算"
        );

        let external = SplineInput {
            m: 2.0,
            z: 20,
            alpha: PressureAngle::A30,
            root: RootForm::Fillet,
            side: SplineSide::External,
            grade: 5,
            ext_dev: ExtDev::F,
            fit_length: None,
            dp: None,
        };
        let t = spline_tol::compute(&external).unwrap();
        let vals = values(SplineSide::External, &t).unwrap();
        assert_eq!(vals.len(), 21);
        let get = |tag: &str| vals.iter().find(|(t, _)| t == tag).unwrap().1.clone();
        assert_eq!(get("(外)公差等级和配合类别"), "5f");
        assert_eq!(get("(外)大径.上公差"), "0", "Dee 上偏差脚注①=0");
        assert!(get("(外)跨测齿数").parse::<u32>().unwrap() >= 1);
        assert!(vals.iter().all(|(_, v)| !v.is_empty()));
        // Markdown 计算书节
        let md = markdown_table(SplineSide::External, &t).unwrap();
        assert!(md.contains("| (外)公法线长度 |") && md.lines().count() >= 23);
    }

    #[test]
    fn rimin_values_follow_table26_not_coefficient() {
        use crate::spline_tol::{rimin, rimin_table26, PressureAngle, RootForm, SplineInput, SplineSide};
        // 表 26 四档（30°平/30°圆/37.5°/45°）逐 m 对表；卡面值 = 表值，None →「—」。
        for (m, want) in [
            (0.25, [None, None, None, Some(0.06)]),
            (1.0, [Some(0.20), Some(0.40), Some(0.30), Some(0.25)]),
            (2.5, [Some(0.50), Some(1.00), Some(0.75), Some(0.62)]),
            (3.0, [Some(0.60), Some(1.20), Some(0.90), None]),
            (10.0, [Some(2.00), Some(4.00), Some(3.00), None]),
        ] {
            for (root, (idx, key)) in [
                (RootForm::Flat, (0usize, "(内)齿根圆最小曲率半径")),
                (RootForm::Fillet, (1usize, "(内)齿根圆最小曲率半径")),
            ] {
                let input = SplineInput {
                    m,
                    z: 20,
                    alpha: PressureAngle::A30,
                    root,
                    side: SplineSide::Internal,
                    grade: 6,
                    ext_dev: ExtDev::H,
                    fit_length: None,
                    dp: None,
                };
                let got = values(SplineSide::Internal, &spline_tol::compute(&input).unwrap())
                    .unwrap()
                    .into_iter()
                    .find(|(t, _)| t == key)
                    .unwrap()
                    .1;
                let table = rimin_table26(PressureAngle::A30, root, m).unwrap();
                assert_eq!(table, want[idx], "表 26 口径 m={m} root={root:?}");
                match want[idx] {
                    Some(v) => assert_eq!(got, format!("{v:.3}").trim_end_matches('0').trim_end_matches('.'), "卡面应取表值 m={m} root={root:?}"),
                    None => assert_eq!(got, "—", "表 26 未列值应显示「—」m={m} root={root:?}"),
                }
            }
            // 37.5° / 45°：只有圆齿根列进表（表 26 无平齿根分列）；用 30°圆 项位对齐同一表列。
            for (alpha, root, idx) in [
                (PressureAngle::A37_5, RootForm::Fillet, 2usize),
                (PressureAngle::A45, RootForm::Fillet, 3usize),
            ] {
                let input = SplineInput {
                    m,
                    z: 20,
                    alpha,
                    root,
                    side: SplineSide::Internal,
                    grade: 6,
                    ext_dev: ExtDev::H,
                    fit_length: None,
                    dp: None,
                };
                let got = values(SplineSide::Internal, &spline_tol::compute(&input).unwrap())
                    .unwrap()
                    .into_iter()
                    .find(|(t, _)| t == "(内)齿根圆最小曲率半径")
                    .unwrap()
                    .1;
                match want[idx] {
                    Some(v) => assert_eq!(got, format!("{v:.3}").trim_end_matches('0').trim_end_matches('.'), "卡面应取表值 m={m} α={alpha:?}"),
                    None => assert_eq!(got, "—", "表 26 未列值应显示「—」m={m} α={alpha:?}"),
                }
            }
        }
        // 系数式保留（内部/报告口径）：m=0.25 30°平 = 0.05，与卡面「—」并存。
        assert!((rimin(PressureAngle::A30, RootForm::Flat, 0.25) - 0.05).abs() < 1e-12);
    }

    /// 用户 2026-09-27 全仓长标签扫描：GB 模板全部标签（非标题/非符号）用 **TTF 实际宽**
    /// （cap-height 归一 ×1.57）复核：右缘不出标签列界（含两处实体级字宽修复：
    /// 内卡「齿根圆最小曲率半径」0.55→0.52、外卡「公法线长度 」0.95→0.90）。
    /// ③ 卡面批：**中英各跑一遍**（英文用 `en_label_width_factor` 的实体级字宽）。
    #[test]
    fn template_labels_fit_label_column_with_ttf_metrics() {
        let _g = crate::global_state_test_lock();
        for lang in [crate::i18n::Lang::Zh, crate::i18n::Lang::En] {
            crate::i18n::set_lang(lang);
            for side in [SplineSide::Internal, SplineSide::External] {
                let (_, texts, _, _) = side_data(side);
                for (key, x, y, h, wf, _layer) in texts {
                    // 只查标签列（x < −50）且跳过标题行（y > −ROW）。
                    if *x > -50.0 || *y > -6.898_720_166_384_379 {
                        continue;
                    }
                    let value = face_text(key);
                    let b = text_box_ttf([*x, *y], *h, face_width_factor(key, *wf), &value);
                    assert!(
                        b[2] <= -34.865_056_129_880_23 - 0.02,
                        "{lang:?} {side:?} 标签「{value}」越标签列：{b:?}"
                    );
                }
            }
        }
        crate::i18n::set_lang_auto();
    }

    /// ③ 卡面批：GB 花键卡卡面**文字随语言**，**符号/代号/ATTDEF tag/值原样**。
    #[test]
    fn card_face_switches_language_keeping_symbols_and_values() {
        let _g = crate::global_state_test_lock();
        let input = SplineInput {
            m: 2.0,
            z: 20,
            alpha: PressureAngle::A30,
            root: RootForm::Fillet,
            side: SplineSide::External,
            grade: 6,
            ext_dev: ExtDev::F,
            fit_length: None,
            dp: None,
        };
        let table = spline_tol::compute(&input).unwrap();
        let vals = values(SplineSide::External, &table).unwrap();
        let tags = |side| {
            attdefs(side)
                .iter()
                .map(|a| a.tag.clone())
                .collect::<Vec<_>>()
        };
        let tags_zh = tags(SplineSide::External);
        assert!(tags_zh.iter().all(|t| t.starts_with("(外)")));
        // 原始表里的**符号/代号**（非 `card.` key）→ 两语必须逐字相同
        let (_, raw_texts, raw_mtexts, _) = side_data(SplineSide::External);
        let mut want_symbols: Vec<String> = raw_texts
            .iter()
            .filter(|(k, ..)| !k.starts_with("card."))
            .map(|(k, ..)| (*k).to_string())
            .collect();
        want_symbols.extend(raw_mtexts.iter().map(|(v, ..)| (*v).to_string()));
        let block_texts = |side| {
            block_entities(side)
                .iter()
                .filter_map(|e| match e {
                    EntityType::Text(t) => Some(t.value.clone()),
                    EntityType::MText(m) => Some(m.value.clone()),
                    _ => None,
                })
                .collect::<Vec<String>>()
        };

        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        let zh = block_texts(SplineSide::External);
        for want in ["外 花 键 参 数 表", "公法线长度", "跨测齿数", "作用齿厚最大值"] {
            assert!(zh.iter().any(|t| t == want), "zh 卡面缺「{want}」：{zh:?}");
        }
        assert!(
            want_symbols.iter().all(|s| zh.contains(s)),
            "zh 卡面缺符号：{want_symbols:?} / {zh:?}"
        );

        crate::i18n::set_lang(crate::i18n::Lang::En);
        let en = block_texts(SplineSide::External);
        for want in [
            "External Spline Parameter Table",
            "Base tangent length",
            "Span teeth",
            "Max. effective tooth thickness",
            "Tolerance grade and fit",
        ] {
            assert!(en.iter().any(|t| t == want), "en 卡面缺「{want}」：{en:?}");
        }
        assert_eq!(en.len(), zh.len(), "中英卡面文字条数一致");
        assert!(
            !en
                .iter()
                .any(|t| t.chars().any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c))),
            "英文卡面不应再有汉字：{en:?}"
        );
        assert!(
            want_symbols.iter().all(|s| en.contains(s)),
            "英文卡面符号/代号必须原样：{want_symbols:?} / {en:?}"
        );
        assert_eq!(tags(SplineSide::External), tags_zh, "ATTDEF tag 不随语言");
        // 值（数据）两语一致
        assert_eq!(values(SplineSide::External, &table).unwrap(), vals);
        assert!(crate::i18n::missing_keys().is_empty(), "{:?}", crate::i18n::missing_keys());
        crate::i18n::set_lang_auto();
    }

    /// 命令解析 + dp/root/from/at/rot。
    #[test]
    fn spec_parse_expr_and_root_inference() {
        // 用户给的例子：内花键 DA = m(z+1.8) = 65.4 → 圆齿根反解
        let s = SplineTableSpec::parse(
            "std GB 内 6H SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30 \
             dp 1.0 at 10,20 rot 30",
        )
        .unwrap();
        assert_eq!(s.system, "gb3478");
        assert_eq!(s.side, SplineSide::Internal);
        assert_eq!(s.grade, 6);
        assert_eq!(s.ext_dev, ExtDev::H);
        assert!(near(s.gear.m, 3.0) && s.gear.z == 20);
        assert!(near(s.gear.alpha_deg, 30.0));
        assert!(near(s.gear.x, 0.0));
        assert_eq!(s.gear.da, Some(65.4));
        assert_eq!(s.gear.df, Some(57.3436));
        assert_eq!(s.dp, Some(1.0));
        assert_eq!(s.at, Some([10.0, 20.0]));
        assert!(near(s.rot, 30.0));
        let input = s.to_input().unwrap();
        assert_eq!(input.root, RootForm::Fillet, "DA = m(z+1.8) → 圆齿根反解");
        assert_eq!(input.dp, Some(1.0));
        // 显式 root 覆盖反解
        let s = SplineTableSpec::parse(
            "内 6H SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30 root 平",
        )
        .unwrap();
        assert_eq!(s.to_input().unwrap().root, RootForm::Flat);
        // 外花键：DF = m(z−1.5) = 55.5 → 平齿根反解；配合 js
        let s = SplineTableSpec::parse(
            "外 5js SPLINE EX M3 Z20 ALPHA30 X0 DA63 DF55.5 BETA0 H30",
        )
        .unwrap();
        assert_eq!(s.ext_dev, ExtDev::Js);
        let input = s.to_input().unwrap();
        assert_eq!(input.root, RootForm::Flat);
        assert_eq!(input.dp, None, "不填 dp = 自动 R40");
        // 37.5° 推不出平/圆 → 默认圆齿根
        let s = SplineTableSpec::parse(
            "外 5f SPLINE EX M2 Z20 ALPHA37.5 X0 DA41.8 DF37.2 BETA0 H30",
        )
        .unwrap();
        assert_eq!(s.to_input().unwrap().root, RootForm::Fillet);
        // 方向与表达式 KIND 不一致 → 报错
        let e = SplineTableSpec::parse(
            "内 6H SPLINE EX M3 Z20 ALPHA30 X0 DA63 DF54.6 BETA0 H30",
        )
        .unwrap_err();
        assert!(e.contains("KIND"), "{e}");
        // 内花键必须 H；等级非法报错
        assert!(SplineTableSpec::parse(
            "内 6f SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30"
        )
        .is_err());
        assert!(SplineTableSpec::parse(
            "内 8H SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30"
        )
        .is_err());
        // 20° 不是 GB/T 3478 压力角 → `to_input` 报错
        assert!(SplineTableSpec::parse(
            "内 6H GEAR IN M2 Z20 ALPHA20 X0 DA44 DF35 BETA0 H30"
        )
        .unwrap()
        .to_input()
        .is_err());
        // 缺表达式 / 矩形花键（无齿形关键字）/ 多段 / 不认识的体系 → 报错
        assert!(SplineTableSpec::parse("内 6H").is_err());
        assert!(SplineTableSpec::parse("内 6H SPLINE 6x23x26x6 L30").is_err());
        assert!(SplineTableSpec::parse(
            "内 6H SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30 | S30 E30 L20"
        )
        .is_err());
        assert!(SplineTableSpec::parse(
            "std ANSI 内 6H SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30"
        )
        .is_err());
        // 表达式尾巴上的未知 token 会被当成表达式的一部分 → 由 `parse_program` 报错。
        let e = SplineTableSpec::parse(
            "内 6H SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30 bogus",
        )
        .unwrap_err();
        assert!(e.contains("无法解析"), "{e}");
    }

    /// 用户 2026-09-26 修改 ③：上/下公差文字**整体右移一个字符宽**。
    ///
    /// 口径 = 实体级 x 偏移 = 字高 1.8 × 实体字宽 0.7 = 1.26（不动全局 `OCSM_GB`）；
    /// 只动公差 ATTDEF，其余标签/主值属性坐标一格不动。
    #[test]
    fn tolerance_texts_shift_right_one_char() {
        assert!((TOL_X_SHIFT - 1.8 * 0.7).abs() < 1e-12, "一个字符宽 = 1.26");
        for side in sides() {
            let (_, _, _, raw_atts) = side_data(side);
            let built = attdefs(side);
            assert_eq!(built.len(), raw_atts.len());
            for ((tag, x, _, _, _), ad) in raw_atts.iter().zip(built.iter()) {
                let want = *x + tolerance_shift(tag);
                assert!(
                    near(ad.insertion_point.x, want),
                    "{side:?} {tag}: x={}，应为 {want}",
                    ad.insertion_point.x
                );
                if tag.ends_with(".上公差") || tag.ends_with(".下公差") {
                    assert!(near(ad.insertion_point.x, x + TOL_X_SHIFT));
                } else {
                    assert!(near(ad.insertion_point.x, *x), "{side:?} {tag} 不应位移");
                }
            }
        }
    }
}
