// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get navHome => 'Home';

  @override
  String get navInstances => 'Instances';

  @override
  String get navContent => 'Content';

  @override
  String get navServers => 'Servers';

  @override
  String get navSkins => 'Skins';

  @override
  String get navSettings => 'Settings';

  @override
  String get comingSoonTitle => 'Coming soon';

  @override
  String get comingSoonContent =>
      'Search mods, resource packs, shaders and modpacks from Modrinth and CurseForge.';

  @override
  String get comingSoonServers =>
      'Your servers with status, players and ping, ready to join with the instance you pick.';

  @override
  String get comingSoonSkins =>
      '3D view of your skin, upload a new one and manage capes.';

  @override
  String get windowMinimize => 'Minimize';

  @override
  String get windowMaximize => 'Maximize';

  @override
  String get windowRestore => 'Restore';

  @override
  String get windowClose => 'Close';

  @override
  String closeDialogTitle(Object app) {
    return 'Close $app?';
  }

  @override
  String get closeDialogBody =>
      'You can keep it in the background or quit completely. Open games keep running.';

  @override
  String get closeDialogMinimize => 'Keep in background';

  @override
  String get closeDialogQuit => 'Quit';

  @override
  String get closeDialogRemember => 'Remember my choice';

  @override
  String get cancel => 'Cancel';

  @override
  String get save => 'Save';

  @override
  String get create => 'Create';

  @override
  String get delete => 'Delete';

  @override
  String get remove => 'Remove';

  @override
  String get copy => 'Copy';

  @override
  String get copied => 'Copied';

  @override
  String get retry => 'Retry';

  @override
  String get close => 'Close';

  @override
  String get back => 'Back';

  @override
  String get discard => 'Discard';

  @override
  String get details => 'Details';

  @override
  String get loading => 'Loading…';

  @override
  String get accountsTitle => 'Accounts';

  @override
  String get accountsEmpty => 'No accounts yet';

  @override
  String get accountsAddMicrosoft => 'Add Microsoft account';

  @override
  String get accountsAddOffline => 'Add offline account';

  @override
  String get accountMicrosoft => 'Microsoft';

  @override
  String get accountOffline => 'Offline';

  @override
  String get accountActive => 'Active';

  @override
  String get accountUse => 'Use this account';

  @override
  String accountRemoveTitle(Object name) {
    return 'Remove “$name”?';
  }

  @override
  String accountRemoveBody(Object app) {
    return 'It is removed from $app along with its session. You can add it again anytime.';
  }

  @override
  String accountRemoved(Object name) {
    return 'Account “$name” removed';
  }

  @override
  String accountAdded(Object name) {
    return 'Account “$name” added';
  }

  @override
  String get accountNone => 'No account';

  @override
  String get accountNoneHint => 'Add an account to play';

  @override
  String get offlineTitle => 'Offline account';

  @override
  String get offlineBody =>
      'For offline play or servers that don’t verify accounts. Skins can’t be changed.';

  @override
  String get offlineNameLabel => 'Player name';

  @override
  String get offlineNameHint => '3–16 letters, digits or _';

  @override
  String get offlineNameInvalid =>
      'Use 3–16 characters: letters, digits or underscore.';

  @override
  String get add => 'Add';

  @override
  String get msTitle => 'Sign in with Microsoft';

  @override
  String get msRequesting => 'Requesting a code from Microsoft…';

  @override
  String get msStep => 'Open the Microsoft page and enter this code:';

  @override
  String get msOpenBrowser => 'Open browser';

  @override
  String get msCopyCode => 'Copy code';

  @override
  String get msWaiting => 'Waiting for confirmation…';

  @override
  String msExpiresIn(Object time) {
    return 'Expires in $time';
  }

  @override
  String get msExpired => 'The code has expired.';

  @override
  String get msRequestNew => 'Request a new code';

  @override
  String msSignedIn(Object name) {
    return 'Signed in as $name';
  }

  @override
  String get errAuthMissingClientId =>
      'The Microsoft application ID is missing, so signing in is not possible. Configure it in the core or use an offline account.';

  @override
  String get errAuthAppNotApproved =>
      'Microsoft has not yet approved this application to use Minecraft services.';

  @override
  String get errAuthNotEntitled =>
      'This Microsoft account doesn’t own Minecraft: Java Edition.';

  @override
  String get errAuthNoProfile =>
      'The account owns the game but has no profile yet. Sign in once at minecraft.net to choose your name.';

  @override
  String get errAuthXbox =>
      'Xbox Live rejected the sign-in (child account, region, or no Xbox profile yet).';

  @override
  String get errAuthDeclined =>
      'The sign-in was declined on the Microsoft page.';

  @override
  String get errAuthExpired =>
      'The code expired before finishing. Request a new one.';

  @override
  String get errAuthSessionExpired =>
      'This account’s session expired. Sign in again.';

  @override
  String get errAuthGeneric => 'Couldn’t sign in with Microsoft.';

  @override
  String get errNetwork => 'No connection, or the server is not responding.';

  @override
  String get errOffline => 'Offline, and what’s needed isn’t cached.';

  @override
  String get errJavaRequired =>
      'The Java version this instance needs is missing.';

  @override
  String get errGameAlreadyRunning =>
      'A game is already running. Turn on “Allow multiple games” in Settings to open another.';

  @override
  String get errInstanceNotFound => 'The instance no longer exists.';

  @override
  String get errAccountNotFound =>
      'There’s no account to play with. Add one from the avatar at the bottom left.';

  @override
  String get errVersionNotFound => 'That version doesn’t exist.';

  @override
  String get errHashMismatch =>
      'A downloaded file arrived corrupted. Try again.';

  @override
  String get errIo => 'Couldn’t read from or write to disk.';

  @override
  String get errInvalidInput => 'Some input is not valid.';

  @override
  String get errGeneric => 'Something went wrong.';

  @override
  String homeGreeting(Object name) {
    return 'Hi, $name';
  }

  @override
  String get homeGreetingAnon => 'Welcome';

  @override
  String get homeSubtitle => 'What are we playing today?';

  @override
  String get homeContinue => 'Continue';

  @override
  String get homeNeverPlayed => 'Not played yet';

  @override
  String get homeChangeInstance => 'Switch instance';

  @override
  String get homeRecent => 'Recent';

  @override
  String get homeSeeAll => 'See all';

  @override
  String get homeNoInstancesTitle => 'Create your first instance';

  @override
  String get homeNoInstancesBody =>
      'An instance is a game folder with its own version, mods and worlds, kept apart from the others.';

  @override
  String get homeSession => 'Session';

  @override
  String get homeChangeAccount => 'Change';

  @override
  String get homeQuickServers => 'Quick servers';

  @override
  String get homeQuickServersSoon =>
      'Soon you’ll be able to join your favourite servers from here.';

  @override
  String get homeRunningNow => 'Running now';

  @override
  String get homeNothingRunning => 'No games running';

  @override
  String get play => 'Play';

  @override
  String get launchPreparing => 'Preparing…';

  @override
  String launchDownloading(int done, int total) {
    return 'Downloading $done/$total';
  }

  @override
  String get launchStarting => 'Starting…';

  @override
  String get launchRunning => 'Playing';

  @override
  String get launchFailed => 'Couldn’t launch';

  @override
  String get launchCrashed => 'The game crashed';

  @override
  String launchExitCode(int code) {
    return 'The game exited with code $code.';
  }

  @override
  String launchCrashReport(Object path) {
    return 'Crash report: $path';
  }

  @override
  String get launchSeeLogs => 'View logs';

  @override
  String get stageLibraries => 'Libraries';

  @override
  String get stageAssets => 'Assets';

  @override
  String get stageOther => 'Files';

  @override
  String playtimeHours(int h) {
    return '$h h';
  }

  @override
  String playtimeMinutes(int m) {
    return '$m min';
  }

  @override
  String playtimeHoursMinutes(int h, int m) {
    return '$h h $m min';
  }

  @override
  String get playtimeNone => '0 min';

  @override
  String get lastPlayedNever => 'Never';

  @override
  String get relJustNow => 'Just now';

  @override
  String relMinutes(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n minutes ago',
      one: '1 minute ago',
    );
    return '$_temp0';
  }

  @override
  String relHours(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n hours ago',
      one: '1 hour ago',
    );
    return '$_temp0';
  }

  @override
  String relDays(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n days ago',
      one: 'yesterday',
    );
    return '$_temp0';
  }

  @override
  String relMonths(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n months ago',
      one: '1 month ago',
    );
    return '$_temp0';
  }

  @override
  String relYears(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n years ago',
      one: '1 year ago',
    );
    return '$_temp0';
  }

  @override
  String get instancesTitle => 'Instances';

  @override
  String instancesCount(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n instances',
      one: '1 instance',
      zero: 'No instances',
    );
    return '$_temp0';
  }

  @override
  String get instancesSearch => 'Search by name, version or loader';

  @override
  String get instancesNew => 'New instance';

  @override
  String get instancesEmptyTitle => 'No instances yet';

  @override
  String get instancesEmptyBody =>
      'Create one to choose a Minecraft version and loader.';

  @override
  String instancesNoResults(Object q) {
    return 'Nothing matches “$q”';
  }

  @override
  String get viewGrid => 'Grid';

  @override
  String get viewList => 'List';

  @override
  String get instancePlaytime => 'Playtime';

  @override
  String get instanceLastPlayed => 'Last played';

  @override
  String get instanceOpenFolder => 'Open folder';

  @override
  String get instanceDelete => 'Delete instance';

  @override
  String instanceDeleteTitle(Object name) {
    return 'Delete “$name”?';
  }

  @override
  String instanceDeleteBody(Object app) {
    return 'It will be moved to the $app trash, where you can restore it for a while.';
  }

  @override
  String instanceDeleted(Object name) {
    return '“$name” was moved to the trash';
  }

  @override
  String get instanceMore => 'More options';

  @override
  String get instanceNotFound => 'This instance no longer exists.';

  @override
  String get tabContent => 'Content';

  @override
  String get tabWorlds => 'Worlds';

  @override
  String get tabScreenshots => 'Screenshots';

  @override
  String get tabBackups => 'Backups';

  @override
  String get tabLogs => 'Logs';

  @override
  String get tabSettings => 'Settings';

  @override
  String get tabSoonContent =>
      'This instance’s mods, resource packs and shaders: enable, remove, update and identify.';

  @override
  String get tabSoonWorlds =>
      'World list with details, datapacks and options to edit or delete them.';

  @override
  String get tabSoonScreenshots =>
      'Gallery of the screenshots you take in game with F2.';

  @override
  String get tabSoonBackups =>
      'Manual and automatic backups, pin and restore them.';

  @override
  String get createTitle => 'New instance';

  @override
  String get createName => 'Name';

  @override
  String get createNameHint => 'My survival world';

  @override
  String get createVersion => 'Minecraft version';

  @override
  String get createSnapshots => 'Snapshots';

  @override
  String get createSearchVersion => 'Search version';

  @override
  String get createLoader => 'Loader';

  @override
  String get createLoaderVersion => 'Loader version';

  @override
  String get createLoaderLatest => 'Latest';

  @override
  String createNoLoaderVersions(Object loader, Object mc) {
    return 'There are no $loader versions for $mc.';
  }

  @override
  String get createNoVersions => 'No versions found.';

  @override
  String get createNameRequired => 'Give it a name.';

  @override
  String createDone(Object name) {
    return 'Instance “$name” created';
  }

  @override
  String get createLatestRelease => 'Latest';

  @override
  String get loaderVanilla => 'Vanilla';

  @override
  String get loaderCustom => 'Custom';

  @override
  String get isGeneral => 'General';

  @override
  String get isName => 'Name';

  @override
  String get isMemory => 'Memory';

  @override
  String get isMemoryDesc => 'Maximum RAM for the game.';

  @override
  String get isUseGlobal => 'Use the global value';

  @override
  String isMemoryGlobal(Object value) {
    return 'Global: $value';
  }

  @override
  String isMemoryWarning(Object total) {
    return 'Over 75% of this computer’s RAM ($total); the system could slow down.';
  }

  @override
  String get isJvm => 'JVM arguments';

  @override
  String get isJvmDesc => 'Extra Java options, separated by spaces.';

  @override
  String get isWindow => 'Game window';

  @override
  String get isResolution => 'Resolution';

  @override
  String get isResolutionDesc => 'Empty = the game’s default size.';

  @override
  String get isWidth => 'Width';

  @override
  String get isHeight => 'Height';

  @override
  String get isFullscreen => 'Fullscreen';

  @override
  String get isFullscreenDesc => 'Open the game in fullscreen.';

  @override
  String get saved => 'Changes saved';

  @override
  String get unsavedChanges => 'You have unsaved changes';

  @override
  String get consoleFilter => 'Filter lines';

  @override
  String get consoleFollow => 'Follow output';

  @override
  String get consoleCopy => 'Copy';

  @override
  String get consoleClear => 'Clear';

  @override
  String get consoleEmptyTitle => 'No logs yet';

  @override
  String get consoleEmptyBody =>
      'Launch the instance to see the game output here.';

  @override
  String consoleCopied(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n lines copied',
      one: '1 line copied',
    );
    return '$_temp0';
  }

  @override
  String consoleLines(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n lines',
      one: '1 line',
    );
    return '$_temp0';
  }

  @override
  String get consoleNoMatches => 'No lines match the filter.';

  @override
  String get logLevelError => 'Errors';

  @override
  String get logLevelWarn => 'Warnings';

  @override
  String get logLevelInfo => 'Info';

  @override
  String get logLevelDebug => 'Debug';

  @override
  String get settingsTitle => 'Settings';

  @override
  String get settingsGeneral => 'General';

  @override
  String get settingsLanguage => 'Language';

  @override
  String get settingsLanguageDesc => 'Interface language.';

  @override
  String get languageSystem => 'System';

  @override
  String get settingsClose => 'When closing the window';

  @override
  String get settingsCloseDesc => 'What to do when pressing the close button.';

  @override
  String get closeAsk => 'Ask';

  @override
  String get closeTray => 'Background';

  @override
  String get closeQuit => 'Quit';

  @override
  String get settingsMultiple => 'Allow multiple games';

  @override
  String get settingsMultipleDesc => 'Run more than one instance at a time.';

  @override
  String get settingsGame => 'Game';

  @override
  String get settingsMemory => 'Default memory';

  @override
  String settingsMemoryDesc(Object total) {
    return 'For instances without their own value. This computer has $total.';
  }

  @override
  String get settingsJvm => 'Default JVM arguments';

  @override
  String get settingsJvmDesc => 'Separated by spaces.';

  @override
  String get settingsDownloads => 'Downloads';

  @override
  String get settingsConcurrency => 'Concurrent downloads';

  @override
  String get settingsConcurrencyDesc =>
      'With a good connection, more is faster.';

  @override
  String get settingsAbout => 'About';

  @override
  String settingsAboutBody(Object version) {
    return 'A personal Minecraft launcher. Core $version.';
  }

  @override
  String get settingsDataDir => 'Data folder';

  @override
  String get settingsMsa => 'Microsoft sign-in';

  @override
  String get msaConfigured => 'Available';

  @override
  String get msaNotConfigured => 'Not configured';

  @override
  String get settingsSaveFailed => 'Couldn’t save settings';

  @override
  String memoryGb(Object value) {
    return '$value GB';
  }
}
