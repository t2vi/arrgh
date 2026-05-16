import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'app.dart';
import 'core/api/api_client.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await api.init();
  runApp(const ProviderScope(child: ArrghApp()));
}
