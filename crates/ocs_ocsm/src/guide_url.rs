//! 引导线 URL 协议（Guide-Line Protocol）。
//!
//! 引导线是 `10引导线层` 上的一条直线，两端点即标注的第一/第二点，其上挂
//! PE_URL 超链接承载标注参数。格式：
//!
//! ```text
//! http://127.0.0.1:<port>/DIM/<TYPE>/[<SUB>]/<dist>?text=<enc>&tol=<enc>&sym=<enc>&ver=<v>&let=<L>
//! TYPE: LINEAR | DIAMETER | RADIUS | DATUM | VIEW
//! SUB (LINEAR): H(水平) | V(竖直) | A(对齐)
//! dist: 与 P2 距离（尺寸线沿法向相对 P2 的偏移，正负定侧）
//! text: 标注文字（"<>" = 测量值，可自定义）；tol/sym 可空，URL-encoded
//! ver (DATUM): 1996 | 2008（GB/T 1182 版本，默认 2008）；let (DATUM): 基准字母（默认 A）
//! let (VIEW): 向视图字母（必填）；scale (VIEW): 比例文本（可缺省，如 1:1）；
//! flip (VIEW): 翻转方向（1 = 加旋转弧线箭头 → 样式3）；mx/my (VIEW): 标记放置点（缺省 0,0）
//! BALLOON (序号): items=序号列表（逗号分隔，从指引线向外/向上）、dir=H 横向 / V 纵向、
//! ins=1 序号冲突时插入后移（否则同序号行数量 +1）
//! WELD (焊接): wu/wl=上下侧符号名、wdash/wcir/wflg/wtail/wc=五开关（虚线/全周边/现场旗/尾部/C）、
//! wut/wuq/wlt/wlq/wtt=五文字槽（上/下侧厚度尺寸、上/下侧数量长度、尾部注释）
//! ```
//!
//! 端口为标注更新服务器端口；解析时忽略（本机单实例）。

use crate::balloon::BalloonDir;

/// 标注主类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuideType {
    Linear,
    Diameter,
    Radius,
    Datum,
    View,
    Angle,
    Section,
    Tolerance,
    Detail,
    ArcLen,
    /// 焊接符号（两段 PLINE 引导：顶点0=焊缝点，顶点1=基准线起点，
    /// 顶点2=基准线末端；生成全家福块——GB/T 324 焊缝标注）。
    Weld,
    /// 引线标注（两段 PLINE 引导：顶点0=箭头点，顶点1=拐点，顶点2=肩线
    /// 末端；生成引线+箭头+肩线+上下侧文字——焊接的减法版）。
    Leader,
    /// 序号标注（引线的变体）：两段 PLINE 引导（顶点0=指针点→画圆点，
    /// 顶点1=拐点，顶点2=肩线末端）；一个组 = 1 指引线 + 1 圆点 + N 条横线
    /// （横向 V 形折线连 / 纵向竖线连）+ N 个序号（各自横线上方居中）。
    Balloon,
}

impl GuideType {
    pub fn as_str(self) -> &'static str {
        match self {
            GuideType::Linear => "LINEAR",
            GuideType::Diameter => "DIAMETER",
            GuideType::Radius => "RADIUS",
            GuideType::Datum => "DATUM",
            GuideType::View => "VIEW",
            GuideType::Angle => "ANGLE",
            GuideType::Section => "SECTION",
            GuideType::Tolerance => "TOLERANCE",
            GuideType::Detail => "DETAIL",
            GuideType::ArcLen => "ARCLEN",
            GuideType::Weld => "WELD",
            GuideType::Leader => "LEADER",
            GuideType::Balloon => "BALLOON",
        }
    }
    fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "LINEAR" => Some(GuideType::Linear),
            "DIAMETER" => Some(GuideType::Diameter),
            "RADIUS" => Some(GuideType::Radius),
            "DATUM" => Some(GuideType::Datum),
            "VIEW" => Some(GuideType::View),
            "ANGLE" => Some(GuideType::Angle),
            "SECTION" | "SEC" => Some(GuideType::Section),
            "TOLERANCE" | "GDT" | "TOL" | "FTCF" => Some(GuideType::Tolerance),
            "DETAIL" | "DET" | "放大" => Some(GuideType::Detail),
            "ARCLEN" | "ARC" | "弧长" => Some(GuideType::ArcLen),
            "WELD" | "焊接" => Some(GuideType::Weld),
            "LEADER" | "引线" | "LEAD" => Some(GuideType::Leader),
            "BALLOON" | "序号" | "序号标注" | "XH" => Some(GuideType::Balloon),
            _ => None,
        }
    }
}

/// 线性标注的子类型（对齐方式）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinearSub {
    Horizontal,
    Vertical,
    Aligned,
}

impl LinearSub {
    pub fn as_str(self) -> &'static str {
        match self {
            LinearSub::Horizontal => "H",
            LinearSub::Vertical => "V",
            LinearSub::Aligned => "A",
        }
    }
    fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "H" => Some(LinearSub::Horizontal),
            "V" => Some(LinearSub::Vertical),
            "A" => Some(LinearSub::Aligned),
            _ => None,
        }
    }
    /// 显示名（GUI 用；随语言）。
    #[allow(dead_code)]
    pub fn label(self) -> String {
        crate::i18n::t(match self {
            LinearSub::Horizontal => "cmd.guide.linear.h",
            LinearSub::Vertical => "cmd.guide.linear.v",
            LinearSub::Aligned => "cmd.guide.linear.a",
        })
    }
}

/// 基准标注标准版本（GB/T 1182）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatumVersion {
    /// GB/T 1182-1996：圆 + 字母 + 短横线。
    GB1996,
    /// GB/T 1182-2008：方框 + 字母 + 实心三角箭头。
    GB2008,
}

impl DatumVersion {
    pub fn as_str(self) -> &'static str {
        match self {
            DatumVersion::GB1996 => "1996",
            DatumVersion::GB2008 => "2008",
        }
    }
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "1996" => Some(DatumVersion::GB1996),
            "2008" => Some(DatumVersion::GB2008),
            _ => None,
        }
    }
}

/// 向视图翻转方向（三态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlipDir {
    /// 无（默认）——不输出弧线箭头。
    #[default]
    None,
    /// 顺时针翻转——弧线箭头为逆时针的镜像（绕 Y 轴，x→−x）。
    Clockwise,
    /// 逆时针翻转——弧线箭头（对照 OCSMDIMGULIDE5-symbol.dxf 样式3）。
    CounterClockwise,
}

impl FlipDir {
    pub fn as_str(self) -> &'static str {
        match self {
            FlipDir::None => "none",
            FlipDir::Clockwise => "cw",
            FlipDir::CounterClockwise => "ccw",
        }
    }
    fn from_url(v: &str) -> Option<Self> {
        match v {
            "" | "0" | "false" | "no" | "none" | "off" => Some(FlipDir::None),
            "1" | "true" | "yes" | "ccw" | "counter" | "counterclockwise" | "逆" => {
                Some(FlipDir::CounterClockwise)
            }
            "cw" | "clock" | "clockwise" | "顺" => Some(FlipDir::Clockwise),
            _ => None,
        }
    }
}

/// 剖切符号的视向箭头侧（相对剖切路径前进方向）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SectionSide {
    /// 默认右侧：箭头朝剖切路径前进方向逆时针旋转 90°。
    #[default]
    Right,
    /// 左侧：顺时针旋转 90°。
    Left,
}

impl SectionSide {
    pub fn as_str(self) -> &'static str {
        match self {
            SectionSide::Right => "R",
            SectionSide::Left => "L",
        }
    }
    fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "R" | "RIGHT" | "右" => Some(SectionSide::Right),
            "L" | "LEFT" | "左" => Some(SectionSide::Left),
            _ => None,
        }
    }
    /// 沿剖切路径前进方向 τ 的箭头侧单位向量。
    pub fn dir(self, tau: (f64, f64)) -> (f64, f64) {
        match self {
            // 右侧 = 逆时针旋转 90°；左侧 = 顺时针旋转 90°。
            SectionSide::Right => (-tau.1, tau.0),
            SectionSide::Left => (tau.1, -tau.0),
        }
    }
    /// 显示名（GUI 用；随语言）。
    #[allow(dead_code)]
    pub fn label(self) -> String {
        crate::i18n::t(match self {
            SectionSide::Right => "cmd.guide.section.right",
            SectionSide::Left => "cmd.guide.section.left",
        })
    }
}

/// 角度标注的角模式（两段 PLINE 引导线，顶点即转折点）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AngleMode {
    /// 劣角（锐角，默认）。
    #[default]
    Minor,
    /// 优角（360°−劣角）。
    Reflex,
}

impl AngleMode {
    pub fn as_str(self) -> &'static str {
        match self {
            AngleMode::Minor => "M",
            AngleMode::Reflex => "R",
        }
    }
    fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "M" | "MINOR" | "劣" => Some(AngleMode::Minor),
            "R" | "REFLEX" | "REF" | "优" => Some(AngleMode::Reflex),
            _ => None,
        }
    }
    /// 显示名（GUI 用；随语言）。
    #[allow(dead_code)]
    pub fn label(self) -> String {
        crate::i18n::t(match self {
            AngleMode::Minor => "cmd.guide.angle.minor",
            AngleMode::Reflex => "cmd.guide.angle.reflex",
        })
    }
}

/// 焊缝打磨方式（GB/T 324 焊缝表面加工；参考 焊接符号-焊缝打磨.dxf 第一行
/// 角焊 6 例：不打磨 / 弧·凹 / 弧·凸 / 直线 / 双弧 / 锯齿）。弧与附加件的
/// 几何按焊缝形式分两种风格：角焊版（倾斜右上）/ 其它焊缝版（正上方）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GrindKind {
    /// 不打磨（无附加件）。
    #[default]
    None,
    /// 弧·凹（弧向焊缝凹入）。
    ArcConcave,
    /// 弧·凸（弧向外鼓；示例.dxf 用的这种）。
    ArcConvex,
    /// 直线（加工成平面）。
    Line,
    /// 双弧（鱼鳞状打磨纹）。
    DoubleArc,
    /// 锯齿（打磨纹）。
    Zigzag,
}

impl GrindKind {
    pub fn as_str(self) -> &'static str {
        match self {
            GrindKind::None => "none",
            GrindKind::ArcConcave => "cav",
            GrindKind::ArcConvex => "cvx",
            GrindKind::Line => "lin",
            GrindKind::DoubleArc => "dbl",
            GrindKind::Zigzag => "zig",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "" | "0" | "none" | "无" | "不打磨" => Some(GrindKind::None),
            "cav" | "concave" | "凹" | "弧凹" => Some(GrindKind::ArcConcave),
            "cvx" | "convex" | "1" | "凸" | "弧凸" => Some(GrindKind::ArcConvex),
            "lin" | "line" | "直线" | "平面" => Some(GrindKind::Line),
            "dbl" | "double" | "双弧" => Some(GrindKind::DoubleArc),
            "zig" | "zigzag" | "锯齿" => Some(GrindKind::Zigzag),
            _ => None,
        }
    }
    /// 显示名（GUI/文档用；随语言）。
    pub fn label(self) -> String {
        crate::i18n::t(match self {
            GrindKind::None => "cmd.guide.grind.none",
            GrindKind::ArcConcave => "cmd.guide.grind.cav",
            GrindKind::ArcConvex => "cmd.guide.grind.cvx",
            GrindKind::Line => "cmd.guide.grind.line",
            GrindKind::DoubleArc => "cmd.guide.grind.dbl",
            GrindKind::Zigzag => "cmd.guide.grind.zig",
        })
    }
}

/// 焊接符号参数（GB/T 324；仅 WELD）。
/// 上/下侧符号名来自焊接符号表.dxf 块名（"无"/空 = 不放符号）；
/// 下侧符号由服务端查镜像版几何。跨线单置块（参考线上的点焊缝等）
/// 只能放上侧（跨线绘制）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WeldParams {
    /// 上侧焊缝符号名（"无"/"" = 无；跨线单置块也填这里）。
    pub upper: String,
    /// 下侧焊缝符号名（"无"/"" = 无；服务端用镜像版几何）。
    pub lower: String,
    /// 虚线（第二基准线，非箭头侧指示；下侧有内容时强制开）。
    pub dash: bool,
    /// 全周边符号（圆圈，基准线起点）。
    pub circle: bool,
    /// 半包围焊缝区域（⊏ 形；区别于全周边圆，表示半包围结构）。与 circle
    /// 互斥（同开时以半包围优先）。
    pub half: bool,
    /// 现场焊接（三角旗，基准线起点竖杆）。
    pub flag: bool,
    /// 尾部（尾叉 + 尾部注释；关 = 基准线保留全长，仅去尾叉注释）。
    pub tail: bool,
    /// 上侧焊缝打磨方式（附加件；几何按焊缝形式选角焊版/其它版）。
    pub grind_upper: GrindKind,
    /// 下侧（另一侧）焊缝打磨方式——与上侧**独立**控制。
    pub grind_lower: GrindKind,
    /// 上侧焊接方法字母（C/G/H/M/R/U；空/"无" = 无标注）。仅角焊缝与喇叭形
    /// 焊缝可用（角焊 / 喇叭形焊 / 单边喇叭形焊），其余符号忽略。
    pub method_upper: String,
    /// 下侧焊接方法字母——与上侧**独立**控制。
    pub method_lower: String,
    /// 上侧厚度尺寸（文字，可空）。
    pub up_thick: String,
    /// 上侧数量长度（文字，可空）。
    pub up_qty: String,
    /// 下侧厚度尺寸（文字，可空）。
    pub lo_thick: String,
    /// 下侧数量长度（文字，可空）。
    pub lo_qty: String,
    /// 尾部注释（文字，可空；tail 开才生成）。
    pub tail_text: String,
}

/// 解析后的标注参数。
#[derive(Debug, Clone, PartialEq)]
pub struct GuideParams {
    pub guide_type: GuideType,
    /// 仅 LINEAR 有子类型。
    pub sub: Option<LinearSub>,
    /// 与 P2 距离（法向偏移）。
    pub dist: f64,
    /// 标注文字（`<>` 或 None = 测量值）。
    pub text: Option<String>,
    /// 公差（可空）。
    pub tol: Option<String>,
    /// 上偏差（公差堆叠 `\S{up}^{dn}`，对照公差.dxf 29C）。
    pub up: Option<String>,
    /// 下偏差（公差堆叠）。
    pub dn: Option<String>,
    /// ISO 286 配合代号（如 "H7" / "g6" / "H7/g6"）：服务端结合测量值算极限偏差。
    pub fit: Option<String>,
    /// 符号（可空）。
    pub sym: Option<String>,
    /// 测量值小数位数（None = 默认 2；渲染时消零：12.70→12.7、12.00→12）。
    pub dec: Option<u32>,
    /// 基准标注标准版本（仅 DATUM；默认 2008）。
    pub ver: DatumVersion,
    /// 基准字母（仅 DATUM；默认 "A"）。
    pub letter: Option<String>,
    /// 向视图比例文本（仅 VIEW；可缺省，如 "1:1"）。
    pub scale: Option<String>,
    /// 向视图翻转方向（仅 VIEW；None=无，Clockwise=顺时针，CounterClockwise=逆时针）。
    pub flip: FlipDir,
    /// 向视图标记放置点（仅 VIEW；缺省 (0,0)，字母中心锚定）。
    pub marker: Option<(f64, f64)>,
    /// 角度标注角模式（仅 ANGLE；默认劣角 Minor）。
    pub angle_mode: AngleMode,
    /// 剖切符号视向箭头侧（仅 SECTION；默认 Right）。
    pub section_side: SectionSide,
    /// 剖切符号是否显示视向箭头（仅 SECTION；默认 true）。
    pub show_arrow: bool,
    /// 形位公差：特征项目符号（gdt 字体字母，仅 TOLERANCE）。如 "b"=垂直度、"j"=位置度。
    pub gdt_sym: Option<String>,
    /// 形位公差：公差值前缀 ⌀（直径公差带，仅 TOLERANCE；默认 false）。
    pub gdt_dia: bool,
    /// 形位公差：公差值（仅 TOLERANCE）。如 "0.03"。
    pub gdt_tol: Option<String>,
    /// 形位公差：基准 1/2/3（仅 TOLERANCE）。
    pub gdt_d1: Option<String>,
    pub gdt_d2: Option<String>,
    pub gdt_d3: Option<String>,
    /// 形位公差多行（stack-perp：每行独立 FCF 框，沿垂直于引导线方向堆叠）。
    /// 空时退化为单行（用 gdt_sym/gdt_dia/gdt_tol/gdt_d1..d3）。
    pub gdt_rows: Vec<GdtRow>,
    /// 形位公差框上方注释（显示在 FCF 框 top 侧，常用如基本尺寸说明）。
    pub gdt_top: Option<String>,
    /// 形位公差框下方注释（显示在 FCF 框 bottom 侧）。
    pub gdt_bot: Option<String>,
    /// 局部放大图：放大比例（绝对比例，默认 2）。
    pub detail_scale: f64,
    /// 局部放大图：序号（罗马数字，空 = 自动编号）。
    pub detail_no: Option<String>,
    /// 局部放大图：放置点偏移（相对引导中心，缺省 = 自动偏移）。
    pub detail_pos: Option<(f64, f64)>,
    /// 局部放大图：生成时的 TF 图幅倍率（frame_scale_at 记录，标注系数用）。
    pub detail_frame: f64,
    /// 焊接符号参数（仅 WELD）。
    pub weld: WeldParams,
    /// 引线标注参数（仅 LEADER）。
    pub leader: LeaderParams,
    /// 序号标注参数（仅 BALLOON）。
    pub balloon: BalloonParams,
}

/// 序号标注参数。
///
/// 一个组 = 1 条指引线 + 1 个圆点 + N 条横线 + N 个序号：
/// `items` 按**从指引线向外/向上**排列（通常升序，如 `[5,6]`：5 贴指引线、6 在上）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BalloonParams {
    /// 序号列表（`items=1,2,3`）。
    pub items: Vec<String>,
    /// 扩展方向：横向 `H`（V 形折线连接）/ 纵向 `V`（竖线连接）。
    pub dir: BalloonDir,
    /// 序号冲突时是否"插入后移"（GUI 勾选）：true = 其后所有序号 +1；
    /// false = 视为同一零件，明细表里对应行**数量 +1**。
    pub insert_mode: bool,
}

/// 引线标注参数：引线上方文字 + 引线下方文字（各单行；空 = 不显示）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LeaderParams {
    /// 上侧文字（肩线上方，锚点 17.413s / +2.750s，左对齐）。
    pub upper: String,
    /// 下侧文字（肩线下方，锚点 17.413s / −2.750s，左对齐）。
    pub lower: String,
}

/// 形位公差单行：符号 + ⌀ + 公差值 + 修饰符(ⓂⓁⓅⓈ) + 基准1/2/3。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GdtRow {
    pub sym: String,   // 特征符号（u c e g d k a b f j i r h t）或基本尺寸 o
    pub dia: bool,     // 公差值前缀 ⌀（直径公差带）
    pub tol: String,   // 公差值（如 "0.02"）
    pub mods: String,  // 修饰符（m/l/p/s → ⓂⓁⓅⓈ），跟在公差值**后面**同一格
    pub d1: String,
    pub d2: String,
    pub d3: String,
}

impl GdtRow {
    /// 该行是否有可渲染内容（符号/公差/基准/修饰符）。
    pub fn is_empty(&self) -> bool {
        self.sym.is_empty() && self.tol.is_empty() && self.mods.is_empty()
            && self.d1.is_empty() && self.d2.is_empty() && self.d3.is_empty()
    }
}

/// 解析多行形位公差 URL 参数（`rows=sym,dia,tol,mods,d1,d2,d3;...`）。
/// dia/mods 为修饰符相关；每行 7 个字段，行内逗号分隔、行间分号。
fn parse_gdt_rows(v: &str) -> Vec<GdtRow> {
    let mut rows = Vec::new();
    for seg in v.split(';').filter(|s| !s.is_empty()) {
        let mut f = seg.split(',');
        let sym = f.next().unwrap_or("").to_string();
        let dia = matches!(f.next().unwrap_or("0"), "1" | "true");
        let tol = f.next().unwrap_or("").to_string();
        let mods = f.next().unwrap_or("").to_string();
        let d1 = f.next().unwrap_or("").to_string();
        let d2 = f.next().unwrap_or("").to_string();
        let d3 = f.next().unwrap_or("").to_string();
        let row = GdtRow { sym, dia, tol, mods, d1, d2, d3 };
        if !row.is_empty() {
            rows.push(row);
        }
    }
    rows
}

impl GuideParams {
    /// 构造一个线性标注参数。
#[allow(dead_code)]
    pub fn linear(sub: LinearSub, dist: f64) -> Self {
        GuideParams {
            guide_type: GuideType::Linear,
            sub: Some(sub),
            dist,
            text: None,
            tol: None,
            up: None,
            dn: None,
            fit: None,
            sym: None,
            dec: None,
            ver: DatumVersion::GB2008,
            letter: None,
            scale: None,
            flip: FlipDir::None,
            marker: None,
            angle_mode: AngleMode::Minor,
            section_side: SectionSide::Right,
            show_arrow: true,
            gdt_sym: None,
            gdt_dia: false,
            gdt_tol: None,
            gdt_d1: None,
            gdt_d2: None,
            gdt_d3: None,
            gdt_rows: Vec::new(),
            gdt_top: None,
            gdt_bot: None,
            detail_scale: 2.0,
            detail_no: None,
            detail_pos: None,
            detail_frame: 1.0,
            weld: WeldParams::default(),
            leader: LeaderParams::default(),
            balloon: BalloonParams::default(),
        }
    }

    /// 构造一个序号标注参数（`items` 从指引线向外/向上排列）。
    #[allow(dead_code)]
    pub fn balloon(items: Vec<String>, dir: BalloonDir) -> Self {
        let mut p = GuideParams::linear(LinearSub::Aligned, 0.0);
        p.guide_type = GuideType::Balloon;
        p.sub = None;
        p.balloon = BalloonParams { items, dir, insert_mode: false };
        p
    }

    /// 解析引导线 URL。失败返回 None。
    pub fn from_url(url: &str) -> Option<GuideParams> {
        // 去掉 scheme://authority，留下 path?query
        let rest = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))?;
        let path = rest.split_once('/').map(|(_, p)| p).unwrap_or(rest);
        let (path, query) = path.split_once('?').map_or((path, ""), |(p, q)| (p, q));
        let mut segs = path.split('/').filter(|s| !s.is_empty());
        let cmd = segs.next()?.to_ascii_uppercase();
        if cmd != "DIM" {
            return None;
        }
        let type_raw = segs.next()?;
        let guide_type = GuideType::from_str(&percent_decode(type_raw))?;

        // 剩余段：线性 = [SUB, dist]；角度 = [mode, dist]（mode 可缺省 → 劣角）。
        let sub = if guide_type == GuideType::Linear {
            Some(LinearSub::from_str(segs.next()?)?)
        } else {
            None
        };
        let mut angle_mode = AngleMode::Minor;
        let dist: f64 = if guide_type == GuideType::Angle {
            // 第 3 段可能是角模式（M/C/R），也可能是直接 dist（缺省劣角）。
            let s3 = segs.next()?;
            if let Ok(d) = s3.parse::<f64>() {
                d
            } else {
                angle_mode = AngleMode::from_str(s3)?;
                segs.next()?.parse().ok()?
            }
        } else {
            segs.next()?.parse().ok()?
        };

        // query 参数（text/tol/up/dn/sym）。
        let mut text = None;
        let mut tol = None;
        let mut up: Option<String> = None;
        let mut dn: Option<String> = None;
        let mut sym = None;
        let mut fit: Option<String> = None;
        let mut dec = None;
        let mut ver = DatumVersion::GB2008;
        let mut letter = None;
        let mut scale = None;
        let mut flip = FlipDir::None;
        let mut marker = None;
        let mut section_side = SectionSide::Right;
        let mut show_arrow = true;
        let mut gdt_sym = None;
        let mut gdt_dia = false;
        let mut gdt_tol = None;
        let mut gdt_d1 = None;
        let mut gdt_d2 = None;
        let mut gdt_d3 = None;
        let mut gdt_rows: Vec<GdtRow> = Vec::new();
        let mut gdt_top = None;
        let mut gdt_bot = None;
        let mut detail_scale = 2.0;
        let mut detail_no = None;
        let mut detail_pos = None;
        let mut detail_frame = 1.0;
        let mut weld = WeldParams::default();
        let mut leader = LeaderParams::default();
        let mut balloon = BalloonParams::default();
        // 打磨/焊接方法：兼容"两侧同值"旧键 + 上下侧独立新键。
        let mut grind_all = GrindKind::None;
        let mut grind_u: Option<GrindKind> = None;
        let mut grind_l: Option<GrindKind> = None;
        let mut method_all = String::new();
        let mut method_u: Option<String> = None;
        let mut method_l: Option<String> = None;
        for pair in query.split('&').filter(|s| !s.is_empty()) {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            let v = percent_decode(v);
            match k {
                "text" => text = Some(v),
                "tol" => tol = Some(v),
                "up" => up = Some(v),
                "dn" => dn = Some(v),
                "fit" => fit = Some(v),
                "sym" => sym = Some(v),
                "dec" => dec = v.parse::<u32>().ok().map(|d| d.min(8)),
                "ver" => ver = DatumVersion::from_str(&v).unwrap_or(DatumVersion::GB2008),
                "let" => letter = Some(v),
                "scale" => scale = (!v.is_empty()).then_some(v),
                "flip" => flip = FlipDir::from_url(&v).unwrap_or(FlipDir::None),
                "mx" | "my" => {
                    let (mut mx, mut my) = marker.unwrap_or((0.0, 0.0));
                    if k == "mx" {
                        mx = v.parse().ok()?;
                    } else {
                        my = v.parse().ok()?;
                    }
                    marker = Some((mx, my));
                }
                "side" => section_side = SectionSide::from_str(&v).unwrap_or(SectionSide::Right),
                "show" => show_arrow = !matches!(&*v, "0" | "false" | "no" | "off" | "隐藏"),
                "gdt" | "gsym" => gdt_sym = (!v.is_empty()).then_some(v),
                "dia" => gdt_dia = matches!(&*v, "1" | "true" | "yes" | "on"),
                "gtol" => gdt_tol = (!v.is_empty()).then_some(v),
                "d1" => gdt_d1 = (!v.is_empty()).then_some(v),
                "d2" => gdt_d2 = (!v.is_empty()).then_some(v),
                "d3" => gdt_d3 = (!v.is_empty()).then_some(v),
                // 多行形位公差：rows=sym,dia,tol,mods,d1,d2,d3;...（行间分号）。
                "rows" => {
                    gdt_rows = parse_gdt_rows(&v);
                }
                "top" => gdt_top = (!v.is_empty()).then_some(v),
                "bot" => gdt_bot = (!v.is_empty()).then_some(v),
                "s" => detail_scale = v.parse::<f64>().ok().filter(|s| *s > 0.0).unwrap_or(2.0),
                "no" => detail_no = (!v.is_empty()).then_some(v),
                "frame" => detail_frame = v.parse::<f64>().ok().filter(|s| *s > 0.0).unwrap_or(1.0),
                "dx" | "dy" => {
                    let (mut dx, mut dy) = detail_pos.unwrap_or((f64::NAN, f64::NAN));
                    if k == "dx" {
                        dx = v.parse().ok()?;
                    } else {
                        dy = v.parse().ok()?;
                    }
                    detail_pos = Some((dx, dy));
                }
                // 焊接符号（仅 WELD）：wu/wl=上下侧符号名；wdash/wcir/wflg/
                // wtail/wc=五开关；wut/wuq/wlt/wlq/wtt=五文字槽。
                "wu" => weld.upper = v,
                "wl" => weld.lower = v,
                // 引线标注（仅 LEADER）：lu=上侧文字、ll=下侧文字（单行）。
                "lu" => leader.upper = v,
                "ll" => leader.lower = v,
                // 序号标注（仅 BALLOON）：items=序号列表（逗号分隔，从指引线向外/向上）、
                // dir=H 横向 / V 纵向、ins=1 序号冲突时插入后移（否则数量 +1）。
                "items" | "nos" => balloon.items = crate::balloon::parse_items(&v),
                "dir" => balloon.dir = BalloonDir::from_str(&v).unwrap_or_default(),
                "ins" => balloon.insert_mode = matches!(&*v, "1" | "true" | "yes" | "on"),
                "wdash" => weld.dash = matches!(&*v, "1" | "true" | "yes" | "on"),
                "wcir" => weld.circle = matches!(&*v, "1" | "true" | "yes" | "on"),
                "whalf" => weld.half = matches!(&*v, "1" | "true" | "yes" | "on"),
                "wflg" => weld.flag = matches!(&*v, "1" | "true" | "yes" | "on"),
                "wtail" => weld.tail = matches!(&*v, "1" | "true" | "yes" | "on"),
                // 旧键 wc=1 等价于打磨=弧·凸（向后兼容）；新键 wgr=打磨方式代号。
                "wc" => {
                    if matches!(&*v, "1" | "true" | "yes" | "on") {
                        grind_all = GrindKind::ArcConvex;
                    }
                }
                // 打磨：旧键 wgr=两侧同值；新键 wgru/wgrl=上下侧独立。
                "wgr" => grind_all = GrindKind::from_str(&v).unwrap_or(GrindKind::None),
                "wgru" => grind_u = Some(GrindKind::from_str(&v).unwrap_or(GrindKind::None)),
                "wgrl" => grind_l = Some(GrindKind::from_str(&v).unwrap_or(GrindKind::None)),
                // 焊接方法字母（仅角焊/喇叭形可选）：旧键 wm=两侧同值；
                // 新键 wmu/wml=上下侧独立。
                "wm" | "wmu" | "wml" => {
                    let t = v.trim().to_ascii_uppercase();
                    let val = if matches!(t.as_str(), "C" | "G" | "H" | "M" | "R" | "U") {
                        t
                    } else {
                        String::new()
                    };
                    match k {
                        "wm" => method_all = val,
                        "wmu" => method_u = Some(val),
                        _ => method_l = Some(val),
                    }
                }
                "wut" => weld.up_thick = v,
                "wuq" => weld.up_qty = v,
                "wlt" => weld.lo_thick = v,
                "wlq" => weld.lo_qty = v,
                "wtt" => weld.tail_text = v,
                _ => {}
            }
        }

        weld.grind_upper = grind_u.unwrap_or(grind_all);
        weld.grind_lower = grind_l.unwrap_or(grind_all);
        weld.method_upper = method_u.unwrap_or_else(|| method_all.clone());
        weld.method_lower = method_l.unwrap_or(method_all);

        Some(GuideParams {
            guide_type,
            sub,
            dist,
            text,
            tol,
            up,
            dn,
            fit,
            sym,
            dec,
            ver,
            letter,
            scale,
            flip,
            marker,
            angle_mode,
            section_side,
            show_arrow,
            gdt_sym,
            gdt_dia,
            gdt_tol,
            gdt_d1,
            gdt_d2,
            gdt_d3,
            gdt_rows,
            gdt_top,
            gdt_bot,
            detail_scale,
            detail_no,
            detail_pos,
            detail_frame,
            weld,
            leader,
            balloon,
        })
    }

    /// 单行形位公差方便访问：若 gdt_rows 非空取第一行，否则回退单行字段。
    pub fn gdt_first_row(&self) -> GdtRow {
        if let Some(r) = self.gdt_rows.first() {
            return r.clone();
        }
        GdtRow {
            sym: self.gdt_sym.clone().unwrap_or_default(),
            dia: self.gdt_dia,
            tol: self.gdt_tol.clone().unwrap_or_default(),
            mods: String::new(),
            d1: self.gdt_d1.clone().unwrap_or_default(),
            d2: self.gdt_d2.clone().unwrap_or_default(),
            d3: self.gdt_d3.clone().unwrap_or_default(),
        }
    }

    /// 形位公差有效行：优先 gdt_rows（过滤空行）；否则单行字段合成一行。
    pub fn gdt_rows_nonempty(&self) -> Vec<GdtRow> {
        if !self.gdt_rows.is_empty() {
            return self.gdt_rows.iter().filter(|r| !r.is_empty()).cloned().collect();
        }
        let row = self.gdt_first_row();
        if row.is_empty() {
            Vec::new()
        } else {
            vec![row]
        }
    }

    /// 生成引导线 URL。`port` 为标注更新服务器端口。
#[allow(dead_code)]
    pub fn to_url(&self, port: u16) -> String {
        let mut url = format!("http://127.0.0.1:{port}/DIM/{}", self.guide_type.as_str());
        // 仅 LINEAR 带子类型段；ANGLE 非劣角带角模式段；其它类型（含 VIEW）无 sub。
        if self.guide_type == GuideType::Linear {
            if let Some(sub) = self.sub {
                url.push('/');
                url.push_str(sub.as_str());
            }
        } else if self.guide_type == GuideType::Angle && self.angle_mode != AngleMode::Minor {
            url.push('/');
            url.push_str(self.angle_mode.as_str());
        }
        url.push('/');
        url.push_str(&format_dist(self.dist));
        let mut q = Vec::new();
        if let Some(t) = &self.text {
            if !t.is_empty() {
                q.push(format!("text={}", percent_encode(t)));
            }
        }
        if let Some(t) = &self.tol {
            if !t.is_empty() {
                q.push(format!("tol={}", percent_encode(t)));
            }
        }
        if let Some(u) = &self.up {
            if !u.is_empty() {
                q.push(format!("up={}", percent_encode(u)));
            }
        }
        if let Some(d) = &self.dn {
            if !d.is_empty() {
                q.push(format!("dn={}", percent_encode(d)));
            }
        }
        if let Some(f) = &self.fit {
            if !f.is_empty() {
                q.push(format!("fit={}", percent_encode(f)));
            }
        }
        if let Some(s) = &self.sym {
            if !s.is_empty() {
                q.push(format!("sym={}", percent_encode(s)));
            }
        }
        if let Some(d) = self.dec {
            q.push(format!("dec={d}"));
        }
        if let Some(l) = &self.letter {
            q.push(format!("let={}", percent_encode(l)));
        }
        // 基准版本：默认 2008 不输出（简洁），非默认才带。
        if self.ver != DatumVersion::GB2008 {
            q.push(format!("ver={}", self.ver.as_str()));
        }
        // 向视图：比例 / 翻转方向 / 标记放置点。
        if let Some(s) = &self.scale {
            if !s.is_empty() {
                q.push(format!("scale={}", percent_encode(s)));
            }
        }
        if self.flip != FlipDir::None {
            q.push(format!("flip={}", self.flip.as_str()));
        }
        if let Some((mx, my)) = self.marker {
            q.push(format!("mx={mx}"));
            q.push(format!("my={my}"));
        }
        // 剖切符号：视向箭头侧 / 是否显示箭头。
        if self.section_side != SectionSide::Right {
            q.push(format!("side={}", self.section_side.as_str()));
        }
        if !self.show_arrow {
            q.push("show=0".to_string());
        }
        // 形位公差：符号 / ⌀ / 公差值 / 基准。
        if let Some(s) = &self.gdt_sym {
            if !s.is_empty() {
                q.push(format!("gdt={}", percent_encode(s)));
            }
        }
        if self.gdt_dia {
            q.push("dia=1".to_string());
        }
        if let Some(t) = &self.gdt_tol {
            if !t.is_empty() {
                q.push(format!("gtol={}", percent_encode(t)));
            }
        }
        for (k, d) in [("d1", &self.gdt_d1), ("d2", &self.gdt_d2), ("d3", &self.gdt_d3)] {
            if let Some(v) = d {
                if !v.is_empty() {
                    q.push(format!("{k}={}", percent_encode(v)));
                }
            }
        }
        // 多行形位公差：rows=sym,dia,tol,mods,d1,d2,d3;... 仅当存在多行时输出（单行走上面的单字段）。
        if self.gdt_rows.len() > 1 {
            let rows = self
                .gdt_rows
                .iter()
                .filter(|r| !r.is_empty())
                .map(|r| {
                    format!("{},{},{},{},{},{},{}",
                        percent_encode(&r.sym),
                        if r.dia { 1 } else { 0 },
                        percent_encode(&r.tol),
                        percent_encode(&r.mods),
                        percent_encode(&r.d1),
                        percent_encode(&r.d2),
                        percent_encode(&r.d3))
                })
                .collect::<Vec<_>>()
                .join(";");
            if !rows.is_empty() {
                q.push(format!("rows={}", percent_encode(&rows)));
            }
        }
        // 形位公差顶部/底部注释（FCF 框上方/下方）。
        if let Some(t) = &self.gdt_top {
            if !t.is_empty() {
                q.push(format!("top={}", percent_encode(t)));
            }
        }
        if let Some(b) = &self.gdt_bot {
            if !b.is_empty() {
                q.push(format!("bot={}", percent_encode(b)));
            }
        }
        if self.detail_scale != 2.0 {
            q.push(format!("s={}", self.detail_scale));
        }
        if let Some(n) = &self.detail_no {
            if !n.is_empty() {
                q.push(format!("no={}", percent_encode(n)));
            }
        }
        if let Some((dx, dy)) = self.detail_pos {
            q.push(format!("dx={}", dx));
            q.push(format!("dy={}", dy));
        }
        // 焊接符号（仅 WELD）：上下侧符号 + 五开关 + 五文字槽。
        if self.guide_type == GuideType::Weld {
            let w = &self.weld;
            if !w.upper.is_empty() && w.upper != "无" {
                q.push(format!("wu={}", percent_encode(&w.upper)));
            }
            if !w.lower.is_empty() && w.lower != "无" {
                q.push(format!("wl={}", percent_encode(&w.lower)));
            }
            if w.dash {
                q.push("wdash=1".to_string());
            }
            if w.circle {
                q.push("wcir=1".to_string());
            }
            if w.half {
                q.push("whalf=1".to_string());
            }
            if w.flag {
                q.push("wflg=1".to_string());
            }
            if w.tail {
                q.push("wtail=1".to_string());
            }
            // 打磨/焊接方法：两侧相同则用旧键（wgr/wm，紧凑且向后兼容），
            // 不同则分侧输出 wgru/wgrl 与 wmu/wml。
            if w.grind_upper == w.grind_lower {
                if w.grind_upper != GrindKind::None {
                    q.push(format!("wgr={}", w.grind_upper.as_str()));
                }
            } else {
                if w.grind_upper != GrindKind::None {
                    q.push(format!("wgru={}", w.grind_upper.as_str()));
                }
                if w.grind_lower != GrindKind::None {
                    q.push(format!("wgrl={}", w.grind_lower.as_str()));
                }
            }
            if w.method_upper == w.method_lower {
                if !w.method_upper.is_empty() {
                    q.push(format!("wm={}", w.method_upper));
                }
            } else {
                if !w.method_upper.is_empty() {
                    q.push(format!("wmu={}", w.method_upper));
                }
                if !w.method_lower.is_empty() {
                    q.push(format!("wml={}", w.method_lower));
                }
            }
            for (k, v) in [
                ("wut", &w.up_thick),
                ("wuq", &w.up_qty),
                ("wlt", &w.lo_thick),
                ("wlq", &w.lo_qty),
                ("wtt", &w.tail_text),
            ] {
                if !v.is_empty() {
                    q.push(format!("{k}={}", percent_encode(v)));
                }
            }
        }
        if self.guide_type == GuideType::Leader {
            let l = &self.leader;
            for (k, v) in [("lu", &l.upper), ("ll", &l.lower)] {
                if !v.is_empty() {
                    q.push(format!("{k}={}", percent_encode(v)));
                }
            }
        }
        if self.guide_type == GuideType::Balloon {
            let b = &self.balloon;
            if !b.items.is_empty() {
                q.push(format!("items={}", percent_encode(&b.items.join(","))));
            }
            q.push(format!("dir={}", b.dir.as_str()));
            if b.insert_mode {
                q.push("ins=1".into());
            }
        }
        if !q.is_empty() {
            url.push('?');
            url.push_str(&q.join("&"));
        }
        url
    }
}

/// 距离格式化：去掉多余小数（-50 / -14.2 / 10）。
#[allow(dead_code)]
fn format_dist(d: f64) -> String {
    if d.fract() == 0.0 && d.abs() < 1e12 {
        format!("{}", d as i64)
    } else {
        format!("{d}")
    }
}

/// 极简 percent-decode（%XX 与保留字符直通）。
pub(crate) fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = hex(bytes[i + 1]);
            let lo = hex(bytes[i + 2]);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// 极简 percent-encode（仅编码非 ASCII 与保留字符；中文等走 UTF-8 字节）。
#[allow(dead_code)]
fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        let keep = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~' | b'<' | b'>');
        if keep {
            out.push(*b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_linear_horizontal() {
        let p = GuideParams::from_url("http://127.0.0.1:23751/DIM/LINEAR/H/-50").unwrap();
        assert_eq!(p.guide_type, GuideType::Linear);
        assert_eq!(p.sub, Some(LinearSub::Horizontal));
        assert_eq!(p.dist, -50.0);
        assert_eq!(p.text, None);
    }

    #[test]
    fn parse_linear_vertical_with_text() {
        let p = GuideParams::from_url(
            "http://127.0.0.1:23751/DIM/LINEAR/V/-14.2?text=%3C%3E&tol=%2B0.1",
        )
        .unwrap();
        assert_eq!(p.sub, Some(LinearSub::Vertical));
        assert_eq!(p.dist, -14.2);
        assert_eq!(p.text.as_deref(), Some("<>"));
        assert_eq!(p.tol.as_deref(), Some("+0.1"));
    }

    #[test]
    fn parse_aligned_and_decimal_dist() {
        let p = GuideParams::from_url("http://127.0.0.1:9/DIM/LINEAR/A/10.0").unwrap();
        assert_eq!(p.sub, Some(LinearSub::Aligned));
        assert_eq!(p.dist, 10.0);
    }

    #[test]
    fn parse_other_types() {
        assert_eq!(
            GuideParams::from_url("http://x/DIM/RADIUS/8.0").unwrap().guide_type,
            GuideType::Radius
        );
        assert_eq!(
            GuideParams::from_url("http://x/DIM/DATUM/0").unwrap().guide_type,
            GuideType::Datum
        );
    }

    #[test]
    fn parse_tolerance_url_roundtrip() {
        use crate::guide_url::GuideType as GT;
        let p = GuideParams::from_url(
            "http://x/DIM/TOLERANCE/0?gdt=j&dia=1&gtol=0.05&d1=A&d2=B&d3=C",
        )
        .unwrap();
        assert_eq!(p.guide_type, GT::Tolerance);
        assert_eq!(p.gdt_sym.as_deref(), Some("j"));
        assert!(p.gdt_dia);
        assert_eq!(p.gdt_tol.as_deref(), Some("0.05"));
        assert_eq!(p.gdt_d1.as_deref(), Some("A"));
        assert_eq!(p.gdt_d2.as_deref(), Some("B"));
        assert_eq!(p.gdt_d3.as_deref(), Some("C"));
        // to_url 反向序列化 → 再解一次。
        let url = p.to_url(23751);
        assert!(url.contains("gdt=j") && url.contains("dia=1") && url.contains("gtol=0.05"));
        let back = GuideParams::from_url(&url).unwrap();
        assert_eq!(back.guide_type, GT::Tolerance);
        assert_eq!(back.gdt_tol, p.gdt_tol);
        assert_eq!(back.gdt_d3, p.gdt_d3);
        // 别名 GDT/TOL 都能解析。
        assert_eq!(
            GuideParams::from_url("http://x/DIM/GDT/0").unwrap().guide_type,
            GT::Tolerance
        );
        assert_eq!(
            GuideParams::from_url("http://x/DIM/TOL/0").unwrap().guide_type,
            GT::Tolerance
        );
    }

    #[test]
    fn parse_tolerance_multiple_rows_roundtrip() {
        use crate::guide_url::GuideType as GT;
        // rows=sym,dia,tol,mods,d1,d2,d3
        let url = "http://x/DIM/TOLERANCE/0?rows=f,0,0.02,,A,,";
        let p = GuideParams::from_url(url).unwrap();
        assert_eq!(p.guide_type, GT::Tolerance);
        assert_eq!(p.gdt_rows.len(), 1);
        assert_eq!(p.gdt_rows[0].sym, "f");
        assert!(!p.gdt_rows[0].dia);
        assert_eq!(p.gdt_rows[0].tol, "0.02");
        assert_eq!(p.gdt_rows[0].mods, "");
        assert_eq!(p.gdt_rows[0].d1, "A");
        // 两行：f 平行度 | A ；j 位置度 ⌀0.05 + m 修饰符 | A B C。
        let url2 = "http://x/DIM/TOLERANCE/0?rows=f,0,0.02,,A,,;j,1,0.05,m,A,B,C";
        let p2 = GuideParams::from_url(url2).unwrap();
        assert_eq!(p2.gdt_rows.len(), 2);
        assert_eq!(p2.gdt_rows[0].sym, "f");
        assert!(!p2.gdt_rows[0].dia);
        assert_eq!(p2.gdt_rows[1].sym, "j");
        assert!(p2.gdt_rows[1].dia);
        assert_eq!(p2.gdt_rows[1].tol, "0.05");
        assert_eq!(p2.gdt_rows[1].mods, "m");
        assert_eq!(p2.gdt_rows[1].d3, "C");
        // to_url 往返（行数 > 1 时输出 rows=）。
        let url = p2.to_url(23751);
        assert!(url.contains("rows="));
        let back = GuideParams::from_url(&url).unwrap();
        assert_eq!(back.gdt_rows, p2.gdt_rows);
        // 单行时 to_url 仍输出单字段（不产生 rows=）。
        let p1 = GuideParams::from_url("http://x/DIM/TOLERANCE/0?gdt=f&gtol=0.02&d1=A").unwrap();
        let url1 = p1.to_url(23751);
        assert!(!url1.contains("rows=") && url1.contains("gdt=f"));
    }

    #[test]
    fn parse_section_side_arrow() {
        use crate::guide_url::SectionSide as SS;
        // 默认：右侧 + 显示箭头。
        let p = GuideParams::from_url("http://x/DIM/SECTION/0").unwrap();
        assert_eq!(p.guide_type, GuideType::Section);
        assert_eq!(p.section_side, SS::Right);
        assert!(p.show_arrow);
        // 左侧 + 隐藏箭头 + 字母 + 比例 + 放置点。
        let p = GuideParams::from_url(
            "http://x/DIM/SECTION/0?side=L&show=0&let=B&scale=1%3A1&mx=10&my=-5",
        )
        .unwrap();
        assert_eq!(p.section_side, SS::Left);
        assert!(!p.show_arrow);
        assert_eq!(p.letter.as_deref(), Some("B"));
        assert_eq!(p.scale.as_deref(), Some("1:1"));
        assert_eq!(p.marker, Some((10.0, -5.0)));
        // SEC 别名。
        assert_eq!(
            GuideParams::from_url("http://x/DIM/SEC/0").unwrap().guide_type,
            GuideType::Section
        );
        // to_url 往返。
        let mut p2 = GuideParams::linear(LinearSub::Aligned, -14.2);
        p2.guide_type = GuideType::Section;
        p2.letter = Some("C".into());
        p2.scale = Some("1:2".into());
        p2.marker = Some((12.0, 34.0));
        let back = GuideParams::from_url(&p2.to_url(1)).unwrap();
        assert_eq!(back.guide_type, GuideType::Section);
        assert_eq!(back.letter.as_deref(), Some("C"));
        assert_eq!(back.scale.as_deref(), Some("1:2"));
        assert_eq!(back.marker, Some((12.0, 34.0)));
        // 左侧往返（side=L）。
        p2.section_side = SS::Left;
        p2.show_arrow = false;
        let back2 = GuideParams::from_url(&p2.to_url(1)).unwrap();
        assert_eq!(back2.section_side, SS::Left);
        assert!(!back2.show_arrow);
        // 默认往返（右侧+显箭）。
        let mut p3 = GuideParams::linear(LinearSub::Aligned, -14.2);
        p3.guide_type = GuideType::Section;
        let back3 = GuideParams::from_url(&p3.to_url(1)).unwrap();
        assert_eq!(back3.section_side, SS::Right);
        assert!(back3.show_arrow);
    }

    #[test]
    fn parse_weld_url_roundtrip() {
        use crate::guide_url::GuideType as GT;
        // 全参数：上=角焊、下=点焊（镜像版）、五开关全开、五文字槽。
        let url = "http://x/DIM/WELD/0?wu=%E8%A7%92%E7%84%8A&wl=%E7%82%B9%E7%84%8A&wdash=1&wcir=1&wflg=1&wtail=1&wgr=cvx&wut=5&wuq=100&wlt=3&wlq=50&wtt=%E5%B0%81%E5%BA%95%E7%84%8A";
        let p = GuideParams::from_url(url).unwrap();
        assert_eq!(p.guide_type, GT::Weld);
        assert_eq!(p.weld.upper, "角焊");
        assert_eq!(p.weld.lower, "点焊");
        assert!(p.weld.dash && p.weld.circle && p.weld.flag && p.weld.tail);
        assert_eq!(p.weld.grind_upper, GrindKind::ArcConvex);
        assert_eq!(p.weld.grind_lower, GrindKind::ArcConvex);
        assert_eq!(p.weld.up_thick, "5");
        assert_eq!(p.weld.up_qty, "100");
        assert_eq!(p.weld.lo_thick, "3");
        assert_eq!(p.weld.lo_qty, "50");
        assert_eq!(p.weld.tail_text, "封底焊");
        // to_url 往返。
        let back = GuideParams::from_url(&p.to_url(1)).unwrap();
        assert_eq!(back.weld, p.weld);
        // 缺省：全关全空；别名 焊接。
        let p0 = GuideParams::from_url("http://x/DIM/%E7%84%8A%E6%8E%A5/0").unwrap();
        assert_eq!(p0.guide_type, GT::Weld);
        assert_eq!(p0.weld, WeldParams::default());
        // 旧键 wc=1 向后兼容 → 打磨=弧·凸。
        let pc = GuideParams::from_url("http://x/DIM/WELD/0?wc=1").unwrap();
        assert_eq!(pc.weld.grind_upper, GrindKind::ArcConvex);
        assert_eq!(pc.weld.grind_lower, GrindKind::ArcConvex);
        // 6 种打磨方式解析 + 序列化往返。
        for k in [
            GrindKind::None, GrindKind::ArcConcave, GrindKind::ArcConvex,
            GrindKind::Line, GrindKind::DoubleArc, GrindKind::Zigzag,
        ] {
            let mut pg = GuideParams::linear(LinearSub::Aligned, 0.0);
            pg.guide_type = GT::Weld;
            pg.weld.grind_upper = k;
            pg.weld.grind_lower = k;
            let backg = GuideParams::from_url(&pg.to_url(1)).unwrap().weld;
            assert_eq!(backg.grind_upper, k);
            assert_eq!(backg.grind_lower, k);
            assert_eq!(GrindKind::from_str(k.as_str()), Some(k));
        }
        // 焊接方法字母：合法/非法/往返（仅 C/G/H/M/R/U）。
        for m in ["C", "G", "H", "M", "R", "U"] {
            let mut pm = GuideParams::linear(LinearSub::Aligned, 0.0);
            pm.guide_type = GT::Weld;
            pm.weld.method_upper = m.to_string();
            pm.weld.method_lower = m.to_string();
            let back = GuideParams::from_url(&pm.to_url(1)).unwrap();
            assert_eq!(back.weld.method_upper, m);
            assert_eq!(back.weld.method_lower, m);
        }
        assert_eq!(
            GuideParams::from_url("http://x/DIM/WELD/0?wm=Z").unwrap().weld.method_upper,
            ""
        );
        assert_eq!(
            GuideParams::from_url("http://x/DIM/WELD/0?wm=g").unwrap().weld.method_upper,
            "G",
            "小写转大写"
        );
        // 上下侧独立：wmu/wml
        let im = GuideParams::from_url("http://x/DIM/WELD/0?wmu=C&wml=U").unwrap();
        assert_eq!(im.weld.method_upper, "C");
        assert_eq!(im.weld.method_lower, "U");
        let rtm = GuideParams::from_url(&im.to_url(1)).unwrap();
        assert_eq!(rtm.weld.method_upper, "C");
        assert_eq!(rtm.weld.method_lower, "U");
        // 非法打磨代号 → 不打磨。
        let bad = GuideParams::from_url("http://x/DIM/WELD/0?wgr=xyz").unwrap();
        assert_eq!(bad.weld.grind_upper, GrindKind::None);
        // 上下侧独立：wgru/wgrl 分别解析并往返。
        let ind = GuideParams::from_url("http://x/DIM/WELD/0?wgru=zig&wgrl=cvx").unwrap();
        assert_eq!(ind.weld.grind_upper, GrindKind::Zigzag);
        assert_eq!(ind.weld.grind_lower, GrindKind::ArcConvex);
        let rt = GuideParams::from_url(&ind.to_url(1)).unwrap();
        assert_eq!(rt.weld.grind_upper, GrindKind::Zigzag);
        assert_eq!(rt.weld.grind_lower, GrindKind::ArcConvex);
        // “无”符号不上 URL（to_url 过滤），解析后仍为空串。
        let mut p1 = GuideParams::linear(LinearSub::Aligned, 0.0);
        p1.guide_type = GT::Weld;
        p1.weld.upper = "无".into();
        let back1 = GuideParams::from_url(&p1.to_url(1)).unwrap();
        assert_eq!(back1.weld.upper, "");
    }

    #[test]
    fn reject_non_dim_or_malformed() {
        assert!(GuideParams::from_url("http://x/FOO/LINEAR/H/1").is_none());
        assert!(GuideParams::from_url("http://x/DIM/LINEAR/H/abc").is_none());
        assert!(GuideParams::from_url("http://x/DIM/LINEAR").is_none());
        assert!(GuideParams::from_url("not-a-url").is_none());
    }

    #[test]
    fn roundtrip() {
        let p = GuideParams::linear(LinearSub::Vertical, -14.2);
        let url = p.to_url(23751);
        let back = GuideParams::from_url(&url).unwrap();
        assert_eq!(back, p);
        // 中文文字（公差/文字 percent-encode 往返）。
        let p2 = GuideParams {
            text: Some("50±0.05".into()),
            tol: Some("+0.1".into()),
            sym: Some("×2".into()),
            ..p
        };
        let url2 = p2.to_url(23751);
        let back2 = GuideParams::from_url(&url2).unwrap();
        assert_eq!(back2, p2);
    }

    #[test]
    fn dist_formatting() {
        let p = GuideParams::linear(LinearSub::Aligned, -14.2);
        assert!(p.to_url(1).ends_with("/DIM/LINEAR/A/-14.2"));
        let p2 = GuideParams::linear(LinearSub::Aligned, 50.0);
        assert!(p2.to_url(1).ends_with("/DIM/LINEAR/A/50"));
    }

    #[test]
    fn parse_dec_query() {
        // dec 参数解析；无 dec = None（生成端默认 2）。
        let p = GuideParams::from_url("http://x/DIM/DIAMETER/-9?dec=3").unwrap();
        assert_eq!(p.dec, Some(3));
        let p = GuideParams::from_url("http://x/DIM/DIAMETER/-9").unwrap();
        assert_eq!(p.dec, None);
        // 非法/越界：>8 截到 8；非数字 = None。
        let p = GuideParams::from_url("http://x/DIM/DIAMETER/-9?dec=99").unwrap();
        assert_eq!(p.dec, Some(8));
        let p = GuideParams::from_url("http://x/DIM/DIAMETER/-9?dec=abc").unwrap();
        assert_eq!(p.dec, None);
        // to_url 往返。
        let mut p = GuideParams::linear(LinearSub::Aligned, -14.2);
        p.dec = Some(2);
        let back = GuideParams::from_url(&p.to_url(1)).unwrap();
        assert_eq!(back.dec, Some(2));
    }

    #[test]
    fn parse_datum_ver_letter() {
        use crate::guide_url::DatumVersion as DV;
        // 默认 2008 / A。
        let p = GuideParams::from_url("http://x/DIM/DATUM/0").unwrap();
        assert_eq!(p.ver, DV::GB2008);
        assert_eq!(p.letter, None);
        // 1996 + 字母 B。
        let p = GuideParams::from_url("http://x/DIM/DATUM/0?ver=1996&let=B").unwrap();
        assert_eq!(p.ver, DV::GB1996);
        assert_eq!(p.letter.as_deref(), Some("B"));
        // 非法 ver 回退 2008。
        let p = GuideParams::from_url("http://x/DIM/DATUM/0?ver=2015").unwrap();
        assert_eq!(p.ver, DV::GB2008);
        // to_url 往返。
        let mut p = GuideParams::linear(LinearSub::Aligned, -14.2);
        p.ver = DV::GB1996;
        p.letter = Some("C".into());
        let back = GuideParams::from_url(&p.to_url(1)).unwrap();
        assert_eq!(back.ver, DV::GB1996);
        assert_eq!(back.letter.as_deref(), Some("C"));
    }

    #[test]
    fn parse_view_scale_flip_marker() {
        // 全部参数：字母 B + 比例 1:1 + 翻转 + 标记放置点(10,-5)。
        let p =
            GuideParams::from_url("http://x/DIM/VIEW/0?let=B&scale=1%3A1&flip=1&mx=10&my=-5")
                .unwrap();
        assert_eq!(p.guide_type, GuideType::View);
        assert_eq!(p.letter.as_deref(), Some("B"));
        assert_eq!(p.scale.as_deref(), Some("1:1"));
        assert_eq!(p.flip, FlipDir::CounterClockwise);
        assert_eq!(p.marker, Some((10.0, -5.0)));
        // 顺时针翻转：flip=cw → Clockwise；flip=ccw → CounterClockwise。
        let p = GuideParams::from_url("http://x/DIM/VIEW/0?flip=cw").unwrap();
        assert_eq!(p.flip, FlipDir::Clockwise);
        let p = GuideParams::from_url("http://x/DIM/VIEW/0?flip=ccw").unwrap();
        assert_eq!(p.flip, FlipDir::CounterClockwise);
        // 旧值 flip=1 向后兼容 → 逆时针。
        let p = GuideParams::from_url("http://x/DIM/VIEW/0?flip=1").unwrap();
        assert_eq!(p.flip, FlipDir::CounterClockwise);
        // 缺省：字母 A、无比例、不翻转、标记 (0,0)。
        let p = GuideParams::from_url("http://x/DIM/VIEW/0").unwrap();
        assert_eq!(p.letter, None);
        assert_eq!(p.scale, None);
        assert_eq!(p.flip, FlipDir::None);
        assert_eq!(p.marker, None);
        // 部分坐标（只给 mx）。
        let p = GuideParams::from_url("http://x/DIM/VIEW/0?mx=3.5").unwrap();
        assert_eq!(p.marker, Some((3.5, 0.0)));
        // 非法坐标 → None。
        assert!(GuideParams::from_url("http://x/DIM/VIEW/0?mx=abc").is_none());
        // to_url 往返。
        let mut p = GuideParams::linear(LinearSub::Aligned, -14.2);
        p.guide_type = GuideType::View;
        p.letter = Some("A".into());
        p.scale = Some("1:2".into());
        p.flip = FlipDir::CounterClockwise;
        p.marker = Some((12.0, 34.0));
        let back = GuideParams::from_url(&p.to_url(1)).unwrap();
        assert_eq!(back.letter.as_deref(), Some("A"));
        assert_eq!(back.scale.as_deref(), Some("1:2"));
        assert_eq!(back.flip, FlipDir::CounterClockwise);
        assert_eq!(back.marker, Some((12.0, 34.0)));
        // 顺时针 to_url 往返（flip=cw）。
        p.flip = FlipDir::Clockwise;
        let back = GuideParams::from_url(&p.to_url(1)).unwrap();
        assert_eq!(back.flip, FlipDir::Clockwise);
    }

    /// 族⑤：引导线 GUI 显示名 —— zh/en 双语（协议别名不译，仍可中文/英文解析）。
    #[test]
    fn guide_labels_switch_language_and_aliases_stay() {
        let _g = crate::global_state_test_lock();
        crate::i18n::clear_missing_keys();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        assert_eq!(LinearSub::Horizontal.label(), "水平");
        assert_eq!(LinearSub::Vertical.label(), "竖直");
        assert_eq!(LinearSub::Aligned.label(), "对齐");
        assert_eq!(SectionSide::Left.label(), "向左");
        assert_eq!(AngleMode::Reflex.label(), "优角");
        assert_eq!(GrindKind::ArcConcave.label(), "弧·凹");

        crate::i18n::set_lang(crate::i18n::Lang::En);
        assert_eq!(LinearSub::Horizontal.label(), "horizontal");
        assert_eq!(LinearSub::Vertical.label(), "vertical");
        assert_eq!(LinearSub::Aligned.label(), "aligned");
        assert_eq!(SectionSide::Left.label(), "left");
        assert_eq!(AngleMode::Reflex.label(), "reflex angle");
        assert_eq!(GrindKind::ArcConcave.label(), "concave arc");
        // 协议别名不译：英文短名照常解析（与语言无关）。
        assert_eq!(GuideParams::from_url("http://127.0.0.1:23751/DIM/LINEAR/H/-50").unwrap().sub, Some(LinearSub::Horizontal));
        assert!(crate::i18n::missing_keys().is_empty(), "{:?}", crate::i18n::missing_keys());
        crate::i18n::set_lang_auto();
    }
}
