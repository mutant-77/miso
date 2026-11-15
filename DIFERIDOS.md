# Diferidos

Pendientes diferidos y deuda técnica secundaria. Brevedad ante todo: una línea por entrada, con fecha y hora.

- 2026-11-14 — Instalador y asociaciones «Abrir con», si algún día se distribuye: sin `bundle.fileAssociations` (en NSIS escribe el predeterminado de `HKCU\Software\Classes\.ext` y quita el programa del usuario); usar `nsis.installerHooks` con `OpenWithProgids`. Incluir las licencias de pdfium y Literata (OFL). Sin fase.
- 2026-11-12 13:23 — Pegar un archivo copiado en el Explorador (`CF_HDROP`): no soportado, el texto entra por Ctrl+V. Sin fase.
- 2026-11-13 — Partición: idiomas aparte de `en`/`es` (añadir patrones y palabras frecuentes) e idioma declarado por el documento (HTML `lang`, DOCX `w:lang`; la fase 5 no lo importa). Sin fase.
- 2026-11-13 02:42 — Fuentes, tamaños y colores del documento, con `ForcePreferredFont`/`ForceForegroundColor` y `mostUsedFontSize`: las fases 5 y 6 pintan todo con prefs (ver REGISTRO, fases 5 y 6). Sin fase.
- 2026-11-13 13:10 — PDF: títulos, negrita/cursiva, cabeceras y pies de página (ver REGISTRO, fase 6). Con números de página o titulillos, la unión entre páginas no salta (actualizado 2026-11-13 13:50). El número de sección que precede a un título (`-5` en «El pensamiento del afuera») sale como bloque propio: es contenido, no número de página; se resolverá con los títulos (actualizado 2026-11-13 21:19). Sin fase.
- 2026-11-13 13:10 — RTF: `\fcharset` por fuente (hoy todo el texto usa `\ansicpgN`; afecta a RTF antiguos con fuentes de otro alfabeto). Sin fase.
- 2026-11-14 13:55 — Composición de libro: saltos de página de DOCX (`w:br w:type="page"`, `w:pageBreakBefore`) y RTF (`\page`, `\sect`) como capítulo; hoy sólo EPUB (Q1, ver REGISTRO, «Composición de libro»). Sin fase.
- 2026-11-14 13:55 — Composición de libro: títulos que el EPUB marca por clase (`<p class="titulo">`) y no con `hN` no se centran ni cuentan como título. Sin fase.
- 2026-11-14 — EPUB: comprobar en el ejecutable (`tauri build --debug`, F12) que la CSP deja ver las imágenes `miso-img` sin violaciones; en `tauri dev` no se aplica. Pendiente del titular.
- 2026-11-14 — EPUB: un manifiesto con `media-type` no estándar (`image/jpg`) omite la imagen (C8 del plan EPUB); aceptarlo si aparece en libros reales. Sin fase.
- 2026-11-14 — Imágenes en DOCX (`document.xml.rels` + `a:blip`) y en HTML suelto (archivos vecinos al elegido). Sin fase.
- 2026-11-11 07:20 — `vitest.config.ts` fuera de Windows: sin `ProgramFiles(x86)` la ruta de EdgeCore queda relativa al proyecto. Inofensivo (D7); arreglo de una línea. Sin fase.
- 2026-11-15 07:02 — Licencias: recorrido manual en `pnpm tauri dev` (Preferencias → Licencias: 21 entradas, foco en «Volver», seleccionar y copiar, volver, Escape cierra). Pendiente del titular (ver REGISTRO, «Avisos de licencia embebidos»).
- 2026-11-15 07:02 — Licencias: el control de huérfanos sólo vigila `src-tauri/pdfium/licenses/`; una fuente o binario nuevo con licencia propia fuera de ahí no lo dispara. Sin fase.
- 2026-11-15 07:02 — `Escape` cierra el panel entero desde la vista de licencias; una pila por vista sería más fina. Sin fase.