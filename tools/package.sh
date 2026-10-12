#!/bin/sh
# Empaqueta Shard para Linux a partir de la compilación release.
#
#   tools/package.sh stage <dir>   árbol /usr con la app, .desktop, metainfo e iconos
#   tools/package.sh arch          paquete de pacman en dist/ (no lo instala)
#   tools/package.sh appimage      AppImage en dist/ (necesita appimagetool)
#   tools/package.sh flatpak       Flatpak (.flatpak) en dist/ (necesita flatpak-builder)
#
# SHARD_SKIP_BUILD=1 reutiliza la compilación release existente.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/.." && pwd)
app="$root/app"
bundle="$app/build/linux/x64/release/bundle"
pkgfiles="$app/linux/packaging"
dist="$root/dist"
work="$root/target/package"
app_id="dev.shard.Shard"
version=$(sed -n 's/^version: *\([0-9.]*\).*/\1/p' "$app/pubspec.yaml")
# Versión de paquete creciente entre commits: 0.1.0.r23.
pkgver="$version.r$(git -C "$root" rev-list --count HEAD)"

build() {
  if [ "${SHARD_SKIP_BUILD:-0}" != 1 ] || [ ! -x "$bundle/shard" ]; then
    "$here/flutter.sh" build linux --release
  fi
}

# Copia la app en <dir>/usr/lib/shard con enlace en <dir>/usr/bin y la
# integración de escritorio en <dir>/usr/share.
stage() {
  dest=$1
  rm -rf "$dest"
  mkdir -p "$dest/usr/lib/shard" "$dest/usr/bin" \
    "$dest/usr/share/applications" "$dest/usr/share/metainfo"
  cp -a "$bundle/." "$dest/usr/lib/shard/"
  ln -s ../lib/shard/shard "$dest/usr/bin/shard"
  install -m644 "$pkgfiles/$app_id.desktop" "$dest/usr/share/applications/"
  install -m644 "$pkgfiles/$app_id.metainfo.xml" "$dest/usr/share/metainfo/"
  icons="$dest/usr/share/icons/hicolor"
  for s in 16 24 32 48 64 128 256 512; do
    mkdir -p "$icons/${s}x${s}/apps"
    rsvg-convert -w "$s" -h "$s" "$app/assets/brand/shard.svg" \
      -o "$icons/${s}x${s}/apps/$app_id.png"
  done
  mkdir -p "$icons/scalable/apps"
  install -m644 "$app/assets/brand/shard.svg" "$icons/scalable/apps/$app_id.svg"
}

arch() {
  build
  mkdir -p "$dist" "$work/arch"
  stage "$work/arch/stage"
  cp "$pkgfiles/arch/PKGBUILD" "$work/arch/"
  (
    cd "$work/arch"
    SHARD_STAGE="$work/arch/stage" SHARD_VERSION="$pkgver" \
      PKGDEST="$dist" BUILDDIR="$work/arch/build" makepkg -f --nodeps
  )
}

appimage() {
  build
  tool=${APPIMAGETOOL:-$(command -v appimagetool || true)}
  if [ -z "$tool" ]; then
    echo "falta appimagetool (APPIMAGETOOL=/ruta/appimagetool)" >&2
    exit 1
  fi
  mkdir -p "$dist"
  dir="$work/Shard.AppDir"
  stage "$dir"
  cp "$dir/usr/share/applications/$app_id.desktop" "$dir/"
  cp "$dir/usr/share/icons/hicolor/256x256/apps/$app_id.png" "$dir/"
  ln -s "$app_id.png" "$dir/.DirIcon"
  cat > "$dir/AppRun" <<'EOF'
#!/bin/sh
here=$(dirname "$(readlink -f "$0")")
exec "$here/usr/lib/shard/shard" "$@"
EOF
  chmod +x "$dir/AppRun"
  ARCH=x86_64 "$tool" --no-appstream "$dir" "$dist/Shard-$pkgver-x86_64.AppImage"
}

flatpak() {
  build
  mkdir -p "$dist" "$work/flatpak"
  stage "$work/flatpak/stage"
  cp "$pkgfiles/flatpak/$app_id.yml" "$work/flatpak/"
  # Runtime y SDK de GNOME desde Flathub, en la instalación del usuario.
  command flatpak remote-add --user --if-not-exists flathub \
    https://dl.flathub.org/repo/flathub.flatpakrepo
  (
    cd "$work/flatpak"
    flatpak-builder --user --install-deps-from=flathub --force-clean \
      --repo=repo build "$app_id.yml"
  )
  command flatpak build-bundle "$work/flatpak/repo" \
    "$dist/Shard-$pkgver-x86_64.flatpak" "$app_id"
}

case ${1:-} in
  stage) build; stage "${2:?falta el directorio}" ;;
  arch) arch ;;
  appimage) appimage ;;
  flatpak) flatpak ;;
  *) sed -n '2,9p' "$0"; exit 2 ;;
esac
