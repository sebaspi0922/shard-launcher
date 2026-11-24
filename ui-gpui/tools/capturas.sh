#!/usr/bin/env bash
# Capturas para la paridad: las dos apps con los datos sembrados y el fondo
# quieto (t = 0, como el golden), ratón fuera de la ventana. Necesita X11
# (o Xwayland) e `import` de ImageMagick.
#   tools/capturas.sh DIR_SALIDA [BUNDLE_FLUTTER] [BIN_GPUI]
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd); repo=$(cd "$here/../.." && pwd)
out=$1; flutter=${2:-$repo/app/build/linux/x64/release/bundle}; gpui=${3:-$repo/ui-gpui/target/release/shard-gpui}
mkdir -p "$out"
for app in flutter gpui; do
  data=$(mktemp -d); "$gpui" --seed "$data" >/dev/null
  echo '{"firstRunDone":true,"performance":true}' > "$data/ui.json"
  if [ $app = flutter ]; then
    SHARD_DATA_DIR=$data SHARD_START_ROUTE=/instances GDK_BACKEND=x11 "$flutter/shard" >/dev/null 2>&1 & pid=$!
  else
    "$gpui" --data-dir "$data" >/dev/null 2>&1 & pid=$!
  fi
  xdotool mousemove 1599 999; sleep 6
  w=$(xdotool search --onlyvisible --name '^Shard$' | head -1)
  import -window "$w" "$out/$app.png"
  "$here/tray.sh" "$pid" menu Salir || kill "$pid"; sleep 2; kill "$pid" 2>/dev/null || true
  rm -rf "$data"
done
echo "capturas en $out"
