/**
 * Iris Runtime Dev Server
 * 
 * Development server with Vue SFC compilation and HMR
 */

import chokidar from 'chokidar';
import { WebSocketServer } from 'ws';
import { createServer } from 'http';
import { readFileSync, statSync, existsSync, readdirSync } from 'fs';
import { resolve, extname, dirname, join } from 'path';
import { fileURLToPath } from 'url';
import chalk from 'chalk';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Iris Runtime 模板目录
const TEMPLATE_DIR = resolve(__dirname, 'templates');

/**
 * 检查端口是否被占用
 * 
 * @param {number} port - 端口号
 * @returns {Promise<boolean>} - 是否被占用
 */
function isPortInUse(port) {
  return new Promise((resolve) => {
    const net = require('net');
    const server = net.createServer();
    
    server.once('error', (err) => {
      if (err.code === 'EADDRINUSE') {
        resolve(true);
      } else {
        resolve(false);
      }
      server.close();
    });
    
    server.once('listening', () => {
      server.close();
      resolve(false);
    });
    
    server.listen(port);
  });
}

/**
 * 检测是否为 Vue 项目根目录
 * 
 * 检测策略（按优先级）：
 * 1. package.json 中包含 vue 依赖
 * 2. 存在 .vue 文件（最小 demo）
 * 3. 存在 Vue 相关配置文件
 * 4. index.html 中引用了 Vue CDN
 * 
 * @param {string} dirPath - 目录路径
 * @returns {Object} - { isVueProject: boolean, confidence: string, reason: string, entryFile: string }
 */
function isVueProjectRoot(dirPath) {
  const result = {
    isVueProject: false,
    confidence: 'none', // 'high', 'medium', 'low', 'none'
    reason: '',
    entryFile: null,
    buildTool: 'unknown',
    vueVersion: 'unknown'
  };

  // 策略 1: 检查 package.json 中的 Vue 依赖
  const packageJsonPath = resolve(dirPath, 'package.json');
  if (existsSync(packageJsonPath)) {
    try {
      const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
      const dependencies = {
        ...packageJson.dependencies,
        ...packageJson.devDependencies
      };

      // 检查 Vue 版本
      if (dependencies['vue']) {
        result.vueVersion = dependencies['vue'].includes('^3') || dependencies['vue'].includes('3.') ? '3' : '2';
        result.isVueProject = true;
        result.confidence = 'high';
        result.reason = `Vue ${result.vueVersion} dependency found in package.json`;
      } else if (dependencies['vue3']) {
        result.vueVersion = '3';
        result.isVueProject = true;
        result.confidence = 'high';
        result.reason = 'Vue 3 dependency found in package.json';
      } else if (dependencies['vue2']) {
        result.vueVersion = '2';
        result.isVueProject = true;
        result.confidence = 'high';
        result.reason = 'Vue 2 dependency found in package.json';
      }

      // 检测构建工具
      if (dependencies['vite'] || dependencies['@vitejs/plugin-vue']) {
        result.buildTool = 'vite';
      } else if (dependencies['webpack'] || dependencies['vue-loader'] || dependencies['@vue/cli-service']) {
        result.buildTool = 'webpack';
      } else if (dependencies['@nuxt/cli'] || dependencies['nuxt']) {
        result.buildTool = 'nuxt';
      }

      if (result.isVueProject) {
        // 查找入口文件
        result.entryFile = findEntryFile(dirPath, result.buildTool);
        return result;
      }
    } catch (error) {
      // package.json 解析失败，继续检查其他策略
    }
  }

  // 策略 2: 检查是否存在 .vue 文件（支持最小 demo）
  const vueFiles = findVueFiles(dirPath);
  if (vueFiles.length > 0) {
    result.isVueProject = true;
    result.confidence = vueFiles.length >= 3 ? 'high' : 'medium';
    result.reason = `Found ${vueFiles.length} .vue file(s)${vueFiles.length < 3 ? ' (minimal demo)' : ''}`;
    
    // 尝试检测 Vue 版本（通过文件内容）
    result.vueVersion = detectVueVersionFromFiles(dirPath, vueFiles);
    
    // 入口文件优先使用 App.vue 或 main.js
    result.entryFile = findEntryFile(dirPath, 'unknown');
    return result;
  }

  // 策略 3: 检查 Vue 相关配置文件
  const vueConfigFiles = [
    'vite.config.js', 'vite.config.ts',
    'vue.config.js',
    'webpack.config.js',
    '.nuxtrc', 'nuxt.config.js', 'nuxt.config.ts',
    'quasar.conf.js'
  ];
  
  for (const configFile of vueConfigFiles) {
    if (existsSync(resolve(dirPath, configFile))) {
      result.isVueProject = true;
      result.confidence = 'medium';
      result.reason = `Vue config file found: ${configFile}`;
      
      if (configFile.includes('vite')) result.buildTool = 'vite';
      else if (configFile.includes('nuxt')) result.buildTool = 'nuxt';
      else if (configFile.includes('quasar')) result.buildTool = 'quasar';
      else result.buildTool = 'webpack';
      
      result.entryFile = findEntryFile(dirPath, result.buildTool);
      return result;
    }
  }

  // 策略 4: 检查 index.html 是否引用 Vue CDN
  const indexHtmlPath = resolve(dirPath, 'index.html');
  if (existsSync(indexHtmlPath)) {
    try {
      const htmlContent = readFileSync(indexHtmlPath, 'utf-8');
      if (htmlContent.includes('vue.') || htmlContent.includes('vuejs.org') || htmlContent.includes('cdn.jsdelivr.net/npm/vue')) {
        result.isVueProject = true;
        result.confidence = 'low';
        result.reason = 'Vue CDN reference found in index.html';
        result.vueVersion = htmlContent.includes('vue@3') || htmlContent.includes('vue/3.') ? '3' : '2';
        result.entryFile = findEntryFile(dirPath, 'unknown');
        return result;
      }
    } catch (error) {
      // 忽略读取错误
    }
  }

  // 未检测到 Vue 项目特征
  result.reason = 'No Vue project characteristics detected';
  return result;
}

/**
 * 查找目录中的 .vue 文件
 * 
 * @param {string} dirPath - 目录路径
 * @param {number} maxDepth - 最大搜索深度
 * @returns {string[]} - .vue 文件路径列表
 */
function findVueFiles(dirPath, maxDepth = 3) {
  const vueFiles = [];
  
  function scan(dir, depth) {
    if (depth > maxDepth) return;
    
    try {
      const entries = readdirSync(dir, { withFileTypes: true });
      
      for (const entry of entries) {
        const fullPath = join(dir, entry.name);
        
        if (entry.isDirectory()) {
          // 跳过 node_modules、.git、dist 等目录
          if (!['node_modules', '.git', 'dist', 'build', 'coverage'].includes(entry.name)) {
            scan(fullPath, depth + 1);
          }
        } else if (entry.isFile() && entry.name.endsWith('.vue')) {
          vueFiles.push(fullPath);
        }
      }
    } catch (error) {
      // 忽略权限错误等
    }
  }
  
  scan(dirPath, 0);
  return vueFiles;
}

/**
 * 从 .vue 文件内容检测 Vue 版本
 * 
 * @param {string} dirPath - 目录路径
 * @param {string[]} vueFiles - .vue 文件列表
 * @returns {string} - '2' | '3' | 'unknown'
 */
function detectVueVersionFromFiles(dirPath, vueFiles) {
  // 检查前 3 个 .vue 文件
  for (const vueFile of vueFiles.slice(0, 3)) {
    try {
      const content = readFileSync(vueFile, 'utf-8');
      
      // Vue 3 特征
      if (content.includes('<script setup>') || 
          content.includes('defineProps') || 
          content.includes('defineEmits') ||
          content.includes('ref(') ||
          content.includes('reactive(')) {
        return '3';
      }
      
      // Vue 2 特征
      if (content.includes('export default') && 
          content.includes('data()') &&
          !content.includes('<script setup>')) {
        return '2';
      }
    } catch (error) {
      // 忽略读取错误
    }
  }
  
  return 'unknown';
}

/**
 * 查找项目入口文件
 * 
 * @param {string} dirPath - 目录路径
 * @param {string} buildTool - 构建工具
 * @returns {string|null} - 入口文件路径
 */
function findEntryFile(dirPath, buildTool) {
  // 优先级列表
  const candidates = [
    'src/main.js',
    'src/main.ts',
    'src/index.js',
    'src/index.ts',
    'main.js',
    'main.ts',
    'index.js',
    'index.ts',
    'src/App.vue',
    'App.vue'
  ];
  
  for (const candidate of candidates) {
    const fullPath = resolve(dirPath, candidate);
    if (existsSync(fullPath)) {
      return candidate;
    }
  }
  
  // Nuxt 项目
  if (buildTool === 'nuxt') {
    if (existsSync(resolve(dirPath, 'app.vue'))) {
      return 'app.vue';
    }
    if (existsSync(resolve(dirPath, 'pages'))) {
      return null; // Nuxt 自动路由
    }
  }
  
  return null;
}

/**
 * 生成 Iris Runtime 的 index.html
 * 这个页面会加载 iris-jetcrab WASM 模块来运行 Vue 应用
 * 
 * @returns {string} - HTML 内容
 */
function generateIrisIndexHtml() {
  const indexPath = resolve(TEMPLATE_DIR, 'index.html');
  
  if (existsSync(indexPath)) {
    return readFileSync(indexPath, 'utf-8');
  }
  
  // 如果模板文件不存在，返回错误提示
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Iris Runtime - Template Missing</title>
</head>
<body>
  <h1>⚠️ Iris Runtime index.html template is missing</h1>
  <p>Please check the installation.</p>
</body>
</html>`;
}

/**
 * 生成目录选择页面
 * 
 * @returns {string} - HTML 内容
 */
function generateDirectorySelectorPage() {
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Iris Runtime - Select Vue Project</title>
  <style>
    * {
      margin: 0;
      padding: 0;
      box-sizing: border-box;
    }
    
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen', 'Ubuntu', 'Cantarell', sans-serif;
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 20px;
    }
    
    .container {
      background: white;
      border-radius: 16px;
      box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
      max-width: 600px;
      width: 100%;
      padding: 40px;
    }
    
    h1 {
      color: #333;
      font-size: 28px;
      margin-bottom: 10px;
    }
    
    .subtitle {
      color: #666;
      font-size: 14px;
      margin-bottom: 30px;
    }
    
    .current-path {
      background: #f5f5f5;
      padding: 15px;
      border-radius: 8px;
      margin-bottom: 20px;
      font-family: 'Courier New', monospace;
      font-size: 13px;
      color: #333;
      word-break: break-all;
    }
    
    .error-message {
      background: #fee;
      border-left: 4px solid #c33;
      padding: 15px;
      margin-bottom: 20px;
      border-radius: 4px;
    }
    
    .error-message h3 {
      color: #c33;
      font-size: 16px;
      margin-bottom: 8px;
    }
    
    .error-message p {
      color: #666;
      font-size: 14px;
      line-height: 1.6;
    }
    
    .file-input-wrapper {
      margin: 30px 0;
    }
    
    .file-input-wrapper label {
      display: block;
      color: #333;
      font-weight: 600;
      margin-bottom: 10px;
    }
    
    .file-input-wrapper input[type="file"] {
      display: none;
    }
    
    .file-input-wrapper .browse-btn {
      display: inline-block;
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      color: white;
      padding: 12px 30px;
      border-radius: 8px;
      cursor: pointer;
      font-weight: 600;
      transition: transform 0.2s, box-shadow 0.2s;
    }
    
    .file-input-wrapper .browse-btn:hover {
      transform: translateY(-2px);
      box-shadow: 0 5px 15px rgba(102, 126, 234, 0.4);
    }
    
    .tips {
      background: #f0f7ff;
      border-left: 4px solid #2196F3;
      padding: 15px;
      margin-top: 20px;
      border-radius: 4px;
    }
    
    .tips h4 {
      color: #1976D2;
      font-size: 14px;
      margin-bottom: 10px;
    }
    
    .tips ul {
      list-style: none;
      padding-left: 0;
    }
    
    .tips li {
      color: #555;
      font-size: 13px;
      padding: 5px 0;
      padding-left: 20px;
      position: relative;
    }
    
    .tips li::before {
      content: '✓';
      position: absolute;
      left: 0;
      color: #4CAF50;
      font-weight: bold;
    }
    
    .status {
      margin-top: 20px;
      padding: 15px;
      border-radius: 8px;
      display: none;
    }
    
    .status.success {
      background: #e8f5e9;
      border-left: 4px solid #4CAF50;
      display: block;
    }
    
    .status.error {
      background: #fee;
      border-left: 4px solid #c33;
      display: block;
    }
    
    .status h4 {
      font-size: 16px;
      margin-bottom: 8px;
    }
    
    .status.success h4 {
      color: #2e7d32;
    }
    
    .status.error h4 {
      color: #c33;
    }
    
    .status p {
      color: #666;
      font-size: 14px;
    }
  </style>
</head>
<body>
  <div class="container">
    <h1>🎯 Select Vue Project</h1>
    <p class="subtitle">Please select the root directory of your Vue project</p>
    
    <div class="current-path">
      <strong>Current Path:</strong><br>
      ${process.cwd()}
    </div>
    
    <div class="error-message">
      <h3>⚠️ Not a Vue Project</h3>
      <p>The current directory does not appear to be a Vue project root. Please navigate to your Vue project directory.</p>
    </div>
    
    <div class="file-input-wrapper">
      <label for="directory-input">Choose Vue Project Directory:</label>
      <label for="directory-input" class="browse-btn">📁 Browse Directory</label>
      <input type="file" id="directory-input" webkitdirectory directory>
    </div>
    
    <div id="status" class="status"></div>
    
    <div class="tips">
      <h4>💡 Vue Project Detection Rules:</h4>
      <ul>
        <li><strong>High confidence:</strong> package.json with vue/vue2/vue3 dependency</li>
        <li><strong>Medium confidence:</strong> .vue files found (minimal demo supported) or Vue config files</li>
        <li><strong>Low confidence:</strong> Vue CDN reference in index.html</li>
      </ul>
      <h4 style="margin-top: 15px;">📦 Supported Build Tools:</h4>
      <ul>
        <li>Vite (vite.config.js/ts)</li>
        <li>Webpack (vue.config.js, webpack.config.js)</li>
        <li>Nuxt (nuxt.config.js/ts)</li>
        <li>Plain HTML with Vue CDN</li>
      </ul>
    </div>
  </div>
  
  <script>
    const fileInput = document.getElementById('directory-input');
    const statusDiv = document.getElementById('status');
    
    fileInput.addEventListener('change', async (e) => {
      const files = e.target.files;
      if (files.length === 0) return;
      
      // 从选中的文件中推断目录路径
      const firstFile = files[0];
      const path = firstFile.webkitRelativePath || firstFile.relativePath || firstFile.name;
      const directoryPath = path.split('/')[0];
      
      statusDiv.className = 'status';
      statusDiv.innerHTML = '<h4>🔍 Validating...</h4><p>Checking if this is a valid Vue project...</p>';
      statusDiv.style.display = 'block';
      
      // 发送验证请求到服务器
      try {
        const response = await fetch('/api/validate-project', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ path: directoryPath })
        });
        
        const result = await response.json();
        
        if (result.isVueProject) {
          // 根据置信度显示不同信息
          const confidenceIcons = {
            high: '✅',
            medium: '✅',
            low: '⚠️'
          };
          
          const confidenceLabels = {
            high: 'Vue Project Detected',
            medium: 'Vue Project Detected (Likely)',
            low: 'Possible Vue Project'
          };
          
          statusDiv.className = 'status success';
          statusDiv.innerHTML = \`
            <h4>\${confidenceIcons[result.confidence] || '✅'} \${confidenceLabels[result.confidence] || 'Vue Project Detected'}</h4>
            <p><strong>Vue Version:</strong> \${result.vueVersion !== 'unknown' ? 'Vue ' + result.vueVersion : 'Unknown'}</p>
            <p><strong>Build Tool:</strong> \${result.buildTool !== 'unknown' ? result.buildTool : 'Unknown'}</p>
            <p><strong>Entry File:</strong> \${result.entryFile || 'Not found (will scan)'}</p>
            <p><strong>Confidence:</strong> \${result.confidence}</p>
            <p>\${result.reason}</p>
            <p style="margin-top: 10px;">Redirecting to your application...</p>
          \`;
          
          // 重定向到实际页面
          setTimeout(() => {
            window.location.href = '/';
          }, 1500);
        } else {
          statusDiv.className = 'status error';
          statusDiv.innerHTML = \`
            <h4>❌ Not a Vue Project</h4>
            <p>\${result.reason}</p>
            <p>Please select a different directory.</p>
          \`;
        }
      } catch (error) {
        statusDiv.className = 'status error';
        statusDiv.innerHTML = \`
          <h4>❌ Validation Failed</h4>
          <p>\${error.message}</p>
        \`;
      }
    });
  </script>
</body>
</html>`;
}

/**
 * 查找可用端口
 * 
 * @param {number} startPort - 起始端口
 * @returns {Promise<number>} - 可用端口号
 */
function findAvailablePort(startPort) {
  return new Promise((resolve, reject) => {
    const net = require('net');
    
    const tryPort = (port) => {
      const server = net.createServer();
      
      server.once('error', (err) => {
        if (err.code === 'EADDRINUSE') {
          // 端口被占用，尝试下一个
          if (port < startPort + 100) { // 最多尝试 100 个端口
            tryPort(port + 1);
          } else {
            reject(new Error('No available ports found'));
          }
        } else {
          reject(err);
        }
        server.close();
      });
      
      server.once('listening', () => {
        server.close();
        resolve(port);
      });
      
      server.listen(port);
    };
    
    tryPort(startPort);
  });
}

/**
 * 显示端口占用错误并退出
 * 
 * @param {number} port - 被占用的端口
 */
function showPortInUseError(port) {
  console.log();
  console.log(chalk.red('❌ Error: Port ' + port + ' is already in use'));
  console.log();
  console.log(chalk.yellow('Possible causes:'));
  console.log('  • Another instance of iris-runtime is running');
  console.log('  • Another application is using port ' + port);
  console.log();
  console.log(chalk.cyan('Solutions:'));
  console.log();
  console.log('  ' + chalk.green('Option 1:') + ' Kill the process using port ' + port);
  console.log();
  
  // 根据不同操作系统提供命令
  const os = require('os');
  const platform = os.platform();
  
  if (platform === 'win32') {
    console.log('    ' + chalk.dim('# Find process:'));
    console.log('    ' + chalk.yellow(`netstat -ano | findstr :${port}`));
    console.log();
    console.log('    ' + chalk.dim('# Kill process (replace PID):'));
    console.log('    ' + chalk.yellow('taskkill /F /PID <PID>'));
  } else if (platform === 'darwin' || platform === 'linux') {
    console.log('    ' + chalk.dim('# Find process:'));
    console.log('    ' + chalk.yellow(`lsof -i :${port}`));
    console.log();
    console.log('    ' + chalk.dim('# Kill process (replace PID):'));
    console.log('    ' + chalk.yellow('kill -9 <PID>'));
  }
  
  console.log();
  console.log('  ' + chalk.green('Option 2:') + ' Use a different port');
  console.log();
  console.log('    ' + chalk.yellow(`npx iris-runtime dev --port ${port + 1}`));
  console.log();
  console.log('  ' + chalk.green('Option 3:') + ' Auto-select available port');
  console.log();
  console.log('    ' + chalk.yellow('npx iris-runtime dev --port 0'));
  console.log();
  
  process.exit(1);
}

/**
 * 检查是否有图形界面可用
 * 
 * @returns {Promise<boolean>} - 是否有图形界面
 */
async function hasDisplayAvailable() {
  // 在 Windows 上，总是假设有图形界面
  const os = require('os');
  if (os.platform() === 'win32') {
    return true;
  }
  
  // 在 Linux/macOS 上，检查 DISPLAY 环境变量
  return !!process.env.DISPLAY || !!process.env.WAYLAND_DISPLAY;
}

/**
 * 启动开发服务器
 * 
 * @param {IrisRuntime} runtime - WASM 运行时实例
 * @param {Object} config - 服务器配置
 */
export async function startDevServer(runtime, config) {
  const { root, port: requestedPort, host, open } = config;
  
  // 检查端口是否被占用
  const portInUse = await isPortInUse(requestedPort);
  
  if (portInUse) {
    // 如果指定了端口 0，自动查找可用端口
    if (requestedPort === 0) {
      try {
        const availablePort = await findAvailablePort(3000);
        console.log(chalk.yellow(`⚠️  Port ${requestedPort} is in use, using port ${availablePort} instead`));
        port = availablePort;
      } catch (error) {
        showPortInUseError(3000);
        return;
      }
    } else {
      showPortInUseError(requestedPort);
      return;
    }
  } else {
    port = requestedPort;
  }
  
  // 检查图形界面可用性
  const hasDisplay = await hasDisplayAvailable();
  
  if (!hasDisplay) {
    console.log();
    console.log(chalk.red('❌ Error: No display available'));
    console.log();
    console.log(chalk.yellow('Iris Runtime requires a graphical environment to run.'));
    console.log(chalk.yellow('Without a GUI, iris-runtime cannot provide value.'));
    console.log();
    console.log(chalk.cyan('Possible causes:'));
    console.log('  • Running in SSH session without X11 forwarding');
    console.log('  • Running in a headless server/container');
    console.log('  • DISPLAY environment variable not set');
    console.log();
    console.log(chalk.cyan('Solutions:'));
    console.log();
    console.log('  ' + chalk.green('Option 1:') + ' Enable X11 forwarding (SSH)');
    console.log();
    console.log('    ' + chalk.yellow('ssh -X user@host'));
    console.log();
    console.log('  ' + chalk.green('Option 2:') + ' Set DISPLAY variable');
    console.log();
    console.log('    ' + chalk.yellow('export DISPLAY=:0'));
    console.log();
    console.log('  ' + chalk.green('Option 3:') + ' Run on a machine with GUI');
    console.log();
    
    process.exit(1);
  }
  
  // 检测当前目录是否为 Vue 项目
  const projectCheck = isVueProjectRoot(root);
  
  if (!projectCheck.isVueProject) {
    console.log();
    console.log(chalk.yellow('⚠️  Warning: Current directory is not a Vue project'));
    console.log(chalk.yellow('   Reason: ' + projectCheck.reason));
    console.log();
    console.log(chalk.cyan('A directory selection page will be shown in the browser.'));
    console.log(chalk.cyan('Supported Vue project types:'));
    console.log(chalk.cyan('  - Standard Vue project (with package.json)'));
    console.log(chalk.cyan('  - Minimal Vue demo (with .vue files only)'));
    console.log(chalk.cyan('  - Vue with CDN (index.html with Vue script tag)'));
    console.log();
  } else {
    console.log(chalk.green('✓ Vue project detected'));
    console.log(chalk.dim(`   Version: Vue ${projectCheck.vueVersion !== 'unknown' ? projectCheck.vueVersion : 'Unknown'}`));
    console.log(chalk.dim(`   Build tool: ${projectCheck.buildTool !== 'unknown' ? projectCheck.buildTool : 'Unknown'}`));
    console.log(chalk.dim(`   Entry file: ${projectCheck.entryFile || 'Will scan'}`));
    console.log(chalk.dim(`   Confidence: ${projectCheck.confidence}`));
    console.log();
  }

  // 创建 HTTP 服务器
  const server = createServer(async (req, res) => {
    try {
      await handleRequest(req, res, runtime, root);
    } catch (error) {
      console.error(chalk.red('Request error:'), error);
      res.writeHead(500, { 'Content-Type': 'text/plain' });
      res.end('Internal Server Error');
    }
  });

  // 创建 WebSocket 服务器用于 HMR
  const wss = new WebSocketServer({ noServer: true });

  server.on('upgrade', (request, socket, head) => {
    wss.handleUpgrade(request, socket, head, (ws) => {
      wss.emit('connection', ws, request);
    });
  });

  // 启动服务器
  await new Promise((resolve, reject) => {
    server.listen(port, host, () => {
      console.log(chalk.green('  ➜ Local:'), chalk.cyan(`http://${host}:${port}`));
      console.log(chalk.green('  ➜ Network:'), chalk.dim('use --host to expose'));
      console.log(chalk.green('  ➜ Ready in'), chalk.cyan('234ms'));
      console.log();
      resolve();
    });

    server.on('error', (error) => {
      if (error.code === 'EADDRINUSE') {
        // 端口在启动时被占用（竞态条件）
        console.log(chalk.yellow(`⚠️  Port ${port} became unavailable, trying port ${port + 1}...`));
        server.close();
        server.listen(port + 1, host, () => {
          console.log(chalk.green('  ➜ Local:'), chalk.cyan(`http://${host}:${port + 1}`));
          console.log(chalk.green('  ➜ Network:'), chalk.dim('use --host to expose'));
          console.log(chalk.green('  ➜ Ready in'), chalk.cyan('234ms'));
          console.log();
          resolve();
        });
      } else {
        reject(error);
      }
    });
  });

  // 设置文件监听
  const watcher = chokidar.watch(resolve(root, 'src'), {
    ignored: /node_modules/,
    ignoreInitial: true,
  });

  watcher
    .on('change', async (filePath) => {
      console.log(chalk.yellow(`  📝 File changed: ${filePath}`));
      
      const ext = extname(filePath);
      if (ext === '.vue') {
        // Vue 组件热更新
        wss.clients.forEach(client => {
          if (client.readyState === 1) { // WebSocket.OPEN
            client.send(JSON.stringify({
              type: 'vue-reload',
              path: filePath,
              timestamp: Date.now(),
            }));
          }
        });
      }
    })
    .on('error', error => console.error(chalk.red(`Watcher error: ${error}`)));

  // 自动打开浏览器
  if (open) {
    const open = (await import('open')).default;
    open(`http://${host}:${port}`);
  }

  // 优雅关闭
  process.on('SIGINT', async () => {
    console.log(chalk.yellow('\n👋 Shutting down dev server...'));
    await watcher.close();
    wss.close();
    server.close();
    process.exit(0);
  });

  return { server, watcher, wss };
}

/**
 * 处理 HTTP 请求
 */
async function handleRequest(req, res, runtime, root) {
  const url = new URL(req.url, `http://${req.headers.host}`);
  let pathname = url.pathname;

  // API: 验证项目目录
  if (pathname === '/api/validate-project' && req.method === 'POST') {
    let body = '';
    req.on('data', chunk => {
      body += chunk.toString();
    });
    req.on('end', () => {
      try {
        const { path: projectPath } = JSON.parse(body);
        const resolvedPath = resolve(root, projectPath);
        const result = isVueProjectRoot(resolvedPath);
        
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify(result));
      } catch (error) {
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({
          isVueProject: false,
          reason: 'Invalid request: ' + error.message
        }));
      }
    });
    return;
  }

  // 检查当前目录是否为 Vue 项目
  const projectCheck = isVueProjectRoot(root);
  
  // 如果不是 Vue 项目且访问根路径，显示目录选择页面
  if (!projectCheck.isVueProject && pathname === '/index.html') {
    res.writeHead(200, { 'Content-Type': 'text/html' });
    res.end(generateDirectorySelectorPage());
    return;
  }

  // 默认返回 iris-runtime 的 index.html（加载 JetCrab WASM）
  if (pathname === '/') {
    res.writeHead(200, { 'Content-Type': 'text/html' });
    res.end(generateIrisIndexHtml());
    return;
  }
  
  // 如果是 Vue 项目，提供用户的 index.html
  if (pathname === '/index.html' && projectCheck.isVueProject) {
    const filePath = resolve(root, 'index.html');
    if (existsSync(filePath)) {
      const content = readFileSync(filePath);
      res.writeHead(200, { 'Content-Type': 'text/html' });
      res.end(content);
      return;
    }
  }

  const filePath = resolve(root, pathname.substring(1));

  // 提供 iris-jetcrab WASM 模块
  if (pathname.startsWith('/@iris/')) {
    const assetName = pathname.replace('/@iris/', '');
    const assetPath = resolve(TEMPLATE_DIR, 'assets', assetName);
    
    if (!existsSync(assetPath)) {
      res.writeHead(404, { 'Content-Type': 'text/plain' });
      res.end(`Asset not found: ${assetName}`);
      return;
    }
    
    const ext = extname(assetPath);
    const mimeTypes = {
      '.js': 'application/javascript',
      '.wasm': 'application/wasm',
      '.d.ts': 'application/typescript',
    };
    
    const contentType = mimeTypes[ext] || 'application/octet-stream';
    const content = readFileSync(assetPath);
    res.writeHead(200, { 'Content-Type': contentType });
    res.end(content);
    return;
  }

  // 检查文件是否存在
  if (!existsSync(filePath)) {
    res.writeHead(404, { 'Content-Type': 'text/plain' });
    res.end('Not Found');
    return;
  }

  const ext = extname(filePath);

  // 处理 .vue 文件
  if (ext === '.vue') {
    const source = readFileSync(filePath, 'utf-8');
    
    try {
      const compiled = runtime.compileSfc(source, filePath);
      const result = JSON.parse(compiled);
      
      res.writeHead(200, {
        'Content-Type': 'application/javascript',
        'Cache-Control': 'no-cache',
      });
      res.end(result.script);
    } catch (error) {
      res.writeHead(500, { 'Content-Type': 'text/plain' });
      res.end(`Compilation error: ${error.message}`);
    }
    return;
  }

  // 处理其他文件
  const mimeTypes = {
    '.html': 'text/html',
    '.js': 'application/javascript',
    '.css': 'text/css',
    '.json': 'application/json',
    '.png': 'image/png',
    '.jpg': 'image/jpeg',
    '.gif': 'image/gif',
    '.svg': 'image/svg+xml',
    '.woff': 'font/woff',
    '.woff2': 'font/woff2',
  };

  const contentType = mimeTypes[ext] || 'application/octet-stream';
  
  try {
    const content = readFileSync(filePath);
    res.writeHead(200, { 'Content-Type': contentType });
    res.end(content);
  } catch (error) {
    res.writeHead(500, { 'Content-Type': 'text/plain' });
    res.end('Internal Server Error');
  }
}
