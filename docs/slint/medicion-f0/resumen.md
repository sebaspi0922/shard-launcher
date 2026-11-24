# Resumen (medianas)

## Modo anim

| métrica | flutter | slint | Slint/Flutter |
|---|---|---|---|
| arranque hasta ventana visible (X11) | 288 ms | 92 ms | 32 % |
| arranque según la app (marcador) | 330 ms | 69 ms | 21 % |
| RSS a 10 s | 299.1 MB | 37.9 MB | 13 % |
| PSS a 10 s | 283.6 MB | 32.4 MB | 11 % |
| CPU reposo (10→20 s) | 654.8 % | 31.7 % | 5 % |
| RSS tras 30 s de uso | 304.7 MB | 39.0 MB | 13 % |
| PSS tras 30 s de uso | 291.0 MB | 33.6 MB | 12 % |
| bandeja (menú + Salir) | si | si | |

## Modo quieto

| métrica | flutter | slint | Slint/Flutter |
|---|---|---|---|
| arranque hasta ventana visible (X11) | 282 ms | 60 ms | 21 % |
| arranque según la app (marcador) | 320 ms | 34 ms | 11 % |
| RSS a 10 s | 298.4 MB | 37.8 MB | 13 % |
| PSS a 10 s | 284.9 MB | 32.3 MB | 11 % |
| CPU reposo (10→20 s) | 0.5 % | 0.1 % | 20 % |
| RSS tras 30 s de uso | 301.7 MB | 39.0 MB | 13 % |
| PSS tras 30 s de uso | 288.1 MB | 33.5 MB | 12 % |
| bandeja (menú + Salir) | si | si | |

## Tamaño

| | flutter | slint |
|---|---|---|
| tal cual | 65.8 MB | 36.0 MB |
| tar.zst -19 | 17.3 MB | 12.0 MB |
