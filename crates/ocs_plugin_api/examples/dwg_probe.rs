//! 一次性探针：dump DWG 里"读不懂"的自定义对象/实体的原始数据。
//!
//! 用途：判断 ACM（AutoCAD Mechanical）标准件图（STDPART2D + ACDBSTDPARTRES_*）
//! 里的几何能否在插件进程内自己解出来，而不依赖 AutoCAD 炸开。
//!
//! 用法：
//! ```sh
//! cargo run -p ocs_plugin_api --features host --example dwg_probe -- <file.dwg> [max_objs]
//! ```

use ocs_plugin_api::host::codec::io::dwg::DwgReader;
use ocs_plugin_api::host::codec::objects::ObjectType;
use ocs_plugin_api::host::codec::EntityType;

fn hex_preview(data: &[u8], max: usize) -> String {
    let n = data.len().min(max);
    let mut s = String::new();
    for (i, b) in data[..n].iter().enumerate() {
        if i % 16 == 0 {
            s.push_str(&format!("\n      {:04x}: ", i));
        }
        s.push_str(&format!("{b:02x} "));
    }
    if data.len() > n {
        s.push_str(&format!("… (+{} bytes)", data.len() - n));
    }
    s
}

/// 把一段字节当 f64 小端序列试着解读（proxy graphics / 参数表里坐标多半是 f64）。
fn as_f64s(data: &[u8], max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let n = (data.len() / 8).min(max);
    for i in 0..n {
        let v = f64::from_le_bytes(data[i * 8..i * 8 + 8].try_into().unwrap());
        if v.is_finite() && (v == 0.0 || (v.abs() > 1e-6 && v.abs() < 1e6)) {
            out.push(format!("{v:.3}"));
        } else {
            out.push("·".to_string());
        }
    }
    out
}

/// 在原始字节里找可读 ASCII 串（对象里常带字段名/引用）。
fn ascii_strings(data: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for &b in data {
        if (0x20..0x7f).contains(&b) {
            cur.push(b as char);
        } else {
            if cur.len() >= 5 {
                out.push(cur.clone());
            }
            cur.clear();
        }
    }
    if cur.len() >= 5 {
        out.push(cur);
    }
    out.truncate(24);
    out
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: dwg_probe <file.dwg>");
    let max_objs: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(6);

    let mut reader = DwgReader::from_file(&path).expect("DwgReader::from_file");
    let doc = reader.read().expect("read");

    println!("== 文件: {path}");
    println!("== 版本: {:?} | 实体 {} 个 | 对象 {} 个",
        doc.version, doc.entities().count(), doc.objects.len());

    // ── 实体 ────────────────────────────────────────────────────────────────
    println!("\n── 实体（按类型统计 + Unknown 明细）");
    let mut by_type = std::collections::BTreeMap::<String, usize>::new();
    for e in doc.entities() {
        let name = match e {
            EntityType::Unknown(u) => format!("Unknown({}, code={})", u.dxf_name, u.dwg_type_code),
            other => format!("{:?}", std::mem::discriminant(other)),
        };
        *by_type.entry(name).or_default() += 1;
    }
    for (k, v) in &by_type {
        println!("   {v:5}  {k}");
    }

    for e in doc.entities() {
        if let EntityType::Unknown(u) = e {
            let gd = u.common.graphic_data.as_deref();
            println!("\n   [Unknown] dxf_name={:?} dwg_type_code={} layer={:?} color={:?}",
                u.dxf_name, u.dwg_type_code, u.common.layer, u.common.color);
            println!("      graphic_data(代理图形): {}",
                gd.map(|d| format!("{} 字节", d.len())).unwrap_or("无".into()));
            if let Some(g) = u.common.proxy_graphics() {
                use ocs_plugin_api::host::codec::entities::ProxyGraphicRecord as R;
                println!("      代理图形记录 {} 条:", g.records.len());
                let mut kinds = std::collections::BTreeMap::<u32, (usize, usize)>::new();
                for r in &g.records {
                    if let R::Unknown { record_type, data } = r {
                        let e = kinds.entry(*record_type).or_insert((0, 0));
                        e.0 += 1;
                        e.1 += data.len();
                    } else {
                        kinds.entry(999).or_insert((0, 0)).0 += 1;
                    }
                }
                println!("        类型统计 (类型: 条数/合计字节): {kinds:?}");
                // 大载荷记录（≥ 8 字节）逐条 hex + 解包尝试，最多 8 条
                let mut shown = 0;
                for r in &g.records {
                    if shown >= 8 {
                        break;
                    }
                    if let R::Unknown { record_type, data } = r {
                        if data.len() < 8 {
                            continue;
                        }
                        shown += 1;
                        let n = i32::from_le_bytes(data[0..4].try_into().unwrap_or([0; 4]));
                        println!("        · 类型 {record_type}  {} 字节  首4字节(i32)={}  去头 f64: {:?}",
                            data.len(), n, as_f64s(&data[4..], 10));
                        println!("           hex:{}", hex_preview(data, 56));
                    }
                }
            }
            if let Some(raw) = u.raw_dwg_data.as_deref() {
                println!("      raw_dwg_data: {} 字节", raw.len());
                println!("      ASCII: {:?}", ascii_strings(raw));
                println!("      头部 f64: {:?}", as_f64s(raw, 12));
                println!("      十六进制:{}", hex_preview(raw, 96));
            }
        }
    }

    // ── 对象（ACM 的 ACDBSTDPARTRES_* 等就在这里） ──────────────────────────
    println!("\n── 对象（非图形对象；Unknown 明细，最多 {max_objs} 个）");
    let mut unk = Vec::new();
    for (h, o) in &doc.objects {
        if let ObjectType::Unknown { type_name, handle, raw_dwg_data, .. } = o {
            unk.push((*h, type_name.clone(), *handle, raw_dwg_data.clone()));
        }
    }
    unk.sort_by(|a, b| a.1.cmp(&b.1));
    let mut counts = std::collections::BTreeMap::<String, (usize, usize)>::new();
    for (_, name, _, raw) in &unk {
        let e = counts.entry(name.clone()).or_insert((0, 0));
        e.0 += 1;
        e.1 += raw.as_ref().map(|d| d.len()).unwrap_or(0);
    }
    for (name, (n, bytes)) in &counts {
        println!("   {n:4} × {name}  (合计 {bytes} 字节原始数据)");
    }
    for (h, name, handle, raw) in unk.iter().take(max_objs) {
        println!("\n   [{name}] doc_handle={h:?} obj_handle={handle:?} raw={}",
            raw.as_ref().map(|d| d.len()).unwrap_or(0));
        if let Some(raw) = raw.as_deref() {
            println!("      ASCII: {:?}", ascii_strings(raw));
            println!("      头部 f64: {:?}", as_f64s(raw, 16));
            println!("      十六进制:{}", hex_preview(raw, 128));
        }
    }
}
