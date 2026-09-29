# ANSI B92.1-1970 (R1993) 入库说明（`ansi_b921_formulas.csv` / `ansi_b921_sample_check.csv`）

## 数据落点（为什么只有两个 CSV）

ANSI B92.1 是**公式驱动**体系（Table 2 给公式，不逐齿数印尺寸大表），所以：

- `assets/ansi_b921_formulas.csv`：**径节系列 17 项**（P/Ps/pitch），运行期由
  `invol_spline.rs::ansi_pitches()` 解析（`include_str!` 编进插件）。
- `assets/ansi_b921_sample_check.csv`：**抽样校验 73 行**（17×Ps + 17×p + 14×Sv30 + 14×Sv37.5 +
  11×Sv45），印出值取自 p11 Table 3，复算式即 Table 2 的 `Ps`/`p`/`Sv min` 三行；单元测试
  `ansi_sample_check_csv_all_73_rows` 逐行复算（容差 ≤6e-5）。
- **Table 2 的五列公式系数不另出入库 CSV**：它们是代数系数（`(N+1.35)/P` 之类）而非表值，
  放在 `invol_spline.rs` 的 `ANSI_PRESETS` / `AnsiColumn` 方法里更不易抄错，来源与逐格对应
  见本文件 §3（数据.md §3 为 OCR 逐格原文）。

- 源文件：`~/桌面/OCSM/review/花键标准资料/ANSI-B92.1_数据.md`（195 行，含 Table 2 逐格公式与
  自洽性检查）、`ANSI-B92.1_抽样校验.csv`（73 数据行）、`ANSI-B92.1_notes.md`（页号对照/存疑清单）。
- 源图：`doc88_74687961597535_p*.png`（38 张，中英对照**重排版**；本地 tesseract 多票 OCR）。
- 页号：径节系列 p08 末–p09 顶；Table 1 符号 p09；Table 2 公式 p10；Table 3 数值 p11；rf 说明 p14。

## 覆盖

| 内容 | 出处 | 入库形式 |
|---|---|---|
| 径节 17 项（2.5/5 … 128/256） | p08/p09/p11 三处互证 | `ansi_b921_formulas.csv` |
| Table 1 符号（Do/Dre/Dri/Di/DFe/DFi/cF/rf/Sv/φD…） | p09 | 命名口径进代码注释（本文件 §2） |
| Table 2 五列基本尺寸公式（14 行） | p10 | `invol_spline.rs` `ANSI_PRESETS` + `AnsiColumn` |
| Table 3 的 Ps/p/Sv min 数值 | p11 | `ansi_b921_sample_check.csv`（73 行，测试复算） |
| 公差（Table 4/5、检验量柱等） | p13/p18/p20–p22/p30–p32 | **未收**（本次范围外；见遗留） |
| rf（齿根圆角数值） | p14 明确「不能用一个给定半径规定」 | 无标准值；引擎按切于齿根的过渡弧构造（§4） |

## 符号口径（易踩坑，来自 p09 Table 1）

- `Do` = 外花键**齿顶**大径；`Dre` = 外花键**齿根**小径；`Dri` = 内花键**齿根**（较大）；
  `Di` = 内花键**齿顶**（较小）。即 `Dri > Di`、`Do > Dre`，且装配自洽性要求 `Di > Dre`、`Dri > Do`。
- `Sv min` = **最小有效齿槽宽**（p11 表头写 "(BASIC) Sv min"；任务/引擎口径按 basic 有效齿槽宽用）。
- `cF` = form clearance（径向齿形裕度）；`DFe/DFi` = 外/内花键 form diameter（渐开线起始圆）。
- `φD` = 标准压力角；`P` = 径节（分子，入公式）；`Ps` = 2P（stub pitch，只用于标识/显示）。

## Table 2 五列（p10；A–E 与 `ANSI_PRESETS` 一一对应）

| 列 | 压力角/齿根型式/配合 | 适用径节（表头原文） | 引擎 profile / 代号 |
|---|---|---|---|
| A | 30° 平齿根 齿侧配合 | 2.5/5 – 32/64 | `ANSI30平齿根齿侧` / `ANSI30P` |
| B | 30° 平齿根 **外径配合** | 3/6 – 16/32 | `ANSI30平齿根外径` / `ANSI30PM` |
| C | 30° 圆齿根 齿侧配合 | 2.5/5 – 48/96 | `ANSI30圆齿根齿侧` / `ANSI30R` |
| D | 37.5° 圆齿根 齿侧配合 | 2.5/5 – 48/96 | `ANSI37.5圆齿根齿侧` / `ANSI375R` |
| E | 45° 圆齿根 齿侧配合 | 10/20 – 128/256 | `ANSI45圆齿根齿侧` / `ANSI45R` |

五列公式（引擎实现，`m = 1/P`，N = 齿数）：

| 量 | A | B | C | D | E |
|---|---|---|---|---|---|
| Ps | 2P | 2P | 2P | 2P | 2P |
| D | N/P | N/P | N/P | N/P | N/P |
| Db | D·cosφD | 同左 | 同左 | 同左 | 同左 |
| p | π/P | π/P | π/P | π/P | π/P |
| Sv min | π/(2P) | π/(2P) | π/(2P) | (0.5π+0.1)/P | (0.5π+0.2)/P |
| Dri | (N+1.35)/P | (N+1)/P | (N+1.8)/P | (N+1.6)/P | (N+1.4)/P |
| Do | (N+1)/P | 同左 | 同左 | 同左 | 同左 |
| Di | (N−1)/P | (N−1)/P | (N−1)/P | (N−0.8)/P | (N−0.6)/P |
| DFi | (N+1)/P+2cF | **(N+0.8)/P−0.004+2cF** | (N+1)/P+2cF | (N+1)/P+2cF | (N+1)/P+2cF |
| DFe | (N−1)/P−2cF | (N−1)/P−2cF | (N−1)/P−2cF | (N−0.8)/P−2cF | (N−0.6)/P−2cF |
| cF | `0.001D`，max 0.010，min 0.002（跨列合并格，按全列通用处理） | ←同左 | ←同左 | ←同左 | ←同左 |

`Dre`（外花键小径）按径节**三段**（空 = 该列该段无值）：

| 条件 | A | B | C | D | E |
|---|---|---|---|---|---|
| 2.5/5 – 12/24 | (N−1.35)/P | (N−1.35)/P | (N−1.8)/P | (N−1.3)/P | 空 |
| 16/32 及更细 | (N−1.35)/P | (N−1.35)/P | **(N−2)/P** | (N−1.3)/P | 空 |
| 10/20 及更细 | (N−1.35)/P | (N−1.35)/P | 空 | (N−1.3)/P | **(N−1)/P** |

引擎实现：`df()` 于 `std=ANSI` 时按 `P>12`（16/32 及更细）与 `P≤12` 分段；45° 列只用第三段
`(N−1)/P`（其径节范围 10/20 起）。`B` 列 `DFi` 的 `−0.004` 是**英寸常量**（重排版字级 TSV 高置信；
语义即外径配合时把内花键 form diameter 压到配合直径以下）——本引擎按原式字面实现，但输入边界
统一换算成 mm（`−0.004 in = −0.1016 mm`，见 §5）。

**范围强制（用户定案 A）**：`AnsiColumn::pitch_range()` / `ansi_column_pitch_check()` 按上表每列
适用径节（上下界**含端点**）校验，越界报错点名「列 + 范围原文 + 收到的 P/Ps」——由
`InvolParams::validate` 统一把关，CLI/DSL/查询串/两个 GUI 同一文案；原只查 45° 下限的特例已并入。
**与 Table 3 的区别**：Table 3 把 `Ps`/`p`/`Sv min` 对 17 项径节**全印满**，`Sv min` 公式适用面
比 Table 2 单列更宽 —— `ansi_sv_min_formula(α, P)` 直接从 (α, P) 算，**不受列界约束**
（`ansi_sample_check_csv_all_73_rows` 用它复算 17 项印出值）。

## rf（齿根圆角）—— 无标准数值依据

- p14「Fillets and Chamfers」原文明确：圆齿根 `The curvature along any generated fillet varies and
  can not be specified by a radius of any given value.`；平齿根 `Specification of this fillet is usually
  not required. It is controlled by the form diameter…`。全 38 页只在 p06/p14/p20 命中文字描述，
  **没有 `rf = k/P` 之类的公式或数值表**。
- 引擎处置：`InvolParams::rho_f()` 在 `std=ANSI` 时改走 `ansi_fillet_radius()` —— 构造**与齿轮同类的
  「切于齿根圆的过渡弧」**：圆心 `C` 在半径 `rf+ρ` 上，圆与齿根圆（`Dre/2`）相切，且与渐开线在
  **form diameter `DFe/2` 处相切**（圆心在渐开线法线上）。闭式解
  `ρ = (rs²−rf²)/(2(rf+rs·sinα_s))`（`rs=DFe/2`、`rf=Dre/2`、`α_s=acos(db/2/rs)`）。
  **该 ρ 仅为本引擎的过渡弧构造，无 ANSI 数值依据**（代码注释同步注明）。
- 端视图暂不画该圆角弧（与 GB/DIN/NF 的既有口径一致：`ρf` 只导出数值；视图用径向直线把
  form diameter 接到齿根圆），待有原版 Fig. 6a–6d/刀具标准再补。

## 已知存疑（不改数，测试容差覆盖）

1. **重排版疑印刷错误：p11 Table 3 在 `128/256` 行的圆周齿距印 `0.0246`，应为 `0.0245`**
   （π/128 = 0.0245437，4 位小数 = 0.0245；字形核验末位确为 `6`）。残差 +5.63e-5 是 73 行最大值；
   本入库不改写印出值，测试取容差 6e-5 使该行通过并在断言里点名。
2. `Dre` 第三段条件词已复核 = `10/20 pitch and finer`（高置信：位图逐字母 + 与第二段 IoU=0.833）；
   先前 OCR 的 `coarser` 读数已排除。**存疑不存**。
3. `DFi` 的 `−0.004` 字级 TSV 高置信；语义存疑（英寸常量突兀）但与该列「外径配合」语义相符，按原文实现。
4. `cF` 合并格墨迹只覆盖 C/D/E 列下方、A 列空；但 A 列公式同样含 `2cF`，按「全列通用」处理。
5. 本页组是**重排版**（非原版扫描）：`30 deg` 表头横跨 A–C、`cF` 合并格位置偏移等；原版
   Table 2 在第 2162 页（正文引用）。拿不到原版前，以上 1/3/4 均可能受排版影响。
6. p11 本页只有 `Ps/p/Sv min` 数值，**没有按齿数的基本尺寸大表**：`D/Db/Do/Dri/Di/Dre/DFi/DFe/cF`
   只能靠 Table 2 公式与自洽性检查（数据.md §4），无法用本页组印出值逐项复核。
7. 实际 38 张（p01–p38），非任务书说的 39 张；无缺页迹象。

## 单位口径（重要）

ANSI B92.1 是**英制标准**：P 为**每英寸齿数**，标准公式里 D = N/P 单位为**英寸**，
`cF` 的 0.002/0.010、DFi(B) 的 −0.004 都是英寸常量。

本引擎与 GB/DIN/NF 一致，**内部与输出一律 mm**：入口按 1 in = 25.4 mm 换算——
`m = 25.4/P`，因而 `D/Do/Dri/Di/Dre/DFe/DFi/p/Sv min` 全是 mm；常量同步换代
（`0.004 in = 0.1016 mm`；`cF = clamp(0.001·D_mm, 0.0508, 0.254)`，`0.001·D` 本身与单位无关）。
块名/spec/JSON/界面**保留径节原值 P（及 Ps=2P）标识**（照齿轮 DP 先例：块名写 `P16_PS32`、
JSON `pitch=16`，同时显示换算后的节圆 mm），不再使用"数值口径无单位"的旧说法。

## 引擎接口（`crates/ocs_ocsm/src/invol_spline.rs`）

- `SplineStd::ANSI` 走独立分支：**第 5 参数 `m` 槽位收径节 `P`**（不是模数），引擎内 `m=25.4/P`（mm）；
  `Ps=2P` 只用于 `spec`/块名/显示。CLI 写法 `P8`/`pitch=8`，`M8` 槽位在 ANSI 下也按 P 解释。
- `InvolParams::ansi(profile, p, z)`；`ANSI_PRESETS` 5 条（A–E）+ `ansi_profile_name(α, 平/圆, 齿侧/外径)`；
  代号 `ANSI30P/ANSI30PM/ANSI30R/ANSI375R/ANSI45R`；裸 `ANSI`/`ANSI B92.1` 取默认 A。
- `ansi_pitches()` 径节系列；`ansi_sv_min()`、`ansi_form_dia_external()/internal()`、`ansi_fillet_radius()`。
- `resolve_spline(ANSI, …)`：要求 `P`（第 5 槽）与 `z`；给 `d_B`/`A` 报 `ANSI_D_B_MSG`；
  ANSI 无变位（x≠0 报错）。端视/侧视/剖视/内花键/CHECK 均复用既有共用通路（没有另开视图代码）；
  内花键与内齿轮同口径只出 剖视图 + 端视图，无侧视图（侧视图/剖视图由齿轮侧 `GearView` 过滤）。

## 遗留

- **公差体系未收**：Table 4（p13 最大公差）、Table 5（p18 规格指南）、量柱检验（p30–p32）、
  配合/偏差（p15–p17/p19）——需另立解析与查询，本次只做基本尺寸公式。
- **原版核对**：若拿到原版 B92.1 扫描页，优先核 §已知存疑 1/3/4 与 `Dre` 第三段条件词；
  原版 Table 6（按齿数大表）可把直径公式也做数值复核。
- **rf 绘图**：当前只导出数值 + 视图用径向直线接 form diameter；真实过渡弧等你提供模板/原版齿根图。
- **英制显示切换**：内部/输出固定 mm；暂未做"按英寸显示"的 UI 切换（如需再按 DP 先例加显示层）。
- **新旧单位口径变更**：ANSI 已从"数值=英寸"改为"内部 mm、标识保留 P"；
  旧数据/图档里按英寸口径产出的 ANSI 尺寸现在会被当作 mm（比原值大 25.4 倍），需重建。
