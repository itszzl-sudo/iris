//! CSS Table 布局系统
//!
//! 实现 CSS Table 布局，支持：
//! - table / table-row / table-cell 显示类型
//! - 自动表格宽度分配
//! - 单元格合并（colspan / rowspan）
//! - 表格边框模型（border-collapse / border-spacing）

use crate::style::ComputedStyles;

/// 表格显示类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableDisplayType {
    /// 不是表格元素
    None,
    /// 表格
    Table,
    /// 表格行
    TableRow,
    /// 表格单元格
    TableCell,
    /// 表格行组（tbody, thead, tfoot）
    TableRowGroup,
    /// 表格列
    TableColumn,
    /// 表格列组
    TableColumnGroup,
    /// 表格标题
    TableCaption,
}

impl TableDisplayType {
    /// 从 CSS display 属性解析
    pub fn from_css(css: &str) -> Self {
        match css.trim() {
            "table" => TableDisplayType::Table,
            "table-row" => TableDisplayType::TableRow,
            "table-cell" => TableDisplayType::TableCell,
            "table-row-group" => TableDisplayType::TableRowGroup,
            "table-column" => TableDisplayType::TableColumn,
            "table-column-group" => TableDisplayType::TableColumnGroup,
            "table-caption" => TableDisplayType::TableCaption,
            _ => TableDisplayType::None,
        }
    }
}

/// 表格边框折叠模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderCollapse {
    /// 分离边框（默认）
    Separate,
    /// 折叠边框
    Collapse,
}

impl BorderCollapse {
    /// 从 CSS 字符串解析
    pub fn from_css(css: &str) -> Self {
        match css.trim() {
            "collapse" => BorderCollapse::Collapse,
            _ => BorderCollapse::Separate,
        }
    }
}

/// 表格单元格
#[derive(Debug, Clone)]
pub struct TableCell {
    /// 单元格 ID
    pub cell_id: u64,
    /// 列跨度
    pub colspan: usize,
    /// 行跨度
    pub rowspan: usize,
    /// 内容宽度
    pub content_width: f32,
    /// 内容高度
    pub content_height: f32,
    /// 计算后的 x 坐标
    pub x: f32,
    /// 计算后的 y 坐标
    pub y: f32,
    /// 计算后的宽度（包含跨度）
    pub width: f32,
    /// 计算后的高度（包含跨度）
    pub height: f32,
}

/// 表格行
#[derive(Debug, Clone)]
pub struct TableRow {
    /// 行 ID
    pub row_id: u64,
    /// 单元格列表
    pub cells: Vec<TableCell>,
    /// 行高
    pub height: f32,
    /// y 坐标
    pub y: f32,
}

/// 表格配置
#[derive(Debug, Clone)]
pub struct TableConfig {
    /// 边框折叠模式
    pub border_collapse: BorderCollapse,
    /// 边框间距（horizontal, vertical）
    pub border_spacing: (f32, f32),
    /// 表格宽度（auto 或固定值）
    pub width: Option<f32>,
}

impl TableConfig {
    /// 从 ComputedStyles 创建
    pub fn from_styles(styles: &ComputedStyles) -> Self {
        let border_collapse = styles
            .get("border-collapse")
            .map(|s| BorderCollapse::from_css(s))
            .unwrap_or(BorderCollapse::Separate);

        let border_spacing = styles
            .get("border-spacing")
            .map(|s| {
                let parts: Vec<&str> = s.split_whitespace().collect();
                let h = parts.get(0)
                    .and_then(|v| v.trim_end_matches("px").parse::<f32>().ok())
                    .unwrap_or(2.0);
                let v = parts.get(1)
                    .and_then(|v| v.trim_end_matches("px").parse::<f32>().ok())
                    .unwrap_or(h);
                (h, v)
            })
            .unwrap_or((2.0, 2.0));

        let width = styles
            .get("width")
            .and_then(|s| s.trim_end_matches("px").parse::<f32>().ok());

        Self {
            border_collapse,
            border_spacing,
            width,
        }
    }
}

/// 表格布局结果
#[derive(Debug, Clone)]
pub struct TableLayout {
    /// 表格行列表
    pub rows: Vec<TableRow>,
    /// 表格总宽度
    pub total_width: f32,
    /// 表格总高度
    pub total_height: f32,
    /// 列宽列表
    pub column_widths: Vec<f32>,
}

/// 解析单元格的 colspan 和 rowspan
pub fn parse_cell_spans(styles: &ComputedStyles) -> (usize, usize) {
    let colspan = styles
        .get("colspan")
        .or_else(|| styles.get("column-span"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);

    let rowspan = styles
        .get("rowspan")
        .or_else(|| styles.get("row-span"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);

    (colspan, rowspan)
}

/// 计算表格布局
/// 
/// # 参数
/// 
/// - `config`: 表格配置
/// - `rows_data`: 行数据，每个行包含单元格数据
///   每个单元格: (cell_id, colspan, rowspan, content_width, content_height)
/// - `container_width`: 容器宽度
/// 
/// # 返回
/// 
/// 返回表格布局结果
pub fn compute_table_layout(
    config: &TableConfig,
    rows_data: &[Vec<(u64, usize, usize, f32, f32)>],
    container_width: f32,
) -> TableLayout {
    if rows_data.is_empty() {
        return TableLayout {
            rows: Vec::new(),
            total_width: 0.0,
            total_height: 0.0,
            column_widths: Vec::new(),
        };
    }

    // 计算实际列数
    let max_cols = rows_data
        .iter()
        .map(|row| row.iter().map(|(_, colspan, _, _, _)| *colspan).sum::<usize>())
        .max()
        .unwrap_or(0);

    if max_cols == 0 {
        return TableLayout {
            rows: Vec::new(),
            total_width: 0.0,
            total_height: 0.0,
            column_widths: Vec::new(),
        };
    }

    // 初始化列宽数组
    let mut column_widths = vec![0.0f32; max_cols];

    // 第一轮：计算每列的最小宽度（基于内容）
    for row_data in rows_data {
        let mut col_idx = 0;
        for (_, colspan, _, content_width, _) in row_data {
            let avg_width = content_width / *colspan as f32;
            for i in 0..*colspan {
                if col_idx + i < max_cols {
                    column_widths[col_idx + i] = column_widths[col_idx + i].max(avg_width);
                }
            }
            col_idx += colspan;
        }
    }

    // 应用边框间距
    let total_spacing = config.border_spacing.0 * (max_cols as f32 - 1.0);
    let available_width = container_width - total_spacing;
    let current_total: f32 = column_widths.iter().sum();

    // 如果需要，按比例调整列宽以适应容器
    if config.width.is_none() && current_total < available_width {
        let scale = available_width / current_total.max(0.01);
        for width in &mut column_widths {
            *width *= scale;
        }
    }

    // 构建行和单元格
    let mut rows = Vec::new();
    let mut current_y = 0.0;

    for row_data in rows_data {
        let mut cells = Vec::new();
        let mut col_idx = 0;
        let mut row_height: f32 = 0.0;

        for (cell_id, colspan, rowspan, content_width, content_height) in row_data {
            // 计算单元格宽度（包含跨度）
            let mut cell_width = 0.0;
            for i in 0..*colspan {
                if col_idx + i < max_cols {
                    cell_width += column_widths[col_idx + i];
                }
            }
            
            // 添加跨度内的边框间距
            if *colspan > 1 {
                cell_width += config.border_spacing.0 * (*colspan as f32 - 1.0);
            }

            let cell = TableCell {
                cell_id: *cell_id,
                colspan: *colspan,
                rowspan: *rowspan,
                content_width: *content_width,
                content_height: *content_height,
                x: 0.0, // 将在第二轮计算
                y: 0.0,
                width: cell_width,
                height: *content_height, // 简化实现
            };

            row_height = row_height.max(cell.height);
            cells.push(cell);
            col_idx += colspan;
        }

        // 计算单元格的 x 坐标
        let mut x = 0.0;
        for cell in &mut cells {
            cell.x = x;
            cell.y = current_y;
            x += cell.width + config.border_spacing.0;
        }

        rows.push(TableRow {
            row_id: 0, // 简化实现
            cells,
            height: row_height,
            y: current_y,
        });

        current_y += row_height + config.border_spacing.1;
    }

    let total_width = container_width;
    let total_height = current_y - config.border_spacing.1.max(0.0);

    TableLayout {
        rows,
        total_width,
        total_height,
        column_widths,
    }
}

/// 计算单元格在表格中的绝对位置
/// 
/// # 参数
/// 
/// - `layout`: 表格布局结果
/// - `row_index`: 行索引
/// - `cell_index`: 单元格在行中的索引
/// 
/// # 返回
/// 
/// 返回 (x, y, width, height)
pub fn get_cell_absolute_position(
    layout: &TableLayout,
    row_index: usize,
    cell_index: usize,
) -> Option<(f32, f32, f32, f32)> {
    if row_index >= layout.rows.len() {
        return None;
    }

    let row = &layout.rows[row_index];
    if cell_index >= row.cells.len() {
        return None;
    }

    let cell = &row.cells[cell_index];
    Some((cell.x, cell.y, cell.width, cell.height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_display_type_parsing() {
        assert_eq!(TableDisplayType::from_css("table"), TableDisplayType::Table);
        assert_eq!(TableDisplayType::from_css("table-row"), TableDisplayType::TableRow);
        assert_eq!(TableDisplayType::from_css("table-cell"), TableDisplayType::TableCell);
        assert_eq!(TableDisplayType::from_css("div"), TableDisplayType::None);
    }

    #[test]
    fn test_border_collapse_parsing() {
        assert_eq!(BorderCollapse::from_css("collapse"), BorderCollapse::Collapse);
        assert_eq!(BorderCollapse::from_css("separate"), BorderCollapse::Separate);
    }

    #[test]
    fn test_table_config_from_styles() {
        let mut styles = ComputedStyles::new();
        styles.set("border-collapse", "collapse");
        styles.set("border-spacing", "5px 10px");
        styles.set("width", "600px");

        let config = TableConfig::from_styles(&styles);
        assert_eq!(config.border_collapse, BorderCollapse::Collapse);
        assert!((config.border_spacing.0 - 5.0).abs() < 0.01);
        assert!((config.border_spacing.1 - 10.0).abs() < 0.01);
        assert!((config.width.unwrap() - 600.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_cell_spans() {
        let mut styles = ComputedStyles::new();
        styles.set("colspan", "2");
        styles.set("rowspan", "3");

        let (colspan, rowspan) = parse_cell_spans(&styles);
        assert_eq!(colspan, 2);
        assert_eq!(rowspan, 3);
    }

    #[test]
    fn test_compute_table_layout_simple() {
        let config = TableConfig {
            border_collapse: BorderCollapse::Separate,
            border_spacing: (2.0, 2.0),
            width: None,
        };

        let rows_data = vec![
            vec![
                (1, 1, 1, 100.0, 50.0),
                (2, 1, 1, 150.0, 50.0),
            ],
            vec![
                (3, 1, 1, 120.0, 60.0),
                (4, 1, 1, 130.0, 60.0),
            ],
        ];

        let layout = compute_table_layout(&config, &rows_data, 400.0);

        assert_eq!(layout.rows.len(), 2);
        assert_eq!(layout.column_widths.len(), 2);
        assert!(layout.total_width > 0.0);
        assert!(layout.total_height > 0.0);
    }

    #[test]
    fn test_compute_table_layout_with_colspan() {
        let config = TableConfig {
            border_collapse: BorderCollapse::Separate,
            border_spacing: (2.0, 2.0),
            width: None,
        };

        let rows_data = vec![
            vec![
                (1, 2, 1, 300.0, 50.0), // colspan=2
            ],
            vec![
                (2, 1, 1, 100.0, 50.0),
                (3, 1, 1, 200.0, 50.0),
            ],
        ];

        let layout = compute_table_layout(&config, &rows_data, 400.0);

        assert_eq!(layout.rows.len(), 2);
        assert_eq!(layout.rows[0].cells.len(), 1);
        assert_eq!(layout.rows[0].cells[0].colspan, 2);
    }

    #[test]
    fn test_empty_table() {
        let config = TableConfig {
            border_collapse: BorderCollapse::Separate,
            border_spacing: (2.0, 2.0),
            width: None,
        };

        let layout = compute_table_layout(&config, &[], 400.0);
        assert!(layout.rows.is_empty());
        assert!((layout.total_width - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_get_cell_absolute_position() {
        let config = TableConfig {
            border_collapse: BorderCollapse::Separate,
            border_spacing: (2.0, 2.0),
            width: None,
        };

        let rows_data = vec![
            vec![
                (1, 1, 1, 100.0, 50.0),
            ],
        ];

        let layout = compute_table_layout(&config, &rows_data, 200.0);

        let pos = get_cell_absolute_position(&layout, 0, 0);
        assert!(pos.is_some());
        
        if let Some((x, y, w, h)) = pos {
            assert!((x - 0.0).abs() < 0.01);
            assert!((y - 0.0).abs() < 0.01);
            assert!(w > 0.0);
            assert!((h - 50.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_column_width_distribution() {
        let config = TableConfig {
            border_collapse: BorderCollapse::Separate,
            border_spacing: (2.0, 2.0),
            width: None,
        };

        let rows_data = vec![
            vec![
                (1, 1, 1, 100.0, 50.0),
                (2, 1, 1, 200.0, 50.0),
                (3, 1, 1, 150.0, 50.0),
            ],
        ];

        let layout = compute_table_layout(&config, &rows_data, 500.0);

        assert_eq!(layout.column_widths.len(), 3);
        // 列宽应该按内容比例分配
        let total: f32 = layout.column_widths.iter().sum();
        assert!(total > 0.0);
    }

    #[test]
    fn test_border_spacing_affects_height() {
        let config_spaced = TableConfig {
            border_collapse: BorderCollapse::Separate,
            border_spacing: (2.0, 10.0),
            width: None,
        };

        let config_compact = TableConfig {
            border_collapse: BorderCollapse::Separate,
            border_spacing: (2.0, 2.0),
            width: None,
        };

        let rows_data = vec![
            vec![(1, 1, 1, 100.0, 50.0)],
            vec![(2, 1, 1, 100.0, 50.0)],
        ];

        let layout_spaced = compute_table_layout(&config_spaced, &rows_data, 200.0);
        let layout_compact = compute_table_layout(&config_compact, &rows_data, 200.0);

        // 更大的边框间距应该产生更大的总高度
        assert!(layout_spaced.total_height > layout_compact.total_height);
    }
}
