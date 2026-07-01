use crate::core::profile::ProfileConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuração de compartilhamento de rede.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareConfig {
    /// Porta do servidor HTTP para compartilhamento.
    pub port: u16,
    /// Se o servidor de compartilhamento está ativo.
    pub enabled: bool,
    /// Nome da máquina na rede.
    pub machine_name: String,
    /// ID único da máquina.
    pub machine_id: String,
    /// Nome do usuário.
    pub username: String,
}

impl Default for ShareConfig {
    fn default() -> Self {
        Self {
            port: 7100,
            enabled: false,
            machine_name: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
            machine_id: uuid::Uuid::new_v4().to_string(),
            username: whoami::username(),
        }
    }
}

/// Representa um pedido de autorização para modificar um arquivo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationRequest {
    pub id: uuid::Uuid,
    pub requester_machine: String,
    pub requester_user: String,
    pub file_path: PathBuf,
    pub operation: String,
    pub status: RequestStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequestStatus {
    Pending,
    Approved,
    Denied,
}

impl ModificationRequest {
    pub fn new(
        requester_machine: String,
        requester_user: String,
        file_path: PathBuf,
        operation: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            requester_machine,
            requester_user,
            file_path,
            operation,
            status: RequestStatus::Pending,
        }
    }
}

/// Determina quais arquivos podem ser compartilhados na rede.
pub fn get_shareable_files(
    root: &Path,
    profile_config: &ProfileConfig,
) -> Vec<PathBuf> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| profile_config.is_network_available(entry.path()))
        .map(|entry| entry.path().to_path_buf())
        .collect()
}
