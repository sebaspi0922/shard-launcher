// Formato de tiempos, memoria y loaders para la UI.

import 'package:intl/intl.dart';

import '../l10n/app_localizations.dart';

String formatPlaytime(AppLocalizations l, BigInt seconds) {
  final total = seconds.toInt();
  final h = total ~/ 3600;
  final m = (total % 3600) ~/ 60;
  if (h == 0 && m == 0) return l.playtimeNone;
  if (h == 0) return l.playtimeMinutes(m);
  if (m == 0 || h >= 100) return l.playtimeHours(h);
  return l.playtimeHoursMinutes(h, m);
}

/// «hace 3 días» a partir de segundos Unix. `now` es inyectable para tests.
String formatRelative(AppLocalizations l, int? unixSeconds, {DateTime? now}) {
  if (unixSeconds == null) return l.lastPlayedNever;
  final at = DateTime.fromMillisecondsSinceEpoch(unixSeconds * 1000);
  final diff = (now ?? DateTime.now()).difference(at);
  if (diff.inMinutes < 1) return l.relJustNow;
  if (diff.inHours < 1) return l.relMinutes(diff.inMinutes);
  if (diff.inDays < 1) return l.relHours(diff.inHours);
  if (diff.inDays < 30) return l.relDays(diff.inDays);
  if (diff.inDays < 365) return l.relMonths(diff.inDays ~/ 30);
  return l.relYears(diff.inDays ~/ 365);
}

/// «4 GB», «4,5 GB» según el idioma.
String formatMemory(AppLocalizations l, int mb) {
  final gb = mb / 1024;
  final f = NumberFormat('0.#', l.localeName);
  return l.memoryGb(f.format(gb));
}

/// Nombre visible de un loader del núcleo.
String loaderName(AppLocalizations l, String loader) => switch (loader) {
      'vanilla' => l.loaderVanilla,
      'fabric' => 'Fabric',
      'quilt' => 'Quilt',
      'forge' => 'Forge',
      'neoforge' => 'NeoForge',
      'labymod' => 'LabyMod',
      _ => l.loaderCustom,
    };

/// Loaders que se pueden elegir al crear una instancia.
const kCreatableLoaders = ['vanilla', 'fabric', 'quilt', 'forge', 'neoforge'];

/// Texto corto de versión: «1.21.1 · Fabric».
String versionLine(AppLocalizations l, String mc, String loader) =>
    '$mc · ${loaderName(l, loader)}';
