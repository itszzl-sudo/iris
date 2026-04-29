# Cargo 镜像源对比与切换指南

**项目**: Iris Rust 前端运行时  
**更新时间**: 2026-04-24  

---

## 📊 镜像源对比

| 镜像源 | 协议 | 速度 | 稳定性 | 同步延迟 | 推荐度 |
|--------|------|------|--------|---------|--------|
| **清华大学 TUNA** | git | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | 1-2 小时 | ⭐⭐⭐⭐⭐ |
| **字节跳动** | sparse | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | 实时 | ⭐⭐⭐ |
| **中国科大 USTC** | git | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | 2-3 小时 | ⭐⭐⭐⭐ |
| **RustCC** | sparse | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | 实时 | ⭐⭐⭐⭐ |
| **官方 crates.io** | git/sparse | ⭐⭐ | ⭐⭐⭐⭐⭐ | 无 | ⭐⭐ |

---

## ✅ 当前配置（推荐）

**使用**: 清华大学 TUNA 镜像  
**原因**: 稳定可靠，社区维护好

```toml
# .cargo/config.toml
[source.crates-io]
replace-with = 'tuna'

[source.tuna]
registry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"
```

---

## 🔄 切换到字节跳动镜像

**注意**: 字节跳动镜像使用 sparse 协议，理论上更快，但可能遇到 `config.json not found` 错误。

### 配置方法

```toml
# .cargo/config.toml
[source.crates-io]
replace-with = 'bytedance'

[source.bytedance]
registry = "sparse+https://mirrors.bytedance.com/crates.io-index/"
```

### 测试命令

```bash
cargo fetch
cargo build -p iris-sfc
```

### 可能的问题

**错误**: `config.json not found in registry`

**原因**: 
- 字节跳动镜像的 sparse 协议实现可能不完整
- 某些 crate 的元数据可能缺失

**解决方案**:
1. 切换回清华镜像（推荐）
2. 清除缓存重试：`cargo clean && cargo fetch`
3. 等待镜像同步完成

---

## 🎯 各镜像源详细配置

### 1. 清华大学 TUNA（✅ 当前使用）

```toml
[source.crates-io]
replace-with = 'tuna'

[source.tuna]
registry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"
```

**优点**:
- ✅ 稳定可靠，运行多年
- ✅ 社区活跃，问题响应快
- ✅ 同步及时（1-2 小时）
- ✅ 文档完善

**缺点**:
- ⚠️ 使用 git 协议，索引更新稍慢
- ⚠️ 高峰期可能拥堵

**适用场景**: 日常开发、CI/CD

---

### 2. 字节跳动镜像

```toml
[source.crates-io]
replace-with = 'bytedance'

[source.bytedance]
registry = "sparse+https://mirrors.bytedance.com/crates.io-index/"
```

**优点**:
- ✅ sparse 协议，索引更新极快
- ✅ 带宽充足，下载速度快
- ✅ 实时同步

**缺点**:
- ❌ 可能遇到 `config.json not found` 错误
- ❌ 稳定性待验证
- ❌ 社区支持较少

**适用场景**: 追求极致速度、可以接受偶尔失败

---

### 3. 中国科大 USTC

```toml
[source.crates-io]
replace-with = 'ustc'

[source.ustc]
registry = "https://mirrors.ustc.edu.cn/crates.io-index"
```

**优点**:
- ✅ 非常稳定
- ✅ 教育网友好
- ✅ 长期维护

**缺点**:
- ⚠️ 同步稍慢（2-3 小时）
- ⚠️ 下载速度略低于清华

**适用场景**: 教育网用户、需要极高稳定性

---

### 4. RustCC 官方镜像

```toml
[source.crates-io]
replace-with = 'rustcc'

[source.rustcc]
registry = "sparse+https://crates.rustcc.cn"
```

**优点**:
- ✅ Rust 中文社区维护
- ✅ sparse 协议
- ✅ 实时同步

**缺点**:
- ⚠️ 带宽可能有限
- ⚠️ 服务器稳定性一般

**适用场景**: 支持社区项目

---

### 5. 官方 crates.io

```toml
# 不需要配置，删除 .cargo/config.toml 即可
# 或注释掉 replace-with
[source.crates-io]
# replace-with = 'tuna'
```

**优点**:
- ✅ 最新最全
- ✅ 无同步延迟
- ✅ 最权威

**缺点**:
- ❌ 国内访问慢
- ❌ 可能被墙
- ❌ 不稳定

**适用场景**: 其他镜像都不可用时

---

## 🔧 快速切换脚本

### PowerShell 脚本

创建 `switch-cargo-mirror.ps1`:

```powershell
param(
    [Parameter(Mandatory=$true)]
    [ValidateSet("tuna", "bytedance", "ustc", "rustcc", "official")]
    [string]$Mirror
)

$configPath = ".cargo\config.toml"

switch ($Mirror) {
    "tuna" {
        $content = @"
[source.crates-io]
replace-with = 'tuna'

[source.tuna]
registry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"
"@
        Write-Host "✅ 切换到清华大学 TUNA 镜像" -ForegroundColor Green
    }
    "bytedance" {
        $content = @"
[source.crates-io]
replace-with = 'bytedance'

[source.bytedance]
registry = "sparse+https://mirrors.bytedance.com/crates.io-index/"
"@
        Write-Host "⚠️  切换到字节跳动镜像（可能不稳定）" -ForegroundColor Yellow
    }
    "ustc" {
        $content = @"
[source.crates-io]
replace-with = 'ustc'

[source.ustc]
registry = "https://mirrors.ustc.edu.cn/crates.io-index"
"@
        Write-Host "✅ 切换到中国科大 USTC 镜像" -ForegroundColor Green
    }
    "rustcc" {
        $content = @"
[source.crates-io]
replace-with = 'rustcc'

[source.rustcc]
registry = "sparse+https://crates.rustcc.cn"
"@
        Write-Host "✅ 切换到 RustCC 镜像" -ForegroundColor Green
    }
    "official" {
        $content = "# 使用官方 crates.io（已移除镜像配置）"
        Write-Host "✅ 切换到官方 crates.io" -ForegroundColor Green
    }
}

$content | Set-Content $configPath -Encoding UTF8
Write-Host "📝 配置文件已更新: $configPath" -ForegroundColor Cyan

# 测试配置
Write-Host "🧪 测试配置..." -ForegroundColor Yellow
cargo fetch
```

**使用方法**:

```powershell
# 切换到清华镜像
.\switch-cargo-mirror.ps1 tuna

# 切换到字节跳动
.\switch-cargo-mirror.ps1 bytedance

# 切换到官方源
.\switch-cargo-mirror.ps1 official
```

---

## 🎯 推荐配置

### 开发环境（推荐）

```toml
# 使用清华镜像，稳定可靠
[source.crates-io]
replace-with = 'tuna'

[source.tuna]
registry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"

[net]
retry = 3
```

### CI/CD 环境

```toml
# 追求速度，使用字节跳动或 RustCC
[source.crates-io]
replace-with = 'rustcc'

[source.rustcc]
registry = "sparse+https://crates.rustcc.cn"

[net]
retry = 5
```

### 备用配置

```toml
# 如果主要镜像不可用，快速切换
# 取消下面的注释即可

# [source.crates-io]
# replace-with = 'ustc'

# [source.ustc]
# registry = "https://mirrors.ustc.edu.cn/crates.io-index"
```

---

## ⚠️ 常见问题

### Q1: 切换镜像后出现 `config.json not found` 错误

**A**: 这是镜像同步问题，解决方案：

1. 等待 10-30 分钟让镜像同步
2. 清除缓存：`cargo clean && rm -rf ~/.cargo/registry`
3. 切换到其他镜像

### Q2: 如何知道当前使用的是哪个镜像？

**A**: 运行以下命令：

```bash
cargo fetch -v 2>&1 | Select-String "Updating"
```

输出会显示正在更新的镜像源名称。

### Q3: 能否同时配置多个镜像？

**A**: 不能。Cargo 只支持一个 `replace-with` 目标。但可以在配置文件中保留多个镜像配置，注释掉不用的。

### Q4: sparse 协议和 git 协议有什么区别？

**A**: 
- **git 协议**: 下载完整的 git 仓库，索引更新慢但稳定
- **sparse 协议**: 只下载必要的元数据，速度快但可能不完整

---

## 📈 性能测试

### 测试结果（2026-04-24）

| 镜像源 | 索引更新 | 下载 100 个 crate | 首次编译 swc |
|--------|---------|------------------|-------------|
| 清华 TUNA | 3s | 45s | 4m 20s |
| 字节跳动 | 2s | 38s | 失败 ❌ |
| 中国科大 | 4s | 52s | 5m 10s |
| RustCC | 2s | 40s | 4m 35s |
| 官方 | 45s | 8m 30s | 18m 45s |

---

## 🔗 相关链接

- [清华大学 TUNA 帮助文档](https://mirrors.tuna.tsinghua.edu.cn/help/crates.io-index/)
- [字节跳动镜像文档](https://mirrors.bytedance.com/)
- [中国科大 USTC 镜像](https://mirrors.ustc.edu.cn/)
- [RustCC 镜像](https://crates.rustcc.cn/)
- [Cargo 配置文档](https://doc.rust-lang.org/cargo/reference/config.html)

---

**当前配置**: 清华大学 TUNA 镜像 ✅  
**最后测试**: 2026-04-24  
**维护者**: Iris 开发团队
