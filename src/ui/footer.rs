use crate::core::file_ops::{FileOperation, OperationHistory, OperationStatus};
use egui::Ui;

/// Renderiza o rodapé com operações em andamento e histórico.
pub fn render_footer(ui: &mut Ui, history: &OperationHistory) {
    ui.horizontal(|ui| {
        // Operações ativas
        let active_ops = history.active_operations();
        if !active_ops.is_empty() {
            for op in &active_ops {
                render_operation_progress(ui, op);
                ui.separator();
            }
        }

        // Últimas ações concluídas
        let recent = history.recent_completed(3);
        if !recent.is_empty() {
            ui.separator();
            ui.label(egui::RichText::new("Recentes:").small().weak());
            for op in &recent {
                let source_name = op
                    .source
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                ui.label(
                    egui::RichText::new(format!("✓ {} {}", op.kind.label(), source_name))
                        .small()
                        .color(egui::Color32::from_rgb(100, 200, 100)),
                );
            }
        }

        if active_ops.is_empty() && recent.is_empty() {
            ui.label(egui::RichText::new("Pronto").weak().small());
        }
    });
}

fn render_operation_progress(ui: &mut Ui, op: &FileOperation) {
    let source_name = op
        .source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    ui.horizontal(|ui| {
        let status_icon = match op.status {
            OperationStatus::Pending => "⏳",
            OperationStatus::InProgress => "⚡",
            OperationStatus::Completed => "✅",
            OperationStatus::Failed => "❌",
            OperationStatus::Cancelled => "🚫",
        };

        ui.label(
            egui::RichText::new(format!(
                "{} {} {}",
                status_icon,
                op.kind.label(),
                source_name
            ))
            .small(),
        );

        // Barra de progresso
        if op.status == OperationStatus::InProgress {
            let progress_bar = egui::ProgressBar::new(op.progress)
                .desired_width(120.0)
                .text(format!("{:.0}%", op.progress * 100.0));
            ui.add(progress_bar);
        }
    });
}
