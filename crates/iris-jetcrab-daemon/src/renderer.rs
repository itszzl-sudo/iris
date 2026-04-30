//! 软渲染器 - 彩虹图标（系统 emoji 字体渲染）+ 粒子 + 轨迹 + 呼吸光晕

use crate::particles::ParticleSystem;
use std::sync::OnceLock;

/// 彩虹图标 RGBA 像素数据
pub struct RainbowIcon {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA
}

/// 全局缓存：只渲染一次，后续复用
static RAINBOW_ICON: OnceLock<RainbowIcon> = OnceLock::new();

/// ============================================================
/// 彩虹图标生成：优先加载 iris.png，回退到 emoji 字体渲染
/// ============================================================

/// 生成彩虹图标（缓存为全局静态，仅首次调用时实际生成）
pub fn generate_rainbow_icon() -> &'static RainbowIcon {
    RAINBOW_ICON.get_or_init(|| {
        // 优先尝试加载 iris.png
        if let Some(icon) = load_png_icon() {
            return icon;
        }

        let w = 64u32;
        let h = 64u32;
        let mut pixels = vec![0u8; (w * h * 4) as usize];

        // 回退：尝试从系统 emoji 字体加载 🌈 轮廓
        if let Some(font_data) = load_emoji_font_data() {
            let font_data = extract_from_ttc_if_needed(&font_data);
            if let Some(fd) = font_data {
                if let Ok(font) = fontdue::Font::from_bytes(fd, fontdue::FontSettings::default()) {
                    if render_emoji_glyph(&mut pixels, w, h, &font) {
                        return RainbowIcon { width: w, height: h, pixels };
                    }
                }
            }
        }

        // 最终回退：程序化生成
        generate_rainbow_procedural(&mut pixels, w, h);
        RainbowIcon { width: w, height: h, pixels }
    })
}

// ── PNG 图标加载 ────────────────────────────────────────────

/// 从嵌入的资源解码 iris.png（预压缩为 64x64）
fn load_png_icon() -> Option<RainbowIcon> {
    // include_bytes! 在编译时嵌入资源文件
    let png_data = include_bytes!("../res/iris.png");

    match image::load_from_memory(png_data) {
        Ok(img) => {
            let rgba = img.into_rgba8();
            let pixels = rgba.into_raw();
            tracing::info!(
                "Loaded embedded iris.png ({} bytes), decoded to 64x64 RGBA",
                png_data.len()
            );
            Some(RainbowIcon {
                width: 64,
                height: 64,
                pixels,
            })
        }
        Err(e) => {
            tracing::error!("Failed to decode embedded iris.png: {}", e);
            None
        }
    }
}

// ── 跨平台 emoji 字体加载 ──────────────────────────────────

/// 加载系统 emoji 字体的原始字节
fn load_emoji_font_data() -> Option<Vec<u8>> {
    // 按平台候选路径
    let candidates: &[&str] = if cfg!(target_os = "windows") {
        &[
            "C:\\Windows\\Fonts\\seguiemj.ttf",
            "C:\\Windows\\Fonts\\Segoe UI Emoji\\seguiemj.ttf",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/Apple Color Emoji.ttc",
            "/System/Library/Fonts/Apple Color Emoji - Alternate.ttc",
        ]
    } else {
        // Linux
        &[
            "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
            "/usr/share/fonts/noto/NotoColorEmoji.ttf",
            "/usr/share/fonts/NotoColorEmoji.ttf",
            "/usr/share/fonts/google-noto-emoji/NotoColorEmoji.ttf",
        ]
    };

    for path in candidates {
        if let Ok(data) = std::fs::read(path) {
            return Some(data);
        }
    }
    None
}

/// 如果是 TTC (TrueType Collection)，提取第一个字体
fn extract_from_ttc_if_needed(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 12 || &data[0..4] != b"ttcf" {
        return Some(data.to_vec()); // 已经是 TTF
    }

    let num_fonts = read_u32_be(data, 8);
    if num_fonts == 0 {
        return None;
    }
    let offset = read_u32_be(data, 12) as usize;
    let end = if num_fonts > 1 {
        read_u32_be(data, 16) as usize
    } else {
        data.len()
    };
    Some(data[offset..end].to_vec())
}

fn read_u32_be(data: &[u8], pos: usize) -> u32 {
    if pos + 4 > data.len() {
        return 0;
    }
    u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
}

// ── 栅格化并着色 ────────────────────────────────────────────

/// 使用 fontdue 栅格化 🌈 字符，并用彩虹色着色
/// 返回 true 表示渲染成功
fn render_emoji_glyph(pixels: &mut [u8], w: u32, h: u32, font: &fontdue::Font) -> bool {
    let ch: char = '\u{1F308}'; // 🌈
    let char_index = font.lookup_glyph_index(ch);
    if char_index == 0 {
        return false;
    }

    // 在 64px 中渲染 48px 大小
    let px_size = 48.0;
    let (metrics, bitmap) = font.rasterize(ch, px_size);

    if metrics.width == 0 || metrics.height == 0 {
        return false;
    }

    let gw = metrics.width as i32;
    let gh = metrics.height as i32;

    // 居中位置
    let offset_x = (w as i32 - gw) / 2 + metrics.xmin as i32;
    let offset_y = (h as i32 - gh) / 2 + metrics.ymin as i32;

    // 找 glyph 的垂直边界（排除噪声像素）
    let mut top = f32::MAX;
    let mut bottom = f32::MIN;
    for py in 0..gh {
        for px in 0..gw {
            if bitmap[(py * gw + px) as usize] > 10 {
                let py_f = py as f32 + metrics.ymin as f32;
                top = top.min(py_f);
                bottom = bottom.max(py_f);
            }
        }
    }
    if bottom <= top {
        return false;
    }
    let total_height = bottom - top;
    // 顶部 ~55% 为彩虹弧，底部 ~45% 为云朵
    let rainbow_split = 0.55;

    // 着色：按 Y 位置分配色彩
    for py in 0..gh {
        for px in 0..gw {
            let coverage = bitmap[(py * gw + px) as usize];
            if coverage == 0 {
                continue;
            }

            let bx = offset_x + px;
            let by = offset_y + py;
            if bx < 0 || bx >= w as i32 || by < 0 || by >= h as i32 {
                continue;
            }

            let idx = ((by as u32 * w + bx as u32) * 4) as usize;
            let py_f = py as f32 + metrics.ymin as f32;
            let ny = (py_f - top) / total_height; // 0=顶部, 1=底部

            if ny < rainbow_split {
                // 彩虹弧区域：按 Y 位置分配色带（顶部=红 → 底部=紫）
                let band = ((ny / rainbow_split) * 5.5).floor() as usize;
                let band = band.min(5);
                let (r, g, b) = RAINBOW_BANDS[band];
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                pixels[idx + 3] = coverage;
            } else {
                // 云朵区域 — 暖白色
                pixels[idx] = 255;
                pixels[idx + 1] = 252;
                pixels[idx + 2] = 245;
                pixels[idx + 3] = coverage;
            }
        }
    }

    true
}

// ── 程序化回退 ──────────────────────────────────────────────

/// 程序化生成彩虹图标（回退方案）
fn generate_rainbow_procedural(pixels: &mut [u8], w: u32, h: u32) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 * 0.50;

    // Pass 1: 彩虹弧
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            let dx = x as f32 - cx;
            let dy = (y as f32 - cy) * 0.82;
            let dist = (dx * dx + dy * dy).sqrt();

            let angle = dy.atan2(dx);
            let angle_span = std::f32::consts::PI * 0.7;
            let in_arc = angle.abs() < angle_span;

            let inner_r = 7.0;
            let outer_r = 25.0;
            let band_count = 6;
            let band_width = (outer_r - inner_r) / band_count as f32;

            if in_arc && dist >= inner_r && dist <= outer_r {
                let band = ((dist - inner_r) / band_width) as usize;
                let band = band.min(band_count - 1);
                let (r, g, b) = RAINBOW_BANDS[band];

                let edge_dist = (dist - inner_r).min(outer_r - dist).min(band_width * 0.45);
                let alpha = (edge_dist / (band_width * 0.45) * 255.0).min(255.0) as u8;

                let end_fade = 1.0 - ((angle.abs() - angle_span * 0.7) / (angle_span * 0.3)).max(0.0);
                let end_alpha = (alpha as f32 * end_fade) as u8;

                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                pixels[idx + 3] = end_alpha;
            }
        }
    }

    // Pass 2: 云朵
    let cloud_centers = [(cx * 0.48, cy * 1.38), (cx * 0.62, cy * 1.40), (cx * 0.75, cy * 1.32)];
    let cloud_radii = [9.0, 10.0, 8.0];
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            let mut max_cloud_a: u8 = 0;
            let mut cloud_r = 255u8;
            let mut cloud_g = 255u8;
            let mut cloud_b = 255u8;

            for (i, (ccx, ccy)) in cloud_centers.iter().enumerate() {
                let dx = x as f32 - ccx;
                let dy = (y as f32 - ccy) * 1.3;
                let d = (dx * dx + dy * dy).sqrt();
                let r = cloud_radii[i];
                if d < r {
                    let a = ((1.0 - d / r) * 220.0) as u8;
                    if a > max_cloud_a {
                        max_cloud_a = a;
                        cloud_r = 255;
                        cloud_g = 252;
                        cloud_b = 245;
                    }
                }
            }

            if max_cloud_a > pixels[idx + 3] {
                pixels[idx] = cloud_r;
                pixels[idx + 1] = cloud_g;
                pixels[idx + 2] = cloud_b;
                pixels[idx + 3] = max_cloud_a;
            }
        }
    }
}

/// 彩虹六色带
const RAINBOW_BANDS: [(u8, u8, u8); 6] = [
    (230, 30, 30),    // 红
    (255, 155, 0),    // 橙
    (255, 230, 0),    // 黄
    (0, 190, 0),      // 绿
    (0, 120, 255),    // 蓝
    (150, 40, 200),   // 紫
];

/// 在像素缓冲区中心绘制彩虹图标
pub fn draw_icon(buffer: &mut [u8], w: u32, h: u32, icon: &RainbowIcon) {
    let buf_w = w as i32;
    let buf_h = h as i32;
    let icon_w = icon.width as i32;
    let icon_h = icon.height as i32;

    // 居中
    let offset_x = (buf_w - icon_w) / 2;
    let offset_y = (buf_h - icon_h) / 2 + 2; // 略偏上，给光晕留空间

    for iy in 0..icon_h {
        for ix in 0..icon_w {
            let bx = offset_x + ix;
            let by = offset_y + iy;
            if bx < 0 || bx >= buf_w || by < 0 || by >= buf_h {
                continue;
            }
            let src_idx = ((iy * icon_w + ix) * 4) as usize;
            let dst_idx = ((by * buf_w + bx) * 4) as usize;

            let sa = icon.pixels[src_idx + 3] as f32 / 255.0;
            if sa <= 0.0 {
                continue;
            }

            // Alpha blend
            let da = buffer[dst_idx + 3] as f32 / 255.0;
            let out_a = sa + da * (1.0 - sa);
            if out_a <= 0.0 {
                continue;
            }

            for c in 0..3 {
                let src_c = icon.pixels[src_idx + c] as f32;
                let dst_c = buffer[dst_idx + c] as f32;
                buffer[dst_idx + c] = ((src_c * sa + dst_c * da * (1.0 - sa)) / out_a) as u8;
            }
            buffer[dst_idx + 3] = (out_a * 255.0) as u8;
        }
    }
}

/// 绘制呼吸光晕
pub fn draw_breathe_glow(buffer: &mut [u8], w: u32, h: u32, alpha: f32) {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let radius = (w.min(h) as f32 / 2.0) * 0.85;

    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < radius {
                let glow = (1.0 - dist / radius) * alpha;
                if glow < 0.01 {
                    continue;
                }
                let idx = ((y * w + x) * 4) as usize;
                let glow_alpha = (glow * 255.0) as u8;

                // 渐变色光晕（柔和的彩虹渐变）
                let angle = dy.atan2(dx);
                let norm_angle = (angle / std::f32::consts::TAU + 0.5).fract();
                let band = ((norm_angle * 5.0) as usize).min(5);
                let (r, g, b) = RAINBOW_BANDS[band];

                // Alpha blend with existing
                let sa = glow_alpha as f32 / 255.0;
                let da = buffer[idx + 3] as f32 / 255.0;
                let out_a = sa + da * (1.0 - sa);
                if out_a <= 0.0 {
                    continue;
                }

                buffer[idx] = ((r as f32 * sa + buffer[idx] as f32 * da * (1.0 - sa)) / out_a) as u8;
                buffer[idx + 1] = ((g as f32 * sa + buffer[idx + 1] as f32 * da * (1.0 - sa)) / out_a) as u8;
                buffer[idx + 2] = ((b as f32 * sa + buffer[idx + 2] as f32 * da * (1.0 - sa)) / out_a) as u8;
                buffer[idx + 3] = (out_a * 255.0) as u8;
            }
        }
    }
}

/// 绘制拖拽彩虹轨迹（心形）
pub fn draw_trail(buffer: &mut [u8], w: u32, h: u32, particles: &ParticleSystem) {
    let trail = &particles.trail;
    let len = trail.len();
    if len < 2 {
        return;
    }

    // 从最新到最旧绘制心形
    for i in 0..len {
        let alpha_pct = ParticleSystem::trail_alpha(i, len);
        let (r, g, b) = ParticleSystem::trail_color(i, len);
        let pt = &trail[i];

        let _scale = (w as f32 / 80.0).max(1.0);
        // 心形大小随新旧变化（最近的更大）
        let size = if particles.is_dragging {
            (6.0 + (i as f32 / len as f32 * 14.0)) as i32
        } else {
            (3.0 + (i as f32 / len as f32 * 8.0)) as i32
        };
        let alpha = (alpha_pct * 220.0) as u8;

        draw_heart(buffer, w, h, pt.x, pt.y, size, r, g, b, alpha);
    }
}

/// 绘制星光粒子
pub fn draw_sparkles(buffer: &mut [u8], w: u32, h: u32, particles: &ParticleSystem) {
    for sparkle in &particles.sparkles {
        let (r, g, b) = particles.sparkle_color(sparkle);
        let alpha = (sparkle.life * 220.0) as u8;
        let size = sparkle.size as i32;

        // 绘制四角星
        let x = sparkle.x as i32;
        let y = sparkle.y as i32;

        // 主点
        set_pixel(buffer, w, h, x, y, r, g, b, alpha);
        // 4个尖角
        if size >= 2 {
            set_pixel(buffer, w, h, x + 1, y, r, g, b, (alpha as f32 * 0.8) as u8);
            set_pixel(buffer, w, h, x - 1, y, r, g, b, (alpha as f32 * 0.8) as u8);
            set_pixel(buffer, w, h, x, y + 1, r, g, b, (alpha as f32 * 0.8) as u8);
            set_pixel(buffer, w, h, x, y - 1, r, g, b, (alpha as f32 * 0.8) as u8);
        }
        if size >= 3 {
            set_pixel(buffer, w, h, x + 2, y, r, g, b, (alpha as f32 * 0.4) as u8);
            set_pixel(buffer, w, h, x - 2, y, r, g, b, (alpha as f32 * 0.4) as u8);
            set_pixel(buffer, w, h, x, y + 2, r, g, b, (alpha as f32 * 0.4) as u8);
            set_pixel(buffer, w, h, x, y - 2, r, g, b, (alpha as f32 * 0.4) as u8);
        }
    }
}

/// 设置单个像素（带透明度混合）
fn set_pixel(buffer: &mut [u8], w: u32, h: u32, x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
    if x < 0 || x >= w as i32 || y < 0 || y >= h as i32 {
        return;
    }
    if a == 0 {
        return;
    }
    let idx = ((y as u32 * w + x as u32) * 4) as usize;

    let sa = a as f32 / 255.0;
    let da = buffer[idx + 3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0 {
        return;
    }

    buffer[idx] = ((r as f32 * sa + buffer[idx] as f32 * da * (1.0 - sa)) / out_a) as u8;
    buffer[idx + 1] = ((g as f32 * sa + buffer[idx + 1] as f32 * da * (1.0 - sa)) / out_a) as u8;
    buffer[idx + 2] = ((b as f32 * sa + buffer[idx + 2] as f32 * da * (1.0 - sa)) / out_a) as u8;
    buffer[idx + 3] = (out_a * 255.0) as u8;
}

/// 绘制心形（带透明度）- 使用心形隐式方程 (x²+y²-1)³ - x²y³ ≤ 0
fn draw_heart(buffer: &mut [u8], w: u32, h: u32, cx: i32, cy: i32, size: i32, r: u8, g: u8, b: u8, a: u8) {
    if a == 0 || size <= 0 {
        return;
    }
    let min_x = (cx - size).max(0);
    let max_x = (cx + size).min(w as i32 - 1);
    let min_y = (cy - size).max(0);
    let max_y = (cy + size).min(h as i32 - 1);

    // 心形在归一化空间 [-1.3, 1.3] × [-1.3, 1.2]
    let scale = size as f32 / 1.3;

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let dx = (px - cx) as f32;
            let dy = (py - cy) as f32;
            // 归一化到心形方程空间
            let nx = dx / scale;
            let ny = -dy / scale; // 翻转 Y 使心尖朝下
            let nx2 = nx * nx;
            let ny2 = ny * ny;
            // 心形隐式方程: (x² + y² - 1)³ - x²y³ ≤ 0
            let val = (nx2 + ny2 - 1.0).powi(3) - nx2 * ny.powi(3);
            if val <= 0.0 {
                // 边缘柔化
                let edge = (0.0f32.max(-val) / 0.08).min(1.0);
                let alpha = ((a as f32) * edge).min(255.0) as u8;
                set_pixel(buffer, w, h, px, py, r, g, b, alpha);
            }
        }
    }
}

/// 绘制悬停提示文字（使用 fontdue 渲染文本）
pub fn draw_tooltip(buffer: &mut [u8], w: u32, h: u32, text: &str, x: i32, y: i32) {
    let font_data = match load_system_ui_font() {
        Some(d) => d,
        None => return,
    };
    let font = match fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default()) {
        Ok(f) => f,
        Err(_) => return,
    };

    let font_size = 14.0;
    let mut total_width = 0i32;
    let mut glyph_metrics = Vec::new();
    let mut glyph_bitmaps = Vec::new();

    for ch in text.chars() {
        let char_index = font.lookup_glyph_index(ch);
        if char_index == 0 { continue; }
        let (metrics, bitmap) = font.rasterize(ch, font_size);
        glyph_metrics.push((metrics, total_width));
        glyph_bitmaps.push(bitmap);
        total_width += metrics.width as i32 + metrics.xmin as i32 + 1;
    }

    if glyph_metrics.is_empty() { return; }

    let start_x = x - total_width / 2;
    let line_height = font_size as i32 + 2;
    let start_y = y - line_height / 2;

    for (i, (metrics, x_offset)) in glyph_metrics.iter().enumerate() {
        let gw = metrics.width as i32;
        let gh = metrics.height as i32;
        if gw <= 0 || gh <= 0 { continue; }

        for py in 0..gh {
            for px in 0..gw {
                let coverage = glyph_bitmaps[i][(py * gw + px) as usize];
                if coverage == 0 { continue; }
                let bx = start_x + x_offset + px;
                let by = start_y + py;
                if bx < 0 || bx >= w as i32 || by < 0 || by >= h as i32 { continue; }
                let idx = ((by as u32 * w + bx as u32) * 4) as usize;
                let sa = coverage as f32 / 255.0;
                let da = buffer[idx + 3] as f32 / 255.0;
                let out_a = sa + da * (1.0 - sa);
                if out_a <= 0.0 { continue; }
                buffer[idx] = ((255.0 * sa + buffer[idx] as f32 * da * (1.0 - sa)) / out_a) as u8;
                buffer[idx + 1] = ((255.0 * sa + buffer[idx + 1] as f32 * da * (1.0 - sa)) / out_a) as u8;
                buffer[idx + 2] = ((255.0 * sa + buffer[idx + 2] as f32 * da * (1.0 - sa)) / out_a) as u8;
                buffer[idx + 3] = (out_a * 255.0) as u8;
            }
        }
    }
}

/// 加载系统 UI 字体
fn load_system_ui_font() -> Option<Vec<u8>> {
    let candidates: &[&str] = if cfg!(target_os = "windows") {
        &[
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\Segoe UI\\segoeui.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
            "C:\\Windows\\Fonts\\msyh.ttc",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/SFNS.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
        ]
    } else {
        &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        ]
    };
    for path in candidates {
        if let Ok(data) = std::fs::read(path) {
            if path.ends_with(".ttc") {
                return extract_from_ttc_if_needed(&data);
            }
            return Some(data);
        }
    }
    None
}

/// 绘制下载进度条
pub fn draw_progress_bar(buffer: &mut [u8], w: u32, h: u32,
    x: i32, y: i32, width: u32, bar_height: u32,
    percentage: f64, r: u8, g: u8, b: u8) {
    if percentage <= 0.0 { return; }
    let fill_w = ((width as f64 * percentage / 100.0) as u32).max(1).min(width);
    for py in y..(y + bar_height as i32) {
        for px in x..(x + width as i32) {
            if px < 0 || px >= w as i32 || py < 0 || py >= h as i32 { continue; }
            let idx = ((py as u32 * w + px as u32) * 4) as usize;
            let bg_a = 80u8;
            let da = buffer[idx + 3] as f32 / 255.0;
            let sa = bg_a as f32 / 255.0;
            let out_a = sa + da * (1.0 - sa);
            if out_a <= 0.0 { continue; }
            buffer[idx] = ((50.0 * sa + buffer[idx] as f32 * da * (1.0 - sa)) / out_a) as u8;
            buffer[idx + 1] = ((50.0 * sa + buffer[idx + 1] as f32 * da * (1.0 - sa)) / out_a) as u8;
            buffer[idx + 2] = ((50.0 * sa + buffer[idx + 2] as f32 * da * (1.0 - sa)) / out_a) as u8;
            buffer[idx + 3] = (out_a * 255.0) as u8;
        }
    }
    for py in y..(y + bar_height as i32) {
        for px in x..(x + fill_w as i32) {
            if px < 0 || px >= w as i32 || py < 0 || py >= h as i32 { continue; }
            let idx = ((py as u32 * w + px as u32) * 4) as usize;
            let sa = 200u8;
            let da = buffer[idx + 3] as f32 / 255.0;
            let out_a = sa as f32 / 255.0 + da * (1.0 - sa as f32 / 255.0);
            if out_a <= 0.0 { continue; }
            buffer[idx] = ((r as f32 * sa as f32 / 255.0 + buffer[idx] as f32 * da * (1.0 - sa as f32 / 255.0)) / out_a) as u8;
            buffer[idx + 1] = ((g as f32 * sa as f32 / 255.0 + buffer[idx + 1] as f32 * da * (1.0 - sa as f32 / 255.0)) / out_a) as u8;
            buffer[idx + 2] = ((b as f32 * sa as f32 / 255.0 + buffer[idx + 2] as f32 * da * (1.0 - sa as f32 / 255.0)) / out_a) as u8;
            buffer[idx + 3] = (out_a * 255.0) as u8;
        }
    }
}
