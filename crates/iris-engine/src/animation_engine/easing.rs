//! 缓动函数（Easing Functions）
//!
//! 提供 CSS 标准缓动函数和自定义缓动效果。

/// 缓动函数类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EasingFunction {
    /// 线性（匀速）
    Linear,
    /// 缓入（慢进快出）
    EaseIn,
    /// 缓出（快进慢出）
    EaseOut,
    /// 缓入缓出（慢进慢出）
    EaseInOut,
    /// 弹性缓动
    EaseElastic,
    /// 弹跳缓动
    EaseBounce,
    /// 自定义贝塞尔曲线 (x1, y1, x2, y2)
    CubicBezier(f32, f32, f32, f32),
}

/// 线性缓动
pub fn linear(t: f32) -> f32 {
    t
}

/// 缓入缓出（默认缓动）
pub fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}

/// 缓入（慢进）
pub fn ease_in(t: f32) -> f32 {
    t * t * t
}

/// 缓出（慢出）
pub fn ease_out(t: f32) -> f32 {
    let t = t - 1.0;
    t * t * t + 1.0
}

/// 弹性缓动
pub fn ease_elastic(t: f32) -> f32 {
    if t == 0.0 || t == 1.0 {
        return t;
    }

    let p = 0.3;
    let s = p / 4.0;

    if t < 1.0 {
        let t = t - 1.0;
        -(2.0_f32.powf(10.0 * t)) * ((t - s) * (2.0 * std::f32::consts::PI) / p).sin()
    } else {
        let t = t - 1.0;
        (2.0_f32.powf(-10.0 * t)) * ((t - s) * (2.0 * std::f32::consts::PI) / p).sin() + 1.0
    }
}

/// 弹跳缓动
pub fn ease_bounce(t: f32) -> f32 {
    if t < 1.0 / 2.75 {
        7.5625 * t * t
    } else if t < 2.0 / 2.75 {
        let t = t - 1.5 / 2.75;
        7.5625 * t * t + 0.75
    } else if t < 2.5 / 2.75 {
        let t = t - 2.25 / 2.75;
        7.5625 * t * t + 0.9375
    } else {
        let t = t - 2.625 / 2.75;
        7.5625 * t * t + 0.984375
    }
}

impl EasingFunction {
    /// 计算缓动值
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            EasingFunction::Linear => linear(t),
            EasingFunction::EaseIn => ease_in(t),
            EasingFunction::EaseOut => ease_out(t),
            EasingFunction::EaseInOut => ease_in_out(t),
            EasingFunction::EaseElastic => ease_elastic(t),
            EasingFunction::EaseBounce => ease_bounce(t),
            EasingFunction::CubicBezier(x1, y1, x2, y2) => {
                cubic_bezier(t, *x1, *y1, *x2, *y2)
            }
        }
    }
}

/// 三次贝塞尔曲线近似计算
fn cubic_bezier(t: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let mut t = t;
    for _ in 0..8 {
        let x = calculate_bezier_x(t, x1, x2);
        if (x - t).abs() < 0.001 {
            break;
        }
        t += (t - x) * 0.5;
    }
    
    calculate_bezier_y(t, y1, y2)
}

fn calculate_bezier_x(t: f32, x1: f32, x2: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    3.0 * (1.0 - t) * (1.0 - t) * t * x1 + 3.0 * (1.0 - t) * t2 * x2 + t3
}

fn calculate_bezier_y(t: f32, y1: f32, y2: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    3.0 * (1.0 - t) * (1.0 - t) * t * y1 + 3.0 * (1.0 - t) * t2 * y2 + t3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear() {
        assert!((linear(0.0_f32) - 0.0_f32).abs() < f32::EPSILON);
        assert!((linear(0.5_f32) - 0.5_f32).abs() < f32::EPSILON);
        assert!((linear(1.0_f32) - 1.0_f32).abs() < f32::EPSILON);
    }

    #[test]
    fn test_ease_in_out() {
        assert!((ease_in_out(0.0_f32) - 0.0_f32).abs() < f32::EPSILON);
        assert!((ease_in_out(1.0_f32) - 1.0_f32).abs() < f32::EPSILON);
        assert!((ease_in_out(0.5_f32) - 0.5_f32).abs() < f32::EPSILON);
    }

    #[test]
    fn test_ease_in() {
        assert!((ease_in(0.0_f32) - 0.0_f32).abs() < f32::EPSILON);
        assert!((ease_in(1.0_f32) - 1.0_f32).abs() < f32::EPSILON);
        assert!(ease_in(0.5_f32) < 0.5_f32);
    }

    #[test]
    fn test_ease_out() {
        assert!((ease_out(0.0_f32) - 0.0_f32).abs() < f32::EPSILON);
        assert!((ease_out(1.0_f32) - 1.0_f32).abs() < f32::EPSILON);
        assert!(ease_out(0.5_f32) > 0.5_f32);
    }

    #[test]
    fn test_easing_function_apply() {
        let easing = EasingFunction::EaseInOut;
        assert!((easing.apply(0.5_f32) - 0.5_f32).abs() < f32::EPSILON);
    }
}
