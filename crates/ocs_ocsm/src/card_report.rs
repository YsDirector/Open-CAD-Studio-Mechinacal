//! 智能卡片**计算书**（GUI「计算书」按钮 → `POST /api/card_report`）。
//!
//! ## 同源铁律
//! 本模块**不引入任何新算法**：报告第 1 段「卡片项」逐字来自
//! [`crate::guide_server::apply_card_preview`]（即 `/api/card_preview` 的同一入口），
//! 与卡片 ATTDEF 用同一份 `compute()` / `values()` / `spec()`；
//! 第 2 段「引擎计算书」直接调既有 `gear::build_report` / `invol_spline::build_report_card`
//! （ANSI `Wn/Kn`、NF `K/W` 的公式推导都在后者的「检验量公式推导」小节）。
//! 同源断言见文件末尾测试（逐项值必须出现在报告文本里）。
//!
//! ## 口径
//! * 卡片没有「有效长度」输入 → 引擎报告走 `build_report_card`（不打印该行）；
//! * 缺项/表外：照卡片 `items` 的原值（通常是「—」）原样进报告，不臆造；
//! * 公式/来源进报告表格列（GUI 里也进每行 `title`）。

use crate::card::{CardRenderer, CardTypeSpec};

/// Markdown 表格单元格清洗（裸 `|` 会断开表格；换行压成空格）。
fn cell(s: &str) -> String {
    s.replace('|', "／").replace(['\n', '\r'], " ")
}

/// 「卡片项」小节：与卡片 ATTDEF 同一份取值的逐项表（值 + 公式/口径 + 来源）。
fn items_section(heading: &str, preview: &serde_json::Value) -> String {
    let mut md = String::new();
    md.push_str(heading);
    md.push_str(&crate::i18n::t("gui.report.items.head"));
    let empty = Vec::new();
    for it in preview["items"].as_array().unwrap_or(&empty) {
        let label = it["label"].as_str().unwrap_or("");
        let formula = it["formula"].as_str().unwrap_or("");
        let value = it["value"].as_str().unwrap_or("");
        let unit = it["unit"].as_str().unwrap_or("");
        let source = it["source"].as_str().unwrap_or("");
        let v = if value.is_empty() {
            "—".to_string()
        } else if unit.is_empty() {
            value.to_string()
        } else {
            format!("{value} {unit}")
        };
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            cell(label),
            cell(formula),
            cell(&v),
            cell(source)
        ));
    }
    md.push('\n');
    md
}

/// 报告公共尾巴：缺项说明 + 数据来源（卡片 summary / preview missing_note）。
fn sources_section(preview: &serde_json::Value, card: &CardTypeSpec) -> String {
    let mut md = String::new();
    md.push_str("---\n\n");
    md.push_str(&crate::i18n::t_fmt(
        "gui.report.sources.card_type",
        &[("name", &card.label()), ("id", card.id)],
    ));
    md.push_str(&crate::i18n::t_fmt(
        "gui.report.sources.card_def",
        &[("note", &cell(&card.summary()))],
    ));
    if let Some(note) = preview["missing_note"].as_str() {
        if !note.trim().is_empty() {
            md.push_str(&crate::i18n::t_fmt(
                "gui.report.sources.missing_def",
                &[("note", &cell(note))],
            ));
        }
    }
    if let Some(missing) = preview["missing"].as_array() {
        if !missing.is_empty() {
            let list = missing
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(&crate::i18n::t("gui.report.sources.list_sep"));
            md.push_str(&crate::i18n::t_fmt(
                "gui.report.sources.missing_list",
                &[("list", &list)],
            ));
        }
    }
    md.push_str(&crate::i18n::t("gui.report.sources.same_source"));
    md
}

/// 取 JSON 数字并格式化（卡片读数用；缺失 → `—`）。
fn num(preview: &serde_json::Value, key: &str) -> String {
    match preview.get(key) {
        Some(v) if v.is_number() => crate::partgen_kit::trim(v.as_f64().unwrap_or(0.0)),
        _ => "—".to_string(),
    }
}

/// 生成（标题, Markdown）。`card` = 卡类型表项；`model` = 与 `/api/card_preview` 同形 JSON。
pub fn build(card: &CardTypeSpec, model: &serde_json::Value) -> Result<(String, String), String> {
    let mut v = model.clone();
    v["card"] = serde_json::Value::String(card.id.to_string());
    let bytes =
        serde_json::to_vec(&v).map_err(|e| {
            crate::i18n::t_fmt("gui.report.err.serialize", &[("e", &e.to_string())])
        })?;
    // 同源：与 /api/card_preview 完全同一条分派（含 GB 花键卡）。
    let preview: serde_json::Value =
        serde_json::from_str(&crate::guide_server::apply_card_preview(&bytes)?).map_err(|e| {
            crate::i18n::t_fmt("gui.report.err.preview_json", &[("e", &e.to_string())])
        })?;
    let title = crate::i18n::t_fmt("gui.card.report.title", &[("card", &card.label())]);
    // 请求模型反序列化失败的报错前缀（卡名随语言；`card.name.*` = 旧报错前缀原文）。
    let model_err = |name_key: &str, e: serde_json::Error| {
        crate::i18n::t_fmt(
            "gui.report.err.model",
            &[("card", &crate::i18n::t(name_key)), ("e", &e.to_string())],
        )
    };
    let mut md = String::new();
    match card.renderer {
        CardRenderer::SplineTable => {
            md.push_str(&format!("# {title}\n\n"));
            let side_key = if preview["side"].as_str() == Some("ext") {
                "card.gb.side.ext"
            } else {
                "card.gb.side.int"
            };
            md.push_str(&crate::i18n::t_fmt(
                "gui.report.gb.side",
                &[("v", &crate::i18n::t(side_key))],
            ));
            md.push_str(&crate::i18n::t_fmt(
                "gui.report.gb.grade_fit",
                &[("v", preview["grade_fit"].as_str().unwrap_or("—"))],
            ));
            if let Some(expr) = preview["expr"].as_str() {
                md.push_str(&crate::i18n::t_fmt("gui.report.gb.expr", &[("expr", expr)]));
            }
            md.push_str(&crate::i18n::t("gui.report.gb.inputs_head"));
            let (fm, fz) = (num(&preview, "m"), num(&preview, "z"));
            let (fa, fx) = (num(&preview, "alpha"), num(&preview, "x"));
            let (fda, fdf) = (num(&preview, "da"), num(&preview, "df"));
            md.push_str(&crate::i18n::t_fmt(
                "gui.report.gb.inputs_rows",
                &[
                    ("m", &fm),
                    ("z", &fz),
                    ("alpha", &fa),
                    ("x", &fx),
                    ("da", &fda),
                    ("df", &fdf),
                    ("root", preview["root"].as_str().unwrap_or("—")),
                    (
                        "root_source",
                        preview["root_source"].as_str().unwrap_or("—"),
                    ),
                ],
            ));
            md.push_str(&items_section(
                &crate::i18n::t("gui.report.gb.items_head"),
                &preview,
            ));
            // R_imin 双口径（卡面=表 26；计算=图 2 系数）——用户 2026-09-27 拍板。
            if let Some(calc) = preview["rimin"]["calc"].as_f64() {
                let m = num(&preview, "m");
                let alpha = num(&preview, "alpha");
                let root_cn = crate::i18n::t(if preview["root"] == "fillet" {
                    "card.gb.root.fillet"
                } else {
                    "card.gb.root.flat"
                });
                md.push_str(&crate::i18n::t("gui.report.gb.rimin_head"));
                match preview["rimin"]["table"].as_f64() {
                    Some(v) => md.push_str(&crate::i18n::t_fmt(
                        "gui.report.gb.rimin_table_hit",
                        &[
                            ("m", &m),
                            ("alpha", &alpha),
                            ("root_cn", &root_cn),
                            ("v", &crate::partgen_kit::trim(v)),
                        ],
                    )),
                    None => md.push_str(&crate::i18n::t_fmt(
                        "gui.report.gb.rimin_table_miss",
                        &[("m", &m), ("alpha", &alpha), ("root_cn", &root_cn)],
                    )),
                }
                md.push_str(&crate::i18n::t_fmt(
                    "gui.report.gb.rimin_calc",
                    &[
                        (
                            "coef",
                            &crate::partgen_kit::trim(
                                preview["rimin"]["coef"].as_f64().unwrap_or(0.0),
                            ),
                        ),
                        ("calc", &crate::partgen_kit::trim(calc)),
                    ],
                ));
            }
            // 量棒/跨棒距口径（**只对内花键**；公式来自 GB/T 3478.6 §3.1）。
            // 外花键 M_Re 属「备用」量，按用户口径**只在面板出现**（面板/报告不重复堆同一量）。
            let dp = &preview["dp"];
            if dp["applicable"] == serde_json::Value::Bool(true)
                && preview["side"].as_str() == Some("int")
            {
                md.push_str(&crate::i18n::t("gui.report.gb.pin_head"));
                if let Some(f) = dp["formula"].as_str() {
                    md.push_str(&crate::i18n::t_fmt(
                        "gui.report.gb.pin_formula",
                        &[("v", &cell(f))],
                    ));
                }
                if let Some(f) = dp["md_formula"].as_str() {
                    md.push_str(&crate::i18n::t_fmt(
                        "gui.report.gb.md_formula",
                        &[("v", &cell(f))],
                    ));
                }
                if let Some(s) = dp["standard"].as_str() {
                    md.push_str(&crate::i18n::t_fmt(
                        "gui.report.gb.pin_source",
                        &[("v", &cell(s))],
                    ));
                }
                md.push_str(&crate::i18n::t_fmt(
                    "gui.report.gb.pin_values",
                    &[
                        (
                            "dp",
                            &dp["current"]
                                .as_f64()
                                .map(crate::partgen_kit::trim)
                                .unwrap_or_else(|| "—".to_string()),
                        ),
                        (
                            "md",
                            &dp["md"]["value"]
                                .as_f64()
                                .map(crate::partgen_kit::trim)
                                .unwrap_or_else(|| "—".to_string()),
                        ),
                    ],
                ));
            }
            md.push_str(&sources_section(&preview, card));
        }
        CardRenderer::SplineLite => {
            md.push_str(&format!("# {title}\n\n"));
            let side_key = if preview["side"].as_str() == Some("ext") {
                "card.gb.side.ext"
            } else {
                "card.gb.side.int"
            };
            md.push_str(&crate::i18n::t_fmt(
                "gui.report.gb.side",
                &[("v", &crate::i18n::t(side_key))],
            ));
            if let Some(expr) = preview["expr"].as_str() {
                md.push_str(&crate::i18n::t_fmt("gui.report.gb.expr", &[("expr", expr)]));
            }
            md.push_str(&crate::i18n::t("gui.report.lite.scope"));
            md.push_str(&items_section(
                &crate::i18n::t("gui.report.lite.items_head"),
                &preview,
            ));
            md.push_str(&sources_section(&preview, card));
        }
        CardRenderer::CardLite => {
            md.push_str(&format!("# {title}\n\n"));
            if let Some(c) = preview["card"].as_str() {
                md.push_str(&crate::i18n::t_fmt("gui.report.card_lite.card", &[("c", c)]));
            }
            if let Some(expr) = preview["expr"].as_str() {
                if !expr.is_empty() {
                    md.push_str(&crate::i18n::t_fmt("gui.report.gb.expr", &[("expr", expr)]));
                }
            }
            let n = preview["items"].as_array().map(|a| a.len()).unwrap_or(0);
            md.push_str(&crate::i18n::t("gui.report.card_lite.scope"));
            md.push_str(&items_section(
                &crate::i18n::t_fmt("gui.report.card_lite.items_head", &[("n", &n.to_string())]),
                &preview,
            ));
            md.push_str(&sources_section(&preview, card));
        }
        CardRenderer::GearTable => {
            let m: crate::gear_table::GearTableModel = serde_json::from_value(model.clone())
                .map_err(|e| model_err("card.name.gear", e))?;
            let spec = m.spec()?;
            let engine = crate::gear::build_report(&spec.params)?;
            md.push_str(engine.trim_end());
            md.push_str("\n\n");
            md.push_str(&items_section(
                &crate::i18n::t("gui.report.append.gear"),
                &preview,
            ));
            md.push_str(&sources_section(&preview, card));
        }
        CardRenderer::AnsiTableCn | CardRenderer::AnsiTableEn => {
            let m: crate::ansi_table::AnsiTableModel = serde_json::from_value(model.clone())
                .map_err(|e| model_err("card.name.ansi", e))?;
            let mut m = m;
            if m.side.trim().is_empty() {
                m.side = card.direction.unwrap_or("int").to_string();
            }
            let spec = m.spec()?;
            let q = spec.params()?;
            let engine = crate::invol_spline::build_report_card(&q, None);
            md.push_str(engine.trim_end());
            md.push_str("\n\n");
            md.push_str(&items_section(
                &crate::i18n::t("gui.report.append.invol"),
                &preview,
            ));
            md.push_str(&sources_section(&preview, card));
        }
        CardRenderer::NfTable => {
            let m: crate::nf_table::NfTableModel = serde_json::from_value(model.clone())
                .map_err(|e| model_err("card.name.nf", e))?;
            let spec = m.spec()?;
            let d = crate::nf_table::derive(&spec)?;
            let (q, o) = crate::invol_spline::resolve_spline(
                crate::invol_spline::SplineStd::NF,
                "",
                Some(spec.a),
                Some(spec.m),
                d.z,
                d.x,
            )?;
            let engine = crate::invol_spline::build_report_card(&q, o.as_ref());
            md.push_str(engine.trim_end());
            md.push_str("\n\n");
            md.push_str(&items_section(
                &crate::i18n::t("gui.report.append.nf"),
                &preview,
            ));
            md.push_str(&sources_section(&preview, card));
        }
        CardRenderer::NfExtTable => {
            let m: crate::nf_ext_table::NfExtTableModel = serde_json::from_value(model.clone())
                .map_err(|e| model_err("card.name.nf_ext", e))?;
            let spec = m.spec()?;
            let d = crate::nf_ext_table::derive(&spec)?;
            let (q, o) = crate::invol_spline::resolve_spline(
                crate::invol_spline::SplineStd::NF,
                "",
                Some(spec.a),
                Some(spec.m),
                d.z,
                d.x,
            )?;
            let engine = crate::invol_spline::build_report_card(&q, o.as_ref());
            md.push_str(engine.trim_end());
            md.push_str("\n\n");
            md.push_str(&items_section(
                &crate::i18n::t("gui.report.append.nf"),
                &preview,
            ));
            md.push_str(&sources_section(&preview, card));
        }
        CardRenderer::DinTable => {
            let m: crate::din_table::DinTableModel = serde_json::from_value(model.clone())
                .map_err(|e| model_err("card.name.din", e))?;
            let mut m = m;
            if m.side.is_none() {
                m.side = Some(card.direction.unwrap_or("int").to_string());
            }
            let spec = m.spec()?;
            let (q, o) = crate::invol_spline::resolve_spline(
                crate::invol_spline::SplineStd::DIN,
                "",
                Some(spec.d_b),
                Some(spec.m),
                Some(spec.z),
                None,
            )?;
            let engine = crate::invol_spline::build_report_card(&q, o.as_ref());
            md.push_str(engine.trim_end());
            md.push_str("\n\n");
            md.push_str(&items_section(
                &crate::i18n::t("gui.report.append.din"),
                &preview,
            ));
            md.push_str(&sources_section(&preview, card));
        }
    }
    Ok((title, md))
}

/// `POST /api/card_report` 的实现：body = 与 `/api/card_preview` 同形 JSON。
pub fn build_report_json(body: &[u8]) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_slice(body)
        .map_err(|e| crate::i18n::t_fmt("gui.report.err.not_json", &[("e", &e.to_string())]))?;
    let card_id = v.get("card").and_then(|c| c.as_str()).unwrap_or("");
    let card = crate::card::card_type_by_token(card_id).ok_or_else(|| {
        crate::i18n::t_fmt(
            "gui.report.err.card_unknown",
            &[("card_id", card_id)],
        )
    })?;
    let (title, markdown) = build(card, &v)?;
    Ok(serde_json::json!({"ok": true, "title": title, "markdown": markdown}).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::card_type_by_token;

    fn card(id: &str) -> &'static CardTypeSpec {
        card_type_by_token(id).unwrap()
    }

    /// 报告第 1 段必须逐项包含卡片取值（与 `preview_json` 字节一致）——
    /// 同源铁律的直接断言：报告不改数、不另算。
    fn assert_items_in_report(card_id: &str, model: serde_json::Value) {
        let c = card(card_id);
        let mut v = model.clone();
        v["card"] = serde_json::Value::String(c.id.to_string());
        let bytes = serde_json::to_vec(&v).unwrap();
        let preview: serde_json::Value =
            serde_json::from_str(&crate::guide_server::apply_card_preview(&bytes).unwrap())
                .unwrap();
        let (title, md) = build(c, &model).unwrap();
        assert!(title.starts_with(&c.label()), "{card_id} 标题应含卡名");
        assert!(md.contains("同一份"), "{card_id} 报告缺同源说明：\n{md}");
        let empty = Vec::new();
        let items = preview["items"].as_array().unwrap_or(&empty);
        assert!(!items.is_empty(), "{card_id} 预览无项");
        for it in items {
            let value = it["value"].as_str().unwrap_or("");
            let label = it["label"].as_str().unwrap_or("");
            assert!(
                md.contains(label),
                "{card_id} 报告缺项标签 `{label}`"
            );
            if !value.is_empty() && value != "—" {
                assert!(
                    md.contains(value),
                    "{card_id} 报告缺卡片值 `{value}`（项 `{label}`）"
                );
            }
            let formula = it["formula"].as_str().unwrap_or("");
            if !formula.is_empty() {
                let first = formula.split(['|', '\n']).next().unwrap_or("");
                assert!(
                    md.contains(first),
                    "{card_id} 报告缺公式/口径 `{first}`（项 `{label}`）"
                );
            }
        }
        // 报告 = 卡片项 + 引擎计算书 + 来源；不再出现裸 `|` 断表（除表格分隔行）。
        assert!(md.contains("## 附：卡片项") || card_id == "花键参数表" || card_id == "花键参数表_外"
            || card_id.starts_with("GB花键精简表"), "{card_id} 报告缺卡片项节：{md}");
    }

    #[test]
    fn report_gb_card_shares_one_compute_with_attributes() {
        assert_items_in_report(
            "花键参数表",
            serde_json::json!({
                "card": "花键参数表", "side": "int", "grade": 6, "fit": "H",
                "expr": "SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30",
                "root": "auto", "dp": null, "at": [10.0, 20.0], "rot": 0.0
            }),
        );
        let (_, md) = build(
            card("花键参数表"),
            &serde_json::json!({
                "card": "花键参数表", "side": "int", "grade": 6, "fit": "H",
                "expr": "SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30",
                "root": "auto", "dp": null, "at": null, "rot": 0.0
            }),
        )
        .unwrap();
        assert!(md.contains("## 2. 参数表（21 项"), "{md}");
        assert!(md.contains("量棒/跨棒距口径"), "{md}");
        assert!(md.contains("spline_tol::compute()"), "{md}");
        // 面板同一份值：Dp 取当前选棒（不是占位「—」）、Md 为中值。
        assert!(md.contains("Dp="), "{md}");
        assert!(!md.contains("Dp=—"), "报告 Dp 不应是占位符：\n{md}");
    }

    /// 外花键 M_Re：**只在面板出现**（用户 2026-09-27：公法线为主、跨棒距备用；
    /// 同一量只在一处给）——报告不得再堆一份 Dp/M_Re 数值或公式节。
    #[test]
    fn external_report_has_no_m_re_section() {
        let (_, md) = build(
            card("花键参数表_外"),
            &serde_json::json!({
                "card": "花键参数表_外", "side": "ext", "grade": 6, "fit": "f",
                "expr": "SPLINE EX M2 Z20 ALPHA30 X0 DA42 DF37 BETA0 H30",
                "root": "auto", "dp": null, "at": null, "rot": 0.0
            }),
        )
        .unwrap();
        assert!(md.contains("## 2. 参数表（21 项"), "{md}");
        assert!(!md.contains("量棒/跨棒距口径"), "外花键报告不应重复 M_Re：\n{md}");
        assert!(!md.contains("M_Re"), "外花键报告不应出现 M_Re 值/公式：\n{md}");
    }

    #[test]
    fn report_gear_ansi_nf_din_share_engine_and_card_items() {
        // 齿轮：引擎计算书（式→代入→结果）+ 卡片 19 项。
        assert_items_in_report(
            "齿轮参数表",
            serde_json::json!({
                "card": "齿轮参数表",
                "expr": "GEAR EX M2 Z40 ALPHA20 X0 DA84 DF75 BETA0 H30",
                "mate_z": null, "dwg": null, "grade": null, "center": null,
                "at": null, "rot": 0.0
            }),
        );
        // ANSI：引擎报告必须带 Wn/Kn 推导；卡片 17 项同源。
        let (_, ansi_md) = {
            let c = card("ANSI花键参数表_中文");
            let model = serde_json::json!({
                "card": c.id, "p": 8.0, "z": 20, "profile": null, "expr": null,
                "at": null, "rot": 0.0
            });
            let mut v = model.clone();
            v["card"] = serde_json::Value::String(c.id.to_string());
            let bytes = serde_json::to_vec(&v).unwrap();
            let preview: serde_json::Value =
                serde_json::from_str(&crate::guide_server::apply_card_preview(&bytes).unwrap()).unwrap();
            let empty = Vec::new();
            for it in preview["items"].as_array().unwrap_or(&empty) {
                let value = it["value"].as_str().unwrap_or("");
                let wrong = it["missing"].as_bool().unwrap_or(false);
                assert!(!wrong || value == "—", "缺项值必须是「—」");
            }
            let (_, md) = build(c, &model).unwrap();
            (c, md)
        };
        assert!(
            ansi_md.contains("### 检验量公式推导（Wn / Kn；渐开线几何）"),
            "{ansi_md}"
        );
        assert!(ansi_md.contains("Wn = m·cosα"), "{ansi_md}");
        assert!(ansi_md.contains("## 附：卡片项"), "{ansi_md}");
        // NF 内/外：引擎报告带 K/W 推导；卡片 18 项同源。
        for id in ["NF内花键参数表", "NF外花键参数表"] {
            assert_items_in_report(
                id,
                serde_json::json!({
                    "card": id, "a": 300.0, "m": 7.5, "z": 38,
                    "centering": null, "root": null, "fit": null,
                    "at": null, "rot": 0.0
                }),
            );
        }
        let (_, nf_md) = build(
            card("NF外花键参数表"),
            &serde_json::json!({
                "card": "NF外花键参数表", "a": 300.0, "m": 7.5, "z": 38,
                "centering": null, "root": null, "fit": null, "at": null, "rot": 0.0
            }),
        )
        .unwrap();
        assert!(nf_md.contains("### 检验量公式推导（K / W）"), "{nf_md}");
        assert!(nf_md.contains("W = m·cos20°"), "{nf_md}");
        assert!(nf_md.contains("K = 6"), "{nf_md}");
        // DIN：引擎报告 + 卡片 13 项同源。
        assert_items_in_report(
            "DIN花键参数表",
            serde_json::json!({
                "card": "DIN花键参数表", "m": 3.0, "z": 38, "d_b": 120.0,
                "hub": null, "shaft": null, "e2": null, "ae": null, "as_": null,
                "tact_n": null, "teff_n": null, "tact_w": null, "teff_w": null,
                "at": null, "rot": 0.0
            }),
        );
    }

    /// 同源铁律（照既有「同一份 compute()」测试）：报告文本必须逐个包含
    /// 卡片出表时写进图纸的 ATTRIB 值（`pending_attrs()` = 块/INSERT 的唯一填值来源）。
    #[test]
    fn report_and_card_attdefs_share_one_source() -> Result<(), String> {
        // (卡 id, 模型, 是否整份零汉字) —— GB 全卡的 21 项**列口径**来自 `spline_gui.rs` 的
        // 专用面板（本批范围外，见 §36 交接）；报告自身正文/表头仍断言随语言。
        let cases: Vec<(&str, serde_json::Value)> = vec![
            (
                "花键参数表",
                serde_json::json!({"card":"花键参数表","side":"int","grade":6,"fit":"H",
                    "expr":"SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30",
                    "root":"auto","dp":null,"at":null,"rot":0.0}),
            ),
            (
                "齿轮参数表",
                serde_json::json!({"card":"齿轮参数表",
                    "expr":"GEAR EX M2 Z40 ALPHA20 X0 DA84 DF75 BETA0 H30",
                    "mate_z":null,"dwg":null,"grade":null,"center":null,"at":null,"rot":0.0}),
            ),
            (
                "ANSI花键参数表_中文",
                serde_json::json!({"card":"ANSI花键参数表_中文","side":"int","p":8.0,"z":20,
                    "profile":null,"expr":null,"at":null,"rot":0.0}),
            ),
            (
                "NF内花键参数表",
                serde_json::json!({"card":"NF内花键参数表","a":300.0,"m":7.5,"z":38,
                    "centering":null,"root":null,"fit":null,"at":null,"rot":0.0}),
            ),
            (
                "NF外花键参数表",
                serde_json::json!({"card":"NF外花键参数表","a":300.0,"m":7.5,"z":38,
                    "centering":null,"root":null,"fit":null,"at":null,"rot":0.0}),
            ),
            (
                "DIN花键参数表",
                serde_json::json!({"card":"DIN花键参数表","side":"int","m":3.0,"z":38,"d_b":120.0,
                    "hub":null,"shaft":null,"e2":null,"ae":null,"as_":null,
                    "tact_n":null,"teff_n":null,"tact_w":null,"teff_w":null,"at":null,"rot":0.0}),
            ),
            (
                "GB花键精简表_内",
                serde_json::json!({"card":"GB花键精简表_内","grade":6,"fit":"H",
                    "expr":"SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30",
                    "root":"auto","dp":null,"at":null,"rot":0.0}),
            ),
            (
                "GB花键精简表_外",
                serde_json::json!({"card":"GB花键精简表_外","grade":6,"fit":"h",
                    "expr":"SPLINE EX M3 Z20 ALPHA30 X0 DA66 DF52.5 BETA0 H30",
                    "root":"auto","dp":null,"at":null,"rot":0.0}),
            ),
            (
                "NF花键精简表_内",
                serde_json::json!({"card":"NF花键精简表_内","expr":null,"a":300.0,"m":7.5,
                    "z":38,"centering":null,"root":null,"fit":null,"at":null,"rot":0.0}),
            ),
            (
                "DIN花键精简表_内",
                serde_json::json!({"card":"DIN花键精简表_内","side":"int","expr":null,
                    "m":3.0,"z":38,"d_b":120.0,"hub":"9H","at":null,"rot":0.0}),
            ),
            (
                "齿轮精简表",
                serde_json::json!({"card":"齿轮精简表",
                    "expr":"GEAR EX M2 Z40 ALPHA20 X0 DA84 DF75 BETA0 H30",
                    "mate_z":null,"dwg":null,"grade":null,"center":null,"at":null,"rot":0.0}),
            ),
        ];
        for (id, model) in cases {
            let c = card(id);
            let (_, md) = build(c, &model).unwrap();
            let attrs: Vec<(String, String)> = match c.renderer {
                CardRenderer::SplineTable => {
                    let mut m: crate::spline_gui::SplineTableModel =
                        serde_json::from_value(model.clone()).unwrap();
                    if m.side.trim().is_empty() {
                        m.side = "int".to_string();
                    }
                    m.pending_attrs()?
                        .into_iter()
                        .map(|(ad, v)| (ad.tag.clone(), v))
                        .collect()
                }
                CardRenderer::GearTable => {
                    let m: crate::gear_table::GearTableModel =
                        serde_json::from_value(model.clone()).unwrap();
                    m.pending_attrs()?
                        .into_iter()
                        .map(|(ad, v)| (ad.tag.clone(), v))
                        .collect()
                }
                CardRenderer::AnsiTableCn | CardRenderer::AnsiTableEn => {
                    let mut m: crate::ansi_table::AnsiTableModel =
                        serde_json::from_value(model.clone()).unwrap();
                    if m.side.trim().is_empty() {
                        m.side = c.direction.unwrap_or("int").to_string();
                    }
                    m.pending_attrs()?
                        .into_iter()
                        .map(|(ad, v)| (ad.tag.clone(), v))
                        .collect()
                }
                CardRenderer::NfTable => {
                    let m: crate::nf_table::NfTableModel =
                        serde_json::from_value(model.clone()).unwrap();
                    m.pending_attrs()?
                        .into_iter()
                        .map(|(ad, v)| (ad.tag.clone(), v))
                        .collect()
                }
                CardRenderer::NfExtTable => {
                    let m: crate::nf_ext_table::NfExtTableModel =
                        serde_json::from_value(model.clone()).unwrap();
                    m.pending_attrs()?
                        .into_iter()
                        .map(|(ad, v)| (ad.tag.clone(), v))
                        .collect()
                }
                CardRenderer::DinTable => {
                    let mut m: crate::din_table::DinTableModel =
                        serde_json::from_value(model.clone()).unwrap();
                    if m.side.is_none() {
                        m.side = Some(c.direction.unwrap_or("int").to_string());
                    }
                    m.pending_attrs()?
                        .into_iter()
                        .map(|(ad, v)| (ad.tag.clone(), v))
                        .collect()
                }
                CardRenderer::SplineLite => {
                    let mut m: crate::spline_gui::SplineTableModel =
                        serde_json::from_value(model.clone()).unwrap();
                    if m.side.trim().is_empty() {
                        m.side = c.direction.unwrap_or("int").to_string();
                    }
                    let input = m.to_input()?;
                    let table = crate::spline_tol::compute(&input)?;
                    crate::spline_lite::pending_attrs(input.side, &table)?
                        .into_iter()
                        .map(|(ad, v)| (ad.tag.clone(), v))
                        .collect()
                }
                CardRenderer::CardLite => {
                    let lc = crate::card_lite::by_id(c.id).unwrap();
                    let preview = crate::card_lite::preview_json(lc, &model)?;
                    crate::card_lite::values_from_preview(lc, &preview)?
                }
            };
            assert!(!attrs.is_empty(), "{id} 无 ATTRIB");
            let mut checked = 0usize;
            for (tag, v) in attrs {
                if v.is_empty() || v == "—" {
                    continue;
                }
                assert!(
                    md.contains(&v),
                    "{id} 报告缺卡片 ATTRIB 值 `{v}`（tag `{tag}`）——报告与卡片不同源？"
                );
                checked += 1;
            }
            assert!(checked > 0, "{id} 未比到任何非缺项 ATTRIB 值");
        }
        Ok(())
    }

    /// 表 26 / C_F 口径补全（2026-09-26）：GB 报告与卡片同源地带上出处与缺口说明。
    #[test]
    fn gb_report_notes_table26_rimin_and_cf_scope() -> Result<(), String> {
        let model = serde_json::json!({"card":"花键参数表","side":"int","grade":6,"fit":"H",
            "expr":"SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30",
            "root":"auto","dp":null,"at":null,"rot":0.0});
        let (_, md) = build(card("花键参数表"), &model)?;
        assert!(md.contains("表 26"), "报告应含 R_imin 的表 26 出处：\n{md}");
        assert!(md.contains("逐格核对"), "报告应含表 26 逐格核对口径：\n{md}");
        assert!(
            md.contains("R_imin 口径") && md.contains("卡面按表 26"),
            "报告应写明卡面=表 26 口径：\n{md}"
        );
        assert!(md.contains("0.6"), "m=3/30°平禁面应为表值 0.6：\n{md}");
        assert!(
            md.contains("全 70 页未列值") || md.contains("全文档无 CF 变化值表"),
            "报告应说明非 H/h 的 C_F 缺口：\n{md}"
        );
        // m=0.25（表 26 未列值）报告应写「未列值 + 图 2 系数 = 0.05」，且卡面项值为「—」。
        let (_, md25) = build(
            card("花键参数表"),
            &serde_json::json!({"card":"花键参数表","side":"int","grade":6,"fit":"H",
                "expr":"SPLINE IN M0.25 Z20 ALPHA30 X0 DA5.375 DF4.375 BETA0 H30",
                "root":"flat","dp":null,"at":null,"rot":0.0}),
        )?;
        assert!(md25.contains("表 26 未列值"), "m=0.25 应注明表 26 未列值：\n{md25}");
        assert!(md25.contains("0.05"), "m=0.25 报告应给图 2 系数对照 0.05：\n{md25}");
        assert!(md25.contains("| 齿根圆最小曲率半径 R_imin |"), "{md25}");
        assert!(md25.contains("| — mm |"), "卡面项值应为「— mm」：\n{md25}");
        Ok(())
    }

    #[test]
    fn report_lite_card_is_same_source_and_has_no_tolerance_items() -> Result<(), String> {
        // 内/外两张精简卡：报告 9 项与卡片 ATTRIB 同源；负断言无公差项/文本。
        for (id, expr, fit) in [
            ("GB花键精简表_内", "SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30", "H"),
            ("GB花键精简表_外", "SPLINE EX M3 Z20 ALPHA30 X0 DA66 DF52.5 BETA0 H30", "h"),
        ] {
            let model = serde_json::json!({
                "card": id, "grade": 6, "fit": fit, "expr": expr,
                "root": "auto", "dp": null, "at": null, "rot": 0.0,
            });
            assert_items_in_report(id, model.clone());
            let (title, md) = build(card(id), &model)?;
            assert!(title.contains("精简"), "{title}");
            assert!(md.contains("不含任何公差"), "{md}");
            assert!(md.contains("精简项（9 项"), "{md}");
            // 报告表格/正文不得出现公差项标签。
            for bad in ["公差等级和配合类别", "大径上公差", "大径下公差", "量棒直径上公差"] {
                assert!(!md.contains(bad), "精简卡报告不应含「{bad}」：\n{md}");
            }
        }
        Ok(())
    }

    /// 阶段 5：计算书**正文/标题/表头随语言**（en 侧零汉字；公式体/符号/键名/标准号原样；
    /// zh 侧与 HEAD 逐字一致 —— 见 `i18n_检查/card_report_i18n_gen.py` 的 `--verify`）。
    #[test]
    fn report_body_switches_language_and_has_no_cjk_in_english() {
        use crate::i18n::{clear_missing_keys, set_lang, set_lang_auto, Lang};
        let _g = crate::global_state_test_lock();
        let cjk = |s: &str| s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
        // (卡 id, 模型, 是否整份零汉字) —— GB 全卡的 21 项**列口径**来自 `spline_gui.rs` 的
        // 专用面板（本批范围外，见 §36 交接）；报告自身正文/表头仍断言随语言。
        let cases: Vec<(&str, serde_json::Value, bool)> = vec![
            (
                "花键参数表",
                serde_json::json!({"card":"花键参数表","side":"int","grade":6,"fit":"H",
                    "expr":"SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30",
                    "root":"auto","dp":null,"at":null,"rot":0.0}),
                false,
            ),
            (
                "齿轮参数表",
                serde_json::json!({"card":"齿轮参数表",
                    "expr":"GEAR EX M2 Z40 ALPHA20 X0 DA84 DF75 BETA0 H30",
                    "mate_z":null,"dwg":null,"grade":null,"center":null,"at":null,"rot":0.0}),
                true,
            ),
            (
                // ★ 用**英文版** ANSI 卡：中文版的卡面**值**（花键类型）是中文，属卡自身语种。
                "ANSI花键参数表_英文",
                serde_json::json!({"card":"ANSI花键参数表_英文","side":"int","p":8.0,"z":20,
                    "profile":null,"expr":null,"at":null,"rot":0.0}),
                true,
            ),
            (
                "NF内花键参数表",
                serde_json::json!({"card":"NF内花键参数表","a":300.0,"m":7.5,"z":38,
                    "centering":null,"root":null,"fit":null,"at":null,"rot":0.0}),
                true,
            ),
            (
                "DIN花键参数表",
                serde_json::json!({"card":"DIN花键参数表","side":"int","m":3.0,"z":38,"d_b":120.0,
                    "hub":null,"shaft":null,"e2":null,"ae":null,"as_":null,
                    "tact_n":null,"teff_n":null,"tact_w":null,"teff_w":null,"at":null,"rot":0.0}),
                true,
            ),
            (
                "GB花键精简表_内",
                serde_json::json!({"card":"GB花键精简表_内","grade":6,"fit":"H",
                    "expr":"SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30",
                    "root":"auto","dp":null,"at":null,"rot":0.0}),
                true,
            ),
            (
                "NF花键精简表_内",
                serde_json::json!({"card":"NF花键精简表_内","expr":null,"a":300.0,"m":7.5,
                    "z":38,"centering":null,"root":null,"fit":null,"at":null,"rot":0.0}),
                true,
            ),
        ];
        clear_missing_keys();
        for (id, model, full_clean) in &cases {
            let c = card(id);
            set_lang(Lang::Zh);
            let (zh_title, zh) = build(c, model).unwrap();
            set_lang(Lang::En);
            let (en_title, en) = build(c, model).unwrap();
            assert_ne!(zh_title, en_title, "{id} 标题应随语言");
            assert!(cjk(&zh_title) && !cjk(&en_title), "{id} 标题：{en_title}");
            assert!(cjk(&zh), "{id} zh 报告应含汉字");
            if !*full_clean {
                assert!(en.contains("- Direction: **internal spline**"), "{en}");
                assert!(en.contains("## 2. Parameter table"), "{en}");
            }
            // ★ 报告**自身**的正文/标题/表头不得残留中文（引擎报告里的图库数据串/文件名、
            //   卡 id、齿廓预设名是**数据/协议记号** —— 翻了就对不上，见 §35 口径）。
            for bad in [
                "卡类型：",
                "卡片口径：",
                "缺项说明：",
                "本卡标缺项：",
                "同源：",
                "方向：",
                "等级/配合：",
                "表达式：",
                "输入与反解",
                "参数表（21 项",
                "R_imin 口径",
                "量棒/跨棒距口径",
                "精确",
                "精简项（",
                "附：卡片项",
                "不外推",
                "不臆造",
            ] {
                assert!(!en.contains(bad), "{id} en 报告仍含报告自身中文：{bad}\n{en}");
            }
            assert!(!cjk(&en_title), "{id} en 标题仍有汉字：{en_title}");
            // 表头/来源行两语同结构（同一份计算）。
            assert!(en.contains("## Appendix: card items") || en.contains("## 1. Lite items")
                || en.contains("## 2. Parameter table"), "{id} en 报告缺表头：\n{en}");
            assert!(en.contains("Same source"), "{id} en 报告缺同源说明：\n{en}");
        }
        set_lang_auto();
        let missing = crate::i18n::missing_keys();
        assert!(missing.is_empty(), "缺词条：{missing:?}");
    }

    /// 请求报错亦随语言（卡名/字段/JSON/未知卡）。
    #[test]
    fn report_errors_switch_language() {
        use crate::i18n::{set_lang, set_lang_auto, Lang};
        let _g = crate::global_state_test_lock();
        let cjk = |s: &str| s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
        set_lang(Lang::En);
        let e1 = build_report_json(b"not json").unwrap_err();
        assert!(!cjk(&e1) && e1.contains("not JSON"), "{e1}");
        let e2 = build_report_json(r#"{"card":"NOSUCHCARD","m":1}"#.as_bytes()).unwrap_err();
        assert!(!cjk(&e2) && e2.contains("unknown card type"), "{e2}");
        let e3 =
            build_report_json(r#"{"card":"齿轮参数表","expr":"GEAR EX M2 Z40"}"#.as_bytes())
                .unwrap_err();
        assert!(!cjk(&e3) && !e3.is_empty(), "{e3}");
        set_lang_auto();
    }

    #[test]
    fn report_json_errors_are_actionable() {
        let e = build_report_json(b"not json").unwrap_err();
        assert!(e.contains("不是 JSON"), "{e}");
        let e = build_report_json(r#"{"card":"铭牌","m":1}"#.as_bytes()).unwrap_err();
        assert!(e.contains("不认识的卡类型"), "{e}");
        let e = build_report_json(r#"{"card":"齿轮参数表","expr":"GEAR EX M2 Z40"}"#.as_bytes())
            .unwrap_err();
        assert!(!e.is_empty(), "缺 DA/DF 必须报错");
    }
}
