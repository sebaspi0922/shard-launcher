// Estado de la app sobre el puente: cuentas, instancias y ajustes.

import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'bridge.dart';

/// Puente con el núcleo. `main` y los tests lo sustituyen.
final bridgeProvider = Provider<LauncherBridge>(
  (ref) => throw UnimplementedError('bridgeProvider sin configurar'),
);

/// Stream de eventos del núcleo (difusión).
final uiEventsProvider = Provider<Stream<UiEvent>>(
  (ref) => ref.watch(bridgeProvider).events,
);

/// Último evento recibido; útil para reaccionar en widgets.
final lastEventProvider = StreamProvider<UiEvent>(
  (ref) => ref.watch(uiEventsProvider),
);

final systemMemoryProvider = Provider<int>(
  (ref) => ref.watch(bridgeProvider).systemMemoryMb,
);

/// Escucha eventos del núcleo mientras viva el proveedor.
void listenEvents(Ref ref, void Function(UiEvent e) onEvent) {
  final sub = ref.watch(uiEventsProvider).listen(onEvent);
  ref.onDispose(sub.cancel);
}

class AccountsController extends AsyncNotifier<List<AccountView>> {
  LauncherBridge get _bridge => ref.read(bridgeProvider);

  @override
  Future<List<AccountView>> build() {
    listenEvents(ref, (e) {
      if (e is UiEvent_AccountsChanged) unawaited(refresh());
    });
    return ref.watch(bridgeProvider).listAccounts();
  }

  Future<void> refresh() async {
    final list = await AsyncValue.guard(_bridge.listAccounts);
    if (ref.mounted) state = list;
  }

  Future<AccountView> addOffline(String name) async {
    final a = await _bridge.addOfflineAccount(name);
    await refresh();
    return a;
  }

  Future<void> setActive(String id) async {
    await _bridge.setActiveAccount(id);
    await refresh();
  }

  Future<void> remove(String id) async {
    await _bridge.removeAccount(id);
    await refresh();
  }
}

final accountsProvider =
    AsyncNotifierProvider<AccountsController, List<AccountView>>(
  AccountsController.new,
);

final activeAccountProvider = Provider<AccountView?>((ref) {
  final list = ref.watch(accountsProvider).value ?? const [];
  final available = list.where((account) => !account.shard);
  for (final a in available) {
    if (a.active) return a;
  }
  return available.firstOrNull;
});

class InstancesController extends AsyncNotifier<List<InstanceView>> {
  LauncherBridge get _bridge => ref.read(bridgeProvider);

  @override
  Future<List<InstanceView>> build() {
    listenEvents(ref, (e) {
      // El tiempo jugado y la última partida cambian al cerrar el juego.
      if (e is UiEvent_GameExited) unawaited(refresh());
    });
    return ref.watch(bridgeProvider).listInstances();
  }

  Future<void> refresh() async {
    final list = await AsyncValue.guard(_bridge.listInstances);
    if (ref.mounted) state = list;
  }

  Future<InstanceView> create({
    required String name,
    required String mc,
    required String loader,
    String? loaderVersion,
  }) async {
    final i = await _bridge.createInstance(
      name: name,
      mc: mc,
      loader: loader,
      loaderVersion: loaderVersion,
    );
    await refresh();
    return i;
  }

  Future<InstanceView> updateSettings({
    required String id,
    required String name,
    int? memoryMb,
    List<String>? jvmArgs,
    int? width,
    int? height,
    required bool fullscreen,
  }) async {
    final i = await _bridge.updateInstanceSettings(
      id: id,
      name: name,
      memoryMb: memoryMb,
      jvmArgs: jvmArgs,
      width: width,
      height: height,
      fullscreen: fullscreen,
    );
    await refresh();
    return i;
  }

  Future<void> delete(String id) async {
    await _bridge.deleteInstance(id);
    await refresh();
    ref.invalidate(trashProvider);
  }

  Future<InstanceView> changeVersion({
    required String id,
    required String mc,
    required String loader,
    String? loaderVersion,
  }) async {
    final i = await _bridge.changeInstanceVersion(
      id: id,
      mc: mc,
      loader: loader,
      loaderVersion: loaderVersion,
    );
    await refresh();
    return i;
  }

  Future<InstanceView> restore(String trashId) async {
    final i = await _bridge.restoreInstance(trashId);
    await refresh();
    ref.invalidate(trashProvider);
    return i;
  }

  Future<void> purge(String trashId) async {
    await _bridge.purgeInstance(trashId);
    ref.invalidate(trashProvider);
  }
}

/// Instancias en la papelera.
final trashProvider = FutureProvider.autoDispose<List<TrashedView>>(
  (ref) => ref.watch(bridgeProvider).listTrash(),
);

final instancesProvider =
    AsyncNotifierProvider<InstancesController, List<InstanceView>>(
  InstancesController.new,
);

/// Instancias ordenadas por última partida (las nunca jugadas, al final).
final recentInstancesProvider = Provider<List<InstanceView>>((ref) {
  final list = [...?ref.watch(instancesProvider).value];
  list.sort((a, b) {
    final la = a.lastPlayed ?? -1;
    final lb = b.lastPlayed ?? -1;
    if (la != lb) return lb.compareTo(la);
    return b.createdAt.compareTo(a.createdAt);
  });
  return list;
});

final instanceByIdProvider = Provider.family<InstanceView?, String>((ref, id) {
  for (final i in ref.watch(instancesProvider).value ?? const <InstanceView>[]) {
    if (i.id == id) return i;
  }
  return null;
});

/// Instancia elegida en Inicio; `null` = la última jugada.
class HomeSelection extends Notifier<String?> {
  @override
  String? build() => null;

  void select(String id) => state = id;
}

final homeSelectionProvider =
    NotifierProvider<HomeSelection, String?>(HomeSelection.new);

final homeInstanceProvider = Provider<InstanceView?>((ref) {
  final recent = ref.watch(recentInstancesProvider);
  final selected = ref.watch(homeSelectionProvider);
  if (selected != null) {
    for (final i in recent) {
      if (i.id == selected) return i;
    }
  }
  return recent.isEmpty ? null : recent.first;
});

class SettingsController extends AsyncNotifier<SettingsView> {
  @override
  Future<SettingsView> build() => ref.watch(bridgeProvider).getSettings();

  Future<void> save(SettingsView value) async {
    final previous = state;
    // Optimista: la UI responde al momento y se corrige si falla.
    state = AsyncData(value);
    try {
      final saved = await ref.read(bridgeProvider).saveSettings(value);
      if (ref.mounted) state = AsyncData(saved);
    } catch (_) {
      if (ref.mounted) state = previous;
      rethrow;
    }
  }
}

final settingsProvider =
    AsyncNotifierProvider<SettingsController, SettingsView>(
  SettingsController.new,
);
