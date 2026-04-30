export async function apiHandler(req, res, url, cache, projectRoot, projectInfo) {
  const pathname = url.pathname;

  if (pathname === '/api/project-info') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      isVueProject: projectInfo.isVueProject,
      confidence: projectInfo.confidence,
      reason: projectInfo.reason,
      entryFile: projectInfo.entryFile,
      buildTool: projectInfo.buildTool,
      vueVersion: projectInfo.vueVersion,
      cacheStats: cache.stats(),
    }));
    return true;
  }

  if (pathname === '/api/cache-stats') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(cache.stats()));
    return true;
  }

  if (pathname === '/api/clear-cache' && req.method === 'POST') {
    cache.clear();
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ success: true, message: 'Cache cleared' }));
    return true;
  }

  if (pathname === '/api/project-root') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ root: projectRoot }));
    return true;
  }

  if (pathname === '/api/dependency-issues') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      missing: [],
      ready: [],
      message: 'WASM engine required for full dependency scanning',
    }));
    return true;
  }

  if (pathname === '/api/resolve-dependencies' && req.method === 'POST') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      success: true,
      message: 'Dependency resolution requires WASM engine. Run: npm run build:wasm',
    }));
    return true;
  }

  return false;
}
