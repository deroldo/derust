// Walks every fixture of the derust test-suite with the REAL parsers of the derust-stac fork:
//
// * `fixtures_reactive/**` (reactive vocabulary: envelope, bindings, state actions, templates);
// * `fixtures/**` (the official v1.6.0 fixtures: the fork must keep accepting all of them).
//
// The strict model parsers (`StacParser.getModel`) do not accept a raw binding (`data` is cast
// to `String`, ...). The runtime therefore prepares the JSON of each widget/action before
// parsing it; this test repeats that pipeline with the same functions of the fork:
//
//   `StacEnvelope.rewriteBind` -> resolve bindings -> `StacEnvelope.apply` ->
//   `StacEnvelope.coerceResolved` -> `getModel`
//
// Bindings are resolved against an example scope (`sampleScope`). `StacScope` cannot be built
// outside a widget tree, so the resolution is a small re-implementation (`fork.StacBinding.isBinding`
// and `StacExpression` are the fork's). A binding that evaluates to `null` against the example
// scope stands for "a value of the type the property wants": the harness tries a few typed
// placeholders (text, number, bool, list, map) and requires that at least one makes the strict
// parser accept the node. This checks the JSON shape (keys, nesting), not the value types.
//
// Parse errors are never swallowed (`Stac.fromJson` would hide them behind an error widget).
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:stac/src/state/stac_binding.dart' as fork;
import 'package:stac/stac.dart';

import '../../v1/test/fixture_models.g.dart';

final reactiveDir = Directory(
  '../../../crates/derust/tests/sdui_stac_v1/fixtures_reactive',
);
final officialDir = Directory(
  '../../../crates/derust/tests/sdui_stac_v1/fixtures',
);
final negativeDir = Directory('test/negative');
final allowlistFile = File('allowlist.txt');

/// `fixture-id` -> reason, for fixtures the fork itself cannot parse (keep empty or justified).
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
    dir
        .listSync(recursive: true)
        .whereType<File>()
        .where((f) => f.path.endsWith('.json'))
        .toList()
      ..sort((a, b) => a.path.compareTo(b.path));

String idOf(File f, Directory base) {
  final rel = f.path.substring(base.path.length + 1);
  return rel.substring(0, rel.length - '.json'.length);
}

final widgetTypes = WidgetType.values.map((e) => e.name).toSet();

/// Example values for the names the fixtures read.
final Map<String, Object?> sampleScope = {
  'name': 'Ana',
  'title': 'Title',
  'greeting': 'Hello',
  'qty': 2,
  'price': 10.5,
  'total': 21,
  'step': 1,
  'limit': 3,
  'n': 1,
  'id': 1,
  'show': true,
  'ready': true,
  'saved': false,
  'failed': false,
  'editable': true,
  'enabled': true,
  'blur': 2.0,
  'selected': [1],
  'draft': 'draft',
  'todos': [
    {'id': 1, 'name': 'Buy milk', 'done': false},
  ],
  'items': [
    {'id': 1, 'name': 'One'},
    {'id': 2, 'name': 'Two'},
  ],
  'user': {
    'name': 'Ana',
    'address': {'city': 'Rio'},
  },
  'product': 'Coffee',
  'global': {'token': 'abc', 'userName': 'Ana'},
  'params': {'name': 'Coffee', 'id': 1},
  'result': {'qty': 1},
  'response': {
    'body': {
      'items': [
        {'id': 1},
      ],
    },
  },
  'event': {'value': 1},
  'item': {'id': 1, 'name': 'One'},
  'index': 0,
  'i': 0,
};

class _SampleContext implements StacEvalContext {
  @override
  Object? lookup(String root) => sampleScope[root];
}

Object? evaluateBinding(Map binding) {
  final source = (binding[r'$state'] ?? binding[r'$expr']) as String;
  try {
    return StacExpression.parse(source).evaluate(_SampleContext());
  } on StacExpressionException {
    return null;
  }
}

bool isChild(Map map) {
  if (map['actionType'] is String) return true;
  final type = map['type'];
  return type is String && StacRegistry.instance.getParser(type) != null;
}

/// Placeholders tried for a binding that evaluated to null.
final List<Object?> placeholders = [
  'x',
  1,
  1.5,
  true,
  <Object?>[],
  <String, Object?>{},
  null,
];

/// Resolves the bindings of [value]; maps that are widgets/actions are prepared on their own.
Object? substitute(
  Object? value,
  List<Object> path,
  List<List<Object>> nulls,
  String where,
) {
  if (value is Map) {
    if (fork.StacBinding.isBinding(value)) {
      final result = evaluateBinding(value);
      if (result == null) nulls.add(List.of(path));
      return result;
    }
    if (isChild(value)) {
      final child = prepareNode(Map<String, dynamic>.from(value), where);
      // places of the child that evaluated to null are also placeholder candidates of the parent
      for (final p in nullsFor[child] ?? const <List<Object>>[]) {
        nulls.add([...path, ...p]);
      }
      return child;
    }
    return <String, dynamic>{
      for (final e in value.entries)
        e.key as String: substitute(
          e.value,
          [...path, e.key as Object],
          nulls,
          '$where.${e.key}',
        ),
    };
  }
  if (value is List) {
    return <dynamic>[
      for (var i = 0; i < value.length; i++)
        substitute(value[i], [...path, i], nulls, '$where[$i]'),
    ];
  }
  return value;
}

void setAt(Object? root, List<Object> path, Object? value) {
  Object? cur = root;
  for (var i = 0; i < path.length - 1; i++) {
    cur = cur is Map ? cur[path[i]] : (cur as List)[path[i] as int];
  }
  if (cur is Map) {
    cur[path.last] = value;
  } else {
    (cur as List)[path.last as int] = value;
  }
}

/// Runtime pipeline for one widget/action; returns the JSON the parser received.
Map<String, dynamic> prepareNode(Map<String, dynamic> node, String where) {
  final actionType = node['actionType'];
  final type = node['type'];
  if (actionType is String) {
    final nulls = <List<Object>>[];
    final own = <String, dynamic>{
      for (final e in node.entries)
        e.key: substitute(e.value, [e.key], nulls, '$where.${e.key}'),
    };
    nullsFor[own] = nulls;
    return own;
  }
  var json = StacEnvelope.rewriteBind(node, type as String);
  final nulls = <List<Object>>[];
  var own = <String, dynamic>{
    for (final e in json.entries)
      e.key: substitute(e.value, [e.key], nulls, '$where.${e.key}'),
  };
  // the runtime parses `visible: false` as "not rendered"; the harness parses it anyway
  final applied =
      StacEnvelope.apply(own, type) ??
      (Map<String, dynamic>.of(own)..remove('visible'));
  own = StacEnvelope.coerceResolved(applied, type);
  nullsFor[own] = nulls;
  return own;
}

final Expando<List<List<Object>>> nullsFor = Expando();

/// Strictly parses an already prepared widget [json]; on failure retries the places that
/// evaluated to null with typed placeholders.
void parseWithPlaceholders(Map<String, dynamic> json, String where) {
  final parser = StacRegistry.instance.getParser(json['type'] as String);
  if (parser == null) {
    fail('$where: widget type "${json['type']}" is not registered in the fork');
  }
  final nulls = nullsFor[json] ?? const <List<Object>>[];
  final Object firstError;
  final StackTrace firstStack;
  try {
    parser.getModel(json);
    return;
  } catch (e, s) {
    firstError = e;
    firstStack = s;
  }
  if (nulls.isEmpty || nulls.length > 3) {
    Error.throwWithStackTrace(firstError, firstStack);
  }
  // product of placeholders over the null places (at most 3 places)
  List<List<Object?>> combos = [[]];
  for (var i = 0; i < nulls.length; i++) {
    combos = [
      for (final c in combos)
        for (final p in placeholders) [...c, p],
    ];
  }
  for (final combo in combos) {
    final copy = jsonDecode(jsonEncode(json)) as Map<String, dynamic>;
    for (var i = 0; i < nulls.length; i++) {
      setAt(copy, nulls[i], combo[i]);
    }
    try {
      parser.getModel(copy);
      return;
    } catch (_) {}
  }
  Error.throwWithStackTrace(firstError, firstStack);
}

/// Strictly parses [node] (and everything below it) the way the runtime would.
void checkTree(dynamic node, String where) {
  if (node is List) {
    for (var i = 0; i < node.length; i++) {
      checkTree(node[i], '$where[$i]');
    }
    return;
  }
  if (node is! Map<String, dynamic>) return;
  final actionType = node['actionType'];
  final type = node['type'];
  if (actionType == 'showSnackBar' && !node.containsKey('content')) {
    // the button of a SnackBar is parsed by `StacSnackBarAction.fromJson`, not by the registry
    StacSnackBarAction.fromJson(node);
  } else if (actionType is String) {
    final parser = StacRegistry.instance.getActionParser(actionType);
    if (parser == null) {
      fail('$where: actionType "$actionType" is not registered in the fork');
    }
    parser.getModel(prepareNode(Map<String, dynamic>.from(node), where));
  } else if (type is String &&
      (StacRegistry.instance.getParser(type) != null ||
          widgetTypes.contains(type))) {
    parseWithPlaceholders(
      prepareNode(Map<String, dynamic>.from(node), where),
      where,
    );
  }
  // inner widgets/actions were already parsed as part of this node; walk the data around them
  // to find the ones the parent does not model (custom data, overlays)
  node.forEach((key, value) {
    if (value is Map<String, dynamic> &&
        (value.containsKey('type') || value.containsKey('actionType'))) {
      checkTree(value, '$where.$key');
    } else if (value is List || value is Map<String, dynamic>) {
      checkTree(value, '$where.$key');
    }
  });
}

void checkFixture(String id, dynamic json) {
  if (enumValues.containsKey(id)) {
    expect(
      json,
      enumValues[id],
      reason: 'fixture $id must list every Dart enum value',
    );
  } else if (modelParsers.containsKey(id)) {
    modelParsers[id]!(json as Map<String, dynamic>);
    checkTree(json, id);
  } else if (json is Map<String, dynamic>) {
    final root = json['actionType'] ?? json['type'];
    expect(root, isA<String>(), reason: '$id: missing type/actionType');
    checkTree(json, id);
  } else {
    fail('$id: unexpected fixture shape');
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final allowlist = loadAllowlist();

  setUpAll(() async {
    await Stac.initialize(showErrorWidgets: false);
  });

  for (final (label, dir) in [
    ('reactive', reactiveDir),
    ('official', officialDir),
  ]) {
    group('$label fixtures are accepted by the fork', () {
      for (final file in jsonFiles(dir)) {
        final id = idOf(file, dir);
        if (label == 'reactive' && id.startsWith('expressions/'))
          continue; // expression_dump_test
        test(id, () {
          if (allowlist.containsKey(id)) {
            markTestSkipped('allowlisted: ${allowlist[id]}');
            return;
          }
          checkFixture(id, jsonDecode(file.readAsStringSync()));
        });
      }
    });
  }

  group('bind', () {
    test(
      'a slider with bind and no value is accepted (premise of Slider::bound)',
      () {
        final json = {
          'type': 'slider',
          'bind': 'volume',
          'min': 0.0,
          'max': 10.0,
        };
        expect(json.containsKey('value'), isFalse);
        checkTree(json, 'slider');
        // `bind` was rewritten into `value: {$state}` + onChanged by the runtime function
        final rewritten = StacEnvelope.rewriteBind(json, 'slider');
        expect(rewritten['value'], {r'$state': 'volume'});
        expect(rewritten.containsKey('bind'), isFalse);
      },
    );
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
