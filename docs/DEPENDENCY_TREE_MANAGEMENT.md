# 依赖树管理功能

## 概述

`DependencyTree` 模块负责管理 Vue 项目的 npm 依赖树，实现：
1. 解析 package.json 构建完整的依赖关系
2. 排除编译工具类依赖（vite、webpack、babel 等）
3. 检测依赖版本变化
4. 按需重新编译受影响的模块

## 核心功能

### 1. 依赖解析

从 `package.json` 中解析所有依赖：

```rust
use iris_jetcrab_engine::dependency_tree::DependencyTree;

let dep_tree = DependencyTree::from_package_json(&project_root)?;
```

**解析内容**：
- `dependencies`（运行时依赖）
- `devDependencies`（开发依赖）
- 包的实际安装版本
- 包的依赖关系

### 2. 编译工具过滤

自动排除不需要编译到运行时的工具类依赖：

**排除列表**：
```rust
const BUILD_TOOLS: &[&str] = &[
    // 构建工具
    "vite", "webpack", "webpack-cli", "webpack-dev-server",
    "rollup", "parcel", "esbuild", "swc",
    // Babel 相关
    "babel-loader", "@babel/core", "@babel/preset-env",
    // TypeScript 编译
    "typescript", "ts-loader", "ts-node",
    // 开发工具
    "eslint", "prettier", "stylelint",
    "eslint-loader", "css-loader", "sass-loader",
    // 测试工具
    "jest", "vitest", "mocha", "chai",
    // 其他
    "nodemon", "concurrently", "cross-env",
];
```

**运行时依赖**（保留）：
- vue
- vue-router
- vuex / pinia
- axios
- lodash
- 等等...

### 3. 版本变化检测

通过哈希比较检测依赖变化：

```rust
let old_tree = DependencyTree::load_from_cache(&project_root)?;
let new_tree = DependencyTree::from_package_json(&project_root)?;

if old_tree.has_changed(&new_tree) {
    let changes = old_tree.get_changed_dependencies(&new_tree);
    
    for change in &changes {
        match change.change_type {
            ChangeType::Added => println!("+ {} (new)", change.name),
            ChangeType::Updated => println!("~ {} ({} -> {})", 
                change.name,
                change.old_version.unwrap_or("unknown"),
                change.new_version.unwrap_or("unknown")),
            ChangeType::Removed => println!("- {} (removed)", change.name),
        }
    }
}
```

### 4. 按需重新编译

当依赖版本变化时，自动重新编译受影响的模块：

```rust
// 获取需要重新编译的模块
let modules_to_rebuild = dep_tree.get_modules_to_rebuild(
    &changes,
    &module_dependencies // 模块 -> 依赖的 npm 包映射
);

for module in &modules_to_rebuild {
    println!("Rebuilding module: {}", module);
    // 重新编译模块
}
```

## 数据结构

### DependencyTree

```rust
pub struct DependencyTree {
    /// 项目根目录
    pub project_root: PathBuf,
    /// 所有依赖
    pub dependencies: HashMap<String, DependencyInfo>,
    /// 运行时依赖（非编译工具）
    pub runtime_dependencies: HashMap<String, DependencyInfo>,
    /// 依赖哈希（用于检测变化）
    pub dependency_hash: String,
}
```

### DependencyInfo

```rust
pub struct DependencyInfo {
    /// 包名
    pub name: String,
    /// 版本要求（如 "^3.0.0"）
    pub version_req: String,
    /// 实际安装的版本
    pub installed_version: Option<String>,
    /// 是否为开发依赖
    pub is_dev_dependency: bool,
    /// 是否为编译工具（需要排除）
    pub is_build_tool: bool,
    /// 包路径
    pub package_path: Option<PathBuf>,
    /// 依赖的其他包
    pub dependencies: Vec<String>,
}
```

### ChangedDependency

```rust
pub struct ChangedDependency {
    pub name: String,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
    pub change_type: ChangeType,
}

pub enum ChangeType {
    Added,    // 新增
    Updated,  // 更新
    Removed,  // 删除
}
```

## 缓存机制

依赖树会自动缓存到 `.iris-cache/dependency-tree.json`：

```rust
// 保存到缓存
dep_tree.save_to_cache()?;

// 从缓存加载
let dep_tree = DependencyTree::load_from_cache(&project_root)?;
```

**缓存优势**：
1. 避免重复解析 package.json
2. 快速检测依赖变化
3. 提升启动速度

## 集成到编译器

在 `CompilerCache` 中自动集成依赖树管理：

```rust
// 编译项目时
async fn compile_project(&self) -> Result<()> {
    // 构建并检查依赖树
    let new_dep_tree = DependencyTree::from_package_json(&self.project_root)?;
    
    // 检查依赖是否变化
    let needs_full_rebuild = {
        let old_tree = self.dependency_tree.lock().await;
        if let Some(old) = old_tree.as_ref() {
            old.has_changed(&new_dep_tree)
        } else {
            true // 首次编译
        }
    };
    
    if needs_full_rebuild {
        info!("Full rebuild required due to dependency changes");
    }
    
    // ... 编译逻辑
    
    // 保存新的依赖树
    *self.dependency_tree.lock().await = Some(new_dep_tree.clone());
    let _ = new_dep_tree.save_to_cache();
}
```

## 工作流程

```
启动服务器
    ↓
加载缓存的依赖树
    ↓
首次请求时编译项目
    ↓
解析 package.json 构建新依赖树
    ↓
比较依赖哈希
    ├─ 无变化 → 使用缓存的编译结果
    └─ 有变化 → 重新编译项目
         ↓
    保存新的依赖树到缓存
```

## 测试

运行测试：

```bash
cargo test -p iris-jetcrab-engine dependency_tree
```

**测试覆盖**：
- ✅ 依赖树创建
- ✅ 编译工具检测
- ✅ 依赖哈希计算
- ✅ 版本变化检测

## 示例输出

当检测到依赖变化时：

```
Dependency changes detected: 2 changes
  ~ vue (3.3.0 -> 3.4.0)
  + axios (new)
Full rebuild required
Project compiled: 15 modules
```

## 优势

1. **智能过滤**：自动排除编译工具，只关注运行时依赖
2. **增量编译**：只在依赖变化时重新编译
3. **版本追踪**：精确检测版本变化
4. **缓存优化**：避免重复解析
5. **详细日志**：清晰展示变化内容

## 未来优化

- [ ] 支持 monorepo（多 package.json）
- [ ] 支持 peerDependencies 检测
- [ ] 支持可选依赖（optionalDependencies）
- [ ] 依赖树可视化
- [ ] 依赖冲突检测
