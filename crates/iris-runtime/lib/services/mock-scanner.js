/**
 * Iris Mock API Scanner
 * 扫描 Vue 项目源码，自动识别 API 端点
 */

import { readFileSync, existsSync, readdirSync, statSync } from 'fs';
import { resolve, join, relative, extname } from 'path';

// API 调用模式匹配
const API_PATTERNS = [
  // axios 风格: axios.get('/api/...'), axios.post('/api/...', data)
  {
    regex: /axios\.(get|post|put|delete|patch|head|options)\s*\(\s*['"`](\/[^'"`]+)['"`]/g,
    methodIndex: 1,
    pathIndex: 2,
  },
  // fetch: fetch('/api/...'), fetch('/api/...', { method: 'POST' })
  {
    regex: /fetch\s*\(\s*['"`](\/[^'"`]+)['"`]/g,
    pathIndex: 1,
  },
  // $http: this.$http.get(...), $http.post(...)
  {
    regex: /\$http\.(get|post|put|delete|patch|head)\s*\(\s*['"`](\/[^'"`]+)['"`]/g,
    methodIndex: 1,
    pathIndex: 2,
  },
  // VueUse useFetch: useFetch('/api/...')
  {
    regex: /useFetch\s*\(\s*['"`](\/[^'"`]+)['"`]/g,
    pathIndex: 1,
  },
  // useAxios: useAxios('/api/...')
  {
    regex: /useAxios\s*\(\s*['"`](\/[^'"`]+)['"`]/g,
    pathIndex: 1,
  },
  // api 模块调用: api.getUsers(), api.fetchOrders()
  // 捕获 api.xxx() 形式的调用
  {
    regex: /\bapi\s*\.\s*(\w+)\s*\(/g,
    pathIndex: 1,
    isApiObject: true,
  },
  // await $api.xxx()
  {
    regex: /\$\s*api\s*\.\s*(\w+)\s*\(/g,
    pathIndex: 1,
    isApiObject: true,
  },
  // request 封装: request('/api/...'), request.get('/api/...')
  {
    regex: /request(?:\.(get|post|put|delete|patch))?\s*\(\s*['"`](\/[^'"`]+)['"`]/g,
    methodIndex: 1,
    pathIndex: 2,
  },
  // service 调用: userService.getList(), userService.add()
  {
    regex: /(\w+)Service\.(getList|getDetail|add|update|remove|delete|save|fetch|list|get|create)\s*\(/g,
    pathIndex: 1,
    isServiceCall: true,
  },
  // import 中的 api 路径: import { ... } from '@/api/user'
  {
    regex: /from\s+['"`](?:\/src\/)?(?:@\/)?api\/([^'"`.]+)/g,
    pathIndex: 1,
    isImport: true,
  },
];

// 常见的 RESTful 方法名映射
const METHOD_MAP = {
  getList: 'GET',
  getDetail: 'GET',
  add: 'POST',
  create: 'POST',
  update: 'PUT',
  save: 'POST',
  remove: 'DELETE',
  delete: 'DELETE',
  del: 'DELETE',
  list: 'GET',
  get: 'GET',
  fetch: 'GET',
};

// 忽略的目录
const IGNORE_DIRS = new Set([
  'node_modules', '.git', 'dist', 'build', 'coverage',
  '.vscode', '.idea', '__pycache__', '.iris-cache',
]);

// 扫描的文件扩展名
const SCAN_EXTENSIONS = new Set(['.vue', '.js', '.jsx', '.ts', '.tsx']);

export class MockScanner {
  constructor(rootDir) {
    this.rootDir = rootDir;
    this.srcDir = this._findSrcDir();
  }

  _findSrcDir() {
    const candidates = ['src', 'client/src', 'frontend/src', 'web/src', 'app/src'];
    for (const dir of candidates) {
      const fullPath = resolve(this.rootDir, dir);
      if (existsSync(fullPath) && statSync(fullPath).isDirectory()) {
        return fullPath;
      }
    }
    // 如果没有 src 目录，直接用根目录
    return this.rootDir;
  }

  /**
   * 执行扫描，返回检测到的端点列表
   * @returns {Array<{path: string, method: string, file: string, line: number, context: string}>}
   */
  scan() {
    const results = [];
    const files = this._collectFiles(this.srcDir);

    for (const file of files) {
      try {
        const content = readFileSync(file, 'utf-8');
        const lines = content.split('\n');
        const fileEndpoints = this._scanFile(content, lines, file);
        results.push(...fileEndpoints);
      } catch (err) {
        // 跳过无法读取的文件
      }
    }

    // 去重和合并
    return this._deduplicate(results);
  }

  /**
   * 递归收集可扫描的文件
   */
  _collectFiles(dir, maxDepth = 5) {
    const files = [];
    if (maxDepth <= 0) return files;

    try {
      const entries = readdirSync(dir, { withFileTypes: true });
      for (const entry of entries) {
        const fullPath = join(dir, entry.name);
        if (entry.isDirectory()) {
          if (!IGNORE_DIRS.has(entry.name)) {
            files.push(...this._collectFiles(fullPath, maxDepth - 1));
          }
        } else if (entry.isFile() && SCAN_EXTENSIONS.has(extname(entry.name))) {
          files.push(fullPath);
        }
      }
    } catch (_) {
      // 跳过无法访问的目录
    }

    return files;
  }

  /**
   * 扫描单个文件
   */
  _scanFile(content, lines, filePath) {
    const endpoints = [];
    const relPath = relative(this.rootDir, filePath).replace(/\\/g, '/');

    for (const pattern of API_PATTERNS) {
      const matches = content.matchAll(pattern.regex);
      for (const match of matches) {
        // 找到匹配所在的行号
        const matchIndex = match.index;
        let lineNum = 0;
        let charCount = 0;
        for (let i = 0; i < lines.length; i++) {
          charCount += lines[i].length + 1; // +1 for newline
          if (charCount > matchIndex) {
            lineNum = i + 1;
            break;
          }
        }

        if (pattern.isImport) {
          // import 路径：推断 API 路径
          const apiName = match[pattern.pathIndex];
          endpoints.push({
            path: '/api/' + apiName,
            method: 'GET',
            file: relPath,
            line: lineNum,
            context: lines[lineNum - 1]?.trim() || '',
            confidence: 'medium',
          });
        } else if (pattern.isApiObject) {
          // api.xxx() 形式：xxx 可能是方法名
          const methodName = match[pattern.pathIndex];
          endpoints.push({
            path: '/api/' + this._methodNameToPath(methodName),
            method: METHOD_MAP[methodName] || 'GET',
            file: relPath,
            line: lineNum,
            context: lines[lineNum - 1]?.trim() || '',
            confidence: 'low',
          });
        } else if (pattern.isServiceCall) {
          // service 调用
          const serviceName = match[pattern.pathIndex];
          const actionName = match[pattern.methodIndex];
          endpoints.push({
            path: '/api/' + serviceName,
            method: METHOD_MAP[actionName] || 'GET',
            file: relPath,
            line: lineNum,
            context: lines[lineNum - 1]?.trim() || '',
            confidence: 'low',
          });
        } else if (pattern.methodIndex && pattern.pathIndex) {
          // axios/$http/request 风格：有明确方法和路径
          let rawPath = match[pattern.pathIndex];
          // 清理查询参数
          const qIdx = rawPath.indexOf('?');
          if (qIdx !== -1) rawPath = rawPath.substring(0, qIdx);
          endpoints.push({
            path: rawPath,
            method: (match[pattern.methodIndex] || 'get').toUpperCase(),
            file: relPath,
            line: lineNum,
            context: lines[lineNum - 1]?.trim() || '',
            confidence: 'high',
          });
        } else {
          // fetch 风格：需要额外寻找 method
          let rawPath = match[pattern.pathIndex];
          // 清理查询参数
          const qIdx = rawPath.indexOf('?');
          if (qIdx !== -1) rawPath = rawPath.substring(0, qIdx);
          const method = this._inferMethod(lines, lineNum);
          endpoints.push({
            path: rawPath,
            method: method,
            file: relPath,
            line: lineNum,
            context: lines[lineNum - 1]?.trim() || '',
            confidence: 'high',
          });
        }
      }
    }

    return endpoints;
  }

  /**
   * 从 fetch 调用上下文推断 HTTP 方法
   */
  _inferMethod(lines, lineNum) {
    // 检查当前行或附近行是否包含 method
    const start = Math.max(0, lineNum - 3);
    const end = Math.min(lines.length, lineNum + 3);
    for (let i = start; i < end; i++) {
      const methodMatch = lines[i].match(/method:\s*['"`](GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS)['"`"]/i);
      if (methodMatch) return methodMatch[1].toUpperCase();
    }
    return 'GET';
  }

  /**
   * 将驼峰方法名转为 API 路径
   * getUsers -> users
   * getUserList -> user-list
   * createOrder -> orders
   */
  _methodNameToPath(name) {
    // 去除常见动词前缀
    let path = name.replace(/^(get|create|add|update|delete|remove|fetch|save|list|del)_?/i, '');
    if (!path) path = name;

    // 驼峰转连字符
    path = path.replace(/([A-Z])/g, '-$1').toLowerCase();
    // 首字符处理
    if (path.startsWith('-')) path = path.slice(1);

    // 复数化（简单规则）
    if (!path.endsWith('s') && !path.endsWith('list')) {
      path = path + 's';
    }

    return path;
  }

  /**
   * 去重并合并结果
   */
  _deduplicate(endpoints) {
    const map = new Map();

    for (const ep of endpoints) {
      const key = ep.method + ' ' + ep.path;
      if (map.has(key)) {
        const existing = map.get(key);
        // 保留更高置信度的结果
        const confOrder = { high: 3, medium: 2, low: 1 };
        if (confOrder[ep.confidence] > confOrder[existing.confidence]) {
          map.set(key, ep);
        }
      } else {
        map.set(key, ep);
      }
    }

    return Array.from(map.values());
  }

  /**
   * 从扫描结果生成 Mock 配置
   * @param {Array} endpoints - 扫描结果
   * @returns {object} Mock 配置
   */
  generateMockConfig(endpoints) {
    const endpointsConfig = {};

    for (const ep of endpoints) {
      const path = ep.path;
      if (!endpointsConfig[path]) {
        endpointsConfig[path] = {};
      }

      // 推断是否为列表/详情/操作
      const isList = ep.method === 'GET' && !ep.path.match(/\/(\d+)$/);
      const isDetail = ep.method === 'GET' && !!ep.path.match(/\/(\d+)$/);
      const isMutation = ['POST', 'PUT', 'PATCH', 'DELETE'].includes(ep.method);

      if (!endpointsConfig[path][ep.method]) {
        if (isList) {
          endpointsConfig[path][ep.method] = {
            status: 200,
            data: {
              type: 'paginated',
              pageSize: 20,
              total: 100,
            },
          };
        } else if (isDetail) {
          endpointsConfig[path][ep.method] = {
            status: 200,
            data: { type: 'object' },
          };
        } else if (isMutation) {
          endpointsConfig[path][ep.method] = {
            status: 200,
            data: { type: 'object' },
          };
        }
      }
    }

    return {
      mock: {
        enabled: true,
        autoScan: true,
        endpoints: endpointsConfig,
      },
    };
  }

  /**
   * 获取扫描统计信息
   */
  getScanSummary(endpoints) {
    const methods = {};
    const paths = new Set();

    for (const ep of endpoints) {
      methods[ep.method] = (methods[ep.method] || 0) + 1;
      paths.add(ep.path);
    }

    return {
      total: endpoints.length,
      uniquePaths: paths.size,
      methods,
      sourceFiles: new Set(endpoints.map(e => e.file)).size,
    };
  }
}
