# roxy-alt

Launcher de Minecraft propio para Linux, inspirado en el flujo de Roxy Launcher 0.6.0 (Tauri 2, semi-abandonado). Solo para uso personal.

## Stack
Núcleo Rust (`core/`) + UI Flutter (`app/`) vía flutter_rust_bridge; servicios propios opcionales en `server/` (home-server). Motivo y alternativas: `docs/decisiones.md` (D1). No cambiar el stack sin una nueva decisión registrada ahí.

## Reglas
- Original de referencia: `reference/roxy-launcher-0.6.0.AppImage` (enlace simbólico; no copiar ni modificar). Extracciones en `extracted/`. Ambas carpetas están ignoradas por git.
- Nunca leer, imprimir ni copiar tokens de `~/.local/share/.roxy/session.json` ni `launcher_account.json`; solo la estructura de claves.
- No contactar servidores de Roxy durante el análisis: nada de login, telemetría ni peticiones a sus APIs. Solo análisis estático o capturas offline.
- No reutilizar el código, los assets, el nombre ni el logo del original. Reimplementar el comportamiento con código propio.
- El repo es privado: sin remoto público.
- Hallazgos en `research/`, un archivo por área, con su evidencia (archivo/offset/función) y lo que sigue desconocido.
- Datos de juego existentes (~4 GB): `~/.local/share/.roxy/game`. Solo lectura hasta decidir la migración.
