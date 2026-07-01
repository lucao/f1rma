use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Controle de permissões para modificação de arquivos por máquinas remotas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionManager {
    /// Máquinas com permissão permanente (whitelist).
    pub trusted_machines: Vec<String>,
    /// Permissões temporárias ativas (machine_id -> arquivos permitidos).
    pub active_permissions: HashMap<String, Vec<PathBuf>>,
    /// Se deve pedir confirmação para cada operação.
    pub always_ask: bool,
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self {
            trusted_machines: Vec::new(),
            active_permissions: HashMap::new(),
            always_ask: true,
        }
    }
}

impl PermissionManager {
    /// Verifica se uma máquina tem permissão para modificar um arquivo.
    pub fn has_permission(&self, machine_id: &str, file_path: &PathBuf) -> bool {
        if self.trusted_machines.contains(&machine_id.to_string()) {
            return true;
        }

        if let Some(allowed_files) = self.active_permissions.get(machine_id) {
            return allowed_files.contains(file_path);
        }

        false
    }

    /// Concede permissão temporária a uma máquina para um arquivo.
    pub fn grant_permission(&mut self, machine_id: String, file_path: PathBuf) {
        self.active_permissions
            .entry(machine_id)
            .or_default()
            .push(file_path);
    }

    /// Revoga permissão de uma máquina para um arquivo.
    pub fn revoke_permission(&mut self, machine_id: &str, file_path: &PathBuf) {
        if let Some(files) = self.active_permissions.get_mut(machine_id) {
            files.retain(|f| f != file_path);
        }
    }

    /// Adiciona uma máquina à lista de confiança.
    pub fn trust_machine(&mut self, machine_id: String) {
        if !self.trusted_machines.contains(&machine_id) {
            self.trusted_machines.push(machine_id);
        }
    }

    /// Remove uma máquina da lista de confiança.
    pub fn untrust_machine(&mut self, machine_id: &str) {
        self.trusted_machines.retain(|m| m != machine_id);
    }

    /// Limpa todas as permissões temporárias.
    pub fn clear_temporary_permissions(&mut self) {
        self.active_permissions.clear();
    }
}
