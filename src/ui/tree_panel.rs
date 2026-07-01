use crate::app::WorkspaceConfig;
use crate::core::profile::{Profile, ProfileConfig, ProfileFilter};
use crate::core::search;
use egui::Ui;
use std::path::{Path, PathBuf};

/// Estado do painel de árvore de diretórios.
pub struct TreePanelState {
    pub search_query: String,
    pub search_results: Vec<search::SearchResult>,
    pub expanded_dirs: std::collections::HashSet<PathBuf>,
    pub status_message: Option<(String, std::time::Instant)>,
    pub adding_workspace: bool,
    pub new_workspace_name: String,
    pub editing_workspace: Option<usize>,
    pub edit_workspace_name: String,
}

impl Default for TreePanelState {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            search_results: Vec::new(),
            expanded_dirs: std::collections::HashSet::new(),
            status_message: None,
            adding_workspace: false,
            new_workspace_name: String::new(),
            editing_workspace: None,
            edit_workspace_name: String::new(),
        }
    }
}

/// Renderiza o painel lateral esquerdo.
pub fn render_tree_panel(
    ui: &mut Ui,
    state: &mut TreePanelState,
    workspace_config: &mut WorkspaceConfig,
    current_path: &mut PathBuf,
    profile_config: &mut ProfileConfig,
    profile_filter: &ProfileFilter,
) {
    ui.vertical(|ui| {
        // === WORKSPACE SELECTOR (pasta aberta no painel central) ===
        render_workspace_selector(ui, state, workspace_config, current_path);

        ui.separator();

        // === BUSCA ===
        ui.horizontal(|ui| {
            ui.label("🔍");
            let response = ui.text_edit_singleline(&mut state.search_query);
            if response.changed() {
                if state.search_query.len() >= 2 {
                    let ws_path = workspace_config.active().path.clone();
                    state.search_results =
                        search::search_files(&ws_path, &state.search_query, 20);
                } else {
                    state.search_results.clear();
                }
            }
        });

        // Resultados de busca
        if !state.search_results.is_empty() {
            ui.label(egui::RichText::new(format!(
                "Resultados ({})", state.search_results.len()
            )).small());

            egui::ScrollArea::vertical()
                .max_height(150.0)
                .id_salt("search_scroll")
                .show(ui, |ui| {
                    for result in &state.search_results {
                        let icon = if result.is_dir { "📁" } else { "📄" };
                        if ui.selectable_label(false, format!("{} {}", icon, result.name)).clicked() {
                            if result.is_dir {
                                *current_path = result.path.clone();
                            } else if let Some(parent) = result.path.parent() {
                                *current_path = parent.to_path_buf();
                            }
                        }
                    }
                });
            ui.separator();
        }

        // Status message
        if let Some((ref msg, instant)) = state.status_message {
            if instant.elapsed().as_secs() < 3 {
                ui.label(egui::RichText::new(msg).small().color(
                    egui::Color32::from_rgb(100, 200, 100),
                ));
                ui.separator();
            }
        }

        // === ÁRVORE DE DIRETÓRIOS (raiz = drives do computador) ===
        ui.label(egui::RichText::new("Computador").strong());

        egui::ScrollArea::vertical()
            .id_salt("tree_scroll")
            .show(ui, |ui| {
                let drives = get_system_drives();
                for drive in &drives {
                    render_drive_node(ui, drive, current_path, state, profile_config, profile_filter);
                }
            });
    });
}

/// Detecta os drives disponíveis no sistema (Windows).
fn get_system_drives() -> Vec<PathBuf> {
    let mut drives = Vec::new();
    // Em Windows, verifica letras A-Z
    for letter in b'A'..=b'Z' {
        let path = PathBuf::from(format!("{}:\\", letter as char));
        if path.exists() {
            drives.push(path);
        }
    }
    // Fallback se nenhum drive encontrado
    if drives.is_empty() {
        drives.push(PathBuf::from("C:\\"));
    }
    drives
}

/// Renderiza um nó de drive na árvore.
fn render_drive_node(
    ui: &mut Ui,
    drive: &Path,
    current_path: &mut PathBuf,
    state: &mut TreePanelState,
    profile_config: &mut ProfileConfig,
    profile_filter: &ProfileFilter,
) {
    let drive_label = drive.to_string_lossy().to_string();
    let is_expanded = state.expanded_dirs.contains(&drive.to_path_buf());
    let is_current = *current_path == drive;

    let label = format!("💾 {}", drive_label);
    let header = egui::CollapsingHeader::new(
        egui::RichText::new(&label).color(if is_current {
            egui::Color32::from_rgb(100, 180, 255)
        } else {
            ui.style().visuals.text_color()
        }),
    )
    .default_open(is_expanded)
    .show(ui, |ui| {
        render_dir_tree(ui, drive, current_path, state, profile_config, profile_filter, 0);
    });

    if header.header_response.clicked() {
        *current_path = drive.to_path_buf();
        if is_expanded {
            state.expanded_dirs.remove(&drive.to_path_buf());
        } else {
            state.expanded_dirs.insert(drive.to_path_buf());
        }
    }

    header.header_response.context_menu(|ui| {
        render_tree_context_menu(ui, drive, profile_config, state);
    });
}

/// Renderiza recursivamente a árvore de subdiretórios.
fn render_dir_tree(
    ui: &mut Ui,
    path: &Path,
    current_path: &mut PathBuf,
    state: &mut TreePanelState,
    profile_config: &mut ProfileConfig,
    profile_filter: &ProfileFilter,
    depth: usize,
) {
    if depth > 6 {
        return;
    }

    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| profile_config.is_visible_filtered(&e.path(), profile_filter))
        .map(|e| e.path())
        .collect();

    dirs.sort();

    for dir in dirs {
        let name = dir.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Ignora diretórios ocultos/sistema
        if name.starts_with('.') || name.starts_with('$') {
            continue;
        }

        let is_expanded = state.expanded_dirs.contains(&dir);
        let is_current = *current_path == dir;

        let needs_profile = profile_config.needs_profile_assignment(&dir);
        let label = if needs_profile {
            format!("📁 {} ⚠", name)
        } else {
            format!("📁 {}", name)
        };

        let header = egui::CollapsingHeader::new(
            egui::RichText::new(&label).color(if is_current {
                egui::Color32::from_rgb(100, 180, 255)
            } else {
                ui.style().visuals.text_color()
            }),
        )
        .default_open(is_expanded)
        .show(ui, |ui| {
            render_dir_tree(ui, &dir, current_path, state, profile_config, profile_filter, depth + 1);
        });

        if header.header_response.clicked() {
            *current_path = dir.clone();
            if is_expanded {
                state.expanded_dirs.remove(&dir);
            } else {
                state.expanded_dirs.insert(dir.clone());
            }
        }

        let dir_clone = dir.clone();
        header.header_response.context_menu(|ui| {
            render_tree_context_menu(ui, &dir_clone, profile_config, state);
        });
    }
}

/// Seletor de workspace — a pasta raiz exibida no painel central.
fn render_workspace_selector(
    ui: &mut Ui,
    state: &mut TreePanelState,
    workspace_config: &mut WorkspaceConfig,
    current_path: &mut PathBuf,
) {
    // Título + botão adicionar
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("⚡ Workspace").strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("➕").on_hover_text("Adicionar workspace").clicked() {
                state.adding_workspace = true;
                state.new_workspace_name.clear();
            }
        });
    });

    // Dropdown para selecionar workspace
    let active_name = workspace_config.active().name.clone();
    let ws_list: Vec<(usize, String)> = workspace_config.workspaces.iter()
        .enumerate().map(|(i, ws)| (i, ws.name.clone())).collect();
    let active_idx = workspace_config.active_index;

    egui::ComboBox::from_id_salt("workspace_combo")
        .selected_text(&active_name)
        .width(ui.available_width() - 4.0)
        .show_ui(ui, |ui| {
            for (i, name) in &ws_list {
                if ui.selectable_label(*i == active_idx, format!("📁 {}", name)).clicked() {
                    workspace_config.set_active(*i);
                    *current_path = workspace_config.active().path.clone();
                    state.expanded_dirs.clear();
                }
            }
        });

    // Botões de ação no workspace ativo
    ui.horizontal(|ui| {
        if ui.small_button("✏").on_hover_text("Renomear").clicked() {
            state.editing_workspace = Some(workspace_config.active_index);
            state.edit_workspace_name = workspace_config.active().name.clone();
        }
        if ui.small_button("📂").on_hover_text("Mudar pasta").clicked() {
            if let Some(folder) = rfd::FileDialog::new()
                .set_directory(&workspace_config.active().path)
                .pick_folder()
            {
                workspace_config.workspaces[workspace_config.active_index].path = folder.clone();
                *current_path = folder;
                state.expanded_dirs.clear();
            }
        }
        if workspace_config.workspaces.len() > 1 {
            if ui.small_button("🗑").on_hover_text("Remover").clicked() {
                workspace_config.remove(workspace_config.active_index);
                *current_path = workspace_config.active().path.clone();
                state.expanded_dirs.clear();
            }
        }

        // Atalho: navegar para o workspace no painel central
        if ui.small_button("→").on_hover_text("Ir para workspace").clicked() {
            *current_path = workspace_config.active().path.clone();
        }
    });

    // Diálogos
    if state.adding_workspace {
        render_add_workspace_dialog(ui, state, workspace_config, current_path);
    }
    if state.editing_workspace.is_some() {
        render_edit_workspace_dialog(ui, state, workspace_config);
    }
}

/// Menu de contexto para diretórios na árvore.
fn render_tree_context_menu(
    ui: &mut Ui,
    dir: &Path,
    profile_config: &mut ProfileConfig,
    state: &mut TreePanelState,
) {
    let name = dir.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| dir.to_string_lossy().to_string());

    ui.label(egui::RichText::new(&name).strong());
    ui.separator();

    if ui.button("📂 Abrir no explorador").clicked() {
        let _ = open::that(dir);
        ui.close_menu();
    }
    ui.separator();

    if ui.button("📁 Nova pasta aqui").clicked() {
        create_item_in_dir(dir, true, state);
        ui.close_menu();
    }
    if ui.button("📄 Novo arquivo aqui").clicked() {
        create_item_in_dir(dir, false, state);
        ui.close_menu();
    }
    ui.separator();

    if ui.button("📎 Copiar caminho").clicked() {
        ui.ctx().copy_text(dir.to_string_lossy().to_string());
        state.status_message = Some(("Caminho copiado".to_string(), std::time::Instant::now()));
        ui.close_menu();
    }
    ui.separator();

    // Perfil
    ui.menu_button("👤 Definir perfil", |ui| {
        let current_profile = profile_config.resolve_profile(dir).cloned();
        let all = profile_config.all_profiles();
        for profile in all {
            let is_current = current_profile.as_ref() == Some(&profile);
            let label = if is_current { format!("✓ {}", profile.label()) } else { profile.label().to_string() };
            if ui.selectable_label(is_current, label).clicked() {
                profile_config.assign_profile(dir.to_path_buf(), profile.clone());
                state.status_message = Some((format!("Perfil '{}' atribuído", profile.label()), std::time::Instant::now()));
                ui.close_menu();
            }
        }
        ui.separator();
        if current_profile.is_some() && ui.button("❌ Remover perfil").clicked() {
            profile_config.remove_profile(dir);
            ui.close_menu();
        }
    });
    ui.separator();

    // Proteger
    let is_secure = profile_config.secure_directories.contains(&dir.to_path_buf());
    if is_secure {
        if ui.button("🔓 Remover proteção de rede").clicked() {
            profile_config.secure_directories.retain(|p| p != dir);
            ui.close_menu();
        }
    } else if ui.button("🔒 Proteger (não compartilhar)").clicked() {
        profile_config.secure_directories.push(dir.to_path_buf());
        ui.close_menu();
    }
    ui.separator();

    // Propriedades
    ui.menu_button("ℹ Propriedades", |ui| {
        ui.label("Tipo: Diretório");
        if let Ok(entries) = std::fs::read_dir(dir) {
            ui.label(format!("Itens: {}", entries.count()));
        }
    });
    ui.separator();

    if ui.button(egui::RichText::new("🗑 Mover para lixeira").color(
        egui::Color32::from_rgb(220, 80, 80)
    )).clicked() {
        let _ = trash::delete(dir);
        state.status_message = Some(("Excluído".to_string(), std::time::Instant::now()));
        ui.close_menu();
    }
}

/// Cria um item dentro de um diretório.
fn create_item_in_dir(dir: &Path, is_folder: bool, state: &mut TreePanelState) {
    let base_name = if is_folder { "Nova pasta" } else { "Novo arquivo.txt" };
    let mut path = dir.join(base_name);
    let mut counter = 2;
    while path.exists() {
        let name = if is_folder { format!("Nova pasta ({})", counter) } else { format!("Novo arquivo ({}).txt", counter) };
        path = dir.join(name);
        counter += 1;
    }
    let result = if is_folder { std::fs::create_dir_all(&path) } else { std::fs::write(&path, "") };
    match result {
        Ok(_) => { state.status_message = Some(("Criado".to_string(), std::time::Instant::now())); }
        Err(_) => { state.status_message = Some(("Erro ao criar".to_string(), std::time::Instant::now())); }
    }
}

/// Diálogo para adicionar workspace.
fn render_add_workspace_dialog(
    ui: &mut Ui,
    state: &mut TreePanelState,
    workspace_config: &mut WorkspaceConfig,
    current_path: &mut PathBuf,
) {
    egui::Window::new("Novo Workspace")
        .collapsible(false)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Nome:");
                ui.text_edit_singleline(&mut state.new_workspace_name);
            });
            ui.horizontal(|ui| {
                if ui.button("📁 Escolher pasta").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        let name = if state.new_workspace_name.is_empty() {
                            folder.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "Workspace".to_string())
                        } else {
                            state.new_workspace_name.clone()
                        };
                        workspace_config.add(name, folder.clone());
                        workspace_config.set_active(workspace_config.workspaces.len() - 1);
                        *current_path = folder;
                        state.expanded_dirs.clear();
                        state.adding_workspace = false;
                    }
                }
                if ui.button("✗ Cancelar").clicked() {
                    state.adding_workspace = false;
                }
            });
        });
}

/// Diálogo para renomear workspace.
fn render_edit_workspace_dialog(
    ui: &mut Ui,
    state: &mut TreePanelState,
    workspace_config: &mut WorkspaceConfig,
) {
    egui::Window::new("Renomear Workspace")
        .collapsible(false)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Nome:");
                let resp = ui.text_edit_singleline(&mut state.edit_workspace_name);
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if let Some(idx) = state.editing_workspace {
                        if !state.edit_workspace_name.is_empty() {
                            workspace_config.workspaces[idx].name = state.edit_workspace_name.clone();
                        }
                    }
                    state.editing_workspace = None;
                }
            });
            ui.horizontal(|ui| {
                if ui.button("✓ Confirmar").clicked() {
                    if let Some(idx) = state.editing_workspace {
                        if !state.edit_workspace_name.is_empty() {
                            workspace_config.workspaces[idx].name = state.edit_workspace_name.clone();
                        }
                    }
                    state.editing_workspace = None;
                }
                if ui.button("✗ Cancelar").clicked() {
                    state.editing_workspace = None;
                }
            });
        });
}
