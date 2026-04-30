//! 软渲染器 - 彩虹图标（系统 emoji 字体渲染）+ 粒子 + 轨迹 + 呼吸光晕

use crate::particles::ParticleSystem;

/// 窗口尺寸常量
pub const WINDOW_WIDTH: u32 = 80;
pub const WINDOW_HEIGHT: u32 = 80;

/// 彩虹图标 RGBA 像素数据
pub struct RainbowIcon {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA
}

/// ============================================================
/// 彩虹图标生成：优先使用系统 emoji 字体渲染 🌈 字符
/// ============================================================

/// 生成彩虹图标
pub fn generate_rainbow_icon() -> RainbowIcon {
    let w = 64u32;
    let h = 64u32;
    let mut pixels = vec![0u8; (w * h * 4) as usize];

    // 尝试从系统 emoji 字体加载 🌈 轮廓
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

    // 回退：程序化生成
    generate_rainbow_procedural(&mut pixels, w, h);
    RainbowIcon { width: w, height: h, pixels }
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
pub fn draw_icon(buffer: &mut [u8], icon: &RainbowIcon) {
    let buf_w = WINDOW_WIDTH as i32;
    let buf_h = WINDOW_HEIGHT as i32;
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
pub fn draw_breathe_glow(buffer: &mut [u8], alpha: f32) {
    let cx = WINDOW_WIDTH as f32 / 2.0;
    let cy = WINDOW_HEIGHT as f32 / 2.0;
    let radius = 35.0;

    for y in 0..WINDOW_HEIGHT {
        for x in 0..WINDOW_WIDTH {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < radius {
                let glow = (1.0 - dist / radius) * alpha;
                if glow < 0.01 {
                    continue;
                }
                let idx = ((y * WINDOW_WIDTH + x) * 4) as usize;
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

/// 绘制拖拽彩虹轨迹
pub fn draw_trail(buffer: &mut [u8], particles: &ParticleSystem) {
    let trail = &particles.trail;
    let len = trail.len();
    if len < 2 {
        return;
    }

    // 从最新到最旧绘制
    for i in 0..len {
        let alpha_pct = ParticleSystem::trail_alpha(i, len);
        let (r, g, b) = ParticleSystem::trail_color(i, len);
        let pt = &trail[i];

        // 轨迹点大小随新旧变化（最近的更大）
        let radius = if particles.is_dragging {
            3.0 + (i as f32 / len as f32) * 4.0
        } else {
            2.0 + (i as f32 / len as f32) * 3.0
        };
        let alpha = (alpha_pct * 200.0) as u8;

        draw_circle(buffer, pt.x, pt.y, radius as i32, r, g, b, alpha);
    }
}

/// 绘制星光粒子
pub fn draw_sparkles(buffer: &mut [u8], particles: &ParticleSystem) {
    for sparkle in &particles.sparkles {
        let (r, g, b) = particles.sparkle_color(sparkle);
        let alpha = (sparkle.life * 220.0) as u8;
        let size = sparkle.size as i32;

        // 绘制四角星
        let x = sparkle.x as i32;
        let y = sparkle.y as i32;

        // 主点
        set_pixel(buffer, x, y, r, g, b, alpha);
        // 4个尖角
        if size >= 2 {
            set_pixel(buffer, x + 1, y, r, g, b, (alpha as f32 * 0.8) as u8);
            set_pixel(buffer, x - 1, y, r, g, b, (alpha as f32 * 0.8) as u8);
            set_pixel(buffer, x, y + 1, r, g, b, (alpha as f32 * 0.8) as u8);
            set_pixel(buffer, x, y - 1, r, g, b, (alpha as f32 * 0.8) as u8);
        }
        if size >= 3 {
            set_pixel(buffer, x + 2, y, r, g, b, (alpha as f32 * 0.4) as u8);
            set_pixel(buffer, x - 2, y, r, g, b, (alpha as f32 * 0.4) as u8);
            set_pixel(buffer, x, y + 2, r, g, b, (alpha as f32 * 0.4) as u8);
            set_pixel(buffer, x, y - 2, r, g, b, (alpha as f32 * 0.4) as u8);
        }
    }
}

/// 设置单个像素（带透明度混合）
fn set_pixel(buffer: &mut [u8], x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
    if x < 0 || x >= WINDOW_WIDTH as i32 || y < 0 || y >= WINDOW_HEIGHT as i32 {
        return;
    }
    if a == 0 {
        return;
    }
    let idx = ((y as u32 * WINDOW_WIDTH + x as u32) * 4) as usize;

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

/// 绘制实心圆（带透明度）
fn draw_circle(buffer: &mut [u8], cx: i32, cy: i32, radius: i32, r: u8, g: u8, b: u8, a: u8) {
    if a == 0 || radius <= 0 {
        return;
    }
    let min_x = (cx - radius).max(0);
    let max_x = (cx + radius).min(WINDOW_WIDTH as i32 - 1);
    let min_y = (cy - radius).max(0);
    let max_y = (cy + radius).min(WINDOW_HEIGHT as i32 - 1);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = (x - cx) as f32;
            let dy = (y - cy) as f32;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= radius as f32 {
                // 边缘柔化
                let edge = (radius as f32 - dist).min(2.0) / 2.0;
                let alpha = ((a as f32) * edge).min(255.0) as u8;
                set_pixel(buffer, x, y, r, g, b, alpha);
            }
        }
    }
}
