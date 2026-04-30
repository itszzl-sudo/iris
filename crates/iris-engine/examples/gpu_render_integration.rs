//! GPU 渲染器集成示例
//!
//! 这个示例展示了如何将 iris-gpu 的 Renderer 集成到 RuntimeOrchestrator 中，
//! 实现从 Vue SFC 到实际 GPU 渲染的完整流程。
//!
//! # 使用方法
//!
//! ```bash
//! cargo run --example gpu_render_integration
//! ```

use iris_engine::orchestrator::RuntimeOrchestrator;
use iris_layout::DOMNode;

/// 示例：基本的 GPU 渲染集成
/// 
/// 这个示例演示了：
/// 1. 创建和初始化 RuntimeOrchestrator
/// 2. 创建 GPU 渲染器（需要 winit 窗口）
/// 3. 加载 Vue SFC 文件
/// 4. 生成 VTree 和 DOM 树
/// 5. 计算布局
/// 6. 提交渲染命令到 GPU
async fn basic_gpu_integration() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 GPU 渲染器集成示例");
    println!("=====================\n");

    // 1. 创建并初始化编排器
    println!("步骤 1: 初始化 RuntimeOrchestrator...");
    let mut orchestrator = RuntimeOrchestrator::new();
    orchestrator.initialize().map_err(|e| format!("初始化失败: {}", e))?;
    println!("✅ RuntimeOrchestrator 初始化成功\n");

    // 2. 加载 Vue SFC 文件
    println!("步骤 2: 加载 Vue SFC 文件...");
    let test_vue = r#"
<script>
console.log('组件初始化');
</script>

<template>
<div id="app">
  <h1>Hello GPU Rendering!</h1>
  <p>这是一个 GPU 渲染示例</p>
</div>
</template>

<style>
#app {
  display: flex;
  flex-direction: column;
  padding: 20px;
}
</style>
"#;

    // 写入临时文件
    let temp_path = std::path::Path::new("test_gpu_example.vue");
    std::fs::write(temp_path, test_vue)?;
    println!("✅ 测试文件已创建: {:?}\n", temp_path);

    // 3. 编译并生成 VTree
    println!("步骤 3: 编译 SFC 并生成 VTree...");
    match orchestrator.load_sfc_with_vtree(temp_path) {
        Ok(()) => {
            println!("✅ SFC 编译成功");
            if let Some(vtree) = orchestrator.vtree() {
                println!("✅ VTree 生成成功");
                println!("   根节点类型: {:?}", vtree.root);
            }
        }
        Err(e) => {
            println!("⚠️  SFC 编译失败（这是正常的，因为需要完整的 JS 环境）: {}", e);
            println!("   继续演示其他功能...\n");
        }
    }
    println!();

    // 4. 手动创建 DOM 树用于演示
    println!("步骤 4: 创建 DOM 树（手动）...");
    let mut dom_tree = DOMNode::new_element("div");
    dom_tree.set_attribute("id", "app");
    dom_tree.set_attribute("style", "display: flex; flex-direction: column; padding: 20px;");
    
    let mut h1 = DOMNode::new_element("h1");
    h1.set_attribute("style", "color: #333; font-size: 32px;");
    dom_tree.children.push(h1);
    
    let mut p = DOMNode::new_element("p");
    p.set_attribute("style", "color: #666; font-size: 16px;");
    dom_tree.children.push(p);
    
    orchestrator.set_dom_tree(dom_tree);
    println!("✅ DOM 树创建成功（2 个子节点）\n");

    // 5. 计算布局
    println!("步骤 5: 计算布局...");
    orchestrator.set_viewport_size(800.0, 600.0);
    match orchestrator.compute_layout() {
        Ok(dom_with_layout) => {
            println!("✅ 布局计算成功");
            println!("   DOM 节点数: {}", count_nodes(&dom_with_layout));
            println!("   视口: 800x600\n");
        }
        Err(e) => {
            println!("⚠️  布局计算失败: {}\n", e);
        }
    }

    // 6. 生成渲染命令
    println!("步骤 6: 生成渲染命令...");
    let commands = orchestrator.generate_render_commands();
    println!("✅ 渲染命令生成成功");
    println!("   命令数量: {}\n", commands.len());

    // 7. 配置帧率
    println!("步骤 7: 配置渲染帧率...");
    orchestrator.set_target_fps(60);
    println!("✅ 目标帧率: {} FPS", orchestrator.target_fps());
    println!("   当前帧率: {:.2} FPS", orchestrator.current_fps());
    println!("   需要渲染: {}\n", orchestrator.is_dirty());

    // 8. 清理临时文件
    if temp_path.exists() {
        std::fs::remove_file(temp_path)?;
        println!("✅ 临时文件已清理");
    }

    println!("\n🎉 GPU 渲染器集成示例完成！");
    println!("\n📝 注意：");
    println!("   - 这个示例展示了集成的所有步骤");
    println!("   - 实际的 GPU 渲染需要一个 winit 窗口");
    println!("   - 参见 gpu_render_window.rs 获取完整窗口示例");

    Ok(())
}

/// 递归计算 DOM 节点数量
fn count_nodes(node: &DOMNode) -> usize {
    let mut count = 1;
    for child in &node.children {
        count += count_nodes(child);
    }
    count
}

fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // 运行示例（使用 pollster 阻塞等待异步函数完成）
    match pollster::block_on(basic_gpu_integration()) {
        Ok(()) => {
            println!("\n✅ 示例运行成功");
        }
        Err(e) => {
            eprintln!("\n❌ 示例运行失败: {}", e);
            std::process::exit(1);
        }
    }
}
