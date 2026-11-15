import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, posix, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const PDFIUM_VERSION = '158.0.8125.0';
const PDFIUM_RELEASE = 'chromium/8125';
const LICENSES_DIR = 'src-tauri/pdfium/licenses';
const RUST_OUT = 'src-tauri/src/licenses_data.rs';
const MD_OUT = 'THIRD_PARTY_NOTICES.md';
const BUNDLED = 'Bundled in PDFium';
const L = `${LICENSES_DIR}/`;

const MANIFEST = [
  { title: 'Miso', kind: 'MIT', origin: 'este repositorio', path: 'LICENSE' },
  { title: 'Literata', kind: 'OFL-1.1', origin: 'https://github.com/googlefonts/literata', path: 'src/assets/fonts/OFL.txt' },
  { title: 'pdfium-binaries', kind: 'MIT', origin: 'https://github.com/bblanchon/pdfium-binaries', path: 'src-tauri/pdfium/LICENSE' },
  { title: 'PDFium', kind: 'BSD-3-Clause', origin: 'https://pdfium.googlesource.com/pdfium/', path: `${L}pdfium.txt` },
  { title: 'Abseil', kind: 'Apache-2.0', origin: BUNDLED, path: `${L}abseil.txt` },
  { title: 'Anti-Grain Geometry 2.3', kind: 'Permissive notice', origin: BUNDLED, path: `${L}agg23.txt` },
  { title: 'Dragonbox (LLVM)', kind: 'Apache-2.0 WITH LLVM-exception', origin: BUNDLED, path: `${L}dragonbox-Apache2-LLVM.txt` },
  { title: 'Dragonbox (Boost)', kind: 'BSL-1.0', origin: BUNDLED, path: `${L}dragonbox-Boost.txt` },
  { title: 'fast_float', kind: 'MIT', origin: BUNDLED, path: `${L}fast_float.txt` },
  { title: 'FreeType', kind: 'FTL', origin: BUNDLED, path: `${L}freetype.txt` },
  { title: 'HarfBuzz', kind: 'MIT-Modern-Variant', origin: BUNDLED, path: `${L}harfbuzz.txt` },
  { title: 'Highway', kind: 'Apache-2.0', origin: BUNDLED, path: `${L}highway.txt` },
  { title: 'ICU', kind: 'Unicode-3.0', origin: BUNDLED, path: `${L}icu.txt` },
  { title: 'Little CMS', kind: 'MIT', origin: BUNDLED, path: `${L}lcms.txt` },
  { title: 'libjpeg-turbo (IJG)', kind: 'IJG', origin: BUNDLED, path: `${L}libjpeg_turbo.ijg` },
  { title: 'libjpeg-turbo (BSD)', kind: 'BSD-3-Clause AND Zlib', origin: BUNDLED, path: `${L}libjpeg_turbo.md` },
  { title: 'OpenJPEG', kind: 'BSD-2-Clause', origin: BUNDLED, path: `${L}libopenjpeg.txt` },
  { title: 'libpng', kind: 'libpng-2.0', origin: BUNDLED, path: `${L}libpng.txt` },
  { title: 'LLVM libc', kind: 'Apache-2.0 WITH LLVM-exception', origin: BUNDLED, path: `${L}llvm-libc.txt` },
  { title: 'simdutf', kind: 'MIT', origin: BUNDLED, path: `${L}simdutf.txt` },
  { title: 'zlib', kind: 'Zlib', origin: BUNDLED, path: `${L}zlib.txt` },
];

const check = process.argv.includes('--check');

const problemas = [];
for (const { title, path } of MANIFEST) {
  if (!existsSync(join(ROOT, path))) problemas.push(`falta ${path} (${title})`);
}
const declarados = new Set(MANIFEST.map((e) => e.path));
for (const nombre of readdirSync(join(ROOT, LICENSES_DIR))) {
  const ruta = `${LICENSES_DIR}/${nombre}`;
  if (!declarados.has(ruta)) problemas.push(`sin entrada en el manifiesto: ${ruta}`);
}
if (problemas.length) {
  console.error(problemas.join('\n'));
  process.exit(1);
}

const utf8 = new TextDecoder('utf-8', { fatal: true });
const texto = (e) => {
  try {
    return utf8.decode(readFileSync(join(ROOT, e.path))).replace(/\r\n/g, '\n');
  } catch {
    console.error(`no es UTF-8 (include_str! no compilaria): ${e.path}`);
    process.exit(1);
  }
};
const entradas = MANIFEST.map((e) => ({ ...e, body: texto(e) }));
const conFence = entradas.filter((e) => e.body.includes('```')).map((e) => e.title);
if (conFence.length) {
  console.error(`contienen un bloque de codigo (\`\`\`): ${conFence.join(', ')}`);
  process.exit(1);
}

const rust = [
  '// Generado por scripts/licenses.mjs; no editar a mano.',
  'pub const ENTRIES: &[(&str, &str, &str)] = &[',
  ...entradas.flatMap((e) => [
    '    (',
    `        ${JSON.stringify(e.title)},`,
    `        ${JSON.stringify(e.kind)},`,
    `        include_str!(${JSON.stringify(posix.relative('src-tauri/src', e.path))}),`,
    '    ),',
  ]),
  '];',
  '',
].join('\n');

const fila = (e) => `| ${e.title} | ${e.kind} | ${e.origin} |`;

const md = [
  '# Avisos de terceros',
  '',
  `Miso se distribuye bajo la licencia MIT (\`LICENSE\`). Incluye \`pdfium.dll\` x64, versión ${PDFIUM_VERSION}, release \`${PDFIUM_RELEASE}\` de pdfium-binaries, y la fuente Literata (SIL OFL 1.1), además de componentes de terceros con licencias permisivas. Sus textos completos se reproducen a continuación.`,
  '',
  '> Generado por `scripts/licenses.mjs` (`pnpm gen:licenses`). No editar a mano.',
  '',
  'El código fuente declara sus dependencias en `package.json` y `src-tauri/Cargo.toml`; cada una conserva su licencia original.',
  '',
  '| Componente | Licencia | Origen |',
  '|---|---|---|',
  ...entradas.map(fila),
  '',
  ...entradas.flatMap((e) => [`## ${e.title}`, '', '```', e.body.replace(/\n$/, ''), '```', '']),
].join('\n');

const salidas = [
  [RUST_OUT, rust],
  [MD_OUT, md],
];

function leer(ruta) {
  try {
    return readFileSync(join(ROOT, ruta), 'utf8');
  } catch {
    return null;
  }
}

if (check) {
  const desactualizados = salidas.filter(([ruta, contenido]) => leer(ruta) !== contenido).map(([ruta]) => ruta);
  if (desactualizados.length) {
    console.error(`Desactualizados (ejecutar pnpm gen:licenses):\n${desactualizados.join('\n')}`);
    process.exit(1);
  }
  console.log('Avisos de licencia al día.');
} else {
  for (const [ruta, contenido] of salidas) writeFileSync(join(ROOT, ruta), contenido, 'utf8');
  console.log(`Generados: ${RUST_OUT}, ${MD_OUT}`);
}
