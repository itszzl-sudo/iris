/**
 * Iris Mock API Route Handler
 * 拦截 /api/ 请求并返回 Mock 数据
 */

import chalk from 'chalk';
import { MockEngine } from '../services/mock-engine.js';
import { MockScanner } from '../services/mock-scanner.js';
import { existsSync, readFileSync } from 'fs';
import { resolve } from 'path';

const MOCK_CONFIG_FILES = ['iris.mock.json', 'mock.config.json'];

export class MockApiHandler {
  constructor(root, options = {}) {
    this.root = root;
    this.enabled = options.enabled !== false;
    this.autoScan = options.autoScan !== false;
    this.delay = options.delay || 0;
    this.endpoints = {};
    this.scannedEndpoints = [];
    this.engine = null;
    this.initialized = false;
  }

  /**
   * 初始化 Mock 系统
   * 加载配置 -> 扫描项目 -> 初始化引擎
   */
  async initialize() {
    if (this.initialized) return;

    // 1. 加载用户自定义配置
    const userConfig = this._loadUserConfig();

    // 2. 自动扫描项目
    if (this.autoScan) {
      await this._scanProject();
    }

    // 3. 合并配置
    const mergedConfig = this._mergeConfig(userConfig);

    // 4. 初始化引擎
    this.engine = new MockEngine(mergedConfig);
    this.initialized = true;

    // 5. 输出日志
    this._printSummary();

    return mergedConfig;
  }

  /**
   * 处理 Mock API 请求
   * @returns {boolean} 是否已处理
   */
  async handle(req, res, url) {
    if (!this.enabled || !this.initialized) {
      return false;
    }

    const pathname = url.pathname;
    const method = req.method;

    // 调用引擎生成数据
    const query = Object.fromEntries(url.searchParams.entries());

    // 检查是否有配置
    if (!this.engine.hasEndpoint(pathname, method) && !this._isAutoScanable(pathname)) {
      // 没有匹配的端点，跳过
      return false;
    }

    try {
      // 模拟网络延迟
      const delayMs = this._getDelay(pathname, method);
      if (delayMs > 0) {
        await new Promise(resolve => setTimeout(resolve, delayMs));
      }

      const result = this.engine.generate(pathname, method, query, null);

      // 记录请求
      this._logRequest(method, pathname, result);

      res.writeHead(200, {
        'Content-Type': 'application/json',
        'Access-Control-Allow-Origin': '*',
        'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, PATCH, OPTIONS',
        'Access-Control-Allow-Headers': 'Content-Type, Authorization',
        'X-Mock-Enabled': 'true',
      });
      res.end(JSON.stringify(result));
      return true;

    } catch (error) {
      console.error(chalk.red('  [Mock] Error:'), error.message);
      return false;
    }
  }

  /**
   * 处理 OPTIONS 预检请求
   */
  handleOptions(req, res) {
    if (!this.enabled) return false;
    res.writeHead(204, {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, PATCH, OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type, Authorization',
      'Access-Control-Max-Age': '86400',
    });
    res.end();
    return true;
  }

  /**
   * 加载用户配置
   */
  _loadUserConfig() {
    for (const configFile of MOCK_CONFIG_FILES) {
      const configPath = resolve(this.root, configFile);
      if (existsSync(configPath)) {
        try {
          const raw = readFileSync(configPath, 'utf-8');
          const config = JSON.parse(raw);
          if (config && config.mock) {
            console.log(chalk.cyan('  [Mock] Loaded config:'), configFile);
            return config.mock;
          }
        } catch (err) {
          console.error(chalk.yellow('  [Mock] Failed to parse'), configFile, err.message);
        }
      }
    }
    return {};
  }

  /**
   * 扫描项目中的 API 调用
   */
  async _scanProject() {
    try {
      const scanner = new MockScanner(this.root);
      this.scannedEndpoints = scanner.scan();
      const summary = scanner.getScanSummary(this.scannedEndpoints);

      if (this.scannedEndpoints.length > 0) {
        console.log(chalk.cyan(`  [Mock] Scanned ${summary.sourceFiles} files, found ${summary.uniquePaths} API endpoints:`));
        const seen = new Set();
        for (const ep of this.scannedEndpoints) {
          const key = ep.method + ' ' + ep.path;
          if (!seen.has(key)) {
            seen.add(key);
            const confColor = ep.confidence === 'high' ? chalk.green : ep.confidence === 'medium' ? chalk.yellow : chalk.gray;
            console.log(`        ${confColor(ep.method.padEnd(6))} ${chalk.cyan(ep.path)} ${chalk.dim('← ' + ep.file + ':' + ep.line)}`);
          }
        }
      } else {
        console.log(chalk.dim('  [Mock] No API endpoints detected in project files'));
      }
    } catch (err) {
      console.error(chalk.yellow('  [Mock] Scan error:'), err.message);
    }
  }

  /**
   * 合并用户配置和扫描结果
   */
  _mergeConfig(userConfig) {
    const config = {
      enabled: this.enabled,
      autoScan: this.autoScan,
      delay: userConfig.delay || this.delay,
      endpoints: { ...(userConfig.endpoints || {}) },
    };

    // 将扫描结果作为默认配置（不覆盖用户自定义）
    if (this.autoScan && this.scannedEndpoints.length > 0) {
      const scanner = new MockScanner(this.root);
      const scannedConfig = scanner.generateMockConfig(this.scannedEndpoints);
      if (scannedConfig.mock && scannedConfig.mock.endpoints) {
        for (const [path, methods] of Object.entries(scannedConfig.mock.endpoints)) {
          if (!config.endpoints[path]) {
            config.endpoints[path] = methods;
          }
        }
      }
    }

    return config;
  }

  /**
   * 是否为可自动扫描的路径
   */
  _isAutoScanable(pathname) {
    return this.scannedEndpoints.some(ep => ep.path === pathname);
  }

  /**
   * 获取延迟时间
   */
  _getDelay(pathname, method) {
    const ep = this.engine._findEndpoint(pathname, method);
    if (ep && ep.delay !== undefined) return ep.delay;
    return this.delay || 0;
  }

  /**
   * 记录请求
   */
  _logRequest(method, path, result) {
    const status = result ? chalk.green('200') : chalk.red('404');
    const dataSize = result ? JSON.stringify(result).length : 0;
    console.log(chalk.dim(`  [Mock] ${method} ${path} → ${status} (${dataSize}B)`));
  }

  /**
   * 打印启动摘要
   */
  _printSummary() {
    const endpointCount = Object.keys(this.engine.customSchemas || {}).length;
    if (endpointCount > 0) {
      const delayStr = this.delay > 0 ? `, delay: ${this.delay}ms` : '';
      console.log(chalk.green(`  [Mock] Mock API Server ready: ${endpointCount} endpoints configured${delayStr}`));
      console.log(chalk.dim('  [Mock] Create iris.mock.json in project root to customize'));
    } else {
      console.log(chalk.dim('  [Mock] No mock endpoints configured'));
    }
    console.log();
  }
}
