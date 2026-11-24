# ui-gpui — spike de Shard en GPUI

Fase 0 de `PLAN.md`: la pantalla **Instancias** (rejilla) con el marco de la
app (raíl, barra de título propia, fondo «Grafito» animado), leyendo las
instancias con `lcore` directo (sin `flutter_rust_bridge`), ventana sin
decoraciones de 1280×800 (mín. 960×600) y bandeja con `gpui-tray`
(Abrir Shard / Ocultar / Salir; clic en el icono = abrir).

Resultados y veredicto: `../ENTREGA-SPIKE.md`.

## Dependencias fijadas

- `gpui-kit = "=0.11.0"` → `gpui-pre = "=0.3.14"` (Longbridge).
- `gpui-tray = "=0.3.0"` con `features = ["gpui-kit"]` (usa el mismo
  `gpui-pre`, un solo GPUI en el árbol).
- Workspace propio (`[workspace]` en `Cargo.toml` y `exclude` en el raíz):
  no toca el `Cargo.lock` de `core/` ni de `app/rust`.

## Compilar

```sh
cd ui-gpui
cargo build --release          # target/release/shard-gpui (~46 MB, con strip)
cargo test                     # hash/Random de Dart, formato, siembra con lcore
```

Linux necesita Vulkan (GPU o Mesa lavapipe). Windows: `cargo build --release`
con MSVC; no se probó en este spike.

Flutter para comparar: **Flutter 3.47.x** (estable). Con 3.50 o la beta 3.51
no compila: `tray_manager 0.8` → `nativeapi 0.5.2` usa `Isolate.create`, que
en 3.50 pasó a `dart:_vm`.

## Uso

```sh
shard-gpui --seed DIR             # escribe las 5 instancias de prueba y sale
shard-gpui --data-dir DIR         # o SHARD_DATA_DIR; sin nada, la carpeta real
shard-gpui --background           # arranca solo en la bandeja
```

`performance: true` en `DIR/ui.json` deja el fondo quieto (como el modo
rendimiento de Flutter). Flutter abre la misma pantalla con
`SHARD_DATA_DIR=DIR SHARD_START_ROUTE=/instances` (parche del spike en
`app/lib/main.dart`).

## Herramientas

- `tools/medir.sh` — Flutter vs GPUI: arranque, RSS/PSS, CPU en reposo, uso de
  30 s, bandeja, tamaños; 3 repeticiones y medianas. Ver cabecera.
- `tools/medir.ps1` — lo mismo en Windows (sin fase de uso).
- `tools/tray.sh` — maneja la bandeja SNI de un proceso por D-Bus.
- `tools/paridad.py` — SSIM y ΔE>10 entre capturas, con mapa de diferencias.
- `tools/subset_icons.py` — genera `assets/fonts/ShardIcons*.ttf` (subset de
  Material Symbols Rounded con los iconos usados).
