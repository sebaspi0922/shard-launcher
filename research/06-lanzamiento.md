# Instalación y lanzamiento de instancias (análisis estático de Roxy 0.6.0)

Fuentes: `extracted/appimage/usr/bin/roxy-launcher` (Rust, con `.symtab`: los nombres `tauri_app_lib::minecraft::*` se resuelven con `nm -C`), pool de cadenas (`.rodata`), `extracted/frontend-pretty/index.js` y estructura de `~/.local/share/.roxy/game` (solo lectura; no se leyó `session.json`, `launcher_account.json` ni ningún `latest_launch.log`).
Método: REA/Ghidra no terminó (timeout de 330 s en `open/search/pseudo_code`). Se usó `nm`, `objdump` y capstone con un índice propio de referencias a cadenas por función. Direcciones = VA del ELF.
Marcas: **[C]** confirmado en código/cadena/disco; **[I]** inferido; **[?]** desconocido.

Nota sobre el binario: las plantillas `format!` están compactadas en `.rodata` como `<len><texto>` + marcador de argumento `0xC0 <n>`, no como arrays de `&str`. Por eso las cadenas parecen concatenadas con bytes de control.

## 1. Disposición en disco

Raíz de datos `~/.local/share/.roxy` (`app_dir::app_dir`); `paths::game_root` = `<raíz>/game` [C] (`0x1154e10`).

```
game/
  accounts/<id>/instances/<slug>-<epoch>/    paths::account_instances_root / instance_dir_path
  versions/<versionId>/<versionId>.jar       client.jar vanilla renombrado con el id fusionado
  libraries/<ruta maven>/…jar                layout maven estándar (también los natives-linux.jar)
  natives/<versionId>/{lwjgl,jna,netty}/…    ver nota
  assets/{indexes/<id>.json, objects/<hh>/<sha1>, skins/<hh>/<sha1>}
  java/<major>/bin/java                      JRE Temurin descomprimido (java_runtime_dir)
  installers/                                caché: instaladores Forge/NeoForge y optifine-shading-v1/
  tmp/                                       temporales (modpacks .mrpack/.zip con nombre uuid_like) [I]
  launcher_profiles.json                     solo si se instaló Forge (stub, ver §3)
  quickplay/log.json                         valor de ${quickPlayPath} (se crea el dir en cada lanzamiento) [C]
```

- **<id> de `accounts/`** [I, alta confianza]: es el id de la cuenta Roxy (`LauncherUser.id`), no el UUID de Minecraft. `commands::minecraft::current_account_id` (`0x11cfd30`) solo clona un `Option<String>` guardado en el estado gestionado por `launcher_account::set_current_user_id` (`0x12b3720`); `paths::account_instances_root(account_id)` (`0x1154a70`) hace `game/accounts/<id>/instances`, rechazando ids con separadores (`require_single_path_segment`). `07-datos-locales.md` confirma que el nombre coincide con `launcher_account.id`. El UUID de MC aparece aparte (`mcUuid`, `MinecraftAccountSummary`). Sin cuenta Roxy no hay `current_account_id` y fallan los comandos de instancias → es lo que bloquea el launcher con el backend caído.
- **versions/**: el JSON de versión **no se guarda** (solo el `.jar`). Al lanzar se vuelve a pedir el manifiesto y el perfil del loader (Fabric/Quilt: `…/profile/json`) y se fusionan en memoria [C: `loader::fabric::resolve` no llama a `version_dir`]. En disco hay un único `fabric-loader-0.19.3-26.2/fabric-loader-0.19.3-26.2.jar` (39 MB, firmado MOJANGCS = client vanilla). Excepciones: Forge/NeoForge (el instalador deja `versions/<id>/<id>.json`) y versiones custom importadas (`versions/<id>/<id>.json` + jar).
- **natives/<versionId>/**: `versionId` es el id fusionado (p. ej. `fabric-loader-0.19.3-26.2`), compartido por todas las instancias con el mismo MC+loader. Hoy solo `lwjgl/3.4.1-snapshot/x64/*.so` tiene contenido; `jna/` y `netty/` están vacíos. En los JAR de LWJGL 3.4 los natives viajan dentro de `libraries/org/lwjgl/**/*-natives-linux.jar` (`linux/x64/org/lwjgl/*.so`). `libraries::extract_natives` (`0x12c60f0`) extrae zips respetando `extract.exclude` y con protección zip-slip (`enclosed_name`), pero no se encontró ningún `-Dorg.lwjgl…`/`-Djna.tmpdir`/`-Dio.netty.native.workdir` literal en el binario, así que esas propiedades (si existen) vienen del JSON de Mojang con `${natives_directory}` [I]. La estructura `lwjgl/<ver>/x64` la crea LWJGL en tiempo de ejecución [I].
- **instancia**: id = `slugify(nombre)` (minúsculas, no alfanumérico → `-`, vacío → `instance`) + `-` + epoch en segundos [C: `instance::create`, `0x12c1310`]. El nombre visible puede cambiar sin renombrar la carpeta (`fabulously-optimized-…` se llama "Principal"). El directorio de la instancia **es** el `game_directory` (contiene `mods`, `saves`, `config`, `options.txt`, `latest_launch.log`, `backups/`, `.modrinth_sources.json`, `instance.json`).
- **instance.json** [C, `instance::save` `0x12c0660` + disco], JSON pretty (2 espacios): `id, name, mcVersion, loaderType, loaderVersion|null, createdAt, lastPlayed|null, playtimeSeconds, iconPath, memoryMb|null, jvmArgs|null, resolutionWidth|null, resolutionHeight|null, devMode, backupAutoEnabled, backupIntervalHours|null, backupMaxCount|null, deletedAt` (esta última solo se escribe si es `Some`). `loaderType` ∈ `vanilla|fabric|quilt|forge|neoforge|labymod|custom` (+ perfil OptiFine aparte, §3).
- **Papelera**: `mc_delete_instance` es borrado lógico (`deletedAt` = ahora); `purge_expired_trash` borra definitivamente pasados 2 592 000 s = **30 días** (`0x12bd4e0`, constante `0x278d00`); `mc_restore_deleted_instance`/`mc_purge_instance`. `mc_list_unmanaged_instances`/`mc_adopt_instance_folder`: carpetas sin `instance.json` bajo la raíz de la cuenta.

## 2. Pipeline de instalación/descarga

Entradas: `mc_launch` (versión vanilla suelta; usa la "instancia" `default`), `mc_launch_instance`, `mc_connect_instance_to_server`, `mc_connect_server`, `mc_launch_optifine`. Todas acaban en `launch::run` (`0xefc400`, 37 KB):

1. `launch::launch` (`0xf05710`) = `manifest::resolve_version` → `account::resolve_account` → `paths::instance_dir(…,"default")` → `run`. `launch_instance_with_server` (`0xefb6e0`) = `resolve_account` → `instance_dir` → `loader::resolve` → `run` [C].
2. **Manifiesto**: único origen `https://launchermeta.mojang.com/mc/game/version_manifest_v2.json` (sin espejo BMCLAPI para vanilla; sin caché local ni fallback offline vistos) [C `fetch_manifest` `0xf0ce90`]. Luego `GET <url de la versión>` (error "Couldn't download the version definition (status N)"). `ManifestEntry{id,type,url,releaseTime,…}`; `mc_list_versions` filtra con `default_version_type`.
3. **Fusión** (`VersionInfo::merge_with_base` `0x9e91c0`): concatena `libraries` (`merge_libraries`, descarta duplicados), `arguments.jvm/game` (`ArgEntry`), y el overlay manda en `id/mainClass/…`; el base se resuelve por `inheritsFrom`. `merge_libraries_only` se usa para LabyMod.
4. **Java** (§4): `java::find_java(requiredMajor, override)`; si no hay → error `JavaVersionRequired{requiredMajor}` (el frontend ofrece instalar).
5. **Descargas** con `download::download_all` (tareas `DownloadTask{url,dest,sha1?}`): semáforo de **12** descargas concurrentes (`Semaphore::new(0xc)`, `0xf0c747`) sobre `JoinSet`; por tarea (`0xca22c0`): crea el directorio padre → si el destino existe lee el fichero y compara SHA-1 (coincide → se salta) → `GET` + `error_for_status` → `bytes()` completo en memoria → SHA-1; si no coincide `HashMismatch` ("Downloaded file '…' doesn't match the expected hash (it may be corrupted)") → escribe `<dest>.part` y `rename` → envía `ProgressUpdate`. **No hay reintentos** en la tarea [C]. Se emite `mc://progress` `{done,total}` (nº de ficheros; la UI muestra "Downloading done/total").
   - client jar: `versions/<id>/<id>.jar` desde `downloads.client` (`VersionInfo::client_download`); en loaders fusionados se descarga el del base.
   - libraries: `Library::resolve_artifact` (`downloads.artifact`, o por coordenada maven: `maven_coordinate_to_path` + `url` de la lib o `https://libraries.minecraft.net` por defecto) y `resolve_classifier_artifact` (natives clásicos: `natives.linux` → classifier). Filtradas con `rules::rules_allow` (`0x96eb40`): solo `os.name == "linux"` y `os.arch == "x86_64"` (cualquier otra arch con regla se descarta); `features` = mapa de flags activos. Si una lib no tiene URL ni existe (típico de Forge/NeoForge: artefactos generados por el instalador) → error "library X should have been generated locally by the mod loader installer but is missing — launch the instance again to re-run the installer" [C].
   - assets: índice `assetIndex.url` → `assets/indexes/<id>.json` (copia literal del JSON de Mojang; hoy `32.json`, 586 KB, 5 919 objetos) → `AssetIndexFile.objects` → `https://resources.download.minecraft.net/<hh>/<hash>` a `assets/objects/<hh>/<hash>` [C]. Segunda ronda de `download_all` independiente de la de libs/jar.
   - natives clásicos: tras descargar, `spawn_blocking(libraries::download_libraries{closure#1})` → `extract_natives` ("extraction failed unexpectedly" si falla).
6. `${classpath}`: `build_classpath(libs, client_jar)` pone el **jar del cliente al final**, separador `:` (`0x12b7ea0`).
7. Reglas por SO: el código solo conoce `linux`/`x86_64` (cadena `linux` y `x64` fijas para Adoptium); no hay literales `windows/osx` en la ruta de lanzamiento (el original es un port de un launcher Windows: hay mensajes "Program Files", `mc_relaunch_elevated` → "Elevation is only supported on Windows").
8. HTTP: un `reqwest::Client` global (`http::CLIENT`, OnceLock), rustls; respeta `SSL_CERT_FILE/SSL_CERT_DIR` (cadenas vistas) y logs "using proxy"; User-Agent de Modrinth `github.com/sasuke/minecraft-launcher/1.0 (modrinth-integration)`. Timeouts globales del cliente: [?].

## 3. Loaders (`loader::resolve`, `0xf06000`, despacha por `loaderType`)

| Loader | Listado de versiones | Resolución / instalación |
|---|---|---|
| Vanilla | `mc_list_versions` | `resolve_version` |
| **Fabric** | `GET https://meta.fabricmc.net/v2/versions/loader/<mc>` → `[{loader:{version,…}}]`, ordenado con `version_order` (más nuevo primero) | `GET …/loader/<mc>/<loader>/profile/json` → `VersionInfo` (id `fabric-loader-<loader>-<mc>`) → `merge_with_base`. Sin instalador; todo en memoria |
| **Quilt** (existe, comparte el código de Fabric; no hay `quilt.rs`) | `GET https://meta.quiltmc.org/v3/versions/loader/<mc>` | `…/loader/<mc>/<loader>/profile/json`, misma función `fabric::resolve` con otra base [C: `loader::resolve` llama dos veces a `fabric::resolve`] |
| **Forge** | `https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml` (parseo de `<version>`) + `https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json` (`promos["<mc>-recommended|latest"]`) | descarga `<base>/<v>/forge-<v>-installer.jar` a `installers/` y **ejecuta el instalador oficial**: `java -jar <installer> --installClient [<game_root>]` (Java vía `find_any_java`; el argumento de ruta tras `--installClient` es [I]), timeout **300 s** ("installer didn't finish after N minutes"). Antes escribe `game/launcher_profiles.json` = `{"profiles":{},"settings":{},"version":3}` (`ensure_launcher_profiles_stub`). El instalador ejecuta sus processors; Roxy luego busca el directorio nuevo en `versions/` (`list_version_dirs` antes/después, `find_version_dir_by_loader_version`, `install_is_complete`), lee su JSON y `merge_with_base`. Si no queda completo, error con `installer_log_hint`. Instaladores antiguos: `try_legacy_install` (`install_profile.json` con `install.filePath`/`install.path`/`versionInfo.id`: extrae el universal jar y el JSON a mano) |
| **NeoForge** | `https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml` | igual que Forge (`neoforge-<v>-installer.jar`, `--installClient`) |
| **OptiFine** (`mc_launch_optifine`, perfil global, no loader de instancia) | `GET https://bmclapi2.bangbang93.com/optifine/<mc>` → `[OptiFineBuild{…3 campos}]` (error "no OptiFine build found for Minecraft X") | descarga el jar desde el mismo mirror BMCLAPI (timeout **120 s**) a `installers/`, quita firmas del vanilla (`<…>-vanilla-unsigned.jar`, `strip_jar_signature`), ejecuta `java -cp <optifine.jar> optifine.Patcher <vanilla-unsigned.jar> <optifine.jar> <parcheado>` (timeout **120 s**, `patched_jar_path`), y construye un "shading jar" en `installers/optifine-shading-v1/` (`build_shading_jar`). El jar parcheado reemplaza al cliente [I] |
| **LabyMod 4** | `https://laby-releases.s3.de.io.cloud.ovh.net/api/v1/manifest/<canal [I]>/latest.json` (`LabyModManifest{tag?, commitReference, minecraftVersions[…customManifestUrl…]}`) | `…/api/v1/libraries/<…>.json` (`LibrariesResponse`), jar `…/api/v1/download/labymod4/<canal>/<commit>.jar` → `libraries/net/labymod/LabyMod/<commit>/LabyMod-<commit>.jar` (`net.labymod:LabyMod:<commit>`); id tipo `<…>-labymod4-<…>`; `merge_libraries_only`. Error "LabyMod doesn't support Minecraft X" |
| **Custom** (`mc_import_custom_version {sourcePath}`) | — | acepta carpeta o `version.json` (error si hay 0 o >1 `.json`); parsea `VersionInfo`, deduce MC con `inheritsFrom` o `assetIndex` (`guess_mc_version`), escribe `versions/<id>/<id>.json` y copia los `.jar` hermanos. En el lanzamiento: jar de `inheritsFrom`, o `downloads.client`, o el jar importado que contenga `net/minecraft/client/main/Main.class` + `net/minecraft/client/Minecraft.class` (`find_self_sufficient_client_jar`, p. ej. OptiFine/Forge "universal"); si no, error "has no inheritsFrom, no downloads.client, and no matching .jar was imported…" |

Acciones relacionadas: `mc_change_instance_version` (cambia `mcVersion/loaderType/loaderVersion` en `instance.json`, no toca mundos/mods), `mc_clear_installer_cache` (vacía `installers/`), Doctor `ReinstallLoader`.

## 4. Java gestionado

- Versión requerida = `javaVersion.majorVersion` del JSON fusionado (struct `JavaVersionInfo{majorVersion}`; no se guarda `component`).
- `java::find_java` (`0x12b9ab0`): (1) override `javaPath` (ajuste global) solo si `java -version` (`java_major_version`, parsea stderr) da **exactamente** el major requerido; si no, `eprintln` "find_java: ignoring Java path override (…) — this Minecraft version needs Java N"; (2) `game/java/<major>/bin/java`; (3) candidatos de `PATH` (`path_candidates`) con igualdad de major [I: comparación `cmp/jne`, sin `>=`]. `find_any_java` (cualquier JRE de `java/*` o `PATH`) se usa para instaladores Forge/OptiFine.
- Instalación (`mc_install_java {major}`, `java_install::install_java` `0xcdf240`): `GET https://api.adoptium.net/v3/assets/latest/<major>/hotspot?image_type=jre&os=linux&architecture=x64` → `[AdoptiumAsset{binary:{package:{link,name}}}]` (no se lee ningún checksum → **sin verificación de hash del JRE** [C por forma del struct]). Descarga `<archivo>.part` con progreso `mc://progress` (`done/total`, probablemente bytes con `content_length` [I]) → renombra → `extract_archive` (`0x12b4f90`: `.zip` con zip-slip; resto `tar.gz` vía flate2+tar) en un directorio `…-extract-tmp`, se mueve la carpeta raíz a `java/<major>`, `chmod` de `bin/java`. Bucle de reintentos con `sleep` para errores de acceso transitorios ("os error 5", "access is denied", `is_transient_access_error`) y mensaje "install_java: download attempt N failed (…), retrying". Error "Java N isn't available for this platform (HTTP S)".
- `java/25/release` actual: Temurin 25.0.4+7, imagen JRE.

## 5. Argumentos JVM y de juego

Orden final del comando [C por flujo de `run` y `apply_java_args` `0x1155040`]:

```
<java> [-Xmx<memoryMb>M | -Xmx2G]  -Xms512M
       [-javaagent:<authlib-injector.jar>=<BACKEND>/yggdrasil  -Dauthlibinjector.side=client]   # solo cuentas offline
       <jvm args del JSON (arguments.jvm) o, si no existe (JSON legacy): -Djava.library.path=<natives> -cp <classpath>>
       <jvmArgs de la instancia/global, partidos por espacios>
       <mainClass> <game args>
```

- **Memoria** [C]: `memoryMb` de la instancia (u32 opcional) → `-Xmx{}M`; si es `None` → `-Xmx2G`; `-Xms512M` siempre. La UI siembra 4096 y dice "Applied automatically from the RAM slider: -Xms512M -Xmx{N}M". `mc_launch`/`mc_launch_optifine` reciben `memoryMb`/`jvmArgs`/`javaPath` del ajuste global (`globalMemoryMb`, `globalJvmArgs`, `javaPath`); `mc_launch_instance` no recibe memoria (sale de `instance.json`). No se vio ningún tope contra la RAM del sistema (`mc_get_system_memory_mb` usa `sysinfo`; solo informa y alimenta Doctor).
- **jvmArgs del usuario**: texto libre, `split_whitespace`, se añaden **después** (ganan en `-Xmx`, `-XX`). Valor por defecto que la UI inyecta cuando está vacío (`jb()`, idéntico al `instance.json` real): `-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 -XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC -XX:+AlwaysPreTouch -XX:G1NewSizePercent=30 -XX:G1MaxNewSizePercent=40 -XX:G1HeapRegionSize=8M -XX:G1ReservePercent=20 -XX:G1HeapWastePercent=5 -XX:G1MixedGCCountTarget=4 -XX:InitiatingHeapOccupancyPercent=15 -XX:G1MixedGCLiveThresholdPercent=90 -XX:G1RSetUpdatingPauseTimePercent=5 -XX:SurvivorRatio=32 -XX:+PerfDisableSharedMem -XX:MaxTenuringThreshold=1`. (Posible origen del bug "26.2 no cargaba por flags JVM" del Discord; `Doctor` reconoce "Unrecognized VM option" y ofrece quitar el flag.)
- **Plantilla y sustituciones** (`args::placeholders` `0x12b6df0`, `args::substitute` `0x12b67e0`): `auth_player_name, version_name, game_directory, assets_root, game_assets, assets_index_name, auth_uuid, auth_access_token, auth_session, user_type, version_type, natives_directory, launcher_name, launcher_version, classpath, library_directory, classpath_separator, user_properties, quickPlayMultiplayer, quickPlayPath`.
  - `launcher_name` = literal `SasukeMC-Launcher` [C]; `user_type` = `msa` (premium) | `legacy` (offline); `auth_access_token` offline = un único carácter (probablemente `0`) [I]; `auth_uuid` offline = `account::offline_uuid` (`0xa76480`: MD5 de `"OfflinePlayer:<nombre>"`, UUID v3 estilo Java) [C por md5 + cadena]; `quickPlayPath` = `game/quickplay/log.json`.
  - **No** hay placeholders de resolución (`resolution_width/height`) ni `--width/--height` literales: `resolutionWidth/Height` se guarda y se muestra, pero no se encontró código que los aplique (coincide con el bug del Discord "la resolución no se guardaba") [C ausencia].
  - Features de reglas (`args::enabled_features`): solo `has_quick_plays_support` e `is_quick_play_multiplayer`, y únicamente si se pidió conexión a servidor. `supports_quick_play_multiplayer` (`0x12b8590`) mira si el JSON trae el feature `is_quick_play_multiplayer`.
  - Juego: `arguments.game` filtrado por reglas, o `minecraftArguments` (legacy) partido por espacios.
- **Conexión directa** (`mc_connect_server`, `mc_connect_instance_to_server`, `serverAddress`): si el JSON soporta quickPlay → `--quickPlayMultiplayer ${quickPlayMultiplayer}` (= `host:port`); si no, fallback `--server <host> --port <port>` (puerto por defecto `25565`; split por `:`) [C `0xefe78c`].
- **authlib-injector** [C]: versión 1.2.8, `https://authlib-injector.yushi.moe/artifact/56/authlib-injector-1.2.8.jar`, SHA-1 fijado `0e0e66d8a4f91a26f33b9c09f5cdffce4a11f0b8`; si el jar guardado (`authlib-injector.jar`) no coincide se descarga (`.part` → rename). Args (`authlib::jvm_args` `0xa76760`): `-javaagent:<jar>=<LAUNCHER_BACKEND_URL>/yggdrasil` y `-Dauthlibinjector.side=client`; el backend de Roxy (`LAUNCHER_BACKEND_URL`, por defecto `https://api.roxylauncher.com`) hace de servidor Yggdrasil para dar skins a cuentas offline (`offlineSkinHash`). Si falla: " authlib-injector no disponible (…) — se lanza sin skin custom". Condición exacta de uso (¿solo offline?) y carpeta del jar: [?]. Para una reimplementación sin backend hay que reemplazarlo o dropear las skins offline.
- **cape-provider**: `.mods/cape-provider/texture-cache` pertenece al **mod Cape Provider** (Modrinth, `mods/cape-provider-5.5.0.jar`), no al launcher.
- **Elevación** (`mc_relaunch_elevated`): solo Windows; en Linux devuelve "Elevation is only supported on Windows". La UI lo ofrece tras fallar `mc_install_java` ("Retry as Administrator").
- **Entorno del proceso**: no se fija ninguna variable de entorno (sin `LD_LIBRARY_PATH`, `DRI_PRIME`, etc.) [C ausencia de literales].

## 6. Proceso, logs, crash, playtime

- `tokio::process::Command` con stdout/stderr en pipe (`spawn_and_watch`, `launch.rs`): tres tareas: lector de stdout, lector de stderr (líneas → canal `(LogStream, String)`), consumidor del canal, y una tarea de espera. Se vuelca además `latest_launch.log` en el directorio de la instancia (contenido exacto no inspeccionado por regla del proyecto; el backup lo incluye) [C nombre].
- **Consumidor** (`0xca0f50`): `log_console::build_log_line` parsea `[HH:MM:SS] [Hilo/NIVEL] …` con `take_bracket_group`; niveles `TRACE DEBUG INFO WARN(ING) ERROR/SEVERE FATAL` (+`unknown`); `isContinuation` para líneas sin cabecera (stack traces). Campos de `LogLine`: `instanceId, seq, timestampMs, level, logger, thread, raw, isContinuation, stream(stdout|stderr)`. Se guarda en `LogBufferState` (**anillo de 5000 líneas** por instancia, `mc_get_log_backlog`) y se emite `mc://log`. Si la línea contiene `Crash report saved to:` / `crash report has been saved to:` → `log_console::crash_report_path` y evento `mc://crash {instanceId, crashReportPath}`.
- **Espera/salida** (`0xca1970`): `try_wait` en bucle con `sleep` de 15 s (constante `0xf`, pasada a `tokio::time::sleep`; seguramente complementa la detección por EOF de los pipes [I]) → al terminar: `launch::record_playtime` → `instance::record_play_session` (suma segundos de `Instant::elapsed` a `playtimeSeconds` y actualiza `lastPlayed`), `discord_rpc::set_idle_activity_if_enabled`, `mc://doctor-alert` si la salida fue anómala (la UI muestra "Minecraft closed unexpectedly… Diagnose"), y `mc://exited` con `{code, secondsPlayed, instanceId}`; si `RunningGameState` tenía "salir al cerrar el juego" (`request_quit_when_game_closes`) → `AppHandle::exit`.
- **Estado**: `RunningGameState` guarda un único proceso (`is_game_running`, `mc_game_running`): el frontend impide lanzar una segunda instancia ("alreadyRunning"). No hay multi-instancia de juego.
- **devMode**: abre una ventana de consola (`build_console_window`, `mc_open_console_window`) con el log coloreado/filtrable; no cambia argumentos. Error "couldn't open the console window for '…'".
- **LAN**: tras lanzar, `lan_watch::spawn` vigila los puertos del juego (`netstat2`) y líneas "Started serving on port"/"Published LAN server on port" → eventos `mc://lan-*` (fuera de este alcance).
- **Ajustes de cierre** (`local_settings.json` + ajustes en backend): `closeBehavior` ∈ `ask|tray|quit` (`local_settings_set_close_behavior`, `quit_launcher`), `allowMultipleInstances` (single-instance vía D-Bus `org.SingleInstance.DBus`, aplica al reabrir el launcher), `closeOnLaunch` (oculta la ventana al lanzar), `keepLauncherOpen` (la vuelve a mostrar en `mc://exited`). Eventos de progreso: `mc://progress {done,total}`.
- **Playtime de cuenta**: `launcher_account::session_timer` (tick de 60 s) llama a `add_playtime` del backend si el juego corre o la ventana principal está visible; independiente del `playtimeSeconds` local.
- **Discord RPC** (`discord_rpc.rs`, crate `discord-rich-presence` IPC unix): application id `1527679900063109140` [C]. Reposo: `details="Eligiendo qué jugar"`, `state="En el inicio del launcher"`, imagen grande `launcher_icon`/"Roxy Launcher", botones "Jugar"→`https://roxylauncher.com` y "Discord"→`https://discord.roxylauncher.com`. Jugando: imagen grande `minecraft_icon`, pequeña `launcher_icon`, botón "Descargar Roxy Launcher"→`https://roxylauncher.com`; textos con nombre de instancia/versión. Se enciende con `mc_rpc_set_enabled` / `discordRpcEnabled`; `mc_rpc_set_idle`.

## 7. Roxy Doctor

`doctor_diagnose {instanceId}` (`0xc8c5a0`): carga `instance.json`, busca el crash report más reciente en `crash-reports/` y el `hs_err_pid*.log` más reciente en la instancia (`find_latest_*`, por mtime), lee la cola de `latest_launch.log` (`read_tail`), lista mods instalados, resuelve la versión (para la Java requerida), `find_java`, memoria total (`sysinfo`) y ejecuta `signatures::run_all` (`0x11560e0`, coincidencias en minúsculas con `contains`). Devuelve `DoctorReport{healthy, diagnoses[], crashExcerpt, crashReportPath}`; cada `Diagnosis{id, kind, category, severity(critical|warning|info), title, explanation, confidence(%), culprit, params, fix}` (orden por severidad/confianza). Eventos `mc://doctor-alert`, `doctor://progress`. Categorías UI: `java, memory, jvm_args, mod_conflict, missing_dependency, corrupted_file, graphics_driver, loader, account, network, system, world_corruption, unknown`.

`FixAction` (`doctor_apply_fix`, `0xc89cd0`): `DisableMod{…}` (renombra a `.disabled`), `SetMemory{mb}` ("Memory allocation set to N MB for this instance."), `InstallJava{major}`, `RemoveJvmArg{needle}` (`Remove "X" from JVM arguments`), `ClearJvmArgs` ("Cleared this instance's custom JVM arguments."), `DeleteInstanceFile{relative_path}` (`safe_instance_file` restringe la ruta; "Deleted "X" — it'll be regenerated…"), `ReinstallLoader` (borra la instalación del loader; "Removed the current loader install… your mods, worlds and settings are untouched"), `ClearWorldLocks` (borra `saves/*/session.lock`: "Removed N leftover world lock file(s)").

Firmas (`kind`, disparador textual, arreglo) [C: cadenas + i18n `roxyDoctorDiagnoses`]:

| kind | Disparador (log/crash en minúsculas) | Fix |
|---|---|---|
| `java_version_mismatch` | `unsupportedclassversionerror`, "has been compiled by a more recent version of the java runtime", "unsupported class file major version" (params `required`, `mcVersion`) | InstallJava |
| `out_of_memory` | `java.lang.outofmemoryerror`, "out of memory" (params `current`,`total`) | SetMemory |
| `reserve_space_failed` | "could not reserve enough space for" (`suggested`) | SetMemory |
| `bad_jvm_flag` / `bad_jvm_flag_unknown` / `bad_jvm_flag_no_custom_args` | "unrecognized vm option", "unrecognized option", "could not create the java virtual machine", "error occurred during initialization of vm" | RemoveJvmArg / ClearJvmArgs |
| `mixin_crash_known` / `_unknown` | `mixinapplyerror`, `mixintransformererror`, "mixin"+"failed to apply"; culpable por `mixins.<x>.json` → `find_mod_file_matching` | DisableMod |
| `forge_dependency_version_mismatch` | "Unsupported installed mandatory/optional dependencies:", `Mod ID:`, `Requested by:`, `Expected range:`, `Actual version:` | — |
| `missing_dependency` | "missing or unsupported mandatory dependencies", "mod_menu requires", "requires {", "requires version " | — |
| `forge_mod_resolution_failed` | "mods were rejected by forge's mod resolution", "errors were found", "mod loading" | — |
| `incompatible_mod_set_fabric` | "incompatible mod set" | — |
| `duplicate_mods` | "duplicate mod", "found a duplicate mod", "Duplicate mod ID:" | DisableMod |
| `corrupted_archive` | "zip end header not found", "invalid cen header", "zip file is corrupt", "error in opening zip file", "not a valid zip file" | reinstalar/quitar el mod |
| `corrupted_options` | "options.txt"+"failed to load/parse" | DeleteInstanceFile(options.txt) |
| `corrupted_servers_dat` | `servers.dat` ilegible | DeleteInstanceFile(servers.dat) |
| `session_lock_corrupted` | "failed to check session lock" | ClearWorldLocks |
| `antivirus_quarantined_file` | `filenotfoundexception` sobre un `.jar` | — |
| `world_tick_crash_known` / `_unknown` | "description: ticking entity/block entity/exception ticking world|level/exception generating new chunk"; "Entity Type"/"Block Entity Type" | — (no repara mundos) |
| `verify_error` | `java.lang.verifyerror` | — |
| `native_driver_crash` | "a fatal error has been detected by the java runtime environment" + "problematic frame" con `nvoglv, nvidia, amd, vlk, atifxig9icd, intel, lwjgl, opengl32.dll` → GPU NVIDIA/AMD/Intel | — |
| `optifine_incompatibility` | `nosuchmethoderror`/`classcastexception` con OptiFine instalado | DisableMod |
| `loader_install_broken` | `noclassdeffounderror: net/minecraftforge|net/neoforged|net/fabricmc`, "could not find or load main class", "a jni error has occurred", "missing or corrupt launcher profile" (loaders Fabric/Quilt/LabyMod/Custom…) | ReinstallLoader |
| `wrong_loader_mod` | `wrong_loader_mod_diagnosis`: mod Forge en instancia NeoForge o viceversa, etc. | DisableMod |
| `incompatible_pair_optifine_sodium`, `…_optifine_iris`, `…_phosphor_starlight`, `…_fabricproxy_fabricproxy-lite` | pares de mods presentes en `mods/` | DisableMod |
| `vanilla_opengl_not_supported` | "not compatible with minimum opengl version" | — |
| `pixel_format_not_accelerated` | "pixel format not accelerated" | — |
| `realms_notavailable` | "failed to verify username" | — |
| `address_already_in_use` | "address already in use" | — |
| `too_many_open_files` | "too many open files" | — |
| `disk_full` | "no space left on device" | — |
| `access_denied_writing` | `java.io.ioexception` + acceso denegado | — |
| `curseforge_missing_dependency` | `noclassdeffounderror` de clase ausente (dependencia) | — |
| `unknown_crash` | nada coincide (muestra extracto: línea `Description:` o primer `Caused by:`) | — |

La columna *Fix* está inferida del texto/UI salvo donde la acción aparece nombrada en el binario (`FixAction`); los disparadores son las cadenas literales de `signatures.rs`.

Además, sin crash, comprueba Java instalada vs requerida, memoria y flags/mods (mensaje "No issues found: Java, memory, your mods and this instance's settings all check out").

## 8. Backups y mundos

- **Backups** (`backup.rs`): carpeta `instances/<id>/backups/` con `backup-<epoch inicio>.zip` (deflate, `zip_dir_recursive` recursivo sobre **todo el directorio de la instancia** excepto `backups/`; incluye `instance.json`, `latest_launch.log`, `.modrinth_sources.json`…; ~600–740 MB en la instancia real) y `.backups.json` (pretty) = `[{id:"backup-<epoch>", createdAt, sizeBytes, pinned, label|null, automatic}]` [C: disco + `load/save_entries`]. `createdAt` es el instante en que termina; `id` el de inicio. API: `mc_create_backup {label,pinned}`, `mc_list_backups`, `mc_delete_backup`, `mc_toggle_backup_pin`, `mc_restore_backup` (extrae el zip sobre la instancia con `enclosed_name`; no vacía antes [I]), `mc_update_backup_settings {backupAutoEnabled, backupIntervalHours, backupMaxCount}`.
- **Retención**: tras crear, `sort_by_key(createdAt)` sobre las entradas filtradas y borra las más antiguas por encima de `backupMaxCount`; se saltan las `pinned` [I, por la estructura de `create_backup`]. Defaults de UI: 24 h y 5 copias; los de la instancia real: 72 h / 3.
- **Scheduler** (`backup_scheduler::spawn`, `0xce3d90`): bucle `sleep(900 s)` → `run_once` (`0x12b5f80`): requiere el estado `LauncherAccountState` (sesión Roxy), recorre `instance::list`, y por cada instancia con `backupAutoEnabled` crea backup `automatic=true` si `ahora − último automático ≥ backupIntervalHours × 3600` (`imul 0xe10`). Errores: `eprintln "backup_scheduler: auto-backup failed for instance …"`.
- **Mundos** (`world.rs`, `commands::world`): `saves/<carpeta>/`. `read_level_dat`/`write_level_dat`: `level.dat` = NBT gzip, leído/escrito con `flate2` + `fastnbt::Value` (`Data` compound; "level.dat has no Data compound"). Campos tocados: `LevelName`, `GameType`, `Difficulty`, `hardcore`, `allowCommands`, `SpawnX/Y/Z`, `BorderCenterX/Z`, `BorderSize`, `GameRules` (+ lectura de `RandomSeed`, `Version`, `DataVersion`, `DayTime`, `LastPlayed`, `DragonFight/DragonKilled`, `ServerBrands`, `DataPacks`). `mc_rename_world` solo reescribe `Data.LevelName` (la carpeta no se renombra [I]); `mc_update_world_details {patch}` aplica `WorldDetailsPatch` (12 campos). `mc_delete_world`, `mc_open_world_folder`. Tamaño con `dir_size`.
- **Datapacks**: `saves/<w>/datapacks/`; desactivar = mover a `datapacks/.disabled_packs/` (`mc_toggle_datapack`); `mc_install_datapack {worldFolders[], downloadUrl, filename, sha1,…}` descarga a cada mundo; `mc_check_datapack_installed`. Metadatos Modrinth en el `.modrinth_sources.json` del directorio del mundo (`load_source_meta_at`) [I].

## 9. Modrinth / CurseForge en la instancia

- **API Modrinth**: directa a `https://api.modrinth.com/v2` (search con `facets` = `project_type`, `versions:<mc>`, `categories:<loader>`; `index` por defecto `relevance`; `/project/<id>/version`, `/version/<id>`, `/projects?ids=`); comandos `modrinth_search, _get_versions, _get_version_by_id, _get_projects_by_ids, _install, _install_local_file, _install_modpack, _check_installed, _list_installed, _toggle_file, _delete_file`. La UI resuelve dependencias `required` (hasta 3 niveles) antes de instalar.
- **CurseForge**: **no** va directo; usa el backend de Roxy (`/curseforge/search`, `/curseforge/categories`, `/curseforge/mods/<id>/files`, `/curseforge/files/bulk`; la clave de API está en el servidor). Ids `cf:<modId>` en la UI; clases 6 (mod), 12 (resourcepack), 6552 (shader). Hay que sustituirlo (clave propia de CF o solo Modrinth).
- **Ficheros**: `category_dir`: `mod→mods/`, `resourcepack→resourcepacks/`, `shader→shaderpacks/`. Descarga con verificación SHA-1.
- **`.modrinth_sources.json`** (en la raíz de la instancia, JSON pretty, `HashMap`): clave `"<tipo>/<nombre de archivo>"` (`mod/…`, `resourcepack/…`; `.disabled` incluido cuando está desactivado) → `{download_url, sha1, project_id, project_name, version_number, mod_category}` (los tres últimos y `mod_category` salen `null` en instalaciones por modpack). Instalación individual rellena `project_id/project_name/version_number/mod_category` desde la UI [C: disco + `ModrinthSourceMeta`].
- **Activación**: `toggle_file` renombra `<f>` ↔ `<f>.disabled` y re-clavea la entrada de `.modrinth_sources.json` (visto en disco: `mod/xaerominimap-….jar.disabled`). `list_installed` mezcla ficheros reales + metadatos (`InstalledFile{source:"modrinth"|…}`).
- **Modpacks** (`modrinth_install_modpack {instanceName, downloadUrl}`): descarga el `.mrpack` a `game/tmp/<uuid_like>.mrpack` [I], lee `modrinth.index.json` (`MrpackIndex{files[{path,hashes,env,downloads,fileSize}], dependencies}`), `loader_from_dependencies` (`minecraft`, `fabric-loader`, `quilt-loader`, `forge`, `neoforge`), crea la instancia, descarga los `files` (filtra por `env`, `download_all` con SHA-1) y extrae `overrides/` y `client-overrides/` (`extract_overrides`, zip-slip protegido; error "overrides extraction panicked"); escribe `.modrinth_sources.json` con `source_meta_key_for_mrpack_path`. CurseForge (`curseforge_install_modpack`): `manifest.json` (`minecraft.modLoaders[].id` → `parse_loader_id`, `files[{projectID,fileID}]`), resuelve URLs por `/curseforge/files/bulk` ("Couldn't resolve CurseForge modpack files"), extrae `overrides/`. Progreso por `mc://progress`.
- **Importación de instancias** (`mc_inspect_import_source`, `mc_import_instance`, `…_into_instance`, `…_from_instance`): reconoce `.minecraft` oficial (`launcher_profiles.json`, `launcher_accounts.json`) y Prism/MultiMC (`instance.cfg`, `mmc-pack.json`, `components`, `uid`); secciones copiables: `saves, mods, config, resourcepacks, shaderpacks, screenshots, options.txt, servers.dat`. `mc_default_dot_minecraft_path`. Sincroniza metadatos con el backend (`sync_instance_upsert`, `sync_instance_files` → `/me/instances`, `/me/instances/<id>/files`) — a eliminar.

## 10. Desconocidos / pendiente

- Dónde se guarda `authlib-injector.jar` y la condición exacta de uso (offline vs cualquier cuenta con skin); contenido de `latest_launch.log` (no leído).
- Origen de los subdirectorios `natives/<id>/{lwjgl,jna,netty}` (¿JVM props del JSON de Mojang 26.x?). No se pudo contrastar con el JSON de 26.2 (no persiste en disco).
- Ningún uso de `resolutionWidth/Height` en el lanzamiento (posible bug del original); `devMode` solo abre la consola.
- Timeouts/proxy del `reqwest::Client` global, pool de hilos y política de errores HTTP más allá de `error_for_status`.
- Comportamiento ante manifiesto inaccesible (sin caché: probablemente falla el lanzamiento, incluso con todo descargado).
- `createdAt` de backups y su orden exacto de borrado (retención con `pinned`) solo por estructura de código, no por datos.
- Mapeo exacto NeoForge↔MC, formato del `id` de LabyMod y proceso exacto de `install_is_complete` (Forge).
- Valor real de `auth_access_token` offline (1 byte) y `version_type`.
- `try_wait` cada 15 s: no se confirmó si también hay detección por EOF de pipes.
- Qué hace `installPath` (ajuste del backend) frente al `game_root` fijo; `mc_relaunch_elevated` en Linux es un error.
- Mundos: no se leyó el cuerpo completo de `get_world_details` (p. ej. cómo se obtiene `RandomSeed` en 1.16+, `WorldGenSettings`).
