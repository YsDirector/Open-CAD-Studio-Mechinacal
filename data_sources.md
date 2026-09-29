# 数据出处清单 / Data provenance

> **口径（可作 README 附录）**
> **本插件的数值数据取自各国标准（GB / ISO / DIN / ANSI / NF / ASME）及公开机械设计手册；
> 每份内嵌数据文件都自带 `source` 字段或在表头写明出处；标准号随相应功能界面（零件族名、
> 智能卡片名、孔/螺纹选择项、手册正文）实时显示。标准文本本身不随插件分发。**
>
> *One-line policy: **All numeric data is taken from published standards (GB / ISO / DIN /
> ANSI / NF / ASME) and public mechanical-design handbooks. Every embedded data file carries a
> `source` field or a provenance header, and the standard number is shown in the corresponding
> UI (part-family names, smart-card titles, hole/thread pickers, handbook text). The standard
> documents themselves are not redistributed.***

范围说明：

* `crates/ocs_ocsm/assets/**` —— 渐开线花键/平键/齿根圆弧等**标准表原始资产**
  （CSV + 溯源笔记），构建时经 `include_str!` 编进 `.so`。
* `crates/ocs_ocsm/src/tables/*.json` —— **标准件/螺纹/公差表的入库数据**
  （运行期经 `serde_json::from_str(include_str!(...))` 读入），每份 JSON 顶层带 `source` 字段。
* `crates/ocs_ocsm/bom/` —— 明细表**图块**（DWG/DXF，非数值数据）。
* `crates/ocs_ocsm/assets/icons/*.svg` —— 自绘界面图标，**非数据**。

---

## 1. `crates/ocs_ocsm/assets/**` —— 标准表资产（20 个 CSV + 4 篇笔记）

### 1.1 GB / 国标

| 文件 | 出处（表头/`source` 声明的标准号与来源） | 载入位置 |
| --- | --- | --- |
| `keyway_gb1095.csv` | **GB/T 1095-2003** 平键键槽（轴槽 t1 / 毂槽 t2）26 档；数据源 = 三来源交叉校验的 `键槽_GB1095_表_v2.csv`；b=100 的 t2 取官方 19.5（主源印 19.4）；另含 1979 旧版 d 列 | `src/shaft.rs:613` |
| `key1096_shaft_ranges.csv` | **GB/T 1095** 表「轴的公称直径 d」区间 → 键宽 b（1979 旧版 d 列，2003 版已取消该列） | `src/partgen_keys.rs:846` |
| `spline_gb3478_ev_sv.csv` | **GB/T 3478.1-2008 表 23**：作用齿槽宽 Ev 下偏差 / 作用齿厚 Sv 上偏差（μm）；来源 = 嘉立创FA机械设计手册 5-3-52 + GB/T 3478.1 §8.7.2 | `src/spline_tol.rs:175` |
| `spline_gb3478_ext_dia_dev.csv` | **GB/T 3478.1-2008 表 24**：外花键小径 Die / 大径 Dee 上偏差（μm）；来源 = JLC 5-3-50 + 标准 §8.7.3 | `src/spline_tol.rs:176` |
| `spline_gb3478_limits.csv` | **GB/T 3478.1-2008 表 25**：内花键小径 Dii 极限偏差（H10/H11/H12）/ 外花键大径 Dee 公差（IT10/11/12） | `src/spline_tol.rs:177` |
| `spline_gb3478_fit.csv` | **GB/T 3478.1-2008** 齿侧配合类别（基孔制 H + 外花键 d/e/f/h/js/k）§8.7.2/表 23 | `src/spline_tol.rs:178` |
| `spline_gb3478_pin_series.csv` | **GB/T 3478.9-2008 表 1** 量棒尺寸 D_R 系列（67 档，±0.001 mm；即 GB/T 321 R40 优先数系）；来源 = 安特派官方预览页逐值人工看 + tesseract 交叉 | `src/spline_tol.rs:179` |
| `spline_gb3478_table26_rimin.csv` | **GB/T 3478.1-2008 表 26** 齿根圆弧最小曲率半径 R_imin / R_emin（mm）；来源 = doc88 扫描页逐格 OCR（三读 + 词框定位） | `src/spline_tol.rs:180` |

### 1.2 DIN

| 文件 | 出处 | 载入位置 |
| --- | --- | --- |
| `din5480_2_nominal.csv` | **DIN 5480-2:2015-03** 名义表（721 行入库；旧 355 行 + p29–p41 续跑 OCR）；双源冲突按 `d=m·z`、`e2` 公式机械裁决 | `src/invol_spline.rs:902` |
| `din5480_2_inspection.csv` | **DIN 5480-2:2015-03** 检验尺寸表（原 164 行 + m=1.5 截图 56 行 + m=5 截图 47 行，三源列集相同） | `src/invol_spline.rs:2805` |
| `din5480_1_table7_dev.csv` | **DIN 5480-1:2006-03 Tabelle 7** 上段：Abmaße für Zahnlücke Ae / Zahndicke As（μm），18 系列 × 9 直径档；德文原文 PDF 600 dpi + 英文译本双源 | `src/din_table.rs:119` |
| `din5480_1_table7_tol.csv` | **DIN 5480-1:2006-03 Tabelle 7** 下段：Maßtoleranzen（Tact/Teff），仅 6–9 级有实锚，其余留空（卡内显示「—」） | `src/din_table.rs:142` |
| `din5480_1_bild6_anchor.csv` | **DIN 5480-1:2006-03 Bild 6**（Datenfeld 示例 N120×3×38×9H / W120×3×38×8f）逐行数值；德文 p18 + 英文译本 p18 双源照录 | `src/din_table.rs:144` |
| `din5480_1_table7_notes.md` | 上表的 OCR 溯源笔记（素材目录、页号、复核方法） | 文档 |

### 1.3 ANSI

| 文件 | 出处 | 载入位置 |
| --- | --- | --- |
| `ansi_b921_formulas.csv` | **ANSI B92.1-1970 (R1993)** 径节系列 17 项（P / Ps=2P），入 Table 2 公式；来源 = p08/p09 正文 + p11 Table 3 三处互证 | `src/invol_spline.rs:350` |
| `ansi_b921_sample_check.csv` | **ANSI B92.1-1970 (R1993)** p11 Table 3 印出值 vs p10 Table 2 公式复算（抽样校验，π 取表内注 3.1415927） | `src/invol_spline.rs:7322`（单测） |
| `ansi_b921_notes.md` | 上表溯源笔记（含 Table 2/4/5 口径） | 文档 |

### 1.4 NF（法国）

| 文件 | 出处 | 载入位置 |
| --- | --- | --- |
| `nf_e22141_dims.csv` | **NF E22-141**（中文译本）尺寸表，p01–p20 OCR（p18 拉削内花键外径定心 / p20 尺寸表 m=0.50–1.25…） | `src/invol_spline.rs:1602` |
| `nf_e22141_check.csv` | **NF E22-141** 检查尺寸 / 公差值 / 配合表，p23–p27 第三轮 OCR | `src/invol_spline.rs:2446` |
| `nf_e22141_e_xm_tol.csv` | **NF E22-141** 中文译本 **p29**「检查尺寸的公差值」E（公法线长）/ xm（刀具切入量）偏差（μm） | `src/nf_table.rs:429` |
| `nf_internal_card_anchor.csv` | **NF E22-141** 内花键参数表锚点（模板示例 m=7.5 / A=300 / z=38）；仅用于单测交叉核对 | `src/nf_table.rs:2121`（单测） |
| `nf_external_card_anchor.csv` | **NF E22-141** 外花键参数表锚点（同上）；仅用于单测 | `src/nf_ext_table.rs:1539`（单测） |
| `nf_e22141_notes.md` | NF 全套溯源笔记（页清单、OCR 轮次、脚本、补抄记录） | 文档 |

### 1.5 图标（非数据）

`assets/icons/{frame,centerline,gear,shaft,powerdim,dimguide,dim2gb,parts,hole,card}.svg` —— 自绘双色扁平图标（24×24 网格、1.625 线宽）；`assets/icons/README.md` 说明用法。**与标准数据无关。**

---

## 2. `crates/ocs_ocsm/src/tables/*.json` —— 入库数据表（49 个 JSON）

每份 JSON 顶层 `source` 字段即上表口径的完整出处（部分字段很长，含交叉校验与取舍记录）。

### 2.1 公差 / 偏差

| 文件 | 标准号 / 出处 |
| --- | --- |
| `toleranceTable.json` | IT 等级表（**GB/T 1800.1 / ISO 286-1**，>3150 mm 无值）——**无 `source` 字段**（历史遗留，来自早期插件） |
| `correctionTable.json` | IT 值修正表 —— **无 `source` 字段**（历史遗留） |
| `holeDeviationTable.json` | 孔极限偏差 —— **无 `source` 字段**（历史遗留） |
| `shaftDeviationTable.json` | 轴极限偏差 —— **无 `source` 字段**（历史遗留） |
| `identifierCond.json` | 基本偏差代号适用性矩阵 —— **无 `source` 字段**（历史遗留） |

> 上述 5 个文件来自本插件之前的 OCS 早期实现，**建议补 `source` 字段（写明 GB/T 1800.1/1800.2）**
> 再公开；或在文档里统一说明「IT/偏差表取自 GB/T 1800 系列」。

### 2.2 孔 / 螺纹（`hole*`、`thread*`）

| 文件 | 标准号 / 出处（`source` 字段摘录） |
| --- | --- |
| `holeTapDrill.json` | 公制螺纹底孔牙深明细表（用户提供 PNG，粗牙+细牙） |
| `holeClearance.json` | 嘉立创FA机械设计手册（螺栓/螺钉通孔间隙） |
| `holeCounterbore.json` | **GB/T 152.3-1988**（易紧通官方结构；与 JLC 逐值交叉；M30 d3 按官方 36 修正） |
| `holeCountersink.json` | **GB/T 152.2-2014**（代替 1988 版；与 JLC 交叉校验） |
| `holeDrill.json` | **GB/T 6135.3-1996** 直柄麻花钻 d1 h8 列（嘉立创FA 手册 2.1.2.3） |
| `threadIso724.json` | **ISO 724** 公制螺纹系列（粗牙/细牙），2026-09 抓取参考站前端数据 |
| `threadG.json` | **ISO 228-1:2000**（GB/T 7307-2001）55° 非密封管螺纹 |
| `threadR.json` | **ISO 7-1:1994**（GB/T 7306.1/.2-2000）55° 密封管螺纹 |
| `threadNpt.json` | **ASME B1.20.1-2013**（GB/T 12716-2011）60° 密封管螺纹 |
| `threadUn.json` | **ANSI/ASME B1.1-2003** 统一英寸螺纹（牙型 ISO 68-2、直径/牙数 ISO 263、基本尺寸 GB/T 20666~20670-2006） |
| `threadTr.json` | **ISO 2901:2016**（GB/T 5796.1/.2/.3-2005）米制梯形螺纹 |
| `threadAcme.json` | **ASME B1.5-1997** 爱克母（29°）螺纹 |

### 2.3 标准件（`parts*`）

| 文件 | 族 id | 标准号 |
| --- | --- | --- |
| `partsHexBoltC.json` | `hex_bolt_c` | **GB/T 5780-2016** 六角头螺栓 C 级 |
| `partsHexBolt5782.json` | `hex_bolt_ab` | **GB/T 5782-2016** A/B 级（易紧通逐规格官方图；与 C 级画法复用） |
| `partsHexBoltBFull.json` | `hex_bolt_b_full` | **GB/T 5783-2016** 全螺纹 B 级 |
| `partsHexBoltHoleA.json` | `hex_bolt_hole_a` | **GB/T 32.1-2020** 头部带孔 A 级 |
| `partsSocketHead.json` | `socket_head` | **GB/T 70.1-2008** 内六角圆柱头螺钉 |
| `partsSocketButton702.json` | `socket_button_702` | **GB/T 70.2-2015** 内六角平圆头螺钉（用户 PNG 目录名 702 有误；r/w 补自手册表 6-1-116） |
| `partsSocketTorx2671.json` | `socket_torx_2671` | **GB/T 2671.1-2017** 内六角花形低圆柱头螺钉 |
| `partsSetScrew77.json` | `set_screw_77` | **GB/T 77-2007** 内六角平端紧定螺钉（易紧通 164580 info_26503） |
| `partsEyeBolt825.json` | `eye_bolt_825` | **GB/T 825-1988** 吊环螺钉 A 型（用户 PNG + 易紧通 info_13787 核对） |
| `partsNut6170.json` | `nut_6170` | **GB/T 6170-2015** 1 型六角螺母 |
| `partsNut61721.json` | `nut_61721` | **GB/T 6172.1-2016** 六角薄螺母 |
| `partsNutC41.json` | `nut_c41` | **GB/T 41-2016** 六角螺母 C 级（164580 + 华人螺丝网 + mechtool 三源） |
| `partsRoundNut812.json` | `round_nut_812` | **GB/T 812-1988** 圆螺母（易紧通 info_13770） |
| `partsWasher971.json` | `washer_971` | **GB/T 97.1-2002** 平垫圈 A 级 |
| `partsWasher93.json` | `washer_93` | **GB/T 93-2025** 标准型弹簧垫圈 |
| `partsLockWasher858s.json` / `partsLockWasher858l.json` | `lock_washer_858` | **GB/T 858-1988** 圆螺母用止动垫圈（易紧通 info_3349；s 列按用户参数表修正） |
| `partsRing893.json` | `ring_893` | **GB/T 893-2017** 孔用弹性挡圈 A 型 |
| `partsRing894.json` | `ring_894` | **GB/T 894-2017** 轴用弹性挡圈 A 型 |
| `partsPin1191.json` | `pin_1191` | **GB/T 119.1-2000** 圆柱销 A 型（164580 + ACM 老库互证） |
| `partsPin1201.json` | `pin_1201` | **GB/T 120.1-2000** 内螺纹圆柱销 |
| `partsBearing276.json` | `bearing_276` | **GB/T 276-2013** 深沟球轴承 60000 型（mechtool + 用户 PNG 逐行核对） |
| `partsBearing297.json` | `bearing_297` | **GB/T 297-1994** 圆锥滚子轴承 30000 型 02 系列（**仅 20 行不臆造**，E 列无公开来源） |
| `partsBearing288.json` | `bearing_288` | **GB/T 288-1994** 调心滚子轴承 20000C 型（mechtool + SKF 双源抽检） |
| `partsSealFb.json` | `seal_fb` | **GB/T 13871.1-2007** 内包骨架有副唇密封圈 FB 型 |
| `partsKey1096A/B/C.json` | `key_1096_a/b/c` | **GB/T 1096-2003** 普通平键 A/B/C 型 |
| `partsKey1097A/B.json` | `key_1097_a/b` | **GB/T 1097-2003** 导向平键 A/B 型 |
| `partsHexBolt.json` / `partsHexNut.json` | （早期族，已被 `hex_bolt_c` 等取代） | **无 `source` 字段**，建议公开前确认是否仍被引用 |

### 2.4 结构要素（`detail_*`，数据在 `src/detail.rs`）

| 族 id | 标准号 |
| --- | --- |
| `detail_grind_od` | **GB/T 6403.5-2008** 磨外圆砂轮越程槽 |
| `detail_thread_relief` | **GB/T 3-1997 表 2** 外螺纹退刀槽 |
| `detail_hub_keyway` | **GB/T 1095-2003 表 1**（复用 `assets/keyway_gb1095.csv`）+ 毂槽模板反解 |
| `detail_spline_rect` | **GB/T 1144-2001** 矩形花键（de 查 **GB/T 10952** 表 1/表 2） |

### 2.5 其它

| 文件 | 说明 |
| --- | --- |
| `ref_3fams_expected.rs` | 回归测试的期望几何（**非标准数据**，源自用户模板实测） |
| `crates/ocs_ocsm/bom/*.dwg,*.dxf` | 明细表图块源（**非标准数据**，模板几何） |

---

## 3. 标准号在界面上的呈现（可验证）

| 位置 | 证据 |
| --- | --- |
| 零件库树 | 分级为 `零件库 / 螺栓 / 六角螺栓 / 六角头螺栓 C级 GB/T 5780-2016`（`PLUGIN.md`「标准件（参数化）」节） |
| 智能卡片名 | `GB 花键参数表（内/外）`、`齿轮参数表（GB/T 10095）`、`ANSI 花键参数表`、`NF E22-141 内/外花键参数表`、`DIN 5480 内/外花键参数表`（`src/card.rs:164` 起 22 条 `CardTypeSpec`，卡名带标准号与方向） |
| 结构要素 | `GB/T 6403.5-2008`、`GB/T 3-1997`、`GB/T 1095-2003`、`GB/T 1144-2001`（`PLUGIN.md`「结构要素」节） |
| 手册 | `handbook/03-标准件库.md`、`16-齿轮.md`、`17-孔生成器.md`、`20–23` 知识篇逐处写标准号；英文镜像同 |
| 卡片计算书 | 「计算书」按钮输出 Markdown（卡片项 + 引擎式 → 代入 → 结果），含标准口径 |

---

## 4. 已知的数据缺口（如实标注，不臆造）

* **DIN 5480-1 Tabelle 7 下段**：只抽到 6–9 级、模数组 1,75–4 的实锚，其余在卡内显示「—」。
* **NF E22-141**：表外值显示「—」。
* **DIN Table 7 >400 侧**、部分公差表体缺口：显示「—」。
* **GB/T 297-1994（圆锥滚子轴承）**：E 列（外圈滚道小端直径）无公开可信来源 → 只保留 20 行。
* **GB/T 276-2013**：4 组 (d,B) 撞键的重复规格在 GUI 中不可选（数据保留）。
* **ACME / UN / NPT / 梯形螺纹**：直径与基本尺寸取自公开参考站前端数据，已与 mechtool / 易紧通逐行交叉一致，
  但**不是标准原文扫描件**；如需法务级溯源建议补标准原文页。
