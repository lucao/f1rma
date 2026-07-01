use crate::core::annotations::AnnotationStore;
use crate::core::file_ops::OperationHistory;
use crate::core::profile::{Profile, ProfileConfig, ProfileFilter};
use crate::network::discovery::{DiscoveryService, DiscoveryState};
use crate::network::share::ShareConfig;
use crate::ui::main_panel::{self, MainPanelState};
use crate::ui::preview_panel::PreviewPanelState;
use crate::ui::tree_panel::TreePanelState;
use std::path::PathBuf;

/// Aplicação principal F1RMA.
pub struct F1rmaApp {
    // Estado de navegação
    pub current_path: PathBuf,
    pub profile_filter: ProfileFilter,

    // Configurações
    pub profile_config: ProfileConfig,
    pub share_config: ShareConfig,

    // Anotações
    pub annotations: AnnotationStore,

    // Operações de arquivo
    pub operation_history: OperationHistory,

    // Estado dos painéis de UI
    pub tree_state: TreePanelState,
    pub main_state: MainPanelState,
    pub preview_state: PreviewPanelState,

    // Rede / Discovery
    pub discovery_state: DiscoveryState,
    pub discovery_service: Option<DiscoveryService>,

    // Cache
    pub needs_refresh: bool,
}

impl F1rmaApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let current_path = dirs_home().unwrap_or_else(|| PathBuf::from("C:\\"));
        let config_dir = config_dir();

        // Carrega configurações salvas
        let profile_config = load_config::<ProfileConfig>(&config_dir.join("profiles.json"))
            .unwrap_or_default();
        let share_config = load_config::<ShareConfig>(&config_dir.join("share.json"))
            .unwrap_or_default();
        let annotations =
            AnnotationStore::load(&config_dir.join("annotations.json")).unwrap_or_default();

        let mut app = Self {
            current_path: current_path.clone(),
            profile_filter: ProfileFilter::none(),
            profile_config,
            share_config,
            annotations,
            operation_history: OperationHistory::new(100),
            tree_state: TreePanelState::default(),
            main_state: MainPanelState::default(),
            preview_state: PreviewPanelState::default(),
            discovery_state: DiscoveryState::new(),
            discovery_service: None,
            needs_refresh: true,
        };

        // Carrega o diretório inicial
        app.refresh_directory();

        // Inicia o serviço de discovery na rede
        app.start_discovery();

        app
    }

    fn start_discovery(&mut self) {
        let state = self.discovery_state.clone();
        let mut service = DiscoveryService::new(state, self.share_config.machine_id.clone());

        match service.start(
            &self.share_config.machine_name,
            &self.share_config.machine_id,
            &self.share_config.username,
            self.share_config.port,
        ) {
            Ok(_) => {
                log::info!("Discovery mDNS iniciado com sucesso");
                self.discovery_service = Some(service);
            }
            Err(e) => {
                log::error!("Falha ao iniciar discovery: {}", e);
            }
        }
    }

    fn refresh_directory(&mut self) {
        self.main_state.entries = main_panel::load_directory(
            &self.current_path,
            &self.profile_config,
            &self.profile_filter,
        );
        self.needs_refresh = false;
    }

    fn save_configs(&self) {
        let config_dir = config_dir();
        let _ = std::fs::create_dir_all(&config_dir);
        let _ = save_config(&config_dir.join("profiles.json"), &self.profile_config);
        let _ = save_config(&config_dir.join("share.json"), &self.share_config);
        let _ = self.annotations.save(&config_dir.join("annotations.json"));
    }
}

impl eframe::App for F1rmaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Refresh se necessário
        if self.needs_refresh {
            self.refresh_directory();
        }

        let prev_path = self.current_path.clone();
        let prev_filter = self.profile_filter.clone();

        // === CABEÇALHO (topo) ===
        egui::TopBottomPanel::top("header_panel")
            .min_height(36.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                crate::ui::header::render_header(
                    ui,
                    &mut self.profile_filter,
                    &self.current_path,
                    &self.share_config.machine_name,
                    &self.discovery_state,
                );
                ui.add_space(4.0);
            });

        // === RODAPÉ (base) ===
        egui::TopBottomPanel::bottom("footer_panel")
            .min_height(28.0)
            .show(ctx, |ui| {
                ui.add_space(2.0);
                crate::ui::footer::render_footer(ui, &self.operation_history);
                ui.add_space(2.0);
            });

        // === PAINEL ESQUERDO (árvore de diretórios) ===
        egui::SidePanel::left("tree_panel")
            .default_width(220.0)
            .min_width(150.0)
            .max_width(400.0)
            .resizable(true)
            .show(ctx, |ui| {
                crate::ui::tree_panel::render_tree_panel(
                    ui,
                    &mut self.tree_state,
                    &dirs_home().unwrap_or_else(|| PathBuf::from("C:\\")),
                    &mut self.current_path,
                    &mut self.profile_config,
                    &self.profile_filter,
                );
            });

        // === PAINEL DIREITO (preview + anotações) ===
        egui::SidePanel::right("preview_panel")
            .default_width(280.0)
            .min_width(200.0)
            .max_width(500.0)
            .resizable(true)
            .show(ctx, |ui| {
                crate::ui::preview_panel::render_preview_panel(
                    ui,
                    self.main_state.selected_file.as_ref(),
                    &mut self.preview_state,
                    &mut self.annotations,
                    &self.share_config.username,
                    &self.share_config.machine_id,
                );
            });

        // === PAINEL CENTRAL (arquivos) ===
        egui::CentralPanel::default().show(ctx, |ui| {
            crate::ui::main_panel::render_main_panel(
                ui,
                &mut self.main_state,
                &mut self.current_path,
                &mut self.profile_config,
                &self.profile_filter,
            );
        });

        // Detecta mudanças de diretório ou perfil para atualizar
        if self.current_path != prev_path || self.profile_filter.active != prev_filter.active {
            self.refresh_directory();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Para o discovery mDNS
        if let Some(ref mut service) = self.discovery_service {
            service.stop();
        }
        self.save_configs();
    }
}

// === Utilitários ===

fn dirs_home() -> Option<PathBuf> {
    directories::UserDirs::new().map(|d| d.home_dir().to_path_buf())
}

fn config_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "f1rma", "F1RMA")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".f1rma"))
}

fn load_config<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Option<T> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_config<T: serde::Serialize>(path: &PathBuf, config: &T) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, json)
}
