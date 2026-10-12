// Raíz de la app: tema, idioma y rutas.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import 'core/app_info.dart';
import 'core/providers.dart';
import 'l10n/app_localizations.dart';
import 'ui/home/home_page.dart';
import 'ui/instances/instance_detail_page.dart';
import 'ui/instances/instances_page.dart';
import 'ui/settings/settings_page.dart';
import 'ui/shell/app_shell.dart';
import 'ui/shell/coming_soon_page.dart';
import 'ui/shell/ui_scale.dart';
import 'ui/theme/accent.dart';
import 'ui/theme/app_theme.dart';

/// Ruta inicial (los tests la cambian para abrir una pantalla concreta).
final initialLocationProvider = Provider<String>((ref) => '/');

/// Transición corta entre secciones.
Page<void> _fade(GoRouterState state, Widget child) => CustomTransitionPage(
      key: state.pageKey,
      child: child,
      transitionDuration: const Duration(milliseconds: 180),
      reverseTransitionDuration: const Duration(milliseconds: 120),
      transitionsBuilder: (_, anim, _, child) =>
          FadeTransition(opacity: anim, child: child),
    );

final routerProvider = Provider<GoRouter>((ref) {
  final router = GoRouter(
    initialLocation: ref.read(initialLocationProvider),
    routes: [
      ShellRoute(
        builder: (context, state, child) =>
            AppShell(location: state.uri.path, child: child),
        routes: [
          GoRoute(
            path: '/',
            pageBuilder: (_, s) => _fade(s, const HomePage()),
          ),
          GoRoute(
            path: '/instances',
            pageBuilder: (_, s) => _fade(s, const InstancesPage()),
            routes: [
              GoRoute(
                path: ':id',
                pageBuilder: (_, s) => _fade(
                  s,
                  InstanceDetailPage(
                    id: s.pathParameters['id']!,
                    tab: parseTab(s.uri.queryParameters['tab']),
                  ),
                ),
              ),
            ],
          ),
          GoRoute(
            path: '/content',
            pageBuilder: (context, s) => _fade(
              s,
              Builder(
                builder: (context) {
                  final l = AppLocalizations.of(context);
                  return ComingSoonPage(
                    title: l.navContent,
                    icon: Symbols.extension_rounded,
                    body: l.comingSoonContent,
                  );
                },
              ),
            ),
          ),
          GoRoute(
            path: '/servers',
            pageBuilder: (context, s) => _fade(
              s,
              Builder(
                builder: (context) {
                  final l = AppLocalizations.of(context);
                  return ComingSoonPage(
                    title: l.navServers,
                    icon: Symbols.dns_rounded,
                    body: l.comingSoonServers,
                  );
                },
              ),
            ),
          ),
          GoRoute(
            path: '/skins',
            pageBuilder: (context, s) => _fade(
              s,
              Builder(
                builder: (context) {
                  final l = AppLocalizations.of(context);
                  return ComingSoonPage(
                    title: l.navSkins,
                    icon: Symbols.checkroom_rounded,
                    body: l.comingSoonSkins,
                  );
                },
              ),
            ),
          ),
          GoRoute(
            path: '/settings',
            pageBuilder: (_, s) => _fade(s, const SettingsPage()),
          ),
        ],
      ),
    ],
  );
  ref.onDispose(router.dispose);
  return router;
});

/// Idioma elegido en ajustes (`null` = el del sistema).
final localeProvider = Provider<Locale?>((ref) {
  final lang = ref.watch(settingsProvider).value?.language;
  return switch (lang) {
    'es' || 'en' => Locale(lang!),
    _ => null,
  };
});

class LauncherApp extends ConsumerWidget {
  const LauncherApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final accent = ref.watch(accentProvider);
    return MaterialApp.router(
      title: kAppName,
      debugShowCheckedModeBanner: false,
      theme: buildTheme(accent),
      themeMode: ThemeMode.dark,
      darkTheme: buildTheme(accent),
      locale: ref.watch(localeProvider),
      supportedLocales: AppLocalizations.supportedLocales,
      localizationsDelegates: const [
        AppLocalizations.delegate,
        ...GlobalMaterialLocalizations.delegates,
      ],
      // Idioma del sistema si es es/en; si no, español.
      localeResolutionCallback: (locale, supported) {
        for (final s in supported) {
          if (s.languageCode == locale?.languageCode) return s;
        }
        return const Locale('es');
      },
      routerConfig: ref.watch(routerProvider),
      builder: (context, child) => UiScale(child: child!),
    );
  }
}
