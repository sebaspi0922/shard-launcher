# Resumen (medianas)

## Modo anim

| métrica | Flutter | GPUI | GPUI/Flutter |
|---|---|---|---|
| arranque hasta 1er frame | 420 ms | 275 ms | 65 % |
| RSS a 10 s | 347.7 MB | 272.4 MB | 78 % |
| PSS a 10 s | 173.1 MB | 117.1 MB | 68 % |
| CPU reposo (10→20 s) | 47.0 % | 15.2 % | 32 % |
| RSS tras 30 s de uso | 351.2 MB | 292.3 MB | 83 % |
| PSS tras 30 s de uso | 177.4 MB | 138.3 MB | 78 % |
| bandeja (menú + Salir) | no | no/si | |

## Modo quieto

| métrica | Flutter | GPUI | GPUI/Flutter |
|---|---|---|---|
| arranque hasta 1er frame | 397 ms | 269 ms | 68 % |
| RSS a 10 s | 343.5 MB | 270.0 MB | 79 % |
| PSS a 10 s | 173.7 MB | 118.0 MB | 68 % |
| CPU reposo (10→20 s) | 0.0 % | 0.1 % | — |
| RSS tras 30 s de uso | 344.5 MB | 308.5 MB | 90 % |
| PSS tras 30 s de uso | 174.6 MB | 156.5 MB | 90 % |
| bandeja (menú + Salir) | si | si | |

## Tamaño

| | Flutter (bundle) | GPUI (binario) |
|---|---|---|
| tal cual | 65.7 MB | 46.3 MB |
| con strip | 55.3 MB | 46.3 MB |
| tar.zst -19 | 16.5 MB | 13.6 MB |
