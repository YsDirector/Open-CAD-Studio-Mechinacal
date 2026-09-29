#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""螺栓连接**承压面压溃（Flächenpressung / 承压压应力）校核** —— OCSM 手册配套计算器。

判据：   p = F / A_p  ≤  p_G
其中     F   = 装配预紧力（按等级与规格取表值，见 F_M 表）
         A_p = 支承面承压面积（按头型/有无垫圈算，见下表）
         p_G = 被连接件材料的许用表面压力（VDI 2230 表值）

数据出处（**全部抄自公开技术资料，未自造**）：
  · 支承面积/预紧压力/F_M：Bossard《Auslegung, Konstruktion, Montage · Flächenpressung》01-2025
    （按 VDI 2230-1:2015：90% R_p0.2 利用、μ_G=0.12、ISO 273 中等（H13）通孔、室温、孔无倒角、外径足够大）
  · 许用表面压力 p_G：同资料两张表 —— VDI 2230:2015（实验值）与 VDI 2230:1986（沿用值）
  · 垫圈硬度等级匹配：同资料 ISO 887 表
判据的两种用法（对应设计的两个方向）：
  ① **正向校核**：定了螺栓/孔/材料 → 算 p，看是否 ≤ p_G（默认模式）
  ② **反向定限**：给定基体材料与承压面 → 算**允许的最大预紧力 F_allow 与最大拧紧力矩 T_max**，
     并告诉你"谁在控制"（基体 or 螺栓）。螺栓强度通常高于基体，**力矩上限常由基体决定**。
     `--allow` 模式即此（见下）。

用法：
  python3 crush_check.py M8 8.8 hex_ab                 # 默认：中等孔、钢 Q235、无垫圈
  python3 crush_check.py M8 8.8 hex_ab --allow mat=6061 dh=10   # 反算：允许 F 与 T，控制方
  python3 crush_check.py M8 8.8 hex_ab mat=6061 dh=10
  python3 crush_check.py M8 8.8 hex_ab mat=6061 dh=10 washer=97.1   # 加 GB/T 97.1 平垫
  python3 crush_check.py M8 8.8 hex_c  dh=10 mat=Q235   # C 级头（无垫圈面，六角全支承面）
  python3 crush_check.py M8 8.8 socket mat=6061         # 内六角圆柱头（ISO 4762）
  python3 crush_check.py M8 8.8 hex_ab nut=41           # 螺母侧（C 级 41：无垫圈面）
  python3 crush_check.py --tables                       # 打印全部表（可贴进手册）
  python3 crush_check.py --selftest                     # 自检：与出处表逐格对数

约定：长度 mm、力 N、应力 MPa(=N/mm²)。**本工具是"判据 + 查表"，不是强度设计全流程**：
横向载荷、疲劳、预紧力控制精度、温度等还需按 VDI 2230 / 你们手册另算。
"""
from __future__ import annotations
import json
import math
import sys
from pathlib import Path

# ── 1. 规格几何（来源表：六角头 DIN 931/933 = ISO 4014/4017；内六角 ISO 4762）──
# 字段：(d, s/dK, dw, dh_中等H13, Ap, As, p88, p109, p129)
HEX = {
    4:  (7, 5.9, 4.5, 11.4, 8.78, 385, 568, 665),
    5:  (8, 6.9, 5.5, 13.6, 14.2, 528, 777, 909),
    6:  (10, 8.9, 6.6, 28.0, 20.1, 364, 532, 625),
    8:  (13, 11.6, 9.0, 42.1, 36.6, 442, 649, 761),
    10: (16, 14.63, 11.0, 73.1, 58.0, 405, 594, 695),   # ISO 4014 的 s=16
    12: (18, 16.63, 13.5, 74.1, 84.3, 580, 853, 999),
    14: (21, 19.64, 15.5, 114.3, 115.0, 517, 759, 888),
    16: (24, 22.5, 17.5, 157.1, 157.0, 515, 756, 885),
    18: (27, 25.3, 20.0, 188.6, 192.0, 541, 769, 901),
    20: (30, 28.2, 22.0, 244.4, 245.0, 532, 761, 888),
    22: (32, 30.0, 24.0, 254.5, 303.0, 637, 908, 1065),
    24: (36, 33.6, 26.0, 355.8, 353.0, 528, 750, 880),
    27: (41, 38.0, 30.0, 427.3, 459.0, 576, 821, 960),
    30: (46, 42.7, 33.0, 576.7, 561.0, 520, 740, 865),
}
# 内六角圆柱头 ISO 4762：(dK, dw, dh, Ap, As)——预紧力同尺寸同等级（头型不改变预紧力）
SOCKET = {
    4:  (7.0, 6.53, 4.5, 17.6, 8.78),
    5:  (8.5, 8.03, 5.5, 26.9, 14.2),
    6:  (10.0, 9.38, 6.6, 34.9, 20.1),
    8:  (13.0, 12.33, 9.0, 55.8, 36.6),
    10: (16.0, 15.33, 11.0, 89.5, 58.0),
    12: (18.0, 17.23, 13.5, 90.0, 84.3),
    14: (21.0, 20.17, 15.5, 130.8, 115.0),
    16: (24.0, 23.17, 17.5, 181.1, 157.0),
    18: (27.0, 25.87, 20.0, 211.5, 192.0),
    20: (30.0, 28.87, 22.0, 274.5, 245.0),
    22: (33.0, 31.81, 24.0, 342.3, 303.0),
    24: (36.0, 34.81, 26.0, 420.8, 353.0),
    27: (40.0, 38.61, 30.0, 464.0, 459.0),
    30: (45.0, 43.61, 33.0, 638.4, 561.0),
}
# 六角头**C 级**（GB/T 5780 / ISO 4016）与 **C 级螺母**（GB/T 41 / ISO 4034）：**无垫圈面**
# → 支承面 = 六角全端面；用对边宽 s 算 A_hex = (√3/2)·s²，再减通孔。
# 来源：GB/T 5780 / GB/T 41 的 s（本仓库 tables/partsHexBoltC.json、partsNutC41.json 实测值）
HEX_C_S = {4: 7.0, 5: 8.0, 6: 10.0, 8: 13.0, 10: 16.0, 12: 18.0, 14: 21.0,
           16: 24.0, 18: 27.0, 20: 30.0, 22: 32.0, 24: 36.0, 27: 41.0, 30: 46.0}
# 螺母（1 型 GB/T 6170 / 薄型 GB/T 6172.1）**有垫圈面** dw ≈ 六角头 A/B 级同值；C 级 41 无垫圈面。
# 键 = 规格，P = 粗牙螺距（用于算 A_s；ISO 724）
PITCH = {4: 0.7, 5: 0.8, 6: 1.0, 8: 1.25, 10: 1.5, 12: 1.75, 14: 2.0,
         16: 2.0, 18: 2.5, 20: 2.5, 22: 2.5, 24: 3.0, 27: 3.0, 30: 3.5}
# 平垫圈 GB/T 97.1 / ISO 7089 的 d2（外径）——本仓库 tables/partsWasher971.json 的值
WASHER_971 = {4: 9.0, 5: 10.0, 6: 12.0, 8: 16.0, 10: 20.0, 12: 24.0, 14: 28.0,
              16: 30.0, 18: 34.0, 20: 37.0, 22: 39.0, 24: 44.0, 27: 50.0, 30: 56.0}

# ── 2. 许用表面压力 p_G（MPa）——VDI 2230 两套表 ──
# (材料代号, 国标近似对应, Rm_min, pG_2015; pG_1986 用 None 表示该表无此项)
# 国标对应按"Rm 相当"近似，**不是标准等同**，用于工程初选。
PG = [
    # 牌号(EN/旧),              Rm,  pG(2015实验值), pG(1986沿用值), 国标近似
    ("S235JRG1 (USt 37-2)", 340, 490, 260, "Q235 / Q235B"),
    ("E295 (St 50-2)", 470, 710, 420, "Q275 / 20"),
    ("S355JO (St 52-3U)", 490, 760, None, "Q355 / Q345"),
    ("Cq 45", 700, 770, 700, "45（调质）"),
    ("34 CrMo 4", 900, 1170, 850, "35CrMo / 42CrMo 类"),
    ("34 CrNiMo 6", 1100, 1430, None, "34CrNiMo6 / 40CrNiMo"),
    ("38 MnSi-VS 5-BY", 900, 990, None, "非调质钢（近似）"),
    ("16 MnCr 5", 1000, 1300, None, "20CrMnTi 类（渗碳钢）"),
    ("X4 CrNi 18 12", 500, 630, 210, "06Cr19Ni10(304)"),
    ("X5 CrNiMo 17 12 2", 530, 630, None, "06Cr17Ni12Mo2(316)"),
    ("X6 NiCrTiMoVB 25-15-2", 960, 1200, None, "高温合金（近似）"),
    ("NiCr20TiAl", 1000, 1000, None, "镍基高温合金"),
    ("GJL-250 (GG-25)", 250, 850, 800, "HT250"),
    ("GJS-400 (GGG-40)", 400, 600, 480, "QT400-18"),
    ("GJS-500 (GGG-50)", 500, 750, None, "QT500-7"),
    ("GJS-600 (GGG-60)", 600, 900, None, "QT600-3"),
    ("GG 15", 150, None, 600, "HT150"),
    ("GG 35", 350, None, 900, "HT350"),
    ("GG 40", 400, None, 1100, "HT400（少见）"),
    ("AlMgSi 1 F31 (AW-6082)", 290, 360, None, "6061-T6（近似）"),
    ("AlMgSi 1 F28", 260, 325, None, "6061-T6（时效不足）"),
    ("AlMg4.5Mn F27 (AW-5083)", 260, 325, None, "5083-O/H（近似）"),
    ("AlZnMgCu 1.5 (AW-7075)", 540, 540, None, "7075-T6（近似）"),
    ("AlZnMgCu 0,5", 450, None, 370, "硬铝（近似）"),
    ("DG MgAl 9", 300, None, 220, "镁合金（压铸）"),
    ("GK MgAl 9", 200, None, 140, "镁合金（砂型/重力）"),
    ("Titan, unlegiert", 500, None, 300, "工业纯钛 TA1/TA2"),
]
# 修正系数（同资料）：
CHAMFER_BONUS = 1.20        # 孔口倒角：许用值可提高约 20%（资料另处写 25%，取保守 20%）
POWER_TOOL_PENALTY = 0.75   # 机动（电动/气动）拧紧：许用值可能小 25%
# 垫圈硬度等级与螺栓等级的匹配（ISO 887 表）：
WASHER_MATRIX = {
    # 螺栓等级: {垫圈硬度 HV: 是否允许}
    "6.8":  {100: True, 200: True, 300: True},
    "8.8":  {100: False, 200: True, 300: True},
    "9.8":  {100: False, 200: False, 300: True},
    "10.9": {100: False, 200: False, 300: True},
    "12.9": {100: False, 200: False, 300: False},
    "A2-50": {100: True, 140: True, 200: True},
    "A2-70": {100: False, 140: True, 200: True},
    "A2-80": {100: False, 140: False, 200: True},
}
R_P02 = {"4.8": 340, "6.8": 480, "8.8": 640, "10.9": 940, "12.9": 1100}
# 拧紧力矩系数 K（T = K·F·d 的工程估算；细算按 VDI 2230 的 μ_G/μ_K 分解）
K_FACTOR = {
    "dry": 0.20,        # 干态、无润滑（钢/镀锌，默认）
    "zinc": 0.18,       # 镀锌（略滑）
    "oiled": 0.15,      # 轻油润滑
    "mos2": 0.14,       # MoS2 润滑
    "paste": 0.12,      # 不锈钢防咬膏/含固体润滑
}


def area_stress(d: float, p: float) -> float:
    """A_s = π/4·((d2+d3)/2)²（ISO 724 / VDI 2230）。d2 = d − 0.6495P，d3 = d − 1.2269P。"""
    d2 = d - 0.6495 * p
    d3 = d - 1.2269 * p
    return math.pi / 4.0 * ((d2 + d3) / 2.0) ** 2


def contact_area(d: int, dh: float, head: str = "hex_ab", washer: float | None = None,
                 nut_family: str | None = None) -> tuple[float, str]:
    """支承面承压面积 A_p（mm²）与说明。"""
    if washer is not None:
        d2 = WASHER_971.get(d, washer)
        return math.pi / 4.0 * (d2 * d2 - dh * dh), f"平垫 Ø{d2}（97.1）"
    if head == "hex_ab":                 # 六角头 A/B 级（GB/T 5782/5783）带垫圈面 dw
        dw = HEX[d][1]
        return math.pi / 4.0 * (dw * dw - dh * dh), f"六角头垫圈面 dw={dw}"
    if head == "hex_c":                  # C 级（GB/T 5780）无垫圈面 → 六角全端面
        s = HEX_C_S[d]
        return math.sqrt(3) / 2.0 * s * s - math.pi / 4.0 * dh * dh, f"C 级六角全端面 s={s}"
    if head == "socket":
        dw = SOCKET[d][1]
        return math.pi / 4.0 * (dw * dw - dh * dh), f"内六角头支承面 dw={dw}"
    if head == "nut_c41":
        s = HEX_C_S[d]
        return math.sqrt(3) / 2.0 * s * s - math.pi / 4.0 * dh * dh, f"C 级螺母全端面 s={s}"
    if head == "nut_6170":               # 1型/薄型螺母（6170/6172.1）带垫圈面
        dw = HEX[d][1]
        return math.pi / 4.0 * (dw * dw - dh * dh), f"螺母垫圈面 dw={dw}"
    raise SystemExit(f"未知头型：{head}")


def preload(d: int, grade: str) -> float:
    """装配预紧力 F_M（N）：优先用出处表值（p×A_p），否则按 0.79·A_s·R_p0.2（含扭转，μ_G=0.12）。"""
    if d in HEX:
        _s, _dw, _dh, ap, _as_, p88, p109, p129 = HEX[d]
        table = {"8.8": p88, "10.9": p109, "12.9": p129}
        if grade in table:
            return table[grade] * ap
    rp = R_P02.get(grade)
    if rp is None:
        raise SystemExit(f"等级 {grade} 没有 R_p0.2 数据；可用：{sorted(R_P02)}")
    return 0.79 * area_stress(d, PITCH[d]) * rp


def find_material(key: str):
    """按牌号/国标关键词找材料行。"""
    key_u = key.strip().upper()
    for name, rm, pg15, pg86, cn in PG:
        hay = f"{name} {cn}".upper()
        if key_u in hay:
            return name, rm, pg15, pg86, cn
    return None


def recommend_materials(required: float, limit: int = 6):
    """按所需 p_G 反查可用材料。"""
    out = []
    for name, rm, pg15, pg86, cn in PG:
        pg = pg15 if pg15 is not None else pg86
        if pg is not None and pg >= required:
            src = "2015" if pg15 is not None else "1986"
            out.append((name, cn, pg, src))
    out.sort(key=lambda x: x[2])
    return out[:limit]


def check(d: int, grade: str, mat: str, head: str = "hex_ab", dh: float | None = None,
          washer: float | None = None, chamfer: bool = False, power: bool = False) -> dict:
    if d not in HEX:
        raise SystemExit(f"没收录 M{d}（可用：{sorted(HEX)}）")
    dh = dh if dh is not None else HEX[d][2]
    m = find_material(mat)
    if m is None:
        raise SystemExit(f"没收录材料 {mat!r}；用 --tables 看清单")
    name, rm, pg15, pg86, cn = m
    pg = pg15 if pg15 is not None else pg86
    f = preload(d, grade)
    ap, how = contact_area(d, dh, head, washer)
    p = f / ap
    allow = pg
    notes = []
    if chamfer:
        allow *= CHAMFER_BONUS
        notes.append(f"孔口倒角 ×{CHAMFER_BONUS}")
    if power:
        allow *= POWER_TOOL_PENALTY
        notes.append(f"机动拧紧 ×{POWER_TOOL_PENALTY}")
    return {
        "d": d, "grade": grade, "material": name, "material_cn": cn, "pg": pg, "pg_src": "2015" if pg15 is not None else "1986",
        "allow": allow, "dh": dh, "f": f, "ap": ap, "area_how": how, "p": p,
        "ok": p <= allow, "notes": notes, "rm": rm,
    }


def fmt(v: float, nd: int = 1) -> str:
    return f"{v:,.{nd}f}"


def print_report(r: dict) -> None:
    print(f"── 承压（压溃）校核：M{r['d']} {r['grade']} ／ {r['material']}（≈{r['material_cn']}）")
    print(f"   通孔 Ø{r['dh']:g}（ISO 273 中等 = GB/T 5277 中等装配）")
    print(f"   支承面：{r['area_how']} → A_p = {fmt(r['ap'], 1)} mm²")
    print(f"   预紧力 F_M = {fmt(r['f'], 0)} N（出处表值，90% R_p0.2、μ_G=0.12）")
    print(f"   压应力 p = F/A_p = {fmt(r['p'], 0)} N/mm²")
    print(f"   许用 p_G = {fmt(r['pg'], 0)} N/mm²（VDI 2230:{r['pg_src']}）"
          + (f"，修正后 {fmt(r['allow'], 0)}（{'; '.join(r['notes'])}）" if r["notes"] else ""))
    verdict = "✅ 通过" if r["ok"] else f"❌ 超限 {fmt(r['p'] / r['allow'], 2)} 倍"
    print(f"   → {verdict}")
    if not r["ok"]:
        if r["d"] in WASHER_971:
            ap2, how2 = contact_area(r["d"], r["dh"], "hex_c" if "C 级" in r["area_how"] else "hex_ab", WASHER_971[r["d"]])
            p2 = r["f"] / ap2
            print(f"   ↳ 加平垫 Ø{WASHER_971[r['d']]}：A_p = {fmt(ap2, 1)} mm² → p = {fmt(p2, 0)} N/mm² "
                  f"→ {'✅ 通过' if p2 <= r['allow'] else '❌ 仍超限'}（{how2}）")
        want = r["p"] * 1.05
        cands = recommend_materials(want)
        if cands:
            print("   ↳ 换材料（p_G ≥ 当前 p 5% 余量）：" + "；".join(f"{c[1]}({fmt(c[2], 0)})" for c in cands))
        print("   ↳ 其它降压力手段：法兰螺栓/法兰螺母、孔口倒角（+20%）、通孔改较精系列（ISO 273）、降低预紧力矩（必要时）")


def allow_load(d: int, grade: str, mat: str, head: str = "hex_ab", dh: float | None = None,
               washer: float | None = None, chamfer: bool = False, power: bool = False,
               k_key: str = "dry") -> dict:
    """反向定限：基体允许的最大预紧力、最大拧紧力矩，以及"谁在控制"。"""
    r = check(d, grade, mat, head, dh, washer, chamfer, power)
    f_base = r["allow"] * r["ap"]          # 基体（承压）允许的预紧力
    f_bolt = r["f"]                        # 螺栓本身按等级/规格的装配预紧力（表值）
    gov = "基体（承压面）" if f_base < f_bolt else "螺栓（等级/规格）"
    f_allow = min(f_base, f_bolt)
    k = K_FACTOR.get(k_key, K_FACTOR["dry"])
    t_allow = k * f_allow * d / 1000.0     # N·m
    return dict(r, f_base=f_base, f_bolt=f_bolt, governing=gov, f_allow=f_allow,
                k=k, k_key=k_key, t_allow=t_allow,
                base_ratio=f_base / f_bolt)


def print_allow(a: dict) -> None:
    r = a
    print(f"── 反向定限：M{r['d']} {r['grade']} ／ {r['material']}（≈{r['material_cn']}）"
          f"／ 通孔 Ø{r['dh']:g} ／ {r['area_how']}")
    print(f"   基体许用 p_G = {fmt(r['allow'], 0)} N/mm²"
          + (f"（修正：{'; '.join(r['notes'])}）" if r["notes"] else ""))
    print(f"   承压面积 A_p = {fmt(r['ap'], 1)} mm²")
    print(f"   → 基体允许预紧力 F_allow(基体) = {fmt(r['f_base'], 0)} N")
    print(f"   → 螺栓本身预紧力 F_M(螺栓)      = {fmt(r['f_bolt'], 0)} N（等级 {r['grade']}）")
    print(f"   **控制方：{r['governing']}**"
          f"（基体能力 / 螺栓能力 = {r['base_ratio'] * 100:.0f}%）")
    print(f"   → 允许预紧力 F = {fmt(r['f_allow'], 0)} N"
          f"  → 最大拧紧力矩 T ≈ K·F·d = {r['t_allow']:.1f} N·m（K={r['k']}，{r['k_key']}）")
    if r["f_base"] < r["f_bolt"]:
        print(f"   ↳ 基体控制（差 {100 - r['base_ratio'] * 100:.0f}%）→ 打不到螺栓的设计预紧力："
              "加平垫/法兰件、增孔数、降等级或换基体材料；防松与防滑移改靠机械方式（弹垫/双螺母/锁固胶）。")
    else:
        print("   ↳ 螺栓控制 → 预紧力由等级与规格决定，力矩按上值取（并留拧紧方法散差）。")


def print_tables() -> None:
    print("## 表 A 六角头（GB/T 5782/5783 类，带垫圈面）支承面与满预紧压应力（VDI 2230:2015 基准）")
    print("| d | s | dw | dh(中等) | A_p mm² | A_s mm² | p(8.8) | p(10.9) | p(12.9) | F_M(8.8) N |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    for d in sorted(HEX):
        s, dw, dh, ap, a_s, p88, p109, p129 = HEX[d]
        print(f"| M{d} | {s:g} | {dw:g} | {dh:g} | {ap:g} | {a_s:g} | {p88} | {p109} | {p129} | {p88 * ap:,.0f} |")
    print()
    print("## 表 B 内六角圆柱头 ISO 4762 支承面（预紧力同上表）")
    print("| d | dK | dw | dh | A_p mm² | p(8.8) | p(10.9) | p(12.9) |")
    print("|---|---|---|---|---|---|---|---|")
    for d in sorted(SOCKET):
        dk, dw, dh, ap, _as_ = SOCKET[d]
        f88 = HEX[d][5] * HEX[d][3]
        f109 = HEX[d][6] * HEX[d][3]
        f129 = HEX[d][7] * HEX[d][3]
        print(f"| M{d} | {dk:g} | {dw:g} | {dh:g} | {ap:g} | {f88 / ap:,.0f} | {f109 / ap:,.0f} | {f129 / ap:,.0f} |")
    print()
    print("## 表 C 许用表面压力 p_G（N/mm²）")
    print("| 材料（EN / 旧牌号） | 国标近似对应 | Rm min | p_G (VDI 2230:2015) | p_G (VDI 2230:1986) |")
    print("|---|---|---|---|---|")
    for name, rm, pg15, pg86, cn in PG:
        print(f"| {name} | {cn} | {rm:g} | {pg15 if pg15 is not None else '—'} | {pg86 if pg86 is not None else '—'} |")
    print()
    print("## 表 E C 级（GB/T 5780 头 / GB/T 41 螺母：**无垫圈面**，六角全端面）压应力（8.8 级）")
    print("| d | s | A_p(六角全端面−孔)，中等孔 | 中等孔 p | 粗装配孔(近似) | 粗装配 p(近似) |")
    print("|---|---|---|---|---|---|")
    for d in sorted(HEX_C_S):
        s = HEX_C_S[d]
        dh_m = HEX[d][2]
        dh_f = {"4": 5.0, "5": 6.0, "6": 7.0, "8": 10.0, "10": 12.0, "12": 14.0, "14": 16.0,
                "16": 18.0, "18": 20.0, "20": 24.0, "22": 26.0, "24": 28.0, "27": 30.0, "30": 33.0}[str(d)]
        if d not in HEX:
            continue
        f = HEX[d][5] * HEX[d][3]
        ap_m = math.sqrt(3) / 2 * s * s - math.pi / 4 * dh_m * dh_m
        ap_f = math.sqrt(3) / 2 * s * s - math.pi / 4 * dh_f * dh_f
        if abs(ap_f - ap_m) < 1e-9:
            print(f"| M{d} | {s:g} | {ap_m:.1f} | {f / ap_m:,.0f} | — | — |")
        else:
            print(f"| M{d} | {s:g} | {ap_m:.1f} | {f / ap_m:,.0f} | {ap_f:.1f} | {f / ap_f:,.0f} |")
    print()
    print("> 中等孔 = ISO 273 中等（H13）= GB/T 5277 中等装配（出处表值 ✓）；**粗装配列是按 d+2 估的近似值，"
          "未逐档核对**——用 `dh=<实际孔径>` 复算，以你们手册的 GB/T 5277 表为准。")
    print()
    print("## 表 D 平垫圈（GB/T 97.1）对承压面积/压应力的改善（M8 8.8、中等孔）")
    print("| d | 无垫圈 A_p | 加平垫 A_p（Ød2） | 压力降幅 | 无垫圈 p | 加垫圈 p |")
    print("|---|---|---|---|---|---|")
    for d in sorted(WASHER_971):
        dh = HEX[d][2]
        ap1, _ = contact_area(d, dh, "hex_ab")
        ap2, _ = contact_area(d, dh, "hex_ab", WASHER_971[d])
        f = preload(d, "8.8")
        print(f"| M{d} | {ap1:.1f} | {ap2:.1f}（Ø{WASHER_971[d]:g}） | {(1 - ap1 / ap2) * 100:.0f}% | {f / ap1:,.0f} | {f / ap2:,.0f} |")


def selftest() -> int:
    bad = 0
    for d, (s, dw, dh, ap, a_s, p88, p109, p129) in HEX.items():
        calc_ap = math.pi / 4.0 * (dw * dw - dh * dh)
        if abs(calc_ap - ap) > 0.15:
            print(f"  ✗ M{d} A_p：表 {ap} vs 算 {calc_ap:.2f}")
            bad += 1
        calc_as = area_stress(d, PITCH[d])
        if abs(calc_as - a_s) / a_s > 0.03:
            print(f"  ✗ M{d} A_s：表 {a_s} vs 算 {calc_as:.2f}")
            bad += 1
    for d, (dk, dw, dh, ap, a_s) in SOCKET.items():
        calc_ap = math.pi / 4.0 * (dw * dw - dh * dh)
        if abs(calc_ap - ap) > 0.15:
            print(f"  ✗(内六角) M{d} A_p：表 {ap} vs 算 {calc_ap:.2f}")
            bad += 1
    # 压力一致性：p = F/A_p，F 用同表反推
    f88 = HEX[8][5] * HEX[8][3]
    cases = [("M8 8.8 / Q235", 442, check(8, "8.8", "S235", "hex_ab")["p"], 40),
             ("M8 8.8 / 6061", 442, check(8, "8.8", "AlMgSi", "hex_ab")["p"], 40),
             ("M8 8.8 加平垫", 135.4, check(8, "8.8", "6061", "hex_ab", washer=16.0)["p"], 2),
             ("M8 8.8 C级头(六角全端面,孔9)", 18600 / (math.sqrt(3) / 2 * 169 - math.pi / 4 * 81),
              check(8, "8.8", "Q235", "hex_c")["p"], 2)]
    for label, want, got, tol in cases:
        if abs(want - got) > tol:
            print(f"  ✗ {label}：期望 {want:.1f} vs 得到 {got:.1f}")
            bad += 1
    print(f"自检：{'全部通过 ✅' if bad == 0 else f'{bad} 处不一致 ❌'}"
          f"（对照 Bossard/VDI 2230 出处表：A_p 由 π/4(dw²−dh²) 复算、A_s 由螺距复算、p 由 F/A_p 复算）")
    return 1 if bad else 0


def main(argv: list[str]) -> int:
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__)
        return 0
    if argv[0] == "--tables":
        print_tables()
        return 0
    if argv[0] == "--selftest":
        return selftest()
    args = [a for a in argv[1:] if a != "--allow"]
    opts = {}
    for a in args:
        if "=" in a:
            k, v = a.split("=", 1)
            opts[k.lower()] = v
    spec = argv[0].upper().lstrip("M")
    if not spec.isdigit():
        raise SystemExit(f"规格写法：M8 或 8（现在给了 {argv[0]!r}）")
    d = int(spec)
    grade = args[0] if args and "=" not in args[0] else "8.8"
    head = args[1] if len(args) > 1 and "=" not in args[1] else opts.pop("head", "hex_ab")
    mat = opts.pop("mat", "S235")
    dh = float(opts.pop("dh", HEX[d][2]))
    washer = opts.pop("washer", None)
    washer = float(WASHER_971.get(d, 0)) if washer in ("1", "yes", "97.1", "true") else (float(washer) if washer else None)
    if "nut" in opts:
        head = {"41": "nut_c41", "c41": "nut_c41", "6170": "nut_6170", "61721": "nut_6170"}.get(opts.pop("nut"), "nut_c41")
    chamfer = opts.pop("chamfer", "0") in ("1", "yes", "true")
    power = opts.pop("power", "0") in ("1", "yes", "true")
    if opts.pop("allow", None) is not None or "--allow" in argv:
        k_key = opts.pop("k", "dry")
        a = allow_load(d, grade, mat, head, dh, washer, chamfer, power, k_key)
        print_allow(a)
        return 0 if a["f_base"] >= a["f_bolt"] else 2   # 2 = 基体控制（力矩上限不由螺栓定）
    r = check(d, grade, mat, head, dh, washer, chamfer=chamfer, power=power)
    print_report(r)
    for k in list(opts):
        print(f"   （忽略未识别参数 {k}）")
    return 0 if r["ok"] else 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
