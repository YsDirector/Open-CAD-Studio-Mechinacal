//! 局部放大图裁剪引擎（纯函数，便于单测）。
//!
//! 把 `10引导线层` 上引导形状（圆 / 轴对齐矩形）**内部**的内容实体裁剪出来：
//! 与窗口求交，丢弃窗口外部分，保留窗口内部分。裁剪窗口为凸形：
//! - 圆窗口：直线最多 1 段、圆弧最多 1 段；
//! - 矩形窗口：直线最多 1 段、圆弧最多 2 段。
//!
//! 输出统一按 `(p − center) × k` 变换（k = 局部放大比例 × TF 图幅倍率），
//! 生成块成员实体，保留原图层/颜色。

use ocs_plugin_api::host::acadrust::entities::{
    Arc, BoundaryEdge, BoundaryPath, Circle, Ellipse, Hatch, Line, LwPolyline, Polyline,
    Polyline2D, PolylineEdge, Spline,
};
use ocs_plugin_api::host::acadrust::types::{Color, Vector2, Vector3};
use ocs_plugin_api::host::acadrust::EntityType as E;

/// 裁剪窗口（凸形）。
#[derive(Debug, Clone, Copy)]
pub enum Win {
    /// 圆：中心 + 半径。
    Circle { center: (f64, f64), radius: f64 },
    /// 轴对齐矩形：min/max 角。
    Rect { min: (f64, f64), max: (f64, f64) },
}

impl Win {
    pub fn contains(&self, p: (f64, f64)) -> bool {
        match self {
            Win::Circle { center, radius } => {
                (p.0 - center.0).powi(2) + (p.1 - center.1).powi(2) <= radius * radius + EPS
            }
            Win::Rect { min, max } => {
                p.0 >= min.0 - EPS && p.0 <= max.0 + EPS && p.1 >= min.1 - EPS && p.1 <= max.1 + EPS
            }
        }
    }

    /// 窗口中心（放大变换原点）。
    pub fn center(&self) -> (f64, f64) {
        match self {
            Win::Circle { center, .. } => *center,
            Win::Rect { min, max } => ((min.0 + max.0) * 0.5, (min.1 + max.1) * 0.5),
        }
    }

    /// 外接半径（放大块默认偏移/框尺寸估算用）。
    pub fn circum_radius(&self) -> f64 {
        match self {
            Win::Circle { radius, .. } => *radius,
            Win::Rect { min, max } => {
                ((max.0 - min.0).powi(2) + (max.1 - min.1).powi(2)).sqrt() * 0.5
            }
        }
    }

    /// 线段裁剪：返回窗口内子段（a→b 参数化），None = 全部在外。
    pub fn clip_segment(&self, a: (f64, f64), b: (f64, f64)) -> Option<((f64, f64), (f64, f64))> {
        match self {
            Win::Circle { center: c, radius } => {
                clip_segment_circle(a, b, *c, *radius)
            }
            Win::Rect { min, max } => clip_segment_rect(a, b, *min, *max),
        }
    }

    /// 圆弧裁剪（start→end 逆时针扫角，span = norm(end−start) ∈ [0,2π)）。
    /// 返回窗口内弧段（角度区间，仍为逆时针方向），最多 2 段。
    pub fn clip_arc(
        &self,
        c: (f64, f64),
        r: f64,
        a0: f64,
        a1: f64,
    ) -> Vec<(f64, f64)> {
        match self {
            Win::Circle { center: wc, radius } => {
                clip_arc_circle(c, r, a0, a1, *wc, *radius)
            }
            Win::Rect { min, max } => clip_arc_rect(c, r, a0, a1, *min, *max),
        }
    }

    /// 圆裁剪：返回 0..=1 个窗口内弧区间（角度，逆时针 span）。
    pub fn clip_circle(&self, c: (f64, f64), r: f64) -> Vec<(f64, f64)> {
        // π/2 起点的整圆扫描 → 把圆当作 0..2π 的弧。
        let segs = self.clip_arc(c, r, 0.0, std::f64::consts::TAU);
        segs.into_iter()
            .map(|(s, e)| {
                // 区间整圆（e−s≥2π−ε）→ (0,2π)；否则弧（角度已规范化）。
                if e - s >= std::f64::consts::TAU - 1e-6 {
                    (0.0, std::f64::consts::TAU)
                } else {
                    (s, e)
                }
            })
            .collect()
    }
}

const EPS: f64 = 1e-9;

/// 规范化角度到 [0, 2π)。
#[inline]
pub fn norm_angle(a: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    let mut x = a % tau;
    if x < 0.0 {
        x += tau;
    }
    x
}

/// 线段与圆求交：窗口内子段。圆内 = 到圆心的距离 ≤ r。
fn clip_segment_circle(
    a: (f64, f64),
    b: (f64, f64),
    c: (f64, f64),
    r: f64,
) -> Option<((f64, f64), (f64, f64))> {
    let (ax, ay) = (a.0 - c.0, a.1 - c.1);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let d2 = dx * dx + dy * dy;
    // 退化点
    if d2 <= EPS * EPS {
        return if ax * ax + ay * ay <= r * r + EPS {
            Some((a, b))
        } else {
            None
        };
    }
    let q = ax * dx + ay * dy;
    let det = q * q - d2 * (ax * ax + ay * ay - r * r);
    if det < -EPS {
        return None;
    }
    let sq = det.max(0.0).sqrt();
    let t0 = (-q - sq) / d2;
    let t1 = (-q + sq) / d2;
    // 窗口内 参数区间 = [t0,t1] ∩ [0,1]（圆外 ⇒ 区间外）。
    let s = t0.max(0.0);
    let e = t1.min(1.0);
    if s > e + EPS {
        return None;
    }
    let pa = (a.0 + s * dx, a.1 + s * dy);
    let pb = (a.0 + e * dx, a.1 + e * dy);
    Some((pa, pb))
}

/// 线段与轴对齐矩形求交（半平面求交）。
fn clip_segment_rect(
    a: (f64, f64),
    b: (f64, f64),
    min: (f64, f64),
    max: (f64, f64),
) -> Option<((f64, f64), (f64, f64))> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (s, e) = clip_interval(a, (dx, dy), min, max);
    if s >= e + EPS {
        return None;
    }
    Some(((a.0 + s * dx, a.1 + s * dy), (a.0 + e * dx, a.1 + e * dy)))
}

/// 参数化直线被 4 个半平面约束后的参数区间。
fn clip_interval(p: (f64, f64), d: (f64, f64), min: (f64, f64), max: (f64, f64)) -> (f64, f64) {
    let mut s: f64 = 0.0;
    let mut e: f64 = 1.0;
    // 每个约束：n·(p + t d) ≥ c。
    for (n, c) in [
        ((1.0, 0.0), min.0),
        ((-1.0, 0.0), -max.0),
        ((0.0, 1.0), min.1),
        ((0.0, -1.0), -max.1),
    ] {
        let nd = n.0 * d.0 + n.1 * d.1;
        let np = n.0 * p.0 + n.1 * p.1;
        if nd > EPS {
            let t = (c - np) / nd;
            s = s.max(t);
        } else if nd < -EPS {
            let t = (c - np) / nd;
            e = e.min(t);
        } else if np < c - EPS {
            // 平行且在外 → 无解
            return (1.0, 0.0);
        }
    }
    (s, e)
}

/// 圆与圆交点极角（相对弧心），返回 0..2 个角度。
fn circle_circle_angles(
    ac: (f64, f64),
    ar: f64,
    bc: (f64, f64),
    br: f64,
) -> Vec<f64> {
    let (dx, dy) = (bc.0 - ac.0, bc.1 - ac.1);
    let d = (dx * dx + dy * dy).sqrt();
    if d <= EPS {
        return Vec::new(); // 同心 → 无孤立交点
    }
    if d > ar + br + 1e-7 || d < (ar - br).abs() - 1e-7 {
        return Vec::new();
    }
    let x = (d * d + ar * ar - br * br) / (2.0 * d);
    let h2 = ar * ar - x * x;
    if h2 < -1e-9 {
        return Vec::new();
    }
    let h = h2.max(0.0).sqrt();
    let base = dy.atan2(dx);
    // 沿圆心连线方向的投影 x 除**弧半径**才是夹角余弦（除以 d 是错的：
    // 会把裁剪角放大，弧穿出窗口——用户 局部放大2.dxf 实测）。
    let off = (x / ar).clamp(-1.0, 1.0).acos();
    let mut out = vec![norm_angle(base + off)];
    if h > 1e-9 {
        out.push(norm_angle(base - off));
    }
    out
}

/// 弧的逆时针扫描角（跨 [0,2π)）；整圆特例（a0≈a1 或差≈2π）→ 2π。
#[inline]
fn arc_span(a0: f64, a1: f64) -> f64 {
    let diff = a1 - a0;
    if diff.abs() <= EPS || diff.abs() >= std::f64::consts::TAU - EPS {
        std::f64::consts::TAU
    } else {
        norm_angle(diff)
    }
}

/// 圆弧裁剪（圆窗口）。
fn clip_arc_circle(
    c: (f64, f64),
    r: f64,
    a0: f64,
    a1: f64,
    wc: (f64, f64),
    wr: f64,
) -> Vec<(f64, f64)> {
    let span = arc_span(a0, a1);
    let hits = circle_circle_angles(c, r, wc, wr);
    let mut ts: Vec<f64> = hits.into_iter().map(|alpha| arc_t_of_angle(a0, span, alpha)).collect();
    ts.push(0.0);
    ts.push(1.0);
    ts.sort_by(|x, y| x.partial_cmp(y).unwrap());
    segments_inside(ts, |th| {
        let p = (c.0 + r * th.cos(), c.1 + r * th.sin());
        (p.0 - wc.0).powi(2) + (p.1 - wc.1).powi(2) <= wr * wr + EPS
    }, a0, span)
}

/// 圆弧裁剪（矩形窗口）：与 4 边线段求交。
fn clip_arc_rect(
    c: (f64, f64),
    r: f64,
    a0: f64,
    a1: f64,
    min: (f64, f64),
    max: (f64, f64),
) -> Vec<(f64, f64)> {
    let span = arc_span(a0, a1);
    let mut ts = Vec::new();
    // 圆与矩形 4 边的交点（线段参数 t_line 与 弧参数 t_arc）。
    let edges = [
        ((min.0, min.1), (max.0, min.1)),
        ((max.0, min.1), (max.0, max.1)),
        ((max.0, max.1), (min.0, max.1)),
        ((min.0, max.1), (min.0, min.1)),
    ];
    for (ea, eb) in edges {
        for t in line_circle_params(ea, eb, c, r) {
            let p = (ea.0 + t * (eb.0 - ea.0), ea.1 + t * (eb.1 - ea.1));
            let alpha = norm_angle((p.1 - c.1).atan2(p.0 - c.0));
            ts.push(arc_t_of_angle(a0, span, alpha));
        }
    }
    ts.push(0.0);
    ts.push(1.0);
    ts.sort_by(|x, y| x.partial_cmp(y).unwrap());
    segments_inside(ts, |th| {
        let p = (c.0 + r * th.cos(), c.1 + r * th.sin());
        p.0 >= min.0 - EPS && p.0 <= max.0 + EPS && p.1 >= min.1 - EPS && p.1 <= max.1 + EPS
    }, a0, span)
}

/// 线段与圆求交：返回线段参数 t∈[0,1] 上的全部圆交点（0..=2 个）。
fn line_circle_params(a: (f64, f64), b: (f64, f64), c: (f64, f64), r: f64) -> Vec<f64> {
    let (ax, ay) = (a.0 - c.0, a.1 - c.1);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let d2 = dx * dx + dy * dy;
    if d2 <= EPS * EPS {
        return Vec::new();
    }
    let q = ax * dx + ay * dy;
    let det = q * q - d2 * (ax * ax + ay * ay - r * r);
    if det < -EPS {
        return Vec::new();
    }
    let sq = det.max(0.0).sqrt();
    let mut out = Vec::with_capacity(2);
    for t in [(-q - sq) / d2, (-q + sq) / d2] {
        if t >= -EPS && t <= 1.0 + EPS {
            out.push(t.clamp(0.0, 1.0));
        }
    }
    out
}

/// 弧参数 t（0..=1）→ 角度。
fn arc_angle(a0: f64, span: f64, t: f64) -> f64 {
    a0 + span * t
}

/// 角度 α（[0,2π)）→ 弧参数 t（弧覆盖圈内等价角）。
fn arc_t_of_angle(a0: f64, span: f64, alpha: f64) -> f64 {
    // θ(t) = a0 + span·t；求 t 使 θ(t) ≡ alpha (mod 2π)。
    let rel = norm_angle(alpha - a0);
    if span <= EPS {
        return 0.0;
    }
    (rel / span).min(1.0)
}

/// 临界点排序后相邻区间中点采样判定 inside，合并连续 inside 段 → 角度区间。
fn segments_inside(
    ts: Vec<f64>,
    inside: impl Fn(f64) -> bool,
    a0: f64,
    span: f64,
) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    let mut i = 0;
    while i + 1 < ts.len() {
        let (t0, t1) = (ts[i], ts[i + 1]);
        // t0/t1 都算临界，避免浮点重合
        if (t1 - t0).abs() <= 1e-9 {
            i += 1;
            continue;
        }
        let mid = (t0 + t1) * 0.5;
        if inside(arc_angle(a0, span, mid)) {
            if let Some(last) = out.last_mut() {
                // 合并连续段（同一 inside 段被相邻临界点细分）
                if (last.1 - t0).abs() <= 1e-6 {
                    last.1 = t1;
                    i += 1;
                    continue;
                }
            }
            out.push((t0, t1));
        }
        i += 1;
    }
    out.into_iter()
        .map(|(s, e)| (arc_angle(a0, span, s), arc_angle(a0, span, e)))
        .collect()
}

// ── bulge 弧 ──────────────────────────────────────────────────────────────

/// 由顶点与 bulge 计算圆弧：返回 (圆心, 半径, 起始角, 终止角, 逆时针)。
/// 逆时针 span = 4·atan(|bulge|)；bulge>0 → 弧从 a 逆时针到 b；bulge<0 → 从 b 逆时针到 a
/// （等价于从 a 顺时针到 b）。起止角均为 [0,2π)。
pub fn bulge_arc(
    a: (f64, f64),
    b: (f64, f64),
    bulge: f64,
) -> Option<((f64, f64), f64, f64, f64, bool)> {
    if bulge.abs() <= EPS {
        return None;
    }
    let theta = 4.0 * bulge.atan();
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt();
    if len <= EPS {
        return None;
    }
    let half = theta * 0.5;
    let r = len / (2.0 * half.sin().abs()).max(EPS);
    // 圆心 = 弦中点 + 左手法向 × λ；λ = r·cos(half)·sign(bulge)。
    // （θ=±π 时 λ=0，半圆退化为中点，方向由 start/end 顺序表达。）
    let lam = r * half.cos() * if bulge > 0.0 { 1.0 } else { -1.0 };
    let (nx, ny) = (-dy / len, dx / len);
    let c = ((a.0 + b.0) * 0.5 + lam * nx, (a.1 + b.1) * 0.5 + lam * ny);
    let sa = norm_angle((a.1 - c.1).atan2(a.0 - c.0));
    let sb = norm_angle((b.1 - c.1).atan2(b.0 - c.0));
    // 起止角：bulge>0 → start=a（逆时针到 b）；bulge<0 → start=b（逆时针到 a）。
    if bulge > 0.0 {
        Some((c, r, sa, sa + norm_angle(sb - sa), true))
    } else {
        Some((c, r, sb, sb + norm_angle(sa - sb), false))
    }
}

// ── 裁剪入口 ──────────────────────────────────────────────────────────────

/// 放大变换：p' = (p − center) × k。
#[inline]
pub fn scale_pt(p: (f64, f64), center: (f64, f64), k: f64) -> (f64, f64) {
    ((p.0 - center.0) * k, (p.1 - center.1) * k)
}

/// 输出实体辅助。
fn mk_line(a: (f64, f64), b: (f64, f64), src: &Line) -> E {
    let mut e = E::Line(Line::from_coords(a.0, a.1, 0.0, b.0, b.1, 0.0));
    e.common_mut().layer = src.common.layer.clone();
    e.common_mut().color = src.common.color;
    e
}

fn mk_arc(c: (f64, f64), r: f64, a0: f64, a1: f64, src: &Arc) -> E {
    let mut e = E::Arc(Arc::from_center_radius_angles(Vector3::new(c.0, c.1, 0.0), r, a0, a1));
    e.common_mut().layer = src.common.layer.clone();
    e.common_mut().color = src.common.color;
    e
}

fn mk_circle(c: (f64, f64), r: f64, src: &Circle) -> E {
    let mut e = E::Circle(Circle::from_center_radius(Vector3::new(c.0, c.1, 0.0), r));
    e.common_mut().layer = src.common.layer.clone();
    e.common_mut().color = src.common.color;
    e
}

/// 圆实体裁剪（窗口内 0..=1 段）。
fn clip_circle_entity(win: &Win, c: &Circle, k: f64, origin: (f64, f64)) -> Vec<E> {
    let ctr = (c.center.x, c.center.y);
    let r = c.radius;
    if !win.contains(ctr) && win.circum_radius() > 0.0 {
        // 快速全外预判：圆心在外的凸窗口，若最近距离 > r 则无交。
        let outside = match win {
            Win::Circle { center: wc, radius: wr } => {
                let d = ((ctr.0 - wc.0).powi(2) + (ctr.1 - wc.1).powi(2)).sqrt();
                d - r > *wr + 1e-7
            }
            Win::Rect { min, max } => {
                let nx = (ctr.0 - min.0).max(0.0).max(min.0 - ctr.0);
                let ny = (ctr.1 - min.1).max(0.0).max(min.1 - ctr.1);
                let d = (nx * nx + ny * ny).sqrt();
                d > r + 1e-7
            }
        };
        if outside {
            return Vec::new();
        }
    }
    let segs = win.clip_circle(ctr, r);
    let cc = scale_pt(ctr, origin, k);
    let rr = r * k;
    segs.into_iter()
        .flat_map(|(s, e)| {
            if e - s >= std::f64::consts::TAU - 1e-6 {
                // 整圆在窗口内 → 保持 CIRCLE 语义
                vec![mk_circle(cc, rr, c)]
            } else {
                vec![mk_arc(cc, rr, s, e, &Arc {
                    common: c.common.clone(),
                    center: c.center,
                    radius: c.radius,
                    start_angle: 0.0,
                    end_angle: 0.0,
                    thickness: 0.0,
                    normal: c.normal,
                })]
            }
        })
        .collect()
}

/// 弧实体裁剪。
fn clip_arc_entity(win: &Win, a: &Arc, k: f64, origin: (f64, f64)) -> Vec<E> {
    let ctr = (a.center.x, a.center.y);
    let segs = win.clip_arc(ctr, a.radius, a.start_angle, a.end_angle);
    let cc = scale_pt(ctr, origin, k);
    let rr = a.radius * k;
    segs.into_iter()
        .map(|(s, e)| mk_arc(cc, rr, s, e, a))
        .collect()
}

/// 线实体裁剪。
fn clip_line_entity(win: &Win, l: &Line, k: f64, origin: (f64, f64)) -> Vec<E> {
    let a = (l.start.x, l.start.y);
    let b = (l.end.x, l.end.y);
    match win.clip_segment(a, b) {
        Some((pa, pb)) => vec![mk_line(scale_pt(pa, origin, k), scale_pt(pb, origin, k), l)],
        None => Vec::new(),
    }
}

/// LWPOLYLINE 裁剪：逐段输出 LINE/ARC。
fn clip_lwpolyline_entity(win: &Win, pl: &LwPolyline, k: f64, origin: (f64, f64)) -> Vec<E> {
    let mut out = Vec::new();
    let n = pl.vertices.len();
    if n == 0 {
        return out;
    }
    let at = |i: usize, pl: &LwPolyline| (pl.vertices[i].location.x, pl.vertices[i].location.y);
    let segs: Vec<(usize, usize, f64)> = (0..n)
        .filter_map(|i| {
            let j = (i + 1) % n;
            if !pl.is_closed && j == 0 {
                return None;
            }
            Some((i, j, pl.vertices[i].bulge))
        })
        .collect();
    for (i, j, bulge) in segs {
        let a = at(i, pl);
        let b = at(j, pl);
        if bulge.abs() <= EPS {
            // 直线段
            if let Some((pa, pb)) = win.clip_segment(a, b) {
                out.push(mk_line(
                    scale_pt(pa, origin, k),
                    scale_pt(pb, origin, k),
                    &Line::from_coords(a.0, a.1, 0.0, b.0, b.1, 0.0),
                ));
            }
        } else if let Some((c, r, s, e, _ccw)) = bulge_arc(a, b, bulge) {
            for (s0, e0) in win.clip_arc(c, r, s, e) {
                out.push(mk_arc(
                    scale_pt(c, origin, k),
                    r * k,
                    s0,
                    e0,
                    &Arc {
                        common: pl.common.clone(),
                        center: Vector3::new(c.0, c.1, 0.0),
                        radius: r,
                        start_angle: s,
                        end_angle: e,
                        thickness: 0.0,
                        normal: pl.normal,
                    },
                ));
            }
        }
    }
    out
}

/// 折线（heavy Polyline / Polyline2D）裁剪：与 LWPOLYLINE 同法。
/// `layer`/`color`：原多段线实体的图层与颜色（输出 LINE/ARC 保留）。
fn clip_polyline_entity(
    win: &Win,
    pts: &[(f64, f64)],
    bulges: &[f64],
    closed: bool,
    k: f64,
    origin: (f64, f64),
    layer: &str,
    color: Color,
) -> Vec<E> {
    let mut out = Vec::new();
    let n = pts.len();
    if n == 0 {
        return out;
    }
    let segs: Vec<(usize, usize, f64)> = (0..n)
        .filter_map(|i| {
            let j = (i + 1) % n;
            if !closed && j == 0 {
                return None;
            }
            Some((i, j, bulges.get(i).copied().unwrap_or(0.0)))
        })
        .collect();
    for (i, j, bulge) in segs {
        let a = pts[i];
        let b = pts[j];
        if bulge.abs() <= EPS {
            if let Some((pa, pb)) = win.clip_segment(a, b) {
                let mut e = E::Line(Line::from_coords(pa.0, pa.1, 0.0, pb.0, pb.1, 0.0));
                e.common_mut().layer = layer.to_string();
                e.common_mut().color = color;
                out.push(e);
            }
        } else if let Some((c, r, s, e, _ccw)) = bulge_arc(a, b, bulge) {
            for (s0, e0) in win.clip_arc(c, r, s, e) {
                let mut ae =
                    E::Arc(Arc::from_center_radius_angles(Vector3::new(c.0, c.1, 0.0), r, s0, e0));
                ae.common_mut().layer = layer.to_string();
                ae.common_mut().color = color;
                out.push(ae);
            }
        }
    }
    out
}

/// SPLINE 裁剪：fit_points（有则用）否则控制点折线近似 → 逐段裁。
fn clip_spline_entity(win: &Win, s: &Spline, k: f64, origin: (f64, f64)) -> Vec<E> {
    let pts: Vec<(f64, f64)> = if !s.fit_points.is_empty() {
        s.fit_points.iter().map(|p| (p.x, p.y)).collect()
    } else {
        s.control_points.iter().map(|p| (p.x, p.y)).collect()
    };
    clip_polyline_entity(
        win,
        &pts,
        &vec![0.0; pts.len()],
        false,
        k,
        origin,
        &s.common.layer,
        s.common.color,
    )
}

/// ELLIPSE 裁剪：等参采样折线 → 逐段裁。
fn clip_ellipse_entity(win: &Win, el: &Ellipse, k: f64, origin: (f64, f64)) -> Vec<E> {
    let ma = (el.major_axis.x, el.major_axis.y);
    let mi = (-ma.1 * el.minor_axis_ratio, ma.0 * el.minor_axis_ratio);
    let c = (el.center.x, el.center.y);
    let span = el.end_parameter - el.start_parameter;
    let n = 64usize;
    let mut pts = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let t = el.start_parameter + span * (i as f64) / (n as f64);
        pts.push((
            c.0 + ma.0 * t.cos() + mi.0 * t.sin(),
            c.1 + ma.1 * t.cos() + mi.1 * t.sin(),
        ));
    }
    clip_polyline_entity(
        win,
        &pts,
        &vec![0.0; pts.len()],
        false,
        k,
        origin,
        &el.common.layer,
        el.common.color,
    )
}

/// HATCH 裁剪：边界逐边裁剪后以 PolylineEdge 重建；全内则整体复制。
fn clip_hatch_entity(win: &Win, h: &Hatch, k: f64, origin: (f64, f64)) -> Vec<E> {
    use ocs_plugin_api::host::acadrust::entities::{
        BoundaryEdge, BoundaryPath, CircularArcEdge, EllipticArcEdge, HatchPattern, LineEdge,
        PolylineEdge,
    };
    // 全在窗口内 → 整体复制（坐标变换，pattern 间距不变）。
    let fully_inside = h.paths.iter().all(|p| {
        p.edges.iter().all(|e| edge_inside(win, e))
    });
    if fully_inside {
        let mut nh = h.clone();
        transform_hatch(&mut nh, k, origin);
        return vec![E::Hatch(nh)];
    }
    let mut nh = h.clone();
    nh.paths = Vec::new();
    nh.is_associative = false;
    for path in &h.paths {
        let mut np = BoundaryPath::with_flags(path.flags.clone());
        for e in &path.edges {
            let clipped: Vec<BoundaryEdge> = clip_edge(win, e, k, origin);
            np.edges.extend(clipped);
        }
        if !np.edges.is_empty() {
            np.boundary_handles.clear();
            nh.paths.push(np);
        }
    }
    if nh.paths.is_empty() {
        return Vec::new();
    }
    vec![E::Hatch(nh)]
}

/// 边界边是否完全在窗口内。
fn edge_inside(win: &Win, e: &BoundaryEdge) -> bool {
    match e {
        BoundaryEdge::Line(l) => win.contains((l.start.x, l.start.y)) && win.contains((l.end.x, l.end.y)),
        BoundaryEdge::CircularArc(a) => {
            let n = 24;
            let (s, e0) = if a.counter_clockwise {
                (a.start_angle, a.end_angle)
            } else {
                (a.end_angle, a.start_angle)
            };
            let span = norm_angle(e0 - s);
            (0..=n).all(|i| {
                let t = s + span * (i as f64) / (n as f64);
                win.contains((a.center.x + a.radius * t.cos(), a.center.y + a.radius * t.sin()))
            })
        }
        BoundaryEdge::EllipticArc(a) => {
            let ma = (a.major_axis_endpoint.x, a.major_axis_endpoint.y);
            let mi = (-ma.1 * a.minor_axis_ratio, ma.0 * a.minor_axis_ratio);
            let (s, e0) = if a.counter_clockwise {
                (a.start_angle, a.end_angle)
            } else {
                (a.end_angle, a.start_angle)
            };
            let span = e0 - s;
            (0..=24).all(|i| {
                let t = s + span * (i as f64) / 24.0;
                win.contains((
                    a.center.x + ma.0 * t.cos() + mi.0 * t.sin(),
                    a.center.y + ma.1 * t.cos() + mi.1 * t.sin(),
                ))
            })
        }
        BoundaryEdge::Spline(s) => {
            let pts = spline_edge_points(s);
            pts.iter().all(|p| win.contains(*p))
        }
        BoundaryEdge::Polyline(p) => {
            p.vertices
                .iter()
                .all(|v| win.contains((v.x, v.y)))
        }
    }
}

fn spline_edge_points(s: &ocs_plugin_api::host::acadrust::entities::SplineEdge) -> Vec<(f64, f64)> {
    s.fit_points
        .iter()
        .map(|p| (p.x, p.y))
        .chain(s.control_points.iter().map(|p| (p.x, p.y)))
        .collect()
}

/// 裁剪一条边界边 → 以 Line/Polyline 段输出（统一用 PolylineEdge 折线重建）。
fn clip_edge(win: &Win, e: &BoundaryEdge, k: f64, origin: (f64, f64)) -> Vec<BoundaryEdge> {
    match e {
        BoundaryEdge::Line(l) => {
            let a = (l.start.x, l.start.y);
            let b = (l.end.x, l.end.y);
            match win.clip_segment(a, b) {
                Some((pa, pb)) => vec![BoundaryEdge::Polyline(pl_edges(vec![
                    scale_pt(pa, origin, k),
                    scale_pt(pb, origin, k),
                ]))],
                None => Vec::new(),
            }
        }
        BoundaryEdge::CircularArc(a) => {
            let c = (a.center.x, a.center.y);
            let (s, e0) = if a.counter_clockwise {
                (a.start_angle, a.end_angle)
            } else {
                (a.end_angle, a.start_angle)
            };
            let mut out = Vec::new();
            for (s0, e0) in win.clip_arc(c, a.radius, s, e0) {
                // 采样折线（弧边界够密即可）
                let n = 16;
                let span = norm_angle(e0 - s0);
                let mut pts = Vec::new();
                for i in 0..=n {
                    let t = s0 + span * (i as f64) / (n as f64);
                    pts.push(scale_pt(
                        (c.0 + a.radius * t.cos(), c.1 + a.radius * t.sin()),
                        origin,
                        k,
                    ));
                }
                out.push(BoundaryEdge::Polyline(pl_edges(pts)));
            }
            out
        }
        BoundaryEdge::EllipticArc(a) => {
            let ma = (a.major_axis_endpoint.x, a.major_axis_endpoint.y);
            let mi = (-ma.1 * a.minor_axis_ratio, ma.0 * a.minor_axis_ratio);
            let (s, e0) = if a.counter_clockwise {
                (a.start_angle, a.end_angle)
            } else {
                (a.end_angle, a.start_angle)
            };
            let span = e0 - s;
            let mut out = Vec::new();
            let n = 16;
            for i in 0..n {
                let t0 = s + span * (i as f64) / (n as f64);
                let t1 = s + span * ((i + 1) as f64) / (n as f64);
                let p0 = (
                    a.center.x + ma.0 * t0.cos() + mi.0 * t0.sin(),
                    a.center.y + ma.1 * t0.cos() + mi.1 * t0.sin(),
                );
                let p1 = (
                    a.center.x + ma.0 * t1.cos() + mi.0 * t1.sin(),
                    a.center.y + ma.1 * t1.cos() + mi.1 * t1.sin(),
                );
                if let Some((pa, pb)) = win.clip_segment(p0, p1) {
                    out.push(BoundaryEdge::Polyline(pl_edges(vec![
                        scale_pt(pa, origin, k),
                        scale_pt(pb, origin, k),
                    ])));
                }
            }
            out
        }
        BoundaryEdge::Spline(s) => {
            let pts = spline_edge_points(s);
            let mut out = Vec::new();
            for w in pts.windows(2) {
                if let Some((pa, pb)) = win.clip_segment(w[0], w[1]) {
                    out.push(BoundaryEdge::Polyline(pl_edges(vec![
                        scale_pt(pa, origin, k),
                        scale_pt(pb, origin, k),
                    ])));
                }
            }
            out
        }
        BoundaryEdge::Polyline(p) => {
            let vs: Vec<(f64, f64, f64)> = p
                .vertices
                .iter()
                .map(|v| (v.x, v.y, v.z))
                .collect();
            let coords: Vec<(f64, f64)> = vs.iter().map(|v| (v.0, v.1)).collect();
            let mut out = Vec::new();
            let n = coords.len();
            if n < 2 {
                return out;
            }
            for i in 0..n {
                let j = (i + 1) % n;
                if !p.is_closed && j == 0 {
                    break;
                }
                let (a, b) = (coords[i], coords[j]);
                let bulge = vs[i].2;
                if bulge.abs() <= EPS {
                    if let Some((pa, pb)) = win.clip_segment(a, b) {
                        out.push(BoundaryEdge::Polyline(pl_edges(vec![
                            scale_pt(pa, origin, k),
                            scale_pt(pb, origin, k),
                        ])));
                    }
                } else if let Some((c, r, s0, e0, _ccw)) = bulge_arc(a, b, bulge) {
                    for (s1, e1) in win.clip_arc(c, r, s0, e0) {
                        let n2 = 16;
                        let span = norm_angle(e1 - s1);
                        let mut pts = Vec::new();
                        for m in 0..=n2 {
                            let t = s1 + span * (m as f64) / (n2 as f64);
                            pts.push(scale_pt(
                                (c.0 + r * t.cos(), c.1 + r * t.sin()),
                                origin,
                                k,
                            ));
                        }
                        out.push(BoundaryEdge::Polyline(pl_edges(pts)));
                    }
                }
            }
            out
        }
    }
}

fn pl_edges(pts: Vec<(f64, f64)>) -> PolylineEdge {
    let mut pe = PolylineEdge::new(vec![], false);
    for (x, y) in pts {
        pe.add_vertex(Vector2::new(x, y), 0.0);
    }
    pe
}

/// 整体复制变换（全内 HATCH）。
fn transform_hatch(h: &mut Hatch, k: f64, origin: (f64, f64)) {
    let tf = |p: &Vector2| Vector2::new(scale_pt((p.x, p.y), origin, k).0, scale_pt((p.x, p.y), origin, k).1);
    for path in &mut h.paths {
        for e in &mut path.edges {
            match e {
                BoundaryEdge::Line(l) => {
                    let (a, b) = (l.start.clone(), l.end.clone());
                    l.start = tf(&a);
                    l.end = tf(&b);
                }
                BoundaryEdge::CircularArc(a) => {
                    let c = a.center.clone();
                    a.center = tf(&c);
                    a.radius *= k;
                }
                BoundaryEdge::EllipticArc(a) => {
                    let c = a.center.clone();
                    a.center = tf(&c);
                    let ma = a.major_axis_endpoint.clone();
                    a.major_axis_endpoint = Vector2::new(ma.x * k, ma.y * k);
                }
                BoundaryEdge::Spline(_) => {}
                BoundaryEdge::Polyline(p) => {
                    for v in &mut p.vertices {
                        let (x, y) = (v.x, v.y);
                        let (nx, ny) = scale_pt((x, y), origin, k);
                        v.x = nx;
                        v.y = ny;
                    }
                }
            }
        }
    }
    h.seed_points = h
        .seed_points
        .iter()
        .map(|p| Vector2::new(scale_pt((p.x, p.y), origin, k).0, scale_pt((p.x, p.y), origin, k).1))
        .collect();
}

/// 主入口：裁剪一个实体。返回块成员（已放大变换）。
pub fn clip_entity(win: &Win, e: &E, k: f64, origin: (f64, f64)) -> Vec<E> {
    match e {
        E::Line(l) => clip_line_entity(win, l, k, origin),
        E::Arc(a) => clip_arc_entity(win, a, k, origin),
        E::Circle(c) => clip_circle_entity(win, c, k, origin),
        E::LwPolyline(p) => clip_lwpolyline_entity(win, p, k, origin),
        E::Polyline(p) => {
            let pts: Vec<(f64, f64)> = p
                .vertices
                .iter()
                .map(|v| (v.location.x, v.location.y))
                .collect();
            // 3D heavy polyline 无 bulge（直线段）。
            let bulges = vec![0.0; pts.len()];
            clip_polyline_entity(win, &pts, &bulges, p.flags.is_closed(), k, origin, &p.common.layer, p.common.color)
        }
        E::Polyline2D(p) => {
            let pts: Vec<(f64, f64)> = p
                .vertices
                .iter()
                .map(|v| (v.location.x, v.location.y))
                .collect();
            let bulges: Vec<f64> = p.vertices.iter().map(|v| v.bulge).collect();
            clip_polyline_entity(win, &pts, &bulges, p.flags.is_closed(), k, origin, &p.common.layer, p.common.color)
        }
        E::Spline(s) => clip_spline_entity(win, s, k, origin),
        E::Ellipse(el) => clip_ellipse_entity(win, el, k, origin),
        E::Hatch(h) => clip_hatch_entity(win, h, k, origin),
        _ => Vec::new(),
    }
}

/// 构造待扫描的 5 层名列表。
pub const CONTENT_LAYERS: [&str; 5] = [
    "1轮廓实线层",
    "2细线层",
    "3中心线层",
    "4虚线层",
    "5剖面线层",
];

/// 是否为放大内容层（与现有块成员使用的图层名一致，作容错比较）。
pub fn is_content_layer(layer: &str) -> bool {
    CONTENT_LAYERS.iter().any(|l| *l == layer)
}

// ── 测试 ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ocs_plugin_api::host::acadrust::entities::Arc as A;
    use ocs_plugin_api::host::acadrust::types::Vector3 as V3;

    fn circle_win() -> Win {
        Win::Circle { center: (0.0, 0.0), radius: 10.0 }
    }
    fn rect_win() -> Win {
        Win::Rect { min: (-5.0, -5.0), max: (5.0, 5.0) }
    }

    #[test]
    fn segment_circle_full_inside() {
        let w = circle_win();
        assert!(w.contains((1.0, 1.0)));
        // 全内
        let r = w.clip_segment((-3.0, 0.0), (5.0, 4.0)).unwrap();
        assert!((r.0.0 + 3.0).abs() < 1e-9 && r.1.0.abs() - 5.0 < 1e-9);
    }

    #[test]
    fn segment_circle_cross_clip() {
        let w = circle_win();
        let r = w.clip_segment((-20.0, 0.0), (20.0, 0.0)).unwrap();
        assert!((r.0 .0 + 10.0).abs() < 1e-9);
        assert!((r.1 .0 - 10.0).abs() < 1e-9);
        // 完全在外
        assert!(w.clip_segment((11.0, 0.0), (20.0, 0.0)).is_none());
    }

    #[test]
    fn segment_rect_clip() {
        let w = rect_win();
        let r = w.clip_segment((-100.0, 0.0), (100.0, 0.0)).unwrap();
        assert!((r.0 .0 + 5.0).abs() < 1e-9 && (r.1 .0 - 5.0).abs() < 1e-9);
        let r2 = w.clip_segment((2.0, 2.0), (4.0, 4.0)).unwrap();
        assert!((r2.0 .0 - 2.0).abs() < 1e-9);
        let r3 = w.clip_segment((-1.0, -100.0), (-1.0, 100.0)).unwrap();
        assert!((r3.0 .1 + 5.0).abs() < 1e-9 && (r3.1 .1 - 5.0).abs() < 1e-9);
        assert!(w.clip_segment((6.0, 0.0), (10.0, 0.0)).is_none());
    }

    #[test]
    fn circle_partial_becomes_arc() {
        // 圆心 (20,0) 半径 5：与圆窗口 {0,0,r10} 相离 → 全外
        let w = circle_win();
        let cc = E::Circle(ocs_plugin_api::host::acadrust::entities::Circle::from_center_radius(Vector3::new(20.0, 0.0, 0.0), 5.0));
        assert!(clip_entity(&w, &cc, 2.0, (0.0, 0.0)).is_empty());
        // 圆心 (8,0) r5：部分在窗口内（窗口 r10 包容 x∈[3,10] 段）→ ARC
        let c2 = E::Circle(ocs_plugin_api::host::acadrust::entities::Circle::from_center_radius(Vector3::new(8.0, 0.0, 0.0), 5.0));
        let out = clip_entity(&w, &c2, 2.0, (0.0, 0.0));
        assert_eq!(out.len(), 1);
        match &out[0] {
            E::Arc(a) => {
                // 圆心 (8,0) 与窗口 {0,0,10} 的交点：d=8, x=(64+25-100)/16=-0.6875 → 相对角 acos(-0.6875/8)=94.9°
                // 内部段 = 从左交点绕到右交点（经过 x≤10 的区域）
                assert!(a.radius - 10.0 < 1e-6);
                let span = norm_angle(a.end_angle - a.start_angle);
                assert!(span > 0.0 && span < std::f64::consts::TAU);
            }
            other => panic!("期望 ARC，实际 {other:?}"),
        }
    }

    #[test]
    fn arc_circle_window_single_segment() {
        // 圆心 (0,0) r5 位于窗口 {0,0,10} 内；弧 0°..300°（5π/3）全内 → 原样（span 300°）
        let w = circle_win();
        let arc = E::Arc(A::from_center_radius_angles(
            Vector3::new(0.0, 0.0, 0.0),
            5.0,
            0.0,
            5.0 * std::f64::consts::PI / 3.0,
        ));
        let out = clip_entity(&w, &arc, 2.0, (0.0, 0.0));
        assert_eq!(out.len(), 1);
        match &out[0] {
            E::Arc(a) => {
                assert!((a.start_angle - 0.0).abs() < 1e-6);
                assert!((a.end_angle - 5.0 * std::f64::consts::PI / 3.0).abs() < 1e-4);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn arc_rect_window_two_segments() {
        // 矩形窗口横条 [-10,8]×[-2,2]；弧圆心 (0,0) r=8 全圆 → 窗口内弧 = 上下 2 段。
        let w = Win::Rect { min: (-10.0, -2.0), max: (8.0, 2.0) };
        // 横条窗口 [-10,8]×[-2,2]；圆 r=8：圆周 inside = 右上瓣 + 左半整段 + 右下瓣 = 3 段。
        let segs = w.clip_arc((0.0, 0.0), 8.0, 0.0, std::f64::consts::TAU);
        assert_eq!(segs.len(), 3, "横条窗口内全圆弧应为 3 段: {segs:?}");
        for (s, e) in &segs {
            let mid = (s + e) * 0.5;
            let p = (8.0 * mid.cos(), 8.0 * mid.sin());
            assert!(w.contains(p), "段中点应在窗口内: {:?}", (p.0, p.1));
        }
    }

    #[test]
    fn polyline_bulge_clip() {
        // 带 bulge 的闭合多段线（圆角矩形）；窗口矩形 (2,2)-(8,8) 与四边相交 → 4 段。
        let mut pl = LwPolyline::new();
        pl.add_point_with_bulge(Vector2::new(0.0, 2.0), 0.0);
        pl.add_point_with_bulge(Vector2::new(10.0, 2.0), 0.0);
        pl.add_point_with_bulge(Vector2::new(10.0, 8.0), 0.0);
        pl.add_point_with_bulge(Vector2::new(0.0, 8.0), 0.0);
        pl.close();
        let w = Win::Rect { min: (2.0, 2.0), max: (8.0, 8.0) };
        let out = clip_entity(&w, &E::LwPolyline(pl), 2.0, (5.0, 5.0));
        // 只与上下两边相交 → 2 段；放大 ×2（中心 (5,5)）：底边 (2,2)-(8,2) → x∈[-6,6] 长 12
        assert_eq!(out.len(), 2);
        for e in &out {
            match e {
                E::Line(l) => {
                    let len = ((l.end.x - l.start.x).powi(2) + (l.end.y - l.start.y).powi(2)).sqrt();
                    assert!((len - 12.0).abs() < 1e-6, "边 {len}");
                }
                _ => panic!("期望 LINE"),
            }
        }
    }

    #[test]
    fn bulge_arc_math() {
        // a=(0,0) b=(10,0) bulge=1 → 半圆（逆时针 a→b = 下半），圆心 (5,0)，半径 5。
        let (c, r, s, e, ccw) = bulge_arc((0.0, 0.0), (10.0, 0.0), 1.0).unwrap();
        assert!((c.0 - 5.0).abs() < 1e-9 && c.1.abs() < 1e-9);
        assert!((r - 5.0).abs() < 1e-9);
        assert!(ccw);
        // 弧从 a 逆时针到 b：a 相对圆心角 π；span=π → s=π, e=2π。
        assert!((s - std::f64::consts::PI).abs() < 1e-6, "s={s}");
        assert!((e - std::f64::consts::TAU).abs() < 1e-6, "e={e}");
        // bulge=-1 → 顺时针半圆（从 a 经顶部到 b）：圆心仍在弦中点 (5,0)，
        // 方向由 s/e 表达：start=b(≈0) → end=π（逆时针 0→π 经过顶部）。
        let (c2, r2, s2, e2, ccw2) = bulge_arc((0.0, 0.0), (10.0, 0.0), -1.0).unwrap();
        assert!((c2.0 - 5.0).abs() < 1e-6 && c2.1.abs() < 1e-6);
        assert!((r2 - 5.0).abs() < 1e-9);
        assert!(!ccw2);
        assert!(s2.abs() < 1e-6 || (s2 - std::f64::consts::TAU).abs() < 1e-6, "s2={s2}");
        assert!((norm_angle(e2 - s2) - std::f64::consts::PI).abs() < 1e-6, "span2={}", e2 - s2);
    }

    #[test]
    fn fully_inside_kept_unchanged_count() {
        // 全内圆 → 一个 CIRCLE（×k）
        let w = circle_win();
        let c = E::Circle(ocs_plugin_api::host::acadrust::entities::Circle::from_center_radius(Vector3::new(2.0, 3.0, 0.0), 4.0));
        let out = clip_entity(&w, &c, 3.0, (1.0, 1.0));
        assert_eq!(out.len(), 1);
        match &out[0] {
            E::Circle(c2) => {
                assert!((c2.radius - 12.0).abs() < 1e-9);
                assert!((c2.center.x - 3.0).abs() < 1e-9);
                assert!((c2.center.y - 6.0).abs() < 1e-9);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn spline_and_ellipse_clip() {
        // SPLINE：fit 点折线裁剪 → 不崩且结果在窗口内
        let mut sp = Spline::new();
        sp.fit_points = vec![
            Vector3::new(-10.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(10.0, 0.0, 0.0),
        ];
        let w = circle_win();
        let out = clip_entity(&w, &E::Spline(sp), 2.0, (0.0, 0.0));
        assert!(!out.is_empty());
        // ELLIPSE 采样
        let el = E::Ellipse(ocs_plugin_api::host::acadrust::entities::Ellipse::from_center_axes(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(3.0, 0.0, 0.0),
            1.0,
        ));
        let out2 = clip_entity(&w, &el, 2.0, (0.0, 0.0));
        assert!(!out2.is_empty());
    }

    #[test]
    fn circle_intersect_angle_uses_arc_radius_not_d() {
        // 用户回归：局部放大2.dxf。原圆（相对引导中心）c=(89.68,-50.95) r=59.17，
        // 引导圆 c=(0,0) r=67.49。正确交点角（引导圆内弧段边界）
        // = base(150.38°) ± acos(x/r)：x=46.48 → off=38.26° → [112.12°,188.64°]。
        // bug 版用 acos(x/d)=63.22° → [87.16°,213.60°]（弧穿出放大框）。
        let hits = circle_circle_angles((89.68, -50.95), 59.17, (0.0, 0.0), 67.49);
        assert_eq!(hits.len(), 2, "{hits:?}");
        let deg = |a: f64| a * 180.0 / std::f64::consts::PI;
        let a0 = deg(hits[0]);
        let a1 = deg(hits[1]);
        let lo = a0.min(a1);
        let hi = a0.max(a1);
        assert!((lo - 112.12).abs() < 0.3, "lo={lo}");
        assert!((hi - 188.64).abs() < 0.3, "hi={hi}");
    }

    #[test]
    fn hatch_full_inside_copied() {
        // 全内 HATCH → 整体复制，pattern_scale 不变
        let mut h = Hatch::solid();
        let mut path = BoundaryPath::external();
        path.add_edge(BoundaryEdge::Polyline(pl_edges(vec![
            (0.0, 0.0),
            (4.0, 0.0),
            (4.0, 4.0),
            (0.0, 4.0),
        ])));
        h.paths.push(path);
        let w = circle_win();
        let out = clip_entity(&w, &E::Hatch(h), 2.0, (1.0, 1.0));
        assert_eq!(out.len(), 1);
        match &out[0] {
            E::Hatch(nh) => {
                assert_eq!(nh.paths.len(), 1);
                // 边界顶点已 ×k 变换
                let v = match &nh.paths[0].edges[0] {
                    BoundaryEdge::Polyline(p) => p.vertices[0],
                    _ => panic!(),
                };
                assert!((v.x - (-2.0)).abs() < 1e-9);
                assert!((v.y - (-2.0)).abs() < 1e-9);
            }
            _ => panic!(),
        }
    }
}
