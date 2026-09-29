#!/usr/bin/env python3
"""matplotlib 中文字体开关（画验证图/叠加图时用）。

为什么要有这个文件：matplotlib 默认字体是 DejaVu Sans，**没有中文字形** ——
中文标题/图例会渲染成一个个空心方框（终端里还会有
`UserWarning: Glyph 27169 missing from font(s) DejaVu Sans`），
看图的人只能靠猜。这个项目里经常要出"叠加图/对比图"给人验收，
所以统一在这里开关。

用法：

    import sys; sys.path.insert(0, "<repo>/tools")
    from mpl_zh import use_zh
    use_zh()                      # 配好中文字体（顺带关掉负号乱码）
    import matplotlib.pyplot as plt
    ...

或者命令行自检（会渲染一张样图并检查有没有缺字形）：

    python3 tools/mpl_zh.py /tmp/font_check.png
"""

from __future__ import annotations

import os

# 按优先级尝试的字体族（本机实测：思源黑体/Noto Sans CJK SC 都有）
CANDIDATES = (
    "Source Han Sans SC",   # 思源黑体
    "Noto Sans CJK SC",     # 同源
    "WenQuanYi Zen Hei",    # 文泉驿
    "WenQuanYi Micro Hei",
    "Microsoft YaHei",      # 万一在 Windows 上跑
    "PingFang SC",          # macOS
)


def pick_font() -> str | None:
    """挑一个本机装着的 CJK 字体名。"""
    try:
        from matplotlib import font_manager as fm
    except ImportError:  # pragma: no cover
        return None
    installed = {f.name for f in fm.fontManager.ttflist}
    for name in CANDIDATES:
        if name in installed:
            return name
    # 兜底：扫描字体文件名里带 CJK/Han 的
    for f in fm.fontManager.ttflist:
        low = (f.name or "").lower()
        if "cjk" in low or "han" in low or "hei" in low:
            return f.name
    return None


def use_zh(font: str | None = None, verbose: bool = False) -> str | None:
    """把 matplotlib 配成能画中文。返回选中的字体名（None = 没找到）。"""
    import matplotlib

    name = font or pick_font()
    if name:
        # 放在最前面，别的字体当后备（数字/西文用 DejaVu 更好看）
        matplotlib.rcParams["font.sans-serif"] = [name, "DejaVu Sans"]
        matplotlib.rcParams["font.family"] = "sans-serif"
    # 负号：默认 U+2212 在中文字体里常常也没有 → 用 ASCII 减号
    matplotlib.rcParams["axes.unicode_minus"] = False
    if verbose:
        print(f"[mpl_zh] 使用字体：{name or '（没找到 CJK 字体，中文仍会显示成方框）'}")
    return name


def main() -> int:
    import argparse
    import warnings

    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    ap = argparse.ArgumentParser(description="渲染中文样图并检查缺字形")
    ap.add_argument("out", nargs="?", default="/tmp/mpl_zh_check.png")
    ap.add_argument("--font", default=None, help="强制指定字体族名")
    args = ap.parse_args()

    name = use_zh(args.font, verbose=True)
    sample = "齿轮 剖视图 侧视图 简化正视图 常规正视图 模数 齿数 螺旋角 根切 −0.05"
    fig, ax = plt.subplots(figsize=(7.5, 3.0))
    ax.set_title("中文字体自检：齿廓 / 齿根过渡 / 渐开线")
    ax.text(0.03, 0.72, sample, fontsize=13)
    ax.text(0.03, 0.50, "次摆线等距线（真根切）", fontsize=13)
    ax.text(0.03, 0.28, "标注：直径 Ø84、齿根圆角 ρ=0.38m、偏差 −0.05", fontsize=12)
    ax.text(0.03, 0.06, "green=模板 DXF   red=OCSMGEAR 生成", fontsize=10, color="#666")
    ax.set_xticks([])
    ax.set_yticks([])
    fig.tight_layout()

    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        fig.savefig(args.out, dpi=110)
    missing = [str(w.message) for w in caught if "missing from font" in str(w.message)]
    print(f"[mpl_zh] 样图已写：{args.out}")
    if missing:
        print(f"[mpl_zh] ✗ 仍有 {len(missing)} 个字形缺失（第一个：{missing[0][:80]}…）")
        if name is None:
            print("[mpl_zh] 原因：本机没找到 CJK 字体。装一个即可，例如：")
            print("        sudo pacman -S noto-fonts-cjk   （或 ttf-source-han-sans-sc）")
        return 1
    print("[mpl_zh] ✓ 中文/负号全部有字形，没有缺字形告警")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
