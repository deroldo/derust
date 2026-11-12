// Compares the canonical dump (`Expr::sexp()` in derust) with the dump of the AST built by the
// REAL expression parser of the fork, for every `{source, sexp}` pair of
// crates/derust/tests/sdui_stac_v1/fixtures_reactive/expressions/*.json.
//
// The AST is public inside `package:stac/src/state/expression/parser.dart` (`parseExpression`,
// `StacNode` and subclasses); importing from `src/` is accepted in tests. `StacExpression` itself
// keeps its root private, so it is not used here (it is only used to check that the public
// entry point also accepts every source).
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:stac/src/state/expression/lexer.dart';
import 'package:stac/src/state/expression/parser.dart';
import 'package:stac/stac.dart';

final expressionsDir = Directory(
  '../../../crates/derust/tests/sdui_stac_v1/fixtures_reactive/expressions',
);

/// Number as `Expr::sexp()` prints it: integral values without `.0`.
String dumpNumber(num value) {
  if (value is int) return value.toString();
  if (value.isFinite && value == value.truncateToDouble()) {
    return value.toInt().toString();
  }
  return value.toString();
}

String dumpText(String text) {
  final out = StringBuffer('"');
  for (final rune in text.runes) {
    final c = String.fromCharCode(rune);
    switch (c) {
      case r'\':
        out.write(r'\\');
      case '"':
        out.write(r'\"');
      case '\n':
        out.write(r'\n');
      case '\t':
        out.write(r'\t');
      case '\r':
        out.write(r'\r');
      default:
        out.write(c);
    }
  }
  out.write('"');
  return out.toString();
}

const _binarySymbols = {
  StacTokenType.plus: '+',
  StacTokenType.minus: '-',
  StacTokenType.star: '*',
  StacTokenType.slash: '/',
  StacTokenType.percent: '%',
  StacTokenType.lt: '<',
  StacTokenType.lte: '<=',
  StacTokenType.gt: '>',
  StacTokenType.gte: '>=',
  StacTokenType.eq: '==',
  StacTokenType.neq: '!=',
  StacTokenType.and: '&&',
  StacTokenType.or: '||',
};

/// Dump of the parsed structure, in the format documented on `Expr::sexp()`.
String dump(StacNode node) {
  switch (node) {
    case StacLiteral(:final value):
      if (value == null) return 'null';
      if (value is bool) return value.toString();
      if (value is num) return dumpNumber(value);
      if (value is String) return dumpText(value);
      throw StateError('unexpected literal ${value.runtimeType}');
    case StacIdentifier(:final name):
      return name;
    case StacFieldAccess(:final target, :final field):
      return '(. ${dump(target)} $field)';
    case StacIndexAccess(:final target, :final index):
      return '(idx ${dump(target)} ${dump(index)})';
    case StacCall(:final name, :final args):
      return '(call $name ${args.map(dump).join(' ')})';
    case StacUnary(:final operator, :final operand):
      final name = switch (operator) {
        StacTokenType.bang => 'not',
        StacTokenType.minus => 'neg',
        _ => throw StateError('unexpected unary $operator'),
      };
      return '($name ${dump(operand)})';
    case StacBinary(:final operator, :final left, :final right):
      final symbol = _binarySymbols[operator];
      if (symbol == null) throw StateError('unexpected binary $operator');
      return '($symbol ${dump(left)} ${dump(right)})';
    case StacTernary(:final condition, :final whenTrue, :final whenFalse):
      return '(? ${dump(condition)} ${dump(whenTrue)} ${dump(whenFalse)})';
  }
}

void main() {
  final files =
      expressionsDir
          .listSync()
          .whereType<File>()
          .where((f) => f.path.endsWith('.json'))
          .toList()
        ..sort((a, b) => a.path.compareTo(b.path));

  test('there are expression fixtures', () {
    expect(files, isNotEmpty, reason: 'no files in ${expressionsDir.path}');
  });

  test(
    'the dump distinguishes different structures (sanity of the dumper)',
    () {
      expect(dump(parseExpression('a + b * c')), '(+ a (* b c))');
      expect(dump(parseExpression('(a + b) * c')), '(* (+ a b) c)');
      expect(dump(parseExpression('a - b - c')), '(- (- a b) c)');
      expect(dump(parseExpression('a - (b - c)')), '(- a (- b c))');
    },
  );

  for (final file in files) {
    final name = file.uri.pathSegments.last.replaceAll('.json', '');
    final entries = (jsonDecode(file.readAsStringSync()) as List)
        .cast<Map<String, dynamic>>();
    group('expressions/$name', () {
      for (final entry in entries) {
        final source = entry['source'] as String;
        final expected = entry['sexp'] as String;
        test(source, () {
          final StacNode ast;
          try {
            ast = parseExpression(source);
          } on StacExpressionException catch (e) {
            fail('the parser of the fork rejects `$source`: $e');
          }
          expect(
            dump(ast),
            expected,
            reason:
                'derust prints `$source` with a different structure than the parser of the '
                'fork reads (precedence/associativity/escape divergence)',
          );
          // The public entry point must accept the same source.
          expect(() => StacExpression.parse(source), returnsNormally);
        });
      }
    });
  }
}
