#!/bin/sh
# Releases de Shard para Linux (decisión D8 en docs/decisiones.md).
#
#   tools/release.sh build [--notes TEXTO] [--notes-url URL]
#       Empaqueta AppImage, Flatpak y pacman, firma cada archivo con minisign y
#       deja en dist/release/<versión>/ el mismo árbol que tendrá el bucket:
#         latest.json
#         linux/<versión>/<archivos versionados> (+ .minisig)
#         linux/<nombres fijos para la web> (enlaces duros)
#       Exige el árbol de git limpio: la versión sale del número de commits.
#   tools/release.sh upload <versión>
#       Sube dist/release/<versión>/ al bucket R2 con wrangler (sesión iniciada
#       con `npx wrangler login`). latest.json va el último.
#
# Variables:
#   SHARD_SIGNING_KEY  clave privada (~/.config/shard-release/minisign.key)
#   SHARD_DL_BASE      base de las URL (https://dl.shardlauncher.com)
#   SHARD_R2_BUCKET    bucket de R2 (shard-dl)
#   SHARD_ALLOW_DIRTY  =1 permite el árbol sucio (solo para pruebas)
set -eu

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/.." && pwd)
dist="$root/dist"
key=${SHARD_SIGNING_KEY:-$HOME/.config/shard-release/minisign.key}
pubkey="$here/release/minisign.pub"
base=${SHARD_DL_BASE:-https://dl.shardlauncher.com}
bucket=${SHARD_R2_BUCKET:-shard-dl}
version=$(sed -n 's/^version: *\([0-9.]*\).*/\1/p' "$root/app/pubspec.yaml")
pkgver="$version.r$(git -C "$root" rev-list --count HEAD)"
platform=linux-x86_64

# sign <archivo> <tipo>: firma con el comentario confiable que exige la app.
sign() {
  minisign -S -s "$key" -m "$1" -x "$1.minisig" -t "shard $pkgver $platform-$2" >/dev/null
  minisign -V -p "$pubkey" -m "$1" -x "$1.minisig" -q
}

build() {
  notes=""
  notes_url=""
  while [ $# -gt 0 ]; do
    case $1 in
      --notes) notes=$2; shift 2 ;;
      --notes-url) notes_url=$2; shift 2 ;;
      *) echo "opción desconocida: $1" >&2; exit 2 ;;
    esac
  done
  if [ "${SHARD_ALLOW_DIRTY:-0}" != 1 ] && [ -n "$(git -C "$root" status --porcelain)" ]; then
    echo "el árbol de git tiene cambios: haz commit antes de una release" >&2
    exit 1
  fi
  [ -r "$key" ] || { echo "falta la clave privada: $key" >&2; exit 1; }

  # Una sola compilación (con la versión dentro) para los tres paquetes.
  "$here/flutter.sh" build linux --release --dart-define=SHARD_VERSION="$pkgver"
  for target in appimage flatpak arch; do
    SHARD_SKIP_BUILD=1 "$here/package.sh" "$target"
  done

  out="$dist/release/$pkgver"
  files="$out/linux/$pkgver"
  rm -rf "$out"
  mkdir -p "$files"
  cp "$dist/Shard-$pkgver-x86_64.AppImage" "$files/"
  cp "$dist/Shard-$pkgver-x86_64.flatpak" "$files/"
  cp "$dist/shard-$pkgver-1-x86_64.pkg.tar.zst" "$files/shard-$pkgver-x86_64.pkg.tar.zst"
  sign "$files/Shard-$pkgver-x86_64.AppImage" appimage
  sign "$files/Shard-$pkgver-x86_64.flatpak" flatpak
  sign "$files/shard-$pkgver-x86_64.pkg.tar.zst" arch
  # Nombres fijos que enlaza la web (siempre la última versión).
  ln "$files/Shard-$pkgver-x86_64.AppImage" "$out/linux/Shard-x86_64.AppImage"
  ln "$files/Shard-$pkgver-x86_64.flatpak" "$out/linux/Shard-x86_64.flatpak"
  ln "$files/shard-$pkgver-x86_64.pkg.tar.zst" "$out/linux/shard-x86_64.pkg.tar.zst"

  python3 -I - "$out" "$pkgver" "$base" "$platform" "$notes" "$notes_url" <<'EOF'
import datetime, hashlib, json, os, sys
out, ver, base, platform, notes, notes_url = sys.argv[1:7]
names = {
    "appimage": f"Shard-{ver}-x86_64.AppImage",
    "flatpak": f"Shard-{ver}-x86_64.flatpak",
    "arch": f"shard-{ver}-x86_64.pkg.tar.zst",
}
files = {}
for kind, name in names.items():
    path = os.path.join(out, "linux", ver, name)
    data = open(path, "rb").read()
    files[f"{platform}-{kind}"] = {
        "url": f"{base}/linux/{ver}/{name}",
        "size": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
        "signature": open(path + ".minisig").read(),
    }
manifest = {
    "schema": 1,
    "version": ver,
    "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "notes": notes or None,
    "notes_url": notes_url or None,
    "files": files,
}
with open(os.path.join(out, "latest.json"), "w") as f:
    json.dump(manifest, f, ensure_ascii=False, indent=2)
    f.write("\n")
EOF
  echo "Release $pkgver lista en $out"
}

upload() {
  ver=${1:?falta la versión (carpeta de dist/release)}
  out="$dist/release/$ver"
  [ -f "$out/latest.json" ] || { echo "no existe $out/latest.json" >&2; exit 1; }
  wrangler="npx --prefix $root/../shard-web wrangler"
  put() {
    obj=$1
    shift
    $wrangler r2 object put "$bucket/$obj" --remote --file "$out/$obj" "$@"
  }
  for f in "$out"/linux/"$ver"/*; do
    put "linux/$ver/$(basename "$f")"
  done
  for f in Shard-x86_64.AppImage Shard-x86_64.flatpak shard-x86_64.pkg.tar.zst; do
    put "linux/$f" --cache-control "no-cache"
  done
  put latest.json --content-type application/json --cache-control "no-cache"
  echo "Publicada $ver en $bucket"
}

case ${1:-} in
  build) shift; build "$@" ;;
  upload) shift; upload "$@" ;;
  *) sed -n '2,21p' "$0"; exit 2 ;;
esac
