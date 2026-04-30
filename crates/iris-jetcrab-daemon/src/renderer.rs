//! 软渲染器 - 彩虹图标 + 粒子 + 轨迹 + 呼吸光晕

use crate::particles::ParticleSystem;

/// 窗口尺寸常量
pub const WINDOW_WIDTH: u32 = 80;
pub const WINDOW_HEIGHT: u32 = 80;

/// 彩虹图标 RGBA 像素数据（程序化生成）
pub struct RainbowIcon {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA
}

/// 生成彩虹图标像素数据
pub fn generate_rainbow_icon() -> RainbowIcon {
    let w = 64u32;
    let h = 64u32;
    let mut pixels = vec![0u8; (w * h * 4) as usize];

    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            let cx = w as f32 / 2.0;
            let cy = h as f32 / 2.0;
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let _angle = dy.atan2(dx);

            // 彩虹半圆环
            let inner_r = 6.0;
            let outer_r = 26.0;
            let band_count = 7;
            let band_width = (outer_r - inner_r) / band_count as f32;

            if dy < 0.0 && dist >= inner_r && dist <= outer_r {
                // 计算所在色带
                let band = ((dist - inner_r) / band_width) as usize;
                let band = band.min(band_count - 1);
                let (r, g, b) = RAINBOW_BANDS[band];
                // 边缘柔化
                let edge_dist = (dist - inner_r).min(outer_r - dist).min(band_width / 2.0);
                let alpha = (edge_dist * 255.0 / (band_width / 2.0)).min(255.0) as u8;

                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                pixels[idx + 3] = alpha;
            } else if dist < 12.0 && dy > -8.0 {
                // 云朵（白色椭圆）
                let cloud_alpha = if dy > 0.0 || dy > -8.0 && dist < 10.0 {
                    let cloud_dist = ((dx * dx * 0.5 + dy * dy * 1.5).sqrt()).max(1.0);
                    ((1.0 - (cloud_dist / 12.0)).max(0.0) * 220.0) as u8
                } else {
                    0
                };
                pixels[idx] = 255;
                pixels[idx + 1] = 255;
                pixels[idx + 2] = 255;
                pixels[idx + 3] = cloud_alpha;
            } else {
                pixels[idx + 3] = 0; // 透明
            }
        }
    }
    RainbowIcon {
        width: w,
        height: h,
        pixels,
    }
}

const RAINBOW_BANDS: [(u8, u8, u8); 7] = [
    (255, 0, 0),     // 红
    (255, 128, 0),   // 橙
    (255, 255, 0),   // 黄
    (0, 200, 0),     // 绿
    (0, 100, 255),   // 蓝
    (75, 0, 200),    // 靛
    (180, 0, 180),   // 紫
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
                let band = ((norm_angle * 6.0) as usize).min(6);
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
