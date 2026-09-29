//! 标准件"视图拆分"工具：把一个 DWG 拆成**每视图一个文件**，并按 OCSM 图层归并。
//!
//! - 输入：ACM 标准件 DWG（几何在 `STDPART2D` 代理图形里）或普通 DWG
//!   （几何就是模型空间图元）。
//! - 输出：`<out>/<族目录>/<名称>_<标准号>_<规格>_<视图名>.dwg`
//!   （例：`六角头螺栓 - GB 5780-86/六角头螺栓_GB5780-86_M10x100_主视图.dwg`），
//!   图元落在 OCSM 图层上（白→1轮廓实线层、绿→2细线层、青→3中心线层、
//!   洋红→4虚线层、红→5剖面线层、253→8符号标注层），颜色/线型/线宽一律 ByLayer。
//! - 同时写 `<out>/catalog.csv`：族/名称/标准号/规格/视图/文件/宽高/图元数/基点标记。
//! - 坐标系保持不变（插入基点由插件按标记圆或示范规则另算）。
//!
//! 用法：
//! ```sh
//! cargo run -q -p ocs_plugin_api --features host --example split_views -- <in.dwg|dir> <out_dir> [limit] [stride]
//! ```

#[path = "kit/part_lib.rs"]
mod part_lib;

use ocs_plugin_api::host::codec::io::dwg::{DwgReadOptions, DwgReader, DwgWriter};
use ocs_plugin_api::host::codec::EntityType;
use part_lib::dec;

fn main() {
    let mut a = std::env::args().skip(1);
    let src = a.next().expect("usage: split_views <in.dwg|dir> <out_dir> [limit] [stride]");
    let out = a.next().expect("usage: split_views <in.dwg|dir> <out_dir> [limit] [stride]");
    let limit: usize = a.next().and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
    let stride: usize = a.next().and_then(|s| s.parse().ok()).unwrap_or(1);

    let in_path = std::path::Path::new(&src);
    let mut files = Vec::new();
    if in_path.is_file() {
        files.push(src.clone());
    } else {
        collect(in_path, &mut files, 0);
        files.sort();
    }
    let picked: Vec<&String> = files.iter().step_by(stride.max(1)).take(limit).collect();
    println!("输入 {} 个 DWG，处理 {} 个 → 输出根 {out}", files.len(), picked.len());

    let mut catalog = String::from("族目录,名称,标准号,规格,视图,文件,宽mm,高mm,图元数,基点标记x,基点标记y\n");
    let mut n_files = 0usize;
    let mut n_views = 0usize;
    let mut n_ent = 0usize;
    let mut fails: Vec<String> = Vec::new();
    for f in &picked {
        match split_one(f, &out, &mut catalog) {
            Ok((views, ents)) => {
                n_files += 1;
                n_views += views;
                n_ent += ents;
                if n_files % 200 == 0 {
                    println!("  已处理 {n_files} 个文件 / {n_views} 个视图 / {n_ent} 个图元");
                }
            }
            Err(e) => fails.push(format!("{f}: {e}")),
        }
    }
    std::fs::write(std::path::Path::new(&out).join("catalog.csv"), &catalog).ok();
    println!("\n完成：文件 {n_files} → 视图 {n_views} 个（图元 {n_ent}）| 失败 {}", fails.len());
    for f in fails.iter().take(20) {
        println!("  ✗ {f}");
    }
}

/// `六角头螺栓 - GB 5780-86` → ("六角头螺栓", "GB5780-86")；兼容 `沉头带榫螺栓_GB_11_88`。
fn parse_family(dir: &str) -> (String, String) {
    let d = dir.trim();
    if let Some((name, std)) = d.split_once(" - ") {
        return (name.trim().to_string(), std.replace(' ', ""));
    }
    if let Some(i) = d.find("_GB") {
        let name = d[..i].trim().to_string();
        let std = d[i + 1..].replace('_', " ").replace(' ', "");
        return (name, std);
    }
    if let Some(i) = d.find("GB") {
        return (d[..i].trim().trim_end_matches(['-', '_', ' ']).to_string(), d[i..].replace(' ', ""));
    }
    (d.to_string(), String::new())
}

/// 视图名（启发式，可被 catalog.csv 覆盖）：先按包围盒面积从大到小，
/// 面积接近（±5%）时按图元数从多到少 → 主视图 / 俯视图 / 左视图 / 右视图 / 后视图 / 视图N。
fn view_names(metrics: &[(f64, usize)]) -> Vec<String> {
    let order = ["主视图", "俯视图", "左视图", "右视图", "后视图"];
    let mut idx: Vec<usize> = (0..metrics.len()).collect();
    idx.sort_by(|a, b| {
        let (aa, an) = metrics[*a];
        let (ba, bn) = metrics[*b];
        let near = (aa - ba).abs() <= 0.05 * aa.max(ba).max(1e-9);
        if near {
            bn.cmp(&an)
        } else {
            ba.partial_cmp(&aa).unwrap_or(std::cmp::Ordering::Equal)
        }
    });
    let mut names = vec![String::new(); metrics.len()];
    for (rank, i) in idx.iter().enumerate() {
        names[*i] = order
            .get(rank)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("视图{}", rank + 1));
    }
    names
}

fn sanitize(s: &str) -> String {
    s.replace(['/', '\\', ' ', '\t'], "_").trim_matches('_').to_string()
}

fn split_one(src: &str, out_root: &str, catalog: &mut String) -> Result<(usize, usize), String> {
    let doc = DwgReader::from_file_with_options(src, DwgReadOptions::failsafe())
        .and_then(|mut r| r.read())
        .map_err(|e| e.to_string())?;

    let mut views: Vec<Vec<dec::Piece>> = Vec::new();
    let mut plain: Vec<EntityType> = Vec::new();
    for e in doc.entities() {
        match e {
            EntityType::Unknown(u) if u.dxf_name.eq_ignore_ascii_case("STDPART2D") => {
                if let Some(gd) = u.common.graphic_data.as_deref() {
                    let d = dec::decode(gd);
                    if !d.pieces.is_empty() {
                        views.push(d.pieces);
                    }
                }
            }
            EntityType::Point(_) => {}
            other => plain.push(other.clone()),
        }
    }
    if views.is_empty() {
        if plain.is_empty() {
            return Err("没有可插入的几何".into());
        }
        views.push(Vec::new());
    }

    let src_path = std::path::Path::new(src);
    let spec = src_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "part".into());
    let dir_name = src_path
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "parts".into());
    let (part_name, std_no) = parse_family(&dir_name);
    let dir = std::path::Path::new(out_root).join(&dir_name);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    // 面积排序定视图名
    let metrics: Vec<(f64, usize)> = views
        .iter()
        .map(|p| {
            if p.is_empty() {
                (1.0, p.len())
            } else {
                let b = part_lib::dec::Decoded { pieces: p.clone(), ..Default::default() }.bbox_xy();
                let a = b.map(|b| ((b[2] - b[0]).max(0.0)) * ((b[3] - b[1]).max(0.0))).unwrap_or(1.0);
                (a, p.len())
            }
        })
        .collect();
    let names = view_names(&metrics);

    let mut written_views = 0usize;
    let mut written_ent = 0usize;
    for (i, pieces) in views.iter().enumerate() {
        let mut doc = part_lib::new_doc();
        let (n, marker) = if pieces.is_empty() {
            let mut n = 0;
            for e in &plain {
                let color = entity_aci(e);
                let layer = part_lib::layer_for(color, 0);
                let mut e = e.clone();
                {
                    let c = e.common_mut();
                    c.layer = layer.to_string();
                    c.color = ocs_plugin_api::host::codec::types::Color::ByLayer;
                    c.linetype = "ByLayer".to_string();
                    c.line_weight = ocs_plugin_api::host::codec::types::LineWeight::ByLayer;
                }
                if doc.add_entity(e).is_ok() {
                    n += 1;
                }
            }
            (n, None)
        } else {
            (part_lib::add_pieces(&mut doc, pieces), part_lib::base_marker_center(pieces))
        };
        if n == 0 {
            continue;
        }
        let file_name = format!(
            "{}_{}_{}_{}",
            sanitize(&part_name),
            sanitize(&std_no),
            sanitize(&spec),
            names[i]
        );
        let out_file = dir.join(format!("{file_name}.dwg"));
        DwgWriter::write_to_file(&out_file, &doc).map_err(|e| e.to_string())?;

        let bb = if pieces.is_empty() {
            None
        } else {
            part_lib::dec::Decoded { pieces: pieces.clone(), ..Default::default() }.bbox_xy()
        };
        let (w, h) = bb.map(|b| (b[2] - b[0], b[3] - b[1])).unwrap_or((0.0, 0.0));
        catalog.push_str(&format!(
            "{},{},{},{},{},{},{:.2},{:.2},{},{}{}\n",
            dir_name,
            part_name,
            std_no,
            spec,
            names[i],
            file_name,
            w,
            h,
            n,
            marker.map(|m| format!("{:.3}", m[0])).unwrap_or_default(),
            marker.map(|m| format!(",{:.3}", m[1])).unwrap_or_else(|| ",".into()),
        ));
        written_views += 1;
        written_ent += n;
    }
    if written_views == 0 {
        return Err("所有视图都为空".into());
    }
    Ok((written_views, written_ent))
}

fn entity_aci(e: &EntityType) -> u16 {
    match e.common().color {
        ocs_plugin_api::host::codec::types::Color::Index(i) => i as u16,
        _ => 7,
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
