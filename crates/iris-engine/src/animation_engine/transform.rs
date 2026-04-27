//! Transform 动画和 3D 变换支持
//!
//! 实现 CSS transform 属性的动画，包括：
//! - 2D 变换：translate, rotate, scale, skew
//! - 3D 变换：translate3d, rotate3d, scale3d, perspective
//! - transform-origin 支持
//! - will-change 性能优化提示

use std::collections::HashMap;

/// Transform 函数类型
#[derive(Debug, Clone, PartialEq)]
pub enum TransformFunction {
    // 2D 变换
    /// 平移 (tx, ty)
    Translate(f32, f32),
    /// 平移 X
    TranslateX(f32),
    /// 平移 Y
    TranslateY(f32),
    /// 旋转（角度，单位：度）
    Rotate(f32),
    /// 缩放 (sx, sy)
    Scale(f32, f32),
    /// 缩放 X
    ScaleX(f32),
    /// 缩放 Y
    ScaleY(f32),
    /// 倾斜 (ax, ay)（单位：度）
    Skew(f32, f32),
    /// 倾斜 X
    SkewX(f32),
    /// 倾斜 Y
    SkewY(f32),
    /// 矩阵变换 (a, b, c, d, tx, ty)
    Matrix(f32, f32, f32, f32, f32, f32),
    
    // 3D 变换
    /// 3D 平移 (tx, ty, tz)
    Translate3d(f32, f32, f32),
    /// 3D 旋转 (x, y, z, angle)
    Rotate3d(f32, f32, f32, f32),
    /// 绕 X 轴旋转
    RotateX(f32),
    /// 绕 Y 轴旋转
    RotateY(f32),
    /// 绕 Z 轴旋转
    RotateZ(f32),
    /// 3D 缩放 (sx, sy, sz)
    Scale3d(f32, f32, f32),
    /// 3D 缩放 X
    ScaleX3d(f32),
    /// 3D 缩放 Y
    ScaleY3d(f32),
    /// 3D 缩放 Z
    ScaleZ3d(f32),
    /// 透视（深度）
    Perspective(f32),
    /// 3D 矩阵 (16 个值)
    Matrix3d([f32; 16]),
}

/// Transform 变换链
#[derive(Debug, Clone)]
pub struct TransformChain {
    /// Transform 函数列表（按顺序应用）
    pub functions: Vec<TransformFunction>,
}

impl TransformChain {
    /// 创建空的变换链
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
    }

    /// 从 CSS transform 字符串解析
    /// 例如: "translate(10px, 20px) rotate(45deg) scale(1.5)"
    pub fn from_css(css: &str) -> Self {
        let mut functions = Vec::new();
        let css = css.trim();
        
        if css.is_empty() || css == "none" {
            return Self { functions };
        }

        // 简化的解析器（生产环境需要更完整的 CSS 解析）
        let mut i = 0;
        let chars: Vec<char> = css.chars().collect();
        
        while i < chars.len() {
            // 跳过空格
            while i < chars.len() && chars[i] == ' ' {
                i += 1;
            }
            
            if i >= chars.len() {
                break;
            }
            
            // 查找函数名
            let start = i;
            while i < chars.len() && chars[i] != '(' {
                i += 1;
            }
            
            if i >= chars.len() {
                break;
            }
            
            let func_name: String = chars[start..i].iter().collect();
            i += 1; // 跳过 '('
            
            // 查找参数
            let param_start = i;
            let mut paren_depth = 1;
            while i < chars.len() && paren_depth > 0 {
                if chars[i] == '(' {
                    paren_depth += 1;
                } else if chars[i] == ')' {
                    paren_depth -= 1;
                }
                i += 1;
            }
            
            let params: String = chars[param_start..i-1].iter().collect();
            
            // 解析具体的 transform 函数
            if let Some(func) = Self::parse_transform_function(&func_name, &params) {
                functions.push(func);
            }
            
            // 跳过 ')'
            if i < chars.len() && chars[i] == ')' {
                i += 1;
            }
        }
        
        Self { functions }
    }

    /// 解析单个 transform 函数
    fn parse_transform_function(name: &str, params: &str) -> Option<TransformFunction> {
        let values: Vec<f32> = params
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .filter_map(|s| {
                s.trim_end_matches("px")
                    .trim_end_matches("deg")
                    .trim_end_matches("%")
                    .parse::<f32>()
                    .ok()
            })
            .collect();

        match name {
            "translate" => {
                if values.len() >= 2 {
                    Some(TransformFunction::Translate(values[0], values[1]))
                } else if values.len() == 1 {
                    Some(TransformFunction::Translate(values[0], 0.0))
                } else {
                    None
                }
            }
            "translateX" => {
                if values.len() == 1 {
                    Some(TransformFunction::TranslateX(values[0]))
                } else {
                    None
                }
            }
            "translateY" => {
                if values.len() == 1 {
                    Some(TransformFunction::TranslateY(values[0]))
                } else {
                    None
                }
            }
            "rotate" => {
                if values.len() == 1 {
                    Some(TransformFunction::Rotate(values[0]))
                } else {
                    None
                }
            }
            "scale" => {
                if values.len() >= 2 {
                    Some(TransformFunction::Scale(values[0], values[1]))
                } else if values.len() == 1 {
                    Some(TransformFunction::Scale(values[0], values[0]))
                } else {
                    None
                }
            }
            "scaleX" => {
                if values.len() == 1 {
                    Some(TransformFunction::ScaleX(values[0]))
                } else {
                    None
                }
            }
            "scaleY" => {
                if values.len() == 1 {
                    Some(TransformFunction::ScaleY(values[0]))
                } else {
                    None
                }
            }
            "skew" => {
                if values.len() >= 2 {
                    Some(TransformFunction::Skew(values[0], values[1]))
                } else if values.len() == 1 {
                    Some(TransformFunction::Skew(values[0], 0.0))
                } else {
                    None
                }
            }
            "skewX" => {
                if values.len() == 1 {
                    Some(TransformFunction::SkewX(values[0]))
                } else {
                    None
                }
            }
            "skewY" => {
                if values.len() == 1 {
                    Some(TransformFunction::SkewY(values[0]))
                } else {
                    None
                }
            }
            "matrix" => {
                if values.len() == 6 {
                    Some(TransformFunction::Matrix(
                        values[0], values[1], values[2], values[3], values[4], values[5],
                    ))
                } else {
                    None
                }
            }
            "translate3d" => {
                if values.len() >= 3 {
                    Some(TransformFunction::Translate3d(values[0], values[1], values[2]))
                } else {
                    None
                }
            }
            "rotate3d" => {
                if values.len() >= 4 {
                    Some(TransformFunction::Rotate3d(values[0], values[1], values[2], values[3]))
                } else {
                    None
                }
            }
            "rotateX" => {
                if values.len() == 1 {
                    Some(TransformFunction::RotateX(values[0]))
                } else {
                    None
                }
            }
            "rotateY" => {
                if values.len() == 1 {
                    Some(TransformFunction::RotateY(values[0]))
                } else {
                    None
                }
            }
            "rotateZ" => {
                if values.len() == 1 {
                    Some(TransformFunction::RotateZ(values[0]))
                } else {
                    None
                }
            }
            "scale3d" => {
                if values.len() >= 3 {
                    Some(TransformFunction::Scale3d(values[0], values[1], values[2]))
                } else {
                    None
                }
            }
            "scaleX3d" | "scaleZ" => {
                if values.len() == 1 {
                    Some(TransformFunction::ScaleX3d(values[0]))
                } else {
                    None
                }
            }
            "scaleY3d" => {
                if values.len() == 1 {
                    Some(TransformFunction::ScaleY3d(values[0]))
                } else {
                    None
                }
            }
            "scaleZ3d" => {
                if values.len() == 1 {
                    Some(TransformFunction::ScaleZ3d(values[0]))
                } else {
                    None
                }
            }
            "perspective" => {
                if values.len() == 1 {
                    Some(TransformFunction::Perspective(values[0]))
                } else {
                    None
                }
            }
            "matrix3d" => {
                if values.len() == 16 {
                    let mut matrix = [0.0f32; 16];
                    matrix.copy_from_slice(&values);
                    Some(TransformFunction::Matrix3d(matrix))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 在两个变换链之间插值
    pub fn interpolate(&self, other: &TransformChain, t: f32) -> TransformChain {
        let mut result = TransformChain::new();
        
        let max_len = self.functions.len().max(other.functions.len());
        
        for i in 0..max_len {
            let self_func = self.functions.get(i);
            let other_func = other.functions.get(i);
            
            if let (Some(sf), Some(of)) = (self_func, other_func) {
                if let Some(interpolated) = Self::interpolate_functions(sf, of, t) {
                    result.functions.push(interpolated);
                }
            } else if let Some(f) = self_func {
                // 只在起始变换链中存在
                result.functions.push(f.clone());
            } else if let Some(f) = other_func {
                // 只在结束变换链中存在
                result.functions.push(f.clone());
            }
        }
        
        result
    }

    /// 插值两个 transform 函数
    fn interpolate_functions(a: &TransformFunction, b: &TransformFunction, t: f32) -> Option<TransformFunction> {
        // 只有相同类型的函数才能插值
        match (a, b) {
            (TransformFunction::Translate(ax, ay), TransformFunction::Translate(bx, by)) => {
                Some(TransformFunction::Translate(
                    Self::lerp(*ax, *bx, t),
                    Self::lerp(*ay, *by, t),
                ))
            }
            (TransformFunction::Rotate(a_angle), TransformFunction::Rotate(b_angle)) => {
                Some(TransformFunction::Rotate(Self::lerp(*a_angle, *b_angle, t)))
            }
            (TransformFunction::Scale(ax, ay), TransformFunction::Scale(bx, by)) => {
                Some(TransformFunction::Scale(
                    Self::lerp(*ax, *bx, t),
                    Self::lerp(*ay, *by, t),
                ))
            }
            (TransformFunction::Translate3d(ax, ay, az), TransformFunction::Translate3d(bx, by, bz)) => {
                Some(TransformFunction::Translate3d(
                    Self::lerp(*ax, *bx, t),
                    Self::lerp(*ay, *by, t),
                    Self::lerp(*az, *bz, t),
                ))
            }
            (TransformFunction::Rotate3d(ax, ay, az, a_angle), TransformFunction::Rotate3d(bx, by, bz, b_angle)) => {
                Some(TransformFunction::Rotate3d(
                    Self::lerp(*ax, *bx, t),
                    Self::lerp(*ay, *by, t),
                    Self::lerp(*az, *bz, t),
                    Self::lerp(*a_angle, *b_angle, t),
                ))
            }
            (TransformFunction::Scale3d(ax, ay, az), TransformFunction::Scale3d(bx, by, bz)) => {
                Some(TransformFunction::Scale3d(
                    Self::lerp(*ax, *bx, t),
                    Self::lerp(*ay, *by, t),
                    Self::lerp(*az, *bz, t),
                ))
            }
            _ => None, // 类型不匹配，无法插值
        }
    }

    /// 线性插值
    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    /// 转换为 CSS transform 字符串
    pub fn to_css(&self) -> String {
        if self.functions.is_empty() {
            return "none".to_string();
        }

        self.functions
            .iter()
            .map(|f| match f {
                TransformFunction::Translate(x, y) => format!("translate({}px, {}px)", x, y),
                TransformFunction::TranslateX(x) => format!("translateX({}px)", x),
                TransformFunction::TranslateY(y) => format!("translateY({}px)", y),
                TransformFunction::Rotate(angle) => format!("rotate({}deg)", angle),
                TransformFunction::Scale(x, y) => format!("scale({}, {})", x, y),
                TransformFunction::ScaleX(x) => format!("scaleX({})", x),
                TransformFunction::ScaleY(y) => format!("scaleY({})", y),
                TransformFunction::Skew(ax, ay) => format!("skew({}deg, {}deg)", ax, ay),
                TransformFunction::SkewX(ax) => format!("skewX({}deg)", ax),
                TransformFunction::SkewY(ay) => format!("skewY({}deg)", ay),
                TransformFunction::Matrix(a, b, c, d, tx, ty) => {
                    format!("matrix({}, {}, {}, {}, {}, {})", a, b, c, d, tx, ty)
                }
                TransformFunction::Translate3d(x, y, z) => format!("translate3d({}px, {}px, {}px)", x, y, z),
                TransformFunction::Rotate3d(x, y, z, angle) => {
                    format!("rotate3d({}, {}, {}, {}deg)", x, y, z, angle)
                }
                TransformFunction::RotateX(angle) => format!("rotateX({}deg)", angle),
                TransformFunction::RotateY(angle) => format!("rotateY({}deg)", angle),
                TransformFunction::RotateZ(angle) => format!("rotateZ({}deg)", angle),
                TransformFunction::Scale3d(x, y, z) => format!("scale3d({}, {}, {})", x, y, z),
                TransformFunction::ScaleX3d(x) => format!("scaleX3d({})", x),
                TransformFunction::ScaleY3d(y) => format!("scaleY3d({})", y),
                TransformFunction::ScaleZ3d(z) => format!("scaleZ3d({})", z),
                TransformFunction::Perspective(p) => format!("perspective({}px)", p),
                TransformFunction::Matrix3d(m) => {
                    format!("matrix3d({})", m.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Transform 动画状态
#[derive(Debug, Clone)]
pub struct TransformAnimation {
    /// 元素 ID
    pub element_id: u64,
    /// 起始变换
    pub from: TransformChain,
    /// 结束变换
    pub to: TransformChain,
    /// 当前进度 (0.0 - 1.0)
    pub progress: f32,
    /// 当前计算的变换
    pub current: TransformChain,
}

impl TransformAnimation {
    /// 创建新的 transform 动画
    pub fn new(element_id: u64, from: TransformChain, to: TransformChain) -> Self {
        Self {
            element_id,
            from: from.clone(),
            to: to.clone(),
            progress: 0.0,
            current: from,
        }
    }

    /// 更新动画进度
    pub fn update(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
        self.current = self.from.interpolate(&self.to, self.progress);
    }

    /// 检查动画是否完成
    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }
}

/// 性能优化提示（will-change 属性）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WillChange {
    /// 无优化提示
    Auto,
    /// 优化滚动
    ScrollPosition,
    /// 优化内容变化
    Contents,
    /// 优化特定属性
    Properties(Vec<String>),
}

impl WillChange {
    /// 从 CSS 字符串解析
    pub fn from_css(css: &str) -> Self {
        match css.trim() {
            "auto" => WillChange::Auto,
            "scroll-position" => WillChange::ScrollPosition,
            "contents" => WillChange::Contents,
            other => {
                let props: Vec<String> = other
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                WillChange::Properties(props)
            }
        }
    }

    /// 转换为 CSS 字符串
    pub fn to_css(&self) -> String {
        match self {
            WillChange::Auto => "auto".to_string(),
            WillChange::ScrollPosition => "scroll-position".to_string(),
            WillChange::Contents => "contents".to_string(),
            WillChange::Properties(props) => props.join(", "),
        }
    }
}

/// Transform-origin 配置
#[derive(Debug, Clone)]
pub struct TransformOrigin {
    /// X 坐标（可以是像素值或百分比）
    pub x: f32,
    /// Y 坐标
    pub y: f32,
    /// Z 坐标（3D 变换）
    pub z: f32,
}

impl TransformOrigin {
    /// 创建新的 transform-origin
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// 从 CSS 字符串解析
    /// 例如: "center center", "50% 50%", "10px 20px"
    pub fn from_css(css: &str) -> Self {
        let parts: Vec<&str> = css.split_whitespace().collect();
        
        let x = if parts.len() > 0 {
            Self::parse_coordinate(parts[0])
        } else {
            50.0 // 默认 center
        };
        
        let y = if parts.len() > 1 {
            Self::parse_coordinate(parts[1])
        } else {
            50.0
        };
        
        let z = if parts.len() > 2 {
            parts[2].trim_end_matches("px").parse::<f32>().unwrap_or(0.0)
        } else {
            0.0
        };
        
        Self { x, y, z }
    }

    /// 解析坐标值（支持像素和百分比）
    fn parse_coordinate(s: &str) -> f32 {
        if s == "left" || s == "top" {
            0.0
        } else if s == "center" {
            50.0
        } else if s == "right" || s == "bottom" {
            100.0
        } else if s.ends_with('%') {
            s.trim_end_matches('%').parse::<f32>().unwrap_or(50.0)
        } else {
            // 像素值，保持原样
            s.trim_end_matches("px").parse::<f32>().unwrap_or(0.0)
        }
    }

    /// 转换为 CSS 字符串
    pub fn to_css(&self) -> String {
        format!("{}% {}% {}px", self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_translate() {
        let chain = TransformChain::from_css("translate(10px, 20px)");
        assert_eq!(chain.functions.len(), 1);
        assert!(matches!(&chain.functions[0], TransformFunction::Translate(10.0, 20.0)));
    }

    #[test]
    fn test_parse_rotate() {
        let chain = TransformChain::from_css("rotate(45deg)");
        assert_eq!(chain.functions.len(), 1);
        assert!(matches!(&chain.functions[0], TransformFunction::Rotate(45.0)));
    }

    #[test]
    fn test_parse_scale() {
        let chain = TransformChain::from_css("scale(1.5, 2.0)");
        assert_eq!(chain.functions.len(), 1);
        assert!(matches!(&chain.functions[0], TransformFunction::Scale(1.5, 2.0)));
    }

    #[test]
    fn test_parse_chain() {
        let chain = TransformChain::from_css("translate(10px, 20px) rotate(45deg) scale(1.5)");
        assert_eq!(chain.functions.len(), 3);
    }

    #[test]
    fn test_parse_3d_transform() {
        let chain = TransformChain::from_css("translate3d(10px, 20px, 30px) rotate3d(1, 0, 0, 45deg)");
        assert_eq!(chain.functions.len(), 2);
        assert!(matches!(&chain.functions[0], TransformFunction::Translate3d(10.0, 20.0, 30.0)));
    }

    #[test]
    fn test_interpolate_translate() {
        let from = TransformChain::from_css("translate(0px, 0px)");
        let to = TransformChain::from_css("translate(100px, 50px)");
        
        let result = from.interpolate(&to, 0.5);
        assert_eq!(result.functions.len(), 1);
        
        if let TransformFunction::Translate(x, y) = &result.functions[0] {
            assert!((*x - 50.0).abs() < 0.01);
            assert!((*y - 25.0).abs() < 0.01);
        } else {
            panic!("Expected Translate");
        }
    }

    #[test]
    fn test_interpolate_rotate() {
        let from = TransformChain::from_css("rotate(0deg)");
        let to = TransformChain::from_css("rotate(90deg)");
        
        let result = from.interpolate(&to, 0.5);
        assert_eq!(result.functions.len(), 1);
        
        if let TransformFunction::Rotate(angle) = &result.functions[0] {
            assert!((*angle - 45.0).abs() < 0.01);
        } else {
            panic!("Expected Rotate");
        }
    }

    #[test]
    fn test_transform_animation() {
        let from = TransformChain::from_css("scale(1.0)");
        let to = TransformChain::from_css("scale(2.0)");
        
        let mut anim = TransformAnimation::new(1, from, to);
        assert!(!anim.is_complete());
        
        anim.update(0.5);
        assert!(!anim.is_complete());
        
        anim.update(1.0);
        assert!(anim.is_complete());
    }

    #[test]
    fn test_will_change() {
        let wc = WillChange::from_css("transform, opacity");
        assert!(matches!(wc, WillChange::Properties(_)));
        
        if let WillChange::Properties(props) = wc {
            assert_eq!(props.len(), 2);
            assert_eq!(props[0], "transform");
            assert_eq!(props[1], "opacity");
        }
    }

    #[test]
    fn test_transform_origin() {
        let origin = TransformOrigin::from_css("center center");
        assert!((origin.x - 50.0).abs() < 0.01);
        assert!((origin.y - 50.0).abs() < 0.01);
        
        let origin2 = TransformOrigin::from_css("10px 20px 30px");
        assert!((origin2.x - 10.0).abs() < 0.01);
        assert!((origin2.y - 20.0).abs() < 0.01);
        assert!((origin2.z - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_to_css() {
        let chain = TransformChain::from_css("translate(10px, 20px) rotate(45deg)");
        let css = chain.to_css();
        assert!(css.contains("translate"));
        assert!(css.contains("rotate"));
    }
}
