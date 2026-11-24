#!/usr/bin/env bash
# Arnés de paridad Flutter → Slint.
#
#   tools/paridad.sh <escena|all> [--refs] [--out DIR] [--no-fail]
#
# 1. (--refs) Regenera las referencias de Flutter en tools/paridad/ref con
#    `flutter test` (mismo arranque y FakeBridge que los goldens). Sin --refs
#    usa las que están en el repo: CI no necesita Flutter.
# 2. Pinta las escenas en Slint con el renderer por software, sin ventana.
# 3. Compara: SSIM >= 0.97 por escena y cajas ±2 px; deja informe.html,
#    informe.md, informe.json y las imágenes lado a lado en --out
#    (por defecto target/paridad/informe).
#
# Escenas: los nombres de tools/paridad/ref/*.json (p. ej. instances_grid_es).
# Necesita cargo y python3 con numpy, pillow y scikit-image (PYTHON=… para
# elegir intérprete). --refs necesita Flutter (tools/flutter.sh o `flutter`).
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
scene=${1:-}
[ -n "$scene" ] || { sed -n '2,17p' "$0"; exit 2; }
shift
refs=0; out="$root/target/paridad/informe"; fail=1
while [ $# -gt 0 ]; do
  case $1 in
    --refs) refs=1; shift ;;
    --out) out=$2; shift 2 ;;
    --no-fail) fail=0; shift ;;
    *) echo "argumento desconocido: $1" >&2; exit 2 ;;
  esac
done
py=${PYTHON:-python3}

if [ "$refs" = 1 ]; then
  test_file="$root/app/test/_parity_ref_test.dart"
  cp "$root/tools/paridad/flutter/parity_ref_test.dart" "$test_file"
  trap 'rm -f "$test_file"' EXIT
  args=(test test/_parity_ref_test.dart)
  [ "$scene" = all ] || args+=(--plain-name "$scene")
  if command -v flutter >/dev/null 2>&1; then
    (cd "$root/app" && flutter "${args[@]}")
  else
    "$root/tools/flutter.sh" "${args[@]}"
  fi
  rm -f "$test_file"
fi

slint_out="$root/target/paridad/slint"
rm -rf "$slint_out" && mkdir -p "$slint_out"
PARIDAD_OUT="$slint_out" PARIDAD_SCENES="$scene" \
  cargo test -p shard-ui --test paridad --quiet -- --nocapture

set +e
"$py" "$root/tools/paridad/compare.py" --ref "$root/tools/paridad/ref" --slint "$slint_out" \
  --out "$out" --scenes "$scene"
status=$?
set -e
echo "informe: $out/informe.html"
[ "$fail" = 1 ] && exit $status || exit 0
