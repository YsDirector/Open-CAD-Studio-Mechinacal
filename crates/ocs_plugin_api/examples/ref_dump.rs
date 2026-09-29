//! 参考图分析工具：把用户的规范三视图（DWG/DXF）dump 成**精确数值清单（TSV）**
//! 并渲染一张 SVG 供人工核对。用于逐条反解标准件的画法。
//!
//! - 递归展开 INSERT（块内几何 + 基点/旋转/缩放变换）；
//! - TSV 列：`TYPE layer color lt a b c d …`，圆弧角度输出**度**；
//! - 同时输出 `<out>.svg`（整体包围盒自适应）。
//!
//! 用法：
//! ```sh
//! cargo run -q -p ocs_plugin_api --features host --example ref_dump -- <in.dwg|dxf> [out.tsv]
//! ```

use ocs_plugin_api::host::codec::io::dwg::DwgReader;
use ocs_plugin_api::host::codec::{CadDocument, EntityType};

#[derive(Clone, Debug)]
struct Prim {
    kind: String,
    layer: String,
    color: String,
    lt: String,
    nums: Vec<f64>,
    text: String,
}

fn color_str(c: &ocs_plugin_api::host::codec::types::Color) -> String {
    format!("{c:?}")
}

fn entities_of(doc: &CadDocument, name: &str) -> Vec<EntityType> {
    doc.entities_in_block(name).cloned().collect()
}

/// 展开一个实体（含 INSERT 递归），插入变换：缩放 → 旋转 → 平移。
fn expand(
    ents: &[EntityType],
    out: &mut Vec<Prim>,
    tf: &Transform2,
    doc: &CadDocument,
    depth: usize,
) {
    if depth > 8 {
        return;
    }
    for e in ents {
        let p = |x: f64, y: f64| tf.apply(x, y);
        match e {
            EntityType::Line(l) => {
                let a = p(l.start.x, l.start.y);
                let b = p(l.end.x, l.end.y);
                out.push(Prim {
                    kind: "LINE".into(),
                    layer: l.common.layer.clone(),
                    color: color_str(&l.common.color),
                    lt: l.common.linetype.clone(),
                    nums: vec![a.0, a.1, b.0, b.1],
                    text: String::new(),
                });
            }
            EntityType::Circle(c) => {
                let (cx, cy) = p(c.center.x, c.center.y);
                out.push(Prim {
                    kind: "CIRCLE".into(),
                    layer: c.common.layer.clone(),
                    color: color_str(&c.common.color),
                    lt: c.common.linetype.clone(),
                    nums: vec![cx, cy, c.radius * tf.scale],
                    text: String::new(),
                });
            }
            EntityType::Arc(a) => {
                let (cx, cy) = p(a.center.x, a.center.y);
                out.push(Prim {
                    kind: "ARC".into(),
                    layer: a.common.layer.clone(),
                    color: color_str(&a.common.color),
                    lt: a.common.linetype.clone(),
                    nums: vec![
                        cx,
                        cy,
                        a.radius * tf.scale,
                        a.start_angle.to_degrees() + tf.rot_deg,
                        a.end_angle.to_degrees() + tf.rot_deg,
                    ],
                    text: String::new(),
                });
            }
            EntityType::LwPolyline(pl) => {
                let mut nums = Vec::new();
                for v in &pl.vertices {
                    let (x, y) = p(v.location.x, v.location.y);
                    nums.push(x);
                    nums.push(y);
                    nums.push(v.bulge);
                }
                out.push(Prim {
                    kind: format!("LWPOLY{}", if pl.is_closed { "C" } else { "" }),
                    layer: pl.common.layer.clone(),
                    color: color_str(&pl.common.color),
                    lt: pl.common.linetype.clone(),
                    nums,
                    text: String::new(),
                });
            }
            EntityType::Polyline2D(pl) => {
                let mut nums = Vec::new();
                for v in &pl.vertices {
                    let (x, y) = p(v.location.x, v.location.y);
                    nums.push(x);
                    nums.push(y);
                    nums.push(0.0);
                }
                out.push(Prim {
                    kind: format!("POLY{}", if pl.flags.is_closed() { "C" } else { "" }),
                    layer: pl.common.layer.clone(),
                    color: color_str(&pl.common.color),
                    lt: pl.common.linetype.clone(),
                    nums,
                    text: String::new(),
                });
            }
            EntityType::Text(t) => {
                let (x, y) = p(t.insertion_point.x, t.insertion_point.y);
                out.push(Prim {
                    kind: "TEXT".into(),
                    layer: t.common.layer.clone(),
                    color: color_str(&t.common.color),
                    lt: t.common.linetype.clone(),
                    nums: vec![x, y, t.height],
                    text: t.value.clone(),
                });
            }
            EntityType::MText(mt) => {
                out.push(Prim {
                    kind: "MTEXT".into(),
                    layer: mt.common.layer.clone(),
                    color: color_str(&mt.common.color),
                    lt: mt.common.linetype.clone(),
                    nums: vec![mt.insertion_point.x, mt.insertion_point.y, mt.height],
                    text: mt.value.clone(),
                });
            }
            EntityType::Dimension(d) => {
                let b = d.base();
                out.push(Prim {
                    kind: "DIM".into(),
                    layer: b.common.layer.clone(),
                    color: color_str(&b.common.color),
                    lt: b.common.linetype.clone(),
                    nums: vec![b.text_middle_point.x, b.text_middle_point.y, 0.0],
                    text: if b.text.is_empty() {
                        b.user_text.clone().unwrap_or_default()
                    } else {
                        b.text.clone()
                    },
                });
            }
            EntityType::Insert(ins) => {
                let sc = ins.x_scale();
                let sub = Transform2 {
                    ox: ins.insert_point.x,
                    oy: ins.insert_point.y,
                    rot_deg: ins.rotation.to_degrees(),
                    scale: sc * tf.scale,
                    rx: tf.rx,
                    ry: tf.ry,
                };
                let inner = entities_of(doc, &ins.block_name);
                expand(&inner, out, &sub, doc, depth + 1);
            }
            other => {
                let name = format!("{other:?}");
                let name = name.split(['(', ' ', '{']).next().unwrap_or("?").to_string();
                if !name.starts_with("Unknown") {
                    out.push(Prim {
                        kind: format!("<{name}>"),
                        layer: String::new(),
                        color: String::new(),
                        lt: String::new(),
                        nums: vec![],
                        text: String::new(),
                    });
                }
            }
        }
    }
}

/// 2D 相似变换（该工具只处理平面视图）。
#[derive(Clone, Copy)]
struct Transform2 {
    ox: f64,
    oy: f64,
    rot_deg: f64,
    scale: f64,
    /// 父变换的旋转/缩放（用于累计，保持简化：只支持等比+旋转+平移）
    rx: f64,
    ry: f64,
}

impl Default for Transform2 {
    fn default() -> Self {
        Self { ox: 0.0, oy: 0.0, rot_deg: 0.0, scale: 1.0, rx: 0.0, ry: 0.0 }
    }
}

impl Transform2 {
    fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let (s, c) = self.rot_deg.to_radians().sin_cos();
        let sx = x * self.scale;
        let sy = y * self.scale;
        (self.ox + sx * c - sy * s, self.oy + sx * s + sy * c)
    }
}

fn main() {
    let mut a = std::env::args().skip(1);
    let src = a.next().expect("usage: ref_dump <in.dwg|dxf> [out.tsv]");
    let out = a.next().unwrap_or_else(|| format!("{src}.tsv"));

    let doc = if src.to_ascii_lowercase().ends_with(".dxf") {
        ocs_plugin_api::host::codec::io::dxf::DxfReader::from_file(&src)
            .and_then(|r| r.read())
            .expect("读 DXF")
    } else {
        DwgReader::from_file(&src).and_then(|mut r| r.read()).expect("读 DWG")
    };

    let mut prims = Vec::new();
    let ents: Vec<EntityType> = doc.entities().cloned().collect();
    expand(&ents, &mut prims, &Transform2::default(), &doc, 0);

    // TSV
    let mut tsv = String::from("type\tlayer\tcolor\tlinetype\tdata\ttext\n");
    for p in &prims {
        tsv.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            p.kind,
            p.layer,
            p.color,
            p.lt,
            p.nums.iter().map(|v| fmt(*v)).collect::<Vec<_>>().join(" "),
            p.text.replace('\n', " ")
        ));
    }
    std::fs::write(&out, &tsv).unwrap();
    println!("图元 {} → {out}", prims.len());

    // 统计
    use std::collections::BTreeMap;
    let mut hist: BTreeMap<String, usize> = BTreeMap::new();
    for p in &prims {
        *hist.entry(format!("{} | {} | {}", p.kind, p.layer, p.color)).or_insert(0) += 1;
    }
    for (k, v) in &hist {
        println!("  {v:4}  {k}");
    }

    // SVG（包围盒自适应）
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    let mut bb = |x: f64, y: f64| {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    };
    for p in &prims {
        match p.kind.as_str() {
            "LINE" => {
                bb(p.nums[0], p.nums[1]);
                bb(p.nums[2], p.nums[3]);
            }
            "CIRCLE" | "ARC" => {
                bb(p.nums[0] - p.nums[2], p.nums[1] - p.nums[2]);
                bb(p.nums[0] + p.nums[2], p.nums[1] + p.nums[2]);
            }
            k if k.starts_with("LWPOLY") || k.starts_with("POLY") => {
                for c in p.nums.chunks(3) {
                    if c.len() >= 2 {
                        bb(c[0], c[1]);
                    }
                }
            }
            "TEXT" => bb(p.nums[0], p.nums[1]),
            _ => {}
        }
    }
    if x0 > x1 {
        (x0, y0, x1, y1) = (0.0, 0.0, 10.0, 10.0);
    }
    let (w, h) = (x1 - x0, y1 - y0);
    let pad = (w.max(h) * 0.06).max(2.0);
    let px = 1000.0;
    let sc = px / (w + 2.0 * pad).max(1e-6);
    let py = (h + 2.0 * pad) * sc;
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{px:.0}" height="{py:.0}" viewBox="0 0 {px:.0} {py:.0}"><rect width="100%" height="100%" fill="#fff"/><g stroke="#111" fill="none" stroke-width="1.4">"##
    );
    let t = |x: f64, y: f64| ((x - x0 + pad) * sc, py - (y - y0 + pad) * sc);
    for p in &prims {
        let col = match p.color.as_str() {
            s if s.contains("Green") => "#00a000",
            s if s.contains("Cyan") => "#0088aa",
            s if s.contains("Red") => "#cc0000",
            s if s.contains("Magenta") => "#aa00aa",
            _ => "#111",
        };
        match p.kind.as_str() {
            "LINE" => {
                let a = t(p.nums[0], p.nums[1]);
                let b = t(p.nums[2], p.nums[3]);
                svg.push_str(&format!(
                    r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{col}"/>"#,
                    a.0, a.1, b.0, b.1
                ));
            }
            "CIRCLE" => {
                let c = t(p.nums[0], p.nums[1]);
                svg.push_str(&format!(
                    r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" stroke="{col}"/>"#,
                    c.0,
                    c.1,
                    p.nums[2] * sc
                ));
            }
            "ARC" => {
                let (cx, cy, r, a1, a2) = (p.nums[0], p.nums[1], p.nums[2], p.nums[3], p.nums[4]);
                let c = t(cx, cy);
                let (s, e) = (a1.to_radians(), a2.to_radians());
                let p1 = (c.0 + r * sc * s.cos(), c.1 - r * sc * s.sin());
                let p2 = (c.0 + r * sc * e.cos(), c.1 - r * sc * e.sin());
                let mut sweep = (a2 - a1) % 360.0;
                if sweep <= 0.0 {
                    sweep += 360.0;
                }
                let large = if sweep > 180.0 { 1 } else { 0 };
                svg.push_str(&format!(
                    r#"<path d="M {:.2} {:.2} A {:.2} {:.2} 0 {large} 0 {:.2} {:.2}" stroke="{col}"/>"#,
                    p1.0,
                    p1.1,
                    r * sc,
                    r * sc,
                    p2.0,
                    p2.1
                ));
            }
            k if k.starts_with("LWPOLY") || k.starts_with("POLY") => {
                let mut d = String::new();
                for c in p.nums.chunks(3) {
                    if c.len() >= 2 {
                        let q = t(c[0], c[1]);
                        d.push_str(&format!(
                            "{}{:.2} {:.2} ",
                            if d.is_empty() { "M " } else { "L " },
                            q.0,
                            q.1
                        ));
                    }
                }
                if p.kind.ends_with('C') {
                    d.push('Z');
                }
                svg.push_str(&format!(r#"<path d="{d}" stroke="{col}"/>"#));
            }
            _ => {}
        }
    }
    svg.push_str("</g></svg>");
    let svg_path = format!("{}.svg", out.trim_end_matches(".tsv"));
    std::fs::write(&svg_path, svg).unwrap();
    println!("SVG → {svg_path}");
}

fn fmt(v: f64) -> String {
    let r = (v * 10000.0).round() / 10000.0;
    if r == 0.0 {
        "0".into()
    } else {
        format!("{r}")
    }
}
