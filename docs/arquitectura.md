# Arquitectura (spec v1, 2026-10-11)

Base: inventario de la fase 1 (`research/02`–`08`) y D1 (Rust + Flutter). Aquí se fija qué se construye y cómo. Las decisiones con alternativas están en `docs/decisiones.md` (D2–D7); el orden de trabajo, en `docs/plan.md`.

## 1. Alcance

**Se replica** (con código propio) el flujo de Roxy 0.6.0 que funciona en local o contra APIs públicas: login de Microsoft, cuentas offline, instancias (crear, editar, papelera, importar), instalación de versiones y loaders (Vanilla, Fabric, Quilt, Forge, NeoForge; OptiFine y LabyMod con prioridad baja), Java gestionado, lanzamiento con consola, detección de crash y Doctor, backups, mundos y datapacks, Modrinth y CurseForge (con clave propia), skins y capas, servidores rápidos, Rich Presence, ajustes locales, tema adaptativo, tutorial e i18n es/en.

**No se replica**: cuenta Roxy, respaldo de tokens en servidor, ecosistema (wallet, tienda, inventario, logros), pase de batalla, leaderboard, referidos, noticias de partners, vinculación con Google/Discord, telemetría.

**Más adelante, en `server/`** (opcional): amigos y presencia, LAN por internet (relay), instancias compartidas o sincronizadas, skins offline (Yggdrasil), releases del updater.

**Mejoras frente al original** (bugs y huecos de `research/06` y `08`):
- Sin cuenta propia obligatoria: el launcher arranca y lanza sin servidor.
- Multiplataforma real: reglas de librerías y natives para linux/windows/osx y x86_64/aarch64; el original solo conocía linux x86_64.
- Descargas con reintentos y backoff, caché de los JSON (manifiesto, versión, loaders) y modo offline cuando todo está instalado.
- JSON de versión guardado en disco en el layout estándar `versions/<id>/<id>.json` (el original lo pedía en cada arranque). La fusión `inheritsFrom` se hace al cargar.
- JRE verificado con checksum de Adoptium.
- Resolución aplicada (`--width/--height`), que el original guardaba pero no usaba.
- Varias partidas a la vez (opcional por ajuste).
- Metadatos del launcher fuera del `gameDir` y backups fuera de la instancia.
- Estado de servidores con Server List Ping propio en vez de `mcsrvstat`; render de skins local en vez de `visage`/`minotar`.

## 2. Estructura del repositorio

```
Cargo.toml              workspace: core, app/rust (y server/ cuando exista)
core/                   crate `lcore` (biblioteca) + binario CLI `shard`
  src/lib.rs            fachada pública (`Core`) y re-exports
  src/bin/shard.rs      CLI (clap); feature `cli`, activa por defecto
  tests/                integración (wiremock, fixtures, golden)
  tests/fixtures/       JSON reales de Mojang, Fabric, Forge, logs y crash reports
app/                    proyecto Flutter
  rust/                 crate puente `lcore_bridge` (flutter_rust_bridge); solo adapta tipos
  lib/                  Dart: UI y estado
server/                 (hito 5) servicios del home-server
docs/  research/  tools/
```

`lcore` es el nombre interno provisional del crate; el nombre visible de la app está pendiente (D2).

Regla de dependencias: `core` no conoce Flutter ni frb. `app/rust` depende de `core` y expone una API pequeña con tipos planos. La CLI y la app usan la misma fachada, así que todo lo que hace la UI se puede probar y depurar desde la terminal.

## 3. Núcleo (`core/`)

### 3.1 Módulos

| Módulo | Responsabilidad | Referencia en research |
|---|---|---|
| `paths` | Directorios de datos por SO (`directories`), layout de §4, rutas seguras (sin `..`, zip-slip) | 06 §1, 07 |
| `settings` | `settings.json` global (versionado, migraciones de esquema) | 04 §1.3, 07 |
| `net` | Cliente HTTP único (reqwest + rustls), User-Agent propio, timeouts, reintentos con backoff exponencial y jitter, caché JSON con ETag/Last-Modified y TTL, modo offline | 06 §2.8 |
| `download` | Cola de descargas con límite de concurrencia (por defecto 12, configurable), verificación SHA-1/SHA-512, escritura `.part` + rename, progreso agregado (archivos y bytes), cancelación | 06 §2.5 |
| `auth` | `msa` (device code, refresco), `xbox` (XBL, XSTS, XErr), `mc` (login_with_xbox, entitlements, perfil, skins, capas), `offline` (UUID v3 `OfflinePlayer:<nombre>`), `store` (cuentas públicas en JSON y secretos en keyring) | 05 §2 |
| `meta` | Manifiesto de Mojang, JSON de versión, `inheritsFrom` y fusión, reglas (`os`, `arch`, `features`), índice de assets | 06 §2 |
| `loader` | `fabric`, `quilt` (perfil JSON de meta), `forge`/`neoforge` (instalador oficial `--installClient`, ver D6), `optifine`, `labymod`, `custom` | 06 §3 |
| `java` | Detección (override, gestionado, PATH, rutas típicas por SO), instalación desde Adoptium con checksum, selección por major | 06 §4 |
| `instance` | Modelo, CRUD, papelera de 30 días, cambio de versión, icono, playtime | 06 §1, 07 |
| `launch` | Plan de lanzamiento (descargas pendientes), natives, classpath, sustitución de argumentos, quickPlay/`--server`, resolución, proceso, logs, crash, salida | 06 §5–6 |
| `logs` | Parser de líneas `[HH:MM:SS] [Hilo/NIVEL]`, continuaciones, anillo de 5000 líneas por partida, archivo de log propio | 06 §6 |
| `doctor` | Firmas de diagnóstico (tabla de 06 §7), `FixAction`, informe | 06 §7 |
| `content` | Modrinth (`api.modrinth.com/v2`), CurseForge (API oficial con clave del usuario), instalar, activar/desactivar, borrar, registro de procedencia, dependencias, actualizaciones | 06 §9 |
| `modpack` | `.mrpack` y zip de CurseForge: crear instancia, descargar, `overrides` | 06 §9 |
| `backup` | Zip de la instancia, retención con fijados, restaurar, planificador | 06 §8 |
| `world` | `level.dat` (fastnbt + flate2), renombrar, editar, borrar, datapacks | 06 §8 |
| `import` | Roxy (D4), `.minecraft` oficial, Prism/MultiMC, carpeta suelta | 06 §9, 07 |
| `servers` | Lista local editable de servidores rápidos; Server List Ping (TCP, handshake + status) con caché TTL | 03, 08 |
| `rpc` | Discord Rich Presence con application id propio; nombre de instancia y versión | 06 §6 |
| `events` | Bus de eventos (`tokio::sync::broadcast`) | 04 (eventos `mc://*`) |

### 3.2 Fachada

`Core` es un handle clonable (`Arc` interno) creado con `Core::open(Options)`; `Options` permite fijar el directorio de datos (tests, `--data-dir` en la CLI). Métodos asíncronos agrupados por área, por ejemplo:

```rust
core.accounts().begin_msa_login() -> DeviceCode
core.accounts().complete_msa_login(&DeviceCode, CancelToken) -> Account
core.instances().create(NewInstance) -> Instance
core.launch(&instance_id, LaunchOptions) -> GameHandle
core.content().search(Query) -> Page<Project>
core.subscribe() -> broadcast::Receiver<Event>
```

Las operaciones largas aceptan un `CancelToken` y publican progreso en el bus con un `task_id`.

### 3.3 Eventos

```rust
enum Event {
    Progress { task_id, stage, done, total, unit },   // archivos o bytes
    TaskFinished { task_id, result },
    GameLog { game_id, line: LogLine },
    GameCrashed { game_id, crash_report: Option<PathBuf> },
    GameExited { game_id, instance_id, code, seconds_played },
    DoctorAlert { instance_id },
    LanOpened { game_id, port } / LanClosed { game_id },
}
```

La CLI los imprime (barra de progreso con `indicatif`, logs en la terminal). La app los recibe por un `StreamSink` de frb.

### 3.4 Errores

`thiserror` en `core` con una enum por área, unificada en `lcore::Error`. Cada variante lleva un `kind` estable (cadena) para que la UI traduzca el mensaje y Doctor pueda reaccionar. Ejemplos: `JavaVersionRequired { major }`, `HashMismatch`, `Offline`, `XboxError { xerr }`, `NotEntitled`, `AppNotApproved` (403 de `login_with_xbox`).

### 3.5 Concurrencia y procesos

- Runtime tokio multihilo. El trabajo de CPU o disco pesado (zip, hash de archivos grandes, extracción) va a `spawn_blocking`.
- Cada partida es un `GameHandle` con id propio. Se permiten varias si `settings.allow_multiple_games` está activo.
- El proceso del juego se lanza en su propio grupo de procesos para que sobreviva si se cierra el launcher (D1: «durante la partida el launcher se puede cerrar»). Si el launcher se cierra, la partida sigue y el playtime se recupera en el siguiente arranque a partir de la marca de inicio guardada.
- Salida detectada por EOF de stdout/stderr más `wait()`, sin sondeo de 15 s.

### 3.6 Secretos

- Refresh token de Microsoft: en el keyring del SO (Secret Service, Credential Manager, Keychain), servicio `<app-id>.msa`, usuario = UUID de la cuenta de Minecraft.
- Token de acceso de Minecraft: en memoria. Opcionalmente en el keyring con su caducidad, para no rehacer la cadena en cada arranque.
- Clave de CurseForge: en el keyring, servicio `<app-id>.curseforge`.
- Si no hay keyring (p. ej. sesión sin Secret Service): error claro y opción explícita de guardar en un archivo con permisos `0600`, desactivada por defecto.
- Nunca se escriben tokens en logs; el comando de lanzamiento que se registra enmascara `--accessToken`.

### 3.7 CLI

```
shard [--data-dir DIR] [--offline] [--json]
  account  login | list | use <id> | remove <id> | add-offline <nombre> | refresh
  version  list [--snapshots] | install <id>
  loader   versions <fabric|quilt|forge|neoforge> <mc>
  java     list | install <major>
  instance list | create <nombre> --mc <v> [--loader L --loader-version V] | show | edit | delete | restore | purge
  launch   <instancia> [--server host:port] [--account id]
  content  search <q> [--type mod|resourcepack|shader|datapack] [--instance id] | install | list | toggle | remove
  modpack  install <archivo|url>
  backup   create | list | restore | delete | pin
  world    list | show | rename
  doctor   <instancia> [--fix <id>]
  import   roxy [--dry-run] | dot-minecraft <ruta> | prism <ruta>
  servers  list | add | ping <host>
```

`--json` imprime resultados en JSON para pruebas y scripts.

### 3.8 Pruebas

- **Unitarias** en cada módulo: reglas de SO y arquitectura, fusión de versiones, sustitución de argumentos, slugs, parser de logs, firmas de Doctor, retención de backups, rutas seguras.
- **Golden** (`insta`): línea de comandos completa generada a partir de fixtures reales (vanilla 1.8.9 legacy, 1.20.1, 26.2; perfil Fabric; JSON de Forge y NeoForge instalados) para linux, windows y macos.
- **Integración HTTP** con `wiremock`: cadena de login completa (device code pendiente, `slow_down`, éxito, XErr 2148916233, 403 de app no aprobada), refresco con rotación, descargas con hash incorrecto y reintentos, caché con 304.
- **Disco** con `tempfile`: crear, editar, borrar e importar instancias; importación de Roxy sobre un árbol falso que imita el real (sin tokens).
- **Red real** marcadas `#[ignore]` (se lanzan con `cargo test -- --ignored`): instalar vanilla y Fabric en un directorio temporal, instalar Java.
- **Humo manual** con la CLI: lanzar una instancia offline y una con cuenta de Microsoft.
- `cargo clippy -D warnings` y `cargo fmt --check` antes de cada commit.

## 4. Datos en disco

Raíz `<data>`: `~/.local/share/<app-id>` (Linux), `%APPDATA%\<app-id>` (Windows), `~/Library/Application Support/<app-id>` (macOS). Configurable con `--data-dir` o la variable `SHARD_DATA_DIR`.

```
<data>/
  settings.json            ajustes globales (idioma, tema, cierre, memoria y JVM por defecto, rpc…)
  accounts.json            cuentas: uuid, nombre, tipo (msa|offline), skin en caché, cuenta activa. Sin tokens
  servers.json             servidores rápidos
  cache/http/              respuestas JSON con metadatos de caché
  versions/<id>/<id>.json  JSON de versión tal cual (Mojang, perfiles de loaders, instaladores)
  versions/<id>/<id>.jar   cliente
  libraries/               layout Maven (compatible con el de Roxy y Mojang)
  assets/{indexes,objects,skins}/
  natives/<version-id>/
  java/<major>/            JRE gestionados
  installers/              caché de instaladores
  instances/<id>/
    instance.json          metadatos propios (esquema §4.1)
    content.json           procedencia de mods, resourcepacks, shaders (equivale a .modrinth_sources.json)
    icon.png
    logs/                  logs de cada partida (rotados)
    game/                  --gameDir: mods, config, saves, options.txt…
  backups/<instance-id>/   zips y backups.json
  trash/<instance-id>/     instancias borradas (30 días)
  logs/                    logs del launcher
```

### 4.1 `instance.json`

```json
{
  "schema": 1,
  "id": "fabulously-optimized",
  "name": "Principal",
  "game": { "mc": "26.2", "loader": "fabric", "loader_version": "0.19.3" },
  "created_at": 1785816370,
  "last_played": 1787450000,
  "playtime_seconds": 0,
  "java": { "path": null, "memory_mb": 12288, "jvm_args": ["-XX:+UseG1GC", "..."] },
  "window": { "width": null, "height": null, "fullscreen": false },
  "backups": { "auto": true, "interval_hours": 72, "max_count": 3 },
  "dev_console": false,
  "imported_from": { "kind": "roxy", "original_id": "fabulously-optimized-1785816370" }
}
```

Diferencias con el original: id estable sin epoch (con sufijo si choca), `jvm_args` como lista, campos agrupados, `schema` para migraciones, papelera fuera del JSON (carpeta `trash/`).

## 5. App (`app/`)

- Flutter estable (versión fijada con fvm en `app/.fvmrc`), Dart 3.
- `flutter_rust_bridge` 2.x estable (la última estable vista es 2.12.0; las 2.13/2.14 están en beta). Se fija la versión exacta en el hito 4.
- Estado: Riverpod. Rutas: go_router. i18n: `gen-l10n` con ARB es y en (textos propios, no copiados del original).
- Pantallas (equivalentes a `research/03`): arranque, cuentas (añadir MS con device code y offline), Home (jugar, selector de cuenta y versión, servidores rápidos), Instancias (lista, detalle con pestañas mods, mundos, capturas, backups, ajustes, consola), Contenido (buscar en Modrinth y CurseForge), Skins, Ajustes, Doctor (modal), consola en ventana aparte o panel.
- Diseño propio: tema oscuro con acento derivado del fondo (paleta extraída en local), blur y oscurecido configurables, base 1280×800 redimensionable, decoraciones de ventana propias (`window_manager`). Los assets del original no se usan.
- Bandeja, autostart, instancia única y notificaciones con plugins de Flutter de escritorio (`tray_manager`, `launch_at_startup`, `local_notifier` o equivalentes, a validar en el hito 4).

## 6. Empaquetado y actualizaciones

- Linux: AppImage y Flatpak; además un `PKGBUILD` local para el equipo propio.
- Windows: instalador (Inno Setup o MSIX). macOS: `.dmg` sin firma.
- Actualizaciones: comprobación contra releases propias (home-server o repo privado) con firma minisign propia, o delegar en Flatpak. No bloquea el arranque.
- CI: más adelante; de entrada, `cargo test` y `flutter build linux` en local.

## 7. Desconocidos que afectan a la spec

- Aprobación de la app de Azure por Mojang: plazo y si la conceden a proyectos personales (ver `docs/guia-azure.md`). Sin aprobación, `login_with_xbox` devuelve 403 y solo hay cuentas offline.
- Forge/NeoForge: si el instalador oficial con `--installClient` funciona sin GUI en todos los SO y versiones antiguas (el original solo lo usó en Linux). Alternativa: ejecutar los processors como Prism o Modrinth App.
- Plugins de Flutter de escritorio para bandeja y autostart en Wayland: validar en el hito 4.
