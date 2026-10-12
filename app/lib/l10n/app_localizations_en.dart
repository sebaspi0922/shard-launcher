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

  @override
  String get openFolder => 'Open folder';

  @override
  String get previous => 'Previous';

  @override
  String get next => 'Next';

  @override
  String get install => 'Install';

  @override
  String get unknownValue => 'Unknown';

  @override
  String get closeGameFirst => 'Close this instance\'s game first';

  @override
  String get filterAll => 'All';

  @override
  String get filterAny => 'Any';

  @override
  String get filterVersion => 'Version';

  @override
  String get filterLoader => 'Loader';

  @override
  String get sourceUnknown => 'Unknown';

  @override
  String byAuthor(Object author) {
    return 'by $author';
  }

  @override
  String get errCurseforgeKey =>
      'The CurseForge API key is missing. Add it in Settings › Integrations.';

  @override
  String get errSecrets => 'The system keyring could not be used.';

  @override
  String get kindMods => 'Mods';

  @override
  String get kindResourcepacks => 'Resource packs';

  @override
  String get kindShaders => 'Shaders';

  @override
  String get kindDatapacks => 'Datapacks';

  @override
  String get kindModpacks => 'Modpacks';

  @override
  String get kindMod => 'Mod';

  @override
  String get kindResourcepack => 'Resource pack';

  @override
  String get kindShader => 'Shader';

  @override
  String get kindDatapack => 'Datapack';

  @override
  String get kindModpack => 'Modpack';

  @override
  String get contentAdd => 'Add';

  @override
  String get contentBrowse => 'Browse content';

  @override
  String get contentCheckUpdates => 'Check for updates';

  @override
  String get contentFilterHint => 'Filter…';

  @override
  String get contentEnable => 'Enable';

  @override
  String get contentDisable => 'Disable';

  @override
  String get contentDisabled => 'Disabled';

  @override
  String get contentEmptyTitle => 'No content yet';

  @override
  String get contentEmptyBody =>
      'Add mods, resource packs or shaders from Modrinth or CurseForge.';

  @override
  String get contentNoMatches => 'Nothing matches the filter.';

  @override
  String get contentIdentify => 'Identify';

  @override
  String get contentUpdateAll => 'Update all';

  @override
  String contentUpdatesAvailable(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count updates available',
      one: '1 update available',
    );
    return '$_temp0';
  }

  @override
  String contentUnknownFiles(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count files of unknown origin',
      one: '1 file of unknown origin',
    );
    return '$_temp0';
  }

  @override
  String contentUpdateTo(Object version) {
    return 'Update to $version';
  }

  @override
  String contentUpdated(Object name, Object version) {
    return '$name updated to $version';
  }

  @override
  String contentUpdatedAll(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count files updated',
      one: '1 file updated',
    );
    return '$_temp0';
  }

  @override
  String contentUpdatedSome(int ok, int failed) {
    return '$ok updated, $failed failed';
  }

  @override
  String contentIdentified(int found, int missing) {
    return 'Identified: $found. Not recognized: $missing.';
  }

  @override
  String contentRemoveTitle(Object name) {
    return 'Remove “$name”?';
  }

  @override
  String get contentRemoveBody => 'The file is deleted from the instance.';

  @override
  String contentRemoved(Object name) {
    return '“$name” removed';
  }

  @override
  String get modeSurvival => 'Survival';

  @override
  String get modeCreative => 'Creative';

  @override
  String get modeAdventure => 'Adventure';

  @override
  String get modeSpectator => 'Spectator';

  @override
  String get diffPeaceful => 'Peaceful';

  @override
  String get diffEasy => 'Easy';

  @override
  String get diffNormal => 'Normal';

  @override
  String get diffHard => 'Hard';

  @override
  String get worldsEmptyTitle => 'No worlds yet';

  @override
  String get worldsEmptyBody =>
      'Worlds you create while playing this instance will show up here.';

  @override
  String get worldsNone => 'This instance has no worlds yet.';

  @override
  String get worldUnreadable => 'level.dat could not be read';

  @override
  String get worldRename => 'Rename';

  @override
  String get worldName => 'World name';

  @override
  String get worldLockedRunning =>
      'The game is running: close it to edit the world.';

  @override
  String get worldGame => 'Game';

  @override
  String get worldMode => 'Game mode';

  @override
  String get worldDifficulty => 'Difficulty';

  @override
  String get worldHardcore => 'Hardcore';

  @override
  String get worldHardcoreDesc =>
      'One life only: when you die, the world switches to spectator.';

  @override
  String get worldCheats => 'Allow cheats';

  @override
  String get worldCheatsDesc => 'Enables commands such as /gamemode or /tp.';

  @override
  String get worldDelete => 'Delete the world';

  @override
  String get worldDeleteDesc =>
      'It is deleted forever, without going to the trash.';

  @override
  String worldDeleteTitle(Object name) {
    return 'Delete “$name”?';
  }

  @override
  String get worldDeleteBody =>
      'The world is deleted forever and cannot be recovered. If you want to keep it, create a backup first in the Backups tab.';

  @override
  String get worldDeleteAction => 'Delete forever';

  @override
  String worldDeleted(Object name) {
    return 'World “$name” deleted';
  }

  @override
  String get datapacksAdd => 'Add datapacks';

  @override
  String get datapacksEmpty => 'This world has no datapacks.';

  @override
  String get datapackRemoveBody => 'The datapack is deleted from this world.';

  @override
  String get screenshotsEmptyTitle => 'No screenshots';

  @override
  String get screenshotsEmptyBody =>
      'Press F2 while playing and your screenshots will show up here.';

  @override
  String screenshotsCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count screenshots',
      one: '1 screenshot',
    );
    return '$_temp0';
  }

  @override
  String get screenshotOpen => 'Open with the system viewer';

  @override
  String get screenshotDeleteTitle => 'Delete the screenshot?';

  @override
  String screenshotDeleteBody(Object file) {
    return '“$file” is deleted from disk.';
  }

  @override
  String get backupsTitle => 'Backups';

  @override
  String backupsSummary(int count, Object size) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count backups',
      one: '1 backup',
    );
    return '$_temp0 · $size in total';
  }

  @override
  String get backupsEmptyTitle => 'No backups yet';

  @override
  String get backupsEmptyBody =>
      'Create a backup before big changes. It includes worlds, mods and settings (not logs).';

  @override
  String get backupCreate => 'Create backup';

  @override
  String get backupCreateTitle => 'New backup';

  @override
  String get backupCreateBody =>
      'The whole instance is saved: worlds, mods, resources and settings.';

  @override
  String get backupLabelOptional => 'Label (optional)';

  @override
  String get backupLabelHint => 'Before installing Create';

  @override
  String get backupLabel => 'Label';

  @override
  String get backupLabelTitle => 'Change label';

  @override
  String get backupPinOnCreate => 'Pin this backup';

  @override
  String get backupPinnedNote =>
      'Pinned backups are never deleted automatically.';

  @override
  String get backupCreating => 'Creating the backup…';

  @override
  String get backupCreated => 'Backup created';

  @override
  String get backupAutomatic => 'Automatic backup';

  @override
  String get backupManual => 'Manual backup';

  @override
  String get backupAutoBadge => 'Automatic';

  @override
  String get backupPinnedBadge => 'Pinned';

  @override
  String get backupPin => 'Pin';

  @override
  String get backupUnpin => 'Unpin';

  @override
  String get backupRestore => 'Restore';

  @override
  String get backupDeleteTitle => 'Delete the backup?';

  @override
  String backupDeleteBody(Object name) {
    return '“$name” is deleted from disk.';
  }

  @override
  String get backupAutoTitle => 'Automatic backups';

  @override
  String get backupAutoDesc => 'While the app is open and the game is closed.';

  @override
  String get backupInterval => 'Frequency';

  @override
  String get backupDaily => 'Daily';

  @override
  String get backupWeekly => 'Weekly';

  @override
  String backupEveryHours(int hours) {
    return 'Every $hours h';
  }

  @override
  String backupEveryDays(int days) {
    return 'Every $days days';
  }

  @override
  String get backupKeep => 'Automatic backups to keep';

  @override
  String backupKeepCount(int count) {
    return '$count';
  }

  @override
  String get backupRestoreTitle => 'Restore backup';

  @override
  String backupRestoreFrom(Object name, Object date) {
    return 'Backup “$name” from $date.';
  }

  @override
  String get backupModeFull => 'Everything';

  @override
  String get backupModeFullDesc =>
      'Worlds, mods, resources, shaders and game settings.';

  @override
  String get backupModeWorlds => 'Worlds only';

  @override
  String get backupModeWorldsDesc =>
      'Replaces the worlds folder; mods and settings stay as they are.';

  @override
  String get backupWarnFull =>
      'Everything currently in the instance is replaced by the backup. Changes made after the backup are lost.';

  @override
  String get backupWarnWorlds =>
      'The current worlds are replaced by the ones in the backup. Progress made after the backup is lost.';

  @override
  String get backupSafety => 'Back up the current state first';

  @override
  String get backupSafetyLabel => 'Before restoring';

  @override
  String get backupRestoreFull => 'Restore everything';

  @override
  String get backupRestoreWorlds => 'Restore worlds';

  @override
  String get backupRestoring => 'Restoring…';

  @override
  String backupRestored(Object name) {
    return 'Backup “$name” restored';
  }

  @override
  String get contentTitle => 'Content';

  @override
  String get contentSubtitle =>
      'Mods, resource packs, shaders, datapacks and modpacks.';

  @override
  String contentSearchHint(Object kind) {
    return 'Search $kind…';
  }

  @override
  String contentResults(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count results',
      one: '1 result',
    );
    return '$_temp0';
  }

  @override
  String get contentNoResultsTitle => 'No results';

  @override
  String get contentNoResultsBody => 'Try other words or remove a filter.';

  @override
  String get contentEnd => 'No more results';

  @override
  String get contentInstalled => 'Installed';

  @override
  String contentAlreadyInstalled(Object version) {
    return 'Already installed in this instance ($version).';
  }

  @override
  String get contentTargetPrefix => 'Adding to ';

  @override
  String contentTargetWorld(Object world) {
    return 'world “$world”';
  }

  @override
  String get contentBackToInstance => 'Back to the instance';

  @override
  String get contentClearTarget => 'Search without an instance';

  @override
  String get sortRelevance => 'Relevance';

  @override
  String get sortDownloads => 'Downloads';

  @override
  String get sortFollows => 'Followers';

  @override
  String get sortNewest => 'Newest';

  @override
  String get sortUpdated => 'Recently updated';

  @override
  String get installIn => 'Install in';

  @override
  String get installInWorld => 'World';

  @override
  String get chooseInstance => 'Choose an instance';

  @override
  String get chooseInstanceFirst =>
      'Choose an instance to see compatible versions.';

  @override
  String get chooseWorld => 'Choose a world';

  @override
  String get installLatest => 'Install latest';

  @override
  String get versionsCompatible => 'Compatible versions';

  @override
  String get versionsNoneCompatible =>
      'There are no versions for this Minecraft version and loader.';

  @override
  String get releaseStable => 'Release';

  @override
  String get releaseBeta => 'Beta';

  @override
  String get releaseAlpha => 'Alpha';

  @override
  String installedIn(Object name, Object instance) {
    return '$name installed in $instance';
  }

  @override
  String installedWithDeps(Object name, Object instance, int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count dependencies',
      one: '1 dependency',
    );
    return '$name installed in $instance with $_temp0';
  }

  @override
  String installPartial(Object name, int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count dependencies were',
      one: '1 dependency was',
    );
    return '$name installed, but $_temp0 skipped';
  }

  @override
  String get cfKeyNeededTitle => 'CurseForge needs an API key';

  @override
  String get cfKeyNeededBody =>
      'CurseForge only allows searching with a personal key. You can request one for free at console.curseforge.com and save it here; it stays in the system keyring.';

  @override
  String get cfKeyAdd => 'Add key';

  @override
  String get cfKeyChange => 'Change key';

  @override
  String get cfKeyTitle => 'CurseForge API key';

  @override
  String get cfKeyLabel => 'API key';

  @override
  String get cfKeyHelper =>
      'It is stored in the system keyring and never shown again.';

  @override
  String get cfKeySaved => 'Key saved';

  @override
  String get cfKeySet => 'Configured';

  @override
  String get cfKeyNotSet => 'Not configured';

  @override
  String get cfKeyRemoveTitle => 'Remove the CurseForge key?';

  @override
  String get cfKeyRemoveBody =>
      'You will not be able to search CurseForge until you add another one.';

  @override
  String get modpackImport => 'Import modpack';

  @override
  String get modpackImportTitle => 'Import a modpack';

  @override
  String modpackInstallTitle(Object name) {
    return 'Install “$name”';
  }

  @override
  String get modpackBody =>
      'A new instance is created with the modpack\'s Minecraft, loader and mods.';

  @override
  String get modpackFromFile => 'File';

  @override
  String get modpackFromUrl => 'URL';

  @override
  String get modpackChooseFile => 'Choose a .mrpack or .zip…';

  @override
  String get modpackFiles => 'Modpacks';

  @override
  String get modpackName => 'Instance name';

  @override
  String get modpackNameHint => 'The modpack\'s name';

  @override
  String get modpackInstall => 'Install modpack';

  @override
  String get modpackInstalling => 'Downloading the modpack files…';

  @override
  String get modpackInstallingHint =>
      'It may take a few minutes. If something fails, the half-made instance is removed.';

  @override
  String get modpackDoneTitle => 'Modpack installed';

  @override
  String modpackDoneBody(int done, int total) {
    return '$done of $total files were downloaded.';
  }

  @override
  String get modpackOpenInstance => 'Open instance';

  @override
  String get serversTitle => 'Servers';

  @override
  String get serversSubtitle =>
      'Your servers with their status, ready to join.';

  @override
  String serversCount(int count, int online) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count servers',
      one: '1 server',
    );
    return '$_temp0 · $online online';
  }

  @override
  String get serversRefresh => 'Refresh status';

  @override
  String get serversImport => 'Import';

  @override
  String serversImported(int count, Object instance) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count servers imported',
      one: '1 server imported',
    );
    return '$_temp0 from $instance';
  }

  @override
  String serversImportNone(Object instance) {
    return '$instance has no new servers';
  }

  @override
  String get serversEmptyTitle => 'No servers yet';

  @override
  String get serversEmptyBody =>
      'Add one by hand or import them from an instance\'s server list.';

  @override
  String get serverAdd => 'Add server';

  @override
  String get serverEdit => 'Edit server';

  @override
  String get serverAddress => 'Address';

  @override
  String get serverAddressInvalid =>
      'Enter a valid address (host or host:port).';

  @override
  String get serverName => 'Name (optional)';

  @override
  String get serverNameHint => 'Same as the address';

  @override
  String get serverInstance => 'Instance to join with';

  @override
  String get serverInstanceNone => 'None';

  @override
  String get serverAccount => 'Account';

  @override
  String get serverAccountActive => 'The active account';

  @override
  String get serverJoin => 'Join';

  @override
  String serverJoinWith(Object instance) {
    return 'Join with $instance';
  }

  @override
  String serverAlreadyRunning(Object instance) {
    return '$instance is already running';
  }

  @override
  String get serverNoInstance => 'No instance assigned';

  @override
  String get serverOffline => 'Offline';

  @override
  String get serverUnreachable => 'Not responding';

  @override
  String get serverPinging => 'Checking…';

  @override
  String serverPing(int ms) {
    return '$ms ms';
  }

  @override
  String get serverCopyAddress => 'Copy address';

  @override
  String serverRemoveTitle(Object name) {
    return 'Remove “$name”?';
  }

  @override
  String get serverRemoveBody =>
      'It is removed from the list. You can add it again any time.';

  @override
  String get homeQuickServersEmpty =>
      'Add your favorite servers to join with one click.';

  @override
  String get settingsIntegrations => 'Integrations';

  @override
  String get settingsCurseforge => 'CurseForge';

  @override
  String get settingsCurseforgeDesc =>
      'Personal API key to search and install from CurseForge.';

  @override
  String get settingsDiscord => 'Discord Rich Presence';

  @override
  String get settingsDiscordDesc => 'Shows what you are playing on Discord.';

  @override
  String serverPlayersText(Object online, Object max) {
    return '$online / $max';
  }

  @override
  String get homeSeeAllServers => 'See all';
}
