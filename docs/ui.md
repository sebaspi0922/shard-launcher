# App Flutter: diseño (spec H4, 2026-10-11)

Reimplementa el flujo de Roxy (`research/03-ui-pantallas.md`) sin su cuenta, su ecosistema ni sus assets, con diseño propio. La app solo pinta y gestiona estado. Todo lo demás lo hace `shard` a través del puente (D7).

## 1. Estructura

```
app/
  .fvmrc                 versión de Flutter fijada
  pubspec.yaml
  flutter_rust_bridge.yaml
  rust/                  crate `shard_bridge` (cdylib): adapta `lcore::Core` a frb
    src/api/*.rs         funciones expuestas (una por área)
  lib/
    main.dart            arranque: ventana, puente, ProviderScope
    src/rust/            código generado por frb (no se edita)
    core/                proveedores de Riverpod sobre el puente (estado y caché)
    ui/
      shell/             marco: barra de título, navegación lateral, fondo
      home/  instances/  content/  accounts/  servers/  settings/  console/  doctor/
      widgets/           componentes comunes (tarjetas, botones, diálogos, toasts)
      theme/             tokens, tema oscuro, acento adaptativo
    l10n/                ARB es/en (textos propios)
  assets/                iconos y fondo por defecto propios
```

Paquetes:
- `flutter_riverpod` (estado), `go_router` (rutas) y `flutter_localizations` + `intl` (gen-l10n).
- `window_manager` (barra de título propia, tamaño), `tray_manager` (bandeja), `launch_at_startup`, `url_launcher` y `file_selector`.
- `material_color_utilities` (acento desde la imagen de fondo).

Los plugins de escritorio se validan en Wayland al empezar. Si alguno falla, se busca alternativa o se implementa desde Rust.

Bandeja (validado en niri/Wayland, 2026-10-11): `tray_manager` 0.8 usa nativeapi, que en Linux publica un StatusNotifierItem por D-Bus (GDBus) con su menú por dbusmenu; no necesita `libayatana-appindicator`. Hace falta un panel que aloje SNI (waybar, KDE…); en GNOME, la extensión AppIndicator. El menú solo se exporta con disparador (`ContextMenuTrigger.rightClicked`). El autoarranque crea `~/.config/autostart/Shard.desktop` con `--background` (arranca oculta en la bandeja).

## 2. Frontera con el núcleo

- **Llamadas:** funciones `async` en `app/rust/src/api/` que delegan en `Core` y devuelven tipos planos (structs/enums que frb traduce a clases Dart). Los errores llegan como una excepción con el `kind` estable de `lcore::Error` más su mensaje. La UI traduce por `kind`.
- **Eventos:** `subscribe_events(sink: StreamSink<UiEvent>)` reenvía el bus de `shard`: progreso, logs, juego iniciado/cerrado/crash y cambios de cuentas. Riverpod los reparte con un `StreamProvider`.
- **Operaciones largas** (instalar, lanzar, modpack, backup): devuelven en seguida un `task_id`. El progreso llega por eventos.
- **Logs:** el puente agrupa líneas (cada 50 ms o 200 líneas) para no saturar el canal. Cada partida tiene un anillo de 5000 líneas en Rust (`logs::LogBuffer`) y la consola pide el histórico al abrirse.
- **Un solo `Core`** por proceso, creado al arrancar. Al lanzar la app se llama a `reconcile_sessions`, y el planificador de backups corre en segundo plano.

## 3. Navegación y pantallas

Ventana sin decoraciones del sistema, de 1280×800 por defecto, redimensionable y con mínimo de 960×600. La barra de título es propia: arrastrar, doble clic para maximizar y botones minimizar/maximizar/cerrar. El cierre sigue `close_behavior` (preguntar, bandeja o salir).

Raíl lateral con iconos y etiqueta al pasar el ratón: Inicio, Instancias, Contenido, Servidores, Skins y Ajustes. Abajo, la cuenta activa (cabeza de la skin) con su selector.

| Pantalla | Contenido |
|---|---|
| **Primer arranque** | Bienvenida y elección de idioma. Pregunta si importar desde Roxy (si existe `~/.local/share/.roxy`), `.minecraft` o Prism, mostrando la simulación antes de importar. Luego añadir cuenta (Microsoft u offline). Se puede saltar. |
| **Cuentas** (panel del avatar) | Lista con la activa marcada, «Añadir Microsoft» (diálogo con el código en grande, copiar, «Abrir navegador», cuenta atrás y cancelar), «Añadir offline», quitar. Errores claros: app no aprobada, sin juego, XErr. |
| **Inicio** | Tarjeta grande de «Jugar»: última instancia, botón principal con estados (Preparando → Descargando x/y → Iniciando → En juego → Reintentar) y selector de instancia. Fila de servidores rápidos (estado, jugadores, ping, «Entrar» con su instancia). Instancias recientes. Skin del jugador renderizada en local. |
| **Instancias** | Rejilla o lista con icono, nombre, versión, loader, tiempo jugado y última vez. Crear (nombre, versión con filtro release/snapshot, loader y su versión), importar, papelera. |
| **Detalle de instancia** | Cabecera (icono, nombre, versión, Jugar, abrir carpeta) y pestañas: **Contenido** (mods/recursos/shaders con activar, quitar, actualizar e identificar, más «Añadir» que abre el buscador filtrado), **Mundos** (lista con detalles, editar, datapacks, borrar), **Capturas** (galería), **Backups** (crear, fijar, restaurar completo o solo mundos, automáticos), **Registros** (consola) y **Ajustes** (memoria con deslizador y aviso según la RAM, JVM, Java, resolución, pantalla completa, consola de desarrollo, cambiar versión). |
| **Contenido** | Buscador de Modrinth y CurseForge (este solo con clave) por tipo, versión, loader y orden. Ficha del proyecto con versiones compatibles e instalación en una instancia elegida. Modpacks: instalar desde la búsqueda, un archivo o una URL. |
| **Servidores** | Lista editable y reordenable con ping, instancia y cuenta por servidor. Importar desde `servers.dat`. |
| **Skins** | Vista 3D de la skin actual, subir PNG o URL (clásica o delgada), restablecer y capas. Solo cuentas de Microsoft. |
| **Ajustes** | General (idioma, al cerrar, varias partidas, autoarranque), Juego (memoria y JVM por defecto, Javas detectadas e instalar), Apariencia (fondo, acento auto o preset, desenfoque, oscurecido, modo rendimiento), Descargas (concurrencia), Integraciones (clave de CurseForge, Rich Presence) y Acerca de. |
| **Consola** | Panel o ventana aparte: líneas coloreadas por nivel, filtro por texto y nivel, seguir al final, copiar, abrir el log. Al detectar un crash: botón «Diagnosticar». |
| **Doctor** | Diálogo con diagnósticos por gravedad, explicación, culpable y botón «Aplicar arreglo». |

Notificaciones del sistema: juego cerrado con error, descarga de modpack terminada y backup automático fallido.

## 4. Sistema visual (propio)

- **Tema oscuro** por defecto. El claro se deja para después.
- **Tokens:** fondo base casi negro con un leve tinte del acento, superficies en capas (1–3) con transparencia y bordes de 1 px a baja opacidad, radio 12 en tarjetas y 8 en controles, espaciado en múltiplos de 4.
- **Acento:** se extrae del fondo con `material_color_utilities` (cuantizador + puntuación) o se elige entre presets. A partir de él se genera la escala de tonos (contenedor, texto sobre acento, hover).
- **Fondo:** imagen elegida por el usuario o un degradado generado (sin arte de Roxy), con desenfoque y oscurecido ajustables. En «modo rendimiento» no hay desenfoque ni animaciones de fondo.
- **Tipografía:** Inter (OFL, incluida) para la interfaz y una monoespaciada para la consola. Nada de fuentes de Mojang.
- **Movimiento:** transiciones cortas (150–250 ms), muelles suaves en diálogos y respeto a «reducir movimiento».
- **Escalado:** la UI escala con el tamaño de ventana entre 0,8× y 1,5× respecto a 1280×800 (Roxy lo hacía con `rem`), además de la escala del sistema.
- **Iconos:** Material Symbols o Lucide (licencias abiertas).

## 5. Rendimiento (el motivo del cambio de stack)

- Impeller en Linux si está estable con NVIDIA y Wayland; si no, Skia. Se mide al empezar.
- Listas virtualizadas (consola, mods, búsqueda) e imágenes con caché y tamaño acotado.
- Con el juego abierto, la app puede minimizarse a la bandeja sin coste: no hay animaciones fuera de pantalla y los sondeos se pausan.

## 6. Orden de trabajo en H4

1. Instalar Flutter (fvm), crear `app/` (Linux, Windows y macOS), integrar frb con un `ping` de prueba y medir el arranque en Wayland.
2. Marco (ventana, raíl, tema, l10n) y eventos.
3. Cuentas, Inicio con «Jugar» y consola: es el camino crítico para poder usar la app a diario.
4. Instancias con su detalle, Contenido, Servidores, Skins y Ajustes.
5. Primer arranque con importación, bandeja y autoarranque. Empaquetado (AppImage y Flatpak) y `PKGBUILD` local.
