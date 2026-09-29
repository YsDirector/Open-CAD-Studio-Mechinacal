//! OCSMGEAR —— 参数化圆柱齿轮画法（一期：**外齿轮**）。
//!
//! # 画法唯一权威
//!
//! 用户模板 `~/桌面/OCSM/齿轮/齿轮画法.dxf`（m=2, z=40, ha*=1, c*=0.25, β=0, h=20, Xn=0，
//! 即 d=80 / da=84 / df=75）。本文件的每个构造规则都是从那张图**反解并逐点核对**出来的，
//! 不是查手册臆造的：
//!
//! | 元素 | 规则 | 反解核对 |
//! |---|---|---|
//! | 齿廓 | 渐开线（默认 α=20°，可用 `alpha=` 指定 14.5°/25° 等），用 **clamped 3 次 B 样条** 表示，7 个控制点 | 全 80 条样条对理论渐开线最大偏差 **0.012°** |
//! | 拟合点 | 齿根圆角切点 + `r = rb + (ra−rb)·{¼,½,¾,1}` 共 5 点，**弦长参数化** | 模板 5 个拟合点半径 38.1327/38.6908/39.7939/40.8970/42.0001 |
//! | 齿根过渡圆角 | ρ = **0.38m**（GB/T 1356），圆心在 `r = rf+ρ` 上、且到齿廓起点距离 = ρ | 模板圆心 r=38.2600 = rf+ρ；其弧终点与样条起点同一点 |
//! | 齿顶弧 / 齿根弧 | 齿顶弧在**齿中心线**两侧各半条；齿根弧在**齿槽中心线**两侧各半条 | 每齿 8 个实体，40 齿 = 320 个 |
//! | 分度圆 | 点划线（3中心线层）圆 | 簇3/簇4 都在 3中心线层 |
//! | 剖视图剖面线 | ANSI31、比例 **0.9448818897637796 = 3.0/3.175**（垂距 **3.0mm**，用户定案）、角度 0，**一个 HATCH 两个边界环**（轴线上下各一环，flags=17） | 模板 HATCH offset 分量 ±3.0/√2 |
//! | 轴向倒角 | C = **round(0.6m)**（用户定案 2026-09-17），45°，落在齿顶圆柱两端 | 模板 C=1 = round(0.6×2) |
//! | 中心线 | 长度 = 直径/长度 + **6n**（n=视图比例），与 `OCSMCENTERLINE` 同一口径 | 簇4 十字 = da+6；模板另两处 da+4/h+4 属手绘偏差，统一取 6n |
//! | 螺旋线（斜齿轮） | **侧视图**里三条平行的 2细线层细实线，与中心线夹角 = 90°−β，间距 k×5mm，左旋 `\\` 右旋 `/` | 用户 2026-09-17 口述规则 |
//!
//! # 精度硬约束（用户 2026-09-17）
//!
//! **不提高精度**：模板这套（每齿 8 实体、齿廓 7 控制点）是"兼顾加工与性能"的验证过的
//! 最佳平衡，试过更高精度电脑带不动。本实现保持同一实体数/点数，只把拟合点取在**精确**
//! 渐开线上（模板自身的点上误差约 0.01mm，本实现 ≤3µm，同一量级）。
//!
//! # 范围（齿轮 + 花键）
//!
//! * 齿轮：外齿轮（β≠0 斜齿轮、Xn≠0 变位）+ 内齿轮（齿圈，模板只给剖视/端视）；
//! * **花键模式**（`GearParams.spline = Some(..)`，GUI 顶部复选框）：渐开线花键（GB/T 3478.1 /
//!   DIN 5480 / NF E22-141 / ANSI B92.1），几何全在 `invol_spline.rs` **共用引擎**；本体只负责视图组织 / 块名 / meta。
//!   花键复用同一套「外/内」toggle（外花键/内花键）；内花键与内齿轮同口径：**只有剖视图 + 端视图**，
//!   **无侧视图**（用户定案「内花键剖视图和内齿轮一样，不存在侧视图」，见 `GearParams::spline_notes`）。

use ocs_plugin_api::host::acadrust::entities::{Arc, Circle, Hatch, Line, Spline};
use ocs_plugin_api::host::acadrust::types::{Color, LineWeight, Vector3};
use ocs_plugin_api::host::acadrust::EntityType;

use crate::partgen::{hatch_svg_lines, GenPart, PartMeta, LAYER_CENTER, LAYER_MAIN, LAYER_THIN};
use crate::partgen_kit::{hatch_ansi31_rings, HatchEdge};

/// 剖面线层（与 partgen_more/partgen_kit 同一层名）。
pub const LAYER_HATCH: &str = "5剖面线层";

/// 齿轮/花键剖面线的**垂直距离** = 3.0 mm（用户定案，照模板 `齿轮画法.dxf`）。
///
/// 模板 HATCH 的比例就是 0.9448818897637796 = 3.0/3.175，offset 分量 ±3.0/√2。
pub const HATCH_SPACING_MM: f64 = 3.0;
/// 3.0 mm 垂距对应的 ANSI31 `pattern_scale` = `HATCH_SPACING_MM / partgen_kit::ANSI31_SPACING_MM`。
pub const HATCH_PATTERN_SCALE: f64 = HATCH_SPACING_MM / crate::partgen_kit::ANSI31_SPACING_MM;

/// 默认基准齿形角（GB/T 1356 基本齿廓；`GearParams::alpha_deg` 缺省值）。
pub const ALPHA_N_DEG: f64 = 20.0;
/// 齿根过渡圆角系数 ρf = 0.38m（GB/T 1356；模板实测 0.76 = 0.38×2）。
pub const RHO_RATIO: f64 = 0.38;
/// 轴向倒角系数：C = round(0.6m)（用户 2026-09-17 定案）。
pub const CHAMFER_RATIO: f64 = 0.6;
/// 齿廓样条的 5 个拟合点半径 = rb + (ra−rb)·FIT_FRACTIONS（反解自模板）。
pub const FIT_FRACTIONS: [f64; 5] = [0.125, 0.25, 0.5, 0.75, 1.0];
/// 中心线伸出量系数：长度 = 被标注长度 + 6n（与 OCSMCENTERLINE 同口径）。
pub const CENTER_OVERHANG: f64 = 6.0;

/// 中心线（分度线/轴线/十字线）总长 = 特征长 + [`CENTER_OVERHANG`]×n（两端各 3n）。
///
/// **全口径唯一入口**：外齿轮 / 内齿轮 / 外花键 / 内花键的轴向视图都调它，
/// 禁止各处再写 `… + 6.0 * n` / `… + 3.0 * n`（用户 2026-09-21 定案：内外统一 6n，
/// 原内齿轮模板的 `L+3n`/`L+4n` 作废）。
pub fn centerline_len(feature_len: f64, n: f64) -> f64 {
    feature_len + CENTER_OVERHANG * n
}

/// 斜齿轮侧视图三条细实线的间距 = HELIX_SPACING×k（k = 视图比例）。
pub const HELIX_SPACING: f64 = 5.0;

// ─────────────────────────── 参数 ───────────────────────────

/// 齿轮种类（外齿轮 = 一期；内齿轮 = 二期，用户 2026-09-17 给模板）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GearKind {
    /// 外齿轮：齿顶圆在外、齿根圆在内（da > d > df）
    #[default]
    External,
    /// 内齿轮（齿圈）：齿顶圆在**内**、齿根圆在**外**（da < d < df），齿朝圆心长
    Internal,
}

impl GearKind {
    pub fn is_internal(self) -> bool {
        matches!(self, GearKind::Internal)
    }
    pub fn label(self) -> &'static str {
        match self {
            GearKind::External => "外齿轮",
            GearKind::Internal => "内齿轮",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            GearKind::External => "external",
            GearKind::Internal => "internal",
        }
    }
    /// 解析种类名（命令行/HTTP 共用）。
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "int" | "internal" | "ring" | "内" | "内齿" | "内齿轮" | "内花键" | "齿圈" => {
                Some(GearKind::Internal)
            }
            "ext" | "external" | "外" | "外齿" | "外齿轮" | "外花键" => Some(GearKind::External),
            _ => None,
        }
    }
}

/// 齿轮体系（**显式参数标识**）：普通模数制 / 径节制。
///
/// * `M` = 模数制（输入模数 `m`；默认）；
/// * `DP` = 径节制（输入径节 `DP`，`m = 25.4/DP`）—— 块名/spec 保留 **DP 原值**，
///   避免与同模数的模数制件串块。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GearStd {
    /// 模数制（m）。
    M,
    /// 径节制（DP；`m = 25.4/DP`）。
    DP,
}

impl GearStd {
    /// 短标签（块名/spec/JSON 用）。
    pub fn label(self) -> &'static str {
        match self {
            Self::M => "M",
            Self::DP => "DP",
        }
    }

    /// 英文键（查询串/JSON）。
    pub fn key(self) -> &'static str {
        match self {
            Self::M => "M",
            Self::DP => "DP",
        }
    }

    /// 体系标识解析（大小写/空格不敏感）：`M`/`module`/`模数`/`模数制`、`DP`/`径节`/`径节制`。
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "m" | "module" | "modul" | "模数" | "模数制" => Some(Self::M),
            "dp" | "径节" | "径节制" => Some(Self::DP),
            _ => None,
        }
    }
}

impl Default for GearStd {
    fn default() -> Self {
        Self::M
    }
}

/// 径节基准（1 in = 25.4 mm）：`m = 25.4/DP`。
pub const DP_BASE: f64 = 25.4;

/// 径节 → 模数：`m = 25.4/DP`。
pub fn m_from_dp(dp: f64) -> f64 {
    DP_BASE / dp
}

/// 模数 → 径节：`DP = 25.4/m`（换算显示用）。
pub fn dp_from_m(m: f64) -> f64 {
    DP_BASE / m
}

/// `M` 与 `DP` 同给的统一报错文案（互斥；不静默取其一）。
pub fn m_dp_conflict_msg() -> String {
    crate::i18n::t("cmd.gear.err.m_dp_conflict")
}

/// 齿轮几何参数（GUI / 命令行同一套）。
#[derive(Debug, Clone, PartialEq)]
pub struct GearParams {
    /// 齿轮种类（外/内）——花键模式下复用为**外花键/内花键**开关。
    pub kind: GearKind,
    /// 法向模数 m（GB/T 1357 优先系列；斜齿轮时是 **Mn**）
    pub m: f64,
    /// 齿数 z
    pub z: u32,
    /// 基准齿形角 α（**度**；GB/T 1356 默认 20°，14.5°/25°/30°/37.5°/45° 等系统直接填）
    pub alpha_deg: f64,
    /// 齿顶高系数 ha*
    pub ha: f64,
    /// 顶隙系数 c*（仅齿轮模式；花键模式用 [`SplineOpts::hf_star`]）
    pub c: f64,
    /// 螺旋角 β（**度**，右旋为正、左旋为负；0 = 直齿）
    pub beta_deg: f64,
    /// 齿轮厚度（轴向宽度）h；花键模式下 = 有效长度 L
    pub h: f64,
    /// 变位系数 Xn（花键模式下 DIN 可用；GB 通常 0）
    pub x: f64,
    /// **花键模式**（`Some` = 渐开线花键，`None` = 普通齿轮）。
    /// 齿轮模式不允许标准号/基准直径；花键模式的 m/z/× 以 [`SplineOpts`] 里的 Optional 为准
    /// （齿轮模式的 m/z 保持必给）。
    pub spline: Option<SplineOpts>,
    /// 齿轮体系（`M` 模数制 / `DP` 径节制；花键模式下不用，恒 `M`）。
    pub std: GearStd,
    /// **径节 DP 原值**（`std == DP` 时必给；`m` 字段恒存有效模数 `25.4/DP`）。
    /// 块名/spec/JSON 用这个原值，避免与同模数的模数制件串块。
    pub dp: Option<f64>,
}

/// 花键模式参数（齿轮生成器「花键模式」复选框后面那组输入）。
///
/// * `std`：**显式体系标识** —— GB/T 3478.1-2008 / DIN 5480-1:2015 /
///   NF E22-141（已入库，A 为主参数；兼容旧字段名 `d_b`）/ ANSI B92.1（径节 P，公式驱动）；
/// * `profile`：齿廓代号（空 = 标准默认）；
/// * `d_b`：**基准直径主参数槽位**——DIN 是 `d_B`、NF 是公称直径 `A`（GB/ANSI 给值直接报错）；
/// * `m`/`z`/`x`：`None` = 未给（DIN 可由 `d_B` 查表/推导补全）；
///   **ANSI 下 `m` 槽位收径节 P**（`pitch` 字段可显式替代；入口层 `P5/10` 按 A/B 解析后也进这里），`x` 不允许；
/// * 系数 `alpha_deg`/`ha_star`/`hf_star`/`rho_star`/`c_f_star`：`None` = 用预设，`Some` = 覆盖（ANSI 不允许覆盖）。
#[derive(Debug, Clone, PartialEq)]
pub struct SplineOpts {
    /// 标准体系（GB/T 3478.1 / DIN 5480-1）。
    pub std: crate::invol_spline::SplineStd,
    /// 齿廓代号或中文齿廓名；空 = 标准默认齿廓。
    pub profile: String,
    /// DIN 5480 基准直径 d_B（主参数）；NF 时此槽位收公称直径 A。
    pub d_b: Option<f64>,
    /// 模数 m（`None` = 未给）。
    pub m: Option<f64>,
    /// 齿数 z（`None` = 未给）。
    pub z: Option<u32>,
    /// 变位系数 x（`None` = 0 / 由 d_B 定）。
    pub x: Option<f64>,
    /// 压力角覆盖（None = 预设）。
    pub alpha_deg: Option<f64>,
    /// 齿顶高系数覆盖。
    pub ha_star: Option<f64>,
    /// 齿根高系数覆盖。
    pub hf_star: Option<f64>,
    /// 齿根圆角系数覆盖。
    pub rho_star: Option<f64>,
    /// 齿形裕度系数覆盖。
    pub c_f_star: Option<f64>,
}

impl Default for SplineOpts {
    fn default() -> Self {
        Self {
            std: crate::invol_spline::SplineStd::GB,
            profile: String::new(),
            d_b: None,
            m: None,
            z: None,
            x: None,
            alpha_deg: None,
            ha_star: None,
            hf_star: None,
            rho_star: None,
            c_f_star: None,
        }
    }
}

impl SplineOpts {
    /// 解析花键**体系标识**：`GB` / `GB/T 3478.1` / `DIN` / `DIN 5480` / `NF` / `NF E22-141` /
    /// `ANSI` / `ANSI B92.1`（大小写/空格/斜杠不敏感）。
    /// `NF`/`ANSI` 也返回对应体系 —— 查表/参数错误由 [`crate::invol_spline::resolve_spline`]
    /// 统一报错（不在这里静默掉）。`M`/`DP` 是**齿轮**体系，报错指路。
    pub fn parse_std(s: &str) -> Result<crate::invol_spline::SplineStd, String> {
        use crate::invol_spline::SplineStd;
        let key: String = s
            .trim()
            .to_ascii_uppercase()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        match key.as_str() {
            "M" | "MODULE" | "MODUL" | "DP" => {
                return Err(crate::i18n::t_fmt("cmd.gear.err.std_is_gear_system", &[("s", s)]));
            }
            _ => {}
        }
        if key == "GB" || key.starts_with("GBT3478") {
            return Ok(SplineStd::GB);
        }
        if key == "DIN" || key.starts_with("DIN5480") {
            return Ok(SplineStd::DIN);
        }
        if key == "NF" || key.starts_with("NFE22") {
            return Ok(SplineStd::NF);
        }
        if key == "ANSI" || key.starts_with("ANSIB92") {
            return Ok(SplineStd::ANSI);
        }
        Err(crate::i18n::t_fmt("cmd.gear.err.std_unknown", &[("s", s)]))
    }
}

/// 报错用：齿轮种类显示名（随语言；与 spec/GUI 用的 [`GearKind::label`] 分开）。
pub fn kind_label(kind: GearKind) -> String {
    crate::i18n::t(match kind {
        GearKind::External => "cmd.gear.kind.external",
        GearKind::Internal => "cmd.gear.kind.internal",
    })
}

/// 报错用：视图显示名（随语言；内齿轮剖视带说明）。
pub fn view_label(view: GearView) -> String {
    crate::i18n::t(match view {
        GearView::Section => "cmd.gear.view.section",
        GearView::Front => "cmd.gear.view.front",
        GearView::Side => "cmd.gear.view.side",
        GearView::Simplified => "cmd.gear.view.simplified",
    })
}

impl Default for GearParams {
    fn default() -> Self {
        // 与用户模板/DXF 完全一致的示例件（外齿轮 齿轮画法.dxf：m=2 z=40 h=20）
        Self {
            kind: GearKind::External,
            m: 2.0,
            z: 40,
            alpha_deg: ALPHA_N_DEG,
            ha: 1.0,
            c: 0.25,
            beta_deg: 0.0,
            h: 20.0,
            x: 0.0,
            spline: None,
            std: GearStd::M,
            dp: None,
        }
    }
}

impl GearParams {
    /// 法向基准齿形角 αn（弧度；来自 `alpha_deg`）。
    pub fn alpha_n(&self) -> f64 {
        self.alpha_deg.to_radians()
    }
    /// 螺旋角（弧度，带正负）。
    pub fn beta(&self) -> f64 {
        self.beta_deg.to_radians()
    }
    /// 是否径节制（DP）。
    pub fn is_dp(&self) -> bool {
        self.std == GearStd::DP
    }
    /// 径节原值（DP 体系必给；M 体系为 `None`）。
    pub fn dp_value(&self) -> Option<f64> {
        self.dp
    }
    /// 是否斜齿轮。
    pub fn is_helical(&self) -> bool {
        self.beta_deg.abs() > 1e-9
    }
    /// 端面模数 mt = Mn/cosβ。
    pub fn mt(&self) -> f64 {
        self.m / self.beta().cos()
    }
    /// 端面压力角 αt = atan(tanαn/cosβ)。
    pub fn alpha_t(&self) -> f64 {
        (self.alpha_n().tan() / self.beta().cos()).atan()
    }
    /// 分度圆直径 d = mt·z。
    pub fn d(&self) -> f64 {
        self.mt() * self.z as f64
    }
    /// 齿顶高（不扣齿顶高变动系数 Δy —— 单件图没有配对中心距信息）。
    pub fn ha_height(&self) -> f64 {
        self.m * (self.ha + self.x)
    }
    /// 齿根高。
    pub fn hf_height(&self) -> f64 {
        self.m * (self.ha + self.c - self.x)
    }
    /// 齿顶圆直径 da。
    ///
    /// * 外齿轮：`d + 2ha`（在最外）
    /// * 内齿轮：`d − 2ha`（在**最内** —— 齿朝圆心长，反解自模板：m2 z40 时 da=76）
    pub fn da(&self) -> f64 {
        if self.kind.is_internal() {
            (self.d() - 2.0 * self.ha_height()).max(0.0)
        } else {
            self.d() + 2.0 * self.ha_height()
        }
    }
    /// 齿根圆直径 df。
    ///
    /// * 外齿轮：`d − 2hf`（在内）
    /// * 内齿轮：`d + 2hf`（在**外** —— 模板实测 85 = 80 + 2×1.25×2）
    pub fn df(&self) -> f64 {
        if self.kind.is_internal() {
            self.d() + 2.0 * self.hf_height()
        } else {
            (self.d() - 2.0 * self.hf_height()).max(0.0)
        }
    }
    /// 齿根过渡圆角的**圆心所在半径**。
    ///
    /// * 外齿轮：`rf + ρ`（圆心在齿根圆外，模板实测 38.26 = 37.5 + 0.76）
    /// * 内齿轮：`rf − ρ`（圆心在齿根圆**内**，模板实测 41.74 = 42.5 − 0.76）
    pub fn fillet_center_radius(&self) -> f64 {
        let rf = self.df() / 2.0;
        if self.kind.is_internal() {
            rf - self.rho()
        } else {
            rf + self.rho()
        }
    }
    /// 内齿轮的**齿槽**中心线到该半径处齿廓的夹角（弧度）。
    ///
    /// 关键结论（用户内齿轮模板反解 + 逐点核对）：**内齿轮的齿槽形状 = 同参数外齿轮的齿形** ——
    /// 同一条渐开线、同一个 `ψ(R) = st/(2r) + (inv αt − inv αR)` 公式，
    /// 只是这个角要从**齿槽中心线**量（外齿轮从**齿中心线**量）。
    /// 模板核对：m2 z40 时 R=42.076 处 ψ=0.9869°（理论 0.987°）
    pub fn space_half_angle(&self, radius: f64) -> f64 {
        self.half_tooth_angle(radius)
    }
    /// 内齿轮简化的**齿顶**半角（齿中心线到齿顶弧末端，弧度）。
    ///
    /// 齿槽半角 ψ 大于齿距半角时（齿顶圆低于基圆，z ≲ 33 的常见内齿轮），
    /// 齿廓到基圆就没了、用径向直线收到齿顶圆：此时弧半角按**基圆**处的 ψ 取。
    pub fn internal_tip_half_angle(&self) -> f64 {
        let ra = self.da() / 2.0;
        let rb = self.db() / 2.0;
        let pitch_half = self.pitch_angle() / 2.0;
        (pitch_half - self.space_half_angle(ra.max(rb))).max(1e-6)
    }
    /// 内齿轮的齿廓/直线**有效起点半径**（渐开线能画到哪里）。
    pub fn internal_flank_r_min(&self) -> f64 {
        let ra = self.da() / 2.0;
        let rb = self.db() / 2.0;
        if ra >= rb {
            ra
        } else {
            // 齿顶圆低于基圆：用基圆略上方起画（避开渐开线在基圆处的曲率奇异，同 fcgear 的 fs=0.01）
            rb + 0.02 * (self.df() / 2.0 - rb)
        }
    }
    /// 基圆直径 db = d·cosαt。
    pub fn db(&self) -> f64 {
        self.d() * self.alpha_t().cos()
    }
    /// 端面分度圆齿厚 st = (πMn/2)/cosβ + 2Xn·Mn·tanαn。
    pub fn st(&self) -> f64 {
        std::f64::consts::PI * self.m / 2.0 / self.beta().cos()
            + 2.0 * self.x * self.m * self.alpha_n().tan()
    }
    /// **跨测齿数 k 与公法线长度 W**（直齿外齿轮；`None` = 该组合无此测量口径）。
    ///
    /// 公式：`k = round(z·αt/180° + 0.5)`（αt = 端面压力角，直齿 = αn），
    /// `W = m·cosαt·[(k−0.5)π + z·invαt] + 2·x·m·sinαt`（`invα = tanα − α`）。
    /// 与 `spline_tol` 的 GB/T 3478.6 式(11) `K = z/6 + 0.5`（αD=30°）同一条渐开线跨距公式
    /// （该式的 `z·αD/π + 0.5` 即本式；花键多了 esv − (T+λ) 修正项）。
    ///
    /// 不适用的组合（返回 `None`，卡片如实标缺）：
    /// * **内齿轮** —— 公法线测量用于外齿轮；
    /// * **斜齿轮**（β≠0）—— 公法线要在法面测量，本仓没有斜齿跨距口径。
    pub fn span_measurement(&self) -> Option<(u32, f64)> {
        if self.kind.is_internal() || self.is_helical() {
            return None;
        }
        // 唯一实现：`invol_spline::involute_span`（直齿 αt = αn；ANSI/NF 计算书同调此函数）。
        Some(crate::invol_spline::involute_span(
            self.m,
            self.z,
            self.alpha_t().to_degrees(),
            self.x,
        ))
    }

    /// 齿根过渡圆角半径 ρ = 0.38Mn。
    pub fn rho(&self) -> f64 {
        RHO_RATIO * self.m
    }
    /// 轴向倒角 C = round(0.6m)（取整；小模数可能为 0 = 不倒角）。
    pub fn chamfer(&self) -> f64 {
        (CHAMFER_RATIO * self.m).round()
    }
    /// 齿距角（弧度）。
    pub fn pitch_angle(&self) -> f64 {
        std::f64::consts::TAU / self.z as f64
    }
    /// 齿厚半角 ψ(R)（弧度）：齿中心线（内齿轮时是**齿槽中心线**）到该半径处齿廓的夹角。
    ///
    /// `ψ(R) = st/(2r) + (inv αt − inv αR)`，`αR = acos(rb/R)`；`R < rb` 时按 rb 夹住
    /// （渐开线不存在于基圆以内）。
    pub fn half_tooth_angle(&self, radius: f64) -> f64 {
        let r = self.d() / 2.0;
        let rb = self.db() / 2.0;
        let a_r = (rb / radius.max(rb)).clamp(-1.0, 1.0).acos();
        self.st() / (2.0 * r) + (inv(self.alpha_t()) - inv(a_r))
    }
    /// 参数自检（错误信息直接给用户看）。
    pub fn validate(&self) -> Result<(), String> {
        if !(self.m.is_finite() && self.m > 0.0) {
            return Err(crate::i18n::t("cmd.gear.err.module_positive"));
        }
        if self.is_dp() {
            let dp = self.dp.ok_or_else(|| {
                crate::i18n::t("cmd.gear.err.dp_missing_std")
            })?;
            if !(dp.is_finite() && dp > 0.0) {
                return Err(crate::i18n::t_fmt("cmd.gear.err.dp_positive", &[("v", &trim(dp))]));
            }
            let m = m_from_dp(dp);
            if (self.m - m).abs() > 1e-9 * m.max(1.0) {
                return Err(crate::i18n::t_fmt(
                    "cmd.gear.err.dp_mismatch",
                    &[("dp", &trim(dp)), ("m", &trim(m)), ("cur", &trim(self.m))],
                ));
            }
        } else if self.dp.is_some() {
            return Err(m_dp_conflict_msg());
        }
        if !(2..=1000).contains(&self.z) {
            return Err(crate::i18n::t_fmt("cmd.gear.err.z_range", &[("z", &self.z.to_string())]));
        }
        if !(self.alpha_deg.is_finite() && self.alpha_deg > 10.0 && self.alpha_deg < 50.0) {
            return Err(crate::i18n::t_fmt(
                "cmd.gear.err.alpha_range",
                &[("v", &self.alpha_deg.to_string())],
            ));
        }
        if !(self.ha.is_finite() && self.ha >= 0.5) {
            return Err(crate::i18n::t("cmd.gear.err.ha_min"));
        }
        if !(self.c.is_finite() && self.c >= 0.0) {
            return Err(crate::i18n::t("cmd.gear.err.c_negative"));
        }
        if !(self.beta_deg.is_finite() && self.beta_deg.abs() < 45.0) {
            return Err(crate::i18n::t_fmt("cmd.gear.err.beta_range", &[("v", &self.beta_deg.to_string())]));
        }
        if !(self.h.is_finite() && self.h > 0.0) {
            return Err(crate::i18n::t("cmd.gear.err.h_positive"));
        }
        if !(self.x.is_finite() && self.x.abs() <= 1.0) {
            return Err(crate::i18n::t_fmt("cmd.gear.err.x_range", &[("v", &self.x.to_string())]));
        }
        if self.kind.is_internal() {
            if self.da() <= 1e-6 {
                return Err(crate::i18n::t("cmd.gear.err.internal_da_nonpositive"));
            }
            if self.da() / 2.0 >= self.df() / 2.0 {
                return Err(crate::i18n::t("cmd.gear.err.internal_da_ge_df"));
            }
        } else if self.df() <= 1e-6 {
            return Err(crate::i18n::t("cmd.gear.err.df_nonpositive"));
        }
        if self.chamfer() * 2.0 >= self.h {
            return Err(crate::i18n::t_fmt(
                "cmd.gear.err.chamfer_too_big",
                &[("c", &format!("{:.0}", self.chamfer())), ("h", &format!("{:.1}", self.h))],
            ));
        }
        if self.chamfer() >= self.da() / 2.0 {
            return Err(crate::i18n::t("cmd.gear.err.chamfer_ge_tip"));
        }
        Ok(())
    }
    /// 生成后的提示（不阻断，只提醒）。
    pub fn notes(&self) -> Vec<String> {
        let mut v = Vec::new();
        if self.is_dp() {
            let dp = self.dp.unwrap_or_else(|| dp_from_m(self.m));
            v.push(crate::i18n::t_fmt(
                "cmd.gear.note.dp_spec",
                &[("dp", &trim(dp)), ("m", &trim(self.m)), ("m2", &trim(self.m))],
            ));
        }
        if self.z < 17 && self.x.abs() < 1e-9 {
            v.push(crate::i18n::t_fmt(
                "cmd.gear.note.undercut",
                &[("z", &self.z.to_string())],
            ));
        }
        if self.is_helical() {
            v.push(crate::i18n::t_fmt(
                "cmd.gear.note.helical",
                &[
                    ("beta", &trim(self.beta_deg)),
                    ("hand", &crate::i18n::t(if self.beta_deg > 0.0 {
                        "cmd.gear.dir.right"
                    } else {
                        "cmd.gear.dir.left"
                    })),
                    ("mt", &format!("{:.4}", self.mt())),
                    ("at", &format!("{:.3}", self.alpha_t().to_degrees())),
                ],
            ));
        }
        if self.x.abs() > 1e-9 {
            v.push(crate::i18n::t_fmt(
                "cmd.gear.note.shift",
                &[("x", &trim(self.x))],
            ));
        }
        if (self.alpha_deg - ALPHA_N_DEG).abs() > 1e-9 {
            v.push(crate::i18n::t_fmt(
                "cmd.gear.note.alpha_nonstandard",
                &[
                    ("alpha", &trim(self.alpha_deg)),
                    ("ha", &trim(self.ha)),
                    ("c", &trim(self.c)),
                    ("rho", &trim(self.rho())),
                ],
            ));
        }
        if self.tooth_tip_crossed() {
            v.push(crate::i18n::t_fmt(
                "cmd.gear.note.tip_crossed",
                &[
                    ("alpha", &trim(self.alpha_deg)),
                    ("ha", &trim(self.ha)),
                    ("psi", &format!("{:.4}", self.half_tooth_angle(self.da() / 2.0).to_degrees())),
                ],
            ));
        }
        if self.internal_tooth_crossed() {
            v.push(crate::i18n::t_fmt(
                "cmd.gear.note.internal_space_wide",
                &[
                    ("alpha", &trim(self.alpha_deg)),
                    ("ha", &trim(self.ha)),
                    ("psi", &format!("{:.4}", self.space_half_angle(self.da().max(self.db()) / 2.0).to_degrees())),
                    ("half", &format!("{:.4}", (self.pitch_angle() / 2.0).to_degrees())),
                ],
            ));
        }
        if let Some(w) = self.root_style_note() {
            v.push(w);
        }
        if self.kind.is_internal() && self.z >= 4 {
            // 内齿轮的圆角是“解”出来的（不是模板那套先定起点的构造），解不出来才提示。
            // 注意：不能把外齿轮的 root_style_note 用在内齿轮上 ——
            // 外齿轮口径是“圆心在 rf+ρ”，内齿轮是“rf−ρ”，直接套会**假警报**（已踩）。
            let s0 = self.pitch_angle() / 2.0;
            let ok = solve_internal_fillet(self, s0, 1.0).is_some()
                && solve_internal_fillet(self, s0, -1.0).is_some();
            if !ok {
                v.push(crate::i18n::t_fmt(
                    "cmd.gear.note.internal_fillet_none",
                    &[
                        ("z", &self.z.to_string()),
                        ("m", &trim(self.m)),
                        ("rc", &format!("{:.3}", self.fillet_center_radius())),
                        ("rho", &format!("{:.3}", self.rho())),
                    ],
                ));
            }
        }
        if self.kind.is_internal() {
            v.push(crate::i18n::t("cmd.gear.note.internal_section_template"));
        }
        if self.internal_tip_falls_below_base() {
            v.push(crate::i18n::t_fmt(
                "cmd.gear.note.internal_tip_below_base",
                &[
                    ("z", &self.z.to_string()),
                    ("da", &format!("{:.3}", self.da())),
                    ("db", &format!("{:.3}", self.db())),
                ],
            ));
        }
        if self.is_helical() && self.kind.is_internal() {
            v.push(crate::i18n::t("cmd.gear.note.internal_helical"));
        }
        v
    }
    /// 内齿轮：渐开线能不能画到齿顶圆（齿顶圆是不是低于基圆）。
    ///
    /// 内齿轮齿顶圆 `da = d − 2m`，而基圆 `db = d·cos20°` —— `da < db` ⇔ `z < 2/(1−cos20°) ≈ 33.2`，
    /// 也就是说 **z ≤ 33 的内齿轮都是“齿顶圆低于基圆”**（内齿轮常用 z=24…40，正好落在两侧）。
    /// 此时渐开线下不去，只能到基圆，再按径向直线收到齿顶圆（简化画法，会有提示）。
    pub fn internal_tip_falls_below_base(&self) -> bool {
        self.kind.is_internal() && self.da() / 2.0 < self.db() / 2.0 - 1e-9
    }

    /// 外齿轮：齿顶圆处两条渐开线齿廓是否已经相交（齿顶变尖）—— `ψ(da/2) ≤ 0`。
    ///
    /// 与齿数无关的大 α 判据：`2·ha*·tanα ≳ π/2`（ha*=1 时 α ≳ 38.15°），
    /// 所以 45° 花键必须配小齿顶高系数（如 ha*=0.5）。内齿轮不适用（齿顶在里侧、
    /// 用径向直线降级，`internal_tip_half_angle` 已做钳位）。
    pub fn tooth_tip_crossed(&self) -> bool {
        !self.kind.is_internal() && self.half_tooth_angle(self.da() / 2.0) <= 1e-9
    }

    /// 内齿轮的反向退化：齿槽半角 ≥ 半齿距时，相邻齿槽的齿廓在齿顶圆之前相交。
    /// 与外齿轮的「齿顶变尖」互为镜像（大 α / 短齿距会触发，例如 α=45°、z=40、ha*=1）。
    pub fn internal_tooth_crossed(&self) -> bool {
        self.kind.is_internal()
            && self.space_half_angle(self.da().max(self.db()) / 2.0)
                >= self.pitch_angle() / 2.0 - 1e-9
    }

    /// 齿廓拟合点的**有效起点半径**（模板口径 = rb）：
    /// 基圆落到齿根圆以内时（`rb < rf`，大压力角/小齿数，例如 z=40 时 α>20.36°），
    /// 渐开线在根圆以内没有材料意义，拟合带改从**齿根圆**起算 —— 否则模板的 1/8 起点
    /// 会掉进根圆里、圆角三角形必无解。默认 20° 的模板件 `rb=37.588 > rf=37.5`，
    /// 取 rb、与加 α 参数前逐点一致。
    pub fn flank_band_start(&self) -> f64 {
        let rb = self.db() / 2.0;
        let rf = self.df() / 2.0;
        if rb < rf {
            rf
        } else {
            rb
        }
    }

    /// 齿廓第一个拟合点半径 = 有效起点 + (ra − 有效起点)/8（模板口径的 1/8 规律）。
    pub fn flank_start_radius(&self) -> f64 {
        let r0 = self.flank_band_start();
        r0 + (self.da() / 2.0 - r0) * FIT_FRACTIONS[0]
    }

    /// 齿根那一段用哪种画法（模板口径能不能解出来）。
    ///
    /// **只针对外齿轮**（内齿轮走 `solve_internal_fillet`，它的圆心在 rf−ρ、切点由解唯一确定，
    /// 套用本函数会得出相反的结论 —— 已踩：内齿轮模板件会被报成"圆角无解"）。
    ///
    /// 模板（z=40）的构造要求“圆角圆心在 rf+ρ、且到齿廓起点距离 = ρ”，
    /// 这等价于三角形两边 `r_c = rf+ρ`、`r_s = 齿廓起点半径`、夹角未知——
    /// **可解条件 = |r_c − r_s| ≤ ρ**。齿数小的时候 r_s 比 r_c 高出超过 ρ，就无解了
    /// （用户 2026-09-17 实测 z=17 渲染异常）。
    pub fn root_style(&self) -> RootStyle {
        let (rb, rf) = (self.db() / 2.0, self.df() / 2.0);
        let rho = self.rho();
        let r_c = rf + rho;
        let r_s = self.flank_start_radius();
        if (r_c - r_s).abs() <= rho {
            return RootStyle::TemplateArc;
        }
        // 降级①：渐开线只到基圆，基圆以下留一段径向直线（要求基圆高于圆角圆心）
        if rb > r_c + 1e-9 && (rb - rf) > rho {
            return RootStyle::BaseCircleLine;
        }
        // 降级②
        RootStyle::NoFillet
    }

    /// 齿根降级的提示（要显示给人看；模板口径时返回 None）。**内齿轮不走这条路**。
    pub fn root_style_note(&self) -> Option<String> {
        if self.kind.is_internal() {
            return None;
        }
        let style = self.root_style();
        if style == RootStyle::TemplateArc {
            return None;
        }
        let r_c = self.df() / 2.0 + self.rho();
        let r_s = self.flank_start_radius();
        Some(crate::i18n::t_fmt(
            "cmd.gear.note.root_style_fallback",
            &[
                ("z", &self.z.to_string()),
                ("m", &trim(self.m)),
                ("rs", &format!("{:.3}", r_s.abs())),
                ("rc", &format!("{:.3}", r_c.abs())),
                ("diff", &format!("{:.3}", (r_s - r_c).abs())),
                ("rho", &format!("{:.3}", self.rho())),
                ("style", &crate::i18n::t(match style {
                    RootStyle::TemplateArc => "cmd.gear.rootstyle.template_arc",
                    RootStyle::BaseCircleLine => "cmd.gear.rootstyle.base_circle_line",
                    RootStyle::NoFillet => "cmd.gear.rootstyle.none",
                })),
            ],
        ))
    }

    /// 规格文本（块名/明细表用）。DP 体系保留 **DP 原值**（附换算模数）。
    pub fn spec(&self) -> String {
        let mut s = String::new();
        if self.kind.is_internal() {
            s.push_str(&crate::i18n::t("cmd.gear.spec.internal_prefix"));
        }
        if self.is_dp() {
            let dp = self.dp.unwrap_or_else(|| dp_from_m(self.m));
            s.push_str(&crate::i18n::t_fmt("cmd.gear.spec.dp", &[("dp", &trim(dp)), ("m", &trim(self.m))]));
        } else {
            s.push_str(&format!("m{}", trim(self.m)));
        }
        s.push_str(&format!(" z{} h{}", self.z, trim(self.h)));
        if (self.alpha_deg - ALPHA_N_DEG).abs() > 1e-9 {
            s.push_str(&format!(" α{}", trim(self.alpha_deg)));
        }
        if self.is_helical() {
            s.push_str(&format!(" β{}", trim(self.beta_deg)));
        }
        if self.x.abs() > 1e-9 {
            s.push_str(&format!(" x{}", trim(self.x)));
        }
        s
    }
}

// ─────────────────────── 花键模式（OCSMGEAR「花键」复选框） ───────────────────────

impl GearParams {
    /// 是否花键模式。
    pub fn is_spline(&self) -> bool {
        self.spline.is_some()
    }

    /// 花键模式下 kind 的中文名（外花键/内花键）。
    pub fn spline_kind_label(&self) -> &'static str {
        if self.kind.is_internal() {
            "内花键"
        } else {
            "外花键"
        }
    }

    /// 花键参数 → 引擎 [`crate::invol_spline::InvolParams`]（齿轮模式调用会报错）。
    ///
    /// 齿轮模式给标准号/`d_B` 已在解析层拦；花键模式：`d_B` 交给引擎（只有 DIN 有），
    /// 预设系数可逐个覆盖，最后套内/外花键并整体校验。
    pub fn spline_engine(
        &self,
    ) -> Result<
        (
            crate::invol_spline::InvolParams,
            Option<crate::invol_spline::D_bOrigin>,
        ),
        String,
    > {
        let s = self
            .spline
            .as_ref()
            .ok_or_else(|| crate::i18n::t("cmd.gear.err.spline_mode_required"))?;
        if self.beta_deg.abs() > 1e-9 {
            return Err(crate::i18n::t_fmt(
                "cmd.gear.err.spline_spur_only",
                &[("v", &trim(self.beta_deg))],
            ));
        }
        if !(self.h.is_finite() && self.h > 0.0) {
            return Err(crate::i18n::t("cmd.gear.err.h_spline_positive"));
        }
        let (mut p, origin) = crate::invol_spline::resolve_spline(
            s.std,
            &s.profile,
            s.d_b,
            s.m,
            s.z,
            s.x,
        )?;
        if s.alpha_deg.is_some()
            || s.ha_star.is_some()
            || s.hf_star.is_some()
            || s.rho_star.is_some()
            || s.c_f_star.is_some()
        {
            if s.std == crate::invol_spline::SplineStd::ANSI {
                return Err(
                    "ANSI B92.1：压力角/系数由 Table 2 五列固定（选对应 profile），\
                     不要再覆盖 alpha/ha*/hf*/rho*/cf*。"
                        .to_string(),
                );
            }
            // 压力角不是独立自变量：每个齿廓预设已固定 α（GB 30/37.5/45°、DIN 30°、NF 20°）。
            // 只接受与预设一致的值（等价于没给）；不一致 → 明确拒绝，不静默把几何改成非标准。
            if let Some(a) = s.alpha_deg {
                if (a - p.alpha_deg).abs() > 1e-9 {
                    let why = match s.std {
                        crate::invol_spline::SplineStd::GB => {
                            "压力角由齿廓固定（表 3/图 2：30 平与 30 圆 30°、37.5 圆 37.5°、45 圆 45°）"
                        }
                        crate::invol_spline::SplineStd::DIN => {
                            "只有 30° 齿廓（DIN 5480-1:2015，无齿廓选择）"
                        }
                        crate::invol_spline::SplineStd::NF => {
                            "压力角固定 20°（p07）"
                        }
                        crate::invol_spline::SplineStd::ANSI => unreachable!(),
                    };
                    return Err(format!(
                        "{}：{}；与齿廓「{}」预设 α={}° 不符（收到 α={}°）—— 压力角不是独立参数，不要覆盖。",
                        s.std.code(),
                        why,
                        p.profile,
                        trim(p.alpha_deg),
                        trim(a)
                    ));
                }
            }
            let (a0, ha0, hf0, rho0, cf0) = (
                p.alpha_deg,
                p.ha_star,
                p.hf_star,
                p.rho_star,
                p.c_f_star,
            );
            p = p
                .with_alpha_deg(s.alpha_deg.unwrap_or(a0))
                .with_coeffs(
                    s.ha_star.unwrap_or(ha0),
                    s.hf_star.unwrap_or(hf0),
                    s.rho_star.unwrap_or(rho0),
                    s.c_f_star.unwrap_or(cf0),
                );
        }
        p = p.with_internal(self.kind.is_internal());
        p.validate().map_err(|e| format!("{e}"))?;
        Ok((p, origin))
    }

    /// 花键规格文本（引擎 `spec` + `L` + d_B 来源）。
    pub fn spline_spec(
        &self,
        p: &crate::invol_spline::InvolParams,
        origin: Option<&crate::invol_spline::D_bOrigin>,
    ) -> String {
        let mut s = p.spec();
        s.push_str(&format!(" L{}", trim(self.h)));
        if let Some(o) = origin {
            s.push_str(&format!("（{}）", o.note()));
        }
        s
    }

    /// 花键块名：`OCSM_SPLINE[_INT]_<标准>_<齿廓>_M<m>_Z<z>[_X<x>][_DB<d_B>]_H<h>_<VIEW>`。
    pub fn spline_block_name(
        &self,
        p: &crate::invol_spline::InvolParams,
        view: GearView,
    ) -> String {
        let mut s = String::from("OCSM_SPLINE");
        if self.kind.is_internal() {
            s.push_str("_INT");
        }
        s.push_str(&format!("_{}", p.std.label()));
        if let Some(code) = crate::invol_spline::preset_code(p.std, p.profile) {
            s.push_str(&format!("_{code}"));
        }
        if p.std == crate::invol_spline::SplineStd::ANSI {
            let pv = p.ansi_p();
            s.push_str(&format!(
                "_P{}_PS{}_N{}",
                trim(pv).replace('.', "_"),
                trim(2.0 * pv).replace('.', "_"),
                p.z
            ));
        } else {
            s.push_str(&format!("_M{}_Z{}", trim(p.m).replace('.', "_"), p.z));
        }
        if p.x.abs() > 1e-9 {
            s.push_str(&format!(
                "_X{}{}",
                if p.x < 0.0 { "N" } else { "" },
                trim(p.x.abs()).replace('.', "_")
            ));
        }
        if let Some(a) = p.a {
            s.push_str(&format!("_A{}", trim(a).replace('.', "_")));
        } else if let Some(d_b) = p.d_b {
            s.push_str(&format!("_DB{}", trim(d_b).replace('.', "_")));
        }
        s.push_str(&format!("_H{}", trim(self.h).replace('.', "_")));
        s.push('_');
        s.push_str(&view.key().to_ascii_uppercase());
        s.chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect()
    }

    /// 花键提示（不阻断；内花键无侧视图/齿顶低于基圆/GB 变位等）。
    pub fn spline_notes(&self, p: &crate::invol_spline::InvolParams) -> Vec<String> {
        let mut v = Vec::new();
        if p.internal {
            v.push(
                "内花键：齿朝内、材料在外。**剖视图与内齿轮同口径（齿圈内齿不剖）**：端面/齿顶线/\
                 齿根线/内孔壁/孔口倒角 + 分度线与轴线，**不打剖面线**，齿圈外壁留你按实际结构延伸。\
                 用户定案：「内花键剖视图和内齿轮一样，不存在侧视图」——可用视图只有**剖视图 + 端视图**。"
                    .to_string(),
            );
            if p.internal_tip_radius() < p.db() / 2.0 - 1e-9 {
                v.push(format!(
                    "内花键小径 D_ii={} 低于基圆 db={}：真实齿顶由插齿/拉刀包络（非渐开线），\
                     端视图按「渐开线到基圆 → 径向直线到齿顶」简化。",
                    trim(p.internal_minor_dia()),
                    trim(p.db())
                ));
            }
            let (_, radial_only) = p.internal_involute_band();
            if radial_only {
                if p.internal_root_radius() < p.db() / 2.0 - 1e-9 {
                    v.push(format!(
                        "内花键外侧齿根 D_ei={} 低于基圆 db={}：材料带 [D_ii/2, D_ei/2] 内没有渐开线，\
                         端视图按从齿顶到齿根的径向直线降级（同内齿轮口径）。",
                        trim(p.internal_major_dia()),
                        trim(p.db())
                    ));
                } else {
                    v.push(format!(
                        "内花键 form 直径 DFi={} 不低于外侧齿根 Dri={}（细径节下 cF 夹取所致）：\
                         材料带内没有有效渐开线段，端视图按从齿顶到齿根的径向直线降级。",
                        trim(p.ansi_form_dia_internal()),
                        trim(p.internal_major_dia())
                    ));
                }
            }
        }
        if p.std == crate::invol_spline::SplineStd::GB && p.x.abs() > 1e-9 {
            v.push(format!(
                "GB/T 3478 基本齿廓不含变位；此处为内部构造路径，按 x={} 计入几何（命令行/GUI 入口已明确拒绝 x≠0）。",
                trim(p.x)
            ));
        }
        if p.std == crate::invol_spline::SplineStd::ANSI {
            v.push(format!(
                "ANSI B92.1：公式驱动（径节 P={}、Ps={}；Table 2 五列），节圆 φ{} mm。\
                 标准是英制：引擎内部已按 1 in=25.4 mm 换算（直径/DFe/DFi/cF/p 输出均 mm；\
                 DFi 的 −0.004 in=−0.1016 mm、cF 夹取 0.002~0.010 in=0.0508~0.254 mm）。\
                 rf 标准没有给值（p14 明确圆齿根曲率不能用给定半径规定），当前 ρf={} 是切于齿根的\
                 过渡弧构造、无标准数值依据。",
                trim(p.ansi_p()),
                trim(2.0 * p.ansi_p()),
                trim(p.d()),
                trim(p.rho_f())
            ));
        }
        v
    }
}

/// 渐开线展角函数 inv α = tanα − α。
fn inv(a: f64) -> f64 {
    a.tan() - a
}

/// 数字转文本（去掉多余小数）。
pub fn trim(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        let s = format!("{:.4}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

// ─────────────────────────── 视图 ───────────────────────────

/// 四个视图（一次生成一个）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GearView {
    /// 剖视图（轴向剖切：齿顶圆柱带倒角 + 齿根线 + 上下两环剖面线）
    Section,
    /// 侧视图（轴向投影，不剖；β≠0 时带三条螺旋线细实线）
    Side,
    /// 简化正视图（齿顶圆 + 分度圆 + 齿根圆 + 十字中心线）
    Simplified,
    /// 常规正视图（真实渐开线齿廓 + 分度圆 + 十字中心线）
    Front,
}

impl GearView {
    /// 英文键（命令行 `view=`、块名后缀、SVG 参数）。
    pub fn key(self) -> &'static str {
        match self {
            GearView::Section => "section",
            GearView::Side => "side",
            GearView::Simplified => "simplified",
            GearView::Front => "front",
        }
    }
    /// 中文名（GUI 按钮、报错提示）。
    pub fn label(self) -> &'static str {
        match self {
            GearView::Section => "剖视图",
            GearView::Side => "侧视图",
            GearView::Simplified => "简化正视图",
            GearView::Front => "常规正视图",
        }
    }
    pub const ALL: [GearView; 4] =
        [GearView::Section, GearView::Side, GearView::Simplified, GearView::Front];

    /// 该视图在这个种类的齿轮下能不能生成。
    ///
    /// 内齿轮模板（`内齿轮.dxf`）只给了 **剖视图 + 端视图** 两个视图 ——
    /// 按项目规矩「模板没有的画法不去猜」，其余两个视图在不给模板前不画。
    pub fn available_for(self, kind: GearKind) -> bool {
        match kind {
            GearKind::External => true,
            GearKind::Internal => matches!(self, GearView::Section | GearView::Front),
        }
    }

    /// 人看的名字（内齿轮下 End 视图叫"端视图"更准确）
    pub fn label_for(self, kind: GearKind) -> &'static str {
        match (kind, self) {
            (GearKind::Internal, GearView::Section) => "剖视图（齿圈内齿不剖）",
            (GearKind::Internal, GearView::Front) => "端视图",
            _ => self.label(),
        }
    }

    /// 花键模式可用视图：外花键 = 端视图 / 侧视图 / 剖视图（无简化正视图）；
    /// **内花键 = 剖视图 + 端视图**（与内齿轮同口径，无侧视图/简化正视图）。
    ///
    /// 花键不是齿轮：没有分度圆简化正视图这一说（外花键简化正视图无画法依据，不做）。
    /// 内花键的剖视/端视与内齿轮同模板口径；侧视图明确报错，不出旧的缺模板依据草案。
    pub fn available_for_spline(self, kind: GearKind) -> bool {
        match kind {
            GearKind::External => !matches!(self, GearView::Simplified),
            GearKind::Internal => matches!(self, GearView::Section | GearView::Front),
        }
    }

    /// 花键下的视图名；内花键的 section/front 与内齿轮 [`Self::label_for`] 同名同口径。
    pub fn label_for_spline(self, kind: GearKind) -> &'static str {
        match (kind, self) {
            (GearKind::Internal, GearView::Section) => "剖视图（齿圈内齿不剖）",
            (GearKind::Internal, GearView::Front) => "端视图",
            (_, GearView::Front) => "端视图",
            (_, GearView::Side) => "侧视图",
            (_, GearView::Section) => "剖视图",
            (_, GearView::Simplified) => "简化正视图",
        }
    }

    /// 解析视图名（中英文名都认；**不用数字别名**——那会和位置参数 `<m> <z> <h>` 冲突）。
    pub fn parse(s: &str) -> Result<Self, String> {
        let t = s.trim().to_ascii_lowercase();
        let t = t
            .trim_start_matches("view")
            .trim_start_matches('=')
            .trim_start_matches(':')
            .trim();
        let hit = match t {
            "section" | "cut" | "剖" | "剖视" | "剖视图" | "剖面" => Some(GearView::Section),
            "side" | "axial" | "侧" | "侧视" | "侧视图" => Some(GearView::Side),
            "simplified" | "simple" | "简化" | "简化正视图" | "简化视图" => {
                Some(GearView::Simplified)
            }
            "front" | "regular" | "正视" | "正视图" | "常规" | "常规正视" | "常规正视图" | "端视"
            | "端视图" => Some(GearView::Front),
            _ => None,
        };
        hit.ok_or_else(|| {
            format!(
                "视图名无法识别：`{}`。可用：section|剖视图、side|侧视图、simplified|简化正视图、front|常规正视图。",
                s.trim()
            )
        })
    }
}

/// 内花键无侧视图的统一文案（用户定案：「内花键剖视图和内齿轮一样，不存在侧视图」）。
///
/// `GearView` 校验与引擎 [`crate::invol_spline::InvolParams::side_view`] 共用；语气同内齿轮的
/// 「模板没有的画法不猜」，不静默忽略、不出乱图。
pub fn internal_spline_no_side_view_msg() -> String {
    crate::i18n::t("cmd.gear.err.internal_spline_no_side")
}

// ─────────────────────────── 齿廓（渐开线 + 圆角 + 样条）─────────

/// 齿根那一段怎么画（模板口径 / 小齿数降级）。
///
/// 模板（z=40）的“圆角圆与齿根圆相切、且过齿廓起点”在齿数小时**无解**：
/// 齿廓起点半径与圆角圆心半径差超过 ρ 时三角形不成立。用户 2026-09-17 定案：
/// **别出乱图** —— 给警告 + 降级。降级分两级，都有警告、都写进 `notes()`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootStyle {
    /// 模板口径：0.38m 圆角切齿根圆、且过齿廓起点（大齿数，模板 z=40 就是这样）
    TemplateArc,
    /// 降级①（FreeCAD `fcgear/involute.py` 的做法，用户指定参考）：
    /// 渐开线只到**基圆**，基圆以下用**指向圆心的直线**接到圆角弧顶（圆角保留）
    BaseCircleLine,
    /// 降级②：连圆角都放不下 → **圆角 = 0**（直线直接落到齿根圆）
    NoFillet,
}

impl RootStyle {
    /// 人看的名字（警告/手册用）。
    pub fn label(self) -> &'static str {
        match self {
            RootStyle::TemplateArc => "齿根圆角（模板口径）",
            RootStyle::BaseCircleLine => "基圆以下直线 + 圆角（FreeCAD 口径降级）",
            RootStyle::NoFillet => "无齿根圆角（直线到齿根圆）",
        }
    }
}

/// 齿底那一段（含圆角）的构造结果 —— 局部坐标，齿中心线为 +x 轴。
#[derive(Debug, Clone)]
struct Flank {
    /// 渐开线在 5 个拟合点上的点
    fit: [[f64; 2]; 5],
    /// 7 个 B 样条控制点
    ctrl: Vec<[f64; 2]>,
    /// 11 个节点（弦长参数化，clamped）
    knots: Vec<f64>,
    /// 齿廓起点（= 样条近似曲线的最低端）
    start: [f64; 2],
    /// 齿廓终点（在齿顶圆上）
    end: [f64; 2],
    /// 齿根圆角圆心
    fillet_c: [f64; 2],
    /// 齿根圆角与齿根圆的切点
    root_pt: [f64; 2],
    /// 圆角弧的起/讫角（相对**圆角圆心**，度，CCW）
    fillet_a0: f64,
    fillet_a1: f64,
    /// 采用哪种齿根画法
    style: RootStyle,
    /// 降级时：从基圆（或渐开线起点）到圆角弧顶的那段**径向直线**的两端
    radial: Option<([f64; 2], [f64; 2])>,
}

/// 渐开线上一点：齿中心线角 `c`（弧度），`sign`=+1 取齿廓一侧（逆时针那侧）。
fn involute_point(p: &GearParams, radius: f64, c: f64, sign: f64) -> [f64; 2] {
    let th = c + sign * p.half_tooth_angle(radius);
    [radius * th.cos(), radius * th.sin()]
}

/// B 样条基函数 N_{i,p}(u)（Cox–de Boor，clamped 节点向量）。
fn basis(i: usize, p: usize, u: f64, knots: &[f64]) -> f64 {
    // 右端点 u = u_max 取极限：只有**最后一个控制点**对应的基函数为 1
    // （clamped 节点向量的性质；不特判的话求和会得到 0，采样点会掉到原点）。
    let last = knots.len() - 1;
    if i + p + 1 == last && (u - knots[last]).abs() < 1e-12 {
        return 1.0;
    }
    if p == 0 {
        return if knots[i] <= u && u < knots[i + 1] { 1.0 } else { 0.0 };
    }
    let mut out = 0.0;
    let d1 = knots[i + p] - knots[i];
    if d1 > 0.0 {
        out += (u - knots[i]) / d1 * basis(i, p - 1, u, knots);
    }
    let d2 = knots[i + p + 1] - knots[i + 1];
    if d2 > 0.0 {
        out += (knots[i + p + 1] - u) / d2 * basis(i + 1, p - 1, u, knots);
    }
    out
}

/// 高斯消元（列主元）解 n×n 线性方程组；`b` 会被改写。
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Result<Vec<f64>, String> {
    let n = b.len();
    for c in 0..n {
        let mut piv = c;
        for r in c + 1..n {
            if a[r][c].abs() > a[piv][c].abs() {
                piv = r;
            }
        }
        if a[piv][c].abs() < 1e-12 {
            return Err(crate::i18n::t("cmd.gear.err.fillet_singular"));
        }
        a.swap(c, piv);
        b.swap(c, piv);
        for r in c + 1..n {
            let f = a[r][c] / a[c][c];
            if f == 0.0 {
                continue;
            }
            for k in c..n {
                a[r][k] -= f * a[c][k];
            }
            b[r] -= f * b[c];
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut s = b[i];
        for k in i + 1..n {
            s -= a[i][k] * x[k];
        }
        x[i] = s / a[i][i];
    }
    Ok(x)
}

/// 由 5 个拟合点解出 clamped 3 次 B 样条的 7 个控制点。
///
/// 未知数 d1..d5（d0 = Q0、d6 = Q4 由 clamped 条件固定）。方程：
/// * 3 条插值方程（参数 = 三个内节点）；
/// * 2 条**自然端条件**（两端二阶导为 0）—— 模板样条的端条件无法从 DXF 反解，
///   自然端条件给出的曲线与模板逐点核对差 < 2µm、与精确渐开线差 < 3µm。
fn fit_cubic_bspline(knots: &[f64], fit: &[[f64; 2]; 5]) -> Result<Vec<[f64; 2]>, String> {
    const P: usize = 3;
    let mut a = vec![vec![0.0f64; 5]; 5];
    let (mut bx, mut by) = (vec![0.0; 5], vec![0.0; 5]);
    for (row, k) in (1..=3usize).enumerate() {
        let u = knots[P + k];
        for (col, i) in (1..=5usize).enumerate() {
            a[row][col] = basis(i, P, u, knots);
        }
        let w0 = basis(0, P, u, knots);
        let w6 = basis(6, P, u, knots);
        bx[row] = fit[k][0] - w0 * fit[0][0] - w6 * fit[4][0];
        by[row] = fit[k][1] - w0 * fit[0][1] - w6 * fit[4][1];
    }
    // 自然端条件（clamped 下：C''0 ∝ d0−2d1+d2 = 0，C''1 ∝ d4−2d5+d6 = 0）
    a[3][0] = -2.0;
    a[3][1] = 1.0;
    bx[3] = -fit[0][0];
    by[3] = -fit[0][1];
    a[4][3] = 1.0;
    a[4][4] = -2.0;
    bx[4] = -fit[4][0];
    by[4] = -fit[4][1];
    let xs = solve(a.clone(), bx)?;
    let ys = solve(a, by)?;
    let mut d = Vec::with_capacity(7);
    d.push(fit[0]);
    for i in 0..5 {
        d.push([xs[i], ys[i]]);
    }
    d.push(fit[4]);
    Ok(d)
}

/// 由参数生成一条齿廓（含齿根圆角），`c` = 齿中心线角（弧度），`sign` = ±1 取哪一侧。
fn make_flank(p: &GearParams, c: f64, sign: f64) -> Result<Flank, String> {
    let ra = p.da() / 2.0;
    let rf = p.df() / 2.0;
    let r_band0 = p.flank_band_start();
    let rho = p.rho();
    let r_c = rf + rho; // 圆角圆心半径（与齿根圆相切）
    let style = p.root_style();

    // ① 5 个拟合点。模板口径：渐开线上 r = 有效起点 + (ra−有效起点)·{1/8,¼,½,¾,1}
    //    （反解自模板；有效起点通常 = rb，仅 rb<rf 的大压力角下换成 rf，见 `flank_band_start`）。
    //    降级口径：齿廓只到**基圆**（下面用直线接），所以从 rb 起取 {0,¼,½,¾,1}。
    //    降级口径：齿廓只到**基圆略上方**（避开渐开线在基圆处的曲率奇异 —— 与 FreeCAD
    //    `fcgear` 的 fs=0.01 同一手法：起点正好落在基圆上时，插值样条会在起点附近“勾”回去）。
    let fracs: [f64; 5] = match style {
        RootStyle::TemplateArc => FIT_FRACTIONS,
        _ => [0.02, 0.25, 0.5, 0.75, 1.0],
    };
    let mut fit = [[0.0f64; 2]; 5];
    for (i, f) in fracs.iter().enumerate() {
        let radius = r_band0 + (ra - r_band0) * f;
        fit[i] = involute_point(p, radius, c, sign);
    }
    // ② 弦长参数化 → clamped 节点（**11 个** = 7 控制点 + 3 次 + 1，与模板同构）+ ③ 控制点
    let (knots, ctrl) = fit_cubic_bspline_knots(&fit)?;

    // ④ 齿根那一段（三种画法，见 `RootStyle`）
    let start = fit[0];
    let th_start = start[1].atan2(start[0]);
    let mut radial: Option<([f64; 2], [f64; 2])> = None;
    let mut arc_top = start; // 圆角弧的另一端（降级时是弧顶）
    let (fillet_c, root_pt) = match style {
        RootStyle::TemplateArc => {
            // 圆心在 r = rf+ρ 上、且 |圆心 − 齿廓起点| = ρ（余弦定理）
            let rs = (start[0] * start[0] + start[1] * start[1]).sqrt();
            let cosd = ((r_c * r_c + rs * rs - rho * rho) / (2.0 * r_c * rs)).clamp(-1.0, 1.0);
            let th_c = th_start + sign * cosd.acos();
            (
                [r_c * th_c.cos(), r_c * th_c.sin()],
                [rf * th_c.cos(), rf * th_c.sin()],
            )
        }
        RootStyle::BaseCircleLine => {
            // FreeCAD `fcgear` 口径：弧顶在齿廓起点**同一半径线**上（径向直线），
            // 圆心角 δ 由“弧顶落在该半径线上”反解：δ = 2·asin(ρ/(2·Rci))
            let dth = 2.0 * (rho / (2.0 * r_c)).min(1.0).asin();
            let th_c = th_start + sign * dth;
            let top = [r_c * th_start.cos(), r_c * th_start.sin()];
            radial = Some((start, top));
            arc_top = top;
            (
                [r_c * th_c.cos(), r_c * th_c.sin()],
                [rf * th_c.cos(), rf * th_c.sin()],
            )
        }
        RootStyle::NoFillet => {
            // 圆角 = 0：直线直接落到齿根圆
            let foot = [rf * th_start.cos(), rf * th_start.sin()];
            radial = Some((start, foot));
            (foot, foot)
        }
    };
    // root_pt / arc_top 相对**齿轮中心**的极角（用于判定圆角弧取哪一支）
    let th_root = root_pt[1].atan2(root_pt[0]);
    let th_top = arc_top[1].atan2(arc_top[0]);
    // 圆角弧的起/讫角（相对圆角圆心，度）—— NoFillet 时弧退化，调用方不画
    let ang = |q: [f64; 2]| {
        let v = (q[0] - fillet_c[0], q[1] - fillet_c[1]);
        v.1.atan2(v.0).to_degrees()
    };
    let (mut a0, mut a1) = (ang(root_pt), ang(arc_top));
    if style != RootStyle::NoFillet && (a1 - a0).rem_euclid(360.0) > 180.0 {
        // 取**短弧**那一支：圆角弧两种画法都 ≲90°（模板 80°、降级①约 90°），
        // 短弧判定唯一正确；早先“按弧中点是否落在两端之间”的写法会把 270° 那支选进来，
        // 采样链会先鼓出去再缩回来（z=7/17 单调性测试就挂在这里）。
        std::mem::swap(&mut a0, &mut a1);
    }
    Ok(Flank {
        fit,
        ctrl,
        knots,
        start,
        end: fit[4],
        fillet_c,
        root_pt,
        fillet_a0: a0,
        fillet_a1: a1,
        style,
        radial,
    })
}

/// 一条齿根段的采样点（**从齿根端到齿顶**，用于质量估算与闭合性自检）。
fn flank_points(p: &GearParams, f: &Flank, seg: usize) -> Vec<[f64; 2]> {
    let rho = p.rho();
    let mut out = Vec::with_capacity(3 * seg + 4);
    // 圆角弧：root_pt → start（若 NoFillet 则跳过，由直线承担）
    if f.style != RootStyle::NoFillet {
        let mut seg_pts = Vec::with_capacity(seg + 1);
        // DXF 圆弧是“从 start 逆时针扫到 end”：a1 可能比 a0 小（例如 a0=97.9°、a1=−176.2°），
        // 直接线性插值会反着绕过 0°（把圆角的“肚子”也扫进来）→ 必须先解出 CCW 扫角。
        let sweep = (f.fillet_a1 - f.fillet_a0).rem_euclid(360.0);
        for k in 0..=seg {
            let a = (f.fillet_a0 + sweep * k as f64 / seg as f64).to_radians();
            seg_pts.push([f.fillet_c[0] + rho * a.cos(), f.fillet_c[1] + rho * a.sin()]);
        }
        // 弧实体按 CCW(a0→a1) 存（渲染口径），但采样链必须**从齿根端往齿顶走**：
        // 若首点比末点更靠近圆心，就把这一段反过来。
        let r = |q: [f64; 2]| (q[0] * q[0] + q[1] * q[1]).sqrt();
        if r(seg_pts[0]) > r(seg_pts[seg]) {
            seg_pts.reverse();
        }
        out.extend(seg_pts);
    }
    // 径向直线（降级时）：start → 弧顶（与上一段相接）
    if let Some((from, to)) = f.radial {
        // 链方向是“齿根→齿顶”：本段要从弧顶/齿根脚走到渐开线起点
        let (a, b) = (to, from);
        for k in 0..=seg {
            let t = k as f64 / seg as f64;
            out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
    }
    // 渐开线样条：按真实曲线采样
    let (t0, t1) = (f.knots[3], f.knots[7]);
    for k in 0..=3 * seg {
        let u = t0 + (t1 - t0) * k as f64 / (3 * seg) as f64;
        let mut q = [0.0f64; 2];
        for (j, d) in f.ctrl.iter().enumerate() {
            let w = basis(j, 3, u, &f.knots);
            q[0] += w * d[0];
            q[1] += w * d[1];
        }
        out.push(q);
    }
    out
}

/// 由 5 个拟合点造 clamped 三次 B 样条：弦长参数化出 11 个节点，再解 7 个控制点。
///
/// 外齿轮（`make_flank`）与内齿轮（`make_flank_internal`）共用 —— 两边的实体结构完全同构，
/// 差别只在拟合点取自哪条曲线段。
fn fit_cubic_bspline_knots(fit: &[[f64; 2]; 5]) -> Result<(Vec<f64>, Vec<[f64; 2]>), String> {
    let mut knots = vec![0.0, 0.0, 0.0, 0.0];
    let mut acc = 0.0;
    for i in 0..4 {
        acc += ((fit[i + 1][0] - fit[i][0]).powi(2) + (fit[i + 1][1] - fit[i][1]).powi(2)).sqrt();
        if i < 3 {
            knots.push(acc);
        }
    }
    knots.extend_from_slice(&[acc; 4]);
    debug_assert_eq!(knots.len(), 11);
    let ctrl = fit_cubic_bspline(&knots, fit)?;
    Ok((knots, ctrl))
}

/// 一条齿廓 B 样条实体（外/内齿轮共用）。
fn spline_entity(knots: &[f64], ctrl: &[[f64; 2]]) -> EntityType {
    let mut sp = Spline::new();
    sp.degree = 3;
    sp.flags.planar = true;
    sp.flags.rational = true; // 与模板一致：标 rational + 权重全 1（等价非有理）
    sp.knots = knots.to_vec();
    sp.control_points = ctrl.iter().map(|q| Vector3::new(q[0], q[1], 0.0)).collect();
    sp.weights = vec![1.0; ctrl.len()];
    sp.normal = Vector3::new(0.0, 0.0, 1.0);
    set_layer(&mut sp, LAYER_MAIN);
    EntityType::Spline(sp)
}

/// 齿根段的实体：圆角弧（模板/降级①）+ 降级时的**径向直线**；NoFillet 时只有直线。
fn push_root_seg(out: &mut Vec<EntityType>, p: &GearParams, f: &Flank) {
    if f.style != RootStyle::NoFillet {
        out.push(arc_deg(f.fillet_c, p.rho(), f.fillet_a0, f.fillet_a1, LAYER_MAIN));
    }
    if let Some((from, to)) = f.radial {
        out.push(line(from, to, LAYER_MAIN));
    }
}

/// 一条齿廓对应的 B 样条实体。
fn spline_of(f: &Flank) -> EntityType {
    spline_entity(&f.knots, &f.ctrl)
}

// ─────────────────────────── 实体助手 ───────────────────────────

fn line(a: [f64; 2], b: [f64; 2], layer: &str) -> EntityType {
    let mut l = Line::from_points(Vector3::new(a[0], a[1], 0.0), Vector3::new(b[0], b[1], 0.0));
    set_layer(&mut l, layer);
    EntityType::Line(l)
}

fn arc_deg(c: [f64; 2], r: f64, a0: f64, a1: f64, layer: &str) -> EntityType {
    let mut a = Arc::from_center_radius_angles(
        Vector3::new(c[0], c[1], 0.0),
        r,
        a0.to_radians(),
        a1.to_radians(),
    );
    set_layer(&mut a, layer);
    EntityType::Arc(a)
}

/// `arc_deg` 的 DXF 语义是 start→end **逆时针**扫：大压力角下齿根切点会越过
/// 齿槽中心线（两个代表角的先后反过来），直接画会绕一整圈。取短弧（口径同
/// `make_flank_internal` 的“短弧那一支”），端点仍是同两个点。
/// 默认 20° 下跨度本来就小，等价于直接 `arc_deg`（行为不变）。
fn short_arc_deg(c: [f64; 2], r: f64, a0: f64, a1: f64, layer: &str) -> EntityType {
    if (a1 - a0).rem_euclid(360.0) > 180.0 {
        arc_deg(c, r, a1, a0, layer)
    } else {
        arc_deg(c, r, a0, a1, layer)
    }
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

/// 让 set_layer 适配 Line/Arc/Circle/Spline/Hatch。
trait CommonLayer {
    fn common_mut(&mut self) -> &mut ocs_plugin_api::host::acadrust::entities::EntityCommon;
}
macro_rules! impl_common_layer {
    ($($t:ty),*) => {$(impl CommonLayer for $t {
        fn common_mut(&mut self) -> &mut ocs_plugin_api::host::acadrust::entities::EntityCommon { &mut self.common }
    })*};
}
impl_common_layer!(
    Line,
    Arc,
    Circle,
    Spline,
    Hatch,
    ocs_plugin_api::host::acadrust::entities::Insert
);

/// 十字中心线（长度 = 直径 + 6n，与 OCSMCENTERLINE 同规则）。
pub(crate) fn cross_centerlines(center: [f64; 2], dia: f64, n: f64) -> Vec<EntityType> {
    let half = centerline_len(dia, n) / 2.0;
    vec![
        line([center[0] - half, center[1]], [center[0] + half, center[1]], LAYER_CENTER),
        line([center[0], center[1] - half], [center[0], center[1] + half], LAYER_CENTER),
    ]
}

// ─────────────────────────── 四个视图 ───────────────────────────

/// 常规正视图：真实渐开线齿廓（每齿 8 个实体，与模板同构）。
///
/// 齿中心线在 `pitch/2 + k·pitch`（模板实测：齿中心 4.5°+9k，齿槽中心 0°+9k）。
fn front_regular(p: &GearParams, n: f64) -> Result<Vec<EntityType>, String> {
    if p.tooth_tip_crossed() {
        return Err(format!(
            "α={}° 配 ha*={} 时齿顶变尖（ψ(da/2)={:.4}° ≤ 0）：两条渐开线在齿顶圆之前相交，\
             常规正视图画不出真实齿廓。请减小 ha*（45° 花键常用 ha*=0.5；本工具下限 0.5）、减小 α 或增大齿数；\
             剖视图/侧视图/简化正视图不受影响。",
            trim(p.alpha_deg),
            trim(p.ha),
            p.half_tooth_angle(p.da() / 2.0).to_degrees()
        ));
    }
    let ra = p.da() / 2.0;
    let rf = p.df() / 2.0;
    let pitch = p.pitch_angle();
    // 齿廓在齿中心线两侧各一条（+1 侧 = 逆时针那侧）
    let mut out = Vec::with_capacity(p.z as usize * 8 + 3);
    let c_first = pitch / 2.0; // 模板里的“齿中心在半个齿距处”
    for t in 0..p.z as usize {
        let c = c_first + pitch * t as f64;
        let c_deg = c.to_degrees();
        let fl = make_flank(p, c, 1.0)?; // 该齿 +1 侧
        let fr = make_flank(p, c, -1.0)?; // 该齿 −1 侧
        // 齿根弧（前一个齿槽的半条）：从齿槽中心到本齿 −1 侧圆角切点
        out.push(short_arc_deg(
            [0.0, 0.0],
            rf,
            c_deg - pitch.to_degrees() / 2.0,
            near(th_of(fr.root_pt), c_deg - 3.0),
            LAYER_MAIN,
        ));
        // 齿根段 + 齿廓（−1 侧）：齿根端 → 弧/直线 → 渐开线
        push_root_seg(&mut out, p, &fr);
        out.push(spline_of(&fr));
        // 齿顶弧：以齿中心线为界各半条（模板同构）
        let psi_end = p.half_tooth_angle(ra).to_degrees();
        out.push(arc_deg([0.0, 0.0], ra, c_deg - psi_end, c_deg, LAYER_MAIN));
        out.push(arc_deg([0.0, 0.0], ra, c_deg, c_deg + psi_end, LAYER_MAIN));
        // 齿廓 + 齿根段（+1 侧）：渐开线 → 直线/弧 → 齿根端
        out.push(spline_of(&fl));
        push_root_seg(&mut out, p, &fl);
        // 齿根弧（本齿槽的另半条）：从 +1 侧圆角切点到下一个齿槽中心
        // 最后一齿要收口到 360°（= 第一齿齿槽中心 0°，闭合链条）
        let gap_hi = if t + 1 == p.z as usize {
            360.0
        } else {
            c_deg + pitch.to_degrees() / 2.0
        };
        out.push(short_arc_deg(
            [0.0, 0.0],
            rf,
            near(th_of(fl.root_pt), c_deg + 3.0),
            gap_hi,
            LAYER_MAIN,
        ));
    }
    // 分度圆（点划线）+ 十字中心线
    out.push(circle([0.0, 0.0], p.d() / 2.0, LAYER_CENTER));
    out.extend(cross_centerlines([0.0, 0.0], p.da(), n));
    Ok(out)
}

/// 极角（度，规范到 0..360）。
fn th_of(q: [f64; 2]) -> f64 {
    let d = q[1].atan2(q[0]).to_degrees();
    if d < 0.0 {
        d + 360.0
    } else {
        d
    }
}

/// 把角度平移到离 `reference` 最近的那个周数（避免 ±180° 回绕：齿中心 4.5° 的
/// 侧根切点算出来 355.85°，不平移的话圆弧会绕一大圈）。
fn near(angle_deg: f64, reference_deg: f64) -> f64 {
    let mut a = angle_deg;
    while a - reference_deg > 180.0 {
        a -= 360.0;
    }
    while reference_deg - a > 180.0 {
        a += 360.0;
    }
    a
}

/// 简化正视图：齿顶圆（粗实线）+ 分度圆（点划线）+ 齿根圆（细实线）+ 十字中心线。
fn front_simplified(p: &GearParams, n: f64) -> Result<Vec<EntityType>, String> {
    let mut out = vec![
        circle([0.0, 0.0], p.da() / 2.0, LAYER_MAIN),
        circle([0.0, 0.0], p.d() / 2.0, LAYER_CENTER),
    ];
    if p.df() > 1e-9 {
        out.push(circle([0.0, 0.0], p.df() / 2.0, LAYER_THIN));
    }
    out.extend(cross_centerlines([0.0, 0.0], p.da(), n));
    Ok(out)
}

/// 轴向视图（侧视/剖视共用）的轮廓：齿顶圆柱（h×da）四角倒角 C。
///
/// 与模板同构：端面线画到 ±(ra−C)、齿顶面画到 ±(h/2−C)。
/// **`steps` 区别两个视图**（反解自模板）：
/// * 侧视图有两条倒角台阶线（x=±(h/2−C)，从 −ra 到 +ra）——模板实测 10 条轮廓线；
/// * 剖视图**没有**这两条（模板实测 18 条里就是它们不在，全剖时该棱边按惯例省略）。
fn axial_outline(p: &GearParams, steps: bool) -> Vec<EntityType> {
    let ra = p.da() / 2.0;
    let hh = p.h / 2.0;
    let c = p.chamfer();
    let mut out = Vec::new();
    let hi = ra - c; // 倒角后齿顶面的一半长度
    let hc = hh - c; // 倒角后端面的一半长度
    // 端面线（±(ra−C) 高）
    out.push(line([-hh, -hi], [-hh, hi], LAYER_MAIN));
    out.push(line([hh, -hi], [hh, hi], LAYER_MAIN));
    // 倒角台阶线（±ra 高）—— 只有侧视图画
    if steps {
        out.push(line([-hc, -ra], [-hc, ra], LAYER_MAIN));
        out.push(line([hc, -ra], [hc, ra], LAYER_MAIN));
    }
    // 齿顶面（±(h/2−C) 长）
    out.push(line([-hc, ra], [hc, ra], LAYER_MAIN));
    out.push(line([-hc, -ra], [hc, -ra], LAYER_MAIN));
    // 四个 45° 倒角
    out.push(line([-hh, hi], [-hc, ra], LAYER_MAIN));
    out.push(line([hc, ra], [hh, hi], LAYER_MAIN));
    out.push(line([-hh, -hi], [-hc, -ra], LAYER_MAIN));
    out.push(line([hc, -ra], [hh, -hi], LAYER_MAIN));
    out
}

/// 侧视图：轴向投影（不剖）。β≠0 时加三条螺旋线细实线。
pub(crate) fn side_view(p: &GearParams, n: f64) -> Result<Vec<EntityType>, String> {
    let mut out = axial_outline(p, true);
    // 分度线（点划线）+ 轴线（中心线）
    let d2 = p.d() / 2.0;
    let cl = centerline_len(p.h, n);
    out.push(line([-cl / 2.0, d2], [cl / 2.0, d2], LAYER_CENTER));
    out.push(line([-cl / 2.0, -d2], [cl / 2.0, -d2], LAYER_CENTER));
    out.push(line([-cl / 2.0, 0.0], [cl / 2.0, 0.0], LAYER_CENTER));
    // 斜齿轮：三条平行细实线（与中心线夹角 = 90°−|β|；左旋 "\"、右旋 "/"）
    if p.is_helical() {
        let ra = p.da() / 2.0;
        let phi = (90.0 - p.beta_deg.abs()).to_radians();
        let phi = if p.beta_deg > 0.0 { phi } else { -phi }; // 右旋 = "/"（方向角 +）
        let (dx, dy) = (phi.cos(), phi.sin());
        // 长度盖住整个视图（含齿顶圆直径与中心线伸出）
        let half = (p.da() + CENTER_OVERHANG * n) / 2.0 / phi.sin().abs().max(1e-3);
        for k in [-1.0f64, 0.0, 1.0] {
            let off = k * HELIX_SPACING * n;
            let (nx, ny) = (-dy, dx); // 线的法向
            let (cx, cy) = (nx * off, ny * off);
            out.push(line(
                [cx - dx * half, cy - dy * half],
                [cx + dx * half, cy + dy * half],
                LAYER_THIN,
            ));
        }
        let _ = ra;
    }
    Ok(out)
}

/// 剖视图：轴向剖切。轮廓 + 齿根线（±df/2）+ 分度线 + 轴线 + 剖面线（轴线上下两环）。
fn section_view(p: &GearParams, n: f64) -> Result<Vec<EntityType>, String> {
    let mut out = axial_outline(p, false);
    let rf = p.df() / 2.0;
    let d2 = p.d() / 2.0;
    let cl = centerline_len(p.h, n);
    // 齿根线（剖切后可见，长度 = h）—— 齿部按不剖（GB/T 4459.2）：只在齿根圆之间打剖面线
    out.push(line([-p.h / 2.0, rf], [p.h / 2.0, rf], LAYER_MAIN));
    out.push(line([-p.h / 2.0, -rf], [p.h / 2.0, -rf], LAYER_MAIN));
    // 分度线 + 轴线
    out.push(line([-cl / 2.0, d2], [cl / 2.0, d2], LAYER_CENTER));
    out.push(line([-cl / 2.0, -d2], [cl / 2.0, -d2], LAYER_CENTER));
    out.push(line([-cl / 2.0, 0.0], [cl / 2.0, 0.0], LAYER_CENTER));
    // 剖面线：一个 HATCH、两个边界环（齿根圆 ↔ 轴线，上下各一环）—— 齿部按不剖
    out.push(section_hatch(p.h, rf));
    Ok(out)
}

/// 剖面线（ANSI31，一个 HATCH 两个边界环：轴线上下各一环）。
///
/// 间距口径见 [`HATCH_SPACING_MM`]：走 `partgen_kit::hatch_ansi31_rings`（预览读的是同一份
/// 图案定义），环 flags = EXTERNAL|OUTERMOST（与花键 flags=17 同口径）。
fn section_hatch(h: f64, rf: f64) -> EntityType {
    let ring = |y0: f64, y1: f64| {
        vec![
            HatchEdge::Line { a: [-h / 2.0, y0], b: [h / 2.0, y0] },
            HatchEdge::Line { a: [h / 2.0, y0], b: [h / 2.0, y1] },
            HatchEdge::Line { a: [h / 2.0, y1], b: [-h / 2.0, y1] },
            HatchEdge::Line { a: [-h / 2.0, y1], b: [-h / 2.0, y0] },
        ]
    };
    hatch_ansi31_rings(&[ring(-rf, 0.0), ring(0.0, rf)], 0.0, HATCH_PATTERN_SCALE)
}

// ─────────────────── 内齿轮（齿圈）—— 二期，用户 2026-09-17 给模板 ───────────────────
//
// 反解自 `~/桌面/OCSM/齿轮/内齿轮.dxf`（m=2 z=40，与外齿轮模板同参数，"镜像件"）。
// 核心结论（逐点核对过）：**内齿轮的齿槽形状 = 同参数外齿轮的齿形** —— 同一条渐开线、
// 同一个 ψ(R) 公式，只是 ψ 要从**齿槽中心线**量、材料在外侧。因此：
//   * 齿顶圆 da = d − 2ha（最内）、齿根圆 df = d + 2hf（最外）；
//   * 圆角圆心在 **rf − ρ**（外齿轮是 rf + ρ）；
//   * 圆角切点是**解出来的**（不像外齿轮那样先定齿廓起点再解圆心）：
//     解 `|P(R) + ρ·n(R)| = rf − ρ`；模板核对 R=42.076、圆心角 184.446°（实测 184.445°）。
//   * 每齿 8 图元、拓扑与外齿轮同构（2 齿廓样条 + 2 圆角弧 + 2 齿根弧 + 2 齿顶弧）。

/// 内齿轮一个**齿槽单侧**的构造结果。
#[derive(Debug, Clone)]
struct FlankInt {
    knots: Vec<f64>,
    ctrl: Vec<[f64; 2]>,
    /// 齿廓在齿顶侧（内侧）的一端
    tip: [f64; 2],
    /// 齿廓在齿根圆角切点那一端
    end: [f64; 2],
    /// 齿根圆角圆心（在 rf − ρ 上）
    fillet_c: [f64; 2],
    /// 圆角与齿根圆的切点
    root_pt: [f64; 2],
    /// 圆角弧的起/讫角（相对圆角圆心，度，CCW）
    fillet_a0: f64,
    fillet_a1: f64,
    /// 齿顶圆低于基圆时：从齿顶圆到齿廓起点的径向直线
    radial_tip: Option<([f64; 2], [f64; 2])>,
    /// 圆角无解时：从齿廓末端径向落到齿根圆的直线
    radial_root: Option<([f64; 2], [f64; 2])>,
    /// 圆角是否退化（无解 → 圆角 = 0）
    no_fillet: bool,
}

/// 径向直线（或点）相对齿槽中心线的角度差（度）。
fn ang_delta_deg(q: [f64; 2], base_deg: f64) -> f64 {
    let a = q[1].atan2(q[0]).to_degrees() - base_deg;
    let mut d = a % 360.0;
    if d > 180.0 {
        d -= 360.0;
    }
    if d <= -180.0 {
        d += 360.0;
    }
    d
}

/// 解内齿轮齿根圆角的切点/圆心。返回 `(切点半径, 圆心, 切点)`。
///
/// 两个法向分支里取"圆心落在齿槽中心与齿廓之间"的那个（另一个会把圆角切到齿的另一侧）。
fn solve_internal_fillet(p: &GearParams, s: f64, sign: f64) -> Option<(f64, [f64; 2], [f64; 2])> {
    let rho = p.rho();
    let r_c = p.fillet_center_radius();
    let rf = p.df() / 2.0;
    let r0 = p.internal_flank_r_min();
    let r1 = rf - 1e-7;
    if r1 <= r0 {
        return None;
    }
    let s_deg = s.to_degrees();
    let pt = |r: f64| involute_point(p, r, s, sign);
    let normal = |r: f64| {
        let d = 1e-5;
        let a = pt((r - d).max(r0));
        let b = pt((r + d).min(r1));
        let (tx, ty) = (b[0] - a[0], b[1] - a[1]);
        let l = (tx * tx + ty * ty).sqrt().max(1e-12);
        (-ty / l, tx / l)
    };
    let mut best: Option<(f64, [f64; 2], f64)> = None;
    const SCAN: usize = 2000;
    for flip in [1.0f64, -1.0] {
        let eval = |r: f64| -> ([f64; 2], f64) {
            let q = pt(r);
            let (nx, ny) = normal(r);
            let f = [q[0] + rho * nx * flip, q[1] + rho * ny * flip];
            (f, (f[0] * f[0] + f[1] * f[1]).sqrt() - r_c)
        };
        let mut prev: Option<(f64, f64)> = None;
        for k in 0..=SCAN {
            let r = r0 + (r1 - r0) * k as f64 / SCAN as f64;
            let (_, e) = eval(r);
            if let Some((rp, ep)) = prev {
                if ep * e < 0.0 {
                    let rr = rp - ep * (r - rp) / (e - ep);
                    let (f, _) = eval(rr);
                    let dc = ang_delta_deg(f, s_deg);
                    // 判据（踩过两次坑才定下来）：圆心必须落在**它自己那个半径处齿槽的角范围**内，
                    // 即 |dc| < ψ槽(|F|)。四个根（两个法向分支 × 两侧）里只有这一个稳：
                    //   * 只按"离齿槽中心线更近"挑 → m=10 时并列挑错（根弧扫过整个齿）；
                    //   * 按"与齿廓同侧"挑 → m=2 模板对、m=10 错（圆心确实会跨过齿槽中心线）。
                    // 落在齿槽外 = 圆心跑进相邻齿的材料里，那种圆角是错的。
                    let fc = (f[0] * f[0] + f[1] * f[1]).sqrt();
                    let psi_c = p.space_half_angle(fc).to_degrees();
                    if dc.abs() < psi_c - 1e-9
                        && (best.is_none() || dc.abs() < best.as_ref().unwrap().2)
                    {
                        best = Some((rr, f, dc.abs()));
                    }
                }
            }
            prev = Some((r, e));
        }
    }
    best.map(|(r, f, _)| (r, f, pt(r)))
}

/// 内齿轮一个齿槽单侧的齿廓 + 齿根圆角（`sign=+1` 为逆时针那侧）。
fn make_flank_internal(p: &GearParams, s: f64, sign: f64) -> Result<FlankInt, String> {
    let rho = p.rho();
    let rf = p.df() / 2.0;
    let ra = p.da() / 2.0;
    let r_min = p.internal_flank_r_min();
    let solved = solve_internal_fillet(p, s, sign);
    let no_fillet = solved.is_none();
    let (r_t, fillet_c, end) = match solved {
        Some(t) => t,
        None => {
            // 退化：圆角 = 0 —— 齿廓画到齿根圆之前的某半径，再用径向直线落到齿根圆
            let r_lim = (r_min + 0.5 * (rf - r_min)).min(rf - 1e-6);
            let q = involute_point(p, r_lim, s, sign);
            (r_lim, q, q)
        }
    };
    // 5 个拟合点：从有效起点（齿顶圆，或降级时的基圆略上方）到圆角切点，半径等分
    let mut fit = [[0.0f64; 2]; 5];
    for (i, f) in [0.0, 0.25, 0.5, 0.75, 1.0].iter().enumerate() {
        let radius = r_min + (r_t - r_min) * f;
        fit[i] = involute_point(p, radius, s, sign);
    }
    let (knots, ctrl) = fit_cubic_bspline_knots(&fit)?;
    let tip = fit[0];
    let end = if no_fillet { fit[4] } else { end };
    // 圆角与齿根圆的切点：圆心沿径向拉到 rf
    let fc_len = (fillet_c[0] * fillet_c[0] + fillet_c[1] * fillet_c[1]).sqrt().max(1e-12);
    let root_pt = [fillet_c[0] * rf / fc_len, fillet_c[1] * rf / fc_len];
    let ang = |q: [f64; 2]| {
        let v = (q[0] - fillet_c[0], q[1] - fillet_c[1]);
        v.1.atan2(v.0).to_degrees()
    };
    // 圆角弧 CCW 方向（模板口径）：−1 侧 切点→齿根切点；+1 侧 齿根切点→切点
    let (mut a0, mut a1) = if sign > 0.0 {
        (ang(root_pt), ang(end))
    } else {
        (ang(end), ang(root_pt))
    };
    if (a1 - a0).rem_euclid(360.0) > 180.0 {
        std::mem::swap(&mut a0, &mut a1);
    }
    // 齿顶圆低于基圆：齿廓到不了齿顶圆，用径向直线补一段（简化画法）
    let radial_tip = if r_min > ra + 1e-12 {
        let th = tip[1].atan2(tip[0]);
        Some(([ra * th.cos(), ra * th.sin()], tip))
    } else {
        None
    };
    // 圆角退化：齿廓末端径向落到齿根圆
    let radial_root = if no_fillet {
        let th = end[1].atan2(end[0]);
        Some((end, [rf * th.cos(), rf * th.sin()]))
    } else {
        None
    };
    Ok(FlankInt {
        knots,
        ctrl,
        tip,
        end,
        fillet_c,
        root_pt,
        fillet_a0: a0,
        fillet_a1: a1,
        radial_tip,
        radial_root,
        no_fillet,
    })
}

/// 内齿轮齿廓的实体（B 样条）。
fn spline_of_int(f: &FlankInt) -> EntityType {
    spline_entity(&f.knots, &f.ctrl)
}

/// 内齿轮的齿根那一段实体：径向直线（如有）+ 圆角弧（如有）。
fn push_root_int(out: &mut Vec<EntityType>, p: &GearParams, f: &FlankInt) {
    if let Some((a, b)) = f.radial_root {
        out.push(line(a, b, LAYER_MAIN));
    }
    if !f.no_fillet {
        out.push(arc_deg(f.fillet_c, p.rho(), f.fillet_a0, f.fillet_a1, LAYER_MAIN));
    }
}

/// 内齿轮端视图（常规正视图）：**齿槽**按外齿轮齿形画，齿朝圆心。
///
/// 相位：齿槽中心在半个齿距处（模板：齿中心 0°、齿槽 4.5°），与外齿轮模板的相位约定同构。
/// **不画分度圆**——内齿轮模板里没有（外齿轮模板有；模板是唯一权威）。
fn front_internal(p: &GearParams, n: f64) -> Result<Vec<EntityType>, String> {
    if p.internal_tooth_crossed() {
        return Err(format!(
            "内齿轮 α={}° 配 ha*={} 时齿槽过宽（ψ(da/2)={:.4}° ≥ 半齿距 {:.4}°）：相邻齿槽的齿廓\
             在齿顶圆之前相交，端视图画不出真实齿廓。请减小 ha*（短齿顶）、减小 α 或增大齿数；剖视图不受影响。",
            trim(p.alpha_deg),
            trim(p.ha),
            p.space_half_angle(p.da().max(p.db()) / 2.0).to_degrees(),
            (p.pitch_angle() / 2.0).to_degrees()
        ));
    }
    let ra = p.da() / 2.0;
    let rf = p.df() / 2.0;
    let pitch = p.pitch_angle();
    let psi_tip = p.internal_tip_half_angle().to_degrees();
    let mut out = Vec::with_capacity(p.z as usize * 8 + 4);
    let s_first = pitch / 2.0;
    for t in 0..p.z as usize {
        let s = s_first + pitch * t as f64;
        let s_deg = s.to_degrees();
        let fl = make_flank_internal(p, s, 1.0)?; // 齿槽 +1 侧
        let fr = make_flank_internal(p, s, -1.0)?; // 齿槽 −1 侧
        let pitch_deg = pitch.to_degrees();
        let tooth_c = s_deg + pitch_deg / 2.0;
        // CCW 链：−1 侧齿廓（齿顶→圆角）→ 圆角 → 齿根弧半条
        if let Some((a, b)) = fr.radial_tip {
            out.push(line(a, b, LAYER_MAIN));
        }
        out.push(spline_of_int(&fr));
        push_root_int(&mut out, p, &fr);
        // 齿根弧两半：两端是两侧圆角的齿根切点。**按相对齿槽中心的偏移排序**取，不能按
        // "−侧在前"写死 —— 圆心跨过齿槽中心线时（m=10 常见）两边会互换，写死会让根弧
        // 反向扫掉一整圈（链条自检当场抓到 34.6mm 的"断链"）。
        let (d_lo, d_hi) = {
            let (a, b) = (ang_delta_deg(fr.root_pt, s_deg), ang_delta_deg(fl.root_pt, s_deg));
            if a <= b {
                (a, b)
            } else {
                (b, a)
            }
        };
        out.push(arc_deg(
            [0.0, 0.0],
            rf,
            near(s_deg + d_lo, s_deg - 1.0),
            near(s_deg, s_deg - 1.0),
            LAYER_MAIN,
        ));
        // 齿根弧另半条 → +1 侧圆角 → 齿廓（圆角→齿顶）
        out.push(arc_deg(
            [0.0, 0.0],
            rf,
            near(s_deg, s_deg + 1.0),
            near(s_deg + d_hi, s_deg + 1.0),
            LAYER_MAIN,
        ));
        push_root_int(&mut out, p, &fl);
        out.push(spline_of_int(&fl));
        if let Some((a, b)) = fl.radial_tip {
            out.push(line(a, b, LAYER_MAIN));
        }
        // 齿顶弧：以齿中心线为界各半条（模板同构）
        // 端点 = 齿中心 ± 齿顶半角 ψtip = pitch/2 − ψ槽 —— 注意不是 s ± ψtip！
        // （走过坑：写成 s + ψtip 时齿顶弧起点错到齿的另一侧，链条在齿顶处拉出 1mm 直线）
        out.push(arc_deg(
            [0.0, 0.0],
            ra,
            near(tooth_c - psi_tip, tooth_c - 1.0),
            near(tooth_c, tooth_c - 1.0),
            LAYER_MAIN,
        ));
        out.push(arc_deg(
            [0.0, 0.0],
            ra,
            near(tooth_c, tooth_c + 1.0),
            near(tooth_c + psi_tip, tooth_c + 1.0),
            LAYER_MAIN,
        ));
    }
    // 十字中心线：模板实测长 91 = 齿根圆直径 85 + 1×6
    out.extend(cross_centerlines([0.0, 0.0], p.df(), n));
    Ok(out)
}

/// **内齿圈剖视图的共用模板**（内齿轮与内花键同构，逐条对齐用户「内齿轮使用示例.dxf」）。
///
/// 每半侧 10 条：端面（0→ra+C）+ 外侧端面（ra+C→rf）×2 端 + 齿顶线 + 齿根线 + 内孔壁 ×2 + 孔口 45° 倒角 ×2；
/// 加 分度线 ×2（点划线）+ 轴线 ×1，长度都 = [`centerline_len`]`(len, n)` = `len + 6n`
/// （两端各 3n，绕长度中心对称；用户 2026-09-21 定案，与外齿轮/外花键全口径统一）。
/// 共 23 条（20 粗实线 + 3 中心线）。
/// **不打剖面线**（用户 2026-09-17 明确：齿圈外壁结构由用户/AI 在生产环境里延伸后再打）、**不画齿圈外壁**。
///
/// 内齿轮与内花键只在齿形参数上不同：`ra_bore` = 里侧齿顶半径（内齿轮 `da/2`；
/// 内花键 `D_ii/2`）；`rf_outer` = 外侧齿根半径（内齿轮 `df/2`；内花键 `D_ei/2`）；
/// `pitch_dia` = 分度圆直径；`c` = 孔口倒角（都取 `round(0.6m)`）。
/// `x0` = 左端面 x（内齿轮以齿宽中心为原点 → `-h/2`；花键按自身基点口径 → 0）。
fn internal_bore_section(
    x0: f64,
    ra_bore: f64,
    rf_outer: f64,
    pitch_dia: f64,
    len: f64,
    c: f64,
    n: f64,
) -> Vec<EntityType> {
    let mut out = Vec::with_capacity(23);
    for s in [1.0f64, -1.0] {
        // 端面：x0 → ra+C 与 ra+C → rf（模板把它画成两段）
        out.push(line([x0, 0.0], [x0, s * (ra_bore + c)], LAYER_MAIN));
        out.push(line([x0, s * (ra_bore + c)], [x0, s * rf_outer], LAYER_MAIN));
        out.push(line([x0 + len, 0.0], [x0 + len, s * (ra_bore + c)], LAYER_MAIN));
        out.push(line([x0 + len, s * (ra_bore + c)], [x0 + len, s * rf_outer], LAYER_MAIN));
        // 齿顶线（内孔）与齿根线（外圆）
        out.push(line([x0 + c, s * ra_bore], [x0 + len - c, s * ra_bore], LAYER_MAIN));
        out.push(line([x0, s * rf_outer], [x0 + len, s * rf_outer], LAYER_MAIN));
        // 内孔壁
        out.push(line([x0 + c, 0.0], [x0 + c, s * ra_bore], LAYER_MAIN));
        out.push(line([x0 + len - c, 0.0], [x0 + len - c, s * ra_bore], LAYER_MAIN));
        // 孔口 45° 倒角（C = round(0.6m)，与模板的 1×1 一致）
        out.push(line([x0 + c, s * ra_bore], [x0, s * (ra_bore + c)], LAYER_MAIN));
        out.push(line([x0 + len - c, s * ra_bore], [x0 + len, s * (ra_bore + c)], LAYER_MAIN));
    }
    // 分度线（点划线）+ 轴线（中心线）：长度 = centerline_len(len, n) = len + 6n（两端各 3n），
    // 绕长度中心对称。用户 2026-09-21 定案：与外齿轮/外花键统一（原模板口径 L+3n/L+4n 作废）。
    let d2 = pitch_dia / 2.0;
    let half = centerline_len(len, n) / 2.0;
    let xm = x0 + len / 2.0;
    out.push(line([xm - half, d2], [xm + half, d2], LAYER_CENTER));
    out.push(line([xm - half, -d2], [xm + half, -d2], LAYER_CENTER));
    out.push(line([xm - half, 0.0], [xm + half, 0.0], LAYER_CENTER));
    out
}

/// 内齿轮剖视图（模板结构，逐条对齐）：走内齿圈共用模板 [`internal_bore_section`]。
fn section_internal(p: &GearParams, n: f64) -> Result<Vec<EntityType>, String> {
    Ok(internal_bore_section(
        -p.h / 2.0,
        p.da() / 2.0,
        p.df() / 2.0,
        p.d(),
        p.h,
        p.chamfer(),
        n,
    ))
}

/// 内齿轮端视图的闭合链采样（质量估算 / 连续性自检用）。
///
/// **必须与 `front_internal` 的实体顺序逐条对齐**（走过一次坑：自己另写一套弧方向，
/// 结果根弧多扫一整圈、链条在齿顶到齿根之间拉出 5mm 直线，测试当场抓到）。
fn internal_chain_points(p: &GearParams, seg: usize) -> Result<Vec<[f64; 2]>, String> {
    let ra = p.da() / 2.0;
    let rf = p.df() / 2.0;
    let rho = p.rho();
    let pitch = p.pitch_angle();
    let psi_tip = p.internal_tip_half_angle().to_degrees();
    let mut out: Vec<[f64; 2]> = Vec::new();
    let arc_of = |c: [f64; 2], r: f64, a0: f64, a1: f64, out: &mut Vec<[f64; 2]>| {
        let sweep = (a1 - a0).rem_euclid(360.0);
        for k in 0..=seg {
            let a = (a0 + sweep * k as f64 / seg as f64).to_radians();
            out.push([c[0] + r * a.cos(), c[1] + r * a.sin()]);
        }
    };
    let spline_pts = |f: &FlankInt, reverse: bool, out: &mut Vec<[f64; 2]>| {
        let (t0, t1) = (f.knots[3], f.knots[7]);
        let mut v = Vec::with_capacity(3 * seg + 1);
        for k in 0..=3 * seg {
            let u = t0 + (t1 - t0) * k as f64 / (3 * seg) as f64;
            let mut q = [0.0f64; 2];
            for (j, d) in f.ctrl.iter().enumerate() {
                let w = basis(j, 3, u, &f.knots);
                q[0] += w * d[0];
                q[1] += w * d[1];
            }
            v.push(q);
        }
        if reverse {
            v.reverse();
        }
        out.extend(v);
    };
    let s_first = pitch / 2.0;
    for t in 0..p.z as usize {
        let s = s_first + pitch * t as f64;
        let s_deg = s.to_degrees();
        let tooth_c = s_deg + pitch.to_degrees() / 2.0;
        let fl = make_flank_internal(p, s, 1.0)?;
        let fr = make_flank_internal(p, s, -1.0)?;
        // 径向直线也要按 seg 插值（否则 3mm 的直线段会被连续性自检当成"断链"）
        let seg_line = |a: [f64; 2], b: [f64; 2], out: &mut Vec<[f64; 2]>| {
            for k in 0..=seg {
                let t = k as f64 / seg as f64;
                out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
            }
        };
        // −1 侧：径向直线（降级）→ 齿廓 → 圆角/径向落地 → 齿根弧两半
        if let Some((a, b)) = fr.radial_tip {
            seg_line(a, b, &mut out);
        }
        spline_pts(&fr, false, &mut out);
        if !fr.no_fillet {
            arc_of(fr.fillet_c, rho, fr.fillet_a0, fr.fillet_a1, &mut out);
        }
        if let Some((a, b)) = fr.radial_root {
            seg_line(a, b, &mut out);
        }
        let (d_lo, d_hi) = {
            let (a, b) = (ang_delta_deg(fr.root_pt, s_deg), ang_delta_deg(fl.root_pt, s_deg));
            if a <= b {
                (a, b)
            } else {
                (b, a)
            }
        };
        arc_of(
            [0.0, 0.0],
            rf,
            near(s_deg + d_lo, s_deg - 1.0),
            near(s_deg, s_deg - 1.0),
            &mut out,
        );
        arc_of(
            [0.0, 0.0],
            rf,
            near(s_deg, s_deg + 1.0),
            near(s_deg + d_hi, s_deg + 1.0),
            &mut out,
        );
        // +1 侧：圆角 → 齿廓 → 径向直线（降级）
        if let Some((a, b)) = fl.radial_root {
            // +1 侧：链从齿根圆走向齿廓末端（入口在齿根圆上）
            seg_line(b, a, &mut out);
        }
        if !fl.no_fillet {
            arc_of(fl.fillet_c, rho, fl.fillet_a0, fl.fillet_a1, &mut out);
        }
        spline_pts(&fl, true, &mut out);
        if let Some((a, b)) = fl.radial_tip {
            seg_line(b, a, &mut out);
        }
        // 齿顶弧两半（端点 = 齿中心 ± ψtip，与 front_internal 一致）
        arc_of(
            [0.0, 0.0],
            ra,
            near(tooth_c - psi_tip, tooth_c - 1.0),
            near(tooth_c, tooth_c - 1.0),
            &mut out,
        );
        arc_of(
            [0.0, 0.0],
            ra,
            near(tooth_c, tooth_c + 1.0),
            near(tooth_c + psi_tip, tooth_c + 1.0),
            &mut out,
        );
    }
    Ok(out)
}

/// 内齿轮质量估算（kg）：齿圈材料 = π·rf² − 齿廓围成的面积（齿廓围的是空腔），× 宽度 × 钢 7.85e-6。
///
/// 注意：这是**齿高那一段齿圈**的质量（剖视图画到齿根圆为止的那部分），
/// 真实齿圈还要加轮缘/腹板 —— 与模板"外壁留给用户"的口径一致。
fn internal_weight_kg(p: &GearParams) -> String {
    if p.internal_tooth_crossed() {
        return String::new(); // 齿槽在齿顶前相交，面积无意义
    }
    let rf = p.df() / 2.0;
    let pts = match internal_chain_points(p, 8) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };
    let n = pts.len();
    let mut area = 0.0;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        area += a[0] * b[1] - b[0] * a[1];
    }
    let void = (area / 2.0).abs();
    let ring = (std::f64::consts::PI * rf * rf - void).max(0.0);
    format!("{:.3}", ring * p.h * 7.85e-6)
}

// ─────────────────────────── 对外接口 ───────────────────────────

/// OCSM 初始化检查（**插入前拦一下**，用户 2026-09-17 要求）。
///
/// 没跑过 `OCSM`（也没被其它 OCSM 命令“确保”过）的图纸直接生成齿轮，出图会是：
/// 中心线变**实线白线**（`3中心线层` 没挂 `CENTER2`）、轮廓/剖面线层的颜色线宽也不对。
/// 判据取这两件**直接影响出图外观**的事：图形层在、中心线层挂着点划线。
pub fn ocsm_ready(doc: &ocs_plugin_api::host::acadrust::CadDocument) -> Result<(), String> {
    let find = |name: &str| {
        doc.layers
            .iter()
            .find(|ly| ly.name.eq_ignore_ascii_case(name))
    };
    const NEED: [&str; 4] = [LAYER_MAIN, LAYER_THIN, LAYER_CENTER, LAYER_HATCH];
    let missing: Vec<&str> = NEED.iter().copied().filter(|n| find(n).is_none()).collect();
    let center_ok = find(LAYER_CENTER)
        .map(|ly| ly.line_type.eq_ignore_ascii_case("CENTER2"))
        .unwrap_or(false);
    if missing.is_empty() && center_ok {
        return Ok(());
    }
    let why = if !missing.is_empty() {
        crate::i18n::t_fmt(
            "cmd.gear.warn.missing_layers",
            &[("list", &missing.join(&crate::i18n::t("cmd.gear.sep.list")))],
        )
    } else {
        crate::i18n::t_fmt("cmd.gear.warn.no_center2", &[("layer", LAYER_CENTER)])
    };
    Err(crate::i18n::t_fmt(
        "cmd.gear.err.not_initialized",
        &[("why", &why)],
    ))
}

/// 生成一个视图的图元（已落 OCSM 图层）。
///
/// `n` = 视图比例系数（图框 `比例` 属性，1:2 → 2.0）：只影响中心线/螺旋线的伸出与间距，
/// 不影响齿轮本身尺寸（齿轮始终按真实尺寸画，图框负责比例）。
pub fn generate(p: &GearParams, view: GearView, n: f64) -> Result<GenPart, String> {
    if p.is_spline() {
        return generate_spline(p, view, n);
    }
    p.validate()?;
    if !view.available_for(p.kind) {
        return Err(crate::i18n::t_fmt(
            "cmd.gear.err.view_unavailable",
            &[("kind", &kind_label(p.kind)), ("view", &view_label(view))],
        ));
    }
    let entities = match (p.kind, view) {
        (GearKind::External, GearView::Section) => section_view(p, n)?,
        (GearKind::External, GearView::Side) => side_view(p, n)?,
        (GearKind::External, GearView::Simplified) => front_simplified(p, n)?,
        (GearKind::External, GearView::Front) => front_regular(p, n)?,
        (GearKind::Internal, GearView::Section) => section_internal(p, n)?,
        (GearKind::Internal, GearView::Front) => front_internal(p, n)?,
        (_, v) => {
            return Err(crate::i18n::t_fmt(
                "cmd.gear.err.view_unavailable_short",
                &[("kind", &kind_label(p.kind)), ("view", &view_label(v))],
            ))
        }
    };
    let bbox = bbox_of(&entities);
    Ok(GenPart {
        entities,
        meta: PartMeta {
            code: p.spec(),
            name: crate::i18n::t_fmt("cmd.gear.name.pair", &[("a", p.kind.label()), ("b", view.label_for(p.kind))]),
            spec: p.spec(),
            material: String::new(),
            weight: if p.kind.is_internal() {
                internal_weight_kg(p)
            } else {
                solid_weight_kg(p)
            },
        },
        bbox,
    })
}

/// 花键端面倒角 C = round(0.6m)（花键体系没有单独的端面倒角数据，沿用齿轮口径；
/// 与轴段 `INVOLSPLINE` 段同口径）。内花键剖视的孔口倒角也用它。
fn spline_chamfer(p: &crate::invol_spline::InvolParams) -> f64 {
    (CHAMFER_RATIO * p.m).round()
}

/// 外花键模式的剖面线：轴线↔齿根圆两环（齿部按不剖）。x 跨度 = 有效长度 L。
/// 内花键**不打剖面线**（用户定案：同内齿轮齿圈剖视，走 [`internal_bore_section`]）。
fn spline_section_hatch(p: &crate::invol_spline::InvolParams, len: f64) -> EntityType {
    let r_in = 0.0;
    let r_out = p.df() / 2.0;
    let upper = vec![
        HatchEdge::Line { a: [0.0, r_in], b: [len, r_in] },
        HatchEdge::Line { a: [len, r_in], b: [len, r_out] },
        HatchEdge::Line { a: [len, r_out], b: [0.0, r_out] },
        HatchEdge::Line { a: [0.0, r_out], b: [0.0, r_in] },
    ];
    let lower = vec![
        HatchEdge::Line { a: [0.0, -r_in], b: [0.0, -r_out] },
        HatchEdge::Line { a: [0.0, -r_out], b: [len, -r_out] },
        HatchEdge::Line { a: [len, -r_out], b: [len, -r_in] },
        HatchEdge::Line { a: [len, -r_in], b: [0.0, -r_in] },
    ];
    hatch_ansi31_rings(&[upper, lower], 0.0, HATCH_PATTERN_SCALE)
}

/// 外花键轴向视图（GEAR 侧视/剖视共用）：**两端倒角**，口径照齿轮 [`axial_outline`]。
///
/// 与齿轮的几何差异只有基点：花键以自身左端面为 `x=0`（`0..L`），齿轮以齿宽中心为原点；
/// 倒角三角完全同口径：`C = round(0.6m)`、端面高 `ra−C`、齿顶面长 `L−2C`，倒角斜线 + 台阶竖线
/// （`steps=true` 侧视图画；剖视图按齿轮 `section_view` 口径不画这两条棱边）。
/// 中心线（分度线 ×2 + 轴线）长度 = `L + CENTER_OVERHANG×n`（两端各 3n），绕 `L/2` 对称，
/// 与齿轮 `side_view`/`section_view` 的 `cl = p.h + CENTER_OVERHANG * n` 同一算法。
fn spline_axial_view(
    ra: f64,
    rf: f64,
    pitch_dia: f64,
    len: f64,
    c: f64,
    n: f64,
    minor_layer: &str,
    steps: bool,
) -> Vec<EntityType> {
    let mut out = Vec::with_capacity(15);
    let hi = ra - c; // 倒角后端面半高
    // 端面线（两端，±(ra−C) 高）
    out.push(line([0.0, -hi], [0.0, hi], LAYER_MAIN));
    out.push(line([len, -hi], [len, hi], LAYER_MAIN));
    // 倒角台阶线（±ra 高）—— 侧视图画，剖视图不画（同齿轮）
    if steps {
        out.push(line([c, -ra], [c, ra], LAYER_MAIN));
        out.push(line([len - c, -ra], [len - c, ra], LAYER_MAIN));
    }
    // 齿顶面（±ra，长 L−2C）
    out.push(line([c, ra], [len - c, ra], LAYER_MAIN));
    out.push(line([c, -ra], [len - c, -ra], LAYER_MAIN));
    // 四个 45° 倒角
    out.push(line([0.0, hi], [c, ra], LAYER_MAIN));
    out.push(line([len - c, ra], [len, hi], LAYER_MAIN));
    out.push(line([0.0, -hi], [c, -ra], LAYER_MAIN));
    out.push(line([len - c, -ra], [len, -hi], LAYER_MAIN));
    // 小径线（侧视 = 2细线层 / 剖视 = 1轮廓实线层）
    out.push(line([0.0, -rf], [len, -rf], minor_layer));
    out.push(line([0.0, rf], [len, rf], minor_layer));
    // 分度线 ×2 + 轴线：长度 = centerline_len(len, n) = L + 6n（两端各 3n），与齿轮中心线同一算法
    let cl = centerline_len(len, n);
    let (x0, x1) = (len / 2.0 - cl / 2.0, len / 2.0 + cl / 2.0);
    let d2 = pitch_dia / 2.0;
    out.push(line([x0, d2], [x1, d2], LAYER_CENTER));
    out.push(line([x0, -d2], [x1, -d2], LAYER_CENTER));
    out.push(line([x0, 0.0], [x1, 0.0], LAYER_CENTER));
    out
}

/// 外花键轴向视图入口：先按齿轮 `validate()` 同口径校验倒角（C 太大/N 不小于齿顶半径直接报错），
/// 再按 `steps` 出侧视（含台阶竖线）或剖视（剖视另加剖面线，见调用方）。
fn external_spline_axial_view(
    engine: &crate::invol_spline::InvolParams,
    len: f64,
    n: f64,
    minor_layer: &str,
    steps: bool,
) -> Result<Vec<EntityType>, String> {
    let c = spline_chamfer(engine);
    let ra = engine.da() / 2.0;
    if c * 2.0 >= len {
        return Err(format!(
            "外花键轴向倒角 C=round(0.6m)={:.0} 太大（有效长度 L={}）——加大 L 或减小模数（同齿轮校验口径）。",
            c,
            trim(len)
        ));
    }
    if c >= ra {
        return Err(format!(
            "外花键轴向倒角 C={:.0} 不小于大径半径 da/2={}，无法画图（同齿轮口径）。",
            c,
            trim(ra)
        ));
    }
    Ok(spline_axial_view(
        ra,
        engine.df() / 2.0,
        engine.d(),
        len,
        c,
        n,
        minor_layer,
        steps,
    ))
}

/// **花键模式出图**（齿轮生成器入口）：几何全在 `invol_spline.rs`，这里只组织视图/meta/块名。
///
/// 视图：`端视图` = 真实渐开线端面齿廓（内/外花键各自几何）；
/// `剖视图` 外花键 = 两端倒角的轴向轮廓 + 剖面线，内花键 = 内齿轮齿圈剖视模板（[`internal_bore_section`]，不打剖面线）；
/// `侧视图` 只服务外花键（内花键无侧视图，明确报错）；`简化正视图` 无花键画法，不提供。
fn generate_spline(p: &GearParams, view: GearView, n: f64) -> Result<GenPart, String> {
    let (engine, origin) = p.spline_engine()?;
    if !view.available_for_spline(p.kind) {
        return Err(match (p.kind, view) {
            (GearKind::Internal, GearView::Side) => internal_spline_no_side_view_msg(),
            (GearKind::Internal, GearView::Simplified) => crate::i18n::t_fmt(
                "cmd.gear.err.internal_spline_view_unavailable",
                &[("view", &view_label(view))],
            ),
            _ => crate::i18n::t_fmt(
                "cmd.gear.err.spline_view_unavailable",
                &[("view", &view_label(view))],
            ),
        });
    }
    let len = p.h;
    let entities = match view {
        GearView::Front => engine.front_view(n)?,
        // 花键是独立零件（两侧都是自由外角）：照齿轮侧视图**两端都倒角**（口径见 `spline_axial_view`）。
        GearView::Side => external_spline_axial_view(&engine, len, n, LAYER_THIN, true)?,
        GearView::Section => {
            if engine.internal {
                // 内花键剖视 = 内齿轮那套（齿圈内齿不剖、不打剖面线、不画齿圈外壁），同一模板换齿形参数。
                // 厚度/倒角校验与内齿轮 `validate()` 同口径：C 太大直接报错，不出乱图。
                let c = spline_chamfer(&engine);
                if c * 2.0 >= len {
                    return Err(crate::i18n::t_fmt(
                        "cmd.gear.err.internal_spline_chamfer_len",
                        &[("c", &format!("{:.0}", c)), ("len", &trim(len))],
                    ));
                }
                if c >= engine.internal_tip_radius() {
                    return Err(crate::i18n::t_fmt(
                        "cmd.gear.err.internal_spline_chamfer_radius",
                        &[("c", &format!("{:.0}", c)), ("r", &trim(engine.internal_tip_radius()))],
                    ));
                }
                internal_bore_section(
                    0.0,
                    engine.internal_tip_radius(),
                    engine.internal_root_radius(),
                    engine.d(),
                    len,
                    c,
                    n,
                )
            } else {
                // 剖视同齿轮 `section_view`：倒角照画，但不画两条台阶竖线（全剖时该棱边按惯例省略）。
                let mut v = external_spline_axial_view(&engine, len, n, LAYER_MAIN, false)?;
                v.push(spline_section_hatch(&engine, len));
                v
            }
        }
        _ => unreachable!("available_for_spline 已拦住"),
    };
    let bbox = bbox_of(&entities);
    Ok(GenPart {
        entities,
        meta: PartMeta {
            code: engine.std.code().into(),
            name: crate::i18n::t_fmt("cmd.gear.name.pair", &[("a", p.spline_kind_label()), ("b", view.label_for_spline(p.kind))]),
            spec: p.spline_spec(&engine, origin.as_ref()),
            material: String::new(),
            weight: String::new(),
        },
        bbox,
    })
}

/// 实心（未开孔）齿轮质量估算 kg：齿廓围成的面积 × 厚度 × 钢 7.85e-6 kg/mm³。
///
/// 一期不画轴孔，所以这里给的是**毛坯质量**（明细表里如需要请按实际孔型修正）。
fn solid_weight_kg(p: &GearParams) -> String {
    if p.tooth_tip_crossed() {
        return String::new(); // 齿廓在齿顶前相交，面积无意义
    }
    let ra = p.da() / 2.0;
    let rf = p.df() / 2.0;
    let pitch = p.pitch_angle();
    let c_first = pitch / 2.0;
    const SEG: usize = 12;
    let arc = |r: f64, a0_deg: f64, a1_deg: f64, out: &mut Vec<[f64; 2]>| {
        for k in 0..=SEG {
            let a = (a0_deg + (a1_deg - a0_deg) * k as f64 / SEG as f64).to_radians();
            out.push([r * a.cos(), r * a.sin()]);
        }
    };
    let mut pts: Vec<[f64; 2]> = Vec::with_capacity(p.z as usize * (8 * SEG + 4));
    for t in 0..p.z as usize {
        let c = c_first + pitch * t as f64;
        let (fl, fr) = match (make_flank(p, c, 1.0), make_flank(p, c, -1.0)) {
            (Ok(a), Ok(b)) => (a, b),
            _ => return String::new(),
        };
        let c_deg = c.to_degrees();
        // 与 front_regular 同一条 CCW 链：
        // 齿根弧 → −1 侧齿根段 → −1 侧齿廓 → 齿顶弧 → +1 侧齿廓 → +1 侧齿根段
        arc(rf, c_deg - pitch.to_degrees() / 2.0, near(th_of(fr.root_pt), c_deg - 3.0), &mut pts);
        let neg = flank_points(p, &fr, SEG); // 齿根端 → 齿顶
        pts.extend(neg.iter().copied());
        arc(
            ra,
            c_deg - p.half_tooth_angle(ra).to_degrees(),
            c_deg + p.half_tooth_angle(ra).to_degrees(),
            &mut pts,
        );
        let pos = flank_points(p, &fl, SEG); // 齿根端 → 齿顶（要反着走）
        pts.extend(pos.iter().rev().copied());
    }
    let n = pts.len();
    let mut area = 0.0;
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        area += a[0] * b[1] - b[0] * a[1];
    }
    area = (area / 2.0).abs();
    format!("{:.3}", area * p.h * 7.85e-6)
}

/// 以 `center` 为圆心、半径 r 的圆弧采样（角度由 a0 到 a1，度）。
fn arc_center(
    center: [f64; 2],
    r: f64,
    a0: f64,
    a1: f64,
    seg: usize,
    out: &mut Vec<[f64; 2]>,
) {
    for k in 0..=seg {
        let a = (a0 + (a1 - a0) * k as f64 / seg as f64).to_radians();
        out.push([center[0] + r * a.cos(), center[1] + r * a.sin()]);
    }
}

/// 齿廓起点（第一个拟合点）半径：rb + (ra−rb)/8。
fn fr_start_radius(p: &GearParams) -> f64 {
    p.flank_start_radius()
}

fn bbox_of(entities: &[EntityType]) -> [f64; 4] {
    let mut bb = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    let mut add = |x: f64, y: f64| {
        bb[0] = bb[0].min(x);
        bb[1] = bb[1].min(y);
        bb[2] = bb[2].max(x);
        bb[3] = bb[3].max(y);
    };
    for e in entities {
        match e {
            EntityType::Line(l) => {
                add(l.start.x, l.start.y);
                add(l.end.x, l.end.y);
            }
            EntityType::Circle(c) => {
                add(c.center.x - c.radius, c.center.y - c.radius);
                add(c.center.x + c.radius, c.center.y + c.radius);
            }
            EntityType::Arc(a) => {
                add(a.center.x - a.radius, a.center.y - a.radius);
                add(a.center.x + a.radius, a.center.y + a.radius);
            }
            EntityType::Spline(s) => {
                for q in &s.control_points {
                    add(q.x, q.y);
                }
            }
            _ => {}
        }
    }
    if !bb[0].is_finite() {
        bb = [-1.0, -1.0, 1.0, 1.0];
    }
    bb
}

/// 块名：`OCSM_GEAR_M2_Z40_H20_FRONT`（与标准件 `OCSM_<族>_<规格>_<视图>` 同风格）。
/// 斜齿轮加 `_B<|β|>R|L`（右/左旋），变位加 `_X<Xn>`（负值用 `N` 前缀）；
/// 径节制用 `_DP<原值>` 代替 `_M<换算模数>`（同模数不同来源不串块）。
pub fn block_name(p: &GearParams, view: GearView) -> String {
    if p.is_spline() {
        // 参数给全时用引擎算出的 m/z/x/d_B（与 spline_block_name 同名）；
        // 参数未给全（GUI 渐进输入/预览）时用选项占位，保证同参数稳定同名。
        if let Ok((engine, _)) = p.spline_engine() {
            return p.spline_block_name(&engine, view);
        }
        let s = p.spline.as_ref().unwrap();
        let mut name = String::from("OCSM_SPLINE");
        if p.kind.is_internal() {
            name.push_str("_INT");
        }
        name.push_str(&format!("_{}", s.std.label()));
        if !s.profile.trim().is_empty() {
            name.push_str(&format!("_{}", s.profile.trim()));
        }
        if let Some(d_b) = s.d_b {
            name.push_str(&format!("_DB{}", trim(d_b).replace('.', "_")));
        }
        if s.std == crate::invol_spline::SplineStd::ANSI {
            // ANSI：块名保留径节原值 `_P<P>_PS<2P>_N<z>`（与 `spline_block_name` 同口径）。
            if let Some(pv) = s.m {
                name.push_str(&format!(
                    "_P{}_PS{}_N{}",
                    trim(pv).replace('.', "_"),
                    trim(2.0 * pv).replace('.', "_"),
                    p.z
                ));
            } else {
                name.push_str(&format!("_M{}_Z{}", trim(p.m).replace('.', "_"), p.z));
            }
        } else {
            name.push_str(&format!("_M{}_Z{}", trim(p.m).replace('.', "_"), p.z));
        }
        if p.x.abs() > 1e-9 {
            name.push_str(&format!(
                "_X{}{}",
                if p.x < 0.0 { "N" } else { "" },
                trim(p.x.abs()).replace('.', "_")
            ));
        }
        name.push_str(&format!("_H{}", trim(p.h).replace('.', "_")));
        name.push('_');
        name.push_str(&view.key().to_ascii_uppercase());
        return name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect();
    }
    let mut s = String::from("OCSM_GEAR");
    if p.kind.is_internal() {
        s.push_str("_INT"); // 内齿轮标记；外齿轮块名保持一期原样不变
    }
    if p.is_dp() {
        // 径节制：块名保留 DP 原值（不写换算后的 m，避免与同模数的模数制件同名串块）。
        let dp = p.dp.unwrap_or_else(|| dp_from_m(p.m));
        s.push_str(&format!("_DP{}_Z{}", trim(dp).replace('.', "_"), p.z));
    } else {
        s.push_str(&format!("_M{}_Z{}", trim(p.m).replace('.', "_"), p.z));
    }
    if (p.alpha_deg - ALPHA_N_DEG).abs() > 1e-9 {
        // 非默认压力角必须进块名：否则 20° 与 25° 的同规格件会命中同名块（幂等复用会串图）
        s.push_str(&format!("_A{}", trim(p.alpha_deg.abs()).replace('.', "_")));
    }
    if p.is_helical() {
        s.push_str(&format!(
            "_B{}{}",
            trim(p.beta_deg.abs()).replace('.', "_"),
            if p.beta_deg > 0.0 { "R" } else { "L" }
        ));
    }
    if p.x.abs() > 1e-9 {
        s.push_str(&format!(
            "_X{}{}",
            if p.x < 0.0 { "N" } else { "" },
            trim(p.x.abs()).replace('.', "_")
        ));
    }
    s.push_str(&format!("_H{}", trim(p.h).replace('.', "_")));
    s.push('_');
    s.push_str(&view.key().to_ascii_uppercase());
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}

/// 体系标识写入（同一行写两个不同的花键体系 → 报错，不静默覆盖）。
fn set_spline_std_once(
    slot: &mut Option<crate::invol_spline::SplineStd>,
    std: crate::invol_spline::SplineStd,
) -> Result<(), String> {
    match *slot {
        Some(old) if old != std => Err(crate::i18n::t_fmt("cmd.gear.err.two_spline_stds", &[("v0", &old.label()), ("v1", &std.label())])),
        _ => {
            *slot = Some(std);
            Ok(())
        }
    }
}

/// 齿轮体系标识写入（`M` 与 `DP` 互斥；同值重复不报）。
fn set_gear_std_once(slot: &mut Option<GearStd>, std: GearStd) -> Result<(), String> {
    match *slot {
        Some(old) if old != std => Err(m_dp_conflict_msg()),
        _ => {
            *slot = Some(std);
            Ok(())
        }
    }
}

/// 花键模式没显式给体系标识时的默认：齿廓代号/体系标识自带标准（GB30R→GB、DIN→DIN、
/// NF→NF、ANSI→ANSI）；否则 GB（默认）。
///
/// **不再从 `d_B` 反推体系**（用户定案）：给 `d_B` 而不显式写 `std=DIN`/`std=NF` 时按 GB
/// 处理并报 [`crate::invol_spline::GB_D_B_MSG`]（体系必须显式声明）。
pub fn default_spline_std(profile: &str) -> crate::invol_spline::SplineStd {
    if let Some((std, _)) = crate::invol_spline::parse_preset_token(profile) {
        return std;
    }
    crate::invol_spline::SplineStd::GB
}

/// 解析命令行参数：`<m> <z> [h=..|alpha=..|ha=..|c=..|beta=..|x=..] [view 名] [at x,y] [rot 度]`
#[derive(Debug)]
pub struct GearRequest {
    pub params: GearParams,
    pub view: GearView,
    pub at: Option<[f64; 2]>,
    pub rotation: f64,
}

/// 命令行/HTTP 参数解析（人侧 GUI 与 AI 侧共用同一套键名）。
pub fn parse_request(raw: &str) -> Result<GearRequest, String> {
    let usage = crate::i18n::t("cmd.gear.usage");
    let toks: Vec<&str> = raw.split_whitespace().collect();
    if toks.is_empty() {
        return Err(usage);
    }
    let mut p = GearParams { h: 0.0, ..GearParams::default() };
    let mut h_given = false;
    let mut view: Option<GearView> = None;
    let mut at: Option<[f64; 2]> = None;
    let mut rotation = 0.0;
    let mut positional = 0usize;
    let mut i = 0usize;
    // 花键模式（`花键`/`mode=spline`）与花键专用输入；提供给/未提供分清楚：
    // `d_B + m` 与 `d_B + z` 两种给法靠“没给的那个是 None”区分。
    let mut spline_on = false;
    let mut sp_std: Option<crate::invol_spline::SplineStd> = None;
    let mut sp_profile = String::new();
    let mut sp_d_b: Option<f64> = None;
    let mut sp_pitch: Option<f64> = None; // ANSI 径节 P（`P8`/`pitch=8`；兼容 m 槽位）
    let mut sp_hf: Option<f64> = None;
    let mut sp_rho: Option<f64> = None;
    let mut sp_cf: Option<f64> = None;
    // 齿轮体系（显式标识）：M = 模数制（默认），DP = 径节制（`DP8`/`dp=8`）。
    let mut gear_std: Option<GearStd> = None;
    let mut gear_dp: Option<f64> = None;
    let mut m_given = false;
    let mut z_given = false;
    let mut alpha_given = false;
    let mut ha_given = false;
    let mut x_given = false;
    // 位置参数缓存：齿轮 M = [m,z,h]；齿轮 DP = [z,h]（`DP` 值走标识/键）；花键 = [m,z,h]。
    // 体系标识可能出现在位置参数之后，所以先缓存、解析完再按体系回填。
    let mut pos_vals: Vec<f64> = Vec::new();
    // 花键模式识别：预设代号（GB30R 等）、体系标识（GB/DIN/NF/ANSI）或裸 `GB`/`DIN`
    let mut spline_spec_token = |tok: &str, spline_on: &mut bool, sp_std: &mut Option<crate::invol_spline::SplineStd>, sp_profile: &mut String| -> Result<bool, String> {
        if let Some((std, profile)) = crate::invol_spline::parse_preset_token(tok) {
            set_spline_std_once(sp_std, std)?;
            *spline_on = true;
            *sp_profile = profile.to_string();
            return Ok(true);
        }
        Ok(false)
    };
    while i < toks.len() {
        let t = toks[i];
        let (key, val) = t.split_once('=').map(|(a, b)| (a, Some(b))).unwrap_or((t, None));
        let mut num = |key: &str, val: Option<&str>| -> Result<f64, String> {
            let v = match val {
                Some(v) => v.to_string(),
                None => {
                    i += 1;
                    toks.get(i).map(|s| s.to_string()).unwrap_or_default()
                }
            };
            v.trim()
            .parse::<f64>()
            .map_err(|_| crate::i18n::t_fmt("cmd.gear.err.num_expected", &[("key", key), ("v", &v)]))
        };
        match key.to_ascii_lowercase().as_str() {
            "m" | "模数" => {
                // 裸 `M`（后面不是数字）= **模数制体系标识**；`M3`/`m=3`/`m 3` 仍是模数。
                let next_numeric = val.is_some()
                    || toks
                        .get(i + 1)
                        .is_some_and(|s| s.trim().parse::<f64>().is_ok());
                if val.is_none() && !next_numeric {
                    set_gear_std_once(&mut gear_std, GearStd::M)?;
                } else {
                    let text = match val {
                        Some(v) => v.to_string(),
                        None => {
                            i += 1;
                            toks.get(i).map(|s| s.to_string()).unwrap_or_default()
                        }
                    };
                    p.m = text
                        .trim()
                        .parse::<f64>()
                        .map_err(|_| crate::i18n::t_fmt("cmd.gear.err.m_expected", &[("text", &text)]))?;
                    m_given = true;
                }
            }
            "z" | "齿数" => {
                let v = num("z", val)?;
                p.z = v.round() as u32;
                z_given = true;
            }
            "mode" | "模式" => {
                let v = match val {
                    Some(v) => v.to_string(),
                    None => {
                        i += 1;
                        toks.get(i).map(|s| s.to_string()).unwrap_or_default()
                    }
                };
                let lv = v.trim().to_ascii_lowercase();
                if matches!(lv.as_str(), "spline" | "花键" | "invol" | "involute") {
                    spline_on = true;
                } else if matches!(lv.as_str(), "gear" | "齿轮") {
                    spline_on = false;
                } else {
                    return Err(crate::i18n::t_fmt("cmd.gear.err.mode_unknown", &[("v", &v)]));
                }
            }
            "std" | "standard" | "标准" | "标准号" | "体系" | "system" => {
                let v = match val {
                    Some(v) => v.to_string(),
                    None => {
                        i += 1;
                        toks.get(i).map(|s| s.to_string()).unwrap_or_default()
                    }
                };
                // M/DP = 齿轮体系；GB/DIN/NF/ANSI = 花键体系。同一行两个 → `set_*_once` 报错。
                if let Some(gs) = GearStd::parse(&v) {
                    set_gear_std_once(&mut gear_std, gs)?;
                } else {
                    let s = SplineOpts::parse_std(&v)?;
                    set_spline_std_once(&mut sp_std, s)?;
                }
                // 不自动切花键模式：模式由 `花键`/`mode=spline` 显式开（齿轮模式给花键标准号要报错）。
            }
            "dp" | "径节" | "径节制" => {
                let v = num("dp", val)?;
                if !(v.is_finite() && v > 0.0) {
                    return Err(crate::i18n::t_fmt("cmd.gear.err.dp_positive", &[("v", &trim(v))]));
                }
                set_gear_std_once(&mut gear_std, GearStd::DP)?;
                gear_dp = Some(v);
            }
            "profile" | "齿廓" | "齿廓类型" => {
                let v = match val {
                    Some(v) => v.to_string(),
                    None => {
                        i += 1;
                        toks.get(i).map(|s| s.to_string()).unwrap_or_default()
                    }
                };
                sp_profile = v;
            }
            "db" | "d_b" | "基准直径" => {
                sp_d_b = Some(num("db", val)?);
            }
            "p" | "pitch" | "径节P" => {
                let raw = val.ok_or_else(|| {
                    crate::i18n::t_fmt(
                        "cmd.gear.err.p_pitch_missing",
                        &[("form", &crate::invol_spline::ansi_pitch_form_msg())],
                    )
                })?;
                sp_pitch = Some(crate::invol_spline::parse_ansi_pitch(raw)?);
            }
            "a" | "公称直径" => {
                sp_d_b = Some(num("公称直径 A", val)?);
            }
            "hf" | "hf*" | "齿根高" => {
                sp_hf = Some(num("hf", val)?);
            }
            "rho" | "ρf" | "ρf*" | "齿根圆角" => {
                sp_rho = Some(num("rho", val)?);
            }
            "cf" | "cf*" | "齿形裕度" => {
                sp_cf = Some(num("cf", val)?);
            }
            "h" | "厚度" => {
                p.h = num("h", val)?;
                h_given = true;
            }
            "alpha" | "α" | "压力角" => {
                p.alpha_deg = num("alpha", val)?;
                alpha_given = true;
            }
            "ha" | "ha*" => {
                p.ha = num("ha", val)?;
                ha_given = true;
            }
            "c" | "c*" | "顶隙" => p.c = num("c", val)?,
            "beta" | "β" | "螺旋角" => p.beta_deg = num("beta", val)?,
            "x" | "xn" | "变位" => {
                p.x = num("x", val)?;
                x_given = true;
            }
            "view" | "视图" => {
                // 支持 `view=front`、`view front`、`view=剖视图`
                let name = match val {
                    Some(v) => v.to_string(),
                    None => {
                        i += 1;
                        toks.get(i).map(|s| s.to_string()).unwrap_or_default()
                    }
                };
                view = Some(GearView::parse(&name)?);
            }
            "at" => {
                let pair = match val {
                    Some(v) => v.to_string(),
                    None => {
                        // `at x,y` 或 `at x y`
                        i += 1;
                        let a = toks.get(i).map(|s| s.to_string()).unwrap_or_default();
                        if a.contains(',') || a.contains('，') {
                            a
                        } else {
                            i += 1;
                            format!("{},{}", a, toks.get(i).cloned().unwrap_or_default())
                        }
                    }
                };
                let mut it = pair.split([',', '，']);
                let x = it
                    .next()
                    .unwrap_or("")
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| crate::i18n::t_fmt("cmd.gear.err.at_pair", &[("pair", &pair)]))?;
                let y = it
                    .next()
                    .unwrap_or("")
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| crate::i18n::t_fmt("cmd.gear.err.at_pair", &[("pair", &pair)]))?;
                at = Some([x, y]);
            }
            "rot" | "角度" => rotation = num("rot", val)?,
            "kind" | "type" | "种类" | "类型" => {
                let v = match val {
                    Some(v) => v.to_string(),
                    None => {
                        i += 1;
                        toks.get(i).map(|s| s.to_string()).unwrap_or_default()
                    }
                };
                p.kind = GearKind::parse(&v)
                    .ok_or_else(|| crate::i18n::t_fmt("cmd.gear.err.kind_unknown", &[("v", &v)]))?;
            }
            other => {
                // `α25` / `alpha25` 这类紧凑写法（压力角；`α=25`/`alpha=25` 走上面的匹配臂）
                let lower = other.to_ascii_lowercase();
                if let Some(rest) = other
                    .strip_prefix('α')
                    .or_else(|| lower.strip_prefix("alpha"))
                {
                    let rest = rest.strip_prefix('=').unwrap_or(rest);
                    if !rest.is_empty() {
                        p.alpha_deg = rest
                            .parse::<f64>()
                            .map_err(|_| crate::i18n::t_fmt("cmd.gear.err.alpha_expected", &[("v", &other)]))?;
                        i += 1;
                        continue;
                    }
                }
                // 位置参数：第 1 个 m、第 2 个 z、第 3 个 h；也可以是中文视图名/种类名
                if let Ok(vw) = GearView::parse(other) {
                    view = Some(vw);
                    i += 1;
                    continue;
                }
                if let Some(k) = GearKind::parse(other) {
                    p.kind = k;
                    i += 1;
                    continue;
                }
                // 花键模式开关键字 / 体系标识 / 预设代号（`花键`、`spline`、`GB`、`DIN`、`NF`、`ANSI`、`GB30R`、`DIN30`）
                let lower_tok = other.to_ascii_lowercase();
                if matches!(lower_tok.as_str(), "spline" | "花键" | "invol" | "involute") {
                    spline_on = true;
                    i += 1;
                    continue;
                }
                if spline_spec_token(other, &mut spline_on, &mut sp_std, &mut sp_profile)? {
                    i += 1;
                    continue;
                }
                // 齿轮体系标识：`DP8` 贴写（裸 `DP`/`dp 8` 走上方的 `dp` 键，裸 `M` 走 `m` 键）。
                if let Some(rest) = lower_tok
                    .strip_prefix("dp")
                    .map(|r| r.strip_prefix(['=', ':']).unwrap_or(r))
                {
                    if !rest.is_empty() {
                        let v: f64 = rest
                            .parse()
                            .map_err(|_| crate::i18n::t_fmt("cmd.gear.err.dp_expected", &[("v", &other)]))?;
                        if !(v.is_finite() && v > 0.0) {
                            return Err(crate::i18n::t_fmt("cmd.gear.err.dp_positive", &[("v", &trim(v))]));
                        }
                        set_gear_std_once(&mut gear_std, GearStd::DP)?;
                        gear_dp = Some(v);
                        i += 1;
                        continue;
                    }
                }
                // ANSI 径节贴写：`P5/10`（A/B 成对）/ `P8`（裸数字，需在 17 项系列；`P=5/10` 也收）。
                if let Some(rest) = lower_tok
                    .strip_prefix('p')
                    .map(|r| r.strip_prefix(['=', ':']).unwrap_or(r))
                {
                    if !rest.is_empty()
                        && rest.chars().next().is_some_and(|c| c.is_ascii_digit() || c == '.')
                    {
                        sp_pitch = Some(crate::invol_spline::parse_ansi_pitch(rest)?);
                        i += 1;
                        continue;
                    }
                }
                // NF 公称直径贴写：`A66` / `A=66`（`a=66` / `公称直径=66` 走上面的键臂）。
                if let Some(rest) = lower_tok
                    .strip_prefix('a')
                    .map(|r| r.strip_prefix(['=', ':']).unwrap_or(r))
                {
                    if !rest.is_empty()
                        && rest.chars().next().is_some_and(|c| c.is_ascii_digit() || c == '.')
                    {
                        let v: f64 = rest
                            .parse()
                            .map_err(|_| crate::i18n::t_fmt("cmd.gear.err.a_expected", &[("v", &other)]))?;
                        if !(v.is_finite() && v > 0.0) {
                            return Err(crate::i18n::t_fmt("cmd.gear.err.a_positive", &[("v", &trim(v))]));
                        }
                        sp_d_b = Some(v);
                        i += 1;
                        continue;
                    }
                }
                let v = other
                    .parse::<f64>()
                    .map_err(|_| {
                    crate::i18n::t_fmt(
                        "cmd.gear.err.unknown_param",
                        &[("arg", other), ("usage", &usage)],
                    )
                })?;
                if positional >= 3 {
                    return Err(crate::i18n::t_fmt(
                    "cmd.gear.err.extra_param",
                    &[("arg", other), ("usage", &usage)],
                ));
                }
                // 位置参数缓存：语义按体系在后文回填（M = [m,z,h]，DP = [z,h]）。
                pos_vals.push(v);
                positional += 1;
            }
        }
        i += 1;
    }
    if positional < 2 && (raw.contains("m=") || raw.contains("z=")) == false && toks.len() < 2 {
        return Err(crate::i18n::t_fmt(
                "cmd.gear.err.need_m_z",
                &[("usage", &usage)],
            ));
    }
    // ── 同一行两个体系 → 报错（体系只能写一个）；随后按体系回填位置参数 ──
    let any_gear_id = gear_std.is_some() || gear_dp.is_some();
    if sp_std.is_some() && any_gear_id {
        return Err(crate::i18n::t_fmt("cmd.gear.err.two_spline_stds", &[("v0", &sp_std.map(|s| s.label()).unwrap_or("GB")), ("v1", &gear_std.map(|g| g.label()).unwrap_or("DP"))]));
    }
    if spline_on {
        if any_gear_id {
            return Err(crate::i18n::t_fmt("cmd.gear.err.gear_spline_std_conflict", &[("v0", &gear_std.map(|g| g.label()).unwrap_or("DP"))]));
        }
        // 花键位置参数 = [m, z, h]（与旧行为一致）。
        if let Some(v) = pos_vals.first() {
            if !m_given {
                p.m = *v;
                m_given = true;
            }
        }
        if let Some(v) = pos_vals.get(1) {
            if !z_given {
                p.z = v.round() as u32;
                z_given = true;
            }
        }
        if let Some(v) = pos_vals.get(2) {
            p.h = *v;
            h_given = true;
        }
        if sp_std.is_none() && sp_profile.is_empty() {
            // 只写了 `花键` 没写体系/代号：默认 GB 默认齿廓（与 GUI 一致）。
            sp_std = Some(crate::invol_spline::SplineStd::GB);
        }
        let std = sp_std.unwrap_or_else(|| default_spline_std(&sp_profile));
        // 径节 P/Ps 只属 ANSI B92.1：GB/DIN/NF 收到 P/DP/pitch → 明确拒绝（不静默丢参）。
        if sp_pitch.is_some() && std != crate::invol_spline::SplineStd::ANSI {
            return Err(crate::i18n::t_fmt(
                "cmd.gear.err.system_pitch",
                &[("code", std.code()), ("msg", &crate::invol_spline::pitch_only_ansi_msg())],
            ));
        }
        // ANSI：`P8`/`pitch=8` 优先；否则位置参数第 1 个就是径节 P（与 m 槽位同义）。
        let ansi_pitch = if std == crate::invol_spline::SplineStd::ANSI {
            sp_pitch.or_else(|| m_given.then_some(p.m))
        } else {
            None
        };
        if let Some(pv) = ansi_pitch {
            // ANSI 的有效模数 m = 25.4/P（mm；供 h 默认值与块名换算用）。
            p.m = 25.4 / pv;
        }
        let opts = SplineOpts {
            std,
            profile: sp_profile,
            d_b: sp_d_b,
            m: if std == crate::invol_spline::SplineStd::ANSI {
                ansi_pitch
            } else {
                m_given.then_some(p.m)
            },
            z: z_given.then_some(p.z),
            x: x_given.then_some(p.x),
            alpha_deg: alpha_given.then_some(p.alpha_deg),
            ha_star: ha_given.then_some(p.ha),
            hf_star: sp_hf,
            rho_star: sp_rho,
            c_f_star: sp_cf,
        };
        p.spline = Some(opts);
        // 花键模式下 `c`/`beta` 不参与几何；β 由 spline_engine 报错拦截。
    } else {
        if sp_std.is_some() {
            return Err(gear_mode_std_error());
        }
        if sp_d_b.is_some() {
            return Err(gear_mode_db_error());
        }
        if !sp_profile.is_empty()
            || sp_hf.is_some()
            || sp_rho.is_some()
            || sp_cf.is_some()
            || sp_pitch.is_some()
        {
            return Err(crate::i18n::t("cmd.gear.err.spline_params_in_gear"));
        }
        // 齿轮体系落地：DP → **位置参数 = [z, h]**（DP 值走 `DP8`/`dp=8`），m = 25.4/DP；
        // M → 位置参数 = [m, z, h]（与旧行为一致）。
        let is_dp = gear_std == Some(GearStd::DP) || gear_dp.is_some();
        if is_dp {
            if gear_std == Some(GearStd::M) {
                return Err(m_dp_conflict_msg());
            }
            let dp = gear_dp.ok_or_else(|| crate::i18n::t("cmd.gear.err.dp_missing_cli"))?;
            if !(dp.is_finite() && dp > 0.0) {
                return Err(crate::i18n::t_fmt("cmd.gear.err.dp_positive", &[("v", &trim(dp))]));
            }
            if m_given {
                return Err(crate::i18n::t("cmd.gear.err.no_m_in_dp"));
            }
            if pos_vals.len() > 2 {
                return Err(crate::i18n::t_fmt(
                    "cmd.gear.err.dp_pos_count",
                    &[("n", &(pos_vals.len() - 2).to_string()), ("usage", &usage)],
                ));
            }
            if let Some(v) = pos_vals.first() {
                if !z_given {
                    p.z = v.round() as u32;
                    z_given = true;
                }
            }
            if let Some(v) = pos_vals.get(1) {
                p.h = *v;
                h_given = true;
            }
            p.std = GearStd::DP;
            p.dp = Some(dp);
            p.m = m_from_dp(dp);
        } else {
            if let Some(v) = pos_vals.first() {
                if !m_given {
                    p.m = *v;
                    m_given = true;
                }
            }
            if let Some(v) = pos_vals.get(1) {
                if !z_given {
                    p.z = v.round() as u32;
                    z_given = true;
                }
            }
            if let Some(v) = pos_vals.get(2) {
                p.h = *v;
                h_given = true;
            }
            p.std = gear_std.unwrap_or(GearStd::M);
        }
    }
    if !h_given {
        // 模板里 h 是用户给的：外齿轮模板 h=20（m=2，即 10m）、内齿轮模板 h=30（m=2，即 15m）
        p.h = if p.kind.is_internal() {
            (15.0 * p.m).max(1.0)
        } else {
            (10.0 * p.m).max(1.0)
        };
    }
    if view.is_none() {
        // 中文位置参数形式：`OCSMGEAR 2 40 20 剖视图`
        for t in &toks {
            if let Ok(vw) = GearView::parse(t) {
                view = Some(vw);
                break;
            }
        }
    }
    // 花键模式：入口就把 d_B/GB/系数/查表问题拦出来（不拖到出图阶段）。
    if p.is_spline() {
        p.spline_engine()?;
    }
    Ok(GearRequest {
        params: p,
        view: view.unwrap_or(GearView::Section),
        at,
        rotation,
    })
}

// ─────────────────────────── SVG 预览（GUI 用）─────────────────

/// GUI 预览：`/api/gear_svg?m=2&z=40&h=20&alpha=20&ha=1&c=0.25&beta=0&x=0&view=section&n=1`
pub fn preview_svg(query: &str) -> Result<String, String> {
    let (p, view, n) = params_from_query(query)?;
    let part = generate(&p, view, n)?;
    Ok(svg_of(&part.entities, 460.0))
}

/// 从查询串解析参数（`/api/gear_svg` 与 `/api/gear_info` 共用）。
pub fn params_from_query(query: &str) -> Result<(GearParams, GearView, f64), String> {
    let get = |k: &str| {
        query
            .split('&')
            .filter_map(|kv| kv.split_once('='))
            .find(|(a, _)| *a == k)
            .map(|(_, v)| v.to_string())
    };
    let f = |k: &str, d: f64| get(k).and_then(|v| v.parse::<f64>().ok()).unwrap_or(d);
    let opt_f = |k: &str| get(k).and_then(|v| v.trim().parse::<f64>().ok());
    let kind = match get("kind") {
        Some(v) => GearKind::parse(&v)
            .ok_or_else(|| crate::i18n::t_fmt("cmd.gear.err.kind_unknown_query", &[("v", &v)]))?,
        None => GearKind::External,
    };
    let view = GearView::parse(&get("view").unwrap_or_else(|| "section".into()))?;
    let n = f("n", 1.0);
    let mode = get("mode").unwrap_or_default();
    let is_spline = matches!(
        mode.trim().to_ascii_lowercase().as_str(),
        "spline" | "花键" | "invol" | "involute"
    );
    let p = if is_spline {
        let std_raw = get("std");
        let std = match std_raw.as_deref() {
            Some(v) if !v.trim().is_empty() => {
                if let Some(gs) = GearStd::parse(v) {
                    return Err(format!(
                        "花键模式不认齿轮体系 `{}`（M = 模数制 / DP = 径节制）；花键体系可用 GB / DIN / NF / ANSI。",
                        gs.label()
                    ));
                }
                SplineOpts::parse_std(v)?
            }
            _ => default_spline_std(&get("profile").unwrap_or_default()),
        };
        // 径节 P/Ps 只属 ANSI B92.1：GB/DIN/NF 收到 p/pitch → 明确拒绝（不静默丢参）。
        if std != crate::invol_spline::SplineStd::ANSI
            && get("p").or_else(|| get("pitch")).is_some_and(|v| !v.trim().is_empty())
        {
            return Err(crate::i18n::t_fmt(
                "cmd.gear.err.system_pitch",
                &[("code", std.code()), ("msg", &crate::invol_spline::pitch_only_ansi_msg())],
            ));
        }
        let m_or_p = if std == crate::invol_spline::SplineStd::ANSI {
            // `p`/`pitch` 支持 A/B 成对写法（`p=2.5/5`）；`m` 槽位仍兼容数值 P。
            match get("p").or_else(|| get("pitch")) {
                Some(v) if !v.trim().is_empty() => {
                    Some(crate::invol_spline::parse_ansi_pitch(&v)?)
                }
                _ => opt_f("m"),
            }
        } else {
            opt_f("m")
        };
        let opts = SplineOpts {
            std,
            profile: get("profile").unwrap_or_default(),
            d_b: opt_f("db").or_else(|| opt_f("d_b")).or_else(|| opt_f("a")).or_else(|| opt_f("A")),
            m: m_or_p,
            z: opt_f("z").map(|v| v.round() as u32),
            x: opt_f("x"),
            alpha_deg: opt_f("alpha"),
            ha_star: opt_f("ha"),
            hf_star: opt_f("hf"),
            rho_star: opt_f("rho"),
            c_f_star: opt_f("cf"),
        };
        // ANSI：第 5 参 m 槽位 = 径节 P；GearParams.m 存换算后 mm 模数 25.4/P（与 parse_request 一致）。
        let m_or_p = opts.m.unwrap_or(2.0);
        let m = if std == crate::invol_spline::SplineStd::ANSI {
            crate::invol_spline::ANSI_INCH_MM / m_or_p
        } else {
            m_or_p
        };
        GearParams {
            kind,
            m,
            z: opts.z.unwrap_or(40),
            alpha_deg: opts.alpha_deg.unwrap_or(ALPHA_N_DEG),
            ha: opts.ha_star.unwrap_or(1.0),
            c: 0.25,
            beta_deg: f("beta", 0.0),
            h: f("h", 10.0 * m),
            x: opts.x.unwrap_or(0.0),
            spline: Some(opts),
            std: GearStd::M,
            dp: None,
        }
    } else {
        // 体系：`std=M|DP`；`dp=<值>` 本身也能定 DP 体系（M 与 DP 同给 → 报错）。
        let std_raw = get("std").filter(|v| !v.trim().is_empty());
        let mut gear_std = match std_raw.as_deref() {
            Some(v) => GearStd::parse(v).ok_or_else(gear_mode_std_error)?,
            None => GearStd::M,
        };
        let dp_in = opt_f("dp").or_else(|| opt_f("径节"));
        if dp_in.is_some() {
            if std_raw.is_some() && gear_std == GearStd::M {
                return Err(m_dp_conflict_msg());
            }
            gear_std = GearStd::DP;
        }
        if get("db").is_some() || get("d_b").is_some() {
            return Err(gear_mode_db_error());
        }
        if get("profile").is_some()
            || get("hf").is_some()
            || get("rho").is_some()
            || get("cf").is_some()
        {
            return Err(crate::i18n::t("cmd.gear.err.spline_params_in_gear_query"));
        }
        if get("p").or_else(|| get("pitch")).is_some_and(|v| !v.trim().is_empty()) {
            return Err(crate::i18n::t("cmd.gear.err.p_in_gear"));
        }
        let (m, dp_value) = if gear_std == GearStd::DP {
            let dp = dp_in.ok_or_else(|| crate::i18n::t("cmd.gear.err.dp_missing_query"))?;
            if !(dp.is_finite() && dp > 0.0) {
                return Err(crate::i18n::t_fmt("cmd.gear.err.dp_positive", &[("v", &trim(dp))]));
            }
            if get("m").is_some() {
                return Err(crate::i18n::t("cmd.gear.err.no_m_in_dp"));
            }
            (m_from_dp(dp), Some(dp))
        } else {
            (f("m", 2.0), None)
        };
        GearParams {
            kind,
            m,
            z: f("z", 40.0).round() as u32,
            alpha_deg: f("alpha", ALPHA_N_DEG),
            ha: f("ha", 1.0),
            c: f("c", 0.25),
            beta_deg: f("beta", 0.0),
            h: f("h", if kind.is_internal() { 15.0 * m } else { 20.0 }),
            x: f("x", 0.0),
            spline: None,
            std: gear_std,
            dp: dp_value,
        }
    };
    // 花键模式：在查询解析阶段就把 `d_B`/GB/系数问题拦出来（报错文案与出图/信息行一致）。
    if is_spline {
        p.spline_engine()?;
    }
    Ok((p, view, n))
}

/// 齿轮模式给标准号的报错（不静默忽略；与花键模式 `std` 区分开）。
///
/// 用户定案文案：**模数制齿轮不使用标准号与基准直径**（GB/T 3478.1 / DIN 5480 /
/// NF E22-141 / ANSI B92.1 都是花键体系）。
pub fn gear_mode_std_error() -> String {
    crate::i18n::t("cmd.gear.err.std_mode")
}

/// 齿轮模式给 `d_B` 的报错（用户定案文案：模数制齿轮不使用标准号与基准直径）。
pub fn gear_mode_db_error() -> String {
    crate::i18n::t("cmd.gear.err.db_mode")
}

/// `/api/gear_info`：派生尺寸 + 提示（GUI 信息行用）。花键模式走 [`spline_info_json`]。
pub fn info_json(query: &str) -> Result<String, String> {
    let (p, view, _) = params_from_query(query)?;
    if p.is_spline() {
        return spline_info_json(&p, view);
    }
    let err = p.validate().err();
    Ok(serde_json::json!({
        "ok": err.is_none(),
        "error": err,
        "mode": "gear",
        "std": p.std.label(),
        "dp": p.dp,
        "m": round4(p.m),
        "kind": p.kind.key(),
        "kind_label": p.kind.label(),
        "view": view.key(),
        "view_label": view.label_for(p.kind),
        "d": round4(p.d()),
        "da": round4(p.da()),
        "df": round4(p.df()),
        "db": round4(p.db()),
        "mt": round4(p.mt()),
        "alpha": round4(p.alpha_deg),
        "alpha_t": round4(p.alpha_t().to_degrees()),
        "rho": round4(p.rho()),
        "fillet_center_r": round4(p.fillet_center_radius()),
        "internal_tip_below_base": p.internal_tip_falls_below_base(),
        "chamfer": p.chamfer(),
        "pitch_angle": round4(p.pitch_angle().to_degrees()),
        // 统一齿形表达式（齿轮/花键共用；齿轮生成器两种模式都输出它，见 `tooth_expr`）。
        "expr": gear_tooth_expr(&p),
        "half_tooth_angle_tip": round4(p.half_tooth_angle(p.da() / 2.0).to_degrees()),
        "helical": p.is_helical(),
        "block": block_name(&p, view),
        "spec": p.spec(),
        "notes": p.notes(),
    })
    .to_string())
}

/// 花键模式的 `/api/gear_info`：引擎派生值 + d_B 来源 + 提示。失败也返回 JSON（GUI 显示红字）。
fn spline_info_json(p: &GearParams, view: GearView) -> Result<String, String> {
    let (engine, origin) = match p.spline_engine() {
        Ok(v) => v,
        Err(e) => {
            return Ok(serde_json::json!({
                "ok": false,
                "error": e,
                "mode": "spline",
                "kind": p.kind.key(),
                "kind_label": p.spline_kind_label(),
                "view": view.key(),
                "view_label": view.label_for_spline(p.kind),
            })
            .to_string())
        }
    };
    let (da, df) = if engine.internal {
        (engine.internal_major_dia(), engine.internal_minor_dia())
    } else {
        (engine.da(), engine.df())
    };
    // DIN 花键：能算就算检验尺寸（M2 = 内花键棒间距）——不阻断 info，仅作附加信息。
    let inspection = if engine.std == crate::invol_spline::SplineStd::DIN {
        engine
            .d_b
            .and_then(|db| crate::invol_spline::inspection_query(db, engine.m, engine.z).ok())
            .map(|r| crate::invol_spline::inspection_summary(&r))
    } else {
        None
    };
    Ok(serde_json::json!({
        "ok": true,
        "error": null,
        "mode": "spline",
        "kind": p.kind.key(),
        "kind_label": p.spline_kind_label(),
        "view": view.key(),
        "view_label": view.label_for_spline(p.kind),
        "std": engine.std.label(),
        "std_code": engine.std.code(),
        "profile": engine.profile,
        "m": engine.m,
        "z": engine.z,
        "x": round4(engine.x),
        "alpha": round4(engine.alpha_deg),
        "d": round4(engine.d()),
        "db": round4(engine.db()),
        "da": round4(da),
        "df": round4(df),
        "d_b": engine.d_b.map(round4),
        "a": engine.a.map(round4),
        "origin": origin.as_ref().map(|o| o.note()),
        "rho": round4(engine.rho_f()),
        "cf": round4(engine.c_f()),
        // ANSI 专用（其余体系为 null）：P/Ps、DFe/DFi、Sv min。
        "pitch": if engine.std == crate::invol_spline::SplineStd::ANSI { Some(round4(engine.ansi_p())) } else { None },
        "ps": if engine.std == crate::invol_spline::SplineStd::ANSI { Some(round4(2.0 * engine.ansi_p())) } else { None },
        // 径节原始 A/B 写法（P/Ps；与 spec()/块名的原值口径一致，避免同 m 不同来源串块）。
        "pitch_label": if engine.std == crate::invol_spline::SplineStd::ANSI {
            Some(format!("{}/{}", trim(engine.ansi_p()), trim(2.0 * engine.ansi_p())))
        } else { None },
        "dfe": if engine.std == crate::invol_spline::SplineStd::ANSI { Some(round4(engine.ansi_form_dia_external())) } else { None },
        "dfi": if engine.std == crate::invol_spline::SplineStd::ANSI { Some(round4(engine.ansi_form_dia_internal())) } else { None },
        "sv_min": if engine.std == crate::invol_spline::SplineStd::ANSI { Some(round4(engine.ansi_sv_min())) } else { None },
        "internal_major": round4(engine.internal_major_dia()),
        "internal_minor": round4(engine.internal_minor_dia()),
        // 统一齿形表达式（花键模式也走同一套；DA/DF 由引擎各体系公式算出）。
        "expr": spline_tooth_expr(&engine, p.kind, p.h),
        "inspection": inspection,
        "block": p.spline_block_name(&engine, view),
        "spec": p.spline_spec(&engine, origin.as_ref()),
        "notes": p.spline_notes(&engine),
    })
    .to_string())
}

fn round4(v: f64) -> f64 {
    (v * 1e4).round() / 1e4
}

// ─────────────────────── 统一齿形表达式（齿轮 / 花键共用） ───────────────────────

/// 统一齿形段表达式：`MARK KIND M… Z… ALPHA… X… DA… DF… BETA… H…`。
///
/// * `MARK` = `GEAR`（齿轮）/ `SPLINE`（渐开线花键）—— **画法开关**：决定轴生成器
///   **常规侧视图**是否画内侧直径的青色细实线（`2细线层`；外齿 = 齿根圆 df，内齿 = 里侧齿顶圆）。
///   因此它必须随 DSL/JSON/表达式往返保留。
/// * `KIND` = `EX`/`IN`；`DA`/`DF` 恒为**大径/小径**（外齿 = 齿顶/齿根；内齿 = 外侧齿根/里侧齿顶）——
///   与 `/api/gear_info` 的 `da`/`df` 同口径。
/// * `BETA` 轴段当前只支持 0（斜齿未实现，非 0 由轴侧明确报错）。
/// * `H` 是轴段尺寸（齿轮模式 = 厚度 h，花键模式 = 有效长度 L），必须保留，否则粘贴不成段。
pub fn tooth_expr(
    mark: &str,
    kind: GearKind,
    m: f64,
    z: u32,
    alpha_deg: f64,
    x: f64,
    major: f64,
    minor: f64,
    beta_deg: f64,
    h: f64,
) -> String {
    format!(
        "{} {} M{} Z{} ALPHA{} X{} DA{} DF{} BETA{} H{}",
        mark,
        if kind.is_internal() { "IN" } else { "EX" },
        trim(m),
        z,
        trim(alpha_deg),
        trim(x),
        trim(major),
        trim(minor),
        trim(beta_deg),
        trim(h)
    )
}

/// 齿轮模式的统一表达式（内齿时 DA/DF 按大径/小径转换：DA = df、DF = da）。
pub fn gear_tooth_expr(p: &GearParams) -> String {
    let (major, minor) = if p.kind.is_internal() {
        (p.df(), p.da())
    } else {
        (p.da(), p.df())
    };
    tooth_expr("GEAR", p.kind, p.m, p.z, p.alpha_deg, p.x, major, minor, p.beta_deg, p.h)
}

/// 花键模式的统一表达式（DA/DF 用引擎口径：外花键 da/df；内花键大径 D_ei / 小径 D_ii）。
pub fn spline_tooth_expr(e: &crate::invol_spline::InvolParams, kind: GearKind, h: f64) -> String {
    let (major, minor) = if e.internal {
        (e.internal_major_dia(), e.internal_minor_dia())
    } else {
        (e.da(), e.df())
    };
    tooth_expr("SPLINE", kind, e.m, e.z, e.alpha_deg, e.x, major, minor, 0.0, h)
}

/// 把图元渲染成 SVG（预览用，仅支持 LINE/CIRCLE/ARC/SPLINE/HATCH 边界）。
/// `pub(crate)`：轴生成器（`shaft.rs`）的 `/api/shaft_preview` 也走这一份渲染口径。
pub(crate) fn svg_of(entities: &[EntityType], size: f64) -> String {
    let bb = bbox_of(entities);
    let (w, h) = (bb[2] - bb[0], bb[3] - bb[1]);
    let span = w.max(h).max(1.0) * 1.12;
    let s = size / span;
    let cx = (bb[0] + bb[2]) / 2.0;
    let cy = (bb[1] + bb[3]) / 2.0;
    let tx = |x: f64| size / 2.0 + (x - cx) * s;
    let ty = |y: f64| size / 2.0 - (y - cy) * s;
    let mut body = String::new();
    let color = |layer: &str| match layer {
        LAYER_MAIN => "#e8e8e8",
        LAYER_THIN => "#5aa0ff",
        LAYER_CENTER => "#ff5555",
        LAYER_HATCH => "#3fa13f",
        _ => "#9a9a9a",
    };
    let width = |layer: &str| if layer == LAYER_MAIN { 1.5 } else { 0.9 };
    for e in entities {
        match e {
            EntityType::Line(l) => {
                body.push_str(&format!(
                    "<line x1='{:.2}' y1='{:.2}' x2='{:.2}' y2='{:.2}' stroke='{}' stroke-width='{}'/>\n",
                    tx(l.start.x), ty(l.start.y), tx(l.end.x), ty(l.end.y),
                    color(&l.common.layer), width(&l.common.layer)
                ));
            }
            EntityType::Circle(c) => {
                body.push_str(&format!(
                    "<circle cx='{:.2}' cy='{:.2}' r='{:.2}' fill='none' stroke='{}' stroke-width='{}'/>\n",
                    tx(c.center.x), ty(c.center.y), c.radius * s,
                    color(&c.common.layer), width(&c.common.layer)
                ));
            }
            EntityType::Arc(a) => {
                let (r, x, y) = (a.radius, a.center.x, a.center.y);
                let (mut a0, mut a1) = (a.start_angle.to_degrees(), a.end_angle.to_degrees());
                while a1 < a0 {
                    a1 += 360.0;
                }
                let (sx, sy) = (x + r * a0.to_radians().cos(), y + r * a0.to_radians().sin());
                let (ex, ey) = (x + r * a1.to_radians().cos(), y + r * a1.to_radians().sin());
                let large = if (a1 - a0) > 180.0 { 1 } else { 0 };
                // SVG 的 y 轴向下 → CCW 变 CW：sweep-flag = 0
                body.push_str(&format!(
                    "<path d='M {:.2} {:.2} A {:.2} {:.2} 0 {} 0 {:.2} {:.2}' fill='none' stroke='{}' stroke-width='{}'/>\n",
                    tx(sx), ty(sy), r * s, r * s, large, tx(ex), ty(ey),
                    color(&a.common.layer), width(&a.common.layer)
                ));
            }
            EntityType::Spline(sp) => {
                if sp.control_points.is_empty() {
                    continue;
                }
                let mut d = String::new();
                for (i, q) in sp.control_points.iter().enumerate() {
                    d.push_str(&format!(
                        "{}{:.2} {:.2} ",
                        if i == 0 { "M " } else { "L " },
                        tx(q.x),
                        ty(q.y)
                    ));
                }
                body.push_str(&format!(
                    "<path d='{}' fill='none' stroke='{}' stroke-width='{}' stroke-linecap='round'/>\n",
                    d.trim_end(),
                    color(&sp.common.layer),
                    width(&sp.common.layer)
                ));
            }
            EntityType::Hatch(hh) => {
                // 预览与出图同源：`partgen::hatch_svg_lines` 直接读图案定义（与宿主同一读法），
                // 不再另写 45°/3.175 的第二套口径。
                body.push_str(&hatch_svg_lines(hh, &tx, &ty, color(LAYER_HATCH), 0.7));
            }
            _ => {}
        }
    }
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{size}' height='{size}' viewBox='0 0 {size} {size}'>\
         <rect width='{size}' height='{size}' fill='#111'/>\n{body}</svg>"
    )
}

// ─────────────────────── 计算书（命令入口 `OCSMGEAR … report`） ───────────────────────

/// 从 `OCSMGEAR` 原始参数里摘出计算书关键字：`REPORT`（stdout）与
/// `REPORT=<path>` / `REPORT-OUT=<path>`（可选写文件；大小写不敏感）。
///
/// 返回 `(去掉报告关键字后的参数, 是否请求报告, 报告输出路径)`。
/// 没有报告关键字时原样返回（不重排空白/换行，轴段 DSL/JSON 不受影响）；
/// 轴段 DSL 多行文本保留行结构（每行只丢 `report` token），不会把段拼错。
/// 命令入口：`OCSMGEAR … report` / `OCSMSHAFT INVOLSPLINE … report`。
pub fn split_report_args(raw: &str) -> (String, bool, Option<String>) {
    let mut want = false;
    let mut out_path: Option<String> = None;
    // 先扫描：没有报告关键字就原样返回，避免动 JSON/DSL 的任何空白。
    for tok in raw.split_whitespace() {
        let lower = tok.to_ascii_lowercase();
        if lower == "report" || lower.starts_with("report=") || lower.starts_with("report-out=") {
            want = true;
        }
    }
    if !want {
        return (raw.to_string(), false, None);
    }
    let mut lines: Vec<String> = Vec::new();
    for line in raw.lines() {
        let mut kept: Vec<&str> = Vec::new();
        for tok in line.split_whitespace() {
            let lower = tok.to_ascii_lowercase();
            if lower == "report" {
                continue;
            }
            if lower.starts_with("report=") || lower.starts_with("report-out=") {
                if let Some((_, v)) = tok.split_once('=') {
                    if !v.is_empty() {
                        out_path = Some(v.to_string());
                    }
                }
                continue;
            }
            kept.push(tok);
        }
        lines.push(kept.join(" "));
    }
    (lines.join("\n"), true, out_path)
}

/// 计算书步骤表一行（列：步骤/公式/代入/结果/依据来源）。
fn report_row(
    n: &mut usize,
    name: &str,
    formula: &str,
    subst: &str,
    result: &str,
    source: &str,
) -> String {
    *n += 1;
    format!(
        "| {} | {} | {} | {} | {} | {} |\n",
        *n, name, formula, subst, result, source
    )
}

/// 普通齿轮（M/DP，外/内）计算书：输入参数 → 逐步计算（公式+代入+结果+来源）→
/// 派生几何 → 数据来源与校验（恒等式残差）。纯数据、无 IO。
fn gear_report(p: &GearParams) -> String {
    let mut md = String::new();
    md.push_str(&crate::i18n::t_fmt(
        "cmd.gear.report.title",
        &[
            ("kind", &kind_label(p.kind)),
            ("std", &crate::i18n::t(if p.std == GearStd::DP {
                "cmd.gear.report.std_dp"
            } else {
                "cmd.gear.report.std_m"
            })),
        ],
    ));
    md.push_str(&crate::i18n::t("cmd.gear.report.units"));

    md.push_str(&crate::i18n::t("cmd.gear.report.h_inputs"));
    if p.std == GearStd::DP {
        md.push_str(&crate::i18n::t_fmt(
            "cmd.gear.report.row_dp",
            &[("v", &trim(p.dp.unwrap_or(0.0)))],
        ));
        md.push_str(&crate::i18n::t_fmt("cmd.gear.report.row_m_conv", &[("v", &trim(p.m))]));
    } else {
        md.push_str(&crate::i18n::t_fmt("cmd.gear.report.row_m", &[("v", &trim(p.m))]));
    }
    md.push_str(&crate::i18n::t_fmt("cmd.gear.report.row_z", &[("v", &p.z.to_string())]));
    md.push_str(&crate::i18n::t_fmt("cmd.gear.report.row_alpha", &[("v", &trim(p.alpha_deg))]));
    md.push_str(&crate::i18n::t_fmt(
        "cmd.gear.report.row_hcx",
        &[("ha", &trim(p.ha)), ("c", &trim(p.c)), ("x", &trim(p.x))],
    ));
    md.push_str(&crate::i18n::t_fmt("cmd.gear.report.row_beta", &[("v", &trim(p.beta_deg))]));
    md.push_str(&crate::i18n::t_fmt("cmd.gear.report.row_h", &[("v", &trim(p.h))]));

    md.push_str(&crate::i18n::t("cmd.gear.report.h_steps"));
    md.push_str(&crate::i18n::t("cmd.gear.report.rows_head"));
    md.push_str("|---|---|---|---|---|---|\n");
    let mut n = 0usize;
    let mut steps = String::new();
    {
        let mut s = |name: &str, formula: &str, subst: &str, result: &str, source: &str| {
            steps.push_str(&report_row(&mut n, name, formula, subst, result, source));
        };
        if p.std == GearStd::DP {
            s(
                &crate::i18n::t("cmd.gear.report.step.dp"),
                &crate::i18n::t("cmd.gear.report.formula.dp"),
                &format!("25.4 / {}", trim(p.dp.unwrap_or(0.0))),
                &format!("{} mm", trim(p.m)),
                &crate::i18n::t("cmd.gear.report.source.dp"),
            );
        }
        if p.is_helical() {
            s(
                &crate::i18n::t("cmd.gear.report.step.face"),
                &crate::i18n::t("cmd.gear.report.formula.face"),
                &crate::i18n::t_fmt(
                    "cmd.gear.report.subst.face",
                    &[("m", &trim(p.m)), ("beta", &trim(p.beta_deg)), ("alpha", &trim(p.alpha_deg))],
                ),
                &crate::i18n::t_fmt(
                    "cmd.gear.report.result.face",
                    &[("mt", &trim(p.mt())), ("at", &trim(p.alpha_t().to_degrees()))],
                ),
                &crate::i18n::t("cmd.gear.report.source.face"),
            );
        } else {
            s(
                &crate::i18n::t("cmd.gear.report.step.face_spur"),
                &crate::i18n::t("cmd.gear.report.formula.face_spur"),
                &crate::i18n::t_fmt(
                    "cmd.gear.report.subst.face_spur",
                    &[("m", &trim(p.m)), ("alpha", &trim(p.alpha_deg))],
                ),
                &crate::i18n::t_fmt(
                    "cmd.gear.report.result.face",
                    &[("mt", &trim(p.mt())), ("at", &trim(p.alpha_t().to_degrees()))],
                ),
                &crate::i18n::t("cmd.gear.report.source.face_spur"),
            );
        }
        s(
            &crate::i18n::t("cmd.gear.report.step.d"),
            &crate::i18n::t("cmd.gear.report.formula.d"),
            &format!("{} × {}", trim(p.mt()), p.z),
            &format!("{} mm", trim(p.d())),
            &crate::i18n::t("cmd.gear.report.source.geom"),
        );
        s(
            &crate::i18n::t("cmd.gear.report.step.db"),
            &crate::i18n::t("cmd.gear.report.formula.db"),
            &format!("{} × cos {}°", trim(p.d()), trim(p.alpha_t().to_degrees())),
            &format!("{} mm", trim(p.db())),
            &crate::i18n::t("cmd.gear.report.source.db"),
        );
        s(
            &crate::i18n::t("cmd.gear.report.step.ha"),
            &crate::i18n::t("cmd.gear.report.formula.ha"),
            &format!("{} × ({} + {})", trim(p.m), trim(p.ha), trim(p.x)),
            &format!("{} mm", trim(p.ha_height())),
            &crate::i18n::t("cmd.gear.report.source.ha"),
        );
        s(
            &crate::i18n::t("cmd.gear.report.step.hf"),
            &crate::i18n::t("cmd.gear.report.formula.hf"),
            &format!(
                "{} × ({} + {} − {})",
                trim(p.m),
                trim(p.ha),
                trim(p.c),
                trim(p.x)
            ),
            &format!("{} mm", trim(p.hf_height())),
            &crate::i18n::t("cmd.gear.report.source.hf"),
        );
        s(
            &crate::i18n::t("cmd.gear.report.step.da"),
            &crate::i18n::t(if p.kind.is_internal() {
                "cmd.gear.report.formula.da_int"
            } else {
                "cmd.gear.report.formula.da_ext"
            }),
            &format!(
                "{} {} 2×{}",
                trim(p.d()),
                if p.kind.is_internal() { "−" } else { "+" },
                trim(p.ha_height())
            ),
            &format!("{} mm", trim(p.da())),
            &crate::i18n::t("cmd.gear.report.source.geom"),
        );
        s(
            &crate::i18n::t("cmd.gear.report.step.df"),
            &crate::i18n::t(if p.kind.is_internal() {
                "cmd.gear.report.formula.df_int"
            } else {
                "cmd.gear.report.formula.df_ext"
            }),
            &format!(
                "{} {} 2×{}",
                trim(p.d()),
                if p.kind.is_internal() { "+" } else { "−" },
                trim(p.hf_height())
            ),
            &format!("{} mm", trim(p.df())),
            &crate::i18n::t("cmd.gear.report.source.geom"),
        );
        s(
            &crate::i18n::t("cmd.gear.report.step.st"),
            &crate::i18n::t("cmd.gear.report.formula.st"),
            &format!(
                "π×{}/(2cos{}°) + 2×{}×{}×tan{}°",
                trim(p.m),
                trim(p.beta_deg),
                trim(p.x),
                trim(p.m),
                trim(p.alpha_deg)
            ),
            &format!("{} mm", trim(p.st())),
            &crate::i18n::t("cmd.gear.report.source.st"),
        );
        s(
            &crate::i18n::t("cmd.gear.report.step.rho"),
            &crate::i18n::t("cmd.gear.report.formula.rho"),
            &format!("0.38 × {}", trim(p.m)),
            &format!("{} mm", trim(p.rho())),
            &crate::i18n::t("cmd.gear.report.source.rho"),
        );
        s(
            &crate::i18n::t("cmd.gear.report.step.chamfer"),
            &crate::i18n::t("cmd.gear.report.formula.chamfer"),
            &format!("round(0.6 × {})", trim(p.m)),
            &format!("{} mm", trim(p.chamfer())),
            &crate::i18n::t("cmd.gear.report.source.chamfer"),
        );
    }
    md.push_str(&steps);
    md.push('\n');

    md.push_str(&crate::i18n::t("cmd.gear.report.h_derived"));
    md.push_str(&crate::i18n::t_fmt("cmd.gear.report.row_d", &[("v", &trim(p.d()))]));
    md.push_str(&crate::i18n::t_fmt("cmd.gear.report.row_db", &[("v", &trim(p.db()))]));
    md.push_str(&crate::i18n::t_fmt("cmd.gear.report.row_da", &[("v", &trim(p.da()))]));
    md.push_str(&crate::i18n::t_fmt("cmd.gear.report.row_df", &[("v", &trim(p.df()))]));
    md.push_str(&crate::i18n::t_fmt("cmd.gear.report.row_mt", &[("v", &trim(p.mt()))]));
    md.push_str(&crate::i18n::t_fmt(
        "cmd.gear.report.row_alpha_t",
        &[("v", &trim(p.alpha_t().to_degrees()))],
    ));
    md.push_str(&crate::i18n::t_fmt(
        "cmd.gear.report.row_pitch_angle",
        &[("v", &trim(p.pitch_angle().to_degrees()))],
    ));
    md.push('\n');

    md.push_str(&crate::i18n::t("cmd.gear.report.h_checks"));
    md.push_str(&crate::i18n::t("cmd.gear.report.no_checks"));

    md.push_str(&crate::i18n::t("cmd.gear.report.h_sources"));
    md.push_str(&crate::i18n::t("cmd.gear.report.source_engine"));
    if p.std == GearStd::DP {
        md.push_str(&crate::i18n::t_fmt(
            "cmd.gear.report.source_dp",
            &[("m", &trim(p.m)), ("dp", &trim(p.dp.unwrap_or(0.0)))],
        ));
    }
    let res_d = (p.d() - p.mt() * p.z as f64).abs();
    let res_db = (p.db() - p.d() * p.alpha_t().cos()).abs();
    md.push_str(&crate::i18n::t_fmt(
        "cmd.gear.report.identity",
        &[("r1", &format!("{:.2e}", res_d)), ("r2", &format!("{:.2e}", res_db))],
    ));
    for note in p.notes() {
        md.push_str(&crate::i18n::t_fmt("cmd.gear.report.note_line", &[("note", &note)]));
    }
    md
}

/// **齿轮/花键计算书入口**（纯数据、无 IO）：
/// * 花键模式 → 复用 [`crate::invol_spline::build_report`]（含 d_B/A 来源与 DIN 检验尺寸）；
/// * 普通齿轮（M/DP、内/外）→ 本文件 [`gear_report`]。
/// 命令入口：`OCSMGEAR … report`（可选 `report=<path>` 写文件）。
pub fn build_report(p: &GearParams) -> Result<String, String> {
    if p.is_spline() {
        let (engine, origin) = p.spline_engine()?;
        return Ok(crate::invol_spline::build_report(
            &engine,
            origin.as_ref(),
            p.h,
        ));
    }
    p.validate()?;
    Ok(gear_report(p))
}

// ─────────────────────────── 测试 ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge;

    /// 用户模板（齿轮画法.dxf）的实测数字 —— 这是本文件所有断言的依据。
    fn tmpl() -> GearParams {
        GearParams {
            kind: GearKind::External,
            m: 2.0,
            z: 40,
            alpha_deg: ALPHA_N_DEG,
            ha: 1.0,
            c: 0.25,
            beta_deg: 0.0,
            h: 20.0,
            x: 0.0,
            spline: None,
            std: GearStd::M,
            dp: None,
        }
    }

    #[test]
    fn template_basic_dimensions() {
        let p = tmpl();
        p.validate().unwrap();
        assert!((p.d() - 80.0).abs() < 1e-9, "分度圆 80");
        assert!((p.da() - 84.0).abs() < 1e-9, "齿顶圆 84");
        assert!((p.df() - 75.0).abs() < 1e-9, "齿根圆 75");
        assert!((p.rho() - 0.76).abs() < 1e-9, "齿根圆角 0.38m = 0.76");
        assert!((p.chamfer() - 1.0).abs() < 1e-9, "倒角 round(0.6×2) = 1");
        assert!((p.pitch_angle().to_degrees() - 9.0).abs() < 1e-9, "齿距角 9°");
    }

    /// 公法线跨距：直齿外齿轮有值（模板 m2 z40 α20 → k=5、W≈27.6896）；内齿轮/斜齿轮标缺。
    #[test]
    fn span_measurement_spur_external_only() {
        let p = tmpl();
        let (k, w) = p.span_measurement().unwrap();
        assert_eq!(k, 5, "k = round(40×20/180 + 0.5)");
        // 独立手算：inv20° = tan20° − 20°（弧度）
        let a = 20f64.to_radians();
        let want = 2.0 * a.cos() * (4.5 * std::f64::consts::PI + 40.0 * (a.tan() - a));
        assert!((w - want).abs() < 1e-12, "W={w} vs {want}");
        assert!((w - 27.6896).abs() < 5e-4, "模板档 W≈27.6896：{w}");
        // 变位项 = +2·x·m·sinαt
        let mut ps = tmpl();
        ps.x = 0.5;
        let (_, wx) = ps.span_measurement().unwrap();
        assert!((wx - w - 2.0 * 0.5 * 2.0 * a.sin()).abs() < 1e-12, "变位项");
        // 内齿轮 / 斜齿轮：无口径 → None（卡片如实标缺）
        let mut pi = tmpl();
        pi.kind = GearKind::Internal;
        assert!(pi.span_measurement().is_none(), "内齿轮无公法线测量");
        let mut ph = tmpl();
        ph.beta_deg = 8.0;
        assert!(ph.span_measurement().is_none(), "斜齿轮口径未收");
    }

    #[test]
    fn chamfer_rounding_matches_rule() {
        // round(0.6m)：m=1 → 1；m=0.5 → 0（不倒角）；m=5 → 3
        for (m, want) in [(1.0, 1.0), (0.5, 0.0), (5.0, 3.0), (2.0, 1.0), (3.0, 2.0)] {
            let p = GearParams { m, z: 20, h: 20.0, ..GearParams::default() };
            assert!(
                (p.chamfer() - want).abs() < 1e-9,
                "m={} → C={} 应为 {}",
                m,
                p.chamfer(),
                want
            );
        }
    }

    #[test]
    fn flank_fit_points_match_template() {
        // 模板 5 个拟合点半径：38.1327 / 38.6908 / 39.7939 / 40.8970 / 42.0001
        // （第 1 个是齿根圆角切点，与 rb+(ra−rb)/8 差 ~7µm；其余 4 个吻合到 5e-4）
        let p = tmpl();
        let f = make_flank(&p, (p.pitch_angle() / 2.0), 1.0).unwrap();
        let want = [38.1327, 38.6908, 39.7939, 40.8969, 42.0];
        let tol = [1e-2, 2e-3, 2e-3, 2e-3, 2e-3];
        for (i, (q, (w, tl))) in f.fit.iter().zip(want.iter().zip(tol.iter())).enumerate() {
            let r = (q[0] * q[0] + q[1] * q[1]).sqrt();
            assert!((r - w).abs() < *tl, "拟合点{} 半径 {:.4} vs 模板 {:.4}", i, r, w);
        }
        assert_eq!(f.ctrl.len(), 7, "7 个控制点（精度硬约束）");
        assert_eq!(f.knots.len(), 11, "11 个节点");
    }

    #[test]
    fn flank_curve_matches_involute_and_template() {
        // 用生成的 knots/control_points 采样，与解析渐开线比：应 ≤ 3µm（模板自身约 10µm）
        let p = tmpl();
        let c = p.pitch_angle() / 2.0;
        let f = make_flank(&p, c, 1.0).unwrap();
        let (t0, t1) = (f.knots[3], f.knots[7]);
        let mut worst = 0.0f64;
        for i in 0..=60 {
            let u = t0 + (t1 - t0) * i as f64 / 60.0;
            let mut q = [0.0f64; 2];
            for (j, d) in f.ctrl.iter().enumerate() {
                let w = basis(j, 3, u, &f.knots);
                q[0] += w * d[0];
                q[1] += w * d[1];
            }
            let r = (q[0] * q[0] + q[1] * q[1]).sqrt();
            let th = q[1].atan2(q[0]);
            let want = c + p.half_tooth_angle(r);
            let dev = r * (th - want).abs();
            if dev > worst {
                worst = dev;
            }
        }
        assert!(
            worst < 1e-2,
            "齿廓与精确渐开线最大偏差 {:.5} mm（模板自身 ~0.009，要求 <0.01）",
            worst
        );
    }

    #[test]
    fn fillet_is_tangent_to_root_and_touches_flank() {
        // 模板实测：圆心半径 = rf+ρ = 38.2600；圆角终点 = 齿廓起点
        let p = tmpl();
        let f = make_flank(&p, p.pitch_angle() / 2.0, 1.0).unwrap();
        let ctr = (f.fillet_c[0] * f.fillet_c[0] + f.fillet_c[1] * f.fillet_c[1]).sqrt();
        let root = p.df() / 2.0;
        assert!((ctr - (root + p.rho())).abs() < 1e-9, "圆心半径 {:.6}", ctr);
        let d = ((f.start[0] - f.fillet_c[0]).powi(2) + (f.start[1] - f.fillet_c[1]).powi(2)).sqrt();
        assert!((d - p.rho()).abs() < 1e-9, "圆心到齿廓起点 = ρ（{:.6}）", d);
        let rr = (f.root_pt[0] * f.root_pt[0] + f.root_pt[1] * f.root_pt[1]).sqrt();
        assert!((rr - root).abs() < 1e-9, "切点在齿根圆上 {:.6}", rr);
        // 模板圆角圆心角 8.622°（齿中心 4.5°）
        let deg = f.fillet_c[1].atan2(f.fillet_c[0]).to_degrees();
        assert!((deg - 8.622).abs() < 0.02, "圆角圆心角 {:.4}° vs 模板 8.622°", deg);
    }

    #[test]
    fn front_view_entity_count_matches_template() {
        // 模板 40 齿 = 320 个实体（每齿 2 样条 + 2 半齿顶弧 + 2 圆角 + 2 半齿根弧）
        let p = tmpl();
        let v = front_regular(&p, 1.0).unwrap();
        let main = v.iter().filter(|e| layer_of(e) == LAYER_MAIN).count();
        assert_eq!(main, 320, "轮廓层实体数（模板 320）");
        assert_eq!(v.iter().filter(|e| layer_of(e) == LAYER_CENTER).count(), 3, "分度圆 + 十字");
    }

    #[test]
    fn simplified_view_has_three_circles_on_expected_layers() {
        let p = tmpl();
        let v = front_simplified(&p, 1.0).unwrap();
        let lay = |layer: &str| {
            v.iter()
                .filter_map(|e| match e {
                    EntityType::Circle(c) if c.common.layer == layer => Some(c.radius),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(lay(LAYER_MAIN), vec![42.0], "齿顶圆在轮廓层");
        assert_eq!(lay(LAYER_CENTER), vec![40.0], "分度圆在中心线层");
        assert_eq!(lay(LAYER_THIN), vec![37.5], "齿根圆在细线层");
    }

    #[test]
    fn section_view_has_two_hatch_loops_and_root_lines() {
        let p = tmpl();
        let v = section_view(&p, 1.0).unwrap();
        // 剖视图轮廓 = 8 条（端面2 + 齿顶面2 + 倒角4；**无**倒角台阶线，模板同构）
        // + 齿根线 2 条 = 10 条落在轮廓层
        assert_eq!(
            v.iter().filter(|e| layer_of(e) == LAYER_MAIN).count(),
            10,
            "8 条轮廓 + 2 条齿根线"
        );
        let hat = v
            .iter()
            .find_map(|e| match e {
                EntityType::Hatch(h) => Some(h),
                _ => None,
            })
            .expect("有剖面线");
        assert_eq!(hat.paths.len(), 2, "轴线上下一共两个边界环（模板同构）");
        assert_eq!(hat.common.layer, LAYER_HATCH);
        assert!((hat.pattern_scale - HATCH_PATTERN_SCALE).abs() < 1e-12);
        assert_eq!(hat.pattern.name, "ANSI31");
        // ① 垂距 = 3.0mm（从 offset 反算：dy = −ox·sin a + oy·cos a）；环 flags = external|outermost（同花键）。
        let ln = &hat.pattern.lines[0];
        let dy = (-ln.offset.x * ln.angle.sin() + ln.offset.y * ln.angle.cos()).abs();
        assert!((dy - HATCH_SPACING_MM).abs() < 1e-9, "剖面线垂距 {dy} ≠ 3.0mm");
        for path in &hat.paths {
            assert_eq!(path.flags.bits() & 0x11, 0x11, "flags=external|outermost（同花键）");
        }
        // 齿根线 ±37.5、长度 = h
        let root_lines = v
            .iter()
            .filter_map(|e| match e {
                EntityType::Line(l) if l.common.layer == LAYER_MAIN => Some(l),
                _ => None,
            })
            .filter(|l| (l.start.y.abs() - 37.5).abs() < 1e-9 && (l.end.y.abs() - 37.5).abs() < 1e-9)
            .count();
        assert_eq!(root_lines, 2, "上下齿根线");
    }

    /// 从任意 HATCH 反算宿主读出的垂直距离（`scene/entity.rs` 同一公式）。
    fn host_hatch_dy(e: &EntityType) -> f64 {
        let EntityType::Hatch(h) = e else {
            panic!("应为 HATCH")
        };
        let ln = &h.pattern.lines[0];
        (-ln.offset.x * ln.angle.sin() + ln.offset.y * ln.angle.cos()).abs()
    }

    /// ① 剖面线垂距 3.0mm 全体系覆盖：外齿轮 / 外花键 / 矩形花键侧剖；
    /// 内齿圈（内齿轮 + 内花键）按用户定案不剖 → 无 HATCH（保持）。
    #[test]
    fn hatch_spacing_is_3mm_across_gear_and_splines() {
        // 外齿轮剖视（用户实测旧口径 3.175×√2 = 4.4901mm 的那处）
        let g = generate(&tmpl(), GearView::Section, 1.0).unwrap();
        let gh = g
            .entities
            .iter()
            .find(|e| matches!(e, EntityType::Hatch(_)))
            .expect("外齿轮剖视有剖面线");
        assert!((host_hatch_dy(gh) - HATCH_SPACING_MM).abs() < 1e-9, "齿轮垂距");
        // 外花键剖视（gear.rs 花键模式）
        let mut p = spline_tmpl(crate::invol_spline::SplineStd::GB);
        p.spline.as_mut().unwrap().m = Some(3.0);
        p.spline.as_mut().unwrap().z = Some(20);
        let s = generate(&p, GearView::Section, 1.0).unwrap();
        let sh = s
            .entities
            .iter()
            .find(|e| matches!(e, EntityType::Hatch(_)))
            .expect("外花键剖视有剖面线");
        assert!((host_hatch_dy(sh) - HATCH_SPACING_MM).abs() < 1e-9, "花键垂距");
        // 矩形花键侧剖（spline.rs 同一口径）
        let rs = crate::spline::RectSpline::from_code("6x23x26x6", None, 30.0).unwrap();
        let rh = rs
            .section_view(30.0)
            .into_iter()
            .find(|e| matches!(e, EntityType::Hatch(_)))
            .expect("矩形花键侧剖有剖面线");
        assert!((host_hatch_dy(&rh) - HATCH_SPACING_MM).abs() < 1e-9, "矩形花键垂距");
        // 内齿圈：内齿轮 + 内花键剖视都不打剖面线
        let pi = GearParams { kind: GearKind::Internal, ..tmpl() };
        assert!(!generate(&pi, GearView::Section, 1.0)
            .unwrap()
            .entities
            .iter()
            .any(|e| matches!(e, EntityType::Hatch(_))), "内齿轮不剖");
        let psi = GearParams { kind: GearKind::Internal, ..p.clone() };
        assert!(!generate(&psi, GearView::Section, 1.0)
            .unwrap()
            .entities
            .iter()
            .any(|e| matches!(e, EntityType::Hatch(_))), "内花键不剖");
    }

    /// ① 预览 = 出图：`svg_of` 的剖面线垂距（像素换算回 mm）
    /// 与 HATCH offset 反算值一致（3.0mm）；预览直接读同一份图案定义。
    #[test]
    fn hatch_preview_spacing_matches_export() {
        let p = tmpl();
        let entities = section_view(&p, 1.0).unwrap();
        let hat = entities
            .iter()
            .find_map(|e| match e {
                EntityType::Hatch(h) => Some(h),
                _ => None,
            })
            .unwrap();
        let ln = &hat.pattern.lines[0];
        let dy = (-ln.offset.x * ln.angle.sin() + ln.offset.y * ln.angle.cos()).abs();
        assert!((dy - HATCH_SPACING_MM).abs() < 1e-9, "出图垂距 {dy}");

        // 解析预览里剖面线层（#3fa13f）的线段中点/方向，按法向投影测相邻线垂距。
        let svg = svg_of(&entities, 460.0);
        let mut mids: Vec<(f64, f64, f64, f64)> = Vec::new();
        for piece in svg.split("<line ").skip(1) {
            if !piece.contains("#3fa13f") {
                continue;
            }
            let attr = |name: &str| -> f64 {
                let k = format!("{name}=\"");
                let s = piece.find(&k).unwrap() + k.len();
                let e = piece[s..].find('"').unwrap() + s;
                piece[s..e].parse().unwrap()
            };
            let (x1, y1, x2, y2) = (attr("x1"), attr("y1"), attr("x2"), attr("y2"));
            let (dx, dyv) = (x2 - x1, y2 - y1);
            let len = dx.hypot(dyv);
            mids.push(((x1 + x2) / 2.0, (y1 + y2) / 2.0, dx / len, dyv / len));
        }
        assert!(mids.len() > 4, "预览应有剖面线线段，实得 {}", mids.len());
        let (nx, ny) = (-mids[0].3, mids[0].2); // 法向
        let mut ts: Vec<f64> = mids.iter().map(|m| m.0 * nx + m.1 * ny).collect();
        ts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // 上下两环在轴线处共线 -> 同一无限直线会出两次，坐标只保留 2 位小数；
        // 先按容差去重，再测相邻线的垂距。
        let mut uniq: Vec<f64> = Vec::new();
        for t in ts {
            if uniq.last().is_none_or(|p| (t - p).abs() > 0.05) {
                uniq.push(t);
            }
        }
        let bb = bbox_of(&entities);
        let span = (bb[2] - bb[0]).max(bb[3] - bb[1]).max(1.0) * 1.12;
        let s = 460.0 / span;
        assert!(uniq.len() > 4, "预览剖面线条数太少：{}", uniq.len());
        for w in uniq.windows(2) {
            let mm = (w[1] - w[0]) / s;
            assert!((mm - HATCH_SPACING_MM).abs() < 0.02, "预览垂距 {mm}mm ≠ 3.0mm");
        }
    }

    #[test]
    fn side_view_chamfer_geometry_matches_template() {
        // 模板：端面线 ±(ra−C)=±41 高、台阶线 ±ra=±42 高、齿顶面长 h−2C = 18
        let p = tmpl();
        let v = side_view(&p, 1.0).unwrap();
        let lines: Vec<_> = v
            .iter()
            .filter_map(|e| match e {
                EntityType::Line(l) if l.common.layer == LAYER_MAIN => Some(l),
                _ => None,
            })
            .collect();
        assert_eq!(lines.len(), 10, "轮廓 10 条线（模板侧视图同构）");
        let tips = lines
            .iter()
            .filter(|l| (l.start.y - l.end.y).abs() < 1e-9 && (l.start.y.abs() - 42.0).abs() < 1e-9)
            .count();
        assert_eq!(tips, 2, "上下齿顶面");
        assert!(
            lines.iter().any(|l| (l.start.x + 10.0).abs() < 1e-9
                && (l.start.y + 41.0).abs() < 1e-9
                && (l.end.y - 41.0).abs() < 1e-9),
            "左端面线从 −41 到 +41"
        );
    }

    #[test]
    fn helix_lines_only_for_helical_and_follow_hand_rule() {
        let base = tmpl();
        assert_eq!(
            side_view(&base, 1.0)
                .unwrap()
                .iter()
                .filter(|e| layer_of(e) == LAYER_THIN)
                .count(),
            0,
            "直齿轮侧视图没有细实线"
        );
        // 与渐开线花键的**唯一画法区别**（用户定案）：齿轮侧视图**没有**齿根圆
        // 那两条 `2细线层` 直线（花键侧视才有小径/齿根圆细线）。
        let rf = base.df() / 2.0;
        let root_thin = side_view(&base, 1.0)
            .unwrap()
            .iter()
            .any(|e| {
                matches!(e, EntityType::Line(l)
                    if l.common.layer == LAYER_THIN
                        && (l.start.y.abs() - rf).abs() < 1e-9
                        && (l.start.y - l.end.y).abs() < 1e-9)
            });
        assert!(!root_thin, "齿轮侧视图不应有 2细线层的齿根圆直线（花键才有）");
        let right = GearParams { beta_deg: 8.0, ..base.clone() };
        let v = side_view(&right, 2.0).unwrap();
        let thin: Vec<_> = v.iter().filter(|e| layer_of(e) == LAYER_THIN).collect();
        assert_eq!(thin.len(), 3, "斜齿轮三条细实线");
        // 右旋 → "/"：方向角 +82°（= 90−8）
        if let EntityType::Line(l) = thin[1] {
            let d = (l.end.y - l.start.y).atan2(l.end.x - l.start.x).to_degrees();
            assert!(d > 0.0 && (d - 82.0).abs() < 1e-6, "右旋方向角 {:.3}°", d);
        }
        // 左旋 → "\"：方向角 −82°
        let left = GearParams { beta_deg: -8.0, ..base.clone() };
        let v2 = side_view(&left, 2.0).unwrap();
        if let Some(EntityType::Line(l)) = v2.iter().find(|e| layer_of(e) == LAYER_THIN) {
            let d = (l.end.y - l.start.y).atan2(l.end.x - l.start.x).to_degrees();
            assert!(d < 0.0 && (d + 82.0).abs() < 1e-6, "左旋方向角 {:.3}°", d);
        }
        // 间距 = 5×k（k=2 → 10）
        let mid = thin[1];
        let up = thin[2];
        let (m1, m2) = (mid_line_offset(&mid), mid_line_offset(&up));
        assert!((m2 - m1 - 10.0).abs() < 1e-6, "间距 {:.3}（应 5×2=10）", m2 - m1);
    }

    fn mid_line_offset(e: &EntityType) -> f64 {
        if let EntityType::Line(l) = e {
            let d = l.end.x - l.start.x;
            let n = (l.end.y - l.start.y).atan2(d);
            // 沿线方向取投影（offset 由法向给出，这里用 mid 点的法向距离）
            let mx = (l.start.x + l.end.x) / 2.0;
            let my = (l.start.y + l.end.y) / 2.0;
            -mx * n.sin() + my * n.cos()
        } else {
            0.0
        }
    }

    #[test]
    fn centerlines_use_6n_overhang() {
        let p = tmpl();
        let v = front_simplified(&p, 2.0).unwrap();
        let ln = v
            .iter()
            .find_map(|e| match e {
                EntityType::Line(l) => Some(l),
                _ => None,
            })
            .unwrap();
        let len = ((ln.end.x - ln.start.x).powi(2) + (ln.end.y - ln.start.y).powi(2)).sqrt();
        assert!((len - (84.0 + 6.0 * 2.0)).abs() < 1e-9, "长度 = da + 6n = {}", len);
    }

    /// 小齿数回归（用户 2026-09-17 报 z=17 渲染异常）：
    /// 模板口径的圆角在齿数小时无解 → 必须降级 + 给警告，且**齿廓链不能断**。
    #[test]
    fn small_tooth_counts_fall_back_and_stay_continuous() {
        for (m, z, want) in [
            (10.0, 7u32, RootStyle::BaseCircleLine),
            (10.0, 14, RootStyle::BaseCircleLine),
            (10.0, 17, RootStyle::BaseCircleLine),
            (10.0, 40, RootStyle::TemplateArc),
            (2.0, 40, RootStyle::TemplateArc),
        ] {
            let p = GearParams { m, z, h: 20.0, ..GearParams::default() };
            assert_eq!(p.root_style(), want, "m={m} z={z} 的齿根画法");
            if want != RootStyle::TemplateArc {
                assert!(p.root_style_note().is_some(), "m={m} z={z} 应有降级警告");
            } else {
                assert!(p.root_style_note().is_none());
            }
            // 齿廓链连续性：采样点半径从 rf 单调升到 ra，且相邻点间距没有大跳（原 bug 就是这里断开）
            let pitch = p.pitch_angle();
            for sign in [1.0f64, -1.0] {
                let f = make_flank(&p, pitch / 2.0, sign).unwrap();
                let pts = flank_points(&p, &f, 12);
                let radii: Vec<f64> = pts.iter().map(|q| (q[0] * q[0] + q[1] * q[1]).sqrt()).collect();
                assert!(
                    (radii[0] - p.df() / 2.0).abs() < 1e-6,
                    "链首应在齿根圆上：{:.6} vs {:.6}",
                    radii[0],
                    p.df() / 2.0
                );
                assert!(
                    (radii[radii.len() - 1] - p.da() / 2.0).abs() < 1e-6,
                    "链尾应在齿顶圆上：{:.6} vs {:.6}",
                    radii[radii.len() - 1],
                    p.da() / 2.0
                );
                for w in radii.windows(2) {
                    assert!(w[1] >= w[0] - 1e-6, "半径应单调升：{} → {}", w[0], w[1]);
                }
                // 连续性：最大步长不该是常见步长的好几倍（原 bug 就是圆角弧与样条之间差 ~1.6mm 的断口）
                let mut steps: Vec<f64> = pts
                    .windows(2)
                    .map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt())
                    .collect();
                let mut sorted = steps.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let median = sorted[sorted.len() / 2];
                steps.sort_by(|a, b| b.partial_cmp(a).unwrap());
                let worst = steps[0];
                assert!(
                    worst < 3.0 * median,
                    "相邻采样点最大间距 {:.4}（中位 {:.4}，链应连续）",
                    worst,
                    median
                );
            }
            // 出图实体也不该有异常张角的弧
            let v = front_regular(&p, 1.0).unwrap();
            for e in &v {
                if let EntityType::Arc(a) = e {
                    let span = (a.end_angle - a.start_angle).to_degrees().rem_euclid(360.0);
                    assert!(span < 180.0, "弧张角异常 {:.2}°（r={:.3}）", span, a.radius);
                }
            }
        }
    }

    #[test]
    fn z17_fallback_adds_radial_line_and_keeps_fillet() {
        let p = GearParams { m: 10.0, z: 17, h: 20.0, ..GearParams::default() };
        let f = make_flank(&p, p.pitch_angle() / 2.0, 1.0).unwrap();
        let (from, to) = f.radial.expect("降级①应有径向直线");
        let r1 = (from[0] * from[0] + from[1] * from[1]).sqrt();
        let r2 = (to[0] * to[0] + to[1] * to[1]).sqrt();
        let r_start = p.db() / 2.0 + (p.da() / 2.0 - p.db() / 2.0) * 0.02; // 降级起点（基圆略上方）
        assert!((r1 - r_start).abs() < 1e-9, "直线从齿廓起点起（{:.3} vs {:.3}）", r1, r_start);
        assert!(
            (r2 - (p.df() / 2.0 + p.rho())).abs() < 1e-9,
            "直线到圆角弧顶（{:.3}）",
            r2
        );
        // 直线是径向的（两端同角度）
        let a1 = from[1].atan2(from[0]);
        let a2 = to[1].atan2(to[0]);
        assert!((a1 - a2).abs() < 1e-9, "径向直线");
        // 圆角弧仍保留、且不过 180°
        let span = (f.fillet_a1 - f.fillet_a0).rem_euclid(360.0);
        assert!(span < 180.0 && span > 1.0, "圆角弧张角 {:.3}°", span);
    }

    #[test]
    fn fillet_arc_span_matches_template() {
        // 模板圆角弧张角 79.79°；本实现取精确渐开线齿廓起点（比模板的生成器准 ~7µm），
        // 圆心角相应差 ~0.5°（换算到弧长 <8µm，不可见）
        let p = tmpl();
        let f = make_flank(&p, p.pitch_angle() / 2.0, 1.0).unwrap();
        let span = (f.fillet_a1 - f.fillet_a0).rem_euclid(360.0);
        assert!(span < 180.0, "圆角弧必须取短边（{:.3}°）", span);
        assert!((span - 79.79).abs() < 0.6, "圆角弧张角 {:.3}° vs 模板 79.79°", span);
    }

    /// 把四个视图导出成 CSV（世界坐标），供人工/脚本与用户模板 `齿轮画法.dxf` 叠加比对。
    ///
    /// 跑法：`cargo test -p ocs_ocsm --lib gear::tests::dump_views_csv -- --ignored --nocapture`
    ///
    /// 输出目录用**持久目录** `~/桌面/OCSM/review/`（以前写 /tmp —— 重启就没了，
    /// 2026-09-17 被清过一次，所有核对产物全丢）。
    /// 输出：`/tmp/gear_review/<view>.csv`（每行 `TYPE,layer,...`；样条已按真实曲线采样）。
    #[test]
    #[ignore]
    fn dump_views_csv() {
        // 除模板参数（m=2 z=40）外，再导两组小齿数供人工核对（用户 2026-09-17 报 z=17 异常）
        for (tag, m, z) in [("smallz17", 10.0, 17u32), ("smallz7", 10.0, 7)] {
            let p = GearParams { m, z, h: 20.0, ..GearParams::default() };
            let part = generate(&p, GearView::Front, 1.0).unwrap();
            let mut out = String::new();
            for e in &part.entities {
                match e {
                    EntityType::Line(l) => out.push_str(&format!(
                        "LINE,{},{:.6},{:.6},{:.6},{:.6}\n",
                        l.common.layer, l.start.x, l.start.y, l.end.x, l.end.y
                    )),
                    EntityType::Arc(a) => out.push_str(&format!(
                        "ARC,{},{:.6},{:.6},{:.6},{:.4},{:.4}\n",
                        a.common.layer, a.center.x, a.center.y, a.radius,
                        a.start_angle.to_degrees(), a.end_angle.to_degrees()
                    )),
                    EntityType::Spline(sp) => {
                        let (t0, t1) = (sp.knots[3], sp.knots[sp.knots.len() - 4]);
                        out.push_str(&format!("POLY,{},", sp.common.layer));
                        for i in 0..=40 {
                            let u = t0 + (t1 - t0) * i as f64 / 40.0;
                            let mut q = [0.0f64; 2];
                            for (j, d) in sp.control_points.iter().enumerate() {
                                let w = basis(j, 3, u, &sp.knots);
                                q[0] += w * d.x;
                                q[1] += w * d.y;
                            }
                            out.push_str(&format!("({:.6},{:.6})", q[0], q[1]));
                        }
                        out.push('\n');
                    }
                    _ => {}
                }
            }
            let review_dir = std::path::PathBuf::from(format!(
                "{}/桌面/OCSM/review",
                std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
            ));
            std::fs::create_dir_all(&review_dir).unwrap();
            std::fs::write(review_dir.join(format!("{tag}.csv")), out).unwrap();
            println!("已导出 {}/{tag}.csv（{} 实体）", review_dir.display(), part.entities.len());
        }
        let p = tmpl();
        // 关键数字对账（与模板实测值同列）
        let ra = p.da() / 2.0;
        println!(
            "da={:.4} df={:.4} rho={:.4} C={:.3} ψ(ra)={:.4}° ψ(r_start)={:.4}° r_start={:.4}",
            p.da(), p.df(), p.rho(), p.chamfer(),
            p.half_tooth_angle(ra).to_degrees(),
            p.half_tooth_angle(fr_start_radius(&p)).to_degrees(),
            fr_start_radius(&p)
        );
        println!("（模板：ψ(ra)=1.0371°  r_start=38.1327  ψ(r_start)=3.0019°）");
        let dir = std::path::PathBuf::from(format!(
            "{}/桌面/OCSM/review",
            std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
        ));
        std::fs::create_dir_all(&dir).unwrap();
        for view in GearView::ALL {
            let part = generate(&p, view, 1.0).unwrap();
            let mut out = String::new();
            for e in &part.entities {
                match e {
                    EntityType::Line(l) => out.push_str(&format!(
                        "LINE,{},{:.6},{:.6},{:.6},{:.6}\n",
                        l.common.layer, l.start.x, l.start.y, l.end.x, l.end.y
                    )),
                    EntityType::Circle(c) => out.push_str(&format!(
                        "CIRCLE,{},{:.6},{:.6},{:.6}\n",
                        c.common.layer, c.center.x, c.center.y, c.radius
                    )),
                    EntityType::Arc(a) => out.push_str(&format!(
                        "ARC,{},{:.6},{:.6},{:.6},{:.4},{:.4}\n",
                        a.common.layer,
                        a.center.x,
                        a.center.y,
                        a.radius,
                        a.start_angle.to_degrees(),
                        a.end_angle.to_degrees()
                    )),
                    EntityType::Spline(sp) => {
                        // 按真实曲线采样（不是控制多边形！）
                        let (t0, t1) = (sp.knots[3], sp.knots[sp.knots.len() - 4]);
                        out.push_str(&format!("POLY,{},", sp.common.layer));
                        for i in 0..=40 {
                            let u = t0 + (t1 - t0) * i as f64 / 40.0;
                            let mut q = [0.0f64; 2];
                            for (j, d) in sp.control_points.iter().enumerate() {
                                let w = basis(j, 3, u, &sp.knots);
                                q[0] += w * d.x;
                                q[1] += w * d.y;
                            }
                            out.push_str(&format!("({:.6},{:.6})", q[0], q[1]));
                        }
                        out.push('\n');
                    }
                    EntityType::Hatch(hh) => {
                        out.push_str(&format!("HATCH,{},{}\n", hh.common.layer, hh.paths.len()));
                        for (pi, path) in hh.paths.iter().enumerate() {
                            out.push_str(&format!("HLOOP,{},,{},", hh.common.layer, pi));
                            for edge in &path.edges {
                                if let BoundaryEdge::Line(le) = edge {
                                    out.push_str(&format!(
                                        "({:.6},{:.6})({:.6},{:.6})",
                                        le.start.x, le.start.y, le.end.x, le.end.y
                                    ));
                                }
                            }
                            out.push('\n');
                        }
                    }
                    _ => {}
                }
            }
            let path = dir.join(format!("{}.csv", view.key()));
            std::fs::write(&path, out).unwrap();
            println!("已导出 {}（{} 实体）", path.display(), part.entities.len());
        }
        // 内齿轮：模板件（m2 z40 h30）+ 齿顶圆低于基圆的常用小齿数（z=24 / z=17）
        for (tag, m, z, h) in [("int_m2_z40", 2.0, 40u32, 30.0), ("int_m10_z24", 10.0, 24, 60.0), ("int_m10_z17", 10.0, 17, 60.0)] {
            let p = GearParams { kind: GearKind::Internal, m, z, h, ..GearParams::default() };
            for view in [GearView::Front, GearView::Section] {
                let part = generate(&p, view, 1.0).unwrap();
                let mut out = String::new();
                for e in &part.entities {
                    match e {
                        EntityType::Line(l) => out.push_str(&format!(
                            "LINE,{},{:.6},{:.6},{:.6},{:.6}\n",
                            l.common.layer, l.start.x, l.start.y, l.end.x, l.end.y
                        )),
                        EntityType::Arc(a) => out.push_str(&format!(
                            "ARC,{},{:.6},{:.6},{:.6},{:.4},{:.4}\n",
                            a.common.layer, a.center.x, a.center.y, a.radius,
                            a.start_angle.to_degrees(), a.end_angle.to_degrees()
                        )),
                        EntityType::Spline(sp) => {
                            let (t0, t1) = (sp.knots[3], sp.knots[sp.knots.len() - 4]);
                            out.push_str(&format!("POLY,{},", sp.common.layer));
                            for i in 0..=40 {
                                let u = t0 + (t1 - t0) * i as f64 / 40.0;
                                let mut q = [0.0f64; 2];
                                for (j, d) in sp.control_points.iter().enumerate() {
                                    let w = basis(j, 3, u, &sp.knots);
                                    q[0] += w * d.x;
                                    q[1] += w * d.y;
                                }
                                out.push_str(&format!("({:.6},{:.6})", q[0], q[1]));
                            }
                            out.push('\n');
                        }
                        _ => {}
                    }
                }
                let path = dir.join(format!("{tag}_{}.csv", view.key()));
                std::fs::write(&path, out).unwrap();
                println!(
                    "已导出 {}（{} 实体，提示 {} 条）",
                    path.display(),
                    part.entities.len(),
                    p.notes().len()
                );
                for nt in p.notes() {
                    println!("    提示：{nt}");
                }
            }
        }
    }

    #[test]
    fn ocsm_ready_gate_blocks_uninitialized_drawing() {
        let doc = ocs_plugin_api::host::acadrust::CadDocument::new();
        let err = ocsm_ready(&doc).unwrap_err();
        assert!(err.contains("OCSM 初始化"), "{err}");
        assert!(err.contains("缺图层"), "应报缺哪些层：{err}");
        // 只把图层建上、但中心线层没挂点划线 → 仍拦
        let mut doc2 = ocs_plugin_api::host::acadrust::CadDocument::new();
        for name in [LAYER_MAIN, LAYER_THIN, LAYER_CENTER, LAYER_HATCH] {
            let mut ly = ocs_plugin_api::host::acadrust::tables::Layer::new(name);
            ly.line_type = "Continuous".to_string();
            doc2.layers.add_or_replace(ly);
        }
        let err2 = ocsm_ready(&doc2).unwrap_err();
        assert!(err2.contains("CENTER2"), "{err2}");
        // 挂上 CENTER2 → 放行
        let mut ly = ocs_plugin_api::host::acadrust::tables::Layer::new(LAYER_CENTER);
        ly.line_type = "CENTER2".to_string();
        doc2.layers.add_or_replace(ly);
        ocsm_ready(&doc2).unwrap();
    }

    #[test]
    fn view_parse_accepts_chinese_and_english() {
        assert_eq!(GearView::parse("剖视图").unwrap(), GearView::Section);
        assert_eq!(GearView::parse("section").unwrap(), GearView::Section);
        assert_eq!(GearView::parse("侧视图").unwrap(), GearView::Side);
        assert_eq!(GearView::parse("side").unwrap(), GearView::Side);
        assert_eq!(GearView::parse("简化").unwrap(), GearView::Simplified);
        assert_eq!(GearView::parse("simplified").unwrap(), GearView::Simplified);
        assert_eq!(GearView::parse("常规正视图").unwrap(), GearView::Front);
        assert_eq!(GearView::parse("front").unwrap(), GearView::Front);
        assert!(GearView::parse("斜视").is_err());
    }

    #[test]
    fn parse_request_positional_and_keyword_forms() {
        // 位置式：`OCSMGEAR 2 40 20 剖视图`
        let r = parse_request("2 40 20 剖视图").unwrap();
        assert_eq!(r.params.m, 2.0);
        assert_eq!(r.params.z, 40);
        assert_eq!(r.params.h, 20.0);
        assert_eq!(r.view, GearView::Section);
        // 关键字式 + at + rot
        let r = parse_request("m=2 z=40 h=20 beta=-8 x=0.2 view front at 120,45 rot 30").unwrap();
        assert!((r.params.beta_deg + 8.0).abs() < 1e-9);
        assert!((r.params.x - 0.2).abs() < 1e-9);
        assert_eq!(r.view, GearView::Front);
        assert_eq!(r.at, Some([120.0, 45.0]));
        assert!((r.rotation - 30.0).abs() < 1e-9);
        // `view 侧视图`（空格分隔）
        let r = parse_request("2 40 20 view 侧视图").unwrap();
        assert_eq!(r.view, GearView::Side);
        // 缺参数 → 报用法
        assert!(parse_request("2").is_err(), "只给模数要报用法");
        assert!(parse_request("2 40 20 斜视").is_err(), "错视图名要报");
    }

    #[test]
    fn block_name_is_sanitized_and_stable() {
        let p = tmpl();
        assert_eq!(block_name(&p, GearView::Front), "OCSM_GEAR_M2_Z40_H20_FRONT");
        let h = GearParams { beta_deg: -8.5, x: 0.3, ..p.clone() };
        let nm = block_name(&h, GearView::Side);
        assert_eq!(nm, "OCSM_GEAR_M2_Z40_B8_5L_X0_3_H20_SIDE", "{}", nm);
        let h2 = GearParams { beta_deg: 8.0, x: -0.25, ..p.clone() };
        assert_eq!(
            block_name(&h2, GearView::Front),
            "OCSM_GEAR_M2_Z40_B8R_XN0_25_H20_FRONT"
        );
        assert!(nm.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
    }

    #[test]
    fn generate_all_views_and_svg_preview() {
        let p = tmpl();
        for v in GearView::ALL {
            let part = generate(&p, v, 1.0).unwrap();
            assert!(!part.entities.is_empty(), "{} 非空", v.label());
            assert!(part.bbox[2] > part.bbox[0]);
        }
        let svg = preview_svg("m=2&z=40&h=20&view=front").unwrap();
        assert!(svg.starts_with("<svg") && svg.contains("path"));
        let svg2 = preview_svg("m=2&z=40&h=20&view=side&beta=8").unwrap();
        assert!(svg2.contains("line"), "斜齿轮侧视图有三条细实线");
    }

    #[test]
    fn helical_uses_transverse_params() {
        let p = GearParams { beta_deg: 20.0, ..tmpl() };
        assert!((p.mt() - 2.0 / 20f64.to_radians().cos()).abs() < 1e-9);
        assert!(p.alpha_t().to_degrees() > 20.0, "端面压力角大于法向");
        assert!(p.d() > 80.0, "斜齿轮分度圆更大（mt·z）");
        p.validate().unwrap();
        let _ = p.notes(); // 斜齿轮提示
    }

    #[test]
    fn validate_rejects_bad_params() {
        assert!(GearParams { m: 0.0, ..GearParams::default() }.validate().is_err());
        assert!(GearParams { z: 1, ..GearParams::default() }.validate().is_err());
        assert!(GearParams { h: -1.0, ..GearParams::default() }.validate().is_err());
        assert!(GearParams { beta_deg: 60.0, ..GearParams::default() }.validate().is_err());
        // 倒角比厚度还大 → 拒绝
        assert!(GearParams { m: 10.0, h: 6.0, ..GearParams::default() }.validate().is_err());
    }

    #[test]
    fn solid_weight_is_plausible() {
        // m=2 z=40 h=20 实心钢件：面积 ≈ π·ra² − 齿槽 ≈ 4920 mm² → ≈ 0.77 kg
        let p = tmpl();
        let w: f64 = solid_weight_kg(&p).parse().unwrap();
        assert!(w > 0.70 && w < 0.90, "毛坯质量 {:.3} kg 不合理（期望 0.77 左右）", w);
    }

    // ─────────────────────────── 压力角 α（新增参数）───────────────────────────

    /// 图元几何指纹（FNV-1a 64；喂 Debug 文本）—— 给默认 20° 冻结回归用。
    fn entity_fingerprint(entities: &[EntityType]) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for e in entities {
            for b in format!("{:?}", e).as_bytes() {
                h ^= *b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        h
    }

    /// 默认 20° 回归锁（用户要求）：
    /// * 旧入口（位置参数 / JSON 默认 / `GearParams::default()`）都不传 α，必须还是 20°；
    /// * 显式 `alpha 20` 与不传 α 生成的图元**逐字节一致**；
    /// * 四个视图的几何指纹冻结在加 α 参数之前的值（防止默认路径被改漂）。
    #[test]
    fn default_alpha_is_20_and_geometry_is_frozen() {
        let legacy = tmpl();
        let implicit = parse_request("2 40 20").unwrap().params;
        let query = params_from_query("m=2&z=40&h=20").unwrap().0;
        for p in [&legacy, &implicit, &query] {
            assert!((p.alpha_deg - 20.0).abs() < 1e-12, "缺省压力角必须是 20°");
        }
        assert_eq!(legacy, implicit, "位置参数入口的默认值应与结构体默认一致");
        assert_eq!((legacy.m, legacy.z, legacy.alpha_deg), (query.m, query.z, query.alpha_deg));
        assert_eq!(legacy.spec(), "m2 z40 h20", "默认 spec 不该带 α");
        assert_eq!(block_name(&legacy, GearView::Front), "OCSM_GEAR_M2_Z40_H20_FRONT");
        assert!(
            !legacy.notes().iter().any(|n| n.contains("压力角") || n.contains("α")),
            "默认 20° 不应有 α 提示"
        );

        // 冻结指纹（加入 α 参数前的实测值）—— 只锁「默认 20°」这条路径。
        //
        // ⚠️ 指纹哈希的是 `format!("{:?}", 图元)`（见 `entity_fingerprint`），所以
        // **随 acadrust 的结构体字段漂移**：2026-09-21 上游把 acadrust 从 `8a28c21` 升到
        // `5b682ed`，其中 `EntityCommon` 新增 `raw_record: Option<Arc<RawRecord>>`（serde 跳过，
        // 但 **Debug 会打印**）→ 每个图元的 Debug 串都变，四个指纹全漂。
        // 已按新旧几何未变的前提下重新冻结（图元数不变：section 14 / side / simplified / front）。
        // 2026-09 用户定案：剖视剖面线口径改为 3.0mm（HATCH 图元改了 offset/scale）→ section 指纹重冻。
        // 下次升 acadrust 若再漂，先确认几何真的没变（比对图元数与关键坐标）再改这里的常数。
        let frozen: [(&str, u64); 4] = [
            ("section", 0xfccdf1d19555193c),
            ("side", 0xdb9046b86e5a3e94),
            ("simplified", 0x39e0cbb64e6195cd),
            ("front", 0xbb4f49b5f2d49d91),
        ];
        for (name, want) in frozen {
            let view = GearView::parse(name).unwrap();
            let a = generate(&legacy, view, 1.0).unwrap();
            let b = generate(&implicit, view, 1.0).unwrap();
            assert_eq!(
                format!("{:?}", a.entities),
                format!("{:?}", b.entities),
                "{name}: 不传 α 与显式 alpha 20 必须逐字节一致"
            );
            let got = entity_fingerprint(&a.entities);
            println!("FROZEN {name} = {:#018x} entities={}", got, a.entities.len());
            assert_eq!(got, want, "{name}: 默认 20° 几何指纹漂了");
        }
    }

    /// α=25° 自检：db=d·cosα、ψ(R) 解析值、齿顶弧端点、圆角切点，以及「α 改变确实改变几何」。
    #[test]
    fn alpha_25_changes_geometry_and_follows_formulas() {
        let p20 = tmpl();
        let p25 = GearParams { alpha_deg: 25.0, ..tmpl() };
        p25.validate().unwrap();
        let alpha = 25f64.to_radians();
        // 基圆 db = d·cosα（直齿 αt = αn）
        assert!((p25.alpha_t() - alpha).abs() < 1e-12, "直齿 αt 必须等于 αn");
        assert!(
            (p25.db() - p25.d() * alpha.cos()).abs() < 1e-12,
            "db={} 应为 d·cosα={}",
            p25.db(),
            p25.d() * alpha.cos()
        );
        // ψ(R) = st/(2r) + (inv αt − inv αR) 的独立解析复算
        let r = p25.d() / 2.0;
        let alpha_r = (p25.db() / 2.0 / 42.0).acos();
        let psi_want = p25.st() / (2.0 * r) + (alpha.tan() - alpha) - (alpha_r.tan() - alpha_r);
        assert!(
            (p25.half_tooth_angle(42.0) - psi_want).abs() < 1e-12,
            "ψ(42)={}，解析值 {}",
            p25.half_tooth_angle(42.0),
            psi_want
        );
        // 齿顶弧端点 = 齿中心 ± ψ(ra)（模板相位：齿中心在半个齿距处）
        let ra = p25.da() / 2.0;
        let c_deg = (p25.pitch_angle() / 2.0).to_degrees();
        let psi_end = p25.half_tooth_angle(ra).to_degrees();
        let front = front_regular(&p25, 1.0).unwrap();
        let tip_ok = front.iter().any(|e| match e {
            EntityType::Arc(a) if (a.radius - ra).abs() < 1e-9 => {
                let (s, t) = (a.start_angle.to_degrees(), a.end_angle.to_degrees());
                ((s - (c_deg - psi_end)).abs() < 1e-9 && (t - c_deg).abs() < 1e-9)
                    || ((s - c_deg).abs() < 1e-9 && (t - (c_deg + psi_end)).abs() < 1e-9)
            }
            _ => false,
        });
        assert!(tip_ok, "齿顶弧端点应 = 齿中心 ± ψ(ra)（ψ(ra)={psi_end}）");
        // 齿根圆角：圆心在 rf+ρ、切点在 rf、齿廓起点到圆心 = ρ（模板口径不因 α 变）
        let f = make_flank(&p25, p25.pitch_angle() / 2.0, 1.0).unwrap();
        let rc = (f.fillet_c[0].powi(2) + f.fillet_c[1].powi(2)).sqrt();
        assert!((rc - (p25.df() / 2.0 + p25.rho())).abs() < 1e-9, "圆角圆心半径 {rc}");
        let d_r = ((f.start[0] - f.fillet_c[0]).powi(2) + (f.start[1] - f.fillet_c[1]).powi(2)).sqrt();
        assert!((d_r - p25.rho()).abs() < 1e-9, "圆心到齿廓起点 {d_r} ≠ ρ");
        // 「α 改变确实改变几何」
        let a20 = format!("{:?}", generate(&p20, GearView::Front, 1.0).unwrap().entities);
        let a25 = format!("{:?}", generate(&p25, GearView::Front, 1.0).unwrap().entities);
        assert_ne!(a20, a25, "α=25 的齿廓必须与 20° 不同");
        assert!((p20.db() - p25.db()).abs() > 1.0, "db 应随 α 变");
        // 元数据 / 块名：非默认 α 要带上，避免与 20° 的同名块串图
        assert_eq!(p25.spec(), "m2 z40 h20 α25");
        assert_eq!(block_name(&p25, GearView::Front), "OCSM_GEAR_M2_Z40_A25_H20_FRONT");
        assert!(
            p25.notes().iter().any(|n| n.contains("不随 α 自动改变")),
            "非 20° 必须提醒系数不自动变：{:?}",
            p25.notes()
        );
        // 内齿轮同口径：db / 齿顶半角 / 端视图都随 α 变
        let i20 = int_tmpl();
        let i25 = GearParams { alpha_deg: 25.0, ..i20.clone() };
        assert!((i25.db() - i20.db()).abs() > 1.0);
        assert!((i25.internal_tip_half_angle() - i20.internal_tip_half_angle()).abs() > 1e-6);
        assert_ne!(
            format!("{:?}", front_internal(&i20, 1.0).unwrap()),
            format!("{:?}", front_internal(&i25, 1.0).unwrap()),
            "内齿轮端视图应随 α 变"
        );
    }

    /// 14.5° 老系统可用；`alpha 25` / `alpha=25` / `α25` / `α=25` 四种入口都收。
    #[test]
    fn alpha_145_system_and_compact_alpha_forms() {
        let p = GearParams { alpha_deg: 14.5, ..tmpl() };
        p.validate().unwrap();
        let a = 14.5f64.to_radians();
        assert!((p.db() - 80.0 * a.cos()).abs() < 1e-12, "db = d·cos14.5");
        // x=0 时分度圆齿厚 st = πm/2 与 α 无关；分度圆处 ψ = st/(2r) 与 α 无关（= 2.25°）
        assert!((p.st() - std::f64::consts::PI).abs() < 1e-12);
        let psi_pitch = p.half_tooth_angle(p.d() / 2.0).to_degrees();
        assert!((psi_pitch - 2.25).abs() < 1e-9, "分度圆 ψ={psi_pitch}");
        // 齿顶处 ψ 比 20° 更大（小压力角齿顶更宽、更不易变尖）
        let p20 = tmpl();
        assert!(
            p.half_tooth_angle(p.da() / 2.0) > p20.half_tooth_angle(p20.da() / 2.0),
            "14.5° 齿顶应比 20° 宽（ψ={} vs {}）",
            p.half_tooth_angle(p.da() / 2.0).to_degrees(),
            p20.half_tooth_angle(p20.da() / 2.0).to_degrees()
        );
        let _ = generate(&p, GearView::Front, 1.0).unwrap();
        // 紧凑写法 / 显式写法
        for form in ["2 40 20 alpha 25", "2 40 20 alpha=25", "2 40 20 α25", "2 40 20 α=25"] {
            let r = parse_request(form).unwrap();
            assert!((r.params.alpha_deg - 25.0).abs() < 1e-9, "{form}");
        }
        // 内齿轮入口也带 α
        let r = parse_request("int 2 40 30 alpha 25 view 端视图").unwrap();
        assert_eq!(r.params.kind, GearKind::Internal);
        assert!((r.params.alpha_deg - 25.0).abs() < 1e-9);
        assert!((r.params.h - 30.0).abs() < 1e-9);
        // HTTP 查询串
        let (q, v, _) = params_from_query("kind=internal&m=2&z=40&alpha=22.5&view=front").unwrap();
        assert_eq!(v, GearView::Front);
        assert!((q.alpha_deg - 22.5).abs() < 1e-9);
        assert!((q.db() - 80.0 * 22.5f64.to_radians().cos()).abs() < 1e-12);
        // 非法 α 全部拒绝（合理区间 10°<α<50°，30°/37.5°/45° 属合法）
        for bad in [0.0, 5.0, 10.0, 50.0, 90.0, f64::NAN] {
            assert!(
                GearParams { alpha_deg: bad, ..tmpl() }.validate().is_err(),
                "α={bad} 应拒绝"
            );
        }
        // info JSON 带 alpha / alpha_t / db
        let j = info_json("m=2&z=40&alpha=25").unwrap();
        assert!(j.contains("\"alpha\":25"), "{j}");
        assert!(j.contains("\"alpha_t\":25"), "{j}");
        assert!(j.contains("\"db\":"), "{j}");
    }

    /// α=30°/45° 自检（渐开线花键常用）：db 随 α 明显变小；30° 全齿高可用；
    /// 45° 全齿高（ha*=1）齿顶变尖 → 正视图必须**明确报错而不是出乱图**；
    /// 45° 配短齿顶（ha*=0.5）几何正常；小齿数 + 大 α 降级逻辑不许崩。
    #[test]
    fn alpha_30_and_45_geometry_self_checks() {
        // ── α=30°：db、拟合带（基圆已低于齿根圆）、齿顶弧端点、圆角口径 ──
        let p30 = GearParams { alpha_deg: 30.0, ..tmpl() };
        p30.validate().unwrap();
        let a30 = 30f64.to_radians();
        assert!(
            (p30.db() - p30.d() * a30.cos()).abs() < 1e-12,
            "db={} 应为 d·cos30={}",
            p30.db(),
            p30.d() * a30.cos()
        );
        assert!(
            p30.db() / 2.0 < p30.df() / 2.0,
            "30° 时基圆已经在齿根圆以内（大 α 特征）"
        );
        assert!((p30.flank_band_start() - p30.df() / 2.0).abs() < 1e-12, "拟合带应从齿根圆起");
        assert!(!p30.tooth_tip_crossed(), "30° 全齿高不应变尖");
        assert_eq!(p30.root_style(), RootStyle::TemplateArc, "30° 圆角应仍可解");
        let f30 = make_flank(&p30, p30.pitch_angle() / 2.0, 1.0).unwrap();
        let r_start30 = (f30.fit[0][0].powi(2) + f30.fit[0][1].powi(2)).sqrt();
        assert!(
            r_start30 >= p30.df() / 2.0 - 1e-9,
            "齿廓起点 {r_start30} 不得落进齿根圆以内"
        );
        let front30 = front_regular(&p30, 1.0).unwrap();
        let ra30 = p30.da() / 2.0;
        let c30 = (p30.pitch_angle() / 2.0).to_degrees();
        let psi30 = p30.half_tooth_angle(ra30).to_degrees();
        assert!(psi30 > 0.0 && psi30 < 4.5, "ψ30(ra)={psi30}");
        assert!(
            front30.iter().any(|e| match e {
                EntityType::Arc(a) if (a.radius - ra30).abs() < 1e-9 => {
                    let (s, t) = (a.start_angle.to_degrees(), a.end_angle.to_degrees());
                    ((s - (c30 - psi30)).abs() < 1e-9 && (t - c30).abs() < 1e-9)
                        || ((s - c30).abs() < 1e-9 && (t - (c30 + psi30)).abs() < 1e-9)
                }
                _ => false,
            }),
            "30° 齿顶弧端点应 = 齿中心 ± ψ(ra)"
        );
        assert!(
            front30
                .iter()
                .any(|e| matches!(e, EntityType::Arc(a) if (a.radius - p30.df() / 2.0).abs() < 1e-9)),
            "应有半径 = rf 的齿根弧"
        );
        // 齿根弧端点 = 齿槽中心（0°、半个齿距 4.5°），不随 α 变（模板相位同构）
        let root_ends: Vec<f64> = front30
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) if (a.radius - p30.df() / 2.0).abs() < 1e-9 => {
                    Some([a.start_angle.to_degrees(), a.end_angle.to_degrees()])
                }
                _ => None,
            })
            .flatten()
            .collect();
        let pitch_deg30 = p30.pitch_angle().to_degrees();
        assert!(
            root_ends.iter().any(|d| (d - 0.0).abs() < 1e-6)
                && root_ends.iter().any(|d| (d - pitch_deg30).abs() < 1e-6),
            "齿根弧应覆盖齿槽中心 0°/{pitch_deg30}°：{root_ends:?}"
        );

        // ── α=45° 全齿高：齿顶变尖，正视图报错、其它三视图照常 ──
        let p45 = GearParams { alpha_deg: 45.0, ..tmpl() };
        p45.validate().unwrap();
        assert!(
            (p45.db() - p45.d() * 45f64.to_radians().cos()).abs() < 1e-12,
            "db = d·cos45"
        );
        assert!(p45.tooth_tip_crossed(), "45° 全齿高（ha*=1、z=40）应判为齿顶变尖");
        assert!(p45.half_tooth_angle(p45.da() / 2.0) < 0.0, "ψ45(ra) 应为负");
        let err = front_regular(&p45, 1.0).unwrap_err();
        assert!(err.contains("齿顶变尖"), "报错要讲清原因：{err}");
        for v in [GearView::Section, GearView::Side, GearView::Simplified] {
            assert!(
                !generate(&p45, v, 1.0).unwrap().entities.is_empty(),
                "{v:?} 不该被齿顶变尖拦住"
            );
        }
        assert!(
            p45.notes().iter().any(|n| n.contains("齿顶变尖")),
            "GUI 提示要说明 45° 全齿高的问题"
        );
        assert!(
            p45.notes().iter().any(|n| n.contains("ha*=0.5")),
            "提示要给出短齿顶系数指路"
        );

        // ── α=45° + 短齿顶 ha*=0.5（工具允许的最小齿顶高）：几何正常，齿顶弧端点仍 = 齿中心 ± ψ(ra) ──
        let p45s = GearParams { alpha_deg: 45.0, ha: 0.5, ..tmpl() };
        p45s.validate().unwrap();
        assert!(!p45s.tooth_tip_crossed(), "ha*=0.5 时 45° 不应变尖");
        assert!(
            (p45s.db() - p45s.d() * 45f64.to_radians().cos()).abs() < 1e-12,
            "db 与 ha* 无关"
        );
        let front45 = front_regular(&p45s, 1.0).unwrap();
        let ra45 = p45s.da() / 2.0;
        let c45 = (p45s.pitch_angle() / 2.0).to_degrees();
        let psi45 = p45s.half_tooth_angle(ra45).to_degrees();
        assert!(psi45 > 0.0, "ψ45(ra)={psi45}");
        assert!(
            front45.iter().any(|e| match e {
                EntityType::Arc(a) if (a.radius - ra45).abs() < 1e-9 => {
                    let (s, t) = (a.start_angle.to_degrees(), a.end_angle.to_degrees());
                    ((s - (c45 - psi45)).abs() < 1e-9 && (t - c45).abs() < 1e-9)
                        || ((s - c45).abs() < 1e-9 && (t - (c45 + psi45)).abs() < 1e-9)
                }
                _ => false,
            }),
            "45°+ha*=0.5 齿顶弧端点应 = 齿中心 ± ψ(ra)"
        );
        assert!(
            front45
                .iter()
                .any(|e| matches!(e, EntityType::Arc(a) if (a.radius - p45s.df() / 2.0).abs() < 1e-9)),
            "45°+ha*=0.5 应有半径 = rf 的齿根弧"
        );

        // ── 小齿数 + 大 α：降级逻辑不许崩（m=10 z=7）──
        let small30 = GearParams { m: 10.0, z: 7, h: 20.0, alpha_deg: 30.0, ..GearParams::default() };
        small30.validate().unwrap();
        let v30 = generate(&small30, GearView::Front, 1.0).unwrap();
        for e in &v30.entities {
            if let EntityType::Arc(a) = e {
                let span = (a.end_angle - a.start_angle).to_degrees().rem_euclid(360.0);
                assert!(span < 180.0, "m10 z7 α30 弧张角异常 {span:.2}°（r={:.3}）", a.radius);
            }
        }
        let small45 = GearParams { m: 10.0, z: 7, h: 20.0, alpha_deg: 45.0, ..GearParams::default() };
        small45.validate().unwrap();
        // 45° 全齿高在 z=7 会变尖 → 正视图明确报错；不出乱图、不 panic
        if small45.tooth_tip_crossed() {
            assert!(front_regular(&small45, 1.0).is_err());
        } else {
            let _ = front_regular(&small45, 1.0).unwrap();
        }
        for v in [GearView::Section, GearView::Side, GearView::Simplified] {
            let _ = generate(&small45, v, 1.0).unwrap();
        }
        // 37.5° 全齿高在 z=40 也已临界变尖（ψ=−0.035°）→ 同样拦；说明区间不是写死的白名单
        let p375 = GearParams { alpha_deg: 37.5, ..tmpl() };
        p375.validate().unwrap();
        assert!(p375.tooth_tip_crossed());
        // 把 ha* 降低一点就能画
        let p375ok = GearParams { alpha_deg: 37.5, ha: 0.9, ..tmpl() };
        assert!(!p375ok.tooth_tip_crossed());
        let _ = front_regular(&p375ok, 1.0).unwrap();

        // ── 内齿轮同口径：30° 可出；45° 全齿高齿槽过宽 → 端视图明确报错、剖视图照常；
        //    45°+ha*=0.5 可出且弧不绕圈 ──
        let i30 = GearParams {
            kind: GearKind::Internal,
            m: 2.0,
            z: 40,
            h: 30.0,
            alpha_deg: 30.0,
            ..GearParams::default()
        };
        i30.validate().unwrap();
        assert!(!i30.internal_tooth_crossed());
        let v30 = generate(&i30, GearView::Front, 1.0).unwrap();
        for e in &v30.entities {
            if let EntityType::Arc(a) = e {
                let span = (a.end_angle - a.start_angle).to_degrees().rem_euclid(360.0);
                assert!(span < 180.0, "内齿轮 α=30 弧张角异常 {span:.2}°");
            }
        }
        let i45 = GearParams {
            kind: GearKind::Internal,
            m: 2.0,
            z: 40,
            h: 30.0,
            alpha_deg: 45.0,
            ..GearParams::default()
        };
        i45.validate().unwrap();
        assert!(i45.internal_tooth_crossed(), "45° 内齿轮全齿高应判齿槽过宽");
        let err45 = front_internal(&i45, 1.0).unwrap_err();
        assert!(err45.contains("齿槽过宽"), "内齿轮 45° 报错要讲清原因：{err45}");
        assert!(i45.notes().iter().any(|n| n.contains("齿槽过宽")));
        let _ = generate(&i45, GearView::Section, 1.0).unwrap();
        let i45s = GearParams {
            kind: GearKind::Internal,
            m: 2.0,
            z: 40,
            h: 30.0,
            alpha_deg: 45.0,
            ha: 0.5,
            ..GearParams::default()
        };
        i45s.validate().unwrap();
        assert!(!i45s.internal_tooth_crossed());
        let v45s = generate(&i45s, GearView::Front, 1.0).unwrap();
        for e in &v45s.entities {
            if let EntityType::Arc(a) = e {
                let span = (a.end_angle - a.start_angle).to_degrees().rem_euclid(360.0);
                assert!(span < 180.0, "内齿轮 α=45+ha*=0.5 弧张角异常 {span:.2}°");
            }
        }
    }

    fn layer_of(e: &EntityType) -> &str {
        match e {
            EntityType::Line(l) => &l.common.layer,
            EntityType::Circle(c) => &c.common.layer,
            EntityType::Arc(a) => &a.common.layer,
            EntityType::Spline(s) => &s.common.layer,
            EntityType::Hatch(h) => &h.common.layer,
            _ => "",
        }
    }

    // ─────────────────────────── 内齿轮（二期）───────────────────────────

    /// 用户内齿轮模板件：`~/桌面/OCSM/齿轮/内齿轮.dxf` = m2 z40 h30（da=76 df=85 ρ=0.76）。
    fn int_tmpl() -> GearParams {
        GearParams { kind: GearKind::Internal, m: 2.0, z: 40, h: 30.0, ..GearParams::default() }
    }

    #[test]
    fn internal_tmpl_diameters_match() {
        let p = int_tmpl();
        assert!((p.da() - 76.0).abs() < 1e-9, "da={}（模板 76）", p.da());
        assert!((p.df() - 85.0).abs() < 1e-9, "df={}（模板 85）", p.df());
        assert!((p.rho() - 0.76).abs() < 1e-9, "ρ=0.38m={}", p.rho());
        assert!(
            (p.fillet_center_radius() - 41.74).abs() < 1e-9,
            "圆角圆心半径 {:.4}（模板实测 41.74 = rf − ρ）",
            p.fillet_center_radius()
        );
        assert!(!p.internal_tip_falls_below_base(), "z=40：da=76 > db=75.18");
    }

    #[test]
    fn internal_space_half_angle_matches_template() {
        // 模板实测：R=42.076 处齿槽半角 0.9869°；齿顶弧半角 1.4578°（理论 1.462°）
        let p = int_tmpl();
        let psi = p.space_half_angle(42.076).to_degrees();
        assert!((psi - 0.987).abs() < 0.002, "ψ(42.076)={psi}");
        let tip = p.internal_tip_half_angle().to_degrees();
        assert!((tip - 1.462).abs() < 0.01, "齿顶半角={tip}（模板 1.4578）");
    }

    #[test]
    fn internal_fillet_solves_to_template_values() {
        // 模板实测：切点 R=42.076、圆心半径 41.74、圆心角 4.445°（齿槽中心 4.5°，偏 0.055°）
        let p = int_tmpl();
        let s = p.pitch_angle() / 2.0;
        let (r_t, f, tang) = solve_internal_fillet(&p, s, -1.0).expect("模板参数下圆角应有解");
        assert!((r_t - 42.076).abs() < 0.003, "切点半径 {r_t}（模板 42.076）");
        let fl = (f[0] * f[0] + f[1] * f[1]).sqrt();
        assert!((fl - 41.74).abs() < 1e-6, "圆心半径 {fl}");
        let a = f[1].atan2(f[0]).to_degrees().rem_euclid(360.0);
        assert!((a - 4.445).abs() < 0.01, "圆心角 {a}（模板 4.445）");
        let d = (tang[0] * tang[0] + tang[1] * tang[1]).sqrt();
        assert!((d - r_t).abs() < 1e-9, "切点应在齿廓上");
        let (_, f2, _) = solve_internal_fillet(&p, s, 1.0).unwrap();
        let a2 = f2[1].atan2(f2[0]).to_degrees().rem_euclid(360.0);
        assert!((a2 - 4.555).abs() < 0.01, "+1 侧圆心角 {a2}（模板 4.555）");
        let sd = s.to_degrees();
        assert!(((a - sd) + (a2 - sd)).abs() < 0.02, "两侧应对称");
    }

    #[test]
    fn internal_front_entity_count_and_chain_continuity() {
        let p = int_tmpl();
        let part = generate(&p, GearView::Front, 1.0).unwrap();
        let n_geo = part.entities.iter().filter(|e| !matches!(e, EntityType::Line(_))).count();
        assert_eq!(n_geo, p.z as usize * 8, "每齿 8 图元（2 样条 + 2 圆角弧 + 2 齿根弧 + 2 齿顶弧）");
        let pts = internal_chain_points(&p, 6).unwrap();
        let ra = p.da() / 2.0;
        let rf = p.df() / 2.0;
        for w in pts.windows(2) {
            let d = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
            assert!(d < 0.8, "链断了：{:.3}mm @ {:?}→{:?}", d, w[0], w[1]);
        }
        for q in &pts {
            let r = (q[0] * q[0] + q[1] * q[1]).sqrt();
            assert!(r > ra - 1e-6 && r < rf + 1e-6, "半径越界 {r}（应在 {ra}..{rf}）");
        }
    }

    #[test]
    fn internal_small_z_tip_below_base_degrades_with_warning() {
        // z ≤ 33 的内齿轮齿顶圆低于基圆（渐开线下不去）→ 简化画法：径向直线 + 提示
        for z in [33u32, 24, 17] {
            let p = GearParams { kind: GearKind::Internal, m: 10.0, z, h: 60.0, ..GearParams::default() };
            assert!(p.internal_tip_falls_below_base(), "z={z} 应低于基圆");
            let part = generate(&p, GearView::Front, 1.0).unwrap();
            let n_line = part.entities.iter().filter(|e| matches!(e, EntityType::Line(_))).count();
            assert!(n_line >= p.z as usize * 2 + 2, "z={z} 应每齿多 2 条径向直线，实得 {n_line}");
            assert!(
                p.notes().iter().any(|s| s.contains("低于基圆")),
                "z={z} 必须给提示（不能默默出图）"
            );
            // 采样要够密：模数 10 时齿廓有 ~18mm，seg=6 只有 18 个采样点（step 1.5mm），
            // 会被下面的 0.9mm 阈值误判成"断链" —— 那是采样问题，不是几何问题。
            let pts = internal_chain_points(&p, 24).unwrap();
            let ra = p.da() / 2.0;
            let rf = p.df() / 2.0;
            for (wi, w) in pts.windows(2).enumerate() {
                let d = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
                let r0 = (w[0][0] * w[0][0] + w[0][1] * w[0][1]).sqrt();
                let r1 = (w[1][0] * w[1][0] + w[1][1] * w[1][1]).sqrt();
                let a0 = w[0][1].atan2(w[0][0]).to_degrees();
                let a1 = w[1][1].atan2(w[1][0]).to_degrees();
                assert!(d < 0.9, "z={z} 链断了 @idx {wi}/{}：{d:.3}mm  r {r0:.3}→{r1:.3}  θ {a0:.3}→{a1:.3}", pts.len());
            }
            for q in &pts {
                let r = (q[0] * q[0] + q[1] * q[1]).sqrt();
                assert!(r > ra - 1e-6 && r < rf + 1e-6, "z={z} 半径越界 {r}");
            }
        }
    }

    #[test]
    fn internal_section_is_template_block_no_hatch() {
        // 用户「内齿轮使用示例.dxf」里的块 = 23 条线（20 轮廓 + 3 中心线），无剖面线
        let p = int_tmpl();
        let part = generate(&p, GearView::Section, 1.0).unwrap();
        assert_eq!(part.entities.len(), 23);
        assert!(part.entities.iter().all(|e| matches!(e, EntityType::Line(_))), "不能有 HATCH/圆弧");
        assert_eq!(
            part.entities
                .iter()
                .filter(|e| matches!(e, EntityType::Line(l) if l.common.layer == LAYER_CENTER))
                .count(),
            3,
            "分度线 ×2 + 轴线 ×1"
        );
        let rf = p.df() / 2.0;
        let roots: Vec<f64> = part
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Line(l) if (l.start.y.abs() - rf).abs() < 1e-9 => Some((l.end.x - l.start.x).abs()),
                _ => None,
            })
            .collect();
        assert_eq!(roots.len(), 2, "上下齿根线各一条");
        for len in roots {
            assert!((len - p.h).abs() < 1e-9, "齿根线长 {len} ≠ h={}", p.h);
        }
        let ra = p.da() / 2.0;
        let tips: Vec<f64> = part
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Line(l)
                    if (l.start.y.abs() - ra).abs() < 1e-9
                        && (l.end.y.abs() - ra).abs() < 1e-9 =>
                {
                    Some((l.end.x - l.start.x).abs())
                }
                _ => None,
            })
            .collect();
        assert_eq!(tips.len(), 2);
        for len in tips {
            assert!(
                (len - (p.h - 2.0 * p.chamfer())).abs() < 1e-9,
                "齿顶线长 {len}（模板 28 = h − 2C）"
            );
        }
    }

    #[test]
    fn internal_views_without_template_are_rejected() {
        let p = int_tmpl();
        for v in [GearView::Side, GearView::Simplified] {
            let e = generate(&p, v, 1.0).unwrap_err();
            assert!(e.contains("模板"), "{v:?} 的报错应说明模板没有这个视图：{e}");
        }
    }

    #[test]
    fn internal_block_name_parse_and_notes() {
        let p = int_tmpl();
        assert_eq!(block_name(&p, GearView::Front), "OCSM_GEAR_INT_M2_Z40_H30_FRONT");
        assert_eq!(block_name(&p, GearView::Section), "OCSM_GEAR_INT_M2_Z40_H30_SECTION");
        // 外齿轮块名不能变（一期兼容）
        assert_eq!(block_name(&tmpl(), GearView::Front), "OCSM_GEAR_M2_Z40_H20_FRONT");
        assert_eq!(parse_request("int 2 40 30 view 端视图").unwrap().params.kind, GearKind::Internal);
        assert_eq!(parse_request("内齿轮 2 40").unwrap().params.kind, GearKind::Internal);
        assert_eq!(parse_request("m=2 z=40 kind=internal").unwrap().params.kind, GearKind::Internal);
        assert_eq!(parse_request("2 40 20").unwrap().params.kind, GearKind::External);
        // 默认齿宽：内齿轮 15m（模板 30 = 15×2）、外齿轮 10m（模板 20 = 10×2）
        assert!((parse_request("int 2 40").unwrap().params.h - 30.0).abs() < 1e-9);
        assert!((parse_request("2 40").unwrap().params.h - 20.0).abs() < 1e-9);
        // 提示语必须告诉人"剖视图没打剖面线、齿圈外壁留给用户"
        assert!(p.notes().iter().any(|s| s.contains("剖面线") && s.contains("齿圈")));
    }

    #[test]
    fn internal_query_params_and_info() {
        let (p, v, _) = params_from_query("kind=internal&m=2&z=40&view=front").unwrap();
        assert_eq!(p.kind, GearKind::Internal);
        assert_eq!(v, GearView::Front);
        assert!((p.h - 30.0).abs() < 1e-9, "内齿轮默认齿宽 15m={}", p.h);
        let j = info_json("kind=internal&m=2&z=40").unwrap();
        assert!(j.contains("\"kind\":\"internal\""), "{j}");
        assert!(j.contains("\"da\":76"), "{j}");
        assert!(j.contains("\"df\":85"), "{j}");
    }

    // ─────────────────────── 花键模式（齿轮侧入口） ───────────────────────

    /// 花键模式参数骨架。
    fn spline_tmpl(std: crate::invol_spline::SplineStd) -> GearParams {
        GearParams {
            kind: GearKind::External,
            m: 3.0,
            z: 20,
            h: 30.0,
            spline: Some(SplineOpts { std, ..SplineOpts::default() }),
            ..GearParams::default()
        }
    }

    /// GB 下 `d_B` 必须报统一文案（齿轮花键模式、CLI、查询串）；齿轮模式给标准号/d_B 也报错。
    #[test]
    fn spline_mode_gb_rejects_d_b_and_gear_mode_rejects_spline_input() {
        // GB + d_B（走引擎）
        let mut p = spline_tmpl(crate::invol_spline::SplineStd::GB);
        if let Some(s) = p.spline.as_mut() {
            s.d_b = Some(40.0);
            s.m = Some(3.0);
            s.z = Some(20);
        }
        let e = p.spline_engine().unwrap_err();
        assert_eq!(e, crate::invol_spline::gb_d_b_msg(), "{e}");
        // GB + d_B（查询串）
        let e = params_from_query("mode=spline&std=GB&db=40&m=3&z=20").unwrap_err();
        assert_eq!(e, crate::invol_spline::gb_d_b_msg(), "{e}");
        // GB + 非零 x：CLI/查询串都走统一报错（GUI 花键模式下 x 已锁 0）。
        let e = parse_request("花键 GB30P 3 20 x=0.2 h=30").unwrap_err();
        assert!(e.contains(&crate::invol_spline::gb_x_msg()), "{e}");
        let e = params_from_query("mode=spline&std=GB&m=3&z=20&x=0.2&h=30").unwrap_err();
        assert!(e.contains(&crate::invol_spline::gb_x_msg()), "{e}");
        // GB + x=0：正常解析（x 省略与 0 等价）。
        let r = parse_request("花键 GB30P 3 20 x=0 h=30").unwrap();
        assert!(r.params.spline_engine().is_ok(), "GB x=0 应可用");
        // 齿轮模式给标准号/d_B → 明确报错（不静默忽略）
        let e = parse_request("2 40 20 std=DIN").unwrap_err();
        assert!(e.contains("齿轮模式不认标准号"), "{e}");
        let e = parse_request("2 40 20 db=40").unwrap_err();
        assert!(e.contains("齿轮模式不认基准直径"), "{e}");
        let e = params_from_query("m=2&z=40&std=DIN").unwrap_err();
        assert!(e.contains("齿轮模式不认标准号"), "{e}");
        let e = params_from_query("m=2&z=40&db=40").unwrap_err();
        assert!(e.contains("齿轮模式不认基准直径"), "{e}");
        // ANSI 可用（`m` 槽位 = 径节 P）；给 d_B 报径节制文案；NF 已入库（无 A 时由 m/z/x 正算 A）。
        let (p, _, _) = params_from_query("mode=spline&std=ANSI&m=3&z=20").unwrap();
        let sa = p.spline.as_ref().expect("ANSI 花键应该可用");
        assert_eq!(sa.std, crate::invol_spline::SplineStd::ANSI);
        let (ea, oa) = p.spline_engine().unwrap();
        assert_eq!(ea.pitch, Some(3.0));
        assert!((ea.d() / 25.4 - 20.0 / 3.0).abs() < 1e-12, "D 回英寸应 = N/P");
        assert!((ea.d() - 20.0 * 25.4 / 3.0).abs() < 1e-12, "D=N/P×25.4（mm）");
        assert!(oa.is_none());
        let e = params_from_query("mode=spline&std=ANSI&db=40&m=3&z=20").unwrap_err();
        assert_eq!(e, crate::invol_spline::ansi_d_b_msg(), "{e}");
        // ANSI 径节入口：`p=5/10`（A/B）解析为 P=5；系列外 `p=2` 只带一层前缀并列 17 项 A/B。
        let (p, _, _) = params_from_query("mode=spline&std=ANSI&p=5/10&z=20").unwrap();
        let ea = p.spline_engine().unwrap().0;
        assert_eq!(ea.ansi_p(), 5.0);
        let e = params_from_query("mode=spline&std=ANSI&p=2&z=20").unwrap_err();
        assert_eq!(e.matches("ANSI B92.1：").count(), 1, "{e}");
        assert!(e.contains("不在标准系列") && e.contains("2.5/5"), "{e}");
        // A/B 的 B != 2A → 语法报错说明 Ps=2P
        let e = params_from_query("mode=spline&std=ANSI&p=5/11&z=20").unwrap_err();
        assert!(e.contains("Ps 恒为 2P"), "{e}");
        // NF 模数只能取 NF E22-141 尺寸表实际 m 列（3.75 在表内；3.0 不是 NF 档位）。
        let (p, _, _) = params_from_query("mode=spline&std=NF&m=3.75&z=19").unwrap();
        let s = p.spline.as_ref().expect("NF 花键应该可用");
        assert_eq!(s.std, crate::invol_spline::SplineStd::NF);
        let (p, _, _) = params_from_query("mode=spline&std=NF&a=80&m=3.75&z=19").unwrap();
        let (e, _) = p.spline_engine().unwrap();
        assert_eq!((e.m, e.z), (3.75, 19));
        assert_eq!(e.a, Some(80.0), "A 主参数应回填");
        // 命令行：`花键 std=NF a=80 3.75 19`（关键字式）与 `A80` 贴写都收。
        for text in [
            "花键 std=NF a=80 3.75 19",
            "花键 std=NF A80 3.75 19",
            "花键 NFP A80 3.75 19",
        ] {
            let r = parse_request(text).unwrap_or_else(|e| panic!("{text}: {e}"));
            let (e, _) = r.params.spline_engine().unwrap_or_else(|e| panic!("{text}: {e}"));
            assert_eq!(e.a, Some(80.0), "{text}");
            assert_eq!(e.std, crate::invol_spline::SplineStd::NF, "{text}");
        }
        // A 槽位在 GB 下报统一文案；M/DP 是齿轮体系，花键模式直接报错。
        let e = params_from_query("mode=spline&std=GB&a=66&m=3&z=20").unwrap_err();
        assert_eq!(e, crate::invol_spline::gb_d_b_msg(), "{e}");
        for bad in ["M", "DP"] {
            let e = params_from_query(&format!("mode=spline&std={bad}&a=66&m=3&z=20")).unwrap_err();
            assert!(e.contains("齿轮体系"), "std={bad}：{e}");
        }
    }

    /// ANSI 径节两种来源都要能出图（后端解析口径）：`m=` 槽位（裸数字 P）与 `pitch=`（A/B 原值）。
    #[test]
    fn ansi_pitch_m_slot_and_pitch_key_both_render_svg() {
        let m_form = "mode=spline&std=ANSI&m=5&z=20&h=50&view=section";
        let pitch_form = "mode=spline&std=ANSI&pitch=5/10&z=20&h=50&view=section";
        let (pa, _, _) = params_from_query(m_form).unwrap();
        assert_eq!(pa.spline_engine().unwrap().0.ansi_p(), 5.0, "m 槽位 = 径节 P");
        let (pb, _, _) = params_from_query(pitch_form).unwrap();
        assert_eq!(pb.spline_engine().unwrap().0.ansi_p(), 5.0, "pitch=A/B 的 A 就是 P");
        assert!(preview_svg(m_form).unwrap().contains("<svg"), "m 槽位应能出图");
        assert!(
            preview_svg(pitch_form).unwrap().contains("<svg"),
            "pitch=5/10（A/B）应能出图"
        );
    }

    /// DIN 三种给法与“以 d_B 为准”：引擎已在 invol_spline 侧锁住，这里锁齿轮桥接与 meta。
    #[test]
    fn spline_mode_din_three_ways_and_adjust_note() {
        let mk = |f: &dyn Fn(&mut SplineOpts)| {
            let mut p = spline_tmpl(crate::invol_spline::SplineStd::DIN);
            f(p.spline.as_mut().unwrap());
            p
        };
        // d_B + m → 查表补 z=18、x=0.45（表值）
        let p = mk(&|s| {
            s.profile = "DIN30".into();
            s.d_b = Some(40.0);
            s.m = Some(2.0);
        });
        let (e, o) = p.spline_engine().unwrap();
        assert_eq!((e.m, e.z), (2.0, 18));
        assert!((e.x - 0.45).abs() < 1e-9);
        assert!(matches!(o.unwrap(), crate::invol_spline::D_bOrigin::Table(_)));
        assert_eq!(p.spline_block_name(&e, GearView::Front), "OCSM_SPLINE_DIN_DIN30_M2_Z18_X0_45_DB40_H30_FRONT");
        assert_eq!(p.spline_spec(&e, None), "DIN DIN30 m2 z18 x0.45 d_B40 L30");
        // d_B + z → 查表补 m=2
        let p = mk(&|s| {
            s.d_b = Some(40.0);
            s.z = Some(18);
        });
        let (e, o) = p.spline_engine().unwrap();
        assert_eq!(e.m, 2.0);
        assert!(matches!(o.unwrap(), crate::invol_spline::D_bOrigin::Table(_)));
        // m + z（无 d_B）→ d_B 由公式算出并显示，不报“不匹配”
        let p = mk(&|s| {
            s.m = Some(2.0);
            s.z = Some(18);
            s.x = Some(0.2);
        });
        let (e, o) = p.spline_engine().unwrap();
        assert!((e.d_b.unwrap() - 39.0).abs() < 1e-9);
        assert!(matches!(o.unwrap(), crate::invol_spline::D_bOrigin::Computed));
        // 三者都给且不相容 → 以 d_B 为准重算 z，提示不静默
        let p = mk(&|s| {
            s.d_b = Some(40.0);
            s.m = Some(2.0);
            s.z = Some(14);
        });
        let (e, o) = p.spline_engine().unwrap();
        assert_eq!(e.z, 18);
        let note = o.unwrap().note();
        assert!(note.contains("按基准直径 d_B=40 取 z=18") && note.contains("与输入 z=14 不符"), "{note}");
    }

    /// 花键模式出图：外花键三视图不变；内花键 = 剖视 + 端视（与内齿轮同模板，无侧视/简化正视图）。
    #[test]
    fn spline_mode_generates_views_and_internal_rules() {
        // 外花键 DIN：端视图真实渐开线、剖视有剖面线（轴线↔df 两环）
        let mut p = spline_tmpl(crate::invol_spline::SplineStd::DIN);
        p.spline.as_mut().unwrap().d_b = Some(40.0);
        p.spline.as_mut().unwrap().m = Some(2.0);
        p.h = 30.0;
        let front = generate(&p, GearView::Front, 1.0).unwrap();
        assert_eq!(front.meta.code, crate::invol_spline::DIN_CODE);
        assert_eq!(front.meta.name, "外花键（端视图）");
        assert!(front.meta.spec.contains("d_B40") && front.meta.spec.contains("查表命中 p27"));
        let section = generate(&p, GearView::Section, 1.0).unwrap();
        assert!(section.entities.iter().any(|e| matches!(e, EntityType::Hatch(_))));
        assert_eq!(block_name(&p, GearView::Section), p.spline_block_name(&p.spline_engine().unwrap().0, GearView::Section));
        // 简化正视图：花键没有这个画法
        let e = generate(&p, GearView::Simplified, 1.0).unwrap_err();
        assert!(e.contains("花键不提供"), "{e}");

        // 内花键：同一 d_B/m/z，端视图半径互换；剖视 = 内齿轮齿圈模板（23 条线，无剖面线）
        let mut pi = p.clone();
        pi.kind = GearKind::Internal;
        let (engine, _) = pi.spline_engine().unwrap();
        assert!(engine.internal);
        let ifront = generate(&pi, GearView::Front, 1.0).unwrap();
        let radii: Vec<f64> = ifront
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) => Some(a.radius),
                _ => None,
            })
            .collect();
        assert!(radii.iter().any(|r| (r - engine.internal_major_dia() / 2.0).abs() < 1e-9));
        assert!(radii.iter().any(|r| (r - engine.internal_minor_dia() / 2.0).abs() < 1e-9));
        assert!(!radii.iter().any(|r| (r - engine.da() / 2.0).abs() < 1e-9));
        let isec = generate(&pi, GearView::Section, 1.0).unwrap();
        assert_eq!(isec.entities.len(), 23, "内花键剖视 = 内齿轮同模板 23 条线");
        assert!(isec.entities.iter().all(|e| matches!(e, EntityType::Line(_))));
        assert!(!isec.entities.iter().any(|e| matches!(e, EntityType::Hatch(_))), "同内齿轮：不打剖面线");
        assert_eq!(isec.meta.name, "内花键（剖视图（齿圈内齿不剖））");
        // 侧视图/简化正视图：内花键无此视图 → 明确报错（同内齿轮风格，不出乱图）
        for v in [GearView::Side, GearView::Simplified] {
            let e = generate(&pi, v, 1.0).unwrap_err();
            assert!(e.contains("内花键不提供"), "{v:?}: {e}");
        }
        let e = generate(&pi, GearView::Side, 1.0).unwrap_err();
        assert!(e.contains("不存在侧视图") && e.contains("模板没有的画法不猜"), "{e}");
        // 薄壁内花键：孔口倒角放不下 → 同内齿轮校验口径明确报错（不出乱图）
        let mut thin = pi.clone();
        thin.h = 2.0; // m2 → C=round(0.6×2)=1，2C=2 ≥ L
        let e = generate(&thin, GearView::Section, 1.0).unwrap_err();
        assert!(e.contains("倒角") && e.contains("太大"), "{e}");
        // 可用视图集合 = 内齿轮集合（顺序一致）
        let gear_set: Vec<GearView> = GearView::ALL
            .iter()
            .copied()
            .filter(|v| v.available_for(GearKind::Internal))
            .collect();
        let spline_set: Vec<GearView> = GearView::ALL
            .iter()
            .copied()
            .filter(|v| v.available_for_spline(GearKind::Internal))
            .collect();
        assert_eq!(gear_set, spline_set);
        assert_eq!(gear_set, vec![GearView::Section, GearView::Front]);
        // 提示（不阻断）：内花键与内齿轮同口径、无侧视图
        assert!(pi
            .spline_notes(&engine)
            .iter()
            .any(|n| n.contains("内齿轮同口径") && n.contains("不存在侧视图")));
        // 外花键没有内花键提示
        assert!(!p.spline_notes(&p.spline_engine().unwrap().0).iter().any(|n| n.contains("内花键")));
    }

    /// 内花键剖视图与内齿轮剖视图**同构**：同一个 `internal_bore_section` 模板，
    /// 图元数/类型序列/图层序列逐条一致；差异只在齿形参数（D_ei/D_ii vs da/df、分度圆）。
    #[test]
    fn internal_spline_section_is_isomorphic_to_internal_gear() {
        let n = 1.0;
        // 内齿轮（模板件 m2 z40 h30）
        let pg = GearParams { kind: GearKind::Internal, m: 2.0, z: 40, h: 30.0, ..GearParams::default() };
        let gear_sec = generate(&pg, GearView::Section, n).unwrap().entities;
        let gear_tpl = internal_bore_section(
            -pg.h / 2.0,
            pg.da() / 2.0,
            pg.df() / 2.0,
            pg.d(),
            pg.h,
            pg.chamfer(),
            n,
        );
        assert_eq!(format!("{gear_sec:?}"), format!("{gear_tpl:?}"), "内齿轮剖视 = 共用模板");
        // 内花键（GB30R m2 z20 h30）：齿形参数不同，模板相同
        let ps = GearParams {
            kind: GearKind::Internal,
            m: 2.0,
            z: 20,
            h: 30.0,
            spline: Some(SplineOpts {
                std: crate::invol_spline::SplineStd::GB,
                m: Some(2.0),
                z: Some(20),
                ..SplineOpts::default()
            }),
            ..GearParams::default()
        };
        let (engine, _) = ps.spline_engine().unwrap();
        let spline_sec = generate(&ps, GearView::Section, n).unwrap().entities;
        let spline_tpl = internal_bore_section(
            0.0,
            engine.internal_tip_radius(),
            engine.internal_root_radius(),
            engine.d(),
            ps.h,
            spline_chamfer(&engine),
            n,
        );
        assert_eq!(format!("{spline_sec:?}"), format!("{spline_tpl:?}"), "内花键剖视 = 同一个共用模板");
        // 两侧同构：23 条、全 LINE、图层序列一致、都不打剖面线
        assert_eq!(gear_sec.len(), 23);
        assert_eq!(spline_sec.len(), 23);
        let kind_of = |e: &EntityType| match e {
            EntityType::Line(_) => "LINE",
            EntityType::Arc(_) => "ARC",
            EntityType::Hatch(_) => "HATCH",
            _ => "OTHER",
        };
        let layer_of = |e: &EntityType| -> String {
            match e {
                EntityType::Line(l) => l.common.layer.clone(),
                EntityType::Arc(a) => a.common.layer.clone(),
                EntityType::Hatch(h) => h.common.layer.clone(),
                _ => String::new(),
            }
        };
        assert_eq!(
            gear_sec.iter().map(kind_of).collect::<Vec<_>>(),
            spline_sec.iter().map(kind_of).collect::<Vec<_>>(),
            "图元类型序列一致"
        );
        assert_eq!(
            gear_sec.iter().map(layer_of).collect::<Vec<_>>(),
            spline_sec.iter().map(layer_of).collect::<Vec<_>>(),
            "图层序列一致"
        );
        assert!(!gear_sec.iter().any(|e| matches!(e, EntityType::Hatch(_))));
        assert!(!spline_sec.iter().any(|e| matches!(e, EntityType::Hatch(_))));
        // 内花键剖视也有分度圆两条（`3中心线层`，y=±d/2；与内齿轮同模板）。
        let pitch_lines = spline_sec
            .iter()
            .filter(|e| {
                matches!(e, EntityType::Line(l)
                    if l.common.layer == LAYER_CENTER
                        && (l.start.y.abs() - engine.d() / 2.0).abs() < 1e-9
                        && (l.start.y - l.end.y).abs() < 1e-9)
            })
            .count();
        assert_eq!(pitch_lines, 2, "内花键剖视分度圆两条");
    }

    /// ① 花键分度线两端各外延 `CENTER_OVERHANG/2×n`（=3n，与齿轮同算法）：
    /// 外花键侧视/剖视对照外齿轮 `side_view` 的 `centerline_len`，轴线与分度线同长；
    /// 内花键剖视走内齿轮共用模板，统一 = `centerline_len(L, n)` = `L + 6n`（用户 2026-09-21 定案）。
    #[test]
    fn spline_pitch_lines_extend_both_ends_like_gear() {
        let n = 2.0;
        // 外齿轮基准：分度线长度 = h + CENTER_OVERHANG×n（gear.rs::side_view 的 cl）。
        let pg = tmpl();
        let gear = side_view(&pg, n).unwrap();
        let gear_pitch: Vec<f64> = gear
            .iter()
            .filter_map(|e| match e {
                EntityType::Line(l)
                    if l.common.layer == LAYER_CENTER
                        && (l.start.y - pg.d() / 2.0).abs() < 1e-9 =>
                {
                    Some((l.end.x - l.start.x).abs())
                }
                _ => None,
            })
            .collect();
        assert_eq!(gear_pitch, vec![centerline_len(pg.h, n)], "齿轮分度线口径");

        // 外花键（GB30R m3 z20 h30）：侧视 + 剖视都套齿轮同一算法。
        let mut p = spline_tmpl(crate::invol_spline::SplineStd::GB);
        p.spline.as_mut().unwrap().m = Some(3.0);
        p.spline.as_mut().unwrap().z = Some(20);
        let (engine, _) = p.spline_engine().unwrap();
        let d2 = engine.d() / 2.0;
        let want = centerline_len(p.h, n);
        let over = CENTER_OVERHANG * n / 2.0; // 每端外延 3n
        for view in [GearView::Side, GearView::Section] {
            let part = generate(&p, view, n).unwrap();
            let (mut pitch, mut axis) = (Vec::new(), Vec::new());
            for e in &part.entities {
                if let EntityType::Line(l) = e {
                    if (l.start.y - l.end.y).abs() > 1e-9 {
                        continue;
                    }
                    let len = (l.end.x - l.start.x).abs();
                    let x0 = l.start.x.min(l.end.x);
                    if (l.start.y.abs() - d2).abs() < 1e-9 {
                        assert!((len - want).abs() < 1e-9, "{view:?} 分度线长 {len} 应为 L+6n={want}");
                        assert!((x0 + over).abs() < 1e-9, "{view:?} 分度线左端应外延 3n");
                        pitch.push(len);
                    } else if l.start.y.abs() < 1e-9 {
                        assert!((len - want).abs() < 1e-9, "{view:?} 轴线长 {len} 应与分度线同口径");
                        axis.push(len);
                    }
                }
            }
            assert_eq!((pitch.len(), axis.len()), (2, 1), "{view:?} 分度线 ×2 + 轴线 ×1");
        }

        // 内花键剖视 = 内齿轮共用模板：同一函数逐条一致；分度线/轴线统一 = L + 6n（用户 2026-09-21 定案）。
        let pi = GearParams { kind: GearKind::Internal, ..p.clone() };
        let (ei, _) = pi.spline_engine().unwrap();
        let se = generate(&pi, GearView::Section, n).unwrap().entities;
        let tpl = internal_bore_section(
            0.0,
            ei.internal_tip_radius(),
            ei.internal_root_radius(),
            ei.d(),
            pi.h,
            spline_chamfer(&ei),
            n,
        );
        assert_eq!(format!("{se:?}"), format!("{tpl:?}"), "内花键剖视 = 内齿轮共用模板");
        let want_i = centerline_len(pi.h, n);
        let mut n_i = 0;
        for e in &se {
            if let EntityType::Line(l) = e {
                if l.common.layer == LAYER_CENTER && (l.start.y - l.end.y).abs() < 1e-9 {
                    let len = (l.end.x - l.start.x).abs();
                    let x0 = l.start.x.min(l.end.x);
                    assert!((len - want_i).abs() < 1e-9, "内花键剖视中心线 = L+6n={want_i}，实得 {len}");
                    assert!((x0 + over).abs() < 1e-9, "内花键剖视中心线左端应外延 3n");
                    n_i += 1;
                }
            }
        }
        assert_eq!(n_i, 3, "内花键剖视分度线 ×2 + 轴线 ×1");
    }

    /// **四方统一**（用户 2026-09-21 定案「内外都是两端各 3n，齿轮也是，全部保持统一」）：
    /// 外齿轮 / 内齿轮 / 外花键 / 内花键的轴向视图，`3中心线层` 的分度线 ×2 + 轴线 ×1
    /// 长度都 = [`centerline_len`]`(特征长, n)` = 特征长 + 6n（两端各 3n），并用**同一个函数**断言。
    #[test]
    fn centerline_overhang_is_uniform_6n_for_all_gear_and_spline_views() {
        let n = 2.0;
        let pg = tmpl();
        let pi = int_tmpl();
        let mut pse = spline_tmpl(crate::invol_spline::SplineStd::GB);
        pse.spline.as_mut().unwrap().m = Some(3.0);
        pse.spline.as_mut().unwrap().z = Some(20);
        let psi = GearParams { kind: GearKind::Internal, ..pse.clone() };
        let cases: [(&str, &GearParams, GearView); 6] = [
            ("外齿轮侧视", &pg, GearView::Side),
            ("外齿轮剖视", &pg, GearView::Section),
            ("内齿轮剖视", &pi, GearView::Section),
            ("外花键侧视", &pse, GearView::Side),
            ("外花键剖视", &pse, GearView::Section),
            ("内花键剖视", &psi, GearView::Section),
        ];
        for (label, p, view) in cases {
            let part = generate(p, view, n).unwrap();
            let want = centerline_len(p.h, n); // 同一函数：四方共用
            // 轴类视图基点：齿轮绕齿宽中心（x=0），花键以左端面为 x=0。
            let center = if p.spline.is_some() { p.h / 2.0 } else { 0.0 };
            let mut seen = 0;
            for e in &part.entities {
                let EntityType::Line(l) = e else { continue };
                if l.common.layer != LAYER_CENTER || (l.start.y - l.end.y).abs() > 1e-9 {
                    continue;
                }
                let (a, b) = (l.start.x.min(l.end.x), l.start.x.max(l.end.x));
                assert!(
                    (b - a - want).abs() < 1e-9,
                    "{label}：中心线长 {} ≠ centerline_len(h, n)={want}",
                    b - a
                );
                assert!(
                    (a - (center - want / 2.0)).abs() < 1e-9
                        && (b - (center + want / 2.0)).abs() < 1e-9,
                    "{label}：应绕特征中心 {center} 对称、两端各 3n（实得 {a}..{b}）"
                );
                seen += 1;
            }
            assert_eq!(seen, 3, "{label}：分度线 ×2 + 轴线 ×1");
        }
    }

    /// ② GEAR 花键两端都倒角（口径照齿轮侧视图）：`C=round(0.6m)`、端面高 `ra−C`、齿顶面长 `L−2C`、
    /// 倒角斜线；台阶竖线只侧视（同齿轮）；内花键剖视按内齿顶口径由共用模板两端倒角。
    #[test]
    fn gear_spline_chamfers_both_ends() {
        let n = 1.0;
        let mut p = spline_tmpl(crate::invol_spline::SplineStd::GB); // m3 z20 h30
        p.spline.as_mut().unwrap().m = Some(3.0);
        p.spline.as_mut().unwrap().z = Some(20);
        let (engine, _) = p.spline_engine().unwrap();
        let (ra, rf) = (engine.da() / 2.0, engine.df() / 2.0);
        let c = spline_chamfer(&engine);
        assert!((c - 2.0).abs() < 1e-9, "GB m3 → C=round(0.6×3)=2，实得 {c}");
        let hi = ra - c;
        let has = |es: &[EntityType], a: [f64; 2], b: [f64; 2]| {
            es.iter().any(|e| match e {
                EntityType::Line(l) => {
                    let s = [l.start.x, l.start.y];
                    let t = [l.end.x, l.end.y];
                    let near = |q: [f64; 2], w: [f64; 2]| {
                        (q[0] - w[0]).abs() < 1e-9 && (q[1] - w[1]).abs() < 1e-9
                    };
                    (near(s, a) && near(t, b)) || (near(s, b) && near(t, a))
                }
                _ => false,
            })
        };
        for (view, steps) in [(GearView::Side, true), (GearView::Section, false)] {
            let es = generate(&p, view, n).unwrap().entities;
            // 端面高 ra−C（两端各一条）
            assert!(has(&es, [0.0, -hi], [0.0, hi]), "{view:?} 左端面高应为 ra−C");
            assert!(has(&es, [p.h, -hi], [p.h, hi]), "{view:?} 右端面高应为 ra−C");
            // 齿顶面长 L−2C
            assert!(has(&es, [c, ra], [p.h - c, ra]), "{view:?} 齿顶面长应为 L−2C");
            // 两端倒角斜线（上下各 2 条）
            assert!(
                has(&es, [0.0, hi], [c, ra]) && has(&es, [p.h - c, ra], [p.h, hi]),
                "{view:?} 上侧两端应有倒角斜线"
            );
            assert!(
                has(&es, [0.0, -hi], [c, -ra]) && has(&es, [p.h - c, -ra], [p.h, -hi]),
                "{view:?} 下侧两端应有倒角斜线"
            );
            // 台阶竖线（±ra 高）：侧视有、剖视无（同齿轮 side_view / section_view）
            let step_l = has(&es, [c, -ra], [c, ra]);
            let step_r = has(&es, [p.h - c, -ra], [p.h - c, ra]);
            assert_eq!(step_l && step_r, steps, "{view:?} 台阶竖线口径应同齿轮（侧视有、剖视无）");
            // 小径线满长（±rf），不受端部倒角影响
            assert!(
                has(&es, [0.0, rf], [p.h, rf]) && has(&es, [0.0, -rf], [p.h, -rf]),
                "{view:?} 小径线应满长"
            );
        }
        // 内花键剖视：两端孔口倒角（共用模板，内齿顶口径 D_ii/2）各断言一次。
        let pi = GearParams { kind: GearKind::Internal, ..p.clone() };
        let (ei, _) = pi.spline_engine().unwrap();
        let (ib, ic) = (ei.internal_tip_radius(), spline_chamfer(&ei));
        let es = generate(&pi, GearView::Section, n).unwrap().entities;
        for s in [1.0f64, -1.0] {
            assert!(has(&es, [0.0, s * (ib + ic)], [ic, s * ib]), "内花键左端孔口倒角（s={s}）");
            assert!(has(&es, [pi.h - ic, s * ib], [pi.h, s * (ib + ic)]), "内花键右端孔口倒角（s={s}）");
        }
    }

    /// 花键模式 CLI 解析（`OCSMGEAR 花键 …`）：预设代号隐式开模式、GB 下 d_B 统一文案、内花键 kind。
    #[test]
    fn spline_mode_cli_parse_request() {
        let req = parse_request("花键 DIN30 db=40 2 h=30 view 端视图").unwrap();
        let s = req.params.spline.as_ref().unwrap();
        assert_eq!(s.std, crate::invol_spline::SplineStd::DIN);
        assert_eq!(s.d_b, Some(40.0));
        assert_eq!(s.m, Some(2.0));
        assert_eq!(s.z, None, "CLI 没给 Z 就应是 None（由 d_B+m 查表）");
        assert_eq!(req.view, GearView::Front);
        assert_eq!(req.params.spline_engine().unwrap().0.z, 18);
        // 预设代号 / 标准名隐式开模式
        let req = parse_request("花键 GB30R 3 20 h=30 view 侧视图").unwrap();
        // 预设代号归一为齿廓名（引擎口径）；块名仍带 GB30R 代号
        assert_eq!(req.params.spline.as_ref().unwrap().profile, "30圆齿根");
        let (e, o) = req.params.spline_engine().unwrap();
        assert!(e.d_b.is_none() && o.is_none(), "GB 无 d_B（无来源）");
        assert_eq!(
            req.params.spline_block_name(&e, req.view),
            "OCSM_SPLINE_GB_GB30R_M3_Z20_H30_SIDE"
        );
        // GB + d_B（花键模式）→ 统一文案
        let e = parse_request("花键 GB30R db=40 3 20 h=30").unwrap_err();
        assert_eq!(e, crate::invol_spline::gb_d_b_msg(), "{e}");
        // 体系不支持的模数：命令行侧也明确报错并指出来源表（GUI 侧同一入口）。
        let e = parse_request("花键 GB30R 0.7 20 h=30").unwrap_err();
        assert!(e.contains("表 2") && e.contains("0.75"), "{e}");
        let e = parse_request("花键 NFP a=80 2 19 h=30").unwrap_err();
        assert!(e.contains("NF E22-141") && e.contains("尺寸表"), "{e}");
        // 内花键：复用「内齿轮」位置参数（同一套 toggle）
        let req = parse_request("花键 内花键 GB30R 3 20 h=30").unwrap();
        assert_eq!(req.params.kind, GearKind::Internal);
    }

    /// 花键模式 GUI 查询链：`/api/gear_svg` / `/api/gear_info` 的 mode/std/db 解析与 info 字段。
    #[test]
    fn spline_mode_query_info_and_preview() {
        let svg = preview_svg("mode=spline&std=GB&profile=GB30R&m=3&z=20&h=30&view=side").unwrap();
        assert!(svg.contains("<svg"), "{svg}");
        let j = info_json("mode=spline&std=GB&profile=GB30R&m=3&z=20&h=30&view=front").unwrap();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["mode"], "spline");
        assert_eq!(v["std"], "GB");
        assert_eq!(v["profile"], "30圆齿根");
        assert_eq!(v["d_b"], serde_json::Value::Null);
        assert!((v["da"].as_f64().unwrap() - 63.0).abs() < 1e-9);
        assert!((v["df"].as_f64().unwrap() - 54.6).abs() < 1e-9);
        // DIN d_B+m：info 显示补全的 z/x 与来源
        let j = info_json("mode=spline&std=DIN&profile=DIN30&db=40&m=2&h=30&view=front").unwrap();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["z"], 18);
        assert!((v["x"].as_f64().unwrap() - 0.45).abs() < 1e-9);
        assert!(v["origin"].as_str().unwrap().contains("查表命中 p27"));
        // 内花键 info：da/df 换义为 大径 D_ei / 小径 D_ii
        let j = info_json("mode=spline&std=GB&profile=GB30R&m=3&z=20&kind=internal&h=30").unwrap();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["kind_label"], "内花键");
        assert!((v["internal_major"].as_f64().unwrap() - 65.4).abs() < 1e-9);
        assert!(v["internal_minor"].as_f64().unwrap() < 60.0);
        assert!(v["notes"].as_array().unwrap().iter().any(|n| n.as_str().unwrap().contains("不存在侧视图")));
        // DIN 花键 info：附检验尺寸（M2 = 内花键棒间距）
        let j = info_json("mode=spline&std=DIN&profile=DIN30&db=6&m=0.5&z=10&kind=internal&h=20").unwrap();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert!(
            v["inspection"].as_str().unwrap().contains("M2=3.796")
                && v["inspection"].as_str().unwrap().contains("M1=8.215"),
            "内花键检验应同时给 M2/M1：{j}"
        );
    }

    // ── 齿轮体系（M/DP，本轮新增）──

    /// M/DP 体系标识解析、DP↔m 换算、块名/spec 保留 DP 原值、M+DP 互斥。
    #[test]
    fn gear_system_m_dp_identifiers_conversion_and_block_name() {
        // 标识解析（大小写/中文）。
        assert_eq!(GearStd::parse("M"), Some(GearStd::M));
        assert_eq!(GearStd::parse("module"), Some(GearStd::M));
        assert_eq!(GearStd::parse("模数制"), Some(GearStd::M));
        assert_eq!(GearStd::parse("DP"), Some(GearStd::DP));
        assert_eq!(GearStd::parse("径节"), Some(GearStd::DP));
        assert_eq!(GearStd::parse("径节制"), Some(GearStd::DP));
        assert_eq!(GearStd::parse("GB"), None);
        // 换算：m = 25.4/DP 双向。
        assert!((m_from_dp(8.0) - 3.175).abs() < 1e-12);
        assert!((m_from_dp(10.0) - 2.54).abs() < 1e-12);
        assert!((dp_from_m(3.175) - 8.0).abs() < 1e-12);
        // DP 参数构造 + 校验。
        let p = GearParams {
            std: GearStd::DP,
            dp: Some(8.0),
            m: m_from_dp(8.0),
            z: 40,
            h: 20.0,
            ..GearParams::default()
        };
        p.validate().unwrap();
        assert!(p.is_dp() && p.dp_value() == Some(8.0));
        assert!(p.spec().starts_with("DP8（m3.175）"), "{}", p.spec());
        // 块名保留 DP 原值（不与同模数模数制件串块）。
        let dp_block = block_name(&p, GearView::Front);
        assert!(dp_block.contains("_DP8_Z40"), "{dp_block}");
        assert!(!dp_block.contains("_M3_175"), "{dp_block}");
        let m_block = block_name(
            &GearParams { m: 3.175, z: 40, h: 20.0, ..GearParams::default() },
            GearView::Front,
        );
        assert_ne!(dp_block, m_block);
        // M+DP 同给 → 互斥；DP 缺值 / DP 与 m 不符 → 拒绝。
        let bad = GearParams {
            std: GearStd::M,
            dp: Some(8.0),
            m: 3.175,
            z: 40,
            h: 20.0,
            ..GearParams::default()
        };
        assert!(bad.validate().unwrap_err().contains("M 与 DP"));
        let bad = GearParams {
            std: GearStd::DP,
            dp: None,
            m: 3.175,
            z: 40,
            h: 20.0,
            ..GearParams::default()
        };
        assert!(bad.validate().unwrap_err().contains("缺径节值"));
        let bad = GearParams {
            std: GearStd::DP,
            dp: Some(8.0),
            m: 3.0,
            z: 40,
            h: 20.0,
            ..GearParams::default()
        };
        assert!(bad.validate().unwrap_err().contains("25.4/DP"));
    }

    /// 命令行/查询串：M/DP 显式标识、DP 位置参数 = [z,h]、M+DP 互斥、d_B 拦截、
    /// 花键体系显式（只给 d_B 不再反推 DIN）。
    #[test]
    fn gear_system_parse_request_and_query_dp_rules() {
        // `std=DP dp=8`：m 由 DP 换算。
        let r = parse_request("std=DP dp=8 z=40 h=20").unwrap();
        assert_eq!(r.params.std, GearStd::DP);
        assert_eq!(r.params.dp, Some(8.0));
        assert!((r.params.m - 3.175).abs() < 1e-12);
        // `DP8` 贴写 + 位置参数 = [z, h]（DP 不写 m）。
        let r = parse_request("DP8 40 20").unwrap();
        assert_eq!(r.params.std, GearStd::DP);
        assert_eq!(r.params.dp, Some(8.0));
        assert_eq!(r.params.z, 40);
        assert!((r.params.h - 20.0).abs() < 1e-12);
        // 裸 `M` = 模数制标识；`m=3` 仍是模数。
        let r = parse_request("M z=40 h=20").unwrap();
        assert_eq!(r.params.std, GearStd::M);
        assert_eq!(parse_request("m=3 z=40 h=20").unwrap().params.m, 3.0);
        // M 与 DP 同给 → 报错。
        for raw in ["std=M dp=8 z=40", "DP8 std=M z=40", "M dp=8 z=40"] {
            let e = parse_request(raw).unwrap_err();
            assert!(e.contains("M 与 DP"), "{raw} → {e}");
        }
        // DP 体系不许再给 m；DP 缺径节值。
        assert!(parse_request("std=DP dp=8 m=3.175 z=40")
            .unwrap_err()
            .contains("不要再给 m"));
        assert!(parse_request("std=DP z=40").unwrap_err().contains("缺径节值"));
        // 查询串同口径。
        let (p, _, _) = params_from_query("kind=external&std=DP&dp=8&z=40&h=20").unwrap();
        assert_eq!(p.std, GearStd::DP);
        assert_eq!(p.dp, Some(8.0));
        assert!((p.m - 25.4 / 8.0).abs() < 1e-12);
        // info JSON 保留 DP 原值；preview/generate 与 M 模式共用几何。
        let j = info_json("std=DP&dp=8&z=40&h=20&view=section").unwrap();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["std"], "DP");
        assert_eq!(v["dp"], 8.0);
        assert!((v["m"].as_f64().unwrap() - 3.175).abs() < 1e-12);
        assert!(v["block"].as_str().unwrap().contains("_DP8_Z40"), "{j}");
        assert!(v["spec"].as_str().unwrap().starts_with("DP8（m3.175）"), "{j}");
        assert!(preview_svg("std=DP&dp=8&z=40&h=20&view=section")
            .unwrap()
            .contains("<svg"));
        assert!(params_from_query("m=2&z=40&std=DP&dp=8")
            .unwrap_err()
            .contains("不要再给 m"));
        assert!(params_from_query("std=M&dp=8&z=40")
            .unwrap_err()
            .contains("M 与 DP"));
        assert!(params_from_query("std=DP&z=40")
            .unwrap_err()
            .contains("缺径节值"));
        // d_B 在 M/DP 下都报错（M 的文案含用户定案句）。
        let e = params_from_query("m=2&z=40&std=M&db=40").unwrap_err();
        assert!(e.contains("齿轮模式不认基准直径") && e.contains("模数制齿轮不使用标准号与基准直径"), "{e}");
        assert!(parse_request("std=DP dp=8 z=40 db=40")
            .unwrap_err()
            .contains("齿轮模式不认基准直径"));
        // 齿轮模式给花键标准号 → 拒绝（保留旧行为）。
        assert!(parse_request("2 40 20 std=GB")
            .unwrap_err()
            .contains("齿轮模式不认标准号"));
        // 花键体系必须显式：只给 d_B 不再反推 DIN（默认 GB → GB_D_B_MSG）。
        let e = parse_request("花键 db=40 m=2 z=18").unwrap_err();
        assert_eq!(e, crate::invol_spline::gb_d_b_msg(), "{e}");
        let e = params_from_query("mode=spline&db=40&m=2&z=18").unwrap_err();
        assert_eq!(e, crate::invol_spline::gb_d_b_msg(), "{e}");
        // 显式 DIN 才走查表。
        let r = parse_request("花键 std=DIN db=40 m=2").unwrap();
        assert_eq!(r.params.spline.as_ref().unwrap().std, crate::invol_spline::SplineStd::DIN);
        // 同一行两个花键体系 → 报错；花键模式拒绝齿轮体系。
        assert!(parse_request("花键 GB30R std=DIN m=2 z=18")
            .unwrap_err()
            .contains("同一行出现两个体系的标识"));
        let e = parse_request("花键 std=DP dp=8 z=40").unwrap_err();
        assert!(e.contains("齿轮体系") && e.contains("花键体系"), "{e}");
    }

    /// `OCSMGEAR … report` 的命令解析层：关键字摘除（大小写/`report-out=`）
    /// + 齿轮 M 计算书四要素（公式/代入/结果/依据来源）。
    #[test]
    fn report_keyword_parsing_and_gear_report_four_elements() {
        let (clean, want, out) = split_report_args(
            "花键 GB30R m=3 z=20 h=30 REPORT report-out=/tmp/ocsm_report.md",
        );
        assert!(want);
        assert_eq!(out.as_deref(), Some("/tmp/ocsm_report.md"));
        assert!(!clean.to_lowercase().contains("report"), "{clean}");
        assert!(clean.contains("GB30R") && clean.contains("h=30"), "{clean}");
        // 没有 report 时原样返回（不动 JSON/DSL 空白）。
        let raw = "{\"segments\": []}";
        let (same, want2, out2) = split_report_args(raw);
        assert_eq!(same, raw);
        assert!(!want2 && out2.is_none());
        // 齿轮 M 计算书四要素。
        let req = parse_request("2 40 20").unwrap();
        let md = build_report(&req.params).unwrap();
        for needle in ["d = mt·z", "2 × 40", "80 mm", "OCSMGEAR"] {
            assert!(md.contains(needle), "齿轮计算书缺 `{needle}`：\n{md}");
        }
        // DP：换算步骤与 DP 来源在。
        let req = parse_request("std=DP dp=8 20 40").unwrap();
        let md = build_report(&req.params).unwrap();
        assert!(md.contains("25.4 / 8") && md.contains("3.175"), "{md}");
    }

    // ── 2026-09 审计回归：体系不支持的参数必须明确拒绝；报告与 JSON 同源 ──

    /// 径节 P/Ps 只属 ANSI：GB/DIN/NF（CLI 与 GUI 查询两侧）都给明确报错；齿轮模式给 P 也拒。
    #[test]
    fn spline_rejects_pitch_outside_ansi() {
        for raw in [
            "花键 GB30R 3 20 h=30 p=8",
            "花键 DIN30 db=40 m=2 h=30 p=8",
            "花键 NFP a=80 m=3.75 h=30 p=8",
        ] {
            let e = parse_request(raw).unwrap_err();
            assert!(
                e.contains("径节 P/Ps 是 ANSI B92.1")
                    && (e.contains("GB/T 3478") || e.contains("DIN 5480") || e.contains("NF E22-141")),
                "`{raw}` 应报径节只属 ANSI：{e}"
            );
            assert!(!e.contains("未实现"), "{e}");
        }
        // GUI 查询串（/api/gear_svg、/api/gear_info 共用）：同一口径。
        for q in [
            "mode=spline&std=GB&profile=GB30R&m=3&z=20&h=30&p=8",
            "mode=spline&std=NF&profile=NFP&db=80&m=3.75&h=30&pitch=5/10",
        ] {
            let e = params_from_query(q).unwrap_err();
            assert!(e.contains("径节 P/Ps 是 ANSI B92.1"), "`{q}` → {e}");
        }
        // 齿轮模式给 P：也不静默忽略。
        let e = parse_request("M 2 40 h=30 p=8").unwrap_err();
        assert!(e.contains("齿轮模式不认花键参数") && e.contains("径节 P"), "{e}");
        let e = params_from_query("m=2&z=40&h=30&p=8").unwrap_err();
        assert!(e.contains("齿轮模式不认径节 P"), "{e}");
        // ANSI 合法路径不受影响（CLI 位置参数 / p= 都按 P）。
        for raw in ["花键 ANSI30P 5 20 h=30", "花键 ANSI30P p=5/10 5 20 h=30"] {
            let req = parse_request(raw).unwrap();
            let (engine, _) = req.params.spline_engine().unwrap();
            assert_eq!(engine.std, crate::invol_spline::SplineStd::ANSI);
            assert!((engine.ansi_p() - 5.0).abs() < 1e-12, "{raw}");
        }
        // Table 2 每列适用径节范围（用户定案 A）：命令行与 GUI 查询串两侧与引擎同一文案。
        // A 列 30°平/齿侧 2.5/5—32/64：40/80 超上限；B 列 30°平/外径 3/6—16/32：20/40 超上限。
        let e = parse_request("花键 ANSI30P 40 20 h=30").unwrap_err();
        assert!(
            e.contains("ANSI B92.1 Table 2")
                && e.contains("30° 平齿根 / 齿侧配合")
                && e.contains("2.5/5 — 32/64")
                && e.contains("40/80")
                && e.contains("超出上限"),
            "{e}"
        );
        let e = params_from_query("mode=spline&std=ANSI&profile=ANSI30PM&pitch=20/40&z=20&h=30")
            .unwrap_err();
        assert!(
            e.contains("30° 平齿根 / 外径配合")
                && e.contains("3/6 — 16/32")
                && e.contains("20/40")
                && e.contains("超出上限"),
            "{e}"
        );
        // 45° E 列 8/16 低于下限 10/20（原 45° 下限校验并入）。
        let e = parse_request("花键 ANSI45R 8 20 h=30").unwrap_err();
        assert!(
            e.contains("10/20 — 128/256") && e.contains("8/16") && e.contains("低于下限"),
            "{e}"
        );
    }

    /// 压力角由齿廓预设固定：GB/DIN/NF 给不一致的 α 必须拒绝；一致（等价没给）可过。
    #[test]
    fn spline_alpha_must_match_profile_preset() {
        let mk = |std, profile: &str, d_b: Option<f64>, m: Option<f64>, alpha: f64| {
            GearParams {
                kind: GearKind::External,
                m: 3.0,
                z: 20,
                h: 30.0,
                spline: Some(SplineOpts {
                    std,
                    profile: profile.to_string(),
                    d_b,
                    m,
                    z: Some(20),
                    alpha_deg: Some(alpha),
                    ..SplineOpts::default()
                }),
                ..GearParams::default()
            }
        };
        use crate::invol_spline::SplineStd;
        // NF：α=20° 固定；30° → 拒。
        let e = mk(SplineStd::NF, "NFP", Some(80.0), Some(3.75), 30.0)
            .spline_engine()
            .unwrap_err();
        assert!(e.contains("压力角固定 20°") && e.contains("不要覆盖"), "{e}");
        assert!(mk(SplineStd::NF, "NFP", Some(80.0), Some(3.75), 20.0)
            .spline_engine()
            .is_ok());
        // DIN：只有 30°；20° → 拒，30° → 过。
        let e = mk(SplineStd::DIN, "DIN30", Some(40.0), Some(2.0), 20.0)
            .spline_engine()
            .unwrap_err();
        assert!(e.contains("只有 30° 齿廓"), "{e}");
        assert!(mk(SplineStd::DIN, "DIN30", Some(40.0), Some(2.0), 30.0)
            .spline_engine()
            .is_ok());
        // GB：α 由齿廓固定（GB30R=30°）；20° → 拒，30° → 过。
        let e = mk(SplineStd::GB, "GB30R", None, Some(3.0), 20.0)
            .spline_engine()
            .unwrap_err();
        assert!(e.contains("压力角由齿廓固定"), "{e}");
        assert!(mk(SplineStd::GB, "GB30R", None, Some(3.0), 30.0)
            .spline_engine()
            .is_ok());
        // CLI/GUI 查询入口同口径。
        let e = parse_request("花键 NFP a=80 m=3.75 h=30 alpha=30").unwrap_err();
        assert!(e.contains("压力角固定 20°"), "{e}");
        let e = params_from_query("mode=spline&std=NF&profile=NFP&db=80&m=3.75&h=30&alpha=30")
            .unwrap_err();
        assert!(e.contains("压力角固定 20°"), "{e}");
    }

    /// 内花键「材料带内无渐开线」（ANSI 细径节 DFi ≥ Dri）在齿轮桥接的提示中明说。
    #[test]
    fn spline_notes_report_radial_fallback_outside_root() {
        let p = GearParams {
            kind: GearKind::Internal,
            m: 25.4 / 128.0,
            z: 200,
            h: 30.0,
            spline: Some(SplineOpts {
                std: crate::invol_spline::SplineStd::ANSI,
                // P=128 只在 45° E 列（10/20—128/256）范围内；内部降级条件与压力角无关。
                profile: "ANSI45圆齿根齿侧".to_string(),
                m: Some(128.0),
                z: Some(200),
                ..SplineOpts::default()
            }),
            ..GearParams::default()
        };
        let (engine, _) = p.spline_engine().unwrap();
        assert!(engine.ansi_form_dia_internal() > engine.internal_major_dia());
        // 端视图能出（降级），并在提示里明说径向直线降级。
        let views = crate::gear::generate(&p, GearView::Front, 1.0).unwrap();
        assert!(!views.entities.is_empty());
        let notes = p.spline_notes(&engine);
        assert!(
            notes
                .iter()
                .any(|n| n.contains("form 直径") && n.contains("径向直线降级")),
            "{notes:?}"
        );
    }

    /// 报告与 JSON 同源：花键 info JSON 的 d/da/df/rho/cf 与引擎访问器逐值一致；
    /// 内花键 JSON 的 internal_major/minor 与 build_report 表格同值。
    #[test]
    fn spline_report_and_json_share_one_source() {
        let p = GearParams {
            kind: GearKind::Internal,
            m: 3.0,
            z: 20,
            h: 30.0,
            spline: Some(SplineOpts {
                std: crate::invol_spline::SplineStd::GB,
                profile: "30圆齿根".to_string(),
                m: Some(3.0),
                z: Some(20),
                ..SplineOpts::default()
            }),
            ..GearParams::default()
        };
        let (engine, _) = p.spline_engine().unwrap();
        let j: serde_json::Value =
            serde_json::from_str(&spline_info_json(&p, GearView::Front).unwrap()).unwrap();
        assert_eq!(j["ok"], true);
        // JSON 数值统一 round4；同源比对时同样取 4 位。
        let r4 = |v: f64| (v * 1e4).round() / 1e4;
        assert_eq!(j["d"].as_f64().unwrap(), r4(engine.d()));
        assert_eq!(j["da"].as_f64().unwrap(), r4(engine.internal_major_dia()));
        assert_eq!(j["df"].as_f64().unwrap(), r4(engine.internal_minor_dia()));
        assert_eq!(j["rho"].as_f64().unwrap(), r4(engine.rho_f()));
        assert_eq!(j["cf"].as_f64().unwrap(), r4(engine.c_f()));
        assert_eq!(j["internal_major"].as_f64().unwrap(), r4(engine.internal_major_dia()));
        assert_eq!(j["internal_minor"].as_f64().unwrap(), r4(engine.internal_minor_dia()));
        let md = build_report(&p).unwrap();
        assert!(md.contains(&format!("| 内花键大径 D_ei（外侧齿根） | {} mm |", trim(engine.internal_major_dia()))), "{md}");
        assert!(md.contains(&format!("| 内花键小径 D_ii（里侧齿顶） | {} mm |", trim(engine.internal_minor_dia()))), "{md}");
    }

    /// 统一齿形表达式：九项 + H；DA/DF = 大径/小径；齿轮/花键同格式；四个示例串钉死。
    #[test]
    fn unified_tooth_expr_format_and_examples() {
        // 外齿轮 m3 z20（ha*=1、c*=0.25、x=0）：d=60、da=66、df=52.5。
        let p = GearParams { kind: GearKind::External, m: 3.0, z: 20, h: 30.0, ..GearParams::default() };
        assert_eq!(gear_tooth_expr(&p), "GEAR EX M3 Z20 ALPHA20 X0 DA66 DF52.5 BETA0 H30");
        // 内齿轮同参数：DA=大径=df=67.5、DF=小径=da=54。
        let p = GearParams { kind: GearKind::Internal, m: 3.0, z: 20, h: 30.0, ..GearParams::default() };
        assert_eq!(gear_tooth_expr(&p), "GEAR IN M3 Z20 ALPHA20 X0 DA67.5 DF54 BETA0 H30");
        // 外花键 GB30R m3 z20 x0：da=63、df=54.6。
        let (e, k) = spline_expr_case(false);
        assert_eq!(
            spline_tooth_expr(&e, k, 30.0),
            "SPLINE EX M3 Z20 ALPHA30 X0 DA63 DF54.6 BETA0 H30"
        );
        // 内花键同参数：DA=大径 D_ei=65.4、DF=小径 D_ii=gb_form+2cF。
        let (e, k) = spline_expr_case(true);
        assert_eq!(
            spline_tooth_expr(&e, k, 30.0),
            "SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30"
        );
        // info JSON 两模式都带 expr（GUI 直接显示这个串）。
        let j: serde_json::Value = serde_json::from_str(
            &info_json("m=3&z=20&h=30").unwrap(),
        )
        .unwrap();
        assert_eq!(j["mode"], "gear");
        assert_eq!(j["expr"], "GEAR EX M3 Z20 ALPHA20 X0 DA66 DF52.5 BETA0 H30");
        let j: serde_json::Value = serde_json::from_str(
            &info_json("mode=spline&std=GB&profile=GB30R&m=3&z=20&h=30").unwrap(),
        )
        .unwrap();
        assert_eq!(j["mode"], "spline");
        assert_eq!(j["expr"], "SPLINE EX M3 Z20 ALPHA30 X0 DA63 DF54.6 BETA0 H30");
    }

    /// 构造统一表达式测试用的 GB30R m3 z20 引擎参数（外/内）。
    fn spline_expr_case(internal: bool) -> (crate::invol_spline::InvolParams, GearKind) {
        let kind = if internal { GearKind::Internal } else { GearKind::External };
        let p = GearParams {
            kind,
            m: 3.0,
            z: 20,
            h: 30.0,
            spline: Some(SplineOpts {
                std: crate::invol_spline::SplineStd::GB,
                profile: "30圆齿根".to_string(),
                m: Some(3.0),
                z: Some(20),
                ..SplineOpts::default()
            }),
            ..GearParams::default()
        };
        let (e, _) = p.spline_engine().unwrap();
        (e, kind)
    }

    /// 统一表达式**往返**：CLI/GUI 的 `expr` → 轴段解析 → 几何一致
    /// （m/z/α/x/DA/DF/内/外/标记）；齿轮模式 + 花键四体系，每个体系外/内各一例。
    #[test]
    fn unified_tooth_expr_round_trips_gear_and_spline_four_standards() {
        let near = |a: f64, b: f64| (a - b).abs() < 1e-3; // 表达式按 4 位小数发射
        // ── 齿轮模式（外/内）──
        for (cli, internal) in [("m=3 z=20 h=30", false), ("int m=3 z=20 h=30", true)] {
            let r = parse_request(cli).unwrap_or_else(|e| panic!("{cli}: {e}"));
            let expr = gear_tooth_expr(&r.params);
            assert!(expr.starts_with("GEAR "), "{expr}");
            let sp = crate::shaft::parse_program(&expr).unwrap_or_else(|e| panic!("{expr}: {e}"));
            let g = sp.segments[0].gear.expect("轴侧应解析出齿形段");
            assert_eq!(g.kind.is_internal(), internal, "{cli}: {expr}");
            assert!(!g.involute, "{cli}: {expr}");
            assert!(near(g.m, r.params.m) && g.z == r.params.z, "{cli}: {expr}");
            assert!(near(g.alpha_deg, r.params.alpha_deg) && near(g.x, r.params.x), "{cli}: {expr}");
            let (da, df) = if internal {
                (r.params.df(), r.params.da())
            } else {
                (r.params.da(), r.params.df())
            };
            assert!(near(g.major_radius() * 2.0, da), "{cli}: {expr}");
            assert!(near(g.minor_radius() * 2.0, df), "{cli}: {expr}");
        }
        // ── 花键四体系（每个体系外/内各一例）──
        let cases = [
            ("花键 GB30R 3 20 h=30", "花键 内花键 GB30R 3 20 h=30"),
            (
                "花键 DIN30 db=40 m=2 z=18 x=0.45 h=30",
                "花键 内花键 DIN30 db=40 m=2 z=18 x=0.45 h=30",
            ),
            ("花键 NFP a=80 m=3.75 h=30", "花键 内花键 NFP a=80 m=3.75 h=30"),
            (
                "花键 ANSI30P p=5/10 5 20 h=30",
                "花键 内花键 ANSI30P p=5/10 5 20 h=30",
            ),
        ];
        for (ext, int) in cases {
            for (cli, internal) in [(ext, false), (int, true)] {
                let r = parse_request(cli).unwrap_or_else(|e| panic!("{cli}: {e}"));
                let (engine, _) = r.params.spline_engine().unwrap_or_else(|e| panic!("{cli}: {e}"));
                assert_eq!(engine.internal, internal, "{cli}");
                let expr = spline_tooth_expr(&engine, r.params.kind, r.params.h);
                assert!(expr.starts_with("SPLINE "), "{cli}: {expr}");
                let sp = crate::shaft::parse_program(&expr)
                    .unwrap_or_else(|e| panic!("{cli}: {expr}: {e}"));
                let g = sp.segments[0].gear.expect("轴侧应解析出齿形段");
                assert!(g.involute, "{cli}: {expr}");
                assert_eq!(g.kind.is_internal(), internal, "{cli}: {expr}");
                assert!(near(g.m, engine.m) && g.z == engine.z, "{cli}: {expr}");
                assert!(near(g.alpha_deg, engine.alpha_deg) && near(g.x, engine.x), "{cli}: {expr}");
                let (da, df) = if internal {
                    (engine.internal_major_dia(), engine.internal_minor_dia())
                } else {
                    (engine.da(), engine.df())
                };
                assert!(near(g.major_radius() * 2.0, da), "{cli}: {expr}");
                assert!(near(g.minor_radius() * 2.0, df), "{cli}: {expr}");
            }
        }
    }

    /// 三入口一致：命令行 `OCSMGEAR 花键 …` 与 GUI 查询串 `/api/gear_*` 解析出的引擎参数
    /// 逐项相等（与轴段共享 `resolve_spline`，参数不另算一套）。
    #[test]
    fn three_entrances_agree_on_engine_params() {
        let cases: [(&str, &str); 4] = [
            (
                "花键 DIN30 db=40 m=2 z=18 x=0.45 h=30",
                "mode=spline&std=DIN&profile=DIN30&db=40&m=2&z=18&x=0.45&h=30",
            ),
            (
                "花键 NFP a=80 m=3.75 h=30",
                "mode=spline&std=NF&profile=NFP&db=80&m=3.75&h=30",
            ),
            (
                "花键 GB30P 3 20 h=30",
                "mode=spline&std=GB&profile=GB30P&m=3&z=20&h=30",
            ),
            (
                "花键 ANSI30P p=5/10 5 20 h=30",
                "mode=spline&std=ANSI&profile=ANSI30P&pitch=5/10&m=5&z=20&h=30",
            ),
        ];
        for (cli, q) in cases {
            let a = parse_request(cli).unwrap();
            let (b, _, _) = params_from_query(q).unwrap();
            // 齿廓代号可能一边写代号（NFP）一边保持原样（NF平齿根），以引擎解析结果为准。
            let (ea, oa) = a.params.spline_engine().unwrap();
            let (eb, ob) = b.spline_engine().unwrap();
            assert_eq!(ea, eb, "`{cli}` vs `{q}` 引擎参数");
            assert_eq!(
                oa.as_ref().map(|o| o.note()),
                ob.as_ref().map(|o| o.note()),
                "`{cli}` vs `{q}` d_B/A 来源"
            );
        }
    }

    /// 花键 vs 齿轮画法实测：同参数、系数对齐时外花键剖视/侧视（剔除小径细线）与齿轮逐条相同；
    /// 内花键剖视与内齿轮同结构（23 条/图层序列同）；两侧剖面线都是 3.0mm 垂距。
    /// 已知有意差异只有：花键侧视多两条 `2细线层` 齿根线、花键无简化正视图。
    #[test]
    fn spline_and_gear_views_share_drawing_method() {
        let ek = |e: &EntityType| -> String {
            match e {
                EntityType::Line(l) => format!(
                    "L {:.9},{:.9} {:.9},{:.9} {}",
                    l.start.x, l.start.y, l.end.x, l.end.y, l.common.layer
                ),
                EntityType::Arc(a) => format!(
                    "A {:.9},{:.9} r{:.9} {:.9} {:.9} {}",
                    a.center.x, a.center.y, a.radius, a.start_angle, a.end_angle, a.common.layer
                ),
                EntityType::Circle(c) => format!(
                    "C {:.9},{:.9} r{:.9} {}",
                    c.center.x, c.center.y, c.radius, c.common.layer
                ),
                EntityType::Hatch(h) => {
                    let ln = &h.pattern.lines[0];
                    let dy = (-ln.offset.x * ln.angle.sin() + ln.offset.y * ln.angle.cos()).abs();
                    format!("H scale={} angle={} loops={} dy={:.9}", h.pattern_scale, h.pattern_angle, h.paths.len(), dy)
                }
                other => format!("{other:?}"),
            }
        };
        // 外：齿轮 m2.5 z40 h20 α20 ha*=1 c*=0.25；NF 花键同参数（系数覆盖对齐）。
        let gear = GearParams {
            kind: GearKind::External,
            m: 2.5,
            z: 40,
            alpha_deg: 20.0,
            ha: 1.0,
            c: 0.25,
            beta_deg: 0.0,
            h: 20.0,
            x: 0.0,
            spline: None,
            std: GearStd::M,
            dp: None,
        };
        let sp = GearParams {
            kind: GearKind::External,
            m: 2.5,
            z: 40,
            h: 20.0,
            spline: Some(SplineOpts {
                std: crate::invol_spline::SplineStd::NF,
                profile: "NF平齿根".to_string(),
                m: Some(2.5),
                z: Some(40),
                x: Some(0.0),
                ha_star: Some(1.0),
                hf_star: Some(1.25),
                ..SplineOpts::default()
            }),
            ..GearParams::default()
        };
        let (e0, _) = sp.spline_engine().unwrap();
        let (g0, _) = {
            gear.validate().unwrap();
            (gear.clone(), ())
        };
        assert!((e0.da() - g0.da()).abs() < 1e-9 && (e0.df() - g0.df()).abs() < 1e-9);
        // 剖视：逐条相同（含剖面线 3.0mm 垂距）。
        // 已知有意差异：花键以左端面为原点（0..L）、齿轮以齿宽中心为原点（−h/2..h/2）
        // （见《齿轮vs花键_画法对照》二、基点条）——比较时把花键整体平移 −h/2。
        let dx_sp = -sp.h / 2.0;
        let ek_shift = |e: &EntityType, dx: f64| -> String {
            match e {
                EntityType::Line(l) => format!(
                    "L {:.9},{:.9} {:.9},{:.9} {}",
                    l.start.x + dx, l.start.y, l.end.x + dx, l.end.y, l.common.layer
                ),
                EntityType::Arc(a) => format!(
                    "A {:.9},{:.9} r{:.9} {:.9} {:.9} {}",
                    a.center.x + dx, a.center.y, a.radius, a.start_angle, a.end_angle, a.common.layer
                ),
                EntityType::Circle(c) => format!(
                    "C {:.9},{:.9} r{:.9} {}",
                    c.center.x + dx, c.center.y, c.radius, c.common.layer
                ),
                other => ek(other),
            }
        };
        let a = generate(&gear, GearView::Section, 1.0).unwrap().entities;
        let b = generate(&sp, GearView::Section, 1.0).unwrap().entities;
        assert_eq!(a.len(), b.len(), "剖视图元数");
        // 图元顺序（齿根线上下先后）不作口径，比排序后的多重集。
        let mut a_keys: Vec<String> = a.iter().map(|e| ek_shift(e, 0.0)).collect();
        let mut b_keys: Vec<String> = b.iter().map(|e| ek_shift(e, dx_sp)).collect();
        a_keys.sort();
        b_keys.sort();
        assert_eq!(a_keys, b_keys, "剖视图元（平移/排序后）应逐条相同");
        // 侧视：花键多两条小径细线（有意），其余逐条相同。
        let a = generate(&gear, GearView::Side, 1.0).unwrap().entities;
        let b = generate(&sp, GearView::Side, 1.0).unwrap().entities;
        let thin = b
            .iter()
            .filter(|e| layer_of(e) == LAYER_THIN)
            .count();
        assert_eq!(thin, 2, "花键侧视只多 2 条小径细线");
        let b2: Vec<String> = b
            .iter()
            .filter(|e| layer_of(e) != LAYER_THIN)
            .map(|e| ek_shift(e, dx_sp))
            .collect();
        let mut a2: Vec<String> = a.iter().map(|e| ek_shift(e, 0.0)).collect();
        let mut b2 = b2;
        a2.sort();
        b2.sort();
        assert_eq!(a2, b2, "侧视（除小径细线、排序后）应逐条相同");
        // 内：剖视（共用模板）类型/图层序列相同、都是 23 条。
        let gi = GearParams {
            kind: GearKind::Internal,
            m: 2.5,
            z: 40,
            alpha_deg: 20.0,
            ha: 1.0,
            c: 0.25,
            beta_deg: 0.0,
            h: 30.0,
            x: 0.0,
            spline: None,
            std: GearStd::M,
            dp: None,
        };
        let si = GearParams {
            kind: GearKind::Internal,
            m: 3.0,
            z: 20,
            h: 30.0,
            spline: Some(SplineOpts {
                std: crate::invol_spline::SplineStd::GB,
                profile: "30圆齿根".to_string(),
                m: Some(3.0),
                z: Some(20),
                ..SplineOpts::default()
            }),
            ..GearParams::default()
        };
        let a = generate(&gi, GearView::Section, 1.0).unwrap().entities;
        let b = generate(&si, GearView::Section, 1.0).unwrap().entities;
        assert_eq!(a.len(), 23, "内齿轮剖视 23 条");
        assert_eq!(b.len(), 23, "内花键剖视 23 条（同模板）");
        let seq = |v: &[EntityType]| {
            v.iter()
                .map(|e| match e {
                    EntityType::Line(_) => "L".to_string(),
                    EntityType::Arc(_) => "A".to_string(),
                    EntityType::Circle(_) => "C".to_string(),
                    EntityType::Hatch(_) => "H".to_string(),
                    _ => "?".to_string(),
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(seq(&a), seq(&b), "内剖视图元类型序列");
        let layers = |v: &[EntityType]| {
            v.iter()
                .map(|e| layer_of(e).to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(layers(&a), layers(&b), "内剖视图层序列");
        // 内花键无侧视/简化正视图（有意，用户定案：同内齿轮）。
        assert!(generate(&si, GearView::Side, 1.0).is_err());
        assert!(generate(&si, GearView::Simplified, 1.0).is_err());
    }

    /// 族③a：齿轮命令输出/报错/计算书 —— zh/en 双语断言 + 关键数据原样。
    #[test]
    fn gear_messages_switch_language_keeping_data() {
        let _g = crate::global_state_test_lock();
        crate::i18n::clear_missing_keys();

        // ── zh ──
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        let p = GearParams { m: 0.0, ..GearParams::default() };
        let e = p.validate().unwrap_err();
        assert!(e.contains("模数 m 必须是正数"), "{e}");
        let p = GearParams { beta_deg: 50.0, ..GearParams::default() };
        let e = p.validate().unwrap_err();
        assert!(e.contains("螺旋角 β 超出范围") && e.contains("50"), "{e}");
        let e = parse_request("mode=nope").unwrap_err();
        assert!(e.contains("模式无法识别") && e.contains("nope"), "{e}");
        let e = parse_request("kind=nope").unwrap_err();
        assert!(e.contains("种类无法识别") && e.contains("nope"), "{e}");
        let e = parse_request("1 40 20 at x,y").unwrap_err();
        assert!(e.contains("at 坐标需要") && e.contains("x,y"), "{e}");
        let p = GearParams { z: 5, ..GearParams::default() };
        assert!(p.notes().iter().any(|n| n.contains("z=5") && n.contains("根切")), "{:?}", p.notes());
        let p = GearParams { beta_deg: 30.0, ..GearParams::default() };
        assert!(p.notes().iter().any(|n| n.contains("斜齿轮") && n.contains("右旋") && n.contains("30")), "{:?}", p.notes());
        let md = build_report(&GearParams::default()).unwrap();
        assert!(md.contains("# 外齿轮 （模数制 M） 计算书"), "{md}");
        assert!(md.contains("## 2. 逐步计算") && md.contains("| 分度圆 d | 80 mm |"), "{md}");
        assert!(gear_mode_db_error().contains("模数制齿轮不使用标准号与基准直径"), "{}", gear_mode_db_error());

        // ── en：同一批调用，数据原样 ──
        crate::i18n::set_lang(crate::i18n::Lang::En);
        let p = GearParams { m: 0.0, ..GearParams::default() };
        let e = p.validate().unwrap_err();
        assert!(e.contains("Module m must be positive"), "{e}");
        let p = GearParams { beta_deg: 50.0, ..GearParams::default() };
        let e = p.validate().unwrap_err();
        assert!(e.contains("Helix angle β out of range") && e.contains("50"), "{e}");
        let e = parse_request("mode=nope").unwrap_err();
        assert!(e.contains("Unrecognized mode") && e.contains("nope"), "{e}");
        let e = parse_request("kind=nope").unwrap_err();
        assert!(e.contains("Unrecognized kind") && e.contains("nope"), "{e}");
        let e = parse_request("1 40 20 at x,y").unwrap_err();
        assert!(e.contains("at coordinates must be") && e.contains("x,y"), "{e}");
        let p = GearParams { z: 5, ..GearParams::default() };
        assert!(p.notes().iter().any(|n| n.contains("z=5") && n.contains("undercut")), "{:?}", p.notes());
        let p = GearParams { beta_deg: 30.0, ..GearParams::default() };
        assert!(p.notes().iter().any(|n| n.contains("Helical gear") && n.contains("right-hand") && n.contains("30")), "{:?}", p.notes());
        let md = build_report(&GearParams::default()).unwrap();
        assert!(md.contains("# external gear (module M) computation report"), "{md}");
        assert!(md.contains("## 2. Step-by-step calculation") && md.contains("| Pitch d | 80 mm |"), "{md}");
        assert!(gear_mode_db_error().contains("Module-mode gears do not use a standard number or reference diameter"), "{}", gear_mode_db_error());
        assert!(
            crate::i18n::missing_keys().is_empty(),
            "缺词条：{:?}",
            crate::i18n::missing_keys()
        );
        crate::i18n::set_lang_auto();
    }
}
