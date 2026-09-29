#!/usr/bin/env python3
"""guide_gui 免重启预览代理：让改完的 guide_gui.html 立即可用，不必重启 OCS。

背景：插件把 GUI 用 include_str! 编进 .so，运行中的 OCS 只会吐旧版 HTML。
本工具在本地起一个小 HTTP 服务：
  - GET `/` 或 `/guide.html*` → 直接回**仓库里最新的** crates/ocs_ocsm/src/guide_gui.html
  - 其余 GET/POST → 原样转发给正在运行的插件 HTTP 服务（/api/* 等同源代理）

用法：
  python3 tools/guide_gui_proxy.py [上游端口] [本端口]
  默认上游 23751（插件 guide_server），本端口 23752。

然后浏览器打开 http://127.0.0.1:23752/guide.html?handle=<真实标注handle>
OCS 下次重启后插件自带的就是新前端，本代理即可停掉。
"""
import http.server
import pathlib
import sys
import urllib.error
import urllib.request

UP = int(sys.argv[1]) if len(sys.argv) > 1 else 23751
PORT = int(sys.argv[2]) if len(sys.argv) > 2 else 23752
HTML = pathlib.Path(__file__).resolve().parents[1] / "crates/ocs_ocsm/src/guide_gui.html"
UP_BASE = f"http://127.0.0.1:{UP}"

HOP = {"connection", "keep-alive", "transfer-encoding", "content-encoding", "content-length"}


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def _proxy(self, body=None):
        url = UP_BASE + self.path
        req = urllib.request.Request(url, data=body, method=self.command)
        for k, v in self.headers.items():
            if k.lower() not in ("host", "connection", "content-length"):
                req.add_header(k, v)
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                data = r.read()
                self.send_response(r.status)
                for k, v in r.headers.items():
                    if k.lower() not in HOP:
                        self.send_header(k, v)
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)
        except urllib.error.HTTPError as e:
            data = e.read()
            self.send_response(e.code)
            self.send_header("Content-Type", e.headers.get("Content-Type", "text/plain; charset=utf-8"))
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
        except Exception as e:  # noqa: BLE001
            msg = f"proxy error: {e}".encode()
            self.send_response(502)
            self.send_header("Content-Type", "text/plain; charset=utf-8")
            self.send_header("Content-Length", str(len(msg)))
            self.end_headers()
            self.wfile.write(msg)

    def do_GET(self):  # noqa: N802
        if self.path == "/" or self.path.startswith("/guide.html"):
            b = HTML.read_bytes()
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(b)))
            self.end_headers()
            self.wfile.write(b)
        else:
            self._proxy()

    def do_POST(self):  # noqa: N802
        n = int(self.headers.get("Content-Length") or 0)
        self._proxy(self.rfile.read(n) if n else None)

    def log_message(self, *a):  # 静默
        pass


if __name__ == "__main__":
    srv = http.server.ThreadingHTTPServer(("127.0.0.1", PORT), Handler)
    print(f"guide_gui proxy: http://127.0.0.1:{PORT}/guide.html?handle=...  -> 上游 {UP_BASE}（HTML 取自 {HTML}）")
    srv.serve_forever()
