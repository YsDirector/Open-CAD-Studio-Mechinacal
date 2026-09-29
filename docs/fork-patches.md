# 本地（fork）补丁台账 · Local patch inventory

> 本仓库是 OpenCADStudio 的**二开 fork**：主线功能 + `crates/ocs_ocsm`（OCSMechanical
> 机械工具包插件）+ 为插件打通宿主所需的**本地补丁**。本文件登记所有与上游不同的
> 宿主侧改动，供**每次同步上游后照单核对**，避免补丁被合掉却无人发现。
>
> 上游：`origin` = HakanSeven12/OpenCADStudio（经 ghfast 镜像）
> 最近同步点：`fd0f5dc2`（Merge tag **`v2026.38`** = 上游 `0d023d26`，**148 提交**，执行记录见 §0.12）
> 台账基准：`git diff v2026.38..HEAD -- src/ crates/ocs_plugin_api Cargo.toml tests/`
> （**46 文件 / +11093 −103**，不含插件 crate；含 `crates/ocs_ocsm*` 则为 186 文件 / +114306 −107）
> ⚠️ 上一个同步点 `5200ef5d`（Merge 上游 `65c0fe54`，213 提交，见 §0.11）→ 本次之前上游又走了 148 提交。
>
> **2026-09-21 状态刷新**：
> * `§0.6/§0.7/§0.9/§0.10` 的分支仍在 `prf`、上游 PR 仍未开（同 §0.11 记录）。
> * ⚠️ **上游首次动了插件面**（`crates/ocs_plugin_api` 的 `process.rs`/`process/v4.rs`，纯重构去重）
>   → 「插件 API 上游永远不碰」这个假设作废，详见 §0.12。
>
> **2026-09-17 状态刷新（保留备查）**：
> * §0.6 / §0.7 / §0.9 的分支**都已推到 `prf` 远端，但上游 PR 未开** ——
>   `gh pr list --author YsDirector -R HakanSeven12/OpenCADStudio --state all` 只有已合并的
>   **#1234 / #1235**（2026-09-13T18:14:50Z）。要发 PR 直接用各节的 compare 链接
>   （`桌面/OCSM/PR-*-链接.txt` 里已经是拼好的 create-PR URL）。
>   ⚠️ **开 PR 前先把那四个分支 rebase 到 `origin/main`**（它们的基点是几个月前的上游，
>   直接用旧 diff 开 PR 会冲突）。
> * 上游那次 213 提交**完全没有碰 `crates/ocs_plugin_api/**` 与 `src/app/plugin_host.rs`**
>   （已用 `git diff fc1788df..origin/main --name-only` 核对）→ 插件 API v5/v6 天然安全。
> * 别人的上游 PR **#1306**（*docs(plugin): fix inaccuracies and flag real gaps found building a plugin*）
>   还开着，值得看一眼它点名的插件坑（可能是我们也会踩的）。

## 0. 怎么用（每次同步上游后）

```bash
git fetch origin && git merge <upstream-tag>          # 解决冲突
# ① 核对补丁是否都在（与本文件 §2 的表一致，缺文件=补丁丢了）
git diff --stat $(git rev-parse <merge>^2)..HEAD -- src/ crates/ocs_plugin_api Cargo.toml tests/
# ② 两套产物都要重编！（插件 .so ≠ 宿主二进制）
cargo build --release                                  # 宿主（也是插件 runner）
cargo build --release -p ocs_ocsm && cp target/release/libocs_ocsm.so \
    ~/.config/OpenCADStudio/plugins/opencad.ocsm/       # 插件（含 plugin.toml 的 rustc 门禁）
# ③ 回归
cargo test -p ocs_ocsm                                 # 插件 380（2026-09-17 起；9-14 时为 178）
cargo test -p OpenCADStudio --lib dimtmove              # 宿主 DIMTMOVE 引线 4
OCS_SMOKE_PLUGIN=$PWD/target/release/libocs_ocsm.so \
  OCS_PLUGIN_RUNNER_EXE=$PWD/target/release/OpenCADStudio \
  cargo test -p OpenCADStudio --lib installed_plugin_registers_its_ribbon
# ④ 重启 OCS 复测
```

⚠️ **用户启动入口**：`~/桌面/OpenCADStudio.desktop` → `Exec=<repo>/target/release/OpenCADStudio`。
所以**改到宿主 `src/` 的渲染/几何逻辑必须重编宿主**（只换 `.so` 会看到旧渲染，
2026-09-13 曾因此误判"改了没效果"）。

## 0.5 已上游化（2026-09-14 记 — **已在同日的上游同步中执行完毕**）

> ✅ **执行结果**：`git merge origin/main`（上游 `fc1788df`，即 **v2026.37.0**）完成，
> 合并提交 `d4007055`（父：fork `ba6fab35` + 上游 `fc1788df`）。9 个冲突按本表处置：
> `selection.rs` / `dim_leader_render_check.rs` 取上游；`viewport.rs` / `app/mod.rs` /
> `manifest.rs` 融合（fork 钩子与上游新结构两边都留）；`dimension.rs` 取上游 DIMTMOVE +
> **手工补回 fork 的 C-2 文字宽度自适应**（上游没有）；`Cargo.toml`/`Cargo.lock` 保住镜像 URL 与
> workspace 成员。另修一处自动合并产生的重复再导出（`src/scene/mod.rs` 同时出现上游
> `pub(crate) use selection::pe_url_of;` 与 fork 旧 `pub use`，后者会编译报错，已删）。
> 验证：`cargo check --lib` ✅ · `--lib dimtmove` **5 passed** · `--test dim_leader_render_check` **1 passed** ·
> `-p ocs_ocsm` **178 passed**（插件 API 未被上游改动破坏）。安全网分支/tag 仍保留指向 `ba6fab35`。

上游 owner HakanSeven12 于 **2026-09-13T18:14:50Z** 一次性合并了本 fork 的两个 PR：

| PR | 标题 | 合并提交 | 上游实现说明 |
|---|---|---|---|
| **#1234** | `feat(viewport): Ctrl+click opens an entity PE_URL hyperlink` | `84836b0f` | 直接收录本 fork 的提交 `171fc489`（哈希一致） |
| **#1235** | `fix(dimension): extend the DIMTMOVE=1 leader under the text for linear dims` | `64e64c80` | **改造后收录**：合并信息写 *"with kernel-based dimension leaders"*，上游把该修复接进了他们新的 kernel 渲染器 |

**同步动作（重要）**
1. 冲突时**一律取上游版**（`--theirs` 语义），不要保留本地副本——否则同一功能会有两份实现。
2. 删除本地对应补丁条目：本文档 **C-1**（`cbaf6d61f` → `src/entities/dimension.rs`、
   `tests/dim_leader_render_check.rs`）与 **E-1**（`381c328e`/`6ba3da8e` → `src/scene/selection.rs`、
   `src/scene/mod.rs`、`src/app/update/viewport.rs`）。本地这两处改动**尚未进过台账表格**，
   同步后无需再登记。
3. 注意 **DIMTMOVE 的本地版可能与上游 kernel 版语义不同**（上游重构了引线绘制）→ 同步后必须
   重跑 `cargo test -p OpenCADStudio --lib dimtmove` 与 `tests/dim_leader_render_check.rs`，
   并实测 `DIMTMOVE=1` 的显示（fork 的 OCSM 标注也依赖这条路径）。
4. `crates/ocs_ocsm/src/guide_server.rs` 里对 DIMTMOVE 的处理（`cbaf6d61f` 顺带改动）**属于插件侧**，
   不受上游合并影响，保留。
5. 收尾：合并后可删除分支 `feat/ctrl-click-hyperlink`、`fix/dimtmove-leader-under-text`
   （本地 `/tmp/ocs-pr` 工作区 + fork 远端）。

> 结论：fork 的**上游接受路线**已成功（对照 `obs-2026-08-22-pr-fork` 的双保险策略，
> 无需启用"fork 单向吸收"备选方案）。

## 0.6 待上游合并（2026-09-15 开 PR）

> **修复**：MCP `ocs_read op:"commands"` 的清单响应把 `detail_parameters.name` /
> `search_parameter.search` 硬编码成示例值 `"LINE"`（任何 search 都回显 LINE，客户端无法判断
> 实际生效的过滤条件）。现改为回显真正应用的 `search`（未过滤时为 `null`），
> `detail_parameters.name` 固定 `null`（带 `name` 的请求在上方提前返回 `command` 清单）。
>
> | 项 | 值 |
> |---|---|
> | 文件 | `src/app/control/mod.rs`（+27 −2，含 1 个回归测试） |
> | fork 提交 | `c0f42f4e`（与 PR 提交 **逐字节相同的 diff/消息/作者**） |
> | PR 分支 | `fix/mcp-command-listing-filters`（`34175d9a`，基点 = 上游 `d738ebfc`） |
> | 验证 | 上游基点 `cargo test -p OpenCADStudio --lib control::` → **9 passed**；fork 同套 **9 passed** |
> | 报告 | `桌面/OCSM/PR-MCP命令清单回显-{正文.md,链接.txt}` |
>
> **同步动作**：上游合并后**直接取上游版**（两边提交内容一致，git 会自动归并，无需手工补回）；
> 然后删掉本节、删除分支 `fix/mcp-command-listing-filters`（fork 远端 + 本地 worktree `/tmp/ocs-pr-upstream`）。
> 该改动**不属于 fork 独有补丁**，故不进 §2 总表。

## 0.7 待上游合并 · 插件撤销事务 + 高亮事件（2026-09-15 开 PR）

> 两条宿主侧改动（都源于 OCSM 的 MCP/自动化实测缺口），各自一个上游 PR：
>
> | 项 | 内容 | 文件 | PR 分支 | fork 提交 | 报告 |
> |---|---|---|---|---|---|
> | **撤销事务** | `PluginRequest::BeginUndo/CommitUndo` + `HostApi::begin_undo/commit_undo`：宿主每条 message 末尾提交（空快照即丢）pending 快照，插件一次用户动作（HTTP 流程）必须显式开/关事务才能成一个撤销条目 | `crates/ocs_plugin_api/src/host.rs`、`.../ipc/{protocol.rs,server.rs,client.rs,v4/client.rs}`、`src/app/{document.rs,history.rs,plugin_host.rs}`、`docs/plugin-architecture.md` | `feat/plugin-undo-transaction` | `30425cbc` | `桌面/OCSM/PR-插件撤销事务-正文.md` |
> | **高亮事件** | `state.selection_revision` + `op:"events"` 的 `kind:"selection"` 事件（宿主本来只广播给 V4 插件，自动化客户端只能轮询） | `src/app/{control/mod.rs,update/mod.rs}`、`src/mcp.rs`、`docs/automation/README.md` | `feat/automation-selection-events` | `8845d5f8`+`7bb6d1e2` | `桌面/OCSM/PR-高亮选择事件-正文.md` |
>
> ⚠️ **同步注意（重要）**：`crates/ocs_plugin_api/src/host.rs` 与 `ipc/protocol.rs` 在本 fork 里还带着**尚未上游的 v5 插件面**（`ensure_layers/ensure_linetypes/ensure_text_styles/ensure_dim_styles`、
> `add_block_record`、`SelectedHandles`、`SetCurrentLayer`、frame picker 等；实测上游 `d738ebfc` 里 `ensure_layers`/`add_block_record`/`SelectedHandles` 均 **0 命中**）。
> 上游 PR **只含 v6 追加部分**（追加在 trait/enum 末尾，保 vtable 槽位），合并时两边都要留：上游 v6 段 ≠ fork v5 段，别把 fork 的 v5 段当成已上游。
>
> **同步动作**：上游合并后取上游版（内容一致，git 自动归并）；删本节 + 删分支 `feat/plugin-undo-transaction`、`feat/automation-selection-events`。
> 插件侧（`crates/ocs_ocsm`，非上游）已在 `20ea5ef6` 改用 Begin/Commit；命令驱动流程继续用 `push_undo`（同一条 message 内本来就成立）。

## 0.8 已开上游 issue · 面板扩展点 RFC（2026-09-16）

> **#1303**「RFC: plugin-supplied dock panels — declarative widget surface instead of embedding
> external windows」· https://github.com/HakanSeven12/OpenCADStudio/issues/1303
>
> **背景**：宿主 dock 的 `PanelId` 是**编译期枚举**（`dock.rs` 原注释：*"New palettes add a
> variant"*），第三方 `.so` 插件拿不到面板。本 fork 先做过通用方案 —— 宿主预留停靠列 + 上报矩形，
> 插件把自己的 GTK3+WebKitGTK 进程用 `XReparentWindow` 嵌进去（`cc5ec098`）：**实测像素级成功**
> （槽位 385×554 逻辑 @(640,184) → 481×692 设备像素 @(800,230)），但代价是
> ①X11/XWayland 专属（宿主须 `env -u WAYLAND_DISPLAY` 启动，Wayland/Windows/macOS/wasm 全废）
> ②多一个 GTK 进程 ③200ms 轮询矩形 + GDK 反复重申几何 ④焦点交接脆弱 ⑤为矩形握手扩 ABI（v7）
> —— 已在 `a4f122ec` 整体删除（−1811 行，含 `crates/ocs_webpanel` 5 个文件）。
>
> **issue 里给出两件事**：
> 1. **建议形态**：插件在 `plugin.toml` 声明面板 + 一棵声明式控件树（`row`/`column`/`scrollable`/
>    `label`/`button`/`text_input`/`checkbox`/`select`/`list`/`progress`），宿主用同一套 iced 控件渲染，
>    事件带 `widget_id` 回插件 —— 跨平台、wasm 无害、无第二进程；明确反对做立即模式绘图面。
> 2. **可独立成小 PR 的抓手**：每面板宽度策略（`min_width`/`max_width`/`max_fraction`）
>    + `Length::Fixed(width)` 宽度契约（`Fill` 面板会与画布抢同一行 —— fork「占半屏」的根因）。
>
> **同步动作**：无需同步代码。等维护者回复：若只要「小 PR」，按 §2 F-1 的宽度策略段落剥独立分支
> （英文注释、去掉助手面板依赖、以现有面板为消费者）；若接受声明式控件面，另开设计分支 ——
> **不要**复活 `cc5ec098` 的 reparent 方案。
> 本地材料：`~/桌面/OCSM/issue-插件面板UI-正文.md` + `issue-插件面板UI-链接.txt`。

## 0.9 分支已推、**PR 未开** · 插件请求跟「活跃图纸」（2026-09-16）

> **修复**：插件自建 HTTP 服务（OCSMechanical 的 `GET /api/guide`、`POST /api/apply_refresh`…）
> 发的**无 tab 请求**原先被 `Message::DrainPluginRequests` 从 **tab 0** 开始 drain，
> 于是“总是答在启动那张图上”：读错（`DocumentSnapshot`/`SelectedHandles`/图层扫描）、写也落错图。
> 一张图时看不出来，一旦有第二个标签（New/Open/自动化建图）就暴露（实测：活动图里的引导线
> 在插件 HTTP 侧报 `no such guide entity`，而插件命令 `GDIM/OCSM` 看得到）。
> 修法：drain 顺序改为**先活跃标签、再其它**（带 tab id 的请求仍回自己的 session）。
>
> | 项 | 值 |
> |---|---|
> | 文件 | `src/app/update/mod.rs`（drain 顺序） |
> | fork 提交 | `3b35b258` |
> | PR 分支 | `fix/plugin-requests-active-tab`（已推 `prf` 远端） |
> | 验证 | OCSMechanical + 自动化客户端建第二图：修前 `no such guide entity`/`400`，修后 `200` + 几何，
> 生成的标注落进**活跃**文档（`op:"query"` 在 8符号标注层查到）；宿主 `cargo test --lib` 973 passed |
> | 报告 | `桌面/OCSM/PR-插件请求活动标签页-{正文.md,链接.txt}` |
>
> **同步动作**：上游合并后取上游版（内容一致时 git 自动归并）；删本节 + 删分支
> `fix/plugin-requests-active-tab`。**本条 2026-09-17 才登记进台账**（先前只在 fork 提交里）。

## 0.10 分支未开 PR · 插件一步提交多实体（2026-09-17，`CommandStep::CommitEntities*`）

> **缺口**：插件命令一步只能落**一个**实体（`CommandStep::Commit` / `CommitAndEnd`），
> 而宿主 `CmdResult` 早就有 `CommitEntities` / `CommitEntitiesAndExit`（镜像/复制等内置命令在用）。
> 插件要画“复合几何”（例：`OCSMCENTERLINE` 的十字中心线 = **两条 `LINE`**）就只能
> ① 塞进匿名块（用户没法直接修剪/夹点），或 ② 走插件 HTTP 通道异步补实体（撤销分组/原子性变差）。
>
> **改法**（两处，纯追加）：
> * `crates/ocs_plugin_api/src/host.rs`：`CommandStep` **末尾**追加
>   `CommitEntities(Vec<EntityType>)` / `CommitEntitiesAndExit(Vec<EntityType>)`
>   —— 必须追加在末尾：该类型走 **bincode**（`ipc::transport`），动已有变体的判别值就断旧插件；
> * `src/app/plugin_host.rs`：`plugin_step_to_result()` 加两条映射到宿主同名 `CmdResult`。
>
> | 项 | 值 |
> |---|---|
> | 文件 | `crates/ocs_plugin_api/src/host.rs`（+~22）、`src/app/plugin_host.rs`（+2 + 1 测试） |
> | 验证 | 宿主 `plugin_commit_entities_and_exit_lands_both_in_one_undo_entry`：两条线一次落图 + 命令结束 + **一个**撤销条目 + `undo` 一次两条一起回去；插件侧 `centerline::` 15 个单测 |
> | 报告 | （待写）`桌面/OCSM/PR-插件多实体提交-正文.md` |
>
> **同步动作**：上游若接受，取上游版；若上游改成别的形态（如 `CommitMany`），
> 插件侧只需改 `centerline.rs` 的返回值（其余逻辑不依赖）。

## 0.11 上游同步执行记录（2026-09-17，**213 提交** / `fc1788df` → `65c0fe54`）

> 安全网：分支 `pre-sync-2026-09-17` + tag `presync-30c67f2f` → 合并前 `30c67f2f`。
> 手法：**在临时 worktree `/tmp/ocs-sync` 里合**（主工作树保持干净），解完冲突再
> `git merge --ff-only sync-2026-09-17` 快进主树 → 合并提交 `5200ef5d`。
>
> **8 个冲突的处置（上游 213 提交里真正跟我们撞车的只有这 8 处）**：
>
> | 文件 | 冲突 | 处置 |
> |---|---|---|
> | `Cargo.lock` | 1 | 取上游后由 cargo 重新生成（acadrust 要回到 ghfast 镜像源） |
> | `src/app/mod.rs` | 1 块 | 并集：fork `OcsmFramePicker` + 上游 `InsertTable`/`DataLinkManager`/`DataExtraction` |
> | `src/app/update/dialog.rs` | 1 块 | 并集：fork `PanelId::Assistant` + 上游 `ExternalReferences` |
> | `src/app/update/viewport.rs` | 1 块 | **取 fork**（上游只是把同一条件换行重排；保住 fork 的 `click_snap`/`click_pt`） |
> | `src/app/view/mod.rs` | 2 块 | 并集（面板派发 + 面板渲染两处） |
> | `src/app/view/modal.rs` | 1 块 | 并集：fork `OcsmFramePicker` 标题 + 上游三个新标题 |
> | `src/scene/mod.rs` | 1 块 | **取 fork**（`pub(crate) fn tessellate_one`，插件预览要调） |
> | `src/ui/dock.rs` | 6 块 | 并集：fork 助手面板宽度策略（`min_width`/`max_fraction`）+ 上游 xref 策略；`PanelId::ALL` 补 `ExternalReferences` |
>
> **两处非冲突但必须手工修的**：
> 1. `Cargo.toml`：上游把 acadrust 升到 rev `8a28c21` → 镜像 `[patch]` 段**跟升同一 rev**（URL 保持
>    `ghfast.top`）；`Cargo.lock` 里 `name = "acadrust"` 的 source 必须是 ghfast（不是 github.com）。
> 2. `src/ui/dock.rs` 的 `PanelId::ALL`：fork 用它遍历归一化宽度，**上游新增的变体要补进去**
>    （本次补 `ExternalReferences` → `[PanelId; 4]`）。
>
> **⚠️ 上游删掉了 `.cargo/config.toml` 的 `[net] git-fetch-with-cli = true`** —— 本次合并把它保住了
> （自动化合并恰好取 fork 侧）。**下次同步要盯**：没有它 libgit2 会忽略全局 `url.insteadOf`，
> 直接卡在 github.com 拉取。
>
> **验证（全绿）**：`cargo check --lib` ✅ ·
> 宿主 `cargo test -p OpenCADStudio --lib` **1360 passed / 0 failed / 18 ignored** ✅ ·
> 插件 `cargo test -p ocs_ocsm` **380 passed / 0 failed / 24 ignored** ✅ ·
> `--lib dimtmove` **5 passed** ✅ · `--test dim_leader_render_check` **1 passed** ✅ ·
> `--test leader_smoke_render` **1 passed** ✅ ·
> `OCS_SMOKE_PLUGIN=… installed_plugin…` **1 passed** ✅（新宿主加载新 .so 的 ABI 验证）·
> 两套 release 产物已重编并安装 ✅ · **实机 smoke** ✅（重启后 `TF a3_landscape 1:2 at 0,0` →
> 「已插入图框…比例 1:2（缩放 2.00 倍）」；Ø20 圆 `ZX` → 32 = 20 + 2×6）。
>
> **⚠️ 坑 3（本次最坑的一个）：上游 API≥4 新增 `acadrust_source` 门禁，插件会被静默拒绝。**
> 新宿主要求 `plugin.toml` 的 `[opencad]` **同时**声明 `rustc_version` 与 `acadrust_source`
> （上游 `docs/plugin-architecture.md:247/322`：值从 `Cargo.lock` 里 `name = "acadrust"` 的 `source` 取）；
> 漏填一个插件就直接不加载，命令行只报一句含糊的
> `✕ 插件"opencad.ocsm" 无法装入: Plugin built for acadrust @unknown, but this host uses @8a28c215…`。
> 已把部署步骤写成一个脚本防复发：**`tools/deploy_plugin.sh`**（读 Cargo.lock 填两个占位符 + 拷手册 +
> 自检占位符已替换 + 校验 `acadrust_source` 形状）。手动做法等价于：
> ```bash
> SRC=$(python3 -c "import re;print(re.search(r'name = \"acadrust\"\nversion = \"[^\"]+\"\nsource = \"([^\"]+)\"', open('Cargo.lock').read()).group(1))")
> sed -e "s|__RUSTC_VERSION__|$(rustc --version)|" -e "s|__ACADRUST_SOURCE__|$SRC|" \
>     crates/ocs_ocsm/plugin.toml > "$HOME/.config/OpenCADStudio/plugins/opencad.ocsm/plugin.toml"
> ```
>
> **⚠️ 坑 1：上游 xref 测试对语言敏感（不是合并问题）**：`src/app/commands/blocks.rs` 里上游新加的一批
> 用例断言**英文文案**（`"Path set"` / `"No external references"` / `"across drives"` …），
> 而 `locales/zh-CN/opencadstudio.ftl`（上游 `737666b8` 引入，fork 没改过 `locales/`）会把它们翻成中文
> → **在 zh-CN 语言环境下必然失败**（本次一次跑出 6 个，全模块强制英文后 **27 passed / 0 failed**）。
> 跑宿主测试请强制英文：`LC_ALL=C LANG=C LANGUAGE=C cargo test -p OpenCADStudio --lib`。
> → 值得给上游提 issue/PR（让这些测试固定 `Language::EnUs`，或断言 message id 而不是文案），
> 与上游开着的 **#1306**（"docs(plugin): fix inaccuracies and flag real gaps"）同一类。
>
> **⚠️ 坑 2：自动合并的“语义冲突”**：`tests/leader_smoke_render.rs`（fork 侧）还在调旧的 13 参数
> `export_pdf(...)`，而上游把它重构成了 `export_pdf(&PdfPageInput, &Path)`；两边改的是同一文件的
> **不同区域** → git 不报冲突，直接拼出一份编译不过的文件（`E0061`）。已改成新签名
> （`PlotContent { wires: Arc::new(wires), ..Default::default() }` + `PdfPageInput { … }`）。
> **教训：同一文件双方都改时，即使没冲突也要靠 `cargo test` 编一遍兜住。**
>
> **盯防清单（双方都改过的 23 个文件，本次只有 8 个真冲突，其余自动合并成功）**：
> `.cargo/config.toml`、`Cargo.toml`、`docs/plugin-architecture.md`、`src/app/commands/{display,mod}.rs`、
> `src/app/control/mod.rs`、`src/app/{document,history,mod}.rs`、`src/app/update/{dialog,mod,viewport}.rs`、
> `src/app/view/{mod,modal}.rs`、`src/command.rs`、`src/entities/{dimension,text_support}.rs`、
> `src/lib.rs`、`src/mcp.rs`、`src/plugin/external.rs`、`src/scene/mod.rs`、`src/ui/{dock,overlay}.rs`
>
> **哨兵（同步后必查，全过）**：`crates/ocs_plugin_api/src/host.rs` 的
> `ensure_layers`/`add_block_record`/`show_frame_picker`/`begin_undo`/`CommandStep::CommitEntities*`、
> `src/app/plugin_host.rs` 的 `plugin_preview_entity`/`inject_pick_snap`/`CmdResult::CommitEntities`、
> `src/scene/mod.rs` 的 `pub(crate) tessellate_one`、`src/app/update/viewport.rs` 的 `click_snap`/
> `entity_pick_accepts_points`、`src/app/commands/mod.rs` 的 `plugin_wins`、`src/plugin/external.rs`
> 的 `shutdown_plugins`、`src/mcp.rs` 的 `selection_revision`、`src/entities/dimension.rs` 的 C-2
> （`visible_len`）、`src/ui/assistant_panel.rs` 等 F 组新文件、`Cargo.toml` 的 `ocs_ocsm`/`ocs_ocsm_mcp`
> 成员 + ghfast 镜像。
>
> **上游本次顺带做的**（无需动作，仅备案）：`tests/sketch_constraints_solve.rs` →
> `tests/parametric_constraints_solve.rs`（改名）、删 `tests/sketch_constraints_xrecord_roundtrip.rs`、
> 加 `tests/pdf_export_images_check.rs`、新增 `crates/plugin-template-api2`。

## 0.12 上游同步执行记录（2026-09-21，**v2026.38 / 148 提交**）

> 同步点：上游 tag `v2026.38`（`0d023d26`，2026-09-20T15:45Z，周发布流程）→ **合并提交 `fd0f5dc2`**。
> 安全网：tag `presync-342d6505` + 分支 `pre-sync-2026-09-21`（合并前 `342d6505`）。
> 范围：`v2026.37..v2026.38` 共 372 提交 / 388 文件 / +117831 −15473，但 fork 上次已吃到 `65c0fe54`
> → **本次新增 = 148 提交 / 184 文件 / +38372 −2619**。`origin/main` 已领先 tag **23 提交**，本次不追。
>
> **7 个冲突的处置（比上次预案的 8 个更少，但首次出现插件面冲突）**：
>
> | 文件 | 冲突 | 处置 |
> |---|---|---|
> | `Cargo.lock` | 1 | acadrust source 行 = 「镜像 URL + 新 rev」：`ghfast.top…?rev=5b682ed#5b682ed6…` |
> | `crates/ocs_plugin_api/src/process/v4.rs` | 1 | **取上游**：上游把 `call_timeout`/`request_timeout`/`base_max_floor`/`request_kind` 搬进 `process.rs`；fork 的 3 个变体（`WantsTextInput`/`WantsMouseMove`/`EntityPickOsnap`）已在 `process.rs` 的共享 helper 里（自动合并保住），v4.rs 的本地副本整段删 |
> | `src/app/commands/mod.rs` | 1 | 并集：上游 `'PAN`/`'ZOOM` 透明前缀（`quoted`）+ fork `plugin_wins`（顺序：剥前缀 → 判插件 → 查别名） |
> | `src/app/mod.rs` | 2 块 | 并集：`ModalKind::OcsmFramePicker` + `ModalKind::Hyperlink`；两个 `ocsm_*` 初始化 + 5 个 `hyperlink_editor_*` |
> | `src/app/update/mod.rs` | 1 块 | 并集：`mod assistant;` + `mod page_setup_import;` |
> | `src/app/view/modal.rs` | 1 块 | 并集：`OcsmFramePicker` 标题 + 上游 `Hyperlink` 标题 |
> | `src/ui/dock.rs` | 1 块 | 并集：保留 `for id in PanelId::ALL`，**把上游新变体 `PanelId::Browser` 补进 `PanelId::ALL`**（`[PanelId; 4]` → `[PanelId; 5]`） |
>
> **两处非冲突但必须手工修的**：
> 1. `Cargo.toml`：上游把 acadrust 从 `8a28c21` 升到 `5b682ed` → 镜像 `[patch]` 段跟升同一 rev（URL 保持
>    `ghfast.top`）；顺带把 `crates/plugin-template-api2`、`docs/plugin-template-v2` 的样例 rev 也升齐。
> 2. `crates/ocs_ocsm/src/gear.rs` 的 `default_alpha_is_20_and_geometry_is_frozen`：acadrust `5b682ed` 给
>    `EntityCommon` 新增 `raw_record: Option<Arc<RawRecord>>`（serde skip、**Debug 会打印**）→ 靠
>    `format!("{:?}", 图元)` 哈希的四个冻结指纹全漂。图元数未变（section 14）下重新冻结：
>    `section 0x723163d7dfa02d27` / `side 0xdb9046b86e5a3e94` / `simplified 0x39e0cbb64e6195cd` /
>    `front 0xbb4f49b5f2d49d91`，并在测试注释里写明「升 acadrust 会漂、先验几何再改」。
>
> **⚠️ 坑（本次唯一一个）**：acadrust 给实体结构体加字段 = 插件里所有「Debug 串哈希」型冻结测试都会漂，
> 而 `cargo check` **看不见** → 同步后必须跑 `cargo test -p ocs_ocsm`。
>
> **验证**：`cargo check --lib` ✅（58s，acadrust/cadkernel 经 ghfast 拉到）·
> `--lib dimtmove` **6 passed** ✅ · `--test dim_leader_render_check` **1 passed** ✅ ·
> `cargo test -p ocs_ocsm` **559 passed / 0 failed / 25 ignored** ✅ ·
> release 双产物重编 ✅（宿主 2m16s / 插件 50s）·
> `OCS_SMOKE_PLUGIN=… installed_plugin_registers_its_ribbon` **1 passed** ✅ ·
> 宿主全量 `cargo test --lib`（`LC_ALL=C LANG=C`）**1546 passed / 1 failed / 18 ignored** —— 唯一失败见 §0.13（**上游自带**）。
> `tools/deploy_plugin.sh --skip-build` 已跑：装机 `plugin.toml` 的 `acadrust_source` 已刷成 `…?rev=5b682ed#…`
> （不刷会被宿主的 API≥4 指纹门禁**静默拒载**）。
>
> **上游本次顺带做的**（备案，无需动作）：新增 `src/ui/window/browser.rs`（Browser 面板：原点三平面/sketch/
> Solid3D 体）、`src/gpu_backend.rs`（GPU 探测 + 自动回退）、`src/ui/popup/context_menu.rs` +
> `src/app/update/context_menu.rs`（右键菜单重做）、参数化约束 6 个新模块（symmetric/equal/concentric/fixed/
> horizontal/smooth/constraint_bar）、plot 家族（`windows_media.rs`/`page_setup_import.rs`/`plotvars.rs`/
> PSETUPIN/页面设置单位）、Nix flake（`flake.nix`/`.envrc`）、`.github/workflows/web-check.yml`（wasm32 type-check）；
> 行为变更：**对象捕捉默认开启**、等轴测草图仅会话内。
>
> **盯防清单（本次双方都改过的 20 个文件，只有 7 个真冲突）**：`Cargo.toml`、
> `crates/ocs_plugin_api/src/process.rs`（+`process/v4.rs`）、`src/app/commands/{display,mod}.rs`、
> `src/app/control/mod.rs`、`src/app/document.rs`、`src/app/mod.rs`、`src/app/update/{dialog,mod,viewport}.rs`、
> `src/app/view/{mod,modal}.rs`、`src/command.rs`、`src/entities/{dimension,text_support}.rs`、`src/lib.rs`、
> `src/scene/mod.rs`、`src/ui/{dock,overlay}.rs`。
> 其中 `src/app/update/dialog.rs`、`update/viewport.rs`、`view/mod.rs`、`scene/mod.rs` **自动合并成功**
> （上次这四个都在冲突名单里）。

## 0.13 上游 stale 测试清单

> 收录「上游实现已变、上游测试未跟」的宿主单测。原则：**以实现意图为准，改测试不改实现**；
> 若确认是上游实现真的漏了 marker，只报告、不代改。

### S-1 `scene::parametric_constraints::tests::arc_grips_drive_center_start_and_end_but_not_midpoint`（已收口）

```bash
LC_ALL=C cargo test -p OpenCADStudio --lib \
  scene::parametric_constraints::tests::arc_grips_drive_center_start_and_end_but_not_midpoint
# v2026.38 时 FAILED：left = ParametricRef { entity: Handle(8), marker: None }（`ParametricRef::whole`）
#                      right = ParametricRef { entity: Handle(8), marker: Some(0) }（测试期待的 `point(handle,0)`）
```

- **用例名**：`arc_grips_drive_center_start_and_end_but_not_midpoint`（旧）→ 收口后改名
  `arc_grips_drive_center_or_the_whole_arc`。
- **原因**：上游 `9aadc97c`（PR **#1352** `da02e382`，*feat: complete symmetric constraint behavior*，
  2026-09-18）把 Arc/Circle/Ellipse 的非中心 grip 反查从 `point(handle, n)` 改为
  `ParametricRef::whole(handle)`——**端点 marker 不再产生是预期行为**，测试未同步 → stale。归属与
  `b24d04d6`/`829a69ae`（axis-hover-markers，只改 `constraint_hover_points` 悬停高亮）**无关**。
- **处置**：照上游修复 `ececb0ef`（2026-09-21，commit message 明说 *"The arc grip test follows #1352"*）
  照录测试：grip 0 → `center`，grips 1..=3 → `whole`，grip 4 → 空；本地**仅改测试期望、实现零改动**。
  下次同步上游时**直接取上游版**（本地该 hunk 与上游基本一致，至多多一句注释）。
- **日期**：发现 2026-09-21（v2026.38 同步，§0.12）· 收口 2026-09-23。
- **验证（2026-09-23）**：用例连跑 3 次 passed ✅；宿主全量 `cargo test --lib`
  **1547 passed / 0 failed / 18 ignored** ✅；`cargo test -p ocs_ocsm --lib` **633 passed / 0 failed / 25 ignored** ✅。

## 0.14 已上游化 · R13/R14 图层可打印标志（2026-09-28 开 PR **#1507**，bug：R14 图纸打印/预览全白）

> ✅ **本轮同步（上游 `1bbdce31`）已按判据删除 G-1（2026-09-28 二轮）**：
> **结论**：**G-1 已于 2026-09-28 二轮同步中删除；上游 `8c9b485b` 接管（R13/R14 打印标志改由 codec 提供），
> 判据 `pre_r2000_layer_plot_tests` 4/4 通过**。
> 上游 `8834d77d` 合了我们的 PR #1507，紧接着 `8c9b485b refactor(io): R13/R14 plot flags come from the codec now`
> 把修法**移进 codec 仓库**（`opencadcodec` reads R13/R14 LAYER records as plottable），宿主侧的
> `fix_pre_r2000_layer_plot_flags` 函数与两处调用（`load_bytes` 的 dwg 分支、`finalize_loaded_outcome`）
> 已**整段删除**；上游留下的 4 个测试改名为每例独立临时文件（`saved_path(test, version)`）后保留。
> **判据验证（不是照抄上游）**：删除后 `pre_r2000_layer_plot_tests` 4 例仍全过 ——
> R14 `load_bytes` 读回图层全可打印 · R13 走 `load_file`/`finalize_loaded_outcome` 路径入口 · 现代文件保留文件内标志
> （DEFPOINTS 语义）· R14 全部 wire `plot_visible`；本轮宿主全量 `cargo test --release --lib`
> **1830 passed / 0 failed / 23 ignored**（含这 4 例，详见 §验证）。
> **pin 同步**：`[patch."…opencadcodec.git"]` rev `42b44d2` → **`ad16215`**（ghfast 镜像保留），Lock 里 codec
> source 已核对为 `git+https://ghfast.top/…?rev=ad16215#ad16215faa…`；同批上游还升 kernel `237827e`、graph `fe36957`（无镜像 patch，跟上游官方 URL）。
> fork 提交 `6d88be35` / PR #1507 的内容随之作废（被上游实现取代，不再需要本地维护）。
> 本节保留为历史记录：下面是当初的现象、根因与当时的证据。

> **现象**：打开任何 R14（AC1014）图纸，屏幕显示完全正常，但 PLOT / 打印预览 / QUICKPRINT 出来的
> PDF 是空白页 —— 文件里除一个白底矩形和一个裁剪框外**没有任何绘制指令**（实测 1.2 KB，渲染全白）。
>
> **根因（既不在打印路径，也与打印样式无关）**：DWG 的 LAYER 记录**在 R2000 才引入 plottable 位**；
> R13/R14 只有 frozen / off / frozen-in-new-viewport / locked 四个独立位（LibreDWG `src/dwg.spec`
> 的 `VERSIONS (R_13b1, R_14)` 段，plotflag 自 `SINCE (R_2000b)` 起）。acadrust 的 DWG 读取器
> 在 R13/R14 分支不读该位，字段保留其初始 `false` → **整张图的每个图层都被当成“不打印”**
> （实测：库中 7 个图层 plottable 全 false，198 条 wire 全 `plot_visible=false`）。OCS 的打印收集
> （`plot_scene_content`）只保留 `plot_visible` 的线 → 一条不剩 → 空白页。屏幕不受影响，因为
> `display_visible` 与 `plot_visible` 是两套标志 —— 这正是“图看得见、打印空白”的原因。
> 影响面：`~/桌面/AI抄图练习` 库 303 张图中 **295 张是 AC1014**，即整批练习图的打印都受影响。
>
> | 项 | 值 |
> |---|---|
> | 文件 | `src/io/mod.rs`（+149：1 个加载期修正 + 2 处调用 + 4 个回归测试） |
> | fork 提交 | `6d88be35` |
> | 修法 | 新增 `fix_pre_r2000_layer_plot_flags`：`AC1012`(R13)/`AC1014`(R14) 时把图层恢复为可打印（格式的默认语义），`AC1015+` 原样保留文件里的标志（DEFPOINTS 这类真“不打印”图层不受影响）；挂在两条加载入口 —— `load_bytes`（内存/自动化）与 `finalize_loaded_outcome`（按路径打开） |
> | 验证 | 4 个新单测（R14 读回全可打印 / R13 走路径入口 / 现代文件保留原标志 / R14 的 wire 全部 `plot_visible`）· 宿主全量 `cargo test --lib` **1554 passed / 0 failed / 18 ignored** · 实测：用户图纸 + 其 dialog 设置（window `(54.09,91.11)-(321.68,264.36)`、A4 横向、Fit、`Monochrome.ctb`）导出 **1.2 KB 全白 → 47.9 KB 完整拨叉图**（尺寸/剖面线/中心线齐全，黑色输出证明样式一直正常） |
> | PR 分支 | `fix/r14-layer-plot-flags` ✅ 已推 `prf`（提交 `29a252de`），**PR [#1507](https://github.com/HakanSeven12/OpenCADStudio/pull/1507) 已开**（OPEN / MERGEABLE，1 file +149）—— 基点 = 上游 `78c9128a` |
> | 上游基点验证 | `cargo check --lib` ✅（4m44s，0 error，不动 `Cargo.lock`）· `cargo test --lib pre_r2000_layer_plot_tests` ✅ **4 passed**（上游 1759 个用例中的新增 4 个） |
> | 报告 | `桌面/OCSM/PR-R14图层可打印-{正文.md,链接.txt}` |
>
> **同步动作**：上游合并后**直接取上游版**，删本节、删分支。PR 版与本 fork 版内容一致，唯一差别是 crate 前缀 ——
> fork 侧仍是 `acadrust::`（依赖名未改），PR 版按上游重命名后的 `codec::`（`package = "opencadcodec"`）写；
> 上游这轮重命名随同步进来后会自然对齐。
> 合并前可用 `gh pr view 1507 -R HakanSeven12/OpenCADStudio` 查状态（`gh` 在 `~/.local/bin`，不在默认 PATH）。
> 若上游改在别处实现（例如直接改 acadrust 读 R14 时的默认值），保留“R14 图层默认可打印”这一语义
> 即可 —— 检查点：`git grep fix_pre_r2000_layer_plot_flags`，或实测任意 AC1014 图纸的打印/预览非空。
>
> ⚠️ **根因其实在依赖仓库**：真正缺位的是 `acadrust`（cadcodec）的 `read_layer` ——
> `dwg_stream_readers/object_reader/tables.rs` 里 `let mut plottable = false;` 只在 `r2000_plus()`
> 分支被赋值，于是 R13/R14 恒为 false（与其 `Layer::new()` 的默认值 `true` 也不一致）。cadcodec 不属本
> fork（属 HakanSeven12），所以修在 OCS 的加载层；要根治可另向 cadcodec 提 issue/PR（把默认值改为 `true`）。

## 0.15 CI 双红修复记录（2026-09-28，`a27360ed` 上的两条流水线）

背景：fork 的 `main` 上 `Tests`（run 36444537815）与 `Web build check`（run 36444537753）全红。
两条**都不是**依赖/网络问题，是 fork 自己的代码没按目标 cfg。

| # | 流水线 / 原命令 | 真因 | 改法 |
|---|---|---|---|
| ① | `Web build check`（job `wasm32`）<br>`cargo check --locked --target wasm32-unknown-unknown -p OpenCADStudio -p ocs_web_worker` | fork 代码在 web 构建里引用了**只在桌面存在**的东西（共 9 个错）：<br>(a) `ocs_plugin_api::host::{FrameItem,FrameSelection}` —— 根 `Cargo.toml` 的 wasm 段故意不带 `host` feature（`src/app/mod.rs` 两字段 + `OcsmFramePicker`、`src/app/view/modal.rs` 模态臂、`src/app/update/mod.rs` 回调）；<br>(b) `src/assistant.rs` 调桌面专属 `crate::config::config_dir()`；<br>(c) `src/app/commands/mod.rs` 的 `plugin_wins` 调桌面专属 `disabled_plugin_ids()`（定义在**上游** `src/app/update/file.rs`，带 `#[cfg(not(wasm32))]`）；<br>(d) `src/app/update/assistant.rs` 调桌面专属 `Scene::selection_fingerprint()`（定义在**上游** `src/scene/selection.rs`，`#[cfg(any(test,not(wasm32)))]`）。 | **只给 fork 自己的代码加 `#[cfg(not(target_arch = "wasm32"))]`**，上游文件一行不动（(c)(d) 是 fork 的**调用点**缺 cfg）：OCSM 图框选择器整块桌面化（`update::update_inner` 与 `view::modal` 两个 match **都没有** catch-all ⇒ 各补一条 wasm 合并/空臂）；`plugin_wins` 在 wasm 直接 `false`；助手面板的选中指纹缓存按目标出两版（web 每 poll 重算 label，纯缓存）；`backend_mode_path()` 加 wasm 版返回 `None`。<br>**`Cargo.toml` 不用改** —— `ocs_plugin_api` 早已按目标拆两段（`cfg(not(wasm32))` / `cfg(wasm32)`）。 |
| ② | `Tests`（job `test`）<br>`cargo test --workspace --locked` | `crates/ocs_plugin_api/examples/` 里 3 个文件是被 `#[path]` 引入的**模块片段**而非独立 bin：`proxy_decode.rs`、`part_lib.rs`、`svg_render.rs`（都没有 `fn main`，`svg_render.rs` 还用 `crate::part_lib::…`）⇒ `cargo test` 默认连 examples 一起编 ⇒ `E0601`/`E0433`。**与 `--locked`/镜像无关。** | 把三个片段挪进 `examples/kit/`（子目录里没有 `main.rs` ⇒ cargo 不当它是 example target），并改 5 处 `#[path]`（`kit/part_lib.rs` 里的 `#[path = "proxy_decode.rs"]` 是同目录兄弟，不用改）。`cargo metadata` 确认 examples 只剩 7 个真 bin。 |
| ③ | （预防，**不是**本次红的成因） | `Cargo.lock` 里 `opencadcodec` 的 source 是**第三方代理** `git+https://ghfast.top/https://github.com/…`，而 `.cargo/config.toml` 带 `git-fetch-with-cli = true` ⇒ CI runner 要拿 `git` 去拉这个代理。上游没这段。 | 摘掉根 `Cargo.toml` 的 `[patch."https://github.com/HakanSeven12/opencadcodec.git"]` 段（见 §2 A-3），重建锁 ⇒ source 回到官方 URL（rev 不变）。本机加速改由**全局 git insteadOf** 提供。 |

> ⚠️ **别再自造 wasm 复现**：`cargo check --target wasm32-unknown-unknown --workspace`（或任何选中
> `ocs_ocsm` 的组合）**会**失败 —— `ocs_ocsm` 是 cdylib 插件、开 `ocs_plugin_api/host`，features 统一后
> 把 `host`（⇒ `interprocess`，不支持 wasm32）灌进 wasm 图。CI 的 `-p OpenCADStudio -p ocs_web_worker`
> **不含** `ocs_ocsm`，两者不是一回事。判据：
> `cargo tree -i interprocess --target wasm32-unknown-unknown -p OpenCADStudio -p ocs_web_worker` ⇒ `nothing to print`。

## 1. 补丁总表（基准：上游 tag `v2026.38` = `0d023d26` → 合并 `fd0f5dc2`，**46 文件 / +11093 −103**，不含插件 crate）

> A–E 组的行数是 v2026.36 基准时的记录（功能性描述仍适用）；F 组为 2026-09-16 新增。

| 组 | 文件 | 规模 | 作用 |
|---|---|---|---|
| **A. 插件 API v5（基础设施）** | `crates/ocs_plugin_api/src/host.rs` | +349 | HostApi 扩展方法 + 数据类型 |
| | `crates/ocs_plugin_api/src/ipc/protocol.rs` | +238 | `PluginRequest/Response` 追加变体 |
| | `crates/ocs_plugin_api/src/ipc/client.rs` | +155 | 代理转发（V2/V3） |
| | `crates/ocs_plugin_api/src/ipc/v4/client.rs` | +177 | V4 代理转发 |
| | `crates/ocs_plugin_api/src/ipc/server.rs` | +19 | 服务端分发 |
| | `crates/ocs_plugin_api/src/process.rs` / `process/v4.rs` / `runner.rs` | +123 / +6 / +208 | 交互命令、预览实体、通知 |
| | `crates/ocs_plugin_api/src/manifest.rs` | +7 | `API_VERSION` 4 → **5** |
| | `Cargo.toml` | +5 | workspace 成员（ocs_ocsm / ocs_ocsm_mcp）+ **codec rev pin**（`[patch]` 镜像段已于 2026-09-28 摘除，见 §2 A-3） |
| **B. 宿主落地与 UI** | `src/app/plugin_host.rs` | +1095 | v5 请求在宿主的实现 |
| | `src/app/update/mod.rs` | +96 | drain 插件请求 + 图框选择回调 |
| | `src/app/view/modal.rs` | +75 | `OcsmFramePicker` 模态框 |
| | `src/app/mod.rs` | +38 | 图框选择/待取状态 |
| | `src/app/update/viewport.rs` | +88 | 插件命令的对象捕捉/点选语义 |
| | `src/app/view/mod.rs` | +5 | 插件点选模式隐藏十字 |
| | `src/ui/overlay.rs` | +61 | 拾取方框光标（纯方框，无十字） |
| | `src/command.rs` | +36 | 插件交互命令：预览实体、`hides_crosshair` 等钩子 |
| | `src/app/commands/mod.rs` | +17 | **插件命令优先于别名表**（`plugin_wins`） |
| | `src/scene/mod.rs` | +3 | `tessellate_one()`（预览用单实体剖分） |
| **C. 渲染器** | `src/entities/dimension.rs` | +189 | **C-1** DIMTMOVE=1 线性引线（本次）+ 4 测试；**C-2** 标注文字宽度自适应 |
| | `src/entities/text_support.rs` | +30 | MTEXT 花括号作用域字体恢复（`{\fGDT;x}` 供符号字形） |
| **D. 测试** | `src/plugin/external.rs` | +67 | `OCS_SMOKE_PLUGIN` 外部插件测试（纯新增） |
| | `tests/dim_leader_render_check.rs` | +107 | 引线渲染级测试（新文件） |
| **E. 插件本体（无需台账）** | `crates/ocs_ocsm/**`、`crates/ocs_ocsm_mcp/**` | 4400+ | 与上游天然解耦；仅 `Cargo.toml` 成员需保留 |
| **F. 内建面板：助手面板（fork 本地，2026-09-16）** | `src/ui/assistant_panel.rs`（新） | +2239 | 面板 UI（聊天流 / 审批 / 用量统计 / 表格横向滚动） |
| | `src/assistant.rs`、`src/assistant_rpc.rs`、`src/app/update/assistant.rs`（新） | +1957 / +961 / +328 | 助手 HTTP 后端 / 助手 RPC 子进程后端 / 消息处理 |
| | `src/ui/dock.rs` | ~+30 | `PanelId::Assistant` 变体 + `ALL` + `title()` + 每面板宽度策略（340 / 150 / 1200 / 0.4）+ 默认钉住 |
| | `src/app/view/mod.rs` | ~+20 | 面板渲染分支（展开态 L1863 / 收边态 L2945）+ 10Hz `assistant_poll`（L2589） |
| | `src/app/{mod.rs,document.rs,update/mod.rs,update/dialog.rs}` | ~+40 | `Message::Assistant` / `AssistantImagePasted` 路由、`show_assistant_panel` 开关、**每标签页** `AssistantPanelState`、× 关闭语义 |
| | `src/lib.rs`、`src/ui/mod.rs` | +3 | 模块注册（`assistant`、`assistant_rpc`、`assistant_panel`） |
| | 依赖 | 0 | **无新增 crate**（`ureq`/`base64`/`image` 上游本有；仅 `Cargo.lock` +3 行） |
| **G. IO 兼容修复（2026-09-28）** | `src/io/mod.rs` | +149 → **0** | ~~**G-1** R13/R14 图层可打印标志~~ —— **2026-09-28 二轮同步已删除**（上游 `8c9b485b` 让 codec 修根因，判据 `pre_r2000_layer_plot_tests` **4/4 通过**，见 §0.14） |

## 2. 关键补丁详情

### C-2 `src/entities/dimension.rs` — 标注文字宽度自适应（fork 本地）

- **位置**：`fn dimension_text_entity()`（`DA::` → `MA::` 映射之后）。
- **为什么**：cadcodec 的 `MText` 默认 `rectangle_width = 10`，长 dimtext（测量值 + 公差堆叠 +
  后缀，如 `Ø100+H7/g6+`）会被按 10 单位宽度折行。
- **改法**：按可见字符数估算宽度 `(visible_len * text_height * 0.75).max(10.0)`。
- **验证**：给标注加长文字/公差，观察不折行。
- **合并注意**：与 C-1 同文件不同区域；**上游 PR 分支不含此项**（见下）。

### C-1 `src/entities/dimension.rs` — DIMTMOVE=1 引线（2026-09-13，提交 `cbaf6d61f`）
> ⚠️ **已上游化**：PR **#1235** 已合并（`64e64c80`，上游改用 kernel 渲染器实现）
> → 下轮同步**按上游版、删本条目**，见 §0.5。

- **位置**：私有 `dimtmove_leader_endpoints()`（重写）+ 新增私有 `linear_leader()` +
  2 处调用点（`tessellate_dimension_inner` ≈L3768、baked 渲染 ≈L7183）+ 测试模块
  `dimtmove_leader_tests`（4 例）。
- **为什么**：旧实现把引线画成「尺寸线**中点** → 文字」的斜线；机械制图（GB/T 4458.4，
  同 AutoCAD DIMTMOVE=1）要求文字移出尺寸线范围时，**尺寸线沿轴延伸到文字底下**。
- **接口影响**：无（全部私有）；**行为被 `DIMTMOVE=1` 门控**（宿主默认 `dimtmove=0`，
  故上游/其它图档行为不变；目前只有 OCSM 生成/转换的标注在实体 XDATA 里置 1）。
- **验证**：`cargo test -p OpenCADStudio --lib dimtmove`；
  `OCSM_DIM_LEADER_PDF=/tmp/x.pdf cargo test --test dim_leader_render_check` → PDF 转图目视。
- **合并注意**：若上游重写该引线，保留"文字在外→水平延伸"这一形状语义（可对照本行说明）。
- **已提交上游 PR**（2026-09-13）：分支 `fix/dimtmove-leader-under-text`（fork
  `YsDirector/OpenCADStudio`），基点 = 上游 `main` `052b6b23`，提交 `e7675160`。
  PR 分支上该文件的注释为**英文**（上游语境），且**不含 C-2**（文字宽度补丁是 fork 本地项）。
  即：PR 分支与本仓库该文件会有注释语言差异——上游若合并，记得同步回来（或保留 fork 版本）。

### B-1 `src/app/commands/mod.rs` — 插件命令优先于别名表（`plugin_wins`）

- **为什么**：OCSM 的 `D`（OCSMPOWERDIM）、`CC`（粗糙度）必须压过随包别名 `D`→`*DIMSTYLE`、
  `CC`→`COPYCLIP`，否则用户敲不出来。
- **合并注意**：上游若重构别名解析（`src/app/alias.rs` 与 dispatch 顺序），要保证
  **先查插件命令、后查别名**。

### A-1 插件 API v5（`crates/ocs_plugin_api`）

- **新增 HostApi**（全部带默认实现，旧插件仍可加载）：`selected_handles`、`set_current_layer`、
  `ensure_layers`、`ensure_linetypes`、`ensure_text_styles`、`ensure_dim_styles`、
  `show_frame_picker`、`take_pending_frame_selection`、`import_frame_block`、
  `add_block_record`，以及交互命令钩子：`wants_text_input`、`on_text_input`、
  `wants_mouse_move`、`on_mouse_move`、`on_object_pick_snapped`、
  `entity_pick_applies_osnap`。
- **IPC**：`PluginRequest/PluginResponse` **末尾追加**变体（bincode discriminant 稳定）；
  类型：`LayerDef`/`LinetypeDef`/`TextStyleDef`/`DimStyleDef`/`FrameItem`/`FrameSelection`/
  `ImportFrameBlockRequest`。
- **`DimStyleDef` 类型专属 DIMVAR 补全（2026-09-23）**：`host.rs` 追加 `dimlfac`（线性比例，144）、
  `dimtfac`（公差字高，146）、`dimazin`（角度消零，79）、`dimfrac`（角度小数制，276）、
  `dimtmove`（文字移动，279）；默认值 1.0 / 1.0 / 0 / 0 / 0。
  **为什么**：宿主 `ensure_dim_styles` 对 def 未列字段从**当前样式**继承 → 外来图档（`TH_GBDIM`/
  `块内标注`……）的旧值会漏进 `OCSM_GB`，线性（dimlfac）/角度（dimazin+dimfrac）标注表现为
  “样式丢失”。插件 `dim_style_defs()` 已把它们显式钉死为黄金模板值。
  **合并注意**：字段追加在 `DimStyleDef` 中 `dimpost` 之后、`annotative` 之前（bincode 位置固定）；
  上游若改该结构，核对这 5 个字段、`Default` 与宿主映射（`src/app/plugin_host.rs::ensure_dim_styles`）别被覆盖。
- **合并注意**：若上游也改 `crates/ocs_plugin_api`，**优先取上游**，再把本组的追加项补回
  （追加在枚举/方法末尾，保持既有次序）。

### A-2 `Cargo.toml` — workspace 成员与 acadrust pin

- 成员新增 `crates/ocs_ocsm`、`crates/ocs_ocsm_mcp`；
  `acadrust` 固定到 fork 镜像 rev（`ghfast.top/.../cadcodec`）。
  **2026-09-14 上游同步时 rev 由 `5b56571a` 升到 `5eea24cf`（acadrust 0.5.5）**：上游 542 提交依赖
  更新的 cadcodec/cadkernel（DIMTMOVE kernel 渲染器等），沿用旧 rev 无法编译；**镜像 URL 保持 ghfast.top，
  未被覆盖成官方 URL**。宿主与插件经 `cargo tree` 确认共用同一份 acadrust（无重复版本），插件 178 测试全过
  → 实体 API 未破坏。这符合本节原定规则「上游更新 rev 时同步升 rev 并重跑插件测试」。
- **合并注意**：上游更新 acadrust rev 时，需确认插件依赖的实体 API（ATTDEF/块/标注字段）
  没变；必要时同步升 rev 并重跑插件测试。~~**不要**让上游覆盖成本地的镜像 URL（网络环境原因）~~
  —— **2026-09-28 起这条作废**：镜像 `[patch]` 已摘除，见 §2 A-3。

### A-3 `Cargo.toml` — opencadcodec 镜像 `[patch]` 摘除（2026-09-28，**CI 依赖，勿回退**）

- **原样**（曾长期存在）：`[patch."https://github.com/HakanSeven12/opencadcodec.git"]` 把上游的官方 URL
  指到 `https://ghfast.top/https://github.com/…`，本机拉得快；代价是 `Cargo.lock` 的 source 变成
  **第三方代理 URL**。
- **为什么必须摘**：fork 的 `Tests` / `Web build check` 跑在 GitHub runner 上，`--locked` 会照锁文件里的
  source 去 `git`（`.cargo/config.toml` 有 `[net] git-fetch-with-cli = true`）拉 **ghfast.top** ——
  等于把 CI 挂在一个不受我们控制、且上游没有的代理上。上游 CI 绿，差距就在这一段。
- **摘除后**：`Cargo.lock` 的 source 回到
  `git+https://github.com/HakanSeven12/opencadcodec.git?rev=ad16215#ad16215faa2b625d52346bdb1f147596567f70f7`
  （rev/commit 不变；`grep -c ghfast Cargo.lock` = **0**）。
- **本机加速不受影响**：`~/.gitconfig` 的
  `url.https://ghfast.top/https://github.com/.insteadof = https://github.com/`（与 `git@github.com:`）
  在 fetch 时改写官方 URL ⇒ 实际仍走镜像。取证：
  `GIT_TRACE=1 git ls-remote https://github.com/HakanSeven12/opencadcodec.git` 会打印
  `git-remote-https … https://ghfast.top/https://github.com/…`。**这是本机配置，不进仓库。**
- **同步规则（改后）**：上游升 codec rev 时，只改根 `Cargo.toml` /
  `crates/ocs_plugin_api/Cargo.toml` / `crates/ocs_web_worker/Cargo.toml` 的 `rev` 与 `Cargo.lock`，
  **不要把镜像 URL 写回仓库**。
- **2026-09-21（v2026.38）：rev `8a28c21` → `5b682ed`**（acadrust 0.5.5，上游这次升了 30 个提交，
  内容以 DWG/DXF IO 修复为主）。三处一起升：根 `Cargo.toml`、`[patch]` 镜像段、`Cargo.lock` 的 source 行
  （`ghfast.top…?rev=5b682ed#5b682ed66ea2c89be8142c8dd83d83774fc3de08`）。**副作用**：`EntityCommon` 新增
  `raw_record` 字段 → 见 §0.12 的 gear 指纹重冻结。升级后必须重跑 `tools/deploy_plugin.sh`（指纹门禁）。

### F-1 `src/ui/` 等 — 内建助手面板（fork 本地，2026-09-16）

> ★ 本节（及上表 F 组）按公开子集的**中性别名**书写该面板标识符（模块 `assistant*`、
> `PanelId::Assistant`、`Message::Assistant*`）；文件位置、行号与符号角色均未变。

- **为什么**：宿主 dock 的 `PanelId` 是编译期枚举，第三方插件拿不到面板（→ §0.8 的 RFC）。
  本面板是**内建**面板，用的就是上游 dock 自己的扩展方式（"New palettes add a variant"），
  **未改任何插件 ABI**（`ocs_plugin_api` 仍是 v5；`cc5ec098` 的 v7 追加已随 `a4f122ec` 删除）。
- **接点清单**（同步上游后逐项核对，缺一即面板报错或丢失；行号为 2026-09-16 的值，
  实际以符号名搜索 `PanelId::Assistant` / `assistant_poll` / `AssistantPanelState` / `on_assistant_msg` 为准）：
  1. `src/ui/dock.rs`：`PanelId::Assistant` 变体、`PanelId::ALL`（2026-09-21 起为 `[PanelId; 5]`，含上游的
     `ExternalReferences` 与 `Browser`）、`title()`、`default_width()` 340、
     `min_width()` 150、`max_width()` 1200、`max_fraction()` 0.4、`auto_collapse` 默认 `true`（钉住）。
  2. `src/app/view/mod.rs`：`expanded_panel` 的 `PanelId::Assistant` 分支（L1863）、收边/占用分支（L2945）、
     `assistant_poll` 订阅（L2589：`show_assistant_panel && worker.is_some()` 时 100ms）。
  3. `src/app/mod.rs`：`Message::Assistant`（L1900）、`show_assistant_panel` 开关（L688 / 初始化 L3659）。
  4. `src/app/document.rs`：**每标签页** 的 `assistant_panel: AssistantPanelState`（L133 / 初始化 L585）——
     上游若重构 Tab 结构，这处要跟着搬。
  5. `src/app/update/mod.rs`：`Message::Assistant` → `on_assistant_msg`、`Message::AssistantImagePasted` →
     `on_assistant_image_pasted`（L499/502）。
  6. `src/app/update/dialog.rs`：dock × 关闭语义（× 只是取消停靠，面板仍在栈里，`ASSISTANT` 命令可再开）。
  7. 模块注册：`src/lib.rs`（`pub mod assistant; pub mod assistant_rpc;`）、`src/ui/mod.rs`（`pub mod assistant_panel;`）、
     `src/app/update/`（`mod assistant;`）。
- **接口影响**：插件 ABI 无改动。但面板 view **必须** `.width(Length::Fixed(width))` —— `Fill`
  会与画布抢同一行空间（「占半个屏」的根因，提交 `675e552b`）；宽度策略在 `7ee77938`。
- **验证**：`cargo test --lib assistant` → **145 passed**；实测：`ASSISTANT` 命令开面板、后端标签
  （后端标签 `rpc · <cwd>` 或 `http · <url>`）、用量统计行、表格横向滚动、审批 UI
  （细节见 fork 本地 `docs/` 的面板扩展文档）。
- **合并注意**：
  - 按 §0 规则「冲突一律取上游版」会**丢掉全部 7 处接点**，必须手工补回（清单见上）；
    `PanelId::ALL` 的数组长度、`auto_collapse` 默认值、`Fixed(width)` 契约是三个易丢点。
  - 若上游同步后要跑 **wasm 构建**（上游有 pages 工作流），`src/assistant.rs` / `assistant_rpc.rs` /
    `ui/assistant_panel.rs` 依赖 `ureq`、文件系统与子进程 → 需 `#[cfg(not(target_arch = "wasm32"))]`
    门控，否则 wasm 目标编译失败。
  - 面板需要**运行时环境**（本机助手 CLI 或助手 HTTP 服务）：不影响构建，没有它时面板显示错误态。

### G-1 `src/io/mod.rs` — R13/R14 图层可打印标志（✅ **已删除**，2026-09-28 二轮同步）

- **删除理由**：上游 `8c9b485b` 把修法移进 `opencadcodec`（本 fork pin 已跟到 `ad16215`），宿主侧
  `fix_pre_r2000_layer_plot_flags` + 两处调用属重复实现，且与上游继续演进冲突；按 §0.14 判据删除。
- **删除判据（已实证）**：AC1014 图纸读回后图层与 wire 都可打印 —— `pre_r2000_layer_plot_tests`
  4 例（R14 `load_bytes` / R13 `load_file` 路径 / 现代文件保留原标志 / R14 wire `plot_visible`）在删除后全过（**4/4 通过**）。
- **历史**（供回溯）：fork 提交 `6d88be35`，+149（1 个加载期修正 + 2 处调用 + 4 个回归测试），
  PR [#1507](https://github.com/HakanSeven12/OpenCADStudio/pull/1507) 已被上游合并（`8834d77d`），
  修法随后被上游移进 codec。原改法与验证数字见 §0.14 与 `docs/sync-2026-09-28-冲突评估.md`。
- **回归检查点**：`git grep fix_pre_r2000_layer_plot_flags`（应为空）；
  `cargo test --release --lib pre_r2000_layer_plot_tests`（应 4 passed）。

## 3. 已知取舍

- 宿主补丁刻意保持**最小、可局部合并**：C 组只有 1 个文件、1 个私有函数；
  A/B 组是"给插件开的口子"，追加式修改，不动既有语义。
- 若上游接受，C-1 可作为上游 PR（旧行为从尺寸线中点拉引线本身可疑）。
  D 组渲染测试与 C-2 曾考虑作上游贡献候选，**用户判断采纳概率不大（2026-09-14）→ 不再推进**。
- ✅ **2026-09-14 状态更新**：C-1（→ PR #1235）与 Ctrl+点击超链接（→ PR #1234）**均已被上游合并**。
  `tests/leader_smoke_render.rs`（引线渲染测试）与 C-2（标注文字宽度自适应）**仍为 fork 本地**；
  用户判断这两项**上游采纳概率不大** → **不作为上游贡献推进**，仅本地保留。
- ⚠️ **2026-09-16 结论：「让第三方插件加窗口」的通用机制不作为上游候选。** 该机制
  （`cc5ec098`：`PanelId::Web` + `HostApi::web_panel_rect` + 插件 API v7 + `crates/ocs_webpanel`）
  因 X11/XWayland 专属等五点代价（见 §0.8）已在 `a4f122ec` 整体删除；**同步/合并时不要**把它
  当作「已上游化的能力」复活，也不要把它算作本 fork 的待推补丁。替代路线（插件声明式控件面）
  已开 issue #1303 征询意向。
- 供参考：内建助手面板的宿主侧改动属**本地独有**（F 组），**不由**上游候选清单管理；
  它依赖外部助手运行时，是产品选择而非通用补丁，故不推上游。
