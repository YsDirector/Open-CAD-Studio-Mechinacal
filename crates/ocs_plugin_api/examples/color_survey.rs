//! 颜色来源普查：库里的每种"非标准"颜色（红/洋红/灰…）出自哪些零件。
//!
//! 用法：
//! ```sh
//! cargo run -q -p ocs_plugin_api --features host --example color_survey -- <库根> [文件上限]
//! ```

#[path = "kit/proxy_decode.rs"]
mod dec;

use ocs_plugin_api::host::codec::io::dwg::DwgReader;
use ocs_plugin_api::host::codec::EntityType;
use std::collections::BTreeMap;

fn main() {
    let dir = std::env::args().nth(1).expect("usage: color_survey <dir> [max]");
    let max: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
    let mut files = Vec::new();
    collect(std::path::Path::new(&dir), &mut files, 0);
    files.sort();
    println!("库根 {dir}：{} 个 DWG", files.len());

    // 颜色 → (目录名 → 次数)，以及颜色 → 例子文件
    let mut by_color: BTreeMap<u16, BTreeMap<String, usize>> = BTreeMap::new();
    let mut examples: BTreeMap<u16, Vec<String>> = BTreeMap::new();
    let mut seen = 0usize;
    for f in files.iter().take(max) {
        let Ok(doc) = DwgReader::from_file(f).and_then(|mut r| r.read()) else { continue };
        seen += 1;
        let mut colors: Vec<u16> = Vec::new();
        for e in doc.entities() {
            let EntityType::Unknown(u) = e else { continue };
            let Some(gd) = u.common.graphic_data.as_deref() else { continue };
            let d = dec::decode(gd);
            for p in &d.pieces {
                let c = p.attrs.color;
                if !colors.contains(&c) {
                    colors.push(c);
                }
            }
        }
        let family = std::path::Path::new(f)
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        for c in colors {
            *by_color.entry(c).or_default().entry(family.clone()).or_default() += 1;
            let ex = examples.entry(c).or_default();
            if ex.len() < 4 {
                ex.push(f.clone());
            }
        }
    }
    println!("已扫描 {seen} 个文件\n");

    for (color, fams) in &by_color {
        let total: usize = fams.values().sum();
        println!("══ 颜色 {color}（{total} 个视图）");
        let mut list: Vec<(&String, &usize)> = fams.iter().collect();
        list.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (fam, n) in list.iter().take(25) {
            println!("     {n:5}  {fam}");
        }
        if list.len() > 25 {
            println!("     … 另有 {} 个零件族", list.len() - 25);
        }
        for ex in examples.get(color).into_iter().flatten() {
            println!("     例: {}", ex.replace(&std::env::var("HOME").unwrap_or_default(), "~"));
        }
        println!();
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
