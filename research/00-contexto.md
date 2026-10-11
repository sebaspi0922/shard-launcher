# Contexto inicial (2026-10-11)

- AppImage: `~/Applications/roxy-launcher-0.6.0.AppImage` (98 MB, ELF estático). Tauri 2 con WebKitGTK; frontend web embebido en el binario y backend en Rust (tokio, zip/lzma, socks, tauri-plugin-updater).
- Datos: `~/.local/share/.roxy/` contiene `game/` (accounts, assets, installers, java, libraries, natives, versions; 4 GB), `launcher_account.json` (cuenta Roxy: id/email/username), `session.json` (perfil MC + `mc_access_token`) y `local_settings.json`.
- Datos WebView: `~/.local/share/com.roxylauncher.app/` (localStorage, cookies).
- roxylauncher.com devuelve 522 de Cloudflare (origen caído) en todas las rutas; coincide con el login roto. Sin capturas en Wayback ni repos en GitHub.
- Equipo: RTX 4070 + Wayland. El lag de WebKitGTK con NVIDIA/Wayland es un problema conocido (tauri-apps/tauri#8541).
