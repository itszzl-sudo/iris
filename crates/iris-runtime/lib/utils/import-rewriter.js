const RELATIVE_IMPORT_RE = /(?:from\s+['"]|import\(['"])(\.\.?\/)([^'"]+)['"]/g;
const STATIC_IMPORT_RE = /from\s+['"]([^'"]+)['"]/g;
const DYNAMIC_IMPORT_RE = /import\(['"]([^'"]+)['"]\)/g;

export function rewriteNpmRelativeImports(script, packageName, subdir) {
  const subdirNormalized = subdir.replace(/\\/g, '/');
  return script.replace(RELATIVE_IMPORT_RE, (match, relativePrefix, importPath, offset, str) => {
    const isDynamic = str.slice(Math.max(0, offset - 7), offset).includes('import(');
    let resolvedPath;
    if (relativePrefix === './') {
      resolvedPath = `/@npm/${packageName}/${subdirNormalized}/${importPath}`;
    } else {
      const parts = subdirNormalized.split('/'); parts.pop();
      const parentDir = parts.join('/');
      resolvedPath = parentDir ? `/@npm/${packageName}/${parentDir}/${importPath}` : `/@npm/${packageName}/${importPath}`;
    }
    return isDynamic ? `import('${resolvedPath}')` : `from '${resolvedPath}'`;
  });
}

export function rewriteBareImports(script, modulePath) {
  const sourceDir = resolveSourceDir(modulePath);
  const resolveToSrc = (importPath) => {
    if (!modulePath) {
      if (importPath.startsWith('./') || importPath.startsWith('../') || importPath.startsWith('/')) return importPath;
      return `/@npm/${importPath}`;
    }
    if (importPath.startsWith('./')) return `/src/${importPath.slice(2)}`;
    if (importPath.startsWith('../')) {
      if (!sourceDir) return `/src/${importPath.split('/').pop()}`;
      const parts = [...sourceDir.split('/')];
      for (const seg of importPath.split('/')) { if (seg === '..') parts.pop(); else if (seg !== '.') parts.push(seg); }
      return `/src/${parts.join('/')}`;
    }
    if (importPath.startsWith('/')) return importPath;
    return `/@npm/${importPath}`;
  };

  let result = script.replace(STATIC_IMPORT_RE, (match, importPath) => `from '${resolveToSrc(importPath)}'`);
  result = result.replace(DYNAMIC_IMPORT_RE, (match, importPath) => {
    if (importPath.startsWith('./') || importPath.startsWith('../')) return `import('${resolveToSrc(importPath)}')`;
    return match;
  });
  return result;
}

function resolveSourceDir(modulePath) {
  if (!modulePath) return '';
  const lastSlash = modulePath.lastIndexOf('/');
  if (lastSlash > 0) return modulePath.slice(0, lastSlash);
  if (!modulePath.includes('.') && !modulePath.includes('/') && !modulePath.includes('\\')) return modulePath;
  return '';
}

export function replaceNodeEnv(script) {
  return script.replace(/process\.env\.NODE_ENV/g, "'development'");
}

export function generateStyleInjectCode(styles) {
  let code = '';
  for (const style of styles) {
    code += '\n\n/* Injected Styles */\n';
    code += '(function(){var s=document.createElement("style");';
    code += 's.setAttribute("data-iris-hmr","");';
    code += 's.textContent=`' + style.code.replace(/`/g, '\\`') + '`;';
    code += 'document.head.appendChild(s)})();\n';
  }
  return code;
}
