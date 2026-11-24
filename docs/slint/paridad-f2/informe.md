# Paridad Flutter → Slint

Umbral: SSIM ≥ 0.97, cajas ±2.0 px (textos: x, y, alto; botones Jugar: borde derecho, y, alto).

| escena | SSIM | cajas | píxeles ΔE>10 | estado |
|---|---|---|---|---|
| accounts_panel_es | 0.9764 | 8/8 | 1.69 % | ok |
| instances_empty_es | 0.9806 | 9/9 | 1.68 % | ok |
| instances_grid_es | 0.9781 | 24/24 | 1.68 % | ok |
| instances_list_en | 0.9798 | 23/23 | 1.75 % | ok |
| instances_noresults_es | 0.9908 | 8/8 | 0.85 % | ok |
| instances_search_es | 0.9885 | 11/11 | 0.94 % | ok |
| ms_login_es | 0.9706 | 8/8 | 1.88 % | ok |
| offline_account_en | 0.9745 | 6/6 | 1.7 % | ok |
| shard_account_es | 0.9734 | 7/7 | 1.79 % | ok |
| skins_es | 0.9746 | 17/17 | 1.6 % | ok |
| skins_offline_en | 0.9768 | 5/5 | 1.92 % | ok |

## accounts_panel_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| card-0 | sobra | — | (108.0, 127.0, 273.0, 232.0) | — |
| card-0-icon | sobra | — | (125.0, 208.0, 48.0, 48.0) | — |
| card-0-play | sobra | — | (272.0, 140.0, 96.0, 34.0) | — |
| card-1 | sobra | — | (397.0, 127.0, 273.0, 232.0) | — |
| card-1-icon | sobra | — | (414.0, 208.0, 48.0, 48.0) | — |
| card-1-play | sobra | — | (561.0, 140.0, 96.0, 34.0) | — |
| card-2 | sobra | — | (686.0, 127.0, 273.0, 232.0) | — |
| card-2-icon | sobra | — | (703.0, 208.0, 48.0, 48.0) | — |
| card-2-play | sobra | — | (850.0, 140.0, 96.0, 34.0) | — |
| card-3 | sobra | — | (975.0, 127.0, 273.0, 232.0) | — |
| card-3-icon | sobra | — | (992.0, 208.0, 48.0, 48.0) | — |
| card-3-play | sobra | — | (1139.0, 140.0, 96.0, 34.0) | — |
| card-4 | sobra | — | (108.0, 375.0, 273.0, 232.0) | — |
| card-4-icon | sobra | — | (125.0, 456.0, 48.0, 48.0) | — |
| card-4-play | sobra | — | (272.0, 388.0, 96.0, 34.0) | — |
| new | sobra | — | (1083.0, 55.0, 165.0, 48.0) | — |
| search | sobra | — | (587.0, 55.0, 300.0, 48.0) | — |
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |
| trash | sobra | — | (1023.0, 55.0, 48.0, 48.0) | — |
| txt-count | sobra | — | (108.0, 83.0, 76.0, 20.0) | — |
| txt-title | sobra | — | (108.0, 48.0, 126.0, 31.0) | — |
| view-toggle | sobra | — | (899.0, 63.0, 112.0, 40.0) | — |

## instances_empty_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| empty-action | sobra | — | (595.5, 518.5, 165.0, 48.0) | — |

## ms_login_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| card-0 | sobra | — | (108.0, 127.0, 273.0, 232.0) | — |
| card-0-icon | sobra | — | (125.0, 208.0, 48.0, 48.0) | — |
| card-0-play | sobra | — | (272.0, 140.0, 96.0, 34.0) | — |
| card-1 | sobra | — | (397.0, 127.0, 273.0, 232.0) | — |
| card-1-icon | sobra | — | (414.0, 208.0, 48.0, 48.0) | — |
| card-1-play | sobra | — | (561.0, 140.0, 96.0, 34.0) | — |
| card-2 | sobra | — | (686.0, 127.0, 273.0, 232.0) | — |
| card-2-icon | sobra | — | (703.0, 208.0, 48.0, 48.0) | — |
| card-2-play | sobra | — | (850.0, 140.0, 96.0, 34.0) | — |
| card-3 | sobra | — | (975.0, 127.0, 273.0, 232.0) | — |
| card-3-icon | sobra | — | (992.0, 208.0, 48.0, 48.0) | — |
| card-3-play | sobra | — | (1139.0, 140.0, 96.0, 34.0) | — |
| card-4 | sobra | — | (108.0, 375.0, 273.0, 232.0) | — |
| card-4-icon | sobra | — | (125.0, 456.0, 48.0, 48.0) | — |
| card-4-play | sobra | — | (272.0, 388.0, 96.0, 34.0) | — |
| new | sobra | — | (1083.0, 55.0, 165.0, 48.0) | — |
| search | sobra | — | (587.0, 55.0, 300.0, 48.0) | — |
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |
| trash | sobra | — | (1023.0, 55.0, 48.0, 48.0) | — |
| txt-count | sobra | — | (108.0, 83.0, 76.0, 20.0) | — |
| txt-title | sobra | — | (108.0, 48.0, 126.0, 31.0) | — |
| view-toggle | sobra | — | (899.0, 63.0, 112.0, 40.0) | — |

## offline_account_en: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| card-0 | sobra | — | (108.0, 127.0, 273.0, 232.0) | — |
| card-0-icon | sobra | — | (125.0, 208.0, 48.0, 48.0) | — |
| card-0-play | sobra | — | (272.0, 140.0, 96.0, 34.0) | — |
| card-1 | sobra | — | (397.0, 127.0, 273.0, 232.0) | — |
| card-1-icon | sobra | — | (414.0, 208.0, 48.0, 48.0) | — |
| card-1-play | sobra | — | (561.0, 140.0, 96.0, 34.0) | — |
| card-2 | sobra | — | (686.0, 127.0, 273.0, 232.0) | — |
| card-2-icon | sobra | — | (703.0, 208.0, 48.0, 48.0) | — |
| card-2-play | sobra | — | (850.0, 140.0, 96.0, 34.0) | — |
| card-3 | sobra | — | (975.0, 127.0, 273.0, 232.0) | — |
| card-3-icon | sobra | — | (992.0, 208.0, 48.0, 48.0) | — |
| card-3-play | sobra | — | (1139.0, 140.0, 96.0, 34.0) | — |
| card-4 | sobra | — | (108.0, 375.0, 273.0, 232.0) | — |
| card-4-icon | sobra | — | (125.0, 456.0, 48.0, 48.0) | — |
| card-4-play | sobra | — | (272.0, 388.0, 96.0, 34.0) | — |
| new | sobra | — | (1099.0, 55.0, 149.0, 48.0) | — |
| search | sobra | — | (603.0, 55.0, 300.0, 48.0) | — |
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |
| trash | sobra | — | (1039.0, 55.0, 48.0, 48.0) | — |
| txt-count | sobra | — | (108.0, 83.0, 73.0, 20.0) | — |
| txt-title | sobra | — | (108.0, 48.0, 120.0, 31.0) | — |
| view-toggle | sobra | — | (915.0, 63.0, 112.0, 40.0) | — |

## shard_account_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| card-0 | sobra | — | (108.0, 127.0, 273.0, 232.0) | — |
| card-0-icon | sobra | — | (125.0, 208.0, 48.0, 48.0) | — |
| card-0-play | sobra | — | (272.0, 140.0, 96.0, 34.0) | — |
| card-1 | sobra | — | (397.0, 127.0, 273.0, 232.0) | — |
| card-1-icon | sobra | — | (414.0, 208.0, 48.0, 48.0) | — |
| card-1-play | sobra | — | (561.0, 140.0, 96.0, 34.0) | — |
| card-2 | sobra | — | (686.0, 127.0, 273.0, 232.0) | — |
| card-2-icon | sobra | — | (703.0, 208.0, 48.0, 48.0) | — |
| card-2-play | sobra | — | (850.0, 140.0, 96.0, 34.0) | — |
| card-3 | sobra | — | (975.0, 127.0, 273.0, 232.0) | — |
| card-3-icon | sobra | — | (992.0, 208.0, 48.0, 48.0) | — |
| card-3-play | sobra | — | (1139.0, 140.0, 96.0, 34.0) | — |
| card-4 | sobra | — | (108.0, 375.0, 273.0, 232.0) | — |
| card-4-icon | sobra | — | (125.0, 456.0, 48.0, 48.0) | — |
| card-4-play | sobra | — | (272.0, 388.0, 96.0, 34.0) | — |
| new | sobra | — | (1083.0, 55.0, 165.0, 48.0) | — |
| search | sobra | — | (587.0, 55.0, 300.0, 48.0) | — |
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |
| trash | sobra | — | (1023.0, 55.0, 48.0, 48.0) | — |
| txt-count | sobra | — | (108.0, 83.0, 76.0, 20.0) | — |
| txt-title | sobra | — | (108.0, 48.0, 126.0, 31.0) | — |
| view-toggle | sobra | — | (899.0, 63.0, 112.0, 40.0) | — |

## skins_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |

## skins_offline_en: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |
