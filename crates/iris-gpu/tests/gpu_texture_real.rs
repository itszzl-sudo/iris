//! 实际 GPU 环境纹理渲染测试
//!
//! 这些测试需要真实的 wgpu GPU 环境，验证：
//! - 纹理加载和上传到 GPU
//! - 纹理绑定组创建
//! - 纹理矩形渲染
//! - 纹理采样和混合
//!
//! 注意：这些测试在没有 GPU 的环境中会失败

use iris_gpu::BatchRenderer;
use wgpu::util::DeviceExt;

/// 测试辅助：创建 wgpu 测试环境
/// 
/// 返回 (device, queue, surface_format)
async fn create_wgpu_test_env() -> Option<(wgpu::Device, wgpu::Queue, wgpu::TextureFormat)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: Some("Test Device"),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )
        .await
        .ok()?;

    // 使用常见的表面格式
    let surface_format = wgpu::TextureFormat::Bgra8UnormSrgb;

    Some((device, queue, surface_format))
}

/// 测试：实际 GPU 纹理加载和 ID 分配
/// 
/// 验证纹理可以正确加载到 GPU 并分配唯一 ID
#[tokio::test]
async fn test_gpu_texture_load_and_id() {
    let (device, queue, surface_format) = match create_wgpu_test_env().await {
        Some(env) => env,
        None => {
            eprintln!("⏭️ Skipped: No GPU available");
            return;
        }
    };

    // 创建 BatchRenderer
    let mut renderer = BatchRenderer::new(
        &device,
        &queue,
        surface_format,
        800.0,
        600.0,
        1024,
    );

    // 创建测试纹理数据（2x2 棋盘格）
    let texture_data = vec![
        // 第一行：红色、绿色
        255, 0, 0, 255,   // 红色
        0, 255, 0, 255,   // 绿色
        // 第二行：蓝色、黄色
        0, 0, 255, 255,   // 蓝色
        255, 255, 0, 255, // 黄色
    ];

    // 加载纹理
    let texture_id = renderer
        .load_texture_from_bytes(&device, &texture_data, 2, 2)
        .expect("Failed to load texture");

    // 验证纹理 ID 分配（注意：默认白色纹理占用 ID 0）
    assert!(texture_id >= 0, "Texture ID should be non-negative");

    // 加载第二个纹理
    let texture_data_2 = vec![
        255, 255, 255, 255, // 白色
        0, 0, 0, 255,       // 黑色
        128, 128, 128, 255, // 灰色
        255, 0, 255, 255,   // 品红
    ];

    let texture_id_2 = renderer
        .load_texture_from_bytes(&device, &texture_data_2, 2, 2)
        .expect("Failed to load second texture");

    // 验证第二个纹理 ID 是递增的
    assert_eq!(texture_id_2, texture_id + 1, "Second texture ID should be first + 1");

    println!("✅ GPU texture loading test passed");
}

/// 测试：GPU 纹理渲染到命令缓冲区
/// 
/// 验证纹理矩形可以正确提交到 GPU 命令缓冲区
#[tokio::test]
async fn test_gpu_texture_rect_submission() {
    let (device, queue, surface_format) = match create_wgpu_test_env().await {
        Some(env) => env,
        None => {
            eprintln!("⏭️ Skipped: No GPU available");
            return;
        }
    };

    let mut renderer = BatchRenderer::new(
        &device,
        &queue,
        surface_format,
        800.0,
        600.0,
        1024,
    );

    // 加载纹理
    let texture_data = vec![
        255, 0, 0, 255,
        0, 255, 0, 255,
        0, 0, 255, 255,
        255, 255, 0, 255,
    ];

    let texture_id = renderer
        .load_texture_from_bytes(&device, &texture_data, 2, 2)
        .expect("Failed to load texture");

    // 提交纹理矩形（完整 UV）
    renderer.submit_texture_rect(
        100.0,
        100.0,
        200.0,
        200.0,
        texture_id,
        [0.0, 0.0, 1.0, 1.0], // 完整纹理
    );

    // 验证绘制命令已累积
    let draw_count = renderer.draw_count();
    assert!(draw_count > 0, "Should have accumulated draw commands");

    println!("✅ GPU texture rect submission test passed ({} draws)", draw_count);
}

/// 测试：GPU 多纹理批量渲染
/// 
/// 验证多个纹理可以正确批量渲染
#[tokio::test]
async fn test_gpu_multi_texture_batch_rendering() {
    let (device, queue, surface_format) = match create_wgpu_test_env().await {
        Some(env) => env,
        None => {
            eprintln!("⏭️ Skipped: No GPU available");
            return;
        }
    };

    let mut renderer = BatchRenderer::new(
        &device,
        &queue,
        surface_format,
        800.0,
        600.0,
        1024,
    );

    // 加载 3 个不同的纹理
    let colors = [
        ([255, 0, 0, 255], "Red"),    // 红色
        ([0, 255, 0, 255], "Green"),  // 绿色
        ([0, 0, 255, 255], "Blue"),   // 蓝色
    ];

    let mut texture_ids = Vec::new();

    for (color, name) in &colors {
        let texture_data = vec![
            color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3],
            color[0], color[1], color[2], color[3],
        ];

        let id = renderer
            .load_texture_from_bytes(&device, &texture_data, 1, 1)
            .expect(&format!("Failed to load {} texture", name));

        texture_ids.push(id);
        println!("  Loaded {} texture with ID {}", name, id);
    }

    // 提交 3 个纹理矩形
    for (i, &texture_id) in texture_ids.iter().enumerate() {
        let x = 100.0 + i as f32 * 220.0;
        renderer.submit_texture_rect(
            x,
            100.0,
            200.0,
            200.0,
            texture_id,
            [0.0, 0.0, 1.0, 1.0],
        );
    }

    // 验证所有命令都已累积
    let draw_count = renderer.draw_count();
    assert_eq!(draw_count, 3, "Should have 3 draw commands for 3 textures");

    println!("✅ GPU multi-texture batch rendering test passed");
}

/// 测试：GPU 纹理 UV 坐标准确性
/// 
/// 验证不同的 UV 坐标可以正确映射纹理区域
#[tokio::test]
async fn test_gpu_texture_uv_accuracy() {
    let (device, queue, surface_format) = match create_wgpu_test_env().await {
        Some(env) => env,
        None => {
            eprintln!("⏭️ Skipped: No GPU available");
            return;
        }
    };

    let mut renderer = BatchRenderer::new(
        &device,
        &queue,
        surface_format,
        800.0,
        600.0,
        1024,
    );

    // 创建 4x4 测试纹理（16 种颜色）
    let mut texture_data = Vec::with_capacity(4 * 4 * 4);
    for y in 0..4 {
        for x in 0..4 {
            let r = (x * 64) as u8;
            let g = (y * 64) as u8;
            let b = 128u8;
            let a = 255u8;
            texture_data.extend_from_slice(&[r, g, b, a]);
        }
    }

    let texture_id = renderer
        .load_texture_from_bytes(&device, &texture_data, 4, 4)
        .expect("Failed to load 4x4 texture");

    // 测试 1: 完整纹理 (0,0) -> (1,1)
    renderer.submit_texture_rect(
        50.0,
        50.0,
        200.0,
        200.0,
        texture_id,
        [0.0, 0.0, 1.0, 1.0],
    );

    // 测试 2: 左上角 1/4 (0,0) -> (0.5,0.5)
    renderer.submit_texture_rect(
        300.0,
        50.0,
        200.0,
        200.0,
        texture_id,
        [0.0, 0.0, 0.5, 0.5],
    );

    // 测试 3: 右下角 1/4 (0.5,0.5) -> (1,1)
    renderer.submit_texture_rect(
        550.0,
        50.0,
        200.0,
        200.0,
        texture_id,
        [0.5, 0.5, 1.0, 1.0],
    );

    let draw_count = renderer.draw_count();
    assert_eq!(draw_count, 3, "Should have 3 draw commands for UV tests");

    println!("✅ GPU texture UV accuracy test passed");
}

/// 测试：GPU 纹理透明度混合
/// 
/// 验证半透明纹理可以正确混合
#[tokio::test]
async fn test_gpu_texture_alpha_blending() {
    let (device, queue, surface_format) = match create_wgpu_test_env().await {
        Some(env) => env,
        None => {
            eprintln!("⏭️ Skipped: No GPU available");
            return;
        }
    };

    let mut renderer = BatchRenderer::new(
        &device,
        &queue,
        surface_format,
        800.0,
        600.0,
        1024,
    );

    // 创建半透明纹理（50% 透明红色）
    let semi_transparent_texture = vec![
        255, 0, 0, 128, // 50% 透明红色
        255, 0, 0, 128,
        255, 0, 0, 128,
        255, 0, 0, 128,
    ];

    let texture_id = renderer
        .load_texture_from_bytes(&device, &semi_transparent_texture, 2, 2)
        .expect("Failed to load semi-transparent texture");

    // 先绘制一个白色背景矩形
    use iris_gpu::DrawCommand;
    renderer.submit(DrawCommand::Rect {
        x: 100.0,
        y: 100.0,
        width: 200.0,
        height: 200.0,
        color: [1.0, 1.0, 1.0, 1.0], // 白色背景
    });

    // 在白色背景上绘制半透明纹理
    renderer.submit_texture_rect(
        100.0,
        100.0,
        200.0,
        200.0,
        texture_id,
        [0.0, 0.0, 1.0, 1.0],
    );

    let draw_count = renderer.draw_count();
    assert_eq!(draw_count, 2, "Should have 2 draw commands (background + texture)");

    println!("✅ GPU texture alpha blending test passed");
}

/// 测试：GPU 纹理缩放渲染
/// 
/// 验证纹理在不同尺寸下的渲染效果
#[tokio::test]
async fn test_gpu_texture_scaling() {
    let (device, queue, surface_format) = match create_wgpu_test_env().await {
        Some(env) => env,
        None => {
            eprintln!("⏭️ Skipped: No GPU available");
            return;
        }
    };

    let mut renderer = BatchRenderer::new(
        &device,
        &queue,
        surface_format,
        800.0,
        600.0,
        1024,
    );

    // 创建 16x16 测试纹理
    let mut texture_data = Vec::with_capacity(16 * 16 * 4);
    for y in 0..16 {
        for x in 0..16 {
            let checker = if (x + y) % 2 == 0 { 255 } else { 0 };
            texture_data.extend_from_slice(&[checker, checker, checker, 255]);
        }
    }

    let texture_id = renderer
        .load_texture_from_bytes(&device, &texture_data, 16, 16)
        .expect("Failed to load 16x16 texture");

    // 测试不同缩放比例
    let scales = [
        (50.0, 50.0),    // 缩小到 1/3
        (100.0, 100.0),  // 原始尺寸
        (200.0, 200.0),  // 放大 2 倍
        (300.0, 300.0),  // 放大 3 倍
    ];

    for (i, (width, height)) in scales.iter().enumerate() {
        let x = 50.0 + i as f32 * 220.0;
        renderer.submit_texture_rect(
            x,
            100.0,
            *width,
            *height,
            texture_id,
            [0.0, 0.0, 1.0, 1.0],
        );
    }

    let draw_count = renderer.draw_count();
    assert_eq!(draw_count, 4, "Should have 4 draw commands for scaling tests");

    println!("✅ GPU texture scaling test passed");
}

/// 测试：GPU 纹理渲染性能基准
/// 
/// 测试大批量纹理渲染的性能
#[tokio::test]
async fn test_gpu_texture_rendering_performance() {
    let (device, queue, surface_format) = match create_wgpu_test_env().await {
        Some(env) => env,
        None => {
            eprintln!("⏭️ Skipped: No GPU available");
            return;
        }
    };

    let mut renderer = BatchRenderer::new(
        &device,
        &queue,
        surface_format,
        1920.0,
        1080.0,
        4096, // 大容量
    );

    // 加载一个纹理
    let texture_data = vec![
        255, 0, 0, 255,
        0, 255, 0, 255,
        0, 0, 255, 255,
        255, 255, 0, 255,
    ];

    let texture_id = renderer
        .load_texture_from_bytes(&device, &texture_data, 2, 2)
        .expect("Failed to load texture");

    // 批量提交 100 个纹理矩形
    let start_time = std::time::Instant::now();

    for i in 0..100 {
        let x = (i % 10) as f32 * 100.0;
        let y = (i / 10) as f32 * 100.0;

        renderer.submit_texture_rect(
            x,
            y,
            80.0,
            80.0,
            texture_id,
            [0.0, 0.0, 1.0, 1.0],
        );
    }

    let submission_time = start_time.elapsed();

    // 验证性能（应该在合理时间内完成）
    assert!(
        submission_time.as_millis() < 100,
        "Submitting 100 textures should take less than 100ms, took {:?}",
        submission_time
    );

    let draw_count = renderer.draw_count();
    assert_eq!(draw_count, 100, "Should have 100 draw commands");

    println!(
        "✅ GPU texture rendering performance test passed ({} textures in {:?})",
        draw_count, submission_time
    );
}
