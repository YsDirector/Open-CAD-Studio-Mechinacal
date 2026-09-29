//! 一次性普查工具：统计 ACM 标准件库里 `STDPART2D` 代理图形用到的
//! 记录类型 / 颜色 / 线型 / 线宽，以及解出来的几何规模。
//!
//! 用法：
//! ```sh
//! cargo run -q -p ocs_plugin_api --features host --example proxy_survey -- <目录> [文件上限] [取样间隔]
//! ```

use ocs_plugin_api::host::codec::io::dwg::DwgReader;
use ocs_plugin_api::host::codec::objects::ObjectType;
use ocs_plugin_api::host::codec::EntityType;
use std::collections::{BTreeMap, BTreeSet};

/// 代理图形记录类型（ezdxf ProxyGraphicTypes 权威表）。
fn type_name(t: u32) -> &'static str {
    match t {
        1 => "EXTENTS",
        2 => "CIRCLE",
        3 => "CIRCLE_3P",
        4 => "CIRCULAR_ARC",
        5 => "CIRCULAR_ARC_3P",
        6 => "POLYLINE",
        7 => "POLYGON",
        8 => "MESH",
        9 => "SHELL",
        10 => "TEXT",
        11 => "TEXT2",
        12 => "XLINE",
        13 => "RAY",
        14 => "ATTR_COLOR",
        16 => "ATTR_LAYER",
        18 => "ATTR_LINETYPE",
        19 => "ATTR_MARKER",
        20 => "ATTR_FILL",
        22 => "ATTR_TRUE_COLOR",
        23 => "ATTR_LINEWEIGHT",
        24 => "ATTR_LTSCALE",
        25 => "ATTR_THICKNESS",
        26 => "ATTR_PLOT_STYLE",
        27 => "PUSH_CLIP",
        28 => "POP_CLIP",
        29 => "PUSH_MATRIX",
        30 => "PUSH_MATRIX2",
        31 => "POP_MATRIX",
        32 => "POLYLINE_WITH_NORMALS",
        33 => "LWPOLYLINE",
        34 => "ATTR_MATERIAL",
        35 => "ATTR_MAPPER",
        36 => "UNICODE_TEXT",
        37 => "UNKNOWN_37",
        38 => "UNICODE_TEXT2",
        44 => "ELLIPTIC_ARC",
        _ => "其它",
    }
}

struct RecIter<'a> {
    data: &'a [u8],
    off: usize,
}

impl<'a> Iterator for RecIter<'a> {
    type Item = (u32, &'a [u8]);
    fn next(&mut self) -> Option<Self::Item> {
        // 头 8 字节: total_size(u32) + record_count(u32)
        if self.off == 0 {
            self.off = 8;
        }
        if self.off + 8 > self.data.len() {
            return None;
        }
        let size = u32::from_le_bytes(self.data[self.off..self.off + 4].try_into().ok()?) as usize;
        let ty = u32::from_le_bytes(self.data[self.off + 4..self.off + 8].try_into().ok()?);
        if size < 8 || self.off + size > self.data.len() {
            return None;
        }
        let payload = &self.data[self.off + 8..self.off + size];
        self.off += size;
        Some((ty, payload))
    }
}

fn f64_at(d: &[u8], i: usize) -> f64 {
    let s = i * 8;
    if s + 8 > d.len() {
        f64::NAN
    } else {
        f64::from_le_bytes(d[s..s + 8].try_into().unwrap())
    }
}

#[derive(Default)]
struct Stats {
    rec_types: BTreeMap<u32, usize>,
    colors: BTreeMap<u32, usize>,
    ltypes: BTreeMap<u32, usize>,
    lweights: BTreeMap<u32, usize>,
    /// (颜色序号, 记录类型) 组合：看每种颜色下都画了什么
    color_geom: BTreeMap<(u32, u32), usize>,
    /// (颜色, 线型索引) 组合：定图层映射表的关键依据
    color_ltype: BTreeMap<(u32, u32), usize>,
    files: usize,
    parts: usize,
    no_proxy: usize,
    read_err: usize,
    geom: BTreeMap<&'static str, usize>,
    bbox: Option<([f64; 3], [f64; 3])>,
    unknown_types: BTreeSet<u32>,
    view_counts: BTreeMap<usize, usize>,
    /// 每个 STDPART2D 实体的几何量分布（看"三视图"各自多大）
    parts_geom: Vec<(String, usize, [f64; 4])>,
}

fn main() {
    let dir = std::env::args().nth(1).expect("usage: proxy_survey <dir> [max] [stride]");
    let max: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(120);
    let stride: usize = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(37);

    let mut files: Vec<String> = Vec::new();
    collect(&std::path::PathBuf::from(&dir), &mut files, 0);
    files.sort();
    let sampled: Vec<&String> = files.iter().filter(|_| true).step_by(stride.max(1)).take(max).collect();
    println!("目录 {} 共 {} 个 DWG，取样 {} 个（步长 {}）", dir, files.len(), sampled.len(), stride);

    let mut st = Stats::default();
    for f in &sampled {
        st.files += 1;
        let doc = match DwgReader::from_file(f).and_then(|mut r| r.read()) {
            Ok(d) => d,
            Err(e) => {
                st.read_err += 1;
                println!("  读失败 {f}: {e}");
                continue;
            }
        };
        let layers: Vec<String> = doc.layers.iter().map(|l| l.name.clone()).collect();
        let ltypes: Vec<String> = doc.line_types.iter().map(|l| l.name.clone()).collect();
        let mut n_parts = 0;
        for e in doc.entities() {
            if let EntityType::Unknown(u) = e {
                n_parts += 1;
                let Some(gd) = u.common.graphic_data.as_deref() else {
                    st.no_proxy += 1;
                    continue;
                };
                st.parts += 1;
                let mut color: u32 = 256; // BYLAYER
                let mut lt: u32 = 0;
                let mut ngeom = 0usize;
                let mut bb: Option<[f64; 4]> = None;
                for (ty, p) in (RecIter { data: gd, off: 0 }) {
                    *st.rec_types.entry(ty).or_default() += 1;
                    let touch = |p: &[u8], bb: &mut Option<[f64; 4]>, ngeom: &mut usize| {
                        *ngeom += 1;
                        let add = |bb: &mut Option<[f64;4]>, x: f64, y: f64| {
                            if !x.is_finite() || !y.is_finite() || x.abs() > 1e6 || y.abs() > 1e6 {
                                return;
                            }
                            let e = bb.get_or_insert([x, y, x, y]);
                            e[0] = e[0].min(x); e[1] = e[1].min(y);
                            e[2] = e[2].max(x); e[3] = e[3].max(y);
                        };
                        match ty {
                            2 | 4 => {
                                // 圆/弧：圆心 + 半径（弧还带 normal/start/sweep，此处只看包围盒粗估）
                                let (cx, cy, r) = (f64_at(p, 0), f64_at(p, 1), f64_at(p, 3).abs());
                                add(bb, cx, cy);
                                add(bb, cx - r, cy - r);
                                add(bb, cx + r, cy + r);
                            }
                            6 | 7 => {
                                // 载荷：i32 点数 + 点 i 的 x,y,z 位于字节 4+i*24 / 12+i*24
                                let n = i32::from_le_bytes(p[0..4].try_into().unwrap_or([0; 4])).max(0) as usize;
                                for i in 0..n.min(4096) {
                                    let base = 4 + i * 24;
                                    if base + 16 > p.len() { break; }
                                    add(bb,
                                        f64::from_le_bytes(p[base..base+8].try_into().unwrap()),
                                        f64::from_le_bytes(p[base+8..base+16].try_into().unwrap()));
                                }
                            }
                            _ => {}
                        }
                    };
                    match ty {
                        2 => {
                            *st.color_geom.entry((color, 2)).or_default() += 1;
                            *st.geom.entry("圆").or_default() += 1;
                            touch(p, &mut bb, &mut ngeom);
                        }
                        4 => {
                            *st.geom.entry("弧").or_default() += 1;
                            *st.color_geom.entry((color, 4)).or_default() += 1;
                            touch(p, &mut bb, &mut ngeom);
                        }
                        6 | 7 => {
                            *st.geom.entry("折线").or_default() += 1;
                            *st.color_geom.entry((color, 6)).or_default() += 1;
                            touch(p, &mut bb, &mut ngeom);
                        }
                        10 | 11 | 36 | 38 => {
                            *st.geom.entry("文字").or_default() += 1;
                        }
                        12 | 13 => {
                            *st.geom.entry("构造线").or_default() += 1;
                        }
                        32 | 33 => {
                            *st.geom.entry("复杂折线").or_default() += 1;
                            touch(p, &mut bb, &mut ngeom);
                        }
                        14 => {
                            color = u32::from_le_bytes(p[0..4].try_into().unwrap_or([0; 4]));
                            *st.colors.entry(color).or_default() += 1;
                            *st.color_ltype.entry((color, lt)).or_default() += 1;
                        }
                        16 => {
                            let idx = u32::from_le_bytes(p[0..4].try_into().unwrap_or([0; 4]));
                            let name = layers.get(idx as usize).cloned().unwrap_or_else(|| format!("#{idx}"));
                            let _ = name;
                        }
                        18 => {
                            lt = u32::from_le_bytes(p[0..4].try_into().unwrap_or([0; 4]));
                            *st.ltypes.entry(lt).or_default() += 1;
                            *st.color_ltype.entry((color, lt)).or_default() += 1;
                        }
                        23 => {
                            let lw = i32::from_le_bytes(p[0..4].try_into().unwrap_or([0; 4]));
                            *st.lweights.entry(lw as u32).or_default() += 1;
                        }
                        20 => {
                            let _fill = u32::from_le_bytes(p[0..4].try_into().unwrap_or([0; 4]));
                        }
                        t => {
                            st.unknown_types.insert(t);
                        }
                    }
                }
                if let Some(b) = bb {
                    let e = st.bbox.get_or_insert(([b[0], b[1], 0.0], [b[2], b[3], 0.0]));
                    e.0[0] = e.0[0].min(b[0]); e.0[1] = e.0[1].min(b[1]);
                    e.1[0] = e.1[0].max(b[2]); e.1[1] = e.1[1].max(b[3]);
                }
                *st.view_counts.entry(n_parts).or_default() += 1;
                if st.parts_geom.len() < 200 {
                    let name = std::path::Path::new(f).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    st.parts_geom.push((name, ngeom, bb.unwrap_or([0.0; 4])));
                }
            }
        }
        if st.unknown_types.len() > 32 {
            break;
        }
    }

    println!("\n── 概况");
    println!("  文件 {}（读失败 {}）| 含代理图形的 STDPART2D {} 个 | 无代理图形 {} 个",
        st.files, st.read_err, st.parts, st.no_proxy);
    println!("  每文件视图数分布: {:?}", st.view_counts);
    println!("  几何总计: {:?}", st.geom);
    println!("  全局包围盒: {:?}", st.bbox);
    println!("\n── 代理图形记录类型（次数）");
    for (t, n) in &st.rec_types {
        println!("  {t:3} {:<22} {n}", type_name(*t));
    }
    println!("\n── 颜色（ACI 索引 → 出现次数）");
    for (c, n) in &st.colors {
        println!("  {c:4} → {n}");
    }
    println!("\n── 线型索引");
    for (l, n) in &st.ltypes {
        println!("  {l:4} → {n}");
    }
    println!("\n── 线宽值");
    for (w, n) in &st.lweights {
        println!("  {w:6} → {n}");
    }
    println!("\n── 颜色 × 几何组合");
    for ((c, g), n) in &st.color_geom {
        println!("  颜色 {c:3} × {} → {n}", type_name(*g));
    }
    println!("\n── 颜色 × 线型（定图层映射表用；线型名来自文件线型表）");
    for ((c, l), n) in &st.color_ltype {
        println!("  颜色 {c:3} × 线型索引 {l} → {n}");
    }
    if !st.unknown_types.is_empty() {
        println!("\n⚠ 未处理的记录类型: {:?}", st.unknown_types);
    }
    println!("\n── 前若干视图（文件 / 几何件数 / bbox xmin ymin xmax ymax）");

    for (n, g, b) in st.parts_geom.iter().take(12) {
        println!("  {n:<28} {g:5}  [{:.1} {:.1} {:.1} {:.1}]", b[0], b[1], b[2], b[3]);
    }
}

fn collect(dir: &std::path::Path, out: &mut Vec<String>, depth: usize) {
    if depth > 3 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            let name = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            if name.starts_with('_') || name == "免费资料下载" {
                continue;
            }
            collect(&p, out, depth + 1);
        } else if p.extension().map(|x| x.eq_ignore_ascii_case("dwg")).unwrap_or(false) {
            out.push(p.to_string_lossy().into_owned());
        }
    }
    let _ = std::any::type_name::<ObjectType>();
}
