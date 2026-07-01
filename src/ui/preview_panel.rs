use crate::core::annotations::{Annotation, AnnotationStore};
use egui::Ui;
use std::path::{Path, PathBuf};

/// Estado do painel de pré-visualização.
pub struct PreviewPanelState {
    pub file_content_preview: Option<String>,
    pub new_annotation_text: String,
}

impl Default for PreviewPanelState {
    fn default() -> Self {
        Self {
            file_content_preview: None,
            new_annotation_text: String::new(),
        }
    }
}

/// Renderiza o painel lateral direito com preview e anotações.
pub fn render_preview_panel(
    ui: &mut Ui,
    selected_file: Option<&PathBuf>,
    state: &mut PreviewPanelState,
    annotations: &mut AnnotationStore,
    username: &str,
    machine_id: &str,
) {
    match selected_file {
        None => {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Selecione um arquivo para pré-visualizar")
                        .weak()
                        .italics(),
                );
            });
        }
        Some(path) => {
            ui.vertical(|ui| {
                // Informações do arquivo
                render_file_info(ui, path);
                ui.separator();

                // Preview do conteúdo
                render_file_preview(ui, path, state);
                ui.separator();

                // Anotações
                render_annotations(ui, path, state, annotations, username, machine_id);
            });
        }
    }
}

fn render_file_info(ui: &mut Ui, path: &Path) {
    ui.label(egui::RichText::new("Informações").strong());

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    ui.label(format!("Nome: {}", name));

    if let Ok(metadata) = std::fs::metadata(path) {
        ui.label(format!("Tamanho: {}", format_size(metadata.len())));

        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.elapsed() {
                ui.label(format!("Modificado: {}", format_time_ago(duration)));
            }
        }

        if metadata.is_file() {
            if let Some(ext) = path.extension() {
                ui.label(format!("Tipo: .{}", ext.to_string_lossy()));
            }
        }
    }
}

fn render_file_preview(ui: &mut Ui, path: &Path, state: &mut PreviewPanelState) {
    ui.label(egui::RichText::new("Pré-visualização").strong());

    if !path.is_file() {
        ui.label("(diretório)");
        return;
    }

    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        // Arquivos de texto
        "txt" | "md" | "rs" | "py" | "js" | "ts" | "json" | "toml" | "yaml" | "yml"
        | "xml" | "html" | "css" | "c" | "cpp" | "h" | "java" | "go" | "sh" | "bat"
        | "csv" | "log" | "cfg" | "ini" => {
            // Lê apenas os primeiros 4KB para preview
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    let preview = if content.len() > 4096 {
                        // Find a safe char boundary near 4096
                        let mut end = 4096;
                        while end > 0 && !content.is_char_boundary(end) {
                            end -= 1;
                        }
                        format!("{}...\n\n[arquivo truncado]", &content[..end])
                    } else {
                        content
                    };
                    state.file_content_preview = Some(preview.clone());

                    egui::ScrollArea::vertical()
                        .max_height(250.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(&preview).monospace().size(11.0),
                            );
                        });
                }
                Err(_) => {
                    ui.label("Não foi possível ler o arquivo");
                }
            }
        }
        // Imagens - indicar que pode ser visualizado
        "png" | "jpg" | "jpeg" | "gif" | "bmp" => {
            ui.label("🖼 Arquivo de imagem");
            ui.label(
                egui::RichText::new("(pré-visualização de imagem em desenvolvimento)")
                    .small()
                    .weak(),
            );
        }
        // Outros tipos
        _ => {
            ui.label("Pré-visualização não disponível para este tipo de arquivo");
        }
    }
}

fn render_annotations(
    ui: &mut Ui,
    path: &Path,
    state: &mut PreviewPanelState,
    annotations: &mut AnnotationStore,
    username: &str,
    machine_id: &str,
) {
    ui.label(egui::RichText::new("📝 Anotações").strong());

    // Exibir anotações existentes
    let path_buf = path.to_path_buf();
    if let Some(file_annotations) = annotations.get_annotations(path).cloned() {
        egui::ScrollArea::vertical()
            .max_height(150.0)
            .id_salt("annotations_scroll")
            .show(ui, |ui| {
                for annotation in &file_annotations {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&annotation.author)
                                    .strong()
                                    .small(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "({})",
                                    &annotation.machine_id[..8]
                                ))
                                .weak()
                                .small(),
                            );
                        });
                        ui.label(&annotation.content);
                        ui.label(
                            egui::RichText::new(
                                annotation.created_at.format("%d/%m/%Y %H:%M").to_string(),
                            )
                            .weak()
                            .small(),
                        );
                    });
                }
            });
    } else {
        ui.label(egui::RichText::new("Nenhuma anotação").weak().small());
    }

    ui.separator();

    // Adicionar nova anotação
    ui.label("Nova anotação:");
    ui.text_edit_multiline(&mut state.new_annotation_text);

    if ui.button("💾 Salvar anotação").clicked() && !state.new_annotation_text.is_empty() {
        let annotation = Annotation::new(
            state.new_annotation_text.clone(),
            username.to_string(),
            machine_id.to_string(),
        );
        annotations.add_annotation(path_buf, annotation);
        state.new_annotation_text.clear();
    }
}

fn format_size(bytes: u64) -> String {
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

fn format_time_ago(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        "agora".to_string()
    } else if secs < 3600 {
        format!("{} min atrás", secs / 60)
    } else if secs < 86400 {
        format!("{} h atrás", secs / 3600)
    } else {
        format!("{} dias atrás", secs / 86400)
    }
}
