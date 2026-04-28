//! 热模块替换（HMR）管理器
//!
//! 管理 Vue 组件的热更新

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info};
use serde::{Serialize, Deserialize};

/// HMR 补丁类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatchType {
    /// Vue 组件重载
    VueReload,
    /// CSS 更新
    CSSUpdate,
    /// 完整页面重载
    FullReload,
}

/// HMR 补丁
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HMRPatch {
    /// 补丁类型
    pub patch_type: PatchType,
    /// 文件路径
    pub file_path: String,
    /// 时间戳
    pub timestamp: u64,
    /// 补丁内容
    pub content: Option<String>,
}

/// HMR 管理器
pub struct HMRManager {
    /// 文件修改时间缓存
    file_timestamps: HashMap<String, u64>,
    /// 补丁队列
    pending_patches: Vec<HMRPatch>,
}

impl HMRManager {
    /// 创建新的 HMR 管理器
    pub fn new() -> Self {
        Self {
            file_timestamps: HashMap::new(),
            pending_patches: Vec::new(),
        }
    }

    /// 检查文件是否已修改
    pub fn check_file_change(&mut self, file_path: &str, current_timestamp: u64) -> bool {
        let old_timestamp = self.file_timestamps.get(file_path).copied().unwrap_or(0);
        
        if current_timestamp > old_timestamp {
            debug!(
                "File changed: {} ({} -> {})",
                file_path, old_timestamp, current_timestamp
            );
            self.file_timestamps.insert(file_path.to_string(), current_timestamp);
            true
        } else {
            false
        }
    }

    /// 生成 Vue 组件重载补丁
    pub fn generate_vue_reload_patch(&mut self, file_path: &str, content: &str) -> HMRPatch {
        let timestamp = self.current_timestamp();
        
        let patch = HMRPatch {
            patch_type: PatchType::VueReload,
            file_path: file_path.to_string(),
            timestamp,
            content: Some(content.to_string()),
        };

        self.pending_patches.push(patch.clone());
        info!("Generated Vue reload patch for: {}", file_path);

        patch
    }

    /// 生成 CSS 更新补丁
    pub fn generate_css_update_patch(&mut self, file_path: &str, content: &str) -> HMRPatch {
        let timestamp = self.current_timestamp();
        
        let patch = HMRPatch {
            patch_type: PatchType::CSSUpdate,
            file_path: file_path.to_string(),
            timestamp,
            content: Some(content.to_string()),
        };

        self.pending_patches.push(patch.clone());
        info!("Generated CSS update patch for: {}", file_path);

        patch
    }

    /// 生成完整页面重载补丁
    pub fn generate_full_reload_patch(&mut self, reason: &str) -> HMRPatch {
        let timestamp = self.current_timestamp();
        
        let patch = HMRPatch {
            patch_type: PatchType::FullReload,
            file_path: String::new(),
            timestamp,
            content: Some(reason.to_string()),
        };

        self.pending_patches.push(patch.clone());
        info!("Generated full reload patch: {}", reason);

        patch
    }

    /// 获取待处理的补丁
    pub fn get_pending_patches(&mut self) -> Vec<HMRPatch> {
        std::mem::take(&mut self.pending_patches)
    }

    /// 清除补丁队列
    pub fn clear_patches(&mut self) {
        self.pending_patches.clear();
    }

    /// 清除文件时间戳缓存
    pub fn clear_timestamps(&mut self) {
        self.file_timestamps.clear();
    }

    /// 获取当前时间戳
    fn current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// 获取文件最后修改时间
    pub fn get_file_timestamp(&self, file_path: &str) -> Option<u64> {
        self.file_timestamps.get(file_path).copied()
    }

    /// 设置文件最后修改时间
    pub fn set_file_timestamp(&mut self, file_path: &str, timestamp: u64) {
        self.file_timestamps.insert(file_path.to_string(), timestamp);
    }
}

impl Default for HMRManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_file_change() {
        let mut hmr = HMRManager::new();
        
        // 首次检查（应该检测到变化）
        assert!(hmr.check_file_change("test.vue", 1000));
        
        // 相同时间戳（应该没有变化）
        assert!(!hmr.check_file_change("test.vue", 1000));
        
        // 新的时间戳（应该检测到变化）
        assert!(hmr.check_file_change("test.vue", 2000));
    }

    #[test]
    fn test_generate_vue_reload_patch() {
        let mut hmr = HMRManager::new();
        let patch = hmr.generate_vue_reload_patch("App.vue", "<template>...</template>");
        
        assert_eq!(patch.patch_type, PatchType::VueReload);
        assert_eq!(patch.file_path, "App.vue");
        assert!(patch.content.is_some());
    }

    #[test]
    fn test_generate_css_update_patch() {
        let mut hmr = HMRManager::new();
        let patch = hmr.generate_css_update_patch("style.css", "body { color: red; }");
        
        assert_eq!(patch.patch_type, PatchType::CSSUpdate);
        assert_eq!(patch.file_path, "style.css");
    }

    #[test]
    fn test_generate_full_reload_patch() {
        let mut hmr = HMRManager::new();
        let patch = hmr.generate_full_reload_patch("Entry file changed");
        
        assert_eq!(patch.patch_type, PatchType::FullReload);
        assert_eq!(patch.content, Some("Entry file changed".to_string()));
    }

    #[test]
    fn test_pending_patches() {
        let mut hmr = HMRManager::new();
        
        hmr.generate_vue_reload_patch("A.vue", "...");
        hmr.generate_vue_reload_patch("B.vue", "...");
        
        let patches = hmr.get_pending_patches();
        assert_eq!(patches.len(), 2);
        
        // 再次获取应该为空
        let patches = hmr.get_pending_patches();
        assert_eq!(patches.len(), 0);
    }

    #[test]
    fn test_clear_timestamps() {
        let mut hmr = HMRManager::new();
        
        hmr.set_file_timestamp("A.vue", 1000);
        hmr.set_file_timestamp("B.vue", 2000);
        
        assert_eq!(hmr.file_timestamps.len(), 2);
        
        hmr.clear_timestamps();
        assert_eq!(hmr.file_timestamps.len(), 0);
    }
}
