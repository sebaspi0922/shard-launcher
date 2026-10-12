// Secciones que llegarán en próximas rondas.

import 'package:material_ui/material_ui.dart';

import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';
import '../widgets/common.dart';

class ComingSoonPage extends StatelessWidget {
  const ComingSoonPage({
    super.key,
    required this.title,
    required this.icon,
    required this.body,
  });

  final String title;
  final IconData icon;
  final String body;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(Space.x8, Space.x1, Space.x8, Space.x8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Text(title, style: context.text.headlineMedium),
              const SizedBox(width: Space.x3),
              Tag(l.comingSoonTitle),
            ],
          ),
          Expanded(
            child: EmptyState(icon: icon, title: l.comingSoonTitle, body: body),
          ),
        ],
      ),
    );
  }
}
