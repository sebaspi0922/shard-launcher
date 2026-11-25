# Shard

Launcher de Minecraft de escritorio (Linux, Windows y macOS) con núcleo en Rust y UI en Slint.

- Cuentas de Microsoft (código de dispositivo) y offline; servidor de autenticación propio compatible con Yggdrasil (Drasl, ver `deploy/drasl/`).
- Instancias con Fabric, Quilt, Forge y NeoForge, Java gestionado, mods de Modrinth y CurseForge.
- Importa instancias de Prism y de Roxy Launcher.

Documentación en `docs/` (arquitectura, plan, guía de Azure, hosting).

## Compilar

```sh
cargo build --release
cargo test
```

## Licencia

GNU General Public License v3.0 (GPL-3.0). Ver `LICENSE`.
