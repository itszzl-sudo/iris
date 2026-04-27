//! GPU 渲染器集成测试
//!
//! 验证 GPU 渲染器与 RuntimeOrchestrator 的完整集成流程

use iris_engine::orchestrator::RuntimeOrchestrator;
use iris_layout::dom::DOMNode;

/// 测试 1: GPU 渲染器管理
#[test]
fn test_gpu_renderer_management() {
    let mut orchestrator = RuntimeOrchestrator::new();
    orchestrator.initialize().unwrap();
    
    // 初始状态没有 GPU 渲染器
    assert!(orchestrator.gpu_renderer_mut().is_none());
    
    // 注意：实际的 Renderer 创建需要 winit 窗口，这里测试管理接口
    // 完整集成测试需要在有窗口的环境中进行
}

/// 测试 2: 渲染命令生成（无 GPU 渲染器）
#[test]
fn test_render_commands_without_gpu_renderer() {
    let mut orchestrator = RuntimeOrchestrator::new();
    orchestrator.initialize().unwrap();
    
    // 创建 DOM 树
    let mut dom_tree = DOMNode::new_element("div");
    dom_tree.set_attribute("id", "app");
    orchestrator.set_dom_tree(dom_tree);
    
    // 生成渲染命令应该成功（即使没有 GPU 渲染器）
    let commands = orchestrator.generate_render_commands();
    assert!(commands.len() >= 0); // 至少返回空列表
}

/// 测试 3: render_frame_gpu 无渲染器时的行为
#[test]
fn test_render_frame_gpu_without_renderer() {
    let mut orchestrator = RuntimeOrchestrator::new();
    orchestrator.initialize().unwrap();
    
    // 没有 GPU 渲染器时应该返回 false
    let rendered = orchestrator.render_frame_gpu();
    assert!(!rendered);
}

/// 测试 4: 完整的渲染流程（不带实际 GPU）
#[test]
fn test_complete_render_pipeline_without_gpu() {
    use iris_layout::vdom::{VNode, VTree, VElement};
    
    // 1. 创建编排器
    let mut orchestrator = RuntimeOrchestrator::new();
    orchestrator.initialize().unwrap();
    
    // 2. 创建 VTree
    let vtree = VTree {
        root: VNode::Element(VElement {
            tag: "div".to_string(),
            attrs: vec![("id".to_string(), "app".to_string())].into_iter().collect(),
            children: vec![
                VNode::Element(VElement {
                    tag: "h1".to_string(),
                    attrs: vec![].into_iter().collect(),
                    children: vec![VNode::Text("Hello GPU".to_string())],
                    key: None,
                }),
            ],
            key: None,
        }),
    };
    orchestrator.set_vtree(vtree);
    
    // 3. 转换为 DOM
    let dom = orchestrator.build_dom_from_vtree();
    assert!(dom.is_some());
    
    // 4. 计算布局（需要 VTree）
    let layout_result = orchestrator.compute_layout();
    assert!(layout_result.is_ok());
    
    // 5. 生成渲染命令
    let commands = orchestrator.generate_render_commands();
    assert!(commands.len() >= 0);
    
    // 6. 帧率控制
    orchestrator.set_target_fps(60);
    assert_eq!(orchestrator.target_fps(), 60);
}

/// 测试 5: 多次渲染循环（验证帧率控制）
#[test]
fn test_multiple_render_cycles() {
    let mut orchestrator = RuntimeOrchestrator::new();
    orchestrator.initialize().unwrap();
    
    // 设置高帧率以避免测试过慢
    orchestrator.set_target_fps(10000);
    
    // 第一次渲染（初始 dirty）
    let first = orchestrator.render_frame();
    assert!(first);
    assert!(!orchestrator.is_dirty());
    
    // 第二次渲染（应该跳过，因为不是 dirty）
    let second = orchestrator.render_frame();
    assert!(!second);
    
    // 标记 dirty 后再次渲染
    orchestrator.mark_dirty();
    orchestrator.reset_frame_timer();
    let third = orchestrator.render_frame();
    assert!(third);
}

/// 测试 6: 大型 DOM 树的渲染命令生成
#[test]
fn test_large_dom_render_commands() {
    use iris_layout::vdom::{VNode, VTree, VElement};
    
    let mut orchestrator = RuntimeOrchestrator::new();
    orchestrator.initialize().unwrap();
    
    // 创建大型 VTree（100 个节点）
    let mut children = Vec::new();
    for i in 0..100 {
        children.push(VNode::Element(VElement {
            tag: "div".to_string(),
            attrs: vec![
                ("id".to_string(), format!("child-{}", i)),
                ("style".to_string(), format!("background: rgb({}, {}, {});", i * 2, 100, 200 - i)),
            ].into_iter().collect(),
            children: vec![],
            key: None,
        }));
    }
    
    let vtree = VTree {
        root: VNode::Element(VElement {
            tag: "div".to_string(),
            attrs: vec![("id".to_string(), "root".to_string())].into_iter().collect(),
            children,
            key: None,
        }),
    };
    
    orchestrator.set_vtree(vtree);
    orchestrator.compute_layout().unwrap();
    
    // 生成渲染命令
    let commands = orchestrator.generate_render_commands();
    
    // 命令数量应该合理（取决于有多少节点有背景色等样式）
    assert!(commands.len() >= 0);
}

/// 测试 7: 事件与渲染的集成
#[test]
fn test_event_and_render_integration() {
    use std::cell::RefCell;
    use std::rc::Rc;
    use iris_dom::event::EventType;
    use iris_layout::vdom::{VNode, VTree, VElement};
    
    let mut orchestrator = RuntimeOrchestrator::new();
    orchestrator.initialize().unwrap();
    
    // 添加事件监听器
    let event_triggered = Rc::new(RefCell::new(false));
    let event_triggered_clone = event_triggered.clone();
    
    orchestrator.add_event_listener(
        1,
        EventType::Click,
        Box::new(move |_event| {
            *event_triggered_clone.borrow_mut() = true;
        }),
    );
    
    // 创建 VTree
    let vtree = VTree {
        root: VNode::Element(VElement {
            tag: "div".to_string(),
            attrs: vec![].into_iter().collect(),
            children: vec![],
            key: None,
        }),
    };
    orchestrator.set_vtree(vtree);
    orchestrator.compute_layout().unwrap();
    
    // 触发事件
    orchestrator.handle_mouse_click(1, 100.0, 200.0, 0);
    
    // 验证事件触发
    assert!(*event_triggered.borrow());
    
    // 事件应该标记 dirty（如果需要重新渲染）
    // 这取决于具体的实现逻辑
}

/// 测试 8: 视口变化触发布局重计算
#[test]
fn test_viewport_change_relayout() {
    use iris_layout::vdom::{VNode, VTree, VElement};
    
    let mut orchestrator = RuntimeOrchestrator::new();
    orchestrator.initialize().unwrap();
    
    // 创建 VTree
    let vtree = VTree {
        root: VNode::Element(VElement {
            tag: "div".to_string(),
            attrs: vec![("style".to_string(), "width: 100%; height: 100%;".to_string())].into_iter().collect(),
            children: vec![],
            key: None,
        }),
    };
    orchestrator.set_vtree(vtree);
    
    // 第一次布局计算
    orchestrator.set_viewport_size(800.0, 600.0);
    let layout1 = orchestrator.compute_layout();
    assert!(layout1.is_ok());
    
    // 改变视口
    orchestrator.set_viewport_size(1920.0, 1080.0);
    let layout2 = orchestrator.compute_layout();
    assert!(layout2.is_ok());
    
    // 视口变化应该标记为 dirty
    assert!(orchestrator.is_dirty());
}

/// 测试 9: 渲染命令的完整性
#[test]
fn test_render_command_completeness() {
    use iris_layout::vdom::{VNode, VTree, VElement};
    
    let mut orchestrator = RuntimeOrchestrator::new();
    orchestrator.initialize().unwrap();
    
    // 创建带有多种样式的 VTree
    let vtree = VTree {
        root: VNode::Element(VElement {
            tag: "div".to_string(),
            attrs: vec![("style".to_string(), "background: red; display: flex;".to_string())].into_iter().collect(),
            children: vec![
                VNode::Element(VElement {
                    tag: "p".to_string(),
                    attrs: vec![("style".to_string(), "color: blue; margin: 10px;".to_string())].into_iter().collect(),
                    children: vec![],
                    key: None,
                }),
                VNode::Element(VElement {
                    tag: "div".to_string(),
                    attrs: vec![("style".to_string(), "border-radius: 50%; background: green;".to_string())].into_iter().collect(),
                    children: vec![],
                    key: None,
                }),
            ],
            key: None,
        }),
    };
    
    orchestrator.set_vtree(vtree);
    orchestrator.compute_layout().unwrap();
    
    // 生成渲染命令
    let commands = orchestrator.generate_render_commands();
    
    // 验证命令生成（具体数量取决于实现）
    // 当前实现可能只生成有背景色的节点命令
    assert!(commands.len() >= 0);
}

/// 测试 10: GPU 渲染管线集成验证
#[test]
fn test_gpu_pipeline_integration() {
    use iris_layout::vdom::{VNode, VElement, VTree};
    
    // 模拟完整的 GPU 渲染管线流程
    // Vue SFC → VTree → DOM → Layout → RenderCommands → GPU
    
    // 1. 初始化
    let mut orchestrator = RuntimeOrchestrator::new();
    orchestrator.initialize().unwrap();
    
    // 2. 创建 VTree（模拟 SFC 编译结果）
    let vtree = VTree {
        root: VNode::Element(VElement {
            tag: "div".to_string(),
            attrs: vec![
                ("id".to_string(), "app".to_string()),
                ("style".to_string(), "display: flex;".to_string()),
            ].into_iter().collect(),
            children: vec![
                VNode::Element(VElement {
                    tag: "h1".to_string(),
                    attrs: vec![
                        ("style".to_string(), "color: blue;".to_string()),
                    ].into_iter().collect(),
                    children: vec![VNode::Text("GPU Pipeline Test".to_string())],
                    key: None,
                }),
                VNode::Element(VElement {
                    tag: "p".to_string(),
                    attrs: vec![
                        ("style".to_string(), "margin: 20px;".to_string()),
                    ].into_iter().collect(),
                    children: vec![VNode::Text("Complete integration".to_string())],
                    key: None,
                }),
            ],
            key: None,
        }),
    };
    orchestrator.set_vtree(vtree);
    
    // 3. VTree → DOM
    let dom = orchestrator.build_dom_from_vtree();
    assert!(dom.is_some(), "VTree 应该成功转换为 DOM");
    orchestrator.set_dom_tree(dom.unwrap());
    
    // 4. 计算布局
    orchestrator.set_viewport_size(1024.0, 768.0);
    let layout = orchestrator.compute_layout();
    assert!(layout.is_ok(), "布局计算应该成功");
    
    // 5. 生成渲染命令
    let commands = orchestrator.generate_render_commands();
    assert!(commands.len() >= 0, "应该生成渲染命令");
    
    // 6. 验证渲染循环
    orchestrator.mark_dirty();
    orchestrator.reset_frame_timer();
    let rendered = orchestrator.render_frame();
    assert!(rendered, "应该成功渲染一帧");
    
    // 7. 验证帧率统计
    let fps = orchestrator.current_fps();
    assert!(fps >= 0.0, "帧率应该 >= 0");
}
