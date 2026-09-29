//! 智能卡片「九字段统一齿形表达式」反解：**唯一解析器 + 表驱动字段映射**。
//!
//! 用户 2026-09-26 要求：齿轮卡 · ANSI 中/英 · NF 内花键 · DIN 五张卡都能吃轴/齿轮
//! 生成器复制的九字段表达式（`MARK KIND M Z ALPHA X DA DF BETA H`），并反解出本卡
//! 输入。口径（照国标花键卡 `spline_table::parse_expr_gear` 抄）：
//!
//! * **解析器只有一份**：本模块调 `spline_table::parse_gear_expr`（内部
//!   `shaft::parse_program`），五张卡共用；各卡只给「报错前缀」。
//! * **映射表驱动**：每卡在自己模块里给一份 [`ExprPolicy`]（体系/方向/压力角/直齿/
//!   变位口径 + [`ExprRule`] 字段清单）；公式只有 [`ExprOp`] 一处实现，五卡共用执行器
//!   [`resolve`]，不在五处 if-else 里各写一遍。
//! * **体系/方向自洽**：表达式 `MARK`/`KIND` 与本卡体系/方向不符 → 明确报错并指出冲突
//!   （文案照 GB 卡「表达式 KIND 写的是 IN（内），与卡片方向「外」不一致」同款具体）。

use crate::shaft::Gear;

/// 表达式齿形标记（`GEAR` / `SPLINE`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprMark {
    /// `GEAR`（齿轮）。
    Gear,
    /// `SPLINE`（渐开线花键）。
    Spline,
}

impl ExprMark {
    /// 报错文案里的全名。
    pub fn label(self) -> &'static str {
        match self {
            ExprMark::Gear => "GEAR（齿轮）",
            ExprMark::Spline => "SPLINE（花键）",
        }
    }
}

/// 解析后的九字段（保留 `KIND` 是否**显式**写了 `IN`/`EX`）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExprFields {
    /// 齿形段（`shaft::Gear`；`da/df/h` 可为 `None`）。
    pub gear: Gear,
    /// `GEAR` / `SPLINE`。
    pub mark: ExprMark,
    /// `Some(true)` = 显式 `IN`；`Some(false)` = 显式 `EX`；`None` = 表达式没写（默认外）。
    pub kind: Option<bool>,
}

impl ExprFields {
    /// 解析表达式（**复用既有 `spline_table::parse_gear_expr`**；`card` = 报错前缀）。
    pub fn parse(expr: &str, card: &str) -> Result<Self, String> {
        let gear = crate::spline_table::parse_gear_expr(expr, card)?;
        Ok(ExprFields {
            mark: if gear.involute {
                ExprMark::Spline
            } else {
                ExprMark::Gear
            },
            kind: crate::spline_table::expr_explicit_kind(expr),
            gear,
        })
    }

    /// 模数 m。
    pub fn m(&self) -> f64 {
        self.gear.m
    }

    /// 齿数 z。
    pub fn z(&self) -> u32 {
        self.gear.z
    }

    /// 压力角 α（度）。
    pub fn alpha_deg(&self) -> f64 {
        self.gear.alpha_deg
    }

    /// 变位系数 x。
    pub fn shift_x(&self) -> f64 {
        self.gear.x
    }

    /// 螺旋角 β（度）。
    pub fn beta_deg(&self) -> f64 {
        self.gear.beta_deg
    }

    /// ANSI 径节 `P = 25.4/m`（每英寸齿数；`m` 来自毫米制表达式）。
    pub fn ansi_pitch(&self) -> f64 {
        crate::invol_spline::ANSI_INCH_MM / self.gear.m
    }

    /// NF 基准直径 `A = m(z + 0.4 + 2x)`（NF E22-141：`x = (A − m(z+0.4))/(2m)`）。
    pub fn nf_base_a(&self) -> f64 {
        self.gear.m * (self.gear.z as f64 + 0.4 + 2.0 * self.gear.x)
    }

    /// DIN 基准直径 `d_B = m(z + 1.1 + 2x)`（DIN 5480-2 名义表自洽式）。
    pub fn din_base_d_b(&self) -> f64 {
        self.gear.m * (self.gear.z as f64 + 1.1 + 2.0 * self.gear.x)
    }
}

/// 映射算子（**公式只有这一处**；各卡 `ExprRule` 表里只写用哪个）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprOp {
    /// 模数 `m`。
    Module,
    /// 齿数 `z`。
    Teeth,
    /// ANSI 径节 `P = 25.4/m`。
    Pitch,
    /// NF 基准直径 `A = m(z + 0.4 + 2x)`。
    NfBaseA,
    /// DIN 基准直径 `d_B = m(z + 1.1 + 2x)`。
    DinBaseDB,
}

impl ExprOp {
    /// 求值（返回值恒为有限数；解析器已挡住非法的 m/z）。
    pub fn eval(self, f: &ExprFields) -> Result<f64, String> {
        let v = match self {
            ExprOp::Module => f.m(),
            ExprOp::Teeth => f.z() as f64,
            ExprOp::Pitch => {
                if !(f.m().is_finite() && f.m() > 0.0) {
                    return Err(crate::i18n::t_fmt(
                        "cmd.cardexpr.err.module",
                        &[("m", &crate::partgen_kit::trim(f.m()))],
                    ));
                }
                f.ansi_pitch()
            }
            ExprOp::NfBaseA => f.nf_base_a(),
            ExprOp::DinBaseDB => f.din_base_d_b(),
        };
        if !v.is_finite() {
            return Err(crate::i18n::t("cmd.cardexpr.err.not_finite"));
        }
        Ok(v)
    }
}

/// 映射表一行：表达式 → 本卡字段（`target` = 卡模型/GUI 控件 key）。
#[derive(Debug, Clone, Copy)]
pub struct ExprRule {
    /// 目标字段 key（GUI 回填按同一 key）。
    pub target: &'static str,
    /// 目标字段显示名（读数/错误文案）。
    pub label: &'static str,
    /// 取值算子。
    pub op: ExprOp,
}

/// 一张卡的表达式策略（各卡在自己模块里给 const）。
#[derive(Debug, Clone, Copy)]
pub struct ExprPolicy {
    /// 卡显示名（报错前缀）。
    pub card: &'static str,
    /// 本卡体系要求的 MARK。
    pub mark: ExprMark,
    /// 允许的压力角（度；空 = 不校验）。
    pub alphas: &'static [f64],
    /// 只做直齿（β 必须为 0）。
    pub spur: bool,
    /// 是否接受变位（`false` = 表达式 `X≠0` 时报错——本卡无变位输入）。
    pub allow_shift: bool,
    /// 字段映射（按表顺序求值；结果即 GUI `fields` 回填内容）。
    pub map: &'static [ExprRule],
}

/// 反解结果：解析后的字段 + 按 [`ExprPolicy::map`] 求出的卡字段值。
#[derive(Debug, Clone)]
pub struct ExprResolved {
    /// 解析后的九字段。
    pub fields: ExprFields,
    /// `(target, label, value)`，顺序 = 策略表。
    pub values: Vec<(&'static str, &'static str, f64)>,
}

impl ExprResolved {
    /// 按 target 取值。
    pub fn value(&self, target: &str) -> Option<f64> {
        self.values
            .iter()
            .find(|(t, _, _)| *t == target)
            .map(|(_, _, v)| *v)
    }
}

/// 压力角列表 → 文案（`30/37.5/45`）。
fn alpha_list(alphas: &[f64]) -> String {
    alphas
        .iter()
        .map(|a| crate::partgen_kit::trim(*a))
        .collect::<Vec<_>>()
        .join("/")
}

/// 自洽校验（**五卡唯一执行器**；测试可直接对构造好的字段跑策略）。
///
/// * `want_internal`：卡方向要校验时给 `Some(true)`（内）/`Some(false)`（外）；
///   `None` = 本卡两向都收（DIN / 齿轮卡）或方向由表达式决定。
/// * 校验顺序 = 用户可读优先级：MARK（体系）→ KIND（方向）→ α → β → X。
pub fn check(
    policy: &ExprPolicy,
    f: &ExprFields,
    want_internal: Option<bool>,
) -> Result<(), String> {
    if f.mark != policy.mark {
        return Err(crate::i18n::t_fmt(
            "cmd.cardexpr.err.mark",
            &[
                ("card", &crate::i18n::card_display(policy.card)),
                ("mark", f.mark.label()),
                ("want", policy.mark.label()),
            ],
        ));
    }
    if let (Some(want), Some(has)) = (want_internal, f.kind) {
        if want != has {
            return Err(crate::i18n::t_fmt(
                "cmd.cardexpr.err.kind",
                &[
                    ("card", &crate::i18n::card_display(policy.card)),
                    (
                        "kind",
                        &crate::i18n::t(if has {
                            "cmd.cardexpr.kind.in"
                        } else {
                            "cmd.cardexpr.kind.ex"
                        }),
                    ),
                    (
                        "want",
                        &crate::i18n::t(if want {
                            "cmd.cardexpr.dir.int"
                        } else {
                            "cmd.cardexpr.dir.ext"
                        }),
                    ),
                ],
            ));
        }
    }
    if !policy.alphas.is_empty()
        && !policy
            .alphas
            .iter()
            .any(|a| (a - f.alpha_deg()).abs() < 1e-9)
    {
        return Err(crate::i18n::t_fmt(
            "cmd.cardexpr.err.alpha",
            &[
                ("card", &crate::i18n::card_display(policy.card)),
                ("alpha", &crate::partgen_kit::trim(f.alpha_deg())),
                ("allowed", &alpha_list(policy.alphas)),
            ],
        ));
    }
    if policy.spur && f.beta_deg().abs() > 1e-9 {
        return Err(crate::i18n::t_fmt(
            "cmd.cardexpr.err.beta",
            &[
                ("card", &crate::i18n::card_display(policy.card)),
                ("beta", &crate::partgen_kit::trim(f.beta_deg())),
            ],
        ));
    }
    if !policy.allow_shift && f.shift_x().abs() > 1e-9 {
        return Err(crate::i18n::t_fmt(
            "cmd.cardexpr.err.shift",
            &[
                ("card", &crate::i18n::card_display(policy.card)),
                ("x", &crate::partgen_kit::trim(f.shift_x())),
            ],
        ));
    }
    Ok(())
}

/// 反解 + 自洽校验（解析一次，再跑 [`check`]，然后按 `map` 求本卡字段值）。
pub fn resolve(
    policy: &ExprPolicy,
    expr: &str,
    want_internal: Option<bool>,
) -> Result<ExprResolved, String> {
    let f = ExprFields::parse(expr, policy.card)?;
    check(policy, &f, want_internal)?;
    let mut values = Vec::with_capacity(policy.map.len());
    for r in policy.map {
        let v = r
            .op
            .eval(&f)
            .map_err(|e| format!("{}：{}", policy.card, e))?;
        values.push((r.target, r.label, v));
    }
    Ok(ExprResolved { fields: f, values })
}

/// 从 token 流里截出九字段表达式：从 `GEAR`/`SPLINE` 开始，到「本卡选项关键字」为止。
///
/// 选项判定从表达式第 7 个 token 起（`MARK KIND M Z ALPHA X` 已过），避免表达式自己的
/// `M3`/`Z20`/`P` 被误判；`DA/DF/BETA/H` 是表达式尾巴，各卡的 `is_option` 不应把它们
/// 当选项（DIN 的 `b`/`db` 前缀要带数字才算）。返回 `(表达式原文, 剩余 token)`。
pub fn split_expr<'a, F: Fn(&str) -> bool>(
    tokens: &[&'a str],
    is_option: F,
) -> Option<(String, Vec<&'a str>)> {
    let mut k = None;
    for (i, t) in tokens.iter().enumerate() {
        let u = t.to_ascii_uppercase();
        if u == "GEAR" || u == "SPLINE" {
            k = Some(i);
            break;
        }
    }
    let k = k?;
    let mut stop = tokens.len();
    for (i, t) in tokens.iter().enumerate().skip(k + 6) {
        if is_option(t) {
            stop = i;
            break;
        }
    }
    let expr = tokens[k..stop].join(" ");
    let rest = tokens
        .iter()
        .enumerate()
        .filter(|(i, _)| *i < k || *i >= stop)
        .map(|(_, t)| *t)
        .collect();
    Some((expr, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 解析器 = `spline_table::parse_gear_expr`（复用 `shaft::parse_program`）：
    /// MARK/KIND/九字段都取到，报错前缀换成本卡名。
    #[test]
    fn parse_reuses_shaft_program_with_card_prefix() {
        let f = ExprFields::parse(
            "SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30",
            "ANSI 花键参数表",
        )
        .unwrap();
        assert_eq!(f.mark, ExprMark::Spline);
        assert_eq!(f.kind, Some(true));
        assert_eq!(f.m(), 3.0);
        assert_eq!(f.z(), 20);
        assert!((f.alpha_deg() - 30.0).abs() < 1e-12);
        assert!(f.shift_x().abs() < 1e-12);
        // GEAR 标记 + 未写 KIND
        let g = ExprFields::parse("GEAR EX M2 Z40 ALPHA20 X0.5 BETA0 H20", "齿轮参数表").unwrap();
        assert_eq!(g.mark, ExprMark::Gear);
        assert_eq!(g.kind, Some(false));
        // 报错前缀 = 本卡
        let e = ExprFields::parse("", "NF 内花键参数表").unwrap_err();
        assert!(e.starts_with("NF 内花键参数表："), "{e}");
        let e = ExprFields::parse("bogus", "DIN 花键参数表").unwrap_err();
        assert!(e.starts_with("DIN 花键参数表："), "{e}");
    }

    /// 映射公式（唯一实现）：ANSI P、NF A、DIN d_B。
    #[test]
    fn ops_map_module_teeth_pitch_and_base_diameters() {
        // P16：m = 25.4/16 = 1.5875
        let f = ExprFields::parse(
            "SPLINE IN M1.5875 Z20 ALPHA30 X0 BETA0 H30",
            "t",
        )
        .unwrap();
        assert!((ExprOp::Pitch.eval(&f).unwrap() - 16.0).abs() < 1e-12);
        assert!((ExprOp::Module.eval(&f).unwrap() - 1.5875).abs() < 1e-12);
        assert!((ExprOp::Teeth.eval(&f).unwrap() - 20.0).abs() < 1e-12);
        // NF 锚点：m7.5 z38 x0.8 → A = 7.5·(38+0.4+1.6) = 300
        let f = ExprFields::parse(
            "SPLINE IN M7.5 Z38 ALPHA20 X0.8 BETA0 H30",
            "t",
        )
        .unwrap();
        assert!((f.nf_base_a() - 300.0).abs() < 1e-9);
        // DIN Bild 6：m3 z38 x0.45 → d_B = 3·(38+1.1+0.9) = 120
        let f = ExprFields::parse(
            "SPLINE IN M3 Z38 ALPHA30 X0.45 BETA0 H30",
            "t",
        )
        .unwrap();
        assert!((f.din_base_d_b() - 120.0).abs() < 1e-9);
    }

    /// 策略校验：MARK / KIND / α / β / X 五条错误路径文案具体。
    #[test]
    fn policy_conflicts_report_specific_reasons() {
        const P: ExprPolicy = ExprPolicy {
            card: "测试卡",
            mark: ExprMark::Spline,
            alphas: &[30.0, 37.5],
            spur: true,
            allow_shift: false,
            map: &[ExprRule {
                target: "p",
                label: "径节",
                op: ExprOp::Pitch,
            }],
        };
        // MARK 不符
        let e = resolve(&P, "GEAR EX M2 Z20 ALPHA20 X0 BETA0 H20", None).unwrap_err();
        assert!(e.contains("MARK") && e.contains("GEAR（齿轮）"), "{e}");
        // KIND 不符
        let e = resolve(
            &P,
            "SPLINE EX M3 Z20 ALPHA30 X0 BETA0 H30",
            Some(true),
        )
        .unwrap_err();
        assert!(e.contains("KIND") && e.contains("EX（外）") && e.contains("「内」"), "{e}");
        // 压力角不符：列出允许档
        let e = resolve(
            &P,
            "SPLINE IN M3 Z20 ALPHA20 X0 BETA0 H30",
            None,
        )
        .unwrap_err();
        assert!(e.contains("压力角 20°") && e.contains("30/37.5"), "{e}");
        // 螺旋角：`build` 手工造 β≠0 的字段（表达式解析器本身已挡斜齿）跑策略。
        let mut beta = ExprFields::parse("SPLINE IN M3 Z20 ALPHA30 X0 BETA0 H30", "t").unwrap();
        beta.gear.beta_deg = 8.0;
        let e = check(&P, &beta, None).unwrap_err();
        assert!(e.contains("螺旋角 β=8°"), "{e}");
        // 变位
        let e = resolve(
            &P,
            "SPLINE IN M3 Z20 ALPHA30 X0.5 BETA0 H30",
            None,
        )
        .unwrap_err();
        assert!(e.contains("变位 X=0.5"), "{e}");
        // 通过：字段表求值 + fields_json
        let r = resolve(
            &P,
            "SPLINE IN M3 Z20 ALPHA30 X0 BETA0 H30",
            None,
        )
        .unwrap();
        assert!((r.value("p").unwrap() - 25.4 / 3.0).abs() < 1e-12);
        assert_eq!(r.values[0].1, "径节");
    }

    /// 表达式截取：短表达式（缺 DA/DF）后跟选项；选项判定从第 7 token 起。
    #[test]
    fn split_expr_stops_at_option_not_expression_fields() {
        let toks: Vec<&str> = "内 SPLINE IN M1.5875 Z20 ALPHA30 X0 DA3 DF2 BETA0 H30 profile ANSI30R at 1,2"
            .split_whitespace()
            .collect();
        let is_opt = |t: &str| {
            let l = t.to_ascii_lowercase();
            matches!(l.as_str(), "profile" | "at" | "rot") || l.starts_with("p") && l != "profile"
        };
        let (expr, rest) = split_expr(&toks, is_opt).unwrap();
        assert_eq!(expr, "SPLINE IN M1.5875 Z20 ALPHA30 X0 DA3 DF2 BETA0 H30");
        assert_eq!(rest, vec!["内", "profile", "ANSI30R", "at", "1,2"]);
        // 短表达式（到 X 就结束，紧跟选项）
        let toks: Vec<&str> = "SPLINE IN M1.5875 Z20 ALPHA30 X0 P16".split_whitespace().collect();
        let (expr, rest) = split_expr(&toks, |t| t.starts_with('P') || t.starts_with('p')).unwrap();
        assert_eq!(expr, "SPLINE IN M1.5875 Z20 ALPHA30 X0");
        assert_eq!(rest, vec!["P16"]);
        // 没有表达式 → None
        assert!(split_expr(&["a", "b"], |_| false).is_none());
    }
}
