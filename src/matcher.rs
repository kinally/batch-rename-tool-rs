/// 文件名匹配模块
///
/// 核心逻辑：
/// 1. 对每个文件名（去扩展名），尝试匹配到某列的值
/// 2. 支持全角/半角空格、短横线等分隔符
/// 3. 找出最佳匹配列

use std::collections::HashSet;

/// 常见分隔符（含全角版本）
const SEPARATORS: &[&str] = &[
    " ", "　",   // 半角/全角空格
    "-", "－", "–", // 半角/全角短横线
    "_", "＿",   // 半角/全角下划线
    ".", "．",   // 半角/全角点
    "~", "～",   // 波浪线
    "·",         // 间隔号
    ",", "，",   // 逗号
    "、",        // 顿号
    "|", "｜",   // 竖线
    "/", "／",   // 斜线
];

/// 规范化字符串：全角字母数字转半角
fn normalize(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        let code = ch as u32;
        if (0xFF01..=0xFF5E).contains(&code) {
            result.push(char::from_u32(code - 0xFEE0).unwrap_or(ch));
        } else {
            result.push(ch);
        }
    }
    result
}

/// 按分隔符拆分文件名
fn split_filename(name: &str) -> Vec<String> {
    let name = normalize(name);
    // 用所有分隔符拆分
    let mut parts = vec![name.as_str()];
    for &sep in SEPARATORS {
        let mut new_parts = Vec::new();
        for part in parts {
            let sub: Vec<&str> = part.split(sep).collect();
            new_parts.extend(sub);
        }
        parts = new_parts;
    }
    parts
        .into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// 检查一个文件名是否匹配某列的某个值
fn match_score(filename_stem: &str, value: &str) -> bool {
    // 策略1: 直接匹配
    if filename_stem == value {
        return true;
    }

    let normalized_stem = normalize(filename_stem);
    let normalized_val = normalize(value);

    // 策略2: 归一化后匹配
    if normalized_stem == normalized_val {
        return true;
    }

    // 策略3: 按分隔符拆分后匹配
    let parts = split_filename(filename_stem);
    for part in &parts {
        if part == value || part == &normalized_val {
            return true;
        }
    }

    // 策略4: 值作为子串出现在文件名中（如 "发票编号INV001" 匹配 "INV001"）
    if !value.is_empty() && filename_stem.contains(value) {
        return true;
    }
    if !normalized_val.is_empty() && normalized_stem.contains(&normalized_val) {
        return true;
    }

    false
}

/// 找出最佳匹配列
///
/// 对每个文件，统计各列匹配的文件数，返回匹配文件数最多的列索引
pub fn find_best_column(
    filenames: &[String],
    headers: &[String],
    rows: &[Vec<String>],
) -> Option<usize> {
    if headers.is_empty() || rows.is_empty() || filenames.is_empty() {
        return None;
    }

    let file_stems: Vec<String> = filenames
        .iter()
        .map(|f| {
            let path = std::path::Path::new(f);
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(f)
                .to_string()
        })
        .collect();

    let mut best_col = None;
    let mut best_count = 0;

    for col_idx in 0..headers.len() {
        // 收集该列的所有值
        let values: HashSet<String> = rows
            .iter()
            .filter_map(|row| row.get(col_idx))
            .filter(|v| !v.trim().is_empty())
            .map(|v| normalize(v.trim()))
            .collect();

        if values.is_empty() {
            continue;
        }

        let mut matched_count = 0;
        for stem in &file_stems {
            for val in &values {
                if match_score(stem, val) {
                    matched_count += 1;
                    break;
                }
            }
        }

        if matched_count > best_count {
            best_count = matched_count;
            best_col = Some(col_idx);
        }
    }

    best_col
}

/// 为每个文件名找到匹配行的完整数据
///
/// 返回: Vec<(filename, Option<row_index>, HashMap<col_name, value>)>
pub fn match_files_to_rows(
    filenames: &[String],
    headers: &[String],
    rows: &[Vec<String>],
    matched_column: usize,
) -> Vec<(String, Option<usize>, Vec<(String, String)>)> {
    if matched_column >= headers.len() {
        return filenames
            .iter()
            .map(|f| (f.clone(), None, Vec::new()))
            .collect();
    }

    // 收集匹配列的所有值
    let col_values: Vec<String> = rows
        .iter()
        .map(|row| {
            row.get(matched_column)
                .map(|v| normalize(v.trim()))
                .unwrap_or_default()
        })
        .collect();

    let file_stems: Vec<String> = filenames
        .iter()
        .map(|f| {
            std::path::Path::new(f)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(f)
                .to_string()
        })
        .collect();

    let mut results = Vec::new();
    for (fname, stem) in filenames.iter().zip(file_stems.iter()) {
        let mut matched_row = None;
        let mut row_data = Vec::new();

        // 找匹配行
        for (idx, val) in col_values.iter().enumerate() {
            if !val.is_empty() && match_score(stem, val) {
                matched_row = Some(idx);
                break;
            }
        }

        if let Some(row_idx) = matched_row {
            if let Some(row) = rows.get(row_idx) {
                for (col_idx, header) in headers.iter().enumerate() {
                    let val = row.get(col_idx).cloned().unwrap_or_default();
                    row_data.push((header.clone(), val));
                }
            }
        }

        results.push((fname.clone(), matched_row, row_data));
    }

    results
}
