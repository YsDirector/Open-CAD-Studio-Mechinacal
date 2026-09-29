//! 视图 → SVG 渲染（选择器页预览 / 人工核对共用）。
//!
//! 与坐标无关：按每个视图的包围盒自动缩放居中，颜色/线型沿用图元属性。

use crate::part_lib::dec::{self, aci_rgb, Prim};

/// 一个待渲染的视图。
pub struct View {
    pub caption: String,
    pub pieces: Vec<dec::Piece>,
    /// 选中/高亮（页面里标出"当前视图"用），此处直接换描边宽度。
    pub emphasize: bool,
}

pub fn render_svg(title: &str, views: &[View]) -> String {
    const PANEL_W: f64 = 460.0;
    const PANEL_H: f64 = 340.0;
    const PAD: f64 = 24.0;
    let cols = views.len().min(3).max(1);
    let rows = views.len().div_ceil(cols).max(1);
    let width = PAD + cols as f64 * (PANEL_W + PAD);
    let height = PAD + rows as f64 * (PANEL_H + PAD + 28.0) + 26.0;

    let mut s = String::new();
    s.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
<rect width="{width}" height="{height}" fill="#ffffff"/>
<style>
  text {{ font-family: sans-serif; }}
  .t {{ font-size: 15px; font-weight: 600; fill: #111; }}
  .c {{ font-size: 12px; fill: #444; }}
</style>
<text x="{PAD}" y="22" class="t">{}</text>
"##,
        esc(title)
    ));

    for (i, v) in views.iter().enumerate() {
        let cx = PAD + (i % cols) as f64 * (PANEL_W + PAD);
        let cy = 34.0 + (i / cols) as f64 * (PANEL_H + PAD + 28.0);
        s.push_str(&format!(
            r##"<rect x="{cx}" y="{cy}" width="{PANEL_W}" height="{PANEL_H}" fill="#fbfbfb" stroke="#ddd"/>
"##
        ));
        let bb = bbox(&v.pieces).unwrap_or([0.0, 0.0, 1.0, 1.0]);
        let (x0, y0, x1, y1) = (bb[0], bb[1], bb[2], bb[3]);
        let (w, h) = ((x1 - x0).max(1e-6), (y1 - y0).max(1e-6));
        let k = ((PANEL_W - 2.0 * PAD) / w).min((PANEL_H - 2.0 * PAD) / h);
        let tx = |x: f64| cx + PAD + (x - x0) * k;
        let ty_ = |y: f64| cy + PANEL_H - PAD - (y - y0) * k;
        s.push_str(&format!(
            r##"<text x="{}" y="{}" class="c">{}</text>
"##,
            cx + 8.0,
            cy + PANEL_H + 16.0,
            esc(&v.caption)
        ));
        let base_lw = if v.emphasize { 1.9 } else { 1.4 };
        for p in &v.pieces {
            let (r, g, b) = aci_rgb(p.attrs.color);
            let color = format!("rgb({r},{g},{b})");
            let dash = if v.emphasize { "" } else { dash_for(p.attrs.linetype) };
            let lw = base_lw;
            match &p.prim {
                Prim::Line { a, b: b2 } => s.push_str(&format!(
                    r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{color}" stroke-width="{lw}" fill="none" {dash}/>
"##,
                    tx(a[0]),
                    ty_(a[1]),
                    tx(b2[0]),
                    ty_(b2[1])
                )),
                Prim::Polyline { pts, closed } => {
                    let d: Vec<String> = pts
                        .iter()
                        .enumerate()
                        .map(|(j, q)| {
                            format!("{} {:.2} {:.2}", if j == 0 { "M" } else { "L" }, tx(q[0]), ty_(q[1]))
                        })
                        .collect();
                    let d = d.join(" ") + if *closed { " Z" } else { "" };
                    s.push_str(&format!(
                        r##"<path d="{d}" stroke="{color}" stroke-width="{lw}" fill="none" {dash}/>
"##
                    ));
                }
                Prim::Circle { c, r } => s.push_str(&format!(
                    r##"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" stroke="{color}" stroke-width="{lw}" fill="none" {dash}/>
"##,
                    tx(c[0]),
                    ty_(c[1]),
                    r * k
                )),
                Prim::Arc { c, r, start_deg, end_deg } => {
                    let a0 = -start_deg;
                    let a1 = -end_deg;
                    let sweep = a1 - a0;
                    let large = if sweep.abs() > 180.0 { 1 } else { 0 };
                    let dir = if sweep >= 0.0 { 1 } else { 0 };
                    let (sx, sy) = (
                        tx(c[0]) + r * k * a0.to_radians().cos(),
                        ty_(c[1]) + r * k * a0.to_radians().sin(),
                    );
                    let (ex, ey) = (
                        tx(c[0]) + r * k * a1.to_radians().cos(),
                        ty_(c[1]) + r * k * a1.to_radians().sin(),
                    );
                    s.push_str(&format!(
                        r##"<path d="M {sx:.2} {sy:.2} A {:.2} {:.2} 0 {large} {dir} {ex:.2} {ey:.2}" stroke="{color}" stroke-width="{lw}" fill="none" {dash}/>
"##,
                        r * k,
                        r * k
                    ));
                }
                Prim::Point { p: q } => s.push_str(&format!(
                    r##"<circle cx="{:.2}" cy="{:.2}" r="2" fill="{color}"/>
"##,
                    tx(q[0]),
                    ty_(q[1])
                )),
                Prim::Text { pos, text, height, rot_deg } => s.push_str(&format!(
                    r##"<text x="{:.2}" y="{:.2}" font-size="{:.2}" fill="{color}" transform="rotate({:.1} {:.2} {:.2})">{}</text>
"##,
                    tx(pos[0]),
                    ty_(pos[1]),
                    (height * k).max(6.0),
                    -rot_deg,
                    tx(pos[0]),
                    ty_(pos[1]),
                    esc(text)
                )),
            }
        }
    }
    s.push_str("</svg>\n");
    s
}

/// 图元集合的 xy 包围盒 [xmin, ymin, xmax, ymax]。
pub fn bbox(pieces: &[dec::Piece]) -> Option<[f64; 4]> {
    let mut bb: Option<[f64; 4]> = None;
    let mut add = |x: f64, y: f64, bb: &mut Option<[f64; 4]>| {
        if !x.is_finite() || !y.is_finite() || x.abs() > 1e7 || y.abs() > 1e7 {
            return;
        }
        let e = bb.get_or_insert([x, y, x, y]);
        e[0] = e[0].min(x);
        e[1] = e[1].min(y);
        e[2] = e[2].max(x);
        e[3] = e[3].max(y);
    };
    for p in pieces {
        match &p.prim {
            Prim::Line { a, b } => {
                add(a[0], a[1], &mut bb);
                add(b[0], b[1], &mut bb);
            }
            Prim::Polyline { pts, .. } => {
                for q in pts {
                    add(q[0], q[1], &mut bb);
                }
            }
            Prim::Circle { c, r } | Prim::Arc { c, r, .. } => {
                add(c[0] - r, c[1] - r, &mut bb);
                add(c[0] + r, c[1] + r, &mut bb);
            }
            Prim::Point { p } => add(p[0], p[1], &mut bb),
            Prim::Text { pos, .. } => add(pos[0], pos[1], &mut bb),
        }
    }
    bb
}

fn dash_for(linetype: u32) -> &'static str {
    match linetype {
        5 => "stroke-dasharray=\"14 4 3 4\"",
        6 => "stroke-dasharray=\"10 3 2 3\"",
        _ => "",
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
