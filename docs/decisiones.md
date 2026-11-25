# Decisiones

## D1 – Stack (2026-10-11)

**Elegido: núcleo en Rust + interfaz en Flutter, unidos con flutter_rust_bridge v2.**

Criterios, por orden: rendimiento, que funcione igual en cualquier distro, Windows y macOS, y facilidad de desarrollo.

| Opción | Rendimiento | Igual en todos los SO | Facilidad | Veredicto |
|---|---|---|---|---|
| Tauri 2 (WebView del sistema) | Backend excelente; UI limitada por el WebView | No: WebKitGTK en Linux (lag NVIDIA/Wayland, varía por distro), WebView2 en Windows, WKWebView en macOS | Alta | Descartado; es la causa del problema actual |
| Tauri 3 + CEF | Bueno | Sí | Alta | Revisar cuando sea estable (en oct 2026 sigue en alpha) |
| Electron + núcleo Rust (napi-rs) | UI fluida, pero 250–400 MB de RAM y arranque más lento | Sí (Chromium empaquetado) | La más alta | Plan B si Flutter frena el desarrollo |
| **Flutter + núcleo Rust** | Motor de render propio (Impeller/Skia) compilado AOT, ~100 MB, arranque rápido | Sí: no depende del WebView ni del toolkit del sistema | Media-alta | **Elegido** |
| Slint / Iced (Rust puro) | La mejor, ~30–60 MB | Sí | Baja para una UI con blur, animaciones y tema adaptativo | Demasiado coste para replicar la UI |
| Qt/QML (como Prism) | Muy buena | Sí | Media-baja (C++) | No compensa |

Consecuencias:
- `core/` (crate Rust, sin UI y con tests):
  - auth de Microsoft;
  - manifiestos de Mojang;
  - loaders (Fabric, Quilt, Forge, NeoForge);
  - descargas paralelas con verificación de hash;
  - Java gestionado;
  - instancias;
  - Modrinth y CurseForge;
  - lanzamiento del proceso;
  - Rich Presence.

  Debe funcionar también desde CLI.
- `app/` (Flutter): solo UI y estado. Le habla al núcleo por flutter_rust_bridge.
- `server/` (opcional, más adelante): servicios propios en home-server para sincronización de instancias, amigos y relay de LAN por internet.
- Referencias legales de solo lectura: el código de Modrinth App (`theseus`, GPL-3) y de Prism Launcher (GPL-3) para auth y loaders. Si se copia código, el proyecto pasa a ser GPL-3 (aceptable mientras sea privado/personal).
- Empaquetado:
  - Linux: Flatpak y AppImage.
  - Windows: instalador.
  - macOS: dmg. Sin firma de Apple hay que abrirlo con clic derecho la primera vez.
- Durante la partida, el launcher se puede cerrar o minimizar sin dejar procesos pesados.

## D2 – Nombre e identidad (2026-10-11)

**Elegido por el usuario: «Shard».**

- `APP_ID` = `shard`: directorio de datos `~/.local/share/shard` (Windows `%APPDATA%\shard`, macOS `~/Library/Application Support/shard`), servicios del keyring `shard.msa`, `shard.mc` y `shard.curseforge`, y variables `SHARD_DATA_DIR` y `SHARD_MSA_CLIENT_ID`.
- La CLI se llama `shard`. El crate del núcleo conserva el nombre interno `lcore`, que no se ve.
- Los datos importados que estaban en `~/.local/share/lcore` se movieron con `mv` y se conservan los enlaces duros con Roxy.

## D3 – Cuentas sin servidor propio (2026-10-11)

**Elegido: el launcher no tiene cuenta propia. Las cuentas son de Microsoft (app de Azure propia) u offline, y los secretos solo se guardan en el keyring del sistema.**

- El original bloqueaba todo el launcher si su servidor no respondía y guardaba el refresh token de Microsoft en su backend. Las dos cosas se eliminan.
- Login por device code (igual que el original y apto para la CLI). El login con navegador y redirect a `localhost` queda como mejora de la UI.
- Alternativa descartada: reutilizar el client ID de Roxy o de otro launcher. No es nuestro, puede revocarse y va contra las reglas del proyecto.

## D4 – Migración no destructiva (2026-10-11)

**Elegido: importar desde `.roxy` en modo solo lectura, copiando las instancias y enlazando con enlaces duros el contenido inmutable (libraries, assets, jars del cliente y backups).**

- Instancias copiadas (2,7 GB) en vez de enlazadas: el juego modifica mundos y configs en el sitio y un enlace duro alteraría también la copia de Roxy.
- Mismo sistema de archivos (ext4, sin reflinks): los enlaces duros no ocupan espacio extra.
- Alternativa descartada: mover las carpetas. Es irreversible y deja a Roxy inservible si su servidor vuelve.
- Detalle en `docs/migracion.md`.

## D5 – Layout de datos propio (2026-10-11)

**Elegido: metadatos del launcher fuera del `gameDir` (`instances/<id>/{instance.json, content.json, game/}`) y backups en `backups/<id>/`, fuera de la instancia.**

- En Roxy el directorio de la instancia era el `gameDir` y los backups vivían dentro, así que cada backup podía incluir los anteriores si cambiaba la exclusión, y los mods veían archivos del launcher.
- `libraries/`, `assets/` y `versions/` siguen el layout estándar de Mojang, compatible con lo que ya hay en `.roxy`.
- Detalle en `docs/arquitectura.md` §4.

## D6 – Forge y NeoForge con el instalador oficial (2026-10-11)

**Elegido para empezar: ejecutar el instalador oficial con `--installClient` sobre el directorio de datos, como el original.**

- Es lo más rápido y lo que soportan los propios proyectos.
- Riesgo: depende del comportamiento del instalador en cada versión y SO. Si falla en alguna combinación, se pasa a ejecutar los processors del `install_profile.json` en el núcleo, como Prism Launcher o Modrinth App.

## D7 – Frontera Rust/Flutter (2026-10-11)

**Elegido: `core/` no depende de Flutter ni de flutter_rust_bridge. Un crate puente aparte (`app/rust`) adapta la fachada `Core` a tipos planos y expone los eventos como stream.**

- La CLI y la app usan la misma fachada, así que todo se puede probar y depurar sin UI.
- Si algún día se cambia la UI (plan B de D1: Electron), el núcleo no cambia.

## D8 – Actualizaciones firmadas desde R2 (2026-10-12)

**Elegido: un `latest.json` en `dl.shardlauncher.com` (R2) con, por paquete, la URL, el tamaño, el SHA-256 y la firma minisign del archivo. La app consulta al arrancar y cada 6 horas (desactivable). Solo el AppImage se sustituye solo; con Flatpak y pacman avisa y enlaza la descarga.**

- El comentario confiable de cada firma es `shard <versión> <plataforma>-<paquete>`. La app lo compara con el manifiesto, así que reutilizar una firma vieja para hacer pasar otra versión u otro paquete no funciona.
- Clave pública incorporada en el núcleo (`tools/release/minisign.pub`). La privada está en `~/.config/shard-release/minisign.key`, fuera del repo y sin contraseña para poder firmar desde `tools/release.sh`. Si se pierde, las instalaciones existentes ya no aceptan actualizaciones y hay que reinstalar a mano.
- Versión = `pubspec` + número de commits (`0.1.0.r27`), metida en la compilación con `--dart-define=SHARD_VERSION`. Las compilaciones sin empaquetar no buscan solas.
- `latest.json` vive en R2 junto a los archivos, no en el Worker de la web: una release es una sola subida y no obliga a redesplegar la web.
- Descartado: `AppImageUpdate`/zsync (depende de una herramienta externa y no cubre Flatpak ni pacman) y un repositorio Flatpak propio (más infraestructura que un aviso, para uso personal).
