//! GB/T 3478.1-2008 渐开线花键（公差/偏差/几何）**数据 + 公式层**。
//!
//! XL「花键参数表」（GB 内/外）阶段 1：只做数据与公式，不做 GUI/XL 命令。
//! 手填项只有三个（用户 2026-09-25 裁定）：① 公差等级和配合类别（4/5/6/7 + H/d/e/f/h/js/k）；
//! ② 量棒直径 `Dp`（可填；不填则按标准 R40 规则自动选）；③ 从 GEAR/花键段继承的 `m/z/αD` 等。
//! 其余全部自动算，**表外不插值不外推**（明确报错）。
//!
//! ## 数据来源（逐条落盘，见 `assets/spline_gb3478_*.csv` 的 source/note）
//!
//! - 表 23/24/25、配合类别、公式表：嘉立创 FA 机械设计手册「花键连接」章（`jlc_5-3-50/51/52/53`、
//!   `jlc_new05260`、`jlc_new05268`），与 `review/花键公差_资料/` 的 OCR/HTML 交叉核对。
//! - 量棒公式（式(1)~(12)）与 inv 常数：**国家标准全文公开系统 openstd 在线预览**（真浏览器抓页，
//!   `3478.6/7/8_在线预览_p*.png`）；hcno 见 `review/花键公差_资料/花键章节_导航_嘉立创.md`。
//! - `(T+λ)/λ/Fp/Fα` 公式与 4/5/6/7 级系数：GB/T 3478.1 p17/p18 页图 + `jlc_5-3-46`。
//! - 齿根圆弧最小曲率半径系数：GB/T 3478.1 **图 2（p07 a) / p08 b–d)）**（30°平 0.2m、30°圆 0.4m、
//!   37.5° 0.3m、45° 0.25m）；§8.7.5 指 **表 26**（doc88 p51＝书页 50，逐模数列表，已抓）——本库按图 2 系数乘 m
//!   （用户口径：R_imin 是计算量）；表 26 同值并存 `assets/spline_gb3478_table26_rimin.csv`（口径/缺口见文件头）。
//! - IT/H 公差（表 25）与 GB/T 1800 一致，复用本仓 `tolerance` 模块的 IT 表；表外尺寸按 GB/T 1800。
//!
//! ## 口径备忘
//!
//! - `i_d/D`、`i_E/E`、`i_S/S` 单位 μm（D/E/S 单位 mm）；(T+λ) 内花键用 i_E、外花键用 i_S。
//! - λ 用**未修约**的 Fp/Fα/Fβ 算；Fβ 的花键长度 g 省略 = 分度圆直径一半（标准表 7~21 的同口径）。
//! - 公差/偏差一律修约到整数 μm（与标准表 7~25 一致）；几何尺寸保留 mm。
//! - D 区间按「lo 开 hi 闭」匹配；D>1000 或 m 不在 0.25…10 或等级不在 4/5/6/7 → 报错。
//! - 量棒：标准只规定 **R40 取最接近的较大值（1 个）**；`dp_candidates_3` 另给「3 个工程备选」。
//! - 表 24 脚注①：**外花键大径 Dee 的上偏差取 0**；表值用于小径 Die。
//! - 表 3 注 4：CF=0.1m 仅 H/h；其它配合类别齿形裕度会变——**全 70 页无列值**（p07 §5.6 与附录 C 示例
//!   也只用 0.1m）→ 本库按 0.1m 并注明“标准未列值”（不臆造）。
//! - 表 25 的 Dii 列在原 HTML 里是图片（5-3-51.files/image0xx.gif，已核对为 `+IT/0` 叠加格）；
//!   本库按 H10/H11/H12 = 下偏差 0、上偏差 +IT10/11/12 实现，与 `tolerance` 的 IT 表逐格对得上。

//! （阶段 1 只入数据+公式；阶段 2 的 GUI/XL 命令接入前，模块内暂有未调用项。）
#![allow(dead_code)]

use crate::tolerance;

// ══════════════════════════════════════════════════════════════════════════
// 枚举与输入
// ══════════════════════════════════════════════════════════════════════════

/// 标准压力角 αD（GB/T 3478.1 规定 30°/37.5°/45°）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureAngle {
    A30,
    A37_5,
    A45,
}

impl PressureAngle {
    pub fn deg(self) -> f64 {
        match self {
            PressureAngle::A30 => 30.0,
            PressureAngle::A37_5 => 37.5,
            PressureAngle::A45 => 45.0,
        }
    }
    /// 压力角（弧度）。
    pub fn rad(self) -> f64 {
        self.deg().to_radians()
    }
    /// `inv αD`（标准正文常数：30°=0.0537515、37.5°=0.1128285、45°=0.2146018）。
    pub fn inv(self) -> f64 {
        match self {
            PressureAngle::A30 => 0.053_751_5,
            PressureAngle::A37_5 => 0.112_828_5,
            PressureAngle::A45 => 0.214_601_8,
        }
    }
    /// 解析「30 / 37.5 / 45」（含「30°」写法）。
    pub fn parse(text: &str) -> Result<Self, String> {
        match text.trim().trim_end_matches('°') {
            "30" => Ok(PressureAngle::A30),
            "37.5" => Ok(PressureAngle::A37_5),
            "45" => Ok(PressureAngle::A45),
            other => Err(crate::i18n::t_fmt(
                "cmd.spline.err.alpha_invalid",
                &[("other", other)],
            )),
        }
    }
}

/// 齿根型式：平齿根（仅 30°）/ 圆齿根。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootForm {
    Flat,
    Fillet,
}

/// 内/外花键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplineSide {
    Internal,
    External,
}

/// 外花键基本偏差（基孔制：内花键恒 H）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtDev {
    D,
    E,
    F,
    H,
    Js,
    K,
}

impl ExtDev {
    /// 记号（`d`/`e`/`f`/`h`/`js`/`k`）。
    pub fn code(self) -> &'static str {
        match self {
            ExtDev::D => "d",
            ExtDev::E => "e",
            ExtDev::F => "f",
            ExtDev::H => "h",
            ExtDev::Js => "js",
            ExtDev::K => "k",
        }
    }
    /// 解析配合类别（`H/k`、`6H`、`5f`、`js`…）。
    pub fn parse(text: &str) -> Result<Self, String> {
        let t = text.trim().to_ascii_lowercase();
        // `H/k` 去掉孔侧 H（**只去 `h/` 这两字符**）；单独一个 `h` 是外花键基本偏差 H，
        // 不能被同一个 strip_prefix('h') 误删成空串。
        let t = t.strip_prefix("h/").unwrap_or(&t);
        let t = t.trim_start_matches('/');
        match t {
            "d" => Ok(ExtDev::D),
            "e" => Ok(ExtDev::E),
            "f" => Ok(ExtDev::F),
            "h" => Ok(ExtDev::H),
            "js" => Ok(ExtDev::Js),
            "k" => Ok(ExtDev::K),
            other => Err(crate::i18n::t_fmt(
                "cmd.spline.err.fit_invalid",
                &[("other", other)],
            )),
        }
    }
}

/// 输入：手填只有 ① 公差等级+配合类别 ② Dp（可空）③ 从齿形段继承的 m/z/αD/齿根。
#[derive(Debug, Clone, Copy)]
pub struct SplineInput {
    pub m: f64,
    pub z: u32,
    pub alpha: PressureAngle,
    pub root: RootForm,
    pub side: SplineSide,
    /// 公差等级 4/5/6/7（齿槽宽/齿厚总公差那一套）。
    pub grade: u32,
    /// 外花键基本偏差；内花键忽略（恒 H）。
    pub ext_dev: ExtDev,
    /// 配合长度 g（mm，Fβ 用）；`None` = 分度圆直径一半（标准表口径）。
    pub fit_length: Option<f64>,
    /// 量棒直径 Dp（mm）；`None` = 按标准公式 + R40 自动选。
    pub dp: Option<f64>,
}

impl SplineInput {
    /// 配合类别记号（内花键 `6H`、外花键 `5f`）。
    pub fn grade_fit_label(&self) -> String {
        match self.side {
            SplineSide::Internal => format!("{}H", self.grade),
            SplineSide::External => format!("{}{}", self.grade, self.ext_dev.code()),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 数据（表 23/24/25 / 配合 / 量棒系列）
// ══════════════════════════════════════════════════════════════════════════

const EV_SV_CSV: &str = include_str!("../assets/spline_gb3478_ev_sv.csv");
const EXT_DIA_DEV_CSV: &str = include_str!("../assets/spline_gb3478_ext_dia_dev.csv");
const LIMITS_CSV: &str = include_str!("../assets/spline_gb3478_limits.csv");
const FIT_CSV: &str = include_str!("../assets/spline_gb3478_fit.csv");
const PIN_SERIES_CSV: &str = include_str!("../assets/spline_gb3478_pin_series.csv");
const RIMIN_T26_CSV: &str = include_str!("../assets/spline_gb3478_table26_rimin.csv");

/// 表 23 一行（D 区间 + d/e/f 的 esv 值 μm）。
#[derive(Debug, Clone, Copy)]
pub struct EvSvRow {
    pub d_lo: f64,
    pub d_hi: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

/// 表 24 一行（D 区间 + d/e/f × 30°/37.5°/45° 的 esv/tanαD 值 μm）。
#[derive(Debug, Clone, Copy)]
pub struct ExtDiaDevRow {
    pub d_lo: f64,
    pub d_hi: f64,
    pub d: [f64; 3],
    pub e: [f64; 3],
    pub f: [f64; 3],
}

/// 表 25 一行（印刷格；`None` = 原表未印，按 IT 表补）。
#[derive(Debug, Clone, Copy)]
pub struct LimitsRow {
    pub d_lo: f64,
    pub d_hi: f64,
    pub it10: Option<f64>,
    pub it11: Option<f64>,
    pub it12: Option<f64>,
}

/// 配合类别行。
#[derive(Debug, Clone)]
pub struct FitRow {
    pub fit: String,
    pub ext_dev: ExtDev,
    pub preferred_45: bool,
    /// CSV 的 memo 列（口径说明；GUI 选项表的 title 直接用）。
    pub memo: String,
}

fn parse_num(v: &str, what: &str) -> Result<f64, String> {
    v.trim()
        .parse::<f64>()
        .map_err(|e| {
            crate::i18n::t_fmt(
                "cmd.spline.err.num",
                &[("what", what), ("v", v), ("e", &e.to_string())],
            )
        })
}

fn parse_opt(v: &str) -> Option<f64> {
    let t = v.trim();
    if t.is_empty() {
        None
    } else {
        t.parse().ok()
    }
}

fn num_lines(csv: &'static str) -> impl Iterator<Item = &'static str> {
    csv.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
}

/// 表 23 全表（升序）。
pub fn ev_sv_rows() -> Result<Vec<EvSvRow>, String> {
    let mut rows = Vec::new();
    let mut lines = num_lines(EV_SV_CSV);
    let header = lines.next().ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.spline.err.csv_header_missing",
            &[("file", "spline_gb3478_ev_sv.csv")],
        )
    })?;
    if header.trim() != "d_lo,d_hi,esv_d,esv_e,esv_f" {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.csv_header_bad",
            &[("file", "spline_gb3478_ev_sv.csv"), ("header", header)],
        ));
    }
    for line in lines {
        let v: Vec<&str> = line.split(',').collect();
        if v.len() != 5 {
            return Err(crate::i18n::t_fmt(
                "cmd.spline.err.csv_cols",
                &[("file", "spline_gb3478_ev_sv.csv"), ("line", line)],
            ));
        }
        let lo = if v[0].trim().is_empty() {
            f64::NEG_INFINITY
        } else {
            parse_num(v[0], "d_lo")?
        };
        rows.push(EvSvRow {
            d_lo: lo,
            d_hi: parse_num(v[1], "d_hi")?,
            d: parse_num(v[2], "esv_d")?,
            e: parse_num(v[3], "esv_e")?,
            f: parse_num(v[4], "esv_f")?,
        });
    }
    Ok(rows)
}

/// 表 24 全表。
pub fn ext_dia_dev_rows() -> Result<Vec<ExtDiaDevRow>, String> {
    let mut rows = Vec::new();
    let mut lines = num_lines(EXT_DIA_DEV_CSV);
    let header = lines.next().ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.spline.err.csv_header_missing",
            &[("file", "spline_gb3478_ext_dia_dev.csv")],
        )
    })?;
    if header.trim() != "d_lo,d_hi,d30,d375,d45,e30,e375,e45,f30,f375,f45" {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.csv_header_bad",
            &[("file", "spline_gb3478_ext_dia_dev.csv"), ("header", header)],
        ));
    }
    for line in lines {
        let v: Vec<&str> = line.split(',').collect();
        if v.len() != 11 {
            return Err(crate::i18n::t_fmt(
                "cmd.spline.err.csv_cols",
                &[("file", "spline_gb3478_ext_dia_dev.csv"), ("line", line)],
            ));
        }
        let n = |i: usize, what: &str| parse_num(v[i], what);
        let lo = if v[0].trim().is_empty() {
            f64::NEG_INFINITY
        } else {
            parse_num(v[0], "d_lo")?
        };
        rows.push(ExtDiaDevRow {
            d_lo: lo,
            d_hi: parse_num(v[1], "d_hi")?,
            d: [n(2, "d30")?, n(3, "d375")?, n(4, "d45")?],
            e: [n(5, "e30")?, n(6, "e375")?, n(7, "e45")?],
            f: [n(8, "f30")?, n(9, "f375")?, n(10, "f45")?],
        });
    }
    Ok(rows)
}

/// 表 25 全表（印刷格）。
pub fn limits_rows() -> Result<Vec<LimitsRow>, String> {
    let mut rows = Vec::new();
    let mut lines = num_lines(LIMITS_CSV);
    let header = lines.next().ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.spline.err.csv_header_missing",
            &[("file", "spline_gb3478_limits.csv")],
        )
    })?;
    if header.trim() != "d_lo,d_hi,it10,it11,it12" {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.csv_header_bad",
            &[("file", "spline_gb3478_limits.csv"), ("header", header)],
        ));
    }
    for line in lines {
        let v: Vec<&str> = line.split(',').collect();
        if v.len() != 5 {
            return Err(crate::i18n::t_fmt(
                "cmd.spline.err.csv_cols",
                &[("file", "spline_gb3478_limits.csv"), ("line", line)],
            ));
        }
        let lo = if v[0].trim().is_empty() {
            f64::NEG_INFINITY
        } else {
            parse_num(v[0], "d_lo")?
        };
        rows.push(LimitsRow {
            d_lo: lo,
            d_hi: parse_num(v[1], "d_hi")?,
            it10: parse_opt(v[2]),
            it11: parse_opt(v[3]),
            it12: parse_opt(v[4]),
        });
    }
    Ok(rows)
}

/// 配合类别表（基孔制 6 种）。
pub fn fit_rows() -> Result<Vec<FitRow>, String> {
    let mut rows = Vec::new();
    let mut lines = num_lines(FIT_CSV);
    let header = lines.next().ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.spline.err.csv_header_missing",
            &[("file", "spline_gb3478_fit.csv")],
        )
    })?;
    if header.trim() != "fit,ext_dev,preferred_45,memo" {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.csv_header_bad",
            &[("file", "spline_gb3478_fit.csv"), ("header", header)],
        ));
    }
    for line in lines {
        let v: Vec<&str> = line.split(',').collect();
        if v.len() != 4 {
            return Err(crate::i18n::t_fmt(
                "cmd.spline.err.csv_cols",
                &[("file", "spline_gb3478_fit.csv"), ("line", line)],
            ));
        }
        rows.push(FitRow {
            fit: v[0].trim().to_string(),
            ext_dev: ExtDev::parse(v[1])
                .map_err(|e| crate::i18n::t_fmt("cmd.spline.err.fit_row", &[("e", &e)]))?,
            preferred_45: v[2].trim() == "true",
            memo: v[3].trim().to_string(),
        });
    }
    Ok(rows)
}

/// 量棒直径系列（GB/T 3478.9-2008 表 1，67 档；即 GB/T 321 R40 在本范围的取值）。
pub fn pin_series() -> Result<Vec<f64>, String> {
    let mut out = Vec::new();
    let mut lines = num_lines(PIN_SERIES_CSV);
    let header = lines.next().ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.spline.err.csv_header_missing",
            &[("file", "spline_gb3478_pin_series.csv")],
        )
    })?;
    if header.trim() != "d_r,pin_std,dev" {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.csv_header_bad",
            &[("file", "spline_gb3478_pin_series.csv"), ("header", header)],
        ));
    }
    for line in lines {
        let v: Vec<&str> = line.split(',').collect();
        if v.len() != 3 {
            return Err(crate::i18n::t_fmt(
                "cmd.spline.err.csv_cols",
                &[("file", "spline_gb3478_pin_series.csv"), ("line", line)],
            ));
        }
        if v[1].trim() == "true" {
            out.push(parse_num(v[0], "d_r")?);
        }
    }
    Ok(out)
}

/// 表 26 一行（m 与四档 R_imin，mm；`None` = 原表印“—”未列值）。
#[derive(Debug, Clone, Copy)]
pub struct RiminRow {
    pub m: f64,
    pub r30_flat: Option<f64>,
    pub r30_fillet: Option<f64>,
    pub r37_5: Option<f64>,
    pub r45: Option<f64>,
}

/// 表 26 全表（doc88 p51＝书页 50；15 档模数）。
pub fn rimin_table26_rows() -> Result<Vec<RiminRow>, String> {
    let mut rows = Vec::new();
    let mut lines = num_lines(RIMIN_T26_CSV);
    let header = lines.next().ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.spline.err.csv_header_missing",
            &[("file", "spline_gb3478_table26_rimin.csv")],
        )
    })?;
    if header.trim() != "m,r_30_flat,r_30_fillet,r_37_5,r_45,source,note" {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.csv_header_bad",
            &[("file", "spline_gb3478_table26_rimin.csv"), ("header", header)],
        ));
    }
    for line in lines {
        let v: Vec<&str> = line.split(',').collect();
        if v.len() != 7 {
            return Err(crate::i18n::t_fmt(
                "cmd.spline.err.csv_cols",
                &[("file", "spline_gb3478_table26_rimin.csv"), ("line", line)],
            ));
        }
        rows.push(RiminRow {
            m: parse_num(v[0], "m")?,
            r30_flat: parse_opt(v[1]),
            r30_fillet: parse_opt(v[2]),
            r37_5: parse_opt(v[3]),
            r45: parse_opt(v[4]),
        });
    }
    Ok(rows)
}

/// 表 26 查值：m 必须精确落在标准 15 档（**表外不外推**）；原表“—” → `Ok(None)`。
/// 计算口径仍走 `rimin()`（图 2 系数式），本函数供对照/复算。
pub fn rimin_table26(
    alpha: PressureAngle,
    root: RootForm,
    m: f64,
) -> Result<Option<f64>, String> {
    let rows = rimin_table26_rows()?;
    let row = rows
        .iter()
        .find(|r| (r.m - m).abs() < 1e-9)
        .ok_or_else(|| {
            crate::i18n::t_fmt(
                "cmd.spline.err.m_not_t26",
                &[("m", &m.to_string())],
            )
        })?;
    Ok(match (alpha, root) {
        (PressureAngle::A30, RootForm::Flat) => row.r30_flat,
        (PressureAngle::A30, RootForm::Fillet) => row.r30_fillet,
        (PressureAngle::A37_5, _) => row.r37_5,
        (PressureAngle::A45, _) => row.r45,
    })
}

/// D 区间索引（lo 开 hi 闭；不在表内 → None，**不插值不外推**）。
fn row_index<T, F>(rows: &[T], d: f64, lo: F) -> Option<usize>
where
    F: Fn(&T) -> (f64, f64),
{
    rows.iter().position(|r| {
        let (lo, hi) = lo(r);
        d > lo && d <= hi + 1e-12
    })
}

// ══════════════════════════════════════════════════════════════════════════
// 8.1~8.7 公差公式（μm）
// ══════════════════════════════════════════════════════════════════════════

/// 修约到整数 μm（半值远离 0；标准表 7~25 的整数口径）。
pub fn round_um(v: f64) -> f64 {
    v.round()
}

/// `i_d = 0.45·∛D + 0.001D`（D≤500）；`= 0.004D + 2.1`（D>500）。
pub fn i_d(d: f64) -> f64 {
    if d <= 500.0 {
        0.45 * d.cbrt() + 0.001 * d
    } else {
        0.004 * d + 2.1
    }
}

/// `i_E = 0.45·∛E + 0.001E`（内花键用）。
pub fn i_e(e: f64) -> f64 {
    0.45 * e.cbrt() + 0.001 * e
}

/// `i_S = 0.45·∛S − 0.001S`（外花键用）。
pub fn i_s(s: f64) -> f64 {
    0.45 * s.cbrt() - 0.001 * s
}

fn grade_coef(grade: u32) -> Result<(f64, f64), String> {
    match grade {
        4 => Ok((10.0, 40.0)),
        5 => Ok((16.0, 64.0)),
        6 => Ok((25.0, 100.0)),
        7 => Ok((40.0, 160.0)),
        other => Err(crate::i18n::t_fmt(
            "cmd.spline.err.grade_invalid",
            &[("grade", &other.to_string())],
        )),
    }
}

/// 总公差 `(T+λ)`（μm，修约；内用 i_E、外用 i_S）。
pub fn total_tolerance_um(side: SplineSide, grade: u32, m: f64, z: u32) -> Result<f64, String> {
    let (c1, c2) = grade_coef(grade)?;
    let d = m * z as f64;
    let e = 0.5 * std::f64::consts::PI * m;
    let i_base = match side {
        SplineSide::Internal => i_e(e),
        SplineSide::External => i_s(e),
    };
    Ok(round_um(c1 * i_d(d) + c2 * i_base))
}

fn fp_coef(grade: u32) -> Result<(f64, f64), String> {
    match grade {
        4 => Ok((2.5, 6.3)),
        5 => Ok((3.55, 9.0)),
        6 => Ok((5.0, 12.5)),
        7 => Ok((7.1, 18.0)),
        other => Err(crate::i18n::t_fmt(
            "cmd.spline.err.grade_invalid",
            &[("grade", &other.to_string())],
        )),
    }
}

fn fa_coef(grade: u32) -> Result<(f64, f64), String> {
    match grade {
        4 => Ok((1.6, 10.0)),
        5 => Ok((2.5, 16.0)),
        6 => Ok((4.0, 25.0)),
        7 => Ok((6.3, 40.0)),
        other => Err(crate::i18n::t_fmt(
            "cmd.spline.err.grade_invalid",
            &[("grade", &other.to_string())],
        )),
    }
}

fn fb_coef(grade: u32) -> Result<(f64, f64), String> {
    match grade {
        4 => Ok((0.8, 4.0)),
        5 => Ok((1.0, 5.0)),
        6 => Ok((1.25, 6.3)),
        7 => Ok((2.0, 10.0)),
        other => Err(crate::i18n::t_fmt(
            "cmd.spline.err.grade_invalid",
            &[("grade", &other.to_string())],
        )),
    }
}

fn fa_raw(grade: u32, m: f64, z: u32) -> Result<f64, String> {
    let (a, b) = fa_coef(grade)?;
    Ok(a * (m + 0.0125 * m * z as f64) + b)
}

fn fb_raw(grade: u32, m: f64, z: u32, fit_length: Option<f64>) -> Result<f64, String> {
    let (a, b) = fb_coef(grade)?;
    let g = fit_length.unwrap_or(m * z as f64 / 2.0);
    if !(g > 0.0) || !g.is_finite() {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.fit_len",
            &[("g", &g.to_string())],
        ));
    }
    Ok(a * g.sqrt() + b)
}

/// 齿距累积公差 `Fp`（μm，修约；`L = πmz/2`）。
pub fn fp_um(grade: u32, m: f64, z: u32) -> Result<f64, String> {
    let (a, b) = fp_coef(grade)?;
    let l = std::f64::consts::PI * m * z as f64 / 2.0;
    Ok(round_um(a * l.sqrt() + b))
}

/// 齿形公差 `Fα`（= 模板 `Ff`；μm，修约）。
pub fn fa_um(grade: u32, m: f64, z: u32) -> Result<f64, String> {
    Ok(round_um(fa_raw(grade, m, z)?))
}

/// 齿向公差 `Fβ`（μm，修约；g = 配合长度，省略 = 分度圆直径一半）。
pub fn fb_um(grade: u32, m: f64, z: u32, fit_length: Option<f64>) -> Result<f64, String> {
    Ok(round_um(fb_raw(grade, m, z, fit_length)?))
}

/// 综合公差 `λ = 0.6√(Fp²+Fα²+Fβ²)`（μm，修约；用**未修约**的 F 值）。
pub fn lambda_um(grade: u32, m: f64, z: u32, fit_length: Option<f64>) -> Result<f64, String> {
    let (a, b) = fp_coef(grade)?;
    let l = std::f64::consts::PI * m * z as f64 / 2.0;
    let fp = a * l.sqrt() + b;
    let fa = fa_raw(grade, m, z)?;
    let fb = fb_raw(grade, m, z, fit_length)?;
    Ok(round_um(0.6 * (fp * fp + fa * fa + fb * fb).sqrt()))
}

/// `(T+λ, λ, T)`（μm；`T = (T+λ) − λ`）。
pub fn machining_tolerance_um(
    side: SplineSide,
    grade: u32,
    m: f64,
    z: u32,
    fit_length: Option<f64>,
) -> Result<(f64, f64, f64), String> {
    let total = total_tolerance_um(side, grade, m, z)?;
    let lambda = lambda_um(grade, m, z, fit_length)?;
    Ok((total, lambda, total - lambda))
}

// ══════════════════════════════════════════════════════════════════════════
// 表 23/24/25 查值
// ══════════════════════════════════════════════════════════════════════════

/// 表 23 的 d/e/f 值（μm）；H/h = 0；js/k 由公式列算。
pub fn esv_um(input: &SplineInput) -> Result<f64, String> {
    let (total, _, _) = machining_tolerance_um(
        input.side,
        input.grade,
        input.m,
        input.z,
        input.fit_length,
    )?;
    match input.ext_dev {
        ExtDev::H => Ok(0.0),
        ExtDev::Js => Ok(total / 2.0),
        ExtDev::K => Ok(total),
        dev @ (ExtDev::D | ExtDev::E | ExtDev::F) => {
            let d = pitch_dia(input.m, input.z);
            let rows = ev_sv_rows()?;
            let idx = row_index(&rows, d, |r| (r.d_lo, r.d_hi)).ok_or_else(|| {
                crate::i18n::t_fmt(
                    "cmd.spline.err.d_not_t23",
                    &[("d", &d.to_string())],
                )
            })?;
            let r = rows[idx];
            Ok(match dev {
                ExtDev::D => r.d,
                ExtDev::E => r.e,
                _ => r.f,
            })
        }
    }
}

/// 表 24：外花键小径 `Die` 上偏差 `esv/tanαD`（μm）；js/k 用公式列；大径 Dee 取 0（脚注①）。
pub fn ext_dia_dev_um(input: &SplineInput) -> Result<f64, String> {
    let d = pitch_dia(input.m, input.z);
    let alpha = input.alpha;
    match input.ext_dev {
        ExtDev::H => Ok(0.0),
        ExtDev::Js | ExtDev::K => {
            let (total, _, _) = machining_tolerance_um(
                input.side,
                input.grade,
                input.m,
                input.z,
                input.fit_length,
            )?;
            let tan = alpha.rad().tan();
            Ok(match input.ext_dev {
                ExtDev::Js => total / (2.0 * tan),
                _ => total / tan,
            })
        }
        dev @ (ExtDev::D | ExtDev::E | ExtDev::F) => {
            let rows = ext_dia_dev_rows()?;
            let idx = row_index(&rows, d, |r| (r.d_lo, r.d_hi)).ok_or_else(|| {
                crate::i18n::t_fmt(
                    "cmd.spline.err.d_not_t24",
                    &[("d", &d.to_string())],
                )
            })?;
            let r = rows[idx];
            let col = match alpha {
                PressureAngle::A30 => 0,
                PressureAngle::A37_5 => 1,
                PressureAngle::A45 => 2,
            };
            Ok(match dev {
                ExtDev::D => r.d[col],
                ExtDev::E => r.e[col],
                _ => r.f[col],
            })
        }
    }
}

/// 表 25：按模数档选 IT 等级（m 0.25~0.75→10；1~1.75→11；2~10→12）。
pub fn dia_tol_it(m: f64) -> Result<u32, String> {
    if !(0.25..=10.0).contains(&m) {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.m_range",
            &[("m", &m.to_string())],
        ));
    }
    Ok(if m <= 0.75 {
        10
    } else if m <= 1.75 {
        11
    } else {
        12
    })
}

/// 内花键大径 / 外花键小径的公差等级（表 3：从 IT12、IT13 或 IT14 中选取；
/// 本库按与表 25 同构的模数档默认 12/13/14）。
pub fn side_dia_tol_it(m: f64) -> Result<u32, String> {
    Ok(dia_tol_it(m)? + 2)
}

/// GB/T 1800 IT 值（μm）；D 超出 3150 或表无值 → Err。
pub fn it_um(d: f64, it: u32) -> Result<f64, String> {
    tolerance::bounds(d, &format!("H{it}"))
        .map(|l| round_um(l.upper * 1000.0))
        .ok_or_else(|| {
            crate::i18n::t_fmt(
                "cmd.spline.err.it_missing",
                &[("it", &it.to_string()), ("d", &d.to_string())],
            )
        })
}

// ══════════════════════════════════════════════════════════════════════════
// 尺寸（表 3）与 R_imin
// ══════════════════════════════════════════════════════════════════════════

/// 分度圆直径 D = mz。
pub fn pitch_dia(m: f64, z: u32) -> f64 {
    m * z as f64
}

/// 基圆直径 Db = mz·cosαD。
pub fn base_dia(alpha: PressureAngle, m: f64, z: u32) -> f64 {
    pitch_dia(m, z) * alpha.rad().cos()
}

/// 基本齿槽宽 E / 基本齿厚 S = 0.5πm。
pub fn basic_width(m: f64) -> f64 {
    0.5 * std::f64::consts::PI * m
}

/// 内花键大径基本尺寸 Dei。
pub fn internal_major(alpha: PressureAngle, root: RootForm, m: f64, z: u32) -> f64 {
    let zz = z as f64;
    m * match (alpha, root) {
        (PressureAngle::A30, RootForm::Flat) => zz + 1.5,
        (PressureAngle::A30, RootForm::Fillet) => zz + 1.8,
        (PressureAngle::A37_5, _) => zz + 1.4,
        (PressureAngle::A45, _) => zz + 1.2,
    }
}

/// 外花键大径基本尺寸 Dee。
pub fn external_major(alpha: PressureAngle, m: f64, z: u32) -> f64 {
    let zz = z as f64;
    m * match alpha {
        PressureAngle::A30 => zz + 1.0,
        PressureAngle::A37_5 => zz + 0.9,
        PressureAngle::A45 => zz + 0.8,
    }
}

/// 外花键小径基本尺寸 Die。
pub fn external_minor(alpha: PressureAngle, root: RootForm, m: f64, z: u32) -> f64 {
    let zz = z as f64;
    m * match (alpha, root) {
        (PressureAngle::A30, RootForm::Flat) => zz - 1.5,
        (PressureAngle::A30, RootForm::Fillet) => zz - 1.8,
        (PressureAngle::A37_5, _) => zz - 1.4,
        (PressureAngle::A45, _) => zz - 1.2,
    }
}

/// 齿形裕度 CF（表 3：0.1m；注 4 说明仅 H/h，其它配合类别会变——素材未给，标注缺）。
pub fn tooth_clearance(m: f64) -> f64 {
    0.1 * m
}

/// 内花键渐开线终止圆直径最小值 D_Fimin（表 3）。
pub fn d_fimin(alpha: PressureAngle, m: f64, z: u32) -> f64 {
    let zz = z as f64;
    m * match alpha {
        PressureAngle::A30 => zz + 1.0,
        PressureAngle::A37_5 => zz + 0.9,
        PressureAngle::A45 => zz + 0.8,
    } + 2.0 * tooth_clearance(m)
}

/// 外花键渐开线起始圆直径最大值 D_Femax（表 3 注 3；`h_s` 见图 2）。
pub fn d_femax(alpha: PressureAngle, root: RootForm, m: f64, z: u32, esv_um: f64) -> f64 {
    let d = pitch_dia(m, z);
    let db = base_dia(alpha, m, z);
    let hs = match (alpha, root) {
        (PressureAngle::A30, _) => 0.6,
        (PressureAngle::A37_5, _) => 0.55,
        (PressureAngle::A45, _) => 0.5,
    } * m;
    let a = alpha.rad();
    let esv = esv_um / 1000.0;
    let inner = 0.5 * d * a.sin() - (hs - 0.5 * esv / a.tan()) / a.sin();
    2.0 * ((0.5 * db).powi(2) + inner.powi(2)).sqrt()
}

/// 内花键小径基本尺寸 `Dii = D_Femax + 2CF`（表 3；注 2：按 H/h 取 D_Femax，即 esv=0）。
pub fn internal_minor(alpha: PressureAngle, root: RootForm, m: f64, z: u32) -> f64 {
    d_femax(alpha, root, m, z, 0.0) + 2.0 * tooth_clearance(m)
}

/// 齿根圆弧最小曲率半径 R_imin = R_emin（图 2 系数 × m；**计算口径**）：
/// 30°平 0.2m / 30°圆 0.4m / 37.5° 0.3m / 45° 0.25m。
/// 表 26（doc88 p51＝书页 50）逐模数列表同值（有值格 = coef·m 半偶舍入到 2 位；
/// m=0.25 原表仅 45° 有值）——对照见 `rimin_table26()` 与资产 CSV，不替换本式。
pub fn rimin(alpha: PressureAngle, root: RootForm, m: f64) -> f64 {
    rimin_coef(alpha, root) * m
}

/// 图 2 系数（表 26 有值档同值来源）：30°平 0.2 / 30°圆 0.4 / 37.5° 0.3 / 45° 0.25。
/// 卡面按表 26 表值显示（未列值 →「—」）；本系数只作计算/报告口径。
#[must_use]
pub fn rimin_coef(alpha: PressureAngle, root: RootForm) -> f64 {
    match (alpha, root) {
        (PressureAngle::A30, RootForm::Flat) => 0.2,
        (PressureAngle::A30, RootForm::Fillet) => 0.4,
        (PressureAngle::A37_5, _) => 0.3,
        (PressureAngle::A45, _) => 0.25,
    }
}

/// 表 26 查值（卡面口径）：m 必须精确落在标准 15 档、且该齿根档位原表有值；
/// 表外/原表「—」→ `None`（卡面显示「—」，不外推）；错误同样收敛为 `None`（表外不外推）。
#[must_use]
pub fn rimin_table_for_card(alpha: PressureAngle, root: RootForm, m: f64) -> Option<f64> {
    rimin_table26(alpha, root, m).unwrap_or(None)
}

// ══════════════════════════════════════════════════════════════════════════
// 量棒 / M / W（GB/T 3478.6/.7/.8）
// ══════════════════════════════════════════════════════════════════════════

/// `inv α = tanα − α`（α 弧度）。
pub fn inv(a: f64) -> f64 {
    a.tan() - a
}

/// 解 `inv α = y`（Newton；α 弧度）。y≤0 → 0；y>inv(85°) → Err。
pub fn solve_inv(y: f64) -> Result<f64, String> {
    if y <= 0.0 {
        return Ok(0.0);
    }
    if y > inv(85f64.to_radians()) {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.inv_range",
            &[("y", &y.to_string())],
        ));
    }
    // 初值：小角近似 inv α ≈ α³/3。
    let mut a = (3.0 * y).cbrt().max(1e-4);
    for _ in 0..80 {
        let f = a.tan() - a - y;
        let df = 1.0 / (a.cos() * a.cos()) - 1.0;
        let step = f / df;
        a -= step;
        if step.abs() < 1e-15 {
            break;
        }
    }
    Ok(a)
}

/// 内花键量棒计算直径 `D'_Ri`（式(1)；D_ci=(Dee max + Dii min)/2）。
pub fn d_ri_calc(
    alpha: PressureAngle,
    d_eemax: f64,
    d_iimin: f64,
    m: f64,
    z: u32,
    e_max: f64,
) -> Result<f64, String> {
    let db = base_dia(alpha, m, z);
    if !(db > 0.0) || !(d_eemax > 0.0) || !(d_iimin > 0.0) {
        return Err(crate::i18n::t("cmd.spline.err.dri_base"));
    }
    let d = pitch_dia(m, z);
    let d_ci = (d_eemax + d_iimin) / 2.0;
    if d_ci <= db {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.dri_dci",
            &[("d_ci", &d_ci.to_string()), ("db", &db.to_string())],
        ));
    }
    let alpha_ci = (db / d_ci).acos();
    let term = alpha_ci - e_max / d + inv(alpha_ci) - alpha.inv();
    let dp = db * (alpha_ci.tan() - term.tan());
    if !(dp > 0.0) {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.dri_le0",
            &[("dp", &dp.to_string())],
        ));
    }
    Ok(dp)
}

/// 外花键量棒计算直径 `D'_Re`（式(6)；S_min 按 7 级 + h 取值）。
pub fn d_re_calc(
    alpha: PressureAngle,
    d_eemax: f64,
    d_iimin: f64,
    m: f64,
    z: u32,
    s_min: f64,
) -> Result<f64, String> {
    let db = base_dia(alpha, m, z);
    let d = pitch_dia(m, z);
    let d_ce = (d_eemax + d_iimin) / 2.0;
    if d_ce <= db {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.dre_dce",
            &[("d_ce", &d_ce.to_string()), ("db", &db.to_string())],
        ));
    }
    let alpha_ce = (db / d_ce).acos();
    let pi_z = std::f64::consts::PI / z as f64;
    let inside = alpha_ce + inv(alpha_ce) + pi_z - s_min / d - alpha.inv();
    let dp = db * (inside.tan() - alpha_ce.tan());
    if !(dp > 0.0) {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.dre_le0",
            &[("dp", &dp.to_string())],
        ));
    }
    Ok(dp)
}

/// 标准选棒：量棒系列中**最接近且较大的值**（GB/T 3478.6/7/8 §3.1.1 原文）。
pub fn dp_standard_pick(dp_calc: f64) -> Result<f64, String> {
    let series = pin_series()?;
    series
        .iter()
        .copied()
        .find(|p| *p >= dp_calc - 1e-9)
        .ok_or_else(|| {
            crate::i18n::t_fmt(
                "cmd.spline.err.dp_over_series",
                &[
                    ("dp", &format!("{dp_calc:.4}")),
                    (
                        "max",
                        &crate::partgen_kit::trim(series.last().copied().unwrap_or(0.0)),
                    ),
                ],
            )
        })
}

/// 工程备选 3 个：与 D' 最接近的 3 个系列值（升序）。标准只给 1 个（R40 较大值），
/// 「3 个」为用户/工程口径；`Md/M_Re` 必须随所填 Dp 重算。
pub fn dp_candidates_3(dp_calc: f64) -> Result<Vec<f64>, String> {
    let mut series = pin_series()?;
    series.sort_by(|a, b| {
        (a - dp_calc)
            .abs()
            .partial_cmp(&(b - dp_calc).abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut out: Vec<f64> = series.into_iter().take(3).collect();
    out.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Ok(out)
}

/// 内花键棒间距 `M_Ri`（min 用 E_min、max 用 E_max；偶/奇齿）。
pub fn m_ri(
    alpha: PressureAngle,
    m: f64,
    z: u32,
    dp: f64,
    e: f64,
    odd: bool,
) -> Result<f64, String> {
    let db = base_dia(alpha, m, z);
    let d = pitch_dia(m, z);
    let inv_a_i = e / d + alpha.inv() - dp / db;
    let a_i = solve_inv(inv_a_i)?;
    let base = db / a_i.cos();
    let v = if odd {
        base * (std::f64::consts::PI / (2.0 * z as f64)).cos() - dp
    } else {
        base - dp
    };
    Ok(v)
}

/// 外花键跨棒距 `M_Re`（min 用 S_min、max 用 S_max；偶/奇齿）。
pub fn m_re(
    alpha: PressureAngle,
    m: f64,
    z: u32,
    dp: f64,
    s: f64,
    odd: bool,
) -> Result<f64, String> {
    let db = base_dia(alpha, m, z);
    let d = pitch_dia(m, z);
    let pi_z = std::f64::consts::PI / z as f64;
    let inv_a_e = dp / db + alpha.inv() + s / d - pi_z;
    let a_e = solve_inv(inv_a_e)?;
    let base = db / a_e.cos();
    let v = if odd {
        base * (std::f64::consts::PI / (2.0 * z as f64)).cos() + dp
    } else {
        base + dp
    };
    Ok(v)
}

/// 跨测齿数 `K`（式(11) 注：K = z/6 + 0.5 取整数）。
pub fn wn_k(z: u32) -> u32 {
    ((z as f64) / 6.0 + 0.5).round().max(1.0) as u32
}

/// 外花键公法线平均长度 `W_min/W_max`（式(11)(12)，mm）。
pub fn wn(
    alpha: PressureAngle,
    m: f64,
    z: u32,
    esv_um: f64,
    total_tol_um: f64,
    t_um: f64,
) -> (f64, f64) {
    let d = pitch_dia(m, z);
    let k = wn_k(z) as f64;
    let a = alpha.rad();
    let w_min = a.cos()
        * ((k - 0.5) * std::f64::consts::PI * m + d * alpha.inv() + esv_um / 1000.0
            - total_tol_um / 1000.0);
    (w_min, w_min + t_um / 1000.0 * a.cos())
}

// ══════════════════════════════════════════════════════════════════════════
// 21 属性表（内/外）——XL「花键参数表」的目标输出
// ══════════════════════════════════════════════════════════════════════════

/// 内花键 21 项（属性名与 `~/桌面/GB/参数表/内花键参数表GB.dxf` 的 ATTDEF 对齐）。
///
/// 口径：`md_lower/md_upper` 是**绝对极限值**（E_min/E_max 对应的 M_Ri 小/大值），
/// `md` 取两者中值；阶段 2 填模板时再换算成「基本尺寸 ± 公差」写法。同理 `wn_*`。
#[derive(Debug, Clone, PartialEq)]
pub struct InternalSplineTable {
    pub alpha_deg: f64,
    pub z: u32,
    pub m: f64,
    /// 齿根型式（输入回填；卡面「齿根样式」用）。
    pub root: RootForm,
    pub grade_fit: String,
    pub major_dia: f64,
    pub major_lower: f64,
    pub major_upper: f64,
    pub dfimin: f64,
    pub minor_dia: f64,
    pub minor_lower: f64,
    pub minor_upper: f64,
    pub md: f64,
    pub md_lower: f64,
    pub md_upper: f64,
    pub dp: f64,
    /// 量棒计算直径 `D'_Ri`（式(1)，未按系列取整）。
    pub dp_calc: f64,
    /// 3 个工程备选（GB/T 3478.9 系列中与 D' 最接近的 3 个；标准只取 R40 较大值）。
    pub dp_candidates: Vec<f64>,
    pub eval_min: f64,
    pub e_max: f64,
    /// 图 2 系数计算口径（报告对照用；卡面不直接显示）。
    pub rimin: f64,
    /// 表 26 表值（卡面口径；`None` = 原表「—」/表外 → 卡面显示「—」）。
    pub rimin_table: Option<f64>,
    pub ff: f64,
    pub fp: f64,
    pub lambda: f64,
}

/// 外花键 21 项（与 `外花键参数表GB.dxf` 的 ATTDEF 对齐）。
///
/// 口径：`wn_lower/wn_upper` = W_min/W_max 绝对极限（式(11)(12)），`wn` 取中值；
/// `major_upper=0`（表 24 脚注①，Dee）；`minor_upper = esv/tanαD`（表 24，Die）。
#[derive(Debug, Clone, PartialEq)]
pub struct ExternalSplineTable {
    pub alpha_deg: f64,
    pub z: u32,
    pub m: f64,
    /// 齿根型式（输入回填；卡面「齿根样式」用）。
    pub root: RootForm,
    pub grade_fit: String,
    pub major_dia: f64,
    pub major_lower: f64,
    pub major_upper: f64,
    pub dfemax: f64,
    pub minor_dia: f64,
    pub minor_lower: f64,
    pub minor_upper: f64,
    pub wn: f64,
    pub wn_lower: f64,
    pub wn_upper: f64,
    pub kn: u32,
    pub s_min: f64,
    pub sv_max: f64,
    /// 外花键跨棒距 `M_Re`（用户 2026-09-27 点单：公法线为主、跨棒距备用；**不上卡面**）。
    /// 中值 = (M_min+M_max)/2，`md_lower/md_upper` = S_min/S_max 对应的绝对极限值。
    pub md: f64,
    pub md_lower: f64,
    pub md_upper: f64,
    /// 量棒直径（标准 R40 选值或手填；与内花键同口径）。
    pub dp: f64,
    /// 量棒计算直径 `D'_Re`（式(6)，未按系列取整）。
    pub dp_calc: f64,
    /// 3 个工程备选（GB/T 3478.9 系列中与 D' 最接近的 3 个）。
    pub dp_candidates: Vec<f64>,
    /// 图 2 系数计算口径（报告对照用；卡面不直接显示）。
    pub rimin: f64,
    /// 表 26 表值（卡面口径；`None` = 原表「—」/表外 → 卡面显示「—」）。
    pub rimin_table: Option<f64>,
    pub ff: f64,
    pub fp: f64,
    pub lambda: f64,
}

fn check_common(input: &SplineInput) -> Result<(), String> {
    if !input.m.is_finite() || !(0.25..=10.0).contains(&input.m) {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.m_range15",
            &[("m", &input.m.to_string())],
        ));
    }
    if input.z < 6 {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.z_too_small",
            &[("z", &input.z.to_string())],
        ));
    }
    // 注 1：37.5°/45° 内花键**允许**平齿根（此时 Dei 应大于 D_Fimin）；本库不拦，
    // 由 `internal_table` 按同一基本尺寸公式给出 Dei 供校核。
    if input.side == SplineSide::Internal && input.ext_dev != ExtDev::H {
        return Err(crate::i18n::t_fmt(
            "cmd.spline.err.internal_h",
            &[("dev", input.ext_dev.code())],
        ));
    }
    Ok(())
}

/// 计算内花键 21 项。
pub fn internal_table(input: &SplineInput) -> Result<InternalSplineTable, String> {
    check_common(input)?;
    if input.side != SplineSide::Internal {
        return Err(crate::i18n::t("cmd.spline.err.internal_received_ext"));
    }
    let (m, z, alpha, root) = (input.m, input.z, input.alpha, input.root);
    let (total, lambda, _t) = machining_tolerance_um(input.side, input.grade, m, z, input.fit_length)?;
    let fp = fp_um(input.grade, m, z)?;
    let ff = fa_um(input.grade, m, z)?;
    let ev_min = basic_width(m);
    let e_max = ev_min + total / 1000.0;
    let e_min = ev_min + lambda / 1000.0;
    let major = internal_major(alpha, root, m, z);
    let minor = internal_minor(alpha, root, m, z);
    let major_tol = it_um(major, side_dia_tol_it(m)?)?;
    let minor_tol = it_um(minor, dia_tol_it(m)?)?;
    // 量棒：D_ci 用 Dee max（大径上偏差 0）与 Dii min（H 下偏差 0）；E_max 按 7 级（标准表口径）。
    let (total7, _, _) = machining_tolerance_um(input.side, 7, m, z, input.fit_length)?;
    let e_max7 = ev_min + total7 / 1000.0;
    let dee = external_major(alpha, m, z);
    let dp_calc = d_ri_calc(alpha, dee, minor, m, z, e_max7)?;
    let dp = resolve_dp(input, dp_calc)?;
    let dp_candidates = dp_candidates_3(dp_calc)?;
    let m_min = m_ri(alpha, m, z, dp, e_min, z % 2 == 1)?;
    let m_max = m_ri(alpha, m, z, dp, e_max, z % 2 == 1)?;
    Ok(InternalSplineTable {
        alpha_deg: alpha.deg(),
        z,
        m,
        root,
        grade_fit: input.grade_fit_label(),
        major_dia: major,
        major_lower: 0.0,
        major_upper: major_tol / 1000.0,
        dfimin: d_fimin(alpha, m, z),
        minor_dia: minor,
        minor_lower: 0.0,
        minor_upper: minor_tol / 1000.0,
        md: (m_min + m_max) / 2.0,
        md_lower: m_min,
        md_upper: m_max,
        dp,
        dp_calc,
        dp_candidates,
        eval_min: ev_min,
        e_max,
        rimin: rimin(alpha, root, m),
        rimin_table: rimin_table_for_card(alpha, root, m),
        ff,
        fp,
        lambda,
    })
}

/// 计算外花键 21 项。
pub fn external_table(input: &SplineInput) -> Result<ExternalSplineTable, String> {
    check_common(input)?;
    if input.side != SplineSide::External {
        return Err(crate::i18n::t("cmd.spline.err.external_received_int"));
    }
    let (m, z, alpha, root) = (input.m, input.z, input.alpha, input.root);
    let (total, lambda, t) =
        machining_tolerance_um(input.side, input.grade, m, z, input.fit_length)?;
    let fp = fp_um(input.grade, m, z)?;
    let ff = fa_um(input.grade, m, z)?;
    let s = basic_width(m);
    let esv = esv_um(input)? / 1000.0;
    let sv_max = s + esv;
    let s_min = sv_max - total / 1000.0;
    let major = external_major(alpha, m, z);
    let minor = external_minor(alpha, root, m, z);
    let major_tol = it_um(major, dia_tol_it(m)?)?;
    let minor_tol = it_um(minor, side_dia_tol_it(m)?)?;
    // 小径 Die 上偏差 esv/tanαD（表 24）；大径 Dee 上偏差 = 0（表 24 脚注①）。
    let die_dev = ext_dia_dev_um(input)? / 1000.0;
    let dfemax = d_femax(alpha, root, m, z, esv_um(input)?);
    let (w_min, w_max) = wn(alpha, m, z, esv_um(input)?, total, t);
    // 量棒/跨棒距（GB/T 3478.6 §3.2 式(6)~(10)；用户 2026-09-27：外花键公法线为主、跨棒距备用）。
    // D'_Re 的 S_min 按标准口径取「7 级 + 基本偏差 h」（与内花键 E_max 取 7 级同构）；
    // D_ce = (Dee max + Dii min)/2（配对内花键为 H，Dii 下偏差 0 → 基本尺寸）。
    let (total7, _, _) = machining_tolerance_um(input.side, 7, m, z, input.fit_length)?;
    let s_min7 = basic_width(m) - total7 / 1000.0;
    let dii_min = internal_minor(alpha, root, m, z);
    let dp_calc = d_re_calc(alpha, major, dii_min, m, z, s_min7)?;
    let dp = resolve_dp(input, dp_calc)?;
    let dp_candidates = dp_candidates_3(dp_calc)?;
    let odd = z % 2 == 1;
    let m_min = m_re(alpha, m, z, dp, s_min, odd)?;
    let m_max = m_re(alpha, m, z, dp, sv_max, odd)?;
    Ok(ExternalSplineTable {
        alpha_deg: alpha.deg(),
        z,
        m,
        root,
        grade_fit: input.grade_fit_label(),
        major_dia: major,
        major_lower: -major_tol / 1000.0,
        major_upper: 0.0,
        dfemax,
        minor_dia: minor,
        minor_lower: die_dev - minor_tol / 1000.0,
        minor_upper: die_dev,
        wn: (w_min + w_max) / 2.0,
        wn_lower: w_min,
        wn_upper: w_max,
        kn: wn_k(z),
        s_min,
        sv_max,
        md: (m_min + m_max) / 2.0,
        md_lower: m_min,
        md_upper: m_max,
        dp,
        dp_calc,
        dp_candidates,
        rimin: rimin(alpha, root, m),
        rimin_table: rimin_table_for_card(alpha, root, m),
        ff,
        fp,
        lambda,
    })
}

/// 量棒直径：手填优先（必须是 GB/T 3478.9 系列值），否则按标准 R40 规则自动选。
fn resolve_dp(input: &SplineInput, dp_calc: f64) -> Result<f64, String> {
    match input.dp {
        Some(v) => {
            if !(v > 0.0) {
                return Err(crate::i18n::t_fmt(
                    "cmd.spline.err.dp_positive",
                    &[("v", &v.to_string())],
                ));
            }
            let series = pin_series()?;
            if !series.iter().any(|p| (p - v).abs() < 1e-9) {
                return Err(crate::i18n::t_fmt(
                    "cmd.spline.err.dp_not_series",
                    &[("v", &v.to_string())],
                ));
            }
            Ok(v)
        }
        None => dp_standard_pick(dp_calc),
    }
}

/// 统一入口：按 side 出 21 项（阶段 2 的 XL 命令/GUI 直接用）。
#[derive(Debug, Clone, PartialEq)]
pub enum SplineTable {
    Internal(InternalSplineTable),
    External(ExternalSplineTable),
}

/// 计算 21 项（自动按 `side` 分派）。
pub fn compute(input: &SplineInput) -> Result<SplineTable, String> {
    match input.side {
        SplineSide::Internal => Ok(SplineTable::Internal(internal_table(input)?)),
        SplineSide::External => Ok(SplineTable::External(external_table(input)?)),
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 测试：对标准逐值核对 + 表外报错 + 不变量
// ══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    /// 调研报告抽验：m=0.25、z=10 的 (T+λ)/λ/Fp/Fα（表 7，4/5/6/7 级）。
    #[test]
    fn m025_z10_matches_standard_table7() {
        let (m, z) = (0.25, 10);
        let total: Vec<f64> = [4, 5, 6, 7]
            .iter()
            .map(|g| total_tolerance_um(SplineSide::Internal, *g, m, z).unwrap())
            .collect();
        assert_eq!(total, vec![19.0, 31.0, 48.0, 77.0], "(T+λ)");
        let lambda: Vec<f64> = [4, 5, 6, 7]
            .iter()
            .map(|g| lambda_um(*g, m, z, None).unwrap())
            .collect();
        assert_eq!(lambda, vec![10.0, 14.0, 21.0, 32.0], "λ");
        let fp: Vec<f64> = [4, 5, 6, 7].iter().map(|g| fp_um(*g, m, z).unwrap()).collect();
        assert_eq!(fp, vec![11.0, 16.0, 22.0, 32.0], "Fp");
        let fa: Vec<f64> = [4, 5, 6, 7].iter().map(|g| fa_um(*g, m, z).unwrap()).collect();
        assert_eq!(fa, vec![10.0, 17.0, 26.0, 42.0], "Fα");
    }

    /// 表 23 抽 6 档 + H/h/js/k 口径。
    #[test]
    fn ev_sv_table_six_rows_and_fit_rules() {
        let mk = |dev, m: f64, z: u32, grade| SplineInput {
            m,
            z,
            alpha: PressureAngle::A30,
            root: RootForm::Fillet,
            side: SplineSide::External,
            grade,
            ext_dev: dev,
            fit_length: None,
            dp: None,
        };
        // 表值（D=2.4/8/15/25/40/65/100/160/…）
        for (d, want) in [
            (2.4, (-30.0, -20.0, -10.0)),
            (8.0, (-40.0, -25.0, -13.0)),
            (15.0, (-50.0, -32.0, -16.0)),
            (25.0, (-65.0, -40.0, -20.0)),
            (40.0, (-80.0, -50.0, -25.0)),
            (160.0, (-145.0, -85.0, -43.0)),
            (900.0, (-320.0, -170.0, -86.0)),
        ] {
            let z = 10;
            let m = d / z as f64;
            // 用 m/z 反推 D 会落在同一档；把 m 调到整数档外的区间边界需直接断言表行。
            let rows = ev_sv_rows().unwrap();
            let idx = row_index(&rows, d, |r| (r.d_lo, r.d_hi)).unwrap();
            let r = rows[idx];
            assert_eq!((r.d, r.e, r.f), want, "表 23 D={d}");
            let _ = (m, z);
        }
        // H/h = 0；js = (T+λ)/2；k = (T+λ)；d/e/f 查表；表外报错。
        assert_eq!(esv_um(&mk(ExtDev::H, 0.5, 20, 6)).unwrap(), 0.0);
        let (total, _, _) =
            machining_tolerance_um(SplineSide::External, 6, 0.5, 20, None).unwrap();
        assert_eq!(esv_um(&mk(ExtDev::Js, 0.5, 20, 6)).unwrap(), total / 2.0);
        assert_eq!(esv_um(&mk(ExtDev::K, 0.5, 20, 6)).unwrap(), total);
        // D = m·z = 10 落在 >6~10 档：d=-40、e=-25、f=-13。
        assert_eq!(esv_um(&mk(ExtDev::D, 0.5, 20, 6)).unwrap(), -40.0);
        assert_eq!(esv_um(&mk(ExtDev::E, 0.5, 20, 6)).unwrap(), -25.0);
        assert_eq!(esv_um(&mk(ExtDev::F, 0.5, 20, 6)).unwrap(), -13.0);
        // D>1000 表外：报错（不插值）。
        let err = esv_um(&mk(ExtDev::D, 1.0, 1200, 6)).unwrap_err();
        assert!(err.contains("表 23"), "{err}");
    }

    /// 表 24 抽 6 档（含三个压力角） + js/k 公式 + Dee 脚注。
    #[test]
    fn ext_dia_dev_table_six_rows() {
        for (d, want30, want375, want45) in [
            (5.0, -52.0, -39.0, -30.0),
            (8.0, -69.0, -52.0, -40.0),
            (15.0, -87.0, -65.0, -50.0),
            (25.0, -113.0, -85.0, -65.0),
            (100.0, -208.0, -156.0, -120.0),
            (200.0, -294.0, -222.0, -170.0),
            (900.0, -554.0, -417.0, -320.0),
        ] {
            let rows = ext_dia_dev_rows().unwrap();
            let idx = row_index(&rows, d, |r| (r.d_lo, r.d_hi)).unwrap();
            let r = rows[idx];
            assert_eq!(r.d, [want30, want375, want45], "表 24 D={d} d 列");
        }
        // h=0；js/k 公式（用 30°）；表外报错。
        let mk = |dev, alpha| SplineInput {
            m: 0.5,
            z: 20,
            alpha,
            root: RootForm::Fillet,
            side: SplineSide::External,
            grade: 6,
            ext_dev: dev,
            fit_length: None,
            dp: None,
        };
        assert_eq!(ext_dia_dev_um(&mk(ExtDev::H, PressureAngle::A30)).unwrap(), 0.0);
        let (total, _, _) =
            machining_tolerance_um(SplineSide::External, 6, 0.5, 20, None).unwrap();
        let tan30 = 30f64.to_radians().tan();
        assert!((ext_dia_dev_um(&mk(ExtDev::Js, PressureAngle::A30)).unwrap()
            - total / (2.0 * tan30))
            .abs()
            < 1e-9);
        assert!((ext_dia_dev_um(&mk(ExtDev::K, PressureAngle::A30)).unwrap()
            - total / tan30)
            .abs()
            < 1e-9);
        // D=10 落在 >6~10 档：d 列 30° = -69、37.5° = -52、45° = -40。
        assert_eq!(ext_dia_dev_um(&mk(ExtDev::D, PressureAngle::A30)).unwrap(), -69.0);
        assert_eq!(ext_dia_dev_um(&mk(ExtDev::D, PressureAngle::A37_5)).unwrap(), -52.0);
        assert_eq!(ext_dia_dev_um(&mk(ExtDev::D, PressureAngle::A45)).unwrap(), -40.0);
        // 表外 D>1000 → 报错（不插值）。
        let far = SplineInput {
            m: 100.0,
            z: 20,
            ..mk(ExtDev::D, PressureAngle::A30)
        };
        assert!(ext_dia_dev_um(&far).unwrap_err().contains("表 24"));
    }

    /// 表 25 印刷格与 GB/T 1800 IT 表一致；模数→等级映射。
    #[test]
    fn limits_table_matches_it_and_module_bins() {
        let rows = limits_rows().unwrap();
        for r in &rows {
            for (it, cell) in [(10u32, r.it10), (11, r.it11), (12, r.it12)] {
                if let Some(v) = cell {
                    let mid = if r.d_hi <= 6.0 {
                        6.0
                    } else {
                        r.d_hi - 1e-9
                    };
                    assert!(
                        (it_um(mid, it).unwrap() - v).abs() < 1e-9,
                        "表 25 D≤{} IT{} 印刷 {} ≠ IT 表",
                        r.d_hi,
                        it,
                        v
                    );
                }
            }
        }
        assert_eq!(dia_tol_it(0.25).unwrap(), 10);
        assert_eq!(dia_tol_it(0.75).unwrap(), 10);
        assert_eq!(dia_tol_it(1.0).unwrap(), 11);
        assert_eq!(dia_tol_it(1.75).unwrap(), 11);
        assert_eq!(dia_tol_it(2.0).unwrap(), 12);
        assert_eq!(dia_tol_it(10.0).unwrap(), 12);
        assert!(dia_tol_it(0.1).is_err());
        assert_eq!(side_dia_tol_it(0.5).unwrap(), 12);
        assert_eq!(side_dia_tol_it(5.0).unwrap(), 14);
    }

    /// 量棒系列：67 档、首末值、标准 R40 较大值、3 备选。
    #[test]
    fn pin_series_and_selection() {
        let s = pin_series().unwrap();
        assert_eq!(s.len(), 67);
        assert!(near(s[0], 0.56) && near(s[66], 25.0));
        assert!(near(dp_standard_pick(0.9309).unwrap(), 0.95));
        assert!(near(dp_standard_pick(1.0).unwrap(), 1.0), "恰好是系列值");
        assert!(near(dp_standard_pick(1.01).unwrap(), 1.06));
        assert!(dp_standard_pick(25.5).is_err(), "超出系列上限报错");
        let cand = dp_candidates_3(1.01).unwrap();
        assert_eq!(cand.len(), 3);
        assert!(cand.contains(&1.06) && cand.windows(2).all(|w| w[0] < w[1]));
    }

    /// 表 3 几何 + R_imin 系数（图 2）。
    #[test]
    fn geometry_and_rimin_match_standard() {
        let (m, z) = (2.0_f64, 20_u32);
        assert!(near(pitch_dia(m, z), 40.0));
        assert!(near(internal_major(PressureAngle::A30, RootForm::Flat, m, z), 43.0));
        assert!(near(internal_major(PressureAngle::A30, RootForm::Fillet, m, z), 43.6));
        assert!(near(external_major(PressureAngle::A30, m, z), 42.0));
        assert!(near(external_major(PressureAngle::A37_5, m, z), 41.8));
        assert!(near(external_major(PressureAngle::A45, m, z), 41.6));
        assert!(near(external_minor(PressureAngle::A30, RootForm::Flat, m, z), 37.0));
        assert!(near(external_minor(PressureAngle::A30, RootForm::Fillet, m, z), 36.4));
        assert!(near(external_minor(PressureAngle::A37_5, RootForm::Fillet, m, z), 37.2));
        assert!(near(external_minor(PressureAngle::A45, RootForm::Fillet, m, z), 37.6));
        assert!(near(d_fimin(PressureAngle::A30, m, z), 42.0 + 0.4));
        assert!(near(d_fimin(PressureAngle::A45, m, z), 41.6 + 0.4));
        assert!(near(rimin(PressureAngle::A30, RootForm::Flat, m), 0.4));
        assert!(near(rimin(PressureAngle::A30, RootForm::Fillet, m), 0.8));
        assert!(near(rimin(PressureAngle::A37_5, RootForm::Fillet, m), 0.6));
        assert!(near(rimin(PressureAngle::A45, RootForm::Fillet, m), 0.5));
    }

    /// 表 26（doc88 p51＝书页 50）：有值格 = 图 2 系数式（含半偶舍入 2 位）；
    /// m=0.25 与 m≥3 的 45° 缺口照原表“—”；表外报错。
    #[test]
    fn table26_rimin_matches_figure2_coefficients() {
        let rows = rimin_table26_rows().unwrap();
        assert_eq!(rows.len(), 15, "表 26 应为 15 档模数");
        assert!(near(rows[0].m, 0.25) && near(rows[14].m, 10.0));
        for r in &rows {
            for (a, root, tv) in [
                (PressureAngle::A30, RootForm::Flat, r.r30_flat),
                (PressureAngle::A30, RootForm::Fillet, r.r30_fillet),
                (PressureAngle::A37_5, RootForm::Fillet, r.r37_5),
                (PressureAngle::A45, RootForm::Fillet, r.r45),
            ] {
                match tv {
                    Some(v) => {
                        let c = rimin(a, root, r.m);
                        assert!(
                            (v - c).abs() <= 0.0051,
                            "m={} 表值 {v} 与图 2 系数式 {c} 不一致",
                            r.m
                        );
                        assert_eq!(rimin_table26(a, root, r.m).unwrap(), Some(v));
                    }
                    None => assert!(
                        rimin_table26(a, root, r.m).unwrap().is_none(),
                        "m={} 原表应为—",
                        r.m
                    ),
                }
            }
        }
        // 原表抽核（防 OCR 漏读）：m=1.75 行、m=2.5 的 45°、m=0.25 的缺口。
        assert_eq!(rimin_table26(PressureAngle::A45, RootForm::Fillet, 1.75).unwrap(), Some(0.44));
        assert_eq!(rimin_table26(PressureAngle::A37_5, RootForm::Fillet, 1.75).unwrap(), Some(0.52));
        assert_eq!(rimin_table26(PressureAngle::A45, RootForm::Fillet, 2.5).unwrap(), Some(0.62));
        assert_eq!(rimin_table26(PressureAngle::A45, RootForm::Fillet, 0.25).unwrap(), Some(0.06));
        assert!(rimin_table26(PressureAngle::A30, RootForm::Flat, 0.25).unwrap().is_none());
        // 计算口径保留图 2 系数式（m=0.25 30°平：表为—，式给 0.05）。
        assert!(near(rimin(PressureAngle::A30, RootForm::Flat, 0.25), 0.05));
        // 表外不外推。
        assert!(rimin_table26(PressureAngle::A30, RootForm::Flat, 0.3).is_err());
    }

    /// 量棒/M 对标准附录表 1（30° 内花键 m=0.5）：z=20、6H 用 Dp=1.00 的 M_min/M_max。
    #[test]
    fn m_ri_matches_standard_annex_table() {
        // 表 1（m=0.5）：z=20、D_Ri=1.00、6H min=8.326 / max=8.417（mm）。
        let m = 0.5_f64;
        let z = 20_u32;
        let dev = SplineInput {
            m,
            z,
            alpha: PressureAngle::A30,
            root: RootForm::Fillet,
            side: SplineSide::Internal,
            grade: 6,
            ext_dev: ExtDev::H,
            fit_length: None,
            dp: Some(1.00),
        };
        let t = internal_table(&dev).unwrap();
        assert!(
            (t.md_lower - 8.326).abs() < 0.01,
            "M_Ri min={}（表 8.326）",
            t.md_lower
        );
        assert!(
            (t.md_upper - 8.417).abs() < 0.01,
            "M_Ri max={}（表 8.417）",
            t.md_upper
        );
        assert!(near(t.dp, 1.00));
        assert!(near(t.eval_min, 0.5 * std::f64::consts::PI * m));
    }

    /// 外花键 M_Re / D'_Re（用户 2026-09-27：公法线为主、跨棒距备用）：
    /// 用 `inv`/`acos` 直接反代回环，独立于 `solve_inv` 的 Newton 迭代验证式(6)~(10)。
    #[test]
    fn external_m_re_and_d_re_round_trip() {
        let (m, z) = (3.0_f64, 20_u32);
        let root = RootForm::Fillet;
        for alpha in [
            PressureAngle::A30,
            PressureAngle::A37_5,
            PressureAngle::A45,
        ] {
            let db = base_dia(alpha, m, z);
            let d = pitch_dia(m, z);
            let (s_min, s_max) = (basic_width(m) - 0.05, basic_width(m));
            let dp = 4.0_f64;
            let m_even = m_re(alpha, m, z, dp, s_max, false).unwrap();
            let m_odd = m_re(alpha, m, z, dp, s_min, true).unwrap();
            // 回环：α_e = acos(Db/(M − Dp))，invα_e 必须等于式(8)/(9) 右边。
            let ae = (db / (m_even - dp)).acos();
            let want = dp / db + alpha.inv() + s_max / d
                - std::f64::consts::PI / z as f64;
            assert!(
                (inv(ae) - want).abs() < 1e-9,
                "{alpha:?} M_Re 回环失败：invα_e={} vs {}（奇齿值 {m_odd}）",
                inv(ae),
                want
            );
            assert!(m_even > m_odd, "S_max → M_Re 应更大");
            // D'_Re 回代：式(6) 两边 tan 一致。
            let dee = external_major(alpha, m, z);
            let dii = internal_minor(alpha, root, m, z);
            let dp_calc = d_re_calc(alpha, dee, dii, m, z, s_min).unwrap();
            let ace = (db / ((dee + dii) / 2.0)).acos();
            let rhs = (ace + inv(ace) + std::f64::consts::PI / z as f64
                - s_min / d
                - alpha.inv())
                .tan()
                - ace.tan();
            assert!((dp_calc / db - rhs).abs() < 1e-9, "{alpha:?} D'_Re 回代失败");
        }
    }

    /// 外花键 21 项结果表：M_Re 三件（dp/calc/candidates）与 S_min/S_max 同源；
    /// 手填 Dp 必须重算 M_Re（与内花键同口径）。**卡面 21 项不因它加行**。
    #[test]
    fn external_table_m_re_fields_same_compute() {
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
        let t = external_table(&input).unwrap();
        assert!(t.md_lower < t.md && t.md < t.md_upper, "M_Re 三值序");
        assert!(t.dp > 0.0 && t.dp_calc > 0.0 && t.dp_candidates.len() == 3);
        assert!(
            (t.md_lower - m_re(PressureAngle::A30, 2.0, 20, t.dp, t.s_min, false).unwrap()).abs()
                < 1e-12
        );
        assert!(
            (t.md_upper - m_re(PressureAngle::A30, 2.0, 20, t.dp, t.sv_max, false).unwrap()).abs()
                < 1e-12
        );
        let pick = t.dp_candidates[0];
        if (pick - t.dp).abs() > 1e-9 {
            let mut manual = input.clone();
            manual.dp = Some(pick);
            let t2 = external_table(&manual).unwrap();
            assert!((t2.dp - pick).abs() < 1e-12, "手填 Dp 应回填");
            assert!((t2.md - t.md).abs() > 1e-12, "选了别的 Dp，M_Re 必须重算");
        }
    }

    /// 21 项完整输出：内外各一例，字段齐、数量=21、无 NaN。
    #[test]
    fn compute_21_fields_both_sides() {
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
        let t = internal_table(&internal).unwrap();
        for v in [
            t.alpha_deg, t.m, t.major_dia, t.major_lower, t.major_upper, t.dfimin,
            t.minor_dia, t.minor_lower, t.minor_upper, t.md, t.md_lower, t.md_upper, t.dp,
            t.eval_min, t.e_max, t.rimin, t.ff, t.fp, t.lambda,
        ] {
            assert!(v.is_finite(), "内部字段含 NaN");
        }
        assert_eq!(t.grade_fit, "6H");
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
        let e = external_table(&external).unwrap();
        assert_eq!(e.grade_fit, "5f");
        assert!(e.wn_upper > e.wn_lower);
        assert!(e.kn >= 1);
        assert!(e.minor_upper <= 0.0, "Die 上偏差 esv/tanαD 为负");
        assert!(near(e.major_upper, 0.0), "Dee 上偏差脚注①=0");
    }

    /// 表外/非法组合：明确报错、不插值。
    #[test]
    fn out_of_table_errors() {
        let base = SplineInput {
            m: 0.5,
            z: 20,
            alpha: PressureAngle::A30,
            root: RootForm::Fillet,
            side: SplineSide::External,
            grade: 6,
            ext_dev: ExtDev::H,
            fit_length: None,
            dp: None,
        };
        for (what, m, z, alpha, root, grade, dev) in [
            ("m 太小", 0.1, 20, PressureAngle::A30, RootForm::Fillet, 6, ExtDev::H),
            ("m 太大", 12.0, 20, PressureAngle::A30, RootForm::Fillet, 6, ExtDev::H),
            ("等级非法", 0.5, 20, PressureAngle::A30, RootForm::Fillet, 8, ExtDev::H),
            ("z 太小", 0.5, 5, PressureAngle::A30, RootForm::Fillet, 6, ExtDev::H),
        ] {
            let p = SplineInput {
                m,
                z,
                alpha,
                root,
                side: SplineSide::External,
                grade,
                ext_dev: dev,
                fit_length: None,
                dp: None,
            };
            assert!(external_table(&p).is_err(), "{what} 应报错");
        }
        // 内花键非 H → 报错。
        let p = SplineInput {
            side: SplineSide::Internal,
            ext_dev: ExtDev::F,
            ..base
        };
        assert!(internal_table(&p).unwrap_err().contains("基孔制"));
    }
}
