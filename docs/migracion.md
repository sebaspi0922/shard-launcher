# Migración desde `~/.local/share/.roxy` (spec, 2026-10-11)

Objetivo: pasar las instancias y el contenido descargado de Roxy al launcher nuevo sin perder mundos, mods ni ajustes, sin tocar el original y sin copiar ningún token. Se implementa como `shard import roxy` en el hito 2 y se ofrece también desde la UI (hito 4).

## 1. Principios

1. **Solo lectura sobre `.roxy`.** El importador nunca escribe, mueve ni borra nada allí. Roxy sigue funcionando igual (si su servidor vuelve) y la migración se puede repetir.
2. **Sin secretos.** No se leen `session.json`, `launcher_account.json` (salvo su `id` para localizar la carpeta, que ya coincide con el nombre de `accounts/<id>`, así que tampoco hace falta), el keyring de Roxy ni `latest_launch.log`. La cuenta de Microsoft se vuelve a añadir con el login propio.
3. **Primero simular.** `--dry-run` muestra qué se copiaría, cuánto ocupa y los conflictos, sin escribir nada.
4. **Idempotente.** Si una instancia ya se importó (`imported_from.original_id`), se salta salvo con `--force`.

## 2. Inventario de origen (medido el 2026-10-11)

| Origen | Tamaño | Tratamiento |
|---|---|---|
| `game/libraries/` | 81 MB | Enlace duro archivo a archivo (contenido inmutable, mismo sistema de archivos ext4). Si falla el enlace (otro disco), copia. Se valida el SHA-1 en el siguiente lanzamiento |
| `game/assets/{indexes,objects}` | 990 MB | Enlace duro (objetos direccionados por hash) |
| `game/assets/skins` | — | Se ignora (caché) |
| `game/versions/<id>/<id>.jar` | 38 MB | Enlace duro del jar del cliente. El JSON no existe en Roxy: se vuelve a descargar a `versions/<id>/<id>.json` |
| `game/java/25` | 192 MB | Copia a `java/25` (se comprueba con `java -version`). Opcional: si se prefiere, se reinstala desde Adoptium con checksum |
| `game/natives/` | 21 MB | No se migra: se regenera en el primer lanzamiento |
| `game/installers/`, `tmp/`, `quickplay/`, `launcher_profiles.json` | — | No se migran |
| `game/accounts/<id>/instances/<slug>-<epoch>/` | 2,6 GB + 104 MB | Ver §3 |

Los enlaces duros no duplican espacio. Si después se borra `.roxy`, los archivos siguen existiendo en el launcher nuevo.

## 3. Instancias

Para cada carpeta de `accounts/*/instances/*` con `instance.json` y sin `deletedAt`:

1. **Id nuevo**: el slug sin el epoch (`fabulously-optimized-1785816370` → `fabulously-optimized`); si ya existe, se añade `-2`, `-3`…
2. **`instance.json`**, campo a campo:

| Roxy | Nuevo | Nota |
|---|---|---|
| `id` | `imported_from.original_id` | |
| `name` | `name` | |
| `mcVersion`, `loaderType`, `loaderVersion` | `game.{mc, loader, loader_version}` | `loaderType` igual: vanilla, fabric, quilt, forge, neoforge, labymod, custom |
| `createdAt`, `lastPlayed`, `playtimeSeconds` | `created_at`, `last_played`, `playtime_seconds` | |
| `memoryMb` | `java.memory_mb` | `null` → valor global |
| `jvmArgs` (cadena) | `java.jvm_args` (lista) | Partido por espacios. Se quitan `-Xmx`/`-Xms` repetidos porque los pone la memoria |
| `resolutionWidth`, `resolutionHeight` | `window.width`, `window.height` | Ahora sí se aplican |
| `devMode` | `dev_console` | |
| `backupAutoEnabled`, `backupIntervalHours`, `backupMaxCount` | `backups.*` | |
| `iconPath` | `icon.png` | Se copia el archivo si existe |
| `deletedAt` | — | Instancias en la papelera de Roxy: solo con `--include-trash` |

3. **Archivos del juego**: copia completa a `instances/<id>/game/` (no enlace duro, porque el juego modifica mundos y configs en el sitio y eso alteraría también la copia de Roxy). Se excluyen:
   - `instance.json`, `.modrinth_sources.json` (se convierten);
   - `backups/` (ver paso 5);
   - `latest_launch.log` (puede contener el token en la línea de comandos);
   - `logs/` y `crash-reports/` se copian (útiles para Doctor) salvo con `--skip-logs`.
4. **Procedencia**: `.modrinth_sources.json` → `content.json`. Las claves `"<tipo>/<archivo>"` pasan a una lista `[{type, file, enabled, source: "modrinth"|"unknown", project_id, version_id?, version_number, sha1, url}]`. El sufijo `.disabled` se refleja en `enabled`. Las entradas sin `project_id` (vienen del modpack) se intentan identificar después por hash con `POST /v2/version_files` de Modrinth (opcional, `--resolve-hashes`).
5. **Backups**: los `backup-<epoch>.zip` y `.backups.json` van a `backups/<id>/` con enlace duro (son inmutables). Aviso: los zips de Roxy incluyen su `latest_launch.log`, que puede contener un token de Minecraft en la línea de comandos. El enlace duro no crea una copia nueva, y esos tokens caducan a las 24 h, así que el riesgo es bajo. Aun así, los backups no deben compartirse sin limpiar. Se convierte `.backups.json` al formato nuevo. Con `--skip-backups` no se importan (ahorra tiempo si se prefiere empezar de cero).
6. **Verificación**: número de archivos y bytes de `game/` frente al origen (sin las exclusiones), `instance.json` válido y el lanzamiento de prueba en modo `--dry-run` (plan de descargas vacío o mínimo).

## 4. Qué no se migra

- Cuenta Roxy, sesión y tokens. Ajustes de la nube (`/me/settings`): no hay copia local; se configuran de nuevo.
- `local_settings.json`: solo `closeBehavior` y `allowMultipleInstances` se traducen a `settings.json` (son booleanos o enums, no sensibles).
- Datos del WebView (`~/.local/share/com.roxylauncher.app`): solo contienen `low-power-mode`; se ignora.

## 5. Después de migrar (manual, opcional)

1. Comprobar que las dos instancias arrancan y que los mundos están.
2. Borrar las 3 entradas del keyring de Roxy en Seahorse y revocar la app de Roxy en la cuenta de Microsoft.
3. Mantener `.roxy` un tiempo como copia de seguridad; borrarlo cuando no haga falta. Los enlaces duros garantizan que no se pierde nada del lado nuevo.

## 6. Pruebas

- Árbol falso que imita `.roxy` (sin `session.json` real) en `tests/fixtures/roxy/`, con: una instancia normal, una en papelera, una con `jvmArgs` nulo, `.modrinth_sources.json` con entradas `.disabled` y sin `project_id`, y un backup.
- Comprobar: dry-run no escribe nada; segunda ejecución es idempotente; nunca se abre `session.json` ni `latest_launch.log` (el test los crea con contenido centinela y verifica que no aparece en ninguna salida ni archivo de destino).
