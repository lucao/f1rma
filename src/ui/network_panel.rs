use crate::network::discovery::DiscoveryState;
use egui::Ui;

/// Renderiza o painel de rede com a lista de peers descobertos via mDNS.
pub fn render_network_panel(ui: &mut Ui, discovery_state: &DiscoveryState) {
    ui.vertical(|ui| {
        // Título
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("🌐 Rede").strong().size(14.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let is_running = *discovery_state.is_running.lock().unwrap();
                if is_running {
                    ui.label(egui::RichText::new("● Ativo").small().color(
                        egui::Color32::from_rgb(100, 200, 100),
                    ));
                } else {
                    ui.label(egui::RichText::new("● Inativo").small().color(
                        egui::Color32::from_rgb(200, 100, 100),
                    ));
                }
            });
        });

        ui.separator();

        let peers = discovery_state.all_peers();

        if peers.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("Nenhum computador encontrado")
                        .weak()
                        .italics(),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Computadores com F1RMA na mesma rede aparecerão aqui")
                        .weak()
                        .small(),
                );
                ui.add_space(20.0);
            });
        } else {
            ui.label(egui::RichText::new(format!(
                "{} computador(es) na rede",
                peers.iter().filter(|p| p.online).count()
            )).small());

            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .id_salt("network_peers_scroll")
                .show(ui, |ui| {
                    for peer in &peers {
                        render_peer_card(ui, peer);
                    }
                });
        }
    });
}

/// Renderiza o card de um peer individual.
fn render_peer_card(ui: &mut Ui, peer: &crate::network::discovery::Peer) {
    let frame_color = if peer.online {
        egui::Color32::from_rgba_premultiplied(40, 80, 40, 60)
    } else {
        egui::Color32::from_rgba_premultiplied(80, 40, 40, 60)
    };

    egui::Frame::none()
        .fill(frame_color)
        .rounding(4.0)
        .inner_margin(8.0)
        .outer_margin(4.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Ícone de status
                let status_icon = if peer.online { "🟢" } else { "🔴" };
                ui.label(egui::RichText::new(status_icon).size(16.0));

                ui.vertical(|ui| {
                    // Nome da máquina + usuário
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&peer.machine_name)
                                .strong()
                                .size(13.0),
                        );
                        ui.label(
                            egui::RichText::new(format!("({})", peer.username))
                                .weak()
                                .small(),
                        );
                    });

                    // Endereço e porta
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{}:{}", peer.address, peer.port))
                                .monospace()
                                .small()
                                .weak(),
                        );

                        if !peer.version.is_empty() {
                            ui.label(
                                egui::RichText::new(format!("v{}", peer.version))
                                    .small()
                                    .weak(),
                            );
                        }
                    });
                });
            });

            // Botões de ação
            ui.horizontal(|ui| {
                if peer.online {
                    if ui.small_button("📂 Ver arquivos").clicked() {
                        // TODO: Abrir navegação de arquivos remotos
                    }
                    if ui.small_button("📋 Copiar IP").clicked() {
                        ui.ctx().copy_text(format!("{}:{}", peer.address, peer.port));
                    }
                }
            });
        });
}
