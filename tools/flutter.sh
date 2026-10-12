#!/bin/sh
# Ejecuta Flutter (versión fijada con fvm en app/.fvmrc) con el sustituto de
# rustup en el PATH si no hay rustup real. Uso: tools/flutter.sh run -d linux
here=$(cd "$(dirname "$0")" && pwd)
flutter_bin="$HOME/fvm/versions/$(sed -n 's/.*"flutter": *"\([^"]*\)".*/\1/p' "$here/../app/.fvmrc")/bin"
if ! command -v rustup >/dev/null 2>&1; then
  PATH="$here/rustup-shim:$PATH"
fi
PATH="$flutter_bin:$HOME/.cargo/bin:$PATH"
export PATH
cd "$here/../app" && exec flutter "$@"
