#!/usr/bin/env python3
"""参考图 TSV 分析器（ref_dump 的输出 → 视图聚类 + 精确几何清单）。

持久化版本（放仓库里，因为 /tmp 会被清空）。
用法：
    python3 tools/ref_analyze.py <file.tsv> [--all]
"""
import sys, math
from collections import defaultdict

GEOM = {"1轮廓实线层": "轮廓", "2细线层": "细线", "3中心线层": "中心线",
        "4虚线层": "虚线", "5剖面线层": "剖面线", "0": "层0"}


def load(path):
    prims = []
    with open(path, encoding="utf-8") as f:
        next(f)
        for ln in f:
            c = ln.rstrip("\n").split("\t")
            if len(c) < 6:
                continue
            kind, layer = c[0], c[1]
            if layer not in GEOM:
                continue
            nums = []
            for v in (c[4] if len(c) > 4 else "").split():
                try:
                    nums.append(float(v))
                except ValueError:
                    pass
            prims.append(dict(kind=kind, layer=layer, color=c[2], nums=nums,
                              text=c[5] if len(c) > 5 else ""))
    return prims


def bbox(p):
    k, n = p["kind"], p["nums"]
    if k == "LINE" and len(n) >= 4:
        return (min(n[0], n[2]), min(n[1], n[3]), max(n[0], n[2]), max(n[1], n[3]))
    if k in ("CIRCLE", "ARC") and len(n) >= 3:
        return (n[0] - n[2], n[1] - n[2], n[0] + n[2], n[1] + n[2])
    if (k.startswith("LWPOLY") or k.startswith("POLY")) and len(n) >= 3:
        xs = [n[i] for i in range(0, len(n), 3)]
        ys = [n[i + 1] for i in range(1, len(n), 3)]
        return (min(xs), min(ys), max(xs), max(ys))
    return None


def cluster(prims, gap=4.0):
    boxes = [(i, p) for i, p in enumerate(prims) if bbox(p)]
    if not boxes:
        return []
    par = list(range(len(prims)))

    def find(a):
        while par[a] != a:
            par[a] = par[par[a]]
            a = par[a]
        return a

    def uni(a, b):
        ra, rb = find(a), find(b)
        if ra != rb:
            par[rb] = ra

    bs = [bbox(p) for _, p in boxes]
    for i in range(len(boxes)):
        for j in range(i + 1, len(boxes)):
            a, b = bs[i], bs[j]
            dx = max(0, max(a[0], b[0]) - min(a[2], b[2]))
            dy = max(0, max(a[1], b[1]) - min(a[3], b[3]))
            if dx < gap and dy < gap:
                uni(boxes[i][0], boxes[j][0])
    groups = defaultdict(list)
    for i, p in boxes:
        groups[find(i)].append(p)
    out = []
    for g in groups.values():
        x0 = min(bbox(p)[0] for p in g)
        y0 = min(bbox(p)[1] for p in g)
        x1 = max(bbox(p)[2] for p in g)
        y1 = max(bbox(p)[3] for p in g)
        out.append((x0, y0, x1, y1, g))
    out.sort(key=lambda t: (-len(t[4]), t[0]))
    return out


def f(v):
    r = round(v, 4)
    return f"{r:g}"


def show(prims, name):
    print(f"\n══ {name}  图元 {len(prims)}")
    lines = [p for p in prims if p["kind"] == "LINE" and len(p["nums"]) >= 4]
    arcs = [p for p in prims if p["kind"] == "ARC" and len(p["nums"]) >= 5]
    circles = [p for p in prims if p["kind"] == "CIRCLE" and len(p["nums"]) >= 3]
    polys = [p for p in prims if p["kind"].startswith(("LWPOLY", "POLY"))]
    for p in prims:
        if p["kind"] not in ("LINE", "ARC", "CIRCLE") and not p["kind"].startswith(("LWPOLY", "POLY")):
            print(f"     {p['kind']}  {p['nums']}")
    hor = [p for p in lines if abs(p["nums"][1] - p["nums"][3]) < 1e-6]
    ver = [p for p in lines if abs(p["nums"][0] - p["nums"][2]) < 1e-6]
    oth = [p for p in lines if p not in hor and p not in ver]
    if hor:
        print("  ── 水平线")
        for p in sorted(hor, key=lambda p: (-p["nums"][1], min(p["nums"][0], p["nums"][2]))):
            n = p["nums"]
            print(f"     H y={f(n[1]):>10}  x {f(min(n[0], n[2])):>10} → {f(max(n[0], n[2])):>10}   {GEOM[p['layer']]}")
    if ver:
        print("  ── 垂直线")
        for p in sorted(ver, key=lambda p: min(p["nums"][0], p["nums"][2])):
            n = p["nums"]
            print(f"     V x={f(n[0]):>10}  y {f(min(n[1], n[3])):>10} → {f(max(n[1], n[3])):>10}   {GEOM[p['layer']]}")
    if oth:
        print("  ── 斜线")
        for p in oth:
            n = p["nums"]
            ang = math.degrees(math.atan2(n[3] - n[1], n[2] - n[0]))
            ln = math.hypot(n[2] - n[0], n[3] - n[1])
            print(f"     S ({f(n[0])},{f(n[1])}) → ({f(n[2])},{f(n[3])})  ⟨{ang:.2f}° 长{ln:.4g}⟩  {GEOM[p['layer']]}")
    for p in circles:
        n = p["nums"]
        print(f"     C ({f(n[0])},{f(n[1])}) r={f(n[2])}  Ø{f(2*n[2])}  {GEOM[p['layer']]}")
    for p in arcs:
        n = p["nums"]
        print(f"     A ({f(n[0])},{f(n[1])}) r={f(n[2])}  {f(n[3])}° → {f(n[4])}° ⟨扫{n[4]-n[3]:g}°⟩  {GEOM[p['layer']]}")
    for p in polys:
        n = p["nums"]
        pts = [(n[i], n[i + 1]) for i in range(0, len(n), 3)]
        print(f"     P[{p['kind']}] {len(pts)} 点 " + " ".join(f"({f(x)},{f(y)})" for x, y in pts[:12]))


if __name__ == "__main__":
    path = sys.argv[1]
    show_all = "--all" in sys.argv
    prims = load(path)
    print(f"{path}: 几何图元 {len(prims)}")
    groups = cluster(prims)
    if show_all or len(groups) <= 1:
        show(prims, "全部")
    else:
        for i, (x0, y0, x1, y1, g) in enumerate(groups):
            show(g, f"视图{i+1}  bbox({f(x0)},{f(y0)})–({f(x1)},{f(y1)})  {f(x1-x0)}×{f(y1-y0)}")
