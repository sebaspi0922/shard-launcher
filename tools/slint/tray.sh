#!/usr/bin/env bash
# Maneja la bandeja SNI de un proceso por D-Bus, como lo haría un panel:
#   tray.sh PID activate | menu "Etiqueta" | labels
# Lee el layout de DBusMenu cada vez (los ids cambian al reconstruir el menú).
set -euo pipefail
pid=$1; cmd=$2
name=$(busctl --user list --no-pager | awk -v p="$pid" '$2==p && $1 ~ /StatusNotifierItem/ {print $1; exit}')
[ -n "$name" ] || { echo "sin item SNI para pid $pid" >&2; exit 1; }
menu=$(gdbus call --session --dest "$name" --object-path /StatusNotifierItem \
  --method org.freedesktop.DBus.Properties.Get org.kde.StatusNotifierItem Menu | grep -o "'/[^']*'" | tr -d "'")
layout=$(gdbus call --session --dest "$name" --object-path "$menu" --method com.canonical.dbusmenu.GetLayout -- 0 -1 '[]')
case "$cmd" in
  activate) gdbus call --session --dest "$name" --object-path /StatusNotifierItem \
              --method org.kde.StatusNotifierItem.Activate -- 0 0 >/dev/null ;;
  labels) grep -o "'label': <'[^']*'>" <<<"$layout" | sed "s/'label': <'//; s/'>//" ;;
  menu) label=$3
        id=$(grep -oP "\((\d+), \{[^}]*'label': <'\Q$label\E'>" <<<"$layout" | grep -oP '^\(\K\d+')
        [ -n "$id" ] || { echo "no hay «$label» en el menú" >&2; exit 1; }
        gdbus call --session --dest "$name" --object-path "$menu" \
          --method com.canonical.dbusmenu.Event -- "$id" clicked '<0>' 0 >/dev/null ;;
esac
