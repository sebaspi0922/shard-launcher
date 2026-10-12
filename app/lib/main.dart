import 'dart:io';

import 'package:flutter/material.dart';
import 'package:lcore_app/src/rust/api/app.dart';
import 'package:lcore_app/src/rust/api/instances.dart';
import 'package:lcore_app/src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  final root = await start();
  // Prueba de humo del puente: `LCORE_SMOKE=1` lista las instancias y sale.
  if (Platform.environment['LCORE_SMOKE'] == '1') {
    final list = await listInstances();
    stdout.writeln('lcore ${coreVersion()} en $root');
    for (final i in list) {
      stdout.writeln('${i.id} ${i.mc} ${i.loader} ${i.playtimeSeconds}s');
    }
    exit(0);
  }
  runApp(const MaterialApp(home: Scaffold(body: Center(child: Text('lcore')))));
}
