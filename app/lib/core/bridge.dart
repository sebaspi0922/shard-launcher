// Frontera con el núcleo: la UI solo habla con `LauncherBridge`. La versión
// real delega en las funciones generadas por flutter_rust_bridge; los tests
// usan una falsa con datos fijos.

import 'dart:async';

import '../src/rust/api/accounts.dart' as accounts;
import '../src/rust/api/app.dart' as app;
import '../src/rust/api/backups.dart' as backups;
import '../src/rust/api/content.dart' as content;
import '../src/rust/api/instances.dart' as instances;
import '../src/rust/api/servers.dart' as servers;
import '../src/rust/api/settings.dart' as settings;
import '../src/rust/api/types.dart';
import '../src/rust/api/worlds.dart' as worlds;

import '../src/rust/api/backups.dart'
    show BackupView, BackupSettingsView, RestoreView;
import '../src/rust/api/content.dart'
    show
        ProjectView,
        SearchPageView,
        ContentVersionView,
        InstalledContentView,
        ContentUpdateView,
        InstallOutcomeView,
        IdentifyView,
        ModpackInstallView;
import '../src/rust/api/servers.dart' show ServerView, ServerStatusView;
import '../src/rust/api/worlds.dart'
    show WorldView, DatapackView, ScreenshotView;

export '../src/rust/api/backups.dart'
    show BackupView, BackupSettingsView, RestoreView;
export '../src/rust/api/content.dart'
    show
        ProjectView,
        SearchPageView,
        ContentVersionView,
        InstalledContentView,
        ContentUpdateView,
        InstalledItemView,
        SkippedItemView,
        InstallOutcomeView,
        IdentifyView,
        ModpackInstallView;
export '../src/rust/api/servers.dart' show ServerView, ServerStatusView;
export '../src/rust/api/types.dart';
export '../src/rust/api/worlds.dart'
    show WorldView, DatapackView, ScreenshotView;

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
  Future<GameView> launchInstance(
    String id, {
    String? account,
    String? server,
  });
  Future<List<GameView>> runningGames();

  /// Histórico de registros de una partida (síncrono: sin carreras con los
  /// eventos en vivo).
  List<LogLineView> logBacklog(BigInt gameId);

  /// Partida más reciente de la instancia con histórico disponible.
  BigInt? latestGameFor(String instanceId);

  Future<SettingsView> getSettings();
  Future<SettingsView> saveSettings(SettingsView value);

  // Contenido.
  Future<SearchPageView> searchContent({
    required String source,
    required String kind,
    required String text,
    String? mc,
    String? loader,
    required String sort,
    required int offset,
    required int limit,
  });
  Future<ProjectView> getProject(String source, String project);
  Future<List<ContentVersionView>> contentVersions(
    String source,
    String project, {
    String? mc,
    String? loader,
  });
  Future<InstallOutcomeView> installContent(
    String instanceId,
    String source,
    String project, {
    String? version,
    String? world,
  });
  Future<List<InstalledContentView>> listContent(String instanceId);
  Future<InstalledContentView> setContentEnabled(
    String instanceId,
    String file,
    bool enabled,
  );
  Future<InstalledContentView> removeContent(String instanceId, String file);
  Future<List<ContentUpdateView>> checkContentUpdates(String instanceId);
  Future<String?> updateContent(String instanceId, String file);
  Future<IdentifyView> identifyContent(String instanceId);
  Future<ModpackInstallView> installModpack(String source, {String? name});
  Future<bool> hasCurseforgeKey();
  Future<void> setCurseforgeKey(String key);
  Future<void> clearCurseforgeKey();

  /// Ruta local de una imagen remota (descargada una vez), o `null`.
  Future<String?> cachedImage(String url);

  // Mundos y capturas.
  Future<List<WorldView>> listWorlds(String instanceId);
  Future<WorldView> renameWorld(String instanceId, String folder, String name);
  Future<WorldView> updateWorld(
    String instanceId,
    String folder, {
    String? gameMode,
    String? difficulty,
    bool? hardcore,
    bool? cheats,
  });
  Future<void> deleteWorld(String instanceId, String folder);
  Future<List<DatapackView>> listDatapacks(String instanceId, String world);
  Future<void> setDatapackEnabled(
    String instanceId,
    String world,
    String name,
    bool enabled,
  );
  Future<void> removeDatapack(String instanceId, String world, String name);
  Future<List<ScreenshotView>> listScreenshots(String instanceId);
  Future<void> deleteScreenshot(String instanceId, String file);

  // Copias de seguridad.
  Future<List<BackupView>> listBackups(String instanceId);
  Future<BackupView> createBackup(
    String instanceId, {
    String? label,
    bool pinned = false,
  });
  Future<void> deleteBackup(String instanceId, String backupId);
  Future<BackupView> setBackupPinned(
    String instanceId,
    String backupId,
    bool pinned,
  );
  Future<BackupView> setBackupLabel(
    String instanceId,
    String backupId,
    String? label,
  );
  Future<RestoreView> restoreBackup(
    String instanceId,
    String backupId, {
    required bool worldsOnly,
  });
  Future<BackupSettingsView> getBackupSettings(String instanceId);
  Future<BackupSettingsView> updateBackupSettings(
    String instanceId, {
    required bool auto,
    required int intervalHours,
    required int maxCount,
  });

  // Servidores.
  Future<List<ServerView>> listServers();
  Future<ServerView> addServer({
    required String name,
    required String address,
    String? instanceId,
    String? accountId,
  });
  Future<ServerView> updateServer({
    required String id,
    required String name,
    required String address,
    String? instanceId,
    String? accountId,
  });
  Future<void> removeServer(String id);
  Future<List<ServerView>> moveServer(String id, int index);
  Future<int> importServersDat(String instanceId);
  Future<List<ServerStatusView>> pingServers(List<String> addresses);
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
  Future<GameView> launchInstance(
    String id, {
    String? account,
    String? server,
  }) =>
      instances.launchInstance(id: id, account: account, server: server);

  @override
  Future<List<GameView>> runningGames() => instances.runningGames();

  @override
  List<LogLineView> logBacklog(BigInt gameId) =>
      app.getLogBacklog(gameId: gameId);

  @override
  BigInt? latestGameFor(String instanceId) =>
      app.latestGameForInstance(instanceId: instanceId);

  @override
  Future<SettingsView> getSettings() => settings.getSettings();

  @override
  Future<SettingsView> saveSettings(SettingsView value) =>
      settings.saveSettings(settings: value);

  @override
  Future<SearchPageView> searchContent({
    required String source,
    required String kind,
    required String text,
    String? mc,
    String? loader,
    required String sort,
    required int offset,
    required int limit,
  }) =>
      content.searchContent(
        source: source,
        kind: kind,
        text: text,
        mc: mc,
        loader: loader,
        sort: sort,
        offset: offset,
        limit: limit,
      );

  @override
  Future<ProjectView> getProject(String source, String project) =>
      content.getProject(source: source, project: project);

  @override
  Future<List<ContentVersionView>> contentVersions(
    String source,
    String project, {
    String? mc,
    String? loader,
  }) =>
      content.contentVersions(
        source: source,
        project: project,
        mc: mc,
        loader: loader,
      );

  @override
  Future<InstallOutcomeView> installContent(
    String instanceId,
    String source,
    String project, {
    String? version,
    String? world,
  }) =>
      content.installContent(
        instanceId: instanceId,
        source: source,
        project: project,
        version: version,
        world: world,
      );

  @override
  Future<List<InstalledContentView>> listContent(String instanceId) =>
      content.listContent(instanceId: instanceId);

  @override
  Future<InstalledContentView> setContentEnabled(
    String instanceId,
    String file,
    bool enabled,
  ) =>
      content.setContentEnabled(
        instanceId: instanceId,
        file: file,
        enabled: enabled,
      );

  @override
  Future<InstalledContentView> removeContent(String instanceId, String file) =>
      content.removeContent(instanceId: instanceId, file: file);

  @override
  Future<List<ContentUpdateView>> checkContentUpdates(String instanceId) =>
      content.checkContentUpdates(instanceId: instanceId);

  @override
  Future<String?> updateContent(String instanceId, String file) =>
      content.updateContent(instanceId: instanceId, file: file);

  @override
  Future<IdentifyView> identifyContent(String instanceId) =>
      content.identifyContent(instanceId: instanceId);

  @override
  Future<ModpackInstallView> installModpack(String source, {String? name}) =>
      content.installModpack(source: source, name: name);

  @override
  Future<bool> hasCurseforgeKey() => content.hasCurseforgeKey();

  @override
  Future<void> setCurseforgeKey(String key) =>
      content.setCurseforgeKey(key: key);

  @override
  Future<void> clearCurseforgeKey() => content.clearCurseforgeKey();

  @override
  Future<String?> cachedImage(String url) => content.cachedImage(url: url);

  @override
  Future<List<WorldView>> listWorlds(String instanceId) =>
      worlds.listWorlds(instanceId: instanceId);

  @override
  Future<WorldView> renameWorld(String instanceId, String folder, String name) =>
      worlds.renameWorld(instanceId: instanceId, folder: folder, name: name);

  @override
  Future<WorldView> updateWorld(
    String instanceId,
    String folder, {
    String? gameMode,
    String? difficulty,
    bool? hardcore,
    bool? cheats,
  }) =>
      worlds.updateWorld(
        instanceId: instanceId,
        folder: folder,
        gameMode: gameMode,
        difficulty: difficulty,
        hardcore: hardcore,
        cheats: cheats,
      );

  @override
  Future<void> deleteWorld(String instanceId, String folder) =>
      worlds.deleteWorld(instanceId: instanceId, folder: folder);

  @override
  Future<List<DatapackView>> listDatapacks(String instanceId, String world) =>
      worlds.listDatapacks(instanceId: instanceId, world: world);

  @override
  Future<void> setDatapackEnabled(
    String instanceId,
    String world,
    String name,
    bool enabled,
  ) =>
      worlds.setDatapackEnabled(
        instanceId: instanceId,
        world: world,
        name: name,
        enabled: enabled,
      );

  @override
  Future<void> removeDatapack(String instanceId, String world, String name) =>
      worlds.removeDatapack(instanceId: instanceId, world: world, name: name);

  @override
  Future<List<ScreenshotView>> listScreenshots(String instanceId) =>
      worlds.listScreenshots(instanceId: instanceId);

  @override
  Future<void> deleteScreenshot(String instanceId, String file) =>
      worlds.deleteScreenshot(instanceId: instanceId, file: file);

  @override
  Future<List<BackupView>> listBackups(String instanceId) =>
      backups.listBackups(instanceId: instanceId);

  @override
  Future<BackupView> createBackup(
    String instanceId, {
    String? label,
    bool pinned = false,
  }) =>
      backups.createBackup(
        instanceId: instanceId,
        label: label,
        pinned: pinned,
      );

  @override
  Future<void> deleteBackup(String instanceId, String backupId) =>
      backups.deleteBackup(instanceId: instanceId, backupId: backupId);

  @override
  Future<BackupView> setBackupPinned(
    String instanceId,
    String backupId,
    bool pinned,
  ) =>
      backups.setBackupPinned(
        instanceId: instanceId,
        backupId: backupId,
        pinned: pinned,
      );

  @override
  Future<BackupView> setBackupLabel(
    String instanceId,
    String backupId,
    String? label,
  ) =>
      backups.setBackupLabel(
        instanceId: instanceId,
        backupId: backupId,
        label: label,
      );

  @override
  Future<RestoreView> restoreBackup(
    String instanceId,
    String backupId, {
    required bool worldsOnly,
  }) =>
      backups.restoreBackup(
        instanceId: instanceId,
        backupId: backupId,
        worldsOnly: worldsOnly,
      );

  @override
  Future<BackupSettingsView> getBackupSettings(String instanceId) =>
      backups.getBackupSettings(instanceId: instanceId);

  @override
  Future<BackupSettingsView> updateBackupSettings(
    String instanceId, {
    required bool auto,
    required int intervalHours,
    required int maxCount,
  }) =>
      backups.updateBackupSettings(
        instanceId: instanceId,
        auto: auto,
        intervalHours: intervalHours,
        maxCount: maxCount,
      );

  @override
  Future<List<ServerView>> listServers() => servers.listServers();

  @override
  Future<ServerView> addServer({
    required String name,
    required String address,
    String? instanceId,
    String? accountId,
  }) =>
      servers.addServer(
        name: name,
        address: address,
        instanceId: instanceId,
        accountId: accountId,
      );

  @override
  Future<ServerView> updateServer({
    required String id,
    required String name,
    required String address,
    String? instanceId,
    String? accountId,
  }) =>
      servers.updateServer(
        id: id,
        name: name,
        address: address,
        instanceId: instanceId,
        accountId: accountId,
      );

  @override
  Future<void> removeServer(String id) => servers.removeServer(id: id);

  @override
  Future<List<ServerView>> moveServer(String id, int index) =>
      servers.moveServer(id: id, index: index);

  @override
  Future<int> importServersDat(String instanceId) =>
      servers.importServersDat(instanceId: instanceId);

  @override
  Future<List<ServerStatusView>> pingServers(List<String> addresses) =>
      servers.pingServers(addresses: addresses);
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
