//! F1RMA MCP Server — expõe ferramentas de gerenciamento de arquivos para LLMs.
//! Protocolo: JSON-RPC 2.0 sobre stdio (stdin/stdout).

mod protocol;
mod tools;

use protocol::{JsonRpcRequest, JsonRpcResponse, McpError};
use std::io::{self, BufRead, Write};

fn main() {
    env_logger::init();
    log::info!("F1RMA MCP Server iniciado");

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let error_resp = JsonRpcResponse::error(
                    serde_json::Value::Null,
                    McpError::parse_error(&format!("JSON inválido: {}", e)),
                );
                let _ = writeln!(stdout, "{}", serde_json::to_string(&error_resp).unwrap());
                let _ = stdout.flush();
                continue;
            }
        };

        let response = handle_request(&request);
        let _ = writeln!(stdout, "{}", serde_json::to_string(&response).unwrap());
        let _ = stdout.flush();
    }
}

fn handle_request(req: &JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => protocol::handle_initialize(req),
        "notifications/initialized" => JsonRpcResponse::success(req.id.clone(), serde_json::json!({})),
        "tools/list" => tools::handle_list_tools(req),
        "tools/call" => tools::handle_call_tool(req),
        _ => JsonRpcResponse::error(
            req.id.clone(),
            McpError::method_not_found(&req.method),
        ),
    }
}
