# Paridad Flutter → Slint

Umbral: SSIM ≥ 0.97, cajas ±2.0 px (textos y botones `-btn`: x, y, alto; botones Jugar: borde derecho, y, alto).

| escena | SSIM | cajas | píxeles ΔE>10 | estado |
|---|---|---|---|---|
| content_curseforge_en | 0.9791 | 16/16 | 1.92 % | ok |
| content_search_es | 0.9733 | 33/33 | 3.39 % | ok |
| content_target_es | 0.9707 | 34/34 | 3.57 % | ok |
| modpack_installing_es | 0.9702 | 7/7 | 2.81 % | ok |
| project_sheet_es | 0.9726 | 16/16 | 2.73 % | ok |
| server_dialog_en | 0.9712 | 9/9 | 2.58 % | ok |
| servers_es | 0.9769 | 17/17 | 2.42 % | ok |

## content_curseforge_en: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |

## content_search_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |

## content_target_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |

## modpack_installing_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| card-0 | sobra | — | (108.0, 231.0, 564.0, 148.0) | — |
| card-0-icon | sobra | — | (124.0, 247.0, 64.0, 64.0) | — |
| card-0-play | sobra | — | (559.0, 315.0, 97.0, 48.0) | — |
| card-1 | sobra | — | (684.0, 231.0, 564.0, 148.0) | — |
| card-1-icon | sobra | — | (700.0, 247.0, 64.0, 64.0) | — |
| card-1-play | sobra | — | (1135.0, 315.0, 97.0, 48.0) | — |
| card-2 | sobra | — | (108.0, 391.0, 564.0, 148.0) | — |
| card-2-icon | sobra | — | (124.0, 407.0, 64.0, 64.0) | — |
| card-2-play | sobra | — | (559.0, 475.0, 97.0, 48.0) | — |
| card-3 | sobra | — | (684.0, 391.0, 564.0, 148.0) | — |
| card-3-icon | sobra | — | (700.0, 407.0, 64.0, 64.0) | — |
| card-3-play | sobra | — | (1135.0, 475.0, 97.0, 48.0) | — |
| card-4 | sobra | — | (108.0, 551.0, 564.0, 148.0) | — |
| card-4-icon | sobra | — | (124.0, 567.0, 64.0, 64.0) | — |
| card-4-play | sobra | — | (559.0, 635.0, 97.0, 48.0) | — |
| card-5 | sobra | — | (684.0, 551.0, 564.0, 148.0) | — |
| card-5-icon | sobra | — | (700.0, 567.0, 64.0, 64.0) | — |
| card-5-play | sobra | — | (1135.0, 635.0, 97.0, 48.0) | — |
| filter-0 | sobra | — | (696.0, 119.0, 198.0, 48.0) | — |
| filter-1 | sobra | — | (902.0, 119.0, 195.0, 48.0) | — |
| filter-2 | sobra | — | (1105.0, 119.0, 143.0, 48.0) | — |
| import | sobra | — | (1069.0, 55.0, 179.0, 48.0) | — |
| kind-0 | sobra | — | (108.0, 179.0, 88.5, 36.0) | — |
| kind-1 | sobra | — | (204.5, 179.0, 192.5, 36.0) | — |
| kind-2 | sobra | — | (405.0, 179.0, 105.5, 36.0) | — |
| kind-3 | sobra | — | (518.5, 179.0, 120.5, 36.0) | — |
| kind-4 | sobra | — | (647.0, 179.0, 119.5, 36.0) | — |
| search | sobra | — | (108.0, 119.0, 576.0, 48.0) | — |
| source | sobra | — | (875.0, 63.0, 182.0, 40.0) | — |
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |
| txt-results | sobra | — | (1177.0, 188.5, 71.0, 17.0) | — |
| txt-subtitle | sobra | — | (108.0, 83.0, 401.0, 20.0) | — |
| txt-title | sobra | — | (108.0, 48.0, 128.0, 31.0) | — |

## project_sheet_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| card-0 | sobra | — | (108.0, 313.0, 564.0, 148.0) | — |
| card-0-icon | sobra | — | (124.0, 329.0, 64.0, 64.0) | — |
| card-0-play | sobra | — | (570.0, 422.0, 86.0, 23.0) | — |
| card-1 | sobra | — | (684.0, 313.0, 564.0, 148.0) | — |
| card-1-icon | sobra | — | (700.0, 329.0, 64.0, 64.0) | — |
| card-1-play | sobra | — | (1146.0, 422.0, 86.0, 23.0) | — |
| card-2 | sobra | — | (108.0, 473.0, 564.0, 148.0) | — |
| card-2-icon | sobra | — | (124.0, 489.0, 64.0, 64.0) | — |
| card-2-play | sobra | — | (570.0, 582.0, 86.0, 23.0) | — |
| card-3 | sobra | — | (684.0, 473.0, 564.0, 148.0) | — |
| card-3-icon | sobra | — | (700.0, 489.0, 64.0, 64.0) | — |
| card-3-play | sobra | — | (1135.0, 557.0, 97.0, 48.0) | — |
| card-4 | sobra | — | (108.0, 633.0, 564.0, 148.0) | — |
| card-4-icon | sobra | — | (124.0, 649.0, 64.0, 64.0) | — |
| card-4-play | sobra | — | (559.0, 717.0, 97.0, 48.0) | — |
| card-5 | sobra | — | (684.0, 633.0, 564.0, 148.0) | — |
| card-5-icon | sobra | — | (700.0, 649.0, 64.0, 64.0) | — |
| card-5-play | sobra | — | (1135.0, 717.0, 97.0, 48.0) | — |
| filter-0 | sobra | — | (696.0, 201.0, 198.0, 48.0) | — |
| filter-1 | sobra | — | (902.0, 201.0, 195.0, 48.0) | — |
| filter-2 | sobra | — | (1105.0, 201.0, 143.0, 48.0) | — |
| import | sobra | — | (1069.0, 55.0, 179.0, 48.0) | — |
| kind-0 | sobra | — | (108.0, 261.0, 88.5, 36.0) | — |
| kind-1 | sobra | — | (204.5, 261.0, 192.5, 36.0) | — |
| kind-2 | sobra | — | (405.0, 261.0, 105.5, 36.0) | — |
| kind-3 | sobra | — | (518.5, 261.0, 120.5, 36.0) | — |
| kind-4 | sobra | — | (647.0, 261.0, 119.5, 36.0) | — |
| search | sobra | — | (108.0, 201.0, 576.0, 48.0) | — |
| source | sobra | — | (875.0, 63.0, 182.0, 40.0) | — |
| target | sobra | — | (108.0, 119.0, 1140.0, 66.0) | — |
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |
| txt-results | sobra | — | (1177.0, 270.5, 71.0, 17.0) | — |
| txt-subtitle | sobra | — | (108.0, 83.0, 401.0, 20.0) | — |
| txt-title | sobra | — | (108.0, 48.0, 128.0, 31.0) | — |

## server_dialog_en: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| add | sobra | — | (1116.0, 55.0, 132.0, 48.0) | — |
| import | sobra | — | (1003.0, 55.0, 101.0, 48.0) | — |
| server-0 | sobra | — | (108.0, 127.0, 1140.0, 84.0) | — |
| server-0-icon | sobra | — | (149.0, 141.0, 56.0, 56.0) | — |
| server-0-play | sobra | — | (1097.0, 145.0, 86.0, 48.0) | — |
| server-1 | sobra | — | (108.0, 223.0, 1140.0, 84.0) | — |
| server-1-icon | sobra | — | (149.0, 237.0, 56.0, 56.0) | — |
| server-1-play | sobra | — | (1097.0, 241.0, 86.0, 48.0) | — |
| server-2 | sobra | — | (108.0, 319.0, 1140.0, 84.0) | — |
| server-2-icon | sobra | — | (149.0, 333.0, 56.0, 56.0) | — |
| server-2-play | sobra | — | (1097.0, 337.0, 86.0, 48.0) | — |
| server-3 | sobra | — | (108.0, 415.0, 1140.0, 84.0) | — |
| server-3-icon | sobra | — | (149.0, 429.0, 56.0, 56.0) | — |
| server-3-play | sobra | — | (1097.0, 433.0, 86.0, 48.0) | — |
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |
| txt-subtitle | sobra | — | (108.0, 83.0, 123.0, 20.0) | — |
| txt-title | sobra | — | (108.0, 48.0, 96.0, 31.0) | — |

## servers_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |
