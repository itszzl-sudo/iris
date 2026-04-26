# 纹理渲染管线集成指南

## 当前状态

✅ **已完成**:
- WGSL Shader 支持纹理采样
- UV 坐标传递
- 纹理和采样器绑定声明
- 颜色与纹理混合

⏳ **待完成**:
- 默认纹理创建和绑定
- 渲染管线布局更新
- flush() 中绑定纹理

## 集成步骤

### 1. 更新渲染管线布局

在 `BatchRenderer::new()` 中：

```rust
// 创建纹理绑定组布局（已有）
let texture_bind_group_layout = device.create_bind_group_layout(...);

// 更新管线布局使用纹理绑定
let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    bind_group_layouts: &[&texture_bind_group_layout],  // 改这里
    ..
});
```

### 2. 创建默认纹理

在 `Self { ... }` 之后：

```rust
let mut renderer = Self { ... };

// 创建 1x1 白色纹理
let white_pixel = [255u8, 255, 255, 255];
let default_texture = device.create_texture(&wgpu::TextureDescriptor {
    size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
    format: wgpu::TextureFormat::Rgba8UnormSrgb,
    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    ..Default::default()
});

renderer.queue.write_texture(
    wgpu::TexelCopyTextureInfo {
        texture: &default_texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
    },
    &white_pixel,
    wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(4),
        rows_per_image: Some(1),
    },
    wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
);

let default_view = default_texture.create_view(&wgpu::TextureViewDescriptor::default());
renderer.textures.push(default_texture);
renderer.texture_views.push(default_view);
```

### 3. 创建默认绑定组

```rust
renderer.texture_bind_group = Some(
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &renderer.texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&renderer.texture_views[0]),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&renderer.texture_sampler),
            },
        ],
        ..Default::default()
    })
);

renderer  // 返回 renderer
```

### 4. 在 flush() 中绑定纹理

在 `flush()` 方法的 render pass 中：

```rust
// 在 draw_indexed 之前
if let Some(bind_group) = &self.texture_bind_group {
    render_pass.set_bind_group(0, bind_group, &[]);
}

render_pass.set_index_buffer(...);
render_pass.set_vertex_buffer(...);
render_pass.draw_indexed(...);
```

### 5. 实现纹理矩形渲染

在 `submit_texture_rect()` 中创建真正的纹理矩形而不是占位符：

```rust
pub fn submit_texture_rect(&mut self, x: f32, y: f32, width: f32, height: f32, texture_id: u32, uv: [f32; 4]) {
    let (x1, y1) = self.to_ndc(x, y);
    let (x2, y2) = self.to_ndc(x + width, y + height);
    
    let base_index = self.vertices.len() as u16;
    
    // 4 个顶点，带 UV 坐标
    self.vertices.push(BatchVertex { position: [x1, y1], color: [1.0, 1.0, 1.0, 1.0], uv: [uv[0], uv[1]] });
    self.vertices.push(BatchVertex { position: [x2, y1], color: [1.0, 1.0, 1.0, 1.0], uv: [uv[2], uv[1]] });
    self.vertices.push(BatchVertex { position: [x2, y2], color: [1.0, 1.0, 1.0, 1.0], uv: [uv[2], uv[3]] });
    self.vertices.push(BatchVertex { position: [x1, y2], color: [1.0, 1.0, 1.0, 1.0], uv: [uv[0], uv[3]] });
    
    // 6 个索引
    self.indices.push(base_index);
    self.indices.push(base_index + 1);
    self.indices.push(base_index + 2);
    self.indices.push(base_index);
    self.indices.push(base_index + 2);
    self.indices.push(base_index + 3);
}
```

## 注意事项

1. **作用域问题**: 必须在 `Self { ... }` 之后使用 `let mut renderer = ...` 而不是直接在 `Self { ... }` 块内操作
2. **纹理格式**: 使用 `Rgba8UnormSrgb` 确保颜色空间正确
3. **绑定组**: 必须在渲染前设置，使用 `set_bind_group(0, ...)`
4. **UV 坐标**: 范围 0.0-1.0，(0,0) 是左上角

## 测试验证

创建集成测试验证：
- 纹理加载
- 纹理渲染
- UV 坐标准确性
- 透明度混合

## 参考

- Shader: `crates/iris-gpu/src/batch_shader.wgsl`
- 渲染器: `crates/iris-gpu/src/batch_renderer.rs`
- WebGPU 文档: https://gpuweb.github.io/gpuweb/
