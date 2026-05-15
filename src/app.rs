/// 主应用 — GUI 界面 + 状态管理
///
/// 三步流程：
///   0. 导入数据源
///   1. 添加文件
///   2. 构建规则 → 预览 → 执行

use crate::data;
use crate::matcher;
use crate::renamer;
use crate::renamer::TemplatePart;

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::collections::HashMap;
use std::path::Path;

// ─── 颜色常量 ─────────────────────────────────────────────

const COLOR_PRIMARY: egui::Color32 = egui::Color32::from_rgb(0x63, 0x6a, 0xf1);
const COLOR_SUCCESS: egui::Color32 = egui::Color32::from_rgb(0x2e, 0xcc, 0x71);
const COLOR_DANGER: egui::Color32 = egui::Color32::from_rgb(0xe7, 0x4c, 0x3c);
const COLOR_WARNING: egui::Color32 = egui::Color32::from_rgb(0xf3, 0x93, 0x19);
const COLOR_BG_CARD: egui::Color32 = egui::Color32::from_rgb(0xf8, 0xf9, 0xfa);
const COLOR_BORDER: egui::Color32 = egui::Color32::from_rgb(0xde, 0xe2, 0xe6);

// ─── 应用状态 ─────────────────────────────────────────────

pub struct BatchRenameApp {
    // ── 步骤导航 ──
    current_step: usize,

    // ── Step 0: 数据源 ──
    data_headers: Vec<String>,
    data_rows: Vec<Vec<String>>,
    data_path: Option<String>,
    data_loaded: bool,

    // ── Step 1: 文件列表 ──
    files: Vec<String>,
    file_match_results: Vec<(String, Option<usize>, Vec<(String, String)>)>,
    matched_column: Option<usize>,

    // ── Step 2: 模板 ──
    template_parts: Vec<TemplatePart>,
    seq_enabled: bool,
    seq_start: u32,
    seq_digits: u32,
    time_enabled: bool,
    time_format: String,
    custom_text: String,

    // ── 预览 ──
    preview_items: Vec<(String, String, bool)>, // (old, new, matched)

    // ── 执行 ──
    is_running: bool,
    status_message: String,
    result_summary: String,
    output_excel_path: Option<String>,

    // ── 临时 ──
    show_variable_panel: bool,
    variable_to_add: Option<String>,
}

impl Default for BatchRenameApp {
    fn default() -> Self {
        Self {
            current_step: 0,
            data_headers: Vec::new(),
            data_rows: Vec::new(),
            data_path: None,
            data_loaded: false,
            files: Vec::new(),
            file_match_results: Vec::new(),
            matched_column: None,
            template_parts: Vec::new(),
            seq_enabled: false,
            seq_start: 1,
            seq_digits: 3,
            time_enabled: false,
            time_format: "YYYYMMDD".to_string(),
            custom_text: String::new(),
            preview_items: Vec::new(),
            is_running: false,
            status_message: "准备就绪，请先导入数据源".to_string(),
            result_summary: String::new(),
            output_excel_path: None,
            show_variable_panel: true,
            variable_to_add: None,
        }
    }
}

// ─── eframe::App 实现 ─────────────────────────────────────

impl eframe::App for BatchRenameApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 处理拖拽文件
        self.handle_dropped_files(ctx);

        // 顶部标题栏
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading("📋 批量文件重命名工具");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.data_loaded {
                        ui.colored_label(COLOR_SUCCESS, "● 已加载数据");
                    }
                });
            });
            ui.add_space(4.0);
        });

        // 步骤导航
        egui::TopBottomPanel::top("nav_bar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                self.step_button(ui, 0, "📥 导入数据");
                ui.label(" ▶ ");
                self.step_button(ui, 1, "📄 添加文件");
                ui.label(" ▶ ");
                self.step_button(ui, 2, "✏️ 重命名规则");
                ui.add_space(8.0);
            });
            ui.add_space(4.0);
            ui.separator();
        });

        // 底部状态栏
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.separator();
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(&self.status_message);
                if let Some(path) = &self.output_excel_path {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("📂 打开输出目录").clicked() {
                            if let Some(parent) = Path::new(path).parent() {
                                let _ = open::that(parent);
                            }
                        }
                    });
                }
                ui.add_space(8.0);
            });
        });

        // 主内容区
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.current_step {
                    0 => self.render_step0(ui),
                    1 => self.render_step1(ui),
                    2 => self.render_step2(ui),
                    _ => {}
                }
            });
        });

        // 延迟添加变量（避免在迭代中修改模板）
        if let Some(col) = self.variable_to_add.take() {
            self.template_parts
                .push(TemplatePart::Column { name: col });
            self.update_preview();
        }
    }
}

// ─── 辅助方法 ─────────────────────────────────────────────

impl BatchRenameApp {
    fn step_button(&mut self, ui: &mut egui::Ui, step: usize, label: &str) {
        let is_current = self.current_step == step;
        let can_go = match step {
            0 => true,
            1 => self.data_loaded,
            2 => self.data_loaded && !self.files.is_empty(),
            _ => false,
        };

        let btn = if is_current {
            egui::Button::new(label).fill(COLOR_PRIMARY)
        } else {
            egui::Button::new(label)
        };

        if ui.add_enabled(can_go, btn).clicked() && can_go {
            self.current_step = step;
            if step == 2 {
                self.run_matching();
                self.update_preview();
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }

        let paths: Vec<String> = dropped
            .iter()
            .filter_map(|f| f.path.as_ref())
            .filter_map(|p| p.to_str().map(|s| s.to_string()))
            .collect();

        if paths.is_empty() {
            return;
        }

        // 判断拖入的是数据文件还是目标文件
        let is_data_file = paths.iter().any(|p| {
            let ext = Path::new(p)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            matches!(ext.to_lowercase().as_str(), "xlsx" | "xls" | "csv")
        });

        if !self.data_loaded && is_data_file {
            // 拖入的是数据文件
            for p in paths {
                if self.load_data(&p) {
                    break;
                }
            }
        } else if self.data_loaded {
            // 拖入的是待重命名文件
            for p in paths {
                let path = Path::new(&p);
                if path.is_dir() {
                    self.add_folder_recursive(&p);
                } else if path.is_file() {
                    if !self.files.contains(&p) {
                        self.files.push(p);
                    }
                }
            }
            self.status_message = format!("已添加 {} 个文件", self.files.len());
        }
    }

    fn add_folder_recursive(&mut self, dir: &str) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    self.add_folder_recursive(&path.to_string_lossy());
                } else if path.is_file() {
                    let p = path.to_string_lossy().to_string();
                    if !self.files.contains(&p) {
                        self.files.push(p);
                    }
                }
            }
        }
    }
}

// ─── Step 0: 导入数据 ────────────────────────────────────

impl BatchRenameApp {
    fn render_step0(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.heading("📥 导入数据源");
        ui.label("选择或拖拽 Excel / CSV 文件作为数据源");
        ui.add_space(8.0);

        // 数据区域
        egui::Frame::group(ui.style())
            .fill(COLOR_BG_CARD)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("📂 选择文件").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Excel / CSV", &["xlsx", "xls", "csv"])
                            .pick_file()
                        {
                            self.load_data(&path.to_string_lossy());
                        }
                    }
                    ui.label("或拖拽文件到此处");
                    if self.data_loaded {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🗑 清除数据").clicked() {
                                self.clear_data();
                            }
                        });
                    }
                });
                ui.add_space(4.0);

                if let Some(path) = &self.data_path {
                    ui.label(format!("📎 {}", path));
                }
            });

        ui.add_space(12.0);

        // 数据预览
        if self.data_loaded {
            ui.label(format!(
                "共 {} 列 × {} 行",
                self.data_headers.len(),
                self.data_rows.len()
            ));
            ui.add_space(4.0);

            let mut table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .min_scrolled_height(200.0);

            // 列宽
            for _ in &self.data_headers {
                table = table.column(Column::auto().resizable(true).clip(true));
            }

            table
                .header(22.0, |mut header| {
                    for h in &self.data_headers {
                        header.col(|ui| {
                            ui.strong(h);
                        });
                    }
                })
                .body(|mut body| {
                    let max_rows = self.data_rows.len().min(500);
                    for row_idx in 0..max_rows {
                        body.row(20.0, |mut row| {
                            if let Some(row_data) = self.data_rows.get(row_idx) {
                                for cell in row_data {
                                    row.col(|ui| {
                                        ui.label(cell);
                                    });
                                }
                            }
                        });
                    }
                });

            ui.add_space(8.0);
            if self.data_rows.len() > 500 {
                ui.label(format!(
                    "（仅显示前 500 行，共 {} 行）",
                    self.data_rows.len()
                ));
            }

            ui.add_space(8.0);
            if ui
                .add(egui::Button::new("下一步 → 添加文件").fill(COLOR_PRIMARY))
                .clicked()
            {
                self.current_step = 1;
            }
        }
    }

    fn load_data(&mut self, path: &str) -> bool {
        match data::load_file(path) {
            Ok((headers, rows)) => {
                self.data_headers = headers;
                self.data_rows = rows;
                self.data_path = Some(path.to_string());
                self.data_loaded = true;
                self.current_step = 1;
                self.status_message = format!("已加载数据：{} 列 × {} 行", self.data_headers.len(), self.data_rows.len());
                true
            }
            Err(e) => {
                self.status_message = format!("❌ 加载失败: {}", e);
                false
            }
        }
    }

    fn clear_data(&mut self) {
        self.data_headers.clear();
        self.data_rows.clear();
        self.data_path = None;
        self.data_loaded = false;
        self.files.clear();
        self.file_match_results.clear();
        self.matched_column = None;
        self.template_parts.clear();
        self.preview_items.clear();
        self.output_excel_path = None;
        self.result_summary.clear();
        self.status_message = "准备就绪，请先导入数据源".to_string();
    }
}

// ─── Step 1: 添加文件 ────────────────────────────────────

impl BatchRenameApp {
    fn render_step1(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.heading("📄 添加待重命名的文件");
        ui.label("选择、拖拽文件或文件夹");
        ui.add_space(8.0);

        // 操作区
        egui::Frame::group(ui.style())
            .fill(COLOR_BG_CARD)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("📁 添加文件").clicked() {
                        if let Some(files) = rfd::FileDialog::new()
                            .add_filter("所有文件", &["*"])
                            .pick_files()
                        {
                            for f in files {
                                let p = f.to_string_lossy().to_string();
                                if !self.files.contains(&p) {
                                    self.files.push(p);
                                }
                            }
                            self.status_message = format!("已添加 {} 个文件", self.files.len());
                        }
                    }

                    if ui.button("📂 添加文件夹").clicked() {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            let p = dir.to_string_lossy().to_string();
                            self.add_folder_recursive(&p);
                            self.status_message = format!("已添加 {} 个文件", self.files.len());
                        }
                    }

                    if !self.files.is_empty() {
                        if ui.button("🗑 清空列表").clicked() {
                            self.files.clear();
                            self.file_match_results.clear();
                            self.matched_column = None;
                            self.preview_items.clear();
                            self.status_message = "已清空文件列表".to_string();
                        }
                    }
                });
                ui.add_space(4.0);
                ui.label(format!("已添加 {} 个文件", self.files.len()));
            });

        ui.add_space(12.0);

        // 文件列表
        if !self.files.is_empty() {
            ui.label("文件列表（自动匹配数据行）:");
            ui.add_space(4.0);

            let total = self.files.len();
            let matched = self
                .file_match_results
                .iter()
                .filter(|r| r.1.is_some())
                .count();

            if matched > 0 {
                ui.colored_label(
                    COLOR_SUCCESS,
                    format!("匹配率: {}/{} ({:.0}%)", matched, total, matched as f64 / total as f64 * 100.0),
                );
            } else if self.matched_column.is_some() {
                ui.colored_label(COLOR_WARNING, "暂无文件匹配成功，请检查文件名与数据列内容是否对应");
            }

            ui.add_space(4.0);

            let table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .min_scrolled_height(200.0)
                .column(Column::auto().at_least(30.0).clip(true))
                .column(Column::remainder())
                .column(Column::auto().at_least(80.0));

            table
                .header(22.0, |mut header| {
                    header.col(|ui| { ui.strong("状态"); });
                    header.col(|ui| { ui.strong("文件名"); });
                    header.col(|ui| { ui.strong("操作"); });
                })
                .body(|mut body| {
                    let to_remove = std::cell::Cell::new(None::<usize>);
                    for (idx, fname) in self.files.iter().enumerate() {
                        body.row(24.0, |mut row| {
                            // 匹配状态
                            row.col(|ui| {
                                let matched = self
                                    .file_match_results
                                    .get(idx)
                                    .and_then(|r| r.1);
                                if matched.is_some() {
                                    ui.label("✅");
                                } else {
                                    ui.label("❌");
                                }
                            });

                            // 文件名
                            row.col(|ui| {
                                let name = Path::new(fname)
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or(fname);
                                ui.label(name);
                            });

                            // 删除按钮
                            row.col(|ui| {
                                let idx_remove = idx;
                                if ui.button("✕").clicked() {
                                    to_remove.set(Some(idx_remove));
                                }
                            });
                        });
                    }

                    if let Some(idx) = to_remove.into_inner() {
                        self.files.remove(idx);
                        self.file_match_results.clear();
                        self.matched_column = None;
                    }
                });

            ui.add_space(8.0);
            if ui
                .add(egui::Button::new("下一步 → 重命名规则").fill(COLOR_PRIMARY))
                .clicked()
            {
                self.run_matching();
                self.current_step = 2;
                self.update_preview();
            }
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label("拖拽文件到此处，或点击上方按钮添加");
                ui.add_space(60.0);
            });
        }
    }

    fn run_matching(&mut self) {
        if self.data_rows.is_empty() || self.files.is_empty() {
            return;
        }

        // 找最佳匹配列
        self.matched_column = matcher::find_best_column(
            &self.files,
            &self.data_headers,
            &self.data_rows,
        );

        let col = match self.matched_column {
            Some(c) => c,
            None => {
                self.file_match_results = self
                    .files
                    .iter()
                    .map(|f| (f.clone(), None, Vec::new()))
                    .collect();
                self.status_message = "未找到匹配列，请检查数据".to_string();
                return;
            }
        };

        self.file_match_results = matcher::match_files_to_rows(
            &self.files,
            &self.data_headers,
            &self.data_rows,
            col,
        );

        let matched_count = self
            .file_match_results
            .iter()
            .filter(|r| r.1.is_some())
            .count();

        self.status_message = format!(
            "匹配完成：{}/{} 个文件匹配到数据（列: {}）",
            matched_count,
            self.files.len(),
            self.data_headers.get(col).unwrap_or(&"?".to_string())
        );
    }
}

// ─── Step 2: 重命名规则 ──────────────────────────────────

impl BatchRenameApp {
    fn render_step2(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.heading("✏️ 构建重命名规则");
        ui.add_space(8.0);

        // ── 数据列卡片（可点击添加到模板） ──
        egui::Frame::group(ui.style())
            .fill(COLOR_BG_CARD)
            .show(ui, |ui| {
                ui.label("点击列名添加到模板：");
                ui.add_space(4.0);
                let headers = self.data_headers.clone();
                ui.horizontal_wrapped(|ui| {
                    for header in &headers {
                        let label = format!("【{}】", header);
                        if ui.button(label).clicked() {
                            self.template_parts
                                .push(TemplatePart::Column { name: header.clone() });
                            self.update_preview();
                        }
                    }
                });
                    ui.label("  ");
                    if ui.button("📅 时间戳").clicked() {
                        self.template_parts.push(TemplatePart::Timestamp);
                        self.update_preview();
                    }
                    if ui.button("# 序号").clicked() {
                        self.template_parts.push(TemplatePart::Sequence);
                        self.update_preview();
                    }
                });
            });

        ui.add_space(8.0);

        // ── 模板编辑 ──
        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                ui.label("当前模板（点击 × 删除片段）：");
                ui.add_space(4.0);

                // 显示已添加的模板片段
                if self.template_parts.is_empty() {
                    ui.label("（空 — 点击上方列名添加）");
                } else {
                    let mut remove_idx = None;
                    ui.horizontal_wrapped(|ui| {
                        for (idx, part) in self.template_parts.iter().enumerate() {
                            let (label, color) = match part {
                                TemplatePart::Column { name } => {
                                    (format!("[{}]", name), COLOR_PRIMARY)
                                }
                                TemplatePart::Text(t) => {
                                    (format!("「{}」", t), egui::Color32::GRAY)
                                }
                                TemplatePart::Sequence => ("#序号".to_string(), COLOR_SUCCESS),
                                TemplatePart::Timestamp => ("📅时间".to_string(), COLOR_WARNING),
                            };

                            let resp = ui.add(
                                egui::Button::new(format!("{} ×", label))
                                    .fill(color)
                                    .small(),
                            );
                            if resp.clicked() {
                                remove_idx = Some(idx);
                            }
                        }
                    });

                    if let Some(idx) = remove_idx {
                        self.template_parts.remove(idx);
                        self.update_preview();
                    }
                }

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                // 自定义文本输入
                ui.horizontal(|ui| {
                    ui.label("添加文本：");
                    if ui
                        .add(egui::TextEdit::singleline(&mut self.custom_text).desired_width(200.0))
                        .lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        if !self.custom_text.is_empty() {
                            self.template_parts.push(TemplatePart::Text(
                                self.custom_text.clone(),
                            ));
                            self.custom_text.clear();
                            self.update_preview();
                        }
                    }
                    if ui.button("➕ 添加").clicked() && !self.custom_text.is_empty() {
                        self.template_parts
                            .push(TemplatePart::Text(self.custom_text.clone()));
                        self.custom_text.clear();
                        self.update_preview();
                    }
                    if ui.button("🗑 清空模板").clicked() {
                        self.template_parts.clear();
                        self.update_preview();
                    }
                });
            });

        ui.add_space(8.0);

        // ── 序号 & 时间戳配置 ──
        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // 序号配置
                    ui.checkbox(&mut self.seq_enabled, "🔢 序号");
                    if self.seq_enabled {
                        ui.add(
                            egui::DragValue::new(&mut self.seq_start)
                                .prefix("起始: ")
                                .range(0..=9999),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.seq_digits)
                                .prefix("位数: ")
                                .range(1..=10),
                        );
                    }

                    ui.separator();

                    // 时间戳配置
                    ui.checkbox(&mut self.time_enabled, "📅 时间戳");
                    if self.time_enabled {
                        let formats = [
                            "YYYYMMDD",
                            "YYYY-MM-DD",
                            "YYYYMMDD_HHMMSS",
                            "YYYY-MM-DD_HH-MM-SS",
                            "YYMMDD",
                        ];
                        egui::ComboBox::from_id_salt("time_fmt")
                            .selected_text(&self.time_format)
                            .show_ui(ui, |ui| {
                                for fmt in &formats {
                                    ui.selectable_value(
                                        &mut self.time_format,
                                        fmt.to_string(),
                                        *fmt,
                                    );
                                }
                            });
                    }
                });
            });

        ui.add_space(8.0);

        // ── 预览 ──
        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                ui.label("实时预览（原文件 → 新文件名）:");
                ui.add_space(4.0);

                let table = TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .min_scrolled_height(180.0)
                    .column(Column::auto().at_least(30.0))
                    .column(Column::remainder())
                    .column(Column::remainder());

                table
                    .header(22.0, |mut header| {
                        header.col(|ui| { ui.strong("状态"); });
                        header.col(|ui| { ui.strong("原文件名"); });
                        header.col(|ui| { ui.strong("新文件名"); });
                    })
                    .body(|mut body| {
                        for (old, new_name, matched) in &self.preview_items {
                            body.row(20.0, |mut row| {
                                row.col(|ui| {
                                    if *matched {
                                        ui.label("✅");
                                    } else {
                                        ui.label("❌");
                                    }
                                });
                                row.col(|ui| {
                                    let name = Path::new(old)
                                        .file_name()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or(old);
                                    ui.label(name);
                                });
                                row.col(|ui| {
                                    if new_name.is_empty() {
                                        ui.colored_label(COLOR_DANGER, "(未匹配)");
                                    } else {
                                        let ext = Path::new(old)
                                            .extension()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or("");
                                        let display = if ext.is_empty() {
                                            new_name.clone()
                                        } else {
                                            format!("{}.{}", new_name, ext)
                                        };
                                        ui.colored_label(COLOR_PRIMARY, display);
                                    }
                                });
                            });
                        }
                    });
            });

        ui.add_space(12.0);

        // ── 执行按钮 ──
        let has_template = !self.template_parts.is_empty();
        let can_execute = has_template && !self.files.is_empty() && !self.is_running;

        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new("▶ 执行重命名")
                        .fill(if can_execute { COLOR_PRIMARY } else { egui::Color32::GRAY })
                        .min_size(egui::vec2(180.0, 36.0)),
                )
                .clicked()
                && can_execute
            {
                self.execute_rename();
            }

            if !self.result_summary.is_empty() {
                ui.colored_label(COLOR_SUCCESS, &self.result_summary);
            }
        });
    }

    fn update_preview(&mut self) {
        self.preview_items.clear();

        if self.template_parts.is_empty() {
            // 即使没有模板，也显示文件列表
            for fname in &self.files {
                let _name = Path::new(fname)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(fname)
                    .to_string();
                self.preview_items
                    .push((fname.clone(), String::new(), false));
            }
            return;
        }

        let timestamp = if self.time_enabled {
            Some(format_timestamp(&self.time_format))
        } else {
            None
        };

        for (fname, matched_row, row_data) in &self.file_match_results {
            let matched = matched_row.is_some();
            if matched {
                let mut data_map = HashMap::new();
                for (k, v) in row_data {
                    data_map.insert(k.clone(), v.clone());
                }

                let new_name = renamer::build_new_name(
                    &data_map,
                    &self.template_parts,
                    None, // seq 在预览时不带序号
                    self.seq_digits,
                    timestamp.as_deref(),
                );
                self.preview_items
                    .push((fname.clone(), new_name, true));
            } else {
                let _name = Path::new(fname)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(fname)
                    .to_string();
                self.preview_items
                    .push((fname.clone(), String::new(), false));
            }
        }
    }

    fn execute_rename(&mut self) {
        let timestamp = if self.time_enabled {
            Some(format_timestamp(&self.time_format))
        } else {
            None
        };

        let mut seq_num = self.seq_start;
        let mut rename_inputs = Vec::new();

        for (fname, matched_row, row_data) in &self.file_match_results {
            if matched_row.is_none() {
                rename_inputs.push(renamer::RenameInput {
                    old_path: fname.clone(),
                    new_name: String::new(),
                });
                continue;
            }

            let mut data_map = HashMap::new();
            for (k, v) in row_data {
                data_map.insert(k.clone(), v.clone());
            }

            let seq = if self.seq_enabled {
                let n = seq_num;
                seq_num += 1;
                Some(n)
            } else {
                None
            };

            let new_name = renamer::build_new_name(
                &data_map,
                &self.template_parts,
                seq,
                self.seq_digits,
                timestamp.as_deref(),
            );
            rename_inputs.push(renamer::RenameInput {
                old_path: fname.clone(),
                new_name,
            });
        }

        // 确定输出目录（使用第一个文件所在目录）
        let output_dir = self
            .files
            .first()
            .map(|f| {
                Path::new(f)
                    .parent()
                    .unwrap_or(Path::new("."))
                    .to_string_lossy()
                    .to_string()
            })
            .unwrap_or_else(|| ".".to_string());

        let output_path = Path::new(&output_dir)
            .join("重命名结果.xlsx")
            .to_string_lossy()
            .to_string();

        self.is_running = true;
        self.status_message = "正在重命名...".to_string();

        // 执行重命名
        let (results, summary) = renamer::execute_rename(&rename_inputs, &output_dir);

        // 输出 Excel
        match renamer::write_output_excel(&results, &output_path) {
            Ok(path) => {
                self.output_excel_path = Some(path);
            }
            Err(e) => {
                self.status_message = format!("❌ 输出 Excel 失败: {}", e);
            }
        }

        let success = results.iter().filter(|r| r.success).count();
        let fail = results.iter().filter(|r| !r.success).count();
        self.result_summary = format!("✅ {} 成功 / ❌ {} 失败", success, fail);
        self.status_message = format!(
            "重命名完成！{} 输出文件: {}",
            summary, output_path
        );
        self.is_running = false;
    }
}

// ─── 工具函数 ─────────────────────────────────────────────

fn format_timestamp(fmt: &str) -> String {
    use chrono::Local;
    let now = Local::now();
    match fmt {
        "YYYYMMDD" => now.format("%Y%m%d").to_string(),
        "YYYY-MM-DD" => now.format("%Y-%m-%d").to_string(),
        "YYYYMMDD_HHMMSS" => now.format("%Y%m%d_%H%M%S").to_string(),
        "YYYY-MM-DD_HH-MM-SS" => now.format("%Y-%m-%d_%H-%M-%S").to_string(),
        "YYMMDD" => now.format("%y%m%d").to_string(),
        _ => now.format("%Y%m%d").to_string(),
    }
}
