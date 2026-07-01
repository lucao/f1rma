use crate::core::profile::{Profile, ProfileConfig, ProfileFilter};
use egui::Ui;
use std::path::{Path, PathBuf};

/// Ação pendente no clipboard interno do gerenciador.
#[derive(Debug, Clone, PartialEq)]
pub enum ClipboardAction {
    Copy,
    Cut,
}

/// Estado do clipboard para operações de copiar/colar.
#[derive(Debug, Clone)]
pub struct FileClipboard {
    pub paths: Vec<PathBuf>,
    pub action: ClipboardAction,
}

/// Ação solicitada pelo menu de contexto que precisa ser processada externamente.
#[derive(Debug, Clone)]
pub enum ContextAction {
    None,
    Rename(PathBuf),
    NewFolder(PathBuf),
    NewFile(PathBuf),
    Refresh,
}

/// Estado do diálogo de renomear.
#[derive(Debug, Clone, Default)]
pub struct RenameDialogState {
    pub active: bool,
    pub path: Option<PathBuf>,
    pub new_name: String,
}

/// Estado do diálogo de novo arquivo/pasta.
#[derive(Debug, Clone, Default)]
pub struct NewItemDialogState {
    pub active: bool,
    pub is_folder: bool,
    pub name: String,
    pub parent: Option<PathBuf>,
}

/// Modos de exibição dos arquivos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    List,
    Icons,
    Details,
    Compact,
}

impl ViewMode {
    pub fn label(&self) -> &'static str {
        match self {
            ViewMode::List => "Lista",
            ViewMode::Icons => "Ícones",
            ViewMode::Details => "Detalhes",
            ViewMode::Compact => "Compacto",
        }
    }

    pub fn all() -> &'static [ViewMode] {
        &[
            ViewMode::List,
            ViewMode::Icons,
            ViewMode::Details,
            ViewMode::Compact,
        ]
    }
}

/// Informações de um item no diretório.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
    pub needs_profile: bool,
    pub profiles: Vec<Profile>,
}

/// Estado do painel principal.
pub struct MainPanelState {
    pub view_mode: ViewMode,
    pub selected_file: Option<PathBuf>,
    pub selected_files: Vec<PathBuf>,
    pub entries: Vec<FileEntry>,
    pub sort_by: SortField,
    pub sort_ascending: bool,
    pub clipboard: Option<FileClipboard>,
    pub rename_dialog: RenameDialogState,
    pub new_item_dialog: NewItemDialogState,
    pub context_action: ContextAction,
    pub show_hidden: bool,
    pub status_message: Option<(String, std::time::Instant)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortField {
    Name,
    Size,
    Modified,
    Type,
}

impl Default for MainPanelState {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::Details,
            selected_file: None,
            selected_files: Vec::new(),
            entries: Vec::new(),
            sort_by: SortField::Name,
            sort_ascending: true,
            clipboard: None,
            rename_dialog: RenameDialogState::default(),
            new_item_dialog: NewItemDialogState::default(),
            context_action: ContextAction::None,
            show_hidden: false,
            status_message: None,
        }
    }
}

/// Carrega os arquivos do diretório atual.
pub fn load_directory(
    path: &Path,
    profile_config: &ProfileConfig,
    profile_filter: &ProfileFilter,
) -> Vec<FileEntry> {
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut files: Vec<FileEntry> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let p = e.path();
            profile_config.is_visible_filtered(&p, profile_filter)
        })
        .filter_map(|e| {
            let path = e.path();
            let metadata = e.metadata().ok()?;
            let name = e.file_name().to_string_lossy().to_string();

            // Ignora arquivos ocultos do sistema
            if name.starts_with('.') {
                return None;
            }

            Some(FileEntry {
                needs_profile: profile_config.needs_profile_assignment(&path),
                profiles: profile_config.resolve_profiles(&path).into_iter().cloned().collect(),
                path,
                name,
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                modified: metadata.modified().ok(),
            })
        })
        .collect();

    // Ordena: diretórios primeiro, depois por nome
    files.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    files
}

/// Renderiza o painel principal com os arquivos.
pub fn render_main_panel(
    ui: &mut Ui,
    state: &mut MainPanelState,
    current_path: &mut PathBuf,
    profile_config: &mut ProfileConfig,
    profile_filter: &ProfileFilter,
) {
    // Processar atalhos de teclado
    handle_keyboard_shortcuts(ui, state, current_path, profile_config, profile_filter);

    // Barra de ferramentas de exibição
    ui.horizontal(|ui| {
        ui.label("Exibição:");
        for mode in ViewMode::all() {
            if ui
                .selectable_label(state.view_mode == *mode, mode.label())
                .clicked()
            {
                state.view_mode = *mode;
            }
        }

        ui.separator();

        // Botão de voltar
        if ui.button("⬆ Pasta pai").clicked() {
            if let Some(parent) = current_path.parent() {
                *current_path = parent.to_path_buf();
                state.entries =
                    load_directory(current_path, profile_config, profile_filter);
            }
        }

        ui.separator();

        // Botão de atualizar
        if ui.button("🔄 Atualizar").clicked() {
            state.entries = load_directory(current_path, profile_config, profile_filter);
        }

        ui.separator();

        // Mostrar/ocultar arquivos ocultos
        if ui
            .selectable_label(state.show_hidden, "👁 Ocultos")
            .clicked()
        {
            state.show_hidden = !state.show_hidden;
            state.entries = load_directory(current_path, profile_config, profile_filter);
        }

        // Indicador de clipboard
        if let Some(ref clipboard) = state.clipboard {
            ui.separator();
            let action_label = match clipboard.action {
                ClipboardAction::Copy => "📋 Copiado",
                ClipboardAction::Cut => "✂ Recortado",
            };
            ui.label(
                egui::RichText::new(format!(
                    "{} ({} item(s))",
                    action_label,
                    clipboard.paths.len()
                ))
                .small()
                .weak(),
            );
        }

        // Status message temporária
        if let Some((ref msg, instant)) = state.status_message {
            if instant.elapsed().as_secs() < 3 {
                ui.separator();
                ui.label(
                    egui::RichText::new(msg)
                        .small()
                        .color(egui::Color32::from_rgb(100, 200, 100)),
                );
            } else {
                state.status_message = None;
            }
        }
    });

    ui.separator();

    // Diálogo de renomear
    if state.rename_dialog.active {
        render_rename_dialog(ui, state);
    }

    // Diálogo de novo item
    if state.new_item_dialog.active {
        render_new_item_dialog(ui, state, current_path);
    }

    // Menu de contexto no fundo (clique direito em área vazia)
    let panel_response = ui.interact(
        ui.available_rect_before_wrap(),
        ui.id().with("panel_ctx"),
        egui::Sense::click(),
    );
    panel_response.context_menu(|ui| {
        render_background_context_menu(ui, state, current_path, profile_config, profile_filter);
    });

    // Conteúdo dos arquivos
    match state.view_mode {
        ViewMode::Details => render_details_view(ui, state, current_path, profile_config, profile_filter),
        ViewMode::List => render_list_view(ui, state, current_path, profile_config, profile_filter),
        ViewMode::Icons => render_icons_view(ui, state, current_path, profile_config, profile_filter),
        ViewMode::Compact => render_compact_view(ui, state, current_path, profile_config, profile_filter),
    }
}

fn render_details_view(
    ui: &mut Ui,
    state: &mut MainPanelState,
    current_path: &mut PathBuf,
    profile_config: &mut ProfileConfig,
    profile_filter: &ProfileFilter,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("file_grid")
            .striped(true)
            .min_col_width(50.0)
            .show(ui, |ui| {
                // Cabeçalho
                ui.strong("Nome");
                ui.strong("Tamanho");
                ui.strong("Modificado");
                ui.strong("Perfil");
                ui.end_row();

                let entries = state.entries.clone();
                for entry in &entries {
                    let icon = if entry.is_dir {
                        "📁"
                    } else {
                        file_icon(&entry.name)
                    };

                    let label_text = format!("{} {}", icon, entry.name);
                    let is_selected = state.selected_file.as_ref() == Some(&entry.path);

                    // Nome com indicador de perfil ausente
                    let mut text = egui::RichText::new(&label_text);
                    if entry.needs_profile {
                        text = text.color(egui::Color32::from_rgb(255, 180, 50));
                    }

                    let response = ui.selectable_label(is_selected, text);

                    if response.clicked() {
                        state.selected_file = Some(entry.path.clone());
                    }

                    if response.double_clicked() && entry.is_dir {
                        *current_path = entry.path.clone();
                        state.entries =
                            load_directory(current_path, profile_config, profile_filter);
                        return;
                    }

                    // Menu de contexto (botão direito)
                    let entry_clone = entry.clone();
                    let cp = current_path.clone();
                    response.context_menu(|ui| {
                        render_context_menu(ui, &entry_clone, state, profile_config, &cp);
                    });

                    // Tamanho
                    if entry.is_dir {
                        ui.label("—");
                    } else {
                        ui.label(format_file_size(entry.size));
                    }

                    // Data de modificação
                    if let Some(modified) = entry.modified {
                        if let Ok(duration) = modified.elapsed() {
                            ui.label(format_duration(duration));
                        } else {
                            ui.label("—");
                        }
                    } else {
                        ui.label("—");
                    }

                    // Perfil
                    if entry.profiles.is_empty() {
                        ui.label(
                            egui::RichText::new("⚠ Sem perfil")
                                .color(egui::Color32::from_rgb(255, 180, 50))
                                .small(),
                        );
                    } else {
                        let labels: Vec<&str> = entry.profiles.iter().map(|p| p.label()).collect();
                        ui.label(labels.join(", "));
                    }

                    ui.end_row();
                }
            });
    });
}

fn render_list_view(
    ui: &mut Ui,
    state: &mut MainPanelState,
    current_path: &mut PathBuf,
    profile_config: &mut ProfileConfig,
    profile_filter: &ProfileFilter,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        let entries = state.entries.clone();
        for entry in &entries {
            let icon = if entry.is_dir {
                "📁"
            } else {
                file_icon(&entry.name)
            };

            let is_selected = state.selected_file.as_ref() == Some(&entry.path);
            let response =
                ui.selectable_label(is_selected, format!("{} {}", icon, entry.name));

            if response.clicked() {
                state.selected_file = Some(entry.path.clone());
            }

            if response.double_clicked() && entry.is_dir {
                *current_path = entry.path.clone();
                state.entries =
                    load_directory(current_path, profile_config, profile_filter);
                return;
            }

            let entry_clone = entry.clone();
            let cp = current_path.clone();
            response.context_menu(|ui| {
                render_context_menu(ui, &entry_clone, state, profile_config, &cp);
            });
        }
    });
}

fn render_icons_view(
    ui: &mut Ui,
    state: &mut MainPanelState,
    current_path: &mut PathBuf,
    profile_config: &mut ProfileConfig,
    profile_filter: &ProfileFilter,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        let available_width = ui.available_width();
        let icon_size = 80.0;
        let cols = (available_width / icon_size).floor().max(1.0) as usize;

        let entries = state.entries.clone();
        egui::Grid::new("icon_grid").show(ui, |ui| {
            for (i, entry) in entries.iter().enumerate() {
                let icon = if entry.is_dir {
                    "📁"
                } else {
                    file_icon(&entry.name)
                };

                let is_selected = state.selected_file.as_ref() == Some(&entry.path);

                ui.vertical(|ui| {
                    ui.set_width(icon_size);
                    let response = ui.selectable_label(
                        is_selected,
                        egui::RichText::new(icon).size(32.0),
                    );

                    if response.clicked() {
                        state.selected_file = Some(entry.path.clone());
                    }
                    if response.double_clicked() && entry.is_dir {
                        *current_path = entry.path.clone();
                        state.entries =
                            load_directory(current_path, profile_config, profile_filter);
                    }

                    let entry_clone = entry.clone();
                    let cp = current_path.clone();
                    response.context_menu(|ui| {
                        render_context_menu(ui, &entry_clone, state, profile_config, &cp);
                    });

                    let name = if entry.name.chars().count() > 12 {
                        let truncated: String = entry.name.chars().take(10).collect();
                        format!("{}...", truncated)
                    } else {
                        entry.name.clone()
                    };
                    ui.label(egui::RichText::new(name).small());
                });

                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });
    });
}

fn render_compact_view(
    ui: &mut Ui,
    state: &mut MainPanelState,
    current_path: &mut PathBuf,
    profile_config: &mut ProfileConfig,
    profile_filter: &ProfileFilter,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        let available_width = ui.available_width();
        let col_width = 200.0;
        let cols = (available_width / col_width).floor().max(1.0) as usize;

        let entries = state.entries.clone();
        egui::Grid::new("compact_grid").show(ui, |ui| {
            for (i, entry) in entries.iter().enumerate() {
                let icon = if entry.is_dir { "📁" } else { "📄" };
                let is_selected = state.selected_file.as_ref() == Some(&entry.path);

                let response = ui.selectable_label(
                    is_selected,
                    format!("{} {}", icon, entry.name),
                );

                if response.clicked() {
                    state.selected_file = Some(entry.path.clone());
                }
                if response.double_clicked() && entry.is_dir {
                    *current_path = entry.path.clone();
                    state.entries =
                        load_directory(current_path, profile_config, profile_filter);
                    return;
                }

                let entry_clone = entry.clone();
                let cp = current_path.clone();
                response.context_menu(|ui| {
                    render_context_menu(ui, &entry_clone, state, profile_config, &cp);
                });

                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });
    });
}

/// Menu de contexto completo para arquivos/diretórios.
fn render_context_menu(
    ui: &mut Ui,
    entry: &FileEntry,
    state: &mut MainPanelState,
    profile_config: &mut ProfileConfig,
    _current_path: &Path,
) {
    ui.label(egui::RichText::new(&entry.name).strong());
    ui.separator();

    // Abrir
    if ui.button("📂 Abrir").clicked() {
        let _ = open::that(&entry.path);
        ui.close_menu();
    }
    if entry.is_dir {
        if ui.button("📂 Abrir no explorador").clicked() {
            let _ = open::that(&entry.path);
            ui.close_menu();
        }
    } else if ui.button("📂 Abrir pasta do arquivo").clicked() {
        if let Some(parent) = entry.path.parent() {
            let _ = open::that(parent);
        }
        ui.close_menu();
    }

    ui.separator();

    // Copiar / Recortar
    if ui.button("📋 Copiar (Ctrl+C)").clicked() {
        let paths = get_action_paths(state, &entry.path);
        state.clipboard = Some(FileClipboard { paths, action: ClipboardAction::Copy });
        state.status_message = Some(("Copiado".to_string(), std::time::Instant::now()));
        ui.close_menu();
    }
    if ui.button("✂ Recortar (Ctrl+X)").clicked() {
        let paths = get_action_paths(state, &entry.path);
        state.clipboard = Some(FileClipboard { paths, action: ClipboardAction::Cut });
        state.status_message = Some(("Recortado".to_string(), std::time::Instant::now()));
        ui.close_menu();
    }
    if state.clipboard.is_some() && entry.is_dir {
        if ui.button("📥 Colar aqui (Ctrl+V)").clicked() {
            execute_paste(state, &entry.path);
            ui.close_menu();
        }
    }

    ui.separator();

    // Renomear / Duplicar
    if ui.button("✏ Renomear (F2)").clicked() {
        state.rename_dialog.active = true;
        state.rename_dialog.path = Some(entry.path.clone());
        state.rename_dialog.new_name = entry.name.clone();
        ui.close_menu();
    }
    if !entry.is_dir {
        if ui.button("📄 Duplicar").clicked() {
            duplicate_file(&entry.path);
            state.status_message = Some(("Duplicado".to_string(), std::time::Instant::now()));
            ui.close_menu();
        }
    }

    ui.separator();

    // Copiar informações
    ui.menu_button("📎 Copiar info", |ui| {
        if ui.button("Caminho completo").clicked() {
            ui.ctx().copy_text(entry.path.to_string_lossy().to_string());
            ui.close_menu();
        }
        if ui.button("Nome").clicked() {
            ui.ctx().copy_text(entry.name.clone());
            ui.close_menu();
        }
        if let Some(parent) = entry.path.parent() {
            if ui.button("Pasta pai").clicked() {
                ui.ctx().copy_text(parent.to_string_lossy().to_string());
                ui.close_menu();
            }
        }
    });

    ui.separator();

    // Novo (dentro de pasta)
    if entry.is_dir {
        ui.menu_button("➕ Novo", |ui| {
            if ui.button("📁 Nova pasta").clicked() {
                state.new_item_dialog = NewItemDialogState {
                    active: true, is_folder: true, name: String::new(),
                    parent: Some(entry.path.clone()),
                };
                ui.close_menu();
            }
            if ui.button("📄 Novo arquivo").clicked() {
                state.new_item_dialog = NewItemDialogState {
                    active: true, is_folder: false, name: String::new(),
                    parent: Some(entry.path.clone()),
                };
                ui.close_menu();
            }
        });
        ui.separator();
    }

    // Perfil
    ui.menu_button("👤 Definir perfil", |ui| {
        for profile in profile_config.all_profiles() {
            let is_current = entry.profiles.contains(&profile);
            let label = if is_current { format!("✓ {}", profile.label()) } else { profile.label().to_string() };
            if ui.selectable_label(is_current, label).clicked() {
                if is_current {
                    // Toggle off: remove este perfil
                    profile_config.unassign_profile(&entry.path, &profile);
                    state.status_message = Some((format!("Perfil '{}' removido", profile.label()), std::time::Instant::now()));
                } else {
                    // Toggle on: adiciona este perfil
                    profile_config.assign_profile(entry.path.clone(), profile.clone());
                    state.status_message = Some((format!("Perfil '{}' atribuído", profile.label()), std::time::Instant::now()));
                }
                ui.close_menu();
            }
        }
        ui.separator();
        if !entry.profiles.is_empty() && ui.button("❌ Remover todos os perfis").clicked() {
            profile_config.remove_profile(&entry.path);
            ui.close_menu();
        }
    });

    ui.separator();

    // Segurança (pastas)
    if entry.is_dir {
        let is_secure = profile_config.secure_directories.contains(&entry.path);
        if is_secure {
            if ui.button("🔓 Remover proteção de rede").clicked() {
                profile_config.secure_directories.retain(|p| p != &entry.path);
                ui.close_menu();
            }
        } else if ui.button("🔒 Proteger (não compartilhar)").clicked() {
            profile_config.secure_directories.push(entry.path.clone());
            ui.close_menu();
        }
        ui.separator();
    }

    // Propriedades
    ui.menu_button("ℹ Propriedades", |ui| {
        if let Ok(metadata) = std::fs::metadata(&entry.path) {
            if entry.is_dir {
                ui.label("Tipo: Diretório");
                if let Ok(entries) = std::fs::read_dir(&entry.path) {
                    ui.label(format!("Itens: {}", entries.count()));
                }
            } else {
                ui.label(format!("Tamanho: {}", format_file_size(metadata.len())));
            }
            if let Ok(m) = metadata.modified() {
                if let Ok(d) = m.elapsed() { ui.label(format!("Modificado: {}", format_duration(d))); }
            }
            ui.label(format!("Somente leitura: {}", if metadata.permissions().readonly() { "Sim" } else { "Não" }));
        }
    });

    ui.separator();

    // Excluir
    if ui.button(egui::RichText::new("🗑 Mover para lixeira (Del)").color(egui::Color32::from_rgb(220, 80, 80))).clicked() {
        let paths = get_action_paths(state, &entry.path);
        for p in &paths { let _ = trash::delete(p); }
        state.status_message = Some((format!("{} excluído(s)", paths.len()), std::time::Instant::now()));
        state.selected_files.clear();
        state.selected_file = None;
        ui.close_menu();
    }
}

/// Retorna um ícone baseado na extensão do arquivo.
fn file_icon(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" | "py" | "js" | "ts" | "c" | "cpp" | "java" | "go" => "💻",
        "txt" | "md" | "doc" | "docx" | "pdf" => "📝",
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" => "🖼",
        "mp3" | "wav" | "flac" | "ogg" => "🎵",
        "mp4" | "avi" | "mkv" | "mov" => "🎬",
        "zip" | "tar" | "gz" | "rar" | "7z" => "📦",
        "exe" | "msi" | "bat" | "sh" => "⚙",
        "toml" | "yaml" | "yml" | "json" | "xml" => "🔧",
        _ => "📄",
    }
}

fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        "agora".to_string()
    } else if secs < 3600 {
        format!("{}min atrás", secs / 60)
    } else if secs < 86400 {
        format!("{}h atrás", secs / 3600)
    } else {
        format!("{}d atrás", secs / 86400)
    }
}

/// Retorna os caminhos selecionados para ação (múltipla seleção ou item individual).
fn get_action_paths(state: &MainPanelState, clicked_path: &Path) -> Vec<PathBuf> {
    if state.selected_files.contains(&clicked_path.to_path_buf()) && state.selected_files.len() > 1 {
        state.selected_files.clone()
    } else {
        vec![clicked_path.to_path_buf()]
    }
}

/// Executa a operação de colar (copiar ou mover).
fn execute_paste(state: &mut MainPanelState, destination: &Path) {
    if let Some(clipboard) = state.clipboard.take() {
        let mut success_count = 0;
        for source in &clipboard.paths {
            let file_name = source
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let dest_path = destination.join(&file_name);
            let final_dest = if dest_path.exists() && dest_path != *source {
                get_unique_path(&dest_path)
            } else {
                dest_path
            };

            let result = match clipboard.action {
                ClipboardAction::Copy => {
                    if source.is_dir() {
                        copy_dir_recursive(source, &final_dest)
                    } else {
                        std::fs::copy(source, &final_dest).map(|_| ()).map_err(|e| e.to_string())
                    }
                }
                ClipboardAction::Cut => {
                    std::fs::rename(source, &final_dest).map_err(|e| e.to_string())
                }
            };
            if result.is_ok() {
                success_count += 1;
            }
        }
        let action_label = match clipboard.action {
            ClipboardAction::Copy => "copiado(s)",
            ClipboardAction::Cut => "movido(s)",
        };
        state.status_message = Some((
            format!("{} item(s) {}", success_count, action_label),
            std::time::Instant::now(),
        ));
        // Restaura clipboard se foi copiar
        if clipboard.action == ClipboardAction::Copy {
            state.clipboard = Some(clipboard);
        }
    }
}

/// Duplica um arquivo adicionando " - Cópia" ao nome.
fn duplicate_file(path: &Path) {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let parent = path.parent().unwrap_or(path);
    let mut new_path = parent.join(format!("{} - Cópia{}", stem, ext));
    let mut counter = 2;
    while new_path.exists() {
        new_path = parent.join(format!("{} - Cópia ({}){}", stem, counter, ext));
        counter += 1;
    }
    let _ = std::fs::copy(path, &new_path);
}

/// Gera um caminho único adicionando sufixo numérico.
fn get_unique_path(path: &Path) -> PathBuf {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let parent = path.parent().unwrap_or(path);
    let mut counter = 2;
    loop {
        let new_path = parent.join(format!("{} ({}){}", stem, counter, ext));
        if !new_path.exists() {
            return new_path;
        }
        counter += 1;
    }
}

/// Copia um diretório recursivamente.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    let entries = std::fs::read_dir(src).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Diálogo de renomear arquivo/pasta.
fn render_rename_dialog(ui: &mut Ui, state: &mut MainPanelState) {
    egui::Window::new("Renomear")
        .collapsible(false)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Novo nome:");
                let resp = ui.text_edit_singleline(&mut state.rename_dialog.new_name);
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    execute_rename(state);
                }
            });
            ui.horizontal(|ui| {
                if ui.button("✓ Confirmar").clicked() {
                    execute_rename(state);
                }
                if ui.button("✗ Cancelar").clicked() {
                    state.rename_dialog.active = false;
                }
            });
        });
}

fn execute_rename(state: &mut MainPanelState) {
    if let Some(ref old_path) = state.rename_dialog.path {
        if let Some(parent) = old_path.parent() {
            let new_path = parent.join(&state.rename_dialog.new_name);
            if !new_path.exists() {
                let _ = std::fs::rename(old_path, &new_path);
                state.status_message = Some(("Renomeado".to_string(), std::time::Instant::now()));
            } else {
                state.status_message = Some(("Erro: nome já existe".to_string(), std::time::Instant::now()));
            }
        }
    }
    state.rename_dialog.active = false;
}

/// Diálogo para criar novo arquivo/pasta.
fn render_new_item_dialog(ui: &mut Ui, state: &mut MainPanelState, current_path: &Path) {
    let title = if state.new_item_dialog.is_folder { "Nova pasta" } else { "Novo arquivo" };
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Nome:");
                let resp = ui.text_edit_singleline(&mut state.new_item_dialog.name);
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    create_new_item(state, current_path);
                }
            });
            ui.horizontal(|ui| {
                if ui.button("✓ Criar").clicked() {
                    create_new_item(state, current_path);
                }
                if ui.button("✗ Cancelar").clicked() {
                    state.new_item_dialog.active = false;
                }
            });
        });
}

fn create_new_item(state: &mut MainPanelState, current_path: &Path) {
    let name = state.new_item_dialog.name.trim().to_string();
    if name.is_empty() {
        state.new_item_dialog.active = false;
        return;
    }
    let parent = state.new_item_dialog.parent.clone().unwrap_or_else(|| current_path.to_path_buf());
    let new_path = parent.join(&name);
    if new_path.exists() {
        state.status_message = Some(("Erro: já existe".to_string(), std::time::Instant::now()));
        state.new_item_dialog.active = false;
        return;
    }
    let result = if state.new_item_dialog.is_folder {
        std::fs::create_dir_all(&new_path).map_err(|e| e.to_string())
    } else {
        std::fs::write(&new_path, "").map_err(|e| e.to_string())
    };
    match result {
        Ok(_) => {
            let kind = if state.new_item_dialog.is_folder { "Pasta" } else { "Arquivo" };
            state.status_message = Some((format!("{} '{}' criado", kind, name), std::time::Instant::now()));
        }
        Err(e) => {
            state.status_message = Some((format!("Erro: {}", e), std::time::Instant::now()));
        }
    }
    state.new_item_dialog.active = false;
}

/// Menu de contexto para área vazia (background).
fn render_background_context_menu(
    ui: &mut Ui,
    state: &mut MainPanelState,
    current_path: &Path,
    profile_config: &mut ProfileConfig,
    profile_filter: &ProfileFilter,
) {
    ui.label(egui::RichText::new("Pasta atual").strong());
    ui.separator();

    if state.clipboard.is_some() {
        if ui.button("📥 Colar aqui (Ctrl+V)").clicked() {
            execute_paste(state, current_path);
            state.entries = load_directory(current_path, profile_config, profile_filter);
            ui.close_menu();
        }
        ui.separator();
    }

    if ui.button("📁 Nova pasta (Ctrl+Shift+N)").clicked() {
        state.new_item_dialog = NewItemDialogState {
            active: true, is_folder: true, name: String::new(),
            parent: Some(current_path.to_path_buf()),
        };
        ui.close_menu();
    }
    if ui.button("📄 Novo arquivo (Ctrl+N)").clicked() {
        state.new_item_dialog = NewItemDialogState {
            active: true, is_folder: false, name: String::new(),
            parent: Some(current_path.to_path_buf()),
        };
        ui.close_menu();
    }

    ui.separator();

    if ui.button("🔄 Atualizar (F5)").clicked() {
        state.entries = load_directory(current_path, profile_config, profile_filter);
        ui.close_menu();
    }
    if ui.button("☑ Selecionar tudo (Ctrl+A)").clicked() {
        state.selected_files = state.entries.iter().map(|e| e.path.clone()).collect();
        ui.close_menu();
    }

    ui.separator();

    ui.menu_button("👤 Perfil desta pasta", |ui| {
        let current_profiles: Vec<Profile> = profile_config.resolve_profiles(current_path).into_iter().cloned().collect();
        let all = profile_config.all_profiles();
        for profile in all {
            let is_current = current_profiles.contains(&profile);
            let label = if is_current { format!("✓ {}", profile.label()) } else { profile.label().to_string() };
            if ui.selectable_label(is_current, label).clicked() {
                if is_current {
                    profile_config.unassign_profile(current_path, &profile);
                    state.status_message = Some((format!("Perfil '{}' removido", profile.label()), std::time::Instant::now()));
                } else {
                    profile_config.assign_profile(current_path.to_path_buf(), profile.clone());
                    state.status_message = Some((format!("Perfil '{}' atribuído", profile.label()), std::time::Instant::now()));
                }
                ui.close_menu();
            }
        }
    });

    if ui.button("🖥 Abrir no explorador do sistema").clicked() {
        let _ = open::that(current_path);
        ui.close_menu();
    }
}

/// Processa atalhos de teclado.
fn handle_keyboard_shortcuts(
    ui: &mut Ui,
    state: &mut MainPanelState,
    current_path: &mut PathBuf,
    profile_config: &mut ProfileConfig,
    profile_filter: &ProfileFilter,
) {
    let ctx = ui.ctx();

    // Ctrl+C - Copiar
    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::C)) {
        if let Some(ref selected) = state.selected_file {
            let paths = if state.selected_files.is_empty() {
                vec![selected.clone()]
            } else {
                state.selected_files.clone()
            };
            state.clipboard = Some(FileClipboard { paths, action: ClipboardAction::Copy });
            state.status_message = Some(("Copiado".to_string(), std::time::Instant::now()));
        }
    }

    // Ctrl+X - Recortar
    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::X)) {
        if let Some(ref selected) = state.selected_file {
            let paths = if state.selected_files.is_empty() {
                vec![selected.clone()]
            } else {
                state.selected_files.clone()
            };
            state.clipboard = Some(FileClipboard { paths, action: ClipboardAction::Cut });
            state.status_message = Some(("Recortado".to_string(), std::time::Instant::now()));
        }
    }

    // Ctrl+V - Colar
    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::V)) {
        if state.clipboard.is_some() {
            let dest = current_path.clone();
            execute_paste(state, &dest);
            state.entries = load_directory(current_path, profile_config, profile_filter);
        }
    }

    // Delete - Lixeira
    if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
        let paths = if !state.selected_files.is_empty() {
            state.selected_files.clone()
        } else if let Some(ref s) = state.selected_file {
            vec![s.clone()]
        } else {
            Vec::new()
        };
        if !paths.is_empty() {
            for p in &paths { let _ = trash::delete(p); }
            state.status_message = Some((
                format!("{} excluído(s)", paths.len()), std::time::Instant::now()
            ));
            state.selected_files.clear();
            state.selected_file = None;
            state.entries = load_directory(current_path, profile_config, profile_filter);
        }
    }

    // F2 - Renomear
    if ctx.input(|i| i.key_pressed(egui::Key::F2)) {
        if let Some(ref selected) = state.selected_file {
            let name = selected.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            state.rename_dialog = RenameDialogState {
                active: true, path: Some(selected.clone()), new_name: name,
            };
        }
    }

    // F5 - Atualizar
    if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
        state.entries = load_directory(current_path, profile_config, profile_filter);
        state.status_message = Some(("Atualizado".to_string(), std::time::Instant::now()));
    }

    // Ctrl+A - Selecionar tudo
    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::A)) {
        state.selected_files = state.entries.iter().map(|e| e.path.clone()).collect();
    }

    // Ctrl+N - Novo arquivo
    if ctx.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::N)) {
        state.new_item_dialog = NewItemDialogState {
            active: true, is_folder: false, name: String::new(),
            parent: Some(current_path.clone()),
        };
    }

    // Ctrl+Shift+N - Nova pasta
    if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::N)) {
        state.new_item_dialog = NewItemDialogState {
            active: true, is_folder: true, name: String::new(),
            parent: Some(current_path.clone()),
        };
    }

    // Backspace - Pasta pai
    if ctx.input(|i| i.key_pressed(egui::Key::Backspace)) {
        if let Some(parent) = current_path.parent() {
            *current_path = parent.to_path_buf();
            state.entries = load_directory(current_path, profile_config, profile_filter);
        }
    }
}
