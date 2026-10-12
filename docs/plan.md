# Plan por hitos (2026-10-11)

Orden pedido: login MS → instancias y lanzamiento → mods → UI completa → servicios en el home-server. Cada hito termina con pruebas en verde, una demo por CLI (o UI en el H4) y un commit con mensaje aprobado.

La fase 3 empieza por H0 y H1.

## Estado (2026-10-11)

- **H0**: hecho.
- **H1**: hecho en el núcleo y probado contra un servidor simulado y el keyring real. Falta la prueba real con Microsoft (client ID de Azure y aprobación de Mojang).
- **H2**: hecho, salvo Rich Presence (necesita el application id de Discord).
  - Versiones, descargas, assets, Java gestionado (Adoptium), instancias, lanzamiento, logs, Doctor e importación desde Roxy.
  - Loaders: Fabric, Quilt, Forge (incluido el formato antiguo, sin Java) y NeoForge (incluido el esquema 26.x: `26.2` → `26.2.0.*`).
  - Probado en real con cuenta offline:
    - las dos instancias importadas de Roxy (26.2 + Fabric, 157 mods);
    - vanilla 1.8.9 (Java 8 instalada sola, natives de LWJGL 2);
    - NeoForge 1.21.1.
  - Registro `.last_launch.json` por instancia:
    - Doctor solo mira lo de la última partida;
    - el tiempo de las partidas que terminaron con el launcher cerrado se suma después.
- **H3**: hecho, salvo CurseForge real (necesita clave de API).
  - Modrinth: búsqueda, instalación con dependencias, activar/desactivar, actualizaciones por hash e identificación.
  - Modpacks `.mrpack` y zip de CurseForge.
  - Backups automáticos con retención y restauración (también de los zips de Roxy).
  - Mundos (`level.dat`), datapacks y capturas.
  - Servidores rápidos con ping propio (SLP + SRV).
  - Importación de `.minecraft` y Prism/MultiMC.
  - Probado con las instancias reales: mundos de 26.2, backups importados y actualizaciones detectadas.
- **Revisión del núcleo** (Opus, 2026-10-11): 11 defectos encontrados; corregidos 9.
  - Partida en marcha detectada también fuera del proceso.
  - Backups que acompañan a la instancia a la papelera; ids que no heredan backups ajenos.
  - Borrar una instancia exige el juego cerrado.
  - `.part` único por descarga.
  - Token fuera de la serialización.
  - Registro de partida no fatal; candado en `instance.json`; sin doble conteo del tiempo.
  - Reserva atómica de ids.
  - Secretos borrados antes que la cuenta.
- **Limitaciones conocidas:**
  - Los backups omiten los enlaces simbólicos a carpetas (p. ej. `saves` enlazado fuera) y solo lo avisan en el log.
  - Con `--detach` la salida del juego va al log sin enmascarar el token. Minecraft no lo imprime, pero un mod podría.

## H0 – Esqueleto (corto)

- Workspace Cargo con `core/` (lib + bin `lcore`) y `.gitignore` de Rust.
- `paths`, `settings`, `net` (cliente, reintentos, caché), `events`, `Error` y la CLI vacía con `--data-dir`, `--json` y `--offline`.
- Fixtures descargados de fuentes públicas (manifiesto de Mojang, JSON de 1.8.9, 1.20.1 y 26.2, perfiles de Fabric y Quilt).
- **Hecho cuando**: `cargo test`, `cargo clippy -D warnings` y `cargo fmt --check` pasan; `lcore --help` funciona.

## H1 – Login de Microsoft y cuentas

- Device code contra `login.microsoftonline.com/consumers` con scope `XboxLive.signin offline_access`, sondeo con `slow_down` y cancelación.
- XBL → XSTS (con traducción de los XErr a mensajes claros) → `login_with_xbox` → entitlements → perfil.
- Refresco con rotación del refresh token; caché del token de MC con su caducidad.
- Almacén: `accounts.json` (público) + keyring (secretos). Varias cuentas y cuenta activa.
- Cuentas offline (UUID v3).
- Skins y capas: ver, subir archivo, reset, capa activa u oculta.
- CLI: `account login|list|use|remove|add-offline|refresh`, `skin …`.
- Pruebas: cadena completa con wiremock (incluidos 403 de app no aprobada y XErr), keyring con backend de prueba.
- **Depende del usuario**: registro de la app de Azure y solicitud de acceso a la API de Minecraft (`docs/guia-azure.md`). Hasta que Mojang apruebe, se prueba en real hasta XSTS y el resto con mocks; el lanzamiento se prueba con cuentas offline.
- **Hecho cuando**: con una app aprobada, `lcore account login` deja una cuenta usable que sobrevive a un reinicio (refresco desde el keyring).

## H2 – Instancias, instalación y lanzamiento

- `meta` (manifiesto, versiones, reglas multiplataforma, fusión), `download` (cola, hashes, progreso), assets y natives.
- `java`: detección e instalación desde Adoptium con checksum.
- `instance`: CRUD, papelera de 30 días, cambio de versión, icono, playtime.
- Loaders: Vanilla, Fabric y Quilt primero; luego Forge y NeoForge con instalador oficial (D6).
- `launch`: argumentos (memoria, JVM, resolución, quickPlay/`--server`), proceso desacoplado, logs, crash, salida y playtime. Varias partidas a la vez.
- `import roxy` (`docs/migracion.md`) y Doctor (firmas y arreglos de `research/06` §7).
- Rich Presence con application id propio (opcional, si hay id de Discord).
- **Hecho cuando**: las dos instancias migradas (Fabulously Optimized y técnico, MC 26.2 + Fabric 0.19.3) arrancan desde la CLI, con cuenta de MS si ya hay aprobación y offline si no; una instancia vanilla antigua (1.8.9) y una NeoForge también arrancan.

## H3 – Contenido y mantenimiento

- Modrinth: búsqueda con filtros, versiones compatibles, instalación con dependencias, activar/desactivar, borrar, actualizaciones por hash.
- CurseForge con clave propia del usuario (opcional; sin clave se oculta).
- Modpacks `.mrpack` y zip de CurseForge, importar desde archivo y URL.
- Resourcepacks, shaders y datapacks (por mundo).
- Backups (manuales, automáticos, fijados, restaurar), mundos (`level.dat`), capturas.
- Importar `.minecraft` oficial y Prism/MultiMC.
- Servidores rápidos con Server List Ping.
- **Hecho cuando**: se instala un modpack de Modrinth desde la CLI y arranca; los backups automáticos respetan la retención.

## H4 – App Flutter

- Instalar Flutter con fvm (`pacman -S fvm` desde `extra`; `fvm install stable`, versión fijada en `app/.fvmrc`).
- `flutter_rust_bridge` 2.x estable: crate puente `app/rust`, stream de eventos.
- Pantallas de `docs/arquitectura.md` §5 con diseño propio: arranque, cuentas, Home, Instancias y detalle, Contenido, Skins, Ajustes, Doctor, consola.
- Tema oscuro adaptativo, i18n es/en, tutorial breve, bandeja, autostart, instancia única, cierre o minimizado.
- Empaquetado: AppImage y Flatpak; `PKGBUILD` local. Windows y macOS se compilan pero se prueban después.
- **Hecho cuando**: el flujo completo (añadir cuenta, crear o importar instancia, instalar mods, jugar, ver consola, diagnosticar) funciona desde la app en el equipo del usuario (RTX 4070, Wayland) sin lag.

## H5 – Servicios en el home-server (opcional)

Por orden de valor:
1. **LAN por internet**: relay propio (WebSocket o QUIC) o integración con Tailscale/WireGuard; detección de «Abrir a LAN» ya en el núcleo.
2. Sincronización explícita de instancias entre equipos.
3. Amigos y presencia.
4. Instancias compartidas.
5. Yggdrasil propio para skins de cuentas offline (authlib-injector).
6. Releases del updater firmadas.

Se diseña en detalle al llegar; antes se registra la decisión en `docs/decisiones.md`.

## Tareas del usuario (en paralelo)

| Cuándo | Qué | Guía |
|---|---|---|
| Ya (bloquea el fin de H1) | Registrar la app en Azure y pedir acceso a la API de Minecraft; pasar el client ID | `docs/guia-azure.md` §1–2 |
| Antes de H2 (opcional) | Crear una aplicación de Discord para Rich Presence; pasar el application id | `docs/guia-azure.md` §3 |
| Antes de H3 (opcional) | Pedir clave de API de CurseForge | `docs/guia-azure.md` §4 |
| Antes de H4 | Elegir el nombre visible del launcher (D2) | — |
