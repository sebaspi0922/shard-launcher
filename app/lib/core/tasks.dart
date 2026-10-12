// Progreso de las tareas largas del núcleo (descargas, backups, modpacks…),
// por `task_id`. Cada tarea lleva una etapa (`libraries`, `assets`, `java`,
// `content`, `modpack`, `backup`, `restore`, `import`) que la UI usa para saber
// qué está pasando.

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'bridge.dart';
import 'providers.dart';

class TaskProgress {
  const TaskProgress({
    required this.id,
    required this.stage,
    required this.done,
    required this.total,
    required this.bytes,
  });

  final BigInt id;
  final String stage;
  final int done;
  final int total;

  /// `done`/`total` en bytes en vez de archivos.
  final bool bytes;

  double? get fraction => total > 0 ? (done / total).clamp(0, 1) : null;
}

/// Tareas en curso (las terminadas desaparecen).
class TasksController extends Notifier<Map<BigInt, TaskProgress>> {
  @override
  Map<BigInt, TaskProgress> build() {
    listenEvents(ref, (e) {
      switch (e) {
        case UiEvent_Progress(:final taskId, :final stage, :final done, :final total, :final bytes):
          state = {
            ...state,
            taskId: TaskProgress(
              id: taskId,
              stage: stage,
              done: done.toInt(),
              total: total.toInt(),
              bytes: bytes,
            ),
          };
        case UiEvent_TaskFinished(:final taskId):
          if (state.containsKey(taskId)) {
            state = {...state}..remove(taskId);
          }
        default:
          break;
      }
    });
    return const {};
  }

  /// Solo para tests y goldens.
  void debugSet(TaskProgress t) => state = {...state, t.id: t};
}

final tasksProvider =
    NotifierProvider<TasksController, Map<BigInt, TaskProgress>>(
  TasksController.new,
);

/// Tarea más reciente en curso de una etapa (`modpack`, `backup`…).
final stageProgressProvider = Provider.family<TaskProgress?, String>((ref, stage) {
  TaskProgress? best;
  for (final t in ref.watch(tasksProvider).values) {
    if (t.stage == stage && (best == null || t.id > best.id)) best = t;
  }
  return best;
});
