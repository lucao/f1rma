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

/// Renderiza o painel lateral esquerdo com árvore de diretórios e busca.
pub fn render_tree_panel(
    ui: &mut Ui,
    state: &mut TreePanelState,
    root_path: &Path,
    current_path: &mut PathBuf,
    profile_config: &mut ProfileConfig,
    profile_filter: &ProfileFilter,
) {
    ui.vertical(|ui| {
        // Campo de busca
        ui.horizontal(|ui| {
            ui.label("🔍");
            let response = ui.text_edit_singleline(&mut state.search_query);
            if response.changed() {
                if state.search_query.len() >= 2 {
                    state.search_results =
                        search::search_files(root_path, &state.search_query, 20);
                } else {
                    state.search_results.clear();
                }
            }
        });

        ui.separator();

        // Resultados de busca (se houver)
        if !state.search_results.is_empty() {
            ui.label(
                egui::RichText::new(format!(
                    "Resultados ({})",
                    state.search_results.len()
                ))
                .small(),
            );

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for result in &state.search_results {
                        let icon = if result.is_dir { "📁" } else { "📄" };
                        if ui
                            .selectable_label(false, format!("{} {}", icon, result.name))
                            .clicked()
                        {
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

        // Árvore de diretórios
        ui.label(egui::RichText::new("Diretórios").strong());

        egui::ScrollArea::vertical().show(ui, |ui| {
            render_dir_tree(
                ui,
                root_path,
                current_path,
                state,
                profile_config,
                profile_filter,
                0,
            );
        });
    });
}

fn render_dir_tree(
    ui: &mut Ui,
    path: &Path,
    current_path: &mut PathBuf,
    state: &mut TreePanelState,
    profile_config: &mut ProfileConfig,
    profile_filter: &ProfileFilter,
    depth: usize,
) {
    if depth > 8 {
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
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Ignora diretórios ocultos do sistema
        if name.starts_with('.') {
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
            render_dir_tree(
                ui,
                &dir,
                current_path,
                state,
                profile_config,
                profile_filter,
                depth + 1,
            );
        });

        // Clique esquerdo: navegar
        if header.header_response.clicked() {
            *current_path = dir.clone();
            if is_expanded {
                state.expanded_dirs.remove(&dir);
            } else {
                state.expanded_dirs.insert(dir.clone());
            }
        }

        // Menu de contexto (botão direito)
        let dir_clone = dir.clone();
        header.header_response.context_menu(|ui| {
            render_tree_context_menu(ui, &dir_clone, profile_config, state);
        });
    }
}

/// Menu de contexto para diretórios na árvore lateral.
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

    // Abrir no explorador
    if ui.button("📂 Abrir no explorador").clicked() {
        let _ = open::that(dir);
        ui.close_menu();
    }

    ui.separator();

    // Nova pasta / Novo arquivo dentro deste diretório
    if ui.button("📁 Nova pasta aqui").clicked() {
        create_item_in_dir(dir, true, state);
        ui.close_menu();
    }
    if ui.button("📄 Novo arquivo aqui").clicked() {
        create_item_in_dir(dir, false, state);
        ui.close_menu();
    }

    ui.separator();

    // Copiar caminho
    if ui.button("📎 Copiar caminho").clicked() {
        ui.ctx().copy_text(dir.to_string_lossy().to_string());
        state.status_message = Some(("Caminho copiado".to_string(), std::time::Instant::now()));
        ui.close_menu();
    }

    // Renomear
    if ui.button("✏ Renomear").clicked() {
        let new_name = format!("{}_renomeado", name);
        if let Some(parent) = dir.parent() {
            let new_path = parent.join(&new_name);
            if !new_path.exists() {
                let _ = std::fs::rename(dir, &new_path);
                state.status_message = Some(("Renomeado".to_string(), std::time::Instant::now()));
            }
        }
        ui.close_menu();
    }

    ui.separator();

    // Definir perfil
    ui.menu_button("👤 Definir perfil", |ui| {
        let current_profile = profile_config.resolve_profile(dir);
        for profile in Profile::all() {
            let is_current = current_profile == Some(*profile);
            let label = if is_current {
                format!("✓ {}", profile.label())
            } else {
                profile.label().to_string()
            };
            if ui.selectable_label(is_current, label).clicked() {
                profile_config.assign_profile(dir.to_path_buf(), *profile);
                state.status_message = Some((
                    format!("Perfil '{}' atribuído", profile.label()),
                    std::time::Instant::now(),
                ));
                ui.close_menu();
            }
        }
        ui.separator();
        if current_profile.is_some() && ui.button("❌ Remover perfil").clicked() {
            profile_config.remove_profile(dir);
            state.status_message = Some(("Perfil removido".to_string(), std::time::Instant::now()));
            ui.close_menu();
        }
    });

    ui.separator();

    // Proteger / Desproteger (rede)
    let is_secure = profile_config.secure_directories.contains(&dir.to_path_buf());
    if is_secure {
        if ui.button("🔓 Remover proteção de rede").clicked() {
            profile_config.secure_directories.retain(|p| p != dir);
            state.status_message = Some(("Proteção removida".to_string(), std::time::Instant::now()));
            ui.close_menu();
        }
    } else if ui.button("🔒 Proteger (não compartilhar)").clicked() {
        profile_config.secure_directories.push(dir.to_path_buf());
        state.status_message = Some(("Pasta protegida".to_string(), std::time::Instant::now()));
        ui.close_menu();
    }

    ui.separator();

    // Propriedades
    ui.menu_button("ℹ Propriedades", |ui| {
        ui.label("Tipo: Diretório");
        if let Ok(entries) = std::fs::read_dir(dir) {
            ui.label(format!("Itens: {}", entries.count()));
        }
        if let Ok(metadata) = std::fs::metadata(dir) {
            if let Ok(m) = metadata.modified() {
                if let Ok(d) = m.elapsed() {
                    let secs = d.as_secs();
                    let ago = if secs < 60 { "agora".to_string() }
                        else if secs < 3600 { format!("{}min atrás", secs / 60) }
                        else if secs < 86400 { format!("{}h atrás", secs / 3600) }
                        else { format!("{}d atrás", secs / 86400) };
                    ui.label(format!("Modificado: {}", ago));
                }
            }
        }
    });

    ui.separator();

    // Excluir
    if ui.button(
        egui::RichText::new("🗑 Mover para lixeira")
            .color(egui::Color32::from_rgb(220, 80, 80)),
    ).clicked() {
        let _ = trash::delete(dir);
        state.status_message = Some(("Movido para lixeira".to_string(), std::time::Instant::now()));
        ui.close_menu();
    }
}

/// Cria um item diretamente (usa um nome padrão que o usuário pode renomear depois).
fn create_item_in_dir(dir: &Path, is_folder: bool, state: &mut TreePanelState) {
    let base_name = if is_folder { "Nova pasta" } else { "Novo arquivo.txt" };
    let mut path = dir.join(base_name);
    let mut counter = 2;
    while path.exists() {
        let name = if is_folder {
            format!("Nova pasta ({})", counter)
        } else {
            format!("Novo arquivo ({}).txt", counter)
        };
        path = dir.join(name);
        counter += 1;
    }
    let result = if is_folder {
        std::fs::create_dir_all(&path)
    } else {
        std::fs::write(&path, "")
    };
    match result {
        Ok(_) => {
            let kind = if is_folder { "Pasta" } else { "Arquivo" };
            state.status_message = Some((format!("{} criado", kind), std::time::Instant::now()));
        }
        Err(_) => {
            state.status_message = Some(("Erro ao criar".to_string(), std::time::Instant::now()));
        }
    }
}
