//! 扫全库找"基点标记"：每个视图里是否有 r≈1.00 mm 的小圆（ACM 标准件的插入点标记），
//! 以及它的颜色分布、相对视图几何的位置。
//!
//! 用法：
//! ```sh
//! cargo run -q -p ocs_plugin_api --features host --example marker_survey -- <库根> [max]
//! ```

#[path = "kit/proxy_decode.rs"]
mod dec;

use dec::Prim;
use ocs_plugin_api::host::codec::io::dwg::DwgReader;
use ocs_plugin_api::host::codec::EntityType;
use std::collections::BTreeMap;

fn main() {
    let dir = std::env::args().nth(1).expect("usage: marker_survey <dir> [max]");
    let max: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
    let mut files = Vec::new();
    collect(std::path::Path::new(&dir), &mut files, 0);
    files.sort();

    let mut views = 0usize;
    let mut with_marker = 0usize;
    let mut colors: BTreeMap<u16, usize> = BTreeMap::new();
    let mut families: BTreeMap<String, (usize, usize)> = BTreeMap::new(); // 族 → (有标记视图, 总视图)
    let mut samples: Vec<String> = Vec::new();
    for f in files.iter().take(max) {
        let Ok(doc) = DwgReader::from_file(f).and_then(|mut r| r.read()) else { continue };
        let family = std::path::Path::new(f)
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        for e in doc.entities() {
            let EntityType::Unknown(u) = e else { continue };
            let Some(gd) = u.common.graphic_data.as_deref() else { continue };
            let d = dec::decode(gd);
            if d.pieces.is_empty() {
                continue;
            }
            views += 1;
            let ent = families.entry(family.clone()).or_default();
            ent.1 += 1;
            let tiny: Vec<(u16, [f64; 3], f64)> = d
                .pieces
                .iter()
                .filter_map(|p| match &p.prim {
                    Prim::Circle { c, r } if (r - 1.0).abs() < 0.02 => Some((p.attrs.color, *c, *r)),
                    _ => None,
                })
                .collect();
            if !tiny.is_empty() {
                with_marker += 1;
                ent.0 += 1;
                for (c, _, _) in &tiny {
                    *colors.entry(*c).or_default() += 1;
                }
                if samples.len() < 8 {
                    let bb = d.bbox_xy().unwrap_or([0.0; 4]);
                    samples.push(format!(
                        "{} / {} → 标记圆心 ({:.2},{:.2}) 颜色 {} | 视图 bbox [{:.1},{:.1} … {:.1},{:.1}]",
                        family,
                        std::path::Path::new(f).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
                        tiny[0].1[0],
                        tiny[0].1[1],
                        tiny[0].0,
                        bb[0],
                        bb[1],
                        bb[2],
                        bb[3]
                    ));
                }
            }
        }
    }
    println!("视图总数 {views}；含 r≈1.00 小圆的视图 {with_marker}（{:.1}%）", 100.0 * with_marker as f64 / views.max(1) as f64);
    println!("小圆颜色分布: {colors:?}");
    let mut fams: Vec<(&String, &(usize, usize))> = families.iter().filter(|(_, v)| v.0 > 0).collect();
    fams.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    println!("\n含标记的零件族（前 30）:");
    for (name, (m, t)) in fams.iter().take(30) {
        println!("   {m:4}/{t:<4} {name}");
    }
    println!("\n样本:");
    for s in &samples {
        println!("   {s}");
    }
}

fn collect(dir: &std::path::Path, out: &mut Vec<String>, depth: usize) {
    if depth > 3 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if name.starts_with('_') || name == "免费资料下载" {
            continue;
        }
        if p.is_dir() {
            collect(&p, out, depth + 1);
        } else if p.extension().map(|x| x.eq_ignore_ascii_case("dwg")).unwrap_or(false) {
            out.push(p.to_string_lossy().into_owned());
        }
    }
}
