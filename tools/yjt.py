#!/usr/bin/env python3
"""易紧通 164580.com 标准数据挖掘工具（tsc 反爬 + 规格尺寸弹窗解析）。

为什么要它：易紧通的标准页 `info_<id>.html` 有**全量规格尺寸网格**，逐规格弹窗
`biaozhundetail/model_pic.php?x=<规格id>&sid=<标准id>` 有**该规格的尺寸值 + 公称长度系列**。
站内检索带 `tsc` 安全码（防自动抓取），要先取一次拿 tsc 再带着 tsc + cookie 重放。

子命令
------
  search <关键词> [--limit N]       站内标准检索 → TSV: info_id / 代号 / 名称
  info <info_id> [--out 目录]       标准页 → JSON（sid / 规格列表 / 尺寸网格 / 公称长度）
  spec <x> <sid> [--l 长度]         规格弹窗 → JSON（标签路径→值 / 公称长度系列 / 螺纹长 b）
  mine <info_id> <out.json>         逐规格抓弹窗 → 完整数据表 JSON（附 --sleep 秒）
  fetch <url> <out>                 带 UA/cookie 的原始抓取（排查用）

例
--
  python3 tools/yjt.py search "GB/T 70.1"
  python3 tools/yjt.py info 30447
  python3 tools/yjt.py mine 30447 /tmp/yjt/gb70.json
"""
from __future__ import annotations

import json
import re
import sys
import time
from pathlib import Path

import requests
from bs4 import BeautifulSoup

UA = (
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
)
BASE = "https://www.164580.com"
CACHE = Path("/tmp/yjt_cache")
TIMEOUT = 40


def session() -> requests.Session:
    s = requests.Session()
    s.headers.update(
        {
            "User-Agent": UA,
            "Accept-Language": "zh-CN,zh;q=0.9",
            "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        }
    )
    return s


def get(s: requests.Session, url: str, *, referer: str | None = None, cache_key: str | None = None) -> str:
    """带磁盘缓存的 GET（同一 URL 不重复打站）。"""
    CACHE.mkdir(parents=True, exist_ok=True)
    if cache_key is None:
        cache_key = re.sub(r"[^0-9A-Za-z]+", "_", url)[-120:]
    path = CACHE / f"{cache_key}.html"
    if path.exists() and path.stat().st_size > 500:
        return path.read_text(encoding="utf-8", errors="ignore")
    headers = {"Referer": referer} if referer else {}
    r = s.get(url, headers=headers, timeout=TIMEOUT)
    r.raise_for_status()
    r.encoding = "utf-8"
    path.write_text(r.text, encoding="utf-8")
    return r.text


def text_of(node) -> str:
    return re.sub(r"\s+", " ", node.get_text(" ", strip=True)).strip()


# ── 站内检索（tsc 反爬：先取 tsc，再带 tsc + cookie 重放）──────────────────────

def search(keyword: str, limit: int = 20) -> list[dict]:
    s = session()
    url = f"{BASE}/biaozhun/?keyword={requests.utils.quote(keyword)}"
    html = s.get(url, timeout=TIMEOUT).text
    m = re.search(r'name="tsc" value="(\d+)"', html)
    if m:
        url2 = f"{url}&tsc={m.group(1)}"
        html = s.get(url2, headers={"Referer": f"{BASE}/biaozhun/"}, timeout=TIMEOUT).text
    if "链接已过期" in html:
        raise SystemExit("检索仍被 tsc 拦住（页面提示链接已过期）")
    soup = BeautifulSoup(html, "lxml")
    out: list[dict] = []
    seen: set[str] = set()
    for a in soup.find_all("a", href=True):
        mm = re.search(r"/info_(\d+)\.html", a["href"])
        if not mm:
            continue
        iid = mm.group(1)
        if iid in seen:
            continue
        # 规格行：info 链接的祖先 <tr> 里通常有 代号 + 名称
        row = a.find_parent("tr")
        code = name = ""
        if row:
            cells = [text_of(td) for td in row.find_all("td")]
            cells = [c for c in cells if c]
            # 去掉纯图片/空列；代号形如 GB/T 70.1 - 2008 / GB 893-86
            for c in cells:
                if not code and re.search(r"(GB|ISO|DIN|ANSI|JB|QJ|HB|YJT)\s*/?\s*[Tt]?\s*[\d.]+", c):
                    code = c
                elif not name and len(c) > 1 and not c.startswith("http"):
                    if c != code:
                        name = c
        seen.add(iid)
        out.append({"info_id": iid, "code": code, "name": name, "url": f"{BASE}/info_{iid}.html"})
        if len(out) >= limit:
            break
    return out


# ── 标准页：sid + 规格列表 + 尺寸网格 ────────────────────────────────────────

def parse_info(html: str, info_id: str) -> dict:
    soup = BeautifulSoup(html, "lxml")
    title = text_of(soup.title).split("-")[0].strip() if soup.title else ""
    # 规格列表：(x, sid, 规格文字) —— 出现在 winopen(... model_pic.php?x=..&sid=..) 的 <a> 里
    specs: list[dict] = []
    seen: set[str] = set()
    for a in soup.find_all("a", href=True):
        onclick = a.get("onclick", "") or ""
        mm = re.search(r"model_pic\.php\?x=(\d+)&sid=(\d+)", onclick)
        if not mm:
            continue
        x, sid = mm.group(1), mm.group(2)
        if x in seen:
            continue
        seen.add(x)
        label = text_of(a)
        specs.append({"x": x, "sid": sid, "label": label})
    sid = specs[0]["sid"] if specs else ""
    # 公称长度（页面上那条 “公称长度L” 列表）
    lengths: list[str] = []
    box = soup.find(id="cl")
    if box:
        lengths = [text_of(p) for p in box.find_all("p")]
    # 尺寸网格：外层 gg_table 的每个 <tr> = 一行（左列维度名 + 右列各规格值）
    grid: list[dict] = []
    for table in soup.find_all("table", class_="gg_table"):
        for tr in table.find_all("tr", recursive=False):
            cells = tr.find_all("td", recursive=False)
            if len(cells) < 2:
                continue
            dim = text_of(cells[0])
            if not dim or len(dim) > 40:
                continue
            values: list[str] = []
            for td in cells[1:]:
                sub = td.find("table")
                if sub:
                    values += [text_of(td2) for td2 in sub.find_all("td")]
                else:
                    values.append(text_of(td))
            values = [v for v in values if v not in ("", "-")]
            if values:
                grid.append({"dim": dim, "values": values})
    return {
        "info_id": info_id,
        "title": title,
        "sid": sid,
        "specs": specs,
        "nominal_lengths": lengths,
        "grid": grid,
        "source": f"{BASE}/info_{info_id}.html",
    }


# ── 规格弹窗：标签路径 → 值 + 长度系列 + 螺纹长 b ─────────────────────────────

def leaf_labels(table) -> list[str]:
    """把带 rowspan/colspan 的标签表展开成每行一条标签路径（`dk/最大值/光滑头部`）。"""
    out: list[str] = []
    active: list[list] = []  # [文本, 还跨几行]
    for tr in table.find_all("tr", recursive=False):
        tokens = [t for t, _ in active]
        active = [[t, r - 1] for t, r in active]
        for td in tr.find_all(["td", "th"], recursive=False):
            txt = text_of(td)
            tokens.append(txt)
            rs = int(td.get("rowspan") or 1)
            if rs > 1:
                active.append([txt, rs - 1])
        active = [[t, r] for t, r in active if r > 0]
        if tokens:
            out.append("/".join(t for t in tokens if t))
    return out


def parse_spec(html: str) -> dict:
    """弹窗 = 外层 gg_table：行0 `螺纹尺寸 d | <规格>`、行1 `标签表 | 数值表`、行2 重量、行3 螺纹长 b。"""
    soup = BeautifulSoup(html, "lxml")
    body = soup.body or soup
    table = body.find("table", class_="gg_table") or body.find("table")
    spec_label = ""
    dims: list[dict] = []
    b_text = ""
    if table is not None:
        rows = table.find_all("tr", recursive=False)
        if rows:
            c0 = rows[0].find_all(["td", "th"], recursive=False)
            if len(c0) >= 2:
                spec_label = text_of(c0[-1])
        if len(rows) >= 2:
            cells = rows[1].find_all(["td", "th"], recursive=False)
            if len(cells) >= 2:
                lt, vt = cells[0].find("table"), cells[1].find("table")
                labels = leaf_labels(lt) if lt is not None else []
                values = [text_of(tr) for tr in (vt.find_all("tr", recursive=False) if vt is not None else [])]
                values = [v for v in values if v]
                for i, lab in enumerate(labels):
                    dims.append({"label": lab, "value": values[i] if i < len(values) else ""})
                if len(values) != len(labels):
                    dims.append({"warning": f"标签 {len(labels)} 条 vs 数值 {len(values)} 条，请人工核对"})
        if len(rows) >= 4:
            b_text = text_of(rows[3])
    lengths: list[str] = []
    box = soup.find(id="cl")
    if box:
        lengths = [text_of(p) for p in box.find_all("p")]
    return {
        "spec": spec_label,
        "dims": dims,
        "n_dims": len([d for d in dims if "label" in d]),
        "thread_len_row": b_text,
        "lengths": lengths,
        "source_html_len": len(html),
    }


# ── 子命令 ──────────────────────────────────────────────────────────────────

def cmd_search(args: list[str]) -> None:
    limit = 20
    if "--limit" in args:
        i = args.index("--limit")
        limit = int(args[i + 1])
    kw = " ".join(a for a in args if not a.startswith("--") and a != str(limit))
    rows = search(kw, limit)
    for r in rows:
        print(f"{r['info_id']}\t{r['code']}\t{r['name']}")


def cmd_info(args: list[str]) -> None:
    info_id = args[0]
    out_dir = None
    if "--out" in args:
        out_dir = Path(args[args.index("--out") + 1])
    s = session()
    html = get(s, f"{BASE}/info_{info_id}.html", cache_key=f"info_{info_id}")
    data = parse_info(html, info_id)
    if out_dir:
        out_dir.mkdir(parents=True, exist_ok=True)
        (out_dir / f"info_{info_id}.json").write_text(json.dumps(data, ensure_ascii=False, indent=1), "utf-8")
    print(json.dumps(data, ensure_ascii=False, indent=1))


def cmd_spec(args: list[str]) -> None:
    x, sid = args[0], args[1]
    l = ""
    if "--l" in args:
        l = args[args.index("--l") + 1]
    s = session()
    url = f"{BASE}/biaozhundetail/model_pic.php?x={x}&sid={sid}&l={l}"
    html = get(s, url, referer=f"{BASE}/info_.html", cache_key=f"spec_{x}_{sid}_{l}")
    print(json.dumps(parse_spec(html), ensure_ascii=False, indent=1))


def cmd_mine(args: list[str]) -> None:
    info_id, out = args[0], Path(args[1])
    sleep = 0.0
    if "--sleep" in args:
        sleep = float(args[args.index("--sleep") + 1])
    s = session()
    info = parse_info(get(s, f"{BASE}/info_{info_id}.html", cache_key=f"info_{info_id}"), info_id)
    results = []
    for n, sp in enumerate(info["specs"], 1):
        url = f"{BASE}/biaozhundetail/model_pic.php?x={sp['x']}&sid={sp['sid']}&l="
        try:
            html = get(s, url, referer=f"{BASE}/info_{info_id}.html", cache_key=f"spec_{sp['x']}_{sp['sid']}_")
            d = parse_spec(html)
        except Exception as exc:  # noqa: BLE001
            d = {"error": str(exc)}
        d.update({"x": sp["x"], "label": sp["label"]})
        results.append(d)
        if sleep:
            time.sleep(sleep)
        print(f"[{n}/{len(info['specs'])}] {sp['label']} x={sp['x']} dims={d.get('n_dims')}", file=sys.stderr)
    data = {
        "info_id": info_id,
        "title": info["title"],
        "sid": info["sid"],
        "nominal_lengths": info["nominal_lengths"],
        "grid": info["grid"],
        "specs": results,
        "source": info["source"],
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(data, ensure_ascii=False, indent=1), "utf-8")
    print(f"→ {out}（{len(results)} 规格）")


def cmd_fetch(args: list[str]) -> None:
    url, out = args[0], Path(args[1])
    s = session()
    html = get(s, url, cache_key=None)
    out.write_text(html, "utf-8")
    print(f"→ {out} ({len(html)} bytes)")


def main() -> None:
    if len(sys.argv) < 3:
        print(__doc__)
        raise SystemExit(2)
    cmd, args = sys.argv[1], sys.argv[2:]
    {"search": cmd_search, "info": cmd_info, "spec": cmd_spec, "mine": cmd_mine, "fetch": cmd_fetch}[cmd](args)


if __name__ == "__main__":
    main()
