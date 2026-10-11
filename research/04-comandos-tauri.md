# 04 - Catálogo de comandos Tauri y eventos

Área: contrato IPC entre el frontend (React, `extracted/frontend-pretty/index.js`) y el backend Rust (`tauri_app_lib`). Evidencia estática únicamente; no se ejecutó nada ni se contactó ningún servidor.

## 0. Cómo leer este documento

- **Fuente de la lista**: 200 llamadas literales `K("cmd", {...})` propias (index.js:12639 define `K`), más llamadas a plugins de Tauri (sección 3). No hay comandos con nombre dinámico: `K(` solo se invoca con literales (comprobado: todo `K(` con primer argumento no literal es la propia definición).
- **Confirmación en el binario**: los 200 nombres aparecen en el binario (los de ≤16 B solo troceados en comparaciones inmediatas del `match` de despacho, p. ej. `launcher_list_ne|ws`). Listas de registro en `roxy-launcher`: 0x41ec00-0x420600 (nombres + nombres de argumentos camelCase, en orden inverso) y 0x4451a3 (+8103 B, nombres ≥16 B). Los símbolos Rust v0 (`13tauri_app_lib8commands<N><módulo><M><fn>`, p. ej. 0x2805325 `commands::lan::lan_active_join`) dan el módulo.
- **Convenciones de columnas**: "Args (JS)" = claves del objeto que pasa la UI (Tauri convierte camelCase a snake_case del parámetro Rust). "Resultado" = lo que la UI lee; entre paréntesis el struct serde del binario cuando se identifica (nombre y nº de campos de `struct X with N elements`). "Línea / pantalla" = línea de definición del wrapper (`def`) y usos (pantalla inferida por rango de líneas; los wrappers de cuenta se consumen a través del contexto `Bi()`/`useLauncherAuth`, p. ej. pantalla de login 92825-93100 y modal de cuenta 38457). "Pista de backend" = inferencia por rutas HTTP/cadenas estáticas; los verbos HTTP y rutas marcadas "inferido" no están verificados.
- Serialización: structs en `camelCase` (serde `rename_all`). Excepciones snake_case: respuestas de Modrinth crudas (`project_id`, `total_hits`, ...) y `auth_begin/complete_device_login` (`user_code`, `skin_url`).
- **Forma de error** (todo comando que falla): `{kind: string, message: string}` (index.js:28693 `Li`, 35588 `tn`); algunos añaden `requiredMajor`. `kind` (variantes de varios enums de error; 0x420756 y 0x4409f8-0x440acb): `Network, InvalidResponse, InvalidCredentials, RateLimited, KeyringStorage` (+`message`), `ExpiredToken, AuthorizationDeclined, Cancelled, NoXboxAccount, ChildAccountNotAllowed, XstsRejected, NoGameOwnership` (auth Microsoft), `VersionNotFound, LoaderNotSupported, HashMismatch, JavaNotFound, JavaVersionRequired{required_major}, JavaInstallFailed, ProcessSpawnFailed, Extraction, Lan` (minecraft/lan), `NotAuthenticated` (0x30d210). La UI actúa especialmente sobre `NotAuthenticated` (premium sin sesión MS -> pide contraseña y llama `launcher_restore_microsoft_account`, index.js:45633), `JavaVersionRequired` (35674, 45641) e `InvalidCredentials` (44448).
- **Caché de UI** (`Bs(clave, fn, ttl)` index.js:29095): resultados cacheados en memoria por clave (news/videos/servers 5 min, skins propias 15 min, battlepass 30 s, icono de instancia 10 min).
- **Latido de presencia**: `launcher_update_presence(isOnline=true)` cada 60 s mientras hay sesión (index.js:28860-28862).
- Los tokens (access/refresh/device, refresh token de Microsoft) nunca llegan a JS: viven en Rust (`launcher_account::store`, error `KeyringStorage`). No se leyó `session.json` ni `launcher_account.json`.

### Módulos backend (según símbolos y rutas de pánico)

| Módulo | Comandos |
|---|---|
| `commands/launcher_account.rs` | `launcher_*` (cuenta, amigos, ajustes, noticias, ranking, actualización) |
| `commands/auth.rs` | `auth_*` (Microsoft device flow, perfil/skin/capa de Minecraft) |
| `commands/minecraft.rs` | `mc_*` (lanzar, instancias, backups, imports, iconos, capturas), `quit_launcher` |
| `commands/world.rs` | `mc_*world*`, `mc_*datapack*` |
| `commands/modrinth.rs`, `curseforge.rs` | `modrinth_*`, `curseforge_*` |
| `commands/shared_instances.rs` | `shared_*`, `group_*`, `activity_*` |
| `commands/lan.rs` | `lan_*` (+ `mc_game_running`: sin símbolo localizado; el frontend lo agrupa con LAN, index.js:36486) |
| `commands/ecosystem.rs` | `ecosystem_*`, `battlepass_*` |
| `commands/skins_catalog.rs` | `skins_catalog_*` |
| `commands/doctor.rs`, `updater.rs` | `doctor_*`, `updater_*` |
| local_settings (módulo propio) | `local_settings_*` |

## 1. Comandos propios (200)

### 1.1 Cuenta del launcher (`launcher_account.rs`, cliente HTTP propio)

| Comando | Args (JS) | Resultado (campos leídos / struct) | Línea / pantalla | Pista de backend |
|---|---|---|---|---|
| `launcher_register` | email, username, password | `{kind:"loggedIn"}` o `{kind:"needsEmailVerify", pendingTicket, email, username}` | def 28703; AuthProvider 28879 | POST /auth/register (RegisterResponse: pendingTicket,email,username) |
| `launcher_login` | email, password, deviceLabel (la UI nunca lo pasa) | `{kind:"loggedIn"}` o `{kind:"needsEmailVerify"\|otro, pendingTicket}` (OTP de login) | def 28706; AuthProvider 28871 | POST /auth/login; AuthResponse: accessToken, refreshToken, deviceToken, user(LauncherUser). Los tokens se quedan en Rust (store.rs, keyring) |
| `launcher_verify_code` | pendingTicket, code | `{kind:"loggedIn"}` o `{kind:"needsPasswordReset", resetTicket}` | def 28709; AuthProvider 28909 | POST /auth/verify |
| `launcher_resend_code` | pendingTicket | string: nuevo `pendingTicket` | def 28712; AuthProvider 28915 | POST /auth/resend |
| `launcher_forgot_password` | email | string: `pendingTicket` (flujo "passwordReset") | def 28715; AuthProvider 28916 | POST /auth/forgot-password |
| `launcher_reset_password` | resetTicket, newPassword | void | def 28718; AuthProvider 28917 | POST /auth/reset-password |
| `launcher_resend_verification` | - | void | def 28721; AuthProvider 28918 | POST /me/resend-verification |
| `launcher_restore_session` | - | perfil (`MeResponse`) o null; la UI pasa a `authed`/`anonymous` | def 28724; AuthProvider 28838 | Rust lee la sesión cacheada y hace POST /auth/refresh; se llama una vez al arrancar (`pZ`, memoizado) |
| `launcher_get_profile` | - | `MeResponse`: email, username, level, rank, totalPlaytimeSeconds, emailVerifiedAt, hasCompletedTutorial, google{email}, discord{username}, minecraftAccounts[{id, mcUsername, mcUuid, accountType:"PREMIUM"\|"CRACKED", offlineSkinHash, offlineSkinModel:"slim"\|"classic"}] | def 28727; AuthProvider 28867,28873,28881 | GET /me |
| `launcher_complete_tutorial` | - | void | def 28730; Tutorial 42814,42880 | POST /me/tutorial/complete |
| `launcher_logout` | - | void | def 28733; AuthProvider 28920 | POST /auth/logout; antes la UI manda presence offline |
| `launcher_update_profile` | email, username | void (la UI recarga el perfil) | def 28736; AuthProvider 28923 | PATCH /me (inferido) |
| `launcher_change_password` | currentPassword, newPassword | void | def 28739; AuthProvider 28928 | POST /me/change-password |
| `launcher_link_microsoft_account` | - (usa el resultado de `auth_complete_device_login`) | void | def 28742; ProfilesProvider 29061 | Envía el refresh token de Microsoft (`microsoftRefreshToken`) a POST /me/minecraft-accounts; error propio "no Microsoft refresh token to link" |
| `launcher_restore_microsoft_account` | accountId, password (contraseña del launcher) | void; error `InvalidCredentials` -> mensaje propio | def 28745; Modal contraseña sesión MS 44445 | Recupera en este equipo el token MS guardado en el backend (cifrado con la contraseña); reintenta si falla (log "retry also failed") |
| `launcher_add_offline_account` | username | void (perfil recargado) | def 28748; ProfilesProvider 29064 | POST /me/minecraft-accounts (accountType CRACKED). La UI valida `^[A-Za-z0-9_]{3,16}$` |
| `launcher_remove_minecraft_account` | id | void | def 28751; ProfilesProvider 29067 | DELETE /me/minecraft-accounts/{id} |
| `launcher_set_offline_skin_file` | accountId, filePath, model:"slim"\|"classic" | void | def 29011; Editor de skin 88699 | Sube PNG al backend; el hash queda en `offlineSkinHash` y se sirve de `{backendBase}/offline-skins/{hash}[/face\|/bust]?v=3` |
| `launcher_set_offline_skin_url` | accountId, url, model | void | def 29018; Editor de skin 88700 | Igual, descargando desde URL |
| `launcher_reset_offline_skin` | accountId | void | def 29025; Editor de skin 88727 | Borra la skin offline del backend |
| `launcher_link_discord` | - | void (abre OAuth de Discord en navegador; callback en 127.0.0.1:32145) | def 28780; Modal de cuenta 38484 | `discord_oauth.rs`, luego POST /me/discord/link |
| `launcher_unlink_discord` | - | void | def 28783; Modal de cuenta 38494 | DELETE /me/discord |
| `launcher_google_sign_in` | deviceLabel (no usado) | `{kind:"loggedIn"}` \| `{kind:"needsRegistration", email, suggestedUsername, pendingTicket}` \| `{kind:"needsLink"(o similar), email, pendingTicket}` | def 28786; AuthProvider 28891 | OAuth Google en navegador, callback loopback 127.0.0.1:32146 (`google_oauth.rs`); POST /auth/google |
| `launcher_google_complete_registration` | pendingTicket, username, password | void (sesión iniciada) | def 28789; AuthProvider 28899 | POST /auth/google/complete-registration |
| `launcher_google_link_existing` | pendingTicket, password | void (sesión iniciada) | def 28796; AuthProvider 28904 | POST /auth/google/link-existing |
| `launcher_link_google` | - | void | def 28799; Modal de cuenta 38504 | OAuth Google + POST /me/google/link |
| `launcher_unlink_google` | - | void | def 28802; Modal de cuenta 38514 | DELETE /me/google |
| `launcher_list_friends` | - | `FriendUser[]`: id, username, isOnline, currentMcVersion, currentInstanceName, currentSharedInstanceId | def 28766; Panel de amigos 37387; Instancia compartida 50692 | GET /me/friends. Sondeo cada ~N s mientras el panel está abierto |
| `launcher_list_friend_requests` | - | `[{id, sender:{id, username}}]` | def 28757; Panel de amigos 37388 | GET /me/friends/requests |
| `launcher_send_friend_request` | username | void | def 28754; Panel de amigos 37440 | POST /me/friends/requests |
| `launcher_accept_friend_request` | requestId | void | def 28760; Panel de amigos 37457 | POST /me/friends/requests/{id}/accept (inferido) |
| `launcher_decline_friend_request` | requestId | void | def 28763; Panel de amigos 37465 | POST .../decline |
| `launcher_remove_friend` | friendId | void | def 28769; Panel de amigos 37473 | DELETE /me/friends/{id} |
| `launcher_update_presence` | isOnline, currentMcVersion, currentInstanceName, currentSharedInstanceId (todos opcionales salvo isOnline) | void | def 28772; AuthProvider 28860,28862,28920; Layout (exit) 43047; Home 45579; Página Instancias 57853 | PUT /me/presence. Latido cada 60 s mientras `authed`; también al lanzar y al cerrar el juego |
| `launcher_list_news` | - | `NewsItem[]`: id, title, description, imageUrl, link, category, author, createdAt, placement ("sidebar"\|otro) | def 28805; Home 45518 | GET /news |
| `launcher_list_videos` | - | `VideoItem[]`: url (YouTube) + order; la UI resuelve título/miniatura con oEmbed de YouTube | def 28808; Home 45519 | GET /videos |
| `launcher_list_servers` | - | `GameServerItem[]`: id, name?, ip, description, bannerUrl, iconUrl, isFeatured, status{online, playersOnline, playersMax, version, checkedAt}, isQuickPlayDefault, quickPlayOrder | def 28811; Menú rápido 40827; Home 45520; Servidores 46341 | GET /servers |
| `launcher_list_featured_content` | kind: "MOD"\|"RESOURCEPACK"\|"SHADER"\|"MODPACK"\|"DATAPACK" | `FeaturedContentItem[]` (20 campos): id, kind, title, description, source, author, fileName, downloadUrl, sha1, linkButtonLabel/Color, downloads, ... | def 28814; Mods/Descubrir 60900 | GET /featured-content?kind= |
| `launcher_get_online_count` | - | number (jugadores online) | def 28817; Barra superior 40209 | GET /stats/public (OnlineCountResponse) |
| `launcher_get_leaderboard` | limit (50) | `[{position, username, level, rank, totalPlaytimeSeconds}]` (la UI desenvuelve `entries`) | def 28820; Ranking 64001 | GET /stats/leaderboard?limit= |
| `launcher_get_referral_leaderboard` | limit (50) | `[{position, username, confirmedReferrals}]` | def 28823; Ranking 64008 | GET /stats/referral-leaderboard?limit= |
| `launcher_check_update` | - | `{currentVersion, latestRelease:{version, notes, downloadUrl,...}\|null, updateAvailable, updateRequired}` | def 28826; Ajustes 59535; Puerta de actualización 93652 | GET /launcher/releases/current (LauncherReleaseInfo: version, notes, channel, artifacts[ReleaseArtifactInfo], fileSizeBytes, hasSignature, downloadUrl...); compara versiones en `version.rs` |
| `launcher_get_settings` | - | `LauncherSettings` (ver 1.1b) | def 28829; Modal unirse LAN 36742; Barra superior 40674; Menú rápido 40728,40736; Home 45538 | GET /me/settings, con caché local (`set_cached_settings`/`refresh_cached_settings`) |
| `launcher_update_settings` | partial: objeto parcial de LauncherSettings | `LauncherSettings` completo | def 28832; RunningInstanceProvider 35668; Barra superior 40681; Menú rápido 40773; Home 45533 | PATCH /me/settings; la UI agrupa cambios con debounce de 500 ms |

### 1.2 Ajustes locales y ciclo de vida de la app

| Comando | Args (JS) | Resultado (campos leídos / struct) | Línea / pantalla | Pista de backend |
|---|---|---|---|---|
| `local_settings_get` | - | `{allowMultipleInstances:boolean, closeBehavior:"ask"\|"tray"\|"quit"}` | def 36273; Shell/cierre 36284; Ajustes 59465 | Archivo local (no pasa por red) |
| `local_settings_set_allow_multiple_instances` | allow | void | def 36276; Ajustes 59483 | Archivo local |
| `local_settings_set_close_behavior` | behavior: "ask"\|"tray"\|"quit" | void | def 36279; Ajustes 59475; Diálogo cierre 92328 | Archivo local |
| `quit_launcher` | - | void (termina el proceso) | def 36305; Shell/cierre 36316; Diálogo cierre 92329 | `app.exit`; se usa tras cerrar la ventana si closeBehavior="quit" |
| `mc_rpc_set_enabled` | enabled | void | def 29411; Menú rápido 40779; Ajustes 59504 | Discord Rich Presence (`discordRpcEnabled`) |
| `mc_rpc_set_idle` | - | void | def 29408; Home 45541 | Pone la RPC en estado "idle" al abrir Home |

### 2.1 Lanzamiento y runtime (`minecraft.rs`; instalación en `src/minecraft/*`)

| Comando | Args (JS) | Resultado (campos leídos / struct) | Línea / pantalla | Pista de backend |
|---|---|---|---|---|
| `mc_launch` | versionId, username, isPremium, memoryMb?, jvmArgs?, javaPath? | void; la promesa resuelve cuando el proceso del juego ya se lanzó (descarga/instalación antes, con eventos `mc://progress`) | def 29158; Menú rápido 40795; Home 45606 | Instala versión vanilla (manifest launchermeta.mojang.com `version_manifest_v2.json`), libs, assets, natives; auth Microsoft en `auth/*` |
| `mc_launch_optifine` | mcVersion, username, isPremium, loaderVersion?, memoryMb?, jvmArgs?, javaPath? | void | def 29168; Home 45597 | Descarga build OptiFine (espejo bmclapi2.bangbang93.com/optifine). La UI guarda `lastPlayedVersionId = "optifine:<mc>"` |
| `mc_launch_instance` | instanceId, username, isPremium, javaPath? | void. Errores: `{kind:"NotAuthenticated"}` (premium sin sesión MS -> modal contraseña), `{kind:"JavaVersionRequired", requiredMajor}` | def 29306; RunningInstanceProvider 35666; Home 45590 | Usa la config de la instancia (memoryMb, jvmArgs, resolución, devMode); instala loader (Fabric/Quilt/Forge/NeoForge/LabyMod/custom); crea `latest_launch.log`; vigila crash |
| `mc_connect_server` | versionId, username, isPremium, serverAddress, memoryMb?, jvmArgs?, javaPath? | void | def 29397; Modal unirse LAN 36774; Panel de amigos 37612; Menú rápido 40844; Home 45282; Servidores 46358 | Igual que mc_launch con quick-play (`--quickPlayMultiplayer`) al servidor |
| `mc_connect_instance_to_server` | instanceId, serverAddress, username, isPremium, javaPath? | void | def 29314; Modal unirse LAN 36773; Menú rápido 40835; Home/quickplay 45281 | Igual que mc_launch_instance con quick-play |
| `mc_game_running` | - | boolean | def 36487; Modal unirse LAN 36727 | Estado del proceso hijo |
| `mc_open_console_window` | instanceId | void. Abre una ventana Tauri nueva con etiqueta `console-<instanceId>` que carga el mismo bundle (index.js:96184,96207) y renderiza solo la consola | def 40694; Menú rápido 41114 | WebviewWindow en Rust |
| `mc_get_log_backlog` | instanceId | `LogLine[]` (hasta ~5000 en la UI): instanceId, seq, stream ("stdout"/"stderr"), timestampMs, level ("trace"/"debug"/"info"/"warn"/"error"/"fatal"/"unknown"), logger, thread, raw, isContinuation | def 40697; Consola 94424 | Buffer circular por instancia; la UI deduplica con `seq` frente a `mc://log` |
| `mc_install_java` | major | void; progreso por `mc://progress` | def 29179; Modal Java requerido 44288 | Adoptium (api.adoptium.net/v3/assets/latest/...), extrae a dir propio |
| `mc_relaunch_elevated` | - | void (relanza el launcher con privilegios) | def 29182; Modal Java requerido 44300 | Ruta de error "couldn't extract the downloaded files (antivirus)"; típico de Windows |
| `mc_get_system_memory_mb` | - | number (RAM total en MB; la UI usa 16384 como respaldo) | def 29190; hook de memoria del dashboard 51764 (por defecto 16384) | sysinfo |
| `mc_list_versions` | - | `[{id, type:"release"\|"snapshot"\|..., releaseTime}]` | def 29198; Menú rápido 40746; Crear instancia 43784; Crear/importar instancia 47252; Unirse/importar (social) 48460 | version_manifest_v2.json de Mojang |
| `mc_list_loader_versions` | loaderType ("fabric"\|"quilt"\|"forge"\|"neoforge"\|"optifine"\|"labymod"), mcVersion | `string[]` de versiones de loader (la UI toma [0]); vacío = no disponible | def 29300; Menú rápido 40808; Crear instancia 43939; Crear/importar instancia 47267; Unirse/importar (social) 48474 | meta.fabricmc.net/v2/versions/loader, meta.quiltmc.org/v3/versions/loader, files.minecraftforge.net promotions_slim.json, maven.neoforged.net, bmclapi OptiFine, manifest LabyMod (laby-releases.s3...) |
| `mc_import_custom_version` | sourcePath (JSON de versión o carpeta) | `{id, mcVersion, ...}` (summary) | def 29303; Crear/importar instancia 47354 | Copia un version.json personalizado (loader "custom") |
| `mc_get_disk_usage` | - | `{instancesBytes, sharedCacheBytes, installersBytes}` | def 29391; Ajustes 59456,59525 | `dir_size_bytes` |
| `mc_clear_installer_cache` | - | void | def 29394; Ajustes 59524 | Borra caché de instaladores |

### 2.2 Instancias (`minecraft.rs`)

| Comando | Args (JS) | Resultado (campos leídos / struct) | Línea / pantalla | Pista de backend |
|---|---|---|---|---|
| `mc_list_instances` | - | `Instance[]` (18 campos): id, name, mcVersion, loaderType ("vanilla"\|"fabric"\|"quilt"\|"forge"\|"neoforge"\|"labymod"\|"custom"), loaderVersion, createdAt, lastPlayed, playtimeSeconds, memoryMb, jvmArgs, resolutionWidth, resolutionHeight, devMode, backupAutoEnabled, backupIntervalHours, backupMaxCount, deletedAt | def 29264; Menú rápido 40731,40739,40891; Home 45505; Crear/importar instancia 47245; Instancias 57718,57807 | Carpeta de instancias; `modrinth.index.json`/`manifest.json` por instancia; sincroniza con /me/instances (`sync_instance_upsert/delete`) |
| `mc_create_instance` | name, mcVersion, loaderType, loaderVersion (null en vanilla) | `Instance` | def 29201; Modal unirse LAN 36751; Menú rápido (quick instance) 40810; Unirse social 48563; Instancias 57834 | Crea carpeta, instala loader al primer lanzamiento |
| `mc_delete_instance` | id | void (a papelera) | def 29270; Dashboard instancia 52695 | Marca `deletedAt` (papelera restaurable) |
| `mc_list_trash_instances` | - | `Instance[]` con deletedAt | def 29273; Papelera instancias 49341; Página Instancias 57723 | - |
| `mc_restore_deleted_instance` | id | `Instance` | def 29276; Papelera instancias 49352 | - |
| `mc_purge_instance` | id | void (borrado definitivo) | def 29279; Papelera instancias 49374 | - |
| `mc_open_instance_folder` | id | void | def 29282; Menú rápido 41164; Dashboard instancia 52666; Consola 95872 | tauri-plugin-opener / xdg-open |
| `mc_open_instance_config_folder` | id | void | def 29285; Dashboard instancia 52680 | idem (`config/`) |
| `mc_list_account_instances` | - | `RemoteInstance[]`: id, name, mcVersion, loaderType, loaderVersion, lastPlayed (instancias guardadas en la cuenta, no presentes en local) | def 29288; Página Instancias 57750 | GET /me/instances |
| `mc_restore_instance` | id, name, mcVersion, loaderType, loaderVersion (objeto RemoteInstance aplanado) | `{instance, restored[], failed[], skippedNoSource[], remoteLookupFailed}` | def 29291; Página Instancias 57759 | Recrea la instancia y reinstala mods desde Modrinth/CurseForge usando `RemoteInstanceFile` (GET /me/instances/{id}/files) |
| `mc_update_instance_settings` | instanceId, name, memoryMb, jvmArgs, resolutionWidth?, resolutionHeight?, devMode | `Instance` | def 29349; Dashboard instancia 52496 | - |
| `mc_change_instance_version` | instanceId, mcVersion, loaderType, loaderVersion | `Instance` | def 29368; Dashboard instancia 52453 | Elimina el loader instalado y se baja de nuevo en el siguiente lanzamiento (textos "Removed the current loader install...") |
| `mc_get_instance_icon` | instanceId | string data-URL o null (la UI cachea 10 min por `instance-icon:{id}`) | def 29332; Dashboard instancia 52548,52612,52659 | Lee `icon.png` de la instancia |
| `mc_set_instance_icon_from_screenshot` | instanceId, filename | void (invalida caché de icono) | def 29335; Dashboard instancia 52611 | Copia screenshot como icono |
| `mc_set_instance_icon_from_file` | instanceId, sourcePath | void (invalida caché) | def 29342; Crear instancia 47377; Dashboard instancia 52658 | - |
| `mc_list_screenshots` | instanceId | `[{filename, takenAt, dataUrl}]` | def 29323; Dashboard instancia 52541 | Lee `screenshots/` y devuelve miniaturas como data-URL |
| `mc_reveal_screenshot` | instanceId, filename | void | def 29326; Dashboard instancia 52620 | reveal_item_in_dir |
| `mc_delete_screenshot` | instanceId, filename | void | def 29329; Dashboard instancia 52639 | - |
| `mc_create_backup` | instanceId, label?, pinned? | no se lee (la UI relista con mc_list_backups) | def 29376; Menú rápido 41189; Dashboard instancia 52354 | zip de la instancia (zip crate); la UI lo envuelve en una tarea con indicador (`runTask("backup")`) |
| `mc_list_backups` | instanceId | `BackupEntry[]`: id, label, createdAt, sizeBytes, pinned, automatic | def 29379; hooks instancia 52213 | - |
| `mc_delete_backup` | instanceId, backupId | void | def 29382; Dashboard instancia 52365 | - |
| `mc_toggle_backup_pin` | instanceId, backupId | no se lee | def 29385; Dashboard instancia 52375 | - |
| `mc_restore_backup` | instanceId, backupId | void | def 29388; Dashboard instancia 52385 | - |
| `mc_update_backup_settings` | instanceId, backupAutoEnabled, backupIntervalHours, backupMaxCount | no se lee. La UI pasa backupIntervalHours=null si backupAutoEnabled=false | def 29360; Dashboard instancia 52343 | Programador de copias automáticas (hay `ScheduleInterval` en el binario) |

### 2.3 Importación / adopción de instancias

| Comando | Args (JS) | Resultado (campos leídos / struct) | Línea / pantalla | Pista de backend |
|---|---|---|---|---|
| `mc_default_dot_minecraft_path` | - | string ruta de `.minecraft` | def 29249; Crear/importar instancia 47239; Unirse/importar (social) 48453 | - |
| `mc_inspect_import_source` | sourcePath | `{entries:[{key,present,count}], isLikelyDotMinecraft, isPrismOrMultiMcInstance, detectedMcVersion, detectedLoaderType, detectedLoaderVersion, detectedName}` (claves: saves, mods, config, resourcepacks, shaderpacks, screenshots, options.txt, servers.dat) | def 29219; Crear/importar instancia 47035; Unirse/importar (social) 48495 | Lee `mmc-pack.json`, `instance.cfg`, `components` (Prism/MultiMC), `version.json` |
| `mc_inspect_import_source_from_instance` | sourceInstanceId | mismo objeto | def 29222; Crear/importar instancia 47035 | - |
| `mc_import_instance` | name, mcVersion, loaderType, loaderVersion, sourcePath, sections? (claves seleccionadas) | `Instance` | def 29209; Página Instancias 57826,57838 | Copia secciones elegidas |
| `mc_import_instance_from_instance` | name, mcVersion, loaderType, loaderVersion, sourceInstanceId, sections? | `Instance` | def 29225; Página Instancias 57817 | - |
| `mc_import_into_instance` | targetInstanceId, sourcePath, sections? | void | def 29235; Dashboard instancia 55546 | Mezcla secciones en instancia existente |
| `mc_import_into_instance_from_instance` | targetInstanceId, sourceInstanceId, sections? | void | def 29242; Dashboard instancia 55546 | - |
| `mc_list_unmanaged_instances` | - | `[{folderName, preview:{detectedName,detectedMcVersion,detectedLoaderType,detectedLoaderVersion}}]` | def 29252; Página Instancias 57731 | Escanea carpeta de instancias no registradas |
| `mc_adopt_instance_folder` | folderName, name, mcVersion, loaderType, loaderVersion | `Instance` | def 29255; Página Instancias 58150 | - |
| `mc_default_folder_info` | - | `{...preview}` del `.minecraft` por defecto o null (muestra aviso de adopción) | def 29267; Página Instancias 57737 | - |

### 2.4 Mods, packs y datapacks (Modrinth / CurseForge)

| Comando | Args (JS) | Resultado (campos leídos / struct) | Línea / pantalla | Pista de backend |
|---|---|---|---|---|
| `modrinth_search` | query, projectType ("mod"\|"resourcepack"\|"shader"\|"modpack"\|"datapack"), mcVersion?, loader?, category?, index? ("relevance"...), limit=20, offset=0 | JSON crudo Modrinth: `{hits:[{project_id, slug, title, description, icon_url, downloads, author, categories, versions, display_categories, project_type}], total_hits}` | def 51799; Dashboard instancia 54784; Mods/Descubrir 60985,61014 | api.modrinth.com/v2/search con facets; User-Agent propio |
| `modrinth_get_versions` | projectId, projectType, mcVersion?, loader? | Versiones Modrinth crudas: id, version_number, game_versions, loaders, files[{url, filename, primary, hashes{sha1,sha512}, size}], dependencies[{version_id, project_id, file_name, dependency_type}] | def 51834; Dashboard instancia 54808; Mods/Descubrir 61048,61144 | GET /project/{id}/version |
| `modrinth_get_version_by_id` | versionId | una versión (misma forma) | def 51843; resolución de dependencias (`r6`, 52007) | GET /version/{id} |
| `modrinth_get_projects_by_ids` | ids[] | `[{id, slug, title, icon_url, project_type}]` | def 51886; Dashboard instancia (metadatos de mods) 52157 | GET /projects?ids= |
| `modrinth_install` | instanceId, projectType, downloadUrl, filename, sha1, projectId?, projectName?, versionNumber?, modCategory? | void | def 51850; Dashboard instancia 54836,54843; Mods/Descubrir 60943,61076,61083 | Descarga a mods/resourcepacks/shaderpacks y anota metadatos en `modrinth_sources.json`; verifica sha1 (`HashMismatch`) |
| `modrinth_install_local_file` | instanceId, projectType, sourcePath | void | def 51917; Dashboard instancia 52589 | Copia archivo local (drag-and-drop .jar/.zip) |
| `modrinth_check_installed` | instanceId, projectType, projectId | null o `{download_url, sha1, project_id, project_name, version_number, mod_category}` | def 51863; Dashboard instancia 54866; Mods/Descubrir 61105 | Busca en `modrinth_sources.json` |
| `modrinth_list_installed` | instanceId, projectType | `ModFile[]`: filename, enabled, sizeBytes, projectId, projectName, versionNumber, modCategory, source | def 51881; hooks instancia 52132 | Escanea carpeta (incluye .disabled) |
| `modrinth_toggle_file` | instanceId, projectType, filename | void | def 51900; hooks instancia 52176 | Renombra `.jar` <-> `.jar.disabled` |
| `modrinth_delete_file` | instanceId, projectType, filename | void | def 51907; hooks instancia 52187 | - |
| `modrinth_install_modpack` | instanceName, downloadUrl (.mrpack) | no se lee (void) | def 51914; Mods/Descubrir 60966,61174 | Lee `modrinth.index.json` del mrpack (MrpackIndex/MrpackFile, env client/server); error "modpack is missing modrinth.index.json" |
| `curseforge_search` | query, kind ("mod"\|"resourcepack"\|"shader"), mcVersion?, loader?, limit=20, categoryId? | `{hits:[{id, slug, title, description, iconUrl, downloads, author}], totalCount}` | def 51942; Dashboard instancia 54779; Mods/Descubrir 60984 | API CurseForge vía proxy/clave del backend (no hay endpoint directo visible) |
| `curseforge_get_categories` | classId (6 mods, 12 resourcepacks, 6552 shaders) | `CfCategory[]` (id, name, ...) | def 51954; Mods/Descubrir 60858 (filtro de categorías) | - |
| `curseforge_get_files` | modId, mcVersion?, loader? | `[{id, fileName, gameVersions, downloadUrl, sha1, fileLength, dependencies:[modId]}]` | def 51981; Dashboard instancia 54807; Mods/Descubrir 61047,61143 | Usa `BulkFileById` para dependencias; `skippedMods` en modpacks |
| `curseforge_install_modpack` | instanceName, downloadUrl | `{skippedMods: ...}` (mods sin descarga directa; la UI los lista) | def 51989; Mods/Descubrir 61167 | Lee `manifest.json` CurseForge (CfManifest: minecraft{version,modLoaders}, files[{projectID,fileID}]) |
| `mc_list_datapacks` | instanceId, folderName | `[{filename, enabled, sizeBytes, projectName, versionNumber}]` | def 52054; Mundos/datapacks 56226 | `saves/{mundo}/datapacks` |
| `mc_check_datapack_installed` | instanceId, folderName, projectId | null o `{...}` del origen | def 52057; Mundos/datapacks 57164 | - |
| `mc_install_datapack` | instanceId, worldFolders[], downloadUrl, filename, sha1, projectId?, projectName?, versionNumber? | lista de resultados por mundo (`results`; la UI lee folderName y message por entrada) | def 52074; Mundos/datapacks 57146; Mods/Descubrir 61190 | Instala el mismo datapack en varios mundos |
| `mc_toggle_datapack` | instanceId, folderName, filename | no se lee | def 52086; Mundos/datapacks 56483 | - |
| `mc_delete_datapack` | instanceId, folderName, filename | void | def 52089; Mundos/datapacks 56464 | - |

### 2.5 Mundos (`world.rs`)

| Comando | Args (JS) | Resultado (campos leídos / struct) | Línea / pantalla | Pista de backend |
|---|---|---|---|---|
| `mc_list_worlds` | instanceId | `[{folderName, displayName, difficulty, gameType, iconDataUrl, lastPlayed, sizeBytes}]` | def 52032; hooks instancia 52237; Mods/Descubrir 61247 | Lee `level.dat` NBT (fastnbt) de cada mundo |
| `mc_rename_world` | instanceId, folderName, newName | void | def 52035; Dashboard instancia 53870 | Escribe `LevelName` en level.dat |
| `mc_delete_world` | instanceId, folderName | void | def 52038; Dashboard instancia 54656 | - |
| `mc_open_world_folder` | instanceId, folderName | void | def 52041; Dashboard instancia 53884 | - |
| `mc_get_world_details` | instanceId, folderName | `{levelName, difficulty, gameType, hardcore, allowCommands, spawnX, spawnY, spawnZ, borderSize, borderCenterX, borderCenterZ, gameRules}` | def 52044; Mundos/datapacks 56529 | NBT |
| `mc_update_world_details` | instanceId, folderName, patch (WorldDetailsPatch, 12 campos, todos opcionales) | void | def 52047; Mundos/datapacks 56575 | Error "world locked"; `clearWorldLocks` en Doctor |

### 3.1 Instancias compartidas (`shared_instances.rs`; API /me/shared-instances)

| Comando | Args (JS) | Resultado (campos leídos / struct) | Línea / pantalla | Pista de backend |
|---|---|---|---|---|
| `shared_list_instances` | - | `SharedInstanceSummary[]` (12 campos): id, name, mcVersion, loaderType, loaderVersion, memberCount, myRole, mySyncMode ("AUTO"\|"MANUAL"), lastSyncedAt, manifestVersion, isUpToDate, myInstanceId | def 36396; Panel de amigos 37392; Grupos 49815; Instancia compartida 50336 | GET /me/shared-instances |
| `shared_get_instance` | id | `SharedInstanceDetail` (10): id, name, inviteCode, members[{userId, username, role:"OWNER"\|"MEMBER", ...}], myMembership{isUpToDate, syncMode, role, lastSyncedManifestVersion}, manifestVersion, ... | def 36399; Instancia compartida 50339 | GET /me/shared-instances/{id} |
| `shared_create_instance` | instanceId (local), name | `SharedInstanceDetail`/`{id,...}` | def 36393; Instancia compartida 50354 | POST /me/shared-instances + publica manifest de la instancia local |
| `shared_create_invite_code` | id, maxUses?, expiresInHours? | `{inviteCode, expiresAt}` o string | def 36402; Instancia compartida 50712 | POST /me/shared-instances/{id}/invites |
| `shared_revoke_invite_code` | id | void | def 36409; Instancia compartida 50722 | DELETE .../invites |
| `shared_preview_invite` | inviteCode | `{name, mcVersion, loaderType, loaderVersion, ...}` | def 36390; Unirse/importar (social) 48562 | GET /me/shared-instances/preview?inviteCode= |
| `shared_join` | inviteCode, instanceId (local ya creada) | string `sharedInstanceId` | def 36412; Unirse/importar (social) 48569 | POST /me/shared-instances/join |
| `shared_invite_friend` | id, inviteeUserId | void | def 36415; Instancia compartida 50739 | POST .../invites (a un amigo) |
| `shared_get_diff` | id | `{diff:{added[],removed[],updated[]}, manifestVersion,...}` (SharedInstanceDiffResponse, 4 campos) | def 36418; Instancia compartida 50339 | GET .../diff (compara manifest remoto vs. local) |
| `shared_sync` | sharedInstanceId, instanceId, mode: "MERGE_KEEP_MINE"\|"UPDATE_SHARED_ONLY"\|"FULL" | `{installed[], removed[], failed[], skippedNoSource[], manifestVersion, notes}` (la UI usa installed.length/removed.length) | def 36421; Modal unirse LAN 36760,36769; Unirse/importar (social) 48570; Instancia compartida 50500,50759 | Descarga mods nuevos (desde Modrinth/CF por `modrinth_sources.json`) y borra los que ya no están; `undownloadableCount` = archivos sin fuente |
| `shared_publish_manifest` | sharedInstanceId, instanceId | void | def 36424; Instancia compartida 50749 | Calcula manifest (hashes) de la instancia local y lo sube (POST .../manifest) |
| `shared_add_owner` | id, userId | void | def 36427; Instancia compartida 50769 | POST .../owners |
| `shared_remove_owner` | id, userId | void | def 36430; Instancia compartida 50769 | DELETE .../owners/{userId} |
| `shared_set_sync_mode` | id, auto:boolean | void | def 36433; Instancia compartida 50519 | PUT .../membership (syncMode AUTO\|MANUAL) |
| `shared_leave` | id | void | def 36436; Instancia compartida 50529,50787 | DELETE .../membership |
| `shared_delete_instance` | id | void | def 36439; Instancia compartida 50787 | DELETE /me/shared-instances/{id} (se usa cuando es el único miembro) |

### 3.2 Grupos y actividad (`shared_instances.rs`; API /me/groups)

| Comando | Args (JS) | Resultado (campos leídos / struct) | Línea / pantalla | Pista de backend |
|---|---|---|---|---|
| `group_list` | - | `GroupSummary[]` (5): id, name, memberCount, myRole, ... | def 36445; Grupos 49766; Instancia compartida 50695 | GET /me/groups |
| `group_create` | name | `{id,...}` | def 36442; Grupos 49789 | POST /me/groups |
| `group_get` | id | `GroupDetail` (6): id, name, inviteCode, myRole, members[{userId, username, role}], sharedInstances[{id, name, mcVersion, loaderType, memberCount}] | def 36448; Grupos 49775,49824 | GET /me/groups/{id} |
| `group_create_invite_code` | id, expiresInHours? | código (string/obj) | def 36451; Grupos 49850 | POST .../invites |
| `group_revoke_invite_code` | id | void | def 36454; Grupos 49859 | DELETE .../invites |
| `group_join` | inviteCode | string `groupId` | def 36457; Grupos 49803 | POST /me/groups/join |
| `group_link_shared_instance` | id, sharedInstanceId | void | def 36460; Grupos 49823; Instancia compartida 50702 | POST .../shared-instances |
| `group_unlink_shared_instance` | id, sharedInstanceId | void | def 36463; Grupos 49837 | DELETE .../shared-instances/{id} |
| `group_delete` | id | void | def 36466; Grupos 49868 | DELETE /me/groups/{id} |
| `group_leave` | id | void | def 36469; Grupos 49868 | DELETE /me/groups/{id}/members/me (inferido) |
| `activity_list_group_feed` | groupId, limit=30 | `ActivityEvent[]`: id, type, createdAt, user{id,username}, data{sharedInstanceName?, groupName?, level?, summary{added,removed,updated}} | def 36472; Grupos 49777 | GET /me/groups/{id}/activity?limit= |

### 3.3 LAN compartida (`lan.rs`, `lan_client.rs`, `lan_watch.rs`, `lan_tunnel.rs`, `lan_proxy.rs`)

| Comando | Args (JS) | Resultado (campos leídos / struct) | Línea / pantalla | Pista de backend |
|---|---|---|---|---|
| `lan_local_port` | - | number\|null: puerto LAN detectado del juego en curso | def 36481; LanProvider 37314 | `lan_watch` parsea el log ("Started serving on", `parse_lan_port`) y mira `game_listen_ports` |
| `lan_local_address` | - | `{ip, port}` \| null (IP de red local para "Conexión directa") | def 36484; Overlay compartir LAN 37123 | Interfaz local |
| `lan_active_share` | - | string\|null: id de la sesión LAN compartida por mí | def 36490; LanProvider 37317 | Estado en memoria |
| `lan_share_start` | {instanceId, mcVersion, worldName?, visibility:"FRIENDS"\|"GROUP", maxPlayers, groupId?, onlineMode} (objeto directo, sin envoltorio) | string: id de sesión (LanCreateSessionResponse, 5 campos) | def 36493; Overlay compartir LAN 37142 | POST /me/lan/sessions; abre túnel (`lan_tunnel.rs`, WebSocket via tokio-tungstenite) + heartbeat; emite `mc://lan-share-started` |
| `lan_share_stop` | - | void | def 36496; Overlay compartir LAN 37162; LanProvider 37329,37335 | DELETE /me/lan/sessions/{id}; emite `mc://lan-share-stopped` |
| `lan_list_sessions` | - | `LanSessionSummary[]` (11): id, host{id,username}, worldName, mcVersion, loaderType, isModded, playerCount, maxPlayers, onlineMode, ... | def 36475; Panel de amigos 37395 | GET /me/lan/sessions |
| `lan_get_session` | sessionId | `LanSessionDetail` (10): host, mcVersion, worldName, accountWarning, resolution{mode:"VANILLA"\|"NEEDS_JOIN"\|"ALREADY_MEMBER", sharedInstanceId, myInstanceId, isUpToDate, name, mcVersion, loaderType, loaderVersion, manifestVersion, lastSyncedManifestVersion, modCount, undownloadableCount} (LanJoinResolution, 13) | def 36478; Modal unirse LAN 36730 | GET /me/lan/sessions/{id} |
| `lan_adopt` | sessionId, instanceId | void | def 36499; Modal unirse LAN 36758 | Asocia la instancia local a la sesión (para mapear mods) |
| `lan_join` | sessionId | number: puerto local del túnel (`127.0.0.1:<puerto>`) | def 36502; Modal unirse LAN 36771,36785 | POST .../join; abre túnel local (LanJoinResponse, 2 campos); luego la UI lanza el juego con `mc_connect_*` a ese puerto |
| `lan_leave` | - | void | def 36505; LanProvider 37336 | Cierra túnel de cliente |

### 3.4 Ecosistema, tienda y pase de batalla (`ecosystem.rs`; API /me/*, /cosmetics, /marketplace)

| Comando | Args (JS) | Resultado (campos leídos / struct) | Línea / pantalla | Pista de backend |
|---|---|---|---|---|
| `ecosystem_get_wallet` | - | `WalletBalanceInfo[]`: currency ("COINS"), balance (string numérica) | def 36519; EcosystemProvider 36598 | GET /me/wallet |
| `ecosystem_get_inventory` | - | `InventoryItemInfo[]`: id, cosmeticItem, acquiredAt, acquisitionSource, equippedSlots | def 36520; EcosystemProvider 36599 | GET /me/inventory |
| `ecosystem_get_equipped` | - | `[{categoryKey, slot, cosmeticItem}]` (EquippedInfo) | def 36521; EcosystemProvider 36600 | GET /me/equipped |
| `ecosystem_get_categories` | - | `CosmeticCategoryInfo[]` (6): key, name, maxEquipped, sortOrder,... | def 36522; EcosystemProvider 36601 | GET /cosmetics/categories |
| `ecosystem_get_achievements` | - | `AchievementInfo[]` (12): id, name, description, points, unlocked, unlockedAt, isSecret, progress{current, threshold, comparator}, cosmeticItem?, rarity | def 36523; EcosystemProvider 36602 | GET /me/achievements |
| `ecosystem_get_collections` | - | `CollectionInfo[]` (11): id, name, description, totalItems, ownedItems, percentage, items[CollectionEntryInfo{cosmeticItem, owned}] | def 36524; EcosystemProvider 36603 | GET /me/collections |
| `ecosystem_get_events` | - | `EventInfo[]` (10): id, name, description, endsAt, completed, completedAt, progress, ... | def 36525; EcosystemProvider 36604 | GET /me/events |
| `ecosystem_equip` | inventoryItemId, slot=0 | void (la UI recarga todo) | def 36526; EcosystemProvider (`equip`/`unequip` del contexto, ~36637-36642), recarga tras la llamada | POST /me/inventory/equip (inferido) |
| `ecosystem_unequip` | categoryId, slot=0 | void | def 36527; EcosystemProvider 36640 | POST /me/inventory/unequip |
| `ecosystem_get_listings` | category?, search? | `ListingInfo[]`: id, price, currencyType, seller{SellerInfo}, cosmeticItem{id,name,category{key},rarity{key,colorHex},assets[{kind,url}]}, purchaseId? | def 36529; Tienda (Perfil) 90484 | GET /marketplace/listings |
| `ecosystem_purchase` | listingId, idempotencyKey (UUID generado en UI y reutilizado en reintentos) | void/`purchaseId` | def 36533; Perfil/Tienda 90552 | POST /me/marketplace/purchases |
| `ecosystem_report_launcher_opened` | - | `{countedNewDay, dailyRewardGranted, loginDays}` (la UI lee dailyRewardGranted y refresca) | def 36534; EcosystemProvider 36622 | POST /me/launcher-opened (recompensa diaria) |
| `ecosystem_fetch_image_data_url` | url (solo http/https) | string data-URL (imágenes <= tamaño máximo: "Image is too large") | def 36539; Ajustes (fondo/wallpaper) 59361 | Descarga desde Rust (evita CORS); la UI la cachea por URL |
| `battlepass_get_state` | - | `BattlePassStateInfo` (8): active, battlePass{id,name,description,endsAt,imageUrl,premiumPriceCoins,status}, hasPremium, level, isMaxLevel, xpIntoLevel, xpForNextLevel, levels[{level, xpRequired, cumulativeXp, freeReward, premiumReward, freeClaimed, premiumClaimed, unlocked}] | def 36544; Pase de batalla 91666 | GET /me/battlepass (caché UI 30 s) |
| `battlepass_claim_reward` | level, track:"FREE"\|"PREMIUM" | `{granted, rewardGrantId, alreadyGranted, track}` | def 36545; Pase de batalla 91718,91754 | POST /me/battlepass/claim |
| `battlepass_purchase_premium` | idempotencyKey | `BattlePassPurchaseResult` (1 campo) | def 36546; Pase de batalla 91768 | POST /me/battlepass/purchase-premium |

### 4. Microsoft / Minecraft (`auth.rs`, `auth/*`)

| Comando | Args (JS) | Resultado (campos leídos / struct) | Línea / pantalla | Pista de backend |
|---|---|---|---|---|
| `auth_begin_device_login` | - | `{user_code, verification_uri, expires_in}` (la UI lo mapea a userCode/verificationUri/expiresIn) | def 38436; Modal de cuenta 38552 | login.microsoftonline.com/consumers/oauth2/v2.0/devicecode (device flow) |
| `auth_complete_device_login` | - | `{uuid, username, skin_url}`; errores `{kind: ExpiredToken\|AuthorizationDeclined\|Cancelled\|NoXboxAccount\|ChildAccountNotAllowed\|NoGameOwnership\|Network}` | def 38440; Modal de cuenta 38555 | Sondea /token, luego XBL (user.auth.xboxlive.com) -> XSTS -> api.minecraftservices.com/authentication/login_with_xbox -> /entitlements/mcstore -> /minecraft/profile; guarda refresh token en keyring (`auth/session.rs`) |
| `auth_cancel_device_login` | - | void | def 38444; Modal de cuenta 38531,38565 | Cancela el sondeo (se llama al cerrar el modal) |
| `auth_get_own_skins` | - | `{uuid, username, skins:[{id,state:"ACTIVE"\|...,url,variant:"CLASSIC"\|"SLIM"}], capes:[{id,state,url,alias}]}` | def 88401; Editor de skin 88603,88731,88744 | GET api.minecraftservices.com/minecraft/profile |
| `auth_apply_skin_by_url` | url, variant:"CLASSIC"\|"SLIM" | perfil actualizado (misma forma) | def 88404; Editor de skin 88709 | POST /minecraft/profile/skins (JSON url) |
| `auth_upload_skin_file` | filePath, variant | perfil actualizado | def 88407; Editor de skin 88708 | POST multipart a /minecraft/profile/skins |
| `auth_reset_skin` | - | void | def 88410; Editor de skin 88730 | DELETE /minecraft/profile/skins/active |
| `auth_set_active_cape` | capeId | void | def 88413; Editor de skin 88743 | PUT /minecraft/profile/capes/active |
| `auth_hide_cape` | - | void | def 88416; Editor de skin 88743 | DELETE /minecraft/profile/capes/active |
| `auth_lookup_skin_by_username` | username | `{username, skinUrl\|null, variant}` | def 88419; Editor de skin 88470,88657 | api.mojang.com/users/profiles/minecraft/{name} + sessionserver.mojang.com/session/minecraft/profile/{uuid} (TexturesPayload) |
| `auth_read_skin_preview` | filePath | string data-URL (vista previa de PNG local) | def 88422; Editor de skin 88689 | Lee PNG local |

### 5. Catálogo de skins (`skins_catalog.rs`)

| Comando | Args (JS) | Resultado (campos leídos / struct) | Línea / pantalla | Pista de backend |
|---|---|---|---|---|
| `skins_catalog_list` | {sort, category?, q?, cursor?, limit=24} (objeto directo) | `{skins:[{id, slug, name, model:"slim"\|"classic", accent, icon, ...}], nextCursor}` (SkinListResult, 3) | def 88098; Skins comunidad 88217,88234 | GET {backendBase}/skins?... (SkinSummaryInfo, 10) |
| `skins_catalog_get_categories` | - | `[{id, slug, name, ...}]` (SkinCategoryInfo, 4) | def 88101; Skins comunidad 88213 | GET /skins/categories |
| `skins_catalog_list_favorites` | - | `SkinSummaryInfo[]` (la UI usa slug) | def 88104; Skins comunidad 88225 | GET /skins/favorites |
| `skins_catalog_set_favorite` | slug, favorite:boolean | boolean: estado resultante (la UI recarga si difiere) | def 88107; Skins comunidad 88268 | PUT/DELETE /skins/favorites/{slug} |
| `skins_catalog_backend_base_url` | - | string URL base del backend (usada para /offline-skins/...) | def 29003; ProfilesProvider 29033; Editor de skin 88598 | `config.rs::backend_base_url`; memoizado |
| `skins_catalog_web_base_url` | - | string URL base web (para `/api/skins/{slug}/assets/{kind}` y `/download`) | def 88111; Skins comunidad 88117 | `config.rs::web_base_url` |

### 6. Roxy Doctor (`doctor.rs`)

| Comando | Args (JS) | Resultado (campos leídos / struct) | Línea / pantalla | Pista de backend |
|---|---|---|---|---|
| `doctor_diagnose` | instanceId | `{healthy:boolean, diagnoses:[Diagnosis], crashExcerpt?, crashReportPath?}`. Diagnosis: id, kind (clave i18n `roxyDoctorDiagnoses:<kind>`), title, explanation, params, severity ("critical"\|"warning"\|"info"), category ("java"\|"memory"\|"jvm_args"\|"mod_conflict"\|"missing_dependency"\|"corrupted_file"\|"graphics_driver"\|"loader"\|"account"\|"network"\|"system"\|"world_corruption"\|"unknown"), culprit, confidence, fix?{label, warning?, action:{type, ...}} | def 39717; Roxy Doctor 39783 | Analiza `latest_launch.log`/crash-reports/hs_err de la instancia |
| `doctor_apply_fix` | instanceId, action (el `fix.action` del diagnóstico) | string con mensaje de resultado (se muestra bajo el botón) | def 39720; Roxy Doctor 39809 | Acciones (`type`): `disableMod`, `setMemoryMb`, `installJava{major}`, `removeJvmArg{needle}`, `clearJvmArgs`, `deleteInstanceFile{relative_path}`, `reinstallLoader`, `clearWorldLocks`. `installJava` emite `doctor://progress` |

### 7. Actualizador (`updater.rs`, tauri-plugin-updater)

| Comando | Args (JS) | Resultado (campos leídos / struct) | Línea / pantalla | Pista de backend |
|---|---|---|---|---|
| `updater_check` | - | `{version, currentVersion, notes}` \| null | def 58929; Modal actualización 58949 | Endpoint `https://<api>/launcher/releases/updater/{{target}}/{{arch}}/{{current_version}}` con firma minisign embebida en el binario |
| `updater_download_and_install` | - | void; progreso por `update://progress`; al terminar reinicia | def 58937; Modal actualización 58973 | plugin updater: descarga, verifica firma, instala |
### 1.3 Forma de `LauncherSettings` (`launcher_get_settings` / `launcher_update_settings`)

Campos leídos/escritos por la UI (index.js:31802-31950, 34836-34990, 59255-59300) y confirmados en el binario (0x423097, struct `LauncherSettings`): `autoUpdate`, `launchOnStartup` (la UI además llama `plugin:autostart|enable/disable`), `closeOnLaunch`, `accentColor` ("auto" o `#rrggbb`), `themeIntensity` (0-100, def. 55), `backgroundBlur` (def. 0), `backgroundDim` (def. 40), `panelOpacity` (def. 100), `globalMemoryMb`, `globalJvmArgs`, `javaPath`, `installPath`, `soundEnabled`, `updateNotifs`, `downloadNotifs`, `discordRpcEnabled`, `quickPlayCustomServers[{id, name?, address, versionId}]` (`QuickPlayCustomServer`, 4 campos), `hiddenQuickPlayServerIds[]`, `lastPlayedVersionId` (`"optifine:<mc>"` para OptiFine), `lastPlayedInstanceId`. Los valores por defecto de idioma, acento y colocación de noticias viven también en el cliente Rust (`client::default_language`, `default_accent_color`, `default_hero_placement`).

## 2. Eventos backend -> frontend

Todos se escuchan con `listen` (`Qi`, index.js:12800) sin filtro de ventana (`target: Any`). El payload llega en `event.payload`. Offsets: cadenas en `roxy-launcher`.

| Evento | Payload (campos leídos) | Receptores (línea) | Emisor inferido |
|---|---|---|---|
| `mc://progress` (0x41f98f) | `ProgressInfo` (3): `{done, total, file}`; sin id de instancia (global) | wrapper `fx` 29184: RunningInstanceProvider 35648; barra superior 40225 (throttle 200 ms, se oculta 2 s tras `done>=total`); modal Java 44276; Home 45560 | `src/minecraft/*` (descarga de versión, libs, assets, Java, loaders, modpacks) |
| `mc://exited` (0x41fa10) | `{code, secondsPlayed}` (94703); instanceId no leído | wrapper `$l` 29192: 35649, 37331, 40234, 43046 (refresca presencia y perfil), 45566, 57806 (relista instancias), 94433 (consola) | monitor del proceso del juego (`minecraft.rs`) |
| `mc://log` (0x310940) | `LogLine`: `instanceId, seq, stream, timestampMs, level, logger, thread, raw, isContinuation` | wrapper `Zte` 40699, uso 94412: consola (filtra por `instanceId`, dedup por `seq`) | lector stdout/stderr del juego; `mc_get_log_backlog` entrega el histórico |
| `mc://crash` (0x41f9f4) | `{instanceId, crashReportPath}` | wrapper `Jte` 40702, uso 94436: consola, botón "ver informe" (95925, opener reveal) | detecta "Crash report saved to:" en el log (cadena en el binario) |
| `mc://doctor-alert` (0x41f9ff) | `{instanceId, ...}` (solo se lee `instanceId`) | wrapper `zte` 39722, uso 40201: banner rojo que abre Roxy Doctor para esa instancia | `doctor.rs`/monitor tras salida anormal |
| `doctor://progress` (0x41f9da) | `{done, total}` | wrapper `Ote` 39725, uso 39800: barra de `installJava` en el modal Doctor  | `doctor.rs::doctor_apply_fix` (installJava) |
| `mc://lan-opened` (0x41fa86) | `{port, instanceId, mcVersion}` | `Kee` 36507: LanProvider 37323 (habilita "Compartir") | `lan_watch.rs` (parsea el puerto LAN del log del juego) |
| `mc://lan-closed` (0x42051b) | sin payload | 36510: LanProvider 37328 | `lan_watch.rs` |
| `mc://lan-share-started` (0x4333d5) | string: id de sesión compartida | 36513: LanProvider 37326 | `lan.rs::lan_share_start` |
| `mc://lan-share-stopped` (0x4202ef) | sin payload | 36516: LanProvider 37327 | `lan.rs::lan_share_stop` / fin de túnel |
| `update://progress` (0x432f25) | unión etiquetada por `kind`; la UI solo trata `{kind:"progress", downloaded, total}` (otros kinds sin leer, probablemente started/finished) | `Hse` 58931: modal de actualización 58963 | `updater.rs::updater_download_and_install` |

Eventos nativos de Tauri que usa la UI: arrastrar y soltar (`tauri://drag-*`, `onDragDropEvent`, index.js:52570-52590: instala `.jar`/`.zip` soltados sobre el dashboard), `onResized` (36342), y el cierre de ventana personalizado (barra de título propia: `hide`, `minimize`, `toggleMaximize`, `startDragging`; cierre según `closeBehavior`, 36307-36330). La ventana de consola es otra `WebviewWindow` con etiqueta `console-<instanceId>` que carga el mismo bundle (96184, 96207).

## 3. Plugins de Tauri usados por el frontend (no son comandos propios)

| Plugin | Llamadas en el bundle | Uso |
|---|---|---|
| `app` | `plugin:app\|name`, `plugin:app\|version` | nombre/versión del launcher |
| `autostart` | `enable`, `disable` | "iniciar con el sistema" |
| `dialog` | `open` | elegir carpeta/archivo (Java, instalación, imports, skins PNG) |
| `event` | `listen`, `unlisten`, `emit`, `emit_to` | eventos |
| `notification` | `is_permission_granted` (+ `request_permission`, `notify` en el binario) | avisos de error de consola / "listo para jugar" |
| `opener` | `open_url`, `reveal_item_in_dir` | abrir enlaces / mostrar archivo |
| `clipboard-manager` | `write_image` | copiar captura |
| `window` / `webview` / `image` / `resources` | decenas (excluidas del catálogo) | ventana sin decoraciones, consola |
| `single-instance`, `updater`, `fs`, `process`, `dialogs` | solo backend (crates `tauri_plugin_*` en el binario; el updater se envuelve en `updater_check`/`updater_download_and_install`) | instancia única (ajuste `allowMultipleInstances`), autoactualización |

## 4. Comandos en el binario no usados por el frontend

Registrados (aparecen en las listas de registro 0x41ec00 y 0x4451a3) o con símbolo `commands::<mod>::<fn>`, pero sin llamada `K()`:

| Comando | Evidencia | Argumentos | Notas |
|---|---|---|---|
| `lan_active_join` | 0x4455b6, símbolo 0x2805325 (`commands::lan`) | ninguno propio (cadena contigua `lan`: nombre del `State`) | lado cliente: probablemente la sesión LAN a la que estoy unido (par de `lan_active_share`) |
| `lan_kick` | 0x310a70, 0x123862f, símbolo 0x2024c28 (`commands::lan::lan_kick`); `lan_client::kick` | (sin nombre de argumento localizado) | expulsar jugador de mi sesión |
| `auth_logout` | 0x4456a1 | - | probablemente borra la sesión Microsoft guardada (sin evidencia del cuerpo) |
| `auth_get_active_profile` | 0x41f723; símbolo `commands::auth::auth_get_active_profile` | - | devuelve `MinecraftProfile` activo |
| `ecosystem_get_stats` | 0x41ef6a; símbolo `commands::ecosystem` | - | GET /me/stats |
| `launcher_add_playtime` | 0x41f151 | `additionalSeconds` | la UI no lo llama: lo hace `session_timer.rs` (`report_delta`, `should_count_now`) con POST /me/playtime |
| `launcher_list_sessions` | 0x41f593; símbolo | - | `SessionSummary[]` (deviceLabel, ipAddress, expiresAt...); GET /me/sessions |
| `launcher_revoke_session` | 0x41f57c | (argumento no localizado) | DELETE /me/sessions/{id} |
| `group_remove_member` | 0x41efd3; símbolo | (argumentos no localizados; probable id, userId) | |
| `group_set_member_role` | 0x41efe6; símbolo | (argumentos no localizados; probable id, userId, role) | |
| `shared_list_invites` | 0x41f3df; símbolo | - | `SharedInstanceInvitesResponse` (bandeja de invitaciones recibidas) |
| `shared_accept_invite` | 0x41f3cb | (no localizados) | |
| `shared_decline_invite` | 0x41f3b6 | (no localizados) | |
| `activity_list_feed` | 0x41ee4e; símbolo 0x2025002 | `limit` | feed de actividad del usuario (GET /me/activity?limit=) |
| `curseforge_install` | 0x41f78b, símbolo 0x2027ecf; args `projectType, modId, projectName, versionNumber` | | instala un archivo concreto (la UI usa `curseforge_get_files` + `modrinth_install` con la URL de descarga) |

Indicios sin confirmar (solo fragmentos en el bloque de despacho 0x30a000-0x311000; pueden ser troceados de otros nombres): `ecosystem_fetch_skin_by_username` (32 B contiguos en 0x30db70, sin símbolo), `group_add_member` (0x30bfb0; existe `add_group_member` en el cliente HTTP pero no en la lista de registro), `launcher_decline_...`. Funciones internas del cliente que NO son comandos pero muestran endpoints sin UI: `list_remote_instances / upsert_remote_instance / delete_remote_instance / replace_instance_files` (se disparan desde `sync_instance_upsert/delete/files` al crear/borrar/modificar instancias locales), `respond_to_friend_request`, `lan_client::{create_session, close_session, heartbeat, kick}`, `list_shared_instance_invites`.

Frontend sin comando en el binario: ninguno. Los 9 nombres que no aparecen enteros (`ecosystem_get_categories`, `launcher_list_news`, `launcher_list_servers`, `launcher_list_videos`, `local_settings_get`, `mc_default_dot_minecraft_path`, `mc_get_system_memory_mb`, `skins_catalog_backend_base_url`, `skins_catalog_web_base_url`) sí aparecen troceados en el despacho (p. ej. `launcher_list_ne`, `launcher_list_vi`, `launcher_list_se`).

## 5. Índice de rutas HTTP del backend propio (cadenas estáticas, sin contactar)

Rutas literales en rodata (0x2e90fd-0x30987d). Base: `config::backend_base_url` y `web_base_url` (valor concreto en el binario; ver research de red, no repetido aquí). Autenticadas con `Authorization` y renovación vía `/auth/refresh` (`launcher_account/http.rs`, `session.rs::refresh_access_token`, `ecosystem::authed_get`).

- Auth: `/auth/register`, `/auth/login`, `/auth/verify`, `/auth/resend`, `/auth/refresh`, `/auth/logout`, `/auth/forgot-password`, `/auth/reset-password`, `/auth/google`, `/auth/google/complete-registration`, `/auth/google/link-existing`.
- Cuenta: `/me`, `/me/settings`, `/me/presence`, `/me/playtime`, `/me/sessions`, `/me/change-password`, `/me/resend-verification`, `/me/tutorial/complete`, `/me/google(/link)`, `/me/discord(/link)`, `/me/minecraft-accounts(/{id})`, `/me/friends`, `/me/friends/requests(/{id})`.
- Instancias/social: `/me/instances(/{id})`, `/me/shared-instances` (+ `/join`, `/preview?inviteCode=`, `/invites(/{code})`, `/membership`), `/me/groups` (+ `/join`, `/{id}/members/{uid}`, `/{id}/activity?limit=`), `/me/lan/sessions(/{id})`, `/me/activity?limit=`.
- Ecosistema: `/me/events`, `/me/wallet`, `/me/equipped`, `/me/inventory`, `/me/inventory/unequip`, `/me/collections`, `/me/achievements`, `/me/stats`, `/me/battlepass`, `/me/battlepass/claim`, `/me/battlepass/purchase-premium`, `/me/launcher-opened`, `/cosmetics/categories`, `/marketplace/listings`, `/me/marketplace/purchases`.
- Contenido público: `/news`, `/videos`, `/servers`, `/featured-content?kind=`, `/stats/public`, `/stats/leaderboard?limit=`, `/stats/referral-leaderboard?limit=`, `/skins`, `/skins/categories`, `/skins/favorites`, `/api/skins/{slug}/assets/{kind}`, `/api/skins/{slug}/download`, `/offline-skins/{hash}[/face|/bust]`.
- Actualización: `/launcher/releases/current`, `/launcher/releases/updater/{{target}}/{{arch}}/{{current_version}}` (plugin updater, firma minisign).
- Terceros: Microsoft (`login.microsoftonline.com/consumers/oauth2/v2.0/{devicecode,token}`), Xbox Live/XSTS, `api.minecraftservices.com` (`/authentication/login_with_xbox`, `/entitlements/mcstore`, `/minecraft/profile`), `api.mojang.com` y `sessionserver.mojang.com` (perfiles), `launchermeta.mojang.com` (manifest), `api.modrinth.com/v2`, `meta.fabricmc.net`, `meta.quiltmc.org`, `files.minecraftforge.net`, `maven.neoforged.net`, `bmclapi2.bangbang93.com/optifine`, `api.adoptium.net`, `laby-releases.s3...` (LabyMod), `authlib-injector.yushi.moe` (authlib-injector 1.2.8), OAuth Google/Discord con captura loopback.

## 6. Flujos clave derivados del catálogo (para el reimplementador)

1. **Arranque**: `launcher_restore_session` -> si hay perfil `authed` y arranca el latido (`launcher_update_presence`, 60 s); en paralelo `ecosystem_*` x7 (`Promise.allSettled`) y `ecosystem_report_launcher_opened` (recompensa diaria); `launcher_check_update` bloquea la UI si `updateRequired` (93638) y `updater_check` ofrece instalar.
2. **Jugar con cuenta premium**: la UI llama `mc_launch_instance`; si falla con `NotAuthenticated` abre el modal de contraseña y llama `launcher_restore_microsoft_account(accountId, password)`; si `JavaVersionRequired{requiredMajor}` abre el modal Java (`mc_install_java`, con `mc://progress`) y reintenta.
3. **Cuenta Microsoft nueva**: `auth_begin_device_login` (muestra `user_code`/URL) -> `auth_complete_device_login` (bloquea hasta autorizar) -> `launcher_link_microsoft_account` (sube al backend) -> `launcher_get_profile`. Cancelar: `auth_cancel_device_login`.
4. **Compartir LAN**: `mc://lan-opened` habilita `lan_share_start`; el invitado usa `lan_list_sessions`/`lan_get_session`; según `resolution.mode` crea instancia (`mc_create_instance`), `lan_adopt`, `shared_sync(FULL)`, `lan_join` (devuelve puerto de túnel) y lanza `mc_connect_*` a `127.0.0.1:<puerto>` (index.js:36735-36790).
5. **Instancia compartida**: `shared_create_instance` -> `shared_create_invite_code`; el receptor `shared_preview_invite` -> `mc_create_instance` -> `shared_join` -> `shared_sync(FULL)`. Sincronización con tres modos: `MERGE_KEEP_MINE`, `UPDATE_SHARED_ONLY`, `FULL` (index.js:50468-50480).
6. **Tienda**: `ecosystem_purchase`/`battlepass_purchase_premium` llevan `idempotencyKey` (UUID del cliente, reutilizado en reintentos y borrado al éxito, 90545, 91770).

## 7. Desconocidos

- Forma exacta (JSON) de respuestas que la UI no lee: `ecosystem_purchase`, `battlepass_purchase_premium`, `shared_create_instance`, `group_create_invite_code`, `shared_create_invite_code`, `mc_toggle_backup_pin`, `mc_create_backup`, retorno de `shared_publish_manifest`, `modrinth_install_modpack`.
- `kind` adicionales de `launcher_login`/`launcher_google_sign_in` (solo se ven `loggedIn`, `needsEmailVerify`, `needsRegistration`, `needsPasswordReset`; el caso "enlazar Google" se distingue por exclusión).
- Campos completos de `Instance` (se identifican 17 de 18), `SharedInstanceSummary` (12), `LanSessionSummary` (11), `EventInfo`, `AchievementInfo`, `CollectionInfo`, `GameServerItem`, `FeaturedContentItem` (20): los nombres salen de la mezcla "UI + pool de nombres de campo del binario" (0x422189, 0x43f36c...), que está deduplicada y sin orden por struct.
- Tipos de `kind` de `update://progress` distintos de `progress`.
- Si `mc://exited` lleva `instanceId` (la UI no lo usa; permite varias instancias con `allowMultipleInstances`, pero `mc://progress` y `mc://exited` se tratan como globales).
- Disparador exacto y payload completo de `mc://doctor-alert` (solo se lee `instanceId`).
- Semántica real de `lan_active_join`, `lan_kick`, `launcher_revoke_session` y de los comandos de invitaciones (`shared_*_invite`): sin UI en esta versión.
- Verbos HTTP y cuerpo de cada ruta (requieren análisis del cliente `launcher_account/client.rs`; ver research de red/API).
- Cómo obtiene CurseForge su clave/proxy (no se ve endpoint propio de CurseForge en rodata).
- Orden real de parámetros Rust: la lista de nombres de argumento del binario está deduplicada; la UI da el nombre completo pero no los opcionales (`Option<T>`) con certeza: se asumen opcionales los que la UI pasa como `?? null`/`undefined`.
