# Red, endpoints y autenticación de Roxy Launcher 0.6.0

Área: todo lo que sale a la red (backend Rust y frontend). Análisis estático, sin red, sin ejecutar el binario.

## 0. Hallazgos principales y método

- **El binario NO está stripped.** Conserva `.symtab` (98 375 símbolos, 13 378 de `tauri_app_lib`) y los `serde` derivados. Por eso hay nombres de función, structs y rutas exactos. Los `#N` de `run::{closure#2}::{closure#N}` son los comandos IPC.
- **La URL del backend se puede cambiar sin parchear**: `config::backend_base_url` (0x968850) lee la variable de entorno `LAUNCHER_BACKEND_URL` (cadena en 0x40db76) y, si no existe, usa `https://api.roxylauncher.com` (0x40cd1f). Igual para la web con `LAUNCHER_WEB_URL` (0x30c7e0, por defecto `https://roxylauncher.com`, 0x40cd07). El launcher original funcionaría contra un home-server propio con solo exportar esas variables. Los client_id de Google/Discord y la clave minisign del updater siguen fijos.
- **CurseForge no se llama directamente.** No hay `api.curseforge.com` ni clave de API en el binario (0 coincidencias de `x-api-key`, `api_key`, `client_secret`, `GOCSPX`). El backend de Roxy hace de proxy: `GET {backend}/curseforge/{categories,search,mods/{id}/files}` y `POST /curseforge/files/bulk`. Hay que reimplementar ese proxy con una clave propia de CurseForge.
- **Las skins de cuentas offline pasan por un servidor Yggdrasil de Roxy**: el juego se lanza con `-javaagent:<authlib-injector.jar>=<backend>/yggdrasil` (plantilla en 0x304344/0x304354). Hay que replicar `/yggdrasil` y `/offline-skins/...` en el home-server si se quiere esa función.
- No hay secretos en el binario. Solo identificadores públicos: client_id de Microsoft, Google y Discord y la clave pública minisign.
- Método: `nm -C` + desensamblado con capstone (scripts propios, solo en el directorio temporal de la sesión) para extraer plantillas de formato, método HTTP (se decodifica el `http::Method` pasado a `reqwest::Client::request`; validado: GET/POST/PUT/DELETE de Mojang coinciden con la API real), claves JSON (inmediatos ASCII y `movups` de rodata) y campos serde. REA/Ghidra agotó el tiempo de espera (binario de 45 MB) y se descartó.
- Notación: `[B]` = lleva `Authorization: Bearer <accessToken Roxy>`; `[-]` = sin autenticación. Las rutas de Roxy son relativas a `{backend}`. "Confirmado" = visto en código/símbolos; "inferido" = deducido de nombres o estructura.
- Aviso de proceso: el directorio temporal de la sesión es compartido con otros agentes y se pisaron algunos archivos (`nm.txt`, `callers.py`). Mis herramientas están en `scratchpad/net05/`. No afecta a este documento.

## 1. Dominios y hosts contactados

Offsets = desplazamiento de archivo en `roxy-launcher` (en `.rodata` coincide con la dirección virtual).

### 1.1 Roxy (propio)

| Host / URL | Para qué | Módulo | Offset |
|---|---|---|---|
| `https://api.roxylauncher.com` | API (cuenta, ecosistema, LAN, instancias, noticias, CurseForge, Yggdrasil, releases) | `launcher_account::{client,ecosystem,lan_client,skins_catalog,session}`, `minecraft::curseforge`, `commands::*` | 0x40cd1f, 0x412d58 |
| `https://roxylauncher.com` | Web base (`LAUNCHER_WEB_URL`): `/dashboard` (referidos), `/download`, `/privacy`, `/terms`, `/api/skins/{slug}/assets/{kind}` | `config::web_base_url`, frontend (`index.js:38446`, `92450`) | 0x40cd07, 0x440c29 |
| `https://discord.roxylauncher.com` | Enlace de invitación al Discord (solo UI) | frontend (`index.js:92440`) | 0x312660 |
| `https://api.roxylauncher.com/launcher/releases/updater/{{target}}/{{arch}}/{{current_version}}` | Endpoint del updater de Tauri | `tauri-plugin-updater` | 0x412d58 |
| `fusiora.com` | Enlace a un partner (solo `openUrl`) | frontend (`index.js:44530`) | n/a |

### 1.2 Microsoft / Xbox / Minecraft Services / Mojang

| Host / URL | Para qué | Módulo | Offset |
|---|---|---|---|
| `login.microsoftonline.com/consumers/oauth2/v2.0/devicecode` | Inicio de device code | `auth::microsoft::request_device_code` (IPC #216) | 0x42016e |
| `login.microsoftonline.com/consumers/oauth2/v2.0/token` | Sondeo del device code y refresco | `auth::microsoft::{poll_for_token,refresh_token}` (IPC #215), `minecraft::account::resolve_account` | 0x4200fc, 0x43333a |
| `https://user.auth.xboxlive.com/user/authenticate` | Token XBL | `auth::session::complete_chain` | 0x420065 |
| `https://xsts.auth.xboxlive.com/xsts/authorize` | Token XSTS | `auth::session::complete_chain` | 0x42000a |
| `https://api.minecraftservices.com/authentication/login_with_xbox` | Token Minecraft | idem | 0x41ffab |
| `https://api.minecraftservices.com/entitlements/mcstore` | Comprobar licencia Java | idem | 0x41ff75 |
| `https://api.minecraftservices.com/minecraft/profile` | Perfil; base de `/skins`, `/skins/active`, `/capes/active` | `auth::{session,skins}` | 0x41ff42 |
| `https://api.mojang.com/users/profiles/minecraft/{name}` | Nombre a UUID (buscador de skins) | `auth::skins::lookup_skin_by_username` (IPC #205) | 0x2fdde2 |
| `https://sessionserver.mojang.com/session/minecraft/profile/{uuid}` | Texturas (skin/capa) de un UUID | idem | 0x308903 |
| `https://launchermeta.mojang.com/mc/game/version_manifest_v2.json` | Manifiesto de versiones | `minecraft::manifest::fetch_manifest` | 0x42044a |
| `https://libraries.minecraft.net` | Librerías (base por defecto) | `minecraft::libraries` | 0x411ee6 |
| `https://resources.download.minecraft.net` | Assets (`/{hash2}/{hash}`) | `minecraft::launch::run` | 0x4334cd |
| (URLs del manifiesto: client jar, asset index, piston-data) | Descargas de versión | `minecraft::download` | n/a (vienen de la respuesta) |

### 1.3 Loaders, Java y utilidades de instalación

| Host / URL | Para qué | Módulo | Offset |
|---|---|---|---|
| `https://meta.fabricmc.net/v2/versions/loader` (+ `/{mc}/{loader}/profile/json`) | Versiones y perfil de Fabric | `minecraft::loader::fabric` | 0x4203f3 |
| `https://meta.quiltmc.org/v3/versions/loader` (+ `/profile/json`) | Quilt (mismo código que Fabric) | `loader::fabric` genérico | 0x42041f |
| `https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json` | recommended/latest de Forge | `loader::forge` | 0x42063b |
| `https://maven.minecraftforge.net/net/minecraftforge/forge` (+ `/maven-metadata.xml`, `-installer.jar`) | Instalador y metadatos de Forge | `loader::forge::install` | 0x43398c |
| `https://maven.neoforged.net/releases/net/neoforged/neoforge` | Idem para NeoForge | `loader::forge` (rama neoforge) | 0x4206e5 |
| `https://bmclapi2.bangbang93.com/optifine` | Espejo chino de OptiFine (lista y `raw.jar`) | `loader::optifine` | 0x420692 |
| `https://laby-releases.s3.de.io.cloud.ovh.net/api/v1/{manifest/…/latest.json, download/labymod4/…, libraries/…}` | LabyMod 4 (manifiesto, jar, librerías) | `loader::labymod` | 0x2f56ff, 0x2f6635, 0x2f6686 |
| `https://api.adoptium.net/v3/assets/latest/{major}/hotspot?image_type=jre&os=…&architecture=…&vendor=eclipse` | JRE gestionado | `minecraft::java_install` | 0x2f250c |
| `https://authlib-injector.yushi.moe/artifact/56/authlib-injector-1.2.8.jar` (SHA1 `0e0e66d8…11f0b8`) | authlib-injector para skins de cuentas offline | `minecraft::authlib::ensure_authlib_injector` | 0x43366d |

### 1.4 Mods y contenido

| Host / URL | Para qué | Módulo | Offset |
|---|---|---|---|
| `https://api.modrinth.com/v2` (`/search`, `/project/{id}/version`, `/version/{id}`, `/projects?ids=`) | Búsqueda e instalación; UA `github.com/sasuke/minecraft-launcher/1.0 (modrinth-integration)` | `commands::modrinth`, IPC #40/#41/#48/#49 | 0x420493, 0x4204b8 |
| `{backend}/curseforge/*` | CurseForge vía proxy de Roxy (sin clave en el cliente) | `commands::curseforge`, IPC #7/#10/#11 | 0x303420, 0x2f972b |
| CDN de Modrinth/CurseForge | Descarga de archivos con la `downloadUrl` devuelta | `minecraft::download::download_all` | n/a |

### 1.5 Solo frontend (UI)

| Host | Para qué | Dónde |
|---|---|---|
| `api.mcsrvstat.us/3/{host}` | Estado de servidores de la Home (caché 60 s, `index.js:44254`) | frontend |
| `minotar.net/{helm,bust,avatar}/{nombre\|uuid}/{px}.png` | Avatares | frontend (`:28996`, `:37367`, `:49691`, `:63972`, `:88502`) |
| `visage.surgeplay.com/{face,bust}/{px}/{uuid}` | Avatares/bustos 3D | frontend (`:28999`) y backend (`auth::session::complete_chain`: `skin_url = …/face/128/{uuid}`, 0x2fedc2) |
| `www.youtube.com/oembed`, `img.youtube.com/vi/{id}/…` | Títulos y miniaturas de `/videos` | frontend (`:44688`, `:44750`) |
| `minecraft.wiki/images/*`, `images.unsplash.com/…`, `modrinth.com/{mod,modpack,…}/{slug}`, `curseforge.com/minecraft/{tipo}/{slug}` | Imágenes de fondo y enlaces a proyectos | frontend (`:43323`, `:46318`, `:60716`) |

### 1.6 OAuth externo

| Host | Para qué | Offset |
|---|---|---|
| `accounts.google.com/o/oauth2/v2/auth` | Inicio de sesión / enlace con Google | 0x2e90e0 |
| `discord.com/oauth2/authorize` | Vincular Discord | 0x2f33e6 |
| IPC local de Discord (`discord-rich-presence`) | Rich Presence (sin red) | `discord_rpc::*` |

## 2. Login de Microsoft (flujo exacto)

**Tipo: device code** (no authorization code). Confirmado: URL `…/devicecode` y `grant_type=urn:ietf:params:oauth:grant-type:device_code`. Ningún puerto local ni redirect en este flujo.

Constantes (0x4200b5…): client_id público `aa9d84f1-83da-43a1-9d52-f6057db28094`; scope `XboxLive.signin offline_access`; tenant `consumers`.

1. `auth_begin_device_login` (IPC #216): `POST …/devicecode` (form `client_id`, `scope`). Respuesta `DeviceCodeResponse{device_code,user_code,verification_uri,expires_in,interval}`. Guarda en `AuthState` (pending login) y devuelve al frontend `{user_code,verification_uri,expires_in}`; la UI abre `verification_uri` y muestra el código.
2. `auth_complete_device_login` (IPC #215): sondea `POST …/token` (form de 3 campos: `grant_type=urn:ietf:params:oauth:grant-type:device_code`, `client_id`, `device_code`) cada `interval` s. Respuesta OK `TokenSuccessResponse{access_token,refresh_token,…}`. Errores `TokenErrorResponse`: trata `authorization_pending`, `slow_down` (aumenta el intervalo), `expired_token` y rechazo (`authorization_declined`, inferido). `auth_cancel_device_login` aborta.
3. `auth::session::complete_chain`:
   - XBL: `POST https://user.auth.xboxlive.com/user/authenticate` JSON `{"Properties":{"AuthMethod":"RPS","SiteName":"user.auth.xboxlive.com","RpsTicket":"d=<access_token>"},"RelyingParty":"http://auth.xboxlive.com","TokenType":"JWT"}`. Respuesta `{Token, DisplayClaims.xui[0].uhs}`.
   - XSTS: `POST https://xsts.auth.xboxlive.com/xsts/authorize` JSON `{"Properties":{"SandboxId":"RETAIL","UserTokens":[<XBL token>]},"RelyingParty":"rp://api.minecraftservices.com/","TokenType":"JWT"}`. Errores `XstsErrorResponse` (XErr) con mensajes propios: "no tiene perfil de Xbox Live" y "las cuentas infantiles necesitan un adulto en el grupo familiar".
   - Minecraft: `POST https://api.minecraftservices.com/authentication/login_with_xbox` JSON `{"identityToken":"XBL3.0 x=<uhs>;<xsts_token>"}`. Respuesta `{access_token, expires_in}`.
   - Licencia: `GET …/entitlements/mcstore` (Bearer MC). Respuesta `{items:[…]}`. Si está vacío: "esta cuenta no posee Minecraft: Java Edition".
   - Perfil: `GET …/minecraft/profile` (Bearer MC). Respuesta `{id,name,…}`.
4. Resultado `MinecraftProfile{uuid,username,skin_url}`; `skin_url` NO viene de Mojang, es `https://visage.surgeplay.com/face/128/{uuid}`.

**Almacenamiento** (`auth::store`):
- Refresh token de Microsoft: keyring (Secret Service, crate `keyring` 2.3.3), servicio `com.sasukemc.minecraftlauncher.auth`, usuario `ms_refresh_token` (0x417829, 0x30d830).
- `~/.local/share/.roxy/session.json` (dir `.roxy`, 0x41784c): `{"profile":{"uuid","username","skin_url"},"mc_access_token","mc_expires_at"}` (`CachedSession`). Es el único sitio donde queda el token de Minecraft.
- Si el keyring falla: error "No se pudo guardar tu sesión de forma segura" (no hay respaldo en claro del refresh token).

**Refresco** (`auth::session::get_valid_session`): si el `mc_access_token` cacheado caduca en más de 60 s (constante `+0x3c` sobre "ahora") lo reutiliza; si no, `auth::microsoft::refresh_token` hace `POST …/token` con form de 4 campos (`client_id`, `grant_type=refresh_token`, `refresh_token`, `scope=XboxLive.signin offline_access`) y repite `complete_chain` entera. Si Microsoft rota el refresh token, se reescribe en el keyring.

**Restaurar cuentas entre dispositivos** (IPC #187 `launcher_restore_microsoft_account`, #188 `launcher_link_microsoft_account`): los refresh tokens de Microsoft se guardan **en el servidor de Roxy**:
- `POST /me/minecraft-accounts` `[B]` `{accountType:"PREMIUM", mcUsername, mcUuid, microsoftRefreshToken}` (offline: `{accountType:"OFFLINE", mcUsername}`).
- `PATCH /me/minecraft-accounts/{id}/refresh-token` `[B]` `{microsoftRefreshToken}` tras cada rotación.
- `POST /me/minecraft-accounts/{id}/refresh-token` `[B]` `{password}` devuelve `{microsoftRefreshToken}`; el cliente lo canjea contra Microsoft y reinicia la cadena. Si falla, mensaje "el backend aún tiene el refresh token viejo (ya consumido)".

**Skins y capas** (`auth::skins`, todo contra `https://api.minecraftservices.com/minecraft/profile`, Bearer MC; IPC #205–#211):

| Acción | Petición |
|---|---|
| Perfil, skins y capas propias | `GET /minecraft/profile` → `{id,name,skins[{id,state,url,variant}],capes[{id,state,url,alias}]}` |
| Aplicar skin por URL | `POST …/skins` JSON `{url, variant}` (`variant` = `CLASSIC`/`SLIM`) |
| Subir skin | `POST …/skins` multipart `file` (`skin.png`, `image/png`) + `variant` |
| Restablecer skin | `DELETE …/skins/active` |
| Capa activa / ocultar | `PUT …/capes/active` `{capeId}` / `DELETE …/capes/active` |
| Buscar por nombre | `GET api.mojang.com/users/profiles/minecraft/{name}` → `{id,name}`, luego `GET sessionserver…/profile/{id}` → `properties[textures]` (base64: `textures.SKIN{url,metadata.model}`, `CAPE`) |

Un 429 de Mojang se traduce a "Mojang está limitando los cambios de skin".

## 3. API de Roxy

### 3.1 Cliente, sesión y errores

- Cliente `reqwest` con `rustls`, UA `RoxyLauncher/0.15.0 ({os})` (0x2f5352; la versión 0.15.0 no coincide con el "0.6.0" público, ver sección 5). Respeta `HTTPS_PROXY`/`NO_PROXY`. Sin cookies.
- Autenticación: `Authorization: Bearer <accessToken>` guardado en memoria (`LauncherAccountState`). Ante 401, `launcher_account::session::refresh_access_token` hace `POST /auth/refresh` y reintenta (patrón visible en `session_timer`, `sync_instance_files`, `lan_share_start`, `link_discord`).
- Almacenamiento (`launcher_account::store`): keyring servicio `com.sasukemc.minecraftlauncher.launcher-account`, usuarios `refresh_token` y `device_token`; `~/.local/share/.roxy/launcher_account.json` con `{id,email,username}` (`CachedUser`).
- Errores: cuerpo `ErrorResponse{error:string}`; variantes `LauncherAccountError`: `Network`, `ValidationFailed`, `Conflict`, `InvalidCredentials`, `NotFound`, `InvalidResponse`, `RateLimited`, `NotAuthenticated`, `Keyring`, más `EmailNotVerified`. El frontend recibe `{kind,message}`. Mapa exacto estado a variante: inferido (401 a NotAuthenticated/InvalidCredentials, 409 a Conflict, 429 a RateLimited).

### 3.2 Autenticación (`/auth/*`)

| Método y ruta | Auth | Petición | Respuesta | Función (IPC) |
|---|---|---|---|---|
| `POST /auth/register` | - | `{email,username,password}` | `RegisterResponse` (no se pudo extraer; incluye `pendingTicket` o sesión, inferido) | `launcher_register` (#203) |
| `POST /auth/login` | - | `{email,password,deviceLabel,deviceToken?}` | `AuthResponse{user{id,email,username},accessToken,refreshToken,deviceToken?}`; si el correo no está verificado, 4xx `EmailNotVerifiedBody{error,pendingTicket}` | `launcher_login` (#202) |
| `POST /auth/verify` | - | `{pendingTicket,code}` | `AuthResponse` o `{resetTicket}` (flujo de olvido de contraseña) | `launcher_verify_code` (#201) |
| `POST /auth/resend` | - | `{pendingTicket}` | `PendingTicketBody{pendingTicket}` | `launcher_resend_code` (#200) |
| `POST /auth/forgot-password` | - | `{email}` | `{pendingTicket}` | `launcher_forgot_password` (#199) |
| `POST /auth/reset-password` | - | `{resetTicket,newPassword}` | vacío | `launcher_reset_password` (#198) |
| `POST /auth/refresh` | - | `{refreshToken}` | `RefreshResponse{accessToken,refreshToken}` (rotación) | `session::refresh_access_token` |
| `POST /auth/logout` | - | `{refreshToken}` | vacío | `launcher_logout` (#191) |
| `POST /auth/google` | - | `{code,redirectUri,clientType:"desktop",deviceLabel}` | `AuthResponse`, o `{needsRegistration:true,pendingTicket,email,suggestedUsername}`, o `{needsLinkConfirmation:true,pendingTicket,email}` | `launcher_google_sign_in` (#169) |
| `POST /auth/google/complete-registration` | - | `{pendingTicket,username,password}` | `AuthResponse` | #168 |
| `POST /auth/google/link-existing` | - | `{pendingTicket,password}` | `AuthResponse` | #167 |

El `deviceToken` (dispositivo de confianza) se lee y guarda en el keyring del cliente; para qué lo usa el servidor (saltar verificación) es inferido.

### 3.3 Cuenta y perfil (`/me/*`, todos `[B]`)

| Método y ruta | Petición | Respuesta |
|---|---|---|
| `GET /me` | | `MeResponse{id,email,username,role,rank,totalPlaytimeSeconds,level,hasCompletedTutorial,emailVerifiedAt,minecraftAccounts[],discord{id,username}?,google{id,email}?}` |
| `PATCH /me` | `{email?,username?}` | `LauncherUser{id,email,username}` |
| `POST /me/change-password` | `{currentPassword,newPassword}` | vacío |
| `POST /me/resend-verification` | | `{pendingTicket}` |
| `POST /me/tutorial/complete` | | vacío |
| `GET /me/sessions`, `DELETE /me/sessions/{id}` | | `{sessions:[{id,deviceLabel,userAgent,ipAddress,createdAt,expiresAt}]}` |
| `GET /me/settings` | | `LauncherSettings` (ver 3.9) |
| `PATCH /me/settings` | objeto parcial | `LauncherSettings` |
| `POST /me/google/link` | `{code,redirectUri,clientType:"desktop"}` | `GoogleInfo{id,email}` |
| `DELETE /me/google` | | vacío |
| `POST /me/discord/link` | `{code,redirectUri}` | `DiscordInfo{id,username}` |
| `DELETE /me/discord` | | vacío |
| `POST /me/minecraft-accounts` | ver sección 2 | `MinecraftAccountSummary{id,mcUsername,mcUuid,accountType,offlineSkinHash,offlineSkinModel}` |
| `DELETE /me/minecraft-accounts/{id}` | | vacío |
| `PUT /me/minecraft-accounts/{id}/skin` | JSON `{url,mode}` o multipart `file`(`image/png`)+`model` | `MinecraftAccountSummary` (skin de cuenta offline) |
| `DELETE /me/minecraft-accounts/{id}/skin` | | vacío |
| `PATCH` / `POST /me/minecraft-accounts/{id}/refresh-token` | ver sección 2 | `{microsoftRefreshToken}` (POST) |

Valores observados: `accountType` = `PREMIUM`|`OFFLINE`; `model` = `classic`|`slim` (la UI usa `LT()`).

### 3.4 Instancias sincronizadas y compartidas

Sincronización personal (para "restaurar instancias"; se dispara al crear/editar/instalar):

| Método y ruta | Petición | Respuesta |
|---|---|---|
| `GET /me/instances` | | `{instances:[RemoteInstance{id,name,mcVersion,loaderType,loaderVersion,files[]}]}` |
| `POST /me/instances` | `RemoteInstance` (upsert) | |
| `PUT /me/instances/{id}/files` | `{files:[{category,filename,enabled,downloadUrl,sha1}]}` | |
| `DELETE /me/instances/{id}` | | |

Instancias compartidas (todas las rutas en `commands::shared_instances` y `launcher_account::client`):

| Método y ruta | Petición | Respuesta |
|---|---|---|
| `GET /me/shared-instances` | | `{sharedInstances:[SharedInstanceSummary{id,name,mcVersion,loaderType,loaderVersion,manifestVersion,memberCount,myRole,mySyncMode,myInstanceId,isUpToDate,lastSyncedAt}]}` |
| `POST /me/shared-instances` | `{instanceId,name}` | `SharedInstanceDetail{id,name,mcVersion,loaderType,loaderVersion,manifestVersion,inviteCode,files[],members[],myMembership}` |
| `GET` / `DELETE /me/shared-instances/{id}` | | detalle / vacío |
| `PUT /me/shared-instances/{id}/manifest` | `{files:[…]}` | |
| `GET /me/shared-instances/{id}/diff` | | `{diff{added,removed,updated},manifestVersion,isUpToDate}` |
| `POST /me/shared-instances/{id}/sync` | `{mode}` | `{manifestVersion,files}` |
| `POST /me/shared-instances/{id}/invite-code` | `{maxUses,expiresInHours}` | `InviteCodeInfo{inviteCode,inviteCodeExpiresAt}` |
| `DELETE /me/shared-instances/{id}/invite-code` | | |
| `GET /me/shared-instances/preview?inviteCode=` | | `{id,name,mcVersion,loaderType,loaderVersion,…}` |
| `POST /me/shared-instances/join` | `{inviteCode,instanceId}` | `JoinSharedInstanceResponse` (campos no extraídos) |
| `POST /me/shared-instances/{id}/invites` | `{inviteeUserId}` | |
| `GET /me/shared-instances/invites` | | `{invites:[{id,sharedInstance{id,name,mcVersion,loaderType},inviter{id,username},createdAt}]}` |
| `POST /me/shared-instances/invites/{id}/accept` | `{instanceId}` | `JoinSharedInstanceResponse` |
| `POST /me/shared-instances/invites/{id}/decline` | | |
| `POST` / `DELETE /me/shared-instances/{id}/owners[/{userId}]` | `{userId}` | |
| `PATCH` / `DELETE /me/shared-instances/{id}/membership` | `{syncMode}` / | |

Solo se guarda el manifiesto (`files[]` con `downloadUrl` de Modrinth/CurseForge y `sha1`); los archivos no pasan por Roxy. `undownloadableCount` cuenta los que no tienen URL.

### 3.5 Social

| Método y ruta | Petición | Respuesta |
|---|---|---|
| `GET /me/friends` | | `{friends:[FriendUser{id,username,isOnline,currentMcVersion,currentInstanceName,currentSharedInstanceId}]}` |
| `DELETE /me/friends/{id}` | | |
| `GET` / `POST /me/friends/requests` | `{username}` | `{requests:[{id,sender{id,username}}]}` |
| `POST /me/friends/requests/{id}/accept` y `/decline` | | |
| `GET /me/groups` / `POST /me/groups` | `{name}` | `{groups:[GroupSummary{id,name,myRole,memberCount,createdAt}]}` |
| `GET` / `DELETE /me/groups/{id}` | | `GroupDetail{id,name,myRole,members[{userId,username,role,joinedAt}],sharedInstances[{id,name,mcVersion,loaderType}],inviteCode,…}` |
| `POST /me/groups/join` | `{inviteCode}` | `{groupId}` |
| `POST` / `DELETE /me/groups/{id}/invite-code` | `{expiresInHours}` | `InviteCodeInfo` |
| `POST /me/groups/{id}/members` | `{userId}` | |
| `PATCH` / `DELETE /me/groups/{id}/members/{userId}[/role]` | `{role}` | |
| `POST /me/groups/{id}/shared-instances` | `{sharedInstanceId}` | |
| `DELETE /me/groups/{id}/shared-instances/{sid}` | | |
| `GET /me/activity?limit=` / `GET /me/groups/{id}/activity?limit=` | | `{events:[ActivityEvent{id,type,data,user,createdAt}]}` |
| `PATCH /me/presence` | `{isOnline,currentMcVersion?,currentInstanceName?,currentSharedInstanceId?}` | |

### 3.6 Ecosistema (perfil, cosméticos, tienda, pase de batalla)

`GET` autenticados (`ecosystem::authed_get`): `/me/wallet` (`{wallets:[{currency,name,icon,balance,…}]}`), `/me/inventory` (`{items:[InventoryItemInfo{id,acquiredAt,acquisitionSource,equippedSlots,cosmeticItem}]}`), `/me/equipped` (`{equipped:[{categoryKey,slot,cosmeticItem}]}`), `/me/collections`, `/me/achievements`, `/me/events`, `/me/stats`, `/me/battlepass` (`BattlePassStateInfo{battlePass{…},levels[{level,xpRequired,cumulativeXp,unlocked,freeReward,premiumReward,freeClaimed,premiumClaimed}],level,xpIntoLevel,xpForNextLevel,isMaxLevel,hasPremium}`).

Escrituras `[B]`:

| Método y ruta | Petición | Respuesta |
|---|---|---|
| `POST /me/inventory/{inventoryItemId}/equip` | `{slot}` | |
| `POST /me/inventory/unequip` | `{categoryId,slot}` (clave `category…`, sufijo inferido) | |
| `POST /me/marketplace/purchases` | `{listingId,idempotencyKey}` | `PurchaseResult{purchaseId,alreadyProcessed,…}` |
| `POST /me/battlepass/claim` | `{level,track}` | `BattlePassClaimResult{level,track,rewardGrantId,granted,alreadyGranted}` |
| `POST /me/battlepass/purchase-premium` | `{idempotencyKey}` | `BattlePassPurchaseResult` |
| `POST /me/launcher-opened` | vacío | `{dailyRewardGranted,loginDays,countedNewDay,…}` |

Públicas `[-]`: `GET /cosmetics/categories` (`{categories:[{id,key,name,icon,maxEquipped,sortOrder}]}`), `GET /marketplace/listings?category=&search=` (`{listings:[{id,price,currencyType,seller{id,username},cosmeticItem}]}`). Los activos (imágenes de cosméticos) los baja el comando `ecosystem_fetch_image_data_url` (solo `http(s)`, `image/*`, límite de tamaño) y los devuelve como data URL.

### 3.7 Contenido público de la Home y estadísticas `[-]`

| Ruta | Respuesta |
|---|---|
| `GET /news` | `{news:[{id,title,description,category,imageUrl,link,author,order,createdAt,placement}]}` |
| `GET /videos` | `{videos:[{id,url,order}]}` (URL de YouTube) |
| `GET /servers` | `{servers:[{id,name,ip,description,bannerUrl,iconUrl,isFeatured,order,isQuickPlayDefault,quickPlayOrder,status{online,playersOnline,playersMax,version,motd,checkedAt}}]}` |
| `GET /featured-content?kind=` (`MOD`/`RESOURCEPACK`/`SHADER`/`MODPACK`/`DATAPACK`) | `{items:[{id,kind,source,title,description,iconUrl,author,tags,link,linkButtonLabel,linkButtonColor,fileName,fileSizeBytes,sha1,mcVersion,loaderType,downloadCount,downloads,downloadUrl,order}]}` |
| `GET /stats/public` | `{onlineNow}` |
| `GET /stats/leaderboard?limit=` | `{entries:[{position,username,rank,totalPlaytimeSeconds,level}]}` |
| `GET /stats/referral-leaderboard?limit=` | `{entries:[{position,username,rank,confirmedReferrals}]}` |
| `GET /launcher/releases/current` (con Bearer) | `LauncherReleaseInfo{id,version,notes,channel,downloadUrl,isUrgent,createdAt,artifacts[{platform,fileName,fileSizeBytes,hasSignature,downloadUrl}]}` |

`launcher_check_update` compara con `version::compare_versions` y devuelve a la UI `{currentVersion,latestRelease,updateAvailable,updateRequired}` (las dos últimas deducidas del texto del binario).

### 3.8 Catálogo de skins (`skins_catalog`)

La base del catálogo es el backend (`skins_catalog_backend_base_url`) o la web (`skins_catalog_web_base_url`):

| Ruta | Auth | Notas |
|---|---|---|
| `GET /api/skins?sort=&category=&q=&limit=&cursor=` (`sort` = `trending`/`popular`/`newest`) | - | `SkinListResult{skins[{id,slug,name,uuid,model,category,featured,stats{views,downloads,favorites},creator{username},tags[{slug,name}]}],nextCursor,hasMore}` |
| `GET /api/skins/categories` | - | `{categories:[{slug,name,accent,skinCount}]}` |
| `GET /api/skins/favorites` | B | `{skins:[…]}` |
| `POST` / `DELETE /api/skins/{slug}/favorite` | B | `{favorite:bool}` |
| `{web}/api/skins/{slug}/assets/{kind}` | - | Imagen (la construye el frontend, `index.js:88114`) |
| `{backend}/offline-skins/{accountId}[/{face|bust}]?v=3` | - | Textura de la skin de una cuenta offline (frontend `dy()`, `:29006`) |

### 3.9 Ajustes sincronizados (`LauncherSettings`, `GET`/`PATCH /me/settings`)

Campos: `lastPlayedInstanceId, lastPlayedVersionId, accentColor, themeIntensity, panelOpacity, backgroundDim, backgroundBlur, language, soundEnabled, discordRpcEnabled, hiddenQuickPlayServerIds, quickPlayCustomServers[{address,name,versionId,instanceId}], globalJvmArgs, globalMemoryMb, javaPath, installPath, autoUpdate, updateNotifs, downloadNotifs, launchOnStartup, closeOnLaunch, keepLauncherOpen` (los de ruta/memoria/Java son locales de la máquina pero viajan en el mismo objeto).

### 3.10 LAN por internet (relay propio)

REST `[B]` (`launcher_account::lan_client`, `commands::lan`):

| Método y ruta | Petición | Respuesta |
|---|---|---|
| `POST /me/lan/sessions` | `LanCreateSessionRequest{instanceId,mcVersion,loaderType,loaderVersion,worldName,maxPlayers,visibility:"FRIENDS"\|"GROUP",groupId?,onlineMode}` | `{sessionId,relayNodeUrl,relayKey,hostToken}` |
| `GET /me/lan/sessions` | | `{sessions:[LanSessionSummary{id,host{id,username},worldName,mcVersion,loaderType,isModded,sharedInstanceId,sharedInstanceName,playerCount,maxPlayers,onlineMode}]}` |
| `GET /me/lan/sessions/{id}` | | `LanSessionDetail{id,host,worldName,mcVersion,loaderType,playerCount,maxPlayers,onlineMode,accountWarning:"ONLINE_MODE"?,resolution{mode:"VANILLA"\|"NEEDS_JOIN"\|"ALREADY_MEMBER",sharedInstanceId,myInstanceId,isUpToDate,manifestVersion,lastSyncedManifestVersion,name,loaderType,loaderVersion,fileCount,modCount,undownloadableCount}}` |
| `PATCH /me/lan/sessions/{id}` | `{playerCount}` (latido del host) | |
| `DELETE /me/lan/sessions/{id}` | cierra la sesión | |
| `POST /me/lan/sessions/{id}/join` | | `{joinToken,relayNodeUrl}` |
| `POST /me/lan/sessions/{id}/leave` | | |
| `POST /me/lan/sessions/{id}/kick` | `{userId}` | |
| `POST /me/lan/sessions/{id}/adopt` | `{instanceId}` | vincula la instancia local a la compartida |

WebSocket (tokio-tungstenite, `connect_async`):
- **Host** (`commands::lan::lan_share_start`, IPC #115, `minecraft::lan_tunnel`): `GET {relayNodeUrl}/host` con cabecera `Authorization: …<hostToken>`. Antes `minecraft::lan_watch` detecta el puerto LAN del juego (puertos TCP en escucha del proceso del juego, evento `mc://lan-opened`). El host abre una conexión TCP local por cada "stream" que pida un amigo.
- **Invitado** (`lan_join`, IPC #111, `minecraft::lan_proxy`): `GET {relayNodeUrl}/join` con `Authorization: …<joinToken>`; levanta un proxy TCP local en `127.0.0.1:<puerto efímero>` y la UI da esa dirección para entrar desde el juego (`lan_local_address`).
- Mensajes de texto JSON `{"t":"…"}`: `ping` (latido, ambos lados), `open`/`close` con `streamId` (y `friendId`/`friendIdx` en el host), `kick`, `friend-leave`, `error`, `bye`. Los datos TCP van en tramas binarias con cabecera de enteros big-endian (`bswap`): en el host dos u32 (amigo, stream) y en el invitado uno (stream) más el payload. Formato exacto de cabecera: inferido.
- `relayKey` se devuelve al crear la sesión pero no se pudo ver dónde se usa (posible clave de cifrado/ID de sala).

### 3.11 Yggdrasil (skins offline) y updater

- **Yggdrasil**: `minecraft::authlib::jvm_args` construye `-javaagent:{jar}={backend}/yggdrasil` y `-Dauthlibinjector.side=client` (0x416231). authlib-injector usará `GET {backend}/yggdrasil` (metadatos), `…/sessionserver/session/minecraft/{join,hasJoined,profile/{uuid}}` y las texturas `…/offline-skins/…`. Rutas internas del servidor: no están en el cliente (las define authlib-injector). Si el jar no se puede bajar: "authlib-injector no disponible… se lanza sin skin custom".
- **Updater** (tauri-plugin-updater 2.10.1): endpoint `GET {backend}/launcher/releases/updater/{target}/{arch}/{current_version}` (0x412d58) con clave pública minisign embebida, ID `32DCD85349F8CD0C` (base64 en 0x412db6; no se reproduce aquí). Formato de respuesta de Tauri 2: JSON `{version,notes,pub_date,url,signature}` o 204. Sin esa clave no se pueden firmar actualizaciones válidas para este binario; un clon propio generaría la suya. Comandos `updater_check`, `updater_download_and_install` (progreso en `update://progress`). En instalaciones .deb/.rpm la auto-actualización se desactiva.

### 3.12 OAuth locales (redirect URIs y puertos)

| Proveedor | Puerto local | Redirect | client_id (público) | Scope y detalles |
|---|---|---|---|---|
| Google | `127.0.0.1:32146` (0x41fad5) | `http://127.0.0.1:32146/callback` | `192914309028-5n6uhii2cffd6pna3gkabil3mvu79qoq.apps.googleusercontent.com` (0x41fae…) | `openid email profile`, `response_type=code`, `access_type=online`, `prompt=select_account`, `state` aleatorio, sin PKCE. El `code` va al backend (`POST /auth/google` o `/me/google/link`), que lo canjea con su client secret (no está en el cliente). `google_oauth::capture_authorization_code` abre el navegador con `tauri-plugin-opener` y sirve una página "Signed in with Google". |
| Discord | `127.0.0.1:32145` (0x41fbed) | `http://127.0.0.1:32145/callback` | `1527679900063109140` (0x41fbda; el mismo número es el Application ID de Rich Presence, 0x449167) | `identify`, `response_type=code`, `state` aleatorio (validado), tiempo máximo de espera con mensaje "Timed out waiting for Discord's authorization". El `code` va a `POST /me/discord/link`. |

Para un home-server propio hacen falta apps OAuth propias (o un flujo alterno), porque el `redirect_uri` y el secreto del canje viven en las apps de Roxy.

### 3.13 Estructuras serde con "struct X with N elements" (selección)

`MeResponse` 12, `DiscordInfo` 2, `GoogleInfo` 2, `VideoItem` 3, `ServerStatus` 6, `GameServerItem` 11, `FeaturedContentItem` 20, `LeaderboardEntry` 5, `ReferralLeaderboardEntry` 4, `LauncherReleaseInfo` 8, `ReleaseArtifactInfo` 5, `QuickPlayCustomServer` 4, `LauncherUser` 3, `SessionSummary` 6, `RemoteInstance` 6, `RemoteInstanceFile` 5, `FriendUser` 6, `PlaytimeInfo` 2, `NewsItem` 10, `BattlePassClaimResult` 5, `RarityInfo` 5, `CosmeticItemInfo` 7. Todas con `rename_all = "camelCase"`. Campos de cada una en las tablas anteriores; los de `ListingInfo`, `StatsResponse`, `CollectionInfo`, `EventInfo`, `AchievementInfo` (con `rarity`, `progress{current,threshold,comparator}`, `isSecret`, `unlocked`) y `RewardPreviewInfo` salen de los `Serialize` hacia el frontend.

## 4. Telemetría (qué reporta y cuándo)

No hay SDK de analítica (ni Sentry, ni Google Analytics, ni Plausible). Lo que Roxy recibe:

| Dato | Cuándo | Endpoint |
|---|---|---|
| Presencia (`isOnline`, versión MC, nombre de la instancia, instancia compartida) | Al iniciar sesión, cada 60 s mientras hay sesión (`setInterval` de `6e4`, `index.js:28858`), al lanzar un juego, al salir del juego (`mc://exited`) y `isOnline:false` al cerrar sesión | `PATCH /me/presence` |
| Tiempo jugado (`additionalSeconds`) | `session_timer::spawn` (tarea tokio con espera de 60 s, constante `0x3c`, inferido) suma mientras `minecraft::launch::is_game_running` es cierto; el texto de la UI aclara que el tiempo con el launcher en segundo plano no cuenta | `POST /me/playtime` → `{totalPlaytimeSeconds,level}` |
| Apertura del launcher (recompensa diaria) | Al montar el contexto del ecosistema con sesión (`index.js:36622`) | `POST /me/launcher-opened` |
| Lista de instancias y mods instalados (nombre, versión, loader, archivos con URL y sha1) | Al crear/modificar/instalar/borrar (`sync_instance_upsert`, `sync_instance_files`, `ensure_instance_synced`) | `POST /me/instances`, `PUT /me/instances/{id}/files`, `DELETE /me/instances/{id}` |
| Jugadores en la LAN | Latido del host | `PATCH /me/lan/sessions/{id}` |
| Cuentas Minecraft (nombre, UUID, tipo) y refresh tokens de Microsoft | Al vincular/refrescar | `/me/minecraft-accounts*` |
| Etiqueta de dispositivo, UA e IP | En cada login (`deviceLabel`); el servidor guarda UA/IP (`SessionSummary`) | `/auth/login`, `/auth/google` |
| Ajustes y tema | Al cambiarlos | `PATCH /me/settings` |

Fugas a terceros: IP de cada servidor guardado a mcsrvstat, nombres/UUID a minotar y visage, URLs de vídeos a YouTube, consultas a Mojang/Modrinth/Fabric/Quilt/Forge/BMCLAPI/Adoptium. Discord Rich Presence es solo IPC local (`discord_rpc::set_{playing,idle}_activity_if_enabled`; actividad "Minecraft", detalle `instancia · versión`).

## 5. Desconocidos y limitaciones

- Versión: el UA incluye `0.15.0` y la UI/Discord hablan de 0.6.0. No se pudo determinar cuál es `{{current_version}}` del updater (probablemente `CARGO_PKG_VERSION`, 0.15.0). Hay que mirar `tauri.conf.json` embebido (assets del frontend) para confirmarlo.
- Cuerpos de respuesta no extraídos del todo: `RegisterResponse`, `JoinSharedInstanceResponse`, `SharedInstancePreview` (campos extra), `StatsResponse`, `CollectionInfo.items`. Los structs `Deserialize` con nombres cortos se reconstruyen por heurística (comparaciones inline); los de la sección 3 están verificados contra los `Serialize` hacia el frontend, pero pueden faltar campos opcionales.
- Valores de enumeraciones del servidor (`loaderType` en el API, `role` de grupo/instancia, `syncMode`, `sort`, `type` de `ActivityEvent`, `comparator`) no aparecen como literales fiables. El frontend usa `MANUAL` como modo de sincronización (visible en rodata).
- Mapa exacto código HTTP → `LauncherAccountError` y condiciones de reintento no confirmados.
- LAN: formato binario del túnel, mensajes completos de `open/close` y uso de `relayKey` por confirmar leyendo `lan_tunnel::connect`/`lan_proxy::start` con más detalle. El nombre exacto de la cabecera `Authorization` del WebSocket (¿`Bearer`?) no se verificó.
- Yggdrasil: el cliente solo pasa la base; las rutas y la firma de texturas las define authlib-injector (hay que seguir su especificación).
- CurseForge: nombres de los parámetros de consulta (`gameVersion`, `loader`, `classId`, `q`, `kind`) inferidos de los argumentos IPC y de tablas de strings.
- `lan_proxy`, `modrinth::*` y la instalación de loaders se tocaron solo en lo de red; el flujo de lanzamiento queda para otras áreas.
- Limitaciones de método: análisis estático con capstone sobre símbolos; Ghidra/REA no completó. Offsets de rodata son de archivo. No se ejecutó nada ni se contactó ningún servidor.
