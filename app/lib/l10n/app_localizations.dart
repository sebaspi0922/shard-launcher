import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_es.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('es'),
  ];

  /// No description provided for @navHome.
  ///
  /// In es, this message translates to:
  /// **'Inicio'**
  String get navHome;

  /// No description provided for @navInstances.
  ///
  /// In es, this message translates to:
  /// **'Instancias'**
  String get navInstances;

  /// No description provided for @navContent.
  ///
  /// In es, this message translates to:
  /// **'Contenido'**
  String get navContent;

  /// No description provided for @navServers.
  ///
  /// In es, this message translates to:
  /// **'Servidores'**
  String get navServers;

  /// No description provided for @navSkins.
  ///
  /// In es, this message translates to:
  /// **'Skins'**
  String get navSkins;

  /// No description provided for @navSettings.
  ///
  /// In es, this message translates to:
  /// **'Ajustes'**
  String get navSettings;

  /// No description provided for @comingSoonTitle.
  ///
  /// In es, this message translates to:
  /// **'Próximamente'**
  String get comingSoonTitle;

  /// No description provided for @comingSoonContent.
  ///
  /// In es, this message translates to:
  /// **'Buscador de mods, paquetes de recursos, shaders y modpacks de Modrinth y CurseForge.'**
  String get comingSoonContent;

  /// No description provided for @comingSoonServers.
  ///
  /// In es, this message translates to:
  /// **'Tus servidores con su estado, jugadores y ping, listos para entrar con la instancia que elijas.'**
  String get comingSoonServers;

  /// No description provided for @comingSoonSkins.
  ///
  /// In es, this message translates to:
  /// **'Vista 3D de tu skin, subir una nueva y gestionar capas.'**
  String get comingSoonSkins;

  /// No description provided for @windowMinimize.
  ///
  /// In es, this message translates to:
  /// **'Minimizar'**
  String get windowMinimize;

  /// No description provided for @windowMaximize.
  ///
  /// In es, this message translates to:
  /// **'Maximizar'**
  String get windowMaximize;

  /// No description provided for @windowRestore.
  ///
  /// In es, this message translates to:
  /// **'Restaurar'**
  String get windowRestore;

  /// No description provided for @windowClose.
  ///
  /// In es, this message translates to:
  /// **'Cerrar'**
  String get windowClose;

  /// No description provided for @closeDialogTitle.
  ///
  /// In es, this message translates to:
  /// **'¿Cerrar {app}?'**
  String closeDialogTitle(Object app);

  /// No description provided for @closeDialogBody.
  ///
  /// In es, this message translates to:
  /// **'Puedes dejarlo en segundo plano o salir del todo. Las partidas abiertas no se cierran.'**
  String get closeDialogBody;

  /// No description provided for @closeDialogMinimize.
  ///
  /// In es, this message translates to:
  /// **'Dejar en segundo plano'**
  String get closeDialogMinimize;

  /// No description provided for @closeDialogQuit.
  ///
  /// In es, this message translates to:
  /// **'Salir'**
  String get closeDialogQuit;

  /// No description provided for @closeDialogRemember.
  ///
  /// In es, this message translates to:
  /// **'Recordar mi elección'**
  String get closeDialogRemember;

  /// No description provided for @cancel.
  ///
  /// In es, this message translates to:
  /// **'Cancelar'**
  String get cancel;

  /// No description provided for @save.
  ///
  /// In es, this message translates to:
  /// **'Guardar'**
  String get save;

  /// No description provided for @create.
  ///
  /// In es, this message translates to:
  /// **'Crear'**
  String get create;

  /// No description provided for @delete.
  ///
  /// In es, this message translates to:
  /// **'Eliminar'**
  String get delete;

  /// No description provided for @remove.
  ///
  /// In es, this message translates to:
  /// **'Quitar'**
  String get remove;

  /// No description provided for @copy.
  ///
  /// In es, this message translates to:
  /// **'Copiar'**
  String get copy;

  /// No description provided for @copied.
  ///
  /// In es, this message translates to:
  /// **'Copiado'**
  String get copied;

  /// No description provided for @retry.
  ///
  /// In es, this message translates to:
  /// **'Reintentar'**
  String get retry;

  /// No description provided for @close.
  ///
  /// In es, this message translates to:
  /// **'Cerrar'**
  String get close;

  /// No description provided for @back.
  ///
  /// In es, this message translates to:
  /// **'Volver'**
  String get back;

  /// No description provided for @discard.
  ///
  /// In es, this message translates to:
  /// **'Descartar'**
  String get discard;

  /// No description provided for @details.
  ///
  /// In es, this message translates to:
  /// **'Detalles'**
  String get details;

  /// No description provided for @loading.
  ///
  /// In es, this message translates to:
  /// **'Cargando…'**
  String get loading;

  /// No description provided for @accountsTitle.
  ///
  /// In es, this message translates to:
  /// **'Cuentas'**
  String get accountsTitle;

  /// No description provided for @accountsEmpty.
  ///
  /// In es, this message translates to:
  /// **'Aún no hay cuentas'**
  String get accountsEmpty;

  /// No description provided for @accountsAddMicrosoft.
  ///
  /// In es, this message translates to:
  /// **'Añadir cuenta de Microsoft'**
  String get accountsAddMicrosoft;

  /// No description provided for @accountsAddOffline.
  ///
  /// In es, this message translates to:
  /// **'Añadir cuenta offline'**
  String get accountsAddOffline;

  /// No description provided for @accountMicrosoft.
  ///
  /// In es, this message translates to:
  /// **'Microsoft'**
  String get accountMicrosoft;

  /// No description provided for @accountOffline.
  ///
  /// In es, this message translates to:
  /// **'Offline'**
  String get accountOffline;

  /// No description provided for @accountActive.
  ///
  /// In es, this message translates to:
  /// **'Activa'**
  String get accountActive;

  /// No description provided for @accountUse.
  ///
  /// In es, this message translates to:
  /// **'Usar esta cuenta'**
  String get accountUse;

  /// No description provided for @accountRemoveTitle.
  ///
  /// In es, this message translates to:
  /// **'¿Quitar «{name}»?'**
  String accountRemoveTitle(Object name);

  /// No description provided for @accountRemoveBody.
  ///
  /// In es, this message translates to:
  /// **'Se borra de {app} junto con su sesión. Podrás volver a añadirla cuando quieras.'**
  String accountRemoveBody(Object app);

  /// No description provided for @accountRemoved.
  ///
  /// In es, this message translates to:
  /// **'Cuenta «{name}» quitada'**
  String accountRemoved(Object name);

  /// No description provided for @accountAdded.
  ///
  /// In es, this message translates to:
  /// **'Cuenta «{name}» añadida'**
  String accountAdded(Object name);

  /// No description provided for @accountNone.
  ///
  /// In es, this message translates to:
  /// **'Sin cuenta'**
  String get accountNone;

  /// No description provided for @accountNoneHint.
  ///
  /// In es, this message translates to:
  /// **'Añade una cuenta para jugar'**
  String get accountNoneHint;

  /// No description provided for @offlineTitle.
  ///
  /// In es, this message translates to:
  /// **'Cuenta offline'**
  String get offlineTitle;

  /// No description provided for @offlineBody.
  ///
  /// In es, this message translates to:
  /// **'Para jugar sin conexión o en servidores que no verifican cuentas. No permite cambiar la skin.'**
  String get offlineBody;

  /// No description provided for @offlineNameLabel.
  ///
  /// In es, this message translates to:
  /// **'Nombre de jugador'**
  String get offlineNameLabel;

  /// No description provided for @offlineNameHint.
  ///
  /// In es, this message translates to:
  /// **'De 3 a 16 letras, números o _'**
  String get offlineNameHint;

  /// No description provided for @offlineNameInvalid.
  ///
  /// In es, this message translates to:
  /// **'Usa de 3 a 16 caracteres: letras, números o guion bajo.'**
  String get offlineNameInvalid;

  /// No description provided for @add.
  ///
  /// In es, this message translates to:
  /// **'Añadir'**
  String get add;

  /// No description provided for @msTitle.
  ///
  /// In es, this message translates to:
  /// **'Iniciar sesión con Microsoft'**
  String get msTitle;

  /// No description provided for @msRequesting.
  ///
  /// In es, this message translates to:
  /// **'Pidiendo un código a Microsoft…'**
  String get msRequesting;

  /// No description provided for @msStep.
  ///
  /// In es, this message translates to:
  /// **'Abre la página de Microsoft e introduce este código:'**
  String get msStep;

  /// No description provided for @msOpenBrowser.
  ///
  /// In es, this message translates to:
  /// **'Abrir navegador'**
  String get msOpenBrowser;

  /// No description provided for @msCopyCode.
  ///
  /// In es, this message translates to:
  /// **'Copiar código'**
  String get msCopyCode;

  /// No description provided for @msWaiting.
  ///
  /// In es, this message translates to:
  /// **'Esperando confirmación…'**
  String get msWaiting;

  /// No description provided for @msExpiresIn.
  ///
  /// In es, this message translates to:
  /// **'Caduca en {time}'**
  String msExpiresIn(Object time);

  /// No description provided for @msExpired.
  ///
  /// In es, this message translates to:
  /// **'El código ha caducado.'**
  String get msExpired;

  /// No description provided for @msRequestNew.
  ///
  /// In es, this message translates to:
  /// **'Pedir otro código'**
  String get msRequestNew;

  /// No description provided for @msSignedIn.
  ///
  /// In es, this message translates to:
  /// **'Sesión iniciada como {name}'**
  String msSignedIn(Object name);

  /// No description provided for @errAuthMissingClientId.
  ///
  /// In es, this message translates to:
  /// **'Falta el identificador de la aplicación de Microsoft, así que no se puede iniciar sesión. Configúralo en el núcleo o usa una cuenta offline.'**
  String get errAuthMissingClientId;

  /// No description provided for @errAuthAppNotApproved.
  ///
  /// In es, this message translates to:
  /// **'Microsoft aún no ha aprobado esta aplicación para usar los servicios de Minecraft.'**
  String get errAuthAppNotApproved;

  /// No description provided for @errAuthNotEntitled.
  ///
  /// In es, this message translates to:
  /// **'Esta cuenta de Microsoft no tiene Minecraft: Java Edition.'**
  String get errAuthNotEntitled;

  /// No description provided for @errAuthNoProfile.
  ///
  /// In es, this message translates to:
  /// **'La cuenta tiene el juego, pero aún no tiene perfil. Entra una vez en minecraft.net para elegir tu nombre.'**
  String get errAuthNoProfile;

  /// No description provided for @errAuthXbox.
  ///
  /// In es, this message translates to:
  /// **'Xbox Live rechazó el inicio de sesión (cuenta infantil, región o perfil de Xbox sin crear).'**
  String get errAuthXbox;

  /// No description provided for @errAuthDeclined.
  ///
  /// In es, this message translates to:
  /// **'Se rechazó el inicio de sesión en la página de Microsoft.'**
  String get errAuthDeclined;

  /// No description provided for @errAuthExpired.
  ///
  /// In es, this message translates to:
  /// **'El código caducó antes de terminar. Pide otro.'**
  String get errAuthExpired;

  /// No description provided for @errAuthSessionExpired.
  ///
  /// In es, this message translates to:
  /// **'La sesión de esta cuenta caducó. Vuelve a iniciar sesión.'**
  String get errAuthSessionExpired;

  /// No description provided for @errAuthGeneric.
  ///
  /// In es, this message translates to:
  /// **'No se pudo iniciar sesión con Microsoft.'**
  String get errAuthGeneric;

  /// No description provided for @errNetwork.
  ///
  /// In es, this message translates to:
  /// **'No hay conexión o el servidor no responde.'**
  String get errNetwork;

  /// No description provided for @errOffline.
  ///
  /// In es, this message translates to:
  /// **'Sin conexión y sin copia guardada de lo que hace falta.'**
  String get errOffline;

  /// No description provided for @errJavaRequired.
  ///
  /// In es, this message translates to:
  /// **'Falta la versión de Java que necesita esta instancia.'**
  String get errJavaRequired;

  /// No description provided for @errGameAlreadyRunning.
  ///
  /// In es, this message translates to:
  /// **'Ya hay una partida abierta. Activa «Permitir varias partidas» en Ajustes para abrir otra.'**
  String get errGameAlreadyRunning;

  /// No description provided for @errInstanceNotFound.
  ///
  /// In es, this message translates to:
  /// **'La instancia ya no existe.'**
  String get errInstanceNotFound;

  /// No description provided for @errAccountNotFound.
  ///
  /// In es, this message translates to:
  /// **'No hay ninguna cuenta para jugar. Añade una desde el avatar de abajo a la izquierda.'**
  String get errAccountNotFound;

  /// No description provided for @errVersionNotFound.
  ///
  /// In es, this message translates to:
  /// **'Esa versión no existe.'**
  String get errVersionNotFound;

  /// No description provided for @errHashMismatch.
  ///
  /// In es, this message translates to:
  /// **'Un archivo descargado llegó dañado. Vuelve a intentarlo.'**
  String get errHashMismatch;

  /// No description provided for @errIo.
  ///
  /// In es, this message translates to:
  /// **'No se pudo leer o escribir en el disco.'**
  String get errIo;

  /// No description provided for @errInvalidInput.
  ///
  /// In es, this message translates to:
  /// **'Hay un dato que no es válido.'**
  String get errInvalidInput;

  /// No description provided for @errGeneric.
  ///
  /// In es, this message translates to:
  /// **'Algo salió mal.'**
  String get errGeneric;

  /// No description provided for @homeGreeting.
  ///
  /// In es, this message translates to:
  /// **'Hola, {name}'**
  String homeGreeting(Object name);

  /// No description provided for @homeGreetingAnon.
  ///
  /// In es, this message translates to:
  /// **'Te damos la bienvenida'**
  String get homeGreetingAnon;

  /// No description provided for @homeSubtitle.
  ///
  /// In es, this message translates to:
  /// **'¿A qué jugamos hoy?'**
  String get homeSubtitle;

  /// No description provided for @homeContinue.
  ///
  /// In es, this message translates to:
  /// **'Continuar'**
  String get homeContinue;

  /// No description provided for @homeNeverPlayed.
  ///
  /// In es, this message translates to:
  /// **'Sin jugar todavía'**
  String get homeNeverPlayed;

  /// No description provided for @homeChangeInstance.
  ///
  /// In es, this message translates to:
  /// **'Cambiar instancia'**
  String get homeChangeInstance;

  /// No description provided for @homeRecent.
  ///
  /// In es, this message translates to:
  /// **'Recientes'**
  String get homeRecent;

  /// No description provided for @homeSeeAll.
  ///
  /// In es, this message translates to:
  /// **'Ver todas'**
  String get homeSeeAll;

  /// No description provided for @homeNoInstancesTitle.
  ///
  /// In es, this message translates to:
  /// **'Crea tu primera instancia'**
  String get homeNoInstancesTitle;

  /// No description provided for @homeNoInstancesBody.
  ///
  /// In es, this message translates to:
  /// **'Una instancia es una carpeta de juego con su versión, sus mods y sus mundos, separada de las demás.'**
  String get homeNoInstancesBody;

  /// No description provided for @homeSession.
  ///
  /// In es, this message translates to:
  /// **'Sesión'**
  String get homeSession;

  /// No description provided for @homeChangeAccount.
  ///
  /// In es, this message translates to:
  /// **'Cambiar'**
  String get homeChangeAccount;

  /// No description provided for @homeQuickServers.
  ///
  /// In es, this message translates to:
  /// **'Servidores rápidos'**
  String get homeQuickServers;

  /// No description provided for @homeQuickServersSoon.
  ///
  /// In es, this message translates to:
  /// **'Pronto podrás entrar a tus servidores favoritos desde aquí.'**
  String get homeQuickServersSoon;

  /// No description provided for @homeRunningNow.
  ///
  /// In es, this message translates to:
  /// **'En marcha'**
  String get homeRunningNow;

  /// No description provided for @homeNothingRunning.
  ///
  /// In es, this message translates to:
  /// **'Ninguna partida abierta'**
  String get homeNothingRunning;

  /// No description provided for @play.
  ///
  /// In es, this message translates to:
  /// **'Jugar'**
  String get play;

  /// No description provided for @launchPreparing.
  ///
  /// In es, this message translates to:
  /// **'Preparando…'**
  String get launchPreparing;

  /// No description provided for @launchDownloading.
  ///
  /// In es, this message translates to:
  /// **'Descargando {done}/{total}'**
  String launchDownloading(int done, int total);

  /// No description provided for @launchStarting.
  ///
  /// In es, this message translates to:
  /// **'Iniciando…'**
  String get launchStarting;

  /// No description provided for @launchRunning.
  ///
  /// In es, this message translates to:
  /// **'En juego'**
  String get launchRunning;

  /// No description provided for @launchFailed.
  ///
  /// In es, this message translates to:
  /// **'No se pudo iniciar'**
  String get launchFailed;

  /// No description provided for @launchCrashed.
  ///
  /// In es, this message translates to:
  /// **'El juego se cerró con un error'**
  String get launchCrashed;

  /// No description provided for @launchExitCode.
  ///
  /// In es, this message translates to:
  /// **'El juego terminó con el código {code}.'**
  String launchExitCode(int code);

  /// No description provided for @launchCrashReport.
  ///
  /// In es, this message translates to:
  /// **'Informe del fallo: {path}'**
  String launchCrashReport(Object path);

  /// No description provided for @launchSeeLogs.
  ///
  /// In es, this message translates to:
  /// **'Ver registros'**
  String get launchSeeLogs;

  /// No description provided for @stageLibraries.
  ///
  /// In es, this message translates to:
  /// **'Librerías'**
  String get stageLibraries;

  /// No description provided for @stageAssets.
  ///
  /// In es, this message translates to:
  /// **'Recursos'**
  String get stageAssets;

  /// No description provided for @stageOther.
  ///
  /// In es, this message translates to:
  /// **'Archivos'**
  String get stageOther;

  /// No description provided for @playtimeHours.
  ///
  /// In es, this message translates to:
  /// **'{h} h'**
  String playtimeHours(int h);

  /// No description provided for @playtimeMinutes.
  ///
  /// In es, this message translates to:
  /// **'{m} min'**
  String playtimeMinutes(int m);

  /// No description provided for @playtimeHoursMinutes.
  ///
  /// In es, this message translates to:
  /// **'{h} h {m} min'**
  String playtimeHoursMinutes(int h, int m);

  /// No description provided for @playtimeNone.
  ///
  /// In es, this message translates to:
  /// **'0 min'**
  String get playtimeNone;

  /// No description provided for @lastPlayedNever.
  ///
  /// In es, this message translates to:
  /// **'Nunca'**
  String get lastPlayedNever;

  /// No description provided for @relJustNow.
  ///
  /// In es, this message translates to:
  /// **'Ahora mismo'**
  String get relJustNow;

  /// No description provided for @relMinutes.
  ///
  /// In es, this message translates to:
  /// **'{n, plural, one{hace 1 minuto} other{hace {n} minutos}}'**
  String relMinutes(int n);

  /// No description provided for @relHours.
  ///
  /// In es, this message translates to:
  /// **'{n, plural, one{hace 1 hora} other{hace {n} horas}}'**
  String relHours(int n);

  /// No description provided for @relDays.
  ///
  /// In es, this message translates to:
  /// **'{n, plural, one{ayer} other{hace {n} días}}'**
  String relDays(int n);

  /// No description provided for @relMonths.
  ///
  /// In es, this message translates to:
  /// **'{n, plural, one{hace 1 mes} other{hace {n} meses}}'**
  String relMonths(int n);

  /// No description provided for @relYears.
  ///
  /// In es, this message translates to:
  /// **'{n, plural, one{hace 1 año} other{hace {n} años}}'**
  String relYears(int n);

  /// No description provided for @instancesTitle.
  ///
  /// In es, this message translates to:
  /// **'Instancias'**
  String get instancesTitle;

  /// No description provided for @instancesCount.
  ///
  /// In es, this message translates to:
  /// **'{n, plural, =0{Ninguna instancia} one{1 instancia} other{{n} instancias}}'**
  String instancesCount(int n);

  /// No description provided for @instancesSearch.
  ///
  /// In es, this message translates to:
  /// **'Buscar por nombre, versión o loader'**
  String get instancesSearch;

  /// No description provided for @instancesNew.
  ///
  /// In es, this message translates to:
  /// **'Nueva instancia'**
  String get instancesNew;

  /// No description provided for @instancesEmptyTitle.
  ///
  /// In es, this message translates to:
  /// **'Aún no tienes instancias'**
  String get instancesEmptyTitle;

  /// No description provided for @instancesEmptyBody.
  ///
  /// In es, this message translates to:
  /// **'Crea una para elegir versión de Minecraft y loader.'**
  String get instancesEmptyBody;

  /// No description provided for @instancesNoResults.
  ///
  /// In es, this message translates to:
  /// **'Nada coincide con «{q}»'**
  String instancesNoResults(Object q);

  /// No description provided for @viewGrid.
  ///
  /// In es, this message translates to:
  /// **'Cuadrícula'**
  String get viewGrid;

  /// No description provided for @viewList.
  ///
  /// In es, this message translates to:
  /// **'Lista'**
  String get viewList;

  /// No description provided for @instancePlaytime.
  ///
  /// In es, this message translates to:
  /// **'Tiempo jugado'**
  String get instancePlaytime;

  /// No description provided for @instanceLastPlayed.
  ///
  /// In es, this message translates to:
  /// **'Última partida'**
  String get instanceLastPlayed;

  /// No description provided for @instanceOpenFolder.
  ///
  /// In es, this message translates to:
  /// **'Abrir carpeta'**
  String get instanceOpenFolder;

  /// No description provided for @instanceDelete.
  ///
  /// In es, this message translates to:
  /// **'Eliminar instancia'**
  String get instanceDelete;

  /// No description provided for @instanceDeleteTitle.
  ///
  /// In es, this message translates to:
  /// **'¿Eliminar «{name}»?'**
  String instanceDeleteTitle(Object name);

  /// No description provided for @instanceDeleteBody.
  ///
  /// In es, this message translates to:
  /// **'Se moverá a la papelera de {app}, donde podrás recuperarla durante un tiempo.'**
  String instanceDeleteBody(Object app);

  /// No description provided for @instanceDeleted.
  ///
  /// In es, this message translates to:
  /// **'«{name}» se movió a la papelera'**
  String instanceDeleted(Object name);

  /// No description provided for @instanceMore.
  ///
  /// In es, this message translates to:
  /// **'Más opciones'**
  String get instanceMore;

  /// No description provided for @instanceNotFound.
  ///
  /// In es, this message translates to:
  /// **'Esta instancia ya no existe.'**
  String get instanceNotFound;

  /// No description provided for @tabContent.
  ///
  /// In es, this message translates to:
  /// **'Contenido'**
  String get tabContent;

  /// No description provided for @tabWorlds.
  ///
  /// In es, this message translates to:
  /// **'Mundos'**
  String get tabWorlds;

  /// No description provided for @tabScreenshots.
  ///
  /// In es, this message translates to:
  /// **'Capturas'**
  String get tabScreenshots;

  /// No description provided for @tabBackups.
  ///
  /// In es, this message translates to:
  /// **'Copias'**
  String get tabBackups;

  /// No description provided for @tabLogs.
  ///
  /// In es, this message translates to:
  /// **'Registros'**
  String get tabLogs;

  /// No description provided for @tabSettings.
  ///
  /// In es, this message translates to:
  /// **'Ajustes'**
  String get tabSettings;

  /// No description provided for @tabSoonContent.
  ///
  /// In es, this message translates to:
  /// **'Mods, paquetes de recursos y shaders de esta instancia: activar, quitar, actualizar e identificar.'**
  String get tabSoonContent;

  /// No description provided for @tabSoonWorlds.
  ///
  /// In es, this message translates to:
  /// **'Lista de mundos con sus detalles, datapacks y opciones para editarlos o borrarlos.'**
  String get tabSoonWorlds;

  /// No description provided for @tabSoonScreenshots.
  ///
  /// In es, this message translates to:
  /// **'Galería de las capturas que hagas en el juego con F2.'**
  String get tabSoonScreenshots;

  /// No description provided for @tabSoonBackups.
  ///
  /// In es, this message translates to:
  /// **'Copias de seguridad manuales y automáticas, fijarlas y restaurarlas.'**
  String get tabSoonBackups;

  /// No description provided for @createTitle.
  ///
  /// In es, this message translates to:
  /// **'Nueva instancia'**
  String get createTitle;

  /// No description provided for @createName.
  ///
  /// In es, this message translates to:
  /// **'Nombre'**
  String get createName;

  /// No description provided for @createNameHint.
  ///
  /// In es, this message translates to:
  /// **'Mi mundo de supervivencia'**
  String get createNameHint;

  /// No description provided for @createVersion.
  ///
  /// In es, this message translates to:
  /// **'Versión de Minecraft'**
  String get createVersion;

  /// No description provided for @createSnapshots.
  ///
  /// In es, this message translates to:
  /// **'Snapshots'**
  String get createSnapshots;

  /// No description provided for @createSearchVersion.
  ///
  /// In es, this message translates to:
  /// **'Buscar versión'**
  String get createSearchVersion;

  /// No description provided for @createLoader.
  ///
  /// In es, this message translates to:
  /// **'Loader'**
  String get createLoader;

  /// No description provided for @createLoaderVersion.
  ///
  /// In es, this message translates to:
  /// **'Versión del loader'**
  String get createLoaderVersion;

  /// No description provided for @createLoaderLatest.
  ///
  /// In es, this message translates to:
  /// **'La más reciente'**
  String get createLoaderLatest;

  /// No description provided for @createNoLoaderVersions.
  ///
  /// In es, this message translates to:
  /// **'No hay versiones de {loader} para {mc}.'**
  String createNoLoaderVersions(Object loader, Object mc);

  /// No description provided for @createNoVersions.
  ///
  /// In es, this message translates to:
  /// **'No se encontraron versiones.'**
  String get createNoVersions;

  /// No description provided for @createNameRequired.
  ///
  /// In es, this message translates to:
  /// **'Ponle un nombre.'**
  String get createNameRequired;

  /// No description provided for @createDone.
  ///
  /// In es, this message translates to:
  /// **'Instancia «{name}» creada'**
  String createDone(Object name);

  /// No description provided for @createLatestRelease.
  ///
  /// In es, this message translates to:
  /// **'Última'**
  String get createLatestRelease;

  /// No description provided for @loaderVanilla.
  ///
  /// In es, this message translates to:
  /// **'Vanilla'**
  String get loaderVanilla;

  /// No description provided for @loaderCustom.
  ///
  /// In es, this message translates to:
  /// **'Personalizado'**
  String get loaderCustom;

  /// No description provided for @isGeneral.
  ///
  /// In es, this message translates to:
  /// **'General'**
  String get isGeneral;

  /// No description provided for @isName.
  ///
  /// In es, this message translates to:
  /// **'Nombre'**
  String get isName;

  /// No description provided for @isMemory.
  ///
  /// In es, this message translates to:
  /// **'Memoria'**
  String get isMemory;

  /// No description provided for @isMemoryDesc.
  ///
  /// In es, this message translates to:
  /// **'RAM máxima para el juego.'**
  String get isMemoryDesc;

  /// No description provided for @isUseGlobal.
  ///
  /// In es, this message translates to:
  /// **'Usar el valor global'**
  String get isUseGlobal;

  /// No description provided for @isMemoryGlobal.
  ///
  /// In es, this message translates to:
  /// **'Global: {value}'**
  String isMemoryGlobal(Object value);

  /// No description provided for @isMemoryWarning.
  ///
  /// In es, this message translates to:
  /// **'Supera el 75 % de la RAM del equipo ({total}); el sistema podría ir lento.'**
  String isMemoryWarning(Object total);

  /// No description provided for @isJvm.
  ///
  /// In es, this message translates to:
  /// **'Argumentos de JVM'**
  String get isJvm;

  /// No description provided for @isJvmDesc.
  ///
  /// In es, this message translates to:
  /// **'Opciones extra de Java, separadas por espacios.'**
  String get isJvmDesc;

  /// No description provided for @isWindow.
  ///
  /// In es, this message translates to:
  /// **'Ventana del juego'**
  String get isWindow;

  /// No description provided for @isResolution.
  ///
  /// In es, this message translates to:
  /// **'Resolución'**
  String get isResolution;

  /// No description provided for @isResolutionDesc.
  ///
  /// In es, this message translates to:
  /// **'Vacío = tamaño por defecto del juego.'**
  String get isResolutionDesc;

  /// No description provided for @isWidth.
  ///
  /// In es, this message translates to:
  /// **'Ancho'**
  String get isWidth;

  /// No description provided for @isHeight.
  ///
  /// In es, this message translates to:
  /// **'Alto'**
  String get isHeight;

  /// No description provided for @isFullscreen.
  ///
  /// In es, this message translates to:
  /// **'Pantalla completa'**
  String get isFullscreen;

  /// No description provided for @isFullscreenDesc.
  ///
  /// In es, this message translates to:
  /// **'Abrir el juego a pantalla completa.'**
  String get isFullscreenDesc;

  /// No description provided for @saved.
  ///
  /// In es, this message translates to:
  /// **'Cambios guardados'**
  String get saved;

  /// No description provided for @unsavedChanges.
  ///
  /// In es, this message translates to:
  /// **'Hay cambios sin guardar'**
  String get unsavedChanges;

  /// No description provided for @consoleFilter.
  ///
  /// In es, this message translates to:
  /// **'Filtrar líneas'**
  String get consoleFilter;

  /// No description provided for @consoleFollow.
  ///
  /// In es, this message translates to:
  /// **'Seguir al final'**
  String get consoleFollow;

  /// No description provided for @consoleCopy.
  ///
  /// In es, this message translates to:
  /// **'Copiar'**
  String get consoleCopy;

  /// No description provided for @consoleClear.
  ///
  /// In es, this message translates to:
  /// **'Limpiar'**
  String get consoleClear;

  /// No description provided for @consoleEmptyTitle.
  ///
  /// In es, this message translates to:
  /// **'Aún no hay registros'**
  String get consoleEmptyTitle;

  /// No description provided for @consoleEmptyBody.
  ///
  /// In es, this message translates to:
  /// **'Lanza la instancia para ver aquí la salida del juego.'**
  String get consoleEmptyBody;

  /// No description provided for @consoleCopied.
  ///
  /// In es, this message translates to:
  /// **'{n, plural, one{1 línea copiada} other{{n} líneas copiadas}}'**
  String consoleCopied(int n);

  /// No description provided for @consoleLines.
  ///
  /// In es, this message translates to:
  /// **'{n, plural, one{1 línea} other{{n} líneas}}'**
  String consoleLines(int n);

  /// No description provided for @consoleNoMatches.
  ///
  /// In es, this message translates to:
  /// **'Ninguna línea coincide con el filtro.'**
  String get consoleNoMatches;

  /// No description provided for @logLevelError.
  ///
  /// In es, this message translates to:
  /// **'Errores'**
  String get logLevelError;

  /// No description provided for @logLevelWarn.
  ///
  /// In es, this message translates to:
  /// **'Avisos'**
  String get logLevelWarn;

  /// No description provided for @logLevelInfo.
  ///
  /// In es, this message translates to:
  /// **'Info'**
  String get logLevelInfo;

  /// No description provided for @logLevelDebug.
  ///
  /// In es, this message translates to:
  /// **'Depuración'**
  String get logLevelDebug;

  /// No description provided for @settingsTitle.
  ///
  /// In es, this message translates to:
  /// **'Ajustes'**
  String get settingsTitle;

  /// No description provided for @settingsGeneral.
  ///
  /// In es, this message translates to:
  /// **'General'**
  String get settingsGeneral;

  /// No description provided for @settingsLanguage.
  ///
  /// In es, this message translates to:
  /// **'Idioma'**
  String get settingsLanguage;

  /// No description provided for @settingsLanguageDesc.
  ///
  /// In es, this message translates to:
  /// **'Idioma de la interfaz.'**
  String get settingsLanguageDesc;

  /// No description provided for @languageSystem.
  ///
  /// In es, this message translates to:
  /// **'Del sistema'**
  String get languageSystem;

  /// No description provided for @settingsClose.
  ///
  /// In es, this message translates to:
  /// **'Al cerrar la ventana'**
  String get settingsClose;

  /// No description provided for @settingsCloseDesc.
  ///
  /// In es, this message translates to:
  /// **'Qué hacer al pulsar el botón de cerrar.'**
  String get settingsCloseDesc;

  /// No description provided for @closeAsk.
  ///
  /// In es, this message translates to:
  /// **'Preguntar'**
  String get closeAsk;

  /// No description provided for @closeTray.
  ///
  /// In es, this message translates to:
  /// **'Segundo plano'**
  String get closeTray;

  /// No description provided for @closeQuit.
  ///
  /// In es, this message translates to:
  /// **'Salir'**
  String get closeQuit;

  /// No description provided for @settingsMultiple.
  ///
  /// In es, this message translates to:
  /// **'Permitir varias partidas'**
  String get settingsMultiple;

  /// No description provided for @settingsMultipleDesc.
  ///
  /// In es, this message translates to:
  /// **'Abrir más de una instancia a la vez.'**
  String get settingsMultipleDesc;

  /// No description provided for @settingsGame.
  ///
  /// In es, this message translates to:
  /// **'Juego'**
  String get settingsGame;

  /// No description provided for @settingsMemory.
  ///
  /// In es, this message translates to:
  /// **'Memoria por defecto'**
  String get settingsMemory;

  /// No description provided for @settingsMemoryDesc.
  ///
  /// In es, this message translates to:
  /// **'Para las instancias sin un valor propio. Este equipo tiene {total}.'**
  String settingsMemoryDesc(Object total);

  /// No description provided for @settingsJvm.
  ///
  /// In es, this message translates to:
  /// **'Argumentos de JVM por defecto'**
  String get settingsJvm;

  /// No description provided for @settingsJvmDesc.
  ///
  /// In es, this message translates to:
  /// **'Separados por espacios.'**
  String get settingsJvmDesc;

  /// No description provided for @settingsDownloads.
  ///
  /// In es, this message translates to:
  /// **'Descargas'**
  String get settingsDownloads;

  /// No description provided for @settingsConcurrency.
  ///
  /// In es, this message translates to:
  /// **'Descargas simultáneas'**
  String get settingsConcurrency;

  /// No description provided for @settingsConcurrencyDesc.
  ///
  /// In es, this message translates to:
  /// **'Con buena conexión, más es más rápido.'**
  String get settingsConcurrencyDesc;

  /// No description provided for @settingsAbout.
  ///
  /// In es, this message translates to:
  /// **'Acerca de'**
  String get settingsAbout;

  /// No description provided for @settingsAboutBody.
  ///
  /// In es, this message translates to:
  /// **'Launcher de Minecraft para uso personal. Núcleo {version}.'**
  String settingsAboutBody(Object version);

  /// No description provided for @settingsDataDir.
  ///
  /// In es, this message translates to:
  /// **'Carpeta de datos'**
  String get settingsDataDir;

  /// No description provided for @settingsMsa.
  ///
  /// In es, this message translates to:
  /// **'Inicio de sesión de Microsoft'**
  String get settingsMsa;

  /// No description provided for @msaConfigured.
  ///
  /// In es, this message translates to:
  /// **'Disponible'**
  String get msaConfigured;

  /// No description provided for @msaNotConfigured.
  ///
  /// In es, this message translates to:
  /// **'Sin configurar'**
  String get msaNotConfigured;

  /// No description provided for @settingsSaveFailed.
  ///
  /// In es, this message translates to:
  /// **'No se pudieron guardar los ajustes'**
  String get settingsSaveFailed;

  /// No description provided for @memoryGb.
  ///
  /// In es, this message translates to:
  /// **'{value} GB'**
  String memoryGb(Object value);
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'es'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'es':
      return AppLocalizationsEs();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
