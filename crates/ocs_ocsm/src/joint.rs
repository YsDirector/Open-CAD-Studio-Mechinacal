//! `OCSMJOINT`：按**件链**装配螺栓副（确定性执行器）。
//!
//! 只做三件事，全部可复现、可回归测试：
//! 1. 把件链的高度加起来（板 / 垫圈 / 螺母 …）；
//! 2. 按"露出 N 扣"算出所需长度，并在该螺栓族的**表内供货长度系列**里取 ≥ 的最小值；
//!    表内系列是厂家实际可购档（GB/T 是推荐标准，厂家常做"接近标准"的增减档，例如 M8
//!    C 级除 40 起还有 35），所以**以表为准**、不按推标公称系列卡；取不到就报该族范围
//!    并提示换族（例如 C 级 M8 装不下 → 建议 5783 全螺纹）；
//! 3. 沿轴线把每个零件放到它该在的位置（基点约定来自零件族：螺栓 = 头部支承面 × 轴线、
//!    螺母/垫圈 = 端面 × 轴线）；并做**外形视图遮挡裁剪**：被螺母/垫圈包住的那段螺栓杆
//!    （含螺纹细实线）剪掉，只保留露在遮挡件之外的部分 —— 2D 装配图里被挡住的轮廓不画
//!    （否则螺母里会横穿一堆螺栓线，见 `clip_hidden`；`trim=off` 可关掉）。
//!
//! **刻意不判断"该不该加平垫/弹垫/防松件"** —— 那取决于安装面材料、振动环境、企业规范，
//! 由 skill/工况层决定并把件链传进来；命令侧只负责算得对、放得准、能撤销。
//!
//! 命令形式（给 MCP/AI 一行驱动）：
//! ```text
//! OCSMJOINT at 50,10 rot -90 [protrude 2.5] bolt=<族>:<d>[:<l>] [plate=<t>|gap=<t>|<垫圈/螺母族>=<d>]
//! ```
//! 例：`OCSMJOINT at 50,10 rot -90 bolt=hex_bolt_b_full:8 plate=10 plate=10 washer=washer_971:8 washer=washer_93:8 nut=nut_c41:8`

use crate::i18n::{t, t_fmt};
use crate::partgen::{self, family_kind};

/// 件链里的一件。
#[derive(Debug, Clone, PartialEq)]
pub enum JointItem {
    /// 螺栓/螺钉：决定轴线与长度基准（必须位于件链首位）。
    Bolt { family: String, d: f64, l: Option<f64> },
    Nut { family: String, d: f64 },
    Washer { family: String, d: f64 },
    /// 被连接件（按厚度记）：命令不去量它，厚度由调用方给（skill 从选区读）。
    Plate { t: f64 },
    /// 通用间隔件（如垫板、间隙）。
    Gap { t: f64 },
}

impl JointItem {
    /// 该件沿轴线占的高度；螺栓返回 None（它是长度基准，不计入 Σ）。
    fn height(&self) -> Option<f64> {
        match self {
            JointItem::Bolt { .. } => None,
            JointItem::Plate { t } | JointItem::Gap { t } => Some(*t),
            JointItem::Nut { family, d } | JointItem::Washer { family, d } => {
                partgen::size_row(family, *d).map(|row| row.l_min)
            }
        }
    }

    fn describe(&self) -> String {
        match self {
            JointItem::Bolt { family, d, l } => {
                let (name, code) = partgen::family_meta(family);
                match l {
                    Some(l) => t_fmt(
                        "cmd.joint.item.bolt_with_length",
                        &[("name", &name), ("code", &code), ("d", &label_d(*d)), ("l", &num(*l))],
                    ),
                    None => t_fmt(
                        "cmd.joint.item.bolt",
                        &[("name", &name), ("code", &code), ("d", &label_d(*d))],
                    ),
                }
            }
            JointItem::Nut { family, d } => {
                let (name, code) = partgen::family_meta(family);
                match partgen::size_row(family, *d) {
                    Some(row) => t_fmt(
                        "cmd.joint.item.nut",
                        &[("name", &name), ("code", &code), ("d", &label_d(*d)), ("thick", &num(row.l_min))],
                    ),
                    None => t_fmt(
                        "cmd.joint.item.bolt",
                        &[("name", &name), ("code", &code), ("d", &label_d(*d))],
                    ),
                }
            }
            JointItem::Washer { family, d } => {
                let (name, code) = partgen::family_meta(family);
                match partgen::size_row(family, *d) {
                    Some(row) => t_fmt(
                        "cmd.joint.item.washer",
                        &[("name", &name), ("code", &code), ("d", &label_d(*d)), ("thick", &num(row.l_min))],
                    ),
                    None => t_fmt(
                        "cmd.joint.item.bolt",
                        &[("name", &name), ("code", &code), ("d", &label_d(*d))],
                    ),
                }
            }
            JointItem::Plate { t } => t_fmt("cmd.joint.item.plate", &[("t", &num(*t))]),
            JointItem::Gap { t } => t_fmt("cmd.joint.item.gap", &[("t", &num(*t))]),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointSpec {
    /// 螺栓头部支承面所在的点（世界坐标 XY）。
    pub at: [f64; 2],
    /// 轴线方向（度，逆时针）：螺栓杆指向该方向，其余件沿它依次排开。
    pub rot_deg: f64,
    /// 期望露出螺母端的扣数（1 扣 = 1 个螺距），默认 2.5。
    pub protrude_turns: f64,
    /// 外形视图遮挡裁剪（默认开）：被螺母/垫圈包住的杆段不画。
    pub trim: bool,
    /// 视图覆盖（可选）：默认全族用 `main`——螺栓/螺母是“轴线水平”的侧视，
    /// 垫圈的主视图也已按用户要求改为侧视轮廓（平垫=矩形 h×d2、弹垫=月牙）。
    pub view: Option<String>,
    pub items: Vec<JointItem>,
}

impl JointSpec {
    pub const DEFAULT_PROTRUDE_TURNS: f64 = 2.5;

    /// 解析命令行参数（`at/rot/protrude` + 件链）；失败给出用法。
    pub fn parse(args: &str) -> Result<Self, String> {
        let usage = t("cmd.joint.usage");
        let mut tokens = args.split_whitespace();
        let mut at: Option<[f64; 2]> = None;
        let mut rot_deg = 0.0f64;
        let mut protrude_turns = Self::DEFAULT_PROTRUDE_TURNS;
        let mut trim = true;
        let mut view: Option<String> = None;
        let mut items: Vec<JointItem> = Vec::new();
        while let Some(token) = tokens.next() {
            let lower = token.to_ascii_lowercase();
            match lower.as_str() {
                "at" | "@" => {
                    let first = tokens.next().ok_or_else(|| {
                        t_fmt("cmd.joint.err.at_needs_coords", &[("usage", &usage)])
                    })?;
                    let (x, y) = match first.split_once(',') {
                        Some((x, y)) => (number(x)?, number(y)?),
                        None => (
                            number(&first)?,
                            number(&tokens.next().ok_or_else(|| {
                                t_fmt("cmd.joint.err.at_needs_y", &[("usage", &usage)])
                            })?)?,
                        ),
                    };
                    at = Some([x, y]);
                }
                "rot" | "rotation" => {
                    let raw = tokens.next().ok_or_else(|| {
                        t_fmt("cmd.joint.err.rot_needs_angle", &[("usage", &usage)])
                    })?;
                    rot_deg = number(&raw)?;
                }
                "protrude" => {
                    let raw = tokens.next().ok_or_else(|| {
                        t_fmt("cmd.joint.err.protrude_needs_number", &[("usage", &usage)])
                    })?;
                    protrude_turns = number(&raw)?;
                    if protrude_turns < 0.0 {
                        return Err(t("cmd.joint.err.protrude_negative"));
                    }
                }
                // `trim` / `trim on` = 开（默认）；`trim off` / `no-trim` / `notrim` = 关
                "trim" | "no-trim" | "notrim" => {
                    let mut off = lower != "trim";
                    if lower == "trim" {
                        // 只有明确跟一个 off/0/false 才关；否则是"开启"
                        if let Some(next) = tokens.clone().next() {
                            if matches!(next.to_ascii_lowercase().as_str(), "off" | "0" | "false" | "no") {
                                tokens.next();
                                off = true;
                            }
                        }
                    }
                    trim = !off;
                }
                "view" => {
                    let raw = tokens.next().ok_or_else(|| {
                        t_fmt("cmd.joint.err.view_needs_name", &[("usage", &usage)])
                    })?;
                    view = Some(raw.to_ascii_lowercase());
                }
                _ => items.push(parse_item(token)?),
            }
        }
        let at = at.ok_or_else(|| t_fmt("cmd.joint.err.at_required", &[("usage", &usage)]))?;
        if items.is_empty() {
            return Err(t_fmt("cmd.joint.err.empty_chain", &[("usage", &usage)]));
        }
        if !matches!(items.first(), Some(JointItem::Bolt { .. })) {
            return Err(t("cmd.joint.err.chain_needs_bolt"));
        }
        Ok(JointSpec { at, rot_deg, protrude_turns, trim, view, items })
    }

    /// HTTP/JSON 形式（`POST /api/joint` 用）。
    pub fn from_json(body: &[u8]) -> Result<Self, String> {
        #[derive(serde::Deserialize)]
        struct Raw {
            at: [f64; 2],
            #[serde(default)]
            rot: f64,
            #[serde(default = "default_protrude")]
            protrude: f64,
            #[serde(default)]
            view: Option<String>,
            #[serde(default = "default_true")]
            trim: bool,
            items: Vec<RawItem>,
        }
        #[derive(serde::Deserialize)]
        struct RawItem {
            kind: String,
            #[serde(default)]
            family: String,
            #[serde(default)]
            d: f64,
            #[serde(default)]
            l: Option<f64>,
            #[serde(default)]
            t: f64,
        }
        fn default_protrude() -> f64 {
            JointSpec::DEFAULT_PROTRUDE_TURNS
        }
        fn default_true() -> bool {
            true
        }
        let raw: Raw = serde_json::from_slice(body)
            .map_err(|e| t_fmt("cmd.joint.err.bad_json", &[("e", &e.to_string())]))?;
        let items = raw
            .items
            .into_iter()
            .map(|item| {
                Ok(match item.kind.to_ascii_lowercase().as_str() {
                    "bolt" => JointItem::Bolt { family: item.family, d: item.d, l: item.l },
                    "nut" => JointItem::Nut { family: item.family, d: item.d },
                    "washer" => JointItem::Washer { family: item.family, d: item.d },
                    "plate" => JointItem::Plate { t: item.t },
                    "gap" => JointItem::Gap { t: item.t },
                    other => return Err(t_fmt("cmd.joint.err.unknown_kind_json", &[("other", other)])),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if !matches!(items.first(), Some(JointItem::Bolt { .. })) {
            return Err(t("cmd.joint.err.chain_needs_bolt_json"));
        }
        Ok(JointSpec { at: raw.at, rot_deg: raw.rot, protrude_turns: raw.protrude, trim: raw.trim, view: raw.view, items })
    }

    pub fn to_json(&self) -> String {
        let items: Vec<serde_json::Value> = self
            .items
            .iter()
            .map(|item| match item {
                JointItem::Bolt { family, d, l } => serde_json::json!({"kind":"bolt","family":family,"d":d,"l":l}),
                JointItem::Nut { family, d } => serde_json::json!({"kind":"nut","family":family,"d":d}),
                JointItem::Washer { family, d } => serde_json::json!({"kind":"washer","family":family,"d":d}),
                JointItem::Plate { t } => serde_json::json!({"kind":"plate","t":t}),
                JointItem::Gap { t } => serde_json::json!({"kind":"gap","t":t}),
            })
            .collect();
        serde_json::json!({
            "at": self.at,
            "rot": self.rot_deg,
            "protrude": self.protrude_turns,
            "view": self.view,
            "items": items,
        })
        .to_string()
    }
}

/// 单件放置：基点坐标 + 视角（弧度）+ 该件的规格（插入用）。
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub family: String,
    pub d: f64,
    pub l: f64,
    pub at: [f64; 3],
    pub rot_rad: f64,
    /// 人类可读的规格文本（如 "M8x35"）。
    pub spec: String,
    /// 该件采用的视图 id（默认 `main`，见 `JointSpec::view`）。
    pub view: String,
    /// 该件的类（`bolt`/`screw`/`nut`/`washer`/`pin`/`ring`/`bearing`/`seal`/`other`）。
    /// 螺钉（`screw`）不是螺栓，不能进件链（`partgen::family_kind`）。
    pub kind: String,
    /// 基点沿装配轴到螺栓支承面的距离（mm）——件链的排布坐标。
    pub offset: f64,
    /// 该件沿装配轴自身占的区间 `[offset, offset + 轴向长]`（mm）。
    pub span: [f64; 2],
}

#[derive(Debug, Clone)]
pub struct JointPlan {
    /// 件链一句话（如 "板厚 10 + 板厚 10 + 六角螺母 C级 GB/T 41-2016 M8(m7.9)"）。
    pub chain_note: String,
    pub stack: f64,
    /// 长度需求（Σ + 露出）。
    pub need: f64,
    /// 实际外露长度（mm）与扣数。
    pub protrude_mm: f64,
    pub bolt_family: String,
    pub bolt_d: f64,
    pub bolt_l: f64,
    pub placements: Vec<Placement>,
    /// 一行推理（写进命令行日志，便于审计）。
    pub report: String,
}

/// 计算整个件链：长度落点（以表内供货系列为准）+ 各件基点。
pub fn plan(spec: &JointSpec) -> Result<JointPlan, String> {
    let Some(JointItem::Bolt { family, d, l: explicit_l }) = spec.items.first().cloned() else {
        return Err(t("cmd.joint.err.chain_needs_bolt_short"));
    };
    let row = partgen::size_row(&family, d).ok_or_else(|| {
        t_fmt(
            "cmd.joint.err.family_no_size",
            &[("family", &family), ("d", &label_d(d)), ("hint", &diameters_hint(&family))],
        )
    })?;
    if row.pitch <= 0.0 {
        return Err(t_fmt(
            "cmd.joint.err.no_pitch",
            &[("family", &family), ("d", &label_d(d))],
        ));
    }

    // ① Σ 件高（不含螺栓本身）
    let mut stack = 0.0f64;
    let mut chain_text: Vec<String> = Vec::new();
    for item in spec.items.iter().skip(1) {
        let h = item
            .height()
            .ok_or_else(|| t_fmt("cmd.joint.err.item_size_missing", &[("item", &item.describe())]))?;
        stack += h;
        chain_text.push(item.describe());
    }

    // ② 长度：Σ + 露出扣数×螺距，落标准长度系列
    let protrude_want = spec.protrude_turns * row.pitch;
    let need = stack + protrude_want;
    let bolt_l = match explicit_l {
        Some(l) => l,
        None => {
            let candidates: Vec<f64> = if row.lengths.is_empty() {
                // 表里没列 lengths 就按 5 递增在 [l_min, l_max] 内生成（少数族如此）。
                let mut v = Vec::new();
                let mut l = row.l_min;
                while l <= row.l_max + 1e-9 {
                    v.push(l);
                    l += 5.0;
                }
                v
            } else {
                row.lengths.clone()
            };
            candidates.iter().copied().find(|l| *l >= need - 1e-9).ok_or_else(|| {
                t_fmt(
                    "cmd.joint.err.length_too_short",
                    &[
                        ("family", &family),
                        ("d", &label_d(d)),
                        ("lmax", &num(row.l_max)),
                        ("need", &num(need)),
                    ],
                )
            })?
        }
    };
    let protrude_mm = bolt_l - stack;
    let protrude_turns = protrude_mm / row.pitch;

    // ③ 沿轴线排布：螺栓基点 = 头部支承面；其余件基点 = 前一序列的累积高度处
    let rot_rad = spec.rot_deg.to_radians();
    let axis = [rot_rad.cos(), rot_rad.sin()];
    let mut placements = Vec::new();
    let mut offset = 0.0f64;
    for (index, item) in spec.items.iter().enumerate() {
        let (family, d, l, spec_text) = match item {
            JointItem::Bolt { family, d, .. } => (family.clone(), *d, bolt_l, format!("{}{}", label_d(*d), format!("x{}", num(bolt_l)))),
            JointItem::Nut { family, d } => (family.clone(), *d, 0.0, label_d(*d)),
            JointItem::Washer { family, d } => (family.clone(), *d, 0.0, label_d(*d)),
            JointItem::Plate { t } | JointItem::Gap { t } => {
                offset += *t;
                continue;
            }
        };
        let at = [
            spec.at[0] + axis[0] * offset,
            spec.at[1] + axis[1] * offset,
            0.0,
        ];
        let view = spec.view.clone().unwrap_or_else(|| "main".to_string());
        // 沿轴的占位区间：螺栓 = 杆长（头部在支承面之后，不参与遮挡）；其余 = 自身高度
        let axial = if index == 0 { bolt_l } else { item.height().unwrap_or(0.0) };
        let kind = family_kind(&family).to_string();
        placements.push(Placement {
            family,
            d,
            l,
            at,
            rot_rad,
            spec: spec_text,
            view,
            kind,
            offset,
            span: [offset, offset + axial],
        });
        if index > 0 {
            // 该件自身占用 offset 之后的空间（螺栓是长度基准，不推进件链坐标）
            offset += axial;
        }
    }

    let (bolt_name, bolt_code) = partgen::family_meta(&family);
    let chain_note = if chain_text.is_empty() {
        t("cmd.joint.note.no_plates")
    } else {
        chain_text.join(" + ")
    };
    let report = t_fmt(
        "cmd.joint.report",
        &[
            ("bolt_name", &bolt_name),
            ("bolt_code", &bolt_code),
            ("d", &label_d(d)),
            ("l", &num(bolt_l)),
            ("chain", &chain_note),
            ("stack", &num(stack)),
            ("need", &num(need)),
            ("turns", &num(spec.protrude_turns)),
            ("pitch", &num(row.pitch)),
            ("l2", &num(bolt_l)),
            ("pm", &num(protrude_mm)),
            ("pt", &num(protrude_turns.round_at(1))),
            ("x", &num(spec.at[0])),
            ("y", &num(spec.at[1])),
            ("rot", &num(spec.rot_deg)),
            ("n", &placements.len().to_string()),
        ],
    );

    Ok(JointPlan {
        chain_note,
        stack,
        need,
        protrude_mm,
        bolt_family: family,
        bolt_d: d,
        bolt_l,
        placements,
        report,
    })
}

trait RoundAt {
    fn round_at(self, digits: u32) -> f64;
}
impl RoundAt for f64 {
    fn round_at(self, digits: u32) -> f64 {
        let factor = 10f64.powi(digits as i32);
        (self * factor).round() / factor
    }
}

fn diameters_hint(family: &str) -> String {
    let list: Vec<String> = partgen::family_sizes(family).iter().map(|r| label_d(r.d)).collect();
    if list.is_empty() {
        t("cmd.joint.err.family_not_implemented")
    } else {
        list.join(", ")
    }
}

fn parse_item(token: &str) -> Result<JointItem, String> {
    let (key, value) = token
        .split_once('=')
        .ok_or_else(|| {
            t_fmt("cmd.joint.err.item_format", &[("token", token)])
        })?;
    let key = key.to_ascii_lowercase();
    match key.as_str() {
        "plate" | "p" | "板" => return Ok(JointItem::Plate { t: number(value)? }),
        "gap" | "g" | "间隔" => return Ok(JointItem::Gap { t: number(value)? }),
        _ => {}
    }
    // 写法①：直接写族名：`hex_bolt_c=8[:40]`、`nut_c41=8`、`washer_971=8`
    if partgen::family_kind(&key) != "other" {
        let (d, l) = parse_size(value)?;
        return item_for(key, d, l);
    }
    // 写法②：件类型 + 族名：`bolt=hex_bolt_b_full:8[:40]`、`nut=nut_c41:8`
    let (family, rest) = value.split_once(':').ok_or_else(|| {
        t_fmt("cmd.joint.err.item_colon", &[("token", token)])
    })?;
    let family = family.to_ascii_lowercase();
    let (d, l) = parse_size(rest)?;
    let expected = match key.as_str() {
        "bolt" | "screw_bolt" => "bolt",
        "screw" | "螺钉" => {
            return Err(t("cmd.joint.err.no_screw"))
        }
        "nut" => "nut",
        "washer" => "washer",
        other => return Err(t_fmt("cmd.joint.err.unknown_kind", &[("other", other)])),
    };
    if partgen::family_kind(&family) != expected {
        let kind = partgen::family_kind(&family);
        return Err(if kind == "screw" {
            t_fmt(
                "cmd.joint.err.screw_not_expected",
                &[("family", &family), ("expected", expected)],
            )
        } else {
            t_fmt(
                "cmd.joint.err.family_kind_mismatch",
                &[("family", &family), ("expected", expected)],
            )
        });
    }
    item_for(family, d, l)
}

/// `d[:l]`（可选显式长度）。
fn parse_size(raw: &str) -> Result<(f64, Option<f64>), String> {
    let mut parts = raw.split(':');
    let d = number(parts.next().unwrap_or(""))?;
    let l = match parts.next() {
        Some(text) if !text.trim().is_empty() => Some(number(text)?),
        _ => None,
    };
    if parts.next().is_some() {
        return Err(t_fmt("cmd.joint.err.size_format", &[("raw", raw)]));
    }
    Ok((d, l))
}

fn item_for(family: String, d: f64, l: Option<f64>) -> Result<JointItem, String> {
    if partgen::size_row(&family, d).is_none() {
        return Err(t_fmt(
            "cmd.joint.err.library_no_d",
            &[("family", &family), ("d", &num(d)), ("hint", &diameters_hint(&family))],
        ));
    }
    Ok(match partgen::family_kind(&family) {
        "bolt" => JointItem::Bolt { family, d, l },
        "nut" => JointItem::Nut { family, d },
        "washer" => JointItem::Washer { family, d },
        // 螺钉是另一类东西：自带头部、拧入螺纹孔，不配螺母 —— 不能当件链的"螺栓"使
        "screw" => {
            return Err(t_fmt("cmd.joint.err.screw_not_bolt", &[("family", &family)]))
        }
        other => return Err(t_fmt("cmd.joint.err.kind_unsupported", &[("family", &family), ("other", other)])),
    })
}

// ── 外形视图的遮挡裁剪 ─────────────────────────────────────────────────────
//
// 2D 装配（外形）视图里，被别的零件挡住的轮廓**不画**：螺母/垫圈套在螺杆上时，
// 它们包住的那段螺杆轮廓线和螺纹细实线都要剪掉，只留露在遮挡件之外的部分。
// 这些件在库里是整件块（块定义不能裁剪），所以裁剪在**生成几何之后、写块之前**做：
// 被裁剪的件用一个带裁剪标记的块名（`…_CUT23.7x31.6`）建块，块定义本身即裁好的几何。

/// 该件是不是"环形遮挡件"（螺母/垫圈：径向包住杆）。
fn is_ring(kind: &str) -> bool {
    matches!(kind, "nut" | "washer")
}

/// 该件是不是"被包住的杆件"。
fn is_shaft(kind: &str) -> bool {
    matches!(kind, "bolt")
}

/// 算出每件被遮挡的轴向区间（换算到该件的**局部** x 坐标，基点为 0）。
///
/// `outer_radii[i]` = 第 i 件**前段**（基点之后，即伸进件链的那部分）的径向半尺寸：
/// 只有遮挡件的径向尺寸不小于被遮件时才算遮得住（否则轮廓会从旁边露出来，不能剪）。
/// 用"前段"而不是整件 bbox，是因为螺栓的 bbox 被头部撑大（M8 头对角 15 > 弹垫外径
/// 12.3），拿整件比会把弹垫误判成"遮不住杆"——它明明套在杆上。
/// 返回的区间已按起点排序、合并重叠。
pub fn hidden_spans(
    placements: &[Placement],
    parts: &[Vec<ocs_plugin_api::host::acadrust::EntityType>],
) -> Vec<Vec<(f64, f64)>> {
    let mut out: Vec<Vec<(f64, f64)>> = vec![Vec::new(); placements.len()];
    for (i, covered) in placements.iter().enumerate() {
        if !is_shaft(&covered.kind) {
            continue;
        }
        let Some(shaft) = parts.get(i) else { continue };
        let mut spans: Vec<(f64, f64)> = Vec::new();
        for (j, cover) in placements.iter().enumerate() {
            if i == j || !is_ring(&cover.kind) {
                continue;
            }
            let Some(ring) = parts.get(j) else { continue };
            // 两件轴向区间相交的部分 = 被遮住的那段（换算到被遮件的局部 x）
            let a = covered.span[0].max(cover.span[0]) - covered.offset;
            let b = covered.span[1].min(cover.span[1]) - covered.offset;
            if b - a <= 1e-9 {
                continue;
            }
            // 就地比粗细：环形件轮廓 ≥ 杆在这段窗口里的径向尺寸 → 挡得住
            if silhouette_radius(ring) + 1e-9 >= window_radius(shaft, a, b) {
                spans.push((a, b));
            }
        }
        spans.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        // 合并重叠
        let mut merged: Vec<(f64, f64)> = Vec::new();
        for (a, b) in spans {
            match merged.last_mut() {
                Some(last) if a <= last.1 + 1e-9 => last.1 = last.1.max(b),
                _ => merged.push((a, b)),
            }
        }
        out[i] = merged;
    }
    out
}

/// 一个实体的 y 方向半尺寸（含 |y| 极值）与 x 范围：轮廓求极值用。
fn entity_bbox(entity: &ocs_plugin_api::host::acadrust::EntityType) -> Option<(f64, f64, f64)> {
    use ocs_plugin_api::host::acadrust::EntityType;
    match entity {
        EntityType::Line(l) => Some((
            l.start.x.min(l.end.x),
            l.start.x.max(l.end.x),
            l.start.y.abs().max(l.end.y.abs()),
        )),
        EntityType::Circle(c) => Some((
            c.center.x - c.radius,
            c.center.x + c.radius,
            c.center.y.abs() + c.radius,
        )),
        EntityType::Arc(a) => {
            // 弧只扫一段：用两端点 + 扫幅内的角度极值点近似 bbox（够判遮挡，不追求精确包围盒）
            let (mut x0, mut x1) = (a.center.x, a.center.x);
            let mut ymax = a.center.y.abs();
            let a0 = a.start_angle;
            let a1 = a.end_angle;
            let sweep = {
                let mut d = a1 - a0;
                while d < 0.0 {
                    d += std::f64::consts::TAU;
                }
                d
            };
            for t in [0.0, 1.0, 0.25, 0.5, 0.75] {
                let ang = a0 + sweep * t;
                let (x, y) = (
                    a.center.x + a.radius * ang.cos(),
                    a.center.y + a.radius * ang.sin(),
                );
                x0 = x0.min(x);
                x1 = x1.max(x);
                ymax = ymax.max(y.abs());
            }
            Some((x0, x1, ymax))
        }
        EntityType::LwPolyline(p) => {
            let mut x0 = f64::INFINITY;
            let mut x1 = f64::NEG_INFINITY;
            let mut ymax: f64 = 0.0;
            for v in &p.vertices {
                x0 = x0.min(v.location.x);
                x1 = x1.max(v.location.x);
                ymax = ymax.max(v.location.y.abs());
            }
            if x0.is_finite() {
                Some((x0, x1, ymax))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// 件整体轮廓的径向半尺寸（螺母/垫圈的"能挡住多宽"）。
///
/// 环形件用它自己的轮廓极值——平垫 Ø16/2=8、弹垫 Ø12.3/2≈6.2、螺母对角 15/2=7.5。
pub fn silhouette_radius(entities: &[ocs_plugin_api::host::acadrust::EntityType]) -> f64 {
    entities
        .iter()
        .filter_map(entity_bbox)
        .map(|(_, _, y)| y)
        .fold(0.0f64, f64::max)
}

/// 被遮件在 x 窗口 `[lo, hi]`（件局部坐标）里的径向半尺寸——遮挡判据要"就地比粗细"。
///
/// 为什么不用件整体 bbox：螺栓整体 bbox 被头部撑大（M8 对角 15/2=7.5 > 弹垫 6.2），
/// 拿整件比会把"明明套在杆上的弹垫"误判成遮不住；而且杆在窗口里其实只有 Ø8/2=4。
pub fn window_radius(
    entities: &[ocs_plugin_api::host::acadrust::EntityType],
    lo: f64,
    hi: f64,
) -> f64 {
    use ocs_plugin_api::host::acadrust::EntityType;
    let mut r: f64 = 0.0;
    for entity in entities {
        let Some((x0, x1, y)) = entity_bbox(entity) else {
            continue;
        };
        if matches!(entity, EntityType::Line(_)) {
            // 直线：把 y 按 x 线性插值到窗口内（斜线只取窗口内那一段的极值）
            if let EntityType::Line(l) = entity {
                let (xa, ya, xb, yb) = if l.start.x <= l.end.x {
                    (l.start.x, l.start.y, l.end.x, l.end.y)
                } else {
                    (l.end.x, l.end.y, l.start.x, l.start.y)
                };
                let at = |x: f64| {
                    if (xb - xa).abs() < 1e-12 {
                        ya.max(yb)
                    } else {
                        ya + (yb - ya) * (x - xa) / (xb - xa)
                    }
                };
                if x1 < lo - 1e-9 || x0 > hi + 1e-9 {
                    continue; // 不在窗口里
                }
                let a = at(xa.max(lo));
                let b = at(xb.min(hi));
                r = r.max(a.abs()).max(b.abs());
            }
            continue;
        }
        // 别的实体：整体落在窗口内才算进来，否则按"可能更宽"处理（保守 → 不轻易剪）
        if lo - 1e-9 <= x0 && x1 <= hi + 1e-9 {
            r = r.max(y);
        } else if !(x1 < lo - 1e-9 || x0 > hi + 1e-9) {
            r = r.max(y);
        }
    }
    r
}

/// 裁剪标记（拼进块名，保证"同一裁剪"复用同一块、"不同裁剪"不互相污染）。
pub fn cut_tag(spans: &[(f64, f64)]) -> String {
    spans
        .iter()
        .map(|(a, b)| format!("{}x{}", num(*a), num(*b)))
        .collect::<Vec<_>>()
        .join("+")
}

/// 把一个件的几何按被遮挡区间裁剪（局部 x 坐标）。
///
/// - 直线：按 x 区间做减集 —— 整段被遮 → 删；一头被遮 → 截短；中间被遮 → 断成两段
///   （斜线按参数插值取点，不改变方向）；
/// - 圆弧/圆：整体落在遮挡区间内才删（保守，不切弧）；
/// - **中心线层不动**（轴线是装配基准，要通长画）；
/// - 其它实体（文字等）不动。
pub fn clip_hidden(
    entities: &[ocs_plugin_api::host::acadrust::EntityType],
    spans: &[(f64, f64)],
) -> (Vec<ocs_plugin_api::host::acadrust::EntityType>, usize) {
    use ocs_plugin_api::host::acadrust::EntityType;
    // 把 [x1, x2] 减去遮挡区间，返回还留下的子区间（按原方向）。
    fn subtract(x1: f64, x2: f64, spans: &[(f64, f64)]) -> Vec<(f64, f64)> {
        let (lo, hi) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let mut pieces = vec![(lo, hi)];
        for (a, b) in spans {
            let mut next: Vec<(f64, f64)> = Vec::new();
            for (p, q) in pieces {
                let (a, b) = (*a, *b);
                if b <= p + 1e-12 || a >= q - 1e-12 {
                    next.push((p, q)); // 不重叠
                    continue;
                }
                if a > p + 1e-12 {
                    next.push((p, a.min(q)));
                }
                if b < q - 1e-12 {
                    next.push((b.max(p), q));
                }
            }
            pieces = next;
        }
        if x1 > x2 {
            pieces.into_iter().map(|(p, q)| (q, p)).collect()
        } else {
            pieces
        }
    }

    let mut out = Vec::with_capacity(entities.len());
    let mut removed = 0usize;
    for entity in entities {
        let EntityType::Line(line) = entity else {
            // 圆弧/圆等：只做"整体被遮则删"，不做部分裁剪
            let hidden_whole = match entity {
                EntityType::Arc(a) => {
                    let (lo, hi) = (a.center.x - a.radius, a.center.x + a.radius);
                    spans.iter().any(|(p, q)| *p <= lo + 1e-9 && hi <= *q + 1e-9)
                }
                EntityType::Circle(c) => {
                    let (lo, hi) = (c.center.x - c.radius, c.center.x + c.radius);
                    spans.iter().any(|(p, q)| *p <= lo + 1e-9 && hi <= *q + 1e-9)
                }
                _ => false,
            };
            if hidden_whole {
                removed += 1;
            } else {
                out.push(entity.clone());
            }
            continue;
        };
        if line.common.layer == crate::partgen::LAYER_CENTER {
            out.push(entity.clone()); // 轴线通长，不裁
            continue;
        }
        let (x1, x2) = (line.start.x, line.end.x);
        if spans.is_empty() {
            out.push(entity.clone());
            continue;
        }
        let pieces = subtract(x1, x2, spans);
        if pieces.is_empty() {
            removed += 1;
            continue;
        }
        if pieces.len() == 1 && (pieces[0].0 - x1).abs() < 1e-12 && (pieces[0].1 - x2).abs() < 1e-12 {
            out.push(entity.clone()); // 没被切到
            continue;
        }
        for (p, q) in pieces {
            if (q - p).abs() < 1e-9 {
                continue; // 退化成点
            }
            let (t1, t2) = ((p - x1) / (x2 - x1), (q - x1) / (x2 - x1));
            let mut piece = line.clone();
            piece.start.x = p;
            piece.end.x = q;
            piece.start.y = line.start.y + (line.end.y - line.start.y) * t1;
            piece.end.y = line.start.y + (line.end.y - line.start.y) * t2;
            out.push(EntityType::Line(piece));
        }
    }
    (out, removed)
}

// ── 整链几何构建（命令插入 / GUI 预览 / GUI SVG 预览共用）────────────────

/// 一次装配的完整几何产物：各件裁好的几何 + 整装图（装配坐标，基点 = 螺栓支承面）。
pub struct JointBuild {
    /// 各件（局部坐标，基点在自己的原点；`entities` 已裁剪）。
    pub parts: Vec<crate::partgen::GenPart>,
    /// 各件被遮挡的区间（件链坐标，mm；空 = 没裁）。
    pub hidden: Vec<Vec<(f64, f64)>>,
    /// 裁剪说明（写进报告/命令行日志）。
    pub notes: Vec<String>,
    /// 整装图：所有件平移到各自轴向位置后的**合并几何**（给 SVG 预览与光标预览块用）。
    /// 各件仍是独立实体，但已落在装配坐标里，可以直接渲染/建一个预览块。
    pub assembly: crate::partgen::GenPart,
}

/// 生成整链几何：逐件生成 → 算遮挡区间 → 裁剪 → 合并成整装图。
///
/// `trim=false` 时完全跳过遮挡裁剪（要"全件实画"时用）。
pub fn build(plan: &JointPlan, trim: bool) -> Result<JointBuild, String> {
    use ocs_plugin_api::host::acadrust::EntityType;
    let mut parts: Vec<crate::partgen::GenPart> = Vec::new();
    for placement in &plan.placements {
        parts.push(crate::partgen::generate(
            &placement.family,
            placement.d,
            placement.l,
            &placement.view,
        )?);
    }
    let mut geometry: Vec<Vec<EntityType>> = parts.iter().map(|p| p.entities.clone()).collect();
    let hidden = if trim {
        hidden_spans(&plan.placements, &geometry)
    } else {
        vec![Vec::new(); plan.placements.len()]
    };
    let mut notes: Vec<String> = Vec::new();
    for (index, spans) in hidden.iter().enumerate() {
        if spans.is_empty() {
            continue;
        }
        let (clipped, _removed) = clip_hidden(&geometry[index], spans);
        let (name, _) = crate::partgen::family_meta(&plan.placements[index].family);
        let iv = spans
            .iter()
            .map(|(a, b)| format!("{}~{}", num(*a), num(*b)))
            .collect::<Vec<_>>()
            .join("、");
        notes.push(t_fmt(
            "cmd.joint.note.hidden",
            &[("name", &name), ("spec", &parts[index].meta.spec), ("iv", &iv)],
        ));
        parts[index].entities = clipped.clone();
        geometry[index] = clipped;
    }

    // 合并成整装图（装配坐标：件链沿 +x，基点 = 螺栓头部支承面）
    let mut merged: Vec<EntityType> = Vec::new();
    let mut bbox = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for (index, placement) in plan.placements.iter().enumerate() {
        for entity in &geometry[index] {
            let moved = shift_entity(entity, placement.offset);
            if let Some((x0, x1, y)) = entity_bbox(&moved) {
                bbox[0] = bbox[0].min(x0);
                bbox[1] = bbox[1].min(-y);
                bbox[2] = bbox[2].max(x1);
                bbox[3] = bbox[3].max(y);
            }
            merged.push(moved);
        }
    }
    if !bbox[0].is_finite() {
        return Err(t("cmd.joint.err.empty_geometry"));
    }
    let mut assembly = parts
        .first()
        .cloned()
        .ok_or_else(|| t("cmd.joint.err.empty_chain_bare"))?;
    assembly.entities = merged;
    assembly.bbox = bbox;
    Ok(JointBuild { parts, hidden, notes, assembly })
}

/// 实体沿轴线平移（装配坐标 = 件局部 x + 件基点偏移）。
fn shift_entity(
    entity: &ocs_plugin_api::host::acadrust::EntityType,
    dx: f64,
) -> ocs_plugin_api::host::acadrust::EntityType {
    use ocs_plugin_api::host::acadrust::EntityType;
    if dx == 0.0 {
        return entity.clone();
    }
    let mut out = entity.clone();
    match &mut out {
        EntityType::Line(l) => {
            l.start.x += dx;
            l.end.x += dx;
        }
        EntityType::Circle(c) => c.center.x += dx,
        EntityType::Arc(a) => a.center.x += dx,
        EntityType::LwPolyline(p) => {
            for v in &mut p.vertices {
                v.location.x += dx;
            }
        }
        _ => {}
    }
    out
}

fn number(raw: &str) -> Result<f64, String> {
    let value: f64 = raw
        .trim()
        .trim_end_matches(['m', 'M', '°', '度'])
        .parse()
        .map_err(|_| t_fmt("cmd.joint.err.not_number", &[("raw", raw)]))?;
    if !value.is_finite() {
        return Err(t_fmt("cmd.joint.err.not_finite", &[("raw", raw)]));
    }
    Ok(value)
}

fn label_d(d: f64) -> String {
    format!("M{}", num(d))
}

/// 数字 → 去尾零文本（报告/块名/对外 JSON 用）。名字避开 `trim` 开关参数。
pub fn num_text(v: f64) -> String {
    num(v)
}

fn num(v: f64) -> String {
    let mut text = format!("{v:.3}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 语言是进程级全局：断言中文文案的用例与「切语言」用例共用锁并钉死 zh。
    fn zh_guard() -> std::sync::MutexGuard<'static, ()> {
        let g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        g
    }

    #[test]
    fn parses_command_line_forms() {
        let _g = zh_guard();
        let spec = JointSpec::parse(
            "at 50,10 rot -90 bolt=hex_bolt_b_full:8 plate=10 plate=10 nut=nut_c41:8",
        )
        .unwrap();
        assert_eq!(spec.at, [50.0, 10.0]);
        assert_eq!(spec.rot_deg, -90.0);
        assert_eq!(spec.protrude_turns, 2.5);
        assert_eq!(spec.items.len(), 4);
        assert_eq!(spec.items[0], JointItem::Bolt { family: "hex_bolt_b_full".into(), d: 8.0, l: None });

        // 空格分隔的 at、显式长度、大小写混写
        let spec = JointSpec::parse("AT 10 20 ROT 90 BOLT=HEX_BOLT_B_FULL:8:40 plate=5").unwrap();
        assert_eq!(spec.at, [10.0, 20.0]);
        assert_eq!(spec.rot_deg, 90.0);
        assert_eq!(spec.items[0], JointItem::Bolt { family: "hex_bolt_b_full".into(), d: 8.0, l: Some(40.0) });

        // 缺 at / 件链不以 bolt 开头 / 未知族 → 报错
        assert!(JointSpec::parse("bolt=hex_bolt_c:8 plate=10").is_err());
        assert!(JointSpec::parse("at 0,0 plate=10").is_err());
        assert!(JointSpec::parse("at 0,0 bolt=nut_c41:8").is_err());

        // 螺钉 ≠ 螺栓（用户 2026-09-16）：件链两种写法都不收螺钉（拧入螺纹孔、不配螺母）
        for probe in [
            "at 0,0 socket_head=8:30 plate=10",       // 写法①直接写族名
            "at 0,0 bolt=socket_head:8:30 plate=10",  // 写法②显式 bolt=
            "at 0,0 screw=hex_bolt_c:8 plate=10",     // 旧的 screw= 别名已取消
        ] {
            let err = JointSpec::parse(probe).unwrap_err();
            assert!(
                err.contains("螺钉") || err.contains("只支持螺栓"),
                "{probe} 应被拒且提示螺钉/螺栓之别，实际: {err}"
            );
        }
    }

    // ── 遮挡裁剪 ────────────────────────────────────────────────────────────

    fn line_of(x1: f64, y1: f64, x2: f64, y2: f64, layer: &str) -> ocs_plugin_api::host::acadrust::EntityType {
        use ocs_plugin_api::host::acadrust::{entities::Line as AcadLine, types::Vector3, EntityType};
        let mut l = AcadLine::from_points(
            Vector3::new(x1, y1, 0.0),
            Vector3::new(x2, y2, 0.0),
        );
        l.common.layer = layer.to_string();
        EntityType::Line(l)
    }

    /// 实际生成的 M8×35 全螺纹：取 y=±d/2 的杆轮廓线和 y=±dm/2 的螺纹细实线。
    fn shank_segments(entities: &[ocs_plugin_api::host::acadrust::EntityType], y: f64) -> Vec<(f64, f64)> {
        use ocs_plugin_api::host::acadrust::EntityType;
        entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Line(l) if (l.start.y - y).abs() < 1e-9 && (l.end.y - y).abs() < 1e-9 => {
                    Some((l.start.x.min(l.end.x), l.start.x.max(l.end.x)))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn clip_hidden_trims_partial_covered_and_keeps_center_line() {
        use ocs_plugin_api::host::acadrust::EntityType;
        let main = crate::partgen::LAYER_MAIN;
        let center = crate::partgen::LAYER_CENTER;
        let entities = vec![
            line_of(4.2, 0.0, 5.8, 0.0, main),     // 整段落在遮挡区 → 删
            line_of(0.0, 1.0, 10.0, 1.0, main),    // 跨遮挡区间 4~6 → 断成两段
            line_of(0.0, 2.0, 5.0, 2.0, main),     // 尾巴被遮 → 截到 4
            line_of(5.0, 3.0, 9.0, 3.0, main),     // 头被遮 → 从 6 起
            line_of(7.0, 4.0, 7.0, -4.0, main),    // 竖线在遮挡区外 → 留
            line_of(5.0, 5.0, 5.0, -5.0, main),    // 竖线在遮挡区内 → 删
            line_of(-2.0, 0.0, 12.0, 0.0, center), // 轴线：通长不裁
        ];
        let (out, removed) = clip_hidden(&entities, &[(4.0, 6.0)]);
        assert_eq!(removed, 2, "整段被遮 1 + 竖线在区内 1");
        // 杆线：y=1 断成 [0,4]+[6,10]
        let segs = shank_segments(&out, 1.0);
        assert_eq!(segs.len(), 2, "{out:?}");
        assert!(segs.contains(&(0.0, 4.0)) && segs.contains(&(6.0, 10.0)), "{segs:?}");
        // y=2 → [0,4]；y=3 → [6,9]
        assert_eq!(shank_segments(&out, 2.0), vec![(0.0, 4.0)]);
        assert_eq!(shank_segments(&out, 3.0), vec![(6.0, 9.0)]);
        // 轴线还在（原样两头）
        let axis: Vec<(f64, f64)> = out
            .iter()
            .filter_map(|e| match e {
                EntityType::Line(l) if l.common.layer == center => Some((l.start.x, l.end.x)),
                _ => None,
            })
            .collect();
        assert_eq!(axis, vec![(-2.0, 12.0)], "轴线必须通长");
    }

    #[test]
    fn clip_hidden_drops_arc_fully_inside_but_keeps_partial() {
        use ocs_plugin_api::host::acadrust::{
            entities::Arc as AcadArc, types::Vector3, EntityType,
        };
        let arc = |cx: f64, r: f64| {
            let mut a = AcadArc::from_center_radius_angles(
                Vector3::new(cx, 0.0, 0.0),
                r,
                0.0,
                std::f64::consts::PI,
            );
            a.common.layer = crate::partgen::LAYER_MAIN.to_string();
            EntityType::Arc(a)
        };
        let (out, removed) = clip_hidden(&[arc(30.0, 1.0), arc(4.0, 1.0)], &[(28.0, 32.0)]);
        assert_eq!(removed, 1, "只有完全落在遮挡区里的那条弧被删");
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn hidden_spans_cover_shank_under_washer_and_nut() {
        // 两板 20 + 平垫 1.6 + 弹垫 2.1 + 螺母 7.9 → 遮挡区间 [20, 31.6]（连续，合并成一段）
        let spec = JointSpec::parse(
            "at 0,0 bolt=hex_bolt_b_full:8 plate=10 plate=10 washer=washer_971:8 washer=washer_93:8 nut=nut_c41:8",
        )
        .unwrap();
        let plan = plan(&spec).unwrap();
        assert_eq!(plan.placements.len(), 4);
        assert!(
            (plan.placements[3].span[0] - 23.7).abs() < 1e-9
                && (plan.placements[3].span[1] - 31.6).abs() < 1e-9,
            "螺母占位 {:?}",
            plan.placements[3].span
        );
        let parts: Vec<Vec<ocs_plugin_api::host::acadrust::EntityType>> = plan
            .placements
            .iter()
            .map(|p| {
                crate::partgen::generate(&p.family, p.d, p.l, &p.view)
                    .unwrap()
                    .entities
            })
            .collect();
        // 轮廓径向：平垫 Ø16/2、弹垫 Ø12.3/2、螺母 15/2；杆在那段窗口里只有 Ø8/2=4
        assert!((silhouette_radius(&parts[1]) - 8.0).abs() < 1e-9, "平垫 {:?}", silhouette_radius(&parts[1]));
        assert!(silhouette_radius(&parts[2]) > 6.0, "弹垫 {:?}", silhouette_radius(&parts[2]));
        assert!((window_radius(&parts[0], 20.0, 31.6) - 4.0).abs() < 1e-9, "杆在窗口里 {:?}", window_radius(&parts[0], 20.0, 31.6));
        let spans = hidden_spans(&plan.placements, &parts);
        assert_eq!(spans[0].len(), 1, "垫圈+螺母连续 → 合并成一段：{:?}", spans[0]);
        assert!(
            (spans[0][0].0 - 20.0).abs() < 1e-9 && (spans[0][0].1 - 31.6).abs() < 1e-9,
            "螺栓杆被垫圈+螺母遮住 20~31.6，实得 {:?}",
            spans[0]
        );
        for (i, s) in spans.iter().enumerate().skip(1) {
            assert!(s.is_empty(), "第 {i} 件不该被遮：{s:?}");
        }
        assert_eq!(cut_tag(&spans[0]), "20x31.6");
    }

    #[test]
    fn narrow_spring_washer_still_covers_the_shank() {
        // 回归：M8 弹垫外径 Ø12.3 < 螺栓头对角 15 → 若拿"整件 bbox"比粗细会误判成遮不住，
        // 实际它套在 Ø8 杆上，必须算遮得住。
        let spec = JointSpec::parse("at 0,0 bolt=hex_bolt_b_full:8 plate=10 plate=10 washer=washer_93:8 nut=nut_c41:8")
            .unwrap();
        let plan = plan(&spec).unwrap();
        let parts: Vec<Vec<ocs_plugin_api::host::acadrust::EntityType>> = plan
            .placements
            .iter()
            .map(|p| {
                crate::partgen::generate(&p.family, p.d, p.l, &p.view)
                    .unwrap()
                    .entities
            })
            .collect();
        let spans = hidden_spans(&plan.placements, &parts);
        assert_eq!(spans[0].len(), 1, "弹垫+螺母连续，合成一段：{:?}", spans[0]);
        assert!((spans[0][0].0 - 20.0).abs() < 1e-9, "遮挡从板背面(20)起：{:?}", spans[0]);
        // 弹垫 2.1 + 螺母 7.9 = 10 → 遮到 30.0
        assert!((spans[0][0].1 - 30.0).abs() < 1e-9, "遮挡到螺母端面(30)：{:?}", spans[0]);
    }

    #[test]
    fn clipped_bolt_keeps_the_exposed_thread_and_end_face() {
        // 真实几何：M8×35 杆被 20~31.6 遮住后，露出的两段（0~20、31.6~34.925）都还在，
        // 杆端倒角/端面竖线（x=35）和轴线保留，遮挡段内没有杆线。
        use ocs_plugin_api::host::acadrust::EntityType;
        let part = crate::partgen::generate("hex_bolt_b_full", 8.0, 35.0, "main").unwrap();
        let spans = vec![(20.0, 31.6)];
        let (out, removed) = clip_hidden(&part.entities, &spans);
        assert_eq!(removed, 0, "杆线是跨区间的（断开而非删除），不该有整段被删");
        assert!(
            out.len() > part.entities.len(),
            "断开后实体数应变多：{} → {}",
            part.entities.len(),
            out.len()
        );
        let y8 = shank_segments(&out, 4.0); // 杆轮廓 ±d/2
        let yt = shank_segments(&out, 0.85 * 8.0 / 2.0); // 螺纹细实线 ±dm/2
        for (label, segs) in [("杆线", &y8), ("螺纹线", &yt)] {
            assert!(
                segs.iter().any(|(_, b)| (*b - 20.0).abs() < 1e-9),
                "{label} 应保留到 20（遮挡起点）：{segs:?}"
            );
            assert!(
                segs.iter().any(|(a, b)| (*a - 31.6).abs() < 1e-9 && *b > 34.0),
                "{label} 应保留 31.6~杆端：{segs:?}"
            );
            for (a, b) in segs {
                let mid = (a + b) / 2.0;
                assert!(!(20.0 < mid && mid < 31.6), "{label} 遮挡段里不该有线：{segs:?}");
            }
        }
        // 杆轮廓从支承面（x=0）起；螺纹细实线从收尾之后（x=l−a）起
        assert!(y8.iter().any(|(a, _)| a.abs() < 1e-9), "杆轮廓应从支承面起：{y8:?}");
        assert!(yt.iter().all(|(a, _)| *a >= 1.0), "螺纹细实线不该画到头部里：{yt:?}");
        // 端面竖线 x=35 与倒角竖线 x=34.925 保留，轴线通长
        let verticals: Vec<f64> = out
            .iter()
            .filter_map(|e| match e {
                EntityType::Line(l) if (l.start.x - l.end.x).abs() < 1e-9 => Some(l.start.x),
                _ => None,
            })
            .collect();
        assert!(verticals.iter().any(|x| (*x - 35.0).abs() < 1e-9), "{verticals:?}");
        let (_, removed_all) = clip_hidden(&part.entities, &[]);
        assert_eq!(removed_all, 0, "不给遮挡区间时不该动任何线");
    }

    #[test]
    fn plan_lands_on_standard_length_like_the_20mm_stack_case() {
        // 用户工况：两块 10mm 板 + M8（Ø10 孔）；GB/T 5783 全螺纹、GB/T 41 螺母 m=7.9
        let spec = JointSpec::parse(
            "at 50,10 rot -90 bolt=hex_bolt_b_full:8 plate=10 plate=10 nut=nut_c41:8",
        )
        .unwrap();
        let plan = plan(&spec).unwrap();
        assert!((plan.stack - 27.9).abs() < 1e-9, "Σ={}", plan.stack);
        assert!((plan.need - (27.9 + 2.5 * 1.25)).abs() < 1e-9, "need={}", plan.need);
        assert!((plan.bolt_l - 35.0).abs() < 1e-9, "应落 35（5783 M8 系列 30 不够 31.03）");
        // 放置：螺栓基点 = 头部支承面；螺母基点 = Σ 处（板厚累计 20）
        assert_eq!(plan.placements.len(), 2);
        assert_eq!(plan.placements[0].at, [50.0, 10.0, 0.0]);
        assert_eq!(plan.placements[1].at, [50.0, -10.0, 0.0]);
        assert!((plan.placements[1].rot_rad + std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert!(plan.report.contains("5783"), "{}", plan.report);
    }

    #[test]
    fn plan_handles_washers_and_reports_protrusion() {
        // 加两侧平垫 + 弹垫：Σ = 10+10+1.6+2.1+7.9 = 31.6 → need 34.7 → 35
        let spec = JointSpec::parse(
            "at 0,0 rot 0 bolt=hex_bolt_b_full:8 plate=10 plate=10 washer=washer_971:8 washer=washer_93:8 nut=nut_c41:8",
        )
        .unwrap();
        let built = plan(&spec).unwrap();
        assert!((built.stack - 31.6).abs() < 1e-9, "Σ={}", built.stack);
        assert!((built.bolt_l - 35.0).abs() < 1e-9, "l={}", built.bolt_l);
        // 件链顺序（rot 0 → 沿 +x）：螺栓@0 → 板10 → 板10 → 平垫@20 → 弹垫@21.6 → 螺母@23.7
        let offsets: Vec<f64> = built.placements.iter().map(|p| p.at[0]).collect();
        let expected = [0.0, 20.0, 21.6, 23.7];
        assert_eq!(offsets.len(), expected.len());
        for (got, want) in offsets.iter().zip(expected.iter()) {
            assert!((got - want).abs() < 1e-9, "{offsets:?} vs {expected:?}");
        }
        // 全族都用主视图：垫圈的主视图已改为侧视轮廓（2026-09-15 互换后）
        let views: Vec<&str> = built.placements.iter().map(|p| p.view.as_str()).collect();
        assert_eq!(views, vec!["main", "main", "main", "main"], "{views:?}");
        // 显式 view 覆盖全部件
        let forced = JointSpec::parse("at 0,0 view top bolt=hex_bolt_b_full:8 plate=10 nut=nut_c41:8").unwrap();
        assert!(plan(&forced).unwrap().placements.iter().all(|p| p.view == "top"));
    }

    #[test]
    fn plan_rejects_impossible_bolt_family() {
        let _g = zh_guard();
        // 5780 C 级 M8 的库内系列最短 35；件链超过它就必须报错而不是硬塞
        let spec = JointSpec::parse("at 0,0 bolt=hex_bolt_c:8 plate=40 plate=40 nut=nut_c41:8").unwrap();
        let err = plan(&spec).unwrap_err();
        assert!(err.contains("装不下") || err.contains("需"), "{err}");
    }

    #[test]
    fn joint_messages_switch_language_keeping_data() {
        let _g = zh_guard();
        crate::i18n::clear_missing_keys();

        // ── zh：≥5 条面向用户的输出，均需含关键协议/数据 ──
        let err = JointSpec::parse("bolt=hex_bolt_c:8 plate=10").unwrap_err();
        assert!(err.contains("缺 at x,y") && err.contains("OCSMJOINT"), "{err}");
        let err = JointSpec::parse("at 0,0 screw=hex_bolt_c:8 plate=10").unwrap_err();
        assert!(err.contains("螺钉") && err.contains("件链"), "{err}");
        let err = JointSpec::parse("at 0,0 bolt=hex_bolt_c:8:40:50").unwrap_err();
        assert!(err.contains("规格最多") && err.contains("8:40:50"), "{err}");
        let err = item_for("nut_c41".into(), 99.0, None).unwrap_err();
        assert!(err.contains("M99") && err.contains("nut_c41"), "{err}");
        let err = JointSpec::parse("at 0,0 bolt=nut_c41:8 plate=10").unwrap_err();
        assert!(err.contains("不是bolt族"), "{err}");
        let spec = JointSpec::parse(
            "at 50,10 rot -90 bolt=hex_bolt_b_full:8 plate=10 plate=10 nut=nut_c41:8",
        )
        .unwrap();
        let built = plan(&spec).unwrap();
        assert!(built.report.contains("OCSMJOINT") && built.report.contains("M8") && built.report.contains("35"), "{}", built.report);
        let bolt = JointItem::Bolt { family: "hex_bolt_b_full".into(), d: 8.0, l: Some(35.0) };
        assert!(bolt.describe().contains("M8×35"), "{}", bolt.describe());
        let plate = JointItem::Plate { t: 10.0 };
        assert!(plate.describe().contains("10"), "{}", plate.describe());
        let zh_report = built.report.clone();

        // ── en：同一批调用，关键数据（M8/35/hex_bolt_c…）必须原样保留 ──
        crate::i18n::set_lang(crate::i18n::Lang::En);
        let err = JointSpec::parse("bolt=hex_bolt_c:8 plate=10").unwrap_err();
        assert!(err.contains("Missing at x,y") && err.contains("OCSMJOINT"), "{err}");
        assert!(!err.contains("缺"), "{err}");
        let err = JointSpec::parse("at 0,0 screw=hex_bolt_c:8 plate=10").unwrap_err();
        assert!(err.contains("screw") && err.contains("hex-head bolt"), "{err}");
        let err = JointSpec::parse("at 0,0 bolt=hex_bolt_c:8:40:50").unwrap_err();
        assert!(err.contains("Size") && err.contains("8:40:50"), "{err}");
        let err = item_for("nut_c41".into(), 99.0, None).unwrap_err();
        assert!(err.contains("M99") && err.contains("nut_c41") && err.contains("Available"), "{err}");
        let err = JointSpec::parse("at 0,0 bolt=nut_c41:8 plate=10").unwrap_err();
        assert!(err.contains("not a bolt family"), "{err}");
        let spec = JointSpec::parse(
            "at 50,10 rot -90 bolt=hex_bolt_b_full:8 plate=10 plate=10 nut=nut_c41:8",
        )
        .unwrap();
        let built = plan(&spec).unwrap();
        assert!(built.report.contains("OCSMJOINT") && built.report.contains("M8") && built.report.contains("35"), "{}", built.report);
        assert_ne!(built.report, zh_report, "报告应随语言变");
        let bolt = JointItem::Bolt { family: "hex_bolt_b_full".into(), d: 8.0, l: Some(35.0) };
        assert!(bolt.describe().contains("M8×35"), "{}", bolt.describe());
        let plate = JointItem::Plate { t: 10.0 };
        assert!(plate.describe().contains("10") && plate.describe().contains("plate"), "{}", plate.describe());

        assert!(crate::i18n::missing_keys().is_empty(), "缺词条：{:?}", crate::i18n::missing_keys());
        crate::i18n::set_lang_auto();
    }
}
