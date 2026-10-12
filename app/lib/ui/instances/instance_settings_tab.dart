// Pestaña «Ajustes» de una instancia: nombre, memoria, JVM y ventana.

import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/format.dart';
import '../../core/providers.dart';
import '../../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/common.dart';
import '../widgets/memory_slider.dart';

/// Argumentos de JVM separados por espacios (sin comillas).
List<String> splitArgs(String s) =>
    s.trim().split(RegExp(r'\s+')).where((a) => a.isNotEmpty).toList();

class InstanceSettingsTab extends ConsumerStatefulWidget {
  const InstanceSettingsTab({super.key, required this.instance});

  final InstanceView instance;

  @override
  ConsumerState<InstanceSettingsTab> createState() =>
      _InstanceSettingsTabState();
}

class _InstanceSettingsTabState extends ConsumerState<InstanceSettingsTab> {
  late final TextEditingController _name;
  late final TextEditingController _jvm;
  late final TextEditingController _width;
  late final TextEditingController _height;
  late bool _customMemory;
  late int _memory;
  late bool _customJvm;
  late bool _fullscreen;

  /// Instancia sobre la que se editan los cambios.
  late InstanceView _base;
  bool _saving = false;
  Object? _error;

  @override
  void initState() {
    super.initState();
    _name = TextEditingController();
    _jvm = TextEditingController();
    _width = TextEditingController();
    _height = TextEditingController();
    _load(widget.instance);
  }

  void _load(InstanceView i) {
    final global = ref.read(settingsProvider).value?.defaultMemoryMb ?? 4096;
    _base = i;
    _name.text = i.name;
    _customMemory = i.memoryMb != null;
    _memory = i.memoryMb ?? global;
    _customJvm = i.jvmArgs != null;
    _jvm.text = (i.jvmArgs ?? const []).join(' ');
    _width.text = i.width?.toString() ?? '';
    _height.text = i.height?.toString() ?? '';
    _fullscreen = i.fullscreen;
    _error = null;
  }

  @override
  void dispose() {
    _name.dispose();
    _jvm.dispose();
    _width.dispose();
    _height.dispose();
    super.dispose();
  }

  int? get _memoryValue => _customMemory ? _memory : null;
  List<String>? get _jvmValue => _customJvm ? splitArgs(_jvm.text) : null;
  int? _int(TextEditingController c) => int.tryParse(c.text.trim());

  bool get _dirty {
    final i = _base;
    return _name.text.trim() != i.name ||
        _memoryValue != i.memoryMb ||
        !_sameList(_jvmValue, i.jvmArgs) ||
        _int(_width) != i.width ||
        _int(_height) != i.height ||
        _fullscreen != i.fullscreen;
  }

  static bool _sameList(List<String>? a, List<String>? b) {
    if (a == null || b == null) return a == b;
    if (a.length != b.length) return false;
    for (var k = 0; k < a.length; k++) {
      if (a[k] != b[k]) return false;
    }
    return true;
  }

  Future<void> _save() async {
    final l = AppLocalizations.of(context);
    if (_name.text.trim().isEmpty) return;
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      final saved = await ref.read(instancesProvider.notifier).updateSettings(
            id: widget.instance.id,
            name: _name.text.trim(),
            memoryMb: _memoryValue,
            jvmArgs: _jvmValue,
            width: _int(_width),
            height: _int(_height),
            fullscreen: _fullscreen,
          );
      if (mounted) {
        setState(() => _load(saved));
        showToast(context, l.saved);
      }
    } catch (e) {
      if (mounted) setState(() => _error = e);
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  @override
  void didUpdateWidget(InstanceSettingsTab old) {
    super.didUpdateWidget(old);
    // Si la instancia cambia fuera (p. ej. tras jugar) y no hay cambios
    // propios pendientes, recargar.
    if (old.instance != widget.instance && !_saving && !_dirty) {
      setState(() => _load(widget.instance));
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final total = ref.watch(systemMemoryProvider);
    final global = ref.watch(settingsProvider).value;
    final globalMemory = global?.defaultMemoryMb ?? 4096;

    final left = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _Section(
          title: l.isGeneral,
          icon: Symbols.badge_rounded,
          child: TextField(
            controller: _name,
            onChanged: (_) => setState(() {}),
            decoration: InputDecoration(labelText: l.isName),
          ),
        ),
        const SizedBox(height: Space.x4),
        _Section(
          title: l.isMemory,
          icon: Symbols.memory_rounded,
          subtitle: l.isMemoryDesc,
          trailing: _GlobalSwitch(
            label: l.isUseGlobal,
            value: !_customMemory,
            onChanged: (v) => setState(() => _customMemory = !v),
          ),
          child: _customMemory
              ? MemorySlider(
                  valueMb: _memory,
                  totalMb: total,
                  onChanged: (v) => setState(() => _memory = v),
                )
              : Text(
                  l.isMemoryGlobal(formatMemory(l, globalMemory)),
                  style: context.text.bodyMedium!.copyWith(color: c.textSecondary),
                ),
        ),
      ],
    );

    final right = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _Section(
          title: l.isJvm,
          icon: Symbols.code_rounded,
          subtitle: l.isJvmDesc,
          trailing: _GlobalSwitch(
            label: l.isUseGlobal,
            value: !_customJvm,
            onChanged: (v) => setState(() {
              _customJvm = !v;
              if (_customJvm && _jvm.text.isEmpty) {
                _jvm.text = (global?.defaultJvmArgs ?? const []).join(' ');
              }
            }),
          ),
          child: TextField(
            controller: _jvm,
            enabled: _customJvm,
            minLines: 2,
            maxLines: 4,
            onChanged: (_) => setState(() {}),
            style: TextStyle(fontFamily: kMonoFamily, fontSize: 12.5, color: c.text),
            decoration: InputDecoration(
              hintText: _customJvm
                  ? '-XX:+UseG1GC'
                  : (global?.defaultJvmArgs ?? const []).join(' '),
            ),
          ),
        ),
        const SizedBox(height: Space.x4),
        _Section(
          title: l.isWindow,
          icon: Symbols.aspect_ratio_rounded,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  Expanded(child: _NumberField(controller: _width, label: l.isWidth, onChanged: () => setState(() {}))),
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: Space.x2),
                    child: Text('×', style: context.text.titleMedium!.copyWith(color: c.textMuted)),
                  ),
                  Expanded(child: _NumberField(controller: _height, label: l.isHeight, onChanged: () => setState(() {}))),
                ],
              ),
              const SizedBox(height: Space.x1),
              Text(l.isResolutionDesc, style: context.text.bodySmall),
              const SizedBox(height: Space.x2),
              SettingRow(
                title: l.isFullscreen,
                description: l.isFullscreenDesc,
                control: Switch(
                  value: _fullscreen,
                  onChanged: (v) => setState(() => _fullscreen = v),
                ),
              ),
            ],
          ),
        ),
      ],
    );

    final dirty = _dirty;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Expanded(
          child: SingleChildScrollView(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(child: left),
                const SizedBox(width: Space.x4),
                Expanded(child: right),
              ],
            ),
          ),
        ),
        if (_error != null) ...[
          const SizedBox(height: Space.x3),
          ErrorNote(_error!),
        ],
        AnimatedSwitcher(
          duration: Motion.medium,
          child: !dirty
              ? const SizedBox(width: double.infinity)
              : Padding(
                  padding: const EdgeInsets.only(top: Space.x3),
                  child: Panel(
                    level: 2,
                    padding: const EdgeInsets.symmetric(
                      horizontal: Space.x4,
                      vertical: Space.x2,
                    ),
                    child: Row(
                      children: [
                        Icon(Symbols.edit_note_rounded, color: c.accent, size: 20),
                        const SizedBox(width: Space.x2),
                        Expanded(child: Text(l.unsavedChanges)),
                        TextButton(
                          onPressed: () => setState(() => _load(_base)),
                          child: Text(l.discard),
                        ),
                        const SizedBox(width: Space.x2),
                        FilledButton(
                          onPressed: _saving ? null : _save,
                          child: Text(l.save),
                        ),
                      ],
                    ),
                  ),
                ),
        ),
      ],
    );
  }
}

class _Section extends StatelessWidget {
  const _Section({
    required this.title,
    required this.icon,
    required this.child,
    this.subtitle,
    this.trailing,
  });

  final String title;
  final IconData icon;
  final String? subtitle;
  final Widget? trailing;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Panel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Icon(icon, size: 18, color: c.accent),
              const SizedBox(width: Space.x2),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(title, style: context.text.titleSmall),
                    if (subtitle != null)
                      Text(subtitle!, style: context.text.bodySmall),
                  ],
                ),
              ),
              ?trailing,
            ],
          ),
          const SizedBox(height: Space.x4),
          child,
        ],
      ),
    );
  }
}

class _GlobalSwitch extends StatelessWidget {
  const _GlobalSwitch({
    required this.label,
    required this.value,
    required this.onChanged,
  });

  final String label;
  final bool value;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) => Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(label, style: context.text.bodySmall),
          const SizedBox(width: Space.x1),
          Transform.scale(
            scale: 0.8,
            child: Switch(value: value, onChanged: onChanged),
          ),
        ],
      );
}

class _NumberField extends StatelessWidget {
  const _NumberField({
    required this.controller,
    required this.label,
    required this.onChanged,
  });

  final TextEditingController controller;
  final String label;
  final VoidCallback onChanged;

  @override
  Widget build(BuildContext context) => TextField(
        controller: controller,
        keyboardType: TextInputType.number,
        inputFormatters: [
          FilteringTextInputFormatter.digitsOnly,
          LengthLimitingTextInputFormatter(5),
        ],
        onChanged: (_) => onChanged(),
        decoration: InputDecoration(labelText: label),
      );
}
