//! **渐开线花键**（GB/T 3478.1-2008 / DIN 5480-1:2015）—— 核心几何 + 预设表 + 视图图元。
//!
//! 本模块是**共用计算引擎**：换算（`d_B ↔ m/z/x`）、DIN 5480-2 名义表/检验表、GB/DIN 预设、
//! 端视图（真实渐开线齿廓）、侧视/剖视轮廓、收尾弧与数值接口都在这里；
//! **入口按齿轮生成器组织**（`gear.rs` 的花键模式复用本引擎），轴段 `INVOLSPLINE` 也复用本引擎。
//! 外花键与内花键（材料在外侧、齿朝内）都支持；ANSI B92.1 是**公式驱动**（径节 17 项 + Table 2
//! 五列基本尺寸公式，见 `assets/ansi_b921_notes.md`），已接入本引擎（无逐行尺寸大表）。
//! 注册点（`lib.rs` / `guide_server.rs` / 两个 GUI / 手册）由后续接线方另行接入，本文件只交付模块本体。
//!
//! # 一、口径（权威公式）
//!
//! ## GB/T 3478.1-2008（默认预设）
//!
//! 基本齿廓见图 2（p07/p08），外花键大径基本尺寸系列见表 4~表 6，尺寸计算公式见表 3：
//!
//! | 基本齿廓 | α | ha* | hf* | ρf* | cF* | 外花键大径 Dee | 外花键小径 Dfe | 内花键大径 Dii |
//! |---|---|---|---|---|---|---|---|---|
//! | 30°平齿根 | 30° | 0.5 | 0.75 | 0.2 | 0.1 | m(z+1) | m(z−1.5) | m(z+1.5) |
//! | 30°圆齿根 | 30° | 0.5 | 0.9 | 0.4 | 0.1 | m(z+1) | m(z−1.8) | m(z+1.8) |
//! | 37.5°圆齿根 | 37.5° | 0.45 | 0.7 | 0.3 | 0.1 | m(z+0.9) | m(z−1.4) | m(z+1.4) |
//! | 45°圆齿根 | 45° | 0.4 | 0.6 | 0.25 | 0.1 | m(z+0.8) | m(z−1.2) | m(z+1.2) |
//!
//! 任务原文只给了 GB 的统一 `ha*=0.5`；为让 `da` 与表 4~表 6 的权威公式
//! `m(z+1) / m(z+0.9) / m(z+0.8)` 逐项一致，37.5°/45° 预设的 ha* 取等价值 **0.45/0.4**
//! （即 `da = d + 2·ha*·m`）。**直径一律按标准公式算**，不用图上标注的齿顶高反推。
//!
//! ## DIN 5480-1:2015
//!
//! `α=30°`、`ha*=0.45`、`hf*=0.55`（DIN 5480-1:2015 条 5.1：**齿侧对中（Flank centering）
//! 时 h_fP = 0.55·m 是基准**；滚刀加工 −0.1m、插齿刀 ±0.2m 属刀具相关修正，需要时用
//! [`InvolParams::with_coeffs`] 覆盖）、
//! `ρf*=0.16`（DIN 5480-2 表 1：m=0.5~10 全为 0.16m）、`c*=hf*−ha*=0.10`；
//! `d_a1 = m·z + 2x·m + 0.9m`、`d_f1 = m·z + 2x·m − 2·hf*·m`、
//! `s1 = m·π/2 + 2x·m·tanα`；变位范围 `x·m ∈ [−0.05m, +0.45m]`（即 `x ∈ [−0.05, 0.45]`，在
//! [`InvolParams::validate`] 里校验）。
//!
//! `hf*=0.55` 与 DIN 5480-2 名义表逐行吻合：m=0.5 的表里 x·m=0.225（x=0.45）时，
//! z=34 → `d_f1=17.00+0.45−0.55=16.90`、z=10 → `5.00+0.45−0.55=4.90`（`d_a1` 亦为 17.90 / 5.90，
//! 均见表 `DIN5480-2_名义表.csv` p.11）；旧值 hf*=0.60 会差 0.1m（16.85），故按任务改用基准 0.55。
//!
//! ## 内花键（GB 表 3，p09/p10 图 2）
//!
//! 内花键 = 材料在外侧、齿朝内；渐开线同一条（内花键**齿槽** = 同参数外花键**齿形**，
//! 与 `gear.rs` 内齿轮同口径），只是从**齿槽中心线**量齿厚半角 ψ(R)。直径口径：
//!
//! * GB：内花键大径（外侧齿根）`D_ei = m(z+1.5)/(z+1.8)/(z+1.4)/(z+1.2)`（表 3）；
//!   内花键小径（里侧齿顶）`D_ii = D_Fe max + 2C_F`，其中 `C_F=0.1m`、
//!   `D_Fe max = 2√((0.5D_b)² + (0.5D·sinα_D − (h_s − 0.5es_v/tanα_D)/sinα_D)²)`，
//!   H/h 配合取 `es_v=0`；`h_s` 见图 2：30° 平/圆 0.6m、37.5° 0.55m、45° 0.5m（从基准线往下量）。
//! * DIN：内花键齿根 `d_f2 = d_B`、齿顶 `d_a2 = d − 0.9m + 2x·m`（721 行名义表恒等式，
//!   见 `din5480_four_identities_hold_for_every_row`）。
//!
//! **内花键与内齿轮同口径**（用户定案：「内花键剖视图和内齿轮一样，不存在侧视图」）：
//! 可用视图 = **剖视图 + 端视图**，**没有侧视图**（也没有简化正视图）。端视图由本模块
//! [`InvolParams::front_view`] 出图；剖视图按 `gear.rs` 内齿轮剖视模板
//! （`internal_bore_section`：端面/齿顶线/齿根线/内孔壁/孔口倒角 + 分度线/轴线，
//! **不打剖面线**、齿圈外壁留用户延伸）出图 —— 本模块不再出旧的「两条矩形轮廓 + 剖面线两环」草案。
//! [`InvolParams::side_view`] 遇到内花键**明确报错**（不静默忽略、不出乱图）。
//!
//! ## DIN 5480-2 名义表（`d_B` 查表，本轮主路径）
//!
//! 数据入库为 `assets/din5480_2_nominal.csv`（721 行 = 旧 618 行 + m=1.5 56 行 + m=5 47 行；
//! m=1.5/m=5 均由用户截图补入，见 `assets/din5480_2_notes.md`）。按基准直径
//! [`lookup_by_d_b`]：同一 `d_B` 可能有多个 `z/x₁` 变体（表格事实，返回多行）；找不到时列附近
//! 候选。m=5 的 2 处 z 与检验表 17 处 k 的原始 OCR 值留在 CSV/flags，运行期按代码修正表生效。
//!
//! `x_from_d_b(d_B, m, z) = (d_B − m(z + 1.1)) / (2m)` 是**从 OCR 名义表反推并经全表校验**
//! 的关系（等价 `d_B = d + 1.1m + 2x·m`、`z_B = z + 1.1 + 2x`；721 行逐行残差 0），
//! **不是标准原文公式**；[`resolve_din_by_d_b`] 用它把 `DB`+（M 或 Z）补成全参数，
//! 并在 `DB+M+Z` 三参齐给时与表值互相印证。
//!
//! ⚠ `d_B` 只参与**名义值选择/显示与一致性校验**，不改变几何公式（几何仍由 m/z/x 决定）；
//! [`InvolParams::d_b`] 存查表/公式得到的基准直径，[`InvolParams::d_b_estimate`] 是旧的
//! `m(z+2x)` 名义估算（保留兼容，非 `d_B` 定义式）。
//!
//! ## DIN 5480-2 检验尺寸表（M₁/M₂/D_M/k/W_k）
//!
//! 数据入库为 `assets/din5480_2_inspection.csv`（**267 行 / 6 档**：m=0.5/0.75/0.8/1/1.5/5；
//! p12/p16/p18/p20 + m=1.5/m=5 用户截图）。[`inspection_query`] 做集成查询：`(d_B,m,z)` 精确查表
//! 优先，表外 z 走**DIN 5480-2 p09 公式**导出（`x` 由 [`x_from_d_b`] 反解、`k` 由
//! [`span_teeth`] 反推、`D_M` 借邻近表行）。公式已用 [`inspection_formula_report`] 逐行对照：
//! 265 行可算，17 处源表 OCR 异常已逐张核对源图（修正值见 `INSPECTION_OCR_FIXES`，
//! **不改写入库 CSV**；m=5 的 17 处 k 另见 `DIN5480_M5_INSPECTION_K_FIXES`），修正后
//! W_k/M1/M2 全部 ≤2e-3；表外 z 才允许公式，
//! 反解 x 超 [−0.05,0.45] 或检验表无该 m 档时明确报“请提供对应表页”。
//!
//! ## NF E22-141 检查尺寸/公差/配合表
//!
//! 数据入库为 `assets/nf_e22141_check.csv`（**399 行 / 10 张表**：p23–p25 检查尺寸、
//! p26 计算标准-设计尺寸、p27 计算标准-检查尺寸、p29 xm 变动值（微米）、
//! p31 配合表、p32/p33 检查尺寸公差值、p35 F/F1/G/G1 偏差值；页清单见
//! `NF-E22-141_检查公差_页清单.md`）。[`nf_check_by_a_m`] / [`nf_check_by_a_n`] 按
//! `(A, m)` 或 `(A, N)` 取该档检查行（p29 的 A 是直径范围 → 区间包含）；查表命中行
//! 原样携带 `flags`（可疑格只标不改）。[`build_report`] 第 4 节对 NF 输出上述检查量、
//! 偏差行（微米，上/下）与页码/表名/source。
//!
//! # 二、几何
//!
//! * 分度圆 `d=mz`、基圆 `db=d·cosα`、齿厚 `s=mπ/2+2xm·tanα`、齿厚半角
//!   `ψ(R)=s/d+invα−invα_R`（`α_R=acos(db/R)`，`R<db` 时夹在 db）—— 与 `gear.rs` 同口径（直齿）。
//! * 端视图：每齿一条**真实渐开线**（按 [`INVOLUTE_SEGMENTS`] 折线采样），**齿槽心线 = 0°+k·360°/z**
//!   （用户定案，与 `spline.rs` 矩形花键同相）；齿顶弧按大径、齿根弧按小径；基圆高于小径时
//!   用径向直线从齿根圆接到基圆。齿根过渡圆角 `ρf` 本轮只导出数值、不画圆角弧。
//! * 侧视：矩形 L×大径 + 小径线；小径线图层由调用方定（常规 `2细线层` / 剖视 `1轮廓实线层`），
//!   模块只产出带图层标记的 `EntityType` 列表。
//! * 收尾（口径同 `spline.rs`）：`R=de/2`，圆心 `(L, ±(小径/2+R))`，与小径相切、终点交大径，
//!   轴向长 `l=√(h(2R−h))`、`h=(大径−小径)/2`；`de` 由调用方传入，须大于大径。
//!
//! # 三、相位/中心线
//!
//! * 齿槽心线 `0°+k·360°/z`，齿心线 `180°/z+k·360°/z`；
//! * 十字中心线长度 = `大径+6`（与 `spline.rs` 正视图一致）。

#![allow(dead_code)] // 注册/接线由后续改动负责，本轮只交付模块

use std::sync::OnceLock;

use ocs_plugin_api::host::acadrust::entities::EntityType;

use crate::partgen_kit::{arc, circle, line, trim, LAYER_CENTER, LAYER_MAIN, LAYER_THIN};

/// GB/T 3478.1-2008 标准号（默认预设）。
pub const GB_CODE: &str = "GB/T 3478.1-2008";
/// DIN 5480-1:2015 标准号。
pub const DIN_CODE: &str = "DIN 5480-1:2015";
/// NF E22-141（法国）标准号 —— 数据已入库（尺寸表 p18/p20/p21/p22，288 行）。
pub const NF_CODE: &str = "NF E22-141";
/// ANSI B92.1 标准号。
pub const ANSI_CODE: &str = "ANSI B92.1";
/// NF 默认齿廓名（`NF平齿根` = 外径定心基准；另 `NF圆齿根`）。
pub const NF_DEFAULT_PROFILE: &str = "NF平齿根";
/// ANSI 默认齿廓名（Table 2 列 A：30° 平齿根 齿侧配合）。
pub const ANSI_DEFAULT_PROFILE: &str = "ANSI30平齿根齿侧";

/// 端视图每条渐开线齿廓的折线段数（真实渐开线采样；折线保形、实体数可控）。
pub const INVOLUTE_SEGMENTS: usize = 12;

/// **GB/T 3478.1-2008 表 2 模数系列（15 种，第 1 系列优先 + 第 2 系列）**。
///
/// 第 1 系列（优先采用）：0.25、0.5、1、1.5、2、2.5、3、5、10；
/// 第 2 系列：0.75、1.25、1.75、4、6、8。
/// （表 2 原文分两列；`0.25` 亦在表内，共 15 种。数值顺序升序，来源见 `索引_GB3478.md` p06/p07。）
pub const GB_MODULES: &[f64] = &[
    0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0,
];

// ─────────────────────────── 标准 / 预设 ───────────────────────────

/// 花键体系（**显式参数标识**：不再从 `d_B` 反推）。
///
/// * `GB` = GB/T 3478.1-2008（模数制，**不允许 d_B**）；
/// * `DIN` = DIN 5480-1:2015（`d_B` 主参数）；
/// * `NF` = NF E22-141（法国，**也含基准直径主参数 `A`**；尺寸表 288 行已入库）；
/// * `ANSI` = ANSI B92.1（公式驱动：径节 P + Table 2 五列；数据 `assets/ansi_b921_formulas.csv`）。
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplineStd {
    /// GB/T 3478.1-2008（默认预设）。
    GB,
    /// DIN 5480-1:2015。
    DIN,
    /// NF E22-141（法国；尺寸表已入库）。
    NF,
    /// ANSI B92.1（径节制：径节 P + Table 2 五列公式）。
    ANSI,
}

impl SplineStd {
    /// 标准号。
    pub fn code(self) -> &'static str {
        match self {
            Self::GB => GB_CODE,
            Self::DIN => DIN_CODE,
            Self::NF => NF_CODE,
            Self::ANSI => ANSI_CODE,
        }
    }

    /// 短标签（图纸标注/块名/文件名用）。
    pub fn label(self) -> &'static str {
        match self {
            Self::GB => "GB",
            Self::DIN => "DIN",
            Self::NF => "NF",
            Self::ANSI => "ANSI",
        }
    }

    /// 该标准是否**使用基准直径主参数**（DIN 5480 用 `d_B`、NF E22-141 用 `A`；GB/ANSI 不允许）。
    pub fn uses_d_b(self) -> bool {
        matches!(self, Self::DIN | Self::NF)
    }

    /// 该标准下的预设列表（ANSI 无预设立即空表）。
    pub fn presets(self) -> &'static [InvolPreset] {
        match self {
            Self::GB => GB_PRESETS,
            Self::DIN => DIN_PRESETS,
            Self::NF => NF_PRESETS,
            Self::ANSI => ANSI_PRESETS,
        }
    }
}

/// 一条基本齿廓预设（可被 `x`/系数逐个覆盖）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InvolPreset {
    /// 所属标准。
    pub std: SplineStd,
    /// 齿廓名（GB：`30平齿根`/`30圆齿根`/`37.5圆齿根`/`45圆齿根`；DIN：`DIN30`）。
    pub profile: &'static str,
    /// 压力角 α（度）。
    pub alpha_deg: f64,
    /// 齿顶高系数 ha*。
    pub ha_star: f64,
    /// 齿根高系数 hf*。
    pub hf_star: f64,
    /// 齿根圆角系数 ρf*。
    pub rho_star: f64,
    /// 齿形裕度系数 cF*（GB 全为 0.1；DIN 取基础齿廓顶隙 c = h_fP*−h_aP* = 0.10）。
    pub c_f_star: f64,
    /// GB 图 2 的 `h_s`（从基准线往下量，用于 `D_Fe max` 公式）：30° 0.6、37.5° 0.55、45° 0.5。
    /// DIN 名义表/检验表口径不依赖它（DIN 内花键直接用 `d_f2=d_B`、`d_a2=d−0.9m+2xm`）。
    pub h_s_star: f64,
}

/// **GB/T 3478.1-2008 预设表**（图 2 基本齿廓 + 表 4~表 6 大径公式；h_s 见图 2）。
pub const GB_PRESETS: &[InvolPreset] = &[
    InvolPreset {
        std: SplineStd::GB,
        profile: "30平齿根",
        alpha_deg: 30.0,
        ha_star: 0.5,
        hf_star: 0.75,
        rho_star: 0.2,
        c_f_star: 0.1,
        h_s_star: 0.6,
    },
    InvolPreset {
        std: SplineStd::GB,
        profile: "30圆齿根",
        alpha_deg: 30.0,
        ha_star: 0.5,
        hf_star: 0.9,
        rho_star: 0.4,
        c_f_star: 0.1,
        h_s_star: 0.6,
    },
    InvolPreset {
        std: SplineStd::GB,
        profile: "37.5圆齿根",
        alpha_deg: 37.5,
        ha_star: 0.45,
        hf_star: 0.7,
        rho_star: 0.3,
        c_f_star: 0.1,
        h_s_star: 0.55,
    },
    InvolPreset {
        std: SplineStd::GB,
        profile: "45圆齿根",
        alpha_deg: 45.0,
        ha_star: 0.4,
        hf_star: 0.6,
        rho_star: 0.25,
        c_f_star: 0.1,
        h_s_star: 0.5,
    },
];

/// **DIN 5480-1:2015 预设表**（齿侧对中基准 `h_fP*=0.55`；`c_f_star` 见类型注释）。
pub const DIN_PRESETS: &[InvolPreset] = &[InvolPreset {
    std: SplineStd::DIN,
    profile: "DIN30",
    alpha_deg: 30.0,
    ha_star: 0.45,
    hf_star: 0.55,
    rho_star: 0.16,
    c_f_star: 0.10,
    h_s_star: 0.6,
}];

/// **NF E22-141 预设表**（p07 公式：α=20°；`A₁′=A` 外径定心 / `A₂′=A−0.2m` 齿面定心，
/// 本预设默认外径定心；`B=A−2.4m` 平齿根、`B₁=A−2.694m` 圆齿根；
/// `R=0.3m`、`R₁=0.528m`、`h=0.1·max(m,1)`、`D=A−2m`）。
///
/// 口径换算（引擎 `d_eff = d+2xm = A−0.4m`）：
/// * da = A → `ha*=0.2`；
/// * df(平) = A−2.4m → `hf*=1.0`，`ρf*=0.3`；
/// * df(圆) = A−2.694m → `hf*=1.147`，`ρf*=0.528`。
///
/// `c_f_star` NF 几何不用（内花键小径直接用 `D=A−2m`），保留 0.1 仅供显示/与 GB 同构。
pub const NF_PRESETS: &[InvolPreset] = &[
    InvolPreset {
        std: SplineStd::NF,
        profile: "NF平齿根",
        alpha_deg: 20.0,
        ha_star: 0.2,
        hf_star: 1.0,
        rho_star: 0.3,
        c_f_star: 0.1,
        h_s_star: 0.6,
    },
    InvolPreset {
        std: SplineStd::NF,
        profile: "NF圆齿根",
        alpha_deg: 20.0,
        ha_star: 0.2,
        hf_star: 1.147,
        rho_star: 0.528,
        c_f_star: 0.1,
        h_s_star: 0.6,
    },
];

/// GB 体系误给 `d_B`/`A` 的**统一报错文案**（GB/T 3478 没有基准直径这个概念）。
pub fn gb_d_b_msg() -> String {
    crate::i18n::t("cmd.invol.err.gb_db")
}

/// GB 体系误给非零变位系数 `x` 的**统一报错文案**（GB/T 3478 基本齿廓不含变位）。
/// 与 [`GB_D_B_MSG`] / [`PITCH_ONLY_ANSI_MSG`] 同一口径：体系没有的参数在入口层明确拒绝。
pub fn gb_x_msg() -> String {
    crate::i18n::t("cmd.invol.err.gb_x")
}

/// 非 ANSI 体系误给径节的**统一报错文案**（P/Ps 是 ANSI B92.1 的径节制写法）。
pub fn pitch_only_ansi_msg() -> String {
    crate::i18n::t("cmd.invol.err.pitch_only_ansi")
}

/// ANSI B92.1 误给 `d_B`/`A` 的报错文案（径节制用径节 P/Ps，不用基准直径 `d_B`/`A`）。
pub fn ansi_d_b_msg() -> String {
    crate::i18n::t("cmd.invol.err.ansi_db")
}

/// 标准下的默认齿廓名（不给 profile 时用）。
pub fn default_profile(std: SplineStd) -> &'static str {
    match std {
        SplineStd::GB => "30圆齿根",
        SplineStd::DIN => "DIN30",
        SplineStd::NF => NF_DEFAULT_PROFILE,
        SplineStd::ANSI => ANSI_DEFAULT_PROFILE,
    }
}

// ─────────────────── ANSI B92.1-1970 (R1993)（径节系列 + Table 2 五列） ───────────────────

/// 入库径节系列（`assets/ansi_b921_formulas.csv`，**17 项**；p08/p09/p11 三处互证）。
const ANSI_B921_CSV: &str = include_str!("../assets/ansi_b921_formulas.csv");

/// 英寸 → 毫米换算（ANSI B92.1 是英制标准：P 为每英寸齿数、表值为英寸；
/// 引擎内部与输出统一 mm，和 GB/DIN/NF 一致）。
pub const ANSI_INCH_MM: f64 = 25.4;

/// ANSI B92.1 径节系列一行（`P` 入 Table 2 公式；`Ps = 2P` 只用于标识显示）。
#[derive(Debug, Clone, PartialEq)]
pub struct AnsiPitch {
    /// 径节 `P`（Table 2 公式用；单位=每英寸齿数）。
    pub p: f64,
    /// stub pitch `Ps = 2P`（只用于标识显示）。
    pub ps: f64,
    /// 系列写法（`2.5/5` … `128/256`）。
    pub label: String,
}

static ANSI_PITCHES: OnceLock<Vec<AnsiPitch>> = OnceLock::new();

/// 径节系列 17 项（懒加载；解析不了 panic —— 数据随二进制编译，属构建错误）。
pub fn ansi_pitches() -> &'static [AnsiPitch] {
    ANSI_PITCHES.get_or_init(|| {
        parse_ansi_pitches(ANSI_B921_CSV)
            .unwrap_or_else(|e| panic!("ANSI B92.1 径节系列入库数据损坏：{e}"))
    })
}

/// 解析径节系列 CSV（跳过 `#` 注释与表头；`Ps=2P` 自检）。
fn parse_ansi_pitches(text: &str) -> Result<Vec<AnsiPitch>, String> {
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        if f.first().map(|s| *s == "P").unwrap_or(false) {
            continue; // 表头
        }
        if f.len() < 3 {
            return Err(format!("第 {n} 行只有 {} 列（应有 P,Ps,pitch）", f.len()));
        }
        let p = f[0]
            .parse::<f64>()
            .map_err(|_| format!("第 {n} 行 P「{}」非法", f[0]))?;
        let ps = f[1]
            .parse::<f64>()
            .map_err(|_| format!("第 {n} 行 Ps「{}」非法", f[1]))?;
        if !(p.is_finite() && p > 0.0) {
            return Err(format!("第 {n} 行 P={p} 非正"));
        }
        if (ps - 2.0 * p).abs() > 1e-9 {
            return Err(format!("第 {n} 行 Ps={ps} ≠ 2P={}", 2.0 * p));
        }
        rows.push(AnsiPitch {
            p,
            ps,
            label: f[2].to_string(),
        });
    }
    if rows.len() != 17 {
        return Err(format!("应有 17 项径节，入库 {}", rows.len()));
    }
    Ok(rows)
}

/// 按 P 在 17 项系列里查（相对容差 1e-9）；找不到返回 `None`。
pub fn ansi_pitch_row(p: f64) -> Option<&'static AnsiPitch> {
    ansi_pitches()
        .iter()
        .find(|r| p.is_finite() && (r.p - p).abs() <= 1e-9 * r.p.abs().max(1.0))
}

/// 17 项系列的 `A/B` 写法列表（`2.5/5`、`3/6` … `128/256`；错误文案/GUI 共用）。
pub fn ansi_pitch_labels() -> Vec<&'static str> {
    ansi_pitches().iter().map(|r| r.label.as_str()).collect()
}

/// 径节写法统一说明（报错/GUI 提示共用）：`A/B` 成对写法。
pub fn ansi_pitch_form_msg() -> String {
    crate::i18n::t("cmd.invol.pitch.form")
}

/// 解析 ANSI 径节写法：**只校验语法**（`A/B` 的 `B == 2A`，或裸数字为正数），
/// **不查 17 项系列** —— 系列由 [`InvolParams::validate`] 统一把关，这样轴段
/// `INVOLSPLINE … RL@L P1.5` 仍先报「不能与退刀槽同段」而不是被径节系列拦截。
///
/// 返回 `P`（= A）；错误带 `ANSI B92.1：` 前缀。
pub fn parse_ansi_pitch_syntax(text: &str) -> Result<f64, String> {
    let s = text.trim();
    if s.is_empty() {
        return Err(crate::i18n::t_fmt(
            "cmd.invol.pitch.err.missing",
            &[("form", &ansi_pitch_form_msg())],
        ));
    }
    if let Some((a, b)) = s.split_once('/') {
        if b.contains('/') {
            return Err(crate::i18n::t_fmt(
                "cmd.invol.pitch.err.bad_format",
                &[("s", s), ("form", &ansi_pitch_form_msg())],
            ));
        }
        let (a_txt, b_txt) = (a.trim(), b.trim());
        let a: f64 = a_txt.parse().map_err(|_| {
            crate::i18n::t_fmt(
                "cmd.invol.pitch.err.a_not_number",
                &[("s", s), ("form", &ansi_pitch_form_msg())],
            )
        })?;
        let b: f64 = b_txt.parse().map_err(|_| {
            crate::i18n::t_fmt(
                "cmd.invol.pitch.err.b_not_number",
                &[("s", s), ("form", &ansi_pitch_form_msg())],
            )
        })?;
        if !(a.is_finite() && a > 0.0) || !(b.is_finite() && b > 0.0) {
            return Err(crate::i18n::t_fmt(
                "cmd.invol.pitch.err.not_positive",
                &[("s", s)],
            ));
        }
        if (b - 2.0 * a).abs() > 1e-9 * (2.0 * a).abs().max(1.0) {
            return Err(crate::i18n::t_fmt(
                "cmd.invol.pitch.err.b_not_2a",
                &[
                    ("a", &trim(a)),
                    ("b", &trim(b)),
                    ("a2", &trim(a)),
                    ("b2", &trim(2.0 * a)),
                ],
            ));
        }
        return Ok(a);
    }
    let p: f64 = s
        .parse()
        .map_err(|_| {
            crate::i18n::t_fmt(
                "cmd.invol.pitch.err.not_number",
                &[("s", s), ("form", &ansi_pitch_form_msg())],
            )
        })?;
    if !(p.is_finite() && p > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.pitch.err.p_positive", &[("p", &trim(p))]));
    }
    Ok(p)
}

/// 17 项系列校验：不在系列里报错，并把 17 项按 `A/B` 形式列出来。
pub fn ansi_pitch_series_check(p: f64) -> Result<(), String> {
    if ansi_pitch_row(p).is_some() {
        return Ok(());
    }
    Err(crate::i18n::t_fmt(
        "cmd.invol.pitch.err.series",
        &[
            ("p", &trim(p)),
            ("list", &ansi_pitch_labels().join(&crate::i18n::t("cmd.detail.sep.list"))),
            ("form", &ansi_pitch_form_msg()),
        ],
    ))
}

/// 全量解析径节：语法（`B==2A`）+ 17 项系列校验；入口层（CLI/查询串/GUI）用。
pub fn parse_ansi_pitch(text: &str) -> Result<f64, String> {
    let p = parse_ansi_pitch_syntax(text)?;
    ansi_pitch_series_check(p)?;
    Ok(p)
}

/// 给已带前缀的 ANSI 报错补 `ANSI B92.1：`（不重复补）。
fn ansi_prefixed(e: String) -> String {
    if e.starts_with(ANSI_CODE) {
        e
    } else {
        crate::i18n::t_fmt("cmd.invol.err.prefix", &[("code", ANSI_CODE), ("e", &e)])
    }
}

/// ANSI B92.1 Table 2 的五个公式列（p10；A–E）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsiColumn {
    /// A：30° 平齿根 齿侧配合（`Dre` 第 1/2 段 `(N−1.35)/P`）。
    A30FlatSide,
    /// B：30° 平齿根 **外径配合**（`DFi` 含英寸常量 `−0.004 in = −0.1016 mm`）。
    B30FlatMajor,
    /// C：30° 圆齿根 齿侧配合（`Dre` 分 `(N−1.8)/P` 与 `(N−2)/P` 两段）。
    C30FilletSide,
    /// D：37.5° 圆齿根 齿侧配合。
    D375FilletSide,
    /// E：45° 圆齿根 齿侧配合（`Dre = (N−1)/P`，径节 10/20 起）。
    E45FilletSide,
}

impl AnsiColumn {
    /// profile 名 → 列（容 `ANSI` 前缀与空白/度符号，与 [`norm_profile`] 同口径）。
    pub fn from_profile(profile: &str) -> Option<Self> {
        let key = norm_profile(profile);
        let key = key.strip_prefix("ansi").unwrap_or(&key);
        match key {
            "30平齿根齿侧" | "30平齿根" | "30平" | "30p" => Some(Self::A30FlatSide),
            "30平齿根外径" | "30平外径" | "30pm" | "30m" => Some(Self::B30FlatMajor),
            "30圆齿根齿侧" | "30圆齿根" | "30圆" | "30r" => Some(Self::C30FilletSide),
            "37.5圆齿根齿侧" | "37.5圆齿根" | "375圆齿根" | "375r" => Some(Self::D375FilletSide),
            "45圆齿根齿侧" | "45圆齿根" | "45r" => Some(Self::E45FilletSide),
            _ => None,
        }
    }

    /// 该列在 **Table 2 表头**给出的**适用径节范围**（P 值，上下界含端点；`P/Ps` 写法见
    /// [`Self::range_label`]）。用户定案 A：按列强制（更忠实标准原文）。
    ///
    /// **勿与 Table 3 混淆**：Table 2 的范围只约束**该列的基本尺寸公式/几何**；Table 3
    /// 是公式/数值表，把 `Ps`/`p`/`Sv min` 对 17 项径节全部印出，其适用面比每列更宽 ——
    /// 所以 `Sv min` 走 [`ansi_sv_min_formula`]，**不**受本范围约束。
    pub fn pitch_range(&self) -> (f64, f64) {
        match self {
            // 30° 平齿根 齿侧配合：2.5/5 — 32/64
            Self::A30FlatSide => (2.5, 32.0),
            // 30° 平齿根 外径配合：3/6 — 16/32
            Self::B30FlatMajor => (3.0, 16.0),
            // 30° 圆齿根 齿侧配合：2.5/5 — 48/96
            Self::C30FilletSide => (2.5, 48.0),
            // 37.5° 圆齿根 齿侧配合：2.5/5 — 48/96
            Self::D375FilletSide => (2.5, 48.0),
            // 45° 圆齿根 齿侧配合：10/20 — 128/256
            Self::E45FilletSide => (10.0, 128.0),
        }
    }

    /// 列名（压力角 + 齿根型式 + 配合方式；Table 2 表头口径），报错点名用。
    pub fn desc(&self) -> &'static str {
        match self {
            Self::A30FlatSide => "30° 平齿根 / 齿侧配合",
            Self::B30FlatMajor => "30° 平齿根 / 外径配合",
            Self::C30FilletSide => "30° 圆齿根 / 齿侧配合",
            Self::D375FilletSide => "37.5° 圆齿根 / 齿侧配合",
            Self::E45FilletSide => "45° 圆齿根 / 齿侧配合",
        }
    }

    /// 适用径节的 `P/Ps` 范围写法（Table 2 表头原文，含端点）。
    pub fn range_label(&self) -> &'static str {
        match self {
            Self::A30FlatSide => "2.5/5 — 32/64",
            Self::B30FlatMajor => "3/6 — 16/32",
            Self::C30FilletSide => "2.5/5 — 48/96",
            Self::D375FilletSide => "2.5/5 — 48/96",
            Self::E45FilletSide => "10/20 — 128/256",
        }
    }
}

/// `P/Ps` 成对写法（优先取 17 项系列里的标准 label，系列外退化为 `P/2P`）。
fn ansi_pair_label(p: f64) -> String {
    ansi_pitch_row(p)
        .map(|r| r.label.clone())
        .unwrap_or_else(|| format!("{}/{}", trim(p), trim(2.0 * p)))
}

/// 按所选 Table 2 列的**适用径节范围**校验 P（上下界含端点；越界报错点名列与范围原文）。
///
/// 这是 [`InvolParams::validate`] 的 ANSI 分支唯一入口：原来只查 45° 下限的特例已并入本函数
/// （五列统一走 [`AnsiColumn::pitch_range`]）。
///
/// 注意 [`ansi_sv_min_formula`] 不调用本函数 —— Table 3 的 `Sv min` 适用面更宽（见其注释）。
pub fn ansi_column_pitch_check(col: AnsiColumn, p: f64) -> Result<(), String> {
    let (lo, hi) = col.pitch_range();
    // 系列里的径节都是精确十进制值，1e-9 相对容差只兜浮点往返。
    let eps = 1e-9 * p.abs().max(1.0);
    let side = if p < lo - eps {
        crate::i18n::t("cmd.invol.column.below")
    } else if p > hi + eps {
        crate::i18n::t("cmd.invol.column.above")
    } else {
        return Ok(());
    };
    Err(crate::i18n::t_fmt(
        "cmd.invol.column.err.range",
        &[
            ("col", col.desc()),
            ("range", col.range_label()),
            ("got", &ansi_pair_label(p)),
            ("side", &side),
        ],
    ))
}

/// ANSI `cF`（Table 2；标准为英寸常量，本函数入参/返回均 mm）：
/// 原式 `cF_in = clamp(0.001·D_in, 0.002, 0.010)`；夹取是线性的，`0.001·D` 与单位无关，
/// 只有上下限换算 → `cF_mm = clamp(0.001·D_mm, 0.0508, 0.254)`。
pub fn ansi_c_f(d_mm: f64) -> f64 {
    (0.001 * d_mm).clamp(0.0508, 0.254)
}

/// ANSI **`Sv min`（最小有效齿槽宽）的 Table 3 公式**（`alpha_deg` 取 30° / 37.5° / 45°；
/// 入参 P、返回 mm）。
///
/// Table 2 与 Table 3 的口径区别（易混，写死在此）：
/// * **Table 2** 表头给的是**每个列**（压力角 + 齿根型式 + 配合方式）自己的**适用径节范围**，
///   由 [`AnsiColumn::pitch_range`] / [`ansi_column_pitch_check`] 强制；
/// * **Table 3** 是公式/数值表，把 `Ps`/`p`/`Sv min` 对 **17 项径节全部印满**（比每列上界更宽），
///   即 `Sv min` 公式适用面比 Table 2 单列更宽 —— 因此本函数从 (α, P) 直接算，
///   **不选列、不受列界约束**（`ansi_sample_check_csv_all_73_rows` 用它复算 17 项印出值）。
///
/// 公式：30° `π/(2P)`、37.5° `(0.5π+0.1)/P`、45° `(0.5π+0.2)/P`（标准英寸值；输出 mm）。
pub fn ansi_sv_min_formula(alpha_deg: f64, p: f64) -> f64 {
    let k = if (alpha_deg - 37.5).abs() < 1e-9 {
        0.1
    } else if (alpha_deg - 45.0).abs() < 1e-9 {
        0.2
    } else {
        0.0
    };
    (0.5 * std::f64::consts::PI + k) * (ANSI_INCH_MM / p)
}

/// ANSI 齿根型式（Table 2 列：平齿根 / 圆齿根）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsiRoot {
    /// 平齿根（仅 30°；列 A/B）。
    Flat,
    /// 圆齿根（30° 列 C；37.5° 列 D；45° 列 E）。
    Fillet,
}

/// ANSI 配合方式（Table 2 列：齿侧配合 / 外径配合）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsiFit {
    /// 齿侧配合（Side Fit）。
    Side,
    /// 外径配合（Major Dia Fit；仅 30° 平齿根列 B）。
    MajorDia,
}

/// 压力角 + 齿根型式 + 配合方式 → ANSI 预设齿廓名（非法组合给逐条原因）。
///
/// 合法组合只有 Table 2 的五列：30°平/齿侧、30°平/外径、30°圆/齿侧、37.5°圆/齿侧、45°圆/齿侧。
pub fn ansi_profile_name(
    alpha_deg: f64,
    root: AnsiRoot,
    fit: AnsiFit,
) -> Result<&'static str, String> {
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    match (root, fit) {
        (AnsiRoot::Flat, AnsiFit::Side) if near(alpha_deg, 30.0) => {
            Ok("ANSI30平齿根齿侧")
        }
        (AnsiRoot::Flat, AnsiFit::MajorDia) if near(alpha_deg, 30.0) => {
            Ok("ANSI30平齿根外径")
        }
        (AnsiRoot::Fillet, AnsiFit::Side) if near(alpha_deg, 30.0) => {
            Ok("ANSI30圆齿根齿侧")
        }
        (AnsiRoot::Fillet, AnsiFit::Side) if near(alpha_deg, 37.5) => {
            Ok("ANSI37.5圆齿根齿侧")
        }
        (AnsiRoot::Fillet, AnsiFit::Side) if near(alpha_deg, 45.0) => {
            Ok("ANSI45圆齿根齿侧")
        }
        (AnsiRoot::Flat, _) if near(alpha_deg, 37.5) || near(alpha_deg, 45.0) => Err(crate::i18n::t_fmt("cmd.invol.err.ansi_no_flat_col", &[("v0", &trim(alpha_deg))])),
        (AnsiRoot::Fillet, AnsiFit::MajorDia) => Err(crate::i18n::t("cmd.invol.err.ansi_fillet_side_only")),
        _ => Err(crate::i18n::t_fmt("cmd.invol.err.ansi_alpha_unsupported", &[("v0", &trim(alpha_deg))])),
    }
}

/// **ANSI B92.1 Table 2 预设表**（五列 A–E）。
///
/// 系数只作兜底/显示；ANSI 几何由 `InvolParams` 的 `std=ANSI` 分支按 Table 2 公式算：
/// `ha* = 0.5`（→ `Do=(N+1)/P`）；`hf*`/`rho*`/`cF*` 不参与（`Dre/Dri/Di/DFe/DFi/cF` 均按公式）。
/// `h_s_star` 只服务 GB 的 `D_Fe max`，ANSI 不用。
pub const ANSI_PRESETS: &[InvolPreset] = &[
    InvolPreset {
        std: SplineStd::ANSI,
        profile: "ANSI30平齿根齿侧",
        alpha_deg: 30.0,
        ha_star: 0.5,
        hf_star: 0.675,
        rho_star: 0.0,
        c_f_star: 0.0,
        h_s_star: 0.0,
    },
    InvolPreset {
        std: SplineStd::ANSI,
        profile: "ANSI30平齿根外径",
        alpha_deg: 30.0,
        ha_star: 0.5,
        hf_star: 0.5,
        rho_star: 0.0,
        c_f_star: 0.0,
        h_s_star: 0.0,
    },
    InvolPreset {
        std: SplineStd::ANSI,
        profile: "ANSI30圆齿根齿侧",
        alpha_deg: 30.0,
        ha_star: 0.5,
        hf_star: 0.9,
        rho_star: 0.0,
        c_f_star: 0.0,
        h_s_star: 0.0,
    },
    InvolPreset {
        std: SplineStd::ANSI,
        profile: "ANSI37.5圆齿根齿侧",
        alpha_deg: 37.5,
        ha_star: 0.5,
        hf_star: 0.65,
        rho_star: 0.0,
        c_f_star: 0.0,
        h_s_star: 0.0,
    },
    InvolPreset {
        std: SplineStd::ANSI,
        profile: "ANSI45圆齿根齿侧",
        alpha_deg: 45.0,
        ha_star: 0.5,
        hf_star: 0.5,
        rho_star: 0.0,
        c_f_star: 0.0,
        h_s_star: 0.0,
    },
];

/// 齿廓名规范化（去空白/度符号、转小写）：`"30° 圆齿根"` 与 `"30圆齿根"` 等价。
fn norm_profile(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace() && *c != '°')
        .collect::<String>()
        .to_lowercase()
}

/// 按标准 + 齿廓名查预设（找不到时把可用预设列出来）。
pub fn preset(std: SplineStd, profile: &str) -> Result<&'static InvolPreset, String> {
    let key = norm_profile(profile);
    std.presets()
        .iter()
        .find(|p| norm_profile(p.profile) == key)
        .ok_or_else(|| {
            let list = std
                .presets()
                .iter()
                .map(|p| p.profile)
                .collect::<Vec<_>>()
                .join(&crate::i18n::t("cmd.invol.sep.list"));
            crate::i18n::t_fmt("cmd.invol.err.preset_unknown", &[("v0", &std.code()), ("v1", &profile), ("v2", &list)])
        })
}

// ─────────────────────────── 预设代号（CLI / DSL / GUI） ───────────────────────────

/// 预设 → CLI/DSL 代号：**GB 默认 `GB30R`**，另 `GB30P`/`GB375R`/`GB45R`/`DIN30`。
pub fn preset_code(std: SplineStd, profile: &str) -> Option<&'static str> {
    let key = norm_profile(profile);
    std.presets().iter().find(|p| norm_profile(p.profile) == key).map(|p| {
        match (std, p.profile) {
            (SplineStd::GB, "30平齿根") => "GB30P",
            (SplineStd::GB, "30圆齿根") => "GB30R",
            (SplineStd::GB, "37.5圆齿根") => "GB375R",
            (SplineStd::GB, "45圆齿根") => "GB45R",
            (SplineStd::NF, "NF平齿根") => "NFP",
            (SplineStd::NF, "NF圆齿根") => "NFR",
            (SplineStd::ANSI, "ANSI30平齿根齿侧") => "ANSI30P",
            (SplineStd::ANSI, "ANSI30平齿根外径") => "ANSI30PM",
            (SplineStd::ANSI, "ANSI30圆齿根齿侧") => "ANSI30R",
            (SplineStd::ANSI, "ANSI37.5圆齿根齿侧") => "ANSI375R",
            (SplineStd::ANSI, "ANSI45圆齿根齿侧") => "ANSI45R",
            _ => "DIN30",
        }
    })
}

/// 预设代号 → `(标准, 齿廓名)`；收 `GB30P`/`GB30R`/`GB375R`/`GB45R`/`DIN30`，
/// 也收 GB + 中文齿廓名（`GB30圆齿根`）；`GB` / `DIN` 裸写取各自默认齿廓
/// （GB → `30圆齿根`，DIN → `DIN30`）。
///
/// **体系标识**也在这里收（显式标识，不靠 `d_B` 反推）：
/// `NF` / `NFE22141` / `NF E22-141` → [`SplineStd::NF`]（默认 `NF平齿根`），
/// 也收 `NFP`/`NF平齿根` 与 `NFR`/`NF圆齿根`；
/// `ANSI` / `ANSI B92.1` → [`SplineStd::ANSI`]（默认 Table 2 列 A：`ANSI30平齿根齿侧`），
/// 也收五列代号 `ANSI30P`/`ANSI30PM`/`ANSI30R`/`ANSI375R`/`ANSI45R` 与全名
/// （`ANSI30平齿根外径`、`ANSI37.5圆齿根齿侧`…）；
/// `M` / `DP` 是**齿轮**体系标识，这里**不收**（由 `gear.rs` 解析）。
pub fn parse_preset_token(token: &str) -> Option<(SplineStd, &'static str)> {
    let t = token.trim().replace([' ', '\u{3000}', '°'], "");
    if t.is_empty() {
        return None;
    }
    let upper = t.to_ascii_uppercase();
    // NF E22-141 / ANSI B92.1（连字符/空格/点不敏感）—— 先于 GB/DIN 前缀判断。
    let key: String = upper.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if let Some(rest) = upper.strip_prefix("NF") {
        match rest.trim_start_matches([' ', '\u{3000}']) {
            "P" | "平" | "平齿根" => return Some((SplineStd::NF, "NF平齿根")),
            "R" | "圆" | "圆齿根" => return Some((SplineStd::NF, "NF圆齿根")),
            _ => {}
        }
    }
    if key == "NF" || key.starts_with("NFE22") {
        return Some((SplineStd::NF, NF_DEFAULT_PROFILE));
    }
    // ANSI：五列代号 / 全名；裸 `ANSI` / `ANSI B92.1` 取默认列 A。
    if key == "ANSI" || key.starts_with("ANSIB92") {
        return Some((SplineStd::ANSI, ANSI_DEFAULT_PROFILE));
    }
    if upper.starts_with("ANSI") {
        let rest: &str = &upper[4..];
        let compact: String = rest
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '.')
            .collect();
        let profile = match compact.as_str() {
            "30P" | "30平" | "30平齿根" | "30平齿根齿侧" => "ANSI30平齿根齿侧",
            "30PM" | "30M" | "30平外径" | "30平齿根外径" => "ANSI30平齿根外径",
            "30R" | "30圆" | "30圆齿根" | "30圆齿根齿侧" => "ANSI30圆齿根齿侧",
            "375R" | "375圆" | "37.5圆齿根" | "37.5圆齿根齿侧" | "375圆齿根齿侧" => {
                "ANSI37.5圆齿根齿侧"
            }
            "45R" | "45圆" | "45圆齿根" | "45圆齿根齿侧" => "ANSI45圆齿根齿侧",
            _ => return None,
        };
        return Some((SplineStd::ANSI, profile));
    }
    if let Some(rest) = upper.strip_prefix("DIN") {
        return match rest {
            "" | "30" | "30R" | "DIN30" => Some((SplineStd::DIN, "DIN30")),
            _ => None,
        };
    }
    let rest = upper.strip_prefix("GB").unwrap_or(upper.as_str());
    let profile = match rest {
        "" => "30圆齿根", // GB 默认
        "30P" | "30平" | "30平齿根" | "平齿根" => "30平齿根",
        "30R" | "30圆" | "30圆齿根" | "圆齿根" => "30圆齿根",
        "37.5R" | "375R" | "37.5圆" | "37.5圆齿根" | "375圆齿根" => "37.5圆齿根",
        "45R" | "45圆" | "45圆齿根" => "45圆齿根",
        _ => return None,
    };
    Some((SplineStd::GB, profile))
}

// ──────────────── DIN 5480-2 名义表（`d_B` 查表；数据 = assets/din5480_2_nominal.csv） ────────────────

/// 入库名义表（`crates/ocs_ocsm/assets/din5480_2_nominal.csv`，**721 行** = 旧 618 行 + m=1.5 56 行 + m=5 47 行）。
///
/// 来源 `DIN5480-2_名义表_续2_merged.csv`（665 行）剔除 p35（m=5）旧整页 47 行
/// （源图数据区渲染缺陷、双源 OCR 互不一致）；m=1.5 由用户截图 OCR 补入（r15 的 `d_b` 按
/// 该行 flags 反推修正）；m=5 由用户截图 1（表 26）补入，2 处 z 按 flags 反推、由
/// [`DIN5480_M5_NOMINAL_Z_FIXES`] 在解析时修正。`flags/source` 列原样保留便于追溯，运行时不展示。
/// 说明见 `assets/din5480_2_notes.md`。
const DIN5480_2_CSV: &str = include_str!("../assets/din5480_2_nominal.csv");

/// 名义表一行（只保留主路径需要的列；`flags/source` 留作追溯）。
#[derive(Debug, Clone, PartialEq)]
pub struct Din5480Row {
    /// 源 PDF 页码（DIN 5480-2:2015-03）。
    pub page: u16,
    /// 模数 m。
    pub m: f64,
    /// 页内表号（`Table_N`）。
    pub table_no: u32,
    /// **基准直径 `d_B`**（本表主键维度）。
    pub d_b: f64,
    /// 齿数 z。
    pub z: u32,
    /// 分度圆直径 `d = m·z`。
    pub d: f64,
    /// **基圆直径**（CSV 的 `d_b` 列 = `d·cos30°`；与基准直径 `d_B` 同名不同义）。
    pub base_dia: f64,
    /// 变位量 `x₁·m`（CSV 的 `x1_m` 列）。
    pub x_m: f64,
    /// 变位系数 `x₁ = (x₁·m)/m`（由表列派生）。
    pub x: f64,
    /// 公称齿槽宽/齿厚 `e₂ = s₁`（CSV 的 `e2_s1` 列；DIN 5480-1 §10 配合基准）。
    pub e2_s1: f64,
    /// 内花键齿根圆直径 `d_f2`（CSV 的 `d_f2` 列）。
    pub d_f2: f64,
    /// 内花键齿根圆偏差 `A_d_f2`（CSV 的 `A_df2` 列）。
    pub a_df2: f64,
    /// 内花键齿根成形圆直径 min `d_Ff2`（CSV 的 `d_Ff2min` 列）。
    pub d_ff2_min: f64,
    /// 内花键齿顶圆直径 `d_a2`（CSV 的 `d_a2` 列）。
    pub d_a2: f64,
    /// 外花键齿顶圆直径 `d_a1`（CSV 的 `d_a1` 列）。
    pub d_a1: f64,
    /// 外花键齿根成形圆直径 max `d_Ff1`（CSV 的 `d_Ff1max` 列）。
    pub d_ff1_max: f64,
    /// 外花键齿根圆直径 `d_f1`（CSV 的 `d_f1` 列）。
    pub d_f1: f64,
    /// 外花键齿根圆偏差 `A_d_f1`（CSV 的 `A_df1` 列）。
    pub a_df1: f64,
    /// OCR 质量标记（原样保留，运行时不展示）。
    pub flags: String,
    /// OCR 来源（`A+B` / `A续2+B续2` 等；原样保留，运行时不展示）。
    pub source: String,
}

/// 查表数值容差：`d_B` 表值是整数，允许输入 1e-3 误差。
const DIN_D_B_TOL: f64 = 1e-3;
/// 查表模数容差。
const DIN_M_TOL: f64 = 1e-9;
/// 表值 `x₁·m` 最多 4 位小数（x 往返误差 ≤ 5e-5/m）；1e-4 足以判同。
const DIN_X_TOL: f64 = 1e-4;

/// **m=5 名义表 z 的 OCR 修正**（`(page, table_no, d_B, 原读 z, 修正 z, 依据)`）。
///
/// 用户截图 1 的 `z` 格把个位与脚注粘连（147/160）；修正值由 `d = m·z` 及 flags 的
/// `row:vote_suspect:z:expected` 反推，与表内 `d` 列一致。**入库 CSV 保留原始 OCR 值与 flags，
/// 本表仅在解析时生效**（不篡改 assets）。
pub const DIN5480_M5_NOMINAL_Z_FIXES: &[(u16, u32, f64, u32, u32, &str)] = &[
    (
        35,
        26,
        75.0,
        147,
        14,
        "OCR 读成 147（14 与脚注粘连）；d=70 ⇒ z=d/m=14",
    ),
    (
        35,
        26,
        85.0,
        160,
        16,
        "OCR 读成 160（16 与脚注 0 粘连）；d=80 ⇒ z=d/m=16",
    ),
];

/// 应用 [`DIN5480_M5_NOMINAL_Z_FIXES`]（仅当 page/table/d_B/原 z 全命中时替换）。
fn din5480_m5_fix_z(page: u16, table_no: u32, d_b: f64, z: u32) -> u32 {
    DIN5480_M5_NOMINAL_Z_FIXES
        .iter()
        .find(|(p, t, db, zz, _, _)| {
            *p == page && *t == table_no && (*db - d_b).abs() < DIN_D_B_TOL && *zz == z
        })
        .map(|(_, _, _, _, fixed, _)| *fixed)
        .unwrap_or(z)
}

static DIN5480_TABLE: OnceLock<Vec<Din5480Row>> = OnceLock::new();

/// 入库名义表（懒加载；解析不了会 panic —— 数据随二进制编译，属构建错误）。
pub fn din5480_rows() -> &'static [Din5480Row] {
    DIN5480_TABLE.get_or_init(|| {
        parse_din5480_csv(DIN5480_2_CSV)
            .unwrap_or_else(|e| panic!("DIN 5480-2 名义表入库数据损坏：{e}"))
    })
}

/// 已入库模数档（升序；m=5 已由用户截图补入，2026-09-21 起无缺失档）。
pub fn din5480_modules() -> Vec<f64> {
    let mut v: Vec<f64> = din5480_rows().iter().map(|r| r.m).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v.dedup_by(|a, b| (*a - *b).abs() < DIN_M_TOL);
    v
}

/// 拆分一行 CSV（本表十进制逗号字段被双引号包住；无转义引号）。
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

/// 十进制字段（CSV 用逗号作小数点）。
fn csv_decimal(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', ".");
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// 解析入库 CSV（跳过 `#` 注释与表头；缺列/坏值 → 带行号报错）。
fn parse_din5480_csv(text: &str) -> Result<Vec<Din5480Row>, String> {
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let f = split_csv_line(line);
        if f.first().map(|s| s.trim()) == Some("page") {
            continue; // 表头
        }
        if f.len() < 19 {
            return Err(format!("第 {n} 行只有 {} 列（应有 19 列）", f.len()));
        }
        let page = f[0]
            .trim()
            .parse::<u16>()
            .map_err(|_| format!("第 {n} 行 page「{}」非法", f[0]))?;
        let m = csv_decimal(&f[1]).ok_or_else(|| format!("第 {n} 行 m「{}」非法", f[1]))?;
        let table_no = f[2]
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("第 {n} 行 table_no「{}」非法", f[2]))?;
        let d_b = csv_decimal(&f[3]).ok_or_else(|| format!("第 {n} 行 d_B「{}」非法", f[3]))?;
        let z = f[4]
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("第 {n} 行 z「{}」非法", f[4]))?;
        let z = din5480_m5_fix_z(page, table_no, d_b, z);
        let d = csv_decimal(&f[5]).ok_or_else(|| format!("第 {n} 行 d「{}」非法", f[5]))?;
        let base_dia =
            csv_decimal(&f[6]).ok_or_else(|| format!("第 {n} 行 d_b「{}」非法", f[6]))?;
        let x_m = csv_decimal(&f[7]).ok_or_else(|| format!("第 {n} 行 x1_m「{}」非法", f[7]))?;
        let e2_s1 = csv_decimal(&f[8]).ok_or_else(|| format!("第 {n} 行 e2_s1「{}」非法", f[8]))?;
        let d_f2 = csv_decimal(&f[9]).ok_or_else(|| format!("第 {n} 行 d_f2「{}」非法", f[9]))?;
        let a_df2 = csv_decimal(&f[10]).ok_or_else(|| format!("第 {n} 行 A_df2「{}」非法", f[10]))?;
        let d_ff2_min =
            csv_decimal(&f[11]).ok_or_else(|| format!("第 {n} 行 d_Ff2min「{}」非法", f[11]))?;
        let d_a2 = csv_decimal(&f[12]).ok_or_else(|| format!("第 {n} 行 d_a2「{}」非法", f[12]))?;
        let d_a1 = csv_decimal(&f[13]).ok_or_else(|| format!("第 {n} 行 d_a1「{}」非法", f[13]))?;
        let d_ff1_max =
            csv_decimal(&f[14]).ok_or_else(|| format!("第 {n} 行 d_Ff1max「{}」非法", f[14]))?;
        let d_f1 = csv_decimal(&f[15]).ok_or_else(|| format!("第 {n} 行 d_f1「{}」非法", f[15]))?;
        let a_df1 = csv_decimal(&f[16]).ok_or_else(|| format!("第 {n} 行 A_df1「{}」非法", f[16]))?;
        if m <= 0.0 {
            return Err(format!("第 {n} 行 m={m} 非正"));
        }
        rows.push(Din5480Row {
            page,
            m,
            table_no,
            d_b,
            z,
            d,
            base_dia,
            x_m,
            x: x_m / m,
            e2_s1,
            d_f2,
            a_df2,
            d_ff2_min,
            d_a2,
            d_a1,
            d_ff1_max,
            d_f1,
            a_df1,
            flags: f[17].trim().to_string(),
            source: f[18].trim().to_string(),
        });
    }
    Ok(rows)
}

/// 命中 `(d_B, m?)` 的原始行（不产生错误文案；[`lookup_by_d_b`] 的错误路径也用它）。
fn din5480_match(d_b: f64, m: Option<f64>) -> Vec<Din5480Row> {
    din5480_rows()
        .iter()
        .filter(|r| (r.d_b - d_b).abs() < DIN_D_B_TOL)
        .filter(|r| m.is_none_or(|m| (r.m - m).abs() < DIN_M_TOL))
        .cloned()
        .collect()
}

/// **DIN 5480-2 名义表查表**：按基准直径 `d_B`（可再限定模数 `m`）返回名义行。
///
/// * 命中可能**多行** —— 同一 `d_B` 有多个 `z/x₁` 变体是表格事实（如 p21 m=1.25、p25 m=1.75）；
/// * 找不到时列出该 `d_B` 附近候选（同 m 优先；无同 m 行退全表）；
/// * `m=1.5`、`m=5` 均已由用户截图补入，可正常命中（m=5 的 2 处 z 解析时按修正表生效）。
pub fn lookup_by_d_b(d_b: f64, m: Option<f64>) -> Result<Vec<Din5480Row>, String> {
    if !(d_b.is_finite() && d_b > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.din.lookup_db_positive", &[("v0", &trim(d_b))]));
    }
    if let Some(m) = m {
        if !(m.is_finite() && m > 0.0) {
            return Err(crate::i18n::t_fmt("cmd.invol.din.lookup_m_positive", &[("v0", &trim(m))]));
        }
    }
    let mut hits = din5480_match(d_b, m);
    hits.sort_by(|a, b| a.z.cmp(&b.z).then(a.m.partial_cmp(&b.m).unwrap()));
    if hits.is_empty() {
        Err(din5480_miss_message(d_b, m))
    } else {
        Ok(hits)
    }
}

/// 未命中的错误文案：缺失档位说明 + 附近候选。
fn din5480_miss_message(d_b: f64, m: Option<f64>) -> String {
    let mut msg = crate::i18n::t_fmt("cmd.invol.din.lookup_head", &[("v0", &trim(d_b))]);
    if let Some(m) = m {
        msg.push_str(&crate::i18n::t_fmt("cmd.invol.din.lookup_head_m", &[("v0", &trim(m))]));
    }
    msg.push_str(&crate::i18n::t("cmd.invol.din.lookup_no_hit"));
    if let Some(m) = m {
        if !din5480_modules()
            .iter()
            .any(|mm| (mm - m).abs() < DIN_M_TOL)
        {
            let list = din5480_modules()
                .iter()
                .map(|v| trim(*v))
                .collect::<Vec<_>>()
                .join(&crate::i18n::t("cmd.invol.sep.list"));
            msg.push_str(&crate::i18n::t_fmt("cmd.invol.table.no_module_step", &[("v0", &trim(m)), ("v1", &list)]));
        } else {
            msg.push('。');
        }
    } else {
        msg.push('。');
    }
    // 附近候选：同 m（若给了 m）按 |Δd_B| 取最近的 5 个不同 d_B；缺失档退全表。
    let mut pool: Vec<&Din5480Row> = din5480_rows()
        .iter()
        .filter(|r| m.is_none_or(|m| (r.m - m).abs() < DIN_M_TOL))
        .collect();
    if pool.is_empty() {
        pool = din5480_rows().iter().collect();
    }
    pool.sort_by(|a, b| {
        (a.d_b - d_b)
            .abs()
            .partial_cmp(&(b.d_b - d_b).abs())
            .unwrap()
    });
    let mut seen: Vec<f64> = Vec::new();
    let mut cands: Vec<String> = Vec::new();
    for r in pool {
        if seen.iter().any(|v| (*v - r.d_b).abs() < DIN_D_B_TOL) {
            continue;
        }
        seen.push(r.d_b);
        cands.push(crate::i18n::t_fmt("cmd.invol.din.cand_row", &[("v0", &trim(r.d_b)), ("v1", &trim(r.m))]));
        if cands.len() >= 5 {
            break;
        }
    }
    if !cands.is_empty() {
        msg.push_str(&crate::i18n::t_fmt("cmd.invol.table.candidates", &[("v0", &cands.join(&crate::i18n::t("cmd.invol.sep.list")))]));
    }
    msg
}

/// DIN 5480-2 基准直径反解变位系数：
/// `x = (d_B − m(z + 1.1)) / (2m)`，等价 `d_B = d + 1.1m + 2x·m`（`z_B = z + 1.1 + 2x`）。
///
/// **口径**：这是从 OCR 名义表反推并经**全表校验**的关系 —— 已解析 721 行逐行
/// `|x₁·m − (d_B − m(z+1.1))/2| = 0`；**不是标准原文公式**，
/// 引用时以 DIN 5480-2 表值与检验表为准。
pub fn x_from_d_b(d_b: f64, m: f64, z: u32) -> f64 {
    (d_b - m * (z as f64 + 1.1)) / (2.0 * m)
}

/// 由 `m/z/x` 正算基准直径 `d_B = m(z + 1.1 + 2x)`（[`x_from_d_b`] 的逆）。
pub fn d_b_from_x(m: f64, z: u32, x: f64) -> f64 {
    m * (z as f64 + 1.1 + 2.0 * x)
}

/// 基准直径主参数补全/推导的来源（DIN `d_B` / NF `A`；派生值显示用）。
#[derive(Debug, Clone, PartialEq)]
pub enum D_bOrigin {
    /// 查表命中该行。
    Table(Din5480Row),
    /// 公式解出（未命中表；`d_B+m+z` 相容路径）。
    Formula,
    /// 无 `d_B` 输入，由 `m/z/x` 正算 `d_B`（`m+z` 路径）。
    Computed,
    /// 表外由 `d_B` 公式**推导** z/m（附可行区间依据）。
    Derived(String),
    /// `d_B` 为主参数、输入 m/z/x 与其不符 → 按 `d_B` 重算（附明文提示）。
    Adjusted(String),
    /// NF E22-141 尺寸表命中（`A` 为主参数）。
    NfTable(NfE22141Row),
    /// NF 无 `A` 输入，由 `m/z/x` 正算 `A`（`m+z` 路径）。
    ComputedA,
}

impl D_bOrigin {
    /// 给用户看的来源说明（GUI/命令行派生值文本用）。
    pub fn note(&self) -> String {
        match self {
            Self::Table(r) => crate::i18n::t_fmt("cmd.invol.din.origin_table", &[("v0", &r.page.to_string()), ("v1", &trim(r.m))]),
            Self::Formula => crate::i18n::t_static("cmd.invol.din.origin_formula").to_string(),
            Self::Computed => crate::i18n::t_static("cmd.invol.din.origin_computed").to_string(),
            Self::Derived(n) | Self::Adjusted(n) => n.clone(),
            Self::NfTable(r) => crate::i18n::t_fmt("cmd.invol.nf.origin_table", &[("v0", &r.page.to_string()), ("v1", &trim(r.m)), ("v2", &trim(r.a))]),
            Self::ComputedA => crate::i18n::t_static("cmd.invol.nf.origin_computed_a").to_string(),
        }
    }
}

/// 由 `d_B + m` 反求齿数 z：`d_B=m(z+1.1+2x)`、`x∈[−0.05,0.45]`
/// → `z∈[d_B/m−2.0, d_B/m−1.0]`；区间内多个整数时，`prefer` 给定就取离它最近的，
/// 否则取 |x| 最小（x 最接近 0）的。返回 `(z, 区间下界, 区间上界)`。
fn derive_z_from_d_b_m(
    d_b: f64,
    m: f64,
    prefer: Option<u32>,
) -> Result<(u32, f64, f64), String> {
    if !(m.is_finite() && m > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.din.m_positive", &[("v0", &trim(m))]));
    }
    let ratio = d_b / m;
    let z_lo_f = ratio - 2.0;
    let z_hi_f = ratio - 1.0;
    let z_lo = (z_lo_f - 1e-9).ceil();
    let z_hi = (z_hi_f + 1e-9).floor();
    if z_hi < z_lo || z_hi < 3.0 || z_lo > 1000.0 {
        return Err(crate::i18n::t_fmt("cmd.invol.din.z_no_feasible", &[("v0", &trim(d_b)), ("v1", &trim(m)), ("v2", &format!("{:.4}", z_lo_f)), ("v3", &format!("{:.4}", z_hi_f))]));
    }
    let candidates: Vec<u32> = (z_lo.max(3.0) as u32..=z_hi.min(1000.0) as u32).collect();
    let best = candidates
        .iter()
        .copied()
        .min_by(|&a, &b| {
            let xa = x_from_d_b(d_b, m, a).abs();
            let xb = x_from_d_b(d_b, m, b).abs();
            match prefer {
                Some(pz) => (a as i64 - pz as i64)
                    .abs()
                    .cmp(&(b as i64 - pz as i64).abs())
                    .then(xa.partial_cmp(&xb).unwrap())
                    .then(a.cmp(&b)),
                None => xa.partial_cmp(&xb).unwrap().then(a.cmp(&b)),
            }
        })
        .expect("候选非空");
    Ok((best, z_lo_f, z_hi_f))
}

/// 用 `d_B` 补全/校验 DIN 参数（`m/z/x` 缺哪项补哪项；主路径入口）。
///
/// **`d_B` 是主参数**（用户定案）：
/// * `d_B + m + z`：`x=(d_B−m(z+1.1))/(2m)`，在 [−0.05,0.45] 内就认（命中表行时用表值 x）；
///   不在范围内 → **按 `d_B` 重算 z**（保留 m）并明文提示，不报“组合不一致”；
///   `x` 输入与 `d_B` 不符时也明文提示后**按 `d_B` 取 x**；
/// * `d_B + m`：查表补 `z`；表外按公式推 z（标注“推导值、未命中表”+ 区间依据）；
/// * `d_B + z`：查表补 `m`；表外按公式给名义 m（取 x=0）并标注来源；
/// * 只给 `d_B`：查表补 `m/z`（多命中报候选）。
///
/// 命中表行时 `x` 取表值（精度优先），来源 [`D_bOrigin::Table`]；否则 [`D_bOrigin::Formula`]。
pub fn resolve_din_by_d_b(
    d_b: f64,
    m: Option<f64>,
    z: Option<u32>,
    x: Option<f64>,
) -> Result<(InvolParams, D_bOrigin), String> {
    if !(d_b.is_finite() && d_b > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.din.db_positive", &[("v0", &trim(d_b))]));
    }
    if let Some(m) = m {
        if !(m.is_finite() && m > 0.0) {
            return Err(crate::i18n::t_fmt("cmd.invol.din.m_positive", &[("v0", &trim(m))]));
        }
        // 模数必须来自 DIN 5480-2 名义表的实际 m 列（候选随表走，避免系列漂移）。
        module_series_check(SplineStd::DIN, m)?;
    }
    match (m, z) {
        (Some(m), Some(z)) => resolve_with_m_z(d_b, m, z, x),
        (Some(m), None) => resolve_with_m(d_b, m),
        (None, Some(z)) => resolve_with_z(d_b, z),
        (None, None) => {
            let hits = din5480_match(d_b, None);
            if hits.is_empty() {
                return Err(lookup_by_d_b(d_b, None).unwrap_err());
            }
            if hits.len() > 1 {
                return Err(crate::i18n::t_fmt("cmd.invol.din.db_multi_hit", &[("v0", &trim(d_b)), ("v1", &hits.len().to_string()), ("v2", &join_rows(&hits))]));
            }
            row_to_params(hits[0].clone())
        }
    }
}

/// `d_B + m`：查表补 z；表外按公式推 z。
fn resolve_with_m(d_b: f64, m: f64) -> Result<(InvolParams, D_bOrigin), String> {
    let mut hits = din5480_match(d_b, Some(m));
    let mut zs: Vec<u32> = hits.iter().map(|r| r.z).collect();
    zs.sort_unstable();
    zs.dedup();
    if zs.len() > 1 {
        return Err(crate::i18n::t_fmt("cmd.invol.din.db_m_multi_z", &[("v0", &trim(d_b)), ("v1", &trim(m)), ("v2", &join_rows(&hits))]));
    }
    if zs.len() == 1 {
        hits.sort_by_key(|r| r.z);
        return row_to_params(hits.remove(0));
    }
    // 表外：按公式推 z 并标注区间依据。
    let (z, z_lo, z_hi) = derive_z_from_d_b_m(d_b, m, None)?;
    let x = x_from_d_b(d_b, m, z);
    let note = crate::i18n::t_fmt("cmd.invol.din.derive_z_note", &[("v0", &trim(d_b)), ("v1", &trim(m)), ("v2", &format!("{:.4}", z_lo)), ("v3", &format!("{:.4}", z_hi)), ("v4", &z.to_string()), ("v5", &trim(x))]);
    let p = InvolParams::din(m, z, x)
        .map_err(|e| crate::i18n::t_fmt("cmd.invol.din.prefix", &[("v0", &e)]))?
        .with_d_b(d_b);
    Ok((p, D_bOrigin::Derived(note)))
}

/// `d_B + z`：查表补 m；表外按公式给名义 m（取 x=0）并标注来源。
fn resolve_with_z(d_b: f64, z: u32) -> Result<(InvolParams, D_bOrigin), String> {
    let hits: Vec<Din5480Row> = din5480_match(d_b, None)
        .into_iter()
        .filter(|r| r.z == z)
        .collect();
    if !hits.is_empty() {
        let mut ms: Vec<f64> = hits.iter().map(|r| r.m).collect();
        ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        ms.dedup_by(|a, b| (*a - *b).abs() < DIN_M_TOL);
        if ms.len() > 1 {
            return Err(crate::i18n::t_fmt("cmd.invol.din.db_z_multi_m", &[("v0", &trim(d_b)), ("v1", &z.to_string()), ("v2", &join_rows(&hits))]));
        }
        return row_to_params(hits[0].clone());
    }
    // 表外：由 d_B=m(z+1.1+2x) 取 x=0 得名义 m = d_B/(z+1.1)。
    let m = d_b / (z as f64 + 1.1);
    let m_lo = d_b / (z as f64 + 2.0);
    let m_hi = d_b / (z as f64 + 1.0);
    let note = crate::i18n::t_fmt("cmd.invol.din.derive_m_note", &[("v0", &trim(d_b)), ("v1", &z.to_string()), ("v2", &trim(m)), ("v3", &format!("{:.4}", m_lo)), ("v4", &format!("{:.4}", m_hi))]);
    let p = InvolParams::din(m, z, 0.0)
        .map_err(|e| crate::i18n::t_fmt("cmd.invol.din.prefix", &[("v0", &e)]))?
        .with_d_b(d_b);
    Ok((p, D_bOrigin::Derived(note)))
}

/// `d_B + m + z`：`x` 由公式解出；不兼容时以 `d_B` 为准重算 z（保留 m）并明文提示。
///
/// `d_B` 为主参数：显式 `z` 与 `(d_B, m)` 推导不符、且不在 `(d_B, m)` 表内变体里时，
/// **按 `d_B` 取推导 z** 并附明文提示（不静默改）。
fn resolve_with_m_z(
    d_b: f64,
    m: f64,
    z: u32,
    x: Option<f64>,
) -> Result<(InvolParams, D_bOrigin), String> {
    let (z_canon, _) = derive_z_from_bench_m(SplineStd::DIN, d_b, m)?;
    let in_table_for_z = din5480_rows()
        .iter()
        .any(|r| (r.m - m).abs() < DIN_M_TOL && r.z == z && (r.d_b - d_b).abs() < DIN_D_B_TOL);
    if z != z_canon && !in_table_for_z {
        let x2 = x_from_d_b(d_b, m, z_canon);
        let z_lo = d_b / m - 2.0;
        let z_hi = d_b / m - 1.0;
        let mut note = crate::i18n::t_fmt("cmd.invol.din.keep_z_from_db_m", &[("v0", &trim(d_b)), ("v1", &z_canon.to_string()), ("v2", &z.to_string()), ("v3", &format!("{:.4}", z_lo)), ("v4", &format!("{:.4}", z_hi))]);
        if let Some(xi) = x {
            if (xi - x2).abs() > DIN_X_TOL {
                note.push_str(&crate::i18n::t_fmt("cmd.invol.din.also_x", &[("v0", &trim(x2)), ("v1", &trim(xi))]));
            }
        }
        let p = InvolParams::din(m, z_canon, x2)
            .map_err(|e| crate::i18n::t_fmt("cmd.invol.din.prefix", &[("v0", &e)]))?
            .with_d_b(d_b);
        return Ok((p, D_bOrigin::Adjusted(note)));
    }
    let x_formula = x_from_d_b(d_b, m, z);
    let in_range = (-0.05 - 1e-9..=0.45 + 1e-9).contains(&x_formula);
    if in_range {
        // 命中同 (m, z, d_B) 的表行 → 用表值 x（精度优先）。
        let row = din5480_rows()
            .iter()
            .find(|r| (r.m - m).abs() < DIN_M_TOL && r.z == z && (r.d_b - d_b).abs() < DIN_D_B_TOL)
            .cloned();
        let (xv, origin) = match row {
            Some(row) => {
                let origin = match x {
                    Some(xi) if (xi - row.x).abs() > DIN_X_TOL => D_bOrigin::Adjusted(crate::i18n::t_fmt("cmd.invol.din.table_hit_x", &[("v0", &row.page.to_string()), ("v1", &trim(row.m)), ("v2", &trim(d_b)), ("v3", &trim(row.x)), ("v4", &trim(xi))])),
                    _ => D_bOrigin::Table(row.clone()),
                };
                (row.x, origin)
            }
            None => {
                let origin = match x {
                    Some(xi) if (xi - x_formula).abs() > DIN_X_TOL => D_bOrigin::Adjusted(crate::i18n::t_fmt("cmd.invol.din.keep_x_from_db", &[("v0", &trim(d_b)), ("v1", &trim(x_formula)), ("v2", &trim(xi))])),
                    _ => D_bOrigin::Formula,
                };
                (x_formula, origin)
            }
        };
        let p = InvolParams::din(m, z, xv)
            .map_err(|e| crate::i18n::t_fmt("cmd.invol.din.prefix", &[("v0", &e)]))?
            .with_d_b(d_b);
        return Ok((p, origin));
    }
    // 不相容 → 以 d_B 为准，保留 m 重算 z。
    let (z2, z_lo, z_hi) = derive_z_from_d_b_m(d_b, m, Some(z))?;
    let x2 = x_from_d_b(d_b, m, z2);
    let mut note = crate::i18n::t_fmt("cmd.invol.din.keep_z_from_db", &[("v0", &trim(d_b)), ("v1", &z2.to_string()), ("v2", &z.to_string()), ("v3", &format!("{:.4}", z_lo)), ("v4", &format!("{:.4}", z_hi))]);
    if let Some(xi) = x {
        if (xi - x2).abs() > DIN_X_TOL {
            note.push_str(&crate::i18n::t_fmt("cmd.invol.din.also_x", &[("v0", &trim(x2)), ("v1", &trim(xi))]));
        }
    }
    let p = InvolParams::din(m, z2, x2)
        .map_err(|e| crate::i18n::t_fmt("cmd.invol.din.prefix", &[("v0", &e)]))?
        .with_d_b(d_b);
    Ok((p, D_bOrigin::Adjusted(note)))
}

/// 表行 → 参数（用表值 `x`，来源 [`D_bOrigin::Table`]）。
fn row_to_params(row: Din5480Row) -> Result<(InvolParams, D_bOrigin), String> {
    let p = InvolParams::din(row.m, row.z, row.x)
        .map_err(|e| {
            crate::i18n::t_fmt("cmd.invol.din.row_inconsistent", &[("v0", &row.page.to_string()), ("v1", &trim(row.m)), ("v2", &row.z.to_string()), ("v3", &trim(row.x)), ("v4", &e)])
        })?
        .with_d_b(row.d_b);
    Ok((p, D_bOrigin::Table(row)))
}

/// **通用花键参数解析**（齿轮生成器花键模式 / 轴段 / 测试共用）：
///
/// **体系由 `std` 显式给出**（不再从 `d_B` 反推）：
/// * `NF`：走独立分支 —— 第 3 参 `d_b` 槽位收 **公称直径 A**（NF 的基准直径主参数），
///   由 [`resolve_nf_by_a`] 查表/推导；A + m 或 A + z 可缺一项；
/// * `ANSI`：走独立分支 —— **第 5 参 `m` 槽位收径节 `P`**（不是模数；引擎内 `m = 25.4/P`，mm），
///   `z` = 齿数 N；误给 `d_B`/A 报 [`ANSI_D_B_MSG`]；ANSI 无变位（x≠0 报错）；
/// * `d_B` 给了：只有 DIN 有这个概念 —— GB 直接报 [`GB_D_B_MSG`]；DIN 走 [`resolve_din_by_d_b`]；
/// * GB 给非零 `x`：直接报 [`GB_X_MSG`]（基本齿廓无变位；GUI 已锁死为 0）；
/// * 不给 `d_B`：GB/DIN/NF 都要 `m` 与 `z`（缺哪项报哪项）；DIN 由 `m/z/x` 正算 `d_B`、NF 由
///   `m/z/x` 正算 A（来源 [`D_bOrigin::Computed`] / [`D_bOrigin::ComputedA`]）。
///
/// 返回 `(参数, 基准直径来源)`；GB/ANSI 的来源恒为 `None`。
pub fn resolve_spline(
    std: SplineStd,
    profile: &str,
    d_b: Option<f64>,
    m: Option<f64>,
    z: Option<u32>,
    x: Option<f64>,
) -> Result<(InvolParams, Option<D_bOrigin>), String> {
    let profile = if profile.trim().is_empty() {
        default_profile(std)
    } else {
        profile
    };
    // 收“齿廓代号”（GB30R/DIN30/NFP/NFR/ANSI30P…）与中文/英文齿廓名；代号必须与 std 一致。
    let profile = match parse_preset_token(profile) {
        Some((pstd, name)) => {
            if pstd != std {
                return Err(crate::i18n::t_fmt(
                    "cmd.invol.err.profile_std_mismatch",
                    &[("profile", profile), ("pstd", pstd.code()), ("std", std.code())],
                ));
            }
            name
        }
        None => profile,
    };
    // 体系模数校验：显式给的 m 必须在该体系候选里（表外推导的 m 不拦；ANSI 在校验径节时统一处理）。
    if std != SplineStd::ANSI {
        if let Some(mv) = m {
            module_series_check(std, mv)?;
        }
    }
    // ANSI：公式驱动 —— 第 5 参槽位 = 径节 P；无 d_B/A、无变位。
    if std == SplineStd::ANSI {
        if d_b.is_some() {
            return Err(ansi_d_b_msg());
        }
        let p = m.ok_or_else(|| crate::i18n::t("cmd.invol.err.ansi_missing_p"))?;
        let z = z.ok_or_else(|| crate::i18n::t("cmd.invol.err.ansi_missing_n"))?;
        if let Some(xv) = x {
            if xv.abs() > 1e-12 {
                return Err(crate::i18n::t_fmt(
                    "cmd.invol.err.ansi_x",
                    &[("x", &trim(xv))],
                ));
            }
        }
        let p = InvolParams::ansi(profile, p, z)?;
        return Ok((p, None));
    }
    // NF：`d_b` 槽位 = 公称直径 A（NF 主参数）。
    if std == SplineStd::NF {
        if let Some(a) = d_b {
            let (p, origin) = resolve_nf_by_a(a, m, z, x, profile)?;
            return Ok((p, Some(origin)));
        }
        let m = m.ok_or_else(|| crate::i18n::t("cmd.invol.err.nf_missing_m"))?;
        let z = z.ok_or_else(|| crate::i18n::t("cmd.invol.err.nf_missing_n"))?;
        let xv = x.unwrap_or(0.8);
        let a = a_from_x(m, z, xv);
        let p = InvolParams::from_preset(SplineStd::NF, profile, m, z)
            .map_err(|e| crate::i18n::t_fmt("cmd.invol.err.nf_prefix", &[("e", &e)]))?
            .with_x(xv)
            .with_a(a);
        p.validate().map_err(|e| crate::i18n::t_fmt("cmd.invol.err.nf_prefix", &[("e", &e)]))?;
        return Ok((p, Some(D_bOrigin::ComputedA)));
    }
    if let Some(d_b) = d_b {
        if std != SplineStd::DIN {
            return Err(gb_d_b_msg());
        }
        let (p, origin) = resolve_din_by_d_b(d_b, m, z, x)?;
        return Ok((p, Some(origin)));
    }
    // GB/T 3478.1 基本齿廓不含变位：入口层明确拒绝非零 x（与 d_B/径节口径一致）。
    // 不在 `InvolParams::validate` 里拦：引擎原语仍可被内部数学对照测试用 `with_x` 构造。
    if std == SplineStd::GB {
        if let Some(xv) = x {
            if xv.abs() > 1e-12 {
                return Err(crate::i18n::t_fmt(
                    "cmd.invol.err.gb_x_received",
                    &[("msg", &gb_x_msg()), ("x", &trim(xv))],
                ));
            }
        }
    }
    let m = m.ok_or_else(|| {
        crate::i18n::t(match std {
            SplineStd::GB => "cmd.invol.err.missing_m_gb",
            SplineStd::DIN => "cmd.invol.err.missing_m_din",
            SplineStd::NF => "cmd.invol.err.missing_m_nf",
            SplineStd::ANSI => "cmd.invol.err.missing_m_ansi",
        })
    })?;
    let z = z.ok_or_else(|| {
        crate::i18n::t(match std {
            SplineStd::GB => "cmd.invol.err.missing_z_gb",
            SplineStd::DIN => "cmd.invol.err.missing_z_din",
            SplineStd::NF => "cmd.invol.err.missing_z_nf",
            SplineStd::ANSI => "cmd.invol.err.missing_z_ansi",
        })
    })?;
    let mut p = InvolParams::from_preset(std, profile, m, z)?
        .with_x(x.unwrap_or(0.0));
    p.validate().map_err(|e| format!("{e}"))?;
    if std == SplineStd::DIN {
        let d_b = d_b_from_x(p.m, p.z, p.x);
        p = p.with_d_b(d_b);
        return Ok((p, Some(D_bOrigin::Computed)));
    }
    Ok((p, None))
}

/// 行列表（错误信息里的候选展示）。
fn join_rows(rows: &[Din5480Row]) -> String {
    rows.iter()
        .map(|r| crate::i18n::t_fmt("cmd.invol.din.rows_join", &[("v0", &trim(r.m)), ("v1", &r.z.to_string()), ("v2", &trim(r.x))]))
        .collect::<Vec<_>>()
        .join(&crate::i18n::t("cmd.invol.sep.list"))
}

// ──────────────── NF E22-141 尺寸表（`A` 查表；数据 = assets/nf_e22141_dims.csv） ────────────────

/// 入库尺寸表（`crates/ocs_ocsm/assets/nf_e22141_dims.csv`，**288 行**：
/// p18 拉削内花键 144 + p20 尺寸表(m=0.50~1.25) 39 + p21 尺寸表(m=1.667~3.75) 49 +
/// p22 尺寸表(m=5.00~10.00) 56）。
///
/// 来源：`~/桌面/OCSM/review/花键标准资料/NF-E22-141_尺寸表.csv`（docin 抓的 NF E22-141
/// 中文译本 p18/p20/p21/p22 的 OCR 抄录）；`page/table_no/source/flags` 原样保留。
/// **已知 OCR 错格**经逐张核对源图后进 [`NF_OCR_FIXES`]（运行期应用，不改写入库 CSV）；
/// **9 行疑原表印误**见 [`NF_SUSPECTED_SOURCE_ERRORS`]，只标注不改值。
/// 完整说明见 `assets/nf_e22141_notes.md`。
const NF_E22141_CSV: &str = include_str!("../assets/nf_e22141_dims.csv");

/// 查表 `A` 容差：表值是整数（个别一位小数），允许输入 1e-3 误差。
const NF_A_TOL: f64 = 1e-3;
/// 查表模数容差。
const NF_M_TOL: f64 = 1e-9;
/// 表值 `x` 判同容差（表值最多 3 位小数）。
const NF_X_TOL: f64 = 1e-4;
/// 表值 `x` 与 A 公式判同容差：m=1.667 = 5/3 的舍入（`m·z` 与 `m(z+2x+0.4)`
/// 都最多偏 ~3.6e-3），排版精度下的等价值。
const NF_X_MATCH_TOL: f64 = 5e-3;

/// NF 尺寸表一行（保留主路径需要的列；`flags/source` 留作追溯）。
#[derive(Debug, Clone, PartialEq)]
pub struct NfE22141Row {
    /// 源图页码（18/20/21/22）。
    pub page: u16,
    /// 页内表号/子表（`尺寸表(m=0.50~1.25)` 或 `p18L`/`p18R`）。
    pub table_no: String,
    /// OCR 来源（`p20`/`p18L`/`p21`/`p22`）。
    pub source: String,
    /// 模数 m。
    pub m: f64,
    /// **公称直径 `A`（主参数；NF 的基准直径）**。
    pub a: f64,
    /// 齿数 N。
    pub z: u32,
    /// 分度圆直径 `d = m·z`（p18 行只有 D 列，为 `None`）。
    pub d: Option<f64>,
    /// 基圆直径（CSV 的 `dB` 列 = `d·cos20°`；**不是** DIN 的基准直径 `d_B`）。
    pub base_dia: Option<f64>,
    /// 变位系数 `x = (A − m(z+0.4))/(2m)`。
    pub x: Option<f64>,
    /// 分度圆弧齿厚 `s`。
    pub s: Option<f64>,
    /// 基圆弧齿厚 `sB`。
    pub s_b: Option<f64>,
    /// 外花键齿根圆（平齿根）`A−2.4m`。
    pub flat_root: Option<f64>,
    /// 外花键齿根圆（圆齿根）`A−2.694m`。
    pub round_root: Option<f64>,
    /// 外花键齿顶倒角高度 `h=0.1·max(m,1)`。
    pub h: Option<f64>,
    /// 外花键齿根圆角半径（平齿根）`R=0.3m`。
    pub r: Option<f64>,
    /// 外花键齿根圆角半径（圆齿根）`Ri=0.528m`。
    pub r_i: Option<f64>,
    /// 内花键齿顶圆（小径）`D = A−2m`（CSV 列名为“齿根圆直径D(拉削内花键)”，
    /// 对照 p07 公式实为小径 D；p18 拉削内花键表）。
    pub internal_tip: Option<f64>,
    /// 内花键槽底圆角半径（平根齿）。
    pub hub_fillet: Option<f64>,
    /// OCR 质量标记（原样保留，运行时不展示）。
    pub flags: String,
    /// 已应用的 OCR 修正（人类可读，如 `x 0.967→0.633`；空 = 未修）。
    pub fixes: Vec<String>,
}

/// NF 表中可被 OCR 修正的列。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NfField {
    /// 齿数 N（列 5）。
    Z,
    /// 变位系数 x（列 8）。
    X,
    /// 分度圆弧齿厚 s（列 11）。
    S,
    /// 基圆弧齿厚 sB（列 12）。
    SB,
    /// 分度圆直径 d（列 9）。
    PitchD,
    /// 基圆直径 dB = d·cos20°（列 10）。
    BaseDia,
    /// 内花键小径 D（列 6，p18）。
    InternalD,
    /// 外花键齿根圆（平齿根）（列 7）。
    FlatRoot,
    /// 外花键齿根圆（圆齿根）（列 15）。
    RoundRoot,
    /// 外花键齿顶倒角高度 h（列 16）。
    H,
}

/// 一条 OCR 修正：`key = (page, m, A, field)`；`printed → fixed`。
///
/// 入库 CSV 保留印刷原值，运行期查表/推导时按本表修正（既有 DIN `INSPECTION_OCR_FIXES` 惯例）。
#[derive(Debug, Clone, Copy)]
pub struct NfOcrFix {
    /// 源图页码。
    pub page: u16,
    /// 模数 m。
    pub m: f64,
    /// 公称直径 A。
    pub a: f64,
    /// 被修正的列。
    pub field: NfField,
    /// OCR 读到的印刷值（用于核对确实命中原行原格）。
    pub printed: f64,
    /// 修正值（依据见 `basis`）。
    pub fixed: f64,
    /// 判定依据（源图/表内自洽关系）。
    pub basis: &'static str,
}

/// **NF E22-141 已知 OCR 错格修正表**（不篡改入库 CSV；逐条注明依据）。
///
/// 其中 m=3.75/7.50 的 `x=0.633/0.967` 交替本身是**次系列真实设计值**（任务明确），
/// 本表只修「整格读错/丢首位字母/跨格粘连」与「相邻两行局部互换」这类 OCR 错误；
/// 值本身可疑、无法与 OCR 区分开的 9 行见 [`NF_SUSPECTED_SOURCE_ERRORS`]，**不改**。
pub const NF_OCR_FIXES: &[NfOcrFix] = &[
    NfOcrFix { page: 22, m: 5.00, a: 105.0, field: NfField::Z, printed: 192.0, fixed: 19.0,
        basis: "192 = 19 与邻格尾数 2 粘连；p18 同 (m,A) N=19，且 d=95、sB=11.447 按 z=19 自洽" },
    NfOcrFix { page: 21, m: 1.667, a: 45.0, field: NfField::PitchD, printed: 4.667, fixed: 41.667,
        basis: "印 4.667 丢首位 1；表内 dB=39.154=41.667·cos20° 佐证" },
    NfOcrFix { page: 21, m: 2.50, a: 100.0, field: NfField::BaseDia, printed: 85.271, fixed: 89.271,
        basis: "印 85.271，末位 5↔9；d·cos20°=95·cos20°=89.271" },
    NfOcrFix { page: 21, m: 3.75, a: 80.0, field: NfField::PitchD, printed: 7.25, fixed: 71.25,
        basis: "印 7.25 丢首位 1；m·z=71.25" },
    NfOcrFix { page: 21, m: 3.75, a: 130.0, field: NfField::PitchD, printed: 23.75, fixed: 123.75,
        basis: "印 23.75 丢首位 1；m·z=123.75，dB=116.287 佐证" },
    NfOcrFix { page: 21, m: 3.75, a: 130.0, field: NfField::BaseDia, printed: 116.287877, fixed: 116.287,
        basis: "印 116.287877（跨格粘连尾数 877）；123.75·cos20°=116.287" },
    NfOcrFix { page: 21, m: 3.75, a: 130.0, field: NfField::X, printed: 0.967, fixed: 0.633,
        basis: "印 0.967（与 A=140 行局部互换）；A 方程与 sB=8.892 反推 0.633" },
    NfOcrFix { page: 21, m: 3.75, a: 130.0, field: NfField::S, printed: 8.530, fixed: 7.618,
        basis: "印 8.530 随错位 x=0.967；x=0.633 时 s=m(π/2+2x·tan20)=7.618（同行 sB 已自洽）" },
    NfOcrFix { page: 21, m: 3.75, a: 140.0, field: NfField::X, printed: 0.633, fixed: 0.967,
        basis: "印 0.633（与 A=130 行局部互换）；A 方程与 sB=9.854 反推 0.967" },
    NfOcrFix { page: 21, m: 3.75, a: 140.0, field: NfField::S, printed: 7.618, fixed: 8.530,
        basis: "印 7.618 随错位 x=0.633；x=0.967 时 s=8.530" },
    NfOcrFix { page: 22, m: 5.00, a: 110.0, field: NfField::PitchD, printed: 0.0, fixed: 100.0,
        basis: "印 00 丢首位 1；m·z=100，dB=93.969 佐证" },
    NfOcrFix { page: 22, m: 5.00, a: 110.0, field: NfField::SB, printed: 1.517, fixed: 11.517,
        basis: "印 1.517 丢首位 1；公式值 sB=m·cos20°·(π/2+z·inv20+2x·tan20)=11.517" },
    NfOcrFix { page: 22, m: 5.00, a: 120.0, field: NfField::SB, printed: 1.657, fixed: 11.657,
        basis: "印 1.657 丢首位 1；公式值 sB=11.657" },
    NfOcrFix { page: 22, m: 7.50, a: 240.0, field: NfField::X, printed: 0.633, fixed: 0.8,
        basis: "印 0.633（与 A=260 行局部互换）；A 方程与同行 s=16.149/sB=18.326 反推 0.8" },
    NfOcrFix { page: 22, m: 7.50, a: 260.0, field: NfField::X, printed: 0.800, fixed: 0.633,
        basis: "印 0.800（与 A=240 行局部互换）；A 方程与同行 s=15.237/sB=17.784 反推 0.633" },
    NfOcrFix { page: 22, m: 10.00, a: 160.0, field: NfField::PitchD, printed: 40.0, fixed: 140.0,
        basis: "印 40 丢首位 1；m·z=140，dB=131.557 佐证" },
    NfOcrFix { page: 22, m: 10.00, a: 160.0, field: NfField::RoundRoot, printed: 83.06, fixed: 133.06,
        basis: "印 83.060 丢首位 1；A−2.694m=133.060" },
];

/// **9 行「疑原表印误」**（`page, m, A, z, 列, 说明`）：数据不改，只在注释/notes 标注。
///
/// 这些行的异常无法用“丢首位/跨格粘连/相邻互换”解释（同表其余行该列均符合公式），
/// 且 p20 m=0.50 A=10 的 dB 字形已核对为印刷的 6（非 8 的误读），故按疑原表/译本印误处理。
pub const NF_SUSPECTED_SOURCE_ERRORS: &[(u16, f64, f64, u32, &str, &str)] = &[
    (18, 3.75, 130.0, 33, "D(内花键小径)", "印 132.5；A−2m=122.5，恰为 A=140 行的值，疑该列局部错行"),
    (18, 3.75, 140.0, 35, "D(内花键小径)", "印 142.5；A−2m=132.5，与上行同型错位"),
    (20, 0.50, 10.0, 18, "dB(基圆)", "印 6.457；字形核对为 6（非 8 误读）但 d·cos20°=8.457，疑原表/译本错误"),
    (21, 2.50, 20.0, 6, "平齿根齿根圆", "印 15；A−2.4m=14，同表其余行均符合公式"),
    (21, 2.50, 20.0, 6, "sB(基圆弧齿厚)", "印 5.628；公式值 5.268"),
    (22, 7.50, 120.0, 14, "dB(基圆)", "印 96.668；d·cos20°=98.668，同行其余列自洽"),
    (22, 10.00, 160.0, 14, "sB(基圆弧齿厚)", "印 22.94；公式值 22.194，疑错位/印刷错误"),
    (22, 10.00, 190.0, 17, "平齿根齿根圆", "印 176；A−2.4m=166；与 A=200 行连续 +10，疑原表印刷错位"),
    (22, 10.00, 200.0, 18, "平齿根齿根圆", "印 186；A−2.4m=176；与 A=190 行连续 +10，疑原表印刷错位"),
];

static NF_E22141_TABLE: OnceLock<Vec<NfE22141Row>> = OnceLock::new();

/// 入库尺寸表（懒加载；解析不了会 panic —— 数据随二进制编译，属构建错误）。
pub fn nf_e22141_rows() -> &'static [NfE22141Row] {
    NF_E22141_TABLE
        .get_or_init(|| parse_nf_csv(NF_E22141_CSV).unwrap_or_else(|e| panic!("NF E22-141 尺寸表入库数据损坏：{e}")))
}

/// 已入库模数档（升序；0.50…10.00 共 10 档）。
pub fn nf_e22141_modules() -> Vec<f64> {
    let mut v: Vec<f64> = nf_e22141_rows().iter().map(|r| r.m).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v.dedup_by(|a, b| (*a - *b).abs() < NF_M_TOL);
    v
}

/// 体系支持的模数/径节候选（来源固定；GUI 下拉与报错文案共用）。
///
/// * GB：`GB/T 3478.1-2008 表 2` 的 15 种（第 1 系列优先 + 第 2 系列）；
/// * DIN：`assets/din5480_2_nominal.csv` 实际出现的 `m` 列（16 档）；
/// * NF：`assets/nf_e22141_dims.csv` 实际出现的 `m` 列（10 档，含 1.667/3.75/7.5）；
/// * ANSI：径节 `P` 17 项（`assets/ansi_b921_formulas.csv`；单位每英寸齿数，不是模数）。
pub fn module_candidates(std: SplineStd) -> Vec<f64> {
    match std {
        SplineStd::GB => GB_MODULES.to_vec(),
        SplineStd::DIN => din5480_modules(),
        SplineStd::NF => nf_e22141_modules(),
        SplineStd::ANSI => ansi_pitches().iter().map(|r| r.p).collect(),
    }
}

/// 体系候选值的**来源说明**（报错文案要指出“来自哪张表”）。
pub fn module_source(std: SplineStd) -> &'static str {
    match std {
        SplineStd::GB => {
            crate::i18n::t_static("cmd.invol.src.module.gb")
        }
        SplineStd::DIN => crate::i18n::t_static("cmd.invol.src.module.din"),
        SplineStd::NF => crate::i18n::t_static("cmd.invol.src.module.nf"),
        SplineStd::ANSI => crate::i18n::t_static("cmd.invol.src.module.ansi"),
    }
}

/// **体系模数校验**：不在该体系候选里 → 明确报错并指出来源表与可用值。
/// ANSI 走 [`ansi_pitch_series_check`]（径节不是模数）。
pub fn module_series_check(std: SplineStd, m: f64) -> Result<(), String> {
    if std == SplineStd::ANSI {
        return ansi_pitch_series_check(m);
    }
    if !(m.is_finite() && m > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.err.series_m_positive", &[("v0", &std.code()), ("v1", &trim(m))]));
    }
    let list = module_candidates(std);
    if list
        .iter()
        .any(|v| (v - m).abs() <= 1e-9 * v.abs().max(1.0))
    {
        return Ok(());
    }
    Err(crate::i18n::t_fmt("cmd.invol.err.series_m_not_in_list", &[("v0", &std.code()), ("v1", &trim(m)), ("v2", &module_source(std)), ("v3", &list.iter().map(|v| trim(*v)).collect::<Vec<_>>().join(&crate::i18n::t("cmd.invol.sep.list")))]))
}

/// **花键引擎目录数据**（JSON）：预设表、DIN/NF 名义表、DIN 检验表、各体系模数/径节候选。
///
/// 齿轮生成器（`gear_gui.html`）与轴生成器（`shaft_gui.html`）共用；由 `partgen::catalog_json`
/// 以顶层 `spline_engine` 字段下发。渐开线花键的 XL 结构要素入口已移除，本数据不再挂在
/// `families` 下（避免再次出现第二个“生成入口”）。
pub fn catalog_payload() -> serde_json::Value {
    let presets: Vec<serde_json::Value> =
        [SplineStd::GB, SplineStd::DIN, SplineStd::NF, SplineStd::ANSI]
            .iter()
            .flat_map(|std| {
                std.presets().iter().map(move |p| {
                    serde_json::json!({
                        "code": preset_code(*std, p.profile).unwrap_or(""),
                        "std": std.label(),
                        "profile": p.profile,
                        "alpha": p.alpha_deg,
                        "ha": p.ha_star,
                        "hf": p.hf_star,
                        "rho": p.rho_star,
                        "cf": p.c_f_star,
                    })
                })
            })
            .collect();
    let din_nominal: Vec<serde_json::Value> = din5480_rows()
        .iter()
        .map(|r| {
            serde_json::json!({
                "db": r.d_b,
                "m": r.m,
                "z": r.z,
                "x": r.x,
                "page": r.page,
            })
        })
        .collect();
    // NF 名义表**整表下发（288 行）**：包含 p18 与 p20/21/22 的 (m,A,N) 重复变体。
    // 不在载荷层去重有两层原因：
    //   ① GUI 的候选列表按「当前 m」过滤后再用 Set 去重显示，重复行无害；
    //   ② 去重若保留先出现的 p18 行（无 x 列，JSON 里 `x:null`），会盖掉 p20+ 的 x 表值，
    //      前端 `deriveBenchX` 的 `Number(null)===0` 会让只读 x 显示 0（已有护栏在测）。
    // 引擎查表 `lookup_by_a()` 自己按 (m,A,N) 去重并优先留信息全的行（既有行为）。
    let nf_nominal: Vec<serde_json::Value> = nf_e22141_rows()
        .iter()
        .map(|r| {
            serde_json::json!({
                "a": r.a,
                "m": r.m,
                "z": r.z,
                "x": r.x,
                "page": r.page,
                "source": r.source,
                "table": r.table_no,
                "fixes": r.fixes,
            })
        })
        .collect();
    let din_inspection: Vec<serde_json::Value> = inspection_rows()
        .iter()
        .map(|r| {
            serde_json::json!({
                "db": r.d_b,
                "m": r.m,
                "z": r.z,
                "dm_hub": r.d_m_hub,
                "m2": r.m2,
                "dm_shaft": r.d_m_shaft,
                "m1": r.m1,
                "k": r.k,
                "wk": r.w_k,
                "page": r.page,
                "table": r.table_no,
                "source": r.source,
            })
        })
        .collect();
    serde_json::json!({
        // ⑥ 说明串随语言（协议/数据记号原样）；取词在 `i18n::cmd.invol.engine.*`。
        "source": crate::i18n::t_fmt("cmd.invol.engine.source", &[("v0", GB_CODE), ("v1", DIN_CODE)]),
        "din_notes": crate::i18n::t_static("cmd.invol.engine.din_notes"),
        "nf_notes": crate::i18n::t_static("cmd.invol.engine.nf_notes"),
        "ansi_notes": crate::i18n::t_static("cmd.invol.engine.ansi_notes"),
        "invol_presets": presets,
        "din_nominal": din_nominal,
        "din_inspection": din_inspection,
        "nf_nominal": nf_nominal,
        // 各体系模数/径节候选（GUI 下拉与“体系不支持的模数”报错共用）。
        "gb_modules": GB_MODULES,
        "din_modules": din5480_modules(),
        "nf_modules": nf_e22141_modules(),
        "module_sources": {
            "GB": module_source(SplineStd::GB),
            "DIN": module_source(SplineStd::DIN),
            "NF": module_source(SplineStd::NF),
            "ANSI": module_source(SplineStd::ANSI),
        },
        // ANSI 径节 17 项的 A/B 写法（gear_gui 下拉选项；Ps=2P）。
        "ansi_pitches": ansi_pitch_labels(),
    })
}

fn nf_field_get(row: &NfE22141Row, field: NfField) -> Option<f64> {
    match field {
        NfField::Z => Some(row.z as f64),
        NfField::X => row.x,
        NfField::S => row.s,
        NfField::SB => row.s_b,
        NfField::PitchD => row.d,
        NfField::BaseDia => row.base_dia,
        NfField::InternalD => row.internal_tip,
        NfField::FlatRoot => row.flat_root,
        NfField::RoundRoot => row.round_root,
        NfField::H => row.h,
    }
}

fn nf_field_set(row: &mut NfE22141Row, field: NfField, value: f64) {
    match field {
        NfField::Z => row.z = value.round() as u32,
        NfField::X => row.x = Some(value),
        NfField::S => row.s = Some(value),
        NfField::SB => row.s_b = Some(value),
        NfField::PitchD => row.d = Some(value),
        NfField::BaseDia => row.base_dia = Some(value),
        NfField::InternalD => row.internal_tip = Some(value),
        NfField::FlatRoot => row.flat_root = Some(value),
        NfField::RoundRoot => row.round_root = Some(value),
        NfField::H => row.h = Some(value),
    }
}

/// 对一行应用 [`NF_OCR_FIXES`]（key = page/m/A；仅当当前值与 `printed` 一致时应用）。
fn nf_apply_fixes(row: &mut NfE22141Row) {
    for fix in NF_OCR_FIXES {
        if row.page != fix.page
            || (row.m - fix.m).abs() > NF_M_TOL
            || (row.a - fix.a).abs() > NF_A_TOL
        {
            continue;
        }
        let Some(cur) = nf_field_get(row, fix.field) else { continue };
        if (cur - fix.printed).abs() < 1e-6 {
            nf_field_set(row, fix.field, fix.fixed);
            row.fixes.push(format!(
                "{} {}→{}",
                match fix.field {
                    NfField::Z => "z",
                    NfField::X => "x",
                    NfField::S => "s",
                    NfField::SB => "sB",
                    NfField::PitchD => "d",
                    NfField::BaseDia => "dB",
                    NfField::InternalD => "D",
                    NfField::FlatRoot => "c4(平齿根)",
                    NfField::RoundRoot => "c12(圆齿根)",
                    NfField::H => "h",
                },
                trim(fix.printed),
                trim(fix.fixed)
            ));
        }
    }
}

/// 解析 NF 入库 CSV（跳过 `#` 注释与 `page` 表头；缺列/坏值 → 带行号报错）。
fn parse_nf_csv(text: &str) -> Result<Vec<NfE22141Row>, String> {
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let f = split_csv_line(line);
        if f.first().map(|s| s.trim()) == Some("page") {
            continue; // 表头
        }
        if f.len() < 7 {
            return Err(format!("NF 第 {n} 行只有 {} 列（至少应有 page/table/source/m/A/N 7 列）", f.len()));
        }
        let f: Vec<String> = f.into_iter().chain(std::iter::repeat(String::new())).take(19).collect();
        let page = f[0]
            .trim()
            .parse::<u16>()
            .map_err(|_| format!("NF 第 {n} 行 page「{}」非法", f[0]))?;
        let m = csv_decimal(&f[3]).ok_or_else(|| format!("NF 第 {n} 行 m「{}」非法", f[3]))?;
        let a = csv_decimal(&f[4]).ok_or_else(|| format!("NF 第 {n} 行 A「{}」非法", f[4]))?;
        let z = f[5]
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("NF 第 {n} 行 N「{}」非法", f[5]))?;
        if !(m.is_finite() && m > 0.0) {
            return Err(format!("NF 第 {n} 行 m={m} 非正"));
        }
        if !(a.is_finite() && a > 0.0) {
            return Err(format!("NF 第 {n} 行 A={a} 非正"));
        }
        if z == 0 {
            return Err(format!("NF 第 {n} 行 N=0 非法"));
        }
        let opt = |idx: usize| csv_decimal(&f[idx]);
        let mut row = NfE22141Row {
            page,
            table_no: f[1].trim().to_string(),
            source: f[2].trim().to_string(),
            m,
            a,
            z,
            d: opt(9),
            base_dia: opt(10),
            x: opt(8),
            s: opt(11),
            s_b: opt(12),
            flat_root: opt(7),
            round_root: opt(15),
            h: opt(16),
            r: opt(13),
            r_i: opt(14),
            internal_tip: opt(6),
            hub_fillet: opt(17),
            flags: f[18].trim().to_string(),
            fixes: Vec::new(),
        };
        nf_apply_fixes(&mut row);
        rows.push(row);
    }
    Ok(rows)
}

/// 命中 `(A, m?)` 的原始行（不产生错误文案；已按 `(m,z)` 去重，优先保留信息全的行）。
fn nf_match(a: f64, m: Option<f64>) -> Vec<NfE22141Row> {
    let mut hits: Vec<NfE22141Row> = nf_e22141_rows()
        .iter()
        .filter(|r| (r.a - a).abs() < NF_A_TOL)
        .filter(|r| m.is_none_or(|m| (r.m - m).abs() < NF_M_TOL))
        .cloned()
        .collect();
    // 同一 `(m,z)` 在 p18（只有 D 列）与 p20/p21/p22（整行）各出现一次：保留更全的一行。
    let mut out: Vec<NfE22141Row> = Vec::new();
    for r in hits.drain(..) {
        match out.iter_mut().find(|o| {
            (o.m - r.m).abs() < NF_M_TOL && o.z == r.z
        }) {
            Some(o) => {
                if nf_row_weight(&r) > nf_row_weight(o) {
                    *o = r;
                }
            }
            None => out.push(r),
        }
    }
    out.sort_by(|x, y| x.z.cmp(&y.z).then(x.m.partial_cmp(&y.m).unwrap()));
    out
}

/// 行信息量（非空列数）：用于 p18/整行重复行去重。
fn nf_row_weight(r: &NfE22141Row) -> usize {
    [
        r.d.is_some(),
        r.base_dia.is_some(),
        r.x.is_some(),
        r.s.is_some(),
        r.s_b.is_some(),
        r.flat_root.is_some(),
        r.round_root.is_some(),
    ]
    .iter()
    .filter(|v| **v)
    .count()
}

fn nf_join_rows(rows: &[NfE22141Row]) -> String {
    rows.iter()
        .map(|r| format!("A={} m={} N={} x={}", trim(r.a), trim(r.m), r.z, r.x.map(trim).unwrap_or_default()))
        .collect::<Vec<_>>()
        .join(&crate::i18n::t("cmd.invol.sep.list"))
}

/// **NF E22-141 尺寸表查表**：按公称直径 `A`（可再限定模数 `m`）返回名义行。
///
/// * 命中可能多行 —— 同一 `A` 的多个 `m`（不给 m 时）或同一 `(m,A)` 的多个 N 变体；
/// * 找不到时列出附近候选（`A=…（m=…）`）；
/// * p18 的拉削内花键行只有 `D=A−2m`，与 p20/p21/p22 同 `(m,A,N)` 行合并时保留信息全的后者。
pub fn lookup_by_a(a: f64, m: Option<f64>) -> Result<Vec<NfE22141Row>, String> {
    if !(a.is_finite() && a > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.nf.lookup_a_positive", &[("v0", &trim(a))]));
    }
    if let Some(m) = m {
        if !(m.is_finite() && m > 0.0) {
            return Err(crate::i18n::t_fmt("cmd.invol.nf.lookup_m_positive", &[("v0", &trim(m))]));
        }
    }
    let hits = nf_match(a, m);
    if hits.is_empty() {
        Err(nf_miss_message(a, m))
    } else {
        Ok(hits)
    }
}

/// 未命中的错误文案：已入库档位 + 附近候选。
fn nf_miss_message(a: f64, m: Option<f64>) -> String {
    let mut msg = crate::i18n::t_fmt("cmd.invol.nf.lookup_head", &[("v0", &trim(a))]);
    if let Some(m) = m {
        msg.push_str(&crate::i18n::t_fmt("cmd.invol.din.lookup_head_m", &[("v0", &trim(m))]));
    }
    msg.push_str(&crate::i18n::t("cmd.invol.din.lookup_no_hit"));
    if let Some(m) = m {
        if !nf_e22141_modules().iter().any(|mm| (mm - m).abs() < NF_M_TOL) {
            let list = nf_e22141_modules().iter().map(|v| trim(*v)).collect::<Vec<_>>().join(&crate::i18n::t("cmd.invol.sep.list"));
            msg.push_str(&crate::i18n::t_fmt("cmd.invol.table.no_module_step", &[("v0", &trim(m)), ("v1", &list)]));
        } else {
            msg.push('。');
        }
    } else {
        msg.push('。');
    }
    // 附近候选：同 m（若给了 m）按 |ΔA| 取最近的 5 个不同 A；无同 m 行则退全表。
    let mut pool: Vec<&NfE22141Row> = nf_e22141_rows()
        .iter()
        .filter(|r| m.is_none_or(|m| (r.m - m).abs() < NF_M_TOL))
        .collect();
    if pool.is_empty() {
        pool = nf_e22141_rows().iter().collect();
    }
    pool.sort_by(|x, y| (x.a - a).abs().partial_cmp(&(y.a - a).abs()).unwrap());
    let mut seen: Vec<(i64, i64)> = Vec::new();
    let mut cands: Vec<String> = Vec::new();
    for r in pool {
        let key = ((r.m * 1000.0).round() as i64, (r.a * 1000.0).round() as i64);
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        cands.push(format!("A={}（m={}）", trim(r.a), trim(r.m)));
        if cands.len() >= 5 {
            break;
        }
    }
    if !cands.is_empty() {
        msg.push_str(&crate::i18n::t_fmt("cmd.invol.table.candidates", &[("v0", &cands.join(&crate::i18n::t("cmd.invol.sep.list")))]));
    }
    msg
}

/// NF E22-141 齿形变位系数（p07 公式）：
/// `x = (A − m(N + 0.4)) / (2m)`；逆式 `A = m(N + 2x + 0.4)`。
pub fn x_from_a(a: f64, m: f64, z: u32) -> f64 {
    (a - m * (z as f64 + 0.4)) / (2.0 * m)
}

/// 由 `m/z/x` 正算公称直径 `A = m(N + 2x + 0.4)`（[`x_from_a`] 的逆）。
pub fn a_from_x(m: f64, z: u32, x: f64) -> f64 {
    m * (z as f64 + 2.0 * x + 0.4)
}

/// 表行 → 参数（`x` 由 `A` 公式解出 —— A 是主参数；表值 x 另见 [`NfE22141Row::x`]）。
fn nf_row_to_params(row: NfE22141Row, profile: &str) -> Result<(InvolParams, D_bOrigin), String> {
    let xv = x_from_a(row.a, row.m, row.z);
    let p = InvolParams::from_preset(SplineStd::NF, profile, row.m, row.z)
        .map_err(|e| crate::i18n::t_fmt("cmd.invol.nf.row_tag", &[("v0", &row.page.to_string()), ("v1", &trim(row.m)), ("v2", &row.z.to_string()), ("v3", &e)]))?
        .with_x(xv)
        .with_a(row.a);
    p.validate().map_err(|e| crate::i18n::t_fmt("cmd.invol.err.nf_prefix", &[("e", &e)]))?;
    Ok((p, D_bOrigin::NfTable(row)))
}

/// 由 `A + m` 反求齿数 N：`A=m(N+2x+0.4)`、主系列 `x=0.8`
/// → `N = A/m − 2.0`；表外取最接近的整数（标注推导值）。返回 `(N, 区间下界, 区间上界)`。
fn derive_nf_z_from_a_m(a: f64, m: f64, prefer: Option<u32>, x: Option<f64>) -> Result<(u32, f64, f64), String> {
    if !(m.is_finite() && m > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.nf.m_positive", &[("v0", &trim(m))]));
    }
    let x0 = x.unwrap_or(0.8);
    let z_lo_f = (a - m * (0.4 + 2.0 * 0.967)) / m - 1e-9;
    let z_hi_f = (a - m * (0.4 + 2.0 * 0.6)) / m + 1e-9;
    let nominal = (a / m - 0.4 - 2.0 * x0).round();
    let z = match prefer {
        Some(pz) => pz as f64,
        None => nominal,
    };
    if !(1.0..=1000.0).contains(&z) {
        return Err(crate::i18n::t_fmt("cmd.invol.nf.z_out_of_range", &[("v0", &trim(a)), ("v1", &trim(m)), ("v2", &trim(x0)), ("v3", &trim(z))]));
    }
    Ok((z as u32, z_lo_f, z_hi_f))
}

/// **由基准直径（DIN `d_B` / NF `A`）与模数 `m` 推导齿数 `z`**（DIN/NF 联动锁定齿数的唯一入口）。
///
/// * 表内直查优先：命中 `(d_B/A, m)` 的 `z` 唯一 → 用它；同一 `(d_B/A, m)` 多个 `z` 变体
///   （表格事实）→ 取 |x| 最小者（DIN；NF 取最接近表内主系列 x=0.8 者）；
/// * 表外按公式区间取整：DIN `d_B=m(z+1.1+2x)`、`x∈[−0.05,0.45]`；NF `A=m(N+2x+0.4)`、主系列 x=0.8；
/// * 返回 `(z, 来源说明)`；GB/ANSI 无此概念 → 报错。
///
/// 口径与图形界面一致：GUI 把这个 `z` 回填到只读齿数框，命令行显式给 `z` 时由
/// [`resolve_din_by_d_b`]/[`resolve_nf_by_a`] 与推导值比对，不符则以基准直径为准并明文提示。
pub fn derive_z_from_bench_m(std: SplineStd, bench: f64, m: f64) -> Result<(u32, String), String> {
    if !(bench.is_finite() && bench > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.nf.bench_positive", &[("v0", &std.code()), ("v1", &trim(bench))]));
    }
    if !(m.is_finite() && m > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.err.series_m_positive", &[("v0", &std.code()), ("v1", &trim(m))]));
    }
    match std {
        SplineStd::DIN => {
            let mut zs: Vec<u32> = din5480_match(bench, Some(m)).iter().map(|r| r.z).collect();
            zs.sort_unstable();
            zs.dedup();
            if zs.is_empty() {
                let (z, z_lo, z_hi) = derive_z_from_d_b_m(bench, m, None)?;
                return Ok((z, crate::i18n::t_fmt("cmd.invol.nf.derive_z_table_note", &[("v0", &trim(bench)), ("v1", &trim(m)), ("v2", &format!("{:.4}", z_lo)), ("v3", &format!("{:.4}", z_hi)), ("v4", &z.to_string()), ("v5", &trim(x_from_d_b(bench, m, z)))])));
            }
            let z = zs
                .iter()
                .copied()
                .min_by(|&a, &b| {
                    x_from_d_b(bench, m, a)
                        .abs()
                        .partial_cmp(&x_from_d_b(bench, m, b).abs())
                        .unwrap()
                        .then(a.cmp(&b))
                })
                .expect("zs 非空");
            let note = if zs.len() == 1 {
                crate::i18n::t_fmt("cmd.invol.din.bench_hit_z", &[("v0", &trim(bench)), ("v1", &trim(m)), ("v2", &z.to_string())])
            } else {
                let variants = zs
                    .iter()
                    .map(|v| crate::i18n::t_fmt("cmd.invol.din.z_with_x", &[("v0", &v.to_string()), ("v1", &trim(x_from_d_b(bench, m, *v)))]))
                    .collect::<Vec<_>>()
                    .join(&crate::i18n::t("cmd.invol.sep.list"));
                crate::i18n::t_fmt("cmd.invol.din.bench_multi_z", &[("v0", &trim(bench)), ("v1", &trim(m)), ("v2", &variants), ("v3", &z.to_string())])
            };
            Ok((z, note))
        }
        SplineStd::NF => {
            let mut zs: Vec<u32> = nf_match(bench, Some(m)).iter().map(|r| r.z).collect();
            zs.sort_unstable();
            zs.dedup();
            if zs.is_empty() {
                let (z, z_lo, z_hi) = derive_nf_z_from_a_m(bench, m, None, None)?;
                return Ok((z, format!(
                    "表外推导：A={}、m={} 不在 NF E22-141 尺寸表；按主系列 x=0.8 取 N={}（表内 x∈[0.6,0.967] 时 N∈[{:.4},{:.4}]）",
                    trim(bench), trim(m), z, z_lo, z_hi
                )));
            }
            let z = zs
                .iter()
                .copied()
                .min_by(|&a, &b| {
                    (x_from_a(bench, m, a) - 0.8)
                        .abs()
                        .partial_cmp(&(x_from_a(bench, m, b) - 0.8).abs())
                        .unwrap()
                        .then(a.cmp(&b))
                })
                .expect("zs 非空");
            let note = if zs.len() == 1 {
                crate::i18n::t_fmt("cmd.invol.nf.bench_hit_n", &[("v0", &trim(bench)), ("v1", &trim(m)), ("v2", &z.to_string())])
            } else {
                let variants = zs
                    .iter()
                    .map(|v| crate::i18n::t_fmt("cmd.invol.nf.rows_join", &[("v0", &v.to_string()), ("v1", &trim(x_from_a(bench, m, *v)))]))
                    .collect::<Vec<_>>()
                    .join(&crate::i18n::t("cmd.invol.sep.list"));
                crate::i18n::t_fmt("cmd.invol.nf.note_multi_variant", &[("v0", &trim(bench)), ("v1", &trim(m)), ("v2", &variants), ("v3", &z.to_string())])
            };
            Ok((z, note))
        }
        _ => Err(crate::i18n::t_fmt("cmd.invol.derive_z_not_applicable", &[("v0", &std.label())])),
    }
}

/// 用 `A` 补全/校验 NF 参数（`m/N/x` 缺哪项补哪项；主路径入口）。
///
/// **`A` 是主参数**（NF E22-141 p07：`A = m(N+2x+0.4)`）：
/// * `A + m + N`：`x=(A−m(N+0.4))/(2m)`；查表命中取表值 x（与公式不符时明文提示后按 A 取表值/公式）；
/// * `A + m`：查表补 N（同 A 多个 N 变体时列候选）；表外按主系列 x=0.8 推 N 并标注来源；
/// * `A + N`：查表补 m；表外按 x=0.8 给名义 `m = A/(N+2)` 并标注来源；
/// * 只给 `A`：查表补 `m/N`（多命中列候选）。
pub fn resolve_nf_by_a(
    a: f64,
    m: Option<f64>,
    z: Option<u32>,
    x: Option<f64>,
    profile: &str,
) -> Result<(InvolParams, D_bOrigin), String> {
    if !(a.is_finite() && a > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.nf.a_positive", &[("v0", &trim(a))]));
    }
    if let Some(m) = m {
        if !(m.is_finite() && m > 0.0) {
            return Err(crate::i18n::t_fmt("cmd.invol.nf.m_positive", &[("v0", &trim(m))]));
        }
        // 模数必须来自 NF E22-141 尺寸表的实际 m 列（与 GB/DIN 系列不同）。
        module_series_check(SplineStd::NF, m)?;
    }
    match (m, z) {
        (Some(m), Some(z)) => {
            // A 为主参数：N 由 (A, m) 推导；输入 N 不符且不是表内变体 → 以 A 为准重算并明文提示。
            let (z_canon, _) = derive_z_from_bench_m(SplineStd::NF, a, m)?;
            let row = nf_match(a, Some(m)).into_iter().find(|r| r.z == z);
            if z != z_canon && row.is_none() {
                let x2 = x_from_a(a, m, z_canon);
                let mut note = crate::i18n::t_fmt("cmd.invol.nf.keep_n_from_a", &[("v0", &trim(a)), ("v1", &z_canon.to_string()), ("v2", &z.to_string())]);
                if let Some(xi) = x {
                    if (xi - x2).abs() > NF_X_TOL {
                        note.push_str(&crate::i18n::t_fmt("cmd.invol.nf.also_x", &[("v0", &trim(x2)), ("v1", &trim(xi))]));
                    }
                }
                let p = InvolParams::from_preset(SplineStd::NF, profile, m, z_canon)
                    .map_err(|e| crate::i18n::t_fmt("cmd.invol.err.nf_prefix", &[("e", &e)]))?
                    .with_x(x2)
                    .with_a(a);
                p.validate().map_err(|e| crate::i18n::t_fmt("cmd.invol.err.nf_prefix", &[("e", &e)]))?;
                return Ok((p, D_bOrigin::Adjusted(note)));
            }
            let x_formula = x_from_a(a, m, z);
            let (xv, origin) = match row {
                Some(row) => {
                    let with_fixes = if row.fixes.is_empty() {
                        String::new()
                    } else {
                        crate::i18n::t_fmt("cmd.invol.ocr_fix_note", &[("v0", &row.fixes.join(&crate::i18n::t("cmd.invol.sep.list")))])
                    };
                    let origin = match x {
                        Some(xi) if (xi - x_formula).abs() > NF_X_TOL => D_bOrigin::Adjusted(crate::i18n::t_fmt("cmd.invol.nf.table_hit_x", &[("v0", &row.page.to_string()), ("v1", &trim(row.m)), ("v2", &trim(a)), ("v3", &with_fixes), ("v4", &trim(x_formula)), ("v5", &trim(xi))])),
                        _ if row.x.is_some_and(|v| (v - x_formula).abs() > NF_X_MATCH_TOL) => {
                            D_bOrigin::Adjusted(crate::i18n::t_fmt("cmd.invol.nf.table_x_mismatch", &[("v0", &row.page.to_string()), ("v1", &trim(row.m)), ("v2", &trim(a)), ("v3", &with_fixes), ("v4", &row.x.map(trim).unwrap_or_default()), ("v5", &trim(x_formula))]))
                        }
                        _ => D_bOrigin::NfTable(row),
                    };
                    (x_formula, origin)
                }
                None => {
                    let origin = match x {
                        Some(xi) if (xi - x_formula).abs() > NF_X_TOL => D_bOrigin::Adjusted(crate::i18n::t_fmt("cmd.invol.nf.keep_x_from_a", &[("v0", &trim(a)), ("v1", &trim(x_formula)), ("v2", &trim(xi))])),
                        _ => D_bOrigin::Formula,
                    };
                    (x_formula, origin)
                }
            };
            let p = InvolParams::from_preset(SplineStd::NF, profile, m, z)
                .map_err(|e| crate::i18n::t_fmt("cmd.invol.err.nf_prefix", &[("e", &e)]))?
                .with_x(xv)
                .with_a(a);
            p.validate().map_err(|e| crate::i18n::t_fmt("cmd.invol.err.nf_prefix", &[("e", &e)]))?;
            Ok((p, origin))
        }
        (Some(m), None) => {
            let hits = nf_match(a, Some(m));
            let mut zs: Vec<u32> = hits.iter().map(|r| r.z).collect();
            zs.sort_unstable();
            zs.dedup();
            if zs.len() > 1 {
                return Err(crate::i18n::t_fmt("cmd.invol.nf.a_m_multi_z", &[("v0", &trim(a)), ("v1", &trim(m)), ("v2", &nf_join_rows(&hits))]));
            }
            if zs.len() == 1 {
                let row = hits.into_iter().next().expect("zs 非空");
                return nf_row_to_params(row, profile);
            }
            // 表外：主系列 x=0.8 → A=m(N+2)。
            let (z, z_lo, z_hi) = derive_nf_z_from_a_m(a, m, None, x)?;
            let xv = x.unwrap_or(0.8);
            let note = crate::i18n::t_fmt("cmd.invol.nf.derive_z_note", &[("v0", &trim(a)), ("v1", &trim(m)), ("v2", &trim(xv)), ("v3", &z.to_string()), ("v4", &format!("{:.4}", z_lo)), ("v5", &format!("{:.4}", z_hi))]);
            let p = InvolParams::from_preset(SplineStd::NF, profile, m, z)
                .map_err(|e| crate::i18n::t_fmt("cmd.invol.err.nf_prefix", &[("e", &e)]))?
                .with_x(xv)
                .with_a(a);
            p.validate().map_err(|e| crate::i18n::t_fmt("cmd.invol.err.nf_prefix", &[("e", &e)]))?;
            Ok((p, D_bOrigin::Derived(note)))
        }
        (None, Some(z)) => {
            let hits: Vec<NfE22141Row> = nf_match(a, None).into_iter().filter(|r| r.z == z).collect();
            if !hits.is_empty() {
                let mut ms: Vec<f64> = hits.iter().map(|r| r.m).collect();
                ms.sort_by(|x, y| x.partial_cmp(y).unwrap());
                ms.dedup_by(|x, y| (*x - *y).abs() < NF_M_TOL);
                if ms.len() > 1 {
                    return Err(crate::i18n::t_fmt("cmd.invol.nf.a_z_multi_m", &[("v0", &trim(a)), ("v1", &z.to_string()), ("v2", &nf_join_rows(&hits))]));
                }
                return nf_row_to_params(hits[0].clone(), profile);
            }
            // 表外：主系列 x=0.8 → m = A/(N+2)。
            let xv = x.unwrap_or(0.8);
            let m = a / (z as f64 + 2.0 * xv + 0.4);
            let note = crate::i18n::t_fmt("cmd.invol.nf.derive_m_note", &[("v0", &trim(a)), ("v1", &z.to_string()), ("v2", &trim(xv)), ("v3", &trim(m))]);
            let p = InvolParams::from_preset(SplineStd::NF, profile, m, z)
                .map_err(|e| crate::i18n::t_fmt("cmd.invol.err.nf_prefix", &[("e", &e)]))?
                .with_x(xv)
                .with_a(a);
            p.validate().map_err(|e| crate::i18n::t_fmt("cmd.invol.err.nf_prefix", &[("e", &e)]))?;
            Ok((p, D_bOrigin::Derived(note)))
        }
        (None, None) => {
            let hits = nf_match(a, None);
            if hits.is_empty() {
                return Err(nf_miss_message(a, None));
            }
            if hits.len() > 1 {
                return Err(crate::i18n::t_fmt("cmd.invol.nf.a_multi_hit", &[("v0", &trim(a)), ("v1", &hits.len().to_string()), ("v2", &nf_join_rows(&hits))]));
            }
            nf_row_to_params(hits.into_iter().next().expect("非空"), profile)
        }
    }
}

// ──────────────── NF E22-141 检查尺寸/公差/配合表（assets/nf_e22141_check.csv） ────────────────

/// 入库检查表（`crates/ocs_ocsm/assets/nf_e22141_check.csv`，**399 行 / 10 张表**）：
/// p23 检查尺寸(m=0.50~1.25) 39 + p24 检查尺寸(m=1.667~3.75) 49 +
/// p25 检查尺寸(m=5.00~10.00) 56 + p26 计算标准-设计尺寸(m=1.00) 48 +
/// p27 计算标准-检查尺寸(m=1.00) 49 + p29 公差值-xm的变动值(微米) 14 +
/// p31 配合表(m=0.50~1.25) 39 + p32 检查尺寸的公差值(m=1.667~2.50) 28 +
/// p33 检查尺寸的公差值(m=3.75~5.00) 45 + p35 F,F1,G,G1的偏差值(微米) 32。
///
/// 来源：`~/桌面/OCSM/review/花键标准资料/NF-E22-141_检查公差表.csv`（页清单见同名
/// `_页清单.md`）。`page/table_no/source/section/flags` 原样保留；**可疑格只标不改**
/// （未读行与反推值都在 `flags`），仅 2 处错格进 [`NF_CHECK_OCR_FIXES`]
/// （1 处主尺寸表同源粘连，1 处看图裁决末位 1 被吃掉）
/// —— 与主尺寸表 [`NF_OCR_FIXES`] 同一惯例（CSV 不动，修正只在解析期生效）。
const NF_E22141_CHECK_CSV: &str = include_str!("../assets/nf_e22141_check.csv");

/// NF 检查表一行（45 列；各表共用列位，空列 = `None`/空串）。
#[derive(Debug, Clone, PartialEq)]
pub struct NfCheckRow {
    /// 源图页码（23/24/25/26/27/29/31/32/33/35）。
    pub page: u16,
    /// 页内表名（如 `检查尺寸(m=0.50~1.25)`、`配合表(m=0.50~1.25)`）。
    pub table_no: String,
    /// OCR 来源（`p23`…`p35`）。
    pub source: String,
    /// 页内子表/分区（如 `m=1.00|m=1.25:L(m=1.0)`、`检查尺寸`）。
    pub section: String,
    /// 有效模数：优先 `m` 列；`m` 列为空时取 `section` 末尾 `(m=X)`（p31–p35 偏差表）。
    pub m: Option<f64>,
    /// `m` 是否来自 `section`（偏差表左右子表标注）。
    pub m_from_section: bool,
    /// 原始 `A` 格（可为 `4~15` 范围 / `110,120,…` 列表 —— p29；这些行 `a` 为 `None`）。
    pub a_raw: String,
    /// 数值 `A`（范围/列表格为 `None`，匹配走 [`nf_check_a_matches`]）。
    pub a: Option<f64>,
    /// 齿数 N（偏差表有；p29 为空）。
    pub n: Option<u32>,
    /// p23–p25/p27 检查尺寸列：跨齿数 K。
    pub k: Option<f64>,
    /// K 齿公法线 E。
    pub e: Option<f64>,
    /// 外花键量棒直径 U。
    pub u: Option<f64>,
    /// 外花键跨量棒距 F。
    pub f: Option<f64>,
    /// 外花键跨量棒距 F1。
    pub f1: Option<f64>,
    /// 内花键量棒直径 V。
    pub v: Option<f64>,
    /// 内花键量棒削边尺寸 V1。
    pub v1: Option<f64>,
    /// 内花键量棒跨距 G。
    pub g: Option<f64>,
    /// 内花键量棒跨距 G1。
    pub g1: Option<f64>,
    /// p26 计算标准-设计尺寸列：齿根圆（平齿根）B。
    pub b: Option<f64>,
    /// 变位系数 x。
    pub x: Option<f64>,
    /// 分度圆直径 d。
    pub d: Option<f64>,
    /// 基圆直径 dB。
    pub db: Option<f64>,
    /// 分度圆弧齿厚 s。
    pub s: Option<f64>,
    /// 基圆弧齿厚 sB。
    pub sb: Option<f64>,
    /// 齿根圆角半径 Rf（平齿根）。
    pub rf: Option<f64>,
    /// 齿根圆角半径 Rr（圆齿根）。
    pub rr: Option<f64>,
    /// 齿根圆（圆齿根）B2。
    pub b2: Option<f64>,
    /// 齿顶倒角高度 h。
    pub h: Option<f64>,
    /// 槽底圆角半径 ri。
    pub ri: Option<f64>,
    /// p27 右侧 7 列（表头未辨认，位置命名 q1..q7）。
    pub q: [Option<f64>; 7],
    /// p29/p31–p35 偏差列（每格 `+上偏差/下偏差`，微米）。
    pub dev: [String; 10],
    /// OCR 质量标记（原样保留；可疑格 + 反推值都在这里）。
    pub flags: String,
    /// 已应用的 OCR 修正（人类可读，如 `N 192→19`；空 = 未修）。
    pub fixes: Vec<String>,
}

/// 一条 NF 检查表 OCR 修正：`key = (page, m, A)`；CSV 原值保留，解析时按 key 修正。
#[derive(Debug, Clone, Copy)]
pub struct NfCheckFix {
    /// 源图页码。
    pub page: u16,
    /// 模数 m。
    pub m: f64,
    /// 公称直径 A。
    pub a: f64,
    /// OCR 读到的印刷 N（用于核对确实命中原行原格）。
    pub printed_n: u32,
    /// 修正后的 N。
    pub fixed_n: u32,
    /// 判定依据（表内自洽关系 / 主尺寸表同一错格）。
    pub basis: &'static str,
}

/// **NF E22-141 检查表已知 OCR 错格修正表**（不篡改入库 CSV；逐条注明依据）。
pub const NF_CHECK_OCR_FIXES: &[NfCheckFix] = &[
    NfCheckFix {
        page: 25,
        m: 5.0,
        a: 105.0,
        printed_n: 192,
        fixed_n: 19,
        basis: "192=19 与邻格尾数 2 粘连；同行 E=40.968 按 N=19 公式自洽；主尺寸表 p22 同一错格已在 NF_OCR_FIXES 修",
    },
    NfCheckFix {
        page: 24,
        m: 1.667,
        a: 55.0,
        printed_n: 3,
        fixed_n: 31,
        basis: "看图裁决：印刷 N=31，末位 1 被 OCR 吃掉；同页 m=2.50 A=55 的 N=20 是另一行，不合并",
    },
];

impl NfCheckRow {
    /// 给用户看的来源说明（页码 + 表名 + source + 页内子表）。
    pub fn source_note(&self) -> String {
        crate::i18n::t_fmt("cmd.invol.check.src_nf_row", &[("v0", &self.page.to_string()), ("v1", &self.table_no.to_string()), ("v2", &self.source), ("v3", &self.section)])
    }

    /// 是否为检查尺寸行（p23–p25/p27：有 K/E/U/F/V/G 任一列）。
    pub fn has_check_dims(&self) -> bool {
        self.k.is_some()
            || self.e.is_some()
            || self.u.is_some()
            || self.f.is_some()
            || self.f1.is_some()
            || self.v.is_some()
            || self.v1.is_some()
            || self.g.is_some()
            || self.g1.is_some()
    }

    /// 是否为偏差行（p29/p31–p35：有 dev1..dev10 任一格）。
    pub fn has_deviations(&self) -> bool {
        self.dev.iter().any(|s| !s.is_empty())
    }
}

/// 从 `section`（如 `m=1.00|m=1.25:L(m=1.0)`）末尾的 `(m=X)` 提取有效模数（p31–p35）。
fn nf_check_section_m(section: &str) -> Option<f64> {
    let start = section.rfind("(m=")? + 3;
    let rest = &section[start..];
    let end = rest.find(')')?;
    rest[..end]
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0)
}

/// 解析 NF 检查表 CSV（跳过 `#` 注释与 `page` 表头；缺列/坏值 → 带行号报错）。
fn parse_nf_check_csv(text: &str) -> Result<Vec<NfCheckRow>, String> {
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let f = split_csv_line(line);
        if f.first().map(|s| s.trim()) == Some("page") {
            continue; // 表头
        }
        if f.len() < 45 {
            return Err(format!("NF 检查表第 {n} 行只有 {} 列（应有 45 列）", f.len()));
        }
        let page = f[0]
            .trim()
            .parse::<u16>()
            .map_err(|_| format!("NF 检查表第 {n} 行 page「{}」非法", f[0]))?;
        let section = f[3].trim().to_string();
        let m_col = f[4].trim();
        let (m, m_from_section) = if m_col.is_empty() {
            (nf_check_section_m(&section), true)
        } else {
            (csv_decimal(m_col), false)
        };
        if let Some(m) = m {
            if !(m.is_finite() && m > 0.0) {
                return Err(format!("NF 检查表第 {n} 行 m={m} 非正"));
            }
        }
        // A 用普通 f64 解析（不能用 csv_decimal：`110,120` 会被误当成小数点）；
        // `4~15`/`10-35` 这类范围原样保留、查询时按区间包含匹配。
        let a_raw = f[5].trim().to_string();
        let a = a_raw.parse::<f64>().ok().filter(|v| v.is_finite() && *v > 0.0);
        let n_teeth = f[6].trim().parse::<u32>().ok();
        let opt = |idx: usize| -> Option<f64> {
            let s = f[idx].trim();
            if s.is_empty() {
                None
            } else {
                csv_decimal(s)
            }
        };
        let mut q: [Option<f64>; 7] = [None; 7];
        for (j, slot) in q.iter_mut().enumerate() {
            *slot = opt(27 + j);
        }
        let dev: [String; 10] = std::array::from_fn(|j| f[34 + j].trim().to_string());
        rows.push(NfCheckRow {
            page,
            table_no: f[1].trim().to_string(),
            source: f[2].trim().to_string(),
            section,
            m,
            m_from_section,
            a_raw,
            a,
            n: n_teeth,
            k: opt(18),
            e: opt(19),
            u: opt(20),
            f: opt(21),
            f1: opt(22),
            v: opt(23),
            v1: opt(24),
            g: opt(25),
            g1: opt(26),
            b: opt(7),
            x: opt(8),
            d: opt(9),
            db: opt(10),
            s: opt(11),
            sb: opt(12),
            rf: opt(13),
            rr: opt(14),
            b2: opt(15),
            h: opt(16),
            ri: opt(17),
            q,
            dev,
            flags: f[44].trim().to_string(),
            fixes: Vec::new(),
        });
        nf_check_apply_fixes(rows.last_mut().expect("刚 push"));
    }
    Ok(rows)
}

/// 对一行应用 [`NF_CHECK_OCR_FIXES`]（key = page/m/A；仅当当前 N 与 `printed_n` 一致时应用）。
fn nf_check_apply_fixes(row: &mut NfCheckRow) {
    for fix in NF_CHECK_OCR_FIXES {
        if row.page != fix.page {
            continue;
        }
        let (Some(m), Some(a)) = (row.m, row.a) else { continue };
        if (m - fix.m).abs() > NF_M_TOL || (a - fix.a).abs() > NF_A_TOL {
            continue;
        }
        if row.n == Some(fix.printed_n) {
            row.n = Some(fix.fixed_n);
            row.fixes.push(format!("N {}→{}", fix.printed_n, fix.fixed_n));
        }
    }
}

static NF_E22141_CHECK_TABLE: OnceLock<Vec<NfCheckRow>> = OnceLock::new();

/// 入库检查表（懒加载；解析失败 panic —— 数据随二进制编译，属构建错误）。
pub fn nf_check_rows() -> &'static [NfCheckRow] {
    NF_E22141_CHECK_TABLE.get_or_init(|| {
        parse_nf_check_csv(NF_E22141_CHECK_CSV)
            .unwrap_or_else(|e| panic!("NF E22-141 检查表入库数据损坏：{e}"))
    })
}

/// 入库检查表按 `(page, table_no)` 的分组统计（首见顺序；页清单核对/测试用）。
pub fn nf_check_tables() -> Vec<(u16, String, usize)> {
    let mut out: Vec<(u16, String, usize)> = Vec::new();
    for r in nf_check_rows() {
        match out
            .iter_mut()
            .find(|(p, t, _)| *p == r.page && *t == r.table_no)
        {
            Some((_, _, n)) => *n += 1,
            None => out.push((r.page, r.table_no.clone(), 1)),
        }
    }
    out
}

/// `4~15` / `10-35` 形式的直径范围（两端互换时按小-大返回）。
fn nf_check_range(raw: &str) -> Option<(f64, f64)> {
    if let Some((l, r)) = raw.split_once('~') {
        let lo = l.trim().parse::<f64>().ok()?;
        let hi = r.trim().parse::<f64>().ok()?;
        return Some(if lo <= hi { (lo, hi) } else { (hi, lo) });
    }
    // 首个非首位 `-` 作分隔（p29 的 `10-35`；值均为正）。
    let idx = raw.char_indices().skip(1).find(|(_, c)| *c == '-')?.0;
    let lo = raw[..idx].trim().parse::<f64>().ok()?;
    let hi = raw[idx + 1..].trim().parse::<f64>().ok()?;
    Some(if lo <= hi { (lo, hi) } else { (hi, lo) })
}

/// `A` 格是否含给定直径：数值相等（[`NF_A_TOL`]）/ 范围包含 / 逗号列表逐项相等。
fn nf_check_a_matches(row: &NfCheckRow, a: f64) -> bool {
    if let Some(av) = row.a {
        return (av - a).abs() < NF_A_TOL;
    }
    let raw = row.a_raw.trim();
    if raw.is_empty() {
        return false;
    }
    if let Some((lo, hi)) = nf_check_range(raw) {
        return a >= lo - NF_A_TOL && a <= hi + NF_A_TOL;
    }
    if raw.contains(',') {
        return raw
            .split(',')
            .filter_map(|t| t.trim().parse::<f64>().ok())
            .any(|v| (v - a).abs() < NF_A_TOL);
    }
    false
}

/// **NF E22-141 检查表按 `(A, m)` 查询**：返回该公称直径/模数档的检查尺寸、设计尺寸、
/// 公差与配合行（p23–p27/p29/p31–p35）。p29 的 `A` 是直径范围 → 按区间包含匹配；
/// 无命中返回空 `Vec`（报告侧另给覆盖率说明）。
pub fn nf_check_by_a_m(a: f64, m: f64) -> Vec<NfCheckRow> {
    nf_check_rows()
        .iter()
        .filter(|r| r.m.is_some_and(|mm| (mm - m).abs() < NF_M_TOL))
        .filter(|r| nf_check_a_matches(r, a))
        .cloned()
        .collect()
}

/// **NF E22-141 检查表按 `(A, N)` 查询**（不限定 m）：返回该直径/齿数档的检查/公差行。
/// 无命中也返回空 `Vec`（与 [`nf_check_by_a_m`] 一致，错误文案由调用方决定）。
pub fn nf_check_by_a_n(a: f64, n: u32) -> Vec<NfCheckRow> {
    nf_check_rows()
        .iter()
        .filter(|r| r.n == Some(n))
        .filter(|r| nf_check_a_matches(r, a))
        .cloned()
        .collect()
}

/// 报告小表辅助：`Some` 时追加 `| label | value |` 行，返回是否有值。
fn nf_check_push_row(md: &mut String, label: &str, v: Option<f64>) -> bool {
    match v {
        Some(v) => {
            md.push_str(&format!("| {label} | {} |\n", trim(v)));
            true
        }
        None => false,
    }
}

// ──────────────── DIN 5480-2 检验尺寸表（M₁/M₂/D_M/k/W_k；assets/din5480_2_inspection.csv） ────────────────

/// 入库检验表（`crates/ocs_ocsm/assets/din5480_2_inspection.csv`，**267 行**）：
/// 6 个模数档 0.5（p12 表 3）/ 0.75（p16 表 7）/ 0.8（p18 表 9）/ 1（p20 表 11）+
/// m=1.5（用户截图 2，`page=2/table_no=15`、`source=user_screenshot_2`）+
/// m=5（用户截图 2，`page=36/table_no=27`、`source=user_screenshot_m5_2`；页号为推断，
/// 截图题头无页码）。
///
/// 列序照抄 CSV：`D_M_1,M2_between,A_M2,D_M_2,M1_over,A_M1` —— 与 DIN 5480-2 原表的
/// 两组（量棒直径 | 跨/棒间距 | 偏差系数）一一对应：**第一组是内花键 M2、第二组是外花键 M1**
/// （p08 图 4：M2 = Dimension between 2 measuring circles / Hub；M1 = Dimension over 2
/// measuring circles / Shaft），所以 CSV 的 `D_M_1`/`D_M_2` 是“第几组”而不是“配哪个 M”。
const DIN5480_2_INSPECTION_CSV: &str = include_str!("../assets/din5480_2_inspection.csv");

/// 检验表一行（DIN 5480-2:2015-03；内/外花键各一组量棒—间距—偏差系数 + 跨测齿数/公法线）。
#[derive(Debug, Clone, PartialEq)]
pub struct InspectionRow {
    /// 源 PDF 页码（DIN 5480-2:2015-03；m=1.5 为截图语义 2）。
    pub page: u16,
    /// 页内表号（m=0.5→3、0.75→7、0.8→9、1→11；m=1.5 截图语义 15）。
    pub table_no: u32,
    /// 模数 m。
    pub m: f64,
    /// 基准直径 `d_B`（本表主键维度）。
    pub d_b: f64,
    /// 齿数 z。
    pub z: u32,
    /// 内花键量棒直径（CSV `D_M_1`；配 M2）。
    pub d_m_hub: f64,
    /// 内花键棒间距 `M2`（CSV `M2_between`）。
    pub m2: f64,
    /// 内花键偏差系数 `A*_M2`。
    pub a_m2: f64,
    /// 外花键量棒直径（CSV `D_M_2`；配 M1）。
    pub d_m_shaft: f64,
    /// 外花键跨棒距 `M1`（CSV `M1_over`）。
    pub m1: f64,
    /// 外花键偏差系数 `A*_M1`。
    pub a_m1: f64,
    /// 跨测齿数 k。
    pub k: u32,
    /// 公法线长度 `W_k`。
    pub w_k: f64,
    /// OCR 质量标记（原样保留可查，运行时不展示）。
    pub flags: String,
    /// OCR 来源（`A` / `A(续跑)` / `user_screenshot_2`）。
    pub source: String,
}

impl InspectionRow {
    /// 给用户看的来源说明（页码 + 表号 + source）。
    pub fn source_note(&self) -> String {
        crate::i18n::t_fmt("cmd.invol.check.src_row", &[("v0", &self.page.to_string()), ("v1", &self.table_no.to_string()), ("v2", &self.source)])
    }
}

/// 检验表数值容差：表值 3 位小数（半末位 5e-4）；公式解经反解迭代，取 2e-3。
const INSPECTION_TOL: f64 = 2e-3;

/// 齿顶凸出下限（从 267 行检验表反推）：跨测接触圆须离齿顶圆 `≥0.07m`（p09 式 (5) 的
/// 齿顶凸出 `a=(d_a1−d_M)/2`，把 d_M 换成对称跨测接触圆）。264 行可算 k 与此一致。
const SPAN_TIP_MARGIN_FACTOR: f64 = 0.07;

/// **m=5 检验表 k 的 OCR 修正**（`(page, table_no, d_B, z, 原读 k, 修正 k)`，17 处）。
///
/// 用户截图 2 的 `k` 格把个位与脚注/邻格粘连（如 35→3、471→4、511→5）；修正值是单个数位，
/// 与 flags 的 `row:vote_suspect:k:expected=N` 一致，并已用 `W_k` 公式逐行独立复核。
/// **入库 CSV 保留原始 OCR 值与 flags，本表仅在解析时生效**（不篡改 assets）。
pub const DIN5480_M5_INSPECTION_K_FIXES: &[(u16, u32, f64, u32, u32, u32)] = &[
    (36, 27, 50.0, 8, 22, 2),
    (36, 27, 55.0, 9, 25, 2),
    (36, 27, 58.0, 10, 22, 2),
    (36, 27, 60.0, 10, 30, 3),
    (36, 27, 62.0, 11, 27, 2),
    (36, 27, 80.0, 14, 30, 3),
    (36, 27, 85.0, 15, 35, 3),
    (36, 27, 88.0, 16, 38, 3),
    (36, 27, 90.0, 16, 471, 4),
    (36, 27, 92.0, 17, 31, 3),
    (36, 27, 95.0, 18, 35, 3),
    (36, 27, 110.0, 20, 43, 4),
    (36, 27, 140.0, 26, 511, 5),
    (36, 27, 170.0, 32, 611, 6),
    (36, 27, 200.0, 38, 75, 7),
    (36, 27, 260.0, 50, 97, 9),
    (36, 27, 320.0, 62, 113, 11),
];

/// 应用 [`DIN5480_M5_INSPECTION_K_FIXES`]（仅当 page/table/d_B/z/原 k 全命中时替换）。
fn din5480_m5_fix_k(page: u16, table_no: u32, d_b: f64, z: u32, k: u32) -> u32 {
    DIN5480_M5_INSPECTION_K_FIXES
        .iter()
        .find(|(p, t, db, zz, kk, _)| {
            *p == page
                && *t == table_no
                && (*db - d_b).abs() < DIN_D_B_TOL
                && *zz == z
                && *kk == k
        })
        .map(|(_, _, _, _, _, fixed)| *fixed)
        .unwrap_or(k)
}

static DIN5480_INSPECTION_TABLE: OnceLock<Vec<InspectionRow>> = OnceLock::new();

/// 入库检验表（懒加载；解析失败 panic —— 数据随二进制编译，属构建错误）。
pub fn inspection_rows() -> &'static [InspectionRow] {
    DIN5480_INSPECTION_TABLE.get_or_init(|| {
        parse_inspection_csv(DIN5480_2_INSPECTION_CSV)
            .unwrap_or_else(|e| panic!("DIN 5480-2 检验表入库数据损坏：{e}"))
    })
}

/// 已入库检验模数档（升序；**6 档** 0.5/0.75/0.8/1/1.5/5；其余档名义表有但检验表未并入）。
pub fn inspection_modules() -> Vec<f64> {
    let mut v: Vec<f64> = inspection_rows().iter().map(|r| r.m).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v.dedup_by(|a, b| (*a - *b).abs() < DIN_M_TOL);
    v
}

/// 解析检验表 CSV（跳过 `#` 注释与 `page` 表头；缺列/坏值 → 带行号报错）。
fn parse_inspection_csv(text: &str) -> Result<Vec<InspectionRow>, String> {
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let f = split_csv_line(line);
        if f.first().map(|s| s.trim()) == Some("page") {
            continue; // 表头
        }
        if f.len() < 15 {
            return Err(format!("检验表第 {n} 行只有 {} 列（应有 15 列）", f.len()));
        }
        let page = f[0]
            .trim()
            .parse::<u16>()
            .map_err(|_| format!("检验表第 {n} 行 page「{}」非法", f[0]))?;
        let m = csv_decimal(&f[1])
            .ok_or_else(|| format!("检验表第 {n} 行 m「{}」非法", f[1]))?;
        let table_no = f[2]
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("检验表第 {n} 行 table_no「{}」非法", f[2]))?;
        let d_b = csv_decimal(&f[3])
            .ok_or_else(|| format!("检验表第 {n} 行 d_B「{}」非法", f[3]))?;
        let z = f[4]
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("检验表第 {n} 行 z「{}」非法", f[4]))?;
        let d_m_hub = csv_decimal(&f[5])
            .ok_or_else(|| format!("检验表第 {n} 行 D_M_1「{}」非法", f[5]))?;
        let m2 = csv_decimal(&f[6])
            .ok_or_else(|| format!("检验表第 {n} 行 M2_between「{}」非法", f[6]))?;
        let a_m2 = csv_decimal(&f[7])
            .ok_or_else(|| format!("检验表第 {n} 行 A_M2「{}」非法", f[7]))?;
        let d_m_shaft = csv_decimal(&f[8])
            .ok_or_else(|| format!("检验表第 {n} 行 D_M_2「{}」非法", f[8]))?;
        let m1 = csv_decimal(&f[9])
            .ok_or_else(|| format!("检验表第 {n} 行 M1_over「{}」非法", f[9]))?;
        let a_m1 = csv_decimal(&f[10])
            .ok_or_else(|| format!("检验表第 {n} 行 A_M1「{}」非法", f[10]))?;
        let k = f[11]
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("检验表第 {n} 行 k「{}」非法", f[11]))?;
        let k = din5480_m5_fix_k(page, table_no, d_b, z, k);
        let w_k = csv_decimal(&f[12])
            .ok_or_else(|| format!("检验表第 {n} 行 W_k「{}」非法", f[12]))?;
        if m <= 0.0 || d_b <= 0.0 || d_m_hub <= 0.0 || d_m_shaft <= 0.0 {
            return Err(format!("检验表第 {n} 行 m/d_B/D_M 非正"));
        }
        rows.push(InspectionRow {
            page,
            table_no,
            m,
            d_b,
            z,
            d_m_hub,
            m2,
            a_m2,
            d_m_shaft,
            m1,
            a_m1,
            k,
            w_k,
            flags: f[13].trim().to_string(),
            source: f[14].trim().to_string(),
        });
    }
    Ok(rows)
}

/// 命中 `(d_B, m?)` 的原始检验行（不产生错误文案）。
fn inspection_match(d_b: f64, m: Option<f64>) -> Vec<InspectionRow> {
    inspection_rows()
        .iter()
        .filter(|r| (r.d_b - d_b).abs() < DIN_D_B_TOL)
        .filter(|r| m.is_none_or(|m| (r.m - m).abs() < DIN_M_TOL))
        .cloned()
        .collect()
}

/// **DIN 5480-2 检验表查表**：按基准直径 `d_B`（可再限定模数 `m`）返回检验行。
///
/// * 命中可能多行（同一 `d_B` 多个 z 变体，如 m=0.8/d_B=20 → z=24 与 z=93 残行）；
/// * 找不到时列出附近候选（带页码与 source）；
/// * 检验表没收录的档（如 m=2）明确报「检验表没有 m=X 档」并给已入库检验档位。
pub fn lookup_inspection(d_b: f64, m: Option<f64>) -> Result<Vec<InspectionRow>, String> {
    if !(d_b.is_finite() && d_b > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.check.db_pos", &[("v0", &trim(d_b))]));
    }
    if let Some(m) = m {
        if !(m.is_finite() && m > 0.0) {
            return Err(crate::i18n::t_fmt("cmd.invol.check.m_pos", &[("v0", &trim(m))]));
        }
    }
    let mut hits = inspection_match(d_b, m);
    hits.sort_by(|a, b| a.z.cmp(&b.z).then(a.m.partial_cmp(&b.m).unwrap()));
    if hits.is_empty() {
        Err(inspection_miss_message(d_b, m))
    } else {
        Ok(hits)
    }
}

/// 检验表未命中的错误文案：缺档说明 + 附近候选（带页码/source）。
fn inspection_miss_message(d_b: f64, m: Option<f64>) -> String {
    let mut msg = crate::i18n::t_fmt("cmd.invol.check.head_db", &[("v0", &trim(d_b))]);
    if let Some(m) = m {
        msg.push_str(&crate::i18n::t_fmt("cmd.invol.din.lookup_head_m", &[("v0", &trim(m))]));
    }
    msg.push_str(&crate::i18n::t("cmd.invol.din.lookup_no_hit"));
    if let Some(m) = m {
        if !inspection_modules().iter().any(|mm| (mm - m).abs() < DIN_M_TOL) {
            let list = inspection_modules()
                .iter()
                .map(|v| trim(*v))
                .collect::<Vec<_>>()
                .join(&crate::i18n::t("cmd.invol.sep.list"));
            msg.push_str(&crate::i18n::t_fmt("cmd.invol.check.no_m_step", &[("v0", &trim(m)), ("v1", &list)]));
        } else {
            msg.push('。');
        }
    } else {
        msg.push('。');
    }
    // 附近候选：同 m（若给了 m）按 |Δd_B| 取最近的 5 个不同 d_B；无同 m 行则退全表。
    let mut pool: Vec<&InspectionRow> = inspection_rows()
        .iter()
        .filter(|r| m.is_none_or(|m| (r.m - m).abs() < DIN_M_TOL))
        .collect();
    if pool.is_empty() {
        pool = inspection_rows().iter().collect();
    }
    pool.sort_by(|a, b| {
        (a.d_b - d_b)
            .abs()
            .partial_cmp(&(b.d_b - d_b).abs())
            .unwrap()
    });
    let mut seen: Vec<f64> = Vec::new();
    let mut cands: Vec<String> = Vec::new();
    for r in pool {
        if seen.iter().any(|v| (*v - r.d_b).abs() < DIN_D_B_TOL) {
            continue;
        }
        seen.push(r.d_b);
        cands.push(crate::i18n::t_fmt("cmd.invol.check.cand_row", &[("v0", &trim(r.d_b)), ("v1", &trim(r.m)), ("v2", &r.source_note())]));
        if cands.len() >= 5 {
            break;
        }
    }
    if !cands.is_empty() {
        msg.push_str(&crate::i18n::t_fmt("cmd.invol.table.candidates", &[("v0", &cands.join(&crate::i18n::t("cmd.invol.sep.list")))]));
    }
    msg
}

/// 按 `(d_B, m, z)` 精确取一行检验值；同一 `(d_B, m)` 有多个 z 变体时列出可选 z 与来源。
pub fn inspection_for(d_b: f64, m: f64, z: u32) -> Result<InspectionRow, String> {
    let rows = lookup_inspection(d_b, Some(m))?;
    if let Some(r) = rows.iter().find(|r| r.z == z) {
        return Ok(r.clone());
    }
    let list = rows
        .iter()
        .map(|r| format!("z={}（{}）", r.z, r.source_note()))
        .collect::<Vec<_>>()
        .join(&crate::i18n::t("cmd.invol.sep.list"));
    Err(crate::i18n::t_fmt("cmd.invol.check.z_variants", &[("v0", &trim(d_b)), ("v1", &trim(m)), ("v2", &rows.len().to_string()), ("v3", &z.to_string()), ("v4", &list)]))
}

/// `m` 由名义表反查的检验查询：`(d_B, z)` 唯一定位 m 后走 [`inspection_for`]。
pub fn inspection_for_d_b_z(d_b: f64, z: u32) -> Result<InspectionRow, String> {
    if !(d_b.is_finite() && d_b > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.check.db_pos", &[("v0", &trim(d_b))]));
    }
    let mut ms: Vec<f64> = din5480_rows()
        .iter()
        .filter(|r| (r.d_b - d_b).abs() < DIN_D_B_TOL && r.z == z)
        .map(|r| r.m)
        .collect();
    ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ms.dedup_by(|a, b| (*a - *b).abs() < DIN_M_TOL);
    match ms.as_slice() {
        [] => Err(crate::i18n::t_fmt("cmd.invol.check.nominal_no_row", &[("v0", &trim(d_b)), ("v1", &z.to_string())])),
        [m] => inspection_for(d_b, *m, z),
        many => Err(crate::i18n::t_fmt("cmd.invol.check.nominal_multi_m", &[("v0", &trim(d_b)), ("v1", &z.to_string()), ("v2", &many.iter().map(|m| trim(*m)).collect::<Vec<_>>().join(&crate::i18n::t("cmd.invol.sep.list")))])),
    }
}

/// `inv α = tanα − α` 的反解（bisection；α 解在 (−1.4, 1.4) 内才有值）。
fn inv_solve(v: f64) -> Option<f64> {
    if !v.is_finite() {
        return None;
    }
    let (mut lo, mut hi) = (-1.4f64, 1.4f64);
    if inv(lo) > v || inv(hi) < v {
        return None;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if inv(mid) < v {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(0.5 * (lo + hi))
}

/// **公法线长度**（表行逐行反推验证的公式，α=30°）：
/// `W_k = m·cosα·[(k−0.5)π + z·inv α] + 2·x·m·sinα`。
pub fn base_tangent_length(m: f64, z: u32, x: f64, k: u32) -> f64 {
    let a = 30f64.to_radians();
    m * a.cos()
        * ((k as f64 - 0.5) * std::f64::consts::PI + z as f64 * inv(a))
        + 2.0 * x * m * a.sin()
}

/// **跨测齿数 k**（从 267 行检验表反推）：取最大 k 使对称跨测接触圆
/// `d_M = 2·√(r_b² + (W_k/2)²)` 与齿顶圆 `d_a1` 留出 `≥ 0.07m` 齿顶凸出
/// （`a = (d_a1 − d_M)/2 ≥ 0.07m`）。表内可算的 **264 行 264/264** 一致；
/// 另 3 行为 k/z 的 OCR 残值（p18 z=35 的 k=63、z=93/97 重复残行）。
pub fn span_teeth(m: f64, z: u32, x: f64) -> Result<u32, String> {
    if !(m.is_finite() && m > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.check.m_pos_k", &[("v0", &trim(m))]));
    }
    if !(3..=1000).contains(&z) {
        return Err(crate::i18n::t_fmt("cmd.invol.check.z_range_insp", &[("v0", &z.to_string())]));
    }
    if !x.is_finite() {
        return Err(crate::i18n::t_fmt("cmd.invol.check.x_finite", &[("v0", &trim(x))]));
    }
    let a = 30f64.to_radians();
    let d = m * z as f64;
    let rb = d * a.cos() / 2.0;
    let d_a1 = d + 2.0 * x * m + 0.9 * m;
    let limit = d_a1 - 2.0 * SPAN_TIP_MARGIN_FACTOR * m;
    let mut best: Option<u32> = None;
    for k in 1..z {
        let w = base_tangent_length(m, z, x, k);
        let d_m = 2.0 * (rb * rb + (w / 2.0) * (w / 2.0)).sqrt();
        if d_m <= limit {
            best = Some(k);
        } else {
            break;
        }
    }
    best.ok_or_else(|| {
        crate::i18n::t_fmt("cmd.invol.check.no_span_k", &[("v0", &trim(m)), ("v1", &z.to_string()), ("v2", &trim(x))])
    })
}

/// 量棒中心圆半径与奇/偶齿修正因子（DIN 5480-2 p09 式 (1)~(4)/(7)~(10)）：
/// `invδ = invα + s/d ∓ D_M/d_b`（外花键取 `−π/z` 项再 `+D`；内花键 `−D`），
/// `r_M = d_b/(2·cosδ)`；奇数齿的测量距离含 `cos(π/(2z))` 因子。
fn ball_center_radius(
    m: f64,
    z: u32,
    x: f64,
    d_m: f64,
    internal: bool,
) -> Result<(f64, f64), String> {
    if !(m.is_finite() && m > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.check.m_pos_span", &[("v0", &trim(m))]));
    }
    if !(3..=1000).contains(&z) {
        return Err(crate::i18n::t_fmt("cmd.invol.check.z_range_insp", &[("v0", &z.to_string())]));
    }
    if !x.is_finite() {
        return Err(crate::i18n::t_fmt("cmd.invol.check.x_finite", &[("v0", &trim(x))]));
    }
    if !(d_m.is_finite() && d_m > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.check.dm_pos", &[("v0", &trim(d_m))]));
    }
    let a = 30f64.to_radians();
    let d = m * z as f64;
    let db = d * a.cos();
    let s = std::f64::consts::PI * m / 2.0 + 2.0 * x * m * a.tan();
    if !(db > 0.0 && s > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.check.geom_nonpositive", &[("v0", &trim(m)), ("v1", &z.to_string()), ("v2", &trim(x)), ("v3", &trim(db)), ("v4", &trim(s))]));
    }
    let inv_delta = if internal {
        inv(a) + s / d - d_m / db
    } else {
        inv(a) + s / d - std::f64::consts::PI / z as f64 + d_m / db
    };
    let delta = inv_solve(inv_delta).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.invol.check.inv_delta_no_solution",
            &[
                ("v0", &format!("{inv_delta}")),
                ("v1", &trim(m)),
                ("v2", &z.to_string()),
                ("v3", &trim(x)),
                ("v4", &trim(d_m)),
            ],
        )
    })?;
    let r_m = db / (2.0 * delta.cos());
    let fac = if z % 2 == 0 {
        1.0
    } else {
        (std::f64::consts::PI / (2.0 * z as f64)).cos()
    };
    Ok((r_m, fac))
}

/// **外花键跨棒距 M1**（over 2 measuring circles）：偶齿 `2·r_M + D_M`，
/// 奇齿 `2·r_M·cos(π/(2z)) + D_M`。
pub fn measure_over_balls(m: f64, z: u32, x: f64, d_m: f64) -> Result<f64, String> {
    let (r_m, fac) = ball_center_radius(m, z, x, d_m, false)?;
    Ok(2.0 * r_m * fac + d_m)
}

/// **内花键棒间距 M2**（between 2 measuring circles）：偶齿 `2·r_M − D_M`，
/// 奇齿 `2·r_M·cos(π/(2z)) − D_M`（`e = s = πm/2 + 2xm·tanα`）。
pub fn measure_between_balls(m: f64, z: u32, x: f64, d_m: f64) -> Result<f64, String> {
    let (r_m, fac) = ball_center_radius(m, z, x, d_m, true)?;
    Ok(2.0 * r_m * fac - d_m)
}

/// 检验值查询结果：`row` 为 `Some` = 精确查表命中，`None` = 公式导出（自定义 z）。
#[derive(Debug, Clone, PartialEq)]
pub struct InspectionResult {
    /// 基准直径 `d_B`。
    pub d_b: f64,
    /// 模数 m。
    pub m: f64,
    /// 齿数 z。
    pub z: u32,
    /// 变位系数 x（查表命中 = 由 d_B 反解；公式路径 = 由 d_B 反解）。
    pub x: f64,
    /// 内花键量棒直径（配 M2）。
    pub d_m_hub: f64,
    /// 内花键棒间距 M2。
    pub m2: f64,
    /// 内花键偏差系数（表行才有；公式路径借用邻近行）。
    pub a_m2: Option<f64>,
    /// 外花键量棒直径（配 M1）。
    pub d_m_shaft: f64,
    /// 外花键跨棒距 M1。
    pub m1: f64,
    /// 外花键偏差系数。
    pub a_m1: Option<f64>,
    /// 跨测齿数 k。
    pub k: u32,
    /// 公法线长度 W_k。
    pub w_k: f64,
    /// 精确命中的检验表行（公式路径为 `None`）。
    pub row: Option<InspectionRow>,
    /// 来源说明（查表页/source 或「公式导出」）。
    pub source: String,
    /// 附加提示（脚注 a、公式借用邻近行的 D_M 等）。
    pub notes: Vec<String>,
}

impl InspectionResult {
    /// 是否精确查表命中（`false` = 公式导出）。
    pub fn from_table(&self) -> bool {
        self.row.is_some()
    }
}

/// 公式路径借用 D_M 的邻近行（同 m、|Δz| 最小、同距取小 z）。
fn nearest_inspection_row(m: f64, z: u32) -> Option<InspectionRow> {
    inspection_rows()
        .iter()
        .filter(|r| (r.m - m).abs() < DIN_M_TOL)
        .min_by(|a, b| {
            let da = (a.z as f64 - z as f64).abs();
            let db = (b.z as f64 - z as f64).abs();
            da.partial_cmp(&db).unwrap().then(a.z.cmp(&b.z))
        })
        .cloned()
}

/// **检验值查询**：先按 `(d_B, m, z)` 精确查表；不在表里（自定义 z）则用公式导出
/// （`x` 由 [`x_from_d_b`] 反解、`k` 由 [`span_teeth`]、D_M 取邻近表行）。
/// 公式在全部可算表行上逐行对照通过（见 [`inspection_formula_report`]），否则报
/// 「该 z 不在表中、公式未验证通过，请提供对应表页」。
pub fn inspection_query(d_b: f64, m: f64, z: u32) -> Result<InspectionResult, String> {
    if !(d_b.is_finite() && d_b > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.check.db_pos", &[("v0", &trim(d_b))]));
    }
    if !(m.is_finite() && m > 0.0) {
        return Err(crate::i18n::t_fmt("cmd.invol.check.m_pos", &[("v0", &trim(m))]));
    }
    if !(3..=1000).contains(&z) {
        return Err(crate::i18n::t_fmt("cmd.invol.check.z_range_insp_table", &[("v0", &z.to_string())]));
    }
    // ① 精确查表。
    if let Some(row) = inspection_match(d_b, Some(m)).into_iter().find(|r| r.z == z) {
        let mut notes = Vec::new();
        if row.flags.contains("footnote_marker") {
            notes.push(
                crate::i18n::t_static("cmd.invol.check.footnote_a")
                    .to_string(),
            );
        }
        return Ok(InspectionResult {
            d_b: row.d_b,
            m: row.m,
            z: row.z,
            x: x_from_d_b(row.d_b, row.m, row.z),
            d_m_hub: row.d_m_hub,
            m2: row.m2,
            a_m2: Some(row.a_m2),
            d_m_shaft: row.d_m_shaft,
            m1: row.m1,
            a_m1: Some(row.a_m1),
            k: row.k,
            w_k: row.w_k,
            source: crate::i18n::t_fmt("cmd.invol.check.src_table", &[("v0", &row.source_note())]),
            row: Some(row),
            notes,
        });
    }
    // ② 公式导出（自定义 z）。
    if !inspection_formula_validated() {
        return Err(crate::i18n::t_fmt(
            "cmd.invol.check.z_no_formula",
            &[("z", &z.to_string())],
        ));
    }
    let near = nearest_inspection_row(m, z).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.invol.check.z_no_ball_row",
            &[
                ("z", &z.to_string()),
                ("v0", &inspection_miss_message(d_b, Some(m))),
            ],
        )
    })?;
    let x = x_from_d_b(d_b, m, z);
    if !(-0.05 - 1e-9..=0.45 + 1e-9).contains(&x) {
        return Err(crate::i18n::t_fmt(
            "cmd.invol.check.z_x_out_of_range",
            &[
                ("z", &z.to_string()),
                ("v0", &trim(d_b)),
                ("v1", &trim(m)),
                ("v2", &trim(x)),
            ],
        ));
    }
    let k = span_teeth(m, z, x)?;
    let w_k = base_tangent_length(m, z, x, k);
    let m1 = measure_over_balls(m, z, x, near.d_m_shaft)?;
    let m2 = measure_between_balls(m, z, x, near.d_m_hub)?;
    Ok(InspectionResult {
        d_b,
        m,
        z,
        x,
        d_m_hub: near.d_m_hub,
        m2,
        a_m2: Some(near.a_m2),
        d_m_shaft: near.d_m_shaft,
        m1,
        a_m1: Some(near.a_m1),
        k,
        w_k,
        row: None,
        source: crate::i18n::t_fmt("cmd.invol.check.src_formula_z", &[("z", &z.to_string())]),
        notes: vec![
            crate::i18n::t_fmt("cmd.invol.check.notes_line1", &[("v0", &trim(INSPECTION_TOL))]),
            crate::i18n::t_fmt("cmd.invol.check.notes_line2", &[("v0", &near.source_note()), ("v1", &trim(near.d_m_hub)), ("v2", &trim(near.d_m_shaft)), ("k", &k.to_string())]),
        ],
    })
}

/// 单行检验结果摘要（GUI/命令行 info 用；含来源）。
pub fn inspection_summary(r: &InspectionResult) -> String {
    crate::i18n::t_fmt("cmd.invol.check.summary", &[("v0", &trim(r.m1)), ("v1", &trim(r.d_m_shaft)), ("v2", &trim(r.m2)), ("v3", &trim(r.d_m_hub)), ("v4", &r.k.to_string()), ("v5", &trim(r.w_k)), ("v6", &r.source)])
}

/// `/api/invol_check`：`?db=40&m=2&z=18[&check=1]` → 检验值 JSON。
/// 不给 `check`（或 `check=0`）时**只认精确查表命中**；`check=1`/`CHECK` 才允许公式导出。
pub fn inspection_json(query: &str) -> Result<String, String> {
    let param = |key: &str| {
        query
            .split('&')
            .filter_map(|kv| kv.split_once('='))
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
    };
    let db = param("db")
        .or_else(|| param("d_b"))
        .and_then(|v| v.parse::<f64>().ok())
        .ok_or_else(|| crate::i18n::t_static("cmd.invol.check.missing_db").to_string())?;
    let m = param("m")
        .and_then(|v| v.parse::<f64>().ok())
        .ok_or_else(|| crate::i18n::t_static("cmd.invol.check.missing_m").to_string())?;
    let z = param("z")
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or_else(|| crate::i18n::t_static("cmd.invol.check.missing_z").to_string())?;
    let check = param("check").is_some_and(|v| !(v == "0" || v.eq_ignore_ascii_case("false")));
    if !check && inspection_match(db, Some(m)).iter().all(|r| r.z != z) {
        return Err(crate::i18n::t_fmt("cmd.invol.check.need_check_flag", &[("v0", &inspection_miss_message(db, Some(m)))]));
    }
    let r = inspection_query(db, m, z)?;
    let mut obj = serde_json::json!({
        "ok": true,
        "from_table": r.from_table(),
        "db": r.d_b,
        "m": r.m,
        "z": r.z,
        "x": r.x,
        "d_m_hub": r.d_m_hub,
        "m2": r.m2,
        "a_m2": r.a_m2,
        "d_m_shaft": r.d_m_shaft,
        "m1": r.m1,
        "a_m1": r.a_m1,
        "k": r.k,
        "w_k": r.w_k,
        "source": r.source,
        "notes": r.notes,
    });
    if let Some(row) = &r.row {
        obj["page"] = serde_json::json!(row.page);
        obj["table_no"] = serde_json::json!(row.table_no);
        obj["row_source"] = serde_json::json!(row.source);
        obj["flags"] = serde_json::json!(row.flags);
    }
    Ok(obj.to_string())
}

/// 检验表一行（含公式算值）的逐行残差。
#[derive(Debug, Clone, PartialEq)]
pub struct InspectionResidual {
    /// 源页码。
    pub page: u16,
    /// 页内表号。
    pub table_no: u32,
    /// OCR 来源。
    pub source: String,
    /// 模数。
    pub m: f64,
    /// 基准直径。
    pub d_b: f64,
    /// 齿数。
    pub z: u32,
    /// 名义表反查的 x（`None` = 名义表无该 (m,d_B,z)，如 z=93/97 残行）。
    pub x: Option<f64>,
    /// 表值 W_k / M1 / M2。
    pub w_k_table: f64,
    /// 表值 M1。
    pub m1_table: f64,
    /// 表值 M2。
    pub m2_table: f64,
    /// 公式算值。
    pub w_k_calc: Option<f64>,
    /// 公式算值 M1。
    pub m1_calc: Option<f64>,
    /// 公式算值 M2。
    pub m2_calc: Option<f64>,
    /// 残差（表值 − 算值，绝对值）。
    pub w_k_residual: Option<f64>,
    /// M1 残差。
    pub m1_residual: Option<f64>,
    /// M2 残差。
    pub m2_residual: Option<f64>,
    /// 异常说明（空串 = 通过）。
    pub note: String,
}

/// 逐行残差报告汇总。
#[derive(Debug, Clone, PartialEq)]
pub struct InspectionFormulaReport {
    /// 入库总行数。
    pub total: usize,
    /// 有名义 x、可做公式对照的行数。
    pub checked: usize,
    /// 无名义 x 被跳过的行 `(page, d_B, z)`。
    pub skipped: Vec<(u16, f64, u32)>,
    /// 原始残差 ≤ 容差的行数（W_k / M1 / M2）。
    pub w_k_pass: usize,
    /// M1 通过行数。
    pub m1_pass: usize,
    /// M2 通过行数。
    pub m2_pass: usize,
    /// 通过行的最大残差。
    pub max_w_k_residual: f64,
    /// 通过行的最大 M1 残差。
    pub max_m1_residual: f64,
    /// 通过行的最大 M2 残差。
    pub max_m2_residual: f64,
    /// > 容差的源数据异常（含源图核对后的修正说明，未改写 CSV）。
    pub anomalies: Vec<String>,
    /// 公式是否全表验证通过（异常均有源图核对修正且修正后通过）。
    pub validated: bool,
    /// 一行摘要。
    pub summary: String,
}

/// 已按源图人工核对的 OCR 异常（17 处：16 处量棒直径/表值 + 1 处 k）：
/// `(page, m, d_B, z, 字段, 源图正确值)`。字段 = `d_m_hub`/`d_m_shaft`/`m2`/`k`。
/// **不改写入库 CSV**，只用于残差报告的「修正后复核」与公式可信度判定。
const INSPECTION_OCR_FIXES: &[(u16, f64, f64, u32, &str, f64)] = &[
    (16, 0.75, 25.0, 32, "d_m_shaft", 1.55),
    (16, 0.75, 28.0, 36, "d_m_shaft", 1.55),
    (16, 0.75, 32.0, 41, "d_m_shaft", 1.55),
    (16, 0.75, 34.0, 44, "d_m_shaft", 1.55),
    (16, 0.75, 37.0, 48, "d_m_shaft", 1.55),
    (16, 0.75, 38.0, 49, "d_m_shaft", 1.55),
    (16, 0.75, 42.0, 54, "d_m_shaft", 1.55),
    (16, 0.75, 45.0, 58, "d_m_shaft", 1.55),
    (16, 0.75, 48.0, 62, "d_m_shaft", 1.55),
    (18, 0.8, 17.0, 20, "d_m_hub", 1.55),
    (18, 0.8, 28.0, 34, "d_m_hub", 1.55),
    (18, 0.8, 34.0, 41, "d_m_hub", 1.55),
    (18, 0.8, 36.0, 44, "d_m_hub", 1.55),
    (18, 0.8, 45.0, 55, "d_m_hub", 1.55),
    (18, 0.8, 52.0, 64, "d_m_hub", 1.55),
    (20, 1.0, 10.0, 8, "m2", 5.583),
    (18, 0.8, 29.0, 35, "k", 6.0),
];

/// 查找源图核对修正记录。
fn inspection_ocr_fix(
    page: u16,
    m: f64,
    d_b: f64,
    z: u32,
) -> Option<&'static (u16, f64, f64, u32, &'static str, f64)> {
    INSPECTION_OCR_FIXES.iter().find(|(p, mm, db, zz, _, _)| {
        *p == page
            && (*mm - m).abs() < DIN_M_TOL
            && (*db - d_b).abs() < DIN_D_B_TOL
            && *zz == z
    })
}

/// 267 行逐行残差（表值 vs 公式值；`x` 由名义表反查）。
pub fn inspection_residuals() -> Vec<InspectionResidual> {
    let mut out = Vec::with_capacity(inspection_rows().len());
    for row in inspection_rows() {
        let nom = din5480_rows().iter().find(|r| {
            (r.m - row.m).abs() < DIN_M_TOL
                && (r.d_b - row.d_b).abs() < DIN_D_B_TOL
                && r.z == row.z
        });
        let mut res = InspectionResidual {
            page: row.page,
            table_no: row.table_no,
            source: row.source.clone(),
            m: row.m,
            d_b: row.d_b,
            z: row.z,
            x: nom.map(|r| r.x),
            w_k_table: row.w_k,
            m1_table: row.m1,
            m2_table: row.m2,
            w_k_calc: None,
            m1_calc: None,
            m2_calc: None,
            w_k_residual: None,
            m1_residual: None,
            m2_residual: None,
            note: String::new(),
        };
        let Some(x) = res.x else {
            res.note = crate::i18n::t_static("cmd.invol.check.residual_x_note").to_string();
            out.push(res);
            continue;
        };
        let w = base_tangent_length(row.m, row.z, x, row.k);
        let m1 = measure_over_balls(row.m, row.z, x, row.d_m_shaft);
        let m2 = measure_between_balls(row.m, row.z, x, row.d_m_hub);
        res.w_k_calc = Some(w);
        res.w_k_residual = Some((w - row.w_k).abs());
        match m1 {
            Ok(v) => {
                res.m1_calc = Some(v);
                res.m1_residual = Some((v - row.m1).abs());
            }
            Err(e) => res.note.push_str(&crate::i18n::t_fmt("cmd.invol.check.m1_fail", &[("e", &e)])),
        }
        match m2 {
            Ok(v) => {
                res.m2_calc = Some(v);
                res.m2_residual = Some((v - row.m2).abs());
            }
            Err(e) => res.note.push_str(&crate::i18n::t_fmt("cmd.invol.check.m2_fail", &[("e", &e)])),
        }
        // 异常标注 + 源图核对修正复核。
        let mut bad = Vec::new();
        if res.w_k_residual.is_some_and(|r| r > INSPECTION_TOL) {
            bad.push("W_k");
        }
        if res.m1_residual.is_some_and(|r| r > INSPECTION_TOL) {
            bad.push("M1");
        }
        if res.m2_residual.is_some_and(|r| r > INSPECTION_TOL) {
            bad.push("M2");
        }
        if !bad.is_empty() {
            match inspection_ocr_fix(row.page, row.m, row.d_b, row.z) {
                Some((_, _, _, _, field, value)) => {
                    // 用源图正确值重算该字段。
                    let fixed = match *field {
                        "d_m_shaft" => Some(
                            measure_over_balls(row.m, row.z, x, *value)
                                .map(|v| (v - row.m1).abs())
                                .unwrap_or((*value - row.m1).abs()),
                        ),
                        "d_m_hub" => Some(
                            measure_between_balls(row.m, row.z, x, *value)
                                .map(|v| (v - row.m2).abs())
                                .unwrap_or((*value - row.m2).abs()),
                        ),
                        "m2" => res.m2_calc.map(|calc| (calc - *value).abs()),
                        "k" => Some((base_tangent_length(row.m, row.z, x, *value as u32) - row.w_k).abs()),
                        _ => None,
                    };
                    let fixed_ok = fixed.is_some_and(|r| r <= INSPECTION_TOL);
                    res.note = crate::i18n::t_fmt(
                        "cmd.invol.check.ocr_anomaly",
                        &[
                            ("v0", &bad.join("/")),
                            ("v1", &field),
                            (
                                "v2",
                                &if *field == "k" {
                                    row.k.to_string()
                                } else {
                                    trim(match *field {
                                        "d_m_shaft" => row.d_m_shaft,
                                        "d_m_hub" => row.d_m_hub,
                                        "m2" => row.m2,
                                        _ => row.k as f64,
                                    })
                                },
                            ),
                            ("v3", &trim(*value)),
                            (
                                "v4",
                                if fixed_ok {
                                    crate::i18n::t_static("cmd.invol.check.ocr_fixed_ok")
                                } else {
                                    crate::i18n::t_static("cmd.invol.check.ocr_still_bad")
                                },
                            ),
                        ],
                    );
                }
                None => {
                    res.note = crate::i18n::t_fmt("cmd.invol.check.mismatch_no_fix", &[("v0", &bad.join("/"))]);
                }
            }
        }
        out.push(res);
    }
    out
}

static INSPECTION_FORMULA_REPORT: OnceLock<InspectionFormulaReport> = OnceLock::new();

/// 267 行逐行对照报告（缓存；含最大残差、异常清单与“公式已验证”判定）。
pub fn inspection_formula_report() -> &'static InspectionFormulaReport {
    INSPECTION_FORMULA_REPORT.get_or_init(|| {
        let residuals = inspection_residuals();
        let mut report = InspectionFormulaReport {
            total: residuals.len(),
            checked: 0,
            skipped: Vec::new(),
            w_k_pass: 0,
            m1_pass: 0,
            m2_pass: 0,
            max_w_k_residual: 0.0,
            max_m1_residual: 0.0,
            max_m2_residual: 0.0,
            anomalies: Vec::new(),
            validated: true,
            summary: String::new(),
        };
        for r in &residuals {
            let Some(_) = r.x else {
                report.skipped.push((r.page, r.d_b, r.z));
                continue;
            };
            report.checked += 1;
            if let Some(v) = r.w_k_residual {
                if v <= INSPECTION_TOL {
                    report.w_k_pass += 1;
                    report.max_w_k_residual = report.max_w_k_residual.max(v);
                }
            }
            if let Some(v) = r.m1_residual {
                if v <= INSPECTION_TOL {
                    report.m1_pass += 1;
                    report.max_m1_residual = report.max_m1_residual.max(v);
                }
            }
            if let Some(v) = r.m2_residual {
                if v <= INSPECTION_TOL {
                    report.m2_pass += 1;
                    report.max_m2_residual = report.max_m2_residual.max(v);
                }
            }
            if !r.note.is_empty() {
                let fix_ok = inspection_ocr_fix(r.page, r.m, r.d_b, r.z)
                    .is_some_and(|(_, _, _, _, _, _)| r.note.contains(crate::i18n::t_static("cmd.invol.check.fixed_pass")));
                if !fix_ok {
                    report.validated = false;
                }
                report.anomalies.push(crate::i18n::t_fmt(
                    "cmd.invol.check.anomaly_row",
                    &[
                        ("v0", &r.page.to_string()),
                        ("v1", &trim(r.d_b)),
                        ("v2", &trim(r.m)),
                        ("v3", &r.z.to_string()),
                        ("v4", &r.source),
                        ("v5", &r.note),
                    ],
                ));
            }
        }
        report.summary = crate::i18n::t_fmt(
            "cmd.invol.check.formula_summary",
            &[
                ("v0", &report.total.to_string()),
                ("v1", &report.checked.to_string()),
                ("v2", &report.skipped.len().to_string()),
                ("v3", &report.w_k_pass.to_string()),
                ("v4", &report.checked.to_string()),
                ("v5", &report.m1_pass.to_string()),
                ("v6", &report.checked.to_string()),
                ("v7", &report.m2_pass.to_string()),
                ("v8", &report.checked.to_string()),
                ("v9", &trim(INSPECTION_TOL)),
                ("v10", &format!("{:.4}", report.max_w_k_residual)),
                ("v11", &format!("{:.4}", report.max_m1_residual)),
                ("v12", &format!("{:.4}", report.max_m2_residual)),
                ("v13", &report.anomalies.len().to_string()),
                ("v14", &trim(INSPECTION_TOL)),
                (
                    "v15",
                    if report.validated {
                        crate::i18n::t_static("cmd.invol.check.validated")
                    } else {
                        crate::i18n::t_static("cmd.invol.check.not_validated")
                    },
                ),
            ],
        );
        report
    })
}

/// 公式是否已通过 267 行逐行对照（未通过时 [`inspection_query`] 拒绝自定义 z）。
pub fn inspection_formula_validated() -> bool {
    inspection_formula_report().validated
}

// ─────────────────────────── 参数 ───────────────────────────

/// 渐开线花键几何参数（预设 + 逐个覆盖）。
#[derive(Debug, Clone, PartialEq)]
pub struct InvolParams {
    /// 模数 m（ANSI 下 `m = 25.4/P`，由径节换算；引擎内部与输出统一 mm）。
    pub m: f64,
    /// 齿数 z。
    pub z: u32,
    /// **ANSI B92.1 径节 P**（`std=ANSI` 时 `Some`；仅用于 `P/Ps` 标识与 Table 2 分段；
    /// 引擎内 `m = 25.4/P`（mm））。
    pub pitch: Option<f64>,
    /// 变位系数 x（GB 通常 0；DIN 见 `x·m` 范围）。
    pub x: f64,
    /// 压力角 α（度）。
    pub alpha_deg: f64,
    /// 齿顶高系数 ha*。
    pub ha_star: f64,
    /// 齿根高系数 hf*。
    pub hf_star: f64,
    /// 齿根圆角系数 ρf*（`ρf = rho_star·m`）。
    pub rho_star: f64,
    /// 齿形裕度系数 cF*（`cF = c_f_star·m`）。
    pub c_f_star: f64,
    /// 所属标准。
    pub std: SplineStd,
    /// 齿廓名（预设名，如 `30圆齿根` / `DIN30`）。
    pub profile: &'static str,
    /// DIN 5480 基准直径 d_B（身份标识/显示；`None` = 未给，外部传入/回填）。
    pub d_b: Option<f64>,
    /// NF E22-141 公称直径 `A`（NF 的基准直径主参数；`None` = 未给）。
    pub a: Option<f64>,
    /// 内花键（hub）：材料在外侧、齿朝内（`false` = 外花键，齿朝外）。
    pub internal: bool,
    /// GB 图 2 的 `h_s`（从预设带入；仅 `D_Fe max` 公式用）。
    pub h_s_star: f64,
}

impl InvolParams {
    /// 由预设构造（`x=0`、`d_b=None`、`a=None`、外花键），立即校验。
    pub fn from_preset(std: SplineStd, profile: &str, m: f64, z: u32) -> Result<Self, String> {
        let preset = preset(std, profile)?;
        let p = Self {
            m,
            z,
            pitch: None,
            x: 0.0,
            alpha_deg: preset.alpha_deg,
            ha_star: preset.ha_star,
            hf_star: preset.hf_star,
            rho_star: preset.rho_star,
            c_f_star: preset.c_f_star,
            std,
            profile: preset.profile,
            d_b: None,
            a: None,
            internal: false,
            h_s_star: preset.h_s_star,
        };
        p.validate()?;
        Ok(p)
    }

    /// GB/T 3478.1-2008（默认预设）构造。
    pub fn gb(profile: &str, m: f64, z: u32) -> Result<Self, String> {
        Self::from_preset(SplineStd::GB, profile, m, z)
    }

    /// DIN 5480-1:2015 构造（`x` 必须在 `[−0.05, 0.45]`）。
    pub fn din(m: f64, z: u32, x: f64) -> Result<Self, String> {
        let mut p = Self::from_preset(SplineStd::DIN, "DIN30", m, z)?;
        p.x = x;
        p.validate()?;
        Ok(p)
    }

    /// ANSI B92.1-1970 (R1993) 构造：`p` = 径节 P，`z` = 齿数 N（无变位）。
    ///
    /// 标准公式是英寸口径（`D = N/P` 英寸）；引擎内部统一 mm：`m = 25.4/P`，
    /// 因而 `D/Do/Dri/Di/Dre/DFe/DFi/p` 等输出全为 mm。
    /// `profile` 必须是 Table 2 五列之一（见 [`ANSI_PRESETS`]）；用
    /// [`ansi_profile_name`] 可由「压力角 + 齿根型式 + 配合方式」拿到合法齿廓名。
    pub fn ansi(profile: &str, p: f64, z: u32) -> Result<Self, String> {
        if !(p.is_finite() && p > 0.0) {
            return Err(ansi_prefixed(crate::i18n::t_fmt("cmd.invol.check.pitch_pos", &[("v0", &trim(p))])));
        }
        // 构造即用 mm 模数；`pitch` 回填原值（`ansi_p()` 优先用它，避免整数/小数往返丢精度）。
        let mut q = Self::from_preset(SplineStd::ANSI, profile, ANSI_INCH_MM / p, z)
            .map_err(ansi_prefixed)?;
        q.pitch = Some(p);
        q.validate().map_err(ansi_prefixed)?;
        Ok(q)
    }

    // ── 覆盖（builder；调用方负责再 `validate()`）──

    /// 覆盖变位系数 x。
    pub fn with_x(mut self, x: f64) -> Self {
        self.x = x;
        self
    }

    /// 覆盖/回填 ANSI 径节 P（**不改 `m`**；调用方自行保证 `m = 25.4/P`）。
    pub fn with_pitch(mut self, p: f64) -> Self {
        self.pitch = Some(p);
        self
    }

    /// 覆盖压力角（度）。
    pub fn with_alpha_deg(mut self, alpha_deg: f64) -> Self {
        self.alpha_deg = alpha_deg;
        self
    }

    /// 逐个覆盖四个系数。
    pub fn with_coeffs(mut self, ha_star: f64, hf_star: f64, rho_star: f64, c_f_star: f64) -> Self {
        self.ha_star = ha_star;
        self.hf_star = hf_star;
        self.rho_star = rho_star;
        self.c_f_star = c_f_star;
        self
    }

    /// 外部传入/回填 DIN 基准直径 d_B（身份标识/显示；DIN 内花键几何也直接用它 `d_f2=d_B`）。
    pub fn with_d_b(mut self, d_b: f64) -> Self {
        self.d_b = Some(d_b);
        self
    }

    /// 外部传入/回填 NF 公称直径 `A`（身份标识/显示；NF 内花键几何也直接用它）。
    pub fn with_a(mut self, a: f64) -> Self {
        self.a = Some(a);
        self
    }

    /// 切换内花键（`true`）/外花键（`false`）。
    pub fn with_internal(mut self, internal: bool) -> Self {
        self.internal = internal;
        self
    }

    // ── 导出量 ──

    /// 压力角 α（弧度）。
    pub fn alpha(&self) -> f64 {
        self.alpha_deg.to_radians()
    }

    /// 分度圆直径 `d = m·z`（DIN 表 2 的 “Pitch diameter d” 口径）。
    pub fn d(&self) -> f64 {
        self.m * self.z as f64
    }

    /// 计入变位的计算直径 `d′ = m·z + 2x·m`（GB x=0 时 = d；
    /// = `d_a1 − 0.9m` = `d_f1 + 2·hf*·m`）。
    pub fn d_eff(&self) -> f64 {
        self.d() + 2.0 * self.x * self.m
    }

    /// 基圆直径 `db = d·cosα`。
    pub fn db(&self) -> f64 {
        self.d() * self.alpha().cos()
    }

    /// 齿顶圆直径：`d′ + 2·ha*·m`（GB 即 m(z+1) / m(z+0.9) / m(z+0.8)；
    /// DIN 即 `d_a1 = mz+2xm+0.9m`）。
    pub fn da(&self) -> f64 {
        self.d_eff() + 2.0 * self.ha_star * self.m
    }

    /// 齿根圆直径：GB/DIN/NF 为 `d′ − 2·hf*·m`（GB 即 m(z−1.5)/m(z−1.8)/m(z−1.4)/m(z−1.2)；
    /// DIN 即 `d_f1 = mz+2xm−2·hf*·m`）；ANSI 为 Table 2 的 `Dre`（按径节三段，见
    /// [`InvolParams::ansi_dre_offset`]）。
    pub fn df(&self) -> f64 {
        if self.std == SplineStd::ANSI {
            let k = self.ansi_dre_offset().unwrap_or(0.0);
            return self.d() - k * self.m;
        }
        self.d_eff() - 2.0 * self.hf_star * self.m
    }

    /// 分度圆齿厚 `s = mπ/2 + 2x·m·tanα`；ANSI 为 `t = p − Sv min`（Table 2；30° 即 `π/(2P)`）。
    pub fn s(&self) -> f64 {
        if self.std == SplineStd::ANSI {
            return std::f64::consts::PI * self.m - self.ansi_sv_min();
        }
        std::f64::consts::PI * self.m / 2.0 + 2.0 * self.x * self.m * self.alpha().tan()
    }

    /// 变位量 `x·m`。
    pub fn x_m(&self) -> f64 {
        self.x * self.m
    }

    /// 齿根圆角半径 `ρf = rho_star·m`；ANSI 无标准值，用切于齿根的过渡弧构造
    /// （见 [`InvolParams::ansi_fillet_radius`]，**无标准数值依据**）。
    pub fn rho_f(&self) -> f64 {
        if self.std == SplineStd::ANSI {
            return self.ansi_fillet_radius();
        }
        self.rho_star * self.m
    }

    /// 齿形裕度 `cF = c_f_star·m`；ANSI 为 `cF = clamp(0.001·D, 0.002 in, 0.010 in)`
    /// = `clamp(0.001·D_mm, 0.0508, 0.254)`（Table 2；D 与返回值均 mm）。
    pub fn c_f(&self) -> f64 {
        if self.std == SplineStd::ANSI {
            return ansi_c_f(self.d());
        }
        self.c_f_star * self.m
    }

    /// 顶隙 `c = (hf*−ha*)·m`（DIN 为 0.10m；GB 为几何差值，不另查表）。
    pub fn clearance(&self) -> f64 {
        (self.hf_star - self.ha_star) * self.m
    }

    /// 齿距角 `2π/z`（弧度）。
    pub fn pitch_angle(&self) -> f64 {
        std::f64::consts::TAU / self.z as f64
    }

    /// 齿高 `(da−df)/2`（收尾公式里的 h）。
    pub fn tooth_depth(&self) -> f64 {
        (self.da() - self.df()) / 2.0
    }

    /// 齿厚半角 `ψ(R) = s/d + invα − invα_R`（弧度；`R<db` 时按 db 夹住，与 `gear.rs` 同口径）。
    pub fn half_tooth_angle(&self, radius: f64) -> f64 {
        self.half_angle_at(radius, self.s())
    }

    /// 以分度圆处圆周宽 `w` 定义的半角 `ψ(R)=w/(2r)+invα−invα_R`（弧度；`R<db` 时按 db 夹住）。
    fn half_angle_at(&self, radius: f64, w: f64) -> f64 {
        let r = self.d() / 2.0;
        let rb = self.db() / 2.0;
        let a_r = (rb / radius.max(rb)).clamp(-1.0, 1.0).acos();
        w / (2.0 * r) + (inv(self.alpha()) - inv(a_r))
    }

    /// 内花键齿槽半角：GB/DIN/NF 沿用齿厚口径（内花键齿槽 = 同参数外花键齿形）；
    /// ANSI 用 `Sv min`（内花键基本齿槽宽，Table 2）。
    fn internal_half_space_angle(&self, radius: f64) -> f64 {
        if self.std == SplineStd::ANSI {
            self.half_angle_at(radius, self.ansi_sv_min())
        } else {
            self.half_tooth_angle(radius)
        }
    }

    /// 渐开线起始半径（外花键）：`max(db, df)/2`（基圆以内没有渐开线）；
    /// ANSI 按 Table 2 从研开线起始圆 `DFe` 起（不低于齿根/基圆）。
    pub fn r_involute_start(&self) -> f64 {
        if self.std == SplineStd::ANSI {
            return (self.db() / 2.0)
                .max(self.df() / 2.0)
                .max(self.ansi_form_dia_external() / 2.0);
        }
        (self.db() / 2.0).max(self.df() / 2.0)
    }

    /// 渐开线终止半径（外花键）：大径/2。
    pub fn r_involute_end(&self) -> f64 {
        self.da() / 2.0
    }

    /// 渐开线起始圆直径（显示用）。
    pub fn d_involute_start(&self) -> f64 {
        2.0 * self.r_involute_start()
    }

    /// 渐开线终止圆直径（= 大径，显示用）。
    pub fn d_involute_end(&self) -> f64 {
        self.da()
    }

    /// GB 表 3 `D_Fe max`（外花键渐开线起始圆直径最大值，`es_v=0`，H/h 配合）：
    /// `2√((0.5D_b)² + (0.5D·sinα − h_s/sinα)²)`（GB/T 3478.1 表 3 p10，注 2）。
    pub fn gb_form_dia_max(&self) -> f64 {
        let d = self.d();
        let db = self.db();
        let a = self.alpha();
        let t = 0.5 * d * a.sin() - self.h_s_star * self.m / a.sin();
        2.0 * ((0.5 * db).powi(2) + t * t).sqrt()
    }

    /// **内花键大径**（外侧齿根 / 齿槽底）：GB 表 3 `D_ei = m(z+1.5)/(z+1.8)/(z+1.4)/(z+1.2)`；
    /// DIN = `d_f2 = d_B = m(z+1.1+2x)`（721 行名义表恒等式）；NF = `A`（p07 外径定心 `A₁″=A`）；
    /// ANSI = Table 2 的 `Dri`（如 30°平齿根齿侧 `(N+1.35)/P`…）。
    pub fn internal_major_dia(&self) -> f64 {
        match self.std {
            SplineStd::GB => self.d_eff() + 2.0 * self.hf_star * self.m,
            SplineStd::NF => self.a.unwrap_or_else(|| a_from_x(self.m, self.z, self.x)),
            SplineStd::ANSI => self.d() + self.ansi_dri_offset() * self.m,
            // DIN 5480 恒等式。
            SplineStd::DIN => self
                .d_b
                .unwrap_or_else(|| d_b_from_x(self.m, self.z, self.x)),
        }
    }

    /// **内花键小径**（里侧齿顶）：GB 表 3 `D_ii = D_Fe max + 2C_F`；
    /// DIN = `d_a2 = d − 0.9m + 2xm`；NF = `D = A − 2m`（p07 公式）；
    /// ANSI = Table 2 的 `Di`（30° `(N−1)/P`、37.5° `(N−0.8)/P`、45° `(N−0.6)/P`）。
    pub fn internal_minor_dia(&self) -> f64 {
        match self.std {
            SplineStd::GB => self.gb_form_dia_max() + 2.0 * self.c_f(),
            SplineStd::NF => self.internal_major_dia() - 2.0 * self.m,
            SplineStd::ANSI => self.d() - self.ansi_di_offset() * self.m,
            SplineStd::DIN => self.d_eff() - 0.9 * self.m,
        }
    }

    // ── ANSI B92.1-1970 (R1993) 专用导出量（公式驱动；Table 2）──

    /// ANSI 径节 `P`（`std=ANSI` 时由 `pitch` 存；兼容由 mm 模数 `m = 25.4/P` 反算）。
    pub fn ansi_p(&self) -> f64 {
        self.pitch.unwrap_or(ANSI_INCH_MM / self.m)
    }

    /// ANSI Table 2 列（profile 不是 ANSI 列的返回 `None`）。
    pub fn ansi_column(&self) -> Option<AnsiColumn> {
        if self.std == SplineStd::ANSI {
            AnsiColumn::from_profile(self.profile)
        } else {
            None
        }
    }

    /// ANSI `Sv min`（最小有效齿槽宽）：30° `π/(2P)`；37.5° `(0.5π+0.1)/P`；
    /// 45° `(0.5π+0.2)/P`（标准英寸值；本函数返回值 mm，= 原式 ×25.4 = `(0.5π+k)·m`）。
    ///
    /// 口径：走 [`ansi_sv_min_formula`]（**Table 3** 公式，17 项径节全印、适用面比 Table 2
    /// 每列适用范围更宽）；本方法在已选列/已校验的参数上取值，但公式本身不受列界约束。
    pub fn ansi_sv_min(&self) -> f64 {
        ansi_sv_min_formula(self.alpha_deg, self.ansi_p())
    }

    /// ANSI `Dre = (N−k)/P` 的 `k`（Table 2 三段：`≤12/24` / `≥16/32` / 45° 的 `≥10/20`）。
    pub fn ansi_dre_offset(&self) -> Option<f64> {
        let col = self.ansi_column()?;
        let p = self.ansi_p();
        Some(match col {
            AnsiColumn::A30FlatSide | AnsiColumn::B30FlatMajor => 1.35,
            AnsiColumn::C30FilletSide => {
                if p > 12.0 + 1e-9 {
                    2.0
                } else {
                    1.8
                }
            }
            AnsiColumn::D375FilletSide => 1.3,
            AnsiColumn::E45FilletSide => 1.0,
        })
    }

    /// ANSI `Dri = (N+k)/P` 的 `k`（Table 2）。
    pub fn ansi_dri_offset(&self) -> f64 {
        match self.ansi_column() {
            Some(AnsiColumn::A30FlatSide) => 1.35,
            Some(AnsiColumn::B30FlatMajor) => 1.0,
            Some(AnsiColumn::C30FilletSide) => 1.8,
            Some(AnsiColumn::D375FilletSide) => 1.6,
            Some(AnsiColumn::E45FilletSide) => 1.4,
            None => 1.0,
        }
    }

    /// ANSI `Di = (N−k)/P` 的 `k`（Table 2）。
    pub fn ansi_di_offset(&self) -> f64 {
        match self.ansi_column() {
            Some(AnsiColumn::D375FilletSide) => 0.8,
            Some(AnsiColumn::E45FilletSide) => 0.6,
            _ => 1.0,
        }
    }

    /// ANSI 外花键 form diameter `DFe`（Table 2，渐开线起始圆）。
    pub fn ansi_form_dia_external(&self) -> f64 {
        let base = match self.ansi_column() {
            Some(AnsiColumn::D375FilletSide) => self.d() - 0.8 * self.m,
            Some(AnsiColumn::E45FilletSide) => self.d() - 0.6 * self.m,
            _ => self.d() - self.m,
        };
        base - 2.0 * self.c_f()
    }

    /// ANSI 内花键 form diameter `DFi`（Table 2；列 B 含英寸常量 `−0.004 in = −0.1016 mm`）。
    pub fn ansi_form_dia_internal(&self) -> f64 {
        match self.ansi_column() {
            Some(AnsiColumn::B30FlatMajor) => {
                self.d() + 0.8 * self.m - 0.1016 + 2.0 * self.c_f()
            }
            _ => self.d() + self.m + 2.0 * self.c_f(),
        }
    }

    /// ANSI 齿根过渡圆角半径（**无标准数值依据**；p14 明确不按给定半径规定）。
    ///
    /// 构造与齿轮同类：圆与齿根圆 `Dre/2` 相切（圆心半径 `rf+ρ`），且与渐开线在
    /// form diameter `DFe/2` 处相切（圆心在研开线法线上）。闭式解：
    /// `ρ = (rs²−rf²) / (2·(rf + rs·sinα_s))`，`α_s = acos(db/2/rs)`。
    /// 无解（`rs ≤ rf` 或 `rs ≤ db/2` 等）时回退 0。
    pub fn ansi_fillet_radius(&self) -> f64 {
        let rs = (self.ansi_form_dia_external() / 2.0).max(self.df() / 2.0);
        let rf = self.df() / 2.0;
        let rb = self.db() / 2.0;
        if rs <= rf + 1e-12 || rs <= rb + 1e-12 {
            return 0.0;
        }
        let sin_a_s = ((rs * rs - rb * rb).max(0.0)).sqrt() / rs;
        let rho = (rs * rs - rf * rf) / (2.0 * (rf + rs * sin_a_s));
        if rho.is_finite() && rho > 0.0 {
            rho
        } else {
            0.0
        }
    }

    /// 内花键外侧齿根半径（= `internal_major_dia()/2`）。
    pub fn internal_root_radius(&self) -> f64 {
        self.internal_major_dia() / 2.0
    }

    /// 内花键里侧齿顶半径（= `internal_minor_dia()/2`）。
    pub fn internal_tip_radius(&self) -> f64 {
        self.internal_minor_dia() / 2.0
    }

    /// 内花键渐开线有效区间的起点半径与是否退化为径向直线：
    /// `r_inner = max(D_ii/2, db/2[, DFi/2])`；`radial_only = r_inner ≥ D_ei/2`
    /// （材料带内没有可用渐开线：非 ANSI 为基圆超过外侧齿根，ANSI 为 form 直径 `DFi ≥ Dri`）。
    /// 端视图 / 计算书 / GUI 提示共用这一份口径。
    pub fn internal_involute_band(&self) -> (f64, bool) {
        let rb = self.db() / 2.0;
        let mut r_inner = self.internal_tip_radius().max(rb);
        if self.std == SplineStd::ANSI {
            r_inner = r_inner.max(self.ansi_form_dia_internal() / 2.0);
        }
        let r_root = self.internal_root_radius();
        if r_inner >= r_root - 1e-9 {
            (r_root, true)
        } else {
            (r_inner, false)
        }
    }

    /// 旧的 DIN 名义估算 `m(z+2x)`（= `d + 2x·m`，相当于 `d_B − 1.1m`）；
    /// **不是** `d_B` 定义式。新代码用 [`x_from_d_b`] / [`d_b_from_x`] / [`lookup_by_d_b`]。
    pub fn d_b_estimate(&self) -> f64 {
        self.m * (self.z as f64 + 2.0 * self.x)
    }

    /// DIN 变位是否在 `x·m ∈ [−0.05m, +0.45m]`（即 x∈[−0.05, 0.45]）内。
    pub fn x_in_range(&self) -> bool {
        self.x >= -0.05 - 1e-9 && self.x <= 0.45 + 1e-9
    }

    /// 齿顶变尖/渐开线交叉：`ψ(da/2) ≤ 0`（端视图画不出真实齿廓）。
    pub fn tooth_tip_crossed(&self) -> bool {
        self.half_tooth_angle(self.da() / 2.0) <= 1e-9
    }

    /// 齿槽过宽/相邻齿廓在齿根处交叉：`ψ(r_start) ≥ 半齿距`。
    pub fn root_space_crossed(&self) -> bool {
        self.half_tooth_angle(self.r_involute_start()) >= self.pitch_angle() / 2.0 - 1e-9
    }

    /// 参数自检（错误信息直接给用户看）。
    pub fn validate(&self) -> Result<(), String> {
        if !(self.m.is_finite() && self.m > 0.0) {
            return Err(crate::i18n::t_fmt("cmd.invol.err.m_positive", &[("v0", &trim(self.m))]));
        }
        if !(3..=1000).contains(&self.z) {
            return Err(crate::i18n::t_fmt("cmd.invol.err.z_range", &[("v0", &self.z.to_string())]));
        }
        if !(self.alpha_deg.is_finite() && self.alpha_deg > 10.0 && self.alpha_deg < 50.0) {
            return Err(crate::i18n::t_fmt("cmd.invol.err.alpha_range", &[("v0", &trim(self.alpha_deg))]));
        }
        if !(self.ha_star.is_finite() && self.ha_star > 0.0) {
            return Err(crate::i18n::t_fmt("cmd.invol.err.ha_positive", &[("v0", &trim(self.ha_star))]));
        }
        if !(self.hf_star.is_finite() && self.hf_star > 0.0) {
            return Err(crate::i18n::t_fmt("cmd.invol.err.hf_positive", &[("v0", &trim(self.hf_star))]));
        }
        if !(self.rho_star.is_finite() && self.rho_star >= 0.0) {
            return Err(crate::i18n::t_fmt("cmd.invol.err.rho_negative", &[("v0", &trim(self.rho_star))]));
        }
        if !(self.c_f_star.is_finite() && self.c_f_star >= 0.0) {
            return Err(crate::i18n::t_fmt("cmd.invol.err.cf_negative", &[("v0", &trim(self.c_f_star))]));
        }
        if !self.x.is_finite() {
            return Err(crate::i18n::t_fmt("cmd.invol.err.x_finite", &[("v0", &trim(self.x))]));
        }
        if self.std == SplineStd::DIN && !self.x_in_range() {
            return Err(crate::i18n::t_fmt("cmd.invol.err.din_x_range", &[("v0", &trim(self.x))]));
        }
        if self.std == SplineStd::ANSI {
            let p = self.ansi_p();
            if !(p.is_finite() && p > 0.0) {
                return Err(crate::i18n::t_fmt("cmd.invol.pitch.err.p_positive", &[("p", &trim(p))]));
            }
            // 系列校验：错误把 17 项按 A/B 形式列出（不再只报范围端点）。
            ansi_pitch_series_check(p)?;
            let col = match self.ansi_column() {
                Some(c) => c,
                None => {
                    return Err(crate::i18n::t_fmt("cmd.invol.err.ansi_col_unknown", &[("v0", &self.profile)]));
                }
            };
            if self.x.abs() > 1e-12 {
                return Err(crate::i18n::t_fmt("cmd.invol.err.ansi_no_x", &[("v0", &trim(self.x))]));
            }
            // Table 2 每列各自的适用径节范围（用户定案 A）；原 45° 下限特例已并入。
            ansi_column_pitch_check(col, p)?;
        }
        if self.df() <= 1e-9 {
            return Err(crate::i18n::t_fmt("cmd.invol.err.df_nonpositive", &[("v0", &trim(self.df()))]));
        }
        if self.da() <= self.df() + 1e-9 {
            return Err(crate::i18n::t_fmt("cmd.invol.err.da_not_gt_df", &[("v0", &trim(self.da())), ("v1", &trim(self.df()))]));
        }
        // 内花键：外侧齿根 > 里侧齿顶；两条轮廓都必须是正数。
        if self.internal {
            if self.internal_minor_dia() <= 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.invol.err.internal_minor_nonpositive", &[("v0", &trim(self.internal_minor_dia()))]));
            }
            if self.internal_major_dia() <= self.internal_minor_dia() + 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.invol.err.internal_major_le_minor", &[("v0", &trim(self.internal_major_dia())), ("v1", &trim(self.internal_minor_dia()))]));
            }
        }
        Ok(())
    }

    /// 规格文本（块名/明细表用）；ANSI 用 `P/Ps`（径节）标识，不用模数/`d_B`。
    pub fn spec(&self) -> String {
        let head = if self.internal { crate::i18n::t_static("cmd.invol.spec.internal_head") } else { "" };
        let mut s = if self.std == SplineStd::ANSI {
            let p = self.ansi_p();
            // P/Ps 保留标准原值（照 DP 先例）；附录圆 mm（内部统一 mm，避免误读）。
            format!(
                "{head}{} {} P{}/Ps{} N{}{}",
                self.std.label(),
                self.profile,
                trim(p),
                trim(2.0 * p),
                self.z,
                crate::i18n::t_fmt(
                    "cmd.invol.spec.ansi_pitch_circle",
                    &[("v0", &trim(self.d()))]
                )
            )
        } else {
            format!(
                "{head}{} {} m{} z{}",
                self.std.label(),
                self.profile,
                trim(self.m),
                self.z
            )
        };
        if self.x.abs() > 1e-12 {
            s.push_str(&format!(" x{}", trim(self.x)));
        }
        if self.std == SplineStd::NF {
            if let Some(a) = self.a {
                s.push_str(&format!(" A{}", trim(a)));
            }
        } else if let Some(d_b) = self.d_b {
            s.push_str(&format!(" d_B{}", trim(d_b)));
        }
        s
    }

    // ── 端视图（真实渐开线齿廓）──

    /// 渐开线上一点：齿中心线角 `center_rad`，`sign=±1` 取两侧，`radius` 处。
    fn flank_point(&self, radius: f64, center_rad: f64, sign: f64) -> [f64; 2] {
        self.flank_point_w(radius, center_rad, sign, self.s())
    }

    /// 同 [`InvolParams::flank_point`]，但用分度圆圆周宽 `w` 定半角（ANSI 内花键齿槽用 `Sv min`）。
    fn flank_point_w(&self, radius: f64, center_rad: f64, sign: f64, w: f64) -> [f64; 2] {
        let th = center_rad + sign * self.half_angle_at(radius, w);
        [radius * th.cos(), radius * th.sin()]
    }

    /// 一条齿廓（含基圆高于小径时的径向直线），从齿根端到齿顶端（`sign=+1` 侧）。
    fn push_flank(&self, out: &mut Vec<EntityType>, center_rad: f64, sign: f64) {
        let r0 = self.r_involute_start();
        let r1 = self.r_involute_end();
        let rf = self.df() / 2.0;
        let rb = self.db() / 2.0;
        // ANSI 渐开线从 form diameter `DFe` 起（可能高于齿根/基圆）：径向直线接到渐开线起点。
        // 齿根过渡圆角按 `rho_f` 导出数值；视图暂以径向直线近似（无标准数值依据，见 notes）。
        let ansi_gap = self.std == SplineStd::ANSI && r0 > rf + 1e-9;
        let gap_r = if ansi_gap { r0 } else { rb };
        if ansi_gap || rb > rf + 1e-9 {
            let th = center_rad + sign * self.half_tooth_angle(gap_r);
            out.push(line(
                [rf * th.cos(), rf * th.sin()],
                [gap_r * th.cos(), gap_r * th.sin()],
                LAYER_MAIN,
            ));
        }
        let n = INVOLUTE_SEGMENTS;
        for i in 0..n {
            let ra = r0 + (r1 - r0) * i as f64 / n as f64;
            let rb2 = r0 + (r1 - r0) * (i + 1) as f64 / n as f64;
            out.push(line(
                self.flank_point(ra, center_rad, sign),
                self.flank_point(rb2, center_rad, sign),
                LAYER_MAIN,
            ));
        }
    }

    /// 端视图：真实渐开线齿廓 + 齿顶/齿根弧 + 分度圆（`3中心线层`）+ 十字中心线。
    ///
    /// 相位：齿槽心线 `0°+k·360°/z`，齿心线 `180°/z+k·360°/z`。
    /// 内花键（[`InvolParams::internal`]）走 [`InvolParams::front_view_internal`]。
    /// `n` = 视图比例：十字中心线长度 = [`crate::gear::centerline_len`]`(特征直径, n)`
    /// （与齿轮常规端视 / `OCSMCENTERLINE` 同一函数）。
    pub fn front_view(&self, n: f64) -> Result<Vec<EntityType>, String> {
        self.validate()?;
        if self.internal {
            return self.front_view_internal(n);
        }
        if self.tooth_tip_crossed() {
            let psi_deg = format!(
                "{:.4}",
                self.half_tooth_angle(self.da() / 2.0).to_degrees()
            );
            return Err(crate::i18n::t_fmt(
                "cmd.invol.err.tooth_tip_crossed",
                &[
                    ("v0", self.std.label()),
                    ("v1", self.profile),
                    ("v2", &psi_deg),
                ],
            ));
        }
        if self.root_space_crossed() {
            let psi_deg = format!(
                "{:.4}",
                self.half_tooth_angle(self.r_involute_start()).to_degrees()
            );
            let pitch_deg = format!("{:.4}", (self.pitch_angle() / 2.0).to_degrees());
            return Err(crate::i18n::t_fmt(
                "cmd.invol.err.root_space_crossed",
                &[
                    ("v0", self.std.label()),
                    ("v1", self.profile),
                    ("v2", &psi_deg),
                    ("v3", &pitch_deg),
                ],
            ));
        }
        let pitch = self.pitch_angle();
        let ra = self.da() / 2.0;
        let rf = self.df() / 2.0;
        let r0 = self.r_involute_start();
        let psi_root = self.half_tooth_angle(r0);
        let psi_tip = self.half_tooth_angle(ra);
        let mut out = Vec::with_capacity(self.z as usize * (2 * INVOLUTE_SEGMENTS + 2) + 2);
        for k in 0..self.z {
            // 齿心线 180°/z + k·360°/z
            let c = pitch * (k as f64 + 0.5);
            self.push_flank(&mut out, c, 1.0);
            out.push(arc(
                [0.0, 0.0],
                ra,
                (c - psi_tip).to_degrees(),
                (c + psi_tip).to_degrees(),
                LAYER_MAIN,
            ));
            self.push_flank(&mut out, c, -1.0);
            // 齿槽弧：从本齿 +1 侧齿根端到下一齿 −1 侧齿根端（中心 = 齿槽心线 0°+k·360°/z）
            out.push(arc(
                [0.0, 0.0],
                rf,
                (c + psi_root).to_degrees(),
                (c + pitch - psi_root).to_degrees(),
                LAYER_MAIN,
            ));
        }
        // 分度圆（`3中心线层`）：与外齿轮常规端视同口径。
        // GB/DIN/NF 用分度圆直径 `d = m·z`；ANSI 的 `D = N/P` 在引擎内部就是 `d()`
        // （构造时 `m = 25.4/P`），同一出口 —— 不另写第二套半径口径。
        out.push(circle([0.0, 0.0], self.d() / 2.0, LAYER_CENTER));
        // 十字中心线：长度 = 大径 da + 6n（走齿轮同一函数 `centerline_len`）。
        out.extend(crate::gear::cross_centerlines([0.0, 0.0], self.da(), n));
        Ok(out)
    }

    /// 内花键单侧齿廓：从里侧齿顶（或基圆）到外侧齿根。
    /// 起点/降级口径统一走 [`InvolParams::internal_involute_band`]（与端视图/计算书同源）。
    fn push_flank_internal(
        &self,
        out: &mut Vec<EntityType>,
        center_rad: f64,
        sign: f64,
        r_tip: f64,
        r_root: f64,
    ) {
        // 渐开线起点：ANSI 用 form diameter `DFi`（Table 2）；其余体系用 max(齿顶, 基圆)。
        let (r0, _) = self.internal_involute_band();
        let w = if self.std == SplineStd::ANSI {
            self.ansi_sv_min()
        } else {
            self.s()
        };
        // 起始圆高于齿顶：先沿半径从齿顶圆接到起始圆（渐开线从起始圆才开始）。
        if r0 > r_tip + 1e-9 {
            let th = center_rad + sign * self.internal_half_space_angle(r0);
            out.push(line(
                [r_tip * th.cos(), r_tip * th.sin()],
                [r0 * th.cos(), r0 * th.sin()],
                LAYER_MAIN,
            ));
        }
        let n = INVOLUTE_SEGMENTS;
        for i in 0..n {
            let ra = r0 + (r_root - r0) * i as f64 / n as f64;
            let rb2 = r0 + (r_root - r0) * (i + 1) as f64 / n as f64;
            out.push(line(
                self.flank_point_w(ra, center_rad, sign, w),
                self.flank_point_w(rb2, center_rad, sign, w),
                LAYER_MAIN,
            ));
        }
    }

    /// **内花键端视图**（材料在外侧、齿朝内；与 `gear.rs` 内齿轮端视同口径）：
    ///
    /// * 渐开线与同参数外花键**同一条**：内花键的**齿槽** = 外花键的**齿形**，
    ///   凹槽心线与外花键齿心线同相（`(k+0.5)·360°/z`），ψ(R) 同式；
    /// * 外侧齿槽底弧在 `internal_major_dia()/2`、里侧齿顶弧在 `internal_minor_dia()/2`；
    /// * 齿顶圆低于基圆时，齿廓到基圆后径向直线收到齿顶（同 `gear.rs` 内齿轮口径）。
    fn front_view_internal(&self, n: f64) -> Result<Vec<EntityType>, String> {
        let r_root = self.internal_root_radius();
        let r_tip = self.internal_tip_radius();
        // 降级：材料带 [r_tip, r_root] 内没有可用渐开线 ——
        //   * 非 ANSI：基圆已到/超过外侧齿根（rb ≥ D_ei/2）；
        //   * ANSI：form 直径 DFi ≥ Dri（P=128 等细径节 + cF 下夹取时，公式值可高于齿根）。
        // 此时把渐开线起点夹到外侧齿根，整条齿廓退化为从齿顶到齿根的径向直线
        // （ψ 按齿根处取值，与齿根弧端点连续；同 `gear.rs` 内齿轮「基圆以下用径向直线」口径）。
        let (r_inner, radial_only) = self.internal_involute_band();
        let pitch = self.pitch_angle();
        let pitch_half = pitch / 2.0;
        let psi_inner = self.internal_half_space_angle(r_inner);
        if psi_inner >= pitch_half - 1e-9 {
            let psi_inner_deg = format!("{:.4}", psi_inner.to_degrees());
            let pitch_half_deg = format!("{:.4}", pitch_half.to_degrees());
            return Err(crate::i18n::t_fmt(
                "cmd.invol.err.internal_space_crossed",
                &[
                    ("v0", self.std.label()),
                    ("v1", self.profile),
                    ("v2", &psi_inner_deg),
                    ("v3", &pitch_half_deg),
                ],
            ));
        }
        let psi_root = self.internal_half_space_angle(r_root);
        if psi_root <= 1e-9 {
            let psi_root_deg = format!("{:.4}", psi_root.to_degrees());
            return Err(crate::i18n::t_fmt(
                "cmd.invol.err.internal_root_crossed",
                &[
                    ("v0", self.std.label()),
                    ("v1", self.profile),
                    ("v2", &psi_root_deg),
                ],
            ));
        }
        let mut out = Vec::with_capacity(self.z as usize * (2 * INVOLUTE_SEGMENTS + 4) + 2);
        for k in 0..self.z {
            // 齿槽心线（与同参数外花键的齿心线同相：同一条渐开线）。
            let c = pitch * (k as f64 + 0.5);
            if radial_only {
                // 无渐开线段：两侧各一条径向直线（齿顶 → 外侧齿根）。
                for sign in [1.0, -1.0] {
                    let th = c + sign * psi_root;
                    out.push(line(
                        [r_tip * th.cos(), r_tip * th.sin()],
                        [r_root * th.cos(), r_root * th.sin()],
                        LAYER_MAIN,
                    ));
                }
            } else {
                self.push_flank_internal(&mut out, c, 1.0, r_tip, r_root);
                self.push_flank_internal(&mut out, c, -1.0, r_tip, r_root);
            }
            // 齿槽底弧（外侧大径）：c ± ψ(r_root)
            out.push(arc(
                [0.0, 0.0],
                r_root,
                (c - psi_root).to_degrees(),
                (c + psi_root).to_degrees(),
                LAYER_MAIN,
            ));
            // 齿顶弧（里侧小径）：中心 = c + 齿距/2，半角 = 半齿距 − ψ(inner)
            let tooth_c = c + pitch_half;
            let psi_tip = pitch_half - psi_inner;
            out.push(arc(
                [0.0, 0.0],
                r_tip,
                (tooth_c - psi_tip).to_degrees(),
                (tooth_c + psi_tip).to_degrees(),
                LAYER_MAIN,
            ));
        }
        // 分度圆（`3中心线层`）：与外花键端视同口径 —— 半径 = 分度圆半径 d/2
        // （GB `m·z`、DIN/NF 各自 `d()`、ANSI `D=N/P` 在引擎内部就是 `d()`）。
        // 内齿轮仍按模板不画；矩形花键模板没有；本件只补「内花键端视」这一处。
        out.push(circle([0.0, 0.0], self.d() / 2.0, LAYER_CENTER));
        // 十字中心线：长度 = 外侧齿根直径 D_ei + 6n（与内齿轮用齿根圆同口径，走齿轮 `centerline_len`）。
        out.extend(crate::gear::cross_centerlines(
            [0.0, 0.0],
            self.internal_major_dia(),
            n,
        ));
        Ok(out)
    }

    // ── 侧视图（矩形 + 小径线）──

    /// 外花键侧视/剖视轮廓：矩形 `L × 大径 da` + 小径线 `df`（落 `minor_layer`）。
    ///
    /// 内花键的可用视图与内齿轮一致（剖视 + 端视，**无侧视**）；本方法只服务外花键，
    /// 内花键请走 `gear.rs` 的齿圈剖视模板 `internal_bore_section`。
    fn side_view_on(&self, len: f64, minor_layer: &str) -> Vec<EntityType> {
        let (ra, rf) = (self.da() / 2.0, self.df() / 2.0);
        let d2 = self.d() / 2.0;
        let mut out = vec![
            line([0.0, -ra], [0.0, ra], LAYER_MAIN),
            line([len, -ra], [len, ra], LAYER_MAIN),
            line([0.0, ra], [len, ra], LAYER_MAIN),
            line([0.0, -ra], [len, -ra], LAYER_MAIN),
            line([0.0, -rf], [len, -rf], minor_layer),
            line([0.0, rf], [len, rf], minor_layer),
            line([-3.0, 0.0], [len + 3.0, 0.0], LAYER_CENTER),
        ];
        // 分度圆两条（点划线）：与齿轮侧视图同口径 —— 长度 = 该视图轮廓 `[0, len]`。
        // 外花键侧视/剖视都走这里；内花键剖视走 `gear.rs::internal_bore_section`（已含）。
        out.push(line([0.0, d2], [len, d2], LAYER_CENTER));
        out.push(line([0.0, -d2], [len, -d2], LAYER_CENTER));
        out
    }

    /// 常规侧视图（小径线 `2细线层`）。
    ///
    /// **内花键无侧视图**（用户定案：同内齿轮）→ 明确报错，不出旧的「两条矩形轮廓」草案。
    pub fn side_view(&self, len: f64) -> Result<Vec<EntityType>, String> {
        if self.internal {
            return Err(crate::gear::internal_spline_no_side_view_msg().to_string());
        }
        Ok(self.side_view_on(len, LAYER_THIN))
    }

    /// 外花键侧剖轮廓（小径线 `1轮廓实线层`；不含剖面线，由调用方补）。
    ///
    /// 内花键剖视图走 `gear.rs` 的内齿轮剖视模板（`internal_bore_section`：端面/齿顶线/
    /// 齿根线/内孔壁/孔口倒角 + 分度线/轴线，不打剖面线）—— 引擎侧不再出内花键剖视草案。
    pub fn section_view(&self, len: f64) -> Result<Vec<EntityType>, String> {
        if self.internal {
            return Err(
                crate::i18n::t_static("cmd.invol.section.internal_note")
                    .to_string(),
            );
        }
        Ok(self.side_view_on(len, LAYER_MAIN))
    }

    // ── 收尾 ──

    /// 滚刀半径 `R = de/2`。
    pub fn hob_radius(&self, de: f64) -> f64 {
        de / 2.0
    }

    /// 收尾长度 `l = √(h(2R−h))`（`h=(da−df)/2`、`R=de/2`；`de` 须大于大径）。
    pub fn runout_length(&self, de: f64) -> Result<f64, String> {
        if !(de.is_finite() && de > 0.0) {
            return Err(crate::i18n::t_fmt(
                "cmd.invol.runout.de_pos",
                &[("de", &trim(de))],
            ));
        }
        if de <= self.da() + 1e-9 {
            return Err(crate::i18n::t_fmt("cmd.invol.runout.de_gt_da", &[("v0", &trim(de)), ("v1", &trim(self.da()))]));
        }
        let h = self.tooth_depth();
        let r = self.hob_radius(de);
        Ok((h * (2.0 * r - h)).max(0.0).sqrt())
    }

    /// 收尾弧与大径交点在弧圆心中的极角（度）；上弧从 270° 转到 `360°−a`，下弧从 `a` 转到 90°。
    pub fn runout_end_angle(&self, de: f64) -> Result<f64, String> {
        let h = self.tooth_depth();
        let r = self.hob_radius(de);
        let l = self.runout_length(de)?;
        Ok((r - h).atan2(l).to_degrees())
    }

    /// 收尾弧圆心（`x0` = 满齿段右端，`sign=±1` 取上/下；与小径相切于 `(x0, ±df/2)`）。
    pub fn runout_center(&self, x0: f64, de: f64, sign: f64) -> Result<[f64; 2], String> {
        let r = self.hob_radius(de);
        self.runout_length(de)?;
        Ok([x0, sign * (self.df() / 2.0 + r)])
    }

    /// 收尾图元（2 条弧 + 终点竖线，落 `layer`；`x0` = 满齿段右端）。
    pub fn runout_arcs(&self, x0: f64, de: f64, layer: &str) -> Result<Vec<EntityType>, String> {
        let r = self.hob_radius(de);
        let l = self.runout_length(de)?;
        let a = self.runout_end_angle(de)?;
        let rf = self.df() / 2.0;
        let ra = self.da() / 2.0;
        Ok(vec![
            arc([x0, rf + r], r, 270.0, 360.0 - a, layer),
            arc([x0, -(rf + r)], r, a, 90.0, layer),
            line([x0 + l, -ra], [x0 + l, ra], layer),
        ])
    }
}

/// 渐开线展角函数 `inv α = tanα − α`。
fn inv(a: f64) -> f64 {
    a.tan() - a
}

// ─────────────────── 计算书（纯数据；命令入口 `OCSMGEAR … report` / `INVOLSPLINE … report`） ───────────────────

/// 计算书步骤表一行：`| # | 步骤 | 公式（符号含义） | 代入 | 结果 | 依据来源 |`。
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

/// ANSI Dre 的径节/列分支说明（计算书里必须注明走了哪一段）。
fn ansi_dre_branch(e: &InvolParams) -> String {
    match e.ansi_column() {
        Some(AnsiColumn::A30FlatSide) | Some(AnsiColumn::B30FlatMajor) => {
            crate::i18n::t_static("cmd.invol.report.dre.a").to_string()
        }
        Some(AnsiColumn::C30FilletSide) => {
            if e.ansi_p() > 12.0 + 1e-9 {
                crate::i18n::t_static("cmd.invol.report.dre.c_gt12").to_string()
            } else {
                crate::i18n::t_static("cmd.invol.report.dre.c_le12").to_string()
            }
        }
        Some(AnsiColumn::D375FilletSide) => crate::i18n::t_static("cmd.invol.report.dre.d").to_string(),
        Some(AnsiColumn::E45FilletSide) => crate::i18n::t_static("cmd.invol.report.dre.e").to_string(),
        None => crate::i18n::t_static("cmd.invol.report.dre.none").to_string(),
    }
}

/// 单位口径说明（计算书开头必须写清）。
fn report_unit_note(std: SplineStd) -> &'static str {
    match std {
        SplineStd::ANSI => {
            crate::i18n::t_static("cmd.invol.report.units_note.ansi")
        }
        SplineStd::DIN => {
            crate::i18n::t_static("cmd.invol.report.units_note.din")
        }
        SplineStd::NF => crate::i18n::t_static("cmd.invol.report.units_note.nf"),
        SplineStd::GB => crate::i18n::t_static("cmd.invol.report.units_note.gb"),
    }
}

/// 体系预设来源说明。
fn report_preset_source(std: SplineStd) -> &'static str {
    match std {
        SplineStd::GB => {
            crate::i18n::t_static("cmd.invol.report.src.preset.gb")
        }
        SplineStd::DIN => {
            crate::i18n::t_static("cmd.invol.report.src.preset.din")
        }
        SplineStd::NF => crate::i18n::t_static("cmd.invol.report.src.preset.nf"),
        SplineStd::ANSI => crate::i18n::t_static("cmd.invol.report.src.preset.ansi"),
    }
}

/// **跨测齿数 k 与公法线长度 W**（渐开线跨距公式，**全仓唯一实现**）。
///
/// `k = round(z·α/180° + 0.5)`（α 为压力角，单位度）；
/// `W = m·cosα·[(k−0.5)·π + z·invα] + 2·x·m·sinα`（`invα = tanα − α`）。
///
/// 与 GB/T 3478.6 式(11) 是**同一条渐开线跨距公式**（GB 另有 `esv − (T+λ)` 修正项，
/// 不在本函数）；ANSI `Wn/Kn`、NF `W/E` 的检验量推导与齿轮 `GearParams::span_measurement`
/// 都调用本函数（不许两处各算一遍）。
///
/// 口径说明：这是**渐开线几何名义值**；ANSI B92.1 原标准检验表（公法线跨测表）本仓未收，
/// 报告里必须标注「几何推导」而不是冒充原表值。
pub fn involute_span(m: f64, z: u32, alpha_deg: f64, x: f64) -> (u32, f64) {
    let a = alpha_deg.to_radians();
    let k = (z as f64 * alpha_deg / 180.0 + 0.5).round() as u32;
    let k = k.max(1);
    let inv = a.tan() - a;
    let w = m * a.cos() * ((k as f64 - 0.5) * std::f64::consts::PI + z as f64 * inv)
        + 2.0 * x * m * a.sin();
    (k, w)
}

/// **渐开线花键计算书**（Markdown，纯数据、无 IO）：
/// 输入参数（含单位口径）→ 逐步计算（公式 + 代入数值 + 结果 + 依据来源）→
/// 派生几何 → 检验尺寸（DIN：M1/M2/D_M/k/W_k + 查表页/source；
/// NF：K/E/U/F/F1/V/V1/G/G1 + p26 设计尺寸 + p29/p31–p35 偏差行 + 查表页/source/表名）
/// → 数据来源与校验（预设/候选/基准直径来源、恒等式残差、DIN 检验表全表对照）。
///
/// `origin` = 解析时 [`resolve_spline`] 给出的基准直径来源（查表行/公式/推导/纠偏）；
/// DIN 的检验尺寸由本函数按同一 [`inspection_query`] 入口查询，不另算一套几何；
/// NF 按 [`nf_check_by_a_m`] 查检查表原样输出（可疑格保留在 `flags`）。
/// 命令入口：`OCSMGEAR … report` / `OCSMSHAFT INVOLSPLINE … report`。
pub fn build_report(params: &InvolParams, origin: Option<&D_bOrigin>, h: f64) -> String {
    build_report_inner(params, origin, Some(h))
}

/// **卡片计算书入口**：智能卡片没有「有效长度」输入 → 不打印该行；
/// 其余与 [`build_report`] 完全同一实现（同源，不许另算一套）。
pub fn build_report_card(params: &InvolParams, origin: Option<&D_bOrigin>) -> String {
    build_report_inner(params, origin, None)
}

fn build_report_inner(params: &InvolParams, origin: Option<&D_bOrigin>, h: Option<f64>) -> String {
    let e = params;
    let std = e.std;
    let mut md = String::new();
    md.push_str(crate::i18n::t_static("cmd.invol.report.title"));
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.std", &[("v0", &std.label()), ("v1", &std.code())]));
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.mode", &[("v0", &if e.internal {
            crate::i18n::t("cmd.invol.report.mode_internal")
        } else {
            crate::i18n::t("cmd.invol.report.mode_external")
        })]));
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.profile", &[("v0", &e.profile), ("v1", &preset_code(std, e.profile).unwrap_or("-"))]));
    if let Some(h) = h {
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.len", &[("v0", &trim(h))]));
    }
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.units", &[("v0", &report_unit_note(std))]));

    // ── 1. 输入参数 ──
    md.push_str(crate::i18n::t_static("cmd.invol.report.h_inputs"));
    md.push_str(crate::i18n::t_static("cmd.invol.report.t_inputs"));
    if std == SplineStd::ANSI {
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_pitch", &[("v0", &trim(e.ansi_p())), ("v1", &trim(2.0 * e.ansi_p()))]));
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_m_conv", &[("v0", &trim(e.m))]));
    } else {
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_m", &[("v0", &trim(e.m))]));
    }
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_z", &[("v0", &e.z.to_string())]));
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_x", &[("v0", &trim(e.x))]));
    if std == SplineStd::DIN {
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_db_input", &[("v0", &e.d_b.map(trim).unwrap_or_else(|| "—".into()))]));
    }
    if std == SplineStd::NF {
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_a", &[("v0", &e.a.map(trim).unwrap_or_else(|| "—".into()))]));
    }
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_profile", &[("v0", &trim(e.alpha_deg)), ("v1", &trim(e.ha_star)), ("v2", &trim(e.hf_star)), ("v3", &trim(e.rho_star)), ("v4", &trim(e.c_f_star))]));
    md.push('\n');

    // ── 2. 逐步计算 ──
    md.push_str(crate::i18n::t_static("cmd.invol.report.h_steps"));
    md.push_str(crate::i18n::t_static("cmd.invol.report.t_steps"));
    md.push_str("|---|---|---|---|---|---|\n");
    let mut n = 0usize;
    let mut steps = String::new();
    {
        let mut s = |name: &str, formula: &str, subst: &str, result: &str, source: &str| {
            steps.push_str(&report_row(&mut n, name, formula, subst, result, source));
        };
        s(
            crate::i18n::t_static("cmd.invol.report.step.preset"),
            "α / ha* / hf* / ρf* / cF*",
            "—",
            &format!(
                "α={}°，ha*={}，hf*={}，ρf*={}，cF*={}",
                trim(e.alpha_deg),
                trim(e.ha_star),
                trim(e.hf_star),
                trim(e.rho_star),
                trim(e.c_f_star)
            ),
            report_preset_source(std),
        );
        if std == SplineStd::ANSI {
            s(
                crate::i18n::t_static("cmd.invol.report.step.ansi_conv"),
                crate::i18n::t_static("cmd.invol.report.formula.ansi_conv"),
                &format!("25.4 / {}", trim(e.ansi_p())),
                &format!("{} mm", trim(e.m)),
                crate::i18n::t_static("cmd.invol.report.source.ansi_conv"),
            );
        }
        match std {
            SplineStd::DIN => {
                let d_b = e.d_b.unwrap_or_else(|| d_b_from_x(e.m, e.z, e.x));
                s(
                    crate::i18n::t_static("cmd.invol.report.step.din_db"),
                    crate::i18n::t_static("cmd.invol.report.formula.din_db"),
                    &format!("m={}，z={}，x₁={}", trim(e.m), e.z, trim(e.x)),
                    &format!("d_B = {} mm", trim(d_b)),
                    &crate::i18n::t_fmt("cmd.invol.report.source.din_db", &[("v0", &origin.map(|o| o.note()).unwrap_or_default())]),
                );
            }
            SplineStd::NF => {
                let a = e.a.unwrap_or_else(|| a_from_x(e.m, e.z, e.x));
                s(
                    crate::i18n::t_static("cmd.invol.report.step.nf_a"),
                    crate::i18n::t_static("cmd.invol.report.formula.nf_a"),
                    &format!("m={}，N={}，x={}", trim(e.m), e.z, trim(e.x)),
                    &format!("A = {} mm", trim(a)),
                    &crate::i18n::t_fmt("cmd.invol.report.source.nf_a", &[("v0", &origin.map(|o| o.note()).unwrap_or_default())]),
                );
            }
            _ => {}
        }
        s(
            crate::i18n::t_static("cmd.invol.report.step.d"),
            crate::i18n::t_static("cmd.invol.report.formula.d"),
            &format!("{} × {}", trim(e.m), e.z),
            &format!("{} mm", trim(e.d())),
            match std {
                SplineStd::GB => crate::i18n::t_static("cmd.invol.report.source.d.gb"),
                SplineStd::DIN => crate::i18n::t_static("cmd.invol.report.source.d.din"),
                SplineStd::NF => crate::i18n::t_static("cmd.invol.report.source.d.nf"),
                SplineStd::ANSI => crate::i18n::t_static("cmd.invol.report.source.d.ansi"),
            },
        );
        if e.x.abs() > 1e-12 {
            s(
                crate::i18n::t_static("cmd.invol.report.step.d_eff"),
                crate::i18n::t_static("cmd.invol.report.formula.d_eff"),
                &format!("{} + 2×{}×{}", trim(e.d()), trim(e.x), trim(e.m)),
                &format!("{} mm", trim(e.d_eff())),
                match std {
                    SplineStd::DIN => crate::i18n::t_static("cmd.invol.report.source.d_eff.din"),
                    SplineStd::NF => crate::i18n::t_static("cmd.invol.report.source.d_eff.nf"),
                    _ => crate::i18n::t_static("cmd.invol.report.source.d_eff.other"),
                },
            );
        }
        s(
            crate::i18n::t_static("cmd.invol.report.step.db"),
            crate::i18n::t_static("cmd.invol.report.formula.db"),
            &format!("{} × cos {}°", trim(e.d()), trim(e.alpha_deg)),
            &format!("{} mm", trim(e.db())),
            match std {
                SplineStd::GB => crate::i18n::t_static("cmd.invol.report.source.db.gb"),
                SplineStd::DIN => crate::i18n::t_static("cmd.invol.report.source.db.din"),
                SplineStd::NF => crate::i18n::t_static("cmd.invol.report.source.db.nf"),
                SplineStd::ANSI => crate::i18n::t_static("cmd.invol.report.source.db.ansi"),
            },
        );
        s(
            crate::i18n::t_static("cmd.invol.report.step.da"),
            match std {
                SplineStd::DIN => crate::i18n::t_static("cmd.invol.report.formula.da.din"),
                SplineStd::NF => crate::i18n::t_static("cmd.invol.report.formula.da.nf"),
                SplineStd::ANSI => crate::i18n::t_static("cmd.invol.report.formula.da.ansi"),
                SplineStd::GB => crate::i18n::t_static("cmd.invol.report.formula.da.gb"),
            },
            &format!("{} + 2×{}×{}", trim(e.d_eff()), trim(e.ha_star), trim(e.m)),
            &format!("{} mm", trim(e.da())),
            match std {
                SplineStd::GB => {
                    crate::i18n::t_static("cmd.invol.report.source.da.gb")
                }
                SplineStd::DIN => crate::i18n::t_static("cmd.invol.report.source.da.din"),
                SplineStd::NF => crate::i18n::t_static("cmd.invol.report.source.da.nf"),
                SplineStd::ANSI => crate::i18n::t_static("cmd.invol.report.source.da.ansi"),
            },
        );
        s(
            crate::i18n::t_static("cmd.invol.report.step.df"),
            match std {
                SplineStd::DIN => crate::i18n::t_static("cmd.invol.report.formula.df.din"),
                SplineStd::NF => crate::i18n::t_static("cmd.invol.report.formula.df.nf"),
                SplineStd::ANSI => crate::i18n::t_static("cmd.invol.report.formula.df.ansi"),
                SplineStd::GB => crate::i18n::t_static("cmd.invol.report.formula.df.gb"),
            },
            &format!("{} − 2×{}×{}", trim(e.d_eff()), trim(e.hf_star), trim(e.m)),
            &format!("{} mm", trim(e.df())),
            match std {
                SplineStd::GB => {
                    crate::i18n::t_static("cmd.invol.report.source.df.gb")
                }
                SplineStd::DIN => crate::i18n::t_static("cmd.invol.report.source.df.din"),
                SplineStd::NF => crate::i18n::t_static("cmd.invol.report.source.df.nf"),
                SplineStd::ANSI => crate::i18n::t_static("cmd.invol.report.source.df.ansi"),
            },
        );
        if std == SplineStd::ANSI {
            s(
                crate::i18n::t_static("cmd.invol.report.step.dre"),
                crate::i18n::t_static("cmd.invol.report.formula.dre"),
                "—",
                &ansi_dre_branch(e),
                crate::i18n::t_static("cmd.invol.report.source.dre"),
            );
        }
        s(
            crate::i18n::t_static("cmd.invol.report.step.s"),
            match std {
                SplineStd::ANSI => crate::i18n::t_static("cmd.invol.report.formula.s.ansi"),
                _ => crate::i18n::t_static("cmd.invol.report.formula.s.gb_din"),
            },
            &match std {
                // ANSI 的口径是 `t = p − Sv min`，不要照搬 GB/DIN 的齿厚式。
                SplineStd::ANSI => format!("π×{} − {}", trim(e.m), trim(e.ansi_sv_min())),
                _ => format!(
                    "{}×π/2 + 2×{}×{}×tan{}°",
                    trim(e.m),
                    trim(e.x),
                    trim(e.m),
                    trim(e.alpha_deg)
                ),
            },
            &format!("{} mm", trim(e.s())),
            match std {
                SplineStd::GB => crate::i18n::t_static("cmd.invol.report.source.s.gb"),
                SplineStd::DIN => crate::i18n::t_static("cmd.invol.report.source.s.din"),
                SplineStd::NF => "NF E22-141 p07",
                SplineStd::ANSI => {
                    crate::i18n::t_static("cmd.invol.report.source.s.ansi")
                }
            },
        );
        if e.internal {
            match std {
                SplineStd::GB => {
                    s(
                        crate::i18n::t_static("cmd.invol.report.step.int_major"),
                        crate::i18n::t_static("cmd.invol.report.formula.int_major.gb"),
                        &format!("{} + 2×{}×{}", trim(e.d_eff()), trim(e.hf_star), trim(e.m)),
                        &format!("{} mm", trim(e.internal_major_dia())),
                        crate::i18n::t_static("cmd.invol.report.source.int_major.gb"),
                    );
                    s(
                        crate::i18n::t_static("cmd.invol.report.step.int_minor"),
                        crate::i18n::t_static("cmd.invol.report.formula.int_minor.gb"),
                        &format!(
                            "db={}，d={}，α={}°，h_s={}m",
                            trim(e.db()),
                            trim(e.d()),
                            trim(e.alpha_deg),
                            trim(e.h_s_star)
                        ),
                        &format!(
                            "D_Fe max={}，D_ii={} mm",
                            trim(e.gb_form_dia_max()),
                            trim(e.internal_minor_dia())
                        ),
                        crate::i18n::t_static("cmd.invol.report.source.int_minor.gb"),
                    );
                }
                SplineStd::DIN => {
                    s(
                        crate::i18n::t_static("cmd.invol.report.step.int_root"),
                        crate::i18n::t_static("cmd.invol.report.formula.int_root.din"),
                        &format!("d_B = {}", trim(e.internal_major_dia())),
                        &format!("{} mm", trim(e.internal_major_dia())),
                        crate::i18n::t_static("cmd.invol.report.source.int_root.din"),
                    );
                    s(
                        crate::i18n::t_static("cmd.invol.report.step.int_tip"),
                        crate::i18n::t_static("cmd.invol.report.formula.int_tip.din"),
                        &format!("{} − 0.9×{}", trim(e.d_eff()), trim(e.m)),
                        &format!("{} mm", trim(e.internal_minor_dia())),
                        crate::i18n::t_static("cmd.invol.report.source.int_tip.din"),
                    );
                }
                SplineStd::NF => {
                    s(
                        crate::i18n::t_static("cmd.invol.report.step.int_major"),
                        crate::i18n::t_static("cmd.invol.report.formula.int_major.nf"),
                        &format!("A = {}", trim(e.internal_major_dia())),
                        &format!("{} mm", trim(e.internal_major_dia())),
                        "NF E22-141 p07",
                    );
                    s(
                        crate::i18n::t_static("cmd.invol.report.step.int_minor"),
                        crate::i18n::t_static("cmd.invol.report.formula.int_minor.nf"),
                        &format!("{} − 2×{}", trim(e.internal_major_dia()), trim(e.m)),
                        &format!("{} mm", trim(e.internal_minor_dia())),
                        crate::i18n::t_static("cmd.invol.report.source.int_minor.nf"),
                    );
                }
                SplineStd::ANSI => {
                    s(
                        crate::i18n::t_static("cmd.invol.report.step.int_major"),
                        &crate::i18n::t_fmt("cmd.invol.report.formula.int_major.ansi", &[("v0", &trim(e.ansi_dri_offset()))]),
                        &format!("(N+{}) / {}", trim(e.ansi_dri_offset()), trim(e.ansi_p())),
                        &format!("{} mm", trim(e.internal_major_dia())),
                        crate::i18n::t_static("cmd.invol.report.source.int_major.ansi"),
                    );
                    s(
                        crate::i18n::t_static("cmd.invol.report.step.int_minor"),
                        &crate::i18n::t_fmt("cmd.invol.report.formula.int_minor.ansi", &[("v0", &trim(e.ansi_di_offset()))]),
                        &format!("(N−{}) / {}", trim(e.ansi_di_offset()), trim(e.ansi_p())),
                        &format!("{} mm", trim(e.internal_minor_dia())),
                        crate::i18n::t_static("cmd.invol.report.source.int_minor.ansi"),
                    );
                }
            }
        }
        if std == SplineStd::ANSI {
            s(
                crate::i18n::t_static("cmd.invol.report.step.cf"),
                crate::i18n::t_static("cmd.invol.report.formula.cf"),
                &format!("D = {} mm", trim(e.d())),
                &format!("{} mm", trim(e.c_f())),
                crate::i18n::t_static("cmd.invol.report.source.cf"),
            );
            s(
                crate::i18n::t_static("cmd.invol.report.step.dfe"),
                crate::i18n::t_static("cmd.invol.report.formula.dfe"),
                &format!(
                    "d={}，cF={}",
                    trim(e.d()),
                    trim(e.c_f())
                ),
                &format!("{} mm", trim(e.ansi_form_dia_external())),
                crate::i18n::t_static("cmd.invol.report.source.dfe"),
            );
            s(
                crate::i18n::t_static("cmd.invol.report.step.dfi"),
                crate::i18n::t_static("cmd.invol.report.formula.dfi"),
                &format!("d={}，cF={}", trim(e.d()), trim(e.c_f())),
                &format!("{} mm", trim(e.ansi_form_dia_internal())),
                crate::i18n::t_static("cmd.invol.report.source.dfi"),
            );
        }
    }
    md.push_str(&steps);
    md.push('\n');

    // ── 3. 派生几何 ──
    md.push_str(crate::i18n::t_static("cmd.invol.report.h_derived"));
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_d", &[("v0", &trim(e.d()))]));
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_db", &[("v0", &trim(e.db()))]));
    if e.internal {
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_int_major", &[("v0", &trim(e.internal_major_dia()))]));
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_int_minor", &[("v0", &trim(e.internal_minor_dia()))]));
    } else {
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_da", &[("v0", &trim(e.da()))]));
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_df", &[("v0", &trim(e.df()))]));
    }
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_s", &[("v0", &trim(e.s()))]));
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_rho", &[("v0", &trim(e.rho_f()))]));
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_cf", &[("v0", &trim(e.c_f()))]));
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_clearance", &[("v0", &trim(e.clearance()))]));
    if e.internal {
        // 内花键：渐开线有效区间在 [max(D_ii, db[, DFi]), D_ei]（外侧齿根），不是外花键的 [max(df, db), da]。
        // 与 `front_view_internal` 同走 `internal_involute_band()`：若已到/超过外侧齿根则端视径向降级。
        let (r_in, radial_only) = e.internal_involute_band();
        let note = if radial_only {
            crate::i18n::t_static("cmd.invol.report.note_radial_fallback")
        } else {
            ""
        };
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_inv_start_int", &[("v0", &trim(2.0 * r_in)), ("v1", &note)]));
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_inv_end_int", &[("v0", &trim(e.internal_major_dia()))]));
    } else {
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_inv_start", &[("v0", &trim(e.d_involute_start()))]));
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_inv_end", &[("v0", &trim(e.d_involute_end()))]));
    }
    if let Some(d_b) = e.d_b {
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_db_val", &[("v0", &trim(d_b))]));
    }
    if let Some(a) = e.a {
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.row_a_val", &[("v0", &trim(a))]));
    }
    if std == SplineStd::ANSI {
        md.push_str(&format!(
            "| DFe / DFi | {} / {} mm |\n",
            trim(e.ansi_form_dia_external()),
            trim(e.ansi_form_dia_internal())
        ));
    }
    md.push('\n');

    // ── 4. 检验尺寸 ──
    md.push_str(crate::i18n::t_static("cmd.invol.report.h_checks"));
    if std == SplineStd::NF {
        let a = e.a.unwrap_or_else(|| a_from_x(e.m, e.z, e.x));
        let all = nf_check_by_a_m(a, e.m);
        let rows: Vec<NfCheckRow> = all
            .iter()
            .filter(|r| r.n.is_none() || r.n == Some(e.z))
            .cloned()
            .collect();
        if rows.is_empty() {
            if all.is_empty() {
                md.push_str(&crate::i18n::t_fmt("cmd.invol.report.nf_no_table", &[("v0", &trim(a)), ("v1", &trim(e.m)), ("v2", &e.z.to_string())]));
            } else {
                let mut ns: Vec<u32> = all.iter().filter_map(|r| r.n).collect();
                ns.sort_unstable();
                ns.dedup();
                md.push_str(&crate::i18n::t_fmt("cmd.invol.report.nf_n_miss", &[("v0", &trim(a)), ("v1", &trim(e.m)), ("v2", &e.z.to_string()), ("v3", &ns.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(&crate::i18n::t("cmd.invol.sep.list")))]));
            }
        } else {
            md.push_str(&crate::i18n::t_fmt("cmd.invol.report.nf_hit", &[("v0", &rows.len().to_string()), ("v1", &nf_check_rows().len().to_string())]));
            // 检查尺寸（p23–p25/p27；p27 另带 q1..q7 位置列）。
            for r in rows.iter().filter(|r| r.has_check_dims()) {
                md.push_str(&crate::i18n::t_fmt("cmd.invol.report.h_check_row", &[("v0", &r.page.to_string()), ("v1", &r.table_no.to_string()), ("v2", &r.source), ("v3", &r.section)]));
                md.push_str(crate::i18n::t_static("cmd.invol.report.t_qty"));
                let mut any = false;
                for (label, v) in [
                    (crate::i18n::t_static("cmd.invol.report.k_span"), r.k),
                    (crate::i18n::t_static("cmd.invol.report.e_kt"), r.e),
                    (crate::i18n::t_static("cmd.invol.report.u_ext"), r.u),
                    (crate::i18n::t_static("cmd.invol.report.f_ext"), r.f),
                    (crate::i18n::t_static("cmd.invol.report.f1_ext"), r.f1),
                    (crate::i18n::t_static("cmd.invol.report.v_int"), r.v),
                    (crate::i18n::t_static("cmd.invol.report.v1_int"), r.v1),
                    (crate::i18n::t_static("cmd.invol.report.g_int"), r.g),
                    (crate::i18n::t_static("cmd.invol.report.g1_int"), r.g1),
                ] {
                    any |= nf_check_push_row(&mut md, label, v);
                }
                for (i, qv) in r.q.iter().enumerate() {
                    any |= nf_check_push_row(&mut md, &crate::i18n::t_fmt("cmd.invol.report.q_unknown", &[("v0", &(i + 1).to_string())]), *qv);
                }
                if !any {
                    md.push_str("| — | — |\n");
                }
                md.push('\n');
            }
            // 检验量公式推导（K / W）：K = p23–p25 表值；W = E 列，公式见入库 CSV 头注。
            // 与卡片同一份 nf_check_by_a_m 行（不许两处各算一遍）。
            let dims_row = rows.iter().find(|r| r.k.is_some() || r.e.is_some());
            let (k_tab, e_tab) = dims_row
                .map(|r| (r.k, r.e))
                .unwrap_or((None, None));
            md.push_str(crate::i18n::t_static("cmd.invol.report.h_insp_deriv"));
            md.push_str(
                crate::i18n::t_static("cmd.invol.report.nf_conv"),
            );
            md.push_str(crate::i18n::t_static("cmd.invol.report.t_steps"));
            md.push_str("|---|---|---|---|---|---|\n");
            let mut n = 0usize;
            if let Some(k) = k_tab {
                md.push_str(&report_row(
                    &mut n,
                    "跨测齿数 K",
                    crate::i18n::t_static("cmd.invol.report.k_formula"),
                    &crate::i18n::t_fmt("cmd.invol.report.k_subst", &[("v0", &trim(a)), ("v1", &trim(e.m)), ("v2", &e.z.to_string()), ("v3", &dims_row.map(|r| r.page).unwrap_or(0).to_string())]),
                    &format!("K = {}", trim(k)),
                    crate::i18n::t_static("cmd.invol.report.k_source"),
                ));
            } else {
                md.push_str(
                    crate::i18n::t_static("cmd.invol.report.k_no_hit_row"),
                );
            }
            match (k_tab, e_tab) {
                (Some(k), Some(e_tab_v)) => {
                    let (_, w_geo) = involute_span(e.m, e.z, 20.0, e.x);
                    let resid = (w_geo - e_tab_v).abs();
                    md.push_str(&report_row(
                        &mut n,
                        crate::i18n::t_static("cmd.invol.report.w_step"),
                        crate::i18n::t_static("cmd.invol.report.w_formula"),
                        &crate::i18n::t_fmt("cmd.invol.report.w_subst", &[("v0", &trim(e.m)), ("v1", &k.to_string()), ("v2", &e.z.to_string()), ("v3", &trim(e.x))]),
                        &crate::i18n::t_fmt(
                            "cmd.invol.report.w_result",
                            &[
                                ("v0", &trim(w_geo)),
                                ("v1", &trim(e_tab_v)),
                                ("v2", &format!("{:.3}", resid)),
                            ],
                        ),
                        crate::i18n::t_static("cmd.invol.report.w_source"),
                    ));
                    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.same_source_kw", &[("v0", &trim(a)), ("v1", &trim(e.m))]));
                }
                _ => {
                    md.push_str(crate::i18n::t_static("cmd.invol.report.w_no_hit_row"));
                }
            }
            md.push('\n');
            // 计算标准-设计尺寸（p26）。
            for r in rows.iter().filter(|r| r.page == 26) {
                md.push_str(&crate::i18n::t_fmt("cmd.invol.report.h_check_row", &[("v0", &r.page.to_string()), ("v1", &r.table_no.to_string()), ("v2", &r.source), ("v3", &r.section)]));
                md.push_str(crate::i18n::t_static("cmd.invol.report.t_qty"));
                let mut any = false;
                for (label, v) in [
                    (crate::i18n::t_static("cmd.invol.report.d_pitch"), r.d),
                    (crate::i18n::t_static("cmd.invol.report.db_base"), r.db),
                    (crate::i18n::t_static("cmd.invol.report.x_shift"), r.x),
                    (crate::i18n::t_static("cmd.invol.report.s_arc"), r.s),
                    (crate::i18n::t_static("cmd.invol.report.sb_arc"), r.sb),
                    (crate::i18n::t_static("cmd.invol.report.b_flat"), r.b),
                    (crate::i18n::t_static("cmd.invol.report.b2_fillet"), r.b2),
                    (crate::i18n::t_static("cmd.invol.report.rf_fillet"), r.rf),
                    (crate::i18n::t_static("cmd.invol.report.rr_fillet"), r.rr),
                    (crate::i18n::t_static("cmd.invol.report.h_chamfer"), r.h),
                    (crate::i18n::t_static("cmd.invol.report.ri_slot"), r.ri),
                ] {
                    any |= nf_check_push_row(&mut md, label, v);
                }
                if !any {
                    md.push_str("| — | — |\n");
                }
                md.push('\n');
            }
            // 公差/配合偏差（p29/p31–p35；每格 `+上偏差/下偏差`，微米）。
            for r in rows.iter().filter(|r| r.has_deviations()) {
                let devs = r
                    .dev
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| !s.is_empty())
                    .map(|(i, s)| format!("dev{}={s}", i + 1))
                    .collect::<Vec<_>>()
                    .join(&crate::i18n::t("cmd.invol.sep.list"));
                md.push_str(&crate::i18n::t_fmt("cmd.invol.report.dev_line", &[("devs", &devs), ("v0", &r.source_note())]));
            }
            md.push('\n');
        }
    } else if std == SplineStd::ANSI {
        // ANSI B92.1 原标准检验表（公法线跨测表）本仓未收 → 只给**渐开线几何推导**，
        // 与 GB/T 3478.6 式(11)、NF E22-141 同一条跨距公式（`involute_span` 唯一实现）。
        // 卡片 17 项里的「公法线长度 / 跨测齿数」格保持「—」（本仓未收原表，不冒充）。
        md.push_str(crate::i18n::t_static("cmd.invol.report.h_ansi_deriv"));
        md.push_str(
            crate::i18n::t_static("cmd.invol.report.ansi_note"),
        );
        let (kn, wn) = involute_span(e.m, e.z, e.alpha_deg, e.x);
        md.push_str(crate::i18n::t_static("cmd.invol.report.t_steps"));
        md.push_str("|---|---|---|---|---|---|\n");
        let mut n = 0usize;
        md.push_str(&report_row(
            &mut n,
            crate::i18n::t_static("cmd.invol.report.kn_step"),
            crate::i18n::t_static("cmd.invol.report.kn_formula"),
            &crate::i18n::t_fmt("cmd.invol.report.kn_subst", &[("v0", &e.z.to_string()), ("v1", &trim(e.alpha_deg))]),
            &format!("Kn = {kn}"),
            crate::i18n::t_static("cmd.invol.report.kn_source"),
        ));
        md.push_str(&report_row(
            &mut n,
            crate::i18n::t_static("cmd.invol.report.wn_step"),
            crate::i18n::t_static("cmd.invol.report.wn_formula"),
            &crate::i18n::t_fmt("cmd.invol.report.wn_subst", &[("v0", &trim(e.m)), ("v1", &trim(e.alpha_deg)), ("v2", &kn.to_string()), ("v3", &e.z.to_string()), ("v4", &trim(e.x))]),
            &format!("Wn = {} mm", trim(wn)),
            crate::i18n::t_static("cmd.invol.report.wn_source"),
        ));
        md.push_str(&crate::i18n::t("cmd.invol.report.wn_same_formula"));
        md.push_str(&crate::i18n::t_fmt(
            "cmd.invol.report.wn_identity_line",
            &[("v0", &format!("{wn:.4}"))],
        ));
    } else if std != SplineStd::DIN {
        md.push_str(
            crate::i18n::t_static("cmd.invol.report.no_table"),
        );
    } else {
        let d_b = e.d_b.unwrap_or_else(|| d_b_from_x(e.m, e.z, e.x));
        match inspection_query(d_b, e.m, e.z) {
            Ok(r) => {
                md.push_str(crate::i18n::t_static("cmd.invol.report.t_qty_notes"));
                md.push_str(&crate::i18n::t_fmt("cmd.invol.report.m1_row", &[("v0", &trim(r.m1)), ("v1", &trim(r.d_m_shaft))]));
                md.push_str(&crate::i18n::t_fmt("cmd.invol.report.m2_row", &[("v0", &trim(r.m2)), ("v1", &trim(r.d_m_hub))]));
                md.push_str(&crate::i18n::t_fmt("cmd.invol.report.k_row", &[("v0", &r.k.to_string())]));
                md.push_str(&crate::i18n::t_fmt("cmd.invol.report.wk_row", &[("v0", &trim(r.w_k))]));
                if let Some(row) = &r.row {
                    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.tbl_hit_row", &[("v0", &row.page.to_string()), ("v1", &row.table_no.to_string()), ("v2", &row.source)]));
                }
                md.push_str(&crate::i18n::t_fmt("cmd.invol.report.src_line", &[("v0", &r.source), ("v1", &if r.from_table() {
                        crate::i18n::t_static("cmd.invol.report.src_exact")
                    } else {
                        crate::i18n::t_static("cmd.invol.report.src_formula")
                    })]));
                for note in &r.notes {
                    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.note_line", &[("note", &note)]));
                }
                md.push('\n');
            }
            Err(err) => md.push_str(&crate::i18n::t_fmt("cmd.invol.report.no_check_dims", &[("err", &err)])),
        }
    }

    // ── 5. 数据来源与校验 ──
    md.push_str(crate::i18n::t_static("cmd.invol.report.h_sources"));
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.src_preset", &[("v0", &report_preset_source(std)), ("v1", &e.profile)]));
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.src_module", &[("v0", &module_source(std))]));
    md.push_str(&crate::i18n::t_fmt("cmd.invol.report.src_datum", &[("v0", &origin
            .map(|o| o.note())
            .unwrap_or_else(|| crate::i18n::t_static("cmd.invol.report.src_datum_na").to_string()))]));
    if std == SplineStd::DIN {
        md.push_str(&crate::i18n::t_fmt("cmd.invol.report.src_full_check", &[("v0", &inspection_formula_report().summary)]));
    }
    let res_d = (e.d() - e.m * e.z as f64).abs();
    let res_db = (e.db() - e.d() * e.alpha().cos()).abs();
    let res_deff = (e.d_eff() - (e.d() + 2.0 * e.x * e.m)).abs();
    md.push_str(&crate::i18n::t_fmt(
        "cmd.invol.report.identity_line",
        &[
            ("v0", &format!("{res_d:.2e}")),
            ("v1", &format!("{res_db:.2e}")),
            ("v2", &format!("{res_deff:.2e}")),
        ],
    ));
    if let Some(d_b) = e.d_b {
        let res = (d_b - d_b_from_x(e.m, e.z, e.x)).abs();
        md.push_str(&crate::i18n::t_fmt(
            "cmd.invol.report.identity_din",
            &[("v0", &format!("{res:.2e}"))],
        ));
    }
    if let Some(a) = e.a {
        let res = (a - a_from_x(e.m, e.z, e.x)).abs();
        md.push_str(&crate::i18n::t_fmt(
            "cmd.invol.report.identity_nf",
            &[("v0", &format!("{res:.2e}"))],
        ));
    }
    md.push_str(
        crate::i18n::t_static("cmd.invol.report.same_engine"),
    );
    md
}

// ─────────────────────────── 测试 ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn near6(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    fn has_line(entities: &[EntityType], a: [f64; 2], b: [f64; 2]) -> bool {
        entities.iter().any(|e| match e {
            EntityType::Line(l) => {
                let (p, q) = ([l.start.x, l.start.y], [l.end.x, l.end.y]);
                (near(p[0], a[0]) && near(p[1], a[1]) && near(q[0], b[0]) && near(q[1], b[1]))
                    || (near(p[0], b[0])
                        && near(p[1], b[1])
                        && near(q[0], a[0])
                        && near(q[1], a[1]))
            }
            _ => false,
        })
    }

    fn line_layer_at(entities: &[EntityType], y: f64) -> Option<&str> {
        entities.iter().find_map(|e| match e {
            EntityType::Line(l) if near(l.start.y, y) && near(l.end.y, y) => {
                Some(l.common.layer.as_str())
            }
            _ => None,
        })
    }

    fn arc_centers_at(entities: &[EntityType], radius: f64) -> Vec<(f64, f64)> {
        entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) if near(a.radius, radius) => {
                    let c = (a.start_angle.to_degrees() + a.end_angle.to_degrees()) / 2.0;
                    let span = a.end_angle.to_degrees() - a.start_angle.to_degrees();
                    Some((c.rem_euclid(360.0), span))
                }
                _ => None,
            })
            .collect()
    }

    /// GB 四种齿廓：da/df/ρf/cF/d/db/s 与标准公式逐项吻合；内花键接口同步。
    #[test]
    fn gb_four_profiles_match_standard_formulas() {
        let m = 3.0;
        let z = 20u32;
        // (profile, α, ha*, hf*, ρf*, 大径系数, 小径系数)
        let cases: [(&str, f64, f64, f64, f64, f64, f64); 4] = [
            ("30平齿根", 30.0, 0.5, 0.75, 0.2, 1.0, 1.5),
            ("30圆齿根", 30.0, 0.5, 0.9, 0.4, 1.0, 1.8),
            ("37.5圆齿根", 37.5, 0.45, 0.7, 0.3, 0.9, 1.4),
            ("45圆齿根", 45.0, 0.4, 0.6, 0.25, 0.8, 1.2),
        ];
        for (profile, alpha, ha, hf, rho, da_c, df_c) in cases {
            let p = InvolParams::gb(profile, m, z).unwrap();
            assert_eq!(p.std, SplineStd::GB);
            assert!(near(p.alpha_deg, alpha), "{profile} α");
            assert!(near(p.ha_star, ha), "{profile} ha*");
            assert!(near(p.hf_star, hf), "{profile} hf*");
            assert!(near(p.rho_star, rho), "{profile} ρf*");
            assert!(near(p.c_f_star, 0.1), "{profile} cF*");
            // 直径一律标准公式
            assert!(near(p.d(), m * z as f64), "{profile} d");
            assert!(
                near(p.db(), p.d() * alpha.to_radians().cos()),
                "{profile} db"
            );
            assert!(near(p.da(), m * (z as f64 + da_c)), "{profile} da");
            assert!(near(p.df(), m * (z as f64 - df_c)), "{profile} df");
            // 派生量
            assert!(near(p.rho_f(), rho * m), "{profile} ρf");
            assert!(near(p.c_f(), 0.1 * m), "{profile} cF");
            assert!(near(p.s(), std::f64::consts::PI * m / 2.0), "{profile} s");
            assert!(near(p.tooth_depth(), (p.da() - p.df()) / 2.0));
            // GB 内花键：大径 D_ei = m(z+df_c)（表 3），小径 D_ii = D_Fe max + 2C_F
            assert!(
                near(p.internal_major_dia(), m * (z as f64 + df_c)),
                "{profile} D_ei"
            );
            let p_int = p.clone().with_internal(true);
            p_int.validate().unwrap();
            assert!(near(p_int.internal_major_dia(), p.internal_major_dia()));
            assert!(near(
                p_int.internal_minor_dia(),
                p.gb_form_dia_max() + 2.0 * p.c_f()
            ));
            assert!(p_int.internal_minor_dia() < p_int.internal_major_dia());
            // 齿顶/渐开线终止、起始圆
            assert!(near(p.d_involute_end(), p.da()), "{profile} 终止圆 = da");
            assert!(
                near(p.d_involute_start(), p.db().max(p.df())),
                "{profile} 起始圆"
            );
            p.validate().unwrap();
        }
    }

    /// 内花键：GB 表 3 直径（D_ei / D_ii=D_Fe+2C_F）、DIN d_f2/d_a2、端视图几何（半径互换）、
    /// 侧视草案的外/内半径。
    #[test]
    fn internal_spline_diameters_and_views() {
        // GB30R m3 z20：D_ei=65.4，D_ii=D_Fe max+2C_F，且位于 d=60 两侧。
        let p = InvolParams::gb("30圆齿根", 3.0, 20)
            .unwrap()
            .with_internal(true);
        p.validate().unwrap();
        assert!(near(p.internal_major_dia(), 3.0 * (20.0 + 1.8)));
        assert!(near(p.internal_minor_dia(), p.gb_form_dia_max() + 0.6));
        assert!(
            p.internal_minor_dia() < p.d() && p.d() < p.internal_major_dia(),
            "D_ii={} d={} D_ei={}",
            p.internal_minor_dia(),
            p.d(),
            p.internal_major_dia()
        );
        // 端视图：每齿 2×12 渐开线 + 槽底弧 + 齿顶弧 = 26 图元/齿 + 分度圆 1 + 2 中心线。
        let front = p.front_view(1.0).unwrap();
        assert_eq!(front.len(), 20 * (2 * INVOLUTE_SEGMENTS + 2) + 3);
        // 内花键端视也画分度圆（`3中心线层`、半径 d/2）。
        let pitch_circles: Vec<f64> = front
            .iter()
            .filter_map(|e| match e {
                EntityType::Circle(c) if c.common.layer == LAYER_CENTER => Some(c.radius),
                _ => None,
            })
            .collect();
        assert_eq!(pitch_circles.len(), 1, "内花键端视分度圆 1 条");
        assert!(near(pitch_circles[0], p.d() / 2.0), "半径 = d/2");
        // 弧半径 = D_ei/2（外侧齿槽底）与 D_ii/2（里侧齿顶）；不能出现外花键的 da/df 半径。
        let radii: Vec<f64> = front
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) => Some(a.radius),
                _ => None,
            })
            .collect();
        assert!(radii.iter().any(|r| near(*r, p.internal_major_dia() / 2.0)));
        assert!(radii.iter().any(|r| near(*r, p.internal_minor_dia() / 2.0)));
        assert!(!radii.iter().any(|r| near(*r, p.da() / 2.0)));
        // 内花键无侧视图（用户定案：同内齿轮）→ 明确报错；剖视走 gear.rs 内齿轮模板，引擎侧不出草案。
        let e = p.side_view(30.0).unwrap_err();
        assert!(e.contains("内花键不提供") && e.contains("侧视图"), "{e}");
        let e = p.section_view(30.0).unwrap_err();
        assert!(e.contains("内齿轮口径"), "{e}");
        // 外花键行为不变（da/df 口径）。
        let ext = InvolParams::gb("30圆齿根", 3.0, 20).unwrap();
        assert!(near(ext.da(), 63.0) && near(ext.df(), 54.6));
        assert!(!ext.internal);

        // DIN m2 z18 x0.45：内花键 d_f2=d_B=40、d_a2=d−0.9m+2xm=36。
        let d = InvolParams::din(2.0, 18, 0.45)
            .unwrap()
            .with_d_b(40.0)
            .with_internal(true);
        d.validate().unwrap();
        assert!(near(d.internal_major_dia(), 40.0));
        assert!(near(d.internal_minor_dia(), 36.0));
        let dfront = d.front_view(1.0).unwrap();
        assert!(dfront
            .iter()
            .any(|e| matches!(e, EntityType::Arc(a) if near(a.radius, 20.0))));
        assert!(dfront
            .iter()
            .any(|e| matches!(e, EntityType::Arc(a) if near(a.radius, 18.0))));

        // 齿顶低于基圆的内花键：允许径向直线降级（45° 小齿数），不报错。
        let p45 = InvolParams::gb("45圆齿根", 2.0, 8)
            .unwrap()
            .with_internal(true);
        if p45.internal_tip_radius() < p45.db() / 2.0 {
            assert!(p45.front_view(1.0).is_ok(), "低齿顶应走径向直线降级");
        }

        // GB DB 文案常量与报错链接（齿轮/轴/结构要素三处共用）。
        assert!(gb_d_b_msg().contains("DIN 5480") && gb_d_b_msg().contains("GB/T 3478"));
    }

    /// DIN：hf*=0.55（齿侧对中基准）、公式与 DIN 5480-2 名义表逐行吻合；x 边界 [−0.05, 0.45]。
    #[test]
    fn din_formulas_match_5480_2_and_x_boundaries() {
        let (m, z, x) = (2.0, 18u32, 0.2);
        let p = InvolParams::din(m, z, x).unwrap();
        assert_eq!(p.std, SplineStd::DIN);
        assert!(near(p.alpha_deg, 30.0));
        assert!(
            near(p.ha_star, 0.45) && near(p.hf_star, 0.55) && near(p.rho_star, 0.16),
            "DIN 基础齿廓：h_fP*=0.55（DIN 5480-1 条 5.1）"
        );
        assert!(near(p.c_f_star, 0.10), "cF* = c* = h_fP*−h_aP* = 0.10");
        let alpha = 30f64.to_radians();
        let d = m * z as f64;
        assert!(near(p.d(), d));
        assert!(near(p.d_eff(), d + 2.0 * x * m));
        assert!(near(p.da(), d + 2.0 * x * m + 0.9 * m), "d_a1={}", p.da());
        assert!(
            near(p.df(), d + 2.0 * x * m - 2.0 * 0.55 * m),
            "d_f1={}",
            p.df()
        );
        assert!(near(
            p.s(),
            std::f64::consts::PI * m / 2.0 + 2.0 * x * m * alpha.tan()
        ));
        assert!(near(p.db(), d * alpha.cos()));
        assert!(near(p.rho_f(), 0.16 * m));
        assert!(near(p.clearance(), 0.10 * m), "c* = hf*−ha* = 0.10");
        assert!(near(p.c_f(), 0.10 * m), "cF = 0.10m");
        assert!(near(p.x_m(), x * m));
        assert!(p.x_in_range());
        p.validate().unwrap();

        // ── DIN 5480-2 名义表核对（`DIN5480-2_名义表.csv` p.11，m=0.5、x·m=0.225 即 x=0.45）：──
        //   z=34（d_B=18）→ d=17.00、d_a1=17.90、d_f1=16.90；z=10（d_B=6）→ d=5.00、d_a1=5.90、d_f1=4.90。
        //   hf*=0.55 时 d_f1 = mz + 2xm − 1.1m 与表逐行一致（旧 hf*=0.60 会得 16.85/4.85）。
        for (z2, da_want, df_want) in [(34u32, 17.90, 16.90), (10u32, 5.90, 4.90)] {
            let q = InvolParams::din(0.5, z2, 0.45).unwrap();
            assert!(near6(q.d(), 0.5 * z2 as f64), "z={z2} d");
            assert!(near6(q.da(), da_want), "z={z2} d_a1={}", q.da());
            assert!(near6(q.df(), df_want), "z={z2} d_f1={}", q.df());
        }

        // 边界内/外
        for good in [-0.05, 0.0, 0.45] {
            assert!(InvolParams::din(m, z, good).is_ok(), "x={good} 应合法");
        }
        for bad in [-0.050_1, 0.450_1] {
            let e = InvolParams::din(m, z, bad).unwrap_err();
            assert!(e.contains("x"), "错误信息要点名 x：{e}");
        }
    }

    /// 预设代号（CLI/DSL/GUI）：四 GB + DIN 往返；常见别名；非法代号拒绝。
    #[test]
    fn preset_token_round_trip_and_aliases() {
        for std in [SplineStd::GB, SplineStd::DIN] {
            for p in std.presets() {
                let code = preset_code(std, p.profile).expect("有代号");
                let (std2, profile2) = parse_preset_token(code).expect("能解析");
                assert_eq!(std2, std, "{code}");
                assert_eq!(profile2, p.profile, "{code}");
            }
        }
        assert_eq!(parse_preset_token("GB").unwrap().1, "30圆齿根", "GB 裸写默认");
        assert_eq!(parse_preset_token("DIN").unwrap().0, SplineStd::DIN);
        assert_eq!(parse_preset_token("gb 30p").unwrap().1, "30平齿根");
        assert_eq!(parse_preset_token("GB30圆齿根").unwrap().1, "30圆齿根");
        assert_eq!(parse_preset_token("GB37.5圆齿根").unwrap().1, "37.5圆齿根");
        assert_eq!(parse_preset_token("GB375R").unwrap().1, "37.5圆齿根");
        assert_eq!(parse_preset_token("din30r").unwrap().0, SplineStd::DIN);
        // 体系标识（显式，不再靠 d_B 反推）：NF/ANSI 也认（函数只解析，具体参数由各体系 resolve 处理）
        assert_eq!(parse_preset_token("NF").unwrap().0, SplineStd::NF);
        assert_eq!(parse_preset_token("NFE22141").unwrap().0, SplineStd::NF);
        assert_eq!(parse_preset_token("NF E22-141").unwrap().0, SplineStd::NF);
        assert_eq!(parse_preset_token("ANSI").unwrap().0, SplineStd::ANSI);
        assert_eq!(parse_preset_token("ANSI B92.1").unwrap().0, SplineStd::ANSI);
        // M/DP 是齿轮体系，不是花键预设
        for bad in ["", "GB99", "DIN99", "GB30x", "M", "DP"] {
            assert!(parse_preset_token(bad).is_none(), "{bad} 应拒绝");
        }
    }

    /// db 随 α 变化；ψ(R) 与 `gear.rs` 同口径（R=rb 时 ψ=s/d+invα）。
    #[test]
    fn db_and_involute_track_alpha() {
        let (m, z) = (3.0, 20u32);
        let gb30 = InvolParams::gb("30圆齿根", m, z).unwrap();
        let gb375 = InvolParams::gb("37.5圆齿根", m, z).unwrap();
        let gb45 = InvolParams::gb("45圆齿根", m, z).unwrap();
        let d = m * z as f64;
        for p in [&gb30, &gb375, &gb45] {
            assert!(
                near(p.db(), d * p.alpha_deg.to_radians().cos()),
                "{} db",
                p.profile
            );
            let want = p.s() / d + inv(p.alpha_deg.to_radians());
            assert!(
                near6(p.half_tooth_angle(p.db() / 2.0), want),
                "{} ψ(rb)",
                p.profile
            );
        }
        assert!(gb30.db() > gb375.db() && gb375.db() > gb45.db());
        // α 越大齿顶半角越小（同 ha* 下）
        assert!(gb30.half_tooth_angle(gb30.da() / 2.0) > gb45.half_tooth_angle(gb45.da() / 2.0));
    }

    /// 端视图：齿数 / 相位（0° 处是齿槽）/ 渐开线端点解析式 / 实体数。
    #[test]
    fn front_view_phase_tooth_count_and_involute() {
        let p = InvolParams::gb("30圆齿根", 3.0, 20).unwrap();
        let e = p.front_view(1.0).unwrap();
        let (ra, rf) = (p.da() / 2.0, p.df() / 2.0);
        let tips = arc_centers_at(&e, ra);
        let roots = arc_centers_at(&e, rf);
        assert_eq!(tips.len(), 20, "20 条齿顶弧");
        assert_eq!(roots.len(), 20, "20 条齿根弧");
        // 齿顶弧中心 180/z + k·360/z = 9° + 18k
        let mut want_tip: Vec<String> = (0..20)
            .map(|k| format!("{:.4}", (9.0 + 18.0 * k as f64).rem_euclid(360.0)))
            .collect();
        let mut got_tip: Vec<String> = tips.iter().map(|(c, _)| format!("{c:.4}")).collect();
        want_tip.sort();
        got_tip.sort();
        assert_eq!(got_tip, want_tip, "齿心线相位");
        // 齿槽弧中心 0° + k·360/z（0° 处是齿槽）
        let mut want_root: Vec<String> =
            (0..20).map(|k| format!("{:.4}", 18.0 * k as f64)).collect();
        let mut got_root: Vec<String> = roots.iter().map(|(c, _)| format!("{c:.4}")).collect();
        want_root.sort();
        got_root.sort();
        assert_eq!(got_root, want_root, "齿槽心线相位");
        // 0° 齿槽：有一条齿根弧中心归一后为 0°、且几何上跨 0°（start 在 360° 前、end 在 360° 后）
        assert!(
            e.iter().any(|x| matches!(x, EntityType::Arc(a)
                if near(a.radius, rf)
                    && (((a.start_angle + a.end_angle) / 2.0).to_degrees()).rem_euclid(360.0) < 1e-6
                    && a.start_angle.to_degrees().rem_euclid(360.0) > 180.0
                    && a.end_angle.to_degrees().rem_euclid(360.0) < 180.0)),
            "0° 处应是齿槽（齿根弧跨 0°）"
        );
        assert!(!got_tip.iter().any(|s| s == "0.0000"));
        // 渐开线端点解析式（第一齿 +1 侧，c = pitch/2 = 9°）
        let c = p.pitch_angle() / 2.0;
        let q0 = p.flank_point(rf, c, 1.0);
        let q1 = p.flank_point(ra, c, 1.0);
        let q0b = p.flank_point(
            p.r_involute_start() + (ra - p.r_involute_start()) / INVOLUTE_SEGMENTS as f64,
            c,
            1.0,
        );
        assert!(has_line(&e, q0, q0b), "渐开线首段端点");
        let last = e.iter().find_map(|x| match x {
            EntityType::Line(l)
                if near((l.end.x.powi(2) + l.end.y.powi(2)).sqrt(), ra)
                    && (l.end.y.atan2(l.end.x) - (c + p.half_tooth_angle(ra))).abs() < 1e-6 =>
            {
                Some(())
            }
            _ => None,
        });
        assert!(last.is_some(), "渐开线末点应落在齿顶圆上");
        assert!(near(q1[0].hypot(q1[1]), ra));
        // 实体数：每齿 2×12 折线 + 2 弧，加分度圆 1 + 中心线 2（本件 db<df 无径向直线）
        assert!(p.db() / 2.0 < rf);
        assert_eq!(e.len(), 20 * (2 * INVOLUTE_SEGMENTS + 2) + 3);
    }

    /// ②花键端视十字线长度 = `da + 6n`（走齿轮 `centerline_len`，不留硬编码）：
    /// n=1 与 n=2 都成立；外花键 n=2 → 63+12 = **75**，内花键（D_ei=65.4）n=2 → **77.4**。
    #[test]
    fn front_view_cross_centerlines_follow_6n_like_gear() {
        let p = InvolParams::gb("30圆齿根", 3.0, 20).unwrap();
        let (da, _) = (p.da(), p.df());
        for n in [1.0, 2.0] {
            let e = p.front_view(n).unwrap();
            let lens: Vec<f64> = e
                .iter()
                .filter_map(|x| match x {
                    EntityType::Line(l) if l.common.layer == LAYER_CENTER => Some(
                        ((l.end.x - l.start.x).powi(2) + (l.end.y - l.start.y).powi(2)).sqrt(),
                    ),
                    _ => None,
                })
                .collect();
            let want = crate::gear::centerline_len(da, n);
            assert_eq!(lens.len(), 2, "外花键十字线两条（分度圆是 Circle 不算）");
            for len in lens {
                assert!((len - want).abs() < 1e-9, "n={n}：十字线长 {len} ≠ da+6n={want}");
            }
        }
        assert!((crate::gear::centerline_len(da, 2.0) - 75.0).abs() < 1e-9, "n=2 → 75");

        let pi = p.clone().with_internal(true);
        let d_ei = pi.internal_major_dia();
        for n in [1.0, 2.0] {
            let e = pi.front_view(n).unwrap();
            let lens: Vec<f64> = e
                .iter()
                .filter_map(|x| match x {
                    EntityType::Line(l) if l.common.layer == LAYER_CENTER => Some(
                        ((l.end.x - l.start.x).powi(2) + (l.end.y - l.start.y).powi(2)).sqrt(),
                    ),
                    _ => None,
                })
                .collect();
            let want = crate::gear::centerline_len(d_ei, n);
            assert_eq!(lens.len(), 2);
            for len in lens {
                assert!((len - want).abs() < 1e-9, "内花键 n={n}：{len} ≠ D_ei+6n={want}");
            }
        }
        assert!((crate::gear::centerline_len(d_ei, 2.0) - 77.4).abs() < 1e-9, "n=2 → 77.4");
    }

    /// ③ 花键端视画分度圆（`3中心线层`，半径按体系是分度圆半径）：外花键 + **内花键**都画；
    /// 内齿轮仍按模板不画（本测试只锁花键，不碰 `gear.rs` 内齿轮口径）。
    #[test]
    fn spline_front_has_pitch_circle() {
        let p = InvolParams::gb("30圆齿根", 3.0, 20).unwrap();
        let e = p.front_view(1.0).unwrap();
        let circles: Vec<&ocs_plugin_api::host::acadrust::entities::Circle> = e
            .iter()
            .filter_map(|x| match x {
                EntityType::Circle(c) if c.common.layer == LAYER_CENTER => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(circles.len(), 1, "外花键端视分度圆 1 条");
        assert!((circles[0].radius - p.d() / 2.0).abs() < 1e-12, "半径 = d/2");
        assert!((circles[0].center.x).abs() < 1e-12 && (circles[0].center.y).abs() < 1e-12);
        // ANSI：分度圆用 D（= N/P，引擎内部 d()，mm）→ D/2；不能是别的半径。
        let pa = InvolParams::ansi("ANSI30平齿根齿侧", 8.0, 20).unwrap();
        let ea = pa.front_view(1.0).unwrap();
        assert!(
            ea.iter().any(|x| matches!(x, EntityType::Circle(c)
                if c.common.layer == LAYER_CENTER && (c.radius - pa.d() / 2.0).abs() < 1e-12)),
            "ANSI 分度圆 = D/2 = {}",
            pa.d() / 2.0
        );
        // 内花键端视也画分度圆（用户 2026-09-21 后定案；半径 = d/2）。
        let pi = p.clone().with_internal(true);
        let ei = pi.front_view(1.0).unwrap();
        let icircles: Vec<&ocs_plugin_api::host::acadrust::entities::Circle> = ei
            .iter()
            .filter_map(|x| match x {
                EntityType::Circle(c) if c.common.layer == LAYER_CENTER => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(icircles.len(), 1, "内花键端视分度圆 1 条");
        assert!(
            (icircles[0].radius - pi.d() / 2.0).abs() < 1e-12,
            "内花键分度圆半径 = d/2 = {}",
            pi.d() / 2.0
        );
        // ANSI 内花键：D=N/P（引擎内部 d()）→ D/2。
        let pia = pa.clone().with_internal(true);
        let eai = pia.front_view(1.0).unwrap();
        assert!(
            eai.iter().any(|x| matches!(x, EntityType::Circle(c)
                if c.common.layer == LAYER_CENTER && (c.radius - pia.d() / 2.0).abs() < 1e-12)),
            "ANSI 内花键分度圆 = D/2 = {}",
            pia.d() / 2.0
        );
    }

    /// 侧视图/剖视图：矩形 + 小径线图层不同；坐标口径；
    /// 外加分度圆两条（`3中心线层`，与齿轮侧视图同口径）。
    #[test]
    fn side_view_rectangle_and_minor_layers() {
        let p = InvolParams::gb("30圆齿根", 3.0, 20).unwrap();
        let len = 30.0;
        let (ra, rf, d2) = (p.da() / 2.0, p.df() / 2.0, p.d() / 2.0);
        let side = p.side_view(len).unwrap();
        let section = p.section_view(len).unwrap();
        assert_eq!(side.len(), 9, "矩形 4 + 小径 2 + 轴线 1 + 分度圆 2");
        assert_eq!(section.len(), 9);
        for es in [&side, &section] {
            assert!(has_line(es, [0.0, -ra], [0.0, ra]));
            assert!(has_line(es, [len, -ra], [len, ra]));
            assert!(has_line(es, [0.0, ra], [len, ra]));
            assert!(has_line(es, [0.0, -rf], [len, -rf]));
            assert!(has_line(es, [-3.0, 0.0], [len + 3.0, 0.0]));
            // 分度圆两条：`3中心线层`，y=±d/2，长度 = 该视图轮廓 `[0, len]`。
            assert!(has_line(es, [0.0, d2], [len, d2]), "上侧分度线");
            assert!(has_line(es, [0.0, -d2], [len, -d2]), "下侧分度线");
        }
        // 与齿轮侧视图的**唯一区别**：花键侧视有小径（齿根圆）两条 `2细线层`。
        assert_eq!(
            line_layer_at(&side, rf),
            Some(LAYER_THIN),
            "常规侧视上侧小径细线"
        );
        assert_eq!(
            line_layer_at(&side, -rf),
            Some(LAYER_THIN),
            "常规侧视下侧小径细线"
        );
        assert_eq!(
            line_layer_at(&section, rf),
            Some(LAYER_MAIN),
            "剖视小径轮廓线"
        );
        assert_eq!(line_layer_at(&side, d2), Some(LAYER_CENTER), "分度圆中心线层");
        assert_eq!(line_layer_at(&side, -d2), Some(LAYER_CENTER));
        assert_eq!(line_layer_at(&side, ra), Some(LAYER_MAIN), "大径始终轮廓线");
    }

    /// 收尾弧：圆心/相切/终点/l 数值/图元；de 太小报错。
    #[test]
    fn runout_arc_center_tangency_and_length() {
        let p = InvolParams::gb("30圆齿根", 3.0, 20).unwrap();
        let de = 80.0;
        let (h, r) = (p.tooth_depth(), p.hob_radius(de));
        assert!(near(h, 4.2));
        let l = p.runout_length(de).unwrap();
        assert!(near(l, (h * (2.0 * r - h)).sqrt()));
        assert!((l - 17.842_641_2).abs() < 1e-4, "l={l}");
        // 圆心 (x0, rf+R)，切点 (x0, rf)
        let c = p.runout_center(30.0, de, 1.0).unwrap();
        assert!(near(c[0], 30.0) && near(c[1], p.df() / 2.0 + r));
        let t = [30.0, p.df() / 2.0];
        let dist = ((c[0] - t[0]).powi(2) + (c[1] - t[1]).powi(2)).sqrt();
        assert!(near(dist, r), "与小径相切");
        // 终点交大径 (x0+l, ra)
        let a = p.runout_end_angle(de).unwrap().to_radians();
        let endp = [c[0] + r * a.cos(), c[1] - r * a.sin()];
        assert!(near6(endp[0], 30.0 + l) && near6(endp[1], p.da() / 2.0));
        // 图元：2 弧 + 1 终点竖线；竖线长 ±ra
        let ents = p.runout_arcs(30.0, de, LAYER_THIN).unwrap();
        assert_eq!(ents.len(), 3);
        assert!(has_line(
            &ents,
            [30.0 + l, -p.da() / 2.0],
            [30.0 + l, p.da() / 2.0]
        ));
        // de ≤ da 报错
        let e = p.runout_length(p.da()).unwrap_err();
        assert!(e.contains("de"), "{e}");
    }

    // ── DIN 5480-2 名义表（d_B 查表）──

    /// 入库形状 + 全表反推公式 `x_from_d_b` 与表值逐行一致（含 d=m·z、基圆列）。
    #[test]
    fn din5480_table_shape_and_formula_holds_for_every_row() {
        let rows = din5480_rows();
        assert_eq!(
            rows.len(),
            721,
            "入库 721 行（旧 618 + m=1.5 用户截图 56 + m=5 用户截图 47）"
        );
        assert_eq!(
            rows.iter().filter(|r| (r.m - 5.0).abs() < 1e-9).count(),
            47,
            "m=5 用户截图 47 行"
        );
        // m=5 的 2 处 z 修正（原始读数 147/160 仍在 CSV，证据在 flags）：147→14、160→16。
        let r17 = rows
            .iter()
            .find(|r| (r.m - 5.0).abs() < 1e-9 && (r.d_b - 75.0).abs() < 1e-9 && r.z == 14)
            .expect("r17（d_B=75，z 修正为 14）");
        assert!(
            r17.flags.contains("expected=14") && r17.flags.contains("recheck(L=140|S=14)"),
            "修正证据在 flags：{}",
            r17.flags
        );
        let r22 = rows
            .iter()
            .find(|r| (r.m - 5.0).abs() < 1e-9 && (r.d_b - 85.0).abs() < 1e-9 && r.z == 16)
            .expect("r22（d_B=85，z 修正为 16）");
        assert!(
            r22.flags.contains("expected=16") && r22.flags.contains("recheck(L=160|S=16)"),
            "修正证据在 flags：{}",
            r22.flags
        );
        for r in rows {
            assert!(
                (r.d - r.m * r.z as f64).abs() < 1e-9,
                "p{} m{} z{}：d≠m·z",
                r.page,
                trim(r.m),
                r.z
            );
            // 基圆列（CSV 的 d_b = d·cos30°）是排版值，精度 3 位 → 容差 0.05。
            assert!(
                (r.base_dia - r.d * 30f64.to_radians().cos()).abs() < 0.05,
                "p{} 基圆列 d_b 与 d·cos30 偏离过大：{} vs {}",
                r.page,
                r.base_dia,
                r.d * 30f64.to_radians().cos()
            );
            // 任务指定公式：x = (d_B − m(z+1.1)) / (2m)，与表列 x1_m/m 逐行一致。
            let x = x_from_d_b(r.d_b, r.m, r.z);
            assert!(
                (x - r.x).abs() < 1e-9,
                "p{} m{} z{} d_B{}：x_from_d_b={} 表 x={}",
                r.page,
                trim(r.m),
                r.z,
                trim(r.d_b),
                x,
                r.x
            );
            assert!((d_b_from_x(r.m, r.z, x) - r.d_b).abs() < 1e-9, "正反变换不闭合");
        }
        let mods = din5480_modules();
        assert_eq!(mods.len(), 16, "已入库 16 个模数档：{mods:?}");
        assert!(mods.iter().any(|m| (*m - 1.5).abs() < 1e-9), "m=1.5 已补入");
        assert!(mods.iter().any(|m| (*m - 5.0).abs() < 1e-9), "m=5 已补入");
    }

    /// m=1.5（用户截图补入，page=1/table_no=14）抽样：查表命中 + `x_from_d_b` 与表值一致。
    #[test]
    fn din5480_m15_lookup_hits_and_x_from_d_b_matches_table() {
        // (d_B, z, 表 x₁·m)：首行 / r15（d_b 已按 flags 修正）/ 末行。
        let cases: [(f64, u32, f64); 3] = [(12.0, 6, 0.675), (26.0, 16, 0.175), (110.0, 72, 0.175)];
        for (d_b, z, x_m) in cases {
            let rows = lookup_by_d_b(d_b, Some(1.5)).expect("m=1.5 应有数据");
            let r = rows
                .iter()
                .find(|r| r.z == z)
                .unwrap_or_else(|| panic!("m=1.5 d_B={d_b} 应命中 z={z}"));
            assert_eq!((r.page, r.table_no), (1, 14), "用户截图 1 / 表 14");
            assert!((r.x_m - x_m).abs() < 1e-9, "x₁·m={}", r.x_m);
            let x = x_from_d_b(r.d_b, r.m, r.z);
            assert!((x - r.x).abs() < 1e-9, "x_from_d_b 与表值 x 不一致");
            assert!((d_b_from_x(r.m, r.z, x) - r.d_b).abs() < 1e-9, "正反变换不闭合");
            let (p, o) = resolve_din_by_d_b(d_b, Some(1.5), Some(z), None).expect("三参应查表");
            assert!(matches!(o, D_bOrigin::Table(_)));
            assert!((p.x - r.x).abs() < 1e-9);
            assert_eq!(p.d_b, Some(d_b));
        }
        // r15：修正后的 d_b 与 d·cos30 表值口径一致；原读 29,785 仍保留在 flags 里。
        let r15 = din5480_rows()
            .iter()
            .find(|r| (r.m - 1.5).abs() < 1e-9 && r.z == 16 && (r.d_b - 26.0).abs() < 1e-9)
            .expect("r15（d_B=26,z=16）");
        assert!((r15.base_dia - 20.7846).abs() < 1e-9);
        assert!(r15.flags.contains("29,785"), "原读保留在 flags：{}", r15.flags);
    }

    /// 名义表 4 条恒等式（`d_f2=d_B`、`d_a1=d_B−0.2m`、`d_a2=d−0.9m+2x₁m`、
    /// `d_f1=d−1.1m+2x₁m`）全表（721 行）复核，违例必须为 0；m=1.5 行 56/56、m=5 行 47/47 成立。
    #[test]
    fn din5480_four_identities_hold_for_every_row() {
        let tol = 5e-3; // 表值最细 2 位小数 → 半个末位
        let (mut n, mut n15, mut n5) = (0usize, 0usize, 0usize);
        let mut viol: Vec<String> = Vec::new();
        for (i, line) in DIN5480_2_CSV.lines().enumerate() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let f = split_csv_line(line);
            if f.first().map(|s| s.trim()) == Some("page") {
                continue;
            }
            let page = f[0].trim().parse::<u16>().expect("page");
            let m = csv_decimal(&f[1]).expect("m");
            let table_no = f[2].trim().parse::<u32>().expect("table_no");
            let d_b = csv_decimal(&f[3]).expect("d_B");
            let z_raw = f[4].trim().parse::<u32>().expect("z");
            // m=5 的 2 处 z 按代码修正表修正后参与恒等式（assets 原始值不改）。
            let z = din5480_m5_fix_z(page, table_no, d_b, z_raw);
            let d = csv_decimal(&f[5]).expect("d");
            let x_m = csv_decimal(&f[7]).expect("x1_m");
            let d_f2 = csv_decimal(&f[9]).expect("d_f2");
            let d_a2 = csv_decimal(&f[12]).expect("d_a2");
            let d_a1 = csv_decimal(&f[13]).expect("d_a1");
            let d_f1 = csv_decimal(&f[15]).expect("d_f1");
            n += 1;
            if (m - 1.5).abs() < DIN_M_TOL {
                n15 += 1;
            }
            if (m - 5.0).abs() < DIN_M_TOL {
                n5 += 1;
            }
            let checks = [
                ("d=mz", d - m * z as f64),
                ("d_f2=d_B", d_f2 - d_b),
                ("d_a1=d_B−0.2m", d_a1 - (d_b - 0.2 * m)),
                ("d_a2=d−0.9m+2x₁m", d_a2 - (d - 0.9 * m + 2.0 * x_m)),
                ("d_f1=d−1.1m+2x₁m", d_f1 - (d - 1.1 * m + 2.0 * x_m)),
            ];
            for (name, res) in checks {
                if res.abs() > tol {
                    viol.push(format!(
                        "第 {} 行 p{} m{} z{}：{name} 残差 {res}",
                        i + 1,
                        f[0],
                        f[1],
                        z
                    ));
                }
            }
        }
        assert_eq!(n, 721, "全表 721 行");
        assert_eq!(n15, 56, "m=1.5 56 行");
        assert_eq!(n5, 47, "m=5 47 行");
        assert!(viol.is_empty(), "恒等式违例 {} 条：{viol:?}", viol.len());
    }

    /// m=5（用户截图 1/表 26，page=35/table_no=26）抽样：查表命中 + `x_from_d_b` 与表值一致
    /// + 修正后的 47 行 8 条恒等式（`d=mz`、`d_b=d·cos30`、`e₂`、`d_B=d+1.1m+2x₁m`、
    /// `d_f2=d_B`、`d_a1=d_B−0.2m`、`d_a2=d−0.9m+2x₁m`、`d_f1=d−1.1m+2x₁m`）违例 0。
    #[test]
    fn din5480_m5_lookup_and_eight_identities() {
        // (d_B, z, 表 x₁·m)：首行 / 2 处 z 修正行 / 末行。
        let cases: [(f64, u32, f64); 4] = [
            (40.0, 6, 2.25),
            (75.0, 14, -0.25),
            (85.0, 16, -0.25),
            (320.0, 62, 2.25),
        ];
        for (d_b, z, x_m) in cases {
            let rows = lookup_by_d_b(d_b, Some(5.0)).expect("m=5 应有数据");
            let r = rows
                .iter()
                .find(|r| r.z == z)
                .unwrap_or_else(|| panic!("m=5 d_B={d_b} 应命中 z={z}"));
            assert_eq!((r.page, r.table_no), (35, 26), "用户截图 1 / 表 26");
            assert!((r.x_m - x_m).abs() < 1e-9, "x₁·m={}", r.x_m);
            let x = x_from_d_b(r.d_b, r.m, r.z);
            assert!((x - r.x).abs() < 1e-9, "x_from_d_b 与表值 x 不一致");
            assert!((d_b_from_x(r.m, r.z, x) - r.d_b).abs() < 1e-9, "正反变换不闭合");
            let (p, o) = resolve_din_by_d_b(d_b, Some(5.0), Some(z), None).expect("三参应查表");
            assert!(matches!(o, D_bOrigin::Table(_)));
            assert!((p.x - r.x).abs() < 1e-9);
            assert_eq!(p.d_b, Some(d_b));
        }
        // 修正后 47 行跑 8 条恒等式（原始 CSV + `DIN5480_M5_NOMINAL_Z_FIXES`）。
        let tol = 5e-3; // 表值最细 2 位小数 → 半个末位
        let a = 30f64.to_radians();
        let mut n = 0usize;
        let mut viol: Vec<String> = Vec::new();
        for (i, line) in DIN5480_2_CSV.lines().enumerate() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let f = split_csv_line(line);
            if f.first().map(|s| s.trim()) == Some("page") {
                continue;
            }
            let m = csv_decimal(&f[1]).expect("m");
            if (m - 5.0).abs() > DIN_M_TOL {
                continue;
            }
            let page = f[0].trim().parse::<u16>().expect("page");
            let table_no = f[2].trim().parse::<u32>().expect("table_no");
            let d_b = csv_decimal(&f[3]).expect("d_B");
            let z_raw = f[4].trim().parse::<u32>().expect("z");
            let z = din5480_m5_fix_z(page, table_no, d_b, z_raw);
            let d = csv_decimal(&f[5]).expect("d");
            let base_dia = csv_decimal(&f[6]).expect("d_b");
            let x_m = csv_decimal(&f[7]).expect("x1_m");
            let e2 = csv_decimal(&f[8]).expect("e2_s1");
            let d_f2 = csv_decimal(&f[9]).expect("d_f2");
            let d_a2 = csv_decimal(&f[12]).expect("d_a2");
            let d_a1 = csv_decimal(&f[13]).expect("d_a1");
            let d_f1 = csv_decimal(&f[15]).expect("d_f1");
            n += 1;
            let checks = [
                ("d=mz", d - m * z as f64),
                ("d_b=d·cos30", base_dia - d * a.cos()),
                (
                    "e₂=mπ/2+2x₁m·tan30",
                    e2 - (m * std::f64::consts::PI / 2.0 + 2.0 * x_m * a.tan()),
                ),
                ("d_B=d+1.1m+2x₁m", d_b - (d + 1.1 * m + 2.0 * x_m)),
                ("d_f2=d_B", d_f2 - d_b),
                ("d_a1=d_B−0.2m", d_a1 - (d_b - 0.2 * m)),
                ("d_a2=d−0.9m+2x₁m", d_a2 - (d - 0.9 * m + 2.0 * x_m)),
                ("d_f1=d−1.1m+2x₁m", d_f1 - (d - 1.1 * m + 2.0 * x_m)),
            ];
            for (name, res) in checks {
                if res.abs() > tol {
                    viol.push(format!(
                        "第 {} 行 p{} z{}：{name} 残差 {res}",
                        i + 1,
                        f[0],
                        z
                    ));
                }
            }
        }
        assert_eq!(n, 47, "m=5 47 行");
        assert!(viol.is_empty(), "8 条恒等式违例 {} 条：{viol:?}", viol.len());
    }

    /// 查表命中：抽各档（含多行 z/x₁ 变体）；按 d_B+m / d_B+z 补全。
    #[test]
    fn din5480_lookup_hits_samples_and_multi_row_variants() {
        // 各档抽一行：m0.5→p11 z10 x0.45；m1→p19 z16 x0.45；m1.25→p21 z10 x0.05；
        // m1.75→p25 z6；m2→p27 z18 x0.45；m10→p41 z44 x−0.05。
        let cases: [(f64, f64, u32, u16); 6] = [
            (0.5, 6.0, 10, 11),
            (1.0, 18.0, 16, 19),
            (1.25, 14.0, 10, 21),
            (1.75, 13.0, 6, 25),
            (2.0, 40.0, 18, 27),
            (10.0, 450.0, 44, 41),
        ];
        for (m, d_b, z, page) in cases {
            let rows = lookup_by_d_b(d_b, Some(m)).expect("应命中");
            assert!(
                rows.iter().any(|r| r.z == z && r.page == page),
                "m={} d_B={} 应命中 p{} z={}：{:?}",
                m,
                d_b,
                page,
                z,
                rows.iter().map(|r| (r.page, r.z)).collect::<Vec<_>>()
            );
            // 每条命中行都满足 d_B 反解式（表值精度）。
            for r in &rows {
                assert!((x_from_d_b(r.d_b, r.m, r.z) - r.x).abs() < 1e-9);
            }
            // 只给 d_B + m → 补 z；只给 d_B + z → 补 m。
            let (p1, o1) = resolve_din_by_d_b(d_b, Some(m), None, None).expect("d_B+m 应补 z");
            assert_eq!(p1.z, z, "d_B+m 补出的 z");
            assert!(matches!(o1, D_bOrigin::Table(_)));
            let (p2, o2) = resolve_din_by_d_b(d_b, None, Some(z), None).expect("d_B+z 应补 m");
            assert!((p2.m - m).abs() < 1e-9, "d_B+z 补出的 m");
            assert!(matches!(o2, D_bOrigin::Table(_)));
        }
        // 多行变体：p31 m=3 d_B=45 → z=13(x0.45) 与 z=14(x−0.05)，返回两行。
        let rows = lookup_by_d_b(45.0, Some(3.0)).unwrap();
        assert_eq!(rows.len(), 2, "同一 d_B 多个 z/x₁ 是表格事实");
        assert_eq!(rows.iter().map(|r| r.z).collect::<Vec<_>>(), vec![13, 14]);
        assert!((rows[0].x - 0.45).abs() < 1e-9 && (rows[1].x + 0.05).abs() < 1e-9);
        // 只给 d_B（不给 m）也会命中的是**跨模数**的多行（45 在多个 m 档都有）；
        // 补全时多行 → 报候选要 M/Z。
        assert!(lookup_by_d_b(45.0, None).unwrap().len() >= 2);
        let e = resolve_din_by_d_b(45.0, None, None, None).unwrap_err();
        assert!(e.contains("多个") && e.contains("m=3 z=13") && e.contains("m=3 z=14"), "{e}");
        let e = resolve_din_by_d_b(45.0, Some(3.0), None, None).unwrap_err();
        assert!(e.contains("多个 z 变体") && e.contains("请再给 Z"), "{e}");
        let e = resolve_din_by_d_b(6.0, None, Some(6), None).unwrap_err();
        assert!(e.contains("多个 m 变体") && e.contains("0.75") && e.contains("0.8"), "{e}");
    }

    /// 未命中：附近候选；剔除档位（m=5）明确报数据缺失；m=1.5 现已可命中。
    #[test]
    fn din5480_lookup_miss_lists_candidates_and_missing_modules() {
        let e = lookup_by_d_b(41.0, Some(2.0)).unwrap_err();
        assert!(e.contains("附近候选"), "{e}");
        assert!(e.contains("d_B=40") && e.contains("d_B=42"), "应列出附近 40/42：{e}");
        assert!(e.contains("m=2"), "{e}");
        let e = lookup_by_d_b(1_000.0, None).unwrap_err();
        assert!(e.contains("附近候选"), "不给 m 也要给候选：{e}");
        // m=1.5：已由用户截图补入（d_B=20 → z=12 唯一，可直接命中）
        assert!(
            lookup_by_d_b(20.0, Some(1.5)).is_ok(),
            "m=1.5 已补入，不该再报缺失"
        );
        // m=5：已由用户截图补入（d_B=50 → z=8 唯一，可直接命中）
        let m5 = lookup_by_d_b(50.0, Some(5.0)).expect("m=5 已补入，不该再报缺失");
        assert!(
            m5.iter().any(|r| r.z == 8 && (r.page, r.table_no) == (35, 26)),
            "m=5 d_B=50 应命中截图表 26：{m5:?}"
        );
        // 未收录档位（如 m=0.9）列出已入库档位
        let e = lookup_by_d_b(10.0, Some(0.9)).unwrap_err();
        assert!(e.contains("表中没有 m=0.9") && e.contains("已入库档位"), "{e}");
        // 非法输入
        assert!(lookup_by_d_b(-1.0, None).unwrap_err().contains("正数"));
        assert!(lookup_by_d_b(10.0, Some(0.0)).unwrap_err().contains("正数"));
    }

    /// `d_B + m + z` 互相印证：命中表行 → Table；表无此 (m,z) → Formula；不一致/越界报错。
    #[test]
    fn din5480_resolve_cross_checks_d_b_m_z() {
        // p27 m=2 d_B=40 z=18 x=0.45：查表命中。
        let (p, o) = resolve_din_by_d_b(40.0, Some(2.0), Some(18), None).unwrap();
        assert!((p.x - 0.45).abs() < 1e-9 && p.z == 18 && (p.m - 2.0).abs() < 1e-9);
        assert!((p.d() - 36.0).abs() < 1e-9 && p.d_b == Some(40.0));
        assert!(matches!(&o, D_bOrigin::Table(r) if r.page == 27), "{o:?}");
        assert_eq!(o.note(), "查表命中 p27 m=2");
        // 表里没有 (m=2, z=19)：公式解出，未命中表。
        let (p, o) = resolve_din_by_d_b(41.0, Some(2.0), Some(19), None).unwrap();
        assert!((p.x - 0.2).abs() < 1e-9, "x={}", p.x);
        assert_eq!(o.note(), "由公式解出，未命中表");
        // d_B 为主参数：输入 m/z 与 d_B 不相容（x 越界）→ 按 d_B 重算 z（保留 m），明文提示。
        let (p, o) = resolve_din_by_d_b(40.0, Some(2.0), Some(14), None).unwrap();
        assert_eq!(p.z, 18, "按 d_B=40、m=2 取 z=18");
        assert!((p.x - 0.45).abs() < 1e-9);
        assert!(
            matches!(&o, D_bOrigin::Adjusted(n)
                if n.contains("按基准直径 d_B=40 取 z=18")
                    && n.contains("与输入 z=14 不符")
                    && n.contains("d_B 为主参数")),
            "{o:?}"
        );
        // x 输入与 d_B 不符：按 d_B 取 x（保留命中表行的来源），明文提示。
        let (p, o) = resolve_din_by_d_b(40.0, Some(2.0), Some(18), Some(0.2)).unwrap();
        assert!((p.x - 0.45).abs() < 1e-9, "按 d_B 取表值 x=0.45，不用输入 x=0.2");
        assert!(
            matches!(&o, D_bOrigin::Adjusted(n)
                if n.contains("查表命中 p27") && n.contains("与输入 x=0.2 不符")),
            "{o:?}"
        );
        // 输入 z 超界（x>0.45）：表里 (m=2, d_B=100) 唯一行 z=48 → 按 d_B 重算。
        let (p, o) = resolve_din_by_d_b(100.0, Some(2.0), Some(10), None).unwrap();
        assert_eq!(p.z, 48);
        assert!(matches!(&o, D_bOrigin::Adjusted(n) if n.contains("取 z=48")));
        // d_B+m 表外 → 公式推 z（标注“推导值、未命中表”+区间依据）。
        let (p, o) = resolve_din_by_d_b(41.0, Some(2.0), None, None).unwrap();
        assert_eq!(p.z, 19);
        assert!((p.x - 0.2).abs() < 1e-9, "x={}", p.x);
        assert!(
            matches!(&o, D_bOrigin::Derived(n)
                if n.contains("推导值、未命中表")
                    && n.contains("z∈[18.5000,19.5000]")
                    && n.contains("取 z=19")),
            "{o:?}"
        );
        // d_B+z 表外 → 公式给名义 m（取 x=0）并标注来源/可行区间。
        let (p, o) = resolve_din_by_d_b(41.0, None, Some(19), None).unwrap();
        assert!((p.m - 41.0 / 20.1).abs() < 1e-12, "m={}", p.m);
        assert!(p.x.abs() < 1e-12);
        assert!(
            matches!(&o, D_bOrigin::Derived(n)
                if n.contains("推导值、未命中表")
                    && n.contains("取 x=0")
                    && n.contains("可行区间 m∈")),
            "{o:?}"
        );
        // m=5 已补入：d_B+m 查表补 z（d_B=50 → z=8），来源 Table。
        let (p, o) = resolve_din_by_d_b(50.0, Some(5.0), None, None).unwrap();
        assert_eq!(p.z, 8);
        assert!(
            matches!(&o, D_bOrigin::Table(r) if (r.page, r.table_no) == (35, 26)),
            "{o:?}"
        );
    }

    /// GB 体系没有基准直径概念：`resolve_spline` 给 d_B 必须报统一文案；
    /// DIN 不给 d_B 时由 m/z/x 正算 d_B（来源 Computed）。
    #[test]
    fn resolve_spline_gb_rejects_d_b_and_din_computes_d_b() {
        let e = resolve_spline(SplineStd::GB, "GB30R", Some(40.0), Some(3.0), Some(20), None)
            .unwrap_err();
        assert_eq!(e, gb_d_b_msg(), "{e}");
        let e = resolve_spline(SplineStd::GB, "30圆齿根", Some(40.0), None, None, None)
            .unwrap_err();
        assert_eq!(e, gb_d_b_msg(), "{e}");
        // DIN：m+z（无 d_B）→ 由 d_B=m(z+1.1+2x) 正算并回填，不报“不匹配”。
        let (p, origin) =
            resolve_spline(SplineStd::DIN, "DIN30", None, Some(2.0), Some(18), Some(0.2))
                .unwrap();
        assert!((p.d_b.unwrap() - 39.0).abs() < 1e-9, "d_B=m(z+1.1+2x)=2×(18+1.1+0.4)=39.0");
        assert!(matches!(origin, Some(D_bOrigin::Computed)), "{origin:?}");
        assert_eq!(origin.unwrap().note(), "由 m/z/x 正算 d_B=m(z+1.1+2x)");
        // GB：m+z 正常，d_b 保持 None（无此概念）。
        let (p, origin) =
            resolve_spline(SplineStd::GB, "GB30R", None, Some(3.0), Some(20), None).unwrap();
        assert!(p.d_b.is_none() && origin.is_none());
    }

    /// GB/T 3478.1 基本齿廓不含变位：入口层拒绝非零 x（x=0/省略等价通过）；DIN/NF 不受影响。
    #[test]
    fn resolve_spline_gb_rejects_nonzero_x_but_allows_zero() {
        // x 省略 / x=0：两者等价，GB 模型 x 恒 0。
        for x in [None, Some(0.0)] {
            let (p, o) =
                resolve_spline(SplineStd::GB, "GB30R", None, Some(3.0), Some(20), x).unwrap();
            assert_eq!(p.std, SplineStd::GB);
            assert!(p.x.abs() < 1e-12, "GB x 应恒 0，实际 {}", p.x);
            assert!(o.is_none());
        }
        // 非零 x：明确报错，沿用 GB 统一文案口径（点名标准 + 给收到值）。
        for bad in [0.2, -0.05, 1.0] {
            let e = resolve_spline(SplineStd::GB, "GB30R", None, Some(3.0), Some(20), Some(bad))
                .unwrap_err();
            assert!(e.contains(&gb_x_msg()), "{e}");
            assert!(e.contains("收到"), "{e}");
        }
        // DIN/NF 的 x 行为不变（仍是几何自变量/主系列参数）；ANSI 仍拒非零 x（原有口径）。
        let (pd, _) =
            resolve_spline(SplineStd::DIN, "DIN30", None, Some(2.0), Some(18), Some(0.2))
                .unwrap();
        assert!((pd.x - 0.2).abs() < 1e-12);
        let (pn, _) =
            resolve_spline(SplineStd::NF, "NFP", None, Some(2.5), Some(20), Some(0.2)).unwrap();
        assert!((pn.x - 0.2).abs() < 1e-12);
        let e = resolve_spline(SplineStd::ANSI, "ANSI30P", None, Some(5.0), Some(20), Some(0.2))
            .unwrap_err();
        assert!(e.contains("不使用变位系数 x"), "{e}");
    }

    /// ③ 四个体系的模数/径节候选来源：GB 表 2 15 种、DIN/NF 来自各自 CSV 实际 m 列、ANSI 17 项。
    #[test]
    fn module_candidates_follow_each_standard_table() {
        // GB：GB/T 3478.1-2008 表 2，15 种（第 1 系列优先 + 第 2 系列）。
        assert_eq!(GB_MODULES.len(), 15);
        assert_eq!(
            module_candidates(SplineStd::GB),
            vec![
                0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0
            ]
        );
        // DIN：din5480_2_nominal.csv 实际 m 列（16 档），候选随表走、不另写系列。
        let din = module_candidates(SplineStd::DIN);
        assert_eq!(din, din5480_modules(), "DIN 候选必须来自名义表实际 m 列");
        assert_eq!(din.len(), 16);
        // NF：nf_e22141_dims.csv 实际 m 列（10 档；含 1.667/3.75/7.5，不含 GB/DIN 特有档）。
        let nf = module_candidates(SplineStd::NF);
        assert_eq!(nf, nf_e22141_modules());
        assert_eq!(
            nf,
            vec![0.5, 0.75, 1.0, 1.25, 1.667, 2.5, 3.75, 5.0, 7.5, 10.0]
        );
        for want in [1.667, 3.75, 7.5] {
            assert!(nf.iter().any(|v| (v - want).abs() < 1e-9), "NF 应含 {want}");
        }
        for ban in [0.6, 0.8, 1.5, 1.75, 3.0, 4.0, 6.0, 8.0] {
            assert!(
                !nf.iter().any(|v| (v - ban).abs() < 1e-9),
                "NF 不应含 GB/DIN 特有档 {ban}"
            );
            assert!(
                din.iter().any(|v| (v - ban).abs() < 1e-9)
                    || GB_MODULES.iter().any(|v| (v - ban).abs() < 1e-9),
                "{ban} 应在 GB/DIN 候选里"
            );
        }
        // ANSI：17 项径节。
        assert_eq!(module_candidates(SplineStd::ANSI).len(), 17);
        assert_eq!(ansi_pitches().len(), 17);
        // 来源文案各自点到表。
        assert!(module_source(SplineStd::GB).contains("GB/T 3478.1-2008 表 2"));
        assert!(module_source(SplineStd::DIN).contains("din5480_2_nominal.csv"));
        assert!(module_source(SplineStd::NF).contains("nf_e22141_dims.csv"));
        assert!(module_source(SplineStd::ANSI).contains("ansi_b921_formulas.csv"));
    }

    /// 体系外模数：GB/DIN/NF 在 `resolve_spline` 明确报错（给出来源表与可用值）；
    /// 命令行与 GUI 共用同一入口，所以这条错误文案两侧一致。
    #[test]
    fn unsupported_module_errors_point_to_source_table() {
        // GB 没有 0.7（GB/T 1357 有，花键表 2 没有）。
        let e =
            resolve_spline(SplineStd::GB, "GB30R", None, Some(0.7), Some(20), None).unwrap_err();
        assert!(
            e.contains("GB/T 3478.1-2008") && e.contains("表 2") && e.contains("0.75"),
            "{e}"
        );
        assert!(e.contains("不在体系可用系列"), "{e}");
        // DIN 没有 0.7；报错列出名义表实际 m 列。
        let e = resolve_spline(SplineStd::DIN, "DIN30", Some(40.0), Some(0.7), None, None)
            .unwrap_err();
        assert!(
            e.contains("DIN 5480-2 名义表") && e.contains("din5480_2_nominal.csv"),
            "{e}"
        );
        // NF 没有 2.0（GB/DIN 有）——必须按 NF 表报错，并给出 NF 特有档位。
        let e = resolve_spline(SplineStd::NF, "NFP", Some(80.0), Some(2.0), None, None)
            .unwrap_err();
        assert!(
            e.contains("NF E22-141") && e.contains("nf_e22141_dims.csv"),
            "{e}"
        );
        assert!(
            e.contains("1.667") && e.contains("3.75") && e.contains("7.5"),
            "{e}"
        );
        // 合法值通过。
        assert!(resolve_spline(SplineStd::NF, "NFP", None, Some(1.667), Some(45), None).is_ok());
    }

    /// ② DIN/NF：z 由 (d_B/A, m) 推导；显式 z 不符 → 以基准直径为准重算并明文提示。
    #[test]
    fn din_nf_z_derived_and_mismatch_recalculated() {
        // DIN 表内唯一：d_B=40、m=2 → z=18（表内直查）。
        let (z, note) = derive_z_from_bench_m(SplineStd::DIN, 40.0, 2.0).unwrap();
        assert_eq!(z, 18);
        assert!(note.contains("查表命中"), "{note}");
        // DIN 表外：d_B=41、m=2 → z=19（公式区间取整）。
        let (z, note) = derive_z_from_bench_m(SplineStd::DIN, 41.0, 2.0).unwrap();
        assert_eq!(z, 19);
        assert!(note.contains("表外推导"), "{note}");
        // 显式 z 不符 → 按 d_B 重算（沿用 Adjusted 口径）。
        let (p, o) =
            resolve_spline(SplineStd::DIN, "DIN30", Some(40.0), Some(2.0), Some(14), None)
                .unwrap();
        assert_eq!(p.z, 18);
        assert!(
            matches!(&o, Some(D_bOrigin::Adjusted(n))
                if n.contains("d_B 为主参数") && n.contains("与输入 z=14 不符")),
            "{o:?}"
        );
        // DIN 表内其它变体：显式 z 命中表行（45/3 → 13 或 14）→ 保留输入。
        let (p13, _) =
            resolve_spline(SplineStd::DIN, "DIN30", Some(45.0), Some(3.0), Some(13), None)
                .unwrap();
        assert_eq!(p13.z, 13);
        // NF 表内：A=80、m=3.75 → N=19。
        let (z, note) = derive_z_from_bench_m(SplineStd::NF, 80.0, 3.75).unwrap();
        assert_eq!(z, 19);
        assert!(note.contains("查表命中"), "{note}");
        // NF 显式 N 不符 → 按 A 重算 + 提示。
        let (p, o) =
            resolve_spline(SplineStd::NF, "NFP", Some(80.0), Some(3.75), Some(14), None)
                .unwrap();
        assert_eq!(p.z, 19, "按 A=80、m=3.75 取 N=19");
        assert!(
            matches!(&o, Some(D_bOrigin::Adjusted(n))
                if n.contains("A 为主参数") && n.contains("与输入 N=14 不符")),
            "{o:?}"
        );
        // NF 表外：A=81、m=3.75 → 主系列 x=0.8 推 N=20。
        let (z, note) = derive_z_from_bench_m(SplineStd::NF, 81.0, 3.75).unwrap();
        assert_eq!(z, 20);
        assert!(note.contains("表外推导"), "{note}");
        // GB/ANSI 无此概念。
        assert!(derive_z_from_bench_m(SplineStd::GB, 40.0, 2.0).is_err());
        assert!(derive_z_from_bench_m(SplineStd::ANSI, 40.0, 2.0).is_err());
    }

    // ── DIN 5480-2 检验表（M₁/M₂/D_M/k/W_k）──

    /// 入库形状：267 行 / 6 档 / 页表号 / flags 保留 / 首行数值。
    #[test]
    fn din5480_inspection_table_shape_and_parse() {
        let rows = inspection_rows();
        assert_eq!(
            rows.len(),
            267,
            "检验表入库 267 行（164 + m=1.5 截图 56 + m=5 截图 47）"
        );
        assert_eq!(inspection_modules(), vec![0.5, 0.75, 0.8, 1.0, 1.5, 5.0]);
        let pages: std::collections::BTreeSet<(u16, u32)> =
            rows.iter().map(|r| (r.page, r.table_no)).collect();
        for want in [(12, 3), (16, 7), (18, 9), (20, 11), (2, 15), (36, 27)] {
            assert!(pages.contains(&want), "应有 p{}/表{}：{pages:?}", want.0, want.1);
        }
        // flags 原样保留：m=1.5 的 15 处粘连 k 修正记录可查。
        assert!(
            rows.iter()
                .any(|r| r.flags.contains("row:vote_suspect:k:expected=10")),
            "m=1.5 的 k 修正 flags 应保留"
        );
        assert!(rows.iter().any(|r| r.source == "user_screenshot_2"));
        // m=5：17 处粘连 k 按 `DIN5480_M5_INSPECTION_K_FIXES` 生效；原读/反推仍在 flags。
        let m5: Vec<_> = rows.iter().filter(|r| (r.m - 5.0).abs() < 1e-9).collect();
        assert_eq!(m5.len(), 47, "m=5 47 行");
        assert!(m5.iter().any(|r| r.source == "user_screenshot_m5_2"));
        let k35 = m5
            .iter()
            .find(|r| (r.d_b - 85.0).abs() < 1e-9 && r.z == 15)
            .expect("m=5 d_B=85 z=15");
        assert_eq!(k35.k, 3, "35→3");
        assert!(
            k35.flags.contains("multiword(3|5)") && k35.flags.contains("expected=3"),
            "原读与反推证据保留在 flags：{}",
            k35.flags
        );
        let k471 = m5
            .iter()
            .find(|r| (r.d_b - 90.0).abs() < 1e-9 && r.z == 16)
            .expect("m=5 d_B=90 z=16");
        assert_eq!(k471.k, 4, "471→4");
        let k113 = m5
            .iter()
            .find(|r| (r.d_b - 320.0).abs() < 1e-9 && r.z == 62)
            .expect("m=5 d_B=320 z=62");
        assert_eq!(k113.k, 11, "113→11");
        // 首行（p12 m=0.5 d_B=6 z=10）。
        let r = rows.first().expect("非空");
        assert_eq!((r.page, r.table_no, r.z), (12, 3, 10));
        assert!((r.m - 0.5).abs() < 1e-9 && (r.d_b - 6.0).abs() < 1e-9);
        assert!((r.d_m_hub - 1.0).abs() < 1e-9 && (r.m2 - 3.796).abs() < 1e-9);
        assert!((r.a_m2 - 2.01).abs() < 1e-9);
        assert!((r.d_m_shaft - 1.55).abs() < 1e-9 && (r.m1 - 8.215).abs() < 1e-9);
        assert!((r.a_m1 - 1.14).abs() < 1e-9);
        assert_eq!(r.k, 3);
        assert!((r.w_k - 3.859).abs() < 1e-9);
        assert_eq!(r.source_note(), "p12 表3（source=A）");
    }

    /// 查表：各档抽样命中 / 多行变体 / 未收录档位 / 附近候选带 source。
    #[test]
    fn din5480_inspection_lookup_hits_multirow_and_missing() {
        // 6 档各抽一行（值直接来自 CSV；m=5 的 k 是修正表生效后的值）。
        let cases: [(f64, f64, u32, f64, f64, u32, f64); 6] = [
            (0.5, 6.0, 10, 8.215, 3.796, 3, 3.859),
            (0.75, 6.0, 6, 11.061, 2.674, 2, 3.608),
            (0.8, 6.0, 6, 9.784, 2.837, 2, 3.648),
            (1.0, 8.0, 6, 14.388, 3.910, 2, 4.810),
            (1.5, 12.0, 6, 20.173, 6.180, 2, 7.216),
            (5.0, 50.0, 8, 68.226, 31.103, 2, 24.517),
        ];
        for (m, d_b, z, m1, m2, k, w_k) in cases {
            let r = inspection_for(d_b, m, z)
                .unwrap_or_else(|e| panic!("m={m} d_B={d_b} z={z}：{e}"));
            assert!((r.m1 - m1).abs() < 1e-9 && (r.m2 - m2).abs() < 1e-9, "{r:?}");
            assert_eq!(r.k, k);
            assert!((r.w_k - w_k).abs() < 1e-9, "{r:?}");
            let hits = lookup_inspection(d_b, Some(m)).expect("应命中");
            assert!(hits.iter().any(|h| h.z == z && h.page == r.page));
        }
        // 多行变体：m=0.8/d_B=20 → z=24 与 OCR 残行 z=93。
        let rows = lookup_inspection(20.0, Some(0.8)).unwrap();
        assert_eq!(rows.len(), 2, "{rows:?}");
        assert_eq!(rows.iter().map(|r| r.z).collect::<Vec<_>>(), vec![24, 93]);
        // 不给 m 也能命中（跨 m 同 d_B）并带 page/source。
        let all = lookup_inspection(6.0, None).unwrap();
        assert!(all.len() >= 2 && all.iter().all(|r| !r.source.is_empty()));
        // 未命中 → 附近候选带页码/source。
        let e = lookup_inspection(41.0, Some(1.0)).unwrap_err();
        assert!(e.contains("附近候选") && e.contains("source="), "{e}");
        // m=5：已由用户截图补入（d_B=50 → z=8 精确命中）。
        let m5 = lookup_inspection(50.0, Some(5.0)).expect("m=5 已补入");
        assert!(
            m5.iter().any(|r| r.z == 8 && (r.page, r.table_no) == (36, 27)),
            "m=5 d_B=50 应命中截图表 27：{m5:?}"
        );
        // 检验表未收录档位（m=2）：给已入库检验档位。
        let e = lookup_inspection(10.0, Some(2.0)).unwrap_err();
        assert!(
            e.contains("检验表没有 m=2") && e.contains("0.5、0.75、0.8、1、1.5、5"),
            "{e}"
        );
        // 非法输入。
        assert!(lookup_inspection(-1.0, None).unwrap_err().contains("正数"));
        assert!(lookup_inspection(10.0, Some(0.0)).unwrap_err().contains("正数"));
    }

    /// `inspection_for` 精确取值 + 由名义表反查 m 的 `inspection_for_d_b_z`。
    #[test]
    fn din5480_inspection_exact_for_and_d_b_z() {
        let r = inspection_for(6.0, 0.5, 10).unwrap();
        assert!((r.m1 - 8.215).abs() < 1e-9 && (r.m2 - 3.796).abs() < 1e-9);
        assert_eq!(r.source_note(), "p12 表3（source=A）");
        // 同一 (d_B,m) 没有 z=11 → 报错列可选 z 与来源。
        let e = inspection_for(6.0, 0.5, 11).unwrap_err();
        assert!(e.contains("没有 z=11") && e.contains("z=10") && e.contains("source="), "{e}");
        // 反查 m：名义表 d_B=6 z=10 → m=0.5。
        let r2 = inspection_for_d_b_z(6.0, 10).unwrap();
        assert_eq!(r, r2);
        let e = inspection_for_d_b_z(6.0, 11).unwrap_err();
        assert!(e.contains("无法反查"), "{e}");
        // m=5（截图补入）：d_B=50、z=8 精确命中，k 按修正表=2（原读 22 在 flags）。
        let r5 = inspection_for(50.0, 5.0, 8).unwrap();
        assert_eq!((r5.page, r5.table_no, r5.k), (36, 27, 2));
        assert!(r5.flags.contains("multiword(2|2)") && r5.flags.contains("expected=2"));
        let r5b = inspection_for_d_b_z(50.0, 8).unwrap();
        assert_eq!(r5, r5b);
    }

    /// 267 行逐行公式对照（残差报告）：W_k/M1/M2 与表值、异常清单与验证结论。
    #[test]
    fn din5480_inspection_formulas_match_every_row() {
        let report = inspection_formula_report();
        assert_eq!(report.total, 267);
        assert_eq!(report.checked, 265, "z=93/97 两条 OCR 残行无名义 x");
        assert_eq!(report.skipped.len(), 2);
        assert_eq!(report.w_k_pass, 264, "{}", report.summary);
        assert_eq!(report.m1_pass, 256, "{}", report.summary);
        assert_eq!(report.m2_pass, 258, "{}", report.summary);
        assert_eq!(report.anomalies.len(), 17, "{}", report.summary);
        assert!(report.validated, "{}", report.summary);
        assert!(report.max_w_k_residual <= INSPECTION_TOL, "{}", report.summary);
        assert!(report.max_m1_residual <= INSPECTION_TOL, "{}", report.summary);
        assert!(report.max_m2_residual <= INSPECTION_TOL, "{}", report.summary);
        // 残差报告里对源数据 OCR 异常的具体说明。
        let res = inspection_residuals();
        assert_eq!(res.len(), 267);
        let k_row = res
            .iter()
            .find(|r| r.page == 18 && (r.d_b - 29.0).abs() < 1e-9 && r.z == 35)
            .expect("k=63 异常行");
        assert!(
            k_row.note.contains("W_k") && k_row.note.contains("源图") && k_row.note.contains("6"),
            "{k_row:?}"
        );
        let d_row = res
            .iter()
            .find(|r| r.page == 16 && (r.d_b - 37.0).abs() < 1e-9 && r.z == 48)
            .expect("p16 D_M 异常行");
        assert!(
            d_row.note.contains("d_m_shaft") && d_row.note.contains("1.55"),
            "{d_row:?}"
        );
        let m2_row = res
            .iter()
            .find(|r| r.page == 20 && (r.d_b - 10.0).abs() < 1e-9 && r.z == 8)
            .expect("p20 M2 异常行");
        assert!(
            m2_row.note.contains("m2") && m2_row.note.contains("5.583"),
            "{m2_row:?}"
        );
    }

    /// 跨测齿数 k 规则：表内可算的 264 行 264/264 命中（k=63 行为 OCR 残值）。
    #[test]
    fn din5480_inspection_span_teeth_rule_matches_table() {
        let mut ok = 0usize;
        for row in inspection_rows() {
            let Some(nom) = din5480_rows().iter().find(|r| {
                (r.m - row.m).abs() < 1e-9 && (r.d_b - row.d_b).abs() < 1e-9 && r.z == row.z
            }) else {
                continue; // z=93/97
            };
            let k = span_teeth(row.m, row.z, nom.x)
                .unwrap_or_else(|e| panic!("p{} z={}：{e}", row.page, row.z));
            if row.k > 30 {
                assert_eq!(k, 6, "p{} z={} 的 k=63 应反推为 6", row.page, row.z);
                continue;
            }
            assert_eq!(
                k, row.k,
                "p{} m={} d_B={} z={}：表 k={}、规则 k={}",
                row.page, row.m, row.d_b, row.z, row.k, k
            );
            ok += 1;
        }
        assert_eq!(ok, 264);
    }

    /// 自定义 z（不在表里）：公式算 W_k/M1/M2（含奇数齿修正）；不可算时明确报错。
    #[test]
    fn din5480_inspection_custom_z_formula_and_errors() {
        // m=0.5 检验表只覆盖偶 z；d_B=11.25,m=0.5,z=21 → x=0.2 由公式导出。
        let r = inspection_query(11.25, 0.5, 21).unwrap();
        assert!(r.row.is_none() && !r.from_table());
        assert!((r.x - 0.2).abs() < 1e-9, "x={}", r.x);
        assert_eq!(r.k, 4);
        assert!((r.w_k - 5.35).abs() < 1e-4, "W_k={}", r.w_k);
        assert!((r.m1 - 12.763_794).abs() < 1e-4, "M1={}", r.m1);
        assert!((r.m2 - 8.996_690).abs() < 1e-4, "M2={}", r.m2);
        assert!((r.d_m_shaft - 1.2).abs() < 1e-9 && (r.d_m_hub - 1.0).abs() < 1e-9);
        assert!(r.source.contains("公式") && r.source.contains("不在检验表"));
        assert!(r.notes.iter().any(|n| n.contains("逐行对照")));
        assert!(r.notes.iter().any(|n| n.contains("邻近表行")));
        // 偶数齿表内命中对照（查表优先）。
        let t = inspection_query(6.0, 0.5, 10).unwrap();
        assert!(t.from_table());
        assert!((t.m1 - 8.215).abs() < 1e-9 && (t.m2 - 3.796).abs() < 1e-9 && t.k == 3);
        // 奇数齿修正与偶数齿不同：同一 (11.25,0.5,20) 用偶齿公式。
        let even = inspection_query(11.0, 0.5, 20).unwrap();
        assert!(even.from_table());
        assert!(even.m1 != r.m1, "20 与 21 齿的 M1 不应相等");
        // x 超界 → 明确报“请提供对应表页”。
        let e = inspection_query(6.6, 0.5, 21).unwrap_err();
        assert!(e.contains("超出") && e.contains("请提供对应表页"), "{e}");
        // 检验表没有 m=2 档 → 公式缺 D_M，明确报缺档。
        let e = inspection_query(22.1, 2.0, 21).unwrap_err();
        assert!(e.contains("检验表没有 m=2") && e.contains("请提供对应表页"), "{e}");
    }

    /// `/api/invol_check` JSON：默认只查表；`check=1` 才公式导出。
    #[test]
    fn din5480_inspection_json_check_switch() {
        let j = inspection_json("db=6&m=0.5&z=10").unwrap();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["from_table"], true);
        assert_eq!(v["page"], 12);
        assert_eq!(v["table_no"], 3);
        assert_eq!(v["row_source"], "A");
        assert!((v["m1"].as_f64().unwrap() - 8.215).abs() < 1e-9);
        assert!(v["source"].as_str().unwrap().contains("查表 p12"));
        // 不开关：z 不在表 → 报错并提示 check=1。
        let e = inspection_json("db=11.25&m=0.5&z=21").unwrap_err();
        assert!(e.contains("check=1") && e.contains("无命中"), "{e}");
        // 开关：公式导出。
        let j = inspection_json("db=11.25&m=0.5&z=21&check=1").unwrap();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["from_table"], false);
        assert_eq!(v["k"], 4);
        assert!((v["m1"].as_f64().unwrap() - 12.763_794).abs() < 1e-4);
        assert!(v["source"].as_str().unwrap().contains("公式"));
    }

    // ── CSV dump（示例件，供人工审图）──

    fn csv_row(case: &str, view: &str, e: &EntityType) -> String {
        match e {
            EntityType::Line(l) => format!(
                "{case}-{view}-Line,{:.6},{:.6},{:.6},{:.6},{},,,,,\n",
                l.start.x, l.start.y, l.end.x, l.end.y, l.common.layer
            ),
            EntityType::Arc(a) => format!(
                "{case}-{view}-Arc,,,,,{},{:.6},{:.6},{:.6},{:.6},{:.6}\n",
                a.common.layer,
                a.center.x,
                a.center.y,
                a.radius,
                a.start_angle.to_degrees(),
                a.end_angle.to_degrees()
            ),
            _ => format!("{case}-{view}-other,,,,,,,,,,\n"),
        }
    }

    /// 把示例（GB 30°圆齿根 m=3 z=20；DIN m=2 z=18 x=0.2）dump 到
    /// `~/桌面/OCSM/review/invol_spline_demo.csv`（列：entity,x1,y1,x2,y2,layer,cx,cy,r,a0,a1；角度为度）。
    #[test]
    fn dump_demo_csv() {
        let gb = InvolParams::gb("30圆齿根", 3.0, 20).unwrap();
        let din = InvolParams::din(2.0, 18, 0.2).unwrap();
        let mut out = String::from("entity,x1,y1,x2,y2,layer,cx,cy,r,a0,a1\n");
        for (case, p, de) in [("GB", &gb, 80.0), ("DIN", &din, 50.0)] {
            for e in p.front_view(1.0).unwrap() {
                out.push_str(&csv_row(case, "front", &e));
            }
            for e in p.side_view(30.0).unwrap() {
                out.push_str(&csv_row(case, "side", &e));
            }
            for e in p.runout_arcs(30.0, de, LAYER_THIN).unwrap() {
                out.push_str(&csv_row(case, "runout", &e));
            }
        }
        let home = std::env::var("HOME").unwrap_or_default();
        if home.is_empty() {
            eprintln!("跳过 invol_spline_demo.csv：HOME 未设置");
            return;
        }
        let dir = std::path::Path::new(&home).join("桌面/OCSM/review");
        std::fs::create_dir_all(&dir).expect("创建 review 目录");
        let path = dir.join("invol_spline_demo.csv");
        std::fs::write(&path, out).expect("写 invol_spline_demo.csv");
        assert!(path.exists());
    }

    // ── NF E22-141 尺寸表（A 主参数；数据 = assets/nf_e22141_dims.csv）──

    /// 入库形状：288 行 / 10 档 / 17 处 OCR 修正全部生效 / x 公式逐行一致（288 行）。
    #[test]
    fn nf_e22141_table_shape_and_x_formula_for_every_row() {
        let rows = nf_e22141_rows();
        assert_eq!(rows.len(), 288, "入库 288 行（p18 144 + p20 39 + p21 49 + p22 56）");
        let mods = nf_e22141_modules();
        assert_eq!(mods.len(), 10, "10 个模数档：{mods:?}");
        for want in [0.5, 0.75, 1.0, 1.25, 1.667, 2.5, 3.75, 5.0, 7.5, 10.0] {
            assert!(mods.iter().any(|m| (*m - want).abs() < 1e-9), "缺 m={want}");
        }
        // 17 处 OCR 修正在解析时逐条生效（且命中行数与修正表条数一致）。
        let fixes: usize = rows.iter().map(|r| r.fixes.len()).sum();
        assert_eq!(fixes, NF_OCR_FIXES.len(), "OCR 修正应逐条生效");
        // 打印行（p22 m=5 A=105）修正后 N=19；x 互换修正后满足 A 方程。
        let r105 = rows
            .iter()
            .find(|r| (r.m - 5.0).abs() < 1e-9 && (r.a - 105.0).abs() < 1e-9)
            .expect("m=5 A=105");
        assert_eq!(r105.z, 19, "z=192 是 OCR 粘连，应修为 19");
        let r130 = rows
            .iter()
            .find(|r| r.page == 21 && (r.m - 3.75).abs() < 1e-9 && (r.a - 130.0).abs() < 1e-9)
            .expect("p21 m=3.75 A=130");
        assert!((r130.x.unwrap() - 0.633).abs() < 1e-9, "x 互换修正");
        assert!((r130.s.unwrap() - 7.618).abs() < 1e-9, "s 随修正后的 x");
        // x 公式与表值逐行一致（288 行；容差 5e-3 = m=1.667=5/3 的排版精度）。
        let mut max_res = 0.0f64;
        let mut max_a_res = 0.0f64;
        let mut checked = 0usize;
        for r in rows {
            let x = x_from_a(r.a, r.m, r.z);
            if let Some(xv) = r.x {
                max_res = max_res.max((xv - x).abs());
                checked += 1;
            }
            max_a_res = max_a_res.max((a_from_x(r.m, r.z, x) - r.a).abs());
        }
        assert_eq!(checked, 144, "288 行中 144 行是 p18 内花键行（无 x 列）");
        assert!(max_res <= 5e-3, "x 公式最大残差 {max_res}");
        assert!(max_a_res <= 5e-3, "A 正反变换最大残差 {max_a_res}");
        // 9 行疑原表印误：值原样保留（D=132.5 不得被修成 122.5）。
        assert_eq!(NF_SUSPECTED_SOURCE_ERRORS.len(), 9);
        let d130 = rows
            .iter()
            .find(|r| r.page == 18 && (r.a - 130.0).abs() < 1e-9)
            .expect("p18 m=3.75 A=130");
        assert!((d130.internal_tip.unwrap() - 132.5).abs() < 1e-9, "印误值保留");
    }

    /// 目录载荷：NF 名义表**整表 288 行**下发（不在载荷层按 (m,A,N) 去重），
    /// 且逐 m 的 A 候选集合与 `assets/nf_e22141_dims.csv` 完全一致（含 1.667/3.75/7.5）；
    /// `partgen::catalog_json()`（=/api/parts 顶层 `spline_engine`）同样 288 条。
    #[test]
    fn catalog_payload_nf_nominal_full_288_and_per_module_a_sets() {
        use std::collections::{BTreeMap, BTreeSet};
        let p = catalog_payload();
        let nf = p["nf_nominal"].as_array().expect("catalog_payload 缺 nf_nominal 数组");
        assert_eq!(nf.len(), 288, "NF 名义表应整表下发（p18 144 + p20 39 + p21 49 + p22 56）");
        // 载荷 (m→A 集合) 与 CSV 直读集合逐条一致（值乘 1000 取整，避开浮点键）。
        let key = |v: f64| (v * 1000.0).round() as i64;
        let mut payload: BTreeMap<i64, BTreeSet<i64>> = BTreeMap::new();
        for r in nf {
            let m = key(r["m"].as_f64().expect("nf_nominal[m] 非数字"));
            let a = key(r["a"].as_f64().expect("nf_nominal[a] 非数字"));
            payload.entry(m).or_default().insert(a);
        }
        let mut table: BTreeMap<i64, BTreeSet<i64>> = BTreeMap::new();
        for r in nf_e22141_rows() {
            table.entry(key(r.m)).or_default().insert(key(r.a));
        }
        assert_eq!(payload, table, "载荷 (m,A) 集合应与 CSV 完全一致");
        // 逐 m 的 A 个数清单（用户要求；改表改错在这里报出来）。
        let want_counts: [(i64, usize); 10] = [
            (500, 9),
            (750, 9),
            (1000, 10),
            (1250, 11),
            (1667, 11),
            (2500, 17),
            (3750, 21),
            (5000, 24),
            (7500, 15),
            (10000, 17),
        ];
        for (m, n) in want_counts {
            let got = payload.get(&m).map(|s| s.len()).unwrap_or(0);
            assert_eq!(got, n, "m={} 的 A 候选应 {n} 个（实际 {got}）", m as f64 / 1000.0);
        }
        let all_a: BTreeSet<i64> = payload.values().flatten().copied().collect();
        assert_eq!(all_a.len(), 48, "全部 A 候选去重后应 48 个");
        // 完整链路：`/api/parts` 顶层 `spline_engine` 里也必须是 288 条（前端 `involNfNominal` 的输入）。
        let cat: serde_json::Value =
            serde_json::from_str(&crate::partgen::catalog_json()).expect("catalog_json 应是 JSON");
        let via_api = cat["spline_engine"]["nf_nominal"]
            .as_array()
            .expect("catalog_json.spline_engine.nf_nominal");
        assert_eq!(via_api.len(), 288, "catalog_json 的 spline_engine.nf_nominal 应 288 条");
        // DIN 回归对照：名义表 721 行整表下发（前端 dinDbList 的输入）。
        let din = cat["spline_engine"]["din_nominal"].as_array().expect("din_nominal");
        assert_eq!(din.len(), din5480_rows().len(), "DIN 名义表应整表下发");
    }

    /// 查表：各 m 抽样命中（含 p18/p20 重复行的去重）、A+m / A+z 推导、未命中文案。
    #[test]
    fn nf_e22141_lookup_hits_each_module_and_derivations() {
        // (m, A, N, 表 x，None = p18 行由公式解）：p20 m=0.5/A=10/N=18；m=1.25/A=12/N=8/x=0.6；
        // p21 m=1.667/A=45/N=25（d OCR 修正）；m=2.5/A=100/N=38；m=3.75/A=80/N=19/x=0.967；
        // p22 m=5/A=105/N=19（z OCR 修正）；m=7.5/A=240/N=30/x=0.8（互换修正）；m=10/A=160/N=14。
        let cases: [(f64, f64, u32, Option<f64>); 8] = [
            (0.5, 10.0, 18, Some(0.8)),
            (1.25, 12.0, 8, Some(0.6)),
            (1.667, 45.0, 25, Some(0.8)),
            (2.5, 100.0, 38, Some(0.8)),
            (3.75, 80.0, 19, Some(0.967)),
            (5.0, 105.0, 19, Some(0.8)),
            (7.5, 240.0, 30, Some(0.8)),
            (10.0, 160.0, 14, Some(0.8)),
        ];
        for (m, a, z, x) in cases {
            let rows = lookup_by_a(a, Some(m)).unwrap_or_else(|e| panic!("m={m} A={a}：{e}"));
            let r = rows
                .iter()
                .find(|r| r.z == z)
                .unwrap_or_else(|| panic!("m={m} A={a} 应命中 N={z}：{:?}", rows.iter().map(|r| r.z).collect::<Vec<_>>()));
            if let Some(xv) = x {
                assert!((r.x.unwrap() - xv).abs() < 1e-9, "m={m} A={a} x={:?}", r.x);
            }
            // A + m 查表补 N；命中一行且就是该 N。
            let hits = lookup_by_a(a, Some(m)).unwrap();
            assert!(hits.iter().any(|h| h.z == z));
        }
        // p18/p20 重复行去重：A=4 m=0.5 在 p18 与 p20 各一行 → 留信息全的 p20 行（有 x）。
        let r4 = lookup_by_a(4.0, Some(0.5)).unwrap();
        assert_eq!(r4.len(), 1, "同 (m,A,N) 重复行应去重：{r4:?}");
        assert!(r4[0].x.is_some() && r4[0].page == 20, "{r4:?}");
        // 未命中：附近候选 + 列出已入库档位。
        let e = lookup_by_a(81.0, Some(3.75)).unwrap_err();
        assert!(e.contains("附近候选") && e.contains("A=80"), "{e}");
        let e = lookup_by_a(81.0, Some(0.9)).unwrap_err();
        assert!(e.contains("表中没有 m=0.9") && e.contains("已入库档位"), "{e}");
        assert!(lookup_by_a(-1.0, None).unwrap_err().contains("正数"));
        assert!(lookup_by_a(10.0, Some(0.0)).unwrap_err().contains("正数"));
    }

    /// `A + m` / `A + N` 表外推导 + `A` 与输入 x 不符时按 A 提示（D_bOrigin 语义）。
    #[test]
    fn nf_e22141_resolve_derivations_and_adjustments() {
        // 表内 A+m+N：x 由 A 公式解出（A 为主参数），来源 NfTable。
        let (p, o) = resolve_nf_by_a(80.0, Some(3.75), Some(19), None, "NF平齿根").unwrap();
        assert!((p.x - x_from_a(80.0, 3.75, 19)).abs() < 1e-12 && p.a == Some(80.0));
        assert!((p.da() - 80.0).abs() < 1e-9 && (p.df() - (80.0 - 2.4 * 3.75)).abs() < 1e-9);
        assert!(matches!(&o, D_bOrigin::NfTable(r) if r.page == 21), "{o:?}");
        assert!(o.note().contains("查表命中 p21") && o.note().contains("A=80"), "{}", o.note());
        // 表内 A+m：唯一 N 变体 → 直接命中。
        let (p, o) = resolve_nf_by_a(80.0, Some(3.75), None, None, "NF平齿根").unwrap();
        assert_eq!(p.z, 19);
        assert!(matches!(o, D_bOrigin::NfTable(_)));
        // 表内 A+z：唯一 m 变体 → 直接命中。
        let (p, o) = resolve_nf_by_a(80.0, None, Some(19), None, "NF平齿根").unwrap();
        assert!((p.m - 3.75).abs() < 1e-9);
        assert!(matches!(o, D_bOrigin::NfTable(_)));
        // 表外 A+m：按主系列 x=0.8 推 N = A/m − 2。
        let (p, o) = resolve_nf_by_a(81.0, Some(3.75), None, None, "NF平齿根").unwrap();
        assert_eq!(p.z, 20, "81/3.75−2=19.6 → 20");
        assert!((p.x - 0.8).abs() < 1e-12);
        assert!(
            matches!(&o, D_bOrigin::Derived(n) if n.contains("推导值、未命中表") && n.contains("N=20")),
            "{o:?}"
        );
        // 表外 A+z：按 x=0.8 给名义 m = A/(N+2x+0.4)。
        let (p, o) = resolve_nf_by_a(81.0, None, Some(19), None, "NF平齿根").unwrap();
        assert!((p.m - 81.0 / 21.0).abs() < 1e-12, "m={}", p.m);
        assert!((p.x - 0.8).abs() < 1e-12);
        assert!(
            matches!(&o, D_bOrigin::Derived(n) if n.contains("推导值、未命中表") && n.contains("A/(N+2x+0.4)")),
            "{o:?}"
        );
        // A 为主参数：输入 x 与 A 不符 → 按 A 取 x 并明文提示。
        let (p, o) = resolve_nf_by_a(80.0, Some(3.75), Some(19), Some(0.633), "NF平齿根").unwrap();
        assert!((p.x - x_from_a(80.0, 3.75, 19)).abs() < 1e-12, "按 A 取公式解 x");
        assert!(
            matches!(&o, D_bOrigin::Adjusted(n) if n.contains("A 为主参数") && n.contains("与输入 x=0.633 不符")),
            "{o:?}"
        );
        // 只给 A 且多命中 → 列候选要 M/N。
        let e = resolve_nf_by_a(80.0, None, None, None, "NF平齿根").unwrap_err();
        assert!(e.contains("命中") && e.contains("请再给 M"), "{e}");
        // 非法 A/m。
        assert!(resolve_nf_by_a(0.0, Some(2.0), Some(18), None, "NF平齿根").unwrap_err().contains("正数"));
        assert!(resolve_nf_by_a(80.0, Some(-1.0), None, None, "NF平齿根").unwrap_err().contains("正数"));
    }

    /// NF 几何：α=20°/da=A/df=平 A−2.4m、圆 A−2.694m；内花键 D=A−2m；三视图走共用通路。
    #[test]
    fn nf_e22141_geometry_internal_and_views() {
        let (p, _) = resolve_nf_by_a(80.0, Some(3.75), Some(19), None, "NF平齿根").unwrap();
        assert!((p.alpha_deg - 20.0).abs() < 1e-12, "α=20°");
        assert!((p.d() - 3.75 * 19.0).abs() < 1e-9);
        assert!((p.db() - p.d() * (20f64).to_radians().cos()).abs() < 1e-9);
        assert!((p.da() - 80.0).abs() < 1e-9, "外径定心 da=A（ha*=0.2）");
        assert!((p.df() - (80.0 - 2.4 * 3.75)).abs() < 1e-9, "平齿根 df=A−2.4m");
        assert!((p.rho_f() - 0.3 * 3.75).abs() < 1e-9, "R=0.3m");
        let round = "NF圆齿根";
        let (pr, _) = resolve_nf_by_a(80.0, Some(3.75), Some(19), None, round).unwrap();
        assert!((pr.df() - (80.0 - 2.694 * 3.75)).abs() < 1e-9, "圆齿根 df=A−2.694m");
        assert!((pr.rho_f() - 0.528 * 3.75).abs() < 1e-9, "Ri=0.528m");
        // 内花键（外径定心）：大径=A、小径 D=A−2m。
        let pi = p.clone().with_internal(true);
        pi.validate().unwrap();
        assert!((pi.internal_major_dia() - 80.0).abs() < 1e-9, "A₁″=A");
        assert!(
            (pi.internal_minor_dia() - (80.0 - 2.0 * 3.75)).abs() < 1e-9,
            "D=A−2m={}",
            pi.internal_minor_dia()
        );
        // 端视图走共用通路；内花键无侧视图、剖视由 gear.rs 模板出图（引擎 side/section 明确报错）。
        let front = pi.front_view(1.0).unwrap();
        let radii: Vec<f64> = front
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) => Some(a.radius),
                _ => None,
            })
            .collect();
        assert!(radii.iter().any(|r| near(*r, 40.0)), "外侧齿根弧 = A/2");
        assert!(radii.iter().any(|r| near(*r, (80.0 - 7.5) / 2.0)), "里侧齿顶弧 = D/2");
        // 内花键无侧视图（用户定案：同内齿轮）→ 明确报错；剖视走 gear.rs 内齿轮模板。
        let e = pi.side_view(30.0).unwrap_err();
        assert!(e.contains("内花键不提供") && e.contains("侧视图"), "{e}");
        let e = pi.section_view(30.0).unwrap_err();
        assert!(e.contains("内齿轮口径"), "{e}");
        // 外花键侧视 = da/df 口径。
        let ext_side = p.side_view(30.0).unwrap();
        assert!(has_line(&ext_side, [0.0, -40.0], [0.0, 40.0]));
        assert!(has_line(&ext_side, [0.0, -p.df() / 2.0], [30.0, -p.df() / 2.0]));
        p.front_view(1.0).unwrap();
    }

    /// `A`（基准直径槽位）在 GB/ANSI 下报错（各自文案），NF/DIN 允许。
    #[test]
    fn nf_e22141_a_slot_only_allowed_for_nf_and_din() {
        // GB：统一文案；ANSI：径节制文案。
        let e = resolve_spline(SplineStd::GB, "GB30R", Some(66.0), Some(3.0), Some(20), None)
            .unwrap_err();
        assert_eq!(e, gb_d_b_msg(), "{e}");
        let e = resolve_spline(SplineStd::ANSI, "ANSI", Some(66.0), Some(3.0), Some(20), None)
            .unwrap_err();
        assert_eq!(e, ansi_d_b_msg(), "{e}");
        // ANSI：不给 d_B 可正常解析（`m` 槽位 = 径节 P=3）。
        let (pa, oa) = resolve_spline(SplineStd::ANSI, "ANSI", None, Some(3.0), Some(20), None)
            .unwrap();
        assert_eq!(pa.std, SplineStd::ANSI);
        assert_eq!(pa.pitch, Some(3.0));
        assert!(oa.is_none());
        // NF/DIN：同一槽位各自可用（NF=A，DIN=d_B）。
        let (p, o) = resolve_spline(SplineStd::NF, "NF", Some(80.0), Some(3.75), Some(19), None)
            .unwrap();
        assert_eq!(p.a, Some(80.0));
        assert!(matches!(o, Some(D_bOrigin::NfTable(_))));
        let (pd, od) =
            resolve_spline(SplineStd::DIN, "DIN30", Some(40.0), Some(2.0), Some(18), None)
                .unwrap();
        assert_eq!(pd.d_b, Some(40.0));
        assert!(matches!(od, Some(D_bOrigin::Table(_))));
    }

    /// 预设代号/`parse_preset_token`：NF 别名 + NFP/NFR 齿廓。
    #[test]
    fn nf_e22141_preset_tokens_and_codes() {
        assert_eq!(parse_preset_token("NF").unwrap(), (SplineStd::NF, "NF平齿根"));
        assert_eq!(parse_preset_token("NFE22141").unwrap(), (SplineStd::NF, "NF平齿根"));
        assert_eq!(parse_preset_token("NF E22-141").unwrap(), (SplineStd::NF, "NF平齿根"));
        assert_eq!(parse_preset_token("NFP").unwrap(), (SplineStd::NF, "NF平齿根"));
        assert_eq!(parse_preset_token("NFR").unwrap(), (SplineStd::NF, "NF圆齿根"));
        assert_eq!(parse_preset_token("NF圆齿根").unwrap(), (SplineStd::NF, "NF圆齿根"));
        assert_eq!(preset_code(SplineStd::NF, "NF平齿根"), Some("NFP"));
        assert_eq!(preset_code(SplineStd::NF, "NF圆齿根"), Some("NFR"));
        // 预设表：NF 两个齿廓；ANSI 五列（A–E）。
        assert_eq!(SplineStd::NF.presets().len(), 2);
        assert_eq!(SplineStd::ANSI.presets().len(), 5);
    }

    // ── 体系显式标识（本轮核心：不再从 d_B 反推体系）──

    /// 花键四体系 + gear 的 M/DP 解析；NF 可用（A 主参数）/ANSI 可用（P 径节）与 d_B 规则。
    #[test]
    fn explicit_system_identifiers_and_d_b_rules() {
        // 花键体系：GB/DIN/NF/ANSI 都能从标识单独解析（与 GB30R/DIN30 预设代号并存兼容）。
        assert_eq!(parse_preset_token("GB").unwrap().0, SplineStd::GB);
        assert_eq!(parse_preset_token("DIN").unwrap().0, SplineStd::DIN);
        assert_eq!(parse_preset_token("NF").unwrap().0, SplineStd::NF);
        assert_eq!(parse_preset_token("NFE22141").unwrap().0, SplineStd::NF);
        assert_eq!(parse_preset_token("ANSI").unwrap().0, SplineStd::ANSI);
        assert_eq!(parse_preset_token("GB30R").unwrap().0, SplineStd::GB);
        assert_eq!(parse_preset_token("DIN30").unwrap().0, SplineStd::DIN);
        // 体系属性：d_B 只属 DIN/NF。
        assert!(SplineStd::DIN.uses_d_b() && SplineStd::NF.uses_d_b());
        assert!(!SplineStd::GB.uses_d_b() && !SplineStd::ANSI.uses_d_b());

        // GB/ANSI：d_B 一律不允许（GB 沿统一文案；ANSI 径节制文案）。
        let e = resolve_spline(SplineStd::GB, "GB30R", Some(40.0), Some(3.0), Some(20), None)
            .unwrap_err();
        assert_eq!(e, gb_d_b_msg(), "{e}");
        let e = resolve_spline(SplineStd::ANSI, "ANSI", Some(40.0), None, None, None).unwrap_err();
        assert_eq!(e, ansi_d_b_msg(), "{e}");
        assert!(e.contains("d_B") && e.contains("径节") && !e.contains("未实现"), "{e}");

        // NF：已入库 —— A 主参数；A+m+z 走查表，A+m 查表补 N，无 A 时由 m/z/x 正算 A。
        let (p, origin) =
            resolve_spline(SplineStd::NF, "NF", Some(80.0), Some(3.75), Some(19), None).unwrap();
        assert_eq!(p.std, SplineStd::NF);
        assert_eq!(p.a, Some(80.0));
        assert!((p.x - x_from_a(80.0, 3.75, 19)).abs() < 1e-12, "A 主参数 → x 由公式解");
        assert!(matches!(&origin, Some(D_bOrigin::NfTable(r)) if r.page == 21), "{origin:?}");
        let (p, origin) =
            resolve_spline(SplineStd::NF, "NF", Some(80.0), Some(3.75), None, None).unwrap();
        assert_eq!(p.z, 19);
        assert!(matches!(origin, Some(D_bOrigin::NfTable(_))), "{origin:?}");
        let (p, origin) =
            resolve_spline(SplineStd::NF, "NF", None, Some(3.75), Some(19), None).unwrap();
        assert!(matches!(origin, Some(D_bOrigin::ComputedA)), "{origin:?}");
        assert!(
            (p.a.unwrap() - 3.75 * (19.0 + 1.6 + 0.4)).abs() < 1e-9,
            "A={:?}",
            p.a
        );

        // ANSI：可用（第 5 槽 = 径节 P；无 d_B 来源，恒 None）。
        let (p, origin) =
            resolve_spline(SplineStd::ANSI, "ANSI", None, Some(8.0), Some(20), None).unwrap();
        assert_eq!(p.std, SplineStd::ANSI);
        assert_eq!(p.pitch, Some(8.0));
        // P8,N20 → 2.5 in = 63.5 mm（内部/输出 mm）。
        assert!(origin.is_none() && (p.d() - 63.5).abs() < 1e-12, "D={}", p.d());
        assert!(p.spec().contains("P8/Ps16"), "{}", p.spec());

        // 合法路径不被误拦：GB/DIN 正常出参数。
        let (p, origin) =
            resolve_spline(SplineStd::GB, "GB30R", None, Some(3.0), Some(20), None).unwrap();
        assert_eq!(p.std, SplineStd::GB);
        assert!(origin.is_none() && p.d_b.is_none());
        let (p, origin) =
            resolve_spline(SplineStd::DIN, "DIN30", None, Some(2.0), Some(18), Some(0.2)).unwrap();
        assert_eq!(p.std, SplineStd::DIN);
        assert!(matches!(origin, Some(D_bOrigin::Computed)) && p.d_b.is_some());
    }

    // ── ANSI B92.1-1970 (R1993)（径节 17 项 + Table 2 五列公式）──

    /// 入库径节系列：17 项、Ps=2P、与 p08/p09/p11 三处互证的关键值逐项吻合。
    #[test]
    fn ansi_pitch_series_17_matches_standard() {
        let rows = ansi_pitches();
        assert_eq!(rows.len(), 17, "径节系列应为 17 项");
        let want: [(f64, &str); 17] = [
            (2.5, "2.5/5"),
            (3.0, "3/6"),
            (4.0, "4/8"),
            (5.0, "5/10"),
            (6.0, "6/12"),
            (8.0, "8/16"),
            (10.0, "10/20"),
            (12.0, "12/24"),
            (16.0, "16/32"),
            (20.0, "20/40"),
            (24.0, "24/48"),
            (32.0, "32/64"),
            (40.0, "40/80"),
            (48.0, "48/96"),
            (64.0, "64/128"),
            (80.0, "80/160"),
            (128.0, "128/256"),
        ];
        for (r, (p, label)) in rows.iter().zip(want) {
            assert!((r.p - p).abs() < 1e-12, "P {}", r.p);
            assert!((r.ps - 2.0 * p).abs() < 1e-12, "Ps {}", r.ps);
            assert_eq!(r.label, label);
        }
    }

    /// 径节写法：接受 `A/B`（B==2A）与裸数字（17 项系列内）；系列外报错把 17 项按 A/B 列出；
    /// 重复 `ANSI B92.1：` 前缀已修。
    #[test]
    fn ansi_pitch_ab_parse_and_series_errors() {
        // A/B 与裸数字（系列内）
        assert!((parse_ansi_pitch("2.5/5").unwrap() - 2.5).abs() < 1e-12);
        assert!((parse_ansi_pitch("128/256").unwrap() - 128.0).abs() < 1e-12);
        assert!((parse_ansi_pitch("8").unwrap() - 8.0).abs() < 1e-12);
        assert!((parse_ansi_pitch(" 5 / 10 ").unwrap() - 5.0).abs() < 1e-12);
        // B != 2A：明确说明 Ps 恒为 2P，并给出应写形式
        let e = parse_ansi_pitch("5/11").unwrap_err();
        assert!(e.contains("Ps 恒为 2P") && e.contains("5/10"), "{e}");
        // 系列外（裸数字 / 合法 A/B）：列 17 项 A/B
        let e = parse_ansi_pitch("2").unwrap_err();
        assert!(
            e.contains("不在标准系列") && e.contains("2.5/5") && e.contains("128/256")
                && e.contains("Ps 恒为 2P"),
            "{e}"
        );
        let e = parse_ansi_pitch("7/14").unwrap_err();
        assert!(e.contains("不在标准系列") && e.contains("3/6"), "{e}");
        // 语法错：空/非数/多斜杠/非正
        for bad in ["", "abc", "5/10/15", "0/0", "-2"] {
            assert!(parse_ansi_pitch(bad).is_err(), "{bad} 应报错");
        }
        // 重复前缀修复：resolve_spline 的系列错误只带一个 `ANSI B92.1：`
        let e = resolve_spline(SplineStd::ANSI, "ANSI30P", None, Some(2.0), Some(20), None)
            .unwrap_err();
        assert_eq!(e.matches("ANSI B92.1：").count(), 1, "{e}");
        assert!(e.contains("标准系列"), "{e}");
        // InvolParams::ansi 同样只有一层前缀，且合法系列值可用
        let e = InvolParams::ansi("ANSI30平齿根齿侧", 2.0, 20).unwrap_err();
        assert_eq!(e.matches("ANSI B92.1：").count(), 1, "{e}");
        let ok = InvolParams::ansi("ANSI30平齿根齿侧", 5.0, 20).unwrap();
        assert_eq!(ok.pitch, Some(5.0));
        assert!((ok.m - 25.4 / 5.0).abs() < 1e-12);
    }

    /// Table 2 五列公式（P=16、N=30 同时覆盖 5 列；cF 取 min 0.002 in 夹取）。
    /// 标准公式/印出值是英寸；引擎输出 mm，断言时 ÷25.4 回英寸再比。
    #[test]
    fn ansi_table2_basic_formulas_and_columns() {
        use std::f64::consts::PI;
        const IN: f64 = ANSI_INCH_MM;
        let (p, z) = (16.0, 30u32);
        let m_in = 1.0 / p;
        let (n, d_in, c_in) = (z as f64, z as f64 / p, ansi_c_f(z as f64 * IN / p) / IN);
        assert!((c_in - 0.002).abs() < 1e-12, "cF 下夹取");
        // (profile, α, Dri k, Di k, Dre k, DFi 手工式)
        let cases: [(&str, f64, f64, f64, f64, Option<f64>); 5] = [
            ("ANSI30平齿根齿侧", 30.0, 1.35, 1.0, 1.35, None),
            ("ANSI30平齿根外径", 30.0, 1.0, 1.0, 1.35, Some(0.8)),
            ("ANSI30圆齿根齿侧", 30.0, 1.8, 1.0, 2.0, None),
            ("ANSI37.5圆齿根齿侧", 37.5, 1.6, 0.8, 1.3, None),
            ("ANSI45圆齿根齿侧", 45.0, 1.4, 0.6, 1.0, None),
        ];
        for (profile, alpha, dri_k, di_k, dre_k, dfi_b) in cases {
            let q = InvolParams::ansi(profile, p, z).unwrap();
            assert_eq!(q.ansi_column(), AnsiColumn::from_profile(profile));
            assert!((q.alpha_deg - alpha).abs() < 1e-12, "{profile} α");
            assert!((q.d() / IN - d_in).abs() < 1e-12, "{profile} D");
            assert!(
                (q.db() / IN - d_in * alpha.to_radians().cos()).abs() < 1e-12,
                "{profile} Db"
            );
            assert!((q.da() / IN - (n + 1.0) / p).abs() < 1e-12, "{profile} Do");
            assert!((q.c_f() / IN - c_in).abs() < 1e-12, "{profile} cF");
            assert!(
                (q.internal_major_dia() / IN - (n + dri_k) / p).abs() < 1e-12,
                "{profile} Dri"
            );
            assert!(
                (q.internal_minor_dia() / IN - (n - di_k) / p).abs() < 1e-12,
                "{profile} Di"
            );
            assert!((q.df() / IN - (n - dre_k) / p).abs() < 1e-12, "{profile} Dre");
            let dfi_in = match dfi_b {
                Some(k) => (n + k) / p - 0.004 + 2.0 * c_in,
                None => (n + 1.0) / p + 2.0 * c_in,
            };
            assert!(
                (q.ansi_form_dia_internal() / IN - dfi_in).abs() < 1e-12,
                "{profile} DFi"
            );
            assert!(
                (q.ansi_form_dia_external() / IN - ((n - di_k) / p - 2.0 * c_in)).abs() < 1e-12,
                "{profile} DFe"
            );
            // Sv min / 基本齿厚 t = p − Sv
            let k = if (alpha - 37.5).abs() < 1e-9 {
                0.1
            } else if (alpha - 45.0).abs() < 1e-9 {
                0.2
            } else {
                0.0
            };
            let sv_in = (0.5 * PI + k) * m_in;
            assert!((q.ansi_sv_min() / IN - sv_in).abs() < 1e-12, "{profile} Sv");
            assert!((q.s() / IN - (PI * m_in - sv_in)).abs() < 1e-12, "{profile} t");
            assert!((q.s() + q.ansi_sv_min() - PI * q.m).abs() < 1e-12, "{profile} t+Sv=p");
        }
    }

    /// 回归：`ANSI-B92.1_抽样校验.csv` 全 73 行复算（容差 6e-5；含 128/256 印 0.0246 的存疑行）。
    /// 语料是标准印出的**英寸值**；断言时把引擎 mm 输出 ÷25.4 回英寸再比。
    #[test]
    fn ansi_sample_check_csv_all_73_rows() {
        use std::f64::consts::PI;
        const SAMPLE: &str = include_str!("../assets/ansi_b921_sample_check.csv");
        const IN: f64 = ANSI_INCH_MM;
        let (mut checked, mut suspect) = (0usize, 0usize);
        let mut max_res = 0.0f64;
        for (i, line) in SAMPLE.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let f: Vec<&str> = line.split(',').map(str::trim).collect();
            if f.first().map(|s| *s == "row_id").unwrap_or(false) {
                continue;
            }
            assert!(f.len() >= 11, "第 {} 行字段不足", i + 1);
            let p: f64 = f[4].parse().unwrap_or_else(|_| panic!("第 {} 行 P", i + 1));
            let alpha_raw = f[6];
            let quantity = f[7];
            let printed: f64 = f[8].parse().unwrap_or_else(|_| panic!("第 {} 行印出值", i + 1));
            // 印出值只有 Ps/p/Sv 三种量（都与齿数无关，z 任取 20）。
            // Table 3 与 Table 2 口径区别（详见 ansi_sv_min_formula 注释）：
            //  * Ps/p 与齿廓列无关，按 P 选一个列界覆盖它的列构造参数即可；
            //  * Sv min 走 Table 3 公式 ansi_sv_min_formula —— 17 项径节全印，不受 Table 2 列界约束。
            let alpha = if alpha_raw.starts_with("37.5") {
                37.5
            } else if alpha_raw.starts_with("45") {
                45.0
            } else {
                30.0
            };
            // 引擎输出 mm：Ps 无量纲（原值）；p=π·m；Sv min 为长度。全部 ÷25.4 回英寸。
            let got_in = if quantity.starts_with("Ps") || quantity.starts_with("p ") {
                // Ps/p 与列无关：选一个适用径节含 p 的列（A 2.5–32 / C 2.5–48 / E 10–128）。
                let profile = if p <= 32.0 + 1e-9 {
                    "ANSI30平齿根齿侧"
                } else if p <= 48.0 + 1e-9 {
                    "ANSI30圆齿根齿侧"
                } else {
                    "ANSI45圆齿根齿侧"
                };
                let q = InvolParams::ansi(profile, p, 20)
                    .unwrap_or_else(|e| panic!("第 {} 行（{profile} P={p}）：{e}", i + 1));
                if quantity.starts_with("Ps") {
                    2.0 * q.ansi_p()
                } else {
                    PI * q.m / IN
                }
            } else if quantity.starts_with("Sv") {
                ansi_sv_min_formula(alpha, p) / IN
            } else {
                panic!("第 {} 行未知量「{quantity}」", i + 1);
            };
            let res = (printed - got_in).abs();
            max_res = max_res.max(res);
            assert!(
                res <= 6e-5,
                "第 {} 行 {} P={} 残差 {:.3e}（印 {} vs 算 {}）",
                i + 1,
                quantity,
                p,
                res,
                printed,
                got_in
            );
            if res > 5e-5 {
                suspect += 1;
                assert!(
                    (p - 128.0).abs() < 1e-12 && quantity.contains("circular"),
                    "只有 128/256 圆周齿距行是存疑印刷行：第 {} 行 {}",
                    i + 1,
                    quantity
                );
            }
            checked += 1;
        }
        assert_eq!(checked, 73, "抽样校验应有 73 数据行");
        assert_eq!(suspect, 1, "存疑行（重排版印 0.0246）应恰好 1 行");
        assert!((5.0e-5..=6.0e-5).contains(&max_res), "max_res={max_res}");
    }

    /// `Dre` 三段分界（12/24、16/32、10/20）与 `cF` 的 max/min 夹取。
    /// 标准分界/印出值是英寸；引擎输出 mm，断言时 ÷25.4 回英寸。
    #[test]
    fn ansi_dre_segments_and_cf_clamps() {
        const IN: f64 = ANSI_INCH_MM;
        let z = 20u32;
        // 30°圆齿根 C 列：P=12 用 (N−1.8)/P；P=16 起用 (N−2)/P。
        let c12 = InvolParams::ansi("ANSI30圆齿根齿侧", 12.0, z).unwrap();
        assert!((c12.df() / IN - (z as f64 - 1.8) / 12.0).abs() < 1e-12);
        let c16 = InvolParams::ansi("ANSI30圆齿根齿侧", 16.0, z).unwrap();
        assert!((c16.df() / IN - (z as f64 - 2.0) / 16.0).abs() < 1e-12);
        // A/B 列不随径节分段。
        for p in [2.5, 12.0, 16.0, 32.0] {
            let a = InvolParams::ansi("ANSI30平齿根齿侧", p, z).unwrap();
            assert!((a.df() / IN - (z as f64 - 1.35) / p).abs() < 1e-12, "A P={p}");
        }
        // D 列也不分段。
        for p in [2.5, 16.0, 48.0] {
            let d = InvolParams::ansi("ANSI37.5圆齿根齿侧", p, z).unwrap();
            assert!((d.df() / IN - (z as f64 - 1.3) / p).abs() < 1e-12, "D P={p}");
        }
        // 45° E 列：第三段 (N−1)/P，10/20 及更细都适用。
        for p in [10.0, 16.0, 128.0] {
            let e = InvolParams::ansi("ANSI45圆齿根齿侧", p, z).unwrap();
            assert!((e.df() / IN - (z as f64 - 1.0) / p).abs() < 1e-12, "E P={p}");
        }
        // 45° 在 P<10 不适用（Table 2 E 范围 10/20–128/256）。
        let e = InvolParams::ansi("ANSI45圆齿根齿侧", 8.0, z).unwrap_err();
        assert!(e.contains("10/20"), "{e}");
        // cF 夹取（入参/返回 mm）：min 0.002 in（D<2 in）、中段 0.001·D、max 0.010 in（D>10 in）。
        assert!((ansi_c_f(1.0 * IN) - 0.002 * IN).abs() < 1e-12);
        assert!((ansi_c_f(5.0 * IN) - 0.005 * IN).abs() < 1e-12);
        assert!((ansi_c_f(100.0 * IN) - 0.010 * IN).abs() < 1e-12);
        let small = InvolParams::ansi("ANSI45圆齿根齿侧", 128.0, 100).unwrap();
        assert!((small.c_f() - 0.002 * IN).abs() < 1e-12, "cF min");
        let mid = InvolParams::ansi("ANSI30平齿根齿侧", 8.0, 40).unwrap(); // D=5 in
        assert!((mid.c_f() - 0.005 * IN).abs() < 1e-12, "cF 中段");
        let big = InvolParams::ansi("ANSI30平齿根齿侧", 2.5, 30).unwrap(); // D=12 in
        assert!((big.c_f() - 0.010 * IN).abs() < 1e-12, "cF max");
    }

    /// 用户定案 A：**Table 2 五列各自适用径节范围**（上下界含端点）在引擎入口强制；
    /// 越界错误点名「列名 + 范围原文 + 收到的 P/Ps」；原来的 45° 下限特例已并入同一逻辑。
    #[test]
    fn ansi_table2_columns_enforce_pitch_ranges() {
        // 每列：范围端点（含）必须通过；界外报错文案含列名、范围原文与「低于下限/超出上限」。
        for (col, lo, hi) in [
            (AnsiColumn::A30FlatSide, 2.5, 32.0),
            (AnsiColumn::B30FlatMajor, 3.0, 16.0),
            (AnsiColumn::C30FilletSide, 2.5, 48.0),
            (AnsiColumn::D375FilletSide, 2.5, 48.0),
            (AnsiColumn::E45FilletSide, 10.0, 128.0),
        ] {
            assert_eq!(col.pitch_range(), (lo, hi), "{col:?} 范围");
            ansi_column_pitch_check(col, lo).unwrap();
            ansi_column_pitch_check(col, hi).unwrap();
            for (bad, side) in [(lo - 1.0, "低于下限"), (hi + 1.0, "超出上限")] {
                let e = ansi_column_pitch_check(col, bad).unwrap_err();
                assert!(
                    e.contains("ANSI B92.1 Table 2")
                        && e.contains(col.desc())
                        && e.contains(col.range_label())
                        && e.contains(side),
                    "{col:?} P={bad}: {e}"
                );
            }
        }
        // A 列（30° 平齿根 / 齿侧配合）：32/64 通过、40/80 拒（超上限）；2.5/5 通过、2/4 拒（系列外）。
        assert!(InvolParams::ansi("ANSI30平齿根齿侧", 32.0, 20).is_ok());
        let e = InvolParams::ansi("ANSI30平齿根齿侧", 40.0, 20).unwrap_err();
        assert!(
            e.contains("30° 平齿根 / 齿侧配合")
                && e.contains("2.5/5 — 32/64")
                && e.contains("40/80")
                && e.contains("超出上限"),
            "{e}"
        );
        assert!(InvolParams::ansi("ANSI30平齿根齿侧", 2.5, 20).is_ok());
        let e = InvolParams::ansi("ANSI30平齿根齿侧", 2.0, 20).unwrap_err();
        assert!(e.contains("不在标准系列") && e.contains("2.5/5"), "{e}");
        // B 列（30° 平齿根 / 外径配合）：16/32 通过、20/40 拒；下限 3/6 通过、2.5/5 拒（在系列内 → 列范围报错）。
        assert!(InvolParams::ansi("ANSI30平齿根外径", 16.0, 20).is_ok());
        let e = InvolParams::ansi("ANSI30平齿根外径", 20.0, 20).unwrap_err();
        assert!(
            e.contains("30° 平齿根 / 外径配合")
                && e.contains("3/6 — 16/32")
                && e.contains("20/40")
                && e.contains("超出上限"),
            "{e}"
        );
        assert!(InvolParams::ansi("ANSI30平齿根外径", 3.0, 20).is_ok());
        let e = InvolParams::ansi("ANSI30平齿根外径", 2.5, 20).unwrap_err();
        assert!(
            e.contains("3/6 — 16/32") && e.contains("2.5/5") && e.contains("低于下限"),
            "{e}"
        );
        // C/D 列（30°/37.5° 圆齿根）：48/96 通过、64/128 拒（超上限）。
        for profile in ["ANSI30圆齿根齿侧", "ANSI37.5圆齿根齿侧"] {
            assert!(InvolParams::ansi(profile, 48.0, 20).is_ok(), "{profile}");
            let e = InvolParams::ansi(profile, 64.0, 20).unwrap_err();
            assert!(
                e.contains("圆齿根 / 齿侧配合")
                    && e.contains("2.5/5 — 48/96")
                    && e.contains("64/128")
                    && e.contains("超出上限"),
                "{profile}: {e}"
            );
        }
        // E 列（45° 圆齿根）：10/20 通过、8/16 拒；128/256 上端通过；>128（非系列值）直查也要点名范围。
        assert!(InvolParams::ansi("ANSI45圆齿根齿侧", 10.0, 20).is_ok());
        assert!(InvolParams::ansi("ANSI45圆齿根齿侧", 128.0, 20).is_ok());
        let e = InvolParams::ansi("ANSI45圆齿根齿侧", 8.0, 20).unwrap_err();
        assert!(
            e.contains("45° 圆齿根 / 齿侧配合")
                && e.contains("10/20 — 128/256")
                && e.contains("8/16")
                && e.contains("低于下限"),
            "{e}"
        );
        let e = ansi_column_pitch_check(AnsiColumn::E45FilletSide, 200.0).unwrap_err();
        assert!(e.contains("128/256") && e.contains("超出上限"), "{e}");
    }

    /// Table 3 的 `Sv min` 不被 Table 2 每列范围约束：17 项径节全印，公式直算
    /// （30° 在 A 列上界 32 之外仍有印出值）；与列几何入口的拒绝形成对照。
    #[test]
    fn ansi_table3_sv_min_is_not_gated_by_table2_columns() {
        use std::f64::consts::PI;
        for p in [2.5, 32.0, 40.0, 48.0, 64.0, 128.0] {
            let got = ansi_sv_min_formula(30.0, p) / ANSI_INCH_MM;
            assert!((got - 0.5 * PI / p).abs() < 1e-12, "30° P={p}");
        }
        assert!(
            (ansi_sv_min_formula(37.5, 128.0) / ANSI_INCH_MM - (0.5 * PI + 0.1) / 128.0).abs()
                < 1e-12
        );
        assert!(
            (ansi_sv_min_formula(45.0, 2.5) / ANSI_INCH_MM - (0.5 * PI + 0.2) / 2.5).abs() < 1e-12
        );
        // 对照：同 P=40 走列几何入口会被 A 列的 Table 2 范围拒绝（Table 2 ≠ Table 3 口径）。
        assert!(InvolParams::ansi("ANSI30平齿根齿侧", 40.0, 20).is_err());
    }

    /// 单位口径：标准英制 → 引擎 mm（1 in = 25.4 mm；D = N/P in → ×25.4）。
    #[test]
    fn ansi_mm_unit_magnitude() {
        use std::f64::consts::PI;
        // 任务书点名量级：P=16,N=30 → 47.625 mm；P=8,N=30 → 95.25 mm。
        let p16 = InvolParams::ansi("ANSI30平齿根齿侧", 16.0, 30).unwrap();
        assert!((p16.m - 25.4 / 16.0).abs() < 1e-12, "m={}", p16.m);
        assert!((p16.d() - 47.625).abs() < 1e-12, "D={}", p16.d());
        assert!(
            p16.spec().contains("P16/Ps32 N30（节圆 φ47.625）"),
            "{}",
            p16.spec()
        );
        let p8 = InvolParams::ansi("ANSI30平齿根齿侧", 8.0, 30).unwrap();
        assert!((p8.d() - 95.25).abs() < 1e-12, "D={}", p8.d());
        // 圆周齿距 p = π/P in → π·m mm。
        assert!((PI * p16.m - PI * 25.4 / 16.0).abs() < 1e-12);
        // cF 夹取边界（mm）：D<2 in → 0.0508；D>10 in → 0.254；中段 0.001·D_mm。
        // P=128 只在 45° E 列（10/20–128/256）范围内；cF 只与 D=N/P 有关，与列无关。
        let small = InvolParams::ansi("ANSI45圆齿根齿侧", 128.0, 30).unwrap(); // D=0.234 in
        assert!((small.c_f() - 0.0508).abs() < 1e-12, "cF min: {}", small.c_f());
        let mid = InvolParams::ansi("ANSI30平齿根齿侧", 8.0, 40).unwrap(); // D=5 in=127 mm
        assert!((mid.c_f() - 0.127).abs() < 1e-12, "cF mid: {}", mid.c_f());
        let big = InvolParams::ansi("ANSI30平齿根齿侧", 2.5, 30).unwrap(); // D=12 in
        assert!((big.c_f() - 0.254).abs() < 1e-12, "cF max: {}", big.c_f());
        // DFi(B) 的 −0.004 in = −0.1016 mm。
        let b = InvolParams::ansi("ANSI30平齿根外径", 8.0, 30).unwrap();
        let want = (30.0 + 0.8) * 25.4 / 8.0 - 0.1016 + 2.0 * b.c_f();
        assert!(
            (b.ansi_form_dia_internal() - want).abs() < 1e-12,
            "DFi={}",
            b.ansi_form_dia_internal()
        );
    }

    /// 压力角/齿根型式/配合方式 → 预设名（非法组合拒绝）；五列代号往返。
    #[test]
    fn ansi_profile_name_and_tokens() {
        assert_eq!(
            ansi_profile_name(30.0, AnsiRoot::Flat, AnsiFit::Side).unwrap(),
            "ANSI30平齿根齿侧"
        );
        assert_eq!(
            ansi_profile_name(30.0, AnsiRoot::Flat, AnsiFit::MajorDia).unwrap(),
            "ANSI30平齿根外径"
        );
        assert_eq!(
            ansi_profile_name(30.0, AnsiRoot::Fillet, AnsiFit::Side).unwrap(),
            "ANSI30圆齿根齿侧"
        );
        assert_eq!(
            ansi_profile_name(37.5, AnsiRoot::Fillet, AnsiFit::Side).unwrap(),
            "ANSI37.5圆齿根齿侧"
        );
        assert_eq!(
            ansi_profile_name(45.0, AnsiRoot::Fillet, AnsiFit::Side).unwrap(),
            "ANSI45圆齿根齿侧"
        );
        assert!(ansi_profile_name(37.5, AnsiRoot::Flat, AnsiFit::Side).is_err());
        assert!(ansi_profile_name(45.0, AnsiRoot::Flat, AnsiFit::Side).is_err());
        assert!(ansi_profile_name(30.0, AnsiRoot::Fillet, AnsiFit::MajorDia).is_err());
        assert!(ansi_profile_name(20.0, AnsiRoot::Fillet, AnsiFit::Side).is_err());
        // 五列代号往返。
        for pr in SplineStd::ANSI.presets() {
            let code = preset_code(SplineStd::ANSI, pr.profile).expect("有代号");
            assert_eq!(
                parse_preset_token(code),
                Some((SplineStd::ANSI, pr.profile)),
                "{code}"
            );
        }
        assert_eq!(
            parse_preset_token("ANSI B92.1"),
            Some((SplineStd::ANSI, ANSI_DEFAULT_PROFILE))
        );
        assert_eq!(
            parse_preset_token("ANSI30PM").unwrap().1,
            "ANSI30平齿根外径"
        );
        assert_eq!(
            parse_preset_token("ANSI37.5R").unwrap().1,
            "ANSI37.5圆齿根齿侧"
        );
        assert_eq!(
            parse_preset_token("ANSI45圆齿根").unwrap().1,
            "ANSI45圆齿根齿侧"
        );
        assert_eq!(preset_code(SplineStd::ANSI, "ANSI30P").is_none(), true);
    }

    /// `resolve_spline` ANSI 路径：P/N 必给、x 拒绝、`d_B` 文案；端视/内花键走共用通路。
    #[test]
    fn ansi_resolve_errors_and_front_view() {
        let (p, o) =
            resolve_spline(SplineStd::ANSI, "ANSI30P", None, Some(8.0), Some(20), None).unwrap();
        assert_eq!(p.std, SplineStd::ANSI);
        assert_eq!(p.pitch, Some(8.0));
        assert!(o.is_none());
        assert!(p.spec().contains("P8/Ps16") && p.spec().contains("N20"), "{}", p.spec());
        assert!(p.spec().contains("（节圆 φ63.5）"), "{}", p.spec());
        // P8,N20 → 2.5 in = 63.5 mm（引擎内部/输出 mm）。
        assert!((p.d() - 63.5).abs() < 1e-12, "D={}", p.d());
        // d_B/A → ANSI_D_B_MSG（不含「未实现」字样）。
        let e = resolve_spline(SplineStd::ANSI, "ANSI", Some(40.0), Some(8.0), Some(20), None)
            .unwrap_err();
        assert_eq!(e, ansi_d_b_msg(), "{e}");
        assert!(!e.contains("未实现"), "{e}");
        // 缺 P / 缺 N。
        let e = resolve_spline(SplineStd::ANSI, "ANSI", None, None, Some(20), None).unwrap_err();
        assert!(e.contains("缺径节 P"), "{e}");
        let e = resolve_spline(SplineStd::ANSI, "ANSI", None, Some(8.0), None, None).unwrap_err();
        assert!(e.contains("缺齿数 N"), "{e}");
        // 变位 x 拒绝。
        let e =
            resolve_spline(SplineStd::ANSI, "ANSI", None, Some(8.0), Some(20), Some(0.1)).unwrap_err();
        assert!(e.contains("不使用变位系数"), "{e}");
        // 端视：齿根弧、齿顶弧、研开线起点按 DFe。
        let front = p.front_view(1.0).unwrap();
        assert!(front.iter().any(|x| matches!(x, EntityType::Arc(a) if near(a.radius, p.df() / 2.0))));
        assert!(front.iter().any(|x| matches!(x, EntityType::Arc(a) if near(a.radius, p.da() / 2.0))));
        assert!(near(p.r_involute_start(), p.ansi_form_dia_external() / 2.0));
        // 侧视/剖视走共用通路（矩形 4 + 小径 2 + 轴线 1 + 分度圆 2 = 9）。
        assert_eq!(p.side_view(30.0).unwrap().len(), 9);
        assert_eq!(p.section_view(30.0).unwrap().len(), 9);
        // 内花键走共用通路：外侧齿根弧 = Dri/2、里侧齿顶弧 = Di/2。
        let pi = p.clone().with_internal(true);
        pi.validate().unwrap();
        let fi = pi.front_view(1.0).unwrap();
        assert!(fi.iter().any(|x| matches!(x, EntityType::Arc(a) if near(a.radius, pi.internal_major_dia() / 2.0))));
        assert!(fi.iter().any(|x| matches!(x, EntityType::Arc(a) if near(a.radius, pi.internal_minor_dia() / 2.0))));
        // 过渡圆角（无标准数值依据）：圆心半径 rf+ρ 且到 form 起点距离 = ρ
        // （rr² = rs² + ρ² − 2·ρ·rs·sinα_s 的余弦定理口径）。
        let rho = p.rho_f();
        assert!(rho > 0.0, "rho={rho}");
        let rs = p.ansi_form_dia_external() / 2.0;
        let rf = p.df() / 2.0;
        let rb = p.db() / 2.0;
        let sin_a = ((rs * rs - rb * rb).max(0.0)).sqrt() / rs;
        let want = (rs * rs - rf * rf) / (2.0 * (rf + rs * sin_a));
        assert!((rho - want).abs() < 1e-12, "{rho} vs {want}");
        assert!((rf + rho) * (rf + rho) > 0.0);
    }

    /// 计算书四要素齐全：公式（`m·z`）+ 代入数值（`3 × 20`）+ 结果（`60 mm`）
    /// + 依据来源（`GB/T 3478.1`），并含 5 个小节骨架。
    #[test]
    fn report_gb_contains_formula_substitution_result_and_source() {
        let _g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        let p = InvolParams::gb("30圆齿根", 3.0, 20).unwrap();
        let md = build_report(&p, None, 30.0);
        for needle in ["m·z", "3 × 20", "60 mm", "GB/T 3478.1"] {
            assert!(md.contains(needle), "计算书缺 `{needle}`：\n{md}");
        }
        for section in [
            "## 1. 输入参数",
            "## 2. 逐步计算",
            "## 3. 派生几何",
            "## 4. 检验尺寸",
            "## 5. 数据来源与校验",
        ] {
            assert!(md.contains(section), "计算书缺小节 `{section}`：\n{md}");
        }
        assert!(md.contains("内花键") || md.contains("外花键"), "{md}");
    }

    /// ⑧ 批：计算书 / 卡类型报错 / 检验查表随语言切换，且**数据与符号两语一致**。
    #[test]
    fn report_and_lookup_switch_language_and_keep_data() {
        use crate::i18n::{set_lang, set_lang_auto, Lang};
        let _g = crate::global_state_test_lock();
        crate::i18n::clear_missing_keys();

        // ── zh：GB 计算书四要素 + 头部 ──
        set_lang(Lang::Zh);
        let p = InvolParams::gb("30圆齿根", 3.0, 20).unwrap();
        let zh = build_report(&p, None, 30.0);
        for needle in [
            "# 渐开线花键计算书",
            "## 1. 输入参数",
            "| 3 × 20 | 60 mm |",
            "GB/T 3478.1-2008 表 3（d = mz）",
            "## 5. 数据来源与校验",
            "恒等式自检",
        ] {
            assert!(zh.contains(needle), "zh 计算书缺 `{needle}`：\n{zh}");
        }
        // ── en：同参数计算书，结构与数据在、句子为英文 ──
        set_lang(Lang::En);
        let en = build_report(&p, None, 30.0);
        for needle in [
            "# Involute spline computation report",
            "## 1. Input parameters",
            "| 3 × 20 | 60 mm |",
            "GB/T 3478.1-2008 Table 3 (d = mz)",
            "## 5. Data sources and checks",
            "Identity self-check",
        ] {
            assert!(en.contains(needle), "en report missing `{needle}`:\n{en}");
        }
        assert!(!en.contains("渐开线花键计算书") && !en.contains("输入参数"), "{en}");

        // ── 四体系 × 两语：模板占位符必须全部替换（不漏 `{vN}` / `{}`）──
        let mut reports: Vec<(String, String)> = vec![("GB-zh".to_string(), zh.clone())];
        for (tag, std, profile, m, z) in [
            ("DIN", SplineStd::DIN, "DIN30", 3.0, 20u32),
            ("NF", SplineStd::NF, "NF平齿根", 3.75, 19),
            ("ANSI", SplineStd::ANSI, "ANSI30P", 8.0, 20),
        ] {
            let (pp, oo) = resolve_spline(std, profile, None, Some(m), Some(z), None).unwrap();
            set_lang(Lang::Zh);
            reports.push((format!("{tag}-zh"), build_report(&pp, oo.as_ref(), 30.0)));
            set_lang(Lang::En);
            reports.push((format!("{tag}-en"), build_report(&pp, oo.as_ref(), 30.0)));
        }
        for (tag, md) in &reports {
            assert!(
                !md.contains("{v") && !md.contains("{}"),
                "{tag} 计算书有未替换占位符：\n{md}"
            );
        }
        for (tag, md) in &reports {
            if tag.ends_with("-en") {
                assert!(
                    !md.contains("输入参数") && !md.contains("渐开线花键计算书"),
                    "{tag} 仍出中文小节：\n{md}"
                );
            }
        }
        set_lang(Lang::Zh);

        // ── spec()：内花键头随语言；标准号/齿廓名/数值原样 ──
        let pi = p.clone().with_internal(true);
        set_lang(Lang::Zh);
        let sz = pi.spec();
        set_lang(Lang::En);
        let se = pi.spec();
        assert!(sz.starts_with("内花键 GB "), "{sz}");
        assert!(se.starts_with("Internal spline GB "), "{se}");
        for (s, tag) in [(&sz, "zh"), (&se, "en")] {
            assert!(
                s.contains("30圆齿根") && s.contains("m3") && s.contains("z20"),
                "{tag} spec：{s}"
            );
        }

        // ── 几何报错（齿顶变尖）：zh/en 双文案，角度值两语一致 ──
        let sharp = InvolParams::gb("30圆齿根", 3.0, 20)
            .unwrap()
            .with_coeffs(9.0, 0.9, 0.4, 0.1);
        set_lang(Lang::Zh);
        let ez = sharp.front_view(1.0).unwrap_err();
        set_lang(Lang::En);
        let ee = sharp.front_view(1.0).unwrap_err();
        assert!(ez.contains("齿顶变尖") && ez.contains("°"), "{ez}");
        assert!(ee.contains("becomes pointed") && ee.contains("°"), "{ee}");

        // ── DIN 检验查表（未命中 → 附近候选）＋ invol_check 缺参：双向 ──
        set_lang(Lang::Zh);
        let dz = lookup_inspection(41.0, Some(0.9)).unwrap_err();
        let iz = inspection_json("db=40&m=2").unwrap_err();
        set_lang(Lang::En);
        let de = lookup_inspection(41.0, Some(0.9)).unwrap_err();
        let ie = inspection_json("db=40&m=2").unwrap_err();
        assert!(dz.contains("无命中") && dz.contains("附近候选"), "{dz}");
        assert!(de.contains("no hit") && de.contains("Nearby candidates"), "{de}");
        assert!(iz.contains("缺 z") && iz.contains("?db=40&m=2&z=18"), "{iz}");
        assert!(ie.contains("missing z") && ie.contains("?db=40&m=2&z=18"), "{ie}");

        set_lang_auto();
        assert!(
            crate::i18n::missing_keys().is_empty(),
            "{:?}",
            crate::i18n::missing_keys()
        );
    }

    /// 渐开线跨距公式唯一实现：齿轮模板 m2 z40 α20 → k=5、W≈27.6896；
    /// gear 的 `span_measurement` 与之一字不差（ANSI/NF 计算书同调此函数）。
    #[test]
    fn involute_span_shared_formula_matches_known_cases() {
        let (k, w) = involute_span(2.0, 40, 20.0, 0.0);
        assert_eq!(k, 5, "模板跨测齿数");
        assert!((w - 27.6896).abs() < 5e-4, "模板公法线 W={w}");
        let p = crate::gear::GearParams {
            m: 2.0,
            z: 40,
            ..Default::default()
        };
        assert_eq!(p.span_measurement(), Some((k, w)), "gear 必须复用同一实现");
        // α 通用性：30° z=20 → k=round(20×30/180+0.5)=4。
        assert_eq!(involute_span(3.0, 20, 30.0, 0.0).0, 4);
        // 变位项 2xm·sinα：x=0.8 时 W 增大且与手算一致。
        let (_, w0) = involute_span(7.5, 38, 20.0, 0.0);
        let (_, w8) = involute_span(7.5, 38, 20.0, 0.8);
        assert!((w8 - w0 - 2.0 * 0.8 * 7.5 * 20f64.to_radians().sin()).abs() < 1e-9);
    }

    /// ANSI 计算书补 Wn/Kn 公式推导：式 → 代入 → 结果 → 来源；
    /// 并明确标注「原标准检验表本仓未收」（不冒充原表值）。
    #[test]
    fn report_ansi_derives_wn_kn_with_formula_substitution() {
        let _g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        let (p, origin) =
            resolve_spline(SplineStd::ANSI, "ANSI30P", None, Some(8.0), Some(20), None).unwrap();
        let md = build_report_card(&p, origin.as_ref());
        for needle in [
            "### 检验量公式推导（Wn / Kn；渐开线几何）",
            "Kn = round(N·α/180° + 0.5)",
            "Wn = m·cosα·[(Kn−0.5)·π + N·invα] + 2·x·m·sinα",
            "渐开线几何推导",
            "ANSI B92.1 原标准检验表（公法线/跨测表）本仓未收",
            "与 GB/T 3478.6 式(11)",
        ] {
            assert!(md.contains(needle), "ANSI 计算书缺 `{needle}`：\n{md}");
        }
        let (kn, wn) = involute_span(p.m, p.z, p.alpha_deg, p.x);
        assert!(md.contains(&format!("Kn = {kn}")), "{md}");
        assert!(md.contains(&format!("Wn = {} mm", trim(wn))), "{md}");
        // 卡片计算书入口不带「有效长度」行（卡片无该输入）。
        assert!(!md.contains("有效长度"), "卡片计算书不应打印有效长度：\n{md}");
    }

    /// DIN 计算书：d_B 反推口径（721 行全表校验）+ 检验尺寸的查表命中页/source
    /// + 检验表全表对照结论；取一行真实检验表行保证查表命中。
    #[test]
    fn report_din_carries_bench_formula_and_inspection_source() {
        let _g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        let row = inspection_rows()
            .iter()
            .find(|r| (r.m - 1.0).abs() < 1e-9)
            .expect("m=1 检验行")
            .clone();
        let x = x_from_d_b(row.d_b, row.m, row.z);
        let (p, origin) = resolve_din_by_d_b(row.d_b, Some(row.m), Some(row.z), Some(x)).unwrap();
        let md = build_report(&p, Some(&origin), 30.0);
        assert!(md.contains("d_B = d + 1.1m + 2x₁m"), "{md}");
        assert!(md.contains("721 行残差 0"), "{md}");
        assert!(md.contains("## 4. 检验尺寸"), "{md}");
        assert!(
            md.contains(&format!("p{} 表{}", row.page, row.table_no))
                && md.contains(&format!("source={}", row.source)),
            "计算书缺查表页/source（p{} 表{} source={}）：\n{md}",
            row.page,
            row.table_no,
            row.source
        );
        assert!(md.contains("W_k") && md.contains("M1") && md.contains("M2"), "{md}");
        assert!(
            md.contains("检验表逐行对照") && md.contains("验证通过"),
            "{md}"
        );
    }
    // ── 2026-09 审计回归：跨体系核心量 / 内花键降级 / 报告口径 / 全表对账 ──

    /// 图元坐标键（跨体系逐条比较用；1e-9 以内视为同值）。
    fn entity_key(e: &EntityType) -> String {
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
            other => format!("{other:?}"),
        }
    }

    /// 跨体系数学核心必须逐值相等：同 (m,z,x,α) 下 `d/db/s/ψ(R)`（GB 30° 与 DIN 30°）；
    /// 系数对齐后 `da/df/起始圆/终止圆/GB 表 3 form 直径` 与端视图图元也逐条相等。
    #[test]
    fn cross_system_involute_core_is_identical() {
        let (m, z, x) = (4.0, 15u32, 0.3);
        let gb = InvolParams::gb("30平齿根", m, z).unwrap().with_x(x);
        let din = InvolParams::din(m, z, x).unwrap();
        let d = m * z as f64;
        let alpha = 30f64.to_radians();
        for p in [&gb, &din] {
            assert!(near(p.d(), d), "d");
            assert!(near(p.db(), d * alpha.cos()), "db");
            assert!(
                near(p.s(), std::f64::consts::PI * m / 2.0 + 2.0 * x * m * alpha.tan()),
                "s"
            );
        }
        // ψ(R) 在渐开线区间内逐值相等（含 R<db 的夹取口径）
        for r in [p_small(&gb), gb.db() / 2.0, 30.0, 35.0, 40.0, 45.0] {
            assert!(
                near(gb.half_tooth_angle(r), din.half_tooth_angle(r)),
                "ψ({r}) 跨体系不等"
            );
        }
        // 系数对齐（GB30平齿根 → DIN30 的 ha/hf/ρf/cF）后所有导出几何量一致。
        let gb_aligned = gb.clone().with_coeffs(0.45, 0.55, 0.16, 0.10);
        for (a, b, name) in [
            (gb_aligned.da(), din.da(), "da"),
            (gb_aligned.df(), din.df(), "df"),
            (gb_aligned.r_involute_start(), din.r_involute_start(), "起始圆"),
            (gb_aligned.r_involute_end(), din.r_involute_end(), "终止圆"),
            (gb_aligned.gb_form_dia_max(), din.gb_form_dia_max(), "form 直径"),
        ] {
            assert!(near(a, b), "{name}: GB {a} ≠ DIN {b}");
        }
        // 端视图图元逐条一致（393 条）。
        let eg = gb_aligned.front_view(1.0).unwrap();
        let ed = din.front_view(1.0).unwrap();
        assert_eq!(eg.len(), ed.len(), "端视图图元数");
        for (i, (a, b)) in eg.iter().zip(ed.iter()).enumerate() {
            assert_eq!(entity_key(a), entity_key(b), "端视图第 {i} 条图元不一致");
        }
        // NF（α=20°）与直齿轮共用同一 ψ(R)/db/st 口径。
        let nf = InvolParams::from_preset(SplineStd::NF, "NF平齿根", m, z)
            .unwrap()
            .with_x(x);
        let gear = crate::gear::GearParams {
            kind: crate::gear::GearKind::External,
            m,
            z,
            alpha_deg: 20.0,
            ha: 1.0,
            c: 0.25,
            beta_deg: 0.0,
            h: 20.0,
            x,
            spline: None,
            std: crate::gear::GearStd::M,
            dp: None,
        };
        assert!(near(nf.db(), gear.db()), "NF/齿轮 db");
        assert!(near(nf.s(), gear.st()), "NF/齿轮 s");
        for r in [32.0, 35.0, 40.0, 45.0] {
            assert!(
                near(nf.half_tooth_angle(r), gear.half_tooth_angle(r)),
                "NF/齿轮 ψ({r})"
            );
        }
    }

    /// 起始半径辅助（GB 30 平齿根 m4 z15 的 df>db 情形取齿根）。
    fn p_small(p: &InvolParams) -> f64 {
        p.r_involute_start().min(p.db() / 2.0 + 1.0)
    }

    /// 内花键「材料带内无渐开线」降级：基圆超过外侧齿根（GB 45° 小齿数）或 ANSI
    /// `DFi ≥ Dri`（细径节 + cF 下夹取）时，端视图退化为径向直线，不再画反向渐开线。
    #[test]
    fn internal_no_involute_band_degrades_to_radial_lines() {
        // GB 45圆齿根 m10 z4：齿顶高于基圆（rb=14.14 < r_tip=16.81）；GB 内花键
        // D_ei = m(z+1.2) 恒大于 db，不会出现「材料带内无渐开线」——该组锁“正常出渐开线”
        // 这一支不被降级误伤。
        let p = InvolParams::gb("45圆齿根", 10.0, 4)
            .unwrap()
            .with_internal(true);
        p.validate().unwrap();
        assert!(p.db() / 2.0 < p.internal_root_radius());
        let fv = p.front_view(1.0).unwrap();
        assert!(fv.len() > 4 * 4 + 3, "正常内花键应含渐开线段");

        // ANSI P=128、N=200（45° E 列，10/20–128/256）：DFi > Dri（Table 2 公式 + cF 下夹取）。
        let a = InvolParams::ansi("ANSI45圆齿根齿侧", 128.0, 200).unwrap();
        assert!(a.ansi_form_dia_internal() > a.internal_major_dia() + 1e-9);
        let ai = a.clone().with_internal(true);
        let af = ai.front_view(1.0).unwrap();
        assert_eq!(af.len(), 4 * 200 + 3, "ANSI 细径节降级后每齿 4 图元");
        let rr = ai.internal_root_radius();
        for e in &af {
            if let EntityType::Line(l) = e {
                if l.common.layer != LAYER_MAIN {
                    continue;
                }
                assert!(
                    l.start.x.hypot(l.start.y) <= rr + 1e-9
                        && l.end.x.hypot(l.end.y) <= rr + 1e-9,
                    "不得有超出外侧齿根的渐开线端点"
                );
            }
        }
    }

    /// 计算书内花键口径：渐开线有效区间用 `max(D_ii, db)` 与外侧齿根 `D_ei`，
    /// 不再打印外花键的 `d_involute_end（=da）`（与端视图/JSON 同源）。
    #[test]
    fn report_internal_uses_internal_involute_bounds() {
        let _g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        let pi = InvolParams::gb("30圆齿根", 3.0, 20)
            .unwrap()
            .with_internal(true);
        let md = build_report(&pi, None, 30.0);
        assert!(md.contains("渐开线有效起始圆（内花键 max(D_ii, db, DFi)）"), "{md}");
        let start = 2.0 * (pi.internal_minor_dia() / 2.0).max(pi.db() / 2.0);
        assert!(
            md.contains(&format!("| 渐开线有效起始圆（内花键 max(D_ii, db, DFi)） | {} mm |", trim(start))),
            "缺内部起始圆 {start}：\n{md}"
        );
        assert!(
            md.contains(&format!(
                "| 渐开线终止（外侧齿根 D_ei） | {} mm |",
                trim(pi.internal_major_dia())
            )),
            "缺外侧齿根：\n{md}"
        );
        assert!(!md.contains("d_involute_end（=da）"), "内花键不应出现外花键终止圆：\n{md}");
        // 外花键口径不变。
        let pe = InvolParams::gb("30圆齿根", 3.0, 20).unwrap();
        let me = build_report(&pe, None, 30.0);
        assert!(me.contains("d_involute_end（=da）"), "{me}");
        assert!(!me.contains("渐开线有效起始圆（内花键"), "{me}");
    }

    /// NF 尺寸表全 288 行逐行对账（独立复算 d/db/s/sB/齿根圆/R/Ri/h/D，不借用引擎几何）：
    /// 除 9 行「疑原表印误」外残差都在排版精度内（m=1.667=5/3 档放宽到 0.012）。
    #[test]
    fn nf_e22141_all_rows_formula_reconcile() {
        use std::f64::consts::PI;
        let a20 = 20f64.to_radians();
        let inv20 = a20.tan() - a20;
        let is_suspect = |r: &NfE22141Row, col: &str| -> bool {
            NF_SUSPECTED_SOURCE_ERRORS.iter().any(|(p, m, a, z, c, _)| {
                *p == r.page
                    && (*m - r.m).abs() < NF_M_TOL
                    && (*a - r.a).abs() < NF_A_TOL
                    && *z == r.z
                    && (*c).starts_with(col)
            })
        };
        let mut checked = 0usize;
        let mut suspects = 0usize;
        let mut worst = 0.0f64;
        for r in nf_e22141_rows() {
            // p18 行只有 D 列；其余列按需对。
            let x = x_from_a(r.a, r.m, r.z);
            assert!((a_from_x(r.m, r.z, x) - r.a).abs() < 1e-9, "A 反变换");
            // 列容差按源表排版精度：d/dB/s/sB/x ≤3 位小数、p18 的 D 与齿根圆/圆角只印 1~2 位。
            let tol_of = |col: &str| -> f64 {
                let t = match col {
                    "分度圆直径d" => 0.02,
                    "dB(基圆)" => 0.03,
                    "变位系数x" => 0.004,
                    "分度圆弧齿厚s" => 0.004,
                    "sB(基圆弧齿厚)" => 0.004,
                    "外花键齿根圆角半径(平齿根)" | "外花键齿根圆角半径(圆齿根)" => 0.06,
                    "外花键齿顶倒角高度" => 0.06,
                    _ => 0.6, // 平齿根齿根圆/圆齿根齿根圆/D(内花键小径)：只印到 1 位或整数
                };
                t + if (r.m - 1.667).abs() < 1e-6 { 0.01 } else { 0.0 }
            };
            let mut check = |col: &str, got: Option<f64>, want: f64| {                let Some(v) = got else { return };
                let res = (v - want).abs();
                if res > tol_of(col) {
                    assert!(
                        is_suspect(r, col),
                        "p{} m{} A{} N{} {col}：表 {v} vs 算 {want} 残差 {res}",
                        r.page,
                        trim(r.m),
                        trim(r.a),
                        r.z
                    );
                    suspects += 1;
                } else {
                    worst = worst.max(res);
                }
                checked += 1;
            };
            check("分度圆直径d", r.d, r.m * r.z as f64);
            check("dB(基圆)", r.base_dia, r.m * r.z as f64 * a20.cos());
            check("变位系数x", r.x, x);
            check("分度圆弧齿厚s", r.s, r.m * (PI / 2.0 + 2.0 * x * a20.tan()));
            check("sB(基圆弧齿厚)", r.s_b,
                r.m * a20.cos() * (PI / 2.0 + r.z as f64 * inv20 + 2.0 * x * a20.tan()),
            );
            check("平齿根齿根圆", r.flat_root, r.a - 2.4 * r.m);
            check("齿根圆直径(圆齿根)", r.round_root, r.a - 2.694 * r.m);
            check("外花键齿根圆角半径(平齿根)", r.r, 0.3 * r.m);
            check("外花键齿根圆角半径(圆齿根)", r.r_i, 0.528 * r.m);
            check("外花键齿顶倒角高度", r.h, 0.1 * r.m.max(1.0));
            check("D(内花键小径)", r.internal_tip, r.a - 2.0 * r.m);
        }
        assert!(checked >= 288, "应逐列对账（至少 288 格）：{checked}");
        assert_eq!(suspects, 9, "疑原表印误应恰好 9 格（NF_SUSPECTED_SOURCE_ERRORS）");
        // 未命中印误清单的格都在各自列容差内（最宽 0.6 为只印到个位的齿根圆列）。
        assert!(worst <= 0.6, "非印误格最大残差 {worst} 超出排版精度");
    }

    // ── NF E22-141 检查尺寸/公差/配合表（assets/nf_e22141_check.csv）──

    /// 检查表入库形状：399 行 / 10 张表，行数与表名逐张与 `_页清单.md` 一致；
    /// 开头 40 行 `#` 注释与表头都不进数据（页号从 p23 起）；偏差表模数从 `section` 解出。
    #[test]
    fn nf_check_table_shape_and_tables() {
        let rows = nf_check_rows();
        assert_eq!(rows.len(), 399, "入库 399 行（页清单合计）");
        let want: &[(u16, &str, usize)] = &[
            (23, "检查尺寸(m=0.50~1.25)", 39),
            (24, "检查尺寸(m=1.667~3.75)", 49),
            (25, "检查尺寸(m=5.00~10.00)", 56),
            (26, "计算标准-设计尺寸(m=1.00)", 48),
            (27, "计算标准-检查尺寸(m=1.00)", 49),
            (29, "公差值-xm的变动值(微米)", 14),
            (31, "配合表(m=0.50~1.25)", 39),
            (32, "检查尺寸的公差值(m=1.667~2.50)", 28),
            (33, "检查尺寸的公差值(m=3.75~5.00)", 45),
            (35, "F,F1,G,G1的偏差值(微米)", 32),
        ];
        let tables = nf_check_tables();
        assert_eq!(tables.len(), want.len(), "10 张表：{tables:?}");
        for (i, (p, name, n)) in want.iter().enumerate() {
            assert_eq!(tables[i].0, *p, "第 {i} 张表页号");
            assert_eq!(tables[i].1, *name, "第 {i} 张表名");
            assert_eq!(tables[i].2, *n, "第 {i} 张表行数");
        }
        // `#` 注释/表头不得混入（注释 40 行全在数据前；页清单合计 399）。
        assert!(rows.iter().all(|r| r.page >= 23 && !r.table_no.starts_with('#')));
        // 分类计数：检查尺寸 193（p23/24/25/27）+ 设计尺寸 48（p26）+ 偏差 158（p29/31/32/33/35）。
        assert_eq!(rows.iter().filter(|r| r.has_check_dims()).count(), 193);
        assert_eq!(rows.iter().filter(|r| r.page == 26).count(), 48);
        assert_eq!(rows.iter().filter(|r| r.has_deviations()).count(), 158);
        // flags 原样保留：页清单总计 201 行带 flags（p23–p29 的 merged/recheck 也计入）。
        assert_eq!(rows.iter().filter(|r| !r.flags.is_empty()).count(), 201);
        // 1 处与主尺寸表同源的粘连错格在解析期修正（CSV 原值不动）。
        assert_eq!(
            rows.iter().map(|r| r.fixes.len()).sum::<usize>(),
            NF_CHECK_OCR_FIXES.len(),
            "OCR 修正应逐条生效：{NF_CHECK_OCR_FIXES:?}"
        );
        let p25_105 = rows
            .iter()
            .find(|r| r.page == 25 && r.a == Some(105.0))
            .expect("p25 A=105");
        assert_eq!(p25_105.n, Some(19), "N 192→19（与主表 p22 同一错格）");
        assert_eq!(p25_105.fixes, vec!["N 192→19".to_string()]);
        // 检查尺寸表模数在 `m` 列；偏差表在 `section` 末尾的 `(m=X)`。
        let p23 = rows.iter().find(|r| r.page == 23).expect("p23 首行");
        assert_eq!(p23.m, Some(0.5));
        assert!(!p23.m_from_section);
        let p31 = rows.iter().find(|r| r.page == 31).expect("p31 首行");
        assert_eq!(p31.m, Some(0.5));
        assert!(p31.m_from_section, "偏差表模数应从 section 解出：{p31:?}");
        // 页清单把 p31–p35 拆成左右子表：按 `section` 逐个子表核行数。
        let section_rows = |section: &str| rows.iter().filter(|r| r.section == section).count();
        for (section, n) in [
            ("m=0.50|m=0.75:L(m=0.5)", 9),
            ("m=0.50|m=0.75:R(m=0.75)", 9),
            ("m=1.00|m=1.25:L(m=1.0)", 10),
            ("m=1.00|m=1.25:R(m=1.25)", 11),
            ("m=1.667|m=2.50:L(m=1.667)", 11),
            ("m=1.667|m=2.50:R(m=2.5)", 17),
            ("m=3.75|m=5.00:L(m=3.75)", 21),
            ("m=3.75|m=5.00:R(m=5.0)", 24),
            ("m=7.50|m=10.00:L(m=7.5)", 15),
            ("m=7.50|m=10.00:R(m=10.0)", 17),
        ] {
            assert_eq!(section_rows(section), n, "子表 {section} 行数");
        }
        // 列名逐列照抄源表头（解析按列位，列序变了必须报错）。
        let header = NF_E22141_CHECK_CSV
            .lines()
            .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .expect("CSV 表头");
        let cols = split_csv_line(header);
        assert_eq!(cols.len(), 45, "45 列：{cols:?}");
        let want_cols = [
            "page", "table_no", "source", "section", "m", "A", "N", "B", "x", "d", "dB",
            "s", "sB", "Rf", "Rr", "B2", "h", "ri", "K", "E", "U", "F", "F1", "V",
            "V1", "G", "G1", "q1", "q2", "q3", "q4", "q5", "q6", "q7", "dev1",
            "dev2", "dev3", "dev4", "dev5", "dev6", "dev7", "dev8", "dev9", "dev10",
            "flags",
        ];
        for (i, (got, want)) in cols.iter().zip(want_cols).enumerate() {
            assert_eq!(got.trim(), want, "第 {i} 列名");
        }
    }

    /// 抽样查询：p23/p26/p27 检查行、p31 配合行、p29 范围 A 行都能按 (A,m)/(A,N) 命中。
    #[test]
    fn nf_check_lookup_hits_samples() {
        // p23 m=0.50 A=4 N=6：检查尺寸 K/E/U/F/V/V1/G/G1。
        let p23 = nf_check_by_a_m(4.0, 0.5)
            .into_iter()
            .find(|r| r.page == 23)
            .expect("p23 A=4 m=0.5");
        assert_eq!(p23.k, Some(2.0));
        assert!((p23.e.unwrap() - 2.530).abs() < 1e-9);
        assert!((p23.u.unwrap() - 0.90).abs() < 1e-9);
        assert!((p23.f.unwrap() - 4.702).abs() < 1e-9);
        assert!((p23.v.unwrap() - 1.0).abs() < 1e-9);
        assert!((p23.v1.unwrap() - 0.84).abs() < 1e-9);
        assert!((p23.g1.unwrap() - 2.033).abs() < 1e-9);
        // (A,N) 查询不限 m。
        assert!(nf_check_by_a_n(4.0, 6)
            .iter()
            .any(|r| r.page == 23 && r.m == Some(0.5)));
        // p26 m=1.00 A=8 N=6 设计尺寸。
        let p26 = nf_check_by_a_m(8.0, 1.0)
            .into_iter()
            .find(|r| r.page == 26)
            .expect("p26 A=8 m=1");
        assert!((p26.b.unwrap() - 5.6).abs() < 1e-9);
        assert!((p26.x.unwrap() - 0.800).abs() < 1e-9);
        assert!((p26.d.unwrap() - 6.0).abs() < 1e-9);
        assert!((p26.db.unwrap() - 5.638156).abs() < 1e-9);
        assert!((p26.b2.unwrap() - 5.306).abs() < 1e-9);
        // p27 m=1.00 A=8 N=6：q1..q7 位置列。
        let p27 = nf_check_by_a_m(8.0, 1.0)
            .into_iter()
            .find(|r| r.page == 27)
            .expect("p27 A=8 m=1");
        assert!((p27.e.unwrap() - 5.059463).abs() < 1e-9);
        assert_eq!(p27.q[0], Some(1.0));
        assert!((p27.q[1].unwrap() - 1.462).abs() < 1e-9);
        assert!((p27.q[5].unwrap() - 2.712).abs() < 1e-9);
        // p31 配合表（m 由 section 解出）：A=4 N=6 dev1=+68/0。
        let p31 = nf_check_by_a_m(4.0, 0.5)
            .into_iter()
            .find(|r| r.page == 31)
            .expect("p31 A=4 m=0.5");
        assert_eq!(p31.dev[0], "+68/0");
        assert_eq!(p31.dev[2], "-89/-148");
        // p29 的 A 是直径范围（4~15 含 5）；m=0.50。
        let p29 = nf_check_by_a_m(5.0, 0.5)
            .into_iter()
            .find(|r| r.page == 29)
            .expect("p29 范围 4~15 含 A=5");
        assert_eq!(p29.a_raw, "4~15");
        assert_eq!(p29.dev[0], "+25/0");
        // 范围外/表外无命中（返回空 Vec，由报告侧给覆盖率说明）。
        assert!(nf_check_by_a_m(1000.0, 0.5).is_empty());
        assert!(nf_check_by_a_n(1000.0, 6).is_empty());
    }

    /// 与主尺寸表 `nf_e22141_dims.csv` 按 `(m,A,N)` 交叉：三重命中 312、主表缺 72（p26/p27 的
    /// 中间/次系列）、A 命中但 N 不一致 1（p25，只报告不改）、p29 范围行 14；命中行 d 逐行一致。
    #[test]
    fn nf_check_cross_main_dims() {
        let mut exact = 0usize;
        let mut absent = 0usize;
        let mut n_mismatch = 0usize;
        let mut ranged = 0usize;
        let mut d_checked = 0usize;
        let mut d_bad = 0usize;
        let mut mismatch_list = Vec::new();
        for r in nf_check_rows() {
            let (Some(a), Some(n)) = (r.a, r.n) else {
                ranged += 1;
                continue;
            };
            let Some(m) = r.m else { continue };
            let hits: Vec<&NfE22141Row> = nf_e22141_rows()
                .iter()
                .filter(|d| {
                    (d.m - m).abs() < NF_M_TOL && (d.a - a).abs() < NF_A_TOL && d.z == n
                })
                .collect();
            match hits.first() {
                Some(_) => {
                    exact += 1;
                    // 同一 (m,A,N) 可能有 p18（只有 D 列）与 p20/p21/p22（整行）两条：
                    // 带 d 的那条对 d。
                    if let Some(cd) = r.d {
                        let ds: Vec<f64> = hits.iter().filter_map(|d| d.d).collect();
                        if !ds.is_empty() {
                            d_checked += 1;
                            if !ds.iter().any(|md| (cd - md).abs() <= 5e-4) {
                                d_bad += 1;
                            }
                        }
                    }
                }
                None => {
                    let a_hit = nf_e22141_rows().iter().any(|d| {
                        (d.m - m).abs() < NF_M_TOL && (d.a - a).abs() < NF_A_TOL
                    });
                    if a_hit {
                        n_mismatch += 1;
                        mismatch_list.push(format!(
                            "p{} m={} A={} N={}",
                            r.page,
                            trim(m),
                            trim(a),
                            n
                        ));
                    } else {
                        absent += 1;
                    }
                }
            }
        }
        assert_eq!(ranged, 14, "p29 的 A 为范围/列表");
        assert_eq!(exact, 312, "与主尺寸表 (m,A,N) 三重命中");
        assert_eq!(absent, 72, "p26/p27 的中间系列 A 在主表没有");
        assert_eq!(n_mismatch, 1, "A 命中但 N 不一致：{}", mismatch_list.join("；"));
        assert_eq!(d_checked, 12, "检查表带 d 的命中行（p26 两块）");
        assert_eq!(d_bad, 0, "命中的 d 应逐行一致");
        // p24 m=1.667 A=55 的印刷 N=31 被 OCR 吃成 3：解析期按 NF_CHECK_OCR_FIXES 修正后
        // 与主尺寸表一致，故不再计入不一致；CSV 原值 3 保留。
        let p24_55 = nf_check_rows()
            .into_iter()
            .find(|r| r.page == 24 && r.m == Some(1.667) && r.a == Some(55.0))
            .expect("p24 m=1.667 A=55");
        assert_eq!(p24_55.n, Some(31), "看图裁决 N 3→31");
        assert!(p24_55.fixes.iter().any(|f| f == "N 3→31"), "{:?}", p24_55.fixes);
        // p25 m=7.50 A=180 印刷 N=35 保留原值（只报告）：该行夹在 A=260 与 A=300 之间，
        // A 疑为原版误印，N 不动。
        assert!(mismatch_list
            .iter()
            .any(|s| s.contains("p25") && s.contains("7.5") && s.contains("A=180")));
    }

    /// NF 计算书第 4 节：输出检查尺寸（K/E/U/F…）+ p26 设计尺寸 + 偏差行（微米，上/下）
    /// + 页/表/source 来源；表外档明确报“没有该档”而不是静默。
    #[test]
    fn report_nf_carries_check_dimensions_and_source() {
        let _g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        // m=3.75 A=80 N=19：p24 检查尺寸 + p33/p29 偏差行命中。
        let (p, origin) = resolve_nf_by_a(80.0, Some(3.75), Some(19), None, "NF平齿根").unwrap();
        let md = build_report(&p, Some(&origin), 30.0);
        assert!(md.contains("## 4. 检验尺寸"), "{md}");
        assert!(md.contains("NF E22-141 检查表命中"), "{md}");
        assert!(md.contains("assets/nf_e22141_check.csv"), "{md}");
        // 验收③：K/W 公式推导（式→代入→结果→来源）+ 与表值残差。
        for needle in [
            "### 检验量公式推导（K / W）",
            "K = 检查表 K 列",
            "W = m·cos20°·[(K−0.5)·π + N·inv20°] + 2·x·m·sin20°",
            "表值 E=",
            "残差",
        ] {
            assert!(md.contains(needle), "NF 计算书缺 `{needle}`：\n{md}");
        }
        for needle in [
            "p24",
            "检查尺寸(m=1.667~3.75)",
            "source=p24",
            "跨齿数 K",
            "K 齿公法线 E",
            "外花键跨量棒距 F",
            "p33",
            "检查尺寸的公差值(m=3.75~5.00)",
            "p29",
            "dev1=",
        ] {
            assert!(md.contains(needle), "NF 计算书缺 `{needle}`：\n{md}");
        }
        // m=1.0 A=8 N=6：p23 检查尺寸 + p26 设计尺寸 + p27 检查尺寸(q 列) + p31 配合表 + p29 范围。
        let (p1, _) = resolve_nf_by_a(8.0, Some(1.0), Some(6), None, "NF平齿根").unwrap();
        let md1 = build_report(&p1, None, 30.0);
        for needle in [
            "p23",
            "检查尺寸(m=0.50~1.25)",
            "计算标准-设计尺寸(m=1.00)",
            "计算标准-检查尺寸(m=1.00)",
            "配合表(m=0.50~1.25)",
            "q1（表头未辨认）",
            "dev1=+68/0",
            "source=p31",
        ] {
            assert!(md1.contains(needle), "NF 计算书缺 `{needle}`：\n{md1}");
        }
        // 表外档（A=1000 超出 p29 各范围）→ 明确“没有该档”。
        let (p2, _) = resolve_nf_by_a(1000.0, Some(3.75), None, None, "NF平齿根").unwrap();
        let md2 = build_report(&p2, None, 30.0);
        assert!(md2.contains("NF E22-141 检查表没有该档"), "{md2}");
    }

    /// DIN 名义表全 721 行：`e₂/s₁`、`d_b=d·cos30°`、`d_B=d+1.1m+2x₁m` 三式逐行对账
    /// （含 m=5 两处 z 修正；容差按表值排版精度）。
    #[test]
    fn din5480_all_rows_e2_and_bench_identity() {
        let tol_e2 = 1e-3; // e₂ 印 3~4 位小数
        let mut n = 0usize;
        let mut viol: Vec<String> = Vec::new();
        for (i, line) in DIN5480_2_CSV.lines().enumerate() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let f = split_csv_line(line);
            if f.first().map(|s| s.trim()) == Some("page") {
                continue;
            }
            let page = f[0].trim().parse::<u16>().expect("page");
            let m = csv_decimal(&f[1]).expect("m");
            let table_no = f[2].trim().parse::<u32>().expect("table_no");
            let d_b = csv_decimal(&f[3]).expect("d_B");
            let z = din5480_m5_fix_z(page, table_no, d_b, f[4].trim().parse::<u32>().expect("z"));
            let d = csv_decimal(&f[5]).expect("d");
            let base = csv_decimal(&f[6]).expect("d_b");
            let xm = csv_decimal(&f[7]).expect("x1_m");
            let e2 = csv_decimal(&f[8]).expect("e2_s1");
            n += 1;
            let a = 30f64.to_radians();
            for (name, res) in [
                ("e₂", e2 - (m * std::f64::consts::PI / 2.0 + 2.0 * xm * a.tan())),
                ("d_b", base - d * a.cos()),
                ("d_B", d_b - (d + 1.1 * m + 2.0 * xm)),
            ] {
                // d_b 列排版精度较粗（个别行只印 2 位小数），用 0.05 口径与既有测试一致。
                let t = if name == "d_b" { 0.05 } else if name == "e₂" { tol_e2 } else { 1e-9 };
                if res.abs() > t {
                    viol.push(format!("第 {} 行 p{} {name} 残差 {res}", i + 1, page));
                }
            }
        }
        assert_eq!(n, 721, "全表 721 行");
        assert!(viol.is_empty(), "{} 条违例：{viol:?}", viol.len());
    }

    /// 小齿数外部渐开线降级：`rb > rf` 时从齿根圆到基圆补径向直线（GB 30° z=6）；
    /// 奇数齿相位：齿心线 `(k+0.5)·360/z`、齿槽心线 `k·360/z`（z=21 逐条核）。
    #[test]
    fn external_small_z_radial_fallback_and_odd_z_phase() {
        let p = InvolParams::gb("30圆齿根", 2.0, 6).unwrap();
        assert!(p.db() / 2.0 > p.df() / 2.0 + 1e-9, "该组应走 rb>rf 降级");
        let fv = p.front_view(1.0).unwrap();
        // 每齿 2×12 渐开线折线 + 2 条径向直线 + 2 弧 = 28；加 分度圆 1 + 中心线 2。
        assert_eq!(fv.len(), 6 * (2 * INVOLUTE_SEGMENTS + 4) + 3);
        let (rf, rb) = (p.df() / 2.0, p.db() / 2.0);
        let th = p.half_tooth_angle(rb);
        let c = p.pitch_angle() * 0.5;
        let a = [rf * (c + th).cos(), rf * (c + th).sin()];
        let b = [rb * (c + th).cos(), rb * (c + th).sin()];
        assert!(has_line(&fv, a, b), "应有齿根→基圆的径向直线");

        let odd = InvolParams::gb("30圆齿根", 3.0, 21).unwrap();
        let e = odd.front_view(1.0).unwrap();
        assert_eq!(e.len(), 21 * (2 * INVOLUTE_SEGMENTS + 2) + 3);
        let tips = arc_centers_at(&e, odd.da() / 2.0);
        let roots = arc_centers_at(&e, odd.df() / 2.0);
        assert_eq!((tips.len(), roots.len()), (21, 21));
        // 齿心线 = (k+0.5)·360/21 = 8.5714 + k·17.1429（规范化后比对）
        let pitch = 360.0 / 21.0;
        for k in 0..21usize {
            let want_tip = (pitch * (k as f64 + 0.5)).rem_euclid(360.0);
            assert!(
                tips.iter().any(|(c, _)| (c - want_tip).abs() < 1e-9),
                "z=21 齿顶弧中心 {want_tip} 缺"
            );
            let want_root = (pitch * k as f64).rem_euclid(360.0);
            assert!(
                roots.iter().any(|(c, _)| (c - want_root).abs() < 1e-9),
                "z=21 齿根弧中心 {want_root} 缺"
            );
        }
    }

    /// 体系不支持的参数必须明确拒绝（不静默忽略）：跨体系齿廓代号、GB/ANSI 给 d_B/A、
    /// ANSI 给 x、ANSI 45° 低径节、GB/DIN/NF 给径节 —— 各自点名理由与来源。
    #[test]
    fn cross_system_parameter_rejections() {
        // 齿廓代号与体系不符。
        let e = resolve_spline(SplineStd::DIN, "GB30R", None, Some(2.0), Some(18), None)
            .unwrap_err();
        assert!(e.contains("属于") && e.contains("GB/T 3478") && e.contains("DIN 5480"), "{e}");
        let e = resolve_spline(SplineStd::GB, "DIN30", None, Some(3.0), Some(20), None)
            .unwrap_err();
        assert!(e.contains("属于") && e.contains("DIN 5480") && e.contains("GB/T 3478"), "{e}");
        // GB 给 d_B/A。
        let e = resolve_spline(SplineStd::GB, "GB30R", Some(40.0), Some(3.0), Some(20), None)
            .unwrap_err();
        assert_eq!(e, gb_d_b_msg(), "{e}");
        // ANSI 给 d_B/A、给 x、45° 低径节。
        let e = resolve_spline(SplineStd::ANSI, "ANSI30P", Some(40.0), Some(8.0), Some(20), None)
            .unwrap_err();
        assert_eq!(e, ansi_d_b_msg(), "{e}");
        let e = resolve_spline(SplineStd::ANSI, "ANSI30P", None, Some(8.0), Some(20), Some(0.1))
            .unwrap_err();
        assert!(e.contains("不使用变位系数"), "{e}");
        let e = InvolParams::ansi("ANSI45圆齿根齿侧", 8.0, 20).unwrap_err();
        assert!(e.contains("10/20") && e.contains("128/256"), "{e}");
        // NF α 不可覆盖（引擎桥 + CLI 两侧；引擎级见 gear 测试）；validate 本身只查范围。
        let e = InvolParams::from_preset(SplineStd::NF, "NF平齿根", 3.75, 19)
            .unwrap()
            .with_alpha_deg(60.0)
            .validate()
            .unwrap_err();
        assert!(e.contains("压力角") && e.contains("10°<α<50°"), "{e}");
    }

    /// 族③b（部分）：渐开线花键入口层（径节解析 + resolve_spline）—— zh/en 双语断言 + 数据原样。
    #[test]
    fn invol_entry_messages_switch_language_keeping_data() {
        let _g = crate::global_state_test_lock();
        crate::i18n::clear_missing_keys();

        // ── zh ──
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        let e = parse_ansi_pitch_syntax("").unwrap_err();
        assert!(e.contains("缺径节 P") && e.contains("A/B"), "{e}");
        let e = parse_ansi_pitch_syntax("abc").unwrap_err();
        assert!(e.contains("不是数字") && e.contains("abc"), "{e}");
        let e = parse_ansi_pitch_syntax("2.5/4").unwrap_err();
        assert!(e.contains("stub pitch") && e.contains("2.5/4") && e.contains("2.5/5"), "{e}");
        let e = parse_ansi_pitch("7").unwrap_err();
        assert!(e.contains("不在标准系列") && e.contains("17 项"), "{e}");
        let e = resolve_spline(SplineStd::GB, "", Some(40.0), Some(3.0), Some(20), None).unwrap_err();
        assert!(e.contains("基准直径 d_B") && e.contains("GB/T 3478"), "{e}");
        let e = resolve_spline(SplineStd::GB, "", None, Some(3.0), Some(20), Some(0.2)).unwrap_err();
        assert!(e.contains("变位系数 x") && e.contains("0.2"), "{e}");
        let e = resolve_spline(SplineStd::ANSI, "", Some(40.0), Some(8.0), Some(20), None).unwrap_err();
        assert!(e.contains("不使用基准直径"), "{e}");

        // ── en：同一批调用，数据原样 ──
        crate::i18n::set_lang(crate::i18n::Lang::En);
        let e = parse_ansi_pitch_syntax("").unwrap_err();
        assert!(e.contains("missing pitch P") && e.contains("A/B"), "{e}");
        let e = parse_ansi_pitch_syntax("abc").unwrap_err();
        assert!(e.contains("is not a number") && e.contains("abc"), "{e}");
        let e = parse_ansi_pitch_syntax("2.5/4").unwrap_err();
        assert!(e.contains("stub pitch") && e.contains("2.5/4") && e.contains("2.5/5"), "{e}");
        let e = parse_ansi_pitch("7").unwrap_err();
        assert!(e.contains("not in the standard series") && e.contains("17 items"), "{e}");
        let e = resolve_spline(SplineStd::GB, "", Some(40.0), Some(3.0), Some(20), None).unwrap_err();
        assert!(e.contains("Reference diameter d_B") && e.contains("GB/T 3478"), "{e}");
        let e = resolve_spline(SplineStd::GB, "", None, Some(3.0), Some(20), Some(0.2)).unwrap_err();
        assert!(e.contains("Profile shift x") && e.contains("0.2"), "{e}");
        let e = resolve_spline(SplineStd::ANSI, "", Some(40.0), Some(8.0), Some(20), None).unwrap_err();
        assert!(e.contains("does not use reference diameter"), "{e}");
        assert!(
            crate::i18n::missing_keys().is_empty(),
            "缺词条：{:?}",
            crate::i18n::missing_keys()
        );
        crate::i18n::set_lang_auto();
    }
}
