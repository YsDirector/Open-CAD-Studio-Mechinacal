//! **孔生成器螺纹类型表**：公制 M（ISO 724）/ 美制统一 UN（ASME B1.1）/
//! 管用平行 G（ISO 228-1，BSPP/PF）/ 管用锥形 R（ISO 7-1，BSPT/PT）/
//! 美制锥管 NPT（ASME B1.20.1）/ 美制梯形 ACME（ASME B1.5）/ 公制梯形 Tr（ISO 2901）。
//!
//! 每类一份数据文件（`tables/thread<Type>.json`，与既有 `threadIso724.json` 同样的
//! `source`/`note` + 逐行基本尺寸结构）；**与公制 M 那套分开管理**，M 的几何仍走
//! `hole.rs` 既有 `thread_row()`（不改变既有 M 行为）。
//!
//! ## 单位口径（内部统一 mm）
//! - 所有 `d/d2/d1/drill` 均为 **mm**；1 in = 25.4 mm；
//! - 英制系列存 `tpi`（每英寸牙数）并同时给出 `p`（螺距 mm，`p = 25.4/tpi`）；
//! - 管螺纹（G/R/NPT）的「大径」是**螺纹大径**（G/R 为管子外径；NPT 为内螺纹
//!   基本大径），**不是公称通径**——见各文件 `note`。

use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::i18n::t_fmt;

/// 螺纹体系（孔生成器「标准」下拉）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThreadSystem {
    /// 公制 ISO 724（既有 M 表，行为不变）。
    #[default]
    Iso724,
    /// 美制统一 ASME B1.1（UNC/UNF/UNEF + 定距系列）。
    Un,
    /// 管用平行 ISO 228-1（G / BSPP / PF）。
    G,
    /// 管用锥形 ISO 7-1（R / BSPT / PT）。
    R,
    /// 美制锥管 ASME B1.20.1（NPT）。
    Npt,
    /// 美制梯形 ASME B1.5（ACME / Stub ACME）。
    Acme,
    /// 公制梯形 ISO 2901 / GB/T 5796（Tr）。
    Tr,
}

impl ThreadSystem {
    /// 「标准」下拉顺序。
    pub const ALL: [ThreadSystem; 7] = [
        ThreadSystem::Iso724,
        ThreadSystem::Un,
        ThreadSystem::G,
        ThreadSystem::R,
        ThreadSystem::Npt,
        ThreadSystem::Acme,
        ThreadSystem::Tr,
    ];

    /// 模型/GUI 用的稳定 key（serde 同值）。
    pub fn key(self) -> &'static str {
        match self {
            ThreadSystem::Iso724 => "iso724",
            ThreadSystem::Un => "un",
            ThreadSystem::G => "g",
            ThreadSystem::R => "r",
            ThreadSystem::Npt => "npt",
            ThreadSystem::Acme => "acme",
            ThreadSystem::Tr => "tr",
        }
    }

    /// 规格代号（显示用）。
    pub fn code(self) -> &'static str {
        match self {
            ThreadSystem::Iso724 => "M",
            ThreadSystem::Un => "UN",
            ThreadSystem::G => "G",
            ThreadSystem::R => "R",
            ThreadSystem::Npt => "NPT",
            ThreadSystem::Acme => "ACME",
            ThreadSystem::Tr => "Tr",
        }
    }

    /// 下拉显示名。
    pub fn label(self) -> &'static str {
        match self {
            ThreadSystem::Iso724 => "公制 M（ISO 724 / GB/T 196）",
            ThreadSystem::Un => "美制统一 UN（ASME B1.1 / GB/T 20666~20670）",
            ThreadSystem::G => "管用平行 G（ISO 228-1 / GB/T 7307，BSPP/PF）",
            ThreadSystem::R => "管用锥形 R（ISO 7-1 / GB/T 7306，BSPT/PT）",
            ThreadSystem::Npt => "美制锥管 NPT（ASME B1.20.1 / GB/T 12716）",
            ThreadSystem::Acme => "美制梯形 ACME（ASME B1.5，29°）",
            ThreadSystem::Tr => "公制梯形 Tr（ISO 2901 / GB/T 5796，30°）",
        }
    }

    /// 标准号（来源列）。
    pub fn standard(self) -> &'static str {
        match self {
            ThreadSystem::Iso724 => "ISO 724 / GB/T 196",
            ThreadSystem::Un => "ASME B1.1 / GB/T 20666~20670",
            ThreadSystem::G => "ISO 228-1 / GB/T 7307",
            ThreadSystem::R => "ISO 7-1 / GB/T 7306",
            ThreadSystem::Npt => "ASME B1.20.1 / GB/T 12716",
            ThreadSystem::Acme => "ASME B1.5",
            ThreadSystem::Tr => "ISO 2901 / GB/T 5796",
        }
    }

    /// 牙山角（度）。
    pub fn angle_deg(self) -> f64 {
        match self {
            ThreadSystem::Iso724 | ThreadSystem::Un | ThreadSystem::Npt => 60.0,
            ThreadSystem::G | ThreadSystem::R => 55.0,
            ThreadSystem::Acme => 29.0,
            ThreadSystem::Tr => 30.0,
        }
    }

    /// 是否管螺纹（大径是管子/螺纹外径概念，GUI 里要特别说明）。
    pub fn is_pipe(self) -> bool {
        matches!(self, ThreadSystem::G | ThreadSystem::R | ThreadSystem::Npt)
    }

}

/// 一行螺纹规格（内部统一 mm）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ThreadSpec {
    /// 显示名（如 `#1-64 UNC`、`G1/8`、`Tr8×1.5`、`M10×1.25`）。CLI 也按这个名字查表。
    pub name: String,
    /// 大径 D/d（mm）。
    pub d: f64,
    /// 螺距 P（mm；英制系列由 `25.4/tpi` 换算）。
    pub p: f64,
    /// 每英寸牙数（英制系列；公制为 None）。
    #[serde(default)]
    pub tpi: Option<f64>,
    /// 基本中径 D2/d2（mm）。
    pub d2: f64,
    /// 内螺纹基本小径 D1/d1（mm）。
    pub d1: f64,
    /// 底孔（攻丝钻）直径（mm）；缺省 = 按基本小径 D1 兜底。
    #[serde(default)]
    pub drill: Option<f64>,
    /// 管螺纹**基准距离 L1**（mm；表值）：R = ISO 7-1 表第 8 栏 / NPT = 基准距离 L1；G = None。
    /// ASME B1.20.1 对应 **L1 hand-tight engagement（手拧合）**。
    #[serde(default)]
    pub gauge_len: Option<f64>,
    /// 管螺纹**装配余量**（mm）：R / NPT 表列；G = None。
    /// GB/T 12716 表列名为 **L3**；ASME B1.20.1 对应 **L3 wrench make-up（扳手拧合）**——
    /// **不是“装配距离”**（本仓曾误写，已纠正）。
    #[serde(default)]
    pub makeup: Option<f64>,
    /// 管螺纹**外螺纹最小有效螺纹长度**（mm）= 基准距离 + 装配余量；G = None。
    /// GB/T 12716 表列「外螺纹的有效螺纹不小于（基本）」；ASME B1.20.1 对应 **L2（effective thread external）**。
    #[serde(default)]
    pub eff_ext: Option<f64>,
    /// 管螺纹**螺纹有效长度**（mm；内、外螺纹**同一个量** —— "要旋进多深"）：
    /// - R：内螺纹尾部**无退刀槽**时保证达到 = 表第 16 栏（最大基准距离 + 装配余量）；
    ///   有退刀槽时下限为表第 17 栏×80%（LW3-3-06）；
    /// - NPT：基准距离基本 + 装配余量 + **基准平面位置实际偏差**（标准 LW3-41-01 要求内螺纹确保达到；
    ///   本表偏差档位取 **+1P**，依据同族 NPTF 圆锥螺纹基准平面轴向极限偏差 ±1P，LW3-55）；
    /// - G：ISO 228-1 无此口径 → 取同规格 R 的 `eff_len`（**待用户确认**）；无同规格 R 的 9 档为 None。
    /// 孔深（实际加工长度）= 有效长度 + 工艺余量（`hole.rs` 自动规则）。
    #[serde(default)]
    pub eff_len: Option<f64>,
}

impl ThreadSpec {
    /// 底孔径（无推荐钻径时 = 基本小径 D1，明确不插值）。
    pub fn drill_mm(&self) -> f64 {
        self.drill.unwrap_or(self.d1)
    }
}

/// 一个牙型系列（如 UNC / 细牙 / Stub ACME）。
#[derive(Debug, Clone)]
pub struct ThreadGroup {
    pub key: &'static str,
    pub label: &'static str,
    pub rows: Vec<ThreadSpec>,
}

/// 一类螺纹的全部数据。
#[derive(Debug, Clone)]
pub struct ThreadSystemTable {
    pub source: String,
    pub note: String,
    pub units: String,
    pub groups: Vec<ThreadGroup>,
}

#[derive(Debug, serde::Deserialize)]
struct TableFile {
    source: String,
    #[serde(default)]
    note: String,
    #[serde(default)]
    units: String,
    /// 其余顶层键 = 组名 → 行（与 threadIso724.json 的 coarse/fine 同构）。
    #[serde(flatten)]
    groups: BTreeMap<String, Vec<ThreadSpec>>,
}

/// 组的规范顺序与显示名（数据文件里不一定有顺序；缺组会 panic，防数据腐化）。
fn group_order(sys: ThreadSystem) -> &'static [(&'static str, &'static str)] {
    match sys {
        ThreadSystem::Iso724 => &[("coarse", "粗牙"), ("fine", "细牙")],
        ThreadSystem::Un => &[
            ("unc", "UNC 粗牙"),
            ("unf", "UNF 细牙"),
            ("unef", "UNEF 超细牙"),
            ("un4", "4UN 定距"),
            ("un6", "6UN 定距"),
            ("un8", "8UN 定距"),
            ("un12", "12UN 定距"),
            ("un16", "16UN 定距"),
            ("un20", "20UN 定距"),
            ("un28", "28UN 定距"),
            ("un32", "32UN 定距"),
        ],
        ThreadSystem::Acme => &[("general", "一般用途 ACME"), ("stub", "矮牙 Stub ACME")],
        ThreadSystem::G | ThreadSystem::R | ThreadSystem::Npt | ThreadSystem::Tr => {
            &[("standard", "标准")]
        }
    }
}

fn parse(sys: ThreadSystem, json: &'static str) -> ThreadSystemTable {
    let f: TableFile = serde_json::from_str(json)
        .unwrap_or_else(|e| panic!("{} 表解析失败：{e}", sys.key()));
    let mut groups = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (key, label) in group_order(sys) {
        if let Some(rows) = f.groups.get(*key) {
            seen.insert(*key);
            groups.push(ThreadGroup {
                key,
                label,
                rows: rows.clone(),
            });
        }
    }
    if groups.is_empty() {
        panic!("{} 表没有任何规范组", sys.key());
    }
    let extra: Vec<&str> = f
        .groups
        .keys()
        .map(|s| s.as_str())
        .filter(|k| !seen.contains(k))
        .collect();
    if !extra.is_empty() {
        panic!("{} 表出现未登记的组：{extra:?}", sys.key());
    }
    ThreadSystemTable {
        source: f.source,
        note: f.note,
        units: f.units,
        groups,
    }
}

fn table_for(sys: ThreadSystem) -> &'static ThreadSystemTable {
    macro_rules! static_table {
        ($name:ident, $sys:expr, $file:literal) => {{
            static T: OnceLock<ThreadSystemTable> = OnceLock::new();
            T.get_or_init(|| parse($sys, include_str!($file)))
        }};
    }
    match sys {
        ThreadSystem::Iso724 => static_table!(M, ThreadSystem::Iso724, "tables/threadIso724.json"),
        ThreadSystem::Un => static_table!(U, ThreadSystem::Un, "tables/threadUn.json"),
        ThreadSystem::G => static_table!(G, ThreadSystem::G, "tables/threadG.json"),
        ThreadSystem::R => static_table!(R, ThreadSystem::R, "tables/threadR.json"),
        ThreadSystem::Npt => static_table!(N, ThreadSystem::Npt, "tables/threadNpt.json"),
        ThreadSystem::Acme => static_table!(A, ThreadSystem::Acme, "tables/threadAcme.json"),
        ThreadSystem::Tr => static_table!(T, ThreadSystem::Tr, "tables/threadTr.json"),
    }
}

/// 取某体系的全部数据（OnceLock 缓存）。
pub fn table(sys: ThreadSystem) -> &'static ThreadSystemTable {
    table_for(sys)
}

/// 查规格（纯查表，不插值、不外推）：
/// - `group` 为 None/空 = 在该体系所有组里找；
/// - `d` 为公称/大径（mm）；`pitch` 为螺距（mm，英制由 GUI 换算后传入）；
/// - 同一 `d` 命中多行且未给 `pitch` → 明确报错要求指定子类型/螺距。
pub fn lookup(
    sys: ThreadSystem,
    group: Option<&str>,
    d: f64,
    pitch: Option<f64>,
) -> Result<&'static ThreadSpec, String> {
    let t = table(sys);
    let groups: Vec<&ThreadGroup> = match group.filter(|g| !g.is_empty()) {
        Some(g) => t
            .groups
            .iter()
            .filter(|x| x.key == g)
            .collect(),
        None => t.groups.iter().collect(),
    };
    if groups.is_empty() {
        let keys: Vec<&str> = t.groups.iter().map(|g| g.key).collect();
        return Err(t_fmt(
            "cmd.thread.err.group_missing",
            &[
                ("sys", sys.code()),
                ("group", group.unwrap_or("")),
                ("keys", &keys.join("/")),
            ],
        ));
    }
    let mut hits: Vec<&ThreadSpec> = Vec::new();
    for g in &groups {
        for r in &g.rows {
            if (r.d - d).abs() < 1e-6 && pitch.is_none_or(|p| (r.p - p).abs() < 1e-6) {
                hits.push(r);
            }
        }
    }
    match hits.len() {
        1 => Ok(hits[0]),
        0 => {
            let sample: Vec<String> = groups
                .iter()
                .flat_map(|g| g.rows.iter())
                .take(6)
                .map(|r| r.name.clone())
                .collect();
            let p = pitch
                .map(crate::hole::fmt3)
                .unwrap_or_else(|| crate::i18n::t("cmd.thread.coarse_short"));
            Err(t_fmt(
                "cmd.thread.err.spec_missing",
                &[
                    ("sys", sys.code()),
                    ("d", &crate::hole::fmt3(d)),
                    ("p", &p),
                    ("samples", &sample.join("、")),
                ],
            ))
        }
        _ => {
            let names: Vec<String> = hits.iter().map(|r| r.name.clone()).collect();
            Err(t_fmt(
                "cmd.thread.err.ambiguous",
                &[
                    ("sys", sys.code()),
                    ("d", &crate::hole::fmt3(d)),
                    ("names", &names.join("、")),
                ],
            ))
        }
    }
}

// ── 规格名查表（GUI 下拉名 ↔ CLI 字面量；表 `name` 是唯一事实来源）──────────

/// 一次「按名查表」命中。
#[derive(Debug, Clone, Copy)]
pub struct NameHit {
    /// 命中行所属组 key（UN 的 unc/unf…、ACME 的 general/stub…）。
    /// CLI 需把它回写 `thread_group`，保证 `resolve()` 的数值查表走同一行。
    pub group: &'static str,
    pub spec: &'static ThreadSpec,
}

/// 按名查表的失败原因（具体用户文案由调用方拼，不在这里造）。
#[derive(Debug, Clone)]
pub enum NameError {
    /// 该体系里没有这个名字（含前缀容错后仍无）。
    NotFound,
    /// 归一后同时命中多行（候选显示名）。
    Ambiguous(Vec<String>),
}

/// 规格名比较键：小写、`×`/`X` → `x`、去掉全部空白。
/// 只用于匹配，**不改显示**；匹配容错全部由表 `name` 派生，不另造别名表。
fn name_key(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            '×' | 'X' | 'x' => 'x',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

/// 名字去掉「体系代号前缀」后的键（仅 G/R/NPT/Tr：`G1/8` → `1/8`、`Tr8×1.5` → `8x1.5`）；
/// 这样 CLI 写 `G 1/8` / `NPT 1/2` / `Tr 8x1.5`（体系关键字 + 尺寸）也能命中。
/// M/UN/ACME 不适用：M 的名字必须带 `M`（防裸 `10`）、UN/ACME 的牙型后缀在名字里另处理。
fn bare_key<'a>(sys: ThreadSystem, full: &'a str) -> Option<&'a str> {
    if !matches!(
        sys,
        ThreadSystem::G | ThreadSystem::R | ThreadSystem::Npt | ThreadSystem::Tr
    ) {
        return None;
    }
    let code = sys.code();
    let head = full.get(..code.len())?;
    if head.eq_ignore_ascii_case(code) {
        full.get(code.len()..)
    } else {
        None
    }
}

/// 单行名字是否命中：`exact` 层 = 全等；非 exact 层 = 前缀（允许省略牙型后缀，如
/// `1/4-20` 命 `1/4-20 UNC`）；另对 G/R/NPT/Tr 允许省略体系代号前缀。
fn row_matches(sys: ThreadSystem, name: &str, key: &str, exact: bool) -> bool {
    let full = name_key(name);
    let hit = |k: &str| {
        if exact {
            k == key
        } else {
            k.starts_with(key)
        }
    };
    hit(&full) || bare_key(sys, &full).is_some_and(hit)
}

/// 在**一个体系内**按显示名查规格（GUI 下拉里那个 `name`）：
/// - 归一：大小写 / 空格 / `×`=x 等价；
/// - 先全等、再前缀（前缀层允许 `1/4-20`、`ACME 1/4-16` 这类省略后缀写法）；
/// - 同层多命中时优先 `prefer_group`（当前牙型系列）；仍多则 `NameError::Ambiguous`。
///
/// **表外不插值**：这不是数值解析（数值路径仍是 `hole::parse_program` 的 `公称/P`）。
pub fn lookup_name(
    sys: ThreadSystem,
    prefer_group: Option<&str>,
    text: &str,
) -> Result<NameHit, NameError> {
    let key = name_key(text);
    if key.is_empty() {
        return Err(NameError::NotFound);
    }
    let prefer = prefer_group.filter(|g| !g.is_empty());
    for exact in [true, false] {
        let mut pref: Vec<NameHit> = Vec::new();
        let mut rest: Vec<NameHit> = Vec::new();
        for g in &table(sys).groups {
            for r in &g.rows {
                if row_matches(sys, &r.name, &key, exact) {
                    let hit = NameHit {
                        group: g.key,
                        spec: r,
                    };
                    if prefer == Some(g.key) {
                        pref.push(hit);
                    } else {
                        rest.push(hit);
                    }
                }
            }
        }
        let pool = if pref.is_empty() { rest } else { pref };
        match pool.len() {
            0 => {}
            1 => return Ok(pool[0]),
            _ => {
                return Err(NameError::Ambiguous(
                    pool.iter().map(|h| h.spec.name.clone()).collect(),
                ))
            }
        }
    }
    Err(NameError::NotFound)
}

/// 全库按名检索（跨体系，当前体系除外）：CLI 用它把 `G 1/4-20` 这类
/// **体系与规格不匹配**点名为「这是 UN 的规格」。命中不唯一/歧义的体系跳过。
pub fn find_name_elsewhere(sys: ThreadSystem, text: &str) -> Vec<(ThreadSystem, NameHit)> {
    ThreadSystem::ALL
        .iter()
        .copied()
        .filter(|s| *s != sys)
        .filter_map(|s| lookup_name(s, None, text).ok().map(|h| (s, h)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p_of(r: &ThreadSpec) -> f64 {
        if let Some(tpi) = r.tpi {
            25.4 / tpi
        } else {
            r.p
        }
    }

    /// 逐类规格数（数据文件体量门禁）。
    #[test]
    fn per_type_spec_counts() {
        let m = table(ThreadSystem::Iso724);
        assert_eq!(m.groups[0].rows.len(), 40, "M 粗牙");
        assert_eq!(m.groups[1].rows.len(), 171, "M 细牙");
        let un = table(ThreadSystem::Un);
        let counts: Vec<(&str, usize)> = un.groups.iter().map(|g| (g.key, g.rows.len())).collect();
        assert_eq!(
            counts,
            vec![
                ("unc", 33),
                ("unf", 24),
                ("unef", 25),
                ("un4", 21),
                ("un6", 41),
                ("un8", 48),
                ("un12", 50),
                ("un16", 57),
                ("un20", 29),
                ("un28", 18),
                ("un32", 10),
            ]
        );
        assert_eq!(table(ThreadSystem::G).groups[0].rows.len(), 24, "G");
        assert_eq!(table(ThreadSystem::R).groups[0].rows.len(), 15, "R");
        assert_eq!(table(ThreadSystem::Npt).groups[0].rows.len(), 24, "NPT");
        assert_eq!(table(ThreadSystem::Acme).groups[0].rows.len(), 23, "ACME general");
        assert_eq!(table(ThreadSystem::Acme).groups[1].rows.len(), 23, "ACME stub");
        assert_eq!(table(ThreadSystem::Tr).groups[0].rows.len(), 236, "Tr");
    }

    /// 60° 系列（UN）：D2 = D − 0.649519P、D1 = D − 1.082532P；P = 25.4/TPI。
    #[test]
    fn un_basic_profile_relations() {
        for g in &table(ThreadSystem::Un).groups {
            for r in &g.rows {
                let p = p_of(r);
                let tpi = r.tpi.expect("UN 行应有 TPI");
                assert!((p * tpi - 25.4).abs() < 1e-3, "{} P·TPI≠25.4", r.name);
                assert!(
                    (r.d2 - (r.d - 0.649_519 * p)).abs() < 0.006,
                    "{} D2 关系式不符",
                    r.name
                );
                assert!(
                    (r.d1 - (r.d - 1.082_532 * p)).abs() < 0.006,
                    "{} D1 关系式不符",
                    r.name
                );
                assert!(r.d1 < r.d2 && r.d2 < r.d, "{} 直径序错", r.name);
                // 底孔径：攻丝钻应落在小径附近（D−P 经验式与钻表交叉）
                let drill = r.drill_mm();
                assert!(
                    (drill - (r.d - p)).abs() < 0.4,
                    "{} 底孔 {drill} 偏离 D−P",
                    r.name
                );
                assert!(drill < r.d, "{} 底孔不小于大径", r.name);
            }
        }
        // 用户点名规格的样例（交叉参考站/GB/T 20668）
        let s = lookup(ThreadSystem::Un, Some("unc"), 6.35, Some(1.27)).unwrap();
        assert_eq!(s.name, "1/4-20 UNC");
        assert_eq!((s.d2 * 1000.0).round() / 1000.0, 5.525);
        assert_eq!((s.d1 * 1000.0).round() / 1000.0, 4.976);
        let s = lookup(ThreadSystem::Un, Some("unc"), 1.8542, Some(25.4 / 64.0)).unwrap();
        assert_eq!(s.name, "#1-64 UNC");
    }

    /// 55° 惠氏（G/R）：D2 = D − 0.640327P、D1 = D − 1.280654P。
    #[test]
    fn g_r_whitworth_relations() {
        for sys in [ThreadSystem::G, ThreadSystem::R] {
            for g in &table(sys).groups {
                for r in &g.rows {
                    let p = p_of(r);
                    assert!((p * r.tpi.unwrap() - 25.4).abs() < 1e-3, "{}", r.name);
                    assert!(
                        (r.d2 - (r.d - 0.640_327 * p)).abs() < 0.004,
                        "{} D2",
                        r.name
                    );
                    assert!(
                        (r.d1 - (r.d - 1.280_654 * p)).abs() < 0.004,
                        "{} D1",
                        r.name
                    );
                    assert!(r.d1 < r.d2 && r.d2 < r.d, "{} 直径序错", r.name);
                    // 底孔径：G 有攻丝钻；R 沿用同规格 G。与小径差（参考站钻径取整）应 < 0.7mm
                    assert!(
                        (r.drill_mm() - r.d1).abs() < 0.7,
                        "{} 底孔偏离小径",
                        r.name
                    );
                    assert!(r.drill_mm() < r.d, "{} 底孔不小于大径", r.name);
                }
            }
        }
        let s = lookup(ThreadSystem::G, None, 9.728, None).unwrap();
        assert_eq!(s.name, "G1/8");
        assert_eq!(s.drill, Some(8.7));
        let s = lookup(ThreadSystem::R, None, 9.728, None).unwrap();
        assert_eq!(s.name, "R1/8");
        assert_eq!(s.gauge_len, Some(4.0));
    }

    /// NPT（60° 截顶）：D2 = D − 0.8P、D1 = D − 1.6P；底孔 = GB/T 12716 末列。
    #[test]
    fn npt_basic_profile_relations() {
        for r in &table(ThreadSystem::Npt).groups[0].rows {
            let p = p_of(r);
            assert!((p * r.tpi.unwrap() - 25.4).abs() < 1e-3, "{}", r.name);
            assert!((r.d2 - (r.d - 0.8 * p)).abs() < 0.004, "{} D2", r.name);
            assert!((r.d1 - (r.d - 1.6 * p)).abs() < 0.004, "{} D1", r.name);
            assert!(r.d1 < r.d2 && r.d2 < r.d, "{} 直径序错", r.name);
            assert!(r.drill.is_some(), "{} 缺底孔径", r.name);
            assert!(r.gauge_len.is_some(), "{} 缺基准距离", r.name);
        }
        let s = lookup(ThreadSystem::Npt, None, 21.224, None).unwrap();
        assert_eq!(s.name, "NPT1/2");
        assert_eq!(s.drill, Some(17.813));
        assert_eq!(s.d1, 18.321);
    }

    /// ACME：general = D−0.5P/D−P；stub = D−0.3P/D−0.6P（ASME B1.5）。
    #[test]
    fn acme_relations_general_and_stub() {
        let t = table(ThreadSystem::Acme);
        for (g, k2, k1) in [(&t.groups[0], 0.5, 1.0), (&t.groups[1], 0.3, 0.6)] {
            for r in &g.rows {
                let p = p_of(r);
                assert!(
                    (r.d2 - (r.d - k2 * p)).abs() < 0.002,
                    "{} D2={}",
                    r.name,
                    r.d2
                );
                assert!(
                    (r.d1 - (r.d - k1 * p)).abs() < 0.002,
                    "{} D1={}",
                    r.name,
                    r.d1
                );
                assert!((r.drill_mm() - r.d1).abs() < 1e-6, "{} 底孔=小径", r.name);
            }
        }
        // 参考站 ACME = Stub 尺寸（1/4-16：中径 5.874 / 小径 5.398）
        let stub = lookup(ThreadSystem::Acme, Some("stub"), 6.35, None).unwrap();
        assert_eq!((stub.d2 * 1000.0).round() / 1000.0, 5.874);
        assert_eq!((stub.d1 * 1000.0).round() / 1000.0, 5.398);
        let gen = lookup(ThreadSystem::Acme, Some("general"), 6.35, None).unwrap();
        assert!((gen.d2 - 5.556).abs() < 0.001);
    }

    /// Tr（30°）：D2 = d − 0.5P、D1 = d − P；底孔 = D1。
    #[test]
    fn tr_basic_profile_relations() {
        for r in &table(ThreadSystem::Tr).groups[0].rows {
            assert!(
                (r.d2 - (r.d - 0.5 * r.p)).abs() < 1e-9,
                "{} D2",
                r.name
            );
            assert!((r.d1 - (r.d - r.p)).abs() < 1e-9, "{} D1", r.name);
            assert!((r.drill_mm() - r.d1).abs() < 1e-9, "{} 底孔", r.name);
            assert!(r.tpi.is_none(), "Tr 不应有 TPI");
        }
        let s = lookup(ThreadSystem::Tr, None, 8.0, Some(1.5)).unwrap();
        assert_eq!(s.name, "Tr8×1.5");
        assert_eq!(s.d1, 6.5);
    }

    /// 管螺纹长度列（用户裁定）：自动有效长度 = `eff_len`（内、外螺纹同一个量）；
    /// ASME 编号与 GB 表列交叉核对：gauge_len=L1、makeup=L3（wrench make-up）、eff_ext=L2（L1+L3）。
    #[test]
    fn pipe_length_columns() {
        // NPT：gauge_len = ASME L1（手拧合）；makeup = ASME L3（扳手拧合）；
        // eff_ext = L1+L3（ASME L2）；eff_len = eff_ext + P（装配公差项偏差取 +1P，最大/保守档）。
        for r in &table(ThreadSystem::Npt).groups[0].rows {
            let (g, m) = (r.gauge_len.unwrap(), r.makeup.unwrap());
            let eff = r.eff_ext.unwrap();
            let inner = r.eff_len.unwrap();
            assert!((eff - (g + m)).abs() < 0.002, "{} eff_ext≠L1+L3", r.name);
            assert!(
                (inner - (eff + r.p)).abs() < 0.002,
                "{} eff_len≠eff_ext+P",
                r.name
            );
            assert!(g > 0.0 && m > 0.0 && inner > eff, "{}", r.name);
        }
        let s = lookup(ThreadSystem::Npt, None, 10.242, None).unwrap();
        assert!((s.gauge_len.unwrap() - 4.102).abs() < 1e-9, "NPT1/8 L1");
        assert_eq!(s.makeup, Some(2.822));
        assert_eq!(s.eff_len, Some(7.865), "NPT1/8 = 4.102+2.822+0.940741");
        let s = lookup(ThreadSystem::Npt, None, 41.985, None).unwrap();
        assert_eq!(s.name, "NPT1 1/4");
        assert_eq!(s.drill, Some(37.785), "1 1/4 底孔应=GB/T 12716 末列");
        let s = lookup(ThreadSystem::Npt, None, 48.054, None).unwrap();
        assert_eq!(s.drill, Some(43.853), "1 1/2 底孔应=GB/T 12716 末列");
        let s = lookup(ThreadSystem::Npt, None, 21.224, None).unwrap();
        assert_eq!(s.eff_len, Some(15.385), "NPT1/2 = 8.128+5.443+1.814286");
        // R：gauge_len = 基准距离基本；makeup = 装配余量；eff_ext = 外螺纹基本有效；
        // eff_len = 第16栏（无退刀槽 = 基准距离最大 + 装配余量）
        for r in &table(ThreadSystem::R).groups[0].rows {
            let (g, m) = (r.gauge_len.unwrap(), r.makeup.unwrap());
            let eff = r.eff_ext.unwrap();
            let inner = r.eff_len.unwrap();
            assert!(
                (eff - (g + m)).abs() < 0.11,
                "{} eff_ext≠基准+余量（取整）",
                r.name
            );
            assert!(inner > eff, "{} eff_len（第16栏）应 > eff_ext", r.name);
        }
        let s = lookup(ThreadSystem::R, None, 9.728, None).unwrap();
        assert_eq!(
            (s.gauge_len, s.makeup, s.eff_ext, s.eff_len),
            (Some(4.0), Some(2.5), Some(6.5), Some(7.4))
        );
        let s = lookup(ThreadSystem::R, None, 163.83, None).unwrap();
        assert_eq!(s.eff_len, Some(43.6), "R6 第16栏");
        // G：ISO 228-1 无口径 → eff_len = 同规格 R 的 eff_len（待确认）；无同规格 R 的 9 档为 None
        let g = &table(ThreadSystem::G).groups[0].rows;
        let rmap: std::collections::BTreeMap<&str, f64> = table(ThreadSystem::R).groups[0]
            .rows
            .iter()
            .map(|r| (r.name.as_str(), r.eff_len.unwrap()))
            .collect();
        let mut missing = 0;
        for r in g {
            let key = format!("R{}", &r.name[1..]);
            match rmap.get(key.as_str()) {
                Some(inner) => assert_eq!(
                    r.eff_len.map(|v| (v * 1000.0).round() / 1000.0),
                    Some(*inner),
                    "{}",
                    r.name
                ),
                None => {
                    missing += 1;
                    assert_eq!(r.eff_len, None, "{} 应无 eff_len", r.name);
                }
            }
        }
        assert_eq!(missing, 9, "G 无同规格 R 的档数");
        let s = lookup(ThreadSystem::G, None, 9.728, None).unwrap();
        assert_eq!(s.eff_len, Some(7.4), "G1/8 = R1/8 第16栏");
    }

    /// 表外规格必须明确报错（不插值、不外推）。
    #[test]
    fn out_of_table_reports_error() {
        for (sys, group, d, p) in [
            (ThreadSystem::Un, Some("unc"), 4.0, None),
            (ThreadSystem::Un, Some("unc"), 6.35, Some(9.999)),
            (ThreadSystem::G, None, 10.0, None),
            (ThreadSystem::R, None, 7.0, None),
            (ThreadSystem::Npt, None, 22.0, None),
            (ThreadSystem::Acme, Some("general"), 6.0, None),
            (ThreadSystem::Tr, None, 8.0, Some(1.0)),
        ] {
            let e = lookup(sys, group, d, p).unwrap_err();
            assert!(
                e.contains(sys.code()),
                "{sys:?} {d} 的报错不清：{e}"
            );
        }
        // 未知子类型
        assert!(lookup(ThreadSystem::Un, Some("nope"), 6.35, None).is_err());
        // 同一 d 多行且未给 P → 要求指定子类型/螺距
        assert!(lookup(ThreadSystem::Acme, None, 6.35, None)
            .unwrap_err()
            .contains("多行"));
    }

    /// 按名查表：CLI 常见书写变体全部命中 GUI 下拉同名行（表 `name` 唯一事实来源）。
    #[test]
    fn lookup_name_accepts_cli_variants() {
        let g = |text: &str| lookup_name(ThreadSystem::G, Some("standard"), text).unwrap();
        for text in ["G1/8", "g 1/8", "G 1/8", "1/8"] {
            assert_eq!(g(text).spec.name, "G1/8", "{text}");
        }
        assert_eq!(g("1 1/2").spec.name, "G1 1/2", "体系代号可省");
        let un = |text: &str, group: Option<&str>| {
            lookup_name(ThreadSystem::Un, group, text).unwrap()
        };
        assert_eq!(un("1/4-20", Some("unc")).spec.name, "1/4-20 UNC", "省牙型后缀");
        assert_eq!(un("1/4-20 UNC", Some("unc")).spec.name, "1/4-20 UNC");
        assert_eq!(un("#1-64", Some("unc")).spec.name, "#1-64 UNC");
        assert_eq!(un("1 1/4-12 UNF", None).spec.name, "1 1/4-12 UNF");
        assert_eq!(un("1 1/4-12", None).group, "unf", "跨组前缀命中并归位");
        assert_eq!(un("5/16-24", Some("unc")).group, "unf", "当前组无则全体系找");

        let r = lookup_name(ThreadSystem::R, Some("standard"), "R1/8").unwrap();
        assert_eq!(r.spec.name, "R1/8");
        assert_eq!(
            lookup_name(ThreadSystem::R, Some("standard"), "1/8").unwrap().spec.name,
            "R1/8",
            "R 也可省代号"
        );
        let npt = lookup_name(ThreadSystem::Npt, Some("standard"), "1/2").unwrap();
        assert_eq!(npt.spec.name, "NPT1/2");
        assert_eq!(
            lookup_name(ThreadSystem::Npt, None, "NPT1 1/4").unwrap().spec.name,
            "NPT1 1/4"
        );

        let gen = lookup_name(ThreadSystem::Acme, Some("general"), "1/4-16").unwrap();
        assert_eq!(gen.spec.name, "1/4-16 ACME");
        let stub = lookup_name(ThreadSystem::Acme, Some("stub"), "1/4-16").unwrap();
        assert_eq!(stub.spec.name, "1/4-16 Stub ACME", "同层按当前组优先");
        assert_eq!(stub.spec.d1, 5.398);
        assert_eq!(gen.spec.d1, 4.762);

        for text in ["Tr8×1.5", "Tr8x1.5", "8×1.5", "tr8x1.5"] {
            let h = lookup_name(ThreadSystem::Tr, Some("standard"), text).unwrap();
            assert_eq!(h.spec.name, "Tr8×1.5", "{text}");
        }

        // M（ISO 724）：名字必须带 M（裸 10 不命中，防误收）
        assert_eq!(
            lookup_name(ThreadSystem::Iso724, Some("coarse"), "M10").unwrap().spec.name,
            "M10"
        );
        let fine = lookup_name(ThreadSystem::Iso724, Some("fine"), "M10×1.25").unwrap();
        assert_eq!(fine.spec.name, "M10×1.25");
        assert_eq!(
            lookup_name(ThreadSystem::Iso724, Some("fine"), "M10x1.25").unwrap().spec.name,
            "M10×1.25"
        );
        // M10×1.5 不在 ISO 724 表内（1.5 就是 M10 粗牙螺距）——
        // CLI 的 M 数值分支照旧收，名字查表这里明确不命中（表外不插值）。
        assert!(matches!(
            lookup_name(ThreadSystem::Iso724, None, "M10×1.5"),
            Err(NameError::NotFound)
        ));
        assert!(matches!(
            lookup_name(ThreadSystem::Iso724, None, "10"),
            Err(NameError::NotFound)
        ));
    }

    /// 按名查表：未知 / 歧义 / 跨体系冲突（CLI 报错文案的依据）。
    #[test]
    fn lookup_name_reports_unknown_ambiguous_and_conflict() {
        assert!(matches!(
            lookup_name(ThreadSystem::G, Some("standard"), "9/16"),
            Err(NameError::NotFound)
        ));
        match lookup_name(ThreadSystem::Un, None, "1/4") {
            Err(NameError::Ambiguous(names)) => {
                assert!(names.len() >= 2, "应列出多个候选：{names:?}");
                assert!(names.iter().any(|n| n == "1/4-20 UNC"), "{names:?}");
            }
            other => panic!("1/4 应歧义：{other:?}"),
        }
        // 给当前牙型系列（UNC）则不歧义
        assert_eq!(
            lookup_name(ThreadSystem::Un, Some("unc"), "1/4").unwrap().spec.name,
            "1/4-20 UNC"
        );
        // G 体系没有 1/4-20 → 全库检索点名 UN
        let elsewhere = find_name_elsewhere(ThreadSystem::G, "1/4-20");
        assert_eq!(elsewhere.len(), 1, "{elsewhere:?}");
        assert_eq!(elsewhere[0].0, ThreadSystem::Un);
        assert_eq!(elsewhere[0].1.spec.name, "1/4-20 UNC");
        assert!(find_name_elsewhere(ThreadSystem::G, "9/16").is_empty());
    }

    /// 数据内部一致：所有行 d>0、d1>0、drill>0、名字不重复（同组内）。
    #[test]
    fn tables_are_well_formed() {
        for sys in ThreadSystem::ALL {
            let t = table(sys);
            assert!(!t.source.is_empty(), "{} 缺 source", sys.key());
            assert!(!t.note.is_empty(), "{} 缺 note", sys.key());
            for g in &t.groups {
                let mut names = std::collections::BTreeSet::new();
                for r in &g.rows {
                    assert!(r.d > 0.0 && r.p > 0.0 && r.d1 > 0.0, "{}", r.name);
                    assert!(names.insert(r.name.clone()), "{} 组内重名 {}", sys.key(), r.name);
                }
            }
            assert!(sys.angle_deg() > 0.0);
        }
    }
}
