# Licencias de las dependencias (fase 5)

Shard pasa a **GPLv3** (`LICENSE`, `license = "GPL-3.0-only"` en el
workspace). Comprobación repetible: `tools/licencias.py` (lee
`cargo metadata --locked`; sale con error si una dependencia de `lcore` o
`shard-ui`, sin las de desarrollo, no tiene una licencia compatible con
GPLv3 según la lista de la FSF).

Resultado el 2026-11-25: **768 crates, ningún conflicto.**

| Licencia (expresión SPDX) | Crates | Compatible con GPLv3 |
|---|---|---|
| MIT, Apache-2.0 y sus combinaciones (`MIT OR Apache-2.0`, `Apache-2.0 WITH LLVM-exception`…) | ~690 | sí (Apache-2.0 es compatible con GPLv3, no con GPLv2) |
| Unicode-3.0 | 27 | sí |
| Zlib, BSD-2/3-Clause, ISC, 0BSD, Unlicense, CC0-1.0, BSL-1.0 | ~40 | sí (permisivas) |
| `GPL-3.0-only OR LicenseRef-Slint-…` (Slint 1.19) | 14 | sí: se elige la GPLv3 |
| LGPL-3.0-or-later (`minecraft-auth`) | 1 | sí |
| MPL-2.0 (`option-ext`) | 1 | sí (MPL-2.0 §3.3) |
| CDLA-Permissive-2.0 (`webpki-root-certs`) | 1 | sí (permisiva, datos) |
| bzip2-1.0.6 (`libbz2-rs-sys`) | 1 | sí (tipo BSD) |

Notas:

- Slint deja de necesitar la atribución de Royalty-free: con GPLv3 no hay
  que mostrar `AboutSlint`. Se mantiene al pie de Ajustes como cortesía.
- Las fuentes empaquetadas (Inter, JetBrains Mono: OFL-1.1; Material Symbols:
  Apache-2.0) son compatibles para distribuirse junto a un programa GPLv3
  (no son parte del programa).
- `app/` (Flutter) no se revisó: se borra en la fase 7. `pubspec.yaml` no
  tiene campo de licencia; `LICENSE` en la raíz cubre el repo.
