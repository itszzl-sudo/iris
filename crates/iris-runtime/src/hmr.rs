//! 热模块替换 (HMR) 实现

use serde::{Serialize, Deserialize};
use anyhow::Result;

/// HMR 补丁
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HmrPatch {
    /// 补丁类型
    #[serde(rename = "type")]
    pub patch_type: String,
    /// 文件路径
    pub path: String,
    /// 时间戳
    pub timestamp: u64,
    /// 变更内容
    pub changes: Vec<Change>,
}

/// 变更项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    /// 变更类型
    #[serde(rename = "type")]
    pub change_type: String,
    /// 变更内容
    pub content: String,
}

/// 生成 HMR 补丁
pub fn generate_patch(
    old_source: &str,
    new_source: &str,
    filename: &str,
) -> Result<HmrPatch> {
    let changes = diff_sources(old_source, new_source)?;

    Ok(HmrPatch {
        patch_type: "vue-reload".to_string(),
        path: filename.to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        changes,
    })
}

/// 比较源码差异
fn diff_sources(old_source: &str, new_source: &str) -> Result<Vec<Change>> {
    let mut changes = Vec::new();

    // 简单的行级差异比较
    let old_lines: Vec<&str> = old_source.lines().collect();
    let new_lines: Vec<&str> = new_source.lines().collect();

    // 检查是否有变更
    if old_lines == new_lines {
        return Ok(changes);
    }

    // 标记为完整重载（简化实现）
    changes.push(Change {
        change_type: "reload".to_string(),
        content: new_source.to_string(),
    });

    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_patch() {
        let old_source = "<template><div>Old</div></template>";
        let new_source = "<template><div>New</div></template>";

        let patch = generate_patch(old_source, new_source, "App.vue").unwrap();

        assert_eq!(patch.patch_type, "vue-reload");
        assert_eq!(patch.path, "App.vue");
        assert!(!patch.changes.is_empty());
    }

    #[test]
    fn test_no_changes() {
        let source = "<template><div>Same</div></template>";

        let patch = generate_patch(source, source, "App.vue").unwrap();

        // 无变更时应该返回空 changes
        assert!(patch.changes.is_empty() || patch.changes[0].change_type != "reload");
    }
}
