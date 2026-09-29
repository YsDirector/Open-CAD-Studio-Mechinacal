#!/usr/bin/env python3
"""OCS MCP 单会话驱动：**在一个 MCP 客户端里**跑交互命令的 start+input。

为什么必须单会话：宿主 `src/app/control/mod.rs` 的自动化门要求
`control.owner == Some((tab, client_id))` 才允许 `input`，而 MCP 服务器给**每个客户端实例**
随机生成 `client_id`（`src/mcp.rs:326`）。用 `mcporter call ...` 这类"一次调用一个新连接"的方式驱动，
每次 client_id 都不同 → 交互命令的下一步永远返回 `command_busy`。
本脚本开一个 `OpenCADStudio --mcp` 子进程并保持它，因此 `run/start` 之后的 `input` 能通过。
（另一个坑：插件命令要在 `on_text_input` 里返回 `CommandStep::Ignored`，宿主才会把非关键字
token 当十六进制句柄喂进实体拾取步骤 —— 那种写法可以直接 `run "ZX 62"` 一把过。）

用法：
    python3 /tmp/ocs_mcp.py <<'PY'
    newdoc
    run CENTERMARK
    input entity DD 315,350,0
    cancel
    run D2G
    query line 3中心线层
    PY
"""
import json, os, subprocess, sys, threading, queue, time

# 宿主二进制：优先 $OCS_BIN，否则按「本脚本所在仓库根/target/release」推导
_OCS_DEFAULT = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                            "target", "release", "OpenCADStudio")
OCS = os.environ.get("OCS_BIN", _OCS_DEFAULT)
ENV = dict(os.environ)
ENV.setdefault("WAYLAND_DISPLAY", "wayland-0")
ENV.setdefault("XDG_RUNTIME_DIR", "/run/user/1000")

proc = subprocess.Popen([OCS, "--mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                        stderr=subprocess.DEVNULL, text=True, bufsize=1, env=ENV)
q = queue.Queue()
def reader():
    for line in proc.stdout:
        q.put(line.strip())
threading.Thread(target=reader, daemon=True).start()

_id = [int(__import__('time').time()) % 1_000_000]  # 跨进程唯一，避免 request_id_reused
def send(method, params=None, notify=False):
    msg = {"jsonrpc": "2.0", "method": method}
    if params is not None:
        msg["params"] = params
    if not notify:
        _id[0] += 1
        msg["id"] = _id[0]
    proc.stdin.write(json.dumps(msg) + "\n")
    proc.stdin.flush()
    if notify:
        return None
    want = msg["id"]
    deadline = time.time() + 60
    while time.time() < deadline:
        try:
            line = q.get(timeout=max(0.1, deadline - time.time()))
        except queue.Empty:
            break
        if not line:
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            continue
        if obj.get("id") == want:
            return obj
    return {"error": {"message": "timeout"}}

def call_tool(name, args):
    r = send("tools/call", {"name": name, "arguments": args})
    if "error" in r:
        raise SystemExit(f"{name} 失败：{r['error']}")
    # MCP 返回 content[0].text = JSON 字符串
    content = r.get("result", {}).get("content") or []
    if content and content[0].get("type") == "text":
        try:
            return json.loads(content[0]["text"])
        except json.JSONDecodeError:
            return content[0]["text"]
    return r.get("result")

send("initialize", {"protocolVersion": "2024-11-05", "capabilities": {},
                    "clientInfo": {"name": "ocs-session", "version": "1"}})
send("notifications/initialized", None, notify=True)

sess = call_tool("ocs_sessions", {})
rows = sess["result"] if isinstance(sess, dict) and "result" in sess else sess
if isinstance(rows, dict):
    rows = [rows]
sid = rows[0]["session_id"]
doc = rows[0]["document_id"]
print(f"# session={sid} doc={doc} rev={rows[0]['revision']}")

def show(d, tag=""):
    if not isinstance(d, dict):
        print(f"  {tag} ← {str(d)[:120]}")
        return {}
    st = d.get("state") or {}
    cmd = st.get("command") or {}
    print(f"  {tag} status={d.get('status')} code={d.get('code')} err={d.get('error')} "
          f"rev={st.get('revision')} geo={st.get('geometry_revision')} "
          f"prompt={(cmd.get('prompt') or '')[:50]!r} sel={st.get('selection')}")
    return st

for raw in sys.stdin.read().splitlines():
    line = raw.strip()
    if not line or line.startswith("#"):
        continue
    op, *rest = line.split(None, 1)
    arg = rest[0] if rest else ""
    if op == "run":
        d = call_tool("ocs_execute", {"ocs_session_id": sid,
                                      "request": {"request_id": f"mcp-{_id[0]}", "op": "run", "cmd": arg}})
        show(d, f"run {arg!r}")
    elif op == "input":
        parts = arg.split()
        kind = parts[0]
        req = {"request_id": f"mcp-{_id[0]}", "op": "input", "kind": kind}
        if kind == "entity":
            req["handle"] = parts[1]
            if len(parts) > 2:
                req["point"] = [float(x) for x in parts[2].split(",")]
        elif kind == "text" or kind == "token":
            req["text"] = " ".join(parts[1:])
        elif kind == "point":
            # `input point 40,60` → 世界坐标点（ZOOM 窗口角点、拾取点等）
            req["point"] = [float(x) for x in parts[1].split(",")] + [0.0]
            req["space"] = "wcs"
        elif kind == "enter":
            pass  # 回车（kind=enter 本身即语义）
        d = call_tool("ocs_execute", {"ocs_session_id": sid, "request": req})
        show(d, f"input {arg!r}")
    elif op == "newdoc":
        # `newdoc`：新建一张空白图纸（不然新起的会话里没有任何文档，run 会报 no_document）
        d = call_tool("ocs_execute", {"ocs_session_id": sid,
                                      "request": {"request_id": f"mcp-{_id[0]}", "op": "new"}})
        show(d, "newdoc")
    elif op == "cancel":
        d = call_tool("ocs_execute", {"ocs_session_id": sid,
                                      "request": {"request_id": f"mcp-{_id[0]}", "op": "cancel"}})
        show(d, "cancel")
    elif op == "query":
        parts = arg.split()
        params = {}
        if parts:
            params["type"] = parts[0]
        if len(parts) > 1:
            params["layer"] = parts[1]
        params["detail"] = "geometry"
        d = call_tool("ocs_read", {"ocs_session_id": sid, "op": "query", "parameters": params})
        for e in (d.get("entities") or []):
            print("   ", e.get("handle"), e.get("type"), e.get("start"), e.get("end"),
                  e.get("layer"), e.get("name"))
        print(f"    ({len(d.get('entities') or [])} 个)")
    elif op == "select":
        parts = arg.split()
        if parts and parts[0] == "type":
            req = {"request_id": f"mcp-{_id[0]}", "op": "select", "type": parts[1]}
        else:
            req = {"request_id": f"mcp-{_id[0]}", "op": "select", "handles": parts}
        d = call_tool("ocs_execute", {"ocs_session_id": sid, "request": req})
        show(d, f"select {arg!r}")
    else:
        print(f"  ? 未知指令 {line!r}")

proc.terminate()
