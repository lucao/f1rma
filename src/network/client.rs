use crate::core::annotations::Annotation;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Cliente para conectar a outras máquinas na rede.
#[derive(Debug, Clone)]
pub struct NetworkClient {
    pub base_url: String,
}

/// Informações sobre uma máquina descoberta na rede.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteMachine {
    pub name: String,
    pub machine_id: String,
    pub address: String,
    pub port: u16,
}

impl NetworkClient {
    pub fn new(address: &str, port: u16) -> Self {
        Self {
            base_url: format!("http://{}:{}", address, port),
        }
    }

    /// Lista arquivos compartilhados de uma máquina remota.
    pub async fn list_remote_files(&self) -> Result<Vec<PathBuf>, String> {
        // TODO: Implementar com reqwest
        // GET {base_url}/files
        Ok(Vec::new())
    }

    /// Solicita autorização para modificar um arquivo remoto.
    pub async fn request_modification(
        &self,
        file_path: &PathBuf,
        operation: &str,
    ) -> Result<bool, String> {
        // TODO: Implementar com reqwest
        // POST {base_url}/request-modify
        log::info!(
            "Solicitando modificação de {:?} - {}",
            file_path,
            operation
        );
        Ok(false)
    }

    /// Obtém anotações de um arquivo remoto.
    pub async fn get_remote_annotations(
        &self,
        file_path: &PathBuf,
    ) -> Result<Vec<Annotation>, String> {
        // TODO: Implementar com reqwest
        // GET {base_url}/annotations/{path}
        Ok(Vec::new())
    }

    /// Adiciona uma anotação a um arquivo remoto.
    pub async fn add_remote_annotation(
        &self,
        file_path: &PathBuf,
        annotation: &Annotation,
    ) -> Result<(), String> {
        // TODO: Implementar com reqwest
        // POST {base_url}/annotations/{path}
        Ok(())
    }
}
