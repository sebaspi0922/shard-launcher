# Plan por hitos (2026-10-11)

Orden pedido: login MS → instancias y lanzamiento → mods → UI completa → servicios en el home-server. Cada hito termina con pruebas en verde, una demo por CLI (o UI en el H4) y un commit con mensaje aprobado.

La fase 3 empieza por H0 y H1.

## Estado (2026-10-12)

- **H0**: hecho.
- **H1**: implementado en el núcleo. La cadena completa tiene pruebas simuladas y el keyring se probó por separado en el equipo. Queda pendiente la validación real de extremo a extremo.
  - El núcleo usa el flujo OAuth por código de dispositivo. El client ID se busca primero en `SHARD_MSA_CLIENT_ID` y luego en el ajuste `msa_client_id`. El valor compilado, `DEFAULT_MSA_CLIENT_ID`, está vacío. Si falta el identificador, Shard devuelve `auth_missing_client_id`. [Configuración del client ID](guia-azure.md#configurar-el-client-id-en-shard).
  - El 20 de noviembre se reportó que el inicio de sesión ya estaba integrado con Prism en el entorno del usuario. La revisión de `main` en `8ca4cc4` confirmó el flujo de Microsoft con client ID configurable y el importador de instancias de Prism. El mecanismo de integración de cuentas con Prism sigue por confirmar.
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
- **H4**: interfaz hecha en 3 rondas, probada con pruebas golden y con el puente sobre los datos reales en solo lectura.
  - Pantallas: Inicio, Instancias con todas sus pestañas, Contenido, Servidores, Skins con visor 3D, Ajustes, Doctor, primer arranque y bandeja.
  - Ejecutable `shard`.
  - Empaquetado hecho y probado: AppImage, Flatpak y `PKGBUILD` local (`tools/package.sh`).
  - Probado en el equipo (niri, Wayland, 2026-10-12): arranque sin errores, marco propio sin bordes, diálogo de cierre, bandeja (icono registrado y «Abrir» desde el icono). Corregido: con `--background` la ventana se mostraba igualmente; el runner ya no la muestra si Dart la arranca oculta.
  - Falta: probar a mano arrastre, maximizar y el flujo completo desde la app (cuenta, instancia, mods, jugar, consola, Doctor). En niri, «minimizar» no hace nada: el compositor no tiene minimizado.
  - Falta compilar para Windows y macOS (necesita rustup).
- **Revisión del núcleo** (Opus, 2026-10-11): 11 defectos encontrados; corregidos 9.
  - Partida en marcha detectada también fuera del proceso.
  - Backups que acompañan a la instancia a la papelera; ids que no heredan backups ajenos.
  - Borrar una instancia exige el juego cerrado.
  - `.part` único por descarga.
  - Token fuera de la serialización.
  - Registro de partida no fatal; candado en `instance.json`; sin doble conteo del tiempo.
  - Reserva atómica de ids.
  - Secretos borrados antes que la cuenta.
- **Defectos restantes de la revisión** (corregidos el 2026-10-12):
  - Los backups siguen los enlaces a carpetas fuera del juego (p. ej. `saves` en otro disco), sin bucles. Los enlaces a archivos de fuera se siguen omitiendo.
  - Con `--detach`, la salida del juego pasa por un relé (`shard __mask-log`) que oculta el token antes de escribir el log. Probado en real con la instancia técnica.

## Próximo paso (2026-11-20)

1. **Validar el flujo completo de Shard con una cuenta real de Microsoft.** Usar un client ID aprobado para Shard. [Guía de Azure](guia-azure.md).
   - Iniciar sesión desde la app, cerrarla por completo y volver a abrirla. Comprobar que la cuenta sigue activa.
   - Ejecutar `shard account refresh` con el mismo directorio de datos para forzar la renovación desde el keyring.
   - Crear o importar una instancia e instalar un mod.
   - Jugar con esa cuenta.
   - Revisar la consola y el Doctor.

   Con esto se cierra H1 y se verifica el flujo principal pendiente de H4.
2. **H5, empezando por LAN por internet.** Registrar primero la decisión. [Decisiones del proyecto](decisiones.md).

## H0 – Esqueleto (corto)

- Workspace Cargo con `core/` (lib + bin `shard`) y `.gitignore` de Rust.
- `paths`, `settings`, `net` (cliente, reintentos, caché), `events`, `Error` y la CLI vacía con `--data-dir`, `--json` y `--offline`.
- Fixtures descargados de fuentes públicas (manifiesto de Mojang, JSON de 1.8.9, 1.20.1 y 26.2, perfiles de Fabric y Quilt).
- **Hecho cuando**: `cargo test`, `cargo clippy -D warnings` y `cargo fmt --check` pasan; `shard --help` funciona.

## H1 – Login de Microsoft y cuentas

- Device code contra `login.microsoftonline.com/consumers` con scope `XboxLive.signin offline_access`, sondeo con `slow_down` y cancelación.
- XBL → XSTS (con traducción de los XErr a mensajes claros) → `login_with_xbox` → entitlements → perfil.
- Refresco con rotación del refresh token; caché del token de MC con su caducidad.
- Almacén: `accounts.json` (público) + keyring (secretos). Varias cuentas y cuenta activa.
- Cuentas offline (UUID v3).
- Skins y capas: ver, subir archivo, reset, capa activa u oculta.
- CLI: `account login|list|use|remove|add-offline|refresh`, `skin …`.
- Pruebas: cadena completa con wiremock (incluidos 403 de app no aprobada y XErr), keyring con backend de prueba.
- **Depende del usuario**: un client ID aprobado para Shard con acceso a la API de Minecraft. Si todavía falta, registrar la app en Azure y pedir el acceso. Hasta la aprobación se puede probar en real hasta XSTS, simular el resto y lanzar con cuentas offline. [Guía de Azure](guia-azure.md).
- **Hecho cuando**: con una app aprobada, `shard account login` deja una cuenta usable que sobrevive a un reinicio (refresco desde el keyring).

## H2 – Instancias, instalación y lanzamiento

- `meta` (manifiesto, versiones, reglas multiplataforma, fusión), `download` (cola, hashes, progreso), assets y natives.
- `java`: detección e instalación desde Adoptium con checksum.
- `instance`: CRUD, papelera de 30 días, cambio de versión, icono, playtime.
- Loaders: Vanilla, Fabric y Quilt primero; luego Forge y NeoForge con instalador oficial (D6).
- `launch`: argumentos (memoria, JVM, resolución, quickPlay/`--server`), proceso desacoplado, logs, crash, salida y playtime. Varias partidas a la vez.
- `import roxy` (`docs/migracion.md`) y Doctor.
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

**Estado (2026-10-12):** actualizador hecho (D8), salvo publicarlo.
- Núcleo: `update` (manifiesto, versiones, firma minisign con comentario confiable, sustitución atómica del AppImage) y `shard update [--install]`.
- App: aviso flotante, sección en Ajustes › Acerca de y ajuste «Buscar actualizaciones automáticamente».
- `tools/release.sh build|upload`: empaqueta, firma, escribe `latest.json` y sube a R2.
- Probado en real con un servidor local: la CLI instala el AppImage publicado (idéntico byte a byte) y rechaza una firma de otra versión; el AppImage r27 encuentra solo una r28 y muestra el aviso.
- Falta: comprar el dominio, crear el bucket `shard-dl` con `dl.shardlauncher.com` y publicar la primera release. El botón «Actualizar» de la app solo se ha probado con tests (la instalación en sí, también desde la CLI).

## Tareas del usuario (en paralelo)

| Cuándo | Qué | Guía |
|---|---|---|
| Ya (si falta el client ID aprobado de Shard) | Registrar la app en Azure, pedir acceso a la API de Minecraft y configurar el client ID | `docs/guia-azure.md` §1–2 |
| Antes de H2 (opcional) | Crear una aplicación de Discord para Rich Presence; pasar el application id | `docs/guia-azure.md` §3 |
| Antes de H3 (opcional) | Pedir clave de API de CurseForge | `docs/guia-azure.md` §4 |
| Antes de H4 | Elegir el nombre visible del launcher (D2) | — |
