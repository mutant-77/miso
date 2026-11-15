# Cierre de fase 0 — a tener en cuenta en la fase 1

Fase 0 cerrada en `7777d54` (detalle en `REGISTRO.md` y `.historial/_PLAN_FASE_0_completado.md`). Alcance de fase 1: ARCHITECTURE §13.

## 1. Decisiones a fijar al planificar la fase 1

| # | Asunto | Por qué importa | Referencia |
|---|---|---|---|
| 1 | **Puerta de entrada de documentos en fase 1.** §13 pide "un `.txt` se lee", pero Ctrl+O (§7.1) es fase 2 y drop/portapapeles fase 3. Opciones: argumento de lanzamiento (`pnpm tauri dev -- -- ruta.txt`), adelantar `open_dialog`, o ambos. | Sin esto el criterio de hecho no es ejecutable. `open_dialog` ya exige `tauri-plugin-dialog`, que la fase 1 añade por D11. | §9, §13 |
| 2 | **`specta` o `ts-rs`** para generar `types.ts`. Sigue *(verificar)*. | Primer tipo compartido (`Doc`, `AppError`). | §5 |
| 3 | **Creación de `reader-0`.** Hoy la crea `tauri.conf.json` al arrancar. Con archivo de lanzamiento, Rust debe dejar el `Doc` pendiente **antes** de que el frontend llame a `take_pending_document`. Valorar crear la ventana en `setup` (`create: false` en la conf) tras registrar el pendiente, o que el pendiente se resuelva aunque se pida antes. | Carrera que perdería el documento de arranque. | §5 Enrutado, §9.1 |

## 2. Verificaciones abiertas que caen en fase 1

- `MessageDialogBuilder::parent` y `show` no bloqueante en Windows (§5 Errores).
- Apertura de lanzamiento fallida → ventana vacía + aviso encima (§9.1).
- Pánico en importador → `JoinError::is_panic` → `Corrupt`/`Internal` según la fase (§5 Errores).
- Paridad del índice UTF-16 Rust ↔ TS con fixture de emoji, CJK, combinantes, CRLF y `U+FFFC` (§5). Crear `tests/fixtures/`.

## 3. Estado del esqueleto que la fase 1 hereda

- **Frontend:** `App.svelte` vacío; no está instalado `@tauri-apps/api` (se añade en fase 1). `App.svelte` debe decidir por label (`reader-*`).
- **Capabilities:** `reader` ya concede `core:event:default` y `core:window:default`; no añadir `dialog` al frontend (D10, D11, §11).
- **Rust:** `lib.rs` sin plugins ni comandos. Fase 1 añade `tauri-plugin-dialog`, `tauri-plugin-log`, `panic::set_hook` y los módulos de §4 que toque.
- **Tests unitarios:** el proyecto `unit` existe con `--passWithNoTests`. Con los primeros tests (`textIndex`, `columnWidth`), quitar ese flag para que un test perdido se note.
- **Log:** irá a la carpeta de logs del identificador provisional `com.provisional.miso` (cambia al fijar el dominio, fase 7).

## 4. Entorno (trampas observadas)

- Compilar y ejecutar puertas **desde PowerShell**, nunca Git Bash (§4 Entorno).
- Volumen **exFAT**: `.npmrc` hoisted obligatorio; avisos de hard link de Cargo inofensivos.
- Primera compilación en frío de `tauri dev`: ~10–15 min; incremental ~40 s.
- La **CSP no se aplica en `tauri dev`**: una violación (p. ej. `eval`, estilos o scripts en línea fuera de Svelte) no se verá en desarrollo. No introducirlas; se verifica en fase 7.
- Al lanzar `tauri dev` desde un agente: cerrar con `CloseMainWindow()` y matar el árbol del proceso lanzado; comprobar que el puerto 1420 queda libre.
- `test:layout` usa EdgeCore (motor de WebView2) o Edge estándar; sin Edge falla (decisión F2).

## 5. Pendiente menor sin decidir

- `vitest.config.ts` fuera de Windows: si no existe `ProgramFiles(x86)`, la ruta de EdgeCore queda relativa al proyecto. Inofensivo; arreglo de una línea cuando se quiera (no urgente por D7).
