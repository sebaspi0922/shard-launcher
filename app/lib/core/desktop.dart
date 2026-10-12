// Integración con el escritorio: icono en la bandeja (StatusNotifierItem en
// Linux, vía tray_manager/nativeapi) y arranque con la sesión. En tests se
// usan las versiones inertes.

import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:launch_at_startup/launch_at_startup.dart';
import 'package:tray_manager/tray_manager.dart' as tray;

import 'app_info.dart';

/// Argumento con el que arranca la sesión: empieza oculto en la bandeja.
const kBackgroundArg = '--background';

/// Menú de la bandeja (textos ya traducidos).
class TrayMenu {
  const TrayMenu({
    required this.tooltip,
    required this.openLabel,
    required this.quitLabel,
    required this.onOpen,
    required this.onQuit,
    this.playLabel,
    this.onPlay,
  });

  final String tooltip;
  final String openLabel;
  final String quitLabel;
  final VoidCallback onOpen;
  final VoidCallback onQuit;

  /// «Jugar …» con la última instancia (`null` = sin instancias).
  final String? playLabel;
  final VoidCallback? onPlay;
}

abstract class TrayHost {
  /// El sistema admite iconos de bandeja (en Linux: hay bus de sesión).
  bool get supported;

  /// Muestra o actualiza el icono. `iconPng` es la imagen del icono.
  void show(TrayMenu menu, Uint8List iconPng);

  void hide();
}

class NoopTrayHost implements TrayHost {
  TrayMenu? shown;

  @override
  bool get supported => true;

  @override
  void show(TrayMenu menu, Uint8List iconPng) => shown = menu;

  @override
  void hide() => shown = null;
}

class DesktopTrayHost implements TrayHost {
  // Hay que conservar las referencias: si se recogen, el icono desaparece.
  tray.TrayIcon? _icon;
  tray.Menu? _menu;
  final List<tray.MenuItem> _items = [];
  Uint8List? _png;
  VoidCallback? _onActivate;

  @override
  bool get supported {
    try {
      return tray.TrayManager.instance.isSupported();
    } catch (_) {
      return false;
    }
  }

  tray.MenuItem _item(String label, VoidCallback? onClick) {
    final item = tray.MenuItem.createWithLabelAndType(
      label,
      tray.MenuItemType.normal,
    )!;
    if (onClick == null) {
      item.isEnabled = false;
    } else {
      item.addListener((e) {
        if (e is tray.MenuItemClickedEvent) onClick();
      });
    }
    _items.add(item);
    return item;
  }

  @override
  void show(TrayMenu menu, Uint8List iconPng) {
    try {
      var icon = _icon;
      if (icon == null) {
        icon = _icon = tray.TrayIcon.create();
        if (icon == null) return;
        // Clic derecho: menú (en Linux se exporta por dbusmenu solo con un
        // disparador). Clic principal: abrir la ventana.
        icon.setContextMenuTrigger(tray.ContextMenuTrigger.rightClicked);
        icon.addListener((e) {
          if (e is tray.TrayIconClickedEvent ||
              e is tray.TrayIconDoubleClickedEvent) {
            _onActivate?.call();
          }
        });
      }
      _onActivate = menu.onOpen;
      if (!identical(_png, iconPng)) {
        final img = tray.Image.fromBase64(
          'data:image/png;base64,${base64Encode(iconPng)}',
        );
        if (img != null) icon.icon = img;
        _png = iconPng;
      }
      icon.setTooltip(menu.tooltip);
      final previous = _menu;
      final previousItems = [..._items];
      _items.clear();
      final m = tray.Menu.create()!;
      m.addItem(_item(menu.openLabel, menu.onOpen));
      if (menu.playLabel != null) m.addItem(_item(menu.playLabel!, menu.onPlay));
      m.addSeparator();
      m.addItem(_item(menu.quitLabel, menu.onQuit));
      icon.setContextMenu(m);
      icon.setVisible(true);
      _menu = m;
      previous?.dispose();
      for (final i in previousItems) {
        i.dispose();
      }
    } catch (e) {
      debugPrint('bandeja: $e');
    }
  }

  @override
  void hide() {
    _icon?.setVisible(false);
    _icon?.dispose();
    _icon = null;
    _png = null;
  }
}

final trayHostProvider = Provider<TrayHost>((ref) => NoopTrayHost());

/// Arrancar con la sesión del sistema.
abstract class Autostart {
  bool get supported;
  Future<bool> isEnabled();
  Future<void> setEnabled(bool enabled);
}

class NoopAutostart implements Autostart {
  bool enabled = false;

  @override
  bool get supported => true;

  @override
  Future<bool> isEnabled() async => enabled;

  @override
  Future<void> setEnabled(bool v) async => enabled = v;
}

class DesktopAutostart implements Autostart {
  DesktopAutostart() {
    // En una AppImage, el ejecutable real cambia en cada montaje; en Flatpak,
    // la ruta interna no existe fuera del sandbox.
    final env = Platform.environment;
    final flatpak = env['FLATPAK_ID'];
    final path = flatpak != null
        ? 'flatpak run $flatpak'
        : env['APPIMAGE'] ?? Platform.resolvedExecutable;
    launchAtStartup.setup(
      appName: kAppName,
      appPath: path,
      packageName: 'dev.shard.Shard',
      args: const [kBackgroundArg],
    );
  }

  @override
  bool get supported => Platform.isLinux || Platform.isWindows || Platform.isMacOS;

  @override
  Future<bool> isEnabled() => launchAtStartup.isEnabled();

  @override
  Future<void> setEnabled(bool enabled) async {
    if (enabled) {
      await launchAtStartup.enable();
    } else {
      await launchAtStartup.disable();
    }
  }
}

final autostartProvider = Provider<Autostart>((ref) => NoopAutostart());

/// Estado del interruptor de autoarranque.
class AutostartController extends AsyncNotifier<bool> {
  @override
  Future<bool> build() => ref.watch(autostartProvider).isEnabled();

  Future<void> set(bool v) async {
    state = AsyncData(v);
    await ref.read(autostartProvider).setEnabled(v);
    state = AsyncData(await ref.read(autostartProvider).isEnabled());
  }
}

final autostartEnabledProvider =
    AsyncNotifierProvider<AutostartController, bool>(AutostartController.new);
