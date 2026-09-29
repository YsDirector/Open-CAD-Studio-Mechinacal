//! 序号标注（`BALLOON`，命令别名 `XH`）——**引线标注的变体**。
//!
//! 一个"组" = **1 条指引线 + 1 个圆点 + N 条横线 + N 个序号**；横线之间按扩展方向连接：
//! * **横向扩展**（`dir=H`）：横线共线并排，相邻之间画 **V 形折线**（断裂续接符号）；
//! * **纵向扩展**（`dir=V`）：横线上下叠放，在**指引线那一端**用一条竖线连接。
//! 每组只有一条指引线（共用指针），序号各自写在自己横线上方居中。
//!
//! **序号永远水平书写**（用户定案：不要 "竖肩线 + 序号旋转 90°" 那种样式）——
//! 所以第二段（肩线）必须画成**水平**；画成竖直直接报错（叫用户改画成水平）。
//!
//! 比例全部来自用户参考图实测（`~/桌面/OCSM/OCSMDIMGULIDE/序号标注.png`，
//! 423×300，29 个连通单元逐像素量取；基准 = 字高 h = 3.5 mm，下列数值以 h 的倍数记）：
//!
//! | 量 | 实测 | 取用（参考单位，h=3.5） |
//! | --- | --- | --- |
//! | 横线长 | 单字符 1.43h、双字符 1.71h | 文字宽 + 2×0.5h |
//! | 数字基线距横线 | 0.29h | 0.3h |
//! | 横向相邻横线间隙 | 0.48h | 0.5h |
//! | V 形折线深度 | 0.38h | 0.38h |
//! | 纵向横线节距 | 1.57h | 1.6h |
//! | 圆点直径 | 0.31h | 0.33h（八边形实心） |
//!
//! 编号规则（用户 2026-09-15 定案）：默认 = **上一个序号末尾数字 +1**，
//! 前缀原样保留（`A1`→`A2`、`A09`→`A10`、`7`→`8`）；**结尾必须是数字**；
//! 明细表按序号升序排列（`item_no_key` 给排序键）。
//!
//! 本模块是**纯函数**：不读文档、不建块、不发插件请求（与 `build_leader_parts` 同规矩，
//! 交互路径与 D2G 转化共用；可单测）。

use ocs_plugin_api::host::acadrust::entities::{AttachmentPoint, Line, MText, Solid};
use ocs_plugin_api::host::acadrust::types::{Color, Vector3};
use ocs_plugin_api::host::acadrust::EntityType as E;

/// 参考字高（与其它 OCSM 标注同款：3.5 mm）。
pub(crate) const H_REF: f64 = 3.5;
/// 横线两侧余量：0.5h。
const MARGIN_REF: f64 = 1.75;
/// 横向扩展：相邻横线间隙 0.5h。
const GAP_REF: f64 = 1.75;
/// 横向扩展：V 形折线深度 0.38h。
const V_DEPTH_REF: f64 = 1.33;
/// 纵向扩展：横线节距 1.6h。
const PITCH_REF: f64 = 5.6;
/// 序号基线距横线 0.3h。
const TEXT_UP_REF: f64 = 1.05;
/// 指针圆点半径（Ø0.33h / 2）。
const DOT_R_REF: f64 = 0.58;
/// 序号/横线所在图层（与引线/焊接同层）。
const LAYER: &str = "8符号标注层";

/// 序号扩展方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BalloonDir {
    /// 横向扩展：横线共线并排，V 形折线连接。
    #[default]
    Row,
    /// 纵向扩展：横线上下叠放，竖线连接（在指引线那一端）。
    Col,
}

impl BalloonDir {
    pub fn as_str(self) -> &'static str {
        match self {
            BalloonDir::Row => "H",
            BalloonDir::Col => "V",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "H" | "LEVEL" | "ROW" | "横" | "横向" => Some(BalloonDir::Row),
            "V" | "VERT" | "COL" | "纵" | "纵向" => Some(BalloonDir::Col),
            _ => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            BalloonDir::Row => "横向",
            BalloonDir::Col => "纵向",
        }
    }
}

// ── 编号工具（纯函数）─────────────────────────────────────────────────────

/// 拆序号：**末尾连续数字** = 编号，前面 = 前缀。结尾不是数字 → `None`（`"A"`、`""`）。
pub(crate) fn split_item_no(s: &str) -> Option<(&str, &str)> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let digits_start = t
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    if digits_start == t.len() {
        return None; // 结尾没有数字
    }
    Some((&t[..digits_start], &t[digits_start..]))
}

/// 下一个序号：末尾数字 +1，前缀与**位宽**保留（`A09`→`A10`、`9`→`10`、`A9`→`A10`）。
/// 结尾不是数字 → `None`（由调用方决定手填/报错）。
pub(crate) fn item_no_next(s: &str) -> Option<String> {
    let (prefix, digits) = split_item_no(s)?;
    let v: u64 = digits.parse().ok()?;
    let next = v.checked_add(1)?;
    let width = if digits.len() > 1 && digits.starts_with('0') {
        digits.len()
    } else {
        0
    };
    let shown = if width > 0 {
        format!("{:0width$}", next, width = width)
    } else {
        next.to_string()
    };
    Some(format!("{prefix}{shown}"))
}

/// 序号排序键：`(前缀, 数值)`；数值小的在前，同前缀按数值（`A2` < `A10`）。
/// 结尾不是数字的序号排最后（按字面前缀）。
pub(crate) fn item_no_key(s: &str) -> (String, u64, u8) {
    match split_item_no(s) {
        Some((prefix, digits)) => (
            prefix.to_string(),
            digits.parse::<u64>().unwrap_or(0),
            0,
        ),
        None => (s.trim().to_string(), 0, 1),
    }
}

/// 一组序号按升序排（明细表行序 / 冲突后移都用它）。
pub(crate) fn sort_item_nos(items: &mut [String]) {
    items.sort_by_key(|s| item_no_key(s));
}

/// 解析 URL 的 `items=1,2,3` 列表（去空、去重保序）。
pub(crate) fn parse_items(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for seg in s.split([',', ';']) {
        let v = seg.trim();
        if v.is_empty() {
            continue;
        }
        if !out.iter().any(|x| x == v) {
            out.push(v.to_string());
        }
    }
    out
}

/// 从一组已有序号里取"下一个"：先按升序取最大，再看**前缀相同**的项里取最大（`A1`、`B1`、`2`
/// 混排时，下一个跟"排序最靠后"的那个同前缀递增）。
pub(crate) fn next_after_all(existing: &[String], fallback: &str) -> String {
    let mut sorted: Vec<String> = existing.to_vec();
    sort_item_nos(&mut sorted);
    let last = sorted.last().map(|s| s.as_str()).unwrap_or(fallback);
    item_no_next(last).unwrap_or_else(|| "1".to_string())
}

// ── 几何 ────────────────────────────────────────────────────────────────

/// 文字参考宽度（与引线标注同一套：中日韩 2.45、拉丁/数字 1.45，参考单位）。
pub(crate) fn text_w_ref(t: &str) -> f64 {
    t.trim()
        .chars()
        .map(|c| if (c as u32) >= 0x2E80 { 2.45 } else { 1.45 })
        .sum()
}

/// 横线长度（参考单位）= 文字宽 + 2×0.5h。
pub(crate) fn shelf_len_ref(t: &str) -> f64 {
    text_w_ref(t) + 2.0 * MARGIN_REF
}

/// 序号标注几何（块内实体；坐标系 = **块局部**，原点 = 拐点 P0）。
pub(crate) struct BalloonParts {
    /// 块内实体（层与颜色已设）。
    pub members: Vec<E>,
    /// 图幅倍率（`frame_scale_at`）。
    pub scale: f64,
    /// 扩展方向。
    pub dir: BalloonDir,
    /// 序号组总跨度（世界单位；横向 = 横线总长，纵向 = 总高）。
    pub span: f64,
    /// 肩线是否水平（序号标注**恒为 true**：竖肩线已在入口报错）。
    pub horizontal: bool,
    /// 各条横线长度（世界单位）。
    pub shelf_lens: Vec<f64>,
}

/// 生成序号标注几何。
///
/// * `p_tip` 引导顶点0 = 指针点（画**圆点**）；
/// * `p0` 引导顶点1 = 拐点（块原点、横线的**指引线那一端**）；
/// * `p_end` 引导顶点2 = 肩线末端（只用来定方向：横线自拐点朝它延伸）；
/// * `items` 序号列表，**按"从指引线向外/向上"排列**（通常升序，如 `[5,6]`：5 贴指引线、6 在上）。
pub(crate) fn build_balloon_parts(
    p_tip: [f64; 3],
    p0: [f64; 3],
    p_end: [f64; 3],
    s: f64,
    items: &[String],
    dir: BalloonDir,
) -> Result<BalloonParts, String> {
    if items.is_empty() {
        return Err(crate::i18n::t("cmd.balloon.err.need_item"));
    }
    if items.iter().any(|t| t.trim().is_empty()) {
        return Err(crate::i18n::t("cmd.balloon.err.blank_item"));
    }
    if s <= 0.0 || !s.is_finite() {
        return Err(crate::i18n::t("cmd.balloon.err.bad_scale"));
    }
    // ── 肩线必须水平（序号永远水平写；不要竖肩线 + 序号旋转那种样式）──
    let bdx = p_end[0] - p0[0];
    let bdy = p_end[1] - p0[1];
    if bdy.abs() > bdx.abs() {
        return Err(crate::i18n::t("cmd.balloon.err.not_horizontal"));
    }
    if bdx.abs() <= 1e-9 {
        return Err(crate::i18n::t("cmd.balloon.err.zero_segment"));
    }
    // 横线自拐点朝 p_end 延伸（水平）：t 轴 = ±x，n_up = 内容上方 = +y。
    let t_sign = if bdx >= 0.0 { 1.0 } else { -1.0 };
    let d_c: (f64, f64) = (t_sign, 0.0);
    let n_up: (f64, f64) = (0.0, 1.0);
    let text_rot = 0.0;

    // 局部 → **块局部坐标**（原点 = 拐点 p0；INSERT 就落在 p0，所以块内一切
    // 都必须是不含 p0 的相对坐标）——t = 沿横线（+ 朝肩线末端），v = 内容上方。
    let tp = |t: f64, v: f64| -> (f64, f64) { (t * d_c.0 + v * n_up.0, t * d_c.1 + v * n_up.1) };
    // 参考单位 → 世界单位。
    let w = |x: f64| x * s;

    let shelf_lens: Vec<f64> = items
        .iter()
        .map(|t| w(shelf_len_ref(t)))
        .collect();
    let pitch = w(PITCH_REF);
    let gap = w(GAP_REF);
    let v_depth = w(V_DEPTH_REF);
    let text_up = w(TEXT_UP_REF);
    let dot_r = w(DOT_R_REF);
    let n = items.len();

    let mut members: Vec<E> = Vec::new();

    // ── 指针圆点（八边形实心：两枚 SOLID——宿主不画填充圆；Ø0.33h）──
    //    块局部坐标：与指引线一致地减掉拐点。
    {
        let (cx, cy) = (p_tip[0] - p0[0], p_tip[1] - p0[1]);
        let mut pts: Vec<(f64, f64)> = Vec::new();
        for k in 0..8 {
            let a = std::f64::consts::FRAC_PI_4 * (k as f64 + 0.5);
            pts.push((cx + dot_r * a.cos(), cy + dot_r * a.sin()));
        }
        for q in 0..2 {
            let p = |i: usize| -> Vector3 {
                let (x, y) = pts[(q * 4 + i) % 8];
                Vector3::new(x, y, 0.0)
            };
            let solid = Solid::new(p(0), p(1), p(2), p(3));
            let mut e = E::Solid(solid);
            set_layer(&mut e, LAYER);
            e.common_mut().color = Color::from_index(4);
            members.push(e);
        }
    }

    // ── 指引线：指针点 → 拐点（细实线）──
    members.push(line(
        (p_tip[0] - p0[0], p_tip[1] - p0[1]),
        (0.0, 0.0),
        4,
    ));

    // ── 横线 + 连接 + 序号 ──
    let mut span = 0.0f64;
    match dir {
        // 横向扩展：横线共线并排，V 形折线连接（自指引线那端朝肩线末端排）。
        BalloonDir::Row => {
            let mut cursor = 0.0f64;
            for (i, (item, len)) in items.iter().zip(shelf_lens.iter()).enumerate() {
                members.push(line(tp(cursor, 0.0), tp(cursor + len, 0.0), 4));
                // 序号：横线中点上方 0.3h（BottomCenter 锚定）
                members.push(item_text(
                    item,
                    tp(cursor + len / 2.0, text_up),
                    text_rot,
                    s,
                ));
                if i + 1 < n {
                    // V 形折线：横线末端 → 谷底（−0.38h）→ 下一条横线起点
                    let a = tp(cursor + len, 0.0);
                    let b = tp(cursor + len + gap / 2.0, -v_depth);
                    let c = tp(cursor + len + gap, 0.0);
                    members.push(line(a, b, 4));
                    members.push(line(b, c, 4));
                    cursor += len + gap;
                } else {
                    cursor += *len;
                }
            }
            span = cursor;
        }
        // 纵向扩展：横线上下叠放（节距 1.6h），指引线那一端竖线连接。
        BalloonDir::Col => {
            for (i, (item, len)) in items.iter().zip(shelf_lens.iter()).enumerate() {
                let v = pitch * i as f64;
                members.push(line(tp(0.0, v), tp(*len, v), 4));
                members.push(item_text(
                    item,
                    tp(len / 2.0, v + text_up),
                    text_rot,
                    s,
                ));
            }
            // 竖线连接（t = 0 端 = 拐点/指引线一侧）
            let h = pitch * (n as f64 - 1.0);
            members.push(line(tp(0.0, 0.0), tp(0.0, h), 4));
            span = h;
        }
    }

    Ok(BalloonParts {
        members,
        scale: s,
        dir,
        span,
        horizontal: true,
        shelf_lens,
    })
}

/// 块局部线成员（层/颜色已设）。
fn line(a: (f64, f64), b: (f64, f64), color: i16) -> E {
    let mut e = E::Line(Line {
        common: Default::default(),
        start: Vector3::new(a.0, a.1, 0.0),
        end: Vector3::new(b.0, b.1, 0.0),
        thickness: 0.0,
        normal: Vector3::new(0.0, 0.0, 1.0),
    });
    set_layer(&mut e, LAYER);
    e.common_mut().color = Color::from_index(color);
    e
}

/// 序号文字（块内 MTEXT，绿色 3；与引线/焊接同套：`OCSM_GB` 样式、字高 3.5×倍率）。
fn item_text(value: &str, at: (f64, f64), rot: f64, s: f64) -> E {
    let mut m = MText::new();
    m.value = value.trim().to_string();
    m.insertion_point = Vector3::new(at.0, at.1, 0.0);
    m.height = H_REF * s;
    m.rotation = rot;
    m.style = "OCSM_GB".into();
    // 序号短：行宽给足，避免被按 10 单位折行（同引线）。
    m.rectangle_width = (H_REF * s * 10.0).max(10.0 * s);
    m.attachment_point = AttachmentPoint::BottomCenter;
    let mut e = E::MText(m);
    set_layer(&mut e, LAYER);
    e.common_mut().color = Color::from_index(3);
    e
}

fn set_layer(e: &mut E, layer: &str) {
    e.common_mut().layer = layer.to_string();
}

// ── 测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const TIP: [f64; 3] = [0.0, 0.0, 0.0];
    const P0: [f64; 3] = [40.0, 40.0, 0.0];
    const P_END: [f64; 3] = [70.0, 40.0, 0.0];

    fn items(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    // ── 编号 ──

    #[test]
    fn split_item_no_needs_trailing_digits() {
        assert_eq!(split_item_no("A12"), Some(("A", "12")));
        assert_eq!(split_item_no("12"), Some(("", "12")));
        assert_eq!(split_item_no(" 7 "), Some(("", "7")));
        assert_eq!(split_item_no("A"), None);
        assert_eq!(split_item_no("A1B"), None);
        assert_eq!(split_item_no(""), None);
    }

    #[test]
    fn item_no_next_keeps_prefix_and_width() {
        assert_eq!(item_no_next("7").unwrap(), "8");
        assert_eq!(item_no_next("A1").unwrap(), "A2");
        assert_eq!(item_no_next("A09").unwrap(), "A10");
        assert_eq!(item_no_next("A9").unwrap(), "A10");
        assert_eq!(item_no_next("9").unwrap(), "10");
        assert!(item_no_next("A").is_none());
    }

    #[test]
    fn item_no_sort_is_numeric_inside_prefix() {
        let mut v = items(&["A10", "A2", "B1", "1", "2"]);
        sort_item_nos(&mut v);
        assert_eq!(v, items(&["1", "2", "A2", "A10", "B1"]));
        // 前缀为空（纯数字）排最前；结尾非数字的排最后
        let mut w = items(&["5", "A1", "备注"]);
        sort_item_nos(&mut w);
        assert_eq!(w, items(&["5", "A1", "备注"]));
    }

    #[test]
    fn next_after_all_follows_the_last_number() {
        assert_eq!(next_after_all(&items(&["1", "2", "3"]), "1"), "4");
        assert_eq!(next_after_all(&items(&["A1", "A2"]), "1"), "A3");
        // 空表 → 用兜底后缀的 +1
        assert_eq!(next_after_all(&[], "0"), "1");
        // 带前缀与纯数字混排：跟排序最靠后的同前缀递增
        assert_eq!(next_after_all(&items(&["1", "2", "A1"]), "1"), "A2");
    }

    #[test]
    fn parse_items_dedups_and_trims() {
        assert_eq!(parse_items("1, 2 ,2,,3"), items(&["1", "2", "3"]));
        assert_eq!(parse_items(""), Vec::<String>::new());
    }

    // ── 几何 ──

    #[test]
    fn single_item_is_leader_and_shelf_and_dot() {
        let parts = build_balloon_parts(TIP, P0, P_END, 1.0, &items(&["1"]), BalloonDir::Row)
            .unwrap();
        assert_eq!(parts.dir, BalloonDir::Row);
        assert!(parts.horizontal);
        // 圆点 2 枚 SOLID + 指引线 + 1 条横线 + 1 个序号
        assert_eq!(parts.members.len(), 5, "{:?}", kind_list(&parts.members));
        assert_eq!(parts.shelf_lens.len(), 1);
        // 横线长 = 文字宽(1.45) + 3.5 = 4.95；即实测"单字符 ≈1.43h"
        assert!((parts.shelf_lens[0] - 4.95).abs() < 1e-9, "{}", parts.shelf_lens[0]);
        assert!((parts.span - 4.95).abs() < 1e-9);
    }

    #[test]
    fn row_expansion_adds_v_joint_and_keeps_one_leader() {
        let one = build_balloon_parts(TIP, P0, P_END, 1.0, &items(&["2"]), BalloonDir::Row).unwrap();
        let two = build_balloon_parts(TIP, P0, P_END, 1.0, &items(&["2", "3"]), BalloonDir::Row)
            .unwrap();
        // 多一条横线 + 一个序号 + V 的两段线（= +4 成员）
        assert_eq!(two.members.len(), one.members.len() + 4, "{:?}", kind_list(&two.members));
        // 指引线仍只有一条：统计 Line 数量 = 指引线1 + 横线2 + V 2
        assert_eq!(count_kind(&two.members, "Line"), 5);
        assert_eq!(count_kind(&two.members, "Solid"), 2); // 圆点仍是一枚
        assert_eq!(count_kind(&two.members, "MText"), 2); // 两个序号
        // 跨度 = L(2) + gap + L(3)
        let expect = 4.95 + 1.75 + 4.95;
        assert!((two.span - expect).abs() < 1e-9, "{}", two.span);
    }

    #[test]
    fn col_expansion_stacks_and_links_at_the_leader_end() {
        let parts = build_balloon_parts(TIP, P0, P_END, 1.0, &items(&["5", "6"]), BalloonDir::Col)
            .unwrap();
        assert_eq!(parts.dir, BalloonDir::Col);
        assert!((parts.span - 5.6).abs() < 1e-9, "{}", parts.span);
        assert_eq!(count_kind(&parts.members, "Line"), 4); // 指引线 + 2 横线 + 竖线
        assert_eq!(count_kind(&parts.members, "MText"), 2);
        // 竖线连接在 t=0 端（拐点/指引线侧）：x 都等于 p0.x（横线沿 +x 时）
        let mut vert = 0;
        for m in &parts.members {
            if let E::Line(l) = m {
                if (l.start.x - l.end.x).abs() < 1e-9 && (l.start.y - l.end.y).abs() > 1e-9 {
                    vert += 1;
                    // 块局部坐标：拐点 = 原点 → 竖线应在 x = 0
                    assert!(l.start.x.abs() < 1e-9, "竖线应在拐点（x=0）上");
                }
            }
        }
        assert_eq!(vert, 1);
    }

    #[test]
    fn shelf_extends_toward_the_drawn_end() {
        // 肩线末端在拐点左边 → 组也朝左长（t_sign = −1）
        let left = [10.0, 40.0, 0.0];
        let a = build_balloon_parts(TIP, P0, P_END, 1.0, &items(&["1", "2"]), BalloonDir::Row).unwrap();
        let b = build_balloon_parts(TIP, P0, left, 1.0, &items(&["1", "2"]), BalloonDir::Row).unwrap();
        // 块局部坐标：拐点 = 原点（0,0）
        let (lo_r, hi_r) = shelf_x_range(&a);
        let (lo_l, hi_l) = shelf_x_range(&b);
        assert!(hi_r > 1.0, "朝右：横线应伸到拐点右边 {hi_r}");
        assert!(lo_r.abs() < 1e-9, "朝右：最小 x 应贴拐点 {lo_r}");
        assert!(lo_l < -1.0, "朝左：横线应伸到拐点左边 {lo_l}");
        assert!(hi_l.abs() < 1e-9, "朝左：最大 x 应贴拐点 {hi_l}");
        // 镜像对称：跨度相同
        assert!((a.span - b.span).abs() < 1e-9);
    }

    #[test]
    fn vertical_shoulder_is_rejected_by_design() {
        // 用户定案：不要"竖肩线 + 序号旋转 90°"那种样式 → 直接报错，叫改画水平。
        let p_end = [40.0, 70.0, 0.0];
        let e = match build_balloon_parts(TIP, P0, p_end, 1.0, &items(&["1"]), BalloonDir::Row) {
            Ok(_) => panic!("竖肩线应报错"),
            Err(e) => e,
        };
        assert!(e.contains("必须水平"), "{e}");
    }

    #[test]
    fn all_numbers_are_horizontal() {
        let parts = build_balloon_parts(TIP, P0, P_END, 1.0, &items(&["10", "11"]), BalloonDir::Col)
            .unwrap();
        for m in &parts.members {
            if let E::MText(t) = m {
                assert_eq!(t.rotation, 0.0, "序号必须不旋转");
            }
        }
    }

    #[test]
    fn scale_multiplies_everything_and_errors_are_clear() {
        let p1 = build_balloon_parts(TIP, P0, P_END, 1.0, &items(&["1", "2"]), BalloonDir::Col).unwrap();
        let p2 = build_balloon_parts(TIP, P0, P_END, 2.0, &items(&["1", "2"]), BalloonDir::Col).unwrap();
        assert!((p2.span - p1.span * 2.0).abs() < 1e-9);
        assert!(build_balloon_parts(TIP, P0, P_END, 1.0, &[], BalloonDir::Row).is_err());
        assert!(build_balloon_parts(TIP, P0, P_END, 1.0, &items(&[" "]), BalloonDir::Row).is_err());
        // 肩线段零长（p_end == p0）
        assert!(build_balloon_parts(TIP, P0, P0, 1.0, &items(&["1"]), BalloonDir::Row).is_err());
        assert!(BalloonDir::from_str("横向").unwrap() == BalloonDir::Row);
        assert!(BalloonDir::from_str("V").unwrap() == BalloonDir::Col);
        assert!(BalloonDir::from_str("x").is_none());
    }

    #[test]
    fn members_are_on_symbol_layer_with_expected_colors() {
        let parts = build_balloon_parts(TIP, P0, P_END, 1.0, &items(&["9", "10"]), BalloonDir::Row)
            .unwrap();
        for m in &parts.members {
            assert_eq!(m.common().layer, "8符号标注层");
            let c = m.common().color;
            if matches!(m, E::MText(_)) {
                assert!(matches!(c, Color::Index(3)), "序号文字应为绿色：{c:?}");
            } else {
                assert!(matches!(c, Color::Index(4)), "线与圆点应为青色：{c:?}");
            }
        }
    }

    // ── 验收图（人工/视觉检查画法）────────────────────────────────────────────

    /// 出"序号标注验收图"：用**与命令完全相同的生成代码**建块 + INSERT 并写盘，
    /// 供在 OCS 里打开与参考图（`~/桌面/OCSM/OCSMDIMGULIDE/序号标注.png`）逐形比对。
    ///
    /// `cargo test -p ocs_ocsm --lib dump_balloon_acceptance -- --ignored --nocapture`
    #[test]
    #[ignore = "出验收图（手动跑）"]
    fn dump_balloon_acceptance() {
        use crate::partgen::acceptance_dump::{add_block, add_ocsm_layers};
        use ocs_plugin_api::host::acadrust;
        use ocs_plugin_api::host::acadrust::entities::Insert;
        use ocs_plugin_api::host::acadrust::io::dwg::DwgWriter;
        use ocs_plugin_api::host::acadrust::io::dxf::DxfWriter;

        let mut doc = acadrust::CadDocument::new();
        add_ocsm_layers(&mut doc);
        // 四个形态：单序号 / 横向扩展（V 折线）/ 纵向扩展（竖线）/ 竖肩线（序号旋转）。
        let cases: [(&str, Vec<&str>, BalloonDir, [f64; 3], [f64; 3], [f64; 3]); 5] = [
            ("单个 1", vec!["1"], BalloonDir::Row, [0.0, 0.0, 0.0], [25.0, 30.0, 0.0], [45.0, 30.0, 0.0]),
            ("横向 2-3", vec!["2", "3"], BalloonDir::Row, [80.0, 0.0, 0.0], [105.0, 30.0, 0.0], [150.0, 30.0, 0.0]),
            ("纵向 5-6", vec!["5", "6"], BalloonDir::Col, [165.0, 0.0, 0.0], [190.0, 30.0, 0.0], [215.0, 30.0, 0.0]),
            ("前缀 A1-A2-A3", vec!["A1", "A2", "A3"], BalloonDir::Row, [240.0, 0.0, 0.0], [265.0, 30.0, 0.0], [325.0, 30.0, 0.0]),
            ("纵向三个 7-8-9", vec!["7", "8", "9"], BalloonDir::Col, [350.0, 0.0, 0.0], [375.0, 25.0, 0.0], [405.0, 25.0, 0.0]),
        ];
        for (i, (name, items, dir, tip, p0, pe)) in cases.iter().enumerate() {
            let items: Vec<String> = items.iter().map(|s| s.to_string()).collect();
            let parts = build_balloon_parts(*tip, *p0, *pe, 1.0, &items, *dir).expect("几何");
            let block = format!("OCSM_XH_DEMO_{}", i + 1);
            add_block(&mut doc, &block, parts.members.clone());
            // 引导几何示意（细灰线，10引导线层）
            let mut g1 = acadrust::entities::Line::from_coords(tip[0], tip[1], 0.0, p0[0], p0[1], 0.0);
            g1.common.layer = "10引导线层".into();
            doc.add_entity(E::Line(g1)).unwrap();
            let mut g2 = acadrust::entities::Line::from_coords(p0[0], p0[1], 0.0, pe[0], pe[1], 0.0);
            g2.common.layer = "10引导线层".into();
            doc.add_entity(E::Line(g2)).unwrap();
            let mut ins = Insert::new(&block, Vector3::new(p0[0], p0[1], 0.0));
            ins.common.layer = "8符号标注层".into();
            doc.add_entity(E::Insert(ins)).unwrap();
            println!("  {name}：块 {block}，成员 {} 条，跨度 {:.2}", parts.members.len(), parts.span);
        }
        let dir = std::path::Path::new("/home/ysdirector/桌面/OCSM/test");
        std::fs::create_dir_all(dir).unwrap();
        let dwg = dir.join("序号标注-验收.dwg");
        let dxf = dir.join("序号标注-验收.dxf");
        DwgWriter::write_to_file(&dwg, &doc).expect("写 DWG");
        DxfWriter::new(&doc).write_to_file(&dxf).expect("写 DXF");
        println!("已写出：\n  {}\n  {}", dwg.display(), dxf.display());
    }

    // 辅助
    fn kind_list(ms: &[E]) -> Vec<&'static str> {
        ms.iter()
            .map(|m| match m {
                E::Line(_) => "Line",
                E::Solid(_) => "Solid",
                E::MText(_) => "MText",
                _ => "other",
            })
            .collect()
    }
    fn count_kind(ms: &[E], k: &str) -> usize {
        kind_list(ms).iter().filter(|s| **s == k).count()
    }
    /// 全部横线（水平、非零长 Line）的 x 区间。
    fn shelf_x_range(p: &BalloonParts) -> (f64, f64) {
        let mut lo = f64::MAX;
        let mut hi = f64::MIN;
        for m in &p.members {
            if let E::Line(l) = m {
                if (l.start.y - l.end.y).abs() < 1e-9 && (l.start.x - l.end.x).abs() > 1e-9 {
                    lo = lo.min(l.start.x.min(l.end.x));
                    hi = hi.max(l.start.x.max(l.end.x));
                }
            }
        }
        (lo, hi)
    }
}
