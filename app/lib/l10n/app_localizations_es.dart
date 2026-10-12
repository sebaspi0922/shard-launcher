// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Spanish Castilian (`es`).
class AppLocalizationsEs extends AppLocalizations {
  AppLocalizationsEs([String locale = 'es']) : super(locale);

  @override
  String get navHome => 'Inicio';

  @override
  String get navInstances => 'Instancias';

  @override
  String get navContent => 'Contenido';

  @override
  String get navServers => 'Servidores';

  @override
  String get navSkins => 'Skins';

  @override
  String get navSettings => 'Ajustes';

  @override
  String get windowMinimize => 'Minimizar';

  @override
  String get windowMaximize => 'Maximizar';

  @override
  String get windowRestore => 'Restaurar';

  @override
  String get windowClose => 'Cerrar';

  @override
  String closeDialogTitle(Object app) {
    return '¿Cerrar $app?';
  }

  @override
  String get closeDialogBody =>
      'Puedes dejarlo en segundo plano o salir del todo. Las partidas abiertas no se cierran.';

  @override
  String get closeDialogMinimize => 'Dejar en segundo plano';

  @override
  String get closeDialogQuit => 'Salir';

  @override
  String get closeDialogRemember => 'Recordar mi elección';

  @override
  String get cancel => 'Cancelar';

  @override
  String get save => 'Guardar';

  @override
  String get create => 'Crear';

  @override
  String get delete => 'Eliminar';

  @override
  String get remove => 'Quitar';

  @override
  String get copy => 'Copiar';

  @override
  String get copied => 'Copiado';

  @override
  String get retry => 'Reintentar';

  @override
  String get close => 'Cerrar';

  @override
  String get back => 'Volver';

  @override
  String get discard => 'Descartar';

  @override
  String get details => 'Detalles';

  @override
  String get loading => 'Cargando…';

  @override
  String get accountsTitle => 'Cuentas';

  @override
  String get accountsEmpty => 'Aún no hay cuentas';

  @override
  String get accountsAddMicrosoft => 'Añadir cuenta de Microsoft';

  @override
  String get accountsAddOffline => 'Añadir cuenta offline';

  @override
  String get accountMicrosoft => 'Microsoft';

  @override
  String get accountOffline => 'Offline';

  @override
  String get accountActive => 'Activa';

  @override
  String get accountUse => 'Usar esta cuenta';

  @override
  String accountRemoveTitle(Object name) {
    return '¿Quitar «$name»?';
  }

  @override
  String accountRemoveBody(Object app) {
    return 'Se borra de $app junto con su sesión. Podrás volver a añadirla cuando quieras.';
  }

  @override
  String accountRemoved(Object name) {
    return 'Cuenta «$name» quitada';
  }

  @override
  String accountAdded(Object name) {
    return 'Cuenta «$name» añadida';
  }

  @override
  String get accountNone => 'Sin cuenta';

  @override
  String get accountNoneHint => 'Añade una cuenta para jugar';

  @override
  String get offlineTitle => 'Cuenta offline';

  @override
  String get offlineBody =>
      'Para jugar sin conexión o en servidores que no verifican cuentas. No permite cambiar la skin.';

  @override
  String get offlineNameLabel => 'Nombre de jugador';

  @override
  String get offlineNameHint => 'De 3 a 16 letras, números o _';

  @override
  String get offlineNameInvalid =>
      'Usa de 3 a 16 caracteres: letras, números o guion bajo.';

  @override
  String get add => 'Añadir';

  @override
  String get msTitle => 'Iniciar sesión con Microsoft';

  @override
  String get msRequesting => 'Pidiendo un código a Microsoft…';

  @override
  String get msStep => 'Abre la página de Microsoft e introduce este código:';

  @override
  String get msOpenBrowser => 'Abrir navegador';

  @override
  String get msCopyCode => 'Copiar código';

  @override
  String get msWaiting => 'Esperando confirmación…';

  @override
  String msExpiresIn(Object time) {
    return 'Caduca en $time';
  }

  @override
  String get msExpired => 'El código ha caducado.';

  @override
  String get msRequestNew => 'Pedir otro código';

  @override
  String msSignedIn(Object name) {
    return 'Sesión iniciada como $name';
  }

  @override
  String get errAuthMissingClientId =>
      'Falta el identificador de la aplicación de Microsoft, así que no se puede iniciar sesión. Configúralo en el núcleo o usa una cuenta offline.';

  @override
  String get errAuthAppNotApproved =>
      'Microsoft aún no ha aprobado esta aplicación para usar los servicios de Minecraft.';

  @override
  String get errAuthNotEntitled =>
      'Esta cuenta de Microsoft no tiene Minecraft: Java Edition.';

  @override
  String get errAuthNoProfile =>
      'La cuenta tiene el juego, pero aún no tiene perfil. Entra una vez en minecraft.net para elegir tu nombre.';

  @override
  String get errAuthXbox =>
      'Xbox Live rechazó el inicio de sesión (cuenta infantil, región o perfil de Xbox sin crear).';

  @override
  String get errAuthDeclined =>
      'Se rechazó el inicio de sesión en la página de Microsoft.';

  @override
  String get errAuthExpired => 'El código caducó antes de terminar. Pide otro.';

  @override
  String get errAuthSessionExpired =>
      'La sesión de esta cuenta caducó. Vuelve a iniciar sesión.';

  @override
  String get errAuthGeneric => 'No se pudo iniciar sesión con Microsoft.';

  @override
  String get errNetwork => 'No hay conexión o el servidor no responde.';

  @override
  String get errOffline =>
      'Sin conexión y sin copia guardada de lo que hace falta.';

  @override
  String get errJavaRequired =>
      'Falta la versión de Java que necesita esta instancia.';

  @override
  String get errGameAlreadyRunning =>
      'Ya hay una partida abierta. Activa «Permitir varias partidas» en Ajustes para abrir otra.';

  @override
  String get errInstanceNotFound => 'La instancia ya no existe.';

  @override
  String get errAccountNotFound =>
      'No hay ninguna cuenta para jugar. Añade una desde el avatar de abajo a la izquierda.';

  @override
  String get errVersionNotFound => 'Esa versión no existe.';

  @override
  String get errHashMismatch =>
      'Un archivo descargado llegó dañado. Vuelve a intentarlo.';

  @override
  String get errIo => 'No se pudo leer o escribir en el disco.';

  @override
  String get errInvalidInput => 'Hay un dato que no es válido.';

  @override
  String get errGeneric => 'Algo salió mal.';

  @override
  String homeGreeting(Object name) {
    return 'Hola, $name';
  }

  @override
  String get homeGreetingAnon => 'Te damos la bienvenida';

  @override
  String get homeSubtitle => '¿A qué jugamos hoy?';

  @override
  String get homeContinue => 'Continuar';

  @override
  String get homeNeverPlayed => 'Sin jugar todavía';

  @override
  String get homeChangeInstance => 'Cambiar instancia';

  @override
  String get homeRecent => 'Recientes';

  @override
  String get homeSeeAll => 'Ver todas';

  @override
  String get homeNoInstancesTitle => 'Crea tu primera instancia';

  @override
  String get homeNoInstancesBody =>
      'Una instancia es una carpeta de juego con su versión, sus mods y sus mundos, separada de las demás.';

  @override
  String get homeSession => 'Sesión';

  @override
  String get homeChangeAccount => 'Cambiar';

  @override
  String get homeQuickServers => 'Servidores rápidos';

  @override
  String get homeRunningNow => 'En marcha';

  @override
  String get homeNothingRunning => 'Ninguna partida abierta';

  @override
  String get play => 'Jugar';

  @override
  String get launchPreparing => 'Preparando…';

  @override
  String launchDownloading(int done, int total) {
    return 'Descargando $done/$total';
  }

  @override
  String get launchStarting => 'Iniciando…';

  @override
  String get launchRunning => 'En juego';

  @override
  String get launchFailed => 'No se pudo iniciar';

  @override
  String get launchCrashed => 'El juego se cerró con un error';

  @override
  String launchExitCode(int code) {
    return 'El juego terminó con el código $code.';
  }

  @override
  String launchCrashReport(Object path) {
    return 'Informe del fallo: $path';
  }

  @override
  String get launchSeeLogs => 'Ver registros';

  @override
  String get stageLibraries => 'Librerías';

  @override
  String get stageAssets => 'Recursos';

  @override
  String get stageOther => 'Archivos';

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
  String get lastPlayedNever => 'Nunca';

  @override
  String get relJustNow => 'Ahora mismo';

  @override
  String relMinutes(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: 'hace $n minutos',
      one: 'hace 1 minuto',
    );
    return '$_temp0';
  }

  @override
  String relHours(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: 'hace $n horas',
      one: 'hace 1 hora',
    );
    return '$_temp0';
  }

  @override
  String relDays(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: 'hace $n días',
      one: 'ayer',
    );
    return '$_temp0';
  }

  @override
  String relMonths(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: 'hace $n meses',
      one: 'hace 1 mes',
    );
    return '$_temp0';
  }

  @override
  String relYears(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: 'hace $n años',
      one: 'hace 1 año',
    );
    return '$_temp0';
  }

  @override
  String get instancesTitle => 'Instancias';

  @override
  String instancesCount(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n instancias',
      one: '1 instancia',
      zero: 'Ninguna instancia',
    );
    return '$_temp0';
  }

  @override
  String get instancesSearch => 'Buscar por nombre, versión o loader';

  @override
  String get instancesNew => 'Nueva instancia';

  @override
  String get instancesEmptyTitle => 'Aún no tienes instancias';

  @override
  String get instancesEmptyBody =>
      'Crea una para elegir versión de Minecraft y loader.';

  @override
  String instancesNoResults(Object q) {
    return 'Nada coincide con «$q»';
  }

  @override
  String get viewGrid => 'Cuadrícula';

  @override
  String get viewList => 'Lista';

  @override
  String get instancePlaytime => 'Tiempo jugado';

  @override
  String get instanceLastPlayed => 'Última partida';

  @override
  String get instanceOpenFolder => 'Abrir carpeta';

  @override
  String get instanceDelete => 'Eliminar instancia';

  @override
  String instanceDeleteTitle(Object name) {
    return '¿Eliminar «$name»?';
  }

  @override
  String instanceDeleteBody(Object app) {
    return 'Se moverá a la papelera de $app, donde podrás recuperarla durante un tiempo.';
  }

  @override
  String instanceDeleted(Object name) {
    return '«$name» se movió a la papelera';
  }

  @override
  String get instanceMore => 'Más opciones';

  @override
  String get instanceNotFound => 'Esta instancia ya no existe.';

  @override
  String get tabContent => 'Contenido';

  @override
  String get tabWorlds => 'Mundos';

  @override
  String get tabScreenshots => 'Capturas';

  @override
  String get tabBackups => 'Copias';

  @override
  String get tabLogs => 'Registros';

  @override
  String get tabSettings => 'Ajustes';

  @override
  String get createTitle => 'Nueva instancia';

  @override
  String get createName => 'Nombre';

  @override
  String get createNameHint => 'Mi mundo de supervivencia';

  @override
  String get createVersion => 'Versión de Minecraft';

  @override
  String get createSnapshots => 'Snapshots';

  @override
  String get createSearchVersion => 'Buscar versión';

  @override
  String get createLoader => 'Loader';

  @override
  String get createLoaderVersion => 'Versión del loader';

  @override
  String get createLoaderLatest => 'La más reciente';

  @override
  String createNoLoaderVersions(Object loader, Object mc) {
    return 'No hay versiones de $loader para $mc.';
  }

  @override
  String get createNoVersions => 'No se encontraron versiones.';

  @override
  String get createNameRequired => 'Ponle un nombre.';

  @override
  String createDone(Object name) {
    return 'Instancia «$name» creada';
  }

  @override
  String get createLatestRelease => 'Última';

  @override
  String get loaderVanilla => 'Vanilla';

  @override
  String get loaderCustom => 'Personalizado';

  @override
  String get isGeneral => 'General';

  @override
  String get isName => 'Nombre';

  @override
  String get isMemory => 'Memoria';

  @override
  String get isMemoryDesc => 'RAM máxima para el juego.';

  @override
  String get isUseGlobal => 'Usar el valor global';

  @override
  String isMemoryGlobal(Object value) {
    return 'Global: $value';
  }

  @override
  String isMemoryWarning(Object total) {
    return 'Supera el 75 % de la RAM del equipo ($total); el sistema podría ir lento.';
  }

  @override
  String get isJvm => 'Argumentos de JVM';

  @override
  String get isJvmDesc => 'Opciones extra de Java, separadas por espacios.';

  @override
  String get isWindow => 'Ventana del juego';

  @override
  String get isResolution => 'Resolución';

  @override
  String get isResolutionDesc => 'Vacío = tamaño por defecto del juego.';

  @override
  String get isWidth => 'Ancho';

  @override
  String get isHeight => 'Alto';

  @override
  String get isFullscreen => 'Pantalla completa';

  @override
  String get isFullscreenDesc => 'Abrir el juego a pantalla completa.';

  @override
  String get saved => 'Cambios guardados';

  @override
  String get unsavedChanges => 'Hay cambios sin guardar';

  @override
  String get consoleFilter => 'Filtrar líneas';

  @override
  String get consoleFollow => 'Seguir al final';

  @override
  String get consoleCopy => 'Copiar';

  @override
  String get consoleClear => 'Limpiar';

  @override
  String get consoleEmptyTitle => 'Aún no hay registros';

  @override
  String get consoleEmptyBody =>
      'Lanza la instancia para ver aquí la salida del juego.';

  @override
  String consoleCopied(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n líneas copiadas',
      one: '1 línea copiada',
    );
    return '$_temp0';
  }

  @override
  String consoleLines(int n) {
    String _temp0 = intl.Intl.pluralLogic(
      n,
      locale: localeName,
      other: '$n líneas',
      one: '1 línea',
    );
    return '$_temp0';
  }

  @override
  String get consoleNoMatches => 'Ninguna línea coincide con el filtro.';

  @override
  String get logLevelError => 'Errores';

  @override
  String get logLevelWarn => 'Avisos';

  @override
  String get logLevelInfo => 'Info';

  @override
  String get logLevelDebug => 'Depuración';

  @override
  String get settingsTitle => 'Ajustes';

  @override
  String get settingsGeneral => 'General';

  @override
  String get settingsLanguage => 'Idioma';

  @override
  String get settingsLanguageDesc => 'Idioma de la interfaz.';

  @override
  String get languageSystem => 'Del sistema';

  @override
  String get settingsClose => 'Al cerrar la ventana';

  @override
  String get settingsCloseDesc => 'Qué hacer al pulsar el botón de cerrar.';

  @override
  String get closeAsk => 'Preguntar';

  @override
  String get closeTray => 'Segundo plano';

  @override
  String get closeQuit => 'Salir';

  @override
  String get settingsMultiple => 'Permitir varias partidas';

  @override
  String get settingsMultipleDesc => 'Abrir más de una instancia a la vez.';

  @override
  String get settingsGame => 'Juego';

  @override
  String get settingsMemory => 'Memoria por defecto';

  @override
  String settingsMemoryDesc(Object total) {
    return 'Para las instancias sin un valor propio. Este equipo tiene $total.';
  }

  @override
  String get settingsJvm => 'Argumentos de JVM por defecto';

  @override
  String get settingsJvmDesc => 'Separados por espacios.';

  @override
  String get settingsDownloads => 'Descargas';

  @override
  String get settingsConcurrency => 'Descargas simultáneas';

  @override
  String get settingsConcurrencyDesc =>
      'Con buena conexión, más es más rápido.';

  @override
  String get settingsAbout => 'Acerca de';

  @override
  String settingsAboutBody(Object version) {
    return 'Launcher de Minecraft para uso personal. Núcleo $version.';
  }

  @override
  String get settingsDataDir => 'Carpeta de datos';

  @override
  String get settingsMsa => 'Inicio de sesión de Microsoft';

  @override
  String get msaConfigured => 'Disponible';

  @override
  String get msaNotConfigured => 'Sin configurar';

  @override
  String get settingsSaveFailed => 'No se pudieron guardar los ajustes';

  @override
  String memoryGb(Object value) {
    return '$value GB';
  }

  @override
  String get openFolder => 'Abrir carpeta';

  @override
  String get previous => 'Anterior';

  @override
  String get next => 'Siguiente';

  @override
  String get install => 'Instalar';

  @override
  String get unknownValue => 'Desconocido';

  @override
  String get closeGameFirst => 'Cierra el juego de esta instancia primero';

  @override
  String get filterAll => 'Todo';

  @override
  String get filterAny => 'Cualquiera';

  @override
  String get filterVersion => 'Versión';

  @override
  String get filterLoader => 'Loader';

  @override
  String get sourceUnknown => 'Desconocido';

  @override
  String byAuthor(Object author) {
    return 'de $author';
  }

  @override
  String get errCurseforgeKey =>
      'Falta la clave de API de CurseForge. Añádela en Ajustes › Integraciones.';

  @override
  String get errSecrets => 'No se pudo usar el almacén de claves del sistema.';

  @override
  String get kindMods => 'Mods';

  @override
  String get kindResourcepacks => 'Paquetes de recursos';

  @override
  String get kindShaders => 'Shaders';

  @override
  String get kindDatapacks => 'Datapacks';

  @override
  String get kindModpacks => 'Modpacks';

  @override
  String get kindMod => 'Mod';

  @override
  String get kindResourcepack => 'Paquete de recursos';

  @override
  String get kindShader => 'Shader';

  @override
  String get kindDatapack => 'Datapack';

  @override
  String get kindModpack => 'Modpack';

  @override
  String get contentAdd => 'Añadir';

  @override
  String get contentBrowse => 'Explorar contenido';

  @override
  String get contentCheckUpdates => 'Buscar actualizaciones';

  @override
  String get contentFilterHint => 'Filtrar…';

  @override
  String get contentEnable => 'Activar';

  @override
  String get contentDisable => 'Desactivar';

  @override
  String get contentDisabled => 'Desactivado';

  @override
  String get contentEmptyTitle => 'Sin contenido todavía';

  @override
  String get contentEmptyBody =>
      'Añade mods, paquetes de recursos o shaders desde Modrinth o CurseForge.';

  @override
  String get contentNoMatches => 'Nada coincide con el filtro.';

  @override
  String get contentIdentify => 'Identificar';

  @override
  String get contentUpdateAll => 'Actualizar todo';

  @override
  String contentUpdatesAvailable(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count actualizaciones disponibles',
      one: '1 actualización disponible',
    );
    return '$_temp0';
  }

  @override
  String contentUnknownFiles(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count archivos de origen desconocido',
      one: '1 archivo de origen desconocido',
    );
    return '$_temp0';
  }

  @override
  String contentUpdateTo(Object version) {
    return 'Actualizar a $version';
  }

  @override
  String contentUpdated(Object name, Object version) {
    return '$name actualizado a $version';
  }

  @override
  String contentUpdatedAll(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count archivos actualizados',
      one: '1 archivo actualizado',
    );
    return '$_temp0';
  }

  @override
  String contentUpdatedSome(int ok, int failed) {
    return '$ok actualizados, $failed con error';
  }

  @override
  String contentIdentified(int found, int missing) {
    return 'Reconocidos: $found. Sin reconocer: $missing.';
  }

  @override
  String contentRemoveTitle(Object name) {
    return '¿Quitar «$name»?';
  }

  @override
  String get contentRemoveBody => 'Se borra el archivo de la instancia.';

  @override
  String contentRemoved(Object name) {
    return '«$name» quitado';
  }

  @override
  String get modeSurvival => 'Supervivencia';

  @override
  String get modeCreative => 'Creativo';

  @override
  String get modeAdventure => 'Aventura';

  @override
  String get modeSpectator => 'Espectador';

  @override
  String get diffPeaceful => 'Pacífica';

  @override
  String get diffEasy => 'Fácil';

  @override
  String get diffNormal => 'Normal';

  @override
  String get diffHard => 'Difícil';

  @override
  String get worldsEmptyTitle => 'Aún no hay mundos';

  @override
  String get worldsEmptyBody =>
      'Los mundos que crees al jugar con esta instancia aparecerán aquí.';

  @override
  String get worldsNone => 'Esta instancia no tiene mundos todavía.';

  @override
  String get worldUnreadable => 'No se pudo leer level.dat';

  @override
  String get worldRename => 'Cambiar nombre';

  @override
  String get worldName => 'Nombre del mundo';

  @override
  String get worldLockedRunning =>
      'El juego está abierto: ciérralo para editar el mundo.';

  @override
  String get worldGame => 'Partida';

  @override
  String get worldMode => 'Modo de juego';

  @override
  String get worldDifficulty => 'Dificultad';

  @override
  String get worldHardcore => 'Hardcore';

  @override
  String get worldHardcoreDesc =>
      'Una sola vida: al morir, el mundo queda en modo espectador.';

  @override
  String get worldCheats => 'Permitir trucos';

  @override
  String get worldCheatsDesc => 'Habilita los comandos como /gamemode o /tp.';

  @override
  String get worldDelete => 'Borrar el mundo';

  @override
  String get worldDeleteDesc =>
      'Se borra para siempre, sin pasar por la papelera.';

  @override
  String worldDeleteTitle(Object name) {
    return '¿Borrar «$name»?';
  }

  @override
  String get worldDeleteBody =>
      'El mundo se borra para siempre y no se puede recuperar. Si quieres conservarlo, crea antes una copia en la pestaña Copias.';

  @override
  String get worldDeleteAction => 'Borrar para siempre';

  @override
  String worldDeleted(Object name) {
    return 'Mundo «$name» borrado';
  }

  @override
  String get datapacksAdd => 'Añadir datapacks';

  @override
  String get datapacksEmpty => 'Este mundo no tiene datapacks.';

  @override
  String get datapackRemoveBody => 'Se borra el datapack de este mundo.';

  @override
  String get screenshotsEmptyTitle => 'Sin capturas';

  @override
  String get screenshotsEmptyBody =>
      'Pulsa F2 mientras juegas y tus capturas aparecerán aquí.';

  @override
  String screenshotsCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count capturas',
      one: '1 captura',
    );
    return '$_temp0';
  }

  @override
  String get screenshotOpen => 'Abrir con el visor del sistema';

  @override
  String get screenshotDeleteTitle => '¿Borrar la captura?';

  @override
  String screenshotDeleteBody(Object file) {
    return 'Se borra «$file» del disco.';
  }

  @override
  String get backupsTitle => 'Copias de seguridad';

  @override
  String backupsSummary(int count, Object size) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count copias',
      one: '1 copia',
    );
    return '$_temp0 · $size en total';
  }

  @override
  String get backupsEmptyTitle => 'Sin copias todavía';

  @override
  String get backupsEmptyBody =>
      'Crea una copia antes de cambios grandes. Incluye mundos, mods y configuración (no los registros).';

  @override
  String get backupCreate => 'Crear copia';

  @override
  String get backupCreateTitle => 'Nueva copia de seguridad';

  @override
  String get backupCreateBody =>
      'Se guarda todo el contenido de la instancia: mundos, mods, recursos y configuración.';

  @override
  String get backupLabelOptional => 'Etiqueta (opcional)';

  @override
  String get backupLabelHint => 'Antes de instalar Create';

  @override
  String get backupLabel => 'Etiqueta';

  @override
  String get backupLabelTitle => 'Cambiar etiqueta';

  @override
  String get backupPinOnCreate => 'Fijar esta copia';

  @override
  String get backupPinnedNote => 'Las copias fijadas nunca se borran solas.';

  @override
  String get backupCreating => 'Creando la copia…';

  @override
  String get backupCreated => 'Copia creada';

  @override
  String get backupAutomatic => 'Copia automática';

  @override
  String get backupManual => 'Copia manual';

  @override
  String get backupAutoBadge => 'Automática';

  @override
  String get backupPinnedBadge => 'Fijada';

  @override
  String get backupPin => 'Fijar';

  @override
  String get backupUnpin => 'Dejar de fijar';

  @override
  String get backupRestore => 'Restaurar';

  @override
  String get backupDeleteTitle => '¿Eliminar la copia?';

  @override
  String backupDeleteBody(Object name) {
    return 'Se borra «$name» del disco.';
  }

  @override
  String get backupAutoTitle => 'Copias automáticas';

  @override
  String get backupAutoDesc =>
      'Mientras la app está abierta y el juego cerrado.';

  @override
  String get backupInterval => 'Frecuencia';

  @override
  String get backupDaily => 'A diario';

  @override
  String get backupWeekly => 'Semanal';

  @override
  String backupEveryHours(int hours) {
    return 'Cada $hours h';
  }

  @override
  String backupEveryDays(int days) {
    return 'Cada $days días';
  }

  @override
  String get backupKeep => 'Copias automáticas que se conservan';

  @override
  String backupKeepCount(int count) {
    return '$count';
  }

  @override
  String get backupRestoreTitle => 'Restaurar copia';

  @override
  String backupRestoreFrom(Object name, Object date) {
    return 'Copia «$name» del $date.';
  }

  @override
  String get backupModeFull => 'Todo';

  @override
  String get backupModeFullDesc =>
      'Mundos, mods, recursos, shaders y configuración del juego.';

  @override
  String get backupModeWorlds => 'Solo los mundos';

  @override
  String get backupModeWorldsDesc =>
      'Sustituye la carpeta de mundos; los mods y la configuración se quedan como están.';

  @override
  String get backupWarnFull =>
      'Todo lo que hay ahora en la instancia se sustituye por la copia. Los cambios posteriores a la copia se pierden.';

  @override
  String get backupWarnWorlds =>
      'Los mundos actuales se sustituyen por los de la copia. El progreso posterior a la copia se pierde.';

  @override
  String get backupSafety => 'Crear antes una copia del estado actual';

  @override
  String get backupSafetyLabel => 'Antes de restaurar';

  @override
  String get backupRestoreFull => 'Restaurar todo';

  @override
  String get backupRestoreWorlds => 'Restaurar mundos';

  @override
  String get backupRestoring => 'Restaurando…';

  @override
  String backupRestored(Object name) {
    return 'Copia «$name» restaurada';
  }

  @override
  String get contentTitle => 'Contenido';

  @override
  String get contentSubtitle =>
      'Mods, paquetes de recursos, shaders, datapacks y modpacks.';

  @override
  String contentSearchHint(Object kind) {
    return 'Buscar $kind…';
  }

  @override
  String contentResults(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count resultados',
      one: '1 resultado',
    );
    return '$_temp0';
  }

  @override
  String get contentNoResultsTitle => 'Sin resultados';

  @override
  String get contentNoResultsBody =>
      'Prueba con otras palabras o quita algún filtro.';

  @override
  String get contentEnd => 'No hay más resultados';

  @override
  String get contentInstalled => 'Instalado';

  @override
  String contentAlreadyInstalled(Object version) {
    return 'Ya está instalado en esta instancia ($version).';
  }

  @override
  String get contentTargetPrefix => 'Añadiendo a ';

  @override
  String contentTargetWorld(Object world) {
    return 'mundo «$world»';
  }

  @override
  String get contentBackToInstance => 'Volver a la instancia';

  @override
  String get contentClearTarget => 'Buscar sin instancia';

  @override
  String get sortRelevance => 'Relevancia';

  @override
  String get sortDownloads => 'Descargas';

  @override
  String get sortFollows => 'Seguidores';

  @override
  String get sortNewest => 'Más nuevos';

  @override
  String get sortUpdated => 'Actualizados';

  @override
  String get installIn => 'Instalar en';

  @override
  String get installInWorld => 'Mundo';

  @override
  String get chooseInstance => 'Elige una instancia';

  @override
  String get chooseInstanceFirst =>
      'Elige una instancia para ver las versiones compatibles.';

  @override
  String get chooseWorld => 'Elige un mundo';

  @override
  String get installLatest => 'Instalar la última';

  @override
  String get versionsCompatible => 'Versiones compatibles';

  @override
  String get versionsNoneCompatible =>
      'No hay versiones para esta versión de Minecraft y este loader.';

  @override
  String get releaseStable => 'Estable';

  @override
  String get releaseBeta => 'Beta';

  @override
  String get releaseAlpha => 'Alfa';

  @override
  String installedIn(Object name, Object instance) {
    return '$name instalado en $instance';
  }

  @override
  String installedWithDeps(Object name, Object instance, int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count dependencias',
      one: '1 dependencia',
    );
    return '$name instalado en $instance con $_temp0';
  }

  @override
  String installPartial(Object name, int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count dependencias',
      one: '1 dependencia',
    );
    return '$name instalado, pero se omitieron $_temp0';
  }

  @override
  String get cfKeyNeededTitle => 'CurseForge necesita una clave de API';

  @override
  String get cfKeyNeededBody =>
      'CurseForge solo permite buscar con una clave personal. Puedes pedirla gratis en console.curseforge.com y guardarla aquí; se queda en el llavero del sistema.';

  @override
  String get cfKeyAdd => 'Añadir clave';

  @override
  String get cfKeyChange => 'Cambiar clave';

  @override
  String get cfKeyTitle => 'Clave de API de CurseForge';

  @override
  String get cfKeyLabel => 'Clave de API';

  @override
  String get cfKeyHelper =>
      'Se guarda en el llavero del sistema y no se vuelve a mostrar.';

  @override
  String get cfKeySaved => 'Clave guardada';

  @override
  String get cfKeySet => 'Configurada';

  @override
  String get cfKeyNotSet => 'Sin configurar';

  @override
  String get cfKeyRemoveTitle => '¿Quitar la clave de CurseForge?';

  @override
  String get cfKeyRemoveBody =>
      'Dejarás de poder buscar en CurseForge hasta que añadas otra.';

  @override
  String get modpackImport => 'Importar modpack';

  @override
  String get modpackImportTitle => 'Importar un modpack';

  @override
  String modpackInstallTitle(Object name) {
    return 'Instalar «$name»';
  }

  @override
  String get modpackBody =>
      'Se crea una instancia nueva con el Minecraft, el loader y los mods del modpack.';

  @override
  String get modpackFromFile => 'Archivo';

  @override
  String get modpackFromUrl => 'URL';

  @override
  String get modpackChooseFile => 'Elegir un .mrpack o .zip…';

  @override
  String get modpackFiles => 'Modpacks';

  @override
  String get modpackName => 'Nombre de la instancia';

  @override
  String get modpackNameHint => 'El del modpack';

  @override
  String get modpackInstall => 'Instalar modpack';

  @override
  String get modpackInstalling => 'Descargando los archivos del modpack…';

  @override
  String get modpackInstallingHint =>
      'Puede tardar unos minutos. Si algo falla, la instancia a medias se borra.';

  @override
  String get modpackDoneTitle => 'Modpack instalado';

  @override
  String modpackDoneBody(int done, int total) {
    return 'Se descargaron $done de $total archivos.';
  }

  @override
  String get modpackOpenInstance => 'Abrir instancia';

  @override
  String get serversTitle => 'Servidores';

  @override
  String get serversSubtitle =>
      'Tus servidores con su estado, listos para entrar.';

  @override
  String serversCount(int count, int online) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count servidores',
      one: '1 servidor',
    );
    return '$_temp0 · $online en línea';
  }

  @override
  String get serversRefresh => 'Actualizar estado';

  @override
  String get serversImport => 'Importar';

  @override
  String serversImported(int count, Object instance) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count servidores importados',
      one: '1 servidor importado',
    );
    return '$_temp0 de $instance';
  }

  @override
  String serversImportNone(Object instance) {
    return '$instance no tiene servidores nuevos';
  }

  @override
  String get serversEmptyTitle => 'Aún no hay servidores';

  @override
  String get serversEmptyBody =>
      'Añade uno a mano o impórtalos de la lista de servidores de una instancia.';

  @override
  String get serverAdd => 'Añadir servidor';

  @override
  String get serverEdit => 'Editar servidor';

  @override
  String get serverAddress => 'Dirección';

  @override
  String get serverAddressInvalid =>
      'Escribe una dirección válida (host o host:puerto).';

  @override
  String get serverName => 'Nombre (opcional)';

  @override
  String get serverNameHint => 'El de la dirección';

  @override
  String get serverInstance => 'Instancia con la que entrar';

  @override
  String get serverInstanceNone => 'Ninguna';

  @override
  String get serverAccount => 'Cuenta';

  @override
  String get serverAccountActive => 'La cuenta activa';

  @override
  String get serverJoin => 'Entrar';

  @override
  String serverJoinWith(Object instance) {
    return 'Entrar con $instance';
  }

  @override
  String serverAlreadyRunning(Object instance) {
    return '$instance ya está abierta';
  }

  @override
  String get serverNoInstance => 'Sin instancia asignada';

  @override
  String get serverOffline => 'Sin conexión';

  @override
  String get serverUnreachable => 'No responde';

  @override
  String get serverPinging => 'Consultando…';

  @override
  String serverPing(int ms) {
    return '$ms ms';
  }

  @override
  String get serverCopyAddress => 'Copiar dirección';

  @override
  String serverRemoveTitle(Object name) {
    return '¿Quitar «$name»?';
  }

  @override
  String get serverRemoveBody =>
      'Se quita de la lista. Puedes volver a añadirlo cuando quieras.';

  @override
  String get homeQuickServersEmpty =>
      'Añade tus servidores favoritos para entrar con un clic.';

  @override
  String get settingsIntegrations => 'Integraciones';

  @override
  String get settingsCurseforge => 'CurseForge';

  @override
  String get settingsCurseforgeDesc =>
      'Clave de API personal para buscar e instalar desde CurseForge.';

  @override
  String get settingsDiscord => 'Discord Rich Presence';

  @override
  String get settingsDiscordDesc => 'Muestra en Discord a qué estás jugando.';

  @override
  String serverPlayersText(Object online, Object max) {
    return '$online / $max';
  }

  @override
  String get homeSeeAllServers => 'Ver todos';

  @override
  String get settingsAppearance => 'Apariencia';

  @override
  String get appearanceBackground => 'Fondo';

  @override
  String get appearanceBackgroundDesc =>
      'Detrás de toda la app: el del estilo o una imagen tuya (se copia a la carpeta de datos).';

  @override
  String get appearanceAurora => 'Aurora';

  @override
  String get appearanceLandscape => 'Paisaje';

  @override
  String get appearanceImage => 'Imagen propia';

  @override
  String get appearancePickImage => 'Elegir imagen…';

  @override
  String get appearanceChangeImage => 'Cambiar imagen';

  @override
  String get appearanceImages => 'Imágenes';

  @override
  String get appearanceStyle => 'Estilo';

  @override
  String get appearanceStyleDesc =>
      'Colores, formas y fondo de toda la app. Debajo puedes cambiar el fondo y el acento.';

  @override
  String get styleGraphite => 'Grafito';

  @override
  String get styleSebaspi => 'sebaspi';

  @override
  String get appearanceBackgroundStyle => 'El del estilo';

  @override
  String get appearanceAccentStyle => 'Del estilo';

  @override
  String get appearanceAccentStyleDesc => 'El color propio del estilo.';

  @override
  String get appearanceAccent => 'Color de acento';

  @override
  String get appearanceAccentDesc => 'Botones, selección y detalles.';

  @override
  String get appearanceAccentAuto => 'Auto';

  @override
  String get appearanceAccentAutoDesc =>
      'Automático: se toma de tu imagen de fondo.';

  @override
  String get appearanceBlur => 'Desenfoque';

  @override
  String get appearanceBlurDesc => 'Del paisaje o de tu imagen.';

  @override
  String get appearanceDim => 'Oscurecido';

  @override
  String get appearanceDimDesc =>
      'Para que el texto se lea bien sobre el fondo.';

  @override
  String get appearancePerformance => 'Modo rendimiento';

  @override
  String get appearancePerformanceDesc =>
      'Sin desenfoque ni animaciones de fondo, y cambio de página instantáneo. Útil en equipos modestos.';

  @override
  String get settingsAutostart => 'Abrir al iniciar sesión';

  @override
  String settingsAutostartDesc(Object app) {
    return '$app arranca con el sistema, oculto en la bandeja.';
  }

  @override
  String get settingsJava => 'Java';

  @override
  String get javaDetected => 'Javas detectadas';

  @override
  String get javaDetectedDesc =>
      'Para cada versión se usa la adecuada; si falta, se instala sola al jugar.';

  @override
  String get javaRescan => 'Volver a buscar';

  @override
  String get javaNone =>
      'No se encontró ninguna Java. Se instalará la necesaria al jugar.';

  @override
  String get javaInstall => 'Instalar Java';

  @override
  String get javaInstallDesc =>
      'Descarga un JRE de Eclipse Adoptium a la carpeta de datos.';

  @override
  String javaInstalling(int major) {
    return 'Instalando Java $major…';
  }

  @override
  String javaInstallingProgress(int major, Object done, Object total) {
    return 'Instalando Java $major: $done de $total';
  }

  @override
  String javaInstalled(int major, Object version) {
    return 'Java $major instalada ($version)';
  }

  @override
  String get javaManaged => 'Gestionada';

  @override
  String get javaOverride => 'Elegida a mano';

  @override
  String get javaSystem => 'Del sistema';

  @override
  String get instanceChangeVersion => 'Cambiar versión';

  @override
  String changeVersionTitle(Object name) {
    return 'Cambiar versión de «$name»';
  }

  @override
  String changeVersionCurrent(Object version) {
    return 'Ahora: $version';
  }

  @override
  String get changeVersionApply => 'Cambiar versión';

  @override
  String get changeVersionWarning =>
      'Bajar de versión puede romper los mundos, y los mods tienen que ser de la nueva versión.';

  @override
  String get changeVersionBackup => 'Copia antes';

  @override
  String changeVersionBackupLabel(Object version) {
    return 'Antes de cambiar desde $version';
  }

  @override
  String changeVersionDone(Object version) {
    return 'Versión cambiada a $version';
  }

  @override
  String get trashTitle => 'Papelera';

  @override
  String trashHint(int days) {
    return 'Las instancias eliminadas se guardan aquí $days días, con sus copias de seguridad.';
  }

  @override
  String get trashEmptyStateTitle => 'La papelera está vacía';

  @override
  String trashEmptyStateBody(int days) {
    return 'Lo que elimines se queda aquí $days días por si cambias de idea.';
  }

  @override
  String get trashRestore => 'Recuperar';

  @override
  String trashRestored(Object name) {
    return '«$name» se recuperó';
  }

  @override
  String get trashPurge => 'Borrar para siempre';

  @override
  String trashPurgeTitle(Object name) {
    return '¿Borrar «$name» para siempre?';
  }

  @override
  String trashEmptyTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '¿Borrar $count instancias para siempre?',
      one: '¿Borrar 1 instancia para siempre?',
    );
    return '$_temp0';
  }

  @override
  String get trashPurgeBody =>
      'Se borran sus mundos, mods y copias de seguridad. No se puede deshacer.';

  @override
  String get trashEmptyAll => 'Vaciar papelera';

  @override
  String trashDeletedAgo(Object when) {
    return 'eliminada $when';
  }

  @override
  String trashDaysLeft(int days) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: 'quedan $days días',
      one: 'queda 1 día',
      zero: 'se borra hoy',
    );
    return '$_temp0';
  }

  @override
  String get doctorDiagnose => 'Diagnosticar';

  @override
  String doctorTitle(Object name) {
    return 'Doctor · $name';
  }

  @override
  String get doctorAnalyzing =>
      'Revisando el crash report, los registros y los mods…';

  @override
  String get doctorHealthyTitle => 'No vemos ningún problema';

  @override
  String get doctorHealthyBody =>
      'La última partida no dejó errores conocidos. Si el juego sigue fallando, revisa los registros.';

  @override
  String doctorCritical(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count graves',
      one: '1 grave',
    );
    return '$_temp0';
  }

  @override
  String doctorWarnings(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count avisos',
      one: '1 aviso',
    );
    return '$_temp0';
  }

  @override
  String doctorInfo(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count notas',
      one: '1 nota',
    );
    return '$_temp0';
  }

  @override
  String get doctorCrashExcerpt => 'Resumen del cierre';

  @override
  String get doctorOpenReport => 'Abrir crash report';

  @override
  String doctorConfidence(int value) {
    return 'Confianza $value %';
  }

  @override
  String get doctorCulprit => 'Culpable';

  @override
  String get doctorApplyFix => 'Aplicar arreglo';

  @override
  String get doctorFixApplied => 'Arreglo aplicado';

  @override
  String get doctorRerun => 'Volver a analizar';

  @override
  String get crashBannerTitle => 'El juego se cerró inesperadamente';

  @override
  String trayOpen(Object app) {
    return 'Abrir $app';
  }

  @override
  String trayPlay(Object name) {
    return 'Jugar «$name»';
  }

  @override
  String get trayQuit => 'Salir';

  @override
  String get firstRunSkip => 'Saltar asistente';

  @override
  String get firstRunStepWelcome => 'Bienvenida';

  @override
  String get firstRunStepImport => 'Importar';

  @override
  String get firstRunStepAccount => 'Cuenta';

  @override
  String get firstRunStepDone => 'Listo';

  @override
  String get firstRunRailHint => 'Puedes cambiar todo esto luego en Ajustes.';

  @override
  String firstRunWelcomeTitle(Object app) {
    return 'Te damos la bienvenida a $app';
  }

  @override
  String get firstRunWelcomeBody =>
      'Un launcher de Minecraft ligero para tus instancias, mods y servidores. En un par de pasos traemos lo que ya tienes y añadimos tu cuenta.';

  @override
  String get firstRunLanguageSystem => 'El del sistema';

  @override
  String get firstRunStart => 'Empezar';

  @override
  String get firstRunImportTitle => 'Trae lo que ya tienes';

  @override
  String get firstRunImportBody =>
      'Primero ves qué se importaría; nada se copia hasta que lo confirmes. Los originales no se tocan.';

  @override
  String get firstRunImportNoneTitle => 'No encontramos otros launchers';

  @override
  String get firstRunImportNoneBody =>
      'Puedes crear instancias desde cero o importar más tarde.';

  @override
  String get firstRunRoxy => 'Roxy Launcher';

  @override
  String get firstRunMinecraft => 'Launcher oficial (.minecraft)';

  @override
  String firstRunRoxySummary(int count, Object size, Object shared) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count instancias · $size',
      one: '1 instancia · $size',
      zero: 'Nada nuevo que importar',
    );
    return '$_temp0 · reutiliza $shared de librerías y assets';
  }

  @override
  String firstRunMinecraftSummary(Object name, Object version, Object size) {
    return 'Se crea «$name» ($version) con $size:';
  }

  @override
  String firstRunBackups(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count copias',
      one: '1 copia',
    );
    return '$_temp0';
  }

  @override
  String firstRunSkipped(Object reason) {
    return 'se omite: $reason';
  }

  @override
  String get firstRunImportAction => 'Importar';

  @override
  String get firstRunImporting => 'Importando…';

  @override
  String firstRunImportingFiles(int done, int total) {
    return '$done de $total archivos';
  }

  @override
  String firstRunImported(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count instancias importadas',
      one: '1 instancia importada',
      zero: 'No se importó ninguna instancia',
    );
    return '$_temp0';
  }

  @override
  String get firstRunSourceDone => 'Hecho';

  @override
  String get firstRunNotNow => 'Ahora no';

  @override
  String get firstRunAccountTitle => 'Añade tu cuenta';

  @override
  String get firstRunAccountBody =>
      'Con una cuenta de Microsoft juegas en servidores y usas tu skin. Una offline vale para jugar solo.';

  @override
  String get firstRunMicrosoftDesc =>
      'Inicia sesión con un código en el navegador.';

  @override
  String get firstRunOfflineDesc =>
      'Solo un nombre. Sin skins ni servidores con verificación.';

  @override
  String get firstRunAccountsAdded => 'Cuentas añadidas';

  @override
  String get firstRunDoneTitle => 'Todo listo';

  @override
  String get firstRunDoneBody =>
      'Ya puedes jugar. Esto es lo que hemos preparado:';

  @override
  String get firstRunNoInstances => 'ninguna todavía';

  @override
  String get firstRunAccount => 'Cuenta';

  @override
  String get firstRunAppearanceHint => 'fondo y acento en Ajustes';

  @override
  String firstRunFinish(Object app) {
    return 'Ir a $app';
  }

  @override
  String get sectionSaves => 'Mundos';

  @override
  String get sectionMods => 'Mods';

  @override
  String get sectionConfig => 'Configuración';

  @override
  String get sectionResourcepacks => 'Packs de recursos';

  @override
  String get sectionShaderpacks => 'Shaders';

  @override
  String get sectionScreenshots => 'Capturas';

  @override
  String get sectionOptions => 'Opciones';

  @override
  String get sectionServers => 'Servidores';

  @override
  String get skinsTitle => 'Skins';

  @override
  String get skinsSubtitle =>
      'Tu skin y tus capas, tal y como te ven los demás';

  @override
  String get skinsNoAccountTitle => 'Sin cuenta';

  @override
  String get skinsNoAccountBody =>
      'Añade una cuenta de Microsoft para ver y cambiar tu skin.';

  @override
  String get skinsAddAccount => 'Añadir cuenta';

  @override
  String get skinsOfflineTitle => 'Las skins son de las cuentas de Microsoft';

  @override
  String skinsOfflineBody(Object name) {
    return '«$name» es una cuenta offline: el juego le pone una skin por defecto y no se puede cambiar desde aquí. Añade una cuenta de Microsoft para elegir tu skin y tus capas.';
  }

  @override
  String get skinsPngFiles => 'Imágenes PNG';

  @override
  String get skinsInvalidPng =>
      'La skin debe ser un PNG de 64×64 o 64×32 píxeles.';

  @override
  String get skinsApplied => 'Skin actualizada';

  @override
  String get skinsResetTitle => '¿Restablecer la skin?';

  @override
  String get skinsResetBody =>
      'Tu cuenta volverá a la skin por defecto. La actual no se guarda: si la quieres, ten a mano el PNG.';

  @override
  String get skinsReset => 'Restablecer';

  @override
  String get skinsResetDone => 'Skin restablecida';

  @override
  String get skinsTextureMissing => 'No se pudo cargar la textura';

  @override
  String get skinsPreview => 'Vista previa';

  @override
  String get skinsSlim => 'Delgada';

  @override
  String get skinsClassic => 'Clásica';

  @override
  String get skinsSlimDesc => 'Brazos de 3 píxeles';

  @override
  String get skinsClassicDesc => 'Brazos de 4 píxeles';

  @override
  String get skinsDragHint => 'Arrastra para girar · doble clic para centrar';

  @override
  String get skinsFront => 'Frente';

  @override
  String get skinsBack => 'Espalda';

  @override
  String get skinsChange => 'Cambiar skin';

  @override
  String get skinsFromFile => 'Archivo PNG';

  @override
  String get skinsFromUrl => 'URL';

  @override
  String get skinsPickPng => 'Elegir un PNG…';

  @override
  String get skinsPngHint =>
      '64×64 o 64×32 píxeles. Se ve aquí antes de aplicarla.';

  @override
  String get skinsUrl => 'URL de la imagen';

  @override
  String get skinsUrlHint => 'Un enlace directo a un PNG público.';

  @override
  String get skinsModel => 'Modelo';

  @override
  String get skinsApply => 'Aplicar skin';

  @override
  String get skinsCapes => 'Capas';

  @override
  String skinsCapesCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count capas',
      one: '1 capa',
      zero: 'ninguna',
    );
    return '$_temp0';
  }

  @override
  String get skinsNoCapes => 'Esta cuenta no tiene capas.';

  @override
  String get skinsNoCape => 'Sin capa';

  @override
  String get skinsCape => 'Capa';

  @override
  String crashBannerCode(int code) {
    return 'código $code';
  }

  @override
  String get firstRunHighlights => 'Lo que vas a encontrar';

  @override
  String get firstRunFeatureInstances => 'Instancias separadas';

  @override
  String get firstRunFeatureInstancesBody =>
      'Cada una con su versión, loader, mundos y copias.';

  @override
  String get firstRunFeatureContent => 'Mods y modpacks';

  @override
  String get firstRunFeatureContentBody =>
      'Busca e instala desde Modrinth y CurseForge.';

  @override
  String get firstRunFeatureDoctor => 'Doctor';

  @override
  String get firstRunFeatureDoctorBody =>
      'Si el juego se cierra, te dice por qué y lo arregla.';

  @override
  String updateAvailableTitle(Object version) {
    return 'Shard $version disponible';
  }

  @override
  String updateAvailableBody(Object current) {
    return 'Tienes la $current.';
  }

  @override
  String get updateInstall => 'Actualizar';

  @override
  String get updateDownload => 'Descargar';

  @override
  String get updateNotes => 'Novedades';

  @override
  String get updateInstalling => 'Descargando y verificando…';

  @override
  String get updateReady => 'Lista. Reinicia Shard para usar la nueva versión.';

  @override
  String get updateRestart => 'Reiniciar';

  @override
  String get updateFailed => 'No se pudo actualizar';

  @override
  String get updateManualHint =>
      'Este paquete se actualiza a mano: descarga el nuevo e instálalo.';

  @override
  String get settingsUpdates => 'Actualizaciones';

  @override
  String settingsUpdatesCurrent(Object version) {
    return 'Versión $version.';
  }

  @override
  String settingsUpdatesUpToDate(Object version) {
    return 'Tienes la última versión ($version).';
  }

  @override
  String get settingsUpdatesChecking => 'Buscando…';

  @override
  String get settingsUpdatesCheck => 'Buscar';

  @override
  String get settingsUpdatesAuto => 'Buscar actualizaciones automáticamente';

  @override
  String get settingsUpdatesAutoDesc =>
      'Al abrir Shard y cada 6 horas. No se instala nada sin tu permiso.';

  @override
  String get errUpdateRejected =>
      'La actualización no tiene una firma válida y no se instaló.';
}
