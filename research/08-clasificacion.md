# Clasificación de funciones: local / terceros / servidor Roxy (2026-10-11)

Cruza 03 (UI), 04 (comandos), 05 (red), 06 (lanzamiento) y 07 (datos) con 01-discord.md. Decisión del usuario (2026-10-11): lo social y el ecosistema se descartan o pasan al home-server propio.

Leyenda de la columna **Clon**:
- **Núcleo**: se reimplementa en `core/`.
- **Terceros**: se reimplementa en `core/` contra la API pública de un tercero.
- **Home-server**: opcional, en `server/`.
- **Quitar**: no se replica.
- **Mejorar**: se replica cambiando el diseño.

## 1. Local (clonable sin servidor)

| Función | Comandos / módulo original | Evidencia | Clon |
|---|---|---|---|
| Instancias: crear, editar (RAM, JVM, resolución, icono, devMode), borrar con papelera, restaurar o purgar, cambiar de versión | `mc_create_instance`, `mc_update_instance_settings`, `mc_delete_instance`, `mc_list_trash_instances`, `mc_restore_deleted_instance`, `mc_purge_instance`, `mc_change_instance_version`, `mc_set_instance_icon_*` | 04 §2.2, 07 | Núcleo |
| Importar o adoptar instancias (carpeta, `.minecraft`, otra instancia, versiones custom) | `mc_import_*`, `mc_inspect_import_source*`, `mc_adopt_instance_folder`, `mc_list_unmanaged_instances`, `mc_import_custom_version` | 04 §2.3 | Núcleo |
| Instalación de versiones, librerías, assets y natives con verificación de hash | `src/minecraft/{manifest,download,libraries,assets}.rs` | 06 | Núcleo |
| Loaders: Fabric, Quilt, Forge, NeoForge, OptiFine, LabyMod y custom | `src/minecraft/loader/*` | 02, 06 | Núcleo (OptiFine y LabyMod con prioridad baja) |
| Java gestionado (Adoptium) | `mc_install_java`, `java_install.rs`, error `JavaVersionRequired` | 04 §6, 06 | Núcleo + API de Adoptium |
| Lanzamiento, consola de logs, detección de crash, salida | `mc_launch_instance`, `mc_open_console_window`, `mc_get_log_backlog`, eventos `mc://log`, `mc://crash`, `mc://exited`, `mc://progress` | 04 §2.1, 06 | Núcleo |
| Conexión directa a servidor (quickPlay) | `mc_connect_server`, `mc_connect_instance_to_server` | 04, 06 | Núcleo |
| Contador de tiempo de juego | `playtimeSeconds` en `instance.json` | 07 | Núcleo, solo local (el original además lo reportaba a `/me/playtime`) |
| Doctor: diagnóstico de crash y arreglos | `doctor_diagnose`, `doctor_apply_fix`, `src/minecraft/doctor/*` | 03 §3.10, 06 | Núcleo |
| Backups automáticos y manuales, pin, restaurar | `mc_*backup*`, `backup_scheduler.rs` | 04 §2.2, 06 | Núcleo |
| Mundos (listar, renombrar, editar level.dat, borrar), capturas, datapacks | `mc_*world*`, `mc_*screenshot*`, `mc_*datapack*`, `fastnbt` | 04 §2.5 | Núcleo |
| Activar o desactivar mods (`.disabled`), registro de procedencia | `modrinth_toggle_file`, `.modrinth_sources.json` | 07 | Núcleo |
| Discord Rich Presence | `mc_rpc_set_enabled`, `mc_rpc_set_idle`, `discord_rpc.rs` | 06 | Núcleo, con application id propio de Discord |
| Ajustes locales (cerrar o minimizar a la bandeja, multi-instancia, bajo consumo) | `local_settings_*`, `quit_launcher`, localStorage `low-power-mode` | 03 §6, 07 | Núcleo + UI |
| Ajustes de usuario (idioma, tema, notificaciones, cerrar al lanzar…) | `launcher_get_settings`/`launcher_update_settings`: en el original se **sincronizaban con `/me/settings`** | 04 §1.3 | **Mejorar**: guardarlos en local |
| Tema adaptativo al wallpaper, tutorial, i18n es/en | Frontend | 03 §4–5 | UI Flutter |
| Detección de «Abrir a LAN» (puerto local) | `lan_local_port`, `lan_local_address`, `lan_watch.rs`, `netstat2` | 04 §3.3 | Núcleo (base para la LAN por internet) |
| Cuentas offline | `launcher_add_offline_account` | 03 §3.10 | Núcleo |
| Skins de cuentas offline con authlib-injector | `authlib.rs`, `authlib-injector.yushi.moe` 1.2.8; las skins las servía Roxy (`/offline-skins/{hash}`) | 04 §5, 06 | Home-server o quitar (prioridad baja) |
| Autostart, bandeja, instancia única, notificaciones del sistema | plugins de Tauri | 04 §3 | UI/plataforma |

## 2. API pública de terceros

| Función | Servicio | Notas para el clon |
|---|---|---|
| Login de Microsoft (device code) → Xbox Live → XSTS → Minecraft Services, entitlements y perfil | `login.microsoftonline.com/consumers`, `user.auth.xboxlive.com`, `xsts.auth.xboxlive.com`, `api.minecraftservices.com` | **Requiere app propia en Azure** (el client_id del original es de Roxy) y aprobación de Mojang para usar la API de Minecraft. Refresh token en el keyring local. |
| Skins y capas (subir, reset, capa activa u oculta, historial) | `api.minecraftservices.com/minecraft/profile/skins`, `/capes/active` | `auth_*skin*`, `auth_*cape*` |
| Buscar la skin de otro jugador | `api.mojang.com/users/profiles/minecraft`, `sessionserver.mojang.com` | `auth_lookup_skin_by_username` |
| Manifiestos de versiones | `piston-meta`/`launchermeta.mojang.com`, `resources.download.minecraft.net`, `libraries.minecraft.net` | Solo `launchermeta.mojang.com`, sin espejo (BMCLAPI solo para OptiFine). 12 descargas concurrentes, SHA-1, sin reintentos ni caché de JSON (06) → el clon añade reintentos, caché y modo offline |
| Loaders | `meta.fabricmc.net`, `meta.quiltmc.org`, `files.minecraftforge.net`/`maven.minecraftforge.net`, `maven.neoforged.net`, BMCLAPI (OptiFine), `laby-releases.s3.de.io.cloud.ovh.net` | |
| Java | `api.adoptium.net` | |
| Modrinth: búsqueda, versiones, instalación, modpacks `.mrpack` | `api.modrinth.com/v2` | Sin clave |
| CurseForge: búsqueda, archivos, modpacks | API de CurseForge, **pasando por el backend de Roxy** (`/curseforge/{categories,search,mods/{id}/files,files/bulk}`; no hay clave en el binario, ver 05) | **Requiere clave de API propia** (consola de CurseForge para desarrolladores). Se puede llamar directo desde `core/` con la clave en la config local, o con un proxy en el home-server. |
| Estado de servidores en la Home | `api.mcsrvstat.us/3/{host}` (llamada directa desde el WebView) | Pasarlo al núcleo con caché TTL |
| Avatares y renders de skin | `minotar.net`, `visage.surgeplay.com` | Mejor renderizar en local desde la skin (sin depender de terceros) |
| Banners de versión, vídeos | `minecraft.wiki`, YouTube oEmbed e imágenes | Opcional |

## 3. Servidor Roxy (`api.roxylauncher.com`): quitar o pasar al home-server

| Función | Comandos / rutas | Propuesta |
|---|---|---|
| **Cuenta Roxy** (registro, login, código por email, Google, reset, sesiones de dispositivo) | `launcher_login/register/verify_code/...`, `/auth/*`, `/me/sessions` | **Quitar**. Bloqueaba todo el launcher. El clon arranca sin cuenta propia. |
| **Respaldo de cuentas de Microsoft en el servidor** («restaurar entre dispositivos» con contraseña) | `launcher_link_microsoft_account`, `launcher_restore_microsoft_account(accountId, password)`, `/me/minecraft-accounts` | **Quitar**. Los tokens de MS solo en el keyring local. Riesgo de seguridad del original. |
| **Sincronización de instancias** (subida automática al crear, borrar o cambiar archivos) | `sync_instance_upsert/delete/files` → `/me/instances` (04 §4) | Home-server opcional, como sincronización explícita, nunca automática por defecto |
| Ajustes de usuario en la nube, presencia (cada 60 s), playtime, launcher-opened, actividad | `/me/settings`, `/me/presence`, `/me/playtime`, `/me/launcher-opened`, `/me/activity` | Quitar (telemetría). La presencia solo si hay amigos en el home-server. |
| Amigos | `launcher_*friend*`, `/me/friends` | Home-server (fase tardía) |
| Grupos | `group_*`, `/me/groups` | Home-server (fase tardía) |
| Instancias compartidas (manifiesto, invitaciones, 3 modos de sincronización) | `shared_*`, `/me/shared-instances` | Home-server (fase tardía) |
| **LAN por internet** (sesiones, señalización, relay o túnel) | `lan_*`, `/me/lan/sessions`, `relayNodeUrl`, `hostToken`/`joinToken`, `lan_tunnel.rs`, `lan_proxy.rs` | Home-server, con un relay propio o alternativas (Tailscale/WireGuard, e4mc). Es lo más valioso de esta lista. |
| Ecosistema: wallet, tienda, inventario, equipables, logros, colecciones, eventos, recompensa diaria | `ecosystem_*`, `/me/wallet`, `/marketplace/*`, `/cosmetics/*` | Quitar |
| Pase de batalla | `battlepass_*`, `/me/battlepass*` | Quitar |
| Leaderboard y referidos | `launcher_get_leaderboard`, `/stats/*` | Quitar |
| Noticias, vídeos, contenido destacado, servidores de partners | `launcher_list_news/videos/servers/featured_content`, `/news`, `/servers` | **Mejorar**: lista de servidores local y editable (petición de la comunidad). Noticias de Minecraft desde una fuente pública opcional. |
| Catálogo de skins (navegar, favoritos, aplicar) | `skins_catalog_*`, `/skins`, `/api/skins/*` | Quitar o sustituir por un catálogo público. Mantener «aplicar skin por archivo/URL» (es de Minecraft Services). |
| Vinculación con Discord y Google (OAuth) | `launcher_link_discord/google`, `/me/discord`, `/me/google` | Quitar |
| Contador de usuarios online | `launcher_get_online_count` | Quitar |
| Actualizador automático (minisign) | `updater_*`, `/launcher/releases/updater/...` | **Reemplazar**: releases propias (home-server o GitHub privado) con la clave minisign propia, o Flatpak/AppImage update. |
| Tutorial completado | `launcher_complete_tutorial`, `/me/tutorial/complete` | Local |

## 4. Cruce con 01-discord.md

- Lo confirma el binario:
  - «Restaurar cuentas de Microsoft entre dispositivos» guarda los tokens en el servidor y pide la contraseña de Roxy para recuperarlos.
  - LAN por internet con relay propio.
  - Datapacks (0.6.0).
  - Protección Zip Slip (ver 06).
  - Cierre o minimización y multi-instancia en `local_settings.json`.
- Del Discord que se resuelven en el clon:
  - «Fabric no abría» y «la 26.2 no cargaba por flags de JVM» son bugs de args (06).
  - «La resolución no se guardaba»: el campo existe en `instance.json`.
  - «Ajustes no cargaban» se debía a que los ajustes dependían del servidor (`/me/settings`).
- Sugerencias de la comunidad que encajan de forma natural:
  - importar instancias (ya existe `mc_import_*`);
  - args JVM según la RAM;
  - nombre de la instancia en el RPC;
  - servidores de la Home editables;
  - ocultar el email (deja de aplicar, porque no hay cuenta Roxy);
  - skin real en la Home.

## 5. Desconocidos que afectan a la clasificación
- Relay LAN: REST `/me/lan/sessions*` + WebSocket `{relayNodeUrl}/host|/join`, control JSON `{"t":…}` y tramas binarias con cabecera u32 BE; formato binario exacto inferido, no verificado (05 §5).
- CurseForge: resuelto, va por proxy del backend de Roxy (05).
- Cuentas offline: `-javaagent:authlib-injector=<backend>/yggdrasil`; un servidor Yggdrasil propio solo hace falta para skins offline (05).
- El original admite `LAUNCHER_BACKEND_URL`/`LAUNCHER_WEB_URL` (05): útil para probar un home-server con el original, no para el clon.
- BMCLAPI: solo OptiFine (06).
- Ajustes globales de juego (memoria, jvmArgs, `javaPath`) vivían en `/me/settings` del backend Roxy (06) → pasan a local.
- `resolutionWidth/Height` se guardaban pero no se aplicaban al lanzar (06): bug del original a corregir.
- Un solo juego a la vez en el original pese a `allowMultipleInstances` (06).
