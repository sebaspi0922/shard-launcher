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
