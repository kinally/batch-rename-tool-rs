/// 重命名执行 + Excel 输出模块

use rust_xlsxwriter::*;
use std::collections::HashMap;
/// 模板片段类型
#[derive(Clone, Debug)]
pub enum TemplatePart {
    /// 数据列值
    Column { name: String },
    /// 固定文本
    Text(String),
    /// 序号
    Sequence,
    /// 时间戳
    Timestamp,
}

/// 清理不适合作为文件名的字符
fn sanitize(s: &str) -> String {
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*', '\0'];
    s.chars()
        .filter(|c| !invalid_chars.contains(c))
        .collect::<String>()
        .trim()
        .to_string()
}

/// 构建新文件名
pub fn build_new_name(
    row_data: &HashMap<String, String>,
    template_parts: &[TemplatePart],
    seq_num: Option<u32>,
    seq_digits: u32,
    timestamp_str: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    for part in template_parts {
        match part {
            TemplatePart::Column { name } => {
                let val = row_data.get(name.as_str()).map(|s| s.as_str()).unwrap_or("");
                parts.push(sanitize(val));
            }
            TemplatePart::Text(text) => {
                parts.push(text.clone());
            }
            TemplatePart::Sequence => {
                if let Some(n) = seq_num {
                    parts.push(format!("{:0width$}", n, width = seq_digits as usize));
                }
            }
            TemplatePart::Timestamp => {
                if let Some(ts) = timestamp_str {
                    parts.push(ts.to_string());
                }
            }
        }
    }

    let name: String = parts.iter().flat_map(|s| s.chars()).collect();
    let name = name.trim().trim_matches(|c: char| c == '_' || c == '-' || c == ' ').to_string();
    if name.is_empty() { "unnamed".to_string() } else { name }
}

/// 单次重命名的结果
#[derive(Clone)]
pub struct RenameResult {
    pub old_path: String,
    pub old_name: String,
    pub new_name: String,
    pub new_path: String,
    pub success: bool,
    pub error: String,
}

/// 执行重命名
///
/// 将所有文件复制到输出目录并重命名
pub fn execute_rename(
    files: &[RenameInput],
    output_dir: &str,
) -> (Vec<RenameResult>, String) {
    let mut results = Vec::new();
    let mut success_count = 0;
    let mut fail_count = 0;
    let mut no_change_count = 0;

    // 确保输出目录存在
    let _ = std::fs::create_dir_all(output_dir);

    for input in files {
        let old_path = &input.old_path;
        let new_name = &input.new_name;
        let old_name = std::path::Path::new(old_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        if new_name.is_empty() {
            results.push(RenameResult {
                old_path: old_path.clone(),
                old_name,
                new_name: String::new(),
                new_path: old_path.clone(),
                success: false,
                error: "未生成新文件名".to_string(),
            });
            fail_count += 1;
            continue;
        }

        let ext = std::path::Path::new(old_path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let final_name = if ext.is_empty() {
            new_name.clone()
        } else {
            format!("{}.{}", new_name, ext)
        };

        let new_path = std::path::Path::new(output_dir).join(&final_name);
        let new_path_str = new_path.to_string_lossy().to_string();

        match std::fs::copy(old_path, &new_path) {
            Ok(_) => {
                let changed = old_name != final_name;
                results.push(RenameResult {
                    old_path: old_path.clone(),
                    old_name,
                    new_name: final_name,
                    new_path: new_path_str,
                    success: true,
                    error: String::new(),
                });
                if changed {
                    success_count += 1;
                } else {
                    no_change_count += 1;
                }
            }
            Err(e) => {
                results.push(RenameResult {
                    old_path: old_path.clone(),
                    old_name,
                    new_name: final_name,
                    new_path: new_path_str,
                    success: false,
                    error: format!("复制失败: {}", e),
                });
                fail_count += 1;
            }
        }
    }

    let summary = format!(
        "成功: {} 个, 未变化: {} 个, 失败: {} 个",
        success_count, no_change_count, fail_count
    );

    (results, summary)
}

/// 重命名输入
pub struct RenameInput {
    pub old_path: String,
    pub new_name: String,
}

/// 输出结果到 Excel 文件
pub fn write_output_excel(
    results: &[RenameResult],
    output_path: &str,
) -> Result<String, String> {
    let mut workbook = Workbook::new();

    // ---- 成功 sheet ----
    let mut sheet_success = Worksheet::new();
    sheet_success.set_name("重命名成功").map_err(|e| e.to_string())?;
    sheet_success.write_string(0, 0, "原文件名").map_err(|e| e.to_string())?;
    sheet_success.write_string(0, 1, "新文件名").map_err(|e| e.to_string())?;
    sheet_success.write_string(0, 2, "状态").map_err(|e| e.to_string())?;
    sheet_success.write_string(0, 3, "路径").map_err(|e| e.to_string())?;

    let mut success_row = 1;
    for r in results {
        if r.success {
            let status = if r.old_name != r.new_name { "成功" } else { "未变化" };
            sheet_success
                .write_string(success_row, 0, &r.old_name)
                .map_err(|e| e.to_string())?;
            sheet_success
                .write_string(success_row, 1, &r.new_name)
                .map_err(|e| e.to_string())?;
            sheet_success
                .write_string(success_row, 2, status)
                .map_err(|e| e.to_string())?;
            sheet_success
                .write_string(success_row, 3, &r.new_path)
                .map_err(|e| e.to_string())?;
            success_row += 1;
        }
    }

    if success_row == 1 {
        sheet_success
            .write_string(1, 0, "无成功记录")
            .map_err(|e| e.to_string())?;
    }

    // ---- 失败 sheet ----
    let mut sheet_fail = Worksheet::new();
    sheet_fail.set_name("重命名失败").map_err(|e| e.to_string())?;
    sheet_fail.write_string(0, 0, "原文件名").map_err(|e| e.to_string())?;
    sheet_fail.write_string(0, 1, "新文件名").map_err(|e| e.to_string())?;
    sheet_fail.write_string(0, 2, "错误原因").map_err(|e| e.to_string())?;
    sheet_fail.write_string(0, 3, "原路径").map_err(|e| e.to_string())?;

    let mut fail_row = 1;
    for r in results {
        if !r.success {
            sheet_fail
                .write_string(fail_row, 0, &r.old_name)
                .map_err(|e| e.to_string())?;
            sheet_fail
                .write_string(fail_row, 1, &r.new_name)
                .map_err(|e| e.to_string())?;
            sheet_fail
                .write_string(fail_row, 2, &r.error)
                .map_err(|e| e.to_string())?;
            sheet_fail
                .write_string(fail_row, 3, &r.old_path)
                .map_err(|e| e.to_string())?;
            fail_row += 1;
        }
    }

    if fail_row == 1 {
        sheet_fail
            .write_string(1, 0, "无失败记录")
            .map_err(|e| e.to_string())?;
    }

    workbook.push_worksheet(sheet_success);
    workbook.push_worksheet(sheet_fail);

    workbook
        .save(output_path)
        .map_err(|e| format!("保存 Excel 失败: {}", e))?;

    Ok(output_path.to_string())
}
