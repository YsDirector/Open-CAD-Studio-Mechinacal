//! **孔生成器**（`OCSMHOLE` / 短命令 `DK`）——简单孔 / 螺纹孔 / 沉头孔 / 埋头孔
//! （盲孔 / 贯通；沉头/埋头可选带螺纹或不带螺纹）。
//!
//! GUI 参考：`~/桌面/GB/螺纹生成器GUI.png`。数据来源（逐表落在 `tables/hole*.json` 的
//! `source`/`note` 字段）：
//! - 螺纹基本尺寸（粗牙/细牙）：ISO 724 参考站 `mecalculator.tw`（见 `threadIso724.json`）；
//! - 沉头孔 GB/T 152.3-1988 / 埋头孔 GB/T 152.2-1988 / 螺栓间隙 GB/T 5277-1985：
//!   嘉立创 FA 手册 `jlc-jdgf.com/mcbook/5-1-43|5-1-44|new05053.htm`；
//! - 钻头（底孔径）：`~/桌面/GB/公制螺纹底孔牙深明细表.png`。
//!
//! ## 规则（用户定案）
//! - **两个量分开**（用户修正的概念模型）：
//!   **① 螺纹有效长度**（= 啮合长度，内、外螺纹**同一个量**——“要旋进多深”）；
//!   **② 孔深 / 实际加工长度** = 有效长度 + **工艺余量**（刀具退出 + 不完整牙）。
//! - **自动有效长度**：M（iso724）仍 = `1.5 × 公称直径`（不变）；UN/ACME/Tr 暂同 1.5d；
//!   **管螺纹 G/R/NPT = 表内 `eff_len`**（用户裁定）：R = ISO 7-1 表第 16 栏（无退刀槽 = 最大基准距离+装配余量）、
//!   NPT = 基准距离基本 + 装配余量 + 基准平面位置偏差（偏差取 **+1P 最大档**，依据同族 NPTF 表 LW3-55 ±1P）、
//!   G = 同规格 R 的 `eff_len`（ISO 228-1 无此口径，**待确认**）；表内无值明确报错 —— 可手填「有效长度」覆盖。
//! - **自动孔深** = 有效长度 + `2 × 螺距`（既有规则；M10×1.5、有效 15 → 18）。
//!   锥管 R/NPT 的 2P 取锥管工艺口径“额外 1~2 牙”的**上限**，兼顾客锥不完整牙区。
//! - 非公制系列（UN/G/R/NPT/ACME/Tr）时：`d` = 表内**螺纹大径（mm）**、
//!   `P` = 表内**螺距（mm）**；英制体系 `P = 25.4/TPI`（表格 `tpi` 字段）。
//!   管螺纹（G/R/NPT）的大径是螺纹/管子外径（NPT 为内螺纹基本大径），**不是通径**。
//! - **不带螺纹时两个「自动」都不可选**（简单孔、以及沉头/埋头的光孔版本）；
//!   贯通孔深不能自动（板厚/通孔长度必须手填）；
//! - **孔范围**：盲孔 = 底孔带 **118° 底锥**（半角 59°）；贯通 = 无底锥；
//! - 螺纹大径 `D1 = d`、小径 `D = d − 1.0825P`（GB/T 197 基本牙型；M10×1.5 → 8.376）；
//! - 沉头/埋头不带螺纹时，`子类型` = 钻头大小 / 自定义 / 螺栓间隙；只有
//!   `螺栓间隙` 下 `配合`（精装配/中等装配/粗装配）才可选；
//! - 表外规格**明确报错**，不插值、不外推。
//!
//! ## 画法（GB/T 4459.1 简化画法）
//! - 侧视（剖）：底孔壁（小径/底孔径）`1轮廓实线层`；螺纹大径细实线 `2细线层`；
//!   螺纹终止线 `1轮廓实线层`；沉孔壁/底面、埋头 90° 锥线同 `1轮廓实线层`；
//!   轴线 `3中心线层`（两端各伸 3 mm）。
//! - 俯视（端视）：底孔小径整圆粗实线 + 螺纹大径细实线 3/4 圈（库内共有口径，
//!   与轴生成器导向平键槽共用 `partgen_kit::thread_major_arc`：模板换算 265°→185°）；
//!   沉孔/埋头再加深/浅一圈实线圆。
//! - **不生成尺寸标注**（`D1`/`D` 只在 GUI 预览示意图里画）。
//!
//! 输出只是孔本身（**不画材料板轮廓**）：基点 = 孔口中心 × 材料表面，孔向局部 −Y；
//! 沉头/埋头时 `孔深/螺纹有效长度` 从**沉孔底 / 埋头锥底**起算；俯视图（若同时勾选）
//! 放在侧视图右侧、间隙 20 mm。材料板 + 绿色剖面线只出现在 GUI 预览里。

use crate::i18n::{t, t_fmt};
use crate::thread::{self, ThreadSystem};
use ocs_plugin_api::host::acadrust;
use ocs_plugin_api::host::acadrust::entities::{Arc, Circle, EntityType, Line};
use ocs_plugin_api::host::acadrust::types::{Color, LineWeight, Vector3};

/// 轮廓（粗实线）层。
pub const LAYER_MAIN: &str = "1轮廓实线层";
/// 细实线层（螺纹大径、3/4 圈）。
pub const LAYER_THIN: &str = "2细线层";
/// 中心线层。
pub const LAYER_CENTER: &str = "3中心线层";

/// 118° 钻尖：半角 59°。
const CONE_HALF_ANGLE_DEG: f64 = 59.0;
/// 自动螺纹长度系数（× 公称直径）。
pub const AUTO_THREAD_FACTOR: f64 = 1.5;
/// 自动孔深 = 有效长度 + 系数 × 螺距（工艺余量：刀具退出；锥管 R/NPT 再兼顾锥度不完整牙，
/// 取锥管工艺口径“额外 1~2 牙”的上限：即 2P）。
pub const AUTO_RUNOUT_FACTOR: f64 = 2.0;
/// 中心线两端外伸。
const AXIS_OVER: f64 = 3.0;
/// 侧视图与俯视图之间的间隙（mm）。
const VIEW_GAP: f64 = 20.0;

// ── 数据表 ──────────────────────────────────────────────────────────────

/// 一行公制螺纹（ISO 724；`d1` 为参考表值，几何一律用 `d − 1.0825P`）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ThreadRow {
    pub name: String,
    pub d: f64,
    pub p: f64,
    #[allow(dead_code)]
    pub d2: f64,
    pub d1: f64,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ThreadTable {
    #[allow(dead_code)]
    source: String,
    #[allow(dead_code)]
    note: String,
    coarse: Vec<ThreadRow>,
    fine: Vec<ThreadRow>,
}

/// 沉头孔一行（GB/T 152.3-1988；`table` = `gb70`（表1，GB 70）/ `gb6190`（表2，GB 6190、6191、65））。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CounterboreRow {
    pub table: String,
    pub d: f64,
    pub d2: f64,
    pub t: f64,
    #[allow(dead_code)]
    pub d3: Option<f64>,
    #[allow(dead_code)]
    pub d1: f64,
}

/// 埋头孔一行（GB/T 152.2-2014 沉头螺钉用沉孔）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CountersinkRow {
    pub d: f64,
    /// 沉孔直径（取标准 `dc_max`，与 1988 版 `d2` 交叉校验一致）。
    pub d2: f64,
    /// 通孔（标准 `dh_min` = 公称，H13）。
    pub d1: f64,
    /// 名义深度 t≈（几何按 90° 锥：(d2−底孔)/2）。
    pub t: f64,
    #[allow(dead_code)]
    #[serde(default)]
    pub dc_min: f64,
    #[allow(dead_code)]
    #[serde(default)]
    pub dh_max: f64,
}

/// 螺栓间隙通孔一行（GB/T 5277-1985）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ClearanceRow {
    pub d: f64,
    pub close: f64,
    pub normal: f64,
    pub loose: f64,
}

/// 底孔径（钻头大小）一行。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TapRow {
    pub d: f64,
    pub p: f64,
    pub drill: f64,
    #[allow(dead_code)]
    pub series: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct JsonTable<T> {
    #[allow(dead_code)]
    code: String,
    #[allow(dead_code)]
    source: String,
    #[allow(dead_code)]
    note: String,
    rows: Vec<T>,
}

fn thread_table() -> &'static ThreadTable {
    static T: std::sync::OnceLock<ThreadTable> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("tables/threadIso724.json"))
            .expect("tables/threadIso724.json 解析失败")
    })
}

fn counterbore_table() -> &'static JsonTable<CounterboreRow> {
    static T: std::sync::OnceLock<JsonTable<CounterboreRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("tables/holeCounterbore.json"))
            .expect("tables/holeCounterbore.json 解析失败")
    })
}

fn countersink_table() -> &'static JsonTable<CountersinkRow> {
    static T: std::sync::OnceLock<JsonTable<CountersinkRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("tables/holeCountersink.json"))
            .expect("tables/holeCountersink.json 解析失败")
    })
}

fn clearance_table() -> &'static JsonTable<ClearanceRow> {
    static T: std::sync::OnceLock<JsonTable<ClearanceRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("tables/holeClearance.json"))
            .expect("tables/holeClearance.json 解析失败")
    })
}

fn tap_table() -> &'static JsonTable<TapRow> {
    static T: std::sync::OnceLock<JsonTable<TapRow>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("tables/holeTapDrill.json"))
            .expect("tables/holeTapDrill.json 解析失败")
    })
}

/// 标准麻花钻直径系列（GB/T 6135.3-1996 直柄麻花钻）——「钻头大小」子类型。
#[derive(Debug, Clone, serde::Deserialize)]
struct DrillTable {
    #[allow(dead_code)]
    code: String,
    #[allow(dead_code)]
    source: String,
    #[allow(dead_code)]
    note: String,
    diameters: Vec<f64>,
}

fn drill_table() -> &'static DrillTable {
    static T: std::sync::OnceLock<DrillTable> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!("tables/holeDrill.json"))
            .expect("tables/holeDrill.json 解析失败")
    })
}

/// 查螺纹行：给螺距 = 粗牙/细牙里找精确 `(d, P)`；不给 = 查粗牙。
pub fn thread_row(d: f64, pitch: Option<f64>) -> Result<&'static ThreadRow, String> {
    let t = thread_table();
    match pitch {
        Some(p) => t
            .coarse
            .iter()
            .chain(t.fine.iter())
            .find(|r| (r.d - d).abs() < 1e-9 && (r.p - p).abs() < 1e-9)
            .ok_or_else(|| {
                t_fmt(
                    "cmd.hole.err.thread_pair_missing",
                    &[("d", &fmt(d)), ("p", &fmt(p))],
                )
            }),
        None => t
            .coarse
            .iter()
            .find(|r| (r.d - d).abs() < 1e-9)
            .ok_or_else(|| {
                t_fmt(
                    "cmd.hole.err.thread_coarse_missing",
                    &[
                        ("d", &fmt(d)),
                        ("d_lo", &fmt(t.coarse.first().map(|r| r.d).unwrap_or(0.0))),
                        ("d_hi", &fmt(t.coarse.last().map(|r| r.d).unwrap_or(0.0))),
                    ],
                )
            }),
    }
}

/// 是否细牙（P < 粗牙 P）。
pub fn is_fine(row: &ThreadRow) -> bool {
    thread_table()
        .coarse
        .iter()
        .find(|r| (r.d - row.d).abs() < 1e-9)
        .map(|c| row.p < c.p - 1e-9)
        .unwrap_or(true)
}

fn counterbore_row(d: f64, table: &str) -> Result<&'static CounterboreRow, String> {
    let t = counterbore_table();
    if table == "gb70_2" {
        return Err(crate::i18n::t("cmd.hole.err.counterbore_gb70_2"));
    }
    let key = match table {
        "gb70_1" | "gb70" => "gb70",
        "gb6190" => "gb6190",
        other => return Err(t_fmt("cmd.hole.err.reco_unknown", &[("other", other)])),
    };
    t.rows
        .iter()
        .find(|r| r.table == key && (r.d - d).abs() < 1e-9)
        .ok_or_else(|| {
            let list: Vec<String> = t
                .rows
                .iter()
                .filter(|r| r.table == key)
                .map(|r| fmt(r.d))
                .collect();
            t_fmt(
                "cmd.hole.err.counterbore_row_missing",
                &[
                    ("table", key),
                    ("d", &fmt(d)),
                    ("list", &list.join("/")),
                ],
            )
        })
}

fn countersink_row(d: f64) -> Result<&'static CountersinkRow, String> {
    let t = countersink_table();
    t.rows
        .iter()
        .find(|r| (r.d - d).abs() < 1e-9)
        .ok_or_else(|| {
            t_fmt(
                "cmd.hole.err.countersink_row_missing",
                &[
                    ("d", &fmt(d)),
                    ("d_lo", &fmt(t.rows.first().map(|r| r.d).unwrap_or(0.0))),
                    ("d_hi", &fmt(t.rows.last().map(|r| r.d).unwrap_or(0.0))),
                ],
            )
        })
}

fn clearance_row(d: f64) -> Result<&'static ClearanceRow, String> {
    let t = clearance_table();
    t.rows
        .iter()
        .find(|r| (r.d - d).abs() < 1e-9)
        .ok_or_else(|| {
            t_fmt(
                "cmd.hole.err.clearance_row_missing",
                &[
                    ("d", &fmt(d)),
                    ("d_lo", &fmt(t.rows.first().map(|r| r.d).unwrap_or(0.0))),
                    ("d_hi", &fmt(t.rows.last().map(|r| r.d).unwrap_or(0.0))),
                ],
            )
        })
}

fn tap_row(d: f64, pitch: Option<f64>) -> Result<&'static TapRow, String> {
    let t = tap_table();
    let hit = match pitch {
        Some(p) => t
            .rows
            .iter()
            .find(|r| (r.d - d).abs() < 1e-9 && (r.p - p).abs() < 1e-9),
        None => t.rows.iter().find(|r| (r.d - d).abs() < 1e-9),
    };
    hit.ok_or_else(|| {
        let p = pitch.map(|p| format!("×{}", fmt(p))).unwrap_or_default();
        t_fmt(
            "cmd.hole.err.tap_row_missing",
            &[("d", &fmt(d)), ("p", &p)],
        )
    })
}

/// GB/T 5277 三档配合的用户可见名。
pub fn clearance_fit_label(fit: &str) -> Result<&'static str, String> {
    Ok(match fit {
        "close" => "精装配",
        "normal" => "中等装配",
        "loose" => "粗装配",
        other => return Err(t_fmt("cmd.hole.err.clearance_fit_unknown", &[("other", other)])),
    })
}

/// 螺纹公差带（内螺纹，本期只记录，不改变基本牙型几何）。
pub fn thread_fit_label(fit: &str) -> Result<&'static str, String> {
    Ok(match fit {
        "6H" => "6H（内螺纹）",
        "6G" => "6G（内螺纹）",
        other => return Err(t_fmt("cmd.hole.err.thread_fit_unknown", &[("other", other)])),
    })
}

/// GUI `/api/hole_sizes`：全部选择器数据（螺纹/钻头/沉头/埋头/间隙）。
pub fn sizes_json() -> String {
    let t = thread_table();
    let thread_row_json = |r: &ThreadRow| {
        serde_json::json!({
            "name": r.name, "d": r.d, "p": r.p, "d1": r.d1, "d2": r.d2, "fine": is_fine(r),
        })
    };
    serde_json::json!({
        "ok": true,
        "thread_source": t.source,
        "coarse": t.coarse.iter().map(thread_row_json).collect::<Vec<_>>(),
        "fine": t.fine.iter().map(thread_row_json).collect::<Vec<_>>(),
        "tap": tap_table().rows.iter().map(|r| serde_json::json!({
            "name": format!("M{}×{}", fmt(r.d), fmt(r.p)), "d": r.d, "p": r.p, "drill": r.drill,
        })).collect::<Vec<_>>(),
        "drill": {
            "code": drill_table().code,
            "source": drill_table().source,
            "diameters": drill_table().diameters,
        },
        "counterbore": {
            "code": counterbore_table().code,
            "source": counterbore_table().source,
            "note": counterbore_table().note,
            "rows": counterbore_table().rows.iter().map(|r| serde_json::json!({
                "table": r.table, "name": format!("M{}", fmt(r.d)), "d": r.d,
                "d2": r.d2, "t": r.t, "d1": r.d1,
            })).collect::<Vec<_>>(),
        },
        "countersink": {
            "code": countersink_table().code,
            "source": countersink_table().source,
            "note": countersink_table().note,
            "rows": countersink_table().rows.iter().map(|r| serde_json::json!({
                "name": format!("M{}", fmt(r.d)), "d": r.d, "d2": r.d2, "t": r.t, "d1": r.d1,
            })).collect::<Vec<_>>(),
        },
        "clearance": {
            "code": clearance_table().code,
            "source": clearance_table().source,
            "rows": clearance_table().rows.iter().map(|r| serde_json::json!({
                "name": format!("M{}", fmt(r.d)), "d": r.d,
                "close": r.close, "normal": r.normal, "loose": r.loose,
            })).collect::<Vec<_>>(),
        },
        // 螺纹体系（GUI「标准」下拉；M 的 drill 用既有底孔牙深表，其余用行内钻径/小径兜底）
        "systems": thread_systems_json(),
    })
    .to_string()
}

/// 全部螺纹体系（含 M）的 GUI 数据：与 `resolve()` 同口径。
fn thread_systems_json() -> Vec<serde_json::Value> {
    thread::ThreadSystem::ALL
        .iter()
        .map(|&sys| {
            let t = thread::table(sys);
            let groups: Vec<serde_json::Value> = t
                .groups
                .iter()
                .map(|g| {
                    serde_json::json!({
                        "key": g.key,
                        "label": g.label,
                        "rows": g.rows.iter().map(|r| {
                            // 公制 M：底孔优先用底孔牙深表（与 resolve 的兜底一致）。
                            let drill = if sys == thread::ThreadSystem::Iso724 {
                                tap_row(r.d, Some(r.p)).map(|x| x.drill).unwrap_or(r.d1)
                            } else {
                                r.drill_mm()
                            };
                            serde_json::json!({
                                "name": r.name, "d": r.d, "p": r.p, "tpi": r.tpi,
                                "d2": r.d2, "d1": r.d1, "drill": drill, "gauge_len": r.gauge_len,
                                "makeup": r.makeup, "eff_ext": r.eff_ext, "eff_len": r.eff_len,
                            })
                        }).collect::<Vec<_>>(),
                    })
                })
                .collect();
            serde_json::json!({
                "key": sys.key(),
                "code": sys.code(),
                "label": sys.label(),
                "standard": sys.standard(),
                "angle_deg": sys.angle_deg(),
                "is_pipe": sys.is_pipe(),
                "source": t.source,
                "note": t.note,
                "units": if t.units.is_empty() { "mm" } else { t.units.as_str() },
                "groups": groups,
            })
        })
        .collect()
}

// ── 模型 ────────────────────────────────────────────────────────────────

/// 孔类型（本期四类全做）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HoleKind {
    /// 简单孔：一个光孔（无螺纹）。
    Simple,
    /// 螺纹孔：底孔 + 螺纹。
    #[default]
    Threaded,
    /// 沉头孔：圆柱沉孔（GB/T 152.3）+ 底孔，底孔可带螺纹。
    Counterbore,
    /// 埋头孔：90° 沉锥（GB/T 152.2）+ 底孔，底孔可带螺纹。
    Countersink,
}

/// 子类型：螺纹孔/带螺纹的沉头埋头用「标准螺纹/细牙螺纹」；
/// 不带螺纹时用「钻头大小/自定义/螺栓间隙」。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HoleSubtype {
    #[default]
    Standard,
    Fine,
    Drill,
    Custom,
    Clearance,
}

/// 孔范围。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HoleRange {
    /// 盲孔：底孔带 118° 底锥。
    #[default]
    Blind,
    /// 贯通：无底锥。
    Through,
}

/// 视图开关（侧视 = 剖，俯视 = 端视）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HoleViews {
    pub side: bool,
    pub top: bool,
}

impl Default for HoleViews {
    fn default() -> Self {
        Self {
            side: true,
            top: false,
        }
    }
}

/// 孔生成器模型（GUI JSON / CLI DSL 共用；`hole_depth`/`thread_len` = `None` 表示自动）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct HoleModel {
    pub kind: HoleKind,
    pub subtype: HoleSubtype,
    /// 螺纹体系（GUI「标准」下拉）：M/UN/G/R/NPT/ACME/Tr。
    /// 默认 `Iso724` = 既有公制 M 行为（老请求不带此字段也走原路径）。
    pub thread_system: ThreadSystem,
    /// 螺纹子类型组（GUI「子类型」下拉）：如 `unc`/`unf`/`general`/`stub`/`standard`。
    /// 仅非公制体系使用；公制 M 仍用 `subtype`（Standard=粗牙 / Fine=细牙）。
    pub thread_group: Option<String>,
    /// 公称直径（螺纹大径 / 螺栓公称 / 底孔表规格）。
    pub d: f64,
    /// 螺距；`None` = 粗牙（查表）。
    pub pitch: Option<f64>,
    /// 配合：带螺纹 = 6H/6G；螺栓间隙 = close/normal/loose；其余忽略。
    pub fit: String,
    /// 自定义底孔径（子类型 = 自定义时必填）。
    pub custom_d: Option<f64>,
    /// 钻头直径（子类型 = 钻头大小；标准麻花钻系列）。
    pub drill_d: Option<f64>,
    /// 沉头孔推荐值：`gb70_1`（内六角圆柱头 GB/T 70.1，默认）/ `gb70_2`（内六角平圆头，GB/T 152.3 无表 → 报错）/ `gb6190`（GB 6190/6191/65）。
    pub reco: String,
    pub range: HoleRange,
    /// 孔深（从沉孔底/埋头锥底起算；无沉/埋时即孔口）；`None` = 自动。
    pub hole_depth: Option<f64>,
    /// 螺纹有效长度（= 啮合长度）；`None` = 自动（M/UN/ACME/Tr = 1.5d；管螺纹 = 表内 `eff_len`）。
    pub thread_len: Option<f64>,
    /// 全长螺纹（L = H；此时孔深必须手填）。
    pub full_thread: bool,
    pub views: HoleViews,
    /// 显式落点（CLI/AI 一行直插用）。
    pub at: Option<[f64; 2]>,
    /// 放置转角（度）。
    pub rot: f64,
}

impl Default for HoleModel {
    fn default() -> Self {
        Self {
            kind: HoleKind::Threaded,
            subtype: HoleSubtype::Standard,
            thread_system: ThreadSystem::Iso724,
            thread_group: None,
            d: 10.0,
            pitch: None,
            fit: "6H".to_string(),
            custom_d: None,
            drill_d: None,
            reco: "gb70_1".to_string(),
            range: HoleRange::Blind,
            hole_depth: None,
            thread_len: None,
            full_thread: false,
            views: HoleViews::default(),
            at: None,
            rot: 0.0,
        }
    }
}

impl HoleModel {
    /// 是否带螺纹（简单孔恒不带；螺纹孔恒带；沉头/埋头看子类型）。
    pub fn is_threaded(&self) -> bool {
        match self.kind {
            HoleKind::Simple => false,
            HoleKind::Threaded => true,
            HoleKind::Counterbore | HoleKind::Countersink => {
                matches!(self.subtype, HoleSubtype::Standard | HoleSubtype::Fine)
            }
        }
    }
}

/// 派生的显示/几何量。
#[derive(Debug, Clone, PartialEq)]
pub struct HoleValues {
    pub size_name: String,
    pub threaded: bool,
    /// 螺纹体系 key（如 `iso724`/`un`/`g`；不带螺纹 = 空串）。
    pub system: String,
    /// 螺纹体系显示名（如「美制统一 UN…」）。
    pub system_label: String,
    /// 大径 D1（螺纹 = d；不带螺纹 = 底孔径）。
    pub major: f64,
    /// 小径 D（螺纹 = d − 1.0825P；不带螺纹 = 底孔径）。
    pub minor: f64,
    pub pitch: f64,
    /// 底孔（被钻出的孔）直径。
    pub base_d: f64,
    /// 底孔起始深度：沉头 = 沉孔深 t；埋头 = 90° 锥几何深；其余 0。
    pub base_start: f64,
    /// 沉头孔：沉孔直径/深度。
    pub bore_d: Option<f64>,
    pub bore_t: Option<f64>,
    /// 埋头孔：90° 锥名义直径/名义深度 t≈（几何按 90° 推）。
    pub sink_d: Option<f64>,
    pub sink_t: Option<f64>,
    /// 有效螺纹长度（不带螺纹 = 0）。
    pub thread_len: f64,
    /// 孔深 H（从 base_start 起算）。
    pub hole_depth: f64,
    /// 118° 底锥高（贯通 = 0）。
    pub cone_height: f64,
    /// 英制 TPI（每英寸牙数；公制 = None）。
    pub tpi: Option<f64>,
    /// 管螺纹基准长度 / 基准距离 L1（mm；非管螺纹 = None）。
    pub gauge_len: Option<f64>,
    /// 管螺纹自动「螺纹有效长度」（mm；内、外螺纹同一个量；M/UN/ACME/Tr = None）。
    pub eff_len: Option<f64>,
    pub warnings: Vec<String>,
}

impl HoleValues {
    /// 给 GUI 的显示文本（3 位小数去尾零）。
    pub fn display_json(&self) -> serde_json::Value {
        let opt = |v: Option<f64>| v.map(fmt3);
        serde_json::json!({
            "size_name": self.size_name,
            "threaded": self.threaded,
            "system": self.system,
            "system_label": self.system_label,
            "major": fmt3(self.major),
            "minor": fmt3(self.minor),
            "pitch": fmt3(self.pitch),
            "tpi": opt(self.tpi),
            "gauge_len": opt(self.gauge_len),
            "eff_len": opt(self.eff_len),
            "base_d": fmt3(self.base_d),
            "base_start": fmt3(self.base_start),
            "bore_d": opt(self.bore_d),
            "bore_t": opt(self.bore_t),
            "sink_d": opt(self.sink_d),
            "sink_t": opt(self.sink_t),
            "thread_len": fmt3(self.thread_len),
            "hole_depth": fmt3(self.hole_depth),
            "cone_height": fmt3(self.cone_height),
            "warnings": self.warnings,
        })
    }
}

/// 求派生量（含规则/表外校验）。
pub fn resolve(model: &HoleModel) -> Result<HoleValues, String> {
    if !(model.d.is_finite() && model.d > 0.0) {
        return Err(t_fmt("cmd.hole.err.d_invalid", &[("d", &model.d.to_string())]));
    }
    // 子类型与孔类型一致
    match model.kind {
        HoleKind::Simple => {
            if matches!(model.subtype, HoleSubtype::Standard | HoleSubtype::Fine) {
                return Err(t("cmd.hole.err.simple_has_thread_subtype"));
            }
        }
        HoleKind::Threaded => {
            if !matches!(model.subtype, HoleSubtype::Standard | HoleSubtype::Fine) {
                return Err(t("cmd.hole.err.threaded_needs_thread_subtype"));
            }
        }
        HoleKind::Counterbore | HoleKind::Countersink => {}
    }

    let threaded = model.is_threaded();
    let mut warnings: Vec<String> = Vec::new();

    // ① 底孔径 + 螺纹基本尺寸
    // 非公制查到的规格（供体系显示 / 管螺纹自动长 L1 复用）。
    let mut spec_ref: Option<&'static crate::thread::ThreadSpec> = None;
    let (size_name, major, minor, pitch, base_d) = if threaded {
        if model.thread_system != ThreadSystem::Iso724 {
            // 非公制体系：逐表查规格（表外/多义明确报错，不插值）。
            if matches!(model.kind, HoleKind::Counterbore | HoleKind::Countersink) {
                return Err(t("cmd.hole.err.cbore_csink_metric_only"));
            }
            if model.subtype == HoleSubtype::Fine {
                return Err(t("cmd.hole.err.nonmetric_no_metric_fine"));
            }
            // d 取该行螺纹大径（mm）；P 取表内螺距 mm（英制 p=25.4/TPI）。
            // 自动螺纹长 1.5d、孔深 L+2P 均用这两个值（管螺纹 d=大径/外径，非通径）。
            let spec = thread::lookup(
                model.thread_system,
                model.thread_group.as_deref(),
                model.d,
                model.pitch,
            )?;
            spec_ref = Some(spec);
            (
                spec.name.clone(),
                spec.d,
                spec.d1,
                spec.p,
                spec.drill_mm(),
            )
        } else {
        let row = match model.subtype {
            HoleSubtype::Fine => {
                let p = model
                    .pitch
                    .ok_or_else(|| t("cmd.hole.err.fine_needs_concrete_pitch"))?;
                let row = thread_row(model.d, Some(p))?;
                if !is_fine(row) {
                    return Err(t_fmt(
                        "cmd.hole.err.fine_subtype_but_coarse",
                        &[("d", &fmt(model.d)), ("p", &fmt(p))],
                    ));
                }
                row
            }
            _ => {
                let row = thread_row(model.d, model.pitch)?;
                if model.pitch.is_some() && is_fine(row) {
                    return Err(t_fmt(
                        "cmd.hole.err.standard_subtype_but_fine",
                        &[("d", &fmt(model.d)), ("p", &fmt(row.p))],
                    ));
                }
                row
            }
        };
        let p = row.p;
        let minor = model.d - 1.0825 * p;
        if minor <= 0.0 {
            return Err(t_fmt(
                "cmd.hole.err.minor_nonpositive",
                &[
                    ("d", &fmt(model.d)),
                    ("p", &fmt(p)),
                    ("minor", &fmt3(minor)),
                ],
            ));
        }
        // 螺纹孔的钻孔直径：优先用底孔牙深表的实际钻头（用户口径），
        // 表外回退到 GB/T 197 理论小径 d−1.0825P（标准公式，非插值）。
        let drill = tap_row(model.d, Some(p)).map(|r| r.drill).unwrap_or(minor);
        (row.name.clone(), model.d, minor, p, drill)
        }
    } else {
        match model.subtype {
            HoleSubtype::Drill => {
                let d = model
                    .drill_d
                    .filter(|v| v.is_finite() && *v > 0.0)
                    .ok_or_else(|| t("cmd.hole.err.drill_required"))?;
                if !drill_table()
                    .diameters
                    .iter()
                    .any(|x| (x - d).abs() < 1e-9)
                {
                    return Err(t_fmt(
                        "cmd.hole.err.drill_not_in_series",
                        &[("d", &fmt(d))],
                    ));
                }
                (format!("Ø{}", fmt(d)), d, d, 0.0, d)
            }
            HoleSubtype::Custom => {
                let c = model
                    .custom_d
                    .filter(|v| v.is_finite() && *v > 0.0)
                    .ok_or_else(|| t("cmd.hole.err.custom_d_required"))?;
                (
                    if model.kind == HoleKind::Simple {
                        format!("Ø{}", fmt(c))
                    } else {
                        format!("M{}（底孔 Ø{}）", fmt(model.d), fmt(c))
                    },
                    c,
                    c,
                    0.0,
                    c,
                )
            }
            HoleSubtype::Clearance => {
                let row = clearance_row(model.d)?;
                let label = clearance_fit_label(&model.fit)?;
                let v = match model.fit.as_str() {
                    "close" => row.close,
                    "normal" => row.normal,
                    "loose" => row.loose,
                    other => {
                        return Err(t_fmt(
                            "cmd.hole.err.clearance_fit_unknown",
                            &[("other", other)],
                        ))
                    }
                };
                (format!("M{}·{}", fmt(model.d), label), v, v, 0.0, v)
            }
            HoleSubtype::Standard | HoleSubtype::Fine => {
                return Err(t("cmd.hole.err.unthreaded_with_thread_subtype"))
            }
        }
    };

    // ② 沉头 / 埋头
    // 螺纹体系的显示信息（英制 TPI / 管螺纹基准长度与 L1）；不带螺纹 = 空。
    let (system_key, system_label, tpi, gauge_len, eff_len): (
        String,
        String,
        Option<f64>,
        Option<f64>,
        Option<f64>,
    ) = if threaded {
        if model.thread_system == ThreadSystem::Iso724 {
            (
                ThreadSystem::Iso724.key().to_string(),
                ThreadSystem::Iso724.label().to_string(),
                None,
                None,
                None,
            )
        } else {
            let spec = spec_ref.expect("非公制螺纹已在 ① 查表");
            // 自动「螺纹有效长度」（管螺纹）= 表内 eff_len；其余非公制非管 = None
            let auto_eff = match model.thread_system {
                ThreadSystem::G | ThreadSystem::R | ThreadSystem::Npt => spec.eff_len,
                _ => None,
            };
            (
                model.thread_system.key().to_string(),
                model.thread_system.label().to_string(),
                spec.tpi,
                spec.gauge_len,
                auto_eff,
            )
        }
    } else {
        (String::new(), String::new(), None, None, None)
    };
    let (bore_d, bore_t, sink_d, sink_t) = match model.kind {
        HoleKind::Counterbore => {
            let r = counterbore_row(model.d, &model.reco)?;
            if base_d >= r.d2 - 1e-9 {
                return Err(t_fmt(
                    "cmd.hole.err.cbore_pilot_ge_sink",
                    &[
                        ("base", &fmt3(base_d)),
                        ("sink", &fmt3(r.d2)),
                        ("d", &fmt(model.d)),
                    ],
                ));
            }
            (Some(r.d2), Some(r.t), None, None)
        }
        HoleKind::Countersink => {
            let r = countersink_row(model.d)?;
            if base_d >= r.d2 - 1e-9 {
                return Err(t_fmt(
                    "cmd.hole.err.csink_pilot_ge_sink",
                    &[
                        ("base", &fmt3(base_d)),
                        ("sink", &fmt3(r.d2)),
                        ("d", &fmt(model.d)),
                    ],
                ));
            }
            (None, None, Some(r.d2), Some(r.t))
        }
        _ => (None, None, None, None),
    };
    let base_start = match model.kind {
        HoleKind::Counterbore => bore_t.unwrap_or(0.0),
        HoleKind::Countersink => (sink_d.unwrap_or(0.0) - base_d) / 2.0,
        _ => 0.0,
    };

    // ③ 孔深 / 螺纹有效长度（两个量：自动规则）
    if threaded {
        let thread_len = if model.full_thread {
            match model.hole_depth {
                Some(h) => h,
                None => {
                    return Err(t("cmd.hole.err.full_thread_depth_manual"))
                }
            }
        } else if let Some(v) = model.thread_len {
            // 手填（或「全长」已单独处理）优先，不被自动规则覆盖。
            v
        } else {
            match model.thread_system {
                // M 保持 1.5×公称直径；UN/ACME/Tr 用户未裁定口径，维持现行 1.5d。
                ThreadSystem::Iso724
                | ThreadSystem::Un
                | ThreadSystem::Acme
                | ThreadSystem::Tr => AUTO_THREAD_FACTOR * model.d,
                // 管螺纹：自动「螺纹有效长度」= eff_len（内、外螺纹同一个量）；表内无值明确报错。
                ThreadSystem::Npt => spec_ref
                    .and_then(|s| s.eff_len)
                    .ok_or_else(|| {
                        t_fmt(
                            "cmd.hole.err.npt_no_eff_len",
                            &[("d", &fmt(model.d))],
                        )
                    })?,
                ThreadSystem::R => spec_ref
                    .and_then(|s| s.eff_len)
                    .ok_or_else(|| {
                        t_fmt(
                            "cmd.hole.err.r_no_eff_len",
                            &[("d", &fmt(model.d))],
                        )
                    })?,
                ThreadSystem::G => spec_ref
                    .and_then(|s| s.eff_len)
                    .ok_or_else(|| {
                        t_fmt(
                            "cmd.hole.err.g_no_eff_len",
                            &[("d", &fmt(model.d))],
                        )
                    })?,
            }
        };
        if !(thread_len.is_finite() && thread_len > 0.0) {
            return Err(t_fmt(
                "cmd.hole.err.thread_len_invalid",
                &[("len", &thread_len.to_string())],
            ));
        }
        let hole_depth = match model.hole_depth {
            Some(h) => h,
            None => {
                if model.range == HoleRange::Through {
                    return Err(t("cmd.hole.err.through_depth_manual"));
                }
                thread_len + AUTO_RUNOUT_FACTOR * pitch
            }
        };
        if !(hole_depth.is_finite() && hole_depth > 0.0) {
            return Err(t_fmt(
                "cmd.hole.err.hole_depth_invalid",
                &[("h", &hole_depth.to_string())],
            ));
        }
        if thread_len > hole_depth + 1e-9 {
            return Err(t_fmt(
                "cmd.hole.err.thread_len_gt_hole_depth",
                &[
                    ("l", &fmt3(thread_len)),
                    ("h", &fmt3(hole_depth)),
                ],
            ));
        }
        if !model.full_thread
            && model.range == HoleRange::Blind
            && hole_depth + 1e-9 < thread_len + AUTO_RUNOUT_FACTOR * pitch
        {
            warnings.push(t_fmt(
                "cmd.hole.warn.runout_below_2p",
                &[
                    ("h", &fmt3(hole_depth)),
                    ("l", &fmt3(thread_len)),
                    ("need", &fmt3(thread_len + AUTO_RUNOUT_FACTOR * pitch)),
                ],
            ));
        }
        let cone_height = if model.range == HoleRange::Blind {
            (base_d / 2.0) / CONE_HALF_ANGLE_DEG.to_radians().tan()
        } else {
            0.0
        };
        Ok(HoleValues {
            size_name,
            threaded: true,
            system: system_key,
            system_label,
            major,
            minor,
            pitch,
            base_d,
            base_start,
            bore_d,
            bore_t,
            sink_d,
            sink_t,
            thread_len,
            hole_depth,
            cone_height,
            tpi,
            gauge_len,
            eff_len,
            warnings,
        })
    } else {
        let Some(h) = model.hole_depth else {
            return Err(if model.kind == HoleKind::Simple {
                t("cmd.hole.err.simple_depth_manual")
            } else {
                t("cmd.hole.err.unthreaded_cbore_depth_manual")
            });
        };
        if !(h.is_finite() && h > 0.0) {
            return Err(t_fmt(
                "cmd.hole.err.hole_depth_invalid",
                &[("h", &h.to_string())],
            ));
        }
        let cone_height = if model.range == HoleRange::Blind {
            (base_d / 2.0) / CONE_HALF_ANGLE_DEG.to_radians().tan()
        } else {
            0.0
        };
        Ok(HoleValues {
            size_name,
            threaded: false,
            system: system_key,
            system_label,
            major,
            minor,
            pitch: 0.0,
            base_d,
            base_start,
            bore_d,
            bore_t,
            sink_d,
            sink_t,
            thread_len: 0.0,
            hole_depth: h,
            cone_height,
            tpi,
            gauge_len,
            eff_len,
            warnings,
        })
    }
}

// ── 几何 ────────────────────────────────────────────────────────────────

/// 一次生成的结果。
#[derive(Debug, Clone)]
pub struct BuiltHole {
    pub values: HoleValues,
    pub entities: Vec<EntityType>,
    pub kind: HoleKind,
    pub range: HoleRange,
    pub views: HoleViews,
}

/// 生成孔图元（局部坐标：孔口中心 = 原点、材料表面 = y=0、孔向 −Y）。
pub fn build(model: &HoleModel) -> Result<BuiltHole, String> {
    let v = resolve(model)?;
    let threaded = v.threaded;
    let base_r = v.base_d / 2.0;
    let major_r = v.major / 2.0;
    let (hd, cone, tl, start) = (v.hole_depth, v.cone_height, v.thread_len, v.base_start);
    let mut en: Vec<EntityType> = Vec::new();

    let side = model.views.side;
    let top_cx = if side {
        let r = major_r.max(base_r).max(v.bore_d.unwrap_or(0.0) / 2.0).max(v.sink_d.unwrap_or(0.0) / 2.0);
        r + VIEW_GAP + r
    } else {
        0.0
    };

    if side {
        let bottom = start + hd + cone;
        // 轴线（3中心线层）
        en.push(line(
            [0.0, AXIS_OVER],
            [0.0, -(bottom + AXIS_OVER)],
            LAYER_CENTER,
        ));
        // 沉头孔：圆柱沉孔壁 + 底面（环形，被底孔打断）
        if let (Some(bd), Some(bt)) = (v.bore_d, v.bore_t) {
            for sgn in [1.0, -1.0] {
                en.push(line([sgn * bd / 2.0, 0.0], [sgn * bd / 2.0, -bt], LAYER_MAIN));
                en.push(line([sgn * bd / 2.0, -bt], [sgn * base_r, -bt], LAYER_MAIN));
            }
        }
        // 埋头孔：90° 锥线（到理论上口直径）
        if let Some(sd) = v.sink_d {
            let geo = (sd - v.base_d) / 2.0;
            for sgn in [1.0, -1.0] {
                en.push(line([sgn * sd / 2.0, 0.0], [sgn * base_r, -geo], LAYER_MAIN));
            }
        }
        // 底孔壁（小径/底孔径，粗实线）
        for sgn in [1.0, -1.0] {
            en.push(line(
                [sgn * base_r, -start],
                [sgn * base_r, -(start + hd)],
                LAYER_MAIN,
            ));
        }
        // 盲孔 118° 底锥（贯通无锥）
        if model.range == HoleRange::Blind {
            en.push(line([base_r, -(start + hd)], [0.0, -(start + hd + cone)], LAYER_MAIN));
            en.push(line([-base_r, -(start + hd)], [0.0, -(start + hd + cone)], LAYER_MAIN));
        }
        // 螺纹大径细实线 + 螺纹终止线
        if threaded {
            for sgn in [1.0, -1.0] {
                en.push(line(
                    [sgn * major_r, -start],
                    [sgn * major_r, -(start + tl)],
                    LAYER_THIN,
                ));
            }
            if tl < hd - 1e-9 {
                en.push(line(
                    [-major_r, -(start + tl)],
                    [major_r, -(start + tl)],
                    LAYER_MAIN,
                ));
            }
        }
    }

    if model.views.top {
        // 沉孔 / 埋头外圈
        if let Some(bd) = v.bore_d {
            en.push(circle([top_cx, 0.0], bd / 2.0, LAYER_MAIN));
        }
        if let Some(sd) = v.sink_d {
            en.push(circle([top_cx, 0.0], sd / 2.0, LAYER_MAIN));
        }
        // 底孔整圆（螺纹孔 = 小径粗实线）
        en.push(circle([top_cx, 0.0], base_r, LAYER_MAIN));
        if threaded {
            // 缺口方位 = 库内共有口径（265°→185°，模板换算；见 `partgen_kit` 常量）。
            en.push(crate::partgen_kit::thread_major_arc([top_cx, 0.0], major_r));
        }
        let over = major_r
            .max(base_r)
            .max(v.bore_d.unwrap_or(0.0) / 2.0)
            .max(v.sink_d.unwrap_or(0.0) / 2.0)
            + AXIS_OVER;
        en.push(line([top_cx - over, 0.0], [top_cx + over, 0.0], LAYER_CENTER));
        en.push(line([top_cx, -over], [top_cx, over], LAYER_CENTER));
    }

    Ok(BuiltHole {
        values: v,
        entities: en,
        kind: model.kind,
        range: model.range,
        views: model.views,
    })
}

/// 平移 + 旋转（`Line`/`Arc`/`Circle`；Hatch 不由孔生成器产出）。
pub fn place(entities: Vec<EntityType>, at: [f64; 2], rot_deg: f64) -> Vec<EntityType> {
    let rad = rot_deg.to_radians();
    let (cos, sin) = (rad.cos(), rad.sin());
    let map = move |p: [f64; 2]| {
        [
            at[0] + p[0] * cos - p[1] * sin,
            at[1] + p[0] * sin + p[1] * cos,
        ]
    };
    entities
        .into_iter()
        .map(|e| match e {
            EntityType::Line(mut l) => {
                let a = map([l.start.x, l.start.y]);
                let b = map([l.end.x, l.end.y]);
                l.start = Vector3::new(a[0], a[1], l.start.z);
                l.end = Vector3::new(b[0], b[1], l.end.z);
                EntityType::Line(l)
            }
            EntityType::Arc(mut a) => {
                let c = map([a.center.x, a.center.y]);
                a.center = Vector3::new(c[0], c[1], a.center.z);
                a.start_angle += rad;
                a.end_angle += rad;
                EntityType::Arc(a)
            }
            EntityType::Circle(mut c) => {
                let p = map([c.center.x, c.center.y]);
                c.center = Vector3::new(p[0], p[1], c.center.z);
                EntityType::Circle(c)
            }
            other => other,
        })
        .collect()
}

/// 块名：只由几何模型决定（FNV-1a，跨进程稳定）。
pub fn block_name(model: &HoleModel) -> String {
    let canonical = format!(
        "{}|{}|{:?}|{:?}|{}|{}|{:?}|{:?}|{}|{}|{}|{}|{}|{:?}|{}",
        model.thread_system.key(),
        model.thread_group.as_deref().unwrap_or(""),
        model.kind,
        model.subtype,
        model.d,
        model.pitch.map(fmt).unwrap_or_default(),
        model.hole_depth,
        model.thread_len,
        model.full_thread,
        model.views.side,
        model.views.top,
        model.fit,
        model.custom_d.map(fmt).unwrap_or_default(),
        model.range,
        format!("{:?}|{}", model.drill_d, model.reco),
    );
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in canonical.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("OCSM_HOLE_{hash:016X}")
}

/// OCSM 初始化检查（判据与齿轮/轴相同；措辞改成孔）。
pub fn ocsm_ready(doc: &ocs_plugin_api::host::acadrust::CadDocument) -> Result<(), String> {
    let find = |name: &str| {
        doc.layers
            .iter()
            .find(|ly| ly.name.eq_ignore_ascii_case(name))
    };
    const NEED: [&str; 3] = [LAYER_MAIN, LAYER_THIN, LAYER_CENTER];
    let missing: Vec<&str> = NEED
        .iter()
        .copied()
        .filter(|n| find(n).is_none())
        .collect();
    let center_ok = find(LAYER_CENTER)
        .map(|ly| ly.line_type.eq_ignore_ascii_case("CENTER2"))
        .unwrap_or(false);
    if missing.is_empty() && center_ok {
        return Ok(());
    }
    let why = if !missing.is_empty() {
        t_fmt(
            "cmd.hole.why.missing_layers",
            &[("layers", &missing.join("、"))],
        )
    } else {
        t_fmt("cmd.hole.why.center_linetype", &[("layer", LAYER_CENTER)])
    };
    Err(t_fmt("cmd.hole.err.not_initialized", &[("why", &why)]))
}

// ── CLI（`OCSMHOLE <参数>`）─────────────────────────────────────────────

/// 命令行用法（一行直插；按当前语言从 catalog 取词）。
pub fn usage() -> String {
    t("cmd.hole.usage")
}

/// 取关键字参数：`H=18` / `H18` / `H 18` 都收（`keys` 按长到短放）。
fn take_arg(tokens: &[String], i: &mut usize, t: &str, keys: &[&str]) -> Result<String, String> {
    if let Some((_, v)) = t.split_once('=') {
        if !v.is_empty() {
            return Ok(v.to_string());
        }
    }
    for k in keys {
        if t.get(..k.len()).is_some_and(|p| p.eq_ignore_ascii_case(k)) {
            if let Some(rest) = t.get(k.len()..) {
                if !rest.is_empty() {
                    return Ok(rest.to_string());
                }
            }
        }
    }
    *i += 1;
    tokens
        .get(*i)
        .cloned()
        .ok_or_else(|| t_fmt("cmd.hole.err.value_missing", &[("t", t)]))
}

/// 是否“像规格名”：带数字或 `#`/`×`/`/` 的 token 才试查表；纯词仍是「不识别的参数」。
fn looks_like_spec_name(t: &str) -> bool {
    t.chars().any(|c| c.is_ascii_digit()) || t.contains(['#', '×', '*', '/'])
}

/// 体系内可选规格示例（未命中报错用；优先当前牙型系列，取前 `n` 个名字）。
fn spec_name_samples(sys: ThreadSystem, group: Option<&str>, n: usize) -> String {
    let t = thread::table(sys);
    let mut names: Vec<String> = Vec::new();
    for g in &t.groups {
        if group.is_some_and(|want| want != g.key) {
            continue;
        }
        names.extend(g.rows.iter().take(n).map(|r| r.name.clone()));
        if names.len() >= n {
            break;
        }
    }
    names.truncate(n);
    names.join("、")
}

/// 多词规格名查表（表 `name` 唯一事实来源；见 `thread::lookup_name`）：
/// 先试最长（`1/4-20 UNC`、`1 1/4-12 UNF`、`NPT1 1/4`），未命中由调用方回落既有报错。
/// 返回 (命中, 用掉 token 数)。
fn spec_name_lookup(
    tokens: &[String],
    i: usize,
    m: &HoleModel,
) -> Result<Option<(thread::NameHit, usize)>, String> {
    let max = (tokens.len() - i).min(4);
    for used in (1..=max).rev() {
        let text = tokens[i..i + used].join(" ");
        match thread::lookup_name(m.thread_system, m.thread_group.as_deref(), &text) {
            Ok(hit) => return Ok(Some((hit, used))),
            Err(thread::NameError::Ambiguous(names)) => {
                return Err(t_fmt(
                    "cmd.hole.err.spec_name_ambiguous",
                    &[("text", &text), ("names", &names.join("、"))],
                ))
            }
            Err(thread::NameError::NotFound) => {}
        }
    }
    Ok(None)
}

/// 解析命令行（英文/中文关键字都收）。
pub fn parse_program(text: &str) -> Result<HoleModel, String> {
    let mut s = text.trim();
    for cmd in ["OCSMHOLE", "DK"] {
        if s.get(..cmd.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(cmd))
        {
            s = s[cmd.len()..].trim_start();
            break;
        }
    }
    if s.is_empty() {
        return Err(t("cmd.hole.err.missing_args"));
    }
    let mut m = HoleModel {
        kind: HoleKind::Threaded,
        subtype: HoleSubtype::Standard,
        ..Default::default()
    };
    let mut size_seen = false;
    let mut size_is_dia = false;
    let tokens: Vec<String> = s.split_whitespace().map(|t| t.to_string()).collect();
    let mut i = 0;
    while i < tokens.len() {
        let raw = tokens[i].clone();
        let t = raw.trim();
        let lower = t.to_ascii_lowercase();
        match lower.as_str() {
            "简单孔" | "simple" => {
                m.kind = HoleKind::Simple;
                if matches!(m.subtype, HoleSubtype::Standard | HoleSubtype::Fine) {
                    m.subtype = HoleSubtype::Drill;
                }
            }
            "螺纹孔" | "thread" | "threaded" => {
                m.kind = HoleKind::Threaded;
                m.subtype = HoleSubtype::Standard;
            }
            "沉头孔" | "沉孔" | "counterbore" => m.kind = HoleKind::Counterbore,
            "埋头孔" | "埋孔" | "countersink" => m.kind = HoleKind::Countersink,
            "带螺纹" | "有螺纹" => {
                if m.kind == HoleKind::Counterbore || m.kind == HoleKind::Countersink {
                    m.subtype = HoleSubtype::Standard;
                }
            }
            "无螺纹" | "光孔" => {
                if m.kind == HoleKind::Counterbore || m.kind == HoleKind::Countersink {
                    m.subtype = HoleSubtype::Drill;
                }
            }
            "钻孔" | "钻头" | "钻头大小" | "drill" => m.subtype = HoleSubtype::Drill,
            "70.1" | "gb70.1" | "gb70_1" => m.reco = "gb70_1".to_string(),
            "70.2" | "gb70.2" | "gb70_2" => m.reco = "gb70_2".to_string(),
            "6190" | "gb6190" => m.reco = "gb6190".to_string(),
            "自定义" | "custom" => m.subtype = HoleSubtype::Custom,
            "间隙" | "螺栓间隙" | "clearance" => {
                m.subtype = HoleSubtype::Clearance;
                if m.fit == "6H" {
                    m.fit = "normal".to_string();
                }
            }
            "标准螺纹" | "粗牙" | "standard" => m.subtype = HoleSubtype::Standard,
            "细牙" | "细牙螺纹" | "fine" => m.subtype = HoleSubtype::Fine,
            // 螺纹体系 / 牙型系列（GUI「标准」+「子类型」的 CLI 口径）。
            // 规格两种写法（等价）：① 直接写表 `name`（G1/8 / 1/4-20 / Tr8×1.5…，
            // 未命中才报错，见 fallback 的规格名查表）；② 数值 `公称6.35 P1.058`
            // （mm；英制 P=25.4/TPI），表外报错。两种都保留。
            "公制" | "iso724" | "iso" => m.thread_system = ThreadSystem::Iso724,
            "统一" | "un" | "unified" => {
                m.thread_system = ThreadSystem::Un;
                m.thread_group = Some("unc".to_string());
            }
            "unc" => {
                m.thread_system = ThreadSystem::Un;
                m.thread_group = Some("unc".to_string());
            }
            "unf" => {
                m.thread_system = ThreadSystem::Un;
                m.thread_group = Some("unf".to_string());
            }
            "unef" => {
                m.thread_system = ThreadSystem::Un;
                m.thread_group = Some("unef".to_string());
            }
            "4un" | "6un" | "8un" | "12un" | "16un" | "20un" | "28un" | "32un" => {
                m.thread_system = ThreadSystem::Un;
                m.thread_group = Some(lower.clone());
            }
            "管用平行" | "g" | "bspp" | "pf" => {
                m.thread_system = ThreadSystem::G;
                m.thread_group = Some("standard".to_string());
            }
            "管用锥形" | "r" | "bspt" | "pt" => {
                m.thread_system = ThreadSystem::R;
                m.thread_group = Some("standard".to_string());
            }
            "npt" | "美制锥管" => {
                m.thread_system = ThreadSystem::Npt;
                m.thread_group = Some("standard".to_string());
            }
            "acme" | "爱克母" => {
                m.thread_system = ThreadSystem::Acme;
                m.thread_group = Some("general".to_string());
            }
            "矮牙" | "stub" => {
                m.thread_system = ThreadSystem::Acme;
                m.thread_group = Some("stub".to_string());
            }
            "梯形" | "公制梯形" | "tr" => {
                m.thread_system = ThreadSystem::Tr;
                m.thread_group = Some("standard".to_string());
            }
            "盲孔" | "blind" => m.range = HoleRange::Blind,
            "贯通" | "通孔" | "through" => m.range = HoleRange::Through,
            "全长" | "full" => m.full_thread = true,
            "侧视图" | "side" => m.views = HoleViews { side: true, top: false },
            "俯视图" | "top" => m.views = HoleViews { side: false, top: true },
            "双视图" | "both" => {
                m.views = HoleViews { side: true, top: true };
            }
            "精装配" | "精" => m.fit = "close".to_string(),
            "中等装配" | "中" => m.fit = "normal".to_string(),
            "粗装配" | "粗" => m.fit = "loose".to_string(),
            _ if lower.starts_with("view") || lower.starts_with("视图") => {
                let v = if t.contains('=') {
                    t.split_once('=').map(|(_, v)| v.to_string()).unwrap_or_default()
                } else {
                    i += 1;
                    tokens.get(i).cloned().ok_or_else(|| crate::i18n::t("cmd.hole.err.view_name_missing"))?
                };
                match v.trim() {
                    "侧视图" | "侧" | "side" => m.views = HoleViews { side: true, top: false },
                    "俯视图" | "俯" | "top" => m.views = HoleViews { side: false, top: true },
                    "双视图" | "双" | "both" => {
                        m.views = HoleViews { side: true, top: true };
                    }
                    other => return Err(t_fmt("cmd.hole.err.view_unknown", &[("other", other)])),
                }
            }
            "at" => {
                i += 1;
                let v = tokens.get(i).ok_or_else(|| crate::i18n::t("cmd.hole.err.at_xy_missing"))?;
                let (x, y) = v
                    .split_once(',')
                    .ok_or_else(|| t_fmt("cmd.hole.err.at_not_xy", &[("v", v)]))?;
                let x: f64 = x
                    .trim()
                    .parse()
                    .map_err(|_| t_fmt("cmd.hole.err.x_invalid", &[("x", x)]))?;
                let y: f64 = y
                    .trim()
                    .parse()
                    .map_err(|_| t_fmt("cmd.hole.err.y_invalid", &[("y", y)]))?;
                m.at = Some([x, y]);
            }
            "rot" => {
                i += 1;
                let v = tokens.get(i).ok_or_else(|| crate::i18n::t("cmd.hole.err.rot_missing"))?;
                m.rot = v
                    .parse()
                    .map_err(|_| t_fmt("cmd.hole.err.rot_invalid", &[("v", v)]))?;
            }
            _ if (lower.starts_with('p')
                && lower[1..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit() || c == '='))
                || lower == "p" =>
            {
                let v = take_arg(&tokens, &mut i, t, &["P", "p"])?;
                let p: f64 = v
                    .parse()
                    .map_err(|_| t_fmt("cmd.hole.err.pitch_invalid", &[("v", &v)]))?;
                if p <= 0.0 {
                    return Err(t_fmt(
                        "cmd.hole.err.pitch_nonpositive",
                        &[("p", &p.to_string())],
                    ));
                }
                m.pitch = Some(p);
                if m.pitch.is_some()
                    && m.kind == HoleKind::Threaded
                    && thread_row(m.d, m.pitch)
                        .map(|r| is_fine(r))
                        .unwrap_or(false)
                {
                    m.subtype = HoleSubtype::Fine;
                }
            }
            _ if lower.starts_with('h') && !t.starts_with("孔深")
                || t.starts_with("孔深")
                || t.starts_with('深') =>
            {
                let v = take_arg(&tokens, &mut i, t, &["孔深", "H", "h", "深"])?;
                m.hole_depth = Some(
                    v.parse()
                        .map_err(|_| t_fmt("cmd.hole.err.depth_invalid", &[("v", &v)]))?,
                );
            }
            _ if lower.starts_with('l') || t.starts_with("螺纹长") || t.starts_with('长') => {
                let v = take_arg(&tokens, &mut i, t, &["螺纹长", "L", "l", "长"])?;
                m.thread_len = Some(
                    v.parse()
                        .map_err(|_| t_fmt("cmd.hole.err.thread_len_parse_invalid", &[("v", &v)]))?,
                );
            }
            _ if t.starts_with("孔径") || lower.starts_with("custom") || lower.starts_with("cd") => {
                let v = take_arg(&tokens, &mut i, t, &["孔径", "custom", "CD", "cd"])?;
                m.custom_d = Some(
                    v.parse()
                        .map_err(|_| t_fmt("cmd.hole.err.custom_d_invalid", &[("v", &v)]))?,
                );
            }
            _ if t.starts_with("配合") || lower.starts_with("fit") => {
                let v = take_arg(&tokens, &mut i, t, &["配合", "FIT", "fit"])?;
                let v = v.trim().to_ascii_uppercase();
                if v == "6H" || v == "6G" {
                    m.fit = v;
                } else {
                    return Err(t_fmt("cmd.hole.err.fit_unsupported", &[("v", &v)]));
                }
            }
            _ if t.starts_with("公称") => {
                let rest = take_arg(&tokens, &mut i, t, &["公称", "nominal", "nom"])?;
                let body = rest
                    .trim()
                    .trim_start_matches(['m', 'M', 'Ø', 'Φ'])
                    .replace('×', "x");
                m.d = body
                    .split('x')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .parse()
                    .map_err(|_| t_fmt("cmd.hole.err.nominal_invalid", &[("rest", &rest)]))?;
                size_seen = true;
            }
            _ if t.starts_with("推荐") || lower.starts_with("reco") => {
                let v = take_arg(&tokens, &mut i, t, &["推荐", "RECO", "reco"])?;
                m.reco = match v.trim().to_ascii_uppercase().as_str() {
                    "70.1" | "GB70.1" | "GB70_1" => "gb70_1".to_string(),
                    "70.2" | "GB70.2" | "GB70_2" => "gb70_2".to_string(),
                    "6190" | "GB6190" => "gb6190".to_string(),
                    other => {
                        return Err(t_fmt(
                            "cmd.hole.err.reco_unknown_cli",
                            &[("other", other)],
                        ))
                    }
                };
            }
            _ if t.chars().next().is_some_and(|c| c == 'm' || c == 'M' || c == 'Ø' || c == 'Φ') => {
                let dia = t.starts_with('Ø') || t.starts_with('Φ');
                let body = t
                    .trim_start_matches(['m', 'M', 'Ø', 'Φ'])
                    .replace('×', "x")
                    .replace('X', "x");
                let (d_str, p_str) = match body.split_once('x') {
                    Some((d, p)) => (d.to_string(), Some(p.to_string())),
                    None => (body.clone(), None),
                };
                let v: f64 = d_str
                    .trim()
                    .parse()
                    .map_err(|_| t_fmt("cmd.hole.err.size_invalid", &[("t", t)]))?;
                if dia {
                    m.drill_d = Some(v);
                    if m.kind == HoleKind::Simple || !size_seen {
                        m.d = v;
                    }
                    size_is_dia = true;
                } else {
                    m.d = v;
                }
                if let Some(p) = p_str {
                    let p: f64 = p
                        .trim()
                        .parse()
                        .map_err(|_| t_fmt("cmd.hole.err.pitch_token_invalid", &[("t", t)]))?;
                    m.pitch = Some(p);
                    if is_fine(thread_row(m.d, Some(p))?) {
                        m.subtype = HoleSubtype::Fine;
                    }
                }
                size_seen = true;
            }
            _ if lower == "6h" || lower == "6g" => {
                m.fit = lower.to_ascii_uppercase();
            }
            other => {
                // 先试「规格名」查表（GUI 下拉那个 `name`；表内唯一事实来源）：
                // 命中等价于 `公称 d P`；未命中回落既有数值路径/报错，老写法一律不受影响。
                if !looks_like_spec_name(t) {
                    return Err(t_fmt(
                        "cmd.hole.err.unknown_param",
                        &[("other", other), ("usage", &usage())],
                    ));
                }
                match spec_name_lookup(&tokens, i, &m)? {
                    Some((hit, used)) => {
                        m.thread_group = Some(hit.group.to_string());
                        m.d = hit.spec.d;
                        m.pitch = Some(hit.spec.p);
                        size_seen = true;
                        i += used - 1; // 循环尾还有 +1，净推进 used 个 token
                    }
                    None => {
                        // 未命中：先看是不是别的体系的规格（点名体系冲突），否则报未知规格名 + 示例。
                        let max = (tokens.len() - i).min(4);
                        let mut conflict: Option<(String, Vec<(ThreadSystem, thread::NameHit)>)> =
                            None;
                        for used in 1..=max {
                            let text = tokens[i..i + used].join(" ");
                            let elsewhere = thread::find_name_elsewhere(m.thread_system, &text);
                            if !elsewhere.is_empty() {
                                conflict = Some((text, elsewhere));
                                break;
                            }
                        }
                        if let Some((text, elsewhere)) = conflict {
                            let conflicts: Vec<String> = elsewhere
                                .iter()
                                .map(|(sys, hit)| format!("{} 的 {}", sys.code(), hit.spec.name))
                                .collect();
                            return Err(t_fmt(
                                "cmd.hole.err.system_spec_mismatch",
                                &[
                                    ("sys", m.thread_system.code()),
                                    ("text", &text),
                                    ("conflicts", &conflicts.join("、")),
                                    (
                                        "examples",
                                        &spec_name_samples(
                                            m.thread_system,
                                            m.thread_group.as_deref(),
                                            6,
                                        ),
                                    ),
                                ],
                            ));
                        }
                        let text = tokens[i].clone();
                        return Err(t_fmt(
                            "cmd.hole.err.spec_name_unknown",
                            &[
                                ("text", &text),
                                ("sys", m.thread_system.code()),
                                (
                                    "examples",
                                    &spec_name_samples(
                                        m.thread_system,
                                        m.thread_group.as_deref(),
                                        6,
                                    ),
                                ),
                            ],
                        ));
                    }
                }
            }
        }
        i += 1;
    }
    if !size_seen {
        return Err(t("cmd.hole.err.size_missing"));
    }
    // 简单孔没写子类型时默认「钻头大小」（GUI 同口径；不然会报「简单孔不带螺纹」）
    if m.kind == HoleKind::Simple
        && matches!(m.subtype, HoleSubtype::Standard | HoleSubtype::Fine)
    {
        m.subtype = HoleSubtype::Drill;
    }
    // 裸 Ø 值在没有 M 公称时同时当 d（简单孔）
    if m.subtype == HoleSubtype::Drill && m.drill_d.is_none() && size_is_dia {
        m.drill_d = Some(m.d);
    }
    if m.kind == HoleKind::Simple {
        if let Some(v) = m.drill_d {
            m.d = v;
        }
    }
    Ok(m)
}

// ── SVG 预览（GUI 示意图：剖面 + 底锥 + 螺纹线 + D1/D 标注）────────────────

/// 生成 GUI 预览 SVG（浅色底：材料板 + 绿色剖面线 + 孔几何 + 红色 D1/D 标注）。
pub fn preview_svg(model: &HoleModel) -> Result<String, String> {
    let built = build(model)?;
    let v = &built.values;
    let threaded = v.threaded;
    let base_r = v.base_d / 2.0;
    let major_r = v.major / 2.0;
    let (hd, cone, tl, start) = (v.hole_depth, v.cone_height, v.thread_len, v.base_start);
    let side = model.views.side;
    let top = model.views.top;
    let blind = model.range == HoleRange::Blind;

    // 材料板示意（只影响预览，不进图纸）
    let widest = major_r
        .max(base_r)
        .max(v.bore_d.unwrap_or(0.0) / 2.0)
        .max(v.sink_d.unwrap_or(0.0) / 2.0);
    let plate_w = (widest * 5.2).max(32.0);
    let h_abs = start + hd;
    let plate_bottom = if blind { h_abs + cone + 5.0 } else { h_abs };
    let top_cx = if side { widest + VIEW_GAP + widest } else { 0.0 };

    // 左半剖面轮廓（从上表面往下，到孔底锥尖/底孔底）
    let mut left: Vec<[f64; 2]> = Vec::new();
    if let Some(bd) = v.bore_d {
        left.push([-bd / 2.0, 0.0]);
        left.push([-bd / 2.0, -start]);
    }
    if let Some(sd) = v.sink_d {
        left.push([-sd / 2.0, 0.0]);
        left.push([-base_r, -start]);
    }
    left.push([-base_r, -start]);
    left.push([-base_r, -h_abs]);
    if blind {
        left.push([0.0, -(h_abs + cone)]);
    }

    // 世界 bbox
    let mut bb = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    {
        let mut add = |x: f64, y: f64| {
            bb[0] = bb[0].min(x);
            bb[1] = bb[1].min(y);
            bb[2] = bb[2].max(x);
            bb[3] = bb[3].max(y);
        };
        if side {
            add(-plate_w / 2.0, 0.0);
            add(plate_w / 2.0, -plate_bottom);
            add(0.0, AXIS_OVER);
            if blind {
                add(0.0, -(h_abs + cone));
            }
        }
        if top {
            let over = widest + AXIS_OVER;
            add(top_cx - over, over);
            add(top_cx + over, -over);
        }
    }
    if bb[0] == f64::MAX {
        return Err(t("cmd.hole.err.need_one_view"));
    }
    let (cw, ch) = (520.0_f64, 380.0_f64);
    let pad = 18.0;
    let span_x = (bb[2] - bb[0]).max(1.0);
    let span_y = (bb[3] - bb[1]).max(1.0);
    let s = ((cw - 2.0 * pad) / span_x).min((ch - 2.0 * pad) / span_y);
    let cx = (bb[0] + bb[2]) / 2.0;
    let cy = (bb[1] + bb[3]) / 2.0;
    let tx = move |x: f64| cw / 2.0 + (x - cx) * s;
    let ty = move |y: f64| ch / 2.0 - (y - cy) * s;

    // 材料板多边形（even-odd 挖掉孔；贯通板 = 左右两块）
    let mirror = |l: &[[f64; 2]]| -> Vec<[f64; 2]> { l.iter().rev().map(|p| [-p[0], p[1]]).collect() };
    let mut plate_polys: Vec<Vec<[f64; 2]>> = Vec::new();
    if side {
        if blind {
            let mut poly = vec![[-plate_w / 2.0, 0.0]];
            poly.extend(left.iter().copied());
            poly.extend(mirror(&left));
            poly.push([plate_w / 2.0, 0.0]);
            poly.push([plate_w / 2.0, -plate_bottom]);
            poly.push([-plate_w / 2.0, -plate_bottom]);
            plate_polys.push(poly);
        } else {
            let mut l = vec![[-plate_w / 2.0, 0.0]];
            l.extend(left.iter().copied());
            l.push([-plate_w / 2.0, -plate_bottom]);
            plate_polys.push(l);
            let r: Vec<[f64; 2]> = plate_polys.last().unwrap().iter().map(|p| [-p[0], p[1]]).collect();
            plate_polys.push(r);
        }
    }

    let mut body = String::new();
    for poly in &plate_polys {
        let d_path: String = poly
            .iter()
            .map(|p| format!("{:.2} {:.2}", tx(p[0]), ty(p[1])))
            .collect::<Vec<_>>()
            .join(" L ");
        body.push_str(&format!(
            "<path d='M {d_path} Z' fill='#dfe9d2' stroke='#aeb9a0' stroke-width='1'/>\n"
        ));
        body.push_str(&format!(
            "<path d='M {d_path} Z' fill='url(#ocsmh)' stroke='none'/>\n"
        ));
    }
    // 孔几何
    let stroke = |layer: &str| match layer {
        LAYER_MAIN => ("#2b2b2b", 2.1),
        LAYER_THIN => ("#2f6fd0", 1.2),
        _ => ("#c0392b", 0.9),
    };
    for e in &built.entities {
        match e {
            EntityType::Line(l) => {
                let (c, w) = stroke(&l.common.layer);
                let dash = if l.common.layer == LAYER_CENTER {
                    " stroke-dasharray='8 2 1.5 2'"
                } else {
                    ""
                };
                body.push_str(&format!(
                    "<line x1='{:.2}' y1='{:.2}' x2='{:.2}' y2='{:.2}' stroke='{c}' stroke-width='{w}'{dash}/>\n",
                    tx(l.start.x), ty(l.start.y), tx(l.end.x), ty(l.end.y)
                ));
            }
            EntityType::Circle(c) => {
                let (col, w) = stroke(&c.common.layer);
                body.push_str(&format!(
                    "<circle cx='{:.2}' cy='{:.2}' r='{:.2}' fill='none' stroke='{col}' stroke-width='{w}'/>\n",
                    tx(c.center.x),
                    ty(c.center.y),
                    c.radius * s
                ));
            }
            EntityType::Arc(a) => {
                let (col, w) = stroke(&a.common.layer);
                let (r, x, y) = (a.radius, a.center.x, a.center.y);
                let (mut a0, mut a1) = (a.start_angle.to_degrees(), a.end_angle.to_degrees());
                while a1 < a0 {
                    a1 += 360.0;
                }
                let (sx, sy) = (x + r * a0.to_radians().cos(), y + r * a0.to_radians().sin());
                let (ex, ey) = (x + r * a1.to_radians().cos(), y + r * a1.to_radians().sin());
                let large = if (a1 - a0) > 180.0 { 1 } else { 0 };
                body.push_str(&format!(
                    "<path d='M {:.2} {:.2} A {:.2} {:.2} 0 {} 0 {:.2} {:.2}' fill='none' stroke='{col}' stroke-width='{w}'/>\n",
                    tx(sx), ty(sy), r * s, r * s, large, tx(ex), ty(ey)
                ));
            }
            _ => {}
        }
    }
    // 标注（红色，仅侧视图）：D1 = 大径、D = 小径（参考图的界面示意，不进图纸）
    if side {
        let dim_line = |body: &mut String, p0: [f64; 2], p1: [f64; 2]| {
            body.push_str(&format!(
                "<line x1='{:.2}' y1='{:.2}' x2='{:.2}' y2='{:.2}' stroke='#c0392b' stroke-width='1'/>\n",
                tx(p0[0]), ty(p0[1]), tx(p1[0]), ty(p1[1])
            ));
            let arrow = |x: f64, y: f64, dx: f64| {
                let (px, py) = (tx(x), ty(y));
                format!(
                    "<line x1='{:.1}' y1='{:.1}' x2='{:.1}' y2='{:.1}' stroke='#c0392b' stroke-width='1'/>\
                     <line x1='{:.1}' y1='{:.1}' x2='{:.1}' y2='{:.1}' stroke='#c0392b' stroke-width='1'/>",
                    px, py, px + dx * 8.0, py - 3.5,
                    px, py, px + dx * 8.0, py + 3.5
                )
            };
            body.push_str(&arrow(p0[0], p0[1], 1.0));
            body.push_str(&arrow(p1[0], p1[1], -1.0));
        };
        let y_major = -(start + tl * 0.45);
        let y_minor = -(start + (hd * 0.88).max(tl * 0.6));
        dim_line(&mut body, [-major_r, y_major], [major_r, y_major]);
        if threaded {
            dim_line(&mut body, [-base_r, y_minor], [base_r, y_minor]);
            body.push_str(&format!(
                "<text x='{:.2}' y='{:.2}' font-size='12' fill='#c0392b' text-anchor='end'>D1={}</text>\n",
                tx(-major_r) - 6.0,
                ty(y_major) - 3.0,
                fmt3(v.major)
            ));
            body.push_str(&format!(
                "<text x='{:.2}' y='{:.2}' font-size='12' fill='#c0392b' text-anchor='end'>D={}</text>\n",
                tx(-base_r) - 6.0,
                ty(y_minor) + 13.0,
                fmt3(v.minor)
            ));
        } else {
            body.push_str(&format!(
                "<text x='{:.2}' y='{:.2}' font-size='12' fill='#c0392b' text-anchor='end'>Ø{}</text>\n",
                tx(-major_r) - 6.0,
                ty(y_major) - 3.0,
                fmt3(v.base_d)
            ));
        }
        // 沉头/埋头尺寸文字（示意）
        if let (Some(bd), Some(bt)) = (v.bore_d, v.bore_t) {
            body.push_str(&format!(
                "<text x='{:.2}' y='{:.2}' font-size='11' fill='#7a5c2e'>沉孔Ø{}×{}</text>\n",
                tx(-plate_w / 2.0) + 4.0,
                ty(0.0) - 6.0,
                fmt3(bd),
                fmt3(bt)
            ));
        }
        if let Some(sd) = v.sink_d {
            body.push_str(&format!(
                "<text x='{:.2}' y='{:.2}' font-size='11' fill='#7a5c2e'>埋头Ø{} 90°</text>\n",
                tx(-plate_w / 2.0) + 4.0,
                ty(0.0) - 6.0,
                fmt3(sd)
            ));
        }
    }

    Ok(format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{cw}' height='{ch}' viewBox='0 0 {cw} {ch}'>\
         <defs><pattern id='ocsmh' patternUnits='userSpaceOnUse' width='8' height='8' patternTransform='rotate(45)'>\
         <line x1='0' y1='0' x2='0' y2='8' stroke='#b6c8a0' stroke-width='1.2'/></pattern></defs>\
         <rect width='{cw}' height='{ch}' fill='#fbfbf7'/>\n{body}</svg>"
    ))
}

// ── 小工具 ──────────────────────────────────────────────────────────────

/// 3 位小数去尾零（界面/预览；M10 小径 → `8.376`）。
pub fn fmt3(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// JSON 里的紧凑数字。
pub fn fmt(v: f64) -> String {
    crate::partgen_kit::trim(v)
}

fn line(a: [f64; 2], b: [f64; 2], layer: &str) -> EntityType {
    let mut l = Line::from_points(
        Vector3::new(a[0], a[1], 0.0),
        Vector3::new(b[0], b[1], 0.0),
    );
    set_layer(&mut l, layer);
    EntityType::Line(l)
}

fn arc(c: [f64; 2], r: f64, start_deg: f64, end_deg: f64, layer: &str) -> EntityType {
    let mut a = Arc::from_center_radius_angles(
        Vector3::new(c[0], c[1], 0.0),
        r,
        start_deg.to_radians(),
        end_deg.to_radians(),
    );
    set_layer(&mut a, layer);
    EntityType::Arc(a)
}

fn circle(c: [f64; 2], r: f64, layer: &str) -> EntityType {
    let mut e = Circle::from_center_radius(Vector3::new(c[0], c[1], 0.0), r);
    set_layer(&mut e, layer);
    EntityType::Circle(e)
}

fn set_layer<T: CommonLayer>(e: &mut T, layer: &str) {
    let c = e.common_mut();
    c.layer = layer.to_string();
    c.color = Color::ByLayer;
    c.linetype = "ByLayer".to_string();
    c.line_weight = LineWeight::ByLayer;
}

trait CommonLayer {
    fn common_mut(&mut self) -> &mut acadrust::entities::EntityCommon;
}

impl CommonLayer for Line {
    fn common_mut(&mut self) -> &mut acadrust::entities::EntityCommon {
        &mut self.common
    }
}

impl CommonLayer for Arc {
    fn common_mut(&mut self) -> &mut acadrust::entities::EntityCommon {
        &mut self.common
    }
}

impl CommonLayer for Circle {
    fn common_mut(&mut self) -> &mut acadrust::entities::EntityCommon {
        &mut self.common
    }
}

// ── 测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// 语言是进程级全局：断言中文文案的用例与「切语言」用例共用锁并钉死 zh。
    fn zh_guard() -> std::sync::MutexGuard<'static, ()> {
        let g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        g
    }

    fn thread(d: f64, p: Option<f64>) -> HoleModel {
        let fine = p
            .map(|p| thread_row(d, Some(p)).map(is_fine).unwrap_or(false))
            .unwrap_or(false);
        HoleModel {
            kind: HoleKind::Threaded,
            subtype: if fine {
                HoleSubtype::Fine
            } else {
                HoleSubtype::Standard
            },
            d,
            pitch: p,
            ..Default::default()
        }
    }

    /// `resolve(parse(...))` 一步到位（CLI 测试用）。
    fn values(program: &str) -> HoleValues {
        let m = parse_program(program).unwrap_or_else(|e| panic!("{program} 解析失败：{e}"));
        resolve(&m).unwrap_or_else(|e| panic!("{program} 解算失败：{e}"))
    }

    /// 「按名查表 == 按数值等价」的逐值断言（含体系与管螺纹长度列）。
    fn same_values(a: &HoleValues, b: &HoleValues, tag: &str) {
        assert_eq!(a.size_name, b.size_name, "{tag}: size_name");
        assert_eq!(a.system, b.system, "{tag}: system");
        assert_eq!(a.tpi, b.tpi, "{tag}: tpi");
        assert_eq!(a.gauge_len, b.gauge_len, "{tag}: gauge_len");
        assert_eq!(a.eff_len, b.eff_len, "{tag}: eff_len");
        for (x, y, what) in [
            (a.major, b.major, "major"),
            (a.minor, b.minor, "minor"),
            (a.pitch, b.pitch, "pitch"),
            (a.base_d, b.base_d, "base_d"),
            (a.thread_len, b.thread_len, "thread_len"),
            (a.hole_depth, b.hole_depth, "hole_depth"),
        ] {
            assert!((x - y).abs() < 1e-9, "{tag}: {what} {x} != {y}");
        }
    }

    fn plain(kind: HoleKind, subtype: HoleSubtype, d: f64) -> HoleModel {
        let mut m = HoleModel {
            kind,
            subtype,
            d,
            fit: "normal".to_string(),
            hole_depth: Some(20.0),
            ..Default::default()
        };
        if subtype == HoleSubtype::Drill {
            m.drill_d = Some(if kind == HoleKind::Simple { d } else { 8.5 });
        }
        if subtype == HoleSubtype::Custom {
            m.custom_d = Some(5.0);
        }
        m
    }

    /// 自动螺纹长度 = 1.5d（GB/T 3098.1-2010 口径）。
    #[test]
    fn auto_thread_length_is_1_5d() {
        for (d, p, want) in [(10.0, 1.5, 15.0), (12.0, 1.75, 18.0), (6.0, 1.0, 9.0)] {
            let v = resolve(&thread(d, Some(p))).unwrap();
            assert!((v.thread_len - want).abs() < 1e-9, "M{d}×{p} → {}", v.thread_len);
        }
        // 粗牙（不给 P）同样 1.5d
        let v = resolve(&thread(10.0, None)).unwrap();
        assert!((v.thread_len - 15.0).abs() < 1e-9);
        assert!((v.pitch - 1.5).abs() < 1e-9);
    }

    /// 自动孔深 = 有效螺纹深 + 2P（M10×1.5、有效 15 → 18）。
    #[test]
    fn auto_hole_depth_is_thread_len_plus_2p() {
        let v = resolve(&thread(10.0, Some(1.5))).unwrap();
        assert!((v.thread_len - 15.0).abs() < 1e-9);
        assert!((v.hole_depth - 18.0).abs() < 1e-9);
        let mut m = thread(10.0, Some(1.5));
        m.thread_len = Some(20.0);
        let v = resolve(&m).unwrap();
        assert!((v.hole_depth - 23.0).abs() < 1e-9);
        // 沉头/埋头带螺纹：孔深也从沉孔底起算，自动仍是 L + 2P
        let mut m = thread(10.0, None);
        m.kind = HoleKind::Counterbore;
        let v = resolve(&m).unwrap();
        assert!((v.bore_t.unwrap() - 11.0).abs() < 1e-9);
        assert!((v.base_start - 11.0).abs() < 1e-9);
        assert!((v.hole_depth - 18.0).abs() < 1e-9);
    }

    /// 不带螺纹：孔深不能自动（简单孔 / 沉头 / 埋头都是）。
    #[test]
    fn unthreaded_rejects_auto_depth() {
        let _g = zh_guard();
        for (kind, sub) in [
            (HoleKind::Simple, HoleSubtype::Drill),
            (HoleKind::Simple, HoleSubtype::Custom),
            (HoleKind::Simple, HoleSubtype::Clearance),
            (HoleKind::Counterbore, HoleSubtype::Drill),
            (HoleKind::Countersink, HoleSubtype::Clearance),
        ] {
            let mut m = plain(kind, sub, 10.0);
            m.hole_depth = None;
            m.custom_d = Some(5.0);
            assert!(
                resolve(&m).unwrap_err().contains("自动"),
                "{kind:?}/{sub:?} 应拒绝自动孔深"
            );
        }
    }

    /// 贯通：无 118° 锥（几何层面也无斜锥线）。
    #[test]
    fn through_hole_has_no_cone_entities() {
        let mut m = thread(10.0, Some(1.5));
        m.range = HoleRange::Through;
        m.hole_depth = Some(30.0);
        let v = resolve(&m).unwrap();
        assert_eq!(v.cone_height, 0.0);
        let built = build(&m).unwrap();
        for e in &built.entities {
            if let EntityType::Line(l) = e {
                // 中心线允许按 AXIS_OVER 外伸；其余图元不得低于孔底
                if l.common.layer != LAYER_CENTER {
                    assert!(
                        l.start.y >= -30.0 - 1e-9 && l.end.y >= -30.0 - 1e-9,
                        "贯通孔出现了锥尖/更深图元"
                    );
                }
                if l.common.layer == LAYER_MAIN
                    && (l.start.x - l.end.x).abs() > 1e-9
                    && (l.start.y - l.end.y).abs() > 1e-9
                {
                    panic!("贯通侧视出现斜线（锥）");
                }
            }
        }
        // 盲孔同参数一定有锥（两条斜线）
        m.range = HoleRange::Blind;
        let bb = build(&m).unwrap();
        let diag = bb
            .entities
            .iter()
            .filter(|e| {
                if let EntityType::Line(l) = e {
                    l.common.layer == LAYER_MAIN
                        && (l.start.x - l.end.x).abs() > 1e-9
                        && (l.start.y - l.end.y).abs() > 1e-9
                } else {
                    false
                }
            })
            .count();
        assert_eq!(diag, 2, "盲孔应有两条 118° 锥斜线");
    }

    /// 大径/小径对表：d−1.0825P 与参考表 d1 交叉校验（M10×1.5 → 8.376）。
    #[test]
    fn major_minor_match_formula_and_reference_table() {
        let t = thread_table();
        for r in t.coarse.iter().chain(t.fine.iter()) {
            let calc = r.d - 1.0825 * r.p;
            assert!(
                (calc - r.d1).abs() < 0.0015,
                "{}：公式 {calc:.4} vs 表 {}",
                r.name,
                r.d1
            );
        }
        let v = resolve(&thread(10.0, Some(1.5))).unwrap();
        assert_eq!(fmt3(v.major), "10");
        assert_eq!(fmt3(v.minor), "8.376");
        for r in &t.coarse {
            if let Some(row) = crate::partgen_more::nut_row(r.d, false) {
                assert!(
                    (row.pitch - r.p).abs() < 1e-9,
                    "M{}：ISO724 P={} vs 仓库 P={}",
                    r.d,
                    r.p,
                    row.pitch
                );
            }
        }
    }

    /// 沉头/埋头/间隙/钻头四表交叉校验 + 数值样例。
    #[test]
    fn new_tables_cross_check() {
        // GB/T 152.3 表1/表2 的 d1 = 5277 中等装配（M1.6 除外：表1 1.8 vs 2.0，为 1988 版口径）
        let mut diffs = 0;
        for r in &counterbore_table().rows {
            let c = clearance_row(r.d).unwrap();
            if (c.normal - r.d1).abs() > 1e-9 {
                diffs += 1;
                assert!((r.d - 1.6).abs() < 1e-9, "沉头表 M{} d1 与 5277 不符", r.d);
            }
        }
        assert_eq!(diffs, 1, "只有 M1.6 一处已知差异");
        // GB/T 152.2-2014 的 dh = 5277 中等装配（M1.6 同上）
        let mut diffs = 0;
        for r in &countersink_table().rows {
            let c = clearance_row(r.d).unwrap();
            if (c.normal - r.d1).abs() > 1e-9 {
                diffs += 1;
                assert!((r.d - 1.6).abs() < 1e-9, "埋头表 M{} d1 与 5277 不符", r.d);
            }
        }
        assert_eq!(diffs, 1, "埋头表只应有 M1.6 一处已知差异");
        // 样例：M10 沉头（70.1 表1）= Ø18×11；M10 埋头（152.2-2014）= Ø20.3；M10 间隙 = 10.5/11/12
        let cb = counterbore_row(10.0, "gb70_1").unwrap();
        assert!((cb.d2 - 18.0).abs() < 1e-9 && (cb.t - 11.0).abs() < 1e-9);
        // 官方 M30 d3=36（JLC 页误写 6）
        let m30 = counterbore_row(30.0, "gb70_1").unwrap();
        assert_eq!(m30.d3, Some(36.0), "GB/T 152.3 官方 M30 d3=36");
        let cs = countersink_row(10.0).unwrap();
        assert!((cs.d2 - 20.3).abs() < 1e-9);
        let cl = clearance_row(10.0).unwrap();
        assert!((cl.close - 10.5).abs() < 1e-9);
        assert!((cl.normal - 11.0).abs() < 1e-9);
        assert!((cl.loose - 12.0).abs() < 1e-9);
        // 底孔牙深表（螺纹孔底孔）样例：M10×1.5 → Ø8.5；M12×1.25 → Ø10.8
        assert!((tap_row(10.0, Some(1.5)).unwrap().drill - 8.5).abs() < 1e-9);
        assert!((tap_row(12.0, Some(1.25)).unwrap().drill - 10.8).abs() < 1e-9);
        // 标准麻花钻系列：用户例 ∅5.1/5.2/6.7/6.8/8.5 均在；0.20–20.00
        for x in [5.1, 5.2, 6.7, 6.8, 8.5, 0.2, 20.0] {
            assert!(
                drill_table().diameters.iter().any(|v| (v - x).abs() < 1e-9),
                "麻花钻系列缺 Ø{x}"
            );
        }
        assert_eq!(drill_table().diameters.len(), 198);
        // 表外明确报错（不插值）
        assert!(counterbore_row(7.0, "gb70_1").is_err());
        assert!(counterbore_row(10.0, "gb70_2").unwrap_err().contains("70.2"));
        assert!(countersink_row(24.0).is_err());
        assert!(clearance_row(0.9).is_err());
        assert!(tap_row(13.0, None).is_err());
    }

    /// 四类孔的关键派生：沉头底孔起算点、埋头 90° 几何、间隙三档。
    #[test]
    fn kinds_derive_correctly() {
        let _g = zh_guard();
        // 沉头 + 螺纹：底孔小径从沉孔底起算
        let mut m = thread(10.0, None);
        m.kind = HoleKind::Counterbore;
        let v = resolve(&m).unwrap();
        assert_eq!(v.bore_d, Some(18.0));
        assert_eq!(v.bore_t, Some(11.0));
        assert!((v.base_start - 11.0).abs() < 1e-9);
        // 沉头 + 间隙（粗装配）：底孔 12，沉孔仍 Ø18×11
        let mut m = plain(HoleKind::Counterbore, HoleSubtype::Clearance, 10.0);
        m.fit = "loose".to_string();
        let v = resolve(&m).unwrap();
        assert!((v.base_d - 12.0).abs() < 1e-9);
        assert!((v.base_start - 11.0).abs() < 1e-9);
        assert_eq!(v.sink_d, None);
        // 埋头 + 间隙（精装配）：底孔 10.5，90° 几何深 =（20.3−10.5）/2 = 4.9
        let mut m = plain(HoleKind::Countersink, HoleSubtype::Clearance, 10.0);
        m.fit = "close".to_string();
        let v = resolve(&m).unwrap();
        assert!((v.base_d - 10.5).abs() < 1e-9);
        assert!((v.sink_d.unwrap() - 20.3).abs() < 1e-9);
        assert!((v.base_start - 4.9).abs() < 1e-9);
        // 钻头大小：标准麻花钻 Ø6.8（GB/T 6135.3）
        let mut m = plain(HoleKind::Simple, HoleSubtype::Drill, 6.8);
        m.drill_d = Some(6.8);
        let v = resolve(&m).unwrap();
        assert!((v.base_d - 6.8).abs() < 1e-9);
        // 表外钻头（Ø6.85 不在直柄系列）→ 明确报错
        m.drill_d = Some(6.85);
        assert!(resolve(&m).unwrap_err().contains("麻花钻"));
        m.drill_d = Some(6.8);
        // 自定义：孔径 5
        let mut m = plain(HoleKind::Simple, HoleSubtype::Custom, 10.0);
        m.custom_d = Some(5.0);
        let v = resolve(&m).unwrap();
        assert!((v.base_d - 5.0).abs() < 1e-9);
        // 底孔 ≥ 沉孔直径 → 报错
        let mut m = plain(HoleKind::Counterbore, HoleSubtype::Custom, 4.0);
        m.custom_d = Some(9.0);
        assert!(resolve(&m).unwrap_err().contains("沉孔直径"));
    }

    /// 图层只用 OCSM 五层之三；不生成尺寸标注。
    #[test]
    fn layers_are_ocsm_and_no_dimension() {
        for kind in [
            HoleKind::Simple,
            HoleKind::Threaded,
            HoleKind::Counterbore,
            HoleKind::Countersink,
        ] {
            let mut m = if kind == HoleKind::Threaded {
                thread(10.0, None)
            } else {
                plain(kind, HoleSubtype::Drill, 10.0)
            };
            m.kind = kind;
            m.views = HoleViews { side: true, top: true };
            let built = build(&m).unwrap();
            assert!(!built.entities.is_empty(), "{kind:?} 应有图元");
            for e in &built.entities {
                let layer = match e {
                    EntityType::Line(l) => &l.common.layer,
                    EntityType::Arc(a) => &a.common.layer,
                    EntityType::Circle(c) => &c.common.layer,
                    EntityType::Dimension(_) => panic!("孔生成器不得输出尺寸标注"),
                    other => panic!("孔生成器不应输出 {:?}", std::mem::discriminant(other)),
                };
                assert!(
                    [LAYER_MAIN, LAYER_THIN, LAYER_CENTER].contains(&layer.as_str()),
                    "越层：{layer}"
                );
            }
        }
        // 螺纹孔俯视含大径细实线 3/4 圈；起止角 = 模板换算 265°→185°（用户 2026-09-25 裁定）。
        let mut m = thread(10.0, None);
        m.views = HoleViews { side: false, top: true };
        let built = build(&m).unwrap();
        let arcs: Vec<_> = built
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) if a.common.layer == LAYER_THIN => Some(a),
                _ => None,
            })
            .collect();
        assert_eq!(arcs.len(), 1, "俯视应恰有 1 条大径细弧");
        let a = arcs[0];
        assert!((a.radius - 10.0 / 2.0).abs() < 1e-9, "大径半径 = d/2");
        assert!(
            (a.start_angle.to_degrees() - 265.0).abs() < 1e-9
                && (a.end_angle.to_degrees() - 185.0).abs() < 1e-9,
            "起止角应为模板换算 265°→185°（实得 {}→{}）",
            a.start_angle.to_degrees(),
            a.end_angle.to_degrees()
        );
        // 负断言：不得再出现旧的 270°→180°。
        assert!(
            !built.entities.iter().any(|e| matches!(e, EntityType::Arc(a)
                if a.common.layer == LAYER_THIN
                    && (a.start_angle.to_degrees() - 270.0).abs() < 1e-9
                    && (a.end_angle.to_degrees() - 180.0).abs() < 1e-9)),
            "旧的 270°→180° 口径应已作废"
        );
    }

    /// 沉头/埋头几何：沉孔壁/底、90° 锥；俯视多一圈。
    #[test]
    fn counterbore_and_countersink_geometry() {
        let m = plain(HoleKind::Counterbore, HoleSubtype::Drill, 10.0);
        let built = build(&m).unwrap();
        // 沉孔壁在 ±9（Ø18）
        assert!(built.entities.iter().any(|e| matches!(
            e, EntityType::Line(l) if (l.start.x.abs()-9.0).abs()<1e-9 && l.start.y.abs()<1e-9
        )));
        // 沉孔底面水平线：从 ±9 到 ±4.25（Ø8.5）
        assert!(built.entities.iter().any(|e| matches!(
            e, EntityType::Line(l)
                if l.common.layer == LAYER_MAIN
                    && (l.start.y + 11.0).abs() < 1e-9
                    && (l.end.y + 11.0).abs() < 1e-9
                    && l.start.x.abs() > 8.9
                    && l.end.x.abs() < 4.3
        )));
        // 埋头 90°：斜线从 Ø20.3 到 Ø8.5，深 =（20.3−8.5）/2 = 5.9
        let m = plain(HoleKind::Countersink, HoleSubtype::Drill, 10.0);
        let built = build(&m).unwrap();
        assert!(built.entities.iter().any(|e| matches!(
            e, EntityType::Line(l)
                if (l.start.x.abs() - 10.15).abs() < 1e-9 && l.start.y.abs() < 1e-9
                && (l.end.x.abs() - 4.25).abs() < 1e-9
                && (l.end.y + 5.9).abs() < 1e-9
        )));
        // 俯视：沉头 = 外圈 + 底孔两圆
        let mut m = plain(HoleKind::Counterbore, HoleSubtype::Drill, 10.0);
        m.views = HoleViews { side: false, top: true };
        let circles = build(&m)
            .unwrap()
            .entities
            .iter()
            .filter(|e| matches!(e, EntityType::Circle(_)))
            .count();
        assert_eq!(circles, 2);
    }

    /// 全长：L = H；孔深自动被拒；不画终止线。
    #[test]
    fn full_thread_takes_hole_depth_and_requires_manual_depth() {
        let _g = zh_guard();
        let mut m = thread(10.0, Some(1.5));
        m.full_thread = true;
        assert!(resolve(&m).unwrap_err().contains("全长"));
        m.hole_depth = Some(25.0);
        let v = resolve(&m).unwrap();
        assert!((v.thread_len - 25.0).abs() < 1e-9);
        let built = build(&m).unwrap();
        let end_lines = built
            .entities
            .iter()
            .filter(|e| {
                if let EntityType::Line(l) = e {
                    l.common.layer == LAYER_MAIN
                        && (l.start.y - l.end.y).abs() < 1e-9
                        && (l.start.x - l.end.x).abs() > 1e-9
                        && l.start.y.abs() < 25.0 + 1e-9
                        && (l.start.x.abs() - m.d / 2.0).abs() < 1e-9
                } else {
                    false
                }
            })
            .count();
        assert_eq!(end_lines, 0, "全长螺纹不应有终止线");
    }

    /// 贯通孔深不能自动；手动 L > H 报错；退出不足 2P 给软警告。
    #[test]
    fn through_and_runout_validation() {
        let _g = zh_guard();
        let mut m = thread(10.0, Some(1.5));
        m.range = HoleRange::Through;
        assert!(resolve(&m).unwrap_err().contains("贯通"));
        m.hole_depth = Some(17.5);
        let v = resolve(&m).unwrap();
        assert_eq!(v.cone_height, 0.0);
        // 贯通孔不需要刀具退出 → 不警告
        assert!(v.warnings.is_empty(), "贯通孔不应有退出警告");
        // 盲孔同参数：退出 < 2P → 软警告
        m.range = HoleRange::Blind;
        m.hole_depth = Some(17.5);
        let v = resolve(&m).unwrap();
        assert_eq!(v.warnings.len(), 1, "盲孔应有一条退出不足警告");
        m.hole_depth = Some(18.0);
        assert!(resolve(&m).unwrap().warnings.is_empty(), "18 = L+2P 不应警告");
        m.thread_len = Some(20.0);
        assert!(resolve(&m).is_err(), "L > H 必须报错");
    }

    /// 子类型与孔类型不匹配时报错。
    #[test]
    fn subtype_kind_consistency() {
        let mut m = thread(10.0, None);
        m.subtype = HoleSubtype::Drill;
        assert!(resolve(&m).is_err(), "螺纹孔不能选钻头大小");
        let mut m = plain(HoleKind::Simple, HoleSubtype::Standard, 10.0);
        m.subtype = HoleSubtype::Standard;
        assert!(resolve(&m).is_err(), "简单孔不能选标准螺纹");
        // 细牙子类型必须给细牙螺距
        let mut m = thread(10.0, None);
        m.subtype = HoleSubtype::Fine;
        assert!(resolve(&m).is_err());
    }

    /// DSL 解析：四类孔、带/不带螺纹、配合、孔径、贯通、视图、at/rot。
    #[test]
    fn parse_program_accepts_cli_forms() {
        let m = parse_program("螺纹孔 M10×1.25 H18 L15 贯通 双视图 at 100,50 rot 30").unwrap();
        assert_eq!(m.kind, HoleKind::Threaded);
        assert_eq!(m.d, 10.0);
        assert_eq!(m.pitch, Some(1.25));
        assert_eq!(m.subtype, HoleSubtype::Fine);
        assert_eq!(m.hole_depth, Some(18.0));
        assert_eq!(m.thread_len, Some(15.0));
        assert_eq!(m.range, HoleRange::Through);
        assert_eq!(m.views, HoleViews { side: true, top: true });
        assert_eq!(m.at, Some([100.0, 50.0]));
        assert_eq!(m.rot, 30.0);

        let s = parse_program("简单孔 钻孔 Ø8.5 H20 俯视图").unwrap();
        assert_eq!(s.kind, HoleKind::Simple);
        assert_eq!(s.subtype, HoleSubtype::Drill);
        assert_eq!(s.drill_d, Some(8.5));
        assert_eq!(s.d, 8.5, "简单孔 Ø = 公称");
        assert_eq!(s.hole_depth, Some(20.0));
        assert_eq!(s.views, HoleViews { side: false, top: true });
        assert!(resolve(&s).is_ok());
        // 简单孔没写子类型 → 默认钻头大小（不报「简单孔不带螺纹」）；需给 Ø 钻头
        let d = parse_program("简单孔 钻头 Ø8.5 H20").unwrap();
        assert_eq!(d.subtype, HoleSubtype::Drill);
        assert!(resolve(&d).is_ok());
        assert!(parse_program("简单孔 M10 H20").is_err() || resolve(&parse_program("简单孔 M10 H20").unwrap()).is_err());

        let c = parse_program("沉头孔 无螺纹 M10 间隙 中等装配 H22").unwrap();
        assert_eq!(c.kind, HoleKind::Counterbore);
        assert_eq!(c.subtype, HoleSubtype::Clearance);
        assert_eq!(c.fit, "normal");
        assert_eq!(c.reco, "gb70_1");
        // 沉头推荐值 70.2 → 解析得到，但 resolve 明确报「标缺」（GB/T 152.3 无此表）
        let c72 = parse_program("沉头孔 无螺纹 M10 间隙 H22 推荐70.2").unwrap();
        assert_eq!(c72.reco, "gb70_2");
        assert!(resolve(&c72).unwrap_err().contains("70.2"));
        // 沉头 + 公称 + 钻头（两个值）
        let cd = parse_program("沉头孔 无螺纹 钻孔 Ø8.5 公称M10 H22").unwrap();
        assert_eq!(cd.d, 10.0);
        assert_eq!(cd.drill_d, Some(8.5));
        assert!(resolve(&cd).is_ok());

        let k = parse_program("埋头孔 带螺纹 M10 细牙 全长 H30 配合6G").unwrap();
        assert_eq!(k.kind, HoleKind::Countersink);
        assert_eq!(k.subtype, HoleSubtype::Fine);
        assert_eq!(k.fit, "6G");
        assert!(k.full_thread);

        let u = parse_program("简单孔 M10 自定义 孔径 5 H12").unwrap();
        assert_eq!(u.subtype, HoleSubtype::Custom);
        assert_eq!(u.custom_d, Some(5.0));

        assert!(parse_program("M10 不认识的参数").is_err());
        assert!(parse_program("").is_err());
    }

    /// CLI 规格名查表（与 GUI 下拉同一份表 `name`）：
    /// ● 7 类体系各 ≥2 个字面量：逐条断言「按名 == 按数值」（同值）；
    /// ● 变体容错（空格 / ASCII x / 省牙型后缀 / 省体系代号 / 大小写）；
    /// ● 报错具体（未知规格名 + 示例；体系不匹配点名冲突；歧义列出候选）；
    /// ● 回归：`公称/P`、`M10`、`P1.5`、`盲孔/贯通` 等老写法逐条仍通。
    #[test]
    fn parse_program_accepts_thread_spec_names() {
        let _g = zh_guard();
        // ① 7 类体系 × 2 个字面量，name 写法 == 数值（公称/P）写法
        let pairs: [(&str, &str); 14] = [
            ("螺纹孔 M10", "螺纹孔 公称10 P1.5"),
            ("螺纹孔 M10×1.25", "螺纹孔 公称10 P1.25"),
            ("螺纹孔 UNC 1/4-20", "螺纹孔 UN 公称6.35 P1.27"),
            ("螺纹孔 UNC #1-64", "螺纹孔 UN 公称1.8542 P0.396875"),
            ("螺纹孔 G G1/8", "螺纹孔 G 公称9.728 P0.907143"),
            ("螺纹孔 G G1 1/2", "螺纹孔 G 公称47.803 P2.309091"),
            ("螺纹孔 R R1/8", "螺纹孔 R 公称9.728 P0.907143"),
            ("螺纹孔 R R6", "螺纹孔 R 公称163.83 P2.309091"),
            ("螺纹孔 NPT NPT1/2", "螺纹孔 NPT 公称21.224 P1.814286"),
            ("螺纹孔 NPT NPT1 1/4", "螺纹孔 NPT 公称41.985 P2.208696"),
            ("螺纹孔 ACME 1/4-16", "螺纹孔 ACME 公称6.35 P1.5875"),
            ("螺纹孔 矮牙 1/4-16", "螺纹孔 矮牙 公称6.35 P1.5875"),
            ("螺纹孔 Tr Tr8×1.5", "螺纹孔 Tr 公称8 P1.5"),
            ("螺纹孔 Tr Tr10×2", "螺纹孔 Tr 公称10 P2"),
        ];
        for (name, numeric) in pairs {
            same_values(&values(name), &values(numeric), name);
            // 名字命中的模型能一直走到出图元（纯解析层之外的 build 无分支差异）
            build(&parse_program(name).unwrap())
                .unwrap_or_else(|e| panic!("{name} 出图失败：{e}"));
        }

        // ② 变体容错：同一行不同写法互相等价
        let g18 = values("螺纹孔 G G1/8");
        for variant in [
            "DK 螺纹孔 带螺纹 G G1/8",
            "螺纹孔 G 1/8",
            "螺纹孔 管用平行 g1/8",
        ] {
            same_values(&values(variant), &g18, variant);
        }
        let unc = values("螺纹孔 UNC 1/4-20");
        for variant in [
            "螺纹孔 UN 1/4-20",
            "螺纹孔 UN 1/4-20 UNC",
            "螺纹孔 un 1/4-20 Unc",
        ] {
            same_values(&values(variant), &unc, variant);
        }
        let npt = values("螺纹孔 NPT NPT1/2");
        for variant in ["螺纹孔 NPT 1/2", "螺纹孔 NPT NPT1/2"] {
            same_values(&values(variant), &npt, variant);
        }
        let tr = values("螺纹孔 Tr Tr8×1.5");
        for variant in ["螺纹孔 Tr Tr8x1.5", "螺纹孔 Tr 8x1.5", "螺纹孔 Tr 8×1.5"] {
            same_values(&values(variant), &tr, variant);
        }
        // 矮牙 = stub 组（与 general 的底孔不同；名字带 Stub 也命中同组）
        let stub = values("螺纹孔 矮牙 1/4-16");
        for variant in ["螺纹孔 矮牙 1/4-16 Stub ACME", "螺纹孔 Stub 1/4-16"] {
            same_values(&values(variant), &stub, variant);
        }
        // 按名命中的组会回写到模型（保证 resolve 走同一行）
        let m = parse_program("螺纹孔 UN 5/16-24").unwrap();
        assert_eq!(m.thread_group.as_deref(), Some("unf"), "按名跨组归位");
        assert_eq!((m.d, m.pitch), (7.9375, Some(1.058333)));

        // ③ 报错具体
        let e = parse_program("螺纹孔 G 9/16").unwrap_err();
        assert!(e.contains("未知规格名「9/16」"), "{e}");
        assert!(e.contains("G1/8"), "应给体系内示例：{e}");
        let e = parse_program("螺纹孔 G 1/4-20").unwrap_err();
        assert!(e.contains("体系与规格不匹配") && e.contains("UN"), "{e}");
        assert!(e.contains("1/4-20 UNC"), "应点名冲突行：{e}");
        let e = parse_program("螺纹孔 G G1/").unwrap_err();
        assert!(e.contains("命中多行"), "{e}");
        let e = parse_program("螺纹孔 1/4-20 UNC").unwrap_err();
        assert!(e.contains("体系与规格不匹配") && e.contains("UN"), "{e}");
        // 非规格词的旧报错不变
        assert!(parse_program("M10 不认识的参数")
            .unwrap_err()
            .contains("不识别的参数"));

        // ④ 回归：老写法仍可用（数值 / M 字面量 / P / 盲孔贯通）
        let old = values("螺纹孔 G 公称9.728 P0.907143");
        assert_eq!(old.size_name, "G1/8");
        let old = values("螺纹孔 M10 P1.5");
        assert_eq!(old.size_name, "M10");
        let old = values("螺纹孔 M10 盲孔 H18 L15");
        assert!((old.hole_depth - 18.0).abs() < 1e-9 && (old.thread_len - 15.0).abs() < 1e-9);
        let old = values("螺纹孔 M10 贯通 H20");
        assert!((old.hole_depth - 20.0).abs() < 1e-9 && old.cone_height == 0.0);
        let old = values("螺纹孔 UNC 公称6.35 P1.27");
        assert_eq!(old.size_name, "1/4-20 UNC");
    }

    /// 放置：圆/弧随平移/旋转走。
    #[test]
    fn place_moves_circle_arc_and_lines() {
        let built = build(&thread(10.0, Some(1.5))).unwrap();
        let placed = place(built.entities, [100.0, 50.0], 90.0);
        for e in &placed {
            if let EntityType::Line(l) = e {
                assert!((l.start.x - 100.0).abs() < 20.0);
            }
        }
        let mut m = thread(10.0, Some(1.5));
        m.views = HoleViews { side: true, top: true };
        let placed = place(build(&m).unwrap().entities, [100.0, 50.0], 90.0);
        let c = placed
            .iter()
            .find_map(|e| {
                if let EntityType::Circle(c) = e {
                    Some(c.center)
                } else {
                    None
                }
            })
            .unwrap();
        assert!((c.x - 100.0).abs() < 1e-9);
        assert!((c.y - 80.0).abs() < 1e-9);
    }

    /// 块名稳定且随模型变。
    #[test]
    fn block_name_is_stable_and_model_sensitive() {
        let a = block_name(&thread(10.0, Some(1.5)));
        let b = block_name(&thread(10.0, Some(1.5)));
        let c = block_name(&thread(12.0, Some(1.75)));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.starts_with("OCSM_HOLE_"));
    }

    /// 预览 SVG：浅色板 + 绿色剖面线 + D1/D 文本；贯通也画；四类都可预览。
    #[test]
    fn preview_svg_shows_plate_hatch_and_labels() {
        let svg = preview_svg(&thread(10.0, Some(1.5))).unwrap();
        assert!(svg.contains("ocsmh"), "缺剖面线 pattern");
        assert!(svg.contains("D1=10"), "缺 D1 标注");
        assert!(svg.contains("D=8.376"), "缺 D 标注");
        assert!(svg.contains("fill='#dfe9d2'"), "缺材料板");
        let mut through = thread(10.0, Some(1.5));
        through.range = HoleRange::Through;
        through.hole_depth = Some(25.0);
        assert!(preview_svg(&through).unwrap().contains("D=8.376"));
        let mut top = thread(10.0, Some(1.5));
        top.views = HoleViews { side: false, top: true };
        assert!(preview_svg(&top).is_ok());
        let cb = preview_svg(&plain(HoleKind::Counterbore, HoleSubtype::Drill, 10.0)).unwrap();
        assert!(cb.contains("沉孔Ø18×11"), "沉头预览应标沉孔尺寸");
        let cs = {
            let mut m = plain(HoleKind::Countersink, HoleSubtype::Clearance, 10.0);
            m.fit = "close".to_string();
            preview_svg(&m).unwrap()
        };
        assert!(cs.contains("埋头Ø20.3 90°"), "埋头预览应标 90° 尺寸");
        assert!(cs.contains("Ø10.5"), "不带螺纹时标底孔径");
    }

    /// 非公制螺纹（UN/G/R/NPT/ACME/Tr）的派生：TPI→P、1.5d、L+2P、底孔、表外报错。
    #[test]
    fn non_metric_thread_types_resolve() {
        let _g = zh_guard();
        // UN 1/4-20 UNC：P=1.27、d1=4.976、底孔=5.1054；自动 1.5d 与 L+2P。
        let mut m = HoleModel {
            kind: HoleKind::Threaded,
            subtype: HoleSubtype::Standard,
            thread_system: crate::thread::ThreadSystem::Un,
            thread_group: Some("unc".to_string()),
            d: 6.35,
            pitch: Some(1.27),
            ..Default::default()
        };
        let v = resolve(&m).unwrap();
        assert_eq!(v.size_name, "1/4-20 UNC");
        assert!((v.major - 6.35).abs() < 1e-9);
        assert!((v.minor - 4.976).abs() < 1e-9);
        assert!((v.pitch - 1.27).abs() < 1e-9);
        assert!((v.base_d - 5.1054).abs() < 1e-3, "UN 底孔={}", v.base_d);
        // 自动螺纹长 = 1.5×d，d = 表内大径（mm）；自动孔深 = L + 2P（P=表内 mm）。
        // UN 用户未裁定改口径 → 保持 1.5d。
        assert!((v.thread_len - 1.5 * 6.35).abs() < 1e-9);
        assert!((v.hole_depth - (1.5 * 6.35 + 2.0 * 1.27)).abs() < 1e-9);
        assert_eq!(v.system, "un");
        assert_eq!(v.tpi, Some(20.0));
        assert_eq!(v.gauge_len, None);
        assert_eq!(v.eff_len, None);
        // TPI → P 换算与数据一致
        assert!((v.pitch * v.tpi.unwrap() - 25.4).abs() < 1e-3);

        // G1/8：P=25.4/28、底孔 8.7；管螺纹大径 = 9.728（不是通径）。
        m = HoleModel {
            thread_system: crate::thread::ThreadSystem::G,
            thread_group: Some("standard".to_string()),
            d: 9.728,
            pitch: Some(25.4 / 28.0),
            ..m.clone()
        };
        let v = resolve(&m).unwrap();
        assert_eq!(v.size_name, "G1/8");
        assert!((v.major - 9.728).abs() < 1e-9);
        assert!((v.minor - 8.566).abs() < 1e-9);
        assert!((v.base_d - 8.7).abs() < 1e-9);
        // 管螺纹自动长 = 内螺纹最小有效长度（用户裁定）：G1/8 → 同规格 R1/8 第16栏 7.4（待确认）
        assert!((v.thread_len - 7.4).abs() < 1e-9, "G 自动长={}", v.thread_len);
        assert!((v.hole_depth - (7.4 + 2.0 * 25.4 / 28.0)).abs() < 1e-5);
        assert_eq!(v.eff_len, Some(7.4));
        assert_eq!(v.system, "g");

        // NPT1/2：P=25.4/14、d1=18.321、底孔=17.813（GB/T 12716 末列）、基准长 L1。
        m = HoleModel {
            thread_system: crate::thread::ThreadSystem::Npt,
            thread_group: Some("standard".to_string()),
            d: 21.224,
            pitch: Some(25.4 / 14.0),
            ..m.clone()
        };
        let v = resolve(&m).unwrap();
        assert_eq!(v.size_name, "NPT1/2");
        assert!((v.minor - 18.321).abs() < 1e-9);
        assert!((v.base_d - 17.813).abs() < 1e-3);
        assert!((v.pitch - 25.4 / 14.0).abs() < 1e-5, "p={}", v.pitch);
        assert_eq!(v.tpi, Some(14.0));
        assert!((v.gauge_len.unwrap() - 8.128).abs() < 1e-9);
        // NPT 自动长 = 内螺纹最小有效长度 = 8.128 + 5.443 + 1.814286（偏差取 +1P）
        assert!((v.thread_len - 15.385).abs() < 1e-9, "NPT 自动长={}", v.thread_len);
        assert!((v.hole_depth - (15.385 + 2.0 * 25.4 / 14.0)).abs() < 1e-5);
        assert_eq!(v.eff_len, Some(15.385));

        // R1/8：自动长 = 内螺纹最小有效长度（表第16栏 7.4 = 基准距离最大 4.9 + 装配余量 2.5）
        m = HoleModel {
            thread_system: crate::thread::ThreadSystem::R,
            thread_group: Some("standard".to_string()),
            d: 9.728,
            pitch: Some(25.4 / 28.0),
            ..m.clone()
        };
        let v = resolve(&m).unwrap();
        assert_eq!(v.size_name, "R1/8");
        assert!((v.thread_len - 7.4).abs() < 1e-9, "R 自动长={}", v.thread_len);
        assert!((v.hole_depth - (7.4 + 2.0 * 25.4 / 28.0)).abs() < 1e-5);
        assert_eq!(v.eff_len, Some(7.4), "有效长度 = 第16栏 eff_len");
        assert!((v.gauge_len.unwrap() - 4.0).abs() < 1e-9); // ISO 7-1 表列基准距离基本

        // 手填「螺纹有效长度」优先（管螺纹也尊重手改，不被 eff_len 覆盖）
        m.thread_len = Some(12.0);
        let v = resolve(&m).unwrap();
        assert!((v.thread_len - 12.0).abs() < 1e-9);
        // G 无同规格 R（G5/8）→ 自动长明确报错，不插值
        m.thread_system = crate::thread::ThreadSystem::G;
        m.d = 22.911;
        m.pitch = Some(25.4 / 14.0);
        m.thread_len = None;
        let e = resolve(&m).unwrap_err();
        assert!(e.contains("有效长度") && e.contains("不臆造"), "{e}");

        // ACME 矮牙（参考站口径）：1/4-16 → d1=5.398；Tr8×1.5 → d1=6.5。
        m = HoleModel {
            thread_system: crate::thread::ThreadSystem::Acme,
            thread_group: Some("stub".to_string()),
            d: 6.35,
            pitch: Some(25.4 / 16.0),
            ..m.clone()
        };
        let v = resolve(&m).unwrap();
        assert!((v.minor - 5.398).abs() < 1e-9);
        assert!((v.base_d - 5.398).abs() < 1e-9);
        m = HoleModel {
            thread_system: crate::thread::ThreadSystem::Tr,
            thread_group: Some("standard".to_string()),
            d: 8.0,
            pitch: Some(1.5),
            ..m.clone()
        };
        let v = resolve(&m).unwrap();
        assert_eq!(v.size_name, "Tr8×1.5");
        assert!((v.minor - 6.5).abs() < 1e-9);
        assert!((v.base_d - 6.5).abs() < 1e-9);
        assert_eq!(v.tpi, None);

        // 表外规格明确报错（不插值）
        m.d = 6.0;
        let e = resolve(&m).unwrap_err();
        assert!(e.contains("Tr") && e.contains("表外不插值"), "{e}");
        // 非公制 + 沉头/埋头 → 明确报错（推荐值表只覆盖 M）
        m.kind = HoleKind::Counterbore;
        m.d = 8.0;
        let e = resolve(&m).unwrap_err();
        assert!(e.contains("沉头/埋头") && e.contains("公制 M"), "{e}");
        // 非公制发送「细牙」子类型 → 明确报错
        m.kind = HoleKind::Threaded;
        m.subtype = HoleSubtype::Fine;
        assert!(resolve(&m).is_err());

        // CLI 口径：体系/牙型系列关键字 + `公称 d P`（mm）也能走非公制
        let p = parse_program("螺纹孔 UNC 公称6.35 P1.27").unwrap();
        assert_eq!(p.thread_system, crate::thread::ThreadSystem::Un);
        assert_eq!(p.thread_group.as_deref(), Some("unc"));
        assert!(resolve(&p).is_ok(), "CLI UN 应可解");
        let t = parse_program("螺纹孔 Tr 公称8 P1.5").unwrap();
        assert_eq!(t.thread_system, crate::thread::ThreadSystem::Tr);
        assert!(resolve(&t).is_ok(), "CLI Tr 应可解");
    }

    /// 不同螺纹体系的同尺寸块名不会碰撞。
    #[test]
    fn block_name_includes_thread_system() {
        let m = thread(10.0, Some(1.5));
        let mut u = m.clone();
        u.thread_system = crate::thread::ThreadSystem::Un;
        u.thread_group = Some("unc".to_string());
        assert_ne!(block_name(&m), block_name(&u));
    }

    /// `/api/hole_sizes` 输出四套表 + 全部螺纹体系。
    #[test]
    fn sizes_json_has_all_tables() {
        let v: serde_json::Value = serde_json::from_str(&sizes_json()).unwrap();
        assert!(v["coarse"].as_array().unwrap().len() >= 40);
        assert!(v["fine"].as_array().unwrap().len() >= 100);
        assert_eq!(v["counterbore"]["rows"].as_array().unwrap().len(), 25);
        assert_eq!(v["countersink"]["rows"].as_array().unwrap().len(), 10);
        assert_eq!(v["clearance"]["rows"].as_array().unwrap().len(), 50);
        assert_eq!(v["tap"].as_array().unwrap().len(), 83);
        assert_eq!(v["drill"]["diameters"].as_array().unwrap().len(), 198);
        // 螺纹体系：M/UN/G/R/NPT/ACME/Tr 七套，各行数与本表一致
        let systems = v["systems"].as_array().unwrap();
        assert_eq!(systems.len(), 7);
        assert_eq!(systems[0]["key"], "iso724");
        let find = |key: &str| systems.iter().find(|s| s["key"] == key).unwrap();
        assert_eq!(find("g")["groups"][0]["rows"].as_array().unwrap().len(), 24);
        assert_eq!(find("r")["groups"][0]["rows"].as_array().unwrap().len(), 15);
        assert_eq!(find("npt")["is_pipe"], true);
        // 管螺纹长度列进 /api/hole_sizes：G1/8 eff_len=7.4（同规格 R 第16栏）；NPT1/2 gauge/makeup/eff/eff_len
        let g18 = find("g")["groups"][0]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == "G1/8")
            .unwrap();
        assert_eq!(g18["eff_len"], 7.4);
        let npt12 = find("npt")["groups"][0]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == "NPT1/2")
            .unwrap();
        assert_eq!(npt12["gauge_len"], 8.128);
        assert_eq!(npt12["makeup"], 5.443);
        assert_eq!(npt12["eff_ext"], 13.571);
        assert_eq!(npt12["eff_len"], 15.385);
        // 螺纹有效长度（内、外同一量；自动值）：R1/8 无退刀槽第16栏 = 7.4；NPT 含 +1P 装配公差项
        let r18 = find("r")["groups"][0]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == "R1/8")
            .unwrap();
        assert_eq!(r18["eff_len"], 7.4);
        assert_eq!(npt12["eff_len"], 15.385);
        let un = find("un");
        assert_eq!(un["groups"].as_array().unwrap().len(), 11);
        assert_eq!(un["groups"][0]["rows"].as_array().unwrap().len(), 33);
        let acme = find("acme");
        assert_eq!(acme["groups"].as_array().unwrap().len(), 2);
        assert_eq!(find("tr")["groups"][0]["rows"].as_array().unwrap().len(), 236);
        // M 的 drill 与 resolve 口径一致（M10×1.5 → 8.5）
        let m_group = &systems[0]["groups"][0];
        let m10 = m_group["rows"].as_array().unwrap().iter().find(|r| r["name"] == "M10").unwrap();
        assert_eq!(m10["drill"], 8.5);
    }

    /// ② 批双语验收：孔生成器的报错/提示在 zh/en 下均取到，且关键数据（值/表名/规格名）不丢。
    #[test]
    fn hole_messages_switch_language_with_data_intact() {
        let _g = crate::global_state_test_lock();
        let calls: Vec<(&str, Box<dyn Fn() -> String>, &str, &str)> = vec![
            (
                "table.lookup",
                Box::new(|| parse_program("螺纹孔 M99x1.5").unwrap_err()),
                "M99×1.5",
                "M99×1.5",
            ),
            (
                "value.missing",
                Box::new(|| parse_program("螺纹孔 M10 孔深").unwrap_err()),
                "缺数值",
                "is missing its value",
            ),
            (
                "view.name",
                Box::new(|| parse_program("螺纹孔 M10 view").unwrap_err()),
                "缺视图名",
                "missing a view name",
            ),
            (
                "at.xy",
                Box::new(|| parse_program("螺纹孔 M10 at 1").unwrap_err()),
                "x,y",
                "x,y",
            ),
            (
                "rot.invalid",
                Box::new(|| parse_program("螺纹孔 M10 rot x").unwrap_err()),
                "转角非法",
                "Invalid rotation",
            ),
            (
                "pitch.nonpositive",
                Box::new(|| parse_program("螺纹孔 M10 P0").unwrap_err()),
                "P=0",
                "P=0",
            ),
            (
                "unknown.param",
                Box::new(|| parse_program("螺纹孔 M10 公差X").unwrap_err()),
                "公差",
                "公差",
            ),
            (
                "coarse.missing",
                Box::new(|| {
                    let m = parse_program("螺纹孔 公称99").unwrap();
                    resolve(&m).unwrap_err()
                }),
                "M99",
                "M99",
            ),
            (
                "gb70_2",
                Box::new(|| {
                    let m = parse_program("沉头孔 M10 H10 推荐70.2").unwrap();
                    resolve(&m).unwrap_err()
                }),
                "70.2",
                "70.2",
            ),
            (
                "not.initialized",
                Box::new(|| {
                    ocsm_ready(&ocs_plugin_api::host::acadrust::CadDocument::new()).unwrap_err()
                }),
                "缺图层",
                "missing layers",
            ),
        ];
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        let zh: Vec<String> = calls.iter().map(|(_, f, _, _)| f()).collect();
        crate::i18n::set_lang(crate::i18n::Lang::En);
        let en: Vec<String> = calls.iter().map(|(_, f, _, _)| f()).collect();
        crate::i18n::set_lang_auto();
        for ((name, _, need_zh, need_en), (z, e)) in calls.iter().zip(zh.iter().zip(en.iter())) {
            assert!(z.contains(need_zh), "{name} zh 缺关键数据 {need_zh}：{z}");
            assert!(e.contains(need_en), "{name} en 缺关键数据 {need_en}：{e}");
            assert_ne!(z, e, "{name} 两语应不同：{z}");
        }
        // 警告（工艺余量）与提示（回执）也走 catalog，且保留数值/图层协议名。
        let m = parse_program("螺纹孔 M10 H16 L15").unwrap();
        let warn_zh = crate::i18n::t_fmt_lang(
            crate::i18n::Lang::Zh,
            "cmd.hole.warn.runout_below_2p",
            &[("h", "16"), ("l", "15"), ("need", "18")],
        );
        let warn_en = crate::i18n::t_fmt_lang(
            crate::i18n::Lang::En,
            "cmd.hole.warn.runout_below_2p",
            &[("h", "16"), ("l", "15"), ("need", "18")],
        );
        assert!(warn_zh.contains("2P=") && warn_zh.contains("16"), "{warn_zh}");
        assert!(warn_en.contains("2P=") && warn_en.contains("16"), "{warn_en}");
        assert!(resolve(&m).unwrap().warnings.iter().any(|w| w.contains("工艺余量") || w.contains("runout")), "M10 H16 L15 应有工艺余量警告");
        let echo_zh = crate::i18n::t_fmt_lang(
            crate::i18n::Lang::Zh,
            "cmd.hole.echo.created",
            &[
                ("kind", "螺纹孔"),
                ("size", "M10"),
                ("drill", "8.5"),
                ("depth", "18"),
                ("count", "7"),
                ("x", "0"),
                ("y", "0"),
                ("rot", "0"),
            ],
        );
        let echo_en = crate::i18n::t_fmt_lang(
            crate::i18n::Lang::En,
            "cmd.hole.echo.created",
            &[
                ("kind", "threaded hole"),
                ("size", "M10"),
                ("drill", "8.5"),
                ("depth", "18"),
                ("count", "7"),
                ("x", "0"),
                ("y", "0"),
                ("rot", "0"),
            ],
        );
        assert!(echo_zh.contains("已生成螺纹孔 M10") && echo_zh.contains("1轮廓实线层"), "{echo_zh}");
        assert!(
            echo_en.contains("created threaded hole M10")
                && echo_en.contains("8.5")
                && echo_en.contains("1轮廓实线层"),
            "{echo_en}"
        );
    }

    /// ② 批验收：跑过的产品路径不得留下「没录到」的 key（missing_keys 为空）。
    #[test]
    fn hole_paths_leave_no_missing_catalog_keys() {
        let _g = crate::global_state_test_lock();
        crate::i18n::clear_missing_keys();
        for lang in [crate::i18n::Lang::Zh, crate::i18n::Lang::En] {
            crate::i18n::set_lang(lang);
            let _ = parse_program("螺纹孔 M99x1.5");
            let _ = parse_program("螺纹孔 M10 孔深");
            let _ = parse_program("螺纹孔 M10 公差X");
            let _ = resolve(&parse_program("螺纹孔 公称99").unwrap());
            let _ = ocsm_ready(&ocs_plugin_api::host::acadrust::CadDocument::new());
        }
        crate::i18n::set_lang_auto();
        let missing = crate::i18n::missing_keys();
        assert!(missing.is_empty(), "运行期漏挂 key：{missing:?}");
    }
}
