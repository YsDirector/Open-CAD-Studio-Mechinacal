//! 双语机制（中 / 英）：**单一 message catalog** + 取词 API + 语言来源。
//!
//! 用户 2026-09-27 定案（分阶段：本阶段只立骨架 + 打通一条竖切，不做全量翻译）：
//! * 卡定义只有一份，**卡面标签随语言渲染**（禁止给每张卡再复制一份英文卡）；
//!   标准符号（`D_ei` / `Az` / `Wn` / `Kn` / `d_B` …）保持原样不译。
//! * 单一 catalog（表驱动，**一行一 key**：`zh` / `en`），按域分节
//!   （命令输出 / 报错 / GUI / 卡面标签 / 计算书 / 手册）。
//! * 语言 = **跟随宿主 + 可手动覆盖**：宿主 API（`ocs_plugin_api`）当前**没有**
//!   locale/language 字段（已核对 HostApi 全部方法），因此用环境变量兜底 + 程序内
//!   覆盖；一旦宿主暴露语言设置，只需改 [`resolve_env_lang`] 一处。
//!
//! # 语言来源（优先级从高到低）
//!
//! 1. 程序内 [`set_lang`]（GUI 设置 / 命令开关 / 测试；可 [`set_lang_auto`] 复位）；
//! 2. 环境变量 [`LANG_ENV`]（`OCSMLANG=zh|en`，用户/运维手动覆盖）；
//! 3. 别名 [`LANG_ENV_ALIAS`]（`OCSM_LANG`）；
//! 4. `LC_ALL` → `LC_MESSAGES` → `LANG`（宿主/system locale 的代理）；
//! 5. 默认 **`zh`**（保守默认：本仓既有输出全是中文，缺省不改变现行为）。
//!
//! 值接受 `zh` / `zh-CN` / `zh_CN.UTF-8` / `en` / `en_US.UTF-8`（大小写不敏感，
//! 只看 `_` `-` `.` `@` 前的第一段）。
//!
//! # 缺 key 的保守默认（**绝不返回静默空串**）
//!
//! * key 在 catalog 里、目标语言串为空 → **回落 `zh`**，并记入 [`missing_keys`]；
//! * key 完全不在 catalog 里 → 返回 **key 本身**（可见占位，便于一眼定位），
//!   并记入 [`missing_keys`]；
//! * 运行期不 panic（插件里让命令整个挂掉更糟）；缺失由 [`catalog_problems`]
//!   静态测试 + [`missing_keys`] 运行期点名兜住。
//!
//! 其它：`{name}` 占位符用 [`t_fmt`] 插值；catalog 完整性（两语齐全 / 无重复 key /
//! 占位符两语一致）由 [`catalog_problems`] 覆盖，见本文件底部单测。
//!
// 阶段 1 骨架：语言切换/诊断/缺 key 点名等 API 是给第二阶段接线与联调用的，
// 产品路径目前只用了 t/t_fmt；先允许 dead_code，不给构建加新警告。
#![allow(dead_code)]

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;

/// 已支持的语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    /// 稳定语言代号（下发 GUI / 写日志用）。
    pub fn code(self) -> &'static str {
        match self {
            Lang::Zh => "zh",
            Lang::En => "en",
        }
    }

    /// 人类侧显示名（本身也走 catalog，随当前语言）。
    pub fn label(self) -> String {
        t(match self {
            Lang::Zh => "meta.lang.zh",
            Lang::En => "meta.lang.en",
        })
    }
}

/// 一条词条：**一行一 key：`zh` / `en`**。
#[derive(Debug, Clone, Copy)]
pub struct Msg {
    /// 稳定 key（`域.对象.用途`，如 `cmd.tf.usage`）。
    pub key: &'static str,
    /// 中文（缺省语言；本阶段字段名照「先立骨架」口径保留）。
    pub zh: &'static str,
    /// 英文。
    pub en: &'static str,
}

impl Msg {
    const fn new(key: &'static str, zh: &'static str, en: &'static str) -> Self {
        Self { key, zh, en }
    }
}

// ───────────────────────── catalog（按域分节） ─────────────────────────
//
// 本阶段只登记「已接线」的词条（竖切域）与语言元信息；其余全量字符串先盘点
// （`~/桌面/OCSM/review/i18n_盘点.md`），按域分批录入，**列在这里的必须真有人用**。

/// **GUI 深层元数据字典**（零件库/结构要素目录、表单标签、来源说明等）。
///
/// 这些字符串是 `partgen*` / `detail` 目录 JSON 里的**显示元数据**：中文原值即「数据」本身
/// （族定义就地写死），英文按本表查；`t_data` 在 `Lang::Zh` 下原样返回，`Lang::En` 下查表。
/// **协议记号（族 id / 代号 / 标准号 / 文件名）不在表内**，原样透传。
pub const DATA_CATALOG: &[(&str, &str)] = &[
    ("基点 = 头部支承面 × 轴线", "Base point = head bearing face × axis"),
    ("基点 = 头部支承面 × 轴线（画法同 GB/T 5780 C级）", "Base point = head bearing face × axis (drawn the same as GB/T 5780 grade C)"),
    ("基点 = 端面中心（轴线）", "Base point = end-face centre (axis)"),
    ("基点 = 端面中心 × 轴线（主视图：轴线 y=0、端面 x=0）", "Base point = end-face centre × axis (front view: axis y=0, end face x=0)"),
    ("基点 = 端面中心 × 轴线（剖面：轴线 x=0、端面 y=0）", "Base point = end-face centre × axis (section: axis x=0, end face y=0)"),
    ("基点 = 端面 × 轴线", "Base point = end face × axis"),
    ("基点 = 端面（d1 凸台面）× 轴线", "Base point = end face (d1 boss face) × axis"),
    ("基点 = 内六角端端面 × 轴线", "Base point = socket end face × axis"),
    ("基点 = 垫圈端面中心（轴线）", "Base point = washer end-face centre (axis)"),
    ("基点 = 支承面 × 轴线（环心在 +y 侧、杆沿 −y）", "Base point = bearing face × axis (ring centre on the +y side, shank along −y)"),
    ("基点 = 左端面 × 轴线", "Base point = left end face × axis"),
    ("基点 = 左端面（c 锥端）× 轴线", "Base point = left end face (c chamfer end) × axis"),
    ("长度 l", "Length l"),
    ("长度 L", "Length L"),
    ("厚度 s", "Thickness s"),
    ("厚度 T", "Thickness T"),
    ("厚度 h", "Thickness h"),
    ("高度 m", "Height m"),
    ("宽度 B", "Width B"),
    ("螺纹长度 l", "Thread length l"),
    ("外径 D（同轴径变体）", "Outside diameter D (same-shaft-diameter variant)"),
    ("宽度 B（同内径变体）", "Width B (same-bore variant)"),
    ("b1（mm，留空 = 该 d 档默认）", "b1 (mm, blank = default for that d step)"),
    ("毂长 len（mm，默认 30）", "Hub length len (mm, default 30)"),
    ("主视图", "Front view"),
    ("俯视图", "Top view"),
    ("左视图", "Left view"),
    ("A型（双圆头）", "Type A (double round head)"),
    ("B型（双平头）", "Type B (double flat head)"),
    ("C型（单圆头）", "Type C (single round head)"),
    ("零件库", "Part library"),
    ("螺栓", "Bolts"),
    ("螺钉", "Screws"),
    ("螺母", "Nuts"),
    ("垫圈", "Washers"),
    ("销", "Pins"),
    ("挡圈", "Retaining rings"),
    ("内六角", "Socket head"),
    ("六角螺栓", "Hex bolts"),
    ("六角螺母", "Hex nuts"),
    ("平垫圈", "Plain washers"),
    ("弹性垫圈", "Spring washers"),
    ("圆柱销", "Parallel pins"),
    ("六角头螺栓 C级", "Hex head bolt, grade C"),
    ("六角头螺栓 A/B级", "Hex head bolt, grades A/B"),
    ("六角头螺栓 C级 GB/T 5780-2016", "Hex head bolt, grade C GB/T 5780-2016"),
    ("六角头螺栓 A/B级 GB/T 5782-2016", "Hex head bolt, grades A/B GB/T 5782-2016"),
    ("六角头螺栓 全螺纹 GB/T 5783-2016", "Hex head bolt, fully threaded GB/T 5783-2016"),
    ("六角头头部带孔螺栓 GB/T 32.1-2020", "Hex head bolt with hole in head GB/T 32.1-2020"),
    ("内六角圆柱头螺钉 GB/T 70.1-2008", "Hexagon socket head cap screw GB/T 70.1-2008"),
    ("六角螺母 C级 GB/T 41-2016", "Hex nut, grade C GB/T 41-2016"),
    ("六角薄螺母 GB/T 6172.1-2016", "Hex thin nut GB/T 6172.1-2016"),
    ("平垫圈 A级 GB/T 97.1-2002", "Plain washer, grade A GB/T 97.1-2002"),
    ("标准型弹簧垫圈 GB/T 93-2025", "Standard spring washer GB/T 93-2025"),
    ("圆柱销 A型 GB/T 119.1-2000", "Parallel pin, type A GB/T 119.1-2000"),
    ("内螺纹圆柱销 GB/T 120.1-2000", "Parallel pin with internal thread GB/T 120.1-2000"),
    ("1型六角螺母", "Hex nut, style 1"),
    ("孔用弹性挡圈 A型", "Circlip for holes, type A"),
    ("轴用弹性挡圈 A型", "Circlip for shafts, type A"),
    ("内六角平端紧定螺钉", "Hexagon socket set screw with flat point"),
    ("圆螺母", "Round nut"),
    ("圆螺母用止动垫圈", "Tab washer for round nut"),
    ("吊环螺钉 A型", "Lifting eye bolt, type A"),
    ("内包骨架有副唇密封圈 FB型", "Rotary shaft lip seal, metal case with dust lip, type FB"),
    ("深沟球轴承 60000型", "Deep groove ball bearing, 60000 series"),
    ("圆锥滚子轴承 30000型 02系列", "Tapered roller bearing, 30000 series, size series 02"),
    ("调心滚子轴承 20000C型", "Spherical roller bearing, 20000C series"),
    ("内六角平圆头螺钉", "Hexagon socket button head screw"),
    ("内六角花形低圆柱头螺钉", "Hexalobular socket low head cap screw"),
    ("六角头全螺纹螺栓 全螺纹 B级", "Hex head bolt, fully threaded, grade B"),
    ("六角头头部带孔螺栓 A级", "Hex head bolt with hole in head, grade A"),
    ("平垫圈 A级", "Plain washer, grade A"),
    ("标准型弹簧垫圈", "Standard spring washer"),
    ("内六角圆柱头螺钉", "Hexagon socket head cap screw"),
    ("零件库/螺钉/内六角", "Part library/Screws/Socket head"),
        ("普通平键A型", "Parallel key, type A"),
    ("普通平键B型", "Parallel key, type B"),
    ("普通平键C型", "Parallel key, type C"),
    ("导向平键A型", "Gib-head key, type A"),
    ("导向平键B型", "Gib-head key, type B"),
    ("固定键固定在轴上；端置是普通平键 KEY 的画法", "The fixed key stays on the shaft; the end position follows the plain-key KEY drawing"),
("零件库/键/平键", "Part library/Keys/Parallel keys"),
    ("结构要素/砂轮越程槽", "Structural elements/Grinding relief groove"),
    ("结构要素/退刀槽", "Structural elements/Thread relief groove"),
    ("结构要素/毂槽", "Structural elements/Hub keyway"),
    ("结构要素/花键", "Structural elements/Splines"),
    ("螺距 P（mm，必给；表 2 单键）", "Pitch P (mm, required; single value from Table 2)"),
    ("g1 覆盖（mm，留空 = 表值）", "g1 override (mm, blank = table value)"),
    ("g2 覆盖（mm，留空 = 表值）", "g2 override (mm, blank = table value)"),
    ("dg 覆盖（mm，留空 = d − 表值减量）", "dg override (mm, blank = d − table reduction)"),
    ("r 覆盖（mm，留空 = 表值）", "r override (mm, blank = table value)"),
    ("斜壁最小角 α（°，默认 30，必须 ≥30）", "Minimum flank angle α (°, default 30, must be ≥30)"),
    ("毂长 len（mm，留空 = 默认 30）", "Hub length len (mm, blank = default 30)"),
    ("L 满齿段长（mm，侧视/剖视必给）", "L, full-tooth length (mm, required for side/section views)"),
    ("de 滚刀外径覆盖（mm，留空 = 按规格查表）", "de hob outside diameter override (mm, blank = look up by spec)"),
    ("H（内花键基孔制）", "H (internal spline, hole basis)"),
    ("轴径 d（mm，自由输入）", "Shaft diameter d (mm, free input)"),
    ("螺纹公称直径 d（mm，自由输入）", "Nominal thread diameter d (mm, free input)"),
    ("孔径 d（mm，自由输入；b/t₂/r 由 GB/T 1095 查表）", "Hole diameter d (mm, free input; b/t₂/r from GB/T 1095)"),
    ("小径 d（mm；由规格代号派生，自定义规格时才自由输入）", "Minor diameter d (mm; derived from the spec code, free input only for custom specs)"),
    ("GB/T 3-1997（ISO 3508:1976 / ISO 4755:1977）表 2；抄自 164580.com/data/detail_148.html（用户 2026-09-19 核对）", "GB/T 3-1997 (ISO 3508:1976 / ISO 4755:1977) Table 2; copied from 164580.com/data/detail_148.html (checked by the user on 2026-09-19)"),
    ("GB/T 1095-2003 表 1；数据复用 assets/keyway_gb1095.csv（b→t₂/r）+ 1979 d 列；模板 GB-T1095-2003毂槽-{主视图,侧视图}.dxf 逐图元反解", "GB/T 1095-2003 Table 1; data reuses assets/keyway_gb1095.csv (b→t₂/r) + the 1979 d column; template GB-T1095-2003毂槽-{主视图,侧视图}.dxf reverse-engineered entity by entity"),
    ("把手册 md 放到 ~/.agents/skills/ocsm-manual/manual/，或用环境变量 OCSM_MANUAL_DIR 指定目录", "Put the handbook md files under ~/.agents/skills/ocsm-manual/manual/, or point the OCSM_MANUAL_DIR environment variable at a directory"),
    // ② GUI 元数据批补齐项（零件库树/视图名/尺寸下拉/元数据名；带 `{}` 的走 [`t_data_fmt`]）。
    ("六角薄螺母", "Hex thin nut"),
    ("外螺纹退刀槽", "External thread relief groove"),
    ("六角螺母 C级", "Hex nut, grade C"),
    ("基点 = 左端面对称轴（俯视图）/ 左端面×底面（主视图）/ 截面左下角（剖视图）；d 槽位承载 b", "Base point = left end face, symmetry axis (top view) / left end face × bottom face (front view) / lower-left corner of the section (section view); the d slot carries b"),
    ("剖视图", "Section view"),
    ("视图", "View"),
    ("A型", "Type A"),
    ("B型", "Type B"),
    ("C型", "Type C"),
    ("圆头普通平键", "Round-end parallel key"),
    ("导向平键", "Gib-head key"),
    ("轴径", "Shaft diameter"),
    ("内径", "Bore diameter"),
    ("毂槽（{}）", "Hub keyway ({})"),
    ("砂轮越程槽 磨外圆", "Grinding relief groove, external grinding"),
    ("圆柱销 A型", "Parallel pin, type A"),
    ("内螺纹圆柱销", "Parallel pin with internal thread"),
    ("{}（规格代号自带 N/d/D/B）；{} 表1/表2（de）；模板 矩形花键.dxf 逐图元反解", "{} (spec code already carries N/d/D/B); {} Table 1/2 (de); template 矩形花键.dxf reverse-engineered entity by entity"),
];

/// GUI 元数据取词：`zh` 原值 → 当前语言。英文缺条目时**回落中文**并记入 [`missing_keys`]。
/// **有意原样保留**的元数据（文件名/路径等：翻了就对不上磁盘/图纸）。
pub const DATA_KEEP_AS_IS: &[&str] = &[
    "磨外圆_GB-T6403.png + 磨外圆_GB-T6403.5-2008.dxf",
];

pub fn t_data(zh: &str) -> String {
    if DATA_KEEP_AS_IS.contains(&zh) {
        return zh.to_string();
    }
    if lang() == Lang::Zh {
        return zh.to_string();
    }
    match DATA_CATALOG.iter().find(|(k, _)| *k == zh) {
        Some((_, en)) if !en.is_empty() => en.to_string(),
        _ => {
            note_missing(&format!("data:{zh}"));
            zh.to_string()
        }
    }
}

/// JSON 里**要翻**的元数据字段（白名单；其余字符串是数据/协议，原样透传）。
const DATA_JSON_FIELDS: &[&str] = &[
    "name", "base_hint", "len_label", "d_label", "tree_dir", "hint", "place_hint", "label",
    "source",
    "display", "desc",
];

/// 树叶子段 `族名 + " " + 代号` → 英文：按**最长前缀**命中 `DATA_CATALOG`，代号部分原样拼回。
fn t_data_prefix(seg: &str) -> String {
    if lang() == Lang::Zh {
        return seg.to_string();
    }
    let mut best: Option<(&str, &str)> = None;
    for (zh, en) in DATA_CATALOG {
        if zh.is_empty() || !seg.starts_with(zh) {
            continue;
        }
        let rest = &seg[zh.len()..];
        if rest.is_empty() || rest.starts_with(' ') {
            if best.map_or(true, |(b, _)| b.len() < zh.len()) {
                best = Some((zh, en));
            }
        }
    }
    match best {
        Some((zh, en)) => format!("{en}{}", &seg[zh.len()..]),
        None => t_data_soft(seg),
    }
}

/// 带占位符的 GUI 数据取词：模板（含 `{}`）本身进 [`DATA_CATALOG`]，`{}` 按顺序替换。
/// `Lang::Zh` 下原样返回（占位符也填好）；英文缺条目时回落中文并记 missing。
pub fn t_data_fmt(zh: &str, args: &[&str]) -> String {
    if DATA_KEEP_AS_IS.contains(&zh) {
        return zh.to_string();
    }
    let mut out = if lang() == Lang::Zh {
        zh.to_string()
    } else {
        match DATA_CATALOG.iter().find(|(k, _)| *k == zh) {
            Some((_, en)) if !en.is_empty() => en.to_string(),
            _ => {
                note_missing(&format!("data:{zh}"));
                zh.to_string()
            }
        }
    };
    for a in args {
        out = out.replacen("{}", a, 1);
    }
    out
}

/// 软取词：字典有就翻，没有就**原样返回且不记 missing**（用于批量 JSON：数据串不算漏译）。
fn t_data_soft(zh: &str) -> String {
    if lang() == Lang::Zh {
        return zh.to_string();
    }
    match DATA_CATALOG.iter().find(|(k, _)| *k == zh) {
        Some((_, en)) if !en.is_empty() => en.to_string(),
        _ => zh.to_string(),
    }
}

/// 递归把 JSON 对象里**白名单字段**的字符串值取词（键不动；尺寸/代号等数据原样）。
///
/// 用于 `partgen::catalog_json`：目录 JSON 在「各族 JSON 合并后、建树前」整体取词，
/// 树叶子名（`族名 + 代号`）因此也随语言。`tree_path` 按 `>` 分段，段内按最长前缀译。
pub fn translate_json(value: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match value {
        Value::String(s) => Value::String(t_data_soft(s)),
        Value::Array(items) => Value::Array(items.iter().map(translate_json).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    let out = match (k.as_str(), v) {
                        ("tree_path", Value::String(s)) => Value::String(
                            s.split('>')
                                .map(|seg| {
                                    let trimmed = seg.trim();
                                    let lead = &seg[..seg.len() - seg.trim_start().len()];
                                    let tail = &seg[seg.trim_end().len()..];
                                    format!("{lead}{}{tail}", t_data_prefix(trimmed))
                                })
                                .collect::<Vec<_>>()
                                .join(">"),
                        ),
                        (field, Value::String(s)) if DATA_JSON_FIELDS.contains(&field) => {
                            Value::String(t_data_soft(s))
                        }
                        _ => translate_json(v),
                    };
                    (k.clone(), out)
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

/// 单一 message catalog。加词条 = 按域在对应小节加一行。
pub const CATALOG: &[Msg] = &[
    // ── 语言元信息（GUI / 测试）──────────────────────────────────────
    Msg::new("meta.lang.zh", "中文", "Chinese"),
    Msg::new("meta.lang.en", "英文", "English"),

    // ── 命令输出（命令回执 / 用法 / 提示）────────────────────────────
    // 竖切域 ①：`TF` 的用法串（见 lib.rs::parse_frame_args）。
    Msg::new(
        "cmd.tf.usage",
        "用法：TF <名称> [比例] [at x,y] [rot 度]（不带参数 = 打开选择窗口）",
        "Usage: TF <name> [scale] [at x,y] [rot deg] (no args = open the picker window)",
    ),

    // ── 报错（解析错误 / 校验失败）──────────────────────────────────
    // 竖切域 ②：`TF` 参数认不出（带 `{arg}` / `{usage}` 插值，见 lib.rs）。
    Msg::new(
        "cmd.tf.err.unknown_param",
        "认不出的参数「{arg}」。{usage}",
        "Unrecognized argument \"{arg}\". {usage}",
    ),

    // ── GUI：语言切换控件（服务端注入每个页面；见 guide_server::render_page）──
    Msg::new("gui.lang.title", "语言", "Language"),
    // 语言名按「自身语言」显示（双语值相同；切换按钮两侧都要能认出自己）。
    Msg::new("gui.lang.zh", "中文", "中文"),
    Msg::new("gui.lang.en", "English", "English"),

    // ── GUI：公共错误（ocsm_gui_common.js；`{...}` 由页面 JS `.replace()` 填）──
    Msg::new(
        "gui.common.err.404",
        "端点 {path} 不存在（HTTP 404）：可能是插件未重启或版本不匹配 —— 请完全退出并重开 OCS",
        "Endpoint {path} not found (HTTP 404): the plugin may not have been restarted or the version does not match — fully quit and reopen OCS",
    ),
    Msg::new(
        "gui.common.err.http",
        "HTTP {status}：{detail}",
        "HTTP {status}: {detail}",
    ),
    Msg::new("gui.common.err.no_body", "无响应体", "empty response body"),
    Msg::new(
        "gui.common.err.not_json",
        "响应不是 JSON（HTTP {status}）：{text}",
        "Response is not JSON (HTTP {status}): {text}",
    ),
    Msg::new(
        "gui.common.err.net",
        "请求失败（网络层）：{err}",
        "Request failed (network): {err}",
    ),
    Msg::new("gui.common.err.request_failed", "请求失败", "Request failed"),

    // ── GUI：窗口/页面标题（窗口名 + <title>/<h1>）────────────────────
    Msg::new("gui.window.guide", "标注配置", "Annotation settings"),
    Msg::new("gui.window.parts", "零件库", "Parts library"),
    Msg::new("gui.window.gear", "齿轮", "Gear"),
    Msg::new("gui.window.shaft", "轴生成器", "Shaft generator"),
    Msg::new("gui.window.hole", "孔生成器", "Hole generator"),
    Msg::new("gui.window.spline", "花键参数表", "Spline tables"),
    Msg::new("gui.window.joint", "螺栓副装配", "Bolt joint assembly"),
    Msg::new("gui.window.rough", "表面粗糙度", "Surface roughness"),
    Msg::new("gui.window.manual", "命令手册", "Command manual"),
    Msg::new("gui.window.bom", "明细表编辑", "BOM editor"),
    Msg::new("gui.window.default", "标注配置", "Annotation settings"),

    // ── GUI：外部打开的提示页（opened_elsewhere_html）────────────────
    Msg::new(
        "gui.notice.opened_title",
        "「{label}」已经用独立窗口打开了",
        "\"{label}\" is already open in a separate window",
    ),
    Msg::new(
        "gui.notice.opened_body1",
        "图纸里的链接（Ctrl+点击）只能落在这种带地址栏的浏览器页签上，所以插件又开了一个 OCSM 独立窗口\n（去掉了地址栏/页签，与其它 OCSM 页面一致）—— 请到那个窗口里操作，",
        "Links in the drawing (Ctrl+click) can only open in a browser tab with an address bar, so the plugin opened a separate OCSM window\n(no address bar/tabs, consistent with the other OCSM pages) — use that window; ",
    ),
    Msg::new("gui.notice.opened_body2", "本页可以直接关掉", "you can close this page"),
    Msg::new("gui.notice.opened_body3", "。", "."),
    Msg::new(
        "gui.notice.bom_hint1",
        "明细表编辑器下次也可以直接运行命令 ",
        "Next time the BOM editor can also be opened by running the command ",
    ),
    Msg::new(
        "gui.notice.bom_hint2",
        " 打开，不走浏览器。",
        ", without a browser.",
    ),
    Msg::new("gui.notice.close_page", "关闭本页", "Close this page"),

    // ── GUI：标注配置（guide_gui.html）──────────────────────────────
    Msg::new("gui.guide.h1", "标注配置", "Annotation settings"),
    Msg::new("gui.guide.type.linear", "线性", "Linear"),
    Msg::new("gui.guide.type.diameter", "直径", "Diameter"),
    Msg::new("gui.guide.type.radius", "半径", "Radius"),
    Msg::new("gui.guide.type.datum", "基准", "Datum"),
    Msg::new("gui.guide.type.view", "向视图", "Auxiliary view"),
    Msg::new("gui.guide.type.angle", "角度", "Angle"),
    Msg::new("gui.guide.type.section", "剖切", "Section"),
    Msg::new("gui.guide.type.tolerance", "形位公差", "Geometric tolerance"),
    Msg::new("gui.guide.type.weld", "焊接", "Weld"),
    Msg::new("gui.guide.type.leader", "引线", "Leader"),
    Msg::new("gui.guide.type.balloon", "序号", "Item number"),
    Msg::new("gui.guide.type.arclen", "弧长", "Arc length"),
    Msg::new("gui.guide.type.detail", "局部放大", "Detail view"),
    Msg::new("gui.guide.apply", "应用", "Apply"),
    Msg::new("gui.guide.refresh", "应用并刷新", "Apply and refresh"),
    Msg::new("gui.guide.save_new", "另存为新标注", "Save as new annotation"),
    Msg::new("gui.guide.update", "更新标注", "Update annotation"),
    Msg::new("gui.guide.close", "关闭", "Close"),

    // ── GUI：标准件库（parts_gui.html）──────────────────────────────
    Msg::new("gui.parts.h1", "OCSM 标准件库", "OCSM Standard Parts"),
    Msg::new("gui.parts.tree", "零件树（点选零件）", "Part tree (click a part)"),
    Msg::new(
        "gui.parts.tree_note",
        "灰色 = 待实现（已列在树上，按需排期）",
        "Gray = not implemented yet (listed; scheduled on demand)",
    ),
    Msg::new("gui.parts.spec_view", "规格与视图", "Spec and view"),
    Msg::new("gui.parts.type", "型别", "Type"),
    Msg::new("gui.parts.dia", "公称直径", "Nominal diameter"),
    Msg::new(
        "gui.parts.len",
        "长度 l（mm，仅列该直径标准长度系列）",
        "Length l (mm; only standard lengths for this diameter)",
    ),
    Msg::new("gui.parts.spec_code", "规格代号", "Spec code"),
    Msg::new(
        "gui.parts.spec_custom_ph",
        "自定义规格，例 6x23x26x6",
        "Custom spec, e.g. 6x23x26x6",
    ),
    Msg::new("gui.parts.d", "轴径 d（mm，自由输入）", "Shaft diameter d (mm, free input)"),
    Msg::new(
        "gui.parts.b1",
        "槽宽 b1 覆盖（mm，留空 = 该 d 档默认）",
        "b1 override (mm; blank = default for this d)",
    ),
    Msg::new("gui.parts.view", "视图（出库哪个视图）", "View (which view to produce)"),
    Msg::new("gui.parts.export", "零件出库", "Produce part"),
    Msg::new("gui.parts.preview_alt", "零件预览", "Part preview"),
    Msg::new("gui.parts.tip", "← 在左侧展开并选择零件", "← Expand a part in the tree on the left"),
    Msg::new("gui.parts.tag_pending", "待实现", "pending"),
    Msg::new("gui.parts.base_default", "基点 = 头部支承面 × 轴线", "Base point = head bearing face × axis"),
    Msg::new("gui.parts.len_default", "长度 l", "Length l"),
    Msg::new(
        "gui.parts.len_suffix",
        "（mm，仅列该直径的取值）",
        " (mm; values for this diameter only)",
    ),
    Msg::new(
        "gui.parts.err.family",
        "该零件族数据异常：{err}",
        "Part family data error: {err}",
    ),
    Msg::new(
        "gui.parts.err.no_view",
        "该族暂无可出库视图（待画法确认）",
        "No producible view for this family yet (drawing method pending)",
    ),
    Msg::new("gui.parts.err.d_positive", "d 必须是正数", "d must be a positive number"),
    Msg::new("gui.parts.err.d_range", "d 超出标准表范围", "d is outside the standard table range"),
    Msg::new("gui.parts.err.not_number", "{label} 不是数字", "{label} is not a number"),
    Msg::new(
        "gui.parts.err.missing_param",
        "参数 {name} 必给（花键：L 满齿段长；退刀槽：P）",
        "Parameter {name} is required (spline: L = full-tooth length; relief groove: P)",
    ),
    Msg::new(
        "gui.parts.err.p_choice",
        "P 必须是表 2 单键：{choices}",
        "P must be a single key from table 2: {choices}",
    ),
    Msg::new(
        "gui.parts.err.spec_form",
        "规格代号要写成 N×d×D×B（例 6x23x26x6）",
        "Spec code must be N×d×D×B (e.g. 6x23x26x6)",
    ),
    Msg::new(
        "gui.parts.hint.p_ok",
        "P={p}：g1={g1}、g2={g2}、dg=d−{red}、r={r}；表 2 单键（P）：{choices}",
        "P={p}: g1={g1}, g2={g2}, dg=d−{red}, r={r}; single keys (P) from table 2: {choices}",
    ),
    Msg::new(
        "gui.parts.hint.d_band",
        "该 d 属于「{band}」档；默认 b1={def}；可选 b1={choices}",
        "d falls in band \"{band}\"; default b1={def}; choices b1={choices}",
    ),
    Msg::new(
        "gui.parts.hint.len_values",
        "该直径{label} 的取值：{choices} mm",
        "{label} for this diameter: {choices} mm",
    ),
    Msg::new(
        "gui.parts.hint.len_single",
        "该直径{label} = {len} mm",
        "{label} for this diameter = {len} mm",
    ),
    Msg::new("gui.parts.hint.in_table", "表内规格", "Spec from table"),
    Msg::new("gui.parts.hint.off_table", "表外规格（de 必给）", "Spec off table (de required)"),
    Msg::new(
        "gui.parts.hint.spec_tail",
        "；L 必给（侧视/剖视），de 留空 = 按规格查表",
        "; L required (side/section view); de blank = look up by spec",
    ),
    Msg::new("gui.parts.custom_spec", "自定义规格…", "Custom spec…"),
    Msg::new("gui.parts.req_yes", "必给", "required"),
    Msg::new("gui.parts.req_blank", "留空 = 默认", "blank = default"),
    Msg::new("gui.parts.status.exporting", "出库中…", "Producing…"),
    Msg::new("gui.parts.status.exported", "已出库", "Part produced"),
    Msg::new(
        "gui.parts.status.exported_tail",
        "（若本窗口未自动关闭，请切回图纸继续）",
        " (if this window did not close automatically, switch back to the drawing)",
    ),

    // ── GUI：孔生成器（hole_gui.html）───────────────────────────────
    Msg::new("gui.hole.h1", "OCSM 孔生成器", "OCSM Hole Generator"),
    Msg::new(
        "gui.hole.sub",
        "简单 / 螺纹 / 沉头 / 埋头 · 盲孔 / 贯通",
        "Simple / threaded / counterbore / countersink · blind / through",
    ),
    Msg::new("gui.hole.kind.simple", "简单孔", "Simple hole"),
    Msg::new("gui.hole.kind.threaded", "螺纹孔", "Threaded hole"),
    Msg::new("gui.hole.kind.counterbore", "沉头孔", "Counterbore"),
    Msg::new("gui.hole.kind.countersink", "埋头孔", "Countersink"),
    Msg::new(
        "gui.hole.hint",
        "选孔型 → 定孔径 → 点「确定」回到图纸放置（自动值不合适时可手改）。",
        "Pick a hole type → set the diameter → click OK to place it in the drawing (edit automatic values if needed).",
    ),
    Msg::new("gui.hole.ok", "确定", "OK"),
    Msg::new("gui.hole.cancel", "取消", "Cancel"),

    // ── GUI：智能卡片（spline_gui.html）─────────────────────────────
    Msg::new("gui.spline.h1", "OCSM 智能卡片", "OCSM Smart Cards"),
    Msg::new("gui.spline.report", "计算书", "Report"),
    Msg::new("gui.spline.export", "出表", "Create table"),
    Msg::new("gui.spline.cancel", "取消", "Cancel"),
    Msg::new("gui.spline.copy", "复制", "Copy"),
    Msg::new("gui.spline.summary", "摘要表落图纸", "Place summary table"),
    Msg::new("gui.spline.close", "关闭", "Close"),

    // ── GUI：螺栓副装配（joint_gui.html）────────────────────────────
    Msg::new("gui.joint.h1", "OCSM 螺栓副装配", "OCSM Bolt Joint Assembly"),
    Msg::new("gui.joint.bolt", "① 螺栓", "① Bolt"),
    Msg::new(
        "gui.joint.chain",
        "② 件链（从被连接件往外，依次穿过）",
        "② Part chain (from the clamped parts outward)",
    ),
    Msg::new(
        "gui.joint.presets",
        "③ 常用件链（点一下套用，之后仍可逐件改）",
        "③ Common chains (click to apply; still editable part by part)",
    ),
    Msg::new("gui.joint.place_title", "放置", "Placement"),
    Msg::new(
        "gui.joint.preview",
        "装配预览（真实几何，含遮挡裁剪）",
        "Assembly preview (real geometry, with occlusion clipping)",
    ),
    Msg::new("gui.joint.add", "+ 加一件", "+ Add part"),
    Msg::new("gui.joint.clear", "清空件链", "Clear chain"),
    Msg::new("gui.joint.place", "装配到图纸", "Place into drawing"),

    // ── GUI：表面粗糙度（rough_gui.html）────────────────────────────
    Msg::new("gui.rough.h1", "OCSM 表面粗糙度", "OCSM Surface Roughness"),
    Msg::new("gui.rough.base", "基础体（V 内差异）", "Base symbol (difference in V)"),
    Msg::new("gui.rough.extra", "附加标注", "Additional annotation"),
    Msg::new("gui.rough.preview", "预览", "Preview"),
    Msg::new(
        "gui.rough.attrs",
        "属性文字（ATTDEF，可缺省空白）",
        "Attribute text (ATTDEF; may be blank)",
    ),
    Msg::new("gui.rough.attr_a_up", "A′ 粗糙度上限", "A′ upper roughness limit"),
    Msg::new("gui.rough.attr_a", "A 粗糙度下限", "A lower roughness limit"),
    Msg::new("gui.rough.attr_e", "E 备注", "E Remarks"),
    Msg::new("gui.rough.p_symbol", "加工符号 P", "Machining symbol P"),
    Msg::new("gui.rough.rotate", "旋转", "Rotation"),
    Msg::new(
        "gui.rough.rot_hint",
        "符号绕插入点旋转（默认 0）。",
        "Rotate the symbol around the insertion point (default 0).",
    ),
    Msg::new(
        "gui.rough.c2_hint",
        "仅 C2（以不去除材料的方法获得）时强制空白。",
        "Forced blank for C2 (obtained without material removal) only.",
    ),
    Msg::new("gui.rough.blank", "空白", "blank"),
    Msg::new("gui.rough.apply", "应用并关闭", "Apply and close"),
    Msg::new("gui.rough.c1", "C1 通用", "C1 General"),
    Msg::new("gui.rough.c2", "C2 以不去除材料的方法获得", "C2 Obtained without material removal"),
    Msg::new("gui.rough.c3", "C3 去除材料", "C3 Material removal"),
    Msg::new("gui.rough.c4", "C4 焊后加工", "C4 Post-weld machining"),
    Msg::new("gui.rough.r1", "基础", "Basic"),
    Msg::new("gui.rough.r2", "周边相同处理", "Same treatment all around"),
    Msg::new("gui.rough.r3", "高级", "Advanced"),
    Msg::new("gui.rough.r4", "上限开关", "Upper limit switch"),
    Msg::new("gui.rough.r5", "上限开关+周边相同处理", "Upper limit + same treatment all around"),

    // ── GUI：命令手册（manual_gui.html）─────────────────────────────
    Msg::new("gui.manual.h1", "OCSM 命令手册", "OCSM Command Manual"),
    Msg::new("gui.manual.commands", "命令目录", "Command list"),
    Msg::new(
        "gui.manual.topics",
        "操作教程 / 制图知识",
        "Tutorials / drafting knowledge",
    ),
    Msg::new(
        "gui.manual.empty",
        "← 左侧点一个命令或主题；这里显示它的用法与步骤。",
        "← Click a command or topic on the left; its usage and steps appear here.",
    ),
    Msg::new(
        "gui.manual.note",
        "教程正文是磁盘上的 markdown（与 AI 用的 skill 手册同一批文件）。搜索目录：",
        "Tutorials are markdown files on disk (the same set used by the AI skill manual). Search directories:",
    ),
    Msg::new("gui.manual.loading", "读取中…", "Loading…"),
    Msg::new("gui.manual.read_fail", "读取失败", "Failed to load"),
    Msg::new("gui.manual.not_found", "未找到该主题的 md", "No markdown found for this topic"),
    Msg::new("gui.manual.source", "来源：{path}", "Source: {path}"),
    Msg::new(
        "gui.manual.count",
        "　共 {cmds} 条命令、{topics} 篇教程",
        " {cmds} commands, {topics} tutorials",
    ),
    Msg::new(
        "gui.manual.no_md1",
        "没找到手册 md。把手册放到 ",
        "No manual markdown found. Put the manual under ",
    ),
    Msg::new(
        "gui.manual.no_md2",
        "，或用环境变量 ",
        ", or point the environment variable ",
    ),
    Msg::new(
        "gui.manual.no_md3",
        " 指定目录后重开本窗口。",
        " at a directory and reopen this window.",
    ),
    Msg::new(
        "gui.manual.init_fail",
        "初始化失败：{err}",
        "Initialization failed: {err}",
    ),
    Msg::new(
        "gui.manual.request_fail",
        "请求失败：{err}",
        "Request failed: {err}",
    ),
    Msg::new(
        "gui.manual.search_dirs",
        "搜索目录：{dirs}",
        "Search directories: {dirs}",
    ),
    Msg::new("gui.manual.fallback", "中文原文（该篇尚无英文版）", "Chinese original (English edition pending)"),
    Msg::new("gui.manual.group.0", "入门与总览", "Getting started / overview"),    Msg::new("gui.manual.group.1", "建图与插入", "Drawing setup and insertion"),
    Msg::new("gui.manual.group.2", "标注与符号", "Dimensions and symbols"),
    Msg::new("gui.manual.group.3", "表格与自动化", "Tables and automation"),
    Msg::new("gui.manual.group.4", "机械制图知识", "Mechanical drafting knowledge"),

    // ── GUI：明细表编辑（bom_gui.html）──────────────────────────────
    Msg::new("gui.bom.h1", "明细表编辑", "BOM Editor"),
    Msg::new("gui.bom.reload", "刷新", "Reload"),
    Msg::new("gui.bom.export_x", "导出 xlsx", "Export xlsx"),
    Msg::new("gui.bom.export_c", "导出 csv", "Export csv"),
    Msg::new("gui.bom.import", "导入 xlsx/csv…", "Import xlsx/csv…"),
    Msg::new("gui.bom.add", "＋加行", "+ Add row"),
    Msg::new("gui.bom.apply", "应用到图纸", "Apply to drawing"),

    // ── GUI：齿轮 / 轴生成器（页头与主按钮）─────────────────────────
    Msg::new(
        "gui.gear.title",
        "OCSM 齿轮（外齿轮 / 内齿轮）",
        "OCSM Gear (External / Internal)",
    ),
    Msg::new("gui.gear.h1", "OCSM 齿轮 / 花键", "OCSM Gear / Spline"),
    Msg::new(
        "gui.gear.sub",
        "外齿轮 / 内齿轮（齿圈）",
        "External gear / internal gear (ring)",
    ),
    Msg::new("gui.gear.copy_expr", "复制表达式", "Copy expression"),
    Msg::new("gui.gear.export", "生成到图纸", "Generate into drawing"),
    Msg::new("gui.shaft.title", "OCSM 轴生成器", "OCSM Shaft Generator"),
    Msg::new("gui.shaft.h1", "OCSM 轴生成器", "OCSM Shaft Generator"),
    Msg::new(
        "gui.shaft.sub",
        "段表 ↔ 行文本双向同步 + 实时预览",
        "Segment table ↔ DSL two-way sync + live preview",
    ),
    Msg::new("gui.shaft.export", "生成到图纸", "Generate into drawing"),
    Msg::new("gui.shaft.add_row", "+ 加行", "+ Add row"),
    Msg::new("gui.shaft.del_row", "删行", "Delete row"),
    Msg::new("gui.shaft.copy_row", "复制行", "Copy row"),
    Msg::new("gui.shaft.up_row", "上移", "Move up"),
    Msg::new("gui.shaft.down_row", "下移", "Move down"),
    Msg::new("gui.shaft.parse_add", "解析并加段", "Parse and add segment"),
    Msg::new("gui.shaft.open_gear", "打开齿轮生成器", "Open gear generator"),

    // ── 命令目录（手册窗口左侧总表；guide_server::COMMAND_CATALOG 一句话摘要）──
    // 命令关键字/示例/图层名/标准符号照原样保留（不译）；译的是说明性句子。
    Msg::new(
        "cmd.catalog.ocsm",
        "初始化：建图层/线型/文字样式/标注样式（并打开零件库窗口）",
        "Initialize: create layers/linetypes/text styles/dimension styles (also opens the parts library window)",
    ),
    Msg::new(
        "cmd.catalog.digits",
        "切当前图层；有选中对象时把对象移到该层",
        "Switch the current layer; if objects are selected, move them to that layer",
    ),
    Msg::new(
        "cmd.catalog.frameinit",
        "图框：不带参数=打开图框选择窗口；带参数=一行直插（`TF a3_landscape 1:2 at 0,0 [rot 度]`）",
        "Frame: no args = open the frame picker; with args = insert in one line (`TF a3_landscape 1:2 at 0,0 [rot deg]`)",
    ),
    Msg::new(
        "cmd.catalog.frameinsert",
        "按所选图框 + 比例插入（光标跟随，比例感知标注样式）；也可带参数直插（同 TF）",
        "Insert the chosen frame at the chosen scale (cursor-follow, scale-aware dimension styles); args work like TF",
    ),
    Msg::new(
        "cmd.catalog.part",
        "标准件/结构要素插入：不带参数=开零件库窗口（左「标准件」树 + 右「结构要素」树）+放置态；带参数=一行直插（标准件 `XL 族 d l [view …] [at x,y] [rot 度]`；结构要素 `XL detail_grind_od d [b1 值] [at x,y] [rot 度]`；外螺纹退刀槽 `XL detail_thread_relief d P 螺距 [g1 值 g2 值 dg 值 r 值 alpha 值] [at x,y] [rot 度]`；毂槽 `XL detail_hub_keyway d [len 毂长] [view main|side]`（b/t₂/r 由 d 查表，len 缺省 30）；平键 `XL key_1096_{a|b|c} b L`、`XL key_1097_{a|b} b L`（d 槽位=键宽 b，L 省略/0=该档默认，L 须 ∈ 标准系列且 L<10b；1097 的 L1/L2/L3 由 L 查 GB/T 1097 长度系列表派生，表外 L 报错））",
        "Standard parts / structural details: no args = open the parts library window (left \"Standard parts\" tree + right \"Structural details\" tree) + placement mode; with args = insert in one line (standard part `XL family d l [view …] [at x,y] [rot deg]`; structural detail `XL detail_grind_od d [b1 value] [at x,y] [rot deg]`; external thread relief `XL detail_thread_relief d P pitch [g1 v g2 v dg v r v alpha v] [at x,y] [rot deg]`; hub keyway `XL detail_hub_keyway d [len hub length] [view main|side]` (b/t₂/r looked up from d; len defaults to 30); parallel key `XL key_1096_{a|b|c} b L`, `XL key_1097_{a|b} b L` (the `d` argument position = key width b; L omitted/0 = default for that range; L must be in the standard series and L<10b; for 1097, L1/L2/L3 are derived by looking up the GB/T 1097 length-series table; L outside the table is an error))",
    ),
    Msg::new(
        "cmd.catalog.card",
        "智能卡片（工具栏「卡片」组按钮；旧短命令 `XLT` 已移除）：不带参数=开图形界面（七张卡：GB 花键 / 齿轮 / ANSI 花键中英 / NF 内 / NF 外 / DIN 5480；卡类型表驱动，可 `?card=<id>` 深链；均支持九字段齿形表达式反解）；带参数=`OCSMCARD <卡类型> …`，如 `OCSMCARD 花键参数表 [std GB] 内 6H <表达式> [dp 4.5] [root 平|圆] [at x,y]`、`OCSMCARD NF外花键参数表 SPLINE EX M7.5 Z38 ALPHA20 X0.8 BETA0 H30`；表格块几何内建（模板逐图元），卡类型/体系/公式口径表由后端 `CARD_TYPES`+选项表下发；以后加表格/铭牌 = `CARD_TYPES` 加数据行 + 一个渲染器",
        "Smart cards (\"Cards\" toolbar group; old short command `XLT` removed): no args = open the GUI (seven cards: GB spline / gear / ANSI spline CN+EN / NF internal / NF external / DIN 5480; card types are table-driven, with `?card=<id>` deep links; all support nine-field tooth-profile expression back-solving); with args = `OCSMCARD <card type> …`, e.g. `OCSMCARD 花键参数表 [std GB] 内 6H <expression> [dp 4.5] [root 平|圆] [at x,y]`, `OCSMCARD NF外花键参数表 SPLINE EX M7.5 Z38 ALPHA20 X0.8 BETA0 H30`; table-block geometry is built in (template entity by entity); card types/systems/formula tables come from the backend `CARD_TYPES` + option tables; adding future tables/nameplates = add a data row to `CARD_TYPES` + write a renderer",
    ),
    Msg::new(
        "cmd.catalog.joint",
        "螺栓副装配：不带参数=开装配窗口+放置态；带参数=一行直装（件链算长度、遮挡裁剪、一次撤销）",
        "Bolt joint assembly: no args = open the assembly window + placement mode; with args = assemble in one line (chain length computed, occlusion clipping, single undo)",
    ),
    Msg::new(
        "cmd.catalog.powerdim",
        "智能标注：拾取点模式标线性/对齐/半径/直径（Enter 切线段点选）",
        "Smart dimension: point-pick mode for linear/aligned/radius/diameter (Enter switches to segment picking)",
    ),
    Msg::new(
        "cmd.catalog.dimgulide",
        "引导线标注：选引导线 → 配置窗口（尺寸/剖视/向视/局部放大/角度/弧长/焊接/引线/序号/公差/粗糙度/形位公差）",
        "Leader annotation: pick a leader → configuration window (dimension/section/auxiliary view/detail/angle/arc length/weld/leader/item number/tolerance/roughness/geometric tolerance)",
    ),
    Msg::new(
        "cmd.catalog.centerline",
        "中心线：点圆/圆弧 → 十字中心线；点两根直线 → 角平分线中心线（`3中心线层`，线长 = 直径/投影长 + 图框比例×6mm）",
        "Centerlines: click a circle/arc → cross centerlines; click two lines → angle-bisector centerline (`3中心线层`, length = diameter/projected length + frame scale×6mm)",
    ),
    Msg::new(
        "cmd.catalog.gear",
        "齿轮（外齿轮 / 内齿轮（齿圈））+ 渐开线花键（花键模式）：不带参数=开齿轮/花键窗口（模式复选框 + 参数 + 视图按钮 + 实时预览）；带参数=一行直插（`OCSMGEAR 2 40 20 view 剖视图`、`OCSMGEAR int 2 40 30 view 端视图`；花键：`OCSMGEAR 花键 [内花键] std=DIN [profile=DIN30] db=40 2 18 h=30 view 端视图`，预设代号 GB30P/GB30R/GB375R/GB45R/DIN30 可直接代 std+profile）。齿轮模式只认 模数/齿数/压力角/变位系数 等常规项，给标准号或 d_B 明确报错。花键模式：GB 无基准直径（给 d_B 报错）；DIN 的 d_B 是主参数（d_B+m/d_B+z/m+z 三种给法，表外按公式推并标注来源），内/外花键用同一「齿轮种类」开关。内花键与内齿轮同口径（用户定案「内花键剖视图和内齿轮一样，不存在侧视图」）：只有 剖视图 + 端视图，无侧视图；剖视图齿圈内齿不剖（端面/齿顶线/齿根线/内孔壁/孔口倒角 + 分度线/轴线，不打剖面线），齿圈外壁留用户延伸。计算书：命令加 `REPORT`（如 `OCSMGEAR 花键 std=DIN db=40 2 18 h=30 REPORT`）—— 纯计算不插图，输出含公式/代入数值/结果/依据来源的 Markdown 计算书，`REPORT=路径` 另写文件",
        "Gear (external / internal gear (ring)) + involute spline (spline mode): no args = open the gear/spline window (mode checkbox + parameters + view buttons + live preview); with args = insert in one line (`OCSMGEAR 2 40 20 view 剖视图`, `OCSMGEAR int 2 40 30 view 端视图`; spline: `OCSMGEAR 花键 [内花键] std=DIN [profile=DIN30] db=40 2 18 h=30 view 端视图`; presets GB30P/GB30R/GB375R/GB45R/DIN30 can stand in for std+profile). Gear mode accepts only the usual items (module/teeth/pressure angle/profile shift, etc.); a standard number or d_B is a clear error. Spline mode: GB has no reference diameter (giving d_B is an error); for DIN, d_B is the primary parameter (three ways to give it: d_B+m / d_B+z / m+z; outside the table it is derived by formula with the source noted); internal/external splines use the same \"gear kind\" switch. Internal splines follow the same convention as internal gears (user ruling: \"an internal spline section view is the same as an internal gear; there is no side view\"): only section + end views, no side view; in the section view the internal teeth are not sectioned (face/tip/root lines + bore wall + bore chamfer + pitch line/axis, no hatching), and the outer wall of the ring is left for the user to extend. Report: append `REPORT` (e.g. `OCSMGEAR 花键 std=DIN db=40 2 18 h=30 REPORT`) — computation only, no geometry; prints a Markdown report with formulas/substituted values/results/sources; `REPORT=path` also writes a file",
    ),
    Msg::new(
        "cmd.catalog.shaft",
        "轴生成器：不带参数=开轴生成器窗口（段表 ↔ 行文本双向同步 + 实时预览 + 视图按钮）+ 放置态；带参数=行 DSL/JSON 一行直插（段拼接 + 端面倒角 + 砂轮越程槽 + 螺纹段 M + 齿轮段 GEAR + 矩形花键段 SPLINE + 轴槽 KEY（GB/T 1095 平键键槽，本期只做轴槽） + 视图 VIEW 常规|剖视（双视图已移除）；退刀槽就是一小段小直径轴段）。`OCSMSHAFT S30 E30 L45 CH2@L | S40 E40 L7 M1.5 | S36 E36 L5 | GEAR M3 Z20 VIEW 剖视 at x,y rot 度`；齿轮段可 `GEAR M3 Z20 ALPHA25`（压力角默认 20°）；花键 `OCSMSHAFT SPLINE 6x23x26x6 L30`（可 `de 71` 覆盖，矩形花键）；轴槽 `OCSMSHAFT S25 E25 L40 CH2@L KEY A 18 | S30 E30 L30`（**轴段类型**，进段表 KEY 列：键型 A/B/C + 键长 L（所选键型的键长）+ 位置中置/端置；**b×h 由该段直径 d 查 GB/T 1095 d 列自动定**，显式 `b8h7` 只作校验、必须落在该轴径档标准配对；t1 按 b 查 GB/T 1095 表，可 `t1 5` 覆盖；B/C 的键长自动按槽端圆弧折算（圆弧半径 b/2 吃直段：中置 B +b / C +b/2；端置 B +b/2 / C 不折算），实际槽长为折算长度（端置再 +t1）；显示口径：中置恒显示 A、端置 B/C 显示 C；可选 `双槽`（`DOUBLE`）＝绕轴心 180° 对置、仅剖视图体现、约 1.5 倍单键转矩；与 GEAR/SPLINE/M/OV/RL 互斥；导向平键 `OCSMSHAFT S30 E30 L50 KEY A 25 导向`（GB/T 1097：只 A/B、L 取 1097 系列 25…450∩L<10b、槽长 = 键长、槽上自动 2 个固定螺钉螺纹孔 d0×L0（孔心距槽端 L3），与双槽互斥；起键孔不属 1097），见 handbook 03）。渐开线花键只在 OCSMGEAR 花键模式生成（轴段 INVOLSPLINE 已撤）；GB/T 3478 基本齿廓不含变位（花键模式下 GB 的 x 恒为 0 且锁死；CLI/查询串给非零 x 明确报错，DIN/NF 的 x 仍按 d_B/A 派生）。计算书：命令加 `REPORT`（如 `OCSMSHAFT SPLINE 6x23x26x6 L30 REPORT`）—— 纯计算不插图，输出段清单 Markdown 计算书，`REPORT=路径` 另写文件",
        "Shaft generator: no args = open the shaft generator window (segment table ↔ DSL two-way sync + live preview + view buttons) + placement mode; with args = one-line DSL/JSON insert (segment concatenation + end chamfers + grinding relief grooves + thread segments M + gear segments GEAR + rectangular spline segments SPLINE + shaft keyways KEY (GB/T 1095 parallel-key keyways; only shaft keyways for now) + view VIEW normal|section (dual view removed); a relief groove is just a short small-diameter shaft segment). `OCSMSHAFT S30 E30 L45 CH2@L | S40 E40 L7 M1.5 | S36 E36 L5 | GEAR M3 Z20 VIEW 剖视 at x,y rot deg`; gear segments accept `GEAR M3 Z20 ALPHA25` (pressure angle defaults to 20°); spline `OCSMSHAFT SPLINE 6x23x26x6 L30` (`de 71` overrides; rectangular spline); keyway `OCSMSHAFT S25 E25 L40 CH2@L KEY A 18 | S30 E30 L30` (**shaft segment type**, fills the segment table KEY column: key type A/B/C + key length L (for the chosen key type) + position centered/end; **b×h is determined automatically from GB/T 1095 by the segment diameter d**; an explicit `b8h7` is only validated and must match the standard pairing for that diameter range; t1 is looked up by b in GB/T 1095 and `t1 5` overrides; for B/C the key length is converted by the slot-end radius (radius b/2 eats into the straight part: centered B +b / C +b/2; end-position B +b/2 / C no conversion) and the actual slot length is the converted length (end-position adds t1); display convention: centered always shows A, end-position B/C shows C; optional `双槽` (`DOUBLE`) = two slots 180° apart, shown in section view only, about 1.5× single-key torque; mutually exclusive with GEAR/SPLINE/M/OV/RL; guide key `OCSMSHAFT S30 E30 L50 KEY A 25 导向` (GB/T 1097: only A/B, L from the 1097 series 25…450 ∩ L<10b, slot length = key length, two fixing-screw threaded holes d0×L0 added above the slot (hole centers at L3 from the slot end), mutually exclusive with double slot; the key-removal hole is not part of 1097), see handbook 03). Involute splines are generated only in OCSMGEAR spline mode (the INVOLSPLINE shaft segment was removed); the GB/T 3478 basic profile has no profile shift (in spline mode GB x is always 0 and locked; a non-zero x from the CLI/query string is a clear error, while DIN/NF x is still derived from d_B/A). Report: append `REPORT` (e.g. `OCSMSHAFT SPLINE 6x23x26x6 L30 REPORT`) — computation only, no geometry; prints a segment-list Markdown report; `REPORT=path` also writes a file",
    ),
    Msg::new(
        "cmd.catalog.hole",
        "孔生成器：不带参数=开孔生成器窗口（简单孔/螺纹孔/沉头孔/埋头孔；盲孔/贯通；沉头/埋头可选带螺纹；不带螺纹时子类型=钻头大小/自定义/螺栓间隙）+ 放置态；带参数=一行直插（`OCSMHOLE 螺纹孔 M10 H18 L15 at x,y rot 度`、`OCSMHOLE 沉头孔 无螺纹 M10 间隙 中等装配 H22`）；规格可直接写 GUI 下拉名（`G G1/8`、`UNC 1/4-20`、`NPT NPT1/2`、`Tr Tr8×1.5`、`ACME 1/4-16`，数值 `公称6.35 P1.058` 照旧）。自动螺纹长 = 1.5d；自动孔深 = 有效深+2P（仅带螺纹且盲孔时可选）；贯通无 118° 锥。数据：ISO 724 / GB/T 152.3 / GB/T 152.2 / GB/T 5277 / 底孔牙深明细表（见表 JSON 的 source）",
        "Hole generator: no args = open the hole generator window (simple/threaded/counterbore/countersink; blind/through; counterbore/countersink may be threaded; when unthreaded the subtype = drill size/custom/bolt clearance) + placement mode; with args = insert in one line (`OCSMHOLE 螺纹孔 M10 H18 L15 at x,y rot deg`, `OCSMHOLE 沉头孔 无螺纹 M10 间隙 中等装配 H22`); the size can be given directly as the GUI dropdown name (`G G1/8`, `UNC 1/4-20`, `NPT NPT1/2`, `Tr Tr8×1.5`, `ACME 1/4-16`; the numeric form `公称6.35 P1.058` still works). Automatic thread length = 1.5d; automatic hole depth = effective depth + 2P (available only for threaded blind holes); through holes have no 118° cone. Data: ISO 724 / GB/T 152.3 / GB/T 152.2 / GB/T 5277 / tap-drill tooth-depth table (see the source field in the table JSON)",
    ),
    Msg::new(
        "cmd.catalog.edit",
        "改标注：选中 OCSM 生成的标注 → 配置窗口改参数 → 重生成",
        "Edit annotation: select an OCSM-generated annotation → change parameters in the config window → regenerate",
    ),
    Msg::new(
        "cmd.catalog.rgh",
        "表面粗糙度：点选插入点 → 配置窗口（匿名块 + ATTDEF）",
        "Surface roughness: pick an insertion point → config window (anonymous block + ATTDEF)",
    ),
    Msg::new(
        "cmd.catalog.dim2gb",
        "一键转国标：原生标注 → OCSM_GB 样式 + 匿名块；智能圆心标记（CENTERMARK）一并换成 `3中心线层` 中心线（Ø + 图框比例×6）",
        "One-click GB conversion: native dimensions → OCSM_GB style + anonymous blocks; smart center marks (CENTERMARK) are also converted to `3中心线层` centerlines (Ø + frame scale×6)",
    ),
    Msg::new(
        "cmd.catalog.bom",
        "明细表：建表/刷新（BOM 30 = 本次首列 30 行；现有行的手改与数量锁保留）",
        "BOM: build/refresh (BOM 30 = 30 rows in the first column this time; manual edits to existing rows and quantity locks are kept)",
    ),
    Msg::new(
        "cmd.catalog.bomsync",
        "明细表：按序号球标重排/重建（球标联动入口）",
        "BOM: reorder/rebuild from item balloons (balloon-sync entry point)",
    ),
    Msg::new(
        "cmd.catalog.bomedit",
        "明细表：打开网页编辑器（＝Ctrl+点击图纸里的表块；改完点「应用到图纸」）",
        "BOM: open the web editor (= Ctrl+click the table block in the drawing; click \"Apply to drawing\" when done)",
    ),
    Msg::new(
        "cmd.catalog.bomlock",
        "明细表：锁定某行数量（BOMLOCK 5 3 / BOMLOCK 5 off）",
        "BOM: lock a row's quantity (BOMLOCK 5 3 / BOMLOCK 5 off)",
    ),
    Msg::new(
        "cmd.catalog.bomxlsx",
        "明细表：导出到 .xlsx（带「锁定数量」列，可外部编辑）",
        "BOM: export to .xlsx (with a \"lock quantity\" column, editable outside)",
    ),
    Msg::new(
        "cmd.catalog.bomxlsxi",
        "明细表：从 .xlsx/.csv 导入（手改数量自动上锁）",
        "BOM: import from .xlsx/.csv (manually changed quantities are locked automatically)",
    ),
    Msg::new(
        "cmd.catalog.bomcfg",
        "明细表：只改「每列行数」（表头/列宽/格式在 bom/ 模板块里，本命令改不了）",
        "BOM: change only \"rows per column\" (headers/column widths/format live in the bom/ template blocks; this command cannot change them)",
    ),
    Msg::new(
        "cmd.catalog.mcp",
        "打印 MCP/HTTP 接入信息（给外部 AI/脚本）",
        "Print MCP/HTTP access info (for external AI/scripts)",
    ),
    Msg::new(
        "cmd.catalog.help",
        "打开本手册窗口（命令目录 + 操作教程）",
        "Open this manual window (command list + tutorials)",
    ),

    // ── 命令输出/报错：孔生成器（OCSMHOLE / DK；hole.rs）+ 螺纹表（thread.rs）──
    // 报错里保留可核对的原始信息（值/表名/规格名）；命令关键字与表内 `name` 不译。
    Msg::new(
        "cmd.hole.err.thread_pair_missing",
        "螺纹 M{d}×{p} 不在 ISO 724 表里（粗牙/细牙都没有这个组合）",
        "Thread M{d}×{p} is not in the ISO 724 table (neither coarse nor fine has this combination)",
    ),
    Msg::new(
        "cmd.hole.err.thread_coarse_missing",
        "M{d} 不在 ISO 724 粗牙表里（粗牙范围 M{d_lo}…M{d_hi}）",
        "M{d} is not in the ISO 724 coarse-thread table (coarse range M{d_lo}…M{d_hi})",
    ),
    Msg::new(
        "cmd.hole.err.counterbore_gb70_2",
        "GB/T 152.3-1988 没有 GB/T 70.2（内六角平圆头）的沉孔表（官方只有 表1 适用 GB 70、表2 适用 GB 6190、GB 6191 及 GB 65），按不插值规则 70.2 标缺 —— 请选 70.1 或用自定义",
        "GB/T 152.3-1988 has no counterbore table for GB/T 70.2 (hex-socket button head): officially only Table 1 for GB 70 and Table 2 for GB 6190, GB 6191 and GB 65; per the no-interpolation rule 70.2 is marked missing — choose 70.1 or use a custom value",
    ),
    Msg::new(
        "cmd.hole.err.reco_unknown",
        "不认识的沉孔推荐值「{other}」（gb70_1/gb70_2/gb6190）",
        "Unrecognized counterbore recommendation \"{other}\" (gb70_1/gb70_2/gb6190)",
    ),
    Msg::new(
        "cmd.hole.err.reco_unknown_cli",
        "不认识的沉孔推荐值「{other}」（70.1/70.2/6190）",
        "Unrecognized counterbore recommendation \"{other}\" (70.1/70.2/6190)",
    ),
    Msg::new(
        "cmd.hole.err.counterbore_row_missing",
        "GB/T 152.3-1988 {table} 表里没有 M{d}（可用 {list}）",
        "The GB/T 152.3-1988 {table} table has no M{d} (available {list})",
    ),
    Msg::new(
        "cmd.hole.err.countersink_row_missing",
        "GB/T 152.2-2014（沉头螺钉用沉孔）表里没有 M{d}（可用 M{d_lo}…M{d_hi}；标准只到 M10）",
        "The GB/T 152.2-2014 (countersink for flat-head screws) table has no M{d} (available M{d_lo}…M{d_hi}; the standard only goes to M10)",
    ),
    Msg::new(
        "cmd.hole.err.clearance_row_missing",
        "GB/T 5277 通孔表里没有 M{d}（可用 M{d_lo}…M{d_hi}）",
        "The GB/T 5277 clearance-hole table has no M{d} (available M{d_lo}…M{d_hi})",
    ),
    Msg::new(
        "cmd.hole.err.tap_row_missing",
        "底孔径表（螺纹孔底孔）里没有 M{d}{p}",
        "The tap-drill table (threaded-hole pilot) has no M{d}{p}",
    ),
    Msg::new(
        "cmd.hole.err.clearance_fit_unknown",
        "不认识的螺栓间隙配合「{other}」（close/normal/loose）",
        "Unrecognized bolt clearance fit \"{other}\" (close/normal/loose)",
    ),
    Msg::new(
        "cmd.hole.err.thread_fit_unknown",
        "不认识的螺纹配合「{other}」（本期只有 6H/6G）",
        "Unrecognized thread fit \"{other}\" (only 6H/6G for now)",
    ),
    Msg::new(
        "cmd.hole.err.d_invalid",
        "公称直径 d={d} 非法（必须 > 0）",
        "Nominal diameter d={d} is invalid (must be > 0)",
    ),
    Msg::new(
        "cmd.hole.err.simple_has_thread_subtype",
        "简单孔不带螺纹：子类型请选 钻头大小 / 自定义 / 螺栓间隙",
        "A simple hole is unthreaded: choose drill size / custom / bolt clearance as the subtype",
    ),
    Msg::new(
        "cmd.hole.err.threaded_needs_thread_subtype",
        "螺纹孔必须带螺纹：子类型请选 标准螺纹 / 细牙螺纹",
        "A threaded hole must be threaded: choose standard thread / fine thread as the subtype",
    ),
    Msg::new(
        "cmd.hole.err.cbore_csink_metric_only",
        "沉头/埋头推荐值表只覆盖公制 M（GB/T 152.3-1988 / 152.2-2014）—— 非公制螺纹请用简单孔/螺纹孔",
        "The counterbore/countersink recommendation tables cover metric M only (GB/T 152.3-1988 / 152.2-2014) — for non-metric threads use a simple/threaded hole",
    ),
    Msg::new(
        "cmd.hole.err.nonmetric_no_metric_fine",
        "非公制螺纹请用「子类型」选牙型系列（如 UNC/UNF/Stub ACME），不要用公制细牙",
        "For non-metric threads choose the thread family with the subtype (e.g. UNC/UNF/Stub ACME); do not use metric fine",
    ),
    Msg::new(
        "cmd.hole.err.fine_needs_concrete_pitch",
        "细牙螺纹需要具体螺距（大小里选 M10×1.25 这类）",
        "A fine thread needs a concrete pitch (pick e.g. M10×1.25 as the size)",
    ),
    Msg::new(
        "cmd.hole.err.fine_subtype_but_coarse",
        "子类型是细牙，但 M{d}×{p} 是粗牙规格",
        "The subtype is fine, but M{d}×{p} is a coarse-thread spec",
    ),
    Msg::new(
        "cmd.hole.err.standard_subtype_but_fine",
        "子类型是标准螺纹（粗牙），但 M{d}×{p} 是细牙规格 —— 请切「细牙螺纹」",
        "The subtype is standard thread (coarse), but M{d}×{p} is a fine-thread spec — switch to fine thread",
    ),
    Msg::new(
        "cmd.hole.err.minor_nonpositive",
        "螺纹 M{d}×{p} 的小径 d−1.0825P={minor} ≤ 0，参数非法",
        "The minor diameter d−1.0825P={minor} of M{d}×{p} is ≤ 0; invalid parameters",
    ),
    Msg::new(
        "cmd.hole.err.drill_required",
        "钻头大小需要选一个标准麻花钻直径（GB/T 6135.3 系列）",
        "Drill size requires a standard twist-drill diameter (GB/T 6135.3 series)",
    ),
    Msg::new(
        "cmd.hole.err.drill_not_in_series",
        "Ø{d} 不在 GB/T 6135.3-1996 直柄麻花钻直径系列里（0.20–20.00）—— 表外不插值",
        "Ø{d} is not in the GB/T 6135.3-1996 straight-shank twist-drill series (0.20–20.00) — no interpolation outside the table",
    ),
    Msg::new(
        "cmd.hole.err.custom_d_required",
        "自定义孔径必须填一个 > 0 的数值",
        "Custom hole diameter requires a value > 0",
    ),
    Msg::new(
        "cmd.hole.err.cbore_pilot_ge_sink",
        "底孔径 Ø{base} 不小于沉孔直径 Ø{sink}（GB/T 152.3 M{d}）",
        "Pilot diameter Ø{base} is not smaller than the counterbore diameter Ø{sink} (GB/T 152.3 M{d})",
    ),
    Msg::new(
        "cmd.hole.err.csink_pilot_ge_sink",
        "底孔径 Ø{base} 不小于埋头直径 Ø{sink}（GB/T 152.2 M{d}）",
        "Pilot diameter Ø{base} is not smaller than the countersink diameter Ø{sink} (GB/T 152.2 M{d})",
    ),
    Msg::new(
        "cmd.hole.err.full_thread_depth_manual",
        "螺纹有效长度选「全长」时孔深必须手填（自动孔深会循环依赖）",
        "With full-length thread length the hole depth must be entered manually (automatic depth would be circular)",
    ),
    Msg::new(
        "cmd.hole.err.npt_no_eff_len",
        "NPT{d} 表里没有螺纹有效长度 eff_len —— 表外不插值；请手填有效长度",
        "No effective thread length eff_len in the NPT{d} table — no interpolation outside the table; enter the effective length manually",
    ),
    Msg::new(
        "cmd.hole.err.r_no_eff_len",
        "R{d} 表里没有螺纹有效长度 eff_len（表第16栏）—— 表外不插值；请手填有效长度",
        "No effective thread length eff_len in the R{d} table (column 16) — no interpolation outside the table; enter the effective length manually",
    ),
    Msg::new(
        "cmd.hole.err.g_no_eff_len",
        "G{d} 没有有效长度（ISO 228-1 不规定；本表取同规格 R，而 R 无此规格）—— 不臆造：请手填有效长度，或改用 R / M",
        "G{d} has no effective length (ISO 228-1 does not define one; this table takes the same-size R, and R has no such size) — nothing is invented: enter the effective length manually, or switch to R / M",
    ),
    Msg::new(
        "cmd.hole.err.thread_len_invalid",
        "螺纹有效长度 L={len} 非法（必须 > 0）",
        "Effective thread length L={len} is invalid (must be > 0)",
    ),
    Msg::new(
        "cmd.hole.err.through_depth_manual",
        "贯通孔的孔深是板厚/通孔长度，不能自动 —— 请手填孔深",
        "For a through hole the depth is the plate thickness/through length and cannot be automatic — enter the hole depth manually",
    ),
    Msg::new(
        "cmd.hole.err.hole_depth_invalid",
        "孔深 H={h} 非法（必须 > 0）",
        "Hole depth H={h} is invalid (must be > 0)",
    ),
    Msg::new(
        "cmd.hole.err.thread_len_gt_hole_depth",
        "螺纹有效长度 L={l} 大于孔深 H={h}（有效长度必须在孔深内）",
        "Effective thread length L={l} exceeds hole depth H={h} (the effective length must fit inside the depth)",
    ),
    Msg::new(
        "cmd.hole.warn.runout_below_2p",
        "孔深 H={h} < 螺纹有效长度 L={l} + 2P={need}（工艺余量不足 2P）",
        "Hole depth H={h} < effective thread length L={l} + 2P={need} (runout margin below 2P)",
    ),
    Msg::new(
        "cmd.hole.err.simple_depth_manual",
        "简单孔：孔深不能自动（「自动」只用于螺纹孔）—— 请手填孔深",
        "Simple hole: depth cannot be automatic (automatic is only for threaded holes) — enter the hole depth manually",
    ),
    Msg::new(
        "cmd.hole.err.unthreaded_cbore_depth_manual",
        "不带螺纹的沉头/埋头孔：孔深不能自动 —— 请手填孔深",
        "Unthreaded counterbore/countersink: depth cannot be automatic — enter the hole depth manually",
    ),
    Msg::new(
        "cmd.hole.err.unthreaded_with_thread_subtype",
        "不带螺纹的孔不能用标准/细牙螺纹子类型",
        "An unthreaded hole cannot use the standard/fine-thread subtype",
    ),
    Msg::new(
        "cmd.hole.err.not_initialized",
        "这张图还没跑过 OCSM 初始化{why} —— 先执行 OCSM（建图层 + 线型 + 样式），再生成孔。",
        "This drawing has not run OCSM initialization{why} — run OCSM first (create layers + linetypes + styles), then generate holes.",
    ),
    Msg::new(
        "cmd.hole.why.missing_layers",
        "（缺图层：{layers}）",
        " (missing layers: {layers})",
    ),
    Msg::new(
        "cmd.hole.why.center_linetype",
        "（{layer} 没挂 CENTER2 点划线）",
        " ({layer} has no CENTER2 dash-dot linetype)",
    ),
    Msg::new(
        "cmd.hole.usage",
        "OCSMHOLE / DK：\n`OCSMHOLE [简单孔|螺纹孔|沉头孔|埋头孔] [带螺纹|无螺纹] [公制|UN/UNC/UNF/UNEF|G/BSPP|R/BSPT|NPT|ACME|矮牙|Tr] [钻孔 Ø8.5|自定义 孔径8.5|间隙 中等装配] [规格名 M10|M10×1.25|G1/8|1/4-20|NPT1/2|Tr8×1.5|ACME 1/4-16|公称6.35 P1.058] [P1.5] [推荐70.1|70.2] [H18] [L15] [盲孔|贯通] [全长] [6H|6G] [view 侧视图|俯视图|双视图] [at x,y] [rot 度]`\n（规格名直接查 GUI 同一份表 `name`：G 1/8、1/4-20、NPT1/2、Tr8x1.5 等都收；数值写法 `公称6.35 P1.058` 照旧）",
        "OCSMHOLE / DK:\n`OCSMHOLE [简单孔|螺纹孔|沉头孔|埋头孔] [带螺纹|无螺纹] [公制|UN/UNC/UNF/UNEF|G/BSPP|R/BSPT|NPT|ACME|矮牙|Tr] [钻孔 Ø8.5|自定义 孔径8.5|间隙 中等装配] [规格名 M10|M10×1.25|G1/8|1/4-20|NPT1/2|Tr8×1.5|ACME 1/4-16|公称6.35 P1.058] [P1.5] [推荐70.1|70.2] [H18] [L15] [盲孔|贯通] [全长] [6H|6G] [view 侧视图|俯视图|双视图] [at x,y] [rot deg]`\n(size names are looked up directly in the same `name` table as the GUI: G 1/8, 1/4-20, NPT1/2, Tr8x1.5 etc. are all accepted; the numeric form `公称6.35 P1.058` still works)",
    ),
    Msg::new(
        "cmd.hole.err.missing_args",
        "缺少参数",
        "Missing arguments",
    ),
    Msg::new(
        "cmd.hole.err.value_missing",
        "`{t}` 后面缺数值",
        "`{t}` is missing its value",
    ),
    Msg::new(
        "cmd.hole.err.spec_name_ambiguous",
        "规格名「{text}」命中多行（{names}）—— 请写全（带牙型系列/螺距）",
        "Spec name \"{text}\" matches multiple rows ({names}) — write it in full (with thread family/pitch)",
    ),
    Msg::new(
        "cmd.hole.err.view_name_missing",
        "`view` 后面缺视图名",
        "`view` is missing a view name",
    ),
    Msg::new(
        "cmd.hole.err.view_unknown",
        "不认识的视图「{other}」（侧视图/俯视图/双视图）",
        "Unrecognized view \"{other}\" (侧视图/俯视图/双视图)",
    ),
    Msg::new(
        "cmd.hole.err.at_xy_missing",
        "`at` 后面缺 x,y",
        "`at` is missing x,y",
    ),
    Msg::new(
        "cmd.hole.err.at_not_xy",
        "`at` 坐标应是 x,y（收到 {v}）",
        "`at` coordinates must be x,y (got {v})",
    ),
    Msg::new(
        "cmd.hole.err.x_invalid",
        "x 座标非法：{x}",
        "Invalid x coordinate: {x}",
    ),
    Msg::new(
        "cmd.hole.err.y_invalid",
        "y 座标非法：{y}",
        "Invalid y coordinate: {y}",
    ),
    Msg::new(
        "cmd.hole.err.rot_missing",
        "`rot` 后面缺角度",
        "`rot` is missing the angle",
    ),
    Msg::new(
        "cmd.hole.err.rot_invalid",
        "转角非法：{v}",
        "Invalid rotation angle: {v}",
    ),
    Msg::new(
        "cmd.hole.err.pitch_invalid",
        "螺距 P 非法：{v}",
        "Invalid pitch P: {v}",
    ),
    Msg::new(
        "cmd.hole.err.pitch_nonpositive",
        "螺距 P={p} 必须 > 0",
        "Pitch P={p} must be > 0",
    ),
    Msg::new(
        "cmd.hole.err.depth_invalid",
        "孔深非法：{v}",
        "Invalid hole depth: {v}",
    ),
    Msg::new(
        "cmd.hole.err.thread_len_parse_invalid",
        "螺纹有效长度非法：{v}",
        "Invalid effective thread length: {v}",
    ),
    Msg::new(
        "cmd.hole.err.custom_d_invalid",
        "自定义孔径非法：{v}",
        "Invalid custom hole diameter: {v}",
    ),
    Msg::new(
        "cmd.hole.err.fit_unsupported",
        "配合「{v}」不支持（6H/6G 或 精装配/中等装配/粗装配）",
        "Fit \"{v}\" is not supported (6H/6G or 精装配/中等装配/粗装配)",
    ),
    Msg::new(
        "cmd.hole.err.nominal_invalid",
        "公称直径非法：{rest}",
        "Invalid nominal diameter: {rest}",
    ),
    Msg::new(
        "cmd.hole.err.size_invalid",
        "尺寸非法：{t}（例 M10 / M10x1.25 / Ø8.5）",
        "Invalid size: {t} (e.g. M10 / M10x1.25 / Ø8.5)",
    ),
    Msg::new(
        "cmd.hole.err.pitch_token_invalid",
        "螺距非法：{t}",
        "Invalid pitch: {t}",
    ),
    Msg::new(
        "cmd.hole.err.unknown_param",
        "不识别的参数「{other}」（用法：{usage}）",
        "Unrecognized argument \"{other}\" (usage: {usage})",
    ),
    Msg::new(
        "cmd.hole.err.system_spec_mismatch",
        "体系与规格不匹配：{sys} 体系里没有「{text}」—— 这是 {conflicts}；请先写对体系，或改用本体系规格（示例：{examples}）",
        "System/spec mismatch: the {sys} system has no \"{text}\" — this is {conflicts}; write the correct system first, or use a spec from this system (examples: {examples})",
    ),
    Msg::new(
        "cmd.hole.err.spec_name_unknown",
        "未知规格名「{text}」（{sys} 体系）—— 表内示例：{examples}",
        "Unknown spec name \"{text}\" ({sys} system) — table examples: {examples}",
    ),
    Msg::new(
        "cmd.hole.err.size_missing",
        "缺少大小（例 `M10` / `M10×1.25`）",
        "Missing size (e.g. `M10` / `M10×1.25`)",
    ),
    Msg::new(
        "cmd.hole.err.need_one_view",
        "至少勾选一个视图（侧视图/俯视图）",
        "Select at least one view (侧视图/俯视图)",
    ),
    Msg::new(
        "cmd.thread.err.group_missing",
        "{sys} 没有子类型「{group}」（可用：{keys}）",
        "{sys} has no subtype \"{group}\" (available: {keys})",
    ),
    Msg::new(
        "cmd.thread.err.spec_missing",
        "{sys} 表里没有 {d}（mm, P={p}）—— 表外不插值；表内示例：{samples}",
        "The {sys} table has no {d} (mm, P={p}) — no interpolation outside the table; table examples: {samples}",
    ),
    Msg::new("cmd.thread.coarse_short", "粗牙", "coarse"),
    Msg::new(
        "cmd.thread.err.ambiguous",
        "{sys} {d} 命中多行（{names}）—— 请指定子类型/螺距",
        "{sys} {d} matches multiple rows ({names}) — specify a subtype/pitch",
    ),

    // ── 命令输出/报错：OCSMHOLE 命令层回执（lib.rs::cmd_hole）──────────
    Msg::new(
        "cmd.hole.err.parse_wrap",
        "OCSMHOLE 参数无效：{message}",
        "OCSMHOLE invalid arguments: {message}",
    ),
    Msg::new(
        "cmd.hole.err.geometry_wrap",
        "OCSMHOLE 几何非法：{message}",
        "OCSMHOLE invalid geometry: {message}",
    ),
    Msg::new(
        "cmd.hole.info.run_init_direct",
        "OCSMHOLE：先运行 OCSM（或点功能区「图幅」组里的 OCSM 初始化），再直接插孔。",
        "OCSMHOLE: run OCSM first (or click OCSM initialization in the ribbon Frame group), then insert holes directly.",
    ),
    Msg::new(
        "cmd.hole.info.run_init_window",
        "OCSMHOLE：先运行 OCSM 初始化，再打开孔生成器窗口。",
        "OCSMHOLE: run OCSM initialization first, then open the hole generator window.",
    ),
    Msg::new(
        "cmd.hole.err.server_start",
        "OCSMHOLE: 无法启动孔服务（宿主不支持 worker 请求）。",
        "OCSMHOLE: cannot start the hole service (host does not support worker requests).",
    ),
    Msg::new(
        "cmd.hole.info.window_opened",
        "OCSM 孔生成器：已打开窗口（类型 2×2 + 子类型/大小/配合 + 盲孔/贯通 + 孔深/螺纹范围自动）。点「确定」→ 回到图纸点击定位基点 → 移动光标旋转 → 再点击落定（可连续，Esc 结束）。",
        "OCSM Hole Generator: window opened (type 2×2 + subtype/size/fit + blind/through + automatic depth/thread range). Click OK → back in the drawing click the base point → move the cursor to rotate → click to place (repeatable; Esc to finish).",
    ),
    Msg::new(
        "cmd.hole.info.window_focus",
        "OCSM 孔生成器：窗口已打开（Alt+Tab 切换过去）。",
        "OCSM Hole Generator: the window is already open (Alt+Tab to switch to it).",
    ),
    Msg::new(
        "cmd.hole.hint.click_ok",
        "请在孔生成器窗口里点「确定」",
        "Click OK in the hole generator window",
    ),
    Msg::new("cmd.hole.place_what", "OCSM 孔", "OCSM Hole"),
    Msg::new("cmd.hole.kind.simple", "简单孔", "simple hole"),
    Msg::new("cmd.hole.kind.threaded", "螺纹孔", "threaded hole"),
    Msg::new("cmd.hole.kind.counterbore", "沉头孔", "counterbore"),
    Msg::new("cmd.hole.kind.countersink", "埋头孔", "countersink"),
    Msg::new(
        "cmd.hole.echo.created",
        "OCSMHOLE：已生成{kind} {size}（底孔Ø{drill}，深{depth}，{count} 个图元）于 ({x}, {y}) rot {rot}° → 1轮廓实线层 / 2细线层 / 3中心线层",
        "OCSMHOLE: created {kind} {size} (pilot Ø{drill}, depth {depth}, {count} entities) at ({x}, {y}) rot {rot}° → 1轮廓实线层 / 2细线层 / 3中心线层",
    ),
    Msg::new(
        "cmd.hole.info.warn_prefix",
        "OCSMHOLE 提示：{w}",
        "OCSMHOLE notice: {w}",
    ),

    // ── 命令输出/提示：放置态 prompt（lib.rs::PartPlace；★ 各命令共用）─────
    Msg::new(
        "cmd.place.prompt.wait",
        "{what}：{where_to}，然后在此点击放置。",
        "{what}: {where_to}, then click here to place.",
    ),
    Msg::new(
        "cmd.place.prompt.follow",
        "{what}：{l} —— 点击定位基点（可连续，Esc 结束）",
        "{what}: {l} — click to set the base point (repeatable; Esc to finish)",
    ),
    Msg::new(
        "cmd.place.prompt.rotate",
        "{what}：{l} —— 移动光标绕基点旋转，再点击落定（Esc 取消）",
        "{what}: {l} — move the cursor to rotate about the base point, then click to place (Esc to cancel)",
    ),

    // ── 命令输出/报错：中心线（OCSMCENTERLINE / ZX；centerline.rs）──────
    Msg::new(
        "cmd.centerline.err.bad_pick",
        "只能点圆/圆弧或直线",
        "Pick a circle/arc or a line",
    ),
    Msg::new(
        "cmd.centerline.err.zero_len_first",
        "这根线长度是 0，换一根",
        "This line has zero length; pick another",
    ),
    Msg::new(
        "cmd.centerline.err.zero_len_pair",
        "两根线里有一根长度是 0，重来",
        "One of the two lines has zero length; start over",
    ),
    Msg::new(
        "cmd.centerline.prompt.second",
        "OCSMCENTERLINE：再点第二根直线（角平分线中心线），Enter 取消",
        "OCSMCENTERLINE: pick the second line (angle-bisector centerline); Enter to cancel",
    ),
    Msg::new(
        "cmd.centerline.prompt.notice",
        "OCSMCENTERLINE：{n}",
        "OCSMCENTERLINE: {n}",
    ),
    Msg::new(
        "cmd.centerline.prompt.pick",
        "OCSMCENTERLINE 中心线：点圆/圆弧（十字）或直线（角平分线）",
        "OCSMCENTERLINE centerline: click a circle/arc (cross) or a line (bisector)",
    ),
    Msg::new(
        "cmd.centerline.err.no_object",
        "没点到对象：请点在圆/圆弧或直线上",
        "Nothing picked: click on a circle/arc or a line",
    ),
    Msg::new(
        "cmd.centerline.err.bad_selection",
        "OCSMCENTERLINE：选区不是「1 个圆/圆弧」或「2 根直线」→ 改为点选",
        "OCSMCENTERLINE: selection is not 1 circle/arc or 2 lines → switching to click picking",
    ),
    Msg::new(
        "cmd.centerline.undo_label",
        "OCSMCENTERLINE 中心线",
        "OCSMCENTERLINE centerline",
    ),
    Msg::new(
        "cmd.centerline.info.cross",
        "OCSMCENTERLINE：十字中心线（Ø{dia} + {scale}×{over}，线长 {len}）→ {layer}",
        "OCSMCENTERLINE: cross centerline (Ø{dia} + {scale}×{over}, line length {len}) → {layer}",
    ),
    Msg::new(
        "cmd.centerline.info.bisector",
        "OCSMCENTERLINE：角平分线中心线（长 {len} = 投影 {proj} + {scale}×{over}）→ {layer}",
        "OCSMCENTERLINE: angle-bisector centerline (length {len} = projection {proj} + {scale}×{over}) → {layer}",
    ),
    Msg::new(
        "cmd.centerline.info.cross_selection",
        "OCSMCENTERLINE：十字中心线（Ø{dia} + {scale}×{over}）→ {layer}",
        "OCSMCENTERLINE: cross centerline (Ø{dia} + {scale}×{over}) → {layer}",
    ),
    Msg::new(
        "cmd.centerline.info.bisector_selection",
        "OCSMCENTERLINE：角平分线中心线（第一根线 = 选区里第一个，长 {len}）→ {layer}",
        "OCSMCENTERLINE: angle-bisector centerline (first line = first in selection, length {len}) → {layer}",
    ),
    Msg::new(
        "cmd.centerline.info.draw_count",
        "OCSMCENTERLINE：已画出 {n} 条中心线",
        "OCSMCENTERLINE: drew {n} centerlines",
    ),

    // ── 命令输出/报错：序号球标（balloon.rs）────────────────────────
    Msg::new(
        "cmd.balloon.err.need_item",
        "序号标注：至少要有一个序号",
        "Item-number annotation: at least one item number is required",
    ),
    Msg::new(
        "cmd.balloon.err.blank_item",
        "序号标注：序号不能为空",
        "Item-number annotation: item numbers cannot be blank",
    ),
    Msg::new(
        "cmd.balloon.err.bad_scale",
        "序号标注：图幅倍率无效",
        "Item-number annotation: invalid frame scale",
    ),
    Msg::new(
        "cmd.balloon.err.not_horizontal",
        "序号标注：横线必须水平（第二段昣平画）——不支持竖肩线（序号不旋转）",
        "Item-number annotation: the horizontal line must be horizontal (the second segment is drawn flat) — vertical shoulder lines are not supported (item numbers are never rotated)",
    ),
    Msg::new(
        "cmd.balloon.err.zero_segment",
        "序号标注：第二段（横线方向段）长度为零",
        "Item-number annotation: the second segment (horizontal direction) has zero length",
    ),

    // ── 命令输出/报错：图框（TF / OCSMFRAMEINIT / OCSMFRAMEINSERT；lib.rs）──
    Msg::new(
        "cmd.frame.err.folder_missing",
        "找不到图框文件夹 {dir}。请在插件目录的 frame/ 中放入 DWG。",
        "Frame folder {dir} not found. Put DWG files in frame/ inside the plugin folder.",
    ),
    Msg::new(
        "cmd.frame.err.no_dwg",
        "{dir} 下没有 DWG 图框文件。",
        "No DWG frame files under {dir}.",
    ),
    Msg::new(
        "cmd.frame.err.need_name",
        "图框目录里有 {n} 个图框，请指名一个：{names}。",
        "The frame folder has {n} frames; name one: {names}.",
    ),
    Msg::new(
        "cmd.frame.err.name_missing",
        "没有叫「{want}」的图框。现有：{names}。",
        "No frame named \"{want}\". Available: {names}.",
    ),
    Msg::new(
        "cmd.frame.err.name_ambiguous",
        "「{want}」匹配到多个图框（{names}），请写全名。",
        "\"{want}\" matches multiple frames ({names}); write the full name.",
    ),
    Msg::new(
        "cmd.frame.info.found_n",
        "OCSMFRAMEINIT: 找到 {n} 个图框，正在打开选择窗口…",
        "OCSMFRAMEINIT: found {n} frames; opening the picker…",
    ),
    Msg::new(
        "cmd.frame.err.picker_failed",
        "OCSMFRAMEINIT: 无法打开图框选择窗口。",
        "OCSMFRAMEINIT: cannot open the frame picker.",
    ),
    Msg::new(
        "cmd.frame.err.bad_scale",
        "{who}: 非法的比例（必须为两个正整数且其一为 1，如 1:2 / 2:1 / 1:1）。",
        "{who}: invalid scale (two positive integers, one of which must be 1, e.g. 1:2 / 2:1 / 1:1).",
    ),
    Msg::new(
        "cmd.frame.prompt.specify_point",
        "指定图框「{block}」的插入点（比例 {scale}）：",
        "Specify the insertion point of frame \"{block}\" (scale {scale}):",
    ),
    Msg::new(
        "cmd.frame.info.specify_point",
        "指定图框「{block}」的插入点（比例 {scale}，缩放 {s:.2} 倍）…",
        "Specify the insertion point of frame \"{block}\" (scale {scale}, zoom {s:.2}×)…",
    ),
    Msg::new(
        "cmd.frame.info.inserted",
        "OCSMFRAMEINSERT: 已插入图框「{block}」比例 {scale}（缩放 {s:.2} 倍）基点 ({x}, {y})，旋转 {rot}°",
        "OCSMFRAMEINSERT: inserted frame \"{block}\" scale {scale} (zoom {s:.2}×) at ({x}, {y}), rotation {rot}°",
    ),
    Msg::new(
        "cmd.frame.output.inserted",
        "图框 {block} 插入完成（比例 {scale}）",
        "Frame {block} inserted (scale {scale})",
    ),
    Msg::new("cmd.frame.undo_label", "OCSM 插入图框", "OCSM insert frame"),
    Msg::new(
        "cmd.frame.err.no_pending",
        "OCSMFRAMEINSERT: 没有待处理的图框选择。请先运行 TF。",
        "OCSMFRAMEINSERT: no pending frame selection. Run TF first.",
    ),

    // ── 命令输出/报错：TF 参数校验（lib.rs::parse_frame_args / parse_point / parse_scale）──
    Msg::new(
        "cmd.tf.err.at_twice",
        "基点给了两次。{usage}",
        "Base point given twice. {usage}",
    ),
    Msg::new(
        "cmd.tf.err.at_needs_coords",
        "`at` 后面要给坐标，如 at 0,0。{usage}",
        "`at` must be followed by coordinates, e.g. at 0,0. {usage}",
    ),
    Msg::new(
        "cmd.tf.err.rot_needs_angle",
        "`rot` 后面要给角度。{usage}",
        "`rot` must be followed by an angle. {usage}",
    ),
    Msg::new(
        "cmd.tf.err.rot_not_number",
        "旋转角度「{next}」不是数字。{usage}",
        "Rotation angle \"{next}\" is not a number. {usage}",
    ),
    Msg::new(
        "cmd.tf.err.scale_twice",
        "比例给了两次。{usage}",
        "Scale given twice. {usage}",
    ),
    Msg::new(
        "cmd.tf.err.scale_invalid",
        "比例必须是两个正整数且其一为 1（例 1:2 / 2:1 / 1:1）。",
        "Scale must be two positive integers, one of which is 1 (e.g. 1:2 / 2:1 / 1:1).",
    ),
    Msg::new(
        "cmd.tf.err.point_form",
        "坐标「{tok}」应写成 x,y 或 x,y,z。",
        "Coordinate \"{tok}\" must be written as x,y or x,y,z.",
    ),
    Msg::new(
        "cmd.tf.err.point_not_number",
        "坐标「{tok}」里「{p}」不是数字。",
        "\"{p}\" in coordinate \"{tok}\" is not a number.",
    ),
    Msg::new(
        "cmd.tf.err.scale_form",
        "比例「{tok}」写法不对（示例 1:2）。",
        "Scale \"{tok}\" has the wrong form (e.g. 1:2).",
    ),
    Msg::new(
        "cmd.tf.err.scale_not_number",
        "比例「{tok}」不是数字（示例 1:2 / 2 / 0.5）。",
        "Scale \"{tok}\" is not a number (e.g. 1:2 / 2 / 0.5).",
    ),
    Msg::new(
        "cmd.tf.err.scale_not_positive",
        "比例「{tok}」必须是正数。",
        "Scale \"{tok}\" must be positive.",
    ),
    Msg::new(
        "cmd.tf.err.scale_part_not_number",
        "比例「{whole}」里「{s}」不是数字。",
        "\"{s}\" in scale \"{whole}\" is not a number.",
    ),
    Msg::new(
        "cmd.tf.err.scale_not_integer_ratio",
        "比例「{whole}」必须是整数比（示例 1:2）。",
        "Scale \"{whole}\" must be an integer ratio (e.g. 1:2).",
    ),

    // ── 命令输出/报错：螺栓副装配（OCSMJOINT；joint.rs）───────────────────
    Msg::new(
        "cmd.joint.usage",
        "用法：OCSMJOINT at x,y rot 度 [protrude 扣数] bolt=<族>:<d>[:<l>] [plate=<厚>|gap=<厚>|<螺母或垫圈族>=<d>] …",
        "Usage: OCSMJOINT at x,y rot deg [protrude turns] bolt=<family>:<d>[:<l>] [plate=<t>|gap=<t>|<nut or washer family>=<d>] …",
    ),
    Msg::new(
        "cmd.joint.err.at_needs_coords",
        "at 缺坐标。{usage}",
        "at: missing coordinates. {usage}",
    ),
    Msg::new(
        "cmd.joint.err.at_needs_y",
        "at 缺 y。{usage}",
        "at: missing y. {usage}",
    ),
    Msg::new(
        "cmd.joint.err.rot_needs_angle",
        "rot 缺角度。{usage}",
        "rot: missing angle. {usage}",
    ),
    Msg::new(
        "cmd.joint.err.protrude_needs_number",
        "protrude 缺数值。{usage}",
        "protrude: missing value. {usage}",
    ),
    Msg::new(
        "cmd.joint.err.protrude_negative",
        "protrude 不能为负（扣数）",
        "protrude cannot be negative (turns)",
    ),
    Msg::new(
        "cmd.joint.err.view_needs_name",
        "view 缺视图名。{usage}",
        "view: missing view name. {usage}",
    ),
    Msg::new(
        "cmd.joint.err.at_required",
        "缺 at x,y。{usage}",
        "Missing at x,y. {usage}",
    ),
    Msg::new(
        "cmd.joint.err.empty_chain",
        "件链为空。{usage}",
        "The part chain is empty. {usage}",
    ),
    Msg::new(
        "cmd.joint.err.empty_chain_bare",
        "件链为空",
        "the part chain is empty",
    ),
    Msg::new(
        "cmd.joint.err.chain_needs_bolt",
        "件链必须以 bolt=<族>:<d> 开头（它决定轴线与长度基准）。",
        "The part chain must start with bolt=<family>:<d> (it defines the axis and the length datum).",
    ),
    Msg::new(
        "cmd.joint.err.chain_needs_bolt_short",
        "件链必须以 bolt=<族>:<d> 开头",
        "The part chain must start with bolt=<family>:<d>",
    ),
    Msg::new(
        "cmd.joint.err.bad_json",
        "请求 JSON 无效: {e}",
        "Invalid request JSON: {e}",
    ),
    Msg::new(
        "cmd.joint.err.unknown_kind_json",
        "未知件类型: {other}",
        "Unknown part kind: {other}",
    ),
    Msg::new(
        "cmd.joint.err.chain_needs_bolt_json",
        "件链必须以 kind=bolt 开头",
        "The part chain must start with kind=bolt",
    ),
    Msg::new(
        "cmd.joint.err.family_no_size",
        "零件库没有 {family} 的 {d}，可用直径：{hint}",
        "The parts library has no {d} in {family}. Available diameters: {hint}",
    ),
    Msg::new(
        "cmd.joint.err.no_pitch",
        "{family} {d} 没有螺距数据，无法按扣数算露出",
        "{family} {d} has no pitch data; cannot compute protrusion in turns",
    ),
    Msg::new(
        "cmd.joint.err.item_size_missing",
        "{item} 的规格在零件库里查不到",
        "No library entry for the size of {item}",
    ),
    Msg::new(
        "cmd.joint.err.length_too_short",
        "{family} {d} 的供货长度最大 {lmax}，装不下（需 ≥ {need}）。换更长的族或改件链。",
        "{family} {d}: longest supplied length is {lmax}, too short (need ≥ {need}). Use a longer family or change the chain.",
    ),
    Msg::new(
        "cmd.joint.err.item_format",
        "件格式应为 `<类型>=<族>:<规格>` 或 `plate=<厚>`（收到 {token}）",
        "Part format must be `<kind>=<family>:<size>` or `plate=<t>` (got {token})",
    ),
    Msg::new(
        "cmd.joint.err.item_colon",
        "{token} 应为 <类型>=<族>:<规格>，如 bolt=hex_bolt_b_full:8",
        "{token} must be <kind>=<family>:<size>, e.g. bolt=hex_bolt_b_full:8",
    ),
    Msg::new(
        "cmd.joint.err.no_screw",
        "件链不支持螺钉（螺钉拧入螺纹孔、不配螺母）；螺栓副请用 六角头螺栓族",
        "The part chain does not accept screws (a screw threads into a tapped hole and takes no nut); use a hex-head bolt family for a bolted joint",
    ),
    Msg::new(
        "cmd.joint.err.unknown_kind",
        "未知件类型 {other}（可用 bolt/nut/washer/plate/gap，或直接写族名）",
        "Unknown part kind {other} (use bolt/nut/washer/plate/gap, or write the family name directly)",
    ),
    Msg::new(
        "cmd.joint.err.screw_not_expected",
        "{family} 是螺钉（screw）——螺钉拧入螺纹孔、不配螺母，不能当件链的 {expected} 用",
        "{family} is a screw — it threads into a tapped hole and takes no nut, so it cannot serve as the {expected} in the chain",
    ),
    Msg::new(
        "cmd.joint.err.family_kind_mismatch",
        "{family} 不是{expected}族",
        "{family} is not a {expected} family",
    ),
    Msg::new(
        "cmd.joint.err.size_format",
        "规格最多 <d>[:<l>]（收到 {raw}）",
        "Size must be at most <d>[:<l>] (got {raw})",
    ),
    Msg::new(
        "cmd.joint.err.library_no_d",
        "零件库没有 {family} 的 M{d}（可用：{hint}）",
        "The parts library has no M{d} in {family}. Available: {hint}",
    ),
    Msg::new(
        "cmd.joint.err.screw_not_bolt",
        "{family} 是螺钉（screw）——螺钉拧入螺纹孔、不配螺母，不是螺栓；件链只支持 螺栓（六角头螺栓族）/螺母/垫圈",
        "{family} is a screw — it threads into a tapped hole and takes no nut, so it is not a bolt. The chain supports only bolts (hex-head bolt families) / nuts / washers",
    ),
    Msg::new(
        "cmd.joint.err.kind_unsupported",
        "{family} 是 {other} 类，件链里只支持螺栓/螺母/垫圈",
        "{family} is of kind {other}; the chain supports only bolts/nuts/washers",
    ),
    Msg::new(
        "cmd.joint.err.family_not_implemented",
        "（该族未实现）",
        "(family not implemented)",
    ),
    Msg::new(
        "cmd.joint.note.no_plates",
        "无被连接件",
        "no connected parts",
    ),
    Msg::new(
        "cmd.joint.report",
        "OCSMJOINT：{bolt_name} {bolt_code} {d}×{l}｜件链 {chain} → Σ{stack}｜需 l ≥ {need}(= Σ{stack} + 露出{turns}扣×{pitch}) → 取供货长度 {l2}（实际外露 {pm}mm≈{pt}扣）｜基点 ({x}, {y}) rot {rot}°｜共 {n} 件",
        "OCSMJOINT: {bolt_name} {bolt_code} {d}×{l} | chain {chain} → Σ{stack} | need l ≥ {need} (= Σ{stack} + protrusion {turns} turns × {pitch}) → use supplied length {l2} (actual protrusion {pm} mm ≈ {pt} turns) | base point ({x}, {y}) rot {rot}° | {n} parts",
    ),
    Msg::new(
        "cmd.joint.item.bolt_with_length",
        "{name} {code} {d}×{l}",
        "{name} {code} {d}×{l}",
    ),
    Msg::new(
        "cmd.joint.item.bolt",
        "{name} {code} {d}",
        "{name} {code} {d}",
    ),
    Msg::new(
        "cmd.joint.item.nut",
        "{name} {code} {d}(m{thick})",
        "{name} {code} {d}(m{thick})",
    ),
    Msg::new(
        "cmd.joint.item.washer",
        "{name} {code} {d}(厚{thick})",
        "{name} {code} {d}(thk {thick})",
    ),
    Msg::new(
        "cmd.joint.item.plate",
        "板厚 {t}",
        "plate t={t}",
    ),
    Msg::new(
        "cmd.joint.item.gap",
        "间隔 {t}",
        "gap {t}",
    ),
    Msg::new(
        "cmd.joint.note.hidden",
        "{name} {spec} 被遮 {iv}",
        "{name} {spec} hidden {iv}",
    ),
    Msg::new(
        "cmd.joint.err.empty_geometry",
        "整链几何为空",
        "Joint geometry is empty",
    ),
    Msg::new(
        "cmd.joint.err.not_number",
        "不是有效数字: {raw}",
        "Not a valid number: {raw}",
    ),
    Msg::new(
        "cmd.joint.err.not_finite",
        "不是有限数字: {raw}",
        "Not a finite number: {raw}",
    ),

    // ── 命令输出/报错：一键转国标（OCSMDIM2GB / D2G；dim2gb.rs）───────
    Msg::new(
        "cmd.d2g.report",
        "OCSMDIM2GB：扫描原生标注 {seen} 个，转换成功 {converted} 个。",
        "OCSMDIM2GB: scanned {seen} native dimensions, converted {converted}.",
    ),
    Msg::new(
        "cmd.d2g.report.leaders",
        "其中引线标注 {n} 个。",
        " of which {n} were leaders.",
    ),
    Msg::new(
        "cmd.d2g.report.marks",
        "其中智能圆心标记 {n} 个（已换成 `3中心线层` 中心线）。",
        " of which {n} were smart center marks (replaced with `3中心线层` center lines).",
    ),
    Msg::new(
        "cmd.d2g.report.skipped",
        "跳过 {n} 个：",
        " skipped {n}:",
    ),
    Msg::new("cmd.d2g.report.none_skipped", "无跳过。", " none skipped."),
    Msg::new(
        "cmd.d2g.report.undo_hint",
        "（可用一次 Ctrl+Z 全部撤销）",
        "(one Ctrl+Z undoes them all)",
    ),
    Msg::new("cmd.d2g.report.sep", "；", "; "),
    Msg::new(
        "cmd.d2g.err.unsupported_request",
        "dim2gb 收集器不支持该请求: {req}",
        "dim2gb collector does not support this request: {req}",
    ),
    Msg::new("cmd.d2g.type.aligned", "对齐标注", "aligned dimension"),
    Msg::new("cmd.d2g.type.linear", "线性标注", "linear dimension"),
    Msg::new("cmd.d2g.type.radius", "半径标注", "radius dimension"),
    Msg::new("cmd.d2g.type.diameter", "直径标注", "diameter dimension"),
    Msg::new("cmd.d2g.type.angular", "角度标注", "angular dimension"),
    Msg::new("cmd.d2g.type.arc", "弧长标注", "arc-length dimension"),
    Msg::new("cmd.d2g.type.ordinate", "坐标标注", "ordinate dimension"),
    Msg::new("cmd.d2g.type.large_radial", "折弯半径标注", "bend-radius dimension"),
    Msg::new("cmd.d2g.err.zero_radius", "半径为零", "radius is zero"),
    Msg::new(
        "cmd.d2g.err.arc_zero_radius",
        "弧长标注半径为零",
        "arc-length dimension has zero radius",
    ),
    Msg::new(
        "cmd.d2g.err.temp_arc_block",
        "临时引导弧创建失败: {e}",
        "failed to create temporary guide arc: {e}",
    ),
    Msg::new(
        "cmd.d2g.err.diameter_coincident",
        "直径两点重合",
        "the two diameter points coincide",
    ),
    Msg::new(
        "cmd.d2g.err.arc_invalid",
        "弧长标注尺寸无效（可能来自旧文件解析）",
        "invalid arc-length dimension (possibly from parsing an old file)",
    ),
    Msg::new(
        "cmd.d2g.err.ordinate_unsupported",
        "坐标标注本期不转换（OCSM 无对应构件）",
        "ordinate dimensions are not converted yet (no OCSM equivalent)",
    ),
    Msg::new(
        "cmd.d2g.err.large_radial_unsupported",
        "折弯半径本期不转换（OCSM 无对应构件）",
        "bend-radius dimensions are not converted yet (no OCSM equivalent)",
    ),
    Msg::new(
        "cmd.d2g.skip.multileader",
        "多重引线（本期不转）",
        "multileader (not converted yet)",
    ),
    Msg::new(
        "cmd.d2g.skip.non_dimension",
        "非标注对象（{kind}）",
        "non-dimension object ({kind})",
    ),
    Msg::new(
        "cmd.d2g.err.leader_vertices",
        "顶点不足（需 箭头点→拐点→肩线末端）",
        "too few vertices (need arrow point → bend point → shoulder end)",
    ),
    Msg::new(
        "cmd.d2g.err.text_lines",
        "文字 {n} 行（>2 行不转换）",
        "text has {n} lines (>2 lines not converted)",
    ),
    Msg::new(
        "cmd.d2g.err.block_create",
        "建块失败：{e}",
        "failed to create block: {e}",
    ),
    Msg::new(
        "cmd.d2g.err.add_entity",
        "加实体失败：{e}",
        "failed to add entity: {e}",
    ),
    Msg::new(
        "cmd.d2g.skip.handle_missing",
        "handle {h} 不存在",
        "handle {h} does not exist",
    ),
    Msg::new(
        "cmd.d2g.err.geometry_check",
        "几何自检未通过（原 {before} → 新 {after}）",
        "geometry self-check failed (original {before} → new {after})",
    ),
    Msg::new(
        "cmd.d2g.info.none_found",
        "OCSMDIM2GB：未找到可转换的原生标注（仅扫描模型空间）{tail}",
        "OCSMDIM2GB: no convertible native dimensions found (model space only){tail}",
    ),
    Msg::new(
        "cmd.d2g.info.none_found_tail",
        "（{list}）",
        " ({list})",
    ),
    Msg::new(
        "cmd.d2g.undo",
        "OCSMDIM2GB 原生标注转 GB",
        "OCSMDIM2GB: native dimensions → GB",
    ),
    Msg::new(
        "cmd.d2g.err.block_create_named",
        "OCSMDIM2GB：建块 {name} 失败：{e}",
        "OCSMDIM2GB: failed to create block {name}: {e}",
    ),
    Msg::new(
        "cmd.d2g.skip.dim",
        "{label}（{e}）",
        "{label} ({e})",
    ),
    Msg::new(
        "cmd.d2g.skip.leader",
        "引线标注（{e}）",
        "leader ({e})",
    ),
    Msg::new(
        "cmd.d2g.skip.mark_bad_radius",
        "圆心标记（记录里的半径无效，已跳过）",
        "center mark (invalid radius in record, skipped)",
    ),
    Msg::new("cmd.d2g.ent.point", "点", "point"),
    Msg::new("cmd.d2g.ent.line", "直线", "line"),
    Msg::new("cmd.d2g.ent.circle", "圆", "circle"),
    Msg::new("cmd.d2g.ent.arc", "圆弧", "arc"),
    Msg::new("cmd.d2g.ent.polyline", "多段线", "polyline"),
    Msg::new("cmd.d2g.ent.text", "文字", "text"),
    Msg::new("cmd.d2g.ent.mtext", "多行文字", "mtext"),
    Msg::new("cmd.d2g.ent.insert", "块参照", "block reference"),
    Msg::new("cmd.d2g.ent.dimension", "标注", "dimension"),
    Msg::new("cmd.d2g.ent.solid", "实体填充", "solid fill"),
    Msg::new("cmd.d2g.ent.other", "其它", "other"),

    // ── 命令输出/报错：明细表 BOM 系列（bom.rs / bom_xlsx.rs）───────────
    Msg::new("cmd.bom.sep", "、", ", "),
    Msg::new(
        "cmd.bom.undo.table",
        "OCSM 明细表",
        "OCSM BOM table",
    ),
    Msg::new("cmd.bom.undo.qty_lock", "OCSM 数量锁", "OCSM quantity lock"),
    Msg::new("cmd.bom.undo.export", "OCSM 明细表导出", "OCSM BOM export"),
    Msg::new("cmd.bom.undo.import", "OCSM 明细表导入", "OCSM BOM import"),
    Msg::new(
        "cmd.bom.err.template_missing",
        "OCSMBOM: 找不到明细表模板块（{head} / {row}）。请把 OCSM_BOMHEAD.dwg、OCSM_BOMROW.dwg 放进插件目录 bom/（或设 OCSM_BOM_DIR）。",
        "OCSMBOM: BOM template blocks not found ({head} / {row}). Put OCSM_BOMHEAD.dwg and OCSM_BOMROW.dwg into the plugin's bom/ directory (or set OCSM_BOM_DIR).",
    ),
    Msg::new(
        "cmd.bom.err.bad_attdefs",
        "OCSMBOM: 行块 {block} 应有 {expect} 个 ATTDEF，实际 {got} 个 → 模板不对。",
        "OCSMBOM: row block {block} should have {expect} ATTDEFs but has {got} → wrong template.",
    ),
    Msg::new(
        "cmd.bom.err.layout_first_col",
        "图框/配置不对：首列连一行都放不下（检查 sheet_top / first_col_bottom）",
        "Frame/config problem: not even one row fits in the first column (check sheet_top / first_col_bottom)",
    ),
    Msg::new(
        "cmd.bom.err.layout_cont_col",
        "图框/配置不对：续列连一行都放不下（检查 sheet_top / sheet_bottom）",
        "Frame/config problem: not even one row fits in a continuation column (check sheet_top / sheet_bottom)",
    ),
    Msg::new(
        "cmd.bom.err.layout_columns",
        "明细表需要 {cols} 列（共 {rows} 行：首列 {first} 行/上限 {cap_first}，续列每列上限 {cap_cont}），当前图幅只放得下 {fit} 列 → 请换更大图幅，或改用多页明细表。",
        "The BOM needs {cols} columns ({rows} rows total: first column {first} rows / limit {cap_first}, continuation columns limit {cap_cont}) but the current sheet fits only {fit} columns → use a larger sheet or a multi-page BOM.",
    ),
    Msg::new(
        "cmd.bom.col.desc",
        "第{col}列 {n} 行",
        "column {col}: {n} rows",
    ),
    Msg::new(
        "cmd.bom.info.built",
        "OCSMBOM: {rows} 件 → {cols} 列（{desc}）；旧表元 {removed} 个已替换（Ctrl+Z 可整体撤销）；现有行里手改过的列与 BOMLOCK 数量锁已保留。",
        "OCSMBOM: {rows} parts → {cols} columns ({desc}); {removed} old table entities replaced (Ctrl+Z undoes all); manually edited cells and BOMLOCK quantity locks were kept.",
    ),
    Msg::new(
        "cmd.bom.info.count_hint",
        "OCSMBOM: 提示——数量按「图中插入件数」统计，同一零件画在多个视图里会重复计数。",
        "OCSMBOM: note — quantities count inserted occurrences in the drawing, so the same part drawn in several views is counted more than once.",
    ),
    Msg::new(
        "cmd.bom.info.untracked",
        "OCSMBOM: 注意——图中有 {n} 个 OCSM_ 零件块引用但没有 OCSM_PART 台账记录（多为离线生成/早期版本的文件），它们不会进明细表。用 XL 重新放置，或后续用表格导入补录。",
        "OCSMBOM: warning — {n} OCSM_ part block references have no OCSM_PART ledger record (usually from offline/older files); they will not appear in the BOM. Re-place them with XL, or add them later via table import.",
    ),
    Msg::new(
        "cmd.bom.err.sync_nothing",
        "OCSMBOMSYNC: 没有可排的内容（图上没有序号球标，也没有零件台账）。",
        "OCSMBOMSYNC: nothing to list (no balloon items and no part ledger in the drawing).",
    ),
    Msg::new(
        "cmd.bom.err.sync_failed",
        "OCSMBOMSYNC: {e}",
        "OCSMBOMSYNC: {e}",
    ),
    Msg::new(
        "cmd.bom.info.sync",
        "OCSMBOMSYNC: {rows} 行（序号 {nos}）→ {cols} 列；旧表元 {removed} 个已替换。",
        "OCSMBOMSYNC: {rows} rows (item numbers {nos}) → {cols} columns; {removed} old table entities replaced.",
    ),
    Msg::new(
        "cmd.bom.locked.item",
        "{no}（数量 {qty}）",
        "{no} (qty {qty})",
    ),
    Msg::new(
        "cmd.bom.info.sync_locked",
        "OCSMBOMSYNC: {n} 行数量已锁定，未覆盖：{list}。要重算用 `BOMLOCK <序号> off`。",
        "OCSMBOMSYNC: quantity locked on {n} rows, not overwritten: {list}. Use `BOMLOCK <item no> off` to unlock.",
    ),
    Msg::new(
        "cmd.bom.info.squeezed",
        "OCSMBOM: {n} 格文字超宽 → 已自动横向压缩（字高统一不动；压太扁的另点名）。",
        "OCSMBOM: {n} cells were too wide → compressed horizontally (text height unchanged; badly flattened ones are listed separately).",
    ),
    Msg::new(
        "cmd.bom.info.squeezed_hard",
        "OCSMBOM: 下面 {n} 格横向压得偏扁（已压进格内、不会到邻格，但可读性下降）：{list}（建议加宽该列 / 缩短文本 / 把标准号写进名称列）。",
        "OCSMBOM: {n} cells were flattened hard (still inside their cells but less readable): {list} (widen the column, shorten the text, or put the standard number in the name column).",
    ),
    Msg::new(
        "cmd.bom.squeezed_hard.item",
        "{tag}「{value}」{pct}%",
        "{tag} \"{value}\" {pct}%",
    ),
    Msg::new(
        "cmd.bom.lock.usage",
        "OCSMBOMLOCK / BOMLOCK 用法：BOMLOCK <序号|图号> [数量|off]（不给数量 = 按行上现值锁；off = 解锁）",
        "OCSMBOMLOCK / BOMLOCK usage: BOMLOCK <item no|drawing no> [qty|off] (no qty = lock at the current value; off = unlock)",
    ),
    Msg::new(
        "cmd.bom.lock.err.row_missing",
        "OCSMBOMLOCK: 表里找不到序号/图号为「{key}」的行。",
        "OCSMBOMLOCK: no row with item/drawing number \"{key}\" in the table.",
    ),
    Msg::new(
        "cmd.bom.lock.info.unlocked",
        "OCSMBOMLOCK: 序号 {key}→已解锁（数量 {qty}）——下次同步会按件数/引用次数重算。",
        "OCSMBOMLOCK: {key} → unlocked (qty {qty}); the next sync will recompute it from part counts/references.",
    ),
    Msg::new(
        "cmd.bom.lock.err.qty_invalid",
        "OCSMBOMLOCK: 数量「{v}」不是正整数（或写 off 解锁）。",
        "OCSMBOMLOCK: quantity \"{v}\" is not a positive integer (or write off to unlock).",
    ),
    Msg::new(
        "cmd.bom.lock.info.locked",
        "OCSMBOMLOCK: 序号 {key} 数量锁定为 {qty}（同步不再重算；BOMLOCK {key} off 可解锁）。",
        "OCSMBOMLOCK: {key} quantity locked to {qty} (sync will not recompute; BOMLOCK {key} off unlocks).",
    ),
    Msg::new(
        "cmd.bom.err.row_no_empty",
        "有行的序号为空（每行都需要序号）",
        "A row has an empty item number (every row needs one)",
    ),
    Msg::new(
        "cmd.bom.err.row_qty_not_positive_int",
        "序号 {no} 的数量「{qty}」不是正整数",
        "Row {no}: quantity \"{qty}\" is not a positive integer",
    ),
    Msg::new(
        "cmd.bom.err.row_qty_zero",
        "序号 {no} 的数量不能为 0（要删行就删行）",
        "Row {no}: quantity cannot be 0 (delete the row to remove it)",
    ),
    Msg::new(
        "cmd.bom.info.links_stamped",
        "OCSMBOM: 已把 {changed} 个表块链接指向明细表编辑页（Ctrl+点击打开；或运行 `BOMEDIT` 在新窗口打开）。",
        "OCSMBOM: {changed} table blocks now link to the BOM edit page (Ctrl+click, or run `BOMEDIT`).",
    ),
    Msg::new(
        "cmd.bom.err.export_empty",
        "OCSMBOMXLSX: 图上还没有明细表行（先 `BOM` 或 `BOMSYNC` 建表）。",
        "OCSMBOMXLSX: the drawing has no BOM rows yet (run `BOM` or `BOMSYNC` first).",
    ),
    Msg::new(
        "cmd.bom.err.write_failed",
        "写 {path} 失败: {e}",
        "Failed to write {path}: {e}",
    ),
    Msg::new(
        "cmd.bom.err.export_failed",
        "OCSMBOMXLSX: {e}",
        "OCSMBOMXLSX: {e}",
    ),
    Msg::new(
        "cmd.bom.info.export_temp_hint",
        "OCSMBOMXLSX: 提示——本图还没存过盘，所以文件落在**临时目录**（{dir}）。临时目录会被系统清理，要长期保存请先 Ctrl+S 存盘再导出（xlsx 就会生成在图纸同目录、同名-明细表.xlsx）。",
        "OCSMBOMXLSX: note — the drawing has not been saved, so the file went to a **temporary directory** ({dir}). Temp files get cleaned up; for long-term storage press Ctrl+S first, then the xlsx is created next to the drawing as <same name>-明细表.xlsx.",
    ),
    Msg::new(
        "cmd.bom.info.exported",
        "OCSMBOMXLSX: {rows} 行已导出 → {path}（{n} 行记下导出基线；表块已挂编辑页链接，Ctrl+点击打开网页）。",
        "OCSMBOMXLSX: {rows} rows exported → {path} ({n} rows recorded as export baseline; table blocks link to the edit page, Ctrl+click to open).",
    ),
    Msg::new(
        "cmd.bom.info.export_columns",
        "OCSMBOMXLSX: 列 = {cols}（「锁定数量」只在此文件里，不进图纸表格；空=不锁，Y/是=锁为同行数量，数字=锁为该数）。",
        "OCSMBOMXLSX: columns = {cols} (the \"locked quantity\" column exists only in this file, not in the drawing table; empty = unlocked, Y/是 = lock at the row quantity, a number = lock to that value).",
    ),
    Msg::new(
        "cmd.bom.err.import_missing",
        "OCSMBOMXLSXI: 找不到文件 {path}（先 `BOMXLSX` 导出，或给个路径）。",
        "OCSMBOMXLSXI: file not found: {path} (run `BOMXLSX` first, or give a path).",
    ),
    Msg::new(
        "cmd.bom.err.import_csv",
        "OCSMBOMXLSXI: 读 CSV 失败：{e}",
        "OCSMBOMXLSXI: failed to read CSV: {e}",
    ),
    Msg::new(
        "cmd.bom.err.import_empty",
        "OCSMBOMXLSXI: 文件里没有数据行。",
        "OCSMBOMXLSXI: the file has no data rows.",
    ),
    Msg::new(
        "cmd.bom.err.import_failed",
        "OCSMBOMXLSXI: {e}",
        "OCSMBOMXLSXI: {e}",
    ),
    Msg::new(
        "cmd.bom.info.imported",
        "OCSMBOMXLSXI: 从 {path} 导入 {n} 行 → 表 {rows} 行 / {cols} 列（{locked} 行数量已锁定，{kept} 行文件里没有、按图纸保留）。",
        "OCSMBOMXLSXI: imported {n} rows from {path} → {rows} rows / {cols} columns ({locked} rows keep quantity locks; {kept} rows absent from the file were kept from the drawing).",
    ),
    Msg::new(
        "cmd.bom.cfg.info.set",
        "OCSMBOMCFG: 首列行数已设为 {n}（{path}）。",
        "OCSMBOMCFG: first-column row count set to {n} ({path}).",
    ),
    Msg::new(
        "cmd.bom.cfg.err.save",
        "OCSMBOMCFG: 写配置失败：{e}",
        "OCSMBOMCFG: failed to save config: {e}",
    ),
    Msg::new(
        "cmd.bom.cfg.info.show",
        "OCSMBOMCFG: 首列行数 {n}（配置 {path}）；用法：BOMCFG 30 改默认，BOM 30 只改本次。",
        "OCSMBOMCFG: first-column row count {n} (config {path}); usage: BOMCFG 30 changes the default, BOM 30 changes only this run.",
    ),
    Msg::new(
        "cmd.bomxlsx.err.too_many_cols",
        "第 {row} 行有 {have} 列，超过 {max} 列（多出来的列无法识别）",
        "Row {row} has {have} columns, more than the {max} recognized (extra columns are ignored)",
    ),
    Msg::new(
        "cmd.bomxlsx.err.compress",
        "压缩 {name} 失败: {e}",
        "Failed to compress {name}: {e}",
    ),
    Msg::new(
        "cmd.bomxlsx.err.no_eocd",
        "不是有效的 xlsx/zip：找不到 EOCD",
        "Not a valid xlsx/zip: EOCD not found",
    ),
    Msg::new(
        "cmd.bomxlsx.err.zip_central_dir",
        "zip 中央目录损坏",
        "corrupt zip central directory",
    ),
    Msg::new(
        "cmd.bomxlsx.err.zip_local_header",
        "zip 条目 {name} 局部头损坏",
        "zip entry {name} has a corrupt local header",
    ),
    Msg::new(
        "cmd.bomxlsx.err.zip_out_of_bounds",
        "zip 条目 {name} 数据越界",
        "zip entry {name} data out of bounds",
    ),
    Msg::new(
        "cmd.bomxlsx.err.unzip",
        "解压 {name} 失败: {e}",
        "Failed to decompress {name}: {e}",
    ),
    Msg::new(
        "cmd.bomxlsx.err.unsupported_method",
        "zip 条目 {name} 用了不支持的压缩方法 {m}",
        "zip entry {name} uses unsupported compression method {m}",
    ),
    Msg::new(
        "cmd.bomxlsx.err.make_dir",
        "建目录失败: {e}",
        "Failed to create directory: {e}",
    ),
    Msg::new(
        "cmd.bomxlsx.err.parse_sheet",
        "解析 sheet 失败: {e}",
        "Failed to parse sheet: {e}",
    ),
    Msg::new(
        "cmd.bomxlsx.err.read_file",
        "读 {path} 失败: {e}",
        "Failed to read {path}: {e}",
    ),
    Msg::new(
        "cmd.bomxlsx.err.no_sheet",
        "xlsx 里找不到工作表",
        "no worksheet found in the xlsx",
    ),
    Msg::new(
        "cmd.bomxlsx.err.sheet_read",
        "工作表读取失败",
        "failed to read the worksheet",
    ),

    // ── 命令输出/报错：标准件库 / 结构要素（OCSMPART / XL；lib.rs + partgen*.rs + guide_server）──
    Msg::new(
        "cmd.parts.usage",
        "OCSMPART 参数无效。用法：标准件 `OCSMPART <族> <d> <l> [view <视图>] [at x,y] [rot 度]`（例：OCSMPART hex_bolt_c 10 95 at 150,30 rot 0）；结构要素 `OCSMPART detail_grind_od <d> [b1 <值>] [at x,y] [rot 度]`（b1 缺省 = 该 d 档默认行）；矩形花键 `OCSMPART detail_spline_rect <规格代号> L<满齿段长> [de <滚刀外径>] [view front|side|section]`（例：OCSMPART detail_spline_rect 6x23x26x6 L30 view side）；外螺纹退刀槽 `OCSMPART detail_thread_relief <d> P <螺距> [g1 值 g2 值 dg 值 r 值 alpha 值] [at x,y] [rot 度]`（P 必给）；毂槽 `OCSMPART detail_hub_keyway <d> [len 毂长] [view main|side]`（例：OCSMPART detail_hub_keyway 25 len 30 view main；b/t₂/r 由 d 查表，len 缺省 30）；平键 `OCSMPART key_1096_{a|b|c} <b> <L> [view main|top|section]`、`OCSMPART key_1097_{a|b} <b> <L> [view main|top]`（例：OCSMPART key_1096_a 4 8、OCSMPART key_1097_a 8 25；L 省略/0 = 该档默认，L 须 ∈ 标准系列且 L<10b，1097 的 L1/L2/L3 由 L 查长度系列表派生）；不带参数则打开零件库窗口。",
        "Invalid OCSMPART arguments. Usage: standard part `OCSMPART <family> <d> <l> [view <view>] [at x,y] [rot deg]` (e.g. OCSMPART hex_bolt_c 10 95 at 150,30 rot 0); feature `OCSMPART detail_grind_od <d> [b1 <value>] [at x,y] [rot deg]` (b1 omitted = the default row for that d); rectangular spline `OCSMPART detail_spline_rect <spec code> L<full-tooth length> [de <hob OD>] [view front|side|section]` (e.g. OCSMPART detail_spline_rect 6x23x26x6 L30 view side); external thread relief `OCSMPART detail_thread_relief <d> P <pitch> [g1 v g2 v dg v r v alpha v] [at x,y] [rot deg]` (P required); hub keyway `OCSMPART detail_hub_keyway <d> [len hub length] [view main|side]` (e.g. OCSMPART detail_hub_keyway 25 len 30 view main; b/t₂/r looked up from d, len defaults to 30); parallel keys `OCSMPART key_1096_{a|b|c} <b> <L> [view main|top|section]`, `OCSMPART key_1097_{a|b} <b> <L> [view main|top]` (e.g. OCSMPART key_1096_a 4 8, OCSMPART key_1097_a 8 25; L omitted/0 = default for that b, L must be a standard series value with L<10b, and key_1097 derives L1/L2/L3 from the length series table); no arguments opens the parts library window.",
    ),
    Msg::new(
        "cmd.parts.window_opened",
        "OCSM 标准件库：已打开零件库窗口。选零件点「零件出库」→ 回到图纸点击定位基点 → 移动光标旋转 → 再点击落定（可连续，Esc 结束）。",
        "OCSM parts library: window opened. Pick a part and click \"Produce part\" → back in the drawing click to set the base point → move the cursor to rotate → click to place (repeatable; Esc to finish).",
    ),
    Msg::new(
        "cmd.parts.window_exists",
        "OCSM 标准件库：零件库窗口已打开（Alt+Tab 切换过去）。",
        "OCSM parts library: the window is already open (Alt+Tab to switch to it).",
    ),
    Msg::new(
        "cmd.parts.err.no_service",
        "OCSM: 无法启动零件库服务（宿主不支持 worker 请求）。",
        "OCSM: cannot start the parts library service (the host does not support worker requests).",
    ),
    Msg::new(
        "cmd.parts.err.no_worker",
        "OCSM 标准件：宿主不支持 worker 请求，无法参数化插入。",
        "OCSM parts: the host does not support worker requests; parametric insert is unavailable.",
    ),
    Msg::new("cmd.parts.inserted_prefix", "OCSM 标准件：{msg}", "OCSM parts: {msg}"),
    Msg::new(
        "cmd.parts.err.insert_failed",
        "OCSM 标准件插入失败：{e}",
        "OCSM part insert failed: {e}",
    ),
    Msg::new("cmd.parts.undo_insert", "零件插入", "insert part"),
    Msg::new(
        "cmd.parts.inserted_item",
        "已插入 {name} {spec}（{code}）",
        "inserted {name} {spec} ({code})",
    ),
    Msg::new("cmd.req.err.bad_json", "请求 JSON 无效: {e}", "Invalid request JSON: {e}"),
    Msg::new("cmd.parts.prompt.what", "OCSM 标准件", "OCSM part"),
    Msg::new(
        "cmd.parts.prompt.where",
        "请在零件库窗口里点「零件出库」",
        "click \"Produce part\" in the parts library window",
    ),
    Msg::new(
        "cmd.parts.err.no_m",
        "{table} 数据表里没有 M{d}",
        "{table} data table has no M{d}",
    ),
    Msg::new(
        "cmd.parts.err.no_dia",
        "{table} 数据表里没有 Ø{d}",
        "{table} data table has no Ø{d}",
    ),
    Msg::new(
        "cmd.parts.err.len_range_std",
        "M{d} 的长度应在 {lo}~{hi}（标准范围）",
        "M{d} length must be within {lo}~{hi} (standard range)",
    ),
    Msg::new(
        "cmd.parts.err.len_range",
        "M{d} 的长度应在 {lo}~{hi} 之间",
        "M{d} length must be between {lo} and {hi}",
    ),
    Msg::new(
        "cmd.parts.err.len_range_ab",
        "M{d} 的长度应在 {lo}~{hi} 之间（GB/T 5782 A/B级）",
        "M{d} length must be between {lo} and {hi} (GB/T 5782 grade A/B)",
    ),
    Msg::new(
        "cmd.parts.err.len_range_series",
        "{std} Ø{d} 的长度应在 {lo} ~ {hi} 之间（表内系列：{series}）",
        "{std} Ø{d} length must be between {lo} and {hi} (in-table series: {series})",
    ),
    Msg::new(
        "cmd.parts.err.len_range_kit",
        "M{d} 的长度范围是 {lo}…{hi}（收到 {l}）",
        "M{d} length range is {lo}…{hi} (got {l})",
    ),
    Msg::new(
        "cmd.parts.err.view_not_offered",
        "{family} 不提供视图 {view}（可用：{avail}）",
        "{family} does not provide view {view} (available: {avail})",
    ),
    Msg::new(
        "cmd.parts.err.view_absent",
        "{family} 没有视图 {view}（可用：{avail}）",
        "{family} has no view {view} (available: {avail})",
    ),
    Msg::new(
        "cmd.parts.err.view_absent_bare",
        "{family} 没有视图 {view}",
        "{family} has no view {view}",
    ),
    Msg::new(
        "cmd.parts.err.only_main",
        "{family} 只有主视图（模板只有这一个视图）",
        "{family} only has the main view (the template has just this one)",
    ),
    Msg::new(
        "cmd.parts.err.only_main_received",
        "{family} 只有主视图（模板只有这一个视图），收到 {view}",
        "{family} only has the main view (the template has just this one); got {view}",
    ),
    Msg::new("cmd.parts.err.unknown_family", "未知族 {other}", "unknown family {other}"),
    Msg::new(
        "cmd.parts.err.unknown_nut_family",
        "未知螺母族 {other}",
        "unknown nut family {other}",
    ),
    Msg::new(
        "cmd.parts.err.family_todo",
        "零件族 {other} 尚未实现",
        "part family {other} is not implemented yet",
    ),
    Msg::new(
        "cmd.parts.err.nut6170_todo",
        "1型六角螺母 GB/T 6170-2015 暂未提供（画法待模板确认，数据表已备好）",
        "Hex nut style 1 GB/T 6170-2015 is not available yet (drawing method pending template confirmation; the data table is ready)",
    ),
    Msg::new(
        "cmd.parts.err.key_no_b",
        "{table} 数据表里没有 b={b}",
        "{table} data table has no b={b}",
    ),
    Msg::new(
        "cmd.parts.err.key_l_positive",
        "GB/T {std} b={b}：L 必须是正数（收到 {l}）",
        "GB/T {std} b={b}: L must be a positive number (got {l})",
    ),
    Msg::new(
        "cmd.parts.err.key_l_range",
        "GB/T {std} b={b} 的 L 取值范围是 {lo}…{hi}（L 系列值且 L<10b），收到 {l}",
        "GB/T {std} b={b}: L must be within {lo}…{hi} (a series value with L<10b); got {l}",
    ),
    Msg::new(
        "cmd.parts.err.key_l_range_1097",
        "GB/T 1097 b={b} 的 L 取值范围是 {lo}…{hi}（L 系列值且 L<10b，GB/T 1097-2003 注③），收到 {l}",
        "GB/T 1097 b={b}: L must be within {lo}…{hi} (a series value with L<10b, GB/T 1097-2003 note 3); got {l}",
    ),
    Msg::new(
        "cmd.parts.err.key_l_series",
        "GB/T 1096 b={b} 的 L 须取标准系列值（6, 8, 10, …, 400；L<10b），收到 {l}",
        "GB/T 1096 b={b}: L must be a standard series value (6, 8, 10, …, 400; L<10b); got {l}",
    ),
    Msg::new(
        "cmd.parts.err.key_series_missing",
        "GB/T 1097-2003 长度系列表里没有 L={l}（不插值/不外推）；可选 L（{n} 档）：{list}",
        "GB/T 1097-2003 length series has no L={l} (no interpolation/extrapolation); available L ({n} entries): {list}",
    ),
    Msg::new(
        "cmd.parts.err.key_chain",
        "GB/T 1097-2003 长度系列表 L={l} 尺寸链不一致：L1={l1}、L2={l2}、L3={l3}（应满足 L2=L/2、L1+2L3=L）",
        "GB/T 1097-2003 length series L={l} dimension chain inconsistent: L1={l1}, L2={l2}, L3={l3} (expected L2=L/2 and L1+2L3=L)",
    ),
    Msg::new(
        "cmd.parts.err.corner_b",
        "b={b} 不在倒角档位表内",
        "b={b} is not in the chamfer table",
    ),
    Msg::new(
        "cmd.parts.err.key1097_no_c",
        "GB/T 1097 没有 C 型",
        "GB/T 1097 has no type C",
    ),
    Msg::new(
        "cmd.parts.err.json_parse_keys",
        "平键数据表 JSON 解析失败",
        "failed to parse the parallel-key data table JSON",
    ),
    Msg::new(
        "cmd.parts.err.json_parse_parts",
        "零件数据表 JSON 解析失败",
        "failed to parse the part data table JSON",
    ),
    Msg::new(
        "cmd.parts.err.missing_param",
        "缺少参数 {name}",
        "missing parameter {name}",
    ),
    Msg::new(
        "cmd.parts.err.param_not_number",
        "{name} 不是数字",
        "{name} is not a number",
    ),
    Msg::new(
        "cmd.parts.err.thread_len",
        "M{d} 的螺纹长度 l={want}（收到 {got}）",
        "M{d} thread length l={want} (got {got})",
    ),
    Msg::new(
        "cmd.parts.err.no_d1_od",
        "GB/T 13871.1 数据表里没有 d1={d1} D={od}",
        "GB/T 13871.1 data table has no d1={d1} D={od}",
    ),
    Msg::new(
        "cmd.parts.err.no_d_bearing",
        "GB/T 276 数据表里没有 内径 d={d} 宽度 B={b}",
        "GB/T 276 data table has no bore d={d} width B={b}",
    ),
    Msg::new(
        "cmd.parts.err.no_dt",
        "{table} 数据表里没有 d={d} T={t} 的规格",
        "{table} data table has no spec with d={d} T={t}",
    ),
    Msg::new(
        "cmd.parts.err.no_db",
        "{table} 数据表里没有 d={d} B={b} 的规格",
        "{table} data table has no spec with d={d} B={b}",
    ),
    Msg::new(
        "cmd.parts.err.view_absent_no_family",
        "没有视图 {view}（可用：{avail}）",
        "no view {view} (available: {avail})",
    ),

    // ── 命令输出/报错：表面粗糙度（OCSMRGH / CC；guide_server::apply_roughness）──
    Msg::new("cmd.rough.undo_insert", "表面粗糙度", "roughness symbol"),
    Msg::new(
        "cmd.rough.prompt",
        "OCSM 表面粗糙度：指定符号插入点。",
        "OCSM surface roughness: specify the symbol insertion point.",
    ),
    Msg::new(
        "cmd.rough.err.bad_base",
        "无效的基础体 {base}（应为 C1..C4）",
        "invalid base symbol {base} (expected C1..C4)",
    ),
    Msg::new(
        "cmd.rough.err.bad_extra",
        "无效的附加区 {extra}（应为 R1..R5）",
        "invalid additional area {extra} (expected R1..R5)",
    ),

    // ── 命令输出/报错：OCSMCARD 卡类型分派（guide_server / card_report 同句）──
    Msg::new(
        "cmd.card.err.unknown_type_usage",
        "智能卡片：不认识的卡类型「{name}」。\n{usage}",
        "Smart card: unknown card type \"{name}\".\n{usage}",
    ),
    Msg::new(
        "cmd.card.err.unknown_type",
        "智能卡片：不认识的卡类型「{card_id}」（卡类型表见 /api/spline_options 的 card_types）",
        "Smart card: unknown card type \"{card_id}\" (see card_types under /api/spline_options)",
    ),
    Msg::new(
        "cmd.card.err.bad_model_spline",
        "花键参数表：请求字段无效：{e}",
        "Spline table: invalid request field: {e}",
    ),
    Msg::new(
        "cmd.card.err.bad_model_spline_lite",
        "GB 花键精简卡：请求字段无效：{e}",
        "GB spline lite card: invalid request field: {e}",
    ),
    Msg::new(
        "cmd.card.err.bad_model_gear",
        "齿轮参数表：请求字段无效：{e}",
        "Gear table: invalid request field: {e}",
    ),
    Msg::new(
        "cmd.card.err.bad_model_ansi",
        "ANSI 花键参数表：请求字段无效：{e}",
        "ANSI spline table: invalid request field: {e}",
    ),
    Msg::new(
        "cmd.card.err.bad_model_nf",
        "NF 内花键参数表：请求字段无效：{e}",
        "NF internal spline table: invalid request field: {e}",
    ),
    Msg::new(
        "cmd.card.err.bad_model_nf_ext",
        "NF 外花键参数表：请求字段无效：{e}",
        "NF external spline table: invalid request field: {e}",
    ),
    Msg::new(
        "cmd.card.err.bad_model_din",
        "DIN 花键参数表：请求字段无效：{e}",
        "DIN spline table: invalid request field: {e}",
    ),
    Msg::new(
        "cmd.card.err.no_lite_def",
        "精简卡：卡类型「{id}」没有精简定义",
        "Lite card: card type \"{id}\" has no lite definition",
    ),

    // ── 命令输出/报错：OCSMCARD 命令实现（lib.rs）──
    Msg::new(
        "cmd.card.err.no_worker",
        "OCSMCARD: 无法启动智能卡片服务（宿主不支持 worker 请求）。",
        "OCSMCARD: cannot start the smart-card service (host does not support worker requests).",
    ),
    Msg::new(
        "cmd.card.info.opened",
        "OCSM 智能卡片：已打开窗口（22 张卡，一卡一方向：GB 花键内/外、齿轮、ANSI 内/外×中/英、NF 内/外、DIN 内/外；下拉「精简版」分组 = GB/NF/DIN/ANSI×4/齿轮共 11 张只列基本参数+主要测量量的卡）。齿形表达式反解 + 实时结果。点「出表」→ 回到图纸点击定位基点 → 移动光标旋转 → 再点击落定（可连续，Esc 结束）。",
        "OCSM smart card: window opened (22 cards, one direction each: GB spline int/ext, gear, ANSI int/ext × CN/EN, NF int/ext, DIN int/ext; the \"Lite\" group = 11 cards listing only basic parameters plus main measurements for GB/NF/DIN/ANSI×4/gear). Tooth-profile expression resolution + live results. Click \"Export\" → click in the drawing to set the base point → move to rotate → click again to place (repeatable; Esc to finish).",
    ),
    Msg::new(
        "cmd.card.info.already_open",
        "OCSM 智能卡片：窗口已打开（Alt+Tab 切换过去）。",
        "OCSM smart card: window is already open (switch to it with Alt+Tab).",
    ),
    Msg::new("cmd.card.place.what", "OCSM 智能卡片", "OCSM smart card"),
    Msg::new(
        "cmd.card.place.where",
        "请在智能卡片窗口里点「出表」",
        "click \"Export\" in the smart card window",
    ),
    Msg::new(
        "cmd.card.xlt.removed",
        "`XLT` 已移除：请改用 `OCSMCARD 花键参数表 …`（智能卡片）。",
        "`XLT` was removed: use `OCSMCARD 花键参数表 …` (smart card) instead.",
    ),
    Msg::new(
        "cmd.card.err.spline_prefix",
        "花键参数表：{e}",
        "Spline table: {e}",
    ),
    Msg::new(
        "cmd.card.err.spline_block",
        "花键参数表：建块 {block} 失败：{e}",
        "Spline table: failed to create block {block}: {e}",
    ),
    Msg::new(
        "cmd.card.err.spline_no_handle",
        "花键参数表：插入失败（宿主未返回句柄）",
        "Spline table: insert failed (host returned no handle)",
    ),
    Msg::new("cmd.card.undo.spline", "花键参数表插入", "spline table insert"),
    Msg::new("cmd.card.side.internal", "内花键", "internal spline"),
    Msg::new("cmd.card.side.external", "外花键", "external spline"),
    Msg::new(
        "cmd.card.info.spline_inserted",
        "智能卡片：已插入{kind}参数表（{grade}，体系 {system}，表达式 {expr}）于 ({x}, {y}) rot {rot}°{dp_note}。",
        "Smart card: inserted {kind} parameter table ({grade}, system {system}, expression {expr}) at ({x}, {y}) rot {rot}°{dp_note}.",
    ),
    Msg::new(
        "cmd.card.dp_note.calc",
        "；量棒 Dp={dp}（计算值 D'={dp_calc}，备选 {c1}/ {c2}/ {c3}）",
        "; pin Dp={dp} (calculated D'={dp_calc}, candidates {c1}/ {c2}/ {c3})",
    ),
    Msg::new(
        "cmd.card.dp_note.manual",
        "；量棒 Dp={dp}（按所填手算，Md 已重算）",
        "; pin Dp={dp} (manual entry; Md recomputed)",
    ),
    Msg::new(
        "cmd.card.err.lite_prefix",
        "GB 花键精简卡：{e}",
        "GB spline lite card: {e}",
    ),
    Msg::new(
        "cmd.card.err.lite_block",
        "GB 花键精简卡：建块 {block} 失败：{e}",
        "GB spline lite card: failed to create block {block}: {e}",
    ),
    Msg::new(
        "cmd.card.err.lite_no_handle",
        "GB 花键精简卡：插入失败（宿主未返回句柄）",
        "GB spline lite card: insert failed (host returned no handle)",
    ),
    Msg::new(
        "cmd.card.undo.lite",
        "GB 花键精简卡插入",
        "GB spline lite card insert",
    ),
    Msg::new(
        "cmd.card.info.lite_inserted",
        "智能卡片：已插入{kind}精简卡（{grade}，表达式 {expr}）于 ({x}, {y}) rot {rot}°。",
        "Smart card: inserted {kind} lite card ({grade}, expression {expr}) at ({x}, {y}) rot {rot}°.",
    ),
    Msg::new(
        "cmd.card.err.lite_card_prefix",
        "{id}：{e}",
        "{id}: {e}",
    ),
    Msg::new(
        "cmd.card.err.lite_card_block",
        "{id}：建块 {block} 失败：{e}",
        "{id}: failed to create block {block}: {e}",
    ),
    Msg::new(
        "cmd.card.err.lite_card_no_handle",
        "{id}：插入失败（宿主未返回句柄）",
        "{id}: insert failed (host returned no handle)",
    ),
    Msg::new("cmd.card.undo.lite_card", "{label}插入", "insert {label}"),
    Msg::new(
        "cmd.card.info.lite_card_inserted",
        "智能卡片：已插入{label}（{n} 个精简项，不含公差列）于 ({x}, {y}) rot {rot}°。",
        "Smart card: inserted {label} ({n} lite items, no tolerance columns) at ({x}, {y}) rot {rot}°.",
    ),
    Msg::new(
        "cmd.card.gear.err_block",
        "齿轮参数表：建块 {block} 失败：{e}",
        "Gear table: failed to create block {block}: {e}",
    ),
    Msg::new("cmd.card.gear.err_prefix", "齿轮参数表：{e}", "Gear table: {e}"),
    Msg::new("cmd.card.gear.undo", "齿轮参数表插入", "gear table insert"),
    Msg::new(
        "cmd.card.gear.err_no_handle",
        "齿轮参数表：插入失败（宿主未返回句柄）",
        "Gear table: insert failed (host returned no handle)",
    ),
    Msg::new(
        "cmd.card.gear.info",
        "智能卡片：已插入齿轮参数表（{note}）于 ({x}, {y}) rot {rot}°。\nGB/T 10095-88 公差（Fr/FW/ff/fpt/Fβ）与中心距极限偏差本仓未收 —— 表内显示「—」。",
        "Smart card: inserted gear table ({note}) at ({x}, {y}) rot {rot}°.\nGB/T 10095-88 tolerances (Fr/FW/ff/fpt/Fβ) and center-distance limit deviations are not collected in this repo — shown as \"—\" in the table.",
    ),
    Msg::new(
        "cmd.card.ansi.err_block",
        "ANSI 花键参数表：建块 {block} 失败：{e}",
        "ANSI spline table: failed to create block {block}: {e}",
    ),
    Msg::new(
        "cmd.card.ansi.err_prefix",
        "ANSI 花键参数表：{e}",
        "ANSI spline table: {e}",
    ),
    Msg::new(
        "cmd.card.ansi.undo",
        "ANSI 花键参数表插入",
        "ANSI spline table insert",
    ),
    Msg::new(
        "cmd.card.ansi.err_no_handle",
        "ANSI 花键参数表：插入失败（宿主未返回句柄）",
        "ANSI spline table: insert failed (host returned no handle)",
    ),
    Msg::new(
        "cmd.card.ansi.info",
        "智能卡片：已插入 ANSI B92.1 {side}参数表（{lang}，P/Ps={pair}，N={n}）于 ({x}, {y}) rot {rot}°。\n配合/公差/量棒/公法线表本仓未收 —— 相关格显示「—」。",
        "Smart card: inserted ANSI B92.1 {side} table ({lang}, P/Ps={pair}, N={n}) at ({x}, {y}) rot {rot}°.\nFit/tolerance/pin/base-tangent tables are not collected in this repo — the related cells show \"—\".",
    ),
    Msg::new(
        "cmd.card.nf.err_block",
        "NF 内花键参数表：建块 {block} 失败：{e}",
        "NF internal spline table: failed to create block {block}: {e}",
    ),
    Msg::new(
        "cmd.card.nf.err_prefix",
        "NF 内花键参数表：{e}",
        "NF internal spline table: {e}",
    ),
    Msg::new("cmd.card.nf.undo", "NF 内花键参数表插入", "NF internal spline table insert"),
    Msg::new(
        "cmd.card.nf.err_no_handle",
        "NF 内花键参数表：插入失败（宿主未返回句柄）",
        "NF internal spline table: insert failed (host returned no handle)",
    ),
    Msg::new(
        "cmd.card.nf.info",
        "智能卡片：已插入{note}于 ({x}, {y}) rot {rot}°。\n公差按 p28（大径 R7 / 小径 H7，ISO 286）+ p29（跨棒距 = 内花键 E 偏差）；表外 V/G/ri 与 p29 表外偏差显示「—」。",
        "Smart card: inserted {note} at ({x}, {y}) rot {rot}°.\nTolerances follow p28 (major R7 / minor H7, ISO 286) + p29 (span = internal-spline E deviation); off-table V/G/ri and off-table p29 deviations show \"—\".",
    ),
    Msg::new(
        "cmd.card.nfext.err_block",
        "NF 外花键参数表：建块 {block} 失败：{e}",
        "NF external spline table: failed to create block {block}: {e}",
    ),
    Msg::new(
        "cmd.card.nfext.err_prefix",
        "NF 外花键参数表：{e}",
        "NF external spline table: {e}",
    ),
    Msg::new(
        "cmd.card.nfext.undo",
        "NF 外花键参数表插入",
        "NF external spline table insert",
    ),
    Msg::new(
        "cmd.card.nfext.err_no_handle",
        "NF 外花键参数表：插入失败（宿主未返回句柄）",
        "NF external spline table: insert failed (host returned no handle)",
    ),
    Msg::new(
        "cmd.card.nfext.info",
        "智能卡片：已插入{note}于 ({x}, {y}) rot {rot}°。\n公差按模板实测口径（大径 h12 / 小径 H7，ISO 286）+ p29（公法线 = 外花键 E 偏差，按配合）；K/W 与 p29 表外偏差显示「—」。",
        "Smart card: inserted {note} at ({x}, {y}) rot {rot}°.\nTolerances follow the template-measured convention (major h12 / minor H7, ISO 286) + p29 (base tangent = external-spline E deviation, per fit); off-table K/W and p29 deviations show \"—\".",
    ),
    Msg::new(
        "cmd.card.din.err_block",
        "DIN 花键参数表：建块 {block} 失败：{e}",
        "DIN spline table: failed to create block {block}: {e}",
    ),
    Msg::new(
        "cmd.card.din.err_prefix",
        "DIN 花键参数表：{e}",
        "DIN spline table: {e}",
    ),
    Msg::new("cmd.card.din.undo", "DIN 花键参数表插入", "DIN spline table insert"),
    Msg::new(
        "cmd.card.din.err_no_handle",
        "DIN 花键参数表：插入失败（宿主未返回句柄）",
        "DIN spline table: insert failed (host returned no handle)",
    ),
    Msg::new(
        "cmd.card.din.info",
        "智能卡片：已插入{note}于 ({x}, {y}) rot {rot}°（整表缩放 0.17）。\nTable 7 缺口（>400 侧偏差列、≤12 细档、非 6–9 级公差、无检验表行的 D_M/M2/M1）显示「—」。",
        "Smart card: inserted {note} at ({x}, {y}) rot {rot}° (table scaled 0.17).\nTable 7 gaps (>400 flank-deviation columns, ≤12 fine steps, non-6–9 grade tolerances, D_M/M2/M1 without inspection-table rows) show \"—\".",
    ),

    // ── 命令输出/报错：齿形表达式反解（card_expr.rs）──
    Msg::new(
        "cmd.cardexpr.err.module",
        "表达式模数 m={m} 非法，换算不出径节 P",
        "expression module m={m} is invalid; cannot derive diametral pitch P",
    ),
    Msg::new(
        "cmd.cardexpr.err.not_finite",
        "表达式反解出非有限数",
        "the expression resolves to a non-finite number",
    ),
    Msg::new(
        "cmd.cardexpr.err.mark",
        "{card}：表达式 MARK 写的是 {mark}，与卡片体系「{want}」不一致",
        "{card}: expression MARK is {mark}, inconsistent with the card system \"{want}\"",
    ),
    Msg::new(
        "cmd.cardexpr.err.kind",
        "{card}：表达式 KIND 写的是 {kind}，与卡片方向「{want}」不一致",
        "{card}: expression KIND is {kind}, inconsistent with the card direction \"{want}\"",
    ),
    Msg::new("cmd.cardexpr.kind.in", "IN（内）", "IN (internal)"),
    Msg::new("cmd.cardexpr.kind.ex", "EX（外）", "EX (external)"),
    Msg::new("cmd.cardexpr.dir.int", "内", "internal"),
    Msg::new("cmd.cardexpr.dir.ext", "外", "external"),
    Msg::new(
        "cmd.cardexpr.err.alpha",
        "{card}：表达式压力角 {alpha}° 不在卡片体系允许的 α={allowed}°（表达式与卡片体系不一致）",
        "{card}: expression pressure angle {alpha}° is outside the card system's allowed α={allowed}° (expression and card system mismatch)",
    ),
    Msg::new(
        "cmd.cardexpr.err.beta",
        "{card}：表达式螺旋角 β={beta}° 与卡片（只做直齿）不一致",
        "{card}: expression helix angle β={beta}° is inconsistent with the card (spur only)",
    ),
    Msg::new(
        "cmd.cardexpr.err.shift",
        "{card}：表达式径向变位 X={x} 在卡片里没有对应输入（该体系不收变位；请用 X0 的表达式）",
        "{card}: expression profile shift X={x} has no matching input on the card (this system does not accept shift; use an expression with X0)",
    ),

    // ── 命令输出/报错：GB 花键公差/查表（spline_tol.rs）──
    Msg::new(
        "cmd.spline.err.alpha_invalid",
        "花键参数表：压力角「{other}」非法（GB/T 3478.1 只有 30° / 37.5° / 45°）",
        "Spline table: invalid pressure angle \"{other}\" (GB/T 3478.1 only has 30° / 37.5° / 45°)",
    ),
    Msg::new(
        "cmd.spline.err.fit_invalid",
        "花键参数表：齿侧配合「{other}」非法（GB/T 3478.1 只有 H/k、H/js、H/h、H/f、H/e、H/d）",
        "Spline table: invalid flank fit \"{other}\" (GB/T 3478.1 only has H/k, H/js, H/h, H/f, H/e, H/d)",
    ),
    Msg::new(
        "cmd.spline.err.num",
        "{what}：{v} 不是数字（{e}）",
        "{what}: {v} is not a number ({e})",
    ),
    Msg::new(
        "cmd.spline.err.csv_header_missing",
        "{file} 缺表头",
        "{file} is missing its header",
    ),
    Msg::new(
        "cmd.spline.err.csv_header_bad",
        "{file} 表头异常：{header}",
        "{file}: unexpected header: {header}",
    ),
    Msg::new(
        "cmd.spline.err.csv_cols",
        "{file} 列数异常：{line}",
        "{file}: unexpected column count: {line}",
    ),
    Msg::new("cmd.spline.err.fit_row", "fit 行：{e}", "fit row: {e}"),
    Msg::new(
        "cmd.spline.err.m_not_t26",
        "花键参数表：m={m} 不在表 26（GB/T 3478.1 的 15 档模数）",
        "Spline table: m={m} is not in Table 26 (the 15 module steps of GB/T 3478.1)",
    ),
    Msg::new(
        "cmd.spline.err.grade_invalid",
        "花键参数表：公差等级 {grade} 非法（GB/T 3478.1 只有 4/5/6/7）",
        "Spline table: invalid tolerance grade {grade} (GB/T 3478.1 only has 4/5/6/7)",
    ),
    Msg::new(
        "cmd.spline.err.fit_len",
        "花键参数表：配合长度 g={g} 必须 >0",
        "Spline table: fit length g={g} must be > 0",
    ),
    Msg::new(
        "cmd.spline.err.d_not_t23",
        "花键参数表：D={d} 不在表 23 的分度圆直径档（≤6…800~1000），表外不插值",
        "Spline table: D={d} is not in a pitch-diameter band of Table 23 (≤6…800~1000); no interpolation outside the table",
    ),
    Msg::new(
        "cmd.spline.err.d_not_t24",
        "花键参数表：D={d} 不在表 24 的分度圆直径档（≤6…800~1000），表外不插值",
        "Spline table: D={d} is not in a pitch-diameter band of Table 24 (≤6…800~1000); no interpolation outside the table",
    ),
    Msg::new(
        "cmd.spline.err.m_range",
        "花键参数表：模数 m={m} 不在 0.25…10（GB/T 3478.1 的 15 种模数系列）",
        "Spline table: module m={m} is not within 0.25…10 (the 15-module series of GB/T 3478.1)",
    ),
    Msg::new(
        "cmd.spline.err.it_missing",
        "花键参数表：GB/T 1800 IT{it} 在 D={d} 无值（表外）",
        "Spline table: GB/T 1800 IT{it} has no value at D={d} (outside the table)",
    ),
    Msg::new(
        "cmd.spline.err.inv_range",
        "花键参数表：invα={y} 超出可解范围（α>85°）",
        "Spline table: invα={y} is beyond the solvable range (α>85°)",
    ),
    Msg::new(
        "cmd.spline.err.dri_base",
        "花键参数表：D'_Ri 的 Db/D_ee max/D_ii min 必须 >0",
        "Spline table: Db/D_ee max/D_ii min for D'_Ri must be > 0",
    ),
    Msg::new(
        "cmd.spline.err.dri_dci",
        "花键参数表：D_ci={d_ci} ≤ Db={db}（接触点已在基圆内，无法算 D'_Ri）",
        "Spline table: D_ci={d_ci} ≤ Db={db} (contact point is inside the base circle; cannot compute D'_Ri)",
    ),
    Msg::new(
        "cmd.spline.err.dri_le0",
        "花键参数表：D'_Ri={dp} ≤0（E_max 或几何异常）",
        "Spline table: D'_Ri={dp} ≤ 0 (E_max or geometry anomaly)",
    ),
    Msg::new(
        "cmd.spline.err.dre_dce",
        "花键参数表：D_ce={d_ce} ≤ Db={db}（无法算 D'_Re）",
        "Spline table: D_ce={d_ce} ≤ Db={db} (cannot compute D'_Re)",
    ),
    Msg::new(
        "cmd.spline.err.dre_le0",
        "花键参数表：D'_Re={dp} ≤0（S_min 或几何异常）",
        "Spline table: D'_Re={dp} ≤ 0 (S_min or geometry anomaly)",
    ),
    Msg::new(
        "cmd.spline.err.dp_over_series",
        "花键参数表：D'={dp} 超出 GB/T 3478.9 量棒系列上限 {max}",
        "Spline table: D'={dp} exceeds the upper limit {max} of the GB/T 3478.9 pin series",
    ),
    Msg::new(
        "cmd.spline.err.m_range15",
        "花键参数表：模数 m={m} 不在 0.25…10（15 种模数系列）",
        "Spline table: module m={m} is not within 0.25…10 (15-module series)",
    ),
    Msg::new(
        "cmd.spline.err.z_too_small",
        "花键参数表：齿数 z={z} 太小（至少 6）",
        "Spline table: tooth count z={z} is too small (at least 6)",
    ),
    Msg::new(
        "cmd.spline.err.internal_h",
        "花键参数表：内花键是基孔制 H（收到「{dev}」）",
        "Spline table: an internal spline is always H (hole-basis); got \"{dev}\"",
    ),
    Msg::new(
        "cmd.spline.err.internal_received_ext",
        "花键参数表：internal_table 收到外花键输入",
        "Spline table: internal_table received external-spline input",
    ),
    Msg::new(
        "cmd.spline.err.external_received_int",
        "花键参数表：external_table 收到内花键输入",
        "Spline table: external_table received internal-spline input",
    ),
    Msg::new(
        "cmd.spline.err.dp_positive",
        "花键参数表：量棒直径 Dp={v} 必须 >0",
        "Spline table: pin diameter Dp={v} must be > 0",
    ),
    Msg::new(
        "cmd.spline.err.dp_not_series",
        "花键参数表：Dp={v} 不在 GB/T 3478.9 表 1 量棒系列（0.56…25.00，67 档）",
        "Spline table: Dp={v} is not in the GB/T 3478.9 Table 1 pin series (0.56…25.00, 67 steps)",
    ),

    // ── 卡类型显示名（报错前缀；仅用于错误/回执前缀，不动卡面/GUI 标签）──
    Msg::new("card.name.spline", "花键参数表", "Spline table"),
    Msg::new("card.name.gear", "齿轮参数表", "Gear table"),
    Msg::new("card.name.ansi", "ANSI 花键参数表", "ANSI spline table"),
    Msg::new("card.name.nf", "NF 内花键参数表", "NF internal spline table"),
    Msg::new("card.name.nf_ext", "NF 外花键参数表", "NF external spline table"),
    Msg::new("card.name.din", "DIN 花键参数表", "DIN spline table"),

    // ── GUI 元数据：卡类型下拉的显示名/分组（`card.rs::CardTypeSpec`；GUI 卡类型列表）──
    // ★ 前批 ⑥ 的未接线项（§32.6 ②）：只做 **label/group**（summary 仍中文，见 §34 报告）；
    //   `id`/`renderer`/`direction` 是协议记号，**不译**。
    Msg::new("gui.card.group.lite", "精简版", "Lite"),
    Msg::new("gui.card.report.title", "{card}计算书", "{card} report"),
    Msg::new("gui.card.gb_int.label", "GB 花键参数表（内）", "GB spline parameter table (internal)"),
    Msg::new("gui.card.gb_ext.label", "GB 花键参数表（外）", "GB spline parameter table (external)"),
    Msg::new("gui.card.gear.label", "齿轮参数表（GB/T 10095）", "Gear parameter table (GB/T 10095)"),
    Msg::new("gui.card.ansi_cn_int.label", "ANSI 花键参数表（内·纯中文）", "ANSI spline parameter table (internal · Chinese)"),
    Msg::new("gui.card.ansi_cn_ext.label", "ANSI 花键参数表（外·纯中文）", "ANSI spline parameter table (external · Chinese)"),
    Msg::new("gui.card.ansi_en_int.label", "ANSI 花键参数表（内·纯英文）", "ANSI spline parameter table (internal · English)"),
    Msg::new("gui.card.ansi_en_ext.label", "ANSI 花键参数表（外·纯英文）", "ANSI spline parameter table (external · English)"),
    Msg::new("gui.card.nf_int.label", "NF E22-141 内花键参数表", "NF E22-141 internal spline parameter table"),
    Msg::new("gui.card.nf_ext.label", "NF E22-141 外花键参数表", "NF E22-141 external spline parameter table"),
    Msg::new("gui.card.din_int.label", "DIN 5480 内花键参数表", "DIN 5480 internal spline parameter table"),
    Msg::new("gui.card.din_ext.label", "DIN 5480 外花键参数表", "DIN 5480 external spline parameter table"),
    Msg::new("gui.card.gb_int_lite.label", "GB 花键参数表（内·精简版）", "GB spline parameter table (internal · lite)"),
    Msg::new("gui.card.gb_ext_lite.label", "GB 花键参数表（外·精简版）", "GB spline parameter table (external · lite)"),
    Msg::new("gui.card.nf_int_lite.label", "NF E22-141 内花键参数表（精简版）", "NF E22-141 internal spline parameter table (lite)"),
    Msg::new("gui.card.nf_ext_lite.label", "NF E22-141 外花键参数表（精简版）", "NF E22-141 external spline parameter table (lite)"),
    Msg::new("gui.card.din_int_lite.label", "DIN 5480 内花键参数表（精简版）", "DIN 5480 internal spline parameter table (lite)"),
    Msg::new("gui.card.din_ext_lite.label", "DIN 5480 外花键参数表（精简版）", "DIN 5480 external spline parameter table (lite)"),
    Msg::new("gui.card.ansi_cn_int_lite.label", "ANSI 花键参数表（内·纯中文·精简版）", "ANSI spline parameter table (internal · Chinese · lite)"),
    Msg::new("gui.card.ansi_cn_ext_lite.label", "ANSI 花键参数表（外·纯中文·精简版）", "ANSI spline parameter table (external · Chinese · lite)"),
    Msg::new("gui.card.ansi_en_int_lite.label", "ANSI 花键参数表（内·纯英文·精简版）", "ANSI spline parameter table (internal · English · lite)"),
    Msg::new("gui.card.ansi_en_ext_lite.label", "ANSI 花键参数表（外·纯英文·精简版）", "ANSI spline parameter table (external · English · lite)"),
    Msg::new("gui.card.gear_lite.label", "齿轮参数表（GB/T 10095·精简版）", "Gear parameter table (GB/T 10095 · lite)"),

    // ── 卡面标签：GB 花键参数表（`spline_table.rs`，③ 卡面批 ① 族）──
    // 用户 2026-09-27：「**卡面文字也要英译**」；**标准符号/代号/标准号/值原样**（不列在表里）。
    // * zh 侧含 GB 模板自带的对齐空格（`模  数`/`小  径`/`齿 形 角`）＝模板忠实，不动版面；
    //   精简卡（`spline_lite.rs`）用**无空格**的另一组 key（`card.gb.lite.label.*`），英文同译法。
    // * 英文天生比中文长 ⇒ 卡面文字按**实体级 width_factor** 压实（真字体 metrics：
    //   朱雀仿宋 cap=0.637em ⇒ 宽 = 字号 × 1.57 × wf，见 `spline_table::en_label_width_factor`）。
    Msg::new("card.gb.title.int", "内 花 键 参 数 表", "Internal Spline Parameter Table"),
    Msg::new("card.gb.title.ext", "外 花 键 参 数 表", "External Spline Parameter Table"),
    Msg::new("card.gb.title.int_lite", "内 花 键 参 数 表（精简）", "Internal Spline Parameter Table (Lite)"),
    Msg::new("card.gb.title.ext_lite", "外 花 键 参 数 表（精简）", "External Spline Parameter Table (Lite)"),
    Msg::new("card.gb.label.total_composite", "综合公差", "Total composite tolerance"),
    Msg::new("card.gb.label.pitch_cumulative", "齿距累计公差", "Cumulative pitch tolerance"),
    Msg::new("card.gb.label.profile", "齿形公差", "Profile tolerance"),
    Msg::new("card.gb.label.space_width_min", "作用齿槽宽最小值", "Min. effective space width"),
    Msg::new("card.gb.label.space_width_max", "实际齿槽宽最大值", "Max. actual space width"),
    Msg::new("card.gb.label.thickness_min", "实际齿厚最小值", "Min. actual tooth thickness"),
    Msg::new("card.gb.label.thickness_max", "作用齿厚最大值", "Max. effective tooth thickness"),
    Msg::new("card.gb.label.minor_dia", "小  径", "Minor diameter"),
    Msg::new("card.gb.label.major_dia", "大  径", "Major diameter"),
    Msg::new("card.gb.label.form_dia_max", "渐开线终止圆直径最大值", "Max. form diameter"),
    Msg::new("card.gb.label.grade_fit", "公差等级和配合类别", "Tolerance grade and fit"),
    Msg::new("card.gb.label.module", "模  数", "Module"),
    Msg::new("card.gb.label.alpha", "齿 形 角", "Pressure angle"),
    Msg::new("card.gb.label.teeth", "齿  数", "Number of teeth"),
    Msg::new("card.gb.label.root_fillet_radius", "齿根圆最小曲率半径", "Min. root fillet radius"),
    Msg::new("card.gb.label.pin_dia", "量棒直径", "Pin diameter"),
    Msg::new("card.gb.label.over_pins", "测量跨棒距", "Measurement over pins"),
    Msg::new("card.gb.label.span_teeth", "跨测齿数", "Span teeth"),
    Msg::new("card.gb.label.base_tangent", "公法线长度", "Base tangent length"),
    // 精简卡专属（无模板对齐空格的 zh 形态 + GB 精简卡才有的两行）。
    Msg::new("card.gb.lite.label.standard", "执行标准", "Standard"),
    Msg::new("card.gb.lite.label.root_form", "齿根样式", "Root form"),
    Msg::new("card.gb.lite.label.module", "模数", "Module"),
    Msg::new("card.gb.lite.label.teeth", "齿数", "Number of teeth"),
    Msg::new("card.gb.lite.label.alpha", "齿形角", "Pressure angle"),
    Msg::new("card.gb.lite.label.major_dia", "大径", "Major diameter"),
    Msg::new("card.gb.lite.label.minor_dia", "小径", "Minor diameter"),
    // 齿根样式**取值**（写进 ATTRIB、卡面可见；`(简)齿根样式` 行）：术语随语言，
    // 命令关键字（`平`/`圆`/`flat`/`fillet`）仍不译（§六）。
    Msg::new("card.gb.root.flat", "平齿根", "Flat root"),
    Msg::new("card.gb.root.fillet", "圆齿根", "Fillet root"),

    // ── 卡面标签：齿轮参数表（`gear_table.rs`，③ 卡面批 ② 族）──
    // 口径同 `card.gb.*`：**符号/代号/标准号/值/块名/ATTDEF tag 不进表**（原样）。
    // * `卡面 TEXT 表首字段 = key`（`card.` 前缀取词，其余原样＝符号/代号）；
    // * 英文按真字体 metrics 逐条给**实体级 width_factor**（`gear_table::en_label_width_factor`，
    //   预算脚本 `i18n_检查/card_gear_ansi_width_plan.py`）；
    // * **阶梯：符号与译名之间一个空格**（`mn`/`z`/`α` 等仍在模板原位，不动坐标）。
    Msg::new("card.gear.label.normal_module", "法向模数", "Normal module"),
    Msg::new("card.gear.label.teeth", "齿数", "Number of teeth"),
    Msg::new("card.gear.label.alpha", "齿形角", "Pressure angle"),
    Msg::new("card.gear.label.ha_coef", "齿顶高系数", "Addendum coefficient"),
    Msg::new("card.gear.label.beta", "螺旋角", "Helix angle"),
    Msg::new("card.gear.label.hand", "螺旋方向", "Hand of helix"),
    Msg::new("card.gear.label.shift", "径向变位系数", "Profile shift coefficient"),
    Msg::new("card.gear.label.whole_depth", "全齿高", "Whole depth"),
    Msg::new("card.gear.label.grade", "精度等级", "Accuracy grade"),
    Msg::new("card.gear.label.center", "齿轮副中心距及其极限偏差", "Centre distance and limits"),
    Msg::new("card.gear.label.mate", "配对齿轮", "Mating gear"),
    Msg::new("card.gear.label.tol_group", "公差组", "Tolerance group"),
    Msg::new("card.gear.label.fw_var", "公法线长度变动公差", "Base tangent length variation"),
    Msg::new("card.gear.label.profile_tol", "齿形公差", "Profile tolerance"),
    Msg::new("card.gear.label.pitch_dev", "齿距极限偏差", "Pitch limit deviation"),
    Msg::new("card.gear.label.helix_tol", "齿向公差", "Helix tolerance"),
    Msg::new("card.gear.label.base_tangent", "公法线", "Base tangent length"),
    Msg::new("card.gear.label.runout", "齿圈径向跳动公差", "Radial run-out tolerance"),
    Msg::new("card.gear.label.dwg_no", "图号", "DWG No."),
    Msg::new("card.gear.label.mate_teeth", "齿数", "Teeth"),
    Msg::new("card.gear.label.inspect_code", "检验项目代号", "Inspection code"),
    Msg::new("card.gear.label.tol_value", "公差(或极限偏差)值", "Tol. (or limit)"),
    // 取值（枚举显示值随语言；数值/标准号仍是数据，不进表）。
    Msg::new("card.gear.dir.straight", "直齿", "Spur"),
    Msg::new("card.gear.dir.right", "右旋", "Right hand"),
    Msg::new("card.gear.dir.left", "左旋", "Left hand"),

    // ── 卡面标签：齿轮参数表（精简）（`card_lite.rs` 齿轮族；形态同 `card.gb.lite.*`）──
    Msg::new("card.gear.lite.title", "齿轮参数表（精简）", "Gear Parameter Table (Lite)"),
    Msg::new("card.gear.lite.label.module", "模数", "Module"),
    Msg::new("card.gear.lite.label.teeth", "齿数", "Number of teeth"),
    Msg::new("card.gear.lite.label.alpha", "压力角", "Pressure angle"),
    Msg::new("card.gear.lite.label.shift", "变位系数", "Shift coefficient"),
    Msg::new("card.gear.lite.label.pitch_dia", "分度圆", "Pitch diameter"),
    Msg::new("card.gear.lite.label.tip_dia", "齿顶圆", "Tip diameter"),
    Msg::new("card.gear.lite.label.root_dia", "齿根圆", "Root diameter"),
    Msg::new("card.gear.lite.label.base_tangent", "公法线", "Base tangent length"),
    Msg::new("card.gear.lite.label.span_teeth", "跨齿数", "Span teeth"),

    // ── 卡面标签：ANSI B92.1 花键参数表（`ansi_table.rs`，③ 卡面批 ② 族）──
    // ★ 口径（用户 2026-09-27）：**语言开关与「块语种」是两件事** —— 本卡的纯中文/纯英文是
    //   **两个模板块**（`OCSM_ANSI_INT_CN` / `_INT_EN` …）；块语种决定取本表 zh 还是 en，
    //   **不随 `OCSMLANG` 变**（`ocs_gear_ansi_lang_check.py` 双向断言）。
    // * 「跨棒距」/「公法线长度」在英文块里是**换行成两行**的另一条 MTEXT（`LabelLang::En` 行），
    //   中英共用同一个 key（zh=中文单行名、en=英文单行名），几何各按模板原值。
    Msg::new("card.ansi.label.spline_type", "花键类型", "SPLINE TYPE"),
    Msg::new("card.ansi.label.teeth", "齿数", "NUMBER OF TEETH"),
    Msg::new("card.ansi.label.pitch", "径节", "SPLINE PITCH"),
    Msg::new("card.ansi.label.pressure_angle", "压力角", "PRESSURE ANGLE"),
    Msg::new("card.ansi.label.base_dia", "基圆直径", "BASE DIAMETER"),
    Msg::new("card.ansi.label.pitch_dia", "节圆直径", "PITCH DIAMETER"),
    Msg::new("card.ansi.label.major_dia", "大径", "MAJOR DIAMETER"),
    Msg::new("card.ansi.label.minor_dia", "小径", "MINOR DIAMETER"),
    Msg::new("card.ansi.label.form_dia", "有效直径", "FORM DIAMETER"),
    Msg::new("card.ansi.label.form_dia_ext", "渐开线终止圆直径", "FORM DIAMETER"),
    Msg::new("card.ansi.label.ref", "参考", "REF"),
    Msg::new("card.ansi.label.min", "最小", "MIN"),
    Msg::new("card.ansi.label.circular_thickness", "圆周齿厚", "CIRCULAR TOOTH THICKNESS"),
    Msg::new("card.ansi.label.max_actual", "实际齿厚最大值", "MAX ACTUAL"),
    Msg::new("card.ansi.label.min_actual", "实际齿厚最小值", "MIN ACTUAL"),
    Msg::new("card.ansi.label.min_effective", "作用齿厚最小值", "MIN EFFECTIVE"),
    Msg::new("card.ansi.label.max_effective", "作用齿厚最大值", "MAX EFFECTIVE"),
    Msg::new("card.ansi.label.over_pins", "跨棒距", "MEASUREMENT BETWEEN PINS"),
    Msg::new("card.ansi.label.pin_dia", "量棒直径", "PIN DIAMETER"),
    Msg::new("card.ansi.label.base_tangent", "公法线长度", "COMMON NORMAL LINE"),
    Msg::new("card.ansi.label.span_teeth", "跨测齿数", "NUMBER OF TESTING TEETH"),
    Msg::new("card.ansi.label.standard", "标准：ANSI B92.1-1996", "STANDARD: ANSI B92.1-1996"),
    Msg::new("card.ansi.label.title_internal", "内花键参数表", "INTERNAL INVOLUTE SPLINE DATA"),
    Msg::new("card.ansi.label.title_external", "外花键参数表", "EXTERNAL INVOLUTE SPLINE DATA"),

    // ── 卡面标签：ANSI 花键参数表（精简 4 卡；`card_lite.rs` ANSI 族）──
    // 精简 4 卡 = 「一卡一语种」（卡 id/块名已含 CN/EN）⇒ 译文由**族**钉死，与 `OCSMLANG` 无关。
    Msg::new("card.ansi.lite.title.int", "ANSI 内花键参数表（精简）", "ANSI INTERNAL SPLINE (LITE)"),
    Msg::new("card.ansi.lite.title.ext", "ANSI 外花键参数表（精简）", "ANSI EXTERNAL SPLINE (LITE)"),
    Msg::new("card.ansi.lite.label.spline_type", "花键类型", "SPLINE TYPE"),
    Msg::new("card.ansi.lite.label.teeth", "齿数 z", "TEETH z"),
    Msg::new("card.ansi.lite.label.pitch", "径节 P", "PITCH P"),
    Msg::new("card.ansi.lite.label.alpha", "压力角 α", "ALPHA"),
    Msg::new("card.ansi.lite.label.base_dia", "基圆直径 Db", "BASE DIA. Db"),
    Msg::new("card.ansi.lite.label.pitch_dia", "节圆直径 D", "PITCH DIA. D"),
    Msg::new("card.ansi.lite.label.major_dia", "大径", "MAJOR DIA."),
    Msg::new("card.ansi.lite.label.minor_dia", "小径", "MINOR DIA."),
    Msg::new("card.ansi.lite.label.over_pins", "跨棒距 M", "PIN DIST. M"),
    Msg::new("card.ansi.lite.label.pin_dia", "量棒直径 Dp", "PIN DIA. Dp"),
    Msg::new("card.ansi.lite.label.base_tangent", "公法线 W", "BASE TANG. W"),
    Msg::new("card.ansi.lite.label.span_teeth", "跨测齿数 K", "SPAN TEETH K"),

    // ── GUI 元数据：ANSI 卡语种名 + 17 项列口径（`ansi_table.rs`；GUI 表头/口径 tooltip）──
    // ★ 这些是 **GUI 元数据**（不是卡面文字）：列名 = `gui.ansi.col.*`，取值口径/来源分别为
    //   `gui.ansi.formula.*` / `gui.ansi.source.*`（前批 ⑥ 的未接线项）。符号与译名分置。
    Msg::new("card.ansi.lang.cn", "纯中文", "Chinese only"),
    Msg::new("card.ansi.lang.en", "纯英文", "English only"),
    Msg::new(
        "card.ansi.missing_note",
        "ANSI B92.1 的配合/公差（Table 4/5）、量棒检验（p30–p32）、公法线跨测表本仓未收 —— 这些格子显示「—」，不臆造。",
        "ANSI B92.1 fits/tolerances (Table 4/5), pin inspection (p30–p32) and the base tangent/span tables are not collected in this repo — those cells show “—”; no guesswork.",
    ),
    Msg::new(
        "card.ansi.note",
        "内/外共用同一套 Table 2 公式（invol_spline.rs 的 ANSI 分支）；模板按语种拆成纯中文/纯英文两版，位置/字高/层照原双语模板。",
        "Internal and external sides share the same Table 2 formulas (the ANSI branch of invol_spline.rs); the template is split into a Chinese-only and an English-only version, with positions/heights/layers following the original bilingual template.",
    ),
    Msg::new("gui.ansi.col.spline_type", "花键类型", "Spline type"),
    Msg::new("gui.ansi.col.teeth", "齿数", "Number of teeth"),
    Msg::new("gui.ansi.col.pitch", "径节", "Diametral pitch"),
    Msg::new("gui.ansi.col.pressure_angle", "压力角", "Pressure angle"),
    Msg::new("gui.ansi.col.base_dia", "基圆直径", "Base diameter"),
    Msg::new("gui.ansi.col.pitch_dia", "节圆直径", "Pitch diameter"),
    Msg::new("gui.ansi.col.major_dia_up", "大径上差", "Major dia. upper dev."),
    Msg::new("gui.ansi.col.major_dia", "大径", "Major diameter"),
    Msg::new("gui.ansi.col.major_dia_down", "大径下差", "Major dia. lower dev."),
    Msg::new("gui.ansi.col.form_dia", "有效直径", "Form diameter"),
    Msg::new("gui.ansi.col.form_dia_ext", "渐开线终止圆直径", "Form diameter"),
    Msg::new("gui.ansi.col.minor_dia", "小径", "Minor diameter"),
    Msg::new("gui.ansi.col.thick_max_actual", "实际齿厚最大值", "Max. actual tooth thickness"),
    Msg::new("gui.ansi.col.thick_min_effective", "作用齿厚最小值", "Min. effective tooth thickness"),
    Msg::new("gui.ansi.col.over_pins_up", "跨棒距上差", "Over-pins upper dev."),
    Msg::new("gui.ansi.col.over_pins", "跨棒距", "Measurement over pins"),
    Msg::new("gui.ansi.col.over_pins_down", "跨棒距下差", "Over-pins lower dev."),
    Msg::new("gui.ansi.col.pin_dia", "量棒直径", "Pin diameter"),
    Msg::new("gui.ansi.col.thick_max_effective", "作用齿厚最大值", "Max. effective tooth thickness"),
    Msg::new("gui.ansi.col.thick_min_actual", "实际齿厚最小值", "Min. actual tooth thickness"),
    Msg::new("gui.ansi.col.base_tangent_up", "公法线上差", "Base tangent upper dev."),
    Msg::new("gui.ansi.col.base_tangent", "公法线长度", "Base tangent length"),
    Msg::new("gui.ansi.col.base_tangent_down", "公法线下差", "Base tangent lower dev."),
    Msg::new("gui.ansi.col.span_teeth", "跨测齿数", "Span teeth"),
    Msg::new(
        "gui.ansi.formula.profile_column",
        "Table 2 列（30°平/圆齿根 × 齿侧/外径配合；由 profile 预设决定）",
        "Table 2 column (30° flat/fillet root × flank/major-dia. fit; set by the profile preset)",
    ),
    Msg::new("gui.ansi.formula.teeth_input", "输入齿数 N", "Number of teeth N (input)"),
    Msg::new(
        "gui.ansi.formula.pitch_pair",
        "Table 3 的 P/Ps 成对写法（Ps=2P）",
        "Paired P/Ps notation from Table 3 (Ps = 2P)",
    ),
    Msg::new("gui.ansi.formula.alpha_columns", "Table 2 列：30° / 37.5° / 45°", "Table 2 column: 30° / 37.5° / 45°"),
    Msg::new(
        "gui.ansi.formula.base_dia",
        "Db = D·cosφD，D = N/P（英寸→mm 已换算）",
        "Db = D·cos φD, D = N/P (inches → mm already converted)",
    ),
    Msg::new(
        "gui.ansi.formula.pitch_dia",
        "D = N/P（标准英寸值 × 25.4 = mm）",
        "D = N/P (standard inch value × 25.4 = mm)",
    ),
    Msg::new(
        "gui.ansi.formula.fit_missing",
        "ANSI B92.1 配合/公差表 —— 本仓未收",
        "ANSI B92.1 fit/tolerance tables — not collected in this repo",
    ),
    Msg::new(
        "gui.ansi.formula.major_dia_int",
        "Dri = (N+1.35)/P（30°平齿侧 A；B/C/D/E 列各自系数）",
        "Dri = (N+1.35)/P (30° flat root, flank fit A; columns B/C/D/E use their own coefficients)",
    ),
    Msg::new(
        "gui.ansi.formula.form_dia_int",
        "DFi = (N+1)/P + 2cF（B 列含 −0.004 in 修正）",
        "DFi = (N+1)/P + 2cF (column B includes a −0.004 in correction)",
    ),
    Msg::new(
        "gui.ansi.formula.minor_dia_int",
        "Di = (N−1)/P（30°；37.5° −0.8/P、45° −0.6/P）",
        "Di = (N−1)/P (30°; 37.5° −0.8/P, 45° −0.6/P)",
    ),
    Msg::new(
        "gui.ansi.formula.thickness_missing",
        "ANSI B92.1 齿厚/公差表 —— 本仓未收",
        "ANSI B92.1 tooth-thickness/tolerance tables — not collected in this repo",
    ),
    Msg::new(
        "gui.ansi.formula.pin_inspection_missing",
        "ANSI B92.1 量棒检验表（p30–p32）—— 本仓未收",
        "ANSI B92.1 pin inspection tables (p30–p32) — not collected in this repo",
    ),
    Msg::new(
        "gui.ansi.formula.over_pins_missing",
        "ANSI B92.1 量棒直径 + 量棒跨距表 —— 本仓未收",
        "ANSI B92.1 pin-diameter and over-pins tables — not collected in this repo",
    ),
    Msg::new(
        "gui.ansi.formula.pin_dia_missing",
        "ANSI B92.1 量棒直径表 —— 本仓未收",
        "ANSI B92.1 pin-diameter table — not collected in this repo",
    ),
    Msg::new(
        "gui.ansi.formula.major_dia_ext",
        "Do = (N+1)/P（Table 2 全列同式）",
        "Do = (N+1)/P (same formula for all Table 2 columns)",
    ),
    Msg::new(
        "gui.ansi.formula.form_dia_ext",
        "DFe = (N−1)/P − 2cF（37.5° −0.8/P、45° −0.6/P）",
        "DFe = (N−1)/P − 2cF (37.5° −0.8/P, 45° −0.6/P)",
    ),
    Msg::new(
        "gui.ansi.formula.minor_dia_ext",
        "Dre = (N−1.35/P)（30°；圆齿根 16/32 及更细 (N−2)/P；A/B 列 (N−1.35)/P）",
        "Dre = (N−1.35)/P (30°; fillet root 16/32 and finer (N−2)/P; columns A/B (N−1.35)/P)",
    ),
    Msg::new(
        "gui.ansi.formula.base_tangent_missing",
        "ANSI B92.1 公法线/跨测表 —— 本仓未收",
        "ANSI B92.1 base-tangent/span tables — not collected in this repo",
    ),
    Msg::new(
        "gui.ansi.source.table2",
        "ANSI B92.1-1970 (R1993) Table 2（p10，本仓 invol_spline.rs 已入库）",
        "ANSI B92.1-1970 (R1993) Table 2 (p10; already collected in this repo's invol_spline.rs)",
    ),
    Msg::new(
        "gui.ansi.source.missing",
        "缺：本仓未收 ANSI B92.1 配合/公差（Table 4/5）/量棒检验/公法线表 —— 不臆造",
        "Missing: this repo has not collected the ANSI B92.1 fit/tolerance (Table 4/5), pin-inspection and base-tangent tables — no guesswork",
    ),
    Msg::new("gui.ansi.source.user", "用户填写", "Entered by the user"),
    Msg::new("gui.ansi.source.table3", "ANSI B92.1 Table 3（p11）", "ANSI B92.1 Table 3 (p11)"),

    // ── GUI 元数据批：卡类型摘要 + 齿轮/NF/DIN/GB 精简列口径 + 表单提示（随语言；本批）──
    Msg::new("gui.card.gb_int.summary", "GB/T 3478 内花键：九字段齿形表达式 → 21 项参数表（一卡一方向；外键见表下方「（外）」卡）", "GB/T 3478 internal spline: nine-field tooth-profile expression → 21-item parameter table (one card per direction; for external splines see the “(external)” card below)"),
    Msg::new("gui.card.gb_ext.summary", "GB/T 3478 外花键：九字段齿形表达式 → 21 项参数表（一卡一方向，不再面板选内外）", "GB/T 3478 external spline: nine-field tooth-profile expression → 21-item parameter table (one card per direction; the panel no longer selects internal/external)"),
    Msg::new("gui.card.gear.summary", "齿轮（内/外）：九字段齿形表达式反解 ha*/c* + 公法线跨距 → 19 项参数表（GB/T 10095 公差未收，如实标缺）", "Gear (internal/external): nine-field tooth-profile expression back-solves ha*/c* + base tangent span → 19-item parameter table (GB/T 10095 tolerances not collected; shown as missing)"),
    Msg::new("gui.card.ansi_cn_int.summary", "ANSI B92.1 内花键 + P/z → 17 项参数表（纯中文；公差/量棒/公法线未收，如实标缺）", "ANSI B92.1 internal spline + P/z → 17-item parameter table (Chinese only; fits/pins/base tangent not collected, shown as missing)"),
    Msg::new("gui.card.ansi_cn_ext.summary", "ANSI B92.1 外花键 + P/z → 17 项参数表（纯中文；公差/量棒/公法线未收，如实标缺）", "ANSI B92.1 external spline + P/z → 17-item parameter table (Chinese only; fits/pins/base tangent not collected, shown as missing)"),
    Msg::new("gui.card.ansi_en_int.summary", "ANSI B92.1 内花键 + P/z → 17 项参数表（纯英文；与中文版同构，仅文本语种替换）", "ANSI B92.1 internal spline + P/z → 17-item parameter table (English only; same layout as the Chinese version, text language swapped only)"),
    Msg::new("gui.card.ansi_en_ext.summary", "ANSI B92.1 外花键 + P/z → 17 项参数表（纯英文；与中文版同构，仅文本语种替换）", "ANSI B92.1 external spline + P/z → 17-item parameter table (English only; same layout as the Chinese version, text language swapped only)"),
    Msg::new("gui.card.nf_int.summary", "NF E22-141 内花键（拉削，外径定心）：A/m/z 查 p18 表 → 13 行镜像表（Az=A 或 A+0.3m、D=A−2m、V/G 取 p23–p25、ri 取 p22；公差 = p28 R7/H7 + p29 E 偏差，四配合）", "NF E22-141 internal spline (broached, major-dia. centering): A/m/z looked up in the p18 table → 13-row mirror table (Az=A or A+0.3m, D=A−2m, V/G from p23–p25, ri from p22; tolerances = p28 R7/H7 + p29 E deviations, four fits)"),
    Msg::new("gui.card.nf_ext.summary", "NF E22-141 外花键（滚齿，齿面定心）：A/m/z 查 p20–p22 表 → 13 行模板原版表（Dee=A−0.2m、Die=A−2.4m/2.694m、K/W 取 p23–p25；公差 = ISO h12/H7 + p29 外花键 E 偏差，四配合）", "NF E22-141 external spline (hobbed, flank centering): A/m/z looked up in the p20–p22 tables → 13-row table as in the template (Dee=A−0.2m, Die=A−2.4m/2.694m, K/W from p23–p25; tolerances = ISO h12/H7 + p29 external-spline E deviations, four fits)"),
    Msg::new("gui.card.din_int.summary", "DIN 5480-1 Bild 6 内花键（Nabe）单栏 13 行（z/m/α/三直径/e 三极限/D_M/两 M2 极限）；Table 7 上段偏差有实锚，公差表只到 6–9 级锚点，缺口如实标「—」", "DIN 5480-1 Bild 6 internal spline (Nabe) single column, 13 rows (z/m/α/three diameters/three e limits/D_M/two M2 limits); the upper Table 7 deviations have real anchors, the tolerance table only reaches grades 6–9, gaps shown as “—”"),
    Msg::new("gui.card.din_ext.summary", "DIN 5480-1 Bild 6 外花键（Welle）单栏 13 行（z/m/α/三直径/s 三极限/D_M/两 M1 极限）；与内卡同源，缺口同样如实标「—」", "DIN 5480-1 Bild 6 external spline (Welle) single column, 13 rows (z/m/α/three diameters/three s limits/D_M/two M1 limits); same source as the internal card, gaps likewise shown as “—”"),
    Msg::new("gui.card.gb_int_lite.summary", "GB/T 3478 内花键精简卡：只列基本参数（标准/模数/齿数/压力角/齿根样式/大径/小径）+ 量棒 Dp / 跨棒距 Md，不含任何公差列", "GB/T 3478 internal spline lite card: basic parameters only (standard/module/teeth/pressure angle/root form/major dia./minor dia.) + pin Dp / measurement over pins Md, no tolerance columns"),
    Msg::new("gui.card.gb_ext_lite.summary", "GB/T 3478 外花键精简卡：只列基本参数（标准/模数/齿数/压力角/齿根样式/大径/小径）+ 跨测齿数 Kn / 公法线 Wn，不含任何公差列", "GB/T 3478 external spline lite card: basic parameters only (standard/module/teeth/pressure angle/root form/major dia./minor dia.) + span teeth Kn / base tangent Wn, no tolerance columns"),
    Msg::new("gui.card.nf_int_lite.summary", "NF 内花键精简卡：执行标准/定心方式/模数/齿数/压力角/齿根样式/加工方法/大径/小径 + 量棒 V / 跨棒距 G，不含公差列", "NF internal spline lite card: standard/centering/module/teeth/pressure angle/root form/machining/major dia./minor dia. + pin V / measurement over pins G, no tolerance columns"),
    Msg::new("gui.card.nf_ext_lite.summary", "NF 外花键精简卡：执行标准/定心方式/模数/齿数/压力角/齿根样式/加工方法/大径/小径 + 跨测齿数 K / 公法线 W，不含公差列", "NF external spline lite card: standard/centering/module/teeth/pressure angle/root form/machining/major dia./minor dia. + span teeth K / base tangent W, no tolerance columns"),
    Msg::new("gui.card.din_int_lite.summary", "DIN 5480 内花键精简卡：标记/齿数/模数/压力角 + 齿根圆/齿根成形圆/齿顶圆/量圆 D_M/量圆距 M2 极限（按 DIN 自身口径，不含公差列）", "DIN 5480 internal spline lite card: designation/teeth/module/pressure angle + root dia./root form dia./tip dia./measuring circle D_M/measurement over pins M2 limits (per DIN's own definitions, no tolerance columns)"),
    Msg::new("gui.card.din_ext_lite.summary", "DIN 5480 外花键精简卡：标记/齿数/模数/压力角 + 齿顶圆/齿根成形圆/齿根圆/量圆 D_M/量圆距 M1 极限（按 DIN 自身口径，不含公差列）", "DIN 5480 external spline lite card: designation/teeth/module/pressure angle + tip dia./root form dia./root dia./measuring circle D_M/measurement over pins M1 limits (per DIN's own definitions, no tolerance columns)"),
    Msg::new("gui.card.ansi_cn_int_lite.summary", "ANSI B92.1 内花键精简卡（纯中文）：类型/齿数/径节/压力角/基圆/节圆/大径/小径 + 跨棒距 M / 量棒 Dp，不含公差列", "ANSI B92.1 internal spline lite card (Chinese only): type/teeth/pitch/pressure angle/base dia./pitch dia./major dia./minor dia. + measurement over pins M / pin Dp, no tolerance columns"),
    Msg::new("gui.card.ansi_cn_ext_lite.summary", "ANSI B92.1 外花键精简卡（纯中文）：类型/齿数/径节/压力角/基圆/节圆/大径/小径 + 公法线 W / 跨测齿数 K，不含公差列", "ANSI B92.1 external spline lite card (Chinese only): type/teeth/pitch/pressure angle/base dia./pitch dia./major dia./minor dia. + base tangent W / span teeth K, no tolerance columns"),
    Msg::new("gui.card.ansi_en_int_lite.summary", "ANSI B92.1 内花键精简卡（纯英文）：SPLINE TYPE/TEETH/PITCH/ALPHA/base·pitch dia + PIN DIST. / PIN DIA.，不含公差列", "ANSI B92.1 internal spline lite card (English only): SPLINE TYPE/TEETH/PITCH/ALPHA/base·pitch dia + PIN DIST. / PIN DIA., no tolerance columns"),
    Msg::new("gui.card.ansi_en_ext_lite.summary", "ANSI B92.1 外花键精简卡（纯英文）：SPLINE TYPE/TEETH/PITCH/ALPHA/base·pitch dia + BASE TANG. / SPAN TEETH，不含公差列", "ANSI B92.1 external spline lite card (English only): SPLINE TYPE/TEETH/PITCH/ALPHA/base·pitch dia + BASE TANG. / SPAN TEETH, no tolerance columns"),
    Msg::new("gui.card.gear_lite.summary", "齿轮精简卡：模数/齿数/压力角/变位 + 分度圆/齿顶圆/齿根圆 + 公法线 W / 跨齿数 K（公差与未收项一律不上卡）", "Gear lite card: module/teeth/pressure angle/profile shift + pitch dia./tip dia./root dia. + base tangent W / span teeth K (tolerances and uncollected items never go on the card)"),
    Msg::new("gui.gear.col.normal_module", "法向模数 mn", "Normal module mn"),
    Msg::new("gui.gear.formula.normal_module", "九字段表达式反解 `M`（斜齿即法向模数 Mn）", "Back-solved from the nine-field expression `M` (for helical gears this is the normal module Mn)"),
    Msg::new("gui.gear.source.generator_expr", "轴/齿轮生成器同一份表达式", "Same expression as the shaft/gear generator"),
    Msg::new("gui.gear.col.teeth", "齿数 z", "Number of teeth z"),
    Msg::new("gui.gear.formula.teeth", "九字段表达式反解 `Z`", "Back-solved from the nine-field expression `Z`"),
    Msg::new("gui.gear.col.alpha", "齿形角 α", "Profile angle α"),
    Msg::new("gui.gear.formula.alpha", "九字段表达式反解 `ALPHA`（法向齿形角）", "Back-solved from the nine-field expression `ALPHA` (normal profile angle)"),
    Msg::new("gui.gear.col.ha_coef", "齿顶高系数 ha*", "Addendum coefficient ha*"),
    Msg::new("gui.gear.formula.ha_coef", "外齿 ha* = (Da−d)/(2m) − x；内齿 ha* = (d−Da)/(2m) − x（d = mt·z）", "External: ha* = (Da−d)/(2m) − x; internal: ha* = (d−Da)/(2m) − x (d = mt·z)"),
    Msg::new("gui.gear.source.expr_da", "表达式 DA 反解 + gear.rs 的 d()/mt()", "Back-solved from DA in the expression + gear.rs d()/mt()"),
    Msg::new("gui.gear.col.beta", "螺旋角 β", "Helix angle β"),
    Msg::new("gui.gear.formula.beta", "九字段表达式反解 `BETA`（取绝对值；方向看下一行）", "Back-solved from the nine-field expression `BETA` (absolute value; direction is on the next row)"),
    Msg::new("gui.gear.col.hand", "螺旋方向", "Hand of helix"),
    Msg::new("gui.gear.formula.hand", "β>0 右旋、β<0 左旋、β=0 直齿（表达式 BETA 符号）", "β>0 right-hand, β<0 left-hand, β=0 spur (sign of BETA in the expression)"),
    Msg::new("gui.gear.col.shift", "径向变位系数 x", "Profile shift coefficient x"),
    Msg::new("gui.gear.formula.shift", "九字段表达式反解 `X`", "Back-solved from the nine-field expression `X`"),
    Msg::new("gui.gear.col.whole_depth", "全齿高 h", "Whole depth h"),
    Msg::new("gui.gear.formula.whole_depth", "h = |da − df| / 2（内/外齿同一式）", "h = |da − df| / 2 (same formula for internal and external teeth)"),
    Msg::new("gui.gear.source.engine_da_df", "gear.rs 的 da()/df()", "gear.rs da()/df()"),
    Msg::new("gui.gear.col.grade", "精度等级", "Accuracy grade"),
    Msg::new("gui.gear.formula.grade", "手填（模板该行是样例 TEXT `887FHGB10095-88`，本卡提升为属性，缺省「—」）", "Entered by hand (the template row is a sample TEXT `887FHGB10095-88`; this card promotes it to an attribute, default “—”)"),
    Msg::new("gui.gear.source.user_input_uncollected", "用户填写；本仓未收 GB/T 10095 数据", "Entered by the user; GB/T 10095 data not collected in this repo"),
    Msg::new("gui.gear.col.center", "中心距及极限偏差", "Centre distance and limit deviations"),
    Msg::new("gui.gear.formula.center", "a = mt(z₁+z₂)/2（给配对齿数时；`center` 可显式覆盖）；极限偏差 ±fα 本仓未收", "a = mt(z₁+z₂)/2 (when mating teeth are given; `center` can override it); the ±fα limit deviation is not collected in this repo"),
    Msg::new("gui.gear.source.engine_mt", "gear.rs 的 mt()；GB/T 10095-88 未收", "gear.rs mt(); GB/T 10095-88 not collected"),
    Msg::new("gui.gear.col.mate_dwg", "配对齿轮图号", "Mating gear drawing no."),
    Msg::new("gui.gear.formula.mate_dwg", "手填 `dwg`（缺省「—」）", "Entered by hand as `dwg` (default “—”)"),
    Msg::new("gui.gear.source.user_input", "用户填写", "Entered by the user"),
    Msg::new("gui.gear.col.mate_teeth", "配对齿轮齿数", "Mating gear teeth"),
    Msg::new("gui.gear.formula.mate_teeth", "手填 `mate`（中心距用；缺省「—」）", "Entered by hand as `mate` (used for the centre distance; default “—”)"),
    Msg::new("gui.gear.col.runout", "齿圈径向跳动公差 Fr", "Radial runout tolerance Fr"),
    Msg::new("gui.gear.formula.runout", "GB/T 10095-88 齿圈径向跳动公差 —— 本仓未收该标准数据", "GB/T 10095-88 radial runout tolerance — this standard's data is not collected in this repo"),
    Msg::new("gui.gear.source.missing_no_guess", "缺（不臆造）", "Missing (no guesswork)"),
    Msg::new("gui.gear.col.fw_var", "公法线长度变动公差 FW", "Base tangent length variation tolerance FW"),
    Msg::new("gui.gear.formula.fw_var", "GB/T 10095-88 公法线长度变动公差 —— 本仓未收该标准数据", "GB/T 10095-88 base tangent length variation tolerance — this standard's data is not collected in this repo"),
    Msg::new("gui.gear.col.profile_tol", "齿形公差 ff", "Profile tolerance ff"),
    Msg::new("gui.gear.formula.profile_tol", "GB/T 10095-88 齿形公差 —— 本仓未收该标准数据", "GB/T 10095-88 profile tolerance — this standard's data is not collected in this repo"),
    Msg::new("gui.gear.col.pitch_dev", "齿距极限偏差 fpt", "Pitch limit deviation fpt"),
    Msg::new("gui.gear.formula.pitch_dev", "GB/T 10095-88 齿距极限偏差 —— 本仓未收该标准数据", "GB/T 10095-88 pitch limit deviation — this standard's data is not collected in this repo"),
    Msg::new("gui.gear.col.helix_tol", "齿向公差 Fβ", "Helix tolerance Fβ"),
    Msg::new("gui.gear.formula.helix_tol", "GB/T 10095-88 齿向公差 —— 本仓未收该标准数据", "GB/T 10095-88 helix tolerance — this standard's data is not collected in this repo"),
    Msg::new("gui.gear.col.base_tangent", "公法线 W", "Base tangent length W"),
    Msg::new("gui.gear.formula.base_tangent", "W = m·cosαt·[(k−0.5)π + z·invαt] + 2·x·m·sinαt（直齿外齿轮）", "W = m·cosαt·[(k−0.5)π + z·invαt] + 2·x·m·sinαt (spur external gear)"),
    Msg::new("gui.gear.source.span_measurement", "gear.rs span_measurement()（与 GB/T 3478.6 式(11) 同一条渐开线跨距式）", "gear.rs span_measurement() (the same involute span formula as GB/T 3478.6 eq. (11))"),
    Msg::new("gui.gear.col.span_teeth", "公法线跨测齿数 k", "Span teeth k"),
    Msg::new("gui.gear.formula.span_teeth", "k = round(z·αt/180° + 0.5)（直齿外齿轮；30° 时即 z/6+0.5）", "k = round(z·αt/180° + 0.5) (spur external gear; for 30° this is z/6+0.5)"),
    Msg::new("gui.gear.source.span_measurement_plain", "gear.rs span_measurement()", "gear.rs span_measurement()"),
    Msg::new("gui.nf.col.standard", "执行标准", "Standard"),
    Msg::new("gui.nf.formula.standard", "固定 NF E22-141（p01 封面）", "Fixed to NF E22-141 (p01 cover)"),
    Msg::new("gui.nf.source.standard", "NF E22-141", "NF E22-141"),
    Msg::new("gui.nf.col.centering", "定心方式", "Centering"),
    Msg::new("gui.nf.formula.centering", "缺省外径定心（Az=A）；齿面定心 Az=A+0.3m", "Defaults to major-dia. centering (Az=A); flank centering Az=A+0.3m"),
    Msg::new("gui.nf.source.centering_p18", "p10–p12；p18 表题「拉削的内花键(外径定心)」", "p10–p12; p18 table heading “broached internal spline (major-dia. centering)”"),
    Msg::new("gui.nf.col.module", "模数 m", "Module m"),
    Msg::new("gui.nf.formula.module", "输入（NF 模数档 0.50/0.75/1/1.25/1.667/2.5/3.75/5/7.5/10）", "Input (NF module series 0.50/0.75/1/1.25/1.667/2.5/3.75/5/7.5/10)"),
    Msg::new("gui.nf.source.module_col_p18", "p18 尺寸表实际 m 列", "Actual m column of the p18 dimension table"),
    Msg::new("gui.nf.col.teeth", "齿数 z", "Number of teeth z"),
    Msg::new("gui.nf.formula.teeth", "输入或取 p18 表 N；与表值不一致直接报错", "Input or taken from the p18 table N; a mismatch with the table is an error"),
    Msg::new("gui.nf.source.dims", "NF E22-141 中文译本 p18（拉削内花键尺寸表；assets/nf_e22141_dims.csv）", "NF E22-141 Chinese translation p18 (broached internal spline dimension table; assets/nf_e22141_dims.csv)"),
    Msg::new("gui.nf.col.alpha", "压力角 a", "Pressure angle a"),
    Msg::new("gui.nf.formula.alpha", "NF E22-141 全表 20°", "20° throughout NF E22-141"),
    Msg::new("gui.nf.source.p07", "p07", "p07"),
    Msg::new("gui.nf.col.root_form", "齿根样式", "Root form"),
    Msg::new("gui.nf.formula.root_form", "平齿根（缺省）/ 圆齿根；槽底圆角 ri 见预览读数", "Flat root (default) / fillet root; the root fillet ri is shown in the preview readout"),
    Msg::new("gui.nf.source.root_form_p06", "p06；p22「内花键槽底圆角半径(平根齿)」", "p06; p22 “internal spline root fillet radius (flat-root teeth)”"),
    Msg::new("gui.nf.col.machining", "加工方法", "Machining method"),
    Msg::new("gui.nf.formula.machining", "拉削（p18 表题即「拉削的内花键」）", "Broaching (the p18 table heading reads “broached internal spline”)"),
    Msg::new("gui.nf.source.machining_figs", "p18 / p19 图三十二·三十三", "p18 / p19 figures 32·33"),
    Msg::new("gui.nf.col.major_dia", "大径 Az（内花键齿根圆）", "Major dia. Az (internal spline root circle)"),
    Msg::new("gui.nf.formula.major_dia", "外径定心 Az=A；拉削+齿面定心 Az=A+0.3m", "Major-dia. centering Az=A; broached + flank centering Az=A+0.3m"),
    Msg::new("gui.nf.source.p04_p07", "p04 / p07", "p04 / p07"),
    Msg::new("gui.nf.col.minor_dia", "小径 D（内花键齿顶圆）", "Minor dia. D (internal spline tip circle)"),
    Msg::new("gui.nf.formula.minor_dia", "D = A − 2m（任何情况）；与 p18 表 D 列交叉核对", "D = A − 2m (in all cases); cross-checked against column D of the p18 table"),
    Msg::new("gui.nf.source.p04_p05_p07_p18", "p04 / p05 / p07；p18", "p04 / p05 / p07; p18"),
    Msg::new("gui.nf.col.base_size", "基准尺寸 Do", "Basic size Do"),
    Msg::new("gui.nf.formula.base_size", "Do = A（NF 主参数）", "Do = A (NF primary parameter)"),
    Msg::new("gui.nf.col.pin_dia", "量棒直径 V", "Pin diameter V"),
    Msg::new("gui.nf.formula.pin_dia", "p23–p25 检查表 V 列（表外 → 「—」，不外推）", "Column V of the p23–p25 inspection table (outside the table → “—”, no extrapolation)"),
    Msg::new("gui.nf.source.check", "NF E22-141 中文译本 p23–p25（检查尺寸表；assets/nf_e22141_check.csv）", "NF E22-141 Chinese translation p23–p25 (inspection dimension table; assets/nf_e22141_check.csv)"),
    Msg::new("gui.nf.col.over_pins", "跨棒距 G", "Measurement over pins G"),
    Msg::new("gui.nf.formula.over_pins", "p23–p25 检查表 G 列（G1 见预览读数；表外 → 「—」）", "Column G of the p23–p25 inspection table (G1 in the preview readout; outside the table → “—”)"),
    Msg::new("gui.nf.col.major_dia_up", "大径上差", "Major dia. upper dev."),
    Msg::new("gui.nf.formula.major_dia_up", "ISO 286 R7（p28 §4 内花键大径公差）", "ISO 286 R7 (p28 §4 internal spline major-dia. tolerance)"),
    Msg::new("gui.nf.source.major_tol", "NF E22-141 p28 §4（内花键大径公差 R7；数值按 ISO 286 查表）", "NF E22-141 p28 §4 (internal spline major-dia. tolerance R7; values looked up in ISO 286)"),
    Msg::new("gui.nf.col.major_dia_down", "大径下差", "Major dia. lower dev."),
    Msg::new("gui.nf.formula.major_dia_down", "ISO 286 R7（p28 §4 内花键大径公差）", "ISO 286 R7 (p28 §4 internal spline major-dia. tolerance)"),
    Msg::new("gui.nf.col.minor_dia_up", "小径上差", "Minor dia. upper dev."),
    Msg::new("gui.nf.formula.minor_dia_up", "ISO 286 H7（p28 §6 内花键小径公差，参考）", "ISO 286 H7 (p28 §6 internal spline minor-dia. tolerance, reference)"),
    Msg::new("gui.nf.source.minor_tol", "NF E22-141 p28 §6（内花键小径公差 H7，参考；数值按 ISO 286 查表）", "NF E22-141 p28 §6 (internal spline minor-dia. tolerance H7, reference; values looked up in ISO 286)"),
    Msg::new("gui.nf.col.minor_dia_down", "小径下差", "Minor dia. lower dev."),
    Msg::new("gui.nf.formula.minor_dia_down", "ISO 286 H7（p28 §6 内花键小径公差，参考）", "ISO 286 H7 (p28 §6 internal spline minor-dia. tolerance, reference)"),
    Msg::new("gui.nf.col.over_pins_up", "跨棒距上差", "Over-pins upper dev."),
    Msg::new("gui.nf.formula.over_pins_up", "p29 内花键 E 偏差上差（µm→mm）", "p29 internal spline E deviation, upper (µm→mm)"),
    Msg::new("gui.nf.source.over_pins_tol", "NF E22-141 p29（检查尺寸的公差值：内花键 E 的偏差，微米；表外不外推）", "NF E22-141 p29 (tolerance values of the inspection dimensions: internal spline E deviations, micrometres; no extrapolation outside the table)"),
    Msg::new("gui.nf.col.over_pins_down", "跨棒距下差", "Over-pins lower dev."),
    Msg::new("gui.nf.formula.over_pins_down", "p29 内花键 E 偏差下差（µm→mm）", "p29 internal spline E deviation, lower (µm→mm)"),
    Msg::new("gui.nf_ext.col.standard", "执行标准", "Standard"),
    Msg::new("gui.nf_ext.formula.standard", "固定 NF E22-141（p01 封面）", "Fixed to NF E22-141 (p01 cover)"),
    Msg::new("gui.nf_ext.source.standard", "NF E22-141", "NF E22-141"),
    Msg::new("gui.nf_ext.col.centering", "定心方式", "Centering"),
    Msg::new("gui.nf_ext.formula.centering", "缺省齿面定心（Dee=A−0.2m，模板）；外径定心 Dee=A", "Defaults to flank centering (Dee=A−0.2m, template); major-dia. centering Dee=A"),
    Msg::new("gui.nf_ext.source.centering_p04", "p04/p07；模板示例「齿形定心」", "p04/p07; template sample “flank centering”"),
    Msg::new("gui.nf_ext.col.module", "模数 m", "Module m"),
    Msg::new("gui.nf_ext.formula.module", "输入（NF 模数档 0.50/0.75/1/1.25/1.667/2.5/3.75/5/7.5/10）", "Input (NF module series 0.50/0.75/1/1.25/1.667/2.5/3.75/5/7.5/10)"),
    Msg::new("gui.nf_ext.source.module_col_p20_22", "p20–p22 尺寸表实际 m 列", "Actual m column of the p20–p22 dimension tables"),
    Msg::new("gui.nf_ext.col.teeth", "齿数 z", "Number of teeth z"),
    Msg::new("gui.nf_ext.formula.teeth", "输入或取 p20–p22 表 N；与表值不一致直接报错", "Input or taken from the p20–p22 tables N; a mismatch with the table is an error"),
    Msg::new("gui.nf_ext.source.dims", "NF E22-141 中文译本 p20–p22（外花键尺寸表；assets/nf_e22141_dims.csv）", "NF E22-141 Chinese translation p20–p22 (external spline dimension tables; assets/nf_e22141_dims.csv)"),
    Msg::new("gui.nf_ext.col.alpha", "压力角 a", "Pressure angle a"),
    Msg::new("gui.nf_ext.formula.alpha", "NF E22-141 全表 20°", "20° throughout NF E22-141"),
    Msg::new("gui.nf_ext.source.p07", "p07", "p07"),
    Msg::new("gui.nf_ext.col.root_form", "齿根样式", "Root form"),
    Msg::new("gui.nf_ext.formula.root_form", "平齿根（缺省）/ 圆齿根；对应 Die=A−2.4m / A−2.694m", "Flat root (default) / fillet root; corresponds to Die=A−2.4m / A−2.694m"),
    Msg::new("gui.nf_ext.source.p05_p07", "p05 / p07", "p05 / p07"),
    Msg::new("gui.nf_ext.col.machining", "加工方法", "Machining method"),
    Msg::new("gui.nf_ext.formula.machining", "滚齿（外花键模板口径）", "Hobbing (external spline template definition)"),
    Msg::new("gui.nf_ext.source.machining_template", "模板；p19 图三十二/三十三（滚齿/插齿）", "Template; p19 figures 32/33 (hobbing/shaping)"),
    Msg::new("gui.nf_ext.col.major_dia", "大径 Dee（外花键齿顶圆）", "Major dia. Dee (external spline tip circle)"),
    Msg::new("gui.nf_ext.formula.major_dia", "齿面定心 Dee=A−0.2m；外径定心 Dee=A", "Flank centering Dee=A−0.2m; major-dia. centering Dee=A"),
    Msg::new("gui.nf_ext.source.p04_p07_p22", "p04 / p07；p22 表对账", "p04 / p07; reconciled with the p22 tables"),
    Msg::new("gui.nf_ext.col.minor_dia", "小径 Die（外花键齿根圆）", "Minor dia. Die (external spline root circle)"),
    Msg::new("gui.nf_ext.formula.minor_dia", "平齿根 Die=A−2.4m；圆齿根 Die=A−2.694m", "Flat root Die=A−2.4m; fillet root Die=A−2.694m"),
    Msg::new("gui.nf_ext.source.p05_p07_p22", "p05 / p07；p22 表平/圆齿根列对账", "p05 / p07; reconciled with the flat/fillet-root columns of the p22 tables"),
    Msg::new("gui.nf_ext.col.base_size", "基准尺寸 Do", "Basic size Do"),
    Msg::new("gui.nf_ext.formula.base_size", "Do = A（NF 主参数）", "Do = A (NF primary parameter)"),
    Msg::new("gui.nf_ext.col.span_teeth", "跨测齿数 K", "Span teeth K"),
    Msg::new("gui.nf_ext.formula.span_teeth", "p23–p25 检查表 K 列（表外 → 「—」，不外推）", "Column K of the p23–p25 inspection table (outside the table → “—”, no extrapolation)"),
    Msg::new("gui.nf_ext.source.check", "NF E22-141 中文译本 p23–p25（检查尺寸表；assets/nf_e22141_check.csv）", "NF E22-141 Chinese translation p23–p25 (inspection dimension table; assets/nf_e22141_check.csv)"),
    Msg::new("gui.nf_ext.col.base_tangent", "公法线 W", "Base tangent length W"),
    Msg::new("gui.nf_ext.formula.base_tangent", "p23–p25 检查表 E 列 = K 齿公法线长度（表外 → 「—」）", "Column E of the p23–p25 inspection table = base tangent length over K teeth (outside the table → “—”)"),
    Msg::new("gui.nf_ext.col.major_dia_up", "大径上差", "Major dia. upper dev."),
    Msg::new("gui.nf_ext.formula.major_dia_up", "ISO 286 h12（模板实测口径）", "ISO 286 h12 (template-measured definition)"),
    Msg::new("gui.nf_ext.source.major_tol", "ISO 286 h12（外花键齿顶圆非功能直径；模板示例 Dee=298.5 实测 0/−0.460 一致）", "ISO 286 h12 (external spline tip circle, non-functional diameter; consistent with the template sample Dee=298.5 measured 0/−0.460)"),
    Msg::new("gui.nf_ext.col.major_dia_down", "大径下差", "Major dia. lower dev."),
    Msg::new("gui.nf_ext.formula.major_dia_down", "ISO 286 h12（模板实测口径）", "ISO 286 h12 (template-measured definition)"),
    Msg::new("gui.nf_ext.col.minor_dia_up", "小径上差", "Minor dia. upper dev."),
    Msg::new("gui.nf_ext.formula.minor_dia_up", "ISO 286 H7（模板实测口径）", "ISO 286 H7 (template-measured definition)"),
    Msg::new("gui.nf_ext.source.minor_tol", "ISO 286 H7（模板示例 Die=282 实测 +0.052/0 一致；与 NF 内卡小径同口径）", "ISO 286 H7 (consistent with the template sample Die=282 measured +0.052/0; same definition as the NF internal card minor dia.)"),
    Msg::new("gui.nf_ext.col.minor_dia_down", "小径下差", "Minor dia. lower dev."),
    Msg::new("gui.nf_ext.formula.minor_dia_down", "ISO 286 H7（模板实测口径）", "ISO 286 H7 (template-measured definition)"),
    Msg::new("gui.nf_ext.col.base_tangent_up", "公法线上差", "Base tangent upper dev."),
    Msg::new("gui.nf_ext.formula.base_tangent_up", "p29 外花键 E 偏差上差（µm→mm；按所选配合）", "p29 external spline E deviation, upper (µm→mm; per the selected fit)"),
    Msg::new("gui.nf_ext.source.w_tol", "NF E22-141 p29（检查尺寸的公差值：外花键 E 的偏差，微米；按所选配合）", "NF E22-141 p29 (tolerance values of the inspection dimensions: external spline E deviations, micrometres; per the selected fit)"),
    Msg::new("gui.nf_ext.col.base_tangent_down", "公法线下差", "Base tangent lower dev."),
    Msg::new("gui.nf_ext.formula.base_tangent_down", "p29 外花键 E 偏差下差（µm→mm；按所选配合）", "p29 external spline E deviation, lower (µm→mm; per the selected fit)"),
    Msg::new("gui.din.formula.designation", "§8 代号：标签格 `Nabe/Welle DIN 5480` + 值格 `N/W d_B×m×z×等级+偏差字母`", "§8 designation: label cell `Nabe/Welle DIN 5480` + value cell `N/W d_B×m×z×grade+deviation letter`"),
    Msg::new("gui.din.formula.input", "输入", "Input"),
    Msg::new("gui.din.formula.alpha", "DIN 5480 固定 30°", "Fixed at 30° in DIN 5480"),
    Msg::new("gui.din.formula.root_dia", "名义表 d_f2 + A_df2；d_a1（齿侧定心 h11）", "Nominal table d_f2 + A_df2; d_a1 (flank centering h11)"),
    Msg::new("gui.din.formula.root_form_dia", "名义表 d_Ff2min（min.）/ d_Ff1max（max.）", "Nominal table d_Ff2min (min.) / d_Ff1max (max.)"),
    Msg::new("gui.din.formula.tip_dia", "名义表 d_a2（H11）/ d_f1 + A_df1", "Nominal table d_a2 (H11) / d_f1 + A_df1"),
    Msg::new("gui.din.formula.space_width_max", "emax = e2 + Ae + Tact + Teff；svmax = s1 + As", "emax = e2 + Ae + Tact + Teff; svmax = s1 + As"),
    Msg::new("gui.din.formula.space_width_min", "emin = e2 + Ae + Teff（actual Ref.）；smax = s1 + As − Teff", "emin = e2 + Ae + Teff (actual Ref.); smax = s1 + As − Teff"),
    Msg::new("gui.din.formula.space_width_eff", "evmin = e2 + Ae；smin = s1 + As − Tact − Teff", "evmin = e2 + Ae; smin = s1 + As − Tact − Teff"),
    Msg::new("gui.din.formula.measuring_circle", "DIN 3977 系列（5480-2 检验表）", "DIN 3977 series (5480-2 inspection table)"),
    Msg::new("gui.din.formula.over_pins_max", "M2max（棒间距）/ M1max Ref.（跨棒距）", "M2max (pin spacing) / M1max Ref. (measurement over pins)"),
    Msg::new("gui.din.formula.over_pins_min", "M2min Ref. / M1min", "M2min Ref. / M1min"),
    Msg::new("gui.din.source.clause8", "DIN 5480-1:2006 §8", "DIN 5480-1:2006 §8"),
    Msg::new("gui.din.source.bild6_row2", "Bild 6 行 2", "Bild 6 row 2"),
    Msg::new("gui.din.source.bild6_row3", "Bild 6 行 3", "Bild 6 row 3"),
    Msg::new("gui.din.source.clause5_row4", "§5 / Bild 6 行 4", "§5 / Bild 6 row 4"),
    Msg::new("gui.din.source.nominal_table5", "DIN 5480-2 名义表 + Table 5", "DIN 5480-2 nominal table + Table 5"),
    Msg::new("gui.din.source.nominal_table", "DIN 5480-2 名义表", "DIN 5480-2 nominal table"),
    Msg::new("gui.din.source.clause10_8", "§10.8 表 6", "§10.8 table 6"),
    Msg::new("gui.din.source.insp_table", "DIN 5480-2 检验表", "DIN 5480-2 inspection table"),
    Msg::new("gui.din.source.insp_table_bild6", "DIN 5480-2 检验表 / Bild 6", "DIN 5480-2 inspection table / Bild 6"),
    Msg::new("gui.gb_lite.formula.standard_expr", "GB/T 3478.1-2008 渐开线花键（精简卡口径：只列基本参数与主要测量量，不含公差列）", "GB/T 3478.1-2008 involute splines (lite-card definition: basic parameters and main measurement quantities only, no tolerance columns)"),
    Msg::new("gui.gb_lite.formula.module", "m = 九字段齿形表达式反解（与 GB 花键卡同一 `spline_tol::compute()`）", "m = back-solved from the nine-field tooth-profile expression (same `spline_tol::compute()` as the GB spline card)"),
    Msg::new("gui.gb_lite.formula.teeth", "z = 九字段齿形表达式反解", "z = back-solved from the nine-field tooth-profile expression"),
    Msg::new("gui.gb_lite.formula.alpha", "αD = 表达式反解（30/37.5/45）", "αD = back-solved from the expression (30/37.5/45)"),
    Msg::new("gui.gb_lite.formula.root_form", "由表达式 DA/DF 反解或手选；平齿根（仅 30°）/ 圆齿根", "Back-solved from DA/DF in the expression or selected by hand; flat root (30° only) / fillet root"),
    Msg::new("gui.gb_lite.formula.major_dia_int", "Dei = m(z+1.5) 30°平 / m(z+1.8) 30°圆 / m(z+1.4) 37.5° / m(z+1.2) 45°", "Dei = m(z+1.5) 30° flat / m(z+1.8) 30° fillet / m(z+1.4) 37.5° / m(z+1.2) 45°"),
    Msg::new("gui.gb_lite.formula.minor_dia_int", "Dii = D_Femax(H/h) + 2CF（CF = 0.1m）", "Dii = D_Femax(H/h) + 2CF (CF = 0.1m)"),
    Msg::new("gui.gb_lite.formula.pin_dia", "D'_Ri = Db[tanαci − tan(αci − E_max/D + invαci − invαD)]，按 GB/T 3478.9 R40 取最接近较大值", "D'_Ri = Db[tanαci − tan(αci − E_max/D + invαci − invαD)], nearest larger value from the GB/T 3478.9 R40 series"),
    Msg::new("gui.gb_lite.formula.over_pins", "偶齿 M = Db/cosαi ∓ Dp；奇齿再乘 cos(90°/z)", "Even teeth M = Db/cosαi ∓ Dp; odd teeth multiply again by cos(90°/z)"),
    Msg::new("gui.gb_lite.source.gb3478_1_2008", "GB/T 3478.1-2008", "GB/T 3478.1-2008"),
    Msg::new("gui.gb_lite.source.gb3478_1", "GB/T 3478.1", "GB/T 3478.1"),
    Msg::new("gui.gb_lite.source.gb3478_1_t3", "GB/T 3478.1 表 3", "GB/T 3478.1 table 3"),
    Msg::new("gui.gb_lite.source.gb3478_1_s5", "GB/T 3478.1 §5", "GB/T 3478.1 §5"),
    Msg::new("gui.gb_lite.source.gb3478_1_t3_note", "GB/T 3478.1 表 3 注 2/注 3", "GB/T 3478.1 table 3, notes 2/3"),
    Msg::new("gui.gb_lite.source.gb3478_6_eq1", "GB/T 3478.6 式(1)、GB/T 3478.9 表 1", "GB/T 3478.6 eq. (1), GB/T 3478.9 table 1"),
    Msg::new("gui.gb_lite.source.gb3478_6_eq2_5", "GB/T 3478.6 式(2)~(5)", "GB/T 3478.6 eqs. (2)–(5)"),
    Msg::new("gui.gb_lite.formula.major_dia_ext", "Dee = m(z+1) 30° / m(z+0.9) 37.5° / m(z+0.8) 45°", "Dee = m(z+1) 30° / m(z+0.9) 37.5° / m(z+0.8) 45°"),
    Msg::new("gui.gb_lite.formula.minor_dia_ext", "Die = m(z−1.5) 30°平 / m(z−1.8) 30°圆 / m(z−1.4) 37.5° / m(z−1.2) 45°", "Die = m(z−1.5) 30° flat / m(z−1.8) 30° fillet / m(z−1.4) 37.5° / m(z−1.2) 45°"),
    Msg::new("gui.gb_lite.formula.span_teeth", "K = z/6 + 0.5 取整数", "K = z/6 + 0.5, rounded to an integer"),
    Msg::new("gui.gb_lite.formula.base_tangent", "W_min = cosαD[(K−0.5)πm + D·invαD + esv − (T+λ)]；W = (W_min+W_max)/2", "W_min = cosαD[(K−0.5)πm + D·invαD + esv − (T+λ)]; W = (W_min+W_max)/2"),
    Msg::new("gui.gb_lite.source.gb3478_6_eq11_note", "GB/T 3478.6 式(11) 注", "GB/T 3478.6 eq. (11), note"),
    Msg::new("gui.gb_lite.source.gb3478_6_eq11_12", "GB/T 3478.6 式(11)(12)", "GB/T 3478.6 eqs. (11)(12)"),
    Msg::new("gui.gear_lite.formula.pitch_dia", "d = m·z（与齿轮卡同一 `GearParams`）", "d = m·z (same `GearParams` as the gear card)"),
    Msg::new("gui.gear_lite.formula.tip_dia", "da = d + 2m(ha* + x)（外齿；内齿引擎口径）", "da = d + 2m(ha* + x) (external teeth; internal teeth use the engine definition)"),
    Msg::new("gui.gear_lite.formula.root_dia", "df = d − 2m(ha* + c* − x)（外齿；内齿引擎口径）", "df = d − 2m(ha* + c* − x) (external teeth; internal teeth use the engine definition)"),
    Msg::new("gui.gear_lite.source.d", "齿轮引擎 `GearParams::d()`", "Gear engine `GearParams::d()`"),
    Msg::new("gui.gear_lite.source.da", "齿轮引擎 `GearParams::da()`", "Gear engine `GearParams::da()`"),
    Msg::new("gui.gear_lite.source.df", "齿轮引擎 `GearParams::df()`", "Gear engine `GearParams::df()`"),
    Msg::new("gui.form.gear.note", "粘九字段齿轮表达式 → 选填配对齿轮/图号/精度等级/中心距 → 点「出表」回到图纸放置。", "Paste the nine-field gear expression → optionally fill in the mating gear/drawing no./accuracy grade/centre distance → click “Place table” to return to the drawing."),
    Msg::new("gui.form.gear.missing_note", "GB/T 10095-88 的 Fr/FW/ff/fpt/Fβ 与中心距极限偏差本仓未收；内齿轮/斜齿轮的公法线口径本仓未收 —— 这些格显示「—」，不臆造。", "GB/T 10095-88 Fr/FW/ff/fpt/Fβ and the centre-distance limit deviation are not collected in this repo; the base tangent definition for internal/helical gears is not collected either — those cells show “—”, with no guesswork."),
    Msg::new("gui.form.ansi.note", "粘九字段表达式（自动反解 P/N/齿廓）或直接填 P/N → 选方向/齿廓 → 点「出表」回到图纸放置。", "Paste the nine-field expression (P/N/profile auto back-solved) or fill in P/N directly → choose direction/profile → click “Place table” to return to the drawing."),
    Msg::new("gui.form.ansi.missing_note", "ANSI B92.1 配合/公差（Table 4/5）、量棒检验（p30–p32）与公法线/跨测表本仓未收—— 这些格显示「—」，不臆造。", "ANSI B92.1 fits/tolerances (Table 4/5), pin inspection (p30–p32) and the base tangent/span tables are not collected in this repo — those cells show “—”, with no guesswork."),
    Msg::new("gui.form.nf.note", "粘九字段表达式（自动反解 A/m/z）或直接填 A/m（z 选填核对）→ 选定心/齿根/配合 → 点「出表」回到图纸放置。公差：大径 R7 / 小径 H7（p28）+ 跨棒距 = p29 内花键 E 偏差；配合类别在出表前选定（缺省固定）：决定预览里配对外花键的 E/xm 读数。", "Paste the nine-field expression (A/m/z auto back-solved) or fill in A/m directly (z optional for cross-check) → choose centering/root/fit → click “Place table” to return to the drawing. Tolerances: major dia. R7 / minor dia. H7 (p28) + measurement over pins = p29 internal spline E deviation; the fit class is chosen before placing (fixed by default) and determines the mating external-spline E/xm readouts in the preview."),
    Msg::new("gui.form.nf.missing_note", "(m,A) 不在 p29 或 ISO 档缺时对应公差格显示「—」，不外推；V/V1/G/G1 与 ri 只取 p23–p25 / p22 表值，表外显示「—」。", "When (m,A) is not in p29 or the ISO range is missing the corresponding tolerance cell shows “—”, with no extrapolation; V/V1/G/G1 and ri only take values from the p23–p25 / p22 tables, otherwise they show “—”."),
    Msg::new("gui.form.nf_ext.note", "粘九字段表达式（自动反解 A/m/z）或直接填 A/m（z 选填核对）→ 选定心/齿根/配合 → 点「出表」回到图纸放置。公差：大径 h12 / 小径 H7（模板实测口径）+ 公法线 = p29 外花键 E 偏差；配合类别在出表前选定（缺省固定）：决定公法线 W 公差与预览读数。", "Paste the nine-field expression (A/m/z auto back-solved) or fill in A/m directly (z optional for cross-check) → choose centering/root/fit → click “Place table” to return to the drawing. Tolerances: major dia. h12 / minor dia. H7 (template-measured definition) + base tangent = p29 external spline E deviation; the fit class is chosen before placing (fixed by default) and determines the base tangent W tolerance and the preview readout."),
    Msg::new("gui.form.nf_ext.missing_note", "K/W 只取 p23–p25 检查表值，(m,A) 不在表内显示「—」；p29 表外或 ISO 档缺时对应公差格显示「—」，不外推。", "K/W only take values from the p23–p25 inspection table and show “—” when (m,A) is outside it; when p29 is out of range or the ISO range is missing the corresponding tolerance cell shows “—”, with no extrapolation."),
    Msg::new("gui.form.din.note", "粘九字段表达式（自动反解 d_B/m/z）或直接填 m/z/d_B + 齿配合 → 缺行可用 e₂ / Ae / As / Tact / Teff 覆盖 → 点「出表」回到图纸放置。", "Paste the nine-field expression (d_B/m/z auto back-solved) or fill in m/z/d_B + tooth fit directly → missing rows can be overridden with e₂ / Ae / As / Tact / Teff → click “Place table” to return to the drawing."),
    Msg::new("gui.form.din.missing_note", "Table 7 上段 c1/c2（>400 侧）与 c9（≤12 细档）列映射无实锚 → 「—」；\n                   下段公差表只抽到 6–9 级、模数组 1,75–4 的实锚 → 其余等级/模数组 Tact/Teff 显示「—」；\n                   D_M/M2/M1 无检验表行且非 Bild 6 示例时显示「—」。", "No real anchor for the column mapping of the upper Table 7 c1/c2 (>400 side) and c9 (≤12 fine range) → “—”; the lower tolerance table only has real anchors for grades 6–9 and module group 1.75–4 → Tact/Teff for other grades/module groups show “—”; D_M/M2/M1 show “—” when there is no inspection-table row and the value is not the Bild 6 sample."),
    Msg::new("gui.form.gb_lite_int.note", "精简版：只列基本参数 + 量棒/跨棒距，不含任何公差列；需要公差请用「GB 花键参数表（内）」。", "Lite version: basic parameters + pin/measurement over pins only, no tolerance columns; use “GB spline parameter table (internal)” when tolerances are needed."),
    Msg::new("gui.form.gb_lite_int.missing_note", "精简卡面固定不含公差（上/下偏差）；等级/配合只用于 Md 计算（点项标题可看口径）。", "The lite card never carries tolerances (upper/lower deviations); the grade/fit is used only for the Md calculation (click an item title to see the definition)."),
    Msg::new("gui.form.gb_lite_ext.note", "精简版：只列基本参数 + 公法线，不含任何公差列；需要公差请用「GB 花键参数表（外）」。", "Lite version: basic parameters + base tangent only, no tolerance columns; use “GB spline parameter table (external)” when tolerances are needed."),
    Msg::new("gui.form.gb_lite_ext.missing_note", "精简卡面固定不含公差（上/下偏差）；等级/配合只用于 Wn 计算（点项标题可看口径）。", "The lite card never carries tolerances (upper/lower deviations); the grade/fit is used only for the Wn calculation (click an item title to see the definition)."),
    Msg::new("gui.din.missing_note", "Table 7 上段 c1/c2（>400 侧）与 c9（≤12 细档）列映射无实锚 → 「—」；下段公差表只抽到 6–9 级、模数组 1,75–4 的实锚 → 其余等级/模数组 Tact/Teff 显示「—」；D_M/M2/M1 无检验表行且非 Bild 6 示例时显示「—」；可用 ae/as/e2/tactn/teffn/tactw/teffw 显式覆盖。", "No real anchor for the column mapping of the upper Table 7 c1/c2 (>400 side) and c9 (≤12 fine range) → “—”; the lower tolerance table only has real anchors for grades 6–9 and module group 1.75–4 → Tact/Teff for other grades/module groups show “—”; D_M/M2/M1 show “—” when there is no inspection-table row and the value is not the Bild 6 sample; they can be overridden explicitly with ae/as/e2/tactn/teffn/tactw/teffw."),

    // ── GUI 元数据批 ⑥：表单字段（`CardFieldSpec.label/title/placeholder` + 静态选项）──
    // ★ zh 由 `i18n_检查/card_field_i18n_gen.py` 从 `git HEAD` 源文件**自动提取**（防漂移）；
    //   英译人工（`i18n_检查/card_field_i18n_en.py`）；协议记号/数值/示例串**原样不译**
    //   （`card.rs::field_text()` 只看 `gui.fld.` 前缀）。
    Msg::new("gui.fld.ansi_table.form.expr.label", "齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）", "Tooth-profile expression (nine fields; can be copied from the shaft/gear generator GUI)"),
    Msg::new("gui.fld.ansi_table.form.expr.placeholder", "SPLINE IN M1.5875 Z20 ALPHA30 X0 BETA0 H30（外卡用 EX）", "SPLINE IN M1.5875 Z20 ALPHA30 X0 BETA0 H30 (use EX for the external card)"),
    Msg::new("gui.fld.ansi_table.form.expr.title", "九字段统一齿形表达式（MARK KIND M Z ALPHA X DA DF BETA H）；粘贴后自动反解径节 P/齿数 N/齿廓；本卡方向由卡类型固定（KIND 须与之一致）", "Nine-field tooth-profile expression (MARK KIND M Z ALPHA X DA DF BETA H); pasting back-solves the diametral pitch P / number of teeth N / profile; the direction of this card is fixed by the card type (KIND must match)"),
    Msg::new("gui.fld.ansi_table.form.p.label", "径节 P", "Diametral pitch P"),
    Msg::new("gui.fld.ansi_table.form.p.placeholder", "如 16", "e.g. 16"),
    Msg::new("gui.fld.ansi_table.form.p.title", "径节 P（1/in；ANSI B92.1 主参数）", "Diametral pitch P (1/in; ANSI B92.1 primary parameter)"),
    Msg::new("gui.fld.ansi_table.form.profile.label", "齿廓", "Profile"),
    Msg::new("gui.fld.ansi_table.form.profile.title", "ANSI B92.1 齿廓预设（Table 2 列 A–E；齿形类型/α）", "ANSI B92.1 profile preset (Table 2 columns A–E; tooth form type/α)"),
    Msg::new("gui.fld.ansi_table.form.z.label", "齿数 N", "Number of teeth N"),
    Msg::new("gui.fld.ansi_table.form.z.placeholder", "如 20", "e.g. 20"),
    Msg::new("gui.fld.ansi_table.form.z.title", "齿数 N", "Number of teeth N"),
    Msg::new("gui.fld.din_table.form_ext.ae.label", "Ae 覆盖", "Ae override"),
    Msg::new("gui.fld.din_table.form_ext.ae.placeholder", "选填，如 0", "Optional, e.g. 0"),
    Msg::new("gui.fld.din_table.form_ext.ae.title", "Ae（齿槽宽上偏差）显式覆盖（mm）", "Explicit override of Ae (space width upper deviation, mm)"),
    Msg::new("gui.fld.din_table.form_ext.as_.label", "As 覆盖", "As override"),
    Msg::new("gui.fld.din_table.form_ext.as_.placeholder", "选填，如 -0.028", "Optional, e.g. -0.028"),
    Msg::new("gui.fld.din_table.form_ext.as_.title", "As（齿厚上偏差）显式覆盖（mm）", "Explicit override of As (tooth thickness upper deviation, mm)"),
    Msg::new("gui.fld.din_table.form_ext.d_b.label", "基准直径 d_B", "Reference diameter d_B"),
    Msg::new("gui.fld.din_table.form_ext.d_b.placeholder", "如 120", "e.g. 120"),
    Msg::new("gui.fld.din_table.form_ext.d_b.title", "基准直径 d_B = m·z（DIN 5480）", "Reference diameter d_B = m·z (DIN 5480)"),
    Msg::new("gui.fld.din_table.form_ext.e2.label", "e₂=s₁ 覆盖", "e₂=s₁ override"),
    Msg::new("gui.fld.din_table.form_ext.e2.placeholder", "选填（名义表缺行时）", "Optional (when the nominal table row is missing)"),
    Msg::new("gui.fld.din_table.form_ext.e2.title", "e₂=s₁ 名义值显式覆盖（mm；缺行时用）", "Explicit override of the e₂=s₁ nominal value (mm; used when the table row is missing)"),
    Msg::new("gui.fld.din_table.form_ext.expr.label", "齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）", "Tooth-profile expression (nine fields; can be copied from the shaft/gear generator GUI)"),
    Msg::new("gui.fld.din_table.form_ext.expr.title", "九字段统一齿形表达式（MARK KIND M Z ALPHA X DA DF BETA H）；粘贴后自动反解 d_B=m(z+1.1+2x)、m、z；本卡固定外花键（KIND 须 EX）", "Nine-field tooth-profile expression (MARK KIND M Z ALPHA X DA DF BETA H); pasting back-solves d_B=m(z+1.1+2x), m and z; this card is fixed to external splines (KIND must be EX)"),
    Msg::new("gui.fld.din_table.form_ext.m.label", "模数 m", "Module m"),
    Msg::new("gui.fld.din_table.form_ext.m.placeholder", "如 3", "e.g. 3"),
    Msg::new("gui.fld.din_table.form_ext.m.title", "DIN 5480-1 模数 m（Bild 6 主参数）", "DIN 5480-1 module m (Bild 6 primary parameter)"),
    Msg::new("gui.fld.din_table.form_ext.shaft.label", "外花键配合", "External spline fit"),
    Msg::new("gui.fld.din_table.form_ext.shaft.placeholder", "如 8f（v…a）", "e.g. 8f (v…a)"),
    Msg::new("gui.fld.din_table.form_ext.shaft.title", "Welle 配合（字母 v…a + 等级数字；Bild 6 示例 8f）", "Welle fit (letters v…a + grade digits; Bild 6 sample 8f)"),
    Msg::new("gui.fld.din_table.form_ext.tact_w.label", "Tact(W) 覆盖", "Tact(W) override"),
    Msg::new("gui.fld.din_table.form_ext.tact_w.placeholder", "选填", "Optional"),
    Msg::new("gui.fld.din_table.form_ext.tact_w.title", "Welle 实际齿厚公差显式覆盖（mm）", "Explicit override of the Welle actual tooth thickness tolerance (mm)"),
    Msg::new("gui.fld.din_table.form_ext.teff_w.label", "Teff(W) 覆盖", "Teff(W) override"),
    Msg::new("gui.fld.din_table.form_ext.teff_w.placeholder", "选填", "Optional"),
    Msg::new("gui.fld.din_table.form_ext.teff_w.title", "Welle 作用齿厚公差显式覆盖（mm）", "Explicit override of the Welle effective tooth thickness tolerance (mm)"),
    Msg::new("gui.fld.din_table.form_ext.z.label", "齿数 z", "Number of teeth z"),
    Msg::new("gui.fld.din_table.form_ext.z.placeholder", "如 38", "e.g. 38"),
    Msg::new("gui.fld.din_table.form_ext.z.title", "齿数 z", "Number of teeth z"),
    Msg::new("gui.fld.din_table.form_int.ae.label", "Ae 覆盖", "Ae override"),
    Msg::new("gui.fld.din_table.form_int.ae.placeholder", "选填，如 0", "Optional, e.g. 0"),
    Msg::new("gui.fld.din_table.form_int.ae.title", "Ae（齿槽宽上偏差）显式覆盖（mm）", "Explicit override of Ae (space width upper deviation, mm)"),
    Msg::new("gui.fld.din_table.form_int.as_.label", "As 覆盖", "As override"),
    Msg::new("gui.fld.din_table.form_int.as_.placeholder", "选填，如 -0.028", "Optional, e.g. -0.028"),
    Msg::new("gui.fld.din_table.form_int.as_.title", "As（齿厚上偏差）显式覆盖（mm）", "Explicit override of As (tooth thickness upper deviation, mm)"),
    Msg::new("gui.fld.din_table.form_int.d_b.label", "基准直径 d_B", "Reference diameter d_B"),
    Msg::new("gui.fld.din_table.form_int.d_b.placeholder", "如 120", "e.g. 120"),
    Msg::new("gui.fld.din_table.form_int.d_b.title", "基准直径 d_B = m·z（DIN 5480）", "Reference diameter d_B = m·z (DIN 5480)"),
    Msg::new("gui.fld.din_table.form_int.e2.label", "e₂=s₁ 覆盖", "e₂=s₁ override"),
    Msg::new("gui.fld.din_table.form_int.e2.placeholder", "选填（名义表缺行时）", "Optional (when the nominal table row is missing)"),
    Msg::new("gui.fld.din_table.form_int.e2.title", "e₂=s₁ 名义值显式覆盖（mm；缺行时用）", "Explicit override of the e₂=s₁ nominal value (mm; used when the table row is missing)"),
    Msg::new("gui.fld.din_table.form_int.expr.label", "齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）", "Tooth-profile expression (nine fields; can be copied from the shaft/gear generator GUI)"),
    Msg::new("gui.fld.din_table.form_int.expr.title", "九字段统一齿形表达式（MARK KIND M Z ALPHA X DA DF BETA H）；粘贴后自动反解 d_B=m(z+1.1+2x)、m、z；本卡固定内花键（KIND 须 IN）", "Nine-field tooth-profile expression (MARK KIND M Z ALPHA X DA DF BETA H); pasting back-solves d_B=m(z+1.1+2x), m and z; this card is fixed to internal splines (KIND must be IN)"),
    Msg::new("gui.fld.din_table.form_int.hub.label", "内花键配合", "Internal spline fit"),
    Msg::new("gui.fld.din_table.form_int.hub.placeholder", "如 9H（F/G/H/J/K/M）", "e.g. 9H (F/G/H/J/K/M)"),
    Msg::new("gui.fld.din_table.form_int.hub.title", "Nabe 配合（字母 F/G/H/J/K/M + 等级数字；Bild 6 示例 9H）", "Nabe fit (letters F/G/H/J/K/M + grade digits; Bild 6 sample 9H)"),
    Msg::new("gui.fld.din_table.form_int.m.label", "模数 m", "Module m"),
    Msg::new("gui.fld.din_table.form_int.m.placeholder", "如 3", "e.g. 3"),
    Msg::new("gui.fld.din_table.form_int.m.title", "DIN 5480-1 模数 m（Bild 6 主参数）", "DIN 5480-1 module m (Bild 6 primary parameter)"),
    Msg::new("gui.fld.din_table.form_int.tact_n.label", "Tact(N) 覆盖", "Tact(N) override"),
    Msg::new("gui.fld.din_table.form_int.tact_n.placeholder", "选填", "Optional"),
    Msg::new("gui.fld.din_table.form_int.tact_n.title", "Nabe 实际齿槽宽公差显式覆盖（mm）", "Explicit override of the Nabe actual space width tolerance (mm)"),
    Msg::new("gui.fld.din_table.form_int.teff_n.label", "Teff(N) 覆盖", "Teff(N) override"),
    Msg::new("gui.fld.din_table.form_int.teff_n.placeholder", "选填", "Optional"),
    Msg::new("gui.fld.din_table.form_int.teff_n.title", "Nabe 作用齿槽宽公差显式覆盖（mm）", "Explicit override of the Nabe effective space width tolerance (mm)"),
    Msg::new("gui.fld.din_table.form_int.z.label", "齿数 z", "Number of teeth z"),
    Msg::new("gui.fld.din_table.form_int.z.placeholder", "如 38", "e.g. 38"),
    Msg::new("gui.fld.din_table.form_int.z.title", "齿数 z", "Number of teeth z"),
    Msg::new("gui.fld.gear_table.form.center.label", "中心距 a", "Centre distance a"),
    Msg::new("gui.fld.gear_table.form.center.placeholder", "选填，覆盖计算值", "Optional, overrides the computed value"),
    Msg::new("gui.fld.gear_table.form.center.title", "显式覆盖 mt(z₁+z₂)/2（mm）", "Explicit override of mt(z₁+z₂)/2 (mm)"),
    Msg::new("gui.fld.gear_table.form.expr.label", "齿轮齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）", "Gear tooth-profile expression (nine fields; can be copied from the shaft/gear generator GUI)"),
    Msg::new("gui.fld.gear_table.form.expr.title", "九字段统一齿形表达式（与轴/齿轮生成器同口径）；反解 m/z/αt/ha*/c*", "Nine-field tooth-profile expression (same definition as the shaft/gear generator); back-solves m/z/αt/ha*/c*"),
    Msg::new("gui.fld.gear_table.form.grade.label", "精度等级", "Accuracy grade"),
    Msg::new("gui.fld.gear_table.form.grade.placeholder", "选填", "Optional"),
    Msg::new("gui.fld.gear_table.form.grade.title", "手填字符串（如 7-7-7），进「精度等级」行；缺省「—」", "Hand-entered string (e.g. 7-7-7) written into the “Accuracy grade” row; defaults to “—”"),
    Msg::new("gui.fld.gear_table.form.mate_dwg.label", "配对齿轮图号", "Mating gear drawing no."),
    Msg::new("gui.fld.gear_table.form.mate_dwg.placeholder", "选填", "Optional"),
    Msg::new("gui.fld.gear_table.form.mate_dwg.title", "手填字符串，进「配对齿轮图号」行", "Hand-entered string written into the “Mating gear drawing no.” row"),
    Msg::new("gui.fld.gear_table.form.mate_z.label", "配对齿轮齿数 z₂", "Mating gear teeth z₂"),
    Msg::new("gui.fld.gear_table.form.mate_z.placeholder", "选填", "Optional"),
    Msg::new("gui.fld.gear_table.form.mate_z.title", "中心距 a = mt(z₁+z₂)/2；不填则中心距显示「—」", "Centre distance a = mt(z₁+z₂)/2; when left blank the centre distance shows “—”"),
    Msg::new("gui.fld.nf_ext_table.form.a.label", "公称直径 A", "Nominal diameter A"),
    Msg::new("gui.fld.nf_ext_table.form.a.placeholder", "NF 主参数（表值）", "NF primary parameter (table value)"),
    Msg::new("gui.fld.nf_ext_table.form.a.title", "NF 主参数 A；同一 A 可对应不同模数（p20–p22 尺寸表）", "NF primary parameter A; the same A can correspond to different modules (p20–p22 dimension tables)"),
    Msg::new("gui.fld.nf_ext_table.form.centering.label", "定心方式", "Centering"),
    Msg::new("gui.fld.nf_ext_table.form.centering.title", "NF E22-141 外花键定心方式（p04/p07）；缺省齿面定心 Dee=A−0.2m（模板示例）", "NF E22-141 external-spline centering (p04/p07); defaults to flank centering Dee=A−0.2m (template sample)"),
    Msg::new("gui.fld.nf_ext_table.form.expr.label", "齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）", "Tooth-profile expression (nine fields; can be copied from the shaft/gear generator GUI)"),
    Msg::new("gui.fld.nf_ext_table.form.expr.title", "九字段统一齿形表达式（MARK KIND M Z ALPHA X DA DF BETA H）；粘贴后自动反解 A=m(z+0.4+2x)、m、z；NF 压力角恒 20°、方向恒外（EX）", "Nine-field tooth-profile expression (MARK KIND M Z ALPHA X DA DF BETA H); pasting back-solves A=m(z+0.4+2x), m and z; the NF pressure angle is always 20° and the direction is always external (EX)"),
    Msg::new("gui.fld.nf_ext_table.form.fit.label", "配合类别（出表前选定）", "Fit class (chosen before placing)"),
    Msg::new("gui.fld.nf_ext_table.form.fit.opt.fixed", "固定", "Fixed"),
    Msg::new("gui.fld.nf_ext_table.form.fit.opt.loose", "松动", "Loose"),
    Msg::new("gui.fld.nf_ext_table.form.fit.opt.press", "压", "Press"),
    Msg::new("gui.fld.nf_ext_table.form.fit.opt.slide", "滑动", "Slide"),
    Msg::new("gui.fld.nf_ext_table.form.fit.title", "NF E22-141 p31/p34：松动/滑动/固定/压；缺省固定；★ 出表前先选好——该选择决定公法线 W 上/下差（p29 外花键 E）与预览读数", "NF E22-141 p31/p34: loose/slide/fixed/press; fixed by default; ★ choose it before placing — this choice determines the base tangent W upper/lower deviations (p29 external spline E) and the preview readouts"),
    Msg::new("gui.fld.nf_ext_table.form.m.label", "模数 m", "Module m"),
    Msg::new("gui.fld.nf_ext_table.form.m.placeholder", "NF 模数档", "NF module series"),
    Msg::new("gui.fld.nf_ext_table.form.m.title", "NF 模数档（p20–p22 表实际 m 列：0.50/0.75/1/1.25/1.667/2.5/3.75/5/7.5/10）", "NF module series (actual m column of the p20–p22 tables: 0.50/0.75/1/1.25/1.667/2.5/3.75/5/7.5/10)"),
    Msg::new("gui.fld.nf_ext_table.form.root.label", "齿根样式", "Root form"),
    Msg::new("gui.fld.nf_ext_table.form.root.title", "行 7 文本；对应 Die=A−2.4m（平）/A−2.694m（圆）", "Row 7 text; corresponds to Die=A−2.4m (flat) / A−2.694m (fillet)"),
    Msg::new("gui.fld.nf_ext_table.form.z.label", "齿数 z", "Number of teeth z"),
    Msg::new("gui.fld.nf_ext_table.form.z.placeholder", "选填，按 p20–p22 表核对", "Optional, cross-checked against the p20–p22 tables"),
    Msg::new("gui.fld.nf_ext_table.form.z.title", "选填；与 p20–p22 表 N 不一致直接报错（避免 K/W 取错行）", "Optional; a mismatch with column N of the p20–p22 tables is an error (avoids picking the wrong K/W row)"),
    Msg::new("gui.fld.nf_table.form.a.label", "公称直径 A", "Nominal diameter A"),
    Msg::new("gui.fld.nf_table.form.a.placeholder", "NF 主参数（表值）", "NF primary parameter (table value)"),
    Msg::new("gui.fld.nf_table.form.a.title", "NF 主参数 A；同一 A 可对应不同模数（p18 尺寸表）", "NF primary parameter A; the same A can correspond to different modules (p18 dimension table)"),
    Msg::new("gui.fld.nf_table.form.centering.label", "定心方式", "Centering"),
    Msg::new("gui.fld.nf_table.form.centering.title", "NF E22-141 内花键定心方式（p04/p07/p10–p12）；缺省外径定心 Az=A", "NF E22-141 internal-spline centering (p04/p07/p10–p12); defaults to major-dia. centering Az=A"),
    Msg::new("gui.fld.nf_table.form.expr.label", "齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）", "Tooth-profile expression (nine fields; can be copied from the shaft/gear generator GUI)"),
    Msg::new("gui.fld.nf_table.form.expr.title", "九字段统一齿形表达式（MARK KIND M Z ALPHA X DA DF BETA H）；粘贴后自动反解 A=m(z+0.4+2x)、m、z；NF 压力角恒 20°", "Nine-field tooth-profile expression (MARK KIND M Z ALPHA X DA DF BETA H); pasting back-solves A=m(z+0.4+2x), m and z; the NF pressure angle is always 20°"),
    Msg::new("gui.fld.nf_table.form.fit.label", "配合类别（出表前选定）", "Fit class (chosen before placing)"),
    Msg::new("gui.fld.nf_table.form.fit.opt.fixed", "固定", "Fixed"),
    Msg::new("gui.fld.nf_table.form.fit.opt.loose", "松动", "Loose"),
    Msg::new("gui.fld.nf_table.form.fit.opt.press", "压", "Press"),
    Msg::new("gui.fld.nf_table.form.fit.opt.slide", "滑动", "Slide"),
    Msg::new("gui.fld.nf_table.form.fit.title", "NF E22-141 p31/p34：松动/滑动/固定/压；缺省固定；★ 出表前先选好——决定预览里配对外花键的 E/xm 偏差读数（内卡跨棒距 G 公差 = p29 内花键 E，不随配合变）", "NF E22-141 p31/p34: loose/slide/fixed/press; fixed by default; ★ choose it before placing — it determines the mating external-spline E/xm deviation readouts in the preview (the internal card’s measurement over pins G tolerance = p29 internal spline E, independent of the fit)"),
    Msg::new("gui.fld.nf_table.form.m.label", "模数 m", "Module m"),
    Msg::new("gui.fld.nf_table.form.m.placeholder", "NF 模数档", "NF module series"),
    Msg::new("gui.fld.nf_table.form.m.title", "NF 模数档（p18 表实际 m 列：0.50/0.75/1/1.25/1.667/2.5/3.75/5/7.5/10）", "NF module series (actual m column of the p18 table: 0.50/0.75/1/1.25/1.667/2.5/3.75/5/7.5/10)"),
    Msg::new("gui.fld.nf_table.form.root.label", "齿根样式", "Root form"),
    Msg::new("gui.fld.nf_table.form.root.title", "行 7 文本；槽底圆角 ri 在预览读数里", "Row 7 text; the root fillet ri is shown in the preview readout"),
    Msg::new("gui.fld.nf_table.form.z.label", "齿数 z", "Number of teeth z"),
    Msg::new("gui.fld.nf_table.form.z.placeholder", "选填，按 p18 表核对", "Optional, cross-checked against the p18 table"),
    Msg::new("gui.fld.nf_table.form.z.title", "选填；与 p18 表 N 不一致直接报错（避免 V/G 取错行）", "Optional; a mismatch with column N of the p18 table is an error (avoids picking the wrong V/G row)"),
    Msg::new("gui.fld.spline_lite.form_ext.expr.label", "齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）", "Tooth-profile expression (nine fields; can be copied from the shaft/gear generator GUI)"),
    Msg::new("gui.fld.spline_lite.form_ext.expr.title", "表达式反解 m/z/αD/x/Da/Df；与 GB 花键卡/轴生成器同一解析器（card_expr）", "Back-solves m/z/αD/x/Da/Df from the expression; same parser as the GB spline card/shaft generator (card_expr)"),
    Msg::new("gui.fld.spline_lite.form_ext.fit.label", "配合类别（只影响 Wn 计算；卡面不显示公差）", "Fit class (affects only the Wn calculation; the card shows no tolerances)"),
    Msg::new("gui.fld.spline_lite.form_ext.fit.title", "外花键基本偏差 h/js/k/d/e/f（与内花键 H 相配）；精简卡不含公差列", "External spline fundamental deviations h/js/k/d/e/f (mating with internal spline H); the lite card carries no tolerance columns"),
    Msg::new("gui.fld.spline_lite.form_ext.grade.label", "公差等级（只影响 Wn 计算；卡面不显示公差）", "Tolerance grade (affects only the Wn calculation; the card shows no tolerances)"),
    Msg::new("gui.fld.spline_lite.form_ext.grade.title", "4/5/6/7 级；精简卡不含公差列，等级只影响测量量的计算口径", "Grades 4/5/6/7; the lite card carries no tolerance columns — the grade only affects how the measured quantities are computed"),
    Msg::new("gui.fld.spline_lite.form_ext.root.label", "齿根样式", "Root form"),
    Msg::new("gui.fld.spline_lite.form_ext.root.opt.auto", "自动（按表达式反解）", "Auto (back-solved from the expression)"),
    Msg::new("gui.fld.spline_lite.form_ext.root.opt.fillet", "圆齿根", "Fillet root"),
    Msg::new("gui.fld.spline_lite.form_ext.root.opt.flat", "平齿根", "Flat root"),
    Msg::new("gui.fld.spline_lite.form_ext.root.title", "平/圆；auto = 由表达式 DA/DF 反解，再退到按 αD 默认", "Flat/fillet; auto = back-solved from DA/DF in the expression, otherwise the αD default"),
    Msg::new("gui.fld.spline_lite.form_int.dp.label", "量棒直径 Dp（留空 = 标准 R40 自动选）", "Pin diameter Dp (blank = picked automatically from the standard R40 series)"),
    Msg::new("gui.fld.spline_lite.form_int.dp.placeholder", "留空自动", "blank = automatic"),
    Msg::new("gui.fld.spline_lite.form_int.dp.title", "手填必须是 GB/T 3478.9 系列值；留空按标准 R40 规则自动选", "A manual value must come from the GB/T 3478.9 series; when blank it is picked automatically by the standard R40 rule"),
    Msg::new("gui.fld.spline_lite.form_int.expr.label", "齿形表达式（九字段；可从轴/齿轮生成器 GUI 复制）", "Tooth-profile expression (nine fields; can be copied from the shaft/gear generator GUI)"),
    Msg::new("gui.fld.spline_lite.form_int.expr.title", "表达式反解 m/z/αD/x/Da/Df；与 GB 花键卡/轴生成器同一解析器（card_expr）", "Back-solves m/z/αD/x/Da/Df from the expression; same parser as the GB spline card/shaft generator (card_expr)"),
    Msg::new("gui.fld.spline_lite.form_int.fit.label", "配合类别（只影响 Md 计算；卡面不显示公差）", "Fit class (affects only the Md calculation; the card shows no tolerances)"),
    Msg::new("gui.fld.spline_lite.form_int.fit.title", "内花键基孔制 H；精简卡不含公差列", "Internal splines use the basic-hole system H; the lite card carries no tolerance columns"),
    Msg::new("gui.fld.spline_lite.form_int.grade.label", "公差等级（只影响 Md 计算；卡面不显示公差）", "Tolerance grade (affects only the Md calculation; the card shows no tolerances)"),
    Msg::new("gui.fld.spline_lite.form_int.grade.title", "4/5/6/7 级；精简卡不含公差列，等级只影响测量量的计算口径", "Grades 4/5/6/7; the lite card carries no tolerance columns — the grade only affects how the measured quantities are computed"),
    Msg::new("gui.fld.spline_lite.form_int.root.label", "齿根样式", "Root form"),
    Msg::new("gui.fld.spline_lite.form_int.root.opt.auto", "自动（按表达式反解）", "Auto (back-solved from the expression)"),
    Msg::new("gui.fld.spline_lite.form_int.root.opt.fillet", "圆齿根", "Fillet root"),
    Msg::new("gui.fld.spline_lite.form_int.root.opt.flat", "平齿根", "Flat root"),
    Msg::new("gui.fld.spline_lite.form_int.root.title", "平/圆；auto = 由表达式 DA/DF 反解，再退到按 αD 默认", "Flat/fillet; auto = back-solved from DA/DF in the expression, otherwise the αD default"),


    // ── GUI 元数据批 ⑥：options/preview 内联 note + NF 直径公差读数注 ──
    // ★ zh 一律由脚本从 `git HEAD` 自动提取（`--notes-rust` / `--notes-verify` 防漂移）；
    //   英译人工；协议记号（ISO 286 / NF E22-141 p28 / ATTDEF / 符号）两语原样。
    Msg::new("gui.ansi.preview.missing_note", "ANSI B92.1 的配合/公差（Table 4/5）、量棒检验、公法线跨测表本仓未收 —— 如实标缺，不臆造。", "ANSI B92.1 fits/tolerances (Table 4/5), pin inspection and the base tangent/span tables are not collected in this repo — shown as missing, with no guesswork."),
    Msg::new("gui.card_lite.options.missing_note", "精简版：只列基本参数 + 主要测量量，不含任何公差（上/下偏差）列；完整明细请用对应的完整卡。", "Lite version: basic parameters + main measurement quantities only, no tolerance (upper/lower deviation) columns; use the corresponding full card for the complete detail."),
    Msg::new("gui.din.options.note", "版面照 DIN 5480-1:2006 Bild 6（**内外拆成两张单栏卡**；本卡只画本侧 13 行，整表 INSERT 缩放 0.17）；文字样式一律 OCSM_GB；Table 7 上段数值为 2026-09-26 双源 OCR + 逐格复核入库（assets/din5480_1_table7_dev.csv）。", "Layout follows DIN 5480-1:2006 Bild 6 (**internal and external split into two single-column cards**; this card draws only its own side, 13 rows, the whole-table INSERT scaled 0.17); the text style is always OCSM_GB; the upper Table 7 values were entered from a two-source OCR on 2026-09-26 with cell-by-cell review (assets/din5480_1_table7_dev.csv)."),
    Msg::new("gui.din.preview.note", "DIN 5480-1:2006 Bild 6 原示例（N120×3×38×9H / W120×3×38×8f）—— 整表照标准原印值", "DIN 5480-1:2006 Bild 6 original example (N120×3×38×9H / W120×3×38×8f) — the whole table follows the values printed in the standard"),
    Msg::new("gui.gear.options.missing_note", "GB/T 10095-88 的 Fr/FW/ff/fpt/Fβ 与中心距极限偏差本仓未收；内齿轮/斜齿轮的公法线口径本仓未收 —— 这些格子一律显示「—」，不臆造。", "The GB/T 10095-88 Fr/FW/ff/fpt/Fβ and centre-distance limit deviations are not collected in this repo; nor is the base tangent definition for internal/helical gears — those cells always show “—”, with no guesswork."),
    Msg::new("gui.gear.options.note", "取值复用既有齿轮引擎（gear.rs）：表达式反解后 ha*/c* 由 DA/DF 反解，交回 GearParams 复核 da/df 闭环；公法线走 GearParams::span_measurement()。", "Values reuse the existing gear engine (gear.rs): after back-solving the expression, ha*/c* are back-solved from DA/DF and handed back to GearParams to close the da/df loop; the base tangent goes through GearParams::span_measurement()."),
    Msg::new("gui.gear.preview.missing_note", "本仓未收 GB/T 10095-88（Fr/FW/ff/fpt/Fβ）与中心距极限偏差；内齿轮/斜齿轮无公法线口径 —— 如实标缺，不臆造。", "GB/T 10095-88 (Fr/FW/ff/fpt/Fβ) and the centre-distance limit deviations are not collected in this repo; internal/helical gears have no base tangent definition — shown as missing, with no guesswork."),
    Msg::new("gui.lite.options.missing_note", "精简版卡片：固定只列基本参数与主要测量量，不含任何公差（上/下偏差）列；等级/配合只用于测量量计算。需要公差明细请用「GB 花键参数表」。", "Lite card: basic parameters and main measurement quantities only, never any tolerance (upper/lower deviation) columns; the grade/fit is used only to compute the measured quantities. Use “GB spline parameter table” for the tolerance detail."),
    Msg::new("gui.nf.options.missing_note", "公差口径（NF E22-141）：大径上/下差 = ISO 286 R7（p28 §4）；小径上/下差 = ISO 286 H7（p28 §6，参考）；跨棒距上/下差 = p29 检查尺寸的公差值里**内花键 E** 的偏差（µm→mm）。p29 的 xm 内花键 + 所选配合的外花键 E/xm 偏差在预览读数里列出；(m,A) 不在 p29 或 ISO 档缺时对应格显示「—」，不外推；元素（ATTDEF）始终存在，可在 CAD 里改写。", "Tolerance definitions (NF E22-141): major dia. upper/lower deviations = ISO 286 R7 (p28 §4); minor dia. upper/lower deviations = ISO 286 H7 (p28 §6, reference); measurement over pins upper/lower deviations = the **internal spline E** deviations among the p29 inspection-dimension tolerances (µm→mm). The p29 xm internal spline and the mating external spline E/xm deviations for the selected fit are listed in the preview readout; when (m,A) is not in p29 or the ISO range is missing the corresponding cell shows “—”, with no extrapolation; the elements (ATTDEFs) always exist and can be edited in CAD."),
    Msg::new("gui.nf.options.note", "版面照外花键参数表NF.dxf 同构镜像（13 行 × 2 列，66 线 + 13 标签 + 18 属性）；文字样式一律 OCSM_GB；模板末行标签出框已归位；公差按 p28 直径公差 + p29 E 偏差取值（配合类别只影响预览读数）。", "Layout mirrors the external-spline template (13 rows × 2 columns, 66 lines + 13 labels + 18 attributes); the text style is always OCSM_GB; the template's last-row label has been put back inside the frame; tolerances take the p28 diameter tolerances + p29 E deviations (the fit class only affects the preview readout)."),
    Msg::new("gui.nf.preview.missing_note", "公差口径：大径 R7 / 小径 H7（p28，数值 ISO 286）、跨棒距 = p29 内花键 E 偏差；(m,A) 不在 p29 或 ISO 档缺 → 对应格「—」，不外推；V/V1/G/G1 与 ri 只取 p23–p25 / p22 表值，表外显示「—」。", "Tolerance definitions: major dia. R7 / minor dia. H7 (p28, values from ISO 286), measurement over pins = p29 internal spline E deviation; when (m,A) is not in p29 or the ISO range is missing the corresponding cell shows “—”, with no extrapolation; V/V1/G/G1 and ri only take values from the p23–p25 / p22 tables and show “—” outside them."),
    Msg::new("gui.nf.tol.major_none", "—（ISO 286 无 Az={d} 的 R7 档）", "— (no ISO 286 R7 range for Az={d})"),
    Msg::new("gui.nf.tol.major_note", "ISO 286 R7（NF E22-141 p28 §4：拉削/外径定心的内花键大径公差同为 R7）", "ISO 286 R7 (NF E22-141 p28 §4: the internal spline major dia. tolerance for broaching/major-dia. centering is also R7)"),
    Msg::new("gui.nf.tol.minor_none", "—（ISO 286 无 D={d} 的 H7 档）", "— (no ISO 286 H7 range for D={d})"),
    Msg::new("gui.nf.tol.minor_note", "ISO 286 H7（NF E22-141 p28 §6：内花键小径 D 公差 H7，参考）", "ISO 286 H7 (NF E22-141 p28 §6: the internal spline minor dia. D tolerance is H7, reference)"),
    Msg::new("gui.nf_ext.options.missing_note", "公差口径（模板实测 + ISO 286 + NF E22-141 p29）：大径上/下差 = ISO 286 h12（模板示例 Dee=298.5 实测 0/−0.460）；小径上/下差 = ISO 286 H7（模板示例 Die=282 实测 +0.052/0）；公法线上/下差 = p29 检查尺寸的公差值里**外花键 E** 的偏差（µm→mm，按所选配合）。K/W 取 p23–p25 检查表；(m,A) 表外或 ISO 档缺 → 对应格「—」，不外推；元素（ATTDEF）始终存在，可在 CAD 里改写。", "Tolerance definitions (template measurement + ISO 286 + NF E22-141 p29): major dia. upper/lower deviations = ISO 286 h12 (template sample Dee=298.5 measured 0/−0.460); minor dia. upper/lower deviations = ISO 286 H7 (template sample Die=282 measured +0.052/0); base tangent upper/lower deviations = the **external spline E** deviations among the p29 inspection-dimension tolerances (µm→mm, per the selected fit). K/W come from the p23–p25 inspection table; when (m,A) is outside it or the ISO range is missing → the corresponding cell shows “—”, with no extrapolation; the elements (ATTDEFs) always exist and can be edited in CAD."),
    Msg::new("gui.nf_ext.options.note", "版面照模板 `外花键参数表NF.dxf` 原版（13 行 × 2 列，66 线 + 13 标签 + 18 属性）；文字样式一律 OCSM_GB；模板末行标签出框已归位；符号用 NF 原文（Dee/Die/K/W，不用 GB 符号）。", "Layout follows the original external-spline template (13 rows × 2 columns, 66 lines + 13 labels + 18 attributes); the text style is always OCSM_GB; the template's last-row label has been put back inside the frame; the symbols use the NF originals (Dee/Die/K/W, not the GB symbols)."),
    Msg::new("gui.nf_ext.preview.missing_note", "公差口径：大径 h12 / 小径 H7（ISO 286，模板实测） + 公法线 = p29 外花键 E 偏差（按配合）；(m,A) 不在 p29 或 ISO 档缺 → 对应格「—」，不外推；K/W 只取 p23–p25 检查表值，表外显示「—」。", "Tolerance definitions: major dia. h12 / minor dia. H7 (ISO 286, template measurement) + base tangent = p29 external spline E deviation (per the fit); when (m,A) is not in p29 or the ISO range is missing the corresponding cell shows “—”, with no extrapolation; K/W only take values from the p23–p25 inspection table and show “—” outside it."),
    Msg::new("gui.nf_ext.tol.major_none", "—（ISO 286 无 Dee={d} 的 h12 档）", "— (no ISO 286 h12 range for Dee={d})"),
    Msg::new("gui.nf_ext.tol.major_note", "ISO 286 h12（外花键齿顶圆非功能直径；Ø298.5 → 0/−0.520。模板该格实测 0/−0.460 为模板另填，卡按 ISO 286 取值）", "ISO 286 h12 (external spline tip circle, non-functional diameter; Ø298.5 → 0/−0.520. The template row measured 0/−0.460, a template-specific entry; the card takes the ISO 286 value)"),
    Msg::new("gui.nf_ext.tol.minor_none", "—（ISO 286 无 Die={d} 的 H7 档）", "— (no ISO 286 H7 range for Die={d})"),
    Msg::new("gui.nf_ext.tol.minor_note", "ISO 286 H7（模板示例 Die=282 实测 +0.052/0 与 H7 一致；与 NF 内卡小径同口径）", "ISO 286 H7 (the template sample Die=282 measured +0.052/0, consistent with H7; same definition as the NF internal card minor dia.)"),


    Msg::new("gui.nf.centering.flank", "齿面定心（Az=A+0.3m）", "Flank centering (Az=A+0.3m)"),
    Msg::new("gui.nf.centering.outer", "外径定心（Az=A）", "Major dia. centering (Az=A)"),
    Msg::new("gui.nf_ext.centering.flank", "齿面定心（Dee=A−0.2m，模板）", "Flank centering (Dee=A−0.2m, template)"),
    Msg::new("gui.nf_ext.centering.outer", "外径定心（Dee=A）", "Major dia. centering (Dee=A)"),
    Msg::new("card.ansi.type.a30flat_side", "30°平齿根齿侧配合", "FLAT ROOT SIDE FIT"),
    Msg::new("card.ansi.type.b30flat_major", "30°平齿根外径配合", "FLAT ROOT MAJOR DIA FIT"),
    Msg::new("card.ansi.type.c30fillet_side", "30°圆齿根齿侧配合", "FILLET ROOT SIDE FIT"),
    Msg::new("card.ansi.type.d375fillet_side", "37.5°圆齿根齿侧配合", "FILLET ROOT SIDE FIT"),
    Msg::new("card.ansi.type.e45fillet_side", "45°圆齿根齿侧配合", "FILLET ROOT SIDE FIT"),


    // ── 阶段 5：智能卡片计算书（`card_report.rs` 正文/表头/来源）──
    // ★ zh 由 `i18n_检查/card_report_i18n_gen.py` 从 `git HEAD` 自动提取（`--verify` 防漂移）；
    //   英译人工；公式体/标准号/符号/键名（`spline_tol::compute()`、Dp/Md、Table 26）两语原样。
    Msg::new("gui.report.items.head", "\n\n| 项 | 公式/口径 | 值 | 依据来源 |\n|---|---|---|---|\n", "\\n\\n| Item | Formula/definition | Value | Source |\\n|---|---|---|---|\\n"),
    Msg::new("gui.report.sources.card_type", "- 卡类型：{name}（`{id}`）\n", "- Card type: {name} (`{id}`)\\n"),
    Msg::new("gui.report.sources.card_def", "- 卡片口径：{note}\n", "- Card definition: {note}\\n"),
    Msg::new("gui.report.sources.missing_def", "- 缺项说明：{note}\n", "- Missing-item note: {note}\\n"),
    Msg::new("gui.report.sources.missing_list", "- 本卡标缺项：{list}\n", "- Items marked missing on this card: {list}\\n"),
    Msg::new("gui.report.sources.list_sep", "、", ", "),
    Msg::new("gui.report.sources.same_source", "- 同源：本报告与卡片（OCSMCARD）取同一份计算（`/api/card_preview` 同一入口）；报告数值与卡片 ATTRIB 逐项一致，未二次手算。\n", "- Same source: this report and the card (OCSMCARD) use the same computation (the same entry point as `/api/card_preview`); every value matches the card ATTRIB item by item, with no second hand calculation.\\n"),
    Msg::new("gui.report.gb.side", "- 方向：**{v}**\n", "- Direction: **{v}**\\n"),
    Msg::new("gui.report.gb.grade_fit", "- 等级/配合：**{v}**\n", "- Grade/fit: **{v}**\\n"),
    Msg::new("gui.report.gb.expr", "- 表达式：`{expr}`\n", "- Expression: `{expr}`\\n"),
    Msg::new("gui.report.gb.inputs_head", "\n## 1. 输入与反解\n\n| 量 | 值 |\n|---|---|\n", "\\n## 1. Inputs and back-solved values\\n\\n| Quantity | Value |\\n|---|---|\\n"),
    Msg::new("gui.report.gb.inputs_rows", "| 模数 m | {m} mm |\n| 齿数 z | {z} |\n| 压力角 αD | {alpha}° |\n| 变位系数 x | {x} |\n| 表达式大径 Da | {da} mm |\n| 表达式小径 Df | {df} mm |\n| 齿根形式 | {root}（{root_source}） |\n\n", "| Module m | {m} mm |\\n| Teeth z | {z} |\\n| Pressure angle αD | {alpha}° |\\n| Profile shift x | {x} |\\n| Expression major dia. Da | {da} mm |\\n| Expression minor dia. Df | {df} mm |\\n| Root form | {root} ({root_source}) |\\n\\n"),
    Msg::new("gui.report.gb.items_head", "## 2. 参数表（21 项；与卡片 ATTDEF 同一份 `spline_tol::compute()`）", "## 2. Parameter table (21 items; same `spline_tol::compute()` as the card ATTDEFs)"),
    Msg::new("gui.report.gb.rimin_head", "## 2.1 R_imin 口径（卡面按表 26；计算按图 2 系数）\n\n", "## 2.1 R_imin definition (the card follows table 26; the calculation uses the figure 2 coefficient)\\n\\n"),
    Msg::new("gui.report.gb.rimin_table_hit", "- 表 26（GB/T 3478.1-2008 书页 50）m={m}、{alpha}°、{root_cn} 档表值 = **{v}** mm；卡面显示同值。\n", "- Table 26 (GB/T 3478.1-2008, page 50) table value for m={m}, {alpha}°, {root_cn} = **{v}** mm; the card shows the same value.\\n"),
    Msg::new("gui.report.gb.rimin_table_miss", "- 表 26 未列值（原表印「—」；m={m}、{alpha}°、{root_cn}）：卡面按表 26 显示「—」，不外推。\n", "- No value is listed in table 26 (the printed table shows “—”; m={m}, {alpha}°, {root_cn}): the card follows table 26 and shows “—”, with no extrapolation.\\n"),
    Msg::new("gui.report.gb.rimin_calc", "- 计算口径（图 2 系数式；注明性质）：R_imin = {coef}·m = **{calc}** mm —— 仅供内部/报告对照，不直接上卡面。\n\n", "- Calculation definition (figure 2 coefficient formula; noted as such): R_imin = {coef}·m = **{calc}** mm — for internal/report cross-check only, never shown directly on the card.\\n\\n"),
    Msg::new("gui.report.gb.pin_head", "## 3. 量棒/跨棒距口径\n\n", "## 3. Pin / measurement-over-pins definitions\\n\\n"),
    Msg::new("gui.report.gb.pin_formula", "- 量棒直径：{v}\n", "- Pin diameter: {v}\\n"),
    Msg::new("gui.report.gb.md_formula", "- 跨棒距：{v}\n", "- Measurement over pins: {v}\\n"),
    Msg::new("gui.report.gb.pin_source", "- 来源：{v}\n", "- Source: {v}\\n"),
    Msg::new("gui.report.gb.pin_values", "- 取值：Dp={dp}、Md={md}（同一次 `compute()`；卡片 ATTRIB 同值）。\n\n", "- Values: Dp={dp}, Md={md} (the same `compute()`; the card ATTRIB carries the same values).\\n\\n"),
    Msg::new("gui.report.lite.scope", "- 口径：精简版只列基本参数 + 主要测量量，**不含任何公差（上/下偏差）列**；\n  公差等级/配合仅用于测量量（Dp/Md 或 Kn/Wn）计算。\n\n", "- Definition: the lite version lists basic parameters + main measurement quantities only, **with no tolerance (upper/lower deviation) columns**;\\n  the grade/fit is used only to compute the measured quantities (Dp/Md or Kn/Wn).\\n\\n"),
    Msg::new("gui.report.lite.items_head", "## 1. 精简项（9 项；与卡片 ATTDEF 同一份 `spline_tol::compute()`）", "## 1. Lite items (9 items; same `spline_tol::compute()` as the card ATTDEFs)"),
    Msg::new("gui.report.card_lite.card", "- 卡：**{c}**（完整卡口径同源）\n", "- Card: **{c}** (same definition as the full card)\\n"),
    Msg::new("gui.report.card_lite.scope", "- 口径：精简版只列基本参数 + 主要测量量，**不含任何公差（上/下偏差）列**；\n取值/校验全部投影自对应完整卡的同一次计算（查不到 →「—」，不外推）。\n\n", "- Definition: the lite version lists basic parameters + main measurement quantities only, **with no tolerance (upper/lower deviation) columns**;\\nall values/validation are projected from the same computation as the corresponding full card (anything not found → “—”, with no extrapolation).\\n\\n"),
    Msg::new("gui.report.card_lite.items_head", "## 1. 精简项（{n} 项；与完整卡同一份计算）", "## 1. Lite items ({n} items; same computation as the full card)"),
    Msg::new("gui.report.append.gear", "## 附：卡片项（与卡片 ATTDEF 同一份 `GearParams`）", "## Appendix: card items (same `GearParams` as the card ATTDEFs)"),
    Msg::new("gui.report.append.invol", "## 附：卡片项（与卡片 ATTDEF 同一份 `InvolParams`）", "## Appendix: card items (same `InvolParams` as the card ATTDEFs)"),
    Msg::new("gui.report.append.nf", "## 附：卡片项（与卡片 ATTDEF 同一份 NF 查表）", "## Appendix: card items (same NF table lookup as the card ATTDEFs)"),
    Msg::new("gui.report.append.din", "## 附：卡片项（与卡片 ATTDEF 同一份 DIN 查表）", "## Appendix: card items (same DIN table lookup as the card ATTDEFs)"),
    Msg::new("gui.report.err.serialize", "计算书：请求模型序列化失败：{e}", "Report: failed to serialize the request model: {e}"),
    Msg::new("gui.report.err.preview_json", "计算书：卡片预览 JSON 解析失败：{e}", "Report: failed to parse the card preview JSON: {e}"),
    Msg::new("gui.report.err.model", "{card}：请求字段无效：{e}", "{card}: invalid request fields: {e}"),
    Msg::new("gui.report.err.not_json", "计算书：请求不是 JSON：{e}", "Report: the request is not JSON: {e}"),
    Msg::new("gui.report.err.card_unknown", "计算书：不认识的卡类型「{card_id}」（卡类型表见 /api/spline_options 的 card_types）", "Report: unknown card type “{card_id}” (see `card_types` of /api/spline_options)"),


    Msg::new("card.gb.side.ext", "外花键", "external spline"),
    Msg::new("card.gb.side.int", "内花键", "internal spline"),

    // ── 卡面标签：NF E22-141 花键参数表（内/外；`nf_table.rs` / `nf_ext_table.rs`）──
    // ★ 本卡是「用户自定画法」同构镜像（NF 标准里没有参数表版面，§29 调研）⇒ 标签是中文自定
    //   术语；英译贴工程惯例（量棒/跨棒距/公法线沿用 GB 卡既有译法；定心方式按 DIN/ISO 的
    //   centering 口径），拿不准的两条（基准尺寸 / 加工方法）见 §33 报告供用户校。
    // * **符号/标准号/值/块名/ATTDEF tag 原样**（`Az`/`Dee`/`D`/`Die`/`V`/`G`/`K`/`W`/`Do`/`m`/`z`/`a`）。
    // * 标签里的符号**不进表**：卡面 = `t(key)` + `「 」+ symbol`（§31/§32「符号与译名分置」形态）。
    Msg::new("card.nf.title.int", "内花键参数表", "Internal Spline Data"),
    Msg::new("card.nf.title.ext", "外花键参数表", "External Spline Data"),
    Msg::new("card.nf.label.standard", "执行标准", "Standard"),
    Msg::new("card.nf.label.centering", "定心方式", "Centering"),
    Msg::new("card.nf.label.module", "模数", "Module"),
    Msg::new("card.nf.label.teeth", "齿数", "Number of teeth"),
    Msg::new("card.nf.label.alpha", "压力角", "Pressure angle"),
    Msg::new("card.nf.label.root_form", "齿根样式", "Root form"),
    Msg::new("card.nf.label.machining", "加工方法", "Machining method"),
    Msg::new("card.nf.label.major_dia", "大径", "Major diameter"),
    Msg::new("card.nf.label.minor_dia", "小径", "Minor diameter"),
    Msg::new("card.nf.label.base_size", "基准尺寸", "Basic size"),
    Msg::new("card.nf.label.pin_dia", "量棒直径", "Pin diameter"),
    Msg::new("card.nf.label.over_pins", "跨棒距", "Measurement over pins"),
    Msg::new("card.nf.label.span_teeth", "跨测齿数", "Span teeth"),
    Msg::new("card.nf.label.base_tangent", "公法线", "Base tangent length"),

    // ── 卡面取值（枚举显示值随语言；NF E22-141 p04/p07/p18 口径）──
    Msg::new("card.nf.centering.outer", "外径定心", "Major dia. centering"),
    Msg::new("card.nf.centering.flank", "齿面定心", "Flank centering"),
    Msg::new("card.nf.root.flat", "平齿根", "Flat root"),
    Msg::new("card.nf.root.fillet", "圆齿根", "Fillet root"),
    Msg::new("card.nf.machining.broach", "拉削", "Broaching"),
    Msg::new("card.nf.machining.hob", "滚齿", "Hobbing"),

    // ── 卡面标签：NF 花键参数表（精简两卡；`card_lite.rs` NF 族）──
    // 行标签复用上面 `card.nf.label.*`（精简卡 zh 形态与全卡逐字相同），只有标题另立。
    Msg::new("card.nf.lite.title.int", "NF 内花键参数表（精简）", "NF Internal Spline Data (Lite)"),
    Msg::new("card.nf.lite.title.ext", "NF 外花键参数表（精简）", "NF External Spline Data (Lite)"),

    // ── 卡面标签：DIN 5480 花键参数表（内/外 + 精简两卡；`din_table.rs` / `card_lite.rs` DIN 族）──
    // ★ 本卡是「照 DIN 5480-1:2006-03 §9.1 Datenfeld / Bild 6 **自定版面**」（标准里没有参数表模板）
    //   ⇒ 英译贴**德英工程惯例**（DIN 5480-1/-2 英文符号表 + ISO 21771/KISSsoft 用词）：
    //   `Nabe/Welle` → `Hub/Shaft`（标准第 8 章代号字母仍是 `N`/`W`）、`Fußformkreis` →
    //   `Root form diameter`、`Lückenweite/Zahndicke` → `Space width/Tooth thickness`。
    //   ★ 拿不准的写法（行 1 的 Hub/Shaft、`Measuring circle D_M`、`Measurement over pins M1/M2`、
    //   标题 `… Spline Data` vs `… Parameter Table`）见 §34 报告供用户校。
    // * **原样不译**：标记号（`N120×3×38×9H`/`W120×3×38×8f`）、符号（`z`/`m`/`α`/`d_f2`/`d_Ff2`/
    //   `d_a1`/`d_a2`/`d_f1`/`e_max`/`e_vmin`/`s_vmax`/`D_M`/`M1_max`/`M2_min` …）、值、
    //   ATTDEF tag（`N标记`…）、块名（`OCSM_DINTABLE_DIN_*`/`OCSM_LITE_DIN_*`）。
    // * 标签里的符号**不进表**：卡面 = `t(key)` + 「空格 + symbol」（§31/§32「符号与译名分置」形态）。
    Msg::new("card.din.title.int", "DIN 5480 内花键参数表", "DIN 5480 Internal Spline Data"),
    Msg::new("card.din.title.ext", "DIN 5480 外花键参数表", "DIN 5480 External Spline Data"),
    Msg::new("card.din.label.nabe", "Nabe DIN 5480", "Hub DIN 5480"),
    Msg::new("card.din.label.welle", "Welle DIN 5480", "Shaft DIN 5480"),
    Msg::new("card.din.label.teeth", "齿数", "Number of teeth"),
    Msg::new("card.din.label.module", "模数", "Module"),
    Msg::new("card.din.label.alpha", "压力角", "Pressure angle"),
    Msg::new("card.din.label.root_dia", "齿根圆", "Root diameter"),
    Msg::new("card.din.label.tip_dia", "齿顶圆", "Tip diameter"),
    Msg::new("card.din.label.root_form_dia", "齿根成形圆", "Root form diameter"),
    Msg::new("card.din.label.space_width_max", "槽宽 max.", "Space width max."),
    Msg::new("card.din.label.space_width_min", "槽宽 min.", "Space width min."),
    Msg::new("card.din.label.space_width_eff", "槽宽 eff.", "Space width eff."),
    Msg::new("card.din.label.thickness_eff_max", "齿厚 eff.", "Tooth thickness eff."),
    Msg::new("card.din.label.thickness_max", "齿厚 max.", "Tooth thickness max."),
    Msg::new("card.din.label.thickness_min", "齿厚 min.", "Tooth thickness min."),
    Msg::new("card.din.label.measuring_circle", "量圆", "Measuring circle"),
    Msg::new("card.din.label.over_pins", "量圆距", "Measurement over pins"),
    // 精简两卡（`card_lite.rs` DIN 族：行标签复用上面 `card.din.label.*`，只有标记行/标题另立）。
    Msg::new("card.din.lite.title.int", "DIN 内花键参数表（精简）", "DIN Internal Spline Data (Lite)"),
    Msg::new("card.din.lite.title.ext", "DIN 外花键参数表（精简）", "DIN External Spline Data (Lite)"),
    Msg::new("card.din.lite.label.desig", "标记", "Designation"),

    // ── 命令报错：五卡共用的九字段表达式解析（spline_table::parse_gear_expr）──
    Msg::new(
        "cmd.card.err.expr_missing",
        "{card}：缺九字段齿形表达式（形如 `SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30`；轴/齿轮生成器 GUI 可直接复制）",
        "{card}: missing the nine-field tooth-profile expression (like `SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30`; copy it from the shaft/gear generator GUI)",
    ),
    Msg::new(
        "cmd.card.err.expr_parse",
        "{card}：齿形表达式无法解析：{e}",
        "{card}: cannot parse the tooth-profile expression: {e}",
    ),
    Msg::new(
        "cmd.card.err.expr_multi",
        "{card}：表达式应只有一段齿形（收到 {n} 段）——请只粘生成器复制的那一行齿形表达式",
        "{card}: the expression must contain exactly one tooth-profile segment (got {n}) — paste only the single line copied from the generator",
    ),
    Msg::new(
        "cmd.card.err.expr_no_gear",
        "{card}：表达式里没有齿形段（应以 `GEAR`/`SPLINE` + M/Z/ALPHA… 开头）",
        "{card}: the expression has no tooth-profile segment (it should start with `GEAR`/`SPLINE` + M/Z/ALPHA…)",
    ),

    // ── 命令输出/报错：GB 花键参数表命令行（spline_table.rs）──
    Msg::new(
        "cmd.spline.err.values_tag_missing",
        "花键参数表：模板 tag「{tag}」没有取值映射",
        "Spline table: template tag \"{tag}\" has no value mapping",
    ),
    Msg::new(
        "cmd.spline.err.std_missing_system",
        "花键参数表：std 缺少体系（本期只有 GB）",
        "Spline table: std is missing a system (only GB for now)",
    ),
    Msg::new(
        "cmd.spline.err.side_first",
        "花键参数表：第一个参数应为「内」或「外」（收到「{other}」）。\n{usage}",
        "Spline table: the first argument must be \"内\" or \"外\"/int/ext (got \"{other}\").\n{usage}",
    ),
    Msg::new(
        "cmd.spline.err.internal_h_mark",
        "花键参数表：内花键是基孔制 H（收到「{mark}」；写法如 `内 6H`）",
        "Spline table: an internal spline is always H (hole-basis); got \"{mark}\" (write e.g. `内 6H`)",
    ),
    Msg::new(
        "cmd.spline.err.kind_mismatch",
        "花键参数表：表达式 KIND 写的是 {kind}，与卡片方向「{want}」不一致",
        "Spline table: expression KIND is {kind}, inconsistent with the card direction \"{want}\"",
    ),
    Msg::new(
        "cmd.spline.err.need_value",
        "花键参数表：{what} 缺少数值/选项",
        "Spline table: {what} is missing a value/option",
    ),
    Msg::new("cmd.spline.err.dp_num", "dp={v} 不是数字：{e}", "dp={v} is not a number: {e}"),
    Msg::new(
        "cmd.spline.err.dp_parse_positive",
        "花键参数表：dp={dp} 必须 >0",
        "Spline table: dp={dp} must be > 0",
    ),
    Msg::new(
        "cmd.spline.err.root_invalid",
        "花键参数表：齿根形式「{other}」非法（只有 平/圆）",
        "Spline table: invalid root form \"{other}\" (only 平/flat or 圆/fillet)",
    ),
    Msg::new(
        "cmd.spline.err.at_format",
        "花键参数表：at「{v}」应为 `x,y`",
        "Spline table: at \"{v}\" must be `x,y`",
    ),
    Msg::new("cmd.spline.err.at_x", "at x={x} 不是数字：{e}", "at x={x} is not a number: {e}"),
    Msg::new("cmd.spline.err.at_y", "at y={y} 不是数字：{e}", "at y={y} is not a number: {e}"),
    Msg::new("cmd.spline.err.rot", "rot={v} 不是数字：{e}", "rot={v} is not a number: {e}"),
    Msg::new(
        "cmd.spline.err.unknown_param",
        "花键参数表：不认识的参数「{other}」。\n{usage}",
        "Spline table: unrecognized argument \"{other}\".\n{usage}",
    ),
    Msg::new(
        "cmd.spline.err.mark_format",
        "花键参数表：「{mark}」应形如 `6H` / `5f`",
        "Spline table: \"{mark}\" must look like `6H` / `5f`",
    ),
    Msg::new(
        "cmd.spline.err.grade_num",
        "花键参数表：等级「{g}」不是数字：{e}",
        "Spline table: grade \"{g}\" is not a number: {e}",
    ),
    Msg::new(
        "cmd.spline.err.grade_invalid_card",
        "花键参数表：公差等级「{grade}」非法（GB/T 3478.1 只有 4/5/6/7）",
        "Spline table: invalid tolerance grade \"{grade}\" (GB/T 3478.1 only has 4/5/6/7)",
    ),
    Msg::new(
        "cmd.spline.err.mark_no_fit",
        "花键参数表：「{mark}」缺配合类别（内如 `6H`、外如 `5f`）",
        "Spline table: \"{mark}\" is missing the fit class (e.g. `6H` for internal, `5f` for external)",
    ),
    Msg::new(
        "cmd.spline.usage",
        "智能卡片「花键参数表」用法：`OCSMCARD 花键参数表 [std GB] 内 6H <九字段表达式> [dp 4.5] [root 平|圆] [at x,y] [rot 度]`；外花键把 `内 6H` 换成 `外 5f`（表达式 KIND 用 EX）。表达式形如 `SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30`（轴/齿轮生成器 GUI 可直接复制）。等级 4/5/6/7；配合内 H、外 d/e/f/h/js/k；不填 dp = 按标准 R40 自动选；不填 root = 由表达式 DA/DF 反解（30° 可分辨平/圆），再退到按 αD 默认。",
        "Smart card \"Spline table\" usage: `OCSMCARD 花键参数表 [std GB] 内 6H <nine-field expression> [dp 4.5] [root 平|圆] [at x,y] [rot deg]`; for external splines replace `内 6H` with `外 5f` (use KIND EX in the expression). Example expression: `SPLINE IN M3 Z20 ALPHA30 X0 DA65.4 DF57.3436 BETA0 H30` (copy it from the shaft/gear generator GUI). Grades 4/5/6/7; internal fit H, external d/e/f/h/js/k; omit dp = auto-pick per standard R40; omit root = resolve from DA/DF in the expression (30° distinguishes flat/fillet), then fall back to the αD default.",
    ),

    // ── 命令输出/报错：齿轮参数表命令行（gear_table.rs）──
    Msg::new(
        "cmd.geartab.err.m_invalid",
        "齿轮参数表：模数 m={m} 非法",
        "Gear table: invalid module m={m}",
    ),
    Msg::new(
        "cmd.geartab.err.missing_da",
        "齿轮参数表：表达式缺 `DA`（大径）—— 齿顶高系数要从它反解（轴/齿轮生成器 GUI 可复制完整九字段）",
        "Gear table: the expression is missing `DA` (major diameter) — the addendum coefficient is back-solved from it (copy the full nine fields from the shaft/gear generator GUI)",
    ),
    Msg::new(
        "cmd.geartab.err.missing_df",
        "齿轮参数表：表达式缺 `DF`（小径）—— 全齿高与顶隙系数要从它反解",
        "Gear table: the expression is missing `DF` (minor diameter) — the whole depth and clearance coefficient are back-solved from it",
    ),
    Msg::new(
        "cmd.geartab.err.params_invalid",
        "齿轮参数表：表达式反解的齿形参数不合法：{e}",
        "Gear table: the tooth-profile parameters back-solved from the expression are invalid: {e}",
    ),
    Msg::new(
        "cmd.geartab.err.closed_loop",
        "齿轮参数表：表达式 DA/DF 与 M/Z/ALPHA/X 不自洽（反解 ha*={ha}、c*={c} 后引擎给 Da={da}、Df={df}，表达式写 Da={da_in}、Df={df_in}）",
        "Gear table: expression DA/DF is inconsistent with M/Z/ALPHA/X (after back-solving ha*={ha}, c*={c}, the engine gives Da={da}, Df={df}, but the expression says Da={da_in}, Df={df_in})",
    ),
    Msg::new(
        "cmd.geartab.err.tag_order",
        "齿轮参数表：取值映射顺序与模板属性不一致：{got} != {want}",
        "Gear table: value-mapping order differs from the template attributes: {got} != {want}",
    ),
    Msg::new(
        "cmd.geartab.err.mate_z",
        "齿轮参数表：配对齿轮齿数 z₂={z} 超出范围（2–1000）",
        "Gear table: mating-gear tooth count z₂={z} is out of range (2–1000)",
    ),
    Msg::new(
        "cmd.geartab.err.center",
        "齿轮参数表：中心距 center={a} 必须是正数",
        "Gear table: center distance center={a} must be positive",
    ),
    Msg::new(
        "cmd.geartab.err.need",
        "齿轮参数表：{what} 缺少数值/字符串",
        "Gear table: {what} is missing a value/string",
    ),
    Msg::new(
        "cmd.geartab.err.mate_num",
        "齿轮参数表：mate={v} 不是整数：{e}",
        "Gear table: mate={v} is not an integer: {e}",
    ),
    Msg::new(
        "cmd.geartab.err.center_num",
        "齿轮参数表：center={v} 不是数字：{e}",
        "Gear table: center={v} is not a number: {e}",
    ),
    Msg::new(
        "cmd.geartab.err.at_format",
        "齿轮参数表：at「{v}」应为 `x,y`",
        "Gear table: at \"{v}\" must be `x,y`",
    ),
    Msg::new("cmd.geartab.err.at_x", "齿轮参数表：at x={x} 不是数字：{e}", "Gear table: at x={x} is not a number: {e}"),
    Msg::new("cmd.geartab.err.at_y", "齿轮参数表：at y={y} 不是数字：{e}", "Gear table: at y={y} is not a number: {e}"),
    Msg::new("cmd.geartab.err.rot", "齿轮参数表：rot={v} 不是数字：{e}", "Gear table: rot={v} is not a number: {e}"),
    Msg::new(
        "cmd.geartab.err.unknown_param",
        "齿轮参数表：不认识的参数「{other}」。\n{usage}",
        "Gear table: unrecognized argument \"{other}\".\n{usage}",
    ),
    Msg::new(
        "cmd.geartab.usage",
        "智能卡片「齿轮参数表」用法：`OCSMCARD 齿轮参数表 <九字段表达式> [mate z₂] [dwg 图号] [grade 精度等级] [center a] [at x,y] [rot 度]`。表达式形如 `GEAR EX M3 Z20 ALPHA20 X0 DA66 DF52.5 BETA0 H30`（轴/齿轮生成器 GUI 可直接复制）；DA/DF 用来反解 ha*/c*，内/外齿由表达式 KIND 决定。GB/T 10095-88 公差（Fr/FW/ff/fpt/Fβ）与中心距极限偏差本仓未收 —— 卡片显示「—」，不臆造。",
        "Smart card \"Gear table\" usage: `OCSMCARD 齿轮参数表 <nine-field expression> [mate z₂] [dwg drawing-no] [grade precision-grade] [center a] [at x,y] [rot deg]`. Example expression: `GEAR EX M3 Z20 ALPHA20 X0 DA66 DF52.5 BETA0 H30` (copy it from the shaft/gear generator GUI); DA/DF are used to back-solve ha*/c*, and internal/external teeth are determined by KIND in the expression. GB/T 10095-88 tolerances (Fr/FW/ff/fpt/Fβ) and center-distance limit deviations are not collected in this repo — the card shows \"—\" instead of guessing.",
    ),
    Msg::new("cmd.geartab.kind.internal", "内齿轮", "internal gear"),
    Msg::new("cmd.geartab.kind.external", "外齿轮", "external gear"),
    Msg::new(
        "cmd.geartab.echo",
        "{kind} z{z} m{m}（ha*={ha}，c*={c}）公法线 k={k} W={w}",
        "{kind} z{z} m{m} (ha*={ha}, c*={c}) base tangent k={k} W={w}",
    ),

    // ── 命令输出/报错：ANSI 花键参数表命令行（ansi_table.rs）──
    Msg::new(
        "cmd.ansi.err.need_value",
        "ANSI 花键参数表：{what} 缺少数值/选项",
        "ANSI spline table: {what} is missing a value/option",
    ),
    Msg::new(
        "cmd.ansi.err.z_num",
        "ANSI 花键参数表：齿数={v} 不是整数：{e}",
        "ANSI spline table: tooth count={v} is not an integer: {e}",
    ),
    Msg::new(
        "cmd.ansi.err.at_format",
        "ANSI 花键参数表：at「{v}」应为 `x,y`",
        "ANSI spline table: at \"{v}\" must be `x,y`",
    ),
    Msg::new(
        "cmd.ansi.err.at_x",
        "ANSI 花键参数表：at x={x} 不是数字：{e}",
        "ANSI spline table: at x={x} is not a number: {e}",
    ),
    Msg::new(
        "cmd.ansi.err.at_y",
        "ANSI 花键参数表：at y={y} 不是数字：{e}",
        "ANSI spline table: at y={y} is not a number: {e}",
    ),
    Msg::new(
        "cmd.ansi.err.rot",
        "ANSI 花键参数表：rot={v} 不是数字：{e}",
        "ANSI spline table: rot={v} is not a number: {e}",
    ),
    Msg::new(
        "cmd.ansi.err.unknown_param",
        "ANSI 花键参数表：不认识的参数「{t}」。\n{usage}",
        "ANSI spline table: unrecognized argument \"{t}\".\n{usage}",
    ),
    Msg::new(
        "cmd.ansi.err.p_mismatch",
        "ANSI 花键参数表：显式径节 P={p} 与表达式反解的 P/Ps={ep} 不一致",
        "ANSI spline table: explicit diametral pitch P={p} differs from the expression's P/Ps={ep}",
    ),
    Msg::new(
        "cmd.ansi.err.n_mismatch",
        "ANSI 花键参数表：显式齿数 N={given} 与表达式反解的 N={ez} 不一致",
        "ANSI spline table: explicit tooth count N={given} differs from the expression's N={ez}",
    ),
    Msg::new(
        "cmd.ansi.err.missing_p",
        "ANSI 花键参数表：缺径节 P（写法 `P16` / `P 16/32` / `径节 16`，或直接给九字段表达式）。\n{usage}",
        "ANSI spline table: missing diametral pitch P (write `P16` / `P 16/32` / `径节 16`, or supply the nine-field expression).\n{usage}",
    ),
    Msg::new(
        "cmd.ansi.err.missing_z",
        "ANSI 花键参数表：缺齿数 Z（写法 `Z20` / `齿数 20`；表达式反解时无需再给）。\n{usage}",
        "ANSI spline table: missing tooth count Z (write `Z20` / `齿数 20`; not needed when the expression is back-solved).\n{usage}",
    ),
    Msg::new(
        "cmd.ansi.err.no_profile",
        "ANSI 花键参数表：没有压力角 {alpha}° 的 Table 2 齿廓（本仓仅 30/37.5/45）",
        "ANSI spline table: no Table 2 profile for pressure angle {alpha}° (this repo only has 30/37.5/45)",
    ),
    Msg::new(
        "cmd.ansi.err.map_missing_p",
        "ANSI 花键参数表：表达式映射表缺径节 P（内部错误）",
        "ANSI spline table: the expression mapping has no diametral pitch P (internal error)",
    ),
    Msg::new(
        "cmd.ansi.err.map_missing_n",
        "ANSI 花键参数表：表达式映射表缺齿数 N（内部错误）",
        "ANSI spline table: the expression mapping has no tooth count N (internal error)",
    ),
    Msg::new(
        "cmd.ansi.err.side_invalid",
        "ANSI 花键参数表：方向「{other}」非法（可选项 int/ext）",
        "ANSI spline table: invalid direction \"{other}\" (allowed: int/ext)",
    ),
    Msg::new(
        "cmd.ansi.usage",
        "智能卡片「{card}」用法：`OCSMCARD {card} 内|外 <九字段齿形表达式> [profile 齿廓] [at x,y] [rot 度]`（如 `OCSMCARD {card} 内 SPLINE IN M1.5875 Z20 ALPHA30 X0 BETA0 H30`；也可沿用 `P<径节> Z<齿数>` 写法，如 `OCSMCARD {card} 内 P16 Z20`）。表达式反解 P=25.4/m、N=z，齿廓按 α 保持同角列或选默认列（α 只收 30/37.5/45，直齿、不收变位）；齿廓五列：`ANSI30P`/`ANSI30PM`/`ANSI30R`/`ANSI375R`/`ANSI45R`（缺省列 A）；ANSI B92.1 的配合/公差/量棒/公法线表本仓未收 —— 相关格显示「—」，不臆造。",
        "Smart card \"{card}\" usage: `OCSMCARD {card} 内|外 <nine-field expression> [profile PROFILE] [at x,y] [rot deg]` (e.g. `OCSMCARD {card} 内 SPLINE IN M1.5875 Z20 ALPHA30 X0 BETA0 H30`; the `P<pitch> Z<teeth>` form still works, e.g. `OCSMCARD {card} 内 P16 Z20`). The expression is back-solved to P=25.4/m, N=z; the profile keeps the same-angle column or falls back to the default column (only α 30/37.5/45, spur only, no shift); profile tokens: `ANSI30P`/`ANSI30PM`/`ANSI30R`/`ANSI375R`/`ANSI45R` (default column A). ANSI B92.1 fit/tolerance/pin/base-tangent tables are not collected in this repo — the related cells show \"—\" instead of guessing.",
    ),
    Msg::new("cmd.ansi.side.internal", "内花键", "internal spline"),
    Msg::new("cmd.ansi.side.external", "外花键", "external spline"),
    Msg::new(
        "cmd.ansi.echo",
        "{side}，P/Ps={pair}，N={n}，α={alpha}°",
        "{side}, P/Ps={pair}, N={n}, α={alpha}°",
    ),

    // ── 命令输出/报错：NF 内花键参数表命令行（nf_table.rs）──
    Msg::new(
        "cmd.nf.err.a_positive",
        "NF 内花键参数表：公称直径 A={a} 必须是正数",
        "NF internal spline table: nominal diameter A={a} must be positive",
    ),
    Msg::new(
        "cmd.nf.err.m_positive",
        "NF 内花键参数表：模数 m={m} 必须是正数",
        "NF internal spline table: module m={m} must be positive",
    ),
    Msg::new(
        "cmd.nf.err.z_range",
        "NF 内花键参数表：齿数 z={z} 超出范围（3–1000）",
        "NF internal spline table: tooth count z={z} is out of range (3–1000)",
    ),
    Msg::new(
        "cmd.nf.err.z_table_mismatch",
        "NF 内花键参数表：齿数 z={z_in} 与 NF E22-141 表行（A={a} m={m}）的 N={z_tab} 不一致；表值行请去掉 z 或改为 N={z_tab}",
        "NF internal spline table: tooth count z={z_in} differs from N={z_tab} in the NF E22-141 table row (A={a} m={m}); drop z or set N={z_tab} for a table row",
    ),
    Msg::new(
        "cmd.nf.err.tag_order",
        "NF 内花键参数表：取值映射顺序与模板属性不一致：{got} != {want}",
        "NF internal spline table: value-mapping order differs from the template attributes: {got} != {want}",
    ),
    Msg::new(
        "cmd.nf.err.centering_invalid",
        "NF 内花键参数表：定心方式「{t}」非法（可用 外径 / 齿面）",
        "NF internal spline table: invalid centering \"{t}\" (allowed: 外径/major-diameter or 齿面/flank)",
    ),
    Msg::new(
        "cmd.nf.err.root_invalid",
        "NF 内花键参数表：齿根样式「{t}」非法（可用 平 / 圆）",
        "NF internal spline table: invalid root style \"{t}\" (allowed: 平/flat or 圆/fillet)",
    ),
    Msg::new(
        "cmd.nf.err.fit_invalid",
        "NF 内花键参数表：配合类别「{t}」非法（可用 松动 / 滑动 / 固定 / 压）",
        "NF internal spline table: invalid fit class \"{t}\" (allowed: 松动/loose, 滑动/sliding, 固定/fixed, 压/press)",
    ),
    Msg::new(
        "cmd.nf.err.map_missing_a",
        "NF 内花键参数表：表达式映射表缺 A（内部错误）",
        "NF internal spline table: the expression mapping has no A (internal error)",
    ),
    Msg::new(
        "cmd.nf.err.map_missing_m",
        "NF 内花键参数表：表达式映射表缺 m（内部错误）",
        "NF internal spline table: the expression mapping has no m (internal error)",
    ),
    Msg::new(
        "cmd.nf.err.map_missing_z",
        "NF 内花键参数表：表达式映射表缺 z（内部错误）",
        "NF internal spline table: the expression mapping has no z (internal error)",
    ),
    Msg::new(
        "cmd.nf.err.need",
        "NF 内花键参数表：{what} 缺少数值/选项",
        "NF internal spline table: {what} is missing a value/option",
    ),
    Msg::new(
        "cmd.nf.err.at_format",
        "NF 内花键参数表：at「{v}」应为 `x,y`",
        "NF internal spline table: at \"{v}\" must be `x,y`",
    ),
    Msg::new(
        "cmd.nf.err.at_x",
        "NF 内花键参数表：at x={x} 不是数字：{e}",
        "NF internal spline table: at x={x} is not a number: {e}",
    ),
    Msg::new(
        "cmd.nf.err.at_y",
        "NF 内花键参数表：at y={y} 不是数字：{e}",
        "NF internal spline table: at y={y} is not a number: {e}",
    ),
    Msg::new(
        "cmd.nf.err.rot",
        "NF 内花键参数表：rot={v} 不是数字：{e}",
        "NF internal spline table: rot={v} is not a number: {e}",
    ),
    Msg::new(
        "cmd.nf.err.num",
        "NF 内花键参数表：{name}={v} 不是数字：{e}",
        "NF internal spline table: {name}={v} is not a number: {e}",
    ),
    Msg::new(
        "cmd.nf.err.z_num",
        "NF 内花键参数表：{name}={v} 不是整数：{e}",
        "NF internal spline table: {name}={v} is not an integer: {e}",
    ),
    Msg::new(
        "cmd.nf.err.unknown_param",
        "NF 内花键参数表：不认识的参数「{t}」。\n{usage}",
        "NF internal spline table: unrecognized argument \"{t}\".\n{usage}",
    ),
    Msg::new(
        "cmd.nf.err.explicit_mismatch",
        "NF 内花键参数表：显式 {name}={given} 与表达式反解的 {name}={resolved} 不一致",
        "NF internal spline table: explicit {name}={given} differs from the expression's {name}={resolved}",
    ),
    Msg::new(
        "cmd.nf.err.missing_a",
        "NF 内花键参数表：缺公称直径 A（写法 `A300` / `直径 300`，或直接给九字段表达式）。\n{usage}",
        "NF internal spline table: missing nominal diameter A (write `A300` / `直径 300`, or supply the nine-field expression).\n{usage}",
    ),
    Msg::new(
        "cmd.nf.err.missing_m",
        "NF 内花键参数表：缺模数 m（写法 `M7.5` / `模数 7.5`；同一 A 可对应不同模数，必填）。\n{usage}",
        "NF internal spline table: missing module m (write `M7.5` / `模数 7.5`; the same A can match different modules, so it is required).\n{usage}",
    ),
    Msg::new(
        "cmd.nf.usage",
        "智能卡片「NF内花键参数表」用法：`OCSMCARD NF内花键参数表 <九字段齿形表达式> [中心 外径|齿面] [根 平|圆] [配合 松动|滑动|固定|压] [at x,y] [rot 度]`（如 `OCSMCARD NF内花键参数表 SPLINE IN M7.5 Z38 ALPHA20 X0.8 BETA0 H30`；也可沿用 `A300 M7.5 [Z38]` 写法）。表达式反解 A=m(z+0.4+2x)、m、z（NF 压力角恒 20°）；A/m 取 NF E22-141 表值（p18 尺寸表，同一直径可对应不同模数）；定心方式缺省「外径定心」（Az=A；齿面定心 Az=A+0.3m）；加工方法照 p18 = 拉削；V/V1/G/G1 取 p23–p25 检查表、ri 取 p22 —— 表外显示「—」，不外推；公差：大径 R7 / 小径 H7（p28，数值按 ISO 286），跨棒距 = p29 内花键 E 偏差；配合类别（缺省固定）只影响预览里配对外花键的 E/xm 偏差读数。",
        "Smart card \"NF internal spline table\" usage: `OCSMCARD NF内花键参数表 <nine-field expression> [中心 外径|齿面] [根 平|圆] [配合 松动|滑动|固定|压] [at x,y] [rot deg]` (e.g. `OCSMCARD NF内花键参数表 SPLINE IN M7.5 Z38 ALPHA20 X0.8 BETA0 H30`; the `A300 M7.5 [Z38]` form still works). The expression is back-solved to A=m(z+0.4+2x), m, z (NF pressure angle is always 20°); A/m come from the NF E22-141 table (p18 dimension table; the same diameter can match different modules); centering defaults to major-diameter (Az=A; flank centering Az=A+0.3m); the process follows p18 = broaching; V/V1/G/G1 come from the p23–p25 inspection table and ri from p22 — off-table values show \"—\" without extrapolation; tolerances: major R7 / minor H7 (p28, values per ISO 286), span = p29 internal-spline E deviation; the fit class (default fixed) only affects the mated external-spline E/xm deviation readings in the preview.",
    ),
    Msg::new("cmd.nf.centering.outer", "外径定心", "major-diameter centering"),
    Msg::new("cmd.nf.centering.flank", "齿面定心", "flank centering"),
    Msg::new("cmd.nf.fit.loose", "松动", "loose"),
    Msg::new("cmd.nf.fit.sliding", "滑动", "sliding"),
    Msg::new("cmd.nf.fit.fixed", "固定", "fixed"),
    Msg::new("cmd.nf.fit.press", "压", "press"),
    Msg::new(
        "cmd.nf.echo",
        "NF E22-141 内花键 A={a} m={m} z={z}（{centering}，拉削；{fit}配合；V/G/G1 {vg}）",
        "NF E22-141 internal spline A={a} m={m} z={z} ({centering}, broached; {fit} fit; V/G/G1 {vg})",
    ),

    // ── 命令输出/报错：NF 外花键参数表命令行（nf_ext_table.rs）──
    Msg::new(
        "cmd.nfext.err.a_positive",
        "NF 外花键参数表：公称直径 A={a} 必须是正数",
        "NF external spline table: nominal diameter A={a} must be positive",
    ),
    Msg::new(
        "cmd.nfext.err.m_positive",
        "NF 外花键参数表：模数 m={m} 必须是正数",
        "NF external spline table: module m={m} must be positive",
    ),
    Msg::new(
        "cmd.nfext.err.z_range",
        "NF 外花键参数表：齿数 z={z} 超出范围（3–1000）",
        "NF external spline table: tooth count z={z} is out of range (3–1000)",
    ),
    Msg::new(
        "cmd.nfext.err.z_table_mismatch",
        "NF 外花键参数表：齿数 z={z_in} 与 NF E22-141 表行（A={a} m={m}）的 N={z_tab} 不一致；表值行请去掉 z 或改为 N={z_tab}",
        "NF external spline table: tooth count z={z_in} differs from N={z_tab} in the NF E22-141 table row (A={a} m={m}); drop z or set N={z_tab} for a table row",
    ),
    Msg::new(
        "cmd.nfext.err.tag_order",
        "NF 外花键参数表：取值映射顺序与模板属性不一致：{got} != {want}",
        "NF external spline table: value-mapping order differs from the template attributes: {got} != {want}",
    ),
    Msg::new(
        "cmd.nfext.err.centering_invalid",
        "NF 外花键参数表：定心方式「{t}」非法（可用 齿面 / 外径）",
        "NF external spline table: invalid centering \"{t}\" (allowed: 齿面/flank or 外径/major-diameter)",
    ),
    Msg::new(
        "cmd.nfext.err.root_invalid",
        "NF 外花键参数表：齿根样式「{t}」非法（可用 平 / 圆）",
        "NF external spline table: invalid root style \"{t}\" (allowed: 平/flat or 圆/fillet)",
    ),
    Msg::new(
        "cmd.nfext.err.fit_invalid",
        "NF 外花键参数表：配合类别「{t}」非法（可用 松动 / 滑动 / 固定 / 压）",
        "NF external spline table: invalid fit class \"{t}\" (allowed: 松动/loose, 滑动/sliding, 固定/fixed, 压/press)",
    ),
    Msg::new(
        "cmd.nfext.err.map_missing_a",
        "NF 外花键参数表：表达式映射表缺 A（内部错误）",
        "NF external spline table: the expression mapping has no A (internal error)",
    ),
    Msg::new(
        "cmd.nfext.err.map_missing_m",
        "NF 外花键参数表：表达式映射表缺 m（内部错误）",
        "NF external spline table: the expression mapping has no m (internal error)",
    ),
    Msg::new(
        "cmd.nfext.err.map_missing_z",
        "NF 外花键参数表：表达式映射表缺 z（内部错误）",
        "NF external spline table: the expression mapping has no z (internal error)",
    ),
    Msg::new(
        "cmd.nfext.err.need",
        "NF 外花键参数表：{what} 缺少数值/选项",
        "NF external spline table: {what} is missing a value/option",
    ),
    Msg::new(
        "cmd.nfext.err.at_format",
        "NF 外花键参数表：at「{v}」应为 `x,y`",
        "NF external spline table: at \"{v}\" must be `x,y`",
    ),
    Msg::new(
        "cmd.nfext.err.at_x",
        "NF 外花键参数表：at x={x} 不是数字：{e}",
        "NF external spline table: at x={x} is not a number: {e}",
    ),
    Msg::new(
        "cmd.nfext.err.at_y",
        "NF 外花键参数表：at y={y} 不是数字：{e}",
        "NF external spline table: at y={y} is not a number: {e}",
    ),
    Msg::new(
        "cmd.nfext.err.rot",
        "NF 外花键参数表：rot={v} 不是数字：{e}",
        "NF external spline table: rot={v} is not a number: {e}",
    ),
    Msg::new(
        "cmd.nfext.err.num",
        "NF 外花键参数表：{name}={v} 不是数字：{e}",
        "NF external spline table: {name}={v} is not a number: {e}",
    ),
    Msg::new(
        "cmd.nfext.err.z_num",
        "NF 外花键参数表：{name}={v} 不是整数：{e}",
        "NF external spline table: {name}={v} is not an integer: {e}",
    ),
    Msg::new(
        "cmd.nfext.err.unknown_param",
        "NF 外花键参数表：不认识的参数「{t}」。\n{usage}",
        "NF external spline table: unrecognized argument \"{t}\".\n{usage}",
    ),
    Msg::new(
        "cmd.nfext.err.explicit_mismatch",
        "NF 外花键参数表：显式 {name}={given} 与表达式反解的 {name}={resolved} 不一致",
        "NF external spline table: explicit {name}={given} differs from the expression's {name}={resolved}",
    ),
    Msg::new(
        "cmd.nfext.err.missing_a",
        "NF 外花键参数表：缺公称直径 A（写法 `A300` / `直径 300`，或直接给九字段表达式）。\n{usage}",
        "NF external spline table: missing nominal diameter A (write `A300` / `直径 300`, or supply the nine-field expression).\n{usage}",
    ),
    Msg::new(
        "cmd.nfext.err.missing_m",
        "NF 外花键参数表：缺模数 m（写法 `M7.5` / `模数 7.5`；同一 A 可对应不同模数，必填）。\n{usage}",
        "NF external spline table: missing module m (write `M7.5` / `模数 7.5`; the same A can match different modules, so it is required).\n{usage}",
    ),
    Msg::new(
        "cmd.nfext.usage",
        "智能卡片「NF外花键参数表」用法：`OCSMCARD NF外花键参数表 <九字段齿形表达式> [中心 齿面|外径] [根 平|圆] [配合 松动|滑动|固定|压] [at x,y] [rot 度]`（如 `OCSMCARD NF外花键参数表 SPLINE EX M7.5 Z38 ALPHA20 X0.8 BETA0 H30`；也可沿用 `A300 M7.5 [Z38]` 写法）。表达式反解 A=m(z+0.4+2x)、m、z（NF 压力角恒 20°、方向恒外 EX）；定心方式缺省「齿面定心」（Dee=A−0.2m，模板口径；外径定心 Dee=A）；加工方法照模板 = 滚齿；小径 Die 平齿根 A−2.4m / 圆齿根 A−2.694m；K/W 取 p23–p25 检查表 —— 表外显示「—」，不外推；公差：大径 h12 / 小径 H7（ISO 286，模板实测口径），公法线 = p29 外花键 E 偏差（按配合类别，缺省固定）。",
        "Smart card \"NF external spline table\" usage: `OCSMCARD NF外花键参数表 <nine-field expression> [中心 齿面|外径] [根 平|圆] [配合 松动|滑动|固定|压] [at x,y] [rot deg]` (e.g. `OCSMCARD NF外花键参数表 SPLINE EX M7.5 Z38 ALPHA20 X0.8 BETA0 H30`; the `A300 M7.5 [Z38]` form still works). The expression is back-solved to A=m(z+0.4+2x), m, z (NF pressure angle is always 20°, direction always external EX); centering defaults to flank (Dee=A−0.2m, template convention; major-diameter centering Dee=A); the process follows the template = hobbing; minor diameter Die is A−2.4m for flat root / A−2.694m for fillet root; K/W come from the p23–p25 inspection table — off-table values show \"—\" without extrapolation; tolerances: major h12 / minor H7 (ISO 286, template-measured convention), base tangent = p29 external-spline E deviation (per fit class, default fixed).",
    ),
    Msg::new(
        "cmd.nfext.echo",
        "NF E22-141 外花键 A={a} m={m} z={z}（{centering}，滚齿；{fit}配合；K/W {kw}）",
        "NF E22-141 external spline A={a} m={m} z={z} ({centering}, hobbed; {fit} fit; K/W {kw})",
    ),

    // ── 命令输出/报错：DIN 5480 花键参数表命令行（din_table.rs）──
    Msg::new("cmd.din.err.prefix", "DIN 花键参数表：{e}", "DIN spline table: {e}"),
    Msg::new(
        "cmd.din.err.m_positive",
        "DIN 花键参数表：模数 m={m} 必须是正数",
        "DIN spline table: module m={m} must be positive",
    ),
    Msg::new(
        "cmd.din.err.z_range",
        "DIN 花键参数表：齿数 z={z} 超出范围（3–1000）",
        "DIN spline table: tooth count z={z} is out of range (3–1000)",
    ),
    Msg::new(
        "cmd.din.err.db_positive",
        "DIN 花键参数表：基准直径 d_B={d_b} 必须是正数",
        "DIN spline table: base diameter d_B={d_b} must be positive",
    ),
    Msg::new(
        "cmd.din.err.grade_range",
        "公差等级 {grade} 超出范围（1–12）",
        "tolerance grade {grade} is out of range (1–12)",
    ),
    Msg::new(
        "cmd.din.err.dev_letter",
        "{side}偏差系列「{letter}」非法（{allowed}）",
        "{side}deviation series \"{letter}\" is invalid ({allowed})",
    ),
    Msg::new("cmd.din.side.hole", "孔 ", "hole "),
    Msg::new("cmd.din.side.shaft", "轴 ", "shaft "),
    Msg::new(
        "cmd.din.dev.hub",
        "F/G/H/J/K/M（DIN 5480 §10.3 六个）",
        "F/G/H/J/K/M (six, DIN 5480 §10.3)",
    ),
    Msg::new(
        "cmd.din.dev.shaft",
        "v/u/t/s/r/p/n/m/k/js/h/g/f/e/d/c/b/a（十八个）",
        "v/u/t/s/r/p/n/m/k/js/h/g/f/e/d/c/b/a (eighteen)",
    ),
    Msg::new(
        "cmd.din.err.tag_order",
        "DIN 花键参数表：取值映射顺序与模板属性不一致：{got} != {want}",
        "DIN spline table: value-mapping order differs from the template attributes: {got} != {want}",
    ),
    Msg::new(
        "cmd.din.err.need",
        "DIN 花键参数表：{what} 缺少数值",
        "DIN spline table: {what} is missing a value",
    ),
    Msg::new(
        "cmd.din.err.body_invalid",
        "DIN 花键参数表：代号体「{t}」非法。\n{usage}",
        "DIN spline table: invalid designation body \"{t}\".\n{usage}",
    ),
    Msg::new(
        "cmd.din.err.code_invalid",
        "DIN 花键参数表：代号「{t}」非法（应如 `N120×3×38×9H`）。\n{usage}",
        "DIN spline table: invalid designation \"{t}\" (should look like `N120×3×38×9H`).\n{usage}",
    ),
    Msg::new(
        "cmd.din.err.key_missing",
        "DIN 花键参数表：「{key}」缺少数值",
        "DIN spline table: \"{key}\" is missing a value",
    ),
    Msg::new(
        "cmd.din.err.at_format",
        "DIN 花键参数表：at「{v}」应为 `x,y`",
        "DIN spline table: at \"{v}\" must be `x,y`",
    ),
    Msg::new("cmd.din.err.at_x", "at x={x} 不是数字：{e}", "at x={x} is not a number: {e}"),
    Msg::new("cmd.din.err.at_y", "at y={y} 不是数字：{e}", "at y={y} is not a number: {e}"),
    Msg::new("cmd.din.err.rot", "rot={v} 不是数字：{e}", "rot={v} is not a number: {e}"),
    Msg::new(
        "cmd.din.err.num",
        "{name}={v} 不是数字：{e}",
        "{name}={v} is not a number: {e}",
    ),
    Msg::new(
        "cmd.din.err.z_num",
        "{name}={v} 不是整数：{e}",
        "{name}={v} is not an integer: {e}",
    ),
    Msg::new(
        "cmd.din.err.unknown_param_bare",
        "不认识的参数「{t}」",
        "unrecognized argument \"{t}\"",
    ),
    Msg::new(
        "cmd.din.err.wrapped",
        "DIN 花键参数表：{e}。\n{usage}",
        "DIN spline table: {e}.\n{usage}",
    ),
    Msg::new(
        "cmd.din.err.pending_len",
        "DIN 花键参数表：`N`/`W` 后缺长度代号（如 `N 120×3×38×9H`）。\n{usage}",
        "DIN spline table: `N`/`W` is missing a length designation (e.g. `N 120×3×38×9H`).\n{usage}",
    ),
    Msg::new(
        "cmd.din.err.map_missing_m",
        "DIN 花键参数表：表达式映射表缺 m（内部错误）",
        "DIN spline table: the expression mapping has no m (internal error)",
    ),
    Msg::new(
        "cmd.din.err.map_missing_z",
        "DIN 花键参数表：表达式映射表缺 z（内部错误）",
        "DIN spline table: the expression mapping has no z (internal error)",
    ),
    Msg::new(
        "cmd.din.err.map_missing_db",
        "DIN 花键参数表：表达式映射表缺 d_B（内部错误）",
        "DIN spline table: the expression mapping has no d_B (internal error)",
    ),
    Msg::new(
        "cmd.din.err.explicit_mismatch",
        "DIN 花键参数表：显式 {name}={given} 与表达式反解的 {name}={resolved} 不一致",
        "DIN spline table: explicit {name}={given} differs from the expression's {name}={resolved}",
    ),
    Msg::new(
        "cmd.din.err.missing_fields",
        "DIN 花键参数表：缺 {what}（写法 `M3 Z38 B120` / 代号 `N 120×3×38×9H` / 九字段表达式）。\n{usage}",
        "DIN spline table: missing {what} (write `M3 Z38 B120` / designation `N 120×3×38×9H` / nine-field expression).\n{usage}",
    ),
    Msg::new("cmd.din.what.mzdb", "模数 m、齿数 z、基准直径 d_B", "module m, tooth count z, base diameter d_B"),
    Msg::new("cmd.din.what.m", "模数 m", "module m"),
    Msg::new("cmd.din.what.z", "齿数 z", "tooth count z"),
    Msg::new("cmd.din.what.db", "基准直径 d_B", "base diameter d_B"),
    Msg::new(
        "cmd.din.err.fit_grade_digits",
        "缺等级数字（如 `9H`/`8f`）",
        "missing grade digits (e.g. `9H`/`8f`)",
    ),
    Msg::new(
        "cmd.din.err.fit_grade_invalid",
        "等级「{digits}」非法",
        "invalid grade \"{digits}\"",
    ),
    Msg::new(
        "cmd.din.err.fit_letter_missing",
        "缺偏差字母（孔 F/G/H/J/K/M；轴 v…a）",
        "missing deviation letter (hole F/G/H/J/K/M; shaft v…a)",
    ),
    Msg::new(
        "cmd.din.usage",
        "智能卡片「DIN 5480 内/外花键参数表」用法（一卡一方向）：`OCSMCARD DIN花键参数表 <九字段表达式> [N<等级><字母>] [e2 …] [ae …] [as …] [tactn …] [teffn …] [at x,y]`（外卡：`OCSMCARD DIN花键参数表_外 <九字段表达式> [W<等级><字母>] [tactw …] [teffw …] …`；旧写法 `M3 Z38 B120 N9H W8f` 兼容：旧 id 默认内卡，N/W 都收、只取本侧）。表达式反解 d_B=m(z+1.1+2x)、m、z（DIN 5480 压力角恒 30°），KIND 须与卡方向一致（内 IN / 外 EX）。版面 = DIN 5480-1:2006 Bild 6 单栏 13 行（内 = Nabe、外 = Welle），整表 INSERT 缩放 0.17；孔缺省 9H、轴缺省 8f；Ae/As 取 Table 7，Tact/Teff 取 Table 7 公差锚 —— 缺口显示「—」，可用 ae/as/e2/tactn/teffn/tactw/teffw 覆盖。",
        "Smart card \"DIN 5480 internal/external spline table\" usage (one direction per card): `OCSMCARD DIN花键参数表 <nine-field expression> [N<grade><letter>] [e2 …] [ae …] [as …] [tactn …] [teffn …] [at x,y]` (external card: `OCSMCARD DIN花键参数表_外 <nine-field expression> [W<grade><letter>] [tactw …] [teffw …] …`; the old form `M3 Z38 B120 N9H W8f` still works: the old id defaults to the internal card, accepts both N/W and uses only this side). The expression is back-solved to d_B=m(z+1.1+2x), m, z (DIN 5480 pressure angle is always 30°), and KIND must match the card direction (IN for internal / EX for external). Layout = DIN 5480-1:2006 Bild 6 single-column 13 rows (internal = Nabe, external = Welle), the whole INSERT is scaled 0.17; hole default 9H, shaft default 8f; Ae/As come from Table 7 and Tact/Teff from the Table 7 tolerance anchors — gaps show \"—\" and can be overridden with ae/as/e2/tactn/teffn/tactw/teffw.",
    ),
    Msg::new(
        "cmd.din.echo.anchor",
        "，Bild 6 示例原印值",
        ", Bild 6 example printed values",
    ),
    Msg::new(
        "cmd.din.echo",
        "DIN 5480 {designation}（dB={db} m={m} z={z}{anchor}）",
        "DIN 5480 {designation} (dB={db} m={m} z={z}{anchor})",
    ),

    // ── 命令输出/报错：精简卡引擎（spline_lite / card_lite）──
    Msg::new(
        "cmd.splinelite.err.side_mismatch",
        "GB 花键精简卡：方向与计算结果不一致",
        "GB spline lite card: direction does not match the computed result",
    ),
    Msg::new(
        "cmd.cardlite.err.serialize",
        "{id}：请求序列化失败：{e}",
        "{id}: failed to serialize the request: {e}",
    ),
    Msg::new(
        "cmd.cardlite.err.preview",
        "{id}：{e}",
        "{id}: {e}",
    ),
    Msg::new(
        "cmd.cardlite.err.preview_parse",
        "{id}：完整卡预览 JSON 解析失败：{e}",
        "{id}: failed to parse the full-card preview JSON: {e}",
    ),
    Msg::new(
        "cmd.cardlite.err.gear_model",
        "齿轮精简卡：请求字段无效：{e}",
        "Gear lite card: invalid request field: {e}",
    ),
    Msg::new(
        "cmd.cardlite.err.dir_mismatch",
        "{id}：方向记号与卡片方向不一致（卡片固定{side}）",
        "{id}: direction token conflicts with the card direction (card is fixed to {side})",
    ),
    Msg::new(
        "cmd.cardlite.err.extra_missing",
        "{id}：补算项 {tag} 缺失",
        "{id}: computed extra {tag} is missing",
    ),
    Msg::new(
        "cmd.cardlite.err.full_item_missing",
        "{id}：完整卡项「{key}」不在取值表里",
        "{id}: full-card item \"{key}\" is not in the value table",
    ),
    Msg::new(
        "cmd.cardlite.err.preview_missing",
        "{id}：预览缺项 {tag}",
        "{id}: preview is missing item {tag}",
    ),

    // ── 命令输出/报错：OCSMCARD 用法 + 卡导出回执（card.rs / guide_server / spline_gui）──
    Msg::new(
        "cmd.card.label.spline",
        "花键参数表",
        "spline table",
    ),
    Msg::new(
        "cmd.card.export.inserted",
        "已插入{kind}参数表（{echo}）于 ({x}, {y}) rot {rot}°。",
        "Inserted {kind} parameter table ({echo}) at ({x}, {y}) rot {rot}°.",
    ),
    Msg::new(
        "cmd.card.export.pending_note",
        "{kind}参数表（{echo}）",
        "{kind} parameter table ({echo})",
    ),
    Msg::new(
        "cmd.card.export.pending",
        "已生成{kind}参数表（{echo}）：切回图纸，鼠标上已带这张表，左键点击定位基点 → 移动光标旋转 → 再点击落定（可连续，Esc 结束）。",
        "Generated {kind} parameter table ({echo}): switch back to the drawing; the table is attached to the cursor. Click to set the base point → move to rotate → click again to place (repeatable; Esc to finish).",
    ),
    Msg::new(
        "cmd.card.export.inserted_card",
        "已插入{kind}（{echo}）于 ({x}, {y}) rot {rot}°。",
        "Inserted {kind} ({echo}) at ({x}, {y}) rot {rot}°.",
    ),
    Msg::new(
        "cmd.card.export.pending_note_card",
        "{kind}（{echo}）",
        "{kind} ({echo})",
    ),
    Msg::new(
        "cmd.card.export.pending_card",
        "已生成{kind}（{echo}）：切回图纸，鼠标上已带这张表，左键点击定位基点 → 移动光标旋转 → 再点击落定（可连续，Esc 结束）。",
        "Generated {kind} ({echo}): switch back to the drawing; the table is attached to the cursor. Click to set the base point → move to rotate → click again to place (repeatable; Esc to finish).",
    ),
    Msg::new(
        "cmd.spline.echo.dp",
        "；量棒 Dp={dp}（{mode}，D'={dp_calc}，备选 {c1} / {c2} / {c3}）",
        "; pin Dp={dp} ({mode}, D'={dp_calc}, candidates {c1} / {c2} / {c3})",
    ),
    Msg::new(
        "cmd.spline.echo.dp_manual",
        "手填，Md 已重算",
        "manual entry, Md recomputed",
    ),
    Msg::new(
        "cmd.spline.echo.dp_auto",
        "标准 R40 自动选",
        "auto-picked per standard R40",
    ),

    // ── 命令输出/报错：轴生成器命令行解析（shaft.rs）──
    Msg::new(
        "cmd.shaft.label.multi",
        "第 {line} 行第 {chunk} 段",
        "line {line}, segment {chunk}",
    ),
    Msg::new(
        "cmd.shaft.label.single",
        "第 {line} 行（第 {chunk} 段）",
        "line {line} (segment {chunk})",
    ),
    Msg::new(
        "cmd.shaft.err.unknown_keyword",
        "{label}：不识别的关键字「{token}」（本期支持 S/E/L/CH/OV/M/TL/RO/SD/RL/GEAR/SPLINE/KEY/VIEW；齿形段子关键字 M/Z/H/BETA/ALPHA/EX/IN/X/DA/DF（SPLINE + 齿形关键字 = 渐开线花键）；RL 的尺寸参数 P/g1/g2/dg/r 跟在 RL 后面；轴槽子关键字 = 键型 A/B/C + 键尺寸 b…/h…（可连写 `b8h7`）+ 键长（裸数字或 KL…）+ @中/@端（可选 t1）+ 导向（GB/T 1097，可选 `双槽`））",
        "{label}: unrecognized keyword \"{token}\" (supported: S/E/L/CH/OV/M/TL/RO/SD/RL/GEAR/SPLINE/KEY/VIEW; tooth-profile sub-keywords M/Z/H/BETA/ALPHA/EX/IN/X/DA/DF (SPLINE + tooth keywords = involute spline); RL size parameters P/g1/g2/dg/r follow RL; shaft-key sub-keywords = key style A/B/C + key size b…/h… (may be joined as `b8h7`) + key length (bare number or KL…) + @中/@端 (optional t1) + guided (GB/T 1097, optional `双槽`))",
    ),
    Msg::new(
        "cmd.shaft.err.view_unknown",
        "视图名无法识别：`{name}`。可用：normal|常规、section|剖视。",
        "unrecognized view name: `{name}`. Available: normal|常规, section|剖视.",
    ),
    Msg::new(
        "cmd.shaft.err.view_missing",
        "{label}：关键字 VIEW 缺少视图名（常规/剖视）",
        "{label}: keyword VIEW is missing a view name (常规/剖视)",
    ),
    Msg::new(
        "cmd.shaft.err.view_conflict",
        "{label}：视图 VIEW 重复且冲突（已给「{prev}」，又给「{now}」）",
        "{label}: VIEW is repeated with a conflict (already \"{prev}\", now \"{now}\")",
    ),
    Msg::new(
        "cmd.shaft.err.tooth_radii_pos",
        "{label}：齿形段的大径 DA={da}、小径 DF={df} 必须是正数",
        "{label}: tooth-profile segment major DA={da} and minor DF={df} must be positive",
    ),
    Msg::new(
        "cmd.shaft.err.tooth_radii_order",
        "{label}：齿形段的大径 DA={da} 必须大于小径 DF={df}（外齿：DA=齿顶圆；内齿：DA=外侧齿根）",
        "{label}: tooth-profile segment major DA={da} must exceed minor DF={df} (external: DA=tip circle; internal: DA=outer root)",
    ),
    Msg::new(
        "cmd.shaft.err.end_missing",
        "{label}：关键字 {what} 的端别缺省（@ 后要 L 或 R）",
        "{label}: keyword {what} is missing an end (after @ use L or R)",
    ),
    Msg::new(
        "cmd.shaft.err.end_bad",
        "{label}：端别「{bad}」非法（只能用 L 或 R）",
        "{label}: invalid end \"{bad}\" (only L or R)",
    ),
    Msg::new(
        "cmd.shaft.err.key_value_not_num",
        "{label}：关键字 {what} 的值「{value}」不是数字",
        "{label}: value \"{value}\" of keyword {what} is not a number",
    ),
    Msg::new(
        "cmd.shaft.err.key_no_value",
        "{label}：关键字 {what} 缺少数值",
        "{label}: keyword {what} is missing a value",
    ),
    Msg::new(
        "cmd.shaft.err.ch_no_value",
        "{label}：关键字 CH 缺少数值（写法 CH2 / CH2@L / CH2@R）",
        "{label}: keyword CH is missing a value (write CH2 / CH2@L / CH2@R)",
    ),
    Msg::new(
        "cmd.shaft.err.ch_positive",
        "{label}：关键字 CH 的 C={c} 非法（必须 > 0）",
        "{label}: keyword CH C={c} is invalid (must be > 0)",
    ),
    Msg::new(
        "cmd.shaft.err.ov_not_num",
        "{label}：关键字 OV 的值「{value}」不是数字",
        "{label}: value \"{value}\" of keyword OV is not a number",
    ),
    Msg::new(
        "cmd.shaft.err.ov_positive",
        "{label}：关键字 OV 的 b1={b1} 非法（必须 > 0）",
        "{label}: keyword OV b1={b1} is invalid (must be > 0)",
    ),
    Msg::new(
        "cmd.shaft.err.es_cancelled",
        "{label}：`ES` 已取消 —— 退刀槽请用段级 `RL`（例 `S25 E25 L32 RL@L P1.5`）或一小段小直径轴段表示，例如 `S24 E24 L5`",
        "{label}: `ES` was removed — use the segment-level `RL` for relief grooves (e.g. `S25 E25 L32 RL@L P1.5`), or a short small-diameter segment such as `S24 E24 L5`",
    ),
    Msg::new(
        "cmd.shaft.err.m_pitch",
        "{label}：关键字 M 的螺距 P={pitch} 非法（必须 > 0；不写值 = 小径 0.85d 简化画法）",
        "{label}: pitch P={pitch} of keyword M is invalid (must be > 0; omit the value to draw the simplified 0.85d minor diameter)",
    ),
    Msg::new(
        "cmd.shaft.err.ro_invalid_t3",
        "{label}：收尾档位 RO「{other}」非法（GB/T 3 表 1 只有 一般 / 短，没有 长）",
        "{label}: invalid run-out class RO \"{other}\" (GB/T 3 Table 1 only has 一般/normal or 短/short, not 长/long)",
    ),
    Msg::new(
        "cmd.shaft.err.sd_invalid_t3",
        "{label}：肩距档位 SD「{other}」非法（GB/T 3 表 1 只有 一般 / 长 / 短）",
        "{label}: invalid shoulder-distance class SD \"{other}\" (GB/T 3 Table 1 only has 一般/normal, 长/long, 短/short)",
    ),
    Msg::new(
        "cmd.shaft.err.tl_no_value",
        "{label}：关键字 TL 缺少数值（写法 TL20）",
        "{label}: keyword TL is missing a value (write TL20)",
    ),
    Msg::new(
        "cmd.shaft.err.tl_positive",
        "{label}：完整螺纹长度 TL={value} 非法（必须 > 0；不给 TL = 整段全线程）",
        "{label}: full thread length TL={value} is invalid (must be > 0; omit TL for a fully threaded segment)",
    ),
    Msg::new(
        "cmd.shaft.err.rl_missing_params",
        "{label}：RL 退刀槽缺参 —— 表 2 以螺距为键，请给 P（例 `RL@L P1.5`）或显式 g1/g2/dg/r（例 `RL@L g1 2.5 g2 4.5 dg 22.7 r 0.8`）",
        "{label}: RL relief groove is missing parameters — Table 2 is keyed by pitch, so supply P (e.g. `RL@L P1.5`) or explicit g1/g2/dg/r (e.g. `RL@L g1 2.5 g2 4.5 dg 22.7 r 0.8`)",
    ),
    Msg::new(
        "cmd.shaft.err.key_no_type",
        "{label}：KEY 缺少键型（A/B/C；例 `KEY A 18`）",
        "{label}: KEY is missing the key style (A/B/C; e.g. `KEY A 18`)",
    ),
    Msg::new(
        "cmd.shaft.err.key_dup",
        "{label}：关键字 {what} 重复",
        "{label}: keyword {what} is repeated",
    ),
    Msg::new(
        "cmd.shaft.err.key_1097_no_c",
        "{label}：导向平键（GB/T 1097）只有 A/B 型，没有 C 型（用户 2026-09-25 更正）",
        "{label}: guided parallel keys (GB/T 1097) only have types A/B, not C (corrected 2026-09-25)",
    ),
    Msg::new(
        "cmd.shaft.err.key_no_place",
        "{label}：{kind}只有{places}（{tag}）",
        "{label}: {kind} only has {places} ({tag})",
    ),
    Msg::new(
        "cmd.shaft.err.key_no_double",
        "{label}：{kind}不支持双槽（固定键只有一个槽）",
        "{label}: {kind} does not support double keyways (a fixed key has only one slot)",
    ),
    Msg::new(
        "cmd.shaft.err.key_1097_b",
        "{label}：GB/T 1097 导向平键表里没有 b={b}（b=8…45，14 档）",
        "{label}: the GB/T 1097 guided-key table has no b={b} (b=8…45, 14 steps)",
    ),
    Msg::new(
        "cmd.shaft.err.key_1096_b",
        "{label}：{kind} 的平键族表里没有 b={b}（GB/T 1096 表 b=2…50）",
        "{label}: {kind} parallel-key table has no b={b} (GB/T 1096 table b=2…50)",
    ),
    Msg::new(
        "cmd.shaft.err.key_bh_pair",
        "{label}：键尺寸 b{b}×h{h} 不是标准配对（{kind} 的 b={b} 应配 h={std_h}）；h 跟 b 走，不能自由组合",
        "{label}: key size b{b}×h{h} is not a standard pair ({kind} b={b} should pair with h={std_h}); h follows b and cannot be combined freely",
    ),
    Msg::new(
        "cmd.shaft.err.key_no_len",
        "{label}：KEY 缺少键长（写法：裸数字 `18` 或 `KL18`；`L…` 是段长）",
        "{label}: KEY is missing the key length (bare number `18` or `KL18`; `L…` is the segment length)",
    ),
    Msg::new(
        "cmd.shaft.err.key_la_lc",
        "{label}：KEY 的 LA/LC 是旧「槽长」写法（已改口径，不静默兼容）—— 现在按 键型 + 键长 L + 位置：中置 `KEY A 18`；端置 `KEY C 14 @端`（端置槽长 = 键长 + t1，即模板 LC 口径）",
        "{label}: KEY LA/LC is the old \"slot length\" form (removed without silent compatibility) — now use key style + key length L + position: centered `KEY A 18`; end `KEY C 14 @端` (end slot length = key length + t1, the template LC convention)",
    ),
    Msg::new(
        "cmd.shaft.err.key_param_no_value",
        "{label}：KEY 参数 {name} 缺少数值",
        "{label}: KEY parameter {name} is missing a value",
    ),
    Msg::new(
        "cmd.shaft.err.key_param_dup",
        "{label}：KEY 参数 {what} 重复",
        "{label}: KEY parameter {what} is repeated",
    ),
    Msg::new(
        "cmd.shaft.err.key_place_conflict",
        "{label}：KEY 位置重复且冲突（已给「{prev}」，又给「{now}」）",
        "{label}: KEY position is repeated with a conflict (already \"{prev}\", now \"{now}\")",
    ),
    Msg::new(
        "cmd.shaft.err.key_type_conflict",
        "{label}：KEY 键型重复且冲突（已给「{prev}」，又给「{now}」）",
        "{label}: KEY style is repeated with a conflict (already \"{prev}\", now \"{now}\")",
    ),
    Msg::new(
        "cmd.shaft.err.key_type_invalid",
        "{label}：KEY 键型「{value}」非法（可选：{options}）",
        "{label}: invalid KEY style \"{value}\" (available: {options})",
    ),
    Msg::new(
        "cmd.shaft.err.key_guide_conflict",
        "{label}：键型「{kind}」与「导向」冲突（该键型不是导向平键）",
        "{label}: key style \"{kind}\" conflicts with \"导向\" (that style is not a guided parallel key)",
    ),
    Msg::new(
        "cmd.shaft.err.key_param_positive",
        "{label}：KEY 参数 {what}={value} 必须 > 0",
        "{label}: KEY parameter {what}={value} must be > 0",
    ),
    Msg::new(
        "cmd.shaft.err.ch_dup_end",
        "{label}：关键字 CH 在{end}端重复",
        "{label}: keyword CH is repeated at the {end} end",
    ),
    Msg::new(
        "cmd.shaft.err.ov_dup_end",
        "{label}：关键字 OV 在{end}端重复",
        "{label}: keyword OV is repeated at the {end} end",
    ),
    Msg::new(
        "cmd.shaft.err.ro_invalid",
        "{label}：收尾档位 RO「{rest}」非法（只有 一般 / 短）",
        "{label}: invalid run-out class RO \"{rest}\" (only 一般/normal or 短/short)",
    ),
    Msg::new(
        "cmd.shaft.err.sd_invalid",
        "{label}：肩距档位 SD「{rest}」非法（只有 一般 / 长 / 短）",
        "{label}: invalid shoulder-distance class SD \"{rest}\" (only 一般/normal, 长/long, 短/short)",
    ),
    Msg::new(
        "cmd.shaft.err.rl_no_value",
        "{label}：关键字 RL 不带值（写法 RL / RL@L / RL@R；尺寸参数跟在 RL 后面，如 `RL@L P1.5`）",
        "{label}: keyword RL takes no value (write RL / RL@L / RL@R; size parameters follow RL, e.g. `RL@L P1.5`)",
    ),
    Msg::new(
        "cmd.shaft.err.rl_dup_end",
        "{label}：关键字 RL 重复（在{end}端）",
        "{label}: keyword RL is repeated (at the {end} end)",
    ),
    Msg::new(
        "cmd.shaft.err.rl_param_order",
        "{label}：参数 {key} 要跟在 RL 后面（例 `RL@L P1.5` / `RL@L g1 2.5 …`）",
        "{label}: parameter {key} must follow RL (e.g. `RL@L P1.5` / `RL@L g1 2.5 …`)",
    ),
    Msg::new(
        "cmd.shaft.err.rl_param_no_value",
        "{label}：RL 参数 {key} 缺少数值",
        "{label}: RL parameter {key} is missing a value",
    ),
    Msg::new(
        "cmd.shaft.err.rl_param_positive",
        "{label}：RL 参数 {key}={value} 非法（必须 > 0）",
        "{label}: RL parameter {key}={value} is invalid (must be > 0)",
    ),
    Msg::new(
        "cmd.shaft.err.z_not_int",
        "{label}：关键字 Z 的值「{rest}」不是正整数（齿数 z 必须是整数）",
        "{label}: value \"{rest}\" of keyword Z is not a positive integer (tooth count z must be an integer)",
    ),
    Msg::new(
        "cmd.shaft.err.combo_spline_ch",
        "{label}：花键段不能与倒角 CH 同段（不自动画引入倒角；请在相邻轴段上写 CH）",
        "{label}: a spline segment cannot share a segment with chamfer CH (no lead-in chamfer is added automatically; write CH on a neighbouring segment)",
    ),
    Msg::new(
        "cmd.shaft.err.combo_spline_ov",
        "{label}：花键段不能与越程槽 OV 同段",
        "{label}: a spline segment cannot share a segment with an overtravel groove OV",
    ),
    Msg::new(
        "cmd.shaft.err.combo_spline_rl",
        "{label}：花键段不能与退刀槽 RL 同段",
        "{label}: a spline segment cannot share a segment with a relief groove RL",
    ),
    Msg::new(
        "cmd.shaft.err.combo_spline_m",
        "{label}：花键段不能与螺纹段 M/TL/RO/SD 同段",
        "{label}: a spline segment cannot share a segment with thread M/TL/RO/SD",
    ),
    Msg::new(
        "cmd.shaft.err.spline_no_se",
        "{label}：花键段不给 S/E（直径由规格代号导出）",
        "{label}: a spline segment takes no S/E (diameters come from the spec code)",
    ),
    Msg::new(
        "cmd.shaft.err.spline_no_spec",
        "{label}：关键字 SPLINE 缺少规格代号（写法 SPLINE 6x23x26x6 L30）",
        "{label}: keyword SPLINE is missing the spec code (write SPLINE 6x23x26x6 L30)",
    ),
    Msg::new(
        "cmd.shaft.err.spline_no_len",
        "{label}：花键段缺少 L（满齿段长，例 `SPLINE 6x23x26x6 L30`）",
        "{label}: spline segment is missing L (full-tooth segment length, e.g. `SPLINE 6x23x26x6 L30`)",
    ),
    Msg::new(
        "cmd.shaft.err.tlro_needs_m",
        "{label}：关键字 TL/RO/SD 是螺纹参数，需要与 M 同段（例 `M1.5 TL20 RO短 SD长`）",
        "{label}: TL/RO/SD are thread parameters and must share a segment with M (e.g. `M1.5 TL20 RO短 SD长`)",
    ),
    Msg::new(
        "cmd.shaft.err.m_rl_override",
        "{label}：M 段右端的 RL（螺纹收尾）按螺距查表，不支持 P/g1/g2/dg/r 覆盖；段级退刀槽请写在别的圆柱段上（或 `M…RL@L`）",
        "{label}: RL on the right of an M segment (thread run-out) is looked up by pitch and does not accept P/g1/g2/dg/r overrides; put a segment-level relief groove on another cylindrical segment (or use `M…RL@L`)",
    ),
    Msg::new(
        "cmd.shaft.err.rl_ro_exclusive",
        "{label}：RL（表 2 退刀槽收尾）与 RO/SD（螺尾/肩距）互斥，二选一",
        "{label}: RL (Table 2 relief-groove run-out) and RO/SD (thread run-out/shoulder distance) are mutually exclusive; pick one",
    ),
    Msg::new(
        "cmd.shaft.err.m_ov_conflict",
        "{label}：螺纹段 M 不能与越程槽 OV 同段",
        "{label}: a thread segment M cannot share a segment with an overtravel groove OV",
    ),
    Msg::new(
        "cmd.shaft.err.key_mrl_conflict",
        "{label}：轴槽 KEY 不能与螺纹 M / 越程槽 OV / 退刀槽 RL 同段（只能挂在光圆柱段上）",
        "{label}: a shaft key KEY cannot share a segment with thread M / overtravel groove OV / relief groove RL (it can only sit on a plain cylindrical segment)",
    ),
    Msg::new(
        "cmd.shaft.err.missing_s",
        "{label}：缺少 S（起始直径）",
        "{label}: missing S (start diameter)",
    ),
    Msg::new(
        "cmd.shaft.err.missing_l",
        "{label}：缺少 L（段长）",
        "{label}: missing L (segment length)",
    ),
    Msg::new(
        "cmd.shaft.err.gear_no_se",
        "{label}：齿轮段不给 S/E（直径由 M·Z 导出）",
        "{label}: a gear segment takes no S/E (diameters come from M·Z)",
    ),
    Msg::new(
        "cmd.shaft.err.gear_use_h",
        "{label}：齿轮段长度用 H（省略 = 10m），不要再给 L",
        "{label}: use H for gear-segment length (default 10m); do not give L",
    ),
    Msg::new(
        "cmd.shaft.err.gear_no_ch",
        "{label}：齿轮段不能与倒角 CH 同段（齿形用 OCSMGEAR 单独出）",
        "{label}: a gear segment cannot share a segment with chamfer CH (draw the tooth profile separately with OCSMGEAR)",
    ),
    Msg::new(
        "cmd.shaft.err.gear_no_ov",
        "{label}：齿轮段不能与越程槽 OV 同段",
        "{label}: a gear segment cannot share a segment with an overtravel groove OV",
    ),
    Msg::new(
        "cmd.shaft.err.gear_no_m",
        "{label}：齿轮段不能与螺纹段 M 同段",
        "{label}: a gear segment cannot share a segment with thread M",
    ),
    Msg::new(
        "cmd.shaft.err.gear_no_tlro",
        "{label}：齿轮段不能与螺纹参数 TL/RO/SD 同段（TL/RO/SD 只属于 M）",
        "{label}: a gear segment cannot share a segment with TL/RO/SD (those belong to M)",
    ),
    Msg::new(
        "cmd.shaft.err.gear_no_rl",
        "{label}：齿轮段不能与退刀槽 RL 同段",
        "{label}: a gear segment cannot share a segment with a relief groove RL",
    ),
    Msg::new(
        "cmd.shaft.err.gear_no_missing",
        "{label}：关键字 GEAR 缺少 M（模数）",
        "{label}: keyword GEAR is missing M (module)",
    ),
    Msg::new(
        "cmd.shaft.err.gear_no_z",
        "{label}：关键字 GEAR 缺少 Z（齿数）",
        "{label}: keyword GEAR is missing Z (tooth count)",
    ),
    Msg::new(
        "cmd.shaft.err.beta_spur",
        "{label}：关键字 BETA 的斜齿（β={beta}°）本期只做直齿（斜齿未实现）",
        "{label}: the BETA helical value (β={beta}°) is backed by spur gears only in this release (helical not implemented)",
    ),
    Msg::new(
        "cmd.shaft.err.gear_seg",
        "{label}：齿轮段：{e}",
        "{label}: gear segment: {e}",
    ),
    Msg::new(
        "cmd.shaft.err.place_dup",
        "{label}：放置参数 at/rot 重复",
        "{label}: placement parameter at/rot is repeated",
    ),
    Msg::new(
        "cmd.shaft.err.at_no_y",
        "{label}：at 缺 y 坐标（写 at x,y）",
        "{label}: at is missing the y coordinate (write at x,y)",
    ),
    Msg::new(
        "cmd.shaft.err.rot_no_value",
        "{label}：rot 缺少数值",
        "{label}: rot is missing a value",
    ),
    Msg::new(
        "cmd.shaft.err.place_unknown",
        "{label}：无法识别的词「{word}」（放置段只认 at x,y rot 度）",
        "{label}: unrecognized word \"{word}\" (a placement segment only accepts at x,y rot deg)",
    ),
    Msg::new(
        "cmd.shaft.err.prefix",
        "{label}：{e}",
        "{label}: {e}",
    ),
    Msg::new(
        "cmd.shaft.err.field_not_number",
        "{label}：{what}「{text}」不是数字",
        "{label}: {what} \"{text}\" is not a number",
    ),
    Msg::new(
        "cmd.shaft.err.key_place_invalid",
        "KEY 位置「{other}」非法（只有 @中 / @端，或 mid / end）",
        "invalid KEY position \"{other}\" (only @中/@端, or mid/end)",
    ),
    Msg::new(
        "cmd.shaft.err.key_kind_invalid",
        "KEY 键型「{other}」非法（只有 A / B / C；对应 GB/T 1096 平键族 A/B/C 型）",
        "invalid KEY style \"{other}\" (only A / B / C; the GB/T 1096 parallel-key types A/B/C)",
    ),
    Msg::new(
        "cmd.shaft.err.gb1095_no_d",
        "轴径 d={d} 不在 GB/T 1095 的 d 选型表（6…500）里，无法按轴径确定键尺寸 b×h",
        "shaft diameter d={d} is not in the GB/T 1095 d selection table (6…500), so the key size b×h cannot be determined from the shaft diameter",
    ),
    Msg::new(
        "cmd.shaft.err.gb1095_b_out",
        "轴径 d={d} 按 GB/T 1095 应配 b={b}，但超出平键族表范围（GB/T 1096 表 b=2…50）",
        "shaft diameter d={d} should pair with b={b} per GB/T 1095, but that is outside the parallel-key table (GB/T 1096 table b=2…50)",
    ),
    Msg::new(
        "cmd.shaft.err.csv_header_missing",
        "keyway_gb1095.csv 缺表头",
        "keyway_gb1095.csv is missing its header",
    ),
    Msg::new(
        "cmd.shaft.err.csv_header_bad",
        "keyway_gb1095.csv 表头异常：{header}",
        "keyway_gb1095.csv: unexpected header: {header}",
    ),
    Msg::new(
        "cmd.shaft.err.csv_missing_col",
        "keyway_gb1095.csv 缺列 {what}",
        "keyway_gb1095.csv is missing column {what}",
    ),
    Msg::new(
        "cmd.shaft.err.csv_col_num",
        "keyway_gb1095.csv 第 {i} 列 {what} 不是数字：{e}",
        "keyway_gb1095.csv column {i} ({what}) is not a number: {e}",
    ),
    Msg::new(
        "cmd.shaft.err.gb1095_table_prefix",
        "GB/T 1095 表：{e}",
        "GB/T 1095 table: {e}",
    ),
    Msg::new(
        "cmd.shaft.err.gb1095_no_b",
        "GB/T 1095 表里没有 b={b} 这一档（标准 b×h 档：2×2…100×50）",
        "the GB/T 1095 table has no b={b} step (standard b×h steps: 2×2…100×50)",
    ),
    Msg::new(
        "cmd.shaft.err.no_segments",
        "没有解析到任何轴段（至少给一段 `S… L…`）",
        "no shaft segments parsed (give at least one `S… L…` segment)",
    ),
    Msg::new(
        "cmd.shaft.err.rl_param_dup",
        "{label}：RL 参数 {key} 重复",
        "{label}: RL parameter {key} is repeated",
    ),
    Msg::new(
        "cmd.shaft.err.spline_de_dup",
        "{label}：花键的 de 覆盖重复",
        "{label}: spline de override is repeated",
    ),
    Msg::new(
        "cmd.shaft.err.key_gear_conflict",
        "{label}：轴槽 KEY 不能与齿轮/花键段同段",
        "{label}: a shaft key KEY cannot share a segment with a gear/spline segment",
    ),
    Msg::new(
        "cmd.shaft.err.key_1097_screw",
        "{label}：GB/T 1097 固定螺钉 M{d0}：{e}",
        "{label}: GB/T 1097 set screw M{d0}: {e}",
    ),
    Msg::new(
        "cmd.shaft.err.key_end_place_only",
        "{label}：端置轴槽只能开在首段（左端）或末段（右端）",
        "{label}: an end-placed shaft key can only be cut on the first (left) or last (right) segment",
    ),
    Msg::new(
        "cmd.shaft.err.m_gear_dup",
        "{label}：关键字 M 重复（GEAR 段里的 M 是模数；螺纹 M 不能与 GEAR 同段）",
        "{label}: keyword M is repeated (M in a GEAR segment is the module; a thread M cannot share a segment with GEAR)",
    ),
    Msg::new(
        "cmd.shaft.err.no_service",
        "OCSMSHAFT: 无法启动轴服务（宿主不支持 worker 请求）。",
        "OCSMSHAFT: cannot start the shaft service (host does not support worker requests).",
    ),
    Msg::new(
        "cmd.shaft.info.run_init",
        "OCSMSHAFT：先运行 OCSM 初始化，再打开轴生成器窗口。",
        "OCSMSHAFT: run the OCSM initializer first, then open the shaft generator window.",
    ),
    Msg::new(
        "cmd.shaft.info.run_init_direct",
        "OCSMSHAFT：先运行 OCSM（或点功能区「图幅」组里的 OCSM 初始化），再直接插轴。",
        "OCSMSHAFT: run OCSM first (or click the OCSM initializer in the \"图幅\" ribbon group), then insert the shaft directly.",
    ),
    Msg::new(
        "cmd.shaft.info.opened",
        "OCSM 轴生成器：已打开窗口（段表 + 行文本双向同步 + 实时预览）。点「生成到图纸」→ 回到图纸点击定位基点 → 移动光标旋转 → 再点击落定（可连续，Esc 结束）。",
        "OCSM shaft generator: window opened (segment table + two-way text sync + live preview). Click \"Generate to drawing\" → click in the drawing to set the base point → move to rotate → click again to place (repeatable; Esc to finish).",
    ),
    Msg::new(
        "cmd.shaft.info.already_open",
        "OCSM 轴生成器：窗口已打开（Alt+Tab 切换过去）。",
        "OCSM shaft generator: window is already open (switch to it with Alt+Tab).",
    ),
    Msg::new("cmd.shaft.place.what", "OCSM 轴", "OCSM shaft"),
    Msg::new(
        "cmd.shaft.place.where",
        "请在轴生成器窗口里点「生成到图纸」",
        "click \"Generate to drawing\" in the shaft generator window",
    ),
    Msg::new(
        "cmd.shaft.err.params_invalid",
        "OCSMSHAFT 参数无效：{message}",
        "OCSMSHAFT: invalid arguments: {message}",
    ),
    Msg::new(
        "cmd.shaft.info.report_written",
        "OCSMSHAFT：计算书已写入 {path}",
        "OCSMSHAFT: report written to {path}",
    ),
    Msg::new(
        "cmd.shaft.err.report_write",
        "OCSMSHAFT 计算书写文件失败：{e}",
        "OCSMSHAFT: failed to write the report file: {e}",
    ),
    Msg::new(
        "cmd.shaft.err.report",
        "OCSMSHAFT 计算书：{e}",
        "OCSMSHAFT report: {e}",
    ),
    Msg::new(
        "cmd.shaft.err.geometry",
        "OCSMSHAFT 几何非法：{message}",
        "OCSMSHAFT: invalid geometry: {message}",
    ),
    Msg::new("cmd.shaft.undo", "OCSMSHAFT 轴", "OCSMSHAFT shaft"),
    Msg::new(
        "cmd.shaft.info.generated",
        "OCSMSHAFT：已生成轴（{segments} 段，总长 {length}，最大 Ø{max}，{view} 视图，{count} 个图元）→ {layers}",
        "OCSMSHAFT: shaft generated ({segments} segments, total length {length}, max Ø{max}, {view} view, {count} entities) → {layers}",
    ),
    Msg::new("cmd.shaft.view.normal", "常规", "normal"),
    Msg::new("cmd.shaft.view.section", "剖视", "section"),
    Msg::new(
        "cmd.shaft.ready.missing_layers",
        "这张图还没跑过 OCSM 初始化（缺图层：{list}）—— 先执行 OCSM（建 10 个图层 + 线型 + 文字/标注样式），再生成轴；不然中心线会是实线白线。",
        "this drawing has not been through OCSM initialization (missing layers: {list}) — run OCSM first (it creates the 10 layers + linetypes + text/dimension styles), otherwise the centerline will be a solid white line.",
    ),
    Msg::new(
        "cmd.shaft.ready.center_linetype",
        "这张图还没跑过 OCSM 初始化（{layer} 没挂 CENTER2 点划线）—— 先执行 OCSM（建 10 个图层 + 线型 + 文字/标注样式），再生成轴；不然中心线会是实线白线。",
        "this drawing has not been through OCSM initialization ({layer} does not use the CENTER2 dash-dot linetype) — run OCSM first (it creates the 10 layers + linetypes + text/dimension styles), otherwise the centerline will be a solid white line.",
    ),
    Msg::new(
        "cmd.card.usage_line",
        "OCSMCARD 用法：`OCSMCARD <卡类型> …`（本期卡类型：{types}；不带参数 = 开图形界面）。\n* GB 花键参数表（内/外两张卡，一卡一方向）：`OCSMCARD 花键参数表 [std GB] <九字段表达式> [6H|5f] [dp 4.5] [root 平|圆] [at x,y] [rot 度]`（旧 `内/外` 记号兼容；外卡表下方 `花键参数表_外`）\n* 齿轮参数表：`OCSMCARD 齿轮参数表 <九字段表达式> [mate z₂] [dwg 图号] [grade 精度等级] [center a] [at x,y] [rot 度]`\n* ANSI 花键参数表（内/外 × 中/英 = 四张卡）：`OCSMCARD ANSI花键参数表_中文 <九字段表达式> [profile ANSI30R] [at x,y]`（外卡 `ANSI花键参数表_外_中文`；英文版换 `_英文`；旧写法 `内 P16 Z20` 兼容）\n* NF 内/外花键参数表（两张）：`OCSMCARD NF内花键参数表 <九字段表达式> [中心 外径|齿面] [根 平|圆] [配合 松动|滑动|固定|压] [at x,y]`（外卡 `OCSMCARD NF外花键参数表`；旧写法 `A300 M7.5 Z38` 兼容）\n* DIN 5480 内/外花键参数表（两张，单栏 Bild 6）：`OCSMCARD DIN花键参数表 <九字段表达式> [N9H] [ae …] [as …] [at x,y]`（外卡 `DIN花键参数表_外`；旧写法 `M3 Z38 B120 N9H W8f` 兼容）\n* 精简版（下拉「精简版」分组，11 张：GB/NF/DIN/ANSI×4/齿轮）：只列基本参数 + 主要测量量、**不含公差列**；参数/查表/公式与完整卡同源 —— `OCSMCARD <精简卡名> <对应完整卡参数>`（例 `OCSMCARD NF花键精简表_内 A300 M7.5 Z38`、`OCSMCARD 齿轮精简表 <九字段表达式>`）",
        "OCSMCARD usage: `OCSMCARD <card type> …` (card types for this release: {types}; no argument = open the GUI).\n* GB spline table (internal/external cards, one direction per card): `OCSMCARD 花键参数表 [std GB] <nine-field expression> [6H|5f] [dp 4.5] [root 平|圆] [at x,y] [rot deg]` (the old `内/外` token still works; the external card is `花键参数表_外` below the table)\n* Gear table: `OCSMCARD 齿轮参数表 <nine-field expression> [mate z₂] [dwg drawing-no] [grade precision-grade] [center a] [at x,y] [rot deg]`\n* ANSI spline table (internal/external × CN/EN = four cards): `OCSMCARD ANSI花键参数表_中文 <nine-field expression> [profile ANSI30R] [at x,y]` (external card `ANSI花键参数表_外_中文`; use `_英文` for English; the old `内 P16 Z20` form still works)\n* NF internal/external spline table (two cards): `OCSMCARD NF内花键参数表 <nine-field expression> [中心 外径|齿面] [根 平|圆] [配合 松动|滑动|固定|压] [at x,y]` (external card `OCSMCARD NF外花键参数表`; the old `A300 M7.5 Z38` form still works)\n* DIN 5480 internal/external spline table (two cards, single-column Bild 6): `OCSMCARD DIN花键参数表 <nine-field expression> [N9H] [ae …] [as …] [at x,y]` (external card `DIN花键参数表_外`; the old `M3 Z38 B120 N9H W8f` form still works)\n* Lite cards (the \"Lite\" group in the dropdown, 11 cards: GB/NF/DIN/ANSI×4/gear): only basic parameters + main measurements, **no tolerance columns**; parameters/lookups/formulas are the same source as the full cards — `OCSMCARD <lite card name> <full-card arguments>` (e.g. `OCSMCARD NF花键精简表_内 A300 M7.5 Z38`, `OCSMCARD 齿轮精简表 <nine-field expression>`)",
    ),
    Msg::new(
        "cmd.shaft.usage",
        "OCSMSHAFT 轴生成器：行 DSL / JSON → 单视图侧视图（段拼接 + 端面倒角 + 砂轮越程槽 + 螺纹段 M + 齿轮段 GEAR + 矩形花键段 SPLINE + 轴槽 KEY）。\n用法：OCSMSHAFT <行 DSL 或 JSON>\n  行 DSL：一行一段，从左到右拼接；多段用 | 或换行分隔；大小写不敏感、段内关键字顺序无关\n    S 起始直径（靠左）   E 终点直径（省略 = 圆柱段 E=S）   L 段长（必给；齿轮段用 H 代替）\n    CH2@L / CH2@R   端面倒角 C2（@ 省略默认 R）    OV / OV3 / OV3@L   砂轮越程槽\n    M / M1.5   螺纹段：不写值 = 小径 0.85d；M1.5 = 螺距 P，小径 = d − 1.0825P（只能圆柱段）\n               不给 TL/RL = 整段全螺纹（旧行为）\n    M1.5 TL20  局部螺纹（GB/T 3-1997 图 1 第一种形式）：完整螺纹长 TL，靠段右端台肩，\n               自右向左 = 台肩面 + 锥面（a−x）+ 螺尾（细实线，x）+ 分界竖线 + 完整螺纹 TL\n               RO一般|RO短 收尾档（默认一般）   SD一般|SD长|SD短 肩距档（默认一般）\n    M1.5 TL20 RL  用 GB/T 3-1997 表 2 退刀槽收尾（图 2 画法，与 RO/SD 互斥）\n    RL@L / RL@R   段级退刀槽（任意圆柱段，@ 省略默认 R；同端只能一个 CH/OV/RL）\n               取参：① g1/g2/dg/r 全给（dg 是绝对直径）② 给 P 查表 2 ③ 都不给报错\n               例：S25 E25 L32 RL@L P1.5 / RL@L g1 2.5 g2 4.5 dg 22.7 r 0.8\n               前置：该端相邻段更高（有台肩）、本段圆柱，否则报「第 N 段」\n    GEAR M5 Z10 H20   齿轮段（直齿）：d=m·z 导出、不给 S/E；H = 齿宽（省略 = 10m）\n                       ALPHA25 = 压力角 25°（省略 20°）；齿顶轮廓两端倒角 C=round(0.6m)\n                       常规不画齿根、剖视画齿根\n    SPLINE 6x23x26x6 L30   矩形花键段（GB/T 1144 规格代号）：大径线 + 小径细线\n                      （2细线层）+ 收尾弧 R=de/2（圆心 (L, ±(d/2+R))，末端 x=L+l，\n                      l=√(h(2R−h))，6×23×26×6 → l=9.6047）；段长 = L + l；\n                      不给 S/E；可 `de 71` 覆盖滚刀外径；不能与 CH/OV/RL/M/GEAR 同段\n                      （引入倒角由相邻段的 CH 表达）\n    KEY A 18      轴槽（GB/T 1095-2003 平键键槽，本期只做轴槽，不毂槽）——轴段类型，进段表 KEY 列：\n                  只能挂在光圆柱段上（与 GEAR/SPLINE/M/TL/RL/OV 互斥）\n                  键型（普通平键 A/B/C；导向平键 `导向A`/`导向B`——GUI 合并在一个下拉里）\n                  + 键长 L（所选键型；平键族标准系列，L<10b）+ 位置中置/端置（槽长自动折算）\n                  b×h 由本段直径 d 查 GB/T 1095 d 列自动定（h 跟 b 走）；t1 按 b 查 GB/T 1095 表\n                  显式覆盖写 b8h7：必须落在该轴径档的标准配对上，否则明确报错\n                  显示：中置恒显示 A；端置 B/C 显示 C（C 端弧由铣刀铣出）\n                  折算：中置 B +b / C +b/2；端置 B +b/2 / C 不折算；端置槽长再 +t1\n                  可选 双槽（DOUBLE 别名）：绕轴心 180° 对置，仅剖视图体现（常规侧视不变），\n                  可承受转矩约为单键联接的 1.5 倍；剖视剖面线上下两环各带一个缺口\n                  例：KEY A 18 ／ KEY C 14 @端 ／ KEY A 18 b8h7\n                  导向平键（GB/T 1097）用合并键型记号（推荐 `KEY 导向A 25`；旧 `KEY A 25 导向` 过时但可用）：\n                  只 A/B；L 取 1097 系列 25…450∩L<10b；槽长 = L（只有中置）；\n                  自动画 2 个固定螺钉螺纹孔 d0×L0、孔心距槽两端 L3；与双槽互斥；起键孔属 1096\n                  侧视图叠画键 + 剖视缺口含 sagitta 线；不生成尺寸标注\n    VIEW 常规|剖视   视图：常规（默认，只看外形）/ 剖视（轮廓 + ANSI31 剖面线）\n                     （双视图已于 2026-09-23 移除；旧 `VIEW 双` 明确报错）\n    REPORT          计算书：段末加 `REPORT`（大小写不敏感）—— 不插图，直接输出 Markdown\n                    计算书（段清单 + 总长/最大直径）；\n                    `REPORT=<路径>` / `REPORT-OUT=<路径>` 另写文件\n    at x,y rot 度   放置（不写 = 原点、不转）\n  例：OCSMSHAFT S30 E30 L45 CH2@L | S40 E40 L30 CH2@R OV3 | S50 E30 L20 | S30 E30 L15 CH2@R | S40 E40 L7 M1.5 | S36 E36 L5 | GEAR M3 Z20 VIEW 剖视 at 100,50 rot 30\n  JSON：{\"segments\":[{\"s\":30,\"e\":30,\"l\":45,\"ch\":[{\"c\":2,\"end\":\"L\"}]},{\"s\":30,\"e\":30,\"l\":20,\"thread\":1.5}],\"view\":\"section\",\"at\":[100,50],\"rot\":30}\n轮廓/端面/倒角/槽与边界竖线 → 1轮廓实线层，螺纹小径/螺尾 → 2细线层（OV 不画砂轮细线），轴线/分度线 → 3中心线层，剖视剖面线 → 5剖面线层。",
        "OCSMSHAFT shaft generator: line DSL / JSON → single-view side view (segment concatenation + end chamfers + grinding overtravel grooves + thread segment M + gear segment GEAR + rectangular spline segment SPLINE + shaft key KEY).\nUsage: OCSMSHAFT <line DSL or JSON>\n  Line DSL: one segment per line, concatenated left to right; separate segments with | or newlines; case-insensitive, keyword order within a segment does not matter\n    S start diameter (left side)   E end diameter (omitted = cylindrical segment with E=S)   L segment length (required; gear segments use H instead)\n    CH2@L / CH2@R   end chamfer C2 (@ omitted defaults to R)    OV / OV3 / OV3@L   grinding overtravel groove\n    M / M1.5   thread segment: no value = minor diameter 0.85d; M1.5 = pitch P, minor diameter = d − 1.0825P (cylindrical segments only)\n               no TL/RL = fully threaded segment (old behaviour)\n    M1.5 TL20   partial thread (GB/T 3-1997 Fig. 1, first form): full thread length TL next to the right shoulder,\n                drawn right to left = shoulder face + taper (a−x) + thread run-out (thin line, x) + boundary line + full thread TL\n               RO一般|RO短 run-out class (default 一般)   SD一般|SD长|SD短 shoulder-distance class (default 一般)\n    M1.5 TL20 RL   use the GB/T 3-1997 Table 2 relief-groove run-out (Fig. 2; mutually exclusive with RO/SD)\n    RL@L / RL@R   segment-level relief groove (any cylindrical segment; @ omitted defaults to R; only one CH/OV/RL per end)\n               parameters: ① give all of g1/g2/dg/r (dg is an absolute diameter) ② give P to look up Table 2 ③ give none → error\n               e.g. S25 E25 L32 RL@L P1.5 / RL@L g1 2.5 g2 4.5 dg 22.7 r 0.8\n               precondition: the neighbouring segment on that end is larger (there is a shoulder) and this segment is cylindrical, otherwise it reports \"segment N\"\n    GEAR M5 Z10 H20   gear segment (spur): d=m·z derived, no S/E; H = face width (omitted = 10m)\n                       ALPHA25 = pressure angle 25° (omitted 20°); both ends of the tip profile get a chamfer C=round(0.6m)\n                       normal view omits the root; section view draws the root\n    SPLINE 6x23x26x6 L30   rectangular spline segment (GB/T 1144 spec code): major-diameter line + thin minor-diameter line\n                      (2细线层) + run-out arc R=de/2 (centre (L, ±(d/2+R)), end x=L+l,\n                      l=√(h(2R−h)), 6×23×26×6 → l=9.6047); segment length = L + l;\n                      no S/E; `de 71` overrides the hob outer diameter; cannot share a segment with CH/OV/RL/M/GEAR\n                      (the lead-in chamfer is expressed by CH on a neighbouring segment)\n    KEY A 18      shaft key (GB/T 1095-2003 parallel-key seat; only shaft keys for now, no hub keys) — a shaft-segment type, shown in the segment table KEY column:\n                  only on plain cylindrical segments (mutually exclusive with GEAR/SPLINE/M/TL/RL/OV)\n                  key style (normal parallel keys A/B/C; guided parallel keys `导向A`/`导向B` — merged into one GUI dropdown)\n                  + key length L (per style; standard parallel-key series, L<10b) + position centred/end (slot length auto-converted)\n                  b×h is determined from this segment diameter d via the GB/T 1095 d column (h follows b); t1 from the GB/T 1095 table by b\n                  explicit override b8h7: must be a standard pair for that shaft diameter, otherwise it reports an error\n                  display: centred always shows A; end B/C shows C (the C end arc is milled)\n                  conversion: centred B +b / C +b/2; end B +b/2 / C no conversion; end slot length adds t1\n                  optional 双槽 (alias DOUBLE): two opposed slots 180° apart, only visible in section (normal side view unchanged),\n                  torque capacity about 1.5× a single key; the section hatching has one gap in each of its two rings\n                  e.g. KEY A 18 / KEY C 14 @端 / KEY A 18 b8h7\n                  guided parallel keys (GB/T 1097) use the merged style token (recommended `KEY 导向A 25`; the old `KEY A 25 导向` is outdated but works):\n                  only A/B; L from the 1097 series 25…450∩L<10b; slot length = L (centred only);\n                  automatically draws two set-screw threaded holes d0×L0 with centres L3 from the slot ends; mutually exclusive with 双槽; the key-removal hole belongs to 1096\n                  the side view overlays the key + the section gap includes the sagitta line; no dimensions are generated\n    VIEW 常规|剖视   view: normal (default, outline only) / section (outline + ANSI31 hatching)\n                     (dual view was removed on 2026-09-23; the old `VIEW 双` reports an explicit error)\n    REPORT          report: append `REPORT` at the end of a segment (case-insensitive) — no drawing, just prints the Markdown\n                    report (segment list + total length/max diameter);\n                    `REPORT=<path>` / `REPORT-OUT=<path>` also writes a file\n    at x,y rot deg   placement (omitted = origin, no rotation)\n  Example: OCSMSHAFT S30 E30 L45 CH2@L | S40 E40 L30 CH2@R OV3 | S50 E30 L20 | S30 E30 L15 CH2@R | S40 E40 L7 M1.5 | S36 E36 L5 | GEAR M3 Z20 VIEW 剖视 at 100,50 rot 30\n  JSON: {\"segments\":[{\"s\":30,\"e\":30,\"l\":45,\"ch\":[{\"c\":2,\"end\":\"L\"}]},{\"s\":30,\"e\":30,\"l\":20,\"thread\":1.5}],\"view\":\"section\",\"at\":[100,50],\"rot\":30}\nOutline/end faces/chamfers/grooves and boundary lines → 1轮廓实线层; thread minor diameter/run-out → 2细线层 (OV does not draw the grinding thin line); axis/pitch line → 3中心线层; section hatching → 5剖面线层.",
    ),
    Msg::new(
        "cmd.shaft.err.host_prefix",
        "OCSMSHAFT: {msg}",
        "OCSMSHAFT: {msg}",
    ),
    Msg::new("cmd.shaft.end.l", "左", "left"),
    Msg::new("cmd.shaft.end.r", "右", "right"),

    Msg::new(
        "cmd.shaft.label.seg",
        "第 {n} 段",
        "segment {n}",
    ),
    Msg::new(
        "cmd.shaft.err.view_double_removed",
        "不识别的视图名「{name}」：双视图已移除（用户 2026-09-23 定案）——轴生成器只支持 常规/剖视 两个视图；原 `VIEW 双` 的并排输出已撤（需要并排请分别出两次图）。",
        "unrecognized view name \"{name}\": the dual view was removed (decided 2026-09-23) — the shaft generator only supports normal/section; the old `VIEW 双` side-by-side output was withdrawn (run the command twice if you need them side by side).",
    ),
    Msg::new(
        "cmd.shaft.place.mid",
        "中置",
        "centred",
    ),
    Msg::new(
        "cmd.shaft.place.end",
        "端置",
        "end-placed",
    ),
    Msg::new(
        "cmd.shaft.keykind.a",
        "A型（双圆头）",
        "type A (round both ends)",
    ),
    Msg::new(
        "cmd.shaft.keykind.b",
        "B型（双平头）",
        "type B (flat both ends)",
    ),
    Msg::new(
        "cmd.shaft.keykind.c",
        "C型（单圆头）",
        "type C (one round end)",
    ),
    Msg::new(
        "cmd.shaft.err.gb1095_b_pair",
        "轴径 d={p0} 按 GB/T 1095 应配 b={p1}×h={p2}；显式 b={p3} 不是该轴径档的标准键尺寸（不许自由组合）",
        "shaft diameter d={p0} should pair with b={p1}×h={p2} per GB/T 1095; the explicit b={p3} is not a standard key size for that diameter step (free combinations are not allowed)",
    ),
    Msg::new(
        "cmd.shaft.err.thread_str_not_number",
        "thread 字符串「{text}」不是螺距数字",
        "thread string \"{text}\" is not a pitch number",
    ),
    Msg::new(
        "cmd.shaft.err.json_parse",
        "JSON 解析失败：{e}",
        "JSON parse failed: {e}",
    ),
    Msg::new(
        "cmd.shaft.err.json_ch_end_bad",
        "{label}：ch[{k}] 的端别「{bad}」非法（只能用 L 或 R）",
        "{label}: end \"{bad}\" of ch[{k}] is invalid (only L or R)",
    ),
    Msg::new(
        "cmd.shaft.err.json_ch_dup_end",
        "{label}：ch 在{p0}端重复",
        "{label}: ch is repeated at the {p0} end",
    ),
    Msg::new(
        "cmd.shaft.err.json_ov_end_bad",
        "{label}：ov[{k}] 的端别「{bad}」非法（只能用 L 或 R）",
        "{label}: end \"{bad}\" of ov[{k}] is invalid (only L or R)",
    ),
    Msg::new(
        "cmd.shaft.err.json_ov_dup_end",
        "{label}：ov 在{p0}端重复",
        "{label}: ov is repeated at the {p0} end",
    ),
    Msg::new(
        "cmd.shaft.err.json_relief_end_bad",
        "{label}：relief[{k}] 的端别「{bad}」非法（只能用 L 或 R）",
        "{label}: end \"{bad}\" of relief[{k}] is invalid (only L or R)",
    ),
    Msg::new(
        "cmd.shaft.err.json_relief_dup_end",
        "{label}：relief 在{p0}端重复",
        "{label}: relief is repeated at the {p0} end",
    ),
    Msg::new(
        "cmd.shaft.err.json_thread_relief",
        "{label}：M 段右端的退刀槽收尾用 thread.relief 表示，不要再写段级 relief（端别 R）",
        "{label}: the right-end thread run-out of an M segment is expressed by thread.relief; do not add a segment-level relief (end R)",
    ),
    Msg::new(
        "cmd.shaft.err.spline_no_n",
        "{label}：花键段缺 n（齿数）",
        "{label}: spline segment is missing n (tooth count)",
    ),
    Msg::new(
        "cmd.shaft.err.spline_no_d",
        "{label}：花键段缺 d（小径）",
        "{label}: spline segment is missing d (minor diameter)",
    ),
    Msg::new(
        "cmd.shaft.err.spline_no_big",
        "{label}：花键段缺 big（大径 D）",
        "{label}: spline segment is missing big (major diameter D)",
    ),
    Msg::new(
        "cmd.shaft.err.spline_no_b",
        "{label}：花键段缺 b（键宽 B）",
        "{label}: spline segment is missing b (key width B)",
    ),
    Msg::new(
        "cmd.shaft.err.spline_de_not_found",
        "{label}：花键段 de 查不到（不在表 1/表 2），请显式给 de",
        "{label}: spline de not found (not in Table 1/2); give de explicitly",
    ),
    Msg::new(
        "cmd.shaft.err.spline_s_mismatch",
        "{label}：花键段的 s={p0} 应等于大径 D={p1}",
        "{label}: spline s={p0} must equal the major diameter D={p1}",
    ),
    Msg::new(
        "cmd.shaft.err.spline_e_mismatch",
        "{label}：花键段的 e={p0} 应等于大径 D={p1}",
        "{label}: spline e={p0} must equal the major diameter D={p1}",
    ),
    Msg::new(
        "cmd.shaft.err.spline_l_mismatch",
        "{label}：花键段的 l={p0} 应等于 L+l={p1}（L={p2} + 收尾 {p3})），",
        "{label}: spline l={p0} must equal L+l={p1} (L={p2} + run-out {p3})",
    ),
    Msg::new(
        "cmd.shaft.err.json_gear_kind_bad",
        "{label}：齿轮段的 kind「{k}」非法（应为 external/internal）",
        "{label}: invalid gear segment kind \"{k}\" (use external/internal)",
    ),
    Msg::new(
        "cmd.shaft.err.json_gear_helical",
        "{label}：齿轮段斜齿（beta={p0}）本期只做直齿（斜齿未实现）",
        "{label}: helical gear segment (beta={p0}) is spur-only in this release (helical not implemented)",
    ),
    Msg::new(
        "cmd.shaft.err.json_gear_s_mismatch",
        "{label}：齿轮段的 s={p0} 应等于分度圆 d={p1}（由 m·z 导出）",
        "{label}: gear s={p0} must equal the pitch diameter d={p1} (derived from m·z)",
    ),
    Msg::new(
        "cmd.shaft.err.json_gear_e_mismatch",
        "{label}：齿轮段的 e={p0} 应等于分度圆 d={p1}（由 m·z 导出）",
        "{label}: gear e={p0} must equal the pitch diameter d={p1} (derived from m·z)",
    ),
    Msg::new(
        "cmd.shaft.err.json_gear_l_mismatch",
        "{label}：齿轮段的 l={p0} 应等于齿宽 h={p1}",
        "{label}: gear l={p0} must equal the face width h={p1}",
    ),
    Msg::new(
        "cmd.shaft.err.seg_len_positive",
        "{label}：段长 L={p0} 必须 > 0",
        "{label}: segment length L={p0} must be > 0",
    ),
    Msg::new(
        "cmd.shaft.err.seg_s_positive",
        "{label}：起始直径 S={p0} 必须 > 0",
        "{label}: start diameter S={p0} must be > 0",
    ),
    Msg::new(
        "cmd.shaft.err.seg_e_positive",
        "{label}：终点直径 E={p0} 必须 > 0",
        "{label}: end diameter E={p0} must be > 0",
    ),
    Msg::new(
        "cmd.shaft.err.chamfer_positive",
        "{label}：倒角 C={p0} 必须 > 0",
        "{label}: chamfer C={p0} must be > 0",
    ),
    Msg::new(
        "cmd.shaft.err.chamfer_len_overlap",
        "{label}：{p0}端倒角 C={p1} ≥ 段长/2（l={p2}），特征重叠",
        "{label}: {p0}-end chamfer C={p1} ≥ half the segment length (l={p2}); features overlap",
    ),
    Msg::new(
        "cmd.shaft.err.chamfer_dup_end",
        "{label}：倒角 CH 在{p0}端重复",
        "{label}: chamfer CH is repeated at the {p0} end",
    ),
    Msg::new(
        "cmd.shaft.err.ov_b1_positive",
        "{label}：越程槽 b1={p0} 必须 > 0",
        "{label}: overtravel groove b1={p0} must be > 0",
    ),
    Msg::new(
        "cmd.shaft.err.ov_b1_too_long",
        "{label}：越程槽 b1={p0} > 段长 l={p1}",
        "{label}: overtravel groove b1={p0} > segment length l={p1}",
    ),
    Msg::new(
        "cmd.shaft.err.relief_dup_end",
        "{label}：退刀槽 RL 重复（在{p0}端）",
        "{label}: relief groove RL is repeated (at the {p0} end)",
    ),
    Msg::new(
        "cmd.shaft.err.rl_cone_only",
        "{label}：RL 退刀槽只能开在圆柱端（本段是锥面 S={p0} → E={p1}）",
        "{label}: an RL relief groove can only be cut on a cylindrical end (this segment is a cone S={p0} → E={p1})",
    ),
    Msg::new(
        "cmd.shaft.err.rl_free_end_r",
        "{label}：右端是自由端，退刀槽没有台阶面",
        "{label}: the right end is free; a relief groove has no shoulder face",
    ),
    Msg::new(
        "cmd.shaft.err.rl_g2_too_long",
        "{label}：RL 退刀槽 g2={p0} > 段长 L={p1}",
        "{label}: RL relief groove g2={p0} > segment length L={p1}",
    ),
    Msg::new(
        "cmd.shaft.err.rl_ch_conflict",
        "{label}：{p0}端的 RL 退刀槽与倒角 CH 冲突（同端只能一个）",
        "{label}: the RL relief groove on the {p0} end conflicts with chamfer CH (only one per end)",
    ),
    Msg::new(
        "cmd.shaft.err.rl_ov_conflict",
        "{label}：{p0}端的 RL 退刀槽与越程槽 OV 冲突（同端只能一个）",
        "{label}: the RL relief groove on the {p0} end conflicts with overtravel groove OV (only one per end)",
    ),
    Msg::new(
        "cmd.shaft.err.key_round_only",
        "{label}：轴槽只能开在圆柱段（本段是锥面 S={p0} → E={p1}）",
        "{label}: a shaft key can only be cut on a cylindrical segment (this one is a cone S={p0} → E={p1})",
    ),
    Msg::new(
        "cmd.shaft.err.key_len_positive",
        "{label}：轴槽键长 L={p0} 必须 > 0",
        "{label}: shaft-key length L={p0} must be > 0",
    ),
    Msg::new(
        "cmd.shaft.err.key_pair_invalid",
        "{label}：键尺寸 b{p0}×h{p1} 不是标准配对（{p2} 的 b={p3} 应配 h={p4}）",
        "{label}: key size b{p0}×h{p1} is not a standard pair ({p2} with b={p3} should pair with h={p4})",
    ),
    Msg::new(
        "cmd.shaft.err.key_1097_len_bad",
        "{label}：GB/T 1097 导向平键 L={p0} 不在长度系列（∩L<10b）里；可选（{p1} 档）：{p2}",
        "{label}: GB/T 1097 guided-key L={p0} is not in the length series (∩L<10b); available ({p1} steps): {p2}",
    ),
    Msg::new(
        "cmd.shaft.err.key_screw_out_slot",
        "{label}：导向平键固定螺钉孔 M{p0}（半径 {p1}）越出槽端（L3={p2}）",
        "{label}: guided-key set-screw hole M{p0} (radius {p1}) falls outside the slot end (L3={p2})",
    ),
    Msg::new(
        "cmd.shaft.err.key_screw_out_profile",
        "{label}：导向平键固定螺钉孔 M{p0} 越出 A 型圆头轮廓（L={p1}、b={p2}、L3={p3}）",
        "{label}: guided-key set-screw hole M{p0} falls outside the type-A round-end profile (L={p1}, b={p2}, L3={p3})",
    ),
    Msg::new(
        "cmd.shaft.err.keyway_prefix",
        "{label}：轴槽：{e}",
        "{label}: shaft key: {e}",
    ),
    Msg::new(
        "cmd.shaft.err.key_b_ge_d",
        "{label}：轴槽键宽 b={p0} ≥ 轴径 d={p1}（槽切穿轴）",
        "{label}: shaft-key width b={p0} ≥ shaft diameter d={p1} (the slot cuts through the shaft)",
    ),
    Msg::new(
        "cmd.shaft.err.key_t1_positive",
        "{label}：轴槽槽深 t1={p0} 必须 > 0",
        "{label}: shaft-key depth t1={p0} must be > 0",
    ),
    Msg::new(
        "cmd.shaft.err.key_t1_le_sagitta",
        "{label}：轴槽槽深 t1={p0} ≤ sagitta={p1}（槽底没切到圆柱下）",
        "{label}: shaft-key depth t1={p0} ≤ sagitta={p1} (the slot bottom does not cut below the cylinder)",
    ),
    Msg::new(
        "cmd.shaft.err.key_t1_ge_radius",
        "{label}：轴槽槽深 t1={p0} ≥ 轴半径 R={p1}（槽切过轴线，剖视上下环分区不成立）",
        "{label}: shaft-key depth t1={p0} ≥ shaft radius R={p1} (the slot cuts through the axis; the section's two hatched rings no longer hold)",
    ),
    Msg::new(
        "cmd.shaft.err.key_screw_through_axis",
        "{label}：固定螺钉孔深 L0={p0} ＋t1={p1}＋钻尖 {p2} 越过轴线（R={p3}）",
        "{label}: set-screw hole depth L0={p0} + t1={p1} + drill point {p2} crosses the axis (R={p3})",
    ),
    Msg::new(
        "cmd.shaft.err.key_mid_no_fit",
        "{label}：中置轴槽（键长 L={p0}）在净圆柱段（倒角根↔段末）{p1} 内装不下",
        "{label}: centred keyway (key length L={p0}) does not fit in the clean cylindrical run (chamfer root↔segment end) {p1}",
    ),
    Msg::new(
        "cmd.shaft.err.key_single_end_ambiguous",
        "{label}：单段轴的端置轴槽开口端不唯一（改中置 `KEY A 18` 或拆段）",
        "{label}: on a single-segment shaft the open end of an end-placed keyway is ambiguous (use a centred `KEY A 18` or split the shaft)",
    ),
    Msg::new(
        "cmd.shaft.err.key_slot_len_too_long",
        "{label}：端置轴槽槽长（L + t1 = {p0} + {p1} = {p2}）> 段长 l={p3}",
        "{label}: end-placed keyway slot length (L + t1 = {p0} + {p1} = {p2}) > segment length l={p3}",
    ),
    Msg::new(
        "cmd.shaft.err.key_t1_lt_chamfer",
        "{label}：端置轴槽 t1={p0} < {p1}端倒角 C={p2}（槽没切穿倒角，本期不支持）",
        "{label}: end-placed keyway t1={p0} < {p1}-end chamfer C={p2} (the slot does not cut through the chamfer; not supported in this release)",
    ),
    Msg::new(
        "cmd.shaft.err.m_pitch_positive",
        "{label}：螺纹螺距 P={p0} 必须 > 0",
        "{label}: thread pitch P={p0} must be > 0",
    ),
    Msg::new(
        "cmd.shaft.err.m_pitch_too_big",
        "{label}：螺距 P={p0} 太大（小径 d−1.0825P={p1} ≤ 0）",
        "{label}: pitch P={p0} is too large (minor diameter d−1.0825P={p1} ≤ 0)",
    ),
    Msg::new(
        "cmd.shaft.err.m_round_only",
        "{label}：螺纹段必须是圆柱（S==E，当前 S={p0}、E={p1}）",
        "{label}: a thread segment must be cylindrical (S==E; currently S={p0}, E={p1})",
    ),
    Msg::new(
        "cmd.shaft.err.m_gear_conflict",
        "{label}：螺纹段 M 不能与齿轮段 GEAR 同段",
        "{label}: a thread segment M cannot share a segment with a gear segment GEAR",
    ),
    Msg::new(
        "cmd.shaft.err.tlro_need_pitch",
        "{label}：局部螺纹 TL/RL 必须给螺距（写法 M1.5 TL20；表 1/表 2 都按螺距查）",
        "{label}: partial thread TL/RL requires a pitch (write M1.5 TL20; both Table 1 and Table 2 are keyed by pitch)",
    ),
    Msg::new(
        "cmd.shaft.err.tl_positive_plain",
        "{label}：完整螺纹长度 TL 必须 > 0",
        "{label}: full thread length TL must be > 0",
    ),
    Msg::new(
        "cmd.shaft.err.rl_dg_nonpositive",
        "{label}：退刀槽 dg = d − {p0} = {p1} ≤ 0（螺纹直径太小）",
        "{label}: relief groove dg = d − {p0} = {p1} ≤ 0 (the thread diameter is too small)",
    ),
    Msg::new(
        "cmd.shaft.err.rl_dg_too_shallow",
        "{label}：退刀槽底 dg={p0} 不低于螺纹小径 d1={p1}（槽没切进牙底）",
        "{label}: relief-groove bottom dg={p0} is not below the thread minor diameter d1={p1} (the groove does not reach the thread root)",
    ),
    Msg::new(
        "cmd.shaft.err.rl_g2_tl_too_long",
        "{label}：退刀槽 g2={p0} + 完整螺纹 TL={p1} = {p2} 超过段长 L={p3}",
        "{label}: relief groove g2={p0} + full thread TL={p1} = {p2} exceeds segment length L={p3}",
    ),
    Msg::new(
        "cmd.shaft.err.ro_gt_sd",
        "{label}：收尾 x={p0}（RO）大于肩距 a={p1}（SD），档位不搭（把 SD 调大或 RO 调短）",
        "{label}: run-out x={p0} (RO) exceeds shoulder distance a={p1} (SD); the classes do not match (increase SD or shorten RO)",
    ),
    Msg::new(
        "cmd.shaft.err.sd_tl_too_long",
        "{label}：肩距 a={p0} + 完整螺纹 TL={p1} = {p2} 超过段长 L={p3}",
        "{label}: shoulder distance a={p0} + full thread TL={p1} = {p2} exceeds segment length L={p3}",
    ),
    Msg::new(
        "cmd.shaft.err.tlro_no_chamfer_v",
        "{label}：局部螺纹 TL/RL 的右端不能倒角 CH（会吃掉锥面/退刀槽的台肩角）",
        "{label}: the right end of a partial thread TL/RL cannot take a chamfer CH (it would eat the cone/relief-groove shoulder corner)",
    ),
    Msg::new(
        "cmd.shaft.err.ro_sd_need_tl",
        "{label}：RO/SD 只在局部螺纹（给了 TL）时有意义——不给 TL = 整段全螺纹",
        "{label}: RO/SD only make sense for a partial thread (TL given) — no TL means the whole segment is threaded",
    ),
    Msg::new(
        "cmd.shaft.err.spline_se_mismatch",
        "{label}：花键段的 S/E={p0}/{p1} 应等于大径 D={p2}（由规格导出）",
        "{label}: spline S/E={p0}/{p1} must equal the major diameter D={p2} (derived from the spec)",
    ),
    Msg::new(
        "cmd.shaft.err.spline_len_mismatch",
        "{label}：花键段段长 {p0} 应等于 L+l={p1}（L={p2} + 收尾 {p3}）",
        "{label}: spline segment length {p0} must equal L+l={p1} (L={p2} + run-out {p3})",
    ),
    Msg::new(
        "cmd.shaft.err.spline_gear_conflict",
        "{label}：花键段不能与齿轮段 GEAR 同段",
        "{label}: a spline segment cannot share a segment with a gear segment GEAR",
    ),
    Msg::new(
        "cmd.shaft.err.tlro_rl_need_pitch",
        "{label}：局部螺纹 RL 必须给螺距（写法 M1.5 TL20 RL）",
        "{label}: partial thread RL requires a pitch (write M1.5 TL20 RL)",
    ),
    Msg::new(
        "cmd.shaft.err.tlro_tl_need_pitch",
        "{label}：局部螺纹 TL 必须给螺距（写法 M1.5 TL20）",
        "{label}: partial thread TL requires a pitch (write M1.5 TL20)",
    ),
    Msg::new(
        "cmd.shaft.err.gear_adjacent_covers_tip",
        "{label}：齿轮段相邻第 {p0} 段 Ø{p1} 大于齿顶圆 Ø{p2}，会盖住齿顶线（相邻段半径必须 ≤ 齿顶圆半径）",
        "{label}: the adjacent segment {p0} Ø{p1} is larger than the tip circle Ø{p2} and would cover the tip line (adjacent radius must be ≤ tip-circle radius)",
    ),
    Msg::new(
        "cmd.shaft.err.chamfer_same_face",
        "{label}：右端倒角与第 {p0} 段左端倒角落在同一端面，特征重叠",
        "{label}: the right-end chamfer and the left-end chamfer of segment {p0} land on the same end face; features overlap",
    ),
    Msg::new(
        "cmd.shaft.err.ov_same_face",
        "{label}：右端与第 {p0} 段左端都写了越程槽（同一端面只能一侧）",
        "{label}: an overtravel groove is written on both the right end and the left end of segment {p0} (only one side per end face)",
    ),
    Msg::new(
        "cmd.shaft.err.one_relief_per_face",
        "{label}：同一端面只能有一个退刀槽（RL）",
        "{label}: only one relief groove (RL) per end face",
    ),
    Msg::new(
        "cmd.shaft.err.rl_no_chamfer",
        "{label}：退刀槽 RL 所在端面不能倒角 CH（同端只能有一个槽/倒角）",
        "{label}: the end face with relief groove RL cannot take a chamfer CH (only one groove/chamfer per end)",
    ),
    Msg::new(
        "cmd.shaft.err.rl_ov_same_face",
        "{label}：退刀槽 RL 与越程槽 OV 在同一端面冲突（同端只能有一个槽/倒角）",
        "{label}: relief groove RL and overtravel groove OV conflict on the same end face (only one groove/chamfer per end)",
    ),
    Msg::new(
        "cmd.shaft.err.no_face_for_chamfer",
        "{label}：{p0}端没有端面（相邻段直径相同），无法倒角",
        "{label}: no end face on the {p0} end (the neighbouring segment has the same diameter), so no chamfer is possible",
    ),
    Msg::new(
        "cmd.shaft.err.chamfer_eats_face",
        "{label}：{p0}端倒角 C={p1} ≥ 端面直径变化量的一半（Ø{p2} → Ø{p3} 的 {p4}），端面被吃掉",
        "{label}: {p0}-end chamfer C={p1} ≥ half the end-face diameter step (Ø{p2} → Ø{p3}, step {p4}); the end face is eaten away",
    ),
    Msg::new(
        "cmd.shaft.err.chamfer_on_gear",
        "{label}：{p0}端倒角会落在第 {p1} 段齿轮段上（齿轮段不能倒角）",
        "{label}: the {p0}-end chamfer would land on gear segment {p1} (gear segments cannot take a chamfer)",
    ),
    Msg::new(
        "cmd.shaft.err.chamfer_on_spline",
        "{label}：倒角会落在花键段右端（收尾弧占有该端）—— 引入倒角请写在花键左端",
        "{label}: the chamfer would land on the right end of a spline segment (its run-out arc owns that end) — put the lead-in chamfer on the spline's left end",
    ),
    Msg::new(
        "cmd.shaft.err.chamfer_face_overlap",
        "{label}：{p0}端倒角与相邻面重叠（端面点 {p1} ≤ {p2}）",
        "{label}: {p0}-end chamfer overlaps the neighbouring face (face point {p1} ≤ {p2})",
    ),
    Msg::new(
        "cmd.shaft.err.ov_no_step_r",
        "{label}：右端越程槽没有台阶面（相邻段 Ø{p0} 不大于本段 Ø{p1}）",
        "{label}: right-end overtravel groove has no shoulder face (neighbouring Ø{p0} is not larger than this segment Ø{p1})",
    ),
    Msg::new(
        "cmd.shaft.err.ov_cone_r",
        "{label}：右端是锥面，越程槽只能开在圆柱端",
        "{label}: the right end is a cone; an overtravel groove can only be cut on a cylindrical end",
    ),
    Msg::new(
        "cmd.shaft.err.ov_prefix_r",
        "{label}：右端越程槽：{e}",
        "{label}: right-end overtravel groove: {e}",
    ),
    Msg::new(
        "cmd.shaft.err.ov_r_tangent_r",
        "{label}：右端越程槽的 R 圆角切点 {p0} 不低于台阶面 {p1}（相邻台阶太小）",
        "{label}: the R-fillet tangent point {p0} of the right-end overtravel groove is not below the shoulder face {p1} (neighbouring shoulder too small)",
    ),
    Msg::new(
        "cmd.shaft.err.ov_no_step_l",
        "{label}：左端越程槽没有台阶面（相邻段 Ø{p0} 不大于本段 Ø{p1}）",
        "{label}: left-end overtravel groove has no shoulder face (neighbouring Ø{p0} is not larger than this segment Ø{p1})",
    ),
    Msg::new(
        "cmd.shaft.err.ov_cone_l",
        "{label}：左端是锥面，越程槽只能开在圆柱端",
        "{label}: the left end is a cone; an overtravel groove can only be cut on a cylindrical end",
    ),
    Msg::new(
        "cmd.shaft.err.ov_prefix_l",
        "{label}：左端越程槽：{e}",
        "{label}: left-end overtravel groove: {e}",
    ),
    Msg::new(
        "cmd.shaft.err.ov_r_tangent_l",
        "{label}：左端越程槽的 R 圆角切点 {p0} 不低于台阶面 {p1}（相邻台阶太小）",
        "{label}: the R-fillet tangent point {p0} of the left-end overtravel groove is not below the shoulder face {p1} (neighbouring shoulder too small)",
    ),
    Msg::new(
        "cmd.shaft.err.rl_no_step_r",
        "{label}：右端退刀槽没有台阶面（相邻段 Ø{p0} 不大于本段 Ø{p1}）",
        "{label}: right-end relief groove has no shoulder face (neighbouring Ø{p0} is not larger than this segment Ø{p1})",
    ),
    Msg::new(
        "cmd.shaft.err.rl_r_tangent_r",
        "{label}：右端退刀槽的 R 圆角切点 {p0} 不低于台阶面 {p1}（相邻台阶太小）",
        "{label}: the R-fillet tangent point {p0} of the right-end relief groove is not below the shoulder face {p1} (neighbouring shoulder too small)",
    ),
    Msg::new(
        "cmd.shaft.err.rl_no_step_l",
        "{label}：左端退刀槽没有台阶面（相邻段 Ø{p0} 不大于本段 Ø{p1}）",
        "{label}: left-end relief groove has no shoulder face (neighbouring Ø{p0} is not larger than this segment Ø{p1})",
    ),
    Msg::new(
        "cmd.shaft.err.rl_r_tangent_l",
        "{label}：左端退刀槽的 R 圆角切点 {p0} 不低于台阶面 {p1}（相邻台阶太小）",
        "{label}: the R-fillet tangent point {p0} of the left-end relief groove is not below the shoulder face {p1} (neighbouring shoulder too small)",
    ),
    Msg::new(
        "cmd.shaft.err.tlro_no_chamfer",
        "{label}：局部螺纹 TL/RL 的右端不能倒角 CH（相邻段倒角会落在这里）",
        "{label}: the right end of a partial thread TL/RL cannot take a chamfer CH (a neighbouring chamfer would land there)",
    ),
    Msg::new(
        "cmd.shaft.err.chamfer_left_eats_face",
        "{label}：左端倒角 C={p0} ≥ 端面半径（Ø{p1} 的 {p2}），端面被吃掉",
        "{label}: left-end chamfer C={p0} ≥ the end-face radius (Ø{p1}, radius {p2}); the end face is eaten away",
    ),
    Msg::new(
        "cmd.shaft.err.chamfer_left_through",
        "{label}：左端倒角 C={p0} 把端面吃穿了（端面点 {p1} ≤ 0）",
        "{label}: left-end chamfer C={p0} cuts through the end face (face point {p1} ≤ 0)",
    ),
    Msg::new(
        "cmd.shaft.err.ov_free_end_r",
        "{label}：右端是自由端，越程槽没有台阶面",
        "{label}: the right end is free; an overtravel groove has no shoulder face",
    ),
    Msg::new(
        "cmd.shaft.err.tlro_no_chamfer_plain",
        "{label}：局部螺纹 TL/RL 的右端不能倒角 CH",
        "{label}: the right end of a partial thread TL/RL cannot take a chamfer CH",
    ),
    Msg::new(
        "cmd.shaft.err.chamfer_right_eats_face",
        "{label}：右端倒角 C={p0} ≥ 端面半径（Ø{p1} 的 {p2}），端面被吃掉",
        "{label}: right-end chamfer C={p0} ≥ the end-face radius (Ø{p1}, radius {p2}); the end face is eaten away",
    ),
    Msg::new(
        "cmd.shaft.err.chamfer_right_through",
        "{label}：右端倒角 C={p0} 把端面吃穿了（端面点 {p1} ≤ 0）",
        "{label}: right-end chamfer C={p0} cuts through the end face (face point {p1} ≤ 0)",
    ),
    Msg::new(
        "cmd.shaft.err.both_ends_overlap",
        "{label}：两端特征重叠（左 {p0} + 右 {p1} > 段长 {p2}）",
        "{label}: features at both ends overlap (left {p0} + right {p1} > segment length {p2})",
    ),
    Msg::new(
        "cmd.shaft.err.tlro_no_ov",
        "{label}：局部螺纹 TL/RL 的右端不能有越程槽 OV",
        "{label}: the right end of a partial thread TL/RL cannot take an overtravel groove OV",
    ),
    Msg::new(
        "cmd.shaft.err.tlro_no_fit",
        "{label}：局部螺纹装不下（小径细实线长 ≤ 0；TL/RL 与左端倒角/段长冲突）",
        "{label}: partial thread does not fit (minor-diameter thin line ≤ 0; TL/RL conflicts with the left chamfer/segment length)",
    ),
    Msg::new(
        "cmd.shaft.report.header",
        "- 视图：{p0}（{p1}）\n- 段数：{p2}；总长：{p3} mm；最大直径：{p4} mm\n\n",
        "- View: {p0} ({p1})\n- Segments: {p2}; total length: {p3} mm; max diameter: {p4} mm\n\n",
    ),
    Msg::new("cmd.shaft.report.title", "# 轴段计算书", "# Shaft segment report"),
    Msg::new("cmd.shaft.report.sec_list", "## 1. 段清单", "## 1. Segment list"),
    Msg::new(
        "cmd.shaft.report.list_header",
        "| # | 类型 | 关键参数 | 长度 mm | 外径 mm |",
        "| # | Type | Key parameters | Length mm | Outer Ø mm |",
    ),
    Msg::new(
        "cmd.shaft.report.spline_sec_title",
        "## 2. 花键参数表（GB/T 3478，默认 7 级 / H·h）",
        "## 2. Spline parameter table (GB/T 3478, default grade 7 / H·h)",
    ),
    Msg::new(
        "cmd.shaft.report.spline_sec_note",
        "> 与 `OCSMCARD 花键参数表`（智能卡片）**同一份 `spline_tol::compute()`**；\n             > 等级/配合类别/量棒直径 Dp 可在该卡命令里显式给。\n\n",
        "> Shares one `spline_tol::compute()` with `OCSMCARD 花键参数表` (the smart card);\n             > grade/fit class/pin diameter Dp can be given explicitly in that card command.\n\n",
    ),
    Msg::new(
        "cmd.shaft.report.keyway",
        "；轴槽 {p0} 键长 L={p1} {p2} b={p3} t1={p4}",
        "; shaft key {p0}, key length L={p1}, {p2}, b={p3}, t1={p4}",
    ),
    Msg::new(
        "cmd.shaft.report.spline_heading",
        "### 第 {p0} 段（{p1}，默认 7 级 / 基孔制 H）\n\n{body}\n",
        "### Segment {p0} ({p1}, default grade 7 / hole-basis H)\n\n{body}\n",
    ),
    Msg::new(
        "cmd.shaft.err.json_no_segments",
        "JSON 里没有 segments（至少给一段）",
        "JSON has no segments (give at least one)",
    ),
    Msg::new(
        "cmd.shaft.err.json_rot_not_finite",
        "JSON：rot 不是有限数",
        "JSON: rot is not a finite number",
    ),
    Msg::new("cmd.shaft.err.field_at", "at 坐标", "at coordinates"),
    Msg::new("cmd.shaft.err.keyparam.kind", "键型", "key style"),
    Msg::new("cmd.shaft.err.keyparam.len", "键长", "key length"),
    Msg::new("cmd.shaft.err.keyparam.place", "@中/@端", "@mid/@end"),
    Msg::new("cmd.shaft.err.keyparam.double", "双槽", "双槽/DOUBLE"),
    Msg::new("cmd.shaft.err.keyparam.guided", "导向", "导向/guided"),
    Msg::new(
        "cmd.shaft.err.json_at_format",
        "at 字符串要写成 \"x,y\"",
        "the at string must be written as \"x,y\"",
    ),
    Msg::new("cmd.shaft.err.json_at_x", "at 的 x 不是数字", "at's x is not a number"),
    Msg::new("cmd.shaft.err.json_at_y", "at 的 y 不是数字", "at's y is not a number"),
    Msg::new(
        "cmd.shaft.err.validate_no_segments",
        "至少要有一段（S… L…）",
        "at least one segment is required (S… L…)",
    ),
    Msg::new(
        "cmd.shaft.err.rl_free_end_l",
        "{label}：左端是自由端，退刀槽没有台阶面",
        "{label}: the left end is free; a relief groove has no shoulder face",
    ),
    Msg::new(
        "cmd.shaft.err.ov_free_end_l",
        "{label}：左端是自由端，越程槽没有台阶面",
        "{label}: the left end is free; an overtravel groove has no shoulder face",
    ),
    Msg::new(
        "cmd.shaft.report.type_gear",
        "齿轮段",
        "gear segment",
    ),
    Msg::new(
        "cmd.shaft.report.type_spline",
        "矩形花键段",
        "rectangular spline segment",
    ),
    Msg::new(
        "cmd.shaft.report.type_thread",
        "螺纹段",
        "thread segment",
    ),
    Msg::new(
        "cmd.shaft.report.simplified_085d",
        "简化0.85d",
        "simplified 0.85d",
    ),
    Msg::new(
        "cmd.shaft.report.type_plain",
        "普通段",
        "plain segment",
    ),

    // —— 齿轮族 core（③a，2026-09-27）——
    Msg::new("cmd.detail.sep.list", "、", ", "),
    Msg::new("cmd.detail.hint.grind_od", "基点 = 台阶面与轴线交点（轴线为 x 轴；d = 磨出的外圆直径）", "Base point = intersection of the shoulder face and the axis (axis = x; d = ground outside diameter)"),
    Msg::new("cmd.detail.hint.thread_relief", "基点 = 台肩面与轴线交点（轴线为 x 轴；d = 螺纹公称直径）", "Base point = intersection of the shoulder face and the axis (axis = x; d = nominal thread diameter)"),
    Msg::new("cmd.detail.hint.hub_keyway", "主视图基点 = 孔心；侧视图基点 = 左端面×轴线（d = 孔径；键槽开口朝 +Y）", "Front-view base point = hole centre; side-view base point = left end face × axis (d = hole diameter; the keyway opens towards +Y)"),
    Msg::new("cmd.detail.hint.spline_rect", "基点 = 左端面与轴线交点（轴线为 x 轴；正视图 = 齿形中心）", "Base point = intersection of the left end face and the axis (axis = x; front view = tooth-profile centre)"),
    Msg::new("cmd.detail.hub.view.main", "主视图（孔端面）", "Front view (hole end face)"),
    Msg::new("cmd.detail.hub.view.side", "侧视图（纵向剖）", "Side view (longitudinal section)"),
    Msg::new("cmd.detail.spline.view.front", "正视图（端视图）", "Front view (end view)"),
    Msg::new("cmd.detail.spline.view.side", "常规侧视图", "Regular side view"),
    Msg::new("cmd.detail.spline.view.section", "侧剖视图", "Sectional side view"),
    Msg::new("cmd.detail.part_name_rect", "矩形花键（{view}）", "Rectangular spline ({view})"),
    Msg::new("cmd.detail.name.grind_od", "磨外圆", "external grinding"),
    Msg::new("cmd.detail.name.thread_relief", "外螺纹退刀槽", "external thread relief groove"),
    Msg::new("cmd.detail.name.hub_keyway", "普通平键毂槽", "parallel-key hub keyway"),
    Msg::new("cmd.detail.name.spline_rect", "矩形花键", "rectangular spline"),
    Msg::new("cmd.detail.what.thread_relief", "外螺纹退刀槽", "external thread relief groove"),
    Msg::new("cmd.detail.what.relief_seg", "段级退刀槽", "segment-level relief groove"),
    Msg::new("cmd.detail.big.thread_major", "螺纹大径", "thread major diameter"),
    Msg::new("cmd.detail.big.seg_major", "本段大径", "this segment's major diameter"),
    Msg::new("cmd.detail.err.unknown_param_b1", "{name}：不认识参数 {list}（本族只支持 b1）", "{name}: unrecognized parameter {list} (this family only supports b1)"),
    Msg::new("cmd.detail.err.family_unknown", "结构要素族 {family} 尚未实现", "detail-element family {family} is not implemented"),
    Msg::new("cmd.detail.err.view_only", "{name} 只有视图 {list}（收到 {view}）", "{name} only has view(s) {list} (got {view})"),
    Msg::new("cmd.detail.err.d_not_number", "d 不是数字", "d is not a number"),
    Msg::new("cmd.detail.err.missing_d", "缺少参数 d（或 spec 规格代号）", "missing parameter d (or a spec code)"),
    Msg::new("cmd.detail.err.param_not_number", "参数 {key} 不是数字", "parameter {key} is not a number"),
    Msg::new("cmd.detail.grind.err.d_positive", "磨外圆：d 必须是正数（收到 {d}）", "External grinding: d must be positive (got {d})"),
    Msg::new("cmd.detail.grind.err.d_out_of_table", "磨外圆：d={d} 不在数据表范围内", "External grinding: d={d} is outside the data table"),
    Msg::new("cmd.detail.grind.err.empty_table", "磨外圆：数据表为空", "External grinding: the data table is empty"),
    Msg::new("cmd.detail.grind.err.b1_positive", "磨外圆：b1 必须是正数（收到 {b1}）", "External grinding: b1 must be positive (got {b1})"),
    Msg::new("cmd.detail.grind.err.b1_choice", "磨外圆：d={d} 属于「{band}」档，该档可选 b1 = {choices}（收到 {got}）", "External grinding: d={d} falls in the \"{band}\" band, whose b1 values are {choices} (got {got})"),
    Msg::new("cmd.detail.thread.err.p_positive", "外螺纹退刀槽：螺距 P 必须是正数（收到 {p}）", "External thread relief: pitch P must be positive (got {p})"),
    Msg::new("cmd.detail.thread.err.p_not_in_table", "外螺纹退刀槽：表 2 没有螺距 P={p}（可用 P = {choices}；表 2 从 0.25 起，无 0.2）", "External thread relief: Table 2 has no pitch P={p} (available P = {choices}; Table 2 starts at 0.25, no 0.2)"),
    Msg::new("cmd.detail.thread.err.unknown_param", "外螺纹退刀槽：不认识参数 {list}（本族支持 P/g1/g2/dg/r/alpha）", "External thread relief: unrecognized parameter {list} (this family supports P/g1/g2/dg/r/alpha)"),
    Msg::new("cmd.detail.thread.err.d_positive", "外螺纹退刀槽：d 必须是正数（收到 {d}）", "External thread relief: d must be positive (got {d})"),
    Msg::new("cmd.detail.thread.err.p_missing", "外螺纹退刀槽：缺少螺距 P（写法 `XL detail_thread_relief <d> P <P>`）", "External thread relief: missing pitch P (write `XL detail_thread_relief <d> P <P>`)"),
    Msg::new("cmd.detail.relief.err.d_positive", "{what}：d 必须是正数（收到 {d}）", "{what}: d must be positive (got {d})"),
    Msg::new("cmd.detail.relief.err.value_positive", "{what}：{name} 必须是正数（收到 {value}）", "{what}: {name} must be positive (got {value})"),
    Msg::new("cmd.detail.relief.err.alpha_finite", "{what}：alpha 不是有限数", "{what}: alpha is not a finite number"),
    Msg::new("cmd.detail.relief.err.alpha_min", "{what}：alpha 必须 ≥ {min}°（收到 {got}°）—— 斜壁不能比 30° 更平", "{what}: alpha must be ≥ {min}° (got {got}°) — the tapered wall cannot be flatter than 30°"),
    Msg::new("cmd.detail.relief.err.dg_lt_d", "{what}：dg（{dg}）必须小于{big} d（{d}）", "{what}: dg ({dg}) must be smaller than the {big} d ({d})"),
    Msg::new("cmd.detail.relief.err.g2_gt_g1", "{what}：g2（{g2}）必须大于 g1（{g1}）", "{what}: g2 ({g2}) must be greater than g1 ({g1})"),
    Msg::new("cmd.detail.relief.err.g1_gt_r", "{what}：g1（{g1}）必须大于圆角 r（{r}），否则圆角与斜壁打架", "{what}: g1 ({g1}) must be greater than the fillet r ({r}), otherwise the fillet clashes with the tapered wall"),
    Msg::new("cmd.detail.relief.err.r_le_half", "{what}：圆角 r（{r}）超过槽深 (d−dg)/2（{half}），圆角伸到{big}之外", "{what}: fillet r ({r}) exceeds the groove depth (d−dg)/2 ({half}); the fillet sticks out beyond the {big}"),
    Msg::new("cmd.detail.relief.err.wall_angle", "{what}：斜壁实际角 {wall}° 小于 alpha {alpha}°（含表 2 取整容差 {tol}°）—— 检查 g1/g2/dg 覆盖值", "{what}: the actual tapered-wall angle {wall}° is smaller than alpha {alpha}° (including the Table 2 rounding tolerance {tol}°) — check the g1/g2/dg overrides"),
    Msg::new("cmd.detail.thread.err.p_hint", "外螺纹退刀槽需要螺距 P：请用 `XL detail_thread_relief <d> P <P> [g1 值 …]`", "External thread relief needs pitch P: use `XL detail_thread_relief <d> P <P> [g1 value …]`"),
    Msg::new("cmd.detail.hub.err.unknown_param", "毂槽：不认识参数 {list}（本族只支持 len 毂长；b/t₂/r 由 d 查表）", "Hub keyway: unrecognized parameter {list} (this family only supports len; b/t₂/r are looked up from d)"),
    Msg::new("cmd.detail.hub.err.d_positive", "毂槽：孔径 d 必须是正数（收到 {d}）", "Hub keyway: bore d must be positive (got {d})"),
    Msg::new("cmd.detail.hub.err.d_not_in_table", "毂槽：d={d} 不在 GB/T 1095 的 d 选型表（6…500）里，无法按孔径定键宽 b", "Hub keyway: d={d} is not in the GB/T 1095 d selection table (6…500), cannot determine key width b from the bore"),
    Msg::new("cmd.detail.hub.err.table_read", "毂槽：GB/T 1095 表读取失败：{e}", "Hub keyway: failed to read the GB/T 1095 table: {e}"),
    Msg::new("cmd.detail.hub.err.b_unknown", "毂槽：GB/T 1095 表里没有 b={b} 这一档", "Hub keyway: the GB/T 1095 table has no b={b} row"),
    Msg::new("cmd.detail.hub.err.b_ge_d", "毂槽：键宽 b={b} ≥ 孔径 d={d}（槽切穿孔壁），d 与表 1 不符", "Hub keyway: key width b={b} ≥ bore d={d} (the slot would cut through the wall), d does not match Table 1"),
    Msg::new("cmd.detail.hub.err.r_range", "毂槽：圆角 r={r} 必须 >0 且 < b/2={half}（表 1 的 r 范围异常）", "Hub keyway: fillet r={r} must be >0 and < b/2={half} (the r range in Table 1 is anomalous)"),
    Msg::new("cmd.detail.hub.err.t2_positive", "毂槽：表 1 的 t₂={t2} 必须 >0", "Hub keyway: Table 1 t₂={t2} must be >0"),
    Msg::new("cmd.detail.hub.err.len_positive", "毂槽：毂长 len={len} 必须是正数", "Hub keyway: hub length len={len} must be positive"),
    Msg::new("cmd.detail.hub.err.bottom_below", "毂槽：槽底 R+t₂−r={bottom} 低于孔壁交点 {intersect}（表 1 数据/圆角取值异常）", "Hub keyway: groove bottom R+t₂−r={bottom} is below the bore-wall intersection {intersect} (Table 1 data/fillet value anomalous)"),
    Msg::new("cmd.detail.spline.err.unknown_param", "矩形花键：不认识参数 {list}（本族支持 规格代号 spec、L/len、de、N、D、B）", "Rectangular spline: unrecognized parameter {list} (this family supports the spec code, L/len, de, N, D, B)"),
    Msg::new("cmd.detail.spline.err.mismatch_d", "矩形花键：规格 {spec} 的小径 d={v} 与参数 d={p} 不一致", "Rectangular spline: the spec {spec} minor diameter d={v} does not match the parameter d={p}"),
    Msg::new("cmd.detail.spline.err.mismatch_n", "矩形花键：规格 {spec} 的齿数 N={v} 与参数 N={p} 不一致", "Rectangular spline: the spec {spec} tooth count N={v} does not match the parameter N={p}"),
    Msg::new("cmd.detail.spline.err.mismatch_dmajor", "矩形花键：规格 {spec} 的大径 D={v} 与参数 D={p} 不一致", "Rectangular spline: the spec {spec} major diameter D={v} does not match the parameter D={p}"),
    Msg::new("cmd.detail.spline.err.mismatch_b", "矩形花键：规格 {spec} 的键宽 B={v} 与参数 B={p} 不一致", "Rectangular spline: the spec {spec} key width B={v} does not match the parameter B={p}"),
    Msg::new("cmd.detail.spline.err.missing_n", "矩形花键：缺齿数 N（写法 `spec 6x23x26x6` 或数字参数 `N6 D26 B6 d23`）", "Rectangular spline: missing tooth count N (write `spec 6x23x26x6` or numeric parameters `N6 D26 B6 d23`)"),
    Msg::new("cmd.detail.spline.err.n_range", "矩形花键：齿数 N={n} 必须取 3..=100 的整数", "Rectangular spline: tooth count N={n} must be an integer in 3..=100"),
    Msg::new("cmd.detail.spline.err.missing_dmajor", "矩形花键：缺大径 D（数字参数写法 `D26`）", "Rectangular spline: missing major diameter D (numeric form `D26`)"),
    Msg::new("cmd.detail.spline.err.missing_b", "矩形花键：缺键宽 B（数字参数写法 `B6`）", "Rectangular spline: missing key width B (numeric form `B6`)"),
    Msg::new("cmd.detail.spline.err.d_minor_positive", "矩形花键：小径 d={d} 必须是正数", "Rectangular spline: minor diameter d={d} must be positive"),
    Msg::new("cmd.detail.spline.err.de_not_found", "矩形花键：该 N/d/D/B 不在 GB/T 10952-2005 表 1/表 2 里，de 查不到 —— 请给 de", "Rectangular spline: this N/d/D/B is not in GB/T 10952-2005 Table 1/2, de cannot be looked up — please give de"),
    Msg::new("cmd.detail.spline.usage", "矩形花键请给规格代号：`XL detail_spline_rect 6x23x26x6 L30 [de 63] [view side|section|front]`", "Rectangular spline: please give a spec code: `XL detail_spline_rect 6x23x26x6 L30 [de 63] [view side|section|front]`"),
    Msg::new("cmd.detail.spline.err.need_len_side", "矩形花键：常规侧视图需要 L（满齿段长，例 `L30` / `&len=30`）", "Rectangular spline: the regular side view needs L (full-tooth length, e.g. `L30` / `&len=30`)"),
    Msg::new("cmd.detail.spline.err.need_len_section", "矩形花键：侧剖视图需要 L（满齿段长，例 `L30` / `&len=30`）", "Rectangular spline: the side section view needs L (full-tooth length, e.g. `L30` / `&len=30`)"),
    Msg::new("cmd.detail.spline.err.view_unimplemented", "矩形花键：视图 {view} 尚未实现", "Rectangular spline: view {view} is not implemented yet"),
    Msg::new("cmd.detail.runout.err.p_positive", "GB/T 3 表 1：螺距 P 必须是正数（收到 {p}）", "GB/T 3 Table 1: pitch P must be positive (got {p})"),
    Msg::new("cmd.detail.runout.err.p_not_in_table", "GB/T 3 表 1 没有螺距 P={p}（可用 P = {choices}；不插值、不外推）", "GB/T 3 Table 1 has no pitch P={p} (available P = {choices}; no interpolation, no extrapolation)"),
    Msg::new("cmd.gear.usage", "用法：OCSMGEAR [内齿轮|int] <模数m> <齿数z> [h=齿宽] [ha=齿顶高系数] [c=顶隙系数] [alpha=压力角(°,默认20) 或 α25] [beta=螺旋角(右旋为正)] [x=变位系数] [view 剖视图|侧视图|简化正视图|常规正视图|端视图] [at x,y] [rot 度]。\n齿轮体系：默认 M 模数制；径节制写 `std=DP dp=8`（或 `DP8`），此时位置参数 = `<齿数z> <齿宽h>`，m=25.4/DP。\n花键模式：`OCSMGEAR 花键 [内花键] [std=GB|DIN|NF|ANSI] [profile=GB30R] [db=40] [hf=0.9] [rho=0.4] [cf=0.1] <m> <z> [x=..] [h=..] [view 端视图|侧视图|剖视图]`；花键模式 α 由齿廓预设固定（GB 30/37.5/45°、DIN 30°、NF 20°、ANSI Table 2 列），不可覆盖；径节 P/Ps 只属 ANSI（GB/DIN/NF 给 P 会报错）；GB/T 3478 基本齿廓不含变位（x 恒为 0，给非零 x 明确报错；GUI 花键模式下 x 已锁死）；内花键与内齿轮同口径：只有 `view 端视图|剖视图`（无侧视图，用户定案）；花键参数也可用预设代号（GB30P/GB30R/GB375R/GB45R/DIN30/NFP/NFR/ANSI30P/ANSI30PM/ANSI30R/ANSI375R/ANSI45R）代替 std+profile；花键模式：`db=40` 是 DIN 的 d_B，NF 用 `a=66`（或 `公称直径=66`，也兼容 `db=` 当 A）；ANSI 是径节制：写 `P2.5/5`（A/B 成对，A=P、B=Ps=2P；`pitch=2.5/5`、裸 `P8` 也收），位置参数 = <径节P> <齿数N> <有效长度L>，x 不允许。\n不带参数则打开齿轮窗口。内齿轮（齿圈）目前只有 剖视图 + 端视图（模板只有这两个）；剖视图不画齿圈外壁与剖面线，由用户/AI 按实际齿圈结构延伸。", "Usage: OCSMGEAR [内齿轮|int] <module m> <teeth z> [h=face width] [ha=addendum coeff] [c=clearance coeff] [alpha=pressure angle (°, default 20) or α25] [beta=helix angle (right-hand positive)] [x=profile shift] [view 剖视图|侧视图|简化正视图|常规正视图|端视图] [at x,y] [rot deg].\nGear system: default M (module); for diametral pitch write `std=DP dp=8` (or `DP8`), then positional parameters = `<teeth z> <face width h>`, m=25.4/DP.\nSpline mode: `OCSMGEAR 花键 [内花键] [std=GB|DIN|NF|ANSI] [profile=GB30R] [db=40] [hf=0.9] [rho=0.4] [cf=0.1] <m> <z> [x=..] [h=..] [view 端视图|侧视图|剖视图]`; in spline mode α is fixed by the profile preset (GB 30/37.5/45°, DIN 30°, NF 20°, ANSI Table 2 column) and cannot be overridden; pitch P/Ps belongs only to ANSI (GB/DIN/NF report an error if P is given); the GB/T 3478 basic profile has no profile shift (x is always 0; a non-zero x is a clear error; the GUI locks x in spline mode); internal splines share the internal-gear convention: only `view 端视图|剖视图` (no side view, decided by the user); spline parameters can also use preset tokens (GB30P/GB30R/GB375R/GB45R/DIN30/NFP/NFR/ANSI30P/ANSI30PM/ANSI30R/ANSI375R/ANSI45R) instead of std+profile; in spline mode `db=40` is DIN's d_B, NF uses `a=66` (or `公称直径=66`, `db=` is also accepted as A); ANSI uses diametral pitch: write `P2.5/5` (paired A/B, A=P, B=Ps=2P; `pitch=2.5/5` and bare `P8` are also accepted), positional parameters = <pitch P> <teeth N> <effective length L>, x is not allowed.\nWithout arguments the gear window opens. Internal gears (rings) currently have only 剖视图 + 端视图 (the template has only these two); the section view does not draw the ring's outer wall or hatching; the user/AI extends it per the actual ring structure."),
    Msg::new("cmd.gear.err.std_mode", "模数制齿轮不使用标准号与基准直径 —— 齿轮模式不认标准号（GB/T 3478.1 / DIN 5480-1 / NF E22-141 / ANSI B92.1 是**花键**体系）；要用花键请在窗口勾选「花键模式」（命令行加 `花键` 或 `mode=spline`）。", "Module-mode gears do not use a standard number or reference diameter — gear mode does not accept a standard number (GB/T 3478.1 / DIN 5480-1 / NF E22-141 / ANSI B92.1 are **spline** systems); to use a spline, tick \"spline mode\" in the window (add `花键` or `mode=spline` on the command line)."),
    Msg::new("cmd.gear.err.m_dp_conflict", "M 与 DP 同给：模数制与径节制只能选一个（M 用 m，DP 用 DP=25.4/m）。", "M and DP were both given: choose module or diametral pitch, not both (use m for M, DP=25.4/m for DP)."),
    Msg::new("cmd.gear.err.module_positive", "模数 m 必须是正数。", "Module m must be positive."),
    Msg::new("cmd.gear.err.dp_missing_std", "径节制（std=DP）缺径节值 DP（写法 `dp=8` / `DP8`；m = 25.4/DP）。", "Diametral-pitch mode (std=DP) is missing the DP value (write `dp=8` / `DP8`; m = 25.4/DP)."),
    Msg::new("cmd.gear.err.dp_positive", "径节 DP={v} 必须是正数。", "Diametral pitch DP={v} must be positive."),
    Msg::new("cmd.gear.err.dp_mismatch", "径节制：DP={dp} 对应 m=25.4/DP={m}，与当前 m={cur} 不符（DP 体系不要另给 m）。", "Diametral pitch: DP={dp} gives m=25.4/DP={m}, which does not match the current m={cur} (do not give m again in DP mode)."),
    Msg::new("cmd.gear.err.z_range", "齿数 z 超出范围（2–1000）：{z}", "Number of teeth z out of range (2–1000): {z}"),
    Msg::new("cmd.gear.err.alpha_range", "压力角 α 超出范围（10°<α<50°）：{v}°", "Pressure angle α out of range (10°<α<50°): {v}°"),
    Msg::new("cmd.gear.err.ha_min", "齿顶高系数 ha* 至少 0.5。", "Addendum coefficient ha* must be at least 0.5."),
    Msg::new("cmd.gear.err.c_negative", "顶隙系数 c* 不能为负。", "Clearance coefficient c* must not be negative."),
    Msg::new("cmd.gear.err.beta_range", "螺旋角 β 超出范围（|β|<45°）：{v}°", "Helix angle β out of range (|β|<45°): {v}°"),
    Msg::new("cmd.gear.err.h_positive", "厚度 h 必须是正数。", "Thickness h must be positive."),
    Msg::new("cmd.gear.err.x_range", "变位系数 Xn 超出范围（|Xn|≤1）：{v}", "Profile shift Xn out of range (|Xn|≤1): {v}"),
    Msg::new("cmd.gear.err.internal_da_nonpositive", "内齿轮齿顶圆直径非正（齿朝圆心长太长）：检查 m/z/ha*/Xn 组合。", "Internal-gear tip diameter is not positive (teeth extend too far toward the centre): check the m/z/ha*/Xn combination."),
    Msg::new("cmd.gear.err.internal_da_ge_df", "内齿轮齿顶圆不小于齿根圆：检查 m/z/ha*/c* 组合。", "Internal-gear tip circle is not smaller than the root circle: check the m/z/ha*/c* combination."),
    Msg::new("cmd.gear.err.df_nonpositive", "齿根圆直径非正：检查 m/z/ha*/c*/Xn 组合。", "Root diameter is not positive: check the m/z/ha*/c*/Xn combination."),
    Msg::new("cmd.gear.err.chamfer_too_big", "轴向倒角 C=round(0.6m)={c} 太大（厚度 h={h}）——加大厚度或减小模数。", "Axial chamfer C=round(0.6m)={c} is too large (thickness h={h}) — increase the thickness or reduce the module."),
    Msg::new("cmd.gear.err.chamfer_ge_tip", "轴向倒角大于齿顶圆半径，无法画图。", "Axial chamfer exceeds the tip-circle radius; cannot draw."),
    Msg::new("cmd.gear.err.fillet_singular", "齿廓样条方程组奇异（参数太极端）", "Tooth-profile spline system is singular (parameters too extreme)"),
    Msg::new("cmd.gear.err.spline_mode_required", "齿轮模式不是花键（请先勾选「花键模式」再给标准号/d_B）。", "Gear mode is not spline mode (tick \"spline mode\" before giving a standard number/d_B)."),
    Msg::new("cmd.gear.err.h_spline_positive", "花键厚度/有效长度 h 必须是正数。", "Spline thickness/effective length h must be positive."),
    Msg::new("cmd.gear.err.db_mode", "模数制齿轮不使用标准号与基准直径 —— 齿轮模式不认基准直径 d_B（d_B 是 DIN 5480 / NF E22-141 花键体系的概念）；要用花键请在窗口勾选「花键模式」（命令行加 `花键` 或 `mode=spline`）。", "Module-mode gears do not use a standard number or reference diameter — gear mode does not accept reference diameter d_B (d_B belongs to the DIN 5480 / NF E22-141 spline systems); to use a spline, tick \"spline mode\" in the window (add `花键` or `mode=spline` on the command line)."),
    Msg::new("cmd.gear.err.spline_params_in_gear", "齿轮模式不认花键参数（齿廓/hf/ρf/cf/径节 P）；要用花键请在窗口勾选「花键模式」（命令行加 `花键`；径节制齿轮写 `DP…`）。", "Gear mode does not accept spline parameters (profile/hf/ρf/cf/pitch P); to use a spline, tick \"spline mode\" in the window (add `花键` on the command line; a diametral-pitch gear writes `DP…`)."),
    Msg::new("cmd.gear.err.spline_params_in_gear_query", "齿轮模式不认花键参数（齿廓/hf/ρf/cf）；要用花键请勾选「花键模式」。", "Gear mode does not accept spline parameters (profile/hf/ρf/cf); to use a spline, tick \"spline mode\"."),
    Msg::new("cmd.gear.err.p_in_gear", "齿轮模式不认径节 P（径节制齿轮写 `std=DP&dp=8`；花键径节只属 ANSI B92.1 花键模式）。", "Gear mode does not accept pitch P (a diametral-pitch gear writes `std=DP&dp=8`; spline pitch belongs only to ANSI B92.1 spline mode)."),
    Msg::new("cmd.gear.err.dp_missing_cli", "径节制 DP：缺径节值（写法 `DP8` 或 `dp=8`）。", "Diametral pitch DP: missing the DP value (write `DP8` or `dp=8`)."),
    Msg::new("cmd.gear.err.dp_missing_query", "径节制 DP：缺径节值（写法 `std=DP&dp=8`）。", "Diametral pitch DP: missing the DP value (write `std=DP&dp=8`)."),
    Msg::new("cmd.gear.err.no_m_in_dp", "DP 体系用径节 DP 定模数（m = 25.4/DP），不要再给 m。", "In DP mode the module is defined by the diametral pitch (m = 25.4/DP); do not give m again."),
    Msg::new("cmd.gear.err.dp_pos_count", "径节制位置参数最多 2 个（<齿数z> <齿宽h>），多给了 {n} 个。\n{usage}", "DP mode accepts at most 2 positional parameters (<teeth z> <face width h>); {n} extra given.\n{usage}"),
    Msg::new("cmd.gear.err.std_is_gear_system", "`{s}` 是齿轮体系（M = 模数制 / DP = 径节制），不是花键体系；花键体系可用 GB / DIN / NF / ANSI。", "`{s}` is a gear system (M = module / DP = diametral pitch), not a spline system; spline systems are GB / DIN / NF / ANSI."),
    Msg::new("cmd.gear.err.std_unknown", "花键体系标识无法识别：`{s}`（可用 GB / GB/T 3478.1 / DIN / DIN 5480 / NF / NF E22-141 / ANSI / ANSI B92.1）。", "Unrecognized spline-system token: `{s}` (available: GB / GB/T 3478.1 / DIN / DIN 5480 / NF / NF E22-141 / ANSI / ANSI B92.1)."),
    Msg::new("cmd.gear.err.num_expected", "{key} 需要数字，收到 `{v}`", "{key} expects a number, got `{v}`"),
    Msg::new("cmd.gear.err.m_expected", "m 需要数字，收到 `{text}`", "m expects a number, got `{text}`"),
    Msg::new("cmd.gear.err.mode_unknown", "模式无法识别：`{v}`（可用 gear|齿轮、spline|花键）。", "Unrecognized mode: `{v}` (available: gear|齿轮, spline|花键)."),
    Msg::new("cmd.gear.err.at_pair", "at 坐标需要 `x,y`，收到 `{pair}`", "at coordinates must be `x,y`, got `{pair}`"),
    Msg::new("cmd.gear.err.kind_unknown", "种类无法识别：`{v}`。可用 internal|内齿轮（花键模式下 = 内花键）、external|外齿轮。", "Unrecognized kind: `{v}`. Available: internal|内齿轮 (internal spline in spline mode), external|外齿轮."),
    Msg::new("cmd.gear.err.alpha_expected", "alpha 需要数字，收到 `{v}`", "alpha expects a number, got `{v}`"),
    Msg::new("cmd.gear.err.dp_expected", "径节 DP 需要数字，收到 `{v}`", "Diametral pitch DP expects a number, got `{v}`"),
    Msg::new("cmd.gear.err.a_expected", "公称直径 A 需要数字，收到 `{v}`", "Nominal diameter A expects a number, got `{v}`"),
    Msg::new("cmd.gear.err.a_positive", "公称直径 A={v} 必须是正数。", "Nominal diameter A={v} must be positive."),
    Msg::new("cmd.gear.err.unknown_param", "无法识别的参数 `{arg}`。\n{usage}", "Unrecognized argument `{arg}`.\n{usage}"),
    Msg::new("cmd.gear.err.extra_param", "多余的参数 `{arg}`。\n{usage}", "Extra argument `{arg}`.\n{usage}"),
    Msg::new("cmd.gear.err.need_m_z", "至少要给模数 m 和齿数 z（例如 `OCSMGEAR 2 40 20`）。\n{usage}", "At least the module m and the number of teeth z are required (e.g. `OCSMGEAR 2 40 20`).\n{usage}"),
    Msg::new("cmd.gear.err.kind_unknown_query", "kind 无法识别：`{v}`（可用 internal|内齿轮、external|外齿轮）。", "Unrecognized kind: `{v}` (available: internal|内齿轮, external|外齿轮)."),
    Msg::new("cmd.gear.note.dp_spec", "径节制（{dp} DP）：m = 25.4/DP = {m}；块名/规格按 **DP 原值** 出，避免与同模数 {m2} 的模数制件串块。", "Diametral pitch ({dp} DP): m = 25.4/DP = {m}; block name/spec use the **original DP value**, to avoid clashing with a module-mode part of the same module {m2}."),
    Msg::new("cmd.gear.note.undercut", "齿数 z={z} < 17 且未变位：实际滚齿会有根切，本视图未画根切（如需要请给变位或根切画法模板）。", "z={z} < 17 with no profile shift: real hobbing would undercut; this view does not draw the undercut (supply a profile shift or an undercut template if needed)."),
    Msg::new("cmd.gear.note.helical", "斜齿轮（β={beta}°，{hand}）：正视图齿廓按端面参数 mt={mt}／αt={at}° 画，侧视图按 GB 简化画法加三条细实线表示轮齿倾斜方向。", "Helical gear (β={beta}°, {hand}): the front view draws the tooth profile with transverse parameters mt={mt} / αt={at}°; the side view adds three thin solid lines per the GB simplified convention to show the tooth inclination."),
    Msg::new("cmd.gear.note.shift", "变位齿轮（Xn={x}）：da/df 与齿厚按国标公式算（齿顶高未扣 Δy，单件图无配对中心距）。", "Profile-shifted gear (Xn={x}): da/df and tooth thickness use the national-standard formulas (the addendum does not deduct Δy; a single-part drawing has no mating centre distance)."),
    Msg::new("cmd.gear.note.alpha_nonstandard", "非 20° 基准齿形角（α={alpha}°）：α 已代入渐开线/基圆/齿厚公式（db=d·cosαt、ψ(R) 用 inv αt）；ha*、c*、齿根圆角系数 ρ=0.38m 不随 α 自动改变（当前 ha*={ha}、c*={c}、ρ={rho}），14.5°/25° 等系统的系数、以及 30°/37.5°/45° 花键的短齿顶请按所用标准手动填 ha*/c*。", "Non-20° basic profile angle (α={alpha}°): α has been substituted into the involute/base-circle/thickness formulas (db=d·cosαt; ψ(R) uses inv αt); ha*, c* and the root fillet coefficient ρ=0.38m do not change with α automatically (currently ha*={ha}, c*={c}, ρ={rho}); for 14.5°/25° systems and the short addendum of 30°/37.5°/45° splines, fill ha*/c* manually per the applicable standard."),
    Msg::new("cmd.gear.note.tip_crossed", "齿顶变尖/渐开线交叉：α={alpha}° 配 ha*={ha} 时 ψ(da/2)={psi}° ≤ 0 —— 两条齿廓在齿顶圆之前就相交，「常规正视图」画不出真实渐开线齿廓（会报错；剖视/侧视/简化正视图不受影响）。45° 花键通常配小齿顶高系数（例如 ha*=0.5；本工具 ha* 下限就是 0.5）；或减小 α/齿顶高、增大齿数。", "Tip pointed / involutes crossing: with α={alpha}° and ha*={ha}, ψ(da/2)={psi}° ≤ 0 — the two tooth profiles meet before the tip circle, so the \"regular front view\" cannot draw a true involute profile (it reports an error; section/side/simplified front views are unaffected). 45° splines usually use a small addendum coefficient (e.g. ha*=0.5; the tool's lower bound is 0.5); or reduce α/addendum, or increase the number of teeth."),
    Msg::new("cmd.gear.note.internal_space_wide", "内齿轮齿槽过宽：α={alpha}° 配 ha*={ha} 时齿槽半角 ψ(da/2)={psi}° ≥ 半齿距 {half}° —— 相邻齿槽齿廓在齿顶圆之前相交，「端视图」画不出真实齿廓（会报错；剖视图不受影响）。减小 ha*/α 或增大齿数。", "Internal-gear space too wide: with α={alpha}° and ha*={ha}, the space half-angle ψ(da/2)={psi}° ≥ half pitch {half}° — adjacent space profiles meet before the tip circle, so the \"front view\" cannot draw a true profile (it reports an error; the section view is unaffected). Reduce ha*/α or increase the number of teeth."),
    Msg::new("cmd.gear.note.root_style_fallback", "齿根圆角无解：z={z}、m={m} 时齿廓起点半径 {rs} 与圆角圆心半径 {rc} 相差 {diff} > 圆角半径 ρ={rho}，模板口径的 0.38m 圆角放不下。已按{style}出图（想看真实根切曲线请用变位，或按 GB 允许的轻微根切另画）。", "Root fillet has no solution: for z={z}, m={m} the profile start radius {rs} differs from the fillet centre radius {rc} by {diff} > fillet radius ρ={rho}, so the template's 0.38m fillet does not fit. Drawn as {style} (use a profile shift for the real undercut curve, or draw the slight undercut allowed by GB separately)."),
    Msg::new("cmd.gear.rootstyle.template_arc", "齿根圆角（模板口径）", "root fillet (template convention)"),
    Msg::new("cmd.gear.rootstyle.base_circle_line", "基圆以下直线 + 圆角（FreeCAD 口径降级）", "line below the base circle + fillet (FreeCAD-convention fallback)"),
    Msg::new("cmd.gear.rootstyle.none", "无齿根圆角（直线到齿根圆）", "no root fillet (line to the root circle)"),
    Msg::new("cmd.gear.note.internal_fillet_none", "内齿轮齿根圆角无解（z={z}、m={m}）：rf−ρ={rc} 与齿廓之间放不下 ρ={rho} 的圆角，已按无圆角画（齿廓末端径向直线落到齿根圆）。", "Internal-gear root fillet has no solution (z={z}, m={m}): a fillet ρ={rho} does not fit between rf−ρ={rc} and the profile; drawn without a fillet (the end of the profile drops radially to the root circle)."),
    Msg::new("cmd.gear.note.internal_section_template", "内齿轮剖视图按模板只画到**齿根圆**，不打剖面线 —— 齿圈外壁结构（轮缘/腹板/键槽等）由用户/AI 按实际结构延伸，这样一张图能服务不同齿圈。延伸画法：从齿根线往外加厚齿圈（轴向宽度保持 h），新轮廓落 1轮廓实线层、剖面线用 ANSI31 放 5剖面线层（一个 HATCH 两个环，轴线上下各一环）。", "The internal-gear section view follows the template and draws only down to the **root circle**, without hatching — the outer ring structure (rim/web/keyway etc.) is extended by the user/AI per the actual part, so one drawing can serve different rings. To extend: thicken the ring outward from the root line (axial width stays h); the new outline goes on 1轮廓实线层 and ANSI31 hatching on 5剖面线层 (one HATCH with two loops, one above and one below the axis)."),
    Msg::new("cmd.gear.note.internal_tip_below_base", "内齿轮 z={z} 时齿顶圆 da={da} 低于基圆 db={db}：真实齿顶由插齿刀刀尖包络成形（非渐开线），本图按简化画法画「渐开线到基圆 → 径向直线到齿顶圆」。要精确齿顶画法请给模板。", "For the internal gear z={z} the tip circle da={da} lies below the base circle db={db}: the real tip is formed by the envelope of the gear-shaper cutter tip (non-involute); this drawing uses the simplified convention \"involute to the base circle → radial line to the tip circle\". Supply a template for an exact tip drawing."),
    Msg::new("cmd.gear.note.internal_helical", "内齿轮斜齿：剖视图按轴向剖面画，未画三条螺旋线细实线（外齿轮模板侧视图才有，内齿轮模板没有侧视图）。", "Helical internal gear: the section view is drawn as an axial section without the three thin helix lines (they exist only in the external-gear template's side view; the internal-gear template has no side view)."),
    Msg::new("cmd.gear.dir.right", "右旋", "right-hand"),
    Msg::new("cmd.gear.dir.left", "左旋", "left-hand"),
    Msg::new("cmd.gear.kind.external", "外齿轮", "external gear"),
    Msg::new("cmd.gear.kind.internal", "内齿轮", "internal gear"),
    Msg::new("cmd.gear.view.section", "剖视图", "section view"),
    Msg::new("cmd.gear.view.front", "端视图", "front view"),
    Msg::new("cmd.gear.view.side", "侧视图", "side view"),
    Msg::new("cmd.gear.view.simplified", "简化正视图", "simplified front view"),
    Msg::new("cmd.gear.view.section_internal", "剖视图（齿圈内齿不剖）", "section view (ring teeth not sectioned)"),
    Msg::new("cmd.gear.err.view_unavailable", "{kind}不提供「{view}」视图 —— 用户给的模板（内齿轮.dxf）里只有**剖视图**和**端视图**两个视图；\n模板没有的画法不猜（避免出一张看起来对、实际没依据的图）。要用请先给对应模板。", "{kind} does not provide the \"{view}\" view — the template supplied by the user (内齿轮.dxf) has only **section view** and **front view**;\nwe do not guess a drawing the template lacks (to avoid a drawing that looks right but has no basis). Supply the matching template first."),
    Msg::new("cmd.gear.err.view_unavailable_short", "{kind}不提供「{view}」视图（见上文）。", "{kind} does not provide the \"{view}\" view (see above)."),
    Msg::new("cmd.gear.err.internal_spline_no_side", "内花键不提供「侧视图」—— 用户定案：内花键剖视图和内齿轮一样，不存在侧视图。\n内花键可用视图只有**剖视图**和**端视图**两个（与内齿轮同模板口径）；模板没有的画法不猜（避免出一张看起来对、实际没依据的图）。", "Internal splines do not provide a \"side view\" — decided by the user: like internal gears, an internal spline has no side view.\nThe only available views are **section view** and **front view** (same template convention as internal gears); we do not guess a drawing the template lacks (to avoid a drawing that looks right but has no basis)."),
    Msg::new("cmd.gear.err.internal_spline_view_unavailable", "内花键不提供「{view}」视图 —— 内花键与内齿轮同口径，只有 **剖视图 + 端视图** 两个视图。", "Internal splines do not provide the \"{view}\" view — same convention as internal gears: only **section view + front view**."),
    Msg::new("cmd.gear.err.spline_view_unavailable", "花键不提供「{view}」视图 —— 花键只有 端视图 / 侧视图 / 剖视图 三个视图（端视图=真实渐开线齿廓；侧视/剖视=轴向轮廓）。", "Splines do not provide the \"{view}\" view — a spline has only front view / side view / section view (front = true involute profile; side/section = axial outline)."),
    Msg::new("cmd.gear.err.internal_spline_chamfer_len", "内花键（齿圈）轴向倒角 C=round(0.6m)={c} 太大（有效长度 L={len}）——加大 L 或减小模数（同内齿轮校验口径）。", "Internal-spline (ring) axial chamfer C=round(0.6m)={c} is too large (effective length L={len}) — increase L or reduce the module (same check as internal gears)."),
    Msg::new("cmd.gear.err.internal_spline_chamfer_radius", "内花键（齿圈）轴向倒角 C={c} 不小于小径半径 D_ii/2={r}，无法画图（同内齿轮口径）。", "Internal-spline (ring) axial chamfer C={c} is not smaller than the minor-diameter radius D_ii/2={r}; cannot draw (same convention as internal gears)."),
    Msg::new("cmd.gear.warn.missing_layers", "（缺图层：{list}）", " (missing layers: {list})"),
    Msg::new("cmd.gear.warn.no_center2", "（{layer} 没挂 CENTER2 点划线）", " ({layer} has no CENTER2 dash-dot linetype)"),
    Msg::new("cmd.gear.sep.list", "、", ", "),
    Msg::new("cmd.gear.err.not_initialized", "这张图还没跑过 OCSM 初始化{why} —— 先执行 OCSM（建 10 个图层 + 线型 + 文字/标注样式），再生成齿轮；不然中心线会是实线白线、剖面线层颜色也不对。", "This drawing has not been through OCSM initialization{why} — run OCSM first (creates the 10 layers + linetypes + text/dimension styles), then generate the gear; otherwise centrelines will be white solid lines and the hatch-layer colour will be wrong."),
    Msg::new("cmd.gear.name.pair", "{a}（{b}）", "{a} ({b})"),
    Msg::new("cmd.gear.spec.internal_prefix", "内齿轮 ", "internal gear "),
    Msg::new("cmd.gear.spec.dp", "DP{dp}（m{m}）", "DP{dp} (m{m})"),
    Msg::new("cmd.gear.report.title", "# {kind} {std} 计算书\n\n", "# {kind} {std} computation report\n\n"),
    Msg::new("cmd.gear.report.std_dp", "（径节制 DP）", "(diametral pitch DP)"),
    Msg::new("cmd.gear.report.std_m", "（模数制 M）", "(module M)"),
    Msg::new("cmd.gear.report.units", "- 单位口径：长度 mm，角度 °；DP 体系 m = 25.4/DP。\n\n", "- Units: length mm, angle °; in DP mode m = 25.4/DP.\n\n"),
    Msg::new("cmd.gear.report.h_inputs", "## 1. 输入参数\n\n| 输入 | 原值 | 说明 |\n|---|---|---|\n", "## 1. Input parameters\n\n| Input | Value | Notes |\n|---|---|---|\n"),
    Msg::new("cmd.gear.report.row_dp", "| 径节 DP | {v} | m = 25.4/DP |\n", "| Diametral pitch DP | {v} | m = 25.4/DP |\n"),
    Msg::new("cmd.gear.report.row_m_conv", "| 模数 m（换算） | {v} mm | 25.4/DP |\n", "| Module m (converted) | {v} mm | 25.4/DP |\n"),
    Msg::new("cmd.gear.report.row_m", "| 模数 m | {v} mm | 模数制 |\n", "| Module m | {v} mm | module mode |\n"),
    Msg::new("cmd.gear.report.row_z", "| 齿数 z | {v} | — |\n", "| Teeth z | {v} | — |\n"),
    Msg::new("cmd.gear.report.row_alpha", "| 压力角 αn | {v}° | — |\n", "| Pressure angle αn | {v}° | — |\n"),
    Msg::new("cmd.gear.report.row_hcx", "| 齿顶高/顶隙/变位系数 | ha*={ha}、c*={c}、x={x} | — |\n", "| Addendum/clearance/shift | ha*={ha}, c*={c}, x={x} | — |\n"),
    Msg::new("cmd.gear.report.row_beta", "| 螺旋角 β | {v}° | — |\n", "| Helix angle β | {v}° | — |\n"),
    Msg::new("cmd.gear.report.row_h", "| 厚度 h | {v} mm | — |\n\n", "| Thickness h | {v} mm | — |\n\n"),
    Msg::new("cmd.gear.report.h_steps", "## 2. 逐步计算\n\n", "## 2. Step-by-step calculation\n\n"),
    Msg::new("cmd.gear.report.rows_head", "| # | 步骤 | 公式（符号含义） | 代入 | 结果 | 依据来源 |\n", "| # | Step | Formula (symbol meaning) | Substitution | Result | Source |\n"),
    Msg::new("cmd.gear.report.step.dp", "径节换算", "DP conversion"),
    Msg::new("cmd.gear.report.formula.dp", "m = 25.4 / DP（DP 径节，1/in）", "m = 25.4 / DP (DP pitch, 1/in)"),
    Msg::new("cmd.gear.report.source.dp", "径节制定义（与 OCSMGEAR 同口径）", "definition of diametral pitch (same convention as OCSMGEAR)"),
    Msg::new("cmd.gear.report.step.face", "端面换算", "Transverse conversion"),
    Msg::new("cmd.gear.report.formula.face", "mt = m / cosβ；αt = atan(tanαn / cosβ)", "mt = m / cosβ; αt = atan(tanαn / cosβ)"),
    Msg::new("cmd.gear.report.source.face", "斜齿轮端面几何（与 gear.rs 同式）", "helical-gear transverse geometry (same formulas as gear.rs)"),
    Msg::new("cmd.gear.report.step.face_spur", "端面参数", "Transverse parameters"),
    Msg::new("cmd.gear.report.formula.face_spur", "直齿：mt = m；αt = αn", "Spur gear: mt = m; αt = αn"),
    Msg::new("cmd.gear.report.source.face_spur", "直齿退化为法向参数", "spur gear degenerates to normal parameters"),
    Msg::new("cmd.gear.report.step.d", "分度圆直径", "Pitch diameter"),
    Msg::new("cmd.gear.report.formula.d", "d = mt·z（mt 端面模数，z 齿数）", "d = mt·z (mt transverse module, z teeth)"),
    Msg::new("cmd.gear.report.source.geom", "齿轮几何（与 OCSMGEAR 同式）", "gear geometry (same formulas as OCSMGEAR)"),
    Msg::new("cmd.gear.report.step.db", "基圆直径", "Base diameter"),
    Msg::new("cmd.gear.report.formula.db", "db = d·cosαt", "db = d·cosαt"),
    Msg::new("cmd.gear.report.source.db", "渐开线基圆定义", "involute base-circle definition"),
    Msg::new("cmd.gear.report.step.ha", "齿顶高", "Addendum"),
    Msg::new("cmd.gear.report.formula.ha", "ha = m·(ha* + x)", "ha = m·(ha* + x)"),
    Msg::new("cmd.gear.report.source.ha", "齿轮齿顶高公式", "gear addendum formula"),
    Msg::new("cmd.gear.report.step.hf", "齿根高", "Dedendum"),
    Msg::new("cmd.gear.report.formula.hf", "hf = m·(ha* + c* − x)", "hf = m·(ha* + c* − x)"),
    Msg::new("cmd.gear.report.source.hf", "齿轮齿根高公式", "gear dedendum formula"),
    Msg::new("cmd.gear.report.step.da", "齿顶圆直径", "Tip diameter"),
    Msg::new("cmd.gear.report.formula.da_int", "da = d − 2ha（内齿轮齿顶朝圆心）", "da = d − 2ha (internal-gear tip points to the centre)"),
    Msg::new("cmd.gear.report.formula.da_ext", "da = d + 2ha", "da = d + 2ha"),
    Msg::new("cmd.gear.report.step.df", "齿根圆直径", "Root diameter"),
    Msg::new("cmd.gear.report.formula.df_int", "df = d + 2hf（内齿轮齿根在外）", "df = d + 2hf (internal-gear root outside)"),
    Msg::new("cmd.gear.report.formula.df_ext", "df = d − 2hf", "df = d − 2hf"),
    Msg::new("cmd.gear.report.step.st", "端面分度圆齿厚", "Transverse pitch-circle tooth thickness"),
    Msg::new("cmd.gear.report.formula.st", "st = πm/(2cosβ) + 2x·m·tanαn", "st = πm/(2cosβ) + 2x·m·tanαn"),
    Msg::new("cmd.gear.report.source.st", "齿轮齿厚公式（与 OCSMGEAR 同式）", "gear tooth-thickness formula (same as OCSMGEAR)"),
    Msg::new("cmd.gear.report.step.rho", "齿根圆角半径", "Root fillet radius"),
    Msg::new("cmd.gear.report.formula.rho", "ρ = 0.38·m", "ρ = 0.38·m"),
    Msg::new("cmd.gear.report.source.rho", "OCSMGEAR 齿根圆角口径（模板反解）", "OCSMGEAR root-fillet convention (reverse-engineered from the template)"),
    Msg::new("cmd.gear.report.step.chamfer", "轴向倒角", "Axial chamfer"),
    Msg::new("cmd.gear.report.formula.chamfer", "C = round(0.6·m)", "C = round(0.6·m)"),
    Msg::new("cmd.gear.report.source.chamfer", "OCSMGEAR 倒角口径（模板反解）", "OCSMGEAR chamfer convention (reverse-engineered from the template)"),
    Msg::new("cmd.gear.report.h_derived", "## 3. 派生几何\n\n| 量 | 值 |\n|---|---|\n", "## 3. Derived geometry\n\n| Quantity | Value |\n|---|---|\n"),
    Msg::new("cmd.gear.report.row_d", "| 分度圆 d | {v} mm |\n", "| Pitch d | {v} mm |\n"),
    Msg::new("cmd.gear.report.row_db", "| 基圆 db | {v} mm |\n", "| Base db | {v} mm |\n"),
    Msg::new("cmd.gear.report.row_da", "| 齿顶圆 da | {v} mm |\n", "| Tip da | {v} mm |\n"),
    Msg::new("cmd.gear.report.row_df", "| 齿根圆 df | {v} mm |\n", "| Root df | {v} mm |\n"),
    Msg::new("cmd.gear.report.row_mt", "| 端面模数 mt | {v} mm |\n", "| Transverse module mt | {v} mm |\n"),
    Msg::new("cmd.gear.report.row_alpha_t", "| 端面压力角 αt | {v}° |\n", "| Transverse pressure angle αt | {v}° |\n"),
    Msg::new("cmd.gear.report.row_pitch_angle", "| 齿距角 | {v}° |\n", "| Pitch angle | {v}° |\n"),
    Msg::new("cmd.gear.report.h_checks", "## 4. 检验尺寸\n\n", "## 4. Inspection dimensions\n\n"),
    Msg::new("cmd.gear.report.no_checks", "齿轮模式无入库检验尺寸表（检验表属 DIN 5480-2 花键体系）。\n\n", "Gear mode has no stored inspection-dimension table (inspection tables belong to the DIN 5480-2 spline system).\n\n"),
    Msg::new("cmd.gear.report.h_sources", "## 5. 数据来源与校验\n\n", "## 5. Data sources and checks\n\n"),
    Msg::new("cmd.gear.report.source_engine", "- 数据来源：OCSM 齿轮引擎（`gear.rs`），与 OCSMGEAR/GUI 同一套公式；未二次手算。\n", "- Data source: the OCSM gear engine (`gear.rs`), the same formulas as OCSMGEAR/GUI; not hand-recomputed.\n"),
    Msg::new("cmd.gear.report.source_dp", "- 径节制：m = 25.4/DP = {m}（原值 DP={dp}）；块名/规格按 DP 原值出。\n", "- Diametral pitch: m = 25.4/DP = {m} (original DP={dp}); block name/spec use the original DP value.\n"),
    Msg::new("cmd.gear.report.identity", "- 恒等式自检（定义式，残差应 0）：|d−mt·z|={r1}、|db−d·cosαt|={r2}；违例 0。\n", "- Identity self-check (definition equations, residuals should be 0): |d−mt·z|={r1}, |db−d·cosαt|={r2}; violations 0.\n"),
    Msg::new("cmd.gear.report.note_line", "- 提示：{note}\n", "- Note: {note}\n"),
    Msg::new("cmd.gear.report.subst.face", "m={m}，β={beta}°，αn={alpha}°", "m={m}, β={beta}°, αn={alpha}°"),
    Msg::new("cmd.gear.report.result.face", "mt={mt} mm，αt={at}°", "mt={mt} mm, αt={at}°"),
    Msg::new("cmd.gear.report.subst.face_spur", "β=0，m={m}，αn={alpha}°", "β=0, m={m}, αn={alpha}°"),
    Msg::new("cmd.gear.err.spline_spur_only", "渐开线花键为直齿（β 必须为 0）；收到 β={v}°。", "Involute splines are spur (β must be 0); got β={v}°."),
    // —— 引导线族 GUI 显示名（⑤）——
    Msg::new("cmd.invol.err.gb_db", "基准直径 d_B 是 DIN 5480 的概念，GB/T 3478 体系请给 m 与 z（本体系不用 d_B）", "Reference diameter d_B is a DIN 5480 concept; the GB/T 3478 system takes m and z (this system does not use d_B)"),
    Msg::new("cmd.invol.err.gb_x", "变位系数 x 是 DIN 5480/NF E22-141 的概念，GB/T 3478 基本齿廓不含变位（x 恒为 0）；请去掉 x（齿轮生成器已锁死为 0）", "Profile shift x is a DIN 5480/NF E22-141 concept; the GB/T 3478 basic profile has no shift (x is always 0); remove x (the gear generator already locks it to 0)"),
    Msg::new("cmd.invol.err.gb_x_received", "{msg}（收到 x={x}）", "{msg} (got x={x})"),
    Msg::new("cmd.invol.err.pitch_only_ansi", "径节 P/Ps 是 ANSI B92.1 的写法（P=每英寸齿数、Ps=2P）；GB/DIN/NF 用模数 m（写法 `M…`），不要给径节 P/DP", "Pitch P/Ps is the ANSI B92.1 notation (P = teeth per inch, Ps = 2P); GB/DIN/NF use module m (written `M…`), do not give pitch P/DP"),
    Msg::new("cmd.invol.err.ansi_db", "ANSI B92.1 不使用基准直径 d_B/A（径节制用径节 P/Ps 与压力角）；请给径节 P（与齿数 N）", "ANSI B92.1 does not use reference diameter d_B/A (diametral-pitch mode uses P/Ps and the pressure angle); give pitch P (and tooth count N)"),
    Msg::new("cmd.invol.pitch.form", "径节写法应为 A/B（如 2.5/5、3/6…128/256），Ps 恒为 2P", "Pitch is written A/B (e.g. 2.5/5, 3/6…128/256); Ps is always 2P"),
    Msg::new("cmd.invol.pitch.err.missing", "ANSI B92.1：缺径节 P（{form}）。", "ANSI B92.1: missing pitch P ({form})."),
    Msg::new("cmd.invol.pitch.err.bad_format", "ANSI B92.1：径节「{s}」格式不对（{form}）。", "ANSI B92.1: pitch \"{s}\" has a bad format ({form})."),
    Msg::new("cmd.invol.pitch.err.a_not_number", "ANSI B92.1：径节「{s}」的分子 A 不是数字（{form}）。", "ANSI B92.1: the numerator A of pitch \"{s}\" is not a number ({form})."),
    Msg::new("cmd.invol.pitch.err.b_not_number", "ANSI B92.1：径节「{s}」的分母 B 不是数字（{form}）。", "ANSI B92.1: the denominator B of pitch \"{s}\" is not a number ({form})."),
    Msg::new("cmd.invol.pitch.err.not_positive", "ANSI B92.1：径节 A/B 的 A、B 都必须是正数；收到「{s}」。", "ANSI B92.1: both A and B of pitch A/B must be positive; got \"{s}\"."),
    Msg::new("cmd.invol.pitch.err.b_not_2a", "ANSI B92.1：径节写法 A/B 的 B 是 stub pitch Ps，标准中 Ps 恒为 2P；收到 {a}/{b}，应写 {a2}/{b2}。", "ANSI B92.1: in the A/B notation B is the stub pitch Ps, which is always 2P; got {a}/{b}, write {a2}/{b2}."),
    Msg::new("cmd.invol.pitch.err.p_positive", "ANSI B92.1：径节 P={p} 必须是正数。", "ANSI B92.1: pitch P={p} must be positive."),
    Msg::new("cmd.invol.pitch.err.not_number", "ANSI B92.1：径节「{s}」不是数字（{form}）。", "ANSI B92.1: pitch \"{s}\" is not a number ({form})."),
    Msg::new("cmd.invol.pitch.err.series", "ANSI B92.1：径节 P={p} 不在标准系列（17 项：{list}）；{form}。", "ANSI B92.1: pitch P={p} is not in the standard series (17 items: {list}); {form}."),
    Msg::new("cmd.invol.err.prefix", "{code}：{e}", "{code}: {e}"),
    Msg::new("cmd.invol.column.err.range", "ANSI B92.1 Table 2「{col}」列的适用径节为 {range}，收到 {got}（{side}）。", "ANSI B92.1 Table 2 column \"{col}\" accepts pitches {range}; got {got} ({side})."),
    Msg::new("cmd.invol.column.below", "低于下限", "below the lower limit"),
    Msg::new("cmd.invol.column.above", "超出上限", "above the upper limit"),
    Msg::new("cmd.invol.err.profile_std_mismatch", "齿廓代号「{profile}」属于 {pstd}，与所选标准 {std} 不符。", "Profile token \"{profile}\" belongs to {pstd}, which does not match the selected standard {std}."),
    Msg::new("cmd.invol.err.ansi_missing_p", "ANSI B92.1：缺径节 P（写法 `P5/10`（A/B）或 `P8`（裸数字，需在 17 项系列）；也兼容 `pitch=5/10` 与 `m=` 槽位）", "ANSI B92.1: missing pitch P (write `P5/10` (A/B) or `P8` (bare number, must be in the 17-item series); `pitch=5/10` and the `m=` slot are also accepted)"),
    Msg::new("cmd.invol.err.ansi_missing_n", "ANSI B92.1：缺齿数 N（写法 `Z20`）", "ANSI B92.1: missing tooth count N (write `Z20`)"),
    Msg::new("cmd.invol.err.ansi_x", "ANSI B92.1：不使用变位系数 x（Table 2 基本尺寸无 x 项）；收到 x={x}。", "ANSI B92.1: profile shift x is not used (Table 2 basic dimensions have no x term); got x={x}."),
    Msg::new("cmd.invol.err.nf_missing_m", "NF E22-141：缺模数 m（给 A 与 m，或给 A 与 z 由表/公式推导另一项）", "NF E22-141: missing module m (give A and m, or A and z and derive the other from the table/formula)"),
    Msg::new("cmd.invol.err.nf_missing_n", "NF E22-141：缺齿数 N（给 A 与 N，或给 A 与 m 由表/公式推导另一项）", "NF E22-141: missing tooth count N (give A and N, or A and m and derive the other from the table/formula)"),
    Msg::new("cmd.invol.err.nf_prefix", "NF E22-141：{e}", "NF E22-141: {e}"),
    Msg::new("cmd.invol.err.missing_m_gb", "GB/T 3478：缺模数 m（写法 `M3`）", "GB/T 3478: missing module m (write `M3`)"),
    Msg::new("cmd.invol.err.missing_m_din", "DIN 5480：缺模数 m（给 m 与 z，或给基准直径 d_B 由表补全）", "DIN 5480: missing module m (give m and z, or d_B to complete from the table)"),
    Msg::new("cmd.invol.err.missing_m_nf", "NF E22-141：缺模数 m（给 A 与 m，或给 A 与 z）", "NF E22-141: missing module m (give A and m, or A and z)"),
    Msg::new("cmd.invol.err.missing_m_ansi", "ANSI B92.1：缺径节 P（写法 `P5/10`（A/B）或 `P8`）", "ANSI B92.1: missing pitch P (write `P5/10` (A/B) or `P8`)"),
    Msg::new("cmd.invol.err.missing_z_gb", "GB/T 3478：缺齿数 z（写法 `Z20`）", "GB/T 3478: missing tooth count z (write `Z20`)"),
    Msg::new("cmd.invol.err.missing_z_din", "DIN 5480：缺齿数 z（给 m 与 z，或给基准直径 d_B 由表补全）", "DIN 5480: missing tooth count z (give m and z, or d_B to complete from the table)"),
    Msg::new("cmd.invol.err.missing_z_nf", "NF E22-141：缺齿数 N（给 A 与 N，或给 A 与 m）", "NF E22-141: missing tooth count N (give A and N, or A and m)"),
    Msg::new("cmd.invol.err.missing_z_ansi", "ANSI B92.1：缺齿数 N（写法 `Z20`）", "ANSI B92.1: missing tooth count N (write `Z20`)"),
    Msg::new("cmd.gear.err.system_pitch", "{code} 体系：{msg}。", "{code} system: {msg}."),
    Msg::new("cmd.gear.err.p_pitch_missing", "径节 P 需要 A/B 写法（{form}），如 `p=2.5/5`。", "Pitch P must use the A/B form ({form}), e.g. `p=2.5/5`."),
    Msg::new("cmd.guide.linear.h", "水平", "horizontal"),
    Msg::new("cmd.guide.linear.v", "竖直", "vertical"),
    Msg::new("cmd.guide.linear.a", "对齐", "aligned"),
    Msg::new("cmd.guide.section.right", "向右", "right"),
    Msg::new("cmd.guide.section.left", "向左", "left"),
    Msg::new("cmd.guide.angle.minor", "劣角", "minor angle"),
    Msg::new("cmd.guide.angle.reflex", "优角", "reflex angle"),
    Msg::new("cmd.guide.grind.none", "不打磨", "no grinding"),
    Msg::new("cmd.guide.grind.cav", "弧·凹", "concave arc"),
    Msg::new("cmd.guide.grind.cvx", "弧·凸", "convex arc"),
    Msg::new("cmd.guide.grind.line", "直线", "straight"),
    Msg::new("cmd.guide.grind.dbl", "双弧", "double arc"),
    Msg::new("cmd.guide.grind.zig", "锯齿", "zigzag"),

    Msg::new("cmd.gear.err.two_spline_stds", "同一行出现两个体系的标识：{v0} 与 {v1}（体系只能写一个）。", "Two system tokens on the same line: {v0} and {v1} (only one system may be given)."),
    Msg::new("cmd.gear.err.gear_spline_std_conflict", "同一行出现两个体系的标识：{v0} 是齿轮体系（M = 模数制 / DP = 径节制），不能与花键体系（GB/DIN/NF/ANSI）同用。", "Two system tokens on the same line: {v0} is a gear system (M = module / DP = diametral pitch) and cannot be combined with a spline system (GB/DIN/NF/ANSI)."),
    Msg::new("cmd.invol.err.m_positive", "渐开线花键：模数 m={v0} 必须是正数。", "Involute spline: module m={v0} must be positive."),
    Msg::new("cmd.invol.err.z_range", "渐开线花键：齿数 z={v0} 超出范围（3..=1000）。", "Involute spline: tooth count z={v0} is out of range (3..=1000)."),
    Msg::new("cmd.invol.err.alpha_range", "渐开线花键：压力角 α 超出范围（10°<α<50°）：{v0}°。", "Involute spline: pressure angle α is out of range (10°<α<50°): {v0}°."),
    Msg::new("cmd.invol.err.ha_positive", "渐开线花键：齿顶高系数 ha*={v0} 必须是正数。", "Involute spline: addendum coefficient ha*={v0} must be positive."),
    Msg::new("cmd.invol.err.hf_positive", "渐开线花键：齿根高系数 hf*={v0} 必须是正数。", "Involute spline: dedendum coefficient hf*={v0} must be positive."),
    Msg::new("cmd.invol.err.rho_negative", "渐开线花键：齿根圆角系数 ρf*={v0} 不能为负。", "Involute spline: root fillet coefficient ρf*={v0} must not be negative."),
    Msg::new("cmd.invol.err.cf_negative", "渐开线花键：齿形裕度系数 cF*={v0} 不能为负。", "Involute spline: form clearance coefficient cF*={v0} must not be negative."),
    Msg::new("cmd.invol.err.x_finite", "渐开线花键：变位系数 x={v0} 必须是有限数。", "Involute spline: profile shift x={v0} must be finite."),
    Msg::new("cmd.invol.err.din_x_range", "DIN 5480：变位系数 x={v0} 超出 x·m∈[−0.05m, +0.45m]（x∈[−0.05, 0.45]）。", "DIN 5480: profile shift x={v0} is outside x·m∈[−0.05m, +0.45m] (x∈[−0.05, 0.45])."),
    Msg::new("cmd.invol.err.ansi_col_unknown", "ANSI B92.1：齿廓「{v0}」不是 Table 2 五列之一（用 `ANSI30P`/`ANSI30PM`/`ANSI30R`/`ANSI375R`/`ANSI45R`）。", "ANSI B92.1: profile “{v0}” is not one of the five Table 2 columns (use `ANSI30P`/`ANSI30PM`/`ANSI30R`/`ANSI375R`/`ANSI45R`)."),
    Msg::new("cmd.invol.err.ansi_no_x", "ANSI B92.1：不使用变位系数 x（Table 2 基本尺寸无 x 项）；收到 x={v0}。", "ANSI B92.1: profile shift x is not used (Table 2 basic dimensions have no x term); got x={v0}."),
    Msg::new("cmd.invol.err.df_nonpositive", "渐开线花键：齿根圆直径 df={v0} 非正（检查 m/z/x/hf* 组合）。", "Involute spline: root diameter df={v0} is not positive (check the m/z/x/hf* combination)."),
    Msg::new("cmd.invol.err.da_not_gt_df", "渐开线花键：齿顶圆 da={v0} 不大于齿根圆 df={v1}。", "Involute spline: tip diameter da={v0} is not greater than root diameter df={v1}."),
    Msg::new("cmd.invol.err.internal_minor_nonpositive", "内花键：小径 D_ii={v0} 非正（检查 m/z/x/系数组合）。", "Internal spline: minor diameter D_ii={v0} is not positive (check the m/z/x/coefficient combination)."),
    Msg::new("cmd.invol.err.internal_major_le_minor", "内花键：大径 D_ei={v0} 不大于小径 D_ii={v1}。", "Internal spline: major diameter D_ei={v0} is not greater than minor diameter D_ii={v1}."),
    Msg::new("cmd.invol.err.preset_unknown", "{v0} 没有齿廓预设「{v1}」；可用：{v2}", "{v0} has no profile preset “{v1}”; available: {v2}"),
    Msg::new("cmd.invol.err.ansi_no_flat_col", "ANSI B92.1 Table 2 没有 {v0}° 平齿根列（37.5°/45° 只有圆齿根）", "ANSI B92.1 Table 2 has no {v0}° flat-root column (37.5°/45° have fillet roots only)"),
    Msg::new("cmd.invol.err.ansi_fillet_side_only", "ANSI B92.1 Table 2 的圆齿根只有齿侧配合（外径配合仅 30° 平齿根列 B）", "ANSI B92.1 Table 2 fillet roots are side-fit only (major-diameter fit exists only for the 30° flat-root column B)"),
    Msg::new("cmd.invol.err.ansi_alpha_unsupported", "ANSI B92.1 压力角只支持 30° / 37.5° / 45°（收到 {v0}°）；组合见 Table 2 五列", "ANSI B92.1 supports pressure angles 30° / 37.5° / 45° only (got {v0}°); see the five Table 2 columns for combinations"),
    Msg::new("cmd.invol.err.series_m_positive", "{v0}：模数 m={v1} 必须是正数。", "{v0}: module m={v1} must be positive."),
    Msg::new("cmd.invol.err.series_m_not_in_list", "{v0}：模数 m={v1} 不在体系可用系列（来源：{v2}；可用值：{v3}）。", "{v0}: module m={v1} is not in the system’s available series (source: {v2}; available: {v3})."),
    Msg::new("cmd.invol.din.db_positive", "DIN 5480：d_B={v0} 必须是正数。", "DIN 5480: d_B={v0} must be positive."),
    Msg::new("cmd.invol.din.m_positive", "DIN 5480：m={v0} 必须是正数。", "DIN 5480: m={v0} must be positive."),
    Msg::new("cmd.invol.din.db_multi_hit", "DIN 5480：d_B={v0} 命中 {v1} 行（同一 d_B 多个 m/z/x₁ 变体）：{v2}；请再给 M 和/或 Z。", "DIN 5480: d_B={v0} matched {v1} rows (the same d_B has several m/z/x₁ variants): {v2}; give M and/or Z as well."),
    Msg::new("cmd.invol.din.db_m_multi_z", "DIN 5480：d_B={v0}、m={v1} 有多个 z 变体：{v2}；请再给 Z（同一 d_B 多个 z/x₁ 是表格事实）。", "DIN 5480: d_B={v0}, m={v1} has several z variants: {v2}; give Z as well (several z/x₁ for one d_B is a table fact)."),
    Msg::new("cmd.invol.din.db_z_multi_m", "DIN 5480：d_B={v0}、z={v1} 有多个 m 变体：{v2}；请再给 M。", "DIN 5480: d_B={v0}, z={v1} has several m variants: {v2}; give M as well."),
    Msg::new("cmd.invol.din.z_no_feasible", "DIN 5480：由 d_B={v0}、m={v1} 与 x∈[−0.05,0.45] 得不到可行齿数（z∈[{v2},{v3}]）——请核对 d_B 与 m。", "DIN 5480: d_B={v0}, m={v1} with x∈[−0.05,0.45] yields no feasible tooth count (z∈[{v2},{v3}]) — check d_B and m."),
    Msg::new("cmd.invol.din.derive_z_note", "推导值、未命中表：d_B={v0}、m={v1} 不在 DIN 5480-2 名义表；由 d_B=m(z+1.1+2x)、x∈[−0.05,0.45] 得 z∈[{v2},{v3}]，取 z={v4}（x={v5}）", "Derived value, table not hit: d_B={v0}, m={v1} are not in the DIN 5480-2 nominal table; from d_B=m(z+1.1+2x), x∈[−0.05,0.45] we get z∈[{v2},{v3}], take z={v4} (x={v5})"),
    Msg::new("cmd.invol.din.derive_m_note", "推导值、未命中表：d_B={v0}、z={v1} 不在 DIN 5480-2 名义表；由 d_B=m(z+1.1+2x) 取 x=0 得 m={v2}（可行区间 m∈[{v3},{v4}]）", "Derived value, table not hit: d_B={v0}, z={v1} are not in the DIN 5480-2 nominal table; from d_B=m(z+1.1+2x) with x=0 we get m={v2} (feasible range m∈[{v3},{v4}])"),
    Msg::new("cmd.invol.din.prefix", "DIN 5480：{v0}", "DIN 5480: {v0}"),
    Msg::new("cmd.invol.din.lookup_db_positive", "DIN 5480-2 查表：d_B={v0} 必须是正数。", "DIN 5480-2 lookup: d_B={v0} must be positive."),
    Msg::new("cmd.invol.din.lookup_m_positive", "DIN 5480-2 查表：m={v0} 必须是正数。", "DIN 5480-2 lookup: m={v0} must be positive."),
    Msg::new("cmd.invol.din.lookup_head", "DIN 5480-2 查表：d_B={v0}", "DIN 5480-2 lookup: d_B={v0}"),
    Msg::new("cmd.invol.din.lookup_head_m", "（m={v0}）", " (m={v0})"),
    Msg::new("cmd.invol.din.lookup_no_hit", " 无命中", " no hit"),
    Msg::new("cmd.invol.table.no_module_step", "；表中没有 m={v0} 档（已入库档位：{v1}）。", "; the table has no m={v0} step (loaded steps: {v1})."),
    Msg::new("cmd.invol.din.cand_row", "d_B={v0}（m={v1}）", "d_B={v0} (m={v1})"),
    Msg::new("cmd.invol.table.candidates", "附近候选：{v0}。", "Nearby candidates: {v0}."),
    Msg::new("cmd.invol.din.rows_join", "m={v0} z={v1} x={v2}", "m={v0} z={v1} x={v2}"),
    Msg::new("cmd.invol.nf.lookup_a_positive", "NF E22-141 查表：A={v0} 必须是正数。", "NF E22-141 lookup: A={v0} must be positive."),
    Msg::new("cmd.invol.nf.lookup_m_positive", "NF E22-141 查表：m={v0} 必须是正数。", "NF E22-141 lookup: m={v0} must be positive."),
    Msg::new("cmd.invol.nf.lookup_head", "NF E22-141 查表：A={v0}", "NF E22-141 lookup: A={v0}"),
    Msg::new("cmd.invol.nf.a_positive", "NF E22-141：A={v0} 必须是正数。", "NF E22-141: A={v0} must be positive."),
    Msg::new("cmd.invol.nf.m_positive", "NF E22-141：m={v0} 必须是正数。", "NF E22-141: m={v0} must be positive."),
    Msg::new("cmd.invol.nf.row_tag", "NF E22-141：查表行 p{v0} m={v1} N={v2}：{v3}", "NF E22-141: table row p{v0} m={v1} N={v2}: {v3}"),
    Msg::new("cmd.invol.nf.z_out_of_range", "NF E22-141：由 A={v0}、m={v1} 取 x={v2} 得 N={v3} 超出 1..=1000（表内 x∈[0.6,0.967]）——请核对 A 与 m。", "NF E22-141: A={v0}, m={v1} with x={v2} gives N={v3}, outside 1..=1000 (x∈[0.6,0.967] in the table) — check A and m."),
    Msg::new("cmd.invol.nf.bench_positive", "{v0}：基准直径/公称直径={v1} 必须是正数。", "{v0}: reference/nominal diameter={v1} must be positive."),
    Msg::new("cmd.invol.nf.a_m_multi_z", "NF E22-141：A={v0}、m={v1} 有多个 N 变体：{v2}；请再给 N。", "NF E22-141: A={v0}, m={v1} has several N variants: {v2}; give N as well."),
    Msg::new("cmd.invol.nf.a_z_multi_m", "NF E22-141：A={v0}、N={v1} 有多个 m 变体：{v2}；请再给 M。", "NF E22-141: A={v0}, N={v1} has several m variants: {v2}; give M as well."),
    Msg::new("cmd.invol.nf.a_multi_hit", "NF E22-141：A={v0} 命中 {v1} 行：{v2}；请再给 M 和/或 N。", "NF E22-141: A={v0} matched {v1} rows: {v2}; give M and/or N as well."),
    Msg::new("cmd.invol.nf.derive_z_note", "推导值、未命中表：A={v0}、m={v1} 不在 NF E22-141 尺寸表；按主系列 x={v2} 取 N={v3}（表内 x∈[0.6,0.967] 时 N∈[{v4},{v5}]）", "Derived value, table not hit: A={v0}, m={v1} are not in the NF E22-141 dimension table; with the main series x={v2} we take N={v3} (with x∈[0.6,0.967] in the table, N∈[{v4},{v5}])"),
    Msg::new("cmd.invol.nf.derive_m_note", "推导值、未命中表：A={v0}、N={v1} 不在 NF E22-141 尺寸表；按 x={v2} 取名义 m=A/(N+2x+0.4)={v3}", "Derived value, table not hit: A={v0}, N={v1} are not in the NF E22-141 dimension table; with x={v2} we take the nominal m=A/(N+2x+0.4)={v3}"),
    Msg::new("cmd.invol.nf.note_multi_variant", "查表命中 A={v0}、m={v1} 的多个 N 变体（{v2}），按最接近主系列 x=0.8 取 N={v3}", "Table hit: several N variants for A={v0}, m={v1} ({v2}); take N={v3}, the one closest to the main series x=0.8"),
    Msg::new("cmd.invol.nf.derive_z_table_note", "表外推导：d_B={v0}、m={v1} 不在 DIN 5480-2 名义表；由 d_B=m(z+1.1+2x)、x∈[−0.05,0.45] 得 z∈[{v2},{v3}]，取 z={v4}（x={v5}）", "Derived outside the table: d_B={v0}, m={v1} are not in the DIN 5480-2 nominal table; from d_B=m(z+1.1+2x), x∈[−0.05,0.45] we get z∈[{v2},{v3}], take z={v4} (x={v5})"),
    Msg::new("cmd.invol.derive_z_not_applicable", "{v0} 不使用基准直径推齿数（GB 用 m/z/x，ANSI 用径节 P 与齿数 N）。", "{v0} does not derive the tooth count from a reference diameter (GB uses m/z/x, ANSI uses pitch P and tooth count N)."),
    Msg::new("cmd.invol.nf.rows_join", "N={v0}（x={v1}）", "N={v0} (x={v1})"),
    Msg::new("cmd.invol.sep.list", "、", ", "),
    Msg::new("cmd.invol.report.title", "# 渐开线花键计算书\n\n", "# Involute spline computation report\n\n"),
    Msg::new("cmd.invol.report.std", "- 体系：**{v0}**（{v1}）\n", "- System: **{v0}** ({v1})\n"),
    Msg::new("cmd.invol.report.mode", "- 模式：**{v0}**\n", "- Mode: **{v0}**\n"),
    Msg::new("cmd.invol.report.mode_internal", "内花键（材料在外、齿朝内）", "internal spline (material outside, teeth pointing inward)"),
    Msg::new("cmd.invol.report.mode_external", "外花键", "external spline"),
    Msg::new("cmd.invol.report.profile", "- 齿廓/预设：**{v0}**（代号 `{v1}`）\n", "- Profile/preset: **{v0}** (code `{v1}`)\n"),
    Msg::new("cmd.invol.report.len", "- 有效长度 L：{v0} mm\n", "- Effective length L: {v0} mm\n"),
    Msg::new("cmd.invol.report.units", "- 单位口径：{v0}\n\n", "- Units: {v0}\n\n"),
    Msg::new("cmd.invol.report.h_inputs", "## 1. 输入参数\n\n", "## 1. Input parameters\n\n"),
    Msg::new("cmd.invol.report.t_inputs", "| 输入 | 原值 | 说明 |\n|---|---|---|\n", "| Input | Value | Notes |\n|---|---|---|\n"),
    Msg::new("cmd.invol.report.row_pitch", "| 径节 P / Ps | {v0} / {v1} | A/B 写法；P 单位 1/in，Ps 恒 = 2P |\n", "| Pitch P / Ps | {v0} / {v1} | A/B form; P in 1/in, Ps always = 2P |\n"),
    Msg::new("cmd.invol.report.row_m_conv", "| 模数 m = 25.4/P | {v0} mm | 引擎内部统一 mm |\n", "| Module m = 25.4/P | {v0} mm | engine works in mm internally |\n"),
    Msg::new("cmd.invol.report.row_m", "| 模数 m | {v0} mm | 模数制 |\n", "| Module m | {v0} mm | module system |\n"),
    Msg::new("cmd.invol.report.row_z", "| 齿数 z（ANSI 记 N） | {v0} | — |\n", "| Teeth z (N in ANSI) | {v0} | — |\n"),
    Msg::new("cmd.invol.report.row_x", "| 变位系数 x | {v0} | — |\n", "| Profile shift x | {v0} | — |\n"),
    Msg::new("cmd.invol.report.row_db_input", "| 基准直径 d_B | {v0} | 主参数（DIN 5480） |\n", "| Reference diameter d_B | {v0} | master parameter (DIN 5480) |\n"),
    Msg::new("cmd.invol.report.row_a", "| 公称直径 A | {v0} | 主参数（NF E22-141） |\n", "| Nominal diameter A | {v0} | master parameter (NF E22-141) |\n"),
    Msg::new("cmd.invol.report.row_profile", "| 基本齿廓 α / ha* / hf* / ρf* / cF* | {v0}° / {v1} / {v2} / {v3} / {v4} | 预设（可覆盖） |\n", "| Basic profile α / ha* / hf* / ρf* / cF* | {v0}° / {v1} / {v2} / {v3} / {v4} | preset (overridable) |\n"),
    Msg::new("cmd.invol.report.h_steps", "## 2. 逐步计算\n\n", "## 2. Step-by-step calculation\n\n"),
    Msg::new("cmd.invol.report.t_steps", "| # | 步骤 | 公式（符号含义） | 代入 | 结果 | 依据来源 |\n", "| # | Step | Formula (symbol meaning) | Substitution | Result | Source |\n"),
    Msg::new("cmd.invol.report.step.preset", "预设基本齿廓", "Preset basic profile"),
    Msg::new("cmd.invol.report.step.ansi_conv", "英制换算", "Inch conversion"),
    Msg::new("cmd.invol.report.formula.ansi_conv", "m = 25.4 / P（P 径节 1/in；m 模数 mm）", "m = 25.4 / P (P pitch 1/in; m module mm)"),
    Msg::new("cmd.invol.report.source.ansi_conv", "ANSI B92.1 英制标准（1 in = 25.4 mm）", "ANSI B92.1 inch standard (1 in = 25.4 mm)"),
    Msg::new("cmd.invol.report.step.din_db", "基准直径/变位", "Reference diameter / shift"),
    Msg::new("cmd.invol.report.formula.din_db", "d_B = d + 1.1m + 2x₁m；逆式 x₁ = (d_B − m(z+1.1)) / (2m)", "d_B = d + 1.1m + 2x₁m; inverse x₁ = (d_B − m(z+1.1)) / (2m)"),
    Msg::new("cmd.invol.report.source.din_db", "由 DIN 5480-2 名义表（OCR）反推并经全表逐行校验（721 行残差 0）；{v0}", "Reverse-derived from the DIN 5480-2 nominal table (OCR) and validated row by row across the full table (721 rows, residual 0); {v0}"),
    Msg::new("cmd.invol.report.step.nf_a", "公称直径/变位", "Nominal diameter / shift"),
    Msg::new("cmd.invol.report.formula.nf_a", "A = m(N + 2x + 0.4)；逆式 x = (A − m(N+0.4)) / (2m)", "A = m(N + 2x + 0.4); inverse x = (A − m(N+0.4)) / (2m)"),
    Msg::new("cmd.invol.report.source.nf_a", "NF E22-141 p07；{v0}", "NF E22-141 p07; {v0}"),
    Msg::new("cmd.invol.report.step.d", "分度圆直径", "Pitch diameter"),
    Msg::new("cmd.invol.report.formula.d", "d = m·z（m 模数，z 齿数）", "d = m·z (m module, z teeth)"),
    Msg::new("cmd.invol.report.source.d.gb", "GB/T 3478.1-2008 表 3（d = mz）", "GB/T 3478.1-2008 Table 3 (d = mz)"),
    Msg::new("cmd.invol.report.source.d.din", "DIN 5480-2 名义表 d 列 / DIN 5480-1:2015（d = mz）", "DIN 5480-2 nominal table column d / DIN 5480-1:2015 (d = mz)"),
    Msg::new("cmd.invol.report.source.d.nf", "NF E22-141 p07（d = mN）", "NF E22-141 p07 (d = mN)"),
    Msg::new("cmd.invol.report.source.d.ansi", "ANSI B92.1 Table 2（D = N/P，英寸 → m·z mm）", "ANSI B92.1 Table 2 (D = N/P, inches → m·z mm)"),
    Msg::new("cmd.invol.report.step.d_eff", "计算直径", "Working diameter"),
    Msg::new("cmd.invol.report.formula.d_eff", "d′ = d + 2x·m（计入变位）", "d′ = d + 2x·m (includes shift)"),
    Msg::new("cmd.invol.report.source.d_eff.din", "DIN 5480-1:2015 条 5.1（d′ = mz + 2xm）", "DIN 5480-1:2015 clause 5.1 (d′ = mz + 2xm)"),
    Msg::new("cmd.invol.report.source.d_eff.nf", "NF E22-141 p07（d′ = m(N+2x)）", "NF E22-141 p07 (d′ = m(N+2x))"),
    Msg::new("cmd.invol.report.source.d_eff.other", "通用变位口径（GB 基本齿廓不含变位，x=0 时 d′=d）", "general shifted-profile convention (the GB basic profile has no shift; d′=d when x=0)"),
    Msg::new("cmd.invol.report.step.db", "基圆直径", "Base diameter"),
    Msg::new("cmd.invol.report.formula.db", "db = d·cosα（α 压力角）", "db = d·cosα (α pressure angle)"),
    Msg::new("cmd.invol.report.source.db.gb", "GB/T 3478.1-2008 表 3（基圆 db）", "GB/T 3478.1-2008 Table 3 (base circle db)"),
    Msg::new("cmd.invol.report.source.db.din", "DIN 5480-1:2015 渐开线定义", "DIN 5480-1:2015 involute definition"),
    Msg::new("cmd.invol.report.source.db.nf", "NF E22-141 渐开线定义", "NF E22-141 involute definition"),
    Msg::new("cmd.invol.report.source.db.ansi", "ANSI B92.1 渐开线定义（α 由 Table 2 列定）", "ANSI B92.1 involute definition (α fixed by the Table 2 column)"),
    Msg::new("cmd.invol.report.step.da", "齿顶圆直径", "Tip diameter"),
    Msg::new("cmd.invol.report.formula.da.gb", "da = d′ + 2·ha*·m", "da = d′ + 2·ha*·m"),
    Msg::new("cmd.invol.report.formula.da.din", "d_a1 = d′ + 2·ha*·m（ha*=0.45 ⇒ d + 2xm + 0.9m）", "d_a1 = d′ + 2·ha*·m (ha*=0.45 ⇒ d + 2xm + 0.9m)"),
    Msg::new("cmd.invol.report.formula.da.nf", "da = A（外径定心 A₁′=A，等效 ha*=0.2）", "da = A (major-diameter centring A₁′=A, equivalent ha*=0.2)"),
    Msg::new("cmd.invol.report.formula.da.ansi", "Do = (N+1)/P ⇒ da = d + m（所选列 ha*=0.5）", "Do = (N+1)/P ⇒ da = d + m (selected column ha*=0.5)"),
    Msg::new("cmd.invol.report.source.da.gb", "GB/T 3478.1-2008 表 3 + 表 4~表 6（30° 平/圆 → m(z+1)；37.5° → m(z+0.9)；45° → m(z+0.8)）", "GB/T 3478.1-2008 Table 3 + Tables 4–6 (30° flat/fillet → m(z+1); 37.5° → m(z+0.9); 45° → m(z+0.8))"),
    Msg::new("cmd.invol.report.source.da.din", "DIN 5480-1:2015（d_a1 = mz + 2xm + 0.9m）", "DIN 5480-1:2015 (d_a1 = mz + 2xm + 0.9m)"),
    Msg::new("cmd.invol.report.source.da.nf", "NF E22-141 p07（外径定心）", "NF E22-141 p07 (major-diameter centring)"),
    Msg::new("cmd.invol.report.source.da.ansi", "ANSI B92.1 Table 2（Do 公式）", "ANSI B92.1 Table 2 (Do formula)"),
    Msg::new("cmd.invol.report.step.df", "齿根圆直径", "Root diameter"),
    Msg::new("cmd.invol.report.formula.df.gb", "df = d′ − 2·hf*·m", "df = d′ − 2·hf*·m"),
    Msg::new("cmd.invol.report.formula.df.din", "d_f1 = d′ − 2·hf*·m（hf*=0.55 ⇒ d′ − 1.1m）", "d_f1 = d′ − 2·hf*·m (hf*=0.55 ⇒ d′ − 1.1m)"),
    Msg::new("cmd.invol.report.formula.df.nf", "df = A − 2.4m（平齿根）/ A − 2.694m（圆齿根）", "df = A − 2.4m (flat root) / A − 2.694m (fillet root)"),
    Msg::new("cmd.invol.report.formula.df.ansi", "Dre = (N−k)/P；df = d − k·m", "Dre = (N−k)/P; df = d − k·m"),
    Msg::new("cmd.invol.report.source.df.gb", "GB/T 3478.1-2008 表 3 + 表 4~表 6（30°平 m(z−1.5)；30°圆 m(z−1.8)；37.5° m(z−1.4)；45° m(z−1.2)）", "GB/T 3478.1-2008 Table 3 + Tables 4–6 (30° flat m(z−1.5); 30° fillet m(z−1.8); 37.5° m(z−1.4); 45° m(z−1.2))"),
    Msg::new("cmd.invol.report.source.df.din", "DIN 5480-1:2015（d_f1；齿侧对中基准 h_fP*=0.55）", "DIN 5480-1:2015 (d_f1; flank-centring datum h_fP*=0.55)"),
    Msg::new("cmd.invol.report.source.df.nf", "NF E22-141 p07（B=A−2.4m / B₁=A−2.694m）", "NF E22-141 p07 (B=A−2.4m / B₁=A−2.694m)"),
    Msg::new("cmd.invol.report.source.df.ansi", "ANSI B92.1 Table 2（Dre；k 按所选列与径节分段）", "ANSI B92.1 Table 2 (Dre; k by the selected column and pitch segment)"),
    Msg::new("cmd.invol.report.step.dre", "Dre 径节分段", "Dre pitch segment"),
    Msg::new("cmd.invol.report.formula.dre", "按 Table 2 列与径节 P 取 Dre 的 k", "k for Dre from the Table 2 column and pitch P"),
    Msg::new("cmd.invol.report.source.dre", "ANSI B92.1 Table 2（Dre 分段：P>12 与 P≤12）", "ANSI B92.1 Table 2 (Dre segments: P>12 and P≤12)"),
    Msg::new("cmd.invol.report.step.s", "分度圆齿厚", "Pitch-circle tooth thickness"),
    Msg::new("cmd.invol.report.formula.s.gb_din", "s = mπ/2 + 2x·m·tanα", "s = mπ/2 + 2x·m·tanα"),
    Msg::new("cmd.invol.report.formula.s.ansi", "t = p − Sv min（p = πm 齿距；Sv min 由压力角列定）", "t = p − Sv min (p = πm circular pitch; Sv min fixed by the pressure-angle column)"),
    Msg::new("cmd.invol.report.source.s.gb", "GB/T 3478.1-2008 表 3（分度圆齿厚）", "GB/T 3478.1-2008 Table 3 (pitch-circle tooth thickness)"),
    Msg::new("cmd.invol.report.source.s.din", "DIN 5480-1:2015（s1 = mπ/2 + 2xm·tanα）", "DIN 5480-1:2015 (s1 = mπ/2 + 2xm·tanα)"),
    Msg::new("cmd.invol.report.source.s.ansi", "ANSI B92.1 Table 2（Sv min：30° π/(2P)；37.5° (0.5π+0.1)/P；45° (0.5π+0.2)/P）", "ANSI B92.1 Table 2 (Sv min: 30° π/(2P); 37.5° (0.5π+0.1)/P; 45° (0.5π+0.2)/P)"),
    Msg::new("cmd.invol.report.step.int_major", "内花键大径", "Internal spline major diameter"),
    Msg::new("cmd.invol.report.step.int_minor", "内花键小径", "Internal spline minor diameter"),
    Msg::new("cmd.invol.report.formula.int_major.gb", "D_ei = d′ + 2·hf*·m（外侧齿根）", "D_ei = d′ + 2·hf*·m (outer root)"),
    Msg::new("cmd.invol.report.source.int_major.gb", "GB/T 3478.1-2008 表 3（内花键大径 D_ei）", "GB/T 3478.1-2008 Table 3 (internal spline major diameter D_ei)"),
    Msg::new("cmd.invol.report.formula.int_minor.gb", "D_ii = D_Fe max + 2cF；D_Fe max = 2√((db/2)² + (d/2·sinα − h_s/sinα)²)", "D_ii = D_Fe max + 2cF; D_Fe max = 2√((db/2)² + (d/2·sinα − h_s/sinα)²)"),
    Msg::new("cmd.invol.report.source.int_minor.gb", "GB/T 3478.1-2008 表 3（D_Fe max，es_v=0，H/h 配合；h_s 见图 2）", "GB/T 3478.1-2008 Table 3 (D_Fe max, es_v=0, H/h fit; h_s in Figure 2)"),
    Msg::new("cmd.invol.report.step.int_root", "内花键齿根", "Internal spline root"),
    Msg::new("cmd.invol.report.step.int_tip", "内花键齿顶", "Internal spline tip"),
    Msg::new("cmd.invol.report.formula.int_root.din", "d_f2 = d_B（名义表恒等式）", "d_f2 = d_B (nominal-table identity)"),
    Msg::new("cmd.invol.report.source.int_root.din", "DIN 5480-2 名义表恒等式（d_f2 = d_B）", "DIN 5480-2 nominal-table identity (d_f2 = d_B)"),
    Msg::new("cmd.invol.report.formula.int_tip.din", "d_a2 = d′ − 0.9m", "d_a2 = d′ − 0.9m"),
    Msg::new("cmd.invol.report.source.int_tip.din", "DIN 5480-1:2015（d_a2 = d − 0.9m + 2xm）", "DIN 5480-1:2015 (d_a2 = d − 0.9m + 2xm)"),
    Msg::new("cmd.invol.report.formula.int_major.nf", "D_ei = A（外径定心）", "D_ei = A (major-diameter centring)"),
    Msg::new("cmd.invol.report.formula.int_minor.nf", "D_ii = A − 2m", "D_ii = A − 2m"),
    Msg::new("cmd.invol.report.source.int_minor.nf", "NF E22-141 p07（D = A − 2m）", "NF E22-141 p07 (D = A − 2m)"),
    Msg::new("cmd.invol.report.formula.int_major.ansi", "Dri = (N+{v0})/P", "Dri = (N+{v0})/P"),
    Msg::new("cmd.invol.report.formula.int_minor.ansi", "Di = (N−{v0})/P", "Di = (N−{v0})/P"),
    Msg::new("cmd.invol.report.source.int_major.ansi", "ANSI B92.1 Table 2（Dri 公式）", "ANSI B92.1 Table 2 (Dri formula)"),
    Msg::new("cmd.invol.report.source.int_minor.ansi", "ANSI B92.1 Table 2（Di 公式）", "ANSI B92.1 Table 2 (Di formula)"),
    Msg::new("cmd.invol.report.step.cf", "齿形裕度", "Form clearance"),
    Msg::new("cmd.invol.report.formula.cf", "cF = clamp(0.001·D_mm, 0.0508, 0.254)（原式 clamp(0.001·D_in, 0.002in, 0.010in)）", "cF = clamp(0.001·D_mm, 0.0508, 0.254) (original clamp(0.001·D_in, 0.002in, 0.010in))"),
    Msg::new("cmd.invol.report.source.cf", "ANSI B92.1 Table 2（cF 夹取）", "ANSI B92.1 Table 2 (cF clamp)"),
    Msg::new("cmd.invol.report.step.dfe", "外花键 form 直径", "External spline form diameter"),
    Msg::new("cmd.invol.report.formula.dfe", "DFe = d − k·m − 2cF（k 按列：默认 1，37.5° 0.8，45° 0.6）", "DFe = d − k·m − 2cF (k by column: default 1, 37.5° 0.8, 45° 0.6)"),
    Msg::new("cmd.invol.report.source.dfe", "ANSI B92.1 Table 2（DFe）", "ANSI B92.1 Table 2 (DFe)"),
    Msg::new("cmd.invol.report.step.dfi", "内花键 form 直径", "Internal spline form diameter"),
    Msg::new("cmd.invol.report.formula.dfi", "DFi = d + k·m + 2cF（列 B 另含英寸常量 −0.004 in = −0.1016 mm）", "DFi = d + k·m + 2cF (column B also has the inch constant −0.004 in = −0.1016 mm)"),
    Msg::new("cmd.invol.report.source.dfi", "ANSI B92.1 Table 2（DFi）", "ANSI B92.1 Table 2 (DFi)"),
    Msg::new("cmd.invol.report.h_derived", "## 3. 派生几何\n\n| 量 | 值 |\n|---|---|\n", "## 3. Derived geometry\n\n| Quantity | Value |\n|---|---|\n"),
    Msg::new("cmd.invol.report.row_d", "| 分度圆 d | {v0} mm |\n", "| Pitch d | {v0} mm |\n"),
    Msg::new("cmd.invol.report.row_db", "| 基圆 db | {v0} mm |\n", "| Base db | {v0} mm |\n"),
    Msg::new("cmd.invol.report.row_int_major", "| 内花键大径 D_ei（外侧齿根） | {v0} mm |\n", "| Internal spline major D_ei (outer root) | {v0} mm |\n"),
    Msg::new("cmd.invol.report.row_int_minor", "| 内花键小径 D_ii（里侧齿顶） | {v0} mm |\n", "| Internal spline minor D_ii (inner tip) | {v0} mm |\n"),
    Msg::new("cmd.invol.report.row_da", "| 齿顶圆 da | {v0} mm |\n", "| Tip da | {v0} mm |\n"),
    Msg::new("cmd.invol.report.row_df", "| 齿根圆 df | {v0} mm |\n", "| Root df | {v0} mm |\n"),
    Msg::new("cmd.invol.report.row_s", "| 分度圆齿厚 s | {v0} mm |\n", "| Pitch-circle thickness s | {v0} mm |\n"),
    Msg::new("cmd.invol.report.row_rho", "| 齿根圆角 ρf | {v0} mm |\n", "| Root fillet ρf | {v0} mm |\n"),
    Msg::new("cmd.invol.report.row_cf", "| 齿形裕度 cF | {v0} mm |\n", "| Form clearance cF | {v0} mm |\n"),
    Msg::new("cmd.invol.report.row_clearance", "| 顶隙 c | {v0} mm |\n", "| Tip clearance c | {v0} mm |\n"),
    Msg::new("cmd.invol.report.note_radial_fallback", "（已到/超过外侧齿根：材料带内无渐开线，端视图按径向直线降级）", "(at/past the outer root: no involute within the material band; the end view falls back to radial lines)"),
    Msg::new("cmd.invol.report.row_inv_start_int", "| 渐开线有效起始圆（内花键 max(D_ii, db, DFi)） | {v0} mm{v1} |\n", "| Effective involute start circle (internal: max(D_ii, db, DFi)) | {v0} mm{v1} |\n"),
    Msg::new("cmd.invol.report.row_inv_end_int", "| 渐开线终止（外侧齿根 D_ei） | {v0} mm |\n", "| Involute end (outer root D_ei) | {v0} mm |\n"),
    Msg::new("cmd.invol.report.row_inv_start", "| 渐开线起始圆 d_involute_start | {v0} mm |\n", "| Involute start circle d_involute_start | {v0} mm |\n"),
    Msg::new("cmd.invol.report.row_inv_end", "| 渐开线终止圆 d_involute_end（=da） | {v0} mm |\n", "| Involute end circle d_involute_end (=da) | {v0} mm |\n"),
    Msg::new("cmd.invol.report.row_db_val", "| 基准直径 d_B | {v0} mm |\n", "| Reference diameter d_B | {v0} mm |\n"),
    Msg::new("cmd.invol.report.row_a_val", "| 公称直径 A | {v0} mm |\n", "| Nominal diameter A | {v0} mm |\n"),
    Msg::new("cmd.invol.report.h_checks", "## 4. 检验尺寸\n\n", "## 4. Inspection dimensions\n\n"),
    Msg::new("cmd.invol.report.nf_no_table", "NF E22-141 检查表没有该档（A={v0}、m={v1}、N={v2}）；已入库检查表覆盖 p23–p27/p29/p31–p33/p35（`assets/nf_e22141_check.csv`，399 行）。\n\n", "The NF E22-141 check table has no entry for this case (A={v0}, m={v1}, N={v2}); the loaded check tables cover p23–p27/p29/p31–p33/p35 (`assets/nf_e22141_check.csv`, 399 rows).\n\n"),
    Msg::new("cmd.invol.report.nf_n_miss", "NF E22-141 检查表：A={v0}、m={v1} 有行但 N={v2} 未命中；同档 N 变体：{v3}。\n\n", "NF E22-141 check table: rows exist for A={v0}, m={v1} but N={v2} is not hit; N variants in the same case: {v3}.\n\n"),
    Msg::new("cmd.invol.report.nf_hit", "NF E22-141 检查表命中 {v0} 行（`assets/nf_e22141_check.csv`，共 {v1} 行/10 张表；页清单 `NF-E22-141_检查公差_页清单.md`）。\n\n", "NF E22-141 check table hit {v0} rows (`assets/nf_e22141_check.csv`, {v1} rows / 10 tables; page list `NF-E22-141_检查公差_页清单.md`).\n\n"),
    Msg::new("cmd.invol.report.h_check_row", "### p{v0} {v1}（source={v2}；{v3}）\n\n", "### p{v0} {v1} (source={v2}; {v3})\n\n"),
    Msg::new("cmd.invol.report.t_qty", "| 量 | 值 |\n|---|---|\n", "| Quantity | Value |\n|---|---|\n"),
    Msg::new("cmd.invol.report.k_span", "跨齿数 K", "Span teeth K"),
    Msg::new("cmd.invol.report.e_kt", "K 齿公法线 E", "Base tangent E for K teeth"),
    Msg::new("cmd.invol.report.u_ext", "外花键量棒直径 U", "External spline ball diameter U"),
    Msg::new("cmd.invol.report.f_ext", "外花键跨量棒距 F", "External spline over-balls F"),
    Msg::new("cmd.invol.report.f1_ext", "外花键跨量棒距 F1", "External spline over-balls F1"),
    Msg::new("cmd.invol.report.v_int", "内花键量棒直径 V", "Internal spline ball diameter V"),
    Msg::new("cmd.invol.report.v1_int", "内花键量棒削边 V1", "Internal spline ball flat V1"),
    Msg::new("cmd.invol.report.g_int", "内花键量棒跨距 G", "Internal spline between-balls G"),
    Msg::new("cmd.invol.report.g1_int", "内花键量棒跨距 G1", "Internal spline between-balls G1"),
    Msg::new("cmd.invol.report.q_unknown", "q{v0}（表头未辨认）", "q{v0} (header not recognized)"),
    Msg::new("cmd.invol.report.h_insp_deriv", "### 检验量公式推导（K / W）\n\n", "### Inspection-value formula derivation (K / W)\n\n"),
    Msg::new("cmd.invol.report.nf_conv", "> 口径：**K**（跨测齿数）取 NF E22-141 p23–p25 检查表表值（标准按 N 分档给出，本仓未收该分档的独立公式 —— 不臆造）；**W**（公法线）= 检查表 `E` 列，公式见 `assets/nf_e22141_check.csv` 头注（对入库行复算命中）。\n\n", "> Convention: **K** (span teeth) is taken from the NF E22-141 p23–p25 check table (the standard gives it per N step; this repo does not hold an independent formula for that step — no guessing); **W** (base tangent) = the check table `E` column, formula in the `assets/nf_e22141_check.csv` header note (re-computed and matched for the loaded rows).\n\n"),
    Msg::new("cmd.invol.report.k_formula", "K = 检查表 K 列（标准按 N 分档给出；表外不外推）", "K = check-table column K (given per N step; no extrapolation off-table)"),
    Msg::new("cmd.invol.report.k_subst", "A={v0}、m={v1}、N={v2} → p{v3} 检查表行", "A={v0}, m={v1}, N={v2} → p{v3} check-table row"),
    Msg::new("cmd.invol.report.k_source", "NF E22-141 p23–p25（assets/nf_e22141_check.csv）", "NF E22-141 p23–p25 (assets/nf_e22141_check.csv)"),
    Msg::new("cmd.invol.report.k_no_hit_row", "| 1 | 跨测齿数 K | K = 检查表 K 列 | A/m/N 未命中 p23–p25 | — | 表外不外推 |\n", "| 1 | Span teeth K | K = check-table column K | A/m/N not hit in p23–p25 | — | no extrapolation off-table |\n"),
    Msg::new("cmd.invol.report.w_step", "公法线 W（= E）", "Base tangent W (= E)"),
    Msg::new("cmd.invol.report.w_formula", "W = m·cos20°·[(K−0.5)·π + N·inv20°] + 2·x·m·sin20°", "W = m·cos20°·[(K−0.5)·π + N·inv20°] + 2·x·m·sin20°"),
    Msg::new("cmd.invol.report.w_subst", "m={v0}、K={v1}、N={v2}、x={v3}", "m={v0}, K={v1}, N={v2}, x={v3}"),
    Msg::new("cmd.invol.report.w_source", "公式：assets/nf_e22141_check.csv 头注；表值：同表 E 列", "Formula: assets/nf_e22141_check.csv header note; table values: column E of the same table"),
    Msg::new("cmd.invol.report.same_source_kw", "- 同源：K/W 与卡片同取 [`nf_check_by_a_m`]（A={v0}、m={v1}）的同一表行；卡片第 12/13 行（跨测齿数 K / 公法线 W）与本表逐值一致。\n", "- Same source: K/W use the same table row from [`nf_check_by_a_m`] as the card (A={v0}, m={v1}); card rows 12/13 (span teeth K / base tangent W) match this table value by value.\n"),
    Msg::new("cmd.invol.report.w_no_hit_row", "| 1 | 公法线 W（= E） | W = m·cos20°·[(K−0.5)·π + N·inv20°] + 2·x·m·sin20° | K 未命中 | — | 表外不外推（assets/nf_e22141_check.csv） |\n", "| 1 | Base tangent W (= E) | W = m·cos20°·[(K−0.5)·π + N·inv20°] + 2·x·m·sin20° | K not hit | — | no extrapolation off-table (assets/nf_e22141_check.csv) |\n"),
    Msg::new("cmd.invol.report.d_pitch", "分度圆 d", "Pitch d"),
    Msg::new("cmd.invol.report.db_base", "基圆 dB", "Base dB"),
    Msg::new("cmd.invol.report.x_shift", "变位系数 x", "Profile shift x"),
    Msg::new("cmd.invol.report.s_arc", "分度圆弧齿厚 s", "Pitch-circle arc thickness s"),
    Msg::new("cmd.invol.report.sb_arc", "基圆弧齿厚 sB", "Base-circle arc thickness sB"),
    Msg::new("cmd.invol.report.b_flat", "齿根圆 B（平齿根）", "Root B (flat root)"),
    Msg::new("cmd.invol.report.b2_fillet", "齿根圆 B2（圆齿根）", "Root B2 (fillet root)"),
    Msg::new("cmd.invol.report.rf_fillet", "齿根圆角 Rf", "Root fillet Rf"),
    Msg::new("cmd.invol.report.rr_fillet", "齿根圆角 Rr", "Root fillet Rr"),
    Msg::new("cmd.invol.report.h_chamfer", "齿顶倒角高度 h", "Tip chamfer height h"),
    Msg::new("cmd.invol.report.ri_slot", "槽底圆角 ri", "Slot-bottom fillet ri"),
    Msg::new("cmd.invol.report.dev_line", "- 偏差（微米，上/下）：{devs}；{v0}。\n", "- Deviations (µm, upper/lower): {devs}; {v0}.\n"),
    Msg::new("cmd.invol.report.h_ansi_deriv", "### 检验量公式推导（Wn / Kn；渐开线几何）\n\n", "### Inspection-value formula derivation (Wn / Kn; involute geometry)\n\n"),
    Msg::new("cmd.invol.report.ansi_note", "> 说明：ANSI B92.1 原标准检验表（公法线/跨测表）本仓未收 —— 下表是**渐开线几何推导**\n> （与 GB/T 3478.6 式(11)、NF E22-141 同一条跨距公式），不是原标准表值；\n> 卡片里的「公法线长度 / 跨测齿数」格仍显示「—」（不冒充原表；极限/平均长度需原标准表）。\n\n", "> Note: the original ANSI B92.1 inspection tables (base-tangent/span tables) are not held in this repo — the table below is an **involute-geometry derivation**\n> (the same span formula as GB/T 3478.6 eq. (11) and NF E22-141), not original standard table values;\n> the card cells for “base tangent length / span teeth” still show “—” (no pretending to be the original table; limit/average lengths need it).\n\n"),
    Msg::new("cmd.invol.report.kn_step", "跨测齿数 Kn", "Span teeth Kn"),
    Msg::new("cmd.invol.report.kn_formula", "Kn = round(N·α/180° + 0.5)（α 单位度）", "Kn = round(N·α/180° + 0.5) (α in degrees)"),
    Msg::new("cmd.invol.report.kn_subst", "round({v0}×{v1}/180 + 0.5)", "round({v0}×{v1}/180 + 0.5)"),
    Msg::new("cmd.invol.report.kn_source", "渐开线跨距公式（与 GB/T 3478.6 式(11) 同式）；ANSI 原跨测表本仓未收", "involute span formula (same as GB/T 3478.6 eq. (11)); the original ANSI span table is not held here"),
    Msg::new("cmd.invol.report.wn_step", "公法线长度 Wn", "Base tangent length Wn"),
    Msg::new("cmd.invol.report.wn_formula", "Wn = m·cosα·[(Kn−0.5)·π + N·invα] + 2·x·m·sinα（invα = tanα − α）", "Wn = m·cosα·[(Kn−0.5)·π + N·invα] + 2·x·m·sinα (invα = tanα − α)"),
    Msg::new("cmd.invol.report.wn_subst", "m={v0}、α={v1}°、Kn={v2}、N={v3}、x={v4}", "m={v0}, α={v1}°, Kn={v2}, N={v3}, x={v4}"),
    Msg::new("cmd.invol.report.wn_source", "渐开线几何推导；ANSI B92.1 原公法线表本仓未收", "involute-geometry derivation; the original ANSI base-tangent table is not held here"),
    Msg::new("cmd.invol.report.wn_same_formula", "- 与其它体系同式：GB/T 3478.6 式(11)（多 `esv − (T+λ)` 修正项）、NF E22-141 检查表（α=20°）；本式是同一渐开线跨距的 α 通用形式。\n", "- Same formula as other systems: GB/T 3478.6 eq. (11) (with the extra `esv − (T+λ)` correction) and the NF E22-141 check table (α=20°); this is the general-α form of the same involute span.\n"),
    Msg::new("cmd.invol.report.no_table", "本体系无入库检验尺寸表（DIN 5480-2 检验表覆盖 DIN 预设；NF E22-141 检查表覆盖 NF；ANSI 只给渐开线几何推导）。\n\n", "This system has no stored inspection-dimension table (the DIN 5480-2 inspection table covers DIN presets; the NF E22-141 check table covers NF; ANSI only gives an involute-geometry derivation).\n\n"),
    Msg::new("cmd.invol.report.m1_row", "| 跨棒距 M1 | {v0} mm | 外花键；量棒 D_M={v1} mm |\n", "| Over-balls M1 | {v0} mm | external spline; ball D_M={v1} mm |\n"),
    Msg::new("cmd.invol.report.m2_row", "| 棒间距 M2 | {v0} mm | 内花键；量棒 D_M={v1} mm |\n", "| Between-balls M2 | {v0} mm | internal spline; ball D_M={v1} mm |\n"),
    Msg::new("cmd.invol.report.k_row", "| 跨测齿数 k | {v0} | — |\n", "| Span teeth k | {v0} | — |\n"),
    Msg::new("cmd.invol.report.wk_row", "| 公法线 W_k | {v0} mm | — |\n", "| Base tangent W_k | {v0} mm | — |\n"),
    Msg::new("cmd.invol.report.tbl_hit_row", "| 查表命中 | p{v0} 表{v1} | source={v2} |\n", "| Table hit | p{v0} table {v1} | source={v2} |\n"),
    Msg::new("cmd.invol.report.src_line", "- 来源：{v0}{v1}\n", "- Source: {v0}{v1}\n"),
    Msg::new("cmd.invol.report.src_exact", "（精确查表）", " (exact table hit)"),
    Msg::new("cmd.invol.report.src_formula", "（公式导出）", " (formula-derived)"),
    Msg::new("cmd.invol.report.note_line", "- 注：{note}\n", "- Note: {note}\n"),
    Msg::new("cmd.invol.report.no_check_dims", "无法给出检验尺寸：{err}\n\n", "Cannot provide inspection dimensions: {err}\n\n"),
    Msg::new("cmd.invol.report.h_sources", "## 5. 数据来源与校验\n\n", "## 5. Data sources and checks\n\n"),
    Msg::new("cmd.invol.report.src_preset", "- 预设来源：{v0}（`{v1}`）\n", "- Preset source: {v0} (`{v1}`)\n"),
    Msg::new("cmd.invol.report.src_module", "- 候选/查表来源：{v0}\n", "- Candidate/lookup source: {v0}\n"),
    Msg::new("cmd.invol.report.src_datum", "- 基准直径来源：{v0}\n", "- Reference-diameter source: {v0}\n"),
    Msg::new("cmd.invol.report.src_datum_na", "不适用（GB/ANSI 无 d_B/A 主参数）", "not applicable (GB/ANSI have no d_B/A master parameter)"),
    Msg::new("cmd.invol.report.src_full_check", "- 检验表全表对照：{v0}\n", "- Inspection table full comparison: {v0}\n"),
    Msg::new("cmd.invol.report.same_engine", "- 计算书内容与 JSON 输出同源：所有数值由 OCSM 计算引擎（`invol_spline`）导出，未二次手算。\n", "- Report content shares its source with the JSON output: all values come from the OCSM engine (`invol_spline`), not hand-recomputed.\n"),
    Msg::new("cmd.invol.src.module.gb", "GB/T 3478.1-2008 表 2（15 种：第 1 系列 0.25/0.5/1/1.5/2/2.5/3/5/10 + 第 2 系列 0.75/1.25/1.75/4/6/8）", "GB/T 3478.1-2008 Table 2 (15 values: series 1 0.25/0.5/1/1.5/2/2.5/3/5/10 + series 2 0.75/1.25/1.75/4/6/8)"),
    Msg::new("cmd.invol.src.module.din", "DIN 5480-2 名义表（assets/din5480_2_nominal.csv 实际 m 列）", "DIN 5480-2 nominal table (actual m column of assets/din5480_2_nominal.csv)"),
    Msg::new("cmd.invol.src.module.nf", "NF E22-141 尺寸表（assets/nf_e22141_dims.csv 实际 m 列）", "NF E22-141 dimension table (actual m column of assets/nf_e22141_dims.csv)"),
    Msg::new("cmd.invol.src.module.ansi", "ANSI B92.1 径节 17 项（assets/ansi_b921_formulas.csv，P/Ps=A/B）", "ANSI B92.1 pitch series 17 items (assets/ansi_b921_formulas.csv, P/Ps=A/B)"),
    // ⑥ 引擎目录说明串（`partgen::catalog_json` 顶层 `spline_engine` payload；GUI/API 元数据）。
    // 协议/数据记号（T 代号、行数、文件名、公式符号、页码）原样；只翻句子。
    Msg::new("cmd.invol.engine.source",
        "{v0}（图 2 基本齿廓 + 表 3~表 6）；{v1}（条 5.1：齿侧对中 h_fP=0.55m）；NF E22-141（中文译本 p18/p20/p21/p22，288 行：α=20°、A=m(N+2x+0.4)、D=A−2m）；ANSI B92.1-1970 (R1993)（公式驱动：径节 17 项 + Table 2 五列；D=N/P、Do=(N+1)/P、cF=0.001D 夹取）；DIN 5480-2 名义表（721 行；m=1.5/m=5 均由用户截图补入）+ 检验表（267 行；M₁/M₂/D_M/k/W_k）",
        "{v0} (Figure 2 basic profile + Tables 3–6); {v1} (clause 5.1: flank centring h_fP=0.55m); NF E22-141 (Chinese translation p18/p20/p21/p22, 288 rows: α=20°, A=m(N+2x+0.4), D=A−2m); ANSI B92.1-1970 (R1993) (formula-driven: 17 pitch items + Table 2 five columns; D=N/P, Do=(N+1)/P, cF=0.001D clamped); DIN 5480-2 nominal table (721 rows; m=1.5/m=5 both added from user screenshots) + inspection table (267 rows; M₁/M₂/D_M/k/W_k)"),
    Msg::new("cmd.invol.engine.din_notes",
        "DIN 5480-2 名义表：721 行；m=1.5（56 行）与 m=5（47 行）均由用户截图补入；x=(d_B−m(z+1.1))/(2m) 为反推关系（m=5 的 2 处 z 在解析期按代码修正表生效）。检验表：267 行（p12/16/18/20 + m=1.5/m=5 截图），6 档 0.5/0.75/0.8/1/1.5/5（m=5 的 17 处 k 同走代码修正表）；查表外 z 走公式（267 行逐行对照验证）",
        "DIN 5480-2 nominal table: 721 rows; m=1.5 (56 rows) and m=5 (47 rows) both added from user screenshots; x=(d_B−m(z+1.1))/(2m) is the inverse relation (the 2 z values at m=5 take effect through the code correction table at parse time). Inspection table: 267 rows (p12/16/18/20 + m=1.5/m=5 screenshots), 6 steps 0.5/0.75/0.8/1/1.5/5 (the 17 k values at m=5 also use the code correction table); z values outside the table use the formula (validated row by row against all 267 rows)"),
    Msg::new("cmd.invol.engine.nf_notes",
        "NF E22-141：288 行（p18 拉削内花键 144 + p20 39 + p21 49 + p22 56）；A=m(N+2x+0.4)、D=A−2m、db=d·cos20°；17 处 OCR 错格走代码修正表（不改 CSV），9 行疑原表印误只标注；m=0.75/3.75/7.50 的次系列 x=0.633/0.967 交替是真实设计值",
        "NF E22-141: 288 rows (p18 broached internal splines 144 + p20 39 + p21 49 + p22 56); A=m(N+2x+0.4), D=A−2m, db=d·cos20°; the 17 OCR-misaligned cells use the code correction table (CSV not rewritten), 9 rows suspected of printing errors are only annotated; the alternating x=0.633/0.967 of the secondary series m=0.75/3.75/7.50 are real design values"),
    Msg::new("cmd.invol.engine.ansi_notes",
        "ANSI B92.1-1970 (R1993) 公式驱动：径节 17 项 + Table 2 五列（30°平/齿侧、30°平/外径、30°圆/齿侧、37.5°圆/齿侧、45°圆/齿侧）；D=N/P、Db=D·cosφD、p=π/P、Do=(N+1)/P、Dre 三段、Sv min 随 φD、cF=0.001D 夹取；rf 标准未给值（p14），引擎按切于齿根的过渡弧构造（无标准数值依据）；**标准为英制，引擎内部/输出统一 mm（m=25.4/P，cF 夹取 0.0508~0.254 mm）**；抽样 73 行复算见 assets/ansi_b921_notes.md",
        "ANSI B92.1-1970 (R1993) formula-driven: 17 pitch items + Table 2 five columns (30° flat/flank, 30° flat/outside diameter, 30° round/flank, 37.5° round/flank, 45° round/flank); D=N/P, Db=D·cosφD, p=π/P, Do=(N+1)/P, Dre in three segments, Sv min follows φD, cF=0.001D clamped; rf is not given by the standard (p14), so the engine builds a transition arc tangent to the root (no standard numerical basis); **the standard is inch-based while the engine works and outputs in mm (m=25.4/P, cF clamped to 0.0508~0.254 mm)**; a 73-row sample recheck is in assets/ansi_b921_notes.md"),
    Msg::new("cmd.invol.report.units_note.ansi", "ANSI B92.1 为英制标准：径节 P 单位 1/in、Ps=2P；引擎内部按 1 in = 25.4 mm 换算，输出全为 mm。", "ANSI B92.1 is an inch standard: pitch P in 1/in, Ps=2P; the engine converts via 1 in = 25.4 mm and outputs all values in mm."),
    Msg::new("cmd.invol.report.units_note.din", "长度 mm、角度 °；d_B 为 DIN 5480 基准直径（由名义表/公式确定几何）。", "Length mm, angle °; d_B is the DIN 5480 reference diameter (geometry determined from the nominal table/formulas)."),
    Msg::new("cmd.invol.report.units_note.nf", "长度 mm、角度 °；A 为 NF E22-141 公称直径主参数。", "Length mm, angle °; A is the NF E22-141 nominal-diameter master parameter."),
    Msg::new("cmd.invol.report.units_note.gb", "长度 mm、角度 °；GB/T 3478.1 不用基准直径，几何由 m/z/x 决定。", "Length mm, angle °; GB/T 3478.1 has no reference diameter — geometry follows m/z/x."),
    Msg::new("cmd.invol.report.src.preset.gb", "GB/T 3478.1-2008 表 2（模数系列）+ 图 2（基本齿廓）+ 表 3（尺寸公式）", "GB/T 3478.1-2008 Table 2 (module series) + Figure 2 (basic profile) + Table 3 (dimension formulas)"),
    Msg::new("cmd.invol.report.src.preset.din", "DIN 5480-1:2015 条 5.1（齿侧对中基准 h_fP*=0.55）+ DIN 5480-2 名义表/检验表", "DIN 5480-1:2015 clause 5.1 (flank-centring datum h_fP*=0.55) + DIN 5480-2 nominal/inspection tables"),
    Msg::new("cmd.invol.report.src.preset.nf", "NF E22-141 p07 公式（α=20°；外径定心 A₁′=A）", "NF E22-141 p07 formulas (α=20°; major-diameter centring A₁′=A)"),
    Msg::new("cmd.invol.report.src.preset.ansi", "ANSI B92.1 Table 2 五列公式（p10；英制换 mm：1in=25.4mm）", "ANSI B92.1 Table 2 five-column formulas (p10; inch→mm: 1in=25.4mm)"),
    Msg::new("cmd.invol.din.origin_table", "查表命中 p{v0} m={v1}", "Table hit p{v0} m={v1}"),
    Msg::new("cmd.invol.din.origin_formula", "由公式解出，未命中表", "solved by formula, no table hit"),
    Msg::new("cmd.invol.din.origin_computed", "由 m/z/x 正算 d_B=m(z+1.1+2x)", "computed from m/z/x: d_B=m(z+1.1+2x)"),
    Msg::new("cmd.invol.nf.origin_table", "查表命中 p{v0} m={v1} A={v2}", "Table hit p{v0} m={v1} A={v2}"),
    Msg::new("cmd.invol.nf.origin_computed_a", "由 m/z/x 正算 A=m(z+2x+0.4)", "computed from m/z/x: A=m(z+2x+0.4)"),
    Msg::new("cmd.invol.report.dre.a", "30° 平齿根（列 A/B）：k=1.35，不分径节段", "30° flat root (columns A/B): k=1.35, no pitch subdivision"),
    Msg::new("cmd.invol.report.dre.c_gt12", "30° 圆齿根（列 C）：P>12 段，Dre=(N−2)/P", "30° fillet root (column C): P>12 segment, Dre=(N−2)/P"),
    Msg::new("cmd.invol.report.dre.c_le12", "30° 圆齿根（列 C）：P≤12 段，Dre=(N−1.8)/P", "30° fillet root (column C): P≤12 segment, Dre=(N−1.8)/P"),
    Msg::new("cmd.invol.report.dre.d", "37.5° 圆齿根（列 D）：k=1.3", "37.5° fillet root (column D): k=1.3"),
    Msg::new("cmd.invol.report.dre.e", "45° 圆齿根（列 E）：k=1.0（径节 10/20 起）", "45° fillet root (column E): k=1.0 (from pitch 10/20)"),
    Msg::new("cmd.invol.report.dre.none", "（无 Table 2 列）", "(no Table 2 column)"),
    Msg::new("cmd.invol.check.src_row", "p{v0} 表{v1}（source={v2}）", "p{v0} table{v1} (source={v2})"),
    Msg::new("cmd.invol.check.src_nf_row", "p{v0} 表{v1}（source={v2}；{v3}）", "p{v0} table{v1} (source={v2}; {v3})"),
    Msg::new("cmd.invol.check.db_pos", "DIN 5480-2 检验表：d_B={v0} 必须是正数。", "DIN 5480-2 inspection table: d_B={v0} must be positive."),
    Msg::new("cmd.invol.check.m_pos", "DIN 5480-2 检验表：m={v0} 必须是正数。", "DIN 5480-2 inspection table: m={v0} must be positive."),
    Msg::new("cmd.invol.check.head_db", "DIN 5480-2 检验表：d_B={v0}", "DIN 5480-2 inspection table: d_B={v0}"),
    Msg::new("cmd.invol.check.no_m_step", "；检验表没有 m={v0} 档（已入库检验档位：{v1}；检验表仅覆盖 0.5/0.75/0.8/1/1.5/5，其余档名义表有但检验表未并入，需另找表页）。", "; the inspection table has no m={v0} step (loaded steps: {v1}; the inspection table covers only 0.5/0.75/0.8/1/1.5/5 — other steps exist in the nominal table but were not merged into the inspection table, so their table pages must be found elsewhere)."),
    Msg::new("cmd.invol.check.cand_row", "d_B={v0}（m={v1}，{v2}）", "d_B={v0} (m={v1}, {v2})"),
    Msg::new("cmd.invol.check.z_variants", "DIN 5480-2 检验表：d_B={v0}、m={v1} 命中 {v2} 行但没有 z={v3}；同组的 z 变体：{v4}。", "DIN 5480-2 inspection table: d_B={v0}, m={v1} matched {v2} rows but no z={v3}; z variants in the same group: {v4}."),
    Msg::new("cmd.invol.check.nominal_no_row", "DIN 5480-2 检验表：名义表里没有 d_B={v0}、z={v1} 的行，无法反查 m；请直接给 m（数据来源：DIN 5480-2 名义表 721 行 + 检验表 267 行）。", "DIN 5480-2 inspection table: the nominal table has no row for d_B={v0}, z={v1}, so m cannot be looked up; give m directly (data source: DIN 5480-2 nominal table 721 rows + inspection table 267 rows)."),
    Msg::new("cmd.invol.check.nominal_multi_m", "DIN 5480-2 检验表：d_B={v0}、z={v1} 在名义表有多个 m 变体：{v2}；请直接给 m。", "DIN 5480-2 inspection table: d_B={v0}, z={v1} has several m variants in the nominal table: {v2}; give m directly."),
    Msg::new("cmd.invol.check.m_pos_k", "DIN 5480-2 检验：m={v0} 必须是正数（跨测齿数 k）。", "DIN 5480-2 inspection: m={v0} must be positive (span teeth k)."),
    Msg::new("cmd.invol.check.z_range_insp", "DIN 5480-2 检验：齿数 z={v0} 超出范围（3..=1000）。", "DIN 5480-2 inspection: tooth count z={v0} is out of range (3..=1000)."),
    Msg::new("cmd.invol.check.x_finite", "DIN 5480-2 检验：变位系数 x={v0} 必须是有限数。", "DIN 5480-2 inspection: profile shift x={v0} must be finite."),
    Msg::new("cmd.invol.check.no_span_k", "DIN 5480-2 检验：m={v0}、z={v1}、x={v2} 没有可用的跨测齿数 k（齿数太小，接触圆超出齿顶圆）。", "DIN 5480-2 inspection: m={v0}, z={v1}, x={v2} has no usable span-teeth count k (too few teeth; the contact circle exceeds the tip circle)."),
    Msg::new("cmd.invol.check.m_pos_span", "DIN 5480-2 检验：m={v0} 必须是正数（棒间距/跨棒距）。", "DIN 5480-2 inspection: m={v0} must be positive (between-balls/over-balls)."),
    Msg::new("cmd.invol.check.dm_pos", "DIN 5480-2 检验：量棒直径 D_M={v0} 必须是正数。", "DIN 5480-2 inspection: ball diameter D_M={v0} must be positive."),
    Msg::new("cmd.invol.check.geom_nonpositive", "DIN 5480-2 检验：m={v0}、z={v1}、x={v2} 的几何量非正（d_b={v3}、s={v4}）。", "DIN 5480-2 inspection: geometry is non-positive for m={v0}, z={v1}, x={v2} (d_b={v3}, s={v4})."),
    Msg::new("cmd.invol.check.footnote_a", "该表行带 a 脚注：测量点靠近齿根圆角或齿顶边缘，测量值不可靠（DIN 5480-2 表注）；棒间距/跨棒距 M1/M2 优先。", "This table row has footnote a: the measuring point is close to the root fillet or tip edge, so the measured value is unreliable (DIN 5480-2 table note); prefer between-balls/over-balls M1/M2."),
    Msg::new("cmd.invol.check.src_table", "查表 {v0}", "Table hit {v0}"),
    Msg::new("cmd.invol.check.src_formula_z", "公式导出（z={z} 不在检验表中）", "Formula-derived (z={z} is not in the inspection table)"),
    Msg::new("cmd.invol.check.notes_line1", "公式已与检验表逐行对照：可算 265/267 行，W_k/M1/M2 残差 ≤{v0}（17 处源表 OCR 异常已按源图核对）。", "The formula has been compared row by row with the inspection table: 265/267 rows computable, W_k/M1/M2 residuals ≤{v0} (17 source-table OCR anomalies verified against the source pages)."),
    Msg::new("cmd.invol.check.notes_line2", "量棒直径借用邻近表行 {v0}：D_M_hub={v1}、D_M_shaft={v2}；k={k} 由跨测齿数规则反推。", "Ball diameters borrowed from the nearest table row {v0}: D_M_hub={v1}, D_M_shaft={v2}; k={k} reverse-derived from the span-teeth rule."),
    Msg::new("cmd.invol.check.summary", "M1={v0}（D_M={v1}） M2={v2}（D_M={v3}） k={v4} W_k={v5}；{v6}", "M1={v0} (D_M={v1}) M2={v2} (D_M={v3}) k={v4} W_k={v5}; {v6}"),
    Msg::new("cmd.invol.check.missing_db", "invol_check：缺 db（基准直径 d_B，例 ?db=40&m=2&z=18）", "invol_check: missing db (reference diameter d_B, e.g. ?db=40&m=2&z=18)"),
    Msg::new("cmd.invol.check.missing_m", "invol_check：缺 m（模数，例 ?db=40&m=2&z=18）", "invol_check: missing m (module, e.g. ?db=40&m=2&z=18)"),
    Msg::new("cmd.invol.check.missing_z", "invol_check：缺 z（齿数，例 ?db=40&m=2&z=18）", "invol_check: missing z (tooth count, e.g. ?db=40&m=2&z=18)"),
    Msg::new("cmd.invol.check.need_check_flag", "{v0}（公式导出请加 check=1 / CHECK）", "{v0} (for formula derivation add check=1 / CHECK)"),
    Msg::new("cmd.invol.check.residual_x_note", "名义表无该 (m,d_B,z)（z 疑 OCR 残行，不参与公式对照）", "the nominal table has no such (m,d_B,z) (z is probably an OCR residue row, not used in the formula comparison)"),
    Msg::new("cmd.invol.check.m1_fail", "M1 计算失败：{e} ", "M1 computation failed: {e} "),
    Msg::new("cmd.invol.check.m2_fail", "M2 计算失败：{e} ", "M2 computation failed: {e} "),
    Msg::new("cmd.invol.check.ocr_fixed_ok", "通过（≤2e-3）", "passes (≤2e-3)"),
    Msg::new("cmd.invol.check.ocr_still_bad", "仍不一致", "still inconsistent"),
    Msg::new("cmd.invol.check.mismatch_no_fix", "与公式不一致（{v0}，无源图核对记录）", "inconsistent with the formula ({v0}, no source-page verification record)"),
    Msg::new("cmd.invol.check.fixed_pass", "修正后通过", "passes after correction"),
    Msg::new("cmd.invol.check.validated", "验证通过", "validated"),
    Msg::new("cmd.invol.check.not_validated", "未通过", "not validated"),
    Msg::new("cmd.invol.check.pitch_pos", "径节 P={v0} 必须是正数。", "Pitch P={v0} must be positive."),
    Msg::new("cmd.invol.spec.internal_head", "内花键 ", "Internal spline "),
    Msg::new("cmd.invol.section.internal_note", "内花键剖视图按内齿轮口径出图（齿圈内齿不剖：端面/齿顶线/齿根线/内孔壁/孔口倒角 + 分度线/轴线，不打剖面线），由 gear.rs 的 internal_bore_section 模板生成，不在引擎侧出图。", "The internal-spline section view follows the internal-gear convention (ring internal teeth are not sectioned: end face/tip line/root line/bore wall/bore chamfer + pitch line/axis, no hatching) and is generated by the gear.rs internal_bore_section template, not by the engine."),
    Msg::new("cmd.invol.runout.de_gt_da", "渐开线花键：滚刀外径 de={v0} 必须大于外花键大径 da={v1}（否则收尾弧切不出去）。", "Involute spline: hob outside diameter de={v0} must be greater than the external-spline major diameter da={v1} (otherwise the runout arc cannot cut out)."),
    Msg::new("cmd.invol.spec.ansi_pitch_circle", "（节圆 φ{v0}）", " (pitch circle φ{v0})"),
    Msg::new("cmd.invol.err.tooth_tip_crossed", "{v0} {v1}：齿顶变尖（ψ(da/2)={v2}° ≤ 0），两条渐开线在齿顶圆之前相交，端视图画不出真实齿廓；请减小 ha* / 增大齿数。", "{v0} {v1}: tooth tip becomes pointed (ψ(da/2)={v2}° ≤ 0); the two involutes intersect before the tip circle, so the end view cannot draw a real profile; reduce ha* or increase the tooth count."),
    Msg::new("cmd.invol.err.root_space_crossed", "{v0} {v1}：齿槽过宽（ψ(r_start)={v2}° ≥ 半齿距 {v3}°），相邻齿廓在齿根之前相交，端视图画不出真实齿廓；请减小 hf*/α 或增大齿数。", "{v0} {v1}: tooth space too wide (ψ(r_start)={v2}° ≥ half pitch {v3}°); adjacent profiles intersect before the root, so the end view cannot draw a real profile; reduce hf*/α or increase the tooth count."),
    Msg::new("cmd.invol.err.internal_space_crossed", "{v0} {v1} 内花键：齿槽过宽（ψ={v2}° ≥ 半齿距 {v3}°），相邻齿槽的齿廓在齿顶之前相交，端视图画不出真实齿廓；请减小 hf*/α 或增大齿数。", "{v0} {v1} internal spline: tooth space too wide (ψ={v2}° ≥ half pitch {v3}°); adjacent space profiles intersect before the tip, so the end view cannot draw a real profile; reduce hf*/α or increase the tooth count."),
    Msg::new("cmd.invol.err.internal_root_crossed", "{v0} {v1} 内花键：齿槽在齿根处已相交（ψ(D_ei/2)={v2}° ≤ 0），端视图画不出真实齿廓；请减小齿槽宽（如减小 hf* 或调整 x）。", "{v0} {v1} internal spline: tooth space already intersects at the root (ψ(D_ei/2)={v2}° ≤ 0); the end view cannot draw a real profile; reduce the space width (e.g. reduce hf* or adjust x)."),
    Msg::new("cmd.invol.runout.de_pos", "渐开线花键：滚刀外径 de={de} 必须是正数。", "Involute spline: hob outside diameter de={de} must be positive."),
    Msg::new("cmd.invol.check.inv_delta_no_solution", "DIN 5480-2 检验：invδ={v0} 在范围内无解（m={v1}、z={v2}、x={v3}、D_M={v4}），请核对量棒/参数。", "DIN 5480-2 inspection: invδ={v0} has no solution in range (m={v1}, z={v2}, x={v3}, D_M={v4}); check the measuring balls/parameters."),
    Msg::new("cmd.invol.check.z_no_formula", "DIN 5480-2 检验表：z={z} 不在表中、公式未验证通过，请提供对应表页。", "DIN 5480-2 inspection table: z={z} is not in the table and the formula has not been validated; provide the corresponding table page."),
    Msg::new("cmd.invol.check.z_no_ball_row", "DIN 5480-2 检验表：z={z} 不在表中，且公式不可用（缺量棒直径 D_M）——{v0}请提供对应表页。", "DIN 5480-2 inspection table: z={z} is not in the table and the formula is unusable (missing ball diameter D_M) — {v0} provide the corresponding table page."),
    Msg::new("cmd.invol.check.z_x_out_of_range", "DIN 5480-2 检验表：z={z} 不在表中；由 d_B={v0}、m={v1} 反解 x={v2} 超出 x∈[−0.05, 0.45]（DIN 5480-1），公式不可用 —— 请提供对应表页（数据来源：检验表 267 行 / p12·p16·p18·p20 + m=1.5/m=5 截图）。", "DIN 5480-2 inspection table: z={z} is not in the table; from d_B={v0}, m={v1} the inverse solution x={v2} lies outside x∈[−0.05, 0.45] (DIN 5480-1), so the formula is unusable — provide the corresponding table page (data source: inspection table 267 rows / p12·p16·p18·p20 + m=1.5/m=5 screenshots)."),
    Msg::new("cmd.invol.check.anomaly_row", "p{v0} d_B={v1} m={v2} z={v3}（{v4}）：{v5}", "p{v0} d_B={v1} m={v2} z={v3} ({v4}): {v5}"),
    Msg::new("cmd.invol.check.ocr_anomaly", "源表 OCR 异常（{v0} 与公式不一致）：{v1} 印为 {v2}、源图为 {v3}；修正后{v4}。", "source-table OCR anomaly ({v0} inconsistent with the formula): {v1} printed as {v2}, source page shows {v3}; after correction {v4}."),
    Msg::new("cmd.invol.check.formula_summary", "DIN 5480-2 检验表逐行对照：{v0} 行（可算 {v1}，跳过 {v2} 行无名义 x）；W_k {v3}/{v4} 行、M1 {v5}/{v6} 行、M2 {v7}/{v8} 行残差 ≤{v9}（max W_k {v10} / M1 {v11} / M2 {v12}）；源数据 OCR 异常 {v13} 处（已按源图核对、未改写 CSV，修正后全部 ≤{v14}）：公式{v15}。", "DIN 5480-2 inspection table row-by-row comparison: {v0} rows (computable {v1}, skipped {v2} rows without a nominal x); W_k {v3}/{v4} rows, M1 {v5}/{v6} rows, M2 {v7}/{v8} rows with residual ≤{v9} (max W_k {v10} / M1 {v11} / M2 {v12}); {v13} source-data OCR anomalies (verified against source pages, CSV not rewritten; after correction all ≤{v14}): formula {v15}."),
    Msg::new("cmd.invol.report.w_result", "W = {v0} mm（表值 E={v1}，残差 {v2}）", "W = {v0} mm (table value E={v1}, residual {v2})"),
    Msg::new("cmd.invol.report.wn_identity_line", "- 单位：长度 mm；倒式 `invα = tanα − α`。恒等式自检：|W − {v0}| < 1e-9（同一函数返回值）。\n\n", "- Units: length mm; inverse form `invα = tanα − α`. Identity self-check: |W − {v0}| < 1e-9 (returned by the same function).\n\n"),
    Msg::new("cmd.invol.report.identity_line", "- 恒等式自检（定义式，残差应 0）：|d−m·z|={v0}、|db−d·cosα|={v1}、|d′−(d+2xm)|={v2}；违例 0。\n", "- Identity self-check (definition equations, residuals should be 0): |d−m·z|={v0}, |db−d·cosα|={v1}, |d′−(d+2xm)|={v2}; violations 0.\n"),
    Msg::new("cmd.invol.report.identity_din", "- DIN 基准直径恒等式：|d_B−(m(z+1.1+2x))|={v0}（721 行名义表逐行残差 0）。\n", "- DIN reference-diameter identity: |d_B−(m(z+1.1+2x))|={v0} (721 nominal-table rows, residual 0).\n"),
    Msg::new("cmd.invol.report.identity_nf", "- NF 公称直径恒等式：|A−(m(N+2x+0.4))|={v0}（NF E22-141 p07）。\n", "- NF nominal-diameter identity: |A−(m(N+2x+0.4))|={v0} (NF E22-141 p07).\n"),
    Msg::new("cmd.invol.din.bench_hit_z", "查表命中 d_B={v0}、m={v1} → z={v2}", "Table hit d_B={v0}, m={v1} → z={v2}"),
    Msg::new("cmd.invol.din.bench_multi_z", "查表命中 d_B={v0}、m={v1} 的多个 z 变体（{v2}），按 |x| 最小取 z={v3}", "Table hit d_B={v0}, m={v1} has several z variants ({v2}); take the smallest |x|, z={v3}"),
    Msg::new("cmd.invol.din.z_with_x", "z={v0}（x={v1}）", "z={v0} (x={v1})"),
    Msg::new("cmd.invol.nf.bench_hit_n", "查表命中 A={v0}、m={v1} → N={v2}", "Table hit A={v0}, m={v1} → N={v2}"),
    Msg::new("cmd.invol.ocr_fix_note", "（OCR 修正：{v0}）", " (OCR correction: {v0})"),
    Msg::new("cmd.invol.check.z_range_insp_table", "DIN 5480-2 检验表：齿数 z={v0} 超出范围（3..=1000）。", "DIN 5480-2 inspection table: tooth count z={v0} is out of range (3..=1000)."),
    Msg::new("cmd.invol.report.t_qty_notes", "| 量 | 值 | 说明 |\n|---|---|---|\n", "| Quantity | Value | Notes |\n|---|---|---|\n"),
    Msg::new("cmd.invol.din.keep_z_from_db_m", "按基准直径 d_B={v0} 取 z={v1}（与输入 z={v2} 不符，d_B 为主参数；z 由 d_B 与 m 决定；由 d_B=m(z+1.1+2x)、x∈[−0.05,0.45] 得 z∈[{v3},{v4}]）", "Take z={v1} from the reference diameter d_B={v0} (input z={v2} differs; d_B is the master parameter; z follows from d_B and m; from d_B=m(z+1.1+2x), x∈[−0.05,0.45] we get z∈[{v3},{v4}])"),
    Msg::new("cmd.invol.din.keep_z_from_db", "按基准直径 d_B={v0} 取 z={v1}（与输入 z={v2} 不符，d_B 为主参数；由 d_B=m(z+1.1+2x)、x∈[−0.05,0.45] 得 z∈[{v3},{v4}]）", "Take z={v1} from the reference diameter d_B={v0} (input z={v2} differs; d_B is the master parameter; from d_B=m(z+1.1+2x), x∈[−0.05,0.45] we get z∈[{v3},{v4}])"),
    Msg::new("cmd.invol.din.also_x", "；同时按 d_B 取 x={v0}（与输入 x={v1} 不符）", "; meanwhile x={v0} is taken from d_B (input x={v1} differs)"),
    Msg::new("cmd.invol.din.table_hit_x", "查表命中 p{v0} m={v1}；按基准直径 d_B={v2} 取 x={v3}（与输入 x={v4} 不符，d_B 为主参数）", "Table hit p{v0} m={v1}; x={v3} is taken from the reference diameter d_B={v2} (input x={v4} differs; d_B is the master parameter)"),
    Msg::new("cmd.invol.din.keep_x_from_db", "按基准直径 d_B={v0} 取 x={v1}（与输入 x={v2} 不符，d_B 为主参数）", "Take x={v1} from the reference diameter d_B={v0} (input x={v2} differs; d_B is the master parameter)"),
    Msg::new("cmd.invol.din.row_inconsistent", "DIN 5480：查表行 p{v0} m={v1} z={v2} x={v3} 不自洽：{v4}", "DIN 5480: table row p{v0} m={v1} z={v2} x={v3} is inconsistent: {v4}"),
    Msg::new("cmd.invol.nf.keep_n_from_a", "按公称直径 A={v0} 取 N={v1}（与输入 N={v2} 不符，A 为主参数；N 由 A 与 m 决定）", "Take N={v1} from the nominal diameter A={v0} (input N={v2} differs; A is the master parameter; N follows from A and m)"),
    Msg::new("cmd.invol.nf.also_x", "；同时按 A 取 x={v0}（与输入 x={v1} 不符）", "; meanwhile x={v0} is taken from A (input x={v1} differs)"),
    Msg::new("cmd.invol.nf.table_hit_x", "查表命中 p{v0} m={v1} A={v2}{v3}；按 A 取 x={v4}（与输入 x={v5} 不符，A 为主参数）", "Table hit p{v0} m={v1} A={v2}{v3}; x={v4} is taken from A (input x={v5} differs; A is the master parameter)"),
    Msg::new("cmd.invol.nf.table_x_mismatch", "查表命中 p{v0} m={v1} A={v2}{v3}；表值 x={v4} 与 A 公式解 x={v5} 不符（超排版精度），按 A 取公式解", "Table hit p{v0} m={v1} A={v2}{v3}; table value x={v4} differs from the A-formula solution x={v5} (beyond typesetting precision); take the formula solution from A"),
    Msg::new("cmd.invol.nf.keep_x_from_a", "按公称直径 A={v0} 取 x={v1}（与输入 x={v2} 不符，A 为主参数）", "Take x={v1} from the nominal diameter A={v0} (input x={v2} differs; A is the master parameter)"),
    // —— 轴族（JSON/几何校验/计算书）补齐（§26）——
    // —— 阶段 3⑦ 批：卡预览读数行 k 标签（§37，60 条）——
    Msg::new("gui.ansi.readout.pitch", "径节 P/Ps", "Diametral pitch P/Ps"),
    Msg::new("gui.ansi.readout.teeth", "齿数 N", "Number of teeth N"),
    Msg::new("gui.ansi.readout.alpha", "压力角 φD", "Pressure angle φD"),
    Msg::new("gui.ansi.readout.pitch_dia", "节圆直径 D", "Pitch diameter D"),
    Msg::new("gui.ansi.readout.base_dia", "基圆直径 Db", "Base diameter Db"),
    Msg::new("gui.gear.readout.module", "反解：模数 m", "Back-solved: module m"),
    Msg::new("gui.gear.readout.teeth", "齿数 z", "Number of teeth z"),
    Msg::new("gui.gear.readout.alpha", "齿形角 α", "Profile angle α"),
    Msg::new("gui.gear.readout.shift", "变位 x", "Profile shift x"),
    Msg::new("gui.gear.readout.beta", "螺旋角 β", "Helix angle β"),
    Msg::new("gui.gear.readout.expr_da_df", "表达式 Da / Df", "Expression Da / Df"),
    Msg::new("gui.nf.readout.nominal_a", "公称直径 A（主参数）", "Nominal diameter A (main parameter)"),
    Msg::new("gui.nf.readout.major_dia", "大径 Az（{az_formula}）", "Major diameter Az ({az_formula})"),
    Msg::new("gui.nf.readout.minor_dia", "小径 D = A − 2m", "Minor diameter D = A − 2m"),
    Msg::new("gui.nf.readout.table_minor_dia", "p18 表小径 D 交叉核对", "p18 table minor diameter D cross-check"),
    Msg::new("gui.nf.readout.pin_v", "量棒 V / V1", "Pin V / V1"),
    Msg::new("gui.nf.readout.over_pins_g", "跨棒距 G / G1", "Measurement over pins G / G1"),
    Msg::new("gui.nf.readout.root_fillet_ri", "槽底圆角 ri（p22）", "Root fillet ri (p22)"),
    Msg::new("gui.nf.readout.shift_x", "变位系数 x（p22）", "Profile shift x (p22)"),
    Msg::new("gui.nf.readout.p29_int_dev", "p29 内花键偏差（µm）", "p29 internal spline deviations (µm)"),
    Msg::new("gui.nf.readout.p29_ext_dev", "配对外花键·{fit}偏差（µm；p29）", "Mating external spline · {fit} deviations (µm; p29)"),
    Msg::new("gui.nf.readout.major_tol", "大径上/下差", "Major dia. upper/lower deviation"),
    Msg::new("gui.nf.readout.minor_tol", "小径上/下差", "Minor dia. upper/lower deviation"),
    Msg::new("gui.nf.readout.p18_row", "p18 行", "p18 row"),
    Msg::new("gui.nf.readout.p22_row", "p22 行", "p22 row"),
    Msg::new("gui.nf.readout.p25_row", "p25 行", "p25 row"),
    Msg::new("gui.nf_ext.readout.nominal_a", "公称直径 A（主参数）", "Nominal diameter A (main parameter)"),
    Msg::new("gui.nf_ext.readout.major_dia", "大径 Dee（{dee_formula}）", "Major diameter Dee ({dee_formula})"),
    Msg::new("gui.nf_ext.readout.minor_dia", "小径 Die（{die_formula}）", "Minor diameter Die ({die_formula})"),
    Msg::new("gui.nf_ext.readout.table_root_dia", "p20–p22 表齿根圆交叉核对（平 / 圆）", "p20–p22 table root-diameter cross-check (flat / round)"),
    Msg::new("gui.nf_ext.readout.span_teeth_base_tangent", "跨测齿数 K / 公法线 W", "Span teeth K / base tangent length W"),
    Msg::new("gui.nf_ext.readout.shift_x", "变位系数 x（p20–p22）", "Profile shift x (p20–p22)"),
    Msg::new("gui.nf_ext.readout.pitch_base_dia", "分度圆 d / 基圆 dB", "Pitch diameter d / base diameter dB"),
    Msg::new("gui.nf_ext.readout.thickness_s_sb", "分度圆弧齿厚 s / 基圆 sB", "Circular tooth thickness s / base sB"),
    Msg::new("gui.nf_ext.readout.root_radius", "齿根圆角 Rf / Rr", "Root fillet Rf / Rr"),
    Msg::new("gui.nf_ext.readout.tip_chamfer_h", "齿顶倒角高度 h", "Tip chamfer height h"),
    Msg::new("gui.nf_ext.readout.p29_ext_dev", "p29 外花键·{fit}偏差（µm）", "p29 external spline · {fit} deviations (µm)"),
    Msg::new("gui.nf_ext.readout.p29_int_dev", "p29 内花键 E 偏差（µm；对照）", "p29 internal spline E deviations (µm; reference)"),
    Msg::new("gui.nf_ext.readout.major_tol", "大径上/下差", "Major dia. upper/lower deviation"),
    Msg::new("gui.nf_ext.readout.minor_tol", "小径上/下差", "Minor dia. upper/lower deviation"),
    Msg::new("gui.nf_ext.readout.p20_row", "p20–p22 行", "p20–p22 row"),
    Msg::new("gui.nf_ext.readout.p23_row", "p23–p25 行", "p23–p25 row"),
    Msg::new("gui.din.readout.e2_s1", "e₂ = s₁（名义）", "e₂ = s₁ (nominal)"),
    Msg::new("gui.din.readout.ae", "Ae（孔 {hub}）", "Ae (hub {hub})"),
    Msg::new("gui.din.readout.as", "As（轴 {shaft}）", "As (shaft {shaft})"),
    Msg::new("gui.din.readout.tact_hub", "孔 {hub}：Tact / Teff", "Hub {hub}: Tact / Teff"),
    Msg::new("gui.din.readout.tact_shaft", "轴 {shaft}：Tact / Teff", "Shaft {shaft}: Tact / Teff"),
    Msg::new("gui.din.readout.anchor_path", "Bild 6 示例路径", "Bild 6 example path"),
    Msg::new("gui.din.readout.ae_source", "Ae 出处", "Ae source"),
    Msg::new("gui.din.readout.as_source", "As 出处", "As source"),
    Msg::new("gui.din.readout.tol_hub_source", "孔公差出处", "Hub tolerance source"),
    Msg::new("gui.din.readout.tol_shaft_source", "轴公差出处", "Shaft tolerance source"),
    Msg::new("gui.din.readout.nominal_table", "名义表", "Nominal table"),
    Msg::new("gui.gb_lite.readout.module", "模数 m", "Module m"),
    Msg::new("gui.gb_lite.readout.teeth", "齿数 z", "Number of teeth z"),
    Msg::new("gui.gb_lite.readout.alpha", "齿形角 αD", "Pressure angle αD"),
    Msg::new("gui.gb_lite.readout.root_form", "齿根样式", "Root form"),
    Msg::new("gui.gb_lite.readout.side", "方向", "Direction"),
    Msg::new("gui.din.readout.anchor_yes", "是（整表照标准原印值）", "Yes (whole table takes the standard's printed values)"),
    Msg::new("gui.din.readout.anchor_no", "否（按公式/表值计算）", "No (computed from formulas/table values)"),

    // —— 阶段 3⑦ 批：spline_gui.rs GB 专用面板元数据（§37，119 条）——
    Msg::new("gui.gb.col.alpha.formula", "GB/T 3478.1 基本齿廓：αD = 30° / 37.5° / 45°（本页所选值）", "GB/T 3478.1 basic profile: αD = 30° / 37.5° / 45° (the value selected on this page)"),
    Msg::new("gui.gb.col.alpha.label", "齿形角", "Pressure angle"),
    Msg::new("gui.gb.col.alpha.source", "GB/T 3478.1 基本齿廓", "GB/T 3478.1 basic profile"),
    Msg::new("gui.gb.col.base_tangent.formula", "W_min = cosαD[(K−0.5)πm + D·invαD + esv − (T+λ)]；W_mid = (W_min+W_max)/2", "W_min = cosαD[(K−0.5)πm + D·invαD + esv − (T+λ)]; W_mid = (W_min+W_max)/2"),
    Msg::new("gui.gb.col.base_tangent.label", "公法线长度", "Base tangent length"),
    Msg::new("gui.gb.col.base_tangent.source", "GB/T 3478.6 式(11)(12)", "GB/T 3478.6 eqs. (11)(12)"),
    Msg::new("gui.gb.col.base_tangent_down.formula", "W_min − W_mid", "W_min − W_mid"),
    Msg::new("gui.gb.col.base_tangent_down.label", "公法线长度下公差", "Base tangent length lower deviation"),
    Msg::new("gui.gb.col.base_tangent_up.formula", "W_max − W_mid，W_max = W_min + T·cosαD", "W_max − W_mid, W_max = W_min + T·cosαD"),
    Msg::new("gui.gb.col.base_tangent_up.label", "公法线长度上公差", "Base tangent length upper deviation"),
    Msg::new("gui.gb.col.base_tangent_up.source", "GB/T 3478.6 式(12)", "GB/T 3478.6 eq. (12)"),
    Msg::new("gui.gb.col.ext_form_dia_max.formula", "D_Femax 按表 3 注 3（h_s 见图 2；含 esv 修正）", "D_Femax follows table 3 note 3 (h_s in figure 2; includes the esv correction)"),
    Msg::new("gui.gb.col.ext_form_dia_max.source", "GB/T 3478.1 表 3 注 3", "GB/T 3478.1 table 3 note 3"),
    Msg::new("gui.gb.col.ext_grade_fit.formula", "等级 4/5/6/7；基本偏差 h/js/k/d/e/f（与内花键 H 相配）", "Grades 4/5/6/7; fundamental deviations h/js/k/d/e/f (mating with internal spline H)"),
    Msg::new("gui.gb.col.ext_grade_fit.source", "GB/T 3478.1 §8.7.2、表 23/24", "GB/T 3478.1 §8.7.2, tables 23/24"),
    Msg::new("gui.gb.col.ext_major_dia.formula", "Dee = m(z+1) 30° / m(z+0.9) 37.5° / m(z+0.8) 45°", "Dee = m(z+1) 30° / m(z+0.9) 37.5° / m(z+0.8) 45°"),
    Msg::new("gui.gb.col.ext_major_dia_down.formula", "Dee 用 IT12/IT13/IT14（模数档 0.25~0.75 / 1~1.75 / 2~10）", "Dee uses IT12/IT13/IT14 (module steps 0.25–0.75 / 1–1.75 / 2–10)"),
    Msg::new("gui.gb.col.ext_major_dia_up.formula", "Dee 上偏差取 0", "Dee upper deviation is 0"),
    Msg::new("gui.gb.col.ext_minor_dia.formula", "Die = m(z−1.5) 30°平 / m(z−1.8) 30°圆 / m(z−1.4) 37.5° / m(z−1.2) 45°", "Die = m(z−1.5) 30° flat / m(z−1.8) 30° fillet / m(z−1.4) 37.5° / m(z−1.2) 45°"),
    Msg::new("gui.gb.col.ext_minor_dia_down.formula", "esv/tanαD − IT（Die 公差用 IT12/IT13/IT14，模数档 0.25~0.75 / 1~1.75 / 2~10）", "esv/tanαD − IT (Die tolerance uses IT12/IT13/IT14, module steps 0.25–0.75 / 1–1.75 / 2–10)"),
    Msg::new("gui.gb.col.ext_minor_dia_down.source", "GB/T 3478.1 表 24、表 25", "GB/T 3478.1 tables 24 and 25"),
    Msg::new("gui.gb.col.ext_minor_dia_up.formula", "esv/tanαD；d/e/f 查表 24、h=0、js=+(T+λ)/(2tanαD)、k=+(T+λ)/tanαD", "esv/tanαD; d/e/f from table 24, h=0, js=+(T+λ)/(2tanαD), k=+(T+λ)/tanαD"),
    Msg::new("gui.gb.col.ext_minor_dia_up.source", "GB/T 3478.1 表 24", "GB/T 3478.1 table 24"),
    Msg::new("gui.gb.col.form_dia_max.formula", "D_Fimin = m(z+1)/(z+0.9)/(z+0.8) + 2CF，CF = 0.1m（表 3 注 4：非 H/h 有变化，标准全 70 页未列值）", "D_Fimin = m(z+1)/(z+0.9)/(z+0.8) + 2CF, CF = 0.1m (table 3 note 4: it varies for fits other than H/h; the 70-page standard lists no values)"),
    Msg::new("gui.gb.col.form_dia_max.label", "渐开线终止圆直径最大值", "Max. form diameter"),
    Msg::new("gui.gb.col.form_dia_max.source", "GB/T 3478.1 表 3 与注 4（全文档无 CF 变化值表）", "GB/T 3478.1 table 3 and note 4 (the whole document has no CF variation table)"),
    Msg::new("gui.gb.col.grade_fit.formula", "等级 4/5/6/7（表 7~21 的 (T+λ) 系数 10/16/25/40）；内花键恒为基孔制 H", "Grades 4/5/6/7 ((T+λ) coefficients 10/16/25/40 from tables 7–21); an internal spline is always hole-basis H"),
    Msg::new("gui.gb.col.grade_fit.label", "公差等级和配合类别", "Tolerance grade and fit"),
    Msg::new("gui.gb.col.grade_fit.source", "GB/T 3478.1 §8.7、表 23", "GB/T 3478.1 §8.7, table 23"),
    Msg::new("gui.gb.col.major_dia.formula", "Dei = m(z+1.5) 30°平 / m(z+1.8) 30°圆 / m(z+1.4) 37.5° / m(z+1.2) 45°", "Dei = m(z+1.5) 30° flat / m(z+1.8) 30° fillet / m(z+1.4) 37.5° / m(z+1.2) 45°"),
    Msg::new("gui.gb.col.major_dia.label", "大径", "Major diameter"),
    Msg::new("gui.gb.col.major_dia.source", "GB/T 3478.1 表 3", "GB/T 3478.1 table 3"),
    Msg::new("gui.gb.col.major_dia_down.formula", "基孔制 H：下偏差 0", "Hole-basis H: lower deviation 0"),
    Msg::new("gui.gb.col.major_dia_down.label", "大径下公差", "Major dia. lower deviation"),
    Msg::new("gui.gb.col.major_dia_up.formula", "Dei 用 IT12/IT13/IT14（模数档 0.25~0.75 / 1~1.75 / 2~10）", "Dei uses IT12/IT13/IT14 (module steps 0.25–0.75 / 1–1.75 / 2–10)"),
    Msg::new("gui.gb.col.major_dia_up.label", "大径上公差", "Major dia. upper deviation"),
    Msg::new("gui.gb.col.major_dia_up.source", "GB/T 3478.1 表 24 脚注①", "GB/T 3478.1 table 24 footnote ①"),
    Msg::new("gui.gb.col.minor_dia.formula", "Dii = D_Femax(H/h) + 2CF（表 3 注 2）；D_Femax 按表 3 注 3（h_s 见图 2）", "Dii = D_Femax(H/h) + 2CF (table 3 note 2); D_Femax follows table 3 note 3 (h_s in figure 2)"),
    Msg::new("gui.gb.col.minor_dia.label", "小径", "Minor diameter"),
    Msg::new("gui.gb.col.minor_dia.source", "GB/T 3478.1 表 3 注 2/注 3", "GB/T 3478.1 table 3 notes 2/3"),
    Msg::new("gui.gb.col.minor_dia_down.formula", "Dii 用 H10/H11/H12（模数档 0.25~0.75 / 1~1.75 / 2~10），下偏差 0", "Dii uses H10/H11/H12 (module steps 0.25–0.75 / 1–1.75 / 2–10), lower deviation 0"),
    Msg::new("gui.gb.col.minor_dia_down.label", "小径下公差", "Minor dia. lower deviation"),
    Msg::new("gui.gb.col.minor_dia_down.source", "GB/T 3478.1 表 25、GB/T 1800", "GB/T 3478.1 table 25, GB/T 1800"),
    Msg::new("gui.gb.col.minor_dia_up.formula", "+IT10/IT11/IT12（模数档同上）；表 25 原页 Dii 列为图片，已核对为 +IT/0", "+IT10/IT11/IT12 (same module steps); the Dii column is an image on the original page 25 and was verified to be +IT/0"),
    Msg::new("gui.gb.col.minor_dia_up.label", "小径上公差", "Minor dia. upper deviation"),
    Msg::new("gui.gb.col.minor_dia_up.source", "GB/T 3478.1 表 25", "GB/T 3478.1 table 25"),
    Msg::new("gui.gb.col.module.label", "模数", "Module"),
    Msg::new("gui.gb.col.over_pins_down.formula", "M_min − M_mid（E_min = E + λ 一侧）", "M_min − M_mid (the E_min = E + λ side)"),
    Msg::new("gui.gb.col.over_pins_down.label", "测量跨棒距下公差", "Measurement over pins lower deviation"),
    Msg::new("gui.gb.col.over_pins_md.formula", "偶齿 M = Db/cosαi ∓ Dp；奇齿再乘 cos(90°/z)；αi 由 E/D + invαD − Dp/Db 反解", "even teeth M = Db/cosαi ∓ Dp; for odd teeth multiply by cos(90°/z); αi is back-solved from E/D + invαD − Dp/Db"),
    Msg::new("gui.gb.col.over_pins_md.label", "测量跨棒距", "Measurement over pins"),
    Msg::new("gui.gb.col.over_pins_md.source", "GB/T 3478.6 式(2)~(5)", "GB/T 3478.6 eqs. (2)–(5)"),
    Msg::new("gui.gb.col.over_pins_up.formula", "M_max − M_mid（E_max = E + (T+λ) 一侧）", "M_max − M_mid (the E_max = E + (T+λ) side)"),
    Msg::new("gui.gb.col.over_pins_up.label", "测量跨棒距上公差", "Measurement over pins upper deviation"),
    Msg::new("gui.gb.col.pin_dia_dp.formula", "D'_Ri = Db[tanαci − tan(αci − E_max/D + invαci − invαD)]，再按 R40 取最接近较大值", "D'_Ri = Db[tanαci − tan(αci − E_max/D + invαci − invαD)], then take the nearest larger R40 value"),
    Msg::new("gui.gb.col.pin_dia_dp.label", "量棒直径", "Pin diameter"),
    Msg::new("gui.gb.col.pin_dia_dp.source", "GB/T 3478.6 式(1)、GB/T 3478.9 表 1", "GB/T 3478.6 eq. (1), GB/T 3478.9 table 1"),
    Msg::new("gui.gb.col.pitch_cum_tol.formula", "Fp = a√L + b，L = πmz/2（4/5/6/7 级系数 2.5/3.55/5/7.1、6.3/9/12.5/18）", "Fp = a√L + b, L = πmz/2 (coefficients 2.5/3.55/5/7.1 and 6.3/9/12.5/18 for grades 4/5/6/7)"),
    Msg::new("gui.gb.col.pitch_cum_tol.label", "齿距累计公差", "Cumulative pitch tolerance"),
    Msg::new("gui.gb.col.pitch_cum_tol.source", "GB/T 3478.1 §8.2、表 7~21", "GB/T 3478.1 §8.2, tables 7–21"),
    Msg::new("gui.gb.col.profile_tol.formula", "Fα = aφ1 + b，φ1 = m + 0.0125mz（4/5/6/7 级系数 1.6/2.5/4/6.3、10/16/25/40）", "Fα = aφ1 + b, φ1 = m + 0.0125mz (coefficients 1.6/2.5/4/6.3 and 10/16/25/40 for grades 4/5/6/7)"),
    Msg::new("gui.gb.col.profile_tol.label", "齿形公差", "Profile tolerance"),
    Msg::new("gui.gb.col.profile_tol.source", "GB/T 3478.1 §8.3、表 7~21", "GB/T 3478.1 §8.3, tables 7–21"),
    Msg::new("gui.gb.col.root_fillet_radius.formula", "卡面 = 表 26（GB/T 3478.1 书页 50）该模数档表值；表 26 未列值（如 m=0.25 的 30°平/30°圆/37.5°）→「—」。计算口径 = 图 2 系数式 R_imin = 0.2m 30°平 / 0.4m 30°圆 / 0.3m 37.5° / 0.25m 45°（性质：内部/报告对照，不直接上卡面；有值格与表值一致）", "The card shows the table 26 value (GB/T 3478.1 page 50) for this module step; values not listed in table 26 (e.g. 30° flat / 30° fillet / 37.5° at m=0.25) show “—”. Calculation definition = figure 2 coefficient formula R_imin = 0.2m 30° flat / 0.4m 30° fillet / 0.3m 37.5° / 0.25m 45° (internal/report cross-check only, never shown directly on the card; where a value exists it matches the table value)"),
    Msg::new("gui.gb.col.root_fillet_radius.label", "齿根圆最小曲率半径", "Min. root fillet radius"),
    Msg::new("gui.gb.col.root_fillet_radius.source", "GB/T 3478.1-2008 表 26（书页 50）逐格核对；图 2 系数式作计算口径", "GB/T 3478.1-2008 table 26 (page 50) checked cell by cell; the figure 2 coefficient formula is the calculation definition"),
    Msg::new("gui.gb.col.space_width_max.formula", "E max = E + (T+λ)；(T+λ) = 10/16/25/40·i_d + 40/64/100/160·i_E（4/5/6/7 级）", "E max = E + (T+λ); (T+λ) = 10/16/25/40·i_d + 40/64/100/160·i_E (grades 4/5/6/7)"),
    Msg::new("gui.gb.col.space_width_max.label", "实际齿槽宽最大值", "Max. actual space width"),
    Msg::new("gui.gb.col.space_width_max.source", "GB/T 3478.1 §8.1、表 7~21", "GB/T 3478.1 §8.1, tables 7–21"),
    Msg::new("gui.gb.col.space_width_min.formula", "基本齿槽宽 E = 0.5πm（模板该列口径；与按 λ 修正的 E_min 分开）", "Basic space width E = 0.5πm (the template's definition for this column; kept separate from the λ-corrected E_min)"),
    Msg::new("gui.gb.col.space_width_min.label", "作用齿槽宽最小值", "Min. effective space width"),
    Msg::new("gui.gb.col.span_teeth.formula", "K = z/6 + 0.5 取整数", "K = z/6 + 0.5 rounded to an integer"),
    Msg::new("gui.gb.col.span_teeth.label", "跨测齿数", "Span teeth"),
    Msg::new("gui.gb.col.span_teeth.source", "GB/T 3478.6 式(11) 注", "GB/T 3478.6 eq. (11) note"),
    Msg::new("gui.gb.col.teeth.formula", "由九字段齿形表达式反解（MARK KIND M Z ALPHA X DA DF BETA H；轴/齿轮生成器 GUI 可复制）", "Back-solved from the nine-field tooth-form expression (MARK KIND M Z ALPHA X DA DF BETA H; copyable from the shaft/gear generator GUI)"),
    Msg::new("gui.gb.col.teeth.label", "齿数", "Number of teeth"),
    Msg::new("gui.gb.col.teeth.source", "手填 / 图纸继承", "Entered by hand / inherited from the drawing"),
    Msg::new("gui.gb.col.thickness_max.formula", "S_v max = S + esv，基本齿厚 S = 0.5πm", "S_v max = S + esv, basic tooth thickness S = 0.5πm"),
    Msg::new("gui.gb.col.thickness_max.label", "作用齿厚最大值", "Max. effective tooth thickness"),
    Msg::new("gui.gb.col.thickness_max.source", "GB/T 3478.1 表 3、表 23", "GB/T 3478.1 table 3, table 23"),
    Msg::new("gui.gb.col.thickness_min.formula", "S_min = S_v max − (T+λ)，S_v max = S + esv", "S_min = S_v max − (T+λ), S_v max = S + esv"),
    Msg::new("gui.gb.col.thickness_min.label", "实际齿厚最小值", "Min. actual tooth thickness"),
    Msg::new("gui.gb.col.total_composite.formula", "λ = 0.6√(Fp²+Fα²+Fβ²)，用未修约的 F 值", "λ = 0.6√(Fp²+Fα²+Fβ²), using unrounded F values"),
    Msg::new("gui.gb.col.total_composite.label", "综合公差", "Total composite tolerance"),
    Msg::new("gui.gb.col.total_composite.source", "GB/T 3478.1 §8.6", "GB/T 3478.1 §8.6"),
    Msg::new("gui.gb.fit.int.label", "H（内花键基孔制）", "H (internal spline, hole basis)"),
    Msg::new("gui.gb.fit.int.memo", "内花键恒 H；与外花键基本偏差 d/e/f/h/js/k 形成配合", "An internal spline is always H; it forms a fit with the external spline fundamental deviations d/e/f/h/js/k"),
    Msg::new("gui.gb.info.dynamic", "异常与需手填只在出现时显示", "errors and manual entries are shown only when they occur"),
    Msg::new("gui.gb.info.guide", "常显只放操作引导", "the always-visible area shows operating guidance only"),
    Msg::new("gui.gb.info.title", "公式/口径/来源一律进原生 title=", "formulas/definitions/sources always go into the native title="),
    Msg::new("gui.gb.pin.ext.dp_label", "量棒直径 D_Re", "Pin diameter D_Re"),
    Msg::new("gui.gb.pin.ext.formula", "D'_Re = Db[tan(α_ce + invα_ce + π/z − S_min/D − invαD) − tanα_ce]，α_ce = acos(Db/D_ce)、D_ce = (D_ee max + D_ii min)/2；S_min 按 7 级 + 基本偏差 h 取（GB/T 3478.6 §3.2.1 式(6)）", "D'_Re = Db[tan(α_ce + invα_ce + π/z − S_min/D − invαD) − tanα_ce], α_ce = acos(Db/D_ce), D_ce = (D_ee max + D_ii min)/2; S_min follows grade 7 + fundamental deviation h (GB/T 3478.6 §3.2.1 eq. (6))"),
    Msg::new("gui.gb.pin.ext.label", "量棒直径 D_Re 与跨棒距 M_Re（公法线为主、跨棒距备用）", "Pin diameter D_Re and measurement over pins M_Re (base tangent primary, over pins as backup)"),
    Msg::new("gui.gb.pin.ext.md_formula", "偶齿 M_Re max/min = Db/cosα_e max/min + D_Re；奇齿再乘 cos(90°/z)；invα_e min = D_Re/Db + invαD + S_min/D − π/z，invα_e max = D_Re/Db + invαD + S_max/D − π/z（GB/T 3478.6 §3.2.2 式(7)~(10)）", "even teeth M_Re max/min = Db/cosα_e max/min + D_Re; for odd teeth multiply by cos(90°/z); invα_e min = D_Re/Db + invαD + S_min/D − π/z, invα_e max = D_Re/Db + invαD + S_max/D − π/z (GB/T 3478.6 §3.2.2 eqs. (7)–(10))"),
    Msg::new("gui.gb.pin.ext.md_label", "跨棒距 M_Re", "Measurement over pins M_Re"),
    Msg::new("gui.gb.pin.ext.standard", "D'_Re 算完后按 GB/T 321 的 R40 系列取最接近且较大的值（GB/T 3478.9 表 1，67 档）；3 个备选 = 系列中与 D' 最接近的 3 个（工程口径）", "After D'_Re is computed, take the nearest larger value in the R40 series of GB/T 321 (GB/T 3478.9 table 1, 67 steps); the 3 alternatives are the 3 series values closest to D' (engineering practice)"),
    Msg::new("gui.gb.pin.int.dp_label", "量棒直径 Dp", "Pin diameter Dp"),
    Msg::new("gui.gb.pin.int.formula", "D'_Ri = Db[tanα_ci − tan(α_ci − E_max/D + invα_ci − invαD)]，α_ci = acos(Db/D_ci)、D_ci = (D_ee max + D_ii min)/2（GB/T 3478.6 §3.1.1 式(1)）", "D'_Ri = Db[tanα_ci − tan(α_ci − E_max/D + invα_ci − invαD)], α_ci = acos(Db/D_ci), D_ci = (D_ee max + D_ii min)/2 (GB/T 3478.6 §3.1.1 eq. (1))"),
    Msg::new("gui.gb.pin.int.label", "量棒直径 Dp 与测量跨棒距 Md", "Pin diameter Dp and measurement over pins Md"),
    Msg::new("gui.gb.pin.int.md_formula", "偶齿 M_Ri max/min = Db/cosα_i max/min − Dp；奇齿再乘 cos(90°/z)；invα_i max/min = E_max/min/D + invαD − Dp/Db（GB/T 3478.6 §3.1.2 式(2)~(5)）", "even teeth M_Ri max/min = Db/cosα_i max/min − Dp; for odd teeth multiply by cos(90°/z); invα_i max/min = E_max/min/D + invαD − Dp/Db (GB/T 3478.6 §3.1.2 eqs. (2)–(5))"),
    Msg::new("gui.gb.pin.int.md_label", "测量跨棒距 Md", "Measurement over pins Md"),
    Msg::new("gui.gb.pin.int.standard", "D'_Ri 算完后按 GB/T 321 的 R40 系列取最接近且较大的值（GB/T 3478.9 表 1，67 档）；3 个备选 = 系列中与 D' 最接近的 3 个（工程口径）", "After D'_Ri is computed, take the nearest larger value in the R40 series of GB/T 321 (GB/T 3478.9 table 1, 67 steps); the 3 alternatives are the 3 series values closest to D' (engineering practice)"),
    Msg::new("gui.gb.pin.not_applicable", "该方向的参数表不含量棒/跨棒距面板", "the parameter table for this direction has no pin / measurement-over-pins panel"),
    Msg::new("gui.gb.pin.tag_alt", "备选", "Alternative"),
    Msg::new("gui.gb.pin.tag_standard", "标准解", "Standard solution"),
    Msg::new("gui.gb.pin_series_note", "GB/T 3478.9-2008 表 1（67 档，R40；极限偏差 ±0.001 mm）", "GB/T 3478.9-2008 table 1 (67 steps, R40; limit deviation ±0.001 mm)"),
    Msg::new("gui.gb.root.fillet.note", "GB/T 3478.1 表 3：30° 圆齿根 Dei = m(z+1.8)、外花键 Die = m(z−1.8)；37.5°/45° 按各自系列公式", "GB/T 3478.1 table 3: 30° fillet root Dei = m(z+1.8), external spline Die = m(z−1.8); 37.5°/45° follow their own series formulas"),
    Msg::new("gui.gb.root.flat.note", "GB/T 3478.1 表 3：30° 平齿根 Dei = m(z+1.5)、外花键 Die = m(z−1.5)；37.5°/45° 允许平齿根（基本尺寸按圆齿根系列公式）", "GB/T 3478.1 table 3: 30° flat root Dei = m(z+1.5), external spline Die = m(z−1.5); 37.5°/45° allow a flat root (basic dimensions follow the fillet-root series formulas)"),
    Msg::new("gui.gb.side.ext.alpha_note", "压力角 αD = 30° / 37.5° / 45°（基本齿廓）；invαD = 0.0537515 / 0.1128285 / 0.2146018", "Pressure angle αD = 30° / 37.5° / 45° (basic profile); invαD = 0.0537515 / 0.1128285 / 0.2146018"),
    Msg::new("gui.gb.side.ext.grade_note", "公差等级 4/5/6/7：等级越高公差越小；(T+λ) 系数 10/16/25/40、i_S/i_d 同 GB/T 3478.1 §8.1（表 7~21）", "Tolerance grades 4/5/6/7: the higher the grade, the smaller the tolerance; (T+λ) coefficients 10/16/25/40 and i_S/i_d as in GB/T 3478.1 §8.1 (tables 7–21)"),
    Msg::new("gui.gb.side.ext.label", "外花键", "External spline"),
    Msg::new("gui.gb.side.ext.title", "外花键：基本偏差 d/e/f/h/js/k 与内花键 H 相配；参数表按 GB/T 3478.1 出公法线长度 Wn / 跨测齿数 Kn", "External spline: fundamental deviations d/e/f/h/js/k mate with internal spline H; the parameter table gives base tangent length Wn / span teeth Kn per GB/T 3478.1"),
    Msg::new("gui.gb.side.int.alpha_note", "压力角 αD = 30° / 37.5° / 45°（基本齿廓）；invαD = 0.0537515 / 0.1128285 / 0.2146018", "Pressure angle αD = 30° / 37.5° / 45° (basic profile); invαD = 0.0537515 / 0.1128285 / 0.2146018"),
    Msg::new("gui.gb.side.int.grade_note", "公差等级 4/5/6/7：等级越高公差越小；(T+λ) 系数 10/16/25/40、i_E/i_d 同 GB/T 3478.1 §8.1（表 7~21）", "Tolerance grades 4/5/6/7: the higher the grade, the smaller the tolerance; (T+λ) coefficients 10/16/25/40 and i_E/i_d as in GB/T 3478.1 §8.1 (tables 7–21)"),
    Msg::new("gui.gb.side.int.label", "内花键", "Internal spline"),
    Msg::new("gui.gb.side.int.title", "内花键（基孔制 H）：参数表含量棒直径 Dp 与测量跨棒距 Md", "Internal spline (hole-basis H): the parameter table includes pin diameter Dp and measurement over pins Md"),
    Msg::new("gui.gb.sys.label", "GB/T 3478.1-2008 渐开线花键（GB）", "GB/T 3478.1-2008 involute splines (GB)"),
    Msg::new("gui.gb.sys.note", "本期只实现 GB 体系；口径与数据见 assets/spline_gb3478_*.csv 的 source/note。以后加 ANSI / NF = 本表加数据行 + 引擎补口径。", "Only the GB system is implemented here; definitions and data are in the source/note columns of assets/spline_gb3478_*.csv. Adding ANSI / NF later means adding data rows to this table and completing the engine definitions."),
    Msg::new("gui.gb.sys.x_note", "GB/T 3478.1 基本齿廓不含变位系数 x；x 仅记录（从 GEAR / 轴块继承时带回）", "The GB/T 3478.1 basic profile has no profile shift x; x is recorded only (carried back when inherited from a GEAR / shaft block)"),

];

/// 手动覆盖：环境变量 `OCSMLANG`（最高优先级的进程外开关）。
pub const LANG_ENV: &str = "OCSMLANG";
/// 手动覆盖：`OCSM_LANG`（兼容别名）。
pub const LANG_ENV_ALIAS: &str = "OCSM_LANG";

// 0 = 自动（跟随环境），1 = zh，2 = en。
const LANG_AUTO: u8 = 0;
const LANG_ZH: u8 = 1;
const LANG_EN: u8 = 2;

static OVERRIDE: AtomicU8 = AtomicU8::new(LANG_AUTO);
/// 环境解析结果缓存（`OnceLock` 不能复位，测试要复位 → 用 `Mutex<Option<_>>`）。
static AUTO_CACHE: Mutex<Option<Lang>> = Mutex::new(None);
/// 运行期缺 key 点名表（有上限，避免异常路径把内存撑爆）。
static MISSING: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// 缺 key 记录上限。
const MISSING_LIMIT: usize = 256;

// 单测（`cargo test`）：语言覆盖**只作用于当前测试线程**。
// `cargo test` 一个进程里并行跑所有用例，而生产里语言是进程级；若测试里
// `set_lang` 也写全局，一个切 En 的用例会把断言中文文案的用例带崩（曾实测）。
// 测试线程内的产品路径仍走这层；服务端/其它线程用全局/环境（稳）。
// 产品构建没有这层：GUI `/api/i18n` 的切换仍是进程级 `OVERRIDE`。
#[cfg(test)]
thread_local! {
    static TEST_OVERRIDE: std::cell::Cell<Option<Lang>> = const { std::cell::Cell::new(None) };
}

/// 按 key 查词条（线性扫描；catalog 到几百条前够用，变大再换表）。
pub fn lookup(key: &str) -> Option<&'static Msg> {
    CATALOG.iter().find(|m| m.key == key)
}

/// 当前语言（解析见模块头注释）。
pub fn lang() -> Lang {
    #[cfg(test)]
    {
        // ★ 测试下语言**只认线程局部覆盖**（`set_lang`），不读进程 env：env 是进程级全局，
        // 与 `i18n::tests::env_resolution_*`（持锁期间临时改 `LANG`/`OCSMLANG`）并发时，
        // 未取全局锁的用例（如 `invol_spline` 的中文断言）会偶发拿到 en ⇒ 假红。
        // 2026-09-27 ② 卡面批实测：18 轮里 1 轮 7 红，全部落在未取锁的中文断言上。
        return TEST_OVERRIDE.with(|c| c.get()).unwrap_or(Lang::Zh);
    }
    #[cfg(not(test))]
    match OVERRIDE.load(Ordering::Relaxed) {
        LANG_ZH => Lang::Zh,
        LANG_EN => Lang::En,
        _ => {
            let mut cache = AUTO_CACHE.lock().unwrap_or_else(|e| e.into_inner());
            *cache.get_or_insert_with(resolve_env_lang)
        }
    }
}

/// 手动覆盖当前语言（GUI 设置 / 命令开关 / 测试）。
///
/// `cargo test` 下只覆盖**当前线程**（并行用例互不干扰，见 `TEST_OVERRIDE`）；
/// 产品构建写进程级 `OVERRIDE`（GUI `/api/i18n` 选一次全插件生效）。
pub fn set_lang(lang: Lang) {
    #[cfg(test)]
    TEST_OVERRIDE.with(|c| c.set(Some(lang)));
    #[cfg(not(test))]
    OVERRIDE.store(
        match lang {
            Lang::Zh => LANG_ZH,
            Lang::En => LANG_EN,
        },
        Ordering::Relaxed,
    );
}

/// 复位为「自动」（跟随环境；顺带丢掉环境解析缓存，避免拿到旧值）。
pub fn set_lang_auto() {
    #[cfg(test)]
    TEST_OVERRIDE.with(|c| c.set(None));
    #[cfg(not(test))]
    OVERRIDE.store(LANG_AUTO, Ordering::Relaxed);
    reset_auto_cache();
}

/// 复位环境解析缓存（进程内改了 `LANG*` / `OCSMLANG` 后想立刻生效时调用；
/// 正常部署只在进程启动时解析一次）。
pub fn reset_auto_cache() {
    *AUTO_CACHE.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// 解析一个语言值（`zh-CN` / `en_US.UTF-8` / `C` …）；不认识的返回 `None`。
pub fn parse_lang(raw: &str) -> Option<Lang> {
    let s = raw.trim().to_ascii_lowercase();
    if s.is_empty() {
        return None;
    }
    let head = s.split(['_', '-', '.', '@']).next().unwrap_or("");
    match head {
        "zh" | "cn" | "chinese" => Some(Lang::Zh),
        "en" | "english" => Some(Lang::En),
        _ => None,
    }
}

/// 环境变量 → 语言（含手动覆盖 `OCSMLANG`；见模块头优先级）。默认 `zh`。
pub fn resolve_env_lang() -> Lang {
    for var in [LANG_ENV, LANG_ENV_ALIAS, "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            if let Some(l) = parse_lang(&v) {
                return l;
            }
        }
    }
    Lang::Zh
}

/// 语言来源诊断（GUI / 日志）：返回 `(变量名, 原值)`；全未命中 = `None`。
pub fn env_lang_source() -> Option<(&'static str, String)> {
    for var in [LANG_ENV, LANG_ENV_ALIAS, "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            if parse_lang(&v).is_some() {
                return Some((var, v));
            }
        }
    }
    None
}

fn note_missing(key: &str) {
    let mut set = MISSING.lock().unwrap_or_else(|e| e.into_inner());
    if set.len() < MISSING_LIMIT && !set.iter().any(|k| k == key) {
        set.push(key.to_string());
    }
}

/// 渲染一条词条（空串 → 回落 `zh`；`zh` 也空 → `None`）。单测直接喂构造词条。
fn render_msg(m: &Msg, lang: Lang) -> Option<&'static str> {
    let pick = match lang {
        Lang::Zh => m.zh,
        Lang::En => m.en,
    };
    if !pick.is_empty() {
        Some(pick)
    } else if !m.zh.is_empty() {
        Some(m.zh)
    } else {
        None
    }
}

/// 取词（按当前语言；返回 owned `String`，避免把 catalog 生命周期漏到调用方）。
pub fn t(key: &str) -> String {
    t_lang(lang(), key)
}

/// 取词（返回 `&'static str`）：catalog 命中时零分配，供 match 分支等需要长生命周期的借用。
/// 未命中仍记 `missing_keys` 并回落 key 本身（泄漏一份，量级 = 缺 key 种类数，可接受）。
pub fn t_static(key: &str) -> &'static str {
    match lookup(key) {
        Some(m) => match lang() {
            Lang::Zh => {
                if m.zh.is_empty() {
                    m.en
                } else {
                    m.zh
                }
            }
            Lang::En => {
                if m.en.is_empty() {
                    m.zh
                } else {
                    m.en
                }
            }
        },
        None => {
            note_missing(key);
            Box::leak(key.to_string().into_boxed_str())
        }
    }
}

/// 取词（指定语言；测试 / GUI 预渲染用）。
pub fn t_lang(lang: Lang, key: &str) -> String {
    match lookup(key) {
        Some(m) => match render_msg(m, lang) {
            Some(s) => s.to_string(),
            // catalog 里 zh 也是空（不该发生；catalog_problems 会点名）→ 可见 key。
            None => {
                note_missing(key);
                key.to_string()
            }
        },
        None => {
            note_missing(key);
            // 保守默认：不返回空串 / 不 panic；key 本身可见，便于一眼定位。
            key.to_string()
        }
    }
}

/// 卡类型显示名（报错前缀）：中文名 → 当前语言的显示名（catalog `card.name.*`）。
/// 只用于错误/回执前缀，不影响 GUI 卡类型下拉（那张表另有 `label`）。
pub fn card_display(name: &str) -> String {
    let key = match name {
        "花键参数表" => "card.name.spline",
        "齿轮参数表" => "card.name.gear",
        "ANSI 花键参数表" => "card.name.ansi",
        "NF 内花键参数表" => "card.name.nf",
        "NF 外花键参数表" => "card.name.nf_ext",
        "DIN 花键参数表" => "card.name.din",
        _ => return name.to_string(),
    };
    t(key)
}

/// 带参数插值：`{name}` → `args` 里的值（未给到的占位符原样保留）。
pub fn t_fmt(key: &str, args: &[(&str, &str)]) -> String {
    t_fmt_lang(lang(), key, args)
}

/// 带参数插值（指定语言）。
pub fn t_fmt_lang(lang: Lang, key: &str, args: &[(&str, &str)]) -> String {
    let mut out = t_lang(lang, key);
    for (k, v) in args {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// 按语言列出全 catalog（`/api/i18n` 用；目标语言为空时回落 `zh`）。
pub fn catalog_pairs(lang: Lang) -> Vec<(&'static str, String)> {
    CATALOG
        .iter()
        .map(|m| (m.key, render_msg(m, lang).unwrap_or(m.key).to_string()))
        .collect()
}

/// 运行期缺失 key 名单（去重点名；有上限）。测试 / 联调用。
pub fn missing_keys() -> Vec<String> {
    MISSING.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// 清空缺失记录。
pub fn clear_missing_keys() {
    MISSING.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

/// catalog 静态问题清单：空 key / 重复 key / 任一语为空 / `{占位符}` 两语不一致。
/// 空 = 健康。
pub fn catalog_problems() -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for m in CATALOG {
        if m.key.trim().is_empty() {
            problems.push("空 key".to_string());
        }
        if seen.contains(&m.key) {
            problems.push(format!("重复 key：{}", m.key));
        } else {
            seen.push(m.key);
        }
        if m.zh.trim().is_empty() {
            problems.push(format!("{}：zh 为空", m.key));
        }
        if m.en.trim().is_empty() {
            problems.push(format!("{}：en 为空", m.key));
        }
        if placeholders(m.zh) != placeholders(m.en) {
            problems.push(format!("{}：占位符 zh/en 不一致", m.key));
        }
    }
    problems
}

/// 提取 `{name}` 占位符名（排序去重）。
fn placeholders(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest = s;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else { break };
        let name = &after[..end];
        if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            out.push(name.to_string());
        }
        rest = &after[end + 1..];
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 用例之间语言/环境是进程级全局 —— 与仓内各模块共用同一把测试锁。
    fn lock() -> std::sync::MutexGuard<'static, ()> {
        crate::global_state_test_lock()
    }

    #[test]
    fn catalog_is_complete_and_has_no_duplicate_keys() {
        let problems = catalog_problems();
        assert!(problems.is_empty(), "catalog 静态问题：{problems:?}");
        assert!(!CATALOG.is_empty(), "catalog 不该为空（至少竖切域词条）");
    }

    #[test]
    fn placeholder_extraction_handles_nested_and_bad_forms() {
        assert_eq!(placeholders("a {x} b {y} {x}"), vec!["x", "y"]);
        assert_eq!(placeholders("无占位符"), Vec::<String>::new());
        assert_eq!(placeholders("残缺 {x"), Vec::<String>::new());
        assert_eq!(placeholders("{}"), Vec::<String>::new());
    }

    #[test]
    fn t_switches_between_zh_and_en() {
        let _g = lock();
        set_lang(Lang::Zh);
        let zh = t("cmd.tf.usage");
        assert!(zh.starts_with("用法：TF <名称>"), "中文取词：{zh}");
        set_lang(Lang::En);
        let en = t("cmd.tf.usage");
        assert!(en.starts_with("Usage: TF <name>"), "英文取词：{en}");
        assert_ne!(zh, en);
        set_lang_auto();
    }

    #[test]
    fn t_fmt_interpolates_in_both_languages() {
        let _g = lock();
        let arg = "extra";
        let usage = "U";
        let zh = t_fmt_lang(
            Lang::Zh,
            "cmd.tf.err.unknown_param",
            &[("arg", arg), ("usage", usage)],
        );
        assert_eq!(zh, "认不出的参数「extra」。U");
        let en = t_fmt_lang(
            Lang::En,
            "cmd.tf.err.unknown_param",
            &[("arg", arg), ("usage", usage)],
        );
        assert_eq!(en, "Unrecognized argument \"extra\". U");
    }

    #[test]
    fn missing_en_falls_back_to_zh_and_is_named() {
        let _g = lock();
        // 合成词条：en 空 → 回落 zh（真实 catalog 由完整性测试保证不为空）。
        let m = Msg::new("synthetic.only.zh", "只有中文", "");
        assert_eq!(render_msg(&m, Lang::En), Some("只有中文"));
        assert_eq!(render_msg(&m, Lang::Zh), Some("只有中文"));
        // zh / en 都空 → None（调用方转成「可见 key」，不是空串）。
        let empty = Msg::new("synthetic.empty", "", "");
        assert_eq!(render_msg(&empty, Lang::En), None);

        // catalog 里根本没有的 key：返回 key 本身 + 运行期点名，绝不空串。
        clear_missing_keys();
        let got = t_lang(Lang::En, "no.such.key.at.all");
        assert_eq!(got, "no.such.key.at.all");
        assert!(missing_keys().iter().any(|k| k == "no.such.key.at.all"));
        clear_missing_keys();
    }

    #[test]
    fn env_resolution_prefers_ocsmlang_then_locale() {
        let _g = lock();
        // 环境变量是进程级全局：改前存旧值，测完恢复。
        let vars = [LANG_ENV, LANG_ENV_ALIAS, "LC_ALL", "LC_MESSAGES", "LANG"];
        let saved: Vec<(&str, Option<String>)> =
            vars.iter().map(|v| (*v, std::env::var(v).ok())).collect();
        for v in vars {
            std::env::remove_var(v);
        }
        assert_eq!(resolve_env_lang(), Lang::Zh, "全无环境 → 保守默认 zh");
        std::env::set_var("LANG", "en_US.UTF-8");
        assert_eq!(resolve_env_lang(), Lang::En, "跟随 LANG");
        std::env::set_var("LC_ALL", "zh_CN.UTF-8");
        assert_eq!(resolve_env_lang(), Lang::Zh, "LC_ALL 优先于 LANG");
        std::env::set_var(LANG_ENV, "en");
        assert_eq!(resolve_env_lang(), Lang::En, "OCSMLANG 是手动覆盖（最高）");
        std::env::set_var(LANG_ENV, "de_DE");
        assert_eq!(
            resolve_env_lang(),
            Lang::Zh,
            "不认识的值跳过，看下一来源（LC_ALL=zh 生效）"
        );
        std::env::set_var("LC_ALL", "de_DE");
        assert_eq!(resolve_env_lang(), Lang::En, "LC_ALL 也不认识 → 落到 LANG=en");
        assert_eq!(parse_lang("zh-CN"), Some(Lang::Zh));
        assert_eq!(parse_lang("EN_us"), Some(Lang::En));
        assert_eq!(parse_lang("C"), None);
        assert_eq!(parse_lang("  "), None);
        for (k, v) in saved {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }
}
