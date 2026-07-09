//! Tipos do protocolo MCP (JSON-RPC 2.0).

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

impl JsonRpcResponse {
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Value, error: McpError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
}

impl McpError {
    pub fn parse_error(msg: &str) -> Self {
        Self { code: -32700, message: msg.to_string() }
    }
    pub fn method_not_found(method: &str) -> Self {
        Self { code: -32601, message: format!("Método não encontrado: {}", method) }
    }
    pub fn invalid_params(msg: &str) -> Self {
        Self { code: -32602, message: msg.to_string() }
    }
}

/// Responde ao `initialize` com as capacidades do server.
pub fn handle_initialize(req: &JsonRpcRequest) -> JsonRpcResponse {
    let result = serde_json::json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "f1rma-mcp",
            "version": env!("CARGO_PKG_VERSION")
        }
    });
    JsonRpcResponse::success(req.id.clone(), result)
}
