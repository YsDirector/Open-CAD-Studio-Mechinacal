//! OCSMechanical（OCSM）机械工具包插件。
//!
//! 三个功能：
//! 1. `OCSM` 初始化：幂等创建 9 个机械制图图层（从 opencad.layers_quick
//!    迁移）+ 防御性线型 + 文字样式 `OCSM_GB`。
//! 2. 数字键 `1`~`10` 快速切层：无选中切当前图层（不记 undo）；有选中把对象
//!    移到目标层（记 undo、保留选择集，锁定层被宿主拒绝）。
//! 3. `TF` / `OCSMFRAMEINIT`：打开宿主侧图框选择窗口，从插件目录 `frame/`
//!    选择 DWG，填写比例 `值1:值2`（int 且其一为 1），点击后以块插入；
//!    插入比例 = 值2/值1，`比例`/`SCALE` 属性填入 `值1:值2`。
//!
//! 标注引导服务：
//! - `OCSMDIMGULIDE` / `GDIM`：选中引导线（`10引导线层` LINE）后弹浏览器
//!   GUI 配置标注参数；`应用` 写 PE_URL 超链接，`应用并刷新` 经标注更新
//!   服务器把引导线替换为真实标注（7标注层 / OCSM_GB）并 REGEN。
//! - `OCSMMCP`：确保标注更新服务器运行（独立 MCP 二进制经 TCP 桥接）。

mod balloon;
mod balloon_sync;
mod bom_xlsx;
mod bom;
mod ansi_table;
mod card;
mod card_expr;
mod card_lite;
mod card_report;
mod centerline;
mod detail;
mod detail_clip;
mod dim2gb;
mod din_table;
mod gear;
mod gear_table;
mod hole;
mod guide_server;
mod guide_url;
mod i18n;
mod invol_spline;
mod joint;
mod nf_table;
mod nf_ext_table;
mod partgen;
mod partgen_b1;
mod partgen_b2;
mod partgen_b3;
mod partgen_b4;
mod partgen_b5;
mod partgen_kit;
mod partgen_keys;
mod partgen_more;
mod shaft;
mod spline;
mod spline_gui;
mod spline_lite;
mod spline_table;
mod spline_tol;
mod thread;
pub mod tolerance;


use ocs_plugin_api::export_plugin;
use ocs_plugin_api::host::acadrust;
use ocs_plugin_api::host::acadrust::entities::Dimension;
use ocs_plugin_api::host::acadrust::types::{Color, LineWeight, Vector3};
use ocs_plugin_api::host::{
    BuiltinPlugin, CommandStep, DimStyleDef, FrameItem, HostApi, HostNotification,
    ImportFrameBlockRequest, InteractiveCommand, LayerDef, LinetypeDef, PluginRequestSender,
    TextStyleDef,
};
use ocs_plugin_api::manifest::{ApiVersion, PluginManifest};
use ocs_plugin_api::ribbon::{
    CadModule, IconKind, ModuleEvent, RibbonGroup, RibbonItem, ToolDef,
};

static MANIFEST: PluginManifest = PluginManifest {
    id: "opencad.ocsm",
    name: "OCSMechanical 机械工具包",
    // 单一来源：Cargo.toml 的 [package] version（= plugin.toml 的 [plugin] version，
    // 由 tests 守住；见 plugin_toml_version_matches_cargo_pkg_version）。
    version: env!("CARGO_PKG_VERSION"),
    description: "OCSM 初始化 + 数字键 1-10 快速图层 + 图框插入（TF）+ 智能标注（D）",
    api_version: ApiVersion { major: 5 },
    ribbon_order: 100,
    xdata_apps: &["OCSM_BOM"],
    command_prefixes: &[
        "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "TF", "OCSM",
        "OCSMFRAMEINIT", "OCSMFRAMEINSERT", "D", "OCSMPOWERDIM", "OCSMDIMGULIDE",
        "GDIM", "OCSMMCP", "OCSMRGH", "CC", "OCSMDIM2GB", "D2G", "OCSMEDIT", "ME",
        "OCSMPART", "XL", "OCSMJOINT", "OCSMHELP", "OH", "OCSMBOM", "BOM", "BOMSYNC", "OCSMBOMSYNC", "OCSMBOMCFG", "BOMCFG",
        "OCSMBOMEDIT", "BOMEDIT", "OCSMBOMLOCK", "BOMLOCK", "OCSMBOMXLSX", "BOMXLSX", "OCSMBOMXLSXI", "BOMXLSXI",
        "OCSMCENTERLINE", "ZX",
        "OCSMGEAR",
        "OCSMSHAFT",
        "OCSMHOLE",
        "DK",
        "OCSMCARD",
    ],
};

/// 一卡一方向（用户 2026-09-26）后的**旧 CLI 兼容**：若命令里没显式写 `内/外`，
/// 按卡方向在 `[std 体系]` 之后注入默认记号；写了就原样保留（旧写法可覆盖）。
pub(crate) fn inject_default_side(args: &str, side: &str) -> String {
    const SIDES: &[&str] = &[
        "内", "内部", "内花键", "int", "internal", "外", "外部", "外花键", "ext", "external",
    ];
    let mut toks: Vec<&str> = args.split_whitespace().collect();
    if toks.is_empty() {
        return side.to_string();
    }
    let mut at = 0usize;
    if matches!(
        toks[0].to_ascii_lowercase().as_str(),
        "std" | "标准" | "体系"
    ) {
        at = 2.min(toks.len());
    }
    if toks
        .get(at)
        .map(|t| SIDES.iter().any(|s| s.eq_ignore_ascii_case(t)))
        .unwrap_or(false)
    {
        return args.trim().to_string();
    }
    toks.insert(at, side);
    toks.join(" ")
}

/// 10 层模板（来自 opencad.layers_quick，名称数字后无空格；线型全部在
/// OCS 内置 OpenCADStudio.lin 表中）。第 10 层为引导线层：默认关闭（隐藏），
/// 线宽 0mm（本身不打印），用于承载 AI/用户画的引导线。
pub(crate) fn layer_defs() -> Vec<LayerDef> {
    let lw = |v: i16| LineWeight::from_value(v);
    vec![
        LayerDef {
            name: "1轮廓实线层".into(),
            color: Color::from_index(7),
            linetype: "Continuous".into(),
            lineweight: lw(35),
            plottable: true,
            off: false,
        },
        LayerDef {
            name: "2细线层".into(),
            color: Color::from_index(4),
            linetype: "Continuous".into(),
            lineweight: lw(18),
            plottable: true,
            off: false,
        },
        LayerDef {
            name: "3中心线层".into(),
            color: Color::from_index(1),
            linetype: "CENTER2".into(),
            lineweight: lw(18),
            plottable: true,
            off: false,
        },
        LayerDef {
            name: "4虚线层".into(),
            color: Color::from_index(6),
            linetype: "DASHED2".into(),
            lineweight: lw(18),
            plottable: true,
            off: false,
        },
        LayerDef {
            name: "5剖面线层".into(),
            color: Color::from_index(2),
            linetype: "Continuous".into(),
            lineweight: lw(18),
            plottable: true,
            off: false,
        },
        LayerDef {
            name: "6文字层".into(),
            color: Color::from_index(3),
            linetype: "Continuous".into(),
            lineweight: lw(18),
            plottable: true,
            off: false,
        },
        LayerDef {
            name: "7标注层".into(),
            color: Color::from_index(4),
            linetype: "Continuous".into(),
            lineweight: lw(18),
            plottable: true,
            off: false,
        },
        LayerDef {
            name: "8符号标注层".into(),
            color: Color::from_rgb(255, 191, 127),
            linetype: "Continuous".into(),
            lineweight: lw(18),
            plottable: true,
            off: false,
        },
        LayerDef {
            name: "9双点划线层".into(),
            color: Color::from_index(6),
            linetype: "DIVIDE2".into(),
            lineweight: lw(18),
            plottable: true,
            off: false,
        },
        // 引导线层：默认关闭（off），线宽 0mm 不打印。
        LayerDef {
            name: "10引导线层".into(),
            color: Color::from_index(5),
            linetype: "Continuous".into(),
            lineweight: lw(0),
            plottable: true,
            off: true,
        },
    ]
}

/// 防御性线型（宿主内置表已有则跳过；保证 DWG 写出时层引用的线型在表内）。
pub(crate) fn linetype_defs() -> Vec<LinetypeDef> {
    vec![
        LinetypeDef {
            name: "CENTER2".into(),
            description: "Center (.5x)".into(),
            elements: vec![19.05, -3.175, 3.175, -3.175],
        },
        LinetypeDef {
            name: "DASHED2".into(),
            description: "Dashed (.5x)".into(),
            elements: vec![6.35, -3.175],
        },
        LinetypeDef {
            name: "DIVIDE2".into(),
            description: "Divide (.5x)".into(),
            elements: vec![6.35, -3.175, 0.0, -3.175, 0.0, -3.175],
        },
        // 焊接符号第二基准线（虚线）：对照 焊接符号示例.dxf 的 ACISOWELD（__ __）。
        LinetypeDef {
            name: "ACISOWELD".into(),
            description: "Weld dash __ __ __".into(),
            elements: vec![2.0, -1.0],
        },
    ]
}

/// OCSM_GB 文字样式（固定高度 3.5、宽度因子 0.7、注释性）。
pub(crate) fn text_style_defs() -> Vec<TextStyleDef> {
    vec![TextStyleDef {
        name: "OCSM_GB".into(),
        font_file: "Unicode".into(),
        big_font_file: String::new(),
        true_type_font: "Zhuque Fangsong".into(),
        height: 3.5,
        width_factor: 0.7,
        annotative: true,
        is_shape_file: false,
        is_vertical: false,
    }]
}

/// OCSM_GB 标注样式：参数取自用户示例（标注对比.dxf 的 Mechanical 风格），
/// 但 dimtxsty 换成 OCSM_GB 字体（project.md 要求）。
pub(crate) fn dim_style_defs() -> Vec<DimStyleDef> {
    vec![DimStyleDef {
        name: "OCSM_GB".into(),
        make_current: true,
        dimtxt: 2.5,
        dimasz: 2.5,
        dimcen: 2.5,
        dimexe: 2.0,
        dimexo: 0.625,
        dimgap: 1.0,
        dimdli: 0.38,
        dimdle: 0.0,
        dimscale: 1.0,
        dimtad: 1,
        dimjust: 0,
        dimdec: 2,
        dimlunit: 2,
        dimzin: 8,
        dimaunit: 0,
        dimadec: 0,
        dimtih: false,
        // 文字强制水平（general.dxf DSTYLE XDATA dimtoh=1）。
        dimtoh: true,
        dimtix: false,
        dimsoxd: false,
        dimtofl: true,
        dimtol: false,
        dimtp: 0.0,
        dimtm: 0.0,
        dimtdec: 3,
        dimclrd: 130,
        dimclre: 130,
        dimclrt: 3,
        dimlwd: -1,
        dimlwe: -1,
        dimtxsty: "OCSM_GB".into(),
        dimpost: String::new(),
        // 类型专属 DIMVAR（黄金模板 OCSM_GB 实测值）：显式钉死，不再从当前样式继承——
        // 否则外来图档（TH_GBDIM/块内标注……）的旧值会漏进 OCSM_GB，
        // 表现为线性（dimlfac）/角度（dimazin/dimfrac）标注“样式丢失”。
        dimlfac: 1.0,
        dimtfac: 1.0,
        dimazin: 0,
        dimfrac: 0,
        dimtmove: 0,
        annotative: false,
    }]
}

/// 按 `scale` 生成缩放版标注样式（frame 比例感知用）：全部尺寸用 dimscale 表达，
/// 命名 `OCSM_GB_x{scale}`（如 OCSM_GB_x2、OCSM_GB_x0.5）。
pub(crate) fn scaled_dim_style_def(scale: f64) -> DimStyleDef {
    let base = dim_style_defs().into_iter().next().expect("base dim style");
    DimStyleDef {
        name: format!("OCSM_GB_x{}", trim_scale(scale)),
        make_current: false,
        dimscale: scale,
        ..base
    }
}

/// 缩放值转样式名片段：2 → "2"、0.5 → "0.5"、0.25 → "0.25"。
pub(crate) fn trim_scale(scale: f64) -> String {
    let s = format!("{scale:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() { "1".to_string() } else { s.to_string() }
}

/// 图框文件夹：优先 `OCSM_FRAME_DIR` 环境变量（测试/排障），否则为插件
/// 安装目录下的 `frame/`（插件 .so 由宿主以 `--ocs-plugin-runner <socket>
/// <cdylib>` 加载，取其父目录即可）。
/// 插件**安装目录**（`.so` 所在目录）：宿主以 `--ocs-plugin-runner <socket> <lib>` 启动插件，
/// 从该参数反解。`frame/` 与 `handbook/` 都挂在它下面（随插件分发）。
pub(crate) fn plugin_install_dir() -> Option<std::path::PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    for (i, arg) in args.iter().enumerate() {
        if arg == "--ocs-plugin-runner" {
            if let Some(lib) = args.get(i + 2) {
                let path = std::path::PathBuf::from(lib);
                if let Some(dirs) = path.parent() {
                    return Some(dirs.to_path_buf());
                }
            }
        }
    }
    None
}

fn frame_dir() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("OCSM_FRAME_DIR") {
        let dir = dir.trim();
        if !dir.is_empty() {
            return std::path::PathBuf::from(dir);
        }
    }
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "--ocs-plugin-runner" {
            if let (Some(_socket), Some(lib)) = (args.next(), args.next()) {
                let p = std::path::PathBuf::from(lib);
                if let Some(dir) = p.parent() {
                    return dir.join("frame");
                }
            }
            break;
        }
    }
    std::path::PathBuf::from("frame")
}

/// `OCSMBOM`/`BOM`/`OCSMBOMCFG`/`BOMCFG`/`BOMEDIT`（整行可能带参数，如 `BOM 30`）。
fn is_bom_command(cmd: &str) -> bool {
    let name = cmd
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(name.as_str(), "OCSMBOM" | "BOM" | "OCSMBOMCFG" | "BOMCFG" | "OCSMBOMSYNC" | "BOMSYNC" | "OCSMBOMLOCK" | "BOMLOCK" | "OCSMBOMXLSX" | "BOMXLSX" | "OCSMBOMXLSXI" | "BOMXLSXI" | "OCSMBOMEDIT" | "BOMEDIT")
}

struct OcsmPlugin;

/// **当前标签页的存盘状态缓存**（插件进程级）。
///
/// 宿主没有实现 `GetTabId` / `DocumentPath` 两个请求 → 引导服务的 HTTP 线程拿不到图纸
/// 路径；而命令路径的 `HostApi::document_path()` 是好的。于是**每次跑插件命令时刷新**这个
/// 缓存（`dispatch` 开头），HTTP 侧（序号标注落地时）读它来判断"这图存过盘没有"。
/// `None` = 还不知道（没跑过命令）；`Some((tab, None))` = 该标签页还没存过盘。
static DOC_SAVE_STATE: std::sync::OnceLock<std::sync::Mutex<Option<(u64, Option<std::path::PathBuf>)>>> =
    std::sync::OnceLock::new();

fn doc_save_state() -> &'static std::sync::Mutex<Option<(u64, Option<std::path::PathBuf>)>> {
    DOC_SAVE_STATE.get_or_init(|| std::sync::Mutex::new(None))
}

/// 命令路径调用：刷新"当前标签页是否存过盘 + 图纸路径"。
fn refresh_doc_save_state(host: &dyn HostApi) {
    let tab = host.tab_id();
    let path = host.document_path(tab);
    if let Ok(mut g) = doc_save_state().lock() {
        *g = Some((tab, path));
    }
}

/// HTTP 侧调用：`Some((tab, None))` = 这张图**还没存过盘**；`None` = 未知。
pub(crate) fn current_doc_unsaved() -> Option<u64> {
    let g = doc_save_state().lock().ok()?;
    match g.as_ref() {
        Some((tab, None)) => Some(*tab),
        _ => None,
    }
}

/// HTTP 侧调用：当前图纸路径（存过盘时）。
pub(crate) fn current_doc_path() -> Option<std::path::PathBuf> {
    let g = doc_save_state().lock().ok()?;
    g.as_ref().and_then(|(_, p)| p.clone())
}

/// 标注更新服务器端口（插件进程级；out-of-process 插件不能存宿主 state，
/// 用进程内 static 保存）。
/// 各插件页面窗口的最后心跳（页面每 5 s ping 一次；用来避免重复开窗）。
static PAGE_PING: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, std::time::Instant>>,
> = std::sync::OnceLock::new();

fn page_ping_map() -> &'static std::sync::Mutex<std::collections::HashMap<String, std::time::Instant>> {
    PAGE_PING.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// 页面心跳：`key` 标识页面（含 handle 这类区分项），`bye=true` 表示窗口正在关闭。
pub(crate) fn page_window_ping(key: &str, bye: bool) {
    let mut m = page_ping_map().lock().unwrap();
    if bye {
        m.remove(key);
    } else {
        m.insert(key.to_string(), std::time::Instant::now());
    }
}

/// 该页面窗口是否还开着（15 s 内有心跳）。
pub(crate) fn page_window_alive(key: &str) -> bool {
    page_ping_map()
        .lock()
        .unwrap()
        .get(key)
        .map(|t| t.elapsed() < std::time::Duration::from_secs(15))
        .unwrap_or(false)
}

/// 引导/标注配置窗口的页面心跳键：`guide-<句柄十六进制（去0x、小写）>[@<图纸号>]`。
///
/// 页面 (`guide_gui.html`) 按同一规则算键（句柄/图纸号从 URL 里取），所以插件的
/// 「窗口还开着就别重复开」判断真的生效；带上图纸号 → 两张图里同句柄的引导线
/// 各开各的窗口，不会互相挡住。
pub(crate) fn guide_ping_key(handle: acadrust::Handle, tab: Option<u64>) -> String {
    match tab {
        Some(tab) => format!("guide-{:x}@{tab}", u64::from(handle)),
        None => format!("guide-{:x}", u64::from(handle)),
    }
}

/// 打开某个插件页面为 **chromium `--app=` 独立窗口**（沉浸式；页面自关后窗口随之退出）。
/// 已开着（有心跳）则不再重复开；返回 true=已（尝试）打开，false=已开着。
pub(crate) fn open_plugin_page(
    port: u16,
    path_and_query: &str,
    ping_key: &str,
    width: u32,
    height: u32,
) -> bool {
    if page_window_alive(ping_key) {
        return false;
    }
    // `app=1` 标记：外部打开同一个页面（Ctrl+点击 PE_URL、粘地址栏）时，
    // 插件据此判断"这页本来就该在 app 窗口里"，不会再补一个窗口（见
    // `guide_server::upgrade_external_page`）。
    let url = format!(
        "http://127.0.0.1:{port}{}",
        crate::guide_server::with_app_marker(path_and_query)
    );
    if open_app_window(&url, width, height) {
        return true;
    }
    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
    true
}

/// 给页面 URL 接上 `tab=<图纸号>`（页面会把它带回给插件的每个请求，见
/// `guide_server::SenderRouter`）：**命令开窗时知道自己在哪张图上**，就把它钉死。
/// 拿不到 tab（`PE_URL` 链接、旧窗口）→ 不带参数：插件按「当前活跃图纸」算。
pub(crate) fn with_tab(path: &str, tab: Option<u64>) -> String {
    match tab {
        Some(tab) => format!(
            "{path}{}tab={tab}",
            if path.contains('?') { '&' } else { '?' }
        ),
        None => path.to_string(),
    }
}

/// 打开零件库页面：优先 **chromium `--app=` 独立窗口**（沉浸式，无地址栏/标签栏；
/// 且出库后页面能自己 `window.close()` 关掉它），失败时退回 `xdg-open`。
///
/// 坑：插件进程环境是**精简的**（宿主给 plugin runner 的 environ 里没有
/// `DISPLAY`/`WAYLAND_DISPLAY`），直接 spawn chromium 会以
/// "Missing X server or $DISPLAY" 失败 —— 所以先补齐会话的显示变量。
pub(crate) fn open_parts_window(port: u16, tab: Option<u64>) -> bool {
    open_plugin_page(port, &with_tab("/parts", tab), "parts", 960, 900)
}

/// 打开**齿轮**窗口（人用 GUI：参数表单 + 4 个视图按钮 + 实时预览；AI 走命令行/HTTP 同一实现）。
pub(crate) fn open_gear_window(port: u16, tab: Option<u64>) -> bool {
    open_plugin_page(port, &with_tab("/gear", tab), "gear", 980, 940)
}

/// 打开**轴生成器**窗口（人用 GUI：段表 + 行文本双向同步 + 实时预览；AI 走命令行/HTTP 同一实现）。
pub(crate) fn open_shaft_window(port: u16, tab: Option<u64>) -> bool {
    open_plugin_page(port, &with_tab("/shaft", tab), "shaft", 1120, 940)
}

/// 打开**孔生成器**窗口（人用 GUI：孔类型 + 参数 + 自动规则 + 实时预览；AI 走命令行/HTTP 同一实现）。
pub(crate) fn open_hole_window(port: u16, tab: Option<u64>) -> bool {
    open_plugin_page(port, &with_tab("/hole", tab), "hole", 860, 720)
}

/// 打开**智能卡片**窗口（人用 GUI：11 张卡，一卡一方向——GB 花键内/外、齿轮、
/// ANSI 内/外×中/英、NF 内/外、DIN 内/外；字段/读数/出表由卡类型表统一下发；
/// `OCSMCARD` 不带参数时打开）。
pub(crate) fn open_card_window(port: u16, tab: Option<u64>) -> bool {
    open_plugin_page(port, &with_tab("/spline", tab), "spline", 980, 900)
}

/// 打开**螺栓副装配**窗口（人用 GUI；AI 走命令行/HTTP 同一实现）。
pub(crate) fn open_joint_window(port: u16, tab: Option<u64>) -> bool {
    open_plugin_page(port, &with_tab("/joint", tab), "joint", 1080, 900)
}

/// 打开**命令手册**窗口（人类侧：命令目录 + 操作教程；教程 md 与 AI 的 skill 手册同一批文件）。
/// 手册页只读磁盘上的 md（不碰图纸）→ 不需要 `tab=`。
pub(crate) fn open_manual_window(port: u16) -> bool {
    open_plugin_page(port, "/manual", "manual", 1180, 940)
}

/// 用 chromium/chrome 的 `--app=` 打开独立窗口；成功返回 true。
pub(crate) fn open_app_window(url: &str, width: u32, height: u32) -> bool {
    let bin = [
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/google-chrome",
        "/usr/bin/microsoft-edge",
    ]
    .iter()
    .find(|p| std::path::Path::new(p).exists());
    let Some(bin) = bin else { return false };
    let mut cmd = std::process::Command::new(bin);
    cmd.arg(format!("--app={url}"))
        .arg(format!("--window-size={width},{height}"))
        .arg("--class=OCSM-Parts");
    let envs = session_display_env();
    // Chromium 默认走 X11：纯 Wayland 会话（没有 DISPLAY、只有 WAYLAND_DISPLAY）下
    // 它会以 “Missing X server or $DISPLAY” 秒退 —— 此时显式指 ozone 平台。
    // （2026-09-17 实机踩到：齿轮窗口起不来就是因为这个；以前的 X11 会话不需要这一条。）
    let has_x = envs.iter().any(|(k, _)| k == "DISPLAY");
    let has_wl = envs.iter().any(|(k, _)| k == "WAYLAND_DISPLAY");
    if has_wl && !has_x {
        cmd.arg("--ozone-platform=wayland");
    } else if has_wl {
        cmd.arg("--ozone-platform-hint=auto");
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.spawn().is_ok()
}

/// 取会话的显示相关环境变量（插件自身没有时分两步补齐）：
/// ① 自己环境里已有就用；② `systemctl --user show-environment`（X11/Wayland 会话都登记）；
/// ③ 再兜底按 `XDG_RUNTIME_DIR` 下的 wayland socket / xauth 文件推断。
/// 解析 `systemctl --user show-environment` 输出中的显示相关变量（纯函数，便于测试）。
pub(crate) fn parse_session_env(text: &str) -> Vec<(String, String)> {
    const KEYS: [&str; 6] = [
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XAUTHORITY",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
        "XDG_SESSION_TYPE",
    ];
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            if KEYS.contains(&k) && !v.trim().is_empty() && !out.iter().any(|(kk, _)| kk == k) {
                out.push((k.to_string(), v.trim().to_string()));
            }
        }
    }
    out
}

fn session_display_env() -> Vec<(String, String)> {
    const KEYS: [&str; 6] = [
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XAUTHORITY",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
        "XDG_SESSION_TYPE",
    ];
    let mut out: Vec<(String, String)> = Vec::new();
    let push = |out: &mut Vec<(String, String)>, k: &str, v: &str| {
        if !v.is_empty() && !out.iter().any(|(kk, _)| kk == k) {
            out.push((k.to_string(), v.to_string()));
        }
    };
    for k in KEYS {
        if let Ok(v) = std::env::var(k) {
            push(&mut out, k, &v);
        }
    }
    fn has_display(out: &[(String, String)]) -> bool {
        out.iter().any(|(k, _)| k == "DISPLAY" || k == "WAYLAND_DISPLAY")
    }
    if !has_display(&out) {
        if let Ok(o) = std::process::Command::new("systemctl")
            .args(["--user", "show-environment"])
            .output()
        {
            for (k, v) in parse_session_env(&String::from_utf8_lossy(&o.stdout)) {
                push(&mut out, &k, &v);
            }
        }
    }
    if !has_display(&out) {
        // 兜底：按运行时目录里的 socket / xauth 文件推断
        let rt = out
            .iter()
            .find(|(k, _)| k == "XDG_RUNTIME_DIR")
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| format!("/run/user/{}", unsafe { libc_getuid() }));
        if let Ok(rd) = std::fs::read_dir(&rt) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with("wayland-") && !name.ends_with(".lock") {
                    push(&mut out, "WAYLAND_DISPLAY", &name);
                } else if name.starts_with("xauth_") {
                    push(&mut out, "XAUTHORITY", &format!("{rt}/{name}"));
                }
            }
        }
        push(&mut out, "DISPLAY", ":0");
    }
    out
}

/// 当前用户 uid（Linux）。
unsafe fn libc_getuid() -> u32 {
    extern "C" {
        fn getuid() -> u32;
    }
    getuid()
}

/// 待放置零件（「零件出库」登记，`XL` 放置态读它做光标预览与落件）。
#[derive(Clone)]
pub(crate) struct PendingPart {
    pub block: String,
    pub meta_json: String,
    pub label: String,
    /// 带属性的块（花键参数表 21 项）：落件时按 ATTDEF 生成 ATTRIB 并施加 INSERT 变换。
    /// 普通零件/轴/孔为空。
    pub attrs: Vec<(acadrust::entities::AttributeDefinition, String)>,
    /// INSERT 缩放（默认 1.0；NF/DIN 卡整表 0.17 —— 与直接落点的 build_insert 同口径）。
    pub scale: f64,
}

impl PendingPart {
    /// 无属性块（标准件/齿轮/轴/孔等；既有调用点统一走这里）。
    pub(crate) fn new(
        block: impl Into<String>,
        meta_json: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        PendingPart {
            block: block.into(),
            meta_json: meta_json.into(),
            label: label.into(),
            attrs: Vec::new(),
            scale: 1.0,
        }
    }

    /// 带 ATTRIB 模板 + 取值的块（花键参数表：块里有 21 个 ATTDEF）。
    pub(crate) fn with_attrs(
        block: impl Into<String>,
        meta_json: impl Into<String>,
        label: impl Into<String>,
        attrs: Vec<(acadrust::entities::AttributeDefinition, String)>,
    ) -> Self {
        PendingPart {
            block: block.into(),
            meta_json: meta_json.into(),
            label: label.into(),
            attrs,
            scale: 1.0,
        }
    }

    /// 链式设置 INSERT 缩放（整表卡 NF/DIN 用 0.17；默认 1.0 = 不缩放）。
    pub(crate) fn with_scale(mut self, scale: f64) -> Self {
        if scale.is_finite() && scale > 0.0 {
            self.scale = scale;
        }
        self
    }
}

static PENDING_PART: std::sync::OnceLock<std::sync::Mutex<Option<PendingPart>>> =
    std::sync::OnceLock::new();

/// **测试用进程级串行锁**：`PENDING_PART` / `PENDING_JOINT` / `parts_point_slot` 等是进程级
/// 全局状态，测试并行时会互相覆盖（曾致 `shaft_export_registers_pending_part_and_direct_insert`
/// 偶发读到别的测试注册的块）。**任何读/写这些全局的测试都必须持本锁**；跨模块共享
/// （`guide_server::integration::export_lock()` 委托到这里），不要各自新建局部锁。
/// 锁只用于测试：生产路径不获取，不存在与业务锁的嵌套。
#[cfg(test)]
pub(crate) fn global_state_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static L: std::sync::Mutex<()> = std::sync::Mutex::new(());
    L.lock().unwrap_or_else(|e| e.into_inner())
}

fn pending_slot() -> &'static std::sync::Mutex<Option<PendingPart>> {
    PENDING_PART.get_or_init(|| std::sync::Mutex::new(None))
}

pub(crate) fn set_pending_part(p: PendingPart) {
    *pending_slot().lock().unwrap() = Some(p);
}

fn pending_part_label() -> Option<String> {
    pending_slot().lock().unwrap().as_ref().map(|p| p.label.clone())
}

fn pending_block() -> Option<String> {
    pending_slot().lock().unwrap().as_ref().map(|p| p.block.clone())
}

/// 测试用：取待放置件的元数据 JSON（校验轴块 `gears[]` 快照等）。
#[cfg(test)]
pub(crate) fn pending_part_meta_for_test() -> Option<String> {
    pending_slot().lock().unwrap().as_ref().map(|p| p.meta_json.clone())
}

/// 测试用：整份待放置件（花键参数表要断言 21 个 ATTRIB；落件路径 `place_one` 也用它）。
#[cfg(test)]
pub(crate) fn pending_part_for_test() -> Option<PendingPart> {
    pending_slot().lock().unwrap().clone()
}

/// 待放置的**件链**（螺栓副）：GUI 点「装配到图纸」后登记，图纸里点击落定。
///
/// 与单个零件（`PendingPart`）分开存：件链落定走 `apply_joint`（各件独立块 + 一次事务），
/// 而预览块（`OCSMJOINT_PREV_*`）内含整链裁好的几何，只用于光标跟随。
#[derive(Clone)]
pub(crate) struct PendingJoint {
    /// 件链规格 JSON（`at`/`rot` 在落定时被图纸点击覆盖）。
    pub spec_json: String,
    /// 光标预览块名。
    pub block: String,
    /// 命令行提示文本。
    pub label: String,
}

static PENDING_JOINT: std::sync::OnceLock<std::sync::Mutex<Option<PendingJoint>>> =
    std::sync::OnceLock::new();

fn pending_joint_slot() -> &'static std::sync::Mutex<Option<PendingJoint>> {
    PENDING_JOINT.get_or_init(|| std::sync::Mutex::new(None))
}

pub(crate) fn set_pending_joint(j: PendingJoint) {
    *pending_joint_slot().lock().unwrap() = Some(j);
}

fn pending_joint() -> Option<PendingJoint> {
    pending_joint_slot().lock().unwrap().clone()
}

/// 测试用：取当前待放置件链（guide_server 的集成测试断言用）。
pub(crate) fn pending_joint_for_test() -> Option<PendingJoint> {
    pending_joint()
}

/// 测试用：把图纸点选的基点/转角写进件链规格。
pub(crate) fn joint_spec_at_rot_for_test(spec_json: &str, pt: [f64; 3], rotation: f64) -> String {
    joint_spec_at_rot(spec_json, pt, rotation)
}

/// 一个待落件（基点位置 + 绕基点的旋转角，弧度 + **该落哪张图**的 sender）。
///
/// sender 必须跟着任务走，不能存在 worker 线程里：插件进程比图纸活得久，
/// 首次命住的那种图一旦被关掉，后续落件就会落到旧图（或永远没回应）。
#[derive(Clone)]
pub(crate) struct PlaceTask {
    pub pt: [f64; 3],
    pub rotation: f64,
    pub sender: std::sync::Arc<dyn PluginRequestSender>,
}

/// 落件通道（`XL` 放置态第二下点击 → 放置 worker 落一个件）。
static PLACE_TX: std::sync::OnceLock<std::sync::mpsc::Sender<PlaceTask>> =
    std::sync::OnceLock::new();

fn place_sender() -> std::sync::mpsc::Sender<PlaceTask> {
    PLACE_TX
        .get_or_init(|| {
            let (tx, rx) = std::sync::mpsc::channel::<PlaceTask>();
            std::thread::Builder::new()
                .name("ocsm-part-place".into())
                .spawn(move || {
                    for task in rx {
                        let pt = task.pt;
                        let sender = task.sender.clone();
                        // 宿主此刻不在命令回调里，worker 请求可安全下发。
                        // 与宿主交互事件处理的微小时序差：稍等一拍更稳。
                        std::thread::sleep(std::time::Duration::from_millis(120));
                        let p = pending_slot().lock().unwrap().clone();
                        let Some(p) = p else { continue };
                        if let Err(e) = place_one(&sender, &p, task) {
                            use ocs_plugin_api::ipc::protocol::PluginRequest;
                            let _ = sender.request(PluginRequest::PushError(format!(
                                "OCSM 标准件：放置失败 {e}"
                            )));
                        }
                    }
                })
                .ok();
            tx
        })
        .clone()
}

/// 在点处落一个件：INSERT + `OCSM_PART` 记录 + 标脏。
fn place_one(
    sender: &std::sync::Arc<dyn PluginRequestSender>,
    p: &PendingPart,
    task: PlaceTask,
) -> Result<(), String> {
    let pt = task.pt;
    use ocs_plugin_api::ipc::protocol::{PluginRequest, PluginResponse};
    use ocs_plugin_api::host::acadrust::entities::Insert;
    use ocs_plugin_api::host::acadrust::types::{Color, LineWeight, Vector3};
    let mut ins = Insert::new(&p.block, Vector3::new(pt[0], pt[1], pt[2]));
    // Insert.rotation 单位是**弧度**（同 Arc，DXF 读入已转换）
    ins.rotation = task.rotation;
    // 整表缩放（NF/DIN 卡 0.17）：与 `build_insert` 同口径；ATTRIB 随 INSERT 缩放。
    ins.set_x_scale(p.scale);
    ins.set_y_scale(p.scale);
    ins.set_z_scale(p.scale);
    {
        let c = &mut ins.common;
        c.layer = partgen::LAYER_MAIN.to_string();
        c.color = Color::ByLayer;
        c.linetype = "ByLayer".to_string();
        c.line_weight = LineWeight::ByLayer;
    }
    // 带属性块（花键参数表）：落件时把 21 个 ATTRIB 一起带上（与 CLI 同一构造口径）。
    if !p.attrs.is_empty() {
        use ocs_plugin_api::host::acadrust::entities::Entity as _;
        for (ad, val) in &p.attrs {
            let mut tmpl = ad.clone();
            tmpl.rotation = 0.0; // 旋转由 INSERT 变换施加
            let mut attr =
                ocs_plugin_api::host::acadrust::entities::AttributeEntity::from_definition(
                    &tmpl,
                    Some(val.clone()),
                );
            attr.apply_transform(&ins.get_transform());
            ins.attributes.push(attr);
        }
    }
    let resp = sender
        .request(PluginRequest::AddEntities(vec![acadrust::EntityType::Insert(ins)]))
        .map_err(|e| e.to_string())?;
    let handle = match resp {
        PluginResponse::Handles(hs) => hs.first().copied(),
        _ => None,
    };
    if let Some(h) = handle {
        let mut rec = ocs_plugin_api::host::acadrust::xdata::ExtendedDataRecord::new("OCSM_PART");
        rec.values.push(ocs_plugin_api::host::acadrust::xdata::XDataValue::String(
            p.meta_json.clone(),
        ));
        let _ = sender.request(PluginRequest::WriteRecord { handle: h, record: rec });
    }
    let _ = sender.request(PluginRequest::BumpGeometry);
    let _ = sender.request(PluginRequest::SetDirty);
    Ok(())
}

/// `OCSMPART`/`XL` 的参数化形式。
///
/// - **标准件**：`<族> <d> <l> [view <视图>] [at x,y] [rot 度]`
///   例：`OCSMPART hex_bolt_c 10 95 at 150,30 rot 0`、`OCSMPART hex_bolt_ab 8 40`。
/// - **结构要素**（`detail_*`）：没有长度 l，`<族> <d> [b1 <值>] [view <视图>] [at x,y] [rot 度]`
///   例：`OCSMPART detail_grind_od 100 b1 10 at 150,30 rot 0`（b1 缺省 = 该 d 档默认行）。
/// - **矩形花键**（`detail_spline_rect`）：`<族> <规格代号> [L 满齿段长] [de 覆盖] [view 视图] [at x,y] [rot]`
///   例：`XL detail_spline_rect 6x23x26x6 L30 de63 view side`（规格代号可自定义，de 表外规格必给）。
/// - **渐开线花键**：**只在 OCSMGEAR 的花键模式生成**（XL 旧入口与轴段 `INVOLSPLINE` 均已移除；
///   引擎仍在 `invol_spline.rs`）。
/// - **外螺纹退刀槽**（`detail_thread_relief`）：`<族> <d> P <螺距> [g1 值 g2 值 dg 值 r 值 alpha 值] [at x,y] [rot 度]`
///   例：`OCSMPART detail_thread_relief 20 P 1.5`（P 必给，其余可选，见 `detail.rs` 表 2）。
/// - **毂槽**（`detail_hub_keyway`，GB/T 1095-2003）：`<族> <d> [len 毂长] [view main|side] [at x,y] [rot 度]`
///   例：`OCSMPART detail_hub_keyway 25 len 30 view main`（b/t₂/r 由 d 查表；len 缺省 30）。
/// - **平键**（`key_1096_{a,b,c}` = GB/T 1096 三型；`key_1097_{a,b}` = GB/T 1097 两型）：
///   `<族> <b> <L> [view 视图] [at x,y] [rot 度]`（`d` 槽位承载键宽 b，h 查表派生；`l=0` = 该档默认 L）
///   例：`OCSMPART key_1096_a 4 8`（b=4、L=8）、`OCSMPART key_1096_b 22 0`、
///   `OCSMPART key_1097_a 8 25`（短键；L1/L2/L3 由 L 查 GB/T 1097 长度系列表派生）。
///
/// 解析失败（或参数为空）返回 `None` → 回退到原 GUI（零件库窗口 + 鼠标放置）流程。
#[derive(Debug, PartialEq)]
struct PartsSpec {
    family: String,
    d: f64,
    /// 标准件的长度；结构要素无长度（固定 0.0）。
    l: f64,
    /// 结构要素可选 b1 覆盖（标准件恒为 None）。
    b1: Option<f64>,
    /// 结构要素的通用数值参数（退刀槽的 P/g1/g2/dg/r/alpha；花键的 N/D/B/de/len）。
    params: std::collections::BTreeMap<String, f64>,
    /// 结构要素的规格代号（花键 `6x23x26x6`）。
    spec: Option<String>,
    view: String,
    at: Option<[f64; 2]>,
    rotation: Option<f64>,
}

/// 结构要素（矩形花键）的额外参数键：`N6` / `N 6` / `D26` / `B6` / `L30` / `de63` / `spec=…`；
/// 返回 `(规范键, 贴写值)`；贴写值为空串 = 值在下一个 token。`b1` 与退刀槽参数不走这里。
fn split_detail_param(token: &str) -> Option<(&'static str, &str)> {
    // 长前缀在前（`big` > `b`、`len` > `l`、`de`/`db` > `d`）；`b1` 是历史槽位，不拦截。
    const KEYS: [(&str, &str); 8] = [
        ("spec", "spec"),
        ("big", "big"),
        ("de", "de"),
        ("len", "len"),
        ("n", "n"),
        ("d", "big"),
        ("b", "b"),
        ("l", "len"),
    ];
    if token == "b1" {
        return None;
    }
    for (prefix, canonical) in KEYS {
        if token == prefix {
            return Some((canonical, ""));
        }
        if let Some(rest) = token.strip_prefix(prefix) {
            let rest = rest.strip_prefix(['=', ':']).unwrap_or(rest);
            if canonical == "spec" {
                if !rest.is_empty() {
                    return Some((canonical, rest));
                }
            } else if rest.chars().next().is_some_and(|c| {
                c.is_ascii_digit() || c == '.' || c == '-' || c == '+'
            }) {
                return Some((canonical, rest));
            }
        }
    }
    None
}

impl PartsSpec {
    fn parse(args: &str) -> Option<Self> {
        let mut tokens = args.split_whitespace();
        let family = tokens.next()?;
        if family.is_empty() || family.starts_with('-') {
            return None;
        }
        let family = family.to_ascii_lowercase();
        let second = tokens.next()?;
        // 结构要素（磨外圆等）没有长度，第二个数字参数后直接进关键字；
        // 花键的第二个参数是规格代号（`6x23x26x6`）—— 交给族解析。
        let detail = crate::detail::is_detail(&family);
        // 结构要素的默认视图：老要素只有 `main`；花键（无 main）默认出侧视图。
        let default_view = if detail {
            let views = crate::detail::family_views(&family);
            if views.contains(&"side") && !views.contains(&"main") {
                "side"
            } else {
                "main"
            }
        } else {
            "main"
        };
        let mut spec = PartsSpec {
            family,
            d: 0.0,
            l: 0.0,
            b1: None,
            params: std::collections::BTreeMap::new(),
            spec: None,
            view: default_view.to_string(),
            at: None,
            rotation: None,
        };
        if let Ok(d) = second.parse::<f64>() {
            if !(d.is_finite() && d > 0.0) {
                return None;
            }
            spec.d = d;
        } else if detail {
            let (d, _) = crate::detail::parse_spec_token(&spec.family, second)?;
            spec.d = d;
            spec.spec = Some(second.to_string());
        } else {
            return None;
        }
        if !detail {
            let l: f64 = tokens.next()?.parse().ok()?;
            if !(l.is_finite() && l > 0.0) {
                return None;
            }
            spec.l = l;
        }
        while let Some(token) = tokens.next() {
            // 关键字大小写不敏感（宿主把整条命令交给插件时可能是大写）。
            let lower = token.to_ascii_lowercase();
            // 结构要素（花键）的额外参数：`L30` / `de63` / `N6 D26 B6` / `spec=…`。
            if detail {
                if let Some((key, attached)) = split_detail_param(&lower) {
                    let value_text = if attached.is_empty() {
                        tokens.next()?
                    } else {
                        attached
                    };
                    if key == "spec" {
                        if value_text.is_empty() {
                            return None;
                        }
                        spec.spec = Some(value_text.to_string());
                    } else {
                        let value: f64 = value_text.parse().ok()?;
                        if !value.is_finite() {
                            return None;
                        }
                        spec.params.insert(key.to_string(), value);
                    }
                    continue;
                }
                // 规格代号族的裸数字 = L（例 `XL detail_spline_rect 6x23x26x6 30`）。
                if spec.spec.is_some() && !spec.params.contains_key("len") {
                    if let Ok(value) = lower.parse::<f64>() {
                        if !(value.is_finite() && value > 0.0) {
                            return None;
                        }
                        spec.params.insert("len".to_string(), value);
                        continue;
                    }
                }
            }
            match lower.as_str() {
                // 结构要素的可选 b1 覆盖（标准件不认这个关键字）。
                "b1" if detail => {
                    let b1: f64 = tokens.next()?.parse().ok()?;
                    if !b1.is_finite() {
                        return None;
                    }
                    spec.b1 = Some(b1);
                }
                // 结构要素的通用数值参数（退刀槽：P 必给，g1/g2/dg/r/alpha 可选）。
                // 不在这批关键字里的族会在生成时报“不认识参数 …”。
                key if detail && matches!(key, "p" | "g1" | "g2" | "dg" | "r" | "alpha") => {
                    let value: f64 = tokens.next()?.parse().ok()?;
                    if !value.is_finite() {
                        return None;
                    }
                    spec.params.insert(key.to_string(), value);
                }
                "view" | "--view" => {
                    let view = tokens.next()?;
                    if view.is_empty() {
                        return None;
                    }
                    spec.view = view.to_ascii_lowercase();
                }
                // `at 150,30` 与 `at 150 30` 都收。
                "at" | "@" => {
                    let first = tokens.next()?;
                    let (x, y) = match first.split_once(',') {
                        Some((x, y)) => (x.parse::<f64>().ok()?, y.parse::<f64>().ok()?),
                        None => (
                            first.parse::<f64>().ok()?,
                            tokens.next()?.parse::<f64>().ok()?,
                        ),
                    };
                    if !(x.is_finite() && y.is_finite()) {
                        return None;
                    }
                    spec.at = Some([x, y]);
                }
                "rot" | "rotation" | "--rot" => {
                    let deg: f64 = tokens.next()?.parse().ok()?;
                    if !deg.is_finite() {
                        return None;
                    }
                    spec.rotation = Some(deg);
                }
                // 多余/无法识别的 token：当作没用参数，回退 GUI 流程。
                _ => return None,
            }
        }
        Some(spec)
    }

    /// `/api/part_pick` 的请求体（与选择器页同一条生成/插入路径）。
    fn to_body(&self) -> String {
        let mut obj = serde_json::json!({
            "family": self.family,
            "d": self.d,
            "view": self.view,
        });
        // 结构要素没有 l（服务端 l 可缺省）；标准件必须带。
        if !crate::detail::is_detail(&self.family) {
            obj["l"] = serde_json::json!(self.l);
        }
        if let Some(b1) = self.b1 {
            obj["b1"] = serde_json::json!(b1);
        }
        if let Some(spec) = &self.spec {
            obj["spec"] = serde_json::json!(spec);
        }
        if !self.params.is_empty() {
            obj["params"] = serde_json::json!(self.params);
        }
        if let Some([x, y]) = self.at {
            obj["x"] = serde_json::json!(x);
            obj["y"] = serde_json::json!(y);
        }
        if let Some(rotation) = self.rotation {
            obj["rotation"] = serde_json::json!(rotation);
        }
        obj.to_string()
    }
}

/// `SP` 命令里点选的插入点（旧路径保留）。
static PARTS_POINT: std::sync::OnceLock<std::sync::Mutex<Option<[f64; 3]>>> =
    std::sync::OnceLock::new();

/// 件链落件通道（`OCSMJOINT` 放置态第二下点击 → worker 落整链）。
static JOINT_TX: std::sync::OnceLock<std::sync::mpsc::Sender<PlaceTask>> =
    std::sync::OnceLock::new();

fn joint_place_sender() -> std::sync::mpsc::Sender<PlaceTask> {
    JOINT_TX
        .get_or_init(|| {
            let (tx, rx) = std::sync::mpsc::channel::<PlaceTask>();
            std::thread::Builder::new()
                .name("ocsm-joint-place".into())
                .spawn(move || {
                    for task in rx {
                        // sender 跟着任务走：worker 不住首次命住的那张图（见 PlaceTask）
                        let sender = task.sender.clone();
                        // 宿主此刻不在命令回调里，但点击事件刚过：稍等一拍更稳（与零件放置同因）
                        std::thread::sleep(std::time::Duration::from_millis(120));
                        let Some(pending) = pending_joint() else { continue };
                        let body = joint_spec_at_rot(&pending.spec_json, task.pt, task.rotation);
                        use ocs_plugin_api::ipc::protocol::PluginRequest;
                        match crate::guide_server::apply_joint(&sender, body.as_bytes()) {
                            Ok(out) => {
                                let report = serde_json::from_str::<serde_json::Value>(&out)
                                    .ok()
                                    .and_then(|v| v["report"].as_str().map(str::to_string))
                                    .unwrap_or(out);
                                let _ = sender.request(PluginRequest::PushOutput(report));
                            }
                            Err(e) => {
                                let _ = sender.request(PluginRequest::PushError(format!(
                                    "OCSMJOINT 落件失败：{e}"
                                )));
                            }
                        }
                    }
                })
                .expect("joint-place worker");
            tx
        })
        .clone()
}

/// 把图纸点选的基点/转角写进件链规格（覆盖 GUI 里的 at/rot）。
fn joint_spec_at_rot(spec_json: &str, pt: [f64; 3], rotation: f64) -> String {
    let mut value: serde_json::Value =
        serde_json::from_str(spec_json).unwrap_or_else(|_| serde_json::json!({}));
    value["at"] = serde_json::json!([pt[0], pt[1]]);
    value["rot"] = serde_json::json!(rotation.to_degrees());
    value.to_string()
}

/// `OCSMJOINT` 放置态：与零件放置同体验（基点 → 光标旋转 → 落定）。
struct JointPlace {
    sender: std::sync::Arc<dyn PluginRequestSender>,
    phase: std::cell::Cell<PlacePhase>,
    base: std::cell::Cell<[f64; 3]>,
}

impl InteractiveCommand for JointPlace {
    fn prompt(&self) -> String {
        match (pending_joint().map(|p| p.label), self.phase.get()) {
            (None, _) => {
                "OCSM 螺栓副：请在螺栓副窗口里点「装配到图纸」，然后在此点击放置。".to_string()
            }
            (Some(l), PlacePhase::Follow) => {
                format!("OCSM 螺栓副：{l} —— 点击定位基点（可连续，Esc 结束）")
            }
            (Some(l), PlacePhase::Rotate) => format!(
                "OCSM 螺栓副：{l} —— 移动光标绕基点旋转，再点击落定（Esc 取消）"
            ),
        }
    }

    fn wants_mouse_move(&self) -> bool {
        true
    }

    fn on_mouse_move(&mut self, pt: [f64; 3]) -> Option<acadrust::EntityType> {
        let pending = pending_joint()?;
        let (at, rot) = match self.phase.get() {
            PlacePhase::Follow => (pt, 0.0),
            PlacePhase::Rotate => (self.base.get(), rotate_angle(self.base.get(), pt)),
        };
        let mut ins = acadrust::entities::Insert::new(&pending.block, Vector3::new(at[0], at[1], at[2]));
        ins.rotation = rot;
        {
            let c = &mut ins.common;
            c.layer = partgen::LAYER_MAIN.to_string();
            c.color = Color::ByLayer;
            c.linetype = "ByLayer".to_string();
            c.line_weight = LineWeight::ByLayer;
        }
        Some(acadrust::EntityType::Insert(ins))
    }

    fn on_point(&mut self, pt: [f64; 3]) -> CommandStep {
        if pending_joint().is_none() {
            // 还没在窗口里点「装配到图纸」：先记下点，等窗口那边登记
            *parts_point_slot().lock().unwrap() = Some(pt);
            return CommandStep::NeedPoint;
        }
        match self.phase.get() {
            PlacePhase::Follow => {
                self.base.set(pt);
                self.phase.set(PlacePhase::Rotate);
                CommandStep::NeedPoint
            }
            PlacePhase::Rotate => {
                let task = PlaceTask {
                    pt: self.base.get(),
                    rotation: rotate_angle(self.base.get(), pt),
                    sender: self.sender.clone(),
                };
                let _ = joint_place_sender().send(task);
                CommandStep::Done
            }
        }
    }
}

fn parts_point_slot() -> &'static std::sync::Mutex<Option<[f64; 3]>> {
    PARTS_POINT.get_or_init(|| std::sync::Mutex::new(None))
}

pub(crate) fn take_parts_point() -> Option<[f64; 3]> {
    parts_point_slot().lock().unwrap().take()
}

static GUIDE_PORT: std::sync::OnceLock<std::sync::Mutex<Option<u16>>> =
    std::sync::OnceLock::new();

/// 引导服务器「请求该发给哪张图」的路由表（进程级）。
///
/// 端口在进程级复用，但 sender 绑定标签页：关掉一张图 / 新建一张之后必须换成
/// 当前图的 sender，否则服务器会一直对着旧图（旧图被关掉时宿主根本不回答那个
/// `tab_id` 的请求 → GUI 5 s 超时「读不到引导线」）。详见 `guide_server::SenderRouter`。
static GUIDE_ROUTER: std::sync::OnceLock<std::sync::Arc<guide_server::SenderRouter>> =
    std::sync::OnceLock::new();

/// 宿主通知里的「当前活跃标签页」（`SelectionChangedV4` 只对活动标签页发，见
/// `notify_plugins_selection_changed`）。比「最近一次命令所在的图」更准：用户切个
/// 标签页不动命令，插件也知道现在看的是哪张图。
///
/// 单独存一份：通知可能在引导服务器（路由表）创建**之前**就到了。
static HOST_ACTIVE_TAB: std::sync::OnceLock<std::sync::Mutex<Option<u64>>> =
    std::sync::OnceLock::new();

pub(crate) fn host_active_tab() -> Option<u64> {
    *HOST_ACTIVE_TAB
        .get_or_init(|| std::sync::Mutex::new(None))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// 处理宿主通知（`BuiltinPlugin::on_notification`）：只维护路由表需要的两条信息。
pub(crate) fn on_host_notification(notification: &HostNotification) {
    match notification {
        // 活动标签页换了（切图、改选择都会发）→ 无 `tab=` 的请求跟它走。
        HostNotification::SelectionChangedV4 { tab_id, .. } => {
            *HOST_ACTIVE_TAB
                .get_or_init(|| std::sync::Mutex::new(None))
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(*tab_id);
            if let Some(router) = GUIDE_ROUTER.get() {
                router.set_active(*tab_id);
            }
        }
        // 标签页关了 → 那张图上的窗口请求立即报错（而不是 5 s 超时），
        // 也不会退化成「写到当前图」。
        HostNotification::DocumentTabClosed { tab_id } => {
            {
                let mut active = HOST_ACTIVE_TAB
                    .get_or_init(|| std::sync::Mutex::new(None))
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                if *active == Some(*tab_id) {
                    *active = None; // 等宿主的下一条 SelectionChangedV4
                }
            }
            if let Some(router) = GUIDE_ROUTER.get() {
                router.mark_closed(*tab_id);
            }
        }
        _ => {}
    }
}

/// 登记 `sender`（`tab` 已知时同时按标签页登记），并把它设为默认目标。
/// 返回进程级路由表（服务器线程持有同一个）。
fn guide_router(
    tab: Option<u64>,
    sender: std::sync::Arc<dyn PluginRequestSender>,
) -> std::sync::Arc<guide_server::SenderRouter> {
    let router = GUIDE_ROUTER.get_or_init(|| {
        std::sync::Arc::new(guide_server::SenderRouter::new(
            std::sync::Arc::clone(&sender),
            host_active_tab(),
        ))
    });
    match tab {
        Some(tab) => router.set_current(tab, sender),
        None => router.set_default_sender(sender),
    }
    std::sync::Arc::clone(router)
}

/// 确保引导服务器在跑（端口进程级防重），并把当前图的 sender 记进路由表。
/// 拿不到 sender 时返回 None。
fn ensure_guide_server_running(sender: std::sync::Arc<dyn PluginRequestSender>, tab: Option<u64>) -> Option<u16> {
    let router = guide_router(tab, sender);
    let mut port_slot = GUIDE_PORT.get_or_init(|| std::sync::Mutex::new(None)).lock().unwrap();
    if let Some(port) = *port_slot {
        return Some(port);
    }
    let server = crate::guide_server::spawn(router)?;
    *port_slot = Some(server.port);
    Some(server.port)
}

/// 当前标注更新服务器端口（未启动时 None）。guide_server 回写"可编辑链接"要用。
pub(crate) fn current_guide_port() -> Option<u16> {
    GUIDE_PORT.get().and_then(|m| *m.lock().unwrap())
}

/// 写端口（测试用：spawn() 起的测试服务器也要能让挂链逻辑拿到端口）。
#[cfg(test)]
pub(crate) fn set_guide_port_for_test(port: u16) {
    *GUIDE_PORT
        .get_or_init(|| std::sync::Mutex::new(None))
        .lock()
        .unwrap() = Some(port);
}

/// 上次已刷新过链接的端口（避免每次调用都扫图）。
static EDIT_LINK_PORT: std::sync::OnceLock<std::sync::Mutex<Option<u16>>> =
    std::sync::OnceLock::new();

/// 该实体的 `PE_URL` 是否需要重写成当前端口。
/// 需要：无链接 / 链接指向别的端口 / 链接里的 handle 对不上（实体被复制过）。
fn needs_link_rewrite(stored: Option<&str>, port: u16, handle: acadrust::Handle) -> bool {
    let want = format!("/guide.html?handle={:#X}", u64::from(handle));
    match stored {
        Some(u) => {
            !u.contains(&want)
                || !u
                    .split_once("127.0.0.1:")
                    .map(|(_, rest)| rest.starts_with(&format!("{port}/")))
                    .unwrap_or(false)
        }
        None => true,
    }
}

/// 把图纸里带 `OCSM_EDIT` 的标注的 `PE_URL` 重写到当前端口（只写真正需要改的），
/// 使“Ctrl+点击标注回编辑 GUI”在插件换端口后仍然有效。
fn refresh_edit_links(host: &mut dyn HostApi, port: u16) {
    let slot = EDIT_LINK_PORT.get_or_init(|| std::sync::Mutex::new(None));
    {
        let mut last = slot.lock().unwrap();
        if *last == Some(port) {
            return;
        }
        *last = Some(port);
    }
    // 先收集目标（读、写分开借用 host）。
    let targets: Vec<(acadrust::Handle, Option<String>)> = {
        let doc = host.document();
        doc.entities()
            .filter(|e| e.common().extended_data.get_record("OCSM_EDIT").is_some())
            .map(|e| {
                let h = e.common().handle;
                let url = e
                    .common()
                    .extended_data
                    .get_record("PE_URL")
                    .and_then(|r| {
                        r.values.iter().find_map(|v| match v {
                            acadrust::xdata::XDataValue::String(s) if !s.is_empty() => Some(s.clone()),
                            _ => None,
                        })
                    });
                (h, url)
            })
            .collect()
    };
    let mut changed = 0usize;
    for (h, stored) in targets {
        if !needs_link_rewrite(stored.as_deref(), port, h) {
            continue;
        }
        let gui = format!(
            "http://127.0.0.1:{port}/guide.html?handle={:#X}",
            u64::from(h)
        );
        if host.write_record(h, crate::guide_server::pe_url_record(&gui)) {
            changed += 1;
        }
    }
    if changed > 0 {
        host.set_dirty();
        host.push_info(&format!(
            "OCSM：已更新 {changed} 个标注的编辑链接（端口 {port}）。"
        ));
    }
    // ── 明细表块（表头 + 行块）：链接是同一张编辑页（bom.html，不带 handle）──
    // 老图纸里这里可能还存着指向 xlsx 文件路径的死链接（宿主 web_hyperlink
    // 只放行 http/https），一并重写掉。
    let bom_targets: Vec<(acadrust::Handle, Option<String>)> = {
        let doc = host.document();
        doc.entities()
            .filter(|e| {
                e.common()
                    .extended_data
                    .get_record(crate::bom::XDATA_BOM)
                    .is_some()
            })
            .map(|e| {
                let h = e.common().handle;
                let url = e
                    .common()
                    .extended_data
                    .get_record("PE_URL")
                    .and_then(|r| {
                        r.values.iter().find_map(|v| match v {
                            acadrust::xdata::XDataValue::String(s) if !s.is_empty() => Some(s.clone()),
                            _ => None,
                        })
                    });
                (h, url)
            })
            .collect()
    };
    let mut bom_changed = 0usize;
    for (h, stored) in bom_targets {
        if crate::bom::bom_link_is_current(stored.as_deref(), port) {
            continue;
        }
        if host.write_record(
            h,
            crate::guide_server::pe_url_record(&crate::bom::bom_edit_url(port)),
        ) {
            bom_changed += 1;
        }
    }
    if bom_changed > 0 {
        host.set_dirty();
        host.push_info(&format!(
            "OCSM：已把 {bom_changed} 个明细表块链接指向编辑页（Ctrl+点击打开）。"
        ));
    }
}

// ── 功能区图标：双色扁平化 SVG ──────────────────────────────────
//
// 24×24 网格，主结构色 `#B4B6B9`、强调色 `#6DB7ED`；宿主 `icons::semantic()`
// （src/ui/icons.rs）按当前主题把这两个 token 解析为「文字色 / 主色」，因此同一套
// 图在深色与浅色主题下都清晰。SVG 用 `include_bytes!` 编译进 .so，部署不需要
// 额外资源文件（tools/deploy_plugin.sh 只拷贝 .so / plugin.toml / handbook）。
const ICON_FRAME: IconKind = IconKind::Svg(include_bytes!("../assets/icons/frame.svg"));
const ICON_CENTERLINE: IconKind =
    IconKind::Svg(include_bytes!("../assets/icons/centerline.svg"));
const ICON_GEAR: IconKind = IconKind::Svg(include_bytes!("../assets/icons/gear.svg"));
const ICON_SHAFT: IconKind = IconKind::Svg(include_bytes!("../assets/icons/shaft.svg"));
const ICON_POWERDIM: IconKind =
    IconKind::Svg(include_bytes!("../assets/icons/powerdim.svg"));
const ICON_DIMGUIDE: IconKind =
    IconKind::Svg(include_bytes!("../assets/icons/dimguide.svg"));
const ICON_DIM2GB: IconKind = IconKind::Svg(include_bytes!("../assets/icons/dim2gb.svg"));
const ICON_PARTS: IconKind = IconKind::Svg(include_bytes!("../assets/icons/parts.svg"));
const ICON_HOLE: IconKind = IconKind::Svg(include_bytes!("../assets/icons/hole.svg"));
const ICON_CARD: IconKind = IconKind::Svg(include_bytes!("../assets/icons/card.svg"));

impl BuiltinPlugin for OcsmPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }

    fn ribbon(&self) -> Box<dyn CadModule> {
        struct OcsmModule;
        impl CadModule for OcsmModule {
            fn id(&self) -> &'static str {
                MANIFEST.id
            }
            fn title(&self) -> &'static str {
                "OCSMechanical 机械工具包"
            }
            fn ribbon_groups(&self) -> &[RibbonGroup] {
                static GROUPS: std::sync::OnceLock<Vec<RibbonGroup>> =
                    std::sync::OnceLock::new();
                GROUPS.get_or_init(|| {
                    vec![
                        RibbonGroup {
                            title: "图框",
                            tools: vec![RibbonItem::LargeTool(ToolDef {
                                id: "OCSMFRAMEINIT",
                                label: "插入图幅",
                                icon: ICON_FRAME,
                                event: ModuleEvent::Command("OCSMFRAMEINIT".to_string()),
                            })],
                        },
                        RibbonGroup {
                            title: "中心线",
                            tools: vec![RibbonItem::LargeTool(ToolDef {
                                id: "OCSMCENTERLINE",
                                label: "中心线",
                                icon: ICON_CENTERLINE,
                                event: ModuleEvent::Command("OCSMCENTERLINE".to_string()),
                            })],
                        },
                        RibbonGroup {
                            title: "齿轮",
                            tools: vec![RibbonItem::LargeTool(ToolDef {
                                id: "OCSMGEAR",
                                label: "齿轮",
                                icon: ICON_GEAR,
                                event: ModuleEvent::Command("OCSMGEAR".to_string()),
                            })],
                        },
                        RibbonGroup {
                            title: "轴",
                            tools: vec![RibbonItem::LargeTool(ToolDef {
                                id: "OCSMSHAFT",
                                label: "轴生成器",
                                icon: ICON_SHAFT,
                                event: ModuleEvent::Command("OCSMSHAFT".to_string()),
                            })],
                        },
                        RibbonGroup {
                            title: "孔",
                            tools: vec![RibbonItem::LargeTool(ToolDef {
                                id: "OCSMHOLE",
                                label: "孔生成器",
                                icon: ICON_HOLE,
                                event: ModuleEvent::Command("OCSMHOLE".to_string()),
                            })],
                        },
                        RibbonGroup {
                            title: "标准件",
                            tools: vec![RibbonItem::LargeTool(ToolDef {
                                id: "OCSMPART",
                                label: "标准件库",
                                icon: ICON_PARTS,
                                event: ModuleEvent::Command("OCSMPART".to_string()),
                            })],
                        },
                        // ⑥ 智能卡片：与标准件库同款的 LargeTool；不带参数 = 开卡片窗口 + 放置态。
                        RibbonGroup {
                            title: "卡片",
                            tools: vec![RibbonItem::LargeTool(ToolDef {
                                id: "OCSMCARD",
                                label: "智能卡片",
                                icon: ICON_CARD,
                                event: ModuleEvent::Command("OCSMCARD".to_string()),
                            })],
                        },
                        RibbonGroup {
                            title: "标注",
                            tools: vec![
                                RibbonItem::LargeTool(ToolDef {
                                    id: "OCSMPOWERDIM",
                                    label: "智能标注",
                                    icon: ICON_POWERDIM,
                                    event: ModuleEvent::Command("OCSMPOWERDIM".to_string()),
                                }),
                                RibbonItem::LargeTool(ToolDef {
                                    id: "OCSMDIMGULIDE",
                                    label: "尺寸引导",
                                    icon: ICON_DIMGUIDE,
                                    event: ModuleEvent::Command("OCSMDIMGULIDE".to_string()),
                                }),
                                RibbonItem::LargeTool(ToolDef {
                                    id: "OCSMDIM2GB",
                                    label: "标注转GB",
                                    icon: ICON_DIM2GB,
                                    event: ModuleEvent::Command("OCSMDIM2GB".to_string()),
                                }),
                            ],
                        },
                    ]
                })
            }
        }
        Box::new(OcsmModule)
    }

    /// 宿主通知（API v4+）：维护「当前活跃图纸」，并驱动新建空图自动初始化。
    ///
    /// `SelectionChangedV4` / `DocumentTabClosed` 是宿主主动推的（不靠命令），
    /// 所以用户**只切标签页不跑命令**时插件也知道现在看的是哪张图；图纸被关掉
    /// 时也能立刻拒绝那张图上的窗口请求（而不是干等 5 s 超时）。
    /// `SelectionChangedV4`（建图/切图）与 `DocumentChangedV4` 都触发一次
    /// 「新建空图」检查 → 是则经 `on_load` 缓存的 sender 立即初始化，不等命令。
    fn on_load(&mut self, host: &mut dyn HostApi) {
        // API v5：拿通知路径要用的 worker sender + 记下启动页 tab id。
        crate::on_load_auto_init(host);
    }

    fn on_notification(&mut self, _command_id: Option<u64>, notification: HostNotification) {
        // 建图/切图/文档变化 → 不等命令，立即查一次「新建空图」并初始化。
        match &notification {
            HostNotification::SelectionChangedV4 { tab_id, .. }
            | HostNotification::DocumentChangedV4 { tab_id, .. } => {
                crate::maybe_auto_init_after_notification(*tab_id);
            }
            _ => {}
        }
        crate::on_host_notification(&notification);
    }

    fn dispatch(&self, host: &mut dyn HostApi, cmd: &str) -> bool {
        // auto-start：宿主把每条命令（含核心命令）都路由给插件 dispatch，
        // 在这里幂等启动标注更新服务器（GUIDE_PORT 防重），MCP 客户端无需
        // 先手动运行 OCSMMCP/GDIM 即可连接。失败静默，GDIM/OCSMMCP 会给出
        // 明确错误。
        let _ = self.ensure_guide_server(host);
        // 顺路刷新"这图存过盘没有"（HTTP 侧要用来提示 xlsx 落点）。
        refresh_doc_save_state(host);
        // 命令名（大写，大小写不敏感）+ 其后的参数（**保留原大小写**，供参数化命令
        // 解析：`OCSMPART hex_bolt_c 10 95 at 150,30 rot 0`）。
        let raw = cmd.trim();
        let (name, rest) = match raw.split_once(char::is_whitespace) {
            Some((name, rest)) => (name, rest.trim()),
            None => (raw, ""),
        };
        let upper = name.to_ascii_uppercase();
        // 新建空图纸 → 自动 OCSM 初始化（旧图不动作）。必须在具体命令分发**前**：
        // OCSMGEAR/OCSMSHAFT 的 `ocsm_ready` 拦截这时就能看到刚建好的图层/样式。
        self.auto_init_new_document(host, &upper);
        match upper.as_str() {
            "OCSM" => {
                self.cmd_init(host);
                true
            }
            "TF" | "OCSMFRAMEINIT" => {
                self.cmd_frame_init(host, rest);
                true
            }
            "OCSMFRAMEINSERT" => {
                self.cmd_frame_insert(host, rest);
                true
            }
            "D" | "OCSMPOWERDIM" => {
                self.cmd_powerdim(host);
                true
            }
            // 中心线：点圆/圆弧 → 十字；点两根直线 → 角平分线（`3中心线层`）
            "OCSMCENTERLINE" | "ZX" => {
                self::centerline::cmd_centerline(host);
                true
            }
            // 齿轮（外齿轮 / 内齿轮）：不带参数 = 齿轮窗口 + 放置态；带参数 = 一行直插
            "OCSMGEAR" => {
                self.cmd_gear(host, rest);
                true
            }
            // 轴生成器（一期骨架）：行 DSL / JSON 一行直插（单视图侧视图）
            "OCSMSHAFT" => {
                self.cmd_shaft(host, rest);
                true
            }
            // 孔生成器：不带参数 = 孔窗口 + 放置态；带参数 = 一行直插
            "OCSMHOLE" | "DK" => {
                self.cmd_hole(host, rest);
                true
            }
            "OCSMDIMGULIDE" | "GDIM" => {
                self.cmd_guide(host);
                true
            }
            "OCSMEDIT" | "ME" => {
                self.cmd_edit(host);
                true
            }
            "OCSMRGH" | "CC" => {
                self.cmd_roughness(host);
                true
            }
            // 按件链装配螺栓副（确定性执行器）：`OCSMJOINT at … bolt=… plate=… nut=…`
            "OCSMHELP" | "OH" => {
                self.cmd_help(host);
                true
            }
            "OCSMJOINT" => {
                self.cmd_joint(host, rest);
                true
            }
            "OCSMPART" | "XL" => {
                self.cmd_parts(host, rest);
                true
            }
            // 智能卡片（正式名 `OCSMCARD`；11 张卡，卡片类型表驱动 → 各卡属性插图）
            "OCSMCARD" => {
                self.cmd_card(host, rest);
                true
            }
            // 旧短命令 `XLT` 已移除（用户 2026-09-25 定名收敛）：明确报错并指路，不静默。
            "XLT" => {
                host.push_error(&crate::i18n::t("cmd.card.xlt.removed"));
                host.push_output(&crate::card::usage_line());
                true
            }
            // 明细表：建表/刷新（`BOM 30` = 本次首列 30 行）、配置（`BOMCFG 30`）
            _ if is_bom_command(cmd) => {
                // 只有**命令名**转大写；参数（路径、序号等）保留原始大小写
                // —— 否则 `BOMXLSXI ~/桌面/x.xlsx` 会变成 `~/桌面/X.XLSX` 找不到文件。
                let trimmed = cmd.trim();
                let name = trimmed
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_ascii_uppercase();
                let rest = trimmed[name.len().min(trimmed.len())..].trim().to_string();
                if name == "OCSMBOMCFG" || name == "BOMCFG" {
                    bom::cmd_bom_cfg(host, &rest);
                } else if name == "OCSMBOMEDIT" || name == "BOMEDIT" {
                    self.cmd_bom_edit(host);
                } else if name == "OCSMBOMSYNC" || name == "BOMSYNC" {
                    bom::cmd_bom_sync(host, &rest);
                } else if name == "OCSMBOMLOCK" || name == "BOMLOCK" {
                    bom::cmd_bom_lock(host, &rest);
                } else if name == "OCSMBOMXLSX" || name == "BOMXLSX" {
                    bom::cmd_bom_xlsx(host, &rest);
                } else if name == "OCSMBOMXLSXI" || name == "BOMXLSXI" {
                    bom::cmd_bom_xlsxi(host, &rest);
                } else {
                    bom::cmd_bom(host, &rest);
                }
                true
            }
            "OCSMDIM2GB" | "D2G" => {
                self.cmd_dim2gb(host);
                true
            }
            "OCSMMCP" => {
                self.cmd_mcp(host);
                true
            }
            _ => {
                // 数字键 1~10（单字符 + 两位数 10，避免与其它命令冲突；DYN
                // 输入时数字由宿主路由进距离字段，不会到这里）。
                let bytes = cmd.as_bytes();
                let digit = if bytes == b"10" {
                    10
                } else if bytes.len() == 1 && (b'1'..=b'9').contains(&bytes[0]) {
                    (bytes[0] - b'0') as usize
                } else {
                    0
                };
                if digit >= 1 {
                    self.cmd_layer_key(host, digit);
                    return true;
                }
                false
            }
        }
    }
}

/// 自动初始化要跳过的命令（命令名已大写）：文档生命周期 / 帮助 / 全局设置类。
///
/// 与宿主 `start_allowed`（`src/app/commands/mod.rs`）同口径：这些命令在
/// 欢迎（Start）标签页也允许执行，不能拿它们当作「当前有真实图纸」的信号去
/// 初始化；`OCSM` 本身跳过是因为它自己就会初始化（否则一条命令两个撤销点）。
fn auto_init_skipped_command(upper: &str) -> bool {
    upper == "OCSM"
        || upper.starts_with("OPEN_RECENT")
        || matches!(
            upper,
            "NEW" | "OPEN" | "EXIT" | "QUIT" | "REPORT" | "CHANGELOG" | "ABOUT"
                | "PLUGINS" | "PLUGINMANAGER" | "DONATE" | "WEBVERSION" | "HELP"
                | "OCSMHELP" | "OH" | "PERF" | "CUI" | "ALIASEDIT" | "CUILOAD"
                | "CUIIMPORT" | "OPTIONS" | "OP"
        )
}

/// 「新建空图纸」判据（安全第一，宿主没有「文档新建/打开」事件时的严格启发式）：
///
/// - 存过盘 / 打开 / 另存过的一律不动（`path.is_some()`，**空文件也算老图**）；
/// - 有任何实体一律不动；
/// - 有任何 OCSM 痕迹（10 层里任一层，或 `OCSM_GB` 文字/标注样式）视为老图不动。
///
/// 三条全满足才算「新建空图纸」——宁可漏初始化（用户可手打 `OCSM`），不可误伤旧图。
pub(crate) fn is_new_blank_drawing(
    doc: &acadrust::CadDocument,
    path: Option<&std::path::Path>,
) -> bool {
    if path.is_some() || doc.entity_count() > 0 {
        return false;
    }
    let ocsm_layer = layer_defs()
        .iter()
        .any(|def| doc.layers.iter().any(|ly| ly.name.eq_ignore_ascii_case(&def.name)));
    let ocsm_style = doc
        .text_styles
        .iter()
        .any(|s| s.name.eq_ignore_ascii_case("OCSM_GB"))
        || doc
            .dim_styles
            .iter()
            .any(|s| s.name.eq_ignore_ascii_case("OCSM_GB"));
    !ocsm_layer && !ocsm_style
}

/// 自动初始化来源提示：命令入口与通知入口共用（逐字一致）。
const AUTO_INIT_INFO: &str = "新建图纸：自动执行 OCSM 初始化（旧图纸不会自动初始化）。";

/// 初始化完成行：手动 `OCSM` 与两条自动路径共用（逐字一致）。
fn ocsm_init_done_line(layers: usize, linetypes: usize, styles: usize, dim_styles: usize) -> String {
    format!(
        "OCSM 初始化完成：新增图层 {layers} 个、线型 {linetypes} 个、文字样式 {styles} 个（OCSM_GB）、标注样式 {dim_styles} 个（OCSM_GB，当前样式）。数字键 1-10、TF、D 已就绪。"
    )
}

/// 旧图救字体（用户 2026-09-28）：`ensure_text_styles` 对同名样式只跳过、不重建，
/// 所以旧图里 `OCSM_GB` 的 `true_type_font` 是空的（旧版插件建的、或外来图档带来的）。
/// 这里在 `ensure` 前后各看一眼：**`ensure` 之前**就存在且 TTF 为空、且属于
/// [`text_style_defs`] 里的样式名 ⇒ 按插件自己的定义 `TextStyleModify` 补上。
///
/// 硬规则（与宿主侧同口径）：**只在为空时补**；已有非空 TTF（哪怕不是我们的字体）
/// 一律不碰；只处理我们定义的样式名。快照必须在 `ensure` 前取，否则新建出来的
/// 样式（TTF 由宿主填）会被误判成「旧图缺字」。
///
/// 新宿主已在内建 `ensure_text_styles` 里做同样的事，这条插件侧路径通常是 no-op ——
/// 留着是因为插件与宿主**分开部署**（旧宿主 + 新插件也不能丢字体）。
fn ensure_ocsm_text_styles(host: &mut dyn HostApi) -> usize {
    use ocs_plugin_api::host::{TableOperation, TextStyleConfig};
    // ① 先取快照：本来就在、TTF 为空的插件样式（旧图）。
    let stale: Vec<(String, String)> = host
        .document()
        .text_styles
        .iter()
        .filter(|s| s.true_type_font.trim().is_empty())
        .filter_map(|s| {
            text_style_defs()
                .into_iter()
                .find(|d| d.name.eq_ignore_ascii_case(&s.name) && !d.true_type_font.trim().is_empty())
                .map(|d| (s.name.clone(), d.true_type_font))
        })
        .collect();
    // ② 正常建缺失样式（新宿主会顺手把已存在但 TTF 为空的也补好）。
    let created = host.ensure_text_styles(text_style_defs());
    // ③ 宿主没补（旧宿主）→ 插件自己补，并告诉用户补了什么。
    let mut repaired: Vec<String> = Vec::new();
    for (name, ttf) in stale {
        let still_empty = host
            .document()
            .text_styles
            .get(&name)
            .map(|s| s.true_type_font.trim().is_empty())
            .unwrap_or(false);
        if !still_empty {
            // 宿主已在 `ensure_text_styles` 里补好（新宿主）——照实报给用户。
            repaired.push(format!("{name} → {ttf}"));
            continue;
        }
        match host.table_operation(TableOperation::TextStyleModify {
            config: TextStyleConfig {
                name: name.clone(),
                true_type_font: Some(ttf.clone()),
                ..Default::default()
            },
        }) {
            Ok(_) => repaired.push(format!("{name} → {ttf}")),
            Err(e) => host.push_error(&format!(
                "OCSM：样式 {name} 缺少 TrueType 字体，按插件定义补齐失败：{e}"
            )),
        }
    }
    if !repaired.is_empty() {
        host.push_info(&format!(
            "OCSM：旧图样式缺少 TrueType 字体，已按插件定义补齐（{}）。",
            repaired.join("、")
        ));
    }
    created
}

/// 通知路径（`maybe_auto_init_after_notification`）的进程级状态。
struct AutoInitState {
    /// `on_load` 缓存的 sender：工作线程用它发宿主请求（不必等命令）。
    sender: Option<std::sync::Arc<dyn PluginRequestSender>>,
    /// 插件启动时所在的标签页（欢迎/Start 页）：通知路径不初始化欢迎页。
    boot_tab: Option<u64>,
    /// 已检查过的标签页：同一张图的多次通知只起一次工作线程（幂等 + 防抖）。
    checked: std::collections::BTreeSet<u64>,
    /// 正在初始化的标签页：命令入口先让路，避免两条路径对同一张图各推一个撤销点。
    in_flight: std::collections::BTreeSet<u64>,
}

static AUTO_INIT_STATE: std::sync::Mutex<AutoInitState> = std::sync::Mutex::new(AutoInitState {
    sender: None,
    boot_tab: None,
    checked: std::collections::BTreeSet::new(),
    in_flight: std::collections::BTreeSet::new(),
});

fn auto_init_state_lock() -> std::sync::MutexGuard<'static, AutoInitState> {
    AUTO_INIT_STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// 工作线程退出时解除 `in_flight`（包括 panic 展开）。
struct InFlightGuard(u64);

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        auto_init_state_lock().in_flight.remove(&self.0);
    }
}

/// `BuiltinPlugin::on_load`（API v5）：缓存通知路径用的 sender + 启动页 tab id。
pub(crate) fn on_load_auto_init(host: &mut dyn HostApi) {
    let boot_tab = host.tab_id();
    let sender = host
        .plugin_request_sender()
        .map(std::sync::Arc::<dyn PluginRequestSender>::from);
    let mut st = auto_init_state_lock();
    st.sender = sender;
    st.boot_tab = Some(boot_tab);
}

/// 宿主通知（切图/选择变化/文档变化）后检查目标图是否新建空图 → 不等命令立即初始化。
///
/// 判据与命令入口 `auto_init_new_document` 完全一致（三门一条不放宽），区别只是
/// 由通知驱动。`DocumentChangedV4` 目前只在插件经 HostSession 改文档时广播，
/// `SelectionChangedV4` 在建图/切图时就会到，所以两条都接。
pub(crate) fn maybe_auto_init_after_notification(tab_id: u64) {
    let sender = {
        let mut st = auto_init_state_lock();
        if st.boot_tab == Some(tab_id) {
            return; // 欢迎页没有“新建图纸”语义
        }
        let Some(sender) = st.sender.clone() else {
            return; // 不在插件进程里（或 on_load 未跑）：等命令入口兜底
        };
        if !st.checked.insert(tab_id) {
            return; // 这张图已经查过/正在查
        }
        st.in_flight.insert(tab_id);
        sender
    };
    // 工作线程：`on_notification` 在 runner 主线程上，不能阻塞等宿主 drain。
    let spawned = std::thread::Builder::new()
        .name("ocsm-autoinit".into())
        .spawn(move || {
            let _guard = InFlightGuard(tab_id);
            auto_init_document_via_sender(&*sender, tab_id);
        });
    if spawned.is_err() {
        // 起线程失败 → 撤销防重标记，下一次通知还能再试。
        let mut st = auto_init_state_lock();
        st.checked.remove(&tab_id);
        st.in_flight.remove(&tab_id);
    }
}

/// 经 `sender` 对 `tab_id` 执行「新建空图 → OCSM 初始化」，返回是否初始化。
///
/// 判据与命令入口完全一致（路径 / 实体 / OCSM 痕迹三门），产物与手动 `OCSM`
/// 同序列同文案：PushInfo → PushUndo → 线型 → 图层 → 文字样式 → 标注样式 → 完成行。
pub(crate) fn auto_init_document_via_sender(
    sender: &dyn PluginRequestSender,
    tab_id: u64,
) -> bool {
    use ocs_plugin_api::ipc::protocol::{PluginRequest, PluginResponse};
    // 1) 路径门：存过盘 / 打开过的一律不动（空文件也算老图）。
    let path = match sender.request_for_tab(Some(tab_id), PluginRequest::DocumentPath { tab_id }) {
        Ok(PluginResponse::DocumentPath(path)) => path,
        _ => return false,
    };
    if path.is_some() {
        return false;
    }
    // 2) 文档快照门：有任何实体 / 任何 OCSM 痕迹 → 旧图不动。
    let doc = match sender.request_for_tab(Some(tab_id), PluginRequest::DocumentSnapshot) {
        Ok(PluginResponse::Document(doc)) => doc,
        _ => return false,
    };
    if !is_new_blank_drawing(&doc, None) {
        return false;
    }
    // 3) 表初始化：与 `init_ocsm_tables` 同序列同产物。
    let count = |resp: Result<PluginResponse, ocs_plugin_api::host::PluginRequestError>| match resp {
        Ok(PluginResponse::Count(n)) => Some(n),
        _ => None,
    };
    let _ = sender.request_for_tab(
        Some(tab_id),
        PluginRequest::PushInfo(AUTO_INIT_INFO.to_string()),
    );
    if sender
        .request_for_tab(
            Some(tab_id),
            PluginRequest::PushUndo {
                label: "OCSM 初始化".to_string(),
            },
        )
        .is_err()
    {
        return false;
    }
    let Some(linetypes) = count(sender.request_for_tab(
        Some(tab_id),
        PluginRequest::EnsureLinetypes(linetype_defs()),
    )) else {
        return false;
    };
    let Some(layers) = count(sender.request_for_tab(
        Some(tab_id),
        PluginRequest::EnsureLayers(layer_defs()),
    )) else {
        return false;
    };
    let Some(styles) = count(sender.request_for_tab(
        Some(tab_id),
        PluginRequest::EnsureTextStyles(text_style_defs()),
    )) else {
        return false;
    };
    let Some(dim_styles) = count(sender.request_for_tab(
        Some(tab_id),
        PluginRequest::EnsureDimStyles(dim_style_defs()),
    )) else {
        return false;
    };
    let _ = sender.request_for_tab(
        Some(tab_id),
        PluginRequest::PushOutput(ocsm_init_done_line(layers, linetypes, styles, dim_styles)),
    );
    true
}

impl OcsmPlugin {
    /// `OCSM`：初始化图层 + 线型 + 文字样式 + 标注样式（幂等）。
    fn cmd_init(&self, host: &mut dyn HostApi) {
        self.init_ocsm_tables(host);
    }

    /// 表初始化主体：手动 `OCSM` 与新建图纸自动初始化**共用同一条路径**。
    fn init_ocsm_tables(&self, host: &mut dyn HostApi) {
        // 表变更（线型/图层/文字样式/标注样式）宿主不会自动入撤销栈：命令必须
        // 自己声明撤销点，否则 AI/MCP 驱动初始化后 Ctrl+Z 与 MCP `op:"undo"` 都撤不掉。
        host.push_undo("OCSM 初始化");
        // 线型先于图层：DWG 写出时层引用的线型名必须在 line_types 表。
        let lt = host.ensure_linetypes(linetype_defs());
        let layers = host.ensure_layers(layer_defs());
        let styles = ensure_ocsm_text_styles(host);
        // 标注样式（参数取自标注示例.dwg 的 Mechanical，字体换成 OCSM_GB）。
        let dim_styles = host.ensure_dim_styles(dim_style_defs());
        host.push_output(&ocsm_init_done_line(layers, lt, styles, dim_styles));
    }

    /// 新建空图纸 → 在第一条命令分发前自动跑一遍 OCSM 初始化（旧图纸不动作）。
    ///
    /// 宿主没有「文档新建/打开」事件（见 `handbook/01`），只能在每条命令
    /// `dispatch` 时用 [`is_new_blank_drawing`] 判一次；初始化后图层已在，
    /// 下一条命令判据不再成立（幂等）。初始化走 [`Self::init_ocsm_tables`]，
    /// 与手动 `OCSM` 同一个撤销点、同一套产物。
    fn auto_init_new_document(&self, host: &mut dyn HostApi, upper: &str) {
        if auto_init_skipped_command(upper) {
            return;
        }
        let tab = host.tab_id();
        // 通知路径正在初始化这张图 → 让路（同一次初始化，不叠撤销点）。
        if auto_init_state_lock().in_flight.contains(&tab) {
            return;
        }
        // 先看路径：老图（含空的已存盘图）直接返回，不取文档快照。
        let path = host.document_path(tab);
        if path.is_some() {
            return;
        }
        if !is_new_blank_drawing(host.document(), path.as_deref()) {
            return;
        }
        host.push_info(AUTO_INIT_INFO);
        self.init_ocsm_tables(host);
    }

    /// `OCSMDIMGULIDE` / `GDIM`：选中引导线 → 启动标注更新服务器 →
    /// 浏览器打开标注配置 GUI。
    fn cmd_guide(&self, host: &mut dyn HostApi) {
        let selected = host.selected_handles();
        let Some(&h) = selected.first() else {
            host.push_error("OCSM: 请先选中一条引导线（10引导线层 的直线或两段多段线）。");
            return;
        };
        let doc = host.document();
        let Some(e) = doc.get_entity(h) else {
            host.push_error("OCSM: 找不到选中的实体。");
            return;
        };
        match &e {
            acadrust::EntityType::Line(_) => {}
            // 局部放大图：圆（CIRCLE）或矩形（闭合 4 顶点无 bulge PLINE = RECTANG 产物）。
            acadrust::EntityType::Circle(_) => {}
            // 弧长标注：ARC 引导。
            acadrust::EntityType::Arc(_) => {}
            acadrust::EntityType::LwPolyline(pl) if !pl.is_closed && pl.vertices.len() >= 2 => {}
            acadrust::EntityType::LwPolyline(pl)
                if pl.is_closed
                    && pl.vertices.len() == 4
                    && pl.vertices.iter().all(|v| v.bulge.abs() < 1e-9) => {}
            acadrust::EntityType::Polyline(pl) if !pl.flags.is_closed() && pl.vertices.len() >= 2 => {}
            acadrust::EntityType::Polyline2D(pl) if !pl.flags.is_closed() && pl.vertices.len() >= 2 => {}
            // 矩形引导也可能以 heavy polyline 形式存在（DXF 读取路径转换）。
            acadrust::EntityType::Polyline(pl)
                if pl.flags.is_closed() && pl.vertices.len() == 4 => {}
            acadrust::EntityType::Polyline2D(pl)
                if pl.flags.is_closed()
                    && pl.vertices.len() == 4
                    && pl.vertices.iter().all(|v| v.bulge.abs() < 1e-9) => {}
            _ => {
                host.push_error("OCSM: 引导线必须是直线（LINE）、多段线（PLINE，2 顶点以上）、圆（CIRCLE）、圆弧（ARC）或矩形（RECTANG）。");
                return;
            }
        }
        let Some(port) = self.ensure_guide_server(host) else {
            host.push_error("OCSM: 无法启动标注更新服务器（宿主不支持 worker 请求）。");
            return;
        };
        // 心跳键与页面一致（页面按同样的规则从 URL 算键）：`guide-<句柄>@<图纸号>`。
        let key = guide_ping_key(h, Some(host.tab_id()));
        // `tab=`：把本窗口钉在**这张图**上（关图 → 新建图后老窗口不会跑到新图上写）。
        let opened = open_plugin_page(
            port,
            &format!("/guide.html?handle={:#X}&tab={}", u64::from(h), host.tab_id()),
            &key,
            1020,
            900,
        );
        host.push_info(if opened {
            "OCSM 尺寸引导：已打开标注配置窗口。应用=写超链接；应用并刷新=生成标注（7标注层/OCSM_GB）。"
        } else {
            "OCSM 尺寸引导：标注配置窗口已打开（Alt+Tab 切换过去）。"
        });
    }

    /// `OCSMEDIT` / `ME`：选中一个 OCSM 生成的标注（带 `OCSM_EDIT` 记录）→
    /// 打开同一个标注配置 GUI 改它：GUI 从 `/api/guide` 读该标注的记录恢复
    /// 全部参数；「应用并刷新」时插件会删旧标注并用原引导几何重生成（替换）。
    /// 注意：命令里**不能**用 sender（会与主线程死锁）——直接读 host.document()。
    fn cmd_edit(&self, host: &mut dyn HostApi) {
        let selected = host.selected_handles();
        let Some(&h) = selected.first() else {
            host.push_error("OCSM: 请先选中一个 OCSM 生成的标注（引导生成的尺寸/焊接/引线/公差等），再执行 OCSMEDIT。");
            return;
        };
        let has_edit = {
            let doc = host.document();
            doc.get_entity(h).is_some_and(|e| {
                e.common()
                    .extended_data
                    .get_record("OCSM_EDIT")
                    .is_some()
            })
        };
        if !has_edit {
            host.push_error(
                "OCSM: 选中的对象不是 OCSM 生成的标注（没有可编辑信息）。若它是旧版生成的标注，请先重画一次。",
            );
            return;
        }
        let Some(port) = self.ensure_guide_server(host) else {
            host.push_error("OCSM: 无法启动标注更新服务器（宿主不支持 worker 请求）。");
            return;
        };
        // 心跳键与页面一致（页面按同样的规则从 URL 算键）：`guide-<句柄>@<图纸号>`。
        let key = guide_ping_key(h, Some(host.tab_id()));
        // `tab=`：把本窗口钉在**这张图**上（同 cmd_guide）。
        let opened = open_plugin_page(
            port,
            &format!("/guide.html?handle={:#X}&tab={}", u64::from(h), host.tab_id()),
            &key,
            1020,
            900,
        );
        host.push_info(if opened {
            "OCSM 标注编辑：已打开配置窗口（改完点「应用并刷新」= 替换旧标注）。"
        } else {
            "OCSM 标注编辑：配置窗口已打开（Alt+Tab 切换过去）。"
        });
    }

    /// `OCSMDIM2GB` / `D2G`：把宿主原生标注（模型空间 DIMENSION）**重建**为 OCSM 的
    /// GB 标准版本——OCSM_GB_x{图框比例} 样式 + `7标注层`（尺寸）/ 匿名块（直径、
    /// 半径、角度、弧长），保留原文字覆盖、小数位、样式前后缀与公差。
    /// 另外转**引线标注**（`LEADER` + 绑定 `MTEXT`）：文字 1~2 行→OCSM 引线标注
    /// （第1行→上侧、第2行→下侧；≥３ 行跳过）；多重引线本期不转。
    /// 有选中→只转选中集；无选中→全图扫描。不支持的类型跳过并在结束时报清单，
    /// 整批一次 Ctrl+Z 可全撤。
    fn cmd_dim2gb(&self, host: &mut dyn HostApi) {
        let selected = host.selected_handles();
        let plan = {
            let doc = host.document();
            crate::dim2gb::plan(doc, &selected)
        };
        if plan.seen == 0 {
            let tail = if plan.skipped.is_empty() {
                String::new()
            } else {
                crate::i18n::t_fmt(
                    "cmd.d2g.info.none_found_tail",
                    &[(
                        "list",
                        &plan.skipped.join(&crate::i18n::t("cmd.d2g.report.sep")),
                    )],
                )
            };
            host.push_info(&crate::i18n::t_fmt(
                "cmd.d2g.info.none_found",
                &[("tail", &tail)],
            ));
            return;
        }
        host.push_info(&plan.report());
        if plan.converted == 0 {
            return;
        }
        host.push_undo(&crate::i18n::t("cmd.d2g.undo"));
        // ① 幂等补基建（图层/文字样式/标注样式——图纸可能从没跑过 OCSM 初始化）
        // ② 建匿名块（INSERT 引用它）③ 加新实体 ④ 删原标注。
        host.ensure_layers(layer_defs());
        host.ensure_text_styles(text_style_defs());
        let mut ensured: Vec<String> = Vec::new();
        for def in &plan.styles {
            if ensured.iter().any(|n| n.eq_ignore_ascii_case(&def.name)) {
                continue;
            }
            ensured.push(def.name.clone());
            host.ensure_dim_styles(vec![def.clone()]);
        }
        for (name, ents) in &plan.blocks {
            if let Err(e) = host.add_block_record(name, ents.clone()) {
                host.push_error(&crate::i18n::t_fmt(
                    "cmd.d2g.err.block_create_named",
                    &[("name", name), ("e", &e.to_string())],
                ));
            }
        }
        if !plan.adds.is_empty() {
            let handles = host.add_entities(plan.adds.clone());
            // D2G 产出的标注带 `OCSM_EDIT`（参数 + 引导几何）→ 再补上回编辑 GUI 的
            // `PE_URL`。**必须等实体有了句柄**（URL 里要嵌 handle），所以放在这里。
            // 插件服务没起（端口未知）时跳过——`ME` 命令仍可编辑。
            if let Some(port) = crate::current_guide_port() {
                let targets: Vec<acadrust::Handle> = handles
                    .iter()
                    .copied()
                    .filter(|h| {
                        host.document().get_entity(*h).is_some_and(|e| {
                            e.common()
                                .extended_data
                                .get_record("OCSM_EDIT")
                                .is_some()
                        })
                    })
                    .collect();
                for h in targets {
                    let gui = format!(
                        "http://127.0.0.1:{port}/guide.html?handle={:#X}",
                        u64::from(h)
                    );
                    host.write_record(h, crate::guide_server::pe_url_record(&gui));
                }
            }
        }
        for h in &plan.removes {
            host.remove_entity(*h);
        }
        host.bump_geometry();
        host.set_dirty();
    }

    /// `OCSMRGH` / `CC`：表面粗糙度——交互点选插入点 → 打开粗糙度配置 GUI。
    /// 无引导线：点选位置即符号插入点（URL 带坐标），GUI 里选形态/填属性后
    /// POST /api/rough_apply 生成匿名块（ATTDEF 文字）+ INSERT。
    fn cmd_roughness(&self, host: &mut dyn HostApi) {
        let Some(sender) = host.plugin_request_sender() else {
            host.push_error(&crate::i18n::t("cmd.parts.err.no_service"));
            return;
        };
        host.start_interactive(Box::new(RoughnessPlace {
            sender: std::sync::Arc::from(sender),
            // 命令里知道自己在哪张图上 → 把窗口钉死在这张图（点选回调里拿不到 tab）。
            tab: Some(host.tab_id()),
        }));
    }

    /// `OCSMGEAR`：齿轮/渐开线花键出图（外齿轮 / 内齿轮 / 花键模式）。
    ///
    /// * 不带参数 = 人类侧：开齿轮/花键窗口（模式复选框 + 参数表 + 视图按钮 + 实时预览）+ 进放置态；
    ///   窗口里点「生成到图纸」→ 回图纸点基点 → 移动光标旋转 → 再点落定。
    /// * 带参数 = AI/MCP：齿轮 `OCSMGEAR [内齿轮|int] <m> <z> [h] [ha=..] [c=..] [beta=..] [x=..] [view 视图] [at x,y] [rot 度]`；
    ///   花键 `OCSMGEAR 花键 [内花键] [std=GB|DIN] [profile=GB30R] [db=40] [hf=..] [rho=..] [cf=..] <m> <z> [x=..] [h=..] [view 端视图|侧视图|剖视图]`（内花键与内齿轮同口径：只有 端视图|剖视图，无侧视图）。
    ///   内齿轮（齿圈）：`OCSMGEAR int 2 40 30 view 端视图`；只用 剖视图 + 端视图。GB 无 d_B；DIN 的 d_B 是主参数。
    /// * **计算书**：`OCSMGEAR … report [report=<path>]` —— 纯计算不插图，输出 Markdown
    ///   计算书（输入参数含单位口径 + 逐步计算：公式/代入/结果/依据来源 + 派生几何 +
    ///   检验尺寸含查表页/source + 数据来源与校验）；`report=<path>` 另写文件。
    fn cmd_gear(&self, host: &mut dyn HostApi, args: &str) {
        if !args.trim().is_empty() {
            // 计算书出口：`OCSMGEAR … report [report=<path>]` —— 纯计算，不插图。
            // 结果经 `host.push_output` 落到命令行/MCP 可读输出；给了路径则另写文件。
            let (args_wo, want_report, report_out) = crate::gear::split_report_args(args);
            if want_report {
                let req = match crate::gear::parse_request(&args_wo) {
                    Ok(r) => r,
                    Err(msg) => {
                        host.push_error(&msg);
                        return;
                    }
                };
                match crate::gear::build_report(&req.params) {
                    Ok(md) => {
                        if let Some(path) = &report_out {
                            match std::fs::write(path, &md) {
                                Ok(()) => host.push_info(&format!("OCSMGEAR：计算书已写入 {path}")),
                                Err(e) => host.push_error(&format!("OCSMGEAR 计算书写文件失败：{e}")),
                            }
                        }
                        host.push_output(&md);
                    }
                    Err(e) => host.push_error(&format!("OCSMGEAR 计算书：{e}")),
                }
                return;
            }
            let req = match crate::gear::parse_request(args) {
                Ok(r) => r,
                Err(msg) => {
                    host.push_error(&msg);
                    return;
                }
            };
            // 插之前先拦一道：没跑过 OCSM 初始化的图纸画齿轮会出“实线白中心线”（用户定案）
            if let Err(msg) = crate::gear::ocsm_ready(host.document()) {
                host.push_error(&format!("OCSMGEAR: {msg}"));
                host.push_info("OCSMGEAR：先运行 OCSM（或点功能区「图幅」组里的 OCSM 初始化），再生成齿轮。");
                return;
            }
            let Some(sender) = host.plugin_request_sender() else {
                host.push_error("OCSM 齿轮：宿主不支持 worker 请求，无法参数化插入。");
                return;
            };
            let sender: std::sync::Arc<dyn PluginRequestSender> = std::sync::Arc::from(sender);
            match crate::guide_server::apply_gear_insert(&sender, &req) {
                Ok(msg) => {
                    let v: serde_json::Value =
                        serde_json::from_str(&msg).unwrap_or(serde_json::Value::Null);
                    let text = v
                        .get("message")
                        .and_then(|m| m.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or(msg);
                    host.push_output(&format!("OCSM 齿轮：{text}"));
                    if let Some(notes) = v.get("notes").and_then(|n| n.as_array()) {
                        for nt in notes {
                            if let Some(s) = nt.as_str() {
                                host.push_info(&format!("OCSM 齿轮提示：{s}"));
                            }
                        }
                    }
                }
                Err(e) => host.push_error(&format!("OCSM 齿轮插入失败：{e}")),
            }
            return;
        }
        // 人类侧：先确保服务在跑并开窗，然后进放置态（窗口出图后鼠标即跟随预览）
        // —— 但没初始化就拦下来（同上：避免出“实线白中心线”的废图）
        if let Err(msg) = crate::gear::ocsm_ready(host.document()) {
            host.push_error(&format!("OCSMGEAR: {msg}"));
            host.push_info("OCSMGEAR：先运行 OCSM 初始化，再打开齿轮窗口。");
            return;
        }
        let Some(port) = self.ensure_guide_server(host) else {
            host.push_error("OCSMGEAR: 无法启动齿轮服务（宿主不支持 worker 请求）。");
            return;
        };
        if open_gear_window(port, Some(host.tab_id())) {
            host.push_info(
                "OCSM 齿轮/花键：已打开窗口（顶部可勾选「花键模式」；齿轮下可切外/内齿轮）。选视图 + 填参数点「生成到图纸」→ \
                 回到图纸点击定位基点 → 移动光标旋转 → 再点击落定（可连续，Esc 结束）。",
            );
        } else {
            host.push_info("OCSM 齿轮：齿轮窗口已打开（Alt+Tab 切换过去）。");
        }
        let Some(sender) = host.plugin_request_sender() else {
            return;
        };
        host.start_interactive(Box::new(PartPlace {
            sender: std::sync::Arc::from(sender),
            phase: std::cell::Cell::new(PlacePhase::Follow),
            base: std::cell::Cell::new([0.0, 0.0, 0.0]),
            what: "OCSM 齿轮".to_string(),
            where_to: "请在齿轮窗口里点「生成到图纸」".to_string(),
        }));
    }

    /// `OCSMSHAFT`：轴生成器——行 DSL / JSON → 单视图侧视图（视图开关 `VIEW 常规|剖视`；双视图已移除）。
    ///
    /// * 不带参数 = 人类侧：开轴生成器窗口（段表 + 行文本双向同步 + 视图按钮 + 实时预览）+ 进放置态；
    ///   窗口里点「生成到图纸」→ 回图纸点基点 → 移动光标旋转 → 再点落定。
    /// * 带参数 = AI/MCP：`OCSMSHAFT <行 DSL 或 JSON>` 一行直插（与原来一致）。
    /// * **计算书**：段末（或整体）加 `report`：`OCSMSHAFT SPLINE 6x23x26x6 L30 report`
    ///   —— 纯计算不插图，输出 Markdown 段清单（类型/关键参数/长度/外径）；`report=<path>` 另写文件。
    /// 轮廓/端面/倒角/槽与边界竖线 `1轮廓实线层`、螺纹小径/螺尾 `2细线层`
    /// （磨外圆/OV 不画砂轮细线）、轴线与分度线
    /// `3中心线层`、剖视剖面线 `5剖面线层`；不标尺寸。
    /// **插入前先拦未初始化图纸**（`shaft::ocsm_ready`，与 OCSMGEAR 同口径）。
    fn cmd_shaft(&self, host: &mut dyn HostApi, args: &str) {
        if args.trim().is_empty() {
            // 人类侧：先确保服务在跑并开窗，然后进放置态
            // —— 没初始化就拦下来（避免出“实线白中心线”的废图，与 OCSMGEAR 同口径）。
            if let Err(msg) = crate::shaft::ocsm_ready(host.document()) {
                host.push_error(&crate::i18n::t_fmt("cmd.shaft.err.host_prefix", &[("msg", &msg)]));
                host.push_info(&crate::i18n::t("cmd.shaft.info.run_init"));
                return;
            }
            let Some(port) = self.ensure_guide_server(host) else {
                host.push_error(&crate::i18n::t("cmd.shaft.err.no_service"));
                return;
            };
            if open_shaft_window(port, Some(host.tab_id())) {
                host.push_info(&crate::i18n::t("cmd.shaft.info.opened"));
            } else {
                host.push_info(&crate::i18n::t("cmd.shaft.info.already_open"));
            }
            let Some(sender) = host.plugin_request_sender() else {
                return;
            };
            host.start_interactive(Box::new(PartPlace {
                sender: std::sync::Arc::from(sender),
                phase: std::cell::Cell::new(PlacePhase::Follow),
                base: std::cell::Cell::new([0.0, 0.0, 0.0]),
                what: crate::i18n::t("cmd.shaft.place.what"),
                where_to: crate::i18n::t("cmd.shaft.place.where"),
            }));
            return;
        }
        // 计算书出口：`OCSMSHAFT … report [report=<path>]` —— 纯计算，不插图（不过 OCSM 初始化检查）。
        let (args_wo, want_report, report_out) = crate::gear::split_report_args(args);
        if want_report {
            let program = match crate::shaft::parse_program(&args_wo) {
                Ok(program) => program,
                Err(message) => {
                    host.push_error(&crate::i18n::t_fmt("cmd.shaft.err.params_invalid", &[("message", &message)]));
                    host.push_output(&crate::shaft::usage());
                    return;
                }
            };
            match crate::shaft::build_report(&program) {
                Ok(md) => {
                    if let Some(path) = &report_out {
                        match std::fs::write(path, &md) {
                            Ok(()) => host.push_info(&crate::i18n::t_fmt("cmd.shaft.info.report_written", &[("path", path)])),
                            Err(e) => host.push_error(&crate::i18n::t_fmt("cmd.shaft.err.report_write", &[("e", &e.to_string())])),
                        }
                    }
                    host.push_output(&md);
                }
                Err(e) => host.push_error(&crate::i18n::t_fmt("cmd.shaft.err.report", &[("e", &e.to_string())])),
            }
            return;
        }
        let program = match crate::shaft::parse_program(args) {
            Ok(program) => program,
            Err(message) => {
                host.push_error(&crate::i18n::t_fmt("cmd.shaft.err.params_invalid", &[("message", &message)]));
                host.push_output(&crate::shaft::usage());
                return;
            }
        };
        // 插之前先拦一道：与 OCSMGEAR 同口径 —— 没跑过 OCSM 初始化的图纸
        // 不再自动补层，直接报错指路（避免出“实线白中心线”的废图）。
        if let Err(msg) = crate::shaft::ocsm_ready(host.document()) {
            host.push_error(&crate::i18n::t_fmt("cmd.shaft.err.host_prefix", &[("msg", &msg)]));
            host.push_info(&crate::i18n::t("cmd.shaft.info.run_init_direct"));
            return;
        }
        let at = program.at.unwrap_or([0.0, 0.0]);
        let rot = program.rot.unwrap_or(0.0);
        // 图框比例在落点处查（与 D/GDIM/中心线同口径）；无图框 = 1.0。
        let scale = crate::frame_scale_at(host.document(), [at[0], at[1], 0.0]);
        let shaft = match crate::shaft::build(&program, scale) {
            Ok(shaft) => shaft,
            Err(message) => {
                host.push_error(&crate::i18n::t_fmt("cmd.shaft.err.geometry", &[("message", &message)]));
                return;
            }
        };
        let crate::shaft::Shaft {
            entities,
            total_length,
            max_diameter,
            segment_count,
        } = shaft;
        let entities = crate::shaft::place(entities, at, rot);
        host.push_undo(&crate::i18n::t("cmd.shaft.undo"));
        let count = entities.len();
        let _ = host.add_entities(entities);
        host.set_dirty();
        let layers = if program.view.has_hatch() {
            "1轮廓实线层 / 2细线层 / 3中心线层 / 5剖面线层"
        } else {
            "1轮廓实线层 / 2细线层 / 3中心线层"
        };
        host.push_output(&crate::i18n::t_fmt(
            "cmd.shaft.info.generated",
            &[
                ("segments", &segment_count.to_string()),
                ("length", &crate::partgen_kit::trim(total_length)),
                ("max", &crate::partgen_kit::trim(max_diameter)),
                (
                    "view",
                    &crate::i18n::t(if program.view.has_hatch() {
                        "cmd.shaft.view.section"
                    } else {
                        "cmd.shaft.view.normal"
                    }),
                ),
                ("count", &count.to_string()),
                ("layers", layers),
            ],
        ));
    }

    /// `OCSMHOLE` / `DK`：孔生成器（简单孔 / 螺纹孔 / 沉头孔 / 埋头孔）。
    ///
    /// * 不带参数 = 人类侧：开孔生成器窗口（孔类型 2×2 + 子类型/大小/配合 +
    ///   盲孔/贯通 + 孔深/螺纹范围自动 + 实时剖面预览）+ 进放置态；
    ///   窗口里点「确定」→ 回图纸点基点 → 移动光标旋转 → 再点落定。
    /// * 带参数 = AI/MCP：`OCSMHOLE [类型] [带螺纹|无螺纹] [钻孔|自定义|间隙] M10 ...`
    ///   一行直插（`hole::parse_program`）；「大小」也可直接写 GUI 下拉的规格名
    ///   （`G1/8`/`1/4-20`/`NPT1/2`/`Tr8×1.5`/`ACME 1/4-16`…，查表同源；数值 `公称 d P` 照旧）。
    /// * **自动规则**：螺纹长 = 1.5d；孔深 = 有效螺纹深 + 2P（仅带螺纹且盲孔时可选自动）；
    ///   贯通无 118° 底锥；不带螺纹时两个自动都不可选。
    /// * 数据：ISO 724 / GB/T 152.3 / GB/T 152.2 / GB/T 5277 / 底孔牙深明细表。
    fn cmd_hole(&self, host: &mut dyn HostApi, args: &str) {
        // 带参数 = 一行直插；解析失败报用法（不静默开窗）。
        if !args.trim().is_empty() {
            let model = match crate::hole::parse_program(args) {
                Ok(m) => m,
                Err(message) => {
                    host.push_error(&crate::i18n::t_fmt(
                        "cmd.hole.err.parse_wrap",
                        &[("message", &message)],
                    ));
                    host.push_output(&crate::hole::usage());
                    return;
                }
            };
            if let Err(msg) = crate::hole::ocsm_ready(host.document()) {
                host.push_error(&format!("OCSMHOLE: {msg}"));
                host.push_info(&crate::i18n::t("cmd.hole.info.run_init_direct"));
                return;
            }
            let at = model.at.unwrap_or([0.0, 0.0]);
            let rot = model.rot;
            let built = match crate::hole::build(&model) {
                Ok(b) => b,
                Err(message) => {
                    host.push_error(&crate::i18n::t_fmt(
                        "cmd.hole.err.geometry_wrap",
                        &[("message", &message)],
                    ));
                    return;
                }
            };
            let count = built.entities.len();
            let v = built.values.clone();
            let entities = crate::hole::place(built.entities, at, rot);
            host.push_undo("OCSMHOLE 孔");
            let _ = host.add_entities(entities);
            host.set_dirty();
            let kind = match model.kind {
                crate::hole::HoleKind::Simple => crate::i18n::t("cmd.hole.kind.simple"),
                crate::hole::HoleKind::Threaded => crate::i18n::t("cmd.hole.kind.threaded"),
                crate::hole::HoleKind::Counterbore => crate::i18n::t("cmd.hole.kind.counterbore"),
                crate::hole::HoleKind::Countersink => crate::i18n::t("cmd.hole.kind.countersink"),
            };
            let size = if v.threaded {
                v.size_name.clone()
            } else {
                format!("Ø{}", crate::hole::fmt3(v.base_d))
            };
            host.push_output(&crate::i18n::t_fmt(
                "cmd.hole.echo.created",
                &[
                    ("kind", &kind),
                    ("size", &size),
                    ("drill", &crate::hole::fmt3(v.base_d)),
                    ("depth", &crate::hole::fmt3(v.hole_depth)),
                    ("count", &count.to_string()),
                    ("x", &crate::partgen_kit::trim(at[0])),
                    ("y", &crate::partgen_kit::trim(at[1])),
                    ("rot", &crate::partgen_kit::trim(rot)),
                ],
            ));
            for w in &v.warnings {
                host.push_info(&crate::i18n::t_fmt(
                    "cmd.hole.info.warn_prefix",
                    &[("w", w)],
                ));
            }
            return;
        }
        // 人类侧：先拦截未初始化图纸（与轴/齿轮同口径），再开窗 + 进放置态。
        if let Err(msg) = crate::hole::ocsm_ready(host.document()) {
            host.push_error(&format!("OCSMHOLE: {msg}"));
            host.push_info(&crate::i18n::t("cmd.hole.info.run_init_window"));
            return;
        }
        let Some(port) = self.ensure_guide_server(host) else {
            host.push_error(&crate::i18n::t("cmd.hole.err.server_start"));
            return;
        };
        if open_hole_window(port, Some(host.tab_id())) {
            host.push_info(&crate::i18n::t("cmd.hole.info.window_opened"));
        } else {
            host.push_info(&crate::i18n::t("cmd.hole.info.window_focus"));
        }
        let Some(sender) = host.plugin_request_sender() else {
            return;
        };
        host.start_interactive(Box::new(PartPlace {
            sender: std::sync::Arc::from(sender),
            phase: std::cell::Cell::new(PlacePhase::Follow),
            base: std::cell::Cell::new([0.0, 0.0, 0.0]),
            what: crate::i18n::t("cmd.hole.place_what"),
            where_to: crate::i18n::t("cmd.hole.hint.click_ok"),
        }));
    }

    /// `OCSMPART` / `XL`：直接打开标准件库窗口，并进入放置态
    /// （窗口里「零件出库」→ 回到图纸鼠标跟随预览 → 左键点击放置，可连续，Esc 结束）。
    fn cmd_parts(&self, host: &mut dyn HostApi, args: &str) {
        // 带参数 = 参数化直接插入（给 MCP/AI 一行驱动）：
        //   `OCSMPART <族> <d> <l> [view <视图>] [at x,y] [rot 度]`
        // 例：`OCSMPART hex_bolt_c 10 95 at 150,30 rot 0`
        // 无参数 = 原 GUI 流程（开零件库窗口 + 进放置态）。
        // 有参数但解析失败 = 报用法（不静默开浏览器窗口，避免 AI 驱动时弹出无意义窗口）。
        if !args.trim().is_empty() {
            match PartsSpec::parse(args) {
                Some(spec) => {
                    self.cmd_parts_insert(host, &spec);
                    return;
                }
                None => {
                    host.push_error(&crate::i18n::t("cmd.parts.usage"));
                    return;
                }
            }
        }
        // 先确保标注更新服务器在跑并打开零件库窗口（不需要先点插入点）
        if let Some(port) = self.ensure_guide_server(host) {
            if open_parts_window(port, Some(host.tab_id())) {
                host.push_info(&crate::i18n::t("cmd.parts.window_opened"));
            } else {
                host.push_info(&crate::i18n::t("cmd.parts.window_exists"));
            }
        } else {
            host.push_error(&crate::i18n::t("cmd.parts.err.no_service"));
            return;
        }
        // 同时进入放置态：窗口里出库后，鼠标即跟随预览、左键落件
        let _ = self.ensure_guide_server(host);
        let Some(sender) = host.plugin_request_sender() else {
            return;
        };
        host.start_interactive(Box::new(PartPlace {
            sender: std::sync::Arc::from(sender),
            phase: std::cell::Cell::new(PlacePhase::Follow),
            base: std::cell::Cell::new([0.0, 0.0, 0.0]),
            what: crate::i18n::t("cmd.parts.prompt.what"),
            where_to: crate::i18n::t("cmd.parts.prompt.where"),
        }));
    }

    /// `OCSMPART`/`XL` 参数化插入：直接调零件库同一条生成/插入路径
    /// （`guide_server::apply_part_pick`，已带 Begin/Commit 撤销事务）。
    fn cmd_parts_insert(&self, host: &mut dyn HostApi, spec: &PartsSpec) {
        let Some(sender) = host.plugin_request_sender() else {
            host.push_error(&crate::i18n::t("cmd.parts.err.no_worker"));
            return;
        };
        let sender: std::sync::Arc<dyn PluginRequestSender> = std::sync::Arc::from(sender);
        match crate::guide_server::apply_part_pick(&sender, spec.to_body().as_bytes()) {
            Ok(msg) => host.push_output(&crate::i18n::t_fmt(
                "cmd.parts.inserted_prefix",
                &[("msg", &msg)],
            )),
            Err(e) => host.push_error(&crate::i18n::t_fmt(
                "cmd.parts.err.insert_failed",
                &[("e", &e)],
            )),
        }
    }

    /// `OCSMCARD`：**智能卡片**（通用卡片生成器；11 张卡：GB 花键内/外、齿轮、
    /// ANSI 内/外×中/英、NF 内/外、DIN 内/外；一卡一方向）。
    ///
    /// * 不带参数 = 人类侧：开智能卡片窗口 + 进放置态（与 DK/OCSMHOLE 同款）。
    /// * 带参数 = `OCSMCARD <卡类型> …`；卡类型/方向从 `card::CARD_TYPES` 表里查
    ///   （旧写法 `OCSMCARD 花键参数表 [std GB] 内 6H <九字段表达式> …` 仍兼容）。
    ///
    /// 旧短命令 `XLT` 已移除（dispatch 里明确报错并指路 `OCSMCARD`）。
    fn cmd_card(&self, host: &mut dyn HostApi, args: &str) {
        if args.trim().is_empty() {
            self.open_card_window_and_place(host);
            return;
        }
        let (name, rest) = match args.trim().split_once(char::is_whitespace) {
            Some((name, rest)) => (name, rest.trim()),
            None => (args.trim(), ""),
        };
        let Some(card) = crate::card::card_type_by_token(name) else {
            host.push_error(&crate::i18n::t_fmt(
                "cmd.card.err.unknown_type_usage",
                &[("name", name), ("usage", &crate::card::usage_line())],
            ));
            return;
        };
        match card.renderer {
            crate::card::CardRenderer::SplineTable => {
                // 一卡一方向；旧 CLI 显式内/外仍可覆盖（兼容）。
                let side = match card.direction {
                    Some("ext") => "外",
                    _ => "内",
                };
                let args = crate::inject_default_side(rest, side);
                self.cmd_spline_card(host, &args)
            }
            crate::card::CardRenderer::SplineLite => {
                // 一卡一方向；旧 CLI 显式内/外仍可覆盖（兼容）。
                let side = match card.direction {
                    Some("ext") => "外",
                    _ => "内",
                };
                let args = crate::inject_default_side(rest, side);
                self.cmd_spline_lite_card(host, &args)
            }
            crate::card::CardRenderer::GearTable => self.cmd_gear_card(host, rest),
            crate::card::CardRenderer::AnsiTableCn => self.cmd_ansi_card(
                host,
                &crate::inject_default_side(rest, card.direction.unwrap_or("int")),
                crate::ansi_table::AnsiLang::Cn,
            ),
            crate::card::CardRenderer::AnsiTableEn => self.cmd_ansi_card(
                host,
                &crate::inject_default_side(rest, card.direction.unwrap_or("int")),
                crate::ansi_table::AnsiLang::En,
            ),
            crate::card::CardRenderer::NfTable => self.cmd_nf_card(host, rest),
            crate::card::CardRenderer::NfExtTable => self.cmd_nf_ext_card(host, rest),
            crate::card::CardRenderer::DinTable => {
                self.cmd_din_card(host, rest, card.direction.unwrap_or("int") != "ext")
            }
            crate::card::CardRenderer::CardLite => self.cmd_card_lite(host, card, rest),
        }
    }

    /// 开智能卡片窗口 + 进放置态（`OCSMCARD` 无参调用；与 DK/OCSMHOLE 同款）。
    fn open_card_window_and_place(&self, host: &mut dyn HostApi) {
        let Some(port) = self.ensure_guide_server(host) else {
            host.push_error(&crate::i18n::t("cmd.card.err.no_worker"));
            return;
        };
        if open_card_window(port, Some(host.tab_id())) {
            host.push_info(&crate::i18n::t("cmd.card.info.opened"));
        } else {
            host.push_info(&crate::i18n::t("cmd.card.info.already_open"));
        }
        let Some(sender) = host.plugin_request_sender() else {
            return;
        };
        host.start_interactive(Box::new(PartPlace {
            sender: std::sync::Arc::from(sender),
            phase: std::cell::Cell::new(PlacePhase::Follow),
            base: std::cell::Cell::new([0.0, 0.0, 0.0]),
            what: crate::i18n::t("cmd.card.place.what"),
            where_to: crate::i18n::t("cmd.card.place.where"),
        }));
    }

    /// 卡类型「花键参数表」：
    /// `[std GB] 内 6H <九字段表达式> [dp 4.5] [root 平|圆] [at x,y] [rot 度]`。
    ///
    /// 参数来源 = 表达式（`spline_table::parse_expr_gear` 复用 `shaft::parse_program`
    /// 反解 m/z/αD/x/Da/Df）；表格块几何**内建**（每个方向一块，`OCSM_SPTABLE_GB_*`），
    /// 21 个值走 `spline_tol::compute()` 后写 INSERT.attributes（不依赖外部 DXF）；
    /// 建 INSERT 与 GUI 导出共用 `spline_table::build_insert()`。
    fn cmd_spline_card(&self, host: &mut dyn HostApi, args: &str) {
        use crate::spline_table::SplineTableSpec;
        let spec = match SplineTableSpec::parse(args) {
            Ok(s) => s,
            Err(e) => {
                host.push_error(&e);
                return;
            }
        };
        let input = match spec.to_input() {
            Ok(i) => i,
            Err(e) => {
                host.push_error(&crate::i18n::t_fmt("cmd.card.err.spline_prefix", &[("e", &e)]));
                return;
            }
        };
        let table = match crate::spline_tol::compute(&input) {
            Ok(t) => t,
            Err(e) => {
                host.push_error(&crate::i18n::t_fmt("cmd.card.err.spline_prefix", &[("e", &e)]));
                return;
            }
        };
        let side = spec.side;
        // 基建：图层/文字样式（块成员用 `6文字层` + OCSM_GB）。
        host.ensure_layers(layer_defs());
        host.ensure_text_styles(text_style_defs());
        // 建块：几何与取值无关 → 每个方向只建一次。
        let block = crate::spline_table::block_name(side);
        if host.document().block_records.get(block).is_none() {
            let members = crate::spline_table::block_entities(side);
            if let Err(e) = host.add_block_record(block, members) {
                host.push_error(&crate::i18n::t_fmt("cmd.card.err.spline_block", &[("block", block), ("e", &e)]));
                return;
            }
        }
        let at = spec.at.unwrap_or_else(|| {
            crate::take_parts_point()
                .map(|p| [p[0], p[1]])
                .unwrap_or([0.0, 0.0])
        });
        // CLI 与 GUI 导出共用这一条建 INSERT 路径（21 个 ATTRIB 在函数里填好）。
        let ins = match crate::spline_table::build_insert(side, &table, at, spec.rot) {
            Ok(i) => i,
            Err(e) => {
                host.push_error(&crate::i18n::t_fmt("cmd.card.err.spline_prefix", &[("e", &e)]));
                return;
            }
        };
        host.push_undo(&crate::i18n::t("cmd.card.undo.spline"));
        let handles = host.add_entities(vec![acadrust::EntityType::Insert(ins)]);
        if handles.is_empty() {
            host.push_error(&crate::i18n::t("cmd.card.err.spline_no_handle"));
            return;
        }
        host.set_dirty();
        let kind = crate::i18n::t(if side == crate::spline_tol::SplineSide::Internal {
            "cmd.card.side.internal"
        } else {
            "cmd.card.side.external"
        });
        let dp_note = match (&table, spec.dp) {
            (crate::spline_tol::SplineTable::Internal(t), None) => crate::i18n::t_fmt(
                "cmd.card.dp_note.calc",
                &[
                    ("dp", &crate::partgen_kit::trim(t.dp)),
                    ("dp_calc", &format!("{:.4}", t.dp_calc)),
                    ("c1", &crate::partgen_kit::trim(t.dp_candidates[0])),
                    ("c2", &crate::partgen_kit::trim(t.dp_candidates[1])),
                    ("c3", &crate::partgen_kit::trim(t.dp_candidates[2])),
                ],
            ),
            (crate::spline_tol::SplineTable::Internal(_t), Some(dp)) => crate::i18n::t_fmt(
                "cmd.card.dp_note.manual",
                &[("dp", &crate::partgen_kit::trim(dp))],
            ),
            _ => String::new(),
        };
        host.push_info(&crate::i18n::t_fmt(
            "cmd.card.info.spline_inserted",
            &[
                ("kind", &kind),
                ("grade", &input.grade_fit_label()),
                ("system", spec.system),
                ("expr", &spec.expr),
                ("x", &format!("{:.3}", at[0])),
                ("y", &format!("{:.3}", at[1])),
                ("rot", &crate::partgen_kit::trim(spec.rot)),
                ("dp_note", &dp_note),
            ],
        ));
    }

    /// 卡类型「GB花键精简表_内/外」（用户 2026-09-27 点单）：
    /// `[std GB] 内 6H <九字段表达式> [dp 4.5] [root 平|圆] [at x,y] [rot 度]`。
    ///
    /// 取值与 GB 花键卡**同一份** `spline_tol::compute()`；卡面只列 9 项基本参数/测量量
    /// （内：Dp/Md；外：Kn/Wn），**不含任何公差列**。表达式反解复用 `SplineTableSpec`。
    fn cmd_spline_lite_card(&self, host: &mut dyn HostApi, args: &str) {
        use crate::spline_table::SplineTableSpec;
        let spec = match SplineTableSpec::parse(args) {
            Ok(s) => s,
            Err(e) => {
                host.push_error(&e);
                return;
            }
        };
        let input = match spec.to_input() {
            Ok(i) => i,
            Err(e) => {
                host.push_error(&crate::i18n::t_fmt("cmd.card.err.lite_prefix", &[("e", &e)]));
                return;
            }
        };
        let table = match crate::spline_tol::compute(&input) {
            Ok(t) => t,
            Err(e) => {
                host.push_error(&crate::i18n::t_fmt("cmd.card.err.lite_prefix", &[("e", &e)]));
                return;
            }
        };
        let side = spec.side;
        host.ensure_layers(layer_defs());
        host.ensure_text_styles(text_style_defs());
        let block = crate::spline_lite::block_name(side);
        if host.document().block_records.get(block).is_none() {
            let members = crate::spline_lite::block_entities(side);
            if let Err(e) = host.add_block_record(block, members) {
                host.push_error(&crate::i18n::t_fmt("cmd.card.err.lite_block", &[("block", block), ("e", &e)]));
                return;
            }
        }
        let at = spec.at.unwrap_or_else(|| {
            crate::take_parts_point()
                .map(|p| [p[0], p[1]])
                .unwrap_or([0.0, 0.0])
        });
        let ins = match crate::spline_lite::build_insert(side, &table, at, spec.rot) {
            Ok(i) => i,
            Err(e) => {
                host.push_error(&crate::i18n::t_fmt("cmd.card.err.lite_prefix", &[("e", &e)]));
                return;
            }
        };
        host.push_undo(&crate::i18n::t("cmd.card.undo.lite"));
        let handles = host.add_entities(vec![acadrust::EntityType::Insert(ins)]);
        if handles.is_empty() {
            host.push_error(&crate::i18n::t("cmd.card.err.lite_no_handle"));
            return;
        }
        host.set_dirty();
        host.push_info(&crate::i18n::t_fmt(
            "cmd.card.info.lite_inserted",
            &[
                (
                    "kind",
                    &crate::i18n::t(if side == crate::spline_tol::SplineSide::Internal {
                        "cmd.card.side.internal"
                    } else {
                        "cmd.card.side.external"
                    }),
                ),
                ("grade", &input.grade_fit_label()),
                ("expr", &spec.expr),
                ("x", &format!("{:.3}", at[0])),
                ("y", &format!("{:.3}", at[1])),
                ("rot", &crate::partgen_kit::trim(spec.rot)),
            ],
        ));
    }

    /// 卡类型「精简版」系列（NF / DIN / ANSI×4 / 齿轮）：共享 `card_lite` 引擎。
    /// args 复用对应完整卡的 `Spec::parse`（查表/公式/表达式反解同一份）→ 投影精简项。
    fn cmd_card_lite(
        &self,
        host: &mut dyn HostApi,
        card: &crate::card::CardTypeSpec,
        args: &str,
    ) {
        let Some(c) = crate::card_lite::by_id(card.id) else {
            host.push_error(&crate::i18n::t_fmt("cmd.card.err.no_lite_def", &[("id", card.id)]));
            return;
        };
        let (full, extras, at, rot) = match crate::card_lite::cli_values(c, args) {
            Ok(v) => v,
            Err(e) => {
                host.push_error(&crate::i18n::t_fmt("cmd.card.err.lite_card_prefix", &[("id", c.id), ("e", &e)]));
                return;
            }
        };
        let values = match crate::card_lite::values_from_full(c, &full, &extras) {
            Ok(v) => v,
            Err(e) => {
                host.push_error(&e);
                return;
            }
        };
        host.ensure_layers(layer_defs());
        host.ensure_text_styles(text_style_defs());
        if host.document().block_records.get(c.block).is_none() {
            if let Err(e) = host.add_block_record(c.block, crate::card_lite::block_entities(c)) {
                host.push_error(&crate::i18n::t_fmt("cmd.card.err.lite_card_block", &[("id", c.id), ("block", c.block), ("e", &e)]));
                return;
            }
        }
        let at = at.unwrap_or_else(|| {
            crate::take_parts_point()
                .map(|p| [p[0], p[1]])
                .unwrap_or([0.0, 0.0])
        });
        let ins = match crate::card_lite::build_insert(c, &values, at, rot) {
            Ok(i) => i,
            Err(e) => {
                host.push_error(&crate::i18n::t_fmt("cmd.card.err.lite_card_prefix", &[("id", c.id), ("e", &e)]));
                return;
            }
        };
        host.push_undo(&crate::i18n::t_fmt("cmd.card.undo.lite_card", &[("label", &card.label())]));
        let handles = host.add_entities(vec![acadrust::EntityType::Insert(ins)]);
        if handles.is_empty() {
            host.push_error(&crate::i18n::t_fmt("cmd.card.err.lite_card_no_handle", &[("id", c.id)]));
            return;
        }
        host.set_dirty();
        host.push_info(&crate::i18n::t_fmt(
            "cmd.card.info.lite_card_inserted",
            &[
                ("label", &card.label()),
                ("n", &values.len().to_string()),
                ("x", &format!("{:.3}", at[0])),
                ("y", &format!("{:.3}", at[1])),
                ("rot", &crate::partgen_kit::trim(rot)),
            ],
        ));
    }

    /// 卡类型「齿轮参数表」：
    /// `OCSMCARD 齿轮参数表 <九字段表达式> [mate z₂] [dwg 图号] [grade 精度等级] [center a] [at x,y] [rot 度]`。
    /// 表达式反解交 `gear::GearParams`（同一套公式）；表格块几何内建（`OCSM_GEARTABLE_GB`），
    /// 19 个值写 INSERT.attributes（GB/T 10095 公差未收 → 如实标缺）。
    fn cmd_gear_card(&self, host: &mut dyn HostApi, args: &str) {
        use crate::gear_table::GearTableSpec;
        let spec = match GearTableSpec::parse(args) {
            Ok(s) => s,
            Err(e) => {
                host.push_error(&e);
                return;
            }
        };
        host.ensure_layers(layer_defs());
        host.ensure_text_styles(text_style_defs());
        let block = crate::gear_table::BLOCK;
        if host.document().block_records.get(block).is_none() {
            let members = crate::gear_table::block_entities();
            if let Err(e) = host.add_block_record(block, members) {
                host.push_error(&crate::i18n::t_fmt("cmd.card.gear.err_block", &[("block", block), ("e", &e)]));
                return;
            }
        }
        let at = spec.at.unwrap_or_else(|| {
            crate::take_parts_point()
                .map(|p| [p[0], p[1]])
                .unwrap_or([0.0, 0.0])
        });
        let ins = match crate::gear_table::build_insert(&spec, at, spec.rot) {
            Ok(i) => i,
            Err(e) => {
                host.push_error(&crate::i18n::t_fmt("cmd.card.gear.err_prefix", &[("e", &e)]));
                return;
            }
        };
        host.push_undo(&crate::i18n::t("cmd.card.undo.gear"));
        let handles = host.add_entities(vec![acadrust::EntityType::Insert(ins)]);
        if handles.is_empty() {
            host.push_error(&crate::i18n::t("cmd.card.gear.err_no_handle"));
            return;
        }
        host.set_dirty();
        let note = crate::gear_table::GearTableModel {
            card: "齿轮参数表".into(),
            expr: spec.expr.clone(),
            mate_z: spec.mate_z,
            mate_dwg: spec.mate_dwg.clone(),
            grade: spec.grade.clone(),
            center: spec.center,
            at: spec.at,
            rot: spec.rot,
        }
        .echo_note()
        .unwrap_or_else(|_| String::new());
        host.push_info(&crate::i18n::t_fmt(
            "cmd.card.gear.info",
            &[
                ("note", &note),
                ("x", &format!("{:.3}", at[0])),
                ("y", &format!("{:.3}", at[1])),
                ("rot", &crate::partgen_kit::trim(spec.rot)),
            ],
        ));
    }

    /// 卡类型「ANSI 花键参数表」（纯中文 / 纯英文）：
    /// `OCSMCARD ANSI花键参数表_中文 内 P16 Z20 [profile ANSI30R] [at x,y] [rot 度]`。
    /// 引擎 = `invol_spline.rs` ANSI B92.1 Table 2；表格块几何内建（4 个块：内/外 × 中/英）。
    fn cmd_ansi_card(&self, host: &mut dyn HostApi, args: &str, lang: crate::ansi_table::AnsiLang) {
        use crate::ansi_table::AnsiTableSpec;
        let spec = match AnsiTableSpec::parse(lang, args) {
            Ok(s) => s,
            Err(e) => {
                host.push_error(&e);
                return;
            }
        };
        host.ensure_layers(layer_defs());
        host.ensure_text_styles(text_style_defs());
        let block = crate::ansi_table::block_name(spec.side, spec.lang);
        if host.document().block_records.get(block).is_none() {
            let members = crate::ansi_table::block_entities(spec.side, spec.lang);
            if let Err(e) = host.add_block_record(block, members) {
                host.push_error(&crate::i18n::t_fmt("cmd.card.ansi.err_block", &[("block", block), ("e", &e)]));
                return;
            }
        }
        let at = spec.at.unwrap_or_else(|| {
            crate::take_parts_point()
                .map(|p| [p[0], p[1]])
                .unwrap_or([0.0, 0.0])
        });
        let ins = match crate::ansi_table::build_insert(&spec, at, spec.rot) {
            Ok(i) => i,
            Err(e) => {
                host.push_error(&crate::i18n::t_fmt("cmd.card.ansi.err_prefix", &[("e", &e)]));
                return;
            }
        };
        host.push_undo(&crate::i18n::t("cmd.card.ansi.undo"));
        let handles = host.add_entities(vec![acadrust::EntityType::Insert(ins)]);
        if handles.is_empty() {
            host.push_error(&crate::i18n::t("cmd.card.ansi.err_no_handle"));
            return;
        }
        host.set_dirty();
        let side = crate::i18n::t(match spec.side {
            crate::spline_tol::SplineSide::Internal => "cmd.card.side.internal",
            crate::spline_tol::SplineSide::External => "cmd.card.side.external",
        });
        host.push_info(&crate::i18n::t_fmt(
            "cmd.card.ansi.info",
            &[
                ("side", &side),
                ("lang", &spec.lang.label()),
                ("pair", &crate::ansi_table::pair_label(spec.p)),
                ("n", &spec.z.to_string()),
                ("x", &format!("{:.3}", at[0])),
                ("y", &format!("{:.3}", at[1])),
                ("rot", &crate::partgen_kit::trim(spec.rot)),
            ],
        ));
    }

    /// 卡类型「NF 内花键参数表」：
    /// `OCSMCARD NF内花键参数表 A300 M7.5 [Z38] [中心 外径|齿面] [根 平|圆] [at x,y] [rot 度]`。
    /// 表格块几何内建（`OCSM_NFTABLE_NF_INT`，照 `外花键参数表NF.dxf` 同构镜像）；
    /// 18 个值写 INSERT.attributes（偏差列义未辨 → 6 个公差格显示「—」，不臆造）。
    fn cmd_nf_card(&self, host: &mut dyn HostApi, args: &str) {
        use crate::nf_table::NfTableSpec;
        let spec = match NfTableSpec::parse(args) {
            Ok(s) => s,
            Err(e) => {
                host.push_error(&e);
                return;
            }
        };
        host.ensure_layers(layer_defs());
        host.ensure_text_styles(text_style_defs());
        let block = crate::nf_table::BLOCK;
        if host.document().block_records.get(block).is_none() {
            let members = crate::nf_table::block_entities();
            if let Err(e) = host.add_block_record(block, members) {
                host.push_error(&crate::i18n::t_fmt("cmd.card.nf.err_block", &[("block", block), ("e", &e)]));
                return;
            }
        }
        let at = spec.at.unwrap_or_else(|| {
            crate::take_parts_point()
                .map(|p| [p[0], p[1]])
                .unwrap_or([0.0, 0.0])
        });
        let ins = match crate::nf_table::build_insert(&spec, at, spec.rot) {
            Ok(i) => i,
            Err(e) => {
                host.push_error(&crate::i18n::t_fmt("cmd.card.nf.err_prefix", &[("e", &e)]));
                return;
            }
        };
        host.push_undo(&crate::i18n::t("cmd.card.nf.undo"));
        let handles = host.add_entities(vec![acadrust::EntityType::Insert(ins)]);
        if handles.is_empty() {
            host.push_error(&crate::i18n::t("cmd.card.nf.err_no_handle"));
            return;
        }
        host.set_dirty();
        let note = crate::nf_table::NfTableModel {
            card: "NF内花键参数表".into(),
            expr: None,
            a: spec.a,
            m: spec.m,
            z: spec.z,
            centering: Some(spec.centering.id().into()),
            root: Some(spec.root.id().into()),
            fit: Some(spec.fit.id().into()),
            at: spec.at,
            rot: spec.rot,
        }
        .echo_note()
        .unwrap_or_else(|_| String::new());
        host.push_info(&crate::i18n::t_fmt(
            "cmd.card.nf.info",
            &[
                ("note", &note),
                ("x", &format!("{:.3}", at[0])),
                ("y", &format!("{:.3}", at[1])),
                ("rot", &crate::partgen_kit::trim(spec.rot)),
            ],
        ));
    }

    /// 卡类型「NF 外花键参数表」：
    /// `OCSMCARD NF外花键参数表 A300 M7.5 [Z38] [中心 齿面|外径] [根 平|圆] [配合 …] [at x,y] [rot 度]`。
    /// 表格块几何内建（`OCSM_NFTABLE_NF_EXT`，照 `外花键参数表NF.dxf` 原版）；
    /// 18 个值写 INSERT.attributes（K/W 表外 → 对应格「—」，不臆造）。
    fn cmd_nf_ext_card(&self, host: &mut dyn HostApi, args: &str) {
        use crate::nf_ext_table::NfExtTableSpec;
        let spec = match NfExtTableSpec::parse(args) {
            Ok(s) => s,
            Err(e) => {
                host.push_error(&e);
                return;
            }
        };
        host.ensure_layers(layer_defs());
        host.ensure_text_styles(text_style_defs());
        let block = crate::nf_ext_table::BLOCK;
        if host.document().block_records.get(block).is_none() {
            let members = crate::nf_ext_table::block_entities();
            if let Err(e) = host.add_block_record(block, members) {
                host.push_error(&crate::i18n::t_fmt("cmd.card.nfext.err_block", &[("block", block), ("e", &e)]));
                return;
            }
        }
        let at = spec.at.unwrap_or_else(|| {
            crate::take_parts_point()
                .map(|p| [p[0], p[1]])
                .unwrap_or([0.0, 0.0])
        });
        let ins = match crate::nf_ext_table::build_insert(&spec, at, spec.rot) {
            Ok(i) => i,
            Err(e) => {
                host.push_error(&crate::i18n::t_fmt("cmd.card.nfext.err_prefix", &[("e", &e)]));
                return;
            }
        };
        host.push_undo(&crate::i18n::t("cmd.card.nfext.undo"));
        let handles = host.add_entities(vec![acadrust::EntityType::Insert(ins)]);
        if handles.is_empty() {
            host.push_error(&crate::i18n::t("cmd.card.nfext.err_no_handle"));
            return;
        }
        host.set_dirty();
        let note = crate::nf_ext_table::NfExtTableModel {
            card: "NF外花键参数表".into(),
            expr: None,
            a: spec.a,
            m: spec.m,
            z: spec.z,
            centering: Some(spec.centering.id().into()),
            root: Some(spec.root.id().into()),
            fit: Some(spec.fit.id().into()),
            at: spec.at,
            rot: spec.rot,
        }
        .echo_note()
        .unwrap_or_else(|_| String::new());
        host.push_info(&crate::i18n::t_fmt(
            "cmd.card.nfext.info",
            &[
                ("note", &note),
                ("x", &format!("{:.3}", at[0])),
                ("y", &format!("{:.3}", at[1])),
                ("rot", &crate::partgen_kit::trim(spec.rot)),
            ],
        ));
    }

    /// 卡类型「DIN 花键参数表」（内/外两张，一卡一方向）：
    /// `OCSMCARD DIN花键参数表_外 M3 Z38 B120 W8f`。
    /// 单栏 Bild 6 版面（`OCSM_DINTABLE_DIN_INT`/`_EXT`）；13 个值写 INSERT.attributes；
    /// 整表 INSERT 缩放 0.17（不改块几何）。
    fn cmd_din_card(&self, host: &mut dyn HostApi, args: &str, hub: bool) {
        use crate::din_table::DinTableSpec;
        let spec = match DinTableSpec::parse(args, hub) {
            Ok(s) => s,
            Err(e) => {
                host.push_error(&e);
                return;
            }
        };
        host.ensure_layers(layer_defs());
        host.ensure_text_styles(text_style_defs());
        let block = crate::din_table::block_name(hub);
        if host.document().block_records.get(block).is_none() {
            let members = crate::din_table::block_entities(hub);
            if let Err(e) = host.add_block_record(block, members) {
                host.push_error(&crate::i18n::t_fmt("cmd.card.din.err_block", &[("block", block), ("e", &e)]));
                return;
            }
        }
        let at = spec.at.unwrap_or_else(|| {
            crate::take_parts_point()
                .map(|p| [p[0], p[1]])
                .unwrap_or([0.0, 0.0])
        });
        let ins = match crate::din_table::build_insert(&spec, hub, at, spec.rot) {
            Ok(i) => i,
            Err(e) => {
                host.push_error(&crate::i18n::t_fmt("cmd.card.din.err_prefix", &[("e", &e)]));
                return;
            }
        };
        host.push_undo(&crate::i18n::t("cmd.card.din.undo"));
        let handles = host.add_entities(vec![acadrust::EntityType::Insert(ins)]);
        if handles.is_empty() {
            host.push_error(&crate::i18n::t("cmd.card.din.err_no_handle"));
            return;
        }
        host.set_dirty();
        let note = crate::din_table::DinTableModel {
            card: if hub { "DIN花键参数表".into() } else { "DIN花键参数表_外".into() },
            side: Some(if hub { "int".into() } else { "ext".into() }),
            expr: None,
            m: spec.m,
            z: spec.z,
            d_b: spec.d_b,
            hub: Some(spec.hub.token()),
            shaft: Some(spec.shaft.token()),
            e2: spec.e2_s1,
            ae: spec.ae,
            as_: spec.as_,
            tact_n: spec.tact_hub,
            teff_n: spec.teff_hub,
            tact_w: spec.tact_shaft,
            teff_w: spec.teff_shaft,
            at: spec.at,
            rot: spec.rot,
        }
        .echo_note()
        .unwrap_or_else(|_| String::new());
        host.push_info(&crate::i18n::t_fmt(
            "cmd.card.din.info",
            &[
                ("note", &note),
                ("x", &format!("{:.3}", at[0])),
                ("y", &format!("{:.3}", at[1])),
                ("rot", &crate::partgen_kit::trim(spec.rot)),
            ],
        ));
    }

    /// `OCSMBOMEDIT` / `BOMEDIT`：打开**明细表网页编辑器**（chromium `--app` 独立窗口，
    /// 与零件库/螺栓副/引导窗口同一个开窗方式）。
    ///
    /// 原来只有一条路：Ctrl+点击图纸里的表头/行块（`PE_URL` 链接）—— 那是宿主
    /// `open_url` → `xdg-open`，会落到**带地址栏的浏览器标签页**里，且链接是持久化在
    /// 图纸里的（重开图后端口会变）。这个命令不依赖链接，直接从插件开 app 窗口，
    /// 并把窗口钉在当前图纸上（`tab=`）。
    fn cmd_bom_edit(&self, host: &mut dyn HostApi) {
        let Some(port) = self.ensure_guide_server(host) else {
            host.push_error("OCSMBOMEDIT: 无法启动明细表服务（宿主不支持 worker 请求）。");
            return;
        };
        let opened = open_plugin_page(
            port,
            &with_tab("/bom.html", Some(host.tab_id())),
            "bom",
            1180,
            900,
        );
        host.push_info(if opened {
            "OCSMBOM：明细表编辑器已打开（独立窗口）。改完点「应用到图纸」；也可继续 Ctrl+点击图纸里的表块回到本页。"
        } else {
            "OCSMBOM：明细表编辑器已打开（Alt+Tab 切换过去）。"
        });
    }

    /// `OCSMHELP` / `OH`：打开命令手册窗口（命令目录 + 操作教程）。
    ///
    /// 教程正文是**磁盘上的 md**（`guide_server::manual_dirs()` 搜索：
    /// `OCSM_MANUAL_DIR` → `~/.agents/skills/ocsm-manual/manual` → 仓库 `handbook/`），
    /// 与 AI 侧 skill 共用同一批文件——改一处两边都更新。
    fn cmd_help(&self, host: &mut dyn HostApi) {
        let Some(port) = self.ensure_guide_server(host) else {
            host.push_error("OCSMHELP: 无法启动手册服务（宿主不支持 worker 请求）。");
            return;
        };
        if open_manual_window(port) {
            host.push_info(
                "OCSM 命令手册：已打开手册窗口（左侧命令目录 + 操作教程，教程正文来自磁盘上的手册 md）。",
            );
        } else {
            host.push_info("OCSM 命令手册：手册窗口已打开（Alt+Tab 切换过去）。");
        }
    }

    /// `OCSMJOINT`：按件链装配螺栓副（几何/长度/放置确定性，可撤销）。
    ///
    /// **不判断件链**（该不该加平垫/弹垫/防松件属于工况判断）：件链由 skill/工艺层给出，
    /// 命令只负责“算得对、放得准、能回退”。
    fn cmd_joint(&self, host: &mut dyn HostApi, args: &str) {
        // 不带参数 = **人类侧 GUI**（与 OCSMPART 一致）：开螺栓副窗口 + 进放置态，
        // 窗口里点「装配到图纸」→ 回图纸点基点 → 光标旋转 → 再点落定。
        if args.trim().is_empty() {
            let Some(port) = self.ensure_guide_server(host) else {
                host.push_error("OCSMJOINT: 无法启动螺栓副服务（宿主不支持 worker 请求）。");
                return;
            };
            if open_joint_window(port, Some(host.tab_id())) {
                host.push_info(
                    "OCSM 螺栓副：已打开装配窗口。选好螺栓/件链点「装配到图纸」→ 回到图纸点击定位基点 → 移动光标旋转 → 再点击落定（可连续，Esc 结束）。",
                );
            } else {
                host.push_info("OCSM 螺栓副：装配窗口已打开（Alt+Tab 切换过去）。");
            }
            let Some(sender) = host.plugin_request_sender() else { return };
            host.start_interactive(Box::new(JointPlace {
                sender: std::sync::Arc::from(sender),
                phase: std::cell::Cell::new(PlacePhase::Follow),
                base: std::cell::Cell::new([0.0, 0.0, 0.0]),
            }));
            return;
        }
        let spec = match crate::joint::JointSpec::parse(args) {
            Ok(spec) => spec,
            Err(e) => {
                host.push_error(&format!("OCSMJOINT: {e}"));
                return;
            }
        };
        let Some(sender) = host.plugin_request_sender() else {
            host.push_error("OCSMJOINT: 宿主不支持 worker 请求，无法装配。");
            return;
        };
        let sender: std::sync::Arc<dyn PluginRequestSender> = std::sync::Arc::from(sender);
        match crate::guide_server::apply_joint(&sender, spec.to_json().as_bytes()) {
            Ok(body) => {
                let report = serde_json::from_str::<serde_json::Value>(&body)
                    .ok()
                    .and_then(|value| value["report"].as_str().map(str::to_string))
                    .unwrap_or(body);
                host.push_output(&report);
            }
            Err(e) => host.push_error(&format!("OCSMJOINT: {e}")),
        }
    }

    /// `OCSMMCP`：确保标注更新服务器运行，打印 MCP 接入信息。
    fn cmd_mcp(&self, host: &mut dyn HostApi) {
        let Some(port) = self.ensure_guide_server(host) else {
            host.push_error("OCSM: 无法启动标注更新服务器（宿主不支持 worker 请求）。");
            return;
        };
        host.push_output(&format!(
            "OCSM MCP：标注更新服务器已就绪 http://127.0.0.1:{port} \
             （浏览器 GUI /api/guide 等）。MCP 服务器为独立二进制，AI 客户端经 \
             stdio 启动后经 TCP 桥接到本端口。"
        ));
    }

    /// 确保标注更新服务器在跑，返回端口；失败返回 None。
    fn ensure_guide_server(&self, host: &mut dyn HostApi) -> Option<u16> {
        let sender = std::sync::Arc::from(host.plugin_request_sender()?);
        // 每条命令都走到这里（dispatch 开头）→ 顺手把「当前图纸」的 sender 刷新进
        // 路由表：关图 → 新建图之后，HTTP 侧不再对着已经关闭的标签页发请求。
        let port = ensure_guide_server_running(sender, Some(host.tab_id()))?;
        // 端口与上次不同（含首次、含插件重启后端口变了）→ 把图纸里已有 OCSM 标注的
        // “回编辑”链接重写到当前端口，否则老标注 Ctrl+点击会打开失效地址。
        refresh_edit_links(host, port);
        Some(port)
    }

    /// 数字键 1~10：有选中 → 移动对象到目标层；无选中 → 切当前图层。
    fn cmd_layer_key(&self, host: &mut dyn HostApi, digit: usize) {
        let prefix = digit.to_string();
        let Some(def) = layer_defs()
            .iter()
            .find(|d| d.name.starts_with(&prefix))
            .cloned()
        else {
            host.push_error("OCSM: 图层模板缺失。请先运行 OCSM 初始化。");
            return;
        };
        let target = def.name.clone();

        let selected = host.selected_handles();
        if selected.is_empty() {
            if host.set_current_layer(&target) {
                host.push_info(&format!("当前图层已切换到「{target}」。"));
            } else {
                host.push_error(&format!(
                    "图层「{target}」不存在。请先运行 OCSM 初始化。"
                ));
            }
            return;
        }

        // 有选中对象：移动（不切当前层），记 undo、保留选择集。
        host.push_undo(&format!("移动到图层{target}"));
        let mut moved = 0usize;
        let mut refused = 0usize;
        for h in selected {
            let Some(mut entity) = host.document().get_entity(h).cloned() else {
                continue;
            };
            entity.common_mut().layer = target.clone();
            if host.update_entity(entity) {
                moved += 1;
            } else {
                refused += 1; // 锁定层对象被宿主拒绝
            }
        }
        let mut msg = format!("已将 {moved} 个对象移动到「{target}」。");
        if refused > 0 {
            msg.push_str(&format!("（{refused} 个在锁定图层上，已跳过）"));
        }
        host.push_info(&msg);
    }

    /// `TF` / `OCSMFRAMEINIT`：
    ///
    /// * **不带参数** → 扫描图框文件夹并打开宿主侧选择窗口（**人侧**，与原来一致）；
    /// * **带参数** → `TF <名称> [比例] [at x,y] [rot 度]` **直接插图框**（**AI/自动化侧**）：
    ///   给了 `at` 就一次落图（不进交互），只给名称/比例则进光标跟随放置。
    fn cmd_frame_init(&self, host: &mut dyn HostApi, rest: &str) {
        match parse_frame_args(rest) {
            Err(e) => host.push_error(&format!("TF：{e}")),
            Ok(Some(args)) => self.cmd_frame_direct(host, args),
            Ok(None) => self.cmd_frame_picker(host),
        }
    }

    /// `TF` 带参数：把名称解析到 `frame/*.dwg` 后直接走插入。
    fn cmd_frame_direct(&self, host: &mut dyn HostApi, args: FrameArgs) {
        let frames = match frame_files() {
            Ok(f) => f,
            Err(e) => {
                host.push_error(&format!("TF：{e}"));
                return;
            }
        };
        let item = match resolve_frame(&frames, &args.name) {
            Ok(item) => item,
            Err(e) => {
                host.push_error(&format!("TF：{e}"));
                return;
            }
        };
        self.frame_insert(
            host,
            &item.path,
            args.v1,
            args.v2,
            args.at,
            args.rot_deg,
            "TF",
        );
    }

    /// `TF` 不带参数：打开图框选择窗口（人侧流程）。
    fn cmd_frame_picker(&self, host: &mut dyn HostApi) {
        let frames = match frame_files() {
            Ok(f) => f,
            Err(e) => {
                host.push_error(&format!("OCSMFRAMEINIT：{e}"));
                return;
            }
        };
        host.push_info(&crate::i18n::t_fmt(
            "cmd.frame.info.found_n",
            &[("n", &frames.len().to_string())],
        ));
        if !host.show_frame_picker(frames) {
            host.push_error(&crate::i18n::t("cmd.frame.err.picker_failed"));
        }
    }

    /// 定义图框块 + 插入：`at` 给了就一次落图，否则进交互放置（光标跟随）。
    ///
    /// `who` 只用于错误信息前缀（`TF` / `OCSMFRAMEINSERT`）。
    fn frame_insert(
        &self,
        host: &mut dyn HostApi,
        path: &str,
        v1: i64,
        v2: i64,
        at: Option<[f64; 3]>,
        rot_deg: f64,
        who: &str,
    ) {
        // 复校比例：正整数且其一为 1。
        if v1 <= 0 || v2 <= 0 || (v1 != 1 && v2 != 1) {
            host.push_error(&crate::i18n::t_fmt(
                "cmd.frame.err.bad_scale",
                &[("who", who)],
            ));
            return;
        }
        let scale = v2 as f64 / v1 as f64;
        let scale_text = format!("{v1}:{v2}");
        let block_name = std::path::Path::new(path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "frame".to_string());

        // 图框比例感知：为本次缩放创建对应标注样式（如 OCSM_GB_x2），
        // POWERDIM 在 frame 内标注时按此样式放大文字/箭头。
        // 缩放样式（以及随后的块导入/插入）都要可撤销：宿主 import 自带 push，
        // 这里补一次覆盖缩放样式创建；无改动时宿主会丢弃空条目。
        host.push_undo(&crate::i18n::t("cmd.frame.undo_label"));
        if (scale - 1.0).abs() > 1e-9 {
            host.ensure_dim_styles(vec![scaled_dim_style_def(scale)]);
        }

        // 先在宿主侧定义块（幂等），并取回 ATTDEF 供插入时构造属性。
        let attdefs = match host.import_frame_block(ImportFrameBlockRequest {
            path: path.to_string(),
            block_name: block_name.clone(),
        }) {
            Ok(attdefs) => attdefs,
            Err(e) => {
                host.push_error(&format!("{who}: {e}"));
                return;
            }
        };

        match at {
            None => {
                host.push_info(&crate::i18n::t_fmt(
                    "cmd.frame.info.specify_point",
                    &[
                        ("block", &block_name),
                        ("scale", &scale_text),
                        ("s", &format!("{scale:.2}")),
                    ],
                ));
                host.start_interactive(Box::new(FramePlace {
                    block_name,
                    scale,
                    scale_text,
                    attdefs,
                }));
            }
            Some(base) => {
                let ins =
                    build_frame_insert(&block_name, scale, &scale_text, &attdefs, base, rot_deg);
                let _ = host.add_entities(vec![acadrust::EntityType::Insert(ins)]);
                host.set_dirty();
                host.push_info(&crate::i18n::t_fmt(
                    "cmd.frame.info.inserted",
                    &[
                        ("block", &block_name),
                        ("scale", &scale_text),
                        ("s", &format!("{scale:.2}")),
                        ("x", &trim_num(base[0])),
                        ("y", &trim_num(base[1])),
                        ("rot", &trim_num(rot_deg)),
                    ],
                ));
                host.push_output(&crate::i18n::t_fmt(
                    "cmd.frame.output.inserted",
                    &[("block", &block_name), ("scale", &scale_text)],
                ));
            }
        }
    }

    /// `OCSMFRAMEINSERT`：带参数同 `TF <名称> <比例> at x,y`；
    /// 不带参数 = 取走选择窗口的结果（人侧两步流程）。
    fn cmd_frame_insert(&self, host: &mut dyn HostApi, rest: &str) {
        if !rest.trim().is_empty() {
            return self.cmd_frame_init(host, rest);
        }
        let Some(sel) = host.take_pending_frame_selection() else {
            host.push_error(&crate::i18n::t("cmd.frame.err.no_pending"));
            return;
        };
        self.frame_insert(
            host,
            &sel.path,
            sel.scale_v1,
            sel.scale_v2,
            None,
            0.0,
            "OCSMFRAMEINSERT",
        );
    }

    /// `D` / `OCSMPOWERDIM`：智能标注（点/直线/圆/圆弧 → 自动推断标注类型，
    /// 光标跟随预览，frame 比例感知）。
    fn cmd_powerdim(&self, host: &mut dyn HostApi) {
        // 标注固定放 7标注层（模板层），兜底确保存在；同时确保文字/标注样式
        // 就绪——否则样式缺失时标注回退默认参数（文字变图层色、尺寸异常）。
        // 样式补齐也是文档改动 → 先声明撤销点（标注实体本身由宿主在每次提交时
        // 各自入栈，这里覆盖的是样式表）。
        host.push_undo("OCSM 智能标注");
        host.ensure_layers(layer_defs());
        host.ensure_text_styles(text_style_defs());
        host.ensure_dim_styles(dim_style_defs());
        // 本次 dispatch 的 host proxy 是新建的，document() 快照即最新
        // （用户此前画的线/圆/圆弧/图框都在里面），供命令内几何查询。
        let doc = host.document().clone();
        host.push_info(
            "OCSMPOWERDIM: 拾取点模式（选端点/圆心/交点），Enter 切换线段点选；Esc 取消。",
        );
        host.start_interactive(Box::new(PowerDim::new(doc)));
    }
}

// ── OCSMPOWERDIM ────────────────────────────────────────────────────────────

/// 拾取对象的最小几何描述（命令内不持有宿主，只读快照）。
#[derive(Clone, Copy)]
pub(crate) struct LineGeom {
    start: [f64; 3],
    end: [f64; 3],
}

#[derive(Clone, Copy)]
struct CircleGeom {
    center: [f64; 3],
    radius: f64,
}

#[derive(Clone, Copy)]
struct ArcGeom {
    center: [f64; 3],
    radius: f64,
    start_angle: f64,
    end_angle: f64,
}

#[derive(Clone, Copy)]
enum PickKind {
    Line(LineGeom),
    Circle(CircleGeom),
    Arc(ArcGeom),
}

#[derive(Clone, Copy, PartialEq)]
enum PlaceMode {
    Aligned,
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, PartialEq)]
enum CircleMode {
    Diameter,
    Radius,
}

#[derive(Clone, Copy, PartialEq)]
enum ArcMode {
    Radius,
    ArcLength,
}

enum Step {
    /// 等待第一个点击（对象或拾取点）。
    First,
    /// 已有一个拾取点，等待第二个。
    TwoPoints { first: [f64; 3] },
    /// 两个拾取点，等待放置点（A/H/V 关键字可锁定行为）。
    TwoPointPlace {
        first: [f64; 3],
        second: [f64; 3],
        mode: PlaceMode,
    },
    /// 已点一条直线，等待放置点。
    LinePlace {
        line: LineGeom,
        mode: PlaceMode,
    },
    /// 两直线夹角（劣角/优角关键字可切换）。
    AngularPlace {
        line1: LineGeom,
        line2: LineGeom,
        major: bool,
    },
    /// 圆：直径/半径。
    Circle {
        circle: CircleGeom,
        mode: CircleMode,
    },
    /// 圆弧：半径/弧长。
    Arc {
        arc: ArcGeom,
        mode: ArcMode,
    },
}

/// 拾取模式：`Points` = 双拾取点（对象捕捉开）；`Segments` = 线段点选
/// （对象捕捉关，直接点直线/圆/圆弧，光标只留拾取方框）。按 Enter 切换。
#[derive(Clone, Copy, PartialEq)]
enum PickMode {
    Points,
    Segments,
}

impl Clone for Step {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for Step {}

/// 智能标注交互命令。全程 entity-pick（`snapped=true` 的点视为拾取点，
/// 未吸附的实体点击视为对象选择；空白点击=自由拾取点）。
struct PowerDim {
    doc: acadrust::CadDocument,
    step: Step,
    /// 拾取模式：Points（默认，对象捕捉开）↔ Segments（Enter 切换，点选线段）。
    pick_mode: PickMode,
    /// 第一个拾取点所在 frame 的缩放比例（决定标注样式）。
    frame_scale: f64,
    style_name: String,
    /// 样式缺失时的提示（附加在 prompt 尾部）。
    style_hint: Option<String>,
    /// 刚点了平行线时的提示。
    parallel_notice: bool,
}

impl PowerDim {
    fn new(doc: acadrust::CadDocument) -> Self {
        Self {
            doc,
            step: Step::First,
            pick_mode: PickMode::Points,
            frame_scale: 1.0,
            style_name: "OCSM_GB".to_string(),
            style_hint: None,
            parallel_notice: false,
        }
    }

    /// 按句柄从快照读取几何。
    fn classify(&self, handle: acadrust::Handle) -> Option<PickKind> {
        use acadrust::entities::EntityType as E;
        let e = self.doc.get_entity(handle)?;
        match e {
            E::Line(l) => Some(PickKind::Line(LineGeom {
                start: to_arr(&l.start),
                end: to_arr(&l.end),
            })),
            E::Circle(c) => Some(PickKind::Circle(CircleGeom {
                center: to_arr(&c.center),
                radius: c.radius,
            })),
            E::Arc(a) => Some(PickKind::Arc(ArcGeom {
                center: to_arr(&a.center),
                radius: a.radius,
                start_angle: a.start_angle,
                end_angle: a.end_angle,
            })),
            _ => None,
        }
    }

    /// 第一个拾取点处计算 frame 缩放比例，选定标注样式。
    fn resolve_style(&mut self, pt: [f64; 3]) {
        let scale = frame_scale_at(&self.doc, pt);
        self.frame_scale = scale;
        if (scale - 1.0).abs() < 1e-9 {
            self.style_name = "OCSM_GB".to_string();
            self.style_hint = None;
            return;
        }
        let name = format!("OCSM_GB_x{}", trim_scale(scale));
        if self
            .doc
            .dim_styles
            .iter()
            .any(|s| s.name.eq_ignore_ascii_case(&name))
        {
            self.style_name = name;
            self.style_hint = None;
        } else {
            // 缩放样式不存在（如旧图）：回退 OCSM_GB 并在提示里说明。
            self.style_name = "OCSM_GB".to_string();
            self.style_hint = Some(format!(
                "（未找到标注样式 {name}，已回退 OCSM_GB——请重新插入图框或运行 OCSM）"
            ));
        }
    }

    fn set_place_mode(&mut self, mode: PlaceMode) {
        match &mut self.step {
            Step::TwoPointPlace { mode: m, .. } | Step::LinePlace { mode: m, .. } => *m = mode,
            _ => {}
        }
    }

    fn set_major(&mut self, major: bool) {
        if let Step::AngularPlace { major: m, .. } = &mut self.step {
            *m = major;
        }
    }

    fn set_circle_mode(&mut self, mode: CircleMode) {
        if let Step::Circle { mode: m, .. } = &mut self.step {
            *m = mode;
        }
    }

    fn set_arc_mode(&mut self, mode: ArcMode) {
        if let Step::Arc { mode: m, .. } = &mut self.step {
            *m = mode;
        }
    }

    /// 构建线性/对齐标注（放置点=pt），并盖上图层/样式。
    fn build_linear(
        &self,
        first: [f64; 3],
        second: [f64; 3],
        pt: [f64; 3],
        mode: PlaceMode,
    ) -> Dimension {
        use acadrust::entities::{DimensionAligned, DimensionLinear};
        let (first, second, pt) = (v3(first), v3(second), v3(pt));
        let lift = linear_text_lift(&self.style_name);
        let dim = match mode {
            PlaceMode::Aligned => {
                let mut d = DimensionAligned::new(first, second);
                let axis = (second - first).normalize();
                d.definition_point = pt;
                d.base.definition_point = pt;
                let tp = linear_text_pos(first, second, pt, axis, lift);
                d.base.text_middle_point = tp;
                d.base.insertion_point = tp;
                d.base.actual_measurement = d.measurement();
                Dimension::Aligned(d)
            }
            PlaceMode::Horizontal => {
                let mut d = DimensionLinear::horizontal(first, second);
                let tp = linear_text_pos(first, second, pt, Vector3::new(1.0, 0.0, 0.0), lift);
                d.definition_point = pt;
                d.base.definition_point = pt;
                d.base.text_middle_point = tp;
                d.base.insertion_point = tp;
                d.base.actual_measurement = d.measurement();
                Dimension::Linear(d)
            }
            PlaceMode::Vertical => {
                let mut d = DimensionLinear::vertical(first, second);
                let tp = linear_text_pos(first, second, pt, Vector3::new(0.0, 1.0, 0.0), lift);
                d.definition_point = pt;
                d.base.definition_point = pt;
                d.base.text_middle_point = tp;
                d.base.insertion_point = tp;
                d.base.actual_measurement = d.measurement();
                Dimension::Linear(d)
            }
        };
        stamp(dim, &self.style_name)
    }

    /// 圆/圆弧：直径或半径（光标在内→内标注，在外→外标注+引出线）。
    fn build_radial(
        &self,
        center: [f64; 3],
        radius: f64,
        pt: [f64; 3],
        mode: CircleMode,
    ) -> Dimension {
        use acadrust::entities::{DimensionDiameter, DimensionRadius};
        let (center, pt) = (v3(center), v3(pt));
        let rim = rim_toward(center, pt, radius);
        let dist = pt.distance(&rim);
        let leader = if dist < radius { 0.0 } else { dist };
        let dim = match mode {
            CircleMode::Diameter => {
                let mut d = DimensionDiameter::new(center, rim);
                d.base.definition_point = rim;
                d.base.text_middle_point = pt;
                d.base.insertion_point = pt;
                d.leader_length = leader;
                // 注意：v0.9.8 cadcodec 的 Diameter measurement() = 两点距离
                //（圆心↔圆周点 = 半径），直径需 ×2。
                d.base.actual_measurement = d.measurement() * 2.0;
                Dimension::Diameter(d)
            }
            CircleMode::Radius => {
                let mut d = DimensionRadius::new(center, rim);
                d.base.text_middle_point = pt;
                d.base.insertion_point = pt;
                d.leader_length = leader;
                d.base.actual_measurement = d.measurement();
                Dimension::Radius(d)
            }
        };
        let mut dim = stamp(dim, &self.style_name);
        // 文字跟随点击/光标位置（否则宿主按样式重算，尺寸线总在圆内侧）。
        dim.base_mut().text_user_positioned = true;
        // 宿主自然旋转对半径/直径返回 0（水平）；这里插件直接把字旋转角设为
        // 径向方向（从圆心指向文字点），并 clamp 到 (-90°, 90°] 防倒置。
        let ang = (pt.y - center.y).atan2(pt.x - center.x);
        let pi = std::f64::consts::PI;
        let rot = if ang > pi / 2.0 {
            ang - pi
        } else if ang <= -pi / 2.0 {
            ang + pi
        } else {
            ang
        };
        dim.base_mut().text_rotation = rot;
        dim
    }

    /// 圆弧：半径或弧长。
    fn build_arc(&self, arc: &ArcGeom, pt: [f64; 3], mode: ArcMode) -> Dimension {
        use acadrust::entities::{DimensionArc, DimensionBase, DimensionType};
        match mode {
            ArcMode::Radius => self.build_radial(arc.center, arc.radius, pt, CircleMode::Radius),
            ArcMode::ArcLength => {
                let center = v3(arc.center);
                let start_pt = center
                    + Vector3::new(arc.start_angle.cos(), arc.start_angle.sin(), 0.0) * arc.radius;
                let end_pt = center
                    + Vector3::new(arc.end_angle.cos(), arc.end_angle.sin(), 0.0) * arc.radius;
                let def = v3(pt);
                let outside = def.distance(&center) > arc.radius;
                // 被标注弧的实际扫角（rem_euclid 规范化跨 0° 方向），
                // 保证优弧（>180°）按优弧长度标注并画优弧。
                let sweep = (arc.end_angle - arc.start_angle).rem_euclid(std::f64::consts::TAU);
                let mut d = DimensionArc {
                    base: DimensionBase::new(DimensionType::ArcLength),
                    definition_point: def,
                    first_extension_point: start_pt,
                    second_extension_point: end_pt,
                    center_point: center,
                    is_partial: true,
                    arc_start_parameter: arc.start_angle,
                    // 端点=起点+实际扫角，measurement = r × sweep（含优弧）。
                    arc_end_parameter: arc.start_angle + sweep,
                    has_leader: outside,
                    first_leader_point: def,
                    second_leader_point: rim_toward(center, def, arc.radius),
                };
                d.base.text_middle_point = def;
                d.base.insertion_point = def;
                d.base.actual_measurement = d.measurement();
                let mut dim = stamp(Dimension::Arc(d), &self.style_name);
                dim.base_mut().text_user_positioned = true;
                dim
            }
        }
    }

    /// 两直线夹角标注（劣角默认；优角 = 360°−劣角，弧取优弧）。
    fn build_angular(
        &self,
        line1: LineGeom,
        line2: LineGeom,
        pt: [f64; 3],
        major: bool,
    ) -> Dimension {
        use acadrust::entities::{DimensionAngular2Ln, DimensionBase, DimensionType};
        let minor_deg = angular_minor_sweep_deg(&line1, &line2, pt);
        let measurement = if major {
            360.0 - minor_deg
        } else {
            minor_deg
        };
        let mut d = DimensionAngular2Ln {
            base: DimensionBase::new(DimensionType::Angular),
            dimension_arc: v3(pt),
            first_point: v3(line1.start),
            second_point: v3(line1.end),
            angle_vertex: v3(line2.start),
            definition_point: v3(line2.end),
        };
        d.base.text_middle_point = v3(pt);
        d.base.insertion_point = v3(pt);
        d.base.actual_measurement = measurement;
        // 宿主对 Angular2Ln 的文字用 measurement()（对两线存储语义错误，
        // 可能导致恒 90°）。不改宿主，这里直接用 user_text 提供正确角度值。
        d.base.user_text = Some(format!("{:.0}°", measurement));
        let mut dim = stamp(Dimension::Angular2Ln(d), &self.style_name);
        dim.base_mut().text_user_positioned = true;
        dim
    }
}

impl ocs_plugin_api::host::InteractiveCommand for PowerDim {
    fn prompt(&self) -> String {
        let base = match &self.step {
            Step::First => {
                if matches!(self.pick_mode, PickMode::Points) {
                    "OCSMPOWERDIM 拾取点模式：选择端点/圆心/交点等，或 Enter 切换为线段点选。".to_string()
                } else {
                    "OCSMPOWERDIM 线段点选模式：点击直线/圆/圆弧创建标注，或 Enter 返回拾取点。".to_string()
                }
            }
            Step::TwoPoints { .. } => "选择下一个拾取点。".to_string(),
            Step::TwoPointPlace { .. } | Step::LinePlace { .. } => {
                "选择标注行为：`对齐(A默认)`,`水平(H)`,`竖直(V)`。选择一根与当前直线不平行的直线以创建角度标注。或选择下一个拾取点以放置标注。".to_string()
            }
            Step::AngularPlace { .. } => {
                "选择标注的行为：`劣角(I默认)`,`优角(R)`。或选择下一个拾取点以放置标注。".to_string()
            }
            Step::Circle { .. } => {
                "选择标注行为：`直径(D默认)`，`半径(R)`。或选择下一个拾取点以放置标注。".to_string()
            }
            Step::Arc { .. } => {
                "选择标注行为：`半径(R默认)`，`弧长(A)`。或选择下一个拾取点以放置标注。".to_string()
            }
        };
        if self.parallel_notice {
            format!("{base}（两直线平行，无法创建角度标注）")
        } else if let Some(hint) = &self.style_hint {
            format!("{base}{hint}")
        } else {
            base
        }
    }

    fn on_point(&mut self, _pt: [f64; 3]) -> CommandStep {
        CommandStep::NeedPoint
    }

    fn on_enter(&mut self) -> CommandStep {
        // First 状态：Enter 在 拾取点模式 ↔ 线段点选模式 间切换；
        // 其它状态：Enter 取消命令。
        if matches!(self.step, Step::First) {
            self.pick_mode = match self.pick_mode {
                PickMode::Points => PickMode::Segments,
                PickMode::Segments => PickMode::Points,
            };
            CommandStep::NeedPoint
        } else {
            CommandStep::Cancel
        }
    }

    fn wants_text_input(&self) -> bool {
        true
    }

    fn on_text_input(&mut self, text: &str) -> CommandStep {
        let kw = text.trim().to_ascii_uppercase();
        let mut consumed = false;
        match &self.step {
            Step::TwoPointPlace { .. } | Step::LinePlace { .. } => match kw.as_str() {
                "A" | "ALIGNED" => {
                    self.set_place_mode(PlaceMode::Aligned);
                    consumed = true;
                }
                "H" | "HORIZONTAL" => {
                    self.set_place_mode(PlaceMode::Horizontal);
                    consumed = true;
                }
                "V" | "VERTICAL" => {
                    self.set_place_mode(PlaceMode::Vertical);
                    consumed = true;
                }
                _ => {}
            },
            Step::AngularPlace { .. } => match kw.as_str() {
                "I" | "INTERIOR" | "MINOR" => {
                    self.set_major(false);
                    consumed = true;
                }
                "R" | "REFLEX" | "MAJOR" => {
                    self.set_major(true);
                    consumed = true;
                }
                _ => {}
            },
            Step::Circle { .. } => match kw.as_str() {
                "D" | "DIAMETER" => {
                    self.set_circle_mode(CircleMode::Diameter);
                    consumed = true;
                }
                "R" | "RADIUS" => {
                    self.set_circle_mode(CircleMode::Radius);
                    consumed = true;
                }
                _ => {}
            },
            Step::Arc { .. } => match kw.as_str() {
                "R" | "RADIUS" => {
                    self.set_arc_mode(ArcMode::Radius);
                    consumed = true;
                }
                "A" | "ARCLENGTH" | "LENGTH" => {
                    self.set_arc_mode(ArcMode::ArcLength);
                    consumed = true;
                }
                _ => {}
            },
            _ => {}
        }
        if consumed {
            CommandStep::NeedPoint
        } else {
            // 非关键字：让宿主按常规解释（实体拾取步骤 → 句柄读取）。
            CommandStep::Ignored
        }
    }

    fn needs_object_pick(&self) -> bool {
        true
    }

    /// 拾取点模式（默认）跑对象捕捉；线段点选模式（Enter 切换）关闭，
    /// 直接点选直线/圆/圆弧（光标只留拾取方框）。
    fn entity_pick_applies_osnap(&self) -> bool {
        matches!(self.pick_mode, PickMode::Points)
    }

    fn on_object_pick_snapped(
        &mut self,
        handle: acadrust::Handle,
        pt: [f64; 3],
        snapped: bool,
    ) -> CommandStep {
        use acadrust::entities::EntityType as E;
        let kind = if handle.is_null() {
            None
        } else {
            self.classify(handle)
        };
        let is_line = matches!(kind, Some(PickKind::Line(_)));
        let segment_select = matches!(self.pick_mode, PickMode::Segments);
        // Step 的字段全部 Copy，先取一份再改 self.step。
        let step = self.step;
        match step {
            Step::First => {
                // 线段点选模式：只认实体点击（无 OSNAP，snapped 恒 false）；
                // 空白点击忽略。拾取点模式：吸附点/空白=拾取点。
                if segment_select && handle.is_null() {
                    return CommandStep::NeedPoint;
                }
                match kind {
                    Some(PickKind::Circle(c)) if !snapped => {
                        self.resolve_style(pt);
                        self.step = Step::Circle {
                            circle: c,
                            mode: CircleMode::Diameter,
                        };
                    }
                    Some(PickKind::Arc(a)) if !snapped => {
                        self.resolve_style(pt);
                        self.step = Step::Arc {
                            arc: a,
                            mode: ArcMode::Radius,
                        };
                    }
                    Some(PickKind::Line(l)) if !snapped => {
                        self.resolve_style(pt);
                        self.step = Step::LinePlace {
                            line: l,
                            mode: PlaceMode::Aligned,
                        };
                    }
                    _ => {
                        // 拾取点（吸附点 / 自由点击 / 非几何实体点击）。
                        self.resolve_style(pt);
                        self.step = Step::TwoPoints { first: pt };
                    }
                }
            }
            Step::TwoPoints { first } => {
                self.step = Step::TwoPointPlace {
                    first,
                    second: pt,
                    mode: PlaceMode::Aligned,
                };
            }
            Step::TwoPointPlace { .. } | Step::LinePlace { .. } => {
                // 点击另一条（不平行）直线 → 角度标注；否则放置。
                if is_line && !snapped {
                    let line1 = match step {
                        Step::TwoPointPlace { first, second, .. } => LineGeom {
                            start: first,
                            end: second,
                        },
                        Step::LinePlace { line, .. } => line,
                        _ => unreachable!(),
                    };
                    let Some(PickKind::Line(line2)) = kind else {
                        unreachable!()
                    };
                    if lines_parallel(&line1, &line2) {
                        self.parallel_notice = true;
                        return CommandStep::NeedPoint;
                    }
                    self.parallel_notice = false;
                    self.step = Step::AngularPlace {
                        line1,
                        line2,
                        major: false,
                    };
                    return CommandStep::NeedPoint;
                }
                self.parallel_notice = false;
                let dim = match step {
                    Step::TwoPointPlace { first, second, mode } => {
                        self.build_linear(first, second, pt, mode)
                    }
                    Step::LinePlace { line, mode } => {
                        self.build_linear(line.start, line.end, pt, mode)
                    }
                    _ => unreachable!(),
                };
                return CommandStep::CommitAndEnd(E::Dimension(dim));
            }
            Step::AngularPlace {
                line1,
                line2,
                major,
            } => {
                let dim = self.build_angular(line1, line2, pt, major);
                return CommandStep::CommitAndEnd(E::Dimension(dim));
            }
            Step::Circle { circle, mode } => {
                let dim = self.build_radial(circle.center, circle.radius, pt, mode);
                return CommandStep::CommitAndEnd(E::Dimension(dim));
            }
            Step::Arc { arc, mode } => {
                let dim = self.build_arc(&arc, pt, mode);
                return CommandStep::CommitAndEnd(E::Dimension(dim));
            }
        }
        CommandStep::NeedPoint
    }

    fn wants_mouse_move(&self) -> bool {
        !matches!(self.step, Step::First)
    }

    fn on_mouse_move(&mut self, pt: [f64; 3]) -> Option<acadrust::EntityType> {
        use acadrust::entities::{Line, EntityType as E};
        match &self.step {
            Step::First => None,
            Step::TwoPoints { first } => {
                let mut l = Line::new();
                l.start = v3(*first);
                l.end = v3(pt);
                Some(E::Line(l))
            }
            Step::TwoPointPlace { first, second, mode } => {
                Some(E::Dimension(self.build_linear(*first, *second, pt, *mode)))
            }
            Step::LinePlace { line, mode } => {
                Some(E::Dimension(self.build_linear(line.start, line.end, pt, *mode)))
            }
            Step::AngularPlace { line1, line2, major } => Some(E::Dimension(
                self.build_angular(*line1, *line2, pt, *major),
            )),
            Step::Circle { circle, mode } => Some(E::Dimension(self.build_radial(
                circle.center,
                circle.radius,
                pt,
                *mode,
            ))),
            Step::Arc { arc, mode } => {
                Some(E::Dimension(self.build_arc(arc, pt, *mode)))
            }
        }
    }
}

// ── 几何/样式辅助 ───────────────────────────────────────────────────────────

pub(crate) fn v3(p: [f64; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

pub(crate) fn to_arr(v: &Vector3) -> [f64; 3] {
    [v.x, v.y, v.z]
}

/// 标注实体盖上图层（7标注层）+ 样式。
pub(crate) fn stamp(mut dim: Dimension, style_name: &str) -> Dimension {
    let base = dim.base_mut();
    base.common.layer = "7标注层".to_string();
    base.style_name = style_name.to_string();
    dim
}

/// 线性标注文字中心相对尺寸线的法向抬升（世界单位）。
///
/// 与宿主 `dimension_text_pos_f64` 的 `perp_off` 同口径：`DIMTXT/2 + DIMGAP`，
/// 且随样式 `dimscale` 缩放。OCSM_GB 系列样式把 DIMTXT=2.5、DIMGAP=1.0 钉在
/// `dim_override_record` 的类型专属覆盖里（147/140），缩放取样式名
/// `OCSM_GB_x{n}` 的 n → 抬升 = 2.25×n；文字底边离线 `DIMGAP×n`=1.0×n
/// （对照 `OCSMDIMGULIDE1-general.dxf` 匿名块 MTEXT 的 1.0 插入偏移）。
pub(crate) fn linear_text_lift(style_name: &str) -> f64 {
    let scale = style_name
        .strip_prefix("OCSM_GB_x")
        .and_then(|n| n.parse::<f64>().ok())
        .filter(|n| n.is_finite() && *n > 0.0)
        .unwrap_or(1.0);
    (2.5 / 2.0 + 1.0) * scale
}

/// 线性标注文字锚点（= 文字中心，默认 MiddleCenter）位置。
///
/// 先把两个尺寸界线原点投影到过 `def`、方向 `axis` 的尺寸线上取中点，
/// 再沿“文字上方”法向抬升 `lift`（见 `linear_text_lift`）。
/// “上方”与宿主 `text_on_dim_line` 同规则：尺寸线轴归一化到 nx>0
/// （nx=0 时 ny>0），取 up=(-ny,nx)——反向画的尺寸线文字也在读向的上方。
pub(crate) fn linear_text_pos(
    first: Vector3,
    second: Vector3,
    def: Vector3,
    axis: Vector3,
    lift: f64,
) -> Vector3 {
    let perp = Vector3::new(-axis.y, axis.x, 0.0);
    let dperp = def.x * perp.x + def.y * perp.y;
    let d1 = first + perp * (dperp - (first.x * perp.x + first.y * perp.y));
    let d2 = second + perp * (dperp - (second.x * perp.x + second.y * perp.y));
    let mut n = axis;
    if n.x < 0.0 || (n.x == 0.0 && n.y < 0.0) {
        n = n * -1.0;
    }
    let up = Vector3::new(-n.y, n.x, 0.0);
    (d1 + d2) * 0.5 + up * lift
}

/// 从中心朝 `pt` 方向的圆周点。
pub(crate) fn rim_toward(center: Vector3, pt: Vector3, radius: f64) -> Vector3 {
    let dir = pt - center;
    if dir.length() < 1e-9 {
        return center + Vector3::new(radius, 0.0, 0.0);
    }
    center + dir.normalize() * radius
}

/// 两直线是否平行（叉积 ≈ 0）。
fn lines_parallel(a: &LineGeom, b: &LineGeom) -> bool {
    let u = v3(a.end) - v3(a.start);
    let v = v3(b.end) - v3(b.start);
    let cross = u.x * v.y - u.y * v.x;
    cross.abs() <= 1e-9 * u.length().max(v.length()).max(1.0)
}

/// 两直线交点（平行返回 None）。
pub(crate) fn line_intersection_vertex(a: &LineGeom, b: &LineGeom) -> Option<Vector3> {
    let u = v3(a.end) - v3(a.start);
    let v = v3(b.end) - v3(b.start);
    let denom = u.x * v.y - u.y * v.x;
    if denom.abs() <= 1e-9 * u.length().max(v.length()).max(1.0) {
        return None;
    }
    let w = v3(b.start) - v3(a.start);
    let t = (w.x * v.y - w.y * v.x) / denom;
    Some(v3(a.start) + u * t)
}

/// 镜像宿主 two_line_angle_frame 的选择逻辑：以 `arc_pt` 为弧点，返回含该点
/// 的最小劣弧扫过角（度）。保证标注文字与绘制弧一致。
pub(crate) fn angular_minor_sweep_deg(a: &LineGeom, b: &LineGeom, arc_pt: [f64; 3]) -> f64 {
    const TAU: f64 = std::f64::consts::TAU;
    const PI: f64 = std::f64::consts::PI;
    let Some(vertex) = line_intersection_vertex(a, b) else {
        return 0.0;
    };
    let u = v3(a.end) - v3(a.start);
    let v = v3(b.end) - v3(b.start);
    let target = (v3(arc_pt) - vertex).y.atan2((v3(arc_pt) - vertex).x);
    let angle_of = |d: Vector3| d.y.atan2(d.x);
    let mut best: Option<f64> = None;
    for su in [1.0, -1.0] {
        for sv in [1.0, -1.0] {
            let (from_u, from_v) = (angle_of(u * su), angle_of(v * sv));
            for (start, end) in [(from_u, from_v), (from_v, from_u)] {
                let mut sweep = end - start;
                while sweep < 0.0 {
                    sweep += TAU;
                }
                if sweep <= 1e-9 || sweep > PI {
                    continue;
                }
                let into_target = (target - start).rem_euclid(TAU);
                if into_target > sweep {
                    continue;
                }
                if best.is_none_or(|known| sweep < known) {
                    best = Some(sweep);
                }
            }
        }
    }
    best.unwrap_or(0.0).to_degrees()
}

/// 模型空间里含 `pt` 的（最内层）图框 Insert 的均匀缩放比例；无命中 = 1.0。
/// 候选：块名 ∈ frame 目录 stem，或块内含 tag 比例/SCALE 的 ATTDEF。
pub(crate) fn frame_scale_at(doc: &acadrust::CadDocument, pt: [f64; 3]) -> f64 {
    frame_anchor_at(doc, pt).map(|(_, s)| s).unwrap_or(1.0)
}

/// 含 `pt` 的（最内层）图框 Insert：返回（插入点，均匀缩放）。无命中 = None。
fn frame_anchor_at(
    doc: &acadrust::CadDocument,
    pt: [f64; 3],
) -> Option<(Vector3, f64)> {
    use acadrust::EntityType as E;
    let stems = frame_dir_stems();
    let mut best: Option<(f64, Vector3, f64)> = None; // (AABB 面积, 插入点, scale)
    for e in doc.entities() {
        let E::Insert(ins) = e else { continue };
        let Some(scale) = ins.uniform_scale() else { continue };
        if !is_frame_insert(doc, ins, &stems) {
            continue;
        }
        let Some((min, max)) = insert_world_aabb(doc, ins) else {
            continue;
        };
        if pt[0] < min.x || pt[0] > max.x || pt[1] < min.y || pt[1] > max.y {
            continue;
        }
        let area = (max.x - min.x).max(0.0) * (max.y - min.y).max(0.0);
        if best.is_none_or(|(a, _, _)| area < a) {
            best = Some((area, ins.insert_point, scale));
        }
    }
    best.map(|(_, o, s)| (o, s))
}

/// 明细表用的图框锚点：优先“包含名义锚点的最内层图框”；没有命中但**全图只有一张
/// 图框**时用那张（图框不一定在原点）；多张且都不含名义点 → None（按 1:1 原位）。
/// 返回（图框插入点，均匀缩放）。
pub(crate) fn bom_frame_anchor(
    doc: &acadrust::CadDocument,
    nominal_pt: [f64; 3],
) -> Option<(Vector3, f64)> {
    if let Some(hit) = frame_anchor_at(doc, nominal_pt) {
        return Some(hit);
    }
    // 单一图框兜底：图框不在名义锚点下（如整图平移过/放大后名义点落在框外）也能锚上。
    use acadrust::EntityType as E;
    let stems = frame_dir_stems();
    let mut found: Option<(Vector3, f64)> = None;
    let mut count = 0usize;
    for e in doc.entities() {
        let E::Insert(ins) = e else { continue };
        let Some(scale) = ins.uniform_scale() else { continue };
        if !is_frame_insert(doc, ins, &stems) {
            continue;
        }
        count += 1;
        if found.is_none() {
            found = Some((ins.insert_point, scale));
        }
    }
    if count == 1 {
        found
    } else {
        None
    }
}

/// 是否为图框块引用（帧目录 stem 或含 比例/SCALE ATTDEF）。
fn is_frame_insert(
    doc: &acadrust::CadDocument,
    ins: &acadrust::entities::Insert,
    stems: &[String],
) -> bool {
    use acadrust::EntityType as E;
    if stems
        .iter()
        .any(|s| s.eq_ignore_ascii_case(&ins.block_name))
    {
        return true;
    }
    let Some(br) = doc.block_records.get(&ins.block_name) else {
        return false;
    };
    br.entity_handles.iter().any(|h| {
        doc.get_entity(*h).is_some_and(|e| {
            matches!(
                e,
                E::AttributeDefinition(ad)
                    if ad.tag.eq_ignore_ascii_case("比例")
                        || ad.tag.eq_ignore_ascii_case("SCALE")
            )
        })
    })
}

/// Insert 的世界 AABB：块局部 AABB 四角经插入变换。
pub(crate) fn insert_world_aabb(
    doc: &acadrust::CadDocument,
    ins: &acadrust::entities::Insert,
) -> Option<(Vector3, Vector3)> {
    use acadrust::EntityType as E;
    let br = doc.block_records.get(&ins.block_name)?;
    let mut min: Option<Vector3> = None;
    let mut max: Option<Vector3> = None;
    for h in &br.entity_handles {
        let Some(e) = doc.get_entity(*h) else { continue };
        if matches!(e, E::Block(_) | E::BlockEnd(_)) {
            continue;
        }
        let bb = e.as_entity().bounding_box();
        if !bb.min.x.is_finite() || !bb.max.x.is_finite() {
            continue;
        }
        min = Some(match min {
            Some(m) => Vector3::new(m.x.min(bb.min.x), m.y.min(bb.min.y), m.z.min(bb.min.z)),
            None => bb.min,
        });
        max = Some(match max {
            Some(m) => Vector3::new(m.x.max(bb.max.x), m.y.max(bb.max.y), m.z.max(bb.max.z)),
            None => bb.max,
        });
    }
    let (min, max) = (min?, max?);
    let xf = ins.get_transform();
    // 8 角（Z 忽略，但变换保持完整以兼容旋转）。
    let corners = [
        Vector3::new(min.x, min.y, min.z),
        Vector3::new(max.x, min.y, min.z),
        Vector3::new(min.x, max.y, min.z),
        Vector3::new(max.x, max.y, min.z),
    ];
    let mut wmin: Option<Vector3> = None;
    let mut wmax: Option<Vector3> = None;
    for c in corners {
        let w = xf.apply(c);
        wmin = Some(match wmin {
            Some(m) => Vector3::new(m.x.min(w.x), m.y.min(w.y), m.z.min(w.z)),
            None => w,
        });
        wmax = Some(match wmax {
            Some(m) => Vector3::new(m.x.max(w.x), m.y.max(w.y), m.z.max(w.z)),
            None => w,
        });
    }
    Some((wmin?, wmax?))
}

/// frame 目录里所有 DWG 的文件名（不含扩展名），用于识别图框块。
fn frame_dir_stems() -> Vec<String> {
    frame_files()
        .map(|items| items.into_iter().map(|i| i.label).collect())
        .unwrap_or_default()
}

/// `frame/` 目录里的图框清单（按标签排序）；错误带人话（给命令提示用）。
fn frame_files() -> Result<Vec<FrameItem>, String> {
    let dir = frame_dir();
    let entries = std::fs::read_dir(&dir).map_err(|_| {
        crate::i18n::t_fmt(
            "cmd.frame.err.folder_missing",
            &[("dir", &dir.display().to_string())],
        )
    })?;
    let mut frames: Vec<FrameItem> = entries
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x.eq_ignore_ascii_case("dwg"))
                .unwrap_or(false)
        })
        .filter_map(|e| {
            let path = e.path();
            let label = path.file_stem()?.to_string_lossy().into_owned();
            Some(FrameItem {
                path: path.to_string_lossy().into_owned(),
                label,
            })
        })
        .collect();
    frames.sort_by(|a, b| a.label.cmp(&b.label));
    if frames.is_empty() {
        return Err(crate::i18n::t_fmt(
            "cmd.frame.err.no_dwg",
            &[("dir", &dir.display().to_string())],
        ));
    }
    Ok(frames)
}

/// 按名字在清单里找图框：
/// * 名字为空 → 只有**唯一**一个图框时用它；
/// * 否则先**精确**比 label（大小写不敏感，可带可省 `.dwg`），
/// * 再唯一**包含**匹配（例 `landscape` → `a3_landscape`）；多个候选 → 报错列出。
fn resolve_frame(frames: &[FrameItem], name: &str) -> Result<FrameItem, String> {
    let want = name.trim();
    let want = want.strip_suffix(".dwg").or_else(|| want.strip_suffix(".DWG")).unwrap_or(want);
    if want.is_empty() {
        return match frames {
            [only] => Ok(only.clone()),
            _ => Err(crate::i18n::t_fmt(
                "cmd.frame.err.need_name",
                &[
                    ("n", &frames.len().to_string()),
                    (
                        "names",
                        &frames
                            .iter()
                            .map(|f| f.label.as_str())
                            .collect::<Vec<_>>()
                            .join(" / "),
                    ),
                ],
            )),
        };
    }
    if let Some(hit) = frames
        .iter()
        .find(|f| f.label.eq_ignore_ascii_case(want))
    {
        return Ok(hit.clone());
    }
    let lower = want.to_lowercase();
    let hits: Vec<&FrameItem> = frames
        .iter()
        .filter(|f| f.label.to_lowercase().contains(&lower))
        .collect();
    match hits.as_slice() {
        [only] => Ok((*only).clone()),
        [] => Err(crate::i18n::t_fmt(
            "cmd.frame.err.name_missing",
            &[
                ("want", want),
                (
                    "names",
                    &frames
                        .iter()
                        .map(|f| f.label.as_str())
                        .collect::<Vec<_>>()
                        .join(" / "),
                ),
            ],
        )),
        many => Err(crate::i18n::t_fmt(
            "cmd.frame.err.name_ambiguous",
            &[
                ("want", want),
                (
                    "names",
                    &many
                        .iter()
                        .map(|f| f.label.as_str())
                        .collect::<Vec<_>>()
                        .join(" / "),
                ),
            ],
        )),
    }
}

/// `TF` 的参数（AI/自动化侧直插图框）。
#[derive(Debug, Clone, PartialEq)]
struct FrameArgs {
    /// 图框名（`frame/` 里的 dwg 文件名，可省 `.dwg`；空 = 目录里唯一的那个）。
    name: String,
    /// 比例前项（图）与后项（物）：`1:2` → 1 / 2（缩放 2 倍）。
    v1: i64,
    v2: i64,
    /// 基点（`at x,y`；省略 = 进交互放置）。
    at: Option<[f64; 3]>,
    /// 旋转角度（度）。
    rot_deg: f64,
}

/// 去掉小数尾巴的显示（`2.00` → `2`、`1.5` → `1.5`）。
fn trim_num(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// 解析 `TF` 的 rest：
///
/// ```text
/// ""                          → Ok(None)（人侧：弹选择窗口）
/// "<名>"                      → 1:1 + 交互放置
/// "<名> 1:2"                  → 指比例 + 交互放置
/// "<名> 1:2 at 0,0"           → 全自动直插
/// "<名> 1:2 at 0,0 rot 90"    → 带旋转
/// "1:2 at 0,0"                → 只给比例（图框取目录里唯一的）
/// ```
fn parse_frame_args(rest: &str) -> Result<Option<FrameArgs>, String> {
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.is_empty() {
        return Ok(None);
    }
    // 双语竖切（用户 2026-09-27）：用法串与「参数认不出」走单一 catalog；
    // 其余错误文案仍待按域录入（见 handbook 24 与本文件顶部的 i18n 模块）。
    let usage = crate::i18n::t("cmd.tf.usage");
    let mut args = FrameArgs {
        name: String::new(),
        v1: 1,
        v2: 1,
        at: None,
        rot_deg: 0.0,
    };
    let mut scale_seen = false;
    let mut i = 0;
    while i < tokens.len() {
        let tok = tokens[i];
        let lower = tok.to_ascii_lowercase();
        match lower.as_str() {
            "at" | "@" => {
                if args.at.is_some() {
                    return Err(crate::i18n::t_fmt(
                        "cmd.tf.err.at_twice",
                        &[("usage", &usage)],
                    ));
                }
                let Some(next) = tokens.get(i + 1) else {
                    return Err(crate::i18n::t_fmt(
                        "cmd.tf.err.at_needs_coords",
                        &[("usage", &usage)],
                    ));
                };
                args.at = Some(parse_point(next)?);
                i += 2;
            }
            "rot" | "rotation" => {
                let Some(next) = tokens.get(i + 1) else {
                    return Err(crate::i18n::t_fmt(
                        "cmd.tf.err.rot_needs_angle",
                        &[("usage", &usage)],
                    ));
                };
                let deg: f64 = next.parse().map_err(|_| {
                    crate::i18n::t_fmt(
                        "cmd.tf.err.rot_not_number",
                        &[("next", next), ("usage", &usage)],
                    )
                })?;
                args.rot_deg = normalize_deg(deg);
                i += 2;
            }
            _ => {
                if tok.contains(',') {
                    if args.at.is_some() {
                        return Err(crate::i18n::t_fmt(
                            "cmd.tf.err.at_twice",
                            &[("usage", &usage)],
                        ));
                    }
                    args.at = Some(parse_point(tok)?);
                } else if looks_like_scale(tok) {
                    if scale_seen {
                        return Err(crate::i18n::t_fmt(
                            "cmd.tf.err.scale_twice",
                            &[("usage", &usage)],
                        ));
                    }
                    let (v1, v2) = parse_scale_token(tok)?;
                    args.v1 = v1;
                    args.v2 = v2;
                    scale_seen = true;
                } else if args.name.is_empty() {
                    args.name = tok.to_string();
                } else {
                    return Err(crate::i18n::t_fmt(
                        "cmd.tf.err.unknown_param",
                        &[("arg", tok), ("usage", &usage)],
                    ));
                }
                i += 1;
            }
        }
    }
    if args.v1 <= 0 || args.v2 <= 0 || (args.v1 != 1 && args.v2 != 1) {
        return Err(crate::i18n::t("cmd.tf.err.scale_invalid"));
    }
    Ok(Some(args))
}

/// `x,y` / `x,y,z`。
fn parse_point(tok: &str) -> Result<[f64; 3], String> {
    let parts: Vec<&str> = tok.split(',').map(|s| s.trim()).collect();
    if !(2..=3).contains(&parts.len()) {
        return Err(crate::i18n::t_fmt(
            "cmd.tf.err.point_form",
            &[("tok", tok)],
        ));
    }
    let mut out = [0.0f64; 3];
    for (i, p) in parts.iter().enumerate() {
        out[i] = p
            .parse::<f64>()
            .map_err(|_| crate::i18n::t_fmt("cmd.tf.err.point_not_number", &[("tok", tok), ("p", p)]))?;
    }
    Ok(out)
}

fn normalize_deg(deg: f64) -> f64 {
    let mut d = deg % 360.0;
    if d >= 180.0 {
        d -= 360.0;
    } else if d < -180.0 {
        d += 360.0;
    }
    d
}

/// 像比例吗？（`1:2` / `1：2` / `1/2` / 光数字 `2` / `0.5`）
fn looks_like_scale(tok: &str) -> bool {
    tok.contains(':') || tok.contains('：') || tok.contains('/') || tok.parse::<f64>().is_ok()
}

/// 解析比例：
/// * `a:b`、`a：b`、`a/b` → (a, b)
/// * 光数字 `n`：`n >= 1` → (1, n)（缩小 n 倍）；`0 < n < 1` → (1/n, 1)（放大）。
fn parse_scale_token(tok: &str) -> Result<(i64, i64), String> {
    let norm = tok.replace('：', ":").replace('/', ":");
    if norm.contains(':') {
        let (a, b) = norm
            .split_once(':')
            .ok_or_else(|| crate::i18n::t_fmt("cmd.tf.err.scale_form", &[("tok", tok)]))?;
        return Ok((parse_scale_num(a, tok)?, parse_scale_num(b, tok)?));
    }
    let n: f64 = tok
        .parse()
        .map_err(|_| crate::i18n::t_fmt("cmd.tf.err.scale_not_number", &[("tok", tok)]))?;
    if !(n.is_finite() && n > 0.0) {
        return Err(crate::i18n::t_fmt(
            "cmd.tf.err.scale_not_positive",
            &[("tok", tok)],
        ));
    }
    let (v1, v2) = if n >= 1.0 { (1.0, n) } else { (1.0 / n, 1.0) };
    Ok((
        parse_scale_num(&format!("{}", v1.round()), tok)?,
        parse_scale_num(&format!("{}", v2.round()), tok)?,
    ))
}

fn parse_scale_num(s: &str, whole: &str) -> Result<i64, String> {
    let v: f64 = s
        .trim()
        .parse()
        .map_err(|_| {
            crate::i18n::t_fmt(
                "cmd.tf.err.scale_part_not_number",
                &[("whole", whole), ("s", s)],
            )
        })?;
    if (v - v.round()).abs() > 1e-9 {
        return Err(crate::i18n::t_fmt(
            "cmd.tf.err.scale_not_integer_ratio",
            &[("whole", whole)],
        ));
    }
    Ok(v.round() as i64)
}

/// 建图框块引用：按比例缩放 + 填属性（`比例`/`SCALE` 填 `v1:v2`，其余用 ATTDEF 默认值）+ 旋转。
fn build_frame_insert(
    block_name: &str,
    scale: f64,
    scale_text: &str,
    attdefs: &[acadrust::entities::AttributeDefinition],
    base: [f64; 3],
    rot_deg: f64,
) -> acadrust::entities::Insert {
    use acadrust::entities::{AttributeEntity, Entity, HorizontalAlignment, Insert, VerticalAlignment};
    use acadrust::types::Vector3;

    let mut ins = Insert::new(block_name, Vector3::new(base[0], base[1], base[2]));
    ins.set_x_scale(scale);
    ins.set_y_scale(scale);
    ins.set_z_scale(scale);
    ins.rotation = rot_deg.to_radians();
    let xform = ins.get_transform();
    for ad in attdefs {
        let is_scale =
            ad.tag.eq_ignore_ascii_case("比例") || ad.tag.eq_ignore_ascii_case("SCALE");
        let value = if is_scale {
            scale_text.to_string()
        } else {
            ad.default_value.clone()
        };
        let mut attr = AttributeEntity::from_definition(ad, Some(value));
        // 有的导出器把 halign/valign 非 0 的 ATTDEF 的 11 组（对齐点）写空，
        // 属性于是按 (0,0) 定位：文字落到块原点，标题栏看起来是空的、
        // 「图样代号」按左对齐横穿图框。DXF 语义里对齐点缺失时文本按**插入点**
        // 定位，故此处退回插入点——保留对齐语义，不清零（用户 2026-09-28 清单 §P2.1）。
        // 左/基线（默认对齐）的 ATTDEF 不做替换：它们的对齐点本来就该是空的，
        // 按插入点排字才是正常做法。
        let alignment_lost = attr.alignment_point.x == 0.0
            && attr.alignment_point.y == 0.0
            && attr.alignment_point.z == 0.0;
        let aligned_by_point = !matches!(ad.horizontal_alignment, HorizontalAlignment::Left)
            || !matches!(ad.vertical_alignment, VerticalAlignment::Baseline);
        if alignment_lost && aligned_by_point {
            attr.alignment_point = attr.insertion_point;
        }
        attr.apply_transform(&xform);
        ins.attributes.push(attr);
    }
    ins
}

// ── 标准件库（parts/）═════════════════════════════════════════════════════
//
// 【已作废 · 2026-09-17 标注】下面这组 `*_scan` / `PartEntry` 是 2026-09-14 前
// “插件直读 DWG 标准件库（ACM 代理图形解码）”路线的遗留（曾预留给二期序号/三期明细表）。
// 用户 2026-09-14 定案（`OCSMBOM-plan.md` §16）**标准件全部走参数化**，本路线不再使用；
// 现行实现见 `src/partgen*.rs`（`catalog_json` + `OCSMPART`）与 `src/tables/*.json`。
// 保留代码仅作历史/对比参考，**不要接线**；要清就整块删（含其单测）。
// 库契约见 `OCSMBOM-plan.md` §2：每个标准件一个 DWG（含 ATTDEF +
// 正确基点），可选一份 `catalog.csv` 索引。此处只负责"列出库里有什么"，
// 真正的块导入/属性写入由命令路径完成（见 §9 实施清单 2~5 步）。

/// 标准件库里的一项。
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // 二期（序号）/三期（明细表）接线后用满
pub(crate) struct PartEntry {
    /// 类别（如 `bolt` / `nut` / `washer`）。
    pub category: String,
    /// 标准号（如 `GB/T 5782`）。
    pub std: String,
    /// 名称（如 `六角头螺栓`）；无索引时回退为文件名主干。
    pub name: String,
    /// 规格（如 `M10x40`）。
    pub spec: String,
    /// 材料（索引可选）。
    pub material: String,
    /// 单件重量（索引可选，文本，保留库里的写法）。
    pub weight: String,
    /// DWG 绝对路径。
    pub path: String,
}

/// 标准件文件夹：优先 `OCSM_PARTS_DIR` 环境变量（测试/排障），否则为插件
/// 安装目录下的 `parts/`。与 `frame_dir()` 同一套路。
#[allow(dead_code)]
fn parts_dir() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("OCSM_PARTS_DIR") {
        let dir = dir.trim();
        if !dir.is_empty() {
            return std::path::PathBuf::from(dir);
        }
    }
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "--ocs-plugin-runner" {
            if let (Some(_socket), Some(lib)) = (args.next(), args.next()) {
                let p = std::path::PathBuf::from(lib);
                if let Some(dir) = p.parent() {
                    return dir.join("parts");
                }
            }
        }
    }
    std::path::PathBuf::from("parts")
}

/// 文件名兜底解析：`<类别>_<标准号>_<规格>.dwg` → `(类别, 标准号, 规格)`。
/// 段数不足或为空时该位留空（不报错：库文件不规范也要能列出来）。
#[allow(dead_code)]
fn parse_part_stem(stem: &str) -> (String, String, String) {
    let mut it = stem.split('_').map(|x| x.trim());
    let category = it.next().unwrap_or_default().to_string();
    let std = it.next().unwrap_or_default().to_string();
    // 规格里允许出现 `_`（如 `M10_40`）→ 剩余部分全部接回，保真。
    let spec = it.collect::<Vec<_>>().join("_");
    (category, std, spec)
}

/// 解析 `catalog.csv` 文本。列序：`类别,标准号,名称,规格,材料,单重,文件名`。
/// - 首行若以「类别」开头视为表头跳过；`#` 开头为注释；空行忽略。
/// - 列数不足的行走"能填多少填多少"（不丢弃，避免库文件一点点不规范就消失）。
#[allow(dead_code)]
fn parse_catalog_csv(text: &str) -> Vec<(String, String, String, String, String, String, String)> {
    let mut rows = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<String> = line
            .split(',')
            .map(|c| c.trim().trim_matches('"').to_string())
            .collect();
        if idx == 0 && cols.first().map(|c| c.as_str()) == Some("类别") {
            continue; // 表头
        }
        let get = |i: usize| cols.get(i).cloned().unwrap_or_default();
        let file = get(6);
        if file.is_empty() {
            continue; // 没有文件名 → 无法插入，跳过
        }
        rows.push((
            get(0),
            get(1),
            get(2),
            get(3),
            get(4),
            get(5),
            file,
        ));
    }
    rows
}

/// 扫描指定目录（纯函数，便于测试；`parts_scan()` 传 `parts_dir()`）。
/// 优先用 `catalog.csv`（只保留实际存在的 DWG）；没有索引则扫 `*.dwg`
/// 并用文件名兜底解析。
#[allow(dead_code)]
fn parts_scan_in(dir: &std::path::Path) -> Vec<PartEntry> {
    let dwg_names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|x| x.to_str())
                        .map(|x| x.eq_ignore_ascii_case("dwg"))
                        .unwrap_or(false)
                })
                .filter_map(|e| e.path().file_name().map(|s| s.to_string_lossy().into_owned()))
                .collect()
        })
        .unwrap_or_default();

    let mut out: Vec<PartEntry> = Vec::new();

    if let Ok(text) = std::fs::read_to_string(dir.join("catalog.csv")) {
        for (category, std, name, spec, material, weight, file) in parse_catalog_csv(&text) {
            // 文件名对不上（大小写不敏感）时按原样试拼，仍不存在则跳过。
            let matched = dwg_names
                .iter()
                .find(|n| n.eq_ignore_ascii_case(&file))
                .cloned()
                .unwrap_or_else(|| file.clone());
            let path = dir.join(&matched);
            if !path.exists() {
                continue;
            }
            out.push(PartEntry {
                category,
                std,
                name,
                spec,
                material,
                weight,
                path: path.to_string_lossy().into_owned(),
            });
        }
    } else {
        for name in &dwg_names {
            let stem = std::path::Path::new(name)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let (category, std, spec) = parse_part_stem(&stem);
            out.push(PartEntry {
                category,
                std,
                name: stem.clone(),
                spec,
                material: String::new(),
                weight: String::new(),
                path: dir.join(name).to_string_lossy().into_owned(),
            });
        }
    }

    out.sort_by(|a, b| {
        (&a.category, &a.std, &a.spec, &a.path).cmp(&(&b.category, &b.std, &b.spec, &b.path))
    });
    out
}

/// 扫描标准件库（`parts_dir()`）。
#[allow(dead_code)]
pub(crate) fn parts_scan() -> Vec<PartEntry> {
    parts_scan_in(&parts_dir())
}

/// 表面粗糙度交互：点选插入点 → 打开粗糙度 GUI（rough.html?x=&y=&tab=N）。
struct RoughnessPlace {
    sender: std::sync::Arc<dyn PluginRequestSender>,
    /// 打开窗口时钉住的图纸（标签页号；命令启动时记下，点选时拿不到）。
    tab: Option<u64>,
}

impl InteractiveCommand for RoughnessPlace {
    fn prompt(&self) -> String {
        crate::i18n::t("cmd.rough.prompt")
    }

    fn on_point(&mut self, pt: [f64; 3]) -> CommandStep {
        // 确保标注更新服务器在跑（GUIDE_PORT 进程级防重），打开 GUI。
        // 点选回调里**不能**发宿主请求（会与主线程死锁）→ 只换默认目标
        // （该标签页的登记在命令 dispatch 时已做好）。
        let sender = self.sender.clone();
        let Some(port) = ensure_guide_server_running(sender, None) else {
            return CommandStep::Cancel;
        };
        open_plugin_page(
            port,
            &with_tab(&format!("/rough.html?x={}&y={}", pt[0], pt[1]), self.tab),
            "rough",
            900,
            880,
        );
        CommandStep::Cancel
    }
}

/// 标准件插入：点选插入点 → 记下点 + 打开选择器页（族/规格/视图由页面挑）。
/// 与粗糙度同构：点选回调里**不能**发宿主请求，写图统一由页面 `/api/part_pick` 触发。
struct PartPlace {
    sender: std::sync::Arc<dyn PluginRequestSender>,
    /// 放置阶段（`Cell`：prompt 只用 &self）
    phase: std::cell::Cell<PlacePhase>,
    /// 已定位的基点
    base: std::cell::Cell<[f64; 3]>,
    /// 人看的名字（“OCSM 标准件” / “OCSM 齿轮”）——只影响提示语
    what: String,
    /// 去哪里出图（“请在零件库窗口里点「零件出库」” / “请在齿轮窗口里点「生成到图纸」”）
    where_to: String,
}

/// 放置阶段：`Follow` = 零件跟光标（未定位基点）；`Rotate` = 已定位基点、跟随光标绕基点旋转。
#[derive(Clone, Copy, PartialEq, Eq)]
enum PlacePhase {
    Follow,
    Rotate,
}

impl InteractiveCommand for PartPlace {
    fn prompt(&self) -> String {
        let (what, where_to) = (&self.what, &self.where_to);
        match (pending_part_label(), self.phase.get()) {
            (None, _) => crate::i18n::t_fmt(
                "cmd.place.prompt.wait",
                &[("what", what), ("where_to", where_to)],
            ),
            (Some(l), PlacePhase::Follow) => crate::i18n::t_fmt(
                "cmd.place.prompt.follow",
                &[("what", what), ("l", &l)],
            ),
            (Some(l), PlacePhase::Rotate) => crate::i18n::t_fmt(
                "cmd.place.prompt.rotate",
                &[("what", what), ("l", &l)],
            ),
        }
    }

    /// 光标跟随预览（与 POWERDIM 同机制）：定位前跟光标走，定位后绕基点旋转。
    fn wants_mouse_move(&self) -> bool {
        true
    }

    fn on_mouse_move(&mut self, pt: [f64; 3]) -> Option<acadrust::EntityType> {
        let block = pending_block()?;
        let (at, rot) = match self.phase.get() {
            PlacePhase::Follow => (pt, 0.0),
            PlacePhase::Rotate => (self.base.get(), rotate_angle(self.base.get(), pt)),
        };
        let mut ins = acadrust::entities::Insert::new(&block, Vector3::new(at[0], at[1], at[2]));
        ins.rotation = rot;
        {
            let c = &mut ins.common;
            c.layer = partgen::LAYER_MAIN.to_string();
            c.color = Color::ByLayer;
            c.linetype = "ByLayer".to_string();
            c.line_weight = LineWeight::ByLayer;
        }
        Some(acadrust::EntityType::Insert(ins))
    }

    fn on_point(&mut self, pt: [f64; 3]) -> CommandStep {
        if pending_block().is_none() {
            // 还没出库：只进入放置态等待（兼容旧的"先点后选"路径）
            *parts_point_slot().lock().unwrap() = Some(pt);
            return CommandStep::NeedPoint;
        }
        match self.phase.get() {
            // 第一下：定位基点，转入旋转阶段
            PlacePhase::Follow => {
                self.base.set(pt);
                self.phase.set(PlacePhase::Rotate);
                CommandStep::NeedPoint
            }
            // 第二下：落定（交给放置 worker 发宿主请求：INSERT + OCSM_PART + 标脏）
            PlacePhase::Rotate => {
                let task = PlaceTask {
                    pt: self.base.get(),
                    rotation: rotate_angle(self.base.get(), pt),
                    sender: self.sender.clone(),
                };
                let _ = place_sender().send(task);
                self.phase.set(PlacePhase::Follow);
                CommandStep::NeedPoint
            }
        }
    }
}

/// 绕基点旋转角（弧度）：基点 → 光标方向；光标离基点太近（<1 mm）视为 0，避免抖动。
fn rotate_angle(base: [f64; 3], pt: [f64; 3]) -> f64 {
    let (dx, dy) = (pt[0] - base[0], pt[1] - base[1]);
    if (dx * dx + dy * dy).sqrt() < 1.0 {
        0.0
    } else {
        dy.atan2(dx)
    }
}

/// 交互插入：点击给出插入点 → 构造带属性的 Insert 交给宿主提交。
struct FramePlace {
    block_name: String,
    scale: f64,
    scale_text: String,
    attdefs: Vec<acadrust::entities::AttributeDefinition>,
}

impl InteractiveCommand for FramePlace {
    fn prompt(&self) -> String {
        crate::i18n::t_fmt(
            "cmd.frame.prompt.specify_point",
            &[("block", &self.block_name), ("scale", &self.scale_text)],
        )
    }

    fn on_point(&mut self, pt: [f64; 3]) -> CommandStep {
        let ins = build_frame_insert(
            &self.block_name,
            self.scale,
            &self.scale_text,
            &self.attdefs,
            pt,
            0.0,
        );
        CommandStep::CommitAndEnd(acadrust::EntityType::Insert(ins))
    }
}

export_plugin!(OcsmPlugin);

#[cfg(test)]
mod tests {
    // ── 功能区注册（按钮 ↔ 命令前缀两处同步） ─────────────────────
    #[test]
    fn ribbon_registers_shaft_button_and_command_prefixes_match_manifest() {
        // 按钮口径与「齿轮」一致：id = 命令名、label = 显示名、event = Command(id)；
        // 点击即 `OCSMSHAFT`（不带参数 = 开轴生成器窗口 + 放置态，不是直插）。
        let module = OcsmPlugin.ribbon();
        let mut found = false;
        for group in module.ribbon_groups() {
            for item in &group.tools {
                if let RibbonItem::LargeTool(t) = item {
                    if t.id == "OCSMSHAFT" {
                        assert_eq!(group.title, "轴");
                        assert_eq!(t.label, "轴生成器");
                        assert!(matches!(
                            &t.event,
                            ModuleEvent::Command(c) if c == "OCSMSHAFT"
                        ));
                        found = true;
                    }
                }
            }
        }
        assert!(found, "功能区缺少「轴生成器」按钮（OCSMSHAFT）");
        assert!(
            MANIFEST.command_prefixes.contains(&"OCSMSHAFT"),
            "MANIFEST.command_prefixes 缺少 OCSMSHAFT"
        );
        assert!(
            MANIFEST.command_prefixes.contains(&"OCSMCARD"),
            "MANIFEST.command_prefixes 缺少 OCSMCARD（智能卡片）"
        );
        assert!(
            !MANIFEST.command_prefixes.contains(&"XLT"),
            "XLT 已移除（用户定名收敛：OCSMCARD）——不该再注册"
        );

        // src/lib.rs 的 MANIFEST 与 plugin.toml 是两处维护（部署时 sed 只替换
        // rustc/acadrust 占位符）——命令前缀必须两处一致，否则会出现
        // “命令能用但按钮不显示 / 按钮点了没命令”的不对称。
        let toml = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/plugin.toml"))
            .expect("读 plugin.toml");
        let prefix_line = toml
            .lines()
            .find(|l| l.trim_start().starts_with("command_prefixes"))
            .expect("plugin.toml 缺 command_prefixes");
        let list = prefix_line
            .split_once('[')
            .and_then(|(_, r)| r.split_once(']'))
            .map(|(l, _)| l)
            .expect("command_prefixes 不是数组");
        let mut in_toml: Vec<String> = list
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let mut expected: Vec<String> = MANIFEST
            .command_prefixes
            .iter()
            .map(|s| s.to_string())
            .collect();
        in_toml.sort();
        expected.sort();
        assert_eq!(
            in_toml, expected,
            "plugin.toml 与 MANIFEST.command_prefixes 不同步"
        );
    }

    /// 版本**三处同源**（用户 2026-09-27）：`plugin.toml` 的 `[plugin] version`（宿主
    /// 显示口径 = 0.2.0）、`Cargo.toml` 的 `[package] version`、`MANIFEST.version`。
    /// 后两者本就是一处（`env!("CARGO_PKG_VERSION")`），这里把与 plugin.toml 的那处
    /// 漂移变成**测试失败**（此前 0.1.0 / 0.2.0 不一致静默存在）。
    #[test]
    fn plugin_toml_version_matches_cargo_pkg_version() {
        let toml = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/plugin.toml"))
            .expect("读 plugin.toml");
        // 取 [plugin] 段里的 version（`[opencad]` 段没有 version，但要防以后加）
        let mut in_plugin_section = false;
        let mut toml_version: Option<String> = None;
        for line in toml.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_plugin_section = line == "[plugin]";
                continue;
            }
            if in_plugin_section {
                if let Some(rest) = line.strip_prefix("version") {
                    toml_version = rest
                        .trim_start()
                        .strip_prefix('=')
                        .map(|v| v.trim().trim_matches('"').to_string());
                    break;
                }
            }
        }
        let toml_version = toml_version.expect("plugin.toml 的 [plugin] 段缺 version");
        assert_eq!(
            toml_version,
            env!("CARGO_PKG_VERSION"),
            "plugin.toml 的 [plugin] version 与 Cargo.toml 的 version 不一致"
        );
        assert_eq!(
            MANIFEST.version,
            env!("CARGO_PKG_VERSION"),
            "MANIFEST.version 应走 env!(CARGO_PKG_VERSION)"
        );
    }

    /// 定名收敛（用户 2026-09-25）：旧短命令 `XLT` 已移除 —— 调用时要**明确报错并指路
    /// `OCSMCARD`**（不静默），卡类型表能查到（命令入口的表驱动基础）。
    #[test]
    fn xlt_removed_and_points_to_ocsmcard() {
        let _g = global_state_test_lock(); // dispatch 会 refresh_doc_save_state（进程级全局）
        let mut host = UndoOrderSpy::default();
        assert!(OcsmPlugin.dispatch(
            &mut host,
            "XLT 内 6H SPLINE IN M3 Z20 ALPHA30 X0 DA64.5 DF57.3436 BETA0 H30"
        ));
        assert!(
            host.errors.iter().any(|e| e.contains("OCSMCARD")),
            "XLT 应报错并指路 OCSMCARD：{:?}",
            host.errors
        );
        assert!(crate::card::card_type_by_token("花键参数表").is_some());
        assert!(crate::card::card_type_by_token("spline").is_some());
    }

    #[test]
    fn ribbon_tool_icons_are_two_colour_svg() {
        // 所有功能区按钮的图标必须是内嵌的双色 SVG（主 #B4B6B9 / 强调 #6DB7ED），
        // 不允许回退到 emoji/字形：字形无法双色，且在 Web 构建的 Fira Sans 下会变豆腐块。
        let module = OcsmPlugin.ribbon();
        let mut checked = 0usize;
        for group in module.ribbon_groups() {
            for item in &group.tools {
                if let RibbonItem::LargeTool(t) = item {
                    match t.icon {
                        IconKind::Svg(bytes) => {
                            assert!(!bytes.is_empty(), "{} 图标字节为空", t.id);
                            let text = std::str::from_utf8(bytes)
                                .unwrap_or_else(|_| panic!("{} 图标不是 UTF-8 SVG", t.id));
                            assert!(
                                text.contains("viewBox=\"0 0 24 24\""),
                                "{} 图标不是 24×24 网格",
                                t.id
                            );
                            assert!(
                                text.contains("#B4B6B9") && text.contains("#6DB7ED"),
                                "{} 图标缺主色/强调色 token",
                                t.id
                            );
                        }
                        IconKind::Glyph(g) => {
                            panic!("{} 仍是字形图标（{g}），应为双色 SVG", t.id)
                        }
                    }
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 10, "功能区应有 10 个 LargeTool 按钮（当前 {checked}）");
    }

    /// XL/`OCSMPART` 按钮：点击（不带参数）开「OCSM 标准件库」窗口 + 放置态。
    /// id/label/组/事件 + 图标非空都锁住，避免以后再出现「命令能用但按钮不见了」。
    #[test]
    fn ribbon_registers_parts_button_open_library() {
        let module = OcsmPlugin.ribbon();
        let mut found = false;
        for group in module.ribbon_groups() {
            for item in &group.tools {
                if let RibbonItem::LargeTool(t) = item {
                    if t.id == "OCSMPART" {
                        assert_eq!(group.title, "标准件");
                        assert_eq!(t.label, "标准件库");
                        assert!(matches!(
                            &t.event,
                            ModuleEvent::Command(c) if c == "OCSMPART"
                        ));
                        match t.icon {
                            IconKind::Svg(bytes) => assert!(!bytes.is_empty()),
                            IconKind::Glyph(g) => panic!("标准件库按钮仍是字形图标（{g}）"),
                        }
                        found = true;
                    }
                }
            }
        }
        assert!(found, "功能区缺少「标准件库」按钮（OCSMPART / XL）");
        assert!(
            MANIFEST.command_prefixes.contains(&"OCSMPART")
                && MANIFEST.command_prefixes.contains(&"XL"),
            "MANIFEST.command_prefixes 缺少 OCSMPART/XL"
        );
    }

    /// `OCSMCARD` 工具栏按钮（⑥）：卡片图标 + 分组 + 事件；三处同步（MANIFEST / plugin.toml /
    /// COMMAND_CATALOG）由既有断言锁定。
    #[test]
    fn ribbon_registers_card_button_open_card_gui() {
        let module = OcsmPlugin.ribbon();
        let mut found = false;
        let mut groups: Vec<&str> = Vec::new();
        for group in module.ribbon_groups() {
            groups.push(group.title);
            for item in &group.tools {
                if let RibbonItem::LargeTool(t) = item {
                    if t.id == "OCSMCARD" {
                        assert_eq!(group.title, "卡片");
                        assert_eq!(t.label, "智能卡片");
                        assert!(matches!(
                            &t.event,
                            ModuleEvent::Command(c) if c == "OCSMCARD"
                        ));
                        match t.icon {
                            IconKind::Svg(bytes) => {
                                let svg = std::str::from_utf8(bytes).expect("SVG UTF-8");
                                assert!(svg.contains("viewBox=\"0 0 24 24\""));
                                assert!(svg.contains("#B4B6B9") && svg.contains("#6DB7ED"));
                            }
                            IconKind::Glyph(g) => panic!("智能卡片按钮仍是字形图标（{g}）"),
                        }
                        found = true;
                    }
                }
            }
        }
        assert!(found, "功能区缺少「智能卡片」按钮（OCSMCARD）");
        // 位置：靠近标准件/标注区（标准件之后、标注之前）。
        let ip = groups.iter().position(|g| *g == "标准件");
        let ic = groups.iter().position(|g| *g == "卡片");
        let ia = groups.iter().position(|g| *g == "标注");
        if let (Some(p), Some(c), Some(a)) = (ip, ic, ia) {
            assert!(p < c && c < a, "卡片组应在标准件与标注之间：{groups:?}");
        }
        assert!(
            MANIFEST.command_prefixes.contains(&"OCSMCARD"),
            "MANIFEST.command_prefixes 缺少 OCSMCARD"
        );
    }

    // ── 标准件库扫描 ──────────────────────────────────────────────
    #[test]
    fn parse_part_stem_splits_three_parts() {
        assert_eq!(
            parse_part_stem("bolt_GBT5782_M10x40"),
            ("bolt".to_string(), "GBT5782".to_string(), "M10x40".to_string())
        );
        // 规格里含 `_` → 剩余段全部接回（保真）。
        assert_eq!(
            parse_part_stem("bearing_GBT276_M10_40"),
            ("bearing".to_string(), "GBT276".to_string(), "M10_40".to_string())
        );
        // 段数不足：缺的空着，不报错。
        assert_eq!(
            parse_part_stem("washer"),
            ("washer".to_string(), String::new(), String::new())
        );
    }

    #[test]
    #[test]
    fn parts_spec_parses_parameterized_form() {
        assert_eq!(PartsSpec::parse(""), None);
        assert_eq!(PartsSpec::parse("hex_bolt_c"), None);
        assert_eq!(PartsSpec::parse("hex_bolt_c 10"), None);
        let spec = PartsSpec::parse("HEX_BOLT_C 10 95").unwrap();
        assert_eq!(spec.family, "hex_bolt_c");
        assert_eq!((spec.d, spec.l), (10.0, 95.0));
        assert_eq!(spec.view, "main");
        assert_eq!(spec.at, None);
        assert_eq!(spec.rotation, None);
        assert_eq!(spec.b1, None);

        // 结构要素：没有长度 l，可选 b1（`b1 <值>`），其余关键字同标准件。
        let spec = PartsSpec::parse("detail_grind_od 100").unwrap();
        assert_eq!(spec.family, "detail_grind_od");
        assert_eq!((spec.d, spec.l), (100.0, 0.0));
        assert_eq!(spec.b1, None);
        let spec = PartsSpec::parse("DETAIL_GRIND_OD 100 b1 8 at 150,30 rot 45").unwrap();
        assert_eq!(spec.b1, Some(8.0));
        assert_eq!(spec.at, Some([150.0, 30.0]));
        assert_eq!(spec.rotation, Some(45.0));
        // 结构要素不接受第二个数字当长度（必须显式 b1），也不接受标准件里的 b1。
        assert_eq!(PartsSpec::parse("detail_grind_od 100 10"), None);
        assert_eq!(PartsSpec::parse("hex_bolt_c 10 95 b1 3"), None);
        // to_body：结构要素不带 l、带 b1。
        let body: serde_json::Value = serde_json::from_str(
            &PartsSpec::parse("detail_grind_od 100 b1 8 at 1,2 rot 45")
                .unwrap()
                .to_body(),
        )
        .unwrap();
        assert_eq!(body["family"], "detail_grind_od");
        assert!(body.get("l").is_none(), "结构要素请求体不应带 l：{body}");
        assert_eq!(body["b1"], 8.0);
        assert_eq!(body["x"], 1.0);
        assert_eq!(body["y"], 2.0);

        // 外螺纹退刀槽：P 必给（通用 params），g1/g2/dg/r/alpha 可选；键大小写不敏感
        let spec = PartsSpec::parse("DETAIL_THREAD_RELIEF 20 P 1.5").unwrap();
        assert_eq!(spec.family, "detail_thread_relief");
        assert_eq!(spec.params.get("p"), Some(&1.5));
        assert_eq!(spec.b1, None);
        let spec = PartsSpec::parse(
            "detail_thread_relief 20 P 1.5 g1 2.5 g2 4.5 dg 17.7 r 0.8 alpha 30 at 10,20 rot 15",
        )
        .unwrap();
        assert_eq!(spec.params.len(), 6);
        assert_eq!(spec.params.get("alpha"), Some(&30.0));
        assert_eq!(spec.at, Some([10.0, 20.0]));
        assert_eq!(spec.rotation, Some(15.0));
        let body: serde_json::Value = serde_json::from_str(&spec.to_body()).unwrap();
        assert_eq!(body["family"], "detail_thread_relief");
        assert_eq!(body["params"]["p"], 1.5);
        assert_eq!(body["params"]["g1"], 2.5);
        assert!(body.get("l").is_none(), "结构要素请求体不应带 l：{body}");
        // 参数关键字只对结构要素开放：标准件写了 P 仍回退 GUI（不误吞）
        assert_eq!(PartsSpec::parse("hex_bolt_c 10 95 p 1.5"), None);

        let spec = PartsSpec::parse("hex_bolt_c 10 95 view top at 150,30 rot 90").unwrap();
        assert_eq!(spec.view, "top");
        assert_eq!(spec.at, Some([150.0, 30.0]));
        assert_eq!(spec.rotation, Some(90.0));
        // 空白分隔的落点同样收。
        assert_eq!(
            PartsSpec::parse("nut_c41 10 9.5 at 150 30").unwrap().at,
            Some([150.0, 30.0])
        );
        // 无法识别的尾巴 → 回退 GUI 流程。
        assert_eq!(PartsSpec::parse("hex_bolt_c 10 95 嗯"), None);
        // 宿主交给插件时可能是大写 → 关键字大小写不敏感（实测：曾经因大写
        // `AT`/`ROT` 解析失败 → 静默回退到 GUI 流程，白开一个浏览器窗口）。
        let spec = PartsSpec::parse("NUT_C41 10 9.5 AT 150,30 ROT 180").unwrap();
        assert_eq!(spec.family, "nut_c41");
        assert_eq!(spec.at, Some([150.0, 30.0]));
        assert_eq!(spec.rotation, Some(180.0));

        let body: serde_json::Value = serde_json::from_str(
            &PartsSpec::parse("hex_bolt_c 10 95 at 1,2 rot 45")
                .unwrap()
                .to_body(),
        )
        .unwrap();
        assert_eq!(body["family"], "hex_bolt_c");
        assert_eq!(body["x"], 1.0);
        assert_eq!(body["y"], 2.0);
        assert_eq!(body["rotation"], 45.0);

        // 矩形花键：第二个 token 是**规格代号**（非数字）→ 族解析出小径 d；
        // `L30` / `de63` / `view side` 都认（大小写不敏感）。
        let spec = PartsSpec::parse("DETAIL_SPLINE_RECT 6x23x26x6 L30 de63 view side").unwrap();
        assert_eq!(spec.family, "detail_spline_rect");
        assert_eq!(spec.d, 23.0);
        assert_eq!(spec.spec.as_deref(), Some("6x23x26x6"));
        assert_eq!(spec.params.get("len"), Some(&30.0));
        assert_eq!(spec.params.get("de"), Some(&63.0));
        assert_eq!(spec.view, "side");
        let body: serde_json::Value = serde_json::from_str(&spec.to_body()).unwrap();
        assert_eq!(body["spec"], "6x23x26x6");
        assert_eq!(body["d"], 23.0);
        assert_eq!(body["params"]["len"], 30.0);
        assert_eq!(body["params"]["de"], 63.0);
        // 不给 view → 花键默认侧视图（老要素仍默认 main）
        assert_eq!(
            PartsSpec::parse("detail_spline_rect 6x23x26x6 L30").unwrap().view,
            "side"
        );
        assert_eq!(PartsSpec::parse("detail_grind_od 100").unwrap().view, "main");
        // 裸数字 = L（规格代号族）：`XL detail_spline_rect 6x23x26x6 30`
        let spec = PartsSpec::parse("detail_spline_rect 6x23x26x6 30 de63").unwrap();
        assert_eq!(spec.params.get("len"), Some(&30.0));
        assert_eq!(spec.params.get("de"), Some(&63.0));
        assert_eq!(spec.d, 23.0);
        // 贴写 / 空格混用 + `de` 不写值走查表
        let spec = PartsSpec::parse("detail_spline_rect 6×23×26×6 L 30 de 71").unwrap();
        assert_eq!(spec.d, 23.0);
        assert_eq!(spec.params.get("len"), Some(&30.0));
        assert_eq!(spec.params.get("de"), Some(&71.0));
        // 数字参数路线：`d23 N6 D26 B6 L30`（不依赖规格代号）
        let spec = PartsSpec::parse("detail_spline_rect 23 N6 D26 B6 L30").unwrap();
        assert_eq!(spec.d, 23.0);
        assert!(spec.spec.is_none());
        assert_eq!(spec.params.get("n"), Some(&6.0));
        assert_eq!(spec.params.get("big"), Some(&26.0), "D → big");
        assert_eq!(spec.params.get("b"), Some(&6.0));
        assert_eq!(spec.params.get("len"), Some(&30.0));
        // 非结构要素的第二个 token 不是数字 → 仍回退 GUI（不误吞）
        assert_eq!(PartsSpec::parse("hex_bolt_c abc 95"), None);
        // 花键不认 b1，但解析不报错（到生成时统一报「不认识参数」）
        assert!(PartsSpec::parse("detail_spline_rect 6x23x26x6 L30 b1 3").is_some());

        // 渐开线花键的 XL 入口已移除：不再作为结构要素族解析（唯一入口 = OCSMGEAR 花键模式）。
        assert_eq!(
            PartsSpec::parse("detail_invol_spline GB30R M3 Z20 L30 view side"),
            None
        );
    }

    fn parse_catalog_csv_skips_header_comments_and_blanks() {
        let text = "类别,标准号,名称,规格,材料,单重,文件名\n\
                    # 这是注释\n\
                    \n\
                    bolt,GB/T 5782,六角头螺栓,M10x40,8.8,31.5,bolt_GBT5782_M10x40.dwg\n\
                    nut,GB/T 6170,1型六角螺母,M10,8,11,\n";
        let rows = parse_catalog_csv(text);
        assert_eq!(rows.len(), 1, "表头/注释/空行/无文件名行都应被跳过");
        assert_eq!(rows[0].0, "bolt");
        assert_eq!(rows[0].2, "六角头螺栓");
        assert_eq!(rows[0].6, "bolt_GBT5782_M10x40.dwg");
    }

    #[test]
    fn parts_scan_without_index_falls_back_to_filename() {
        let dir = std::env::temp_dir().join(format!("ocsm_parts_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp parts dir");
        std::fs::write(dir.join("bolt_GBT5782_M10x40.dwg"), b"not a real dwg").unwrap();
        std::fs::write(dir.join("readme.txt"), b"ignore me").unwrap();

        let entries = parts_scan_in(&dir);
        assert_eq!(entries.len(), 1, "非 dwg 文件必须被忽略");
        assert_eq!(entries[0].category, "bolt");
        assert_eq!(entries[0].std, "GBT5782");
        assert_eq!(entries[0].spec, "M10x40");
        assert_eq!(entries[0].name, "bolt_GBT5782_M10x40", "无索引时名称回退为文件名主干");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parts_scan_with_index_uses_catalog_and_drops_missing_files() {
        let dir = std::env::temp_dir().join(format!("ocsm_parts_idx_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp parts dir");
        std::fs::write(dir.join("bolt_GBT5782_M10x40.dwg"), b"x").unwrap();
        std::fs::write(
            dir.join("catalog.csv"),
            "类别,标准号,名称,规格,材料,单重,文件名\n\
             bolt,GB/T 5782,六角头螺栓,M10x40,8.8级,31.5,bolt_GBT5782_M10x40.dwg\n\
             nut,GB/T 6170,1型六角螺母,M10,8级,11,nut_GBT6170_M10.dwg\n",
        )
        .unwrap();

        let entries = parts_scan_in(&dir);
        assert_eq!(entries.len(), 1, "索引里缺失的 DWG 必须被跳过");
        assert_eq!(entries[0].name, "六角头螺栓", "有索引时用中文名称而非文件名");
        assert_eq!(entries[0].material, "8.8级");
        assert_eq!(entries[0].weight, "31.5");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 端口/句柄不一致时才重写链接（避免每次开图都改图纸）。
    #[test]
    fn needs_link_rewrite_only_when_stale() {
        let h = acadrust::Handle::new(0x4A);
        let good = "http://127.0.0.1:23751/guide.html?handle=0x4A";
        assert!(!needs_link_rewrite(Some(good), 23751, h), "端口与句柄都对 → 不写");
        assert!(needs_link_rewrite(Some(good), 23752, h), "端口变了 → 要写");
        assert!(
            needs_link_rewrite(Some("http://127.0.0.1:23751/guide.html?handle=0x99"), 23751, h),
            "句柄对不上（被复制过）→ 要写"
        );
        assert!(needs_link_rewrite(None, 23751, h), "没链接 → 要写");
        assert!(
            needs_link_rewrite(Some("https://example.com/x"), 23751, h),
            "无关链接 → 要写"
        );
    }

    use super::*;
    use acadrust::entities::{
        Arc, Circle, EntityType, HorizontalAlignment, Insert, Line, VerticalAlignment,
    };

    fn lw(v: i16) -> LineWeight {
        LineWeight::from_value(v)
    }

    #[test]
    fn layer_defs_match_layers_quick_template() {
        let defs = layer_defs();
        assert_eq!(defs.len(), 10);
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "1轮廓实线层",
                "2细线层",
                "3中心线层",
                "4虚线层",
                "5剖面线层",
                "6文字层",
                "7标注层",
                "8符号标注层",
                "9双点划线层",
                "10引导线层",
            ]
        );
        // 轮廓实线层 0.35mm；2~9 层 0.18mm；引导线层 0mm（不打印）。
        assert_eq!(defs[0].lineweight, lw(35));
        for d in &defs[1..9] {
            assert_eq!(d.lineweight, lw(18));
        }
        assert_eq!(defs[9].lineweight, lw(0));
        // 线型：中心线 CENTER2、虚线 DASHED2、双点划线 DIVIDE2。
        assert_eq!(defs[2].linetype, "CENTER2");
        assert_eq!(defs[3].linetype, "DASHED2");
        assert_eq!(defs[8].linetype, "DIVIDE2");
        // 符号标注层 RGB(255,191,127)。
        assert_eq!(defs[7].color, Color::from_rgb(255, 191, 127));
        // 引导线层：蓝色(5)、Continuous、默认关闭（off），其余层默认开启。
        assert_eq!(defs[9].color, Color::from_index(5));
        assert_eq!(defs[9].linetype, "Continuous");
        assert!(defs[9].plottable);
        assert!(defs[9].off);
        for d in &defs[..9] {
            assert!(!d.off, "{} 不应默认关闭", d.name);
        }
    }

    #[test]
    fn linetype_defs_cover_template_linetypes() {
        let defs = linetype_defs();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"CENTER2"));
        assert!(names.contains(&"DASHED2"));
        assert!(names.contains(&"DIVIDE2"));
        // DIVIDE2 模式与内置 OpenCADStudio.lin 一致。
        let divide = defs.iter().find(|d| d.name == "DIVIDE2").unwrap();
        assert_eq!(divide.elements, vec![6.35, -3.175, 0.0, -3.175, 0.0, -3.175]);
    }

    #[test]
    fn text_style_def_is_ocsm_gb() {
        let defs = text_style_defs();
        assert_eq!(defs.len(), 1);
        let s = &defs[0];
        assert_eq!(s.name, "OCSM_GB");
        assert_eq!(s.height, 3.5);
        assert_eq!(s.width_factor, 0.7);
        assert!(s.annotative);
        assert_eq!(s.font_file, "Unicode");
        assert_eq!(s.true_type_font, "Zhuque Fangsong");
    }

    #[test]
    fn scale_factor_semantics() {
        // 1:2 → 2x；2:1 → 0.5x。
        let factor = |v1: i64, v2: i64| v2 as f64 / v1 as f64;
        assert!((factor(1, 2) - 2.0).abs() < 1e-9);
        assert!((factor(2, 1) - 0.5).abs() < 1e-9);
        assert!((factor(1, 1) - 1.0).abs() < 1e-9);
        assert!((factor(1, 100) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn scale_validation_rule_one_must_be_one() {
        let valid = |v1: i64, v2: i64| v1 > 0 && v2 > 0 && (v1 == 1 || v2 == 1);
        assert!(valid(1, 2));
        assert!(valid(2, 1));
        assert!(valid(1, 1));
        assert!(!valid(2, 2));
        assert!(!valid(3, 2));
        assert!(!valid(0, 1));
        assert!(!valid(1, 0));
        assert!(!valid(-1, 1));
    }

    #[test]
    fn frame_dir_prefers_env_override() {
        let _g = global_state_test_lock();   // OCSM_FRAME_DIR 是进程级环境变量
        // OCSM_FRAME_DIR 优先于安装目录解析。
        unsafe { std::env::set_var("OCSM_FRAME_DIR", "/tmp/ocsm-frames") };
        assert_eq!(frame_dir(), std::path::PathBuf::from("/tmp/ocsm-frames"));
        unsafe { std::env::remove_var("OCSM_FRAME_DIR") };
    }

    #[test]
    fn frame_place_on_point_builds_insert_with_attributes() {
        use acadrust::entities::AttributeDefinition;
        let attdefs = vec![
            AttributeDefinition::new("图号".into(), "Drawing No.".into(), " ".into()),
            AttributeDefinition::new("比例".into(), "Scale".into(), " ".into()),
        ];
        let mut cmd = FramePlace {
            block_name: "a3_landscape_att".into(),
            scale: 2.0,
            scale_text: "1:2".into(),
            attdefs,
        };
        match cmd.on_point([100.0, 200.0, 0.0]) {
            CommandStep::CommitAndEnd(acadrust::EntityType::Insert(ins)) => {
                assert_eq!(ins.block_name, "a3_landscape_att");
                assert_eq!(ins.uniform_scale(), Some(2.0));
                assert_eq!(ins.insert_point.x, 100.0);
                assert_eq!(ins.insert_point.y, 200.0);
                assert_eq!(ins.attributes.len(), 2);
                // 属性顺序与 ATTDEF 一致；比例属性填入比例文本。
                assert_eq!(ins.attributes[0].tag, "图号");
                assert_eq!(ins.attributes[0].value, " ");
                assert_eq!(ins.attributes[1].tag, "比例");
                assert_eq!(ins.attributes[1].value, "1:2");
            }
            other => panic!("expected CommitAndEnd Insert, got {other:?}"),
        }
    }

    // ── 标注样式 ────────────────────────────────────────────────────────────

    #[test]
    fn dim_style_def_is_ocsm_gb_mechanical() {
        let defs = dim_style_defs();
        assert_eq!(defs.len(), 1);
        let d = &defs[0];
        // 参数来自用户示例（标注对比.dxf 的 Mechanical 风格，Phase 0 分析）。
        assert_eq!(d.name, "OCSM_GB");
        assert!(d.make_current);
        assert_eq!(d.dimtxt, 2.5);
        assert_eq!(d.dimasz, 2.5);
        assert_eq!(d.dimcen, 2.5);
        assert_eq!(d.dimexe, 2.0);
        assert!((d.dimexo - 0.625).abs() < 1e-9);
        assert_eq!(d.dimgap, 1.0);
        // 与 general.dxf DSTYLE XDATA 一致（渲染实际值）。
        assert!((d.dimdli - 0.38).abs() < 1e-9);
        assert_eq!(d.dimscale, 1.0);
        assert_eq!(d.dimtad, 1);
        assert_eq!(d.dimtoh, true); // 文字强制水平（general.dxf dimtoh=1）
        assert_eq!(d.dimtofl, true);
        assert_eq!(d.dimdec, 2); // 2 位小数（用户指定：原为 4）
        assert_eq!(d.dimlunit, 2);
        assert_eq!(d.dimzin, 8);
        assert_eq!(d.dimclrd, 130);
        assert_eq!(d.dimclre, 130);
        assert_eq!(d.dimclrt, 3);
        assert_eq!(d.dimlwd, -1);
        assert_eq!(d.dimlwe, -1);
        // 字体必须是 OCSM_GB（project.md：标注字体换成 OCSM_GB）。
        assert_eq!(d.dimtxsty, "OCSM_GB");
        // 类型专属 DIMVAR：与黄金模板 OCSM_GB 一致（线性 dimlfac / 角度 dimazin+dimfrac /
        // 公差 dimtfac / 直径半径 dimtmove）——显式钉死，防被模板继承值污染。
        assert_eq!(d.dimlfac, 1.0, "线性比例因子");
        assert_eq!(d.dimtfac, 1.0, "公差字高比例");
        assert_eq!(d.dimazin, 0, "角度消零");
        assert_eq!(d.dimfrac, 0, "角度小数制");
        assert_eq!(d.dimtmove, 0, "文字移动");
        assert!(!d.annotative);
    }

    #[test]
    fn scaled_dim_style_keeps_base_params_with_dimscale() {
        let s = scaled_dim_style_def(2.0);
        assert_eq!(s.name, "OCSM_GB_x2");
        assert_eq!(s.dimscale, 2.0);
        assert_eq!(s.dimtxt, 2.5); // 尺寸不变，由 dimscale 放大
        // 类型专属 DIMVAR 也要随缩放样式一起带上（否则框内线性/角度又会“丢样式”）。
        assert_eq!(s.dimlfac, 1.0);
        assert_eq!(s.dimtfac, 1.0);
        assert_eq!(s.dimazin, 0);
        assert_eq!(s.dimfrac, 0);
        assert_eq!(s.dimtmove, 0);
        assert!(!s.make_current);
        let s = scaled_dim_style_def(0.5);
        assert_eq!(s.name, "OCSM_GB_x0.5");
        assert_eq!(s.dimscale, 0.5);
        assert_eq!(trim_scale(1.0), "1");
        assert_eq!(trim_scale(2.5), "2.5");
        assert_eq!(trim_scale(0.125), "0.125");
    }

    /// 防再丢：按标注类型锁死 OCSM_GB 的关键 DIMVAR（线性 / 角度）。
    /// 黄金参照 = 用户模板 `轴生成器-普通平键.dxf` / `GB-T1095-2003毂槽-*.dxf` 里的
    /// OCSM_GB 记录（`ezdxf` dump）；容差 1e-9（浮点）。
    #[test]
    fn dim_style_def_pins_linear_and_angular_vars() {
        let d = &dim_style_defs()[0];
        // 线性专用：小数位、单位制、比例因子、整体比例、前后缀。
        assert_eq!(d.dimdec, 2, "DIMDEC（线性小数位）");
        assert_eq!(d.dimlunit, 2, "DIMLUNIT（十进制）");
        assert!((d.dimlfac - 1.0).abs() < 1e-9, "DIMLFAC（线性比例）");
        assert!((d.dimscale - 1.0).abs() < 1e-9, "DIMSCALE（整体比例）");
        assert_eq!(d.dimpost, "", "DIMPOST（前后缀）");
        // 角度专用：小数位、单位、消零、小数制。
        assert_eq!(d.dimadec, 0, "DIMADEC（角度小数位）");
        assert_eq!(d.dimaunit, 0, "DIMAUNIT（度）");
        assert_eq!(d.dimazin, 0, "DIMAZIN（角度消零）");
        assert_eq!(d.dimfrac, 0, "DIMFRAC（角度小数制）");
        // 公差 / 直径半径也顺带锁住（任务同一批“类型专属”字段）。
        assert!((d.dimtfac - 1.0).abs() < 1e-9, "DIMTFAC（公差字高比例）");
        assert_eq!(d.dimtmove, 0, "DIMTMOVE（文字移动）");
        assert_eq!(d.dimjust, 0, "DIMJUST（文字水平位置）");
    }

    // ── frame 比例感知 ───────────────────────────────────────────────────────

    /// 合成文档：一个 100×50 的图框块（含 比例 ATTDEF），模型空间插入一次。
    fn synthetic_frame_doc(scale: f64, insert_at: [f64; 2]) -> acadrust::CadDocument {
        use acadrust::entities::{
            AttributeDefinition, Block, BlockEnd, EntityType as E, Insert, Line,
        };
        use acadrust::tables::BlockRecord;
        let mut doc = acadrust::CadDocument::default();
        let mut att = AttributeDefinition::new("比例".into(), "Scale".into(), " ".into());
        att.tag = "比例".into();
        let lines: Vec<E> = vec![
            E::Line(Line {
                common: Default::default(),
                start: Vector3::new(0.0, 0.0, 0.0),
                end: Vector3::new(100.0, 0.0, 0.0),
                thickness: 0.0,
                normal: Vector3::new(0.0, 0.0, 1.0),
            }),
            E::Line(Line {
                common: Default::default(),
                start: Vector3::new(100.0, 0.0, 0.0),
                end: Vector3::new(100.0, 50.0, 0.0),
                thickness: 0.0,
                normal: Vector3::new(0.0, 0.0, 1.0),
            }),
            E::Line(Line {
                common: Default::default(),
                start: Vector3::new(100.0, 50.0, 0.0),
                end: Vector3::new(0.0, 50.0, 0.0),
                thickness: 0.0,
                normal: Vector3::new(0.0, 0.0, 1.0),
            }),
            E::Line(Line {
                common: Default::default(),
                start: Vector3::new(0.0, 50.0, 0.0),
                end: Vector3::new(0.0, 0.0, 0.0),
                thickness: 0.0,
                normal: Vector3::new(0.0, 0.0, 1.0),
            }),
            E::AttributeDefinition(att),
        ];
        let mut br = BlockRecord::new("a3_test");
        br.handle = doc.allocate_handle();
        br.entity_handles
            .push(doc.add_entity(E::Block(Block::new("a3_test", Vector3::ZERO))).unwrap());
        for e in lines {
            br.entity_handles.push(doc.add_entity(e).unwrap());
        }
        br.entity_handles
            .push(doc.add_entity(E::BlockEnd(BlockEnd::new())).unwrap());
        doc.block_records.add(br).unwrap();
        let mut ins = Insert::new("a3_test", Vector3::new(insert_at[0], insert_at[1], 0.0));
        ins.set_x_scale(scale);
        ins.set_y_scale(scale);
        ins.set_z_scale(scale);
        doc.add_entity(E::Insert(ins)).unwrap();
        doc
    }

    #[test]
    fn tf_args_parse_full_and_partial_forms() {
        // 不带参数 → 人侧：弹选择窗口
        assert_eq!(parse_frame_args("").unwrap(), None);
        assert_eq!(parse_frame_args("   ").unwrap(), None);
        // 只给名称 → 1:1 + 交互放置
        let a = parse_frame_args("a3_landscape").unwrap().unwrap();
        assert_eq!(a.name, "a3_landscape");
        assert_eq!((a.v1, a.v2), (1, 1));
        assert_eq!(a.at, None);
        // 名称 + 比例（全角冒号 / 斜杠 / 光数字都收）→ 仍进交互放置
        for token in ["1:2", "1：2", "1/2", "2"] {
            let a = parse_frame_args(&format!("a3_landscape {token}")).unwrap().unwrap();
            assert_eq!((a.v1, a.v2), (1, 2), "比例 {token}");
            assert_eq!(a.at, None, "没给 at → 交互放置");
        }
        // 全自动：名称 + 比例 + at（+ 旋转）
        let a = parse_frame_args("a3_landscape 1:2 at 100,200").unwrap().unwrap();
        assert_eq!((a.v1, a.v2), (1, 2));
        assert_eq!(a.at, Some([100.0, 200.0, 0.0]));
        let a = parse_frame_args("a3_landscape 1:2 at 100,200,5 rot 90").unwrap().unwrap();
        assert_eq!(a.at, Some([100.0, 200.0, 5.0]));
        assert_eq!(a.rot_deg, 90.0);
        // 省略 `at` 关键字、只给坐标也认；放大比例 2:1 / 0.5 也认
        let a = parse_frame_args("a3 200,0").unwrap().unwrap();
        assert_eq!(a.at, Some([200.0, 0.0, 0.0]));
        for token in ["2:1", "0.5"] {
            let a = parse_frame_args(&format!("a3 {token} at 0,0")).unwrap().unwrap();
            assert_eq!((a.v1, a.v2), (2, 1), "比例 {token}");
        }
        // 名称可省（目录里有唯一图框时）
        let a = parse_frame_args("1:2 at 0,0").unwrap().unwrap();
        assert_eq!(a.name, "");
        assert_eq!((a.v1, a.v2), (1, 2));
    }

    #[test]
    fn tf_args_reject_bad_input_with_usage() {
        let _g = global_state_test_lock(); // 语言是进程级全局（与竖切用例串行）
        crate::i18n::set_lang(crate::i18n::Lang::Zh); // 本用例断中文：不随宿主 locale 飘
        for bad in [
            "a3 3:2",          // 两个正整数但都不是 1
            "a3 0:2",          // 不能是 0
            "a3 1.5:2",        // 必须是整数比
            "a3 1:2 at 1,2,3,4", // 坐标维度不对
            "a3 1:2 at 1,x",   // 坐标不是数字
            "a3 1:2 rot",      // rot 缺角度
            "a3 1:2 at",       // at 缺坐标
            "a3 1:2 2:1",      // 比例给了两次
            "a3 1:2 at 0,0 at 1,1", // 基点给了两次
            "a3 1:2 乱写",      // 认不出的参数
        ] {
            let e = parse_frame_args(bad).unwrap_err();
            assert!(
                e.contains("用法") || e.contains("比例") || e.contains("坐标"),
                "{bad} → {e}"
            );
        }
        crate::i18n::set_lang_auto();
    }

    #[test]
    fn tf_usage_and_unknown_param_switch_language_via_catalog() {
        // 双语竖切（用户 2026-09-27）：catalog 取词 → zh/en 切换 → 两种语言都断言。
        let _g = global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        let zh = parse_frame_args("a3 1:2 乱写").unwrap_err();
        assert!(zh.contains("认不出的参数「乱写」。"), "{zh}");
        assert!(zh.contains("用法：TF <名称>"), "{zh}");

        crate::i18n::set_lang(crate::i18n::Lang::En);
        let en = parse_frame_args("a3 1:2 乱写").unwrap_err();
        assert!(en.contains("Unrecognized argument \"乱写\"."), "{en}");
        assert!(en.contains("Usage: TF <name>"), "{en}");
        assert!(!en.contains("用法"), "英文模式不该再出中文用法：{en}");
        // 用法串本身也随语言（另一条错误路径：at 缺坐标）。
        let en_at = parse_frame_args("a3 at").unwrap_err();
        assert!(en_at.contains("Usage: TF <name>"), "{en_at}");
        crate::i18n::set_lang_auto();
    }

    #[test]
    fn resolve_frame_matches_stem_substring_and_reports_ambiguity() {
        let mk = |label: &str| FrameItem {
            path: format!("/tmp/{label}.dwg"),
            label: label.to_string(),
        };
        let frames = vec![mk("a3_landscape"), mk("a4_portrait")];
        // 全名 / 大小写 / 带 .dwg
        assert_eq!(resolve_frame(&frames, "a3_landscape").unwrap().label, "a3_landscape");
        assert_eq!(resolve_frame(&frames, "A3_LANDSCAPE").unwrap().label, "a3_landscape");
        assert_eq!(resolve_frame(&frames, "a3_landscape.dwg").unwrap().label, "a3_landscape");
        // 唯一包含匹配
        assert_eq!(resolve_frame(&frames, "landscape").unwrap().label, "a3_landscape");
        // 名字为空：多个 → 报错列出
        let e = resolve_frame(&frames, "").unwrap_err();
        assert!(e.contains("a3_landscape") && e.contains("a4_portrait"), "{e}");
        // 只一个图框时名字可省
        let one = vec![mk("a3_landscape")];
        assert_eq!(resolve_frame(&one, "").unwrap().label, "a3_landscape");
        // 匹配不到 / 多个候选（精确名永远优先）
        let e = resolve_frame(&frames, "a2").unwrap_err();
        assert!(e.contains("没有叫"), "{e}");
        let many = vec![mk("a3_landscape"), mk("a3_landscape_att")];
        let e = resolve_frame(&many, "a3").unwrap_err();
        assert!(e.contains("多个"), "{e}");
        assert_eq!(
            resolve_frame(&many, "a3_landscape").unwrap().label,
            "a3_landscape",
            "精确名优先于包含匹配"
        );
        assert_eq!(
            resolve_frame(&many, "a3_landscape_att.dwg").unwrap().label,
            "a3_landscape_att"
        );
    }

    #[test]
    fn build_frame_insert_scales_and_stamps_scale_attribute() {
        let mut ad_scale =
            acadrust::entities::AttributeDefinition::new("比例".into(), "Scale".into(), " ".into());
        ad_scale.tag = "比例".into();
        let mut ad_title =
            acadrust::entities::AttributeDefinition::new("图名".into(), "Title".into(), " ".into());
        ad_title.tag = "图名".into();
        ad_title.default_value = "零件图".into();
        let ins = build_frame_insert(
            "a3_landscape",
            2.0,
            "1:2",
            &[ad_scale, ad_title],
            [10.0, 20.0, 0.0],
            90.0,
        );
        assert_eq!(ins.block_name, "a3_landscape");
        assert_eq!(ins.insert_point.x, 10.0);
        assert!((ins.rotation - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert_eq!(ins.attributes.len(), 2);
        let by_tag = |t: &str| {
            ins.attributes
                .iter()
                .find(|a| a.tag == t)
                .map(|a| a.value.clone())
                .unwrap_or_default()
        };
        // `比例` 填 v1:v2（不是 ATTDEF 的默认值），其它属性保持默认值
        assert_eq!(by_tag("比例"), "1:2");
        assert_eq!(by_tag("图名"), "零件图");
    }

    /// 模板 `a3_landscape_att` 的 ATTDEF 口径（用户自备模板，2026-09-28 清单 §P2 实测）：
    /// 格内文字 `halign=Middle`（对齐点=格心，≠ 插入点）、材料/图样名称另带 `valign=Middle`；
    /// 左上角附加栏的「图样代号」`halign=Left`/`valign=Baseline`、按插入点定位、
    /// 倒置 180°（DXF group 50=180 ⇒ 记录里是弧度 π）。
    fn template_like_attdefs() -> Vec<acadrust::entities::AttributeDefinition> {
        use acadrust::entities::AttributeDefinition;

        let mut code = AttributeDefinition::new("图样代号".into(), "Code".into(), " ".into());
        code.insertion_point = Vector3::new(59.509, 285.13, 0.0);
        code.rotation = std::f64::consts::PI;
        code.height = 7.0;
        code.text_style = "OCSM_GB".into();

        let mut material = AttributeDefinition::new("材料".into(), "Material".into(), " ".into());
        material.horizontal_alignment = HorizontalAlignment::Middle;
        material.vertical_alignment = VerticalAlignment::Middle;
        material.insertion_point = Vector3::new(299.609, 30.268, 0.0);
        material.alignment_point = Vector3::new(300.0, 32.5, 0.0);
        material.height = 5.0;
        material.text_style = "OCSM_GB".into();

        let mut title = AttributeDefinition::new("图样名称".into(), "Title".into(), " ".into());
        title.horizontal_alignment = HorizontalAlignment::Middle;
        title.vertical_alignment = VerticalAlignment::Middle;
        title.insertion_point = Vector3::new(360.387, 19.697, 0.0);
        title.alignment_point = Vector3::new(360.0, 22.5, 0.0);
        title.height = 5.0;
        title.width_factor = 0.7;
        title.text_style = "OCSM_GB".into();

        vec![code, material, title]
    }

    /// 非 1:1 插入与 1:1 **走同一条路**：对齐枚举、对齐点换算、旋转都不许变形。
    /// （用户清单 §P2 验收：2:1 后 halign/valign 与 1:1 一致、alignment_point = 1:1 值 × 2、
    /// rotation == π。）
    #[test]
    fn frame_insert_keeps_attributes_identical_across_scales() {
        let attdefs = template_like_attdefs();
        let one = build_frame_insert("a3_landscape_att", 1.0, "1:1", &attdefs, [0.0, 0.0, 0.0], 0.0);
        for (scale, label) in [(2.0, "2:1"), (0.5, "1:2")] {
            let scaled =
                build_frame_insert("a3_landscape_att", scale, label, &attdefs, [0.0, 0.0, 0.0], 0.0);
            assert_eq!(scaled.attributes.len(), one.attributes.len(), "{label}");
            for (base, got) in one.attributes.iter().zip(&scaled.attributes) {
                assert_eq!(base.tag, got.tag, "{label} 属性顺序与 ATTDEF 一致");
                assert_eq!(
                    (base.horizontal_alignment, base.vertical_alignment),
                    (got.horizontal_alignment, got.vertical_alignment),
                    "{label} 的 {} 对齐被改动了",
                    base.tag
                );
                assert_eq!(base.width_factor, got.width_factor, "{label} {}", base.tag);
                assert_eq!(base.text_style, got.text_style, "{label} {}", base.tag);
                assert!(
                    (got.alignment_point.x - base.alignment_point.x * scale).abs() < 1e-9
                        && (got.alignment_point.y - base.alignment_point.y * scale).abs() < 1e-9,
                    "{label} 的 {} 对齐点没随比例缩放：{:?} → {:?}",
                    base.tag,
                    base.alignment_point,
                    got.alignment_point
                );
                assert!(
                    (got.rotation - base.rotation).abs() < 1e-12,
                    "{label} 的 {} 旋转被二次换算：{} → {}",
                    base.tag,
                    base.rotation,
                    got.rotation
                );
                assert!(
                    (got.height - base.height * scale).abs() < 1e-9,
                    "{label} 的 {} 字高没随比例缩放",
                    base.tag
                );
            }
        }

        // 倒置的「图样代号」：180° 必须还是弧度 π（0.054831 = to_radians(π) 即为二次换算）。
        let code = one
            .attributes
            .iter()
            .find(|a| a.tag == "图样代号")
            .expect("图样代号属性");
        assert!((code.rotation - std::f64::consts::PI).abs() < 1e-12);
    }

    /// 旋转参数只做一次「度 → 弧度」：`rot 180` ⇒ π（不是 to_radians(π)）。
    #[test]
    fn frame_insert_converts_the_rotation_argument_once() {
        let attdefs = template_like_attdefs();
        let ins = build_frame_insert(
            "a3_landscape_att",
            2.0,
            "2:1",
            &attdefs,
            [0.0, 0.0, 0.0],
            180.0,
        );
        assert!((ins.rotation - std::f64::consts::PI).abs() < 1e-12);
    }

    /// 对齐点丢失（导出器把 group 11 写空）时退回插入点：文字不许落到块原点。
    /// 用户清单 §P2.1 的症状就是这个（标题栏看起来空的、「图样代号」横穿图框）。
    #[test]
    fn frame_insert_restores_a_missing_attribute_alignment_point() {
        use acadrust::entities::AttributeDefinition;
        let mut broken = AttributeDefinition::new("材料".into(), "Material".into(), " ".into());
        broken.horizontal_alignment = HorizontalAlignment::Middle;
        broken.vertical_alignment = VerticalAlignment::Middle;
        broken.insertion_point = Vector3::new(299.609, 30.268, 0.0);
        broken.alignment_point = Vector3::ZERO;
        broken.height = 5.0;

        let ins = build_frame_insert(
            "a3_landscape_att",
            2.0,
            "2:1",
            &[broken],
            [0.0, 0.0, 0.0],
            0.0,
        );
        let attr = &ins.attributes[0];
        assert_eq!
            (
                attr.alignment_point,
                attr.insertion_point,
                "对齐点丢了就退回插入点"
            );
        assert!(attr.alignment_point.x.abs() > 1e-9, "文字不该落在块原点");
        assert_eq!(attr.horizontal_alignment, HorizontalAlignment::Middle);
        assert_eq!(attr.vertical_alignment, VerticalAlignment::Middle);
    }

    #[test]
    fn frame_scale_at_finds_innermost_containing_insert() {
        // 2 倍图框插在 (0,0)，块局部 100×50 → 世界 200×100。
        let doc = synthetic_frame_doc(2.0, [0.0, 0.0]);
        assert_eq!(frame_scale_at(&doc, [50.0, 25.0, 0.0]), 2.0);
        assert_eq!(frame_scale_at(&doc, [199.0, 99.0, 0.0]), 2.0);
        // 框外 → 1.0。
        assert_eq!(frame_scale_at(&doc, [201.0, 25.0, 0.0]), 1.0);
        assert_eq!(frame_scale_at(&doc, [50.0, 101.0, 0.0]), 1.0);
    }

    #[test]
    fn frame_scale_at_prefers_innermost_smaller_frame() {
        // 外层 4x（0,0 起 400×200）内含 2x（100,100 起 200×100）：取内层 2。
        let mut doc = synthetic_frame_doc(4.0, [0.0, 0.0]);
        let inner = synthetic_frame_doc(2.0, [100.0, 100.0]);
        // 合并两个文档的实体/块（简单做法：把内层块的实体也加进 doc）。
        let inner_entities: Vec<EntityType> = {
            let br = inner.block_records.get("a3_test").unwrap();
            br.entity_handles
                .iter()
                .filter_map(|h| inner.get_entity(*h).cloned())
                .collect()
        };
        let mut ins = Insert::new("a3_test", Vector3::new(100.0, 100.0, 0.0));
        ins.set_x_scale(2.0);
        ins.set_y_scale(2.0);
        ins.set_z_scale(2.0);
        doc.add_entity(EntityType::Insert(ins)).unwrap();
        // 块已存在（同名），只补实体到现有块定义里。
        let mut new_handles = Vec::new();
        for e in inner_entities {
            new_handles.push(doc.add_entity(e).unwrap());
        }
        doc.block_records.get_mut("a3_test").unwrap().entity_handles.extend(new_handles);
        // 点在内层框内 → 2；在外层框内但在内层外 → 4。
        assert_eq!(frame_scale_at(&doc, [150.0, 150.0, 0.0]), 2.0);
        assert_eq!(frame_scale_at(&doc, [350.0, 150.0, 0.0]), 4.0);
    }

    // ── POWERDIM 状态机 ───────────────────────────────────────────────────────

    #[test]
    fn powerdim_two_points_places_aligned_dimension() {
        use ocs_plugin_api::host::InteractiveCommand as Ic;
        let mut cmd = PowerDim::new(acadrust::CadDocument::default());
        // 第一次点击：自由拾取点（handle NULL）。
        assert!(matches!(
            cmd.on_object_pick_snapped(acadrust::Handle::NULL, [0.0, 0.0, 0.0], false),
            CommandStep::NeedPoint
        ));
        assert!(cmd.prompt().contains("选择下一个拾取点"));
        // 第二次点击：另一个拾取点。
        assert!(matches!(
            cmd.on_object_pick_snapped(acadrust::Handle::NULL, [100.0, 0.0, 0.0], false),
            CommandStep::NeedPoint
        ));
        assert!(cmd.prompt().contains("对齐(A默认)"));
        // 第三次点击：放置 → 对齐标注。
        match cmd.on_object_pick_snapped(acadrust::Handle::NULL, [50.0, -20.0, 0.0], false) {
            CommandStep::CommitAndEnd(acadrust::EntityType::Dimension(dim)) => {
                assert!(matches!(dim, Dimension::Aligned(_)));
                assert_eq!(dim.base().common.layer, "7标注层");
                assert_eq!(dim.base().style_name, "OCSM_GB");
                assert!((dim.base().actual_measurement - 100.0).abs() < 1e-6);
            }
            other => panic!("expected CommitAndEnd Aligned, got {other:?}"),
        }
    }

    #[test]
    fn powerdim_keywords_lock_horizontal_vertical() {
        use ocs_plugin_api::host::InteractiveCommand as Ic;
        let mut cmd = PowerDim::new(acadrust::CadDocument::default());
        let _ = cmd.on_object_pick_snapped(acadrust::Handle::NULL, [0.0, 0.0, 0.0], false);
        let _ = cmd.on_object_pick_snapped(acadrust::Handle::NULL, [100.0, 0.0, 0.0], false);
        // 关键字 H → 水平线性标注。
        assert!(matches!(cmd.on_text_input("H"), CommandStep::NeedPoint));
        match cmd.on_object_pick_snapped(acadrust::Handle::NULL, [50.0, -20.0, 0.0], false) {
            CommandStep::CommitAndEnd(acadrust::EntityType::Dimension(Dimension::Linear(d))) => {
                assert!(d.rotation.abs() < 1e-9);
                assert!((d.base.actual_measurement - 100.0).abs() < 1e-6);
            }
            other => panic!("expected horizontal Linear, got {other:?}"),
        }
        // V → 竖直。
        let mut cmd = PowerDim::new(acadrust::CadDocument::default());
        let _ = cmd.on_object_pick_snapped(acadrust::Handle::NULL, [0.0, 0.0, 0.0], false);
        let _ = cmd.on_object_pick_snapped(acadrust::Handle::NULL, [0.0, 80.0, 0.0], false);
        assert!(matches!(cmd.on_text_input("V"), CommandStep::NeedPoint));
        match cmd.on_object_pick_snapped(acadrust::Handle::NULL, [30.0, 40.0, 0.0], false) {
            CommandStep::CommitAndEnd(acadrust::EntityType::Dimension(Dimension::Linear(d))) => {
                assert!((d.rotation - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
                assert!((d.base.actual_measurement - 80.0).abs() < 1e-6);
            }
            other => panic!("expected vertical Linear, got {other:?}"),
        }
    }

    #[test]
    fn powerdim_line_line_creates_angular_with_major() {
        use ocs_plugin_api::host::InteractiveCommand as Ic;
        let mut doc = acadrust::CadDocument::default();
        // 两条不平行线段：L1 (0,0)-(100,0)，L2 (0,0)-(0,100)。
        let h1 = doc
            .add_entity(EntityType::Line(Line {
                common: Default::default(),
                start: Vector3::new(0.0, 0.0, 0.0),
                end: Vector3::new(100.0, 0.0, 0.0),
                thickness: 0.0,
                normal: Vector3::new(0.0, 0.0, 1.0),
            }))
            .unwrap();
        let h2 = doc
            .add_entity(EntityType::Line(Line {
                common: Default::default(),
                start: Vector3::new(0.0, 0.0, 0.0),
                end: Vector3::new(0.0, 100.0, 0.0),
                thickness: 0.0,
                normal: Vector3::new(0.0, 0.0, 1.0),
            }))
            .unwrap();
        let mut cmd = PowerDim::new(doc.clone());
        // 先点 L1（无吸附）→ LinePlace。
        assert!(matches!(
            cmd.on_object_pick_snapped(h1, [50.0, 0.0, 0.0], false),
            CommandStep::NeedPoint
        ));
        // 再点 L2 → AngularPlace。
        assert!(matches!(
            cmd.on_object_pick_snapped(h2, [0.0, 50.0, 0.0], false),
            CommandStep::NeedPoint
        ));
        assert!(cmd.prompt().contains("劣角(I默认)"));
        // 劣角：点击在夹角内侧 → 90°（actual_measurement + user_text）。
        match cmd.on_object_pick_snapped(acadrust::Handle::NULL, [25.0, 25.0, 0.0], false) {
            CommandStep::CommitAndEnd(acadrust::EntityType::Dimension(Dimension::Angular2Ln(
                d,
            ))) => {
                assert!((d.base.actual_measurement - 90.0).abs() < 1e-6);
                assert_eq!(d.base.user_text.as_deref(), Some("90°"));
            }
            other => panic!("expected Angular2Ln, got {other:?}"),
        }
        // 优角：R 关键字 → 270°。
        let mut cmd = PowerDim::new(doc.clone());
        let _ = cmd.on_object_pick_snapped(h1, [50.0, 0.0, 0.0], false);
        let _ = cmd.on_object_pick_snapped(h2, [0.0, 50.0, 0.0], false);
        assert!(matches!(cmd.on_text_input("R"), CommandStep::NeedPoint));
        match cmd.on_object_pick_snapped(acadrust::Handle::NULL, [25.0, 25.0, 0.0], false) {
            CommandStep::CommitAndEnd(acadrust::EntityType::Dimension(Dimension::Angular2Ln(
                d,
            ))) => {
                assert!((d.base.actual_measurement - 270.0).abs() < 1e-6);
                assert_eq!(d.base.user_text.as_deref(), Some("270°"));
            }
            other => panic!("expected Angular2Ln major, got {other:?}"),
        }
    }

    #[test]
    fn powerdim_parallel_lines_reject_angle() {
        use ocs_plugin_api::host::InteractiveCommand as Ic;
        let mut doc = acadrust::CadDocument::default();
        let h1 = doc
            .add_entity(EntityType::Line(Line {
                common: Default::default(),
                start: Vector3::new(0.0, 0.0, 0.0),
                end: Vector3::new(100.0, 0.0, 0.0),
                thickness: 0.0,
                normal: Vector3::new(0.0, 0.0, 1.0),
            }))
            .unwrap();
        let h2 = doc
            .add_entity(EntityType::Line(Line {
                common: Default::default(),
                start: Vector3::new(0.0, 10.0, 0.0),
                end: Vector3::new(100.0, 10.0, 0.0),
                thickness: 0.0,
                normal: Vector3::new(0.0, 0.0, 1.0),
            }))
            .unwrap();
        let mut cmd = PowerDim::new(doc);
        let _ = cmd.on_object_pick_snapped(h1, [50.0, 0.0, 0.0], false);
        assert!(matches!(
            cmd.on_object_pick_snapped(h2, [50.0, 10.0, 0.0], false),
            CommandStep::NeedPoint
        ));
        assert!(cmd.parallel_notice);
        assert!(cmd.prompt().contains("两直线平行"));
    }

    #[test]
    fn powerdim_circle_inside_outside_leader() {
        use ocs_plugin_api::host::InteractiveCommand as Ic;
        let mut doc = acadrust::CadDocument::default();
        let h = doc
            .add_entity(EntityType::Circle(Circle {
                common: Default::default(),
                center: Vector3::new(50.0, 50.0, 0.0),
                radius: 20.0,
                thickness: 0.0,
                normal: Vector3::new(0.0, 0.0, 1.0),
            }))
            .unwrap();
        let mut cmd = PowerDim::new(doc.clone());
        // 点击圆（无吸附）→ 直径模式提示。
        assert!(matches!(
            cmd.on_object_pick_snapped(h, [50.0, 30.0, 0.0], false),
            CommandStep::NeedPoint
        ));
        assert!(cmd.prompt().contains("直径(D默认)"));
        // 光标在圆外 → 外标注（leader>0）。
        match cmd.on_object_pick_snapped(acadrust::Handle::NULL, [100.0, 50.0, 0.0], false) {
            CommandStep::CommitAndEnd(acadrust::EntityType::Dimension(Dimension::Diameter(d))) => {
                assert!(d.leader_length > 0.0);
                assert!((d.base.actual_measurement - 40.0).abs() < 1e-6);
            }
            other => panic!("expected Diameter, got {other:?}"),
        }
        // R 关键字 → 半径；光标在圆内 → 内标注（leader=0）。
        let mut cmd = PowerDim::new(doc.clone());
        let _ = cmd.on_object_pick_snapped(h, [50.0, 30.0, 0.0], false);
        assert!(matches!(cmd.on_text_input("R"), CommandStep::NeedPoint));
        match cmd.on_object_pick_snapped(acadrust::Handle::NULL, [55.0, 55.0, 0.0], false) {
            CommandStep::CommitAndEnd(acadrust::EntityType::Dimension(Dimension::Radius(d))) => {
                assert_eq!(d.leader_length, 0.0);
                assert!((d.base.actual_measurement - 20.0).abs() < 1e-6);
            }
            other => panic!("expected Radius, got {other:?}"),
        }
    }

    #[test]
    fn powerdim_arc_radius_and_arclength() {
        use ocs_plugin_api::host::InteractiveCommand as Ic;
        let mut doc = acadrust::CadDocument::default();
        let h = doc
            .add_entity(EntityType::Arc(Arc {
                common: Default::default(),
                center: Vector3::new(0.0, 0.0, 0.0),
                radius: 50.0,
                start_angle: 0.0,
                end_angle: std::f64::consts::FRAC_PI_2,
                thickness: 0.0,
                normal: Vector3::new(0.0, 0.0, 1.0),
            }))
            .unwrap();
        // 半径模式（默认）。
        let mut cmd = PowerDim::new(doc.clone());
        let _ = cmd.on_object_pick_snapped(h, [50.0, 0.0, 0.0], false);
        assert!(cmd.prompt().contains("半径(R默认)"));
        match cmd.on_object_pick_snapped(acadrust::Handle::NULL, [100.0, 50.0, 0.0], false) {
            CommandStep::CommitAndEnd(acadrust::EntityType::Dimension(Dimension::Radius(d))) => {
                assert!((d.base.actual_measurement - 50.0).abs() < 1e-6);
            }
            other => panic!("expected Radius, got {other:?}"),
        }
        // 弧长模式：A 关键字 → DimensionArc，弧长 = r·(π/2)。
        let mut cmd = PowerDim::new(doc.clone());
        let _ = cmd.on_object_pick_snapped(h, [50.0, 0.0, 0.0], false);
        assert!(matches!(cmd.on_text_input("A"), CommandStep::NeedPoint));
        match cmd.on_object_pick_snapped(acadrust::Handle::NULL, [80.0, 80.0, 0.0], false) {
            CommandStep::CommitAndEnd(acadrust::EntityType::Dimension(Dimension::Arc(d))) => {
                let expected = 50.0 * std::f64::consts::FRAC_PI_2;
                assert!((d.base.actual_measurement - expected).abs() < 1e-6);
                assert!(d.has_leader); // 光标在外侧
            }
            other => panic!("expected Arc, got {other:?}"),
        }
    }

    #[test]
    fn powerdim_resolve_style_uses_scaled_style_when_present() {
        use ocs_plugin_api::host::InteractiveCommand as Ic;
        use acadrust::tables::DimStyle;
        let mut doc = synthetic_frame_doc(2.0, [0.0, 0.0]);
        // 模拟 TF 已创建缩放样式：OCSM_GB_x2 存在。
        let mut style = DimStyle::new("OCSM_GB_x2");
        style.handle = doc.allocate_handle();
        style.dimscale = 2.0;
        doc.dim_styles.add_or_replace(style);
        let mut cmd = PowerDim::new(doc);
        // 第一次点击在 2x 框内 → 样式 OCSM_GB_x2。
        let _ = cmd.on_object_pick_snapped(acadrust::Handle::NULL, [50.0, 25.0, 0.0], true);
        assert_eq!(cmd.style_name, "OCSM_GB_x2");
        assert!(cmd.style_hint.is_none());
        // 缩放样式缺失时回退 OCSM_GB 并提示。
        let mut cmd = PowerDim::new(synthetic_frame_doc(4.0, [0.0, 0.0]));
        let _ = cmd.on_object_pick_snapped(acadrust::Handle::NULL, [50.0, 25.0, 0.0], true);
        assert_eq!(cmd.style_name, "OCSM_GB");
        assert!(cmd.style_hint.is_some());
    }

    #[test]
    fn powerdim_enter_toggles_segment_select_mode() {
        use ocs_plugin_api::host::InteractiveCommand as Ic;
        let mut doc = acadrust::CadDocument::default();
        let h = doc
            .add_entity(EntityType::Line(Line {
                common: Default::default(),
                start: Vector3::new(0.0, 0.0, 0.0),
                end: Vector3::new(100.0, 0.0, 0.0),
                thickness: 0.0,
                normal: Vector3::new(0.0, 0.0, 1.0),
            }))
            .unwrap();
        let mut cmd = PowerDim::new(doc);
        // 默认拾取点模式：OSNAP 开。
        assert!(cmd.entity_pick_applies_osnap());
        assert!(cmd.prompt().contains("拾取点模式"));
        // Enter → 线段点选模式：OSNAP 关。
        assert!(matches!(cmd.on_enter(), CommandStep::NeedPoint));
        assert!(!cmd.entity_pick_applies_osnap());
        assert!(cmd.prompt().contains("线段点选"));
        // 点选模式下点击直线（snapped=false）→ 立即进入线段标注。
        assert!(matches!(
            cmd.on_object_pick_snapped(h, [50.0, 0.0, 0.0], false),
            CommandStep::NeedPoint
        ));
        assert!(cmd.prompt().contains("对齐(A默认)"));
        // 放置（任意点）→ 对齐标注。
        match cmd.on_object_pick_snapped(acadrust::Handle::NULL, [50.0, -20.0, 0.0], false) {
            CommandStep::CommitAndEnd(acadrust::EntityType::Dimension(dim)) => {
                assert!(matches!(dim, Dimension::Aligned(_)));
            }
            other => panic!("expected Aligned, got {other:?}"),
        }
        // 点选模式下 First 状态：空白点击忽略。
        let mut cmd = PowerDim::new(acadrust::CadDocument::default());
        assert!(matches!(cmd.on_enter(), CommandStep::NeedPoint));
        assert!(matches!(
            cmd.on_object_pick_snapped(acadrust::Handle::NULL, [10.0, 10.0, 0.0], false),
            CommandStep::NeedPoint
        ));
        assert!(matches!(cmd.step, Step::First));
        // 再 Enter 回到拾取点模式。
        assert!(matches!(cmd.on_enter(), CommandStep::NeedPoint));
        assert!(cmd.entity_pick_applies_osnap());
    }

    #[test]
    fn powerdim_radial_text_is_user_positioned_for_outside() {
        use ocs_plugin_api::host::InteractiveCommand as Ic;
        let mut doc = acadrust::CadDocument::default();
        let h = doc
            .add_entity(EntityType::Circle(Circle {
                common: Default::default(),
                center: Vector3::new(50.0, 50.0, 0.0),
                radius: 20.0,
                thickness: 0.0,
                normal: Vector3::new(0.0, 0.0, 1.0),
            }))
            .unwrap();
        let mut cmd = PowerDim::new(doc);
        let _ = cmd.on_object_pick_snapped(h, [50.0, 30.0, 0.0], false);
        // 光标在圆外 → 外标注：leader>0 且文字跟随点击点（user_positioned）。
        match cmd.on_object_pick_snapped(acadrust::Handle::NULL, [100.0, 50.0, 0.0], false) {
            CommandStep::CommitAndEnd(acadrust::EntityType::Dimension(Dimension::Diameter(d))) => {
                assert!(d.leader_length > 0.0);
                assert!(d.base.text_user_positioned);
                assert!((d.base.text_middle_point.x - 100.0).abs() < 1e-9);
            }
            other => panic!("expected Diameter, got {other:?}"),
        }
    }

    #[test]
    fn powerdim_arc_length_uses_actual_sweep_including_major() {
        use ocs_plugin_api::host::InteractiveCommand as Ic;
        let mut doc = acadrust::CadDocument::default();
        // 270° 优弧：start=0°, end=270°。
        let h = doc
            .add_entity(EntityType::Arc(Arc {
                common: Default::default(),
                center: Vector3::new(0.0, 0.0, 0.0),
                radius: 50.0,
                start_angle: 0.0,
                end_angle: std::f64::consts::TAU * 0.75,
                thickness: 0.0,
                normal: Vector3::new(0.0, 0.0, 1.0),
            }))
            .unwrap();
        let mut cmd = PowerDim::new(doc);
        let _ = cmd.on_object_pick_snapped(h, [50.0, 0.0, 0.0], false);
        assert!(matches!(cmd.on_text_input("A"), CommandStep::NeedPoint));
        match cmd.on_object_pick_snapped(acadrust::Handle::NULL, [80.0, 80.0, 0.0], false) {
            CommandStep::CommitAndEnd(acadrust::EntityType::Dimension(Dimension::Arc(d))) => {
                let expected = 50.0 * std::f64::consts::TAU * 0.75;
                assert!(
                    (d.base.actual_measurement - expected).abs() < 1e-6,
                    "arc length should be the major sweep, got {}",
                    d.base.actual_measurement
                );
                assert!(d.is_partial, "partial sweep keeps the major arc");
                assert!(d.base.text_user_positioned);
            }
            other => panic!("expected Arc, got {other:?}"),
        }
    }

    #[test]
    fn powerdim_angular_measures_45_degrees_not_90() {
        use ocs_plugin_api::host::InteractiveCommand as Ic;
        // 45° 相交的两条线（非正交），验证角度值正确（F5 回归）。
        let minor = angular_minor_sweep_deg(
            &LineGeom {
                start: [0.0, 0.0, 0.0],
                end: [100.0, 0.0, 0.0],
            },
            &LineGeom {
                start: [0.0, 0.0, 0.0],
                end: [100.0, 100.0, 0.0],
            },
            [50.0, 20.0, 0.0],
        );
        assert!((minor - 45.0).abs() < 1e-6, "got {minor}");
    }

    #[test]
    fn powerdim_preview_follows_cursor() {
        use ocs_plugin_api::host::InteractiveCommand as Ic;
        let mut cmd = PowerDim::new(acadrust::CadDocument::default());
        // 第一步无预览。
        assert!(cmd.on_mouse_move([10.0, 10.0, 0.0]).is_none());
        let _ = cmd.on_object_pick_snapped(acadrust::Handle::NULL, [0.0, 0.0, 0.0], false);
        // 两点步骤：橡皮筋线。
        assert!(matches!(
            cmd.on_mouse_move([50.0, 30.0, 0.0]),
            Some(EntityType::Line(_))
        ));
        let _ = cmd.on_object_pick_snapped(acadrust::Handle::NULL, [100.0, 0.0, 0.0], false);
        // 放置步骤：标注实体预览（跟随光标）。
        match cmd.on_mouse_move([50.0, -20.0, 0.0]) {
            Some(EntityType::Dimension(dim)) => {
                assert!(matches!(dim, Dimension::Aligned(_)));
                assert_eq!(dim.base().common.layer, "7标注层");
                assert_eq!(dim.base().style_name, "OCSM_GB");
            }
            other => panic!("expected preview Dimension, got {other:?}"),
        }
    }
    /// 只关心“谁先动了文档”的宿主桩：HostApi 的 ensure_* / ensure_plugin_state 默认
    /// 什么都不做，这里覆盖成记录调用，用来断言 push_undo 发生在表变更之前。
    #[derive(Default)]
    struct UndoOrderSpy {
        log: Vec<String>,
        outputs: Vec<String>,
        errors: Vec<String>,
        infos: Vec<String>,
        undo: Vec<String>,
        doc: ocs_plugin_api::host::CadDocument,
        path: Option<std::path::PathBuf>,
        // ensure_* 收到的 defs（自动初始化 vs 手动 OCSM 逐项对比用）。
        linetypes: Vec<ocs_plugin_api::host::LinetypeDef>,
        layers: Vec<ocs_plugin_api::host::LayerDef>,
        text_styles: Vec<ocs_plugin_api::host::TextStyleDef>,
        dim_styles: Vec<ocs_plugin_api::host::DimStyleDef>,
    }
    /// 空的只读文档视图（HostApi 要求实现 DocumentReader）。
    struct SpyReader;
    impl ocs_plugin_api::host::DocumentReader for SpyReader {
        fn entity_count(&self) -> usize {
            0
        }
        fn for_each_entity(&self, _f: &mut dyn FnMut(ocs_plugin_api::host::ReaderEntity<'_>)) {}
        fn layer_name(&self, _handle: ocs_plugin_api::host::Handle) -> Option<&str> {
            None
        }
        fn app_id_name(&self, _handle: ocs_plugin_api::host::Handle) -> Option<&str> {
            None
        }
    }
    impl ocs_plugin_api::host::HostApi for UndoOrderSpy {
        fn tab_index(&self) -> usize {
            0
        }
        fn document(&self) -> &ocs_plugin_api::host::CadDocument {
            &self.doc
        }
        fn document_mut(&mut self) -> &mut ocs_plugin_api::host::CadDocument {
            &mut self.doc
        }
        fn add_entity(&mut self, _entity: ocs_plugin_api::host::EntityType) -> ocs_plugin_api::host::Handle {
            ocs_plugin_api::host::Handle::NULL
        }
        fn bump_geometry(&mut self) {
            self.log.push("bump_geometry".into());
        }
        fn read_record(
            &self,
            _handle: ocs_plugin_api::host::Handle,
            _app_name: &str,
        ) -> Option<&ocs_plugin_api::host::ExtendedDataRecord> {
            None
        }
        fn write_record(
            &mut self,
            _handle: ocs_plugin_api::host::Handle,
            _record: ocs_plugin_api::host::ExtendedDataRecord,
        ) -> bool {
            false
        }
        fn remove_record(&mut self, _handle: ocs_plugin_api::host::Handle, _app_name: &str) -> bool {
            false
        }
        fn push_undo(&mut self, label: &str) {
            self.undo.push(label.to_string());
            self.log.push(format!("push_undo:{label}"));
        }
        fn set_dirty(&mut self) {}
        fn push_info(&mut self, msg: &str) {
            self.infos.push(msg.to_string());
        }
        fn push_output(&mut self, msg: &str) {
            self.outputs.push(msg.to_string());
        }
        fn push_error(&mut self, msg: &str) {
            self.errors.push(msg.to_string());
        }
        fn start_interactive(
            &mut self,
            _command: Box<dyn ocs_plugin_api::host::InteractiveCommand>,
        ) {
            self.log.push("start_interactive".into());
        }
        fn plugin_state_any(&self, _plugin_id: &str) -> Option<&(dyn std::any::Any + Send + Sync)> {
            None
        }
        fn plugin_state_any_mut(
            &mut self,
            _plugin_id: &str,
        ) -> Option<&mut (dyn std::any::Any + Send + Sync)> {
            None
        }
        fn ensure_plugin_state_any(
            &mut self,
            _plugin_id: &'static str,
            init: &mut dyn FnMut() -> Box<dyn std::any::Any + Send + Sync>,
        ) -> &mut (dyn std::any::Any + Send + Sync) {
            Box::leak(init())
        }
        fn document_path(&self, _tab_id: u64) -> Option<std::path::PathBuf> {
            self.path.clone()
        }
        fn document_reader(&self) -> Box<dyn ocs_plugin_api::host::DocumentReader + '_> {
            Box::new(SpyReader)
        }
        fn ensure_linetypes(&mut self, defs: Vec<ocs_plugin_api::host::LinetypeDef>) -> usize {
            self.log.push("ensure_linetypes".into());
            let mut created = 0;
            for def in &defs {
                if self
                    .doc
                    .line_types
                    .iter()
                    .any(|l| l.name.eq_ignore_ascii_case(&def.name))
                {
                    continue;
                }
                self.doc
                    .line_types
                    .add_or_replace(acadrust::tables::LineType::new(&def.name));
                created += 1;
            }
            self.linetypes.extend(defs);
            created
        }
        fn ensure_layers(&mut self, defs: Vec<ocs_plugin_api::host::LayerDef>) -> usize {
            self.log.push("ensure_layers".into());
            let mut created = 0;
            for def in &defs {
                if self
                    .doc
                    .layers
                    .iter()
                    .any(|l| l.name.eq_ignore_ascii_case(&def.name))
                {
                    continue;
                }
                self.doc
                    .layers
                    .add_or_replace(acadrust::tables::Layer::new(&def.name));
                created += 1;
            }
            self.layers.extend(defs);
            created
        }
        fn ensure_text_styles(&mut self, defs: Vec<ocs_plugin_api::host::TextStyleDef>) -> usize {
            self.log.push("ensure_text_styles".into());
            let mut created = 0;
            for def in &defs {
                if self
                    .doc
                    .text_styles
                    .iter()
                    .any(|s| s.name.eq_ignore_ascii_case(&def.name))
                {
                    continue;
                }
                self.doc
                    .text_styles
                    .add_or_replace(acadrust::tables::TextStyle::new(&def.name));
                created += 1;
            }
            self.text_styles.extend(defs);
            created
        }
        /// 只实现插件用到的 `TextStyleModify`（旧图救字体的兜底路径）；其余操作不支持。
        fn table_operation(
            &mut self,
            operation: ocs_plugin_api::host::TableOperation,
        ) -> Result<ocs_plugin_api::host::Handle, String> {
            use ocs_plugin_api::host::TableOperation;
            match operation {
                TableOperation::TextStyleModify { config } => {
                    self.log.push("table_operation:TextStyleModify".into());
                    let style = self
                        .doc
                        .text_styles
                        .get_mut(&config.name)
                        .ok_or_else(|| format!("text style {:?} does not exist", config.name))?;
                    if let Some(ttf) = config.true_type_font {
                        style.true_type_font = ttf;
                    }
                    Ok(style.handle)
                }
                other => Err(format!("UndoOrderSpy: unsupported {other:?}")),
            }
        }
        fn ensure_dim_styles(&mut self, defs: Vec<ocs_plugin_api::host::DimStyleDef>) -> usize {
            self.log.push("ensure_dim_styles".into());
            let mut created = 0;
            for def in &defs {
                if !self
                    .doc
                    .dim_styles
                    .iter()
                    .any(|s| s.name.eq_ignore_ascii_case(&def.name))
                {
                    self.doc
                        .dim_styles
                        .add_or_replace(acadrust::tables::DimStyle::new(&def.name));
                    created += 1;
                }
                if def.make_current {
                    self.doc.header.current_dimstyle_name = def.name.clone();
                }
            }
            self.dim_styles.extend(defs);
            created
        }
    }

    #[test]
    fn mutating_commands_push_undo_before_touching_tables() {
        // 宿主 ensure_* 直接改表、不自动入撤销栈（MCP `op:"undo"` 也撤不掉），
        // 所以命令必须自己先声明撤销点 —— 2026-09-15 发现的契约缺口。
        let mut host = UndoOrderSpy::default();
        OcsmPlugin.cmd_init(&mut host);
        assert_eq!(
            host.log.first().map(String::as_str),
            Some("push_undo:OCSM 初始化"),
            "{:?}",
            host.log
        );
        assert!(
            host.log.iter().any(|l| l == "ensure_layers"),
            "初始化应真的去建表：{:?}",
            host.log
        );

        let mut host = UndoOrderSpy::default();
        OcsmPlugin.cmd_powerdim(&mut host);
        assert_eq!(
            host.log.first().map(String::as_str),
            Some("push_undo:OCSM 智能标注"),
            "{:?}",
            host.log
        );
    }

    // ── 新建空图纸自动初始化（宿主无「文档新建/打开」事件 → 严格启发式）──

    /// 门禁纯函数：路径 / 实体 / OCSM 痕迹任一命中就不算新图。
    #[test]
    fn is_new_blank_drawing_is_strict() {
        let doc = acadrust::CadDocument::new();
        assert!(is_new_blank_drawing(&doc, None), "空 + 无路径 + 无 OCSM → 新图");
        assert!(
            !is_new_blank_drawing(&doc, Some(std::path::Path::new("/tmp/old.dwg"))),
            "存过盘的旧图（哪怕空）→ 不动"
        );

        let mut with_entity = acadrust::CadDocument::new();
        let _ = with_entity.add_entity(EntityType::Point(acadrust::entities::Point::new()));
        assert!(!is_new_blank_drawing(&with_entity, None), "有实体 → 不动");

        let mut with_layer = acadrust::CadDocument::new();
        with_layer
            .layers
            .add_or_replace(acadrust::tables::Layer::new("1轮廓实线层"));
        assert!(!is_new_blank_drawing(&with_layer, None), "有 OCSM 图层 → 旧图");

        let mut with_style = acadrust::CadDocument::new();
        with_style
            .text_styles
            .add_or_replace(acadrust::tables::TextStyle::new("OCSM_GB"));
        assert!(!is_new_blank_drawing(&with_style, None), "有 OCSM_GB 文字样式 → 旧图");

        let mut with_dim = acadrust::CadDocument::new();
        with_dim
            .dim_styles
            .add_or_replace(acadrust::tables::DimStyle::new("OCSM_GB"));
        assert!(!is_new_blank_drawing(&with_dim, None), "有 OCSM_GB 标注样式 → 旧图");

        let mut custom_layer = acadrust::CadDocument::new();
        custom_layer
            .layers
            .add_or_replace(acadrust::tables::Layer::new("用户自建层"));
        assert!(
            is_new_blank_drawing(&custom_layer, None),
            "只有自定义层不算 OCSM 痕迹"
        );
    }

    /// 空文档上跑任意普通命令 → 自动初始化，且 ensure_* 的 defs 与顺序和
    /// 手动 `OCSM` **逐项一致**（复用同一 `init_ocsm_tables`，不复制产物）。
    #[test]
    fn auto_init_empty_new_drawing_matches_manual_ocsm() {
        // `refresh_doc_save_state` 写进程级 DOC_SAVE_STATE：与其它全局态测试串行。
        let _g = global_state_test_lock();
        let mut auto = UndoOrderSpy::default();
        assert!(
            !OcsmPlugin.dispatch(&mut auto, "ZOOM"),
            "自动初始化不吞命令（ZOOM 仍由宿主执行）"
        );

        let mut manual = UndoOrderSpy::default();
        assert!(OcsmPlugin.dispatch(&mut manual, "OCSM"));

        // 产物逐项一致：同一个撤销点 + 同四张表的确保序列。
        assert_eq!(auto.undo, manual.undo);
        assert_eq!(auto.linetypes, manual.linetypes);
        assert_eq!(auto.layers, manual.layers);
        assert_eq!(auto.text_styles, manual.text_styles);
        assert_eq!(auto.dim_styles, manual.dim_styles);
        assert_eq!(auto.outputs, manual.outputs);
        assert_eq!(auto.outputs.len(), 1, "完成行只报一次");
        assert_eq!(auto.infos.len(), 1, "自动路径给一条来源提示");
        assert!(manual.infos.is_empty(), "手动 OCSM 不应报自动初始化提示");
        assert_eq!(auto.undo, vec!["OCSM 初始化"]);

        // 实际落表也一致（桩按宿主 ensure 口径真的写表）。
        let layer_names = |d: &ocs_plugin_api::host::CadDocument| {
            let mut v: Vec<String> = d.layers.iter().map(|l| l.name.clone()).collect();
            v.sort();
            v
        };
        assert_eq!(layer_names(&auto.doc), layer_names(&manual.doc));
        assert_eq!(
            auto.doc.header.current_dimstyle_name,
            manual.doc.header.current_dimstyle_name
        );

        // 关键产物断言（图层/样式/状态位）。
        let names: Vec<&str> = auto.layers.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "1轮廓实线层", "2细线层", "3中心线层", "4虚线层", "5剖面线层",
                "6文字层", "7标注层", "8符号标注层", "9双点划线层", "10引导线层",
            ]
        );
        let center = auto.layers.iter().find(|d| d.name == "3中心线层").unwrap();
        assert_eq!(center.linetype, "CENTER2", "中心线层必须挂点划线");
        let guide = auto.layers.iter().find(|d| d.name == "10引导线层").unwrap();
        assert!(guide.off && guide.lineweight == LineWeight::from_value(0), "引导线层默认关、0mm");
        let ts = &auto.text_styles[0];
        assert_eq!(ts.name, "OCSM_GB");
        assert_eq!((ts.height, ts.width_factor), (3.5, 0.7));
        let ds = &auto.dim_styles[0];
        assert_eq!(ds.name, "OCSM_GB");
        assert!(ds.make_current, "初始化后 OCSM_GB 应为当前标注样式（状态位）");
        // 线型先于图层：DWG 写出时层引用的线型名必须已在表里。
        assert_eq!(auto.log.first().map(String::as_str), Some("push_undo:OCSM 初始化"));
        assert_eq!(auto.log.get(1).map(String::as_str), Some("ensure_linetypes"));
        assert_eq!(auto.log.get(2).map(String::as_str), Some("ensure_layers"));
    }

    /// 旧图/有内容的图一律不动：含「有内容无 OCSM」与「空白但存过盘」两类。
    #[test]
    fn auto_init_leaves_old_or_non_empty_documents_alone() {
        let _g = global_state_test_lock();
        // 有一点内容但不含 OCSM 内容的旧图（无路径）。
        let mut dirty_old = UndoOrderSpy::default();
        let _ = dirty_old
            .doc
            .add_entity(EntityType::Point(acadrust::entities::Point::new()));
        assert!(!OcsmPlugin.dispatch(&mut dirty_old, "ZOOM"));
        assert!(dirty_old.undo.is_empty() && dirty_old.layers.is_empty());
        assert!(dirty_old.infos.is_empty());

        // 空白但保存/另存过的旧图（有路径）。
        let mut saved_empty = UndoOrderSpy::default();
        saved_empty.path = Some(std::path::PathBuf::from("/tmp/old_empty.dwg"));
        assert!(!OcsmPlugin.dispatch(&mut saved_empty, "ZOOM"));
        assert!(saved_empty.undo.is_empty() && saved_empty.layers.is_empty());

        // 空但有 OCSM 图层的旧图（初始化过又清空实体）。
        let mut initialized = UndoOrderSpy::default();
        initialized
            .doc
            .layers
            .add_or_replace(acadrust::tables::Layer::new("1轮廓实线层"));
        assert!(!OcsmPlugin.dispatch(&mut initialized, "ZOOM"));
        assert!(initialized.undo.is_empty() && initialized.layers.is_empty());
    }

    /// 幂等（触发两次 == 一次）+ 生命周期命令/手动 `OCSM` 不叠加。
    #[test]
    fn auto_init_is_idempotent_and_skips_lifecycle_commands() {
        let _g = global_state_test_lock();
        let mut host = UndoOrderSpy::default();
        assert!(!OcsmPlugin.dispatch(&mut host, "ZOOM"));
        assert!(!OcsmPlugin.dispatch(&mut host, "LINE"));
        assert_eq!(host.undo, vec!["OCSM 初始化"], "两次触发只应一个撤销点");
        assert_eq!(host.outputs.len(), 1);
        assert_eq!(host.infos.len(), 1);
        assert_eq!(host.layers.len(), 10, "第二次不应重发 defs");

        // 生命周期/帮助/设置类命令（欢迎页也走插件的那些）不触发。
        let mut life = UndoOrderSpy::default();
        for cmd in ["NEW", "OPEN", "QUIT", "HELP", "OPTIONS", "OPEN_RECENT:/tmp/x.dwg"] {
            assert!(!OcsmPlugin.dispatch(&mut life, cmd), "{cmd}");
        }
        assert!(life.undo.is_empty() && life.layers.is_empty(), "生命周期命令不初始化");

        // 手动 `OCSM`：只跑一次初始化，不叠加自动路径（不双撤销点）。
        let mut manual = UndoOrderSpy::default();
        assert!(OcsmPlugin.dispatch(&mut manual, "OCSM"));
        assert_eq!(manual.undo, vec!["OCSM 初始化"]);
        assert!(
            manual.outputs.len() == 1 && manual.infos.is_empty(),
            "手动 OCSM 只报完成行"
        );
    }

    // ── 通知路径：建图/切图通知 → 不等命令自动初始化 ──────────────────────

    /// 通知路径测试用宿主替身：模拟 DocumentPath/DocumentSnapshot + ensure_* 落表，
    /// 记录请求序列、路由 tab、收到的 defs（与手动路径逐项对比）。
    #[derive(Default)]
    struct AutoInitSender {
        doc: std::sync::Mutex<ocs_plugin_api::host::CadDocument>,
        path: std::sync::Mutex<Option<std::path::PathBuf>>,
        reqs: std::sync::Mutex<Vec<String>>,
        routed_tabs: std::sync::Mutex<Vec<Option<u64>>>,
        infos: std::sync::Mutex<Vec<String>>,
        outputs: std::sync::Mutex<Vec<String>>,
        linetypes: std::sync::Mutex<Vec<ocs_plugin_api::host::LinetypeDef>>,
        layers: std::sync::Mutex<Vec<ocs_plugin_api::host::LayerDef>>,
        text_styles: std::sync::Mutex<Vec<ocs_plugin_api::host::TextStyleDef>>,
        dim_styles: std::sync::Mutex<Vec<ocs_plugin_api::host::DimStyleDef>>,
    }

    impl AutoInitSender {
        fn req_names(&self) -> Vec<String> {
            self.reqs.lock().unwrap().clone()
        }
        fn with_doc(doc: ocs_plugin_api::host::CadDocument) -> std::sync::Arc<Self> {
            let s = AutoInitSender::default();
            *s.doc.lock().unwrap() = doc;
            std::sync::Arc::new(s)
        }
    }

    impl PluginRequestSender for AutoInitSender {
        fn request(
            &self,
            req: ocs_plugin_api::ipc::protocol::PluginRequest,
        ) -> Result<
            ocs_plugin_api::ipc::protocol::PluginResponse,
            ocs_plugin_api::host::PluginRequestError,
        > {
            // 普通路径不带 tab（宿主按“当前图”处理）；通知路径才会覆盖。
            self.request_for_tab(None, req)
        }

        fn request_for_tab(
            &self,
            tab_id: Option<u64>,
            req: ocs_plugin_api::ipc::protocol::PluginRequest,
        ) -> Result<
            ocs_plugin_api::ipc::protocol::PluginResponse,
            ocs_plugin_api::host::PluginRequestError,
        > {
            use ocs_plugin_api::ipc::protocol::{PluginRequest as R, PluginResponse as P};
            self.routed_tabs.lock().unwrap().push(tab_id);
            match req {
                R::DocumentPath { .. } => Ok(P::DocumentPath(
                    self.path
                        .lock()
                        .unwrap()
                        .clone()
                        .map(|p| p.into_os_string()),
                )),
                R::DocumentSnapshot => Ok(P::Document(Box::new(self.doc.lock().unwrap().clone()))),
                R::PushInfo(msg) => {
                    self.reqs.lock().unwrap().push("push_info".into());
                    self.infos.lock().unwrap().push(msg);
                    Ok(P::Ok)
                }
                R::PushOutput(msg) => {
                    self.reqs.lock().unwrap().push("push_output".into());
                    self.outputs.lock().unwrap().push(msg);
                    Ok(P::Ok)
                }
                R::PushUndo { label } => {
                    self.reqs.lock().unwrap().push(format!("push_undo:{label}"));
                    Ok(P::Ok)
                }
                R::EnsureLinetypes(defs) => {
                    self.reqs.lock().unwrap().push("ensure_linetypes".into());
                    let mut created = 0;
                    for def in &defs {
                        if !self
                            .doc
                            .lock()
                            .unwrap()
                            .line_types
                            .iter()
                            .any(|l| l.name.eq_ignore_ascii_case(&def.name))
                        {
                            self.doc
                                .lock()
                                .unwrap()
                                .line_types
                                .add_or_replace(acadrust::tables::LineType::new(&def.name));
                            created += 1;
                        }
                    }
                    self.linetypes.lock().unwrap().extend(defs);
                    Ok(P::Count(created))
                }
                R::EnsureLayers(defs) => {
                    self.reqs.lock().unwrap().push("ensure_layers".into());
                    let mut created = 0;
                    for def in &defs {
                        if !self
                            .doc
                            .lock()
                            .unwrap()
                            .layers
                            .iter()
                            .any(|l| l.name.eq_ignore_ascii_case(&def.name))
                        {
                            self.doc
                                .lock()
                                .unwrap()
                                .layers
                                .add_or_replace(acadrust::tables::Layer::new(&def.name));
                            created += 1;
                        }
                    }
                    self.layers.lock().unwrap().extend(defs);
                    Ok(P::Count(created))
                }
                R::EnsureTextStyles(defs) => {
                    self.reqs.lock().unwrap().push("ensure_text_styles".into());
                    let mut created = 0;
                    for def in &defs {
                        if !self
                            .doc
                            .lock()
                            .unwrap()
                            .text_styles
                            .iter()
                            .any(|s| s.name.eq_ignore_ascii_case(&def.name))
                        {
                            self.doc
                                .lock()
                                .unwrap()
                                .text_styles
                                .add_or_replace(acadrust::tables::TextStyle::new(&def.name));
                            created += 1;
                        }
                    }
                    self.text_styles.lock().unwrap().extend(defs);
                    Ok(P::Count(created))
                }
                R::EnsureDimStyles(defs) => {
                    self.reqs.lock().unwrap().push("ensure_dim_styles".into());
                    let mut created = 0;
                    for def in &defs {
                        if !self
                            .doc
                            .lock()
                            .unwrap()
                            .dim_styles
                            .iter()
                            .any(|s| s.name.eq_ignore_ascii_case(&def.name))
                        {
                            self.doc
                                .lock()
                                .unwrap()
                                .dim_styles
                                .add_or_replace(acadrust::tables::DimStyle::new(&def.name));
                            created += 1;
                        }
                        if def.make_current {
                            self.doc.lock().unwrap().header.current_dimstyle_name =
                                def.name.clone();
                        }
                    }
                    self.dim_styles.lock().unwrap().extend(defs);
                    Ok(P::Count(created))
                }
                other => Ok(P::Error(format!("AutoInitSender 未实现 {other:?}"))),
            }
        }
    }

    /// 通知路径三门与命令入口同口径：空新图（含宿主默认块表/自定义层）→ 初始化；
    /// 有用户图元 / 有 OCSM 痕迹 / 已存盘（有路径）→ 不动。
    #[test]
    fn notification_auto_init_gates_match_command_path() {
        // 空新图（宿主给新图塞了默认内容：Model/Paper 块表 + 用户自建层）→ 初始化。
        let mut doc = acadrust::CadDocument::new();
        doc.layers
            .add_or_replace(acadrust::tables::Layer::new("用户自建层"));
        let sender = AutoInitSender::with_doc(doc);
        assert!(
            auto_init_document_via_sender(&*sender, 42),
            "空新图（有默认块表/自定义层）也要初始化"
        );
        let routed = sender.routed_tabs.lock().unwrap().clone();
        assert!(
            !routed.is_empty() && routed.iter().all(|t| *t == Some(42)),
            "通知路径的每个请求都必须路由到通知里的 tab：{routed:?}"
        );
        let names = sender.req_names();
        assert_eq!(
            names,
            vec![
                "push_info",
                "push_undo:OCSM 初始化",
                "ensure_linetypes",
                "ensure_layers",
                "ensure_text_styles",
                "ensure_dim_styles",
                "push_output",
            ],
            "序列必须与手动 OCSM 一致（撤销点在最前、线型先于图层）"
        );
        assert_eq!(
            sender.layers.lock().unwrap().len(),
            10,
            "10 个 OCSM 图层一个不能少"
        );

        // 有用户图元 → 不动（且不建表）。
        let mut dirty = acadrust::CadDocument::new();
        let _ = dirty.add_entity(EntityType::Point(acadrust::entities::Point::new()));
        let dirty = AutoInitSender::with_doc(dirty);
        assert!(!auto_init_document_via_sender(&*dirty, 1));
        assert!(dirty.layers.lock().unwrap().is_empty(), "旧图不得被建表");
        assert!(dirty.req_names().is_empty(), "旧图连撤销点都不该有");

        // 已有 OCSM 图层 → 幂等不动。
        let mut initialized = acadrust::CadDocument::new();
        initialized
            .layers
            .add_or_replace(acadrust::tables::Layer::new("1轮廓实线层"));
        let initialized = AutoInitSender::with_doc(initialized);
        assert!(!auto_init_document_via_sender(&*initialized, 1));

        // 存过盘（哪怕空）→ 连快照都不取（路径门先短路）。
        let saved = AutoInitSender::with_doc(acadrust::CadDocument::new());
        *saved.path.lock().unwrap() = Some(std::path::PathBuf::from("/tmp/old_empty.dwg"));
        assert!(!auto_init_document_via_sender(&*saved, 1));
        assert_eq!(
            saved.routed_tabs.lock().unwrap().len(),
            1,
            "有路径应第一道门（DocumentPath）就返回，不取快照"
        );
    }

    /// 通知路径产物与手动 `OCSM` 逐项一致（同序列、同 defs、同文案）。
    #[test]
    fn notification_auto_init_matches_manual_ocsm() {
        // `dispatch` 里 `refresh_doc_save_state` 写进程级状态：与全局态测试串行。
        let _g = global_state_test_lock();
        let sender = AutoInitSender::with_doc(acadrust::CadDocument::new());
        assert!(auto_init_document_via_sender(&*sender, 7));

        let mut manual = UndoOrderSpy::default();
        assert!(OcsmPlugin.dispatch(&mut manual, "OCSM"));

        // 同序列。
        let names = sender.req_names();
        assert_eq!(
            names[1..6].to_vec(),
            vec![
                "push_undo:OCSM 初始化",
                "ensure_linetypes",
                "ensure_layers",
                "ensure_text_styles",
                "ensure_dim_styles",
            ]
        );
        assert_eq!(manual.log.get(0..5), Some(&names[1..6]), "与手动日志逐项一致");

        // 同 defs。
        assert_eq!(*sender.layers.lock().unwrap(), manual.layers);
        assert_eq!(*sender.linetypes.lock().unwrap(), manual.linetypes);
        assert_eq!(*sender.text_styles.lock().unwrap(), manual.text_styles);
        assert_eq!(*sender.dim_styles.lock().unwrap(), manual.dim_styles);

        // 同文案（来源提示 + 完成行）。
        assert_eq!(*sender.infos.lock().unwrap(), vec![AUTO_INIT_INFO.to_string()]);
        assert_eq!(*sender.outputs.lock().unwrap(), manual.outputs);
        assert_eq!(manual.infos.len(), 0, "手动 OCSM 不报自动来源提示");

        // 幂等：同一张图再查一次 → 门（已有 OCSM 痕迹）拦住，不再落任何请求。
        let before = sender.routed_tabs.lock().unwrap().len();
        assert!(!auto_init_document_via_sender(&*sender, 7));
        assert_eq!(
            sender.routed_tabs.lock().unwrap().len(),
            before + 2,
            "第二次只该取路径 + 快照即被 OCSM 痕迹门拦住"
        );
    }

    /// 旧图救字体（用户 2026-09-28）：打开旧图（`OCSM_GB` 已在但 `true_type_font` 为空）
    /// → 跑一次 `OCSM` ⇒ 按插件定义补上 `Zhuque Fangsong`（并报一句提示）。
    #[test]
    fn ocsm_init_fills_missing_ocsm_gb_true_type_font() {
        let mut host = UndoOrderSpy::default();
        host.doc
            .text_styles
            .add_or_replace(acadrust::tables::TextStyle::new("OCSM_GB"));
        assert!(
            host.doc.text_styles.get("OCSM_GB").unwrap().true_type_font.is_empty(),
            "TextStyle::new 的 TTF 应为空（模拟旧图）"
        );
        assert!(OcsmPlugin.dispatch(&mut host, "OCSM"));
        assert_eq!(
            host.doc.text_styles.get("OCSM_GB").unwrap().true_type_font,
            "Zhuque Fangsong"
        );
        assert!(
            host.infos.iter().any(|m| m.contains("TrueType")),
            "补上了就该报一句：{:?}",
            host.infos
        );
    }

    /// 硬规则：同名样式的 `true_type_font` 非空时，**绝不覆盖**（哪怕是别的字体名）。
    #[test]
    fn ocsm_init_never_overwrites_existing_true_type_font() {
        let mut host = UndoOrderSpy::default();
        let mut style = acadrust::tables::TextStyle::new("OCSM_GB");
        style.true_type_font = "MS Gothic".into();
        host.doc.text_styles.add_or_replace(style);
        assert!(OcsmPlugin.dispatch(&mut host, "OCSM"));
        assert_eq!(
            host.doc.text_styles.get("OCSM_GB").unwrap().true_type_font,
            "MS Gothic"
        );
        assert!(
            !host.log.iter().any(|l| l.starts_with("table_operation")),
            "不该动已有字体：{:?}",
            host.log
        );
        assert!(host.infos.is_empty(), "无补齐就不该有提示：{:?}", host.infos);
    }

    /// 通知胶水：欢迎页跳过、同 tab 防重、工作线程真的跑通 `auto_init_document_via_sender`。
    #[test]
    fn notification_glue_skips_boot_tab_and_dedupes() {
        let _g = global_state_test_lock();
        // 1) 启动页（欢迎 Start 页）不动。
        let boot = AutoInitSender::with_doc(acadrust::CadDocument::new());
        {
            let mut st = crate::auto_init_state_lock();
            let boot_sender: std::sync::Arc<dyn PluginRequestSender> = boot.clone();
            st.sender = Some(boot_sender);
            st.boot_tab = Some(7);
            st.checked.clear();
            st.in_flight.clear();
        }
        crate::maybe_auto_init_after_notification(7);
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(boot.req_names().is_empty(), "欢迎页不得被初始化");

        // 2) 新图 tab=9：两次通知只初始化一次。
        let fresh = AutoInitSender::with_doc(acadrust::CadDocument::new());
        {
            let mut st = crate::auto_init_state_lock();
            let fresh_sender: std::sync::Arc<dyn PluginRequestSender> = fresh.clone();
            st.sender = Some(fresh_sender);
            st.boot_tab = Some(7);
            st.checked.clear();
            st.in_flight.clear();
        }
        crate::maybe_auto_init_after_notification(9);
        for _ in 0..100 {
            if fresh.layers.lock().unwrap().len() == 10 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        crate::maybe_auto_init_after_notification(9);
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(fresh.layers.lock().unwrap().len(), 10, "通知路径应完成初始化");
        assert_eq!(fresh.req_names().len(), 7, "两次通知只该跑一轮初始化");

        // 3) 通知路径在途时，命令入口让路（同一张图不叠撤销点）。
        //    UndoOrderSpy 的 tab_id = 0，所以在途标记用 0。
        {
            let mut st = crate::auto_init_state_lock();
            st.in_flight.insert(0);
        }
        let mut collided = UndoOrderSpy::default();
        assert!(!OcsmPlugin.dispatch(&mut collided, "ZOOM"));
        assert!(
            collided.undo.is_empty() && collided.layers.is_empty(),
            "在途初始化期间命令入口不得重复初始化"
        );

        // 清掉全局态，避免影响其它测试。
        let mut st = crate::auto_init_state_lock();
        st.sender = None;
        st.boot_tab = None;
        st.checked.clear();
        st.in_flight.clear();
    }

    /// 计算书命令入口（命令解析层，不必真连宿主）：`OCSMGEAR … report` 与
    /// `OCSMSHAFT SPLINE … report` 把 Markdown 推到输出；`INVOLSPLINE` 轴段已撤，命令同样拒绝。
    #[test]
    fn gear_and_shaft_report_commands_emit_markdown() {
        let mut host = UndoOrderSpy::default();
        OcsmPlugin.cmd_gear(&mut host, "花键 GB30R m=3 z=20 h=30 report");
        assert!(host.errors.is_empty(), "{:?}", host.errors);
        let md = host.outputs.join("\n");
        for needle in ["m·z", "3 × 20", "60 mm", "GB/T 3478.1"] {
            assert!(md.contains(needle), "OCSMGEAR report 缺 `{needle}`：\n{md}");
        }

        let mut host = UndoOrderSpy::default();
        OcsmPlugin.cmd_shaft(&mut host, "SPLINE 6x23x26x6 L30 report");
        assert!(host.errors.is_empty(), "{:?}", host.errors);
        let md = host.outputs.join("\n");
        assert!(md.contains("# 轴段计算书") && md.contains("矩形花键段"), "{md}");
        // 渐开线花键轴段已撤：report 命令同样不再接受 INVOLSPLINE。
        let mut host = UndoOrderSpy::default();
        OcsmPlugin.cmd_shaft(&mut host, "INVOLSPLINE GB30R M3 Z20 L30 report");
        assert!(
            host.errors
                .iter()
                .any(|e| e.contains("INVOLSPLINE") && e.contains("不识别的关键字")),
            "{:?}",
            host.errors
        );
    }
    // ── 标准件放置：两段式（定位基点 → 绕基点旋转 → 落定）──────────────────

    /// 记录型 sender（只记请求，不做真实图纸操作）。
    struct RecordingSender {
        reqs: std::sync::Mutex<Vec<String>>,
    }
    impl RecordingSender {
        fn new() -> Self {
            RecordingSender { reqs: std::sync::Mutex::new(Vec::new()) }
        }
        fn names(&self) -> Vec<String> {
            self.reqs.lock().unwrap().clone()
        }
    }
    impl PluginRequestSender for RecordingSender {
        fn request(
            &self,
            req: ocs_plugin_api::ipc::protocol::PluginRequest,
        ) -> Result<ocs_plugin_api::ipc::protocol::PluginResponse, ocs_plugin_api::host::PluginRequestError>
        {
            use ocs_plugin_api::ipc::protocol::{PluginRequest as R, PluginResponse as P};
            let name = match &req {
                R::AddEntities(v) => {
                    // 记下 INSERT 的旋转角（弧度）便于断言
                    let rot = v.iter().find_map(|e| match e {
                        acadrust::EntityType::Insert(i) => Some(i.rotation),
                        _ => None,
                    });
                    self.reqs
                        .lock()
                        .unwrap()
                        .push(format!("AddEntities rot={:.6}", rot.unwrap_or(f64::NAN)));
                    return Ok(P::Handles(vec![acadrust::Handle::NULL]));
                }
                R::AddBlockRecord { name, .. } => format!("AddBlockRecord {name}"),
                R::WriteRecord { .. } => "WriteRecord".to_string(),
                R::BumpGeometry => "BumpGeometry".to_string(),
                R::SetDirty => "SetDirty".to_string(),
                other => format!("{other:?}"),
            };
            self.reqs.lock().unwrap().push(name);
            Ok(P::Ok)
        }
    }

    #[test]
    fn rotate_angle_follows_cursor_direction() {
        let b = [0.0, 0.0, 0.0];
        assert!(rotate_angle(b, [10.0, 0.0, 0.0]).abs() < 1e-12, "向右 = 0°");
        assert!((rotate_angle(b, [0.0, 10.0, 0.0]) - std::f64::consts::FRAC_PI_2).abs() < 1e-12, "向上 = +90°");
        assert!((rotate_angle(b, [-10.0, 0.0, 0.0]).abs() - std::f64::consts::PI).abs() < 1e-12, "向左 = 180°");
        assert!((rotate_angle(b, [0.0, -10.0, 0.0]) + std::f64::consts::FRAC_PI_2).abs() < 1e-12, "向下 = −90°");
        assert!((rotate_angle(b, [5.0, 5.0, 0.0]) - std::f64::consts::FRAC_PI_4).abs() < 1e-12, "45°");
        assert_eq!(rotate_angle(b, [0.2, 0.2, 0.0]), 0.0, "贴近基点不抖动");
        // 基点不在原点也按"基点→光标"方向算
        assert!((rotate_angle([3.0, 4.0, 0.0], [8.0, 9.0, 0.0]) - std::f64::consts::FRAC_PI_4).abs() < 1e-12, "平移基点");
    }

    #[test]
    fn part_place_is_two_stage_with_rotation_preview() {
        let _g = global_state_test_lock();   // PENDING_PART 进程级全局（与导出测试共用一把锁）
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        let rec = std::sync::Arc::new(RecordingSender::new());
        let sender: std::sync::Arc<dyn PluginRequestSender> = rec.clone();
        set_pending_part(PendingPart::new("OCSM_TEST", "{}", "测试件 M10x40"));
        let mut cmd = PartPlace {
            sender: sender.clone(),
            phase: std::cell::Cell::new(PlacePhase::Follow),
            base: std::cell::Cell::new([0.0, 0.0, 0.0]),
        what: "OCSM 标准件".to_string(),
        where_to: "请在零件库窗口里点「零件出库」".to_string(),
        };
        // 未定位：预览跟光标（旋转 0）
        let prev = cmd.on_mouse_move([7.0, 8.0, 0.0]).expect("预览");
        match prev {
            acadrust::EntityType::Insert(i) => {
                assert_eq!((i.insert_point.x, i.insert_point.y), (7.0, 8.0));
                assert_eq!(i.rotation, 0.0);
                assert_eq!(i.block_name, "OCSM_TEST");
            }
            other => panic!("预览应为 INSERT：{other:?}"),
        }
        assert!(cmd.prompt().contains("点击定位基点"), "提示：{}", cmd.prompt());
        // 第一下：定位基点 → 转入旋转阶段
        assert_eq!(std::mem::discriminant(&cmd.on_point([2.0, 3.0, 0.0])),
                   std::mem::discriminant(&CommandStep::NeedPoint));
        assert!(cmd.prompt().contains("绕基点旋转"), "提示：{}", cmd.prompt());
        // 移动光标：预览停在基点、绕基点旋转（光标在 +y → 90°）
        let prev = cmd.on_mouse_move([2.0, 13.0, 0.0]).expect("旋转预览");
        match prev {
            acadrust::EntityType::Insert(i) => {
                assert_eq!((i.insert_point.x, i.insert_point.y), (2.0, 3.0), "预览吸附到基点");
                assert!((i.rotation - std::f64::consts::FRAC_PI_2).abs() < 1e-12, "旋转 90°，实际 {}", i.rotation);
            }
            other => panic!("预览应为 INSERT：{other:?}"),
        }
        // 第二下：落定（插入交给 worker；这里直接同步调用 place_one 验证请求内容）
        let pending = PendingPart::new("OCSM_TEST", "{}", "测试件");
        place_one(&sender, &pending, PlaceTask { pt: [2.0, 3.0, 0.0], rotation: std::f64::consts::FRAC_PI_2, sender: sender.clone() })
            .expect("落件");
        let names = rec.names();
        assert!(names.iter().any(|n| n.contains("AddEntities") && n.contains("rot=1.570796")), "{names:?}");
        assert!(names.contains(&"WriteRecord".to_string()), "{names:?}");
        assert!(names.contains(&"SetDirty".to_string()), "{names:?}");
        // 落定后回到跟随阶段，可连续放置
        set_pending_part(PendingPart::new("OCSM_TEST", "{}", "测试件"));
        let mut cmd2 = PartPlace {
            sender,
            phase: std::cell::Cell::new(PlacePhase::Follow),
            base: std::cell::Cell::new([0.0, 0.0, 0.0]),
            what: "OCSM 标准件".to_string(),
            where_to: "请在零件库窗口里点「零件出库」".to_string(),
        };
        let _ = cmd2.on_point([1.0, 1.0, 0.0]);
        assert!(cmd2.prompt().contains("绕基点旋转"));
        let _ = cmd2.on_point([1.0, 2.0, 0.0]);
        assert!(cmd2.prompt().contains("点击定位基点"), "落定后回到跟随：{}", cmd2.prompt());
    }

    #[test]
    fn parse_session_env_picks_display_keys() {
        let sample = "LANG=zh_CN.UTF-8\nDISPLAY=:0\nWAYLAND_DISPLAY=wayland-0\nXAUTHORITY=/run/user/1000/xauth_uAXGEA\nXDG_SESSION_TYPE=wayland\nPATH=/usr/bin\nEMPTY=\n";
        let got = parse_session_env(sample);
        let get = |k: &str| got.iter().find(|(kk, _)| kk == k).map(|(_, v)| v.as_str());
        assert_eq!(get("DISPLAY"), Some(":0"));
        assert_eq!(get("WAYLAND_DISPLAY"), Some("wayland-0"));
        assert_eq!(get("XAUTHORITY"), Some("/run/user/1000/xauth_uAXGEA"));
        assert_eq!(get("PATH"), None, "无关变量不入表");
        assert_eq!(get("EMPTY"), None, "空值不入表");
        // 去重：同名只保留第一个
        assert_eq!(parse_session_env("DISPLAY=:1\nDISPLAY=:2\n").len(), 1);
    }

    // ── 引导服务器：请求必须跟着“当前图纸”走（2026-09-16 GDIM bug）──────

    /// 记下自己被要求过几次（用来断言请求落到了哪张图的 sender）。
    struct TabSender {
        asks: std::sync::Mutex<Vec<String>>,
    }
    impl TabSender {
        fn new() -> Self {
            TabSender { asks: std::sync::Mutex::new(Vec::new()) }
        }
        fn asks(&self) -> Vec<String> {
            self.asks.lock().unwrap().clone()
        }
    }
    impl PluginRequestSender for TabSender {
        fn request(
            &self,
            req: ocs_plugin_api::ipc::protocol::PluginRequest,
        ) -> Result<ocs_plugin_api::ipc::protocol::PluginResponse, ocs_plugin_api::host::PluginRequestError>
        {
            self.asks.lock().unwrap().push(format!("{req:?}"));
            Ok(ocs_plugin_api::ipc::protocol::PluginResponse::Ok)
        }
    }

    /// **回归：请求跟着“当前活跃图纸”走**（用户报的 bug + 后续加固）。
    ///
    /// 插件进程比一张图纸活得久：端口进程级复用，但 sender 绑定标签页。旧写法把
    /// 首次 spawn 的 sender 固定进服务器线程，于是「关掉当前图纸 → 新建一张 → GDIM」
    /// 之后 HTTP 侧还在对着已经关闭的标签页发请求（宿主不回答 → 5 s 超时 →
    /// GUI 读不到引导线）。现在：带 `tab=` 的窗口对号入座；不带 `tab=` 的跟宿主通知
    /// 的活跃图纸；目标图没了/没登记过就明确报错，不猜着写到别的图上。
    #[test]
    fn guide_router_follows_the_active_drawing() {
        let s1 = std::sync::Arc::new(TabSender::new());
        let s2 = std::sync::Arc::new(TabSender::new());
        let s3 = std::sync::Arc::new(TabSender::new());
        let tab1: std::sync::Arc<dyn PluginRequestSender> = s1.clone();
        let tab2: std::sync::Arc<dyn PluginRequestSender> = s2.clone();
        let tab3: std::sync::Arc<dyn PluginRequestSender> = s3.clone();
        // 图 1 跑过命令 → 图 1 被关掉、新建图 2 又跑命令（dispatch 里走的就是这两步）。
        let router = guide_router(Some(1), tab1);
        guide_router(Some(2), tab2);

        let ask = |target: &str, body: &[u8]| -> Result<(), String> {
            let sender = router.resolve(target, body)?;
            sender
                .request(ocs_plugin_api::ipc::protocol::PluginRequest::BumpGeometry)
                .unwrap();
            Ok(())
        };

        // ① 无 tab（AI/MCP、新窗口）→ 当前图（最近命令所在）图 2
        ask("/api/guide?handle=2A", b"").expect("无 tab 请求");
        // ② 老窗口（tab=1）→ 图 1；POST body 里的 tab 同样生效
        ask("/api/guide?handle=2A&tab=1", b"").expect("tab=1 请求");
        ask("/api/apply", br#"{"handle":"2A","url":"/DIM/LINEAR/H/10","tab":1}"#).expect("带 tab 的 POST");
        // ③ 宿主通知“活跃图纸换成图 3”（只切标签页、不跑命令）——图 3 还没跑过命令
        //    → 明确报错，不猜着用图 2。
        crate::on_host_notification(&HostNotification::SelectionChangedV4 {
            tab_id: 3,
            handles: Vec::new(),
        });
        let err = ask("/api/guide?handle=2A", b"").expect_err("活跃图未登记应报错");
        assert!(err.contains("还没跑过 OCSM 命令"), "{err}");
        // ④ 图 3 跑过命令（dispatch 里 set_current(3, …)）后，无 tab 的请求就走图 3。
        guide_router(Some(3), tab3);
        ask("/api/guide?handle=2A", b"").expect("活跃图已登记");
        // ⑤ 图 2 被关掉（宿主通知）→ 它的窗口请求立即报错（不超时、不串到图 3）。
        crate::on_host_notification(&HostNotification::DocumentTabClosed { tab_id: 2 });
        let err = ask("/api/guide?handle=2A&tab=2", b"").expect_err("已关图纸的窗口应报错");
        assert!(err.contains("已经关"), "{err}");
        // ⑥ 没登记过的 tab（插件重启前的老页面 / 乱填）→ 报错，不落到别的图。
        let err = ask("/api/guide?handle=2A&tab=99", b"").expect_err("未登记 tab 应报错");
        assert!(err.contains("重新打开窗口"), "{err}");

        assert_eq!(s1.asks().len(), 2, "图 1 只接带 tab=1 的请求：{:?}", s1.asks());
        assert_eq!(s2.asks().len(), 1, "图 2 只接①那次（后来被关且活跃图已换）：{:?}", s2.asks());
        assert_eq!(s3.asks().len(), 1, "图 3 接④那次：{:?}", s3.asks());
    }
}
