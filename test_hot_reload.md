# 文件热更新监听器测试

这个目录用于测试 Iris 的文件监听功能。

修改此文件或创建新的 .vue/.js/.css 文件，Iris 应该会在控制台输出：
```
🔥 File change detected: "path/to/file" (vue)
```

## 触发通道满警告测试（可选）

在 1 秒内修改 500+ 个文件（或在 `poll_file_changes()` 中设置断点），应该看到：
- 控制台：`⚠️ File watcher channel full, events may be lost.`
- **弹窗**：`File watcher event queue is full!` 警告对话框（仅显示一次）
