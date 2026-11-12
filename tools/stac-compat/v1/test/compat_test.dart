// Parses every JSON fixture produced by the derust test-suite
// (crates/derust/tests/sdui_stac_v1/fixtures) with the real Stac v1.6.0 parsers.
//
// * Widgets / actions go through the parser registry of `package:stac`
//   (`StacParser.getModel` / `StacActionParser.getModel`), which runs the strict
//   `json_serializable` `fromJson` of the model. Nested widgets/actions found in the
//   tree are checked the same way.
// * Other models use `Stac<Name>.fromJson` (generated map in fixture_models.g.dart).
// * Enum fixtures must list exactly the values of the Dart enum.
//
// Why not `Stac.fromJson`? It swallows parse errors and returns `null`/an error widget,
// which would make an invalid fixture look like a pass.
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:stac/stac.dart';

import 'fixture_models.g.dart';

final fixturesDir = Directory('../../../crates/derust/tests/sdui_stac_v1/fixtures');
final negativeDir = Directory('test/negative');
final allowlistFile = File('allowlist.txt');

/// `fixture-id` -> reason, for fixtures Stac cannot parse yet (must stay empty or justified).
Map<String, String> loadAllowlist() {
  if (!allowlistFile.existsSync()) return {};
  final out = <String, String>{};
  for (final line in allowlistFile.readAsLinesSync()) {
    final trimmed = line.trim();
    if (trimmed.isEmpty || trimmed.startsWith('#')) continue;
    final idx = trimmed.indexOf(':');
    out[trimmed.substring(0, idx).trim()] = trimmed.substring(idx + 1).trim();
  }
  return out;
}

List<File> jsonFiles(Directory dir) =>
    dir.listSync(recursive: true).whereType<File>().where((f) => f.path.endsWith('.json')).toList()
      ..sort((a, b) => a.path.compareTo(b.path));

String fixtureId(File f) {
  final rel = f.path.substring(fixturesDir.path.length + 1);
  return rel.substring(0, rel.length - '.json'.length);
}

final widgetTypes = WidgetType.values.map((e) => e.name).toSet();

/// Strictly parses [node] as a widget/action when it is one; recurses into every child.
void checkTree(dynamic node, String where) {
  if (node is List) {
    for (var i = 0; i < node.length; i++) {
      checkTree(node[i], '$where[$i]');
    }
  } else if (node is Map<String, dynamic>) {
    final actionType = node['actionType'];
    final type = node['type'];
    if (actionType == 'showSnackBar' && !node.containsKey('content')) {
      // `StacSnackBarAction` (the button of a SnackBar) also writes actionType "showSnackBar"
      // but is parsed by `StacSnackBar`'s parser through its own `fromJson`, not the registry.
      StacSnackBarAction.fromJson(node);
    } else if (actionType is String) {
      final parser = StacRegistry.instance.getActionParser(actionType);
      if (parser == null) {
        fail('$where: actionType "$actionType" is not registered in Stac');
      }
      parser.getModel(node);
    } else if (type is String) {
      final parser = StacRegistry.instance.getParser(type);
      if (parser != null) {
        parser.getModel(node);
      } else if (widgetTypes.contains(type)) {
        fail('$where: widget type "$type" is not registered in Stac');
      }
    }
    node.forEach((key, value) => checkTree(value, '$where.$key'));
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final allowlist = loadAllowlist();

  setUpAll(() async {
    await Stac.initialize(showErrorWidgets: false);
  });

  group('fixtures are accepted by Stac', () {
    for (final file in jsonFiles(fixturesDir)) {
      final id = fixtureId(file);
      test(id, () {
        if (allowlist.containsKey(id)) {
          markTestSkipped('allowlisted: ${allowlist[id]}');
          return;
        }
        final json = jsonDecode(file.readAsStringSync());
        if (enumValues.containsKey(id)) {
          expect(json, enumValues[id], reason: 'fixture $id must list every Dart enum value');
        } else if (modelParsers.containsKey(id)) {
          modelParsers[id]!(json as Map<String, dynamic>);
          checkTree(json, id);
        } else if (json is Map<String, dynamic>) {
          final root = json['actionType'] ?? json['type'];
          expect(root, isA<String>(), reason: '$id: missing type/actionType');
          checkTree(json, id);
          final registered = json['actionType'] != null
              ? StacRegistry.instance.getActionParser(root as String)
              : StacRegistry.instance.getParser(root as String);
          expect(registered, isNotNull, reason: '$id: "$root" is not registered in Stac');
        } else {
          fail('$id: unexpected fixture shape');
        }
      });
    }
  });

  group('the harness detects invalid fixtures', () {
    for (final file in jsonFiles(negativeDir)) {
      test(file.uri.pathSegments.last, () {
        final json = jsonDecode(file.readAsStringSync());
        expect(() => checkTree(json, 'negative'), throwsA(anything));
      });
    }
  });
}
