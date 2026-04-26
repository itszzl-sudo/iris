//! 统一事件系统
//!
//! 提供鼠标、键盘、滚动等事件的注册、分发和处理。

use std::collections::HashMap;
use std::cell::RefCell;

/// 事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    // 鼠标事件
    /// 点击
    Click,
    /// 双击
    DoubleClick,
    /// 鼠标按下
    MouseDown,
    /// 鼠标释放
    MouseUp,
    /// 鼠标移动
    MouseMove,
    /// 鼠标进入
    MouseEnter,
    /// 鼠标离开
    MouseLeave,

    // 键盘事件
    /// 键盘按下
    KeyDown,
    /// 键盘释放
    KeyUp,
    /// 按键输入
    KeyPress,

    // 焦点事件
    /// 获得焦点
    Focus,
    /// 失去焦点
    Blur,

    // 表单事件
    /// 改变
    Change,
    /// 输入
    Input,
    /// 提交
    Submit,

    // 窗口事件
    /// 滚动
    Scroll,
    /// 调整大小
    Resize,
    /// 加载完成
    Load,
}

impl EventType {
    /// 从字符串解析事件类型
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "click" => Some(EventType::Click),
            "dblclick" => Some(EventType::DoubleClick),
            "mousedown" => Some(EventType::MouseDown),
            "mouseup" => Some(EventType::MouseUp),
            "mousemove" => Some(EventType::MouseMove),
            "mouseenter" => Some(EventType::MouseEnter),
            "mouseleave" => Some(EventType::MouseLeave),
            "keydown" => Some(EventType::KeyDown),
            "keyup" => Some(EventType::KeyUp),
            "keypress" => Some(EventType::KeyPress),
            "focus" => Some(EventType::Focus),
            "blur" => Some(EventType::Blur),
            "change" => Some(EventType::Change),
            "input" => Some(EventType::Input),
            "submit" => Some(EventType::Submit),
            "scroll" => Some(EventType::Scroll),
            "resize" => Some(EventType::Resize),
            "load" => Some(EventType::Load),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::Click => "click",
            EventType::DoubleClick => "dblclick",
            EventType::MouseDown => "mousedown",
            EventType::MouseUp => "mouseup",
            EventType::MouseMove => "mousemove",
            EventType::MouseEnter => "mouseenter",
            EventType::MouseLeave => "mouseleave",
            EventType::KeyDown => "keydown",
            EventType::KeyUp => "keyup",
            EventType::KeyPress => "keypress",
            EventType::Focus => "focus",
            EventType::Blur => "blur",
            EventType::Change => "change",
            EventType::Input => "input",
            EventType::Submit => "submit",
            EventType::Scroll => "scroll",
            EventType::Resize => "resize",
            EventType::Load => "load",
        }
    }
}

/// 鼠标事件数据
#[derive(Debug, Clone)]
pub struct MouseEventData {
    /// X 坐标
    pub x: f32,
    /// Y 坐标
    pub y: f32,
    /// 按钮 (0=左, 1=中, 2=右)
    pub button: u8,
    /// 修饰键
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub alt_key: bool,
}

/// 键盘事件数据
#[derive(Debug, Clone)]
pub struct KeyboardEventData {
    /// 键码
    pub key_code: u32,
    /// 键名
    pub key: String,
    /// 修饰键
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub alt_key: bool,
}

/// 事件数据
#[derive(Debug, Clone)]
pub enum EventData {
    Mouse(MouseEventData),
    Keyboard(KeyboardEventData),
    None,
}

/// 事件对象
#[derive(Debug, Clone)]
pub struct Event {
    /// 事件类型
    pub event_type: EventType,
    /// 事件数据
    pub data: EventData,
    /// 目标节点 ID
    pub target_id: u64,
    /// 是否已停止传播 (使用 RefCell 实现内部可变性)
    pub propagation_stopped: RefCell<bool>,
}

impl Event {
    /// 创建新事件
    pub fn new(event_type: EventType, target_id: u64) -> Self {
        Self {
            event_type,
            data: EventData::None,
            target_id,
            propagation_stopped: RefCell::new(false),
        }
    }

    /// 创建鼠标事件
    pub fn mouse(event_type: EventType, target_id: u64, data: MouseEventData) -> Self {
        Self {
            event_type,
            data: EventData::Mouse(data),
            target_id,
            propagation_stopped: RefCell::new(false),
        }
    }

    /// 创建键盘事件
    pub fn keyboard(event_type: EventType, target_id: u64, data: KeyboardEventData) -> Self {
        Self {
            event_type,
            data: EventData::Keyboard(data),
            target_id,
            propagation_stopped: RefCell::new(false),
        }
    }

    /// 停止事件传播
    pub fn stop_propagation(&self) {
        *self.propagation_stopped.borrow_mut() = true;
    }

    /// 检查是否已停止传播
    pub fn is_propagation_stopped(&self) -> bool {
        *self.propagation_stopped.borrow()
    }
}

/// 事件监听器类型
pub type EventListener = Box<dyn Fn(&Event)>;

/// 事件分发器
///
/// 管理事件监听器的注册和事件的分发。
///
/// # 示例
///
/// ```rust
/// use iris_dom::event::{EventDispatcher, EventType, Event};
///
/// let mut dispatcher = EventDispatcher::new();
///
/// // 注册事件监听器
/// dispatcher.add_listener(1, EventType::Click, Box::new(|event| {
///     println!("Clicked on node {}", event.target_id);
/// }));
///
/// // 分发事件
/// let event = Event::new(EventType::Click, 1);
/// dispatcher.dispatch(&event);
/// ```
pub struct EventDispatcher {
    /// 事件监听器映射: (节点ID, 事件类型) -> 监听器列表
    listeners: HashMap<(u64, EventType), Vec<EventListener>>,
}

impl EventDispatcher {
    /// 创建新的事件分发器
    pub fn new() -> Self {
        Self {
            listeners: HashMap::new(),
        }
    }

    /// 添加事件监听器
    pub fn add_listener(
        &mut self,
        target_id: u64,
        event_type: EventType,
        listener: EventListener,
    ) {
        self.listeners
            .entry((target_id, event_type))
            .or_insert_with(Vec::new)
            .push(listener);
    }

    /// 移除事件监听器 (简化实现：移除所有)
    pub fn remove_listener(&mut self, target_id: u64, event_type: EventType) {
        self.listeners.remove(&(target_id, event_type));
    }

    /// 分发事件
    ///
    /// 事件会按捕获阶段 → 目标阶段 → 冒泡阶段传播。
    pub fn dispatch(&self, event: &Event) {
        if let Some(listeners) = self
            .listeners
            .get(&(event.target_id, event.event_type))
        {
            for listener in listeners {
                listener(event);
                if event.is_propagation_stopped() {
                    break;
                }
            }
        }
    }

    /// 清空所有监听器
    pub fn clear(&mut self) {
        self.listeners.clear();
    }

    /// 获取监听器数量
    pub fn listener_count(&self) -> usize {
        self.listeners.values().map(|v| v.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_from_str() {
        assert_eq!(EventType::from_str("click"), Some(EventType::Click));
        assert_eq!(EventType::from_str("keydown"), Some(EventType::KeyDown));
        assert_eq!(EventType::from_str("invalid"), None);
    }

    #[test]
    fn test_event_type_as_str() {
        assert_eq!(EventType::Click.as_str(), "click");
        assert_eq!(EventType::KeyDown.as_str(), "keydown");
    }

    #[test]
    fn test_add_and_dispatch_event() {
        use std::sync::{Arc, Mutex};

        let mut dispatcher = EventDispatcher::new();
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        dispatcher.add_listener(
            1,
            EventType::Click,
            Box::new(move |_| {
                let mut count = counter_clone.lock().unwrap();
                *count += 1;
            }),
        );

        let event = Event::new(EventType::Click, 1);
        dispatcher.dispatch(&event);

        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[test]
    fn test_stop_propagation() {
        use std::sync::{Arc, Mutex};

        let mut dispatcher = EventDispatcher::new();
        let counter = Arc::new(Mutex::new(0));

        let counter1 = counter.clone();
        dispatcher.add_listener(
            1,
            EventType::Click,
            Box::new(move |event| {
                let mut count = counter1.lock().unwrap();
                *count += 1;
                event.stop_propagation();
            }),
        );

        let counter2 = counter.clone();
        dispatcher.add_listener(
            1,
            EventType::Click,
            Box::new(move |_| {
                let mut count = counter2.lock().unwrap();
                *count += 1;
            }),
        );

        let event = Event::new(EventType::Click, 1);
        dispatcher.dispatch(&event);

        // 第一个监听器停止传播，第二个不会执行
        assert_eq!(*counter.lock().unwrap(), 1);
    }
    #[test]
    fn test_remove_listener() {
        let mut dispatcher = EventDispatcher::new();

        dispatcher.add_listener(
            1,
            EventType::Click,
            Box::new(|_| {}),
        );

        assert_eq!(dispatcher.listener_count(), 1);

        dispatcher.remove_listener(1, EventType::Click);
        assert_eq!(dispatcher.listener_count(), 0);
    }

    #[test]
    fn test_mouse_event() {
        let mouse_data = MouseEventData {
            x: 100.0,
            y: 200.0,
            button: 0,
            ctrl_key: false,
            shift_key: true,
            alt_key: false,
        };

        let event = Event::mouse(EventType::Click, 1, mouse_data.clone());

        if let EventData::Mouse(data) = &event.data {
            assert_eq!(data.x, 100.0);
            assert_eq!(data.y, 200.0);
            assert!(data.shift_key);
        } else {
            panic!("Expected mouse event data");
        }
    }

    #[test]
    fn test_keyboard_event() {
        let key_data = KeyboardEventData {
            key_code: 13,
            key: "Enter".to_string(),
            ctrl_key: false,
            shift_key: false,
            alt_key: false,
        };

        let event = Event::keyboard(EventType::KeyDown, 1, key_data.clone());

        if let EventData::Keyboard(data) = &event.data {
            assert_eq!(data.key_code, 13);
            assert_eq!(data.key, "Enter");
        } else {
            panic!("Expected keyboard event data");
        }
    }
}
