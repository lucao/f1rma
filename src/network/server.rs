use crate::core::annotations::AnnotationStore;
use crate::core::profile::ProfileConfig;
use crate::network::share::{ModificationRequest, RequestStatus, ShareConfig};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Estado compartilhado do servidor de rede.
pub struct NetworkServer {
    pub config: ShareConfig,
    pub profile_config: Arc<Mutex<ProfileConfig>>,
    pub annotations: Arc<Mutex<AnnotationStore>>,
    pub pending_requests: Arc<Mutex<Vec<ModificationRequest>>>,
    pub running: Arc<Mutex<bool>>,
}

impl NetworkServer {
    pub fn new(
        config: ShareConfig,
        profile_config: Arc<Mutex<ProfileConfig>>,
        annotations: Arc<Mutex<AnnotationStore>>,
    ) -> Self {
        Self {
            config,
            profile_config,
            annotations,
            pending_requests: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Inicia o servidor HTTP de compartilhamento.
    /// Em uma implementação completa, isso usaria warp para servir os endpoints.
    pub async fn start(&self) -> Result<(), String> {
        *self.running.lock().unwrap() = true;
        log::info!(
            "Servidor de rede iniciado na porta {}",
            self.config.port
        );
        // TODO: Implementar endpoints warp:
        // GET /files - listar arquivos compartilhados
        // GET /files/{path} - baixar arquivo (somente leitura)
        // POST /request-modify - solicitar permissão de modificação
        // GET /annotations/{path} - obter anotações de um arquivo
        // POST /annotations/{path} - adicionar anotação
        Ok(())
    }

    /// Para o servidor de compartilhamento.
    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
        log::info!("Servidor de rede parado");
    }

    /// Processa um pedido de modificação (a ser chamado pela UI para aprovar/negar).
    pub fn process_request(&self, request_id: uuid::Uuid, approved: bool) {
        let mut requests = self.pending_requests.lock().unwrap();
        if let Some(req) = requests.iter_mut().find(|r| r.id == request_id) {
            req.status = if approved {
                RequestStatus::Approved
            } else {
                RequestStatus::Denied
            };
        }
    }

    /// Verifica se um arquivo pode ser compartilhado.
    pub fn can_share_file(&self, path: &PathBuf) -> bool {
        let config = self.profile_config.lock().unwrap();
        config.is_network_available(path)
    }
}
