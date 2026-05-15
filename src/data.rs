/// 数据加载模块
///
/// 支持 .xlsx / .xls / .csv 格式的读取

use calamine::{open_workbook, Reader, Xlsx, Data};
use std::path::Path;

/// 加载 Excel/CSV 文件，返回 (表头列表, 数据行列表)
pub fn load_file(path: &str) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "xlsx" | "xls" => load_excel(path),
        "csv" => load_csv(path),
        _ => Err(format!("不支持的文件格式: .{}", ext)),
    }
}

fn load_excel(path: &str) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let mut workbook: Xlsx<_> =
        open_workbook(path).map_err(|e| format!("无法打开文件: {}", e))?;

    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| "工作簿中没有工作表".to_string())?;

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| format!("读取工作表失败: {}", e))?;

    let mut rows = Vec::new();
    for row in range.rows() {
        let row_data: Vec<String> = row
            .iter()
            .map(|cell| match cell {
                Data::String(s) => s.clone(),
                Data::Float(f) => {
                    // 如果是整数值，显示为整数
                    if *f == f.floor() && f.is_finite() {
                        format!("{}", *f as i64)
                    } else {
                        format!("{}", f)
                    }
                }
                Data::Int(i) => i.to_string(),
                Data::Bool(b) => b.to_string(),
                Data::DateTime(d) => d.to_string(),
                Data::Empty => String::new(),
                _ => String::new(),
            })
            .collect();
        rows.push(row_data);
    }

    if rows.is_empty() {
        return Err("表格中没有数据".to_string());
    }

    let headers = rows.remove(0);
    Ok((headers, rows))
}

fn load_csv(path: &str) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)
        .map_err(|e| format!("无法打开 CSV 文件: {}", e))?;

    // 尝试多种编码（UTF-8 / GBK）
    // csv crate 默认用 UTF-8，如果失败提示用户转码
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| format!("读取 CSV 表头失败: {}", e))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    let mut rows = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| format!("读取 CSV 行失败: {}", e))?;
        let row: Vec<String> = record.iter().map(|f| f.to_string()).collect();
        rows.push(row);
    }

    Ok((headers, rows))
}
