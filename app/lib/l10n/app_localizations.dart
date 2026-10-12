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

  /// No description provided for @openFolder.
  ///
  /// In es, this message translates to:
  /// **'Abrir carpeta'**
  String get openFolder;

  /// No description provided for @previous.
  ///
  /// In es, this message translates to:
  /// **'Anterior'**
  String get previous;

  /// No description provided for @next.
  ///
  /// In es, this message translates to:
  /// **'Siguiente'**
  String get next;

  /// No description provided for @install.
  ///
  /// In es, this message translates to:
  /// **'Instalar'**
  String get install;

  /// No description provided for @unknownValue.
  ///
  /// In es, this message translates to:
  /// **'Desconocido'**
  String get unknownValue;

  /// No description provided for @closeGameFirst.
  ///
  /// In es, this message translates to:
  /// **'Cierra el juego de esta instancia primero'**
  String get closeGameFirst;

  /// No description provided for @filterAll.
  ///
  /// In es, this message translates to:
  /// **'Todo'**
  String get filterAll;

  /// No description provided for @filterAny.
  ///
  /// In es, this message translates to:
  /// **'Cualquiera'**
  String get filterAny;

  /// No description provided for @filterVersion.
  ///
  /// In es, this message translates to:
  /// **'Versión'**
  String get filterVersion;

  /// No description provided for @filterLoader.
  ///
  /// In es, this message translates to:
  /// **'Loader'**
  String get filterLoader;

  /// No description provided for @sourceUnknown.
  ///
  /// In es, this message translates to:
  /// **'Desconocido'**
  String get sourceUnknown;

  /// No description provided for @byAuthor.
  ///
  /// In es, this message translates to:
  /// **'de {author}'**
  String byAuthor(Object author);

  /// No description provided for @errCurseforgeKey.
  ///
  /// In es, this message translates to:
  /// **'Falta la clave de API de CurseForge. Añádela en Ajustes › Integraciones.'**
  String get errCurseforgeKey;

  /// No description provided for @errSecrets.
  ///
  /// In es, this message translates to:
  /// **'No se pudo usar el almacén de claves del sistema.'**
  String get errSecrets;

  /// No description provided for @kindMods.
  ///
  /// In es, this message translates to:
  /// **'Mods'**
  String get kindMods;

  /// No description provided for @kindResourcepacks.
  ///
  /// In es, this message translates to:
  /// **'Paquetes de recursos'**
  String get kindResourcepacks;

  /// No description provided for @kindShaders.
  ///
  /// In es, this message translates to:
  /// **'Shaders'**
  String get kindShaders;

  /// No description provided for @kindDatapacks.
  ///
  /// In es, this message translates to:
  /// **'Datapacks'**
  String get kindDatapacks;

  /// No description provided for @kindModpacks.
  ///
  /// In es, this message translates to:
  /// **'Modpacks'**
  String get kindModpacks;

  /// No description provided for @kindMod.
  ///
  /// In es, this message translates to:
  /// **'Mod'**
  String get kindMod;

  /// No description provided for @kindResourcepack.
  ///
  /// In es, this message translates to:
  /// **'Paquete de recursos'**
  String get kindResourcepack;

  /// No description provided for @kindShader.
  ///
  /// In es, this message translates to:
  /// **'Shader'**
  String get kindShader;

  /// No description provided for @kindDatapack.
  ///
  /// In es, this message translates to:
  /// **'Datapack'**
  String get kindDatapack;

  /// No description provided for @kindModpack.
  ///
  /// In es, this message translates to:
  /// **'Modpack'**
  String get kindModpack;

  /// No description provided for @contentAdd.
  ///
  /// In es, this message translates to:
  /// **'Añadir'**
  String get contentAdd;

  /// No description provided for @contentBrowse.
  ///
  /// In es, this message translates to:
  /// **'Explorar contenido'**
  String get contentBrowse;

  /// No description provided for @contentCheckUpdates.
  ///
  /// In es, this message translates to:
  /// **'Buscar actualizaciones'**
  String get contentCheckUpdates;

  /// No description provided for @contentFilterHint.
  ///
  /// In es, this message translates to:
  /// **'Filtrar…'**
  String get contentFilterHint;

  /// No description provided for @contentEnable.
  ///
  /// In es, this message translates to:
  /// **'Activar'**
  String get contentEnable;

  /// No description provided for @contentDisable.
  ///
  /// In es, this message translates to:
  /// **'Desactivar'**
  String get contentDisable;

  /// No description provided for @contentDisabled.
  ///
  /// In es, this message translates to:
  /// **'Desactivado'**
  String get contentDisabled;

  /// No description provided for @contentEmptyTitle.
  ///
  /// In es, this message translates to:
  /// **'Sin contenido todavía'**
  String get contentEmptyTitle;

  /// No description provided for @contentEmptyBody.
  ///
  /// In es, this message translates to:
  /// **'Añade mods, paquetes de recursos o shaders desde Modrinth o CurseForge.'**
  String get contentEmptyBody;

  /// No description provided for @contentNoMatches.
  ///
  /// In es, this message translates to:
  /// **'Nada coincide con el filtro.'**
  String get contentNoMatches;

  /// No description provided for @contentIdentify.
  ///
  /// In es, this message translates to:
  /// **'Identificar'**
  String get contentIdentify;

  /// No description provided for @contentUpdateAll.
  ///
  /// In es, this message translates to:
  /// **'Actualizar todo'**
  String get contentUpdateAll;

  /// No description provided for @contentUpdatesAvailable.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =1{1 actualización disponible} other{{count} actualizaciones disponibles}}'**
  String contentUpdatesAvailable(int count);

  /// No description provided for @contentUnknownFiles.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =1{1 archivo de origen desconocido} other{{count} archivos de origen desconocido}}'**
  String contentUnknownFiles(int count);

  /// No description provided for @contentUpdateTo.
  ///
  /// In es, this message translates to:
  /// **'Actualizar a {version}'**
  String contentUpdateTo(Object version);

  /// No description provided for @contentUpdated.
  ///
  /// In es, this message translates to:
  /// **'{name} actualizado a {version}'**
  String contentUpdated(Object name, Object version);

  /// No description provided for @contentUpdatedAll.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =1{1 archivo actualizado} other{{count} archivos actualizados}}'**
  String contentUpdatedAll(int count);

  /// No description provided for @contentUpdatedSome.
  ///
  /// In es, this message translates to:
  /// **'{ok} actualizados, {failed} con error'**
  String contentUpdatedSome(int ok, int failed);

  /// No description provided for @contentIdentified.
  ///
  /// In es, this message translates to:
  /// **'Reconocidos: {found}. Sin reconocer: {missing}.'**
  String contentIdentified(int found, int missing);

  /// No description provided for @contentRemoveTitle.
  ///
  /// In es, this message translates to:
  /// **'¿Quitar «{name}»?'**
  String contentRemoveTitle(Object name);

  /// No description provided for @contentRemoveBody.
  ///
  /// In es, this message translates to:
  /// **'Se borra el archivo de la instancia.'**
  String get contentRemoveBody;

  /// No description provided for @contentRemoved.
  ///
  /// In es, this message translates to:
  /// **'«{name}» quitado'**
  String contentRemoved(Object name);

  /// No description provided for @modeSurvival.
  ///
  /// In es, this message translates to:
  /// **'Supervivencia'**
  String get modeSurvival;

  /// No description provided for @modeCreative.
  ///
  /// In es, this message translates to:
  /// **'Creativo'**
  String get modeCreative;

  /// No description provided for @modeAdventure.
  ///
  /// In es, this message translates to:
  /// **'Aventura'**
  String get modeAdventure;

  /// No description provided for @modeSpectator.
  ///
  /// In es, this message translates to:
  /// **'Espectador'**
  String get modeSpectator;

  /// No description provided for @diffPeaceful.
  ///
  /// In es, this message translates to:
  /// **'Pacífica'**
  String get diffPeaceful;

  /// No description provided for @diffEasy.
  ///
  /// In es, this message translates to:
  /// **'Fácil'**
  String get diffEasy;

  /// No description provided for @diffNormal.
  ///
  /// In es, this message translates to:
  /// **'Normal'**
  String get diffNormal;

  /// No description provided for @diffHard.
  ///
  /// In es, this message translates to:
  /// **'Difícil'**
  String get diffHard;

  /// No description provided for @worldsEmptyTitle.
  ///
  /// In es, this message translates to:
  /// **'Aún no hay mundos'**
  String get worldsEmptyTitle;

  /// No description provided for @worldsEmptyBody.
  ///
  /// In es, this message translates to:
  /// **'Los mundos que crees al jugar con esta instancia aparecerán aquí.'**
  String get worldsEmptyBody;

  /// No description provided for @worldsNone.
  ///
  /// In es, this message translates to:
  /// **'Esta instancia no tiene mundos todavía.'**
  String get worldsNone;

  /// No description provided for @worldUnreadable.
  ///
  /// In es, this message translates to:
  /// **'No se pudo leer level.dat'**
  String get worldUnreadable;

  /// No description provided for @worldRename.
  ///
  /// In es, this message translates to:
  /// **'Cambiar nombre'**
  String get worldRename;

  /// No description provided for @worldName.
  ///
  /// In es, this message translates to:
  /// **'Nombre del mundo'**
  String get worldName;

  /// No description provided for @worldLockedRunning.
  ///
  /// In es, this message translates to:
  /// **'El juego está abierto: ciérralo para editar el mundo.'**
  String get worldLockedRunning;

  /// No description provided for @worldGame.
  ///
  /// In es, this message translates to:
  /// **'Partida'**
  String get worldGame;

  /// No description provided for @worldMode.
  ///
  /// In es, this message translates to:
  /// **'Modo de juego'**
  String get worldMode;

  /// No description provided for @worldDifficulty.
  ///
  /// In es, this message translates to:
  /// **'Dificultad'**
  String get worldDifficulty;

  /// No description provided for @worldHardcore.
  ///
  /// In es, this message translates to:
  /// **'Hardcore'**
  String get worldHardcore;

  /// No description provided for @worldHardcoreDesc.
  ///
  /// In es, this message translates to:
  /// **'Una sola vida: al morir, el mundo queda en modo espectador.'**
  String get worldHardcoreDesc;

  /// No description provided for @worldCheats.
  ///
  /// In es, this message translates to:
  /// **'Permitir trucos'**
  String get worldCheats;

  /// No description provided for @worldCheatsDesc.
  ///
  /// In es, this message translates to:
  /// **'Habilita los comandos como /gamemode o /tp.'**
  String get worldCheatsDesc;

  /// No description provided for @worldDelete.
  ///
  /// In es, this message translates to:
  /// **'Borrar el mundo'**
  String get worldDelete;

  /// No description provided for @worldDeleteDesc.
  ///
  /// In es, this message translates to:
  /// **'Se borra para siempre, sin pasar por la papelera.'**
  String get worldDeleteDesc;

  /// No description provided for @worldDeleteTitle.
  ///
  /// In es, this message translates to:
  /// **'¿Borrar «{name}»?'**
  String worldDeleteTitle(Object name);

  /// No description provided for @worldDeleteBody.
  ///
  /// In es, this message translates to:
  /// **'El mundo se borra para siempre y no se puede recuperar. Si quieres conservarlo, crea antes una copia en la pestaña Copias.'**
  String get worldDeleteBody;

  /// No description provided for @worldDeleteAction.
  ///
  /// In es, this message translates to:
  /// **'Borrar para siempre'**
  String get worldDeleteAction;

  /// No description provided for @worldDeleted.
  ///
  /// In es, this message translates to:
  /// **'Mundo «{name}» borrado'**
  String worldDeleted(Object name);

  /// No description provided for @datapacksAdd.
  ///
  /// In es, this message translates to:
  /// **'Añadir datapacks'**
  String get datapacksAdd;

  /// No description provided for @datapacksEmpty.
  ///
  /// In es, this message translates to:
  /// **'Este mundo no tiene datapacks.'**
  String get datapacksEmpty;

  /// No description provided for @datapackRemoveBody.
  ///
  /// In es, this message translates to:
  /// **'Se borra el datapack de este mundo.'**
  String get datapackRemoveBody;

  /// No description provided for @screenshotsEmptyTitle.
  ///
  /// In es, this message translates to:
  /// **'Sin capturas'**
  String get screenshotsEmptyTitle;

  /// No description provided for @screenshotsEmptyBody.
  ///
  /// In es, this message translates to:
  /// **'Pulsa F2 mientras juegas y tus capturas aparecerán aquí.'**
  String get screenshotsEmptyBody;

  /// No description provided for @screenshotsCount.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =1{1 captura} other{{count} capturas}}'**
  String screenshotsCount(int count);

  /// No description provided for @screenshotOpen.
  ///
  /// In es, this message translates to:
  /// **'Abrir con el visor del sistema'**
  String get screenshotOpen;

  /// No description provided for @screenshotDeleteTitle.
  ///
  /// In es, this message translates to:
  /// **'¿Borrar la captura?'**
  String get screenshotDeleteTitle;

  /// No description provided for @screenshotDeleteBody.
  ///
  /// In es, this message translates to:
  /// **'Se borra «{file}» del disco.'**
  String screenshotDeleteBody(Object file);

  /// No description provided for @backupsTitle.
  ///
  /// In es, this message translates to:
  /// **'Copias de seguridad'**
  String get backupsTitle;

  /// No description provided for @backupsSummary.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =1{1 copia} other{{count} copias}} · {size} en total'**
  String backupsSummary(int count, Object size);

  /// No description provided for @backupsEmptyTitle.
  ///
  /// In es, this message translates to:
  /// **'Sin copias todavía'**
  String get backupsEmptyTitle;

  /// No description provided for @backupsEmptyBody.
  ///
  /// In es, this message translates to:
  /// **'Crea una copia antes de cambios grandes. Incluye mundos, mods y configuración (no los registros).'**
  String get backupsEmptyBody;

  /// No description provided for @backupCreate.
  ///
  /// In es, this message translates to:
  /// **'Crear copia'**
  String get backupCreate;

  /// No description provided for @backupCreateTitle.
  ///
  /// In es, this message translates to:
  /// **'Nueva copia de seguridad'**
  String get backupCreateTitle;

  /// No description provided for @backupCreateBody.
  ///
  /// In es, this message translates to:
  /// **'Se guarda todo el contenido de la instancia: mundos, mods, recursos y configuración.'**
  String get backupCreateBody;

  /// No description provided for @backupLabelOptional.
  ///
  /// In es, this message translates to:
  /// **'Etiqueta (opcional)'**
  String get backupLabelOptional;

  /// No description provided for @backupLabelHint.
  ///
  /// In es, this message translates to:
  /// **'Antes de instalar Create'**
  String get backupLabelHint;

  /// No description provided for @backupLabel.
  ///
  /// In es, this message translates to:
  /// **'Etiqueta'**
  String get backupLabel;

  /// No description provided for @backupLabelTitle.
  ///
  /// In es, this message translates to:
  /// **'Cambiar etiqueta'**
  String get backupLabelTitle;

  /// No description provided for @backupPinOnCreate.
  ///
  /// In es, this message translates to:
  /// **'Fijar esta copia'**
  String get backupPinOnCreate;

  /// No description provided for @backupPinnedNote.
  ///
  /// In es, this message translates to:
  /// **'Las copias fijadas nunca se borran solas.'**
  String get backupPinnedNote;

  /// No description provided for @backupCreating.
  ///
  /// In es, this message translates to:
  /// **'Creando la copia…'**
  String get backupCreating;

  /// No description provided for @backupCreated.
  ///
  /// In es, this message translates to:
  /// **'Copia creada'**
  String get backupCreated;

  /// No description provided for @backupAutomatic.
  ///
  /// In es, this message translates to:
  /// **'Copia automática'**
  String get backupAutomatic;

  /// No description provided for @backupManual.
  ///
  /// In es, this message translates to:
  /// **'Copia manual'**
  String get backupManual;

  /// No description provided for @backupAutoBadge.
  ///
  /// In es, this message translates to:
  /// **'Automática'**
  String get backupAutoBadge;

  /// No description provided for @backupPinnedBadge.
  ///
  /// In es, this message translates to:
  /// **'Fijada'**
  String get backupPinnedBadge;

  /// No description provided for @backupPin.
  ///
  /// In es, this message translates to:
  /// **'Fijar'**
  String get backupPin;

  /// No description provided for @backupUnpin.
  ///
  /// In es, this message translates to:
  /// **'Dejar de fijar'**
  String get backupUnpin;

  /// No description provided for @backupRestore.
  ///
  /// In es, this message translates to:
  /// **'Restaurar'**
  String get backupRestore;

  /// No description provided for @backupDeleteTitle.
  ///
  /// In es, this message translates to:
  /// **'¿Eliminar la copia?'**
  String get backupDeleteTitle;

  /// No description provided for @backupDeleteBody.
  ///
  /// In es, this message translates to:
  /// **'Se borra «{name}» del disco.'**
  String backupDeleteBody(Object name);

  /// No description provided for @backupAutoTitle.
  ///
  /// In es, this message translates to:
  /// **'Copias automáticas'**
  String get backupAutoTitle;

  /// No description provided for @backupAutoDesc.
  ///
  /// In es, this message translates to:
  /// **'Mientras la app está abierta y el juego cerrado.'**
  String get backupAutoDesc;

  /// No description provided for @backupInterval.
  ///
  /// In es, this message translates to:
  /// **'Frecuencia'**
  String get backupInterval;

  /// No description provided for @backupDaily.
  ///
  /// In es, this message translates to:
  /// **'A diario'**
  String get backupDaily;

  /// No description provided for @backupWeekly.
  ///
  /// In es, this message translates to:
  /// **'Semanal'**
  String get backupWeekly;

  /// No description provided for @backupEveryHours.
  ///
  /// In es, this message translates to:
  /// **'Cada {hours} h'**
  String backupEveryHours(int hours);

  /// No description provided for @backupEveryDays.
  ///
  /// In es, this message translates to:
  /// **'Cada {days} días'**
  String backupEveryDays(int days);

  /// No description provided for @backupKeep.
  ///
  /// In es, this message translates to:
  /// **'Copias automáticas que se conservan'**
  String get backupKeep;

  /// No description provided for @backupKeepCount.
  ///
  /// In es, this message translates to:
  /// **'{count}'**
  String backupKeepCount(int count);

  /// No description provided for @backupRestoreTitle.
  ///
  /// In es, this message translates to:
  /// **'Restaurar copia'**
  String get backupRestoreTitle;

  /// No description provided for @backupRestoreFrom.
  ///
  /// In es, this message translates to:
  /// **'Copia «{name}» del {date}.'**
  String backupRestoreFrom(Object name, Object date);

  /// No description provided for @backupModeFull.
  ///
  /// In es, this message translates to:
  /// **'Todo'**
  String get backupModeFull;

  /// No description provided for @backupModeFullDesc.
  ///
  /// In es, this message translates to:
  /// **'Mundos, mods, recursos, shaders y configuración del juego.'**
  String get backupModeFullDesc;

  /// No description provided for @backupModeWorlds.
  ///
  /// In es, this message translates to:
  /// **'Solo los mundos'**
  String get backupModeWorlds;

  /// No description provided for @backupModeWorldsDesc.
  ///
  /// In es, this message translates to:
  /// **'Sustituye la carpeta de mundos; los mods y la configuración se quedan como están.'**
  String get backupModeWorldsDesc;

  /// No description provided for @backupWarnFull.
  ///
  /// In es, this message translates to:
  /// **'Todo lo que hay ahora en la instancia se sustituye por la copia. Los cambios posteriores a la copia se pierden.'**
  String get backupWarnFull;

  /// No description provided for @backupWarnWorlds.
  ///
  /// In es, this message translates to:
  /// **'Los mundos actuales se sustituyen por los de la copia. El progreso posterior a la copia se pierde.'**
  String get backupWarnWorlds;

  /// No description provided for @backupSafety.
  ///
  /// In es, this message translates to:
  /// **'Crear antes una copia del estado actual'**
  String get backupSafety;

  /// No description provided for @backupSafetyLabel.
  ///
  /// In es, this message translates to:
  /// **'Antes de restaurar'**
  String get backupSafetyLabel;

  /// No description provided for @backupRestoreFull.
  ///
  /// In es, this message translates to:
  /// **'Restaurar todo'**
  String get backupRestoreFull;

  /// No description provided for @backupRestoreWorlds.
  ///
  /// In es, this message translates to:
  /// **'Restaurar mundos'**
  String get backupRestoreWorlds;

  /// No description provided for @backupRestoring.
  ///
  /// In es, this message translates to:
  /// **'Restaurando…'**
  String get backupRestoring;

  /// No description provided for @backupRestored.
  ///
  /// In es, this message translates to:
  /// **'Copia «{name}» restaurada'**
  String backupRestored(Object name);

  /// No description provided for @contentTitle.
  ///
  /// In es, this message translates to:
  /// **'Contenido'**
  String get contentTitle;

  /// No description provided for @contentSubtitle.
  ///
  /// In es, this message translates to:
  /// **'Mods, paquetes de recursos, shaders, datapacks y modpacks.'**
  String get contentSubtitle;

  /// No description provided for @contentSearchHint.
  ///
  /// In es, this message translates to:
  /// **'Buscar {kind}…'**
  String contentSearchHint(Object kind);

  /// No description provided for @contentResults.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =1{1 resultado} other{{count} resultados}}'**
  String contentResults(int count);

  /// No description provided for @contentNoResultsTitle.
  ///
  /// In es, this message translates to:
  /// **'Sin resultados'**
  String get contentNoResultsTitle;

  /// No description provided for @contentNoResultsBody.
  ///
  /// In es, this message translates to:
  /// **'Prueba con otras palabras o quita algún filtro.'**
  String get contentNoResultsBody;

  /// No description provided for @contentEnd.
  ///
  /// In es, this message translates to:
  /// **'No hay más resultados'**
  String get contentEnd;

  /// No description provided for @contentInstalled.
  ///
  /// In es, this message translates to:
  /// **'Instalado'**
  String get contentInstalled;

  /// No description provided for @contentAlreadyInstalled.
  ///
  /// In es, this message translates to:
  /// **'Ya está instalado en esta instancia ({version}).'**
  String contentAlreadyInstalled(Object version);

  /// No description provided for @contentTargetPrefix.
  ///
  /// In es, this message translates to:
  /// **'Añadiendo a '**
  String get contentTargetPrefix;

  /// No description provided for @contentTargetWorld.
  ///
  /// In es, this message translates to:
  /// **'mundo «{world}»'**
  String contentTargetWorld(Object world);

  /// No description provided for @contentBackToInstance.
  ///
  /// In es, this message translates to:
  /// **'Volver a la instancia'**
  String get contentBackToInstance;

  /// No description provided for @contentClearTarget.
  ///
  /// In es, this message translates to:
  /// **'Buscar sin instancia'**
  String get contentClearTarget;

  /// No description provided for @sortRelevance.
  ///
  /// In es, this message translates to:
  /// **'Relevancia'**
  String get sortRelevance;

  /// No description provided for @sortDownloads.
  ///
  /// In es, this message translates to:
  /// **'Descargas'**
  String get sortDownloads;

  /// No description provided for @sortFollows.
  ///
  /// In es, this message translates to:
  /// **'Seguidores'**
  String get sortFollows;

  /// No description provided for @sortNewest.
  ///
  /// In es, this message translates to:
  /// **'Más nuevos'**
  String get sortNewest;

  /// No description provided for @sortUpdated.
  ///
  /// In es, this message translates to:
  /// **'Actualizados'**
  String get sortUpdated;

  /// No description provided for @installIn.
  ///
  /// In es, this message translates to:
  /// **'Instalar en'**
  String get installIn;

  /// No description provided for @installInWorld.
  ///
  /// In es, this message translates to:
  /// **'Mundo'**
  String get installInWorld;

  /// No description provided for @chooseInstance.
  ///
  /// In es, this message translates to:
  /// **'Elige una instancia'**
  String get chooseInstance;

  /// No description provided for @chooseInstanceFirst.
  ///
  /// In es, this message translates to:
  /// **'Elige una instancia para ver las versiones compatibles.'**
  String get chooseInstanceFirst;

  /// No description provided for @chooseWorld.
  ///
  /// In es, this message translates to:
  /// **'Elige un mundo'**
  String get chooseWorld;

  /// No description provided for @installLatest.
  ///
  /// In es, this message translates to:
  /// **'Instalar la última'**
  String get installLatest;

  /// No description provided for @versionsCompatible.
  ///
  /// In es, this message translates to:
  /// **'Versiones compatibles'**
  String get versionsCompatible;

  /// No description provided for @versionsNoneCompatible.
  ///
  /// In es, this message translates to:
  /// **'No hay versiones para esta versión de Minecraft y este loader.'**
  String get versionsNoneCompatible;

  /// No description provided for @releaseStable.
  ///
  /// In es, this message translates to:
  /// **'Estable'**
  String get releaseStable;

  /// No description provided for @releaseBeta.
  ///
  /// In es, this message translates to:
  /// **'Beta'**
  String get releaseBeta;

  /// No description provided for @releaseAlpha.
  ///
  /// In es, this message translates to:
  /// **'Alfa'**
  String get releaseAlpha;

  /// No description provided for @installedIn.
  ///
  /// In es, this message translates to:
  /// **'{name} instalado en {instance}'**
  String installedIn(Object name, Object instance);

  /// No description provided for @installedWithDeps.
  ///
  /// In es, this message translates to:
  /// **'{name} instalado en {instance} con {count, plural, =1{1 dependencia} other{{count} dependencias}}'**
  String installedWithDeps(Object name, Object instance, int count);

  /// No description provided for @installPartial.
  ///
  /// In es, this message translates to:
  /// **'{name} instalado, pero se omitieron {count, plural, =1{1 dependencia} other{{count} dependencias}}'**
  String installPartial(Object name, int count);

  /// No description provided for @cfKeyNeededTitle.
  ///
  /// In es, this message translates to:
  /// **'CurseForge necesita una clave de API'**
  String get cfKeyNeededTitle;

  /// No description provided for @cfKeyNeededBody.
  ///
  /// In es, this message translates to:
  /// **'CurseForge solo permite buscar con una clave personal. Puedes pedirla gratis en console.curseforge.com y guardarla aquí; se queda en el llavero del sistema.'**
  String get cfKeyNeededBody;

  /// No description provided for @cfKeyAdd.
  ///
  /// In es, this message translates to:
  /// **'Añadir clave'**
  String get cfKeyAdd;

  /// No description provided for @cfKeyChange.
  ///
  /// In es, this message translates to:
  /// **'Cambiar clave'**
  String get cfKeyChange;

  /// No description provided for @cfKeyTitle.
  ///
  /// In es, this message translates to:
  /// **'Clave de API de CurseForge'**
  String get cfKeyTitle;

  /// No description provided for @cfKeyLabel.
  ///
  /// In es, this message translates to:
  /// **'Clave de API'**
  String get cfKeyLabel;

  /// No description provided for @cfKeyHelper.
  ///
  /// In es, this message translates to:
  /// **'Se guarda en el llavero del sistema y no se vuelve a mostrar.'**
  String get cfKeyHelper;

  /// No description provided for @cfKeySaved.
  ///
  /// In es, this message translates to:
  /// **'Clave guardada'**
  String get cfKeySaved;

  /// No description provided for @cfKeySet.
  ///
  /// In es, this message translates to:
  /// **'Configurada'**
  String get cfKeySet;

  /// No description provided for @cfKeyNotSet.
  ///
  /// In es, this message translates to:
  /// **'Sin configurar'**
  String get cfKeyNotSet;

  /// No description provided for @cfKeyRemoveTitle.
  ///
  /// In es, this message translates to:
  /// **'¿Quitar la clave de CurseForge?'**
  String get cfKeyRemoveTitle;

  /// No description provided for @cfKeyRemoveBody.
  ///
  /// In es, this message translates to:
  /// **'Dejarás de poder buscar en CurseForge hasta que añadas otra.'**
  String get cfKeyRemoveBody;

  /// No description provided for @modpackImport.
  ///
  /// In es, this message translates to:
  /// **'Importar modpack'**
  String get modpackImport;

  /// No description provided for @modpackImportTitle.
  ///
  /// In es, this message translates to:
  /// **'Importar un modpack'**
  String get modpackImportTitle;

  /// No description provided for @modpackInstallTitle.
  ///
  /// In es, this message translates to:
  /// **'Instalar «{name}»'**
  String modpackInstallTitle(Object name);

  /// No description provided for @modpackBody.
  ///
  /// In es, this message translates to:
  /// **'Se crea una instancia nueva con el Minecraft, el loader y los mods del modpack.'**
  String get modpackBody;

  /// No description provided for @modpackFromFile.
  ///
  /// In es, this message translates to:
  /// **'Archivo'**
  String get modpackFromFile;

  /// No description provided for @modpackFromUrl.
  ///
  /// In es, this message translates to:
  /// **'URL'**
  String get modpackFromUrl;

  /// No description provided for @modpackChooseFile.
  ///
  /// In es, this message translates to:
  /// **'Elegir un .mrpack o .zip…'**
  String get modpackChooseFile;

  /// No description provided for @modpackFiles.
  ///
  /// In es, this message translates to:
  /// **'Modpacks'**
  String get modpackFiles;

  /// No description provided for @modpackName.
  ///
  /// In es, this message translates to:
  /// **'Nombre de la instancia'**
  String get modpackName;

  /// No description provided for @modpackNameHint.
  ///
  /// In es, this message translates to:
  /// **'El del modpack'**
  String get modpackNameHint;

  /// No description provided for @modpackInstall.
  ///
  /// In es, this message translates to:
  /// **'Instalar modpack'**
  String get modpackInstall;

  /// No description provided for @modpackInstalling.
  ///
  /// In es, this message translates to:
  /// **'Descargando los archivos del modpack…'**
  String get modpackInstalling;

  /// No description provided for @modpackInstallingHint.
  ///
  /// In es, this message translates to:
  /// **'Puede tardar unos minutos. Si algo falla, la instancia a medias se borra.'**
  String get modpackInstallingHint;

  /// No description provided for @modpackDoneTitle.
  ///
  /// In es, this message translates to:
  /// **'Modpack instalado'**
  String get modpackDoneTitle;

  /// No description provided for @modpackDoneBody.
  ///
  /// In es, this message translates to:
  /// **'Se descargaron {done} de {total} archivos.'**
  String modpackDoneBody(int done, int total);

  /// No description provided for @modpackOpenInstance.
  ///
  /// In es, this message translates to:
  /// **'Abrir instancia'**
  String get modpackOpenInstance;

  /// No description provided for @serversTitle.
  ///
  /// In es, this message translates to:
  /// **'Servidores'**
  String get serversTitle;

  /// No description provided for @serversSubtitle.
  ///
  /// In es, this message translates to:
  /// **'Tus servidores con su estado, listos para entrar.'**
  String get serversSubtitle;

  /// No description provided for @serversCount.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =1{1 servidor} other{{count} servidores}} · {online} en línea'**
  String serversCount(int count, int online);

  /// No description provided for @serversRefresh.
  ///
  /// In es, this message translates to:
  /// **'Actualizar estado'**
  String get serversRefresh;

  /// No description provided for @serversImport.
  ///
  /// In es, this message translates to:
  /// **'Importar'**
  String get serversImport;

  /// No description provided for @serversImported.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =1{1 servidor importado} other{{count} servidores importados}} de {instance}'**
  String serversImported(int count, Object instance);

  /// No description provided for @serversImportNone.
  ///
  /// In es, this message translates to:
  /// **'{instance} no tiene servidores nuevos'**
  String serversImportNone(Object instance);

  /// No description provided for @serversEmptyTitle.
  ///
  /// In es, this message translates to:
  /// **'Aún no hay servidores'**
  String get serversEmptyTitle;

  /// No description provided for @serversEmptyBody.
  ///
  /// In es, this message translates to:
  /// **'Añade uno a mano o impórtalos de la lista de servidores de una instancia.'**
  String get serversEmptyBody;

  /// No description provided for @serverAdd.
  ///
  /// In es, this message translates to:
  /// **'Añadir servidor'**
  String get serverAdd;

  /// No description provided for @serverEdit.
  ///
  /// In es, this message translates to:
  /// **'Editar servidor'**
  String get serverEdit;

  /// No description provided for @serverAddress.
  ///
  /// In es, this message translates to:
  /// **'Dirección'**
  String get serverAddress;

  /// No description provided for @serverAddressInvalid.
  ///
  /// In es, this message translates to:
  /// **'Escribe una dirección válida (host o host:puerto).'**
  String get serverAddressInvalid;

  /// No description provided for @serverName.
  ///
  /// In es, this message translates to:
  /// **'Nombre (opcional)'**
  String get serverName;

  /// No description provided for @serverNameHint.
  ///
  /// In es, this message translates to:
  /// **'El de la dirección'**
  String get serverNameHint;

  /// No description provided for @serverInstance.
  ///
  /// In es, this message translates to:
  /// **'Instancia con la que entrar'**
  String get serverInstance;

  /// No description provided for @serverInstanceNone.
  ///
  /// In es, this message translates to:
  /// **'Ninguna'**
  String get serverInstanceNone;

  /// No description provided for @serverAccount.
  ///
  /// In es, this message translates to:
  /// **'Cuenta'**
  String get serverAccount;

  /// No description provided for @serverAccountActive.
  ///
  /// In es, this message translates to:
  /// **'La cuenta activa'**
  String get serverAccountActive;

  /// No description provided for @serverJoin.
  ///
  /// In es, this message translates to:
  /// **'Entrar'**
  String get serverJoin;

  /// No description provided for @serverJoinWith.
  ///
  /// In es, this message translates to:
  /// **'Entrar con {instance}'**
  String serverJoinWith(Object instance);

  /// No description provided for @serverAlreadyRunning.
  ///
  /// In es, this message translates to:
  /// **'{instance} ya está abierta'**
  String serverAlreadyRunning(Object instance);

  /// No description provided for @serverNoInstance.
  ///
  /// In es, this message translates to:
  /// **'Sin instancia asignada'**
  String get serverNoInstance;

  /// No description provided for @serverOffline.
  ///
  /// In es, this message translates to:
  /// **'Sin conexión'**
  String get serverOffline;

  /// No description provided for @serverUnreachable.
  ///
  /// In es, this message translates to:
  /// **'No responde'**
  String get serverUnreachable;

  /// No description provided for @serverPinging.
  ///
  /// In es, this message translates to:
  /// **'Consultando…'**
  String get serverPinging;

  /// No description provided for @serverPing.
  ///
  /// In es, this message translates to:
  /// **'{ms} ms'**
  String serverPing(int ms);

  /// No description provided for @serverCopyAddress.
  ///
  /// In es, this message translates to:
  /// **'Copiar dirección'**
  String get serverCopyAddress;

  /// No description provided for @serverRemoveTitle.
  ///
  /// In es, this message translates to:
  /// **'¿Quitar «{name}»?'**
  String serverRemoveTitle(Object name);

  /// No description provided for @serverRemoveBody.
  ///
  /// In es, this message translates to:
  /// **'Se quita de la lista. Puedes volver a añadirlo cuando quieras.'**
  String get serverRemoveBody;

  /// No description provided for @homeQuickServersEmpty.
  ///
  /// In es, this message translates to:
  /// **'Añade tus servidores favoritos para entrar con un clic.'**
  String get homeQuickServersEmpty;

  /// No description provided for @settingsIntegrations.
  ///
  /// In es, this message translates to:
  /// **'Integraciones'**
  String get settingsIntegrations;

  /// No description provided for @settingsCurseforge.
  ///
  /// In es, this message translates to:
  /// **'CurseForge'**
  String get settingsCurseforge;

  /// No description provided for @settingsCurseforgeDesc.
  ///
  /// In es, this message translates to:
  /// **'Clave de API personal para buscar e instalar desde CurseForge.'**
  String get settingsCurseforgeDesc;

  /// No description provided for @settingsDiscord.
  ///
  /// In es, this message translates to:
  /// **'Discord Rich Presence'**
  String get settingsDiscord;

  /// No description provided for @settingsDiscordDesc.
  ///
  /// In es, this message translates to:
  /// **'Muestra en Discord a qué estás jugando.'**
  String get settingsDiscordDesc;

  /// No description provided for @serverPlayersText.
  ///
  /// In es, this message translates to:
  /// **'{online} / {max}'**
  String serverPlayersText(Object online, Object max);

  /// No description provided for @homeSeeAllServers.
  ///
  /// In es, this message translates to:
  /// **'Ver todos'**
  String get homeSeeAllServers;

  /// No description provided for @settingsAppearance.
  ///
  /// In es, this message translates to:
  /// **'Apariencia'**
  String get settingsAppearance;

  /// No description provided for @appearanceBackground.
  ///
  /// In es, this message translates to:
  /// **'Fondo'**
  String get appearanceBackground;

  /// No description provided for @appearanceBackgroundDesc.
  ///
  /// In es, this message translates to:
  /// **'Detrás de toda la app: el del estilo o una imagen tuya (se copia a la carpeta de datos).'**
  String get appearanceBackgroundDesc;

  /// No description provided for @appearanceAurora.
  ///
  /// In es, this message translates to:
  /// **'Aurora'**
  String get appearanceAurora;

  /// No description provided for @appearanceLandscape.
  ///
  /// In es, this message translates to:
  /// **'Paisaje'**
  String get appearanceLandscape;

  /// No description provided for @appearanceImage.
  ///
  /// In es, this message translates to:
  /// **'Imagen propia'**
  String get appearanceImage;

  /// No description provided for @appearancePickImage.
  ///
  /// In es, this message translates to:
  /// **'Elegir imagen…'**
  String get appearancePickImage;

  /// No description provided for @appearanceChangeImage.
  ///
  /// In es, this message translates to:
  /// **'Cambiar imagen'**
  String get appearanceChangeImage;

  /// No description provided for @appearanceImages.
  ///
  /// In es, this message translates to:
  /// **'Imágenes'**
  String get appearanceImages;

  /// No description provided for @appearanceStyle.
  ///
  /// In es, this message translates to:
  /// **'Estilo'**
  String get appearanceStyle;

  /// No description provided for @appearanceStyleDesc.
  ///
  /// In es, this message translates to:
  /// **'Colores, formas y fondo de toda la app. Debajo puedes cambiar el fondo y el acento.'**
  String get appearanceStyleDesc;

  /// No description provided for @styleGraphite.
  ///
  /// In es, this message translates to:
  /// **'Grafito'**
  String get styleGraphite;

  /// No description provided for @styleSebaspi.
  ///
  /// In es, this message translates to:
  /// **'sebaspi'**
  String get styleSebaspi;

  /// No description provided for @appearanceBackgroundStyle.
  ///
  /// In es, this message translates to:
  /// **'El del estilo'**
  String get appearanceBackgroundStyle;

  /// No description provided for @appearanceAccentStyle.
  ///
  /// In es, this message translates to:
  /// **'Del estilo'**
  String get appearanceAccentStyle;

  /// No description provided for @appearanceAccentStyleDesc.
  ///
  /// In es, this message translates to:
  /// **'El color propio del estilo.'**
  String get appearanceAccentStyleDesc;

  /// No description provided for @appearanceAccent.
  ///
  /// In es, this message translates to:
  /// **'Color de acento'**
  String get appearanceAccent;

  /// No description provided for @appearanceAccentDesc.
  ///
  /// In es, this message translates to:
  /// **'Botones, selección y detalles.'**
  String get appearanceAccentDesc;

  /// No description provided for @appearanceAccentAuto.
  ///
  /// In es, this message translates to:
  /// **'Auto'**
  String get appearanceAccentAuto;

  /// No description provided for @appearanceAccentAutoDesc.
  ///
  /// In es, this message translates to:
  /// **'Automático: se toma de tu imagen de fondo.'**
  String get appearanceAccentAutoDesc;

  /// No description provided for @appearanceBlur.
  ///
  /// In es, this message translates to:
  /// **'Desenfoque'**
  String get appearanceBlur;

  /// No description provided for @appearanceBlurDesc.
  ///
  /// In es, this message translates to:
  /// **'Del paisaje o de tu imagen.'**
  String get appearanceBlurDesc;

  /// No description provided for @appearanceDim.
  ///
  /// In es, this message translates to:
  /// **'Oscurecido'**
  String get appearanceDim;

  /// No description provided for @appearanceDimDesc.
  ///
  /// In es, this message translates to:
  /// **'Para que el texto se lea bien sobre el fondo.'**
  String get appearanceDimDesc;

  /// No description provided for @appearancePerformance.
  ///
  /// In es, this message translates to:
  /// **'Modo rendimiento'**
  String get appearancePerformance;

  /// No description provided for @appearancePerformanceDesc.
  ///
  /// In es, this message translates to:
  /// **'Sin desenfoque ni animaciones de fondo, y cambio de página instantáneo. Útil en equipos modestos.'**
  String get appearancePerformanceDesc;

  /// No description provided for @settingsAutostart.
  ///
  /// In es, this message translates to:
  /// **'Abrir al iniciar sesión'**
  String get settingsAutostart;

  /// No description provided for @settingsAutostartDesc.
  ///
  /// In es, this message translates to:
  /// **'{app} arranca con el sistema, oculto en la bandeja.'**
  String settingsAutostartDesc(Object app);

  /// No description provided for @settingsJava.
  ///
  /// In es, this message translates to:
  /// **'Java'**
  String get settingsJava;

  /// No description provided for @javaDetected.
  ///
  /// In es, this message translates to:
  /// **'Javas detectadas'**
  String get javaDetected;

  /// No description provided for @javaDetectedDesc.
  ///
  /// In es, this message translates to:
  /// **'Para cada versión se usa la adecuada; si falta, se instala sola al jugar.'**
  String get javaDetectedDesc;

  /// No description provided for @javaRescan.
  ///
  /// In es, this message translates to:
  /// **'Volver a buscar'**
  String get javaRescan;

  /// No description provided for @javaNone.
  ///
  /// In es, this message translates to:
  /// **'No se encontró ninguna Java. Se instalará la necesaria al jugar.'**
  String get javaNone;

  /// No description provided for @javaInstall.
  ///
  /// In es, this message translates to:
  /// **'Instalar Java'**
  String get javaInstall;

  /// No description provided for @javaInstallDesc.
  ///
  /// In es, this message translates to:
  /// **'Descarga un JRE de Eclipse Adoptium a la carpeta de datos.'**
  String get javaInstallDesc;

  /// No description provided for @javaInstalling.
  ///
  /// In es, this message translates to:
  /// **'Instalando Java {major}…'**
  String javaInstalling(int major);

  /// No description provided for @javaInstallingProgress.
  ///
  /// In es, this message translates to:
  /// **'Instalando Java {major}: {done} de {total}'**
  String javaInstallingProgress(int major, Object done, Object total);

  /// No description provided for @javaInstalled.
  ///
  /// In es, this message translates to:
  /// **'Java {major} instalada ({version})'**
  String javaInstalled(int major, Object version);

  /// No description provided for @javaManaged.
  ///
  /// In es, this message translates to:
  /// **'Gestionada'**
  String get javaManaged;

  /// No description provided for @javaOverride.
  ///
  /// In es, this message translates to:
  /// **'Elegida a mano'**
  String get javaOverride;

  /// No description provided for @javaSystem.
  ///
  /// In es, this message translates to:
  /// **'Del sistema'**
  String get javaSystem;

  /// No description provided for @instanceChangeVersion.
  ///
  /// In es, this message translates to:
  /// **'Cambiar versión'**
  String get instanceChangeVersion;

  /// No description provided for @changeVersionTitle.
  ///
  /// In es, this message translates to:
  /// **'Cambiar versión de «{name}»'**
  String changeVersionTitle(Object name);

  /// No description provided for @changeVersionCurrent.
  ///
  /// In es, this message translates to:
  /// **'Ahora: {version}'**
  String changeVersionCurrent(Object version);

  /// No description provided for @changeVersionApply.
  ///
  /// In es, this message translates to:
  /// **'Cambiar versión'**
  String get changeVersionApply;

  /// No description provided for @changeVersionWarning.
  ///
  /// In es, this message translates to:
  /// **'Bajar de versión puede romper los mundos, y los mods tienen que ser de la nueva versión.'**
  String get changeVersionWarning;

  /// No description provided for @changeVersionBackup.
  ///
  /// In es, this message translates to:
  /// **'Copia antes'**
  String get changeVersionBackup;

  /// No description provided for @changeVersionBackupLabel.
  ///
  /// In es, this message translates to:
  /// **'Antes de cambiar desde {version}'**
  String changeVersionBackupLabel(Object version);

  /// No description provided for @changeVersionDone.
  ///
  /// In es, this message translates to:
  /// **'Versión cambiada a {version}'**
  String changeVersionDone(Object version);

  /// No description provided for @trashTitle.
  ///
  /// In es, this message translates to:
  /// **'Papelera'**
  String get trashTitle;

  /// No description provided for @trashHint.
  ///
  /// In es, this message translates to:
  /// **'Las instancias eliminadas se guardan aquí {days} días, con sus copias de seguridad.'**
  String trashHint(int days);

  /// No description provided for @trashEmptyStateTitle.
  ///
  /// In es, this message translates to:
  /// **'La papelera está vacía'**
  String get trashEmptyStateTitle;

  /// No description provided for @trashEmptyStateBody.
  ///
  /// In es, this message translates to:
  /// **'Lo que elimines se queda aquí {days} días por si cambias de idea.'**
  String trashEmptyStateBody(int days);

  /// No description provided for @trashRestore.
  ///
  /// In es, this message translates to:
  /// **'Recuperar'**
  String get trashRestore;

  /// No description provided for @trashRestored.
  ///
  /// In es, this message translates to:
  /// **'«{name}» se recuperó'**
  String trashRestored(Object name);

  /// No description provided for @trashPurge.
  ///
  /// In es, this message translates to:
  /// **'Borrar para siempre'**
  String get trashPurge;

  /// No description provided for @trashPurgeTitle.
  ///
  /// In es, this message translates to:
  /// **'¿Borrar «{name}» para siempre?'**
  String trashPurgeTitle(Object name);

  /// No description provided for @trashEmptyTitle.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =1{¿Borrar 1 instancia para siempre?} other{¿Borrar {count} instancias para siempre?}}'**
  String trashEmptyTitle(int count);

  /// No description provided for @trashPurgeBody.
  ///
  /// In es, this message translates to:
  /// **'Se borran sus mundos, mods y copias de seguridad. No se puede deshacer.'**
  String get trashPurgeBody;

  /// No description provided for @trashEmptyAll.
  ///
  /// In es, this message translates to:
  /// **'Vaciar papelera'**
  String get trashEmptyAll;

  /// No description provided for @trashDeletedAgo.
  ///
  /// In es, this message translates to:
  /// **'eliminada {when}'**
  String trashDeletedAgo(Object when);

  /// No description provided for @trashDaysLeft.
  ///
  /// In es, this message translates to:
  /// **'{days, plural, =0{se borra hoy} =1{queda 1 día} other{quedan {days} días}}'**
  String trashDaysLeft(int days);

  /// No description provided for @doctorDiagnose.
  ///
  /// In es, this message translates to:
  /// **'Diagnosticar'**
  String get doctorDiagnose;

  /// No description provided for @doctorTitle.
  ///
  /// In es, this message translates to:
  /// **'Doctor · {name}'**
  String doctorTitle(Object name);

  /// No description provided for @doctorAnalyzing.
  ///
  /// In es, this message translates to:
  /// **'Revisando el crash report, los registros y los mods…'**
  String get doctorAnalyzing;

  /// No description provided for @doctorHealthyTitle.
  ///
  /// In es, this message translates to:
  /// **'No vemos ningún problema'**
  String get doctorHealthyTitle;

  /// No description provided for @doctorHealthyBody.
  ///
  /// In es, this message translates to:
  /// **'La última partida no dejó errores conocidos. Si el juego sigue fallando, revisa los registros.'**
  String get doctorHealthyBody;

  /// No description provided for @doctorCritical.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =1{1 grave} other{{count} graves}}'**
  String doctorCritical(int count);

  /// No description provided for @doctorWarnings.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =1{1 aviso} other{{count} avisos}}'**
  String doctorWarnings(int count);

  /// No description provided for @doctorInfo.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =1{1 nota} other{{count} notas}}'**
  String doctorInfo(int count);

  /// No description provided for @doctorCrashExcerpt.
  ///
  /// In es, this message translates to:
  /// **'Resumen del cierre'**
  String get doctorCrashExcerpt;

  /// No description provided for @doctorOpenReport.
  ///
  /// In es, this message translates to:
  /// **'Abrir crash report'**
  String get doctorOpenReport;

  /// No description provided for @doctorConfidence.
  ///
  /// In es, this message translates to:
  /// **'Confianza {value} %'**
  String doctorConfidence(int value);

  /// No description provided for @doctorCulprit.
  ///
  /// In es, this message translates to:
  /// **'Culpable'**
  String get doctorCulprit;

  /// No description provided for @doctorApplyFix.
  ///
  /// In es, this message translates to:
  /// **'Aplicar arreglo'**
  String get doctorApplyFix;

  /// No description provided for @doctorFixApplied.
  ///
  /// In es, this message translates to:
  /// **'Arreglo aplicado'**
  String get doctorFixApplied;

  /// No description provided for @doctorRerun.
  ///
  /// In es, this message translates to:
  /// **'Volver a analizar'**
  String get doctorRerun;

  /// No description provided for @crashBannerTitle.
  ///
  /// In es, this message translates to:
  /// **'El juego se cerró inesperadamente'**
  String get crashBannerTitle;

  /// No description provided for @trayOpen.
  ///
  /// In es, this message translates to:
  /// **'Abrir {app}'**
  String trayOpen(Object app);

  /// No description provided for @trayPlay.
  ///
  /// In es, this message translates to:
  /// **'Jugar «{name}»'**
  String trayPlay(Object name);

  /// No description provided for @trayQuit.
  ///
  /// In es, this message translates to:
  /// **'Salir'**
  String get trayQuit;

  /// No description provided for @firstRunSkip.
  ///
  /// In es, this message translates to:
  /// **'Saltar asistente'**
  String get firstRunSkip;

  /// No description provided for @firstRunStepWelcome.
  ///
  /// In es, this message translates to:
  /// **'Bienvenida'**
  String get firstRunStepWelcome;

  /// No description provided for @firstRunStepImport.
  ///
  /// In es, this message translates to:
  /// **'Importar'**
  String get firstRunStepImport;

  /// No description provided for @firstRunStepAccount.
  ///
  /// In es, this message translates to:
  /// **'Cuenta'**
  String get firstRunStepAccount;

  /// No description provided for @firstRunStepDone.
  ///
  /// In es, this message translates to:
  /// **'Listo'**
  String get firstRunStepDone;

  /// No description provided for @firstRunRailHint.
  ///
  /// In es, this message translates to:
  /// **'Puedes cambiar todo esto luego en Ajustes.'**
  String get firstRunRailHint;

  /// No description provided for @firstRunWelcomeTitle.
  ///
  /// In es, this message translates to:
  /// **'Te damos la bienvenida a {app}'**
  String firstRunWelcomeTitle(Object app);

  /// No description provided for @firstRunWelcomeBody.
  ///
  /// In es, this message translates to:
  /// **'Un launcher de Minecraft ligero para tus instancias, mods y servidores. En un par de pasos traemos lo que ya tienes y añadimos tu cuenta.'**
  String get firstRunWelcomeBody;

  /// No description provided for @firstRunLanguageSystem.
  ///
  /// In es, this message translates to:
  /// **'El del sistema'**
  String get firstRunLanguageSystem;

  /// No description provided for @firstRunStart.
  ///
  /// In es, this message translates to:
  /// **'Empezar'**
  String get firstRunStart;

  /// No description provided for @firstRunImportTitle.
  ///
  /// In es, this message translates to:
  /// **'Trae lo que ya tienes'**
  String get firstRunImportTitle;

  /// No description provided for @firstRunImportBody.
  ///
  /// In es, this message translates to:
  /// **'Primero ves qué se importaría; nada se copia hasta que lo confirmes. Los originales no se tocan.'**
  String get firstRunImportBody;

  /// No description provided for @firstRunImportNoneTitle.
  ///
  /// In es, this message translates to:
  /// **'No encontramos otros launchers'**
  String get firstRunImportNoneTitle;

  /// No description provided for @firstRunImportNoneBody.
  ///
  /// In es, this message translates to:
  /// **'Puedes crear instancias desde cero o importar más tarde.'**
  String get firstRunImportNoneBody;

  /// No description provided for @firstRunRoxy.
  ///
  /// In es, this message translates to:
  /// **'Roxy Launcher'**
  String get firstRunRoxy;

  /// No description provided for @firstRunMinecraft.
  ///
  /// In es, this message translates to:
  /// **'Launcher oficial (.minecraft)'**
  String get firstRunMinecraft;

  /// No description provided for @firstRunRoxySummary.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =0{Nada nuevo que importar} =1{1 instancia · {size}} other{{count} instancias · {size}}} · reutiliza {shared} de librerías y assets'**
  String firstRunRoxySummary(int count, Object size, Object shared);

  /// No description provided for @firstRunMinecraftSummary.
  ///
  /// In es, this message translates to:
  /// **'Se crea «{name}» ({version}) con {size}:'**
  String firstRunMinecraftSummary(Object name, Object version, Object size);

  /// No description provided for @firstRunBackups.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =1{1 copia} other{{count} copias}}'**
  String firstRunBackups(int count);

  /// No description provided for @firstRunSkipped.
  ///
  /// In es, this message translates to:
  /// **'se omite: {reason}'**
  String firstRunSkipped(Object reason);

  /// No description provided for @firstRunImportAction.
  ///
  /// In es, this message translates to:
  /// **'Importar'**
  String get firstRunImportAction;

  /// No description provided for @firstRunImporting.
  ///
  /// In es, this message translates to:
  /// **'Importando…'**
  String get firstRunImporting;

  /// No description provided for @firstRunImportingFiles.
  ///
  /// In es, this message translates to:
  /// **'{done} de {total} archivos'**
  String firstRunImportingFiles(int done, int total);

  /// No description provided for @firstRunImported.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =0{No se importó ninguna instancia} =1{1 instancia importada} other{{count} instancias importadas}}'**
  String firstRunImported(int count);

  /// No description provided for @firstRunSourceDone.
  ///
  /// In es, this message translates to:
  /// **'Hecho'**
  String get firstRunSourceDone;

  /// No description provided for @firstRunNotNow.
  ///
  /// In es, this message translates to:
  /// **'Ahora no'**
  String get firstRunNotNow;

  /// No description provided for @firstRunAccountTitle.
  ///
  /// In es, this message translates to:
  /// **'Añade tu cuenta'**
  String get firstRunAccountTitle;

  /// No description provided for @firstRunAccountBody.
  ///
  /// In es, this message translates to:
  /// **'Con una cuenta de Microsoft juegas en servidores y usas tu skin. Una offline vale para jugar solo.'**
  String get firstRunAccountBody;

  /// No description provided for @firstRunMicrosoftDesc.
  ///
  /// In es, this message translates to:
  /// **'Inicia sesión con un código en el navegador.'**
  String get firstRunMicrosoftDesc;

  /// No description provided for @firstRunOfflineDesc.
  ///
  /// In es, this message translates to:
  /// **'Solo un nombre. Sin skins ni servidores con verificación.'**
  String get firstRunOfflineDesc;

  /// No description provided for @firstRunAccountsAdded.
  ///
  /// In es, this message translates to:
  /// **'Cuentas añadidas'**
  String get firstRunAccountsAdded;

  /// No description provided for @firstRunDoneTitle.
  ///
  /// In es, this message translates to:
  /// **'Todo listo'**
  String get firstRunDoneTitle;

  /// No description provided for @firstRunDoneBody.
  ///
  /// In es, this message translates to:
  /// **'Ya puedes jugar. Esto es lo que hemos preparado:'**
  String get firstRunDoneBody;

  /// No description provided for @firstRunNoInstances.
  ///
  /// In es, this message translates to:
  /// **'ninguna todavía'**
  String get firstRunNoInstances;

  /// No description provided for @firstRunAccount.
  ///
  /// In es, this message translates to:
  /// **'Cuenta'**
  String get firstRunAccount;

  /// No description provided for @firstRunAppearanceHint.
  ///
  /// In es, this message translates to:
  /// **'fondo y acento en Ajustes'**
  String get firstRunAppearanceHint;

  /// No description provided for @firstRunFinish.
  ///
  /// In es, this message translates to:
  /// **'Ir a {app}'**
  String firstRunFinish(Object app);

  /// No description provided for @sectionSaves.
  ///
  /// In es, this message translates to:
  /// **'Mundos'**
  String get sectionSaves;

  /// No description provided for @sectionMods.
  ///
  /// In es, this message translates to:
  /// **'Mods'**
  String get sectionMods;

  /// No description provided for @sectionConfig.
  ///
  /// In es, this message translates to:
  /// **'Configuración'**
  String get sectionConfig;

  /// No description provided for @sectionResourcepacks.
  ///
  /// In es, this message translates to:
  /// **'Packs de recursos'**
  String get sectionResourcepacks;

  /// No description provided for @sectionShaderpacks.
  ///
  /// In es, this message translates to:
  /// **'Shaders'**
  String get sectionShaderpacks;

  /// No description provided for @sectionScreenshots.
  ///
  /// In es, this message translates to:
  /// **'Capturas'**
  String get sectionScreenshots;

  /// No description provided for @sectionOptions.
  ///
  /// In es, this message translates to:
  /// **'Opciones'**
  String get sectionOptions;

  /// No description provided for @sectionServers.
  ///
  /// In es, this message translates to:
  /// **'Servidores'**
  String get sectionServers;

  /// No description provided for @skinsTitle.
  ///
  /// In es, this message translates to:
  /// **'Skins'**
  String get skinsTitle;

  /// No description provided for @skinsSubtitle.
  ///
  /// In es, this message translates to:
  /// **'Tu skin y tus capas, tal y como te ven los demás'**
  String get skinsSubtitle;

  /// No description provided for @skinsNoAccountTitle.
  ///
  /// In es, this message translates to:
  /// **'Sin cuenta'**
  String get skinsNoAccountTitle;

  /// No description provided for @skinsNoAccountBody.
  ///
  /// In es, this message translates to:
  /// **'Añade una cuenta de Microsoft para ver y cambiar tu skin.'**
  String get skinsNoAccountBody;

  /// No description provided for @skinsAddAccount.
  ///
  /// In es, this message translates to:
  /// **'Añadir cuenta'**
  String get skinsAddAccount;

  /// No description provided for @skinsOfflineTitle.
  ///
  /// In es, this message translates to:
  /// **'Las skins son de las cuentas de Microsoft'**
  String get skinsOfflineTitle;

  /// No description provided for @skinsOfflineBody.
  ///
  /// In es, this message translates to:
  /// **'«{name}» es una cuenta offline: el juego le pone una skin por defecto y no se puede cambiar desde aquí. Añade una cuenta de Microsoft para elegir tu skin y tus capas.'**
  String skinsOfflineBody(Object name);

  /// No description provided for @skinsPngFiles.
  ///
  /// In es, this message translates to:
  /// **'Imágenes PNG'**
  String get skinsPngFiles;

  /// No description provided for @skinsInvalidPng.
  ///
  /// In es, this message translates to:
  /// **'La skin debe ser un PNG de 64×64 o 64×32 píxeles.'**
  String get skinsInvalidPng;

  /// No description provided for @skinsApplied.
  ///
  /// In es, this message translates to:
  /// **'Skin actualizada'**
  String get skinsApplied;

  /// No description provided for @skinsResetTitle.
  ///
  /// In es, this message translates to:
  /// **'¿Restablecer la skin?'**
  String get skinsResetTitle;

  /// No description provided for @skinsResetBody.
  ///
  /// In es, this message translates to:
  /// **'Tu cuenta volverá a la skin por defecto. La actual no se guarda: si la quieres, ten a mano el PNG.'**
  String get skinsResetBody;

  /// No description provided for @skinsReset.
  ///
  /// In es, this message translates to:
  /// **'Restablecer'**
  String get skinsReset;

  /// No description provided for @skinsResetDone.
  ///
  /// In es, this message translates to:
  /// **'Skin restablecida'**
  String get skinsResetDone;

  /// No description provided for @skinsTextureMissing.
  ///
  /// In es, this message translates to:
  /// **'No se pudo cargar la textura'**
  String get skinsTextureMissing;

  /// No description provided for @skinsPreview.
  ///
  /// In es, this message translates to:
  /// **'Vista previa'**
  String get skinsPreview;

  /// No description provided for @skinsSlim.
  ///
  /// In es, this message translates to:
  /// **'Delgada'**
  String get skinsSlim;

  /// No description provided for @skinsClassic.
  ///
  /// In es, this message translates to:
  /// **'Clásica'**
  String get skinsClassic;

  /// No description provided for @skinsSlimDesc.
  ///
  /// In es, this message translates to:
  /// **'Brazos de 3 píxeles'**
  String get skinsSlimDesc;

  /// No description provided for @skinsClassicDesc.
  ///
  /// In es, this message translates to:
  /// **'Brazos de 4 píxeles'**
  String get skinsClassicDesc;

  /// No description provided for @skinsDragHint.
  ///
  /// In es, this message translates to:
  /// **'Arrastra para girar · doble clic para centrar'**
  String get skinsDragHint;

  /// No description provided for @skinsFront.
  ///
  /// In es, this message translates to:
  /// **'Frente'**
  String get skinsFront;

  /// No description provided for @skinsBack.
  ///
  /// In es, this message translates to:
  /// **'Espalda'**
  String get skinsBack;

  /// No description provided for @skinsChange.
  ///
  /// In es, this message translates to:
  /// **'Cambiar skin'**
  String get skinsChange;

  /// No description provided for @skinsFromFile.
  ///
  /// In es, this message translates to:
  /// **'Archivo PNG'**
  String get skinsFromFile;

  /// No description provided for @skinsFromUrl.
  ///
  /// In es, this message translates to:
  /// **'URL'**
  String get skinsFromUrl;

  /// No description provided for @skinsPickPng.
  ///
  /// In es, this message translates to:
  /// **'Elegir un PNG…'**
  String get skinsPickPng;

  /// No description provided for @skinsPngHint.
  ///
  /// In es, this message translates to:
  /// **'64×64 o 64×32 píxeles. Se ve aquí antes de aplicarla.'**
  String get skinsPngHint;

  /// No description provided for @skinsUrl.
  ///
  /// In es, this message translates to:
  /// **'URL de la imagen'**
  String get skinsUrl;

  /// No description provided for @skinsUrlHint.
  ///
  /// In es, this message translates to:
  /// **'Un enlace directo a un PNG público.'**
  String get skinsUrlHint;

  /// No description provided for @skinsModel.
  ///
  /// In es, this message translates to:
  /// **'Modelo'**
  String get skinsModel;

  /// No description provided for @skinsApply.
  ///
  /// In es, this message translates to:
  /// **'Aplicar skin'**
  String get skinsApply;

  /// No description provided for @skinsCapes.
  ///
  /// In es, this message translates to:
  /// **'Capas'**
  String get skinsCapes;

  /// No description provided for @skinsCapesCount.
  ///
  /// In es, this message translates to:
  /// **'{count, plural, =0{ninguna} =1{1 capa} other{{count} capas}}'**
  String skinsCapesCount(int count);

  /// No description provided for @skinsNoCapes.
  ///
  /// In es, this message translates to:
  /// **'Esta cuenta no tiene capas.'**
  String get skinsNoCapes;

  /// No description provided for @skinsNoCape.
  ///
  /// In es, this message translates to:
  /// **'Sin capa'**
  String get skinsNoCape;

  /// No description provided for @skinsCape.
  ///
  /// In es, this message translates to:
  /// **'Capa'**
  String get skinsCape;

  /// No description provided for @crashBannerCode.
  ///
  /// In es, this message translates to:
  /// **'código {code}'**
  String crashBannerCode(int code);

  /// No description provided for @firstRunHighlights.
  ///
  /// In es, this message translates to:
  /// **'Lo que vas a encontrar'**
  String get firstRunHighlights;

  /// No description provided for @firstRunFeatureInstances.
  ///
  /// In es, this message translates to:
  /// **'Instancias separadas'**
  String get firstRunFeatureInstances;

  /// No description provided for @firstRunFeatureInstancesBody.
  ///
  /// In es, this message translates to:
  /// **'Cada una con su versión, loader, mundos y copias.'**
  String get firstRunFeatureInstancesBody;

  /// No description provided for @firstRunFeatureContent.
  ///
  /// In es, this message translates to:
  /// **'Mods y modpacks'**
  String get firstRunFeatureContent;

  /// No description provided for @firstRunFeatureContentBody.
  ///
  /// In es, this message translates to:
  /// **'Busca e instala desde Modrinth y CurseForge.'**
  String get firstRunFeatureContentBody;

  /// No description provided for @firstRunFeatureDoctor.
  ///
  /// In es, this message translates to:
  /// **'Doctor'**
  String get firstRunFeatureDoctor;

  /// No description provided for @firstRunFeatureDoctorBody.
  ///
  /// In es, this message translates to:
  /// **'Si el juego se cierra, te dice por qué y lo arregla.'**
  String get firstRunFeatureDoctorBody;
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
