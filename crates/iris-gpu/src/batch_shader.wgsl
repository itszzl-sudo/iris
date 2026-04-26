// Iris 批渲染 Shader
// 支持颜色插值（渐变）、Alpha 混合和纹理采样

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

// 纹理和采样器绑定（用于纹理渲染）
@group(0) @binding(0)
var texture: texture_2d<f32>;

@group(0) @binding(1)
var tex_sampler: sampler;

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(position, 0.0, 1.0);
    output.color = color;
    output.uv = uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // GPU 自动插值颜色，实现渐变效果
    // 如果有纹理，采样纹理并与颜色混合
    let tex_color = textureSample(texture, tex_sampler, input.uv);
    
    // 混合颜色（颜色 * 纹理颜色）
    return input.color * tex_color;
}
