# DIN 5480-2 入库说明（`din5480_2_nominal.csv` / `din5480_2_inspection.csv`）

- 版本：DIN 5480-2:2015-03（名义尺寸表 + 检验尺寸表）。
- 入库文件：
  - `crates/ocs_ocsm/assets/din5480_2_nominal.csv`（**721 行**；`invol_spline.rs` 用 `include_str!` 编译进插件，主路径查表用）。
  - `crates/ocs_ocsm/assets/din5480_2_inspection.csv`（**267 行**；检验尺寸数据，已接入 `invol_spline.rs` 的解析/查询/公式路径）。
- 源文件：
  - 名义表：`~/桌面/OCSM/review/花键标准资料/DIN5480-2_名义表_续2_merged.csv`（665 数据行，其中 p35/m=5 旧 47 行已剔除）+ `DIN5480-2_m15_名义表.csv`（用户截图 1，56 行）+ `DIN5480-2_m5_名义表.csv`（用户截图 1，47 行）。
  - 检验表：`DIN5480-2_检验表_merged.csv`（p12/p16/p18/p20，164 行）+ `DIN5480-2_m15_检验表.csv`（用户截图 2，56 行）+ `DIN5480-2_m5_检验表.csv`（用户截图 2，47 行）。
- 列：
  - 名义表：`page,m,table_no,d_B,z,d,d_b,x1_m,e2_s1,d_f2,A_df2,d_Ff2min,d_a2,d_a1,d_Ff1max,d_f1,A_df1,flags,source`
  - 检验表：`page,m,table_no,d_B,z,D_M_1,M2_between,A_M2,D_M_2,M1_over,A_M1,k,W_k,flags,source`
  - `d_B` = 基准直径；`d_b` = 基圆直径 = `d·cos30°`；`x1_m` = 变位量 `x₁·m`；`k` = 跨测齿数；`W_k` = 公法线长度。
- `flags` / `source` 原样保留（OCR 质量标记与来源），供追溯，运行时不展示。

## 存量与覆盖

| 处理 | 行数 | 说明 |
|---|---|---|
| 续 2 合并名义源表 | 665 | 旧 355 行（p11..p27）+ 续 2 310 行（p29..p41） |
| 剔除 p35（m=5）旧读数 | −47 | 源图数据区渲染缺陷、双源 OCR 逐格互不一致；整页剔除，不臆造 |
| m=1.5 补入（名义表） | +56 | 用户截图 1（page=1 / table_no=14，截图语义）OCR |
| m=5 补入（名义表） | +47 | 用户截图 1（page=35 / table_no=26，表 26 截图语义）OCR |
| **名义表入库** | **721** | **16 个模数档**：0.5 / 0.6 / 0.75 / 0.8 / 1 / 1.25 / **1.5** / 1.75 / 2 / 2.5 / 3 / 4 / **5** / 6 / 8 / 10 |
| 检验表合并 | 164 + 56 + 47 = **267** | 6 个模数档：p12→0.5（35）、p16→0.75（42）、p18→0.8（45）、p20→1（42）、m=1.5（56）、m=5（47） |

## m=5 并入（2026-09-21）

- 来源：用户截图 OCR（`DIN5480-2_m5_用户截图_1.png` / `_2.png`，脚本 `ocr_m5_screenshots.py`；`source=user_screenshot_m5_1/2`）。
  用户截图 1 = 名义表 **表 26**（公称尺寸，47 行）、用户截图 2 = 检验表 **表 27**（检验尺寸，47 行）。
  `page` / `table_no` 是截图替代页语义（35/26、36/27），不是确定的 doc88 PDF 页号：
  **检验表页码 36 系推断**（截图题头没有页码；与名义表 p35 相邻页、同 m=5 系列推得），
  追溯时以 `source=user_screenshot_m5_2` 为准。
- **修正 2 处 `z`（名义表）**：r17 的 `z` OCR 读成 `147`（与脚注粘连）、r22 读成 `160`；
  由 `d = m·z`（70/5=14、80/5=16）与 flags `row:vote_suspect:z:expected` 反推为 **14 / 16**。
- **修正 10 行 `d_Ff1_max`/`d_f1` 粘连（名义表）**：OCR 把两格读成一格（如 `109,90109,00`），
  recheck 按表内规律拆为 `NN,90` / `NN,00`（109,90/109,00 … 199,89/199,00）；原始粘连串保留在 flags
  （`word_spans_columns(...)`、`value_from_recheck`）。
- **修正 17 处 `k`（检验表）**：OCR 把个位与脚注/邻格粘连（`22→2`、`35→3`、`471→4`、`511→5`、
  `611→6`、`75→7`、`97→9`、`113→11` 等），按 flags `row:vote_suspect:k:expected=N` 取反推的个位数；
  17 处已用 `W_k` 公式逐行独立复核一致。另有 4 行的 `k` 在源 CSV 已是修正值（`value_from_recheck`）。
- **口径**：按既有「原始值 + 代码修正表」惯例，**入库 CSV 不改写原始 OCR 值**：
  - 名义表 2 处 z → `invol_spline.rs` 的 `DIN5480_M5_NOMINAL_Z_FIXES`（解析 `din5480_rows()` 时生效）；
  - 检验表 17 处 k → `DIN5480_M5_INSPECTION_K_FIXES`（解析 `inspection_rows()` 时生效）。
  原始读数与反推证据都留在各行 `flags`，`assets/*.csv` 保持 OCR 原样。

## m=1.5 并入（2026-09-21）

- 来源：用户截图 OCR（`DIN5480-2_m15_用户截图_1.png` / `_2.png`，脚本 `ocr_m15_screenshots.py`；`source=user_screenshot_1/2`）。
  `page` / `table_no` 是截图语义（1/14、2/15），不是 doc88 PDF 页号；m=1.5 已从「已知缺失档位」移出。
- **修正 1 处 `d_b`（名义表 r15，d_B=26、z=16）**：OCR 原读 `29,785` 与 `d·cos30° = 20.7846` 不符，
  按该行 flags `row:vote_suspect:d_b:expected=20.7846` 取反推值 `20,7846`；原读（`cell:d_b:recheck(L=29,785|S=29,785)`）
  与证据仍完整保留在该行 flags。
- **修正 15 处粘连 `k`（检验表）**：OCR 把两位 k 读成一格（21/23/311/38/33/41/48/53/78/91/106/115/121…），
  按各行 flags `row:vote_suspect:k:expected=N` 取反推值；15 处已用 W_k 公式逐行独立复核一致：
  z=8→2、z=9→2、z=10→3、z=11→3、z=14→3、z=15→3、z=16→4、z=21→4、z=26→5、
  z=38→7、z=40→7、z=50→9、z=57→10、z=64→11、z=68→12。
  原读保留在 flags（`cell:k:multiword(...)`），`k` 值已改。
- 检验表两源列集相同（15 列），按名义表口径 `page,m,table_no,...` 合并，无缺列可补；`flags` 中仅原表正常的空标记。

## 4 条恒等式（新增，全表校验）

对名义表全部 721 行（含 m=1.5 56 行、m=5 47 行）逐行校验，容差 5e-3（表值最细 2 位小数 → 半个末位）：

| 恒等式 | m=1.5 | m=5 | 全表 | 违例 |
|---|---|---|---|---|
| `d_f2 = d_B` | 56/56 | 47/47 | 721/721 | **0** |
| `d_a1 = d_B − 0.2m` | 56/56 | 47/47 | 721/721 | **0** |
| `d_a2 = d − 0.9m + 2x₁m` | 56/56 | 47/47 | 721/721 | **0** |
| `d_f1 = d − 1.1m + 2x₁m` | 56/56 | 47/47 | 721/721 | **0** |

回归测试 `din5480_four_identities_hold_for_every_row` 锁死「违例数 = 0」；
m=5 另有 `din5480_m5_lookup_and_eight_identities` 锁 8 条恒等式（再加 `d=mz`、`d_b=d·cos30°`、
`e₂=mπ/2+2x₁m·tan30°`、`d_B=d+1.1m+2x₁m`）。r15/r17/r22 的修正后同样通过（修正值见代码修正表）。

## 档位现状

- **无缺失档位**：m=1.5（截图 1 表 14）与 m=5（截图 1 表 26）均已由用户截图补入；
  `lookup_by_d_b` / `lookup_inspection` 对 m=5 正常命中，不再有「该档位数据缺失」分支。
- 页脚非数据带 p33#45 / p35#48 / p37#54 在合并时已剔除。

## 双源一致性（旧 618 行）

- 除 p35 外，A续/A续2（逐列连通域）与 B续/B续2（网格条带）逐格交叉验证；
  冲突按 `d=m·z` / `d_b≈d·cos30°` 排版值 / `e2` 公式 / `row_fitness` 机械裁决，
  每处裁决记录在 `flags`（`xchk:*`、`conflict:*`）。各页读到的 `d_B`、`z`、`x1_m` 已复核。
- `d_B` 非严格递增（同一 `d_B` 有多个 `z/x₁` 变体行）在 p31/p33/p35/p37/p39/p41 出现，
  属表格结构不是 OCR 错；这正是 `lookup_by_d_b` 可能返回多行的原因。

## 反推关系（`x_from_d_b` 的口径）

`d_B = d + 1.1·m + 2·x₁·m`，等价 `z_B = z + 1.1 + 2·x₁`，即

```text
x₁ = (d_B − m(z + 1.1)) / (2m)
```

- 这是**从 OCR 表反推并经全表校验**的关系：721 行逐行
  `|x₁·m − (d_B − m(z+1.1))/2| = 0`（浮点残差 ≤ 5.2e-15），0 违例。
- **不是标准原文公式**；引用时以 DIN 5480-2 表值与检验表为准。代码注释与手册同此口径。
- 回归测试 `din5480_table_shape_and_formula_holds_for_every_row` 逐行锁死该式；
  `din5480_m15_lookup_hits_and_x_from_d_b_matches_table` 与
  `din5480_m5_lookup_and_eight_identities` 另抽样锁定 m=1.5 / m=5 查表命中与该式一致。

## 查询接口

- `lookup_by_d_b(d_b, Option<m>) -> Result<Vec<Din5480Row>, String>`：
  命中可能多行；未命中列附近候选；m=1.5 / m=5 均可正常命中。
- `resolve_din_by_d_b(d_b, m?, z?, x?)`：`DB+M` 补 `Z`、`DB+Z` 补 `M`、
  `DB+M+Z` 公式解 `x` 并与表值互相印证；结果带来源（查表行 / 公式）。

## 检验尺寸表接入（2026-09-21）

- 解析：`inspection_rows()` / `inspection_modules()`（`InspectionRow` 保留 `flags`/`source`）；
  列语义按原表：CSV 第一组 `D_M_1/M2_between/A_M2` 是**内花键 M2**（棒间距），
  第二组 `D_M_2/M1_over/A_M1` 是**外花键 M1**（跨棒距）——原表列头两组
  `D_M | M | A*` 对应 p08 图 4。
- 查询：`lookup_inspection(d_b, m?)`（多行/附近候选/未收录档）、`inspection_for(d_b,m,z)`、
  `inspection_for_d_b_z(d_b,z)`（m 由名义表反查）、`inspection_query(d_b,m,z)`
  （查表优先，表外 z 走公式）、`inspection_json` → `GET /api/invol_check?db=..&m=..&z=..[&check=1]`。
  所有错误信息带页/表号/source。
- 公式（DIN 5480-2 p09 式 (1)~(11)，α=30°）：
  `W_k = m·cosα[(k−0.5)π + z·invα] + 2·x·m·sinα`；
  外花键 `invδ = invα + s/d − π/z + D_M/d_b`、内花键 `invδ = invα + s/d − D_M/d_b`（e=s），
  `r_M = d_b/(2cosδ)`；偶数齿 `M1 = 2r_M + D_M` / `M2 = 2r_M − D_M`，
  奇数齿再乘 `cos(π/(2z))`。
- 逐行对照（`inspection_formula_report()`，容差 2e-3）：267 行中 265 行可算
  （z=93/97 两条 OCR 残行无名义 x）；W_k 264/265、M1 256/265、M2 258/265 直接通过；
  **17 处旧源表 OCR 异常**（p16 9 处 `D_M_2` 印 1.65/1.85 应为 1.55；p18 6 处 `D_M_1` 同；
  p20 z=8 一处 `M2` 印 5.983 应为 5.583）与 1 处 `k`（p18 z=35 印 63 应为 6）
  已逐张核对源图（p16/p18/p20 截图），修正后 17/17 ≤2e-3 → `validated=true`。
  **入库 CSV 未改写**，修正值存于 `invol_spline.rs` 的 `INSPECTION_OCR_FIXES` 常量；
  m=5 的 17 处粘连 k 同走代码修正表 `DIN5480_M5_INSPECTION_K_FIXES`（修正后 m=5 47/47 通过）。
- 表外自定义 z：`x` 由 `x_from_d_b` 反解（超 [−0.05,0.45] → 明确报“请提供对应表页”）、
  `k` 由“接触圆离齿顶 ≥0.07m”规则反推（表内 264 行 264/264 命中）、
  `D_M` 借用同 m 最邻近表行；结果带“公式导出/邻近行”来源说明。
- 露面：`OCSMGEAR` 花键模式的 CHECK 把 `inspection_summary`（M1/M2/D_M/k/W_k + 来源）
  附到 spec/模型；默认不开、既有行为不变。
  （XL `detail_invol_spline` 与轴段 `INVOLSPLINE` 均已于 2026-09-22 移除；齿轮 GUI 花键模式
  按目录顶层 `spline_engine.din_inspection` 表显示检验值。）
