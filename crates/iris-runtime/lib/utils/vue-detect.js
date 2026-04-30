import { readFileSync, existsSync, readdirSync } from 'fs';
import { resolve, join } from 'path';

export function isVueProjectRoot(dirPath) {
  const result = {
    isVueProject: false,
    confidence: 'none',
    reason: '',
    entryFile: null,
    buildTool: 'unknown',
    vueVersion: 'unknown'
  };

  const packageJsonPath = resolve(dirPath, 'package.json');
  if (existsSync(packageJsonPath)) {
    try {
      const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
      const dependencies = { ...packageJson.dependencies, ...packageJson.devDependencies };
      if (dependencies['vue']) {
        result.vueVersion = dependencies['vue'].includes('^3') || dependencies['vue'].includes('3.') ? '3' : '2';
        result.isVueProject = true;
        result.confidence = 'high';
        result.reason = `Vue ${result.vueVersion} dependency found in package.json`;
      }
      if (dependencies['vite'] || dependencies['@vitejs/plugin-vue']) result.buildTool = 'vite';
      else if (dependencies['webpack'] || dependencies['vue-loader']) result.buildTool = 'webpack';
      if (result.isVueProject) { result.entryFile = findEntryFile(dirPath, result.buildTool); return result; }
    } catch (_) {}
  }

  const vueFiles = findVueFiles(dirPath);
  if (vueFiles.length > 0) {
    result.isVueProject = true;
    result.confidence = vueFiles.length >= 3 ? 'high' : 'medium';
    result.reason = `Found ${vueFiles.length} .vue file(s)${vueFiles.length < 3 ? ' (minimal demo)' : ''}`;
    result.vueVersion = detectVueVersionFromFiles(dirPath, vueFiles);
    result.entryFile = findEntryFile(dirPath, 'unknown');
    return result;
  }

  const vueConfigFiles = ['vite.config.js', 'vite.config.ts', 'vue.config.js'];
  for (const configFile of vueConfigFiles) {
    if (existsSync(resolve(dirPath, configFile))) {
      result.isVueProject = true;
      result.confidence = 'medium';
      result.reason = `Vue config file found: ${configFile}`;
      if (configFile.includes('vite')) result.buildTool = 'vite';
      result.entryFile = findEntryFile(dirPath, result.buildTool);
      return result;
    }
  }

  const indexHtmlPath = resolve(dirPath, 'index.html');
  if (existsSync(indexHtmlPath)) {
    try {
      const htmlContent = readFileSync(indexHtmlPath, 'utf-8');
      if (htmlContent.includes('vue.') || htmlContent.includes('vuejs.org') || htmlContent.includes('cdn.jsdelivr.net/npm/vue')) {
        result.isVueProject = true; result.confidence = 'low';
        result.reason = 'Vue CDN reference found in index.html';
        result.entryFile = findEntryFile(dirPath, 'unknown');
        return result;
      }
    } catch (_) {}
  }

  result.reason = 'No Vue project characteristics detected';
  return result;
}

export function findVueFiles(dirPath, maxDepth = 3) {
  const vueFiles = [];
  function scan(dir, depth) {
    if (depth > maxDepth) return;
    try {
      const entries = readdirSync(dir, { withFileTypes: true });
      for (const entry of entries) {
        const fullPath = join(dir, entry.name);
        if (entry.isDirectory() && !['node_modules', '.git', 'dist', 'build', 'coverage'].includes(entry.name)) scan(fullPath, depth + 1);
        else if (entry.isFile() && entry.name.endsWith('.vue')) vueFiles.push(fullPath);
      }
    } catch (_) {}
  }
  scan(dirPath, 0);
  return vueFiles;
}

function detectVueVersionFromFiles(dirPath, vueFiles) {
  for (const vueFile of vueFiles.slice(0, 3)) {
    try {
      const content = readFileSync(vueFile, 'utf-8');
      if (content.includes('<script setup>') || content.includes('defineProps') || content.includes('defineEmits')) return '3';
      if (content.includes('export default') && content.includes('data()') && !content.includes('<script setup>')) return '2';
    } catch (_) {}
  }
  return 'unknown';
}

export function findEntryFile(dirPath, buildTool) {
  const candidates = ['src/main.js', 'src/main.ts', 'src/index.js', 'src/index.ts', 'main.js', 'main.ts', 'index.js', 'index.ts', 'src/App.vue', 'App.vue'];
  for (const candidate of candidates) { if (existsSync(resolve(dirPath, candidate))) return candidate; }
  if (buildTool === 'nuxt' && existsSync(resolve(dirPath, 'app.vue'))) return 'app.vue';
  return null;
}
