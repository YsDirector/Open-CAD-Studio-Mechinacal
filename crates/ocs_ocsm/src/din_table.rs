//! 智能卡片「DIN 花键参数表」（`OCSMCARD`；`CardRenderer::DinTable`）：
//! **DIN 5480-1:2006-03 §9.1 Datenfeld / Bild 6 的 13 行 × 2 栏版面**（Nabe 左 / Welle 右），
//! 几何内建（不依赖外部 DXF；风格照既有 `nf_table.rs`：外框/分隔线/`OCSM_GB`/字高比例）。
//!
//! # 为什么是 13 行而不是 GB 的 21 项
//!
//! 调研（`~/桌面/OCSM/review/DIN花键参数表_调研.md`）已纠偏：DIN 的「参数表」= Datenfeld，
//! Bild 6 每栏 13 行：`标记 / z / m / α / 三个直径 / e-s 三极限 / D_M / 两个 M 极限`；
//! **d_B、d、d_b、x·m、W_k 不在 Bild 6 里**（那属 DIN 5480-2 名义表），本卡不塞。
//!
//! # 数据口径（逐格可追溯；表外不外推）
//!
//! | 字段组 | 来源 |
//! |---|---|
//! | 直径六项 + `e₂=s₁` | `assets/din5480_2_nominal.csv`（(m, d_B, z) 精确命中；查不到 → 「—」） |
//! | `Ae`/`As` | `assets/din5480_1_table7_dev.csv`（2026-09-26 OCR，双源 600dpi + 逐格复核；见该表头与 `din5480_1_table7_notes.md`） |
//! | `Tact`/`Teff` | `assets/din5480_1_table7_tol.csv`（只抽到 6–9 级、模数组 1,75–4 的实锚；其余 → 「—」，可用 `tactn/teffn/tactw/teffw` 覆盖） |
//! | `D_M`/`M2`/`M1` | `assets/din5480_2_inspection.csv`（(m,d_B,z) 命中时）；Bild 6 示例走 `assets/din5480_1_bild6_anchor.csv` |
//! | e/s 六值 | DIN 5480-1 §10.8 表 6 公式（`emax=e2+Ae+Tact+Teff` 等，见 `derive()`） |
//!
//! **Bild 6 示例**（N120×3×38×9H / W120×3×38×8f）：本卡逐行复刻标准原印值（锚点 CSV），
//! 同时用公式路重算并断言一致（测试 `bild6_example_reproduces_standard`）。
//!
//! # 明示缺口（不臆造）
//!
//! * Table 7 上段：**c1/c2（>400 侧）与 c9（≤12 细档）列映射无实锚** → 这两侧查表返回 `—`；
//!   c3=200–400 / c4=100–200 / c5=50–100 / c6=25–50 / c7=12–25 / c8=≤12 中 c4/c5 为实锚（Bild 6、KISSsoft 样张），
//!   其余按表头阶梯「推断」（来源注明）。
//! * Table 7 下段（Tact/Teff/Fp/fp/Fα/Fβ 全表体）未逐格抽取 → 只入 6–9 级实锚，10/11 级显示「—」。
//! * 直径偏差 `A_df1` 用名义表口径（切削）；Bild 6 示例的冷轧值 `−1,74` 只在锚点示例里出现。

use ocs_plugin_api::host::acadrust::entities::mtext::AttachmentPoint;
use ocs_plugin_api::host::acadrust::entities::{
    AttributeDefinition, AttributeEntity, Entity as _, EntityCommon, EntityType,
    HorizontalAlignment, Insert, MText, VerticalAlignment,
};
use ocs_plugin_api::host::acadrust::types::{Color, LineWeight, Vector3};

/// 表格块名（内外两张单栏卡；旧的两栏合表已拆，用户 2026-09-26）。
pub fn block_name(hub: bool) -> &'static str {
    if hub {
        "OCSM_DINTABLE_DIN_INT"
    } else {
        "OCSM_DINTABLE_DIN_EXT"
    }
}
/// 旧合表块名（仅兼容旧引用；新插入不再建它）。
pub const BLOCK: &str = "OCSM_DINTABLE_DIN";
/// 缺项标记（无表值/无锚/缺口一律显示它；预览与回执点明原因 —— 不臆造）。
pub const MISSING: &str = "—";
/// 单栏宽：标签列 240 + 值列 180（用户截图后加宽：标题/长标签/长标记都不出框、不压线）。
pub const W_LABEL: f64 = 240.0;
/// 单栏总宽。
pub const W_SIDE: f64 = 420.0;
/// 整表宽（单栏 = 单侧）。
pub const W_TOTAL: f64 = W_SIDE;
/// 标题行高。
pub const H_TITLE: f64 = 44.0;
/// 标记（代号）行高。
pub const H_DESIG: f64 = 60.0;
/// 数据行高（12 行）。
pub const H_ROW: f64 = 40.0;
/// 表高（= 44 + 60 + 12×40）。
pub const H_TOTAL: f64 = H_TITLE + H_DESIG + 12.0 * H_ROW;
/// 标签/值字高（照 NF 卡 25）。
pub const TEXT_H: f64 = 25.0;
/// 标题字高。
pub const TITLE_H: f64 = 30.0;
/// 标签**实体级**字宽（长标签不出标签列：240 可用宽，最长「齿根成形圆 d_Ff2」≈165）。
pub const LABEL_W_FACTOR: f64 = 0.75;
/// 值/标记**实体级**字宽（长标记「Nabe DIN 5480 – N120×3×38×9H」≈257 < 408）。
pub const VALUE_W_FACTOR: f64 = 0.6;
/// 整表缩放（用户 2026-09-26 截图）：DIN 卡 **INSERT 缩放 0.17**，
/// **不动块内图元几何**；ATTRIB 文字随 INSERT 缩放。
pub const TABLE_SCALE: f64 = 0.17;

/// 数据行 12 行的行中心 y（自上而下）。
pub fn data_row_y(i: usize) -> f64 {
    -(H_TITLE + H_DESIG) - (i as f64 + 0.5) * H_ROW
}
/// 标记行中心 y。
pub const DESIG_Y: f64 = -(H_TITLE + H_DESIG / 2.0);
/// 标题中心 y。
pub const TITLE_Y: f64 = -H_TITLE / 2.0;

/// 标签插入 x（单栏）。
pub const LABEL_X: f64 = 6.0;
/// 值插入 x（单栏）。
pub const VALUE_X: f64 = W_LABEL + 9.0;
/// 旧两栏坐标（兼容引用；新几何不再用）。
pub const LABEL_X_HUB: f64 = 6.0;
/// 旧两栏坐标（兼容引用）。
pub const LABEL_X_SHAFT: f64 = W_SIDE + 6.0;
/// 旧两栏坐标（兼容引用）。
pub const VALUE_X_HUB: f64 = W_LABEL + 9.0;
/// 旧两栏坐标（兼容引用）。
pub const VALUE_X_SHAFT: f64 = W_SIDE + W_LABEL + 9.0;

// ══════════════════════════════════════════════════════════════════════════
// Table 7 上段（Abmaße）资产
// ══════════════════════════════════════════════════════════════════════════

/// Table 7 上段（偏差表）一行：18 个 Abmaß 系列 × 9 个直径档列（µm）。
#[derive(Debug, Clone, PartialEq)]
pub struct DevRow {
    /// 外花键（Welle）偏差系列字母（v,u,t,s,r,p,n,m,k,js,h,g,f,e,d,c,b,a）。
    pub series: String,
    /// 对应的内花键（Nabe）系列字母（k↔F、js↔G、h↔H、g↔J、f↔K、e↔M；其余空）。
    pub hub_letter: String,
    /// 9 列数值（µm；`None` = 该格 OCR 留空、待核）。
    pub values: [Option<i32>; 9],
    /// 9 列范围标签与置信（`c1:>400_细档:待核|...`）。
    pub range_conf: String,
    /// 逐格 OCR/目视标记（可追溯，运行时不展示）。
    pub flags: String,
}

/// Table 7 上段资产（`include_str!`）。
const TABLE7_DEV_CSV: &str = include_str!("../assets/din5480_1_table7_dev.csv");

/// Table 7 下段（公差）锚点行。
#[derive(Debug, Clone, PartialEq)]
pub struct TolRow {
    /// 公差等级（6/7/8/9；10/11 为待抽空行）。
    pub grade: u32,
    /// 模数组（本资产只有 `1,75–4`）。
    pub module_group: String,
    /// 锚点直径说明。
    pub d_b_anchor: String,
    /// `Tact`（mm；待抽 → `None`）。
    pub tact: Option<f64>,
    /// `Teff`（mm）。
    pub teff: Option<f64>,
    /// `TG = Tact + Teff`。
    pub tg: Option<f64>,
    /// 来源。
    pub source: String,
    /// 备注。
    pub note: String,
}

const TABLE7_TOL_CSV: &str = include_str!("../assets/din5480_1_table7_tol.csv");
/// Bild 6 示例锚点（26 字段照录）。
const BILD6_ANCHOR_CSV: &str = include_str!("../assets/din5480_1_bild6_anchor.csv");

/// 逐行拆 CSV（本仓资产用引号包住含逗号字段；无转义引号）。
fn split_csv_line(line: &str) -> Vec<String> {
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

/// 十进制字段（CSV 用逗号作小数点；空格/减号统一）。
fn csv_decimal(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', ".").replace('−', "-");
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

static TABLE7_DEV: std::sync::OnceLock<Vec<DevRow>> = std::sync::OnceLock::new();
static TABLE7_TOL: std::sync::OnceLock<Vec<TolRow>> = std::sync::OnceLock::new();
static BILD6_ANCHOR: std::sync::OnceLock<Vec<[String; 4]>> = std::sync::OnceLock::new();

/// Table 7 上段（懒加载；坏数据 panic = 构建错误）。
pub fn table7_rows() -> &'static [DevRow] {
    TABLE7_DEV.get_or_init(|| parse_dev_csv(TABLE7_DEV_CSV).expect("DIN 5480-1 Table 7 偏差表损坏"))
}

/// Table 7 下段锚点（懒加载）。
pub fn table7_tol_rows() -> &'static [TolRow] {
    TABLE7_TOL.get_or_init(|| parse_tol_csv(TABLE7_TOL_CSV).expect("DIN 5480-1 Table 7 公差锚损坏"))
}

/// Bild 6 锚点（`[side, field, value, source]`）。
pub fn bild6_anchor() -> &'static [[String; 4]] {
    BILD6_ANCHOR.get_or_init(|| {
        let mut out = Vec::new();
        for line in BILD6_ANCHOR_CSV.lines() {
            if line.trim().is_empty() || line.starts_with('#') || line.starts_with("side,") {
                continue;
            }
            let f = split_csv_line(line);
            if f.len() >= 4 {
                out.push([
                    f[0].trim().to_string(),
                    f[1].trim().to_string(),
                    f[2].trim().to_string(),
                    f[3].trim().to_string(),
                ]);
            }
        }
        out
    })
}

/// 锚点取值（side = `hub`/`shaft`/`meta`）。
pub fn anchor_value(side: &str, field: &str) -> Option<&'static str> {
    bild6_anchor()
        .iter()
        .find(|r| r[0] == side && r[1] == field)
        .map(|r| r[2].as_str())
}

/// 锚点数值（逗号小数/减号归一）。
pub fn anchor_f64(side: &str, field: &str) -> Option<f64> {
    anchor_value(side, field).and_then(csv_decimal)
}

/// 锚点显示串：把德文排版的 `−` 与小数逗号归一为图纸卡内一致的点号写法。
pub fn anchor_display(side: &str, field: &str) -> Option<String> {
    anchor_value(side, field).map(normalize_de)
}

/// `120 +0,76` → `120 +0.76`；`113,4 −1,74` → `113.4 -1.74`；其余原样。
fn normalize_de(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    for (i, ch) in chars.iter().enumerate() {
        match ch {
            '−' => out.push('-'),
            ',' => {
                let prev_digit = i > 0 && chars[i - 1].is_ascii_digit();
                let next_digit = chars.get(i + 1).is_some_and(|c| c.is_ascii_digit());
                out.push(if prev_digit && next_digit { '.' } else { ',' });
            }
            _ => out.push(*ch),
        }
    }
    out
}

fn parse_dev_csv(text: &str) -> Result<Vec<DevRow>, String> {
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if line.trim().is_empty() || line.starts_with('#') || line.starts_with("series,") {
            continue;
        }
        let f = split_csv_line(line);
        if f.len() < 14 {
            return Err(format!("Table7 偏差表第 {n} 行只有 {} 列（应有 14 列)", f.len()));
        }
        let series = f[0].trim().to_string();
        if series.is_empty() {
            return Err(format!("Table7 偏差表第 {n} 行 series 为空"));
        }
        let mut values = [None; 9];
        for (k, slot) in values.iter_mut().enumerate() {
            let cell = f[2 + k].trim();
            if !cell.is_empty() {
                *slot = Some(
                    cell.parse::<i32>()
                        .map_err(|_| format!("Table7 第 {n} 行 c{}「{cell}」非法", k + 1))?,
                );
            }
        }
        rows.push(DevRow {
            series,
            hub_letter: f[1].trim().to_string(),
            values,
            range_conf: f[11].trim().to_string(),
            flags: f[12].trim().to_string(),
        });
    }
    if rows.is_empty() {
        return Err("Table7 偏差表为空".to_string());
    }
    Ok(rows)
}

fn parse_tol_csv(text: &str) -> Result<Vec<TolRow>, String> {
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if line.trim().is_empty() || line.starts_with('#') || line.starts_with("grade,") {
            continue;
        }
        let f = split_csv_line(line);
        if f.len() < 8 {
            return Err(format!("Table7 公差锚第 {n} 行只有 {} 列（应有 8 列)", f.len()));
        }
        let grade = f[0]
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("Table7 公差锚第 {n} 行 grade「{}」非法", f[0]))?;
        let opt = |k: usize| -> Result<Option<f64>, String> {
            let s = f[k].trim();
            if s.is_empty() || s == "—" {
                Ok(None)
            } else {
                csv_decimal(s).map(Some).ok_or_else(|| {
                    format!("Table7 公差锚第 {n} 行列 {}「{s}」非法", k + 1)
                })
            }
        };
        rows.push(TolRow {
            grade,
            module_group: f[1].trim().to_string(),
            d_b_anchor: f[2].trim().to_string(),
            tact: opt(3)?,
            teff: opt(4)?,
            tg: opt(5)?,
            source: f[6].trim().to_string(),
            note: f[7].trim().to_string(),
        });
    }
    if rows.is_empty() {
        return Err("Table7 公差锚为空".to_string());
    }
    Ok(rows)
}

// ── 直径档列映射（Table 7 上段列 c1..c9）────────────────────────────────────

/// 直径档：`(列序 0..9, 范围标签, 置信)`；`None` = 该侧无实锚，运行期不取。
pub fn range_for_d_b(d_b: f64) -> Option<(usize, &'static str, &'static str)> {
    if !(d_b.is_finite() && d_b > 0.0) {
        return None;
    }
    if d_b > 400.0 {
        None // c1/c2 待核：>400 侧列映射无实锚
    } else if d_b > 200.0 {
        Some((2, "200–400", "推断"))
    } else if d_b > 100.0 {
        Some((3, "100–200", "已核(锚)"))
    } else if d_b > 50.0 {
        Some((4, "50–100", "已核(锚)"))
    } else if d_b > 25.0 {
        Some((5, "25–50", "推断"))
    } else if d_b > 12.0 {
        Some((6, "12–25", "推断"))
    } else {
        Some((7, "≤12", "推断"))
    }
}

/// 按侧取 Table 7 偏差行：内花键按 hub_letter（F..M），外花键按 series（v..a）。
pub fn dev_lookup(letter: &str, hub: bool) -> Option<&'static DevRow> {
    let want = letter.trim();
    table7_rows().iter().find(|r| {
        if hub {
            r.hub_letter.eq_ignore_ascii_case(want)
        } else {
            r.series.eq_ignore_ascii_case(want)
        }
    })
}

/// `letter` 在直径 `d_b` 处的 Table 7 偏差（mm）。
///
/// 返回 `(值 mm, 说明)`；说明写清档位、置信、表值（µm）与缺口原因。
pub fn dev_lookup_mm(letter: &str, d_b: f64, hub: bool) -> Result<(f64, String), String> {
    let Some(row) = dev_lookup(letter, hub) else {
        return Err(format!(
            "Table 7 没有{}偏差系列「{letter}」（{}）",
            if hub { "孔 " } else { "轴 " },
            if hub {
                "孔 F/G/H/J/K/M"
            } else {
                "轴 v/u/t/s/r/p/n/m/k/js/h/g/f/e/d/c/b/a"
            }
        ));
    };
    let Some((ci, label, conf)) = range_for_d_b(d_b) else {
        return Err(format!(
            "Table 7 的 d_B={} 落在 >400 侧（列 c1/c2），该侧列映射无实锚 —— 显示「—」；\
             可用 ae=… / as=… 显式覆盖",
            crate::partgen_kit::trim(d_b)
        ));
    };
    let Some(um) = row.values[ci] else {
        return Err(format!(
            "Table 7 系列「{letter}」在档 {label}（列 c{}）OCR 留空待核 —— 显示「—」",
            ci + 1
        ));
    };
    let cell_flag = row
        .flags
        .contains(&format!("c{}:待核", ci + 1));
    Ok((
        um as f64 / 1000.0,
        format!(
            "Table 7 {letter} 行 × {label}（列 c{}，置信：{conf}）：{um} µm{}{}",
            ci + 1,
            if conf == "已核(锚)" {
                ""
            } else {
                "（范围按表头阶梯推断）"
            },
            if cell_flag { "（该格 OCR 标待核）" } else { "" }
        ),
    ))
}

/// 公差等级 `grade` 在模数 `m` 处的 `(Tact, Teff)`（mm）与说明。
pub fn tol_lookup_mm(grade: u32, m: f64) -> Result<(f64, f64, f64, String), String> {
    let in_group = (1.75..=4.0).contains(&m);
    let Some(row) = table7_tol_rows().iter().find(|r| r.grade == grade) else {
        return Err(format!("Table 7 公差锚没有等级 {grade}"));
    };
    if !in_group {
        return Err(format!(
            "Table 7 公差表只抽到模数组 1,75–4 的锚点（m={} 不在内）—— 显示「—」；\
             可用 tactn/teffn/tactw/teffw 显式覆盖",
            crate::partgen_kit::trim(m)
        ));
    }
    match (row.tact, row.teff, row.tg) {
        (Some(tact), Some(teff), Some(tg)) => Ok((
            tact,
            teff,
            tg,
            format!("Table 7 公差 {grade} 级：Tact={tact}，Teff={teff}，TG={tg}（{}）", row.source),
        )),
        _ => Err(format!(
            "Table 7 公差 {grade} 级未抽取（{}）—— 显示「—」；可用 tactn/teffn/tactw/teffw 显式覆盖",
            row.source
        )),
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 表结构：13 行 × 2 侧（照 Bild 6 字段清单与栏序）
// ══════════════════════════════════════════════════════════════════════════

/// 一行字段的静态定义：`(内花键 key, 内符号, 内 tag, 外花键 key, 外符号, 外 tag, 取值口径, 来源)`。
///
/// ★ 标签文字**不进结构体**（卡面随语言）：`key` 是 catalog 键（`card.din.label.*`），`symbol`
/// （`d_f2`/`M2_max`/…两语原样）与译名分置（§31/§32「符号与译名分置」形态）。
#[derive(Debug, Clone, Copy)]
pub struct RowSpec {
    /// 内花键（Nabe）行标签 catalog key。
    pub hub_key: &'static str,
    /// 内花键符号（两语原样；空串 = 该行无符号，如行 1）。
    pub hub_symbol: &'static str,
    /// 内花键 ATTDEF tag。
    pub hub_tag: &'static str,
    /// 外花键（Welle）行标签 catalog key。
    pub shaft_key: &'static str,
    /// 外花键符号（两语原样；空串 = 无符号）。
    pub shaft_symbol: &'static str,
    /// 外花键 ATTDEF tag。
    pub shaft_tag: &'static str,
    /// 公式/口径 key（`gui.din.formula.*`；预览 title，随语言）。
    pub formula_key: &'static str,
    /// 来源 key（`gui.din.source.*`；预览 title，随语言）。
    pub source_key: &'static str,
}

impl RowSpec {
    /// 公式/口径（按当前语言）。
    pub fn formula(&self) -> String {
        crate::i18n::t(self.formula_key)
    }
    /// 来源（按当前语言）。
    pub fn source(&self) -> String {
        crate::i18n::t(self.source_key)
    }
}

/// 13 行（行序 = Bild 6；行 1 = 标记）。
pub const ROWS: &[RowSpec] = &[
    RowSpec { hub_key: "card.din.label.nabe", hub_symbol: "", hub_tag: "N标记",
              shaft_key: "card.din.label.welle", shaft_symbol: "", shaft_tag: "W标记",
              formula_key: "gui.din.formula.designation", source_key: "gui.din.source.clause8" },
    RowSpec { hub_key: "card.din.label.teeth", hub_symbol: "z", hub_tag: "N齿数",
              shaft_key: "card.din.label.teeth", shaft_symbol: "z", shaft_tag: "W齿数",
              formula_key: "gui.din.formula.input", source_key: "gui.din.source.bild6_row2" },
    RowSpec { hub_key: "card.din.label.module", hub_symbol: "m", hub_tag: "N模数",
              shaft_key: "card.din.label.module", shaft_symbol: "m", shaft_tag: "W模数",
              formula_key: "gui.din.formula.input", source_key: "gui.din.source.bild6_row3" },
    RowSpec { hub_key: "card.din.label.alpha", hub_symbol: "α", hub_tag: "N压力角",
              shaft_key: "card.din.label.alpha", shaft_symbol: "α", shaft_tag: "W压力角",
              formula_key: "gui.din.formula.alpha", source_key: "gui.din.source.clause5_row4" },
    RowSpec { hub_key: "card.din.label.root_dia", hub_symbol: "d_f2", hub_tag: "N齿根圆",
              shaft_key: "card.din.label.tip_dia", shaft_symbol: "d_a1", shaft_tag: "W齿顶圆",
              formula_key: "gui.din.formula.root_dia", source_key: "gui.din.source.nominal_table5" },
    RowSpec { hub_key: "card.din.label.root_form_dia", hub_symbol: "d_Ff2", hub_tag: "N齿根成形圆",
              shaft_key: "card.din.label.root_form_dia", shaft_symbol: "d_Ff1", shaft_tag: "W齿根成形圆",
              formula_key: "gui.din.formula.root_form_dia", source_key: "gui.din.source.nominal_table" },
    RowSpec { hub_key: "card.din.label.tip_dia", hub_symbol: "d_a2", hub_tag: "N齿顶圆",
              shaft_key: "card.din.label.root_dia", shaft_symbol: "d_f1", shaft_tag: "W齿根圆",
              formula_key: "gui.din.formula.tip_dia", source_key: "gui.din.source.nominal_table5" },
    RowSpec { hub_key: "card.din.label.space_width_max", hub_symbol: "e_max", hub_tag: "N槽宽max",
              shaft_key: "card.din.label.thickness_eff_max", shaft_symbol: "s_vmax", shaft_tag: "W齿厚svmax",
              formula_key: "gui.din.formula.space_width_max", source_key: "gui.din.source.clause10_8" },
    RowSpec { hub_key: "card.din.label.space_width_min", hub_symbol: "e_min", hub_tag: "N槽宽min",
              shaft_key: "card.din.label.thickness_max", shaft_symbol: "s_max", shaft_tag: "W齿厚smax",
              formula_key: "gui.din.formula.space_width_min", source_key: "gui.din.source.clause10_8" },
    RowSpec { hub_key: "card.din.label.space_width_eff", hub_symbol: "e_vmin", hub_tag: "N槽宽eff",
              shaft_key: "card.din.label.thickness_min", shaft_symbol: "s_min", shaft_tag: "W齿厚smin",
              formula_key: "gui.din.formula.space_width_eff", source_key: "gui.din.source.clause10_8" },
    RowSpec { hub_key: "card.din.label.measuring_circle", hub_symbol: "D_M", hub_tag: "N量圆",
              shaft_key: "card.din.label.measuring_circle", shaft_symbol: "D_M", shaft_tag: "W量圆",
              formula_key: "gui.din.formula.measuring_circle", source_key: "gui.din.source.insp_table" },
    RowSpec { hub_key: "card.din.label.over_pins", hub_symbol: "M2_max", hub_tag: "N量距max",
              shaft_key: "card.din.label.over_pins", shaft_symbol: "M1_max", shaft_tag: "W量距max",
              formula_key: "gui.din.formula.over_pins_max", source_key: "gui.din.source.insp_table_bild6" },
    RowSpec { hub_key: "card.din.label.over_pins", hub_symbol: "M2_min", hub_tag: "N量距min",
              shaft_key: "card.din.label.over_pins", shaft_symbol: "M1_min", shaft_tag: "W量距min",
              formula_key: "gui.din.formula.over_pins_min", source_key: "gui.din.source.insp_table_bild6" },
];

/// 行标签（按当前语言）：`t(key)` + 「空格 + 符号」（符号两语原样；行 1 无符号）。
/// 卡面与 GUI 元数据（`options_json`/`preview_json`）**同源**。
pub fn row_label(r: &RowSpec, hub: bool) -> String {
    let (key, symbol) = if hub {
        (r.hub_key, r.hub_symbol)
    } else {
        (r.shaft_key, r.shaft_symbol)
    };
    crate::nf_table::face_label(key, symbol)
}

/// 13 个 ATTDEF tag（单栏：标记 + 12 值；与 [`ROWS`] 同序）。
pub fn value_tags(hub: bool) -> Vec<&'static str> {
    ROWS.iter()
        .map(|r| if hub { r.hub_tag } else { r.shaft_tag })
        .collect()
}

/// 单栏标题（按当前语言）。
pub fn title_text(hub: bool) -> String {
    crate::i18n::t(if hub {
        "card.din.title.int"
    } else {
        "card.din.title.ext"
    })
}

fn common_of(layer: &str) -> EntityCommon {
    let mut c = EntityCommon::default();
    c.layer = layer.to_string();
    c.color = Color::ByLayer;
    c.linetype = "ByLayer".into();
    c
}

/// 标签/标题 MTEXT（左中；标题用正中），字高 [`TEXT_H`] / 标题 [`TITLE_H`]，样式一律 `OCSM_GB`。
fn mtext_ent(value: &str, x: f64, y: f64, height: f64, attach: i16) -> EntityType {
    let mut m = MText::new();
    m.value = value.to_string();
    m.insertion_point = Vector3::new(x, y, 0.0);
    m.height = height;
    m.rectangle_width = W_SIDE - 2.0 * LABEL_X;
    m.style = "OCSM_GB".into();
    m.attachment_point = match attach {
        5 => AttachmentPoint::MiddleCenter,
        _ => AttachmentPoint::MiddleLeft,
    };
    m.common = common_of("6文字层");
    EntityType::MText(m)
}

/// 标签/标题 MTEXT 的**内联字宽因子表**（`\W<f>;`；有效字宽 = 样式字宽 0.7 × f）。
///
/// `None` = 不压缩。数字来源：`i18n_检查/card_din_width_plan.py`
/// （真字体 advance × cap 0.637 归一 / 仓库 `char_em` / 仓库 `char_em_ttf` **三模型取更紧者**，
/// 再留 ≥3% 列内余量）；标签列可用 = `W_LABEL − LABEL_X − 0.5` = 233.5，标题整宽可用 407。
/// 中文侧 13 行全部装得下（模板自定版面，与前批 NF 卡的「
/// `基准尺寸 Do` 本就偏宽」不同），英文侧 6 条标签 + 两条标题需压缩。
/// 压缩只发生在装不下的那一条上；不动列宽/线位/字高/样式。
pub(crate) fn inline_wf(key: &str, lang: crate::i18n::Lang) -> Option<f64> {
    use crate::i18n::Lang;
    let table: &[(&str, f64, f64)] = &[
        // (key, zh 的 W, en 的 W)；1.0 = 不压缩
        ("card.din.title.int", 1.0, 0.93),
        ("card.din.title.ext", 1.0, 0.91),
        ("card.din.label.root_form_dia", 1.0, 0.75),
        ("card.din.label.space_width_max", 1.0, 0.79),
        ("card.din.label.space_width_min", 1.0, 0.80),
        ("card.din.label.space_width_eff", 1.0, 0.81),
        ("card.din.label.thickness_eff_max", 1.0, 0.69),
        ("card.din.label.thickness_max", 1.0, 0.69),
        ("card.din.label.thickness_min", 1.0, 0.70),
        ("card.din.label.measuring_circle", 1.0, 0.89),
        ("card.din.label.over_pins", 1.0, 0.59),
    ];
    let (_, zh, en) = table.iter().find(|(k, _, _)| *k == key)?;
    let w = if lang == Lang::En { *en } else { *zh };
    (w < 0.995).then_some(w)
}

/// 带内联字宽码的 MTEXT：`\W0.75;Root form diameter d_Ff2`（有效字宽 = 0.7 × wf）。
fn mtext_with_inline_wf(
    text: &str,
    wf: f64,
    x: f64,
    y: f64,
    height: f64,
    attach: i16,
) -> EntityType {
    mtext_ent(&format!("\\W{wf:.2};{text}"), x, y, height, attach)
}

/// 标签 MTEXT（按当前语言取词 + 必要时内联压缩）。
fn label_mtext(key: &str, symbol: &str, x: f64, y: f64) -> EntityType {
    let text = crate::nf_table::face_label(key, symbol);
    match inline_wf(key, crate::i18n::lang()) {
        Some(wf) => mtext_with_inline_wf(&text, wf, x, y, TEXT_H, 4),
        None => mtext_ent(&text, x, y, TEXT_H, 4),
    }
}

/// 标题 MTEXT（按当前语言取词 + 必要时内联压缩；正中）。
fn title_mtext(hub: bool) -> EntityType {
    let key = if hub {
        "card.din.title.int"
    } else {
        "card.din.title.ext"
    };
    let text = crate::i18n::t(key);
    match inline_wf(key, crate::i18n::lang()) {
        Some(wf) => mtext_with_inline_wf(&text, wf, W_TOTAL / 2.0, TITLE_Y, TITLE_H, 5),
        None => mtext_ent(&text, W_TOTAL / 2.0, TITLE_Y, TITLE_H, 5),
    }
}

/// 值/标记 ATTDEF：左中，实体级字宽 [`VALUE_W_FACTOR`]，样式一律 `OCSM_GB`。
fn value_attdef(tag: &str, x: f64, y: f64) -> AttributeDefinition {
    let mut ad = AttributeDefinition::new(tag.to_string(), String::new(), " ".to_string());
    ad.insertion_point = Vector3::new(x, y, 0.0);
    ad.alignment_point = ad.insertion_point;
    ad.height = TEXT_H;
    ad.width_factor = VALUE_W_FACTOR;
    ad.text_style = "OCSM_GB".into();
    ad.horizontal_alignment = HorizontalAlignment::Left;
    ad.vertical_alignment = VerticalAlignment::Middle;
    ad.flags.preset = true;
    ad.common = common_of("6文字层");
    ad
}

/// 单栏 13 行标签 MTEXT（标题 + 12 标签）；线条另见 [`block_entities`]。
fn labels(hub: bool) -> Vec<EntityType> {
    let mut out = Vec::with_capacity(13);
    out.push(title_mtext(hub));
    for (i, r) in ROWS.iter().enumerate() {
        // 行 1（标记行）现在也有自己的**标签格**（`Nabe DIN 5480` / `Welle DIN 5480`），
        // 值格只放代号体（用户 2026-09-26）。
        let y = if i == 0 { DESIG_Y } else { data_row_y(i - 1) };
        let (key, symbol) = if hub {
            (r.hub_key, r.hub_symbol)
        } else {
            (r.shaft_key, r.shaft_symbol)
        };
        out.push(label_mtext(key, symbol, LABEL_X, y));
    }
    out
}

/// 13 个 ATTDEF（标记 + 12 值；与 [`values`] 同序）。
pub fn attdefs(hub: bool) -> Vec<AttributeDefinition> {
    let mut out = Vec::with_capacity(ROWS.len());
    // 标记行：标签是 `Nabe/Welle DIN 5480`（MTEXT），值格只放代号体。
    out.push(value_attdef(
        if hub { ROWS[0].hub_tag } else { ROWS[0].shaft_tag },
        VALUE_X,
        DESIG_Y,
    ));
    for i in 1..ROWS.len() {
        let y = data_row_y(i - 1);
        out.push(value_attdef(
            if hub { ROWS[i].hub_tag } else { ROWS[i].shaft_tag },
            VALUE_X,
            y,
        ));
    }
    out
}

/// 表格块成员（单栏）：18 线 + 13 MTEXT（标题 + 12 标签）+ 13 ATTDEF。
///
/// 版式（y 向上为正，表从 y=0 向下）：
/// * 标题行 `[0, −44]`；标记行 `[−44, −104]`（标记自身即行内容，不另画标签）；
///   12 数据行各 40，底 `y=−584`；
/// * 竖线：全高 `x=0, 420`；标签/值分格 `x=240`（自 −44 起）。
pub fn block_entities(hub: bool) -> Vec<EntityType> {
    let mut out = Vec::new();
    // 横线：0、-44、-104，随后 12 条行底（-144 … -584）。
    let mut ys = vec![0.0, -H_TITLE, -(H_TITLE + H_DESIG)];
    for i in 1..=12 {
        ys.push(-(H_TITLE + H_DESIG) - i as f64 * H_ROW);
    }
    for y in &ys {
        out.push(crate::partgen_kit::line([0.0, *y], [W_TOTAL, *y], "1轮廓实线层"));
    }
    // 全高竖线（外框）。
    for x in [0.0, W_TOTAL] {
        out.push(crate::partgen_kit::line(
            [x, 0.0],
            [x, -(H_TITLE + H_DESIG + 12.0 * H_ROW)],
            "1轮廓实线层",
        ));
    }
    // 标签/值分格竖线（标题行不切）。
    out.push(crate::partgen_kit::line(
        [W_LABEL, -H_TITLE],
        [W_LABEL, -(H_TITLE + H_DESIG + 12.0 * H_ROW)],
        "2细线层",
    ));
    for e in labels(hub) {
        out.push(e);
    }
    for ad in attdefs(hub) {
        out.push(EntityType::AttributeDefinition(ad));
    }
    out
}

// ══════════════════════════════════════════════════════════════════════════
// 输入模型 / 解析
// ══════════════════════════════════════════════════════════════════════════

/// 一个配合（等级 + 偏差系列字母）。
#[derive(Debug, Clone, PartialEq)]
pub struct DinFit {
    /// 公差等级（1–12；DIN 5480 常用 7/8/9/10/11）。
    pub grade: u32,
    /// 偏差系列字母（孔 F/G/H/J/K/M；轴 v…a）。
    pub letter: String,
}

impl DinFit {
    /// 代号里的 `9H` / `8f` 写法。
    pub fn token(&self) -> String {
        format!("{}{}", self.grade, self.letter)
    }
    /// 校验字母是否属该侧（`hub=true` 孔）。
    pub fn validate(&self, hub: bool) -> Result<(), String> {
        const HUB: &[&str] = &["F", "G", "H", "J", "K", "M"];
        const SHAFT: &[&str] = &[
            "v", "u", "t", "s", "r", "p", "n", "m", "k", "js", "h", "g", "f", "e", "d", "c", "b",
            "a",
        ];
        if !(1..=12).contains(&self.grade) {
            return Err(crate::i18n::t_fmt(
                "cmd.din.err.grade_range",
                &[("grade", &self.grade.to_string())],
            ));
        }
        let ok = if hub {
            HUB.iter().any(|x| x.eq_ignore_ascii_case(&self.letter))
        } else {
            SHAFT.iter().any(|x| *x == self.letter.to_ascii_lowercase())
        };
        if !ok {
            return Err(crate::i18n::t_fmt(
                "cmd.din.err.dev_letter",
                &[
                    (
                        "side",
                        &crate::i18n::t(if hub {
                            "cmd.din.side.hole"
                        } else {
                            "cmd.din.side.shaft"
                        }),
                    ),
                    ("letter", &self.letter),
                    (
                        "allowed",
                        &crate::i18n::t(if hub {
                            "cmd.din.dev.hub"
                        } else {
                            "cmd.din.dev.shaft"
                        }),
                    ),
                ],
            ));
        }
        Ok(())
    }
}

impl Default for DinFit {
    fn default() -> Self {
        DinFit { grade: 9, letter: "H".into() }
    }
}

/// DIN 花键参数表卡参数（CLI 与 GUI 同一份字段）。
#[derive(Debug, Clone, PartialEq)]
pub struct DinTableSpec {
    /// 模数 m。
    pub m: f64,
    /// 齿数 z。
    pub z: u32,
    /// 基准直径 d_B。
    pub d_b: f64,
    /// 内花键配合（缺省 9H）。
    pub hub: DinFit,
    /// 外花键配合（缺省 8f）。
    pub shaft: DinFit,
    /// `e₂=s₁` 覆盖（mm）。
    pub e2_s1: Option<f64>,
    /// `Ae` 覆盖（mm）。
    pub ae: Option<f64>,
    /// `As` 覆盖（mm）。
    pub as_: Option<f64>,
    /// `Tact`（孔）覆盖。
    pub tact_hub: Option<f64>,
    /// `Teff`（孔）覆盖。
    pub teff_hub: Option<f64>,
    /// `Tact`（轴）覆盖。
    pub tact_shaft: Option<f64>,
    /// `Teff`（轴）覆盖。
    pub teff_shaft: Option<f64>,
    pub at: Option<[f64; 2]>,
    pub rot: f64,
}

impl Default for DinTableSpec {
    fn default() -> Self {
        DinTableSpec {
            m: 3.0,
            z: 38,
            d_b: 120.0,
            hub: DinFit::default(),
            shaft: DinFit { grade: 8, letter: "f".into() },
            e2_s1: None,
            ae: None,
            as_: None,
            tact_hub: None,
            teff_hub: None,
            tact_shaft: None,
            teff_shaft: None,
            at: None,
            rot: 0.0,
        }
    }
}

fn validate_basic(m: f64, z: u32, d_b: f64) -> Result<(), String> {
    if !(m.is_finite() && m > 0.0) {
        return Err(crate::i18n::t_fmt(
            "cmd.din.err.m_positive",
            &[("m", &trim3(m))],
        ));
    }
    if !(3..=1000).contains(&z) {
        return Err(crate::i18n::t_fmt(
            "cmd.din.err.z_range",
            &[("z", &z.to_string())],
        ));
    }
    if !(d_b.is_finite() && d_b > 0.0) {
        return Err(crate::i18n::t_fmt(
            "cmd.din.err.db_positive",
            &[("d_b", &trim3(d_b))],
        ));
    }
    Ok(())
}

/// 是否 Bild 6 示例（N120×3×38×9H / W120×3×38×8f） —— 是则整表照标准原印值。
pub fn is_bild6_example(spec: &DinTableSpec) -> bool {
    (spec.m - 3.0).abs() < 1e-9
        && spec.z == 38
        && (spec.d_b - 120.0).abs() < 1e-3
        && spec.hub.grade == 9
        && spec.hub.letter.eq_ignore_ascii_case("H")
        && spec.shaft.grade == 8
        && spec.shaft.letter == "f"
        && spec.e2_s1.is_none()
        && spec.ae.is_none()
        && spec.as_.is_none()
        && spec.tact_hub.is_none()
        && spec.teff_hub.is_none()
        && spec.tact_shaft.is_none()
        && spec.teff_shaft.is_none()
}

/// 名义表命中行（(m, d_B, z) 精确；只取本卡需要的字段）。
#[derive(Debug, Clone, PartialEq)]
pub struct DinNominal {
    /// 公称齿槽宽/齿厚 `e₂=s₁`。
    pub e2_s1: f64,
    /// 内花键齿根圆 d_f2 与偏差 A_df2。
    pub d_f2: f64,
    pub a_df2: f64,
    /// 内花键齿根成形圆 min。
    pub d_ff2_min: f64,
    /// 内花键齿顶圆。
    pub d_a2: f64,
    /// 外花键齿顶圆。
    pub d_a1: f64,
    /// 外花键齿根成形圆 max。
    pub d_ff1_max: f64,
    /// 外花键齿根圆与偏差 A_df1。
    pub d_f1: f64,
    pub a_df1: f64,
    /// 行来源说明。
    pub source: String,
}

/// 查名义表 `(m, d_B, z)`（同 d_B 多 z 是表事实，必须带 z）。
pub fn nominal_lookup(m: f64, z: u32, d_b: f64) -> Option<DinNominal> {
    crate::invol_spline::din5480_rows()
        .iter()
        .find(|r| {
            (r.m - m).abs() < 1e-9 && (r.d_b - d_b).abs() < 1e-3 && r.z == z
        })
        .map(|r| DinNominal {
            e2_s1: r.e2_s1,
            d_f2: r.d_f2,
            a_df2: r.a_df2,
            d_ff2_min: r.d_ff2_min,
            d_a2: r.d_a2,
            d_a1: r.d_a1,
            d_ff1_max: r.d_ff1_max,
            d_f1: r.d_f1,
            a_df1: r.a_df1,
            source: format!(
                "DIN 5480-2 名义表 m={} d_B={} z={}（page={} 表{}，source={}）",
                crate::partgen_kit::trim(r.m),
                crate::partgen_kit::trim(r.d_b),
                r.z,
                r.page,
                r.table_no,
                r.source,
            ),
        })
}

/// 检验表命中行（(m, d_B, z)）。
#[derive(Debug, Clone, PartialEq)]
pub struct DinInspection {
    /// 内花键量棒直径与棒间距。
    pub d_m_hub: f64,
    pub m2: f64,
    /// 外花键量棒直径与跨棒距。
    pub d_m_shaft: f64,
    pub m1: f64,
    /// 行来源说明。
    pub source: String,
}

/// 查检验表 `(m, d_B, z)`。
pub fn inspection_lookup(m: f64, z: u32, d_b: f64) -> Option<DinInspection> {
    crate::invol_spline::inspection_rows()
        .iter()
        .find(|r| (r.m - m).abs() < 1e-9 && (r.d_b - d_b).abs() < 1e-3 && r.z == z)
        .map(|r| DinInspection {
            d_m_hub: r.d_m_hub,
            m2: r.m2,
            d_m_shaft: r.d_m_shaft,
            m1: r.m1,
            source: r.source_note(),
        })
}

/// 求值结果（测试/预览/落图共用）。
#[derive(Debug, Clone, PartialEq)]
pub struct DinDerived {
    /// 名义表行（查不到 = None，直径与 e₂ 显示「—」）。
    pub nominal: Option<DinNominal>,
    /// `e₂=s₁`（mm）。
    pub e2_s1: Option<f64>,
    /// 内花键 Ae（mm）与说明。
    pub ae: Option<f64>,
    pub ae_note: String,
    /// 外花键 As（mm）与说明。
    pub as_: Option<f64>,
    pub as_note: String,
    /// 孔 9H 的 Tact/Teff（mm）与说明。
    pub tact_hub: Option<f64>,
    pub teff_hub: Option<f64>,
    pub tol_hub_note: String,
    /// 轴 8f 的 Tact/Teff。
    pub tact_shaft: Option<f64>,
    pub teff_shaft: Option<f64>,
    pub tol_shaft_note: String,
    /// e/s 六值（mm）。
    pub e_max: Option<f64>,
    pub e_min: Option<f64>,
    pub e_vmin: Option<f64>,
    pub s_vmax: Option<f64>,
    pub s_max: Option<f64>,
    pub s_min: Option<f64>,
    /// D_M / M2 / M1（mm）。
    pub d_m_hub: Option<f64>,
    pub m2_max: Option<f64>,
    pub m2_min: Option<f64>,
    pub d_m_shaft: Option<f64>,
    pub m1_max: Option<f64>,
    pub m1_min: Option<f64>,
    /// 是否 Bild 6 示例路径（整表照原印值）。
    pub anchor: bool,
    /// 数据缺口说明（进预览 missing_note/回执）。
    pub notes: Vec<String>,
}

/// 三数相加（任一 None → None）。
fn add3(a: Option<f64>, b: Option<f64>, c: Option<f64>) -> Option<f64> {
    Some(a? + b? + c?)
}

fn sub2(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    Some(a? - b?)
}

/// 求值：名义表 + Table 7（或显式覆盖）+ 表 6 公式；Bild 6 示例走锚点并保持公式一致。
pub fn derive(spec: &DinTableSpec) -> Result<DinDerived, String> {
    validate_basic(spec.m, spec.z, spec.d_b)?;
    spec.hub
        .validate(true)
        .map_err(|e| crate::i18n::t_fmt("cmd.din.err.prefix", &[("e", &e)]))?;
    spec.shaft
        .validate(false)
        .map_err(|e| crate::i18n::t_fmt("cmd.din.err.prefix", &[("e", &e)]))?;
    let nominal = nominal_lookup(spec.m, spec.z, spec.d_b);
    let e2 = spec.e2_s1.or(nominal.as_ref().map(|n| n.e2_s1));
    let mut notes = Vec::new();

    // Ae / As：显式覆盖 > Table 7 查表 > 「—」。
    let mut ae_note;
    let mut as_note;
    let ae = match spec.ae {
        Some(v) => {
            ae_note = format!("显式输入 Ae={}", trim3(v));
            Some(v)
        }
        None => match dev_lookup_mm(&spec.hub.letter, spec.d_b, true) {
            Ok((v, n)) => {
                ae_note = n;
                Some(v)
            }
            Err(e) => {
                ae_note = e.clone();
                notes.push(format!("Ae：{e}"));
                None
            }
        },
    };
    let as_ = match spec.as_ {
        Some(v) => {
            as_note = format!("显式输入 As={}", trim3(v));
            Some(v)
        }
        None => match dev_lookup_mm(&spec.shaft.letter, spec.d_b, false) {
            Ok((v, n)) => {
                as_note = n;
                Some(v)
            }
            Err(e) => {
                as_note = e.clone();
                notes.push(format!("As：{e}"));
                None
            }
        },
    };

    // Tact / Teff：显式覆盖 > Table 7 公差锚 > 「—」。
    let mut resolve_tol = |grade: u32,
                           tact_ov: Option<f64>,
                           teff_ov: Option<f64>,
                           who: &str|
     -> (Option<f64>, Option<f64>, String) {
        if let (Some(tact), Some(teff)) = (tact_ov, teff_ov) {
            return (
                Some(tact),
                Some(teff),
                format!("显式输入 {who} Tact={}、Teff={}", trim3(tact), trim3(teff)),
            );
        }
        match tol_lookup_mm(grade, spec.m) {
            Ok((tact, teff, _tg, n)) => (
                tact_ov.or(Some(tact)),
                teff_ov.or(Some(teff)),
                n,
            ),
            Err(e) => {
                if tact_ov.is_none() && teff_ov.is_none() {
                    notes.push(format!("{who} 公差：{e}"));
                }
                (tact_ov, teff_ov, e)
            }
        }
    };
    let (tact_hub, teff_hub, tol_hub_note) =
        resolve_tol(spec.hub.grade, spec.tact_hub, spec.teff_hub, "孔");
    let (tact_shaft, teff_shaft, tol_shaft_note) =
        resolve_tol(spec.shaft.grade, spec.tact_shaft, spec.teff_shaft, "轴");

    let anchor = is_bild6_example(spec);
    let (mut d_m_hub, mut m2_max, mut m2_min, mut d_m_shaft, mut m1_max, mut m1_min) =
        (None, None, None, None, None, None);
    if anchor {
        d_m_hub = anchor_f64("hub", "D_M");
        m2_max = anchor_f64("hub", "M2_max");
        m2_min = anchor_f64("hub", "M2_min");
        d_m_shaft = anchor_f64("shaft", "D_M");
        m1_max = anchor_f64("shaft", "M1_max");
        m1_min = anchor_f64("shaft", "M1_min");
    } else if let Some(insp) = inspection_lookup(spec.m, spec.z, spec.d_b) {
        d_m_hub = Some(insp.d_m_hub);
        m2_max = Some(insp.m2);
        m2_min = Some(insp.m2);
        d_m_shaft = Some(insp.d_m_shaft);
        m1_max = Some(insp.m1);
        m1_min = Some(insp.m1);
        notes.push(format!(
            "M2/M1 取检验表名义值（{}）：该表只给基本偏差行，max/min 需按 A*_M 系数换算，本卡如实给名义值",
            insp.source
        ));
    } else {
        notes.push(format!(
            "D_M/M2/M1：检验表没有 m={} d_B={} z={} 行（已入库档位见 din5480_2_notes.md）—— 显示「—」",
            crate::partgen_kit::trim(spec.m),
            crate::partgen_kit::trim(spec.d_b),
            spec.z
        ));
    }

    // 表 6 六值。
    let (e_max, e_min, e_vmin, s_vmax, s_max, s_min) = if anchor {
        (
            anchor_f64("hub", "e_max"),
            anchor_f64("hub", "e_min"),
            anchor_f64("hub", "e_vmin"),
            anchor_f64("shaft", "s_vmax"),
            anchor_f64("shaft", "s_max"),
            anchor_f64("shaft", "s_min"),
        )
    } else {
        let ev = e2.zip(ae).map(|(a, b)| a + b);
        let emin = add3(e2, ae, teff_hub);
        let emax = emin.zip(tact_hub).map(|(a, b)| a + b);
        let sv = e2.zip(as_).map(|(a, b)| a + b);
        let smax = sub2(sv, teff_shaft);
        let smin = sv
            .zip(tact_shaft)
            .zip(teff_shaft)
            .map(|((a, b), c)| a - b - c);
        (emax, emin, ev, sv, smax, smin)
    };

    Ok(DinDerived {
        nominal,
        e2_s1: e2,
        ae,
        ae_note,
        as_,
        as_note,
        tact_hub,
        teff_hub,
        tol_hub_note,
        tact_shaft,
        teff_shaft,
        tol_shaft_note,
        e_max,
        e_min,
        e_vmin,
        s_vmax,
        s_max,
        s_min,
        d_m_hub,
        m2_max,
        m2_min,
        d_m_shaft,
        m1_max,
        m1_min,
        anchor,
        notes,
    })
}

/// 3 位小数（显示与内部计算分开）；`None` → 「—」。
pub fn fmt_mm(v: Option<f64>) -> String {
    v.map(|v| trim3(v)).unwrap_or_else(|| MISSING.to_string())
}

/// 最多 3 位小数。
pub fn trim3(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// 带符号偏差显示：`+0.76` / `-1.15`。
pub fn fmt_dev(v: f64) -> String {
    if v >= 0.0 {
        format!("+{}", trim3(v))
    } else {
        trim3(v)
    }
}

/// 代号（§8）全串（回执/元数据）：`Nabe DIN 5480 – N d_B×m×z×等级+字母`。
pub fn designation(spec: &DinTableSpec, hub: bool) -> String {
    format!(
        "{} – {}",
        designation_label(hub),
        designation_body(spec, hub)
    )
}

/// 标记行的**标签格**（用户 2026-09-26：标签/值分列后各放一份内容；**随语言**）。
pub fn designation_label(hub: bool) -> String {
    crate::i18n::t(if hub {
        "card.din.label.nabe"
    } else {
        "card.din.label.welle"
    })
}

/// 标记行的**值格**（§8 代号体）：`N d_B×m×z×等级+字母` / `W …`。
pub fn designation_body(spec: &DinTableSpec, hub: bool) -> String {
    format!(
        "{}{}×{}×{}×{}",
        if hub { "N" } else { "W" },
        trim3(spec.d_b),
        trim3(spec.m),
        spec.z,
        if hub { spec.hub.token() } else { spec.shaft.token() }
    )
}

/// 13 项取值（单栏，顺序 = [`attdefs(hub)`]：标记 + 12 值）。
pub fn values(spec: &DinTableSpec, hub: bool) -> Result<Vec<(String, String)>, String> {
    let d = derive(spec)?;
    let n = d.nominal.as_ref();
    let mut out: Vec<(&'static str, String)> = Vec::with_capacity(ROWS.len());
    for (i, r) in ROWS.iter().enumerate() {
        let v: String = match i {
            0 => designation_body(spec, hub),
            1 => spec.z.to_string(),
            2 => trim3(spec.m),
            3 => "30°".to_string(),
            4 => {
                // 直径行 1：内 = 齿根圆 d_f2 / 外 = 齿顶圆 d_a1
                if hub {
                    if d.anchor {
                        anchor_display("hub", "d_f2").unwrap_or_else(|| MISSING.to_string())
                    } else {
                        n.map(|r| format!("{} {}", trim3(r.d_f2), fmt_dev(r.a_df2)))
                            .unwrap_or_else(|| MISSING.to_string())
                    }
                } else if d.anchor {
                    anchor_display("shaft", "d_a1").unwrap_or_else(|| MISSING.to_string())
                } else {
                    n.map(|r| format!("{} h11", trim3(r.d_a1)))
                        .unwrap_or_else(|| MISSING.to_string())
                }
            }
            5 => {
                // 齿根成形圆
                if hub {
                    if d.anchor {
                        anchor_display("hub", "d_Ff2").unwrap_or_else(|| MISSING.to_string())
                    } else {
                        n.map(|r| format!("{} min.", trim3(r.d_ff2_min)))
                            .unwrap_or_else(|| MISSING.to_string())
                    }
                } else if d.anchor {
                    anchor_display("shaft", "d_Ff1").unwrap_or_else(|| MISSING.to_string())
                } else {
                    n.map(|r| format!("{} max.", trim3(r.d_ff1_max)))
                        .unwrap_or_else(|| MISSING.to_string())
                }
            }
            6 => {
                // 直径行 3：内 = 齿顶圆 d_a2 / 外 = 齿根圆 d_f1
                if hub {
                    if d.anchor {
                        anchor_display("hub", "d_a2").unwrap_or_else(|| MISSING.to_string())
                    } else {
                        n.map(|r| format!("{} H11", trim3(r.d_a2)))
                            .unwrap_or_else(|| MISSING.to_string())
                    }
                } else if d.anchor {
                    anchor_display("shaft", "d_f1").unwrap_or_else(|| MISSING.to_string())
                } else {
                    n.map(|r| format!("{} {}", trim3(r.d_f1), fmt_dev(r.a_df1)))
                        .unwrap_or_else(|| MISSING.to_string())
                }
            }
            7 => if hub { fmt_mm(d.e_max) } else { fmt_mm(d.s_vmax) },
            8 => if hub { fmt_mm(d.e_min) } else { fmt_mm(d.s_max) },
            9 => if hub { fmt_mm(d.e_vmin) } else { fmt_mm(d.s_min) },
            10 => if hub { fmt_mm(d.d_m_hub) } else { fmt_mm(d.d_m_shaft) },
            11 => if hub { fmt_mm(d.m2_max) } else { fmt_mm(d.m1_max) },
            12 => if hub { fmt_mm(d.m2_min) } else { fmt_mm(d.m1_min) },
            _ => MISSING.to_string(),
        };
        out.push((if hub { r.hub_tag } else { r.shaft_tag }, v));
    }

    // 顺序护栏：取值顺序必须与 ATTDEF 表一致（加/改行时先在这里暴露）。
    let got: Vec<&str> = out.iter().map(|(t, _)| *t).collect();
    let want: Vec<String> = attdefs(hub).iter().map(|ad| ad.tag.clone()).collect();
    let want: Vec<&str> = want.iter().map(|s| s.as_str()).collect();
    if got != want {
        return Err(crate::i18n::t_fmt(
            "cmd.din.err.tag_order",
            &[("got", &format!("{got:?}")), ("want", &format!("{want:?}"))],
        ));
    }
    Ok(out.into_iter().map(|(t, v)| (t.to_string(), v)).collect())
}

/// 建 INSERT（单栏；基点在 `at`，旋转 `rot_deg` 度；13 个 ATTRIB 取自 `values(spec, hub)`）。
/// 整表 **INSERT 缩放 0.17**，不改块内图元几何；ATTRIB 随 INSERT 缩放。
pub fn build_insert(
    spec: &DinTableSpec,
    hub: bool,
    at: [f64; 2],
    rot_deg: f64,
) -> Result<Insert, String> {
    let vals = values(spec, hub)?;
    let mut ins = Insert::new(block_name(hub), Vector3::new(at[0], at[1], 0.0));
    ins.rotation = rot_deg.to_radians();
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
    for ad in attdefs(hub) {
        let val = vals
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

/// 13 项 Markdown（回执/计算书）。
pub fn markdown_table(spec: &DinTableSpec, hub: bool) -> Result<String, String> {
    let vals = values(spec, hub)?;
    let mut md = String::new();
    md.push_str("| 属性 | 值 |\n|---|---|\n");
    for (tag, v) in vals {
        md.push_str(&format!("| {tag} | {v} |\n"));
    }
    Ok(md)
}

// ══════════════════════════════════════════════════════════════════════════
// GUI 选项表与 GUI/HTTP 模型
// ══════════════════════════════════════════════════════════════════════════

/// DIN 内花键卡（Nabe）表达式策略（一卡一方向：KIND 必须 IN；α=30、直齿；变位进 d_B）。
pub const EXPR_POLICY_INT: crate::card_expr::ExprPolicy = crate::card_expr::ExprPolicy {
    card: "DIN 5480 内花键参数表",
    mark: crate::card_expr::ExprMark::Spline,
    alphas: &[30.0],
    spur: true,
    allow_shift: true,
    map: &[
        crate::card_expr::ExprRule { target: "d_b", label: "基准直径 d_B", op: crate::card_expr::ExprOp::DinBaseDB },
        crate::card_expr::ExprRule { target: "m", label: "模数 m", op: crate::card_expr::ExprOp::Module },
        crate::card_expr::ExprRule { target: "z", label: "齿数 z", op: crate::card_expr::ExprOp::Teeth },
    ],
};

/// DIN 外花键卡（Welle）表达式策略（一卡一方向：KIND 必须 EX）。
pub const EXPR_POLICY_EXT: crate::card_expr::ExprPolicy = crate::card_expr::ExprPolicy {
    card: "DIN 5480 外花键参数表",
    mark: crate::card_expr::ExprMark::Spline,
    alphas: &[30.0],
    spur: true,
    allow_shift: true,
    map: &[crate::card_expr::ExprRule {
        target: "d_b",
        label: "基准直径 d_B",
        op: crate::card_expr::ExprOp::DinBaseDB,
    }, crate::card_expr::ExprRule {
        target: "m",
        label: "模数 m",
        op: crate::card_expr::ExprOp::Module,
    }, crate::card_expr::ExprRule {
        target: "z",
        label: "齿数 z",
        op: crate::card_expr::ExprOp::Teeth,
    }],
};

/// 公用缺项说明 —— 本批入 catalog（`gui.form.din.missing_note`，随语言；原文见 i18n.rs）。
/// 预览/选项 JSON 的那份（多一句「可用 ae/as/… 覆盖」）另走 `gui.din.missing_note`。

/// DIN 内花键卡（Nabe）表单（单栏 Bild 6）。
pub const FORM_INT: crate::card::CardFormSpec = crate::card::CardFormSpec {
    fields: &[
        crate::card::CardFieldSpec {
            key: "expr", label: "gui.fld.din_table.form_int.expr.label",
            kind: "textarea", placeholder: "SPLINE IN M3 Z38 ALPHA30 X0.45 BETA0 H30", default: "",
            title: "gui.fld.din_table.form_int.expr.title",
            options: &[], options_from: "", min: 0.0, step: 0.0, required: false,
        },
        crate::card::CardFieldSpec { key: "m", label: "gui.fld.din_table.form_int.m.label", kind: "number",
            placeholder: "gui.fld.din_table.form_int.m.placeholder", default: "3",
            title: "gui.fld.din_table.form_int.m.title", options: &[], options_from: "",
            min: 0.0, step: 0.001, required: true },
        crate::card::CardFieldSpec { key: "z", label: "gui.fld.din_table.form_int.z.label", kind: "number",
            placeholder: "gui.fld.din_table.form_int.z.placeholder", default: "38",
            title: "gui.fld.din_table.form_int.z.title", options: &[], options_from: "",
            min: 3.0, step: 1.0, required: true },
        crate::card::CardFieldSpec { key: "d_b", label: "gui.fld.din_table.form_int.d_b.label", kind: "number",
            placeholder: "gui.fld.din_table.form_int.d_b.placeholder", default: "120",
            title: "gui.fld.din_table.form_int.d_b.title", options: &[], options_from: "",
            min: 0.0, step: 0.001, required: true },
        crate::card::CardFieldSpec { key: "hub", label: "gui.fld.din_table.form_int.hub.label", kind: "text",
            placeholder: "gui.fld.din_table.form_int.hub.placeholder", default: "9H",
            title: "gui.fld.din_table.form_int.hub.title", options: &[], options_from: "",
            min: 0.0, step: 0.0, required: false },
        crate::card::CardFieldSpec { key: "e2", label: "gui.fld.din_table.form_int.e2.label", kind: "number",
            placeholder: "gui.fld.din_table.form_int.e2.placeholder", default: "",
            title: "gui.fld.din_table.form_int.e2.title", options: &[], options_from: "",
            min: 0.0, step: 0.001, required: false },
        crate::card::CardFieldSpec { key: "ae", label: "gui.fld.din_table.form_int.ae.label", kind: "number",
            placeholder: "gui.fld.din_table.form_int.ae.placeholder", default: "",
            title: "gui.fld.din_table.form_int.ae.title", options: &[], options_from: "",
            min: 0.0, step: 0.001, required: false },
        crate::card::CardFieldSpec { key: "as_", label: "gui.fld.din_table.form_int.as_.label", kind: "number",
            placeholder: "gui.fld.din_table.form_int.as_.placeholder", default: "",
            title: "gui.fld.din_table.form_int.as_.title", options: &[], options_from: "",
            min: 0.0, step: 0.001, required: false },
        crate::card::CardFieldSpec { key: "tact_n", label: "gui.fld.din_table.form_int.tact_n.label", kind: "number",
            placeholder: "gui.fld.din_table.form_int.tact_n.placeholder", default: "",
            title: "gui.fld.din_table.form_int.tact_n.title", options: &[], options_from: "",
            min: 0.0, step: 0.0001, required: false },
        crate::card::CardFieldSpec { key: "teff_n", label: "gui.fld.din_table.form_int.teff_n.label", kind: "number",
            placeholder: "gui.fld.din_table.form_int.teff_n.placeholder", default: "",
            title: "gui.fld.din_table.form_int.teff_n.title", options: &[], options_from: "",
            min: 0.0, step: 0.0001, required: false },
    ],
    note_key: "gui.form.din.note",
    missing_note_key: "gui.form.din.missing_note",
};

/// DIN 外花键卡（Welle）表单（单栏 Bild 6）。
pub const FORM_EXT: crate::card::CardFormSpec = crate::card::CardFormSpec {
    fields: &[
        crate::card::CardFieldSpec {
            key: "expr", label: "gui.fld.din_table.form_ext.expr.label",
            kind: "textarea", placeholder: "SPLINE EX M3 Z38 ALPHA30 X0.45 BETA0 H30", default: "",
            title: "gui.fld.din_table.form_ext.expr.title",
            options: &[], options_from: "", min: 0.0, step: 0.0, required: false,
        },
        crate::card::CardFieldSpec { key: "m", label: "gui.fld.din_table.form_ext.m.label", kind: "number",
            placeholder: "gui.fld.din_table.form_ext.m.placeholder", default: "3",
            title: "gui.fld.din_table.form_ext.m.title", options: &[], options_from: "",
            min: 0.0, step: 0.001, required: true },
        crate::card::CardFieldSpec { key: "z", label: "gui.fld.din_table.form_ext.z.label", kind: "number",
            placeholder: "gui.fld.din_table.form_ext.z.placeholder", default: "38",
            title: "gui.fld.din_table.form_ext.z.title", options: &[], options_from: "",
            min: 3.0, step: 1.0, required: true },
        crate::card::CardFieldSpec { key: "d_b", label: "gui.fld.din_table.form_ext.d_b.label", kind: "number",
            placeholder: "gui.fld.din_table.form_ext.d_b.placeholder", default: "120",
            title: "gui.fld.din_table.form_ext.d_b.title", options: &[], options_from: "",
            min: 0.0, step: 0.001, required: true },
        crate::card::CardFieldSpec { key: "shaft", label: "gui.fld.din_table.form_ext.shaft.label", kind: "text",
            placeholder: "gui.fld.din_table.form_ext.shaft.placeholder", default: "8f",
            title: "gui.fld.din_table.form_ext.shaft.title", options: &[], options_from: "",
            min: 0.0, step: 0.0, required: false },
        crate::card::CardFieldSpec { key: "e2", label: "gui.fld.din_table.form_ext.e2.label", kind: "number",
            placeholder: "gui.fld.din_table.form_ext.e2.placeholder", default: "",
            title: "gui.fld.din_table.form_ext.e2.title", options: &[], options_from: "",
            min: 0.0, step: 0.001, required: false },
        crate::card::CardFieldSpec { key: "ae", label: "gui.fld.din_table.form_ext.ae.label", kind: "number",
            placeholder: "gui.fld.din_table.form_ext.ae.placeholder", default: "",
            title: "gui.fld.din_table.form_ext.ae.title", options: &[], options_from: "",
            min: 0.0, step: 0.001, required: false },
        crate::card::CardFieldSpec { key: "as_", label: "gui.fld.din_table.form_ext.as_.label", kind: "number",
            placeholder: "gui.fld.din_table.form_ext.as_.placeholder", default: "",
            title: "gui.fld.din_table.form_ext.as_.title", options: &[], options_from: "",
            min: 0.0, step: 0.001, required: false },
        crate::card::CardFieldSpec { key: "tact_w", label: "gui.fld.din_table.form_ext.tact_w.label", kind: "number",
            placeholder: "gui.fld.din_table.form_ext.tact_w.placeholder", default: "",
            title: "gui.fld.din_table.form_ext.tact_w.title", options: &[], options_from: "",
            min: 0.0, step: 0.0001, required: false },
        crate::card::CardFieldSpec { key: "teff_w", label: "gui.fld.din_table.form_ext.teff_w.label", kind: "number",
            placeholder: "gui.fld.din_table.form_ext.teff_w.placeholder", default: "",
            title: "gui.fld.din_table.form_ext.teff_w.title", options: &[], options_from: "",
            min: 0.0, step: 0.0001, required: false },
    ],
    note_key: "gui.form.din.note",
    missing_note_key: "gui.form.din.missing_note",
};

/// DIN 卡选项/口径 JSON（随 `/api/spline_options` 下发；页面只渲染）。
pub fn options_json() -> serde_json::Value {
    serde_json::json!({
        "columns": ROWS.iter().enumerate().flat_map(|(i, r)| {
            let (label, tag, unit, source) = if i == 0 {
                (row_label(r, true), r.hub_tag, "", r.source())
            } else {
                (row_label(r, true), r.hub_tag, "mm", r.source())
            };
            let _ = source;
            vec![
                serde_json::json!({"tag": tag, "label": label, "unit": unit,
                                   "formula": r.formula(), "source": r.source()}),
                serde_json::json!({"tag": r.shaft_tag, "label": row_label(r, false), "unit": unit,
                                   "formula": r.formula(), "source": r.source()}),
            ]
        }).collect::<Vec<_>>(),
        "hub_letters": ["F","G","H","J","K","M"],
        "shaft_letters": ["v","u","t","s","r","p","n","m","k","js","h","g","f","e","d","c","b","a"],
        "grades": [1,2,3,4,5,6,7,8,9,10,11,12],
        "default": {"m": 3.0, "z": 38, "d_b": 120.0, "hub": "9H", "shaft": "8f"},
        "example": {
            "m": 3.0, "z": 38, "d_b": 120.0, "hub": "9H", "shaft": "8f",
            "note": crate::i18n::t("gui.din.preview.note")
        },
        "missing_note": crate::i18n::t("gui.din.missing_note"),
        "columns_int": ROWS.iter().enumerate().map(|(i, r)| serde_json::json!({
            "tag": r.hub_tag, "label": row_label(r, true),
            "unit": if i == 0 { "" } else { "mm" },
            "formula": r.formula(), "source": r.source(),
        })).collect::<Vec<_>>(),
        "columns_ext": ROWS.iter().enumerate().map(|(i, r)| serde_json::json!({
            "tag": r.shaft_tag, "label": row_label(r, false),
            "unit": if i == 0 { "" } else { "mm" },
            "formula": r.formula(), "source": r.source(),
        })).collect::<Vec<_>>(),
        "note": crate::i18n::t("gui.din.options.note"),
    })
}

/// CLI/HTTP 表单模型（字段与 GUI 控件一一对应）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DinTableModel {
    /// 卡类型（GUI 回传；后端按 renderer 分派，这里只记不看）。
    #[serde(default)]
    pub card: String,
    /// 方向（`int`/`ext`；由卡类型固定，后端分派时回填；旧请求显式给也收）。
    #[serde(default)]
    pub side: Option<String>,
    /// 九字段统一齿形表达式（可空；给了则覆盖 m/z/d_B）。
    #[serde(default)]
    pub expr: Option<String>,
    /// 模数 m。
    pub m: f64,
    /// 齿数 z。
    pub z: u32,
    /// 基准直径 d_B。
    pub d_b: f64,
    /// 内花键配合（`9H`）。
    #[serde(default)]
    pub hub: Option<String>,
    /// 外花键配合（`8f`）。
    #[serde(default)]
    pub shaft: Option<String>,
    /// 覆盖值。
    #[serde(default)]
    pub e2: Option<f64>,
    #[serde(default)]
    pub ae: Option<f64>,
    #[serde(default)]
    pub as_: Option<f64>,
    #[serde(default)]
    pub tact_n: Option<f64>,
    #[serde(default)]
    pub teff_n: Option<f64>,
    #[serde(default)]
    pub tact_w: Option<f64>,
    #[serde(default)]
    pub teff_w: Option<f64>,
    #[serde(default)]
    pub at: Option<[f64; 2]>,
    #[serde(default)]
    pub rot: f64,
}

impl DinTableModel {
    /// 方向（缺省内）：`ext`/`外` → 外，其余 → 内。
    pub fn hub(&self) -> bool {
        !matches!(
            self.side.as_deref().map(str::trim).map(|s| s.to_ascii_lowercase()).as_deref(),
            Some("ext") | Some("external") | Some("外") | Some("外花键") | Some("welle")
        )
    }

    /// 方向错误文案用的卡名。
    fn card_label(hub: bool) -> &'static str {
        if hub {
            "DIN 5480 内花键参数表"
        } else {
            "DIN 5480 外花键参数表"
        }
    }

    /// →（校验过的 `DinTableSpec`；方向由卡决定）。
    pub fn spec(&self) -> Result<DinTableSpec, String> {
        let hub = self.hub();
        let card = Self::card_label(hub);
        let hub_fit = parse_fit_token(self.hub.as_deref().unwrap_or("9H"), true)
            .map_err(|e| format!("{card}：孔配合「{}」{e}", self.hub.as_deref().unwrap_or("9H")))?;
        let shaft_fit = parse_fit_token(self.shaft.as_deref().unwrap_or("8f"), false)
            .map_err(|e| format!("{card}：轴配合「{}」{e}", self.shaft.as_deref().unwrap_or("8f")))?;
        let (m, z, d_b) = match self.expr.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(e) => {
                let policy = if hub { &EXPR_POLICY_INT } else { &EXPR_POLICY_EXT };
                let r = crate::card_expr::resolve(policy, e, Some(hub))?;
                (
                    r.value("m")
                        .ok_or_else(|| format!("{card}：表达式映射表缺 m（内部错误）"))?,
                    r.value("z")
                        .ok_or_else(|| format!("{card}：表达式映射表缺 z（内部错误）"))?
                        as u32,
                    r.value("d_b")
                        .ok_or_else(|| format!("{card}：表达式映射表缺 d_B（内部错误）"))?,
                )
            }
            None => (self.m, self.z, self.d_b),
        };
        let spec = DinTableSpec {
            m,
            z,
            d_b,
            hub: hub_fit,
            shaft: shaft_fit,
            e2_s1: self.e2,
            ae: self.ae,
            as_: self.as_,
            tact_hub: self.tact_n,
            teff_hub: self.teff_n,
            tact_shaft: self.tact_w,
            teff_shaft: self.teff_w,
            at: self.at,
            rot: self.rot,
        };
        derive(&spec)?;
        Ok(spec)
    }

    /// 预览 JSON（不碰图纸；单栏 13 项）。
    pub fn preview_json(&self) -> Result<serde_json::Value, String> {
        let hub = self.hub();
        let spec = self.spec()?;
        let d = derive(&spec)?;
        let vals = values(&spec, hub)?;
        let mut items = Vec::with_capacity(ROWS.len());
        let mut missing = Vec::new();
        for r in ROWS.iter() {
            let (tag, label) = if hub {
                (r.hub_tag, row_label(r, true))
            } else {
                (r.shaft_tag, row_label(r, false))
            };
            let value = vals
                .iter()
                .find(|(t, _)| t == tag)
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            if value == MISSING {
                missing.push(label.clone());
            }
            items.push(serde_json::json!({
                "tag": tag,
                "label": label,
                "unit": if tag.ends_with("标记") { "" } else { "mm" },
                "value": value,
                "formula": r.formula(),
                "source": r.source(),
                "missing": value == MISSING,
            }));
        }
        let readout = serde_json::json!([
            {"k": crate::i18n::t("gui.din.readout.e2_s1"), "v": fmt_mm(d.e2_s1)},
            {"k": crate::i18n::t_fmt("gui.din.readout.ae", &[("hub", &spec.hub.token())]), "v": fmt_mm(d.ae)},
            {"k": crate::i18n::t_fmt("gui.din.readout.as", &[("shaft", &spec.shaft.token())]), "v": fmt_mm(d.as_)},
            {"k": crate::i18n::t_fmt("gui.din.readout.tact_hub", &[("hub", &spec.hub.token())]),
             "v": format!("{} / {}", fmt_mm(d.tact_hub), fmt_mm(d.teff_hub))},
            {"k": crate::i18n::t_fmt("gui.din.readout.tact_shaft", &[("shaft", &spec.shaft.token())]),
             "v": format!("{} / {}", fmt_mm(d.tact_shaft), fmt_mm(d.teff_shaft))},
            {"k": crate::i18n::t("gui.din.readout.anchor_path"), "v": if d.anchor { crate::i18n::t("gui.din.readout.anchor_yes") } else { crate::i18n::t("gui.din.readout.anchor_no") }},
            {"k": crate::i18n::t("gui.din.readout.ae_source"), "v": d.ae_note},
            {"k": crate::i18n::t("gui.din.readout.as_source"), "v": d.as_note},
            {"k": crate::i18n::t("gui.din.readout.tol_hub_source"), "v": d.tol_hub_note, "tol": true},
            {"k": crate::i18n::t("gui.din.readout.tol_shaft_source"), "v": d.tol_shaft_note, "tol": true},
            {"k": crate::i18n::t("gui.din.readout.nominal_table"), "v": d.nominal.as_ref().map(|n| n.source.clone()).unwrap_or_else(|| MISSING.to_string())},
        ]);
        let expr_echo = self
            .expr
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let fields = if expr_echo.is_some() {
            serde_json::json!({"m": spec.m, "z": spec.z, "d_b": spec.d_b})
        } else {
            serde_json::Value::Null
        };
        // GUI 标题 = 卡面标题（随语言）+ 本侧代号体（值原样）。
        let title = format!("{}（{}）", title_text(hub), designation_body(&spec, hub));
        Ok(serde_json::json!({
            "ok": true,
            "card": if hub { "DIN花键参数表" } else { "DIN花键参数表_外" },
            "renderer": "din_table",
            "side": if hub { "int" } else { "ext" },
            "expr": expr_echo,
            "fields": fields,
            "title": title,
            "m": spec.m,
            "z": spec.z,
            "d_b": spec.d_b,
            "hub": spec.hub.token(),
            "shaft": spec.shaft.token(),
            "anchor": d.anchor,
            "readout": readout,
            "items": items,
            "missing": missing,
            "missing_note": options_json()["missing_note"],
        }))
    }

    /// 待放置件要带的 13 个 ATTRIB（tag → 值 + ATTDEF 模板）。
    pub fn pending_attrs(&self) -> Result<Vec<(AttributeDefinition, String)>, String> {
        let hub = self.hub();
        let spec = self.spec()?;
        let vals = values(&spec, hub)?;
        let mut out = Vec::with_capacity(vals.len());
        for ad in attdefs(hub) {
            let v = vals
                .iter()
                .find(|(tag, _)| tag == &ad.tag)
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            out.push((ad, v));
        }
        Ok(out)
    }

    /// 直接落点用的 `INSERT`（整表缩放 0.17）。
    pub fn build_insert(&self) -> Result<Insert, String> {
        let hub = self.hub();
        let spec = self.spec()?;
        let at = self.at.unwrap_or([0.0, 0.0]);
        build_insert(&spec, hub, at, self.rot)
    }

    /// 插入回执里的一段（单栏，只报本侧）。
    pub fn echo_note(&self) -> Result<String, String> {
        let hub = self.hub();
        let spec = self.spec()?;
        let d = derive(&spec)?;
        Ok(crate::i18n::t_fmt(
            "cmd.din.echo",
            &[
                ("designation", &designation(&spec, hub)),
                ("db", &trim3(spec.d_b)),
                ("m", &trim3(spec.m)),
                ("z", &spec.z.to_string()),
                (
                    "anchor",
                    &if d.anchor {
                        crate::i18n::t("cmd.din.echo.anchor")
                    } else {
                        String::new()
                    },
                ),
            ],
        ))
    }

    /// 待放置件的 `OCSM_PART` 元数据（薄台账）。
    pub fn part_meta_json(&self) -> Result<String, String> {
        let hub = self.hub();
        let spec = self.spec()?;
        Ok(serde_json::json!({
            "family": "din_table",
            "card": if hub { "DIN花键参数表" } else { "DIN花键参数表_外" },
            "side": if hub { "int" } else { "ext" },
            "d_b": spec.d_b,
            "m": spec.m,
            "z": spec.z,
            "hub": spec.hub.token(),
            "shaft": spec.shaft.token(),
        })
        .to_string())
    }
}

// ══════════════════════════════════════════════════════════════════════════
// CLI 解析
// ══════════════════════════════════════════════════════════════════════════

/// 命令用法（`OCSMCARD` 报错指路）。
/// 表达式截取用：DIN 卡选项关键字（`N9H`/`W8f`/`e2`/`ae`/`as`/`tactn`…/`at`/`rot`）。
/// `BETA0` 不是 `b` 选项（rest 不是数字）；表达式里的 `M3`/`Z38` 在第 7 token 前不判。
fn is_option_token(t: &str) -> bool {
    let l = t.to_ascii_lowercase();
    if t.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return true; // `9H` 裸配合 / 裸数字
    }
    if matches!(
        l.as_str(),
        "at" | "rot" | "旋转" | "内" | "外" | "内花键" | "外花键" | "孔" | "轴" | "m" | "z"
            | "b" | "db" | "模数" | "齿数" | "基准直径" | "直径" | "e2" | "ae" | "as" | "tactn"
            | "teffn" | "tactw" | "teffw"
    ) {
        return true;
    }
    if l.starts_with('n') || l.starts_with('w') {
        if t[1..].starts_with(|c: char| c.is_ascii_digit()) {
            return true;
        }
    }
    for p in ["m", "z", "b"] {
        if let Some(rest) = l.strip_prefix(p) {
            if rest.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return true;
            }
        }
    }
    false
}

pub fn usage() -> String {
    crate::i18n::t("cmd.din.usage")
}

/// 解析 `9H` / `8f` / `js` 这类配合 token。
pub fn parse_fit_token(t: &str, hub: bool) -> Result<DinFit, String> {
    let s = t.trim();
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return Err(crate::i18n::t("cmd.din.err.fit_grade_digits"));
    }
    let grade: u32 = digits
        .parse()
        .map_err(|_| crate::i18n::t_fmt("cmd.din.err.fit_grade_invalid", &[("digits", &digits)]))?;
    let letter = s[digits.len()..].trim().to_string();
    if letter.is_empty() {
        return Err(crate::i18n::t("cmd.din.err.fit_letter_missing"));
    }
    let letter = if hub { letter.to_ascii_uppercase() } else { letter.to_ascii_lowercase() };
    let fit = DinFit { grade, letter };
    fit.validate(hub)?;
    Ok(fit)
}

/// 把 `×`/`X` 统一成 `x`，去掉全角空格。
fn normalize_input(t: &str) -> String {
    t.replace('×', "x").replace('X', "x").replace('　', " ")
}

/// 解析 4 段长度代号体：`120x3x38x9H` → (dB, m, z, fit)。
fn parse_designation_body(body: &str) -> Result<(f64, f64, u32, String), String> {
    let b = body.trim().trim_start_matches(['N', 'n', 'W', 'w', 'A', 'I', 'a', 'i']);
    let parts: Vec<&str> = b.split('x').collect();
    if parts.len() != 4 {
        return Err(format!(
            "长度代号「{body}」应有 4 段 `d_B×m×z×等级字母`（如 `120×3×38×9H`）"
        ));
    }
    let d_b = csv_decimal(parts[0]).ok_or_else(|| format!("d_B「{}」不是数字", parts[0]))?;
    let m = csv_decimal(parts[1]).ok_or_else(|| format!("m「{}」不是数字", parts[1]))?;
    let z: u32 = parts[2]
        .trim()
        .parse()
        .map_err(|_| format!("z「{}」不是整数", parts[2]))?;
    let fit = parts[3].trim().to_string();
    if fit.is_empty() {
        return Err("等级字母段为空".to_string());
    }
    Ok((d_b, m, z, fit))
}

impl DinTableSpec {
    /// 解析 CLI 文本。
    pub fn parse(text: &str, hub: bool) -> Result<Self, String> {
        let norm = normalize_input(text);
        let tokens: Vec<&str> = norm.split_whitespace().collect();
        if tokens.is_empty() {
            return Err(usage());
        }
        let mut spec = DinTableSpec {
            m: 0.0,
            z: 0,
            d_b: 0.0,
            hub: DinFit { grade: 9, letter: "H".into() },
            shaft: DinFit { grade: 8, letter: "f".into() },
            e2_s1: None,
            ae: None,
            as_: None,
            tact_hub: None,
            teff_hub: None,
            tact_shaft: None,
            teff_shaft: None,
            at: None,
            rot: 0.0,
        };
        let mut have = (false, false, false); // m, z, d_b
        let mut pending: Option<bool> = None; // Some(true)=待接孔代号体
        // 表达式形态：`<九字段表达式> [N9H] [W8f] [e2 …] … [at x,y] [rot 度]`。
        let mut expr: Option<String> = None;
        let tokens = if let Some((e, rest)) = crate::card_expr::split_expr(&tokens, is_option_token) {
            expr = Some(e);
            rest
        } else {
            tokens
        };
        let mut i = 0usize;
        let need = |i: &mut usize, tokens: &[&str], what: &str| -> Result<String, String> {
            *i += 1;
            tokens
                .get(*i)
                .map(|s| s.to_string())
                .ok_or_else(|| crate::i18n::t_fmt("cmd.din.err.need", &[("what", what)]))
        };
        while i < tokens.len() {
            let t = tokens[i];
            let lower = t.to_ascii_lowercase();
            // 代号体（pending 或自带 N/W 前缀）
            if let Some(hub) = pending.take() {
                let Some((d_b, m, z, fit)) = parse_designation_body(t).ok() else {
                    return Err(crate::i18n::t_fmt(
                        "cmd.din.err.body_invalid",
                        &[("t", t), ("usage", &usage())],
                    ));
                };
                spec.d_b = d_b;
                spec.m = m;
                spec.z = z;
                have = (true, true, true);
                let f = parse_fit_token(&fit, hub)?;
                if hub { spec.hub = f; } else { spec.shaft = f; }
                i += 1;
                continue;
            }
            if lower == "n" || lower == "w" || lower == "内" || lower == "外" {
                pending = Some(lower == "n" || lower == "内");
                i += 1;
                continue;
            }
            // 形如 n120x3x38x9H / w120x3x38x8f（含 A/I 直径对中前缀也收下后忽略）
            if (lower.starts_with('n') || lower.starts_with('w'))
                && t[1..].contains('x')
                && t[1..].chars().next().is_some_and(|c| c.is_ascii_digit() || c == 'a' || c == 'i')
            {
                let hub = lower.starts_with('n');
                let body = &t[1..];
                // 跳过 A/I 前缀
                let body = body
                    .trim_start_matches(['A', 'I', 'a', 'i'])
                    .trim();
                let Some((d_b, m, z, fit)) = parse_designation_body(body).ok() else {
                    return Err(crate::i18n::t_fmt(
                        "cmd.din.err.code_invalid",
                        &[("t", t), ("usage", &usage())],
                    ));
                };
                spec.d_b = d_b;
                spec.m = m;
                spec.z = z;
                have = (true, true, true);
                let f = parse_fit_token(&fit, hub)?;
                if hub { spec.hub = f; } else { spec.shaft = f; }
                i += 1;
                continue;
            }
            let mut key = lower.clone();
            let mut inline: Option<String> = None;
            if let Some((k, v)) = t.split_once('=') {
                key = k.to_ascii_lowercase();
                inline = Some(v.to_string());
            }
            let value = |i: &mut usize, tokens: &[&str]| -> Result<String, String> {
                if let Some(v) = &inline {
                    if !v.is_empty() {
                        return Ok(v.clone());
                    }
                }
                *i += 1;
                tokens
                    .get(*i)
                    .map(|s| s.to_string())
                    .ok_or_else(|| {
                        crate::i18n::t_fmt("cmd.din.err.key_missing", &[("key", &key)])
                    })
            };
            match key.as_str() {
                "at" => {
                    let v = need(&mut i, &tokens, "at")?;
                    let (x, y) = v
                        .split_once(',')
                        .ok_or_else(|| {
                            crate::i18n::t_fmt("cmd.din.err.at_format", &[("v", &v)])
                        })?;
                    let x: f64 = x.trim().parse().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.at_x",
                            &[("x", x.trim()), ("e", &e.to_string())],
                        )
                    })?;
                    let y: f64 = y.trim().parse().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.at_y",
                            &[("y", y.trim()), ("e", &e.to_string())],
                        )
                    })?;
                    spec.at = Some([x, y]);
                }
                "rot" | "旋转" => {
                    let v = need(&mut i, &tokens, "rot")?;
                    spec.rot = v.parse().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.rot",
                            &[("v", &v), ("e", &e.to_string())],
                        )
                    })?;
                }
                "m" | "模数" => {
                    let v = value(&mut i, &tokens)?;
                    let body = v.trim_start_matches(['m', 'M']);
                    spec.m = body.parse().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.num",
                            &[("name", "m"), ("v", &v), ("e", &e.to_string())],
                        )
                    })?;
                    have.0 = true;
                }
                "z" | "齿数" => {
                    let v = value(&mut i, &tokens)?;
                    let body = v.trim_start_matches(['z', 'Z']);
                    spec.z = body.parse().map_err(|e: std::num::ParseIntError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.z_num",
                            &[("name", "z"), ("v", &v), ("e", &e.to_string())],
                        )
                    })?;
                    have.1 = true;
                }
                "b" | "db" | "基准直径" | "直径" => {
                    let v = value(&mut i, &tokens)?;
                    let body = v.trim_start_matches(['b', 'B']);
                    spec.d_b = body.parse().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.num",
                            &[("name", "d_B"), ("v", &v), ("e", &e.to_string())],
                        )
                    })?;
                    have.2 = true;
                }
                "n" | "内" | "内花键" | "孔" => {
                    let v = value(&mut i, &tokens)?;
                    spec.hub = parse_fit_token(&v, true)?;
                }
                "w" | "外" | "外花键" | "轴" => {
                    let v = value(&mut i, &tokens)?;
                    spec.shaft = parse_fit_token(&v, false)?;
                }
                "e2" => {
                    let v = value(&mut i, &tokens)?;
                    spec.e2_s1 = Some(v.parse().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.num",
                            &[("name", "e2"), ("v", &v), ("e", &e.to_string())],
                        )
                    })?);
                }
                "ae" => {
                    let v = value(&mut i, &tokens)?;
                    spec.ae = Some(v.parse().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.num",
                            &[("name", "ae"), ("v", &v), ("e", &e.to_string())],
                        )
                    })?);
                }
                "as" => {
                    let v = value(&mut i, &tokens)?;
                    spec.as_ = Some(v.parse().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.num",
                            &[("name", "as"), ("v", &v), ("e", &e.to_string())],
                        )
                    })?);
                }
                "tactn" => {
                    let v = value(&mut i, &tokens)?;
                    spec.tact_hub = Some(v.parse().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.num",
                            &[("name", "tactn"), ("v", &v), ("e", &e.to_string())],
                        )
                    })?);
                }
                "teffn" => {
                    let v = value(&mut i, &tokens)?;
                    spec.teff_hub = Some(v.parse().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.num",
                            &[("name", "teffn"), ("v", &v), ("e", &e.to_string())],
                        )
                    })?);
                }
                "tactw" => {
                    let v = value(&mut i, &tokens)?;
                    spec.tact_shaft = Some(v.parse().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.num",
                            &[("name", "tactw"), ("v", &v), ("e", &e.to_string())],
                        )
                    })?);
                }
                "teffw" => {
                    let v = value(&mut i, &tokens)?;
                    spec.teff_shaft = Some(v.parse().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.num",
                            &[("name", "teffw"), ("v", &v), ("e", &e.to_string())],
                        )
                    })?);
                }
                // 无 = 的短写法（M3 / Z38 / B120 / N9H / W8f）
                _ if key.starts_with('m') && key[1..].chars().all(|c| c.is_ascii_digit() || c == '.') => {
                    spec.m = key[1..].parse().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.num",
                            &[("name", "m"), ("v", t), ("e", &e.to_string())],
                        )
                    })?;
                    have.0 = true;
                }
                _ if key.starts_with('z') && key[1..].chars().all(|c| c.is_ascii_digit()) => {
                    spec.z = key[1..].parse().map_err(|e: std::num::ParseIntError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.z_num",
                            &[("name", "z"), ("v", t), ("e", &e.to_string())],
                        )
                    })?;
                    have.1 = true;
                }
                _ if (key.starts_with('b') || key.starts_with("db"))
                    && key.trim_start_matches("db").trim_start_matches('b').chars().all(|c| c.is_ascii_digit() || c == '.') =>
                {
                    let body = key.trim_start_matches("db").trim_start_matches('b');
                    spec.d_b = body.parse().map_err(|e: std::num::ParseFloatError| {
                        crate::i18n::t_fmt(
                            "cmd.din.err.num",
                            &[("name", "d_B"), ("v", t), ("e", &e.to_string())],
                        )
                    })?;
                    have.2 = true;
                }
                _ if key.starts_with('n') && key.len() > 1 => {
                    spec.hub = parse_fit_token(&t[1..], true)?;
                }
                _ if key.starts_with('w') && key.len() > 1 => {
                    spec.shaft = parse_fit_token(&t[1..], false)?;
                }
                _ => {
                    // 孔/轴配合也可用裸 token「9H」直接给（按字母大小写判侧：大写=孔、小写=轴）。
                    let digits: String =
                        t.chars().take_while(|c| c.is_ascii_digit()).collect();
                    let letter = t[digits.len()..].trim();
                    let parsed = if digits.is_empty() || letter.is_empty() {
                        Err(crate::i18n::t_fmt("cmd.din.err.unknown_param_bare", &[("t", t)]))
                    } else if letter.chars().all(|c| c.is_ascii_uppercase()) {
                        parse_fit_token(t, true).map(|f| (f, true))
                    } else if letter.chars().all(|c| c.is_ascii_lowercase()) {
                        parse_fit_token(t, false).map(|f| (f, false))
                    } else {
                        Err(crate::i18n::t_fmt("cmd.din.err.unknown_param_bare", &[("t", t)]))
                    };
                    match parsed {
                        Ok((f, true)) => spec.hub = f,
                        Ok((f, false)) => spec.shaft = f,
                        Err(e) => {
                            return Err(crate::i18n::t_fmt(
                                "cmd.din.err.wrapped",
                                &[("e", &e), ("usage", &usage())],
                            ))
                        }
                    }
                }
            }
            i += 1;
        }
        if pending.is_some() {
            return Err(crate::i18n::t_fmt(
                "cmd.din.err.pending_len",
                &[("usage", &usage())],
            ));
        }
        if let Some(e) = expr {
            let policy = if hub { &EXPR_POLICY_INT } else { &EXPR_POLICY_EXT };
            let r = crate::card_expr::resolve(policy, &e, Some(hub))?;
            let em = r.value("m").ok_or_else(|| {
                crate::i18n::t("cmd.din.err.map_missing_m")
            })?;
            let ez = r.value("z").ok_or_else(|| {
                crate::i18n::t("cmd.din.err.map_missing_z")
            })? as u32;
            let edb = r.value("d_b").ok_or_else(|| {
                crate::i18n::t("cmd.din.err.map_missing_db")
            })?;
            if have.0 && (spec.m - em).abs() > 1e-9 {
                return Err(crate::i18n::t_fmt(
                    "cmd.din.err.explicit_mismatch",
                    &[
                        ("name", "m"),
                        ("given", &trim3(spec.m)),
                        ("resolved", &trim3(em)),
                    ],
                ));
            }
            if have.1 && spec.z != ez {
                return Err(crate::i18n::t_fmt(
                    "cmd.din.err.explicit_mismatch",
                    &[
                        ("name", "z"),
                        ("given", &spec.z.to_string()),
                        ("resolved", &ez.to_string()),
                    ],
                ));
            }
            if have.2 && (spec.d_b - edb).abs() > 1e-3 {
                return Err(crate::i18n::t_fmt(
                    "cmd.din.err.explicit_mismatch",
                    &[
                        ("name", "d_B"),
                        ("given", &trim3(spec.d_b)),
                        ("resolved", &trim3(edb)),
                    ],
                ));
            }
            spec.m = em;
            spec.z = ez;
            spec.d_b = edb;
        } else if !have.0 || !have.1 || !have.2 {
            let what = crate::i18n::t(match have {
                (false, false, false) => "cmd.din.what.mzdb",
                (false, _, _) => "cmd.din.what.m",
                (_, false, _) => "cmd.din.what.z",
                _ => "cmd.din.what.db",
            });
            return Err(crate::i18n::t_fmt(
                "cmd.din.err.missing_fields",
                &[("what", &what), ("usage", &usage())],
            ));
        }
        derive(&spec)?;
        Ok(spec)
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 测试
// ══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn near6(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.5e-6
    }

    fn example() -> DinTableSpec {
        DinTableSpec::default()
    }

    /// Table 7 上段资产：18 行 × 9 列、阶梯倍数（±1 µm 容差）、两个实锚（c4=100–200 / c5=50–100）。
    #[test]
    fn table7_dev_asset_and_range_anchors() {
        let rows = table7_rows();
        assert_eq!(rows.len(), 18, "18 个 Abmaß 系列");
        let ladder = [200, 180, 160, 140, 125, 110, 100, 90, 80];
        let factors = [
            ("v", 1.0), ("u", 0.9), ("t", 0.8), ("s", 0.7), ("r", 0.6), ("p", 0.5),
            ("n", 0.4), ("m", 0.3), ("k", 0.2), ("js", 0.1), ("h", 0.0), ("g", -0.1),
            ("f", -0.2), ("e", -0.3), ("d", -0.4), ("c", -0.6), ("b", -0.8), ("a", -1.0),
        ];
        for (row, (letter, f)) in rows.iter().zip(factors.iter()) {
            assert_eq!(row.series, *letter);
            for (ci, l) in ladder.iter().enumerate() {
                let expect = (f * *l as f64).round() as i32;
                let got = row.values[ci].expect("162 格均无留空");
                // 阶梯第 5 档 125 的 .5 倍在本表不按统一舍入（u=112、s=88、p=62、m=37、js=12），
                // 以表值为准；其余格严格等于系列系数×阶梯。
                assert!(
                    (got - expect).abs() <= 1,
                    "系列 {letter} c{} 应 ≈ {expect}（±1），实为 {got}",
                    ci + 1
                );
            }
        }
        // 表值逐格示例（.5 档按表值）
        let get = |s: &str, ci: usize| dev_lookup(s, false).unwrap().values[ci];
        assert_eq!(get("v", 0), Some(200));
        assert_eq!(get("u", 4), Some(112));
        assert_eq!(get("s", 4), Some(88));
        assert_eq!(get("p", 4), Some(62));
        assert_eq!(get("m", 4), Some(37));
        assert_eq!(get("js", 4), Some(12));
        assert_eq!(get("a", 0), Some(-200));
        // 实锚 1：Bild 6 示例（dB=120, 8f）As = −28 µm ⇒ c4 = 100–200
        let f = dev_lookup("f", false).unwrap();
        assert_eq!(f.series, "f");
        assert_eq!(f.values[3], Some(-28));
        // 实锚 2：KISSsoft 样张（dB=70, 6f）As = −25 µm ⇒ c5 = 50–100
        assert_eq!(f.values[4], Some(-25));
        assert_eq!(range_for_d_b(120.0), Some((3, "100–200", "已核(锚)")));
        assert_eq!(range_for_d_b(70.0), Some((4, "50–100", "已核(锚)")));
        assert_eq!(range_for_d_b(30.0), Some((5, "25–50", "推断")));
        assert_eq!(range_for_d_b(300.0), Some((2, "200–400", "推断")));
        assert_eq!(range_for_d_b(5.0), Some((7, "≤12", "推断")));
        // >400 侧与 c1/c2/c9 不取（无实锚）：
        assert_eq!(range_for_d_b(500.0), None);
        assert!(dev_lookup_mm("f", 500.0, false).is_err());
        // 孔系列：F↔k、M↔e；孔 H 基本偏差 0
        assert_eq!(dev_lookup("F", true).unwrap().series, "k");
        assert_eq!(dev_lookup("M", true).unwrap().series, "e");
        assert_eq!(dev_lookup("F", false).unwrap().series, "f", "大写 F 在轴侧仍是 f");
        assert_eq!(dev_lookup_mm("H", 120.0, true).unwrap().0, 0.0);
    }

    /// Table 7 公差锚：6–9 级有值，10/11 级为待抽空行；模数组 1,75–4 外不可用。
    #[test]
    fn table7_tol_asset() {
        let rows = table7_tol_rows();
        assert_eq!(rows.len(), 6);
        let tol = |g: u32| rows.iter().find(|r| r.grade == g).unwrap().clone();
        assert!(near6(tol(6).tact.unwrap(), 0.018) && near6(tol(6).teff.unwrap(), 0.010));
        assert!(near6(tol(7).tact.unwrap(), 0.025) && near6(tol(7).teff.unwrap(), 0.015));
        assert!(near6(tol(8).tact.unwrap(), 0.040) && near6(tol(8).teff.unwrap(), 0.023));
        assert!(near6(tol(9).tact.unwrap(), 0.056) && near6(tol(9).teff.unwrap(), 0.034));
        for g in [10, 11] {
            assert!(tol(g).tact.is_none() && tol(g).teff.is_none(), "等级 {g} 待抽");
        }
        let (t, e, tg, n) = tol_lookup_mm(9, 3.0).unwrap();
        assert!(near6(t, 0.056) && near6(e, 0.034) && near6(tg, 0.090));
        assert!(n.contains("Table 7"));
        assert!(tol_lookup_mm(9, 7.5).is_err(), "模数组 5–10 未抽");
        assert!(tol_lookup_mm(10, 3.0).is_err(), "10 级未抽");
    }

    /// Bild 6 示例：26 项逐项等于标准原印值；公式路与锚点一致。
    #[test]
    fn bild6_example_reproduces_standard() {
        let spec = example();
        assert!(is_bild6_example(&spec));
        let d = derive(&spec).unwrap();
        assert!(d.anchor);
        // 六值（原印）
        assert!(near(d.e_max.unwrap(), 6.361));
        assert!(near(d.e_min.unwrap(), 6.305));
        assert!(near(d.e_vmin.unwrap(), 6.271));
        assert!(near(d.s_vmax.unwrap(), 6.243));
        assert!(near(d.s_max.unwrap(), 6.220));
        assert!(near(d.s_min.unwrap(), 6.180));
        // 反推自洽：e2=6.271、Ae=0、As=−0.028、Tact 9H=0.056、Teff 9H=0.034、8f=0.040/0.023
        assert!(near(d.e2_s1.unwrap(), 6.271));
        assert!(near(anchor_f64("meta", "Ae_hub").unwrap(), 0.0));
        assert!(near(anchor_f64("meta", "As_shaft").unwrap(), -0.028));
        assert!(near(d.tact_hub.unwrap(), 0.056));
        assert!(near(d.teff_hub.unwrap(), 0.034));
        assert!(near(d.tact_shaft.unwrap(), 0.040));
        assert!(near(d.teff_shaft.unwrap(), 0.023));
        // 公式路复算（显式覆盖 = 锚点反推值；不启用 anchor 路径）
        let mut calc = spec.clone();
        calc.ae = Some(0.0);
        calc.as_ = Some(-0.028);
        calc.tact_hub = Some(0.056);
        calc.teff_hub = Some(0.034);
        calc.tact_shaft = Some(0.040);
        calc.teff_shaft = Some(0.023);
        assert!(!is_bild6_example(&calc));
        let c = derive(&calc).unwrap();
        for (a, b) in [
            (c.e_max.unwrap(), 6.361),
            (c.e_min.unwrap(), 6.305),
            (c.e_vmin.unwrap(), 6.271),
            (c.s_vmax.unwrap(), 6.243),
            (c.s_max.unwrap(), 6.220),
            (c.s_min.unwrap(), 6.180),
        ] {
            assert!(near6(a, b), "公式路 {a} != {b}");
        }
        // 单栏取值（示例逐项；内/外各 13 项）
        let vh = values(&spec, true).unwrap();
        let getn = |tag: &str| vh.iter().find(|(t, _)| t == tag).unwrap().1.clone();
        assert_eq!(getn("N标记"), "N120×3×38×9H");
        assert_eq!(getn("N齿数"), "38");
        assert_eq!(getn("N压力角"), "30°");
        assert_eq!(getn("N齿根圆"), "120 +0.76");
        assert_eq!(getn("N齿根成形圆"), "119.49 min.");
        assert_eq!(getn("N齿顶圆"), "114 H11");
        assert_eq!(getn("N槽宽max"), "6.361");
        assert_eq!(getn("N槽宽min"), "6.305");
        assert_eq!(getn("N槽宽eff"), "6.271");
        assert_eq!(getn("N量圆"), "5.25");
        assert_eq!(getn("N量距max"), "109.266");
        assert_eq!(getn("N量距min"), "109.169");
        let vw = values(&spec, false).unwrap();
        let getw = |tag: &str| vw.iter().find(|(t, _)| t == tag).unwrap().1.clone();
        assert_eq!(getw("W标记"), "W120×3×38×8f");
        assert_eq!(getw("W齿数"), "38");
        assert_eq!(getw("W压力角"), "30°");
        assert_eq!(getw("W齿顶圆"), "119.40 h11");
        assert_eq!(getw("W齿根成形圆"), "113.91 max.");
        assert_eq!(getw("W齿根圆"), "113.4 -1.74");
        assert_eq!(getw("W齿厚svmax"), "6.243");
        assert_eq!(getw("W齿厚smax"), "6.22");
        assert_eq!(getw("W齿厚smin"), "6.18");
        assert_eq!(getw("W量圆"), "6");
        assert_eq!(getw("W量距max"), "126.017");
        assert_eq!(getw("W量距min"), "125.956");
        assert_eq!(vh.len(), 13, "单栏 13 项");
        assert_eq!(vw.len(), 13);
    }

    /// 一般路径：KISSsoft 示例件（m=3、dB=70、z=22、6f/7H）应命中 Table 7 实锚列并复算。
    #[test]
    fn general_path_kisssoft_row() {
        let spec = DinTableSpec {
            m: 3.0,
            z: 22,
            d_b: 70.0,
            hub: DinFit { grade: 7, letter: "H".into() },
            shaft: DinFit { grade: 6, letter: "f".into() },
            ..DinTableSpec::default()
        };
        let d = derive(&spec).unwrap();
        assert!(!d.anchor);
        assert!(near6(d.ae.unwrap(), 0.0));
        assert!(near6(d.as_.unwrap(), -0.025));
        assert!(near6(d.tact_hub.unwrap(), 0.025));
        assert!(near6(d.teff_hub.unwrap(), 0.015));
        assert!(near6(d.tact_shaft.unwrap(), 0.018));
        assert!(near6(d.teff_shaft.unwrap(), 0.010));
        // e2=s1 由名义表（m=3,dB=70,z=22）给出；名义表 3 位小数 = 5,117
        let n = d.nominal.as_ref().unwrap();
        assert!(near6(n.e2_s1, 5.117), "名义表 e2/s1 = {}", n.e2_s1);
        // 与 KISSsoft 表相差 ≤1e-3（名义表 3 位小数舍入所致）
        let near3 = |a: f64, b: f64| (a - b).abs() < 1.5e-3;
        assert!(near3(d.e_vmin.unwrap(), 5.1167));
        assert!(near3(d.e_min.unwrap(), 5.1317));
        assert!(near3(d.e_max.unwrap(), 5.1568));
        assert!(near3(d.s_vmax.unwrap(), 5.0920));
        assert!(near3(d.s_max.unwrap(), 5.0820));
        assert!(near3(d.s_min.unwrap(), 5.0640));
    }

    /// 标缺行为：>400 侧 Ae/As → 「—」；模数组 5–10 的 Tact/Teff → 「—」；未入检验表行 → M 值「—」。
    #[test]
    fn missing_behavior_is_explicit() {
        // dB=500（>400 侧）：H 查不到，derive 返回带缺口说明的 Ok（图仍能出，值显示「—」）
        let spec = DinTableSpec { d_b: 500.0, ..DinTableSpec::default() };
        let d = derive(&spec).unwrap();
        assert!(d.ae.is_none() && d.as_.is_none());
        assert!(d.notes.iter().any(|n| n.contains(">400")), "{:?}", d.notes);
        assert!(vals_has_missing(&spec, true, "N槽宽eff"));
        // 换用显式 Ae/As 后可出 e/s，但直径/M 值仍如实标缺
        let spec2 = DinTableSpec {
            d_b: 500.0,
            ae: Some(0.0),
            as_: Some(-0.05),
            ..DinTableSpec::default()
        };
        let d2 = derive(&spec2).unwrap();
        assert!(d2.nominal.is_none());
        assert!(d2.e2_s1.is_none() && d2.e_vmin.is_none(), "无名义表行 ⇒ e₂ 缺口，e/s 显示「—」");
        assert!(vals_has_missing(&spec2, true, "N齿根圆"));
        assert!(vals_has_missing(&spec2, true, "N量距max"));
        // 模数组 5–10 → 公差「—」
        let spec3 = DinTableSpec { m: 5.0, z: 16, d_b: 80.0, ..DinTableSpec::default() };
        let d3 = derive(&spec3).unwrap();
        assert!(d3.tact_hub.is_none() && d3.teff_hub.is_none());
        assert!(d3.tact_shaft.is_none() && d3.teff_shaft.is_none());
        assert!(d3.e_max.is_none() && d3.e_min.is_none());
        assert!(vals_has_missing(&spec3, true, "N槽宽max"));
        assert!(vals_has_missing(&spec3, false, "W齿厚svmax"));
        // 未知 (m,z,d_B) → 名义表缺口进 notes
        let spec4 = DinTableSpec { m: 3.0, z: 99, d_b: 120.0, ..DinTableSpec::default() };
        let d4 = derive(&spec4).unwrap();
        assert!(d4.nominal.is_none() && d4.e_vmin.is_none());
    }

    fn vals_has_missing(spec: &DinTableSpec, hub: bool, tag: &str) -> bool {
        values(spec, hub)
            .unwrap()
            .iter()
            .any(|(t, v)| t == tag && v == MISSING)
    }

    /// 单栏块结构（内/外 × 中/英）：18 线 + 14 MTEXT（标题 + 13 标签）+ 13 ATTDEF；全部 OCSM_GB；
    /// 标题/标签/值均不出框、标签不进值列（真字体 metrics；英文侧按内联 `\W` 压缩）；
    /// 覆盖示例/KISSsoft/缺口/表外。中文侧零内联码（自定版面本来装得下，与 NF 卡模板偏宽不同）。
    #[test]
    fn block_structure_and_no_text_overlap() {
        use crate::i18n::{set_lang, set_lang_auto, Lang};
        use crate::spline_table::text_extent_ttf;
        let _g = crate::global_state_test_lock();
        let specs = [
            example(),
            DinTableSpec {
                m: 3.0, z: 22, d_b: 70.0,
                hub: DinFit { grade: 7, letter: "H".into() },
                shaft: DinFit { grade: 6, letter: "f".into() },
                ..DinTableSpec::default()
            },
            DinTableSpec { m: 5.0, z: 16, d_b: 80.0, ..DinTableSpec::default() },
            DinTableSpec { d_b: 500.0, ..DinTableSpec::default() },
        ];
        for lang in [Lang::Zh, Lang::En] {
            set_lang(lang);
            for hub in [true, false] {
                let ents = block_entities(hub);
                let n_line = ents.iter().filter(|e| matches!(e, EntityType::Line(_))).count();
                let n_mtext = ents.iter().filter(|e| matches!(e, EntityType::MText(_))).count();
                let n_att = ents
                    .iter()
                    .filter(|e| matches!(e, EntityType::AttributeDefinition(_)))
                    .count();
                assert_eq!(n_line, 18, "15 横线 + 3 竖线（单栏）");
                assert_eq!(n_mtext, 14, "标题 + 13 标签（标记行现在也有标签格）");
                assert_eq!(n_att, 13, "标记 + 12 值属性");
                // 文字框（MTEXT：有效字宽 = 样式 0.7 × 内联 W；标题正中 / 标签左中）
                let mut boxes: Vec<(String, [f64; 4])> = Vec::new();
                for e in &ents {
                    match e {
                        EntityType::MText(m) => {
                            assert_eq!(m.style, "OCSM_GB");
                            let (inline, plain) = crate::nf_table::mtext_inline_wf(&m.value);
                            let w = text_extent_ttf(plain, m.height, 0.7 * inline);
                            let (x, y, h) = (m.insertion_point.x, m.insertion_point.y, m.height);
                            let b = match m.attachment_point {
                                AttachmentPoint::MiddleCenter => {
                                    [x - w / 2.0, y - h / 2.0, x + w / 2.0, y + h / 2.0]
                                }
                                AttachmentPoint::MiddleLeft => [x, y - h / 2.0, x + w, y + h / 2.0],
                                other => panic!("{lang:?} MTEXT {plain:?} 未覆盖的对齐 {other:?}"),
                            };
                            assert!(
                                b[0] >= LABEL_X - 1e-6 && b[2] <= W_TOTAL - LABEL_X + 1e-6,
                                "{lang:?} {hub} MTEXT {plain:?} 出框：{b:?}"
                            );
                            if m.height == TEXT_H {
                                assert!(
                                    b[2] <= W_LABEL - 0.5,
                                    "{lang:?} {hub} 标签 {plain:?} 进值列（右 {:.1} > {}）：{b:?}",
                                    b[2],
                                    W_LABEL - 0.5
                                );
                                if lang == Lang::Zh {
                                    assert_eq!(
                                        inline, 1.0,
                                        "中文侧 {hub} 标签 {plain:?} 不该有内联压缩（预算见 card_din_width_plan.py）"
                                    );
                                }
                                boxes.push((plain.to_string(), b));
                            } else if lang == Lang::En {
                                // 英文标题已到 407 宽的 98%（预算 W=0.98），确认真被压过
                                assert!(inline < 0.995, "英文标题 {plain:?} 应压缩（inline={inline}）");
                            }
                        }
                        EntityType::AttributeDefinition(a) => assert_eq!(a.text_style, "OCSM_GB"),
                        _ => {}
                    }
                }
                // 标签两两不叠（同行只有一条；跨行 40 行高 > 25 字高）
                for i in 0..boxes.len() {
                    for j in i + 1..boxes.len() {
                        let (a, b) = (boxes[i].1, boxes[j].1);
                        assert!(
                            !(a[0] < b[2] - 1e-9 && b[0] < a[2] - 1e-9
                                && a[1] < b[3] - 1e-9 && b[1] < a[3] - 1e-9),
                            "{lang:?} {hub} 标签 {:?} × {:?} 叠字",
                            boxes[i].0, boxes[j].0
                        );
                    }
                }
                // 值（ATTDEF 实体字宽 0.6）不出右框
                for spec in &specs {
                    let vals = values(spec, hub).unwrap();
                    for ad in attdefs(hub) {
                        let v = vals.iter().find(|(t, _)| t == &ad.tag).unwrap().1.clone();
                        let w = text_extent_ttf(&v, ad.height, ad.width_factor);
                        assert!(
                            ad.insertion_point.x + w < W_TOTAL - 0.5,
                            "{lang:?} {hub} 值「{v}」（{}）出右框 {:.1}",
                            ad.tag,
                            ad.insertion_point.x + w
                        );
                    }
                }
            }
        }
        set_lang_auto();
    }

    /// 标记行拆分（用户 2026-09-26）：标签格 `Nabe`/`Welle DIN 5480`（随语言）/ 值格代号体；
    /// 逐图元断言 MTEXT 标签位置与 ATTDEF 值位置分列，且全串代号仍可用于回执/元数据。
    #[test]
    fn din_marking_row_splits_label_and_value() {
        use crate::i18n::{set_lang, set_lang_auto, Lang};
        let _g = crate::global_state_test_lock();
        let spec = example();
        set_lang(Lang::Zh);
        assert_eq!(designation_label(true), "Nabe DIN 5480");
        assert_eq!(designation_label(false), "Welle DIN 5480");
        assert_eq!(designation_body(&spec, true), "N120×3×38×9H");
        assert_eq!(designation_body(&spec, false), "W120×3×38×8f");
        assert_eq!(designation(&spec, true), "Nabe DIN 5480 – N120×3×38×9H");
        assert_eq!(designation(&spec, false), "Welle DIN 5480 – W120×3×38×8f");
        for hub in [true, false] {
            let ents = block_entities(hub);
            let want_label = designation_label(hub);
            let m = ents
                .iter()
                .find_map(|e| match e {
                    EntityType::MText(m) if m.value == want_label => Some(m),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("缺标记行标签 {want_label}"));
            assert!(near(m.insertion_point.x, LABEL_X) && near(m.insertion_point.y, DESIG_Y));
            assert_eq!(m.style, "OCSM_GB");
            let tag = if hub { "N标记" } else { "W标记" };
            let ad = attdefs(hub)
                .into_iter()
                .find(|a| a.tag == tag)
                .unwrap();
            assert!(near(ad.insertion_point.x, VALUE_X) && near(ad.insertion_point.y, DESIG_Y));
            // 值格只放代号体（不是全串，也没有 en-dash 前缀）
            let vals = values(&spec, hub).unwrap();
            let v = vals.iter().find(|(t, _)| t == tag).unwrap().1.clone();
            assert_eq!(v, designation_body(&spec, hub));
            assert!(!v.contains("DIN 5480"), "值格不应重写标签前缀：{v}");
        }
        // 英文侧：标记行标签随语言，代号体（值，`N`/`W` 前缀）两语原样。
        set_lang(Lang::En);
        assert_eq!(designation_label(true), "Hub DIN 5480");
        assert_eq!(designation_label(false), "Shaft DIN 5480");
        assert_eq!(designation(&spec, true), "Hub DIN 5480 – N120×3×38×9H");
        assert_eq!(designation_body(&spec, false), "W120×3×38×8f");
        set_lang_auto();
    }

    /// 卡面随语言（内/外 × 中英）：14 条文字（标题 + 13 标签）条数相等、en 零汉字、符号原样、
    /// ATTDEF tag 不随语言、13 项值（标记号/数据/缺项）两语逐项相等（DIN 无枚举显示值）、
    /// 标题两语不同，`missing_keys()` 无 `card.din.*`。
    #[test]
    fn din_card_faces_switch_language_keeping_symbols_and_values() {
        use crate::i18n::{missing_keys, set_lang, set_lang_auto, Lang};
        let _g = crate::global_state_test_lock();
        let cjk = |s: &str| s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
        let mtexts = |ents: &[EntityType]| -> Vec<String> {
            ents.iter()
                .filter_map(|e| match e {
                    EntityType::MText(m) => {
                        Some(crate::nf_table::mtext_inline_wf(&m.value).1.to_string())
                    }
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
        let syms_hub: &[&str] = &[
            "z", "m", "α", "d_f2", "d_Ff2", "d_a2", "e_max", "e_min", "e_vmin", "D_M",
            "M2_max", "M2_min",
        ];
        let syms_shaft: &[&str] = &[
            "z", "m", "α", "d_a1", "d_Ff1", "d_f1", "s_vmax", "s_max", "s_min", "D_M",
            "M1_max", "M1_min",
        ];
        let spec = example();
        for hub in [true, false] {
            set_lang(Lang::Zh);
            let zh = mtexts(&block_entities(hub));
            let zh_tags = tags(&block_entities(hub));
            set_lang(Lang::En);
            let en = mtexts(&block_entities(hub));
            let en_tags = tags(&block_entities(hub));
            assert_eq!(zh.len(), 14, "标题 + 13 标签（内={hub}）");
            assert_eq!(en.len(), 14);
            assert_eq!(zh_tags, en_tags, "ATTDEF tag 不随语言（内={hub}）");
            assert!(!en.iter().any(|s| cjk(s)), "en 卡面零汉字（内={hub}）：{en:?}");
            set_lang(Lang::Zh);
            for r in ROWS {
                let want = row_label(r, hub);
                assert!(zh.iter().any(|s| s == &want), "zh 卡面缺「{want}」（内={hub}）：{zh:?}");
            }
            set_lang(Lang::En);
            for r in ROWS {
                let want = row_label(r, hub);
                assert!(en.iter().any(|s| s == &want), "en 卡面缺「{want}」（内={hub}）：{en:?}");
            }
            for s in if hub { syms_hub } else { syms_shaft } {
                assert!(
                    zh.iter().any(|t| t.ends_with(*s)),
                    "zh 符号 {s} 原样（内={hub}）：{zh:?}"
                );
                assert!(
                    en.iter().any(|t| t.ends_with(*s)),
                    "en 符号 {s} 原样（内={hub}）：{en:?}"
                );
            }
            // 值：标记号/数据/缺项两语逐项相等（DIN 无枚举显示值）
            set_lang(Lang::Zh);
            let zh_vals = values(&spec, hub).unwrap();
            set_lang(Lang::En);
            let en_vals = values(&spec, hub).unwrap();
            assert_eq!(zh_vals.len(), 13, "标记 + 12 值（内={hub}）");
            assert_eq!(zh_vals, en_vals, "13 项值两语逐项相等（内={hub}）");
            assert_eq!(zh_vals[0].1, designation_body(&spec, hub));
            // 标题两语不同
            set_lang(Lang::Zh);
            let zh_title = mtexts(&block_entities(hub))[0].clone();
            set_lang(Lang::En);
            let en_title = mtexts(&block_entities(hub))[0].clone();
            assert_ne!(zh_title, en_title, "标题随语言（内={hub}）");
            assert_eq!(en_title, title_text(hub));
        }
        // ★ 只查**本批 DIN 键**未漏（`missing_keys()` 是进程全局，其它并行用例会故意点
        //   `no.such.key.at.all` / `data:…` ⇒ 不能断言全局为空，否则偶发假红）。
        let missed = missing_keys();
        assert!(
            !missed.iter().any(|k| k.starts_with("card.din.")),
            "DIN 键有漏：{missed:?}"
        );
        set_lang_auto();
    }

    /// CLI：两种写法（短参数 / 代号 / 混合）与覆盖项；内/外两张卡各解析；报错指路。
    #[test]
    fn cli_parse_forms() {
        let a = DinTableSpec::parse("M3 Z38 B120", true).unwrap();
        assert_eq!(a, DinTableSpec { m: 3.0, z: 38, d_b: 120.0, ..DinTableSpec::default() });
        let b = DinTableSpec::parse("N120×3×38×9H W120×3×38×8f", true).unwrap();
        assert_eq!(b.m, 3.0);
        assert_eq!(b.d_b, 120.0);
        assert_eq!(b.hub.token(), "9H");
        assert_eq!(b.shaft.token(), "8f");
        let c = DinTableSpec::parse("N 120x3x38x9H W 120x3x38x8f at 10,20 rot 30", true).unwrap();
        assert_eq!(c.at, Some([10.0, 20.0]));
        assert_eq!(c.rot, 30.0);
        let d = DinTableSpec::parse("模数 3 齿数 38 基准直径 120 内花键 9H 外花键 8f", true).unwrap();
        assert_eq!(d.hub.token(), "9H");
        // 旧写法在**外卡**同样可用（只取本侧 W8f；内配合保留但不渲染）。
        let de = DinTableSpec::parse("M3 Z38 B120 W8f", false).unwrap();
        assert_eq!(de.shaft.token(), "8f");
        let e = DinTableSpec::parse("M3 Z38 B120 ae=0 as=-0.028 tactn=0.056 teffn=0.034", true).unwrap();
        assert_eq!(e.ae, Some(0.0));
        assert!(near(e.as_.unwrap(), -0.028));
        // 报错
        assert!(DinTableSpec::parse("M3 Z38", true).unwrap_err().contains("基准直径"));
        assert!(DinTableSpec::parse("M3 Z38 B120 N9v", true).unwrap_err().contains("偏差系列"));
        assert!(DinTableSpec::parse("M3 Z38 B120 W9L", true).unwrap_err().contains("偏差系列"));
        assert!(DinTableSpec::parse("M3 Z38 B120 foo", true).unwrap_err().contains("不认识的参数"));
        assert!(DinTableSpec::parse("", true).unwrap_err().contains("用法"));
    }

    /// GUI 模型：单栏 13 项预览 / 标缺 / 内卡与外卡各自方向。
    #[test]
    fn model_preview_json() {
        let m: DinTableModel = serde_json::from_value(serde_json::json!({
            "card": "DIN花键参数表", "side": "int", "m": 3, "z": 38, "d_b": 120,
            "hub": "9H", "shaft": "8f", "at": null, "rot": 0
        }))
        .unwrap();
        let j = m.preview_json().unwrap();
        assert_eq!(j["ok"], true);
        assert_eq!(j["renderer"], "din_table");
        assert_eq!(j["side"], "int");
        assert_eq!(j["anchor"], true);
        assert_eq!(j["items"].as_array().unwrap().len(), 13, "单栏 13 项");
        assert_eq!(j["missing"].as_array().unwrap().len(), 0);
        assert!(j["readout"].as_array().unwrap().len() >= 10);
        assert!(j["title"].as_str().unwrap().contains("内花键"));
        // 无 side 字段的旧请求 → 缺省内卡（旧 id 指向默认方向）
        let m0: DinTableModel = serde_json::from_value(serde_json::json!({
            "m": 3, "z": 38, "d_b": 120, "hub": "9H", "shaft": "8f"
        }))
        .unwrap();
        assert!(m0.hub());
        assert_eq!(m0.preview_json().unwrap()["items"].as_array().unwrap().len(), 13);
        // 外卡（side=ext）：只出 Welle 13 项；KIND=IN 的表达式报错（固定方向仍可报）
        let mut me = m.clone();
        me.side = Some("ext".into());
        let je = me.preview_json().unwrap();
        assert_eq!(je["side"], "ext");
        assert_eq!(je["items"][0]["tag"], "W标记");
        assert_eq!(je["items"].as_array().unwrap().len(), 13);
        assert!(je["title"].as_str().unwrap().contains("外花键"));
        // 标缺路径
        let m2: DinTableModel = serde_json::from_value(serde_json::json!({
            "side": "int", "m": 5, "z": 16, "d_b": 80, "hub": "9H", "shaft": "8f"
        }))
        .unwrap();
        let j2 = m2.preview_json().unwrap();
        assert!(!j2["missing"].as_array().unwrap().is_empty());
        assert!(j2["missing_note"].as_str().unwrap().contains("Table 7"));
    }

    /// 代号与用法串（内外两张卡、单栏 Bild 6）。
    #[test]
    fn designation_and_usage() {
        let spec = example();
        assert_eq!(designation(&spec, true), "Nabe DIN 5480 – N120×3×38×9H");
        assert_eq!(designation(&spec, false), "Welle DIN 5480 – W120×3×38×8f");
        let u = usage();
        assert!(u.contains("DIN花键参数表"));
        assert!(u.contains("Bild 6"));
        assert!(u.contains("0.17"), "用法要写整表缩放");
        assert_eq!(block_name(true), "OCSM_DINTABLE_DIN_INT");
        assert_eq!(block_name(false), "OCSM_DINTABLE_DIN_EXT");
        assert!(table7_rows().iter().all(|r| r.values.len() == 9));
    }

    /// INSERT（内/外）：13 个 ATTRIB、层与旋转、**整表缩放 0.17**（ATTRIB 随缩放）。
    #[test]
    fn insert_attributes() {
        let spec = example();
        for hub in [true, false] {
            let ins = build_insert(&spec, hub, [5.0, 7.0], 15.0).unwrap();
            assert_eq!(ins.attributes.len(), 13);
            assert_eq!(ins.block_name, block_name(hub));
            assert!(near(ins.rotation.to_degrees(), 15.0));
            assert!(near(ins.x_scale(), TABLE_SCALE) && near(ins.y_scale(), TABLE_SCALE));
            let first = if hub { "N标记" } else { "W标记" };
            assert_eq!(ins.attributes[0].tag, first);
            assert!(ins.attributes[0].value.contains(if hub { "N120×3×38×9H" } else { "W120×3×38×8f" }));
            // ATTRIB 随 INSERT 缩放：字高 = 块内 25 × 0.17；位置从基点 + 局部×0.17
            assert!(near6(ins.attributes[0].height, TEXT_H * TABLE_SCALE));
            let flat = build_insert(&spec, hub, [5.0, 7.0], 0.0).unwrap();
            assert!(near6(flat.attributes[0].insertion_point.x, 5.0 + VALUE_X * TABLE_SCALE));
            let last_tag = if hub { "N量距min" } else { "W量距min" };
            assert_eq!(ins.attributes[12].tag, last_tag);
        }
        let model = DinTableModel {
            card: "DIN花键参数表".into(),
            side: Some("int".into()),
            expr: None,
            m: 3.0,
            z: 38,
            d_b: 120.0,
            hub: Some("9H".into()),
            shaft: Some("8f".into()),
            e2: None,
            ae: None,
            as_: None,
            tact_n: None,
            teff_n: None,
            tact_w: None,
            teff_w: None,
            at: Some([0.0, 0.0]),
            rot: 0.0,
        };
        let echo = model.echo_note().unwrap();
        assert!(echo.contains("Bild 6 示例原印值"), "{echo}");
        assert!(echo.contains("Nabe") && !echo.contains("Welle"), "单栏只报本侧：{echo}");
        assert!(model.part_meta_json().unwrap().contains("din_table"));
        assert!(model.pending_attrs().unwrap().len() == 13);
        assert!(markdown_table(&spec, true).unwrap().contains("N槽宽max"));
        assert!(markdown_table(&spec, false).unwrap().contains("W齿厚svmax"));
    }

    fn expr_model(expr: &str, side: &str) -> DinTableModel {
        DinTableModel {
            card: if side == "int" { "DIN花键参数表" } else { "DIN花键参数表_外" }.into(),
            side: Some(side.into()),
            expr: Some(expr.into()),
            m: 0.0,
            z: 0,
            d_b: 0.0,
            hub: None,
            shaft: None,
            e2: None,
            ae: None,
            as_: None,
            tact_n: None,
            teff_n: None,
            tact_w: None,
            teff_w: None,
            at: None,
            rot: 0.0,
        }
    }

    /// ★ 表达式 → 字段 → 卡内值：d_B=m(z+1.1+2x)、m、z；内/外两张卡各自仅出本侧 13 项。
    #[test]
    fn din_expr_maps_to_base_diameter_and_anchor() {
        let spec = expr_model("SPLINE IN M3 Z38 ALPHA30 X0.45 DA120 DF114 BETA0 H30", "int")
            .spec()
            .unwrap();
        assert!(near(spec.d_b, 120.0), "d_B={}", spec.d_b);
        assert!(near(spec.m, 3.0));
        assert_eq!(spec.z, 38);
        assert!(is_bild6_example(&spec), "表达式反解 d_B=120 → 走 Bild 6 锚点");
        let vh = values(&spec, true).unwrap();
        assert!(vh.iter().find(|(t, _)| t == "N标记").unwrap().1.contains("N120×3×38×9H"));
        assert_eq!(vh.iter().find(|(t, _)| t == "N槽宽max").unwrap().1, "6.361");
        let vw = values(&spec, false).unwrap();
        assert!(vw.iter().find(|(t, _)| t == "W标记").unwrap().1.contains("W120×3×38×8f"));
        assert_eq!(vw.iter().find(|(t, _)| t == "W齿厚svmax").unwrap().1, "6.243");
        // 与旧 M/Z/B 输入同值（各侧）
        let old = DinTableSpec::parse("M3 Z38 B120 N9H W8f", true).unwrap();
        assert_eq!(values(&old, true).unwrap(), values(&spec, true).unwrap());
        assert_eq!(values(&old, false).unwrap(), values(&spec, false).unwrap());
        // 预览回填 fields + expr 回显 + 本侧 13 项
        let pj = expr_model("SPLINE IN M3 Z38 ALPHA30 X0.45 DA120 DF114 BETA0 H30", "int")
            .preview_json()
            .unwrap();
        assert!(near(pj["fields"]["d_b"].as_f64().unwrap(), 120.0));
        assert!(near(pj["fields"]["m"].as_f64().unwrap(), 3.0));
        assert_eq!(pj["fields"]["z"], 38);
        assert!(pj["expr"].as_str().unwrap().starts_with("SPLINE IN"));
        assert_eq!(pj["items"].as_array().unwrap().len(), 13);
    }

    /// CLI：完整行/短表达式（缺 DA/DF/BETA/H）两种都能截；旧写法保留；
    /// 一卡一方向：KIND 与卡方向不符时报错（不静默）。
    #[test]
    fn din_cli_expr_form_and_errors() {
        let cli = DinTableSpec::parse(
            "SPLINE IN M3 Z38 ALPHA30 X0.45 DA120 DF114 BETA0 H30 N9H W8f",
            true,
        )
        .unwrap();
        assert!(near(cli.d_b, 120.0) && near(cli.m, 3.0) && cli.z == 38);
        assert_eq!(cli.hub.token(), "9H");
        assert_eq!(cli.shaft.token(), "8f");
        // 短表达式：到 X 就结束，后面直接跟 W8f（无 DA/DF/BETA/H）
        let short = DinTableSpec::parse("SPLINE IN M3 Z38 ALPHA30 X0.45 W8f", true).unwrap();
        assert!(near(short.d_b, 120.0));
        assert_eq!(short.shaft.token(), "8f");
        // 外卡：KIND EX 正常；写 IN 报错（保留报错能力）
        let ex = DinTableSpec::parse(
            "SPLINE EX M3 Z38 ALPHA30 X0.45 DA119.4 DF113.4 BETA0 H30",
            false,
        )
        .unwrap();
        assert!(near(ex.d_b, 120.0));
        let e = DinTableSpec::parse(
            "SPLINE IN M3 Z38 ALPHA30 X0.45 DA120 DF114 BETA0 H30",
            false,
        )
        .unwrap_err();
        assert!(e.contains("KIND") && e.contains("「外」"), "{e}");
        let e = DinTableSpec::parse(
            "SPLINE EX M3 Z38 ALPHA30 X0.45 DA119.4 DF113.4 BETA0 H30",
            true,
        )
        .unwrap_err();
        assert!(e.contains("KIND") && e.contains("「内」"), "{e}");
        // 显式 d_B 与表达式反解不一致 → 报错
        let e = DinTableSpec::parse(
            "SPLINE IN M3 Z38 ALPHA30 X0.45 DA120 DF114 BETA0 H30 B100",
            true,
        )
        .unwrap_err();
        assert!(e.contains("显式 d_B=100") && e.contains("d_B=120"), "{e}");
        // α 必须 30
        let e = DinTableSpec::parse("SPLINE IN M3 Z38 ALPHA20 X0 BETA0 H30", true).unwrap_err();
        assert!(e.contains("压力角 20°") && e.contains("α=30"), "{e}");
        // MARK 体系不符
        let e = expr_model("GEAR IN M3 Z38 ALPHA30 X0.45 BETA0 H30", "int")
            .spec()
            .unwrap_err();
        assert!(e.contains("MARK") && e.contains("GEAR（齿轮）"), "{e}");
        // 外卡的错误前缀 = 外花键卡名
        let e = expr_model("GEAR EX M3 Z38 ALPHA30 X0.45 BETA0 H30", "ext")
            .spec()
            .unwrap_err();
        assert!(e.contains("DIN 5480 外花键参数表"), "{e}");
    }
}
