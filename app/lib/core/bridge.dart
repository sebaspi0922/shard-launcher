// Frontera con el núcleo: la UI solo habla con `LauncherBridge`. La versión
// real delega en las funciones generadas por flutter_rust_bridge; los tests
// usan una falsa con datos fijos.

import 'dart:async';

import '../src/rust/api/accounts.dart' as accounts;
import '../src/rust/api/app.dart' as app;
import '../src/rust/api/instances.dart' as instances;
import '../src/rust/api/settings.dart' as settings;
import '../src/rust/api/types.dart';

export '../src/rust/api/types.dart';

abstract class LauncherBridge {
  /// Eventos del núcleo (stream de difusión: admite varios oyentes).
  Stream<UiEvent> get events;

  /// Carpeta de datos que abrió el núcleo.
  String get dataDir;
  String get coreVersion;
  int get systemMemoryMb;

  Future<List<AccountView>> listAccounts();
  Future<DeviceCodeView> beginMicrosoftLogin();
  Future<AccountView> completeMicrosoftLogin();
  void cancelMicrosoftLogin();
  Future<AccountView> addOfflineAccount(String name);
  Future<void> setActiveAccount(String id);
  Future<void> removeAccount(String id);

  Future<List<InstanceView>> listInstances();
  Future<InstanceView> createInstance({
    required String name,
    required String mc,
    required String loader,
    String? loaderVersion,
  });
  Future<InstanceView> updateInstanceSettings({
    required String id,
    required String name,
    int? memoryMb,
    List<String>? jvmArgs,
    int? width,
    int? height,
    required bool fullscreen,
  });
  Future<String> deleteInstance(String id);
  Future<List<VersionView>> listVersions({required bool snapshots});
  Future<List<String>> listLoaderVersions(String loader, String mc);
  Future<GameView> launchInstance(String id, {String? account});
  Future<List<GameView>> runningGames();

  Future<SettingsView> getSettings();
  Future<SettingsView> saveSettings(SettingsView value);
}

/// Implementación sobre el puente de Rust. `start` ya se llamó en `main`.
class RustBridge implements LauncherBridge {
  RustBridge(this.dataDir);

  @override
  final String dataDir;

  late final Stream<UiEvent> _events = app.subscribeEvents().asBroadcastStream();

  @override
  Stream<UiEvent> get events => _events;

  @override
  String get coreVersion => app.coreVersion();

  @override
  int get systemMemoryMb => settings.systemMemoryMb().toInt();

  @override
  Future<List<AccountView>> listAccounts() => accounts.listAccounts();

  @override
  Future<DeviceCodeView> beginMicrosoftLogin() => accounts.beginMicrosoftLogin();

  @override
  Future<AccountView> completeMicrosoftLogin() =>
      accounts.completeMicrosoftLogin();

  @override
  void cancelMicrosoftLogin() => accounts.cancelMicrosoftLogin();

  @override
  Future<AccountView> addOfflineAccount(String name) =>
      accounts.addOfflineAccount(name: name);

  @override
  Future<void> setActiveAccount(String id) => accounts.setActiveAccount(id: id);

  @override
  Future<void> removeAccount(String id) => accounts.removeAccount(id: id);

  @override
  Future<List<InstanceView>> listInstances() => instances.listInstances();

  @override
  Future<InstanceView> createInstance({
    required String name,
    required String mc,
    required String loader,
    String? loaderVersion,
  }) =>
      instances.createInstance(
        name: name,
        mc: mc,
        loader: loader,
        loaderVersion: loaderVersion,
      );

  @override
  Future<InstanceView> updateInstanceSettings({
    required String id,
    required String name,
    int? memoryMb,
    List<String>? jvmArgs,
    int? width,
    int? height,
    required bool fullscreen,
  }) =>
      instances.updateInstanceSettings(
        id: id,
        name: name,
        memoryMb: memoryMb,
        jvmArgs: jvmArgs,
        width: width,
        height: height,
        fullscreen: fullscreen,
      );

  @override
  Future<String> deleteInstance(String id) => instances.deleteInstance(id: id);

  @override
  Future<List<VersionView>> listVersions({required bool snapshots}) =>
      instances.listVersions(snapshots: snapshots);

  @override
  Future<List<String>> listLoaderVersions(String loader, String mc) =>
      instances.listLoaderVersions(loader: loader, mc: mc);

  @override
  Future<GameView> launchInstance(String id, {String? account}) =>
      instances.launchInstance(id: id, account: account);

  @override
  Future<List<GameView>> runningGames() => instances.runningGames();

  @override
  Future<SettingsView> getSettings() => settings.getSettings();

  @override
  Future<SettingsView> saveSettings(SettingsView value) =>
      settings.saveSettings(settings: value);
}

extension SettingsViewCopy on SettingsView {
  SettingsView copyWith({
    String? Function()? language,
    String? closeBehavior,
    bool? allowMultipleGames,
    int? defaultMemoryMb,
    List<String>? defaultJvmArgs,
    int? maxConcurrentDownloads,
    bool? discordRpc,
  }) =>
      SettingsView(
        language: language != null ? language() : this.language,
        closeBehavior: closeBehavior ?? this.closeBehavior,
        allowMultipleGames: allowMultipleGames ?? this.allowMultipleGames,
        defaultMemoryMb: defaultMemoryMb ?? this.defaultMemoryMb,
        defaultJvmArgs: defaultJvmArgs ?? this.defaultJvmArgs,
        maxConcurrentDownloads:
            maxConcurrentDownloads ?? this.maxConcurrentDownloads,
        discordRpc: discordRpc ?? this.discordRpc,
        msaClientIdSet: msaClientIdSet,
      );
}
