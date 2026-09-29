//! 视觉冒烟：OCSM 引线标注（`*L{n}` 匿名块 + INSERT）在宿主里的实际渲染。
//!
//! 输入 = 插件冒烟写出的 DXF（`OCSM_LEADER_SMOKE_DXF`），输出 = PDF
//! （`OCSM_LEADER_PDF`）。与 `dim_leader_render_check.rs` 同套路：
//! `Scene::entity_wires() → export_pdf()`，用于人工/图像核对"引线+箭头+肩线+
//! 上下侧文字"的观感（数值正确性已由插件侧单测覆盖）。
use codec::EntityType;
use OpenCADStudio::io::load_file;
use OpenCADStudio::io::pdf_export::{export_pdf, PdfPageInput, PdfPlotOptions, PlotContent, PlotWire};
use OpenCADStudio::scene::Scene;

#[test]
fn leader_smoke_renders() {
    let dxf = std::env::var("OCSM_LEADER_SMOKE_DXF").unwrap_or_default();
    let out = std::env::var("OCSM_LEADER_PDF").unwrap_or_default();
    if dxf.trim().is_empty() {
        return; // 未设环境变量：跳过（CI 无副作用）
    }
    let doc = load_file(std::path::Path::new(&dxf)).expect("load leader smoke dxf");
    let mut scene = Scene::new();
    scene.document = doc;
    scene.bump_geometry(); // 让 entity_wires() 重新 tessellate（缓存按几何纪元）
    let wires: Vec<PlotWire> = scene
        .entity_wires()
        .iter()
        .map(|w| PlotWire {
            wire: w.clone(),
            draw_depth: 0.0,
        })
        .collect();
    assert!(!wires.is_empty(), "场景应产生几何（引线/肩线/箭头/文字）");
    // 至少有块引用被 tessellate（INSERT → 块内实体）。
    let has_insert = scene
        .document
        .entities()
        .any(|e| matches!(e, EntityType::Insert(_)));
    assert!(has_insert, "冒烟图纸应含 INSERT");

    if out.trim().is_empty() {
        return;
    }
    let path = std::path::PathBuf::from(&out);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    let wire_count = wires.len();
    export_pdf(
        &PdfPageInput {
            content: PlotContent {
                wires: std::sync::Arc::new(wires),
                ..Default::default()
            },
            paper_w: 210.0,
            paper_h: 297.0,
            offset_x: 0.0,
            offset_y: 0.0,
            rotation_deg: 0,
            scale: 1.0,
            clip: None,
            options: PdfPlotOptions::default(),
            plot_style: None,
        },
        &path,
    )
    .expect("export pdf");
    println!("leader smoke pdf written: {out} ({} wires)", wire_count);
}
