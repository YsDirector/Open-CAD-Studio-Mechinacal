#!/usr/bin/env bash
# 把 OCSM 手册（教程 + 知识，18 篇 md + tools/）同步到**插件安装目录**。
#
# 为什么需要：插件的运行时就近读 `~/.config/OpenCADStudio/plugins/opencad.ocsm/handbook/`，
# 而手册的单一真源在插件仓库 `crates/ocs_ocsm/handbook/`（随版本管理、开源一起分发）。
# 改完手册 → 跑一下本脚本 → 在 OCS 里 `OCSMHELP` 刷新窗口即可看到新版（**不需要重编译插件**）。
#
# 用法：
#   bash handbook/tools/install-handbook.sh              # 同步到默认安装目录
#   OCSM_PLUGIN_DIR=/path/to/plugin bash .../install-handbook.sh
set -eu
SRC="$(cd "$(dirname "$0")/.." && pwd)"
DST="${OCSM_PLUGIN_DIR:-$HOME/.config/OpenCADStudio/plugins/opencad.ocsm}/handbook"
mkdir -p "$DST"
find "$DST" -mindepth 1 -delete
cp -a "$SRC/." "$DST/"
echo "手册已同步：$SRC"
echo "          → $DST"
ls "$DST" | wc -l | xargs echo "          条目数："
