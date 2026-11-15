# Decisiones

Decisiones fijadas por el titular. Prevalecen sobre el resto y no se reabren en auditorías. Brevedad ante todo: una línea por decisión.

- **Plataforma:** Windows primero (ARCHITECTURE D7).
- **PDF:** PDFium empaquetado con `pdfium-render` (ARCHITECTURE §9).
- **RTF:** lector propio en Rust, no RichEdit (2026-11-13, plan de fase 6 C1): menos sobrecarga.
- **Nombre:** Miso (`com.asagoo.tofu` es del original y no se reutiliza).
- **Bundle id:** `com.mutant77.miso` (2026-11-14).
- **Distribución:** sin instalador; ejecutable propio con `pdfium.dll` embebida (2026-11-14, plan de fase 7 C8).
- **EPUB:** con imágenes, servidas por el protocolo propio `miso-img` (por número, sólo a su ventana); título de ventana = título del libro (`dc:title`) (2026-11-14, plan EPUB).
- **Composición de libro:** Miso aplica su propia composición (títulos centrados, sangría de primera línea, media línea entre párrafos salvo en `.txt`, capítulo en columna nueva), no la hoja de estilos del documento (2026-11-14). Capítulo = ítem del spine EPUB que aporta bloques (también los fragmentos de Calibre y las notas en archivo aparte); los demás formatos no tienen capítulos. En libro, las líneas en blanco no se ven (alto 0): la separación la pone Miso.
- **Ventana:** estándar con barra de título nativa, oscura si el sistema lo está (`DWMWA_USE_IMMERSIVE_DARK_MODE`); nada de frameless.
- **Versión:** SemVer desde 1.0.0; fuente única `src-tauri/Cargo.toml`; etiqueta git `vX.Y.Z` y Release en GitHub con el `.exe`; visible en Acerca de como «Miso X.Y.Z»; el ejecutable se sigue llamando `miso.exe` (2026-11-15).
- **Idioma de la interfaz:** inglés por defecto + es, pt (Brasil), de, it, fr, nl, ru, nb (bokmål), sv, fi; detección por la lista de idiomas del sistema y cambio manual en Preferencias; idiomas por su endónimo, sin banderas (2026-11-15).
