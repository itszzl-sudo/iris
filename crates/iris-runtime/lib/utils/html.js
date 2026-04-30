import { readFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const TEMPLATE_DIR = resolve(__dirname, '..', 'templates');

export function generateIrisIndexHtml(projectRoot) {
  let html;
  if (projectRoot) {
    const projectIndex = resolve(projectRoot, 'index.html');
    if (existsSync(projectIndex)) {
      html = readFileSync(projectIndex, 'utf-8');
    }
  }
  if (!html) {
    const templatePath = resolve(TEMPLATE_DIR, 'index.html');
    if (existsSync(templatePath)) {
      html = readFileSync(templatePath, 'utf-8');
    } else {
      html = '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Iris Runtime</title></head><body><div id="app"></div><script type="module" src="/src/main.js"></script></body></html>';
    }
  }

  // 检查是否已有 favicon 链接
  const hasFavicon = html.includes('rel="icon"') || html.includes("rel='icon'") || html.includes('rel=icon');
  
  // 如果没有 favicon，注入彩虹 emoji favicon
  if (!hasFavicon) {
    const faviconLink = '<link rel="icon" type="image/svg+xml" href="/__iris-favicon.svg">';
    const headEnd = html.indexOf('</head>');
    if (headEnd !== -1) {
      html = html.slice(0, headEnd) + faviconLink + html.slice(headEnd);
    }
  }

  return html;
}

export function generateDirectorySelectorPage() {
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Iris Runtime - Select Vue Project</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); min-height: 100vh; display: flex; align-items: center; justify-content: center; padding: 20px; }
    .container { background: white; border-radius: 16px; box-shadow: 0 20px 60px rgba(0,0,0,0.3); max-width: 600px; width: 100%; padding: 40px; }
    h1 { color: #333; font-size: 28px; margin-bottom: 10px; }
    .subtitle { color: #666; font-size: 14px; margin-bottom: 30px; }
    .current-path { background: #f5f5f5; padding: 15px; border-radius: 8px; margin-bottom: 20px; font-family: 'Courier New', monospace; font-size: 13px; color: #333; word-break: break-all; }
    .error-message { background: #fee; border-left: 4px solid #c33; padding: 15px; margin-bottom: 20px; border-radius: 4px; }
    .error-message h3 { color: #c33; font-size: 16px; margin-bottom: 8px; }
    .error-message p { color: #666; font-size: 14px; line-height: 1.6; }
    .tips { background: #f0f7ff; border-left: 4px solid #2196F3; padding: 15px; border-radius: 4px; }
    .tips h4 { color: #1976D2; font-size: 14px; margin-bottom: 10px; }
    .tips ul { list-style: none; padding-left: 0; }
    .tips li { color: #555; font-size: 13px; padding: 3px 0; padding-left: 20px; position: relative; }
    .tips li::before { content: '✓'; position: absolute; left: 0; color: #4CAF50; font-weight: bold; }
  </style>
</head>
<body>
  <div class="container">
    <h1>🎯 Select Vue Project</h1>
    <p class="subtitle">Current directory is not a Vue project</p>
    <div class="current-path"><strong>Current Path:</strong><br>${process.cwd()}</div>
    <div class="error-message"><h3>⚠️ Not a Vue Project</h3><p>The current directory does not appear to be a Vue project root.</p></div>
    <div class="tips">
      <h4>💡 Supported Vue Project Types:</h4>
      <ul>
        <li>Standard Vue project (with package.json + vue dependency)</li>
        <li>Minimal Vue demo (with .vue files only)</li>
        <li>Vue with CDN (index.html with Vue script tag)</li>
      </ul>
    </div>
  </div>
</body>
</html>`;
}

export function generateResolvePageHtml() {
  const templatePath = resolve(TEMPLATE_DIR, 'resolve.html');
  if (existsSync(templatePath)) return readFileSync(templatePath, 'utf-8');
  return '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Resolve Dependencies</title></head><body><h1>Resolve Dependencies</h1></body></html>';
}
