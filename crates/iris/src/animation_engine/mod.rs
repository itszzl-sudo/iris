//! Iris 动画引擎
//!
//! 实现 CSS Transitions 和 Animations，支持：
//! - CSS Transition（属性过渡动画）
//! - CSS @keyframes Animation（关键帧动画）
//! - 缓动函数（Easing Functions）
//! - 动画状态管理和时间轴控制

pub mod easing;
pub mod applier;

pub use easing::{EasingFunction, ease_in_out, ease_in, ease_out, linear, ease_elastic, ease_bounce};
pub use applier::{TransitionConfig, ElementAnimationState, TransitionAnimation, AnimatedValue};
