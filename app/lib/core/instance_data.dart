// Datos de una instancia que se cargan al abrir sus pestañas: contenido,
// actualizaciones, mundos, datapacks, capturas y copias de seguridad.

import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'bridge.dart';
import 'providers.dart';

/// Identificador sin ambigüedad de un archivo de contenido (`tipo/archivo`).
String contentKey(String kind, String file) => '$kind/$file';

// ---------------------------------------------------------------------------
// Contenido instalado
// ---------------------------------------------------------------------------

class InstanceContentController
    extends AsyncNotifier<List<InstalledContentView>> {
  InstanceContentController(this.instanceId);

  final String instanceId;

  LauncherBridge get _bridge => ref.read(bridgeProvider);

  @override
  Future<List<InstalledContentView>> build() =>
      ref.watch(bridgeProvider).listContent(instanceId);

  Future<void> refresh() async {
    final next = await AsyncValue.guard(() => _bridge.listContent(instanceId));
    if (ref.mounted) state = next;
  }

  /// Activa o desactiva al momento y corrige si el núcleo falla.
  Future<void> setEnabled(InstalledContentView item, bool enabled) async {
    final previous = state;
    state = AsyncData([
      for (final c in state.value ?? const <InstalledContentView>[])
        c.kind == item.kind && c.file == item.file ? _withEnabled(c, enabled) : c,
    ]);
    try {
      await _bridge.setContentEnabled(
        instanceId,
        contentKey(item.kind, item.file),
        enabled,
      );
    } catch (_) {
      if (ref.mounted) state = previous;
      rethrow;
    }
  }

  Future<void> remove(InstalledContentView item) async {
    await _bridge.removeContent(instanceId, contentKey(item.kind, item.file));
    await refresh();
  }

  /// Actualiza un archivo y devuelve la versión nueva.
  Future<String?> updateOne(String kind, String file) async {
    final v = await _bridge.updateContent(instanceId, contentKey(kind, file));
    await refresh();
    ref.invalidate(contentUpdatesProvider(instanceId));
    return v;
  }

  /// Actualiza todo lo que tenga versión nueva. Devuelve cuántos se
  /// actualizaron y los errores que hubo.
  Future<(int, List<Object>)> updateAll(List<ContentUpdateView> updates) async {
    var ok = 0;
    final errors = <Object>[];
    for (final u in updates) {
      try {
        await _bridge.updateContent(instanceId, contentKey(u.kind, u.file));
        ok++;
      } catch (e) {
        errors.add(e);
      }
    }
    await refresh();
    ref.invalidate(contentUpdatesProvider(instanceId));
    return (ok, errors);
  }

  Future<IdentifyView> identify() async {
    final r = await _bridge.identifyContent(instanceId);
    await refresh();
    ref.invalidate(contentUpdatesProvider(instanceId));
    return r;
  }
}

InstalledContentView _withEnabled(InstalledContentView c, bool enabled) =>
    InstalledContentView(
      kind: c.kind,
      file: c.file,
      enabled: enabled,
      source: c.source,
      projectId: c.projectId,
      projectName: c.projectName,
      versionId: c.versionId,
      versionNumber: c.versionNumber,
      size: c.size,
      isDir: c.isDir,
      tracked: c.tracked,
      world: c.world,
    );

final instanceContentProvider = AsyncNotifierProvider.autoDispose
    .family<InstanceContentController, List<InstalledContentView>, String>(
  InstanceContentController.new,
);

/// Actualizaciones disponibles (consulta Modrinth/CurseForge).
final contentUpdatesProvider = FutureProvider.autoDispose
    .family<List<ContentUpdateView>, String>(
  (ref, id) => ref.watch(bridgeProvider).checkContentUpdates(id),
);

// ---------------------------------------------------------------------------
// Mundos y datapacks
// ---------------------------------------------------------------------------

class WorldsController extends AsyncNotifier<List<WorldView>> {
  WorldsController(this.instanceId);

  final String instanceId;

  LauncherBridge get _bridge => ref.read(bridgeProvider);

  @override
  Future<List<WorldView>> build() =>
      ref.watch(bridgeProvider).listWorlds(instanceId);

  void _replace(WorldView w) {
    state = AsyncData([
      for (final x in state.value ?? const <WorldView>[])
        x.folder == w.folder ? w : x,
    ]);
  }

  Future<void> rename(String folder, String name) async {
    final w = await _bridge.renameWorld(instanceId, folder, name);
    if (ref.mounted) _replace(w);
  }

  Future<void> patch(
    String folder, {
    String? gameMode,
    String? difficulty,
    bool? hardcore,
    bool? cheats,
  }) async {
    final w = await _bridge.updateWorld(
      instanceId,
      folder,
      gameMode: gameMode,
      difficulty: difficulty,
      hardcore: hardcore,
      cheats: cheats,
    );
    if (ref.mounted) _replace(w);
  }

  Future<void> delete(String folder) async {
    await _bridge.deleteWorld(instanceId, folder);
    if (!ref.mounted) return;
    state = AsyncData([
      for (final x in state.value ?? const <WorldView>[])
        if (x.folder != folder) x,
    ]);
  }
}

final worldsProvider = AsyncNotifierProvider.autoDispose
    .family<WorldsController, List<WorldView>, String>(WorldsController.new);

typedef WorldRef = ({String instance, String world});

final datapacksProvider =
    FutureProvider.autoDispose.family<List<DatapackView>, WorldRef>(
  (ref, w) => ref.watch(bridgeProvider).listDatapacks(w.instance, w.world),
);

// ---------------------------------------------------------------------------
// Capturas
// ---------------------------------------------------------------------------

final screenshotsProvider =
    FutureProvider.autoDispose.family<List<ScreenshotView>, String>(
  (ref, id) => ref.watch(bridgeProvider).listScreenshots(id),
);

// ---------------------------------------------------------------------------
// Copias de seguridad
// ---------------------------------------------------------------------------

class BackupsController extends AsyncNotifier<List<BackupView>> {
  BackupsController(this.instanceId);

  final String instanceId;

  LauncherBridge get _bridge => ref.read(bridgeProvider);

  @override
  Future<List<BackupView>> build() =>
      ref.watch(bridgeProvider).listBackups(instanceId);

  Future<void> refresh() async {
    final next = await AsyncValue.guard(() => _bridge.listBackups(instanceId));
    if (ref.mounted) state = next;
  }

  Future<BackupView> create({String? label, bool pinned = false}) async {
    final b = await _bridge.createBackup(
      instanceId,
      label: label,
      pinned: pinned,
    );
    await refresh();
    return b;
  }

  void _replace(BackupView b) {
    state = AsyncData([
      for (final x in state.value ?? const <BackupView>[]) x.id == b.id ? b : x,
    ]);
  }

  Future<void> setPinned(String id, bool pinned) async {
    final b = await _bridge.setBackupPinned(instanceId, id, pinned);
    if (ref.mounted) _replace(b);
  }

  Future<void> setLabel(String id, String? label) async {
    final b = await _bridge.setBackupLabel(instanceId, id, label);
    if (ref.mounted) _replace(b);
  }

  Future<void> delete(String id) async {
    await _bridge.deleteBackup(instanceId, id);
    await refresh();
  }

  Future<RestoreView> restore(String id, {required bool worldsOnly}) async {
    final r = await _bridge.restoreBackup(
      instanceId,
      id,
      worldsOnly: worldsOnly,
    );
    // Lo restaurado cambia mundos y contenido.
    ref
      ..invalidate(worldsProvider(instanceId))
      ..invalidate(instanceContentProvider(instanceId))
      ..invalidate(screenshotsProvider(instanceId));
    return r;
  }
}

final backupsProvider = AsyncNotifierProvider.autoDispose
    .family<BackupsController, List<BackupView>, String>(BackupsController.new);

class BackupSettingsController extends AsyncNotifier<BackupSettingsView> {
  BackupSettingsController(this.instanceId);

  final String instanceId;

  @override
  Future<BackupSettingsView> build() =>
      ref.watch(bridgeProvider).getBackupSettings(instanceId);

  Future<void> save(BackupSettingsView value) async {
    final previous = state;
    state = AsyncData(value);
    try {
      final saved = await ref.read(bridgeProvider).updateBackupSettings(
            instanceId,
            auto: value.auto,
            intervalHours: value.intervalHours,
            maxCount: value.maxCount,
          );
      if (ref.mounted) state = AsyncData(saved);
    } catch (_) {
      if (ref.mounted) state = previous;
      rethrow;
    }
  }
}

final backupSettingsProvider = AsyncNotifierProvider.autoDispose
    .family<BackupSettingsController, BackupSettingsView, String>(
  BackupSettingsController.new,
);

/// Se usa al cerrar el juego: las listas pueden haber cambiado.
void invalidateInstanceData(Ref ref, String instanceId) {
  ref
    ..invalidate(worldsProvider(instanceId))
    ..invalidate(screenshotsProvider(instanceId))
    ..invalidate(instanceContentProvider(instanceId));
}

/// Refresca mundos, capturas y contenido de la instancia al cerrar el juego.
final instanceDataRefresherProvider = Provider<void>((ref) {
  listenEvents(ref, (e) {
    if (e is UiEvent_GameExited) {
      unawaited(Future.microtask(() => invalidateInstanceData(ref, e.instanceId)));
    }
  });
});
