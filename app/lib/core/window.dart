// Control de la ventana (sin decoraciones del sistema). En tests se usa la
// versión inerte para no depender del plugin.

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:window_manager/window_manager.dart';

abstract class WindowControls {
  /// `true` si la ventana está maximizada (para el icono del botón).
  ValueListenable<bool> get maximized;

  /// Se llama cuando el sistema o el botón propio piden cerrar.
  set onCloseRequested(Future<void> Function()? handler);

  Future<void> minimize();
  Future<void> toggleMaximize();
  Future<void> startDragging();

  /// Pasa a segundo plano sin bandeja: minimiza.
  Future<void> toBackground();

  /// Oculta la ventana (queda el icono de la bandeja).
  Future<void> hide();

  /// Muestra y enfoca la ventana (desde la bandeja).
  Future<void> show();

  /// Cierra la app de verdad.
  Future<void> quit();

  /// Pide el cierre (pasa por `onCloseRequested`).
  Future<void> requestClose();
}

class NoopWindowControls implements WindowControls {
  @override
  final ValueNotifier<bool> maximized = ValueNotifier(false);
  Future<void> Function()? _onClose;

  @override
  set onCloseRequested(Future<void> Function()? handler) => _onClose = handler;

  @override
  Future<void> minimize() async {}
  @override
  Future<void> toggleMaximize() async => maximized.value = !maximized.value;
  @override
  Future<void> startDragging() async {}
  @override
  Future<void> toBackground() async {}
  @override
  Future<void> hide() async => hidden = true;
  @override
  Future<void> show() async => hidden = false;
  @override
  Future<void> quit() async {}

  /// Para tests: la ventana está oculta.
  bool hidden = false;
  @override
  Future<void> requestClose() async => _onClose?.call();
}

class DesktopWindowControls with WindowListener implements WindowControls {
  DesktopWindowControls() {
    windowManager.addListener(this);
    unawaited(windowManager.isMaximized().then((v) => maximized.value = v));
  }

  @override
  final ValueNotifier<bool> maximized = ValueNotifier(false);
  Future<void> Function()? _onClose;

  @override
  set onCloseRequested(Future<void> Function()? handler) => _onClose = handler;

  @override
  Future<void> minimize() => windowManager.minimize();

  @override
  Future<void> toggleMaximize() async {
    if (await windowManager.isMaximized()) {
      await windowManager.unmaximize();
    } else {
      await windowManager.maximize();
    }
  }

  @override
  Future<void> startDragging() => windowManager.startDragging();

  @override
  Future<void> toBackground() => windowManager.minimize();

  @override
  Future<void> hide() => windowManager.hide();

  @override
  Future<void> show() async {
    await windowManager.show();
    await windowManager.focus();
  }

  @override
  Future<void> quit() async {
    await windowManager.setPreventClose(false);
    await windowManager.destroy();
  }

  @override
  Future<void> requestClose() async {
    final handler = _onClose;
    if (handler == null) return quit();
    await handler();
  }

  @override
  void onWindowClose() => unawaited(requestClose());

  @override
  void onWindowMaximize() => maximized.value = true;

  @override
  void onWindowUnmaximize() => maximized.value = false;
}

final windowControlsProvider =
    Provider<WindowControls>((ref) => NoopWindowControls());
