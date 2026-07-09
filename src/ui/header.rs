use crate::core::profile::{Profile, ProfileFilter};
use crate::network::discovery::DiscoveryState;
use egui::Ui;
use std::path::{Path, PathBuf};

/// Estado do diálogo de adicionar perfil.
pub struct HeaderState {
    pub adding_profile: bool,
    pub new_profile_name: String,
}

impl Default for HeaderState {
    fn default() -> Self {
        Self {
            adding_profile: false,
            new_profile_name: String::new(),
        }
    }
}

/// Renderiza o cabeçalho superior.
pub fn render_header(
    ui: &mut Ui,
    profile_filter: &mut ProfileFilter,
    current_path: &mut PathBuf,
    machine_name: &str,
    discovery_state: &DiscoveryState,
    available_profiles: &[Profile],
    header_state: &mut HeaderState,
    on_add_profile: &mut Option<String>,
) {
    ui.horizontal(|ui| {
        // Seletor de perfis
        ui.label("Perfil:");

        // Botão "Todos"
        let all_active = profile_filter.show_all();
        if ui.add(egui::Button::new(
            egui::RichText::new("Todos").strong()
        ).selected(all_active)).clicked() {
            profile_filter.clear();
        }

        // Botões de perfil (toggle)
        for profile in available_profiles {
            let is_active = profile_filter.is_active(profile);
            if ui.add(egui::Button::new(profile.label()).selected(is_active)).clicked() {
                profile_filter.toggle(profile.clone());
            }
        }

        // Botão + para adicionar perfil
        if ui.small_button("➕").on_hover_text("Novo perfil").clicked() {
            header_state.adding_profile = true;
            header_state.new_profile_name.clear();
        }

        ui.separator();

        // Breadcrumb clicável da pasta atual
        ui.label("📁");
        render_breadcrumb(ui, current_path);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(format!("🖥 {}", machine_name)).size(12.0).weak());

            ui.separator();

            // Peers
            let peer_count = discovery_state.peer_count();
            let color = if peer_count > 0 {
                egui::Color32::from_rgb(100, 200, 100)
            } else {
                egui::Color32::from_rgb(150, 150, 150)
            };
            let peers_label = if peer_count == 0 {
                "🌐 Nenhum peer".to_string()
            } else {
                format!("🌐 {} peer(s)", peer_count)
            };
            let peer_btn = ui.add(egui::Button::new(
                egui::RichText::new(&peers_label).size(12.0).color(color)
            ).frame(false));

            if peer_count > 0 {
                peer_btn.on_hover_ui(|ui| {
                    ui.label(egui::RichText::new("Peers na rede:").strong());
                    for peer in discovery_state.online_peers() {
                        ui.horizontal(|ui| {
                            ui.label("🟢");
                            ui.label(format!("{} ({}) - {}:{}", peer.machine_name, peer.username, peer.address, peer.port));
                        });
                    }
                });
            }
        });
    });

    // Diálogo de novo perfil
    if header_state.adding_profile {
        egui::Window::new("Novo Perfil")
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Nome:");
                    let resp = ui.text_edit_singleline(&mut header_state.new_profile_name);
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let name = header_state.new_profile_name.trim().to_string();
                        if !name.is_empty() {
                            *on_add_profile = Some(name);
                        }
                        header_state.adding_profile = false;
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("✓ Criar").clicked() {
                        let name = header_state.new_profile_name.trim().to_string();
                        if !name.is_empty() {
                            *on_add_profile = Some(name);
                        }
                        header_state.adding_profile = false;
                    }
                    if ui.button("✗ Cancelar").clicked() {
                        header_state.adding_profile = false;
                    }
                });
            });
    }
}

/// Renderiza o caminho como breadcrumb clicável (cada segmento navega para aquela pasta).
fn render_breadcrumb(ui: &mut Ui, current_path: &mut PathBuf) {
    let path_clone = current_path.clone();
    let mut segments: Vec<(String, PathBuf)> = Vec::new();

    // Coleta todos os segmentos do path
    let mut p = path_clone.as_path();
    loop {
        let name = p.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| p.to_string_lossy().to_string());
        segments.push((name, p.to_path_buf()));
        if let Some(parent) = p.parent() {
            if parent == p { break; } // Raiz
            p = parent;
        } else {
            break;
        }
    }

    segments.reverse();

    for (i, (name, path)) in segments.iter().enumerate() {
        if i > 0 {
            ui.label(egui::RichText::new("›").weak().size(13.0));
        }
        let is_last = i == segments.len() - 1;
        let text = if is_last {
            egui::RichText::new(name.as_str()).monospace().size(13.0).strong()
        } else {
            egui::RichText::new(name.as_str()).monospace().size(13.0)
        };

        if ui.add(egui::Label::new(text).sense(egui::Sense::click())).clicked() {
            *current_path = path.clone();
        }
    }
}
