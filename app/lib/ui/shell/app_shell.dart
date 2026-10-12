// Marco de la app: fondo, raíl de navegación, barra de título y cierre.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/app_info.dart';
import '../../core/bridge.dart';
import '../../core/crash_notice.dart';
import '../../core/launch.dart';
import '../../core/logs.dart';
import '../../core/providers.dart';
import '../../core/window.dart';
import '../../l10n/app_localizations.dart';
import '../accounts/account_panel.dart';
import '../doctor/crash_banner.dart';
import '../theme/tokens.dart';
import '../widgets/art.dart';
import '../widgets/avatar.dart';
import 'backdrop.dart';
import 'title_bar.dart';
import 'tray_controller.dart';

class NavDestination {
  const NavDestination(this.path, this.icon, this.label);
  final String path;
  final IconData icon;
  final String Function(AppLocalizations l) label;
}

final kDestinations = <NavDestination>[
  NavDestination('/', Symbols.home_rounded, (l) => l.navHome),
  NavDestination('/instances', Symbols.deployed_code_rounded, (l) => l.navInstances),
  NavDestination('/content', Symbols.extension_rounded, (l) => l.navContent),
  NavDestination('/servers', Symbols.dns_rounded, (l) => l.navServers),
  NavDestination('/skins', Symbols.checkroom_rounded, (l) => l.navSkins),
  NavDestination('/settings', Symbols.settings_rounded, (l) => l.navSettings),
];

class AppShell extends ConsumerStatefulWidget {
  const AppShell({super.key, required this.location, required this.child});

  final String location;
  final Widget child;

  @override
  ConsumerState<AppShell> createState() => _AppShellState();
}

class _AppShellState extends ConsumerState<AppShell> {
  bool _closing = false;

  @override
  void initState() {
    super.initState();
    ref.read(windowControlsProvider).onCloseRequested = _onCloseRequested;
  }

  /// Cierre según `close_behavior`: preguntar, segundo plano o salir.
  Future<void> _onCloseRequested() async {
    if (_closing) return;
    _closing = true;
    try {
      final window = ref.read(windowControlsProvider);
      final settings = ref.read(settingsProvider).value;
      switch (settings?.closeBehavior ?? 'ask') {
        case 'quit':
          await window.quit();
        case 'tray':
          await ref.read(trayControllerProvider.notifier).toBackground();
        default:
          if (!mounted) return;
          final choice = await showDialog<_CloseChoice>(
            context: context,
            builder: (_) => const _CloseDialog(),
          );
          if (choice == null) return;
          if (choice.remember && settings != null) {
            await ref.read(settingsProvider.notifier).save(
                  settings.copyWith(closeBehavior: choice.quit ? 'quit' : 'tray'),
                );
          }
          if (choice.quit) {
            await window.quit();
          } else {
            await ref.read(trayControllerProvider.notifier).toBackground();
          }
      }
    } finally {
      _closing = false;
    }
  }

  @override
  Widget build(BuildContext context) {
    // Mantener vivos desde el arranque los consumidores de eventos.
    ref.watch(logHubProvider);
    ref.watch(launchProvider);
    ref.watch(crashNoticeProvider);
    ref.watch(trayControllerProvider);
    return Scaffold(
      backgroundColor: Colors.transparent,
      body: Stack(
        fit: StackFit.expand,
        children: [
          const AppBackdrop(),
          Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              _NavRail(location: widget.location),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    const TitleBar(),
                    Expanded(child: widget.child),
                  ],
                ),
              ),
            ],
          ),
          const Positioned(
            right: Space.x6,
            bottom: Space.x6,
            child: CrashBanner(),
          ),
        ],
      ),
    );
  }
}

class _NavRail extends ConsumerWidget {
  const _NavRail({required this.location});

  final String location;

  bool _selected(String path) =>
      path == '/' ? location == '/' : location.startsWith(path);

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final account = ref.watch(activeAccountProvider);
    return Container(
      width: 76,
      decoration: BoxDecoration(
        color: c.backgroundDeep.withValues(alpha: 0.55),
        border: Border(right: BorderSide(color: c.border)),
      ),
      child: Column(
        children: [
          // La franja superior también arrastra la ventana.
          WindowDragArea(
            child: SizedBox(
              height: 72,
              child: Center(
                child: Tooltip(
                  message: kAppName,
                  child: const BrandMark(size: 30),
                ),
              ),
            ),
          ),
          const SizedBox(height: Space.x2),
          for (final d in kDestinations.take(5))
            _RailItem(
              icon: d.icon,
              label: d.label(l),
              selected: _selected(d.path),
              onTap: () => context.go(d.path),
            ),
          const Spacer(),
          _RailItem(
            icon: kDestinations.last.icon,
            label: kDestinations.last.label(l),
            selected: _selected(kDestinations.last.path),
            onTap: () => context.go(kDestinations.last.path),
          ),
          const SizedBox(height: Space.x2),
          Container(
            width: 36,
            height: 1,
            color: c.border,
          ),
          const SizedBox(height: Space.x3),
          Tooltip(
            message: account?.name ?? l.accountNone,
            preferBelow: false,
            child: InkWell(
              borderRadius: BorderRadius.circular(Radii.card),
              onTap: () => showAccountPanel(context),
              child: Padding(
                padding: const EdgeInsets.all(6),
                child: Stack(
                  clipBehavior: Clip.none,
                  children: [
                    PlayerAvatar(account: account, size: 40),
                    if (account == null)
                      Positioned(
                        right: -3,
                        bottom: -3,
                        child: Container(
                          width: 16,
                          height: 16,
                          decoration: BoxDecoration(
                            color: c.accent,
                            shape: BoxShape.circle,
                            border: Border.all(color: c.backgroundDeep, width: 2),
                          ),
                          child: Icon(Symbols.add_rounded, size: 11, color: c.onAccent, weight: 700),
                        ),
                      ),
                  ],
                ),
              ),
            ),
          ),
          const SizedBox(height: Space.x4),
        ],
      ),
    );
  }
}

class _RailItem extends StatefulWidget {
  const _RailItem({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  State<_RailItem> createState() => _RailItemState();
}

class _RailItemState extends State<_RailItem> {
  bool _hover = false;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    final sel = widget.selected;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Tooltip(
        message: widget.label,
        preferBelow: false,
        verticalOffset: 0,
        margin: const EdgeInsets.only(left: 64),
        child: MouseRegion(
          onEnter: (_) => setState(() => _hover = true),
          onExit: (_) => setState(() => _hover = false),
          cursor: SystemMouseCursors.click,
          child: GestureDetector(
            onTap: widget.onTap,
            child: SizedBox(
              width: 76,
              height: 48,
              child: Stack(
                alignment: Alignment.center,
                children: [
                  // Indicador lateral de la sección activa.
                  AnimatedPositioned(
                    duration: Motion.medium,
                    curve: Motion.curve,
                    left: 0,
                    top: sel ? 12 : 24,
                    bottom: sel ? 12 : 24,
                    child: Container(
                      width: 3,
                      decoration: BoxDecoration(
                        color: sel ? c.accent : Colors.transparent,
                        borderRadius: const BorderRadius.horizontal(
                          right: Radius.circular(3),
                        ),
                      ),
                    ),
                  ),
                  AnimatedContainer(
                    duration: Motion.fast,
                    width: 48,
                    height: 44,
                    decoration: BoxDecoration(
                      color: sel
                          ? c.accentContainer.withValues(alpha: 0.75)
                          : _hover
                              ? c.surface2
                              : Colors.transparent,
                      borderRadius: BorderRadius.circular(Radii.card),
                    ),
                    child: Icon(
                      widget.icon,
                      size: 24,
                      fill: sel ? 1 : 0,
                      weight: sel ? 500 : 400,
                      color: sel ? c.onAccentContainer : c.textSecondary,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _CloseChoice {
  const _CloseChoice({required this.quit, required this.remember});
  final bool quit;
  final bool remember;
}

class _CloseDialog extends StatefulWidget {
  const _CloseDialog();

  @override
  State<_CloseDialog> createState() => _CloseDialogState();
}

class _CloseDialogState extends State<_CloseDialog> {
  bool _remember = false;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Dialog(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 440),
        child: Padding(
          padding: const EdgeInsets.all(Space.x6),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(l.closeDialogTitle(kAppName), style: context.text.titleLarge),
              const SizedBox(height: Space.x3),
              Text(
                l.closeDialogBody,
                style: context.text.bodyMedium!.copyWith(
                  color: context.colors.textSecondary,
                ),
              ),
              const SizedBox(height: Space.x4),
              InkWell(
                borderRadius: BorderRadius.circular(6),
                onTap: () => setState(() => _remember = !_remember),
                child: Row(
                  children: [
                    Checkbox(
                      value: _remember,
                      onChanged: (v) => setState(() => _remember = v ?? false),
                    ),
                    Text(l.closeDialogRemember),
                  ],
                ),
              ),
              const SizedBox(height: Space.x5),
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  TextButton(
                    onPressed: () => Navigator.pop(context),
                    child: Text(l.cancel),
                  ),
                  const SizedBox(width: Space.x2),
                  OutlinedButton(
                    onPressed: () => Navigator.pop(
                      context,
                      _CloseChoice(quit: false, remember: _remember),
                    ),
                    child: Text(l.closeDialogMinimize),
                  ),
                  const SizedBox(width: Space.x2),
                  FilledButton(
                    onPressed: () => Navigator.pop(
                      context,
                      _CloseChoice(quit: true, remember: _remember),
                    ),
                    child: Text(l.closeDialogQuit),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
