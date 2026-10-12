// Modelo del jugador a partir de la plantilla de skin (64×64 o 64×32): cajas
// con su UV, girado y proyectado en ortográfica. Cada píxel de la textura es
// un cuadrilátero; se ordenan por profundidad (pintor) y se dibujan con
// `drawVertices`, sin filtrado (escalado por vecino más próximo).

import 'dart:math' as math;
import 'dart:typed_data';
import 'dart:ui';

import '../../core/skins.dart';

/// Caras de una caja.
enum _Face { top, bottom, right, front, left, back }

/// Luz fija por cara (da volumen sin depender del ángulo).
double _shade(_Face f) => switch (f) {
      _Face.top => 1.0,
      _Face.bottom => 0.55,
      _Face.front || _Face.back => 0.9,
      _Face.right || _Face.left => 0.74,
    };

class _V3 {
  const _V3(this.x, this.y, this.z);
  final double x;
  final double y;
  final double z;
}

/// Giro de una pieza alrededor de un pivote (brazos abiertos, capa caída).
class _Pose {
  const _Pose({required this.pivot, this.roll = 0, this.pitch = 0});
  final _V3 pivot;

  /// Alrededor del eje Z (abre los brazos).
  final double roll;

  /// Alrededor del eje X (la capa cae hacia atrás).
  final double pitch;

  _V3 apply(_V3 p) {
    var x = p.x - pivot.x;
    var y = p.y - pivot.y;
    var z = p.z - pivot.z;
    if (roll != 0) {
      final c = math.cos(roll), s = math.sin(roll);
      final nx = x * c - y * s;
      final ny = x * s + y * c;
      x = nx;
      y = ny;
    }
    if (pitch != 0) {
      final c = math.cos(pitch), s = math.sin(pitch);
      final ny = y * c - z * s;
      final nz = y * s + z * c;
      y = ny;
      z = nz;
    }
    return _V3(x + pivot.x, y + pivot.y, z + pivot.z);
  }
}

class _Box {
  const _Box({
    required this.x,
    required this.y,
    required this.z,
    required this.w,
    required this.h,
    required this.d,
    required this.u,
    required this.v,
    this.inflate = 0,
    this.mirror = false,
    this.opaque = true,
    this.swapFrontBack = false,
    this.pose,
    this.texture = _Tex.skin,
  });

  /// Esquina mínima y tamaño (en píxeles del modelo y de la textura).
  final double x;
  final double y;
  final double z;
  final int w;
  final int h;
  final int d;

  /// Origen de la UV en la plantilla.
  final int u;
  final int v;

  /// Las capas exteriores son algo más grandes que la base.
  final double inflate;

  /// Extremidades izquierdas de la plantilla antigua (copia en espejo).
  final bool mirror;

  /// La capa base es opaca aunque la textura traiga alfa.
  final bool opaque;

  /// La capa usa la cara «frontal» de la UV por fuera (hacia atrás).
  final bool swapFrontBack;
  final _Pose? pose;
  final _Tex texture;
}

enum _Tex { skin, cape }

/// Piezas del jugador (y de la capa, si hay).
List<_Box> _boxes({required bool slim, required bool legacy, required bool cape}) {
  final aw = slim ? 3 : 4;
  final rightArm = _Pose(pivot: _V3(-4.5, 22.5, 0), roll: -0.07);
  final leftArm = _Pose(pivot: _V3(4.5, 22.5, 0), roll: 0.07);
  return [
    // Cabeza y sombrero.
    const _Box(x: -4, y: 24, z: -4, w: 8, h: 8, d: 8, u: 0, v: 0),
    const _Box(x: -4, y: 24, z: -4, w: 8, h: 8, d: 8, u: 32, v: 0, inflate: 0.5, opaque: false),
    // Cuerpo.
    const _Box(x: -4, y: 12, z: -2, w: 8, h: 12, d: 4, u: 16, v: 16),
    // Brazos.
    _Box(x: -4.0 - aw, y: 12, z: -2, w: aw, h: 12, d: 4, u: 40, v: 16, pose: rightArm),
    if (legacy)
      _Box(x: 4, y: 12, z: -2, w: aw, h: 12, d: 4, u: 40, v: 16, mirror: true, pose: leftArm)
    else
      _Box(x: 4, y: 12, z: -2, w: aw, h: 12, d: 4, u: 32, v: 48, pose: leftArm),
    // Piernas.
    const _Box(x: -4, y: 0, z: -2, w: 4, h: 12, d: 4, u: 0, v: 16),
    if (legacy)
      const _Box(x: 0, y: 0, z: -2, w: 4, h: 12, d: 4, u: 0, v: 16, mirror: true)
    else
      const _Box(x: 0, y: 0, z: -2, w: 4, h: 12, d: 4, u: 16, v: 48),
    // Capas exteriores (solo en la plantilla de 64×64).
    if (!legacy) ...[
      const _Box(x: -4, y: 12, z: -2, w: 8, h: 12, d: 4, u: 16, v: 32, inflate: 0.25, opaque: false),
      _Box(x: -4.0 - aw, y: 12, z: -2, w: aw, h: 12, d: 4, u: 40, v: 32, inflate: 0.25, opaque: false, pose: rightArm),
      _Box(x: 4, y: 12, z: -2, w: aw, h: 12, d: 4, u: 48, v: 48, inflate: 0.25, opaque: false, pose: leftArm),
      const _Box(x: -4, y: 0, z: -2, w: 4, h: 12, d: 4, u: 0, v: 32, inflate: 0.25, opaque: false),
      const _Box(x: 0, y: 0, z: -2, w: 4, h: 12, d: 4, u: 0, v: 48, inflate: 0.25, opaque: false),
    ],
    if (cape)
      const _Box(
        x: -5,
        y: 8,
        z: -3.2,
        w: 10,
        h: 16,
        d: 1,
        u: 0,
        v: 0,
        swapFrontBack: true,
        texture: _Tex.cape,
        pose: _Pose(pivot: _V3(0, 24, -2.2), pitch: 0.14),
      ),
  ];
}

/// Rectángulo de la UV de cada cara: (u, v, ancho, alto).
(int, int, int, int) _uv(_Box b, _Face f) {
  final u = b.u, v = b.v, w = b.w, h = b.h, d = b.d;
  var face = f;
  if (b.mirror) {
    face = switch (f) {
      _Face.right => _Face.left,
      _Face.left => _Face.right,
      _ => f,
    };
  }
  if (b.swapFrontBack) {
    face = switch (face) {
      _Face.front => _Face.back,
      _Face.back => _Face.front,
      _ => face,
    };
  }
  return switch (face) {
    _Face.top => (u + d, v, w, d),
    _Face.bottom => (u + d + w, v, w, d),
    _Face.right => (u, v + d, d, h),
    _Face.front => (u + d, v + d, w, h),
    _Face.left => (u + d + w, v + d, d, h),
    _Face.back => (u + 2 * d + w, v + d, w, h),
  };
}

/// Esquinas de la cara vista desde fuera: arriba-izquierda, arriba-derecha y
/// abajo-izquierda (el frente mira a +z; la derecha del jugador es −x).
(_V3, _V3, _V3) _corners(_Box b, _Face f) {
  final i = b.inflate;
  final x0 = b.x - i, y0 = b.y - i, z0 = b.z - i;
  final x1 = b.x + b.w + i, y1 = b.y + b.h + i, z1 = b.z + b.d + i;
  return switch (f) {
    _Face.front => (_V3(x0, y1, z1), _V3(x1, y1, z1), _V3(x0, y0, z1)),
    _Face.back => (_V3(x1, y1, z0), _V3(x0, y1, z0), _V3(x1, y0, z0)),
    _Face.right => (_V3(x0, y1, z0), _V3(x0, y1, z1), _V3(x0, y0, z0)),
    _Face.left => (_V3(x1, y1, z1), _V3(x1, y1, z0), _V3(x1, y0, z1)),
    _Face.top => (_V3(x0, y1, z0), _V3(x1, y1, z0), _V3(x0, y1, z1)),
    _Face.bottom => (_V3(x0, y0, z1), _V3(x1, y0, z1), _V3(x0, y0, z0)),
  };
}

/// Cámara: giro horizontal (`yaw`) y vertical (`pitch`), en radianes.
class SkinCamera {
  const SkinCamera({this.yaw = 0, this.pitch = 0});
  final double yaw;
  final double pitch;
}

/// Malla lista para pintar y su caja en pantalla.
class SkinMesh {
  SkinMesh(this.vertices, this.quads);
  final Vertices? vertices;

  /// Cuadriláteros dibujados (para tests).
  final int quads;
}

/// Píxeles de pantalla por píxel del modelo. El modelo mide ~33 de alto y,
/// girado, ~20 de ancho: se deja margen.
double skinScale(Size size) => math.min(size.height / 40, size.width / 24);

/// Construye la malla del jugador dentro de `size`.
SkinMesh buildSkinMesh({
  required SkinTexture skin,
  SkinTexture? cape,
  required bool slim,
  required SkinCamera camera,
  required Size size,
}) {
  final boxes = _boxes(slim: slim, legacy: skin.legacy, cape: cape != null);
  final scale = skinScale(size);
  final cx = size.width / 2;
  final cy = size.height / 2;
  final cyaw = math.cos(camera.yaw), syaw = math.sin(camera.yaw);
  final cp = math.cos(camera.pitch), sp = math.sin(camera.pitch);

  // Proyección: (x de pantalla, y de pantalla, profundidad: mayor = más cerca).
  (double, double, double) project(_V3 p) {
    final y = p.y - 16;
    final x1 = p.x * cyaw + p.z * syaw;
    final z1 = -p.x * syaw + p.z * cyaw;
    final y2 = y * cp - z1 * sp;
    final z2 = y * sp + z1 * cp;
    return (cx + x1 * scale, cy - y2 * scale, z2);
  }

  final quads = <_Quad>[];
  for (final b in boxes) {
    final tex = b.texture == _Tex.cape ? cape! : skin;
    for (final f in _Face.values) {
      var (tl, tr, bl) = _corners(b, f);
      final pose = b.pose;
      if (pose != null) {
        tl = pose.apply(tl);
        tr = pose.apply(tr);
        bl = pose.apply(bl);
      }
      final (ax, ay, az) = project(tl);
      final (bx, by, bz) = project(tr);
      final (qx, qy, qz) = project(bl);
      // Cara de espaldas a la cámara: no se dibuja.
      final cross = (bx - ax) * (qy - ay) - (by - ay) * (qx - ax);
      if (cross <= 0) continue;
      final (fu, fv, fw, fh) = _uv(b, f);
      final ux = (bx - ax) / fw, uy = (by - ay) / fw, uz = (bz - az) / fw;
      final vx = (qx - ax) / fh, vy = (qy - ay) / fh, vz = (qz - az) / fh;
      final shade = _shade(f);
      for (var j = 0; j < fh; j++) {
        for (var i = 0; i < fw; i++) {
          final ti = b.mirror ? fw - 1 - i : i;
          var argb = tex.at(fu + ti, fv + j);
          final alpha = argb >>> 24;
          if (b.opaque) {
            argb |= 0xFF000000;
          } else if (alpha == 0) {
            continue;
          }
          final x = ax + ux * i + vx * j;
          final y = ay + uy * i + vy * j;
          final depth = az + uz * (i + 0.5) + vz * (j + 0.5);
          quads.add(_Quad(x, y, ux, uy, vx, vy, depth, _lit(argb, shade)));
        }
      }
    }
  }
  quads.sort((a, b) => a.depth.compareTo(b.depth));

  final pos = Float32List(quads.length * 12);
  final colors = Int32List(quads.length * 6);
  for (var n = 0; n < quads.length; n++) {
    final q = quads[n];
    // Un poco de solape (0,04 px) para que no se vean costuras.
    const e = 0.04;
    final x0 = q.x - e, y0 = q.y - e;
    final ux = q.ux * (1 + e), uy = q.uy * (1 + e);
    final vx = q.vx * (1 + e), vy = q.vy * (1 + e);
    final p = n * 12;
    // Triángulos (a, b, c) y (b, d, c).
    pos[p] = x0;
    pos[p + 1] = y0;
    pos[p + 2] = x0 + ux;
    pos[p + 3] = y0 + uy;
    pos[p + 4] = x0 + vx;
    pos[p + 5] = y0 + vy;
    pos[p + 6] = x0 + ux;
    pos[p + 7] = y0 + uy;
    pos[p + 8] = x0 + ux + vx;
    pos[p + 9] = y0 + uy + vy;
    pos[p + 10] = x0 + vx;
    pos[p + 11] = y0 + vy;
    final c = n * 6;
    for (var k = 0; k < 6; k++) {
      colors[c + k] = q.color;
    }
  }
  return SkinMesh(
    quads.isEmpty
        ? null
        : Vertices.raw(VertexMode.triangles, pos, colors: colors),
    quads.length,
  );
}

class _Quad {
  _Quad(this.x, this.y, this.ux, this.uy, this.vx, this.vy, this.depth, this.color);
  final double x;
  final double y;
  final double ux;
  final double uy;
  final double vx;
  final double vy;
  final double depth;
  final int color;
}

/// Aplica la luz de la cara a un color ARGB.
int _lit(int argb, double k) {
  final a = argb >>> 24;
  final r = (((argb >> 16) & 0xFF) * k).round();
  final g = (((argb >> 8) & 0xFF) * k).round();
  final b = ((argb & 0xFF) * k).round();
  // `Int32List` guarda con signo: se pasa tal cual el patrón de bits.
  return ((a << 24) | (r << 16) | (g << 8) | b).toSigned(32);
}

/// Frontal de una capa (10×16 en 1,1) en ARGB, para miniaturas.
Iterable<(int, int, int)> capeFront(SkinTexture cape) sync* {
  for (var y = 0; y < 16; y++) {
    for (var x = 0; x < 10; x++) {
      yield (x, y, cape.at(1 + x, 1 + y));
    }
  }
}
