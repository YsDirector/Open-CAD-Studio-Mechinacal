//! 标准件视图库的公共逻辑：图层模板 + 颜色→图层映射 + 建文档/建块。
//!
//! 这份文件是**试点工具与插件共用的**逻辑原型（插件侧将搬到
//! `crates/ocs_ocsm/src/parts.rs`）：把 ACM 代理图形解出的图元，按
//! 「颜色 + 线型」归到 OCSM 的 10 个图层上，颜色/线型/线宽一律改 ByLayer。

#[path = "proxy_decode.rs"]
pub mod dec;

use dec::Piece;
use ocs_plugin_api::host::acadrust;
use codec::entities::{Arc, Circle, EntityType, Line, LwPolyline, Text};
use codec::tables::{Layer, LineType, LineTypeElement};
use codec::types::{Color, LineWeight, Vector2, Vector3};
use codec::CadDocument;

/// OCSM 图层模板（与 `crates/ocs_ocsm/src/lib.rs::layer_defs()` 一致）。
pub const LAYERS: &[(&str, i16, &str, i16)] = &[
    ("1轮廓实线层", 7, "Continuous", 35),
    ("2细线层", 4, "Continuous", 18),
    ("3中心线层", 1, "CENTER2", 18),
    ("4虚线层", 6, "DASHED2", 18),
    ("5剖面线层", 2, "Continuous", 18),
    ("6文字层", 7, "Continuous", 18),
    ("7标注层", 7, "Continuous", 18),
    ("8符号标注层", 7, "Continuous", 18),
    ("9双点划线层", 7, "DIVIDE2", 18),
    ("10引导线层", 4, "Continuous", 13),
];

/// OCSM 需要的线型（与 `linetype_defs()` 一致）。
pub const LINETYPES: &[(&str, &str, &[f64])] = &[
    ("CENTER2", "Center (.5x)", &[19.05, -3.175, 3.175, -3.175]),
    ("DASHED2", "Dashed (.5x)", &[6.35, -3.175]),
    ("DIVIDE2", "Divide (.5x)", &[6.35, -3.175, 0.0, -3.175, 0.0, -3.175]),
];

/// 颜色（ACI）+ 线型索引 → OCSM 图层（用户 2026-09-14 定案）。
///
/// 实测依据（全库 4863 文件 / 11919 视图普查）：
/// - 7 白 + Continuous（10104）：粗实线（轮廓）
/// - 3 绿 + Continuous（1161）：细实线（螺纹牙底）
/// - 4 青 + AM_ISO09W050 点划线（643）：中心线
/// - 6 洋红 + Amconstr（485）：用户指定 → 虚线层
/// - 1 红（792）：剖面线（方斜垫圈 45° 密集线）/ 滚花密集线 → 剖面线层
/// - 253 灰：**基点标记圆（r=1.0）** → 符号标注层
/// - 256 ByLayer / 其它：兜底轮廓层（另按线型兜底）
pub fn layer_for(color: u16, linetype: u32) -> &'static str {
    match color {
        7 => "1轮廓实线层",
        3 => "2细线层",
        6 => "4虚线层",
        4 => "3中心线层",
        1 => "5剖面线层",
        253 => "8符号标注层",
        256 => "1轮廓实线层",
        _ => match linetype {
            5 => "3中心线层",
            6 => "4虚线层",
            _ => "1轮廓实线层",
        },
    }
}

/// 是否为"基点标记圆"：颜色 253 且 r≈1.00 mm（用户确认＝插入点标记）。
pub fn is_base_marker(piece: &Piece) -> bool {
    matches!(&piece.prim, dec::Prim::Circle { r, .. } if piece.attrs.color == 253 && (r - 1.0).abs() < 0.02)
}

/// 视图里基点标记圆的圆心（若有）。
pub fn base_marker_center(pieces: &[Piece]) -> Option<[f64; 3]> {
    pieces.iter().find(|p| is_base_marker(p)).and_then(|p| match &p.prim {
        dec::Prim::Circle { c, .. } => Some(*c),
        _ => None,
    })
}

/// 新建带 OCSM 图层/线型的空文档（版本 AC1015 = AutoCAD 2000）。
pub fn new_doc() -> CadDocument {
    let mut doc = CadDocument::new();
    for (name, desc, elems) in LINETYPES {
        let mut lt = LineType::new(*name);
        lt.description = (*desc).to_string();
        lt.elements = elems
            .iter()
            .map(|v| LineTypeElement { length: *v, complex: None })
            .collect();
        lt.pattern_length = lt.elements.iter().map(|e| e.length.abs()).sum();
        doc.line_types.add_or_replace(lt);
    }
    for (name, color, linetype, lw) in LAYERS {
        let mut l = Layer::new(*name);
        l.color = Color::from_index(*color);
        l.line_type = (*linetype).to_string();
        l.line_weight = LineWeight::from_value(*lw);
        doc.layers.add_or_replace(l);
    }
    doc
}

/// 把一个视图的图元写进文档：按映射表落层，颜色/线型/线宽全 ByLayer。
pub fn add_pieces(doc: &mut CadDocument, pieces: &[Piece]) -> usize {
    let mut n = 0;
    for p in pieces {
        let layer = layer_for(p.attrs.color, p.attrs.linetype);
        let Some(mut e) = to_entity(&p.prim) else { continue };
        {
            let c = e.common_mut();
            c.layer = layer.to_string();
            c.color = Color::ByLayer;
            c.linetype = "ByLayer".to_string();
            c.line_weight = LineWeight::ByLayer;
        }
        if doc.add_entity(e).is_ok() {
            n += 1;
        }
    }
    n
}

/// 代理图形图元 → acadrust 实体（点/文字暂不产出）。
pub fn to_entity(prim: &dec::Prim) -> Option<EntityType> {
    let v = |a: [f64; 3]| Vector3::new(a[0], a[1], a[2]);
    Some(match prim {
        dec::Prim::Line { a, b } => EntityType::Line(Line::from_points(v(*a), v(*b))),
        dec::Prim::Polyline { pts, closed } => {
            let pts2: Vec<Vector2> = pts.iter().map(|q| Vector2::new(q[0], q[1])).collect();
            let mut pl = LwPolyline::from_points(pts2);
            pl.is_closed = *closed;
            EntityType::LwPolyline(pl)
        }
        dec::Prim::Circle { c, r } => {
            if *r <= 0.0 {
                return None;
            }
            EntityType::Circle(Circle::from_center_radius(v(*c), *r))
        }
        dec::Prim::Arc { c, r, start_deg, end_deg } => {
            if *r <= 0.0 {
                return None;
            }
            EntityType::Arc(Arc::from_center_radius_angles(
                v(*c),
                *r,
                start_deg.to_radians(),
                end_deg.to_radians(),
            ))
        }
        dec::Prim::Point { .. } => return None, // GENIUS 标记点：不进库
        dec::Prim::Text { pos, text, height, rot_deg } => {
            let mut t = Text::new();
            t.insertion_point = v(*pos);
            t.value = text.clone();
            t.height = *height;
            t.rotation = rot_deg.to_radians();
            EntityType::Text(t)
        }
    })
}

/// 实体 → 代理图形图元（预览/校验用；bulge 弧段不展开）。
pub fn prims_from_entities(entities: &[codec::EntityType]) -> Vec<Piece> {
    use codec::entities::EntityType as E;
    let attrs = dec::PrimAttrs::default();
    let mut out = Vec::new();
    let p3 = |v: Vector3| [v.x, v.y, v.z];
    for e in entities {
        let prim = match e {
            E::Line(l) => dec::Prim::Line { a: p3(l.start), b: p3(l.end) },
            E::Circle(c) => dec::Prim::Circle { c: p3(c.center), r: c.radius },
            E::Arc(a) => dec::Prim::Arc {
                c: p3(a.center),
                r: a.radius,
                start_deg: a.start_angle,
                end_deg: a.end_angle,
            },
            E::LwPolyline(p) => dec::Prim::Polyline {
                pts: p.vertices.iter().map(|v| [v.location.x, v.location.y, 0.0]).collect(),
                closed: p.is_closed,
            },
            E::Polyline(p) => dec::Prim::Polyline {
                pts: p.vertices.iter().map(|v| [v.location.x, v.location.y, v.location.z]).collect(),
                closed: p.flags.is_closed(),
            },
            E::Polyline2D(p) => dec::Prim::Polyline {
                pts: p.vertices.iter().map(|v| [v.location.x, v.location.y, v.location.z]).collect(),
                closed: p.flags.is_closed(),
            },
            E::Text(t) => dec::Prim::Text {
                pos: p3(t.insertion_point),
                text: t.value.clone(),
                height: t.height,
                rot_deg: t.rotation.to_degrees(),
            },
            _ => continue,
        };
        out.push(Piece { prim, attrs });
    }
    out
}
