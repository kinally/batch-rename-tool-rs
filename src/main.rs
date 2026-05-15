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
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(app::BatchRenameApp::default()))
        }),
    )
}

/// 将 Windows 系统字体（微软雅黑）加载到 egui 中，
/// 解决中文显示为方框的问题。
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Windows 中文系统内置微软雅黑
    let font_paths = [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyhbd.ttc",
        r"C:\Windows\Fonts\SIMHEI.TTF",
    ];

    for path in &font_paths {
        if let Ok(data) = std::fs::read(path) {
            let name = format!("chinese_font_{}", path.rsplit('\\').next().unwrap_or("unknown"));
            fonts.font_data.insert(name.clone(), egui::FontData::from_owned(data));

            // 在中英文模式下都用这个字体
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, name.clone());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, name);

            break; // 找到第一个可用字体就结束
        }
    }

    ctx.set_fonts(fonts);
}
