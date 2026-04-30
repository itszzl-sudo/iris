const MIME_TYPES = {
  '.html': 'text/html',
  '.js': 'application/javascript',
  '.mjs': 'application/javascript',
  '.css': 'text/css',
  '.json': 'application/json',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
  '.wasm': 'application/wasm',
};

export function getContentType(path) {
  const ext = path.substring(path.lastIndexOf('.')).toLowerCase();
  return MIME_TYPES[ext] || 'application/octet-stream';
}

export function isJavaScript(path) {
  const ext = path.substring(path.lastIndexOf('.')).toLowerCase();
  return ['.js', '.mjs', '.ts', '.tsx'].includes(ext);
}

export function isSourceFile(path) {
  const ext = path.substring(path.lastIndexOf('.')).toLowerCase();
  return ['.vue', '.ts', '.tsx', '.js', '.jsx', '.mjs'].includes(ext);
}
