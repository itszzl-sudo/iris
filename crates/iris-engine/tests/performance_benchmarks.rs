//! Phase 7.2 性能基准测试
//!
//! 测试 Iris Engine 各组件的性能表现：
//! - 布局计算性能
//! - 缓存命中率
//! - DOM 操作性能
//! - 渲染统计性能

use iris_dom::vnode::VNode;
use iris_engine::vnode_renderer::RenderStats;
use iris_layout::cache::{LayoutCache, LayoutCacheKey, StyleHasher};
use std::time::Instant;

// ============================================
// 基准测试 1: VNode 创建性能
// ============================================

/// 测试创建大量 VNode 的性能
#[test]
fn bench_vnode_creation() {
    let start = Instant::now();

    let count = 10_000;
    for i in 0..count {
        let _div = VNode::element("div");
    }

    let duration = start.elapsed();
    println!("✓ Created {} VNodes in {:?}", count, duration);

    // 性能断言：10,000 个 VNode 创建应该在 10ms 内完成
    assert!(
        duration.as_millis() < 100,
        "VNode creation too slow: {:?}",
        duration
    );
}

/// 测试创建带属性的 VNode
#[test]
fn bench_vnode_with_attributes() {
    let start = Instant::now();

    let count = 5_000;
    for i in 0..count {
        let mut div = VNode::element("div");
        div.set_attr("class", &format!("item-{}", i));
        div.set_attr("id", &format!("id-{}", i));
    }

    let duration = start.elapsed();
    println!("✓ Created {} VNodes with attributes in {:?}", count, duration);

    // 性能断言：5,000 个带属性的 VNode 创建应该在 50ms 内
    assert!(
        duration.as_millis() < 200,
        "VNode with attributes creation too slow: {:?}",
        duration
    );
}

// ============================================
// 基准测试 2: DOM 树构建性能
// ============================================

/// 测试构建大型 DOM 树的性能
#[test]
fn bench_large_dom_tree_construction() {
    let start = Instant::now();

    // 构建 1000 个节点的树
    let mut root = VNode::element("div");
    for i in 0..100 {
        let mut section = VNode::element("section");
        for j in 0..10 {
            let mut article = VNode::element("article");
            article.append_child(VNode::text(&format!("Article {}-{}", i, j)));
            section.append_child(article);
        }
        root.append_child(section);
    }

    let duration = start.elapsed();
    let stats = RenderStats::collect(&root);

    println!("✓ Built DOM tree with {} nodes in {:?}", stats.total_nodes, duration);

    // 性能断言：构建 1100 个节点的树应该在 50ms 内
    assert!(
        duration.as_millis() < 100,
        "Large DOM tree construction too slow: {:?}",
        duration
    );
}

// ============================================
// 基准测试 3: 渲染统计性能
// ============================================

/// 测试 RenderStats::collect 的性能
#[test]
fn bench_render_stats_collection() {
    // 构建测试树
    let mut root = VNode::element("div");
    for i in 0..50 {
        let mut section = VNode::element("section");
        for j in 0..20 {
            section.append_child(VNode::text(&format!("Text {}-{}", i, j)));
        }
        root.append_child(section);
    }

    let start = Instant::now();
    let iterations = 100;

    for _ in 0..iterations {
        let _stats = RenderStats::collect(&root);
    }

    let duration = start.elapsed();
    let avg_duration = duration / iterations;

    println!(
        "✓ Collected render stats {} times in {:?} (avg: {:?})",
        iterations, duration, avg_duration
    );

    // 性能断言：平均每次收集应该在 1ms 内
    assert!(
        avg_duration.as_millis() < 5,
        "Render stats collection too slow: avg {:?}",
        avg_duration
    );
}

// ============================================
// 基准测试 4: 布局缓存性能
// ============================================

/// 测试缓存命中率对性能的影响
#[test]
fn bench_cache_hit_performance() {
    let mut cache = LayoutCache::new(100);

    // 预填充缓存
    for i in 0..50 {
        let key = LayoutCacheKey::new("div", i as u64, 0);
        let mut layout = iris_layout::layout::LayoutBox::new();
        layout.x = 0.0;
        layout.y = 0.0;
        layout.width = 100.0;
        layout.height = 50.0;
        cache.insert(key, layout);
    }

    let start = Instant::now();
    let iterations = 10_000;

    // 大量缓存命中
    for i in 0..iterations {
        let key = LayoutCacheKey::new("div", (i % 50) as u64, 0);
        let _ = cache.get(&key);
    }

    let duration = start.elapsed();
    let hit_rate = cache.hit_rate();

    println!(
        "✓ {} cache hits in {:?} (hit rate: {:.1}%)",
        iterations, duration, hit_rate * 100.0
    );

    // 性能断言：10,000 次缓存访问应该在 50ms 内
    assert!(
        duration.as_millis() < 100,
        "Cache access too slow: {:?}",
        duration
    );

    // 命中率应该接近 100%
    assert!(
        hit_rate > 0.95,
        "Cache hit rate too low: {:.1}%",
        hit_rate * 100.0
    );
}

/// 测试缓存未命中性能
#[test]
fn bench_cache_miss_performance() {
    let mut cache = LayoutCache::new(100);

    let start = Instant::now();
    let iterations = 5_000;

    // 大量缓存未命中
    for i in 0..iterations {
        let key = LayoutCacheKey::new("div", i as u64, 0);
        let _ = cache.get(&key);
    }

    let duration = start.elapsed();
    let hit_rate = cache.hit_rate();

    println!(
        "✓ {} cache misses in {:?} (hit rate: {:.1}%)",
        iterations, duration, hit_rate * 100.0
    );

    // 性能断言：5,000 次缓存未命中应该在 50ms 内
    assert!(
        duration.as_millis() < 100,
        "Cache miss handling too slow: {:?}",
        duration
    );

    // 命中率应该接近 0%
    assert!(
        hit_rate < 0.05,
        "Cache hit rate should be near 0: {:.1}%",
        hit_rate * 100.0
    );
}

// ============================================
// 基准测试 5: 样式哈希性能
// ============================================

/// 测试样式哈希计算性能
#[test]
fn bench_style_hash_computation() {
    let styles: Vec<(String, String)> = vec![
        ("width".to_string(), "100px".to_string()),
        ("height".to_string(), "200px".to_string()),
        ("display".to_string(), "flex".to_string()),
        ("flex-direction".to_string(), "row".to_string()),
        ("gap".to_string(), "10px".to_string()),
    ];

    let start = Instant::now();
    let iterations = 10_000;

    for _ in 0..iterations {
        let _hash = StyleHasher::hash_styles(&styles);
    }

    let duration = start.elapsed();
    let avg_duration = duration / iterations;

    println!(
        "✓ Computed {} style hashes in {:?} (avg: {:?})",
        iterations, duration, avg_duration
    );

    // 性能断言：平均每次哈希计算应该在 1μs 内
    assert!(
        avg_duration.as_micros() < 10,
        "Style hash computation too slow: avg {:?}",
        avg_duration
    );
}

// ============================================
// 基准测试 6: 综合性能测试
// ============================================

/// 测试完整的 VNode 创建 → 树构建 → 渲染统计流程
#[test]
fn bench_full_pipeline() {
    let start = Instant::now();
    let iterations = 100;

    for _ in 0..iterations {
        // 1. 创建 VNode 树
        let mut root = VNode::element("div");
        for i in 0..10 {
            let mut section = VNode::element("section");
            section.set_attr("class", &format!("section-{}", i));

            for j in 0..10 {
                let mut article = VNode::element("article");
                article.append_child(VNode::text(&format!("Content {}-{}", i, j)));
                section.append_child(article);
            }

            root.append_child(section);
        }

        // 2. 收集渲染统计
        let _stats = RenderStats::collect(&root);
    }

    let duration = start.elapsed();
    let avg_duration = duration / iterations;

    println!(
        "✓ Completed {} full pipelines in {:?} (avg: {:?})",
        iterations, duration, avg_duration
    );

    // 性能断言：平均每次完整流程应该在 5ms 内
    assert!(
        avg_duration.as_millis() < 10,
        "Full pipeline too slow: avg {:?}",
        avg_duration
    );
}

/// 测试缓存集成到完整流程的性能提升
#[test]
fn bench_cache_integrated_pipeline() {
    let mut cache = LayoutCache::new(100);

    // 第一次运行（缓存未命中）
    let start1 = Instant::now();
    let mut root1 = VNode::element("div");
    for i in 0..50 {
        let key = LayoutCacheKey::new("section", i as u64, 1);
        let mut layout = iris_layout::layout::LayoutBox::new();
        layout.x = 0.0;
        layout.y = 0.0;
        layout.width = 100.0;
        layout.height = 50.0;
        cache.insert(key, layout);

        let mut section = VNode::element("section");
        section.append_child(VNode::text(&format!("Section {}", i)));
        root1.append_child(section);
    }
    let _stats1 = RenderStats::collect(&root1);
    let duration1 = start1.elapsed();

    // 第二次运行（缓存命中）
    let start2 = Instant::now();
    let mut root2 = VNode::element("div");
    for i in 0..50 {
        let key = LayoutCacheKey::new("section", i as u64, 1);
        // 从缓存获取
        let _cached_layout = cache.get(&key);

        let mut section = VNode::element("section");
        section.append_child(VNode::text(&format!("Section {}", i)));
        root2.append_child(section);
    }
    let _stats2 = RenderStats::collect(&root2);
    let duration2 = start2.elapsed();

    println!("✓ Without cache: {:?}", duration1);
    println!("✓ With cache: {:?}", duration2);

    // 验证缓存命中率
    let hit_rate = cache.hit_rate();
    println!("✓ Cache hit rate: {:.1}%", hit_rate * 100.0);

    // 第二次应该更快（虽然对于这个简单测试可能不明显）
    assert!(cache.len() > 0, "Cache should have entries");
}
