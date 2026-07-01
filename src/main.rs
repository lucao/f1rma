mod app;
mod core;
mod network;
mod ui;

use app::F1rmaApp;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([1000.0, 600.0])
            .with_title("F1RMA - Gerenciador de Arquivos"),
        ..Default::default()
    };

    eframe::run_native(
        "F1RMA",
        native_options,
        Box::new(|cc| Ok(Box::new(F1rmaApp::new(cc)))),
    )
}
