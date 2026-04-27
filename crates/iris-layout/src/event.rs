//! 事件系统
//!
//! 提供 DOM 事件的基础设施，包括事件对象、事件目标、事件冒泡/捕获机制。

use std::collections::HashMap;
use std::sync::Arc;

/// 事件阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhase {
    /// 捕获阶段
    Capturing,
    /// 目标阶段
    AtTarget,
    /// 冒泡阶段
    Bubbling,
}

/// 事件对象
#[derive(Debug, Clone)]
pub struct Event {
    /// 事件类型
    pub event_type: String,
    /// 事件目标
    pub target: Option<u64>,
    /// 当前目标（事件传播过程中的当前节点）
    pub current_target: Option<u64>,
    /// 事件阶段
    pub phase: EventPhase,
    /// 是否可冒泡
    pub bubbles: bool,
    /// 是否可取消
    pub cancelable: bool,
    /// 是否已停止传播
    pub propagation_stopped: bool,
    /// 是否已阻止默认行为
    pub default_prevented: bool,
    /// 事件创建时间戳（毫秒）
    pub timestamp: u64,
}

impl Event {
    /// 创建新事件
    pub fn new(event_type: &str) -> Self {
        Self {
            event_type: event_type.to_string(),
            target: None,
            current_target: None,
            phase: EventPhase::Capturing,
            bubbles: true,
            cancelable: false,
            propagation_stopped: false,
            default_prevented: false,
            timestamp: current_timestamp(),
        }
    }

    /// 创建可冒泡事件
    pub fn new_bubbling(event_type: &str) -> Self {
        let mut event = Self::new(event_type);
        event.bubbles = true;
        event
    }

    /// 创建可取消事件
    pub fn new_cancelable(event_type: &str) -> Self {
        let mut event = Self::new(event_type);
        event.cancelable = true;
        event
    }

    /// 停止事件传播
    pub fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    /// 阻止默认行为
    pub fn prevent_default(&mut self) {
        if self.cancelable {
            self.default_prevented = true;
        }
    }
}

/// 事件监听器 trait
pub trait EventListener: Send + Sync {
    /// 处理事件
    fn handle_event(&self, event: &mut Event);
}

/// 函数式事件监听器
pub struct FnListener<F>(pub F)
where
    F: Fn(&mut Event) + Send + Sync;

impl<F> EventListener for FnListener<F>
where
    F: Fn(&mut Event) + Send + Sync,
{
    fn handle_event(&self, event: &mut Event) {
        (self.0)(event);
    }
}

/// 事件监听器包装
#[derive(Clone)]
pub struct EventListenerHandle {
    /// 监听器 ID
    pub id: u64,
    /// 事件类型
    pub event_type: String,
    /// 是否在捕获阶段触发
    pub capture: bool,
    /// 监听器
    pub listener: Arc<dyn EventListener>,
}

/// 事件目标 trait
pub trait EventTarget {
    /// 添加事件监听器
    fn add_event_listener<F>(&mut self, event_type: &str, listener: F, capture: bool) -> u64
    where
        F: Fn(&mut Event) + Send + Sync + 'static;

    /// 移除事件监听器
    fn remove_event_listener(&mut self, listener_id: u64) -> bool;

    /// 触发事件
    fn dispatch_event(&self, event: &mut Event);

    /// 获取节点 ID
    fn node_id(&self) -> u64;
}

/// 事件监听器注册表
#[derive(Default)]
pub struct EventRegistry {
    /// 监听器 ID 计数器
    next_id: u64,
    /// 按节点 ID 和事件类型分组的监听器
    listeners: HashMap<(u64, String), Vec<EventListenerHandle>>,
}

impl EventRegistry {
    /// 创建新的事件注册表
    pub fn new() -> Self {
        Self {
            next_id: 1,
            listeners: HashMap::new(),
        }
    }

    /// 添加事件监听器
    pub fn add_listener<F>(
        &mut self,
        node_id: u64,
        event_type: &str,
        listener: F,
        capture: bool,
    ) -> u64
    where
        F: Fn(&mut Event) + Send + Sync + 'static,
    {
        let id = self.next_id;
        self.next_id += 1;

        let handle = EventListenerHandle {
            id,
            event_type: event_type.to_string(),
            capture,
            listener: Arc::new(FnListener(listener)),
        };

        let key = (node_id, event_type.to_string());
        self.listeners.entry(key).or_insert_with(Vec::new).push(handle);

        id
    }

    /// 移除事件监听器
    pub fn remove_listener(&mut self, node_id: u64, listener_id: u64) -> bool {
        let mut removed = false;

        for (key, listeners) in self.listeners.iter_mut() {
            if key.0 == node_id {
                if let Some(pos) = listeners.iter().position(|l| l.id == listener_id) {
                    listeners.remove(pos);
                    removed = true;
                    break;
                }
            }
        }

        removed
    }

    /// 获取节点指定事件类型的所有监听器
    pub fn get_listeners(&self, node_id: u64, event_type: &str, capture: bool) -> Vec<EventListenerHandle> {
        let key = (node_id, event_type.to_string());
        if let Some(listeners) = self.listeners.get(&key) {
            listeners
                .iter()
                .filter(|l| l.capture == capture)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 检查节点是否有指定事件类型的监听器
    pub fn has_listeners(&self, node_id: u64, event_type: &str) -> bool {
        let key_capture = (node_id, event_type.to_string());
        let key_bubble = (node_id, event_type.to_string());
        self.listeners.contains_key(&key_capture) || self.listeners.contains_key(&key_bubble)
    }
}

/// 获取当前时间戳（毫秒）
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new("click");
        assert_eq!(event.event_type, "click");
        assert!(event.bubbles);
        assert!(!event.cancelable);
        assert!(!event.propagation_stopped);
    }

    #[test]
    fn test_stop_propagation() {
        let mut event = Event::new("click");
        assert!(!event.propagation_stopped);
        event.stop_propagation();
        assert!(event.propagation_stopped);
    }

    #[test]
    fn test_prevent_default() {
        let mut event = Event::new_cancelable("submit");
        assert!(!event.default_prevented);
        event.prevent_default();
        assert!(event.default_prevented);

        // 不可取消的事件
        let mut event2 = Event::new("click");
        event2.prevent_default();
        assert!(!event2.default_prevented);
    }

    #[test]
    fn test_event_registry() {
        let mut registry = EventRegistry::new();
        
        // 添加监听器
        let id1 = registry.add_listener(1, "click", |_| {}, false);
        let id2 = registry.add_listener(1, "click", |_| {}, true);
        let id3 = registry.add_listener(2, "click", |_| {}, false);

        // 验证 ID 递增
        assert!(id1 < id2 && id2 < id3);

        // 获取监听器
        let bubble_listeners = registry.get_listeners(1, "click", false);
        assert_eq!(bubble_listeners.len(), 1);

        let capture_listeners = registry.get_listeners(1, "click", true);
        assert_eq!(capture_listeners.len(), 1);

        // 移除监听器
        assert!(registry.remove_listener(1, id1));
        assert!(!registry.remove_listener(1, id1)); // 已移除

        let bubble_listeners = registry.get_listeners(1, "click", false);
        assert_eq!(bubble_listeners.len(), 0);
    }

    #[test]
    fn test_fn_listener() {
        use std::sync::atomic::{AtomicBool, Ordering};
        
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();
        
        let listener = FnListener(move |event| {
            flag_clone.store(true, Ordering::SeqCst);
            event.stop_propagation();
        });
        
        let mut event = Event::new("test");
        listener.handle_event(&mut event);
        
        assert!(flag.load(Ordering::SeqCst));
        assert!(event.propagation_stopped);
    }
}
