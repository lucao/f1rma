use crate::core::annotations::AnnotationStore;
use crate::core::file_ops::OperationHistory;
use crate::core::profile::{Profile, ProfileConfig, ProfileFilter};
use crate::network::discovery::{DiscoveryService, DiscoveryState};
use crate::network::share::ShareConfig;
use crate::ui::header::HeaderState;
use crate::ui::main_panel::{self, MainPanelState};
use crate::ui::preview_panel::PreviewPanelState;
use crate::ui::tree_panel::TreePanelState;
use std::path::PathBuf;

/// Histórico de navegação para voltar/avançar.
#[derive(Debug, Clone, Default)]
pub struct NavigationHistory {
    pub back_stack: Vec<PathBuf>,
    pub forward_stack: Vec<PathBuf>,
}

impl NavigationHistory {
    pub fn push(&mut self, previous_path: PathBuf) {
        self.back_stack.push(previous_path);
        self.forward_stack.clear();
    }

    pub fn go_back(&mut self, current_path: &PathBuf) -> Option<PathBuf> {
        if let Some(prev) = self.back_stack.pop() {
            self.forward_stack.push(current_path.clone());
            Some(prev)
        } else {
            None
        }
    }

    pub fn go_forward(&mut self, current_path: &PathBuf) -> Option<PathBuf> {
        if let Some(next) = self.forward_stack.pop() {
            self.back_stack.push(current_path.clone());
            Some(next)
        } else {
            None
        }
    }

    pub fn can_go_back(&self) -> bool {
        !self.back_stack.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward_stack.is_empty()
    }
}

/// Uma ação que pode ser desfeita.
#[derive(Debug, Clone)]
pub enum UndoAction {
    Trash { original_path: PathBuf },
    Rename { old_path: PathBuf, new_path: PathBuf },
    Create { path: PathBuf },
    Copy { destination: PathBuf },
    Move { source: PathBuf, destination: PathBuf },
}

/// Pilha de undo.
#[derive(Debug, Clone, Default)]
pub struct UndoStack {
    pub actions: Vec<UndoAction>,
    pub max_size: usize,
}

impl UndoStack {
    pub fn new(max_size: usize) -> Self {
        Self { actions: Vec::new(), max_size }
    }

    pub fn push(&mut self, action: UndoAction) {
        self.actions.push(action);
        if self.actions.len() > self.max_size {
            self.actions.remove(0);
        }
    }

    pub fn undo(&mut self) -> Option<String> {
        let action = self.actions.pop()?;
        match action {
            UndoAction::Rename { old_path, new_path } => {
                if new_path.exists() {
                    let _ = std::fs::rename(&new_path, &old_path);
                    Some(format!("Desfeito: renomear → {}", old_path.display()))
                } else {
                    Some("Erro: arquivo não encontrado".to_string())
                }
            }
            UndoAction::Create { path } => {
                if path.exists() {
                    let _ = trash::delete(&path);
                    Some(format!("Desfeito: criação de {}", path.display()))
                } else {
                    Some("Erro: arquivo já não existe".to_string())
                }
            }
            UndoAction::Copy { destination } => {
                if destination.exists() {
                    let _ = trash::delete(&destination);
                    Some("Desfeito: cópia removida".to_string())
                } else {
                    Some("Erro: cópia não encontrada".to_string())
                }
            }
            UndoAction::Move { source, destination } => {
                if destination.exists() {
                    let _ = std::fs::rename(&destination, &source);
                    Some("Desfeito: movido de volta".to_string())
                } else {
                    Some("Erro: arquivo não encontrado".to_string())
                }
            }
            UndoAction::Trash { original_path } => {
                Some(format!("Verifique a lixeira: {}", original_path.display()))
            }
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.actions.is_empty()
    }

    pub fn last_action_label(&self) -> Option<String> {
        self.actions.last().map(|a| match a {
            UndoAction::Trash { original_path } => format!("Excluir {}", original_path.file_name().unwrap_or_default().to_string_lossy()),
            UndoAction::Rename { old_path, .. } => format!("Renomear {}", old_path.file_name().unwrap_or_default().to_string_lossy()),
            UndoAction::Create { path } => format!("Criar {}", path.file_name().unwrap_or_default().to_string_lossy()),
            UndoAction::Copy { destination } => format!("Copiar {}", destination.file_name().unwrap_or_default().to_string_lossy()),
            UndoAction::Move { source, .. } => format!("Mover {}", source.file_name().unwrap_or_default().to_string_lossy()),
        })
    }
}

/// Aplicação principal F1RMA.
pub struct F1rmaApp {
    pub current_path: PathBuf,
    pub profile_filter: ProfileFilter,
    pub nav_history: NavigationHistory,
    pub undo_stack: UndoStack,

    pub profile_config: ProfileConfig,
    pub share_config: ShareConfig,
    pub annotations: AnnotationStore,
    pub operation_history: OperationHistory,

    pub tree_state: TreePanelState,
    pub main_state: MainPanelState,
    pub preview_state: PreviewPanelState,
    pub header_state: HeaderState,

    pub discovery_state: DiscoveryState,
    pub discovery_service: Option<DiscoveryService>,

    pub needs_refresh: bool,
}

impl F1rmaApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config_dir = config_dir();

        let profile_config = load_config::<ProfileConfig>(&config_dir.join("profiles.json"))
            .unwrap_or_default();
        let share_config = load_config::<ShareConfig>(&config_dir.join("share.json"))
            .unwrap_or_default();
        let annotations =
            AnnotationStore::load(&config_dir.join("annotations.json")).unwrap_or_default();

        let current_path = directories::UserDirs::new()
            .map(|d| d.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("C:\\"));

        let mut app = Self {
            current_path,
            profile_filter: ProfileFilter::none(),
            nav_history: NavigationHistory::default(),
            undo_stack: UndoStack::new(50),
            profile_config,
            share_config,
            annotations,
            operation_history: OperationHistory::new(100),
            tree_state: TreePanelState::default(),
            main_state: MainPanelState::default(),
            preview_state: PreviewPanelState::default(),
            header_state: HeaderState::default(),
            discovery_state: DiscoveryState::new(),
            discovery_service: None,
            needs_refresh: true,
        };

        app.refresh_directory();
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
            Ok(_) => { self.discovery_service = Some(service); }
            Err(e) => { log::error!("Falha ao iniciar discovery: {}", e); }
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
        if self.needs_refresh {
            self.refresh_directory();
        }

        // Atalhos globais
        if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft)) {
            if let Some(path) = self.nav_history.go_back(&self.current_path) {
                self.current_path = path;
                self.needs_refresh = true;
            }
        }
        if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight)) {
            if let Some(path) = self.nav_history.go_forward(&self.current_path) {
                self.current_path = path;
                self.needs_refresh = true;
            }
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
            if let Some(_msg) = self.undo_stack.undo() {
                self.needs_refresh = true;
            }
        }

        let prev_path = self.current_path.clone();
        let prev_filter = self.profile_filter.clone();

        // === HEADER ===
        egui::TopBottomPanel::top("header_panel")
            .min_height(36.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                let profiles = self.profile_config.all_profiles();
                let mut new_profile_name: Option<String> = None;
                crate::ui::header::render_header(
                    ui,
                    &mut self.profile_filter,
                    &mut self.current_path,
                    &self.share_config.machine_name,
                    &self.discovery_state,
                    &profiles,
                    &mut self.header_state,
                    &mut new_profile_name,
                );
                if let Some(name) = new_profile_name {
                    self.profile_config.registry.add_custom(name);
                }

                // Navegação
                ui.horizontal(|ui| {
                    let back = ui.add_enabled(self.nav_history.can_go_back(), egui::Button::new("⬅ Voltar"));
                    if back.clicked() {
                        if let Some(p) = self.nav_history.go_back(&self.current_path) {
                            self.current_path = p;
                            self.needs_refresh = true;
                        }
                    }
                    let fwd = ui.add_enabled(self.nav_history.can_go_forward(), egui::Button::new("Avançar ➡"));
                    if fwd.clicked() {
                        if let Some(p) = self.nav_history.go_forward(&self.current_path) {
                            self.current_path = p;
                            self.needs_refresh = true;
                        }
                    }
                    ui.separator();
                    let undo_label = self.undo_stack.last_action_label()
                        .map(|l| format!("↩ Desfazer: {}", l))
                        .unwrap_or_else(|| "↩ Desfazer".to_string());
                    let undo = ui.add_enabled(self.undo_stack.can_undo(), egui::Button::new(&undo_label));
                    if undo.clicked() {
                        if let Some(_) = self.undo_stack.undo() {
                            self.needs_refresh = true;
                        }
                    }
                });
                ui.add_space(4.0);
            });

        // === FOOTER ===
        egui::TopBottomPanel::bottom("footer_panel")
            .min_height(28.0)
            .show(ctx, |ui| {
                ui.add_space(2.0);
                crate::ui::footer::render_footer(ui, &self.operation_history);
                ui.add_space(2.0);
            });

        // === NETWORK PANEL ===
        egui::TopBottomPanel::bottom("network_panel")
            .min_height(100.0)
            .max_height(300.0)
            .resizable(true)
            .default_height(150.0)
            .show(ctx, |ui| {
                crate::ui::network_panel::render_network_panel(ui, &self.discovery_state);
            });

        // === TREE (left) ===
        egui::SidePanel::left("tree_panel")
            .default_width(220.0)
            .min_width(150.0)
            .max_width(400.0)
            .resizable(true)
            .show(ctx, |ui| {
                crate::ui::tree_panel::render_tree_panel(
                    ui,
                    &mut self.tree_state,
                    &mut self.current_path,
                    &mut self.profile_config,
                    &self.profile_filter,
                );
            });

        // === PREVIEW (right) ===
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

        // === CENTRAL (files) ===
        egui::CentralPanel::default().show(ctx, |ui| {
            crate::ui::main_panel::render_main_panel(
                ui,
                &mut self.main_state,
                &mut self.current_path,
                &mut self.profile_config,
                &self.profile_filter,
            );
        });

        // Detecta mudanças
        if self.current_path != prev_path {
            self.nav_history.push(prev_path);
            self.refresh_directory();
        } else if self.profile_filter.active != prev_filter.active {
            self.refresh_directory();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(ref mut service) = self.discovery_service {
            service.stop();
        }
        self.save_configs();
    }
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
