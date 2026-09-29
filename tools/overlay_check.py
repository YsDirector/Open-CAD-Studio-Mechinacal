#!/usr/bin/env python3
"""模板 vs 生成件 叠合比对器（OCSM 参数化标准件自查用）。

用法：
    python3 tools/overlay_check.py <模板.dxf> <生成.dxf> <输出.png> \
        [--tol 0.02] [--mirror-axis X] [--crop x0,y0,x1,y1] [--title T]

做什么：
  1. 只取 5 个 OCSM 几何图层（1轮廓实线层/2细线层/3中心线层/4虚线层/5剖面线层）的图元；
  2. 把图元归一化成「几何指纹」（LINE/ARC/CIRCLE/LWPOLYLINE/HATCH 的位置+尺寸+图层），
     双向匹配（生成里的每条 → 模板里找；模板里的每条 → 生成里找）；
  3. 匹配允差 tol（mm）；`--mirror-axis X` 允许「镜像副本/180°旋转副本」也算命中
     （模板自带 CAD 瑕疵：对称件常把右端画成左端的平移副本）；
  4. 画叠合图：模板=黑色细实线、生成=红色虚线，自动裁到零件包围盒。

输出：命中/多余/缺失三项计数 + 明细（缺失与多余逐条列出），便于定位画法差异。
"""
import sys
import math
import argparse

import ezdxf
from ezdxf.math import Vec2

GEOM_LAYERS = ["1轮廓实线层", "2细线层", "3中心线层", "4虚线层", "5剖面线层"]


# ── 图元 → 指纹 ────────────────────────────────────────────────────────────
def _line_fp(s, e, layer):
    (x1, y1), (x2, y2) = (s, e)
    if (x1, y1) > (x2, y2):
        x1, y1, x2, y2 = x2, y2, x1, y1
    return dict(kind="LINE", layer=layer, pts=[(x1, y1), (x2, y2)])


def _arc_fp(c, r, a0, a1, layer):
    a0, a1 = a0 % 360.0, a1 % 360.0
    return dict(kind="ARC", layer=layer, pts=[(c[0], c[1])], r=r, a0=a0, a1=a1)


def _circle_fp(c, r, layer):
    return dict(kind="CIRCLE", layer=layer, pts=[(c[0], c[1])], r=r)


def _poly_fp(pts, bulges, closed, layer):
    return dict(kind="LWPOLYLINE", layer=layer, pts=pts, bulges=list(bulges), closed=bool(closed))


def collect(dxf_path, only_geom=True):
    doc = ezdxf.readfile(dxf_path)
    msp = doc.modelspace()
    out = []
    for e in msp:
        t = e.dxftype()
        layer = e.dxf.layer
        if only_geom and layer not in GEOM_LAYERS:
            continue
        if t == "LINE":
            out.append(_line_fp((e.dxf.start.x, e.dxf.start.y), (e.dxf.end.x, e.dxf.end.y), layer))
        elif t == "ARC":
            out.append(_arc_fp((e.dxf.center.x, e.dxf.center.y), e.dxf.radius,
                               e.dxf.start_angle, e.dxf.end_angle, layer))
        elif t == "CIRCLE":
            out.append(_circle_fp((e.dxf.center.x, e.dxf.center.y), e.dxf.radius, layer))
        elif t == "LWPOLYLINE":
            pts = [(p[0], p[1]) for p in e.get_points("xy")]
            bulges = [p[2] for p in e.get_points("xyb")] if e.get_points("xyb") else []
            out.append(_poly_fp(pts, bulges, e.closed, layer))
        elif t == "POLYLINE":
            pts = [(v.dxf.location.x, v.dxf.location.y) for v in e.vertices]
            out.append(_poly_fp(pts, [v.dxf.bulge for v in e.vertices], e.is_closed, layer))
        elif t == "HATCH":
            out.append(dict(kind="HATCH", layer=layer, pts=[], pattern=e.dxf.pattern_name,
                            angle=e.dxf.pattern_angle, scale=e.dxf.pattern_scale,
                            edges=sum(len(p.edges) if hasattr(p, "edges") else 0 for p in e.paths)))
        # 其它类型（DIMENSION/TEXT/INSERT…）不参与比对
    return out


def bbox(fps):
    xs, ys = [], []
    for f in fps:
        for (x, y) in f.get("pts", []):
            xs.append(x)
            ys.append(y)
        if f.get("kind") == "ARC":
            xs.append(f["pts"][0][0] + f["r"])
            xs.append(f["pts"][0][0] - f["r"])
            ys.append(f["pts"][0][1] + f["r"])
            ys.append(f["pts"][0][1] - f["r"])
        if f.get("kind") == "CIRCLE":
            xs += [f["pts"][0][0] + f["r"], f["pts"][0][0] - f["r"]]
            ys += [f["pts"][0][1] + f["r"], f["pts"][0][1] - f["r"]]
    if not xs:
        return (0.0, 0.0, 1.0, 1.0)
    return (min(xs), min(ys), max(xs), max(ys))


# ── 匹配 ───────────────────────────────────────────────────────────────────
def _mirror(f, axis_x):
    g = dict(f)
    g["pts"] = [(2 * axis_x - x, y) for (x, y) in f["pts"]]
    if g["kind"] == "ARC":
        # 关于竖直线镜像：角度 → 180 − θ（并交换起止）
        g["a0"], g["a1"] = (180.0 - f["a1"]) % 360.0, (180.0 - f["a0"]) % 360.0
    if "bulges" in g and g["bulges"]:
        g["bulges"] = [-b for b in f["bulges"]]
    return g


def _rot180(f, cx, cy):
    g = dict(f)
    g["pts"] = [(2 * cx - x, 2 * cy - y) for (x, y) in f["pts"]]
    if g["kind"] == "ARC":
        g["a0"], g["a1"] = (f["a0"] + 180.0) % 360.0, (f["a1"] + 180.0) % 360.0
    return g


def _ang_eq(a, b, tol_deg):
    """角度等价（考虑 360 环绕与起止互换）。"""
    d = abs((a - b + 180.0) % 360.0 - 180.0)
    return d <= tol_deg


def same(f, g, tol, tol_deg=0.5):
    if f["kind"] != g["kind"] or f["layer"] != g["layer"]:
        return False
    k = f["kind"]
    if k in ("LINE", "LWPOLYLINE"):
        if len(f["pts"]) != len(g["pts"]):
            return False
        if k == "LWPOLYLINE":
            if abs(len(f.get("bulges") or []) - len(g.get("bulges") or [])) > 0:
                pass
            if (f.get("bulges") or []) != []:
                if len(f.get("bulges") or []) != len(g.get("bulges") or []):
                    return False
                if any(abs((a or 0) - (b or 0)) > 1e-4 for a, b in zip(f["bulges"], g["bulges"])):
                    return False
        # 顶点序列可整体反向/环移位
        p, q = f["pts"], g["pts"]
        n = len(p)
        for rev in (False, True):
            for shift in range(n):
                cand = [(q[(shift + i) % n]) for i in range(n)]
                if rev:
                    cand = cand[::-1]
                if all(abs(a[0] - b[0]) <= tol and abs(a[1] - b[1]) <= tol for a, b in zip(p, cand)):
                    return True
        return False
    if k in ("CIRCLE",):
        return abs(f["pts"][0][0] - g["pts"][0][0]) <= tol and abs(f["pts"][0][1] - g["pts"][0][1]) <= tol \
            and abs(f["r"] - g["r"]) <= tol
    if k == "ARC":
        return abs(f["pts"][0][0] - g["pts"][0][0]) <= tol and abs(f["pts"][0][1] - g["pts"][0][1]) <= tol \
            and abs(f["r"] - g["r"]) <= tol and _ang_eq(f["a0"], g["a0"], tol_deg) \
            and _ang_eq(f["a1"], g["a1"], tol_deg)
    if k == "HATCH":
        return f["pattern"] == g["pattern"] and abs((f["angle"] or 0) - (g["angle"] or 0)) <= 0.1 \
            and abs((f["scale"] or 0) - (g["scale"] or 0)) <= 1e-6
    return False


def describe(f):
    k = f["kind"]
    p = f.get("pts", [])
    if k == "LINE":
        return f'LINE {f["layer"]} ({p[0][0]:.4f},{p[0][1]:.4f})→({p[1][0]:.4f},{p[1][1]:.4f})'
    if k == "ARC":
        return (f'ARC {f["layer"]} c=({p[0][0]:.4f},{p[0][1]:.4f}) r={f["r"]:.4f} '
                f'{f["a0"]:.4f}°→{f["a1"]:.4f}°')
    if k == "CIRCLE":
        return f'CIRCLE {f["layer"]} c=({p[0][0]:.4f},{p[0][1]:.4f}) r={f["r"]:.4f}'
    if k == "LWPOLYLINE":
        return (f'POLY {f["layer"]} n={len(p)} '
                + " ".join(f'({x:.4f},{y:.4f})' for x, y in p)
                + (" [b]" if f.get("bulges") else ""))
    if k == "HATCH":
        return f'HATCH {f["layer"]} {f["pattern"]} ang={f["angle"]} scale={f["scale"]} edges={f["edges"]}'
    return str(f)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("template")
    ap.add_argument("generated")
    ap.add_argument("out_png")
    ap.add_argument("--tol", type=float, default=0.02)
    ap.add_argument("--mirror-axis", type=float, default=None, help="对称件的对称面 x（允许镜像副本命中）")
    ap.add_argument("--dup-shift", type=float, default=None,
                    help="模板 CAD 璵疵：右端图元常被画成左端的平移副本（传平移量，如螺母 m 的负值）。"
                         "传了它就能把「模板璵疵副本」与「真实缺失」分开统计")
    ap.add_argument("--gen-shift", type=float, default=0.0,
                    help="生成件 x 方向平移后再与模板比对（族基点与模板原点不同时用，如 120.1 平移 +l）")
    ap.add_argument("--title", default="")
    args = ap.parse_args()

    tpl = collect(args.template)
    gen = collect(args.generated)
    if args.gen_shift:
        gen = [dict(f, pts=[(x + args.gen_shift, y) for (x, y) in f["pts"]]) for f in gen]

    # 模板侧**精确重复**图元（同一造型画了两遍：CAD 导出常态）：只留一条，其余记为重复。
    tpl_uniq, tpl_dups = [], []
    for t in tpl:
        if any(same(u, t, args.tol) for u in tpl_uniq):
            tpl_dups.append(t)
        else:
            tpl_uniq.append(t)
    tpl = tpl_uniq

    # 模板侧去重（同一图元可能出现平移副本：同层同型同尺寸只留一条，副本记入 dup）
    used_t = [False] * len(tpl)
    matched_t = [False] * len(tpl)
    matched_gen = []
    extra_gen = []
    for g in gen:
        hit = -1
        cands = []
        variants = [g]
        if args.mirror_axis is not None:
            variants.append(_mirror(g, args.mirror_axis))
            variants.append(_rot180(g, args.mirror_axis, 0.0))
        for v in variants:
            for i, t in enumerate(tpl):
                if used_t[i] or not same(t, v, args.tol):
                    continue
                cands.append(i)
        if cands:
            hit = cands[0]
        if hit >= 0:
            used_t[hit] = True
            matched_t[hit] = True
            matched_gen.append(g)
        else:
            extra_gen.append(g)
    missing_t = [t for i, t in enumerate(tpl) if not matched_t[i]]

    # ── 分类：模板璵疵副本（平移副本）vs 真实缺失；生成的对称补齐 vs 真多余 ──
    def _shift(f, dx):
        g = dict(f)
        g["pts"] = [(x + dx, y) for (x, y) in f["pts"]]
        return g

    def _is_symmetric_fix(g):
        """生成的这条是否为「模板没画、按对称补齐」的那一半：生成集里存在它的镜像。"""
        if args.mirror_axis is None:
            return False
        m = _mirror(g, args.mirror_axis)
        return any(same(h, m, args.tol) for h in gen)

    dup_t = []
    if args.dup_shift is not None:
        real_t = []
        for t in missing_t:
            sh = _shift(t, -args.dup_shift)
            covered = any(same(t2, sh, args.tol) and matched_t[j] for j, t2 in enumerate(tpl))
            (dup_t if covered else real_t).append(t)
        missing_t = real_t

    sym_gen, real_gen = [], []
    for g in extra_gen:
        (sym_gen if _is_symmetric_fix(g) else real_gen).append(g)
    extra_gen = real_gen

    print(f"模板图元 {len(tpl)} 条（另 {len(tpl_dups)} 条精确重复已合并） | 生成图元 {len(gen)} 条 | 命中 {sum(matched_t)}")
    if tpl_dups:
        print(f"—— 模板重复图元（同一图元画了两遍）{len(tpl_dups)} 条")
        for f in tpl_dups:
            print("   ~ " + describe(f))
    if dup_t:
        print(f"—— 模板璵疵副本（平移 {args.dup_shift} 的重复项，不算缺失）{len(dup_t)} 条")
        for f in dup_t:
            print("   ~ " + describe(f))
    if sym_gen:
        print(f"—— 对称补齐（模板该端只画了平移副本，生成按对称补齐）{len(sym_gen)} 条")
        for f in sym_gen:
            print("   = " + describe(f))
    print(f"—— 生成多余（生成里画了、模板没有）{len(extra_gen)} 条")
    for f in extra_gen:
        print("   + " + describe(f))
    print(f"—— 模板未见命中（模板里画了、生成里没有）{len(missing_t)} 条")
    for f in missing_t:
        print("   - " + describe(f))
    print(f"结论：{'画法完全一致' if not (extra_gen or missing_t) else '有差异，逐条见上'}"
          + (f"（模板璵疵副本 {len(dup_t)} / 对称补齐 {len(sym_gen)} 已排除）" if (dup_t or sym_gen) else ""))

    # ── 叠合图 ──
    import matplotlib
    matplotlib.use("Agg")
    # 中文字体：不配的话标题/图例会变成空心方框（matplotlib 默认 DejaVu Sans 无中文字形）
    try:
        import os

        sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
        from mpl_zh import use_zh

        use_zh()
    except Exception:  # 找不到就继续（顶多是方框，不影响比对结果）
        pass
    import matplotlib.pyplot as plt
    from matplotlib.patches import Arc as MplArc, Circle as MplCircle

    x0, y0, x1, y1 = bbox(tpl)
    gx0, gy0, gx1, gy1 = bbox(gen)
    x0, y0 = min(x0, gx0), min(y0, gy0)
    x1, y1 = max(x1, gx1), max(y1, gy1)
    pad = 0.12 * max(x1 - x0, y1 - y0)
    fig, ax = plt.subplots(figsize=(9, 6), dpi=140)
    ax.set_xlim(x0 - pad, x1 + pad)
    ax.set_ylim(y0 - pad, y1 + pad)
    ax.set_aspect("equal")
    ax.grid(True, alpha=0.15, lw=0.4)

    def draw(f, color, ls, lw, alpha=1.0):
        k = f["kind"]
        p = f.get("pts", [])
        if k == "LINE":
            ax.plot([p[0][0], p[1][0]], [p[0][1], p[1][1]], color=color, ls=ls, lw=lw, alpha=alpha)
        elif k == "ARC":
            ax.add_patch(MplArc(p[0], 2 * f["r"], 2 * f["r"], angle=0,
                                theta1=f["a0"], theta2=f["a1"], color=color, ls=ls, lw=lw, alpha=alpha))
        elif k == "CIRCLE":
            ax.add_patch(MplCircle(p[0], f["r"], fill=False, color=color, ls=ls, lw=lw, alpha=alpha))
        elif k == "LWPOLYLINE":
            xs = [q[0] for q in p]
            ys = [q[1] for q in p]
            if f.get("closed") and p:
                xs, ys = xs + [p[0][0]], ys + [p[0][1]]
            ax.plot(xs, ys, color=color, ls=ls, lw=lw, alpha=alpha)
        elif k == "HATCH":
            pass

    for f in tpl:
        draw(f, "black", "-", 1.4)
    for f in gen:
        draw(f, "red", "--", 1.0, 0.85)
    ax.set_title((args.title + "  " if args.title else "")
                 + f"black=template(164580/user) / red-dash=generated | matched {sum(matched_t)}/{len(gen)}"
                 + (f" | extra {len(extra_gen)} missing {len(missing_t)}" if (extra_gen or missing_t) else " | IDENTICAL"))
    fig.tight_layout()
    fig.savefig(args.out_png)
    print(f"叠合图 → {args.out_png}")
    return 0 if (not extra_gen and not missing_t) else 1


if __name__ == "__main__":
    sys.exit(main())
