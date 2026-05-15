# 批量文件重命名工具 (Rust 版) 📋

> 基于 Excel/CSV 数据列的批量文件重命名工具，原生 Windows 图形界面。
>
> Rust + egui 重写版，**打包后单个 exe 约 3~8MB**（相比 Python 版的 100MB+）。

## 功能

| # | 功能 | 说明 |
|---|------|------|
| 1 | 导入数据源 | 支持 `.xlsx` / `.xls` / `.csv` 文件，拖拽或选择 |
| 2 | 自动识别变量 | 读取表头作为重命名可调用的变量 |
| 3 | 拖拽文件/文件夹 | 支持拖拽添加文件，也支持按钮选取 |
| 4 | 智能匹配 | 自动匹配文件名与数据列，支持全角/半角分隔符 |
| 5 | 可视化组合规则 | 点击列名卡片 + 文本输入，直观构建重命名模板 |
| 6 | 序号 & 时间戳 | 内置序号（自定义起始/位数）和时间戳格式 |
| 7 | 实时预览 | 修改规则时即时预览新文件名 |
| 8 | 输出结果 Excel | 成功/失败分 sheet 存放，与原文件同目录 |

## 编译（在任何有 Rust 环境的机器上）

```bash
# 克隆或复制项目后
cd batch-rename-tool-rs

# 调试运行
cargo run

# 发布构建（优化体积）
cargo build --release
```

打包后的 exe 在 `target/release/batch-rename-tool.exe`。

### 可选：打包为更小的 exe

```bash
# Release + LTO 压缩
cargo build --release
strip target/release/batch-rename-tool.exe  # 需要 strip 工具（MSVC 工具链自带）
```

## 使用方法

1. **导入数据**：点击"选择文件"或拖拽 Excel/CSV 到窗口
2. **添加文件**：点击"添加文件/文件夹"或拖拽文件
3. **构建规则**：
   - 点击列名卡片将数据列添加到模板
   - 手动输入分隔文本
   - 可选启用序号和时间戳
4. **执行重命名**：预览无误后点击"执行重命名"
5. 结果文件 `重命名结果.xlsx` 生成在原文件所在目录

## 依赖

| 库 | 用途 |
|----|------|
| eframe / egui | 图形界面 |
| calamine | Excel/CSV 读取（纯 Rust） |
| rust_xlsxwriter | Excel 写入 |
| rfd | 原生文件对话框 |
| chrono | 时间戳格式化 |
| csv | CSV 解析 |

## 对比 Python 版

| 对比项 | Python 版 | Rust 版 |
|--------|-----------|---------|
| 运行时 | 需要 Python 解释器 | 无（原生 exe） |
| 打包体积 | 50~200 MB | 3~8 MB |
| 内存占用 | ~100 MB | ~10 MB |
| 启动速度 | 2~5 秒 | 瞬时 |
| 依赖库 | PySide6 + pandas + openpyxl | eframe + calamine + rust_xlsxwriter |
