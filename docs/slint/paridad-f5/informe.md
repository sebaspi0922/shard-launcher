# Paridad Flutter → Slint

Umbral: SSIM ≥ 0.97, cajas ±2.0 px (textos y botones `-btn`: x, y, alto; botones Jugar: borde derecho, y, alto).

| escena | SSIM | cajas | píxeles ΔE>10 | estado |
|---|---|---|---|---|
| accounts_panel_es | 0.9873 | 8/8 | 1.39 % | ok |
| crash_banner_es | 0.9793 | 20/20 | 2.24 % | ok |
| doctor_es | 0.9703 | 8/8 | 2.47 % | ok |
| first_run_account_en | 0.9872 | 14/14 | 1.39 % | ok |
| first_run_done_es | 0.9896 | 12/12 | 0.99 % | ok |
| first_run_import_es | 0.9757 | 13/13 | 2.04 % | ok |
| first_run_welcome_es | 0.9728 | 15/15 | 1.96 % | ok |
| home_downloading_es | 0.9760 | 20/20 | 2.39 % | ok |
| home_empty_en | 0.9740 | 8/8 | 1.88 % | ok |
| home_error_es | 0.9786 | 20/20 | 2.26 % | ok |
| home_es | 0.9799 | 19/19 | 2.04 % | ok |
| home_style_aurora_es | 0.9796 | 19/19 | 1.99 % | ok |
| home_style_graphite_es | 0.9797 | 19/19 | 2.07 % | ok |
| home_style_landscape_es | 0.9772 | 19/19 | 2.18 % | ok |
| home_style_sebaspi_es | 0.9742 | 19/19 | 2.43 % | ok |
| instances_empty_es | 0.9884 | 9/9 | 1.19 % | ok |
| instances_grid_es | 0.9892 | 24/24 | 1.38 % | ok |
| instances_list_en | 0.9825 | 23/23 | 1.73 % | ok |
| instances_noresults_es | 0.9921 | 8/8 | 0.81 % | ok |
| instances_search_es | 0.9917 | 11/11 | 0.85 % | ok |
| ms_login_es | 0.9816 | 8/8 | 1.55 % | ok |
| offline_account_en | 0.9816 | 6/6 | 1.45 % | ok |
| settings_appearance_es | 0.9805 | 23/23 | 2.11 % | ok |
| settings_es | 0.9790 | 16/16 | 2.09 % | ok |
| settings_integrations_es | 0.9737 | 10/10 | 2.32 % | ok |
| settings_java_es | 0.9700 | 19/19 | 2.49 % | ok |
| settings_update_en | 0.9761 | 10/10 | 2.2 % | ok |
| shard_account_es | 0.9789 | 7/7 | 1.57 % | ok |
| skins_es | 0.9851 | 17/17 | 1.3 % | ok |
| skins_offline_en | 0.9789 | 5/5 | 1.57 % | ok |
| update_banner_es | 0.9790 | 20/20 | 2.25 % | ok |
| web_home_graphite_es | 0.9796 | 19/19 | 2.2 % | ok |

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
| panel-0 | sobra | — | (84.0, 475.0, 340.0, 313.0) | — |
| search | sobra | — | (587.0, 55.0, 300.0, 48.0) | — |
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |
| trash | sobra | — | (1023.0, 55.0, 48.0, 48.0) | — |
| txt-count | sobra | — | (108.0, 83.0, 76.0, 20.0) | — |
| txt-title | sobra | — | (108.0, 48.0, 126.0, 31.0) | — |
| view-toggle | sobra | — | (899.0, 63.0, 112.0, 40.0) | — |

## doctor_es: cajas fuera de tolerancia

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

## first_run_import_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| source | sobra | — | (431.0, 231.0, 658.0, 260.0) | — |

## home_empty_en: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| empty-action | sobra | — | (443.5, 513.0, 149.0, 48.0) | — |

## instances_empty_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| empty-action | sobra | — | (595.5, 517.5, 165.0, 48.0) | — |

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

## settings_appearance_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| panel | sobra | — | (248.0, -169.0, 860.0, 293.0) | — |
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |

## settings_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| panel | sobra | — | (248.0, 131.0, 860.0, 293.0) | — |
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |

## settings_integrations_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| panel | sobra | — | (248.0, 63.6, 860.0, 90.0) | — |
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |

## settings_java_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| panel | sobra | — | (248.0, -428.0, 860.0, 681.5) | — |
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |

## settings_update_en: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| panel | sobra | — | (248.0, 14.6, 860.0, 90.0) | — |
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |

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
| source-0 | sobra | — | (765.0, 180.0, 462.0, 40.0) | — |
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |

## skins_offline_en: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |
