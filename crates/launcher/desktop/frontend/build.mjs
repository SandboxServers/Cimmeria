import { build } from 'esbuild';
import { copyFile, mkdir } from 'node:fs/promises';
await mkdir('dist', {recursive:true});
await build({entryPoints:['src/app.ts'], bundle:true, format:'esm', target:'es2022', minify:true,
  outfile:'dist/app.js', legalComments:'eof'});
await Promise.all(['index.html','style.css'].map(name => copyFile(`ui/${name}`, `dist/${name}`)));
