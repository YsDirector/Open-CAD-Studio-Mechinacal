//! 把一个 DWG 的所有"视图"渲染成 SVG（人工核对/预览用）。
//!
//! - ACM 标准件（几何在 `STDPART2D` 代理图形里）→ 每个对象一个视图
//! - 普通 DWG（图元直接在模型空间）→ 合成一个视图
//!
//! 用法：
//! ```sh
//! cargo run -q -p ocs_plugin_api --features host --example proxy_render -- <in.dwg> <out.svg>
//! PROXY_LIST=1 …   # 顺便逐条打印图元（颜色/线型/几何）
//! ```

#[path = "kit/part_lib.rs"]
mod part_lib;
#[path = "kit/svg_render.rs"]
mod svg_render;

use ocs_plugin_api::host::codec::io::dwg::DwgReader;
use ocs_plugin_api::host::codec::EntityType;
use part_lib::dec::{self, Prim};

fn main() {
    let mut args = std::env::args().skip(1);
    let src = args.next().expect("usage: proxy_render <in.dwg> <out.svg>");
    let out = args.next().expect("usage: proxy_render <in.dwg> <out.svg>");

    let doc = DwgReader::from_file(&src).and_then(|mut r| r.read()).expect("read dwg");
    let mut views: Vec<svg_render::View> = Vec::new();
    let mut plain: Vec<EntityType> = Vec::new();
    for e in doc.entities() {
        match e {
            EntityType::Unknown(u) if u.dxf_name.eq_ignore_ascii_case("STDPART2D") => {
                let d = dec::decode(u.common.graphic_data.as_deref().unwrap_or(&[]));
                // 标注尺寸时排除中心线（否则中心线出头会把尺寸撑大，误导读图）
                let solid: Vec<_> = d
                    .pieces
                    .iter()
                    .filter(|p| part_lib::layer_for(p.attrs.color, p.attrs.linetype) != "3中心线层")
                    .cloned()
                    .collect();
                let bb = svg_render::bbox(&solid)
                    .or_else(|| d.bbox_xy())
                    .unwrap_or([0.0, 0.0, 0.0, 0.0]);
                views.push(svg_render::View {
                    caption: format!(
                        "视角 {} · {} 个图元 · 外形 {:.1}×{:.1} mm",
                        views.len() + 1,
                        d.pieces.len(),
                        bb[2] - bb[0],
                        bb[3] - bb[1]
                    ),
                    pieces: d.pieces,
                    emphasize: false,
                });
            }
            EntityType::Point(_) => {}
            other => plain.push(other.clone()),
        }
    }
    if views.is_empty() {
        let pieces = part_lib::prims_from_entities(&plain);
        views.push(svg_render::View {
            caption: format!("模型空间 · {} 个图元", pieces.len()),
            pieces,
            emphasize: false,
        });
    }

    let svg = svg_render::render_svg(&src, &views);
    std::fs::write(&out, svg).expect("write svg");
    eprintln!("已写出 {out}（视图 {} 个）", views.len());
    if std::env::var("PROXY_LIST").is_ok() {
        for (i, v) in views.iter().enumerate() {
            eprintln!("  视角 {}: {} 个图元", i + 1, v.pieces.len());
            for (j, p) in v.pieces.iter().enumerate() {
                eprintln!(
                    "     {j:3} 层={:<12} 颜色={:<4} 线型={:<3} 线宽={:<4} {}",
                    part_lib::layer_for(p.attrs.color, p.attrs.linetype),
                    p.attrs.color,
                    p.attrs.linetype,
                    p.attrs.lineweight,
                    describe(&p.prim)
                );
            }
        }
    }
}

fn describe(p: &Prim) -> String {
    match p {
        Prim::Line { a, b } => format!("LINE   ({:.2},{:.2}) → ({:.2},{:.2})", a[0], a[1], b[0], b[1]),
        Prim::Polyline { pts, closed } => format!(
            "PLINE  {}点 closed={} 首({:.2},{:.2}) 末({:.2},{:.2})",
            pts.len(),
            closed,
            pts[0][0],
            pts[0][1],
            pts[pts.len() - 1][0],
            pts[pts.len() - 1][1]
        ),
        Prim::Circle { c, r } => format!("CIRCLE 圆心({:.2},{:.2}) r={:.2}", c[0], c[1], r),
        Prim::Arc { c, r, start_deg, end_deg } => format!(
            "ARC    圆心({:.2},{:.2}) r={:.2} {:.1}°→{:.1}°",
            c[0], c[1], r, start_deg, end_deg
        ),
        Prim::Point { p } => format!("POINT  ({:.2},{:.2})", p[0], p[1]),
        Prim::Text { pos, text, .. } => format!("TEXT   {text:?} @({:.2},{:.2})", pos[0], pos[1]),
    }
}
