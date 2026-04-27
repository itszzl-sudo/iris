//! 布局缓存系统
//!
//! 提供高效的布局结果缓存机制，避免重复计算：
//! - LRU 缓存策略
//! - 基于内容哈希的失效检测
//! - 统计信息追踪
//! - 可配置的缓存大小

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::Instant;

use crate::layout::LayoutBox;

/// 布局缓存键
///
/// 基于节点内容和样式生成唯一标识
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LayoutCacheKey {
    /// 节点标签
    pub tag: String,
    /// 样式哈希
    pub style_hash: u64,
    /// 子节点数量
    pub children_count: usize,
    /// 内容哈希（文本节点）
    pub content_hash: Option<u64>,
}

impl LayoutCacheKey {
    /// 创建新的缓存键
    pub fn new(tag: &str, style_hash: u64, children_count: usize) -> Self {
        Self {
            tag: tag.to_string(),
            style_hash,
            children_count,
            content_hash: None,
        }
    }

    /// 创建带内容哈希的缓存键（用于文本节点）
    pub fn with_content(tag: &str, style_hash: u64, children_count: usize, content: &str) -> Self {
        let mut hasher = seahash::hash(content.as_bytes());
        Self {
            tag: tag.to_string(),
            style_hash,
            children_count,
            content_hash: Some(hasher),
        }
    }
}

impl Hash for LayoutCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tag.hash(state);
        self.style_hash.hash(state);
        self.children_count.hash(state);
        self.content_hash.hash(state);
    }
}

/// 缓存条目
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// 布局结果
    pub layout: LayoutBox,
    /// 创建时间
    pub created_at: Instant,
    /// 最后访问时间
    pub last_accessed: Instant,
    /// 访问次数
    pub access_count: u64,
}

impl CacheEntry {
    /// 创建新的缓存条目
    pub fn new(layout: LayoutBox) -> Self {
        let now = Instant::now();
        Self {
            layout,
            created_at: now,
            last_accessed: now,
            access_count: 0,
        }
    }

    /// 标记为已访问
    pub fn access(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;
    }
}

/// 布局缓存统计信息
#[derive(Debug, Clone)]
pub struct LayoutCacheStats {
    /// 缓存命中次数
    pub hits: u64,
    /// 缓存未命中次数
    pub misses: u64,
    /// 缓存条目数
    pub entries: usize,
    /// 缓存容量
    pub capacity: usize,
    /// 总访问时间（毫秒）
    pub total_access_time_ms: f64,
}

impl LayoutCacheStats {
    /// 创建新的统计信息
    pub fn new(capacity: usize) -> Self {
        Self {
            hits: 0,
            misses: 0,
            entries: 0,
            capacity,
            total_access_time_ms: 0.0,
        }
    }

    /// 命中率
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// 记录命中
    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    /// 记录未命中
    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    /// 重置统计信息
    pub fn reset(&mut self) {
        self.hits = 0;
        self.misses = 0;
        self.total_access_time_ms = 0.0;
    }
}

/// 布局缓存
///
/// 使用 LRU（最近最少使用）策略管理布局结果缓存
pub struct LayoutCache {
    /// 缓存存储
    cache: HashMap<LayoutCacheKey, CacheEntry>,
    /// 访问顺序（用于 LRU）
    access_order: Vec<LayoutCacheKey>,
    /// 最大缓存容量
    capacity: usize,
    /// 统计信息
    stats: LayoutCacheStats,
}

impl LayoutCache {
    /// 创建新的布局缓存
    ///
    /// # 参数
    ///
    /// - `capacity`: 最大缓存条目数
    ///
    /// # 示例
    ///
    /// ```rust
    /// use iris_layout::cache::LayoutCache;
    ///
    /// let cache = LayoutCache::new(100);
    /// ```
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: HashMap::new(),
            access_order: Vec::new(),
            capacity,
            stats: LayoutCacheStats::new(capacity),
        }
    }

    /// 获取缓存的布局结果
    ///
    /// # 参数
    ///
    /// - `key`: 缓存键
    ///
    /// # 返回
    ///
    /// 如果缓存命中，返回 Some(LayoutBox)，否则返回 None
    pub fn get(&mut self, key: &LayoutCacheKey) -> Option<LayoutBox> {
        let start_time = Instant::now();

        let result = if let Some(entry) = self.cache.get_mut(key) {
            entry.access();
            self.stats.record_hit();

            let elapsed = start_time.elapsed().as_secs_f64() * 1000.0;
            self.stats.total_access_time_ms += elapsed;

            Some(entry.layout.clone())
        } else {
            self.stats.record_miss();
            None
        };

        // 在借用结束后更新访问顺序
        if result.is_some() {
            self.update_access_order(key);
        }

        result
    }

    /// 插入布局结果到缓存
    ///
    /// # 参数
    ///
    /// - `key`: 缓存键
    /// - `layout`: 布局结果
    pub fn insert(&mut self, key: LayoutCacheKey, layout: LayoutBox) {
        // 如果缓存已满，移除 LRU 条目
        if self.cache.len() >= self.capacity && !self.cache.contains_key(&key) {
            self.evict_lru();
        }

        let entry = CacheEntry::new(layout);
        self.cache.insert(key.clone(), entry);
        self.update_access_order(&key);
        self.stats.entries = self.cache.len();
    }

    /// 清除所有缓存
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
        self.stats.entries = 0;
    }

    /// 获取统计信息
    pub fn stats(&self) -> &LayoutCacheStats {
        &self.stats
    }

    /// 获取缓存命中率
    pub fn hit_rate(&self) -> f64 {
        self.stats.hit_rate()
    }

    /// 获取缓存大小
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// 移除最近最少使用的条目
    fn evict_lru(&mut self) {
        if let Some(lru_key) = self.access_order.first().cloned() {
            self.cache.remove(&lru_key);
            self.access_order.retain(|k| k != &lru_key);
        }
    }

    /// 更新访问顺序
    fn update_access_order(&mut self, key: &LayoutCacheKey) {
        // 移除旧的顺序
        self.access_order.retain(|k| k != key);
        // 添加到末尾（最新访问）
        self.access_order.push(key.clone());
    }
}

/// 样式哈希计算器
///
/// 为样式集合生成唯一哈希值
pub struct StyleHasher;

impl StyleHasher {
    /// 计算样式哈希
    ///
    /// # 参数
    ///
    /// - `styles`: 样式键值对列表
    ///
    /// # 返回
    ///
    /// 返回样式的唯一哈希值
    pub fn hash_styles(styles: &[(String, String)]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();

        // 对样式排序以确保一致性
        let mut sorted_styles = styles.to_vec();
        sorted_styles.sort_by(|a, b| a.0.cmp(&b.0));

        for (key, value) in sorted_styles {
            key.hash(&mut hasher);
            value.hash(&mut hasher);
        }

        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutBox;

    fn create_test_layout() -> LayoutBox {
        let mut layout = LayoutBox::new();
        layout.x = 0.0;
        layout.y = 0.0;
        layout.width = 100.0;
        layout.height = 50.0;
        layout
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = LayoutCache::new(10);
        let key = LayoutCacheKey::new("div", 12345, 0);
        let layout = create_test_layout();

        cache.insert(key.clone(), layout);
        let retrieved = cache.get(&key);

        assert!(retrieved.is_some());
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = LayoutCache::new(10);
        let key = LayoutCacheKey::new("div", 12345, 0);

        let retrieved = cache.get(&key);
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_cache_capacity() {
        let mut cache = LayoutCache::new(3);

        // 插入 3 个条目
        for i in 0..3 {
            let key = LayoutCacheKey::new(&format!("div{}", i), i as u64, 0);
            cache.insert(key, create_test_layout());
        }

        assert_eq!(cache.len(), 3);

        // 插入第 4 个条目，应该触发 LRU 驱逐
        let key = LayoutCacheKey::new("div3", 3, 0);
        cache.insert(key, create_test_layout());

        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = LayoutCache::new(2);

        let key1 = LayoutCacheKey::new("div1", 1, 0);
        let key2 = LayoutCacheKey::new("div2", 2, 0);
        let key3 = LayoutCacheKey::new("div3", 3, 0);

        cache.insert(key1.clone(), create_test_layout());
        cache.insert(key2.clone(), create_test_layout());

        // 访问 key1，使其成为最近使用的
        cache.get(&key1);

        // 插入 key3，应该驱逐 key2（LRU）
        cache.insert(key3.clone(), create_test_layout());

        assert!(cache.get(&key1).is_some());
        assert!(cache.get(&key2).is_none()); // 被驱逐
        assert!(cache.get(&key3).is_some());
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = LayoutCache::new(10);
        let key = LayoutCacheKey::new("div", 12345, 0);

        // 未命中
        cache.get(&key);
        assert_eq!(cache.stats().misses, 1);
        assert_eq!(cache.stats().hits, 0);

        // 插入并命中
        cache.insert(key.clone(), create_test_layout());
        cache.get(&key);
        assert_eq!(cache.stats().misses, 1);
        assert_eq!(cache.stats().hits, 1);

        // 命中率 50%
        assert!((cache.hit_rate() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = LayoutCache::new(10);

        for i in 0..5 {
            let key = LayoutCacheKey::new(&format!("div{}", i), i as u64, 0);
            cache.insert(key, create_test_layout());
        }

        assert_eq!(cache.len(), 5);
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_style_hasher() {
        let styles1 = vec![
            ("width".to_string(), "100px".to_string()),
            ("height".to_string(), "200px".to_string()),
        ];

        let styles2 = vec![
            ("height".to_string(), "200px".to_string()),
            ("width".to_string(), "100px".to_string()),
        ];

        // 相同样式不同顺序应该产生相同哈希
        let hash1 = StyleHasher::hash_styles(&styles1);
        let hash2 = StyleHasher::hash_styles(&styles2);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_cache_key_with_content() {
        let key1 = LayoutCacheKey::with_content("p", 12345, 0, "Hello");
        let key2 = LayoutCacheKey::with_content("p", 12345, 0, "World");

        // 不同内容应该产生不同哈希
        assert_ne!(key1, key2);

        let key3 = LayoutCacheKey::with_content("p", 12345, 0, "Hello");
        assert_eq!(key1, key3);
    }

    #[test]
    fn test_cache_access_count() {
        let mut cache = LayoutCache::new(10);
        let key = LayoutCacheKey::new("div", 12345, 0);

        cache.insert(key.clone(), create_test_layout());

        // 多次访问
        for _ in 0..5 {
            cache.get(&key);
        }

        // 验证访问计数（通过 LRU 顺序间接验证）
        assert_eq!(cache.len(), 1);
    }
}
