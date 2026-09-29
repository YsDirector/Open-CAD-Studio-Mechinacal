#!/usr/bin/env python3
"""通用图框清理：去掉旧环境残留（ZWCAD `Zwm*` / 天河 PCCAD `TH_*` 的 CLASSES 与字典对象），
顺手修掉 LAYER 表颜色损坏（LibreDWG r2004 写坏过）与失效文字样式，再写回 DWG。

背景（用户 2026-09-15）：图框是**通用图框**；源模板里曾随图纸带过来一些旧环境的样式库/明细表
定义名（ZWCAD/PCCAD 那边的命名），不能出现在新产出里。

⚠ **私有名称不进仓**：具体的旧环境/前雇主名称写在**仓库之外**的本机词表里
（`~/.config/ocsm/junk-names.txt`，或环境变量 `OCSM_JUNK_NAMES`，逗号/换行分隔），
由 `tools/hygiene_words.py` 读入；两份都没有 ⇒ 只用通用软件名，不报错、不改变清洗行为。

做法：OCS 导出源 DWG → ezdxf 读 → **只挑要的东西**建一份全新 R2000 文档：
    图层（按 OCSM 约定给正确颜色：1轮廓实线层=7、2细线层=4、6文字层=3 …）
    文字样式（Standard + OCSM_GB(font_file=Zhuque Fangsong, 宽比 0.7)；失效的 SHX 样式换成 OCSM_GB）
   文字样式 `OCSM_GB` 与插件 `text_style_defs()` 同口径：group 3 = TTF 名、宽比 0.7、
   height 3.5、annotative=1（用户 2026-09-28 清单 §P3；height/annotative 见下 ⚠）
    标注样式、模型空间图元（LINE/TEXT/ATTDEF…，属性原样保留：halign/valign/align_point/width/height）
→ 写 DXF（gb2312 字节）→ 补丁版 LibreDWG `--as r2000` 写 DWG（r2004 会写坏图层颜色）

用法：
    python3 tools/frame_clean.py            # 清理 → 出到 ~/桌面/OCSM/frame/clean/
    python3 tools/frame_clean.py --install  # 备份插件 frame/ 后覆盖安装（.bak-<时间戳>）
"""

from __future__ import annotations

import argparse
import datetime as _dt
import shutil
import subprocess
import sys
from pathlib import Path

import ezdxf

try:
    from hygiene_words import junk_names
except ImportError:  # 以包方式导入（tools/ 不在 sys.path 上）时兜底
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from hygiene_words import junk_names

OCS = Path.home() / "dev/OpenCADStudio/target/release/OpenCADStudio"
DXF2DWG = Path.home() / ".local/bin/dxf2dwg"
PLUGIN_FRAME_DIR = Path.home() / ".config/OpenCADStudio/plugins/opencad.ocsm/frame"

# 图层名 → 颜色（OCSM 约定；源文件里被写坏的按这张表恢复）
LAYER_COLOR = {
    "1轮廓实线层": 7,
    "2细线层": 4,
    "3中心线层": 1,
    "4虚线层": 6,
    "5剖面线层": 2,
    "6文字层": 3,
    "7标注层": 4,
    "8符号标注层": 7,
    "9双点划线层": 6,
    "10引导线层": 5,
    "0": 7,
    "Defpoints": 7,
}
STYLE_FIX = "OCSM_GB"          # 失效/旧 SHX 样式统一换成它
# `OCSM_GB` 必须与插件 `crates/ocs_ocsm/src/lib.rs` 的 `text_style_defs()` **同口径**
# （用户 2026-09-28 清单 §P3 定案：以插件为准，模板侧跟着改）：
#   group 3 = TTF 名（不是 SHX 占位 `Unicode`）、宽比 0.7、height = 3.5、annotative = 1。
STYLE_HEIGHT = 3.5              # 插件口径的固定字高（模板原来是 0.0 = 不定高）
DWG_VERSION = "r2000"          # 必须：r2004 会写坏 LAYER 表颜色
# 残留名检查集合 = 本机词表（仓库外，见 tools/hygiene_words.py）+ 通用软件名。
# 本机词表缺失时只剩通用词 ⇒ 不报错（词表位置与建法见 tools/hygiene_words.py）。
JUNK = junk_names()


def set_annotative(style) -> None:
    """给文字样式标 `annotative=1`（插件 `TextStyleDef.annotative = true`）。

    STYLE 表没有对应组码：DXF 与 DWG 都把它放在 XDATA `AcadAnnotative` 里
    （`AnnotativeData { 1 <flag> }`，与 opencadcodec `write_annotative_xdata` 同形状）。
    ⚠ 实测（2026-09-28）：这一步只在**清理出的 DXF** 里可靠 —— 下游 `dxf2dwg`
    （LibreDWG r2000）写 DWG 时会丢掉 STYLE 的 XDATA，装进插件目录的 DWG 读回仍是
    annotative=0。不影响出图：插件在 `TF`/`D` 前调 `ensure_text_styles()`，用
    `TextStyleDef` 把宿主里的 `OCSM_GB` 对齐到插件口径（含 annotative）—— 交付件取的是
    插件侧（见清单 §P3）。height=3.5 能过 DWG 这道，已由 `check()` 守住。
    """
    style.set_xdata("AcadAnnotative", [
        (1000, "AnnotativeData"), (1002, "{"), (1070, 1), (1070, 1), (1002, "}"),
    ])


def ocs_export(dwg: Path, dxf: Path) -> Path:
    subprocess.run([str(OCS), "--export", str(dwg), str(dxf)], check=True, stdout=subprocess.DEVNULL)
    return dxf


def build_clean(src_dxf: Path, out_dxf: Path) -> tuple[Path, dict]:
    src = ezdxf.readfile(src_dxf, encoding="utf-8")
    out = ezdxf.new("R2000")

    # 图层（颜色按约定恢复）
    for layer in src.layers:
        name = layer.dxf.name
        if name in out.layers:
            continue
        lw = out.layers.add(name=name, color=LAYER_COLOR.get(name, max(1, min(7, layer.dxf.color))))
        try:
            lw.dxf.linetype = layer.dxf.linetype
        except Exception:
            pass
        if layer.dxf.lineweight:
            lw.dxf.lineweight = layer.dxf.lineweight

    # 文字样式：保留 Standard；其余换成 OCSM_GB 参数（TTF：Zhuque Fangsong）
    used = {e.dxf.style for e in src.modelspace() if e.dxftype() in ("TEXT", "ATTDEF", "MTEXT")}
    if STYLE_FIX not in out.styles:
        st = out.styles.add(STYLE_FIX, font="Zhuque Fangsong")
        st.dxf.width = 0.7
        st.dxf.height = STYLE_HEIGHT      # 插件口径（模板原来是 0.0）
        set_annotative(st)                # 插件口径 annotative=1
    # 只保留 Standard 与 OCSM_GB：被引用的旧样式（如 SHX 的 PC_TEXTSTYLE）全部改挂 OCSM_GB
    remapped = {name: STYLE_FIX for name in used if name and name not in ("Standard", STYLE_FIX)}
    # 标注样式（通用名，原样保留）
    for dim in src.dimstyles:
        name = dim.dxf.name
        if name not in out.dimstyles:
            try:
                out.dimstyles.new(name)              # 参数走默认，够用（本图框只作参照）
            except Exception:
                pass

    # 模型空间图元：原样复制（含 halign/valign/align_point/width/height）
    n_style_fixed = 0
    for e in src.modelspace():
        if e.dxftype() in ("VIEWPORT", "SEQEND", "ATTRIB"):
            continue
        new = e.copy()
        try:
            st = new.dxf.style
            if st in remapped:
                new.dxf.style = remapped[st]
                n_style_fixed += 1
        except AttributeError:
            pass
        out.modelspace().add_entity(new)

    # 图纸范围（沿用源；OCS 里 ZOOM ALL 落点正常）
    for var in ("$LIMMIN", "$LIMMAX", "$EXTMIN", "$EXTMAX", "$INSUNITS", "$LTSCALE", "$CELTSCALE"):
        try:
            out.header[var] = src.header[var]
        except Exception:
            pass

    out.encoding = "gb2312"
    out.saveas(out_dxf)
    data = out_dxf.read_bytes().replace(b"ANSI_1252", b"GB2312")
    out_dxf.write_bytes(data)

    stats = {
        "ent": dict((t, sum(1 for e in out.modelspace() if e.dxftype() == t))
                    for t in {e.dxftype() for e in out.modelspace()}),
        "layers": [(l.dxf.name, l.dxf.color) for l in out.layers],
        "styles": [s.dxf.name for s in out.styles],
        "style_fixed": n_style_fixed,
    }
    return out_dxf, stats


def to_dwg(dxf: Path, dwg: Path) -> Path:
    dwg.unlink(missing_ok=True)
    r = subprocess.run([str(DXF2DWG), "--as", DWG_VERSION, "-o", str(dwg), str(dxf)],
                       capture_output=True, text=True)
    if not dwg.exists():
        raise SystemExit(f"dxf2dwg 失败：{r.stdout}\n{r.stderr}")
    return dwg


def read_back(dwg: Path, tmp: Path) -> ezdxf.document.Drawing:
    dxf = tmp / (dwg.stem + "-读回.dxf")
    ocs_export(dwg, dxf)
    return ezdxf.readfile(dxf, encoding="utf-8")


def check(dwg: Path, src_dxf: Path, tmp: Path) -> list[str]:
    bad: list[str] = []
    src = ezdxf.readfile(src_dxf, encoding="utf-8")
    got = read_back(dwg, tmp)
    gm, sm = got.modelspace(), src.modelspace()
    glayers = got.layers

    for t in ("LINE", "TEXT", "ATTDEF"):
        a, b = sum(1 for e in sm if e.dxftype() == t), sum(1 for e in gm if e.dxftype() == t)
        if a != b:
            bad.append(f"{t} 数 {b} != 源 {a}")
    # ATTDEF 字段（样式/宽比/对齐/锚点）必须还在。
    # 同一 tag 会重复（「图样代号」在左侧附加栏与标题栏各有一个）⇒ 按**出现顺序**逐个配对：
    # 按 tag 建 dict 只留最后一个，会把两个 ATTDEF 的 halign 互相比对，误报「halign 0 != 源 4」。
    src_att: dict[str, list] = {}
    for a in sm.query("ATTDEF"):
        src_att.setdefault(a.dxf.tag, []).append(a)
    nth: dict[str, int] = {}
    for a in gm.query("ATTDEF"):
        n = nth.get(a.dxf.tag, 0)
        nth[a.dxf.tag] = n + 1
        cands = src_att.get(a.dxf.tag) or []
        o = cands[n] if n < len(cands) else None
        if o is None:
            bad.append(f"多出 ATTDEF {a.dxf.tag}")
            continue
        # halign/valign != 0 的用 align_point 定位；= 0 的用 insert 定位（源里 align_point 本就是 0）
        needs_align = a.dxf.halign != 0 or a.dxf.get("valign", 0) != 0
        pt = a.dxf.get("align_point") if needs_align else a.dxf.insert
        if pt is None or (abs(pt.x) < 1e-9 and abs(pt.y) < 1e-9):
            bad.append(f"ATTDEF {a.dxf.tag} 定位点丢成 (0,0)（halign={a.dxf.halign}）")
        if a.dxf.halign != o.dxf.halign:
            bad.append(f"ATTDEF {a.dxf.tag} halign {a.dxf.halign} != 源 {o.dxf.halign}")
    # 图层颜色
    for l in glayers:
        want = LAYER_COLOR.get(l.dxf.name)
        if want is not None and l.dxf.color != want:
            bad.append(f"图层 {l.dxf.name} 颜色 {l.dxf.color} != {want}")
    # 文字样式 OCSM_GB 与插件同口径（清单 §P3：height=3.5 / 宽比 0.7 / TTF 名在 group 3）
    gst = next((s for s in got.styles if s.dxf.name == STYLE_FIX), None)
    if gst is None:
        bad.append(f"读回缺文字样式 {STYLE_FIX}")
    else:
        if abs(gst.dxf.height - STYLE_HEIGHT) > 1e-9:
            bad.append(f"{STYLE_FIX} height {gst.dxf.height} != 插件口径 {STYLE_HEIGHT}")
        if abs(gst.dxf.width - 0.7) > 1e-9:
            bad.append(f"{STYLE_FIX} 宽比 {gst.dxf.width} != 0.7")
        if "Zhuque" not in (gst.dxf.font or ""):
            bad.append(f"{STYLE_FIX} group 3 不是 TTF 名：{gst.dxf.font!r}")
    # 引用的样式必须存在（避免悬挂样式名）
    names = {s.dxf.name for s in got.styles}
    for e in gm:
        st = getattr(e.dxf, "style", None)
        if st and st not in names:
            bad.append(f"{e.dxftype()} 引用了不存在的样式 {st!r}")
    # 残留
    raw = dwg.read_bytes()
    for enc in ("utf-8", "gb2312", "utf-16-le"):
        t = raw.decode(enc, errors="ignore")
        for n in JUNK:
            if n in t:
                bad.append(f"DWG 里仍有残留串 {n!r}（{enc}）")
    return bad


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", default=str(PLUGIN_FRAME_DIR))
    ap.add_argument("--out", default=str(Path.home() / "桌面/OCSM/frame/clean"))
    ap.add_argument("--tmp", default="/tmp/frameclean")
    ap.add_argument("--install", action="store_true", help="备份并覆盖插件 frame/")
    args = ap.parse_args(argv)

    src_dir, out, tmp = Path(args.src), Path(args.out), Path(args.tmp)
    for d in (out, tmp):
        d.mkdir(parents=True, exist_ok=True)
    frames = sorted(p for p in src_dir.glob("*.dwg"))
    if not frames:
        raise SystemExit(f"没有 DWG：{src_dir}")

    stamp = _dt.datetime.now().strftime("%Y%m%d-%H%M%S")
    results = []
    for dwg in frames:
        src_dxf = ocs_export(dwg, tmp / (dwg.stem + "-src.dxf"))
        clean_dxf, stats = build_clean(src_dxf, out / (dwg.stem + ".dxf"))
        clean_dwg = to_dwg(clean_dxf, out / dwg.name)
        problems = check(clean_dwg, src_dxf, tmp)
        results.append((dwg.name, stats, problems))

    ok = True
    for name, stats, problems in results:
        print(f"— {name}")
        print(f"    图元 {stats['ent']}  图层 {stats['layers']}  样式 {stats['styles']}  改样式 {stats['style_fixed']}")
        if problems:
            ok = False
            for p in problems:
                print("    ✗", p)
        else:
            print("    ✓ 清理通过（图元/ATTDEF 字段/图层颜色/无残留）")

    if args.install and ok:
        src_dwg = out  # 若 --out 与源同目录，避免自我覆盖
        for dwg in frames:
            backup = dwg.with_suffix(f".bak-{stamp}")
            shutil.copy2(dwg, backup)
            shutil.copy2(out / dwg.name, dwg)
            print(f"已安装 {dwg}（备份 {backup.name}）")
    elif args.install and not ok:
        print("有检查未通过 → 不安装（先修问题）")
        return 1
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
