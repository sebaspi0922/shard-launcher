# Resumen (medianas)

## Modo anim

| métrica | Flutter | GPUI | GPUI/Flutter |
|---|---|---|---|
| arranque hasta 1er frame | 631 ms | 243 ms | 39 % |
| RSS a 10 s | 354.5 MB | 177.5 MB | 50 % |
| PSS a 10 s | 307.0 MB | 141.6 MB | 46 % |
| CPU reposo (10→20 s) | 212.7 % | 108.8 % | 51 % |
| RSS tras 30 s de uso | 358.4 MB | 194.9 MB | 54 % |
| PSS tras 30 s de uso | 312.5 MB | 159.0 MB | 51 % |
| bandeja (menú + Salir) | si | si | |

## Modo quieto

| métrica | Flutter | GPUI | GPUI/Flutter |
|---|---|---|---|
| arranque hasta 1er frame | 596 ms | 235 ms | 39 % |
| RSS a 10 s | 353.9 MB | 169.0 MB | 48 % |
| PSS a 10 s | 308.0 MB | 133.0 MB | 43 % |
| CPU reposo (10→20 s) | 0.2 % | 0.3 % | 150 % |
| RSS tras 30 s de uso | 354.0 MB | 187.2 MB | 53 % |
| PSS tras 30 s de uso | 308.0 MB | 151.1 MB | 49 % |
| bandeja (menú + Salir) | si | si | |

## Tamaño

| | Flutter (bundle) | GPUI (binario) |
|---|---|---|
| tal cual | 65.7 MB | 46.3 MB |
| con strip | 55.3 MB | 46.3 MB |
| tar.zst -19 | 16.5 MB | 13.6 MB |
