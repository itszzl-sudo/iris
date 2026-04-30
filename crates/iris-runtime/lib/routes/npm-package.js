import { readFileSync, existsSync, statSync } from 'fs';
import { resolve, dirname } from 'path';
import { getContentType } from '../utils/mime.js';
import { rewriteBareImports, rewriteNpmRelativeImports, replaceNodeEnv } from '../utils/import-rewriter.js';

export async function npmPackageHandler(req, res, url, projectRoot) {
  const pathname = url.pathname;
  const npmPath = pathname.replace('/@npm/', '');
  const { packageName, subPath } = parseNpmPackagePath(npmPath);
  const nodeModulesPath = resolve(projectRoot, 'node_modules');
  const packagePath = subPath ? resolve(nodeModulesPath, packageName, subPath) : resolve(nodeModulesPath, packageName);

  if (existsSync(packagePath) && statSync(packagePath).isDirectory()) {
    await servePackageEntry(res, packagePath, packageName);
    return;
  }
  if (existsSync(packagePath) && statSync(packagePath).isFile()) {
    await serveFile(res, packagePath, packageName);
    return;
  }
  res.writeHead(404, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ error: 'npm package not found: ' + npmPath }));
}

function parseNpmPackagePath(path) {
  const parts = path.split('/');
  if (parts[0].startsWith('@') && parts.length > 1) {
    return { packageName: parts[0] + '/' + parts[1], subPath: parts.slice(2).join('/') };
  }
  return { packageName: parts[0], subPath: parts.slice(1).join('/') };
}

async function servePackageEntry(res, packagePath, packageName) {
  const pkgJsonPath = resolve(packagePath, 'package.json');
  if (!existsSync(pkgJsonPath)) {
    res.writeHead(404, { 'Content-Type': 'text/plain' });
    res.end('package.json not found');
    return;
  }
  try {
    const pkg = JSON.parse(readFileSync(pkgJsonPath, 'utf-8'));
    const entryFile = pkg.module || pkg.main || 'index.js';
    const entryPath = resolve(packagePath, entryFile);
    if (!existsSync(entryPath)) {
      res.writeHead(404, { 'Content-Type': 'text/plain' });
      res.end('Entry not found: ' + entryFile);
      return;
    }
    let js = readFileSync(entryPath, 'utf-8');
    js = replaceNodeEnv(js);
    js = rewriteBareImports(js, '');
    const entryDir = dirname(entryFile);
    if (entryDir && entryDir !== '.') {
      js = rewriteNpmRelativeImports(js, packageName, entryDir.replace(/\\/g, '/'));
    }
    res.writeHead(200, { 'Content-Type': 'application/javascript' });
    res.end(js);
  } catch (err) {
    res.writeHead(500, { 'Content-Type': 'text/plain' });
    res.end('Failed: ' + err.message);
  }
}

async function serveFile(res, filePath, packageName) {
  try {
    let content = readFileSync(filePath, 'utf-8');
    const ct = getContentType(filePath);
    if (ct === 'application/javascript') {
      content = replaceNodeEnv(content);
      content = rewriteBareImports(content, packageName);
    }
    res.writeHead(200, { 'Content-Type': ct });
    res.end(content);
  } catch (err) {
    res.writeHead(500, { 'Content-Type': 'text/plain' });
    res.end('Failed: ' + err.message);
  }
}
