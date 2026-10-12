// Puente falso con datos fijos para tests y goldens.

import 'dart:async';

import 'package:lcore_app/core/bridge.dart';

/// «Ahora» fijo de los tests (el reloj de la UI se ajusta a él en `pumpApp`).
final kTestNow = DateTime(2026, 10, 11, 18, 30);

int _ago(Duration d) =>
    kTestNow.subtract(d).millisecondsSinceEpoch ~/ 1000;

InstanceView fakeInstance({
  required String id,
  required String name,
  required String mc,
  String loader = 'vanilla',
  String? loaderVersion,
  Duration? lastPlayed,
  int playtimeMinutes = 0,
  int? memoryMb,
  List<String>? jvmArgs,
  int? width,
  int? height,
  bool fullscreen = false,
}) =>
    InstanceView(
      id: id,
      name: name,
      mc: mc,
      loader: loader,
      loaderVersion: loaderVersion,
      createdAt: _ago(const Duration(days: 200)),
      lastPlayed: lastPlayed == null ? null : _ago(lastPlayed),
      playtimeSeconds: BigInt.from(playtimeMinutes * 60),
      memoryMb: memoryMb,
      jvmArgs: jvmArgs,
      width: width,
      height: height,
      fullscreen: fullscreen,
      gameDir: '/tmp/shard-test/instances/$id/minecraft',
    );

List<InstanceView> sampleInstances() => [
      fakeInstance(
        id: 'survival-amigos',
        name: 'Survival con amigos',
        mc: '1.21.4',
        loader: 'fabric',
        loaderVersion: '0.16.10',
        lastPlayed: const Duration(hours: 2),
        playtimeMinutes: 86 * 60 + 12,
        memoryMb: 6144,
        jvmArgs: ['-XX:+UseG1GC', '-XX:MaxGCPauseMillis=50'],
        width: 1600,
        height: 900,
      ),
      fakeInstance(
        id: 'create-aeronautica',
        name: 'Create: Aeronáutica',
        mc: '1.20.1',
        loader: 'forge',
        loaderVersion: '47.3.0',
        lastPlayed: const Duration(days: 3),
        playtimeMinutes: 41 * 60,
      ),
      fakeInstance(
        id: 'vanilla-puro',
        name: 'Vanilla puro',
        mc: '1.21.4',
        lastPlayed: const Duration(days: 1, hours: 2),
        playtimeMinutes: 640,
      ),
      fakeInstance(
        id: 'skyblock-tecnico',
        name: 'Skyblock técnico',
        mc: '1.21.1',
        loader: 'neoforge',
        lastPlayed: const Duration(days: 64),
        playtimeMinutes: 19 * 60 + 45,
      ),
      fakeInstance(
        id: 'quilt-pruebas',
        name: 'Pruebas de shaders',
        mc: '1.21.5',
        loader: 'quilt',
      ),
    ];

List<AccountView> sampleAccounts() => const [
      AccountView(id: 'a1', name: 'NadiaK', microsoft: true, active: true),
      AccountView(id: 'a2', name: 'Pruebas_01', microsoft: false, active: false),
    ];

class FakeBridge implements LauncherBridge {
  FakeBridge({
    List<InstanceView>? instances,
    List<AccountView>? accounts,
    this.language = 'es',
  })  : instances = instances ?? sampleInstances(),
        accounts = accounts ?? sampleAccounts();

  List<InstanceView> instances;
  List<AccountView> accounts;
  String? language;
  final _events = StreamController<UiEvent>.broadcast();

  /// Lanza este error al lanzar (para probar los estados de error).
  BridgeError? launchError;

  /// Se completa a mano para controlar el lanzamiento.
  Completer<GameView>? launchCompleter;
  final Completer<AccountView> _login = Completer();
  bool loginCancelled = false;

  void emit(UiEvent e) => _events.add(e);

  @override
  Stream<UiEvent> get events => _events.stream;

  @override
  String get dataDir => '/home/user/.local/share/shard';

  @override
  String get coreVersion => '0.1.0';

  @override
  int get systemMemoryMb => 16384;

  @override
  Future<List<AccountView>> listAccounts() async => accounts;

  @override
  Future<DeviceCodeView> beginMicrosoftLogin() async => DeviceCodeView(
        userCode: 'K7QF-M2XP',
        verificationUri: 'https://www.microsoft.com/link',
        expiresIn: BigInt.from(900),
      );

  @override
  Future<AccountView> completeMicrosoftLogin() => _login.future;

  @override
  void cancelMicrosoftLogin() => loginCancelled = true;

  @override
  Future<AccountView> addOfflineAccount(String name) async {
    final a = AccountView(id: name, name: name, microsoft: false, active: false);
    accounts = [...accounts, a];
    return a;
  }

  @override
  Future<void> setActiveAccount(String id) async {
    accounts = [
      for (final a in accounts)
        AccountView(
          id: a.id,
          name: a.name,
          microsoft: a.microsoft,
          skinUrl: a.skinUrl,
          active: a.id == id,
        ),
    ];
  }

  @override
  Future<void> removeAccount(String id) async =>
      accounts = accounts.where((a) => a.id != id).toList();

  @override
  Future<List<InstanceView>> listInstances() async => instances;

  @override
  Future<InstanceView> createInstance({
    required String name,
    required String mc,
    required String loader,
    String? loaderVersion,
  }) async {
    final i = fakeInstance(
      id: name.toLowerCase().replaceAll(' ', '-'),
      name: name,
      mc: mc,
      loader: loader,
      loaderVersion: loaderVersion,
    );
    instances = [...instances, i];
    return i;
  }

  @override
  Future<InstanceView> updateInstanceSettings({
    required String id,
    required String name,
    int? memoryMb,
    List<String>? jvmArgs,
    int? width,
    int? height,
    required bool fullscreen,
  }) async {
    final old = instances.firstWhere((i) => i.id == id);
    final i = InstanceView(
      id: id,
      name: name,
      mc: old.mc,
      loader: old.loader,
      loaderVersion: old.loaderVersion,
      createdAt: old.createdAt,
      lastPlayed: old.lastPlayed,
      playtimeSeconds: old.playtimeSeconds,
      memoryMb: memoryMb,
      jvmArgs: jvmArgs,
      width: width,
      height: height,
      fullscreen: fullscreen,
      gameDir: old.gameDir,
    );
    instances = [for (final x in instances) x.id == id ? i : x];
    return i;
  }

  @override
  Future<String> deleteInstance(String id) async {
    instances = instances.where((i) => i.id != id).toList();
    return 'trash-$id';
  }

  @override
  Future<List<VersionView>> listVersions({required bool snapshots}) async => [
        if (snapshots) const VersionView(id: '25w14a', kind: 'snapshot'),
        for (final v in [
          '1.21.5',
          '1.21.4',
          '1.21.3',
          '1.21.1',
          '1.21',
          '1.20.6',
          '1.20.4',
          '1.20.1',
          '1.19.4',
          '1.19.2',
          '1.18.2',
          '1.16.5',
        ])
          VersionView(id: v, kind: 'release'),
      ];

  @override
  Future<List<String>> listLoaderVersions(String loader, String mc) async =>
      ['0.16.14', '0.16.13', '0.16.12', '0.16.10'];

  /// Último lanzamiento pedido (para comprobar `server`).
  ({String id, String? account, String? server})? lastLaunch;

  @override
  Future<GameView> launchInstance(
    String id, {
    String? account,
    String? server,
  }) async {
    lastLaunch = (id: id, account: account, server: server);
    if (launchError != null) throw launchError!;
    final c = launchCompleter;
    if (c != null) return c.future;
    return GameView(
      gameId: BigInt.one,
      instanceId: id,
      pid: 4242,
      startedAt: kTestNow.millisecondsSinceEpoch ~/ 1000,
    );
  }

  @override
  Future<List<GameView>> runningGames() async => const [];

  @override
  Future<SettingsView> getSettings() async => SettingsView(
        language: language,
        closeBehavior: 'ask',
        allowMultipleGames: false,
        defaultMemoryMb: 4096,
        defaultJvmArgs: const ['-XX:+UseG1GC'],
        maxConcurrentDownloads: 8,
        discordRpc: false,
        msaClientIdSet: true,
      );

  @override
  Future<SettingsView> saveSettings(SettingsView value) async => value;

  // --- Registros -----------------------------------------------------------

  /// Histórico por partida y última partida por instancia.
  final Map<BigInt, List<LogLineView>> backlogs = {};
  final Map<String, BigInt> latestGames = {};

  @override
  List<LogLineView> logBacklog(BigInt gameId) => backlogs[gameId] ?? const [];

  @override
  BigInt? latestGameFor(String instanceId) => latestGames[instanceId];

  // --- Contenido -----------------------------------------------------------

  List<ProjectView> searchResults = sampleProjects();
  final Map<String, List<InstalledContentView>> content = {
    'survival-amigos': sampleContent(),
  };
  List<ContentUpdateView> updates = sampleUpdates();
  bool curseforgeKey = false;
  final List<String> installed = [];

  /// Se completa a mano para controlar la instalación de modpacks.
  Completer<ModpackInstallView>? modpackCompleter;

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
  }) async {
    if (source == 'curseforge' && !curseforgeKey) {
      throw const BridgeError(kind: 'curseforge_key_missing', message: 'x');
    }
    final all = searchResults
        .where((p) => p.kind == kind || kind == 'mod' && p.kind == null)
        .where((p) => text.isEmpty || p.title.toLowerCase().contains(text.toLowerCase()))
        .toList();
    return SearchPageView(
      items: all.skip(offset).take(limit).toList(),
      offset: offset,
      limit: limit,
      total: all.length,
    );
  }

  @override
  Future<ProjectView> getProject(String source, String project) async =>
      searchResults.firstWhere((p) => p.id == project || p.slug == project);

  @override
  Future<List<ContentVersionView>> contentVersions(
    String source,
    String project, {
    String? mc,
    String? loader,
  }) async =>
      sampleVersions(project, mc: mc ?? '1.21.4', loader: loader);

  @override
  Future<InstallOutcomeView> installContent(
    String instanceId,
    String source,
    String project, {
    String? version,
    String? world,
  }) async {
    installed.add('$instanceId:$project${world == null ? '' : '@$world'}');
    final p = searchResults.firstWhere((x) => x.id == project);
    return InstallOutcomeView(
      installed: [
        InstalledItemView(
          kind: p.kind ?? 'mod',
          projectName: p.title,
          versionNumber: '1.0.0',
          file: '${p.slug}.jar',
          dependency: false,
          alreadyPresent: false,
        ),
      ],
      skipped: const [],
      warnings: const [],
    );
  }

  @override
  Future<List<InstalledContentView>> listContent(String instanceId) async =>
      content[instanceId] ?? const [];

  @override
  Future<InstalledContentView> setContentEnabled(
    String instanceId,
    String file,
    bool enabled,
  ) async {
    final list = content[instanceId]!;
    final i = list.indexWhere((c) => '${c.kind}/${c.file}' == file);
    final c = list[i];
    final n = InstalledContentView(
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
    list[i] = n;
    return n;
  }

  @override
  Future<InstalledContentView> removeContent(String instanceId, String file) async {
    final list = content[instanceId]!;
    final c = list.firstWhere((c) => '${c.kind}/${c.file}' == file);
    list.remove(c);
    return c;
  }

  @override
  Future<List<ContentUpdateView>> checkContentUpdates(String instanceId) async =>
      instanceId == 'survival-amigos' ? updates : const [];

  @override
  Future<String?> updateContent(String instanceId, String file) async {
    final u = updates.where((u) => '${u.kind}/${u.file}' == file).firstOrNull;
    updates = updates.where((x) => x != u).toList();
    return u?.latestVersion;
  }

  @override
  Future<IdentifyView> identifyContent(String instanceId) async =>
      const IdentifyView(identified: ['Lithium'], unmatched: ['mi-mod-casero.jar']);

  @override
  Future<ModpackInstallView> installModpack(String source, {String? name}) {
    final c = modpackCompleter;
    if (c != null) return c.future;
    final i = fakeInstance(id: 'modpack', name: name ?? 'Modpack', mc: '1.20.1', loader: 'fabric');
    instances = [...instances, i];
    return Future.value(ModpackInstallView(
      instance: i,
      filesTotal: 212,
      filesDownloaded: 212,
      warnings: const [],
    ));
  }

  @override
  Future<bool> hasCurseforgeKey() async => curseforgeKey;

  @override
  Future<void> setCurseforgeKey(String key) async => curseforgeKey = true;

  @override
  Future<void> clearCurseforgeKey() async => curseforgeKey = false;

  @override
  Future<String?> cachedImage(String url) async => null;

  // --- Mundos y capturas ---------------------------------------------------

  final Map<String, List<WorldView>> worlds = {'survival-amigos': sampleWorlds()};
  final Map<String, List<DatapackView>> datapacks = {
    'Mundo de verano': sampleDatapacks(),
  };
  List<ScreenshotView> screenshots = sampleScreenshots();

  @override
  Future<List<WorldView>> listWorlds(String instanceId) async =>
      worlds[instanceId] ?? const [];

  WorldView _patchWorld(
    String instanceId,
    String folder,
    WorldView Function(WorldView) f,
  ) {
    final list = worlds[instanceId]!;
    final i = list.indexWhere((w) => w.folder == folder);
    return list[i] = f(list[i]);
  }

  @override
  Future<WorldView> renameWorld(String instanceId, String folder, String name) async =>
      _patchWorld(instanceId, folder, (w) => copyWorld(w, name: name));

  @override
  Future<WorldView> updateWorld(
    String instanceId,
    String folder, {
    String? gameMode,
    String? difficulty,
    bool? hardcore,
    bool? cheats,
  }) async =>
      _patchWorld(
        instanceId,
        folder,
        (w) => copyWorld(
          w,
          gameMode: gameMode,
          difficulty: difficulty,
          hardcore: hardcore,
          cheats: cheats,
        ),
      );

  @override
  Future<void> deleteWorld(String instanceId, String folder) async =>
      worlds[instanceId]!.removeWhere((w) => w.folder == folder);

  @override
  Future<List<DatapackView>> listDatapacks(String instanceId, String world) async =>
      datapacks[world] ?? const [];

  @override
  Future<void> setDatapackEnabled(
    String instanceId,
    String world,
    String name,
    bool enabled,
  ) async {
    final list = datapacks[world]!;
    final i = list.indexWhere((d) => d.name == name);
    final d = list[i];
    list[i] = DatapackView(
      name: d.name,
      isDir: d.isDir,
      enabled: enabled,
      sizeBytes: d.sizeBytes,
    );
  }

  @override
  Future<void> removeDatapack(String instanceId, String world, String name) async =>
      datapacks[world]!.removeWhere((d) => d.name == name);

  @override
  Future<List<ScreenshotView>> listScreenshots(String instanceId) async =>
      instanceId == 'survival-amigos' ? screenshots : const [];

  @override
  Future<void> deleteScreenshot(String instanceId, String file) async =>
      screenshots = screenshots.where((s) => s.file != file).toList();

  // --- Copias de seguridad -------------------------------------------------

  List<BackupView> backups = sampleBackups();
  BackupSettingsView backupSettings =
      const BackupSettingsView(auto: true, intervalHours: 24, maxCount: 5);
  final List<String> restored = [];

  @override
  Future<List<BackupView>> listBackups(String instanceId) async =>
      instanceId == 'survival-amigos' ? backups : const [];

  @override
  Future<BackupView> createBackup(
    String instanceId, {
    String? label,
    bool pinned = false,
  }) async {
    final b = BackupView(
      id: 'backup-${backups.length + 100}',
      createdAt: kTestNow.millisecondsSinceEpoch ~/ 1000,
      sizeBytes: BigInt.from(512 * 1024 * 1024),
      pinned: pinned,
      label: label,
      automatic: false,
    );
    backups = [b, ...backups];
    return b;
  }

  @override
  Future<void> deleteBackup(String instanceId, String backupId) async =>
      backups = backups.where((b) => b.id != backupId).toList();

  BackupView _patchBackup(String id, BackupView Function(BackupView) f) {
    final i = backups.indexWhere((b) => b.id == id);
    final n = f(backups[i]);
    backups = [...backups]..[i] = n;
    return n;
  }

  @override
  Future<BackupView> setBackupPinned(
    String instanceId,
    String backupId,
    bool pinned,
  ) async =>
      _patchBackup(
        backupId,
        (b) => BackupView(
          id: b.id,
          createdAt: b.createdAt,
          sizeBytes: b.sizeBytes,
          pinned: pinned,
          label: b.label,
          automatic: b.automatic,
        ),
      );

  @override
  Future<BackupView> setBackupLabel(
    String instanceId,
    String backupId,
    String? label,
  ) async =>
      _patchBackup(
        backupId,
        (b) => BackupView(
          id: b.id,
          createdAt: b.createdAt,
          sizeBytes: b.sizeBytes,
          pinned: b.pinned,
          label: label,
          automatic: b.automatic,
        ),
      );

  @override
  Future<RestoreView> restoreBackup(
    String instanceId,
    String backupId, {
    required bool worldsOnly,
  }) async {
    restored.add('$backupId:${worldsOnly ? 'worlds' : 'full'}');
    return RestoreView(files: BigInt.from(1834), notes: const []);
  }

  @override
  Future<BackupSettingsView> getBackupSettings(String instanceId) async =>
      backupSettings;

  @override
  Future<BackupSettingsView> updateBackupSettings(
    String instanceId, {
    required bool auto,
    required int intervalHours,
    required int maxCount,
  }) async =>
      backupSettings = BackupSettingsView(
        auto: auto,
        intervalHours: intervalHours,
        maxCount: maxCount,
      );

  // --- Servidores ----------------------------------------------------------

  List<ServerView> servers = sampleServers();
  Map<String, ServerStatusView> statuses = sampleStatuses();

  @override
  Future<List<ServerView>> listServers() async => servers;

  @override
  Future<ServerView> addServer({
    required String name,
    required String address,
    String? instanceId,
    String? accountId,
  }) async {
    final s = ServerView(
      id: 's${servers.length + 1}',
      name: name.isEmpty ? address : name,
      address: address,
      instanceId: instanceId,
      accountId: accountId,
    );
    servers = [...servers, s];
    return s;
  }

  @override
  Future<ServerView> updateServer({
    required String id,
    required String name,
    required String address,
    String? instanceId,
    String? accountId,
  }) async {
    final s = ServerView(
      id: id,
      name: name,
      address: address,
      instanceId: instanceId,
      accountId: accountId,
    );
    servers = [for (final x in servers) x.id == id ? s : x];
    return s;
  }

  @override
  Future<void> removeServer(String id) async =>
      servers = servers.where((s) => s.id != id).toList();

  @override
  Future<List<ServerView>> moveServer(String id, int index) async {
    final list = [...servers];
    final s = list.removeAt(list.indexWhere((x) => x.id == id));
    list.insert(index.clamp(0, list.length), s);
    return servers = list;
  }

  @override
  Future<int> importServersDat(String instanceId) async => 0;

  @override
  Future<List<ServerStatusView>> pingServers(List<String> addresses) async => [
        for (final a in addresses)
          statuses[a] ??
              ServerStatusView(
                address: a,
                online: false,
                latencyMs: BigInt.zero,
                versionName: '',
                playersOnline: 0,
                playersMax: 0,
                sample: const [],
                motd: '',
                error: 'connection refused',
              ),
      ];

  Future<void> close() => _events.close();
}

// --- Datos de ejemplo --------------------------------------------------------

InstalledContentView _c(
  String kind,
  String file, {
  bool enabled = true,
  String source = 'modrinth',
  String? name,
  String? version,
  int kb = 512,
  String? world,
}) =>
    InstalledContentView(
      kind: kind,
      file: file,
      enabled: enabled,
      source: source,
      projectId: source == 'unknown' ? null : file.split(RegExp(r'[-_.]')).first,
      projectName: name,
      versionNumber: version,
      size: BigInt.from(kb * 1024),
      isDir: false,
      tracked: source != 'unknown',
      world: world,
    );

List<InstalledContentView> sampleContent() => [
      _c('mod', 'fabric-api-0.119.2+1.21.4.jar', name: 'Fabric API', version: '0.119.2+1.21.4', kb: 2210),
      _c('mod', 'sodium-fabric-0.6.13+mc1.21.4.jar', name: 'Sodium', version: '0.6.13+mc1.21.4', kb: 1180),
      _c('mod', 'iris-fabric-1.8.8+mc1.21.4.jar', name: 'Iris Shaders', version: '1.8.8+mc1.21.4', kb: 2780),
      _c('mod', 'lithium-fabric-0.14.7+mc1.21.4.jar', name: 'Lithium', version: '0.14.7', kb: 760),
      _c('mod', 'journeymap-1.21.4-6.0.0.jar', source: 'curseforge', name: 'JourneyMap', version: '6.0.0', kb: 6020),
      _c('mod', 'appleskin-fabric-mc1.21.4-3.0.6.jar', name: 'AppleSkin', version: '3.0.6', kb: 120, enabled: false),
      _c('mod', 'mi-mod-casero.jar', source: 'unknown', kb: 48),
      _c('resourcepack', 'FreshAnimations_v1.9.3.zip', name: 'Fresh Animations', version: '1.9.3', kb: 5400),
      _c('shader', 'ComplementaryReimagined_r5.4.zip', name: 'Complementary Shaders - Reimagined', version: 'r5.4', kb: 900),
    ];

List<ContentUpdateView> sampleUpdates() => const [
      ContentUpdateView(
        kind: 'mod',
        file: 'sodium-fabric-0.6.13+mc1.21.4.jar',
        projectId: 'sodium',
        projectName: 'Sodium',
        currentVersion: '0.6.13+mc1.21.4',
        latestVersion: '0.6.14+mc1.21.4',
      ),
      ContentUpdateView(
        kind: 'resourcepack',
        file: 'FreshAnimations_v1.9.3.zip',
        projectId: 'fresh-animations',
        projectName: 'Fresh Animations',
        currentVersion: '1.9.3',
        latestVersion: '1.9.4',
      ),
    ];

ProjectView _p(
  String id,
  String title,
  String author,
  String description,
  int downloads, {
  String kind = 'mod',
  List<String> categories = const [],
}) =>
    ProjectView(
      source: 'modrinth',
      id: id,
      slug: id,
      kind: kind,
      title: title,
      description: description,
      author: author,
      downloads: BigInt.from(downloads),
      follows: BigInt.from(downloads ~/ 90),
      categories: categories,
      dateModified: '2026-09-28T12:00:00Z',
    );

List<ProjectView> sampleProjects() => [
      _p('sodium', 'Sodium', 'jellysquid3', 'El motor de renderizado más rápido para Minecraft, con muchas más imágenes por segundo.', 98400000, categories: ['optimización']),
      _p('lithium', 'Lithium', 'jellysquid3', 'Optimiza la física, la IA de las criaturas y la generación sin cambiar el juego.', 51200000, categories: ['optimización']),
      _p('iris', 'Iris Shaders', 'coderbot', 'Shaders modernos compatibles con OptiFine y con Sodium.', 77300000, categories: ['decoración']),
      _p('entityculling', 'Entity Culling', 'tr7zw', 'Deja de dibujar entidades y bloques que no se ven.', 31900000, categories: ['optimización']),
      _p('modmenu', 'Mod Menu', 'Prospector', 'Añade un menú de mods para ver y configurar lo que tienes instalado.', 64100000, categories: ['utilidad']),
      _p('ferrite-core', 'FerriteCore', 'malte0811', 'Reduce el uso de memoria del juego.', 45800000, categories: ['optimización']),
      _p('fresh-animations', 'Fresh Animations', 'FreshLX', 'Animaciones nuevas para las criaturas del juego.', 12300000, kind: 'resourcepack', categories: ['animaciones']),
      _p('complementary', 'Complementary Shaders', 'EminGT', 'Shaders bonitos y equilibrados.', 18800000, kind: 'shader', categories: ['realista']),
      _p('terralith', 'Terralith', 'Starmute', 'Más de 95 biomas nuevos sin bloques nuevos.', 9100000, kind: 'datapack', categories: ['generación']),
      _p('fabulously-optimized', 'Fabulously Optimized', 'robotkoer', 'Modpack de rendimiento con buenos gráficos.', 13200000, kind: 'modpack', categories: ['optimización']),
    ];

List<ContentVersionView> sampleVersions(String project, {required String mc, String? loader}) => [
      for (final (i, (v, rel, days)) in [
        ('0.6.14', 'release', 4),
        ('0.6.13', 'release', 26),
        ('0.6.12-beta.2', 'beta', 41),
        ('0.6.11', 'release', 63),
      ].indexed)
        ContentVersionView(
          source: 'modrinth',
          id: '$project-v$i',
          projectId: project,
          name: '$project $v',
          versionNumber: v,
          mcVersions: [mc],
          loaders: [loader ?? 'fabric'],
          release: rel,
          datePublished: DateTime(2026, 10, 11)
              .subtract(Duration(days: days))
              .toIso8601String(),
          fileName: '$project-$v.jar',
          fileUrl: 'https://cdn.modrinth.com/data/$project/$v.jar',
          fileSize: BigInt.from(1200000 - i * 40000),
        ),
    ];

WorldView copyWorld(
  WorldView w, {
  String? name,
  String? gameMode,
  String? difficulty,
  bool? hardcore,
  bool? cheats,
}) =>
    WorldView(
      folder: w.folder,
      name: name ?? w.name,
      gameMode: gameMode ?? w.gameMode,
      difficulty: difficulty ?? w.difficulty,
      hardcore: hardcore ?? w.hardcore,
      cheats: cheats ?? w.cheats,
      lastPlayedMs: w.lastPlayedMs,
      versionName: w.versionName,
      sizeBytes: w.sizeBytes,
      icon: w.icon,
      error: w.error,
    );

int _agoMs(Duration d) => kTestNow.subtract(d).millisecondsSinceEpoch;

List<WorldView> sampleWorlds() => [
      WorldView(
        folder: 'Mundo de verano',
        name: 'Mundo de verano',
        gameMode: 'survival',
        difficulty: 'normal',
        hardcore: false,
        cheats: false,
        lastPlayedMs: _agoMs(const Duration(hours: 2)),
        versionName: '1.21.4',
        sizeBytes: BigInt.from(842 * 1024 * 1024),
      ),
      WorldView(
        folder: 'Creativo-pruebas',
        name: 'Creativo de pruebas',
        gameMode: 'creative',
        difficulty: 'peaceful',
        hardcore: false,
        cheats: true,
        lastPlayedMs: _agoMs(const Duration(days: 5)),
        versionName: '1.21.4',
        sizeBytes: BigInt.from(96 * 1024 * 1024),
      ),
      WorldView(
        folder: 'Hardcore 2',
        name: 'Hardcore 2',
        gameMode: 'survival',
        difficulty: 'hard',
        hardcore: true,
        cheats: false,
        lastPlayedMs: _agoMs(const Duration(days: 40)),
        versionName: '1.21.1',
        sizeBytes: BigInt.from(311 * 1024 * 1024),
      ),
    ];

List<DatapackView> sampleDatapacks() => [
      DatapackView(name: 'Terralith_1.21_v2.5.8.zip', isDir: false, enabled: true, sizeBytes: BigInt.from(2900000)),
      DatapackView(name: 'mas-aldeas', isDir: true, enabled: true, sizeBytes: BigInt.from(410000)),
      DatapackView(name: 'sin-phantoms.zip', isDir: false, enabled: false, sizeBytes: BigInt.from(12000)),
    ];

List<ScreenshotView> sampleScreenshots() => [
      for (var i = 0; i < 9; i++)
        ScreenshotView(
          file: '2026-10-${(11 - i).toString().padLeft(2, '0')}_18.0$i.21.png',
          path: '/tmp/shard-test/screenshots/shot-$i.png',
          size: BigInt.from(2400000 + i * 10000),
          modified: kTestNow.subtract(Duration(hours: 5 + i * 19)).millisecondsSinceEpoch ~/ 1000,
        ),
    ];

List<BackupView> sampleBackups() {
  int ago(Duration d) => kTestNow.subtract(d).millisecondsSinceEpoch ~/ 1000;
  const mb = 1024 * 1024;
  return [
    BackupView(id: 'backup-4', createdAt: ago(const Duration(hours: 3)), sizeBytes: BigInt.from(1240 * mb), pinned: false, automatic: true),
    BackupView(id: 'backup-3', createdAt: ago(const Duration(days: 2)), sizeBytes: BigInt.from(1190 * mb), pinned: true, label: 'Antes de la actualización 1.21.4', automatic: false),
    BackupView(id: 'backup-2', createdAt: ago(const Duration(days: 3)), sizeBytes: BigInt.from(1150 * mb), pinned: false, automatic: true),
    BackupView(id: 'backup-1', createdAt: ago(const Duration(days: 21)), sizeBytes: BigInt.from(870 * mb), pinned: false, label: 'Base terminada', automatic: false),
  ];
}

List<ServerView> sampleServers() => const [
      ServerView(id: 's1', name: 'Servidor de los amigos', address: 'mc.amigos.example', instanceId: 'survival-amigos'),
      ServerView(id: 's2', name: 'Hypixel', address: 'mc.hypixel.net', instanceId: 'vanilla-puro'),
      ServerView(id: 's3', name: 'Create SMP', address: 'create.smp.example:25570', instanceId: 'create-aeronautica'),
      ServerView(id: 's4', name: 'Servidor de pruebas', address: 'localhost'),
    ];

Map<String, ServerStatusView> sampleStatuses() => {
      'mc.amigos.example': ServerStatusView(
        address: 'mc.amigos.example',
        online: true,
        latencyMs: BigInt.from(23),
        versionName: '1.21.4',
        playersOnline: 3,
        playersMax: 20,
        sample: const ['NadiaK', 'Rulo', 'Mai'],
        motd: 'Temporada 4 · ¡Nueva aldea en el spawn!',
      ),
      'mc.hypixel.net': ServerStatusView(
        address: 'mc.hypixel.net',
        online: true,
        latencyMs: BigInt.from(118),
        versionName: 'Requires MC 1.8 / 1.21',
        playersOnline: 31842,
        playersMax: 200000,
        sample: const [],
        motd: 'Hypixel Network  [1.8-1.21]',
      ),
      'create.smp.example:25570': ServerStatusView(
        address: 'create.smp.example:25570',
        online: true,
        latencyMs: BigInt.from(64),
        versionName: '1.20.1',
        playersOnline: 0,
        playersMax: 12,
        sample: const [],
        motd: 'Create SMP — trenes, molinos y mucha paciencia',
      ),
    };
