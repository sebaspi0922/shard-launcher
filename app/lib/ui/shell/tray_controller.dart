// Icono de la bandeja: visible si `close_behavior` es «bandeja» o si la app
// se mandó a segundo plano. Menú: abrir, jugar la última instancia y salir.

import 'dart:async';
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/app_info.dart';
import '../../core/desktop.dart';
import '../../core/launch.dart';
import '../../core/providers.dart';
import '../../core/window.dart';
import '../../l10n/app_localizations.dart';
import '../theme/accent.dart';
import '../widgets/art.dart';

/// La app arrancó oculta (autoarranque con `--background`).
final startHiddenProvider = Provider<bool>((ref) => false);

/// Icono de la marca en PNG con el acento actual.
Future<Uint8List?> renderTrayIcon(Color accent, {int size = 64}) async {
  final recorder = ui.PictureRecorder();
  final canvas = Canvas(recorder);
  BrandMarkPainter(accent).paint(canvas, Size.square(size.toDouble()));
  final image = await recorder.endRecording().toImage(size, size);
  final data = await image.toByteData(format: ui.ImageByteFormat.png);
  image.dispose();
  return data?.buffer.asUint8List();
}

class TrayController extends Notifier<bool> {
  bool _requested = false;
  Uint8List? _png;
  Color? _pngAccent;

  TrayHost get _host => ref.read(trayHostProvider);

  /// `true` = el icono está en la bandeja.
  @override
  bool build() {
    _requested = ref.read(startHiddenProvider);
    final host = ref.read(trayHostProvider);
    ref.listen(settingsProvider, (_, _) => unawaited(_refresh()));
    ref.listen(recentInstancesProvider, (_, _) => unawaited(_refresh()));
    ref.listen(accentProvider, (_, _) => unawaited(_refresh()));
    ref.onDispose(host.hide);
    Future.microtask(_refresh);
    return false;
  }

  AppLocalizations _l10n() {
    final lang = ref.read(settingsProvider).value?.language ??
        Platform.localeName.split(RegExp('[_.-]')).first;
    return lookupAppLocalizations(Locale(lang == 'en' ? 'en' : 'es'));
  }

  Future<void> _refresh() async {
    if (!ref.mounted) return;
    final behavior = ref.read(settingsProvider).value?.closeBehavior;
    final wanted = behavior == 'tray' || _requested;
    if (!wanted || !_host.supported) {
      if (state) {
        _host.hide();
        state = false;
      }
      return;
    }
    final accent = ref.read(accentProvider);
    if (_png == null || _pngAccent != accent) {
      _png = await renderTrayIcon(accent);
      _pngAccent = accent;
      if (!ref.mounted) return;
    }
    final png = _png;
    if (png == null) return;
    final l = _l10n();
    final window = ref.read(windowControlsProvider);
    final last = ref.read(recentInstancesProvider).firstOrNull;
    _host.show(
      TrayMenu(
        tooltip: kAppName,
        openLabel: l.trayOpen(kAppName),
        quitLabel: l.trayQuit,
        onOpen: () => unawaited(window.show()),
        onQuit: () => unawaited(window.quit()),
        playLabel: last == null ? null : l.trayPlay(last.name),
        onPlay: last == null
            ? null
            : () => unawaited(ref.read(launchProvider.notifier).launch(last.id)),
      ),
      png,
    );
    state = true;
  }

  /// Oculta la ventana dejando el icono. `false` si no hay bandeja (entonces
  /// solo se minimiza).
  Future<bool> toBackground() async {
    final window = ref.read(windowControlsProvider);
    if (!_host.supported) {
      await window.toBackground();
      return false;
    }
    _requested = true;
    await _refresh();
    await window.hide();
    return true;
  }
}

final trayControllerProvider =
    NotifierProvider<TrayController, bool>(TrayController.new);
