// Spike (T9a): the fork is wired in and its binding/expression machinery accepts the minimal
// reactive fixture `{"type":"text","data":{"$expr":"1 + 2"}}`.
//
// Finding for T9b: the strict model parser (`StacParser.getModel` / `fromJson`) does NOT accept
// a raw binding (`data` is cast to `String`): at runtime `StacReactive` resolves the bindings of
// a widget first (`StacBinding.resolve`) and only then parses the resolved JSON. The full
// fixture walk (compat_reactive_test.dart) therefore has to resolve bindings before `getModel`.
import 'package:flutter_test/flutter_test.dart';
import 'package:stac/src/state/stac_binding.dart' as fork;
import 'package:stac/stac.dart';

class _EmptyContext implements StacEvalContext {
  @override
  Object? lookup(String root) => null;
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const fixture = {
    'type': 'text',
    'data': {r'$expr': '1 + 2'},
  };

  test('the fork recognises the binding in the widget JSON', () {
    expect(fork.StacBinding.isBinding(fixture['data']), isTrue);
    expect(
      fork.StacBinding.hasBindings(
        Map<String, dynamic>.of(fixture),
        isAction: false,
      ),
      isTrue,
    );
  });

  test('the expression of the fixture parses and evaluates', () {
    final expression = StacExpression.parse('1 + 2');
    expect(expression.evaluate(_EmptyContext()), 3);
  });

  test('the text widget parser is registered', () async {
    await Stac.initialize(showErrorWidgets: false);
    expect(StacRegistry.instance.getParser('text'), isNotNull);
  });
}
