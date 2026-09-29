#!/usr/bin/env bash
# 部署 OCSM 插件到 ${OCSM_PLUGIN_DIR:-~/.config/OpenCADStudio/plugins/opencad.ocsm}/
#
# 为什么要有这个脚本：`plugin.toml` 里有两个**部署时占位符**，宿主对 API≥4 的插件
# 两道门禁都靠它们，漏填一个插件就被静默拒绝加载（命令行只报一句
# `Plugin built for acadrust @unknown, but this host uses @…` / rustc 不匹配）：
#
#   __RUSTC_VERSION__   ← `rustc --version` 原串（Rust 无稳定 ABI，宿主按编译器卡）
#   __ACADRUST_SOURCE__ ← Cargo.lock 里 codec 依赖的 source 串
#                         （2026-09-28 起上游把仓库改名 opencadcodec、包名 `acadrust` → `opencadcodec`；
#                          脚本两个名字都认，形如 git+https://…opencadcodec.git?rev=42b44d2#<40位commit>）
#                         上游文档：docs/plugin-architecture.md「acadrust_source is also
#                         required for API v4 and later … reading acadrust_source out of Cargo.lock」
#
# ── 装什么（2026-09-27 补齐；此前只装 .so + plugin.toml + handbook/*.md 顶层，
#    `mkdir frame/` 之后从不拷内容 ⇒ 换机器/干净 XDG 部署时手册只有中文、无图框、缺明细表块）──
#
# 必需（插件**运行期**从**安装目录**解析，缺了功能即坏）——逐条给出源码证据：
#   libocs_ocsm.so            插件本体
#   plugin.toml               宿主加载门禁（上面两个占位符已替换）
#   handbook/*.md             手册中文篇（`OCSMHELP`）
#                             src/guide_server.rs:4246 manual_dirs() → 4254 `dir.join("handbook")`
#                             → 4281 `scan_md_topics` 的 `std::fs::read_dir(dir)`
#   handbook/en/*.md          手册英译篇（`OCSMLANG=en`；中文原文是源）
#                             src/guide_server.rs:4330 `scan_md_topics(&dir.join("en"))` → 4370/4376 读正文
#   bom/OCSM_BOMHEAD.dwg      明细表表头块
#                             src/bom.rs:94 `bom_dir()`（env `OCSM_BOM_DIR` 覆盖）→ 116 `dir.join("bom")`；
#                             116→583 `dir.join("OCSM_BOMHEAD.dwg")`
#   bom/OCSM_BOMROW.dwg       明细表行块（8 个单元格 ATTDEF）—— src/bom.rs:584
#   frame/*.dwg               图框（`TF`/`OCSMFRAMEINSERT` 扫描此目录）
#                             src/lib.rs:348 `frame_dir()`（env `OCSM_FRAME_DIR` 覆盖）→ 361
#                             `dir.join("frame")` → 4644 `std::fs::read_dir(&dir)`
#   **frame/ 内容不在仓库里**（`crates/ocs_ocsm/` 下没有 `frame/` 目录）：样例图框在本机
#   `~/桌面/OCSM/frame/`（PLUGIN.md「构建与安装」第 3 步是**手工 cp**），装机前由
#   `tools/frame_clean.py --install`（该脚本 PLUGIN_FRAME_DIR = 插件目录的 frame/）清理。
#   ⇒ 本脚本把它当**用户自备/外部生成**数据：给了 `OCSM_FRAME_SRC=<目录>` 才去拷（同名不覆盖），
#     否则不动 frame/。自检里 frame/ 为空**默认只警告 + 指路，不算失败**（rc 仍 0 —— 全新用户
#     还没有图框，公开 README 教人跑的就是这个脚本，首次体验不该是“部署失败”）；要把它当硬门禁
#     （发布/CI）请显式设 `OCSM_REQUIRE_FRAME=1`。
#
# 运行期**状态**（用户改过的不能被部署回滚）：
#   bom/settings.json         明细表配置（`OCSM_BOMCFG` 写它）—— src/bom.rs:157 `config_path()`
#                             ⇒ **存在就不覆盖**；不存在才用仓库里的默认值初始化
#
# 不装（**编译期**已 include 进 .so，部署不需要；README/证据见下）：
#   assets/                   卡片锚点与标准数据 CSV **全部**经 `include_str!`/`include_bytes!`
#                             编译进 .so：src/nf_table.rs（`../assets/nf_internal_card_anchor.csv`）、
#                             src/nf_ext_table.rs、src/din_table.rs:119/142/144、
#                             src/invol_spline.rs:350/902/1602/2446、src/partgen_keys.rs
#                             （`../assets/key1096_shaft_ranges.csv`）、图标 src/lib.rs 的
#                             `include_bytes!("../assets/icons/*.svg")`。
#                             全仓**没有运行期**路径读 assets/：唯一的 `join("assets")` 在
#                             src/guide_server.rs:9071-9076，是 `#[cfg(test)]` 的 node 冒烟软链。
#   src/tables/*.json         同理由 `include_str!` 编进 .so（src/hole.rs:157+、src/partgen_*.rs）
#   crates/ocs_ocsm/src/      源码（.so 里没有对它的运行期读取；`CARGO_MANIFEST_DIR` 只在
#                             `include!` 与 `#[cfg(test)]` 里出现，是编译期常量）
#
# 用法：tools/deploy_plugin.sh [--skip-build]
#   OCSM_PLUGIN_DIR=<目录>     改安装目录（默认 ~/.config/OpenCADStudio/plugins/opencad.ocsm）
#   OCSM_FRAME_SRC=<目录>      从该目录装 `*.dwg` 图框（可选；已存在的同名文件不覆盖）
#   OCSM_REQUIRE_FRAME=1      严格模式：frame/ 里一个图框都没有 ⇒ 按失败处理（非零退出）
#                             —— 不设就是默认的“警告 + 指路 + rc 0”
# 旧变量 `OCSM_SKIP_FRAME_CHECK` 已**删除**（默认就不再因 frame 为空而失败，它已无意义；
# 传了也只是被忽略的空字符串，不会有任何效果）。
set -euo pipefail

cd "$(dirname "$0")/.."
export PATH="$PATH:$HOME/.cargo/bin"

PLUGIN_DIR="${OCSM_PLUGIN_DIR:-$HOME/.config/OpenCADStudio/plugins/opencad.ocsm}"
SRC=crates/ocs_ocsm

if [[ "${1:-}" != "--skip-build" ]]; then
    echo "==> 编插件（release）"
    cargo build --release -p ocs_ocsm
fi

ACADRUST_SRC=$(python3 - <<'PY'
import re, sys, pathlib
lock = pathlib.Path('Cargo.lock').read_text(encoding='utf-8')
# 2026-09-28：上游把依赖仓库 cadcodec 改名 opencadcodec（包名 acadrust → opencadcodec，
# 依赖别名 codec）——三个名字都认，优先新名。
m = None
for pkg in ('opencadcodec', 'codec', 'acadrust'):
    m = re.search(rf'name = "{pkg}"\nversion = "[^"]+"\nsource = "([^"]+)"', lock)
    if m:
        break
if not m:
    sys.exit('Cargo.lock 里找不到 opencadcodec / acadrust 的 source')
src = m.group(1)
hash_part = src.rsplit('#', 1)[-1]
if len(hash_part) != 40 or any(c not in '0123456789abcdef' for c in hash_part):
    sys.exit(f'opencadcodec source 末尾不是 40 位 commit：{src}')
print(src)
PY
)
RUSTC_VER=$(rustc --version)
# 版本口径 = plugin.toml 的 [plugin] version（宿主显示的就是它；也是 Cargo.toml/MANIFEST 的一致性锚点）
PLUGIN_VER=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$SRC/plugin.toml" | head -1)
[[ -n "$PLUGIN_VER" ]] || { echo "✗ plugin.toml 里读不到 [plugin] version" >&2; exit 1; }

# 递归复制目录内容（保留相对层级；只增不删，目标是安装目录，不碰用户其它文件）
# 用法：copy_tree <源目录> <目标目录> [跳过的相对路径 ERE]
copy_tree() {
    local src="$1" dst="$2" skip="${3:-}" rel
    while IFS= read -r -d '' rel; do
        rel="${rel#./}"
        if [[ -n "$skip" && "$rel" =~ $skip ]]; then continue; fi
        mkdir -p "$dst/$(dirname "$rel")"
        cp -p "$src/$rel" "$dst/$rel"
    done < <(cd "$src" && find . -type f -print0)
}

mkdir -p "$PLUGIN_DIR/handbook" "$PLUGIN_DIR/bom" "$PLUGIN_DIR/frame"

echo "==> 装 .so + plugin.toml（rustc=${RUSTC_VER}；acadrust=…${ACADRUST_SRC: -40}；版本=${PLUGIN_VER}）"
cp target/release/libocs_ocsm.so "$PLUGIN_DIR/"
sed -e "s|__RUSTC_VERSION__|${RUSTC_VER}|" \
    -e "s|__ACADRUST_SOURCE__|${ACADRUST_SRC}|" \
    "$SRC/plugin.toml" > "$PLUGIN_DIR/plugin.toml"

# 手册：**递归**装（含 en/ 英译篇）。跳过 handbook/tools/ —— 那里面有
# install-handbook.sh（「仓库 → 安装目录」同步脚本），装进安装目录后它的 SRC/DST 会同时在
# 安装目录（`find -delete` 后自拷贝）⇒ 一跑就把已部署手册清空；且运行期只读顶层 md 与 en/。
echo "==> 装手册 handbook/（递归，含 en/；跳过 tools/）"
copy_tree "$SRC/handbook" "$PLUGIN_DIR/handbook" '^tools/'

# 明细表模板块：跳过 *.bak（工作副本）与 settings.json（运行期状态，下面单独处理）
echo "==> 装明细表模板块 bom/（跳过 *.bak）"
copy_tree "$SRC/bom" "$PLUGIN_DIR/bom" '\.bak$|^settings\.json$'

if [[ -f "$PLUGIN_DIR/bom/settings.json" ]]; then
    echo "==> bom/settings.json：已存在 ⇒ 保留（运行期状态，不覆盖）"
else
    cp -p "$SRC/bom/settings.json" "$PLUGIN_DIR/bom/settings.json"
    echo "==> bom/settings.json：初始化默认值（安装目录里原来没有）"
fi

if [[ -n "${OCSM_FRAME_SRC:-}" ]]; then
    echo "==> 装图框 frame/（OCSM_FRAME_SRC=$OCSM_FRAME_SRC；同名不覆盖）"
    if [[ ! -d "$OCSM_FRAME_SRC" ]]; then
        echo "✗ OCSM_FRAME_SRC 不是目录：$OCSM_FRAME_SRC" >&2
        exit 1
    fi
    shopt -s nullglob nocaseglob
    for f in "$OCSM_FRAME_SRC"/*.dwg; do
        b=$(basename "$f")
        if [[ -e "$PLUGIN_DIR/frame/$b" ]]; then
            echo "    跳过（安装目录已有）：$b"
        else
            cp -p "$f" "$PLUGIN_DIR/frame/"
            echo "    + $b"
        fi
    done
    shopt -u nullglob nocaseglob
else
    echo "==> frame/：未给 OCSM_FRAME_SRC ⇒ 保留现有图框（用户自备数据，脚本不生成）"
fi

# ── 清理安装目录 bom/ 里陈旧的 *.bak（构建残留）────────────────────────────
# 本脚本从源仓库装 bom/ 时已经跳过 *.bak；但**旧版手工 `cp crates/ocs_ocsm/bom/*`** 会把构建
# 残留一起带进来，而下面自检把「bom/ 里不该有 *.bak」当不变量 ⇒ 这里自愈，而不是让用户手删。
# 只删 bom/ 这一层里的 *.bak 文件：glob 只可能匹配 *.bak，非普通文件跳过，删完**复验**，
# 没删掉就报出来（别把操作失败当逻辑生效）；其余文件/目录一律不碰。
if [[ -d "$PLUGIN_DIR/bom" ]]; then
    shopt -s nullglob
    stale_bak=( "$PLUGIN_DIR/bom"/*.bak )
    shopt -u nullglob
    if (( ${#stale_bak[@]} > 0 )); then
        echo "==> 清理 bom/ 里陈旧的 *.bak（${#stale_bak[@]} 个；构建残留，自检要求 bom/ 不含 *.bak）"
        for f in "${stale_bak[@]}"; do
            [[ -f "$f" ]] || { echo "  ⚠ 跳过（不是普通文件）：$f" >&2; continue; }
            rm -f -- "$f"
            if [[ -e "$f" ]]; then
                echo "  ⚠ 删除失败，仍存在：$f" >&2
            else
                echo "  - 已删：$f"
            fi
        done
    fi
fi

# ── 自检：装了哪些（计数）+ 必需数据是否齐全（缺哪项就非零退出）──────────────
echo "==> 自检"
MISSING=0
fail() { echo "✗ $*" >&2; MISSING=1; }

n_so=$(find "$PLUGIN_DIR" -maxdepth 1 -type f -name 'libocs_ocsm.so' | wc -l)
n_toml=$(find "$PLUGIN_DIR" -maxdepth 1 -type f -name 'plugin.toml' | wc -l)
n_zh=$(find "$PLUGIN_DIR/handbook" -maxdepth 1 -type f -name '*.md' 2>/dev/null | wc -l)
n_en=$(find "$PLUGIN_DIR/handbook/en" -maxdepth 1 -type f -name '*.md' 2>/dev/null | wc -l)
n_bom=$(find "$PLUGIN_DIR/bom" -type f 2>/dev/null | wc -l)
n_frame=$(find "$PLUGIN_DIR/frame" -maxdepth 1 -type f -iname '*.dwg' 2>/dev/null | wc -l)
n_bak=$(find "$PLUGIN_DIR/bom" -type f -name '*.bak' 2>/dev/null | wc -l)

printf '  %-22s %4d\n' 'libocs_ocsm.so' "$n_so"
printf '  %-22s %4d   版本 %s\n' 'plugin.toml' "$n_toml" "$PLUGIN_VER"
printf '  %-22s %4d   （源 %s）\n' 'handbook/*.md' "$n_zh" "$(find "$SRC/handbook" -maxdepth 1 -type f -name '*.md' | wc -l)"
printf '  %-22s %4d   （源 %s）\n' 'handbook/en/*.md' "$n_en" "$(find "$SRC/handbook/en" -maxdepth 1 -type f -name '*.md' | wc -l)"
printf '  %-22s %4d   （源 %s，不含 *.bak）\n' 'bom/' "$n_bom" "$(find "$SRC/bom" -type f ! -name '*.bak' | wc -l)"
printf '  %-22s %4d   %s\n' 'frame/ *.dwg' "$n_frame" "${OCSM_FRAME_SRC:+（来自 OCSM_FRAME_SRC）}"

# 1) .so / plugin.toml
(( n_so == 1 )) || fail ".so 缺失：$PLUGIN_DIR/libocs_ocsm.so"
(( n_toml == 1 )) || fail "plugin.toml 缺失：$PLUGIN_DIR/plugin.toml"
if [[ -f "$PLUGIN_DIR/plugin.toml" ]]; then
    # 两个占位符都已替换（别让宿主用一句含糊的话拒绝加载）
    if grep -q "__RUSTC_VERSION__\|__ACADRUST_SOURCE__" "$PLUGIN_DIR/plugin.toml"; then
        fail "plugin.toml 仍有未替换的占位符"
    fi
    if ! grep -q '^acadrust_source = "git+' "$PLUGIN_DIR/plugin.toml"; then
        fail "plugin.toml 的 acadrust_source 形状不对"
    fi
fi

# 2) handbook：中文篇 + 英译篇都要有（且不少于源，防递归漏装）
src_zh=$(find "$SRC/handbook" -maxdepth 1 -type f -name '*.md' | wc -l)
src_en=$(find "$SRC/handbook/en" -maxdepth 1 -type f -name '*.md' | wc -l)
(( n_zh >= src_zh && n_zh > 0 )) || fail "handbook 缺失：$PLUGIN_DIR/handbook 有 $n_zh 篇 ≤ 源 $src_zh 篇"
(( n_en >= src_en && n_en > 0 )) || fail "handbook/en 缺失：$PLUGIN_DIR/handbook/en 有 $n_en 篇 ≤ 源 $src_en 篇（英译篇漏装）"

# 3) bom：两个模板块必须在（块内容本身由宿主 import_frame_block 读）
for f in OCSM_BOMHEAD.dwg OCSM_BOMROW.dwg; do
    [[ -s "$PLUGIN_DIR/bom/$f" ]] || fail "bom 缺失：$PLUGIN_DIR/bom/$f（明细表建表要它）"
done
(( n_bak == 0 )) || fail "bom 里混进了 *.bak：$n_bak 个（应被跳过）"
for f in README.md settings.json; do
    [[ -s "$PLUGIN_DIR/bom/$f" ]] || fail "bom 缺失：$PLUGIN_DIR/bom/$f"
done

# 4) frame：**用户自备**数据 ⇒ 默认只**警告 + 指路**（rc 仍 0，不算失败）；要当硬门禁就
#    显式设 OCSM_REQUIRE_FRAME=1（发布/CI 用）。其余自检项不受此开关影响，仍硬失败。
if (( n_frame == 0 )); then
    FRAME_HINT="frame/ 没有任何 *.dwg：TF/OCSMFRAMEINIT 会报「找不到图框文件夹」。请把图框 DWG 放进 $PLUGIN_DIR/frame/，或部署时用 OCSM_FRAME_SRC=<放图框的目录> 一起装（样例图框可用 tools/frame_clean.py --install 清洗后落进 frame/）。"
    if [[ "${OCSM_REQUIRE_FRAME:-}" == "1" ]]; then
        fail "frame 缺失（OCSM_REQUIRE_FRAME=1 严格模式）：$FRAME_HINT"
    else
        echo "  ⚠ $FRAME_HINT" >&2
    fi
fi

if (( MISSING != 0 )); then
    echo "✗ 部署自检失败：安装目录 $PLUGIN_DIR 的必需数据不齐全（见上）" >&2
    exit 1
fi

echo "✓ 部署完成：$PLUGIN_DIR"
echo "  重启 OCS 后命令行应出现：Loaded plugin: OCSMechanical 机械工具包 (opencad.ocsm ${PLUGIN_VER})"
