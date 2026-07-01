use mdns_sd::{DaemonEvent, ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Tipo de serviço mDNS registrado pelo F1RMA.
const SERVICE_TYPE: &str = "_f1rma._tcp.local.";

/// Informações de um peer descoberto na rede.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    /// Nome de exibição da máquina.
    pub machine_name: String,
    /// Identificador único da máquina.
    pub machine_id: String,
    /// Nome do usuário.
    pub username: String,
    /// Endereço IP do peer.
    pub address: String,
    /// Porta do servidor HTTP de compartilhamento.
    pub port: u16,
    /// Versão do F1RMA rodando no peer.
    pub version: String,
    /// Momento em que o peer foi visto pela última vez.
    #[serde(skip)]
    pub last_seen: Option<Instant>,
    /// Se o peer está online (respondendo).
    pub online: bool,
}

/// Estado compartilhado do discovery, acessível pela UI e pelo daemon.
#[derive(Debug, Clone)]
pub struct DiscoveryState {
    pub peers: Arc<Mutex<HashMap<String, Peer>>>,
    pub is_running: Arc<Mutex<bool>>,
}

impl Default for DiscoveryState {
    fn default() -> Self {
        Self {
            peers: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(Mutex::new(false)),
        }
    }
}

impl DiscoveryState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Retorna a lista de peers online.
    pub fn online_peers(&self) -> Vec<Peer> {
        let peers = self.peers.lock().unwrap();
        peers.values().filter(|p| p.online).cloned().collect()
    }

    /// Retorna todos os peers conhecidos.
    pub fn all_peers(&self) -> Vec<Peer> {
        let peers = self.peers.lock().unwrap();
        peers.values().cloned().collect()
    }

    /// Número de peers online.
    pub fn peer_count(&self) -> usize {
        let peers = self.peers.lock().unwrap();
        peers.values().filter(|p| p.online).count()
    }
}

/// Gerenciador do serviço mDNS — registra esta máquina e descobre peers.
pub struct DiscoveryService {
    state: DiscoveryState,
    daemon: Option<ServiceDaemon>,
    local_machine_id: String,
}

impl DiscoveryService {
    pub fn new(state: DiscoveryState, local_machine_id: String) -> Self {
        Self {
            state,
            daemon: None,
            local_machine_id,
        }
    }

    /// Inicia o serviço: registra esta máquina e começa a escutar peers.
    pub fn start(
        &mut self,
        machine_name: &str,
        machine_id: &str,
        username: &str,
        port: u16,
    ) -> Result<(), String> {
        let daemon = ServiceDaemon::new()
            .map_err(|e| format!("Erro ao criar daemon mDNS: {}", e))?;

        // Registra este serviço na rede
        let instance_name = format!("{}_{}", machine_name, &machine_id[..8]);
        let host_name = format!("{}.local.", machine_name.to_lowercase().replace(' ', "-"));

        let mut properties = HashMap::new();
        properties.insert("machine_id".to_string(), machine_id.to_string());
        properties.insert("username".to_string(), username.to_string());
        properties.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());

        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &instance_name,
            &host_name,
            "",  // IP será detectado automaticamente
            port,
            properties,
        )
        .map_err(|e| format!("Erro ao criar ServiceInfo: {}", e))?;

        daemon
            .register(service_info)
            .map_err(|e| format!("Erro ao registrar serviço: {}", e))?;

        // Inicia o browse para descobrir outros peers
        let receiver = daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| format!("Erro ao iniciar browse: {}", e))?;

        *self.state.is_running.lock().unwrap() = true;

        // Spawna thread para processar eventos de discovery
        let peers = self.state.peers.clone();
        let is_running = self.state.is_running.clone();
        let local_id = self.local_machine_id.clone();

        std::thread::spawn(move || {
            loop {
                if !*is_running.lock().unwrap() {
                    break;
                }

                match receiver.recv_timeout(std::time::Duration::from_secs(1)) {
                    Ok(event) => {
                        handle_service_event(&peers, &local_id, event);
                    }
                    Err(flume::RecvTimeoutError::Timeout) => continue,
                    Err(flume::RecvTimeoutError::Disconnected) => break,
                }
            }
            log::info!("Thread de discovery encerrada");
        });

        self.daemon = Some(daemon);
        log::info!("Serviço mDNS iniciado: {} na porta {}", instance_name, port);
        Ok(())
    }

    /// Para o serviço de discovery.
    pub fn stop(&mut self) {
        *self.state.is_running.lock().unwrap() = false;
        if let Some(daemon) = self.daemon.take() {
            let _ = daemon.shutdown();
        }
        log::info!("Serviço mDNS encerrado");
    }

    /// Retorna o estado compartilhado para a UI.
    pub fn state(&self) -> &DiscoveryState {
        &self.state
    }
}

impl Drop for DiscoveryService {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Processa um evento de serviço mDNS descoberto.
fn handle_service_event(
    peers: &Arc<Mutex<HashMap<String, Peer>>>,
    local_machine_id: &str,
    event: ServiceEvent,
) {
    match event {
        ServiceEvent::ServiceResolved(info) => {
            let properties = info.get_properties();

            let machine_id = properties
                .get_property_val_str("machine_id")
                .unwrap_or_default()
                .to_string();

            // Ignora a própria máquina
            if machine_id == local_machine_id {
                return;
            }

            let username = properties
                .get_property_val_str("username")
                .unwrap_or_default()
                .to_string();

            let version = properties
                .get_property_val_str("version")
                .unwrap_or_default()
                .to_string();

            let address = info
                .get_addresses()
                .iter()
                .next()
                .map(|a| a.to_string())
                .unwrap_or_default();

            let peer = Peer {
                machine_name: info.get_fullname().split('.').next().unwrap_or("unknown").to_string(),
                machine_id: machine_id.clone(),
                username,
                address,
                port: info.get_port(),
                version,
                last_seen: Some(Instant::now()),
                online: true,
            };

            log::info!(
                "Peer descoberto: {} ({}:{})",
                peer.machine_name,
                peer.address,
                peer.port
            );

            let mut peers_map = peers.lock().unwrap();
            peers_map.insert(machine_id, peer);
        }
        ServiceEvent::ServiceRemoved(_type, fullname) => {
            log::info!("Peer removido: {}", fullname);
            let mut peers_map = peers.lock().unwrap();
            // Marca como offline pelo fullname
            for peer in peers_map.values_mut() {
                if fullname.contains(&peer.machine_name) {
                    peer.online = false;
                }
            }
        }
        ServiceEvent::SearchStarted(_) => {
            log::debug!("Busca mDNS iniciada");
        }
        ServiceEvent::SearchStopped(_) => {
            log::debug!("Busca mDNS encerrada");
        }
        _ => {}
    }
}
