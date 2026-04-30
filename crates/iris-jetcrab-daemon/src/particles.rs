//! 粒子系统 - 彩虹拖拽轨迹 + 星光粒子 + 呼吸律动

use std::time::Instant;

/// 彩虹颜色循环（暖色调：去除了绿、蓝、紫等冷色）
const RAINBOW_COLORS: [(u8, u8, u8); 7] = [
    (255, 50, 80),   // 玫红
    (255, 80, 60),   // 珊瑚
    (255, 165, 0),   // 橙
    (255, 120, 60),  // 朱红（替代金黄）
    (255, 255, 100), // 暖黄
    (255, 100, 120), // 粉红
    (255, 0, 80),    // 玫瑰
];

/// 拖拽轨迹点
#[derive(Clone)]
pub struct TrailPoint {
    pub x: i32,
    pub y: i32,
    pub time: Instant,
}

/// 星光粒子
pub struct Sparkle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,       // 0.0 ~ 1.0, 1.0 = 刚生成
    pub max_life: f32,   // 生命周期（秒）
    pub size: f32,       // 粒子大小
    pub color_idx: usize, // 彩虹颜色索引
    pub birth: Instant,
}

/// 粒子系统
pub struct ParticleSystem {
    /// 拖拽轨迹点
    pub trail: Vec<TrailPoint>,
    /// 最大轨迹长度
    pub max_trail: usize,
    /// 星光粒子列表
    pub sparkles: Vec<Sparkle>,
    /// 是否正在拖拽
    pub is_dragging: bool,
    /// 上次生成粒子的时间
    last_sparkle_time: Instant,
    /// 呼吸律动相位
    pub breathe_phase: f32,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            trail: Vec::with_capacity(60),
            max_trail: 30,
            sparkles: Vec::new(),
            is_dragging: false,
            last_sparkle_time: Instant::now(),
            breathe_phase: 0.0,
        }
    }

    /// 添加拖拽位置
    pub fn add_trail_point(&mut self, x: i32, y: i32) {
        self.trail.push(TrailPoint {
            x,
            y,
            time: Instant::now(),
        });
        if self.trail.len() > self.max_trail {
            self.trail.remove(0);
        }

        // 拖拽时生成星光粒子
        let now = Instant::now();
        if now.duration_since(self.last_sparkle_time).as_secs_f32() > 0.03 {
            self.spawn_sparkles(x as f32, y as f32);
            self.last_sparkle_time = now;
        }
    }

    /// 生成星光粒子
    fn spawn_sparkles(&mut self, cx: f32, cy: f32) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let mut rng = SimpleRng::new(seed.wrapping_add(self.sparkles.len() as u64));

        for _ in 0..5 {
            let angle = rng.next_f32() * std::f32::consts::TAU;
            let speed = rng.next_f32() * 120.0 + 30.0;
            let life = rng.next_f32() * 0.6 + 0.4;
            let size = rng.next_f32() * 4.0 + 2.5;
            let color_idx = (rng.next_f32() * 7.0) as usize % 7;

            self.sparkles.push(Sparkle {
                x: cx + (rng.next_f32() - 0.5) * 20.0,
                y: cy + (rng.next_f32() - 0.5) * 20.0,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed,
                life: 1.0,
                max_life: life,
                size,
                color_idx,
                birth: Instant::now(),
            });
        }
    }

    /// 更新粒子状态
    pub fn update(&mut self, dt: f32) {
        // 呼吸相位
        self.breathe_phase += dt * 2.0;
        if self.breathe_phase > std::f32::consts::TAU {
            self.breathe_phase -= std::f32::consts::TAU;
        }

        // 更新星光粒子
        let now = Instant::now();
        self.sparkles.retain(|s| {
            let age = now.duration_since(s.birth).as_secs_f32();
            age < s.max_life
        });

        for s in &mut self.sparkles {
            let age = now.duration_since(s.birth).as_secs_f32();
            s.life = 1.0 - (age / s.max_life);
            // 粒子运动 + 减速
            s.x += s.vx * dt;
            s.y += s.vy * dt;
            s.vx *= 0.95;
            s.vy *= 0.95;
            // 略微受重力影响（下沉）
            s.vy += 10.0 * dt;
        }

        // 更新轨迹（删除过旧的轨迹点）
        if !self.is_dragging {
            self.trail.retain(|p| {
                now.duration_since(p.time).as_secs_f32() < 0.8
            });
        }
    }

    /// 获取呼吸光晕透明度
    pub fn breathe_glow_alpha(&self) -> f32 {
        (self.breathe_phase.sin() * 0.15 + 0.25).clamp(0.1, 0.4)
    }

    /// 设置拖拽状态
    pub fn set_dragging(&mut self, dragging: bool) {
        self.is_dragging = dragging;
        if !dragging {
            // 停止拖拽时，再生成一些消散粒子
            if let Some(last) = self.trail.last().cloned() {
                for _ in 0..5 {
                    self.spawn_sparkles(last.x as f32, last.y as f32);
                }
            }
        }
    }

    /// 获取轨迹点的透明度
    pub fn trail_alpha(index: usize, total: usize) -> f32 {
        if total <= 1 {
            return 1.0;
        }
        let t = index as f32 / (total - 1) as f32;
        // 越旧的轨迹点透明度越低，但动态模式下保持可见
        t * t * 0.7 + 0.1
    }

    /// 获取轨迹点的彩虹颜色
    pub fn trail_color(index: usize, total: usize) -> (u8, u8, u8) {
        if total <= 1 {
            return RAINBOW_COLORS[0];
        }
        let t = index as f32 / (total - 1) as f32;
        let color_idx = ((t * 6.0) as usize).min(6);
        RAINBOW_COLORS[color_idx]
    }

    /// 获取粒子颜色
    pub fn sparkle_color(&self, sparkle: &Sparkle) -> (u8, u8, u8) {
        let (r, g, b) = RAINBOW_COLORS[sparkle.color_idx];
        let bright = 0.3 + sparkle.life * 0.7;
        (
            (r as f32 * bright) as u8,
            (g as f32 * bright) as u8,
            (b as f32 * bright) as u8,
        )
    }
}

/// 简易伪随机数生成器
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_f32(&mut self) -> f32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let x = (self.state >> 33) as u32;
        (x as f32) / (u32::MAX as f32)
    }
}
