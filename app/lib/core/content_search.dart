// Búsqueda de contenido (Modrinth/CurseForge) con filtros y paginación.

import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'bridge.dart';
import 'providers.dart';

/// Tipos de contenido que se pueden buscar (valor del núcleo).
const kContentKinds = ['mod', 'resourcepack', 'shader', 'datapack', 'modpack'];

/// Órdenes de búsqueda (valor del núcleo).
const kSortOrders = ['relevance', 'downloads', 'follows', 'newest', 'updated'];

/// Loaders por los que se puede filtrar.
const kFilterLoaders = ['fabric', 'quilt', 'forge', 'neoforge'];

/// ¿El loader de la instancia filtra este tipo de contenido?
bool kindUsesLoader(String kind) => kind == 'mod' || kind == 'modpack';

const _pageSize = 20;

class SearchFilters {
  const SearchFilters({
    this.source = 'modrinth',
    this.kind = 'mod',
    this.text = '',
    this.mc,
    this.loader,
    this.sort = 'relevance',
  });

  final String source;
  final String kind;
  final String text;
  final String? mc;
  final String? loader;
  final String sort;

  SearchFilters copyWith({
    String? source,
    String? kind,
    String? text,
    String? Function()? mc,
    String? Function()? loader,
    String? sort,
  }) =>
      SearchFilters(
        source: source ?? this.source,
        kind: kind ?? this.kind,
        text: text ?? this.text,
        mc: mc != null ? mc() : this.mc,
        loader: loader != null ? loader() : this.loader,
        sort: sort ?? this.sort,
      );

  @override
  bool operator ==(Object other) =>
      other is SearchFilters &&
      other.source == source &&
      other.kind == kind &&
      other.text == text &&
      other.mc == mc &&
      other.loader == loader &&
      other.sort == sort;

  @override
  int get hashCode => Object.hash(source, kind, text, mc, loader, sort);
}

class SearchState {
  const SearchState({
    this.filters = const SearchFilters(),
    this.items = const [],
    this.total = 0,
    this.loading = false,
    this.error,
    this.started = false,
  });

  final SearchFilters filters;
  final List<ProjectView> items;
  final int total;
  final bool loading;
  final Object? error;

  /// Ya se hizo al menos una búsqueda con estos filtros.
  final bool started;

  bool get hasMore => items.length < total;

  SearchState copyWith({
    SearchFilters? filters,
    List<ProjectView>? items,
    int? total,
    bool? loading,
    Object? Function()? error,
    bool? started,
  }) =>
      SearchState(
        filters: filters ?? this.filters,
        items: items ?? this.items,
        total: total ?? this.total,
        loading: loading ?? this.loading,
        error: error != null ? error() : this.error,
        started: started ?? this.started,
      );
}

class ContentSearchController extends Notifier<SearchState> {
  /// Cada búsqueda nueva invalida las respuestas de las anteriores.
  int _generation = 0;
  Timer? _debounce;

  @override
  SearchState build() {
    ref.onDispose(() => _debounce?.cancel());
    return const SearchState();
  }

  /// Cambia los filtros y busca de nuevo (con retardo si es texto).
  void setFilters(SearchFilters f, {bool debounce = false}) {
    if (f == state.filters && state.started) return;
    state = SearchState(filters: f, loading: true);
    _debounce?.cancel();
    if (debounce) {
      _debounce = Timer(const Duration(milliseconds: 350), _reload);
    } else {
      unawaited(_reload());
    }
  }

  /// Primera búsqueda al abrir la página (si no hay ya resultados).
  void ensureLoaded() {
    if (!state.started && !state.loading) unawaited(_reload());
  }

  Future<void> retry() => _reload();

  Future<void> _reload() async {
    final gen = ++_generation;
    state = state.copyWith(loading: true, error: () => null, items: const [], total: 0);
    await _fetch(gen, 0);
  }

  /// Siguiente página (scroll infinito).
  Future<void> loadMore() async {
    if (state.loading || !state.hasMore || state.error != null) return;
    final gen = _generation;
    state = state.copyWith(loading: true);
    await _fetch(gen, state.items.length);
  }

  Future<void> _fetch(int gen, int offset) async {
    final f = state.filters;
    try {
      final page = await ref.read(bridgeProvider).searchContent(
            source: f.source,
            kind: f.kind,
            text: f.text.trim(),
            mc: f.mc,
            loader: kindUsesLoader(f.kind) ? f.loader : null,
            sort: f.sort,
            offset: offset,
            limit: _pageSize,
          );
      if (!ref.mounted || gen != _generation) return;
      final seen = {for (final p in state.items) p.id};
      state = state.copyWith(
        items: [
          ...state.items,
          for (final p in page.items)
            if (!seen.contains(p.id)) p,
        ],
        total: page.total,
        loading: false,
        started: true,
      );
    } catch (e) {
      if (!ref.mounted || gen != _generation) return;
      state = state.copyWith(loading: false, error: () => e, started: true);
    }
  }
}

final contentSearchProvider =
    NotifierProvider<ContentSearchController, SearchState>(
  ContentSearchController.new,
);

/// Versiones de Minecraft (releases) para el filtro.
final releaseVersionsProvider = FutureProvider<List<String>>((ref) async {
  final list = await ref.watch(bridgeProvider).listVersions(snapshots: false);
  return [for (final v in list) v.id];
});

typedef VersionsQuery = ({
  String source,
  String project,
  String? mc,
  String? loader,
});

/// Versiones compatibles de un proyecto.
final projectVersionsProvider = FutureProvider.autoDispose
    .family<List<ContentVersionView>, VersionsQuery>(
  (ref, q) => ref.watch(bridgeProvider).contentVersions(
        q.source,
        q.project,
        mc: q.mc,
        loader: q.loader,
      ),
);

/// Ruta local de una imagen remota (caché en disco del puente).
final cachedImageProvider = FutureProvider.family<String?, String>(
  (ref, url) => ref.watch(bridgeProvider).cachedImage(url),
);
