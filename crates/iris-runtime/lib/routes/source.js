export async function sourceFileHandler(req, res, url, cache) {
  const pathname = url.pathname;
  const relativePath = pathname.replace('/src/', '');
  try {
    const module = await cache.getOrCompile(relativePath);
    const contentType = pathname.endsWith('.css') ? 'text/css' : 'application/javascript';
    res.writeHead(200, { 'Content-Type': contentType, 'Cache-Control': 'no-cache' });
    res.end(module.finalCode || module.script);
  } catch (error) {
    console.error('[Source] Error for', pathname, error.message);
    res.writeHead(500, { 'Content-Type': 'text/plain' });
    res.end('Failed to compile module: ' + error.message);
  }
}
