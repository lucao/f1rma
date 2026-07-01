use crate::core::profile::{Profile, ProfileFilter};
use crate::network::discovery::DiscoveryState;
use egui::Ui;
use std::path::Path;

/// Renderiza o cabeçalho superior com informações de perfil e pasta.
pub fn render_header(
    ui: &mut Ui,
    profile_filter: &mut ProfileFilter,
    current_path: &Path,
    machine_name: &str,
    discovery_state: &DiscoveryState,
) {
    ui.horizontal(|ui| {
        // Seletor de perfis (toggle múltiplo)
        ui.label("Perfil:");

        // Botão "Todos" — limpa filtro
        let all_active = profile_filter.show_all();
        let all_btn = egui::Button::new(
            egui::RichText::new("Todos").strong(),
        ).selected(all_active);
        if ui.add(all_btn).clicked() {
            profile_filter.clear();
        }

        // Botões individuais — toggle cada perfil
        for profile in Profile::all() {
            let is_active = profile_filter.is_active(*profile);
            let button = egui::Button::new(profile.label())
                .selected(is_active);
            if ui.add(button).clicked() {
                profile_filter.toggle(*profile);
            }
        }

        ui.separator();

        // Informação da pasta atual (centro)
        ui.label("📁");
        let path_str = current_path.to_string_lossy();
        ui.label(
            egui::RichText::new(path_str.as_ref())
                .monospace()
                .size(13.0),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Máquina (direita)
            ui.label(
                egui::RichText::new(format!("🖥 {}", machine_name))
                    .size(12.0)
                    .weak(),
            );

            ui.separator();

            // Indicador de peers na rede
            let peer_count = discovery_state.peer_count();
            let peers_label = if peer_count == 0 {
                "🌐 Nenhum peer".to_string()
            } else {
                format!("🌐 {} peer(s)", peer_count)
            };
            let color = if peer_count > 0 {
                egui::Color32::from_rgb(100, 200, 100)
            } else {
                egui::Color32::from_rgb(150, 150, 150)
            };

            let peer_btn = ui.add(
                egui::Button::new(
                    egui::RichText::new(&peers_label).size(12.0).color(color),
                )
                .frame(false),
            );

            if peer_count > 0 {
                peer_btn.on_hover_ui(|ui| {
                    ui.label(egui::RichText::new("Peers na rede:").strong());
                    for peer in discovery_state.online_peers() {
                        ui.horizontal(|ui| {
                            ui.label("🟢");
                            ui.label(format!(
                                "{} ({}) - {}:{}",
                                peer.machine_name,
                                peer.username,
                                peer.address,
                                peer.port
                            ));
                        });
                    }
                });
            }
        });
    });
}
