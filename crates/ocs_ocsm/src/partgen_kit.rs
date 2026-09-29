//! 参数化标准件**共用工具箱**（第四批起的 partgen_b1…b4 用）。
//!
//! 为什么另起一个文件：第四批是 11 个族、由**多个并行分支**各自实现，若都把
//! 图元构造/表加载写进 `partgen.rs`/`partgen_more.rs`，同一个文件会被反复改、
//! 冲突不断。这里把「与族无关」的东西集中一处，各族只在自己的模块里写画法与数据。
//!
//! 图层/颜色约定与 `partgen.rs` **完全一致**（都 ByLayer，颜色随层）：
//! - `1轮廓实线层` 粗实线；`2细线层` 牙底/细线；`3中心线层` 点划线；
//! - `4虚线层` 不可见轮廓；`5剖面线层` 剖面线。
//!
//! 圆弧的角度参数一律用**度**（与 DXF/GB 画法一致），内部自己转弧度
//! （acadrust 的 `Arc` 角度字段是弧度——这是本库最容易踩的坑）。

use ocs_plugin_api::host::acadrust::entities::{Arc, Circle, EntityType, Hatch, Line, LwPolyline};
use ocs_plugin_api::host::acadrust::types::{Color, LineWeight, Vector3};

pub use crate::partgen::{
    across_corners, minor_dia, spec_text_bolt, spec_text_dia, to_svg, BoltView, GenPart, PartMeta,
};

/// 轮廓（粗实线）层。
pub const LAYER_MAIN: &str = "1轮廓实线层";
/// 细实线层（牙底、细线、螺纹大径细线）。
pub const LAYER_THIN: &str = "2细线层";
/// 中心线层（点划线）。
pub const LAYER_CENTER: &str = "3中心线层";
/// 虚线层（不可见轮廓）。
pub const LAYER_HIDDEN: &str = "4虚线层";
/// 剖面线层（剖视图的金属剖面线）。
pub const LAYER_HATCH: &str = "5剖面线层";

// ── 基础图元 ───────────────────────────────────────────────────────────────

/// 一条线（落在指定图层）。
pub fn line(a: [f64; 2], b: [f64; 2], layer: &str) -> EntityType {
    let mut l = Line::from_points(
        Vector3::new(a[0], a[1], 0.0),
        Vector3::new(b[0], b[1], 0.0),
    );
    set_layer(&mut l, layer);
    EntityType::Line(l)
}

/// 圆弧：入参用**度**（与 DXF/GB 画法一致），内部转弧度。
pub fn arc(c: [f64; 2], r: f64, start_deg: f64, end_deg: f64, layer: &str) -> EntityType {
    let mut a = Arc::from_center_radius_angles(
        Vector3::new(c[0], c[1], 0.0),
        r,
        start_deg.to_radians(),
        end_deg.to_radians(),
    );
    set_layer(&mut a, layer);
    EntityType::Arc(a)
}

/// 整圆。
pub fn circle(c: [f64; 2], r: f64, layer: &str) -> EntityType {
    let mut e = Circle::from_center_radius(Vector3::new(c[0], c[1], 0.0), r);
    set_layer(&mut e, layer);
    EntityType::Circle(e)
}

// ── 库内共有的螺纹大径 3/4 细弧（孔生成器俯视 / 轴生成器导向平键槽共用）─────────

/// 螺纹大径 3/4 细弧起始角（度；DXF ARC 逆时针，可直接落图）。
///
/// **口径来源（用户 2026-09-25 裁定：以模板实际几何为准，不机械抄 DXF 原始数字）**：
/// 模板 `~/桌面/OCSM/轴生成器-导向平键槽.dxf` 的 ARC 原始记录为 `start=625°、end=545°`；
/// DXF 规定 ARC 自 start 逆时针画到 end，换算成实际弧区间 = `[265°, 545°]`（扫过 280°），
/// 缺口 = `(185°, 265°)`（中心 225° 左下、宽 80°）；归一化起止 = **265° → 185°**。
/// 旧的 `270° → 180°`（缺口中心同为 225°，但端点各差 5°）按用户裁定作废。
/// ⚠️ 其余标准件端视（螺母/螺钉等）另有各自源模板，不得套用本口径。
pub const THREAD_MAJOR_ARC_START_DEG: f64 = 265.0;

/// 螺纹大径 3/4 细弧终止角（度；换算见 [`THREAD_MAJOR_ARC_START_DEG`]）。
pub const THREAD_MAJOR_ARC_END_DEG: f64 = 185.0;

/// 按库内共有口径画螺纹大径 3/4 细弧（`2细线层`）：入参 = 圆心 + 大径半径。
pub fn thread_major_arc(center: [f64; 2], major_r: f64) -> EntityType {
    arc(
        center,
        major_r,
        THREAD_MAJOR_ARC_START_DEG,
        THREAD_MAJOR_ARC_END_DEG,
        LAYER_THIN,
    )
}

/// 点列 → 单条 LwPolyline（`closed=true` 闭合）。虚线/中心线/异形轮廓都用它。
pub fn polyline(pts: &[[f64; 2]], closed: bool, layer: &str) -> EntityType {
    use ocs_plugin_api::host::acadrust::entities::LwVertex;
    use ocs_plugin_api::host::acadrust::types::Vector2;
    let mut pl = LwPolyline::new();
    for p in pts {
        let mut v = LwVertex::new(Vector2::new(p[0], p[1]));
        v.bulge = 0.0;
        pl.vertices.push(v);
    }
    pl.is_closed = closed;
    set_layer(&mut pl, layer);
    EntityType::LwPolyline(pl)
}

/// ANSI31 图案填充（落在 `5剖面线层`）：边界为闭合折线，`angle_deg` 传度数（默认比例 1.0）。
pub fn hatch_ansi31(verts: &[[f64; 2]], angle_deg: f64) -> EntityType {
    hatch_ansi31_scaled(verts, angle_deg, 1.0)
}

/// 剖面线边界的一段（角度用**度**，逆时针为正）。
#[derive(Debug, Clone, Copy)]
pub enum HatchEdge {
    /// 直线段
    Line { a: [f64; 2], b: [f64; 2] },
    /// 圆弧段（`ccw` = 从 `start_deg` 到 `end_deg` 是否逆时针）
    Arc {
        c: [f64; 2],
        r: f64,
        start_deg: f64,
        end_deg: f64,
        ccw: bool,
    },
}

/// ANSI31 图案定义的**基准垂直间距**（0.125″ = 3.175 mm）——宿主反旋转 offset 后读出的 `|dy|`。
pub const ANSI31_SPACING_MM: f64 = 3.175;

/// 某 `pattern_scale` 下剖面线的**垂直距离**（mm）= 宿主 `scene/entity.rs::family_from_stored_line`
/// 读出的 `|dy|`。
///
/// **全库唯一口径**：出图（[`hatch_ansi31_edges`] / [`hatch_ansi31_rings`] / [`hatch_ansi37_edges`]）
/// 与预览（`partgen::hatch_svg_lines`、`gear::svg_of`）都从这里取，禁止再各写 3.175/45° 的第二套。
pub fn hatch_perpendicular_spacing(pattern_scale: f64) -> f64 {
    ANSI31_SPACING_MM * pattern_scale
}

/// ANSI31 图案 45° 基准下 offset 矢量分量 = 垂距/√2。
///
/// 世界向量写成 `(-c, +c)` 时宿主反旋转后 `dx = 0`、`|dy| = 垂距`；写成 `(+c, +c)` 会
/// `dy = 0` → 宿主画成实心（见 [`hatch_edges_with`] 的写法硬约束①）。
pub fn hatch_offset_component(pattern_scale: f64) -> f64 {
    hatch_perpendicular_spacing(pattern_scale) / std::f64::consts::SQRT_2
}

/// 用任意（直线/圆弧）边界画 ANSI31（5剖面线层）。
///
/// **模板里「深沟球轴承 GB/T 276」「密封圈 FB」的剖面线边界带圆弧**（滚道弧/唇口圆弧），
/// 纯折线版本（`hatch_ansi31_scaled`）画不了，用这个。
pub fn hatch_ansi31_edges(edges: &[HatchEdge], angle_deg: f64, pattern_scale: f64) -> EntityType {
    let off = hatch_offset_component(pattern_scale);
    hatch_edges_with(
        "ANSI31",
        "ANSI Iron, Brick, Stone masonry",
        &[(45.0, -off, off)],
        &[edges.to_vec()],
        angle_deg,
        pattern_scale,
        false,
    )
}

/// 与 [`hatch_ansi31_edges`] 同图案同开关，但边界是**多个独立环**（每个环一条
/// BoundaryPath）。轴剖视用：轴线上/下各一环（`~/桌面/OCSM/review/轴剖视图.dxf`
/// 右视图口径：一个 HATCH、2 环 21+21 边、flags=external|outermost）。
pub fn hatch_ansi31_rings(
    rings: &[Vec<HatchEdge>],
    angle_deg: f64,
    pattern_scale: f64,
) -> EntityType {
    let off = hatch_offset_component(pattern_scale);
    hatch_edges_with(
        "ANSI31",
        "ANSI Iron, Brick, Stone masonry",
        &[(45.0, -off, off)],
        rings,
        angle_deg,
        pattern_scale,
        true,
    )
}

/// 用任意边界画 **ANSI37（双向网纹 45°+135°）**。
///
/// 模板里**橡胶/非金属**的剖面用这个（GB/T 4457.5 材质剖面线约定）：
/// 密封圈 FB 型的唇口橡胶两片在模板里就是 `ANSI37`（金属骨架两片是 `ANSI31`），
/// 两者同名同边界、只差图案 —— 不能都用 ANSI31（视觉上少一个方向）。
pub fn hatch_ansi37_edges(edges: &[HatchEdge], angle_deg: f64, pattern_scale: f64) -> EntityType {
    let off = hatch_offset_component(pattern_scale);
    hatch_edges_with(
        "ANSI37",
        "ANSI Lead, Zinc, Magnesium, Sound/Heat/Elec Insulation",
        &[(45.0, -off, off), (135.0, -off, -off)],
        &[edges.to_vec()],
        angle_deg,
        pattern_scale,
        false,
    )
}

/// 剖面线通用实现。`base_lines` = 基准方向下的 `(线角度, offset.x, offset.y)`（即 dir=0 时的定义）。
///
/// ## 两条“写法硬约束”（都是踩过的坑，别再改回去）
///
/// **① offset 必须是「世界坐标向量」，且让宿主反旋转后得到 dx≈0 / |dy|=线间距。**
/// 宿主与预览都按 `scene/entity.rs::family_from_stored_line` 的读法：
/// `dx = off.x·cos(a)+off.y·sin(a)`（沿线位移）、`dy = -off.x·sin(a)+off.y·cos(a)`（垂距）。
/// 对 ANSI31（a=45°）基准 offset 必须是 **(-off, +off)**（off = [`hatch_offset_component`]）：
/// 反旋转后 dx=0、dy=+off ✓；写成 (+off,+off) 会得到 dx=off、**dy=0 → 间距塔缩 → 宿主渲染成**
/// **实心填充**（用户 2026-09-17 报的“轴承剖面线变纯色填充”就是这个，根因是本文件 refactor 时丢了负号）。
///
/// **② 方向要烘焙到线角度/offset 里，`pattern_angle` 只是记录值。**
/// 模板实测（用户参数化图：6170 剖视 / 276 / 297 / 288 主视）：
/// `line.angle = 45 + dir`、`offset = rotate(基准 offset, dir)`、同时 `pattern_angle = dir`；
/// 宿主 `prebaked` 路径**只用线角度、不再叠加 pattern_angle**（`angle_offset: 0.0`），
/// 所以只改 pattern_angle 不改线角度的话，四片会在 OCS 里全画成 45° 同向。
fn hatch_edges_with(
    name: &str,
    description: &str,
    base_lines: &[(f64, f64, f64)],
    rings: &[Vec<HatchEdge>],
    angle_deg: f64,
    pattern_scale: f64,
    outermost: bool,
) -> EntityType {
    use ocs_plugin_api::host::acadrust::entities::hatch::{
        BoundaryEdge, BoundaryPath, BoundaryPathFlags, CircularArcEdge, HatchPattern,
        HatchPatternLine, LineEdge,
    };
    use ocs_plugin_api::host::acadrust::types::Vector2;
    let mut h = Hatch::new();
    let mut pat = HatchPattern::new(name);
    pat.description = description.into();
    let rot = angle_deg.to_radians();
    let (cr, sr) = (rot.cos(), rot.sin());
    for (deg, ox, oy) in base_lines {
        pat.add_line(HatchPatternLine {
            // 方向烘焙进线角度（模板就是这么存的：45° 基向 + dir）
            angle: deg.to_radians() + rot,
            base_point: Vector2::new(0.0, 0.0),
            // offset 同步旋转（保持 dx≈0 / |dy|=间距 不变）
            offset: Vector2::new(ox * cr - oy * sr, ox * sr + oy * cr),
            dash_lengths: Vec::new(),
        });
    }
    h.pattern = pat;
    h.is_solid = false;
    // 与模板一致：记录值（读者以线角度为准）
    h.pattern_angle = rot;
    h.pattern_scale = pattern_scale;
    for ring in rings {
        let mut bp = BoundaryPath::new();
        // 参考件口径（`轴剖视图.dxf`）：多环时 flags = EXTERNAL | OUTERMOST（DXF 17）；
        // 单环保持既有 EXTERNAL（模板同构），不动老输出。
        bp.flags = if outermost {
            BoundaryPathFlags::from_bits(
                BoundaryPathFlags::EXTERNAL.bits() | BoundaryPathFlags::OUTERMOST.bits(),
            )
        } else {
            let mut flags = BoundaryPathFlags::new();
            flags.set_external(true);
            flags
        };
        for e in ring {
            match *e {
                HatchEdge::Line { a, b } => bp.add_edge(BoundaryEdge::Line(LineEdge {
                    start: Vector2::new(a[0], a[1]),
                    end: Vector2::new(b[0], b[1]),
                })),
                HatchEdge::Arc {
                    c,
                    r,
                    start_deg,
                    end_deg,
                    ccw,
                } => bp.add_edge(BoundaryEdge::CircularArc(CircularArcEdge {
                    center: Vector2::new(c[0], c[1]),
                    radius: r,
                    start_angle: start_deg.to_radians(),
                    end_angle: end_deg.to_radians(),
                    counter_clockwise: ccw,
                })),
            }
        }
        h.paths.push(bp);
    }
    set_layer(&mut h, LAYER_HATCH);
    EntityType::Hatch(h)
}

/// 同上，但边界用**闭合折线**（顶点顺序任意，闭合由工具处理）。
pub fn hatch_ansi31_scaled(verts: &[[f64; 2]], angle_deg: f64, pattern_scale: f64) -> EntityType {
    let edges: Vec<HatchEdge> = (0..verts.len())
        .map(|i| HatchEdge::Line {
            a: verts[i],
            b: verts[(i + 1) % verts.len()],
        })
        .collect();
    hatch_ansi31_edges(&edges, angle_deg, pattern_scale)
}

/// 把实体归到某图层并统一 ByLayer（颜色/线型/线宽随层）。
pub fn set_layer<T: CommonLayer>(e: &mut T, layer: &str) {
    let c = e.common_mut();
    c.layer = layer.to_string();
    c.color = Color::ByLayer;
    c.linetype = "ByLayer".to_string();
    c.line_weight = LineWeight::ByLayer;
}

/// 让 `set_layer` 同时适配 Line/Arc/Circle/LwPolyline/Hatch。
pub trait CommonLayer {
    fn common_mut(&mut self) -> &mut ocs_plugin_api::host::acadrust::entities::EntityCommon;
}

macro_rules! impl_common_layer {
    ($($t:ty),* $(,)?) => {$(
        impl CommonLayer for $t {
            fn common_mut(&mut self) -> &mut ocs_plugin_api::host::acadrust::entities::EntityCommon {
                &mut self.common
            }
        }
    )*};
}
impl_common_layer!(Line, Arc, Circle, Hatch, LwPolyline);

/// 数字→规格文本用的紧凑格式（去掉多余的 0）。
pub fn trim(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

// ── 表加载 ─────────────────────────────────────────────────────────────────

/// 数据表 JSON 的通用外壳（`tables/parts*.json` 统一用这四个键）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Table<T> {
    #[allow(dead_code)]
    pub family: String,
    #[allow(dead_code)]
    pub code: String,
    #[allow(dead_code)]
    pub iso: String,
    pub rows: Vec<T>,
}

impl<T: serde::de::DeserializeOwned> Table<T> {
    /// 解析一张表（配合 `include_str!` + `OnceLock` 用，见各族的 `xxx_table()`）。
    pub fn parse(json: &str) -> Self {
        serde_json::from_str(json).expect(&crate::i18n::t("cmd.parts.err.json_parse_parts"))
    }
    pub fn row(&self, d: f64) -> Option<&T>
    where
        T: HasD,
    {
        self.rows.iter().find(|r| (r.d() - d).abs() < 1e-9)
    }
}

/// 表行的公称直径（各族行结构都带 `d`）。
pub trait HasD {
    fn d(&self) -> f64;
}

// ── 视图与校验 ─────────────────────────────────────────────────────────────

/// 某族的视图注册表 → GUI 用的 JSON（`[{id,name}]`）。
///
/// 视图名的**唯一数据源**是 `partgen_more::family_views`（它会把新批次的
/// `partgen_bN::family_views` 也算进去），这里只做名字本地化，别另建一份。
pub fn views_json(family: &str) -> serde_json::Value {
    serde_json::Value::Array(
        crate::partgen_more::family_views(family)
            .into_iter()
            .map(|id| {
                let name = match id {
                    "main" => "主视图",
                    "section" => "剖视图",
                    "top" => "俯视图",
                    "end" => "左视图",
                    other => other,
                };
                serde_json::json!({ "id": id, "name": name })
            })
            .collect(),
    )
}

/// 视图名（`main`/`top`/`end`/`section`）→ `BoltView`（只有 main/top/end 的老三视图族用）。
pub fn bolt_view(view: &str) -> Result<BoltView, String> {
    match view {
        "main" => Ok(BoltView::Main),
        "top" => Ok(BoltView::Top),
        "end" => Ok(BoltView::End),
        other => Err(crate::i18n::t_fmt(
            "cmd.parts.err.view_absent_no_family",
            &[("view", other), ("avail", "main/top/end")],
        )),
    }
}

/// 长度范围校验（错误文本与既有族一致，GUI/CLI 直接显示）。
pub fn check_length(l_min: f64, l_max: f64, d: f64, l: f64) -> Result<(), String> {
    if l < l_min - 1e-9 || l > l_max + 1e-9 {
        return Err(crate::i18n::t_fmt(
            "cmd.parts.err.len_range_kit",
            &[
                ("d", &trim(d)),
                ("lo", &trim(l_min)),
                ("hi", &trim(l_max)),
                ("l", &trim(l)),
            ],
        ));
    }
    Ok(())
}

/// 重量估算（钢 7.85 g/cm³）：体积 mm³ → kg。各族自己算体积。
pub fn weight_kg(volume_mm3: f64, density_g_cm3: f64) -> f64 {
    volume_mm3 * density_g_cm3 * 1e-3 / 1000.0
}

/// 重量文本（kg，4 位有效小数；与既有族一致）。
pub fn weight_text(kg: f64) -> String {
    let s = format!("{kg:.4}");
    s
}

// ── 自检/导出（人工核对用）──────────────────────────────────────────────────

/// 单视图 SVG（与 `partgen_more::view_svg` 同一渲染器）——给人工核对与叠合比对用。
pub fn part_svg(part: &GenPart, px_w: f64, px_h: f64) -> String {
    to_svg(part, &format!("{} {}", part.meta.code, part.meta.spec), px_w, px_h)
}

/// 测试里把生成结果落盘成 SVG（`dump_*` 测试用；路径放 `/tmp`）。
pub fn dump_svg(part: &GenPart, path: &str) -> std::io::Result<()> {
    std::fs::write(path, part_svg(part, 1200.0, 800.0))
}

#[cfg(test)]
mod tests {
    use super::HatchEdge;
    use ocs_plugin_api::host::acadrust::EntityType;

    /// 剖面线写入的**全局护栏**（管住整库的剖面线，2026-09-17 用户报"轴承剖面线渲染成纯色填充"后加）。
    ///
    /// ① **片数**：各族的剖面线片数必须等于用户模板实测值（此前因误判"模板没有剖面线"漏画过 6 处）。
    /// ② **图案定义可被宿主正确读出**：宿主/预览都按 `scene/entity.rs::family_from_stored_line` 的读法
    ///    `dx = off.x·cos a + off.y·sin a`（沿线位移）、`dy = -off.x·sin a + off.y·cos a`（垂距）。
    ///    必须 `dx ≈ 0` 且 `|dy| = 3.175 mm × pattern_scale`；**dy = 0 就是间距塌缩 → 宿主画成实心**
    ///    （把 ANSI31 基准 offset 写成 (+off,+off) 就会这样）。
    /// ③ **方向烘焙在线角度里**（模板实测存法）：`line.angle = 45°/135° + dir`，
    ///    且 `pattern_angle = dir`（记录值）；只改 pattern_angle 不改线角度，OCS 里四片会同向。
    /// 剖面线间距的**唯一口径**函数（出图与预览共用）：
    /// `hatch_perpendicular_spacing` = 宿主读出的 |dy|；`hatch_offset_component` = 垂距/√2。
    #[test]
    fn hatch_spacing_helpers_are_single_source() {
        assert!((super::hatch_perpendicular_spacing(1.0) - 3.175).abs() < 1e-12);
        assert!(
            (super::hatch_offset_component(1.0) - 3.175 / std::f64::consts::SQRT_2).abs() < 1e-12
        );
        // 齿轮/花键口径 3.0mm：图案比例 = 3.0/3.175 → offset 分量 = 3.0/√2、垂距 = 3.0。
        let scale = 3.0 / super::ANSI31_SPACING_MM;
        assert!((super::hatch_perpendicular_spacing(scale) - 3.0).abs() < 1e-12);
        assert!(
            (super::hatch_offset_component(scale) - 3.0 / std::f64::consts::SQRT_2).abs() < 1e-12
        );
        // 实体图案定义必须与函数同值（出图路径走的就是这两个函数）。
        let ring = vec![HatchEdge::Line {
            a: [0.0, 0.0],
            b: [10.0, 0.0],
        }];
        let h = super::hatch_ansi31_rings(&[ring], 0.0, scale);
        let EntityType::Hatch(hat) = h else {
            panic!("应为 HATCH")
        };
        let ln = &hat.pattern.lines[0];
        let dy = (-ln.offset.x * ln.angle.sin() + ln.offset.y * ln.angle.cos()).abs();
        assert!((dy - super::hatch_perpendicular_spacing(hat.pattern_scale)).abs() < 1e-12);
        assert!((dy - 3.0).abs() < 1e-12, "齿轮口径垂距应为 3.0mm，实得 {dy}");
    }

    #[test]
    fn hatch_patterns_are_readable_by_host() {
        let cases = [
            ("nut_6170", 8.0, 6.8, "section", 2),
            ("ring_893", 50.0, 2.0, "end", 1),
            ("ring_894", 40.0, 1.75, "end", 1),
            ("round_nut_812", 22.0, 10.0, "section", 2),
            ("lock_washer_858", 25.0, 1.0, "section", 2),
            ("lock_washer_858", 160.0, 2.0, "section", 2),
            ("seal_fb", 16.0, 30.0, "main", 4),
            ("bearing_276", 35.0, 7.0, "main", 4),
            ("bearing_297", 15.0, 11.75, "main", 4),
            ("bearing_288", 110.0, 45.0, "main", 6),
        ];
        for (fam, d, l, view, want) in cases {
            let p = crate::partgen::generate(fam, d, l, view)
                .unwrap_or_else(|e| panic!("{fam} {view}: {e}"));
            let hatches: Vec<_> = p
                .entities
                .iter()
                .filter_map(|e| match e {
                    EntityType::Hatch(h) => Some(h),
                    _ => None,
                })
                .collect();
            assert_eq!(
                hatches.len(),
                want,
                "{fam} {view} 剖面线片数应为 {want}（模板实测）"
            );
            for h in hatches {
                let scale = h.pattern_scale;
                let dir = h.pattern_angle.to_degrees();
                assert!(
                    !h.pattern.lines.is_empty(),
                    "{fam} {view}: 剖面线缺图案定义（会被宿主当实心画）"
                );
                for ln in &h.pattern.lines {
                    let (ca, sa) = (ln.angle.cos(), ln.angle.sin());
                    let dx = ln.offset.x * ca + ln.offset.y * sa;
                    let dy = -ln.offset.x * sa + ln.offset.y * ca;
                    assert!(
                        dx.abs() < 1e-9,
                        "{fam} {view}: 沿线位移 dx={dx}（应 0；非 0 说明 offset 不是世界向量）"
                    );
                    assert!(
                        (dy.abs() - 3.175 * scale).abs() < 1e-9,
                        "{fam} {view}: 垂距 |dy|={} ≠ 3.175×{scale}（间距塌缩会被宿主画成实心）",
                        dy.abs()
                    );
                    let base = (ln.angle.to_degrees() - dir).rem_euclid(360.0);
                    assert!(
                        (base - 45.0).abs() < 1e-9 || (base - 135.0).abs() < 1e-9,
                        "{fam} {view}: 线角度 {:.3}° 与 pattern_angle {dir}° 不满足「45/135 + dir」的烘焙约定",
                        ln.angle.to_degrees()
                    );
                }
            }
        }
    }
}
