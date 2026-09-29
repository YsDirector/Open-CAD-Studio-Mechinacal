//! 序号标注 ↔ 明细表 联动（第二期）。
//!
//! 用户 2026-09-15 定的规则（原话落在 `handbook/14-序号标注.md` §三）：
//!
//! * 明细表**按序号升序排列**；每加一个新序号，表跟着加一格；
//! * 手填序号与已有重复时，看「插入序号」开关：
//!   **勾选** → 插入序号**之后的**所有序号全部 +1（插队后移）；
//!   **不勾选** → 视为同一零件，明细表里该序号行**数量 +1**。
//!
//! 本模块只做"序号 → 行"的规划与重编号（写表走 `bom::fill_bom`）：
//!
//! * [`groups`]：读球标台账（`OCSM_BALLOON`）——每组 = 序号列表 + 关联零件（吸附到的）；
//! * [`plan_rows`]：行 = 序号（球标组每个条目一个）∪ 未被球标引用的聚合零件
//!   （沿用旧表里同件的序号，没有就自动接号）→ 按序号升序；
//! * [`sync_after_group`]：新建/改号后调用——冲突重编号（勾选模式）+ 刷新表。

use std::collections::BTreeMap;

use ocs_plugin_api::host::acadrust::entities::{EntityCommon, EntityType, Insert};
use ocs_plugin_api::host::acadrust::xdata::XDataValue;
use ocs_plugin_api::host::acadrust::{CadDocument, Handle};

use crate::balloon::{item_no_key, item_no_next};
use crate::bom::{self, PartMeta, RowSpec};

pub(crate) const XDATA_BALLOON: &str = "OCSM_BALLOON";

/// 一个序号球标组（读自 `OCSM_BALLOON` 台账）。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Group {
    pub handle: Handle,
    /// 序号列表（从指引线向外/向上）。
    pub items: Vec<String>,
    /// 关联零件（指针点吸附到的块台账，没吸附到就是 None）。
    pub part: Option<PartMeta>,
    /// 关联零件的块 handle（改号时用来找新版）。
    pub part_handle: Option<String>,
}

/// `OCSM_BALLOON` 记录里的字符串（JSON）。
fn record_text(common: &EntityCommon, app: &str) -> Option<String> {
    let rec = common.extended_data.get_record(app)?;
    rec.values.iter().find_map(|v| match v {
        XDataValue::String(s) => Some(s.clone()),
        _ => None,
    })
}

/// 图纸上所有序号球标组（按 handle 排序，保证确定性）。
pub(crate) fn groups(doc: &CadDocument) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    for e in doc.model_space_entities() {
        let EntityType::Insert(ins) = e else {
            continue;
        };
        let Some(text) = record_text(&ins.common, XDATA_BALLOON) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let items: Vec<String> = v
            .get("items")
            .and_then(|x| x.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        if items.is_empty() {
            continue;
        }
        let part = v
            .get("part")
            .filter(|p| !p.is_null())
            .and_then(|p| serde_json::from_value::<PartMeta>(p.clone()).ok());
        let part_handle = v
            .get("part_handle")
            .and_then(|h| h.as_str())
            .map(|s| s.to_string());
        out.push(Group {
            handle: ins.common.handle,
            items,
            part,
            part_handle,
        });
    }
    out.sort_by_key(|g| g.handle.value());
    out
}

/// 现有明细表行（`OCSM_BOMROW`）：序号 → (8 格值, 数量锁)。
///
/// 锁来自行块 XDATA `OCSM_BOMLOCK`（用户手改数量后写下的，见 `bom::LOCK_APP`）。
pub(crate) fn old_rows(doc: &CadDocument) -> BTreeMap<String, ([String; 8], Option<usize>, Option<usize>)> {
    let mut out: BTreeMap<String, ([String; 8], Option<usize>, Option<usize>)> = BTreeMap::new();
    for e in doc.model_space_entities() {
        let EntityType::Insert(ins) = e else {
            continue;
        };
        if ins.block_name != bom::ROW_BLOCK {
            continue;
        }
        // ③ 读侧兼容：按列号取（`bom::cell_value` 认 zh/en 两套 tag，老图读得回）。
        let no = bom::cell_value(ins, 0);
        if no.is_empty() {
            continue;
        }
        let values: [String; 8] = [
            no.clone(),
            bom::cell_value(ins, 1),
            bom::cell_value(ins, 2),
            bom::cell_value(ins, 3),
            bom::cell_value(ins, 4),
            bom::cell_value(ins, 5),
            bom::cell_value(ins, 6),
            bom::cell_value(ins, 7),
        ];
        let lock = record_text(&ins.common, bom::LOCK_APP)
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("qty").and_then(|q| q.as_u64()))
            .map(|q| q as usize);
        let exported = record_text(&ins.common, bom::EXPORTED_APP)
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("qty").and_then(|q| q.as_u64()))
            .map(|q| q as usize);
        out.entry(no).or_insert((values, lock, exported));
    }
    out
}

/// 旧表行的 (序号, 图号, 名称) —— 供"未被球标引用的零件沿用旧序号"。
fn old_row_nos(doc: &CadDocument) -> Vec<(String, String, String)> {
    old_rows(doc)
        .into_iter()
        .map(|(no, (v, _, _))| (no, v[1].clone(), v[2].clone()))
        .collect()
}

/// **合并现有行**（用户 2026-09-15 定案）：
/// * 序号以外**非空即保留**（"无法解开的锁"：图号/名称/材料/单重/备注 手改过就不再被覆盖）；
///   空白格才写新值（空行逐渐被填满）。
/// * 数量：有锁 → 锁定值；无锁 → 用算出来的值。
pub(crate) fn merge_prev(r: &mut RowSpec, prev: &([String; 8], Option<usize>, Option<usize>)) {
    let (v, lock, exported) = prev;
    if !v[1].is_empty() {
        r.code = v[1].clone();
    }
    if !v[2].is_empty() {
        // 图纸里"名称"是 名称+规格 合并写的 → 原样保留，规格清空避免二次拼接
        r.name = v[2].clone();
        r.spec = String::new();
    }
    if !v[4].is_empty() {
        r.material = v[4].clone();
    }
    if !v[5].is_empty() {
        r.unit_weight = v[5].clone();
    }
    if !v[7].is_empty() {
        r.remark = v[7].clone();
    }
    if let Some(q) = lock.as_ref().filter(|q| **q > 0) {
        r.qty = *q;
        r.lock_qty = Some(*q);
    }
    r.exported = *exported;
}

/// **把现有表行的手改/数量锁合并进新算出的行**（BOM / BOMSYNC / 网页 apply / 导入 共用同一套政策）。
///
/// 回归背景（2026-09-17 用户要求“调查计划余项”时发现）：`BOM` 刷新路径原先只走
/// `aggregate → fill_bom` 全量重建，**不走 `merge_prev`** → 手改过的图号/名称/材料/备注
/// 与 `BOMLOCK` 数量锁会被冲掉，与 `b3df1fca` 定的“非空即保留、手改即锁”政策不一致。
/// 合并按**序号**对齐：表里的序号与本次算出来的行号一致才合并（BOMSYNC 按球标重编过号时，
/// 想让手改保留请继续用 `BOMSYNC`/网页编辑器 —— 它们本来就是按序号对齐的）。
pub(crate) fn merge_existing(rows: &mut [RowSpec], doc: &CadDocument) {
    let prev = old_rows(doc);
    if prev.is_empty() {
        return;
    }
    for r in rows.iter_mut() {
        if let Some(p) = prev.get(&r.item_no) {
            merge_prev(r, p);
        }
    }
}

/// 零件键（代号 + 材料）——与 `bom::aggregate` 的分组键一致。
fn part_key(code: &str, material: &str) -> String {
    format!("{}\u{1}{}", code.trim(), material.trim())
}

/// 排序：按序号升序（前缀字典序 + 数值，见 [`crate::balloon::item_no_key`]）。
fn sort_rows(rows: &mut [RowSpec]) {
    rows.sort_by(|a, b| {
        let (pa, va, ta) = item_no_key(&a.item_no);
        let (pb, vb, tb) = item_no_key(&b.item_no);
        (pa, va, ta).cmp(&(pb, vb, tb))
    });
}

/// **行规划**（序号驱动）：球标组每个条目一行 + 未被引用的聚合零件一行；按序号升序。
///
/// * 被球标引用的零件：序号 = 球标里引它的那个（组的**第一个条目**），数量 = max(聚合件数, 引用次数)；
/// * 球标里关联不到零件的条目：建**空行**（只写序号，其余待手填）——用户定：只靠吸附；
/// * 未被引用的聚合零件：沿用旧表里同 (图号, 名称) 的序号，没有就自动接号（最大序号 +1 递增）。
pub(crate) fn plan_rows(doc: &CadDocument) -> Result<Vec<RowSpec>, String> {
    let gs = groups(doc);
    // 序号 → 引用次数
    let mut refs: BTreeMap<String, usize> = BTreeMap::new();
    // 序号 → 关联零件（该组第一个条目）
    let mut bound: BTreeMap<String, PartMeta> = BTreeMap::new();
    for g in &gs {
        for (i, it) in g.items.iter().enumerate() {
            *refs.entry(it.clone()).or_insert(0) += 1;
            if i == 0 {
                if let Some(p) = &g.part {
                    bound.entry(it.clone()).or_insert_with(|| p.clone());
                }
            }
        }
    }

    // 聚合零件（数量按图中件数）——用 (代号, 材料) 作键。
    let parts: Vec<PartMeta> = doc
        .model_space_entities()
        .filter_map(|e| match e {
            EntityType::Insert(ins) => bom::part_meta_of(&ins.common),
            _ => None,
        })
        .collect();
    let items = bom::aggregate(&parts);

    // 已被球标绑定的零件键 → 序号（一个零件只占一个序号：取最先出现的）
    let mut taken: BTreeMap<String, String> = BTreeMap::new();
    for (no, p) in &bound {
        taken.entry(part_key(&p.code, &p.material)).or_insert_with(|| no.clone());
    }

    let mut rows: Vec<RowSpec> = Vec::new();
    // ① 球标序号：有关联零件 → 带内容；没有 → 空行
    for (no, n_refs) in &refs {
        match bound.get(no) {
            Some(p) => {
                let agg_qty = items
                    .iter()
                    .find(|it| it.code == p.code.trim() && it.material == p.material.trim())
                    .map(|it| it.qty)
                    .unwrap_or(1);
                let mut r = RowSpec {
                    item_no: no.clone(),
                    code: p.code.clone(),
                    name: p.name.clone(),
                    spec: p.spec.clone(),
                    qty: agg_qty.max(*n_refs),
                    material: p.material.clone(),
                    unit_weight: p.weight.clone(),
                    remark: String::new(),
                    lock_qty: None,
                    exported: None,
                };
                // 图上没这件（台账被删）→ 仍然列出来，数量按引用次数
                if r.name.is_empty() && r.code.is_empty() {
                    r = RowSpec::blank(no.clone());
                }
                rows.push(r);
            }
            None => rows.push(RowSpec::blank(no.clone())),
        }
    }

    // ② 未被球标引用的聚合零件：沿用旧表序号，否则自动接号
    let prev = old_rows(doc);
    let old: Vec<(String, String, String)> = prev
        .iter()
        .map(|(no, (v, _, _))| (no.clone(), v[1].clone(), v[2].clone()))
        .collect();
    let mut next_no = {
        let mut all: Vec<String> = rows.iter().map(|r| r.item_no.clone()).collect();
        all.sort_by_key(|s| item_no_key(s));
        all.last()
            .and_then(|s| item_no_next(s))
            .unwrap_or_else(|| "1".to_string())
    };
    for it in &items {
        let key = part_key(&it.code, &it.material);
        if taken.contains_key(&key) {
            continue; // 已被球标引用
        }
        // 旧表里同 (图号, 名称) 的行 → 沿用它的序号
        let name_shown = if it.spec.is_empty() {
            it.name.clone()
        } else {
            format!("{} {}", it.name, it.spec)
        };
        let reuse = old.iter().find(|(no, code, name)| {
            *code == it.code.trim()
                && (*name == name_shown || name.starts_with(&it.name))
                && !rows.iter().any(|r| r.item_no == *no)
        });
        let no = match reuse {
            Some((no, _, _)) => no.clone(),
            None => {
                // 自动接号：跳过已占用的
                while rows.iter().any(|r| r.item_no == next_no) {
                    next_no = item_no_next(&next_no).unwrap_or(next_no);
                }
                let n = next_no.clone();
                next_no = item_no_next(&n).unwrap_or(n.clone());
                n
            }
        };
        taken.insert(key, no.clone());
        let mut r = RowSpec::from_item(it, no.clone());
        if let Some(p) = prev.get(&no) {
            merge_prev(&mut r, p);
        }
        rows.push(r);
    }

    // ③ 合并现有行：非空即保留（序号以外）、数量锁优先
    for r in rows.iter_mut() {
        if let Some(p) = prev.get(&r.item_no) {
            merge_prev(r, p);
        }
    }

    // ④ **已存在的行不会自己消失**（用户定案的精神：手改过的东西不许被同步悄悄删掉）。
    //    球标拆了 / 零件台账删了，行仍然在；要让某行消失，**手工删掉它**——删行即重置。
    for (no, (v, lock, exported)) in prev.iter() {
        if rows.iter().any(|r| r.item_no == *no) {
            continue;
        }
        let mut r = RowSpec {
            item_no: no.clone(),
            code: v[1].clone(),
            name: v[2].clone(),
            material: v[4].clone(),
            unit_weight: v[5].clone(),
            remark: v[7].clone(),
            qty: v[3].parse().unwrap_or(1),
            lock_qty: *lock,
            exported: *exported,
            ..Default::default()
        };
        r.spec = String::new();
        rows.push(r);
    }

    sort_rows(&mut rows);
    Ok(rows)
}

/// 序号重编号映射（插入模式）：**每个前缀一条独立序列**。
///
/// 插入组在该前缀里占 `[最小新序号, 最小新序号 + N)`，所以该前缀里**值 ≥ 最小新序号**
/// 的已有序号全部 +N（"把插入序号之后的序号全部 +1"，N = 本次插入的条目数）；
/// 前缀不同的序号（`A1` vs `5`）互不影响。
fn renumber_map(existing: &[String], inserted: &[String]) -> BTreeMap<String, String> {
    let mut by_prefix: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    for s in inserted {
        let (p, v, tail) = item_no_key(s);
        if tail == 0 {
            by_prefix.entry(p).or_default().push(v);
        }
    }
    let mut out = BTreeMap::new();
    for x in existing {
        let (p, v, tail) = item_no_key(x);
        if tail != 0 {
            continue; // 结尾不是数字的序号不参与
        }
        let Some(vals) = by_prefix.get(&p) else {
            continue;
        };
        let base = *vals.iter().min().unwrap();
        if v < base {
            continue;
        }
        let mut cur = x.clone();
        for _ in 0..vals.len() {
            match item_no_next(&cur) {
                Some(nv) => cur = nv,
                None => break,
            }
        }
        if cur != *x {
            out.insert(x.clone(), cur);
        }
    }
    out
}

/// 新建/改号一个球标组后的**联动**：
///
/// 1. 勾了「插入序号」且新序号与已有重复 → 把**已有的**受影响球标整体重编号（重建块，
///    几何/台账一起更新；`apply_group` 由调用方注入，避免模块互相依赖）；
/// 2. 规划行（[`plan_rows`]）→ 交给 `fill` 重建明细表。
///
/// `apply_group` 参数：`(旧球标 INSERT handle, 新序号列表)` → 返回重建结果。
pub(crate) fn sync_with<F>(
    doc: &CadDocument,
    new_items: &[String],
    insert_mode: bool,
    mut apply_group: F,
) -> Result<Vec<RowSpec>, String>
where
    F: FnMut(Handle, &[String]) -> Result<(), String>,
{
    if insert_mode && !new_items.is_empty() {
        let gs = groups(doc);
        let existing: Vec<String> = gs
            .iter()
            .flat_map(|g| g.items.iter().cloned())
            .chain(old_row_nos(doc).into_iter().map(|(no, _, _)| no))
            .collect();
        let map = renumber_map(&existing, new_items);
        if !map.is_empty() {
            for g in &gs {
                let new: Vec<String> = g
                    .items
                    .iter()
                    .map(|x| map.get(x).cloned().unwrap_or_else(|| x.clone()))
                    .collect();
                if new != g.items {
                    apply_group(g.handle, &new)?;
                }
            }
        }
    }
    plan_rows(doc)
}

// ── 测试 ────────────────────────────────────────────────────────────────

// ── 验收图（联动，人工/命令路径检查）─────────────────────────────────

/// 出"序号↔明细表联动验收图"：两个零件台账 + 三个球标组（其中一个零件被两个球标引用）
/// → 在 OCS 里 `OCSM` + `BOMSYNC` 后应得到：序号 5（螺栓，数量 2）、6（空行）、7（空行）、8（偏心轴）。
///
/// `cargo test -p ocs_ocsm --lib dump_bom_sync_case -- --ignored --nocapture`
#[cfg(test)]
#[test]
#[ignore = "出验收图（手动跑）"]
fn dump_bom_sync_case() {
    use crate::partgen::acceptance_dump::add_ocsm_layers;
    use ocs_plugin_api::host::acadrust;
    use ocs_plugin_api::host::acadrust::types::Vector3 as V3;
    use ocs_plugin_api::host::acadrust::io::dwg::DwgWriter;
    use ocs_plugin_api::host::acadrust::io::dxf::DxfWriter;
    use ocs_plugin_api::host::acadrust::{xdata::ExtendedDataRecord, xdata::XDataValue};

    let mut doc = CadDocument::new();
    add_ocsm_layers(&mut doc);
    // ① 两个零件台账块（一个带外形的简单实体 + OCSM_PART）
    let parts: [(&str, (f64, f64), &str); 2] = [
        (
            "OCSM_DEMO_BOLT",
            (0.0, 0.0),
            r#"{"code":"GB/T 5782-2016","name":"六角头螺栓","spec":"M8x35","material":"Q235","weight":"0.021"}"#,
        ),
        (
            "OCSM_DEMO_SHAFT",
            (80.0, 0.0),
            r#"{"code":"0165","name":"偏心轴","spec":"φ12","material":"45","weight":"1.35"}"#,
        ),
    ];
    for (name, (x, y), meta) in parts {
        // 零件外形（简单矩形，仅为了图上看得见；台账才是数据源）
        let mut members: Vec<EntityType> = Vec::new();
        for (a, b) in [
            ((0.0, 0.0), (20.0, 0.0)),
            ((20.0, 0.0), (20.0, 12.0)),
            ((20.0, 12.0), (0.0, 12.0)),
            ((0.0, 12.0), (0.0, 0.0)),
        ] {
            let mut l = acadrust::entities::Line::from_coords(a.0, a.1, 0.0, b.0, b.1, 0.0);
            l.common.layer = "4粗实线层".into();
            members.push(EntityType::Line(l));
        }
        crate::partgen::acceptance_dump::add_block(&mut doc, name, members);
        let mut i = Insert::new(name, V3::new(x, y, 0.0));
        let mut r = ExtendedDataRecord::new(bom::XDATA_PART);
        r.values.push(XDataValue::String(meta.to_string()));
        i.common.extended_data.add_record(r);
        doc.add_entity(EntityType::Insert(i)).unwrap();
    }
    // ② 三个球标组：5=螺栓（绑定）、6/7 无零件（空行）、再来一个 5=螺栓（数量 2）
    let p_bolt = r#"{"code":"GB/T 5782-2016","name":"六角头螺栓","spec":"M8x35","material":"Q235","weight":"0.021"}"#;
    for (i, (items, part)) in [
        (vec!["5"], Some(p_bolt)),
        (vec!["6", "7"], None),
        (vec!["5"], Some(p_bolt)),
    ]
    .into_iter()
    .enumerate()
    {
        let block = format!("OCSM_XH_SYNC_{}", i + 1);
        let mut members: Vec<EntityType> = Vec::new();
        for k in 0..items.len() {
            let mut l = acadrust::entities::Line::from_coords(
                0.0,
                5.6 * k as f64,
                0.0,
                5.0,
                5.6 * k as f64,
                0.0,
            );
            l.common.layer = "8符号标注层".into();
            members.push(EntityType::Line(l));
        }
        crate::partgen::acceptance_dump::add_block(&mut doc, &block, members);
        let mut ins = Insert::new(&block, V3::new(200.0 + 40.0 * i as f64, 0.0, 0.0));
        let mut json = serde_json::json!({"items": items, "dir": "H", "ins": false});
        if let Some(p) = part {
            json["part"] = serde_json::from_str(p).unwrap();
        }
        let mut rec = ExtendedDataRecord::new(XDATA_BALLOON);
        rec.values.push(XDataValue::String(json.to_string()));
        ins.common.extended_data.add_record(rec);
        doc.add_entity(EntityType::Insert(ins)).unwrap();
    }
    let dir = std::path::Path::new("/home/ysdirector/桌面/OCSM/test");
    std::fs::create_dir_all(dir).unwrap();
    let dwg = dir.join("序号联动-验收.dwg");
    let dxf = dir.join("序号联动-验收.dxf");
    DwgWriter::write_to_file(&dwg, &doc).expect("写 DWG");
    DxfWriter::new(&doc).write_to_file(&dxf).expect("写 DXF");
    println!("已写出（在 OCS 里跑 OCSM + BOMSYNC）：\n  {}", dwg.display());
}

#[cfg(test)]
mod tests {
    use super::*;
    use ocs_plugin_api::host::acadrust::types::Vector3;

    fn ins(doc: &mut CadDocument, block: &str, pos: (f64, f64), rec: Option<(&str, &str)>) {
        let mut i = Insert::new(block, Vector3::new(pos.0, pos.1, 0.0));
        if let Some((app, json)) = rec {
            let mut r = ocs_plugin_api::host::acadrust::xdata::ExtendedDataRecord::new(app);
            r.values.push(XDataValue::String(json.to_string()));
            i.common.extended_data.add_record(r);
        }
        doc.add_entity(EntityType::Insert(i)).unwrap();
    }

    fn ball(block: &str, items: &[&str], part: Option<&str>) -> String {
        let mut v = serde_json::json!({"items": items, "dir": "H", "ins": false});
        if let Some(p) = part {
            v["part"] = serde_json::from_str(p).unwrap();
        }
        v.to_string()
    }

    /// `BOM` 刷新（以及其它路径）必须保住现有表行的手改与 `BOMLOCK` 数量锁。
    /// 回归：`cmd_bom` 曾经不走 `merge_prev` → 手改列与锁被全量重建冲掉。
    #[test]
    fn merge_existing_keeps_manual_edits_and_qty_lock() {
        use ocs_plugin_api::host::acadrust::entities::AttributeEntity;
        let mut doc = CadDocument::new();
        // 现有表行：序号 1；图号/名称/材料/备注 都是手改过的；数量锁 = 7
        let mut row = Insert::new(bom::ROW_BLOCK, Vector3::new(0.0, 0.0, 0.0));
        let vals = ["1", "GB/T 5782 手改", "六角头螺栓 M8x35", "7", "45钢", "0.02", "0.14", "自制"];
        for (tag, val) in bom::CELL_TAGS.iter().zip(vals) {
            let mut a = AttributeEntity::default();
            a.tag = tag.to_string();
            a.value = val.to_string();
            row.attributes.push(a);
        }
        let mut rec = XDataValue::String("{\"qty\":7}".to_string());
        let mut r = ocs_plugin_api::host::acadrust::xdata::ExtendedDataRecord::new(bom::LOCK_APP);
        r.values.push(rec);
        row.common.extended_data.add_record(r);
        doc.add_entity(EntityType::Insert(row)).unwrap();

        // 本次台账算出来的行：序号 1、数量 3、名称/材料未填
        let mut rows = vec![RowSpec {
            item_no: "1".to_string(),
            code: "GB/T 5782".to_string(),
            name: String::new(),
            spec: "M8x35".to_string(),
            qty: 3,
            material: String::new(),
            unit_weight: String::new(),
            remark: String::new(),
            lock_qty: None,
            exported: None,
        }];
        merge_existing(&mut rows, &doc);
        assert_eq!(rows[0].qty, 7, "数量应取锁值 7（不是重算的 3）");
        assert_eq!(rows[0].lock_qty, Some(7), "锁位要带下去，fill_bom 才会重新写锁");
        assert_eq!(rows[0].code, "GB/T 5782 手改", "手改过的图号不能被覆盖");
        assert_eq!(rows[0].name, "六角头螺栓 M8x35", "手改过的名称不能被覆盖（且规格要清空避免二次拼接）");
        assert!(rows[0].spec.is_empty(), "名称已含规格 → spec 应清空");
        assert_eq!(rows[0].material, "45钢");
        assert_eq!(rows[0].unit_weight, "0.02");
        assert_eq!(rows[0].remark, "自制");
        // 空表时是 no-op
        let mut fresh = vec![RowSpec { lock_qty: None, ..rows[0].clone() }];
        merge_existing(&mut fresh, &CadDocument::new());
        assert!(fresh[0].lock_qty.is_none());
    }

    #[test]
    fn groups_reads_items_and_part() {
        let mut doc = CadDocument::new();
        ins(
            &mut doc,
            "*XH1",
            (0.0, 0.0),
            Some((
                XDATA_BALLOON,
                &ball(
                    "*XH1",
                    &["5", "6"],
                    Some(r#"{"code":"GB/T 5782","name":"六角头螺栓","spec":"M8x35","material":"Q235","weight":"0.02"}"#),
                ),
            )),
        );
        ins(&mut doc, "*XH2", (10.0, 0.0), Some((XDATA_BALLOON, &ball("*XH2", &["A1"], None))));
        let gs = groups(&doc);
        assert_eq!(gs.len(), 2);
        assert_eq!(gs[0].items, vec!["5", "6"]);
        assert_eq!(gs[0].part.as_ref().unwrap().code, "GB/T 5782");
        assert!(gs[1].part.is_none());
    }

    #[test]
    fn plan_rows_binds_first_item_and_keeps_blank_rows() {
        let mut doc = CadDocument::new();
        ins(
            &mut doc,
            "*XH1",
            (0.0, 0.0),
            Some((
                XDATA_BALLOON,
                &ball(
                    "*XH1",
                    &["5", "6"],
                    Some(r#"{"code":"GB/T 5782","name":"六角头螺栓","spec":"M8x35","material":"Q235","weight":"0.021"}"#),
                ),
            )),
        );
        let rows = plan_rows(&doc).unwrap();
        let nos: Vec<&str> = rows.iter().map(|r| r.item_no.as_str()).collect();
        assert_eq!(nos, vec!["5", "6"]);
        assert_eq!(rows[0].name, "六角头螺栓");
        assert_eq!(rows[0].qty, 1);
        assert_eq!(rows[0].values()[5], "0.021");
        // 第二个条目关联不到零件 → 空行（只写序号）
        assert!(rows[1].name.is_empty() && rows[1].code.is_empty());
        assert_eq!(rows[1].values()[3], "");
    }

    #[test]
    fn plan_rows_appends_unreferenced_parts_after_the_last_number() {
        let mut doc = CadDocument::new();
        ins(&mut doc, "*XH1", (0.0, 0.0), Some((XDATA_BALLOON, &ball("*XH1", &["1"], None))));
        // 零件台账：一个被引用的（不绑到序号上，因为组没关联零件 → 走"未引用"路径接号）
        ins(
            &mut doc,
            "OCSM_A",
            (50.0, 50.0),
            Some((
                bom::XDATA_PART,
                r#"{"code":"0165","name":"偏心轴","spec":"φ12","material":"45","weight":"1.35"}"#,
            )),
        );
        let rows = plan_rows(&doc).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].item_no, "1");
        assert_eq!(rows[1].item_no, "2");
        assert_eq!(rows[1].code, "0165");
        assert_eq!(rows[1].qty, 1);
    }

    #[test]
    fn plan_rows_reuses_old_row_number_for_same_part() {
        use ocs_plugin_api::host::acadrust::entities::AttributeEntity;
        let mut doc = CadDocument::new();
        ins(&mut doc, "*XH1", (0.0, 0.0), Some((XDATA_BALLOON, &ball("*XH1", &["1"], None))));
        ins(
            &mut doc,
            "OCSM_A",
            (50.0, 50.0),
            Some((
                bom::XDATA_PART,
                r#"{"code":"0165","name":"偏心轴","spec":"φ12","material":"45","weight":"1.35"}"#,
            )),
        );
        // 旧表里已有「序号 7 = 0165 偏心轴 φ12」→ 刷新后应沿用 7，而不是接 2
        let mut row = Insert::new(bom::ROW_BLOCK, Vector3::new(0.0, 0.0, 0.0));
        for (tag, val) in [("序号", "7"), ("图号", "0165"), ("名称", "偏心轴 φ12")] {
            let mut a = AttributeEntity::default();
            a.tag = tag.into();
            a.value = val.into();
            row.attributes.push(a);
        }
        doc.add_entity(EntityType::Insert(row)).unwrap();
        let rows = plan_rows(&doc).unwrap();
        let nos: Vec<&str> = rows.iter().map(|r| r.item_no.as_str()).collect();
        assert_eq!(nos, vec!["1", "7"], "同件应沿用旧序号");
    }

    /// 造一行现成的明细表行（8 个属性 + 可选数量锁）。
    fn push_row(doc: &mut CadDocument, no: &str, cells: [&str; 8], lock: Option<usize>) {
        use ocs_plugin_api::host::acadrust::entities::AttributeEntity;
        let mut row = Insert::new(bom::ROW_BLOCK, Vector3::new(0.0, 0.0, 0.0));
        for (tag, val) in bom::CELL_TAGS.iter().zip(cells.iter()) {
            let mut a = AttributeEntity::default();
            a.tag = tag.to_string();
            a.value = val.to_string();
            row.attributes.push(a);
        }
        if let Some(q) = lock {
            let mut r = ocs_plugin_api::host::acadrust::xdata::ExtendedDataRecord::new(bom::LOCK_APP);
            r.values.push(XDataValue::String(
                serde_json::json!({"qty": q}).to_string(),
            ));
            row.common.extended_data.add_record(r);
        }
        doc.add_entity(EntityType::Insert(row)).unwrap();
    }

    #[test]
    fn plan_rows_never_drops_existing_rows() {
        let mut doc = CadDocument::new();
        // 图上既没有球标也没有零件台账，但表里有一行（xlsx 里手工加的）
        push_row(&mut doc, "9", ["9", "9999", "手加件", "2", "Q235", "0.5", "1", "试导入"], None);
        let rows = plan_rows(&doc).unwrap();
        assert_eq!(rows.len(), 1, "已存在的行不能被同步丢掉");
        assert_eq!(rows[0].item_no, "9");
        assert_eq!(rows[0].name, "手加件");
        assert_eq!(rows[0].qty, 2);
        assert_eq!(rows[0].remark, "试导入");
    }

    #[test]
    fn plan_rows_keeps_hand_edited_cells_but_recomputes_unlocked_qty() {
        let mut doc = CadDocument::new();
        let p = r#"{"code":"GB/T 6170","name":"六角螺母","spec":"M8","material":"Q235","weight":"0.01"}"#;
        ins(&mut doc, "*XH1", (0.0, 0.0), Some((XDATA_BALLOON, &ball("*XH1", &["3"], Some(p)))));
        // 现成行：图号/名称/材料/备注 都被手改过；数量 9（没锁）
        push_row(
            &mut doc,
            "3",
            ["3", "手改图号", "手改名称", "9", "手改材料", "0.01", "0.09", "外购"],
            None,
        );
        let rows = plan_rows(&doc).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, "手改图号", "图号非空即保留");
        assert_eq!(rows[0].name, "手改名称");
        assert_eq!(rows[0].material, "手改材料");
        assert_eq!(rows[0].remark, "外购");
        assert_eq!(rows[0].qty, 1, "无锁 → 数量按规则重算（引用 1 次）");
        assert!(rows[0].lock_qty.is_none());
    }

    #[test]
    fn plan_rows_honours_qty_lock() {
        let mut doc = CadDocument::new();
        let p = r#"{"code":"GB/T 6170","name":"六角螺母","spec":"M8","material":"Q235","weight":"0.01"}"#;
        ins(&mut doc, "*XH1", (0.0, 0.0), Some((XDATA_BALLOON, &ball("*XH1", &["3"], Some(p)))));
        push_row(
            &mut doc,
            "3",
            ["3", "", "", "9", "", "", "", ""],
            Some(9),
        );
        let rows = plan_rows(&doc).unwrap();
        assert_eq!(rows[0].qty, 9, "锁定量优先");
        assert_eq!(rows[0].lock_qty, Some(9), "锁要跟着新行写回去");
        // 空白格被填上（空行逐渐填满）
        assert_eq!(rows[0].code, "GB/T 6170");
        assert_eq!(rows[0].material, "Q235");
    }

    #[test]
    fn renumber_map_shifts_following_numbers_in_same_series() {
        let existing: Vec<String> = ["1", "2", "5", "6", "A1"].iter().map(|s| s.to_string()).collect();
        // 插一个 5 → 同前缀里 5、6 后移；A1 属另一条序列，不动
        let ins_one: Vec<String> = vec!["5".to_string()];
        let m = renumber_map(&existing, &ins_one);
        assert_eq!(m.get("5").unwrap(), "6");
        assert_eq!(m.get("6").unwrap(), "7");
        assert!(m.get("A1").is_none(), "前缀不同不串号");
        assert!(m.get("1").is_none() && m.get("2").is_none(), "插入点之前的序号不动");
        // 插两个（5、6）→ 占 [5,7)，其后 +2
        let ins_two: Vec<String> = vec!["5".to_string(), "6".to_string()];
        let m2 = renumber_map(&existing, &ins_two);
        assert_eq!(m2.get("5").unwrap(), "7");
        assert_eq!(m2.get("6").unwrap(), "8");
        // 插 A1 → 只有 A 序列的 A1 后移
        let ins_a: Vec<String> = vec!["A1".to_string()];
        let m3 = renumber_map(&existing, &ins_a);
        assert_eq!(m3.len(), 1);
        assert_eq!(m3.get("A1").unwrap(), "A2");
    }

    #[test]
    fn sync_renumbers_existing_groups_only_when_insert_mode() {
        let mut doc = CadDocument::new();
        ins(&mut doc, "*XH1", (0.0, 0.0), Some((XDATA_BALLOON, &ball("*XH1", &["5"], None))));
        let mut calls: Vec<(Handle, Vec<String>)> = Vec::new();
        let five: Vec<String> = vec!["5".to_string()];
        let rows = sync_with(&doc, &five, true, |h, items| {
            calls.push((h, items.to_vec()));
            Ok(())
        })
        .unwrap();
        assert_eq!(calls.len(), 1, "勾选插入 → 旧球标应被重编号");
        assert_eq!(calls[0].1, vec!["6"]);
        // 行规划仍按**现有文档**算（重编号由调用方落地后才会反映），这里只验证它会跑
        assert!(rows.iter().any(|r| r.item_no == "5"));

        // 不勾选：不动旧球标（数量 +1 由 refs 统计自然发生）
        let mut calls2 = Vec::new();
        let _ = sync_with(&doc, &five, false, |h, items| {
            calls2.push((h, items.to_vec()));
            Ok(())
        })
        .unwrap();
        assert!(calls2.is_empty(), "不勾选 → 不改旧球标");
    }

    #[test]
    fn refs_increment_qty_when_same_number_reused() {
        let mut doc = CadDocument::new();
        let p = r#"{"code":"GB/T 6170","name":"六角螺母","spec":"M8","material":"Q235","weight":"0.01"}"#;
        ins(&mut doc, "*XH1", (0.0, 0.0), Some((XDATA_BALLOON, &ball("*XH1", &["3"], Some(p)))));
        ins(&mut doc, "*XH2", (20.0, 0.0), Some((XDATA_BALLOON, &ball("*XH2", &["3"], Some(p)))));
        let rows = plan_rows(&doc).unwrap();
        assert_eq!(rows.len(), 1, "同一序号只占一行");
        assert_eq!(rows[0].item_no, "3");
        assert_eq!(rows[0].qty, 2, "两个球标引同一序号 → 数量 2");
        assert_eq!(rows[0].values()[6], "0.02", "总重 = 0.01 × 2");
    }
}
