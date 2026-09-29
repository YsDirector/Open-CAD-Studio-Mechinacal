//! OCSM MCP 服务器（独立二进制，标准 MCP stdio）。
//!
//! AI 的 MCP 客户端把本进程作为子进程 spawn（stdio 传输，newline-delimited
//! JSON-RPC 2.0）。工具调用经本地 TCP 桥接回 OCSM 插件的标注更新服务器
//! （`POST /api/mcp`），由插件操作 OCS 活文档。
//!
//! 端口：`OCSM_GUIDE_PORT` 环境变量，默认 23751（与插件默认一致）。先在 OCS
//! 里运行 `OCSMMCP` 确保服务器就绪。

use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};

const PROTOCOL_VERSION: &str = "2024-11-05";

/// 插件 `/api/mcp`（`guide_server::api_mcp`）认识的方法名。
///
/// ⚠️ MCP 工具名必须与之一致（桥接原样转发 `method`）：历史上工具名叫
/// `apply_guide` / `apply_guide_refresh`，而插件只认 `apply` / `apply_refresh`，
/// 导致两个写工具实际不可用（返回“未知 MCP 方法”）。2026-09-15 对齐。
const PLUGIN_METHODS: &[&str] = &["list_guides", "get_guide", "apply", "apply_refresh"];

/// MCP 工具名 → 插件方法名（None = 该工具不存在，不发给插件）。
fn plugin_method(tool: &str) -> Option<&'static str> {
    PLUGIN_METHODS.iter().copied().find(|m| *m == tool)
}

fn main() {
    let port = std::env::var("OCSM_GUIDE_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(23751);

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let resp = handle(&req, port);
        // notification（无 id）不响应。
        if req.get("id").is_some() {
            let mut out = serde_json::to_string(&resp).unwrap();
            out.push('\n');
            let mut stdout = std::io::stdout();
            let _ = stdout.write_all(out.as_bytes());
            let _ = stdout.flush();
        }
    }
}

fn handle(req: &Value, port: u16) -> Value {
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => {
            let client = req.pointer("/params/clientInfo").cloned().unwrap_or(json!({}));
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "ocs-ocsm-mcp", "version": env!("CARGO_PKG_VERSION") },
                "instructions": "控制 OCS 的引导线标注：list_guides / get_guide / apply / apply_refresh。先确保 OCS 内运行了 OCSMMCP。",
                "clientInfo": client,
            })
        }
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools() }),
        "tools/call" => {
            let name = req.pointer("/params/name").and_then(|n| n.as_str()).unwrap_or("");
            let args = req.pointer("/params/arguments").cloned().unwrap_or(json!({}));
            call_tool(name, &args, port)
        }
        other => {
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("未知方法: {other}") },
            })
        }
    };
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "list_guides",
            "description": "列出图纸中所有引导线（10引导线层且带 /DIM/ 超链接），含几何与当前 URL。",
            "inputSchema": { "type": "object", "properties": {} },
        }),
        json!({
            "name": "get_guide",
            "description": "获取单条引导线的几何（P1/P2）与当前超链接 URL、解析参数。",
            "inputSchema": {
                "type": "object",
                "properties": { "handle": { "type": "string", "description": "引导线 handle（如 0x3AE）" } },
                "required": ["handle"],
            },
        }),
        json!({
            "name": "apply",
            "description": "把标注参数 URL 写入引导线超链接（应用，不生成标注）。URL 形如 http://127.0.0.1:23751/DIM/LINEAR/H/-50。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "handle": { "type": "string", "description": "引导线 handle" },
                    "url": { "type": "string", "description": "标注参数 URL" },
                },
                "required": ["handle", "url"],
            },
        }),
        json!({
            "name": "apply_refresh",
            "description": "把参数 URL 写入引导线并立即生成真实标注（7标注层/OCSM_GB），删除引导线并刷新（应用并刷新）。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "handle": { "type": "string", "description": "引导线 handle" },
                    "url": { "type": "string", "description": "标注参数 URL" },
                },
                "required": ["handle", "url"],
            },
        }),
    ]
}

fn call_tool(name: &str, args: &Value, port: u16) -> Value {
    // 桥接：工具名 → 插件 `/api/mcp` 的方法名（两者必须一致，见 PLUGIN_METHODS）。
    let Some(method) = plugin_method(name) else {
        return json!({
            "content": [{ "type": "text", "text": format!("未知工具: {name}") }],
            "isError": true
        });
    };
    let body = json!({ "method": method, "params": args });
    match bridge_http(port, &body) {
        Ok(v) => {
            let text = serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into());
            json!({ "content": [{ "type": "text", "text": text }], "isError": false })
        }
        Err(e) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
    }
}

/// 经本地 TCP 把 JSON 请求 POST 到插件的 /api/mcp，返回解析后的 JSON。
fn bridge_http(port: u16, body: &Value) -> Result<Value, String> {
    let body_str = serde_json::to_string(body).map_err(|e| e.to_string())?;
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).map_err(|e| {
        format!(
            "无法连接 OCSM 标注更新服务器(127.0.0.1:{port})：{e}。请先在 OCS 里运行 OCSMMCP（或 GDIM）启动服务器。"
        )
    })?;
    write!(
        stream,
        "POST /api/mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body_str.len(),
        body_str
    )
    .map_err(|e| format!("发送请求失败: {e}"))?;
    stream.flush().map_err(|e| format!("发送请求失败: {e}"))?;

    let mut resp = String::new();
    stream
        .read_to_string(&mut resp)
        .map_err(|e| format!("读取响应失败: {e}"))?;
    let body_part = resp.split("\r\n\r\n").nth(1).unwrap_or("");
    serde_json::from_str(body_part).map_err(|e| format!("响应 JSON 无效: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_returns_capabilities() {
        let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"t","version":"1"}}});
        let resp = handle(&req, 23751);
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert!(resp["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn tools_list_returns_four_tools() {
        let req = json!({"jsonrpc":"2.0","id":2,"method":"tools/list"});
        let resp = handle(&req, 23751);
        let tools = resp["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 4);
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"list_guides"));
        assert!(names.contains(&"apply_refresh"));
    }

    #[test]
    fn every_tool_name_is_a_plugin_method() {
        // 回归：工具名与插件 `/api/mcp` 的方法名必须一一对应，否则写工具会收到
        // “未知 MCP 方法”（2026-09-15 的 apply_guide/apply_guide_refresh 事故）。
        let req = json!({"jsonrpc":"2.0","id":5,"method":"tools/list"});
        let resp = handle(&req, 23751);
        for tool in resp["result"]["tools"].as_array().unwrap() {
            let name = tool["name"].as_str().unwrap();
            assert_eq!(
                plugin_method(name),
                Some(name),
                "工具 {name} 没有对应的插件方法"
            );
        }
        assert_eq!(plugin_method("apply_guide"), None, "旧名不应再映射");
    }

    #[test]
    fn notification_is_ignored() {
        // handle 返回结果但不写 stdout；这里只验证不 panic、id 为 null。
        let req = json!({"jsonrpc":"2.0","method":"notifications/initialized"});
        let resp = handle(&req, 23751);
        assert_eq!(resp["id"], Value::Null);
    }

    #[test]
    fn unknown_method_errors() {
        let req = json!({"jsonrpc":"2.0","id":3,"method":"nope"});
        let resp = handle(&req, 23751);
        assert!(resp["error"]["code"].is_i64());
        assert_eq!(resp["error"]["code"].as_i64(), Some(-32601));
    }

    #[test]
    fn tools_call_unreachable_server_reports_error() {
        // 端口 1 无服务 → 桥接失败 → isError=true。
        let req = json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"list_guides","arguments":{}}});
        let resp = handle(&req, 1);
        let call = resp["result"].clone();
        assert_eq!(call["isError"].as_bool(), Some(true));
    }
}
