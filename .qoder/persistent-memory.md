# 持久化记忆

> 自动导出于 2026-04-30。下次启动时加载此文件以恢复持久化记忆。

---

## 一、项目介绍

### 1.1 彩虹守护进程核心功能
项目新增彩虹守护进程（iris-jetcrab-daemon）核心功能：
- 桌面悬浮🌈图标（80x80透明置顶窗口），支持鼠标拖拽移动；
- 拖拽时生成渐变彩虹柔滑轨迹 + 星光粒子游走消散特效，静置时图标呼吸律动；
- 图标智能定位：Vue项目渲染成功时移至网页内部，失败时回归桌面右下角；
- 双击图标打开嵌入式Web管理面板，支持配置HTTP/Mock端口、Vue工程目录列表、默认启动项目、桌面图标显隐开关；
- 管理面板提供完整REST API（/api/status、/api/config、/api/projects等）；
- 兼容系统：若不支持🌈emoji，则自动降级使用嵌入式PNG图片。

### 1.2 守护进程核心功能与配置管理规范
项目守护进程具备以下核心能力：
- 启动时自动续传未完成的下载任务（npm包和AI模型）
- 浮动图标支持鼠标悬停显示"I 💞 iris"文字、右键弹出菜单（含退出、设置、下载进度条）
- 管理面板中"NPM包管理器配置"已升级为"Iris内置包管理器配置"，新增本地存储目录配置项，目录变更时弹窗提示是否拷贝原有数据
- 本地模型管理界面中，模型仓库与模型文件设为只读，移除缓存目录配置项；运行设备、Temperature、Max Tokens采用系统自适应默认值（如CUDA/Vulkan/CPU自动检测、Temperature=0.15、MaxTokens=4096）
- 全局配置管理支持除项目目录外所有字段设置默认值，并提供分区级（general/ai/npm/mock）"恢复默认值"按钮

### 1.3 管理面板四大配置能力
管理面板需支持以下四大配置能力：
- AI云厂商模型服务：配置服务商（如OpenAI/Anthropic）、API Key、模型名、自定义Endpoint；
- AI本地模型：管理HuggingFace模型仓库、GGUF文件名、下载进度/状态/控制（启动/停止）；
- 内置NPM包管理器：配置registry镜像地址与代理；
- Mock API Server：配置启用开关、端口、模拟延迟毫秒数。

### 1.4 运行状态显示实际端口并移除操作按钮
运行状态面板中显示HTTP服务器、Mock API Server和守护进程的实际监听端口，不再提供启动/停止/刷新等操作按钮。

### 1.5 Vue渲染状态迁移至项目列表
Vue渲染状态从运行状态面板移出，改在项目列表中每个项目项旁显示渲染成功/失败状态（如绿色圆点表示成功）。

### 1.6 AI本地模型文件UI重构
AI本地模型管理模块更名为'AI本地模型文件'，隐藏模型仓库字段，'开始下载/停止下载'按钮合并为'暂停下载/继续下载'单按钮，并新增下载完成度百分比显示，已下载完成时隐藏该按钮。

### 1.7 跨平台浏览器检测排序规则
浏览器自动检测排序规则：Chrome始终排第一；第二优先级按操作系统区分——Windows为Edge、macOS为Safari、Linux为Firefox；其余浏览器保持原有顺序。

### 1.8 浏览器分身多工作空间与单标签页约束
内嵌浏览器分身功能要求：每个工作空间严格对应一个独立浏览器标签页；系统支持同时打开多个工作空间，各工作空间的浏览器窗口相互隔离、互不干扰。

### 1.9 提示语悬停交互逻辑
项目支持基于图标alpha通道的精确悬停检测：仅当鼠标位于图标64×64区域内且对应像素alpha值大于30时显示提示语，鼠标离开该有效区域则提示语消失。

---

## 二、环境配置

### 2.1 内嵌浏览器分身选择逻辑与配置
内嵌浏览器分身采用优先级策略：默认首选Chrome，若系统未安装则回退至Edge，再未安装则尝试其他浏览器（如Firefox）。支持通过配置项`preferred_browser`指定首选浏览器类型（auto/chrome/edge/firefox），当配置的浏览器未安装时自动执行默认回退逻辑。

### 2.2 三服务共用端口范围与自动换端口
HTTP服务器、Mock API Server和控制面板共用同一端口范围，三者启动时若端口被占用，均自动在配置的端口范围内寻找可用端口启动。

### 2.3 daemon端口自动检测与动态分配
守护进程启动时自动检测端口占用，若指定端口被占用，则在配置的端口范围内（默认起始端口19999，范围大小500）查找可用端口并绑定。

### 2.4 SFC编译流程环境变量配置与日志集成
项目编译流程支持环境变量驱动的动态配置：
- IRIS_SOURCE_MAP 控制 TypeScript 编译器是否生成 source_map
- IRIS_CACHE_CAPACITY 设置 SFC 缓存容量（默认 100）
- IRIS_CACHE_ENABLED 控制缓存是否启用（默认 true）
同时，compile() 函数已集成缓存逻辑，并自动记录结构化编译时间日志（含耗时、是否命中缓存等字段）。

---

## 三、重要决策

### 3.1 浏览器名称展示格式-简称+签名组合
- **场景**: 配置面板中所有浏览器相关信息的展示
- **决策**: 浏览器显示格式统一为：简称首字母大写 + 全名括号形式
  - Chrome (Google Chrome)、Edge (Microsoft Edge)、Firefox (Mozilla Firefox)
  - 简称取 `id` 字段，全名取 `name` 字段
- **范围**: 所有需要向用户展示浏览器名称的前端界面位置

### 3.2 Rust CLI npm包采用预编译二进制分发策略
- **场景**: Rust CLI工具通过npm分发时决定安装方式
- **决策**: 采用预编译二进制分发方案：
  - 安装脚本完全移除cargo build调用
  - 从GitHub Releases优先下载，失败后自动回退至Gitee
  - 支持Windows/macOS/Linux及x64/arm64多平台
  - package.json中声明os/cpu字段
- **范围**: 所有面向终端用户的Rust CLI npm包

### 3.3 内部Rust项目镜像配置与发布策略决策
- **场景**: 内部使用Rust项目的镜像源配置与发布策略协同
- **决策**:
  - 日常开发：启用清华镜像源（tuna）
  - 发布准备：临时注释镜像配置，使用`cargo publish --registry crates-io`
  - 不执行发布操作，仅验证发布流程（dry-run）
- **范围**: 所有内部使用的Rust crate项目

### 3.4 代码转换工具集成优先选用高层Compiler API
- **场景**: 集成swc等代码转换工具时的API选型
- **决策**: 优先选择高层Compiler API，具备简单易用、自动配置合并、内置类型转换等优势
- **范围**: 所有需要集成代码编译/转换能力的Rust项目

### 3.5 swc API选型决策原则
swc API选型应基于具体需求在高层Compiler API和底层API之间做合理选择，不预设倾向，需权衡简洁性、可控性和维护成本。

---

## 四、编程实践规范

### 4.1 Vue事件处理器编译规范
Vue模板事件处理器（如@click、@input）在编译为render函数时，其属性值必须是函数引用或函数数组，禁止使用字符串字面量（如onClick: "handleLogout"），否则Vue运行时会报错。

### 4.2 运行环境编码自适应输出规范
程序需要根据运行环境的编码自动适配输出文本格式，优先检测IRIS_DEMO_FORCE_ASCII环境变量，其次检查IRIS_CODE_PAGE、WT_SESSION、LANG、OUTPUTENCODING等环境变量，最终默认使用UTF-8编码，确保在不同终端（如PowerShell/Windows Terminal/CMD）下中文显示正常。

### 4.3 文件监听器日志集成规范
（相关文件监听器的日志集成规范，使用tracing库）

---

## 五、常见问题经验

### 5.1 守护进程API端口就绪需显式重启保障
iris-jetcrab-daemon守护进程虽处于运行状态（tasklist可见），其API端口（19999）可能尚未就绪或已失效，导致集成测试HTTP请求超时或失败。此时需强制重启守护进程以确保API服务监听正常。

### 5.2 Rust模块开发前需先搜索确认类型与函数存在性
在向现有Rust模块添加新功能前，必须先使用grep_code搜索确认相关类型（如RenderStats）和函数（如render）是否已存在，避免重复定义导致编译错误。

### 5.3 PowerShell UTF-8乱码需双编码设置
PowerShell环境下UTF-8乱码需同时设置chcp 65001和$OutputEncoding。

### 5.4 Rust源文件编码兼容性说明
Rust源文件需确保UTF-8编码，避免GBK编码导致编译错误。

### 5.5 fontdue 0.8栅格化方法名与参数变更
fontdue库版本升级后栅格化方法名和参数发生变化，需注意版本适配。

### 5.6 其他常见问题
- Windows Rust增量编译权限错误根因是Defender锁定.working文件
- iris-runtime已无Rust代码，生产构建由iris-engine或iris-jetcrab-engine承担
- npm二进制分发无需Base64编码

---

## 六、技术栈

### 6.1 swc 集成采用高层 Compiler API
swc集成使用高层 Compiler API进行TypeScript编译。

### 6.2 TypeScript编译器技术选型：swc
项目TypeScript编译器采用swc。

### 6.3 iris-runtime二进制本地打包策略
iris-runtime npm包采用本地打包策略：将预编译的iris-cli二进制文件直接放入binaries/目录，安装时由install.js脚本从本地复制至bin/目录，彻底消除网络下载依赖。

---

## 七、计划经验

### 7.1 管理面板功能扩展方案
- **设计原则**: 配置字段按功能分组，全部添加`#[serde(default)]`保证向后兼容
- **API路由**: `/api/{domain}/{action}`命名规范，GET/PUT用于配置读写，POST用于异步操作
- **模板变量**: 统一前缀`cfg{CamelCase}`和`{SELECTED_*}`，避免冲突
- **PNG图标**: 四级路径搜索策略，失败自动回退到emoji字体渲染

### 7.2 Mock API Server设计方案
- 在dev-server中新增Mock API子系统
- 核心模块：mock-scanner.js（API调用扫描）、mock-engine.js（数据生成）、mock-api.js（路由处理）
- 三优先级匹配：自定义 > 自动推断 > 默认

### 7.3 Rust crate模块迁移的标准化计划
（Rust模块拆分与迁移的标准流程）

---

## 八、任务总结经验

### 8.1 iris-jetcrab-daemon多模块新功能验证总结
自动续传下载、悬停提示、右键菜单、管理面板重构及配置默认值验证全部通过。

### 8.2 实现内嵌浏览器分身功能及MockTableDemo修复
后端新增浏览器检测、配置、启动、关闭、状态查询等API；前端增加浏览器配置卡片和工作空间浏览器管理区域；修复MockTableDemo.vue语法错误。

### 8.3 AI服务商DeepSeek添加失败的根本原因与修复路径
（DeepSeek API集成相关问题的根因分析和修复方案）

### 8.4 Iris Engine里程碑确认与SFC Phase A实现
（Iris Engine项目里程碑和SFC第一阶段实现）

### 8.5 iris-jetcrab-cli 实现 Vue 项目浏览器端渲染（代理模式）
（Vue项目浏览器端渲染的代理模式实现）

### 8.6 修复管理面板项目列表加载失败bug
（管理面板项目列表加载失败问题的修复）

### 8.7 iris-app热重载功能完整实现
在iris-app中集成tracing订阅者、实现SFC热重载逻辑、添加集成测试。

---

## 九、学习技能

### 9.1 守护进程功能测试流程
1. 运行测试脚本：`powershell -ExecutionPolicy Bypass -File test-daemon.ps1`
2. 测试覆盖项：进程状态、API连通性、无客户端检测、管理页面、确认页、WebSocket跟踪、代码完整性、编译验证

---

## 十、额外关键概念

### 10.1 项目所属 npm 组织
项目所属 npm 组织为 `irisverse`。

### 10.2 双运行时架构
项目采用 Node.js + WASM 双运行时架构，正在从纯Rust向混合架构演进。
