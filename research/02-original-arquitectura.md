# Arquitectura del original y método de extracción (2026-10-11)

## Artefacto

| | |
|---|---|
| AppImage | `~/Applications/roxy-launcher-0.6.0.AppImage`, sha256 `8277065943edb12efc2f509c5ac01f2c4c1f10d80bb62eae21fbd6bc5758d243` |
| Binario | `usr/bin/roxy-launcher`: ELF x86-64 PIE dinámico, **no stripped** (`.symtab` con ~98 000 símbolos Rust v0), 45 MB, sha256 `768fe065bc08a9eb5ffcbf2e1482222d13d6cdcff531ac7621238309bbcd5ef2` |
| Identificador | `com.roxylauncher.app` (0x412d09); en desarrollo, `devUrl` `http://localhost:1420` |
| Compilado por | usuario `sasuke`: rutas `/home/sasuke/.cargo/registry/...` en las cadenas de pánico |
| .desktop | «A modern, multi-loader Minecraft launcher with Fabric, Forge, and NeoForge support» |
| Ventanas | «Roxy Launcher», «Roxy Launcher Alpha» y «Roxy Launcher · Console», una ventana aparte para la consola (0x313814) |

## Extracción (reproducible, sin ejecutar nada)

1. Squashfs del AppImage: el offset es el final de la tabla de secciones ELF (`e_shoff + e_shentsize*e_shnum` = 944632). Se extrae con `unsquashfs -o 944632 -d extracted/appimage …`. El primer `hsqs` del archivo es un falso positivo dentro del runtime.
2. Frontend embebido de Tauri: `tools/extract_tauri_assets.py`. Recorre `.data.rel.ro` buscando pares `(&str ruta, &[u8] datos)` a partir de las relocaciones `R_X86_64_RELATIVE` y descomprime los datos con Brotli. El resultado está en `extracted/frontend/` y son 15 archivos:
   - `index.html`;
   - `assets/index-BPEhFSEn.js` (1,9 MB) e `index-Dhr_1BgR.css`;
   - 8 imágenes webp;
   - las fuentes `mojangles` y `monocraft-bold`;
   - `tauri.svg` y `vite.svg`.
3. Formateo para leer el código: `npx prettier@3.3.3` → `extracted/frontend-pretty/index.js` (96 212 líneas) e `index.css`. En los demás documentos, `index.js:N` se refiere a este archivo.
4. Cadenas del backend: `strings -n 8` → `extracted/binary-strings.txt`, y `tools/ctx.py` para ver el contexto y el offset. Las cadenas de Rust están concatenadas sin NUL.

### Limitaciones de REA
- `analyze_javascript_application` sobre `extracted/frontend` agotó el heap en la fase semántica, primero con 1 GB y después con 8 GB (Evidence `ev_91a3a521…`, `ev_3a96ba28…`). El bundle Vite es un único archivo de 1,9 MB con unos 430 000 nodos AST. Se pasó a análisis estático dirigido (grep/AST) sobre el bundle formateado.
- El backend se analizó con cadenas (rutas de pánico, campos serde, URLs) y con símbolos (`nm`, nombres `tauri_app_lib::commands::*`), más desensamblado puntual con capstone. Ghidra vía REA dio timeout con 45 MB (ver 05 y 06). El ELF externo del AppImage (el runtime) sí está stripped; el binario de la app no.

## Frontend

- React con react-router, i18next, Vite y la API JS de Tauri 2. Título del index: «Tauri + React + Typescript», la plantilla sin cambiar.
- Llamadas al backend: `K(cmd, args)` → `__TAURI_INTERNALS__.invoke` (index.js:12639). Eventos: `Qi(evento, cb)` (index.js:12800).
- Rutas: `/` (home), `servers`, `instances`, `mods`, `leaderboard`, `skins`, `profile`, `battlepass`, `settings` (index.js:~93800). El menú lateral está en index.js:36230.
- Detalle: 03-ui-pantallas.md y 04-comandos-tauri.md.

## Backend (módulos recuperados de las rutas de pánico)

```
src/lib.rs, src/discord_rpc.rs
src/auth/            http, microsoft, minecraft_services, session, skins, xbox
src/commands/        auth, curseforge, doctor, ecosystem, lan, launcher_account, minecraft,
                     modrinth, shared_instances, skins_catalog, updater, world
src/launcher_account/ client, discord_oauth, ecosystem, google_oauth, http, lan_client,
                     session, session_timer, skins_catalog
src/minecraft/       account, args, assets, authlib, backup, backup_scheduler, curseforge,
                     download, http, java_install, lan_proxy, lan_tunnel, lan_watch, launch,
                     libraries, log_console, manifest, modrinth, world
src/minecraft/loader/ custom, fabric, forge, labymod, mod, optifine
src/minecraft/doctor/ fixes, mod, signatures
```

No existe `loader/quilt.rs` ni `loader/neoforge.rs`. Lo más probable es que Forge y NeoForge compartan `forge.rs` (hay referencias a `maven.neoforged.net`). Quilt: ver 06-lanzamiento.md.

Crates relevantes:
- Tauri y plugins:
  - `tauri 2.11.5`, `wry 0.55.1`, `tao 0.35.3`;
  - plugins `autostart`, `clipboard-manager`, `dialog`, `notification`, `opener`, `single-instance` y `updater`.
- Red: `reqwest` 0.12 y 0.13, `rustls`, `tokio-tungstenite` 0.24 (WebSocket).
- Datos y archivos:
  - `zip 2.4.2`, `tar`, `xz2`, `lzma-rs`, `bzip2`, `zstd`;
  - `fastnbt` (level.dat), `image`.
- Sistema e integración:
  - `keyring 2.3.3` (Secret Service);
  - `discord-rich-presence 0.2.5`;
  - `netstat2` (detecta el puerto LAN del juego);
  - `notify-rust`, `arboard`, `tray-icon`, `auto-launch`.

En la fase 2 sirven como referencia de qué resolver, no como dependencias obligatorias.

## Grandes bloques funcionales

1. **Cuenta Roxy** (`launcher_account/*`): login con email o Google, verificación, perfil, amigos, presencia, ecosistema (wallet, tienda, inventario, logros, colecciones, eventos), pase de batalla, grupos, instancias compartidas, LAN por internet, catálogo de skins, noticias, servidores destacados, leaderboard, referidos y vinculación con Discord. Todo contra `api.roxylauncher.com`.
2. **Cuentas de Minecraft** (`auth/*`): login de Microsoft por device code, Xbox Live, XSTS y Minecraft Services; skins y capas; cuentas offline.
3. **Juego** (`minecraft/*`): manifiestos, descargas, Java, loaders, lanzamiento, consola, crash, «doctor», backups, mundos, Modrinth, CurseForge, importación y datapacks.
4. **Sistema**: updater, Rich Presence, bandeja, autostart, instancia única y modo de bajo consumo.
