#!/usr/bin/env python3
"""用 OCS 自带的自动化端口驱动「正在运行的」OpenCADStudio GUI（本机 OCSM 验证用）。

OpenCADStudio 每个实例会在 `~/.config/OpenCADStudio/automation/<session>.json`
写一份描述文件（session_id / pid / port / token），并在 127.0.0.1:<port> 上
一条连接一条 JSON 请求（协议 1）。本脚本读描述文件 → 发请求 → 打印应答。

用法:
    python3 tools/ocs_drive.py state                    # 列出实例（含 pid/文档）
    python3 tools/ocs_drive.py run "OCSM" --pid 1234     # 在实例里执行命令行
    python3 tools/ocs_drive.py run "ZOOM ALL"
    python3 tools/ocs_drive.py capture /tmp/x.png        # 抓应用窗口 PNG
    python3 tools/ocs_drive.py save                       # 保存当前文档
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import socket
import sys
import uuid
from pathlib import Path

DESC_DIR = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")) / "OpenCADStudio" / "automation"


def descriptors() -> list[dict]:
    out = []
    for path in sorted(glob.glob(str(DESC_DIR / "*.json"))):
        try:
            d = json.loads(Path(path).read_text())
        except Exception:
            continue
        d["_file"] = path
        out.append(d)
    return out


def alive(pid: int) -> bool:
    return Path(f"/proc/{pid}").exists()


def exchange(desc: dict, request: dict, timeout: float = 20.0) -> dict:
    req = dict(request)
    req.setdefault("protocol", 1)
    req.setdefault("request_id", uuid.uuid4().hex)   # 变更类 op 必须带唯一 id
    req["session_id"] = desc["session_id"]
    req["token"] = desc["token"]
    with socket.create_connection(("127.0.0.1", desc["port"]), timeout=timeout) as s:
        s.sendall((json.dumps(req, ensure_ascii=False) + "\n").encode("utf-8"))
        s.shutdown(socket.SHUT_WR)
        buf = b""
        while not buf.endswith(b"\n"):
            chunk = s.recv(1 << 20)
            if not chunk:
                break
            buf += chunk
    return json.loads(buf.decode("utf-8", errors="replace")) if buf else {}


def probe(desc: dict) -> dict | None:
    """只认「描述文件 + 端口 + session_id 三者对得上」的实例。

    描述文件在进程退出后不会自动删（实测残留几十份），pid 还可能被别的进程
    复用，所以必须真连一次 `state` 验活，不能只看 /proc。
    """
    try:
        st = exchange(desc, {"op": "state"}, timeout=2.0)
    except Exception:
        return None
    if st.get("ok") and st.get("session_id") == desc["session_id"]:
        return st
    return None


def pick(pid: int | None, quiet: bool = False) -> tuple[dict, dict]:
    cands = [d for d in descriptors() if alive(d["pid"])]
    if pid is not None:
        cands = [d for d in cands if d["pid"] == pid]
    live: list[tuple[dict, dict]] = []
    for d in cands:
        st = probe(d)
        if st:
            live.append((d, st))
    if not live:
        raise SystemExit("没有匹配的 OCS 自动化实例（先启动 OpenCADStudio GUI）")
    if len(live) > 1 and not quiet:
        print(f"（{len(live)} 个实例，取 pid={live[0][0]['pid']}；可用 --pid 指定）", file=sys.stderr)
    return live[0]


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("op", choices=["state", "list", "run", "capture", "save", "open", "input", "raw"])
    ap.add_argument("arg", nargs="?")
    ap.add_argument("--pid", type=int, default=None)
    ap.add_argument("--json", action="store_true", help="原样打印应答 JSON")
    ap.add_argument("--doc", type=int, default=None, help="document_id（变更类 op 必填）")
    args = ap.parse_args(argv)

    if args.op == "list":
        for d in descriptors():
            print(f"pid={d['pid']:<8} alive={alive(d['pid'])} port={d['port']} session={d['session_id'][:12]}…")
        return 0

    if args.op == "state":
        for d in descriptors():
            if not alive(d["pid"]):
                continue
            st = probe(d)
            if not st:
                continue
            docs = st.get("documents", [])
            print(f"pid={d['pid']} mode={st.get('mode')} enabled={st.get('enabled')} "
                  f"doc={[x['title'] for x in docs]}")
        return 0

    desc, st = pick(args.pid)
    doc_id = args.doc
    if doc_id is None and args.op not in ("state", "list", "raw"):
        docs = [d for d in st.get("documents", []) if not d.get("start")]
        if len(docs) == 1:
            doc_id = docs[0]["id"]
        else:
            print("多个文档，请用 --doc 指定：", [(d["id"], d["path"]) for d in st.get("documents", [])],
                  file=sys.stderr)
    if args.op == "run":
        req = {"op": "run", "cmd": args.arg}
    elif args.op == "capture":
        req = {"op": "capture", "path": args.arg}
    elif args.op == "save":
        req = {"op": "save"} if not args.arg else {"op": "save", "path": args.arg}
    elif args.op == "open":
        req = {"op": "open", "path": args.arg}
    elif args.op == "input":
        # 交互命令喂字符串（如 INSERT 的块名/插入点、属性的 8 个值）；空串 = 空格
        req = {"op": "input", "kind": "token", "text": " " if args.arg == "" else args.arg}
    else:
        req = json.loads(args.arg)
    if doc_id is not None:
        req["document_id"] = doc_id
    reply = exchange(desc, req)
    print(json.dumps(reply, ensure_ascii=False, indent=2) if args.json else
          json.dumps(reply, ensure_ascii=False))
    return 0 if reply.get("ok", True) else 1


if __name__ == "__main__":
    sys.exit(main())
