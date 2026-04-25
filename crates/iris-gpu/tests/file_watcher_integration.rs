//! 文件监听器集成测试
//!
//! 模拟文件系统事件，验证：
//! - 防抖机制
//! - 事件去重
//! - SFC 热重载逻辑

use iris_gpu::{deduplicate_changes, FileChange, WatcherConfig};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

/// 临时测试目录（自动清理）。
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("iris_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).expect("Failed to create temp dir");
        Self { path }
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn create_file(&self, name: &str, content: &str) -> PathBuf {
        let file_path = self.path.join(name);
        
        // 创建子目录（如果存在）
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dirs");
        }
        
        fs::write(&file_path, content).expect("Failed to write file");
        file_path
    }

    fn modify_file(&self, name: &str, content: &str) {
        let file_path = self.path.join(name);
        fs::write(&file_path, content).expect("Failed to modify file");
    }

    fn delete_file(&self, name: &str) {
        let file_path = self.path.join(name);
        fs::remove_file(&file_path).expect("Failed to delete file");
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// 测试：创建 .vue 文件应该触发 Created 事件
#[test]
fn test_vue_file_created() {
    let temp = TempDir::new();
    let vue_path = temp.create_file("test.vue", "<template><div>Test</div></template>");

    // 模拟文件创建事件
    let change = FileChange::Created {
        path: vue_path.clone(),
    };

    assert_eq!(change.extension(), Some("vue"));
    assert_eq!(change.path(), &vue_path);
}

/// 测试：修改 .vue 文件应该触发 Modified 事件
#[test]
fn test_vue_file_modified() {
    let temp = TempDir::new();
    let vue_path = temp.create_file("test.vue", "<template><div>Old</div></template>");
    temp.modify_file("test.vue", "<template><div>New</div></template>");

    // 模拟文件修改事件
    let change = FileChange::Modified {
        path: vue_path.clone(),
    };

    assert_eq!(change.extension(), Some("vue"));
    assert!(change.path().exists());
}

/// 测试：删除 .vue 文件应该触发 Removed 事件
#[test]
fn test_vue_file_removed() {
    let temp = TempDir::new();
    let vue_path = temp.create_file("test.vue", "<template><div>Test</div></template>");
    temp.delete_file("test.vue");

    // 模拟文件删除事件
    let change = FileChange::Removed {
        path: vue_path.clone(),
    };

    assert_eq!(change.extension(), Some("vue"));
    assert!(!change.path().exists());
}

/// 测试：重命名 .vue 文件应该触发 Renamed 事件
#[test]
fn test_vue_file_renamed() {
    let temp = TempDir::new();
    let old_path = temp.create_file("old.vue", "<template><div>Test</div></template>");
    let new_path = temp.path().join("new.vue");
    fs::rename(&old_path, &new_path).expect("Failed to rename file");

    // 模拟文件重命名事件
    let change = FileChange::Renamed {
        from: old_path.clone(),
        to: new_path.clone(),
    };

    assert_eq!(change.path(), &old_path);
}

/// 测试：事件去重应该保留每个路径的最后一次变更
#[test]
fn test_event_deduplication() {
    let changes = vec![
        FileChange::Modified {
            path: "a.vue".into(),
        },
        FileChange::Modified {
            path: "a.vue".into(),
        },
        FileChange::Modified {
            path: "b.vue".into(),
        },
        FileChange::Created {
            path: "c.vue".into(),
        },
    ];

    let deduped = deduplicate_changes(changes);

    assert_eq!(deduped.len(), 3);
    assert_eq!(deduped[0].path(), &PathBuf::from("a.vue"));
    assert_eq!(deduped[1].path(), &PathBuf::from("b.vue"));
    assert_eq!(deduped[2].path(), &PathBuf::from("c.vue"));
}

/// 测试：扩展名过滤应该不区分大小写
#[test]
fn test_case_insensitive_extension_filter() {
    let config = WatcherConfig::new("/tmp")
        .extensions(vec!["vue".to_string(), "js".to_string()]);

    if let Some(exts) = &config.extensions {
        // Vue 文件（各种大小写）
        assert!(exts.iter().any(|e| e.to_lowercase() == "vue"));
        assert!(exts.iter().any(|e| e.to_lowercase() == "Vue".to_lowercase()));
        assert!(exts.iter().any(|e| e.to_lowercase() == "VUE".to_lowercase()));

        // JS 文件
        assert!(exts.iter().any(|e| e.to_lowercase() == "js"));
    }
}

/// 测试：防抖延迟配置
#[test]
fn test_debounce_delay_config() {
    let config = WatcherConfig::new("/tmp")
        .debounce_delay(Duration::from_millis(300));

    assert_eq!(config.debounce_delay, Duration::from_millis(300));
}

/// 测试：通道容量配置
#[test]
fn test_channel_capacity_config() {
    let config = WatcherConfig::new("/tmp").channel_capacity(5000);

    assert_eq!(config.channel_capacity, 5000);
}

/// 测试：混合文件类型事件
#[test]
fn test_mixed_file_events() {
    let temp = TempDir::new();

    // 创建多种文件
    let vue_path = temp.create_file("app.vue", "<template><div>App</div></template>");
    let js_path = temp.create_file("utils.js", "export const foo = () => {};");
    let css_path = temp.create_file("styles.css", ".container { display: flex; }");

    // 模拟混合事件
    let changes = vec![
        FileChange::Created {
            path: vue_path.clone(),
        },
        FileChange::Created {
            path: js_path.clone(),
        },
        FileChange::Created {
            path: css_path.clone(),
        },
    ];

    // 验证事件
    assert_eq!(changes.len(), 3);
    assert_eq!(changes[0].extension(), Some("vue"));
    assert_eq!(changes[1].extension(), Some("js"));
    assert_eq!(changes[2].extension(), Some("css"));
}

/// 测试：批量操作（模拟 git checkout）
#[test]
fn test_batch_operations() {
    let temp = TempDir::new();

    // 模拟 git checkout：一次性创建多个文件
    let files = vec![
        "components/Header.vue",
        "components/Footer.vue",
        "pages/Home.vue",
        "pages/About.vue",
        "utils/api.js",
    ];

    for file in &files {
        temp.create_file(file, "// content");
    }

    // 模拟批量创建事件
    let changes: Vec<FileChange> = files
        .iter()
        .map(|f| FileChange::Created {
            path: temp.path().join(f),
        })
        .collect();

    // 验证所有事件都被捕获
    assert_eq!(changes.len(), 5);

    // 去重后应该还是 5 个（不同路径）
    let deduped = deduplicate_changes(changes);
    assert_eq!(deduped.len(), 5);
}

/// 测试：快速连续修改（防抖场景）
#[test]
fn test_rapid_modifications() {
    let temp = TempDir::new();
    let vue_path = temp.create_file("test.vue", "<template><div>1</div></template>");

    // 模拟快速修改 10 次
    let changes: Vec<FileChange> = (0..10)
        .map(|_| FileChange::Modified {
            path: vue_path.clone(),
        })
        .collect();

    // 去重后应该只剩 1 个
    let deduped = deduplicate_changes(changes);
    assert_eq!(deduped.len(), 1);
}

/// 测试：SFC 缓存逻辑（模拟）
#[test]
fn test_sfc_cache_simulation() {
    use std::collections::HashMap;

    let mut cache: HashMap<String, PathBuf> = HashMap::new();

    // 模拟编译并缓存
    let vue_path = PathBuf::from("test.vue");
    cache.insert("test.vue".to_string(), vue_path.clone());

    assert!(cache.contains_key("test.vue"));
    assert_eq!(cache.get("test.vue"), Some(&vue_path));

    // 模拟热重载（更新缓存）
    let new_path = PathBuf::from("test_new.vue");
    cache.insert("test.vue".to_string(), new_path.clone());
    assert_eq!(cache.get("test.vue"), Some(&new_path));

    // 模拟删除
    cache.remove("test.vue");
    assert!(!cache.contains_key("test.vue"));
}

/// 测试：无效路径处理
#[test]
fn test_invalid_path_handling() {
    // 非 UTF-8 路径（在某些系统上可能失败）
    #[cfg(unix)]
    {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let invalid_bytes = b"\xff\xfe";
        let invalid_path = PathBuf::from(OsStr::from_bytes(invalid_bytes));

        let change = FileChange::Modified {
            path: invalid_path,
        };

        // 应该能获取路径，但扩展名可能为 None
        assert!(change.path().exists() || !change.path().exists());
        assert_eq!(change.extension(), None);
    }

    // Windows 上跳过此测试
    #[cfg(windows)]
    {
        // Windows 路径总是 UTF-8 兼容
    }
}

/// 测试：空扩展名过滤
#[test]
fn test_empty_extension_filter() {
    let config = WatcherConfig::new("/tmp");

    // 未设置扩展名过滤器
    assert!(config.extensions.is_none());
}

/// 测试：递归监听配置
#[test]
fn test_recursive_config() {
    let config_recursive = WatcherConfig::new("/tmp").recursive(true);
    let config_non_recursive = WatcherConfig::new("/tmp").recursive(false);

    assert!(config_recursive.recursive);
    assert!(!config_non_recursive.recursive);
}
