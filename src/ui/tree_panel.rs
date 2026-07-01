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
}

impl Default for TreePanelState {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            search_results: Vec::new(),
            expanded_dirs: std::collections::HashSet::new(),
            status_message: None,
        }
    }
}

/// Renderiza o painel lateral esquerdo.
pub fn render_tree_panel(
    ui: &mut Ui,
    state: &mut TreePanelState,
    current_path: &mut PathBuf,
    profile_config: &mut ProfileConfig,
    profile_filter: &ProfileFilter,
) {
    // Sincroniza a árvore: expande todos os ancestrais do current_path
    ensure_path_expanded(state, current_path);

    ui.vertical(|ui| {
        // === BUSCA ===
        ui.horizontal(|ui| {
            ui.label("🔍");
            let response = ui.text_edit_singleline(&mut state.search_query);
            if response.changed() {
                if state.search_query.len() >= 2 {
                    state.search_results =
                        search::search_files(current_path, &state.search_query, 20);
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

        // === ÁRVORE (raiz = drives do computador) ===
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

/// Garante que todos os ancestrais do current_path estejam expandidos na árvore.
fn ensure_path_expanded(state: &mut TreePanelState, current_path: &Path) {
    let mut ancestor = current_path.to_path_buf();
    loop {
        if !state.expanded_dirs.contains(&ancestor) {
            state.expanded_dirs.insert(ancestor.clone());
        }
        if let Some(parent) = ancestor.parent() {
            if parent == ancestor {
                break; // Chegou na raiz (ex: C:\)
            }
            ancestor = parent.to_path_buf();
        } else {
            break;
        }
    }
}

/// Detecta os drives disponíveis no sistema (Windows).
fn get_system_drives() -> Vec<PathBuf> {
    let mut drives = Vec::new();
    for letter in b'A'..=b'Z' {
        let path = PathBuf::from(format!("{}:\\", letter as char));
        if path.exists() {
            drives.push(path);
        }
    }
    if drives.is_empty() {
        drives.push(PathBuf::from("C:\\"));
    }
    drives
}

/// Renderiza um nó de drive.
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
    if depth > 6 { return; }

    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
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
        let name = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if name.starts_with('.') || name.starts_with('$') { continue; }

        let is_expanded = state.expanded_dirs.contains(&dir);
        let is_current = *current_path == dir;
        let needs_profile = profile_config.needs_profile_assignment(&dir);
        let label = if needs_profile { format!("📁 {} ⚠", name) } else { format!("📁 {}", name) };

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
            if is_expanded { state.expanded_dirs.remove(&dir); }
            else { state.expanded_dirs.insert(dir.clone()); }
        }

        let dir_clone = dir.clone();
        header.header_response.context_menu(|ui| {
            render_tree_context_menu(ui, &dir_clone, profile_config, state);
        });
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
        let current_profiles: Vec<Profile> = profile_config.resolve_profiles(dir).into_iter().cloned().collect();
        let all = profile_config.all_profiles();
        for profile in all {
            let is_current = current_profiles.contains(&profile);
            let label = if is_current { format!("✓ {}", profile.label()) } else { profile.label().to_string() };
            if ui.selectable_label(is_current, label).clicked() {
                if is_current {
                    profile_config.unassign_profile(dir, &profile);
                    state.status_message = Some((format!("Perfil '{}' removido", profile.label()), std::time::Instant::now()));
                } else {
                    profile_config.assign_profile(dir.to_path_buf(), profile.clone());
                    state.status_message = Some((format!("Perfil '{}' atribuído", profile.label()), std::time::Instant::now()));
                }
                ui.close_menu();
            }
        }
        ui.separator();
        if !current_profiles.is_empty() && ui.button("❌ Remover todos").clicked() {
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

    ui.menu_button("ℹ Propriedades", |ui| {
        ui.label("Tipo: Diretório");
        if let Ok(entries) = std::fs::read_dir(dir) {
            ui.label(format!("Itens: {}", entries.count()));
        }
    });
    ui.separator();

    if ui.button(egui::RichText::new("🗑 Mover para lixeira").color(egui::Color32::from_rgb(220, 80, 80))).clicked() {
        let _ = trash::delete(dir);
        state.status_message = Some(("Excluído".to_string(), std::time::Instant::now()));
        ui.close_menu();
    }
}

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
