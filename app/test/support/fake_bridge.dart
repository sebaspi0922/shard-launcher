// Puente falso con datos fijos para tests y goldens.

import 'dart:async';

import 'package:lcore_app/core/bridge.dart';

int _ago(Duration d) =>
    DateTime.now().subtract(d).millisecondsSinceEpoch ~/ 1000;

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

  @override
  Future<GameView> launchInstance(String id, {String? account}) async {
    if (launchError != null) throw launchError!;
    final c = launchCompleter;
    if (c != null) return c.future;
    return GameView(
      gameId: BigInt.one,
      instanceId: id,
      pid: 4242,
      startedAt: DateTime.now().millisecondsSinceEpoch ~/ 1000,
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

  Future<void> close() => _events.close();
}
