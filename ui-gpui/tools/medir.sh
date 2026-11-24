#!/usr/bin/env bash
# Mide Flutter release vs GPUI release en la pantalla «Instancias» con los
# mismos datos (5 instancias sembradas con `shard-gpui --seed`).
#
#   tools/medir.sh [--runs 3] [--flutter DIR_BUNDLE] [--gpui BIN] [--out DIR]
#                  [--modes "anim quieto"] [--no-usage] [--wayland]
#
# Por defecto las dos apps van por X11 (en niri: xwayland-satellite), así el
# uso se automatiza con xdotool. --wayland las lanza en Wayland nativo (sin
# fase de uso ni capturas: no hay xdotool).
#
# Por app, modo y repetición:
#   - arranque: desde el exec hasta el primer fotograma con instancias
#     (marcador SHARD_FIRST_FRAME que imprimen las dos apps);
#   - RSS y PSS (/proc/PID/smaps_rollup) a los 10 s;
#   - CPU en reposo: % de un núcleo entre los segundos 10 y 20;
#   - 30 s de uso (xdotool: hover por las tarjetas, rueda, escribir en el
#     buscador, cerrar a la bandeja y reabrir desde el menú de la bandeja);
#   - RSS y PSS 5 s después del uso;
#   - salida con «Salir» del menú de la bandeja (comprueba que funciona).
# Modos: «anim» = fondo animado (por defecto en Flutter); «quieto» =
# `performance: true` en ui.json (fondo quieto en las dos).
#
# Requisitos: bash, python3, busctl y gdbus (D-Bus de sesión). Para la fase
# de uso y las capturas: sesión X11 o Xwayland con xdotool e `import`
# (ImageMagick). En Wayland puro sin xdotool usa --no-usage.
# La bandeja necesita un host SNI (waybar, KDE, GNOME con AppIndicator…) para
# verse; el script la maneja por D-Bus aunque no haya panel.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
runs=3
flutter="$repo/app/build/linux/x64/release/bundle"
gpui="$repo/ui-gpui/target/release/shard-gpui"
out="$PWD/medicion-$(date +%Y%m%d-%H%M%S)"
modes="anim quieto"
usage=1
backend=x11
while [ $# -gt 0 ]; do
  case $1 in
    --runs) runs=$2; shift 2 ;;
    --flutter) flutter=$2; shift 2 ;;
    --gpui) gpui=$2; shift 2 ;;
    --out) out=$2; shift 2 ;;
    --modes) modes=$2; shift 2 ;;
    --no-usage) usage=0; shift ;;
    --wayland) backend=wayland; usage=0; shift ;;
    *) echo "argumento desconocido: $1" >&2; exit 2 ;;
  esac
done
mkdir -p "$out"
tray="$here/tray.sh"
hz=$(getconf CLK_TCK)
csv="$out/runs.csv"
echo "app,modo,run,arranque_ms,rss10_kb,pss10_kb,cpu_reposo_pct,rss_uso_kb,pss_uso_kb,bandeja" > "$csv"

if [ "$usage" = 1 ] && { [ -z "${DISPLAY:-}" ] || ! command -v xdotool >/dev/null; }; then
  echo "sin DISPLAY/xdotool: se salta la fase de uso" >&2; usage=0
fi

now_ms() { date +%s%3N; }
mem() { awk '/^Rss:/{r=$2} /^Pss:/{p=$2} END{print r","p}' "/proc/$1/smaps_rollup"; }
ticks() { awk '{print $14+$15}' "/proc/$1/stat"; }
window() { xdotool search --onlyvisible --name '^Shard$' 2>/dev/null | head -1; }

{
  echo "# Equipo"
  echo "- fecha: $(date -Iseconds)"
  echo "- kernel: $(uname -sr)"
  echo "- CPU: $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | xargs) ($(nproc) hilos)"
  echo "- RAM: $(free -h | awk '/^Mem/{print $2}')"
  echo "- GPU: $(lspci 2>/dev/null | grep -iE 'vga|3d' | cut -d: -f3- | xargs || echo '?')"
  echo "- backend de las apps: $backend"
  echo "- sesión: XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-?} XDG_CURRENT_DESKTOP=${XDG_CURRENT_DESKTOP:-?} WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-} DISPLAY=${DISPLAY:-}"
  echo "- Vulkan: $(vulkaninfo --summary 2>/dev/null | grep -m2 deviceName | cut -d= -f2 | xargs || echo 'vulkaninfo no instalado')"
} > "$out/equipo.md"

seed() { # dir modo
  "$gpui" --seed "$1" >/dev/null
  if [ "$2" = quieto ]; then echo '{"firstRunDone":true,"performance":true}' > "$1/ui.json"; fi
}

launch() { # app datos log
  # Mismo backend para las dos (GPUI elige Wayland si ve WAYLAND_DISPLAY).
  local env=(env)
  [ "$backend" = x11 ] && env+=(-u WAYLAND_DISPLAY)
  env+=(GDK_BACKEND="$backend")
  if [ "$1" = flutter ]; then
    "${env[@]}" SHARD_DATA_DIR="$2" SHARD_START_ROUTE=/instances SHARD_SPIKE_TIMING=1 \
      "$flutter/shard" >"$3" 2>&1 &
  else
    "${env[@]}" "$gpui" --data-dir "$2" >"$3" 2>&1 &
  fi
  echo $!
}

use_for_30s() { # pid
  local pid=$1 end=$(( $(date +%s) + 30 )) w x y i
  while [ "$(date +%s)" -lt "$end" ]; do
    w=$(window); [ -n "$w" ] || { sleep 0.5; continue; }
    # hover por las 5 tarjetas
    for i in 0 1 2 3; do
      xdotool mousemove --window "$w" $((244 + i * 289)) 240; sleep 0.25
    done
    xdotool mousemove --window "$w" 244 490; sleep 0.25
    # rueda sobre la rejilla
    xdotool mousemove --window "$w" 700 450 click --repeat 4 --delay 60 5 click --repeat 4 --delay 60 4
    # buscar «sur» y borrar
    xdotool mousemove --window "$w" 740 79 click 1; sleep 0.2
    xdotool type --delay 40 sur; sleep 0.3
    xdotool key ctrl+a BackSpace; sleep 0.2
    # cerrar a la bandeja y reabrir desde el menú
    xdotool mousemove --window "$w" 1253 22 click 1; sleep 1
    if [ -n "$(window)" ]; then echo "AVISO: la ventana sigue visible tras cerrar" >&2; echo 1 > "$out/.cerrar_falla"; fi
    "$tray" "$pid" menu "Abrir Shard" || true; sleep 1.5
  done
}

for mode in $modes; do
  for run in $(seq 1 "$runs"); do
    for app in flutter gpui; do
      data=$(mktemp -d); seed "$data" "$mode"
      log="$out/$app-$mode-$run.log"
      t0=$(now_ms); pid=$(launch "$app" "$data" "$log")
      ff=""
      for _ in $(seq 1 300); do
        ff=$(grep -m1 -oP 'SHARD_FIRST_FRAME unix_ms=\K\d+' "$log" || true)
        [ -n "$ff" ] && break; sleep 0.1
      done
      start=$([ -n "$ff" ] && echo $((ff - t0)) || echo "")
      sleep $(( 10 - ($(now_ms) - t0) / 1000 )) 2>/dev/null || true
      m10=$(mem "$pid"); c0=$(ticks "$pid")
      if [ "$run" = 1 ] && [ "$backend" = x11 ] && [ -n "${DISPLAY:-}" ] && command -v import >/dev/null; then
        xdotool mousemove 0 0 2>/dev/null || true
        w=$(window); [ -n "$w" ] && import -window "$w" "$out/captura-$app-$mode.png" || true
      fi
      sleep 10
      c1=$(ticks "$pid")
      cpu=$(python3 -c "print(round(($c1-$c0)/($hz*10)*100,2))")
      muse=","
      if [ "$usage" = 1 ]; then use_for_30s "$pid"; sleep 5; muse=$(mem "$pid"); fi
      # Bandeja: el menú existe y «Salir» termina el proceso.
      tray_ok=no
      if "$tray" "$pid" labels 2>/dev/null | grep -q "Abrir Shard" && "$tray" "$pid" menu Salir; then
        for _ in $(seq 1 50); do kill -0 "$pid" 2>/dev/null || { tray_ok=si; break; }; sleep 0.1; done
      fi
      kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
      echo "$app,$mode,$run,$start,$m10,$cpu,$muse,$tray_ok" | tee -a "$csv"
      rm -rf "$data"; sleep 2
    done
  done
done

# Tamaños: tal cual, con strip y comprimido (tar.zst -19).
{
  b=$(du -sb "$flutter" | cut -f1)
  tmp=$(mktemp -d); cp -r "$flutter" "$tmp/bundle"; find "$tmp/bundle" -name '*.so' -exec strip {} + 2>/dev/null || true
  bs=$(du -sb "$tmp/bundle" | cut -f1)
  bz=$(tar -C "$tmp" -cf - bundle | zstd -19 -q -c | wc -c); rm -rf "$tmp"
  g=$(stat -c %s "$gpui"); gz=$(zstd -19 -q -c "$gpui" | wc -c)
  echo "app,tal_cual_b,strip_b,tar_zst_b"
  echo "flutter,$b,$bs,$bz"
  echo "gpui,$g,$g,$gz"
} > "$out/tamanos.csv"

python3 - "$csv" "$out/tamanos.csv" > "$out/resumen.md" <<'PY'
import csv, statistics, sys
rows = list(csv.DictReader(open(sys.argv[1])))
def med(app, mode, key):
    v = [float(r[key]) for r in rows if r['app'] == app and r['modo'] == mode and r[key] not in ('', None)]
    return statistics.median(v) if v else None
def mb(kb): return '—' if kb is None else f"{kb/1024:.1f} MB"
print("# Resumen (medianas)\n")
for mode in sorted({r['modo'] for r in rows}):
    print(f"## Modo {mode}\n")
    print("| métrica | Flutter | GPUI | GPUI/Flutter |\n|---|---|---|---|")
    for key, name, fmt in [('arranque_ms','arranque hasta 1er frame', lambda v: f"{v:.0f} ms"),
                           ('rss10_kb','RSS a 10 s', mb), ('pss10_kb','PSS a 10 s', mb),
                           ('cpu_reposo_pct','CPU reposo (10→20 s)', lambda v: f"{v:.1f} %"),
                           ('rss_uso_kb','RSS tras 30 s de uso', mb), ('pss_uso_kb','PSS tras 30 s de uso', mb)]:
        f, g = med('flutter', mode, key), med('gpui', mode, key)
        ratio = f"{g/f*100:.0f} %" if f and g is not None else '—'
        show = lambda v: '—' if v is None else fmt(v)
        print(f"| {name} | {show(f)} | {show(g)} | {ratio} |")
    t = {a: sorted({r['bandeja'] for r in rows if r['app']==a and r['modo']==mode}) for a in ('flutter','gpui')}
    print(f"| bandeja (menú + Salir) | {'/'.join(t['flutter'])} | {'/'.join(t['gpui'])} | |\n")
s = {r['app']: r for r in csv.DictReader(open(sys.argv[2]))}
print("## Tamaño\n\n| | Flutter (bundle) | GPUI (binario) |\n|---|---|---|")
for k, n in [('tal_cual_b','tal cual'),('strip_b','con strip'),('tar_zst_b','tar.zst -19')]:
    print(f"| {n} | {int(s['flutter'][k])/1e6:.1f} MB | {int(s['gpui'][k])/1e6:.1f} MB |")
PY
cat "$out/resumen.md"
echo "resultados en $out"
