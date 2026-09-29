//! 中心线（`OCSMCENTERLINE`，命令别名 **`ZX`**）。
//!
//! 两种输入（用户 2026-09-17 定案）：
//!
//! 1. **圆 / 圆弧**：十字交叉的两条直线段，两根的中心都在圆心上，
//!    **线长 = 直径 + n×6 mm**（n = 图框比例，见 [`crate::frame_scale_at`]）；
//! 2. **直线 + 第二根直线**：中心线落在**两根线的角平分线**上，
//!    **长度 = 第一根线在角平分线上的投影长度 + n×6**，
//!    **中点 = 第一根线中点在角平分线上的投影**。
//!
//! 产出的都是**普通 `LINE`**（不是匿名块）——可以直接修剪/延伸/改长度，
//! 落在 `3中心线层`（CENTER2）；`BYLAYER` 线宽/线型，图层不打印属性跟随图层。
//!
//! ## 角平分线怎么选（两根线有两根互相垂直的角平分线）
//!
//! 取**线段自己围出来的那个角**的平分线：交点到两根线各自**远端**（离交点较远的端点）的
//! 两条射线，其夹角平分线即所求。这样：
//! * 两根线**近平行**（槽、键槽、间隙两侧的线）→ 退化成"**两条线中间那条线**" ✓
//!   （钝角平分线会横穿过去，显然不是用户要的）；
//! * 两根线成**角**（折线、V 形、倒角）→ 就是那个角的内角平分线 ✓（不限于锐角）。
//!
//! 两根线**严格平行**时分母为 0，单独走"中线"分支：方向 = 线方向、
//! 位置在两根线正中（过"第一根线中点在第二根线上的垂足"的中点）。
//! 若两根线在别处相交（外延交点）也没关系——只用到**方向**，不用交点位置。

use ocs_plugin_api::host::acadrust;
use ocs_plugin_api::host::{
    CadDocument, CommandStep, EntityType, Handle, HostApi, InteractiveCommand,
};

use crate::{layer_defs, linetype_defs, to_arr, v3};

/// 中心线所在图层（`3中心线层`，CENTER2）。
pub(crate) const LAYER_CENTERLINE: &str = "3中心线层";

/// 超出量（mm）：`n × 6`。
pub(crate) const OVERHANG_MM: f64 = 6.0;

/// 判"两根线平行"的阈值：方向夹角正弦 < 1e-3（≈ 0.057°）。
/// 此时按"中线"处理——外延交点在 1000 倍线长之外，取哪个都无意义。
const PARALLEL_SIN: f64 = 1e-3;

/// 点（只用 XY，z 原样带过）。
pub(crate) type P = [f64; 3];

/// 线段（只关心两端）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Seg {
    pub a: P,
    pub b: P,
}

impl Seg {
    pub(crate) fn new(a: P, b: P) -> Self {
        Self { a, b }
    }

    pub(crate) fn mid(&self) -> P {
        [
            (self.a[0] + self.b[0]) / 2.0,
            (self.a[1] + self.b[1]) / 2.0,
            (self.a[2] + self.b[2]) / 2.0,
        ]
    }

    pub(crate) fn len(&self) -> f64 {
        let (dx, dy) = (self.b[0] - self.a[0], self.b[1] - self.a[1]);
        (dx * dx + dy * dy).sqrt()
    }

    /// 单位方向（XY）；零长线 → `None`。
    pub(crate) fn dir(&self) -> Option<[f64; 2]> {
        let (dx, dy) = (self.b[0] - self.a[0], self.b[1] - self.a[1]);
        let n = (dx * dx + dy * dy).sqrt();
        if n < 1e-12 {
            return None;
        }
        Some([dx / n, dy / n])
    }
}

/// 圆 / 圆弧的十字中心线：横、竖各一条，中心在圆心，长度 `2r + n×6`。
pub(crate) fn cross_for_circle(center: P, radius: f64, scale: f64) -> [Seg; 2] {
    let half = (2.0 * radius + OVERHANG_MM * scale) / 2.0;
    let [x, y, z] = center;
    [
        Seg::new([x - half, y, z], [x + half, y, z]),
        Seg::new([x, y - half, z], [x, y + half, z]),
    ]
}

/// 两根直线的角平分线中心线（见模块头：取"线段围出来的那个角"）。
///
/// * 长度 = `|第一根线在平分线方向上的投影| + n×6`；
/// * 中点 = 第一根线中点在该平分线上的**投影**；
/// * 任一根线零长 → `None`。
pub(crate) fn bisector(first: Seg, second: Seg, scale: f64) -> Option<Seg> {
    let d1 = first.dir()?;
    let d2 = second.dir()?;
    let (base, u) = bisector_base(first, second, d1, d2)?;
    // 投影长度（第一根线在 u 上的投影）与中点（第一根线中点投影到平分线上）。
    let proj_len = ((first.b[0] - first.a[0]) * u[0] + (first.b[1] - first.a[1]) * u[1]).abs();
    let m = first.mid();
    let t = (m[0] - base[0]) * u[0] + (m[1] - base[1]) * u[1];
    let mid = [base[0] + t * u[0], base[1] + t * u[1], m[2]];
    let half = (proj_len + OVERHANG_MM * scale) / 2.0;
    Some(Seg::new(
        [mid[0] - half * u[0], mid[1] - half * u[1], mid[2]],
        [mid[0] + half * u[0], mid[1] + half * u[1], mid[2]],
    ))
}

/// 平分线的一条基准点 + 单位方向（方向已经"取向"，覆盖两根线各自的两种端点顺序）。
fn bisector_base(first: Seg, second: Seg, d1: [f64; 2], d2: [f64; 2]) -> Option<(P, [f64; 2])> {
    let cross = d1[0] * d2[1] - d1[1] * d2[0];
    if cross.abs() < PARALLEL_SIN {
        // ── 平行（含反向平行）：两根线中间那条线 ──
        // 方向统一取第一根的；位置 = 第一根中点往第二根直线作垂足，取中点。
        let m1 = first.mid();
        let p2 = second.a;
        let e2 = [second.b[0] - second.a[0], second.b[1] - second.a[1]];
        let e2n = e2[0] * e2[0] + e2[1] * e2[1];
        let t = ((m1[0] - p2[0]) * e2[0] + (m1[1] - p2[1]) * e2[1]) / e2n;
        let foot = [p2[0] + t * e2[0], p2[1] + t * e2[1], m1[2]];
        let base = [
            (m1[0] + foot[0]) / 2.0,
            (m1[1] + foot[1]) / 2.0,
            m1[2],
        ];
        return Some((base, d1));
    }

    // ── 相交：用"交点到两根线各自远端"的两条射线定角 ──
    let o = line_intersection(first, second)?;
    let far1 = farther_end(&first, &o);
    let far2 = farther_end(&second, &o);
    let (u1, u2) = (
        norm2([far1[0] - o[0], far1[1] - o[1]])?,
        norm2([far2[0] - o[0], far2[1] - o[1]])?,
    );
    let u = norm2([u1[0] + u2[0], u1[1] + u2[1]])?; // 两远端方向相反（共线）时 → None
    Some((o, u))
}

/// 两根**无限直线**（由线段定）的交点；平行 → `None`。
fn line_intersection(first: Seg, second: Seg) -> Option<P> {
    let (p, r) = (
        [first.a[0], first.a[1]],
        [first.b[0] - first.a[0], first.b[1] - first.a[1]],
    );
    let (q, s) = (
        [second.a[0], second.a[1]],
        [second.b[0] - second.a[0], second.b[1] - second.a[1]],
    );
    let denom = r[0] * s[1] - r[1] * s[0];
    if denom.abs() < 1e-12 {
        return None;
    }
    let qp = [q[0] - p[0], q[1] - p[1]];
    let t = (qp[0] * s[1] - qp[1] * s[0]) / denom;
    Some([p[0] + t * r[0], p[1] + t * r[1], first.a[2]])
}

/// 离 `o` 更远的那个端点（远端）。
fn farther_end(seg: &Seg, o: &P) -> P {
    let d = |p: &P| {
        let (dx, dy) = (p[0] - o[0], p[1] - o[1]);
        dx * dx + dy * dy
    };
    if d(&seg.a) >= d(&seg.b) {
        seg.a
    } else {
        seg.b
    }
}

fn norm2(v: [f64; 2]) -> Option<[f64; 2]> {
    let n = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if n < 1e-12 {
        return None;
    }
    Some([v[0] / n, v[1] / n])
}

/// 线段 → `acadrust` 直线（落在 `3中心线层`）。
pub(crate) fn to_line(seg: &Seg) -> acadrust::entities::Line {
    let mut l = acadrust::entities::Line::from_points(v3(seg.a), v3(seg.b));
    l.common.layer = LAYER_CENTERLINE.to_string();
    l
}

/// 把一组线段包成待提交的实体。
pub(crate) fn lines_of(segs: &[Seg]) -> Vec<EntityType> {
    segs.iter().map(|s| EntityType::Line(to_line(s))).collect()
}

// ── 交互命令 ─────────────────────────────────────────────────────────────

/// 拾取到的几何。
#[derive(Debug, Clone, Copy)]
enum Picked {
    /// 圆 / 圆弧：圆心 + 半径。
    Round { center: P, radius: f64 },
    /// 直线段。
    Straight(Seg),
}

/// 一次拾取的结果。
#[derive(Debug)]
pub(crate) enum Advance {
    /// 还需要下一次拾取（`notice` 非空时会在提示里说明原因）。
    KeepGoing(Option<String>),
    /// 已经可以落图（`info` 给命令路径用；交互路径无法 push_info）。
    Commit {
        segs: Vec<Seg>,
        info: String,
    },
}

/// `ZX` 交互命令：点圆/圆弧 → 十字中心线；点直线 → 再点一根 → 角平分线中心线。
pub(crate) struct CenterLinePick {
    doc: CadDocument,
    first: Option<Seg>,
    notice: Option<String>,
    /// 第一次拾取处的图框比例（`n`）。
    scale: f64,
}

impl CenterLinePick {
    pub(crate) fn new(doc: CadDocument) -> Self {
        Self {
            doc,
            first: None,
            notice: None,
            scale: 1.0,
        }
    }

    /// 按句柄从快照读几何。
    fn classify(&self, handle: Handle) -> Option<Picked> {
        match self.doc.get_entity(handle)? {
            EntityType::Line(l) => Some(Picked::Straight(Seg::new(to_arr(&l.start), to_arr(&l.end)))),
            EntityType::Circle(c) => Some(Picked::Round {
                center: to_arr(&c.center),
                radius: c.radius,
            }),
            EntityType::Arc(a) => Some(Picked::Round {
                center: to_arr(&a.center),
                radius: a.radius,
            }),
            _ => None,
        }
    }

    /// 拾取一次（纯逻辑，便于单测）。
    pub(crate) fn advance(&mut self, handle: Handle, pick_pt: P) -> Advance {
        // 图框比例跟拾取点走（和 D/GDIM 的"比例感知"一致）。
        self.scale = crate::frame_scale_at(&self.doc, pick_pt);
        let Some(picked) = self.classify(handle) else {
            self.notice = Some(crate::i18n::t("cmd.centerline.err.bad_pick"));
            return Advance::KeepGoing(self.notice.clone());
        };
        match picked {
            Picked::Round { center, radius } => {
                let segs = cross_for_circle(center, radius, self.scale).to_vec();
                Advance::Commit {
                    info: crate::i18n::t_fmt(
                        "cmd.centerline.info.cross",
                        &[
                            ("dia", &format!("{:.3}", radius * 2.0)),
                            ("scale", &crate::trim_scale(self.scale)),
                            ("over", &OVERHANG_MM.to_string()),
                            (
                                "len",
                                &format!("{:.3}", 2.0 * radius + OVERHANG_MM * self.scale),
                            ),
                            ("layer", LAYER_CENTERLINE),
                        ],
                    ),
                    segs,
                }
            }
            Picked::Straight(seg) => match self.first {
                None => {
                    if seg.len() < 1e-9 {
                        self.notice = Some(crate::i18n::t("cmd.centerline.err.zero_len_first"));
                        return Advance::KeepGoing(self.notice.clone());
                    }
                    self.first = Some(seg);
                    self.notice = None;
                    Advance::KeepGoing(None)
                }
                Some(first) => match bisector(first, seg, self.scale) {
                    Some(cl) => Advance::Commit {
                        info: crate::i18n::t_fmt(
                            "cmd.centerline.info.bisector",
                            &[
                                ("len", &format!("{:.3}", cl.len())),
                                (
                                    "proj",
                                    &format!("{:.3}", cl.len() - OVERHANG_MM * self.scale),
                                ),
                                ("scale", &crate::trim_scale(self.scale)),
                                ("over", &OVERHANG_MM.to_string()),
                                ("layer", LAYER_CENTERLINE),
                            ],
                        ),
                        segs: vec![cl],
                    },
                    None => {
                        self.notice = Some(crate::i18n::t("cmd.centerline.err.zero_len_pair"));
                        Advance::KeepGoing(self.notice.clone())
                    }
                },
            },
        }
    }
}

impl InteractiveCommand for CenterLinePick {
    fn prompt(&self) -> String {
        match (&self.first, &self.notice) {
            (Some(_), _) => crate::i18n::t("cmd.centerline.prompt.second"),
            (None, Some(n)) => {
                crate::i18n::t_fmt("cmd.centerline.prompt.notice", &[("n", n)])
            }
            (None, None) => crate::i18n::t("cmd.centerline.prompt.pick"),
        }
    }

    /// 每一步都拾取**对象**（不是自由点）。
    fn needs_object_pick(&self) -> bool {
        true
    }

    /// 关掉对象捕捉——点的是对象本身，不要弹到象限点/圆心上去。
    fn entity_pick_applies_osnap(&self) -> bool {
        false
    }

    fn wants_mouse_move(&self) -> bool {
        false
    }

    /// 非关键字 token（十六进制句柄、坐标）**不消费**，交回宿主按常规解释
    /// —— 这样自动化/MCP 里可以直接 `ZX <handle>` 指定要处理的对象
    /// （宿主把句柄喂进实体拾取步骤；POWERDIM 同款写法）。
    fn on_text_input(&mut self, _text: &str) -> CommandStep {
        CommandStep::Ignored
    }

    /// 空白处点击（没点到对象）：继续等。
    fn on_point(&mut self, _pt: P) -> CommandStep {
        self.notice = Some(crate::i18n::t("cmd.centerline.err.no_object"));
        CommandStep::NeedPoint
    }

    fn on_object_pick_snapped(&mut self, handle: Handle, pt: P, _snapped: bool) -> CommandStep {
        match self.advance(handle, pt) {
            Advance::KeepGoing(_) => CommandStep::NeedPoint,
            Advance::Commit { segs, .. } => CommandStep::CommitManyAndEnd(lines_of(&segs)),
        }
    }
}

// ── 命令入口（命令路径 / 选区 / 自动化） ─────────────────────────────────

/// 直线段 → 命令行的"选区一次算完"入口：给一组句柄算出中心线。
///
/// 返回 `(线段, 说明)`；选区不符合（不是 1 圆/弧、也不是 2 直线）→ `None`
/// （调用方转交互拾取）。
pub(crate) fn from_selection(doc: &CadDocument, handles: &[Handle], scale: f64) -> Option<(Vec<Seg>, String)> {
    let picked: Vec<Picked> = handles.iter().filter_map(|h| {
        match doc.get_entity(*h)? {
            EntityType::Line(l) => Some(Picked::Straight(Seg::new(to_arr(&l.start), to_arr(&l.end)))),
            EntityType::Circle(c) => Some(Picked::Round {
                center: to_arr(&c.center),
                radius: c.radius,
            }),
            EntityType::Arc(a) => Some(Picked::Round {
                center: to_arr(&a.center),
                radius: a.radius,
            }),
            _ => None,
        }
    })
    .collect();
    match picked.as_slice() {
        [Picked::Round { center, radius }] => Some((
            cross_for_circle(*center, *radius, scale).to_vec(),
            crate::i18n::t_fmt(
                "cmd.centerline.info.cross_selection",
                &[
                    ("dia", &format!("{:.3}", radius * 2.0)),
                    ("scale", &crate::trim_scale(scale)),
                    ("over", &OVERHANG_MM.to_string()),
                    ("layer", LAYER_CENTERLINE),
                ],
            ),
        )),
        [Picked::Straight(a), Picked::Straight(b)] => {
            let cl = bisector(*a, *b, scale)?;
            Some((
                vec![cl],
                crate::i18n::t_fmt(
                    "cmd.centerline.info.bisector_selection",
                    &[
                        ("len", &format!("{:.3}", cl.len())),
                        ("layer", LAYER_CENTERLINE),
                    ],
                ),
            ))
        }
        _ => None,
    }
}

/// 命令入口：带选区就一次算完，否则进交互拾取。
pub(crate) fn cmd_centerline(host: &mut dyn HostApi) {
    ensure_layers_before_draw(host);
    let doc = host.document().clone();
    let sel = host.selected_handles();
    if !sel.is_empty() {
        // 图框比例按选区里第一个实体的基准点查（自动化路径没有“拾取点”）。
        let scale_pt = sel
            .iter()
            .find_map(|h| doc.get_entity(*h))
            .map(|e| match e {
                EntityType::Line(l) => to_arr(&l.start),
                EntityType::Circle(c) => to_arr(&c.center),
                EntityType::Arc(a) => to_arr(&a.center),
                _ => [0.0, 0.0, 0.0],
            })
            .unwrap_or([0.0, 0.0, 0.0]);
        let scale = crate::frame_scale_at(&doc, scale_pt);
        if let Some((segs, info)) = from_selection(&doc, &sel, scale) {
            host.push_undo(&crate::i18n::t("cmd.centerline.undo_label"));
            let n = segs.len();
            let _ = host.add_entities(lines_of(&segs));
            host.set_dirty();
            host.push_info(&info);
            host.push_output(&crate::i18n::t_fmt(
                "cmd.centerline.info.draw_count",
                &[("n", &n.to_string())],
            ));
            return;
        }
        host.push_info(&crate::i18n::t("cmd.centerline.err.bad_selection"));
    }
    host.start_interactive(Box::new(CenterLinePick::new(doc)));
}

/// 图层/线型基建（`OCSM` 没跑过的图也能直接用）。
pub(crate) fn ensure_layers_before_draw(host: &mut dyn HostApi) {
    host.ensure_layers(layer_defs());
    host.ensure_linetypes(linetype_defs());
}

#[cfg(test)]
mod tests {
    use super::*;
    use acadrust::types::Vector3;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    /// 语言是进程级全局：断言中文文案的用例与「切语言」用例共用锁并钉死 zh。
    fn zh_guard() -> std::sync::MutexGuard<'static, ()> {
        let g = crate::global_state_test_lock();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        g
    }

    // ── 圆 / 圆弧 ────────────────────────────────────────────────────────

    #[test]
    fn cross_length_is_diameter_plus_6n() {
        let segs = cross_for_circle([10.0, 20.0, 0.0], 5.0, 1.0);
        for s in segs {
            assert!(close(s.len(), 10.0 + 6.0), "n=1 → 直径 10 + 6 = {}", s.len());
        }
        let segs2 = cross_for_circle([0.0, 0.0, 0.0], 5.0, 2.0);
        for s in segs2 {
            assert!(close(s.len(), 10.0 + 12.0), "n=2 → 10 + 12 = {}", s.len());
        }
        let segs3 = cross_for_circle([0.0, 0.0, 0.0], 2.5, 0.5);
        for s in segs3 {
            assert!(close(s.len(), 5.0 + 3.0), "n=0.5 → 5 + 3 = {}", s.len());
        }
    }

    #[test]
    fn cross_is_centered_on_the_centre_and_axis_aligned() {
        let [h, v] = cross_for_circle([3.0, -4.0, 1.5], 4.0, 1.0);
        // 横线在 y = 圆心 y 上、以圆心为中心
        assert!(close(h.mid()[0], 3.0) && close(h.mid()[1], -4.0));
        assert!(close(h.a[1], h.b[1]) && close(h.a[1], -4.0));
        assert!(close(h.a[0], 3.0 - h.len() / 2.0) && close(h.b[0], 3.0 + h.len() / 2.0));
        // 竖线在 x = 圆心 x 上
        assert!(close(v.mid()[0], 3.0) && close(v.mid()[1], -4.0));
        assert!(close(v.a[0], v.b[0]) && close(v.a[0], 3.0));
        // z 跟着圆心
        assert!(close(h.a[2], 1.5) && close(v.a[2], 1.5));
    }

    #[test]
    fn cross_lines_land_on_the_centreline_layer() {
        let segs = cross_for_circle([0.0, 0.0, 0.0], 3.0, 1.0);
        for e in lines_of(&segs) {
            match e {
                EntityType::Line(l) => {
                    assert_eq!(l.common.layer, "3中心线层");
                    assert_eq!(l.common.layer, LAYER_CENTERLINE);
                }
                other => panic!("应当是直线，得到 {other:?}"),
            }
        }
    }

    // ── 两根直线：角平分线 ───────────────────────────────────────────────

    #[test]
    fn bisector_parallel_lines_gives_the_middle_line() {
        // 两根水平线 y=0 / y=10，都 x 0..100 → 中线 y=5，长 100+6
        let a = Seg::new([0.0, 0.0, 0.0], [100.0, 0.0, 0.0]);
        let b = Seg::new([0.0, 10.0, 0.0], [100.0, 10.0, 0.0]);
        let cl = bisector(a, b, 1.0).expect("平行也要出线");
        assert!(close(cl.len(), 106.0), "长 = 100 + 6 = {}", cl.len());
        assert!(close(cl.mid()[1], 5.0), "中线 y=5，得到 {}", cl.mid()[1]);
        assert!(close(cl.mid()[0], 50.0), "中点 x = 第一根线中点 x");
        assert!(close(cl.a[1], 5.0) && close(cl.b[1], 5.0), "中线必须水平");
    }

    #[test]
    fn bisector_parallel_antiparallel_lines_also_land_in_the_middle() {
        // 第二根线反过来画（端点顺序相反）结果必须一样
        let a = Seg::new([0.0, 0.0, 0.0], [100.0, 0.0, 0.0]);
        let b = Seg::new([100.0, 10.0, 0.0], [0.0, 10.0, 0.0]);
        let cl = bisector(a, b, 1.0).unwrap();
        assert!(close(cl.mid()[1], 5.0) && close(cl.len(), 106.0));
    }

    #[test]
    fn bisector_right_angle_corner_is_45_degrees_through_the_vertex() {
        // 直角：line1 = (0,0)→(100,0)；line2 = (0,0)→(0,100)
        // 内角平分线 = 45° 过原点；中点 = line1 中点 (50,0) 在 45° 线上的投影 = (25,25)
        let a = Seg::new([0.0, 0.0, 0.0], [100.0, 0.0, 0.0]);
        let b = Seg::new([0.0, 0.0, 0.0], [0.0, 100.0, 0.0]);
        let cl = bisector(a, b, 1.0).unwrap();
        let (dx, dy) = (cl.b[0] - cl.a[0], cl.b[1] - cl.a[1]);
        assert!(close(dx.abs(), dy.abs()), "45° 线：{cl:?}");
        assert!(close(cl.mid()[0], 25.0) && close(cl.mid()[1], 25.0), "中点 {:?}", cl.mid());
        assert!(close(cl.len(), 100.0 / 2f64.sqrt() + 6.0), "长 {}", cl.len());
    }

    #[test]
    fn bisector_obtuse_corner_uses_the_angle_that_holds_the_segments() {
        // 135° 的角：line1 = (0,0)→(100,0)；line2 = (0,0)→(−100,100)
        // 两条远端射线夹角 135° → 平分线方向 67.5°
        let a = Seg::new([0.0, 0.0, 0.0], [100.0, 0.0, 0.0]);
        let b = Seg::new([0.0, 0.0, 0.0], [-100.0, 100.0, 0.0]);
        let cl = bisector(a, b, 1.0).unwrap();
        let (dx, dy) = (cl.b[0] - cl.a[0], cl.b[1] - cl.a[1]);
        let ang = dy.atan2(dx).to_degrees();
        assert!(
            close(ang, 67.5) || close(ang, 67.5 - 180.0),
            "应为 67.5°（135° 的内角平分线），得到 {ang}"
        );
        // 平分线**直线**过顶点 (0,0)（中点本身是投影点，不在顶点上）
        let cross = (cl.mid()[0] - 0.0) * dy - (cl.mid()[1] - 0.0) * dx;
        assert!(close(cross, 0.0), "中点离顶点连线应与方向共线：{cross}");
        // 且中点在第一根线中点在平分线上的投影处：M1=(50,0) → t = 50·cos67.5°
        let t = cl.mid()[0] * (dx / dx.hypot(dy)) + cl.mid()[1] * (dy / dx.hypot(dy));
        assert!(close(t, 50.0 * 67.5f64.to_radians().cos()), "投影长度 t={t}");
    }

    #[test]
    fn bisector_midpoint_is_the_projection_of_the_first_midpoint() {
        // 斜交：line1 水平 (0,0)→(200,0)；line2 竖在 x=100： (100,-50)→(100,50)
        // 交点 (100,0) 落在 line1 内、line2 内 → 远端 = (200,0) 与 (100,50) → 45°
        // line1 中点 (100,0) 已在平分线上（平分线过交点）→ 中点 = (100,0)
        let a = Seg::new([0.0, 0.0, 0.0], [200.0, 0.0, 0.0]);
        let b = Seg::new([100.0, -50.0, 0.0], [100.0, 50.0, 0.0]);
        let cl = bisector(a, b, 1.0).unwrap();
        assert!(close(cl.mid()[0], 100.0) && close(cl.mid()[1], 0.0), "{:?}", cl.mid());
        // 两根线在平面里成"十"字（交点在中点）→ 两根平分线都合法，
        // 只断言结果是 45° 家族（|dx| == |dy|），不管朝哪边。
        let (dx, dy) = (cl.b[0] - cl.a[0], cl.b[1] - cl.a[1]);
        assert!(close(dx.abs(), dy.abs()), "角平分线应 ±45° 家族，得到 ({dx},{dy})");
        // 直线必须过交点 (100,0)
        let cross = (cl.mid()[0] - 100.0) * dy - (cl.mid()[1] - 0.0) * dx;
        assert!(close(cross, 0.0), "平分线应过交点：{cross}");
    }

    #[test]
    fn bisector_rejects_zero_length_lines() {
        let a = Seg::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        let b = Seg::new([0.0, 5.0, 0.0], [100.0, 5.0, 0.0]);
        assert!(bisector(a, b, 1.0).is_none());
    }

    #[test]
    fn bisector_scale_enters_only_through_the_overhang() {
        let a = Seg::new([0.0, 0.0, 0.0], [100.0, 0.0, 0.0]);
        let b = Seg::new([0.0, 10.0, 0.0], [100.0, 10.0, 0.0]);
        let s1 = bisector(a, b, 1.0).unwrap();
        let s2 = bisector(a, b, 2.0).unwrap();
        assert!(close(s2.len() - s1.len(), 6.0), "n 每加 1 → 长度 +6mm");
        assert!(close(s1.mid()[1], s2.mid()[1]), "位置与 n 无关");
    }

    // ── 交互命令 ────────────────────────────────────────────────────────

    fn doc_with_circle_and_lines() -> (CadDocument, Handle, Handle, Handle) {
        let mut doc = CadDocument::default();
        let c = doc
            .add_entity(EntityType::Circle(
                acadrust::entities::Circle::from_center_radius(
                    Vector3::new(50.0, 50.0, 0.0),
                    10.0,
                ),
            ))
            .unwrap();
        let l1 = doc
            .add_entity(EntityType::Line(acadrust::entities::Line::from_points(
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(100.0, 0.0, 0.0),
            )))
            .unwrap();
        let l2 = doc
            .add_entity(EntityType::Line(acadrust::entities::Line::from_points(
                Vector3::new(0.0, 10.0, 0.0),
                Vector3::new(100.0, 10.0, 0.0),
            )))
            .unwrap();
        (doc, c, l1, l2)
    }

    #[test]
    fn picking_a_circle_commits_two_cross_lines_in_one_step() {
        let _g = zh_guard();
        let (doc, c, _, _) = doc_with_circle_and_lines();
        let mut cmd = CenterLinePick::new(doc);
        match cmd.advance(c, [60.0, 50.0, 0.0]) {
            Advance::Commit { segs, info } => {
                assert_eq!(segs.len(), 2, "十字 = 两条");
                assert!(info.contains("十字中心线"), "{info}");
                for s in segs {
                    assert!(close(s.len(), 20.0 + 6.0), "Ø20 + 6 = 26，得到 {}", s.len());
                }
            }
            other => panic!("应当一次落两条，得到 {other:?}"),
        }
    }

    #[test]
    fn picking_two_lines_commits_the_bisector_after_the_second_pick() {
        let _g = zh_guard();
        let (doc, _, l1, l2) = doc_with_circle_and_lines();
        let mut cmd = CenterLinePick::new(doc);
        assert!(matches!(cmd.advance(l1, [50.0, 0.0, 0.0]), Advance::KeepGoing(_)));
        match cmd.advance(l2, [50.0, 10.0, 0.0]) {
            Advance::Commit { segs, info } => {
                assert_eq!(segs.len(), 1);
                assert!(close(segs[0].mid()[1], 5.0), "两根线的中线");
                assert!(info.contains("角平分线"), "{info}");
            }
            other => panic!("应当落一条，得到 {other:?}"),
        }
    }

    #[test]
    fn picking_something_else_keeps_asking() {
        let _g = zh_guard();
        let (mut doc, _, _, _) = doc_with_circle_and_lines();
        let t = doc
            .add_entity(EntityType::Point({
                let mut p = acadrust::entities::Point::new();
                p.location = Vector3::new(1.0, 1.0, 0.0);
                p
            }))
            .unwrap();
        let mut cmd = CenterLinePick::new(doc);
        match cmd.advance(t, [1.0, 1.0, 0.0]) {
            Advance::KeepGoing(Some(msg)) => assert!(msg.contains("圆"), "{msg}"),
            other => panic!("应继续等待，得到 {other:?}"),
        }
        assert!(cmd.prompt().contains("只能点圆"), "提示：{}", cmd.prompt());
    }

    #[test]
    fn interactive_trait_reports_entity_pick_without_osnap() {
        use ocs_plugin_api::host::InteractiveCommand as Ic;
        let _g = zh_guard();
        let (doc, _, _, _) = doc_with_circle_and_lines();
        let mut cmd = CenterLinePick::new(doc);
        assert!(cmd.needs_object_pick());
        assert!(!cmd.entity_pick_applies_osnap(), "点对象本身，不要捕捉");
        assert!(!cmd.wants_mouse_move());
        assert!(cmd.prompt().contains("点圆/圆弧"));
        // 句柄/坐标 token 必须**不消费**（宿主才能按句柄拾取；`ZX 70` 自动化路径）
        assert!(matches!(
            cmd.on_text_input("70"),
            CommandStep::Ignored
        ));
    }

    #[test]
    fn trait_object_pick_commits_two_lines_in_one_step() {
        use ocs_plugin_api::host::InteractiveCommand as Ic;
        let (doc, c, _, _) = doc_with_circle_and_lines();
        let mut cmd = CenterLinePick::new(doc);
        match cmd.on_object_pick_snapped(c, [60.0, 50.0, 0.0], false) {
            CommandStep::CommitManyAndEnd(ents) => {
                assert_eq!(ents.len(), 2, "十字 = 两条 LINE 一次提交");
                for e in ents {
                    match e {
                        EntityType::Line(l) => assert_eq!(l.common.layer, "3中心线层"),
                        other => panic!("应当是直线，得到 {other:?}"),
                    }
                }
            }
            other => panic!("应当是 CommitManyAndEnd，得到 {other:?}"),
        }
    }

    #[test]
    fn selection_entry_builds_cross_and_bisector() {
        let _g = zh_guard();
        let (doc, c, l1, l2) = doc_with_circle_and_lines();
        let (segs, info) = from_selection(&doc, &[c], 1.0).expect("1 个圆");
        assert_eq!(segs.len(), 2);
        assert!(info.contains("十字"), "{info}");
        let (segs, info) = from_selection(&doc, &[l1, l2], 2.0).expect("2 根线");
        assert_eq!(segs.len(), 1);
        assert!(info.contains("角平分线"), "{info}");
        assert!(close(segs[0].len(), 106.0 + 6.0), "n=2 → 100 + 12");
        // 不合法选区 → None（转交互）
        assert!(from_selection(&doc, &[l1], 1.0).is_none());
        assert!(from_selection(&doc, &[l1, l2, c], 1.0).is_none());
    }

    /// ② 批双语：中心线的回执/报错在 zh/en 下均取到，且保留数值与图层协议名。
    #[test]
    fn centerline_messages_switch_language_keeping_data() {
        let _g = crate::global_state_test_lock();
        let (doc, c, l1, l2) = doc_with_circle_and_lines();
        crate::i18n::set_lang(crate::i18n::Lang::Zh);
        let (_, zh_cross) = from_selection(&doc, &[c], 1.0).expect("圆");
        let (_, zh_bi) = from_selection(&doc, &[l1, l2], 2.0).expect("两线");
        crate::i18n::set_lang(crate::i18n::Lang::En);
        let (_, en_cross) = from_selection(&doc, &[c], 1.0).expect("圆");
        let (_, en_bi) = from_selection(&doc, &[l1, l2], 2.0).expect("两线");
        let en_pick = crate::i18n::t("cmd.centerline.err.bad_pick");
        crate::i18n::set_lang_auto();
        assert!(
            zh_cross.contains("十字") && zh_cross.contains(LAYER_CENTERLINE),
            "{zh_cross}"
        );
        assert!(
            en_cross.contains("cross") && en_cross.contains(LAYER_CENTERLINE),
            "{en_cross}"
        );
        assert!(
            zh_bi.contains("角平分线") && en_bi.contains("bisector"),
            "{zh_bi} / {en_bi}"
        );
        assert!(en_pick.contains("Pick a circle"), "{en_pick}");
    }
}
