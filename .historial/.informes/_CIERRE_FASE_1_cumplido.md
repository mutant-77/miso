# Cierre de fase 1 — a tener en cuenta en la fase 2

Fase 1 cerrada en `cceeae7` (detalle en `REGISTRO.md` y `.historial/_PLAN_FASE_1_completado.md`). Alcance de fase 2: ARCHITECTURE §13.

## 1. Pendiente antes de planificar

- **Confirmación visual del titular** (no verificada por agentes): aviso de error encima de la ventana, texto en columnas, ←/→, Av Pág/Espacio, Inicio/Fin, rueda, redimensionado y Ctrl+O (ventana con documento → otra ventana; vacía → carga en ella sin tocar las demás). Si algo falla, se corrige antes de empezar la fase 2.
- **Diferidos que vencen "antes de la fase 2"** (`DIFERIDOS.md`): rangos válidos de prefs e idioma de la UI.

## 2. Decisiones a fijar al planificar la fase 2

| # | Asunto | Por qué importa | Referencia |
|---|---|---|---|
| 1 | **Semántica de ↑/↓.** §7 dice "media columna y ajusta al final al inicio de columna más cercano": con `pitch / 2` el ajuste puede devolver a la misma columna o saltar a la siguiente. Fijar qué se quiere. | Sin esto el paso no es implementable sin inventar. | §7 |
| 2 | **Texto visible de `Read`.** `user_message` incluye el `Display` de `io::Error`, que en Windows añade ` (os error 3)` al mensaje del sistema. Decidirlo junto con el idioma de la UI. | Es texto que ve el usuario. | §5 Errores, `error.rs` |
| 3 | **Restricción de comandos por ventana** (diferido). Al añadir `prefs`, decidir si se usa `AppManifest` + permisos o basta con comprobar el *label* en cada comando. | Hoy cualquier ventana puede invocar cualquier comando. | §11 |

## 3. Estado que la fase 2 hereda

- **Ventanas:** las crea Rust (`doc_windows::create_reader`); `tauri.conf.json` no tiene `windows`. Etiquetas `reader-{n}`, contador desde 1. La ventana de preferencias también tendrá que crearse desde Rust.
- **`App.svelte`** monta `Reader` sin mirar el *label* (G9 de fase 1): la fase 2 introduce la decisión por *label*.
- **`Slot::Open` no guarda el `Doc`.** Las fases 3 (regla 1, mismo `doc_key`) y 7 (`save_copy`) lo necesitan: añadirlo entonces.
- **Motor de columnas:** las columnas se **estiran** al ancho de la ventana (aún no hay viewport múltiplo exacto). `remeasure()` en redimensionado ajusta a la columna más cercana **sin ancla**: el texto visible puede cambiar; eso es lo que corrige la fase 2. Valores fijos en `Columns.svelte` (360 px, gap 21.6, padding 20/10.8, Palatino 14 px, interlineado 1.1): pasan a prefs.
- **API del scroller** (`columnScroll.ts`): `go`, `goTo(k, behavior?)`, `first`, `last`, `visibleColumns`, `sync` (en `scrollend`), `remeasure`. El parámetro `behavior` existe para forzar `'instant'` en tests; reutilizarlo en el test de 500 saltos.
- **Índice:** `textIndex.ts` (`blockStarts`, `flatLength`) y `data-char-start` en cada `<p>`. Paridad con Rust probada sobre `tests/fixtures/unicode.doc.json` (K = 25). El ancla se apoya en esto.
- **Tipos TS:** generados por `cargo test` en `src/lib/ipc/types/` (no editar a mano). `Prefs` debe derivar `TS` igual que `Doc`. `pnpm verify` ya corre Rust antes de `check`.

## 4. Trampas observadas

- **Svelte 5 recorta el espacio al principio y final de un bloque `{#each}`** y deja un nodo de texto vacío como ancla. Un espacio literal sólo entra con `{' '}`. Los tests de D9 comprueban "ningún nodo de texto con contenido", no "ningún nodo de texto".
- **Escuchar eventos con `getCurrentWebviewWindow().listen`**, nunca con el `listen` global (recibe los `emit_to` de otras ventanas). Aplica a `prefs-changed`.
- **Comandos que crean ventanas: siempre `async`** (en Windows, los síncronos se bloquean). `blocking_*` del diálogo, dentro de `spawn_blocking`.
- **`tauri dev` corre con el directorio actual en `src-tauri/`**: usar rutas absolutas en los argumentos de lanzamiento.
- **Lanzar `tauri dev` desde un agente:** `Start-Process` oculto, esperar a `Get-Process miso` con ventana, `CloseMainWindow()`, `taskkill /T /F` del árbol y comprobar el puerto 1420. `taskkill` deja código de salida 128: inofensivo. Compilación incremental ≈ 40 s.
- **Log:** `%LOCALAPPDATA%\com.provisional.miso\logs\Miso.log` (más consola en debug). `report()` deja la ruta completa y la causa.
- **Filtrar salidas de Cargo con `Select-String`:** no distingue mayúsculas por defecto y "failed" aparece en los avisos de *hard link* del volumen exFAT; usar `-CaseSensitive` y leer `$LASTEXITCODE` antes de cortar la salida con `Select-Object -First`.
- **Agentes implementadores:** no lanzan `tauri dev`; las comprobaciones en ejecución las hace el orquestador.
