# Shard → Slint: plan de migración de la UI

Fecha: 2026-11-24 · Rama: `slint/f0` · Estado: fase 0 entregada, pendiente de
revisión.

Decisión ya tomada por Sebas: toda la UI pasa de Flutter (`app/`) a Slint
(`ui/`), con UX prácticamente idéntica. Flutter sigue intacto en `app/` hasta
el corte (fase 7). Mediciones que la justificaron (PC de Sebas, niri): Slint
30 MB PSS frente a 188 MB de Flutter, arranque 131 ms frente a 387 ms,
bandeja OK en Noctalia. Plataformas: Linux (Wayland y X11), Windows y macOS.

Contenido:

1. [Inventario de la app Flutter y su equivalente en Slint](#1-inventario)
2. [Arquitectura](#2-arquitectura)
3. [Arnés de paridad](#3-arnés-de-paridad)
4. [Fases](#4-fases)
5. [Licencia de Slint](#5-licencia-de-slint)
6. [Riesgos y mitigación](#6-riesgos-y-mitigación)
7. [Resultados de la fase 0](#7-resultados-de-la-fase-0)

---

## 1. Inventario

Tamaño de la app: 124 archivos `.dart`, ~20,5 K líneas escritas a mano en
`lib/ui` + `lib/core`. Bridge: 16 módulos en `app/rust/src/api`, ~105
funciones y un stream de eventos. 46 goldens de 1280 × 800 en
`app/test/goldens`. Leyenda: ✅ hecho en F0 · ➖ equivalente directo en Slint
· ⚠️ hueco (hay que construirlo) · 🔴 riesgo.

### 1.1 Rutas y pantallas (go_router, `app/lib/app.dart`)

| Ruta Flutter | Archivos | Slint | Goldens (escenas de paridad) | Fase |
|---|---|---|---|---|
| marco (`ShellRoute`) | `shell/app_shell.dart`, `title_bar.dart`, `backdrop.dart`, `update_banner.dart`, `ui_scale.dart` | ✅ `MainWindow` + `NavRail` + `TitleBar` + `Atmosphere`; enum `Route` en vez de URL; ⚠️ fondo con imagen + desenfoque, banners, escala de UI | todas | 0 / 1 |
| `/instances` | `instances/instances_page.dart` | ✅ `screens/instances.slint` (rejilla, lista, búsqueda, carga, error, vacío, sin resultados) | `instances_grid_es`, `instances_list_en` + 3 escenas nuevas | 0 |
| `/` Inicio | `home/home_page.dart` (684) | ⚠️ 4 estilos (Grafito, Aurora, Paisaje, SebasPi con esfera ASCII) | `home_es`, `home_empty_en`, `home_error_es`, `home_downloading_es`, `home_style_*` ×4, `web_home_graphite_es` | 1 |
| `/instances/:id?tab=` | `instance_detail_page.dart` + 6 pestañas (contenido, mundos, capturas, backups, ajustes, registros) | ⚠️ `InstanceView { id, tab }` | `instance_*` ×7 | 3 |
| `/content` | `content/content_page.dart`, `project_sheet.dart`, `install.dart`, `modpack_dialog.dart` | ⚠️ buscador + ficha lateral | `content_search_es`, `content_curseforge_en`, `content_target_es`, `project_sheet_es`, `modpack_installing_es` | 4 |
| `/servers` | `servers/*` | ⚠️ lista con ping, reordenar | `servers_es`, `server_dialog_en` | 4 |
| `/skins` | `skins/skins_page.dart`, `skin_viewer.dart`, `skin_mesh.dart` | 🔴 visor 3D (ver riesgos) | `skins_es`, `skins_offline_en` | 2 |
| `/settings` | `settings/settings_page.dart`, `appearance_group.dart`, `java_group.dart`, `curseforge_key_dialog.dart` | ⚠️ grupos General, Apariencia, Juego, Java, Descargas, Integraciones, Acerca de | `settings_*` ×5 | 5 |
| `/welcome` | `first_run/first_run_page.dart` (1154) | ⚠️ 4 pasos (bienvenida, importar, cuenta, listo) | `first_run_*` ×4 | 5 |

Transiciones: `_fade` entre rutas (260 ms) → `opacity` animada en el
contenedor de la ruta. `Hero` (3 usos) → fundido + escala coordinados; se
acepta que la transición no sea idéntica (no hay golden de transiciones).

### 1.2 Diálogos y menús (19 `showDialog`/menús en 17 archivos)

| Diálogo | Origen | Fase |
|---|---|---|
| ¿Cerrar Shard? (preguntar / segundo plano / recordar) | `shell/app_shell.dart` | ✅ 0 |
| Panel de cuentas, login Microsoft (código de dispositivo), cuenta offline | `accounts/*` | 2 |
| Crear instancia, cambiar versión, papelera, menú de la instancia | `instances/*` | 3 |
| Restaurar backup, borrar backup, captura a pantalla completa (flechas ←/→) | `instances/backups_tab.dart`, `screenshots_tab.dart` | 3 |
| Modpack, ficha de proyecto (sheet) | `content/*` | 4 |
| Servidor (añadir/editar), confirmar borrado | `servers/*` | 4 |
| Doctor, clave de CurseForge, confirmar, texto | `doctor/*`, `settings/*`, `widgets/common.dart` | 5 |
| Selector desplegable (`select_button.dart`) | `widgets/` | 3 |

Base: ✅ `AppDialog` (velo, panel con borde fuerte y sombra, `@children`).

### 1.3 Componentes (`ui/widgets`, `ui/theme`)

| Flutter | Slint | Estado |
|---|---|---|
| `tokens.dart`, `app_theme.dart`, `styles.dart` | `theme.slint`: `Palette` (AppColors), `Theme` (espacio, radios, movimiento, sombras, `blend`), `Type` (TextTheme) | ✅ Grafito; ⚠️ los otros 3 estilos y acentos (F1) |
| Texto (`TextTheme` con `height`) | `Txt`: mismo alto que el párrafo de Flutter y reparto proporcional del interlineado (métricas de Inter) | ✅ |
| `FilledButton`, `OutlinedButton`, `TextButton`, `IconButton.outlined`, `SegmentedButton` | `buttons.slint` (área de toque de 48 como Material) | ✅ |
| `PlayButton` (6 fases) | `PlayButton` + `Fmt.launch-label` | ✅ visual; ⚠️ progreso real (F1) |
| `TextField` (relleno, prefijo, borrar) | `inputs.slint` `TextField` | ✅ |
| `Checkbox`, `Switch`, `Slider` (`memory_slider`), `Tooltip`/`SideTooltip`, `Tag`, `Panel`, `SettingRow` | ✅ `Checkbox`; ⚠️ el resto | 1–5 |
| `EmptyState`, `ErrorNote` (con «Detalles»), `SnackBar` (`showToast`) | `feedback.slint` | ✅ |
| `CircularProgressIndicator` | `Spinner` (arco con `Path`) | ✅ |
| `art.dart`: paisaje de bloques, icono de instancia, marca | ✅ paisaje en Rust (`art.rs`, mismo hash y `Random` que Dart, probado), icono y marca en Slint | ✅ |
| `atmosphere.dart` (fondo Grafito animado) | `Atmosphere` | ✅ |
| `avatar.dart` (cara generada / cara de la skin) | `PlayerAvatar` + `face.rs` | ✅ generada; ⚠️ skin descargada (F2) |
| `ascii_sphere.dart`, `backdrop.dart` (imagen + blur + atenuado) | ⚠️ esfera: `Text` por fotograma; blur: pre-desenfoque en Rust | 1 |
| `smooth_scroll.dart` (rueda suave) | `Flickable` nativo; ⚠️ comparar sensación a mano | 1 |
| `skin_mesh.dart`, `skin_viewer.dart` | 🔴 ver riesgos | 2 |
| `launch_feedback.dart`, `crash_banner.dart`, `update_banner.dart` | ⚠️ | 1 |
| `media.dart` (iconos de mods por URL, caché) | ⚠️ descarga en tokio + `Image::from_rgba8` | 3 |
| `material_symbols_icons` | ✅ subset de Material Symbols Rounded con los mismos ejes que Flutter (`tools/slint/subset_icons.py`, lee `icons.slint`) | ✅ |

### 1.4 Animaciones

| Flutter (usos) | Slint |
|---|---|
| `AnimatedContainer` (12), `AnimatedOpacity` (6), `AnimatedPositioned` (1) | `animate <prop> { duration; easing }` (✅ en hover, raíl, Jugar, aviso) |
| `AnimatedSlide` (2), `AnimatedSwitcher` (2) | `animate x/y` + `opacity`; switcher = dos hijos con opacidad cruzada |
| `AnimationController` (2), `Ticker` (9), `CustomPaint` (16) | `animation-tick()` (fondo ✅), `Timer` de Rust para lo que no deba ir a 60 fps |
| `Hero` (3) | ⚠️ fundido + escala (diferencia aceptada) |
| `Motion.emphasized` `Cubic(0.16, 1, 0.3, 1)`, `easeOutCubic` | `cubic-bezier(...)` |
| Modo rendimiento / reducir movimiento | ✅ `Theme.motion = false` y fondo quieto (`ui.json` `performance`) |

### 1.5 Estados de carga, error y vacío

Cada pantalla con datos tiene los cuatro estados de Riverpod `AsyncValue`:
carga (`Spinner`), error (`ErrorNote` con texto del tipo de error y detalle),
vacío (`EmptyState`) y datos. En Slint: `enum LoadState { loading, ready,
failed }` por pantalla en su global y Rust decide cuál. Textos de error: ✅
`errors.rs` usa la misma tabla `kind → clave del ARB` que `errors.dart`.

### 1.6 Atajos de teclado

Pocos: flechas ←/→ en el visor de capturas, Esc para cerrar diálogos,
Intro para aceptar en campos, rueda con modificadores para desplazamiento
horizontal (`smooth_scroll.dart`). Slint: `FocusScope`/`key-pressed` en el
componente; `TextInput.accepted`. Fases 3 y 5.

### 1.7 i18n

`app/lib/l10n/app_{es,en}.arb`: 633 claves, 27 plurales (con `=0` en
algunos), 108 con argumentos, ningún `select`. ✅ `build.rs` convierte los ARB
a catálogos gettext y Slint los empaqueta (`@tr("clave")`, sin contexto).
Los ARB siguen siendo la única fuente de textos. Convenciones en
`ui/src/arb.rs`: argumentos `{0}`, `{1}`… en el orden de la plantilla
(`app_es.arb`), variable del plural `{n}`, `=0` como clave `<clave>.zero`.
Idioma: ajuste del núcleo → idioma del sistema → español (igual que
`app.dart`). Para lo poco que se formatea en Rust hay `i18n::text`. Fechas
relativas y tiempo jugado: ✅ `fmt.slint` (mismas reglas que `format.dart`).
✅ Un test falla si un `@tr` usa una clave que no existe en los dos ARB.

### 1.8 Ajustes y preferencias

- Núcleo (`settings.json`, vía `lcore`): idioma, cliente de Azure,
  `close_behavior` (✅ usado), varias partidas, memoria y JVM por defecto,
  Java por versión, descargas simultáneas, Discord RPC, buscar
  actualizaciones.
- UI (`<datos>/ui.json`, el mismo archivo que lee Flutter): `style`,
  `accent`, `background`, `image`, `blur`, `dim`, `performance` (✅ leído),
  `first_run_done`.

### 1.9 Ventana, bandeja y sistema

| Flutter | Slint | Estado |
|---|---|---|
| `window_manager`: sin marco, 1280 × 800, mínimo 960 × 600, arrastre, doble clic maximiza, min/max/cerrar | `no-frame`, `WindowMoveArea` + `double-clicked`, `set_minimized`/`set_maximized` | ✅ ⚠️ redimensionar desde el borde (riesgos) |
| `close_behavior` (preguntar / bandeja / salir), Alt+F4 | `Shell` + `on_close_requested` | ✅ |
| `tray_manager`: abrir, jugar la última, salir; clic abre | `SystemTrayIcon` (SNI/ksni en Linux) | ✅ |
| `--background` (arranque oculto) | `shard-ui --background` | ✅ |
| `launch_at_startup` | ⚠️ crate `auto-launch` (registro, LaunchAgent, `.desktop`) | 6 |
| `file_selector` (modpack, fondo, skin) | ⚠️ `rfd` (portal XDG en Linux) | 1–4 |
| `url_launcher` (15) | ⚠️ `open` crate / `webbrowser` | 2–5 |
| `ui_scale.dart` | ⚠️ factor sobre la escala de Slint (`SLINT_SCALE_FACTOR` o propiedad global) | 1 |

### 1.10 Eventos del núcleo

`lcore::events::Event`: `Progress`, `TaskFinished`, `AccountsChanged`,
`GameStarted`, `GameLog`, `GameCrashed`, `GameExited`. ✅ F0 escucha
`GameStarted`/`GameExited`/`GameCrashed`/`AccountsChanged` (estado de
partida en tarjetas y en la barra de título, recarga). ⚠️ `Progress` y
`TaskFinished` (descargas, lanzamiento con progreso) en F1; `GameLog` con el
mismo agrupado de 50 ms / 200 líneas que el bridge, en F3 (consola).

---

## 2. Arquitectura

```
shard-launcher/
  core/          lcore (sin cambios)
  app/           Flutter, intacto hasta el corte
  ui/            crate shard-ui (binario shard-ui)
    build.rs     ARB → .po y compilación de la UI
    slint/       theme, icons, fmt, components/*, screens/*, app.slint
    src/
      main.rs        capa fina: argumentos, tokio, núcleo, ventana, bandeja
      controller.rs  estado de Rust → globals/modelos de Slint; callbacks
      backend.rs     tareas contra lcore en tokio y eventos del núcleo
      window.rs      cierre, bandeja, segundo plano
      instances.rs   lógica pura de la pantalla (orden, búsqueda, fases)
      art.rs face.rs arte procedural (mismo resultado que Dart)
      arb.rs i18n.rs errors.rs prefs.rs fixtures.rs
    tests/       paridad.rs, interaccion.rs
  tools/         paridad.sh, paridad/, medir.sh, slint/
```

**`ui` usa `lcore` directo.** Nada de serialización ni tipos espejo: un
`Backend { core: lcore::Core, rt: tokio::Handle }` llama al núcleo.

**Estado: Rust es la fuente de verdad.** Cada pantalla tiene:
- un modelo Rust puro y probado (`instances::Model`: lista, búsqueda, vista,
  fases de lanzamiento);
- un `export global <Pantalla>State` en Slint con propiedades `in` y
  callbacks;
- `Controller::refresh` copia el modelo a la UI (`VecModel` actualizado fila
  a fila: sin recrear elementos, sin perder el hover).
Slint solo guarda estado de vista (hover, foco, texto que se escribe). Lo
que deba persistir o afectar a otras pantallas vuelve a Rust por callback.

**Hilos.** La UI corre en el hilo principal (bucle de eventos de Slint). El
núcleo, en un runtime tokio de 2 hilos. Cada acción es
`rt.spawn(async { … core … ; post(move |ctl| ctl.…) })`; `post` usa
`slint::invoke_from_event_loop`. La UI nunca espera al núcleo, salvo al
arrancar (`Core::open` y ajustes: leer unos JSON locales, para pintar el
primer fotograma ya con idioma y ajustes; medido: 70 ms hasta el primer
fotograma desde `main`).

**Las ~105 funciones del bridge.** Desaparecen como capa: cada una era
`on_rt(core.xxx())` + conversión a tipos planos. En Slint pasan a métodos de
`Backend` (o del controlador de su pantalla) que llaman a `lcore` y
entregan el resultado con `post`:

| Módulo del bridge | Funciones | Dónde va | Fase |
|---|---|---|---|
| `app` (start, eventos, logs) | 6 | `main.rs` + `backend::pump_events` ✅ | 0 / 3 |
| `instances` | 12 | `backend` + `instances.rs` (listar ✅, lanzar ✅) | 0 / 3 |
| `accounts`, `skins` | 7 + 6 | `accounts` | 2 |
| `worlds`, `backups`, `import` | 9 + 8 + 5 | `instance_detail` | 3 / 5 |
| `content` | 16 | `content` | 4 |
| `servers` | 7 | `servers` | 4 |
| `settings`, `java`, `ui_prefs`, `update`, `doctor` | 3 + 2 + 3 + 3 + 2 | `settings` (`prefs.rs` ✅ lectura) | 1 / 5 |

**Stream de eventos.** `core.subscribe()` (broadcast de tokio) → una tarea
reparte a los controladores con `post`. Los registros del juego se agrupan
(50 ms o 200 líneas) antes de cruzar al hilo de la UI, como hoy.

**Datos falsos.** `fixtures.rs` = `fake_bridge.dart` (mismos ids, nombres,
fechas relativas). Lo usan el arnés de paridad y los tests de interacción, y
`shard-ui --seed DIR` lo escribe con `lcore` para `tools/medir.sh`. En cada
fase se porta la parte de `fake_bridge.dart` que necesite.

**Renderer.** Software por defecto (es el que dio 30 MB); `SLINT_BACKEND`
permite FemtoVG/OpenGL. El mismo renderer por software pinta las escenas de
paridad, así lo que se compara es lo que ve el usuario.

---

## 3. Arnés de paridad

Se usa en todas las fases. Un comando:

```sh
tools/paridad.sh <escena|all> [--refs] [--out DIR] [--no-fail]
```

1. **Referencias de Flutter** (`tools/paridad/ref/<escena>.{png,json}`, en el
   repo). Las genera `tools/paridad/flutter/parity_ref_test.dart` con
   `flutter test` (`--refs`): mismo `pumpApp`, `FakeBridge`, fuentes y
   1280 × 800 que los goldens; las escenas que repiten un golden dan el mismo
   PNG byte a byte (`instances_grid_es` = golden). El JSON trae las cajas de
   los elementos clave y el instante del fondo animado. El test vive fuera de
   `app/` y solo se copia para correrlo. CI no necesita Flutter.
2. **Captura Slint** (`ui/tests/paridad.rs`): backend de pruebas de Slint
   con el renderer por software real, sin ventana ni GPU, tiempo simulado,
   escala 1, fondo quieto en el mismo instante que la referencia, mismos
   datos (`fixtures`). Exporta PNG y cajas por `accessible-id`. Las
   interacciones de la escena (escribir en el buscador) son eventos de
   teclado y clics de verdad.
3. **Comparación** (`tools/paridad/compare.py`): SSIM global ≥ 0,97 por
   escena; cajas ±2 px (textos `txt-*`: x, y y alto; botones `*-play`, que se
   anclan a la derecha y miden lo que su texto: borde derecho, y y alto);
   imagen Flutter | Slint | diferencias; `informe.html`, `informe.md`,
   `informe.json`. Sale con error si algo no pasa.

**Pruebas de interacción** (`ui/tests/interaccion.rs`): clics y teclado
sobre la UI y aserciones sobre el estado (modelo de Rust, globals de Slint,
acciones pedidas al núcleo), no sobre píxeles.

**Rendimiento** (`tools/medir.sh`): arranque hasta el primer fotograma, RSS y
PSS a 10 s, CPU en reposo, RSS/PSS tras 30 s de uso automatizado, bandeja
(menú + «Salir») y tamaño. Mediana de N repeticiones, Flutter frente a Slint
con los mismos datos.

**Añadir una escena** (cada fase): un `testWidgets` en
`parity_ref_test.dart` con sus cajas (`put('nombre', finder)`), `--refs`, y
la misma escena en `SCENES` de `paridad.rs` con `accessible-id` en los
elementos. Una pantalla está hecha cuando pasa el arnés **y** Sebas la ve
bien en el informe.

---

## 4. Fases

Verticales (una pantalla o flujo completo), ordenadas por riesgo, siempre
compilables y usables: lo que aún no existe muestra su título y no rompe
nada. Estimación en hilos de agente (como F0, que fue uno).

| Fase | Contenido | Por qué en este orden | Hecho cuando | Hilos |
|---|---|---|---|---|
| **0** ✅ | Base (tema, componentes, marco, bandeja, cierre, i18n), Instancias, arnés, medición, CI | — | ver §7 | 1 |
| **1** | Inicio con los 4 estilos, fondo con imagen + desenfoque + atenuado, acento desde imagen (`material-colors`), esfera ASCII, banners de actualización y fallo, flujo de lanzamiento con progreso (`Progress`/`TaskFinished`), escala de UI, tooltips, transición entre rutas | Es la pantalla principal y concentra los riesgos visuales (desenfoque, estilos, animaciones) | 10 goldens de inicio + `update_banner_es`, `crash_banner_es` ≥ 0,97 y cajas ±2 px; test de acento = mismo color que Flutter en 4 imágenes; lanzar una instancia real | 2–3 |
| **2** | Cuentas (panel, Microsoft con código, offline, Shard/Yggdrasil), skins con visor 3D, capas, caras de skin en el raíl | Visor 3D es el mayor riesgo técnico | `accounts_panel_es`, `ms_login_es`, `offline_account_en`, `skins_es`, `skins_offline_en`; geometría del visor con tests contra `skin_mesh.dart`; login real probado a mano | 2 |
| **3** | Detalle de instancia (6 pestañas), consola en vivo, crear instancia, cambiar versión, papelera, backups, mundos, capturas | Mucho volumen; la consola es el riesgo de rendimiento | `instance_*` ×7, `create_instance_es`, `change_version_es`, `trash_en`, `backup_restore_es`, `instance_menu_es`; consola con 5000 líneas sin bajar de 60 fps | 3 |
| **4** | Contenido (Modrinth/CurseForge, ficha, instalación, modpacks) y Servidores | Red y listas con imágenes | `content_*` ×3, `project_sheet_es`, `modpack_installing_es`, `servers_es`, `server_dialog_en` | 2 |
| **5** | Ajustes (todos los grupos, Java, Acerca de con `AboutSlint`), primer arranque (4 pasos, importar), Doctor | Formularios, poco riesgo | `settings_*` ×5, `first_run_*` ×4, `doctor_es` | 2 |
| **6** | Plataforma: Windows y macOS probados a mano, autoarranque, redimensionar sin marco, accesibilidad (Orca/Narrador), IME, HiDPI 1,25/1,5/2, empaquetado (AppImage/Flatpak/pacman, MSI, dmg) | Lo que el arnés no ve | checklist por plataforma (niri, GNOME, KDE, X11, Windows 11, macOS) y `medir.sh` en el PC de Sebas | 2 |
| **7** | Corte: una versión con las dos UIs, luego borrar `app/` y `app/rust` | — | una semana de uso de Sebas sin regresiones | 1 |

Total restante: **~14–15 hilos** (12 si se acepta paridad «parecida» en
las pantallas de poco uso).

---

## 5. Licencia de Slint

Slint se distribuye con tres licencias a elegir (cabecera SPDX de cada
crate: `GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR
LicenseRef-Slint-Software-3.0`; textos en
<https://github.com/slint-ui/slint/tree/master/LICENSES>).

| | Royalty-free 2.0 | GPLv3 | Comercial (Software 3.0) |
|---|---|---|---|
| Coste | gratis | gratis | de pago (<https://slint.dev/pricing>) |
| Para | «Desktop, Mobile, and Web Applications»: un launcher de escritorio entra | cualquier uso | cualquier uso, incluido embebido |
| Obligación | atribución: (a) widget `AboutSlint` en una pantalla «Acerca de» accesible desde el menú principal (o en la pantalla de inicio si no hay «Acerca de»), **o** (b) la insignia «Made with Slint» en una web pública, mejor donde se descargan los binarios | todo Shard pasa a GPLv3: código fuente publicado y misma licencia para quien lo redistribuya | según contrato; sin atribución obligatoria |
| Límites | no sistemas embebidos; no redistribuir Slint suelto ni exponer su API | copyleft fuerte | — |
| Encaje con Shard | bueno: Shard ya tiene «Acerca de» en Ajustes; no cambia la licencia del launcher | solo si Shard va a ser software libre GPL | innecesario hoy |

Cita (Royalty-free 2.0, §2): «You may distribute the Software as part of an
Application, modified or unmodified, provided that You do either of the
following: (a) Display the AboutSlint widget in an "About" screen or dialog
that is accessible from the top level menu of the Application. […] (b)
Display the Slint attribution badge on a public webpage, preferably where
the binaries of your Application can be downloaded from […]».

**Recomendación: Royalty-free 2.0**, cumpliendo con las dos atribuciones
(`AboutSlint` en Ajustes → Acerca de en la fase 5 y la insignia en la página
de descargas). Deja libre la licencia de Shard (hoy el repo es privado y sin
licencia) y no cuesta nada. GPLv3 solo si Sebas decide publicar Shard como
software libre GPL; comercial solo si algún día hay embebido o no se quiere
atribución. **Lo decide Sebas**; `ui/Cargo.toml` declara de momento
`GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0` para no adelantar la
decisión.

---

## 6. Riesgos y mitigación

| Riesgo | Prob. | Impacto | Mitigación |
|---|---|---|---|
| **Fuentes fraccionarias en el renderer por software** (nuevo, medido en F0): Slint 1.19 rasteriza el glifo a píxel entero truncado (13,5 px → 13 px; `fonts.rs`, `cast()` a `i16`) aunque el layout use 13,5. Se nota en el cuerpo de texto (13,5) y en 1,25×/1,5× | segura | bajo-medio | el arnés lo absorbe (SSIM 0,978–0,991); abrir issue/PR en Slint para redondear o usar tamaños fraccionarios; alternativa: tamaños enteros en el tema (13 o 14) si Sebas lo prefiere a ojo; FemtoVG/Skia no tienen el problema (más RAM) |
| Texto e IME (acentos, teclas muertas, CJK) | media | medio | F0 prueba «técnico» con eventos de teclado; F6 a mano: teclas muertas en X11/Wayland/Windows, fcitx/ibus; Slint usa el IME de winit |
| Accesibilidad | media | bajo-medio | Slint usa AccessKit; F0 ya da rol y etiqueta a botones, pestañas, casillas y campos (los tests los encuentran por etiqueta); F6 con Orca y Narrador |
| HiDPI | media | medio | escala por pantalla de winit; arte de tarjetas pintado a escala física (✅); F6 a 1,25/1,5/2 y escenas de paridad a 2× |
| Windows / macOS | media | medio | CI compila y pasa los tests en los dos (F0); a mano en F6: bandeja (Shell_NotifyIcon / NSStatusItem), ventana sin marco, `WindowMoveArea`, autoarranque |
| Redimensionar una ventana sin marco | alta | medio | winit no pone bordes de redimensionado sin decoraciones: zonas de borde con `drag_resize_window` (winit, feature `unstable-winit-030`) en F1 |
| Visor 3D de skins | media | alto | Slint no pinta triángulos con textura: geometría de `skin_mesh.dart` portada a Rust puro con tests, rasterizado propio (ortográfica, caras por texel en orden de pintor) a una imagen; F2 empieza por aquí |
| Desenfoque del fondo | segura | bajo | no hay filtro de blur: imagen desenfocada en Rust al cambiar fondo/sigma (`image`/`fast_image_resize`), en caché |
| Animaciones implícitas y `Hero` | segura | bajo | `animate` cubre casi todo; `Hero` → fundido + escala; capturar transiciones a 0/50/100 % si hace falta |
| Bandeja sin host SNI | baja | bajo | sin watcher (Bash, GNOME sin extensión) `SystemTrayIcon` falla: Shard lo registra y, como Flutter, solo minimiza; `tools/slint/sni-watcher.py` para medir sin panel |
| Dependencia de Slint (cambios de versión) | baja | medio | versión fijada (`=1.19.0`); subir de versión en su propio PR con el arnés en verde |

---

## 7. Resultados de la fase 0

Ver la sección de evidencias del PR (se completa con la medición final).
