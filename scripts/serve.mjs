import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { resolve, extname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json', '.css': 'text/css', '.mmd': 'text/plain' };
createServer(async (request, response) => {
  try {
    const path = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    if (path === '/') { response.writeHead(302, { Location: '/examples/web/' }); return response.end(); }
    const file = resolve(root, '.' + path + (path.endsWith('/') ? 'index.html' : ''));
    const normalized = file.slice(root.length).split(sep).join('/');
    if (!file.startsWith(root + sep) ||
        !['/examples/web/', '/bindings/web/', '/dist/package-web/', '/dist/wasm/', '/fixtures/', '/dist/evidence/'].some(prefix => normalized.startsWith(prefix)) ||
        !Object.hasOwn(types, extname(file))) {
      response.writeHead(404); return response.end('Not found');
    }
    const bytes = await readFile(file);
    response.writeHead(200, { 'Content-Type': types[extname(file)] ?? 'application/octet-stream', 'Cache-Control': 'no-store' });
    response.end(bytes);
  } catch { response.writeHead(404); response.end('Not found'); }
}).listen(5191, '127.0.0.1', () => console.log('Pele playground: http://127.0.0.1:5191/examples/web/'));
