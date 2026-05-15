mod app;
mod data;
mod matcher;
mod renamer;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 700.0])
            .with_min_inner_size([720.0, 520.0]),
        ..Default::default()
    };

    eframe::run_native(
        "批量文件重命名工具",
        options,
        Box::new(|_cc| Box::new(app::BatchRenameApp::default())),
    )
}
