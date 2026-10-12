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
  String get comingSoonContent =>
      'Buscador de mods, paquetes de recursos, shaders y modpacks de Modrinth y CurseForge.';

  @override
  String get comingSoonServers =>
      'Tus servidores con su estado, jugadores y ping, listos para entrar con la instancia que elijas.';

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
  String get homeQuickServersSoon =>
      'Pronto podrás entrar a tus servidores favoritos desde aquí.';

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
  String get tabSoonContent =>
      'Mods, paquetes de recursos y shaders de esta instancia: activar, quitar, actualizar e identificar.';

  @override
  String get tabSoonWorlds =>
      'Lista de mundos con sus detalles, datapacks y opciones para editarlos o borrarlos.';

  @override
  String get tabSoonScreenshots =>
      'Galería de las capturas que hagas en el juego con F2.';

  @override
  String get tabSoonBackups =>
      'Copias de seguridad manuales y automáticas, fijarlas y restaurarlas.';

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
}
