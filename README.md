# Open CAD Studio Mechanical (OCSM) · OCSMechanical 机械工具包

> 本 README 分两段：**[中文](#中文)** 在前，**[English](#english)** 在后。
> This README is in two halves: Chinese first, English second.

| 项 / Item | 值 / Value |
| --- | --- |
| 插件 id / plugin id | `opencad.ocsm` |
| 插件名 / plugin name | `OCSMechanical 机械工具包` |
| 插件版本 / plugin version | `0.2.0`（`crates/ocs_ocsm/plugin.toml`） |
| 宿主插件 API / host plugin API | **v5** |
| 许可 / license | **GPL-3.0-only**（见 `LICENSE`、`crates/ocs_ocsm/Cargo.toml`） |
| 目标宿主 / target host | [OpenCADStudio](https://github.com/HakanSeven12/OpenCADStudio) v2026.39+ |

> ⚠️ **非官方声明 / Unofficial notice**
> 本仓是 **OCSM 插件的发布子集**（插件源码 ＋ 宿主增量补丁 ＋ 文档），**不是 OpenCADStudio 官方项目**，
> 与上游 OpenCADStudio 官方**无隶属、无背书关系**；宿主增量只含插件所需的最小改动。
> 补丁**唯一基线 = 上游 `1bbdce31`（v2026.39）**。本仓**不能单独编译**：须先 clone 上游 → 切到该基线 →
> `git apply -3 docs/host-increment.patch` → 再把本仓的插件 crate 叠进上游树（见「构建与安装」）。
>
> This repo is the **OCSM plugin release subset** (plugin source ＋ host-increment patch ＋ docs). It is
> **not an official OpenCADStudio project** and is neither affiliated with nor endorsed by upstream.
> The patch's **only baseline is upstream `1bbdce31` (v2026.39)** and this repo **cannot be built on its own**
> (see "Build and install").

---

## 中文

### 一句话定位

**OCSM 是 OpenCADStudio（OCS）的机械制图插件**：把中国国标（GB）机械制图的那套流程 ——
图层/图框、标准件、结构要素、齿轮/轴/孔/花键、尺寸与公差标注、焊接与序号、明细表 ——
做成 OCS 里可点、可命令行、可脚本驱动的命令，而不是靠手画。

它不是 OCS 的 fork，也不改 OCS 的核心功能：它通过宿主提供的 **插件 ABI（API v5）**
以 C-ABI 动态库（`libocs_ocsm.so`）加载进 OCS，随附一份**为插件打通宿主的最小宿主侧补丁**
（`crates/ocs_plugin_api/**` + `src/app/plugin_host.rs` 等，见 `docs/fork-patches.md`）。

### 主要功能（实况）

功能清单与命令总表以仓库内 `crates/ocs_ocsm/handbook/00-总览.md` 为准（中文 23 篇手册），
下表只是索引：

| 方向 | 命令 / 形态 | 要点 |
| --- | --- | --- |
| 初始化 | `OCSM` | 幂等建 **10 个机械图层**、线型、文字样式 `OCSM_GB`、标注样式 `OCSM_GB`（新建空图纸自动执行） |
| 快速图层 | `1`…`9`、`10` | 无选中切当前层；有选中把对象移到该层 |
| 图幅/图框 | `TF` / `OCSMFRAMEINIT` | 扫描插件目录 `frame/*.dwg` 弹选择窗 + 比例（必含 1）；按比例建 `OCSM_GB_x{n}` 标注样式 |
| 智能标注 | `D` / `OCSMPOWERDIM` | 点选/线选两模式，自动推断线性/对齐/角度/半径/直径/弧长；固定落 `7标注层` |
| 引导线标注 | `GDIM` / `OCSMDIMGULIDE` | 浏览器 GUI：11 种标注类型 + **焊接符号**（GB/T 324）+ **引线** + **序号球标** + 形位公差 |
| 标注再编辑 | `ME` / `OCSMEDIT` | Ctrl+点击标注或选中后 `ME` → 同窗口改参数、重建（替换语义） |
| 表面粗糙度 | `CC` / `OCSMRGH` | 20 种形态（4 基础体 × 5 附加区），文字转 ATTDEF |
| 一键转国标 | `D2G` / `OCSMDIM2GB` | 原生标注 → `OCSM_GB` 样式 + 匿名块；智能圆心标记一并换成中心线 |
| 中心线 | `ZX` / `OCSMCENTERLINE` | 圆/弧 → 十字；两直线 → 角平分线 |
| **齿轮生成器** | `OCSMGEAR` | 外齿轮 / 内齿轮齿圈（剖视/侧视/简化正视/常规正视）+ **渐开线花键模式**（GB/T 3478.1、DIN 5480、ANSI B92.1、NF E22-141） |
| **轴生成器** | `OCSMSHAFT` | 段表 ↔ 行文本双向同步：S/E/L、倒角 `CH`、越程槽 `OV`、螺纹段 `M`（含 `TL/RO/SD/RL`）、段级退刀槽 `RL@L/@R`、齿轮段 `GEAR`、矩形花键段 `SPLINE`、轴槽 `KEY`（GB/T 1095/1096/1097） |
| **孔生成器** | `DK` / `OCSMHOLE` | 简单/螺纹/沉头/埋头；盲孔/贯通；自动螺纹长 1.5d、自动孔深 +2P |
| **标准件库** | `XL` / `OCSMPART` | **24 族**（螺栓/螺钉/螺母/垫圈/挡圈/销/轴承/密封圈）+ **4 种结构要素**；参数化生成，不读图库；两段式放置（基点 → 旋转 → 落定，可连续） |
| 螺栓副装配 | `OCSMJOINT` | 板/垫圈/螺母件链、自动算长度、遮挡裁剪、一次撤销 |
| 明细表 BOM | `BOM` / `BOMSYNC` / `BOMLOCK` / `BOMXLSX` / `BOMXLSXI` / `BOMCFG` | 建表/联动重排/锁数量/xlsx 导入导出/表头配置（自家 xlsx 读写，不引外部工具） |
| **智能卡片** | `OCSMCARD` | **22 张卡**：GB 花键（内/外）、齿轮（GB/T 10095）、ANSI（内/外 × 中/英）、NF E22-141（内/外）、DIN 5480（内/外）＋ 11 张「精简版」；卡面自带计算书（Markdown，可摘要落图） |
| 手册 | `OCSMHELP` / `OH` | 在 OCS 内打开本仓库的 `handbook/` Markdown |
| 自动化 | `OCSMMCP` + `ocs_ocsm_mcp` | 插件内置 HTTP 服务（默认端口 23751）；`crates/ocs_ocsm_mcp` 是标准 MCP stdio 服务器，桥接回插件操作活文档 |

### 许可与继承（重要）

* 本插件以 **GPL-3.0-only** 发布（`LICENSE` = FSF GPL-3.0 全文，674 行）。
* 本插件**运行于 OpenCADStudio 之上**，而 OpenCADStudio 本身即以 **GPL-3.0** 发布
  （本仓库根 `LICENSE` 与上游 [HakanSeven12/OpenCADStudio](https://github.com/HakanSeven12/OpenCADStudio) 一致）。
* 因此：**本插件与其宿主侧增量补丁整体以 GPL-3.0 发布**，并如实继承上游的许可与版权声明；
  上游版权归 OpenCADStudio 作者及其贡献者，本插件的原创部分版权归本插件作者。
* 插件同时**依赖** `crates/ocs_plugin_api`（同仓 `path` 依赖，GPL-3.0）与
  经由 `Cargo.lock` 固定的第三方 crate（`serde`/`serde_json`/`flate2`/`crc32fast`/`quick-xml` 等，
  各自原许可）。分发二进制时请一并保留 `LICENSE` 与依赖的许可声明。

### 构建与安装

两套产物，**必须分清**：

| 改了什么 | 要重建什么 |
| --- | --- |
| `crates/ocs_ocsm/**`（插件本体，含 `src/*.html` GUI、`src/tables/*.json`、`assets/**`） | **插件 `.so`**（`cargo build --release -p ocs_ocsm`） |
| `crates/ocs_plugin_api/**`（插件 ABI / runner / 代理） | **宿主二进制**（`cargo build --release`）—— runner 是宿主自身 |
| `src/**`（宿主的渲染/几何/插件落地点） | **宿主二进制**（`cargo build --release`） |

```bash
# 0. 宿主源码：clone 上游 → 切到补丁基线 → 打宿主增量补丁 → 叠入本仓插件
#    （本仓只有插件 + 宿主增量，缺宿主主体，单独 clone 本仓编不出宿主也编不出插件）
git clone https://github.com/HakanSeven12/OpenCADStudio.git && cd OpenCADStudio
git checkout 1bbdce31                                        # 本补丁唯一基线 = 上游 v2026.39
git apply -3 /path/to/this-repo/docs/host-increment.patch    # git apply --check 应为 rc=0
cp -r /path/to/this-repo/crates/ocs_ocsm     crates/         # 插件本体（补丁不含它）
cp -r /path/to/this-repo/crates/ocs_ocsm_mcp crates/         # MCP 服务器（workspace 成员，别漏）
cp -r /path/to/this-repo/tools .                             # deploy_plugin.sh 等

# 1. 宿主（API v5；插件 runner 就是宿主自身，必须重编）
cargo check            # 仅校验：本批次已在此基线上验证通过
cargo build --release

# 2. 插件 + 安装：编 .so → 填 plugin.toml 占位符 → 装**全运行期数据**（handbook 含 en/、bom 模板块）
#    → 跑**部署后自检**（缺必需数据当场非零退出），一次搞定
tools/deploy_plugin.sh

# 3. 图框 DWG（**用户自备，不在仓库里**；样例在维护机的 ~/桌面/OCSM/frame/）
#    给出 OCSM_FRAME_SRC 就把图框一起装（同名不覆盖已装图框）；不给则保留现有 frame/。
#    自检只在 frame/ 为空时 ⚠ 警告（rc 仍 0，不拦新用户）——要当硬门禁设 OCSM_REQUIRE_FRAME=1。
OCSM_FRAME_SRC=<放图框的目录> tools/deploy_plugin.sh --skip-build

# 脚本自己做的事（★ bom/ **不用再手工拷贝**）：
#   * cargo build --release -p ocs_ocsm
#   * 把 target/release/libocs_ocsm.so 拷进插件目录
#   * 把 plugin.toml 的 __RUSTC_VERSION__  ← rustc --version 原串
#                __ACADRUST_SOURCE__ ← Cargo.lock 里 codec 依赖（opencadcodec）的 source 串
#   * 递归拷 crates/ocs_ocsm/handbook/*.md（**含 en/ 英译篇**）到插件目录的 handbook/
#   * 拷 crates/ocs_ocsm/bom/（OCSM_BOMHEAD.dwg / OCSM_BOMROW.dwg / 明细表模板.dwg / README.md /
#     settings.json；跳过 *.bak；settings.json 已存在则保留 ⇒ 不回滚 OCSM_BOMCFG 改过的运行期配置）
#   * 部署后自检：.so / plugin.toml 占位符与 acadrust_source 形状 / handbook 中英篇数 /
#     bom 两个模板块且无 *.bak ⇒ 缺哪项当场非零退出
```

**补丁基线（必读）**

* `docs/host-increment.patch`（39 文件 / +5889 −45）从二开 fork 相对上游 **`1bbdce31`（v2026.39）** 导出，
  **只在“该 commit 的干净上游树”上验证过**：`git apply --check`、`git apply -3` 均 rc=0；叠入插件 crate 后
  宿主 `cargo check` 与 `cargo check -p ocs_ocsm -p ocs_ocsm_mcp` 也均 rc=0。上游之后的提交会让补丁冲突
  （`-3` 可能仍能合并，但不再保证）。补丁不含 `crates/ocs_ocsm/**`
  ——插件本体在本仓，需按上面第 0 步叠入。
* 本仓与 `Cargo.lock` **只用官方 URL**（仓库里没有镜像 `[patch]`）；如需中国大陆网络加速，请在本机 git 配置
  `url.https://ghfast.top/https://github.com/.insteadof = https://github.com/`
  （配合 `.cargo/config.toml` 的 `[net] git-fetch-with-cli = true`）—— 镜像**属本机设置，不经由本仓**。

**`plugin.toml` 的占位符机制（踩过坑，务必看）**

`crates/ocs_ocsm/plugin.toml` 里有两个**部署时替换**的占位符，**不要手改**：

* `__RUSTC_VERSION__` —— Rust 无稳定 ABI，宿主 v2026.36+ 对 API ≥ 4 的插件按编译器门禁；
* `__ACADRUST_SOURCE__` —— 宿主 API ≥ 4 的 acadrust 源指纹门禁（上游 v2026.38 起强制）。
  值 = `Cargo.lock` 里 codec 依赖（`opencadcodec`，旧名 `acadrust`）那条的 `source`；脚本两个名字都认。

漏填任何一个，插件会被宿主**静默拒绝加载**，命令行只报一句
`Plugin built for acadrust @unknown, but this host uses @…`。
`tools/deploy_plugin.sh` 会读 `Cargo.lock` + `rustc --version` 自动填好并自检；
升级 rustc 或上游升 acadrust rev 后，**重跑一次脚本即可**。

**安装位置与图框**：默认装到 `$HOME/.config/OpenCADStudio/plugins/opencad.ocsm/`
（可用 `OCSM_PLUGIN_DIR` 覆盖）。**图框 DWG 是用户自备数据，不在本仓**（仓库 `crates/ocs_ocsm/`
下没有 `frame/`）：把图框放进该目录的 `frame/` 子目录（也可用环境变量 `OCSM_FRAME_DIR` 覆盖），
或部署时给 `tools/deploy_plugin.sh` 传 `OCSM_FRAME_SRC=<放图框的目录>`（只装 `*.dwg`，同名不覆盖）。
`frame/` 为空时部署自检只 **⚠ 警告 + 仍 `rc 0`**（不拦新用户），要当硬门禁就显式设 `OCSM_REQUIRE_FRAME=1`。
启动后命令行应出现：

```
Loaded plugin: OCSMechanical 机械工具包 (opencad.ocsm 0.2.0)
```

### 文档指引

* **中文手册**：`crates/ocs_ocsm/handbook/` —— **23 篇**
  （`00-总览.md` 是命令总表 + 任务索引，先读它；`01`–`17` 是功能；`20`–`24` 是知识/双语规范）。
* **English handbook**：`crates/ocs_ocsm/handbook/en/` —— **23 篇**（`README.md` 给出 zh→en 对照表）。
  英文是中文的镜像，**中文原件为准**（不一致时改译文，不改原件）。
* 插件级说明：`crates/ocs_ocsm/PLUGIN.md`；宿主侧补丁台账：`docs/fork-patches.md`；
  插件架构：`docs/plugin-architecture.md`；第四批标准件台账：`crates/ocs_ocsm/BATCH4.md`。
* 数据出处：见本仓根目录 `data_sources.md`。
* **发布前卫生扫描（维护机）**：`python3 tools/hygiene_scan.py --history`（工作树 + 全历史）。
  词表在**仓库之外**（`~/.config/ocsm/hygiene-words.txt`，每行一个词），脚本里不写任何私有词，
  命中只报 `私#N`；**非 0 命中 ⇒ 禁止推送**。通用件守卫（`tools/frame_clean.py`、`tools/bom_template.py`）
  同样从仓库外读私有词（`~/.config/ocsm/junk-names.txt` 或 `OCSM_JUNK_NAMES`），仓里只留通用软件名。

### 已知限制

* 图框 DWG 含嵌套块时不支持（报错提示）。
* `Zhuque Fangsong` 字体不随插件分发（系统未装则渲染回退）。
* 宿主 STYLE 写出器原先会在保存时丢掉 `true_type_font`；本仓的宿主增量已在保存/加载两侧补齐（`annotative` 仍由插件在 `TF`/`D` 前对齐）。
* POWERDIM 放置一个标注后命令结束（不做连续标注）。
* 宿主侧补丁刻意保持最小、追加式：插件 API 的扩展方法都带默认实现，旧插件继续可加载。

---

## English

### One-liner

**OCSM is the mechanical-drafting plugin for OpenCADStudio (OCS).** It turns the Chinese
national-standard (GB) mechanical drafting workflow — layers/sheets, standard parts,
structural details, gears/shafts/holes/splines, dimensioning and tolerances, welding and
item numbers, bills of materials — into commands you can click, type, or script, instead of
drawing by hand.

It is **not** a fork of OCS and does not change OCS's core behaviour: it loads into OCS as a
C-ABI dynamic library (`libocs_ocsm.so`) through the host's plugin ABI (**API v5**), together
with the **minimal host-side increment** that the plugin needs
(`crates/ocs_plugin_api/**`, `src/app/plugin_host.rs`, … — inventory in `docs/fork-patches.md`).

### Features (as built)

The authoritative command list is `crates/ocs_ocsm/handbook/00-总览.md` (Chinese, 23 pages;
English mirror in `handbook/en/`). Summary:

| Area | Command | Notes |
| --- | --- | --- |
| Init | `OCSM` | Idempotent: 10 mechanical layers, linetypes, `OCSM_GB` text style and dimension style (auto-runs on a new blank drawing) |
| Quick layers | `1`…`9`, `10` | Switch current layer; with a selection, move it to that layer |
| Sheet & frame | `TF` / `OCSMFRAMEINIT` | Picker over `frame/*.dwg` + scale `a:b` (one side must be 1); creates `OCSM_GB_x{n}` dim styles |
| Smart dimension | `D` / `OCSMPOWERDIM` | Point-pick / line-pick modes; linear, aligned, angular, radius, diameter, arc-length |
| Leader annotation | `GDIM` / `OCSMDIMGULIDE` | Browser GUI: 11 dimension types + welding symbols (GB/T 324) + leaders + item-number balloons + GDT |
| Re-edit dimension | `ME` / `OCSMEDIT` | Ctrl+click a dimension (or select + `ME`) → same GUI, regenerate (replace semantics) |
| Surface roughness | `CC` / `OCSMRGH` | 20 forms (4 base bodies × 5 add-on zones); text becomes ATTDEF |
| One-click GB | `D2G` / `OCSMDIM2GB` | Native dimensions → `OCSM_GB` style + anonymous blocks; center marks become centerlines |
| Centerlines | `ZX` / `OCSMCENTERLINE` | Circle/arc → cross; two lines → angle bisector |
| **Gear generator** | `OCSMGEAR` | External gears / internal ring gears (4 views) + **involute-spline mode** (GB/T 3478.1, DIN 5480, ANSI B92.1, NF E22-141) |
| **Shaft generator** | `OCSMSHAFT` | Segment table ↔ text DSL: `S/E/L`, chamfers, grinding relief grooves, thread segments (`TL/RO/SD/RL`), gear segments, rectangle-spline segments, keyway segments (GB/T 1095/1096/1097) |
| **Hole generator** | `DK` / `OCSMHOLE` | Simple / threaded / counterbore / countersink; blind or through |
| **Standard parts** | `XL` / `OCSMPART` | **24 families** (bolts, screws, nuts, washers, retaining rings, pins, bearings, seals) + **4 structural details**; parametrically generated, no part library files; two-stage placement with rotation |
| Bolt joints | `OCSMJOINT` | Plate/washer/nut chain, auto length, occlusion clipping, single undo entry |
| BOM | `BOM`, `BOMSYNC`, `BOMLOCK`, `BOMXLSX`, `BOMXLSXI`, `BOMCFG` | Create / reorder by balloons / lock quantities / xlsx import+export / header config (own xlsx reader-writer) |
| **Smart cards** | `OCSMCARD` | **22 cards**: GB splines (int/ext), gear (GB/T 10095), ANSI (int/ext × CN/EN), NF E22-141 (int/ext), DIN 5480 (int/ext) + 11 "lite" cards; each card produces a calculation sheet |
| Handbook | `OCSMHELP` / `OH` | Opens this repo's `handbook/` Markdown inside OCS |
| Automation | `OCSMMCP` + `ocs_ocsm_mcp` | Plugin HTTP service (default port 23751); `crates/ocs_ocsm_mcp` is a standard MCP stdio server bridging back to the plugin |

### License and inheritance (important)

* This plugin is released under **GPL-3.0-only** (`LICENSE` = the verbatim FSF GPL-3.0 text).
* This plugin **runs on top of OpenCADStudio**, which is itself released under **GPL-3.0**
  (this repository's root `LICENSE` matches upstream
  [HakanSeven12/OpenCADStudio](https://github.com/HakanSeven12/OpenCADStudio)).
* Therefore: **the plugin together with its host-side increment is distributed as a whole
  under GPL-3.0**, inheriting upstream's license and copyright notices. Upstream copyright
  belongs to the OpenCADStudio authors and contributors; the plugin's own code is
  copyright its authors.
* The plugin also depends on `crates/ocs_plugin_api` (in-tree `path` dependency, GPL-3.0) and on
  third-party crates pinned in `Cargo.lock` (`serde`, `serde_json`, `flate2`, `crc32fast`,
  `quick-xml`, …) under their own licenses. When redistributing binaries, keep `LICENSE`
  and the dependency notices.

### Build and install

Two separate artefacts — do not mix them up:

| You changed | Rebuild |
| --- | --- |
| `crates/ocs_ocsm/**` (plugin body, incl. `src/*.html` GUI, `src/tables/*.json`, `assets/**`) | **plugin `.so`** (`cargo build --release -p ocs_ocsm`) |
| `crates/ocs_plugin_api/**` (plugin ABI / runner / proxies) | **host binary** (`cargo build --release`) — the runner *is* the host |
| `src/**` (host rendering / geometry / plugin host wiring) | **host binary** (`cargo build --release`) |

```bash
# 0. Host source: clone upstream → check out the patch baseline → apply the host increment,
#    then overlay this repo's plugin crates (this repo alone cannot build anything: it has no host body)
git clone https://github.com/HakanSeven12/OpenCADStudio.git && cd OpenCADStudio
git checkout 1bbdce31                                        # the patch's only baseline = upstream v2026.39
git apply -3 /path/to/this-repo/docs/host-increment.patch    # git apply --check must be rc=0
cp -r /path/to/this-repo/crates/ocs_ocsm     crates/         # plugin body (the patch does not carry it)
cp -r /path/to/this-repo/crates/ocs_ocsm_mcp crates/         # MCP server (a workspace member — don't skip it)
cp -r /path/to/this-repo/tools .                             # deploy_plugin.sh and friends

# 1. Host (API v5; the plugin runner is the host binary itself)
cargo check            # verification only — passes on this baseline (see below)
cargo build --release

# 2. Plugin + install: build .so, fill plugin.toml placeholders, install the **full runtime data**
#    set (handbook incl. en/, BOM block templates) and run the **post-deploy self-check**
#    (any missing required data exits non-zero) — all in one go
tools/deploy_plugin.sh

# 3. Frame DWGs (★ user-supplied data, NOT in this repo; samples live in the maintainer's ~/桌面/OCSM/frame/)
#    Pass OCSM_FRAME_SRC to install frames in the same run (an already-installed file is never
#    overwritten); without it the existing frame/ is left alone. The self-check only warns (rc stays 0)
#    when frame/ is empty, so first-time users are not blocked — set OCSM_REQUIRE_FRAME=1 for a hard gate.
OCSM_FRAME_SRC=<dir with frames> tools/deploy_plugin.sh --skip-build

# What the script does for you (★ bom/ no longer needs a manual copy):
#   * cargo build --release -p ocs_ocsm
#   * copy target/release/libocs_ocsm.so into the plugin directory
#   * fill plugin.toml's __RUSTC_VERSION__ ← the literal `rustc --version` string
#                    __ACADRUST_SOURCE__ ← the `source` of the codec dependency (opencadcodec) in Cargo.lock
#   * recursively copy crates/ocs_ocsm/handbook/*.md (incl. en/) into the plugin's handbook/
#   * copy crates/ocs_ocsm/bom/ (OCSM_BOMHEAD.dwg / OCSM_BOMROW.dwg / 明细表模板.dwg / README.md /
#     settings.json; *.bak skipped; an existing settings.json is kept, so the runtime config written
#     by OCSM_BOMCFG is never rolled back)
#   * post-deploy self-check: .so / plugin.toml placeholders and acadrust_source shape / handbook
#     zh+en page counts / both BOM block DWGs present and no *.bak ⇒ any miss exits non-zero
```

**Patch baseline (a must-read)**

* `docs/host-increment.patch` (39 files / +5889 −45) was exported from the downstream fork against
  upstream **`1bbdce31` (v2026.39)** and is **verified only on a clean tree of that commit**:
  `git apply --check` and `git apply -3` both succeed (rc=0), and with the plugin crates overlaid the
  host `cargo check` and `cargo check -p ocs_ocsm -p ocs_ocsm_mcp` also pass (rc=0). Later upstream
  commits make the patch conflict (`-3` may still merge, but is no longer guaranteed). The patch does
  **not** contain `crates/ocs_ocsm/**` — the plugin body is this repo, so overlay it as in step 0.
* This repo and its `Cargo.lock` use **official URLs only** (no mirror `[patch]` ships here); for a
  China-mainland accelerator configure git locally
  (`url.https://ghfast.top/https://github.com/.insteadof = https://github.com/`, together with
  `.cargo/config.toml`'s `[net] git-fetch-with-cli = true`) — mirroring is a **local setting and never
  ships in this repo**.

**`plugin.toml` placeholders (a real trap):** `crates/ocs_ocsm/plugin.toml` carries two
**deploy-time** placeholders — `__RUSTC_VERSION__` and `__ACADRUST_SOURCE__` — because the
host (v2026.36+ for rustc, upstream v2026.38+ for the codec fingerprint) gates API ≥ 4 plugins on the exact
toolchain and on the codec source fingerprint from `Cargo.lock`. Miss either one and the
plugin is **silently refused** with only
`Plugin built for acadrust @unknown, but this host uses @…` on the command line.
`tools/deploy_plugin.sh` fills both, copies the handbook, and self-checks. Re-run it after a
rustc upgrade or an upstream acadrust rev bump.

Installs to `$HOME/.config/OpenCADStudio/plugins/opencad.ocsm/` (override with `OCSM_PLUGIN_DIR`).
**Frame DWGs are user-supplied data and are not in this repo** (there is no `frame/` under
`crates/ocs_ocsm/`): drop them into that plugin directory's `frame/` subfolder (override with
`OCSM_FRAME_DIR`), or pass `OCSM_FRAME_SRC=<dir with frames>` to `tools/deploy_plugin.sh`
(`*.dwg` only, an already-installed file is never overwritten). An empty `frame/` is only a
**⚠ warning with `rc 0`** after deploy (first-time users are not blocked); set
`OCSM_REQUIRE_FRAME=1` to turn it into a hard gate.
Expected log line after start-up:

```
Loaded plugin: OCSMechanical 机械工具包 (opencad.ocsm 0.2.0)
```

### Documentation

* **Chinese handbook**: `crates/ocs_ocsm/handbook/` — **23 pages** (`00-总览.md` is the command
  index; start there).
* **English handbook**: `crates/ocs_ocsm/handbook/en/` — **23 pages** (zh→en mapping table in
  its `README.md`). The English is a mirror; **the Chinese original is the source of truth**.
* Plugin notes: `crates/ocs_ocsm/PLUGIN.md`; host-side patch inventory: `docs/fork-patches.md`;
  plugin architecture: `docs/plugin-architecture.md`; fourth-batch standard parts ledger:
  `crates/ocs_ocsm/BATCH4.md`.
* Data provenance: `data_sources.md` (repository root).
* **Pre-publish hygiene scan (maintainer machine)**: `python3 tools/hygiene_scan.py --history`
  (work tree + full history). The word list lives **outside the repo**
  (`~/.config/ocsm/hygiene-words.txt`, one word per line); no private word is hard-coded in any
  script and hits are reported as `私#N` only — **any hit means: do not push**. The generic-frame
  guards in `tools/frame_clean.py` / `tools/bom_template.py` read their private terms the same way
  (`~/.config/ocsm/junk-names.txt` or `OCSM_JUNK_NAMES`); only generic software names live in the repo.

### Known limitations

* Frame DWGs containing nested blocks are not supported (explicit error).
* The `Zhuque Fangsong` font is not bundled (falls back if not installed).
* The host STYLE writer used to drop `true_type_font` on save; this repo's host increment now persists it across save/load (`annotative` is still aligned by the plugin before `TF`/`D`).
* POWERDIM ends after placing one dimension (no chained dimensioning).
* Host-side patches are deliberately minimal and append-only: every API extension has a
  default implementation, so older plugins keep loading.
