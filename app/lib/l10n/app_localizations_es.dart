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
  String get comingSoonTitle => 'Próximamente';

  @override
  String get comingSoonSkins =>
      'Vista 3D de tu skin, subir una nueva y gestionar capas.';

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
}
