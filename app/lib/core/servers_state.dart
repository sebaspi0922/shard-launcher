// Servidores guardados y su estado en vivo.

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'bridge.dart';
import 'providers.dart';

class ServersController extends AsyncNotifier<List<ServerView>> {
  LauncherBridge get _bridge => ref.read(bridgeProvider);

  @override
  Future<List<ServerView>> build() => ref.watch(bridgeProvider).listServers();

  Future<void> refresh() async {
    final next = await AsyncValue.guard(_bridge.listServers);
    if (ref.mounted) state = next;
  }

  Future<ServerView> add({
    required String name,
    required String address,
    String? instanceId,
    String? accountId,
  }) async {
    final s = await _bridge.addServer(
      name: name,
      address: address,
      instanceId: instanceId,
      accountId: accountId,
    );
    await refresh();
    return s;
  }

  Future<void> edit({
    required String id,
    required String name,
    required String address,
    String? instanceId,
    String? accountId,
  }) async {
    await _bridge.updateServer(
      id: id,
      name: name,
      address: address,
      instanceId: instanceId,
      accountId: accountId,
    );
    await refresh();
  }

  Future<void> remove(String id) async {
    await _bridge.removeServer(id);
    await refresh();
  }

  /// Reordena al momento y guarda.
  Future<void> move(String id, int index) async {
    final list = [...?state.value];
    final from = list.indexWhere((s) => s.id == id);
    if (from < 0) return;
    final s = list.removeAt(from);
    list.insert(index.clamp(0, list.length), s);
    state = AsyncData(list);
    try {
      final saved = await _bridge.moveServer(id, index);
      if (ref.mounted) state = AsyncData(saved);
    } catch (_) {
      await refresh();
      rethrow;
    }
  }

  Future<int> importFrom(String instanceId) async {
    final n = await _bridge.importServersDat(instanceId);
    await refresh();
    return n;
  }
}

final serversProvider =
    AsyncNotifierProvider<ServersController, List<ServerView>>(
  ServersController.new,
);

/// Estado de cada dirección guardada (ping en paralelo; caché de 30 s en el
/// núcleo). `ref.invalidate` vuelve a consultar.
final serverStatusProvider =
    FutureProvider<Map<String, ServerStatusView>>((ref) async {
  final servers = ref.watch(serversProvider).value ?? const <ServerView>[];
  final addresses = {for (final s in servers) s.address}.toList();
  if (addresses.isEmpty) return const {};
  final list = await ref.read(bridgeProvider).pingServers(addresses);
  return {for (final s in list) s.address: s};
});

/// Favicon decodificado (con caché por contenido).
final faviconProvider = Provider.family<Uint8List?, String>((ref, b64) {
  try {
    return base64Decode(b64.replaceAll(RegExp(r'\s'), ''));
  } catch (_) {
    return null;
  }
});

/// ¿Hay clave de CurseForge guardada? (nunca se lee la clave).
class CurseforgeKeyController extends AsyncNotifier<bool> {
  @override
  Future<bool> build() => ref.watch(bridgeProvider).hasCurseforgeKey();

  Future<void> set(String key) async {
    await ref.read(bridgeProvider).setCurseforgeKey(key);
    state = const AsyncData(true);
  }

  Future<void> clear() async {
    await ref.read(bridgeProvider).clearCurseforgeKey();
    state = const AsyncData(false);
  }
}

final curseforgeKeyProvider =
    AsyncNotifierProvider<CurseforgeKeyController, bool>(
  CurseforgeKeyController.new,
);
