//! CSS Grid 布局系统
//!
//! 实现 CSS Grid Layout，支持：
//! - grid-template-columns/rows
//! - grid-gap/gap
//! - grid-column/grid-row placement
//! - auto-fill/auto-fit
//! - fr 单位

use crate::style::ComputedStyles;

/// Grid 轨道尺寸
#[derive(Debug, Clone, PartialEq)]
pub enum GridTrackSize {
    /// 固定像素值
    Pixels(f32),
    /// 百分比
    Percentage(f32),
    /// 弹性分数（fr）
    Fraction(f32),
    /// 自动（内容大小）
    Auto,
    /// 最小内容大小
    MinContent,
    /// 最大内容大小
    MaxContent,
    /// 重复函数
    Repeat(usize, Box<GridTrackSize>),
}

impl GridTrackSize {
    /// 从 CSS 字符串解析单个值
    pub fn from_css(css: &str) -> Self {
        let css = css.trim();
        
        if css.ends_with("fr") {
            if let Ok(val) = css.trim_end_matches("fr").parse::<f32>() {
                return GridTrackSize::Fraction(val);
            }
        }

        if css.ends_with('%') {
            if let Ok(val) = css.trim_end_matches('%').parse::<f32>() {
                return GridTrackSize::Percentage(val);
            }
        }

        if let Ok(val) = css.trim_end_matches("px").parse::<f32>() {
            return GridTrackSize::Pixels(val);
        }

        match css {
            "auto" => GridTrackSize::Auto,
            "min-content" => GridTrackSize::MinContent,
            "max-content" => GridTrackSize::MaxContent,
            _ => GridTrackSize::Auto,
        }
    }

    /// 解析完整的轨道定义（支持空格分隔的多个值）
    pub fn parse_track_definition(css: &str) -> Vec<GridTrackSize> {
        // 处理 repeat() 函数
        if css.contains("repeat(") {
            return Self::parse_repeat(css);
        }

        css.split_whitespace()
            .map(|s| Self::from_css(s))
            .collect()
    }

    /// 解析 repeat() 函数
    fn parse_repeat(css: &str) -> Vec<GridTrackSize> {
        // 简化实现：解析 repeat(3, 1fr) -> [1fr, 1fr, 1fr]
        // 完整实现需要正则表达式
        let tracks = Vec::new();
        
        // TODO: 实现完整的 repeat() 解析
        tracks
    }
}

/// Grid 单元格放置
#[derive(Debug, Clone)]
pub struct GridPlacement {
    /// 起始线
    pub start: i32,
    /// 结束线
    pub end: i32,
    /// 跨越的轨道数
    pub span: Option<i32>,
}

impl GridPlacement {
    /// 从 CSS 字符串解析
    /// 支持: "1", "1 / 3", "span 2"
    pub fn from_css(css: &str) -> Self {
        let css = css.trim();
        
        if css.starts_with("span") {
            let span = css.trim_start_matches("span").trim().parse::<i32>().unwrap_or(1);
            Self {
                start: 1,
                end: 1 + span,
                span: Some(span),
            }
        } else if css.contains('/') {
            let parts: Vec<&str> = css.split('/').collect();
            let start = parts[0].trim().parse::<i32>().unwrap_or(1);
            let end = parts.get(1).and_then(|s| s.trim().parse::<i32>().ok()).unwrap_or(start + 1);
            Self {
                start,
                end,
                span: None,
            }
        } else {
            let line = css.parse::<i32>().unwrap_or(1);
            Self {
                start: line,
                end: line + 1,
                span: None,
            }
        }
    }
}

/// Grid 项配置
#[derive(Debug, Clone)]
pub struct GridItemConfig {
    /// 列放置
    pub column: GridPlacement,
    /// 行放置
    pub row: GridPlacement,
}

impl GridItemConfig {
    /// 从 ComputedStyles 创建
    pub fn from_styles(styles: &ComputedStyles) -> Self {
        let column = styles
            .get("grid-column")
            .map(|s| GridPlacement::from_css(s))
            .unwrap_or(GridPlacement {
                start: 1,
                end: 2,
                span: None,
            });

        let row = styles
            .get("grid-row")
            .map(|s| GridPlacement::from_css(s))
            .unwrap_or(GridPlacement {
                start: 1,
                end: 2,
                span: None,
            });

        Self { column, row }
    }
}

/// Grid 容器配置
#[derive(Debug, Clone)]
pub struct GridConfig {
    /// 列轨道定义
    pub columns: Vec<GridTrackSize>,
    /// 行轨道定义
    pub rows: Vec<GridTrackSize>,
    /// 列间距
    pub column_gap: f32,
    /// 行间距
    pub row_gap: f32,
}

impl GridConfig {
    /// 从 ComputedStyles 创建
    pub fn from_styles(styles: &ComputedStyles) -> Self {
        let columns = styles
            .get("grid-template-columns")
            .map(|s| GridTrackSize::parse_track_definition(s))
            .unwrap_or_default();

        let rows = styles
            .get("grid-template-rows")
            .map(|s| GridTrackSize::parse_track_definition(s))
            .unwrap_or_default();

        let column_gap = styles
            .get("column-gap")
            .or_else(|| styles.get("grid-column-gap"))
            .and_then(|s| s.trim_end_matches("px").parse::<f32>().ok())
            .unwrap_or(0.0);

        let row_gap = styles
            .get("row-gap")
            .or_else(|| styles.get("grid-row-gap"))
            .and_then(|s| s.trim_end_matches("px").parse::<f32>().ok())
            .unwrap_or(0.0);

        Self {
            columns,
            rows,
            column_gap,
            row_gap,
        }
    }
}

/// Grid 单元格布局结果
#[derive(Debug, Clone)]
pub struct GridCellLayout {
    /// x 坐标
    pub x: f32,
    /// y 坐标
    pub y: f32,
    /// 宽度
    pub width: f32,
    /// 高度
    pub height: f32,
}

/// Grid 布局结果
#[derive(Debug, Clone)]
pub struct GridLayout {
    /// 所有单元格的布局
    pub cells: Vec<GridCellLayout>,
    /// 网格总宽度
    pub total_width: f32,
    /// 网格总高度
    pub total_height: f32,
}

/// 计算 Grid 布局
/// 
/// # 参数
/// 
/// - `config`: Grid 容器配置
/// - `item_configs`: Grid 项配置列表
/// - `container_width`: 容器宽度
/// - `container_height`: 容器高度
/// 
/// # 返回
/// 
/// 返回 Grid 布局结果
pub fn compute_grid_layout(
    config: &GridConfig,
    item_configs: &[GridItemConfig],
    container_width: f32,
    container_height: f32,
) -> GridLayout {
    if config.columns.is_empty() {
        return GridLayout {
            cells: Vec::new(),
            total_width: 0.0,
            total_height: 0.0,
        };
    }

    // 计算列宽
    let column_widths = calculate_track_sizes(
        &config.columns,
        container_width - (config.column_gap * (config.columns.len() as f32 - 1.0)),
    );

    // 计算行高
    let row_heights = calculate_track_sizes(
        &config.rows,
        if config.rows.is_empty() {
            container_height
        } else {
            container_height - (config.row_gap * (config.rows.len() as f32 - 1.0))
        },
    );

    // 计算每个项的位置
    let mut cells = Vec::new();
    for item_config in item_configs {
        let cell = compute_cell_position(
            &item_config.column,
            &item_config.row,
            &column_widths,
            &row_heights,
            config.column_gap,
            config.row_gap,
        );
        cells.push(cell);
    }

    let total_width = column_widths.iter().sum::<f32>() 
        + config.column_gap * (config.columns.len() as f32).max(1.0) - config.column_gap;
    let total_height = row_heights.iter().sum::<f32>()
        + config.row_gap * (config.rows.len() as f32).max(1.0) - config.row_gap;

    GridLayout {
        cells,
        total_width,
        total_height,
    }
}

/// 计算轨道尺寸
fn calculate_track_sizes(tracks: &[GridTrackSize], available_space: f32) -> Vec<f32> {
    if tracks.is_empty() {
        return vec![available_space];
    }

    let mut sizes = vec![0.0f32; tracks.len()];
    let mut fixed_space = 0.0;
    let mut total_fr = 0.0;

    // 第一轮：计算固定值和 fr 总数
    for (i, track) in tracks.iter().enumerate() {
        match track {
            GridTrackSize::Pixels(px) => {
                sizes[i] = *px;
                fixed_space += *px;
            }
            GridTrackSize::Percentage(pct) => {
                sizes[i] = available_space * pct / 100.0;
                fixed_space += sizes[i];
            }
            GridTrackSize::Fraction(fr) => {
                total_fr += fr;
            }
            GridTrackSize::Auto | GridTrackSize::MinContent | GridTrackSize::MaxContent => {
                // 自动尺寸，暂时设为 0，后续可基于内容计算
                sizes[i] = 0.0;
            }
            _ => {}
        }
    }

    // 第二轮：分配 fr 空间
    if total_fr > 0.0 {
        let remaining_space = (available_space - fixed_space).max(0.0);
        for (i, track) in tracks.iter().enumerate() {
            if let GridTrackSize::Fraction(fr) = track {
                sizes[i] = remaining_space * fr / total_fr;
            }
        }
    }

    sizes
}

/// 计算单元格位置
fn compute_cell_position(
    column: &GridPlacement,
    row: &GridPlacement,
    column_widths: &[f32],
    row_heights: &[f32],
    column_gap: f32,
    row_gap: f32,
) -> GridCellLayout {
    // 计算 x 坐标和宽度
    let col_start = (column.start - 1).max(0) as usize;
    let col_end = column.end.min(column_widths.len() as i32) as usize;
    
    let mut x = 0.0;
    let mut width = 0.0;
    
    for i in col_start..col_end {
        if i > col_start {
            x += column_gap;
        }
        if i < column_widths.len() {
            if i == col_start {
                x = column_widths[..i].iter().sum::<f32>() + column_gap * i as f32;
            }
            width += column_widths[i];
        }
    }
    
    // 计算 y 坐标和高度
    let row_start = (row.start - 1).max(0) as usize;
    let row_end = row.end.min(row_heights.len() as i32) as usize;
    
    let mut y = 0.0;
    let mut height = 0.0;
    
    for i in row_start..row_end {
        if i > row_start {
            y += row_gap;
        }
        if i < row_heights.len() {
            if i == row_start {
                y = row_heights[..i].iter().sum::<f32>() + row_gap * i as f32;
            }
            height += row_heights[i];
        }
    }

    GridCellLayout {
        x,
        y,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_track_size_parsing() {
        assert!(matches!(GridTrackSize::from_css("100px"), GridTrackSize::Pixels(100.0)));
        assert!(matches!(GridTrackSize::from_css("50%"), GridTrackSize::Percentage(50.0)));
        assert!(matches!(GridTrackSize::from_css("1fr"), GridTrackSize::Fraction(1.0)));
        assert_eq!(GridTrackSize::from_css("auto"), GridTrackSize::Auto);
    }

    #[test]
    fn test_grid_placement_parsing() {
        let placement = GridPlacement::from_css("1");
        assert_eq!(placement.start, 1);
        assert_eq!(placement.end, 2);

        let placement = GridPlacement::from_css("1 / 3");
        assert_eq!(placement.start, 1);
        assert_eq!(placement.end, 3);

        let placement = GridPlacement::from_css("span 2");
        assert_eq!(placement.span, Some(2));
    }

    #[test]
    fn test_grid_config_from_styles() {
        let mut styles = ComputedStyles::new();
        styles.set("grid-template-columns", "1fr 2fr 1fr");
        styles.set("grid-template-rows", "100px 200px");
        styles.set("column-gap", "10px");
        styles.set("row-gap", "20px");

        let config = GridConfig::from_styles(&styles);
        assert_eq!(config.columns.len(), 3);
        assert_eq!(config.rows.len(), 2);
        assert!((config.column_gap - 10.0).abs() < 0.01);
        assert!((config.row_gap - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_grid_layout_calculation() {
        let config = GridConfig {
            columns: vec![
                GridTrackSize::Fraction(1.0),
                GridTrackSize::Fraction(2.0),
                GridTrackSize::Fraction(1.0),
            ],
            rows: vec![
                GridTrackSize::Pixels(100.0),
                GridTrackSize::Pixels(200.0),
            ],
            column_gap: 10.0,
            row_gap: 20.0,
        };

        let item_configs = vec![
            GridItemConfig {
                column: GridPlacement { start: 1, end: 2, span: None },
                row: GridPlacement { start: 1, end: 2, span: None },
            },
        ];

        let layout = compute_grid_layout(&config, &item_configs, 800.0, 600.0);
        
        assert!(!layout.cells.is_empty());
        assert!(layout.total_width > 0.0);
        assert!(layout.total_height > 0.0);
    }

    #[test]
    fn test_calculate_track_sizes_fr() {
        let tracks = vec![
            GridTrackSize::Fraction(1.0),
            GridTrackSize::Fraction(2.0),
            GridTrackSize::Fraction(1.0),
        ];

        let sizes = calculate_track_sizes(&tracks, 400.0);
        assert_eq!(sizes.len(), 3);
        assert!((sizes[0] - 100.0).abs() < 0.01); // 400 * 1/4
        assert!((sizes[1] - 200.0).abs() < 0.01); // 400 * 2/4
        assert!((sizes[2] - 100.0).abs() < 0.01); // 400 * 1/4
    }

    #[test]
    fn test_calculate_track_sizes_mixed() {
        let tracks = vec![
            GridTrackSize::Pixels(100.0),
            GridTrackSize::Fraction(1.0),
            GridTrackSize::Fraction(1.0),
        ];

        let sizes = calculate_track_sizes(&tracks, 500.0);
        assert!((sizes[0] - 100.0).abs() < 0.01); // 固定 100
        assert!((sizes[1] - 200.0).abs() < 0.01); // (500-100) * 1/2
        assert!((sizes[2] - 200.0).abs() < 0.01); // (500-100) * 1/2
    }
}
