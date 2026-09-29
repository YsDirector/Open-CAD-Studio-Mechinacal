//! ACM（AutoCAD Mechanical）`STDPART2D` 标准件的**代理图形**解码器。
//!
//! 背景：`~/桌面/GB/标准件库/` 里的标准件 DWG 是 ACM 参数化对象
//! （`STDPART2D`，几何由 ACM 运行时生成），acadrust/OCS 读不出普通图元。
//! 但这些对象**自带代理图形**（`EntityCommon::graphic_data`，即 plain AutoCAD
//! 也能显示的那份缓存），格式是 ODA 规范里的 "Proxy Entity Graphics" 记录流：
//!
//! ```text
//! 头 8 字节: total_size(u32) + record_count(u32)
//! 每条记录: size(u32) + type(u32) + payload(size-8)
//! ```
//!
//! 本模块把它解成与坐标系统无关的图元列表（含颜色/线型/线宽属性），
//! 供 OCSM 插件建块插入、以及在选择器页里画预览。
//! 记录类型表取自 ezdxf 的 `ProxyGraphicTypes`（权威）。

/// 代理图形记录类型。
pub mod ty {
    pub const EXTENTS: u32 = 1;
    pub const CIRCLE: u32 = 2;
    pub const CIRCLE_3P: u32 = 3;
    pub const CIRCULAR_ARC: u32 = 4;
    pub const CIRCULAR_ARC_3P: u32 = 5;
    pub const POLYLINE: u32 = 6;
    pub const POLYGON: u32 = 7;
    pub const MESH: u32 = 8;
    pub const SHELL: u32 = 9;
    pub const TEXT: u32 = 10;
    pub const TEXT2: u32 = 11;
    pub const XLINE: u32 = 12;
    pub const RAY: u32 = 13;
    pub const ATTRIBUTE_COLOR: u32 = 14;
    pub const ATTRIBUTE_LAYER: u32 = 16;
    pub const ATTRIBUTE_LINETYPE: u32 = 18;
    pub const ATTRIBUTE_MARKER: u32 = 19;
    pub const ATTRIBUTE_FILL: u32 = 20;
    pub const ATTRIBUTE_TRUE_COLOR: u32 = 22;
    pub const ATTRIBUTE_LINEWEIGHT: u32 = 23;
    pub const ATTRIBUTE_LTSCALE: u32 = 24;
    pub const ATTRIBUTE_THICKNESS: u32 = 25;
    pub const ATTRIBUTE_PLOT_STYLE_NAME: u32 = 26;
    pub const PUSH_CLIP: u32 = 27;
    pub const POP_CLIP: u32 = 28;
    pub const PUSH_MATRIX: u32 = 29;
    pub const PUSH_MATRIX2: u32 = 30;
    pub const POP_MATRIX: u32 = 31;
    pub const POLYLINE_WITH_NORMALS: u32 = 32;
    pub const LWPOLYLINE: u32 = 33;
    pub const ATTRIBUTE_MATERIAL: u32 = 34;
    pub const ATTRIBUTE_MAPPER: u32 = 35;
    pub const UNICODE_TEXT: u32 = 36;
    pub const UNICODE_TEXT2: u32 = 38;
    pub const ELLIPTIC_ARC: u32 = 44;

    pub fn name(t: u32) -> &'static str {
        match t {
            EXTENTS => "EXTENTS",
            CIRCLE => "CIRCLE",
            CIRCLE_3P => "CIRCLE_3P",
            CIRCULAR_ARC => "CIRCULAR_ARC",
            CIRCULAR_ARC_3P => "CIRCULAR_ARC_3P",
            POLYLINE => "POLYLINE",
            POLYGON => "POLYGON",
            MESH => "MESH",
            SHELL => "SHELL",
            TEXT => "TEXT",
            TEXT2 => "TEXT2",
            XLINE => "XLINE",
            RAY => "RAY",
            ATTRIBUTE_COLOR => "ATTRIBUTE_COLOR",
            ATTRIBUTE_LAYER => "ATTRIBUTE_LAYER",
            ATTRIBUTE_LINETYPE => "ATTRIBUTE_LINETYPE",
            ATTRIBUTE_MARKER => "ATTRIBUTE_MARKER",
            ATTRIBUTE_FILL => "ATTRIBUTE_FILL",
            ATTRIBUTE_TRUE_COLOR => "ATTRIBUTE_TRUE_COLOR",
            ATTRIBUTE_LINEWEIGHT => "ATTRIBUTE_LINEWEIGHT",
            ATTRIBUTE_LTSCALE => "ATTRIBUTE_LTSCALE",
            ATTRIBUTE_THICKNESS => "ATTRIBUTE_THICKNESS",
            ATTRIBUTE_PLOT_STYLE_NAME => "ATTRIBUTE_PLOT_STYLE_NAME",
            PUSH_CLIP => "PUSH_CLIP",
            POP_CLIP => "POP_CLIP",
            PUSH_MATRIX => "PUSH_MATRIX",
            PUSH_MATRIX2 => "PUSH_MATRIX2",
            POP_MATRIX => "POP_MATRIX",
            POLYLINE_WITH_NORMALS => "POLYLINE_WITH_NORMALS",
            LWPOLYLINE => "LWPOLYLINE",
            ATTRIBUTE_MATERIAL => "ATTRIBUTE_MATERIAL",
            ATTRIBUTE_MAPPER => "ATTRIBUTE_MAPPER",
            UNICODE_TEXT => "UNICODE_TEXT",
            UNICODE_TEXT2 => "UNICODE_TEXT2",
            ELLIPTIC_ARC => "ELLIPTIC_ARC",
            _ => "UNKNOWN",
        }
    }
}

/// 一个解出来的图元（坐标即图纸坐标，暂不做 OCS/矩阵变换）。
#[derive(Debug, Clone, PartialEq)]
pub enum Prim {
    Line { a: [f64; 3], b: [f64; 3] },
    Polyline { pts: Vec<[f64; 3]>, closed: bool },
    Circle { c: [f64; 3], r: f64 },
    Arc { c: [f64; 3], r: f64, start_deg: f64, end_deg: f64 },
    Point { p: [f64; 3] },
    Text { pos: [f64; 3], text: String, height: f64, rot_deg: f64 },
}

/// 图元随附的绘图属性（代理图形里的"当前状态"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimAttrs {
    /// ACI 颜色索引（256 = ByLayer）。
    pub color: u16,
    /// 线型表索引（0 = ByBlock/Continuous；0xFFFF_FFFF = 无效）。
    pub linetype: u32,
    /// 线宽（DWG 值；-3 = ByLayer…此处保留原值）。
    pub lineweight: i32,
    /// 图层表索引。
    pub layer: u32,
    pub fill: bool,
}

impl Default for PrimAttrs {
    fn default() -> Self {
        Self { color: 256, linetype: 0, lineweight: -3, layer: 0, fill: false }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub prim: Prim,
    pub attrs: PrimAttrs,
}

/// 解码结果。
#[derive(Debug, Clone, Default)]
pub struct Decoded {
    pub pieces: Vec<Piece>,
    /// 记录类型直方图（排障/统计用）。
    pub record_types: std::collections::BTreeMap<u32, usize>,
    /// 已忽略的记录类型（没实现的图元种类）。
    pub unsupported: std::collections::BTreeSet<u32>,
}

fn u32_at(d: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(off..off + 4)?.try_into().ok()?))
}

fn f64_at(d: &[u8], i: usize) -> Option<f64> {
    let o = i * 8;
    Some(f64::from_le_bytes(d.get(o..o + 8)?.try_into().ok()?))
}

fn v3(d: &[u8], i: usize) -> Option<[f64; 3]> {
    Some([f64_at(d, i)?, f64_at(d, i + 1)?, f64_at(d, i + 2)?])
}

/// 解码整份代理图形数据（`graphic_data`）。
pub fn decode(data: &[u8]) -> Decoded {
    let mut out = Decoded::default();
    let mut attrs = PrimAttrs::default();
    let mut off = 8usize; // 跳过 8 字节头
    while off + 8 <= data.len() {
        let (Some(size), Some(rt)) = (u32_at(data, off), u32_at(data, off + 4)) else { break };
        let size = size as usize;
        if size < 8 || off + size > data.len() {
            break;
        }
        let p = &data[off + 8..off + size];
        *out.record_types.entry(rt).or_default() += 1;
        match rt {
            ty::CIRCLE => {
                if let (Some(c), Some(r)) = (v3(p, 0), f64_at(p, 3)) {
                    out.push(Prim::Circle { c, r: r.abs() }, attrs);
                }
            }
            ty::CIRCULAR_ARC => {
                // center(3), radius, normal(3), start_vec(3), sweep(rad), arc_type(i32)
                if let (Some(c), Some(r), Some(sv), Some(sweep)) =
                    (v3(p, 0), f64_at(p, 3), v3(p, 7), f64_at(p, 10))
                {
                    let start = sv[1].atan2(sv[0]).to_degrees();
                    out.push(
                        Prim::Arc { c, r: r.abs(), start_deg: start, end_deg: start + sweep.to_degrees() },
                        attrs,
                    );
                }
            }
            ty::POLYLINE | ty::POLYGON => {
                let n = u32_at(p, 0).unwrap_or(0) as usize;
                let mut pts = Vec::with_capacity(n.min(4096));
                for i in 0..n.min(65536) {
                    match v3(p, 0).and_then(|_| poly_point(p, i)) {
                        Some(pt) => pts.push(pt),
                        None => break,
                    }
                }
                out.push_poly(pts, rt == ty::POLYGON, attrs);
            }
            ty::ATTRIBUTE_COLOR => {
                if let Some(c) = u32_at(p, 0) {
                    attrs.color = if c <= 256 { c as u16 } else { 256 };
                }
            }
            ty::ATTRIBUTE_LAYER => {
                if let Some(l) = u32_at(p, 0) {
                    attrs.layer = l;
                }
            }
            ty::ATTRIBUTE_LINETYPE => {
                if let Some(l) = u32_at(p, 0) {
                    attrs.linetype = l;
                }
            }
            ty::ATTRIBUTE_LINEWEIGHT => {
                if let Some(w) = u32_at(p, 0) {
                    attrs.lineweight = w as i32;
                }
            }
            ty::ATTRIBUTE_FILL => {
                if let Some(f) = u32_at(p, 0) {
                    attrs.fill = f != 0;
                }
            }
            ty::ATTRIBUTE_TRUE_COLOR => {
                // 0xC0000000 以上为 true color（低 24 位 BGR）；否则是 ACI。
                if let Some(c) = u32_at(p, 0) {
                    if c & 0xFF00_0000 != 0xC000_0000 {
                        attrs.color = if c <= 256 { c as u16 } else { 256 };
                    }
                }
            }
            ty::ATTRIBUTE_MARKER
            | ty::ATTRIBUTE_LTSCALE
            | ty::ATTRIBUTE_THICKNESS
            | ty::ATTRIBUTE_PLOT_STYLE_NAME
            | ty::ATTRIBUTE_MATERIAL
            | ty::ATTRIBUTE_MAPPER
            | ty::EXTENTS
            | ty::PUSH_CLIP
            | ty::POP_CLIP
            | ty::PUSH_MATRIX
            | ty::PUSH_MATRIX2
            | ty::POP_MATRIX => {}
            ty::TEXT | ty::TEXT2 | ty::UNICODE_TEXT | ty::UNICODE_TEXT2 => {
                let unicode = rt == ty::UNICODE_TEXT || rt == ty::UNICODE_TEXT2;
                if let (Some(pos), Some(dir), Some(h), Some(rot)) =
                    (v3(p, 0), v3(p, 6), f64_at(p, 9), None::<f64>)
                {
                    let rot_deg = dir[1].atan2(dir[0]).to_degrees();
                    let _ = h;
                    let _ = rot;
                    let text = read_text(&p[96..], unicode);
                    out.push(Prim::Text { pos, text, height: f64_at(p, 9).unwrap_or(2.5), rot_deg }, attrs);
                }
            }
            ty::XLINE | ty::RAY => {
                if let (Some(a), Some(d)) = (v3(p, 0), v3(p, 3)) {
                    let k = 1.0e4;
                    out.push(
                        Prim::Line {
                            a: [a[0] - d[0] * k, a[1] - d[1] * k, a[2]],
                            b: [a[0] + d[0] * k, a[1] + d[1] * k, a[2]],
                        },
                        attrs,
                    );
                }
            }
            other => {
                out.unsupported.insert(other);
            }
        }
        off += size;
    }
    out
}

/// POLYLINE / POLYGON 的第 i 个点：载荷 = i32 点数 + i×(x,y,z)，点 i 起于字节 4 + i*24。
fn poly_point(p: &[u8], i: usize) -> Option<[f64; 3]> {
    let base = 4 + i * 24;
    if base + 24 > p.len() {
        return None;
    }
    Some([
        f64::from_le_bytes(p[base..base + 8].try_into().ok()?),
        f64::from_le_bytes(p[base + 8..base + 16].try_into().ok()?),
        f64::from_le_bytes(p[base + 16..base + 24].try_into().ok()?),
    ])
}

fn read_text(tail: &[u8], unicode: bool) -> String {
    if unicode {
        let mut units = Vec::new();
        for chunk in tail.chunks_exact(2) {
            let u = u16::from_le_bytes([chunk[0], chunk[1]]);
            if u == 0 {
                break;
            }
            units.push(u);
        }
        String::from_utf16_lossy(&units)
    } else {
        let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
        String::from_utf8_lossy(&tail[..end]).into_owned()
    }
}

impl Decoded {
    fn push(&mut self, prim: Prim, attrs: PrimAttrs) {
        self.pieces.push(Piece { prim, attrs });
    }

    /// 折线：1 点→点，2 点→线段，多点→折线（与 ezdxf 行为一致）。
    fn push_poly(&mut self, pts: Vec<[f64; 3]>, closed: bool, attrs: PrimAttrs) {
        match pts.len() {
            0 => {}
            1 => self.push(Prim::Point { p: pts[0] }, attrs),
            2 if same(pts[0], pts[1]) => self.push(Prim::Point { p: pts[0] }, attrs),
            2 => self.push(Prim::Line { a: pts[0], b: pts[1] }, attrs),
            _ => self.push(Prim::Polyline { pts, closed }, attrs),
        }
    }

    /// 包围盒（z 忽略时用 [xmin,ymin,xmax,ymax]）。
    pub fn bbox_xy(&self) -> Option<[f64; 4]> {
        let mut bb: Option<[f64; 4]> = None;
        let mut add = |x: f64, y: f64| {
            if !x.is_finite() || !y.is_finite() || x.abs() > 1e7 || y.abs() > 1e7 {
                return;
            }
            let e = bb.get_or_insert([x, y, x, y]);
            e[0] = e[0].min(x);
            e[1] = e[1].min(y);
            e[2] = e[2].max(x);
            e[3] = e[3].max(y);
        };
        for p in &self.pieces {
            match &p.prim {
                Prim::Line { a, b } => {
                    add(a[0], a[1]);
                    add(b[0], b[1]);
                }
                Prim::Polyline { pts, .. } => {
                    for q in pts {
                        add(q[0], q[1]);
                    }
                }
                Prim::Circle { c, r } | Prim::Arc { c, r, .. } => {
                    add(c[0] - r, c[1] - r);
                    add(c[0] + r, c[1] + r);
                }
                Prim::Point { p } => add(p[0], p[1]),
                Prim::Text { pos, .. } => add(pos[0], pos[1]),
            }
        }
        bb
    }
}

fn same(a: [f64; 3], b: [f64; 3]) -> bool {
    (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9 && (a[2] - b[2]).abs() < 1e-9
}

// ── 显示辅助：ACI 颜色 / 线型名 ────────────────────────────────────────────

/// ACI 索引 → RGB（够用的一小部分；其余走近似表）。
pub fn aci_rgb(aci: u16) -> (u8, u8, u8) {
    match aci {
        1 => (255, 0, 0),
        2 => (255, 255, 0),
        3 => (0, 255, 0),
        4 => (0, 255, 255),
        5 => (0, 0, 255),
        6 => (255, 0, 255),
        7 => (0, 0, 0),
        8 => (128, 128, 128),
        9 => (192, 192, 192),
        250 => (51, 51, 51),
        251 => (91, 91, 91),
        252 => (132, 132, 132),
        253 => (173, 173, 173),
        254 => (214, 214, 214),
        255 => (255, 255, 255),
        _ => (120, 120, 120),
    }
}
