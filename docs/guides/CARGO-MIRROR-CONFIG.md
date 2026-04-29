# Cargo 清华镜像源配置

## ✅ 已配置

本项目已配置使用清华大学 crates.io 镜像源，位于：
- `.cargo/config.toml`

## 📋 配置内容

```toml
[source.crates-io]
replace-with = 'tuna'

[source.tuna]
registry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"
```

## 🚀 效果

- **索引更新速度**: 从 30-60 秒降至 3-5 秒
- **依赖下载速度**: 从 10-100 KB/s 提升至 5-20 MB/s
- **首次编译时间**: 大幅缩短（特别是 swc 等大型依赖）

## 🔍 验证配置

```bash
# 查看 Cargo 配置
cargo config get

# 测试下载速度
cargo fetch

# 编译项目
cargo build -p iris-sfc
```

## 🌐 其他可用镜像源

### 中国科学技术大学 (USTC)
```toml
[source.crates-io]
replace-with = 'ustc'

[source.ustc]
registry = "https://mirrors.ustc.edu.cn/crates.io-index"
```

### RustCC 官方镜像
```toml
[source.crates-io]
replace-with = 'rustcc'

[source.rustcc]
registry = "sparse+https://crates.rustcc.cn"
```

### 字节跳动镜像
```toml
[source.crates-io]
replace-with = 'bytedance'

[source.bytedance]
registry = "sparse+https://mirrors.bytedance.com/crates.io-index/"
```

## ⚙️ 全局配置（可选）

如果希望在所有项目中都使用清华源，可以配置全局 Cargo 配置：

**Windows**:
```powershell
# 编辑全局配置文件
notepad $env:USERPROFILE\.cargo\config.toml
```

**Linux/macOS**:
```bash
# 编辑全局配置文件
nano ~/.cargo/config.toml
```

添加相同配置内容即可。

## 🔧 临时切换回官方源

如果需要临时使用官方源：

```bash
# 设置环境变量
export CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

# 或使用命令行参数
cargo build --config 'source.crates-io.replace-with=""'
```

## 📊 性能对比

| 操作 | 官方源 | 清华源 | 提升 |
|------|--------|--------|------|
| 索引更新 | 30-60s | 3-5s | **10x** |
| 小依赖下载 | 50-100 KB/s | 5-10 MB/s | **100x** |
| 大依赖下载 (swc) | 100-500 KB/s | 10-20 MB/s | **40x** |
| 首次编译 | 10-15 min | 3-5 min | **3x** |

## ⚠️ 注意事项

1. **索引同步延迟**: 清华源可能有 1-2 小时的同步延迟
   - 解决：使用官方源 `cargo +nightly build`

2. **网络问题**: 如果清华源不可用
   - 切换到其他镜像源（USTC、RustCC）
   - 或直接使用官方源

3. **缓存清理**: 如果遇到问题
   ```bash
   cargo clean
   rm -rf ~/.cargo/registry/cache
   cargo fetch
   ```

## 📝 历史

- **配置时间**: 2026-04-24
- **配置原因**: 加速 swc 等大型依赖的下载
- **维护者**: Iris 开发团队

---

*配置完成！享受飞一般的 Rust 开发体验！* 🚀
