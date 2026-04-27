//! 动画应用器
//!
//! 将动画引擎的计算结果应用到 VNode 渲染属性。

use crate::animation_engine::easing::EasingFunction;

/// 动画属性值
#[derive(Debug, Clone)]
pub struct AnimatedValue {
    /// 当前计算值
    pub value: f32,
    /// 是否正在动画中
    pub is_animating: bool,
}

/// CSS Transition 配置
#[derive(Debug, Clone)]
pub struct TransitionConfig {
    /// 属性名 (opacity, transform, width, height 等)
    pub property: String,
    /// 持续时间（毫秒）
    pub duration: f64,
    /// 缓动函数
    pub easing: EasingFunction,
    /// 延迟（毫秒）
    pub delay: f64,
}

impl TransitionConfig {
    /// 从 CSS 字符串解析
    /// 格式: "property duration timing-function delay"
    /// 例如: "opacity 0.3s ease-in-out"
    pub fn from_css(css: &str) -> Option<Self> {
        let parts: Vec<&str> = css.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }

        let property = parts[0].to_string();
        let duration = Self::parse_duration(parts[1])?;
        let easing = if parts.len() > 2 {
            Self::parse_easing(parts[2])
        } else {
            EasingFunction::EaseInOut
        };
        let delay = if parts.len() > 3 {
            Self::parse_duration(parts[3]).unwrap_or(0.0)
        } else {
            0.0
        };

        Some(Self {
            property,
            duration,
            easing,
            delay,
        })
    }

    /// 解析持续时间（支持 s 和 ms）
    fn parse_duration(s: &str) -> Option<f64> {
        if s.ends_with("ms") {
            s.trim_end_matches("ms").parse::<f64>().ok()
        } else if s.ends_with('s') {
            s.trim_end_matches('s')
                .parse::<f64>()
                .map(|v| v * 1000.0)
                .ok()
        } else {
            s.parse::<f64>().ok()
        }
    }

    /// 解析缓动函数
    fn parse_easing(s: &str) -> EasingFunction {
        match s {
            "linear" => EasingFunction::Linear,
            "ease" => EasingFunction::EaseInOut,
            "ease-in" => EasingFunction::EaseIn,
            "ease-out" => EasingFunction::EaseOut,
            "ease-in-out" => EasingFunction::EaseInOut,
            _ => EasingFunction::EaseInOut,
        }
    }
}

/// 动画状态跟踪器
#[derive(Debug, Clone)]
pub struct ElementAnimationState {
    /// 元素 ID
    pub element_id: u64,
    /// 当前属性值（用于动画插值）
    pub current_values: std::collections::HashMap<String, f32>,
    /// 活动的 transitions
    pub active_transitions: std::collections::HashMap<String, TransitionAnimation>,
}

/// 单个属性的过渡动画
#[derive(Debug, Clone)]
pub struct TransitionAnimation {
    /// 属性名
    pub property: String,
    /// 起始值
    pub from: f32,
    /// 目标值
    pub to: f32,
    /// 开始时间（毫秒时间戳）
    pub start_time: f64,
    /// 持续时间（毫秒）
    pub duration: f64,
    /// 缓动函数
    pub easing: EasingFunction,
}

impl TransitionAnimation {
    /// 计算当前值
    pub fn current_value(&self, current_time: f64) -> f32 {
        let elapsed = current_time - self.start_time;

        if elapsed >= self.duration {
            return self.to;
        }

        let progress = (elapsed / self.duration) as f32;
        let eased_progress = self.easing.apply(progress);

        self.from + (self.to - self.from) * eased_progress
    }

    /// 检查是否完成
    pub fn is_completed(&self, current_time: f64) -> bool {
        current_time - self.start_time >= self.duration
    }
}

impl ElementAnimationState {
    /// 创建新的动画状态
    pub fn new(element_id: u64) -> Self {
        Self {
            element_id,
            current_values: std::collections::HashMap::new(),
            active_transitions: std::collections::HashMap::new(),
        }
    }

    /// 启动过渡动画
    pub fn start_transition(
        &mut self,
        property: String,
        from: f32,
        to: f32,
        config: &TransitionConfig,
        current_time: f64,
    ) {
        let transition = TransitionAnimation {
            property: property.clone(),
            from,
            to,
            start_time: current_time + config.delay,
            duration: config.duration,
            easing: config.easing.clone(),
        };

        self.active_transitions.insert(property, transition);
    }

    /// 更新所有活动动画
    pub fn update(&mut self, current_time: f64) -> bool {
        let mut has_changes = false;

        // 移除完成的动画
        self.active_transitions.retain(|property, transition| {
            if transition.is_completed(current_time) {
                let value = transition.current_value(current_time);
                self.current_values.insert(property.clone(), value);
                false // 移除
            } else {
                true // 保留
            }
        });

        // 更新进行中的动画
        for (property, transition) in &self.active_transitions {
            let value = transition.current_value(current_time);
            self.current_values.insert(property.clone(), value);
            has_changes = true;
        }

        has_changes
    }

    /// 获取属性的动画值
    pub fn get_animated_value(&self, property: &str) -> Option<f32> {
        self.current_values.get(property).copied()
    }

    /// 检查是否有活动动画
    pub fn has_active_animations(&self) -> bool {
        !self.active_transitions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_transition_css() {
        let config = TransitionConfig::from_css("opacity 0.3s ease-in-out").unwrap();
        assert_eq!(config.property, "opacity");
        assert!((config.duration - 300.0).abs() < f64::EPSILON);
        assert!(matches!(config.easing, EasingFunction::EaseInOut));
    }

    #[test]
    fn test_parse_duration() {
        assert!((TransitionConfig::parse_duration("0.3s").unwrap() - 300.0).abs() < f64::EPSILON);
        assert!((TransitionConfig::parse_duration("300ms").unwrap() - 300.0).abs() < f64::EPSILON);
        assert!((TransitionConfig::parse_duration("1s").unwrap() - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_transition_animation() {
        let transition = TransitionAnimation {
            property: "opacity".to_string(),
            from: 0.0,
            to: 1.0,
            start_time: 1000.0,
            duration: 1000.0,
            easing: EasingFunction::Linear,
        };

        // 50% 进度
        let value = transition.current_value(1500.0);
        assert!((value - 0.5).abs() < 0.01);

        // 100% 进度
        let value = transition.current_value(2000.0);
        assert!((value - 1.0).abs() < f32::EPSILON);

        // 已完成
        assert!(transition.is_completed(2000.0));
        assert!(!transition.is_completed(1500.0));
    }

    #[test]
    fn test_element_animation_state() {
        let mut state = ElementAnimationState::new(1);

        let config = TransitionConfig {
            property: "opacity".to_string(),
            duration: 1000.0,
            easing: EasingFunction::Linear,
            delay: 0.0,
        };

        state.start_transition("opacity".to_string(), 0.0, 1.0, &config, 1000.0);

        assert!(state.has_active_animations());

        // 更新到 50% 进度
        state.update(1500.0);
        let value = state.get_animated_value("opacity").unwrap();
        assert!((value - 0.5).abs() < 0.01);
    }
}
