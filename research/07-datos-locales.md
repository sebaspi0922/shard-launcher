# Datos locales de Roxy Launcher 0.6.0 (2026-10-11)

Método: listado de directorios y volcado de **claves y tipos** con `tools/keys.py`, que nunca imprime valores. Las referencias al binario son offsets en `extracted/appimage/usr/bin/roxy-launcher` (sha256 `768fe065…5ef2`), obtenidos con `tools/ctx.py`. No se leyeron valores de `session.json` ni de `launcher_account.json`. La única excepción es una comparación booleana del `id`, que no es secreto, con el nombre de la carpeta de cuenta.

## Rutas

| Ruta | Contenido | Evidencia |
|---|---|---|
| `~/.local/share/.roxy/` | Raíz de datos (`.roxy` en el directorio de datos del SO) | cadena `.roxy` en 0x4177b9 |
| `.roxy/local_settings.json` | Ajustes locales del launcher | 0x4177b9 |
| `.roxy/launcher_account.json` | Cuenta Roxy: id, email y username | 0x417808 |
| `.roxy/session.json` | Perfil de Minecraft y token de acceso de MC | 0x41784c |
| `.roxy/game/` | Todo lo de juego (4 GB) | listado |
| `~/.local/share/com.roxylauncher.app/` | Datos de WebKitGTK: localStorage, cookies, caché | listado |
| Keyring del sistema (Secret Service) | Refresh tokens. Ver «Keyring» | crate `keyring-2.3.3` y cadenas en 0x417808 |

## Archivos JSON (solo estructura)

`local_settings.json`. Estos valores no son sensibles:
```
allowMultipleInstances: bool     # actual: false
closeBehavior: str               # actual: "quit"; el binario también contiene "ask" (0x417561)
```

`launcher_account.json`:
```
id: str          # UUID de la cuenta Roxy; coincide con el nombre de game/accounts/<id>
email: str
username: str
```

`session.json` (sesión de Minecraft activa):
```
profile: { uuid: str, username: str, skin_url: str }
mc_access_token: str      # SECRETO: token de Minecraft Services
mc_expires_at: int        # epoch
```

## Keyring

En 0x417808 aparecen juntas estas cadenas:

```
com.sasukemc.minecraftlauncher.launcher-account · refresh_token · launcher_account.json · device_token · com.sasukemc.minecraftlauncher.auth · session.json
```

Confirmado en 05 por símbolos y desensamblado:
- La cuenta Roxy guarda dos entradas en el servicio `com.sasukemc.minecraftlauncher.launcher-account`: las cuentas (usuarios) `refresh_token` y `device_token`.
- El refresh token de Microsoft va en el servicio `com.sasukemc.minecraftlauncher.auth`, con la cuenta `ms_refresh_token`. En los JSON solo quedan el access token de MC y los datos públicos.

**Comprobado por el usuario en Seahorse (2026-10-11):** hay 3 entradas, todas con la etiqueta «keyring-rs v2.3.3 for target 'default', service 'com.sasukemc…'».
- `ms_refresh_token`: credencial viva de Microsoft.
- `device_token` y `refresh_token`: de la cuenta Roxy, inútiles con el servidor caído.

No se leyeron los valores y el clon no los necesita: se vuelve a iniciar sesión con la app de Azure propia.

Limpieza recomendada tras migrar: borrar las 3 entradas y revocar el acceso de la app de Roxy en la cuenta de Microsoft (Privacidad → Apps y servicios con acceso). Roxy también tenía una copia del refresh token de Microsoft en su servidor (05).

## WebView (`com.roxylauncher.app`)

- `localstorage/tauri_localhost_0.localstorage` (SQLite, tabla `ItemTable`): una sola clave, `roxy-launcher:low-power-mode` (valor de 2 bytes).
- También hay `cookies`, `hsts-storage.sqlite`, `WebKitCache` y `CacheStorage`. No se inspeccionaron porque pueden contener cookies de sesión. No hacen falta para el clon.

## `game/`

```
game/
├─ accounts/<launcher_account.id>/instances/<slug>-<epoch>/   # instancias POR CUENTA ROXY
├─ assets/{indexes/32.json, objects/, skins/}                # layout estándar de Mojang
├─ installers/   (vacío)
├─ java/25/      # Temurin 25.0.4+7 (Adoptium), JRE completo con bin/ lib/ release
├─ libraries/    # layout Maven estándar (com/, net/, org/, io/, it/, at/…)
├─ natives/<version-id>/{lwjgl,jna,netty}/
├─ tmp/          (vacío)
└─ versions/<version-id>/<version-id>.jar
```

Observaciones:
- `versions/fabric-loader-0.19.3-26.2/` solo contiene el `.jar` (39 MB, el cliente). **No hay JSON de versión guardado**: el launcher lo resuelve en cada arranque (red o memoria). Para migrar, el clon tendrá que volver a descargar los manifiestos. `libraries/` y `assets/` se pueden reutilizar porque siguen el layout estándar.
- Hay una sola carpeta de cuenta Roxy. Las instancias cuelgan de la cuenta Roxy, no de la de Microsoft: sin cuenta Roxy no hay instancias. El clon debe desacoplarlas.

### Instancias actuales

| Carpeta | MC | Loader | Tamaño | Notas |
|---|---|---|---|---|
| `fabulously-optimized-1785816370` | 26.2 | fabric 0.19.3 | 2,6 GB | memoryMb 12288, jvmArgs G1 tuneados, backups automáticos cada 72 h (máximo 3), 3 `backup-<epoch>.zip`, `screenshots/`, `crash-reports/`, `flashback/`, Xaero |
| `minecraft-tecnico-1787449754` | 26.2 | fabric 0.19.3 | 104 MB | valores por defecto (null) |

`instance.json` (las claves en camelCase):
```
id: str                 # = nombre de la carpeta (slug + epoch de creación)
name: str
mcVersion: str
loaderType: str         # "fabric" | … (ver 06-lanzamiento.md)
loaderVersion: str
createdAt: int          # epoch
lastPlayed: int
playtimeSeconds: int
iconPath: str|null
memoryMb: int|null
jvmArgs: str|null       # cadena única, separada por espacios
resolutionWidth: int|null
resolutionHeight: int|null
devMode: bool
backupAutoEnabled: bool
backupIntervalHours: int|null
backupMaxCount: int|null
deletedAt: int|null     # papelera: borrado suave (mc_list_trash_instances / mc_restore_deleted_instance)
```

`.modrinth_sources.json` (procedencia de cada archivo de contenido): un mapa `"<tipo>/<nombre-archivo>" → {download_url, sha1, project_id|null, project_name|null, version_number|null, mod_category|null}`. Los mods desactivados llevan el sufijo `.disabled` también en la clave. Las entradas con `project_id` null vienen de modpack o de una instalación sin metadatos.

Otras carpetas propias del launcher dentro de la instancia:
- `.mods/cape-provider`: lo crea el mod Cape Provider, no el launcher (06).
- `backups/backup-<epoch>.zip`.
- `latest_launch.log`. No se leyó porque puede contener el token en la línea de comandos.

El resto (`mods/`, `config/`, `saves/`, `options.txt`, `servers.dat`…) es el `.minecraft` estándar usado como `--gameDir`.

## Implicaciones para la migración (fase 2)

1. Importar las instancias de `accounts/<id>/instances/*` a la estructura propia y mapear `instance.json` campo a campo. Los nombres coinciden en su mayoría con los de Modrinth App y Prism.
2. Reutilizar `libraries/`, `assets/` y `java/25` (copia o hardlink). Volver a descargar los JSON de versión.
3. No migrar tokens. Iniciar sesión de nuevo con la app de Azure propia.
4. Conservar `.modrinth_sources.json` para saber de dónde vino cada mod y poder actualizarlo.

## Desconocidos
- ~~Contenido del keyring~~: resuelto (ver arriba).
- Si `closeBehavior` admite otros valores aparte de `quit` y `ask` (p. ej. `minimize`/`tray`). Lo cubre 04-comandos-tauri.md.
- Formato de las instancias compartidas (`shared_*`) y de la papelera en disco. No hay ejemplos locales.
