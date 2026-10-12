// Versiones nuevas de ejemplo para las pruebas del actualizador.

import 'package:shard/core/bridge.dart';

const appImageUpdate = UpdateView(
  version: '0.2.0.r40',
  current: '0.1.0.r27',
  notes: 'Actualizador integrado y arreglos de la bandeja.',
  notesUrl: 'https://shardlauncher.com/novedades',
  canInstall: true,
  downloadUrl: 'https://dl.shardlauncher.com/linux/0.2.0.r40/Shard-0.2.0.r40-x86_64.AppImage',
);

const flatpakUpdate = UpdateView(
  version: '0.2.0.r40',
  current: '0.1.0.r27',
  canInstall: false,
  downloadUrl: 'https://dl.shardlauncher.com/linux/0.2.0.r40/Shard-0.2.0.r40-x86_64.flatpak',
);
