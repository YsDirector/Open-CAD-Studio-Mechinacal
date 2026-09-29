#!/usr/bin/env bash
# 第四批参数化标准件：一键出「复核图」
#
# 做什么：
#   1) 跑四个模块的 ignored dump 测试（各族 × 各视图 → /tmp/bN/*.svg）
#   2) 用 rsvg-convert 转 PNG，再按组分片成 contact sheet（montage）
#   3) 有 overlay-*.svg 的（模板红 / 生成蓝叠合图）单独再拼一张
#
# 用法：bash tools/batch4_review.sh [组号...]      # 默认 b1 b2 b3 b4
set -uo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
REPO="$(cd "$(dirname "$0")/.." && pwd)"
GROUPS=("${@:-b1 b2 b3 b4}")
if [ $# -eq 0 ]; then GROUPS=(b1 b2 b3 b4); fi
OUT=/tmp/batch4_review
mkdir -p "$OUT"

cd "$REPO"
for g in "${GROUPS[@]}"; do
  echo "=== [$g] 编译并跑 dump 测试 ==="
  cargo test -q -p ocs_ocsm --lib "${g}::" -- --ignored "dump_${g}_svg" --nocapture 2>&1 | tail -5
done

for g in "${GROUPS[@]}"; do
  dir="/tmp/$g"
  [ -d "$dir" ] || continue
  # 单视图 PNG
  for svg in "$dir"/*.svg; do
    [ -e "$svg" ] || continue
    png="${svg%.svg}.png"
    [ -f "$png" ] || rsvg-convert -b white -o "$png" "$svg"
  done
  # 生成图 contact sheet（排除 overlay）
  if ls "$dir"/*.png >/dev/null 2>&1; then
    mapfile -t files < <(ls "$dir"/*.png | grep -v overlay || true)
    if [ "${#files[@]}" -gt 0 ]; then
      montage "${files[@]}" -tile 4x -geometry 460x340+6+6 -background white \
        "$OUT/${g}_all.png" 2>/dev/null && echo "→ $OUT/${g}_all.png（${#files[@]} 张）"
    fi
  fi
  # 叠合图
  if ls "$dir"/overlay-*.png >/dev/null 2>&1; then
    montage "$dir"/overlay-*.png -tile 2x -geometry 760x560+6+6 -background white \
      "$OUT/${g}_overlay.png" 2>/dev/null && echo "→ $OUT/${g}_overlay.png"
  fi
done
echo "完成：$OUT"
