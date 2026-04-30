export async function vueModuleHandler(req, res, url, cache) {
  const pathname = url.pathname;
  const relativePath = pathname.replace('/@vue/', '');
  try {
    const module = await cache.getOrCompile(relativePath);
    res.writeHead(200, { 'Content-Type': 'application/json', 'Cache-Control': 'no-cache' });
    res.end(JSON.stringify({ code: module.script, styles: module.styles || [], deps: module.deps || [] }));
  } catch (error) {
    res.writeHead(500, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: error.message }));
  }
}
