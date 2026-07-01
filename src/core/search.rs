use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Resultado de busca de arquivo/diretório.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

/// Realiza busca por nome de arquivo ou diretório a partir de um caminho raiz.
pub fn search_files(root: &Path, query: &str, max_results: usize) -> Vec<SearchResult> {
    if query.is_empty() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();

    WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .to_lowercase()
                .contains(&query_lower)
        })
        .take(max_results)
        .map(|entry| SearchResult {
            path: entry.path().to_path_buf(),
            name: entry.file_name().to_string_lossy().to_string(),
            is_dir: entry.file_type().is_dir(),
        })
        .collect()
}

/// Constrói a árvore de diretórios para exibição no painel lateral.
#[derive(Debug, Clone)]
pub struct DirNode {
    pub path: PathBuf,
    pub name: String,
    pub children: Vec<DirNode>,
    pub is_expanded: bool,
}

impl DirNode {
    pub fn from_path(path: &Path, depth: usize) -> Option<Self> {
        if !path.is_dir() {
            return None;
        }

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        let children = if depth > 0 {
            std::fs::read_dir(path)
                .ok()?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .filter_map(|entry| DirNode::from_path(&entry.path(), depth - 1))
                .collect()
        } else {
            Vec::new()
        };

        Some(DirNode {
            path: path.to_path_buf(),
            name,
            children,
            is_expanded: false,
        })
    }
}
