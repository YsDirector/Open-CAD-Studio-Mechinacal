//! 智能卡片「GB 花键参数表·**精简版**」（`OCSMCARD`；用户 2026-09-27 点单）。
//!
//! 用户口径：**不留精度、只有基本信息**——卡面只列「执行标准 / 模数 / 齿数 / 齿形角 /
//! 齿根样式 / 大径 / 小径」+ 主要测量量（内：量棒 Dp、跨棒距 Md；外：跨测齿数 Kn、
//! 公法线 Wn），**不含任何公差列（不显示上/下偏差）**。
//!
//! ## 同源
//! 取值与 GB 花键卡**同一份** `spline_tol::compute()`：本模块只挑字段、不另算。
//! 表达式反解复用 `spline_table`/`card_expr` 的唯一解析器（`SplineTableModel::resolved`）。
//!
//! ## 版面（自定，无用户模板；风格照 GB 卡）
//! 沿用 GB 模板框架坐标与行高（`6.898720166384379`）、线层（`1轮廓实线层`/`2细线层`）、
//! 文字层与 `OCSM_GB` 样式、标签列/值列分栏；**把公差整列去掉**（值列直接到右框 x=0，
//! 不再放 x≈−9.5 的上/下偏差 ATTDEF）。10 行 = 1 标题 + 9 数据行；14 线 + 10 文字 + 9 属性。
//! 值列 ATTDEF 实体级字宽 0.7（与 GB 卡 [`crate::spline_table::VALUE_WIDTH_FACTOR`] 同口径）。
//!
//! ## 扩展点（表驱动）
//! 一卡一方向；以后扩到 NF/DIN/ANSI/齿轮 = 给该体系加一份 [`LiteFieldSpec`] 表 +
//! `CARD_TYPES` 一行 + `CardRenderer` 分派一个分支；GUI 统一骨架/计算书/表达式反解直接复用。

use crate::card::{CardFieldSpec, CardFormSpec};
use crate::spline_tol::{RootForm, SplineSide, SplineTable};
use ocs_plugin_api::host::acadrust::entities::{
    AttributeDefinition, AttributeEntity, Entity as _, EntityType, Insert, Text,
};
use ocs_plugin_api::host::acadrust::types::{Color, LineWeight, Vector3};

/// 内花键精简卡块名。
pub const BLOCK_INT: &str = "OCSM_SPLITE_GB_INT";
/// 外花键精简卡块名。
pub const BLOCK_EXT: &str = "OCSM_SPLITE_GB_EXT";

/// 按方向取块名。
pub fn block_name(side: SplineSide) -> &'static str {
    match side {
        SplineSide::Internal => BLOCK_INT,
        SplineSide::External => BLOCK_EXT,
    }
}

// ── 版面常量（坐标照 GB 模板；右框直接收在值列，公差列不存在）───────────────
/// 行高（GB 模板实测）。
pub(crate) const ROW: f64 = 6.898_720_166_384_379;
/// 数据行数（标题外）。
const DATA_ROWS: usize = 9;
/// 左框 x。
pub(crate) const X_LEFT: f64 = -62.693_742_864_145;
/// 标签/值分栏线 x。
pub(crate) const X_SEP: f64 = -34.865_056_129_880_27;
/// 右框 x（= 原值列右缘；公差列已整列去掉）。
pub(crate) const X_RIGHT: f64 = 0.0;
/// 标签插入 x（照模板）。
pub(crate) const X_LABEL: f64 = -61.709_962_722_953_35;
/// 值插入 x（分栏线右侧留 1.47）。
pub(crate) const X_VALUE: f64 = -33.4;
/// 字高（GB 模板同款）。
pub(crate) const TEXT_H: f64 = 3.6;
/// 标签字宽（模板同款）。
/// 标题实体级字宽（用户 2026-09-27 截图：标题超出表格右界）。
/// 宿主把 TTF 归一化到 cap height=9 再乘 height/9（`src/scene/text/ttf_glyph.rs`），
/// 本字体（朱雀仿宋）cap=0.637em → 实际字宽 ≈ 字号 × 1.57 × width_factor；0.75 下标题宽
/// 46.6 < 可用 51.29（外卡 51.42，余量 9%）。
pub(crate) const TITLE_WF: f64 = 0.75;

/// 长标签的实体级字宽（`None` = [`LABEL_WF`]）。
///
/// 用户 2026-09-27 截图三处（内卡标题/量棒直径 Dp/测量跨棒距 Md）+ 追加两处
/// （外卡跨测齿数 Kn/公法线长度 Wn）：按 TTF 实际 advance × cap-height 归一化预算，
/// 把文本宽压到标签列宽的 91–92%（≥8% 余量），不动列宽/线位/`OCSM_GB` 样式。
fn label_width_factor(tag: &str) -> f64 {
    match tag {
        "(简)量棒直径" | "(简)跨测齿数" => 0.80, // 5.4em → 24.5（列 26.8）
        "(简)测量跨棒距" | "(简)公法线长度" => 0.65, // 6.6–6.7em → 24.2–24.5
        _ => 1.0,
    }
}

/// 英文标签的**实体级字宽**（`None` = 用 [`label_width_factor`] 的 zh 值）。
///
/// 同一口径：真字体 metrics（cap=0.637em ⇒ 宽 = 字号 × 1.57 × wf），逐条按
/// 「宽 ≤ (X_SEP − X_LABEL) × 0.97（≥3% 余量）」预算；同时满足仓库保守模型
/// `text_extent_ttf()`（预算脚本：`~/桌面/OCSM/review/i18n_检查/card_gb_width_plan.py`）。
fn en_label_width_factor(tag: &str) -> Option<f64> {
    Some(match tag {
        "(简)执行标准" => 1.0,
        "(简)模数" => 1.0,
        "(简)齿数" => 0.61,
        "(简)齿形角" => 0.60,
        "(简)齿根样式" => 1.0,
        "(简)大径" => 0.55,
        "(简)小径" => 0.55,
        "(简)量棒直径" => 0.66,
        "(简)测量跨棒距" => 0.40,
        "(简)跨测齿数" => 0.79,
        "(简)公法线长度" => 0.46,
        _ => return None,
    })
}

/// 标签实体字宽：中文用 zh 值；英文用 [`en_label_width_factor`] 覆盖。
fn label_wf(tag: &str) -> f64 {
    match (crate::i18n::lang(), en_label_width_factor(tag)) {
        (crate::i18n::Lang::En, Some(wf)) => wf,
        _ => label_width_factor(tag),
    }
}

/// 标题实体字宽：中文 = [`TITLE_WF`]；英文按同口径预算（内 0.56 / 外 0.55）。
fn title_wf(side: SplineSide) -> f64 {
    match (crate::i18n::lang(), side) {
        (crate::i18n::Lang::En, SplineSide::Internal) => 0.56,
        (crate::i18n::Lang::En, SplineSide::External) => 0.55,
        _ => TITLE_WF,
    }
}
/// 值 ATTDEF 实体级字宽（与 GB 卡 0.7 同口径，不动全局 `OCSM_GB`）。
pub(crate) const VALUE_WF: f64 = 0.7;

/// 按字段指定值字宽的 ATTDEF（长值如 ANSI 英文「花键类型」需再压缩；不动其它行）。
pub(crate) fn attdef_wf(tag: &str, x: f64, y: f64, wf: f64) -> AttributeDefinition {
    let mut ad = attdef(tag, x, y);
    ad.width_factor = wf;
    ad
}
/// 标题插入 y（模板）。
pub(crate) const Y_TITLE: f64 = -5.200_708_803_331_338;

pub(crate) const LAYER_OUTLINE: &str = "1轮廓实线层";
pub(crate) const LAYER_THIN: &str = "2细线层";
pub(crate) const LAYER_TEXT: &str = "6文字层";

/// 一行数据（表驱动；tag 同时是 ATTDEF 键与取值映射键）。
#[derive(Debug, Clone, Copy)]
pub(crate) struct LiteFieldSpec {
    /// ATTDEF tag（`(简)` 前缀，避免与 GB 卡块混淆）。
    pub tag: &'static str,
    /// 卡面标签的 catalog key（**只译文字**；符号由 `symbol` 原样拼回）。
    pub label_key: &'static str,
    /// 标签后的**符号/代号**（原样透传，不译；空串 = 无）。
    pub symbol: &'static str,
    /// 单位（进 GUI 预览；卡面不加单位行）。
    pub unit: &'static str,
    /// 公式/口径 key（`gui.gb_lite.formula.*`；GUI 行 title、计算书）。
    pub formula_key: &'static str,
    /// 来源 key（`gui.gb_lite.source.*`）。
    pub source_key: &'static str,
}

impl LiteFieldSpec {
    /// 公式/口径（按当前语言）。
    pub fn formula(&self) -> String {
        if self.formula_key.is_empty() {
            return String::new();
        }
        crate::i18n::t(self.formula_key)
    }
    /// 来源（按当前语言）。
    pub fn source(&self) -> String {
        if self.source_key.is_empty() {
            return String::new();
        }
        crate::i18n::t(self.source_key)
    }
}

impl LiteFieldSpec {
    /// 卡面标签（随语言）：`模数 m` ⇄ `Module m`（符号/代号恒原样）。
    pub(crate) fn label(&self) -> String {
        let text = crate::i18n::t(self.label_key);
        if self.symbol.is_empty() {
            text
        } else {
            format!("{text} {}", self.symbol)
        }
    }
}

const INTERNAL_FIELDS: &[LiteFieldSpec] = &[
    LiteFieldSpec {
        tag: "(简)执行标准",
        label_key: "card.gb.lite.label.standard",
        symbol: "",
        unit: "",
        formula_key: "gui.gb_lite.formula.standard_expr",
        source_key: "gui.gb_lite.source.gb3478_1_2008",
    },
    LiteFieldSpec {
        tag: "(简)模数",
        label_key: "card.gb.lite.label.module",
        symbol: "m",
        unit: "mm",
        formula_key: "gui.gb_lite.formula.module",
        source_key: "gui.gb_lite.source.gb3478_1",
    },
    LiteFieldSpec {
        tag: "(简)齿数",
        label_key: "card.gb.lite.label.teeth",
        symbol: "z",
        unit: "",
        formula_key: "gui.gb_lite.formula.teeth",
        source_key: "gui.gb_lite.source.gb3478_1",
    },
    LiteFieldSpec {
        tag: "(简)齿形角",
        label_key: "card.gb.lite.label.alpha",
        symbol: "αD",
        unit: "°",
        formula_key: "gui.gb_lite.formula.alpha",
        source_key: "gui.gb_lite.source.gb3478_1_t3",
    },
    LiteFieldSpec {
        tag: "(简)齿根样式",
        label_key: "card.gb.lite.label.root_form",
        symbol: "",
        unit: "",
        formula_key: "gui.gb_lite.formula.root_form",
        source_key: "gui.gb_lite.source.gb3478_1_s5",
    },
    LiteFieldSpec {
        tag: "(简)大径",
        label_key: "card.gb.lite.label.major_dia",
        symbol: "Dei",
        unit: "mm",
        formula_key: "gui.gb_lite.formula.major_dia_int",
        source_key: "gui.gb_lite.source.gb3478_1_t3",
    },
    LiteFieldSpec {
        tag: "(简)小径",
        label_key: "card.gb.lite.label.minor_dia",
        symbol: "Dii",
        unit: "mm",
        formula_key: "gui.gb_lite.formula.minor_dia_int",
        source_key: "gui.gb_lite.source.gb3478_1_t3_note",
    },
    LiteFieldSpec {
        tag: "(简)量棒直径",
        label_key: "card.gb.label.pin_dia",
        symbol: "Dp",
        unit: "mm",
        formula_key: "gui.gb_lite.formula.pin_dia",
        source_key: "gui.gb_lite.source.gb3478_6_eq1",
    },
    LiteFieldSpec {
        tag: "(简)测量跨棒距",
        label_key: "card.gb.label.over_pins",
        symbol: "Md",
        unit: "mm",
        formula_key: "gui.gb_lite.formula.over_pins",
        source_key: "gui.gb_lite.source.gb3478_6_eq2_5",
    },
];

const EXTERNAL_FIELDS: &[LiteFieldSpec] = &[
    LiteFieldSpec {
        tag: "(简)执行标准",
        label_key: "card.gb.lite.label.standard",
        symbol: "",
        unit: "",
        formula_key: "gui.gb_lite.formula.standard_expr",
        source_key: "gui.gb_lite.source.gb3478_1_2008",
    },
    LiteFieldSpec {
        tag: "(简)模数",
        label_key: "card.gb.lite.label.module",
        symbol: "m",
        unit: "mm",
        formula_key: "gui.gb_lite.formula.module",
        source_key: "gui.gb_lite.source.gb3478_1",
    },
    LiteFieldSpec {
        tag: "(简)齿数",
        label_key: "card.gb.lite.label.teeth",
        symbol: "z",
        unit: "",
        formula_key: "gui.gb_lite.formula.teeth",
        source_key: "gui.gb_lite.source.gb3478_1",
    },
    LiteFieldSpec {
        tag: "(简)齿形角",
        label_key: "card.gb.lite.label.alpha",
        symbol: "αD",
        unit: "°",
        formula_key: "gui.gb_lite.formula.alpha",
        source_key: "gui.gb_lite.source.gb3478_1_t3",
    },
    LiteFieldSpec {
        tag: "(简)齿根样式",
        label_key: "card.gb.lite.label.root_form",
        symbol: "",
        unit: "",
        formula_key: "gui.gb_lite.formula.root_form",
        source_key: "gui.gb_lite.source.gb3478_1_s5",
    },
    LiteFieldSpec {
        tag: "(简)大径",
        label_key: "card.gb.lite.label.major_dia",
        symbol: "Dee",
        unit: "mm",
        formula_key: "gui.gb_lite.formula.major_dia_ext",
        source_key: "gui.gb_lite.source.gb3478_1_t3",
    },
    LiteFieldSpec {
        tag: "(简)小径",
        label_key: "card.gb.lite.label.minor_dia",
        symbol: "Die",
        unit: "mm",
        formula_key: "gui.gb_lite.formula.minor_dia_ext",
        source_key: "gui.gb_lite.source.gb3478_1_t3",
    },
    LiteFieldSpec {
        tag: "(简)跨测齿数",
        label_key: "card.gb.label.span_teeth",
        symbol: "Kn",
        unit: "",
        formula_key: "gui.gb_lite.formula.span_teeth",
        source_key: "gui.gb_lite.source.gb3478_6_eq11_note",
    },
    LiteFieldSpec {
        tag: "(简)公法线长度",
        label_key: "card.gb.label.base_tangent",
        symbol: "Wn",
        unit: "mm",
        formula_key: "gui.gb_lite.formula.base_tangent",
        source_key: "gui.gb_lite.source.gb3478_6_eq11_12",
    },
];

/// 本方向的字段表（一卡一方向；扩体系 = 加表 + 分派）。
pub(crate) fn fields(side: SplineSide) -> &'static [LiteFieldSpec] {
    match side {
        SplineSide::Internal => INTERNAL_FIELDS,
        SplineSide::External => EXTERNAL_FIELDS,
    }
}

/// 精简内卡表单（统一骨架；字段全走 `card_types[].form` 下发，页面不写卡专属 HTML）。
pub const FORM_INT: CardFormSpec = CardFormSpec {
    fields: &[
        CardFieldSpec {
            key: "expr",
            label: "gui.fld.spline_lite.form_int.expr.label",
            kind: "textarea",
            placeholder: "SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30",
            default: "SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30",
            title: "gui.fld.spline_lite.form_int.expr.title",
            options: &[],
            options_from: "",
            min: 0.0,
            step: 0.0,
            required: true,
        },
        CardFieldSpec {
            key: "grade",
            label: "gui.fld.spline_lite.form_int.grade.label",
            kind: "number",
            placeholder: "6",
            default: "6",
            title: "gui.fld.spline_lite.form_int.grade.title",
            options: &[],
            options_from: "",
            min: 4.0,
            step: 1.0,
            required: true,
        },
        CardFieldSpec {
            key: "fit",
            label: "gui.fld.spline_lite.form_int.fit.label",
            kind: "select",
            placeholder: "",
            default: "H",
            title: "gui.fld.spline_lite.form_int.fit.title",
            options: &[("H", "H")],
            options_from: "",
            min: 0.0,
            step: 0.0,
            required: true,
        },
        CardFieldSpec {
            key: "root",
            label: "gui.fld.spline_lite.form_int.root.label",
            kind: "select",
            placeholder: "",
            default: "auto",
            title: "gui.fld.spline_lite.form_int.root.title",
            options: &[("auto", "gui.fld.spline_lite.form_int.root.opt.auto"), ("flat", "gui.fld.spline_lite.form_int.root.opt.flat"), ("fillet", "gui.fld.spline_lite.form_int.root.opt.fillet")],
            options_from: "",
            min: 0.0,
            step: 0.0,
            required: false,
        },
        CardFieldSpec {
            key: "dp",
            label: "gui.fld.spline_lite.form_int.dp.label",
            kind: "number",
            placeholder: "gui.fld.spline_lite.form_int.dp.placeholder",
            default: "",
            title: "gui.fld.spline_lite.form_int.dp.title",
            options: &[],
            options_from: "",
            min: 0.0,
            step: 0.01,
            required: false,
        },
    ],
    note_key: "gui.form.gb_lite_int.note",
    missing_note_key: "gui.form.gb_lite_int.missing_note",
};

/// 精简外卡表单。
pub const FORM_EXT: CardFormSpec = CardFormSpec {
    fields: &[
        CardFieldSpec {
            key: "expr",
            label: "gui.fld.spline_lite.form_ext.expr.label",
            kind: "textarea",
            placeholder: "SPLINE EX M3 Z20 ALPHA30 X0 DA66 DF52.5 BETA0 H30",
            default: "SPLINE EX M2 Z20 ALPHA30 X0 DA42 DF37 BETA0 H30",
            title: "gui.fld.spline_lite.form_ext.expr.title",
            options: &[],
            options_from: "",
            min: 0.0,
            step: 0.0,
            required: true,
        },
        CardFieldSpec {
            key: "grade",
            label: "gui.fld.spline_lite.form_ext.grade.label",
            kind: "number",
            placeholder: "6",
            default: "6",
            title: "gui.fld.spline_lite.form_ext.grade.title",
            options: &[],
            options_from: "",
            min: 4.0,
            step: 1.0,
            required: true,
        },
        CardFieldSpec {
            key: "fit",
            label: "gui.fld.spline_lite.form_ext.fit.label",
            kind: "select",
            placeholder: "",
            default: "h",
            title: "gui.fld.spline_lite.form_ext.fit.title",
            options: &[
                ("h", "h"),
                ("js", "js"),
                ("k", "k"),
                ("d", "d"),
                ("e", "e"),
                ("f", "f"),
            ],
            options_from: "",
            min: 0.0,
            step: 0.0,
            required: true,
        },
        CardFieldSpec {
            key: "root",
            label: "gui.fld.spline_lite.form_ext.root.label",
            kind: "select",
            placeholder: "",
            default: "auto",
            title: "gui.fld.spline_lite.form_ext.root.title",
            options: &[("auto", "gui.fld.spline_lite.form_ext.root.opt.auto"), ("flat", "gui.fld.spline_lite.form_ext.root.opt.flat"), ("fillet", "gui.fld.spline_lite.form_ext.root.opt.fillet")],
            options_from: "",
            min: 0.0,
            step: 0.0,
            required: false,
        },
    ],
    note_key: "gui.form.gb_lite_ext.note",
    missing_note_key: "gui.form.gb_lite_ext.missing_note",
};

// ── 块几何（照 GB 模板风格 + 去掉公差整列）───────────────────────────────

pub(crate) fn text_ent(value: &str, x: f64, y: f64, h: f64, wf: f64) -> EntityType {
    let mut t = Text::with_value(value, Vector3::new(x, y, 0.0));
    t.height = h;
    t.width_factor = wf;
    t.style = "OCSM_GB".into();
    t.common.layer = LAYER_TEXT.to_string();
    t.common.color = Color::ByLayer;
    t.common.linetype = "ByLayer".into();
    EntityType::Text(t)
}

pub(crate) fn attdef(tag: &str, x: f64, y: f64) -> AttributeDefinition {
    let mut ad = AttributeDefinition::new(tag.to_string(), String::new(), " ".to_string());
    ad.insertion_point = Vector3::new(x, y, 0.0);
    ad.alignment_point = ad.insertion_point;
    ad.height = TEXT_H;
    ad.width_factor = VALUE_WF;
    ad.text_style = "OCSM_GB".into();
    ad.flags.preset = true;
    ad.common.layer = LAYER_TEXT.to_string();
    ad
}

/// 数据行 i（1 起）顶部 y。
pub(crate) fn row_top(i: usize) -> f64 {
    -(i as f64) * ROW
}

/// 标签基线 y（照模板 row1 = −12.74995）。
pub(crate) fn label_y(i: usize) -> f64 {
    row_top(i) - 5.852_141_401_252_116
}

/// 值基线 y（照模板 row1 = −12.19744）。
pub(crate) fn value_y(i: usize) -> f64 {
    label_y(i) + 0.552_51
}

/// 标题（内/外）catalog key。
fn title_key(side: SplineSide) -> &'static str {
    match side {
        SplineSide::Internal => "card.gb.title.int_lite",
        SplineSide::External => "card.gb.title.ext_lite",
    }
}

/// 卡面标题（随语言）：`内 花 键 参 数 表（精简）` ⇄ `Internal Spline Parameter Table (Lite)`。
pub(crate) fn title_text(side: SplineSide) -> String {
    crate::i18n::t(title_key(side))
}

fn title_x(side: SplineSide) -> f64 {
    match side {
        SplineSide::Internal => -51.288_982_774_652_794,
        SplineSide::External => -51.420_189_287_103_995,
    }
}

/// 精简卡块成员：左框/右框/上下框 + 9 行分隔 + 1 分栏线 = 14 线；
/// 标题 + 9 标签 = 10 文字；9 个 ATTDEF。**无公差列图元/文字/属性**。
pub fn block_entities(side: SplineSide) -> Vec<EntityType> {
    // 标题行 + DATA_ROWS 数据行 = 10 行；底部 = −10·ROW。
    let bottom = -((DATA_ROWS as f64) + 1.0) * ROW;
    let mut out = Vec::with_capacity(14 + 10 + DATA_ROWS);
    let mut line = |a: [f64; 2], b: [f64; 2], layer: &str| out.push(crate::partgen_kit::line(a, b, layer));
    line([X_LEFT, 0.0], [X_LEFT, bottom], LAYER_OUTLINE);
    line([X_RIGHT, 0.0], [X_RIGHT, bottom], LAYER_OUTLINE);
    line([X_LEFT, 0.0], [X_RIGHT, 0.0], LAYER_OUTLINE);
    line([X_LEFT, bottom], [X_RIGHT, bottom], LAYER_OUTLINE);
    for i in 1..=DATA_ROWS {
        line([X_LEFT, row_top(i)], [X_RIGHT, row_top(i)], LAYER_THIN);
    }
    line([X_SEP, -ROW], [X_SEP, bottom], LAYER_THIN);
    out.push(text_ent(
        &title_text(side),
        title_x(side),
        Y_TITLE,
        TEXT_H,
        title_wf(side),
    ));
    for (i, f) in fields(side).iter().enumerate() {
        let row = i + 1;
        out.push(text_ent(
            &f.label(),
            X_LABEL,
            label_y(row),
            TEXT_H,
            label_wf(f.tag),
        ));
    }
    for (i, f) in fields(side).iter().enumerate() {
        out.push(EntityType::AttributeDefinition(attdef(f.tag, X_VALUE, value_y(i + 1))));
    }
    out
}

/// 9 个 ATTDEF（顺序 = `fields`）。
pub fn attdefs(side: SplineSide) -> Vec<AttributeDefinition> {
    fields(side)
        .iter()
        .enumerate()
        .map(|(i, f)| attdef(f.tag, X_VALUE, value_y(i + 1)))
        .collect()
}

// ── 取值（与 GB 卡同一份 `spline_tol::compute()`；本模块不另算）────────────

fn root_label(root: RootForm) -> String {
    crate::i18n::t(match root {
        RootForm::Flat => "card.gb.root.flat",
        RootForm::Fillet => "card.gb.root.fillet",
    })
}

/// 9 个 tag 的值（顺序与 `attdefs()` 一致）。只读 `SplineTable` 既有字段。
pub fn values(side: SplineSide, table: &SplineTable) -> Result<Vec<(String, String)>, String> {
    let s = crate::spline_table::fmt_mm;
    let vals: Vec<(&'static str, String)> = match (side, table) {
        (SplineSide::Internal, SplineTable::Internal(t)) => vec![
            ("(简)执行标准", "GB/T 3478.1-2008".to_string()),
            ("(简)模数", s(t.m)),
            ("(简)齿数", t.z.to_string()),
            ("(简)齿形角", crate::spline_table::fmt_deg(t.alpha_deg)),
            ("(简)齿根样式", root_label(t.root)),
            ("(简)大径", s(t.major_dia)),
            ("(简)小径", s(t.minor_dia)),
            ("(简)量棒直径", s(t.dp)),
            ("(简)测量跨棒距", s(t.md)),
        ],
        (SplineSide::External, SplineTable::External(t)) => vec![
            ("(简)执行标准", "GB/T 3478.1-2008".to_string()),
            ("(简)模数", s(t.m)),
            ("(简)齿数", t.z.to_string()),
            ("(简)齿形角", crate::spline_table::fmt_deg(t.alpha_deg)),
            ("(简)齿根样式", root_label(t.root)),
            ("(简)大径", s(t.major_dia)),
            ("(简)小径", s(t.minor_dia)),
            ("(简)跨测齿数", t.kn.to_string()),
            ("(简)公法线长度", s(t.wn)),
        ],
        _ => return Err(crate::i18n::t("cmd.splinelite.err.side_mismatch")),
    };
    // 顺序护栏：值顺序必须与字段表（ATTDEF 顺序）一致。
    for (f, (tag, _)) in fields(side).iter().zip(&vals) {
        debug_assert_eq!(f.tag, *tag, "精简卡字段表与取值顺序不一致");
    }
    Ok(vals
        .into_iter()
        .map(|(tag, v)| (tag.to_string(), v))
        .collect())
}

/// 预览 9 项（label/单位/公式/来源来自字段表，值同一份 `values()`）。
pub fn items_json(side: SplineSide, table: &SplineTable) -> Result<Vec<serde_json::Value>, String> {
    let vals = values(side, table)?;
    let mut out = Vec::with_capacity(vals.len());
    for (f, (tag, value)) in fields(side).iter().zip(vals) {
        debug_assert_eq!(f.tag, tag);
        out.push(serde_json::json!({
            "tag": tag,
            "label": f.label(),
            "unit": f.unit,
            "value": value,
            "formula": f.formula(),
            "source": f.source(),
        }));
    }
    Ok(out)
}

/// 求值用的 `INSERT`（与 GB 卡同一条构造路径：几何一次、值一次）。
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
        tmpl.rotation = 0.0;
        let mut attr = AttributeEntity::from_definition(&tmpl, Some(val));
        attr.apply_transform(&ins.get_transform());
        ins.attributes.push(attr);
    }
    Ok(ins)
}

/// 待放置件要带的 9 个 ATTRIB。
pub fn pending_attrs(
    side: SplineSide,
    table: &SplineTable,
) -> Result<Vec<(AttributeDefinition, String)>, String> {
    let values = values(side, table)?;
    let mut out = Vec::with_capacity(values.len());
    for ad in attdefs(side) {
        let v = values
            .iter()
            .find(|(tag, _)| tag == &ad.tag)
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        out.push((ad, v));
    }
    Ok(out)
}

/// GUI 预览 JSON（与 `/api/card_preview` 同形；表达式反解复用统一解析器）。
pub fn preview_json(model: &crate::spline_gui::SplineTableModel) -> Result<serde_json::Value, String> {
    let (input, gear) = model.resolved()?;
    let table = crate::spline_tol::compute(&input)?;
    let items = items_json(input.side, &table)?;
    let side = input.side;
    let (card, side_label) = match side {
        SplineSide::Internal => ("GB花键精简表_内", "内花键"),
        SplineSide::External => ("GB花键精简表_外", "外花键"),
    };
    let expr_echo = model
        .expr
        .trim()
        .to_string();
    let root = crate::spline_gui::root_id(input.root);
    let readout = serde_json::json!([
        {"k": crate::i18n::t("gui.gb_lite.readout.module"), "v": crate::spline_table::fmt_mm(input.m)},
        {"k": crate::i18n::t("gui.gb_lite.readout.teeth"), "v": input.z.to_string()},
        {"k": crate::i18n::t("gui.gb_lite.readout.alpha"), "v": crate::spline_table::fmt_deg(input.alpha.deg())},
        {"k": crate::i18n::t("gui.gb_lite.readout.root_form"), "v": root_label(input.root)},
        {"k": crate::i18n::t("gui.gb_lite.readout.side"), "v": side_label},
    ]);
    let fit_code = match side {
        SplineSide::Internal => "H",
        SplineSide::External => input.ext_dev.code(),
    };
    let fields = serde_json::json!({
        "grade": input.grade,
        "fit": fit_code,
        "root": root,
    });
    Ok(serde_json::json!({
        "ok": true,
        "card": card,
        "renderer": "spline_lite",
        "title": format!("GB/T 3478 {side_label}参数表（精简版）"),
        "expr": expr_echo,
        "fields": fields,
        "x": gear.x,
        "side": crate::spline_gui::side_id(side),
        "side_label": side_label,
        "readout": readout,
        "items": items,
        "missing": [],
        "missing_note": crate::i18n::t("gui.lite.options.missing_note"),
    }))
}

/// 待放置件的 `OCSM_PART` 元数据（薄台账；卡类型/方向/等级配合/Dp）。
pub fn part_meta_json(
    side: SplineSide,
    input: &crate::spline_tol::SplineInput,
    dp: Option<f64>,
) -> String {
    serde_json::json!({
        "family": "spline_lite",
        "card": match side {
            SplineSide::Internal => "GB花键精简表_内",
            SplineSide::External => "GB花键精简表_外",
        },
        "side": crate::spline_gui::side_id(side),
        "grade_fit": input.grade_fit_label(),
        "dp": dp,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spline_tol::{ExtDev, PressureAngle, SplineInput};

    fn make_input(
        side: SplineSide,
        m: f64,
        z: u32,
        alpha: PressureAngle,
        root: RootForm,
    ) -> SplineInput {
        SplineInput {
            m,
            z,
            alpha,
            root,
            side,
            grade: 6,
            ext_dev: if side == SplineSide::Internal {
                ExtDev::H
            } else {
                ExtDev::H
            },
            fit_length: None,
            dp: None,
        }
    }

    fn text_values(ents: &[EntityType]) -> Vec<String> {
        ents.iter()
            .filter_map(|e| match e {
                EntityType::Text(t) => Some(t.value.clone()),
                EntityType::MText(m) => Some(m.value.clone()),
                _ => None,
            })
            .collect()
    }

    /// 负断言 + 图元数：9 行基本项，**卡面无任何公差类字段/文字**。
    #[test]
    fn lite_block_has_no_tolerance_column() {
        let _g = crate::global_state_test_lock();
        for lang in [crate::i18n::Lang::Zh, crate::i18n::Lang::En] {
            crate::i18n::set_lang(lang);
            for side in [SplineSide::Internal, SplineSide::External] {
                let ents = block_entities(side);
                let lines = ents
                    .iter()
                    .filter(|e| matches!(e, EntityType::Line(_)))
                    .count();
                let texts = text_values(&ents);
                let atts = attdefs(side);
                assert_eq!(lines, 14, "{lang:?} {side:?} 线数（4 框 + 9 分隔 + 1 分栏）");
                assert_eq!(texts.len(), 10, "{lang:?} {side:?} 文字数（标题 + 9 标签）");
                assert_eq!(atts.len(), DATA_ROWS, "{lang:?} {side:?} ATTDEF 数");
                // 块底部必须到第 10 行（标题 + 9 数据行），否则最后一行会出框。
                let min_y = ents
                    .iter()
                    .filter_map(|e| match e {
                        EntityType::Line(l) => Some(l.start.y.min(l.end.y)),
                        _ => None,
                    })
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    (min_y + ((DATA_ROWS as f64) + 1.0) * ROW).abs() < 1e-9,
                    "{lang:?} {side:?} 块底应在 −10·ROW（含标题行），实 {min_y}"
                );
                // 负断言：卡面/标签不得出现公差类字样（**两语都查**：中文 + 英文）。
                for bad in [
                    "公差", "偏差", "上差", "下差", "Toleran", "Deviat", "deviat", "toleran",
                ] {
                    assert!(
                        !texts.iter().any(|t| t.contains(bad)),
                        "{lang:?} {side:?} 卡面文字不应出现「{bad}」：{texts:?}"
                    );
                    assert!(
                        !atts.iter().any(|a| a.tag.contains(bad)),
                        "{lang:?} {side:?} ATTDEF tag 不应出现「{bad}」"
                    );
                }
            }
        }
        crate::i18n::set_lang_auto();
    }

    /// 与既有 GB 卡同一批 `compute()` 值：精简项逐项等于 GB 卡对应项。
    #[test]
    fn lite_values_equal_gb_card_items_same_compute() {
        let _g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh); // 本用例断言中文卡面值（齿根样式「平齿根」）
        for (side, input) in [
            (
                SplineSide::Internal,
                make_input(SplineSide::Internal, 3.0, 20, PressureAngle::A30, RootForm::Flat),
            ),
            (
                SplineSide::External,
                make_input(SplineSide::External, 3.0, 20, PressureAngle::A30, RootForm::Flat),
            ),
        ] {
            let table = crate::spline_tol::compute(&input).unwrap();
            let gb: std::collections::HashMap<String, String> =
                crate::spline_table::values(side, &table).unwrap().into_iter().collect();
            let lite: std::collections::HashMap<String, String> =
                values(side, &table).unwrap().into_iter().collect();
            let pairs: &[(&str, &str)] = &[
                ("(简)模数", "模数"),
                ("(简)齿数", "齿数"),
                ("(简)齿形角", "齿形角"),
                ("(简)大径", "大径"),
                ("(简)小径", "小径"),
            ];
            for (lite_tag, gb_key) in pairs {
                let gb_tag = format!(
                    "({}){gb_key}",
                    if side == SplineSide::Internal { "内" } else { "外" }
                );
                assert_eq!(
                    lite[*lite_tag], gb[gb_tag.as_str()],
                    "{side:?} 精简 {lite_tag} 应等于 GB 卡 {gb_tag}"
                );
            }
            match side {
                SplineSide::Internal => {
                    assert_eq!(lite["(简)量棒直径"], gb["(内)量棒直径"]);
                    assert_eq!(lite["(简)测量跨棒距"], gb["(内)测量跨棒距"]);
                }
                SplineSide::External => {
                    assert_eq!(lite["(简)跨测齿数"], gb["(外)跨测齿数"]);
                    assert_eq!(lite["(简)公法线长度"], gb["(外)公法线长度"]);
                }
            }
            assert_eq!(lite["(简)执行标准"], "GB/T 3478.1-2008");
            assert_eq!(lite["(简)齿根样式"], "平齿根");
        }
        crate::i18n::set_lang_auto();
    }

    /// 几何（**TTF 实际宽**，宿主 cap-height 归一 ×1.57）：标题/标签/值均在各自单元格内
    /// （标签列留 ≥3% 边距）、标签与值及任意两文本不相交、全体不出块框。
    /// 用户 2026-09-27 截图三处（标题/量棒直径 Dp/测量跨棒距 Md）+ 外卡两处（跨测齿数 Kn/公法线长度 Wn）。
    /// ③ 卡面批：**中英各跑一遍**（英文用 `en_label_width_factor` / `title_wf` 的实体级字宽）。
    #[test]
    fn lite_texts_stay_in_columns_and_do_not_overlap() {
        let _g = crate::global_state_test_lock();
        use crate::spline_table::text_box_ttf;
        let lite_cell = X_SEP - X_LABEL; // 标签列可用宽
        for lang in [crate::i18n::Lang::Zh, crate::i18n::Lang::En] {
            crate::i18n::set_lang(lang);
            for side in [SplineSide::Internal, SplineSide::External] {
                let fields = fields(side);
                // 取一组大值（m=10、z=100）压最宽文本。
                let input = make_input(side, 10.0, 100, PressureAngle::A30, RootForm::Flat);
                let table = crate::spline_tol::compute(&input).unwrap();
                let vals = values(side, &table).unwrap();
                // 标题：不出左框/右框。
                let title_s = title_text(side);
                let title = text_box_ttf(
                    [title_x(side), Y_TITLE],
                    TEXT_H,
                    title_wf(side),
                    &title_s,
                );
                assert!(
                    title[0] >= X_LEFT && title[2] <= X_RIGHT - 0.02,
                    "{lang:?} {side:?} 标题「{title_s}」越表：{title:?}"
                );
                let mut boxes: Vec<(String, [f64; 4])> =
                    vec![(format!("title:{title_s}"), title)];
                for (i, f) in fields.iter().enumerate() {
                    let row = i + 1;
                    let label_s = f.label();
                    let lab = text_box_ttf(
                        [X_LABEL, label_y(row)],
                        TEXT_H,
                        label_wf(f.tag),
                        &label_s,
                    );
                    assert!(
                        lab[0] >= X_LEFT && lab[2] <= X_SEP - 0.02,
                        "{lang:?} {side:?} 标签「{label_s}」越标签列：{lab:?}"
                    );
                    let margin = (X_SEP - lab[2]) / lite_cell;
                    assert!(
                        margin >= 0.03,
                        "{lang:?} {side:?} 标签「{label_s}」列内余量 {:.1}% < 3%：{lab:?}",
                        margin * 100.0
                    );
                    let val = &vals[i].1;
                    let vb = text_box_ttf([X_VALUE, value_y(row)], TEXT_H, VALUE_WF, val);
                    assert!(
                        vb[0] >= X_SEP && vb[2] <= X_RIGHT - 0.02,
                        "{lang:?} {side:?} 值「{val}」越值列：{vb:?}"
                    );
                    assert!(
                        lab[2] < vb[0],
                        "{lang:?} {side:?} 标签「{label_s}」与值「{val}」横向相叠：{lab:?} vs {vb:?}"
                    );
                    let top = row_top(row);
                    let bottom = top - ROW;
                    assert!(
                        lab[1] >= bottom && lab[3] <= top && vb[1] >= bottom && vb[3] <= top,
                        "{lang:?} {side:?} 第 {row} 行文字应在本行带内"
                    );
                    boxes.push((format!("label:{label_s}"), lab));
                    boxes.push((format!("value:{val}"), vb));
                }
                // 任意两文本盒不相交（含标题/跨行）。
                for i in 0..boxes.len() {
                    for j in (i + 1)..boxes.len() {
                        let (a, b) = (&boxes[i].1, &boxes[j].1);
                        let hit = a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3];
                        assert!(
                            !hit,
                            "{lang:?} {side:?} 文本框相交：{} × {}",
                            boxes[i].0, boxes[j].0
                        );
                    }
                }
            }
        }
        crate::i18n::set_lang_auto();
    }

    /// ③ 卡面批：精简卡卡面**文字随语言**、**符号/代号/ATTDEF tag/值原样**。
    #[test]
    fn lite_card_face_switches_language_keeping_symbols_and_values() {
        let _g = crate::global_state_test_lock();
        let input = make_input(SplineSide::External, 2.0, 20, PressureAngle::A30, RootForm::Fillet);
        let table = crate::spline_tol::compute(&input).unwrap();
        let vals = values(SplineSide::External, &table).unwrap();
        let tag_list = |side| {
            attdefs(side)
                .iter()
                .map(|a| a.tag.clone())
                .collect::<Vec<_>>()
        };
        let texts = |side| {
            block_entities(side)
                .iter()
                .filter_map(|e| match e {
                    EntityType::Text(t) => Some(t.value.clone()),
                    _ => None,
                })
                .collect::<Vec<String>>()
        };

        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        let zh = texts(SplineSide::External);
        let tags_zh = tag_list(SplineSide::External);
        assert!(tags_zh.iter().all(|t| t.starts_with("(简)")));
        for want in [
            "外 花 键 参 数 表（精简）",
            "模数 m",
            "齿根样式",
            "跨测齿数 Kn",
            "公法线长度 Wn",
        ] {
            assert!(zh.iter().any(|t| t == want), "zh 卡面缺「{want}」：{zh:?}");
        }
        assert_eq!(vals[4].1, "圆齿根", "卡面值（齿根样式）随语言");

        crate::i18n::set_lang(crate::i18n::Lang::En);
        let en = texts(SplineSide::External);
        for want in [
            "External Spline Parameter Table (Lite)",
            "Module m",
            "Root form",
            "Span teeth Kn",
            "Base tangent length Wn",
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
        assert_eq!(tag_list(SplineSide::External), tags_zh, "ATTDEF tag 不随语言");
        // 数据值（标准号/数字/符号）两语一致；只有「齿根样式」术语随语言。
        let vals_en = values(SplineSide::External, &table).unwrap();
        assert!(vals_en.iter().any(|(t, v)| t == "(简)执行标准" && v == "GB/T 3478.1-2008"));
        assert!(vals_en.iter().any(|(t, v)| t == "(简)公法线长度" && v == &vals[8].1));
        assert_eq!(vals_en[4].1, "Fillet root", "英文卡面值用英文术语");
        assert!(!vals_en.is_empty());
        assert!(crate::i18n::missing_keys().is_empty(), "{:?}", crate::i18n::missing_keys());
        crate::i18n::set_lang_auto();
    }

    /// 表达式反解 + 预览 JSON：9 项、回填 fields（root/grade/fit）、无公差项。
    #[test]
    fn lite_preview_reverse_solves_expression() {
        let model = crate::spline_gui::SplineTableModel {
            side: "int".into(),
            grade: 6,
            fit: "H".into(),
            expr: "SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30".into(),
            root: Some("auto".into()),
            dp: None,
            at: None,
            rot: 0.0,
        };
        let v = preview_json(&model).unwrap();
        assert_eq!(v["renderer"], "spline_lite");
        assert_eq!(v["card"], "GB花键精简表_内");
        let items = v["items"].as_array().unwrap();
        assert_eq!(items.len(), DATA_ROWS);
        let get = |label: &str| {
            items
                .iter()
                .find(|it| it["label"] == label)
                .map(|it| it["value"].as_str().unwrap().to_string())
                .unwrap()
        };
        assert_eq!(get("模数 m"), "3");
        assert_eq!(get("齿数 z"), "20");
        assert_eq!(get("齿形角 αD"), "30°");
        assert_eq!(v["fields"]["root"], "fillet", "DA65.4 → 圆齿根反解");
        assert_eq!(v["fields"]["grade"], 6);
        assert_eq!(v["fields"]["fit"], "H");
        assert!(!items.iter().any(|it| {
            let t = it["tag"].as_str().unwrap();
            let l = it["label"].as_str().unwrap();
            t.contains("公差") || l.contains("公差") || t.contains("偏差")
        }));
    }
}

