#!/usr/bin/env bash
# Mide Flutter release vs Slint release en la pantalla «Instancias» con los
# mismos datos (5 instancias sembradas con `shard-ui --seed`, que usa lcore).
#
#   tools/medir.sh [--runs 3] [--flutter DIR_BUNDLE] [--slint BIN] [--apps "flutter slint"]
#                  [--modes "anim quieto"] [--out DIR] [--no-usage] [--wayland]
#
# Por app, modo y repetición:
#   - arranque: del exec al primer fotograma (marcador SHARD_FIRST_FRAME que
#     imprime la propia app; columna arranque_ms) y del exec a la ventana
#     visible según X11 (xdotool; columna ventana_ms, misma vara para todas).
#     Con el renderer por software Slint no avisa del primer fotograma pintado:
#     su marcador sale en la primera vuelta del bucle, así que ventana_ms es
#     la cifra comparable;
#   - RSS y PSS (/proc/PID/smaps_rollup) a los 10 s;
#   - CPU en reposo: % de un núcleo entre los segundos 10 y 20;
#   - 30 s de uso con xdotool (hover por las tarjetas, rueda, escribir en el
#     buscador, cerrar a la bandeja y reabrir desde su menú) y RSS/PSS 5 s
#     después;
#   - «Salir» desde el menú de la bandeja termina el proceso.
# Modos: «anim» = fondo animado; «quieto» = `performance: true` en ui.json.
#
# El marcador de Flutter solo existe en la rama spike/gpui (parche de
# medición, SHARD_START_ROUTE + SHARD_SPIKE_TIMING); con un Flutter de main
# se mide hasta ventana visible y arranca en Inicio, no en Instancias.
# Slint: SHARD_TIMING=1 (siempre disponible). Renderer de Slint: el de por
# defecto (software); SLINT_BACKEND=winit-femtovg para probar OpenGL.
#
# Requisitos: bash, python3, busctl y gdbus (D-Bus de sesión); para la fase
# de uso y las capturas, X11/Xwayland con xdotool e `import` (ImageMagick).
# Bandeja: el script la maneja por D-Bus aunque no haya panel.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
runs=3
flutter="$root/app/build/linux/x64/release/bundle"
slint="$root/target/release-ui/shard-ui"
apps="flutter slint"
modes="anim quieto"
out="$PWD/medicion-$(date +%Y%m%d-%H%M%S)"
usage=1
backend=x11
while [ $# -gt 0 ]; do
  case $1 in
    --runs) runs=$2; shift 2 ;;
    --flutter) flutter=$2; shift 2 ;;
    --slint) slint=$2; shift 2 ;;
    --apps) apps=$2; shift 2 ;;
    --modes) modes=$2; shift 2 ;;
    --out) out=$2; shift 2 ;;
    --no-usage) usage=0; shift ;;
    --wayland) backend=wayland; usage=0; shift ;;
    *) echo "argumento desconocido: $1" >&2; exit 2 ;;
  esac
done
[ -x "$slint" ] || { echo "falta $slint (cargo build -p shard-ui --profile release-ui)" >&2; exit 2; }
case " $apps " in *" flutter "*) [ -x "$flutter/shard" ] || { echo "falta $flutter/shard (flutter build linux --release)" >&2; exit 2; } ;; esac
mkdir -p "$out"
tray="$root/tools/slint/tray.sh"
hz=$(getconf CLK_TCK)
csv="$out/runs.csv"
echo "app,modo,run,ventana_ms,arranque_ms,rss10_kb,pss10_kb,cpu_reposo_pct,rss_uso_kb,pss_uso_kb,bandeja" > "$csv"
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
  echo "- backend de las apps: $backend; Slint: ${SLINT_BACKEND:-software (por defecto)}"
  echo "- sesión: XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-?} XDG_CURRENT_DESKTOP=${XDG_CURRENT_DESKTOP:-?} WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-} DISPLAY=${DISPLAY:-}"
  echo "- Slint: $slint ($(git -C "$root" rev-parse --short HEAD 2>/dev/null || echo '?'))"
  echo "- Flutter: $flutter"
} > "$out/equipo.md"

seed() { # dir modo
  "$slint" --seed "$1" >/dev/null
  if [ "$2" = quieto ]; then echo '{"firstRunDone":true,"performance":true}' > "$1/ui.json"; fi
}

launch() { # app datos log
  local env=(env)
  [ "$backend" = x11 ] && env+=(-u WAYLAND_DISPLAY)
  env+=(GDK_BACKEND="$backend")
  case $1 in
    flutter) "${env[@]}" SHARD_DATA_DIR="$2" SHARD_START_ROUTE=/instances SHARD_SPIKE_TIMING=1 \
               "$flutter/shard" >"$3" 2>&1 & ;;
    slint) "${env[@]}" SHARD_TIMING=1 "$slint" --data-dir "$2" >"$3" 2>&1 & ;;
  esac
  echo $!
}

use_for_30s() { # pid
  local pid=$1 end=$(( $(date +%s) + 30 )) w i
  while [ "$(date +%s)" -lt "$end" ]; do
    w=$(window); [ -n "$w" ] || { sleep 0.5; continue; }
    for i in 0 1 2 3; do xdotool mousemove --window "$w" $((244 + i * 289)) 240; sleep 0.25; done
    xdotool mousemove --window "$w" 244 490; sleep 0.25
    xdotool mousemove --window "$w" 700 450 click --repeat 4 --delay 60 5 click --repeat 4 --delay 60 4
    xdotool mousemove --window "$w" 740 79 click 1; sleep 0.2
    xdotool type --delay 40 sur; sleep 0.3
    xdotool key ctrl+a BackSpace; sleep 0.2
    xdotool mousemove --window "$w" 1253 22 click 1; sleep 1
    if [ -n "$(window)" ]; then echo "AVISO: la ventana sigue visible tras cerrar ($pid)" >&2; fi
    "$tray" "$pid" menu "Abrir Shard" || true; sleep 1.5
  done
}

for mode in $modes; do
  for run in $(seq 1 "$runs"); do
    for app in $apps; do
      data=$(mktemp -d); seed "$data" "$mode"
      log="$out/$app-$mode-$run.log"
      t0=$(now_ms); pid=$(launch "$app" "$data" "$log")
      start=""; shown=""
      for _ in $(seq 1 500); do
        if [ -z "$shown" ] && [ "$backend" = x11 ] && [ -n "$(window)" ]; then shown=$(( $(now_ms) - t0 )); fi
        if [ -z "$start" ]; then
          ff=$(grep -m1 -oP 'SHARD_FIRST_FRAME unix_ms=\K\d+' "$log" || true)
          [ -n "$ff" ] && start=$((ff - t0))
        fi
        { [ -n "$start" ] && { [ -n "$shown" ] || [ "$backend" != x11 ]; }; } && break
        sleep 0.01
      done
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
      # «Salir» puede cerrar el proceso antes de responder a D-Bus (Slint):
      # cuenta que el proceso termine, no la respuesta.
      tray_ok=no
      if "$tray" "$pid" labels 2>/dev/null | grep -q "Abrir Shard"; then
        "$tray" "$pid" menu Salir 2>/dev/null || true
        for _ in $(seq 1 50); do kill -0 "$pid" 2>/dev/null || { tray_ok=si; break; }; sleep 0.1; done
      fi
      kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
      echo "$app,$mode,$run,$shown,$start,$m10,$cpu,$muse,$tray_ok" | tee -a "$csv"
      rm -rf "$data"; sleep 2
    done
  done
done

{
  echo "app,tal_cual_b,tar_zst_b"
  for app in $apps; do
    case $app in
      flutter) b=$(du -sb "$flutter" | cut -f1); z=$(tar -C "$flutter/.." -cf - "$(basename "$flutter")" | zstd -19 -q -c | wc -c) ;;
      slint) b=$(stat -c %s "$slint"); z=$(zstd -19 -q -c "$slint" | wc -c) ;;
    esac
    echo "$app,$b,$z"
  done
} > "$out/tamanos.csv"

python3 - "$csv" "$out/tamanos.csv" "$apps" > "$out/resumen.md" <<'PY'
import csv, statistics, sys
rows = list(csv.DictReader(open(sys.argv[1])))
apps = sys.argv[3].split()
def med(app, mode, key):
    v = [float(r[key]) for r in rows if r['app'] == app and r['modo'] == mode and r[key] not in ('', None)]
    return statistics.median(v) if v else None
mb = lambda kb: f"{kb/1024:.1f} MB"
print("# Resumen (medianas)\n")
for mode in sorted({r['modo'] for r in rows}):
    print(f"## Modo {mode}\n")
    head = " | ".join(apps)
    ratio = len(apps) == 2
    print(f"| métrica | {head} |" + (" Slint/Flutter |" if ratio else ""))
    print("|---" * (len(apps) + 1 + ratio) + "|")
    for key, name, fmt in [('ventana_ms', 'arranque hasta ventana visible (X11)', lambda v: f"{v:.0f} ms"),
                           ('arranque_ms', 'arranque según la app (marcador)', lambda v: f"{v:.0f} ms"),
                           ('rss10_kb', 'RSS a 10 s', mb), ('pss10_kb', 'PSS a 10 s', mb),
                           ('cpu_reposo_pct', 'CPU reposo (10→20 s)', lambda v: f"{v:.1f} %"),
                           ('rss_uso_kb', 'RSS tras 30 s de uso', mb), ('pss_uso_kb', 'PSS tras 30 s de uso', mb)]:
        vals = [med(a, mode, key) for a in apps]
        cells = ['—' if v is None else fmt(v) for v in vals]
        r = ''
        if ratio:
            f, s = vals
            r = f" {s / f * 100:.0f} % |" if f and s is not None else " — |"
        print(f"| {name} | " + " | ".join(cells) + " |" + r)
    trays = {a: '/'.join(sorted({r['bandeja'] for r in rows if r['app'] == a and r['modo'] == mode})) for a in apps}
    print("| bandeja (menú + Salir) | " + " | ".join(trays[a] for a in apps) + " |" + (" |" if ratio else "") + "\n")
s = {r['app']: r for r in csv.DictReader(open(sys.argv[2]))}
print("## Tamaño\n\n| | " + " | ".join(apps) + " |\n" + "|---" * (len(apps) + 1) + "|")
for k, n in [('tal_cual_b', 'tal cual'), ('tar_zst_b', 'tar.zst -19')]:
    print(f"| {n} | " + " | ".join(f"{int(s[a][k]) / 1e6:.1f} MB" for a in apps) + " |")
PY
cat "$out/resumen.md"
echo "resultados en $out"
