# Principios básicos

**Recuerda:** el plan A es el ingenio; el plan B, la robustez. Recurre a la robustez solo cuando el ingenio no basta. Evita siempre la sobreingeniería.
**Recuerda:** no implementes cambios para que los tests pasen; implementa el comportamiento correcto y deja que los tests pasen como consecuencia.
Se pueden reabrir decisiones siempre que el titular lo apruebe.
Documentación concisa y breve. Prohibido escribir comentarios largos y explicativos.

## Reglas básicas

* No editas código sin tener la confirmación del usuario.
* Si el usuario solicita una auditoría o un plan, entrega un informe claro y comprensible, acompañado de un archivo `.md` técnico y profesional como respaldo que inicie con "_" para que se sitúe arriba del todo.
* Si auditas una auditoría o un plan, propone cambios quirúrgicos sobre el documento original y explica cada observación de forma racional y no técnica.
* Si quedan diferidos tras una implementación, buscarás `DIFERIDOS.md` y añadirás o quitarás con criterio, añadirás fecha y hora. Si no existe el documento lo crearás, nunca dejarás un diferido perdido en un plan que ha sido completado.
* Todo documento de plan implementado, debe renombrarse con "_completado" al final, ej: NOMBRE_DEL_DOCUMENTO_completado.md, y moverse a `.historial/` (no se queda en la raíz).
* Al borrar un plan completado, sus diferidos se reapuntan a `REGISTRO.md` (el asiento que documenta esa implementación) antes de borrarlo: el archivo del plan es temporal, el asiento del registro es permanente.
* Los archivos `_AUDITORIA`, `_DIAGNOSTICO`, `_INFORME` y `_REVISION` se archivan de la raíz una vez cumplida su finalidad: se mueven a `.historial/.informes/` y se renombran con `_cumplido` al final. El usuario puede borrarlos en cualquier momento si ya cumplieron una finalidad puntual.
* Tras cada implementación, añade un registro nuevo en `REGISTRO.md` (fecha + título, resumen breve, archivos tocados, estado) y haz commit. Si no existe el documento lo crearás.
* Tras una implementación nunca intentar compilar para producción ni generar instaladores o empaquetados autónomos.

## Documentos importantes

* `DECISIONES.md`: decisiones fijadas por el titular. Prevalecen sobre el resto y no se reabren en auditorías.
* `ARCHITECTURE.md`: estructura del proyecto y decisiones técnicas. Es la referencia de las decisiones técnicas.
* `DIFERIDOS.md`: pendientes diferidos y deuda técnica secundaria identificada o postergada temporalmente en implementaciones o refactorizaciones.
* `REGISTRO.md`: registro histórico de cambios e intervenciones implementadas y verificadas por agentes (fecha + título, resumen breve, archivos tocados y estado).

## Proceso de compilación (Build)

- **NUNCA** ejecutes `pnpm build` o comandos de compilación similares automáticamente a menos que el usuario lo solicite explícitamente. El usuario ejecutará el build manualmente cuando lo desee.

## Escritura de archivos (Windows)

En otro proyecto, un `AGENTS.md` fue corrompido silenciosamente por una redirección de escritura en la shell: 339 bytes nulos y una regla destrozada, sin ser detectado hasta que alguien lo leyó byte por byte. Estas reglas existen para que no ocurra aquí.

- **Escribe y edita archivos con las herramientas de archivos, no con redirección de shell.** En Windows PowerShell 5.1, `>`, `>>` y `Out-File -Append` usan UTF-16LE por defecto. Añadir eso a un archivo UTF-8 intercala un byte nulo después de cada carácter y destruye la sección sin ningún error.
- **Cada archivo de texto en este repositorio es UTF-8 sin BOM.** Si realmente debes escribir desde PowerShell, usa `Set-Content -Encoding utf8NoBOM`, nunca `>`/`>>`.
- **Nunca construyas contenido de archivo dentro de una cadena de PowerShell con comillas dobles.** El acento grave (backtick) es el carácter de escape de PowerShell, por lo que un bloque de código Markdown como `` `remark-directive` `` pierde sus comillas invertidas y el `` `r `` se convierte en un retorno de carro. Escapar con barra invertida (`` \` ``) es sintaxis de bash y lo empeora: la barra invertida sobrevive literalmente y la comilla invertida sigue siendo eliminada. Usa un here-string con comillas simples (`@'…'@`) con el `'@` de cierre en la columna 0.
- **Verifica después de cualquier escritura por shell:** el archivo debe contener cero bytes nulos. `([IO.File]::ReadAllBytes($p) | Where-Object { $_ -eq 0 }).Count` debe ser `0`.
