#!/bin/sh
# Exporta las capturas de la app (goldens) a la web, en WebP.
# Uso: tools/export-web-shots.sh ../shard-web
# Regenerar antes los goldens si cambió la interfaz:
#   tools/flutter.sh test --update-goldens test/goldens_test.dart
set -eu
here=$(cd "$(dirname "$0")" && pwd)
goldens="$here/../app/test/goldens"
out="${1:?falta la carpeta del repo de la web}/public/img"
mkdir -p "$out"

# golden → nombre en la web
for pair in \
  home_style_graphite_es:home_style_graphite \
  home_style_sebaspi_es:home_style_sebaspi \
  home_style_landscape_es:home_style_landscape \
  home_style_aurora_es:home_style_aurora \
  instance_content_es:instance_content \
  content_search_es:content_search \
  doctor_es:doctor \
  skins_es:skins; do
  magick "$goldens/${pair%%:*}.png" -quality 82 "$out/${pair##*:}.webp"
done
# Sin fondo: la web anima los anillos detrás (conserva la transparencia).
magick "$goldens/web_home_graphite_es.png" -quality 85 -define webp:alpha-quality=100 \
  "$out/home_graphite_live.webp"
echo "capturas en $out"
