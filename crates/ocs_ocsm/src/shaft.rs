//! **轴生成器**：行 DSL / JSON → 单视图侧视图（`OCSMSHAFT`）。
//!
//! 本文件只做三件事：**解析**、**段拼接几何**、**落层/放置**。
//! 不标尺寸、不打剖面线、不建块；尚未支持的复杂特征（键槽/中心孔）会明确
//! 报「不识别的关键字」并列出当前支持集，不静默忽略。
//!
//! ## 行 DSL（一行一段，从左到右拼接）
//!
//! ```text
//! # 注释行；空行忽略；大小写不敏感；段内关键字顺序无关
//! S30 E30 L45 CH2@L
//! S40 E40 L30 CH2@R OV3
//! S50 E30 L20
//! S30 E30 L15 CH2@R
//! S40 E40 L7 M1.5
//! S36 E36 L5
//! GEAR M3 Z20
//! SPLINE 6x23x26x6 L30
//! ```
//!
//! - `S` 起始直径（靠左）、`E` 终点直径（省略 = 圆柱段，`E=S`）、`L` 段长（必给）；
//! - `CH2@L` / `CH2@R`：端面倒角 C2，贴该段左/右端（`@` 省略默认右端）；
//! - `OV` / `OV3` / `OV3@L`：磨外圆砂轮越程槽（GB/T 6403.5），不写值 = 按该端
//!   直径查表；`@` 省略默认右端；
//! - `M` / `M1.5`：**螺纹段标记**（外螺纹侧视图）。不写值 = 简化画法小径 0.85d；
//!   `M1.5` = 给螺距 P，小径 = d − 1.0825P（精确）；只能标在圆柱段（S=E）上，
//!   与 `OV`/`GEAR` 同段报错、可与 `CH` 同段；
//! - `M1.5 TL20`：**局部螺纹**（GB/T 3-1997 图 1 第一种形式）——完整螺纹长 20，
//!   靠**段右端台肩**，自右向左布置：台肩面（既有端面线）→ 锥面过渡（长 a−x）
//!   → 螺尾（细实线斜线，长 x）→ 分界竖线 → 完整螺纹（长 TL）；
//!   `RO一般|RO短` = 收尾 x 档（默认一般）、`SD一般|SD长|SD短` = 肩距 a 档（默认一般）；
//!   不给 `TL` = 整段全螺纹（旧行为，保持兼容）；
//! - `M1.5 TL20 RL`：用 GB/T 3-1997 表 2 的**退刀槽**收尾（图 2 画法）：
//!   台肩面 → R 圆角 → 槽底 dg/2 长 g1 → 30° 斜壁（g2 处接大径）；
//!   `RL` 与 `RO`/`SD` 互斥（给了 `RL` 就不画收尾/肩距）；
//! - `RL@L` / `RL@R`：**段级退刀槽**（任意圆柱段，贴段左/右端，`@` 省略默认右端；
//!   与 `CH`/`OV` 一样同端只能一个）。画法与上一条同一剖面（GB/T 3 表 2）。
//!   取参优先级：① 段内显式 `g1/g2/dg/r`（`dg` 是**绝对直径**，
//!   例 `RL@L g1 2.5 g2 4.5 dg 22.7 r 0.8`）；② 段内给 `P`（例 `RL@L P1.5`）
//!   → 查表 2（可叠加覆盖）；③ 都不给 → 报错（表 2 以螺距为键，不猜）。
//!   前置：该端相邻段更高（有台肩可退）、本段圆柱；否则报「第 N 段」+ 原因；
//! - `ES5*3` 这类旧写法会报错并指路（退刀槽现在用 `RL` 或小直径轴段表示）；
//! - **齿形段（齿轮/渐开线花键统一表达式，2026-09-23 用户定案）**：
//!   `MARK KIND M… Z… ALPHA… X… DA… DF… BETA… H…`（顺序稳定、解析无序）：
//!   * `MARK` = `GEAR`（齿轮）/ `SPLINE`（渐开线花键）—— **画法开关，不是分类标签**：
//!     决定**常规侧视图**是否画内侧直径（外齿 = 齿根圆；内齿 = 里侧齿顶）的
//!     `2细线层`（青色 ACI 4）细实线：`SPLINE` 画小径细实线 / `GEAR` 不画
//!     （既有齿轮口径「无齿根线」；`gear.rs` `helix_lines_only_for_helical_and_follow_hand_rule` + handbook 16 §九）；
//!   * `KIND` = `EX`/`IN`（外/内；缺省 EX）；`X` = 变位系数（缺省 0）；
//!     `DA`/`DF` = 大径/小径（可省，按 `GearParams` 推：ha*=1、c*=0.25）；
//!   * 旧写法 `GEAR M5 Z10 H20`（可 `ALPHA25`）**继续可用**（等价 `GEAR EX … X0`）；
//!   * `H` = 齿宽（省略 = 10m）；分度圆 d = m·z 由参数导出、**不给 S/E**；
//!     齿形用 `OCSMGEAR` 单独出，这里不画齿、本期不做斜齿（`BETA…` 报「斜齿未实现」）；
//!   * 矩形花键 `SPLINE 6x23x26x6 L30`（GB/T 1144）与齿形段共存：`SPLINE` 后跟
//!     齿形关键字（M/Z/EX/IN/DA/DF…）才是渐开线花键齿形段。
//! - `KEY A 18 [双槽]`：**轴槽**（GB/T 1095-2003 平键键槽，本期只做轴槽）——
//!   **轴段类型**（与螺纹/矩形花键并列，进段表 KEY 列）：键型 `A/B/C`（复用 GB/T 1096
//!   平键族口径；键槽侧只做 A/C）+ 键长 `L`（标准 L 系列；中置槽长 = L，端置 `@端` 槽长 = L + t₁）+
//!   位置中置/端置；**键尺寸 `b×h` 由该段直径 d 查 GB/T 1095 d 列自动定**（h 跟 b 走；
//!   显式 `b8h7` 只作校验，必须落在该轴径档标准配对）。`t1` 省略按 b 查表；
//!   与 GEAR / SPLINE / M / OV / RL 互斥。显示：中置恒显示 A（双圆头）；端置 B/C 显示 C（单圆头）。
//!   长度按所选键型自动折算（槽端圆弧吃直段）：中置 B +b / C +b/2；端置 B +b/2 / C 不折算；
//!   实际槽长 = 折算长度（端置再 + t₁）。可选 `双槽`（`DOUBLE` 别名）：绕轴心 180° 对置的
//!   第二个槽，**仅剖视图体现**（常规侧视不变），可承受转矩约为单键联接的 **1.5 倍**；
//!   剖视剖面线按“上下两环各带一个缺口”对称分区。
//! - `KEY A 25 导向`：**导向平键槽（GB/T 1097-2003）**（用户 2026-09-25 开工）——
//!   在 KEY 段上加 `导向`（`GUIDED` 别名）：键型只 A/B（1097 无 C）；**只有中置**
//!   （用户 2026-09-25 裁定，`@端` 明确报错）；L 取 1097 长度系列
//!   （25…450∩L<10b）；槽长 = 键长 L（固定键不折算）；槽上自动画 **2 个固定螺钉螺纹孔**
//!   （d0×L0，自槽底向下、118° 钻尖；孔心距槽两端 L3；`L1/L2/L3` 由 L 查 26 档表）；
//!   与 `双槽` 互斥。**「起键孔」不属于 1097**（那是 GB/T 1096 附录 A 的概念）。详见 `Keyway` 与 handbook 03。
//!   侧视图叠画键 + 剖视缺口含 sagitta 线，见文件中部「轴槽（GB/T 1095）」节
//!   与 `review/键槽_设计.md`。
//! - `VIEW 常规|剖视`：视图开关（默认 `常规`）；独立一行或段内关键字都认
//!   （`VIEW 剖视` / `VIEW=section`），只影响整体视图（**双视图已于 2026-09-23 移除**，
//!   旧 `VIEW 双` 明确报错、不静默降级）；
//! - 多段可用 `|` 或换行分隔；行尾可跟放置参数 `at x,y rot 度`；
//! - 解析错误报「第 N 行（第 k 段）：…」，几何错误报「第 N 段：…」。
//!
//! ## 螺纹段画法口径（外螺纹侧视图，GB 简化画法）
//!
//! 大径 = 段直径（`1轮廓实线层`，即该段上下轮廓，已有）；小径上下各一条
//! **细实线**（`2细线层`，半径 = 小径/2）贯穿该段；螺纹终止线 = 该段末端竖线
//! （复用既有端面线，不重复画），小径细实线止于终止线；同段端面若有倒角、且
//! 倒角切得比小径还深，细实线改止于倒角斜线交点（不挑出材料外）。
//! `M` 段必须是圆柱（S==E），锥面螺纹报错。
//!
//! ## 齿轮段画法口径（侧视图，与 `gear.rs::side_view()` 的轴向投影一致）
//!
//! 齿顶轮廓（= `1轮廓实线层`，即该段轮廓）的端面倒角
//! **C = round(0.6m)**（`gear.rs::GearParams::chamfer()`）**按单侧规则**：该侧
//! 是自由端或邻段外轮廓更小（台阶向下）才倒；邻段更大（肩部）或齐平（端面齐平、
//! 无外角）不倒 —— 轴上的齿轮/花键段不照搬独立齿轮生成器的两端都倒。
//! 端面可见高 = ra − C，倒角斜线只贴该侧端面；倒角终点（台阶）竖线**只在常规
//! 视图**画（半高 = ra，与 `side_view()` 的台阶线同）。另画分度线 r = d/2
//! （`3中心线层`，点划线，不受倒角影响）。**内侧直径线按 MARK 开关**：
//! `SPLINE` 常规侧视图画外齿小径 / 内齿里侧齿顶的 `2细线层` 青色细实线；`GEAR` 不画
//! （既有齿轮口径「无齿根线」）；**剖视两标记都画**
//! 内侧线（`1轮廓实线层`，齿部按不剖，也是剖面线边界）。派生尺寸取 `gear.rs`
//! 同口径（ha*=1、c*=0.25；旧写法 Xn=0 → ra = da/2、rf = df/2；统一表达式的
//! `DA/DF` 为大径/小径，内齿时 `DA` = 外侧齿根、`DF` = 里侧齿顶），不自己另立公式。
//! `ALPHA` = 基准齿形角（度，省略 20°）：按 `gear.rs` 口径进 `GearParams`，影响基圆 db/齿厚 st
//! 等派生值（齿廓用 OCSMGEAR 单独出）；本侧视图只画齿顶/齿根轮廓，半径不由 α 决定。
//! 齿轮段与相邻段的过渡按台阶处理（不做过渡圆角）；**相邻段轮廓半径 > ra
//! 会盖住齿顶线**，报「第 N 段」；≤ ra 一律放行（不再受齿根圆 rf 限制）。
//!
//! ## 花键段画法口径（`SPLINE`，与 `spline.rs` / 模板 `矩形花键.dxf` 同源）
//!
//! `SPLINE 6x23x26x6 L30 [de 71]`：规格代号自带 N/d/D/B（GB/T 1144-2001），
//! `de` 查 GB/T 10952-2005 表 1/表 2（也可覆盖）。
//! - 段长 = **L（满齿段长）+ l（收尾）**（`l = √(h(2R−h))`，R = de/2、h = (D−d)/2；
//!   6×23×26×6 → l = 9.6047）；`s = e = D`，不给 S/E；
//! - 大径线 `y=±D/2` 在 x=L 断开两段（满齿段 + 收尾段，与模板 [43]/[46] 同）；
//! - 小径细线 `y=±d/2`（`2细线层`）从段左端面到 x=L；**收尾弧** R = de/2，
//!   圆心 `(L, ±(d/2+R))`，与小径相切、与大径相交（模板交角 17.75°），
//!   弧上/下两根细竖线在 x=L 与 x=L+l（`2细线层`）；
//! - **不自动画引入倒角**：在相邻段写 `CH`（倒角贴花键凸角；模板的 φ22→φ26 C2
//!   就是这种写法，允许 C 恰好吃满台阶）；花键段本身不能与 CH/OV/RL/M/GEAR 同段；
//! - 剖视：小径线 / 收尾弧改 `1轮廓实线层`，剖面线按**轴线↔小径**两条带
//!   （小径线 → 收尾弧 → 段右端面闭合；齿部按不剖，与独立要素同口径）。
//!
//! ## 几何口径（单视图侧视图）
//!
//! 轴线 = x 轴（x 向右、y = 半径），第 1 段左端面在 x=0；上下对称：
//! - 每段上下各一条线：圆柱 = 水平线，圆锥 = 斜线；
//! - **常规视图**里每条段边界都画一条贯通轴线的竖线（`1轮廓实线层`），包括直径
//!   没有变化的分界；半高 = `min(左段端面实际半径, 右段端面实际半径)`（用户
//!   2026-09-18 更正版口径）；端面有槽时取槽肩（圆角切点）高；**剖视图不画**；
//! - 段间端面 / 两端外端面用竖直线闭合（肩面仍从 `bottom` 画到 `top`）；
//! - **倒角终点**（倒角根）在常规视图也画贯通竖线，半高 = 倒角根半径；
//! - **`OV` / `RL` 槽的起止**：常规视图里槽肩面 / 槽底终止 / 斜壁终点各画一条
//!   贯通竖线，高度规则见 `build_geometry` 内的注释；剖视只画真实槽体；
//! - **倒角贴端面凸角**：相邻段更大 → 倒角落在相邻（大）段一侧；相邻段更小或
//!   本段是自由端 → 倒角落在本段一侧。这样 `S40 E40 L30 CH2@R OV3`（右端有
//!   越程槽 + 相邻 φ50 台阶）里倒角落在 φ50 上，两个特征不重叠——这是本期的
//!   定案口径（待用户复核）。
//! - `OV` 复用 `detail.rs` 的查表与画法（R 圆角/槽底/45° 斜坡），**不画砂轮细线**
//!   （用户 2026-09-18 定案：那条 `2细线层` 青线属于标注，本图不该有；`detail.rs`
//!   的独立「磨外圆」要素同口径也不画，见 `detail.rs` 顶部画法说明）；
//! - 落层：轮廓/端面/倒角/槽/边界竖线 → `1轮廓实线层`；**螺纹小径/螺尾
//!   细实线** → `2细线层`（螺纹线是几何线，保留；本轴无螺纹时自然为 0 条）；
//!   轴线与分度线 → `3中心线层`（点划线；轴线长度 = 总长 +
//!   图框比例 × 6，两端各半；齿形段分度线长度 = 段长 + 图框比例 × 6，两端各半
//!   —— 与齿轮生成器 `gear::centerline_len` 同一口径）；`剖视` 的剖面线 → `5剖面线层`。
//!
//! ## 视图口径（`VIEW`）
//!
//! * `常规`（默认）= 只画外形可见线：真实几何 + **贯通竖线**（段边界 / 倒角终点 /
//!   槽界线 / 齿轮台阶线），无剖面线；
//! * `剖视` = **只画真实几何 + 剖面线**（不再画贯通竖线；用户 2026-09-18 定案）：
//!   段间端面（环形面）、倒角斜线、槽的真实壁面、齿轮齿根线；ANSI31、比例 1.0，
//!   落 `5剖面线层`，走 `partgen_kit::hatch_ansi31_rings` 通路；边界 = 上半边界
//!   按 x 排序后拆成**上/下两个环**（参考件 `轴剖视图.dxf` 右视图：2 环 21+21 边）；
//!   普通全螺纹段按**小径包络**、齿轮段按**齿根圆**取剖面线边界（牙顶/齿部不剖）。
//!   （旧的 `双` = 常规+剖视并排**已于 2026-09-23 移除**：`VIEW 双` 明确报错。）
//!
//! ## JSON（同一模型，给 GUI/HTTP：`/api/shaft_parse` / `/api/shaft_preview` / `/api/shaft_export`）
//!
//! ```json
//! {"segments":[{"s":30,"e":30,"l":45,"ch":[{"c":2,"end":"L"}]},{"s":30,"e":30,"l":20,"thread":1.5}],"view":"section","at":[100,50],"rot":30}
//! ```

use ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge;
use ocs_plugin_api::host::acadrust::entities::{EntityType, Hatch};
use ocs_plugin_api::host::acadrust::types::{Vector2, Vector3};
use serde::{Deserialize, Serialize};

use crate::detail;
use crate::gear::{GearKind, GearParams};
use crate::partgen_kit::{
    arc, line, thread_major_arc, trim, HatchEdge, LAYER_CENTER, LAYER_HATCH, LAYER_MAIN, LAYER_THIN,
};

/// `OCSMSHAFT` 不带参数时打开轴生成器窗口；命令行带参数时此处是用法说明。
/// 轴生成器用法（catalog 唯一来源）。
pub fn usage() -> String {
    crate::i18n::t("cmd.shaft.usage")
}

// ══════════════════════════════════════════════════════════════════════════
// 数据模型
// ══════════════════════════════════════════════════════════════════════════

/// 特征所在端。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum End {
    L,
    R,
}

fn end_cn(end: End) -> String {
    crate::i18n::t(match end {
        End::L => "cmd.shaft.end.l",
        End::R => "cmd.shaft.end.r",
    })
}

/// 段号标签「第 N 段」/ "segment N"（catalog 唯一来源；几何报错/回执共用）。
pub(crate) fn seg_label(n: usize) -> String {
    crate::i18n::t_fmt("cmd.shaft.label.seg", &[("n", &n.to_string())])
}

/// 视图开关（用户 2026-09-18 定案；**双视图已于 2026-09-23 移除**）：`常规` / `剖视`。
///
/// * `常规` = 只画外形可见线（默认）；贯通竖线只属于常规视图；
/// * `剖视` = **真实几何**（不含贯通竖线）+ ANSI31 剖面线（`5剖面线层`，上下两环）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ShaftView {
    Normal,
    Section,
}

impl Default for ShaftView {
    fn default() -> Self {
        ShaftView::Normal
    }
}

impl ShaftView {
    /// 英文键（JSON/HTTP/GUI 传参、块名）。
    pub fn key(self) -> &'static str {
        match self {
            ShaftView::Normal => "normal",
            ShaftView::Section => "section",
        }
    }

    /// 中文名（DSL/GUI/报错提示）。
    pub fn label(self) -> &'static str {
        match self {
            ShaftView::Normal => "常规",
            ShaftView::Section => "剖视",
        }
    }

    /// 是否带剖面线（只有 `剖视`）。
    pub fn has_hatch(self) -> bool {
        matches!(self, ShaftView::Section)
    }

    pub const ALL: [ShaftView; 2] = [ShaftView::Normal, ShaftView::Section];

    /// 解析视图名（中英文都认；`view=` / `视图` 前缀也剥掉）。
    pub fn parse(s: &str) -> Result<Self, String> {
        let t = s.trim().to_ascii_lowercase();
        let t = t
            .trim_start_matches("view")
            .trim_start_matches(['=', ':'])
            .trim_start_matches("视图")
            .trim();
        let hit = match t {
            "normal" | "regular" | "outline" | "常规" | "常规视图" | "外形" | "外观" | "不剖" => {
                Some(ShaftView::Normal)
            }
            "section" | "cut" | "剖" | "剖视" | "剖视图" | "剖面" | "剖开" => {
                Some(ShaftView::Section)
            }
            // 双视图用户 2026-09-23 定案移除：点名报错，不静默降级成常规
            // （风格同撤掉轴段 INVOLSPLINE 时的“不识别的关键字 + 原因/指路”）。
            "both" | "dual" | "双" | "双视图" | "并排" | "常规+剖视" | "两个" => {
                return Err(crate::i18n::t_fmt(
                    "cmd.shaft.err.view_double_removed",
                    &[("name", s.trim())],
                ));
            }
            _ => None,
        };
        hit.ok_or_else(|| {
            crate::i18n::t_fmt("cmd.shaft.err.view_unknown", &[("name", &(s.trim()).to_string())])
        })
    }
}

/// 端面倒角 C（45°：沿轴向 C、半径方向 C）。
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Chamfer {
    pub c: f64,
    pub end: End,
}

/// 砂轮越程槽：`b1 = None` = 按该端直径查 GB 表；`Some` = 显式槽宽。
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Overtravel {
    pub b1: Option<f64>,
    pub end: End,
}

/// **段级退刀槽**（GB/T 3-1997 表 2 画法）：贴在某圆柱段的左/右端、压在段内。
///
/// 与 `M` 段的 `RL`（螺纹收尾，收在 `Thread.relief`）不同：这是任意圆柱段的
/// 肩部特征，可带 `P`（表 2 单键）或显式 `g1/g2/dg/r`（`dg` 为绝对直径）。
/// 圆角心/槽底/斜壁坐标与 `detail.rs::relief_groove_entities` 同一剖面。
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Relief {
    /// 贴在段左端（`RL@L`）还是右端（`RL@R`，`@` 省略默认）。
    pub end: End,
    /// 螺距 P（表 2 单键）；`None` + 全给 `g1/g2/dg/r` = 显式尺寸路径。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p: Option<f64>,
    /// 显式覆盖：槽底平段终点（含 R 圆角段）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub g1: Option<f64>,
    /// 显式覆盖：台肩面 → 斜壁与大径交点的总宽。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub g2: Option<f64>,
    /// 显式覆盖：退刀槽**绝对直径**（表 2 的 `d−Δ`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dg: Option<f64>,
    /// 显式覆盖：槽底圆角半径。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r: Option<f64>,
}

impl Relief {
    /// 是否带任何尺寸覆盖（`M` 段的螺纹收尾 RL 不允许带，保持旧口径）。
    fn has_overrides(&self) -> bool {
        self.p.is_some() || self.g1.is_some() || self.g2.is_some() || self.dg.is_some() || self.r.is_some()
    }
}

/// `RO` 收尾档位（GB/T 3-1997 表 1）：一般 x≈2.5P / 短 x≈1.25P。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RunoutGrade {
    Normal,
    Short,
}

/// `SD` 肩距档位（GB/T 3-1997 表 1）：一般 a≈3P / 长 a=4P / 短 a=2P。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ShoulderGrade {
    Normal,
    Long,
    Short,
}

/// 螺纹段标记（外螺纹侧视图）：不写值 = 简化画法小径 0.85d；写螺距 = 精确小径。
///
/// `TL` / `RO` / `SD` / `RL` 四个局部螺纹关键字都收在这里；不给 `TL` 也不给
/// `RL` 时 = 旧口径整段全螺纹（小径细实线贯穿该段，行为不变）。
/// `RL = true` 表示贴段右端的**螺纹退刀槽收尾**（表 2 按螺距查）；段级（任意
/// 圆柱段、可带 `P`/显式尺寸、可贴左/右端）的 `RL` 在 [`Segment::relief`]。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thread {
    /// 螺距 P（> 0）；`None` = 不写值（小径按 0.85d 简化画法）。
    pub pitch: Option<f64>,
    /// `TL` 完整螺纹长度（> 0）；`None` = 整段全螺纹（旧行为）。
    pub tl: Option<f64>,
    /// `RO` 收尾档位（GB/T 3 表 1；默认一般）。
    pub runout: RunoutGrade,
    /// `SD` 肩距档位（GB/T 3 表 1；默认一般）。
    pub shoulder: ShoulderGrade,
    /// `RL` = 用 GB/T 3 表 2 退刀槽收尾（与 `RO`/`SD` 互斥）。
    pub relief: bool,
}

impl Default for Thread {
    fn default() -> Self {
        Thread {
            pitch: None,
            tl: None,
            runout: RunoutGrade::Normal,
            shoulder: ShoulderGrade::Normal,
            relief: false,
        }
    }
}

impl Thread {
    /// 只有螺距（或什么都不给）的旧形态：JSON 沿 `true` / 螺距数字序列化。
    fn is_plain(&self) -> bool {
        self.tl.is_none()
            && self.runout == RunoutGrade::Normal
            && self.shoulder == ShoulderGrade::Normal
            && !self.relief
    }

    /// 小径 d1：不写值 = 0.85d；写螺距 = d − 1.0825P（GB/T 192 基本尺寸口径）。
    pub fn minor_diameter(&self, d: f64) -> f64 {
        match self.pitch {
            None => 0.85 * d,
            Some(p) => d - 1.0825 * p,
        }
    }

    /// 小径半径 = 小径/2（细实线所在半径）。
    pub fn minor_radius(&self, d: f64) -> f64 {
        self.minor_diameter(d) / 2.0
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 轴槽（GB/T 1095-2003 平键键槽）
// ══════════════════════════════════════════════════════════════════════════

/// 轴槽位置：`Mid` = 中置（槽两端封闭）/ `End` = 端置（开在轴首/末段自由端）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KeywayPlace {
    Mid,
    End,
}

impl KeywayPlace {
    fn cn(self) -> String {
        crate::i18n::t(match self {
            KeywayPlace::Mid => "cmd.shaft.place.mid",
            KeywayPlace::End => "cmd.shaft.place.end",
        })
    }

    /// 机器码（`partgen_keys::KEY_STYLES` 的 `places` 用）：`mid` / `end`。
    pub fn code(self) -> &'static str {
        match self {
            KeywayPlace::Mid => "mid",
            KeywayPlace::End => "end",
        }
    }

    fn parse(text: &str) -> Result<Self, String> {
        match text.trim() {
            "中" | "中置" | "mid" | "Mid" | "MID" => Ok(KeywayPlace::Mid),
            "端" | "端置" | "end" | "End" | "END" => Ok(KeywayPlace::End),
            other => Err(crate::i18n::t_fmt("cmd.shaft.err.key_place_invalid", &[("other", &other.to_string())])),
        }
    }
}

/// 平键型别（复用 GB/T 1096 平键族型别口径）：
/// `A` = 双圆头 / `C` = 单圆头（左圆右方）——**键槽侧只做 A/C**；
/// `B` = 双平头（平键族标准件仍保留，但**没有键槽**：方形盲孔加工困难），传入时明确报错并指路。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum KeyKind {
    A,
    B,
    C,
}

impl KeyKind {
    fn cn(self) -> String {
        crate::i18n::t(match self {
            KeyKind::A => "cmd.shaft.keykind.a",
            KeyKind::B => "cmd.shaft.keykind.b",
            KeyKind::C => "cmd.shaft.keykind.c",
        })
    }

    /// 平键族 id（数据/型别唯一来源）。
    pub fn family_id(self) -> &'static str {
        match self {
            KeyKind::A => "key_1096_a",
            KeyKind::B => "key_1096_b",
            KeyKind::C => "key_1096_c",
        }
    }

    /// 单字母代号（DSL/JSON/GUI 用）。
    pub fn code(self) -> &'static str {
        match self {
            KeyKind::A => "A",
            KeyKind::B => "B",
            KeyKind::C => "C",
        }
    }

    fn key_type(self) -> crate::partgen_keys::KeyType {
        match self {
            KeyKind::A => crate::partgen_keys::KeyType::A,
            KeyKind::B => crate::partgen_keys::KeyType::B,
            KeyKind::C => crate::partgen_keys::KeyType::C,
        }
    }

    fn from_key_type(ty: crate::partgen_keys::KeyType) -> Self {
        match ty {
            crate::partgen_keys::KeyType::A => KeyKind::A,
            crate::partgen_keys::KeyType::B => KeyKind::B,
            crate::partgen_keys::KeyType::C => KeyKind::C,
        }
    }

    fn parse(text: &str) -> Result<Self, String> {
        match text.trim().to_ascii_uppercase().as_str() {
            "A" => Ok(KeyKind::A),
            "B" => Ok(KeyKind::B),
            "C" => Ok(KeyKind::C),
            other => Err(crate::i18n::t_fmt("cmd.shaft.err.key_kind_invalid", &[("other", &other.to_string())])),
        }
    }
}

/// 轴槽（GB/T 1095-2003 平键键槽；本期只画轴槽 t₁，毂槽 t₂ 只入库不画）。
///
/// 交互口径（用户 2026-09-23 定案）：轴槽是**轴段类型**（与螺纹/矩形花键并列），
/// 轴段表列 = 键型 + 键长 L + 位置；**键尺寸 b×h 由该轴段直径 d 自动确定**。
/// * `kind` = 键型 A/B/C（复用平键族型别口径；B/C 的键长自动折算，见 `effective_len`）；
/// * `b` 省略 = 按该段直径 d 查 `assets/key1096_shaft_ranges.csv`（GB/T 1095 d 列，**选型依据**）；
///   显式给了 b 也必须等于该轴径档的标准 b，否则明确报错（不许静默接受非法组合）；
/// * `h` 可选，只作 b×h 配对校验（h 跟 b 走，不允许自由组合）；
/// * `l` = **键长**（GB/T 1096 标准 L 系列；中置槽长 = L；端置槽长 = L + t₁，模板 LC 口径）；
/// * `place` 决定槽在段上的位置（中置 = 该段「倒角根↔段末」净圆柱的中点；端置 = 开在轴端）；
/// * `t1` 省略 = 按 `b` 查 `assets/keyway_gb1095.csv`（GB/T 1095-2003 表 1）。
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Keyway {
    #[serde(rename = "type")]
    pub kind: KeyKind,
    pub l: f64,
    pub place: KeywayPlace,
    /// 键宽 b：省略 = 按该段直径查 GB/T 1095 d 列；给了必须等于该轴径档的标准 b。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub b: Option<f64>,
    /// 键高 h：可选（只作 b×h 配对校验；省略 = 按 b 由平键族派生）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t1: Option<f64>,
    /// **双键槽**（可选项，用户 2026-09-23）：绕轴心 180° 对置的第二个槽；
    /// * **仅剖视图体现**（常规侧视图不受影响）；
    /// * 可承受转矩约为单键联接的 **1.5 倍**（AI 手册口径）；
    /// * 剖视剖面线按“上下两环各带一个缺口”分区（对称镜像，不再去缺口）。
    #[serde(default, skip_serializing_if = "is_false")]
    pub double: bool,
    /// **导向平键（GB/T 1097-2003，用户 2026-09-25 开工）**：
    /// * 槽上带 **2 个固定螺钉螺纹孔**（d0×L0，自槽底向下，118° 钻尖；孔心距槽两端 L3）；
    /// * 槽长 = 键长 L（固定键不折算）；`kind` 只 A/B（1097 无 C 型）；
    /// * **只有中置**（用户 2026-09-25 裁定）：`place = End` 在 assemble/validate/keyway_geom 明确报错；
    /// * 起键孔不属于 1097（那是 GB/T 1096 附录 A 的概念）——槽上只有固定螺钉孔。
    #[serde(default, skip_serializing_if = "is_false")]
    pub guided: bool,
}

impl Keyway {
    /// **实际参与几何的“渲染/槽长基准”**（用户 2026-09-23 口径）：
    /// 槽端圆弧半径 = b/2 会吃掉一段直段，所以按“所选键型”把用户输入的键长折算成
    /// 显示/开槽长度：
    /// * 中置显示 A（两端弧）：A → L；B（需直段 L）→ L + b；C（平端需 L−b/2）→ L + b/2；
    /// * 端置显示 C（单圆头）：B → L + b/2；C → L；A（双圆头）→ L。
    /// * **导向（guided）**：键固定在轴上、长度即槽长，不折算 → L。
    pub fn effective_len(&self, b: f64) -> f64 {
        if self.guided {
            return self.l;
        }
        match (self.place, self.kind) {
            (KeywayPlace::Mid, KeyKind::A) => self.l,
            (KeywayPlace::Mid, KeyKind::B) => self.l + b,
            (KeywayPlace::Mid, KeyKind::C) => self.l + b / 2.0,
            (KeywayPlace::End, KeyKind::A) => self.l,
            (KeywayPlace::End, KeyKind::B) => self.l + b / 2.0,
            (KeywayPlace::End, KeyKind::C) => self.l,
        }
    }

    /// 实际槽长：中置 = 折算长度；端置 = 折算长度 + t₁（模板 LC = 端部槽长口径）。
    /// **导向**：槽长 = 键长 L（固定键；键填满槽本身）；导向只有中置，无端置分支。
    pub fn slot_len(&self, t1: f64, b: f64) -> f64 {
        if self.guided {
            return self.l;
        }
        let eff = self.effective_len(b);
        match self.place {
            KeywayPlace::Mid => eff,
            KeywayPlace::End => eff + t1,
        }
    }
}

/// 键宽 b：**按轴段直径 d 查 GB/T 1095 d 列**（主路径）；显式 b 必须等于该轴径档的标准 b。
pub fn keyway_b(keyway: &Keyway, shaft_d: f64) -> Result<f64, String> {
    let std_b = crate::partgen_keys::key_1096_b_for_shaft(shaft_d).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.shaft.err.gb1095_no_d",
            &[("d", &trim(shaft_d))],
        )
    })?;
    let std_h = crate::partgen_keys::key_1096_h(keyway.kind.key_type(), std_b).ok_or_else(|| {
        crate::i18n::t_fmt(
            "cmd.shaft.err.gb1095_b_out",
            &[("d", &trim(shaft_d)), ("b", &trim(std_b))],
        )
    })?;
    match keyway.b {
        None => Ok(std_b),
        Some(b) if (b - std_b).abs() < 1e-9 => Ok(b),
        Some(b) => Err(crate::i18n::t_fmt("cmd.shaft.err.gb1095_b_pair", &[("p0", &(trim(shaft_d)).to_string()), ("p1", &(trim(std_b)).to_string()), ("p2", &(trim(std_h)).to_string()), ("p3", &(trim(b)).to_string())])),
    }
}

/// GB/T 1095-2003 表 1 一行（轴槽 t₁ + 毂槽 t₂ + 宽度公差带）。
/// 数据 = `assets/keyway_gb1095.csv`（由 `review/键槽_GB1095_表_v2.csv` 入库，d 列剔除，
/// b=100 的 t₂ 取官方 19.5）。
pub struct KeywayRow {
    pub b: f64,
    pub h: f64,
    pub t1: f64,
    pub t1_up: f64,
    pub t1_low: f64,
    pub t2: f64,
    pub t2_up: f64,
    pub t2_low: f64,
    pub r_min: f64,
    pub r_max: f64,
    /// 槽宽公差带（孔制，大写）：松 = H9/D10、正常 = N9/JS9、紧密 = P9。
    pub b_h9: (f64, f64),
    pub b_d10: (f64, f64),
    pub b_n9: (f64, f64),
    pub b_js9: (f64, f64),
    pub b_p9: (f64, f64),
}

/// 入库表（`crates/ocs_ocsm/assets/keyway_gb1095.csv`，26 档）。
const KEYWAY_GB1095_CSV: &str = include_str!("../assets/keyway_gb1095.csv");

fn parse_keyway_gb1095() -> Result<Vec<KeywayRow>, String> {
    let mut rows = Vec::new();
    let mut lines = KEYWAY_GB1095_CSV
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'));
    let header = lines
        .next()
        .ok_or_else(|| crate::i18n::t("cmd.shaft.err.csv_header_missing"))?;
    if !header.starts_with("b,h,") {
        return Err(crate::i18n::t_fmt(
            "cmd.shaft.err.csv_header_bad",
            &[("header", header)],
        ));
    }
    for line in lines {
        let v: Vec<&str> = line.split(',').collect();
        let num = |i: usize, what: &str| -> Result<f64, String> {
            v.get(i)
                .ok_or_else(|| {
                    crate::i18n::t_fmt("cmd.shaft.err.csv_missing_col", &[("what", what)])
                })?
                .parse::<f64>()
                .map_err(|e| {
                    crate::i18n::t_fmt(
                        "cmd.shaft.err.csv_col_num",
                        &[("i", &i.to_string()), ("what", what), ("e", &e.to_string())],
                    )
                })
        };
        rows.push(KeywayRow {
            b: num(0, "b")?,
            h: num(1, "h")?,
            t1: num(2, "t1")?,
            t1_up: num(3, "t1_up")?,
            t1_low: num(4, "t1_low")?,
            t2: num(5, "t2")?,
            t2_up: num(6, "t2_up")?,
            t2_low: num(7, "t2_low")?,
            r_min: num(8, "r_min")?,
            r_max: num(9, "r_max")?,
            b_h9: (num(10, "b_H9_up")?, num(11, "b_H9_low")?),
            b_d10: (num(12, "b_D10_up")?, num(13, "b_D10_low")?),
            b_n9: (num(14, "b_N9_up")?, num(15, "b_N9_low")?),
            b_js9: (num(16, "b_JS9_up")?, num(17, "b_JS9_low")?),
            b_p9: (num(18, "b_P9_up")?, num(19, "b_P9_low")?),
        });
    }
    Ok(rows)
}

/// GB/T 1095 表 1 全表（26 档；解析错误在测试里断言，生产路径 `keyway_row` 报错）。
pub fn keyway_gb1095_rows() -> Result<Vec<KeywayRow>, String> {
    parse_keyway_gb1095()
}

/// 按 b 查 GB/T 1095 表 1（b 必须是标准档）。
fn keyway_row(b: f64) -> Result<KeywayRow, String> {
    let rows = parse_keyway_gb1095()
        .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.gb1095_table_prefix", &[("e", &e)]))?;
    rows.into_iter()
        .find(|row| (row.b - b).abs() < 1e-9)
        .ok_or_else(|| {
            crate::i18n::t_fmt("cmd.shaft.err.gb1095_no_b", &[("b", &trim(b))])
        })
}

/// 轴槽 t₁（显式值优先；否则按 b 查 GB/T 1095 表 1）。
pub fn keyway_t1(b: f64, explicit: Option<f64>) -> Result<f64, String> {
    match explicit {
        Some(t1) => Ok(t1),
        None => keyway_row(b).map(|row| row.t1),
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 数据：GB/T 3-1997 表 1 / 表 2（**表归一** —— 唯一数据源在 `detail.rs`）
// ══════════════════════════════════════════════════════════════════════════
//
// 表 1（收尾 x / 肩距 a）与表 2（外螺纹退刀槽）此前在本文件与 `detail.rs`
// 各有一份，现已收敛到 `detail.rs` 一处（`RUNOUT_ROWS` / `THREAD_RELIEF_ROWS`
// + `runout_row_checked` / `thread_relief_row`）；本文件只保留两个薄封装：
// 档位选择（`x(grade)` / `a(grade)`）与「段级 RL 取参」。

impl detail::RunoutRow {
    /// `RO` 档位 → 收尾 x。
    fn x(&self, grade: RunoutGrade) -> f64 {
        match grade {
            RunoutGrade::Normal => self.x_normal,
            RunoutGrade::Short => self.x_short,
        }
    }

    /// `SD` 档位 → 肩距 a。
    fn a(&self, grade: ShoulderGrade) -> f64 {
        match grade {
            ShoulderGrade::Normal => self.a_normal,
            ShoulderGrade::Long => self.a_long,
            ShoulderGrade::Short => self.a_short,
        }
    }
}

/// 段级 `RL` 取参（优先级按任务口径，不猜）：
/// ① 段内显式覆盖 `g1/g2/dg/r`（`dg` 是**绝对直径**）；
/// ② 段内给 `P` → `detail.rs` 表 2（`g1/g2/dg/r` 可叠加覆盖）；
/// ③ 都不给 → 报错（提示表 2 以螺距为键）。
fn resolve_relief_dims(
    seg: &Segment,
    spec: &Relief,
    label: &str,
) -> Result<detail::ReliefDims, String> {
    let d = seg.s;
    if let Some(p) = spec.p {
        let mut params = detail::DetailParams::new();
        params.insert("P", p);
        for (key, value) in [("g1", spec.g1), ("g2", spec.g2), ("dg", spec.dg), ("r", spec.r)] {
            if let Some(value) = value {
                params.insert(key, value);
            }
        }
        detail::relief_dims(d, &params).map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", label), ("e", &e)]))
    } else if let (Some(g1), Some(g2), Some(dg), Some(r)) = (spec.g1, spec.g2, spec.dg, spec.r) {
        detail::relief_dims_explicit(d, g1, g2, dg, r).map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", label), ("e", &e)]))
    } else {
        Err(crate::i18n::t_fmt("cmd.shaft.err.rl_missing_params", &[("label", label)]))
    }
}

/// 左端退刀槽把螺纹小径细实线起点往右推的距离（从段左端起算）：
/// 槽底低于小径（`r1 ≤ dg/2`）→ 整段槽体 `g2`；否则取斜壁与小径线的交点。
fn relief_minor_inset(r: f64, r1: f64, dims: &detail::ReliefDims) -> f64 {
    let rg = dims.dg / 2.0;
    if r1 <= rg {
        dims.g2
    } else if r1 >= r {
        0.0
    } else {
        dims.g1 + (dims.g2 - dims.g1) * (r1 - rg) / (r - rg)
    }
}

/// 把 `thread` 序列化（给 GUI/HTTP）：
/// - 旧形态（只有螺距 / 什么都不给）保持 `true` / 螺距数字，老 GUI 不受影响；
/// - 带 `TL`/`RO`/`SD`/`RL` 时序列化成对象，保证 JSON 往返不丢新关键字。
fn serialize_thread<S: serde::Serializer>(
    thread: &Option<Thread>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match thread {
        None => serializer.serialize_none(),
        Some(t) if t.is_plain() => match t.pitch {
            Some(pitch) => serializer.serialize_f64(pitch),
            None => serializer.serialize_bool(true),
        },
        Some(t) => {
            let mut value = serde_json::Map::new();
            if let Some(pitch) = t.pitch {
                value.insert("pitch".into(), serde_json::json!(pitch));
            }
            if let Some(tl) = t.tl {
                value.insert("tl".into(), serde_json::json!(tl));
            }
            if t.runout != RunoutGrade::Normal {
                value.insert("runout".into(), serde_json::json!(t.runout));
            }
            if t.shoulder != ShoulderGrade::Normal {
                value.insert("shoulder".into(), serde_json::json!(t.shoulder));
            }
            if t.relief {
                value.insert("relief".into(), serde_json::json!(true));
            }
            serde_json::Value::Object(value).serialize(serializer)
        }
    }
}

/// 齿形段参数（齿轮/渐开线花键**统一表达式**；派生尺寸统一走 [`GearParams`]）。
///
/// 九项字段：`MARK`（[`Gear::involute`]）`KIND`（[`Gear::kind`]）`M Z ALPHA X DA DF BETA` + 轴段 `H`。
/// `DA`/`DF` 恒为**大径/小径**（外齿 = 齿顶/齿根；内齿 = 外侧齿根/里侧齿顶），可显式覆盖。
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Gear {
    /// 模数 m（> 0）。
    pub m: f64,
    /// 齿数 z（整数，2..=1000，与 gear.rs 一致）。
    pub z: u32,
    /// 齿宽 H；`None` = 10m。
    pub h: Option<f64>,
    /// 螺旋角 β（度）；本期只支持 0（斜齿 = 二期）。
    #[serde(rename = "beta")]
    pub beta_deg: f64,
    /// 基准齿形角 α（度；默认 20°，JSON/DSL 可给 `ALPHA25`）。
    /// 缺省值不写出（保持旧 JSON 与 DSL 文本不变）。
    #[serde(rename = "alpha", skip_serializing_if = "alpha_is_default")]
    pub alpha_deg: f64,
    /// 齿形标记：`false` = 齿轮 `GEAR`（常规侧视图**画**内侧直径细实线）、
    /// `true` = 渐开线花键 `SPLINE`（不画，花键制图口径）。**参与几何输出**，随 DSL/JSON 往返。
    #[serde(default, skip_serializing_if = "is_false")]
    pub involute: bool,
    /// 内/外齿（`GEAR IN`/`EX`）；旧写法无 = 外。
    #[serde(default, skip_serializing_if = "kind_is_external")]
    pub kind: GearKind,
    /// 变位系数 Xn（`X`；旧写法无 = 0）。
    #[serde(default, skip_serializing_if = "x_is_zero")]
    pub x: f64,
    /// 大径（外齿齿顶圆 / 内齿外侧齿根）；`None` = 按 m/z/x 推。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub da: Option<f64>,
    /// 小径（外齿齿根圆 / 内齿里侧齿顶）；`None` = 按 m/z/x 推。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub df: Option<f64>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn kind_is_external(kind: &GearKind) -> bool {
    !kind.is_internal()
}

fn x_is_zero(value: &f64) -> bool {
    value.abs() < 1e-12
}

/// GEAR 段的默认压力角（与 `gear.rs::ALPHA_N_DEG` 同值）。
fn default_alpha_deg() -> f64 {
    crate::gear::ALPHA_N_DEG
}

fn alpha_is_default(value: &f64) -> bool {
    (*value - default_alpha_deg()).abs() < 1e-9
}

impl Gear {
    /// 齿宽（H 省略 = 10m）。
    pub fn width(&self) -> f64 {
        self.h.unwrap_or(10.0 * self.m)
    }

    /// 与 `gear.rs` 同口径的齿轮参数（ha*=1、c*=0.25、直齿；kind/x 随表达式）。
    pub fn params(&self) -> GearParams {
        GearParams {
            kind: self.kind,
            m: self.m,
            z: self.z,
            alpha_deg: self.alpha_deg,
            beta_deg: self.beta_deg,
            h: self.width(),
            x: self.x,
            ..GearParams::default()
        }
    }

    /// 分度圆半径 r = d/2（= m·z/2；变位不改变分度圆）。
    pub fn pitch_radius(&self) -> f64 {
        self.params().d() / 2.0
    }

    /// **大径半径**：显式 `DA` 优先；否则外齿 = 齿顶圆 da、内齿 = 外侧齿根 df。
    pub fn major_radius(&self) -> f64 {
        if let Some(v) = self.da {
            return v / 2.0;
        }
        let p = self.params();
        (if self.kind.is_internal() { p.df() } else { p.da() }) / 2.0
    }

    /// **小径半径**：显式 `DF` 优先；否则外齿 = 齿根圆 df、内齿 = 里侧齿顶 da。
    pub fn minor_radius(&self) -> f64 {
        if let Some(v) = self.df {
            return v / 2.0;
        }
        let p = self.params();
        (if self.kind.is_internal() { p.da() } else { p.df() }) / 2.0
    }

    /// 外轮廓半径（= 大径；旧名保留，外部调用/测试沿用）。
    pub fn addendum_radius(&self) -> f64 {
        self.major_radius()
    }

    /// 内侧直径半径（外齿 = 齿根圆；内齿 = 里侧齿顶；旧名保留）。
    pub fn root_radius(&self) -> f64 {
        self.minor_radius()
    }

    /// 轴向倒角 C = round(0.6m)（与 `gear.rs::GearParams::chamfer()` 同口径；
    /// 小模数可能为 0 = 不倒角）。
    pub fn chamfer(&self) -> f64 {
        self.params().chamfer()
    }
}

/// 齿形段的剖面线边界半径：外齿 = 小径（齿根圆，齿部按不剖）；内齿 = 大径
/// （最小实现按实体段近似，不挖内孔 —— 报告已注明）。
fn hatch_bound_radius(gear: &Gear) -> f64 {
    if gear.kind.is_internal() {
        gear.major_radius()
    } else {
        gear.minor_radius()
    }
}

/// 一段轴（从左到右）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Segment {
    /// 起始直径（左端）。齿轮段 = 分度圆直径（由 M·Z 导出）。
    pub s: f64,
    /// 终点直径（右端）。齿轮段 = 分度圆直径。
    pub e: f64,
    /// 段长。齿轮段 = H（省略 10m）。
    pub l: f64,
    /// 倒角（同一端最多一个）。
    pub ch: Vec<Chamfer>,
    /// 越程槽（同一端最多一个）。
    pub ov: Vec<Overtravel>,
    /// 段级退刀槽（`RL@L`/`RL@R`；同一端最多一个）。`M` 段的右端螺纹收尾
    /// 不走这里（仍以 `Thread.relief` 表示），与 `CH`/`OV` 一样按端别互斥。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relief: Vec<Relief>,
    /// 螺纹段标记（`None` = 普通轴段）。
    #[serde(serialize_with = "serialize_thread", skip_serializing_if = "Option::is_none")]
    pub thread: Option<Thread>,
    /// 齿轮段参数（`None` = 普通轴段）。
    pub gear: Option<Gear>,
    /// 矩形花键段（`SPLINE 6x23x26x6 L30`；`None` = 普通轴段）。
    ///
    /// `s`/`e` = 大径 D，`l` = 满齿段长 L + 收尾 l（见 `spline.rs` 的段长口径）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spline: Option<crate::spline::RectSpline>,
    /// 轴槽（GB/T 1095 平键键槽；`None` = 无槽）。只能挂在圆柱段上，与齿轮/花键/
    /// 螺纹/越程槽/退刀槽互斥（见 `validate`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyway: Option<Keyway>,
}

impl Segment {
    /// 某端的**外轮廓半径**（齿形段 = 大径/2；矩形花键段 = 大径半径）。
    pub fn outer_radius(&self, end: End) -> f64 {
        if let Some(gear) = &self.gear {
            return gear.major_radius();
        }
        if let Some(spline) = &self.spline {
            let _ = end;
            return spline.major_radius();
        }
        match end {
            End::L => self.s / 2.0,
            End::R => self.e / 2.0,
        }
    }
}

/// 解析结果：段序列 + 放置参数（`at` / `rot` 由命令层应用）+ 视图开关。
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize)]
pub struct Program {
    pub segments: Vec<Segment>,
    pub at: Option<[f64; 2]>,
    pub rot: Option<f64>,
    /// 视图（默认 `常规`）；`serde` 里序列化成 `normal`/`section`（`both` 已移除）。
    #[serde(default)]
    pub view: ShaftView,
}

impl Program {
    /// 总长。
    pub fn total_length(&self) -> f64 {
        self.segments.iter().map(|s| s.l).sum()
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 解析：行 DSL
// ══════════════════════════════════════════════════════════════════════════

/// 解析行 DSL 或 JSON（首字符 `{` = JSON）。返回的几何还没做合法性检查，
/// 校验在 [`validate`] / [`build`] 里（这样测试与 GUI 能分别拿到解析层错误）。
pub fn parse_program(text: &str) -> Result<Program, String> {
    if text.trim_start().starts_with('{') {
        return parse_json(text);
    }
    let mut program = Program::default();
    let mut view_seen: Option<ShaftView> = None;
    // 全文一个「逻辑段」计数器对不上的情况：同一行用 `|` 分多段时单独标「第 k 段」。
    for (line_idx, raw_line) in text.lines().enumerate() {
        let line_no = line_idx + 1;
        // 注释：整行 `#` 或行内 `#` 之后都忽略。
        let line = raw_line.split('#').next().unwrap_or("");
        if line.trim().is_empty() {
            continue;
        }
        let chunks: Vec<&str> = line.split('|').collect();
        let multi = chunks.iter().filter(|c| !c.trim().is_empty()).count() > 1;
        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let chunk = chunk.trim();
            if chunk.is_empty() {
                continue;
            }
            let label = if multi {
                crate::i18n::t_fmt(
                    "cmd.shaft.label.multi",
                    &[
                        ("line", &line_no.to_string()),
                        ("chunk", &(chunk_idx + 1).to_string()),
                    ],
                )
            } else {
                // 单段行也带上「第 N 段」，便于用户按段定位（与几何错误口径一致）。
                crate::i18n::t_fmt(
                    "cmd.shaft.label.single",
                    &[
                        ("line", &line_no.to_string()),
                        ("chunk", &(program.segments.len() + 1).to_string()),
                    ],
                )
            };
            // `VIEW 剖视` / `视图 剖视` 是整体视图开关：独立一行或段内关键字都认。
            let tokens: Vec<&str> = chunk.split_whitespace().collect();
            let tokens = extract_view_directives(&tokens, &label, &mut view_seen)?;
            if tokens.is_empty() {
                continue;
            }
            if tokens[0].eq_ignore_ascii_case("at") || tokens[0] == "@" {
                parse_placement(&tokens, &label, &mut program)?;
                continue;
            }
            let seg = parse_segment(&tokens.join(" "), &label, &mut program)?;
            program.segments.push(seg);
        }
    }
    if program.segments.is_empty() {
        return Err(crate::i18n::t("cmd.shaft.err.no_segments"));
    }
    program.view = view_seen.unwrap_or_default();
    Ok(program)
}

/// 从一段的 token 里提取 `VIEW …` / `视图 …` 指令（独立一行或段内均认），
/// 返回去掉指令后的 token。同值重复忽略；不同值报冲突。
fn extract_view_directives<'a>(
    tokens: &[&'a str],
    label: &str,
    view: &mut Option<ShaftView>,
) -> Result<Vec<&'a str>, String> {
    let mut out = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        let upper = token.to_ascii_uppercase();
        let mut name: Option<&str> = None;
        if upper == "VIEW" || token == "视图" {
            index += 1;
            name = Some(tokens.get(index).copied().ok_or_else(|| {
                crate::i18n::t_fmt("cmd.shaft.err.view_missing", &[("label", label)])
            })?);
        } else if let Some(rest) = upper.strip_prefix("VIEW") {
            name = rest.strip_prefix(['=', ':']);
        } else if let Some(rest) = token.strip_prefix("视图") {
            name = rest.strip_prefix(['=', ':']);
        }
        if let Some(name) = name {
            let parsed = ShaftView::parse(name).map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", label), ("e", &e)]))?;
            match *view {
                Some(prev) if prev != parsed => {
                    return Err(crate::i18n::t_fmt(
                        "cmd.shaft.err.view_conflict",
                        &[
                            ("label", label),
                            ("prev", prev.label()),
                            ("now", parsed.label()),
                        ],
                    ));
                }
                _ => *view = Some(parsed),
            }
        } else {
            out.push(token);
        }
        index += 1;
    }
    Ok(out)
}

fn unknown_keyword(token: &str, label: &str) -> String {
    crate::i18n::t_fmt("cmd.shaft.err.unknown_keyword", &[("label", label), ("token", token)])
}

/// 齿形段大径/小径合法性（DSL 解析与 JSON 反序列化共用）：正数且 `DA > DF`。
fn validate_tooth_radii(gear: &Gear, label: &str) -> Result<(), String> {
    let (ra, rf) = (gear.major_radius(), gear.minor_radius());
    if !(ra.is_finite() && ra > 0.0 && rf.is_finite() && rf > 0.0) {
        return Err(crate::i18n::t_fmt(
            "cmd.shaft.err.tooth_radii_pos",
            &[
                ("label", label),
                ("da", &trim(ra * 2.0)),
                ("df", &trim(rf * 2.0)),
            ],
        ));
    }
    if ra <= rf + 1e-9 {
        return Err(crate::i18n::t_fmt(
            "cmd.shaft.err.tooth_radii_order",
            &[
                ("label", label),
                ("da", &trim(ra * 2.0)),
                ("df", &trim(rf * 2.0)),
            ],
        ));
    }
    Ok(())
}

/// `SPLINE` 后跟的 token 是否是**齿形段关键字**（`M*`/`Z*`/`EX`/`IN`/`X*`/`DA*`/`DF*`/`ALPHA*`/`BETA*`/`H*`）。
/// 用于区分「渐开线花键齿形段 `SPLINE M3 Z20 …`」与「矩形花键 `SPLINE 6x23x26x6 L30`」（后者原样不动）。
fn tooth_keyword_token(tok: &str) -> bool {
    let u = tok.to_ascii_uppercase();
    u == "EX"
        || u == "IN"
        || u.starts_with('M')
        || u.starts_with('Z')
        || u.starts_with('X')
        || u.starts_with('H')
        || u.starts_with("DA")
        || u.starts_with("DF")
        || u.starts_with("BETA")
        || u.starts_with("ALPHA")
}

fn starts_number(s: &str) -> bool {
    matches!(s.chars().next(), Some(c) if c.is_ascii_digit() || c == '+' || c == '-' || c == '.')
}

/// 关键字后面是否跟了「像数值」的尾巴（空 = 缺值，另行报错；字母开头 = 未知关键字）。
fn looks_like_keyword_value(rest: &str) -> bool {
    let rest = rest.strip_prefix(['=', ':']).unwrap_or(rest);
    rest.is_empty() || starts_number(rest)
}

fn parse_end(text: &str) -> Result<End, String> {
    match text.to_ascii_uppercase().as_str() {
        "L" | "LEFT" | "左" => Ok(End::L),
        "R" | "RIGHT" | "右" => Ok(End::R),
        other => Err(other.to_string()),
    }
}

/// `2@L` → `("2", Some("L"))`；`3` → `("3", None)`。
fn split_end(rest: &str) -> (&str, Option<&str>) {
    match rest.split_once('@') {
        Some((value, end)) => (value, Some(end)),
        None => (rest, None),
    }
}

fn parse_end_token(end: Option<&str>, label: &str, what: &str) -> Result<End, String> {
    match end {
        None => Ok(End::R),
        Some(text) if text.is_empty() => {
            Err(crate::i18n::t_fmt("cmd.shaft.err.end_missing", &[("label", label), ("what", what)]))
        }
        Some(text) => parse_end(text)
            .map_err(|bad| crate::i18n::t_fmt("cmd.shaft.err.end_bad", &[("label", label), ("bad", &bad)])),
    }
}

fn parse_number(value: &str, label: &str, what: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| crate::i18n::t_fmt("cmd.shaft.err.key_value_not_num", &[("label", label), ("what", what), ("value", value)]))
}

fn parse_diameter(token: &str, rest: &str, what: &str, label: &str) -> Result<f64, String> {
    let rest = rest.strip_prefix(['=', ':']).unwrap_or(rest);
    if rest.is_empty() {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.key_no_value", &[("label", label), ("what", what)]));
    }
    if !starts_number(rest) {
        return Err(unknown_keyword(token, label));
    }
    parse_number(rest, label, what)
}

fn parse_ch(token: &str, rest: &str, label: &str) -> Result<Chamfer, String> {
    let rest = rest.strip_prefix(['=', ':']).unwrap_or(rest);
    if rest.is_empty() {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.ch_no_value", &[("label", label)]));
    }
    if !starts_number(rest) {
        return Err(unknown_keyword(token, label));
    }
    let (value, end) = split_end(rest);
    let c = parse_number(value, label, "CH")?;
    if c <= 0.0 {
        return Err(crate::i18n::t_fmt(
            "cmd.shaft.err.ch_positive",
            &[("label", label), ("c", &trim(c))],
        ));
    }
    Ok(Chamfer {
        c,
        end: parse_end_token(end, label, "CH")?,
    })
}

fn parse_ov(token: &str, rest: &str, label: &str) -> Result<Overtravel, String> {
    let rest = rest.strip_prefix(['=', ':']).unwrap_or(rest);
    let (value, end) = split_end(rest);
    let b1 = if value.is_empty() {
        None
    } else {
        if !starts_number(value) {
            return Err(unknown_keyword(token, label));
        }
        let b1 = value
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| crate::i18n::t_fmt("cmd.shaft.err.ov_not_num", &[("label", label), ("value", value)]))?;
        if b1 <= 0.0 {
            return Err(crate::i18n::t_fmt(
                "cmd.shaft.err.ov_positive",
                &[("label", label), ("b1", &trim(b1))],
            ));
        }
        Some(b1)
    };
    Ok(Overtravel {
        b1,
        end: parse_end_token(end, label, "OV")?,
    })
}

/// `ES` 旧关键字已取消：退刀槽用段级 `RL`（或一小段小直径轴段）表示，报错并给写法示例。
fn es_cancelled(label: &str) -> String {
    crate::i18n::t_fmt("cmd.shaft.err.es_cancelled", &[("label", label)])
}

/// `M` / `M1.5`：不写值 = 简化画法；写值 = 螺距 P（> 0）。
fn parse_thread(rest: &str, label: &str) -> Result<Thread, String> {
    let rest = rest.strip_prefix(['=', ':']).unwrap_or(rest);
    if rest.is_empty() {
        return Ok(Thread::default());
    }
    let pitch = parse_number(rest, label, "M")?;
    if pitch <= 0.0 {
        return Err(crate::i18n::t_fmt(
                "cmd.shaft.err.m_pitch",
                &[("label", label), ("pitch", &trim(pitch))],
            ));
    }
    Ok(Thread {
        pitch: Some(pitch),
        ..Thread::default()
    })
}

/// `RO` 收尾档位：一般 / 短（英文 normal / short 也认；`=` `:` 可省）。
fn parse_runout_grade(text: &str, label: &str) -> Result<RunoutGrade, String> {
    let value = text.strip_prefix(['=', ':']).unwrap_or(text);
    match value {
        "" | "一般" | "normal" | "普通" => Ok(RunoutGrade::Normal),
        "短" | "short" => Ok(RunoutGrade::Short),
        other => Err(crate::i18n::t_fmt(
                "cmd.shaft.err.ro_invalid_t3",
                &[("label", label), ("other", other)],
            )),
    }
}

/// `SD` 肩距档位：一般 / 长 / 短（英文 normal / long / short 也认）。
fn parse_shoulder_grade(text: &str, label: &str) -> Result<ShoulderGrade, String> {
    let value = text.strip_prefix(['=', ':']).unwrap_or(text);
    match value {
        "" | "一般" | "normal" | "普通" => Ok(ShoulderGrade::Normal),
        "长" | "long" => Ok(ShoulderGrade::Long),
        "短" | "short" => Ok(ShoulderGrade::Short),
        other => Err(crate::i18n::t_fmt(
                "cmd.shaft.err.sd_invalid_t3",
                &[("label", label), ("other", other)],
            )),
    }
}

/// 档位关键字的尾巴是否像档位：空 / `=` `:` 起头 / 数字 / 中文 / 已知英文档位。
/// 不像（例 `rot`、`SDX`）就交回 `unknown_keyword`，不吞掉普通单词。
fn grade_tail_ok(rest: &str) -> bool {
    let rest = rest.strip_prefix(['=', ':']).unwrap_or(rest);
    rest.is_empty()
        || starts_number(rest)
        || !rest.is_ascii()
        || matches!(
            rest.to_ascii_lowercase().as_str(),
            "normal" | "short" | "long" | "普通"
        )
}

/// `TL` 的数值：`TL20` / `TL=20`；空值/非数报错。
fn parse_thread_length(rest: &str, token: &str, label: &str) -> Result<f64, String> {
    let rest = rest.strip_prefix(['=', ':']).unwrap_or(rest);
    if rest.is_empty() {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.tl_no_value", &[("label", label)]));
    }
    if !starts_number(rest) {
        return Err(unknown_keyword(token, label));
    }
    let value = parse_number(rest, label, "TL")?;
    if value <= 0.0 {
        return Err(crate::i18n::t_fmt(
                "cmd.shaft.err.tl_positive",
                &[("label", label), ("value", &trim(value))],
            ));
    }
    Ok(value)
}

/// `RL` 尺寸参数键（大小写不敏感）：`P` / `g1` / `g2` / `dg` / `r`。
/// 返回（规范小写键，紧跟的附件值）：
/// `P1.5` → `("p", Some("1.5"))`、`g1 2.5` → `("g1", None)`（调用方吞下一个 token）、
/// `g1=2.5` / `g1:2.5` → `("g1", Some("2.5"))`。
fn relief_param_key(token: &str) -> Option<(&'static str, Option<&str>)> {
    let upper = token.to_ascii_uppercase();
    let (key, canonical): (&str, &'static str) = if upper.starts_with("G1") {
        ("G1", "g1")
    } else if upper.starts_with("G2") {
        ("G2", "g2")
    } else if upper.starts_with("DG") {
        ("DG", "dg")
    } else if upper.starts_with('P') {
        ("P", "p")
    } else if upper.starts_with('R') {
        ("R", "r")
    } else {
        return None;
    };
    // 键后必须空 / `=` / `:` / 数值起头；否则不是参数（例 `RO短`、`PH`）。
    let rest = &token[key.len()..];
    let rest = rest.strip_prefix(['=', ':']).unwrap_or(rest);
    if rest.is_empty() {
        return Some((canonical, None));
    }
    if starts_number(rest) {
        return Some((canonical, Some(rest)));
    }
    None
}

/// 把一个 `RL` 尺寸参数写进对应规格（重复报错，不静默覆盖）。
fn set_relief_param(
    spec: &mut Relief,
    key: &str,
    value: f64,
    label: &str,
) -> Result<(), String> {
    let slot = match key {
        "p" => &mut spec.p,
        "g1" => &mut spec.g1,
        "g2" => &mut spec.g2,
        "dg" => &mut spec.dg,
        "r" => &mut spec.r,
        _ => unreachable!("relief_param_key 只返回 p/g1/g2/dg/r"),
    };
    if slot.is_some() {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_param_dup", &[("label", label), ("key", key)]));
    }
    *slot = Some(value);
    Ok(())
}

/// KEY 子关键字（`parse_segment` 用）：键型 A/B/C、键长（裸数字或 `KL…`）、
/// 键尺寸 `b…`/`h…`（`b8h7` 可连写）、`t1`、`@中|@端`。
/// `LA`/`LC` 是旧“槽长”写法，识别后**明确报错**（不静默改语义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyParam {
    Kind,
    /// 合并键型记号（`导向A` / `GUIDED_A` 等；用户 2026-09-25 收尾，键型下拉的 DSL 形式）。
    Style,
    Len,
    B,
    H,
    T1,
    Place,
    /// 双键槽开关：`双槽` / `DOUBLE`。
    Double,
    /// 导向平键（GB/T 1097）：`导向` / `GUIDED` / `GB1097` / `1097`。
    Guided,
    LegacyLa,
    LegacyLc,
}

fn numeric_rest(text: &str) -> bool {
    text.starts_with('=')
        || text.starts_with(':')
        || text.starts_with(|c: char| c.is_ascii_digit() || c == '.' || c == '-' || c == '+')
}

/// 拆分连写的键尺寸 `b8h7` → `("8", "7")`；不是连写返回 None。
fn split_bh(text: &str) -> Option<(&str, &str)> {
    let idx = text.find(['h', 'H'])?;
    let (b, h) = text.split_at(idx);
    let h = &h[1..];
    if b.is_empty() || h.is_empty() {
        return None;
    }
    if b.parse::<f64>().is_ok() && h.parse::<f64>().is_ok() {
        Some((b, h))
    } else {
        None
    }
}

/// 识别 KEY 子关键字；未附值的（`t1`）返回空串，由调用方吃掉下一个 token。
fn key_param_key(token: &str) -> Option<(KeyParam, &str)> {
    if let Some(rest) = token.strip_prefix('@') {
        return Some((KeyParam::Place, rest));
    }
    if token == "双槽" || token.eq_ignore_ascii_case("DOUBLE") {
        return Some((KeyParam::Double, token));
    }
    if matches!(token, "导向" | "導向") || token.eq_ignore_ascii_case("GUIDED") {
        return Some((KeyParam::Guided, token));
    }
    let upper = token.to_ascii_uppercase();
    if matches!(upper.as_str(), "A" | "B" | "C") {
        return Some((KeyParam::Kind, &token[..]));
    }
    // 合并键型记号（`导向A`/`GUIDED_A`；单字母 A/B/C 仍走旧 Kind 路径）。
    // 表外合并记号（如 `导向C`/`GUIDED_X`）也收进 Style 分支，好报「可选键型」而不是「不识别的关键字」。
    let looks_merged = ((token.starts_with("导向") || token.starts_with("導向"))
        && token.chars().count() > 2)
        || (upper.starts_with("GUIDED_") && upper.len() > "GUIDED_".len());
    if token.chars().count() > 1
        && (crate::partgen_keys::key_style_by_token(token).is_some() || looks_merged)
    {
        return Some((KeyParam::Style, &token[..]));
    }
    if let Some(rest) = upper.strip_prefix("KL") {
        if rest.is_empty() || numeric_rest(rest) {
            return Some((KeyParam::Len, &token[2..]));
        }
    }
    if let Some(rest) = upper.strip_prefix("LA") {
        if rest.is_empty() || numeric_rest(rest) {
            return Some((KeyParam::LegacyLa, &token[..]));
        }
    }
    if let Some(rest) = upper.strip_prefix("LC") {
        if rest.is_empty() || numeric_rest(rest) {
            return Some((KeyParam::LegacyLc, &token[..]));
        }
    }
    for (prefix, kind) in [("T1", KeyParam::T1), ("H", KeyParam::H), ("B", KeyParam::B)] {
        let Some(rest_upper) = upper.strip_prefix(prefix) else {
            continue;
        };
        if rest_upper.is_empty() || numeric_rest(rest_upper) {
            return Some((kind, &token[prefix.len()..]));
        }
    }
    // 裸数字 = 键长（例 `KEY A 18`）。
    if token.parse::<f64>().is_ok() {
        return Some((KeyParam::Len, token));
    }
    None
}

/// KEY 位置文本 → [`KeywayPlace`]（DSL 的 `@中/@端` 与 JSON 的 `place` 共用）。
fn parse_keyway_place(text: &str) -> Result<KeywayPlace, String> {
    KeywayPlace::parse(text)
}

/// 键型样式查表（`KeyKind` + 导向 → `partgen_keys::KEY_STYLES` 行）。
fn key_style_for(kind: KeyKind, guided: bool) -> Option<&'static crate::partgen_keys::KeyStyleSpec> {
    crate::partgen_keys::key_style_of(kind.key_type(), guided)
}

/// 样式允许位置的中文（报错用）：`["mid"]` → `中置`，`["mid","end"]` → `中置、端置`。
fn key_places_cn(places: &[&str]) -> String {
    places
        .iter()
        .map(|p| match *p {
            "mid" => "中置",
            _ => "端置",
        })
        .collect::<Vec<_>>()
        .join("、")
}

/// KEY 子关键字 → [`Keyway`]（DSL 与 JSON 共用；无任何参数 = 无轴槽）。
///
/// 三项（段类型）：`key_kind`（A/B/C）、`key_len`（键长 L，必给）、`key_place`（省 = 中置）；
/// `key_b`/`key_h` **可选**：省略 = 按轴段直径查 GB/T 1095 d 列（主路径）；给了就校验
/// 家族存在性与 b×h 配对（与轴径档的一致性在 `keyway_b`/`validate` 查）。
fn assemble_keyway(
    key_kind: Option<KeyKind>,
    key_len: Option<f64>,
    key_b: Option<f64>,
    key_h: Option<f64>,
    key_t1: Option<f64>,
    key_place: Option<KeywayPlace>,
    key_double: bool,
    key_guided: bool,
    label: &str,
) -> Result<Option<Keyway>, String> {
    let any_param = key_kind.is_some()
        || key_len.is_some()
        || key_b.is_some()
        || key_h.is_some()
        || key_t1.is_some()
        || key_place.is_some()
        || key_double
        || key_guided;
    if !any_param {
        return Ok(None);
    }
    let kind = key_kind.ok_or_else(|| {
        crate::i18n::t_fmt("cmd.shaft.err.key_no_type", &[("label", label)])
    })?;
    // b 给了就必须在平键族表里；h 给了一般要求与 b 配对。
    // B/C 型键长会自动按圆弧端折算成显示/开槽长度（见 `Keyway::effective_len`）；
    // 导向（GB/T 1097）只有 A/B，且 b 必须在 1097 表里。
    if key_guided && kind == KeyKind::C {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.key_1097_no_c", &[("label", label)]));
    }
    // 位置/可用选项由键型样式表驱动（导向平键只有中置：用户 2026-09-25 裁定；
    // 加新键型只改 `partgen_keys::KEY_STYLES` 一行）。
    let place = key_place.unwrap_or(KeywayPlace::Mid);
    let spec = key_style_for(kind, key_guided).expect("内建样式覆盖 A/B/C ×(普通/导向)");
    if !spec.places.contains(&place.code()) {
        return Err(crate::i18n::t_fmt(
                "cmd.shaft.err.key_no_place",
                &[
                    ("label", label),
                    ("kind", spec.label),
                    ("places", &key_places_cn(spec.places)),
                    ("tag", spec.place_hint),
                ],
            ));
    }
    if key_double && !spec.allow_double {
        return Err(crate::i18n::t_fmt(
                "cmd.shaft.err.key_no_double",
                &[("label", label), ("kind", spec.label)],
            ));
    }
    if let Some(b) = key_b {
        let std_h = if key_guided {
            crate::partgen_keys::key_1097_row(kind.key_type(), b)
                .map(|r| r.h)
                .ok_or_else(|| {
                    crate::i18n::t_fmt(
                            "cmd.shaft.err.key_1097_b",
                            &[("label", label), ("b", &trim(b))],
                        )
                })?
        } else {
            crate::partgen_keys::key_1096_h(kind.key_type(), b).ok_or_else(|| {
                crate::i18n::t_fmt(
                        "cmd.shaft.err.key_1096_b",
                        &[("label", label), ("kind", &kind.cn()), ("b", &trim(b))],
                    )
            })?
        };
        if let Some(h) = key_h {
            if (h - std_h).abs() > 1e-9 {
                return Err(crate::i18n::t_fmt(
                        "cmd.shaft.err.key_bh_pair",
                        &[
                            ("label", label),
                            ("b", &trim(b)),
                            ("h", &trim(h)),
                            ("kind", &kind.cn()),
                            ("std_h", &trim(std_h)),
                        ],
                    ));
            }
        }
    }
    let l = key_len.ok_or_else(|| {
        crate::i18n::t_fmt("cmd.shaft.err.key_no_len", &[("label", label)])
    })?;
    Ok(Some(Keyway {
        kind,
        l,
        place,
        b: key_b,
        h: key_h,
        t1: key_t1,
        double: key_double,
        guided: key_guided,
    }))
}

fn parse_segment(chunk: &str, label: &str, program: &mut Program) -> Result<Segment, String> {
    let tokens: Vec<&str> = chunk.split_whitespace().collect();
    let (mut s, mut e, mut l) = (None, None, None);
    let mut ch: Vec<Chamfer> = Vec::new();
    let mut ov: Vec<Overtravel> = Vec::new();
    let mut thread: Option<Thread> = None;
    // 轴槽 KEY（GB/T 1095）：**轴段类型**（与 M/SPLINE 同级）= 键型 A/B/C + 键长 L + 位置；
    // b×h 由该段直径 d 查 GB/T 1095 d 列自动定（b/h 可选、只作校验/覆盖）。
    let mut key_on = false;
    let mut key_kind: Option<KeyKind> = None;
    let mut key_len: Option<f64> = None;
    let mut key_b: Option<f64> = None;
    let mut key_h: Option<f64> = None;
    let mut key_t1: Option<f64> = None;
    let mut key_place: Option<KeywayPlace> = None;
    let mut key_double = false;
    let mut key_guided = false;
    // `M` 的局部螺纹扩展关键字（用户 2026-09-19 定稿）：`TL` / `RO` / `SD` / `RL`。
    // 段内顺序无关；到段尾统一装进 [`Thread`] 并校验互斥。
    let mut tl: Option<f64> = None;
    let mut ro: Option<RunoutGrade> = None;
    let mut sd: Option<ShoulderGrade> = None;
    // 段级 RL（用户 2026-09-20 定稿）：可贴任意圆柱段左/右端；
    // `P` / `g1` / `g2` / `dg` / `r` 参数跟在某个 `RL` 后面，绑定到最近一个 RL。
    let mut relief_specs: Vec<Relief> = Vec::new();
    let mut pending_relief: Option<usize> = None;
    // GEAR/SPLINE(渐开线齿形段) 子关键字（M/Z/H/BETA/ALPHA/X/DA/DF/EX/IN）先收齐，段内顺序无关；
    // 没有齿形标记时 M = 螺纹。先扫一遍段里有没有齿形标记：有时 M 一律按模数收（保持段内顺序无关）。
    // 注意：`SPLINE 6x23x26x6 L30` 是矩形花键（GB/T 1144），其后跟齿形关键字（M/Z/EX/IN/DA/DF…）
    // 才是渐开线花键齿形段（用户 2026-09-23 统一表达式）。
    let has_gear = tokens.iter().enumerate().any(|(i, t)| {
        t.eq_ignore_ascii_case("GEAR")
            || (t.eq_ignore_ascii_case("SPLINE")
                && tokens.get(i + 1).copied().map(tooth_keyword_token).unwrap_or(false))
    });
    let mut gear_on = false;
    let mut gear_involute = false;
    let mut gear_internal = false;
    let (mut gear_m, mut gear_z, mut gear_h, mut gear_beta, mut gear_alpha) =
        (None, None, None, None, None);
    let mut gear_x: Option<f64> = None;
    let mut gear_da: Option<f64> = None;
    let mut gear_df: Option<f64> = None;
    let mut loose_gear_token: Option<String> = None;
    // SPLINE 段（矩形花键）：`SPLINE <规格>` + `L<满齿段长>` + 可选 `de` 覆盖。
    let mut spline_on = false;
    let mut spline_spec: Option<String> = None;
    let mut spline_de: Option<f64> = None;
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        // 放置参数只允许出现在一段的末尾（`… at x,y rot 30`），吃掉剩余 token。
        if token.eq_ignore_ascii_case("at") || token == "@" {
            parse_placement(&tokens[index..], label, program)?;
            break;
        }
        let upper = token.to_ascii_uppercase();
        if upper == "SPLINE" || upper.starts_with("SPLINE=") || upper.starts_with("SPLINE:") {
            // `SPLINE` + 齿形关键字 = 渐开线花键齿形段；否则是矩形花键规格代号（原样保留）。
            let involute_form = upper == "SPLINE"
                && tokens.get(index + 1).copied().map(tooth_keyword_token).unwrap_or(false);
            if involute_form {
                if gear_on {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "GEAR/SPLINE")]));
                }
                gear_on = true;
                gear_involute = true;
            } else {
                if spline_on {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "SPLINE")]));
                }
                spline_on = true;
                let rest = &token["SPLINE".len()..];
                let rest = rest.strip_prefix(['=', ':']).unwrap_or(rest).trim();
                if rest.is_empty() {
                    index += 1;
                    let spec = tokens.get(index).ok_or_else(|| {
                        crate::i18n::t_fmt("cmd.shaft.err.spline_no_spec", &[("label", label)])
                    })?;
                    spline_spec = Some((*spec).to_string());
                } else {
                    spline_spec = Some(rest.to_string());
                }
            }
        } else if upper.starts_with("DE") {
            if !spline_on {
                return Err(unknown_keyword(token, label));
            }
            if spline_de.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.spline_de_dup", &[("label", label)]));
            }
            let rest = &token[2..];
            let rest = rest.strip_prefix(['=', ':']).unwrap_or(rest);
            let value_text = if rest.is_empty() {
                index += 1;
                tokens
                    .get(index)
                    .ok_or_else(|| {
                        crate::i18n::t_fmt(
                            "cmd.shaft.err.key_no_value",
                            &[("label", label), ("what", "de")],
                        )
                    })?
            } else {
                rest
            };
            let value = parse_number(value_text, label, "de")?;
            spline_de = Some(value);
        } else if upper == "KEY" {
            if key_on {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "KEY")]));
            }
            key_on = true;
        } else if key_on && key_param_key(token).is_some() {
            let (param, attached) = key_param_key(token).expect("key_param_key 已判 Some");
            let name = match param {
                KeyParam::Kind | KeyParam::Style => crate::i18n::t("cmd.shaft.err.keyparam.kind"),
                KeyParam::Len => crate::i18n::t("cmd.shaft.err.keyparam.len"),
                KeyParam::B => "b".to_string(),
                KeyParam::H => "h".to_string(),
                KeyParam::T1 => "t1".to_string(),
                KeyParam::Place => crate::i18n::t("cmd.shaft.err.keyparam.place"),
                KeyParam::Double => crate::i18n::t("cmd.shaft.err.keyparam.double"),
                KeyParam::Guided => crate::i18n::t("cmd.shaft.err.keyparam.guided"),
                KeyParam::LegacyLa => "LA".to_string(),
                KeyParam::LegacyLc => "LC".to_string(),
            };
            if matches!(param, KeyParam::LegacyLa | KeyParam::LegacyLc) {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_la_lc", &[("label", label)]));
            }
            let value_text = if attached.is_empty() {
                index += 1;
                tokens
                    .get(index)
                    .ok_or_else(|| {
                        crate::i18n::t_fmt(
                            "cmd.shaft.err.key_param_no_value",
                            &[("label", label), ("name", &name)],
                        )
                    })?
            } else {
                attached.strip_prefix(['=', ':']).unwrap_or(attached)
            };
            match param {
                KeyParam::Double => {
                    if key_double {
                        return Err(crate::i18n::t_fmt("cmd.shaft.err.key_param_dup", &[("label", label), ("what", &crate::i18n::t("cmd.shaft.err.keyparam.double"))]));
                    }
                    key_double = true;
                }
                KeyParam::Guided => {
                    key_guided = true;
                }
                KeyParam::Place => {
                    let value = parse_keyway_place(value_text)
                        .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", label), ("e", &e)]))?;
                    match key_place {
                        Some(prev) if prev != value => {
                            return Err(crate::i18n::t_fmt(
                                "cmd.shaft.err.key_place_conflict",
                                &[
                                    ("label", label),
                                    ("prev", &prev.cn()),
                                    ("now", &value.cn()),
                                ],
                            ))
                        }
                        _ => key_place = Some(value),
                    }
                }
                KeyParam::Kind => {
                    let value = KeyKind::parse(value_text)
                        .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", label), ("e", &e)]))?;
                    match key_kind {
                        Some(prev) if prev != value => {
                            return Err(crate::i18n::t_fmt(
                                "cmd.shaft.err.key_type_conflict",
                                &[
                                    ("label", label),
                                    ("prev", &prev.cn()),
                                    ("now", &value.cn()),
                                ],
                            ))
                        }
                        _ => key_kind = Some(value),
                    }
                }
                KeyParam::Style => {
                    // 合并键型（`导向A`）= 旧 (键型, `导向` 开关) 的等价写法；两者须一致。
                    let spec = crate::partgen_keys::key_style_by_token(value_text)
                        .ok_or_else(|| {
                            crate::i18n::t_fmt(
                                "cmd.shaft.err.key_type_invalid",
                                &[
                                    ("label", label),
                                    ("value", value_text),
                                    (
                                        "options",
                                        &crate::partgen_keys::KEY_STYLES
                                            .iter()
                                            .map(|s| s.label)
                                            .collect::<Vec<_>>()
                                            .join(" / "),
                                    ),
                                ],
                            )
                        })?;
                    let value = KeyKind::from_key_type(spec.key_type);
                    match key_kind {
                        Some(prev) if prev != value => {
                            return Err(crate::i18n::t_fmt(
                                "cmd.shaft.err.key_type_conflict",
                                &[
                                    ("label", label),
                                    ("prev", &prev.cn()),
                                    ("now", spec.label),
                                ],
                            ))
                        }
                        _ => key_kind = Some(value),
                    }
                    if key_guided && !spec.guided {
                        return Err(crate::i18n::t_fmt(
                            "cmd.shaft.err.key_guide_conflict",
                            &[("label", label), ("kind", spec.label)],
                        ));
                    }
                    if spec.guided {
                        key_guided = true;
                    }
                }
                _ => {
                    // `b8h7` 连写：b 与 h 成对给出的紧凑形式。
                    if param == KeyParam::B {
                        if let Some((b_text, h_text)) = split_bh(value_text) {
                            for (slot, text, what) in [
                                (&mut key_b, b_text, "b"),
                                (&mut key_h, h_text, "h"),
                            ] {
                                let value = parse_number(text, label, what)?;
                                if !value.is_finite() || value <= 0.0 {
                                    return Err(crate::i18n::t_fmt(
                                        "cmd.shaft.err.key_param_positive",
                                        &[
                                            ("label", label),
                                            ("what", what),
                                            ("value", &trim(value)),
                                        ],
                                    ));
                                }
                                if slot.is_some() {
                                    return Err(crate::i18n::t_fmt(
                                        "cmd.shaft.err.key_param_dup",
                                        &[("label", label), ("what", what)],
                                    ));
                                }
                                *slot = Some(value);
                            }
                            index += 1;
                            continue;
                        }
                    }
                    let value = parse_number(value_text, label, &name)?;
                    if !value.is_finite() || value <= 0.0 {
                        return Err(crate::i18n::t_fmt(
                            "cmd.shaft.err.key_param_positive",
                            &[
                                ("label", label),
                                ("what", &name),
                                ("value", &trim(value)),
                            ],
                        ));
                    }
                    let slot = match param {
                        KeyParam::Len => &mut key_len,
                        KeyParam::B => &mut key_b,
                        KeyParam::H => &mut key_h,
                        KeyParam::T1 => &mut key_t1,
                        _ => unreachable!(),
                    };
                    if slot.is_some() {
                        return Err(crate::i18n::t_fmt(
                            "cmd.shaft.err.key_param_dup",
                            &[("label", label), ("what", &name)],
                        ));
                    }
                    *slot = Some(value);
                }
            }
        } else if upper.starts_with("CH") {
            let item = parse_ch(token, &token[2..], label)?;
            if ch.iter().any(|x| x.end == item.end) {
                return Err(crate::i18n::t_fmt(
                        "cmd.shaft.err.ch_dup_end",
                        &[
                            ("label", label),
                            (
                                "end",
                                &crate::i18n::t(match item.end {
                                    End::L => "cmd.shaft.end.l",
                                    End::R => "cmd.shaft.end.r",
                                }),
                            ),
                        ],
                    ));
            }
            ch.push(item);
        } else if upper.starts_with("OV") {
            let item = parse_ov(token, &token[2..], label)?;
            if ov.iter().any(|x| x.end == item.end) {
                return Err(crate::i18n::t_fmt(
                        "cmd.shaft.err.ov_dup_end",
                        &[
                            ("label", label),
                            (
                                "end",
                                &crate::i18n::t(match item.end {
                                    End::L => "cmd.shaft.end.l",
                                    End::R => "cmd.shaft.end.r",
                                }),
                            ),
                        ],
                    ));
            }
            ov.push(item);
        } else if upper.starts_with("ES") {
            // ES 已取消：给指路提示（不静默丢特征，也不当成 E+S 乱解）。
            return Err(es_cancelled(label));
        } else if upper.starts_with("TL") {
            let item = parse_thread_length(&token[2..], token, label)?;
            if tl.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "TL")]));
            }
            tl = Some(item);
        } else if upper.starts_with("RO") && grade_tail_ok(&token[2..]) {
            if ro.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "RO")]));
            }
            let rest = &token[2..];
            let rest = rest.strip_prefix(['=', ':']).unwrap_or(rest);
            if starts_number(rest) {
                return Err(crate::i18n::t_fmt(
                        "cmd.shaft.err.ro_invalid",
                        &[("label", label), ("rest", rest)],
                    ));
            }
            ro = Some(parse_runout_grade(rest, label)?);
        } else if upper.starts_with("SD") && grade_tail_ok(&token[2..]) {
            if sd.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "SD")]));
            }
            let rest = &token[2..];
            let rest = rest.strip_prefix(['=', ':']).unwrap_or(rest);
            if starts_number(rest) {
                return Err(crate::i18n::t_fmt(
                        "cmd.shaft.err.sd_invalid",
                        &[("label", label), ("rest", rest)],
                    ));
            }
            sd = Some(parse_shoulder_grade(rest, label)?);
        } else if upper == "RL"
            || upper.starts_with("RL@")
            || upper.starts_with("RL=")
            || upper.starts_with("RL:")
        {
            let rest = &token[2..];
            let (value, end) = split_end(rest);
            if !value.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_no_value", &[("label", label)]));
            }
            let end = parse_end_token(end, label, "RL")?;
            if relief_specs.iter().any(|r| r.end == end) {
                return Err(crate::i18n::t_fmt(
                        "cmd.shaft.err.rl_dup_end",
                        &[
                            ("label", label),
                            (
                                "end",
                                &crate::i18n::t(match end {
                                    End::L => "cmd.shaft.end.l",
                                    End::R => "cmd.shaft.end.r",
                                }),
                            ),
                        ],
                    ));
            }
            relief_specs.push(Relief {
                end,
                p: None,
                g1: None,
                g2: None,
                dg: None,
                r: None,
            });
            pending_relief = Some(relief_specs.len() - 1);
        } else if let Some((key, attached)) = relief_param_key(token) {
            // `P1.5` / `g1=2.5` / `g1 2.5`：一律绑定到最近一个 RL。
            let spec_index = pending_relief.ok_or_else(|| {
                crate::i18n::t_fmt(
                        "cmd.shaft.err.rl_param_order",
                        &[("label", label), ("key", key)],
                    )
            })?;
            let value_text = match attached {
                Some(rest) => rest,
                None => {
                    index += 1;
                    tokens
                        .get(index)
                        .ok_or_else(|| {
                        crate::i18n::t_fmt(
                            "cmd.shaft.err.rl_param_no_value",
                            &[("label", label), ("key", key)],
                        )
                    })?
                }
            };
            let value = parse_number(value_text, label, key)?;
            if value <= 0.0 {
                return Err(crate::i18n::t_fmt(
                        "cmd.shaft.err.rl_param_positive",
                        &[("label", label), ("key", key), ("value", &trim(value))],
                    ));
            }
            set_relief_param(&mut relief_specs[spec_index], key, value, label)?;
        } else if upper == "GEAR" {
            if gear_on {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "GEAR")]));
            }
            gear_on = true;
        } else if upper.starts_with("GEAR") {
            return Err(unknown_keyword(token, label));
        } else if upper == "EX" || upper == "IN" {
            if !gear_on {
                return Err(unknown_keyword(token, label));
            }
            gear_internal = upper == "IN";
        } else if upper.starts_with("DA") {
            if !gear_on {
                return Err(unknown_keyword(token, label));
            }
            if gear_da.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "DA")]));
            }
            gear_da = Some(parse_gear_number(token, 2, "DA", label)?);
        } else if upper.starts_with("DF") {
            if !gear_on {
                return Err(unknown_keyword(token, label));
            }
            if gear_df.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "DF")]));
            }
            gear_df = Some(parse_gear_number(token, 2, "DF", label)?);
        } else if upper.starts_with('X') {
            if !gear_on {
                return Err(unknown_keyword(token, label));
            }
            if gear_x.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "X")]));
            }
            gear_x = Some(parse_gear_number(token, 1, "X", label)?);
        } else if upper.starts_with('M') {
            if has_gear {
                let value = parse_gear_number(token, 1, "M", label)?;
                if gear_m.is_some() {
                    return Err(crate::i18n::t_fmt(
                            "cmd.shaft.err.m_gear_dup",
                            &[("label", label)],
                        ));
                }
                gear_m = Some(value);
            } else {
                if thread.is_some() {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "M")]));
                }
                thread = Some(parse_thread(&token[1..], label)?);
            }
        } else if upper.starts_with('Z') {
            let rest = parse_gear_value(token, 1, "Z", label)?;
            let value: u32 = rest.parse().map_err(|_| {
                crate::i18n::t_fmt(
                "cmd.shaft.err.z_not_int",
                &[("label", label), ("rest", rest)],
            )
            })?;
            if gear_z.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "Z")]));
            }
            gear_z = Some(value);
            remember_loose_gear_token(&mut loose_gear_token, token);
        } else if upper.starts_with('H') {
            let value = parse_gear_number(token, 1, "H", label)?;
            if gear_h.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "H")]));
            }
            gear_h = Some(value);
            remember_loose_gear_token(&mut loose_gear_token, token);
        } else if upper.starts_with("BETA") {
            let value = parse_gear_number(token, 4, "BETA", label)?;
            if gear_beta.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "BETA")]));
            }
            gear_beta = Some(value);
            remember_loose_gear_token(&mut loose_gear_token, token);
        } else if upper.starts_with("ALPHA") {
            let value = parse_gear_number(token, 5, "ALPHA", label)?;
            if gear_alpha.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "ALPHA")]));
            }
            gear_alpha = Some(value);
            remember_loose_gear_token(&mut loose_gear_token, token);
        } else if upper.starts_with('S') {
            if !looks_like_keyword_value(&token[1..]) {
                return Err(unknown_keyword(token, label));
            }
            if s.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "S")]));
            }
            s = Some(parse_diameter(token, &token[1..], "S", label)?);
        } else if upper.starts_with('E') {
            if !looks_like_keyword_value(&token[1..]) {
                return Err(unknown_keyword(token, label));
            }
            if e.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "E")]));
            }
            e = Some(parse_diameter(token, &token[1..], "E", label)?);
        } else if upper.starts_with('L') {
            if !looks_like_keyword_value(&token[1..]) {
                return Err(unknown_keyword(token, label));
            }
            if l.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_dup", &[("label", label), ("what", "L")]));
            }
            l = Some(parse_diameter(token, &token[1..], "L", label)?);
        } else {
            return Err(unknown_keyword(token, label));
        }
        index += 1;
    }
    // ── KEY 轴槽（GB/T 1095）：组装子关键字；键型/键尺寸 b×h/键长必给。──
    let keyway = assemble_keyway(key_kind, key_len, key_b, key_h, key_t1, key_place, key_double, key_guided, label)?;
    if keyway.is_some() && (gear_on || spline_on) {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.key_gear_conflict", &[("label", label)]));
    }
    if !gear_on {
        // ── SPLINE 段（矩形花键）：直径由规格代号导出（不给 S/E）；L 必给；
        //    与 CH/OV/RL/M/GEAR 互斥（引入倒角由相邻段的 CH 表达，见 `spline.rs`）──
        if spline_on {
            if let Some(token) = loose_gear_token {
                return Err(unknown_keyword(&token, label));
            }
            if !ch.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.combo_spline_ch", &[("label", label)]));
            }
            if !ov.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.combo_spline_ov", &[("label", label)]));
            }
            if !relief_specs.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.combo_spline_rl", &[("label", label)]));
            }
            if thread.is_some() || tl.is_some() || ro.is_some() || sd.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.combo_spline_m", &[("label", label)]));
            }
            if s.is_some() || e.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.spline_no_se", &[("label", label)]));
            }
            let spec = spline_spec.ok_or_else(|| {
                crate::i18n::t_fmt("cmd.shaft.err.spline_no_spec", &[("label", label)])
            })?;
            // 先校验规格代号 —— 比「缺 L」更切题（例如 `SPLINE L30` 里 L30 不是代号）。
            if let Err(e) = crate::spline::parse_code(&spec) {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", label), ("e", &e)]));
            }
            let len = l.ok_or_else(|| {
                crate::i18n::t_fmt("cmd.shaft.err.spline_no_len", &[("label", label)])
            })?;
            let spline = crate::spline::RectSpline::from_code(&spec, spline_de, len)
                .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", label), ("e", &e)]))?;
            return Ok(Segment {
                s: spline.big,
                e: spline.big,
                l: spline.segment_len(),
                ch: Vec::new(),
                ov: Vec::new(),
                relief: Vec::new(),
                thread: None,
                gear: None,
                spline: Some(spline),
                keyway: None,
            });
        }
        // 没有 GEAR 的 Z/H/BETA 仍按不识别的关键字报（不静默忽略）。
        if let Some(token) = loose_gear_token {
            return Err(unknown_keyword(&token, label));
        }
        if thread.is_none() && (tl.is_some() || ro.is_some() || sd.is_some()) {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.tlro_needs_m", &[("label", label)]));
        }
        let rl_r = relief_specs.iter().any(|r| r.end == End::R);
        if let Some(t) = &mut thread {
            t.tl = tl;
            if let Some(ro) = ro {
                t.runout = ro;
            }
            if let Some(sd) = sd {
                t.shoulder = sd;
            }
            // `M` 段的 `RL`（右端）= 螺纹退刀槽收尾（旧口径，表 2 按螺距查，
            // 行为保持不变）；`RL@L` = 段级左端槽，仍走 `Segment.relief`。
            if rl_r {
                let spec = relief_specs
                    .iter()
                    .find(|r| r.end == End::R)
                    .expect("rl_r = true");
                if spec.has_overrides() {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.m_rl_override", &[("label", label)]));
                }
                if t.runout != RunoutGrade::Normal || t.shoulder != ShoulderGrade::Normal {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_ro_exclusive", &[("label", label)]));
                }
                t.relief = true;
            }
        }
        if thread.is_some() && !ov.is_empty() {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.m_ov_conflict", &[("label", label)]));
        }
        if keyway.is_some()
            && (thread.is_some() || !ov.is_empty() || !relief_specs.is_empty())
        {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.key_mrl_conflict", &[("label", label)]));
        }
        // `M` 段右端 RL 走 `Thread.relief`（不重复存进段级 relief）；其余都进段级。
        let relief: Vec<Relief> = if thread.is_some() && rl_r {
            relief_specs
                .into_iter()
                .filter(|r| r.end != End::R)
                .collect()
        } else {
            relief_specs
        };
        let s = s.ok_or_else(|| crate::i18n::t_fmt("cmd.shaft.err.missing_s", &[("label", label)]))?;
        // E 省略 = 圆柱段；L 必给。
        let l = l.ok_or_else(|| crate::i18n::t_fmt("cmd.shaft.err.missing_l", &[("label", label)]))?;
        return Ok(Segment {
            s,
            e: e.unwrap_or(s),
            l,
            ch,
            ov,
            relief,
            thread,
            gear: None,
            spline: None,
            keyway,
        });
    }
    // ── 齿轮段：直径由 M·Z 导出、长度用 H；CH/OV/M 同段冲突 ──
    if s.is_some() || e.is_some() {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_no_se", &[("label", label)]));
    }
    if l.is_some() {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_use_h", &[("label", label)]));
    }
    if !ch.is_empty() {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_no_ch", &[("label", label)]));
    }
    if !ov.is_empty() {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_no_ov", &[("label", label)]));
    }
    if thread.is_some() {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_no_m", &[("label", label)]));
    }
    if tl.is_some() || ro.is_some() || sd.is_some() {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_no_tlro", &[("label", label)]));
    }
    if !relief_specs.is_empty() {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_no_rl", &[("label", label)]));
    }
    let m = gear_m.ok_or_else(|| crate::i18n::t_fmt("cmd.shaft.err.gear_no_missing", &[("label", label)]))?;
    let z = gear_z.ok_or_else(|| crate::i18n::t_fmt("cmd.shaft.err.gear_no_z", &[("label", label)]))?;
    let beta_deg = gear_beta.unwrap_or(0.0);
    if beta_deg.abs() > 1e-9 {
        return Err(crate::i18n::t_fmt(
                "cmd.shaft.err.beta_spur",
                &[("label", label), ("beta", &trim(beta_deg))],
            ));
    }
    let gear = Gear {
        m,
        z,
        h: gear_h,
        beta_deg,
        alpha_deg: gear_alpha.unwrap_or_else(default_alpha_deg),
        involute: gear_involute,
        kind: if gear_internal {
            GearKind::Internal
        } else {
            GearKind::External
        },
        x: gear_x.unwrap_or(0.0),
        da: gear_da,
        df: gear_df,
    };
    let params = gear.params();
    params
        .validate()
        .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.gear_seg", &[("label", label), ("e", &e)]))?;
    validate_tooth_radii(&gear, label)?;
    let d = params.d();
    Ok(Segment {
        s: d,
        e: d,
        l: gear.width(),
        ch,
        ov,
        relief: Vec::new(),
        thread,
        gear: Some(gear),
        spline: None,
        keyway: None,
    })
}

/// GEAR 子关键字（M/Z/H/BETA/ALPHA）：`M=5` / `M5` 都收；缺值报错。
fn parse_gear_value<'a>(
    token: &'a str,
    prefix_len: usize,
    what: &str,
    label: &str,
) -> Result<&'a str, String> {
    let rest = &token[prefix_len..];
    let rest = rest.strip_prefix(['=', ':']).unwrap_or(rest);
    if rest.is_empty() {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.key_no_value", &[("label", label), ("what", what)]));
    }
    Ok(rest)
}

fn parse_gear_number(token: &str, prefix_len: usize, what: &str, label: &str) -> Result<f64, String> {
    let rest = parse_gear_value(token, prefix_len, what, label)?;
    parse_number(rest, label, what)
}

/// 没有 GEAR 时记住第一个 Z/H/BETA 原文（到段尾统一报不识别的关键字/缺 GEAR）。
fn remember_loose_gear_token(slot: &mut Option<String>, token: &str) {
    if slot.is_none() {
        *slot = Some(token.to_string());
    }
}

fn is_rot(token: &str) -> bool {
    token.eq_ignore_ascii_case("rot") || token.eq_ignore_ascii_case("rotation")
}

fn number_or_err(text: &str, what: &str, label: &str) -> Result<f64, String> {
    text.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| crate::i18n::t_fmt(
            "cmd.shaft.err.field_not_number",
            &[("label", label), ("what", what), ("text", text)],
        ))
}

fn parse_placement(tokens: &[&str], label: &str, program: &mut Program) -> Result<(), String> {
    if program.at.is_some() || program.rot.is_some() {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.place_dup", &[("label", label)]));
    }
    let at_what = crate::i18n::t("cmd.shaft.err.field_at");
    let mut index = 0;
    if index < tokens.len() && (tokens[index].eq_ignore_ascii_case("at") || tokens[index] == "@") {
        index += 1;
    }
    if index < tokens.len() && !is_rot(tokens[index]) {
        let token = tokens[index];
        let (x, y) = if let Some((a, b)) = token.split_once(',') {
            (
                number_or_err(a, &at_what, label)?,
                number_or_err(b, &at_what, label)?,
            )
        } else {
            let b = tokens
                .get(index + 1)
                .ok_or_else(|| crate::i18n::t_fmt("cmd.shaft.err.at_no_y", &[("label", label)]))?;
            if is_rot(b) {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.at_no_y", &[("label", label)]));
            }
            (
                number_or_err(token, &at_what, label)?,
                number_or_err(b, &at_what, label)?,
            )
        };
        program.at = Some([x, y]);
        index += if token.contains(',') { 1 } else { 2 };
    }
    if index < tokens.len() && is_rot(tokens[index]) {
        index += 1;
        let value = tokens
            .get(index)
            .ok_or_else(|| crate::i18n::t_fmt("cmd.shaft.err.rot_no_value", &[("label", label)]))?;
        program.rot = Some(number_or_err(value, "rot", label)?);
        index += 1;
    }
    if index < tokens.len() {
        return Err(crate::i18n::t_fmt(
                "cmd.shaft.err.place_unknown",
                &[("label", label), ("word", tokens[index])],
            ));
    }
    Ok(())
}

// ══════════════════════════════════════════════════════════════════════════
// 解析：JSON（同一模型）
// ══════════════════════════════════════════════════════════════════════════

/// 字段允许单对象或数组（`"ov":{"b1":3}` 与 `"ov":[{"b1":3}]` 都收）。
fn one_or_many<'de, D, T>(de: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Raw<T> {
        // 数组必须先试：结构体字段都有默认值时，serde 会把空数组 `[]` 当成
        // 「全默认的结构体」反序列化（`One` 会误吞空数组）——写成数组就一定是数组。
        Many(Vec<T>),
        One(T),
    }
    Ok(match Option::<Raw<T>>::deserialize(de)? {
        None => Vec::new(),
        Some(Raw::One(value)) => vec![value],
        Some(Raw::Many(values)) => values,
    })
}

#[derive(serde::Deserialize)]
struct JsonProgram {
    #[serde(default)]
    segments: Vec<JsonSegment>,
    #[serde(default, deserialize_with = "json_point")]
    at: Option<[f64; 2]>,
    #[serde(default)]
    rot: Option<f64>,
    /// 视图：`normal|section` 或中文名；缺省 = 常规（`both`/`双` 已移除：明确报错）。
    #[serde(default)]
    view: Option<String>,
}

#[derive(serde::Deserialize)]
struct JsonSegment {
    #[serde(default)]
    s: Option<f64>,
    #[serde(default)]
    e: Option<f64>,
    #[serde(default)]
    l: Option<f64>,
    #[serde(default, deserialize_with = "one_or_many")]
    ch: Vec<JsonChamfer>,
    #[serde(default, deserialize_with = "one_or_many")]
    ov: Vec<JsonOvertravel>,
    /// 段级退刀槽（`RL@L`/`RL@R`）：对象或数组；`M` 段右端收尾仍走 `thread.relief`。
    #[serde(default, deserialize_with = "one_or_many")]
    relief: Vec<JsonRelief>,
    /// 旧字段：`ES` 退刀槽已取消 → 出现就报错指路（不静默丢特征）。
    #[serde(default)]
    es: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "json_thread")]
    thread: Option<Thread>,
    #[serde(default)]
    gear: Option<JsonGear>,
    /// 矩形花键段（`SPLINE`）：序列化回传 `{spec,len,de,...}` 或直接给 N/d/D/B。
    #[serde(default)]
    spline: Option<JsonSpline>,
    /// 轴槽（`KEY`）：`{"type":"A","l":18,"place":"mid","b":8,"t1":null}`；
    /// `b` 必给、`h` 可选（只作 b×h 配对校验）、`t1` 可选；旧 `la`/`lc` 字段明确报错。
    #[serde(default)]
    keyway: Option<JsonKeyway>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonKeyway {
    /// 键型 A/B/C（别名 `kind`）。
    #[serde(rename = "type", alias = "kind")]
    kind: String,
    /// 键长 L（标准系列；别名 `len`）。
    #[serde(default, alias = "len")]
    l: Option<f64>,
    /// 位置：`mid`/`end`（中文「中/端」也认）；省 = 中置。
    #[serde(default)]
    place: Option<String>,
    /// 键宽 b（可选：省略 = 按所在轴段直径查 GB/T 1095 d 列；给了必须等于该档标准 b）。
    #[serde(default)]
    b: Option<f64>,
    /// 键高 h（可选，只作 b×h 配对校验）。
    #[serde(default)]
    h: Option<f64>,
    /// 显式 t₁（缺省 = 按 b 查 GB/T 1095 表）。
    #[serde(default)]
    t1: Option<f64>,
    /// 双键槽（绕轴心 180° 对置；仅剖视图体现；约 1.5 倍单键转矩）。
    #[serde(default)]
    double: bool,
    /// 导向平键（GB/T 1097）：槽上带 2 个固定螺钉螺纹孔；槽长 = 键长 L；只 A/B 型、只有中置。
    #[serde(default)]
    guided: bool,
}

/// JSON 形式的花键段：`{"spec":"6x23x26x6","len":30,"de":63}`；
/// 序列化回传的是 `{n,d,big,b,de,len}`（`RectSpline` 的字段）。
#[derive(serde::Deserialize)]
struct JsonSpline {
    #[serde(default)]
    spec: Option<String>,
    #[serde(default)]
    n: Option<u32>,
    /// 小径 d（`RectSpline` 字段）。
    #[serde(default)]
    d: Option<f64>,
    /// 大径 D（`RectSpline` 字段）。
    #[serde(default)]
    big: Option<f64>,
    /// 键宽 B。
    #[serde(default)]
    b: Option<f64>,
    /// 滚刀外径 de。
    #[serde(default)]
    de: Option<f64>,
    /// 满齿段长 L（别名 `l`/`L`）。
    #[serde(default, alias = "l", alias = "L")]
    len: Option<f64>,
}

#[derive(serde::Deserialize)]
struct JsonChamfer {
    c: f64,
    #[serde(default)]
    end: Option<String>,
}

#[derive(serde::Deserialize)]
struct JsonOvertravel {
    #[serde(default)]
    b1: Option<f64>,
    #[serde(default)]
    end: Option<String>,
}

#[derive(serde::Deserialize)]
struct JsonRelief {
    #[serde(default)]
    end: Option<String>,
    #[serde(default)]
    p: Option<f64>,
    #[serde(default)]
    g1: Option<f64>,
    #[serde(default)]
    g2: Option<f64>,
    #[serde(default)]
    dg: Option<f64>,
    #[serde(default)]
    r: Option<f64>,
}

#[derive(serde::Deserialize)]
struct JsonGear {
    m: f64,
    z: u32,
    #[serde(default)]
    h: Option<f64>,
    #[serde(default)]
    beta: Option<f64>,
    /// 基准齿形角 α（度）；缺省 = 20°。
    #[serde(default)]
    alpha: Option<f64>,
    /// 齿形标记：缺省 `false` = GEAR（常规侧视图画内侧细实线）；
    /// `true` = SPLINE（渐开线花键，不画）。
    #[serde(default)]
    involute: Option<bool>,
    /// 内/外（`"internal"`/`"external"`；缺省 external）。
    #[serde(default)]
    kind: Option<String>,
    /// 变位系数（缺省 0）。
    #[serde(default)]
    x: Option<f64>,
    /// 大径 / 小径（缺省按 m/z/x 推）。
    #[serde(default)]
    da: Option<f64>,
    #[serde(default)]
    df: Option<f64>,
}

/// `thread` 允许：`true` / `1.5`（螺距）/ `{"p":1.5}` / `{"pitch":1.5}` / `"M1.5"`，
/// 以及带新关键字的对象：`{"p":1.5,"tl":20,"runout":"short","shoulder":"long","relief":false}`
/// （`RO`/`SD`/`rl` 别名也认；档位中英文都收）。
fn json_thread<'de, D>(de: D) -> Result<Option<Thread>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Raw {
        On(bool),
        Pitch(f64),
        Text(String),
        Obj {
            #[serde(default)]
            p: Option<f64>,
            #[serde(default)]
            pitch: Option<f64>,
            #[serde(default)]
            tl: Option<f64>,
            #[serde(default, alias = "RO")]
            runout: Option<String>,
            #[serde(default, alias = "SD")]
            shoulder: Option<String>,
            #[serde(default)]
            relief: Option<bool>,
            #[serde(default)]
            rl: Option<bool>,
        },
    }
    let raw = match Option::<Raw>::deserialize(de)? {
        None => return Ok(None),
        Some(raw) => raw,
    };
    let make = |pitch: Option<f64>| Thread {
        pitch,
        ..Thread::default()
    };
    match raw {
        Raw::On(false) => Ok(None),
        Raw::On(true) => Ok(Some(make(None))),
        Raw::Pitch(pitch) => Ok(Some(make(Some(pitch)))),
        Raw::Obj {
            p,
            pitch,
            tl,
            runout,
            shoulder,
            relief,
            rl,
        } => {
            let mut thread = make(p.or(pitch));
            thread.tl = tl;
            if let Some(text) = runout {
                thread.runout = parse_runout_grade(&text, "JSON thread")
                    .map_err(serde::de::Error::custom)?;
            }
            if let Some(text) = shoulder {
                thread.shoulder = parse_shoulder_grade(&text, "JSON thread")
                    .map_err(serde::de::Error::custom)?;
            }
            thread.relief = relief.or(rl).unwrap_or(false);
            Ok(Some(thread))
        }
        Raw::Text(text) => {
            let text = text.trim();
            let text = text
                .strip_prefix(['M', 'm'])
                .map(str::trim)
                .unwrap_or(text);
            if text.is_empty() {
                return Ok(Some(make(None)));
            }
            let pitch = text.parse::<f64>().map_err(|_| {
                serde::de::Error::custom(crate::i18n::t_fmt("cmd.shaft.err.thread_str_not_number", &[("text", &text.to_string())]))
            })?;
            Ok(Some(make(Some(pitch))))
        }
    }
}

/// `at` 允许 `[x,y]` / `"x,y"` / `{"x":…,"y":…}`。
fn json_point<'de, D>(de: D) -> Result<Option<[f64; 2]>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Pair([f64; 2]),
        Text(String),
        Xy { x: f64, y: f64 },
    }
    match Option::<Raw>::deserialize(de)? {
        None => Ok(None),
        Some(Raw::Pair(pair)) => Ok(Some(pair)),
        Some(Raw::Xy { x, y }) => Ok(Some([x, y])),
        Some(Raw::Text(text)) => {
            let (x, y) = text
                .split_once(',')
                .ok_or_else(|| serde::de::Error::custom(crate::i18n::t("cmd.shaft.err.json_at_format")))?;
            let x = x
                .trim()
                .parse::<f64>()
                .map_err(|_| serde::de::Error::custom(crate::i18n::t("cmd.shaft.err.json_at_x")))?;
            let y = y
                .trim()
                .parse::<f64>()
                .map_err(|_| serde::de::Error::custom(crate::i18n::t("cmd.shaft.err.json_at_y")))?;
            Ok(Some([x, y]))
        }
    }
}

fn parse_json(text: &str) -> Result<Program, String> {
    let raw: JsonProgram =
        serde_json::from_str(text).map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.json_parse", &[("e", &e.to_string())]))?;
    if raw.segments.is_empty() {
        return Err(crate::i18n::t("cmd.shaft.err.json_no_segments").into());
    }
    let mut segments = Vec::with_capacity(raw.segments.len());
    for (index, item) in raw.segments.iter().enumerate() {
        let number = index + 1;
        let mut ch = Vec::new();
        for (k, value) in item.ch.iter().enumerate() {
            let end = match &value.end {
                None => End::R,
                Some(text) => parse_end(text).map_err(|bad| {
                    crate::i18n::t_fmt("cmd.shaft.err.json_ch_end_bad", &[("label", &seg_label(number)), ("k", &k.to_string()), ("bad", &bad.to_string())])
                })?,
            };
            if ch.iter().any(|x: &Chamfer| x.end == end) {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.json_ch_dup_end", &[("label", &seg_label(number)), ("p0", &(end_cn(end)).to_string())]));
            }
            ch.push(Chamfer { c: value.c, end });
        }
        let mut ov = Vec::new();
        for (k, value) in item.ov.iter().enumerate() {
            let end = match &value.end {
                None => End::R,
                Some(text) => parse_end(text).map_err(|bad| {
                    crate::i18n::t_fmt("cmd.shaft.err.json_ov_end_bad", &[("label", &seg_label(number)), ("k", &k.to_string()), ("bad", &bad.to_string())])
                })?,
            };
            if ov.iter().any(|x: &Overtravel| x.end == end) {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.json_ov_dup_end", &[("label", &seg_label(number)), ("p0", &(end_cn(end)).to_string())]));
            }
            ov.push(Overtravel { b1: value.b1, end });
        }
        let mut relief = Vec::new();
        for (k, value) in item.relief.iter().enumerate() {
            let end = match &value.end {
                None => End::R,
                Some(text) => parse_end(text).map_err(|bad| {
                    crate::i18n::t_fmt("cmd.shaft.err.json_relief_end_bad", &[("label", &seg_label(number)), ("k", &k.to_string()), ("bad", &bad.to_string())])
                })?,
            };
            if relief.iter().any(|x: &Relief| x.end == end) {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.json_relief_dup_end", &[("label", &seg_label(number)), ("p0", &(end_cn(end)).to_string())]));
            }
            relief.push(Relief {
                end,
                p: value.p,
                g1: value.g1,
                g2: value.g2,
                dg: value.dg,
                r: value.r,
            });
        }
        if item.es.is_some() {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.es_cancelled", &[("label", &seg_label(number))]));
        }
        // 轴槽 KEY：`{"type":"A","l":18,"place":"mid","b":8,"t1":null}`。
        let keyway = match &item.keyway {
            None => None,
            Some(k) => {
                // 新 JSON 形式：`"type":"导向A"`（合并记号）；旧形式 `"type":"A"`+`"guided":true` 兼容。
                let (kind, guided) = match crate::partgen_keys::key_style_by_token(&k.kind) {
                    Some(spec) => (
                        KeyKind::from_key_type(spec.key_type),
                        spec.guided || k.guided,
                    ),
                    None => (
                        KeyKind::parse(&k.kind).map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", &seg_label(number)), ("e", &e.to_string())]))?,
                        k.guided,
                    ),
                };
                let place = match k.place.as_deref() {
                    None => None,
                    Some(text) => Some(
                        parse_keyway_place(text)
                            .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", &seg_label(number)), ("e", &e.to_string())]))?,
                    ),
                };
                assemble_keyway(
                    Some(kind),
                    k.l,
                    k.b,
                    k.h,
                    k.t1,
                    place,
                    k.double,
                    guided,
                    &seg_label(number),
                )?
            }
        };
        let thread = item.thread;
        // `M` 段右端的退刀槽收尾以 `thread.relief` 表示；不要把同一端再写进
        // 段级 `relief`（两套同时画会重复）。
        if thread.as_ref().is_some_and(|t| t.relief) && relief.iter().any(|r| r.end == End::R) {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.json_thread_relief", &[("label", &seg_label(number))]));
        }
        // 花键段：直径由规格代号导出、长度 = L + 收尾 l；与 CH/OV/RL/M 同段冲突。
        if let Some(js) = &item.spline {
            if keyway.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_gear_conflict", &[("label", &seg_label(number))]));
            }
            if !ch.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.combo_spline_ch", &[("label", &seg_label(number))]));
            }
            if !ov.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.combo_spline_ov", &[("label", &seg_label(number))]));
            }
            if !relief.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.combo_spline_rl", &[("label", &seg_label(number))]));
            }
            if thread.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.combo_spline_m", &[("label", &seg_label(number))]));
            }
            let len = js.len.ok_or_else(|| {
                crate::i18n::t_fmt("cmd.shaft.err.spline_no_len", &[("label", &seg_label(number))])
            })?;
            let spline = if let Some(spec) = &js.spec {
                crate::spline::RectSpline::from_code(spec, js.de, len)
            } else {
                let n = js.n.ok_or_else(|| crate::i18n::t_fmt("cmd.shaft.err.spline_no_n", &[("label", &seg_label(number))]))?;
                let d = js.d.ok_or_else(|| crate::i18n::t_fmt("cmd.shaft.err.spline_no_d", &[("label", &seg_label(number))]))?;
                let big = js.big.ok_or_else(|| crate::i18n::t_fmt("cmd.shaft.err.spline_no_big", &[("label", &seg_label(number))]))?;
                let b = js.b.ok_or_else(|| crate::i18n::t_fmt("cmd.shaft.err.spline_no_b", &[("label", &seg_label(number))]))?;
                let de = match js.de {
                    Some(de) => de,
                    None => crate::spline::lookup_de(n, d, big, b).ok_or_else(|| {
                        crate::i18n::t_fmt("cmd.shaft.err.spline_de_not_found", &[("label", &seg_label(number))])
                    })?,
                };
                crate::spline::RectSpline::new(n, d, big, b, de, len)
            }
            .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", &seg_label(number)), ("e", &e.to_string())]))?;
            if let Some(s) = item.s {
                if (s - spline.big).abs() > 1e-9 {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.spline_s_mismatch", &[("label", &seg_label(number)), ("p0", &(trim(s)).to_string()), ("p1", &(trim(spline.big)).to_string())]));
                }
            }
            if let Some(e) = item.e {
                if (e - spline.big).abs() > 1e-9 {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.spline_e_mismatch", &[("label", &seg_label(number)), ("p0", &(trim(e)).to_string()), ("p1", &(trim(spline.big)).to_string())]));
                }
            }
            if let Some(l) = item.l {
                if (l - spline.segment_len()).abs() > 1e-9 {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.spline_l_mismatch", &[("label", &seg_label(number)), ("p0", &(trim(l)).to_string()), ("p1", &(trim(spline.segment_len())).to_string()), ("p2", &(trim(spline.len)).to_string()), ("p3", &(trim(spline.runout())).to_string())]));
                }
            }
            segments.push(Segment {
                s: spline.big,
                e: spline.big,
                l: spline.segment_len(),
                ch: Vec::new(),
                ov: Vec::new(),
                relief: Vec::new(),
                thread: None,
                gear: None,
                spline: Some(spline),
                keyway: None,
            });
            continue;
        }
        // 齿轮段：直径由 m·z 导出、长度用 h；与 CH/OV/M 同段冲突。
        // （序列化回传的 s/e/l 允许出现，但要等于派生值，不允许相互矛盾。）
        if let Some(g) = &item.gear {
            if keyway.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_gear_conflict", &[("label", &seg_label(number))]));
            }
            if !ch.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_no_ch", &[("label", &seg_label(number))]));
            }
            if !ov.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_no_ov", &[("label", &seg_label(number))]));
            }
            if !relief.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_no_rl", &[("label", &seg_label(number))]));
            }
            if thread.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_no_m", &[("label", &seg_label(number))]));
            }
            let kind = match g.kind.as_deref() {
                None => GearKind::External,
                Some(k) => GearKind::parse(k).ok_or_else(|| {
                    crate::i18n::t_fmt("cmd.shaft.err.json_gear_kind_bad", &[("label", &seg_label(number)), ("k", &k.to_string())])
                })?,
            };
            let gear = Gear {
                m: g.m,
                z: g.z,
                h: g.h,
                beta_deg: g.beta.unwrap_or(0.0),
                alpha_deg: g.alpha.unwrap_or_else(default_alpha_deg),
                involute: g.involute.unwrap_or(false),
                kind,
                x: g.x.unwrap_or(0.0),
                da: g.da,
                df: g.df,
            };
            if gear.beta_deg.abs() > 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.json_gear_helical", &[("label", &seg_label(number)), ("p0", &(trim(gear.beta_deg)).to_string())]));
            }
            let params = gear.params();
            params
                .validate()
                .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.gear_seg", &[("label", &seg_label(number)), ("e", &e.to_string())]))?;
            validate_tooth_radii(&gear, &seg_label(number))?;
            let d = params.d();
            if let Some(s) = item.s {
                if (s - d).abs() > 1e-9 {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.json_gear_s_mismatch", &[("label", &seg_label(number)), ("p0", &(trim(s)).to_string()), ("p1", &(trim(d)).to_string())]));
                }
            }
            if let Some(e) = item.e {
                if (e - d).abs() > 1e-9 {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.json_gear_e_mismatch", &[("label", &seg_label(number)), ("p0", &(trim(e)).to_string()), ("p1", &(trim(d)).to_string())]));
                }
            }
            if let Some(l) = item.l {
                if (l - gear.width()).abs() > 1e-9 {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.json_gear_l_mismatch", &[("label", &seg_label(number)), ("p0", &(trim(l)).to_string()), ("p1", &(trim(gear.width())).to_string())]));
                }
            }
            segments.push(Segment {
                s: d,
                e: d,
                l: gear.width(),
                ch,
                ov,
                relief: Vec::new(),
                thread,
                gear: Some(gear),
                spline: None,
                keyway: None,
            });
            continue;
        }
        let s = item
            .s
            .ok_or_else(|| crate::i18n::t_fmt("cmd.shaft.err.missing_s", &[("label", &seg_label(number))]))?;
        let l = item
            .l
            .ok_or_else(|| crate::i18n::t_fmt("cmd.shaft.err.missing_l", &[("label", &seg_label(number))]))?;
        if keyway.is_some() && (thread.is_some() || !ov.is_empty() || !relief.is_empty()) {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.key_mrl_conflict", &[("label", &seg_label(number))]));
        }
        segments.push(Segment {
            s,
            e: item.e.unwrap_or(s),
            l,
            ch,
            ov,
            relief,
            thread,
            gear: None,
            spline: None,
            keyway,
        });
    }
    let view = match &raw.view {
        None => ShaftView::default(),
        Some(text) => ShaftView::parse(text).map_err(|e| format!("JSON：{e}"))?,
    };
    if raw.rot.is_some_and(|v| !v.is_finite()) {
        return Err(crate::i18n::t("cmd.shaft.err.json_rot_not_finite").into());
    }
    Ok(Program {
        segments,
        at: raw.at,
        rot: raw.rot,
        view,
    })
}

// ══════════════════════════════════════════════════════════════════════════
// 合法性检查
// ══════════════════════════════════════════════════════════════════════════

/// 与端面无关的逐段检查（段长/直径/倒角/重复特征）。
/// 端面之间的检查（倒角直径变化量、越程槽台阶…）在 [`build`] 里做。
pub fn validate(program: &Program) -> Result<(), String> {
    if program.segments.is_empty() {
        return Err(crate::i18n::t("cmd.shaft.err.validate_no_segments").into());
    }
    // 每段起点 x（第 1 段左端面 = 0）—— 轴槽中置/端置定位用。
    let mut x0s = Vec::with_capacity(program.segments.len());
    let mut cursor = 0.0;
    for seg in &program.segments {
        x0s.push(cursor);
        cursor += seg.l;
    }
    for (index, seg) in program.segments.iter().enumerate() {
        let number = index + 1;
        if !seg.l.is_finite() || seg.l <= 0.0 {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.seg_len_positive", &[("label", &seg_label(number)), ("p0", &(trim(seg.l)).to_string())]));
        }
        if !seg.s.is_finite() || seg.s <= 0.0 {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.seg_s_positive", &[("label", &seg_label(number)), ("p0", &(trim(seg.s)).to_string())]));
        }
        if !seg.e.is_finite() || seg.e <= 0.0 {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.seg_e_positive", &[("label", &seg_label(number)), ("p0", &(trim(seg.e)).to_string())]));
        }
        for chamfer in &seg.ch {
            if !chamfer.c.is_finite() || chamfer.c <= 0.0 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.chamfer_positive", &[("label", &seg_label(number)), ("p0", &(trim(chamfer.c)).to_string())]));
            }
            if chamfer.c >= seg.l / 2.0 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.chamfer_len_overlap", &[("label", &seg_label(number)), ("p0", &(end_cn(chamfer.end)).to_string()), ("p1", &(trim(chamfer.c)).to_string()), ("p2", &(trim(seg.l)).to_string())]));
            }
        }
        for (a, first) in seg.ch.iter().enumerate() {
            for second in &seg.ch[a + 1..] {
                if first.end == second.end {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.chamfer_dup_end", &[("label", &seg_label(number)), ("p0", &(end_cn(first.end)).to_string())]));
                }
            }
        }
        for ov in &seg.ov {
            if let Some(b1) = ov.b1 {
                if !b1.is_finite() || b1 <= 0.0 {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.ov_b1_positive", &[("label", &seg_label(number)), ("p0", &(trim(b1)).to_string())]));
                }
                if b1 > seg.l + 1e-9 {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.ov_b1_too_long", &[("label", &seg_label(number)), ("p0", &(trim(b1)).to_string()), ("p1", &(trim(seg.l)).to_string())]));
                }
            }
        }
        for (a, first) in seg.ov.iter().enumerate() {
            for second in &seg.ov[a + 1..] {
                if first.end == second.end {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.ov_dup_end", &[("label", &seg_label(number)), ("end", &(end_cn(first.end)).to_string())]));
                }
            }
        }
        // ── 段级退刀槽 RL（任一圆柱段，贴左/右端）：结构 + 取参 + 装得下 ──
        for (a, first) in seg.relief.iter().enumerate() {
            for second in &seg.relief[a + 1..] {
                if first.end == second.end {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.relief_dup_end", &[("label", &seg_label(number)), ("p0", &(end_cn(first.end)).to_string())]));
                }
            }
        }
        if !seg.relief.is_empty() {
            if seg.gear.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_no_rl", &[("label", &seg_label(number))]));
            }
            if seg
                .thread
                .as_ref()
                .is_some_and(|thread| thread.relief)
                && seg.relief.iter().any(|r| r.end == End::R)
            {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.json_thread_relief", &[("label", &seg_label(number))]));
            }
            for spec in &seg.relief {
                if (seg.s - seg.e).abs() > 1e-9 {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_cone_only", &[("label", &seg_label(number)), ("p0", &(trim(seg.s)).to_string()), ("p1", &(trim(seg.e)).to_string())]));
                }
                if index == 0 && spec.end == End::L {
                    return Err(
                        crate::i18n::t_fmt("cmd.shaft.err.rl_free_end_l", &[("label", &seg_label(1))]).to_string()
                    );
                }
                if index + 1 == program.segments.len() && spec.end == End::R {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_free_end_r", &[("label", &seg_label(number))]));
                }
                let dims = resolve_relief_dims(seg, spec, &seg_label(number))?;
                if dims.g2 > seg.l + 1e-9 {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_g2_too_long", &[("label", &seg_label(number)), ("p0", &(trim(dims.g2)).to_string()), ("p1", &(trim(seg.l)).to_string())]));
                }
                // 同一段同端的 CH / OV 是「同端只能有一个槽/倒角」。
                if seg.ch.iter().any(|c| c.end == spec.end) {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_ch_conflict", &[("label", &seg_label(number)), ("p0", &(end_cn(spec.end)).to_string())]));
                }
                if seg.ov.iter().any(|o| o.end == spec.end) {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_ov_conflict", &[("label", &seg_label(number)), ("p0", &(end_cn(spec.end)).to_string())]));
                }
            }
        }
        // ── 轴槽 KEY（GB/T 1095）：结构、装得下、端置位置（几何定位在 `keyway_geom`）──
        if let Some(keyway) = &seg.keyway {
            if seg.gear.is_some() || seg.spline.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_gear_conflict", &[("label", &seg_label(number))]));
            }
            if seg.thread.is_some() || !seg.ov.is_empty() || !seg.relief.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_mrl_conflict", &[("label", &seg_label(number))]));
            }
            if (seg.s - seg.e).abs() > 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_round_only", &[("label", &seg_label(number)), ("p0", &(trim(seg.s)).to_string()), ("p1", &(trim(seg.e)).to_string())]));
            }
            if !keyway.l.is_finite() || keyway.l <= 0.0 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_len_positive", &[("label", &seg_label(number)), ("p0", &(trim(keyway.l)).to_string())]));
            }
            let b = keyway_b(keyway, seg.s).map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", &seg_label(number)), ("e", &e.to_string())]))?;
            // 导向平键（GB/T 1097）：表行 + 长度系列 + 固定螺钉孔校验（用户 2026-09-25 开工）。
            // 可用位置/可用选项由键型样式表驱动（导向平键只有中置：用户 2026-09-25 裁定）。
            if let Some(spec) = key_style_for(keyway.kind, keyway.guided) {
                if keyway.double && !spec.allow_double {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.key_no_double", &[("label", &seg_label(number)), ("kind", &(spec.label).to_string())]));
                }
                if !spec.places.contains(&keyway.place.code()) {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.key_no_place", &[("label", &seg_label(number)), ("kind", &(spec.label).to_string()), ("places", &(key_places_cn(spec.places)).to_string()), ("tag", &(spec.place_hint).to_string())]));
                }
            }
            let row_1097 = if keyway.guided {
                Some(crate::partgen_keys::key_1097_row(keyway.kind.key_type(), b).ok_or_else(
                    || {
                        crate::i18n::t_fmt("cmd.shaft.err.key_1097_b", &[("label", &seg_label(number)), ("b", &(trim(b)).to_string())])
                    },
                )?)
            } else {
                None
            };
            if row_1097.is_none()
                && crate::partgen_keys::key_1096_h(keyway.kind.key_type(), b).is_none()
            {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_1096_b", &[("label", &seg_label(number)), ("kind", &(keyway.kind.cn()).to_string()), ("b", &(trim(b)).to_string())]));
            }
            // h 给了就必须与 b 配对（h 跟 b 走，不允许自由组合）；导向用 1097 表值（与 1096 同值）。
            if let Some(h) = keyway.h {
                let std_h = row_1097
                    .map(|r| r.h)
                    .or_else(|| crate::partgen_keys::key_1096_h(keyway.kind.key_type(), b))
                    .expect("上面已判族表有 b");
                if (h - std_h).abs() > 1e-9 {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.key_pair_invalid", &[("label", &seg_label(number)), ("p0", &(trim(b)).to_string()), ("p1", &(trim(h)).to_string()), ("p2", &(keyway.kind.cn()).to_string()), ("p3", &(trim(b)).to_string()), ("p4", &(trim(std_h)).to_string())]));
                }
            }
            if let Some(row) = row_1097 {
                // L 必须落在 GB/T 1097 长度系列（∩ L<10b）；表外报错并列出可选系列。
                let allowed = crate::partgen_keys::key_1097_l_allowed(b);
                if !allowed.iter().any(|v| (*v - keyway.l).abs() < 1e-9) {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.key_1097_len_bad", &[("label", &seg_label(number)), ("p0", &(trim(keyway.l)).to_string()), ("p1", &(allowed.len()).to_string()), ("p2", &(allowed.iter().map(|v| trim(*v)).collect::<Vec<_>>().join(", ")).to_string())]));
                }
                let len = crate::partgen_keys::key_1097_length_for(keyway.l)
                    .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", &seg_label(number)), ("e", &e.to_string())]))?;
                let major_r = row.d0 / 2.0;
                let minor_r = 0.85 * major_r;
                let cone_h = minor_r / 59f64.to_radians().tan();
                if len.l3 <= major_r + 1e-9 {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.key_screw_out_slot", &[("label", &seg_label(number)), ("p0", &(trim(row.d0)).to_string()), ("p1", &(trim(major_r)).to_string()), ("p2", &(trim(len.l3)).to_string())]));
                }
                // 圆头端（A 型）：孔圈还不得越出槽端圆弧。
                if keyway.kind == KeyKind::A {
                    let along = keyway.l / 2.0 - len.l3; // 孔心到槽中
                    let arc = keyway.l / 2.0 - b / 2.0; // 弧心到槽中
                    if along > arc + 1e-9 {
                        let dy = (b / 2.0).powi(2) - (along - arc).powi(2);
                        if dy <= 0.0 || dy.sqrt() < major_r - 1e-9 {
                            return Err(crate::i18n::t_fmt("cmd.shaft.err.key_screw_out_profile", &[("label", &seg_label(number)), ("p0", &(trim(row.d0)).to_string()), ("p1", &(trim(keyway.l)).to_string()), ("p2", &(trim(b)).to_string()), ("p3", &(trim(len.l3)).to_string())]));
                        }
                    }
                }
            } else {
                crate::partgen_keys::check_length_1096(b, keyway.l)
                    .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.keyway_prefix", &[("label", &seg_label(number)), ("e", &e.to_string())]))?;
            }
            let r = seg.s / 2.0;
            if b / 2.0 >= r - 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_b_ge_d", &[("label", &seg_label(number)), ("p0", &(trim(b)).to_string()), ("p1", &(trim(r * 2.0)).to_string())]));
            }
            let t1 = keyway_t1(b, keyway.t1).map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", &seg_label(number)), ("e", &e.to_string())]))?;
            if !t1.is_finite() || t1 <= 0.0 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_t1_positive", &[("label", &seg_label(number)), ("p0", &(trim(t1)).to_string())]));
            }
            let rk = b / 2.0;
            let sag = r - (r * r - rk * rk).max(0.0).sqrt();
            if t1 <= sag + 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_t1_le_sagitta", &[("label", &seg_label(number)), ("p0", &(trim(t1)).to_string()), ("p1", &(trim(sag)).to_string())]));
            }
            if t1 >= r - 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.key_t1_ge_radius", &[("label", &seg_label(number)), ("p0", &(trim(t1)).to_string()), ("p1", &(trim(r)).to_string())]));
            }
            // 导向固定螺钉孔：孔底（含 118° 钻尖）不得越过轴线。
            if let Some(row) = row_1097 {
                let cone_h = 0.85 * row.d0 / 2.0 / 59f64.to_radians().tan();
                if t1 + row.l0 + cone_h >= r - 1e-9 {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.key_screw_through_axis", &[("label", &seg_label(number)), ("p0", &(trim(row.l0)).to_string()), ("p1", &(trim(t1)).to_string()), ("p2", &(trim(cone_h)).to_string()), ("p3", &(trim(r)).to_string())]));
                }
            }
            let slot_len = keyway.slot_len(t1, b);
            let x0 = x0s[index];
            let ch_l = keyway_chamfer(seg, End::L);
            let ch_r = keyway_chamfer(seg, End::R);
            match keyway.place {
                KeywayPlace::Mid => {
                    let usable = (x0 + seg.l - ch_r) - (x0 + ch_l);
                    if slot_len > usable + 1e-9 {
                        return Err(crate::i18n::t_fmt("cmd.shaft.err.key_mid_no_fit", &[("label", &seg_label(number)), ("p0", &(trim(keyway.l)).to_string()), ("p1", &(trim(usable)).to_string())]));
                    }
                }
                KeywayPlace::End => {
                    if program.segments.len() == 1 {
                        return Err(crate::i18n::t_fmt("cmd.shaft.err.key_single_end_ambiguous", &[("label", &seg_label(number))]));
                    }
                    let side = if index == 0 {
                        End::L
                    } else if index + 1 == program.segments.len() {
                        End::R
                    } else {
                        return Err(crate::i18n::t_fmt("cmd.shaft.err.key_end_place_only", &[("label", &seg_label(number))]));
                    };
                    if slot_len > seg.l + 1e-9 {
                        return Err(crate::i18n::t_fmt("cmd.shaft.err.key_slot_len_too_long", &[("label", &seg_label(number)), ("p0", &(trim(keyway.l)).to_string()), ("p1", &(trim(t1)).to_string()), ("p2", &(trim(slot_len)).to_string()), ("p3", &(trim(seg.l)).to_string())]));
                    }
                    let c = match side {
                        End::L => ch_l,
                        End::R => ch_r,
                    };
                    if c > 0.0 && t1 < c - 1e-9 {
                        return Err(crate::i18n::t_fmt("cmd.shaft.err.key_t1_lt_chamfer", &[("label", &seg_label(number)), ("p0", &(trim(t1)).to_string()), ("p1", &(end_cn(side)).to_string()), ("p2", &(trim(c)).to_string())]));
                    }
                }
            }
        }
        if let Some(thread) = &seg.thread {
            if let Some(pitch) = thread.pitch {
                if !pitch.is_finite() || pitch <= 0.0 {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.m_pitch_positive", &[("label", &seg_label(number)), ("p0", &(trim(pitch)).to_string())]));
                }
                let minor = thread.minor_diameter(seg.s);
                if minor <= 0.0 {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.m_pitch_too_big", &[("label", &seg_label(number)), ("p0", &(trim(pitch)).to_string()), ("p1", &(trim(minor)).to_string())]));
                }
            }
            if (seg.s - seg.e).abs() > 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.m_round_only", &[("label", &seg_label(number)), ("p0", &(trim(seg.s)).to_string()), ("p1", &(trim(seg.e)).to_string())]));
            }
            if !seg.ov.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.m_ov_conflict", &[("label", &seg_label(number))]));
            }
            if seg.gear.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.m_gear_conflict", &[("label", &seg_label(number))]));
            }
            // ── 局部螺纹（`TL` + `RO`/`SD` 或 `RL`）：表 1/表 2 查表 + 几何装得下 ──
            let wants_local = thread.tl.is_some() || thread.relief;
            if thread.relief
                && (thread.runout != RunoutGrade::Normal
                    || thread.shoulder != ShoulderGrade::Normal)
            {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_ro_exclusive", &[("label", &seg_label(number))]));
            }
            if wants_local {
                let pitch = thread.pitch.ok_or_else(|| {
                    crate::i18n::t_fmt("cmd.shaft.err.tlro_need_pitch", &[("label", &seg_label(number))])
                })?;
                if thread.tl.is_some_and(|tl| !tl.is_finite() || tl <= 0.0) {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.tl_positive_plain", &[("label", &seg_label(number))]));
                }
                let tl = thread.tl.unwrap_or(0.0);
                if thread.relief {
                    let row =
                        detail::thread_relief_row(pitch).map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", &seg_label(number)), ("e", &e.to_string())]))?;
                    let dg = seg.s - row.dg_reduction;
                    if dg <= 0.0 {
                        return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_dg_nonpositive", &[("label", &seg_label(number)), ("p0", &(trim(row.dg_reduction)).to_string()), ("p1", &(trim(dg)).to_string())]));
                    }
                    let r1 = thread.minor_radius(seg.s);
                    if r1 <= dg / 2.0 + 1e-9 {
                        return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_dg_too_shallow", &[("label", &seg_label(number)), ("p0", &(trim(dg)).to_string()), ("p1", &(trim(r1 * 2.0)).to_string())]));
                    }
                    let need = row.g2 + tl;
                    if need > seg.l + 1e-9 {
                        return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_g2_tl_too_long", &[("label", &seg_label(number)), ("p0", &(trim(row.g2)).to_string()), ("p1", &(trim(tl)).to_string()), ("p2", &(trim(need)).to_string()), ("p3", &(trim(seg.l)).to_string())]));
                    }
                } else {
                    let row = detail::runout_row_checked(pitch)
                        .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", &seg_label(number)), ("e", &e.to_string())]))?;
                    let x = row.x(thread.runout);
                    let a = row.a(thread.shoulder);
                    if x > a + 1e-9 {
                        return Err(crate::i18n::t_fmt("cmd.shaft.err.ro_gt_sd", &[("label", &seg_label(number)), ("p0", &(trim(x)).to_string()), ("p1", &(trim(a)).to_string())]));
                    }
                    let need = a + tl;
                    if need > seg.l + 1e-9 {
                        return Err(crate::i18n::t_fmt("cmd.shaft.err.sd_tl_too_long", &[("label", &seg_label(number)), ("p0", &(trim(a)).to_string()), ("p1", &(trim(tl)).to_string()), ("p2", &(trim(need)).to_string()), ("p3", &(trim(seg.l)).to_string())]));
                    }
                }
                if seg.ch.iter().any(|c| c.end == End::R) {
                    return Err(crate::i18n::t_fmt("cmd.shaft.err.tlro_no_chamfer_v", &[("label", &seg_label(number))]));
                }
            } else if thread.runout != RunoutGrade::Normal
                || thread.shoulder != ShoulderGrade::Normal
            {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.ro_sd_need_tl", &[("label", &seg_label(number))]));
            }
        }
        if let Some(gear) = &seg.gear {
            gear.params()
                .validate()
                .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.gear_seg", &[("label", &seg_label(number)), ("e", &e.to_string())]))?;
            validate_tooth_radii(gear, &seg_label(number))?;
            if !seg.ch.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_no_ch", &[("label", &seg_label(number))]));
            }
            if !seg.ov.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_no_ov", &[("label", &seg_label(number))]));
            }
            if seg.thread.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_no_m", &[("label", &seg_label(number))]));
            }
        }
        // ── 矩形花键段（SPLINE）：直径由规格导出、段长 = L+l；与 CH/OV/RL/M/GEAR 互斥 ──
        if let Some(spline) = &seg.spline {
            if (seg.s - spline.big).abs() > 1e-9 || (seg.e - spline.big).abs() > 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.spline_se_mismatch", &[("label", &seg_label(number)), ("p0", &(trim(seg.s)).to_string()), ("p1", &(trim(seg.e)).to_string()), ("p2", &(trim(spline.big)).to_string())]));
            }
            if (seg.l - spline.segment_len()).abs() > 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.spline_len_mismatch", &[("label", &seg_label(number)), ("p0", &(trim(seg.l)).to_string()), ("p1", &(trim(spline.segment_len())).to_string()), ("p2", &(trim(spline.len)).to_string()), ("p3", &(trim(spline.runout())).to_string())]));
            }
            if !seg.ch.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.combo_spline_ch", &[("label", &seg_label(number))]));
            }
            if !seg.ov.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.combo_spline_ov", &[("label", &seg_label(number))]));
            }
            if !seg.relief.is_empty() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.combo_spline_rl", &[("label", &seg_label(number))]));
            }
            if seg.thread.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.combo_spline_m", &[("label", &seg_label(number))]));
            }
            if seg.gear.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.spline_gear_conflict", &[("label", &seg_label(number))]));
            }
        }

    }
    Ok(())
}

// ══════════════════════════════════════════════════════════════════════════
// 几何
// ══════════════════════════════════════════════════════════════════════════

/// 构建结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Shaft {
    pub entities: Vec<EntityType>,
    pub total_length: f64,
    pub max_diameter: f64,
    pub segment_count: usize,
}

/// 段内某 x 处的轮廓半径（上半侧；圆锥按线性插值）。
fn radius_at(seg: &Segment, x0: f64, x: f64) -> f64 {
    let r0 = seg.s / 2.0;
    let r1 = seg.e / 2.0;
    r0 + (r1 - r0) * ((x - x0) / seg.l)
}

/// 某端做 C 倒角：返回（轮廓点，端面点）。端面点 = 轮廓点在段内 C 处 − C（45°）。
fn chamfer_geom(seg: &Segment, x0: f64, end: End, c: f64) -> ([f64; 2], [f64; 2]) {
    match end {
        End::L => {
            let x_contour = x0 + c;
            let y_contour = radius_at(seg, x0, x_contour);
            ([x_contour, y_contour], [x0, y_contour - c])
        }
        End::R => {
            let x_face = x0 + seg.l;
            let x_contour = x_face - c;
            let y_contour = radius_at(seg, x0, x_contour);
            ([x_contour, y_contour], [x_face, y_contour - c])
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════
// 轴槽几何（GB/T 1095；口径见 `review/键槽_设计.md`）
// ══════════════════════════════════════════════════════════════════════════

/// 键槽落在本段上的倒角内缩量（倒角根距端面；无 CH = 0）。
/// 中置槽按「倒角根↔段末」净圆柱的中点定位（模板槽中心 = 该段中点 ✓）。
fn keyway_chamfer(seg: &Segment, end: End) -> f64 {
    seg.ch
        .iter()
        .find(|c| c.end == end)
        .map(|c| c.c)
        .unwrap_or(0.0)
}

/// 一个轴槽在局部坐标系里的全部几何量（局部：段左端 x0、轴线 y=0）。
#[derive(Debug, Clone, Copy)]
struct KeywayGeom {
    kind: KeyKind,
    /// 双键槽（绕轴心 180° 对置；仅剖视图体现）。
    double: bool,
    b: f64,
    /// 显示/渲染长度 = `Keyway::effective_len(b)`（B/C 已按圆弧端折算；见模型注释）。
    l: f64,
    /// 实际槽长（中置 = 折算长度；端置 = 折算长度 + t₁）。
    slot_len: f64,
    t1: f64,
    place: KeywayPlace,
    /// `End` 专用：槽开口所在端（左端/右端）。
    side: Option<End>,
    r: f64,
    /// 弦到圆弧中点的矢高 `R − √(R² − (b/2)²)`。
    sag: f64,
    /// 缺口上边线（sagitta 线）高度 = `R − sag`。
    y_sag: f64,
    /// 槽底高度 = `R − t1`。
    floor: f64,
    /// 槽 x 范围（含端置的开口段）。
    slot0: f64,
    slot1: f64,
    /// 端置闭端壁 x（中置 = slot1）。
    wall: f64,
    /// 端置开口端面 x（中置 = 段左端 x0）。
    face: f64,
    /// 键端半圆半径 = b/2。
    rk: f64,
    /// 导向平键（GB/T 1097）固定螺钉孔几何；非导向 = None。
    guided: Option<GuidedHoleGeom>,
}

/// 导向平键（GB/T 1097）轴上固定螺钉孔几何（2 孔，槽底向下）。
#[derive(Debug, Clone, Copy)]
struct GuidedHoleGeom {
    /// 固定螺钉螺纹公称 d0（M3…M12）。
    d0: f64,
    /// 轴上螺纹孔深 L0（自槽底向下，含至 118° 钻尖前的圆柱段）。
    l0: f64,
    /// 简化小径半径 = 0.85·d0/2（源图/键块口径）。
    minor_r: f64,
    /// 螺纹大径半径 = d0/2。
    major_r: f64,
    /// 螺纹可视长度 = L0 − 2P（P 由 d0 查 ISO 724）。
    thread_len: f64,
    /// 118° 钻尖高 = minor_r / tan59°。
    cone_h: f64,
    /// 两孔心 x（全局坐标，与 slot0/slot1 同系）。
    xs: [f64; 2],
}

/// 解算一个 KEY 轴槽：b×h（b 必给）/ t₁（省 = 查 GB/T 1095 表）/ 槽长 / 中置·端置定位。
fn keyway_geom(
    seg: &Segment,
    kw: &Keyway,
    x0: f64,
    index: usize,
    count: usize,
    label: &str,
) -> Result<KeywayGeom, String> {
    let b = keyway_b(kw, seg.s).map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", label), ("e", &e)]))?;
    // 样式表位置约束（导向只有中置；用户 2026-09 裁定）：防住绕过 `validate` 的直调用。
    if let Some(spec) = key_style_for(kw.kind, kw.guided) {
        if !spec.places.contains(&kw.place.code()) {
            return Err(crate::i18n::t_fmt(
                "cmd.shaft.err.key_no_place",
                &[
                    ("label", label),
                    ("kind", spec.label),
                    ("places", &key_places_cn(spec.places)),
                    ("tag", spec.place_hint),
                ],
            ));
        }
    }
    // 导向（GB/T 1097）：查表行 + 长度系列行 + 螺钉粗牙螺距；非导向：沿用 1096 族表检查。
    let guided_src = if kw.guided {
        let row = crate::partgen_keys::key_1097_row(kw.kind.key_type(), b).ok_or_else(|| {
            crate::i18n::t_fmt(
                "cmd.shaft.err.key_1097_b",
                &[("label", label), ("b", &trim(b))],
            )
        })?;
        let len = crate::partgen_keys::key_1097_length_for(kw.l)
            .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", label), ("e", &e)]))?;
        let p = crate::hole::thread_row(row.d0, None)
            .map(|t| t.p)
            .map_err(|e| {
            crate::i18n::t_fmt(
                "cmd.shaft.err.key_1097_screw",
                &[("label", label), ("d0", &trim(row.d0)), ("e", &e)],
            )
        })?;
        Some((row, len, p))
    } else {
        if crate::partgen_keys::key_1096_h(kw.kind.key_type(), b).is_none() {
            return Err(crate::i18n::t_fmt(
                "cmd.shaft.err.key_1096_b",
                &[("label", label), ("kind", &kw.kind.cn()), ("b", &trim(b))],
            ));
        }
        None
    };
    let t1 = keyway_t1(b, kw.t1).map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", label), ("e", &e)]))?;
    let slot_len = kw.slot_len(t1, b);
    let r = seg.s / 2.0;
    let rk = b / 2.0;
    let sag = r - (r * r - rk * rk).max(0.0).sqrt();
    let (side, slot0, slot1, wall, face) = match kw.place {
        KeywayPlace::Mid => {
            let usable0 = x0 + keyway_chamfer(seg, End::L);
            let usable1 = x0 + seg.l - keyway_chamfer(seg, End::R);
            let center = (usable0 + usable1) / 2.0;
            (
                None,
                center - slot_len / 2.0,
                center + slot_len / 2.0,
                center + slot_len / 2.0,
                x0,
            )
        }
        KeywayPlace::End => {
            if index == 0 {
                (Some(End::L), x0, x0 + slot_len, x0 + slot_len, x0)
            } else if index + 1 == count {
                (
                    Some(End::R),
                    x0 + seg.l - slot_len,
                    x0 + seg.l,
                    x0 + seg.l - slot_len,
                    x0 + seg.l,
                )
            } else {
                return Err(crate::i18n::t_fmt(
                        "cmd.shaft.err.key_end_place_only",
                        &[("label", label)],
                    ));
            }
        }
    };
    let guided = guided_src.map(|(row, len, p)| GuidedHoleGeom {
        d0: row.d0,
        l0: row.l0,
        minor_r: 0.85 * row.d0 / 2.0,
        major_r: row.d0 / 2.0,
        thread_len: (row.l0 - 2.0 * p).max(0.0),
        cone_h: 0.85 * row.d0 / 2.0 / 59f64.to_radians().tan(),
        xs: [slot0 + len.l3, slot1 - len.l3],
    });
    Ok(KeywayGeom {
        kind: kw.kind,
        double: kw.double,
        b,
        l: kw.effective_len(b),
        slot_len,
        t1,
        place: kw.place,
        side,
        r,
        sag,
        y_sag: r - sag,
        floor: r - t1,
        slot0,
        slot1,
        wall,
        face,
        rk,
        guided,
    })
}

/// 导向平键（GB/T 1097）常规侧视叠画：键体（A 圆头 / B 平头，长度 = L）+ 2 个固定螺钉孔。
/// 孔圈照模板：小径整圆（实线）+ 大径 3/4 细弧 + 孔中心十字线。
/// 3/4 弧缺口 = 库内共有口径（模板换算 265°→185°，见 `partgen_kit::thread_major_arc`）。
/// **导向只中置**（用户 2026-09 裁定）：键体居中于 `slot0..slot1`（导向时槽长 = 键长）。
fn emit_guided_overlay(
    out: &mut Vec<EntityType>,
    kg: &KeywayGeom,
    gh: &GuidedHoleGeom,
    frame_scale: f64,
) {
    let over = 3.0 * frame_scale;
    let rk = kg.rk;
    let (x_lo, x_hi) = (kg.slot0, kg.slot1);
    match kg.kind {
        KeyKind::A => {
            let (ca, cb) = (x_lo + rk, x_hi - rk);
            out.push(line([ca, rk], [cb, rk], LAYER_MAIN));
            out.push(line([ca, -rk], [cb, -rk], LAYER_MAIN));
            out.push(crate::partgen_kit::arc([ca, 0.0], rk, 90.0, 270.0, LAYER_MAIN));
            out.push(crate::partgen_kit::arc([cb, 0.0], rk, 270.0, 450.0, LAYER_MAIN));
            for x in [ca, cb] {
                out.push(line([x, -(rk + over)], [x, rk + over], LAYER_CENTER));
            }
        }
        KeyKind::B => {
            // B 型平头键：矩形 L×b（1097 B 型键端平头；模板只给 A 型，按标准键形画）。
            out.push(line([x_lo, rk], [x_hi, rk], LAYER_MAIN));
            out.push(line([x_lo, -rk], [x_hi, -rk], LAYER_MAIN));
            out.push(line([x_lo, -rk], [x_lo, rk], LAYER_MAIN));
            out.push(line([x_hi, -rk], [x_hi, rk], LAYER_MAIN));
        }
        KeyKind::C => unreachable!("导向平键没有 C 型（assemble/validate 已拦）"),
    }
    // 固定螺钉孔（轴上螺纹孔）：小径整圆实线 + 大径 3/4 细弧 + 孔中心十字线。
    for x in gh.xs {
        out.push(crate::partgen_kit::circle([x, 0.0], gh.minor_r, LAYER_MAIN));
        out.push(thread_major_arc([x, 0.0], gh.major_r));
        let half = gh.major_r + over;
        out.push(line([x - half, 0.0], [x + half, 0.0], LAYER_CENTER));
        out.push(line([x, -half], [x, half], LAYER_CENTER));
    }
}

/// 导向平键剖视：`[x_start, x_end]` 的剖面线边界路径（沿 2 孔轮廓下凹）+ 孔壁可见线。
/// 可见线（孔壁/118° 钻尖/大径细线/中心线）直接推入 `section_lines`；hatch 边界逐段交给 `notch`。
/// ⚠️ **槽底直线不由本函数画**：槽底是一条贯通槽两端的实线（模板口径，螺钉孔在槽底之下），
/// 由调用方在 `guided_floor` 前后统一推入。
fn guided_floor(
    section_lines: &mut Vec<EntityType>,
    notch: &mut dyn FnMut(HatchEdge),
    kg: &KeywayGeom,
    gh: &GuidedHoleGeom,
    x_start: f64,
    x_end: f64,
    frame_scale: f64,
) {
    let y = kg.floor;
    let y_thr = y - gh.thread_len;
    let y_l0 = y - gh.l0;
    let y_tip = y_l0 - gh.cone_h;
    let mut pts: Vec<[f64; 2]> = vec![[x_start, y]];
    for x in gh.xs {
        pts.extend([
            [x - gh.major_r, y],
            [x - gh.major_r, y_thr],
            [x - gh.minor_r, y_thr],
            [x - gh.minor_r, y_l0],
            [x, y_tip],
            [x + gh.minor_r, y_l0],
            [x + gh.minor_r, y_thr],
            [x + gh.major_r, y_thr],
            [x + gh.major_r, y],
        ]);
    }
    pts.push([x_end, y]);
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        notch(if (a[1] - b[1]).abs() < 1e-12 {
            lr_line(a, b)
        } else {
            HatchEdge::Line { a, b }
        });
    }
    for x in gh.xs {
        // 小径（实线）：孔壁 + 118° 钻尖。
        section_lines.push(line([x - gh.minor_r, y], [x - gh.minor_r, y_l0], LAYER_MAIN));
        section_lines.push(line([x + gh.minor_r, y], [x + gh.minor_r, y_l0], LAYER_MAIN));
        section_lines.push(line([x - gh.minor_r, y_l0], [x, y_tip], LAYER_MAIN));
        section_lines.push(line([x, y_tip], [x + gh.minor_r, y_l0], LAYER_MAIN));
        // 大径（细实线）：螺纹可视长 + 终止线。
        section_lines.push(line([x - gh.major_r, y], [x - gh.major_r, y_thr], LAYER_THIN));
        section_lines.push(line([x + gh.major_r, y], [x + gh.major_r, y_thr], LAYER_THIN));
        section_lines.push(line([x - gh.major_r, y_thr], [x + gh.major_r, y_thr], LAYER_THIN));
        // 孔轴中心线：槽底+3n → 钻尖−3n（模板 x=18.5 竖向中心线 −37.599..−51.365）。
        section_lines.push(line(
            [x, y + 3.0 * frame_scale],
            [x, y_tip - 3.0 * frame_scale],
            LAYER_CENTER,
        ));
    }
}

/// 键叠画（仅常规侧视图）：按平键族型别 A/B/C 画键形 + 键中心线；
/// 端置再加模板的 40° 导入斜线（层 0；x 跨度 = t₁）。
/// 中心线伸出 = `3.0 × frame_scale`（与轴线的 `3n` 同一口径；模板无图框 n=1 → 3.0）。
fn emit_key_overlay(out: &mut Vec<EntityType>, kg: &KeywayGeom, frame_scale: f64) {
    if let Some(gh) = &kg.guided {
        emit_guided_overlay(out, kg, gh, frame_scale);
        return;
    }
    let over = 3.0 * frame_scale;
    let (rk, l) = (kg.rk, kg.l);
    let tan20 = 20.0_f64.to_radians().tan();
    let arc = |out: &mut Vec<EntityType>, c: [f64; 2], a0: f64, a1: f64| {
        out.push(crate::partgen_kit::arc(c, rk, a0, a1, LAYER_MAIN));
    };
    match kg.place {
        KeywayPlace::Mid => {
            // 用户口径（2026-09-23）：**常规侧视图中置恒显示 A 型跑道形** —— 无论所选
            // A/B/C，长度用折算后的 `l`（B 已 +b、C 已 +b/2），槽端都是圆弧、不再出直棱。
            let xc = (kg.slot0 + kg.slot1) / 2.0;
            let (ca, cb) = (xc - l / 2.0 + rk, xc + l / 2.0 - rk);
            out.push(line([ca, rk], [cb, rk], LAYER_MAIN));
            out.push(line([ca, -rk], [cb, -rk], LAYER_MAIN));
            arc(out, [ca, 0.0], 90.0, 270.0);
            arc(out, [cb, 0.0], 270.0, 450.0);
            for x in [ca, cb] {
                out.push(line([x, -(rk + over)], [x, rk + over], LAYER_CENTER));
            }
            out.push(line([kg.slot0 - over, 0.0], [kg.slot1 + over, 0.0], LAYER_CENTER));
        }
        KeywayPlace::End => {
            let side = kg.side.expect("端置轴槽必有 side");
            let (sgn, face) = match side {
                End::L => (1.0, kg.face),
                End::R => (-1.0, kg.face),
            };
            // 键体：[face + t₁, face + t₁ + 折算长度]；内端正好到槽闭端。
            let x_flat = face + sgn * kg.t1;
            let x_inner = face + sgn * (kg.t1 + l);
            if kg.kind == KeyKind::A {
                // A 型端置：双圆头（外端朝开口）。
                let co = x_flat + sgn * rk;
                let ci = x_inner - sgn * rk;
                out.push(line([co, rk], [ci, rk], LAYER_MAIN));
                out.push(line([co, -rk], [ci, -rk], LAYER_MAIN));
                let (a0, a1) = if sgn > 0.0 { (90.0, 270.0) } else { (270.0, 450.0) };
                arc(out, [co, 0.0], a0, a1);
                let (b0, b1) = if sgn > 0.0 { (270.0, 450.0) } else { (90.0, 270.0) };
                arc(out, [ci, 0.0], b0, b1);
                for x in [co, ci] {
                    out.push(line([x, -(rk + over)], [x, rk + over], LAYER_CENTER));
                }
            } else {
                // B/C 端置（俯视/开口槽口径）：**显示 C** —— 平端朝开口（不画端线）+ 内端圆头；
                // B 的折算长度已 +b/2，直段正好容纳 L_B。
                let ci = x_inner - sgn * rk;
                out.push(line([x_flat, rk], [ci, rk], LAYER_MAIN));
                out.push(line([x_flat, -rk], [ci, -rk], LAYER_MAIN));
                let (a0, a1) = if sgn > 0.0 { (270.0, 450.0) } else { (90.0, 270.0) };
                arc(out, [ci, 0.0], a0, a1);
                out.push(line([ci, -(rk + over)], [ci, rk + over], LAYER_CENTER));
                // 40° 导入斜线（层 0；x 跨度 = t₁，y 偏移 = (b/2)·tan20°）。
                let dy = rk * tan20;
                out.push(line([x_flat, rk], [face, rk + dy], "0"));
                out.push(line([x_flat, -rk], [face, -rk - dy], "0"));
            }
            let (hx0, hx1) = if sgn > 0.0 {
                (face - over, x_inner + over)
            } else {
                (x_inner - over, face + over)
            };
            out.push(line([hx0, 0.0], [hx1, 0.0], LAYER_CENTER));
        }
    }
}

/// **贯通轴线的段边界/特征界线竖线**（从 `-half` 到 `+half`，落 `1轮廓实线层`）。
///
/// 用户 2026-09-18 更正版口径（基准 `轴测试更正.dxf`）：
/// * 每条段边界都画（直径不变的分界也画），半高 = `min(左段端半径, 右段端半径)`；
/// * 倒角终点（倒角根）画，半高 = 倒角根半径；
/// * 槽（OV/RL）的槽肩面 / 槽底终止 / 斜壁终点各画一条，槽肩半高 = 圆角切点
///   半径（`d/2 + r − h` 或 `dg/2 + r`），槽底半高 = 槽底半径，斜壁终点
///   半高 = 段（大径）半径。
/// `half ≤ 0`（或非有限）不落图。
fn through_line(entities: &mut Vec<EntityType>, x: f64, half: f64) {
    if half.is_finite() && half > 1e-9 {
        entities.push(line([x, -half], [x, half], LAYER_MAIN));
    }
}

/// 某段某端**端面处实际轮廓半径**：段级退刀槽圆角切点 / 越程槽圆角切点 / 齿顶圆 /
/// 普通半径。只用于齿轮相邻直径检查；落图的端面几何在 `build` 里另算。
fn face_radius(seg: &Segment, end: End) -> f64 {
    if let Some(spec) = seg.relief.iter().find(|r| r.end == end) {
        if let Ok(dims) = resolve_relief_dims(seg, spec, "段") {
            return dims.dg / 2.0 + dims.r;
        }
    }
    if let Some(ov) = seg.ov.iter().find(|o| o.end == end) {
        let d = match end {
            End::L => seg.s,
            End::R => seg.e,
        };
        if let Ok((_, row)) = detail::groove_entities(d, ov.b1) {
            return detail::fillet_tangent_radius(d, row);
        }
    }
    seg.outer_radius(end)
}

/// 沿 x 平移（Hatch 的边界段同步平移，图案 offset 是世界坐标下的周期量，不用改）。
fn translate_x(entity: EntityType, dx: f64) -> EntityType {
    match entity {
        EntityType::Line(mut l) => {
            l.start.x += dx;
            l.end.x += dx;
            EntityType::Line(l)
        }
        EntityType::Arc(mut a) => {
            a.center.x += dx;
            EntityType::Arc(a)
        }
        EntityType::Hatch(mut h) => {
            for path in &mut h.paths {
                for edge in &mut path.edges {
                    match edge {
                        BoundaryEdge::Line(e) => {
                            e.start.x += dx;
                            e.end.x += dx;
                        }
                        BoundaryEdge::CircularArc(e) => e.center.x += dx,
                        BoundaryEdge::EllipticArc(e) => {
                            e.center.x += dx;
                            e.major_axis_endpoint.x += dx;
                        }
                        BoundaryEdge::Polyline(e) => {
                            for v in &mut e.vertices {
                                v.x += dx;
                            }
                        }
                        BoundaryEdge::Spline(e) => {
                            for q in &mut e.control_points {
                                q.x += dx;
                            }
                            for q in &mut e.fit_points {
                                q.x += dx;
                            }
                        }
                    }
                }
            }
            EntityType::Hatch(h)
        }
        other => other,
    }
}

/// 局部坐标（端面在 x=0）→ 镜像到端面 `face_x` 左侧：`x' = face_x − x`。
/// 圆弧镜像后保持 DXF 的逆时针读法（角度按 `π − θ` 处理）。
fn mirror_x(entity: EntityType, face_x: f64) -> EntityType {
    match entity {
        EntityType::Line(mut l) => {
            l.start.x = face_x - l.start.x;
            l.end.x = face_x - l.end.x;
            EntityType::Line(l)
        }
        EntityType::Arc(mut a) => {
            a.center.x = face_x - a.center.x;
            let (start, end) = (a.start_angle, a.end_angle);
            a.start_angle = std::f64::consts::PI - end;
            a.end_angle = std::f64::consts::PI - start;
            EntityType::Arc(a)
        }
        other => other,
    }
}

/// 一次构建出来的原始几何：两视图共用的真实几何 + 剖视用的剖面线环。
struct Geometry {
    /// 两个视图共用的**真实几何**（轮廓/端面/倒角/槽/螺纹线/齿轮倒角）；
    /// 不含贯通竖线（那类只属于常规视图）。
    entities: Vec<EntityType>,
    /// 仅常规视图：段边界 / 倒角终点 / 槽肩、槽底、斜壁终点等**贯通竖线**
    /// （`through_line`；用户 2026-09-18 定案只加在常规视图）。
    through: Vec<EntityType>,
    /// 仅剖视视图的真实几何：齿轮段齿根线（齿部按不剖，也是剖面线边界）。
    section_lines: Vec<EntityType>,
    /// 仅常规视图：花键段的小径细线 / 收尾弧 / 两条细竖线（`2细线层`）。
    spline_regular: Vec<EntityType>,
    /// 仅常规视图：轴槽的键叠画 + 键中心线 + 被槽“挡住”的端面/倒角线
    /// （剖视里这些线被缺口/sagitta 线替换，见 `review/键槽_设计.md`）。
    keyway_normal: Vec<EntityType>,
    /// 轴槽缺口专用的剖面线边界边（构建下环时替换为平顶线）。
    keyway_notch: Vec<HatchEdge>,
    /// 轴槽“无缺口”平顶线跨度：`(xs, xe, R)`。
    keyway_plain: Vec<(f64, f64, f64)>,
    /// 只进下环（对称半边）的边界边：端置槽开在道端时被切掉的上倒角，
    /// 下环（轴下半边）仍保留同样的倒角（模板 C1 下环带倒角边）。
    keyway_plain_extra: Vec<HatchEdge>,
    /// 剖视剖面线边界：上半外轮廓边界（左→右），拆成上/下两个环。
    profile: Vec<HatchEdge>,
    total_length: f64,
    max_diameter: f64,
    segment_count: usize,
}

/// 解析 → 校验 → 生成（`frame_scale` 用于轴线 `6n` 伸出量；无图框传 1.0）。
/// 返回的图元已按 `program.view` 组合（常规 / 剖视；双视图已移除）。
pub fn build(program: &Program, frame_scale: f64) -> Result<Shaft, String> {
    let Geometry {
        entities,
        through,
        section_lines,
        spline_regular,
        keyway_normal,
        keyway_notch,
        keyway_plain,
        keyway_plain_extra,
        profile,
        total_length: total,
        max_diameter,
        segment_count,
    } = build_geometry(program, frame_scale)?;
    let rings = close_hatch_rings(
        profile,
        total,
        &keyway_notch,
        &keyway_plain,
        &keyway_plain_extra,
    );
    let hatch = || {
        if rings.is_empty() {
            Vec::new()
        } else {
            // ANSI31 / 比例 1.0，轴线上/下两个环（走 partgen_kit 的既有 HATCH 通路）。
            vec![crate::partgen_kit::hatch_ansi31_rings(&rings, 0.0, 1.0)]
        }
    };
    let entities = match program.view {
        ShaftView::Normal => {
            let mut out = entities;
            out.extend(through);
            out.extend(spline_regular);
            out.extend(keyway_normal);
            out
        }
        ShaftView::Section => {
            let mut out = entities;
            out.extend(section_lines);
            out.extend(hatch());
            out
        }
    };
    Ok(Shaft {
        entities,
        total_length: total,
        max_diameter,
        segment_count,
    })
}

/// 一段局部螺纹（`M… TL…` 或 `M… RL`）的轴向布局：以段右端面（台肩）为基准。
///
/// 自右向左：`RL` = 台肩面 → R 圆角 → 槽底 `g1` → 斜壁（`g2` 处接大径）；
/// `TL` = 肩距 `a`（锥面 `a−x` + 螺尾 `x`）→ 分界竖线 → 完整螺纹 `TL`。
#[derive(Debug, Clone, Copy)]
struct LocalThread {
    /// 段右端面 x（台肩面）。
    face: f64,
    /// 螺纹大径半径（= 段半径）。
    r: f64,
    /// 螺纹小径半径（细实线所在半径）。
    r1: f64,
    /// 完整螺纹左端 x（`TL`；`RL` 不给 `TL` 时 = 段左端）。
    full_start: f64,
    /// 螺尾起点 x（锥面与螺尾交点）；`RL` 时不用。
    runout_start: f64,
    /// 完整螺纹 / 螺尾分界竖线 x；`RL` 时不用。
    boundary: f64,
    /// `RL`：表 2 退刀槽实际尺寸（`None` = 螺尾画法）。**表归一**后来自 `detail.rs`。
    relief: Option<detail::ReliefDims>,
    /// `RL`：台肩面圆角切点半径 = `dg/2 + r`。
    relief_tangent: f64,
    /// 大径轮廓线右端 x（`RL` = `face − g2`；否则 = `face`）。
    contour_end: f64,
    /// 小径细实线右端 x（螺尾起点，或斜壁与小径的交点）。
    minor_end: f64,
}

/// 该段是否为「普通全螺纹段」：`M` 段且没给 `TL`/`RL`（= 小径细实线贯穿整段的旧口径）。
/// 剖面线边界对这类段按**小径包络**（牙顶区不剖，参考件 `轴剖视图.dxf` 右视图口径）。
fn plain_thread(seg: &Segment) -> Option<&Thread> {
    seg.thread
        .as_ref()
        .filter(|t| t.tl.is_none() && !t.relief)
}

/// 是否矩形花键段（`SPLINE`）：几何上一律禁止 CH，剖面线按“齿部不剖、小径包络”处理。
fn is_spline_seg(seg: &Segment) -> bool {
    seg.spline.is_some()
}

/// 轴上齿轮段的端面自动倒角 C = round(0.6m)（与 `gear.rs::GearParams::chamfer()` 同口径）。
fn auto_face_chamfer(seg: &Segment, _end: End) -> Option<f64> {
    let c = if let Some(gear) = &seg.gear {
        gear.chamfer()
    } else {
        return None;
    };
    (c > 1e-9).then_some(c)
}

/// 端面自动倒角的**单侧规则**（用户 2026-09-21 定案）：自由端（无邻段）或
/// 邻段外轮廓更小（台阶向下）→ 加；邻段更大（肩部）/ 相等（端面齐平、无外角）→ 不加。
fn side_auto_chamfer(c: Option<f64>, own_r: f64, neighbor_r: Option<f64>) -> Option<f64> {
    let c = c?;
    match neighbor_r {
        None => Some(c),
        Some(r) if r < own_r - 1e-12 => Some(c),
        Some(_) => None,
    }
}

/// 解析 → 校验 → 生成常规视图轮廓，并顺手记录剖面线环用的上半外轮廓段。
fn build_geometry(program: &Program, frame_scale: f64) -> Result<Geometry, String> {
    validate(program)?;
    let segs = &program.segments;
    let count = segs.len();

    // 每段起点 x（第 1 段左端面 = 0）。
    let mut x0s = Vec::with_capacity(count);
    let mut cursor = 0.0;
    for seg in segs {
        x0s.push(cursor);
        cursor += seg.l;
    }
    let total = program.total_length();
    let max_diameter = segs
        .iter()
        .map(|s| s.outer_radius(End::L).max(s.outer_radius(End::R)) * 2.0)
        .fold(0.0_f64, f64::max);

    // ── 局部螺纹（`TL` 收尾 / `RL` 退刀槽）布局：贴段右端台肩布置；
    //    不写 `TL` 也不写 `RL` 的 `M` 仍是旧口径（整段全螺纹）。 ──
    let mut local_threads: Vec<Option<LocalThread>> = vec![None; count];
    for (index, seg) in segs.iter().enumerate() {
        let Some(thread) = &seg.thread else { continue };
        if thread.tl.is_none() && !thread.relief {
            continue;
        }
        let face = x0s[index] + seg.l;
        let r = seg.s / 2.0;
        let r1 = thread.minor_radius(seg.s);
        let mut lay = LocalThread {
            face,
            r,
            r1,
            full_start: x0s[index],
            runout_start: face,
            boundary: face,
            relief: None,
            relief_tangent: 0.0,
            contour_end: face,
            minor_end: face,
        };
        if thread.relief {
            let pitch = thread.pitch.ok_or_else(|| {
                crate::i18n::t_fmt("cmd.shaft.err.tlro_rl_need_pitch", &[("label", &seg_label(index + 1))])
            })?;
            let row = detail::thread_relief_row(pitch)
                .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", &seg_label(index + 1)), ("e", &e.to_string())]))?;
            let dims = row.dims(seg.s);
            let rg = dims.dg / 2.0;
            lay.relief = Some(dims);
            lay.relief_tangent = rg + dims.r;
            lay.contour_end = face - dims.g2;
            // 小径细实线与斜壁的交点（斜壁在 `g2−g1` 距离内从 `dg/2` 升到 `d/2`）。
            lay.minor_end = face - dims.g1 - (dims.g2 - dims.g1) * (r1 - rg) / (r - rg);
            lay.full_start = match thread.tl {
                Some(tl) => face - dims.g2 - tl,
                None => x0s[index],
            };
        } else {
            let pitch = thread.pitch.ok_or_else(|| {
                crate::i18n::t_fmt("cmd.shaft.err.tlro_tl_need_pitch", &[("label", &seg_label(index + 1))])
            })?;
            let row = detail::runout_row_checked(pitch)
                .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.prefix", &[("label", &seg_label(index + 1)), ("e", &e.to_string())]))?;
            let a = row.a(thread.shoulder);
            let x = row.x(thread.runout);
            lay.runout_start = face - (a - x);
            lay.boundary = face - a;
            lay.minor_end = lay.runout_start;
            lay.full_start = lay.boundary - thread.tl.unwrap_or(0.0);
        }
        local_threads[index] = Some(lay);
    }

    // 段级 RL 退刀槽：先把每段的 [左, 右] 尺寸解出来（validate 已查过结构/取参，
    // 这里再算一遍供画法与轮廓平移用；邻段台阶/装不下在下面的端面循环里查）。
    let mut relief_dims: Vec<[Option<detail::ReliefDims>; 2]> = vec![[None, None]; count];
    for (index, seg) in segs.iter().enumerate() {
        for spec in &seg.relief {
            let label = seg_label(index + 1);
            let dims = resolve_relief_dims(seg, spec, &label)?;
            relief_dims[index][match spec.end {
                End::L => 0,
                End::R => 1,
            }] = Some(dims);
        }
    }

    // 倒角 / 越程槽 / 段级退刀槽落在**本段自己**身上的量（决定本段轮廓线被吃掉多少）。
    let mut own_ch = vec![[None::<f64>; 2]; count]; // [左, 右]
    let mut own_ov = vec![[None::<f64>; 2]; count];
    let mut own_relief = vec![[None::<f64>; 2]; count];
    let mut entities: Vec<EntityType> = Vec::new();
    // 贯通竖线（`through_line` 系列）：只属于常规视图。
    let mut through: Vec<EntityType> = Vec::new();
    // 仅剖视视图的真实几何（齿轮齿根线）。
    let mut section_lines: Vec<EntityType> = Vec::new();
    // 仅常规视图的花键细线（小径线 / 收尾弧 / 两条细竖线，2细线层）。
    let mut spline_regular: Vec<EntityType> = Vec::new();
    // 仅常规视图的轴槽键叠画 / 键中心线（剖视被缺口替换）。
    let mut keyway_normal: Vec<EntityType> = Vec::new();
    // 轴槽缺口专用剖面线边界边（下环不复制缺口）+ 平顶线跨度（xs, xe, R）。
    let mut keyway_notch: Vec<HatchEdge> = Vec::new();
    let mut keyway_plain: Vec<(f64, f64, f64)> = Vec::new();
    // 端置槽道端被切掉的上倒角（下环仍要）—— 只进 plain 链。
    let mut keyway_plain_extra: Vec<HatchEdge> = Vec::new();
    // 剖面线环用的上半外轮廓段（含倒角/台阶/越程槽圆角/螺纹小径包络/齿轮齿根），
    // 最后按 x 排序拆成上/下两个环。
    let mut profile: Vec<HatchEdge> = Vec::new();

    // ── 齿轮段相邻直径检查（用户 2026-09-18 口径）：相邻半径 ≤ ra 放行；
    //    > ra 会盖住齿顶线 → 报「第 N 段」。不再受齿根圆 rf 限制。 ──
    for (index, seg) in segs.iter().enumerate() {
        let Some(gear) = &seg.gear else { continue };
        let ra = gear.addendum_radius();
        let neighbors = [
            (index.checked_sub(1), End::R),
            (
                if index + 1 < count {
                    Some(index + 1)
                } else {
                    None
                },
                End::L,
            ),
        ];
        for (j, end) in neighbors {
            let Some(j) = j else { continue };
            let r_n = face_radius(&segs[j], end);
            if r_n > ra + 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.gear_adjacent_covers_tip", &[("label", &seg_label(index + 1)), ("p0", &(j + 1).to_string()), ("p1", &(trim(r_n * 2.0)).to_string()), ("p2", &(trim(ra * 2.0)).to_string())]));
            }
        }
    }

    // ── 内部端面（第 i 段右端 ↔ 第 i+1 段左端） ──
    for j in 0..count.saturating_sub(1) {
        let i = j;
        let k = j + 1;
        let x_face = x0s[k];
        let ch_r = segs[i].ch.iter().find(|c| c.end == End::R).map(|c| c.c);
        let ch_l = segs[k].ch.iter().find(|c| c.end == End::L).map(|c| c.c);
        let (chamfer, ch_from_left) = match (ch_r, ch_l) {
            (Some(_), Some(_)) => {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.chamfer_same_face", &[("label", &seg_label(i + 1)), ("p0", &(k + 1).to_string())]))
            }
            (Some(c), None) => (Some(c), true),
            (None, Some(c)) => (Some(c), false),
            (None, None) => (None, false),
        };
        let ov_r = segs[i].ov.iter().find(|o| o.end == End::R).copied();
        let ov_l = segs[k].ov.iter().find(|o| o.end == End::L).copied();
        if ov_r.is_some() && ov_l.is_some() {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.ov_same_face", &[("label", &seg_label(i + 1)), ("p0", &(k + 1).to_string())]));
        }
        // 段级 RL（i 右端 / k 左端）与本端面已有的 M 段螺纹收尾：
        // 同一端面只能有一个退刀槽；与 CH/OV 互斥（同端只能有一个槽/倒角）。
        let relief_r = relief_dims[i][1];
        let relief_l = relief_dims[k][0];
        let thread_relief_here = local_threads[i].filter(|lay| lay.relief.is_some());
        let relief_kinds = relief_r.is_some() as u8
            + relief_l.is_some() as u8
            + thread_relief_here.is_some() as u8;
        if relief_kinds > 1 {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.one_relief_per_face", &[("label", &seg_label(i + 1))]));
        }
        if relief_kinds > 0 {
            if chamfer.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_no_chamfer", &[("label", &seg_label(i + 1))]));
            }
            if ov_r.is_some() || ov_l.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_ov_same_face", &[("label", &seg_label(i + 1))]));
            }
        }
        // 该端面处两侧的轮廓半径。
        let ra = segs[i].outer_radius(End::R);
        let rb = segs[k].outer_radius(End::L);
        // 齿轮/渐开线花键端面自动倒角 C = round(0.6m)：按单侧规则决定该端倒不倒
        //（自由端/邻段更小 → 倒；邻段更大/齐平 → 不倒；用户 CH 已占该侧时让位）。
        let user_land = chamfer.map(|_| if rb > ra { k } else { i });
        let auto_c_r = if user_land == Some(i) {
            None
        } else {
            side_auto_chamfer(auto_face_chamfer(&segs[i], End::R), ra, Some(rb))
        };
        let auto_c_l = if user_land == Some(k) {
            None
        } else {
            side_auto_chamfer(auto_face_chamfer(&segs[k], End::L), rb, Some(ra))
        };

        let mut bottom = (ra - auto_c_r.unwrap_or(0.0)).min(rb - auto_c_l.unwrap_or(0.0));
        let mut top = (ra - auto_c_r.unwrap_or(0.0)).max(rb - auto_c_l.unwrap_or(0.0));
        let mut chamfer_lines: Option<([f64; 2], [f64; 2])> = None;
        // 带 CH 倒角的落点段（用于判断剖面线边界是否由螺纹小径包络接管）。
        let mut chamfer_land: Option<usize> = None;
        // 端面带槽时的贯通竖线半高（槽肩圆角切点）；None = 用 min(left_eff, right_eff)。
        let mut face_through_h: Option<f64> = None;

        // ── 端面自动倒角（斜线 + 台阶竖线；竖线只属常规视图）──
        if let Some(c) = auto_c_r {
            let contour = [x_face - c, ra];
            let face_point = [x_face, ra - c];
            entities.push(line(face_point, contour, LAYER_MAIN));
            entities.push(line(
                [face_point[0], -face_point[1]],
                [contour[0], -contour[1]],
                LAYER_MAIN,
            ));
            through_line(&mut through, contour[0], contour[1]);
            own_ch[i][1] = Some(c);
        }
        if let Some(c) = auto_c_l {
            let contour = [x_face + c, rb];
            let face_point = [x_face, rb - c];
            entities.push(line(face_point, contour, LAYER_MAIN));
            entities.push(line(
                [face_point[0], -face_point[1]],
                [contour[0], -contour[1]],
                LAYER_MAIN,
            ));
            through_line(&mut through, contour[0], contour[1]);
            own_ch[k][0] = Some(c);
        }

        if let Some(c) = chamfer {
            let requester = if ch_from_left { i } else { k };
            let requester_end = crate::i18n::t(if ch_from_left { "cmd.shaft.end.r" } else { "cmd.shaft.end.l" });
            if (ra - rb).abs() < 1e-12 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.no_face_for_chamfer", &[("label", &seg_label(requester + 1)), ("p0", &(requester_end).to_string())]));
            }
            let delta = (ra - rb).abs();
            // 倒角贴**凸角**（大的一侧）；落在齿轮段那侧则冲突。
            let (land, _land_end) = if rb > ra { (k, End::L) } else { (i, End::R) };
            // 花键凸角允许「C = 全台阶」（模板 φ22→φ26 的 C2 就是如此，倒角正好吃掉台阶）。
            let full_step_on_spline = is_spline_seg(&segs[land]) && (c - delta).abs() < 1e-9;
            if c >= delta - 1e-12 && !full_step_on_spline {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.chamfer_eats_face", &[("label", &seg_label(requester + 1)), ("p0", &(requester_end).to_string()), ("p1", &(trim(c)).to_string()), ("p2", &(trim(ra * 2.0)).to_string()), ("p3", &(trim(rb * 2.0)).to_string()), ("p4", &(trim(delta)).to_string())]));
            }
            chamfer_land = Some(land);
            if segs[land].gear.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.chamfer_on_gear", &[("label", &seg_label(requester + 1)), ("p0", &(requester_end).to_string()), ("p1", &(land + 1).to_string())]));
            }
            if is_spline_seg(&segs[land]) && land == i {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.chamfer_on_spline", &[("label", &seg_label(requester + 1))]));
            }
            let (contour, face_point) = if rb > ra {
                chamfer_geom(&segs[k], x0s[k], End::L, c)
            } else {
                chamfer_geom(&segs[i], x0s[i], End::R, c)
            };
            if face_point[1] <= bottom + 1e-9 && !full_step_on_spline {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.chamfer_face_overlap", &[("label", &seg_label(requester + 1)), ("p0", &(requester_end).to_string()), ("p1", &(trim(face_point[1])).to_string()), ("p2", &(trim(bottom)).to_string())]));
            }
            top = face_point[1];
            chamfer_lines = Some((contour, face_point));
            if rb > ra {
                own_ch[k][0] = Some(c);
            } else {
                own_ch[i][1] = Some(c);
            }
        }

        if let Some(ov) = ov_r {
            // 槽开在第 i 段（磨出的外圆），台阶 = 第 i+1 段。
            if !(rb > ra + 1e-12) {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.ov_no_step_r", &[("label", &seg_label(i + 1)), ("p0", &(trim(rb * 2.0)).to_string()), ("p1", &(trim(ra * 2.0)).to_string())]));
            }
            if (segs[i].s - segs[i].e).abs() > 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.ov_cone_r", &[("label", &seg_label(i + 1))]));
            }
            let (groove, row) = detail::groove_entities(segs[i].e, ov.b1)
                .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.ov_prefix_r", &[("label", &seg_label(i + 1)), ("e", &e.to_string())]))?;
            if row.b1 > segs[i].l + 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.ov_b1_too_long", &[("label", &seg_label(i + 1)), ("p0", &(trim(row.b1)).to_string()), ("p1", &(trim(segs[i].l)).to_string())]));
            }
            let fillet = detail::fillet_tangent_radius(segs[i].e, row);
            if fillet >= top - 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.ov_r_tangent_r", &[("label", &seg_label(i + 1)), ("p0", &(trim(fillet)).to_string()), ("p1", &(trim(top)).to_string())]));
            }
            bottom = fillet;
            face_through_h = Some(fillet);
            own_ov[i][1] = Some(row.b1);
            // `detail.rs` 的磨外圆槽体已按用户 2026-09-18 定案不含砂轮细线
            // （`2细线层` 青线属标注；独立要素同口径），这里直接镜像。
            entities.extend(groove.into_iter().map(|e| mirror_x(e, x_face)));
            // 剖面线环：斜坡 → 槽底 → 圆角（上半侧，左→右）
            let rg = segs[i].e / 2.0;
            let yb = rg - row.h;
            // 槽底终止 / 斜壁终点两条贯通竖线（槽肩那条 = 上面的段边界线）。
            through_line(&mut through, x_face - (row.b1 - row.h), yb);
            through_line(&mut through, x_face - row.b1, rg);
            let slope_x1 = x_face - (row.b1 - row.h);
            let groove_x1 = x_face - row.r;
            profile.push(lr_line([x_face - row.b1, rg], [slope_x1, yb]));
            profile.push(lr_line([slope_x1, yb], [groove_x1, yb]));
            profile.push(HatchEdge::Arc {
                c: [groove_x1, fillet],
                r: row.r,
                start_deg: -90.0,
                end_deg: 0.0,
                ccw: true,
            });
        } else if let Some(ov) = ov_l {
            // 槽开在第 k 段（磨出的外圆），台阶 = 第 i 段。
            if !(ra > rb + 1e-12) {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.ov_no_step_l", &[("label", &seg_label(k + 1)), ("p0", &(trim(ra * 2.0)).to_string()), ("p1", &(trim(rb * 2.0)).to_string())]));
            }
            if (segs[k].s - segs[k].e).abs() > 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.ov_cone_l", &[("label", &seg_label(k + 1))]));
            }
            let (groove, row) = detail::groove_entities(segs[k].s, ov.b1)
                .map_err(|e| crate::i18n::t_fmt("cmd.shaft.err.ov_prefix_l", &[("label", &seg_label(k + 1)), ("e", &e.to_string())]))?;
            if row.b1 > segs[k].l + 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.ov_b1_too_long", &[("label", &seg_label(k + 1)), ("p0", &(trim(row.b1)).to_string()), ("p1", &(trim(segs[k].l)).to_string())]));
            }
            let fillet = detail::fillet_tangent_radius(segs[k].s, row);
            if fillet >= top - 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.ov_r_tangent_l", &[("label", &seg_label(k + 1)), ("p0", &(trim(fillet)).to_string()), ("p1", &(trim(top)).to_string())]));
            }
            bottom = fillet;
            face_through_h = Some(fillet);
            own_ov[k][0] = Some(row.b1);
            // `detail.rs` 的磨外圆槽体已不含砂轮细线（同 ov_r 口径），直接平移。
            entities.extend(groove.into_iter().map(|e| translate_x(e, x_face)));
            // 槽底终止 / 斜壁终点两条贯通竖线（槽肩那条 = 上面的段边界线）。
            let rg = segs[k].s / 2.0;
            let yb = rg - row.h;
            through_line(&mut through, x_face + (row.b1 - row.h), yb);
            through_line(&mut through, x_face + row.b1, rg);
            // 剖面线环：圆角 → 槽底 → 斜坡（上半侧，左→右）
            let slope_x1 = x_face + (row.b1 - row.h);
            profile.push(HatchEdge::Arc {
                c: [x_face + row.r, fillet],
                r: row.r,
                start_deg: 180.0,
                end_deg: 270.0,
                ccw: true,
            });
            profile.push(lr_line([x_face + row.r, yb], [slope_x1, yb]));
            profile.push(lr_line([slope_x1, yb], [x_face + row.b1, rg]));
        }

        // ── 段级 RL 退刀槽（GB/T 3 表 2 剖面：台肩面 → R 圆角 → 槽底 g1 → 斜壁 g2）──
        //    槽体画在带槽的那一段上（i 右端 或 k 左端）；邻段必须是更高的台阶。
        let mut segment_relief_left: Option<f64> = None;
        let mut segment_relief_right: Option<f64> = None;
        if let Some(dims) = relief_r {
            if !(rb > ra + 1e-12) {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_no_step_r", &[("label", &seg_label(i + 1)), ("p0", &(trim(rb * 2.0)).to_string()), ("p1", &(trim(ra * 2.0)).to_string())]));
            }
            let tangent = dims.dg / 2.0 + dims.r;
            if tangent >= top - 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_r_tangent_r", &[("label", &seg_label(i + 1)), ("p0", &(trim(tangent)).to_string()), ("p1", &(trim(top)).to_string())]));
            }
            bottom = tangent;
            face_through_h = Some(tangent);
            own_relief[i][1] = Some(dims.g2);
            entities.extend(
                detail::relief_groove_entities(&dims)
                    .into_iter()
                    .map(|e| mirror_x(e, x_face)),
            );
            // 槽底终止 / 斜壁终点两条贯通竖线（槽肩那条 = 上面的段边界线）。
            let rg = dims.dg / 2.0;
            through_line(&mut through, x_face - dims.g1, rg);
            through_line(&mut through, x_face - dims.g2, ra);
            // 剖面线环：斜壁 → 槽底 → 圆角（上半侧，左→右）
            profile.push(lr_line([x_face - dims.g2, ra], [x_face - dims.g1, rg]));
            profile.push(lr_line([x_face - dims.g1, rg], [x_face - dims.r, rg]));
            profile.push(HatchEdge::Arc {
                c: [x_face - dims.r, tangent],
                r: dims.r,
                start_deg: 270.0,
                end_deg: 360.0,
                ccw: true,
            });
            segment_relief_left = Some(tangent);
        } else if let Some(dims) = relief_l {
            if !(ra > rb + 1e-12) {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_no_step_l", &[("label", &seg_label(k + 1)), ("p0", &(trim(ra * 2.0)).to_string()), ("p1", &(trim(rb * 2.0)).to_string())]));
            }
            let tangent = dims.dg / 2.0 + dims.r;
            if tangent >= top - 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.rl_r_tangent_l", &[("label", &seg_label(k + 1)), ("p0", &(trim(tangent)).to_string()), ("p1", &(trim(top)).to_string())]));
            }
            bottom = tangent;
            face_through_h = Some(tangent);
            own_relief[k][0] = Some(dims.g2);
            entities.extend(
                detail::relief_groove_entities(&dims)
                    .into_iter()
                    .map(|e| translate_x(e, x_face)),
            );
            // 槽底终止 / 斜壁终点两条贯通竖线（槽肩那条 = 上面的段边界线）。
            let rg = dims.dg / 2.0;
            through_line(&mut through, x_face + dims.g1, rg);
            through_line(&mut through, x_face + dims.g2, rb);
            // 剖面线环：圆角 → 槽底 → 斜壁（上半侧，左→右）
            profile.push(HatchEdge::Arc {
                c: [x_face + dims.r, tangent],
                r: dims.r,
                start_deg: 180.0,
                end_deg: 270.0,
                ccw: true,
            });
            profile.push(lr_line([x_face + dims.r, rg], [x_face + dims.g1, rg]));
            profile.push(lr_line([x_face + dims.g1, rg], [x_face + dims.g2, rb]));
            segment_relief_right = Some(tangent);
        }

        // ── 局部螺纹（TL/RL）右端接台肩：`RL` 时台肩根只画到圆角切点，
        //    端面线 = 「相邻段半径 ↔ 圆角切点」；锥面/螺尾/槽体在段循环里画。 ──
        let relief_face = local_threads[i]
            .filter(|lay| lay.relief.is_some())
            .map(|lay| lay.relief_tangent);
        if let Some(tangent) = relief_face {
            if chamfer_lines.is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.tlro_no_chamfer", &[("label", &seg_label(i + 1))]));
            }
            let right_surface = if ov_l.is_some() { bottom } else { rb };
            bottom = tangent.min(right_surface);
            top = tangent.max(right_surface);
            face_through_h = Some(tangent);
        }

        // 上半轮廓在端面两侧的实际半径（决定剖面线环竖直段的走向）：
        // 倒角/越程槽会把这侧表面从原始半径上切掉一块。
        let mut left_eff = ra;
        let mut right_eff = rb;
        if let Some((_, face_point)) = chamfer_lines {
            if rb > ra {
                right_eff = face_point[1];
            } else {
                left_eff = face_point[1];
            }
        }
        if ov_r.is_some() {
            left_eff = bottom;
        }
        if ov_l.is_some() {
            right_eff = bottom;
        }
        if let Some(tangent) = segment_relief_left {
            left_eff = tangent;
        }
        if let Some(tangent) = segment_relief_right {
            right_eff = tangent;
        }
        if let Some(tangent) = relief_face {
            left_eff = tangent;
        }
        // 剖面线边界：齿轮端按齿根圆（齿部按不剖）；花键端按小径包络
        // （轴线↔小径两条带，齿部不剖）；普通全螺纹段按小径包络
        // （牙顶/牙底之间的牙型区不剖，参考件 `轴剖视图.dxf` 右视图口径）。
        if let Some(spline) = &segs[i].spline {
            // 花键右端 = 收尾弧终点，已回到大径 → 剖面边界取大径。
            left_eff = spline.major_radius();
        } else if let Some(gear) = &segs[i].gear {
            left_eff = hatch_bound_radius(gear);
        } else if let Some(thread) = plain_thread(&segs[i]) {
            left_eff = left_eff.min(thread.minor_radius(segs[i].s));
        }
        if let Some(spline) = &segs[k].spline {
            // 花键左端小径线起点 → 剖面边界取小径（齿部不剖）；
            // 若左端倒角切到小径以下，剖面线沿倒角走到与小径的交点（保持链单调）。
            let chamfer_below = chamfer_land == Some(k)
                && chamfer_lines
                    .map(|(_, face_point)| face_point[1] < spline.minor_radius() - 1e-12)
                    .unwrap_or(false);
            if !chamfer_below {
                right_eff = spline.minor_radius();
            }
        } else if let Some(gear) = &segs[k].gear {
            right_eff = hatch_bound_radius(gear);
        } else if let Some(thread) = plain_thread(&segs[k]) {
            right_eff = right_eff.min(thread.minor_radius(segs[k].s));
        }

        // ── 段边界贯通竖线（用户 2026-09-18 更正版口径）：半高 = 该端面两侧
        //    实际轮廓半径的较小者；端面带槽时 = 槽肩（圆角切点）高。
        //    花键段右端不画贯通竖线（模板那里是花键自己的两根细竖线）。──
        if !is_spline_seg(&segs[i]) {
            through_line(
                &mut through,
                x_face,
                face_through_h.unwrap_or(left_eff.min(right_eff)),
            );
        }

        if top - bottom > 1e-9 {
            entities.push(line([x_face, bottom], [x_face, top], LAYER_MAIN));
            entities.push(line([x_face, -bottom], [x_face, -top], LAYER_MAIN));
            // 上半轮廓竖直段：左段实际表面 → 右段实际表面（方向就是环的走向）
            profile.push(HatchEdge::Line {
                a: [x_face, left_eff],
                b: [x_face, right_eff],
            });
        }
        if let Some((contour, face_point)) = chamfer_lines {
            entities.push(line(face_point, contour, LAYER_MAIN));
            entities.push(line(
                [face_point[0], -face_point[1]],
                [contour[0], -contour[1]],
                LAYER_MAIN,
            ));
            // 剖面线边界：倒角落在普通全螺纹段时由小径包络接管；
            // 落在花键段时由花键段的小径包络接管（下面花键块里补倒角段）。
            if chamfer_land.map_or(true, |land| {
                plain_thread(&segs[land]).is_none() && !is_spline_seg(&segs[land])
            }) {
                profile.push(lr_line(face_point, contour));
            }
            // 倒角终点（根）贯通竖线，半高 = 倒角根半径（用户版 x=2 ±14）。
            through_line(&mut through, contour[0], contour[1]);
        }
    }

    // ── 左自由端（第 1 段左端面） ──
    if segs[0].ov.iter().any(|o| o.end == End::L) {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.ov_free_end_l", &[("label", &seg_label(1))]).into());
    }
    // 左端是端置轴槽：剖视端面被槽切开（上倒角被切掉），模板 C1/C2。
    let key_end_left = segs[0]
        .keyway
        .as_ref()
        .filter(|k| k.place == KeywayPlace::End);
    if let Some(kw) = key_end_left {
        let kg = keyway_geom(&segs[0], kw, 0.0, 0, count, &seg_label(1))?;
        let double = kw.double;
        let c = segs[0].ch.iter().find(|c| c.end == End::L).map(|c| c.c);
        // face_lo 统一为“端面下端点”的 y（负值），无倒角时为 -R；
        // （旧 bug：无倒角分支用 -R 又被 -face_lo 取反，画成 [0,+R]→[0,floor]，左上角凸出且下半未封闭。）
        let face_lo = if let Some(c) = c {
            let (contour, face_point) = chamfer_geom(&segs[0], 0.0, End::L, c);
            if double {
                // 双槽：下倒角也被下槽切掉 —— 只常规视图画（剖视不画、下环不注册）。
                keyway_normal.push(line(
                    [face_point[0], -face_point[1]],
                    [contour[0], -contour[1]],
                    LAYER_MAIN,
                ));
            } else {
                // 下倒角两视图共用；上倒角只常规视图（剖视里被槽切掉）。
                entities.push(line(
                    [face_point[0], -face_point[1]],
                    [contour[0], -contour[1]],
                    LAYER_MAIN,
                ));
                // 下环（对称半边）的倒角边：上倒角在剖视里被切掉，但下半边没有槽，
                // 倒角边要在模板 C1 的下环里出现。
                keyway_plain_extra.push(lr_line(face_point, contour));
            }
            keyway_normal.push(line(face_point, contour, LAYER_MAIN));
            through_line(&mut through, contour[0], contour[1]);
            -face_point[1]
        } else {
            -segs[0].outer_radius(End::L)
        };
        // 常规视图：完整端面线；剖视：单槽 = 下段到槽底 + 上段槽底→sagitta（模板 C1）；
        // 双槽 = 下槽边 + 中带 + 上槽边（外圆两端都被槽切掉）。
        keyway_normal.push(line([0.0, -face_lo], [0.0, face_lo], LAYER_MAIN));
        if double {
            section_lines.push(line([0.0, -kg.y_sag], [0.0, -kg.floor], LAYER_MAIN));
            section_lines.push(line([0.0, -kg.floor], [0.0, kg.floor], LAYER_MAIN));
            section_lines.push(line([0.0, kg.floor], [0.0, kg.y_sag], LAYER_MAIN));
        } else {
            section_lines.push(line([0.0, face_lo], [0.0, kg.floor], LAYER_MAIN));
            section_lines.push(line([0.0, kg.floor], [0.0, kg.y_sag], LAYER_MAIN));
        }
    } else {
    let left_face = if let Some(c) = side_auto_chamfer(
        auto_face_chamfer(&segs[0], End::L),
        segs[0].outer_radius(End::L),
        None,
    ) {
        // 齿轮/渐开线花键段自由端（轴头）：端面可见高 = ra − C（单侧规则：自由端加）。
        let ra = segs[0].outer_radius(End::L);
        let contour = [c, ra];
        let face_point = [0.0, ra - c];
        entities.push(line(face_point, contour, LAYER_MAIN));
        entities.push(line(
            [face_point[0], -face_point[1]],
            [contour[0], -contour[1]],
            LAYER_MAIN,
        ));
        through_line(&mut through, contour[0], contour[1]);
        own_ch[0][0] = Some(c);
        face_point[1]
    } else if let Some(c) = segs[0].ch.iter().find(|c| c.end == End::L).map(|c| c.c) {
        let radius = segs[0].s / 2.0;
        if c >= radius - 1e-12 {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.chamfer_left_eats_face", &[("label", &seg_label(1)), ("p0", &(trim(c)).to_string()), ("p1", &(trim(segs[0].s)).to_string()), ("p2", &(trim(radius)).to_string())]));
        }
        let (contour, face_point) = chamfer_geom(&segs[0], 0.0, End::L, c);
        if face_point[1] <= 1e-9 {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.chamfer_left_through", &[("label", &seg_label(1)), ("p0", &(trim(c)).to_string()), ("p1", &(trim(face_point[1])).to_string())]));
        }
        own_ch[0][0] = Some(c);
        entities.push(line(face_point, contour, LAYER_MAIN));
        entities.push(line(
            [face_point[0], -face_point[1]],
            [contour[0], -contour[1]],
            LAYER_MAIN,
        ));
        // 剖面线边界：普通全螺纹段由小径包络接管（与内端面同口径）。
        if plain_thread(&segs[0]).is_none() {
            profile.push(lr_line(face_point, contour));
        }
        // 倒角终点（根）贯通竖线，半高 = 倒角根半径（用户版 x=2 ±14）。
        through_line(&mut through, contour[0], contour[1]);
        face_point[1]
    } else {
        segs[0].outer_radius(End::L)
    };
    entities.push(line([0.0, -left_face], [0.0, left_face], LAYER_MAIN));
    }

    // ── 右自由端（最后一段右端面） ──
    let last = count - 1;
    let x_end = x0s[last] + segs[last].l;
    if segs[last].ov.iter().any(|o| o.end == End::R) {
        return Err(crate::i18n::t_fmt("cmd.shaft.err.ov_free_end_r", &[("label", &seg_label(last + 1))]));
    }
    // 右端是端置轴槽：与左端镜像（剖视端面被槽切开）。
    let key_end_right = segs[last]
        .keyway
        .as_ref()
        .filter(|k| k.place == KeywayPlace::End);
    if let Some(kw) = key_end_right {
        let kg = keyway_geom(
            &segs[last],
            kw,
            x0s[last],
            last,
            count,
            &seg_label(last + 1),
        )?;
        let c = segs[last].ch.iter().find(|c| c.end == End::R).map(|c| c.c);
        let double = kw.double;
        let face_hi = if let Some(c) = c {
            let (contour, face_point) = chamfer_geom(&segs[last], x0s[last], End::R, c);
            if double {
                // 双槽：下倒角也被下槽切掉 —— 只常规视图画（剖视不画、下环不注册）。
                keyway_normal.push(line(
                    [face_point[0], -face_point[1]],
                    [contour[0], -contour[1]],
                    LAYER_MAIN,
                ));
            } else {
                entities.push(line(
                    [face_point[0], -face_point[1]],
                    [contour[0], -contour[1]],
                    LAYER_MAIN,
                ));
                keyway_plain_extra.push(lr_line(face_point, contour));
            }
            keyway_normal.push(line(face_point, contour, LAYER_MAIN));
            through_line(&mut through, contour[0], contour[1]);
            face_point[1]
        } else {
            segs[last].outer_radius(End::R)
        };
        keyway_normal.push(line([x_end, -face_hi], [x_end, face_hi], LAYER_MAIN));
        if double {
            section_lines.push(line([x_end, -kg.y_sag], [x_end, -kg.floor], LAYER_MAIN));
            section_lines.push(line([x_end, -kg.floor], [x_end, kg.floor], LAYER_MAIN));
            section_lines.push(line([x_end, kg.floor], [x_end, kg.y_sag], LAYER_MAIN));
        } else {
            section_lines.push(line([x_end, -face_hi], [x_end, kg.floor], LAYER_MAIN));
            section_lines.push(line([x_end, kg.floor], [x_end, kg.y_sag], LAYER_MAIN));
        }
    } else {
    let relief_tail = local_threads[last]
        .filter(|lay| lay.relief.is_some())
        .map(|lay| lay.relief_tangent);
    let right_face = if let Some(tangent) = relief_tail {
        if segs[last].ch.iter().any(|c| c.end == End::R) {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.tlro_no_chamfer_plain", &[("label", &seg_label(last + 1))]));
        }
        tangent
    } else if let Some(c) = side_auto_chamfer(
        auto_face_chamfer(&segs[last], End::R),
        segs[last].outer_radius(End::R),
        None,
    ) {
        // 齿轮/渐开线花键段自由端：端面可见高 = ra − C（单侧规则：自由端加）。
        let ra = segs[last].outer_radius(End::R);
        let contour = [x_end - c, ra];
        let face_point = [x_end, ra - c];
        entities.push(line(face_point, contour, LAYER_MAIN));
        entities.push(line(
            [face_point[0], -face_point[1]],
            [contour[0], -contour[1]],
            LAYER_MAIN,
        ));
        through_line(&mut through, contour[0], contour[1]);
        own_ch[last][1] = Some(c);
        face_point[1]
    } else if let Some(c) = segs[last]
        .ch
        .iter()
        .find(|c| c.end == End::R)
        .map(|c| c.c)
    {
        let radius = segs[last].e / 2.0;
        if c >= radius - 1e-12 {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.chamfer_right_eats_face", &[("label", &seg_label(last + 1)), ("p0", &(trim(c)).to_string()), ("p1", &(trim(segs[last].e)).to_string()), ("p2", &(trim(radius)).to_string())]));
        }
        let (contour, face_point) = chamfer_geom(&segs[last], x0s[last], End::R, c);
        if face_point[1] <= 1e-9 {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.chamfer_right_through", &[("label", &seg_label(last + 1)), ("p0", &(trim(c)).to_string()), ("p1", &(trim(face_point[1])).to_string())]));
        }
        own_ch[last][1] = Some(c);
        entities.push(line(face_point, contour, LAYER_MAIN));
        entities.push(line(
            [face_point[0], -face_point[1]],
            [contour[0], -contour[1]],
            LAYER_MAIN,
        ));
        // 剖面线边界：普通全螺纹段由小径包络接管（与内端面同口径）。
        if plain_thread(&segs[last]).is_none() {
            profile.push(lr_line(contour, face_point));
        }
        // 倒角终点（根）贯通竖线，半高 = 倒角根半径。
        through_line(&mut through, contour[0], contour[1]);
        face_point[1]
    } else {
        segs[last].outer_radius(End::R)
    };
    entities.push(line([x_end, -right_face], [x_end, right_face], LAYER_MAIN));
    }

    // ── 每段上下轮廓线（两端被倒角/越程槽吃掉多少已定）；
    //    齿轮段 = 齿顶线（轮廓，两端带 C 倒角）+ 分度线；螺纹段另加小径细实线 ──
    for (index, seg) in segs.iter().enumerate() {
        if let Some(gear) = &seg.gear {
            let (x0, x1) = (x0s[index], x0s[index] + seg.l);
            let (r, ra, rf) = (
                gear.pitch_radius(),
                gear.major_radius(),
                gear.minor_radius(),
            );
            // 大径面（外齿齿顶面 / 内齿外侧齿根面）两端按**已生效的端面自动倒角**缩进。
            let (ta, tb) = (
                x0 + own_ch[index][0].unwrap_or(0.0),
                x1 - own_ch[index][1].unwrap_or(0.0),
            );
            if tb - ta > 1e-9 {
                entities.push(line([ta, ra], [tb, ra], LAYER_MAIN));
                entities.push(line([ta, -ra], [tb, -ra], LAYER_MAIN));
            }
            // 分度线（点划线，3中心线层；不受倒角影响）—— 凸出量与齿轮生成器中心线**同一口径**：
            // 总长 = 特征长 + `CENTER_OVERHANG`×n（两端各 3n），绕段中心对称（`gear::centerline_len`，
            // 与 `gear.rs::side_view` / `spline_axial_view` / `internal_bore_section` 同一函数）。
            let cl = crate::gear::centerline_len(seg.l, frame_scale);
            let xm = x0 + seg.l / 2.0;
            entities.push(line([xm - cl / 2.0, r], [xm + cl / 2.0, r], LAYER_CENTER));
            entities.push(line([xm - cl / 2.0, -r], [xm + cl / 2.0, -r], LAYER_CENTER));
            // 内侧直径线（外齿 = 齿根圆 / 内齿 = 里侧齿顶）：剖视恒画（`1轮廓实线层`，齿部按不剖，
            // 也是剖面线边界）；**常规侧视图按 MARK 开关** —— `SPLINE`（渐开线花键）画 `2细线层`
            // （青色 ACI 4）小径细实线、`GEAR`（齿轮）不画（既有齿轮口径「无齿根线」；
            // `gear.rs:4364` 既有测试 + handbook 16 §九「花键有小径细实线、齿轮没有」）。
            section_lines.push(line([x0, rf], [x1, rf], LAYER_MAIN));
            section_lines.push(line([x0, -rf], [x1, -rf], LAYER_MAIN));
            if gear.involute {
                spline_regular.push(line([x0, rf], [x1, rf], LAYER_THIN));
                spline_regular.push(line([x0, -rf], [x1, -rf], LAYER_THIN));
            }
            // 剖面线边界：外齿按齿根圆（齿部不剖）；内齿按大径（最小实现：按实体段近似，不挖内孔）。
            let bound = hatch_bound_radius(gear);
            profile.push(lr_line([x0, bound], [x1, bound]));
            continue;
        }
        // ── 花键段（矩形花键）：小径细线 / 收尾弧 / 两根细竖线；
        //    常规视图落 `2细线层`，剖视改 `1轮廓实线层`；剖面线边界 = 小径线 + 收尾弧。──
        if let Some(spline) = &seg.spline {
            let (x0, x1) = (x0s[index], x0s[index] + seg.l); // x1 = L + l
            let (r, ra) = (spline.minor_radius(), spline.major_radius());
            let xm1 = x0 + spline.len; // 满齿段右端 = 收尾起点
            let l = spline.runout();
            let rh = spline.hob_radius();
            let a_end = spline.runout_end_angle();
            // 大径线：满齿段与收尾段两段（模板 [43]/[44] + [46]/[47] 就在 x=L 断开）。
            let xs = x0 + own_ch[index][0].unwrap_or(0.0);
            if xm1 - xs > 1e-9 {
                entities.push(line([xs, ra], [xm1, ra], LAYER_MAIN));
                entities.push(line([xs, -ra], [xm1, -ra], LAYER_MAIN));
            }
            if x1 > xm1 + 1e-9 {
                entities.push(line([xm1, ra], [x1, ra], LAYER_MAIN));
                entities.push(line([xm1, -ra], [x1, -ra], LAYER_MAIN));
            }
            // 左端倒角（如落在本段）把细线起点内缩到倒角与小径的交点。
            let left_inset = own_ch[index][0]
                .map(|c| (c - (ra - r)).max(0.0))
                .unwrap_or(0.0);
            let xm0 = x0 + left_inset;
            // 左端倒角（落在本段）切到小径以下：剖面线的倒角段只取到与小径的交点。
            if let Some(c) = own_ch[index][0] {
                let face_y = ra - c;
                if face_y < r - 1e-12 && xm0 > x0 + 1e-12 {
                    profile.push(lr_line([x0, face_y], [xm0, r]));
                }
            }
            // 常规视图：小径细线 + 收尾弧（与小径相切、与大径相交）+ 两根细竖线。
            spline_regular.push(line([xm0, r], [xm1, r], LAYER_THIN));
            spline_regular.push(line([xm0, -r], [xm1, -r], LAYER_THIN));
            spline_regular.push(arc([xm1, r + rh], rh, 270.0, 360.0 - a_end, LAYER_THIN));
            spline_regular.push(arc([xm1, -(r + rh)], rh, a_end, 90.0, LAYER_THIN));
            spline_regular.push(line([xm1, -ra], [xm1, ra], LAYER_THIN));
            if index + 1 < count {
                // 后面还有段：收尾终点细竖线（自由端时由右端面线闭合）。
                spline_regular.push(line([xm1 + l, -ra], [xm1 + l, ra], LAYER_THIN));
            }
            // 剖视可见的小径线 / 收尾弧（`1轮廓实线层`）。
            section_lines.push(line([xm0, r], [xm1, r], LAYER_MAIN));
            section_lines.push(line([xm0, -r], [xm1, -r], LAYER_MAIN));
            section_lines.push(arc([xm1, r + rh], rh, 270.0, 360.0 - a_end, LAYER_MAIN));
            section_lines.push(arc([xm1, -(r + rh)], rh, a_end, 90.0, LAYER_MAIN));
            // 剖面线边界：小径线 → 收尾弧（齿部不剖，与模板「只填轴线↔小径」同口径）。
            profile.push(lr_line([xm0, r], [xm1, r]));
            profile.push(HatchEdge::Arc {
                c: [xm1, r + rh],
                r: rh,
                start_deg: 270.0,
                end_deg: 360.0 - a_end,
                ccw: true,
            });
            // 大径线/剖面线都已就位：不进入通用轮廓分支（通用会把大径线画成整段）。
            continue;
        }
        // ── 轴槽段（KEY，GB/T 1095）：顶线按视图拆分、缺口 + sagitta 线、键叠画。
        //    下轮廓两视图共用；上轮廓：常规 = 整段连续线，剖视 = 缺口两段；
        //    hatch 边界走带台阶的真实轮廓（模板 C0/C1 逐图元）。──
        if let Some(kw) = &seg.keyway {
            let label = seg_label(index + 1);
            let kg = keyway_geom(seg, kw, x0s[index], index, count, &label)?;
            let xs = x0s[index] + keyway_chamfer(seg, End::L);
            let xe = x0s[index] + seg.l - keyway_chamfer(seg, End::R);
            if xe - xs > 1e-9 {
                if kg.double {
                    // 双槽：下轮廓在剖视里也按槽拆（镜像槽），常规仍整段。
                    keyway_normal.push(line([xs, -kg.r], [xe, -kg.r], LAYER_MAIN));
                } else {
                    entities.push(line([xs, -kg.r], [xe, -kg.r], LAYER_MAIN));
                }
                keyway_normal.push(line([xs, kg.r], [xe, kg.r], LAYER_MAIN));
            }
            // 剖面线分区：
            // * 单槽：下环 = “无键槽”的对称上半边界（键槽只在轴上半边，模板 C0/C1 下环不复制缺口）
            //   → `keyway_notch` 记缺口边、`keyway_plain` 给平顶线跨度；
            // * 双槽：几何关于轴线对称 → 下环直接镜像上环（缺口保留），因此不注册 notch/plain。
            if !kg.double {
                keyway_plain.push((xs, xe, kg.r));
            }
            let register_notch = !kg.double;
            let mut notch = |edge: HatchEdge| {
                profile.push(edge);
                if register_notch {
                    keyway_notch.push(edge);
                }
            };
            match kg.place {
                KeywayPlace::Mid => {
                    if kg.slot0 - xs > 1e-9 {
                        section_lines.push(line([xs, kg.r], [kg.slot0, kg.r], LAYER_MAIN));
                    }
                    if xe - kg.slot1 > 1e-9 {
                        section_lines.push(line([kg.slot1, kg.r], [xe, kg.r], LAYER_MAIN));
                    }
                    section_lines.push(line([kg.slot0, kg.r], [kg.slot0, kg.floor], LAYER_MAIN));
                    if let Some(gh) = kg.guided {
                        guided_floor(
                            &mut section_lines,
                            &mut notch,
                            &kg,
                            &gh,
                            kg.slot0,
                            kg.slot1,
                            frame_scale,
                        );
                    }
                    // 槽底直线：模板里贯通槽两端（固定螺钉孔在槽底之下，被这条线横过）；
                    // 导向/普通一律画。导向曾漏画 → 剖视槽轮廓在槽底不闭合（用户 2026-09-25 报告）。
                    section_lines.push(line([kg.slot0, kg.floor], [kg.slot1, kg.floor], LAYER_MAIN));
                    section_lines.push(line([kg.slot1, kg.floor], [kg.slot1, kg.r], LAYER_MAIN));
                    section_lines.push(line([kg.slot0, kg.y_sag], [kg.slot1, kg.y_sag], LAYER_MAIN));
                    if kg.double {
                        // 双槽：绕轴心 180° 对置 —— 下槽逐线镜像（含槽底与 sagitta 线）。
                        if kg.slot0 - xs > 1e-9 {
                            section_lines.push(line([xs, -kg.r], [kg.slot0, -kg.r], LAYER_MAIN));
                        }
                        if xe - kg.slot1 > 1e-9 {
                            section_lines.push(line([kg.slot1, -kg.r], [xe, -kg.r], LAYER_MAIN));
                        }
                        section_lines.push(line([kg.slot0, -kg.r], [kg.slot0, -kg.floor], LAYER_MAIN));
                        section_lines.push(line([kg.slot0, -kg.floor], [kg.slot1, -kg.floor], LAYER_MAIN));
                        section_lines.push(line([kg.slot1, -kg.floor], [kg.slot1, -kg.r], LAYER_MAIN));
                        section_lines.push(line([kg.slot0, -kg.y_sag], [kg.slot1, -kg.y_sag], LAYER_MAIN));
                    }
                    // hatch 边界（上半，左→右）：顶左 → 壁下 → 槽底 → 壁上 → 顶右。
                    notch(lr_line([xs, kg.r], [kg.slot0, kg.r]));
                    notch(HatchEdge::Line {
                        a: [kg.slot0, kg.r],
                        b: [kg.slot0, kg.floor],
                    });
                    if kg.guided.is_none() {
                        notch(lr_line([kg.slot0, kg.floor], [kg.slot1, kg.floor]));
                    }
                    notch(HatchEdge::Line {
                        a: [kg.slot1, kg.floor],
                        b: [kg.slot1, kg.r],
                    });
                    notch(lr_line([kg.slot1, kg.r], [xe, kg.r]));
                }
                KeywayPlace::End => {
                    let side = kg.side.expect("端置轴槽必有 side");
                    match side {
                        End::L => {
                            if xe - kg.wall > 1e-9 {
                                section_lines.push(line([kg.wall, kg.r], [xe, kg.r], LAYER_MAIN));
                            }
                            section_lines.push(line([kg.wall, kg.floor], [kg.wall, kg.r], LAYER_MAIN));
                            // 端置只属于普通平键（导向已拦，见 `assemble/validate/keyway_geom`）。
                            section_lines.push(line([kg.face, kg.floor], [kg.wall, kg.floor], LAYER_MAIN));
                            section_lines.push(line([kg.face, kg.y_sag], [kg.wall, kg.y_sag], LAYER_MAIN));
                            // 键 B 平端竖线（模板 ent84）：x = 端面 + t₁，槽底 → sagitta。
                            section_lines.push(line(
                                [kg.face + kg.t1, kg.floor],
                                [kg.face + kg.t1, kg.y_sag],
                                LAYER_MAIN,
                            ));
                            if kg.double {
                                // 双槽：下槽镜像（开口端在端面、闭端壁同 x）。
                                if xe - kg.wall > 1e-9 {
                                    section_lines.push(line([kg.wall, -kg.r], [xe, -kg.r], LAYER_MAIN));
                                }
                                section_lines.push(line([kg.wall, -kg.r], [kg.wall, -kg.floor], LAYER_MAIN));
                                section_lines.push(line([kg.face, -kg.floor], [kg.wall, -kg.floor], LAYER_MAIN));
                                section_lines.push(line([kg.face, -kg.y_sag], [kg.wall, -kg.y_sag], LAYER_MAIN));
                                section_lines.push(line(
                                    [kg.face + kg.t1, -kg.floor],
                                    [kg.face + kg.t1, -kg.y_sag],
                                    LAYER_MAIN,
                                ));
                            }
                            notch(lr_line([kg.face, kg.floor], [kg.wall, kg.floor]));
                            notch(HatchEdge::Line {
                                a: [kg.wall, kg.floor],
                                b: [kg.wall, kg.r],
                            });
                            notch(lr_line([kg.wall, kg.r], [xe, kg.r]));
                        }
                        End::R => {
                            if kg.wall - xs > 1e-9 {
                                section_lines.push(line([xs, kg.r], [kg.wall, kg.r], LAYER_MAIN));
                            }
                            section_lines.push(line([kg.wall, kg.r], [kg.wall, kg.floor], LAYER_MAIN));
                            // 端置只属于普通平键（导向已拦）。
                            section_lines.push(line([kg.wall, kg.floor], [kg.face, kg.floor], LAYER_MAIN));
                            section_lines.push(line([kg.wall, kg.y_sag], [kg.face, kg.y_sag], LAYER_MAIN));
                            // 键 B 平端竖线：x = 端面 − t₁，槽底 → sagitta。
                            section_lines.push(line(
                                [kg.face - kg.t1, kg.floor],
                                [kg.face - kg.t1, kg.y_sag],
                                LAYER_MAIN,
                            ));
                            if kg.double {
                                // 双槽：下槽镜像（开口端在右端面、闭端壁同 x）。
                                if kg.wall - xs > 1e-9 {
                                    section_lines.push(line([xs, -kg.r], [kg.wall, -kg.r], LAYER_MAIN));
                                }
                                section_lines.push(line([kg.wall, -kg.floor], [kg.wall, -kg.r], LAYER_MAIN));
                                section_lines.push(line([kg.wall, -kg.floor], [kg.face, -kg.floor], LAYER_MAIN));
                                section_lines.push(line([kg.wall, -kg.y_sag], [kg.face, -kg.y_sag], LAYER_MAIN));
                                section_lines.push(line(
                                    [kg.face - kg.t1, -kg.floor],
                                    [kg.face - kg.t1, -kg.y_sag],
                                    LAYER_MAIN,
                                ));
                            }
                            notch(lr_line([xs, kg.r], [kg.wall, kg.r]));
                            notch(HatchEdge::Line {
                                a: [kg.wall, kg.r],
                                b: [kg.wall, kg.floor],
                            });
                            notch(lr_line([kg.wall, kg.floor], [kg.face, kg.floor]));
                        }
                    }
                }
            }
            emit_key_overlay(&mut keyway_normal, &kg, frame_scale);
            continue;
        }
        let start_shift = own_ch[index][0]
            .unwrap_or(0.0)
            .max(own_ov[index][0].unwrap_or(0.0))
            .max(own_relief[index][0].unwrap_or(0.0));
        let end_shift = own_ch[index][1]
            .unwrap_or(0.0)
            .max(own_ov[index][1].unwrap_or(0.0))
            .max(own_relief[index][1].unwrap_or(0.0))
            .max(
                local_threads[index]
                    .map(|lay| lay.face - lay.contour_end)
                    .unwrap_or(0.0),
            );
        if start_shift + end_shift > seg.l + 1e-9 {
            return Err(crate::i18n::t_fmt("cmd.shaft.err.both_ends_overlap", &[("label", &seg_label(index + 1)), ("p0", &(trim(start_shift)).to_string()), ("p1", &(trim(end_shift)).to_string()), ("p2", &(trim(seg.l)).to_string())]));
        }
        let xs = x0s[index] + start_shift;
        let xe = x0s[index] + seg.l - end_shift;
        if xe - xs > 1e-9 {
            let ys = radius_at(seg, x0s[index], xs);
            let ye = radius_at(seg, x0s[index], xe);
            entities.push(line([xs, ys], [xe, ye], LAYER_MAIN));
            entities.push(line([xs, -ys], [xe, -ye], LAYER_MAIN));
            if seg.spline.is_none() && plain_thread(seg).is_none() {
                profile.push(lr_line([xs, ys], [xe, ye]));
            }
        }
        // 普通全螺纹段的剖面线边界：按小径包络（牙顶区不剖），左/右端倒角
        // 深入小径时沿倒角线收进去；与参考件 `轴剖视图.dxf` 右视图口径一致。
        if let Some(thread) = plain_thread(seg) {
            let (x0, x1) = (x0s[index], x0s[index] + seg.l);
            let r = seg.s / 2.0;
            let r1 = thread.minor_radius(seg.s);
            let cut = (r - r1).max(0.0);
            // 左端：倒角（贴本段时）与 r1 的交点；否则从小径线起。
            let mut flat_start = x0;
            if let Some(c) = own_ch[index][0] {
                let face_y = r - c;
                if face_y < r1 - 1e-12 {
                    let x_cross = x0 + (c - cut);
                    profile.push(lr_line([x0, face_y], [x_cross, r1]));
                    flat_start = x_cross;
                }
            } else if own_ov[index][0].is_some() || own_relief[index][0].is_some() {
                // 槽体边界已由端面循环推入；从小径线起（垂直落下去）。
                flat_start = xs;
                profile.push(HatchEdge::Line {
                    a: [xs, r],
                    b: [xs, r1],
                });
            }
            // 右端：倒角（贴本段时）与 r1 的交点；否则到段末。
            let mut flat_end = x1;
            let mut right_piece: Option<([f64; 2], [f64; 2])> = None;
            if let Some(c) = own_ch[index][1] {
                let face_y = r - c;
                if face_y < r1 - 1e-12 {
                    flat_end = x1 - (c - cut);
                    right_piece = Some(([flat_end, r1], [x1, face_y]));
                }
            } else if own_ov[index][1].is_some() || own_relief[index][1].is_some() {
                flat_end = xe;
                profile.push(HatchEdge::Line {
                    a: [xe, r1],
                    b: [xe, r],
                });
            }
            if flat_end - flat_start > 1e-9 {
                profile.push(lr_line([flat_start, r1], [flat_end, r1]));
            }
            if let Some((a, b)) = right_piece {
                profile.push(lr_line(a, b));
            }
        }
        // ── 螺纹段：`TL`/`RL` 局部螺纹（右端台肩），或旧口径整段全螺纹；
        //    小径细实线上下各一条（2细线层），左端按倒角内缩、右端止于螺尾/
        //    退刀槽斜壁；同段端面倒角比小径深时不挑出材料外。 ──
        if let Some(lay) = local_threads[index] {
            if own_ch[index][1].is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.tlro_no_chamfer_plain", &[("label", &seg_label(index + 1))]));
            }
            if own_ov[index][1].is_some() {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.tlro_no_ov", &[("label", &seg_label(index + 1))]));
            }
            match lay.relief {
                Some(dims) => {
                    // GB/T 3 表 2（图 2）：台肩面 → R 圆角 → 槽底 g1 → 斜壁（g2 接大径）。
                    let rg = dims.dg / 2.0;
                    entities.extend(
                        detail::relief_groove_entities(&dims)
                            .into_iter()
                            .map(|e| mirror_x(e, lay.face)),
                    );
                    // 槽底终止 / 斜壁终点两条贯通竖线（槽肩那条在端面循环按切点画）。
                    through_line(&mut through, lay.face - dims.g1, rg);
                    through_line(&mut through, lay.face - dims.g2, lay.r);
                    // 剖面线环（上半，左→右）：斜壁 → 槽底 → 圆角
                    profile.push(lr_line([lay.face - dims.g2, lay.r], [lay.face - dims.g1, rg]));
                    profile.push(lr_line([lay.face - dims.g1, rg], [lay.face - dims.r, rg]));
                    profile.push(HatchEdge::Arc {
                        c: [lay.face - dims.r, lay.relief_tangent],
                        r: dims.r,
                        start_deg: 270.0,
                        end_deg: 360.0,
                        ccw: true,
                    });
                }
                None => {
                    // GB/T 3 图 1 第一种形式：锥面（粗实线）→ 螺尾（细实线）→ 完整螺纹。
                    if lay.runout_start > lay.boundary + 1e-9 {
                        entities.push(line(
                            [lay.face, lay.r],
                            [lay.runout_start, lay.r1],
                            LAYER_MAIN,
                        ));
                        entities.push(line(
                            [lay.face, -lay.r],
                            [lay.runout_start, -lay.r1],
                            LAYER_MAIN,
                        ));
                    }
                    entities.push(line(
                        [lay.runout_start, lay.r1],
                        [lay.boundary, lay.r],
                        LAYER_THIN,
                    ));
                    entities.push(line(
                        [lay.runout_start, -lay.r1],
                        [lay.boundary, -lay.r],
                        LAYER_THIN,
                    ));
                    // 完整螺纹 / 螺尾分界竖线（图 1(a) 里那条短竖线，粗实线）。
                    entities.push(line([lay.boundary, -lay.r], [lay.boundary, lay.r], LAYER_MAIN));
                }
            }
            // 小径细实线：左端按本段左端倒角/退刀槽内缩，右端止于螺尾起点/斜壁交点。
            let cut = lay.r - lay.r1;
            let mut left_inset = own_ch[index][0]
                .filter(|c| *c > cut + 1e-12)
                .map(|c| c - cut)
                .unwrap_or(0.0);
            if let Some(dims) = relief_dims[index][0] {
                left_inset = left_inset.max(relief_minor_inset(lay.r, lay.r1, &dims));
            }
            let xt0 = (x0s[index] + left_inset).max(lay.full_start);
            let xt1 = lay.minor_end;
            if xt1 - xt0 <= 1e-9 {
                return Err(crate::i18n::t_fmt("cmd.shaft.err.tlro_no_fit", &[("label", &seg_label(index + 1))]));
            }
            entities.push(line([xt0, lay.r1], [xt1, lay.r1], LAYER_THIN));
            entities.push(line([xt0, -lay.r1], [xt1, -lay.r1], LAYER_THIN));
        } else if let Some(thread) = &seg.thread {
            let (x0, x1) = (x0s[index], x0s[index] + seg.l);
            let minor = thread.minor_radius(seg.s);
            let cut = seg.s / 2.0 - minor;
            let inset = |chamfer: Option<f64>| {
                chamfer
                    .filter(|c| *c > cut + 1e-12)
                    .map(|c| c - cut)
                    .unwrap_or(0.0)
            };
            let xt0 = x0 + {
                let mut left = inset(own_ch[index][0]);
                if let Some(dims) = relief_dims[index][0] {
                    left = left.max(relief_minor_inset(seg.s / 2.0, minor, &dims));
                }
                left
            };
            let xt1 = x1 - inset(own_ch[index][1]);
            if xt1 - xt0 > 1e-9 {
                entities.push(line([xt0, minor], [xt1, minor], LAYER_THIN));
                entities.push(line([xt0, -minor], [xt1, -minor], LAYER_THIN));
            }
        }
    }

    // ── 轴线（3中心线层）：长度 = 总长 + 图框比例 × 6，两端各伸出 3n ──
    let half_overhang = 3.0 * frame_scale;
    entities.push(line(
        [-half_overhang, 0.0],
        [total + half_overhang, 0.0],
        LAYER_CENTER,
    ));

    // ── 剖面线环：上半边界（左→右）在 `build()` 里按 x 排序后拆成上/下两个环 ──

    Ok(Geometry {
        entities,
        through,
        section_lines,
        spline_regular,
        keyway_normal,
        keyway_notch,
        keyway_plain,
        keyway_plain_extra,
        profile,
        total_length: total,
        max_diameter,
        segment_count: count,
    })
}

/// 上半剖面线边界（左→右）按 x 拼接后拆成**上/下两个简单环**：
/// 上环 = 上半边界 → 右端面下到轴线 → 轴线回到左端 → 左端面上到上半边界；
/// 下环 = 上环关于 x 轴的镜像反走。参考件 `轴剖视图.dxf` 右视图：一个 HATCH、2 环。
///
/// 端面的闭合高度直接取上半边界链两端点的 y（含齿轮齿根 / 螺纹小径包络 / 端面倒角）。
fn close_hatch_rings(
    profile: Vec<HatchEdge>,
    x_end: f64,
    keyway_notch: &[HatchEdge],
    keyway_plain: &[(f64, f64, f64)],
    keyway_plain_extra: &[HatchEdge],
) -> Vec<Vec<HatchEdge>> {
    if profile.is_empty() {
        return Vec::new();
    }
    // 下环 = “无键槽”的对称上半边界：把轴槽缺口专用边剔掉，换成一条平顶线。
    // （键槽只画在轴的上半边，模板 C0/C1 的下环绝不复制缺口。）
    let same = |a: &HatchEdge, b: &HatchEdge| hatch_edge_same(a, b);
    let mut plain: Vec<HatchEdge> = profile
        .iter()
        .filter(|e| !keyway_notch.iter().any(|n| same(n, e)))
        .cloned()
        .collect();
    for (xs, xe, r) in keyway_plain {
        plain.push(lr_line([*xs, *r], [*xe, *r]));
    }
    plain.extend_from_slice(keyway_plain_extra);
    let Some(upper) = close_one_chain(profile, x_end) else {
        return Vec::new();
    };
    let Some(plain_upper) = close_one_chain(plain, x_end) else {
        return Vec::new();
    };
    let lower: Vec<HatchEdge> = plain_upper
        .iter()
        .rev()
        .map(mirror_reverse_edge)
        .collect();
    vec![upper, lower]
}

/// 一条上半边界链（左→右）→ 闭合环：右端面下到轴线 → 轴线回左端 → 左端面上到链起点。
fn close_one_chain(mut profile: Vec<HatchEdge>, x_end: f64) -> Option<Vec<HatchEdge>> {
    profile.retain(|e| !edge_is_degenerate(e));
    if profile.is_empty() {
        return None;
    }
    profile.sort_by(|a, b| {
        let (a0, a1) = edge_span(a);
        let (b0, b1) = edge_span(b);
        a0.partial_cmp(&b0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a1.partial_cmp(&b1).unwrap_or(std::cmp::Ordering::Equal))
    });
    let (left, right) = hatch_chain_endpoints(&profile)?;
    profile.push(HatchEdge::Line {
        a: [x_end, right[1]],
        b: [x_end, 0.0],
    });
    profile.push(HatchEdge::Line {
        a: [x_end, 0.0],
        b: [0.0, 0.0],
    });
    profile.push(HatchEdge::Line {
        a: [0.0, 0.0],
        b: [0.0, left[1]],
    });
    Some(profile)
}

/// 两条剖面线边界边是否同一条（坐标全等；仅用于剔出轴槽缺口专用边）。
fn hatch_edge_same(a: &HatchEdge, b: &HatchEdge) -> bool {
    match (a, b) {
        (
            HatchEdge::Line { a: a0, b: a1 },
            HatchEdge::Line { a: b0, b: b1 },
        ) => a0 == b0 && a1 == b1,
        (
            HatchEdge::Arc {
                c: c0,
                r: r0,
                start_deg: s0,
                end_deg: e0,
                ccw: w0,
            },
            HatchEdge::Arc {
                c: c1,
                r: r1,
                start_deg: s1,
                end_deg: e1,
                ccw: w1,
            },
        ) => c0 == c1 && r0 == r1 && s0 == s1 && e0 == e1 && w0 == w1,
        _ => false,
    }
}

/// 上半剖面线链的最小/最大 x 端点（用于上/下环的端面闭合）。
fn hatch_chain_endpoints(profile: &[HatchEdge]) -> Option<([f64; 2], [f64; 2])> {
    let mut left: Option<[f64; 2]> = None;
    let mut right: Option<[f64; 2]> = None;
    for edge in profile {
        for p in edge_endpoints(edge) {
            if left.map_or(true, |q| p[0] < q[0]) {
                left = Some(p);
            }
            if right.map_or(true, |q| p[0] > q[0]) {
                right = Some(p);
            }
        }
    }
    Some((left?, right?))
}

/// 边界段两端点（弧用起讫点）。
fn edge_endpoints(edge: &HatchEdge) -> [[f64; 2]; 2] {
    match *edge {
        HatchEdge::Line { a, b } => [a, b],
        HatchEdge::Arc {
            c,
            r,
            start_deg,
            end_deg,
            ..
        } => [
            [
                c[0] + r * start_deg.to_radians().cos(),
                c[1] + r * start_deg.to_radians().sin(),
            ],
            [
                c[0] + r * end_deg.to_radians().cos(),
                c[1] + r * end_deg.to_radians().sin(),
            ],
        ],
    }
}

/// 直线段按 x 方向排成左→右（竖直段不要用它，方向要按环的走向手写）。
fn lr_line(a: [f64; 2], b: [f64; 2]) -> HatchEdge {
    if a[0] <= b[0] {
        HatchEdge::Line { a, b }
    } else {
        HatchEdge::Line { a: b, b: a }
    }
}

/// 边界段两端的 x 范围（弧用起讫点；本文件的弧都是 x 单调的四分之一弧）。
fn edge_span(edge: &HatchEdge) -> (f64, f64) {
    let (a, b) = match *edge {
        HatchEdge::Line { a, b } => (a, b),
        HatchEdge::Arc {
            c,
            r,
            start_deg,
            end_deg,
            ..
        } => (
            [
                c[0] + r * start_deg.to_radians().cos(),
                c[1] + r * start_deg.to_radians().sin(),
            ],
            [
                c[0] + r * end_deg.to_radians().cos(),
                c[1] + r * end_deg.to_radians().sin(),
            ],
        ),
    };
    if a[0] <= b[0] {
        (a[0], b[0])
    } else {
        (b[0], a[0])
    }
}

/// 零长边界段（与 `partgen_b4::dedup_ring` 同口径：重复点会让宿主判边界无效）。
fn edge_is_degenerate(edge: &HatchEdge) -> bool {
    match *edge {
        HatchEdge::Line { a, b } => (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9,
        HatchEdge::Arc {
            r,
            start_deg,
            end_deg,
            ..
        } => r < 1e-9 || (end_deg - start_deg).abs() < 1e-9,
    }
}

/// 关于 x 轴镜像 + 反向（环从上半侧折回下半侧时用）。
///
/// 圆弧推导：镜像是 `θ' = −θ`；再反向一次 ⇒ `start' = −end`、`end' = −start`、
/// `ccw' = ccw`（两次方向翻转相互抵消）。
fn mirror_reverse_edge(edge: &HatchEdge) -> HatchEdge {
    match *edge {
        HatchEdge::Line { a, b } => HatchEdge::Line {
            a: [b[0], -b[1]],
            b: [a[0], -a[1]],
        },
        HatchEdge::Arc {
            c,
            r,
            start_deg,
            end_deg,
            ccw,
        } => HatchEdge::Arc {
            c: [c[0], -c[1]],
            r,
            start_deg: -end_deg,
            end_deg: -start_deg,
            ccw,
        },
    }
}

/// 按 `at` / `rot`（度）放置：绕基点旋转后平移。基点 = **第 1 段轴线左端**。
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
        .map(|entity| match entity {
            EntityType::Line(mut l) => {
                let a = map([l.start.x, l.start.y]);
                let b = map([l.end.x, l.end.y]);
                l.start = Vector3::new(a[0], a[1], l.start.z);
                l.end = Vector3::new(b[0], b[1], l.end.z);
                EntityType::Line(l)
            }
            EntityType::Arc(mut a) => {
                let center = map([a.center.x, a.center.y]);
                a.center = Vector3::new(center[0], center[1], a.center.z);
                a.start_angle += rad;
                a.end_angle += rad;
                EntityType::Arc(a)
            }
            EntityType::Hatch(mut h) => {
                // 剖面线边界跟着转/移；图案方向（ANSI31 45°）是材质约定，不随视图旋转。
                for path in &mut h.paths {
                    for edge in &mut path.edges {
                        match edge {
                            BoundaryEdge::Line(e) => {
                                let a = map([e.start.x, e.start.y]);
                                let b = map([e.end.x, e.end.y]);
                                e.start = Vector2::new(a[0], a[1]);
                                e.end = Vector2::new(b[0], b[1]);
                            }
                            BoundaryEdge::CircularArc(e) => {
                                let c = map([e.center.x, e.center.y]);
                                e.center = Vector2::new(c[0], c[1]);
                                e.start_angle += rad;
                                e.end_angle += rad;
                            }
                            BoundaryEdge::EllipticArc(e) => {
                                let c = map([e.center.x, e.center.y]);
                                e.center = Vector2::new(c[0], c[1]);
                                let (vx, vy) = (e.major_axis_endpoint.x, e.major_axis_endpoint.y);
                                e.major_axis_endpoint =
                                    Vector2::new(vx * cos - vy * sin, vx * sin + vy * cos);
                                e.start_angle += rad;
                                e.end_angle += rad;
                            }
                            BoundaryEdge::Polyline(e) => {
                                for v in &mut e.vertices {
                                    let p = map([v.x, v.y]);
                                    v.x = p[0];
                                    v.y = p[1];
                                }
                            }
                            BoundaryEdge::Spline(e) => {
                                for q in &mut e.control_points {
                                    let p = map([q.x, q.y]);
                                    q.x = p[0];
                                    q.y = p[1];
                                }
                                for q in &mut e.fit_points {
                                    let p = map([q.x, q.y]);
                                    q.x = p[0];
                                    q.y = p[1];
                                }
                            }
                        }
                    }
                }
                EntityType::Hatch(h)
            }
            other => other,
        })
        .collect()
}

// ══════════════════════════════════════════════════════════════════════════
// 对外：初始化拦截 / 块名 / SVG 预览
// ══════════════════════════════════════════════════════════════════════════

/// OCSM 初始化检查（判据与 `gear::ocsm_ready` 相同，措辞改成轴）：
/// 图形层在、中心线层挂着点划线；不满足就在插入前拦下，避免中心线变实线白线。
pub fn ocsm_ready(doc: &ocs_plugin_api::host::acadrust::CadDocument) -> Result<(), String> {
    let find = |name: &str| {
        doc.layers
            .iter()
            .find(|ly| ly.name.eq_ignore_ascii_case(name))
    };
    const NEED: [&str; 4] = [LAYER_MAIN, LAYER_THIN, LAYER_CENTER, LAYER_HATCH];
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
    if !missing.is_empty() {
        let sep = match crate::i18n::lang() {
            crate::i18n::Lang::Zh => "、",
            _ => ", ",
        };
        return Err(crate::i18n::t_fmt(
            "cmd.shaft.ready.missing_layers",
            &[("list", &missing.join(sep))],
        ));
    }
    Err(crate::i18n::t_fmt(
        "cmd.shaft.ready.center_linetype",
        &[("layer", LAYER_CENTER)],
    ))
}

/// 轴块名：只由几何（段序列 + 视图）决定 —— 同模型复用一个块，不同模型不会误用别人的块。
/// 用 FNV-1a 64 位而不是 `DefaultHasher`：跨进程/跨版本稳定，写进 DWG 的块名可复现。
pub fn block_name(program: &Program) -> String {
    let canonical = format!(
        "{}|{}",
        program.view.key(),
        serde_json::to_string(&program.segments).unwrap_or_default()
    );
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in canonical.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("OCSM_SHAFT_{hash:016X}")
}

/// GUI 预览：轴的侧视图 SVG（图元渲染复用 `gear.rs` 的口径）。
///
/// `gear::svg_of` 的 Hatch 预览只按包围盒裁 45° 线（它那里全是轴对齐矩形环），
/// 轴的剖面环是带台阶/倒角/槽的多边形 → 先在 `preview_entities` 里把 HATCH
/// 换成**裁剪到真实边界**的 45° 线，再交给同一套渲染。导出图纸仍是真 HATCH。
pub fn preview_svg(program: &Program) -> Result<String, String> {
    let shaft = build(program, 1.0)?;
    Ok(crate::gear::svg_of(&preview_entities(&shaft.entities), 460.0))
}

/// **轴段计算书**（纯数据、无 IO）：段清单总览（类型/关键参数/长度/外径）。
/// 命令入口：`OCSMSHAFT … report`（可选 `report=<path>` 写文件）。
pub fn build_report(program: &Program) -> Result<String, String> {
    let built = build(program, 1.0)?;
    let mut md = String::new();
    md.push_str(&format!("{}\n\n", crate::i18n::t("cmd.shaft.report.title")));
    md.push_str(&crate::i18n::t_fmt("cmd.shaft.report.header", &[("p0", &(program.view.key()).to_string()), ("p1", &(program.view.label()).to_string()), ("p2", &(built.segment_count).to_string()), ("p3", &(trim(built.total_length)).to_string()), ("p4", &(trim(built.max_diameter)).to_string())]));
    md.push_str(&format!(
        "{}\n\n{}\n|---|---|---|---|---|\n",
        crate::i18n::t("cmd.shaft.report.sec_list"),
        crate::i18n::t("cmd.shaft.report.list_header")
    ));
    for (i, seg) in program.segments.iter().enumerate() {
        let (kind, params) = if let Some(g) = &seg.gear {
            let gp = g.params();
            (
                crate::i18n::t("cmd.shaft.report.type_gear"),
                format!("m={} z={} α={}°", trim(gp.m), gp.z, trim(gp.alpha_deg)),
            )
        } else if let Some(s) = &seg.spline {
            (
                crate::i18n::t("cmd.shaft.report.type_spline"),
                format!("N={} d={} D={} B={}", s.n, trim(s.d), trim(s.big), trim(s.b)),
            )
        } else if let Some(t) = &seg.thread {
            (
                crate::i18n::t("cmd.shaft.report.type_thread"),
                format!(
                    "P={}",
                    t.pitch.map(trim).unwrap_or_else(|| crate::i18n::t("cmd.shaft.report.simplified_085d").into())
                ),
            )
        } else {
            let mut params = format!("S={} E={}", trim(seg.s), trim(seg.e));
            if let Some(keyway) = &seg.keyway {
                let b = keyway_b(keyway, seg.s).unwrap_or(f64::NAN);
                let t1 = keyway_t1(b, keyway.t1)
                    .map(trim)
                    .unwrap_or_else(|_| "?".into());
                params.push_str(&crate::i18n::t_fmt("cmd.shaft.report.keyway", &[("p0", &(keyway.kind.cn()).to_string()), ("p1", &(trim(keyway.l)).to_string()), ("p2", &(keyway.place.cn()).to_string()), ("p3", &(trim(b)).to_string()), ("p4", &(t1).to_string())]));
            }
            (crate::i18n::t("cmd.shaft.report.type_plain"), params)
        };
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            i + 1,
            kind,
            params,
            trim(seg.l),
            trim(seg.outer_radius(End::L) * 2.0)
        ));
    }
    md.push('\n');
    // ── 2. 花键参数表（GB/T 3478）——与 `OCSMCARD 花键参数表` 同一份 `compute()` 结果 ──
    let mut sp_section = String::new();
    let mut sp_count = 0usize;
    for (i, seg) in program.segments.iter().enumerate() {
        let Some(g) = &seg.gear else { continue };
        if !g.involute {
            continue;
        }
        let Ok(input) = crate::spline_table::input_from_gear(g) else {
            continue; // 非 GB/T 3478 压力角（如 DIN/普通齿轮）→ 本节不列
        };
        let Ok(table) = crate::spline_tol::compute(&input) else {
            continue;
        };
        let Ok(body) = crate::spline_table::markdown_table(input.side, &table) else {
            continue;
        };
        sp_count += 1;
        sp_section.push_str(&crate::i18n::t_fmt("cmd.shaft.report.spline_heading", &[("p0", &(i + 1).to_string()), ("p1", &(input.grade_fit_label()).to_string()), ("body", &body.to_string())]));
    }
    if sp_count > 0 {
        md.push_str(&format!("{}\n\n", crate::i18n::t("cmd.shaft.report.spline_sec_title")));
        md.push_str(&sp_section);
        md.push_str(&crate::i18n::t("cmd.shaft.report.spline_sec_note"));
    }
    Ok(md)
}

/// ANSI31 基准线间距（与 `partgen_kit::hatch_ansi31_edges` 的 offset 同源）。
const ANSI31_OFFSET: f64 = 2.245_064_030_267_288;

/// 预览专用：把每个 HATCH 换成其边界内的 45° 图案线（`5剖面线层`）。
fn preview_entities(entities: &[EntityType]) -> Vec<EntityType> {
    let mut out = Vec::with_capacity(entities.len());
    for entity in entities {
        match entity {
            EntityType::Hatch(h) => out.extend(hatch_preview_lines(h)),
            other => out.push(other.clone()),
        }
    }
    out
}

/// 把一个 HATCH 的每条边界环展平成线段（弧按 24 段采样），再用奇偶规则裁成 45° 线。
fn hatch_preview_lines(h: &Hatch) -> Vec<EntityType> {
    let mut out = Vec::new();
    let step = 2.0 * ANSI31_OFFSET * h.pattern_scale.max(0.05);
    for path in &h.paths {
        let mut segs: Vec<([f64; 2], [f64; 2])> = Vec::new();
        for edge in &path.edges {
            match edge {
                BoundaryEdge::Line(e) => {
                    segs.push(([e.start.x, e.start.y], [e.end.x, e.end.y]))
                }
                BoundaryEdge::CircularArc(a) => {
                    let tau = std::f64::consts::TAU;
                    let sweep = if a.counter_clockwise {
                        (a.end_angle - a.start_angle).rem_euclid(tau)
                    } else {
                        -((a.start_angle - a.end_angle).rem_euclid(tau))
                    };
                    let point = |angle: f64| {
                        [
                            a.center.x + a.radius * angle.cos(),
                            a.center.y + a.radius * angle.sin(),
                        ]
                    };
                    let n = 24;
                    let mut prev = point(a.start_angle);
                    for i in 1..=n {
                        let p = point(a.start_angle + sweep * i as f64 / n as f64);
                        segs.push((prev, p));
                        prev = p;
                    }
                }
                BoundaryEdge::Polyline(p) => {
                    let pts: Vec<[f64; 2]> = p.vertices.iter().map(|v| [v.x, v.y]).collect();
                    for i in 0..pts.len().saturating_sub(1) {
                        segs.push((pts[i], pts[i + 1]));
                    }
                    if p.is_closed && pts.len() > 2 {
                        segs.push((pts[pts.len() - 1], pts[0]));
                    }
                }
                _ => {}
            }
        }
        if segs.is_empty() {
            continue;
        }
        let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
        let (mut x1, mut y1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (p, q) in &segs {
            for pt in [p, q] {
                x0 = x0.min(pt[0]);
                x1 = x1.max(pt[0]);
                y0 = y0.min(pt[1]);
                y1 = y1.max(pt[1]);
            }
        }
        // 45° 线方向 (1,1)：`x − y = c`；相邻线间距（法向）= step / √2。
        let k0 = ((x0 - y1) / step).floor() as i64 - 1;
        let k1 = ((x1 - y0) / step).ceil() as i64 + 1;
        for k in k0..=k1 {
            let c = k as f64 * step;
            let mut ts: Vec<f64> = Vec::new();
            for &(p, q) in &segs {
                let (cp, cq) = (p[0] - p[1], q[0] - q[1]);
                if (cp <= c) == (cq <= c) || (cp - cq).abs() < 1e-12 {
                    continue;
                }
                let t = (c - cp) / (cq - cp);
                let ix = p[0] + (q[0] - p[0]) * t;
                let iy = p[1] + (q[1] - p[1]) * t;
                ts.push(ix + iy); // 沿 45° 方向的参数
            }
            ts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            for pair in ts.chunks_exact(2) {
                let (a, b) = (pair[0], pair[1]);
                let p = [(a + c) / 2.0, (a - c) / 2.0];
                let q = [(b + c) / 2.0, (b - c) / 2.0];
                out.push(line(p, q, LAYER_HATCH));
            }
        }
    }
    out
}

// ══════════════════════════════════════════════════════════════════════════
// 测试
// ══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detail;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    fn has_line(shaft: &Shaft, a: [f64; 2], b: [f64; 2]) -> bool {
        shaft.entities.iter().any(|e| match e {
            EntityType::Line(l) => {
                let (p, q) = ([l.start.x, l.start.y], [l.end.x, l.end.y]);
                (near(p[0], a[0]) && near(p[1], a[1]) && near(q[0], b[0]) && near(q[1], b[1]))
                    || (near(p[0], b[0]) && near(p[1], b[1]) && near(q[0], a[0]) && near(q[1], a[1]))
            }
            _ => false,
        })
    }

    fn has_arc(shaft: &Shaft, center: [f64; 2], radius: f64, start_deg: f64, end_deg: f64) -> bool {
        shaft.entities.iter().any(|e| match e {
            EntityType::Arc(a) => {
                near(a.center.x, center[0])
                    && near(a.center.y, center[1])
                    && near(a.radius, radius)
                    && near(normalize_deg(a.start_angle.to_degrees()), normalize_deg(start_deg))
                    && near(normalize_deg(a.end_angle.to_degrees()), normalize_deg(end_deg))
            }
            _ => false,
        })
    }

    fn normalize_deg(deg: f64) -> f64 {
        deg.rem_euclid(360.0)
    }

    /// 两点确定的 LINE 所在图层（找不到返回 None）。
    fn layer_of_line(shaft: &Shaft, a: [f64; 2], b: [f64; 2]) -> Option<&str> {
        shaft.entities.iter().find_map(|e| match e {
            EntityType::Line(l) => {
                let (p, q) = ([l.start.x, l.start.y], [l.end.x, l.end.y]);
                let hit = (near(p[0], a[0]) && near(p[1], a[1]) && near(q[0], b[0]) && near(q[1], b[1]))
                    || (near(p[0], b[0]) && near(p[1], b[1]) && near(q[0], a[0]) && near(q[1], a[1]));
                hit.then(|| l.common.layer.as_str())
            }
            _ => None,
        })
    }

    fn layer_of(entity: &EntityType) -> &str {
        entity.common().layer.as_str()
    }

    /// 图元序列 → CSV（列：entity,x1,y1,x2,y2,layer,cx,cy,r,a0,a1）。
    /// LINE 用两端点；ARC 用起终点 + 圆心/半径/角度；HATCH 用边界 bbox 一行。
    fn entities_csv(entities: &[EntityType]) -> String {
        let mut csv = String::from("entity,x1,y1,x2,y2,layer,cx,cy,r,a0,a1\n");
        for entity in entities {
            match entity {
                EntityType::Line(l) => csv.push_str(&format!(
                    "LINE,{:.6},{:.6},{:.6},{:.6},{},,,,,\n",
                    l.start.x, l.start.y, l.end.x, l.end.y, l.common.layer
                )),
                EntityType::Arc(a) => {
                    let sx = a.center.x + a.radius * a.start_angle.cos();
                    let sy = a.center.y + a.radius * a.start_angle.sin();
                    let ex = a.center.x + a.radius * a.end_angle.cos();
                    let ey = a.center.y + a.radius * a.end_angle.sin();
                    csv.push_str(&format!(
                        "ARC,{:.6},{:.6},{:.6},{:.6},{},{:.6},{:.6},{:.6},{:.6},{:.6}\n",
                        sx,
                        sy,
                        ex,
                        ey,
                        a.common.layer,
                        a.center.x,
                        a.center.y,
                        a.radius,
                        a.start_angle.to_degrees(),
                        a.end_angle.to_degrees()
                    ));
                }
                EntityType::Hatch(h) => {
                    let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
                    let (mut x1, mut y1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
                    for path in &h.paths {
                        for edge in &path.edges {
                            match edge {
                                BoundaryEdge::Line(e) => {
                                    for p in [e.start, e.end] {
                                        x0 = x0.min(p.x);
                                        x1 = x1.max(p.x);
                                        y0 = y0.min(p.y);
                                        y1 = y1.max(p.y);
                                    }
                                }
                                BoundaryEdge::CircularArc(e) => {
                                    x0 = x0.min(e.center.x - e.radius);
                                    x1 = x1.max(e.center.x + e.radius);
                                    y0 = y0.min(e.center.y - e.radius);
                                    y1 = y1.max(e.center.y + e.radius);
                                }
                                _ => {}
                            }
                        }
                    }
                    if !x0.is_finite() {
                        x0 = 0.0;
                        y0 = 0.0;
                        x1 = 0.0;
                        y1 = 0.0;
                    }
                    csv.push_str(&format!(
                        "HATCH,{:.6},{:.6},{:.6},{:.6},{},,,,,\n",
                        x0, y0, x1, y1, h.common.layer
                    ));
                }
                other => panic!("dump 只支持 LINE/ARC/HATCH，得到 {other:?}"),
            }
        }
        csv
    }

    const DEMO: &str = "\
S30 E30 L45 CH2@L
S40 E40 L30 CH2@R OV3
S50 E30 L20
S30 E30 L15 CH2@R
S40 E40 L30 M1.5 TL20
S36 E36 L5
GEAR M3 Z20";

    // ── 解析 ──────────────────────────────────────────────────────────────

    #[test]
    fn dsl_demo_parses() {
        let program = parse_program(DEMO).unwrap();
        assert_eq!(program.segments.len(), 7);
        assert_eq!(program.at, None);
        assert_eq!(program.rot, None);
        assert_eq!(
            (program.segments[0].s, program.segments[0].e, program.segments[0].l),
            (30.0, 30.0, 45.0)
        );
        assert_eq!(
            program.segments[0].ch,
            vec![Chamfer { c: 2.0, end: End::L }]
        );
        assert_eq!(
            program.segments[1].ov,
            vec![Overtravel { b1: Some(3.0), end: End::R }]
        );
        // E 省略 = 圆柱段
        assert_eq!((program.segments[2].s, program.segments[2].e), (50.0, 30.0));
        // M1.5 TL20：局部螺纹，螺距 P=1.5、完整螺纹长 20（第 5 段）
        assert_eq!(
            program.segments[4].thread,
            Some(Thread {
                pitch: Some(1.5),
                tl: Some(20.0),
                ..Thread::default()
            })
        );
        assert!(!program.segments[4].thread.unwrap().relief);
        // 退刀槽 = 一小段小直径轴段（第 6 段 Ø36）
        assert_eq!(program.segments[5].s, 36.0);
        assert_eq!(program.segments[5].thread, None);
        // GEAR M3 Z20：d = m·z = 60；H 省略 = 10m = 30
        let gear = program.segments[6].gear.expect("第 7 段是齿轮段");
        assert_eq!((gear.m, gear.z), (3.0, 20));
        assert_eq!(gear.h, None);
        assert!(near(program.segments[6].s, 60.0));
        assert!(near(program.segments[6].e, 60.0));
        assert!(near(program.segments[6].l, 30.0));
        // 关键字顺序无关 + 大小写不敏感
        let shuffled = parse_program("ch2@l l45 e30 s30").unwrap();
        assert_eq!(shuffled.segments[0], program.segments[0]);
        // OV 不带值 = 查表；OV@L 端别
        let ov = parse_program("S30 E30 L20 OV@L").unwrap();
        assert_eq!(
            ov.segments[0].ov,
            vec![Overtravel { b1: None, end: End::L }]
        );
        // 多段用 | 分隔 + 行尾放置
        let one_line =
            parse_program("S30 E30 L45 CH2@L | S40 E40 L30 OV at 100,50 rot 30").unwrap();
        assert_eq!(one_line.segments.len(), 2);
        assert_eq!(one_line.at, Some([100.0, 50.0]));
        assert_eq!(one_line.rot, Some(30.0));
    }

    #[test]
    fn dsl_errors_report_line_and_word() {
        // 第 2 行 OV b1=0
        let err = parse_program("S30 E30 L45\nS40 E40 L30 OV0").unwrap_err();
        assert!(err.contains("第 2 行"), "{err}");
        assert!(err.contains("OV") && err.contains("b1=0"), "{err}");
        // 尚未支持的关键字（THREAD / Z / H / KEYWAY）→ 明确报「不识别的关键字」
        // + 行号/词，不静默忽略
        for token in ["THREAD", "Z10", "H20", "KEYWAY"] {
            let text = format!("S30 E30 L45\nS40 E40 L30 {token}");
            let err = parse_program(&text).unwrap_err();
            assert!(err.contains("第 2 行"), "{token}: {err}");
            assert!(err.contains(token), "{token}: {err}");
            assert!(err.contains("不识别的关键字"), "{token}: {err}");
        }
        // ES 已取消：报错 + 指路（写法示例）
        for token in ["ES5*3", "ES", "ES2*1@L"] {
            let text = format!("S30 E30 L45\nS40 E40 L12 {token}");
            let err = parse_program(&text).unwrap_err();
            assert!(err.contains("第 2 行"), "{token}: {err}");
            assert!(err.contains("ES` 已取消"), "{token}: {err}");
            assert!(err.contains("S24 E24 L5"), "{token}: {err}");
        }
        // 同一行多段时标注「第 k 段」（GEAR 段缺 M）
        let err = parse_program("S30 E30 L10 | GEAR").unwrap_err();
        assert!(err.contains("第 1 行第 2 段") && err.contains("GEAR"), "{err}");
        // 注释行不影响物理行号
        let err = parse_program("# 注释\nS30 E30 L10\nOV0").unwrap_err();
        assert!(err.contains("第 3 行"), "{err}");
        // 其它语法错误
        assert!(parse_program("S30 E30").unwrap_err().contains("缺少 L"));
        assert!(parse_program("E30 L10").unwrap_err().contains("缺少 S"));
        assert!(parse_program("S30 E30 L10 CH2@X")
            .unwrap_err()
            .contains("端别"));
        assert!(parse_program("S30 E30 L10 S40")
            .unwrap_err()
            .contains("S 重复"));
        assert!(parse_program("S30 E30 L10 OV2 OV3")
            .unwrap_err()
            .contains("OV 在右端重复"));
        assert!(parse_program("S30 E30 L10 at x,1")
            .unwrap_err()
            .contains("at 坐标"));
        assert!(parse_program("S30 E30 L10 rot")
            .unwrap_err()
            .contains("rot"));
        // M 光杆现在是合法的螺纹段标记（不再报不识别的关键字）
        let m = parse_program("S30 E30 L20 M").unwrap();
        assert_eq!(m.segments[0].thread, Some(Thread { pitch: None, ..Thread::default() }));
    }

    #[test]
    fn json_model_matches_dsl() {
        let json = r#"{"segments":[
            {"s":30,"e":30,"l":45,"ch":[{"c":2,"end":"L"}]},
            {"s":40,"e":40,"l":30,"ch":{"c":2,"end":"R"},"ov":{"b1":3,"end":"R"}},
            {"s":50,"e":30,"l":20},
            {"s":30,"e":30,"l":15,"ch":[{"c":2,"end":"R"}]},
            {"s":40,"e":40,"l":30,"thread":{"p":1.5,"tl":20}},
            {"s":36,"e":36,"l":5},
            {"gear":{"m":3,"z":20}}
        ],"at":[10,20],"rot":90}"#;
        let program = parse_program(json).unwrap();
        assert_eq!(program.segments.len(), 7);
        assert_eq!(program.at, Some([10.0, 20.0]));
        assert_eq!(program.rot, Some(90.0));
        assert_eq!(program.segments, parse_program(DEMO).unwrap().segments);
        // `at` 也接受 {"x":…,"y":…}
        let program = parse_program(r#"{"segments":[{"s":30,"l":10}],"at":{"x":1,"y":2}}"#).unwrap();
        assert_eq!(program.at, Some([1.0, 2.0]));
        // `thread` 多种写法：true / 数字 / 对象 / 字符串
        for text in [
            r#"{"segments":[{"s":30,"l":10,"thread":true}]}"#,
            r#"{"segments":[{"s":30,"l":10,"thread":{"p":1.5}}]}"#,
            r#"{"segments":[{"s":30,"l":10,"thread":"M1.5"}]}"#,
        ] {
            let program = parse_program(text).unwrap_or_else(|e| panic!("{text}: {e}"));
            let want = if text.contains("true") {
                None
            } else {
                Some(1.5)
            };
            assert_eq!(program.segments[0].thread, Some(Thread { pitch: want, ..Thread::default() }));
        }
        // JSON 里写旧 `es` 字段 → 报取消 + 指路
        let err = parse_program(r#"{"segments":[{"s":30,"l":10,"es":{"b":5,"h":3}}]}"#)
            .unwrap_err();
        assert!(err.contains("`ES` 已取消") && err.contains("S24 E24 L5"), "{err}");
        assert!(parse_program("{not json").unwrap_err().contains("JSON"));
    }

    #[test]
    fn json_round_trip_and_block_name() {
        // 序列化 → 再解析等于原 program（GUI 的 parse/export 走同一条路）。
        let program = parse_program(DEMO).unwrap();
        let json = serde_json::to_string(&program).unwrap();
        assert!(json.contains("\"tl\":20"), "TL 序列化进 JSON：{json}");
        assert!(json.contains("\"pitch\":1.5"), "{json}");
        assert_eq!(parse_program(&json).unwrap(), program);
        // 只有螺距的旧形态仍序列化成数字（老 GUI/JSON 兼容）
        let plain = parse_program("S40 E40 L30 M1.5").unwrap();
        let plain_json = serde_json::to_string(&plain).unwrap();
        assert!(plain_json.contains("\"thread\":1.5"), "{plain_json}");
        assert_eq!(parse_program(&plain_json).unwrap(), plain);
        // 局部螺纹 JSON 对象往返（新关键字不丢）
        let rich = r#"{"segments":[{"s":40,"e":40,"l":30,"thread":{"p":1.5,"tl":20,"runout":"short","shoulder":"long"}},{"s":36,"e":36,"l":5}]}"#;
        let program = parse_program(rich).unwrap();
        let thread = program.segments[0].thread.unwrap();
        assert_eq!(thread.tl, Some(20.0));
        assert_eq!(thread.runout, RunoutGrade::Short);
        assert_eq!(thread.shoulder, ShoulderGrade::Long);
        assert_eq!(parse_program(&serde_json::to_string(&program).unwrap()).unwrap(), program);
        // 光杆 M 序列化成 true；再解析回来
        let program = parse_program("S30 E30 L20 M").unwrap();
        let json = serde_json::to_string(&program).unwrap();
        assert!(json.contains("\"thread\":true"), "{json}");
        assert_eq!(parse_program(&json).unwrap(), program);
        // 齿轮段的派生 s/e/l 允许回传（= 派生值）；不一致就报错
        let program = parse_program("GEAR M3 Z20").unwrap();
        assert_eq!(parse_program(&serde_json::to_string(&program).unwrap()).unwrap(), program);
        let bad = r#"{"segments":[{"s":999,"gear":{"m":3,"z":20}}]}"#;
        assert!(parse_program(bad).unwrap_err().contains("应等于分度圆"));
        let bad = r#"{"segments":[{"l":999,"gear":{"m":3,"z":20}}]}"#;
        assert!(parse_program(bad).unwrap_err().contains("应等于齿宽"));
        // 块名：只由几何决定（同模型稳定、不同模型不同）
        let a = parse_program("S30 E30 L20 M").unwrap();
        let b = parse_program("s30 l20 e30 m").unwrap();
        let c = parse_program("S30 E30 L21 M").unwrap();
        assert_eq!(block_name(&a), block_name(&b));
        assert_ne!(block_name(&a), block_name(&c));
        assert!(block_name(&a).starts_with("OCSM_SHAFT_"));
    }

    #[test]
    fn preview_svg_renders_and_reports_errors() {
        let svg = preview_svg(&parse_program(DEMO).unwrap()).unwrap();
        assert!(svg.starts_with("<svg") || svg.contains("<svg"), "{svg}");
        assert!(svg.contains("#5aa0ff"), "细线层颜色（螺纹小径/螺尾）");
        assert!(svg.contains("#ff5555"), "中心线层颜色");
        // 剖视：HATCH 在预览里被裁成真实边界内的 45° 线（剖面线层绿色）
        let section = preview_svg(&parse_program("S30 E30 L20 VIEW 剖视").unwrap()).unwrap();
        assert!(section.contains("#3fa13f"), "剖面线颜色");
        // 几何非法同样带「第 N 段」原因
        let err = preview_svg(&parse_program("S30 E40 L10 M").unwrap()).unwrap_err();
        assert!(err.contains("第 1 段") && err.contains("圆柱"), "{err}");
    }

    // ── 段拼接 ────────────────────────────────────────────────────────────

    #[test]
    fn segments_concatenate_total_diameter_and_cone() {
        let program = parse_program("S30 E30 L45\nS40 E30 L20\nS30 L15").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        assert!(near(shaft.total_length, 80.0));
        assert!(near(shaft.max_diameter, 40.0));
        assert_eq!(shaft.segment_count, 3);
        // 圆柱段：x=0..45 的水平线 y=±15
        assert!(has_line(&shaft, [0.0, 15.0], [45.0, 15.0]));
        // 圆锥段：x=45..65，Ø40 → Ø30（半径 20 → 15）
        assert!(has_line(&shaft, [45.0, 20.0], [65.0, 15.0]));
        assert!(has_line(&shaft, [45.0, -20.0], [65.0, -15.0]));
        // 台阶面 x=45：15 ↔ 20（上下两条）
        assert!(has_line(&shaft, [45.0, 15.0], [45.0, 20.0]));
        assert!(has_line(&shaft, [45.0, -15.0], [45.0, -20.0]));
        // 圆锥末端 Ø30 与第 3 段 Ø30 相同 → 没有肩面，但段边界改用贯通竖线：x=65 ±15
        assert!(has_line(&shaft, [65.0, -15.0], [65.0, 15.0]));
        assert!(has_line(&shaft, [65.0, 15.0], [80.0, 15.0]));
        assert!(has_line(&shaft, [80.0, -15.0], [80.0, 15.0]));
        // 轴两端外端面
        assert!(has_line(&shaft, [0.0, -15.0], [0.0, 15.0]));
        // 轴线长度 = 总长 + 6n
        assert!(has_line(&shaft, [-3.0, 0.0], [83.0, 0.0]));
    }

    #[test]
    fn chamfer_on_both_ends_of_a_segment() {
        let program = parse_program("S30 E30 L45 CH2@L CH2@R").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        // 左端倒角：端面缩到 ±13，45° 线 (0,13)→(2,15)
        assert!(has_line(&shaft, [0.0, -13.0], [0.0, 13.0]));
        assert!(has_line(&shaft, [0.0, 13.0], [2.0, 15.0]));
        assert!(has_line(&shaft, [0.0, -13.0], [2.0, -15.0]));
        // 右端倒角：端面缩到 ±13，45° 线 (43,15)→(45,13)
        assert!(has_line(&shaft, [45.0, -13.0], [45.0, 13.0]));
        assert!(has_line(&shaft, [43.0, 15.0], [45.0, 13.0]));
        assert!(has_line(&shaft, [43.0, -15.0], [45.0, -13.0]));
        // 轮廓缩短到 2..43
        assert!(has_line(&shaft, [2.0, 15.0], [43.0, 15.0]));
        // 倒角终点（根）贯通竖线（用户 2026-09-18 更正版）：x=2 / x=43 半高 = 15
        assert!(has_line(&shaft, [2.0, -15.0], [2.0, 15.0]));
        assert!(has_line(&shaft, [43.0, -15.0], [43.0, 15.0]));
    }

    #[test]
    fn chamfer_on_a_shoulder_lands_on_the_convex_corner() {
        // 第 1 段 Ø30 → 第 2 段 Ø40；CH2@R 落在第 2 段（大）的左端
        let program = parse_program("S30 E30 L20 CH2@R\nS40 E40 L10").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        // 端面 x=20 从 15 到 18（20−2）；倒角线 (20,18)→(22,20)
        assert!(has_line(&shaft, [20.0, 15.0], [20.0, 18.0]));
        assert!(has_line(&shaft, [20.0, 18.0], [22.0, 20.0]));
        // 段边界贯通竖线：x=20、半高 = min(15, 18) = 15
        assert!(has_line(&shaft, [20.0, -15.0], [20.0, 15.0]));
        // 倒角终点（根）贯通竖线：x=22、半高 = 倒角根半径 20
        assert!(has_line(&shaft, [22.0, -20.0], [22.0, 20.0]));
        // 第 2 段轮廓从 x=22 开始
        assert!(has_line(&shaft, [22.0, 20.0], [30.0, 20.0]));
        // 等直径分界也画贯通竖线（用户 2026-09-18 更正版：x=88 那种同径分界也有）
        let flat = build(&parse_program("S30 E30 L20\nS30 E30 L10").unwrap(), 1.0).unwrap();
        assert!(has_line(&flat, [20.0, -15.0], [20.0, 15.0]));
    }

    // ── 越程槽 ────────────────────────────────────────────────────────────

    #[test]
    fn overtravel_groove_keeps_detail_geometry_on_both_ends() {
        // 右端槽：第 1 段 Ø50（地面）→ 第 2 段 Ø60（台阶）；OV 默认右端 + 查表
        let program = parse_program("S50 E50 L20 OV\nS60 E60 L10").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        let row = detail::row_for(50.0, None).unwrap();
        let (b1, h, r) = (row.b1, row.h, row.r);
        let fillet = 25.0 + r - h;
        // 右端面 x=20：槽向下镜像，圆角心 (20−r, fillet)，角度 270→360
        assert!(has_arc(&shaft, [20.0 - r, fillet], r, 270.0, 360.0));
        assert!(has_arc(&shaft, [20.0 - r, -fillet], r, 0.0, 90.0));
        // 槽底 + 45° 斜坡
        assert!(has_line(&shaft, [20.0 - b1 + h, 25.0 - h], [20.0 - r, 25.0 - h]));
        assert!(has_line(&shaft, [20.0 - b1, 25.0], [20.0 - b1 + h, 25.0 - h]));
        // 端面线从圆角切点画到台阶顶
        assert!(has_line(&shaft, [20.0, fillet], [20.0, 30.0]));
        // 轮廓从 x=0 到槽的斜坡外侧（20−b1），不是到端面
        assert!(has_line(&shaft, [0.0, 25.0], [20.0 - b1, 25.0]));
        // 槽的三条贯通竖线（用户 2026-09-18 更正版）：槽肩（也是段边界线）/
        // 槽底终止 / 斜壁终点，全落 1轮廓实线层。
        assert_eq!(
            layer_of_line(&shaft, [20.0, -fillet], [20.0, fillet]),
            Some(LAYER_MAIN),
            "槽肩贯通竖线"
        );
        assert_eq!(
            layer_of_line(
                &shaft,
                [20.0 - b1 + h, -(25.0 - h)],
                [20.0 - b1 + h, 25.0 - h]
            ),
            Some(LAYER_MAIN),
            "槽底终止贯通竖线"
        );
        assert_eq!(
            layer_of_line(&shaft, [20.0 - b1, -25.0], [20.0 - b1, 25.0]),
            Some(LAYER_MAIN),
            "斜壁终点贯通竖线"
        );
        // 本例无 M 段 → 整图 0 条 2细线层（只针对本用例；带螺纹的轴会有小径/螺尾线，
        // 不是「OV 一概 0 条」的通用断言）。
        let thin = shaft.entities.iter().filter(|e| layer_of(e) == LAYER_THIN).count();
        assert_eq!(thin, 0, "本用例无 M 段，2细线层 应为 0 条（非通用断言）");
        // 旧砂轮细线的 45° 尾线（模板尾长 15）不应再出现
        assert!(!has_line(
            &shaft,
            [20.0 - b1, 25.0],
            [20.0 - b1 - 15.0, 25.0 + 15.0]
        ));
        // OV@L：地面 = 第 2 段，台阶 = 第 1 段
        let program = parse_program("S60 E60 L10\nS50 E50 L20 OV@L").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        let row = detail::row_for(50.0, None).unwrap();
        let (b1, h, r) = (row.b1, row.h, row.r);
        let fillet = 25.0 + r - h;
        assert!(has_arc(&shaft, [10.0 + r, fillet], r, 180.0, 270.0));
        assert!(has_line(&shaft, [10.0, fillet], [10.0, 30.0]));
        assert!(has_line(&shaft, [10.0 + b1, 25.0], [30.0, 25.0]));
        // 镜像到右端的三条贯通竖线 + 没有砂轮细线
        assert!(has_line(&shaft, [10.0, -fillet], [10.0, fillet]));
        assert!(has_line(
            &shaft,
            [10.0 + b1 - h, -(25.0 - h)],
            [10.0 + b1 - h, 25.0 - h]
        ));
        assert!(has_line(&shaft, [10.0 + b1, -25.0], [10.0 + b1, 25.0]));
        assert!(!has_line(
            &shaft,
            [10.0 + b1, 25.0],
            [10.0 + b1 + 15.0, 25.0 + 15.0]
        ));
        let thin = shaft.entities.iter().filter(|e| layer_of(e) == LAYER_THIN).count();
        assert_eq!(thin, 0, "本用例无 M 段，OV@L 也 0 条（非通用断言）");
    }

    // ── 螺纹段 M ─────────────────────────────────────────────────────────

    #[test]
    fn dsl_thread_parses_and_reports_errors() {
        // 光杆 M = 简化画法；M1.5 / M=1.5 = 螺距 P
        let program = parse_program("S30 E30 L20 M").unwrap();
        assert_eq!(program.segments[0].thread, Some(Thread { pitch: None, ..Thread::default() }));
        let program = parse_program("S36 E36 L5 M1.5").unwrap();
        assert_eq!(
            program.segments[0].thread,
            Some(Thread { pitch: Some(1.5), ..Thread::default() })
        );
        let program = parse_program("S36 E36 L5 M=1.5").unwrap();
        assert_eq!(
            program.segments[0].thread,
            Some(Thread { pitch: Some(1.5), ..Thread::default() })
        );
        // 可与 CH 同段（螺纹端倒角是常规画法）、段内顺序无关
        let program = parse_program("S30 E30 L20 CH2@R M1.5").unwrap();
        assert_eq!(program.segments[0].ch, vec![Chamfer { c: 2.0, end: End::R }]);
        assert_eq!(
            program.segments[0].thread,
            Some(Thread { pitch: Some(1.5), ..Thread::default() })
        );
        // 解析错误（行号 + 原因）
        let cases: &[(&str, &str)] = &[
            ("S30 E30 L20 M0", "螺距 P=0"),
            ("S30 E30 L20 M-1.5", "螺距 P=-1.5"),
            ("S30 E30 L20 M1.5 M2", "M 重复"),
            ("S30 E30 L20 M OV3", "不能与越程槽"),
            ("GEAR M3 Z20 M1.5", "M 重复"),
            ("M1.5 GEAR M3 Z20", "M 重复"),
        ];
        for (text, needle) in cases {
            let err = parse_program(text).unwrap_err();
            assert!(err.contains(needle), "{text}: 期望含「{needle}」，得到 {err}");
        }
    }

    #[test]
    fn thread_geometry_minor_lines_and_termination() {
        // 自由端 M1.5：d1 = 30 − 1.0825×1.5 = 28.37625，半径 14.188125
        let program = parse_program("S30 E30 L20 M1.5").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        let r1 = (30.0 - 1.0825 * 1.5) / 2.0;
        assert!(near(r1, 14.188125));
        // 大径轮廓还是 ±15；小径细实线贯穿该段，落 2细线层
        assert!(has_line(&shaft, [0.0, 15.0], [20.0, 15.0]));
        assert!(has_line(&shaft, [0.0, r1], [20.0, r1]));
        assert!(has_line(&shaft, [0.0, -r1], [20.0, -r1]));
        let thin: Vec<&str> = shaft
            .entities
            .iter()
            .filter(|e| layer_of(e) == LAYER_THIN)
            .map(layer_of)
            .collect();
        assert_eq!(thin.len(), 2, "小径细实线上下各一条");
        // 小径细实线止于段末终止线（此处 = 自由端端面）
        assert!(has_line(&shaft, [20.0, -15.0], [20.0, 15.0]));
        // 光杆 M：0.85d = 25.5，半径 12.75
        let shaft = build(&parse_program("S30 E30 L20 M").unwrap(), 1.0).unwrap();
        assert!(has_line(&shaft, [0.0, 12.75], [20.0, 12.75]));
        // 螺纹段 + 小直径退刀槽段：终止线 = 既有台阶面（18 ↔ 20）
        let program = parse_program("S40 E40 L7 M1.5\nS36 E36 L5").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        let r1 = (40.0 - 1.0825 * 1.5) / 2.0;
        assert!(near(r1, 19.188125));
        assert!(has_line(&shaft, [0.0, r1], [7.0, r1]));
        assert!(
            has_line(&shaft, [7.0, 18.0], [7.0, 20.0]),
            "终止线（既有台阶面）从退刀槽底 18 到螺纹大径 20"
        );
        // 同段端面有深倒角（伸进小径）→ 细实线止于倒角斜线交点，不挑出材料外
        let program = parse_program("S30 E30 L20 M1.5 CH2@R").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        let r1 = (30.0 - 1.0825 * 1.5) / 2.0;
        let cut = 15.0 - r1;
        assert!(has_line(&shaft, [0.0, r1], [20.0 - (2.0 - cut), r1]));
        assert!(has_line(&shaft, [18.0, 15.0], [20.0, 13.0]), "倒角线还在");
        // 浅倒角（切不到小径）→ 细实线仍到端面
        let program = parse_program("S30 E30 L20 M5 CH2@R").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        let r1 = (30.0 - 1.0825 * 5.0) / 2.0; // 12.29375；cut = 2.70625 > c=2
        assert!(has_line(&shaft, [0.0, r1], [20.0, r1]));
    }

    #[test]
    fn thread_validity_errors_point_at_the_segment() {
        let cases: &[(&str, &str)] = &[
            ("S30 E40 L20 M1.5", "必须是圆柱"),
            ("S30 E30 L20 M30", "太大"),
        ];
        for (text, needle) in cases {
            let program = parse_program(text).unwrap_or_else(|e| panic!("{text}: 解析失败 {e}"));
            let err = build(&program, 1.0).unwrap_err();
            assert!(err.contains(needle), "{text}: 期望含「{needle}」，得到 {err}");
        }
        // 手搓模型（绕过 parser）：M 与 OV 同段也不能通过 validate
        let segment = Segment {
            s: 30.0,
            e: 30.0,
            l: 20.0,
            ch: Vec::new(),
            ov: vec![Overtravel { b1: Some(3.0), end: End::R }],
            relief: Vec::new(),
            thread: Some(Thread { pitch: Some(1.5), ..Thread::default() }),
            gear: None,
            spline: None,
            keyway: None,
        };
        let err = validate(&Program {
            segments: vec![segment],
            at: None,
            rot: None,
            view: ShaftView::Normal,

        })
        .unwrap_err();
        assert!(err.contains("不能与越程槽"), "{err}");
    }

    // ── 局部螺纹 M + TL/RO/SD/RL（GB/T 3-1997 图 1 / 表 1 / 表 2） ────────

    #[test]
    fn thread_tables_lookup_and_pitch_errors() {
        // **表归一**：表 1/表 2 只在 `detail.rs` 一份；本测试验证同一份数据在
        // 「直接取表（Option）」与「轴生成器的报错口径（Result）」两条路上一致。
        // 表 1 抽档（P=0.2 起；含订正的 P=1.75 x一般=4.3）
        let row = detail::runout_row_checked(1.5).unwrap();
        assert!(near(row.x_normal, 3.8) && near(row.x_short, 1.9));
        assert!(near(row.a_normal, 4.5) && near(row.a_long, 6.0) && near(row.a_short, 3.0));
        let row = detail::runout_row_checked(1.75).unwrap();
        assert!(near(row.x_normal, 4.3), "P=1.75 x一般 = 4.3（订正网页笔误 1.3）");
        let row = detail::runout_row_checked(0.2).unwrap();
        assert!(near(row.x_normal, 0.5) && near(row.a_normal, 0.6));
        let row = detail::runout_row_checked(6.0).unwrap();
        assert!(near(row.x_normal, 15.0) && near(row.a_long, 24.0));
        // 表 2 抽档（无 P=0.2）
        let row = detail::thread_relief_row(1.5).unwrap();
        assert!(
            near(row.g1, 2.5)
                && near(row.g2, 4.5)
                && near(row.dg_reduction, 2.3)
                && near(row.r, 0.8)
        );
        let row = detail::thread_relief_row(0.25).unwrap();
        assert!(
            near(row.g1, 0.4)
                && near(row.g2, 0.75)
                && near(row.dg_reduction, 0.4)
                && near(row.r, 0.12)
        );
        let row = detail::thread_relief_row(6.0).unwrap();
        assert!(
            near(row.g1, 11.0)
                && near(row.g2, 18.0)
                && near(row.dg_reduction, 8.3)
                && near(row.r, 3.2)
        );
        // P 不在表里：表 1（1.6）；表 2（0.2 表 2 没有）
        let err = detail::runout_row_checked(1.6).unwrap_err();
        assert!(err.contains("表 1") && err.contains("P=1.6"), "{err}");
        let err = detail::thread_relief_row(0.2).unwrap_err();
        assert!(
            err.contains("表 2") && err.contains("0.2") && err.contains("0.25"),
            "{err}"
        );
        // 表归一后两条取表路径必须回同一行（表 1 Option ↔ Result；表 2 一一对应）
        for row in detail::RUNOUT_ROWS {
            let again = detail::runout_row_checked(row.p).unwrap();
            assert_eq!(again, row, "P={} 表 1 两条路取表不一致", trim(row.p));
            assert_eq!(detail::runout_row(row.p), Some(row));
        }
        for row in detail::THREAD_RELIEF_ROWS {
            let again = detail::thread_relief_row(row.p).unwrap();
            assert_eq!(again, row, "P={} 表 2 取表不一致", trim(row.p));
            // 表行的 dims（）与段级显式尺寸路径回同一几何（dg 绝对值口径）
            let dims = row.dims(40.0);
            let explicit = detail::relief_dims_explicit(
                40.0,
                row.g1,
                row.g2,
                40.0 - row.dg_reduction,
                row.r,
            )
            .unwrap();
            assert_eq!(dims.dg, explicit.dg);
            assert_eq!(dims.g1, explicit.g1);
            assert_eq!(dims.g2, explicit.g2);
            assert_eq!(dims.r, explicit.r);
        }
        // 斜壁 30° 自检：任务书的 `g2 = g1 + ((d−dg)/2)/tan30°` 恒等式只有
        // g1/g2 同步取整时才精确；表 2 各自圆整后反算角 28.30°…33.69°，
        // 故按偏离 30° ≤4° 立断言（具体偏差见报告）。
        let mut worst = 0.0_f64;
        for row in detail::THREAD_RELIEF_ROWS {
            let angle = ((row.dg_reduction / 2.0) / (row.g2 - row.g1)).atan().to_degrees();
            worst = worst.max((angle - 30.0).abs());
            assert!(
                (angle - 30.0).abs() <= 4.0,
                "P={} 反算斜壁角 {angle:.2}° 偏离 30° 超过 4°",
                trim(row.p)
            );
        }
        assert!(worst > 0.0, "表值独立取整会带来非零偏差");
    }

    #[test]
    fn dsl_local_thread_parses_with_grades_and_errors() {
        // 基本：M1.5 TL20
        let program = parse_program("S40 E40 L30 M1.5 TL20").unwrap();
        let thread = program.segments[0].thread.unwrap();
        assert_eq!(thread.pitch, Some(1.5));
        assert_eq!(thread.tl, Some(20.0));
        assert_eq!(thread.runout, RunoutGrade::Normal);
        assert_eq!(thread.shoulder, ShoulderGrade::Normal);
        assert!(!thread.relief);
        // 段内顺序无关 + 档位 + `=` 写法
        let program = parse_program("S40 E40 L30 RO短 TL20 M1.5 SD长").unwrap();
        let thread = program.segments[0].thread.unwrap();
        assert_eq!(thread.tl, Some(20.0));
        assert_eq!(thread.runout, RunoutGrade::Short);
        assert_eq!(thread.shoulder, ShoulderGrade::Long);
        // RL；英文档位 / RO 光杆 = 一般
        let program = parse_program("S40 E40 L30 M1.5 TL20 RL").unwrap();
        assert!(program.segments[0].thread.unwrap().relief);
        let program = parse_program("S40 E40 L30 M1.5 TL20 ROshort SDnormal").unwrap();
        let thread = program.segments[0].thread.unwrap();
        assert_eq!(thread.runout, RunoutGrade::Short);
        assert_eq!(thread.shoulder, ShoulderGrade::Normal);
        let program = parse_program("S40 E40 L30 M1.5 TL20 RO SD:短").unwrap();
        let thread = program.segments[0].thread.unwrap();
        assert_eq!(thread.runout, RunoutGrade::Normal);
        assert_eq!(thread.shoulder, ShoulderGrade::Short);
        // 错误：非法档位 / 缺值 / 重复 / 互斥 / 没 M / 齿轮段 / RL 带值 / rot 不吞
        let cases: &[(&str, &str)] = &[
            ("S40 E40 L30 M1.5 TL20 RO长", "RO"),
            ("S40 E40 L30 M1.5 TL20 SD超", "SD"),
            ("S40 E40 L30 M1.5 TL", "TL"),
            ("S40 E40 L30 M1.5 TL0", "TL=0"),
            ("S40 E40 L30 M1.5 TL20 TL15", "TL 重复"),
            ("S40 E40 L30 M1.5 TL20 RO短 RO一般", "RO 重复"),
            ("S40 E40 L30 M1.5 TL20 SD长 SD短", "SD 重复"),
            ("S40 E40 L30 M1.5 TL20 RL RL", "RL 重复"),
            ("S40 E40 L30 M1.5 TL20 RL RO短", "互斥"),
            ("S40 E40 L30 M1.5 TL20 RL SD长", "互斥"),
            ("S40 E40 L30 M1.5 TL20 RL=1", "不带值"),
            ("S40 E40 L30 TL20", "需要与 M 同段"),
            ("S40 E40 L30 RO短", "需要与 M 同段"),
            ("S40 E40 L30 M1.5 TL20 RLx", "不识别的关键字"),
            ("S40 E40 L30 M1.5 TL20 SD=2", "非法"),
            ("GEAR M3 Z20 TL10", "齿轮段"),
            ("S40 E40 L30 M1.5 TL20 rot", "rot"),
        ];
        for (text, needle) in cases {
            let err = parse_program(text).unwrap_err();
            assert!(err.contains(needle), "{text}: 期望含「{needle}」，得到 {err}");
        }
    }

    #[test]
    fn thread_local_runout_geometry_and_layers() {
        // P=1.5 默认档：x=3.8 / a=4.5；d1 = 40 − 1.0825×1.5 = 38.37625（r1=19.188125）
        // 布局：face=30、boundary=25.5、runout_start=29.3、full_start=5.5
        let program = parse_program("S40 E40 L30 M1.5 TL20\nS36 E36 L5").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        let r1 = (40.0 - 1.0825 * 1.5) / 2.0;
        // 大径轮廓照旧贯穿整段（图 1 口径：大径线不断）
        assert!(has_line(&shaft, [0.0, 20.0], [30.0, 20.0]));
        // 锥面过渡（粗实线）：(30,20) → (29.3,r1)
        assert_eq!(
            layer_of_line(&shaft, [30.0, 20.0], [29.3, r1]),
            Some(LAYER_MAIN),
            "锥面 → 1轮廓实线层"
        );
        // 螺尾（细实线）：(29.3,r1) → (25.5,20)，上下对称
        assert_eq!(
            layer_of_line(&shaft, [29.3, r1], [25.5, 20.0]),
            Some(LAYER_THIN),
            "螺尾 → 2细线层"
        );
        assert!(has_line(&shaft, [29.3, -r1], [25.5, -20.0]));
        // 完整螺纹 / 螺尾分界竖线（粗实线，全高）
        assert_eq!(
            layer_of_line(&shaft, [25.5, -20.0], [25.5, 20.0]),
            Some(LAYER_MAIN),
            "分界竖线 → 1轮廓实线层"
        );
        // 小径细实线：5.5 → 29.3（止于螺尾起点）
        assert!(has_line(&shaft, [5.5, r1], [29.3, r1]));
        assert!(has_line(&shaft, [5.5, -r1], [29.3, -r1]));
        let thin = shaft.entities.iter().filter(|e| layer_of(e) == LAYER_THIN).count();
        assert_eq!(thin, 4, "螺尾上下 2 + 小径上下 2");
        // 台肩面仍在 30：18 ↔ 20
        assert!(has_line(&shaft, [30.0, 18.0], [30.0, 20.0]));
        // 档位效果：RO短 x=1.9、SD长 a=6 → 锥面 30→25.9、螺尾 25.9→24、细实线 4→25.9
        let program = parse_program("S40 E40 L30 M1.5 TL20 RO短 SD长\nS36 E36 L5").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        assert!(has_line(&shaft, [30.0, 20.0], [25.9, r1]));
        assert!(has_line(&shaft, [25.9, r1], [24.0, 20.0]));
        assert!(has_line(&shaft, [4.0, r1], [25.9, r1]));
        // RO短 + SD短：x=1.9、a=3 → 锥面长 a−x=1.1（30→28.9）
        let program = parse_program("S40 E40 L30 M1.5 TL20 RO短 SD短\nS36 E36 L5").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        assert!(has_line(&shaft, [30.0, 20.0], [28.9, r1]));
        assert!(has_line(&shaft, [28.9, r1], [27.0, 20.0]));
        assert!(has_line(&shaft, [7.0, r1], [28.9, r1]));
        // 不给 TL = 旧口径回归：整段小径细实线、没有锥面/螺尾/分界线
        let program = parse_program("S40 E40 L30 M1.5\nS36 E36 L5").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        assert!(has_line(&shaft, [0.0, r1], [30.0, r1]));
        assert!(has_line(&shaft, [0.0, -r1], [30.0, -r1]));
        assert!(!has_line(&shaft, [30.0, 20.0], [29.3, r1]), "旧口径没有锥面");
        let thin = shaft.entities.iter().filter(|e| layer_of(e) == LAYER_THIN).count();
        assert_eq!(thin, 2, "旧口径只有小径细实线上下各一条");
    }

    #[test]
    fn thread_local_relief_geometry_and_mutual_exclusion() {
        // P=1.5 表 2：g1=2.5 / g2=4.5 / 减量=2.3 / r=0.8；d=40 → dg=37.7、rg=18.85；
        // 台肩圆角切点 = 18.85+0.8 = 19.65；face=30、完整螺纹 5.5→25.5
        let program = parse_program("S40 E40 L30 M1.5 TL20 RL\nS36 E36 L5").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        let r1 = (40.0 - 1.0825 * 1.5) / 2.0;
        // 大径轮廓止于 g2 处：0 → 25.5
        assert!(has_line(&shaft, [0.0, 20.0], [25.5, 20.0]));
        // R 圆角（粗实线）：心 (29.2,19.65)、270°→360°
        assert!(has_arc(&shaft, [29.2, 19.65], 0.8, 270.0, 360.0));
        assert!(has_arc(&shaft, [29.2, -19.65], 0.8, 0.0, 90.0));
        // 槽底 g1：29.2 → 27.5（y=18.85）；斜壁：27.5,18.85 → 25.5,20
        assert!(has_line(&shaft, [29.2, 18.85], [27.5, 18.85]));
        assert!(has_line(&shaft, [27.5, 18.85], [25.5, 20.0]));
        assert!(has_line(&shaft, [27.5, -18.85], [25.5, -20.0]));
        // 台肩面只画到圆角切点：18 → 19.65（不像螺尾那里画满 20）
        assert!(has_line(&shaft, [30.0, 18.0], [30.0, 19.65]));
        assert!(!has_line(&shaft, [30.0, 18.0], [30.0, 20.0]), "RL 槽根不留到 20");
        // 三条贯通竖线：槽肩（切线高 19.65）/ 槽底终止（27.5）/ 斜壁终点（25.5）
        assert!(has_line(&shaft, [30.0, -19.65], [30.0, 19.65]), "槽肩贯通竖线");
        assert!(has_line(&shaft, [27.5, -18.85], [27.5, 18.85]), "槽底终止贯通竖线");
        assert!(has_line(&shaft, [25.5, -20.0], [25.5, 20.0]), "斜壁终点贯通竖线");
        // 小径细实线止于斜壁交点：5.5 → 26.911957…（y=r1）
        let x_minor = 30.0 - 2.5 - 2.0 * (r1 - 18.85) / (20.0 - 18.85);
        assert!(has_line(&shaft, [5.5, r1], [x_minor, r1]));
        assert!(has_line(&shaft, [5.5, -r1], [x_minor, -r1]));
        // RL 不画收尾/锥面/分界竖线：细实线只有小径上下两条
        assert!(!has_line(&shaft, [29.3, r1], [25.5, 20.0]), "RL 不画螺尾斜线");
        let thin = shaft.entities.iter().filter(|e| layer_of(e) == LAYER_THIN).count();
        assert_eq!(thin, 2, "RL 只有小径细实线上下各一条");
        // RL 不给 TL 也支持：整段圆柱上直接开槽，小径细实线从左端到斜壁交点
        let program = parse_program("S40 E40 L30 M1.5 RL\nS36 E36 L5").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        assert!(has_line(&shaft, [0.0, 20.0], [25.5, 20.0]));
        assert!(has_line(&shaft, [0.0, r1], [x_minor, r1]));
        assert!(has_line(&shaft, [0.0, -r1], [x_minor, -r1]));
        // 段长吃不下：g2 + TL = 24.5 > 14
        let program = parse_program("S40 E40 L14 M1.5 TL20 RL\nS36 E36 L5").unwrap();
        let err = build(&program, 1.0).unwrap_err();
        assert!(err.contains("第 1 段") && err.contains("超过段长"), "{err}");
        // 互斥（JSON 路径也要拦）
        let bad = r#"{"segments":[{"s":40,"l":30,"thread":{"p":1.5,"tl":20,"relief":true,"runout":"short"}}]}"#;
        let program = parse_program(bad).unwrap();
        let err = build(&program, 1.0).unwrap_err();
        assert!(err.contains("第 1 段") && err.contains("互斥"), "{err}");
        // 表 2 没有 P=0.2
        let program = parse_program("S40 E40 L30 M0.2 TL20 RL\nS36 E36 L5").unwrap();
        let err = build(&program, 1.0).unwrap_err();
        assert!(err.contains("第 1 段") && err.contains("表 2"), "{err}");
        // 剖视：RL 槽体也进剖面环（一个 HATCH、简单环、边界首尾相接）
        let program =
            parse_program("S40 E40 L30 M1.5 TL20 RL\nS36 E36 L5 VIEW 剖视").unwrap();
        let section = build(&program, 1.0).unwrap();
        let hatch = section
            .entities
            .iter()
            .find_map(|e| match e {
                EntityType::Hatch(h) => Some(h),
                _ => None,
            })
            .expect("剖视应有一个 HATCH");
        assert_eq!(hatch.paths.len(), 2, "轴线上/下两个环（参考件口径）");
        let pts = |edge: &BoundaryEdge| -> ([f64; 2], [f64; 2]) {
            match edge {
                BoundaryEdge::Line(e) => ([e.start.x, e.start.y], [e.end.x, e.end.y]),
                BoundaryEdge::CircularArc(a) => (
                    [
                        a.center.x + a.radius * a.start_angle.cos(),
                        a.center.y + a.radius * a.start_angle.sin(),
                    ],
                    [
                        a.center.x + a.radius * a.end_angle.cos(),
                        a.center.y + a.radius * a.end_angle.sin(),
                    ],
                ),
                other => panic!("剖面环只应有 LINE/ARC，得到 {other:?}"),
            }
        };
        for path in &hatch.paths {
            let mut prev: Option<[f64; 2]> = None;
            for edge in &path.edges {
                let (a, b) = pts(edge);
                if let Some(p) = prev {
                    assert!(
                        near(p[0], a[0]) && near(p[1], a[1]),
                        "剖面环断开：{p:?} → {a:?}"
                    );
                }
                prev = Some(b);
            }
            let last = prev.unwrap();
            assert!(
                near(last[0], 0.0) && near(last[1].abs(), 20.0),
                "环回到左端面 (0,±20)：{last:?}"
            );
        }
    }

    #[test]
    fn thread_local_fit_errors_point_at_segment() {
        let cases: &[(&str, &str)] = &[
            // a + TL = 24.5 > L=20
            ("S40 E40 L20 M1.5 TL20\nS36 E36 L5", "超过段长"),
            // x=3.8 > a=3.0（RO一般 + SD短）
            (
                "S40 E40 L30 M1.5 TL20 RO一般 SD短\nS36 E36 L5",
                "大于肩距",
            ),
            // 没给螺距
            ("S40 E40 L30 M TL20\nS36 E36 L5", "必须给螺距"),
            // P 不在表 1
            ("S40 E40 L30 M1.6 TL20\nS36 E36 L5", "表 1"),
            // RO/SD 没 TL
            ("S40 E40 L30 M1.5 RO短\nS36 E36 L5", "只在局部螺纹"),
            // 本段右端倒角与 TL 冲突
            ("S40 E40 L30 M1.5 TL20 CH2@R\nS36 E36 L5", "不能倒角"),
            // 相邻段倒角落在本段右端
            ("S40 E40 L30 M1.5 TL20\nS30 E30 L5 CH2@L", "不能倒角"),
        ];
        for (text, needle) in cases {
            let program = parse_program(text).unwrap_or_else(|e| panic!("{text}: 解析失败 {e}"));
            let err = build(&program, 1.0).unwrap_err();
            assert!(err.contains("第 1 段"), "{text}: 应报第 1 段，得到 {err}");
            assert!(err.contains(needle), "{text}: 期望含「{needle}」，得到 {err}");
        }
    }

    // ── 段级 RL（任意圆柱段的肩部退刀槽，GB/T 3-1997 表 2 剖面） ────────

    /// `RL@L` / `RL@R` 几何、落层与剖视剖面环。
    #[test]
    fn segment_relief_geometry_left_right_and_layers() {
        // RL@L：面 x=10，槽向 +x 展开；P=1.5、d=20 → dg=17.7 / g1=2.5 / g2=4.5 / r=0.8
        let program =
            parse_program("S30 E30 L10 | S20 E20 L30 RL@L P1.5 | S30 E30 L10").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        // 中段轮廓从 x=10+4.5=14.5 起
        assert!(has_line(&shaft, [14.5, 10.0], [40.0, 10.0]), "槽后轮廓");
        assert_eq!(
            layer_of_line(&shaft, [14.5, 10.0], [40.0, 10.0]),
            Some(LAYER_MAIN)
        );
        // R 圆角（心 (10.8, 9.65)、180°→270°）+ 镜像
        assert!(has_arc(&shaft, [10.8, 9.65], 0.8, 180.0, 270.0), "左端上圆角");
        assert!(has_arc(&shaft, [10.8, -9.65], 0.8, 90.0, 180.0), "左端下圆角");
        // 槽底 g1=2.5：10.8 → 12.5（y=8.85）；斜壁：12.5,8.85 → 14.5,10
        assert!(has_line(&shaft, [10.8, 8.85], [12.5, 8.85]), "槽底");
        assert!(has_line(&shaft, [12.5, 8.85], [14.5, 10.0]), "斜壁");
        assert!(has_line(&shaft, [10.8, -8.85], [12.5, -8.85]));
        assert!(has_line(&shaft, [12.5, -8.85], [14.5, -10.0]));
        // 台肩面 x=10：圆角切点 9.65 → 左邻半径 15
        assert!(has_line(&shaft, [10.0, 9.65], [10.0, 15.0]), "台肩面");
        // 三条贯通竖线（用户 2026-09-18 更正版）：槽肩（= 段边界线）/ 槽底终止 / 斜壁终点
        assert_eq!(
            layer_of_line(&shaft, [10.0, -9.65], [10.0, 9.65]),
            Some(LAYER_MAIN),
            "槽肩贯通竖线"
        );
        assert_eq!(
            layer_of_line(&shaft, [12.5, -8.85], [12.5, 8.85]),
            Some(LAYER_MAIN),
            "槽底终止贯通竖线"
        );
        assert_eq!(
            layer_of_line(&shaft, [14.5, -10.0], [14.5, 10.0]),
            Some(LAYER_MAIN),
            "斜壁终点贯通竖线"
        );
        // 槽体落层 = 1轮廓实线层（不是细线）
        assert_eq!(
            layer_of_line(&shaft, [10.8, 8.85], [12.5, 8.85]),
            Some(LAYER_MAIN)
        );
        assert_eq!(
            layer_of_line(&shaft, [12.5, 8.85], [14.5, 10.0]),
            Some(LAYER_MAIN)
        );

        // RL@R（默认端）：面 x=40，槽向 −x 展开
        let program =
            parse_program("S30 E30 L10 | S20 E20 L30 RL@R P1.5 | S30 E30 L10").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        assert!(has_line(&shaft, [10.0, 10.0], [35.5, 10.0]), "右端槽前轮廓");
        assert!(has_arc(&shaft, [39.2, 9.65], 0.8, 270.0, 360.0), "右端上圆角");
        assert!(has_arc(&shaft, [39.2, -9.65], 0.8, 0.0, 90.0));
        assert!(has_line(&shaft, [39.2, 8.85], [37.5, 8.85]), "右端槽底");
        assert!(has_line(&shaft, [37.5, 8.85], [35.5, 10.0]), "右端斜壁");
        assert!(has_line(&shaft, [40.0, 9.65], [40.0, 15.0]), "右端台肩面");
        // 右端槽的三条贯通竖线（镜像）
        assert!(has_line(&shaft, [40.0, -9.65], [40.0, 9.65]), "右端槽肩贯通竖线");
        assert!(has_line(&shaft, [37.5, -8.85], [37.5, 8.85]), "右端槽底终止竖线");
        assert!(has_line(&shaft, [35.5, -10.0], [35.5, 10.0]), "右端斜壁终点竖线");
        // `RL` 省略端别 = 右端（既有默认）
        let bare = build(
            &parse_program("S30 E30 L10 | S20 E20 L30 RL P1.5 | S30 E30 L10").unwrap(),
            1.0,
        )
        .unwrap();
        assert!(has_arc(&bare, [39.2, 9.65], 0.8, 270.0, 360.0));

        // 剖视：段级退刀槽也进剖面环（一个 HATCH、边界首尾相接）
        let section = build(
            &parse_program("S30 E30 L10 | S20 E20 L30 RL@L P1.5 | S30 E30 L10 VIEW 剖视")
                .unwrap(),
            1.0,
        )
        .unwrap();
        let hatch = section
            .entities
            .iter()
            .find_map(|e| match e {
                EntityType::Hatch(h) => Some(h),
                _ => None,
            })
            .expect("剖视应有一个 HATCH");
        assert_eq!(hatch.paths.len(), 2, "轴线上/下两个环（参考件口径）");
        let pts = |edge: &ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge|
         -> ([f64; 2], [f64; 2]) {
            match edge {
                ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge::Line(e) => {
                    ([e.start.x, e.start.y], [e.end.x, e.end.y])
                }
                ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge::CircularArc(a) => (
                    [
                        a.center.x + a.radius * a.start_angle.cos(),
                        a.center.y + a.radius * a.start_angle.sin(),
                    ],
                    [
                        a.center.x + a.radius * a.end_angle.cos(),
                        a.center.y + a.radius * a.end_angle.sin(),
                    ],
                ),
                other => panic!("剖面环只应有 LINE/ARC，得到 {other:?}"),
            }
        };
        for path in &hatch.paths {
            let mut prev: Option<[f64; 2]> = None;
            for edge in &path.edges {
                let (a, b) = pts(edge);
                if let Some(p) = prev {
                    assert!(
                        near(p[0], a[0]) && near(p[1], a[1]),
                        "剖面环断开：{p:?} → {a:?}"
                    );
                }
                prev = Some(b);
            }
            let last = prev.unwrap();
            assert!(near(last[0], 0.0), "环回到左端面：{last:?}");
        }
    }

    /// 三种取参路径：① 显式 g1/g2/dg/r（dg 绝对值） ② P 查表 ③ P + 覆盖。
    #[test]
    fn segment_relief_parameter_paths() {
        // ① 显式四件套（d=25 → dg=22.7、g1=2.5、g2=4.5、r=0.8；不给 P）
        let program = parse_program(
            "S35 E35 L10 | S25 E25 L30 RL@L g1 2.5 g2 4.5 dg 22.7 r 0.8 | S35 E35 L10",
        )
        .unwrap();
        let shaft = build(&program, 1.0).unwrap();
        assert!(has_arc(&shaft, [10.8, 12.15], 0.8, 180.0, 270.0));
        assert!(has_line(&shaft, [10.8, 11.35], [12.5, 11.35]));
        assert!(has_line(&shaft, [12.5, 11.35], [14.5, 12.5]));
        // `=` / `:` 写法同样收
        let eq = parse_program(
            "S35 E35 L10 | S25 E25 L30 RL@L g1=2.5 g2:4.5 dg=22.7 r=0.8 | S35 E35 L10",
        )
        .unwrap()
        .segments[1]
        .relief
        .clone();
        assert_eq!(eq.len(), 1);
        assert_eq!(
            (eq[0].g1, eq[0].g2, eq[0].dg, eq[0].r),
            (Some(2.5), Some(4.5), Some(22.7), Some(0.8))
        );
        // ② 只给 P → 表 2
        let program = parse_program("S35 E35 L10 | S25 E25 L30 RL@R P1.5 | S35 E35 L10")
            .unwrap();
        let shaft = build(&program, 1.0).unwrap();
        assert!(has_arc(&shaft, [39.2, 12.15], 0.8, 270.0, 360.0));
        assert!(has_line(&shaft, [37.5, 11.35], [35.5, 12.5]));
        // ③ P + 覆盖（表值取 g2/dg/r，r 换成 0.5）
        let program =
            parse_program("S35 E35 L10 | S25 E25 L30 RL@L P1.5 r 0.5 | S35 E35 L10").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        assert!(has_arc(&shaft, [10.5, 11.85], 0.5, 180.0, 270.0), "覆盖后的圆角");
        assert!(has_line(&shaft, [10.5, 11.35], [12.5, 11.35]), "覆盖后槽底");
        // P 的参数也可分开写：`P 1.5`
        let p = parse_program("S35 E35 L10 | S25 E25 L30 RL@L P 1.5 | S35 E35 L10")
            .unwrap()
            .segments[1]
            .relief
            .clone();
        assert_eq!(p[0].p, Some(1.5));
        // 参数与 RL 顺序无关：`RL@L g1 2.5 P1.5 g2 4.5 …` 也收
        let mixed = parse_program(
            "S35 E35 L10 | S25 E25 L30 RL@L g1 2.5 P1.5 g2 4.5 dg 22.7 r 0.8 | S35 E35 L10",
        )
        .unwrap();
        assert!(build(&mixed, 1.0).is_ok());
    }

    /// 缺参 / 非法尺寸 / 表里没有 P / M 段覆盖：报错带「第 N 段」且不猜。
    #[test]
    fn segment_relief_parameter_errors() {
        let cases: &[(&str, &str)] = &[
            // ③ 都不给：提示给 P 或显式四件套
            ("S30 E30 L10 | S20 E20 L30 RL@L | S30 E30 L10", "缺参"),
            // 只给一部分显式值、没给 P
            ("S30 E30 L10 | S20 E20 L30 RL@L g1 2.5 | S30 E30 L10", "缺参"),
            // P 不在表 2
            ("S30 E30 L10 | S20 E20 L30 RL@L P1.3 | S30 E30 L10", "表 2"),
            // dg ≥ d（槽比大径还大）
            (
                "S30 E30 L10 | S20 E20 L30 RL@L g1 2.5 g2 4.5 dg 20 r 0.8 | S30 E30 L10",
                "dg",
            ),
            // g2 ≤ g1
            (
                "S30 E30 L10 | S20 E20 L30 RL@L g1 4 g2 4 dg 17.7 r 0.8 | S30 E30 L10",
                "g2",
            ),
            // 圆角伸到大径之外
            (
                "S30 E30 L10 | S20 E20 L30 RL@L g1 2.5 g2 4.5 dg 17.7 r 2 | S30 E30 L10",
                "圆角",
            ),
            // g2 > 段长
            ("S30 E30 L10 | S20 E20 L2 RL@L P1.5 | S30 E30 L10", "g2"),
            // 参数没跟在 RL 后面
            ("S30 E30 L10 P1.5 | S20 E20 L30 | S30 E30 L10", "跟在 RL"),
            // RL 带值（旧写法）
            ("S30 E30 L10 | S20 E20 L30 RL=1 | S30 E30 L10", "不带值"),
            // 同一端重复
            (
                "S30 E30 L10 | S20 E20 L30 RL@L P1.5 RL@L P1.5 | S30 E30 L10",
                "RL 重复",
            ),
            // 参数重复
            ("S30 E30 L10 | S20 E20 L30 RL@L g1 2.5 g1 3 | S30 E30 L10", "重复"),
            // 非法正数
            ("S30 E30 L10 | S20 E20 L30 RL@L g1 0 | S30 E30 L10", "必须 > 0"),
            // RL@X 端别非法
            ("S30 E30 L10 | S20 E20 L30 RL@X P1.5 | S30 E30 L10", "端别"),
            // M 段 RL 不接受覆盖参数（旧行为不被静默改写）
            ("S40 E40 L30 M1.5 TL20 RL g1 2.5\nS36 E36 L5", "不支持"),
        ];
        for (text, needle) in cases {
            let err = match parse_program(text) {
                Ok(program) => build(&program, 1.0).unwrap_err(),
                Err(e) => e,
            };
            assert!(
                err.contains("第 1 段") || err.contains("第 2 段"),
                "{text}: 应定位到段，得到 {err}"
            );
            assert!(err.contains(needle), "{text}: 期望含「{needle}」，得到 {err}");
        }
    }

    /// 前置条件：该端相邻段更高（有台肩）、本段圆柱、不能是自由端。
    #[test]
    fn segment_relief_preconditions() {
        let cases: &[(&str, &str)] = &[
            // 左邻同径 → 没有台阶面
            ("S20 E20 L10 | S20 E20 L30 RL@L P1.5 | S30 E30 L10", "没有台阶面"),
            // 右邻同径 → 没有台阶面
            ("S30 E30 L10 | S20 E20 L30 RL@R P1.5 | S20 E20 L10", "没有台阶面"),
            // 锥面不能开槽
            ("S30 E30 L10 | S25 E20 L30 RL@L P1.5 | S30 E30 L10", "圆柱"),
            // 自由端
            ("S20 E20 L30 RL@L P1.5 | S30 E30 L10", "自由端"),
            ("S30 E30 L10 | S20 E20 L30 RL@R P1.5", "自由端"),
        ];
        for (text, needle) in cases {
            let program = parse_program(text).unwrap_or_else(|e| panic!("{text}: 解析失败 {e}"));
            let err = build(&program, 1.0).unwrap_err();
            assert!(err.contains(needle), "{text}: 期望含「{needle}」，得到 {err}");
        }
    }

    /// 与同端 CH / OV / M 的 RL 互斥；`M…RL` 旧行为不变。
    #[test]
    fn segment_relief_mutual_exclusion_and_thread_regression() {
        let cases: &[(&str, &str)] = &[
            // 本段同端 CH
            ("S30 E30 L10 | S20 E20 L30 RL@L CH2@L P1.5 | S30 E30 L10", "倒角"),
            // 相邻段 CH 落在同一端面
            ("S30 E30 L10 CH2@R | S20 E20 L30 RL@L P1.5 | S30 E30 L10", "倒角"),
            // 本段同端 OV
            ("S50 E50 L10 | S40 E40 L30 RL@L OV@L P1.5 | S50 E50 L10", "越程槽"),
            // 两个退刀槽挤在同一端面（中段右端 + 后段左端）
            (
                "S30 E30 L10 | S20 E20 L30 RL@R P1.5 | S30 E30 L10 RL@L P1.5",
                "只能有一个退刀槽",
            ),
        ];
        for (text, needle) in cases {
            let program = parse_program(text).unwrap_or_else(|e| panic!("{text}: 解析失败 {e}"));
            let err = build(&program, 1.0).unwrap_err();
            assert!(err.contains(needle), "{text}: 期望含「{needle}」，得到 {err}");
        }
        // `M…RL@R` 与旧 `M…RL` 几何完全一致（表 2 按螺距；回归）
        let old = build(
            &parse_program("S40 E40 L30 M1.5 TL20 RL\nS36 E36 L5").unwrap(),
            1.0,
        )
        .unwrap();
        let explicit = build(
            &parse_program("S40 E40 L30 M1.5 TL20 RL@R\nS36 E36 L5").unwrap(),
            1.0,
        )
        .unwrap();
        assert_eq!(old.entities.len(), explicit.entities.len());
        assert!(old.entities.iter().all(|e| explicit.entities.contains(e)));
        // M 段右端已有螺纹收尾，后段左端不能再开槽（同一端面双槽）
        let both = parse_program("S40 E40 L30 M1.5 TL20 RL\nS36 E36 L5 RL@L P1.5").unwrap();
        let err = build(&both, 1.0).unwrap_err();
        assert!(err.contains("退刀槽"), "{err}");
    }

    /// `M…RL@L`：左端段级退刀槽 + 右端局部螺纹可共存；
    /// 小径细实线绕开槽体（无 TL 的整段全螺纹也一条）。
    #[test]
    fn thread_segment_left_relief_with_local_thread() {
        // 第 1 段 Ø50 做左邻（更高，有台肩）；第 2 段 d=40、P=1.5 → dg=37.7 / rg=18.85 / tangent=19.65
        let program =
            parse_program("S50 E50 L5\nS40 E40 L30 M1.5 TL24 RL@L P1.5\nS36 E36 L5").unwrap();
        let thread = program.segments[1].thread.unwrap();
        assert!(!thread.relief, "RL@L 不是螺纹收尾（thread.relief 仍为 false）");
        assert_eq!(program.segments[1].relief.len(), 1);
        assert_eq!(program.segments[1].relief[0].end, End::L);
        let shaft = build(&program, 1.0).unwrap();
        let r1 = (40.0 - 1.0825 * 1.5) / 2.0;
        // 第 2 段 [5, 35] 左端槽体（d=40）
        assert!(has_arc(&shaft, [5.8, 19.65], 0.8, 180.0, 270.0));
        assert!(has_line(&shaft, [5.8, 18.85], [7.5, 18.85]));
        assert!(has_line(&shaft, [7.5, 18.85], [9.5, 20.0]));
        assert!(has_line(&shaft, [9.5, 20.0], [35.0, 20.0]), "大径轮廓从槽后起");
        // 小径细实线从斜壁与小径交点起（r1=19.188 > rg=18.85），不是从完整螺纹 6.5 起
        let x0 = 5.0 + 2.5 + 2.0 * (r1 - 18.85) / (20.0 - 18.85);
        assert!(has_line(&shaft, [x0, r1], [34.3, r1]), "小径细实线绕过槽体");
        assert!(!has_line(&shaft, [6.5, r1], [x0, r1]), "槽内不应画小径细实线");
        // 右端 TL24 局部螺纹收尾照旧（锥面 → 螺尾）
        assert!(has_line(&shaft, [35.0, 20.0], [34.3, r1]), "锥面过渡");
        assert!(has_line(&shaft, [34.3, r1], [30.5, 20.0]), "螺尾细实线");

        // 不给 TL（整段全螺纹）也支持：细实线从斜壁交点起
        let plain = build(
            &parse_program("S50 E50 L5\nS40 E40 L30 M1.5 RL@L P1.5\nS36 E36 L5").unwrap(),
            1.0,
        )
        .unwrap();
        assert!(has_line(&plain, [x0, r1], [35.0, r1]));
        assert!(has_line(&plain, [x0, -r1], [35.0, -r1]));
    }

    /// 段级 RL 的 JSON 往返；`M` 段的右端收尾仍走 `thread.relief`。
    #[test]
    fn segment_relief_json_round_trip() {
        let text = "S30 E30 L10 | S20 E20 L30 RL@L P1.5 RL@R g1 2.5 g2 4.5 dg 17.7 r 0.8 | S30 E30 L10";
        let program = parse_program(text).unwrap();
        let json = serde_json::to_string(&program).unwrap();
        assert!(json.contains("\"relief\""), "{json}");
        assert_eq!(parse_program(&json).unwrap(), program);
        // 单对象写法也收
        let one = r#"{"segments":[{"s":30,"l":10},{"s":20,"l":30,"relief":{"end":"L","p":1.5}},{"s":30,"l":10}]}"#;
        let program = parse_program(one).unwrap();
        assert_eq!(program.segments[1].relief[0].end, End::L);
        assert_eq!(program.segments[1].relief[0].p, Some(1.5));
        // 手搓 M 段 thread.relief + 段级 R 端 → 拒绝（避免画两遍）
        let bad = r#"{"segments":[{"s":40,"l":30,"thread":{"p":1.5,"relief":true},"relief":{"end":"R"}}]}"#;
        let err = parse_program(bad).unwrap_err();
        assert!(err.contains("thread.relief"), "{err}");
    }

    // ── 齿轮段 GEAR ──────────────────────────────────────────────────────

    #[test]
    fn dsl_gear_parses_and_derives() {
        // 直齿轮段：d = m·z 导出；默认齿宽 10m
        let program = parse_program("GEAR M3 Z20").unwrap();
        let seg = &program.segments[0];
        assert!(near(seg.s, 60.0) && near(seg.e, 60.0));
        assert!(near(seg.l, 30.0), "默认 H = 10m");
        let gear = seg.gear.expect("齿轮段");
        assert_eq!((gear.m, gear.z), (3.0, 20));
        assert_eq!(gear.h, None);
        assert!((gear.alpha_deg - 20.0).abs() < 1e-12, "GEAR 默认压力角 20°");
        // ALPHA：`ALPHA25` / `ALPHA=30` / `alpha=22.5` 都收，进 GearParams（影响 db/齿厚）
        for (text, want) in [
            ("GEAR M3 Z20 ALPHA25", 25.0),
            ("GEAR M3 Z20 ALPHA=30", 30.0),
            ("ALPHA45 GEAR M3 Z20", 45.0),
            ("GEAR M3 Z20 alpha=22.5", 22.5),
        ] {
            let g = parse_program(text).unwrap().segments[0].gear.unwrap();
            assert!((g.alpha_deg - want).abs() < 1e-9, "{text} → α={}", g.alpha_deg);
        }
        // 压力角只进派生参数，不改分度圆；JSON 序列化默认 20° 不写出、非默认写出
        let g25 = parse_program("GEAR M3 Z20 ALPHA25").unwrap().segments[0].gear.unwrap();
        assert!((g25.pitch_radius() - 30.0).abs() < 1e-9, "d=m·z 与 α 无关");
        let p25 = parse_program("GEAR M3 Z20 ALPHA25").unwrap();
        let j25 = serde_json::to_string(&p25).unwrap();
        assert!(j25.contains("\"alpha\":25"), "{j25}");
        let p20 = parse_program("GEAR M3 Z20").unwrap();
        let j20 = serde_json::to_string(&p20).unwrap();
        assert!(!j20.contains("alpha"), "默认 α 不写出（旧 JSON 兼容）：{j20}");
        assert_eq!(parse_program(&j25).unwrap(), p25, "带 α 的 JSON 往返要一致");
        // JSON 手写段：alpha 可省（默认 20）、可给
        let seg25 = parse_program(r#"{"segments":[{"gear":{"m":3,"z":20,"alpha":25}}]}"#)
            .unwrap()
            .segments[0]
            .gear
            .unwrap();
        assert!((seg25.alpha_deg - 25.0).abs() < 1e-9);
        let seg_def = parse_program(r#"{"segments":[{"gear":{"m":3,"z":20}}]}"#)
            .unwrap()
            .segments[0]
            .gear
            .unwrap();
        assert!((seg_def.alpha_deg - 20.0).abs() < 1e-12, "JSON 缺 alpha → 20°");
        // H 显式 + BETA0（直齿）可写
        let program = parse_program("GEAR M5 Z10 H20 BETA0").unwrap();
        let seg = &program.segments[0];
        assert!(near(seg.l, 20.0));
        assert_eq!(seg.gear.unwrap().h, Some(20.0));
        // 大小写 / 顺序无所谓（Z10 H20 GEAR M5）
        let shuffled = parse_program("z10 h20 gear m5").unwrap();
        assert_eq!(shuffled.segments[0], program.segments[0]);
        // 派生尺寸与 gear.rs 同口径（ra/da/2、r/d/2、rf/df/2）
        let gear = parse_program("GEAR M5 Z10 H20")
            .unwrap()
            .segments[0]
            .gear
            .unwrap();
        let params = crate::gear::GearParams {
            kind: crate::gear::GearKind::External,
            m: 5.0,
            z: 10,
            h: 20.0,
            ..crate::gear::GearParams::default()
        };
        assert!(near(gear.pitch_radius(), params.d() / 2.0));
        assert!(near(gear.addendum_radius(), params.da() / 2.0));
        assert!(near(gear.root_radius(), params.df() / 2.0));
        assert!(near(gear.pitch_radius(), 25.0));
        assert!(near(gear.addendum_radius(), 30.0));
        assert!(near(gear.root_radius(), 18.75));
    }

    #[test]
    fn dsl_gear_reports_errors() {
        let cases: &[(&str, &str)] = &[
            ("GEAR M3", "缺少 Z"),
            ("GEAR Z20", "缺少 M"),
            ("GEAR M3 Z20.5", "不是正整数"),
            ("GEAR M0 Z20", "模数 m 必须是正数"),
            ("GEAR M-3 Z20", "模数 m 必须是正数"),
            ("GEAR M3 Z1", "齿数 z 超出范围"),
            ("GEAR M3 Z20 S40", "不给 S/E"),
            ("GEAR M3 Z20 E40", "不给 S/E"),
            ("GEAR M3 Z20 L30", "不要再给 L"),
            ("GEAR M3 Z20 BETA12", "本期只做直齿"),
            ("GEAR M3 Z20 ALPHA0", "压力角 α 超出范围"),
            ("GEAR M3 Z20 ALPHA60", "压力角 α 超出范围"),
            ("GEAR M3 Z20 ALPHA25 ALPHA30", "关键字 ALPHA 重复"),
            ("GEAR M3 Z20 H0", "厚度 h 必须是正数"),
            ("GEAR M3 Z20 CH2", "不能与倒角"),
            ("GEAR M3 Z20 OV", "不能与越程槽"),
            ("GEAR M3 Z20 M1.5", "M 重复"),
            ("GEAR GEAR M3 Z20", "GEAR 重复"),
            ("S40 E40 L20 GEAR M3 Z20", "不给 S/E"),
        ];
        for (text, needle) in cases {
            let err = parse_program(text).unwrap_err();
            assert!(err.contains(needle), "{text}: 期望含「{needle}」，得到 {err}");
        }
        // GEAR 的 m/z 错误也要能按「第 N 段」定位（含跨行）
        let err = parse_program("GEAR M0 Z20").unwrap_err();
        assert!(err.contains("第 1 段"), "{err}");
        let err = parse_program("S30 E30 L10\nGEAR M3 Z20.5").unwrap_err();
        assert!(err.contains("第 2 段"), "{err}");
    }

    // ── 矩形花键段 SPLINE（GB/T 1144 规格代号） ────────────────────────

    /// DSL：`SPLINE 6x23x26x6 L30 [de …]` → 大径 D、段长 = L + l；关键字位置无关。
    #[test]
    fn dsl_spline_parses_and_derives() {
        let program = parse_program("SPLINE 6x23x26x6 L30").unwrap();
        assert_eq!(program.segments.len(), 1);
        let seg = &program.segments[0];
        let sp = seg.spline.expect("花键段");
        assert_eq!((sp.n, sp.d, sp.big, sp.b, sp.de, sp.len), (6, 23.0, 26.0, 6.0, 63.0, 30.0));
        assert!(near(seg.s, 26.0) && near(seg.e, 26.0), "s=e=大径 D");
        assert!(near(seg.l, 30.0 + sp.runout()), "段长 = L + l");
        assert!(near(sp.runout(), 9.604_687));
        // de 覆盖（贴写 / 空格 / `=`）；大小写 / 顺序无关
        let with_de = parse_program("SPLINE 6x23x26x6 de71 L30").unwrap();
        assert!(near(with_de.segments[0].spline.unwrap().de, 71.0));
        let shuffled = parse_program("l30 spline 6x23x26x6").unwrap();
        assert_eq!(shuffled.segments[0], program.segments[0]);
        for text in ["SPLINE 6x23x26x6 L30 de 71", "SPLINE 6x23x26x6 L30 de=71", "SPLINE 6×23×26×6 L30"] {
            assert!(parse_program(text).is_ok(), "{text}");
        }
        // 表外规格：不给 de 报错并指路；给了 de 可出图
        let err = parse_program("SPLINE 6x11x14x3 L20").unwrap_err();
        assert!(err.contains("de"), "{err}");
        assert!(parse_program("SPLINE 6x11x14x3 L20 de63").is_ok());
        // JSON 往返：序列化模型能原样解析回来
        let json = serde_json::to_string(&program).unwrap();
        assert!(json.contains("spline"), "{json}");
        assert_eq!(parse_program(&json).unwrap(), program);
        let dsl_from_gui = r#"{"segments":[{"spline":{"spec":"6x23x26x6","len":30,"de":71}}]}"#;
        assert!(near(
            parse_program(dsl_from_gui).unwrap().segments[0]
                .spline
                .unwrap()
                .de,
            71.0
        ));
    }

    /// 表内规格不传 de：轴段 DSL 与 JSON 模型都自动查 GB/T 10952 表并算 R/l；
    /// 表外规格仍必须显式 de（独立要素入口另有等价回归，见 `detail.rs`）。
    #[test]
    fn spline_table_spec_without_de_auto_lookup() {
        // ① DSL：SPLINE 6x23x26x6 L30 → de=63（R=31.5）、h=1.5、l=9.6047
        let program = parse_program("SPLINE 6x23x26x6 L30").unwrap();
        let sp = program.segments[0].spline.expect("花键段");
        assert_eq!((sp.de, sp.hob_radius()), (63.0, 31.5), "de 查表 / R=de/2");
        assert!(near(sp.depth(), 1.5) && near(sp.runout(), 9.604_687), "h / l 数值");
        assert!(near(program.segments[0].l, 39.604_687), "段长 = L + l");
        // ② JSON 模型（GUI 段表 → `/api/shaft_export` 的口径）：不传 de 同样查表
        let json = r#"{"segments":[{"spline":{"spec":"6x28x32x7","len":30}}]}"#;
        let sp = parse_program(json).unwrap().segments[0].spline.expect("花键段");
        assert_eq!(sp.de, 71.0, "6x28x32x7 → de=71");
        assert_eq!(sp.hob_radius(), 35.5, "R = de/2");
        // h=2、l=√(2×(71−2))=11.747340
        assert!(near(sp.depth(), 2.0) && near(sp.runout(), 11.747_340), "h / l 数值");
        // ③ 表外规格行为不变：不传 de 报错指路；给了 de 才能出图
        let err = parse_program(r#"{"segments":[{"spline":{"spec":"6x11x14x3","len":20}}]}"#)
            .unwrap_err();
        assert!(err.contains("de 查不到") && err.contains("请给 de 覆盖"), "{err}");
        assert!(parse_program(
            r#"{"segments":[{"spline":{"spec":"6x11x14x3","len":20,"de":63}}]}"#
        )
        .is_ok());
    }

    /// DSL 错误：缺规格/缺 L、不给 S/E、与 CH/OV/RL/M/GEAR 互斥、de ≤ D。
    #[test]
    fn dsl_spline_reports_errors() {
        let cases: &[(&str, &str)] = &[
            ("SPLINE 6x23x26x6", "缺少 L"),
            ("SPLINE L30", "规格代号"),
            ("SPLINE 6x23x26x6 S26 E26 L30", "不给 S/E"),
            ("SPLINE 6x23x26x6 L30 CH2@L", "不能与倒角"),
            ("SPLINE 6x23x26x6 L30 OV3", "不能与越程槽"),
            ("SPLINE 6x23x26x6 L30 RL@L P1.5", "不能与退刀槽"),
            ("SPLINE 6x23x26x6 L30 M1.5", "不能与螺纹"),
            ("SPLINE 6x23x26x6 L30 GEAR M3 Z20", "齿轮段"),
            ("SPLINE 6x23x26x6 L30 de26", "必须大于大径"),
            ("SPLINE 6x23x26x6 L30 de63 de71", "de 覆盖重复"),
        ];
        for (text, needle) in cases {
            let err = parse_program(text).unwrap_err();
            assert!(err.contains(needle), "{text}: 期望含「{needle}」，得到 {err}");
        }
        // 花键段不能与倒角同段（引入倒角写在相邻段上）
        let err = parse_program("SPLINE 6x23x26x6 L30 CH2").unwrap_err();
        assert!(err.contains("相邻轴段") || err.contains("不能与倒角"), "{err}");
    }

    /// 单段花键轴：大径线 / 小径细线 / 收尾弧（圆心、相切、终点）/ 细竖线 / 端面。
    #[test]
    fn spline_segment_geometry_matches_template_group4() {
        let program = parse_program("SPLINE 6x23x26x6 L30").unwrap();
        let sp = program.segments[0].spline.unwrap();
        let shaft = build(&program, 1.0).unwrap();
        let (l, r, ra, rh) = (sp.len, sp.minor_radius(), sp.major_radius(), sp.hob_radius());
        // 整个段长 = L + l；自由端面在 L+l
        assert!(near(program.total_length(), l + sp.runout()));
        assert!(has_line(&shaft, [0.0, -ra], [0.0, ra]), "左端面 ±D/2");
        assert!(has_line(&shaft, [l + sp.runout(), -ra], [l + sp.runout(), ra]), "右端面");
        // 大径线：满齿段 [0, L] 与收尾段 [L, L+l] 两段（模板 [43]/[46] 也在 x=L 断开）
        assert!(has_line(&shaft, [0.0, ra], [l, ra]));
        assert!(has_line(&shaft, [l, ra], [l + sp.runout(), ra]));
        assert!(has_line(&shaft, [0.0, -ra], [l, -ra]));
        assert!(has_line(&shaft, [l, -ra], [l + sp.runout(), -ra]));
        // 小径细线 [0, L] 落 2细线层
        assert_eq!(layer_of_line(&shaft, [0.0, r], [l, r]), Some(LAYER_THIN));
        assert_eq!(layer_of_line(&shaft, [0.0, -r], [l, -r]), Some(LAYER_THIN));
        // 收尾弧：上弧 270°→360°−a；下弧 a→90°（模板 [56]/[57] 同口径）
        let a = sp.runout_end_angle();
        assert!(near(a, 72.247_210));
        assert!(has_arc(&shaft, [l, r + rh], rh, 270.0, 360.0 - a));
        assert!(has_arc(&shaft, [l, -(r + rh)], rh, a, 90.0));
        // 满齿终止细竖线 x=L，跨 ±D/2（模板 [42]）；自由端收尾终点由右端面闭合
        assert_eq!(layer_of_line(&shaft, [l, -ra], [l, ra]), Some(LAYER_THIN));
        // 尾端面是 1轮廓实线层
        assert_eq!(
            layer_of_line(&shaft, [l + sp.runout(), -ra], [l + sp.runout(), ra]),
            Some(LAYER_MAIN)
        );
        // 剖视：小径线 / 收尾弧改 1轮廓实线层 + 轴线↔小径两条带 HATCH
        let section = build(
            &parse_program("SPLINE 6x23x26x6 L30 VIEW 剖视").unwrap(),
            1.0,
        )
        .unwrap();
        assert_eq!(layer_of_line(&section, [0.0, r], [l, r]), Some(LAYER_MAIN));
        assert!(section
            .entities
            .iter()
            .any(|e| matches!(e, EntityType::Hatch(_))));
    }

    /// 相邻段写 CH：倒角落在花键凸角上，小径细线跟着内缩到倒角与小径的交点。
    #[test]
    fn spline_segment_with_neighbor_chamfer() {
        let program = parse_program("S22 E22 L5 CH2@R | SPLINE 6x23x26x6 L30").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        let sp = program.segments[1].spline.unwrap();
        let (r, ra) = (sp.minor_radius(), sp.major_radius());
        // φ22 段右端面 x=5 高 ±11；倒角（贴花键一侧）(5,11)→(7,13)
        assert!(has_line(&shaft, [5.0, -11.0], [5.0, 11.0]));
        assert!(has_line(&shaft, [5.0, 11.0], [7.0, ra]));
        // 小径细线从倒角与小径的交点 x=5+0.5 起，到满齿段右端 x=5+30
        assert_eq!(layer_of_line(&shaft, [5.5, r], [35.0, r]), Some(LAYER_THIN));
        assert!(near(5.5, 5.0 + 0.5), "内缩 = c − (R−r) = 0.5");
        // 花键段右端后面还有别的段时，收尾终点细竖线在 L+l
        let program = parse_program("SPLINE 6x23x26x6 L30 | S40 E40 L10").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        let end = 30.0 + sp.runout();
    }

    fn hatch_csv(entities: &[EntityType]) -> String {
        use ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge;
        let Some(h) = entities.iter().find_map(|e| match e {
            EntityType::Hatch(h) => Some(h),
            _ => None,
        }) else {
            return String::new();
        };
        let mut csv = format!(
            "# {} scale={} angle={} loops={}\n",
            h.pattern.name,
            h.pattern_scale,
            h.pattern_angle.to_degrees(),
            h.paths.len()
        );
        csv.push_str("ring,edge,x1,y1,x2,y2\n");
        for (ring, path) in h.paths.iter().enumerate() {
            for (edge, e) in path.edges.iter().enumerate() {
                if let BoundaryEdge::Line(l) = e {
                    csv.push_str(&format!(
                        "{},{},{:.6},{:.6},{:.6},{:.6}\n",
                        ring, edge, l.start.x, l.start.y, l.end.x, l.end.y
                    ));
                }
            }
        }
        csv
    }

    /// 矩形花键回归样例 → `~/桌面/OCSM/review/spline_*.csv`（`compare_spline.py` 用）。
    #[test]
    fn spline_demo_csv_dump() {
        let path = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
            .join("桌面/OCSM/review");
        std::fs::create_dir_all(&path).expect("建 review 目录");

        // 独立要素三个视图（L=30，局部原点 = 左端面/轴线）
        for (view, file) in [
            ("front", "spline_element_front.csv"),
            ("side", "spline_element_side.csv"),
            ("section", "spline_element_section.csv"),
        ] {
            let mut params = crate::detail::DetailParams::new();
            params.set_spec("6x23x26x6");
            if view != "front" {
                params.insert("len", 30.0);
            }
            let part = crate::detail::generate_params(
                crate::detail::FAMILY_SPLINE_RECT,
                23.0,
                &params,
                view,
            )
            .unwrap_or_else(|e| panic!("{view} 生成失败：{e}"));
            std::fs::write(path.join(file), entities_csv(&part.entities)).expect("写 CSV");
            assert!(path.join(file).is_file());
            if view == "section" {
                std::fs::write(
                    path.join("spline_element_section_hatch.csv"),
                    hatch_csv(&part.entities),
                )
                .expect("写 HATCH CSV");
            }
        }

        // 轴段特征（模板第 4 组的同局部系：x=0 = 左端面、y=0 = 轴线）
        let normal = build(&parse_program("SPLINE 6x23x26x6 L30").unwrap(), 1.0).unwrap();
        std::fs::write(path.join("spline_shaft_normal.csv"), entities_csv(&normal.entities))
            .expect("写 CSV");
        let section = build(
            &parse_program("SPLINE 6x23x26x6 L30 VIEW 剖视").unwrap(),
            1.0,
        )
        .unwrap();
        std::fs::write(path.join("spline_shaft_section.csv"), entities_csv(&section.entities))
            .expect("写 CSV");
        std::fs::write(
            path.join("spline_shaft_section_hatch.csv"),
            hatch_csv(&section.entities),
        )
        .expect("写 HATCH CSV");
        assert!(path.join("spline_shaft_normal.csv").is_file());
        // 带 φ22 引入倒角 + 同径后续段的轴（对齐模板第 4 组的局部系：左端面 x=5 ↔ 模板 47.988）
        let lead = build(
            &parse_program("S22 E22 L5 CH2@R | SPLINE 6x23x26x6 L30 | S26 E26 L40").unwrap(),
            1.0,
        )
        .unwrap();
        std::fs::write(path.join("spline_shaft_with_lead.csv"), entities_csv(&lead.entities))
            .expect("写 CSV");
    }

    #[test]
    fn gear_geometry_addendum_chamfer_pitch_and_root_line_by_view() {
        // 第 1 段 Ø40，第 2 段齿轮 M3 Z20（x=20..50）：d=60、ra=33、rf=26.25、C=2
        let program = parse_program("S40 E40 L20\nGEAR M3 Z20 H30").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        let gear = program.segments[1].gear.unwrap();
        let (r, ra, rf) = (
            gear.pitch_radius(),
            gear.addendum_radius(),
            gear.root_radius(),
        );
        // 齿顶面 = 齿宽两端各缩进 C = round(0.6m) = 2（与 gear.rs::side_view() 同）
        assert!(has_line(&shaft, [22.0, ra], [48.0, ra]));
        assert!(has_line(&shaft, [22.0, -ra], [48.0, -ra]));
        // 四个 45° 倒角（端面点 ra−C=31 → 齿顶 ra=33）
        assert!(has_line(&shaft, [20.0, 31.0], [22.0, 33.0]));
        assert!(has_line(&shaft, [20.0, -31.0], [22.0, -33.0]));
        assert!(has_line(&shaft, [48.0, 33.0], [50.0, 31.0]));
        assert!(has_line(&shaft, [48.0, -33.0], [50.0, -31.0]));
        // 台阶竖线（仅常规）：x=22/48、±ra
        assert!(has_line(&shaft, [22.0, -33.0], [22.0, 33.0]));
        assert!(has_line(&shaft, [48.0, -33.0], [48.0, 33.0]));
        let layer_of_y = |y: f64| -> Option<&str> {
            shaft.entities.iter().find_map(|e| match e {
                EntityType::Line(l)
                    if near(l.start.y, y)
                        && near(l.end.y, y)
                        && near(l.start.x, 17.0)
                        && near(l.end.x, 53.0) =>
                {
                    Some(l.common.layer.as_str())
                }
                _ => None,
            })
        };
        // 分度线 → 3中心线层（点划线）；凸出量 = 3n 每端（与 `gear::centerline_len` 同口径）
        assert!(has_line(&shaft, [17.0, r], [53.0, r]));
        assert!(has_line(&shaft, [17.0, -r], [53.0, -r]));
        assert_eq!(layer_of_y(r), Some(crate::partgen_kit::LAYER_CENTER));
        // 内侧直径线（齿根圆）：统一表达式的 MARK 画法开关 —— 既有口径（`gear.rs:4364` 测试 +
        // handbook 16 §九）：**花键（SPLINE）常规侧视图画小径细实线，齿轮（GEAR）不画（无齿根线）**；
        // 剖视两标记都画。本用例是 GEAR → 常规必须**没有**这条线。
        assert!(!has_line(&shaft, [20.0, rf], [50.0, rf]), "GEAR 常规不画齿根细实线");
        assert!(!has_line(&shaft, [20.0, -rf], [50.0, -rf]), "GEAR 常规不画齿根细实线");
        assert_eq!(layer_of_y(rf), None, "GEAR 齿根圆上无图元");
        // 2细线层定义 = ACI 4（青）+ Continuous（层色可断言；SPLINE 小径线会用这条层）。
        let thin = crate::layer_defs()
            .into_iter()
            .find(|l| l.name == LAYER_THIN)
            .expect("2细线层定义");
        assert_eq!(
            thin.color,
            ocs_plugin_api::host::acadrust::types::Color::from_index(4),
            "2细线层应为青色 ACI 4"
        );
        assert_eq!(thin.linetype, "Continuous", "2细线层应为细实线 Continuous");
        // 分度圆半径 30 不是轮廓（轮廓在齿顶 33）
        assert!(!shaft.entities.iter().any(|e| matches!(e, EntityType::Line(l)
            if near(l.start.y, r) && near(l.end.y, r) && l.common.layer == crate::partgen_kit::LAYER_MAIN)));
        // 端面：x=20 从 Ø40/2=20 到 ra−C=31；x=50 自由端面 ±31
        assert!(has_line(&shaft, [20.0, 20.0], [20.0, 31.0]));
        assert!(has_line(&shaft, [50.0, -31.0], [50.0, 31.0]));
        // 剖视：齿根线可见（也是剖面线边界）
        let section = build(
            &parse_program("S40 E40 L20\nGEAR M3 Z20 H30 | VIEW 剖视").unwrap(),
            1.0,
        )
        .unwrap();
        assert!(has_line(&section, [20.0, rf], [50.0, rf]));
        assert!(has_line(&section, [20.0, -rf], [50.0, -rf]));
        assert!(!has_line(&section, [20.0, -31.0], [20.0, 31.0]), "剖视无贯通端面线");
        // 最大直径按齿顶圆算
        assert!(near(shaft.max_diameter, 66.0));
        assert!(near(shaft.total_length, 50.0));
    }

    /// 跨模块一致性（用户报 bug）：轴侧齿形段的两条 `3中心线层` 分度线，凸出量必须与
    /// GEAR 生成器中心线**同一口径** `gear::centerline_len(L, n) = L + 6n`（两端各 3n，
    /// `gear.rs:61/68`）。外/内齿轮与外/内花键（SPLINE 标记）都查；与 GEAR 生成器同参数
    /// 视图（外齿 `side_view` / 内齿 `generate(Section)`）的 x 范围逐个对齐（容差 1e-6）。
    #[test]
    fn tooth_pitch_lines_match_gear_centerline_overhang() {
        fn pitch_range(entities: &[EntityType], d2: f64) -> (f64, f64) {
            entities
                .iter()
                .find_map(|e| match e {
                    EntityType::Line(l)
                        if l.common.layer == LAYER_CENTER
                            && (l.start.y - d2).abs() < 1e-9
                            && (l.start.y - l.end.y).abs() < 1e-9 =>
                    {
                        Some((l.start.x.min(l.end.x), l.start.x.max(l.end.x)))
                    }
                    _ => None,
                })
                .expect("应有一条分度线")
        }
        let n = 1.0;
        let cases: [(&str, bool); 4] = [
            ("GEAR EX M3 Z20 ALPHA20 X0 DA66 DF52.5 BETA0 H30", false),
            ("SPLINE EX M3 Z20 ALPHA30 X0 DA63 DF54.6 BETA0 H30", false),
            ("GEAR IN M3 Z20 ALPHA20 X0 DA67.5 DF54 BETA0 H30", true),
            ("SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30", true),
        ];
        for (expr, internal) in cases {
            for view in ["", " | VIEW 剖视"] {
            let program = parse_program(&format!("{expr}{view}")).unwrap();
            let g = program.segments[0].gear.unwrap();
            assert_eq!(g.kind.is_internal(), internal, "{expr}{view}");
            let shaft = build(&program, n).unwrap();
            let d2 = g.pitch_radius();
            let (sx0, sx1) = pitch_range(&shaft.entities, d2);
            // 轴侧单独断言：凸出量 = 3n/端，总长 = L + 6n，图层不变。
            let over = crate::gear::CENTER_OVERHANG * n / 2.0; // 3n
            let want = crate::gear::centerline_len(g.width(), n);
            assert!((sx0 + over).abs() < 1e-6, "{expr}{view}: 左端应凸出 3n，实得 {sx0}");
            assert!(
                (sx1 - (g.width() + over)).abs() < 1e-6,
                "{expr}{view}: 右端应凸出 3n，实得 {sx1}"
            );
            assert!(
                ((sx1 - sx0) - want).abs() < 1e-6,
                "{expr}{view}: 分度线长应为 L+6n={want}"
            );
            assert_eq!(
                layer_of_line(&shaft, [sx0, d2], [sx1, d2]),
                Some(LAYER_CENTER),
                "{expr}{view}"
            );
            // 跨模块：GEAR 生成器同参数视图的 x 范围（轴侧以段左端 0、齿轮以中心 0，
            // 齿轮侧平移段半长后应逐值相等）。
            let gp = GearParams {
                kind: g.kind,
                m: g.m,
                z: g.z,
                alpha_deg: g.alpha_deg,
                beta_deg: 0.0,
                h: g.width(),
                x: g.x,
                ..GearParams::default()
            };
            let gear_entities = if internal {
                crate::gear::generate(&gp, crate::gear::GearView::Section, n)
                    .unwrap()
                    .entities
            } else {
                crate::gear::side_view(&gp, n).unwrap()
            };
            let (gx0, gx1) = pitch_range(&gear_entities, d2);
            let shift = g.width() / 2.0;
            assert!(
                (sx0 - (gx0 + shift)).abs() < 1e-6,
                "{expr}{view}: 左端与 GEAR 不一致 {sx0} vs {}",
                gx0 + shift
            );
            assert!(
                (sx1 - (gx1 + shift)).abs() < 1e-6,
                "{expr}{view}: 右端与 GEAR 不一致 {sx1} vs {}",
                gx1 + shift
            );
            }
        }
    }

    /// 统一表达式 MARK 画法开关（方向修正 2026-09-23：以既有口径为准）：同一组参数下，
    /// `GEAR` 与 `SPLINE` 的**常规侧视图差异恰为两处小径/齿根细实线**（`SPLINE` 有、`GEAR` 没有；
    /// `2细线层`，位置 = `DF/2`）；其余图元逐条一致。
    /// **剖视两标记图元完全一致**（都画内侧线 `1轮廓实线层`）。
    #[test]
    fn unified_marker_toggles_root_thin_line_in_regular_view() {
        let base = "M3 Z20 ALPHA20 X0 DA66 DF52.5 BETA0 H30";
        let g = parse_program(&format!("GEAR EX {base}")).unwrap();
        let s = parse_program(&format!("SPLINE EX {base}")).unwrap();
        // 标记与字段经 DSL 保留。
        assert!(!g.segments[0].gear.unwrap().involute, "GEAR 标记");
        assert!(s.segments[0].gear.unwrap().involute, "SPLINE 标记");
        assert_eq!(s.segments[0].gear.unwrap().kind, GearKind::External);
        let sg = build(&g, 1.0).unwrap();
        let ss = build(&s, 1.0).unwrap();
        let rf = 52.5 / 2.0;
        // 常规：SPLINE 有两条 2细线层小径线（位置 = DF/2），GEAR 没有（齿轮「无齿根线」）。
        assert!(has_line(&ss, [0.0, rf], [30.0, rf]) && has_line(&ss, [0.0, -rf], [30.0, -rf]),
            "SPLINE 常规画小径细实线");
        assert!(!has_line(&sg, [0.0, rf], [30.0, rf]) && !has_line(&sg, [0.0, -rf], [30.0, -rf]),
            "GEAR 常规不画齿根细实线");
        assert_eq!(layer_of_line(&ss, [0.0, rf], [30.0, rf]), Some(LAYER_THIN), "小径线落 2细线层");
        assert_eq!(layer_of_line(&sg, [0.0, rf], [30.0, rf]), None, "GEAR 该位置无图元");
        let thin_count = |sh: &Shaft| sh.entities.iter().filter(|e| layer_of(e) == LAYER_THIN).count();
        assert_eq!(thin_count(&ss), 2, "SPLINE 常规恰 2 条细实线（上/下小径）");
        assert_eq!(thin_count(&sg), 0, "GEAR 常规不画内侧细实线");
        // 除那两条细线外，两个常规视图逐条一致。
        let rest = |sh: &Shaft| -> Vec<String> {
            let mut v: Vec<String> = entities_csv(&sh.entities)
                .lines()
                .filter(|l| !l.contains(LAYER_THIN))
                .map(str::to_string)
                .collect();
            v.sort();
            v
        };
        assert_eq!(rest(&sg), rest(&ss), "除内侧细实线外常规视图应完全一致");
        // 剖视：两标记都画内侧线，图元完全一致。
        let vg = build(&parse_program(&format!("GEAR EX {base} | VIEW 剖视")).unwrap(), 1.0).unwrap();
        let vs = build(&parse_program(&format!("SPLINE EX {base} | VIEW 剖视")).unwrap(), 1.0).unwrap();
        assert_eq!(layer_of_line(&vg, [0.0, rf], [30.0, rf]), Some(LAYER_MAIN));
        assert_eq!(layer_of_line(&vs, [0.0, rf], [30.0, rf]), Some(LAYER_MAIN));
        assert_eq!(entities_csv(&vg.entities), entities_csv(&vs.entities), "剖视两标记应完全一致");
    }

    /// 轴侧统一齿形段：新格式 / 旧格式（兼容）/ 缺 DA/DF 回推 / 内齿 / JSON 往返 / 矩形花键不变。
    #[test]
    fn unified_tooth_segment_parse_compat_and_json() {
        // 新格式外齿：显式 DA/DF 直接用。
        let p = parse_program("GEAR EX M3 Z20 ALPHA20 X0.2 DA66 DF52.5 BETA0 H30").unwrap();
        let g = p.segments[0].gear.unwrap();
        assert!(!g.involute && g.kind == GearKind::External);
        assert!((g.x - 0.2).abs() < 1e-12);
        assert!((g.major_radius() - 33.0).abs() < 1e-9 && (g.minor_radius() - 26.25).abs() < 1e-9);
        // 旧格式：仍可解析，DA/DF 按 ha*=1/c*=0.25、X 缺省 0 推。
        let p = parse_program("GEAR M3 Z20 H30 ALPHA20").unwrap();
        let g = p.segments[0].gear.unwrap();
        assert!(!g.involute && g.kind == GearKind::External && g.x.abs() < 1e-12);
        assert!((g.major_radius() - 33.0).abs() < 1e-9);
        assert!((g.minor_radius() - 26.25).abs() < 1e-9);
        // 缺 DA/DF 回推（X≠0）：d′=m(z+2x)=61.2 → DA=67.2、DF=53.7（±2m / −2.5m）。
        let p = parse_program("GEAR EX M3 Z20 X0.2 H30").unwrap();
        let g = p.segments[0].gear.unwrap();
        assert!((g.major_radius() - (61.2 + 6.0) / 2.0).abs() < 1e-9, "DA 回推");
        assert!((g.minor_radius() - (61.2 - 7.5) / 2.0).abs() < 1e-9, "DF 回推");
        // SPLINE + 齿形关键字 = 渐开线花键段（不是矩形花键）；显式 DA/DF 原样。
        let p = parse_program("SPLINE EX M3 Z20 ALPHA30 X0 DA63 DF54.6 BETA0 H30").unwrap();
        let g = p.segments[0].gear.unwrap();
        assert!(g.involute && p.segments[0].spline.is_none());
        assert!((g.major_radius() - 31.5).abs() < 1e-9 && (g.minor_radius() - 27.3).abs() < 1e-9);
        // 内齿：DA=外侧齿根（大径）、DF=里侧齿顶（小径）；外轮廓取 DA；GEAR 常规不画内孔线、剖视画。
        let p = parse_program("GEAR IN M3 Z20 ALPHA20 X0 DA67.5 DF54 BETA0 H30").unwrap();
        let g = p.segments[0].gear.unwrap();
        assert_eq!(g.kind, GearKind::Internal);
        assert!((p.segments[0].outer_radius(End::L) - 33.75).abs() < 1e-9);
        let sh = build(&p, 1.0).unwrap();
        assert!(has_line(&sh, [2.0, 33.75], [28.0, 33.75]), "内齿外轮廓 = 大径（自由端自动倒角 C=2）");
        assert!(!has_line(&sh, [0.0, 27.0], [30.0, 27.0]), "GEAR IN 常规不画内孔线（无齿根线）");
        let gsec = build(&parse_program("GEAR IN M3 Z20 ALPHA20 X0 DA67.5 DF54 BETA0 H30 | VIEW 剖视").unwrap(), 1.0).unwrap();
        assert_eq!(layer_of_line(&gsec, [0.0, 27.0], [30.0, 27.0]), Some(LAYER_MAIN), "GEAR IN 剖视画内孔线");
        // SPLINE IN 同参数：常规画小径细实线（MARK 开关），剖视也有。
        let p2 = parse_program("SPLINE IN M3 Z20 ALPHA30 X0 DA67.5 DF54 BETA0 H30").unwrap();
        let s2 = build(&p2, 1.0).unwrap();
        assert_eq!(layer_of_line(&s2, [0.0, 27.0], [30.0, 27.0]), Some(LAYER_THIN), "SPLINE IN 常规画小径细实线");
        let sc = build(&parse_program("SPLINE IN M3 Z20 ALPHA30 X0 DA67.5 DF54 BETA0 H30 | VIEW 剖视").unwrap(), 1.0).unwrap();
        assert!(has_line(&sc, [0.0, 27.0], [30.0, 27.0]), "SPLINE IN 剖视画内孔线");
        // 矩形花键不受影响：`SPLINE <规格>` 仍是 spline 段（不是齿形段）。
        let p = parse_program("SPLINE 6x23x26x6 L30").unwrap();
        assert!(p.segments[0].gear.is_none() && p.segments[0].spline.is_some());
        // JSON 往返：新字段（marker/kind/x/da/df）全部保留。
        let p = parse_program("SPLINE IN M3 Z20 ALPHA30 X0.3 DA67.5 DF54 BETA0 H30").unwrap();
        let back = parse_json(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(p, back, "统一齿形段 JSON 往返应逐字段一致");
        // 校验：DA ≤ DF 报错；DA 重复报错；无标记时 X/DA/DF 仍是未知关键字（不静默）。
        assert!(parse_program("GEAR EX M3 Z20 DA50 DF60 H30").unwrap_err().contains("必须大于小径"));
        assert!(parse_program("GEAR EX M3 Z20 DA66 DA66 H30").unwrap_err().contains("DA 重复"));
        assert!(parse_program("X0.3").unwrap_err().contains("不识别的关键字"));
    }

    #[test]
    fn gear_adjacent_diameter_rule_only_addendum_matters() {
        // Ø58（r=29 > 齿根 26.25，但 ≤ 齿顶 33）→ 放行（不再受 rf 限制）
        let program = parse_program("S58 E58 L20\nGEAR M3 Z20").unwrap();
        assert!(build(&program, 1.0).is_ok(), "相邻半径 ≤ ra 应放行");
        // 和齿顶齐平（Ø66）也放行
        let program = parse_program("S66 E66 L20\nGEAR M3 Z20").unwrap();
        assert!(build(&program, 1.0).is_ok());
        // Ø70（r=35 > 齿顶 33）→ 报「第 N 段」+「会盖住齿顶线」
        let program = parse_program("S70 E70 L20\nGEAR M3 Z20").unwrap();
        let err = build(&program, 1.0).unwrap_err();
        assert!(err.contains("第 2 段") && err.contains("会盖住齿顶线"), "{err}");
        // 同参数相邻齿轮（齿顶齐平）→ 放行
        let program = parse_program("GEAR M3 Z20 H20\nGEAR M3 Z20 H30").unwrap();
        assert!(build(&program, 1.0).is_ok());
        // 倒角会落在齿轮段上 → 报（第 1 段 Ø40 的 CH2@R 贴到齿轮凸角）
        let program = parse_program("S40 E40 L20 CH2@R\nGEAR M3 Z20").unwrap();
        let err = build(&program, 1.0).unwrap_err();
        assert!(err.contains("齿轮段"), "{err}");
    }

    /// 轴上齿轮/渐开线花键端面自动倒角的**单侧规则**（用户 2026-09-21 定案）：
    /// 自由端 / 邻段更小 → 加；邻段更大（肩部）/ 相等（齐平无外角）→ 不加。
    #[test]
    fn end_chamfer_one_sided_rule_on_shaft() {
        // ── ① 自由端（轴头/轴尾）→ 加：`GEAR` 单段两端都倒 ──
        let program = parse_program("GEAR M3 Z20 H30").unwrap();
        let gear = program.segments[0].gear.unwrap();
        let (ra, c) = (gear.addendum_radius(), gear.chamfer());
        assert!(near(c, 2.0), "C = round(0.6×3) = 2");
        let shaft = build(&program, 1.0).unwrap();
        assert!(has_line(&shaft, [0.0, ra - c], [c, ra]), "左自由端倒角斜线");
        assert!(has_line(&shaft, [30.0 - c, ra], [30.0, ra - c]), "右自由端倒角斜线");
        assert!(has_line(&shaft, [c, ra], [30.0 - c, ra]), "齿顶面两端缩 C");

        // ── ② 邻段更小（台阶向下）→ 加 ──
        let shaft = build(
            &parse_program("S40 E40 L20\nGEAR M3 Z20 H30").unwrap(),
            1.0,
        )
        .unwrap();
        assert!(has_line(&shaft, [20.0, ra - c], [20.0 + c, ra]), "邻段更小 → 左侧加");
        assert!(has_line(&shaft, [20.0 + c, ra], [50.0 - c, ra]), "齿顶面左端缩 C");

        // ── ③ 邻段相等（端面齐平、无外角）→ 不加 ──
        let shaft = build(
            &parse_program("S66 E66 L20\nGEAR M3 Z20 H30").unwrap(),
            1.0,
        )
        .unwrap();
        assert!(
            !has_line(&shaft, [20.0, ra - c], [20.0 + c, ra]),
            "齐平侧不许有倒角斜线"
        );
        assert!(
            !has_line(&shaft, [20.0 + c, -ra], [20.0 + c, ra]),
            "齐平侧无倒角根竖线"
        );
        assert!(has_line(&shaft, [20.0, ra], [50.0 - c, ra]), "齿顶面从端面直起");

    }

    /// 单侧规则的组合冒烟：齿轮/花键 × 自由/更小/更大/相等 × 有无 de × 左右位置，
    /// 构建不许 panic / 报错。
    #[test]
    fn end_chamfer_rule_combinations_build() {
        for text in [
            "GEAR M3 Z20 H30",
            "S40 E40 L20\nGEAR M3 Z20 H30",
            "S66 E66 L20\nGEAR M3 Z20 H30",
            "GEAR M3 Z20 H30\nS40 E40 L20",
        ] {
            let program = parse_program(text).unwrap_or_else(|e| panic!("{text}: {e}"));
            let shaft = build(&program, 1.0).unwrap_or_else(|e| panic!("{text}: {e}"));
            assert!(!shaft.entities.is_empty(), "{text}");
        }
    }

    /// 回归：独立齿轮生成器（`gear.rs::side_view()`）是独立零件、两端都是自由外角，
    /// **仍两端都倒** —— 轴上单侧规则不反向影响它。
    #[test]
    fn independent_gear_generator_still_chamfers_both_ends() {
        let p = crate::gear::GearParams { m: 3.0, z: 20, h: 30.0, ..Default::default() };
        let ra = p.da() / 2.0;
        let c = p.chamfer();
        let (hh, hc, hi) = (15.0, 15.0 - c, ra - c);
        let v = crate::gear::side_view(&p, 1.0).unwrap();
        let has = |a: [f64; 2], b: [f64; 2]| {
            v.iter().any(|e| match e {
                EntityType::Line(l) => {
                    let (p0, p1) = ([l.start.x, l.start.y], [l.end.x, l.end.y]);
                    (near(p0[0], a[0])
                        && near(p0[1], a[1])
                        && near(p1[0], b[0])
                        && near(p1[1], b[1]))
                        || (near(p0[0], b[0])
                            && near(p0[1], b[1])
                            && near(p1[0], a[0])
                            && near(p1[1], a[1]))
                }
                _ => false,
            })
        };
        assert!(has([-hh, hi], [-hc, ra]), "左端倒角斜线");
        assert!(has([hc, ra], [hh, hi]), "右端倒角斜线");
        assert!(has([-hh, -hi], [-hc, -ra]), "左下倒角斜线");
        assert!(has([hc, -ra], [hh, -hi]), "右下倒角斜线");
        assert!(has([-hh, -hi], [-hh, hi]), "左端面 ±(ra−C)");
        assert!(has([hh, -hi], [hh, hi]), "右端面 ±(ra−C)");
    }

    // ── 视图 VIEW（常规 / 剖视；双视图已于 2026-09-23 移除） ────────────

    #[test]
    fn dsl_view_parses_default_inline_standalone_and_json() {
        // 默认 = 常规
        assert_eq!(parse_program("S30 E30 L10").unwrap().view, ShaftView::Normal);
        // 独立一行 / 段内关键字 / = 或 : / 中文或英文
        for (text, want) in [
            ("VIEW 剖视\nS30 E30 L10", ShaftView::Section),
            ("VIEW=section S30 E30 L10", ShaftView::Section),
            ("S30 E30 L10 视图:剖视图", ShaftView::Section),
            ("VIEW 常规\nS30 E30 L10", ShaftView::Normal),
            ("S30 E30 L10", ShaftView::Normal),
        ] {
            assert_eq!(parse_program(text).unwrap().view, want, "{text}");
        }
        // 所有键可解析回自身（现在只有 normal/section）
        for view in ShaftView::ALL {
            assert_eq!(ShaftView::parse(view.key()).unwrap(), view);
        }
        // 多段行里也可放（且不影响段数）
        let program = parse_program("VIEW 剖视 | S30 E30 L10 | S20 E20 L5").unwrap();
        assert_eq!(program.view, ShaftView::Section);
        assert_eq!(program.segments.len(), 2);
        // JSON 同名字段：英文键或中文名
        for view in ["section", "剖视", "normal", "常规"] {
            let text = format!(r#"{{"segments":[{{"s":30,"l":10}}],"view":"{view}"}}"#);
            assert_eq!(
                parse_program(&text).unwrap().view,
                ShaftView::parse(view).unwrap()
            );
        }
        // 序列化回传（GUI/HTTP）：view 键可来回
        let program = parse_program("VIEW 剖视\nS30 E30 L10").unwrap();
        let json = serde_json::to_string(&program).unwrap();
        assert!(json.contains("\"view\":\"section\""), "{json}");
        assert_eq!(parse_program(&json).unwrap(), program);
        // 错误：视图名非法 / 缺名 / 冲突
        let err = parse_program("VIEW 隐藏\nS30 E30 L10").unwrap_err();
        assert!(err.contains("视图名无法识别"), "{err}");
        assert!(err.contains("可用：normal|常规、section|剖视"), "{err}");
        let err = parse_program("VIEW\nS30 E30 L10").unwrap_err();
        assert!(err.contains("缺少视图名"), "{err}");
        let err = parse_program("VIEW 常规\nS30 E30 L10\nVIEW 剖视").unwrap_err();
        assert!(err.contains("重复且冲突"), "{err}");
        // 块名把视图算进去（常规 ≠ 剖视）
        let normal = parse_program("S30 E30 L10").unwrap();
        let section = parse_program("S30 E30 L10 VIEW 剖视").unwrap();
        assert_ne!(block_name(&normal), block_name(&section));
    }

    #[test]
    fn section_view_hatch_ring_closes_on_hatch_layer() {
        // 带自由端倒角 + 越程槽 + 锥面：确认环每段接得上、最后闭合
        let program = parse_program(
            "S30 E30 L20 CH2@L\nS40 E40 L10 OV3\nS50 E30 L12\nVIEW 剖视",
        )
        .unwrap();
        // 常规视图没有 HATCH
        let normal = build(
            &Program {
                view: ShaftView::Normal,
                ..program.clone()
            },
            1.0,
        )
        .unwrap();
        assert!(!normal.entities.iter().any(|e| matches!(e, EntityType::Hatch(_))));
        let shaft = build(&program, 1.0).unwrap();
        let hatches: Vec<&Hatch> = shaft
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Hatch(h) => Some(h),
                _ => None,
            })
            .collect();
        assert_eq!(hatches.len(), 1, "剖视只有一个 HATCH（轴线上/下两环）");
        let h = hatches[0];
        assert_eq!(h.common.layer, crate::partgen_kit::LAYER_HATCH);
        assert_eq!(h.pattern.name, "ANSI31");
        assert_eq!(h.pattern_scale, 1.0);
        assert_eq!(h.paths.len(), 2, "轴线上/下两个环（参考件口径）");
        // 边界首尾相接（每个 edge 的终点 = 下一个 edge 的起点），两环都要闭合
        let pts = |edge: &BoundaryEdge| -> ([f64; 2], [f64; 2]) {
            match edge {
                BoundaryEdge::Line(e) => ([e.start.x, e.start.y], [e.end.x, e.end.y]),
                BoundaryEdge::CircularArc(a) => (
                    [
                        a.center.x + a.radius * a.start_angle.cos(),
                        a.center.y + a.radius * a.start_angle.sin(),
                    ],
                    [
                        a.center.x + a.radius * a.end_angle.cos(),
                        a.center.y + a.radius * a.end_angle.sin(),
                    ],
                ),
                other => panic!("剖面环只应有 LINE/ARC，得到 {other:?}"),
            }
        };
        // 环的 bbox 盖住整个轴剖面（总长 42，最大半径 25）
        let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
        let (mut x1, mut y1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for path in &h.paths {
            let edges = &path.edges;
            assert!(edges.len() >= 8, "每环至少 8 段：{}", edges.len());
            let mut prev: Option<[f64; 2]> = None;
            let mut first: Option<[f64; 2]> = None;
            for edge in edges {
                let (a, b) = pts(edge);
                if let Some(p) = prev {
                    assert!(near(p[0], a[0]) && near(p[1], a[1]), "边界断开：{p:?} → {a:?}");
                } else {
                    first = Some(a);
                }
                prev = Some(b);
                for p in [a, b] {
                    x0 = x0.min(p[0]);
                    x1 = x1.max(p[0]);
                    y0 = y0.min(p[1]);
                    y1 = y1.max(p[1]);
                }
            }
            let start = first.unwrap();
            let last = prev.unwrap();
            assert!(
                near(last[0], start[0]) && near(last[1], start[1]),
                "环未闭合：{last:?} → {start:?}"
            );
        }
        assert!(
            near(x0, 0.0) && near(x1, 42.0) && near(y0, -25.0) && near(y1, 25.0),
            "环 bbox = ({x0},{y0})..({x1},{y1})"
        );
    }

    /// 双视图已移除（用户 2026-09-23 定案）：DSL/JSON 都给**明确报错**，
    /// 不静默降级成常规（风格同撤掉轴段 `INVOLSPLINE` 的“点名报错”）。
    #[test]
    fn view_both_removed_errors_clearly() {
        for text in [
            "S30 E30 L10 VIEW 双",
            "VIEW 双\nS30 E30 L10",
            "S30 E30 L10 VIEW both",
            "S30 E30 L10 视图:并排",
        ] {
            let err = parse_program(text).unwrap_err();
            assert!(err.contains("双视图已移除"), "{text}: {err}");
            assert!(err.contains("常规/剖视"), "{text}: {err}");
        }
        // JSON 同口径
        let err = parse_program(r#"{"segments":[{"s":30,"l":10}],"view":"both"}"#)
            .unwrap_err();
        assert!(err.contains("双视图已移除"), "{err}");
        // 常规/剖视行为不受影响
        assert_eq!(
            parse_program("S30 E30 L10 VIEW 剖视").unwrap().view,
            ShaftView::Section
        );
        assert_eq!(
            parse_program("VIEW 常规\nS30 E30 L10").unwrap().view,
            ShaftView::Normal
        );
    }

    #[test]
    fn place_transforms_hatch_boundaries() {
        let program = parse_program("S30 E30 L20 VIEW 剖视").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        let bbox = |entities: &[EntityType]| -> [f64; 4] {
            let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
            let (mut x1, mut y1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
            for e in entities {
                if let EntityType::Hatch(h) = e {
                    for path in &h.paths {
                        for edge in &path.edges {
                            match edge {
                                BoundaryEdge::Line(e) => {
                                    for p in [e.start, e.end] {
                                        x0 = x0.min(p.x);
                                        x1 = x1.max(p.x);
                                        y0 = y0.min(p.y);
                                        y1 = y1.max(p.y);
                                    }
                                }
                                BoundaryEdge::CircularArc(a) => {
                                    for (px, py) in [
                                        (a.center.x + a.radius * a.start_angle.cos(),
                                         a.center.y + a.radius * a.start_angle.sin()),
                                        (a.center.x + a.radius * a.end_angle.cos(),
                                         a.center.y + a.radius * a.end_angle.sin()),
                                    ] {
                                        x0 = x0.min(px);
                                        x1 = x1.max(px);
                                        y0 = y0.min(py);
                                        y1 = y1.max(py);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            [x0, y0, x1, y1]
        };
        // 平移：边界 bbox → (100,35)..(120,65)
        let placed = place(shaft.entities, [100.0, 50.0], 0.0);
        let b = bbox(&placed);
        assert!(near(b[0], 100.0) && near(b[1], 35.0) && near(b[2], 120.0) && near(b[3], 65.0), "{b:?}");
        // 旋转 90°：原 (x,y) → (−y,x)，bbox x ∈ [−65,−35]、y ∈ [100,120]
        let rotated = place(placed, [0.0, 0.0], 90.0);
        let b = bbox(&rotated);
        assert!(near(b[0], -65.0) && near(b[1], 100.0) && near(b[2], -35.0) && near(b[3], 120.0), "{b:?}");
    }

    // ── 合法性 ────────────────────────────────────────────────────────────

    #[test]
    fn validity_errors_point_at_the_segment() {
        let cases: &[(&str, &str)] = &[
            ("S30 E30 L0", "第 1 段"),
            ("S30 E30 L-5", "第 1 段"),
            ("S0 E30 L10", "第 1 段"),
            ("S30 E-1 L10", "第 1 段"),
            ("S30 E30 L10 CH5@L", "段长/2"),
            ("S30 E30 L20 CH10@L", "段长/2"),
            // C ≥ 直径变化量的一半（Ø30→Ø40 的 5）
            ("S30 E30 L20 CH5@R\nS40 E40 L10", "直径变化量"),
            // 直径相同 → 没有端面可倒角
            ("S30 E30 L20 CH2@R\nS30 E30 L10", "没有端面"),
            // b1 > 段长（显式 3 > 2）
            ("S50 E50 L2 OV3\nS60 E60 L10", "b1=3 > 段长"),
            // b1 > 段长（查表默认 5 > 4；d=60 落 50<d<100 档）
            ("S60 E60 L4 OV\nS70 E70 L10", "b1=5 > 段长"),
            // 显式 b1 不在该 d 档的表里
            ("S40 E40 L20 OV4\nS50 E50 L10", "可选 b1"),
            // 两端槽重叠（2 + 2 > 3；两档都能取 b1=2）
            ("S60 E60 L10\nS50 E50 L3 OV@L OV@R\nS60 E60 L10", "两端特征重叠"),
            // 没有台阶（相邻段更小）
            ("S50 E50 L20 OV\nS40 E40 L10", "没有台阶面"),
            // 自由端不能做槽
            ("S50 E50 L20 OV", "自由端"),
            // 台阶太小：R 圆角切点（25.6）高过相邻台阶面（25.5）
            ("S50 E50 L20 OV\nS51 E51 L10", "台阶太小"),
            // 锥面不能做槽
            ("S50 E30 L20 OV\nS60 E60 L10", "锥面"),
            // 陡锥自由端倒角可能把端面吃穿（C < 端面半径也会发生）
            ("S10 E0.2 L10 CH4.9@L", "吃穿"),
            // 同一端面两侧都写倒角
            ("S30 E30 L20 CH2@R\nS40 E40 L10 CH2@L", "特征重叠"),
        ];
        for (text, needle) in cases {
            let program = parse_program(text).unwrap_or_else(|e| panic!("{text}: 解析失败 {e}"));
            let err = build(&program, 1.0).unwrap_err();
            assert!(err.contains(needle), "{text}: 期望含「{needle}」，得到 {err}");
        }
    }

    // ── 轴线 / 放置 ───────────────────────────────────────────────────────

    #[test]
    fn axis_length_is_total_plus_6n() {
        let program = parse_program("S30 E30 L45\nS40 E30 L20").unwrap();
        for (n, total, start, end) in [(1.0, 71.0, -3.0, 68.0), (2.0, 77.0, -6.0, 71.0)] {
            let shaft = build(&program, n).unwrap();
            let axis = shaft
                .entities
                .iter()
                .filter_map(|e| match e {
                    EntityType::Line(l) if l.common.layer == LAYER_CENTER => {
                        Some((l.start.x, l.end.x))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(axis.len(), 1, "轴线只有一条");
            let (start_x, end_x) = axis[0];
            assert!(near(end_x - start_x, total), "n={n}: {start_x}..{end_x} 长 {}", end_x - start_x);
            assert!(near(start_x, start) && near(end_x, end), "n={n}: {start_x}..{end_x}");
        }
    }

    #[test]
    fn place_rotates_and_translates() {
        let program = parse_program("S30 E30 L10").unwrap();
        let shaft = build(&program, 1.0).unwrap();
        let placed = place(shaft.entities, [100.0, 50.0], 0.0);
        let has = |entities: &Vec<EntityType>, a: [f64; 2], b: [f64; 2]| {
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
        };
        assert!(has(&placed, [97.0, 50.0], [113.0, 50.0]), "平移");
        let rotated = place(placed, [0.0, 0.0], 90.0);
        // 轴线 (97,50)..(113,50) 绕原点转 90° → (−50,97)..(−50,113)
        assert!(has(&rotated, [-50.0, 97.0], [-50.0, 113.0]), "旋转");
    }

    // ── 交付：demo 几何 dump（人工核对用） ────────────────────────────────

    /// 把 7 段 demo 的几何落成 CSV（列：entity,x1,y1,x2,y2,layer,cx,cy,r,a0,a1），
    /// 给用户画叠合/预览图。LINE 用两端点；ARC 用起终点 + 圆心/半径/角度。
    #[test]
    fn dump_demo_geometry_csv() {
        let program = parse_program(DEMO).unwrap();
        let shaft = build(&program, 1.0).unwrap();
        assert!(near(shaft.total_length, 175.0));
        assert!(near(shaft.max_diameter, 66.0));
        assert_eq!(shaft.segment_count, 7);

        let csv = entities_csv(&shaft.entities);

        // 关键几何自检（demo 7 段：30 / 40（OV3 + φ50 台阶倒角）/ 锥 50→30 / 30 /
        // 40（M1.5 TL20 局部螺纹）/ 36（小直径槽段）/ 齿轮段 M3 Z20（d=60、da=66、df=52.5 不落图））
        // OV3（φ40）：三条贯通竖线（槽肩 = 段边界 x=75）；砂轮细线不再画
        assert!(
            has_line(&shaft, [75.0, -20.6], [75.0, 20.6]),
            "OV 槽肩贯通竖线（×20.6 = d/2+r−h）"
        );
        assert!(
            has_line(&shaft, [72.4, -19.6], [72.4, 19.6]),
            "OV 槽底终止贯通竖线"
        );
        assert!(
            has_line(&shaft, [72.0, -20.0], [72.0, 20.0]),
            "OV 斜壁终点贯通竖线"
        );
        assert!(
            !has_line(&shaft, [72.0, 20.0], [57.0, 35.0]),
            "OV 砂轮细线不再画（用户 2026-09-18 定案：青线属标注）"
        );
        assert!(
            has_line(&shaft, [75.0, 22.0], [77.0, 24.0]),
            "φ50 台阶左端的 CH2（贴凸角）"
        );
        assert!(
            has_line(&shaft, [110.0, 15.0], [110.0, 18.0]),
            "前段 φ30 → φ40 台阶面"
        );
        assert!(has_line(&shaft, [110.0, 18.0], [112.0, 20.0]), "φ40 段左端 CH2");
        // ── M1.5 TL20 局部螺纹（GB/T 3 图 1 第一种形式；P=1.5 → x=3.8、a=4.5）──
        // 段 [110,140]：完整螺纹从 115.5 起；台肩面=140、分界=135.5、螺尾起点=139.3
        let r1 = (40.0 - 1.0825 * 1.5) / 2.0;
        assert!(near(r1, 19.188125));
        assert!(has_line(&shaft, [112.0, 20.0], [140.0, 20.0]), "螺纹段大径轮廓");
        assert!(has_line(&shaft, [140.0, 20.0], [139.3, r1]), "锥面过渡（a−x=0.7）");
        assert!(has_line(&shaft, [139.3, r1], [135.5, 20.0]), "螺尾细实线（x=3.8）");
        assert!(
            has_line(&shaft, [135.5, -20.0], [135.5, 20.0]),
            "完整螺纹/螺尾分界竖线"
        );
        assert!(has_line(&shaft, [115.5, r1], [139.3, r1]), "小径细实线（止于螺尾起点）");
        assert!(has_line(&shaft, [115.5, -r1], [139.3, -r1]));
        assert!(
            has_line(&shaft, [140.0, 18.0], [140.0, 20.0]),
            "螺纹终止台肩面（18 ↔ 20）"
        );
        assert_eq!(
            layer_of_line(&shaft, [139.3, r1], [135.5, 20.0]),
            Some(LAYER_THIN),
            "demo 螺尾落 2细线层"
        );
        assert_eq!(
            layer_of_line(&shaft, [140.0, 20.0], [139.3, r1]),
            Some(LAYER_MAIN),
            "demo 锥面落 1轮廓实线层"
        );
        // 小直径槽段 [140,145]：轮廓 18，端面 145 从 18 到 ra−C=31（齿轮端倒角 C=2）
        assert!(has_line(&shaft, [140.0, 18.0], [145.0, 18.0]), "小直径槽段轮廓");
        assert!(
            has_line(&shaft, [145.0, 18.0], [145.0, 31.0]),
            "槽段端面从 18 到 ra−C=31"
        );
        assert!(has_line(&shaft, [145.0, 31.0], [147.0, 33.0]), "齿轮左端倒角");
        assert!(has_line(&shaft, [173.0, 33.0], [175.0, 31.0]), "齿轮右端倒角");
        // 齿轮：齿顶面（h−2C = 26，±33）/ 分度（点划线，±30）；GEAR 常规视图不画齿根细实线（±26.25）
        assert!(has_line(&shaft, [147.0, 33.0], [173.0, 33.0]), "齿顶面（缩进 2C）");
        assert!(has_line(&shaft, [142.0, 30.0], [178.0, 30.0]), "分度线（凸出 3n/端，与 GEAR 同口径）");
        assert!(
            !has_line(&shaft, [145.0, 26.25], [175.0, 26.25]),
            "GEAR 常规视图不画齿根细实线（齿轮「无齿根线」）"
        );
        assert!(
            !has_line(&shaft, [145.0, -26.25], [175.0, -26.25]),
            "GEAR 常规视图下半也不画"
        );
        assert!(
            has_line(&shaft, [175.0, -31.0], [175.0, 31.0]),
            "齿轮自由端面 ±(ra−C)"
        );
        // 齿轮端倒角的台阶竖线（±ra，仅常规视图）
        assert!(has_line(&shaft, [147.0, -33.0], [147.0, 33.0]), "齿轮左台阶竖线");
        assert!(has_line(&shaft, [173.0, -33.0], [173.0, 33.0]), "齿轮右台阶竖线");
        let layer_at = |x: f64, y: f64| -> Option<&str> {
            shaft.entities.iter().find_map(|e| match e {
                EntityType::Line(l)
                    if near(l.start.x, x) && near(l.start.y, y) && near(l.end.y, y) =>
                {
                    Some(l.common.layer.as_str())
                }
                _ => None,
            })
        };
        assert_eq!(
            layer_at(142.0, 30.0),
            Some(crate::partgen_kit::LAYER_CENTER)
        );
        assert_eq!(layer_at(145.0, 26.25), None, "GEAR 常规视图无齿根细实线");

        let path = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
            .join("桌面/OCSM/review");
        std::fs::create_dir_all(&path).expect("建 review 目录");
        let file = path.join("shaft_demo.csv");
        std::fs::write(&file, &csv).expect("写 shaft_demo.csv");
        assert!(file.is_file(), "demo CSV 已落盘：{}", file.display());
        assert!(csv.contains("LINE") && csv.contains("ARC"));
        assert!(csv.contains("3中心线层") && csv.contains("2细线层"));
        // 65 = 旧 demo 59 + 齿轮倒角 6（4 斜线 + 2 台阶竖线）；
        // 旧 59 = 旧 demo 44 + 局部螺纹新增 5（锥面上下 2 + 螺尾上下 2 + 分界竖线 1）
        //      + 段边界贯通竖线 6 + 倒角终点竖线 3 + OV 槽两条界线 2 − OV 砂轮细线 1
        // 统一表达式（2026-09-23，方向修正）：GEAR 常规**不画**内侧细实线；图元数 = 65。
        assert_eq!(shaft.entities.len(), 65, "demo 图元数");

        // 剖视：真实几何 + 齿轮齿根线（2 条）+ 一个 HATCH（2 环）；不再画贯通竖线
        let section_program = parse_program(&format!("{DEMO}\nVIEW 剖视")).unwrap();
        let section = build(&section_program, 1.0).unwrap();
        // 65 − 13 贯通竖线 + 2 齿根线 + 1 HATCH = 55
        assert_eq!(section.entities.len(), 55, "剖视图元数");
        assert!(has_line(&section, [145.0, 26.25], [175.0, 26.25]), "剖视齿根线");
        assert!(
            !has_line(&section, [45.0, -15.0], [45.0, 15.0]),
            "剖视不画贯通竖线"
        );
        assert!(
            !has_line(&section, [75.0, -20.6], [75.0, 20.6]),
            "剖视不画槽肩贯通竖线"
        );
        let section_csv = entities_csv(&section.entities);
        assert!(
            section_csv.contains("HATCH,0.000000,-26.250000,175.000000,26.250000,5剖面线层"),
            "HATCH 一行记 bbox + 5剖面线层：{section_csv}"
        );
        let file = path.join("shaft_demo_section.csv");
        std::fs::write(&file, &section_csv).expect("写 shaft_demo_section.csv");
        assert!(file.is_file(), "剖面 CSV 已落盘：{}", file.display());
    }

    /// 用户实测轴（8 段，第 8 段 `S25 E25 L32 RL@L P1.5`）→
    /// `~/桌面/OCSM/review/轴测试_RL.csv`（列格式同 `shaft_demo.csv`）。
    #[test]
    fn dump_rl_review_axis_csv() {
        let text = "S28 E28 L55 CH2@L | S34 E34 L33 | S34 E35 L1 | S35 E35 L22 | S45 E45 L194 | S35 E35 L22 | S35 E34 L1 | S25 E25 L32 RL@L P1.5";
        let program = parse_program(text).expect("用户轴应能解析");
        let shaft = build(&program, 1.0).expect("用户轴应能出图");
        assert_eq!(shaft.segment_count, 8);
        assert!(near(shaft.total_length, 360.0));
        // 第 8 段 [328, 360] Ø25；左邻第 7 段右端 Ø34（r=17）→ 台肩。
        // 表 2 P1.5、d=25 → dg=22.7 / g1=2.5 / g2=4.5 / r=0.8（切点 12.15）
        assert!(has_arc(&shaft, [328.8, 12.15], 0.8, 180.0, 270.0), "第 8 段左端上圆角");
        assert!(has_arc(&shaft, [328.8, -12.15], 0.8, 90.0, 180.0));
        assert!(has_line(&shaft, [328.8, 11.35], [330.5, 11.35]), "槽底");
        assert!(has_line(&shaft, [330.5, 11.35], [332.5, 12.5]), "斜壁");
        assert!(
            has_line(&shaft, [328.0, 12.15], [328.0, 17.0]),
            "台肩面（切点 → 第 7 段 Ø34/2）"
        );
        assert!(has_line(&shaft, [332.5, 12.5], [360.0, 12.5]), "槽后轮廓");
        // 落盘（LINE/ARC 列：x1,y1,x2,y2,layer,cx,cy,r,a0,a1）
        let csv = entities_csv(&shaft.entities);
        let path = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
            .join("桌面/OCSM/review");
        std::fs::create_dir_all(&path).expect("建 review 目录");
        let file = path.join("轴测试_RL.csv");
        std::fs::write(&file, &csv).expect("写 轴测试_RL.csv");
        assert!(file.is_file(), "RL 轴 CSV 已落盘：{}", file.display());
        assert!(csv.contains("LINE") && csv.contains("ARC"));
    }

    /// **验收基准**：用户手工更正版 `~/桌面/OCSM/review/轴测试更正.dxf`（44 图元）的
    /// 输入轴（8 段 DSL，第 8 段 `OV@L`）。本测试把新几何落成
    /// `~/桌面/OCSM/review/轴测试_v2.csv`，供 `compare_shaft.py` 与基准逐图元比对。
    ///
    /// 更正版相对旧版的差：+9 条贯通竖线（段边界 5 / 倒角终点 1 / OV 槽 3），
    /// −1 条 OV 砂轮细线；本测试把这 10 条钉住。
    #[test]
    fn dump_user_axis_v2_csv() {
        let text = "S28 E28 L55 CH2@L | S34 E34 L33 | S34 E35 L1 | S35 E35 L22 | S45 E45 L194 | S35 E35 L22 | S35 E34 L1 | S25 E25 L32 OV@L";
        let program = parse_program(text).expect("用户轴应能解析");
        let shaft = build(&program, 1.0).expect("用户轴应能出图");
        assert_eq!(shaft.segment_count, 8);
        assert!(near(shaft.total_length, 360.0));
        // 用户更正版比旧版多的 9 条 `1轮廓实线层` 贯通竖线（逐条与 DXF 对齐）
        for (x, h) in [
            (2.0, 14.0),   // 倒角终点 = 倒角根半径（φ28/2）
            (55.0, 14.0),  // 段1|2 分界 = min(14, 17)
            (88.0, 17.0),  // 段2|3 分界（φ34→φ34，同径也画）
            (89.0, 17.5),  // 段3|4 分界（1mm 锥段终点）
            (111.0, 17.5), // 段4|5 分界 = min(17.5, 22.5)
            (305.0, 17.5), // 段5|6 分界 = min(22.5, 17.5)
            (328.0, 13.1), // OV 槽肩 = 台肩面 + 圆角切点（d/2−h+r）
            (330.6, 12.1), // OV 槽底终止 = 台肩面 + b1−h
            (331.0, 12.5), // OV 斜壁终点 = 台肩面 + b1
        ] {
            assert_eq!(
                layer_of_line(&shaft, [x, -h], [x, h]),
                Some(LAYER_MAIN),
                "贯通竖线 x={x} ±{h} 应落 1轮廓实线层"
            );
        }
        // 旧版那条 OV 砂轮细线（2细线层、45°、长 15）不再有 → **本轴（无 M 段）**
        // 整图 0 条 2细线层（限定：带螺纹的轴仍有小径/螺尾细实线，见 demo 测试）
        let thin = shaft.entities.iter().filter(|e| layer_of(e) == LAYER_THIN).count();
        assert_eq!(thin, 0, "本轴无 M 段，2细线层 应为 0 条（非通用断言）");
        assert!(
            !has_line(&shaft, [331.0, 12.5], [346.0, 27.5]),
            "OV 砂轮细线不再画"
        );
        // 既有几何回归（抽查更正版仍在的轮廓/圆角/端面）
        assert!(has_line(&shaft, [0.0, 12.0], [2.0, 14.0]), "左端 C2 上斜线");
        assert!(has_line(&shaft, [2.0, 14.0], [55.0, 14.0]), "φ28 轮廓");
        assert!(has_line(&shaft, [55.0, 14.0], [55.0, 17.0]), "段1|2 肩面");
        assert!(has_line(&shaft, [55.0, 17.0], [88.0, 17.0]), "φ34 轮廓");
        assert!(has_line(&shaft, [88.0, 17.0], [89.0, 17.5]), "1mm 锥面");
        assert!(has_line(&shaft, [111.0, 22.5], [305.0, 22.5]), "φ45 轮廓");
        assert!(has_line(&shaft, [327.0, 17.5], [328.0, 17.0]), "段7 收锥");
        assert!(has_line(&shaft, [328.0, 13.1], [328.0, 17.0]), "OV 台肩面");
        assert!(has_arc(&shaft, [329.0, 13.1], 1.0, 180.0, 270.0), "OV 上圆角");
        assert!(has_line(&shaft, [329.0, 12.1], [330.6, 12.1]), "OV 槽底");
        assert!(has_line(&shaft, [330.6, 12.1], [331.0, 12.5]), "OV 斜坡");
        assert!(has_line(&shaft, [331.0, 12.5], [360.0, 12.5]), "φ25 轮廓");
        assert!(has_line(&shaft, [360.0, -12.5], [360.0, 12.5]), "右端面");
        assert!(has_line(&shaft, [-3.0, 0.0], [363.0, 0.0]), "轴线");
        // 文字规则「每条段边界都画」在 x=327（段6|7，Ø35→Ø35 锥段起点）也会画；
        // 用户手工更正版漏了这条（如 x=88 同构却画了）。验收 diff 见报告：
        // 新生成 45 = 基准 44 + 这一条（不为了对齐而删规则线）。
        assert_eq!(
            layer_of_line(&shaft, [327.0, -17.5], [327.0, 17.5]),
            Some(LAYER_MAIN),
            "段6|7 同径分界贯通竖线（基准漏画，本期按规则保留）"
        );
        // 落盘（列格式同 shaft_demo.csv / 轴测试_RL.csv）
        let csv = entities_csv(&shaft.entities);
        let path = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
            .join("桌面/OCSM/review");
        std::fs::create_dir_all(&path).expect("建 review 目录");
        let file = path.join("轴测试_v2.csv");
        std::fs::write(&file, &csv).expect("写 轴测试_v2.csv");
        assert!(file.is_file(), "v2 CSV 已落盘：{}", file.display());
        // 45 = 基准 44 + x=327 那条（旧版 36 = 35 + 9 − 1）；逐图元 diff 见 compare_shaft.py
        assert_eq!(shaft.entities.len(), 45, "用户轴 v2 图元数");
    }

    // ── 剖视验收（参考件 `~/桌面/OCSM/review/轴剖视图.dxf`） ─────────────

    /// 参考件由几何反推出的轴：总长 152、7 段、`M1.5` 为**普通全螺纹**（不是局部螺纹）、
    /// 末段 `GEAR M3 Z20`（齿宽 30）。
    const REVIEW_AXIS: &str =
        "S30 E30 L45 CH2@L | S40 E40 L30 CH2@R OV3 | S50 E30 L20 | S30 E30 L15 CH2@R | S40 E40 L7 M1.5 | S36 E36 L5 | GEAR M3 Z20";

    /// 剖视验收：常规/剖视两份几何 + 剖面线边界逐边落盘，供 `compare_section.py`
    /// 与参考件两个视图逐图元对比。
    #[test]
    fn dump_section_review_csv() {
        let program = parse_program(REVIEW_AXIS).expect("参考轴应能解析");
        let normal = build(&program, 1.0).expect("参考轴常规视图应能出图");
        let section = build(
            &parse_program(&format!("{REVIEW_AXIS} | VIEW 剖视")).unwrap(),
            1.0,
        )
        .expect("参考轴剖视应能出图");
        assert!(near(normal.total_length, 152.0));
        assert!(near(normal.max_diameter, 66.0));
        assert_eq!(normal.segment_count, 7);

        // ── 常规：贯通竖线仍在（用户 2026-09-18 定案保持）──
        for (x, h) in [(2.0, 15.0), (45.0, 15.0), (72.0, 20.0), (72.4, 19.6), (75.0, 20.6),
                       (77.0, 24.0), (95.0, 15.0), (110.0, 15.0), (112.0, 20.0),
                       (117.0, 18.0), (122.0, 18.0)] {
            assert!(
                has_line(&normal, [x, -h], [x, h]),
                "常规视图贯通竖线 x={x} ±{h}"
            );
        }
        // 齿轮倒角台阶竖线（齿轮.rs::side_view() 的 ±ra 台阶线）：x=124/150、±33。
        assert!(has_line(&normal, [124.0, -33.0], [124.0, 33.0]), "齿轮左台阶竖线");
        assert!(has_line(&normal, [150.0, -33.0], [150.0, 33.0]), "齿轮右台阶竖线");
        // GEAR 常规不画齿根细实线（齿轮「无齿根线」；花键才有小径细实线）
        assert!(!has_line(&normal, [122.0, 26.25], [152.0, 26.25]), "GEAR 常规无齿根细实线");
        assert_eq!(layer_of_line(&normal, [122.0, 26.25], [152.0, 26.25]), None);

        // ── 剖视：不画贯通竖线，只留真实面/倒角/槽体 ──
        for (x, h) in [(2.0, 15.0), (45.0, 15.0), (72.0, 20.0), (72.4, 19.6), (75.0, 20.6),
                       (77.0, 24.0), (95.0, 15.0), (110.0, 15.0), (112.0, 20.0),
                       (117.0, 18.0), (122.0, 18.0)] {
            assert!(
                !has_line(&section, [x, -h], [x, h]),
                "剖视不应有贯通竖线 x={x} ±{h}"
            );
        }
        // 真实几何保留：段间端面 / 倒角 / 槽 / 齿轮倒角 + 齿根线
        assert!(has_line(&section, [45.0, 15.0], [45.0, 20.0]), "真实端面");
        assert!(has_line(&section, [75.0, 20.6], [75.0, 22.0]), "槽肩面到圆角切点");
        assert!(has_line(&section, [75.0, 22.0], [77.0, 24.0]), "倒角斜线");
        assert!(has_line(&section, [110.0, 15.0], [110.0, 18.0]), "真实端面");
        assert!(has_line(&section, [110.0, 18.0], [112.0, 20.0]), "倒角斜线");
        assert!(has_line(&section, [122.0, 18.0], [122.0, 31.0]), "齿轮端面 18→ra−C");
        assert!(has_line(&section, [122.0, 31.0], [124.0, 33.0]), "齿轮倒角斜线");
        assert!(has_line(&section, [124.0, 33.0], [150.0, 33.0]), "齿顶面 h−2C");
        assert!(has_line(&section, [150.0, 33.0], [152.0, 31.0]), "齿轮右倒角斜线");
        assert!(has_line(&section, [122.0, 26.25], [152.0, 26.25]), "剖视齿根线");
        assert!(has_line(&section, [152.0, -31.0], [152.0, 31.0]), "右端面");

        // ── 剖面线：1 个 HATCH、2 环（轴线上/下）、flags = EXTERNAL|OUTERMOST ──
        let hatch = section
            .entities
            .iter()
            .find_map(|e| match e {
                EntityType::Hatch(h) => Some(h),
                _ => None,
            })
            .expect("剖视应有一个 HATCH");
        assert_eq!(hatch.common.layer, crate::partgen_kit::LAYER_HATCH);
        assert_eq!(hatch.pattern.name, "ANSI31");
        assert_eq!(hatch.pattern_scale, 1.0);
        assert_eq!(hatch.paths.len(), 2, "轴线上/下两个环（参考件右视图口径）");
        for path in &hatch.paths {
            assert!(path.flags.is_external() && path.flags.is_outermost());
            assert_eq!(path.edges.len(), 21, "参考件每环 21 边");
        }
        // 参考件右视图的剖面线边界口径：螺纹段沿小径、齿轮段沿齿根、上环沿轴线闭合
        let upper = &hatch.paths[0].edges;
        let has_edge = |a: [f64; 2], b: [f64; 2]| {
            upper.iter().any(|e| match e {
                BoundaryEdge::Line(le) => {
                    let (p, q) = ([le.start.x, le.start.y], [le.end.x, le.end.y]);
                    (near(p[0], a[0]) && near(p[1], a[1]) && near(q[0], b[0]) && near(q[1], b[1]))
                        || (near(p[0], b[0])
                            && near(p[1], b[1])
                            && near(q[0], a[0])
                            && near(q[1], a[1]))
                }
                _ => false,
            })
        };
        assert!(has_edge([111.188125, 19.188125], [117.0, 19.188125]), "边界沿螺纹小径");
        assert!(has_edge([122.0, 26.25], [152.0, 26.25]), "边界沿齿轮齿根");
        assert!(has_edge([0.0, 0.0], [152.0, 0.0]), "上环沿轴线闭合");

        // ── 落盘（列格式同 shaft_demo.csv；HATCH 边界另落一份逐边 CSV）──
        let dir = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
            .join("桌面/OCSM/review");
        std::fs::create_dir_all(&dir).expect("建 review 目录");
        std::fs::write(dir.join("section_review_regular.csv"), entities_csv(&normal.entities))
            .expect("写 section_review_regular.csv");
        std::fs::write(dir.join("section_review_section.csv"), entities_csv(&section.entities))
            .expect("写 section_review_section.csv");
        let mut hatch_csv =
            String::from("path,edge,type,x1,y1,x2,y2,cx,cy,r,a0,a1,flags\n");
        for (pi, path) in hatch.paths.iter().enumerate() {
            let flags = path.flags.bits();
            for (ei, edge) in path.edges.iter().enumerate() {
                match edge {
                    BoundaryEdge::Line(e) => hatch_csv.push_str(&format!(
                        "{pi},{ei},LINE,{:.6},{:.6},{:.6},{:.6},,,,,,{flags}\n",
                        e.start.x, e.start.y, e.end.x, e.end.y
                    )),
                    BoundaryEdge::CircularArc(a) => hatch_csv_arc(
                        &mut hatch_csv,
                        pi,
                        ei,
                        a.center.x,
                        a.center.y,
                        a.radius,
                        a.start_angle.to_degrees(),
                        a.end_angle.to_degrees(),
                        flags,
                    ),
                    other => panic!("剖面边界只应有 LINE/ARC，得到 {other:?}"),
                }
            }
        }
        std::fs::write(dir.join("section_review_hatch.csv"), &hatch_csv)
            .expect("写 section_review_hatch.csv");
        std::fs::write(
            dir.join("section_review_hatch_meta.csv"),
            format!(
                "pattern,{}\nscale,{}\nsolid,{}\npaths,{}\n",
                hatch.pattern.name,
                hatch.pattern_scale,
                hatch.is_solid,
                hatch.paths.len()
            ),
        )
        .expect("写 section_review_hatch_meta.csv");
        assert!(dir.join("section_review_regular.csv").is_file());
        assert!(dir.join("section_review_section.csv").is_file());
    }

    fn hatch_csv_arc(
        csv: &mut String,
        pi: usize,
        ei: usize,
        cx: f64,
        cy: f64,
        r: f64,
        a0: f64,
        a1: f64,
        flags: u32,
    ) {
        let sx = cx + r * a0.to_radians().cos();
        let sy = cy + r * a0.to_radians().sin();
        let ex = cx + r * a1.to_radians().cos();
        let ey = cy + r * a1.to_radians().sin();
        csv.push_str(&format!(
            "{pi},{ei},ARC,{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.4},{:.4},{flags}\n",
            sx, sy, ex, ey, cx, cy, r, a0, a1
        ));
    }

    /// 齿轮段倒角口径自检：`GEAR M3 Z20` 在轴生成器里的齿顶倒角坐标，与
    /// `gear.rs::side_view()` 同参数（m=3、z=20、h=30）的倒角/台阶部分数值一致。
    #[test]
    fn gear_segment_chamfer_matches_gear_side_view() {
        let program = parse_program("S40 E40 L20\nGEAR M3 Z20 H30").unwrap();
        let normal = build(&program, 1.0).unwrap();
        let gear = program.segments[1].gear.unwrap();
        let params = gear.params();
        assert!(near(gear.chamfer(), 2.0), "C = round(0.6×3) = 2");
        // gear.rs 侧视图局部坐标 → 轴里齿轮段 [20,50]：平移 +(20 + h/2) = +35
        let view = crate::gear::side_view(&params, 1.0).unwrap();
        let dx = 20.0 + params.h / 2.0;
        let mut matched = 0;
        for entity in &view {
            if let EntityType::Line(l) = entity {
                if l.common.layer != LAYER_MAIN {
                    continue;
                }
                let a = [l.start.x + dx, l.start.y];
                let b = [l.end.x + dx, l.end.y];
                // 侧视图端面线 ±(ra−C) 在轴上只到邻段半径（20），不是全高；台阶/齿顶/倒角全高可比。
                let is_end_face = near(l.start.x.abs(), params.h / 2.0)
                    && near(l.end.x.abs(), params.h / 2.0)
                    && (l.start.y - l.end.y).abs() > 1e-9;
                if is_end_face {
                    continue;
                }
                assert!(
                    has_line(&normal, a, b),
                    "gear.rs::side_view() 轮廓线 {a:?}–{b:?} 应在轴齿轮段里"
                );
                matched += 1;
            }
        }
        assert_eq!(matched, 8, "齿顶面 2 + 倒角 4 + 台阶线 2");
        // 台阶竖线（仅常规视图）：x=22/48、±ra
        assert!(has_line(&normal, [22.0, -33.0], [22.0, 33.0]));
        assert!(has_line(&normal, [48.0, -33.0], [48.0, 33.0]));
        // 剖视不画台阶线，但齿根线 + 端面到 ra−C 在
        let section = build(
            &parse_program("S40 E40 L20\nGEAR M3 Z20 H30 | VIEW 剖视").unwrap(),
            1.0,
        )
        .unwrap();
        assert!(!has_line(&section, [22.0, -33.0], [22.0, 33.0]));
        assert!(has_line(&section, [20.0, 20.0], [20.0, 31.0]));
        assert!(has_line(&section, [20.0, 26.25], [50.0, 26.25]));
    }

    /// `OCSMSHAFT … REPORT`：渐开线花键段应带「花键参数表（GB/T 3478）」节，
    /// 与 `OCSMCARD 花键参数表` 同一份 `spline_tol::compute()` 结果（不许两处各算一遍）。
    #[test]
    fn report_includes_spline_parameter_table_section() {
        let p = parse_program("SPLINE EX M3 Z20 ALPHA30 X0 DA63 DF54.6 BETA0 H30").unwrap();
        let md = build_report(&p).unwrap();
        assert!(md.contains("## 2. 花键参数表（GB/T 3478"), "{md}");
        assert!(
            md.contains("(外)齿形角") && md.contains("(外)公法线长度"),
            "花键节应含外花键 21 项：{md}"
        );
        assert!(md.contains("同一份 `spline_tol::compute()`"), "{md}");
        // 普通圆柱段 → 无该节
        let p2 = parse_program("S30 E30 L20").unwrap();
        let md2 = build_report(&p2).unwrap();
        assert!(!md2.contains("花键参数表"), "{md2}");
    }

    /// `… report` 命令入口解析层：摘关键字 → 解析 → 段清单计算书。
    /// 渐开线花键段已撤（轴段不再有 INVOLSPLINE），计算书只输出段清单。
    #[test]
    fn report_keyword_parses_and_builds_segment_list() {
        let raw = "SPLINE 6x23x26x6 L30 report";
        let (clean, want, out) = crate::gear::split_report_args(raw);
        assert!(want && out.is_none());
        assert!(!clean.to_lowercase().contains("report"), "{clean}");
        let program = parse_program(&clean).unwrap();
        let md = build_report(&program).unwrap();
        assert!(md.contains("# 轴段计算书"), "{md}");
        assert!(md.contains("矩形花键段") && md.contains("N=6"), "{md}");
        assert!(!md.contains("渐开线花键"), "计算书不应再有渐开线花键段：{md}");
    }

    /// 护栏：轴生成器的 INVOLSPLINE 段已撤 —— DSL/JSON 两侧都不再接受，
    /// 序列化也不会再出现 `invol_spline` 字段（渐开线花键唯一入口 = OCSMGEAR 花键模式）。
    #[test]
    fn involspline_shaft_segment_removed() {
        let e = parse_program("INVOLSPLINE GB30R M3 Z20 L30").unwrap_err();
        assert!(e.contains("INVOLSPLINE") && e.contains("不识别的关键字"), "{e}");
        let e = parse_program(
            r#"{"segments":[{"invol_spline":{"code":"GB30R","m":3,"z":20,"len":30}}]}"#,
        )
        .unwrap_err();
        assert!(e.contains("缺少 S") || e.contains("缺少 L"), "{e}");
        let json = serde_json::to_string(&parse_program("SPLINE 6x23x26x6 L30").unwrap()).unwrap();
        assert!(!json.contains("invol_spline"), "{json}");
        // GEAR 段不受牵连，照常可用。
        let gear = parse_program("GEAR M3 Z20 H30").unwrap();
        assert!(gear.segments[0].gear.is_some());
    }

    // ══════════════════════════════════════════════════════════════════════
    // ══════════════════════════════════════════════════════════════════════
    // 轴槽（GB/T 1095）测试：表 / DSL / 模板逐图元 / sagitta / 三型×两位置
    // ══════════════════════════════════════════════════════════════════════

    /// 模板四视图锚点（`place()` 对齐用）：侧/剖视左端面 x 与轴线 y。
    const ANCHOR_SIDE_MID: [f64; 2] = [2.492_722_837_231_980_3, 13.533_672];
    const ANCHOR_SIDE_END: [f64; 2] = [110.706_968_081_815_19, 13.533_672];
    const ANCHOR_SEC_MID: [f64; 2] = [2.492_722_837_231_980_3, -59.685_371_414_118_96];
    const ANCHOR_SEC_END: [f64; 2] = [110.706_968_081_815_19, -59.685_371_414_118_96];

    /// 模板轴（与 `轴生成器-普通平键.dxf` 同几何）：d25×40（左端 C2）+ d30×30。
    /// 中置用 A 键 L=18 → 槽长 18（模板左上/左下）；端置用 C 键 L=14 → 槽长 18（模板右上/右下）。
    fn template_shaft_mid(kind: KeyKind) -> Program {
        parse_program(&format!(
            "S25 E25 L40 CH2@L KEY {} 18 b8h7 | S30 E30 L30",
            kind.code()
        ))
        .unwrap()
    }
    fn template_shaft_end(kind: KeyKind) -> Program {
        parse_program(&format!(
            "S25 E25 L40 CH2@L KEY {} 14 @端 b8h7 | S30 E30 L30",
            kind.code()
        ))
        .unwrap()
    }

    #[derive(Debug, Clone)]
    struct GoldenRow {
        view: String,
        kind: String,
        layer: String,
        a: [f64; 2],
        b: [f64; 2],
        arc: Option<([f64; 2], f64, f64, f64)>,
    }

    fn golden_rows() -> Vec<GoldenRow> {
        let text = include_str!("../tests/fixtures/keyway_template_golden.csv");
        let mut rows = Vec::new();
        for (i, line) in text.lines().enumerate() {
            if i == 0 || line.trim().is_empty() {
                continue;
            }
            let f: Vec<&str> = line.trim().split(',').collect();
            assert!(f.len() >= 12, "golden 行 {i} 列数不对：{line}");
            let num = |k: usize| -> f64 { f[k].parse().unwrap_or(f64::NAN) };
            let arc = (f[1] == "ARC").then(|| ([num(7), num(8)], num(9), num(10), num(11)));
            rows.push(GoldenRow {
                view: f[0].to_string(),
                kind: f[1].to_string(),
                layer: f[2].to_string(),
                a: [num(3), num(4)],
                b: [num(5), num(6)],
                arc,
            });
        }
        rows
    }

    /// 模板坐标容差：模板自身有 ~2e-6 级建模偏差（如 C1 键端竖线 x=114.7069702
    /// vs 端面+t₁=114.7069681），取 1e-5；sagitta 数值断言另有 1e-6。
    const GOLD_TOL: f64 = 1e-5;

    fn near_gold(a: f64, b: f64) -> bool {
        (a - b).abs() < GOLD_TOL
    }

    fn near_pt(p: [f64; 2], q: [f64; 2]) -> bool {
        near_gold(p[0], q[0]) && near_gold(p[1], q[1])
    }

    fn segment_eq(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
        (near_pt(a, c) && near_pt(b, d)) || (near_pt(a, d) && near_pt(b, c))
    }

    fn line_hit(entities: &[EntityType], a: [f64; 2], b: [f64; 2], layer: &str) -> bool {
        entities.iter().any(|e| match e {
            EntityType::Line(l) => {
                segment_eq([l.start.x, l.start.y], [l.end.x, l.end.y], a, b)
                    && l.common.layer == layer
            }
            _ => false,
        })
    }

    fn arc_hit(
        entities: &[EntityType],
        c: [f64; 2],
        r: f64,
        a0: f64,
        a1: f64,
        layer: &str,
    ) -> bool {
        entities.iter().any(|e| match e {
            EntityType::Arc(arc) => {
                near_gold(arc.center.x, c[0])
                    && near_gold(arc.center.y, c[1])
                    && near_gold(arc.radius, r)
                    && (normalize_deg(arc.start_angle.to_degrees()) - normalize_deg(a0)).abs() < 1e-3
                    && (normalize_deg(arc.end_angle.to_degrees()) - normalize_deg(a1)).abs() < 1e-3
                    && arc.common.layer == layer
            }
            _ => false,
        })
    }

    fn hatch_edges(entities: &[EntityType]) -> Vec<[f64; 4]> {
        let mut out = Vec::new();
        for e in entities {
            if let EntityType::Hatch(h) = e {
                for path in &h.paths {
                    for edge in &path.edges {
                        if let BoundaryEdge::Line(l) = edge {
                            out.push([l.start.x, l.start.y, l.end.x, l.end.y]);
                        }
                    }
                }
            }
        }
        out
    }

    fn count_kind(entities: &[EntityType], what: &str) -> usize {
        entities
            .iter()
            .filter(|e| match what {
                "LINE" => matches!(e, EntityType::Line(_)),
                "ARC" => matches!(e, EntityType::Arc(_)),
                "HATCH" => matches!(e, EntityType::Hatch(_)),
                _ => false,
            })
            .count()
    }

    /// 逐图元对模板：该视簇的 LINE/ARC/HATCH 边界逐条命中 + 数量零多零少。
    fn assert_view_matches_template(
        program: &Program,
        view: ShaftView,
        anchor: [f64; 2],
        tag: &str,
        label: &str,
    ) {
        let mut p = program.clone();
        p.view = view;
        let shaft = build(&p, 1.0).unwrap_or_else(|e| panic!("{label} 生成失败：{e}"));
        let entities = super::place(shaft.entities, anchor, 0.0);
        let gold: Vec<GoldenRow> = golden_rows().into_iter().filter(|r| r.view == tag).collect();
        assert!(!gold.is_empty(), "{label}：golden 分簇 {tag} 为空");
        let gold_lines = gold.iter().filter(|r| r.kind == "LINE").count();
        let gold_arcs = gold.iter().filter(|r| r.kind == "ARC").count();
        let gold_hatch_edges = gold.iter().filter(|r| r.kind == "HATCH").count();
        assert_eq!(count_kind(&entities, "LINE"), gold_lines, "{label}：LINE 数");
        assert_eq!(count_kind(&entities, "ARC"), gold_arcs, "{label}：ARC 数");
        assert_eq!(
            count_kind(&entities, "HATCH"),
            usize::from(gold_hatch_edges > 0),
            "{label}：HATCH 片数"
        );
        for row in &gold {
            match row.kind.as_str() {
                "LINE" => assert!(
                    line_hit(&entities, row.a, row.b, &row.layer),
                    "{label}：缺模板线 {:?}-{:?}（{}）",
                    row.a,
                    row.b,
                    row.layer
                ),
                "ARC" => {
                    let (c, r, a0, a1) = row.arc.expect("ARC 行有弧参数");
                    assert!(
                        arc_hit(&entities, c, r, a0, a1, &row.layer),
                        "{label}：缺模板弧 c={c:?} r={r} {a0}->{a1}"
                    );
                }
                "HATCH" => {}
                other => panic!("{label}：golden kind 未知 {other}"),
            }
        }
        let got = hatch_edges(&entities);
        assert_eq!(got.len(), gold_hatch_edges, "{label}：HATCH 边界边数");
        for row in gold.iter().filter(|r| r.kind == "HATCH") {
            assert!(
                got.iter().any(|g| segment_eq(
                    [g[0], g[1]],
                    [g[2], g[3]],
                    row.a,
                    row.b
                )),
                "{label}：缺模板 HATCH 边 {:?}-{:?}",
                row.a,
                row.b
            );
        }
        if gold_hatch_edges > 0 {
            for e in entities.iter() {
                if let EntityType::Hatch(h) = e {
                    assert!(near(h.pattern_scale, 1.0), "{label}：hatch scale");
                    assert!(near(h.pattern_angle.to_degrees(), 0.0), "{label}：hatch angle");
                    assert_eq!(h.paths.len(), 2, "{label}：hatch 上下两环");
                }
            }
        }
    }

    #[test]
    fn keyway_model_dsl_table_and_data_reuse() {
        let p = parse_program("S25 E25 L40 CH2@L KEY A 18 b8h7").unwrap();
        let kw = p.segments[0].keyway.expect("KEY 解析出轴槽");
        assert_eq!(kw.kind, KeyKind::A);
        assert!(near(kw.l, 18.0) && near(kw.slot_len(4.0, 8.0), 18.0));
        assert_eq!(kw.place, KeywayPlace::Mid);
        assert!(near(kw.b.unwrap(), 8.0), "显式 b8 覆盖");
        assert!(near(keyway_b(&kw, 25.0).unwrap(), 8.0), "d25 档标准 b=8");
        assert!(near(keyway_t1(8.0, None).unwrap(), 4.0), "b8 → t1=4");
        // b 省略 = 按轴段直径自动定（主路径）。
        let kw_auto = parse_program("S25 E25 L40 CH2@L KEY A 18")
            .unwrap()
            .segments[0]
            .keyway
            .unwrap();
        assert_eq!(kw_auto.b, None);
        assert!(near(keyway_b(&kw_auto, 25.0).unwrap(), 8.0), "d25 → b8（GB/T 1095 d 列）");
        // 平键族数据复用：b 必须在所选型别表里（h 配对），L 走族表系列与 L<10b。
        assert!(near(
            crate::partgen_keys::key_1096_h(KeyKind::A.key_type(), 8.0).unwrap(),
            7.0
        ));
        crate::partgen_keys::check_length_1096(8.0, 18.0).unwrap();
        assert!(crate::partgen_keys::check_length_1096(8.0, 400.0).is_err());
        // 键长别名 `KL18`；b 可只给 b（h 跟 b 走）；t1 覆盖。
        let p = parse_program("S25 E25 L40 KEY A KL18 b10 t1 5").unwrap();
        let kw = p.segments[0].keyway.unwrap();
        assert!(near(kw.l, 18.0) && near(kw.b.unwrap(), 10.0) && near(kw.t1.unwrap(), 5.0));
        // h 给了就必须配对（b8→h7）；不配对明确报错。
        let e = parse_program("S25 E25 L40 KEY A 18 b8h10").unwrap_err();
        assert!(e.contains("不是标准配对") && e.contains("h=7"), "{e}");
        // 端置：键长 L + t1 = 槽长（模板 LC 口径）；b8h7。
        let kw = parse_program("S25 E25 L40 KEY C 14 @端 b8h7")
            .unwrap()
            .segments[0]
            .keyway
            .unwrap();
        assert_eq!(kw.place, KeywayPlace::End);
        assert!(near(kw.b.unwrap(), 8.0));
        assert!(near(keyway_t1(kw.b.unwrap(), None).unwrap(), 4.0));
        assert!(near(kw.slot_len(4.0, 8.0), 18.0), "14 + 4 = 18（C 端置：折算长 = 键长）");
        // 轴径选型表（**选型依据**）：26 档；d=6 → b2、d=25 → b8；d 太小 → None。
        assert_eq!(crate::partgen_keys::key_1096_shaft_ranges().len(), 26);
        assert!(near(crate::partgen_keys::key_1096_b_for_shaft(6.0).unwrap(), 2.0));
        assert!(near(crate::partgen_keys::key_1096_b_for_shaft(25.0).unwrap(), 8.0));
        assert!(crate::partgen_keys::key_1096_b_for_shaft(4.0).is_none());
        // GB/T 1095 表：26 档、b=100 t2=19.5（官方修正）、b=14 t1=5.5；三族 × 6 档交叉可查。
        let rows = keyway_gb1095_rows().unwrap();
        assert_eq!(rows.len(), 26);
        let b100 = rows.iter().find(|r| near(r.b, 100.0)).unwrap();
        assert!(near(b100.t2, 19.5) && near(b100.t1, 31.0));
        let b14 = rows.iter().find(|r| near(r.b, 14.0)).unwrap();
        assert!(near(b14.t1, 5.5), "b=14 t1=5.5（主源正确/164580 误印 5）");
        assert!(keyway_row(7.0).is_err(), "非标准 b=7 应报错");
        for ty in [
            crate::partgen_keys::KeyType::A,
            crate::partgen_keys::KeyType::B,
            crate::partgen_keys::KeyType::C,
        ] {
            for b in [2.0, 5.0, 8.0, 14.0, 25.0, 50.0] {
                assert!(crate::partgen_keys::key_1096_h(ty, b).is_some(), "族表缺 b={b}");
                assert!(keyway_t1(b, None).is_ok(), "1095 表缺 b={b}");
            }
        }
        // JSON 往返（type/b/l/place/t1）；h 可选配对校验；旧字段/缺 b 明确报错。
        let p = template_shaft_mid(KeyKind::A);
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"type\":\"A\"") && json.contains("\"b\":8"), "{json}");
        let back = parse_program(&json).unwrap();
        assert_eq!(back.segments[0].keyway, p.segments[0].keyway);
        let p = parse_program(
            r#"{"segments":[{"s":25,"e":25,"l":40,"keyway":{"type":"C","l":14,"place":"end","b":8,"h":7}}]}"#,
        )
        .unwrap();
        let kw = p.segments[0].keyway.unwrap();
        assert_eq!(kw.kind, KeyKind::C);
        assert_eq!(kw.place, KeywayPlace::End);
        // 缺 b → 合法（b 由轴段直径自动定）；显式 b×h 不配对才报错。
        let p = parse_program(
            r#"{"segments":[{"s":25,"e":25,"l":40,"keyway":{"type":"A","l":18}}]}"#,
        )
        .unwrap();
        assert_eq!(p.segments[0].keyway.unwrap().b, None, "JSON 省略 b = 按轴径自动定");
        // b×h 不配对 → 报错。
        let e = parse_program(
            r#"{"segments":[{"s":25,"e":25,"l":40,"keyway":{"type":"A","l":18,"b":8,"h":10}}]}"#,
        )
        .unwrap_err();
        assert!(e.contains("不是标准配对"), "{e}");
        // 旧 JSON（无 type / 带 la）→ 明确报错。
        assert!(parse_program(
            r#"{"segments":[{"s":25,"e":25,"l":40,"keyway":{"b":8,"l":18,"place":"mid"}}]}"#
        )
        .is_err());
        assert!(parse_program(
            r#"{"segments":[{"s":25,"e":25,"l":40,"keyway":{"type":"A","la":18}}]}"#
        )
        .is_err());
    }

    #[test]
    fn keyway_errors_are_explicit() {
        // 旧写法 LA/LC 明确报错（不静默）。
        let e = parse_program("S25 E25 L40 KEY b8 LA18").unwrap_err();
        assert!(e.contains("旧") && e.contains("LA"), "{e}");
        // 缺键型/键长；非法键型/位置；b×h 不配对（b 可省 = 按轴径自动定）。
        assert!(parse_program("S25 E25 L40 KEY 18 b8")
            .unwrap_err()
            .contains("缺少键型"));
        assert!(parse_program("S25 E25 L40 KEY A b8h7")
            .unwrap_err()
            .contains("缺少键长"));
        assert!(parse_program("S25 E25 L40 KEY D 18 b8")
            .unwrap_err()
            .contains("不识别的关键字"));
        assert!(parse_program("S25 E25 L40 KEY A 18 b8 @斜")
            .unwrap_err()
            .contains("位置"));
        assert!(parse_program("S25 E25 L40 KEY A 18 b8h10")
            .unwrap_err()
            .contains("不是标准配对"));
        // 与齿轮/花键/螺纹/越程槽同段。
        for text in [
            "GEAR M3 Z20 KEY A 18 b8h7",
            "SPLINE 6x23x26x6 L30 KEY A 18 b8h7",
            "S25 E25 L40 M KEY A 18 b8h7",
            "S25 E25 L40 OV3 KEY A 18 b8h7",
        ] {
            assert!(parse_program(text).is_err(), "{text} 应报错");
        }
        // 几何：中置装不下 / 端置开中间段 / 锥面 / 单段端置 / L 非系列 / t1 < 倒角 / 槽长超段。
        let e = build(&parse_program("S25 E25 L10 KEY A 18 b8h7").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("装不下"), "{e}");
        let e = build(
            &parse_program("S25 E25 L10 | S25 E25 L40 KEY C 14 @端 b8h7 | S25 E25 L10").unwrap(),
            1.0,
        )
        .unwrap_err();
        assert!(e.contains("只能开在首段"), "{e}");
        let e = build(&parse_program("S25 E20 L40 KEY A 10 b8h7").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("圆柱段"), "{e}");
        let e = build(&parse_program("S25 E25 L40 KEY C 14 @端 b8h7").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("单段轴"), "{e}");
        let e = build(&parse_program("S25 E25 L40 KEY A 17 b8h7").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("L 取值范围") || e.contains("系列"), "{e}");
        let e = build(
            &parse_program("S25 E25 L40 CH5@L KEY C 14 @端 b8h7 | S30 E30 L30").unwrap(),
            1.0,
        )
        .unwrap_err();
        assert!(e.contains("倒角"), "{e}");
        let e = build(
            &parse_program("S25 E25 L16 KEY A 14 @端 b8h7 | S30 E30 L30").unwrap(),
            1.0,
        )
        .unwrap_err();
        assert!(e.contains("段长"), "{e}");
        // b 超出 GB/T 1096 平键族（b≤50）。
        let e = parse_program("S25 E25 L40 KEY A 18 b56h32").unwrap_err();
        assert!(e.contains("没有 b=56"), "{e}");
        // 显式 b 必须落在该轴径档的标准配对上（d25 → b8；给 b10 明确报错，不许静默接受）。
        let e = build(&parse_program("S25 E25 L40 KEY A 18 b10h8").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("应配 b=8") && e.contains("不许自由组合"), "{e}");
        let e = build(&parse_program("S25 E25 L40 KEY A 18 b8").unwrap(), 1.0);
        assert!(e.is_ok(), "d25 + b8 是标准配对，应通过：{e:?}");
        // 轴径不在 GB/T 1095 d 选型表（4 < 6）→ 明确报错。
        let e = build(&parse_program("S4 E4 L12 KEY A 8").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("d 选型表"), "{e}");
    }

    #[test]
    fn keyway_bxh_from_shaft_diameter_26_bands() {
        // GB/T 1095 d 列 26 档：每个区间的代表 d → 标准 b（h 由平键族按 b 配对）。
        let rows = crate::partgen_keys::key_1096_shaft_ranges();
        assert_eq!(rows.len(), 26, "GB/T 1095 d 列 26 档");
        for (lo, hi, incl, b) in rows {
            let d = if incl { lo } else { (lo + hi) / 2.0 };
            let got = crate::partgen_keys::key_1096_b_for_shaft(d)
                .unwrap_or_else(|| panic!("d={d} 应命中区间 {lo}~{hi}"));
            assert!(near(got, b), "d={d} 应配 b={b}，实为 {got}");
            if b <= 50.0 {
                assert!(
                    crate::partgen_keys::key_1096_h(crate::partgen_keys::KeyType::A, b).is_some(),
                    "b={b} 应在平键族表内"
                );
            }
        }
        // 区间左端口径：6 含在首档；8 仍在 >6~8（b2）；10 → b3。
        assert!(near(crate::partgen_keys::key_1096_b_for_shaft(6.0).unwrap(), 2.0));
        assert!(near(crate::partgen_keys::key_1096_b_for_shaft(8.0).unwrap(), 2.0));
        assert!(near(crate::partgen_keys::key_1096_b_for_shaft(10.0).unwrap(), 3.0));
        // 该档标准配对显式给：通过；邻档 b：明确报错（不静默接受）。
        let p = parse_program("S25 E25 L40 KEY A 18 b8h7").unwrap();
        assert!(near(
            keyway_b(p.segments[0].keyway.as_ref().unwrap(), 25.0).unwrap(),
            8.0
        ));
        let p = parse_program("S25 E25 L40 KEY A 18 b10h8").unwrap();
        let e = build(&p, 1.0).unwrap_err();
        assert!(e.contains("应配 b=8") && e.contains("不许自由组合"), "{e}");
        // 每档都能按轴径自动定出 b（不显式给 b 也合法）。
        let p = parse_program("S25 E25 L40 CH2@L KEY A 18").unwrap();
        assert_eq!(p.segments[0].keyway.unwrap().b, None);
        assert!(build(&p, 1.0).is_ok());
    }

    #[test]
    fn keyway_sagitta_is_geometric_not_t1_shift() {
        // 剖视中置：sagitta 线 y = R − √(R²−(b/2)²)，与 t₁ 无关；槽底 = R − t₁。
        let mut p = template_shaft_mid(KeyKind::A);
        p.view = ShaftView::Section;
        let shaft = build(&p, 1.0).unwrap();
        let (r, b, t1) = (12.5_f64, 8.0_f64, 4.0_f64);
        let sag = r - (r * r - (b / 2.0) * (b / 2.0)).sqrt();
        // 模板实测沉降 0.657280718 vs 公式 0.657280701，差 1.6e-8。
        assert!((sag - 0.657_280_701).abs() < 1e-6, "sag={sag}");
        let y_sag = r - sag;
        assert!(line_hit(&shaft.entities, [12.0, y_sag], [30.0, y_sag], LAYER_MAIN), "sagitta 线");
        assert!(line_hit(&shaft.entities, [12.0, r - t1], [30.0, r - t1], LAYER_MAIN), "槽底线");
        // 不许画成“整段下移 t₁”：槽底相对顶线是 t₁，但 sagitta 线只下沉 sag。
        assert!(!near(y_sag, r - t1), "sagitta 线 ≠ 槽底");
        assert!(near(y_sag - (r - t1), t1 - sag), "t₁ − sag");
    }

    #[test]
    fn keyway_side_template_two_key_shapes() {
        // 左上 = A 型（双圆头）中置；右上 = C 型（单圆头）端置。
        assert_view_matches_template(
            &template_shaft_mid(KeyKind::A),
            ShaftView::Normal,
            ANCHOR_SIDE_MID,
            "side_mid",
            "左上 常规·A 中置",
        );
        assert_view_matches_template(
            &template_shaft_end(KeyKind::C),
            ShaftView::Normal,
            ANCHOR_SIDE_END,
            "side_end",
            "右上 常规·C 端置",
        );
    }

    #[test]
    fn keyway_section_template_keeps_golden() {
        // 模板剖视黄金标准：左下 = A 中置（键长 18，槽 18）；右下 = C 端置（键长 14，槽 18）。
        // 新折算口径下 B 的槽长会不同（按型别换算），不再与模板同簇。
        assert_view_matches_template(
            &template_shaft_mid(KeyKind::A),
            ShaftView::Section,
            ANCHOR_SEC_MID,
            "sec_mid",
            "左下 剖视·A 中置",
        );
        assert_view_matches_template(
            &template_shaft_end(KeyKind::C),
            ShaftView::Section,
            ANCHOR_SEC_END,
            "sec_end",
            "右下 剖视·C 端置",
        );
    }

    #[test]
    fn keyway_other_combinations_are_coherent() {
        // C 中置（新口径）：显示 A（两弧）→ 折算 L_eff = L_C + b/2；模板轴 d25：18 + 4 = 22。
        let mut p = template_shaft_mid(KeyKind::C);
        p.view = ShaftView::Normal;
        let shaft = build(&p, 1.0).unwrap();
        assert_eq!(count_kind(&shaft.entities, "ARC"), 2, "C 中置显示 A（两弧）");
        assert!(arc_hit(&shaft.entities, [14.0, 0.0], 4.0, 90.0, 270.0, LAYER_MAIN), "C 折算后 A 左弧");
        assert!(arc_hit(&shaft.entities, [28.0, 0.0], 4.0, 270.0, 450.0, LAYER_MAIN), "C 折算后 A 右弧");
        // A 端置：双圆头（2 弧），A 不折算。
        let mut p = template_shaft_end(KeyKind::A);
        p.view = ShaftView::Normal;
        assert_eq!(count_kind(&build(&p, 1.0).unwrap().entities, "ARC"), 2, "A 端 2 弧");
        // 平端偏置/40° 跨度 = t₁（≠b/2 的探针：d45 → b14、t1=5.5、b/2=7；C 端置）。
        let probe = parse_program("S45 E45 L80 KEY C 32 @端 b14h9 | S20 E20 L10").unwrap();
        let mut p = probe.clone();
        p.view = ShaftView::Normal;
        let shaft = build(&p, 1.0).unwrap();
        let (rk, t1) = (7.0_f64, 5.5_f64);
        let dy = rk * 20.0_f64.to_radians().tan();
        assert!(line_hit(&shaft.entities, [t1, rk], [37.5 - rk, rk], LAYER_MAIN), "平端在 +t1");
        assert!(line_hit(&shaft.entities, [t1, rk], [0.0, rk + dy], "0"), "40° x 跨度 = t1");
        let mut p = probe;
        p.view = ShaftView::Section;
        let shaft = build(&p, 1.0).unwrap();
        let (r, sag) = (22.5_f64, 22.5 - (22.5_f64.powi(2) - rk * rk).sqrt());
        assert!(line_hit(&shaft.entities, [t1, r - t1], [t1, r - sag], LAYER_MAIN), "键端竖线在 t1");
    }

    #[test]
    fn keyway_centerline_overhang_matches_axis() {
        // 跨模块一致：键中心线伸出 = 轴线伸出 3×frame_scale（既有口径）。
        let p = template_shaft_mid(KeyKind::A);
        let shaft = build(&p, 2.0).unwrap();
        // 键横中：槽 [12,30] → [6, 36]；轴中：总长 70 → [-6, 76]。
        assert!(line_hit(&shaft.entities, [6.0, 0.0], [36.0, 0.0], LAYER_CENTER), "键横中");
        assert!(line_hit(&shaft.entities, [-6.0, 0.0], [76.0, 0.0], LAYER_CENTER), "轴中");
        // 键竖中：A 型弧心 ±(b/2+6)。
        assert!(line_hit(&shaft.entities, [16.0, -10.0], [16.0, 10.0], LAYER_CENTER), "键竖中");
    }

    // ── 显示/折算（#1）+ 端置剖视端面闭合/包络（#2）──────────────

    #[test]
    fn keyway_display_rules_abc_and_bc_length_conversion() {
        // 用户口径：中置常规侧视**恒显示 A**（所选 A/B/C 都画跑道形）；端置 B/C **显示 C**；
        // 实际槽长按所选键型折算（圆弧半径 b/2 吃直段）。
        // d25 → b8、t1=4；槽心 = 段中（L40 无倒角 → 20）。
        // 中置 A：显示 A、槽长 = L；直段 = L − b。
        let mut p = template_shaft_mid(KeyKind::A);
        p.view = ShaftView::Normal;
        let shaft = build(&p, 1.0).unwrap();
        assert_eq!(count_kind(&shaft.entities, "ARC"), 2, "中置 A 显示两弧");
        assert!(arc_hit(&shaft.entities, [16.0, 0.0], 4.0, 90.0, 270.0, LAYER_MAIN), "A 左弧");
        assert!(arc_hit(&shaft.entities, [26.0, 0.0], 4.0, 270.0, 450.0, LAYER_MAIN), "A 右弧");
        // 中置 B：显示仍是 A（两弧），长度折算 L_eff = L_B + b；直段 = L_B。
        let mut p = parse_program("S25 E25 L40 KEY B 10").unwrap();
        p.view = ShaftView::Normal;
        let shaft = build(&p, 1.0).unwrap();
        assert_eq!(count_kind(&shaft.entities, "ARC"), 2, "中置 B 也显示 A（两弧）");
        assert_eq!(count_kind(&shaft.entities, "HATCH"), 0);
        assert!(arc_hit(&shaft.entities, [15.0, 0.0], 4.0, 90.0, 270.0, LAYER_MAIN), "B 折算后 A 左弧（直段 10 中置）");
        assert!(arc_hit(&shaft.entities, [25.0, 0.0], 4.0, 270.0, 450.0, LAYER_MAIN), "B 折算后 A 右弧");
        // 中置 C：显示 A，折算 L_eff = L_C + b/2。
        let mut p = parse_program("S25 E25 L40 KEY C 16").unwrap();
        p.view = ShaftView::Normal;
        let shaft = build(&p, 1.0).unwrap();
        assert_eq!(count_kind(&shaft.entities, "ARC"), 2, "中置 C 也显示 A（两弧）");
        assert!(arc_hit(&shaft.entities, [14.0, 0.0], 4.0, 90.0, 270.0, LAYER_MAIN), "C 折算后 A 左弧");
        assert!(arc_hit(&shaft.entities, [26.0, 0.0], 4.0, 270.0, 450.0, LAYER_MAIN), "C 折算后 A 右弧");
        // 端置 B/C：显示 C（1 弧 + 平端 40° 线），折算 L_eff = B: L_B + b/2；C: L_C。
        let mut p = parse_program("S25 E25 L40 KEY B 10 @端 | S30 E30 L30").unwrap();
        p.view = ShaftView::Normal;
        let shaft = build(&p, 1.0).unwrap();
        assert_eq!(count_kind(&shaft.entities, "ARC"), 1, "端置 B 显示 C（1 弧）");
        assert!(arc_hit(&shaft.entities, [4.0 + 10.0, 0.0], 4.0, 270.0, 450.0, LAYER_MAIN), "B 端弧心 = t1 + L_B");
        let mut p = parse_program("S25 E25 L40 KEY C 14 @端 | S30 E30 L30").unwrap();
        p.view = ShaftView::Normal;
        let shaft = build(&p, 1.0).unwrap();
        assert_eq!(count_kind(&shaft.entities, "ARC"), 1, "端置 C 显示 C（1 弧）");
        assert!(arc_hit(&shaft.entities, [4.0 + 10.0, 0.0], 4.0, 270.0, 450.0, LAYER_MAIN), "C 端弧心 = t1 + (L_C − b/2)");
    }

    #[test]
    fn b_key_maps_to_ac_keyway_length() {
        // B 型键（双平头）折算（槽端圆弧半径 b/2 吃直段）：
        //   中置显示 A：输入 L_B → 折算长度 L_eff = L_B + b，跑道直段 = L_B；
        //   端置显示 C：输入 L_B → 折算长度 L_eff = L_B + b/2，平端到弧心 = L_B。
        for (l_b, d, b) in [(10.0_f64, 25.0_f64, 8.0_f64), (18.0, 45.0, 14.0)] {
            // 解析层：模型存的是所选键型的键长 L_B，折算在几何层。
            let kw = parse_program(&format!("S{d} E{d} L80 KEY B {l_b}")).unwrap().segments[0]
                .keyway
                .unwrap();
            let b_auto = keyway_b(&kw, d).unwrap();
            assert!(near(b_auto, b));
            assert!(near(kw.l, l_b), "模型保留输入键长");
            assert!(near(kw.effective_len(b_auto) - b_auto, l_b), "中置折算：直段 = L_B");
            // 几何：中置显示 A，弧心距 = L_B（槽心 40）。
            let mut p = parse_program(&format!("S{d} E{d} L80 KEY B {l_b}")).unwrap();
            p.view = ShaftView::Normal;
            let shaft = build(&p, 1.0).unwrap();
            assert!(arc_hit(&shaft.entities, [40.0 - l_b / 2.0, 0.0], b / 2.0, 90.0, 270.0, LAYER_MAIN));
            assert!(arc_hit(&shaft.entities, [40.0 + l_b / 2.0, 0.0], b / 2.0, 270.0, 450.0, LAYER_MAIN));
            // 端置 B：折算 L_eff = L_B + b/2；弧心 = t1 + L_B。
            let t1 = keyway_t1(b_auto, None).unwrap();
            let text_end = format!("S{d} E{d} L80 KEY B {l_b} @端 | S20 E20 L10");
            let kw = parse_program(&text_end).unwrap().segments[0].keyway.unwrap();
            assert!(near(kw.effective_len(b_auto) - b_auto / 2.0, l_b), "端置折算：直段 = L_B");
            let mut p = parse_program(&text_end).unwrap();
            p.view = ShaftView::Normal;
            let shaft = build(&p, 1.0).unwrap();
            assert!(arc_hit(&shaft.entities, [t1 + l_b, 0.0], b / 2.0, 270.0, 450.0, LAYER_MAIN));
            // C 型同名口径：中置折算 +b/2；端置不折算。
            let kw_c = parse_program(&format!("S{d} E{d} L80 KEY C {l_b}")).unwrap().segments[0]
                .keyway
                .unwrap();
            assert!(near(kw_c.effective_len(b_auto) - b_auto / 2.0, l_b), "C 中置折算：平端直段 = L_B");
            let kw_c_end = parse_program(&format!("S{d} E{d} L80 KEY C {l_b} @端 | S20 E20 L10")).unwrap()
                .segments[0]
                .keyway
                .unwrap();
            assert!(near(kw_c_end.effective_len(b_auto), l_b), "C 端置不折算");
            // A 型不折算（两端圆弧语义与显示一致）。
            let kw_a = parse_program(&format!("S{d} E{d} L80 KEY A {l_b}")).unwrap().segments[0]
                .keyway
                .unwrap();
            assert!(near(kw_a.effective_len(b_auto), l_b), "A 不折算");
        }
    }

    #[test]
    fn guided_keyway_1097_matches_template_and_data() {
        // GB/T 1097 导向平键槽：模板 `轴生成器-导向平键槽.dxf`（Ø30×50、槽 25×8×4、
        // 2×M3×L0=7 固定螺纹孔，孔心距槽端 L3=6）。反解见 review/导向平键槽_几何反解.md。
        let p = parse_program("S30 E30 L50 CH2@L CH2@R KEY A 25 导向").unwrap();
        let kw = p.segments[0].keyway.as_ref().unwrap();
        assert!(kw.guided && kw.kind == KeyKind::A && near(kw.l, 25.0));
        let b = keyway_b(kw, 30.0).unwrap();
        assert!(near(b, 8.0));
        assert!(
            near(kw.slot_len(keyway_t1(b, None).unwrap(), b), 25.0),
            "导向槽长 = 键长 L（不折算）"
        );
        // 1097 表行：d0=3、L0=7、h=7；长度系列 L=25 → L1=13、L3=6。
        let row = crate::partgen_keys::key_1097_row(KeyKind::A.key_type(), b).unwrap();
        assert!(near(row.d0, 3.0) && near(row.l0, 7.0) && near(row.h, 7.0));
        let len = crate::partgen_keys::key_1097_length_for(25.0).unwrap();
        assert!(near(len.l1, 13.0) && near(len.l3, 6.0));
        // 常规侧视：A 键跑道形（弧心 16.5/33.5，R=4）+ 2 固定螺钉孔圈
        // （小径实线整圆 r=1.275 + 大径 3/4 细弧 r=1.5）。
        let mut pn = p.clone();
        pn.view = ShaftView::Normal;
        let shaft = build(&pn, 1.0).unwrap();
        assert!(arc_hit(&shaft.entities, [16.5, 0.0], 4.0, 90.0, 270.0, LAYER_MAIN));
        assert!(arc_hit(&shaft.entities, [33.5, 0.0], 4.0, 270.0, 450.0, LAYER_MAIN));
        assert_eq!(count_kind(&shaft.entities, "ARC"), 4, "常规：键 2 弧 + 孔 2 大径弧");
        assert_eq!(
            shaft.entities.iter().filter(|e| matches!(e, EntityType::Circle(_))).count(),
            2,
            "常规：孔 2 小径整圆"
        );
        for x in [18.5_f64, 31.5] {
            assert!(
                shaft.entities.iter().any(|e| matches!(e,
                    EntityType::Circle(c) if near(c.center.x, x) && near(c.center.y, 0.0)
                        && near(c.radius, 1.275) && c.common.layer == LAYER_MAIN)),
                "孔 {x} 的小径整圆"
            );
            assert!(arc_hit(&shaft.entities, [x, 0.0], 1.5, 265.0, 185.0, LAYER_THIN));
        }
        // 正/负断言：大径 3/4 弧起止角 = 模板换算 265°→185°（用户 2026-09-25 裁定）；旧的 270°→180° 不得复现。
        let thin_arcs: Vec<_> = shaft
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Arc(a) if a.common.layer == LAYER_THIN => Some(a),
                _ => None,
            })
            .collect();
        assert_eq!(thin_arcs.len(), 2, "两孔各 1 条大径细弧");
        for a in thin_arcs {
            assert!(
                (a.start_angle.to_degrees() - 265.0).abs() < 1e-9
                    && (a.end_angle.to_degrees() - 185.0).abs() < 1e-9,
                "起止角应为模板换算 265°→185°（实得 {}→{}）",
                a.start_angle.to_degrees(),
                a.end_angle.to_degrees()
            );
        }
        assert!(
            !shaft.entities.iter().any(|e| matches!(e, EntityType::Arc(a)
                if a.common.layer == LAYER_THIN
                    && (a.start_angle.to_degrees() - 270.0).abs() < 1e-9
                    && (a.end_angle.to_degrees() - 180.0).abs() < 1e-9)),
            "旧的 270°→180° 口径应已作废"
        );
        // 剖视：槽底 = 15−4 = 11；sagitta = 15−√(15²−4²) = 14.4567；
        // 孔小径实线至 y=4（L0=7）、钻尖至 3.234（118°）；大径细线至 y=5（L0−2P，P=0.5）。
        let mut ps = p.clone();
        ps.view = ShaftView::Section;
        let sec = build(&ps, 1.0).unwrap();
        let sag = 15.0 - (15.0_f64 * 15.0 - 4.0 * 4.0).sqrt();
        assert!(line_hit(&sec.entities, [12.5, 15.0], [12.5, 11.0], LAYER_MAIN), "槽左壁");
        assert!(line_hit(&sec.entities, [37.5, 11.0], [37.5, 15.0], LAYER_MAIN), "槽右壁");
        // 槽底直线：模板 ent（12.5,-40.599）→（37.5,-40.599）；旧实现漏画（用户 2026-09-25 报告）。
        assert!(
            line_hit(&sec.entities, [12.5, 11.0], [37.5, 11.0], LAYER_MAIN),
            "槽底直线（模板 12.5..37.5 @ 槽底）"
        );
        assert!(
            !line_hit(&sec.entities, [2.0, 11.0], [12.5, 11.0], LAYER_MAIN),
            "槽底线不得向槽外延伸"
        );
        // 表面线（模板断口 12.5 / 37.5）：槽外两段在、槽口断开。
        assert!(line_hit(&sec.entities, [2.0, 15.0], [12.5, 15.0], LAYER_MAIN), "表面线左段");
        assert!(line_hit(&sec.entities, [37.5, 15.0], [48.0, 15.0], LAYER_MAIN), "表面线右段");
        assert!(
            !line_hit(&sec.entities, [12.5, 15.0], [37.5, 15.0], LAYER_MAIN),
            "槽口不得有贯通表面线"
        );
        assert!(
            line_hit(&sec.entities, [12.5, 15.0 - sag], [37.5, 15.0 - sag], LAYER_MAIN),
            "sagitta 线 0.543253"
        );
        assert!(line_hit(&sec.entities, [17.225, 11.0], [17.225, 4.0], LAYER_MAIN), "小径实线 L0");
        assert!(line_hit(&sec.entities, [19.775, 11.0], [19.775, 4.0], LAYER_MAIN));
        let tip = 11.0 - 7.0 - (0.85 * 1.5) / 59f64.to_radians().tan();
        assert!(line_hit(&sec.entities, [17.225, 4.0], [18.5, tip], LAYER_MAIN), "118° 钻尖");
        assert!(line_hit(&sec.entities, [17.0, 11.0], [17.0, 5.0], LAYER_THIN), "大径细线 L0−2P");
        assert!(line_hit(&sec.entities, [17.0, 5.0], [20.0, 5.0], LAYER_THIN), "螺纹终止线");
        assert_eq!(count_kind(&sec.entities, "HATCH"), 1, "剖视剖面线 1 片");
        // JSON 往返：guided 字段保留。
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"guided\":true"), "JSON 应带 guided：{json}");
        let back = parse_program(&json).unwrap();
        assert!(back.segments[0].keyway.as_ref().unwrap().guided);
        let back = parse_program(
            r#"{"segments":[{"s":30,"l":50,"keyway":{"type":"A","l":25,"guided":true}}]}"#,
        )
        .unwrap();
        assert!(back.segments[0].keyway.as_ref().unwrap().guided);
    }

    /// 收尾（用户 2026-09-25）：键型与「导向」合并为一个下拉/记号——DSL `KEY 导向A 25`
    /// 与旧 `KEY A 25 导向` 等价；JSON `"type":"导向A"` 与旧 `type`+`guided` 等价；
    /// 合并记号仍受同一张样式表的「位置/双槽」约束。
    #[test]
    fn guided_keyway_merged_style_token_and_old_form_are_equivalent() {
        let new_p = parse_program("S30 E30 L50 CH2@L CH2@R KEY 导向A 25").unwrap();
        let old_p = parse_program("S30 E30 L50 CH2@L CH2@R KEY A 25 导向").unwrap();
        let (n, o) = (
            new_p.segments[0].keyway.as_ref().unwrap(),
            old_p.segments[0].keyway.as_ref().unwrap(),
        );
        assert_eq!(n.kind, o.kind);
        assert!(n.guided && o.guided);
        assert_eq!(n.l, o.l);
        // ASCII 别名 `GUIDED_B` 也走同一张表。
        let p = parse_program("S30 E30 L50 KEY GUIDED_B 25").unwrap();
        let kw = p.segments[0].keyway.as_ref().unwrap();
        assert!(kw.guided && kw.kind == KeyKind::B);
        // JSON 新形式 `"type":"导向A"` = 旧 `"type":"A"`+`"guided":true`。
        let jn = parse_program(r#"{"segments":[{"s":30,"l":50,"keyway":{"type":"导向A","l":25}}]}"#)
            .unwrap();
        let jo = parse_program(
            r#"{"segments":[{"s":30,"l":50,"keyway":{"type":"A","l":25,"guided":true}}]}"#,
        )
        .unwrap();
        let (jn, jo) = (
            jn.segments[0].keyway.as_ref().unwrap(),
            jo.segments[0].keyway.as_ref().unwrap(),
        );
        assert!(jn.guided && jo.guided && jn.kind == jo.kind);
        // 合并记号同一套约束：导向只有中置、与双槽互斥、表外型别列可选值。
        let build_err = |text: &str| match parse_program(text) {
            Err(e) => e,
            Ok(p) => build(&p, 1.0).unwrap_err(),
        };
        let err = build_err("S30 E30 L50 KEY 导向A 25 @端 | S20 E20 L10");
        assert!(err.contains("导向平键A型") && err.contains("只有中置"), "{err}");
        let err = build_err("S30 E30 L50 KEY 导向A 25 双槽");
        assert!(err.contains("不支持双槽"), "{err}");
        let err = build_err("S30 E30 L50 KEY 导向C 25");
        assert!(err.contains("键型「导向C」非法") && err.contains("导向平键A型"), "{err}");
        // 旧键型与合并记号冲突（A vs 导向B）。
        let err = build_err("S30 E30 L50 KEY A 25 导向B");
        assert!(err.contains("冲突"), "{err}");
    }

    #[test]
    fn guided_keyway_1097_rejects_bad_combinations() {
        let build_err = |text: &str| match parse_program(text) {
            Err(e) => e,
            Ok(p) => build(&p, 1.0).unwrap_err(),
        };
        // 导向只 A/B（1097 无 C 型）。
        let err = build_err("S30 E30 L50 KEY C 25 导向");
        assert!(err.contains("1097") && err.contains("C"), "{err}");
        // L 必须在 1097 长度系列（∩L<10b）；表外报错列选项。
        let err = build_err("S30 E30 L50 KEY A 26 导向");
        assert!(err.contains("长度系列") && err.contains("25"), "{err}");
        // 小轴（d10 → b=6）没有 1097 b 行。
        let err = build_err("S10 E10 L50 KEY A 25 导向");
        assert!(err.contains("1097") && err.contains("表里没有 b="), "{err}");
        // 导向与双槽互斥。
        let err = build_err("S30 E30 L50 KEY A 25 导向 双槽");
        assert!(err.contains("双槽"), "{err}");
        // 孔底越过轴线（d23 → b=8、R=11.5；t1=4 + L0=7 + 钻尖 0.766 > 11.5）。
        let err = build_err("S23 E23 L50 KEY A 25 导向");
        assert!(err.contains("越过轴线"), "{err}");
        // 端置：导向平键只有中置（用户 2026-09-25 裁定）——明确报错并指路，不生成几何。
        let err = build_err("S30 E30 L50 KEY A 25 导向 @端 | S20 E20 L10");
        assert!(err.contains("导向") && err.contains("只有中置"), "{err}");
        let err = build_err(
            r#"{"segments":[{"s":30,"l":50,"keyway":{"type":"A","l":25,"guided":true,"place":"end"}},{"s":20,"l":10}]}"#,
        );
        assert!(err.contains("只有中置"), "JSON 端置也应拦：{err}");
        // B 型平头键也能建（模板只给 A；B 按标准平头画 2 孔）。
        let mut p = parse_program("S30 E30 L50 CH2@L CH2@R KEY B 25 导向").unwrap();
        p.view = ShaftView::Normal;
        let shaft = build(&p, 1.0).unwrap();
        assert!(!shaft.entities.iter().any(|e| matches!(e,
            EntityType::Arc(a) if near(a.radius, 4.0) && a.common.layer == LAYER_MAIN)),
            "B 型不应有 A 型圆头弧");
    }

    #[test]
    fn keyway_section_end_face_closes_without_chamfer() {
        // #2：端置 + 剖视、无倒角端 —— 端面必须从轴底 −R 封到槽底（旧 bug：face_lo 符号错，
        // 画成 [0,+R]→[0,floor]，左上角凸出且下半端面未封闭）。
        let mut p = parse_program("S25 E25 L40 KEY C 14 @端 | S30 E30 L30").unwrap();
        p.view = ShaftView::Section;
        let shaft = build(&p, 1.0).unwrap();
        let (r, t1) = (12.5_f64, 4.0_f64);
        let floor = r - t1;
        let sag = r - (r * r - (8.0_f64 / 2.0) * (8.0 / 2.0)).sqrt();
        // 端面下半：−R → 槽底；端面上半：槽底 → sagitta（闭合由这 3 段 + 槽底/闭端壁构成）。
        assert!(line_hit(&shaft.entities, [0.0, -r], [0.0, floor], LAYER_MAIN), "端面下半封到槽底");
        assert!(line_hit(&shaft.entities, [0.0, floor], [0.0, r - sag], LAYER_MAIN), "端面上半到 sagitta");
        assert!(!line_hit(&shaft.entities, [0.0, r], [0.0, floor], LAYER_MAIN), "不应有左上角凸出线段");
    }

    #[test]
    fn keyway_section_entities_stay_inside_shaft_envelope() {
        // #2 不变量：剖视里除中心线外，所有图元（缺口/sagitta/hatch 边界）必须落在轴轮廓 bbox 内。
        let cases: &[(&str, f64, f64, f64)] = &[
            // (DSL, x_min, x_max, r_max)
            ("S25 E25 L40 KEY C 14 @端 | S30 E30 L30", 0.0, 70.0, 15.0),
            ("S25 E25 L40 CH2@L KEY C 14 @端 | S30 E30 L30", 0.0, 70.0, 15.0),
            ("S50 E50 L60 KEY C 18 @端 | S30 E30 L30", 0.0, 90.0, 25.0),
            ("S50 E50 L20 | S30 E30 L60 KEY C 18 @端", 0.0, 80.0, 25.0),
            ("S25 E25 L40 KEY A 18 | S30 E30 L30", 0.0, 70.0, 15.0),
            ("S45 E45 L60 CH3@L KEY A 25 | S25 E25 L20", 0.0, 80.0, 22.5),
            // 导向平键（槽底 + 固定螺钉孔 + 118° 钻尖）与双槽加入包络不变量。
            ("S30 E30 L50 KEY A 25 导向", 0.0, 50.0, 15.0),
            (
                "S30 E30 L40 CH2@L KEY A 25 导向 | S30 E30 L10 | S30 E30 L40 CH2@R KEY A 25 导向",
                0.0,
                90.0,
                15.0,
            ),
            ("S30 E30 L50 CH2@L CH2@R KEY A 25 双槽", 0.0, 50.0, 15.0),
        ];
        for (text, x0, x1, rmax) in cases {
            let mut p = parse_program(text).unwrap_or_else(|e| panic!("{text}: {e}"));
            p.view = ShaftView::Section;
            let shaft = build(&p, 1.0).unwrap_or_else(|e| panic!("{text}: {e}"));
            for e in &shaft.entities {
                match e {
                    EntityType::Line(l) if l.common.layer != LAYER_CENTER => {
                        for pt in [[l.start.x, l.start.y], [l.end.x, l.end.y]] {
                            assert!(
                                pt[0] >= x0 - 1e-6 && pt[0] <= x1 + 1e-6
                                    && pt[1] >= -rmax - 1e-6 && pt[1] <= rmax + 1e-6,
                                "{text}：线段端点 {pt:?} 越出轴包络 x[{x0},{x1}] y±{rmax}"
                            );
                        }
                    }
                    EntityType::Arc(a) if a.common.layer != LAYER_CENTER => {
                        assert!(
                            a.center.x - a.radius >= x0 - 1e-6
                                && a.center.x + a.radius <= x1 + 1e-6
                                && a.center.y - a.radius >= -rmax - 1e-6
                                && a.center.y + a.radius <= rmax + 1e-6,
                            "{text}：圆弧越出轴包络"
                        );
                    }
                    EntityType::Hatch(h) => {
                        for path in &h.paths {
                            for edge in &path.edges {
                                if let BoundaryEdge::Line(el) = edge {
                                    for pt in [[el.start.x, el.start.y], [el.end.x, el.end.y]] {
                                        assert!(
                                            pt[0] >= x0 - 1e-6 && pt[0] <= x1 + 1e-6
                                                && pt[1] >= -rmax - 1e-6 && pt[1] <= rmax + 1e-6,
                                            "{text}：hatch 边界 {pt:?} 越出轴包络"
                                        );
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // ── 双键槽（可选项，仅剖视体现；2026-09-23）────────────────────────

    /// HATCH 的每条 path 顶点（本用例键槽相关边界全是直线边）。
    fn hatch_path_points(h: &Hatch) -> Vec<Vec<[f64; 2]>> {
        h.paths
            .iter()
            .map(|path| {
                path.edges
                    .iter()
                    .filter_map(|e| match e {
                        BoundaryEdge::Line(l) => Some([l.start.x, l.start.y]),
                        _ => None,
                    })
                    .collect()
            })
            .collect()
    }

    /// 点是否在 path 多边形内（奇偶规则；顶点按 HATCH 环顺序给出）。
    fn point_in_polygon(p: [f64; 2], poly: &[[f64; 2]]) -> bool {
        let mut inside = false;
        let n = poly.len();
        for i in 0..n {
            let a = poly[i];
            let b = poly[(i + 1) % n];
            if ((a[1] > p[1]) != (b[1] > p[1]))
                && (p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0])
            {
                inside = !inside;
            }
        }
        inside
    }

    /// 剖面线是否覆盖该点（任意 path 内即算填充）。
    fn hatch_filled(entities: &[EntityType], p: [f64; 2]) -> bool {
        entities.iter().any(|e| match e {
            EntityType::Hatch(h) => hatch_path_points(h)
                .iter()
                .any(|poly| poly.len() >= 3 && point_in_polygon(p, poly)),
            _ => false,
        })
    }

    /// 某图元是否有关于轴线（y=0）的镜像对应图元（中心线除外）。
    fn has_mirror_counterpart(entities: &[EntityType], e: &EntityType) -> bool {
        match e {
            EntityType::Line(l) if l.common.layer != LAYER_CENTER => line_hit(
                entities,
                [l.start.x, -l.start.y],
                [l.end.x, -l.end.y],
                &l.common.layer,
            ),
            EntityType::Arc(a) if a.common.layer != LAYER_CENTER => arc_hit(
                entities,
                [a.center.x, -a.center.y],
                a.radius,
                180.0 - a.end_angle.to_degrees(),
                180.0 - a.start_angle.to_degrees(),
                &a.common.layer,
            ),
            EntityType::Hatch(h) => {
                let got = hatch_edges(entities);
                h.paths.iter().all(|path| {
                    path.edges.iter().all(|ed| match ed {
                        BoundaryEdge::Line(el) => got.iter().any(|g| {
                            segment_eq(
                                [g[0], g[1]],
                                [g[2], g[3]],
                                [el.start.x, -el.start.y],
                                [el.end.x, -el.end.y],
                            )
                        }),
                        _ => true,
                    })
                })
            }
            _ => true,
        }
    }

    #[test]
    fn keyway_double_dsl_json_and_normal_view_unchanged() {
        // DSL：`双槽` / `DOUBLE` 两种写法；JSON 往返 `"double":true`。
        let kw = parse_program("S25 E25 L40 KEY A 18 双槽").unwrap().segments[0]
            .keyway
            .unwrap();
        assert!(kw.double, "DSL 双槽");
        let kw2 = parse_program("S25 E25 L40 KEY A 18 DOUBLE").unwrap().segments[0]
            .keyway
            .unwrap();
        assert!(kw2.double, "DSL DOUBLE 别名");
        let json = serde_json::to_string(&parse_program("S25 E25 L40 KEY A 18 双槽").unwrap())
            .unwrap();
        assert!(json.contains("\"double\":true"), "{json}");
        let back = parse_program(&json).unwrap();
        assert!(back.segments[0].keyway.unwrap().double);
        let p = parse_program(
            r#"{"segments":[{"s":25,"e":25,"l":40,"keyway":{"type":"A","l":18,"double":true}}]}"#,
        )
        .unwrap();
        assert!(p.segments[0].keyway.unwrap().double);
        // **常规侧视图不受双槽影响**（用户口径：仅剖视体现）：中置/端置输出逐图元一致。
        for (single, double) in [
            ("S25 E25 L40 KEY A 18", "S25 E25 L40 KEY A 18 双槽"),
            (
                "S25 E25 L40 KEY C 14 @端 | S30 E30 L30",
                "S25 E25 L40 KEY C 14 @端 双槽 | S30 E30 L30",
            ),
        ] {
            let mut a = parse_program(single).unwrap();
            let mut b = parse_program(double).unwrap();
            a.view = ShaftView::Normal;
            b.view = ShaftView::Normal;
            let sa = build(&a, 1.0).unwrap();
            let sb = build(&b, 1.0).unwrap();
            // 逐图元集合一致（双槽只在剖视加线；常规下轮廓改由 keyway_normal 承载，
            // 与单槽共用 entities 的差别仅在拼接顺序，故按排序后比较）。
            let mut la: Vec<String> = entities_csv(&sa.entities).lines().map(String::from).collect();
            let mut lb: Vec<String> = entities_csv(&sb.entities).lines().map(String::from).collect();
            la.sort();
            lb.sort();
            assert_eq!(la, lb, "常规视图不应因双槽改变：{single}");
        }
    }

    #[test]
    fn keyway_double_section_is_mirror_symmetric() {
        // 剖视：双槽绕轴心 180° 对置 → 除中心线外每个图元都有 y→−y 的镜像对应。
        let cases = [
            "S25 E25 L40 KEY A 18 双槽",
            "S25 E25 L40 CH2@L KEY C 14 @端 双槽 | S30 E30 L30",
            "S30 E30 L20 | S25 E25 L40 KEY B 10 @端 双槽",
        ];
        for text in cases {
            let mut p = parse_program(text).unwrap();
            p.view = ShaftView::Section;
            let shaft = build(&p, 1.0).unwrap();
            for e in &shaft.entities {
                assert!(
                    has_mirror_counterpart(&shaft.entities, e),
                    "{text}：图元缺镜像对应：{e:?}"
                );
            }
            // 镜像对至少成对出现（说明确实多出一套槽线）。
            let lines = count_kind(&shaft.entities, "LINE");
            assert!(lines > 0);
        }
    }

    #[test]
    fn keyway_double_hatch_partition_invariants() {
        // 剖面线分区：中带（两槽之间）必须有剖面线；两槽区内不得有；两侧材料必须有。
        // d25：R=12.5、b=8、t1=4 → floor=8.5、y_sag≈11.843；中置槽 [12,30]；端置 wall=18。
        let cases: &[(&str, [f64; 2], [f64; 2], [f64; 2], [f64; 2], [f64; 2])] = &[
            // (DSL, 中带点, 上槽内点, 下槽内点, 上材料点, 下材料点)
            (
                "S25 E25 L40 KEY A 18 双槽",
                [20.0, 1.0],
                [20.0, 10.5],
                [20.0, -10.5],
                [8.0, 11.0],
                [8.0, -11.0],
            ),
            (
                "S25 E25 L40 KEY C 14 @端 双槽 | S30 E30 L30",
                [10.0, 1.0],
                [10.0, 10.5],
                [10.0, -10.5],
                [25.0, 11.0],
                [25.0, -11.0],
            ),
            (
                "S25 E25 L40 CH2@L KEY C 14 @端 双槽 | S30 E30 L30",
                [10.0, 1.0],
                [10.0, 10.5],
                [10.0, -10.5],
                [25.0, 11.0],
                [25.0, -11.0],
            ),
            (
                "S30 E30 L20 | S25 E25 L40 KEY C 14 @端 双槽",
                [50.0, 1.0],
                [50.0, 10.5],
                [50.0, -10.5],
                [30.0, 11.0],
                [30.0, -11.0],
            ),
        ];
        for (text, mid_band, top_slot, bot_slot, top_mat, bot_mat) in cases {
            let mut p = parse_program(text).unwrap();
            p.view = ShaftView::Section;
            let shaft = build(&p, 1.0).unwrap();
            assert!(hatch_filled(&shaft.entities, *mid_band), "{text}：中带漏填充");
            assert!(!hatch_filled(&shaft.entities, *top_slot), "{text}：上槽区不得有剖面线");
            assert!(!hatch_filled(&shaft.entities, *bot_slot), "{text}：下槽区不得有剖面线");
            assert!(hatch_filled(&shaft.entities, *top_mat), "{text}：上材料区漏填充");
            assert!(hatch_filled(&shaft.entities, *bot_mat), "{text}：下材料区漏填充");
        }
        // 单槽回归：下材料区（原本下环不复制缺口）必须仍被填充。
        let mut p = parse_program("S25 E25 L40 KEY A 18").unwrap();
        p.view = ShaftView::Section;
        let single = build(&p, 1.0).unwrap();
        assert!(hatch_filled(&single.entities, [20.0, -10.5]), "单槽下材料区应填充");
        assert!(!hatch_filled(&single.entities, [20.0, 10.5]), "单槽上槽区不得有剖面线");
    }

    /// 剖视槽轮廓闭合（用户 2026-09-25 报告）：导向键槽漏画槽底轮廓实线 →
    /// 剖视“顶边”看起来整体下沉到槽底、表面线不完整。不变量：
    /// * 槽底直线贯通槽两端（模板口径；固定螺钉孔在槽底之下）；
    /// * 槽外表面线连续（轴全长 − 槽口 全部被覆盖，不缺段→无台阶）；
    /// * 槽底线不得向槽外延伸；槽口在表面线处断开（槽是槽、表面是表面）；
    /// * 剖面线不因此破坏。单槽 / 两个键槽 / 双槽 / 普通 / 导向都查。
    #[test]
    fn section_keyway_floor_line_closes_slot_contour() {
        // (名称, DSL, [(槽 x0, 槽 x1, 半径 r, 槽底 floor, 方位 +1 上半 / -1 下半)])
        let cases: &[(&str, &str, &[(f64, f64, f64, f64, f64)])] = &[
            (
                "导向单槽",
                "S30 E30 L50 KEY A 25 导向",
                &[(12.5, 37.5, 15.0, 11.0, 1.0)],
            ),
            (
                "导向两段",
                "S30 E30 L40 CH2@L KEY A 25 导向 | S30 E30 L10 | S30 E30 L40 CH2@R KEY A 25 导向",
                &[(8.5, 33.5, 15.0, 11.0, 1.0), (56.5, 81.5, 15.0, 11.0, 1.0)],
            ),
            (
                "普通单槽",
                "S30 E30 L50 CH2@L CH2@R KEY A 25",
                &[(12.5, 37.5, 15.0, 11.0, 1.0)],
            ),
            (
                "普通双槽",
                "S30 E30 L50 CH2@L CH2@R KEY A 25 双槽",
                &[(12.5, 37.5, 15.0, 11.0, 1.0), (12.5, 37.5, 15.0, 11.0, -1.0)],
            ),
        ];
        for (name, dsl, slots) in cases {
            let mut p = parse_program(&format!("{dsl}\nVIEW 剖视")).unwrap();
            p.view = ShaftView::Section;
            let s = build(&p, 1.0).unwrap_or_else(|e| panic!("{name}: {e}"));
            let main_segs_at = |y: f64| -> Vec<(f64, f64)> {
                s.entities
                    .iter()
                    .filter_map(|e| match e {
                        EntityType::Line(l)
                            if l.common.layer == LAYER_MAIN
                                && (l.start.y - y).abs() < 1e-9
                                && (l.end.y - y).abs() < 1e-9 =>
                        {
                            Some((l.start.x.min(l.end.x), l.start.x.max(l.end.x)))
                        }
                        _ => None,
                    })
                    .collect()
            };
            for &(x0, x1, r, floor, sign) in slots.iter() {
                let (yf, yr) = (sign * floor, sign * r);
                // 正向：槽底直线贯通槽两端（模板口径）。
                assert!(
                    line_hit(&s.entities, [x0, yf], [x1, yf], LAYER_MAIN),
                    "{name}：槽 [{x0},{x1}] 缺槽底直线 y={yf}（表面线会看起来下沉到槽底）"
                );
                assert!(
                    line_hit(&s.entities, [x0, yr], [x0, yf], LAYER_MAIN),
                    "{name}：{x0} 处槽壁"
                );
                assert!(
                    line_hit(&s.entities, [x1, yf], [x1, yr], LAYER_MAIN),
                    "{name}：{x1} 处槽壁"
                );
                // 正向：槽口两侧的表面线必须接在槽壁顶端（轴外表面线不得缺段）。
                let surf = main_segs_at(yr);
                for xt in [x0, x1] {
                    assert!(
                        surf.iter().any(|(a, b)| (a - xt).abs() < 1e-9 || (b - xt).abs() < 1e-9),
                        "{name}：{xt} 处表面线缺失（槽外表面必须连续）"
                    );
                }
                // 负断言：槽底线不得向槽外延伸（否则就是“整段顶边下沉到槽底”的形态）。
                // 同一 y 上可能有多条槽底线（两个键槽）——每条都必须落在某个槽口内。
                for (a, b) in main_segs_at(yf) {
                    assert!(
                        slots.iter().any(|&(sx0, sx1, _, sf, sp)| (sp * sf - yf).abs() < 1e-9
                            && a >= sx0 - 1e-9
                            && b <= sx1 + 1e-9),
                        "{name}：y={yf} 的槽底线 [{a},{b}] 越出所有槽口"
                    );
                }
                // 负断言：槽口在表面线处必须断开。
                assert!(
                    !line_hit(&s.entities, [x0, yr], [x1, yr], LAYER_MAIN),
                    "{name}：槽口不应有贯通表面线"
                );
            }
            // 表面线连续性不变量：轴全长 − 槽口 应被表面线段完全覆盖（不缺段→无台阶）。
            for sign in [1.0_f64, -1.0] {
                let yr = sign * slots[0].2;
                let mut segs = main_segs_at(yr);
                segs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                segs.dedup();
                if segs.is_empty() {
                    continue;
                }
                for w in segs.windows(2) {
                    let (a_end, b_start) = (w[0].1, w[1].0);
                    if b_start - a_end <= 1e-9 {
                        continue;
                    }
                    assert!(
                        slots.iter().any(|&(x0, x1, _, _, sp)| sp == sign
                            && (x0 - a_end).abs() < 1e-9
                            && (x1 - b_start).abs() < 1e-9),
                        "{name}：y={yr} 表面线在 [{a_end},{b_start}] 缺段——只允许是槽口"
                    );
                }
            }
            // 剖面线仍在（轮廓闭合不破坏 HATCH）。
            assert_eq!(count_kind(&s.entities, "HATCH"), 1, "{name}：剖面线 1 片");
        }
    }

    /// 阶段 2 ④ 批（轴族）：DSL 报错/用法随语言切换、关键数据保留。
    #[test]
    fn shaft_messages_switch_language_keeping_data() {
        let _g = crate::global_state_test_lock();
        crate::i18n::clear_missing_keys();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);

        // zh：解析报错 + 用法；关键数据（M1.5/GEAR M3/b8/h9/双）原样。
        assert!(usage().contains("OCSMSHAFT"));
        let e = parse_program("S30 E30 L45 KEY A").unwrap_err();
        assert!(e.contains("KEY 缺少键长"), "{e}");
        let e = parse_program("S30 E30 L45 CH0").unwrap_err();
        assert!(e.contains("C=0") && e.contains("必须 > 0"), "{e}");
        let e = parse_program("S30 E30 L45 M1.5 TL").unwrap_err();
        assert!(e.contains("TL 缺少数值"), "{e}");
        let e = parse_program("GEAR M3").unwrap_err();
        assert!(e.contains("GEAR 缺少 Z"), "{e}");
        let e = parse_program("S30 E30 L45 KEY B 18 b8 h9").unwrap_err();
        assert!(e.contains("不是标准配对") && e.contains("b8") && e.contains("h9"), "{e}");
        let e =
            parse_program("S30 E30 L45 M1.5 TL20 RL g1 2.5 g2 4.5 dg 22.7 r 0.8").unwrap_err();
        assert!(e.contains("按螺距查表") && e.contains("RL"), "{e}");

        // en：同一批调用，数据不变。
        crate::i18n::set_lang(crate::i18n::Lang::En);
        assert!(usage().contains("shaft generator"));
        let e = parse_program("S30 E30 L45 KEY A").unwrap_err();
        assert!(e.contains("KEY is missing the key length"), "{e}");
        let e = parse_program("S30 E30 L45 CH0").unwrap_err();
        assert!(e.contains("C=0") && e.contains("must be > 0"), "{e}");
        let e = parse_program("S30 E30 L45 M1.5 TL").unwrap_err();
        assert!(e.contains("TL is missing a value"), "{e}");
        let e = parse_program("GEAR M3").unwrap_err();
        assert!(e.contains("GEAR is missing Z"), "{e}");
        let e = parse_program("S30 E30 L45 KEY B 18 b8 h9").unwrap_err();
        assert!(e.contains("not a standard pair") && e.contains("b8") && e.contains("h9"), "{e}");
        let e =
            parse_program("S30 E30 L45 M1.5 TL20 RL g1 2.5 g2 4.5 dg 22.7 r 0.8").unwrap_err();
        assert!(e.contains("looked up by pitch") && e.contains("RL"), "{e}");
        assert!(!e.contains("按螺距查表"), "{e}");

        assert!(
            crate::i18n::missing_keys().is_empty(),
            "缺词条：{:?}",
            crate::i18n::missing_keys()
        );
        crate::i18n::set_lang_auto();
    }

    /// 族②补齐（§26）：JSON 输入 / 几何校验 / 计算书正文 —— zh/en 双断言、数据原样。
    #[test]
    fn shaft_json_geometry_and_report_switch_language() {
        let _g = crate::global_state_test_lock();
        crate::i18n::clear_missing_keys();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);

        // ── JSON 输入路径 ──
        let e = parse_program(r#"{"segments":[{"ch":[{"c":2,"end":"X"}]}]}"#).unwrap_err();
        assert!(e.contains("ch[0]") && e.contains("X") && e.contains("只能用 L 或 R"), "{e}");
        let e = parse_program(r#"{"segments":[{"spline":{"spec":"6x23x26x6"}}]}"#).unwrap_err();
        assert!(e.contains("缺少 L") && e.contains("6x23x26x6"), "{e}");
        let e = parse_program(r#"{"segments":[]}"#).unwrap_err();
        assert!(e.contains("segments") && e.contains("至少给一段"), "{e}");
        let e = parse_program(r#"{"segments":[{"gear":{"m":3,"z":20,"h":30,"kind":"nope"}}]}"#).unwrap_err();
        assert!(e.contains("kind") && e.contains("nope"), "{e}");

        // ── 几何校验（build / validate）──
        let e = build(&parse_program("S30 E30 L0").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("L=0") && e.contains("必须 > 0"), "{e}");
        let e = build(&parse_program("S30 E30 L45 CH60").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("C=60") && e.contains("特征重叠"), "{e}");
        let e = build(&parse_program("S30 E30 L45 OV50").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("b1=50") && e.contains("段长 l=45"), "{e}");
        let e = build(&parse_program("S30 E30 L45 OV3@L").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("自由端") && e.contains("越程槽"), "{e}");
        let e = build(
            &parse_program("S30 E30 L45 CH2@R | S30 E30 L20 CH2@L").unwrap(),
            1.0,
        )
        .unwrap_err();
        assert!(e.contains("同一端面") && e.contains("特征重叠"), "{e}");
        let e = build(&parse_program("S80 E80 L10 | GEAR M3 Z20 H30").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("Ø80") && e.contains("Ø66"), "{e}");

        // ── 计算书正文（段清单 + 花键节）──
        let md = build_report(&parse_program("SPLINE 6x23x26x6 L30").unwrap()).unwrap();
        assert!(md.contains("# 轴段计算书") && md.contains("## 1. 段清单"), "{md}");
        assert!(md.contains("矩形花键段") && md.contains("N=6") && md.contains("39.6047"), "{md}");
        let md2 = build_report(&parse_program("SPLINE EX M3 Z20 ALPHA30 X0 DA63 DF54.6 BETA0 H30").unwrap()).unwrap();
        assert!(md2.contains("## 2. 花键参数表（GB/T 3478") && md2.contains("同一份"), "{md2}");

        // ── en：同一批调用，数据原样 ──
        crate::i18n::set_lang(crate::i18n::Lang::En);
        let e = parse_program(r#"{"segments":[{"ch":[{"c":2,"end":"X"}]}]}"#).unwrap_err();
        assert!(e.contains("ch[0]") && e.contains("X") && e.contains("only L or R"), "{e}");
        let e = parse_program(r#"{"segments":[{"spline":{"spec":"6x23x26x6"}}]}"#).unwrap_err();
        assert!(e.contains("missing L") && e.contains("6x23x26x6"), "{e}");
        let e = parse_program(r#"{"segments":[]}"#).unwrap_err();
        assert!(e.contains("segments") && e.contains("at least one"), "{e}");
        let e = parse_program(r#"{"segments":[{"gear":{"m":3,"z":20,"h":30,"kind":"nope"}}]}"#).unwrap_err();
        assert!(e.contains("kind") && e.contains("nope") && e.contains("external/internal"), "{e}");
        let e = build(&parse_program("S30 E30 L0").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("L=0") && e.contains("must be > 0"), "{e}");
        let e = build(&parse_program("S30 E30 L45 CH60").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("C=60") && e.contains("overlap"), "{e}");
        let e = build(&parse_program("S30 E30 L45 OV50").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("b1=50") && e.contains("l=45"), "{e}");
        let e = build(&parse_program("S30 E30 L45 OV3@L").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("left end is free") && e.contains("overtravel groove"), "{e}");
        let e = build(
            &parse_program("S30 E30 L45 CH2@R | S30 E30 L20 CH2@L").unwrap(),
            1.0,
        )
        .unwrap_err();
        assert!(e.contains("same end face") && e.contains("overlap"), "{e}");
        let e = build(&parse_program("S80 E80 L10 | GEAR M3 Z20 H30").unwrap(), 1.0).unwrap_err();
        assert!(e.contains("Ø80") && e.contains("Ø66") && e.contains("tip"), "{e}");
        let md = build_report(&parse_program("SPLINE 6x23x26x6 L30").unwrap()).unwrap();
        assert!(md.contains("# Shaft segment report") && md.contains("## 1. Segment list"), "{md}");
        assert!(md.contains("rectangular spline segment") && md.contains("N=6"), "{md}");
        let md2 = build_report(&parse_program("SPLINE EX M3 Z20 ALPHA30 X0 DA63 DF54.6 BETA0 H30").unwrap()).unwrap();
        assert!(md2.contains("## 2. Spline parameter table") && md2.contains("Shares one"), "{md2}");
        assert!(
            crate::i18n::missing_keys().is_empty(),
            "缺词条：{:?}",
            crate::i18n::missing_keys()
        );
        crate::i18n::set_lang_auto();
    }
}
