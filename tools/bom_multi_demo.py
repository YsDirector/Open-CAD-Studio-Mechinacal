#!/usr/bin/env python3
"""明细表「满列续列」布局演示/生成器（对照用户 2026-09-15 演示图）。

规则（用户演示）：明细表自下而上填写；一列写满（每列行数可定义）后，**在左边另起一列**，
续列自带一个表头，序号继续往上接。首列底边贴标题栏（y=45），续列底边落到图框内下边线上方
约 5 mm（比首列低，见演示图）。

几何（与模板一致）：块宽 180 mm；表头 12 mm；数据行 8 mm；列内线 0/11/48/81/92/127/138/150/180。
续列 x = 首列 x − 180。

用法：
    python3 tools/bom_multi_demo.py                  # 41 件：首列 26 行、续列 15 行（复刻演示图）
    python3 tools/bom_multi_demo.py --parts 60 --per-col 26
产物：~/桌面/OCSM/bom/明细表-续列演示.dwg
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

import ezdxf
from ezdxf.enums import TextEntityAlignment

OCS = Path.home() / "dev/OpenCADStudio/target/release/OpenCADStudio"
DXF2DWG = Path.home() / ".local/bin/dxf2dwg"
TEMPLATE = Path.home() / "dev/OpenCADStudio/crates/ocs_ocsm/bom/明细表模板.dwg"
FRAME = Path.home() / "桌面/OCSM/frame/clean/a3_landscape.dwg"
OUT_DIR = Path.home() / "桌面/OCSM/bom"

W = 180.0          # 块宽
HEAD_H = 12.0      # 表头高
ROW_H = 8.0        # 数据行高
FIRST_X0 = 210.0   # 首列左边界
FIRST_Y0 = 45.0    # 首列底边（贴标题栏）
CONT_Y0 = 5.0      # 续列底边（图框内下边线 y=0 上方 5 mm）
SEQ_CX = 5.5       # 序号格中心（局部）
LAYERS = {"0": 7, "1轮廓实线层": 7, "2细线层": 4, "6文字层": 3, "Defpoints": 7}


def export(dwg: Path, dxf: Path) -> Path:
    subprocess.run([str(OCS), "--export", str(dwg), str(dxf)], check=True, stdout=subprocess.DEVNULL)
    return dxf


def copy_block(src: ezdxf.document.Drawing, dst: ezdxf.document.Drawing, name: str) -> None:
    if name in dst.blocks:
        return
    blk = dst.blocks.new(name)
    for e in src.blocks.get(name):
        blk.add_entity(e.copy())


def build(frame_dxf: Path, tpl_dxf: Path, out_dxf: Path, parts: int, per_col: int) -> dict:
    frame = ezdxf.readfile(frame_dxf, encoding="utf-8")
    tpl = ezdxf.readfile(tpl_dxf, encoding="utf-8")
    doc = ezdxf.new("R2000")

    for layer in frame.layers:
        name = layer.dxf.name
        if name not in doc.layers:
            doc.layers.add(name=name, color=LAYERS.get(name, max(1, min(7, layer.dxf.color))))
    if "OCSM_GB" not in doc.styles:
        st = doc.styles.add("OCSM_GB", font="Zhuque Fangsong")
        st.dxf.width = 0.7
    for e in frame.modelspace():
        doc.modelspace().add_entity(e.copy())
    copy_block(tpl, doc, "OCSM_BOMHEAD")
    copy_block(tpl, doc, "OCSM_BOMROW")

    # 分列：首列 per_col 行，其余依次放到左边各列
    cols: list[list[int]] = []
    left = parts
    while left > 0:
        take = min(per_col, left)
        cols.append(list(range(parts - left + 1, parts - left + take + 1)))
        left -= take

    msp = doc.modelspace()
    geo = []
    for ci, seqs in enumerate(cols):
        x0 = FIRST_X0 - W * ci
        y0 = FIRST_Y0 if ci == 0 else CONT_Y0
        msp.add_blockref("OCSM_BOMHEAD", (x0, y0), dxfattribs={"layer": "2细线层"})
        for k, seq in enumerate(seqs):                      # k=0 → 最下面那一行
            yr = y0 + HEAD_H + ROW_H * k
            msp.add_blockref("OCSM_BOMROW", (x0, yr), dxfattribs={"layer": "2细线层"})
            # 注意：LibreDWG r2000 会把 halign=4 归一成 halign=1 且把点写进 group 10、
            # group 11 清零 → 对齐文本会跑回原点。所以这里用 halign=0（左基线）+ 自己算居中偏移。
            txt = str(seq)
            w = 0.7 * 3.5 * len(txt)                       # OCSM_GB 宽比 0.7
            t = msp.add_text(txt, dxfattribs={"layer": "6文字层", "style": "OCSM_GB",
                                              "height": 3.5, "color": 2})
            t.dxf.insert = (x0 + SEQ_CX - w / 2, yr + ROW_H / 2 - 3.5 * 0.36, 0)
        geo.append((ci, x0, y0, len(seqs)))

    doc.encoding = "gb2312"
    doc.saveas(out_dxf)
    out_dxf.write_bytes(out_dxf.read_bytes().replace(b"ANSI_1252", b"GB2312"))
    return {"cols": geo, "parts": parts}


def to_dwg(dxf: Path, dwg: Path) -> Path:
    dwg.unlink(missing_ok=True)
    r = subprocess.run([str(DXF2DWG), "--as", "r2000", "-o", str(dwg), str(dxf)],
                       capture_output=True, text=True)
    if not dwg.exists():
        raise SystemExit(f"dxf2dwg 失败：{r.stdout}\n{r.stderr}")
    return dwg


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--parts", type=int, default=41)
    ap.add_argument("--per-col", type=int, default=26)
    ap.add_argument("--out", default=str(OUT_DIR / "明细表-续列演示.dwg"))
    ap.add_argument("--tmp", default="/tmp/bomdemo")
    args = ap.parse_args(argv)

    tmp = Path(args.tmp)
    tmp.mkdir(parents=True, exist_ok=True)
    frame_dxf = export(FRAME, tmp / "frame.dxf")
    tpl_dxf = export(TEMPLATE, tmp / "tpl.dxf")
    out_dxf = tmp / "多列.dxf"
    info = build(frame_dxf, tpl_dxf, out_dxf, args.parts, args.per_col)
    dwg = to_dwg(out_dxf, Path(args.out))

    chk = ezdxf.readfile(export(dwg, tmp / "读回.dxf"), encoding="utf-8")
    m = chk.modelspace()
    ins = [e for e in m if e.dxftype() == "INSERT"]
    heads = [e for e in ins if e.dxf.name == "OCSM_BOMHEAD"]
    rows = [e for e in ins if e.dxf.name == "OCSM_BOMROW"]
    print(f"列 {len(info['cols'])} 个：", [(f"第{ci+1}列", f"x={x0}", f"底 y={y0}", f"{n} 行")
                                          for ci, x0, y0, n in info["cols"]])
    print(f"块引用：表头 {len(heads)}、行 {len(rows)}  文字 {sum(1 for e in m if e.dxftype()=='TEXT')}")
    print(f"→ {dwg}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
