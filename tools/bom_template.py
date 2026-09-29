#!/usr/bin/env python3
"""OCSM 明细表模板生成器（用户草图 → 块化模板；计划 OCSMBOM-plan.md §28）。

依据
----
* 用户 2026-09-15 手绘《明细表示意图》(PNG) 逐像素反解（`test/明细表示意图-用户.png`）
* 实测 A3 横式图框标题栏几何：x 210..390（宽 180）、上沿 y=45（`frame/a3_landscape.dwg`）
* 用户追加要求：**表头行与内容行用不同的块**，明细表可动态加行

结构（图框坐标系，自下而上）
--------------------------
    标题栏上沿 y=45
      ┌ 表头块 OCSM_BOMHEAD（180 × 12，12mm，紧贴标题栏）
      │   上半 6mm：单件 | 总重      下半 6mm：重量（跨 23mm 居中）
      ├ 内容行块 OCSM_BOMROW（180 × 8）  ← 动态插入，插几个就长几行
      ├ 内容行块 OCSM_BOMROW（180 × 8）
      └ …（表顶线 = 最上面那个行块的顶线）

  列宽（用户草图，合计 180）：11 37 33 11 35 11 12 30
      序号 | 图号 | 名称 | 数量 | 材料 | 单件重量 | 总重 | 备注

块的实体分工（关键：线不重复画）
    OCSM_BOMHEAD  = 表头全部线条（底线 y=0、分格线 y=6、顶线 y=12、竖分界）
                    + 9 个静态列标题（TEXT，6文字层，ByLayer→绿）
    OCSM_BOMROW   = 本行**顶线** y=8 + 竖分界（0..8）+ 8 个单元格 ATTDEF
                    （行底线由下面那行/表头的顶线兼任 → 不会出现重复线）

ATTDEF 约定（同图框 a3_landscape_att.dwg 的做法）
    tag    = 中文（序号/图号/名称/数量/材料/单重/总重/备注）
    prompt = 英文（ITEM_NO/DRAWING_NO/PART_NAME/QTY/MATERIAL/UNIT_WEIGHT/TOTAL_WEIGHT/REMARK）
    text   = 空（单行 ATTDEF 由 OCS 显示 tag 占位；块内实例由 ATTRIB 带值显示）
    style  = OCSM_GB（font_file=Zhuque Fangsong，OCS 按 family 前缀解析到该 TTF，宽比 0.7）
    color  = 2（黄，用户图框「填写字段」的颜色约定）
    layer  = 6文字层；对齐 = 正中（halign=4 + valign=2，align_point = 单元格中心）

产物
----
    <out>/明细表模板.dwg|.dxf        模板本体：2 个块定义 + 1 个表头 + 2 行示例实例
    <out>/明细表模板-图框.dwg|.dxf    图框 + 明细表（审阅/截图用）
    <out>/…-读回.dxf                 经 OCS 读回的自检 DXF
    仓库副本（默认）：crates/ocs_ocsm/bom/（随插件分发）

工具链注意（2026-09-15 实测，都是坑）
----
1. **写 DWG 必须用补丁版 LibreDWG**（`~/.local/bin/dxf2dwg`）且 **`--as r2000`**：
   `--as r2004` 会把 LAYER 表颜色写坏（实测 7→255、4→255、3→0），落图整片黑白。
2. **不要用 OCS 做 DXF→DWG**：acadrust 的 DXF 读入 `read_attdef()` 只认
   1/2/3/10/40/50/62/280/101，**不读 7(样式)/41(宽比)/72,74(对齐)/11(对齐点)**。
   （OCS 读/写 **DWG** 正常 —— 只有 DXF 读入这条路会丢。）所以 `.dwg` 是交付件。
3. 块内 INSERT 的 ATTRIB 必须写在**世界坐标**（宿主把 attributes 当独立实体按自身坐标渲染）。
4. **LibreDWG 的 DXF→DWG 会丢 INSERT 上的 ATTRIB**（实测 r2000/r2004 都一样；块定义里的
   ATTDEF 不受影响）。所以：`.dxf` 里示例行带值（tag 名，便于别的 CAD 打开看效果），
   `.dwg` 里示例行的格子是空的 —— 这与用户老图 `01215_明细表` 的做法一致（块内 ATTDEF、
   插入实例不带值，值在 INSERT 时填）。插件第一期/三期自己写 ATTRIB（走宿主 API ✓）。

用法
----
    python3 tools/bom_template.py                      # 生成 + 自检 + 同步仓库
    python3 tools/bom_template.py --frame <frame.dxf>  # 指定图框 DXF（默认用 OCS 现导）
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

import ezdxf
from ezdxf.math import Vec3

try:
    from hygiene_words import GENERIC_JUNK, junk_names
except ImportError:  # 以包方式导入（tools/ 不在 sys.path 上）时兜底
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from hygiene_words import GENERIC_JUNK, junk_names

OCS = Path.home() / "dev/OpenCADStudio/target/release/OpenCADStudio"
DXF2DWG = Path.home() / ".local/bin/dxf2dwg"
PLUGIN_FRAME = Path.home() / ".config/OpenCADStudio/plugins/opencad.ocsm/frame/a3_landscape.dwg"

# ── 几何常量（实测自 a3_landscape.dwg 标题栏 + 用户草图尺寸）────────────────
TB_LEFT, TB_RIGHT, TB_TOP = 210.0, 390.0, 45.0
COL_W = [11.0, 37.0, 33.0, 11.0, 35.0, 11.0, 12.0, 30.0]
HEADER_H, ROW_H = 12.0, 8.0
TABLE_W = sum(COL_W)                       # 180 = 标题栏宽

TEXT_H, TEXT_W = 5.0, 0.7
STYLE = "OCSM_GB"
LINE_LAYER = "2细线层"      # 细线（色 4 青）
TEXT_LAYER = "6文字层"      # 文字（色 3 绿）
STATIC_COLOR = 256          # ByLayer → 随层
FIELD_COLOR = 2             # 黄（填写字段）
HEAD_BLOCK, ROW_BLOCK = "OCSM_BOMHEAD", "OCSM_BOMROW"

X = [0.0]
for w in COL_W:
    X.append(X[-1] + w)
Y_HDR_MID = HEADER_H / 2                   # 6（表头分格线）
CELLS = [
    ("序号", "ITEM_NO"),
    ("图号", "DRAWING_NO"),
    ("名称", "PART_NAME"),
    ("数量", "QTY"),
    ("材料", "MATERIAL"),
    ("单重", "UNIT_WEIGHT"),              # 列名「单件重量」，11mm 窄列用短标签（同草图数据行）
    ("总重", "TOTAL_WEIGHT"),
    ("备注", "REMARK"),
]
CELL_CX = [(X[i] + X[i + 1]) / 2 for i in range(len(COL_W))]
# 表头静态标签：(文字, 局部 x 中心, 局部 y 中心)
HEADER_LABELS = [
    ("序号", CELL_CX[0], HEADER_H / 2),
    ("图号", CELL_CX[1], HEADER_H / 2),
    ("名称", CELL_CX[2], HEADER_H / 2),
    ("数量", CELL_CX[3], HEADER_H / 2),
    ("材料", CELL_CX[4], HEADER_H / 2),
    ("单件", CELL_CX[5], Y_HDR_MID + (HEADER_H - Y_HDR_MID) / 2),   # 上半
    ("总重", CELL_CX[6], Y_HDR_MID + (HEADER_H - Y_HDR_MID) / 2),   # 上半
    ("重量", (X[5] + X[7]) / 2, HEADER_H / 4),                      # 下半（跨 23mm 居中）
    ("备注", CELL_CX[7], HEADER_H / 2),
]
# 表格线（局部坐标）：竖分界 x 列表，其中「单件|总重」那条不进表头下半格
V_ALL = X[:]
V_WEIGHT = X[6]


def anchor_points(text: str, cx: float, cy: float) -> tuple[tuple[float, float], tuple[float, float]]:
    """(insert, align_point)：LibreDWG 在两点完全相同时只留一个，故 insert 取「左基线起点」。"""
    tw = len(text) * TEXT_H * TEXT_W
    return (cx - tw / 2, cy - TEXT_H / 2), (cx, cy)


def ensure_tables(doc: ezdxf.document.Drawing) -> None:
    for name, color in ((LINE_LAYER, 4), (TEXT_LAYER, 3)):
        if name not in doc.layers:
            doc.layers.add(name=name, color=color, linetype="Continuous")
    if STYLE not in doc.styles:
        st = doc.styles.add(STYLE, font="Zhuque Fangsong")
        st.dxf.bigfont = ""
        st.dxf.width = TEXT_W
        st.dxf.height = 0.0


def _line(layout, x0, y0, x1, y1) -> None:
    layout.add_line((x0, y0), (x1, y1), dxfattribs={"layer": LINE_LAYER, "color": STATIC_COLOR})


def _text(layout, text: str, cx: float, cy: float) -> None:
    t = layout.add_text(text, height=TEXT_H, dxfattribs={
        "layer": TEXT_LAYER, "color": STATIC_COLOR, "style": STYLE, "width": TEXT_W,
        "halign": 4, "valign": 2,
    })
    t.dxf.insert, t.dxf.align_point = anchor_points(text, cx, cy)


def build_head_block(doc: ezdxf.document.Drawing):
    """表头块：12mm 高的表头（全部线条 + 9 个静态列标题），基点 = 左下角。"""
    if HEAD_BLOCK in doc.blocks:
        doc.blocks.delete_block(HEAD_BLOCK, safe=False)
    blk = doc.blocks.new(HEAD_BLOCK)
    _line(blk, X[0], 0.0, X[-1], 0.0)                       # 底线（贴标题栏）
    _line(blk, X[0], HEAD_H_LINE := HEADER_H, X[-1], HEADER_H)   # 顶线（= 首行底线）
    _line(blk, X[5], Y_HDR_MID, X[7], Y_HDR_MID)            # 单件/总重 与 重量 的分格线
    for i, x in enumerate(V_ALL):
        if i == 0 or i == len(V_ALL) - 1:
            _line(blk, x, 0.0, x, HEADER_H)
        elif x == V_WEIGHT:                                  # 单件|总重：只在上半格
            _line(blk, x, Y_HDR_MID, x, HEADER_H)
        else:
            _line(blk, x, 0.0, x, HEADER_H)
    for text, cx, cy in HEADER_LABELS:
        _text(blk, text, cx, cy)
    return blk


def build_row_block(doc: ezdxf.document.Drawing):
    """内容行块：8mm 高的数据行（顶线 + 竖分界 + 8 个 ATTDEF 单元格），基点 = 左下角。

    不画底线 —— 它由下面那行（或表头）的顶线兼任，动态加行时不会出现重复线。
    """
    if ROW_BLOCK in doc.blocks:
        doc.blocks.delete_block(ROW_BLOCK, safe=False)
    blk = doc.blocks.new(ROW_BLOCK)
    _line(blk, X[0], ROW_H, X[-1], ROW_H)                    # 顶线
    for x in V_ALL:
        _line(blk, x, 0.0, x, ROW_H)                         # 竖分界
    for (tag, prompt), cx in zip(CELLS, CELL_CX):
        ins, ap = anchor_points(tag, cx, ROW_H / 2)
        ad = blk.add_attdef(tag=tag, insert=ins, text="", dxfattribs={
            "layer": TEXT_LAYER, "color": FIELD_COLOR, "style": STYLE, "width": TEXT_W,
            "height": TEXT_H, "halign": 4, "valign": 2, "prompt": prompt,
        })
        ad.dxf.align_point = ap
    return blk


def add_sample(msp, doc: ezdxf.document.Drawing, at=(TB_LEFT, TB_TOP)) -> None:
    """示例实例（= 用户草图：1 个表头 + 2 行）——行块实例带 ATTRIB（值=列名，便于审阅）。"""
    ensure_tables(doc)
    x0, y0 = at
    msp.add_blockref(HEAD_BLOCK, (x0, y0))
    blk = doc.blocks.get(ROW_BLOCK)
    ads = {a.dxf.tag: a for a in blk.query("ATTDEF")}
    for i in range(2):
        ins_at = (x0, y0 + HEADER_H + i * ROW_H)
        ins = msp.add_blockref(ROW_BLOCK, ins_at)
        for tag, _prompt in CELLS:
            ad = ads[tag]
            shift = Vec3(ins_at[0], ins_at[1], 0.0)
            attr = ins.add_attrib(tag, tag, insert=Vec3(ad.dxf.insert) + shift,
                                  dxfattribs={"height": TEXT_H, "style": STYLE,
                                              "width": TEXT_W, "layer": TEXT_LAYER,
                                              "color": FIELD_COLOR, "halign": 4, "valign": 2})
            ap = ad.dxf.get("align_point")
            if ap is not None:
                attr.dxf.align_point = Vec3(ap) + shift


def export_single_block(doc: ezdxf.document.Drawing, block_name: str,
                        out_dxf: Path, out_dwg: Path) -> Path:
    """把某个块定义的图元放到**模型空间**单独出一份 DWG。

    插件侧 `host.import_frame_block(path, block_name)` 是「按模型的图元定义一个块」，
    所以每个块要一份自己的文件（组合模板 `明细表模板.dwg` 是给人看/审阅用的）。
    """
    d2 = ezdxf.new("R2000")
    ensure_tables(d2)
    for e in doc.blocks.get(block_name):
        d2.modelspace().add_entity(e.copy())
    d2.header["$EXTMIN"] = (X[0], 0.0, 0.0)
    d2.header["$EXTMAX"] = (X[-1], HEADER_H if block_name == HEAD_BLOCK else ROW_H, 0.0)
    write_dxf(d2, out_dxf)
    return to_dwg(out_dxf, out_dwg)


def write_dxf(doc: ezdxf.document.Drawing, path: Path) -> Path:
    doc.encoding = "gb2312"
    doc.saveas(path)
    data = path.read_bytes().replace(b"ANSI_1252", b"GB2312")
    path.write_bytes(data)
    return path


# 注意：dxf2dwg --as r2004 会把 LAYER 表颜色写坏（实测 7→255、4→255、3→0，落图整片黑白）；
# --as r2000 实测颜色正确。必须用 r2000。
DWG_VERSION = "r2000"


def to_dwg(dxf: Path, dwg: Path) -> Path:
    dwg.unlink(missing_ok=True)          # 目标已存在时 dxf2dwg 会以退出码 1 结束（但仍覆盖）
    r = subprocess.run([str(DXF2DWG), "--as", DWG_VERSION, "-o", str(dwg), str(dxf)],
                       capture_output=True, text=True)
    if not dwg.exists():
        raise SystemExit(f"dxf2dwg 失败：{r.stdout}\n{r.stderr}")
    return dwg


def frame_dxf(tmp: Path) -> Path:
    tmp.mkdir(parents=True, exist_ok=True)
    out = tmp / "frame.dxf"
    subprocess.run([str(OCS), "--export", str(PLUGIN_FRAME), str(out)], check=True,
                   stdout=subprocess.DEVNULL)
    return out


def read_back(dwg: Path, tmp: Path) -> Path:
    out = tmp / (dwg.stem + "-读回.dxf")
    subprocess.run([str(OCS), "--export", str(dwg), str(out)], check=True, stdout=subprocess.DEVNULL)
    return out


def verify(dxf: Path, strict_attribs: bool = True, encoding: str = "utf-8") -> list[str]:
    """自检：块定义 / ATTDEF 字段 / 示例实例 / 图层颜色。返回问题列表（空 = 通过）。

    strict_attribs=False（用于 DWG 读回）：LibreDWG 的 DXF→DWG 会丢 INSERT 的
    ATTRIB（实测 r2000/r2004 都一样），所以 DWG 里示例行的单元格没有值 —— 不是错误，
    只提示。（块定义里的 ATTDEF 不受影响，插件插入行时自己写 ATTRIB。）
    """
    doc = ezdxf.readfile(dxf, encoding=encoding)
    msp = doc.modelspace()
    bad: list[str] = []

    def close(a, b, tol=1e-6):
        return abs(a - b) <= tol

    # ── 图层颜色（r2004 会写坏 → 整片黑白）
    for name, want in ((LINE_LAYER, 4), (TEXT_LAYER, 3)):
        try:
            got = doc.layers.get(name).dxf.color
        except Exception:
            bad.append(f"缺图层 {name}")
            continue
        if got != want:
            bad.append(f"图层 {name} 颜色 {got} != {want}（图框/明细表会变黑白）")

    # ── 表头块
    if HEAD_BLOCK not in doc.blocks:
        bad.append(f"缺块定义 {HEAD_BLOCK}")
    else:
        hb = doc.blocks.get(HEAD_BLOCK)
        lines = [e for e in hb.query("LINE")]
        if len(lines) != 2 + len(V_ALL) + 1:      # 底/顶 + 竖分界(9) + 分格线
            bad.append(f"{HEAD_BLOCK} 线条数 {len(lines)} != {3 + len(V_ALL)}")
        texts = {e.dxf.text: e for e in hb.query("TEXT")}
        want = {t for t, _, _ in HEADER_LABELS}
        if set(texts) != want:
            bad.append(f"{HEAD_BLOCK} 文字 {sorted(texts)} != {sorted(want)}")
        for e in texts.values():
            if "\\U+" in e.dxf.text or "&#" in e.dxf.text:
                bad.append(f"表头文字转义残留：{e.dxf.text!r}")
            if not close(e.dxf.width, TEXT_W, 1e-4):
                bad.append(f"表头文字 {e.dxf.text} 宽比 {e.dxf.width}")
            if e.dxf.style != STYLE:
                bad.append(f"表头文字 {e.dxf.text} 样式 {e.dxf.style!r}")
        if len(lines) and not close(sum(abs(l.dxf.end.y - l.dxf.start.y) + abs(l.dxf.end.x - l.dxf.start.x) for l in lines), 0, 1e9):
            pass

    # ── 内容行块
    if ROW_BLOCK not in doc.blocks:
        bad.append(f"缺块定义 {ROW_BLOCK}")
    else:
        rb = doc.blocks.get(ROW_BLOCK)
        atts = list(rb.query("ATTDEF"))
        if [a.dxf.tag for a in atts] != [t for t, _ in CELLS]:
            bad.append(f"{ROW_BLOCK} tag 顺序 {[a.dxf.tag for a in atts]}")
        for a in atts:
            ap = a.dxf.get("align_point")
            if ap is None or (close(ap.x, 0.0) and close(ap.y, 0.0)):
                bad.append(f"{ROW_BLOCK} {a.dxf.tag} 锚点 (0,0)")
            if a.dxf.style != STYLE:
                bad.append(f"{ROW_BLOCK} {a.dxf.tag} 样式 {a.dxf.style!r} != {STYLE}")
            if not close(a.dxf.width, TEXT_W, 1e-4):
                bad.append(f"{ROW_BLOCK} {a.dxf.tag} 宽比 {a.dxf.width} != {TEXT_W}")
            if a.dxf.color != FIELD_COLOR:
                bad.append(f"{ROW_BLOCK} {a.dxf.tag} 颜色 {a.dxf.color} != {FIELD_COLOR}")
            if a.dxf.halign != 4 or a.dxf.get("valign", 0) != 2:
                bad.append(f"{ROW_BLOCK} {a.dxf.tag} 对齐 {a.dxf.halign}/{a.dxf.get('valign', 0)}")
            if not a.dxf.prompt:
                bad.append(f"{ROW_BLOCK} {a.dxf.tag} 缺 prompt")
            if not close(a.dxf.height, TEXT_H, 1e-4):
                bad.append(f"{ROW_BLOCK} {a.dxf.tag} 字高 {a.dxf.height}")

    # ── 示例实例：1 表头 + 2 行（行实例各 8 个 ATTRIB，带值）
    refs = [e for e in msp.query("INSERT")]
    heads = [e for e in refs if e.dxf.name == HEAD_BLOCK]
    rows = [e for e in refs if e.dxf.name == ROW_BLOCK]
    if len(heads) != 1 or len(rows) != 2:
        bad.append(f"示例实例 表头 {len(heads)} / 行 {len(rows)} != 1/2")
    if heads and not (close(heads[0].dxf.insert.x, TB_LEFT) and close(heads[0].dxf.insert.y, TB_TOP)):
        bad.append(f"表头插入点 {tuple(heads[0].dxf.insert)} != ({TB_LEFT},{TB_TOP})")
    for r in rows:
        if len(r.attribs) != len(CELLS) and strict_attribs:
            bad.append(f"行实例 ATTRIB 数 {len(r.attribs)} != {len(CELLS)}")
        for at in r.attribs:
            if not at.dxf.text:
                bad.append(f"行实例 {at.dxf.tag} ATTRIB 无值（示例行会显示空白）")
            if (at.dxf.get("align_point") is None
                    and (close(at.dxf.insert.x, 0.0) and close(at.dxf.insert.y, 0.0))):
                bad.append(f"行实例 {at.dxf.tag} ATTRIB 锚点 (0,0)")

    if doc.styles.get(STYLE).dxf.font.strip() == "":
        bad.append("OCSM_GB 样式缺 font")

    # 旧环境残留守卫（用户 2026-09-15：图框是通用图框，不能带 ZWCAD/PCCAD/私有模板命名）
    bad.extend(guard_junk(dxf))
    return bad


def guard_junk(path: Path) -> list[str]:
    """旧环境残留守卫：图框/模板/块文件都必须是通用件（用户 2026-09-15）。

    检查集合 = 本机私有词表（仓库外，`tools/hygiene_words.py`）+ 通用软件名；
    本机词表缺失 ⇒ 只剩通用词。**私有名称不进仓、也不回显**：非通用词只报 `私#N`
    （N = `~/.config/ocsm/junk-names.txt` 里的行序号），需要定位就 `tools/hygiene_scan.py --pattern N`。
    """
    bad: list[str] = []
    names = junk_names()
    raw = path.read_bytes()
    for enc in ("utf-8", "gb2312", "utf-16-le"):
        text = raw.decode(enc, errors="ignore")
        for i, needle in enumerate(names, 1):
            if needle in text:
                tag = repr(needle) if needle in GENERIC_JUNK else f"私#{i}"
                bad.append(f"检出旧环境残留串 {tag}（通用图框必须干净）")
                break
    return bad


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(Path.home() / "桌面/OCSM/bom"))
    ap.add_argument("--tmp", default="/tmp/ocsm_bom")
    ap.add_argument("--frame", default=None, help="图框 DXF（默认由 OCS 现导）")
    ap.add_argument("--repo-out", default=None,
                    help="同步一份到仓库（默认 crates/ocs_ocsm/bom/；传 '' 关闭）")
    args = ap.parse_args(argv)

    out, tmp = Path(args.out), Path(args.tmp)
    out.mkdir(parents=True, exist_ok=True)
    tmp.mkdir(parents=True, exist_ok=True)

    # ① 模板本体：2 个块定义 + 1 表头 + 2 行示例
    doc = ezdxf.new("R2000")
    ensure_tables(doc)
    build_head_block(doc)
    build_row_block(doc)
    add_sample(msp=doc.modelspace(), doc=doc)
    ext = (TB_LEFT, TB_TOP, TB_RIGHT, TB_TOP + HEADER_H + 2 * ROW_H)
    doc.header["$LIMMIN"] = (ext[0], ext[1])
    doc.header["$LIMMAX"] = (ext[2], ext[3])
    doc.header["$EXTMIN"] = (ext[0], ext[1], 0.0)
    doc.header["$EXTMAX"] = (ext[2], ext[3], 0.0)
    tpl_dxf = write_dxf(doc, out / "明细表模板.dxf")
    tpl_dwg = to_dwg(tpl_dxf, out / "明细表模板.dwg")
    # ①b 插件用单块文件（import_frame_block 按「模型空间图元」定义块）
    single = [
        export_single_block(doc, HEAD_BLOCK, out / f"{HEAD_BLOCK}.dxf", out / f"{HEAD_BLOCK}.dwg"),
        export_single_block(doc, ROW_BLOCK, out / f"{ROW_BLOCK}.dxf", out / f"{ROW_BLOCK}.dwg"),
    ]

    # ② 图框 + 明细表（审阅/截图）
    fdoc = ezdxf.readfile(Path(args.frame) if args.frame else frame_dxf(tmp), encoding="utf-8")
    ensure_tables(fdoc)
    head = fdoc.blocks.get(HEAD_BLOCK) if HEAD_BLOCK in fdoc.blocks else None
    if head is None:      # 把块定义搬进图框文档（ezdxf 不自动跨文档复制）
        src = doc.blocks.get(HEAD_BLOCK)
        new = fdoc.blocks.new(HEAD_BLOCK)
        for e in src:
            new.add_entity(e.copy())
        src = doc.blocks.get(ROW_BLOCK)
        new = fdoc.blocks.new(ROW_BLOCK)
        for e in src:
            new.add_entity(e.copy())
    add_sample(msp=fdoc.modelspace(), doc=fdoc)
    demo_dxf = write_dxf(fdoc, out / "明细表模板-图框.dxf")
    demo_dwg = to_dwg(demo_dxf, out / "明细表模板-图框.dwg")

    # ③ 仓库副本
    repo_out = Path(args.repo_out) if args.repo_out is not None else (
        Path(__file__).resolve().parent.parent / "crates/ocs_ocsm/bom")
    if str(repo_out) not in ("", "."):
        repo_out.mkdir(parents=True, exist_ok=True)
        for f in (tpl_dwg, tpl_dxf, *single):
            (repo_out / f.name).write_bytes(f.read_bytes())
        print(f"仓库副本 : {repo_out}")

    # ④ 自检：源 DXF 严格（含示例 ATTRIB），DWG 读回放宽（LibreDWG 丢 INSERT 属性）
    problems = (verify(tpl_dxf, strict_attribs=True, encoding="gb2312")
                + verify(read_back(tpl_dwg, tmp), strict_attribs=False))
    for f in single:
        doc_s = ezdxf.readfile(f.with_suffix(".dxf"), encoding="gb2312")
        bad = guard_junk(f)
        if bad:
            problems.append(f"{f.name}: 残留 {bad}")
        n_ad = sum(1 for e in doc_s.modelspace() if e.dxftype() == "ATTDEF")
        n_ln = sum(1 for e in doc_s.modelspace() if e.dxftype() == "LINE")
        want_ad = 8 if "ROW" in f.name else 0
        if n_ad != want_ad:
            problems.append(f"{f.name}: ATTDEF {n_ad} != {want_ad}")
        if n_ln < 5:
            problems.append(f"{f.name}: LINE 少（{n_ln}）")
    print(f"模板 : {tpl_dwg}")
    print(f"图框 : {demo_dwg}")
    if problems:
        print("自检失败：")
        for p in problems:
            print("  -", p)
        return 1
    print(f"自检通过：块 {HEAD_BLOCK}({HEADER_H:.0f}mm) + {ROW_BLOCK}({ROW_H:.0f}mm)、"
          f"9 静态标题、{len(CELLS)} 单元格 ATTDEF、示例 1 表头 + 2 行（各 8 ATTRIB）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
