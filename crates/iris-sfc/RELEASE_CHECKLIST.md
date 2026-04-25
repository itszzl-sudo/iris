# Iris SFC 发布清单

## 发布前检查清单

### ✅ 代码质量

- [x] 所有测试通过 (72/72)
- [x] 无编译警告
- [x] 代码格式化 (`cargo fmt`)
- [x] Clippy 检查通过 (`cargo clippy`)
- [x] 文档注释完整

### ✅ 文档

- [x] README.md (768 行)
- [x] CHANGELOG.md (163 行)
- [x] TYPESCRIPT_ARCHITECTURE.md (526 行)
- [x] 内联文档注释
- [x] 使用示例

### ✅ 测试覆盖

- [x] 单元测试: 58/58
- [x] 集成测试: 10/10
- [x] 示例测试: 4/4
- [x] 边界情况测试
- [x] 错误处理测试

### ✅ 功能完整性

#### 阶段 1: 模板指令
- [x] v-text
- [x] v-html (含 XSS 警告)
- [x] v-show
- [x] v-if/v-else-if/v-else
- [x] v-for
- [x] v-bind/v-on/v-model
- [x] v-slot
- [x] v-once/v-pre/v-cloak/v-memo

#### 阶段 2: CSS Modules
- [x] `<style module>`
- [x] 类名作用域化
- [x] :local() 伪类
- [x] :global() 伪类
- [x] 类名映射表
- [x] 混合样式块

#### 阶段 3: TypeScript
- [x] swc 62 集成
- [x] 类型擦除
- [x] 可选 tsc 检查
- [x] RAII 文件管理
- [x] 环境变量配置

#### 阶段 4: Script Setup
- [x] `<script setup>` 语法
- [x] defineProps (泛型)
- [x] defineProps (数组)
- [x] defineEmits (泛型)
- [x] defineEmits (数组)
- [x] withDefaults
- [x] 自动 return 生成

### ✅ 错误处理

- [x] 美观错误输出
- [x] 修复建议
- [x] 错误严重性级别
- [x] 格式化方法
- [x] 智能帮助信息

### ✅ 性能

- [x] 编译缓存 (XXH3 + LRU)
- [x] 正则预编译 (LazyLock)
- [x] 全局编译器实例
- [x] 性能基准测试

### ✅ 集成

- [x] iris-app 集成示例
- [x] 序列化/反序列化
- [x] 环境变量配置
- [x] 日志系统 (tracing)

---

## 发布步骤

### 1. 版本更新

```bash
# 更新 Cargo.toml 版本号
# 当前版本: 0.0.1
# 建议发布版本: 0.1.0 (首次公开发布)
```

### 2. 最终测试

```bash
# 运行所有测试
cargo test -p iris-sfc
cargo test -p iris-app --example sfc_integration

# 期望结果: 72/72 通过
```

### 3. 代码检查

```bash
# 格式化
cargo fmt --all

# Clippy
cargo clippy -p iris-sfc -- -D warnings

# 文档检查
cargo doc -p iris-sfc --no-deps
```

### 4. Git 提交

```bash
git add -A
git commit -m "chore: prepare for v0.1.0 release"
git tag v0.1.0
```

### 5. 发布到 crates.io

```bash
# 登录 crates.io
cargo login

# 发布
cd crates/iris-sfc
cargo publish --dry-run  # 测试
cargo publish            # 实际发布
```

### 6. 发布后验证

```bash
# 在新项目中测试
cargo new test-iris
cd test-iris
cargo add iris-sfc

# 编译测试
cat > src/main.rs << 'EOF'
use iris_sfc::compile_from_string;

fn main() {
    let module = compile_from_string("Test", r#"
        <template><div>Hello</div></template>
        <script setup>const msg = "Hello"</script>
    "#).unwrap();
    
    println!("Compiled: {}", module.name);
}
EOF

cargo run
```

---

## 发布内容

### crates/iris-sfc/

```
iris-sfc/
├── src/
│   ├── lib.rs                  # 主入口 (927 行)
│   ├── template_compiler.rs    # 模板编译器 (735 行)
│   ├── ts_compiler.rs          # TS 编译器 (707 行)
│   ├── css_modules.rs          # CSS Modules (234 行)
│   ├── script_setup.rs         # Script Setup (509 行)
│   └── cache.rs                # 缓存系统 (479 行)
├── tests/
│   └── integration_test.rs     # 集成测试 (510 行)
├── examples/
│   └── (示例代码)
├── README.md                   # 使用文档 (768 行)
├── CHANGELOG.md                # 变更日志 (163 行)
├── TYPESCRIPT_ARCHITECTURE.md  # 架构文档 (526 行)
├── Cargo.toml
└── RELEASE_CHECKLIST.md        # 本文件
```

**总代码量**: ~3,600 行核心代码 + ~2,000 行文档

### 依赖项

```toml
[dependencies]
swc = "62"
swc_common = "21"
swc_ecma_parser = "39"
html5ever = "0.27"
regex = "1.10"
lru-cache = "*"
xxhash = "*"
tracing = "0.1"
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
```

---

## 版本历史

### v0.1.0 (Unreleased)

**首次公开发布**

- 完整的 Vue 3 SFC 编译器
- 4 个主要功能阶段
- 72 个测试用例
- 完整的文档和示例

**关键特性**:
- 13+ Vue 指令支持
- CSS Modules 完全支持
- TypeScript 编译和检查
- Script Setup 编译器宏
- 高性能缓存系统
- 美观的错误报告

**性能指标**:
- 编译时间: 1-3ms
- 缓存加速: 1000-3000x
- 内存占用: ~5MB (100 项缓存)

---

## 已知问题

### 已记录

1. **Props 嵌套类型** - 当前只支持扁平 props 接口
   - 状态: 文档说明，警告提示
   - 计划: v0.2.0 支持

2. **复杂 TypeScript** - 某些高级类型可能需要手动处理
   - 状态: 部分支持
   - 计划: v0.2.0 改进

3. **CSS 预处理器** - 暂不支持 SCSS/Less
   - 状态: 计划中
   - 计划: v0.3.0 添加

### 不阻塞发布

以上问题都有文档说明和警告，不影响基本功能使用。

---

## 维护者

- **名称**: Iris Team
- **仓库**: https://gitee.com/wanquanbuhuime/iris
- **许可**: MIT

---

**最后更新**: 2024-04-24
**发布状态**: 准备就绪 ✅
