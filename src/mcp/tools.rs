//! Tools MCP expostas para o LLM.

use crate::protocol::{JsonRpcRequest, JsonRpcResponse, McpError};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Retorna a lista de tools disponíveis.
pub fn handle_list_tools(req: &JsonRpcRequest) -> JsonRpcResponse {
    let tools = json!({
        "tools": [
            {
                "name": "list_files",
                "description": "Lista arquivos e pastas em um diretório. Retorna nome, tipo, tamanho e perfis atribuídos. Não lista conteúdo de diretórios seguros/criptografados.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Caminho do diretório a listar" },
                        "recursive": { "type": "boolean", "description": "Se deve listar recursivamente (padrão: false)" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "read_file",
                "description": "Lê o conteúdo de um arquivo de texto. Não funciona em arquivos dentro de diretórios seguros. Limita a 50KB.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Caminho do arquivo a ler" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "get_file_info",
                "description": "Retorna metadados de um arquivo: tamanho, data de modificação, perfis, anotações existentes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Caminho do arquivo ou pasta" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "search_files",
                "description": "Busca arquivos por nome em um diretório.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Diretório raiz da busca" },
                        "query": { "type": "string", "description": "Termo de busca (nome parcial)" },
                        "max_results": { "type": "integer", "description": "Máximo de resultados (padrão: 50)" }
                    },
                    "required": ["path", "query"]
                }
            },
            {
                "name": "assign_profiles",
                "description": "Atribui um ou mais perfis a um arquivo ou pasta. Perfis disponíveis: Pessoal, Profissional, Dev, ou perfis customizados criados pelo usuário.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Caminho do arquivo ou pasta" },
                        "profiles": { "type": "array", "items": { "type": "string" }, "description": "Lista de nomes de perfis a atribuir" }
                    },
                    "required": ["path", "profiles"]
                }
            },
            {
                "name": "add_annotation",
                "description": "Adiciona uma anotação/nota a um arquivo ou pasta. A anotação fica permanentemente atrelada ao arquivo.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Caminho do arquivo ou pasta" },
                        "content": { "type": "string", "description": "Texto da anotação" }
                    },
                    "required": ["path", "content"]
                }
            },
            {
                "name": "move_file",
                "description": "Move um arquivo ou pasta para outro local. Requer confirmação implícita do usuário.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source": { "type": "string", "description": "Caminho de origem" },
                        "destination": { "type": "string", "description": "Caminho de destino" }
                    },
                    "required": ["source", "destination"]
                }
            },
            {
                "name": "rename_file",
                "description": "Renomeia um arquivo ou pasta.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Caminho atual" },
                        "new_name": { "type": "string", "description": "Novo nome (apenas o nome, sem caminho)" }
                    },
                    "required": ["path", "new_name"]
                }
            },
            {
                "name": "create_folder",
                "description": "Cria uma nova pasta.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Caminho completo da pasta a criar" }
                    },
                    "required": ["path"]
                }
            }
        ]
    });
    JsonRpcResponse::success(req.id.clone(), tools)
}

/// Despacha a chamada de uma tool.
pub fn handle_call_tool(req: &JsonRpcRequest) -> JsonRpcResponse {
    let params = &req.params;
    let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = match tool_name {
        "list_files" => exec_list_files(&arguments),
        "read_file" => exec_read_file(&arguments),
        "get_file_info" => exec_get_file_info(&arguments),
        "search_files" => exec_search_files(&arguments),
        "assign_profiles" => exec_assign_profiles(&arguments),
        "add_annotation" => exec_add_annotation(&arguments),
        "move_file" => exec_move_file(&arguments),
        "rename_file" => exec_rename_file(&arguments),
        "create_folder" => exec_create_folder(&arguments),
        _ => Err(format!("Tool desconhecida: {}", tool_name)),
    };

    match result {
        Ok(content) => JsonRpcResponse::success(req.id.clone(), json!({
            "content": [{ "type": "text", "text": content }]
        })),
        Err(e) => JsonRpcResponse::success(req.id.clone(), json!({
            "content": [{ "type": "text", "text": format!("Erro: {}", e) }],
            "isError": true
        })),
    }
}

/// Diretórios seguros configurados (carrega do config do F1RMA).
fn load_secure_dirs() -> Vec<PathBuf> {
    let config_dir = directories::ProjectDirs::from("com", "f1rma", "F1RMA")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".f1rma"));
    let profiles_path = config_dir.join("profiles.json");
    if let Ok(content) = std::fs::read_to_string(&profiles_path) {
        if let Ok(val) = serde_json::from_str::<Value>(&content) {
            if let Some(dirs) = val.get("secure_directories").and_then(|v| v.as_array()) {
                return dirs.iter()
                    .filter_map(|v| v.as_str().map(PathBuf::from))
                    .collect();
            }
        }
    }
    Vec::new()
}

fn is_path_secure(path: &Path) -> bool {
    let secure_dirs = load_secure_dirs();
    secure_dirs.iter().any(|sd| path.starts_with(sd))
}

fn exec_list_files(args: &Value) -> Result<String, String> {
    let path_str = args.get("path").and_then(|v| v.as_str())
        .ok_or("Parâmetro 'path' obrigatório")?;
    let path = PathBuf::from(path_str);
    let recursive = args.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false);

    if is_path_secure(&path) {
        return Err("Acesso negado: diretório seguro".to_string());
    }
    if !path.exists() {
        return Err(format!("Caminho não existe: {}", path_str));
    }

    let mut entries = Vec::new();
    if recursive {
        for entry in walkdir::WalkDir::new(&path).max_depth(5) {
            if let Ok(e) = entry {
                if is_path_secure(e.path()) { continue; }
                let meta = e.metadata().ok();
                entries.push(json!({
                    "path": e.path().to_string_lossy(),
                    "name": e.file_name().to_string_lossy(),
                    "is_dir": e.file_type().is_dir(),
                    "size": meta.as_ref().map(|m| m.len()).unwrap_or(0),
                }));
                if entries.len() >= 500 { break; }
            }
        }
    } else {
        let dir_entries = std::fs::read_dir(&path).map_err(|e| e.to_string())?;
        for entry in dir_entries.filter_map(|e| e.ok()) {
            let ep = entry.path();
            if is_path_secure(&ep) { continue; }
            let meta = entry.metadata().ok();
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(json!({
                "path": ep.to_string_lossy(),
                "name": name,
                "is_dir": ep.is_dir(),
                "size": meta.as_ref().map(|m| m.len()).unwrap_or(0),
            }));
        }
    }

    Ok(serde_json::to_string_pretty(&entries).unwrap_or_default())
}

fn exec_read_file(args: &Value) -> Result<String, String> {
    let path_str = args.get("path").and_then(|v| v.as_str())
        .ok_or("Parâmetro 'path' obrigatório")?;
    let path = PathBuf::from(path_str);

    if is_path_secure(&path) {
        return Err("Acesso negado: arquivo em diretório seguro".to_string());
    }
    if !path.is_file() {
        return Err("Caminho não é um arquivo".to_string());
    }

    let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    if metadata.len() > 50 * 1024 {
        return Err("Arquivo muito grande (>50KB). Use get_file_info para metadados.".to_string());
    }

    std::fs::read_to_string(&path).map_err(|e| format!("Erro ao ler: {} (pode ser binário)", e))
}

fn exec_get_file_info(args: &Value) -> Result<String, String> {
    let path_str = args.get("path").and_then(|v| v.as_str())
        .ok_or("Parâmetro 'path' obrigatório")?;
    let path = PathBuf::from(path_str);

    if !path.exists() {
        return Err("Caminho não existe".to_string());
    }

    let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;

    // Carrega anotações do sidecar
    let sidecar = PathBuf::from(format!("{}.f1rma", path_str));
    let annotations: Vec<Value> = if sidecar.exists() {
        std::fs::read_to_string(&sidecar).ok()
            .and_then(|c| serde_json::from_str::<Value>(&c).ok())
            .and_then(|v| v.get("annotations")?.as_array().cloned())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let info = json!({
        "path": path_str,
        "is_dir": metadata.is_dir(),
        "size": metadata.len(),
        "readonly": metadata.permissions().readonly(),
        "is_secure": is_path_secure(&path),
        "annotations_count": annotations.len(),
        "annotations": annotations,
    });

    Ok(serde_json::to_string_pretty(&info).unwrap_or_default())
}

fn exec_search_files(args: &Value) -> Result<String, String> {
    let path_str = args.get("path").and_then(|v| v.as_str())
        .ok_or("Parâmetro 'path' obrigatório")?;
    let query = args.get("query").and_then(|v| v.as_str())
        .ok_or("Parâmetro 'query' obrigatório")?;
    let max = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
    let path = PathBuf::from(path_str);

    if !path.exists() {
        return Err("Caminho não existe".to_string());
    }

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for entry in walkdir::WalkDir::new(&path).into_iter().filter_map(|e| e.ok()) {
        if is_path_secure(entry.path()) { continue; }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name.contains(&query_lower) {
            results.push(json!({
                "path": entry.path().to_string_lossy(),
                "name": entry.file_name().to_string_lossy(),
                "is_dir": entry.file_type().is_dir(),
            }));
            if results.len() >= max { break; }
        }
    }

    Ok(serde_json::to_string_pretty(&results).unwrap_or_default())
}

fn exec_assign_profiles(args: &Value) -> Result<String, String> {
    let path_str = args.get("path").and_then(|v| v.as_str())
        .ok_or("Parâmetro 'path' obrigatório")?;
    let profiles = args.get("profiles").and_then(|v| v.as_array())
        .ok_or("Parâmetro 'profiles' obrigatório (array de strings)")?;
    let path = PathBuf::from(path_str);

    if !path.exists() {
        return Err("Caminho não existe".to_string());
    }

    // Carrega config de perfis
    let config_dir = directories::ProjectDirs::from("com", "f1rma", "F1RMA")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".f1rma"));
    let profiles_path = config_dir.join("profiles.json");

    let mut config: Value = std::fs::read_to_string(&profiles_path).ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or(json!({"assignments": {}, "secure_directories": [], "registry": {"custom_profiles": [], "no_share_profiles": []}}));

    let assignments = config.get_mut("assignments")
        .and_then(|v| v.as_object_mut())
        .ok_or("Config de perfis corrompida")?;

    // Converte nomes para o formato de Profile
    let profile_values: Vec<Value> = profiles.iter().map(|p| {
        let name = p.as_str().unwrap_or("");
        match name {
            "Pessoal" => json!("Pessoal"),
            "Profissional" => json!("Profissional"),
            "Dev" => json!("Dev"),
            custom => json!({"Custom": custom}),
        }
    }).collect();

    assignments.insert(path_str.to_string(), json!(profile_values));

    let _ = std::fs::create_dir_all(&config_dir);
    let _ = std::fs::write(&profiles_path, serde_json::to_string_pretty(&config).unwrap());

    Ok(format!("Perfis {:?} atribuídos a {}", 
        profiles.iter().filter_map(|p| p.as_str()).collect::<Vec<_>>(), path_str))
}

fn exec_add_annotation(args: &Value) -> Result<String, String> {
    let path_str = args.get("path").and_then(|v| v.as_str())
        .ok_or("Parâmetro 'path' obrigatório")?;
    let content = args.get("content").and_then(|v| v.as_str())
        .ok_or("Parâmetro 'content' obrigatório")?;
    let path = PathBuf::from(path_str);

    if !path.exists() {
        return Err("Caminho não existe".to_string());
    }

    let sidecar = PathBuf::from(format!("{}.f1rma", path_str));
    let mut data: Value = if sidecar.exists() {
        std::fs::read_to_string(&sidecar).ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or(json!({"annotations": []}))
    } else {
        json!({"annotations": []})
    };

    let annotation = json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "content": content,
        "author": "AI Assistant",
        "machine_id": "mcp-server",
        "created_at": chrono::Utc::now().to_rfc3339(),
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });

    if let Some(arr) = data.get_mut("annotations").and_then(|v| v.as_array_mut()) {
        arr.push(annotation);
    }

    std::fs::write(&sidecar, serde_json::to_string_pretty(&data).unwrap())
        .map_err(|e| e.to_string())?;

    Ok(format!("Anotação adicionada a {}", path_str))
}

fn exec_move_file(args: &Value) -> Result<String, String> {
    let source = args.get("source").and_then(|v| v.as_str())
        .ok_or("Parâmetro 'source' obrigatório")?;
    let dest = args.get("destination").and_then(|v| v.as_str())
        .ok_or("Parâmetro 'destination' obrigatório")?;

    let src_path = PathBuf::from(source);
    let dst_path = PathBuf::from(dest);

    if !src_path.exists() {
        return Err("Origem não existe".to_string());
    }
    if is_path_secure(&src_path) {
        return Err("Acesso negado: origem em diretório seguro".to_string());
    }
    if dst_path.exists() {
        return Err("Destino já existe".to_string());
    }

    // Move o arquivo e o sidecar de anotações junto
    std::fs::rename(&src_path, &dst_path).map_err(|e| e.to_string())?;

    let src_sidecar = PathBuf::from(format!("{}.f1rma", source));
    if src_sidecar.exists() {
        let dst_sidecar = PathBuf::from(format!("{}.f1rma", dest));
        let _ = std::fs::rename(&src_sidecar, &dst_sidecar);
    }

    Ok(format!("Movido: {} → {}", source, dest))
}

fn exec_rename_file(args: &Value) -> Result<String, String> {
    let path_str = args.get("path").and_then(|v| v.as_str())
        .ok_or("Parâmetro 'path' obrigatório")?;
    let new_name = args.get("new_name").and_then(|v| v.as_str())
        .ok_or("Parâmetro 'new_name' obrigatório")?;

    let path = PathBuf::from(path_str);
    if !path.exists() {
        return Err("Caminho não existe".to_string());
    }
    if is_path_secure(&path) {
        return Err("Acesso negado: diretório seguro".to_string());
    }

    let parent = path.parent().ok_or("Sem diretório pai")?;
    let new_path = parent.join(new_name);
    if new_path.exists() {
        return Err(format!("Já existe: {}", new_name));
    }

    std::fs::rename(&path, &new_path).map_err(|e| e.to_string())?;

    // Move sidecar junto
    let old_sidecar = PathBuf::from(format!("{}.f1rma", path_str));
    if old_sidecar.exists() {
        let new_sidecar = PathBuf::from(format!("{}.f1rma", new_path.to_string_lossy()));
        let _ = std::fs::rename(&old_sidecar, &new_sidecar);
    }

    Ok(format!("Renomeado: {} → {}", path_str, new_name))
}

fn exec_create_folder(args: &Value) -> Result<String, String> {
    let path_str = args.get("path").and_then(|v| v.as_str())
        .ok_or("Parâmetro 'path' obrigatório")?;
    let path = PathBuf::from(path_str);

    if path.exists() {
        return Err("Caminho já existe".to_string());
    }

    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(format!("Pasta criada: {}", path_str))
}
