//! ISO 286 极限偏差（公差配合）计算 —— 本地实现。
//!
//! 数据来源：mecalculator.tw 公差查询页的 `utility.min.js`（纯前端 ISO 286 查表实现），
//! 此处将其 5 张表数据（identifierCond / toleranceTable / holeDeviationTable /
//! shaftDeviationTable / correctionTable）与算法移植为本地 Rust，供插件内嵌离线使用。
//! 单位约定：表内数据为 **µm**，`Limits::bounds_mm` 返回 **mm**（与 OCS 测量值一致）。
//!
//! 用法：
//! ```rust
//! use ocs_ocsm::tolerance::{Limits, hole, shaft, fit};
//! let h = hole(25.0, "H7").unwrap();   // 孔 25H7 → upper=+0.021, lower=0 (mm)
//! let s = shaft(25.0, "g6").unwrap();  // 轴 25g6 → upper=-0.007, lower=-0.020
//! let f = fit(25.0, "H7", "g6").unwrap(); // 配合间隙 0.007~0.041
//! ```

use serde_json::Value;
use std::sync::OnceLock;

/// 静态表：(identifierCond, toleranceTable, holeDeviationTable, shaftDeviationTable, correctionTable)
fn tables() -> &'static (Value, Value, Value, Value, Value) {
    static T: OnceLock<(Value, Value, Value, Value, Value)> = OnceLock::new();
    T.get_or_init(|| {
        (
            serde_json::from_str(include_str!("tables/identifierCond.json")).expect("ident"),
            serde_json::from_str(include_str!("tables/toleranceTable.json")).expect("tol"),
            serde_json::from_str(include_str!("tables/holeDeviationTable.json")).expect("hole"),
            serde_json::from_str(include_str!("tables/shaftDeviationTable.json")).expect("shaft"),
            serde_json::from_str(include_str!("tables/correctionTable.json")).expect("corr"),
        )
    })
}

/// 在按尺寸段分组的表中定位 dim 所在行（严格 `>`、`<=`，与 JS findInTable 一致）。
fn find_row<'a>(arr: &'a [Value], dim: f64) -> Option<&'a Value> {
    arr.iter().find(|r| {
        r[">"].as_f64().is_some_and(|lo| dim > lo) && r["<="].as_f64().is_some_and(|hi| dim <= hi)
    })
}

/// 从行中取某列数值（空串/缺失 → None）。
fn col(row: &Value, name: &str) -> Option<f64> {
    row.get(name)
        .and_then(|v| v.as_str().and_then(|s| s.parse::<f64>().ok()))
        .or_else(|| row.get(name).and_then(|v| v.as_f64()))
}

/// 解析 ISO 代号（如 "H7" / "g6" / "js5"）→ (字母前缀, 公差等级)。
/// 等级支持 01/0/1..=18。
pub fn parse_code(code: &str) -> Option<(String, u32)> {
    let code = code.trim();
    let split = code.find(|c: char| c.is_ascii_digit())?;
    let letters = &code[..split];
    let digits = &code[split..];
    if letters.is_empty() || digits.is_empty() {
        return None;
    }
    let it: u32 = match digits {
        "01" => 1,
        "0" => 0,
        d => d.parse().ok()?,
    };
    if it > 18 {
        return None;
    }
    Some((letters.to_string(), it))
}

/// 基本偏差（µm）＋ 修正量。`is_hole` 由代号大小写决定（大写=孔，小写=轴）。
fn basic_deviation(dim: f64, code: &str, it: u32) -> Option<f64> {
    let (_, _, hole, shaft, corr) = tables();
    let is_hole = code == code.to_uppercase();
    let tbl = if is_hole { hole } else { shaft };
    let e = find_row(tbl.as_array()?, dim)?;
    let c = find_row(corr.as_array()?, dim)?;

    let cell = |k: &str| col(e, k);
    // 特殊列（与 JS getDeviation 的 z 对象一一对应）。
    let dev = match code {
        "J" => cell(&format!("J{it}")), // 孔 J 仅定义 IT6/7/8
        "K" => cell(if it > 8 { "K>8" } else { "K<=8" }),
        "M" => {
            if dim > 250.0 && dim <= 315.0 && it == 6 {
                Some(-18.0) // 特例：≤250~315 且 IT6 → -18
            } else {
                cell("M")
            }
        }
        "N" => cell(if it > 8 { "N>8" } else { "N<=8" }),
        "j" => cell(&format!("j{it}")), // 轴 j 仅定义 IT5~8
        "k" => cell(if it < 4 || it > 7 { "k-other" } else { "k4-7" }),
        _ => cell(code),
    }?;

    // 修正量（µm）：孔 K/M/N 在 IT<=8、孔 P~ZC 在 IT<=7 时加 correctionTable 对应等级。
    let adj = |it_col: u32| col(c, &format!("IT{it_col}")).unwrap_or(0.0);
    let corr_adj = match code {
        "K" | "M" | "N" if it <= 8 => adj(it),
        "P" | "R" | "S" | "T" | "U" | "V" | "X" | "Y" | "Z" | "ZA" | "ZB" | "ZC" if it <= 7 => {
            adj(it)
        }
        _ => 0.0,
    };
    Some(dev + corr_adj)
}

/// 该尺寸段的 IT 公差值（µm）。
fn tolerance(dim: f64, it: u32) -> Option<f64> {
    let (_, tol, _, _, _) = tables();
    let row = find_row(tol.as_array()?, dim)?;
    col(row, &format!("IT{it}"))
}

/// 代号在该尺寸段是否可用（identifierCond 校验，与 JS 一致）。
fn available(dim: f64, code: &str) -> bool {
    let (ident, ..) = tables();
    let Some(arr) = ident.as_array() else {
        return false;
    };
    let Some(row) = find_row(arr, dim) else {
        return false;
    };
    row.get(code).is_some()
}

/// 上/下极限偏差（**mm**，含符号）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    pub upper: f64,
    pub lower: f64,
}

impl Limits {
    /// 显示字符串：上/下偏差，零值输出 "0"（与 OCS dimtext 堆叠兼容）。
    pub fn display(&self) -> (String, String) {
        fn fmt(v: f64) -> String {
            if v.abs() < 1e-9 {
                "0".to_string()
            } else {
                format!("{v:+.3}")
            }
        }
        (fmt(self.upper), fmt(self.lower))
    }
}

/// 计算某尺寸 + ISO 代号（如 "H7" / "g6" / "JS7"）的极限偏差（mm）。
/// `code` 大小写决定孔/轴：大写=孔，小写=轴。
pub fn bounds(dim_mm: f64, code: &str) -> Option<Limits> {
    if !(dim_mm > 0.0) || dim_mm > 3150.0 {
        return None;
    }
    let (prefix, it) = parse_code(code)?;
    let code = prefix.as_str();
    if !available(dim_mm, code) {
        return None;
    }
    // 模式（与 JS getToleranceBounds 的 add/minus/half 完全一致，按大小写区分）。
    const ADD: &[&str] = &[
        "A", "B", "C", "CD", "D", "E", "EF", "F", "FG", "G", "H", "j", "k", "m", "n", "p", "r",
        "s", "t", "u", "v", "x", "y", "z", "za", "zb", "zc",
    ];
    const MINUS: &[&str] = &[
        "J", "K", "M", "N", "P", "R", "S", "T", "U", "V", "X", "Y", "Z", "ZA", "ZB", "ZC", "a",
        "b", "c", "cd", "d", "e", "ef", "f", "fg", "g", "h",
    ];
    let z = tolerance(dim_mm, it)?; // µm
    let (upper, lower) = if ADD.contains(&code) {
        let e = basic_deviation(dim_mm, code, it)?; // µm
        (e + z, e)
    } else if MINUS.contains(&code) {
        let e = basic_deviation(dim_mm, code, it)?; // µm
        (e, e - z)
    } else {
        // JS（JS/js）：half，对称 ±IT/2（不需要基本偏差；JS 表列为占位 "IT/2"）
        (z / 2.0, -z / 2.0)
    };
    Some(Limits {
        upper: upper / 1000.0,
        lower: lower / 1000.0,
    })
}

/// 孔极限偏差（mm）。code 通常大写（H7）。
pub fn hole(dim_mm: f64, code: &str) -> Option<Limits> {
    bounds(dim_mm, &code.to_uppercase())
}

/// 轴极限偏差（mm）。code 通常小写（g6）。
pub fn shaft(dim_mm: f64, code: &str) -> Option<Limits> {
    bounds(dim_mm, &code.to_lowercase())
}

/// 配合（孔代号 + 轴代号）：返回孔/轴极限偏差与最大最小间隙/过盈。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    pub hole: Limits,
    pub shaft: Limits,
    /// 最大间隙（>0）或最大过盈（<0）。
    pub max_gap: f64,
    /// 最小间隙（>0）或最小过盈（<0）。
    pub min_gap: f64,
    /// 间隙配合 / 过渡配合 / 过盈配合
    pub kind: FitKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FitKind {
    Clearance,
    Transition,
    Interference,
}

impl FitKind {
    pub fn label(&self) -> &'static str {
        match self {
            FitKind::Clearance => "间隙配合",
            FitKind::Transition => "过渡配合",
            FitKind::Interference => "过盈配合",
        }
    }
}

/// 把配合/代号字符串解析为标注用 (上偏差, 下偏差) 显示（mm，含符号，3 位小数）。
/// - `"H7/g6"`：配合 → 显示**孔**偏差（基孔制惯例，标注以孔为主）；
/// - `"H7"`（大写）：孔偏差；`"g6"`（小写）：轴偏差；
/// - 失败（尺寸超范围/代号不可用/解析失败）返回 None。
pub fn resolve_fit_mm(dim_mm: f64, fit_str: &str) -> Option<(String, String)> {
    let fit_str = fit_str.trim();
    let (h, s) = match fit_str.split_once('/') {
        Some((h, s)) => (h.trim(), s.trim()),
        None if fit_str == fit_str.to_uppercase() => (fit_str, ""),
        None => ("", fit_str),
    };
    let l = if !h.is_empty() && !s.is_empty() {
        fit(dim_mm, h, s)?.hole
    } else if !h.is_empty() {
        hole(dim_mm, h)?
    } else if !s.is_empty() {
        shaft(dim_mm, s)?
    } else {
        return None;
    };
    Some(l.display())
}

/// 计算基孔/基轴配合（mm）。
pub fn fit(dim_mm: f64, hole_code: &str, shaft_code: &str) -> Option<Fit> {
    let h = hole(dim_mm, hole_code)?;
    let s = shaft(dim_mm, shaft_code)?;
    // 孔下偏差 - 轴下偏差 = 最小间隙；孔上偏差 - 轴上偏差 = 最大间隙。
    let max_gap = h.upper - s.lower;
    let min_gap = h.lower - s.upper;
    let kind = if min_gap >= 0.0 {
        FitKind::Clearance
    } else if max_gap <= 0.0 {
        FitKind::Interference
    } else {
        FitKind::Transition
    };
    Some(Fit {
        hole: h,
        shaft: s,
        max_gap,
        min_gap,
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mm(upper_um: f64, lower_um: f64) -> Limits {
        Limits { upper: upper_um / 1000.0, lower: lower_um / 1000.0 }
    }

    #[test]
    fn parse_codes() {
        assert_eq!(parse_code("H7"), Some(("H".into(), 7)));
        assert_eq!(parse_code("g6"), Some(("g".into(), 6)));
        assert_eq!(parse_code("js5"), Some(("js".into(), 5)));
        assert_eq!(parse_code("CD8"), Some(("CD".into(), 8)));
        assert_eq!(parse_code("IT01"), Some(("IT".into(), 1)));
        assert_eq!(parse_code("H01"), Some(("H".into(), 1)));
        assert!(parse_code("").is_none());
        assert!(parse_code("H").is_none());
        assert!(parse_code("7").is_none());
    }

    // ISO 286 权威值对照（µm）：
    // 25H7 = +21/0；25g6 = -7/-20；25f7 = -20/-41；25h6 = 0/-13；
    // 50H7 = +25/0；50p6 = +42/+26；30H8 = +33/0；30f8 = -20/-53。
    #[test]
    fn authoritative_hole_H7() {
        assert_eq!(hole(25.0, "H7").unwrap(), mm(21.0, 0.0));
        assert_eq!(hole(50.0, "H7").unwrap(), mm(25.0, 0.0));
        assert_eq!(hole(30.0, "H8").unwrap(), mm(33.0, 0.0));
    }

    #[test]
    fn authoritative_shaft() {
        assert_eq!(shaft(25.0, "g6").unwrap(), mm(-7.0, -20.0));
        assert_eq!(shaft(25.0, "f7").unwrap(), mm(-20.0, -41.0));
        assert_eq!(shaft(25.0, "h6").unwrap(), mm(0.0, -13.0));
        assert_eq!(shaft(50.0, "p6").unwrap(), mm(42.0, 26.0));
        assert_eq!(shaft(30.0, "f8").unwrap(), mm(-20.0, -53.0));
    }

    #[test]
    fn fit_H7_g6_clearance() {
        // 25H7/g6：孔 +21/0，轴 -7/-20 → 间隙 7~41µm。
        let f = fit(25.0, "H7", "g6").unwrap();
        assert_eq!(f.kind, FitKind::Clearance);
        assert!((f.max_gap - 41e-3).abs() < 1e-9);
        assert!((f.min_gap - 7e-3).abs() < 1e-9);
    }

    #[test]
    fn fit_H7_p6_interference() {
        // 25H7/p6：孔 +21/0，轴 +35/+22 → 过盈 1~35µm。
        let f = fit(25.0, "H7", "p6").unwrap();
        assert_eq!(f.kind, FitKind::Interference);
        assert!((f.max_gap - (-1e-3)).abs() < 1e-9);
        assert!((f.min_gap - (-35e-3)).abs() < 1e-9);
    }

    // 基孔制优先/常用配合表（GB/T 1801-2009，至500mm，参考 基孔制1.png 表2-2-43）。
    // 断言所有组合均为有效 ISO 配合（fit 可算）；分组以用户图为权威
    //（边界组合如 H6/n5 数学上最小过盈 1µm，标准表按人工划过渡，不断言 kind）。
    #[test]
    fn bh_fit_table_valid() {
        let d = 50.0;
        for s in ["f5","g5","h5","js5","k5","m5","n5","p5","r5","s5","t5"] { assert!(fit(d,"H6",s).is_some(), "H6/{s}"); }
        for s in ["f6","g6","h6","js6","k6","m6","n6","p6","r6","s6","t6","u6","v6","x6","y6","z6"] { assert!(fit(d,"H7",s).is_some(), "H7/{s}"); }
        for s in ["e7","f7","g7","h7","d8","e8","f8","h8","js7","k7","m7","n7","p7","r7","s7","t7","u7"] { assert!(fit(d,"H8",s).is_some(), "H8/{s}"); }
        for s in ["c9","d9","e9","f9","h9"] { assert!(fit(d,"H9",s).is_some(), "H9/{s}"); }
        for s in ["c10","d10","h10"] { assert!(fit(d,"H10",s).is_some(), "H10/{s}"); }
        for s in ["a11","b11","c11","d11","h11"] { assert!(fit(d,"H11",s).is_some(), "H11/{s}"); }
        for s in ["b12","h12"] { assert!(fit(d,"H12",s).is_some(), "H12/{s}"); }
    }

    // 基轴制优先/常用配合表（GB/T 1801-2009，至500mm，参考 基轴制1.png 表2-2-44）。
    #[test]
    fn bs_fit_table_valid() {
        let d = 50.0;
        for h in ["F6","G6","H6","JS6","K6","M6","N6","P6","R6","S6","T6"] { assert!(fit(d,h,"h5").is_some(), "{h}/h5"); }
        for h in ["F7","G7","H7","JS7","K7","M7","N7","P7","R7","S7","T7","U7"] { assert!(fit(d,h,"h6").is_some(), "{h}/h6"); }
        for h in ["E8","F8","H8","JS8","K8","M8","N8"] { assert!(fit(d,h,"h7").is_some(), "{h}/h7"); }
        for h in ["D8","E8","F8","H8"] { assert!(fit(d,h,"h8").is_some(), "{h}/h8"); }
        for h in ["D9","E9","F9","H9"] { assert!(fit(d,h,"h9").is_some(), "{h}/h9"); }
        for h in ["D10","H10"] { assert!(fit(d,h,"h10").is_some(), "{h}/h10"); }
        for h in ["A11","B11","C11","D11","H11"] { assert!(fit(d,h,"h11").is_some(), "{h}/h11"); }
        for h in ["B12","H12"] { assert!(fit(d,h,"h12").is_some(), "{h}/h12"); }
    }

    #[test]
    fn fit_H7_n6_transition() {
        // 25H7/n6：孔 +21/0，轴 +28/+15 → 过渡（-15~+6µm）。
        let f = fit(25.0, "H7", "n6").unwrap();
        assert_eq!(f.kind, FitKind::Transition);
    }

    #[test]
    fn js_half_mode() {
        // 25js7：half 模式对称 ±IT/2（IT7=21µm → ±10.5µm）。
        let h = hole(25.0, "JS7").unwrap();
        assert!((h.upper + h.lower).abs() < 1e-9); // 对称 upper == -lower
        assert!((h.upper - 10.5e-3).abs() < 1e-9);
    }

    #[test]
    fn resolve_fit_display() {
        // 25H7/g6 → 孔偏差 +0.021 / 0
        assert_eq!(resolve_fit_mm(25.0, "H7/g6").unwrap(), ("+0.021".into(), "0".into()));
        // 单孔 H7
        assert_eq!(resolve_fit_mm(25.0, "H7").unwrap(), ("+0.021".into(), "0".into()));
        // 单轴 g6
        assert_eq!(resolve_fit_mm(25.0, "g6").unwrap(), ("-0.007".into(), "-0.020".into()));
        // 非法代号/超范围
        assert!(resolve_fit_mm(25.0, "XX7").is_none());
        assert!(resolve_fit_mm(4000.0, "H7").is_none());
        assert!(resolve_fit_mm(25.0, "").is_none());
    }

    #[test]
    fn size_range_boundary() {
        assert!(bounds(0.0, "H7").is_none()); // 必须 >0
        assert!(bounds(3200.0, "H7").is_none()); // 超过 3150
        // 代号在尺寸段不可用（如 V 在 0~3 段为空）
        assert!(bounds(2.0, "V6").is_none());
    }

    #[test]
    fn display_zero_and_sign() {
        let l = hole(25.0, "H7").unwrap();
        assert_eq!(l.display(), ("+0.021".into(), "0".into()));
        let s = shaft(25.0, "g6").unwrap();
        assert_eq!(s.display(), ("-0.007".into(), "-0.020".into()));
    }
}
