//! 脏矩形管理器
//!
//! 用于优化渲染性能，只重绘发生变化的区域。
//!
//! # 工作原理
//!
//! 1. 跟踪上一帧和当前帧的渲染状态
//! 2. 检测变化的区域（脏矩形）
//! 3. 合并重叠的脏矩形以减少绘制调用
//! 4. 只重绘脏矩形区域，而不是整个屏幕

#[derive(Debug, Clone)]
pub struct DirtyRect {
    /// 矩形位置 (x, y)
    pub x: f32,
    pub y: f32,
    /// 矩形尺寸
    pub width: f32,
    pub height: f32,
}

impl DirtyRect {
    /// 创建新的脏矩形
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// 计算两个矩形的并集（包围盒）
    pub fn union(&self, other: &DirtyRect) -> DirtyRect {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);

        DirtyRect {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }

    /// 检查两个矩形是否重叠
    pub fn intersects(&self, other: &DirtyRect) -> bool {
        !(self.x + self.width < other.x
            || other.x + other.width < self.x
            || self.y + self.height < other.y
            || other.y + other.height < self.y)
    }

    /// 计算矩形面积
    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    /// 检查矩形是否有效（面积 > 0）
    pub fn is_valid(&self) -> bool {
        self.width > 0.0 && self.height > 0.0
    }
}

/// 脏矩形管理器
///
/// 管理和优化需要重绘的区域
pub struct DirtyRectManager {
    /// 当前帧的脏矩形列表
    dirty_rects: Vec<DirtyRect>,
    /// 是否启用了脏矩形优化
    enabled: bool,
    /// 脏矩形合并阈值（面积比例）
    merge_threshold: f32,
    /// 屏幕尺寸
    screen_width: f32,
    screen_height: f32,
    /// 统计信息
    stats: DirtyRectStats,
}

/// 脏矩形统计信息
#[derive(Debug, Default)]
pub struct DirtyRectStats {
    /// 总脏矩形数量
    pub total_dirty_rects: usize,
    /// 合并后的脏矩形数量
    pub merged_dirty_rects: usize,
    /// 节省的渲染面积比例
    pub saved_area_ratio: f32,
    /// 是否需要全屏重绘
    pub needs_full_redraw: bool,
}

impl DirtyRectManager {
    /// 创建新的脏矩形管理器
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            dirty_rects: Vec::new(),
            enabled: true,
            merge_threshold: 0.5, // 50% 重叠时合并
            screen_width,
            screen_height,
            stats: DirtyRectStats::default(),
        }
    }

    /// 启用/禁用脏矩形优化
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 检查是否启用了脏矩形优化
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 添加一个脏矩形
    pub fn add_dirty_rect(&mut self, rect: DirtyRect) {
        if !rect.is_valid() {
            return;
        }

        self.dirty_rects.push(rect);
        self.stats.total_dirty_rects += 1;
    }

    /// 添加变化区域（便捷方法）
    pub fn add_change(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.add_dirty_rect(DirtyRect::new(x, y, width, height));
    }

    /// 合并重叠的脏矩形
    ///
    /// 使用简单的迭代合并算法：
    /// 1. 遍历所有脏矩形
    /// 2. 如果两个矩形重叠，合并它们
    /// 3. 重复直到没有更多重叠
    pub fn merge_overlapping(&mut self) {
        if self.dirty_rects.len() <= 1 {
            return;
        }

        let mut merged = true;
        while merged {
            merged = false;
            let mut new_rects = Vec::new();
            let mut used = vec![false; self.dirty_rects.len()];

            for i in 0..self.dirty_rects.len() {
                if used[i] {
                    continue;
                }

                let mut current = self.dirty_rects[i].clone();
                used[i] = true;

                // 尝试与后续矩形合并
                for j in (i + 1)..self.dirty_rects.len() {
                    if used[j] {
                        continue;
                    }

                    if current.intersects(&self.dirty_rects[j]) {
                        current = current.union(&self.dirty_rects[j]);
                        used[j] = true;
                        merged = true;
                    }
                }

                new_rects.push(current);
            }

            self.dirty_rects = new_rects;
        }

        self.stats.merged_dirty_rects = self.dirty_rects.len();
    }

    /// 计算需要重绘的区域
    ///
    /// 返回合并后的脏矩形列表
    pub fn compute_redraw_regions(&mut self) -> Vec<DirtyRect> {
        if !self.enabled || self.dirty_rects.is_empty() {
            // 如果禁用或没有脏矩形，返回全屏
            self.stats.needs_full_redraw = true;
            return vec![DirtyRect::new(
                0.0,
                0.0,
                self.screen_width,
                self.screen_height,
            )];
        }

        // 合并重叠的矩形
        self.merge_overlapping();

        // 检查是否需要全屏重绘
        let total_dirty_area: f32 = self.dirty_rects.iter().map(|r| r.area()).sum();
        let screen_area = self.screen_width * self.screen_height;
        let dirty_ratio = total_dirty_area / screen_area;

        self.stats.needs_full_redraw = dirty_ratio > self.merge_threshold;
        self.stats.saved_area_ratio = 1.0 - dirty_ratio;

        if self.stats.needs_full_redraw {
            // 如果脏区域太大，直接全屏重绘
            self.dirty_rects.clear();
            vec![DirtyRect::new(
                0.0,
                0.0,
                self.screen_width,
                self.screen_height,
            )]
        } else {
            // 返回合并后的脏矩形
            self.dirty_rects.clone()
        }
    }

    /// 清除所有脏矩形（帧结束后调用）
    pub fn clear(&mut self) {
        self.dirty_rects.clear();
        // 保留统计信息用于调试
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> &DirtyRectStats {
        &self.stats
    }

    /// 重置统计信息
    pub fn reset_stats(&mut self) {
        self.stats = DirtyRectStats::default();
    }

    /// 标记全屏为重绘区域
    pub fn mark_full_dirty(&mut self) {
        self.dirty_rects.clear();
        self.add_dirty_rect(DirtyRect::new(
            0.0,
            0.0,
            self.screen_width,
            self.screen_height,
        ));
    }

    /// 获取脏矩形数量
    pub fn dirty_count(&self) -> usize {
        self.dirty_rects.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_rect_union() {
        let rect1 = DirtyRect::new(0.0, 0.0, 100.0, 100.0);
        let rect2 = DirtyRect::new(50.0, 50.0, 100.0, 100.0);

        let union = rect1.union(&rect2);

        assert!((union.x - 0.0).abs() < f32::EPSILON);
        assert!((union.y - 0.0).abs() < f32::EPSILON);
        assert!((union.width - 150.0).abs() < f32::EPSILON);
        assert!((union.height - 150.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_dirty_rect_intersects() {
        let rect1 = DirtyRect::new(0.0, 0.0, 100.0, 100.0);
        let rect2 = DirtyRect::new(50.0, 50.0, 100.0, 100.0);
        let rect3 = DirtyRect::new(200.0, 200.0, 100.0, 100.0);

        assert!(rect1.intersects(&rect2));
        assert!(!rect1.intersects(&rect3));
    }

    #[test]
    fn test_dirty_rect_manager_merge() {
        let mut manager = DirtyRectManager::new(800.0, 600.0);

        // 添加两个重叠的矩形
        manager.add_change(0.0, 0.0, 100.0, 100.0);
        manager.add_change(50.0, 50.0, 100.0, 100.0);

        assert_eq!(manager.dirty_count(), 2);

        manager.merge_overlapping();

        assert_eq!(manager.dirty_count(), 1);
    }

    #[test]
    fn test_dirty_rect_manager_compute_regions() {
        let mut manager = DirtyRectManager::new(800.0, 600.0);

        // 添加小的脏区域
        manager.add_change(100.0, 100.0, 50.0, 50.0);
        manager.add_change(200.0, 200.0, 50.0, 50.0);

        let regions = manager.compute_redraw_regions();

        // 应该是两个不重叠的区域
        assert!(regions.len() <= 2);
    }

    #[test]
    fn test_dirty_rect_manager_full_redraw() {
        let mut manager = DirtyRectManager::new(800.0, 600.0);

        // 标记全屏
        manager.mark_full_dirty();

        let regions = manager.compute_redraw_regions();

        assert_eq!(regions.len(), 1);
        assert!((regions[0].width - 800.0).abs() < f32::EPSILON);
        assert!((regions[0].height - 600.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_dirty_rect_stats() {
        let mut manager = DirtyRectManager::new(800.0, 600.0);

        manager.add_change(0.0, 0.0, 100.0, 100.0);
        manager.add_change(50.0, 50.0, 100.0, 100.0);

        let _ = manager.compute_redraw_regions();

        let stats = manager.get_stats();
        assert!(stats.total_dirty_rects >= 2);
        assert!(stats.merged_dirty_rects <= 2);
    }

    #[test]
    fn test_dirty_rect_disabled() {
        let mut manager = DirtyRectManager::new(800.0, 600.0);
        manager.set_enabled(false);

        manager.add_change(100.0, 100.0, 50.0, 50.0);

        let regions = manager.compute_redraw_regions();

        // 禁用时应该返回全屏
        assert_eq!(regions.len(), 1);
        assert!((regions[0].width - 800.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_dirty_rect_clear() {
        let mut manager = DirtyRectManager::new(800.0, 600.0);

        manager.add_change(0.0, 0.0, 100.0, 100.0);
        manager.add_change(200.0, 200.0, 100.0, 100.0);

        assert_eq!(manager.dirty_count(), 2);

        manager.clear();

        assert_eq!(manager.dirty_count(), 0);
    }
}
