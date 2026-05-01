//! 粒子系统 - 彩虹拖拽轨迹 + 星光粒子 + 呼吸律动

use std::time::Instant;

/// 彩虹颜色循环（暖色调：去除了绿、蓝、紫等冷色，不含金黄）
const RAINBOW_COLORS: [(u8, u8, u8); 5] = [
    (255, 50, 80),   // 玫红
    (255, 80, 60),   // 珊瑚
    (255, 120, 60),  // 朱红
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
    /// 呼吸律动相位（1.5秒周期）
    pub breathe_phase: f32,
    /// 上次鼠标交互时间（用于空闲时关闭呼吸特效）
    last_interaction: Instant,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            trail: Vec::with_capacity(60),
            max_trail: 15,
            sparkles: Vec::new(),
            is_dragging: false,
            last_sparkle_time: Instant::now(),
            breathe_phase: 0.0,
            last_interaction: Instant::now(),
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
        if now.duration_since(self.last_sparkle_time).as_secs_f32() > 0.09 {
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
            let speed = rng.next_f32() * 360.0 + 90.0;
            let life = rng.next_f32() * 0.6 + 0.4;
            let size = rng.next_f32() * 8.0 + 5.0;
            let color_idx = (rng.next_f32() * 5.0) as usize % 5;

            self.sparkles.push(Sparkle {
                x: cx + (rng.next_f32() - 0.5) * 60.0,
                y: cy + (rng.next_f32() - 0.5) * 60.0,
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
        // 呼吸相位（1.5秒周期）
        self.breathe_phase += dt * (std::f32::consts::TAU / 1.5);
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

    /// 记录鼠标交互（重置空闲计时，恢复呼吸特效）
    pub fn record_interaction(&mut self) {
        self.last_interaction = Instant::now();
    }

    /// 获取呼吸光晕透明度（空闲超过 3 秒返回 0，关闭呼吸特效）
    pub fn breathe_glow_alpha(&self) -> f32 {
        let idle = Instant::now().duration_since(self.last_interaction).as_secs_f32();
        if idle > 3.0 {
            return 0.0;
        }
        // sin 曲线 0.1~0.5, 1.5秒周期
        (self.breathe_phase.sin() * 0.2 + 0.3).clamp(0.1, 0.5)
    }

    /// 获取轨迹点的透明度
    pub fn trail_alpha(index: usize, total: usize) -> f32 {
        if total <= 1 {
            return 1.0;
        }
        let t = index as f32 / (total - 1) as f32;
        // 越旧的轨迹点透明度越低，提高最低可见度保证在白背景下可见
        0.4 + t * 0.6
    }

    /// 获取轨迹点的彩虹颜色
    pub fn trail_color(index: usize, total: usize) -> (u8, u8, u8) {
        if total <= 1 {
            return RAINBOW_COLORS[0];
        }
        let t = index as f32 / (total - 1) as f32;
        let color_idx = ((t * 4.0) as usize).min(4);
        RAINBOW_COLORS[color_idx]
    }

    /// 获取粒子颜色（使用星光色系：耀眼白、暖白、淡白、浅黄、蓝白）
    pub fn sparkle_color(&self, sparkle: &Sparkle) -> (u8, u8, u8) {
        const STAR_COLORS: [(u8, u8, u8); 5] = [
            (255, 255, 255),  // 耀眼白
            (255, 245, 220),  // 暖白色
            (240, 240, 245),  // 淡白色
            (255, 250, 200),  // 浅黄色
            (220, 235, 255),  // 蓝白色
        ];
        let (r, g, b) = STAR_COLORS[sparkle.color_idx];
        // 亮度衰减：life=1.0 最亮，life→0 逐渐变暗
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_system_new() {
        let ps = ParticleSystem::new();
        assert!(ps.trail.is_empty());
        assert!(ps.sparkles.is_empty());
        assert!(!ps.is_dragging);
        assert!((ps.breathe_phase - 0.0).abs() < f32::EPSILON);
        assert_eq!(ps.max_trail, 15);
    }

    #[test]
    fn test_trail_alpha_bounds() {
        assert!((ParticleSystem::trail_alpha(0, 10) - 0.4).abs() < f32::EPSILON);
        assert!((ParticleSystem::trail_alpha(9, 10) - 1.0).abs() < f32::EPSILON);
        // 单元素
        assert!((ParticleSystem::trail_alpha(0, 1) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_trail_color_cycling() {
        let c0 = ParticleSystem::trail_color(0, 5);
        let c4 = ParticleSystem::trail_color(4, 5);
        // 第一个和最后一个颜色应不同
        assert_ne!(c0, c4);
        // 单元素
        let csingle = ParticleSystem::trail_color(0, 1);
        assert_eq!(csingle, RAINBOW_COLORS[0]);
    }

    #[test]
    fn test_sparkle_color_brightness() {
        let sparkle = Sparkle {
            x: 0.0, y: 0.0, vx: 0.0, vy: 0.0,
            life: 1.0, max_life: 1.0, size: 5.0,
            color_idx: 0, birth: Instant::now(),
        };
        let ps = ParticleSystem::new();
        let (r, g, b) = ps.sparkle_color(&sparkle);
        // life=1.0 => bright=1.0, 应与耀眼白相同
        assert_eq!(r, 255);
        assert_eq!(g, 255);
        assert_eq!(b, 255);
    }

    #[test]
    fn test_sparkle_color_dimming() {
        let sparkle = Sparkle {
            x: 0.0, y: 0.0, vx: 0.0, vy: 0.0,
            life: 0.5, max_life: 1.0, size: 5.0,
            color_idx: 0, birth: Instant::now(),
        };
        let ps = ParticleSystem::new();
        let (r, _, _) = ps.sparkle_color(&sparkle);
        // life=0.5 => bright=0.65, 颜色应暗于满亮度
        assert!(r < 255, "sparkle_color should dim when life < 1.0");
    }

    #[test]
    fn test_breathe_glow_alpha_range() {
        for phase in [0.0, 0.5, 1.0, 1.5, 3.0, 6.28] {
            let ps = ParticleSystem { breathe_phase: phase, ..ParticleSystem::new() };
            let a = ps.breathe_glow_alpha();
            assert!(a >= 0.1 && a <= 0.5,
                "breathe_glow_alpha={} should be in [0.1, 0.5] for phase={}", a, phase);
        }
    }

    #[test]
    fn test_breathe_phase_wraps() {
        let mut ps = ParticleSystem::new();
        // 模拟很多帧
        for _ in 0..100 {
            ps.update(0.016); // ~60fps 的 dt
        }
        assert!(ps.breathe_phase >= 0.0 && ps.breathe_phase <= std::f32::consts::TAU,
            "breathe_phase={} should wrap to [0, TAU]", ps.breathe_phase);
    }

    #[test]
    fn test_simple_rng_deterministic() {
        let mut rng1 = SimpleRng::new(12345);
        let mut rng2 = SimpleRng::new(12345);
        for _ in 0..20 {
            assert!((rng1.next_f32() - rng2.next_f32()).abs() < f32::EPSILON,
                "SimpleRng should produce deterministic sequence for same seed");
        }
    }

    #[test]
    fn test_simple_rng_range() {
        let mut rng = SimpleRng::new(42);
        for _ in 0..100 {
            let v = rng.next_f32();
            assert!(v >= 0.0 && v <= 1.0, "SimpleRng.next_f32 should be in [0,1], got {}", v);
        }
    }

    #[test]
    fn test_simple_rng_zero_seed() {
        // seed=0 should be handled (converted to 1)
        let mut rng = SimpleRng::new(0);
        let v = rng.next_f32();
        assert!(v >= 0.0 && v <= 1.0);
    }

    #[test]
    fn test_add_trail_point_capped() {
        let mut ps = ParticleSystem::new();
        for i in 0..30 {
            ps.add_trail_point(i, i * 2);
        }
        assert!(ps.trail.len() <= ps.max_trail, "trail len {} exceeds max {}", ps.trail.len(), ps.max_trail);
        // 最新的点应该是最后添加的
        let last = ps.trail.last().unwrap();
        assert_eq!(last.x, 29);
        assert_eq!(last.y, 58);
    }
}
