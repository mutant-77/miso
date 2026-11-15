# Third-party notices

Miso se distribuye bajo la licencia MIT (`LICENSE`). Incluye componentes de terceros con licencias permisivas propias, que se reproducen a continuación.

## PDFium

- Componente: `pdfium.dll` x64, versión 158.0.8125.0, embebido en el ejecutable.
- Origen: [PDFium](https://pdfium.googlesource.com/pdfium/) de Chromium, empaquetado por [pdfium-binaries](https://github.com/bblanchon/pdfium-binaries), release `chromium/8125`.
- Licencia: BSD-3-Clause (`src-tauri/pdfium/licenses/pdfium.txt`); empaquetado MIT (`src-tauri/pdfium/LICENSE`).

## Bibliotecas internas de PDFium

Todas permisivas; textos completos en `src-tauri/pdfium/licenses/`.

| Archivo | Componente | Licencia |
|---|---|---|
| `abseil.txt` | Abseil | Apache-2.0 |
| `agg23.txt` | AGG 2.3 | Permisiva (aviso de copyright) |
| `dragonbox-Apache2-LLVM.txt`, `dragonbox-Boost.txt` | Dragonbox | Apache-2.0 / Boost |
| `fast_float.txt` | fast_float | MIT |
| `freetype.txt` | FreeType | FTL |
| `harfbuzz.txt` | HarfBuzz | MIT |
| `highway.txt` | Highway | Apache-2.0 |
| `icu.txt` | ICU | Unicode License V3 |
| `lcms.txt` | Little CMS | MIT |
| `libjpeg_turbo.ijg`, `libjpeg_turbo.md` | libjpeg-turbo | IJG / BSD-3-Clause / zlib |
| `libopenjpeg.txt` | OpenJPEG | BSD-2-Clause |
| `libpng.txt` | libpng | libpng |
| `llvm-libc.txt` | LLVM libc | Apache-2.0 with LLVM exceptions |
| `simdutf.txt` | simdutf | MIT |
| `zlib.txt` | zlib | zlib |

## Resto de dependencias

El código fuente declara sus dependencias en `package.json` y `src-tauri/Cargo.toml`; cada una conserva su licencia original.
