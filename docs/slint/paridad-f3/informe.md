# Paridad Flutter → Slint

Umbral: SSIM ≥ 0.97, cajas ±2.0 px (textos y botones `-btn`: x, y, alto; botones Jugar: borde derecho, y, alto).

| escena | SSIM | cajas | píxeles ΔE>10 | estado |
|---|---|---|---|---|
| accounts_panel_es | 0.9873 | 8/8 | 1.39 % | ok |
| backup_restore_es | 0.9642 | 24/24 | 3.22 % | **falla** |
| change_version_es | 0.9792 | 40/40 | 1.81 % | ok |
| content_curseforge_en | 0.9791 | 16/16 | 1.92 % | ok |
| content_search_es | 0.9733 | 33/33 | 3.39 % | ok |
| content_target_es | 0.9707 | 34/34 | 3.57 % | ok |
| crash_banner_es | 0.9793 | 20/20 | 2.24 % | ok |
| create_instance_es | 0.9675 | 22/22 | 2.92 % | **falla** |
| home_downloading_es | 0.9760 | 20/20 | 2.39 % | ok |
| home_empty_en | 0.9739 | 8/8 | 1.84 % | ok |
| home_error_es | 0.9786 | 20/20 | 2.26 % | ok |
| home_es | 0.9799 | 19/19 | 2.04 % | ok |
| home_style_aurora_es | 0.9796 | 19/19 | 1.99 % | ok |
| home_style_graphite_es | 0.9797 | 19/19 | 2.07 % | ok |
| home_style_landscape_es | 0.9772 | 19/19 | 2.18 % | ok |
| home_style_sebaspi_es | 0.9742 | 19/19 | 2.43 % | ok |
| instance_backups_es | 0.9723 | 44/44 | 2.59 % | ok |
| instance_content_es | 0.9773 | 54/54 | 2.4 % | ok |
| instance_logs_es | 0.9745 | 29/29 | 3.98 % | ok |
| instance_menu_es | 0.9718 | 16/16 | 2.56 % | ok |
| instance_screenshots_es | 0.9780 | 26/26 | 1.89 % | ok |
| instance_settings_es | 0.9727 | 28/28 | 2.19 % | ok |
| instance_worlds_es | 0.9758 | 40/40 | 2.32 % | ok |
| instances_empty_es | 0.9884 | 9/9 | 1.19 % | ok |
| instances_grid_es | 0.9892 | 24/24 | 1.38 % | ok |
| instances_list_en | 0.9825 | 23/23 | 1.73 % | ok |
| instances_noresults_es | 0.9920 | 8/8 | 0.82 % | ok |
| instances_search_es | 0.9918 | 11/11 | 0.85 % | ok |
| modpack_installing_es | 0.9702 | 7/7 | 2.81 % | ok |
| ms_login_es | 0.9816 | 8/8 | 1.55 % | ok |
| offline_account_en | 0.9816 | 6/6 | 1.45 % | ok |
| project_sheet_es | 0.9726 | 16/16 | 2.73 % | ok |
| server_dialog_en | — | — | — | ERROR: falta la captura de Slint |
| servers_es | — | — | — | ERROR: falta la captura de Slint |
| shard_account_es | 0.9789 | 7/7 | 1.57 % | ok |
| skins_es | 0.9851 | 17/17 | 1.3 % | ok |
| skins_offline_en | 0.9850 | 5/5 | 1.38 % | ok |
| trash_en | 0.9740 | 18/18 | 2.15 % | ok |
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
| search | sobra | — | (587.0, 55.0, 300.0, 48.0) | — |
| titlebar | sobra | — | (76.0, 0.0, 1204.0, 44.0) | — |
| trash | sobra | — | (1023.0, 55.0, 48.0, 48.0) | — |
| txt-count | sobra | — | (108.0, 83.0, 76.0, 20.0) | — |
| txt-title | sobra | — | (108.0, 48.0, 126.0, 31.0) | — |
| view-toggle | sobra | — | (899.0, 63.0, 112.0, 40.0) | — |

## backup_restore_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| cancel | sobra | — | (656.0, 574.5, 83.0, 48.0) | — |

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

## create_instance_es: cajas fuera de tolerancia

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
| version-6 | sobra | — | (309.0, 583.5, 352.2, 36.0) | — |
| view-toggle | sobra | — | (899.0, 63.0, 112.0, 40.0) | — |

## home_empty_en: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| empty-action | sobra | — | (443.5, 513.0, 149.0, 48.0) | — |

## instance_content_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| filter-0 | sobra | — | (108.0, 295.0, 76.0, 36.0) | — |
| filter-1 | sobra | — | (192.0, 295.0, 80.0, 36.0) | — |
| filter-2 | sobra | — | (280.0, 295.0, 184.0, 36.0) | — |
| filter-3 | sobra | — | (472.0, 295.0, 97.0, 36.0) | — |

## instance_logs_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| chip | sobra | — | (380.0, 298.0, 91.0, 30.0) | — |
| filter-0 | sobra | — | (108.0, 289.0, 260.0, 48.0) | — |
| pill | sobra | — | (503.5, 8.0, 213.0, 28.0) | — |

## instances_empty_es: cajas fuera de tolerancia

| caja | estado | Flutter | Slint | Δ máx |
|---|---|---|---|---|
| empty-action | sobra | — | (595.5, 517.5, 165.0, 48.0) | — |

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

## trash_en: cajas fuera de tolerancia

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
