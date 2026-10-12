// Actualizaciones de la app (D8): consulta al arrancar y cada pocas horas si
// el ajuste lo permite y la app viene empaquetada. Solo el AppImage se
// instala solo; los demás paquetes enlazan la descarga.

import 'dart:async';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app_info.dart';
import 'bridge.dart';
import 'providers.dart';
import 'window.dart';

enum UpdateStatus { idle, checking, upToDate, available, installing, ready, failed }

class UpdateState {
  const UpdateState({
    this.status = UpdateStatus.idle,
    this.info,
    this.error,
    this.path,
    this.dismissed = false,
  });

  final UpdateStatus status;

  /// Versión nueva (desde `available` en adelante).
  final UpdateView? info;
  final Object? error;

  /// AppImage ya actualizado (`ready`).
  final String? path;

  /// El aviso flotante se cerró; Ajustes sigue mostrando el estado.
  final bool dismissed;

  UpdateState copyWith({
    UpdateStatus? status,
    UpdateView? info,
    Object? error,
    String? path,
    bool? dismissed,
  }) =>
      UpdateState(
        status: status ?? this.status,
        info: info ?? this.info,
        error: error,
        path: path ?? this.path,
        dismissed: dismissed ?? this.dismissed,
      );
}

/// Espera tras el arranque y entre consultas automáticas.
const kUpdateFirstCheck = Duration(seconds: 20);
const kUpdateInterval = Duration(hours: 6);

class UpdateController extends Notifier<UpdateState> {
  Timer? _timer;

  LauncherBridge get _bridge => ref.read(bridgeProvider);

  /// Versión con la que se compara: la del paquete o, sin ella, la del núcleo.
  String get currentVersion =>
      kPackageVersion.isNotEmpty ? kPackageVersion : _bridge.coreVersion;

  /// Las consultas automáticas solo tienen sentido en una app empaquetada.
  bool get autoCheckAvailable =>
      kPackageVersion.isNotEmpty && _bridge.updatePackage != 'unpackaged';

  @override
  UpdateState build() {
    ref.onDispose(() => _timer?.cancel());
    if (autoCheckAvailable) {
      _timer = Timer(kUpdateFirstCheck, _autoCheck);
    }
    return const UpdateState();
  }

  Future<void> _autoCheck() async {
    _timer = Timer(kUpdateInterval, _autoCheck);
    final settings = ref.read(settingsProvider).value;
    if (settings == null || !settings.checkUpdates) return;
    // Sin molestar mientras se instala o ya hay algo que mostrar.
    if (state.status case UpdateStatus.installing || UpdateStatus.ready) return;
    await check(manual: false);
  }

  /// Consulta el manifiesto. Los fallos de una consulta automática no se
  /// muestran (sin red, servidor caído…).
  Future<void> check({bool manual = true}) async {
    if (state.status == UpdateStatus.checking) return;
    final previous = state;
    state = state.copyWith(status: UpdateStatus.checking);
    try {
      final info = await _bridge.checkUpdate(currentVersion);
      if (!ref.mounted) return;
      if (info == null) {
        state = const UpdateState(status: UpdateStatus.upToDate);
      } else {
        // Un aviso cerrado vuelve a salir solo si hay otra versión.
        final same = previous.info?.version == info.version;
        state = UpdateState(
          status: UpdateStatus.available,
          info: info,
          dismissed: !manual && same && previous.dismissed,
        );
      }
    } catch (e) {
      if (!ref.mounted) return;
      state = manual
          ? previous.copyWith(status: UpdateStatus.failed, error: e)
          : previous;
    }
  }

  Future<void> install() async {
    final info = state.info;
    if (info == null || !info.canInstall) return;
    state = state.copyWith(status: UpdateStatus.installing, dismissed: false);
    try {
      final path = await _bridge.installUpdate();
      if (ref.mounted) state = state.copyWith(status: UpdateStatus.ready, path: path);
    } catch (e) {
      if (ref.mounted) state = state.copyWith(status: UpdateStatus.failed, error: e);
    }
  }

  /// Abre el AppImage nuevo y cierra este proceso. Las partidas abiertas
  /// siguen: el juego no depende del launcher.
  Future<void> restart() async {
    final path = state.path;
    if (path == null) return;
    await Process.start(path, const [], mode: ProcessStartMode.detached);
    await ref.read(windowControlsProvider).quit();
  }

  void dismiss() => state = state.copyWith(status: state.status, dismissed: true);

  /// Solo para tests y goldens.
  void debugSet(UpdateState s) => state = s;
}

final updateProvider = NotifierProvider<UpdateController, UpdateState>(
  UpdateController.new,
);
