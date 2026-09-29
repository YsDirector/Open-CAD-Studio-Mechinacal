# 第四批参数化标准件：并行实现约定（partgen_b1…b4）

> 本文件是第四批（11 个族）**并行开发**的统一约定。每个分支只写自己的模块文件，
> 互不干扰；公共设施已由主代理一次性搭好（`partgen_kit.rs` + 四个 `partgen_bN.rs` 桩 + 派发接线）。

## 一、模块与所有权（**硬约束**）

| 分支 | 文件 | 负责族 |
|---|---|---|
| A | `src/partgen_b1.rs` | `nut_6170`、`ring_893`、`ring_894` |
| B | `src/partgen_b2.rs` | `set_screw_77`、`round_nut_812`、`lock_washer_858` |
| C | `src/partgen_b3.rs` | `eye_bolt_825`、`seal_fb`、`bearing_276` |
| D | `src/partgen_b4.rs` | `bearing_297`、`bearing_288` |

**只允许**改：自己的 `src/partgen_bN.rs`、自己新建的 `src/tables/parts*.json`、你的 `/tmp` 临时文件。
**禁止**改：`partgen.rs`、`partgen_more.rs`、`partgen_kit.rs`、`lib.rs`、其它 `partgen_bN.rs`、别人的表 JSON、
`Cargo.toml`、handbook 文档。需要新的公共设施 → 在自己文件里写私有副本（本库既有风格就是如此）。

## 二、模块必须实现的三个 API（签名固定，已被接线）

```rust
pub fn family_views(family: &str) -> Vec<&'static str>;              // 只有你的族才返回非空
pub fn families_json() -> serde_json::Map<String, serde_json::Value>; // 键 = 族 id
pub fn generate(family: &str, d: f64, l: f64, view: &str) -> Option<Result<GenPart, String>>;
```

- `family_views` / `generate` 对**不属于你**的族必须返回空 / `None`（别抢别人的族）。
- `generate` 里先做视图校验（不在 `family_views` 里的视图 → `Some(Err(...))`），再生成。
- 数据表的 `OnceLock` + `Table::parse(include_str!(...))` 写法照 `partgen_kit.rs` 的注释抄。

### families_json 每族必填字段

```jsonc
{
  "id": "ring_893",
  "name": "孔用弹性挡圈 A型",           // 显示名（不含标准号，标准号走 code）
  "code": "GB/T 893-2017",             // 现行代号
  "iso": "—",                           // 有采标就写，没有写 "—"
  "implemented": true,
  "views": views_json("ring_893"),      // 用 partgen_kit::views_json（唯一数据源 = family_views）
  "sizes": [ { "d": 10, "label": "Ø10", "pitch": 0.0,
               "l_min": 1.0, "l_max": 1.0, "lengths": [1.0],
               "extra": "d3=... s=..." } ],
  "len_label": "厚度 s",                // 长度选择器标题（该族没有长度概念就写主尺寸名）
  "base_hint": "基点 = 端面 × 轴线",     // 基点约定，GUI 会显示
  "tree_dir": "零件库/挡圈"               // 树上目录（叶子名由代码拼：`族名 + 代号`）
}
```

### 上树（`tree_dir` / `tree_path`）——**别把代号里的 `/` 当分隔符**

- **推荐 `tree_dir`**：只写**目录**，`/` 分隔；叶子名由代码统一拼成 `族名 + 代号`
  （例：`"tree_dir": "零件库/螺钉/紧定螺钉"` → 叶子「内六角平端紧定螺钉 GB/T 77-2007」）。
- 或 `tree_path`（整条路径，含叶子名）。`/` 与 `>` 都行，代码会做**代号回接**
  （`GB/T`、`JB/T` 里的斜杠不会被切开），但目录段尽量别取成 `GB`/`ISO` 这种代号前缀。
- 目录可自建：`零件库/螺钉/…`、`零件库/挡圈/…`、`零件库/轴承/…`、`零件库/密封件/…`。
- **螺钉 ≠ 螺栓**（用户 2026-09-16 明确）：`hex_bolt*` 是螺栓（树「螺栓」支，`kind=bolt`）；
  `socket_head` / `set_screw_*` / `eye_bolt_*` 是螺钉（树「螺钉」支，`kind=screw`，不进件链装配）。
  螺钉族的 `tree_dir` 必须走「零件库/螺钉/…」，**不要**放「零件库/螺栓/…」。
- `sizes[0]`（最小规格）+ `lengths[0]` + **每个视图**都必须能成功出图——主代理的
  `catalog_all_implemented_families_usable` 测试会逐个跑（这是全局护栏，别让它挂）。

## 三、环境与命令

```bash
export PATH=$HOME/.cargo/bin:$PATH          # cargo 不在默认 PATH 里
cd /home/ysdirector/dev/OpenCADStudio
# 只跑自己模块的测试（别跑全库：并行分支共用一个 target 目录，也会抢测试端口）
cargo test -q -p ocs_ocsm --lib partgen_b1::            # ← 换成你的模块名
# 需要看编译错误时
cargo build -q -p ocs_ocsm
```
> 共用一个 target 目录 → 别的分支在编译时你会看到 `Blocking waiting for file lock on build directory`，
> 属正常，等一会儿即可，**不要**自己删 target 或改 CARGO_TARGET_DIR。

## 四、反解用户模板（画法的唯一权威）

模板目录：`~/桌面/GB/参数化/<族目录>/`（参数化图 = 某一个规格的实尺图 + 参数名标注）。

```bash
# 1) DXF → 精确数值 TSV（圆弧角度是**度**；同时输出同名 .svg）
/home/ysdirector/dev/OpenCADStudio/target/debug/examples/ref_dump \
  "/home/ysdirector/桌面/GB/参数化/<目录>/<文件>.dxf" /tmp/<你的目录>/x.tsv
#    若二进制不在：cargo build -p ocs_plugin_api --features host --example ref_dump
# 2) TSV 视图聚类/几何清单
python3 tools/ref_analyze.py /tmp/<你的目录>/x.tsv
# 3) 打包看图（肉眼核对，也便于和我沟通）
rsvg-convert -o /tmp/<你的目录>/x.png /tmp/<你的目录>/x.svg
```

要点：
- TSV 的 `MTEXT` 行 `text` 列 = **参数名**（`d D B r e s m T C E r1 r3 l …`）；
  这些参数名的位置/顺序是判断"哪一维是哪个参数"的关键证据。
- 模板是某个规格的实尺图：先把这个规格的参数值定下来（模板 MTEXT/标注里就有，或查在线表），
  再反推**公式**（例如 Δ=(e−s)/2·tan30° 这类）。
- 比对精度：坐标/半径/角度允许 ≤1e-3 差（模板值是四舍五入到 4 位的）。

## 四点五、ACM 老库（`~/桌面/GB/标准件库/`）的画法基准怎么解

老库 DWG 的几何藏在 `STDPART2D` **代理图形**里，`ref_dump` 直接读会只看到几个 `0` 层的线——
必须先拆视图：

```bash
# 1) DWG（或整个族目录）→ 每视图一个 DWG + catalog.csv
/home/ysdirector/dev/OpenCADStudio/target/debug/examples/split_views \
  "$HOME/桌面/GB/标准件库/内六角紧定螺钉 - 平端 - GB 77-85" /tmp/<你的目录>/acm
# 2) 再对拆出来的单视图 DXF/DWG 用 ref_dump
/home/ysdirector/dev/OpenCADStudio/target/debug/examples/ref_dump \
  "/tmp/<你的目录>/acm/内六角紧定螺钉 - 平端 - GB 77-85/内六角紧定螺钉_平端-GB77-85_M3_x_8_主视图.dwg" \
  /tmp/<你的目录>/acm_main.tsv
```

（实测：GB 77-85 / M3 x 8 拆出主视图 11 图元、俯视图 22 图元，可用来互证内六角孔与端部画法。）

## 五、数据（在线权威源）

易紧通 164580（本仓已有工具）：

```bash
python3 tools/yjt.py search "GB/T 77"          # → info id
python3 tools/yjt.py info   26503              # → sid / 规格 x 列表 / 尺寸网格 / 公称长度
python3 tools/yjt.py mine   26503 /tmp/x.json   # → 逐规格弹窗全量（dims 标签路径→值 + lengths）
```
- `dims` 的标签路径里空格 = 下标（`d k/最大值/光滑头部` 读作 d_k 最大值(光滑头部)）。
- **必须交叉核对**：拿模板那个规格的实测值去对在线表；对不上就查另一来源或标记存疑，**不要硬填**。
- 轴承（276/288/297）**不在易紧通**（那是紧固件站）：用 `mechtool.cn/bearing/…`（静态表格）、
  `zc91.cn`、`bearing.cn` 等，并**逐行核对用户 PNG 参数表**给出的行（见各组模块头文件）。
- 表 JSON 落 `src/tables/partsXXX.json`（自取名，避开别人的），第一行注释性的 `source` 字段写清
  来源 URL + 抓取方式 + 核对结论。

## 六、画法与出图要求（全库统一）

- 图层只用这五个：`1轮廓实线层`（粗）、`2细线层`（牙底/细线）、`3中心线层`、`4虚线层`、`5剖面线层`；
  一律 `partgen_kit::set_layer` → `ByLayer`。
- **不生成任何尺寸标注**（用户明确：调用零件时不显示尺寸）。
- 基点沿用全库约定（螺栓=头部支承面×轴线；螺母/挡圈/垫圈=端面×轴线；轴承=端面中心×轴线），
  在 `base_hint` 里写明。
- 只声明模板里真实存在的视图（模板没有的视图不要画）。
- 中心线出头、倒角等"非标准尺寸"的常量：从模板实测（模板量不到就沿用同族既有常量并在注释写明）。

## 七、验收（每条都要做）

1. `cargo test -q -p ocs_ocsm --lib partgen_bN::` **全绿**；测试内容至少覆盖：
   - 每个规格 × 每个视图都能生成（遍历 `sizes`）；
   - 每个规格第一个能出图的长度 = `sizes[0].lengths[0]`（全局护栏会用它）；
   - 图层越界/尺寸标注检查（可抄 `partgen_more.rs` 里 `nut_washer_layers_and_no_dimension`）；
   - **模板那个规格逐条数值回归**（坐标/半径/角度与 TSV 对齐）。
2. 加一个 `#[test] #[ignore] fn dump_bN_svg()`：把每个族 × 每个视图落成
   `/tmp/bN/<族>-<规格>-<视图>.svg`（用 `partgen_kit::dump_svg` / `part_svg`）。
3. `cargo test -q -p ocs_ocsm --lib partgen_bN:: -- --ignored dump_bN_svg` 后
   用 `rsvg-convert` 转 PNG 并**亲眼看图**，与模板 DXF 的 SVG 并排查差异。

## 八、汇报格式（做完给我）

```
族 id | 名称 | 规格数 | 数据来源(URL) | 模板比对结论 | 未实现/存疑项
```
外加一句：`cargo test -p ocs_ocsm --lib partgen_bN::` 的最后一行统计。
