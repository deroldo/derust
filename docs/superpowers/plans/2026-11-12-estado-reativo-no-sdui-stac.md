# Estado reativo no SDUI Stac (derust) — Plano de Implementação

> **Para quem executa:** use `superpowers:subagent-driven-development` ou `superpowers:executing-plans`. Cada tarefa é autocontida. Passos usam checkbox (`- [ ]`).

**Objetivo:** estender o módulo `derust::sduix::stacx::v1` (feature `sdui_stac_1`, já existente, **sem nova feature nem novo módulo**) para descrever, só com tipos Rust, telas com estado compartilhado e reativo conforme o contrato do fork do Stac: blocos `state`/`globalState`/`onInit`, bindings (`$state`) e a linguagem de expressões (`$expr`) tipadas, envelope (`visible`/`enabled`/`bind`/`condition`), actions de estado, `networkRequest` e `navigate` estendidos, templates de lista; mais validação no servidor, inventário/testes de compatibilidade, exemplo e documentação.

**Plano de negócio (fonte das regras):** vault Obsidian `Projects/derust/plans/estado_reativo_no_sdui_stac.md`. Nenhuma regra de negócio nova pode ser decidida aqui. Se uma tarefa esbarrar em conflito entre regra e implementação: parar e perguntar (`AskUserQuestion`). As "Decisões provisórias" no fim deste documento são interpretações de ambiguidades do plano de negócio; confirmar com a pessoa antes ou durante a implementação.

**Plano anterior (já implementado nesta branch):** `docs/superpowers/plans/2026-11-12-sdui-stac.md` (módulo Stac v1.6.0 oficial, gerado por `tools/stac-inventory/gen_rust.py`). Este plano **depende** dele e o estende de forma aditiva.

**Worktree/branch:** `/Users/deroldo/workspace/derust/.worktrees/sdui_stac`, branch `sdui_stac` (exceção autorizada à regra do slug: reaproveita a branch do plano `sdui_stac`). Rodar tudo dentro dela.

## Fatos verificados (base do refinamento)

- **Fonte do contrato:** fork `deroldo/derust-stac` (clone local `/Users/deroldo/workspace/derust-stac`, worktree `.worktrees/estado_compartilhado_reativo`), branch `estado_compartilhado_reativo`, **commit lido no refinamento: `92457f25600e63e8d6754e4361dbb3e81b89dffc`** (branch já em `origin`). Referência: `docs/concepts/state_reference.mdx`; implementação de apoio em `packages/stac/lib/src/state/` (`stac_binding.dart`, `stac_envelope.dart`, `stac_scope.dart`, `stac_item_template.dart`, `expression/{lexer,parser,evaluator,functions}.dart`). Não há tag nem pin: **sem versão fixada**.
- **O fork não é o Stac oficial.** O módulo atual é gerado da tag oficial `v1.6.0` e o harness atual (`tools/stac-compat/v1`) usa a tag oficial. O vocabulário reativo **só** é aceito pelo fork; por isso os fixtures reativos ficam num diretório separado e num harness separado (T9). Os fixtures oficiais existentes **não podem** receber bindings (o Stac oficial rejeitaria).
- **Estado atual do código** (`crates/derust/src/sduix/stacx/`): `common/{dynamic,validate,node,custom}.rs` (`Dyn<T>` = `Literal | Ref(StateRef)` do mecanismo legado `{{chave}}`; `Validate::validate_at(&Path, &mut Vec<ValidationIssue>)`; `CustomWidget/CustomAction`); `v1/{widgets,actions,foundation,theme}/` **gerados** por `tools/stac-inventory/gen_rust.py` (marcados `// @generated`, não editar à mão), `v1/dsl.rs`, `v1/prelude.rs`, `v1/inventory.json`. Cada widget/action é uma struct com campos `Option<T>`, setters encadeados `fn campo(mut self, impl Into<T>) -> Self`, `Serialize` manual (`serialize_map`, discriminador `type`/`actionType`) e `Validate`. `Widget` e `Action` são enums fechados com variante `Custom(CustomNode)`. Ganchos do gerador: `SKIP_CLASSES`, `NESTED_ONLY_ACTIONS`, `FIELD_TYPE_OVERRIDES`, `EXTRA_FIELDS`, `custom_checks`, e arquivos à mão (`foundation/colors/color.rs`, `*_ext.rs`) preservados e ligados no `mod.rs`.
- **`SduiResponse<T>`** (`httpx/response/sdui.rs`): `try_new(status, screen, tags)` roda `screen.validate()`, depois `serde_json::to_string`; erro vira `DerustError::without_body(500, "failed to serialize SDUI screen: ...")`. Bound: `T: Serialize + Validate + Send + Sync`.
- **Colisões de nomes no modelo atual:** campo nativo `visible: Option<bool>` em `Visibility` e `SliverVisibility`; `enabled: Option<bool>` em `BackdropFilter`, `DropdownMenu`, `ListTile`, `Radio`, `TextField`, `TextFormField` (o contrato diz: esses widgets recebem o valor resolvido pelo próprio campo nativo). `NavigateAction.result` hoje é `Option<Map>` (não distingue "ausente" de "nulo"). `NetworkRequest.body/headers/query_parameters` e `NavigateAction.arguments` são `serde_json::Map/Value`. Propriedades de valor sobrescritas por `bind`: `value` (checkBox/switch/slider), `groupValue` (radioGroup), `initialValue` (textField/textFormField), `initialSelection` (dropdownMenu, hoje `Option<serde_json::Value>`).
- **Regras do runtime relevantes (do contrato e do código do fork):**
  - Binding = mapa com **exatamente uma** chave `$state` ou `$expr` cujo valor é texto (`StacBinding.isBinding`). Não existe escape: um dado literal com essa forma vira binding.
  - Binding vale em qualquer propriedade do JSON próprio do widget, em qualquer profundidade, exceto dentro de widgets/actions aninhados (eles resolvem os seus). Em actions, as chaves `values, value, result, params, body, queryParameters, headers, initial` são dado: bindings abaixo delas são resolvidos em qualquer profundidade.
  - Léxico: `IDENT=[A-Za-z_][A-Za-z0-9_]*`; `NUMBER` sem sinal/expoente/ponto inicial, inteiro via `int.tryParse` (inteiro acima de 2^63-1 vira **erro de sintaxe**, e o literal sai como `null`); strings aceitam `'` ou `"`, escapes `\n \t \r` e `\x` -> `x`; `true/false/null` são literais mesmo como nome de chave; nome seguido de `(` é chamada de função. Precedência (do mais solto): `?:` (direita), `||`, `&&`, `== !=`, `< <= > >=`, `+ -`, `* / %`, unário `! -`, postfix `.campo [expr] f() ( )`; binários associativos à esquerda.
  - `screen scope`: `state` aninhado **substitui** (não encadeia com) o externo; escrita em chave não declarada é erro; reservados: `global, params, result, response, event`; `item`/`index` não são reservados.
  - Escopos de variáveis: `response` em `onSuccess/onError/onFinally` (e nas actions que eles rodam), **não** em `onStart`; `event` em `onChanged`/`onSelected` de entradas (e actions encadeadas); `result` em `onResult` (não em `onCancel`); `item`/`index` (ou `as`/`indexAs`) no `itemTemplate`; `params` na tela destino de um `navigate` com `params`.
- **Referência de exemplos:** `state_reference.mdx` tem 6 exemplos JSON completos (globalState+onInit, formulário com `bind`, multiAction, networkRequest com callbacks, navigate com params/result, lista com template). A suíte deve reproduzi-los em Rust (T10). O último usa `{"actionType":"none"}` em `onCancel` (action `none` já existe no fork: `stac_none_action`).
- **Não há `.github/` CI além do `ci.yml` criado no plano anterior** (jobs `rust` e `stac-compat-v1`).

## Decisões de mecânica (o "como")

1. **Onde fica o código novo.** Hand-written, em `crates/derust/src/sduix/stacx/v1/state/` (módulo novo **dentro** de `v1`, sob a mesma feature): `ident.rs`, `path.rs`, `expr.rs`, `binding.rs`, `reactive.rs`, `data.rs`, `envelope.rs`, `decl.rs`, `actions.rs`, `template.rs`, `check.rs`, `mod.rs`. Reexportado em `v1::prelude`. O que precisa tocar os ~250 tipos gerados (envelope, `Reactive`, `condition`) é feito **no gerador** (`gen_rust.py`), nunca editando arquivos `@generated`. O gerador continua lendo a tag oficial `v1.6.0` (baseline oficial inalterado); o vocabulário reativo é uma camada por cima, marcada `source: "fork"` no inventário.
2. **Bindings e expressões tipados.**
   - `Ident`: newtype de `String`; `Validate` exige `IDENT` e rejeita `true`, `false`, `null`.
   - `StatePath`: raiz + segmentos. Raízes: `Root::Key(Ident)` (chave de tela ou variável local), `Root::Global(Ident)` (sempre impresso `global.chave`, **nunca** `global['chave']`), `Root::Params`, `Root::Result`, `Root::Response`, `Root::Event`. Segmentos: `.campo(Ident)` e `[n]` com `usize` literal. `StatePath` **não tem** operadores: `$state` com operação é inexistente em compilação.
   - `Expr` (enum fechado, `Box` nas recursões): `Num(f64)`, `Int(i64)`, `Text(String)`, `Bool`, `Null`, `Ref(StatePath)`, `Field(Box<Expr>, Ident)`, `Index(Box<Expr>, Box<Expr>)`, `Unary(UnOp{Not,Neg}, Box<Expr>)`, `Binary(BinOp, Box<Expr>, Box<Expr>)` com `BinOp` = `Add Sub Mul Div Rem Lt Le Gt Ge Eq Ne And Or`, `Ternary(Box<Expr>,Box<Expr>,Box<Expr>)`, `Group(Box<Expr>)` (parênteses explícitos), e as funções **como variantes**: `Length(Box<Expr>)`, `Trim(Box<Expr>)`, `Contains{list,value}`, `ContainsBy{list,object,field:String}`. Função desconhecida e aridade errada são irrepresentáveis. Açúcar: `std::ops::{Add,Sub,Mul,Div,Rem,Neg,Not}` e métodos `.eq_(..)/.ne_(..)/.lt/.le/.gt/.ge/.and/.or`, `Expr::cond(c,a,b)`, `.field("x")`, `.at(expr)`, `.grouped()`, `Expr::length(..)`, `Expr::trim(..)`, `Expr::contains(..)`, `Expr::contains_by(..)`. Conversões `From<StatePath>`, `From<f64/i64/i32/u32/&str/String/bool>`.
   - **Impressão (`Display`/`to_source`)** com mínimo de parênteses e correção prioritária. Níveis: ternário 1, `||` 2, `&&` 3, `== !=` 4, comparação 5, `+ -` 6, `* / %` 7, unário 8, postfix/primário 9. Regras: filho **esquerdo** de binário recebe parênteses se `nivel(filho) < nivel(op)`; filho **direito** se `nivel(filho) <= nivel(op)` (associatividade à esquerda: `a - (b - c)` mantém parênteses); operando de unário recebe parênteses se nível < 8, e dois unários seguidos saem com espaço (`- -x`, `! !x`); base de `.campo`/`[..]` recebe parênteses se nível < 9 **ou** for literal numérico (`(1).x`); índice entre colchetes e argumentos de função nunca precisam de parênteses; **condição** do ternário recebe parênteses se for ternário; ramos do ternário nunca (associativo à direita). `Group` sempre imprime os parênteses. Operador binário impresso com espaços (`a + b`).
   - **Números:** `Int(i64)` imprime dígitos; `Num(f64)`: negativo vira `Unary(Neg, Num(|x|))` (o literal é sem sinal); `Display` do Rust nunca usa expoente; valor inteiro (sem parte fracionária) imprime sem `.0`; `NaN`/`±inf` e inteiros com módulo >= 2^63 são **erro de `Validate`** (o runtime falharia no `int.tryParse`). `-0.0` normaliza para `0`.
   - **Texto:** sempre entre aspas simples; escapa `\` -> `\\`, `'` -> `\'`, quebra de linha -> `\n`, tab -> `\t`, `\r` -> `\r`; demais caracteres literais. Nunca emite texto não terminado.
   - **Índice dinâmico:** `Validate` rejeita índice cuja raiz (ignorando `Group`) é `Div` ou literal com parte fracionária (o runtime só aceita inteiro).
   - `Binding` = `State(StatePath)` | `Expr(Expr)`; serializa `{"$state":"caminho"}` / `{"$expr":"fonte"}` (um único par).
3. **`Reactive<T>` e propriedades de qualquer tipo.** `enum Reactive<T> { Value(T), Bind(Binding) }`, `Serialize` transparente. O gerador troca o tipo de **todo campo-folha** `Option<T>` por `Option<Reactive<T>>` (folha = qualquer campo que não seja `Widget`/`Action`/`Vec<Widget>`/`Vec<Action>`/`Box<..Action>`; inclui structs auxiliares como `EdgeInsets`, recursivamente, e enums). Listas de não-widgets são ligáveis como um todo (`Reactive<Vec<..>>`); elementos individuais de lista não são ligáveis (limitação documentada). Setters passam a aceitar `impl IntoReactive<T>`. **Coerência:** `trait IntoReactive<T>` implementada por `impl<T: Plain> IntoReactive<T> for T` e por impls concretos para `Binding`, `Expr`, `StatePath`; `Plain` é um trait marcador local (o gerador emite `impl Plain for X` para cada tipo gerado e a mão para primitivos), o que mantém os impls disjuntos para o compilador. Impls extras para compatibilidade com o que os setters aceitavam antes (`&str`->`String`, inteiros e `f32`->`f64`, `Vec`/iteráveis). Isto é a parte de maior risco do plano: **T3 começa com spike e fecha em gate G1**.
   - `Dyn<String>` (legado `{{chave}}`) ganha a variante `Bind(Binding)` e continua nos campos que já o usam; os dois mecanismos coexistem.
4. **Dado com bindings (`Data`).** `enum Data { Null, Bool, Int(i64), Num(f64), Text(String), List(Vec<Data>), Object(BTreeMap<String, Data>), Bind(Binding) }`, `Serialize` transparente, `From<serde_json::Value>` e `From<Binding/Expr/StatePath>`. Substitui `serde_json::Value/Map` em `NetworkRequest.{body,headers,query_parameters}`, `NavigateAction.{arguments,result,params}`, `setState.values`, `appendState.value`, etc. **`Data::Object` com exatamente uma chave `$state`/`$expr` de valor texto é rejeitado por `Validate`** (decisão provisória D2): o contrato não tem escape e emitir viraria binding por engano.
5. **Presença vs. nulo.** `result` de `navigate` é `Option<Data>`: `None` = chave ausente; `Some(Data::Null)` = `"result": null` explícito. Mesma regra para qualquer campo `Option<Data>`. `by`/`min`/`max`: `Option<Reactive<f64>>` (omitido = ausente).
6. **Envelope de widgets.** `struct Envelope { visible: Option<Reactive<bool>>, enabled: Option<Reactive<bool>>, bind: Option<BindKey>, state: Option<StateBlock>, global_state: Option<GlobalStateBlock>, on_init: Option<OnInit> }`. O gerador acrescenta a **todo widget** o campo `envelope: Envelope` e os setters `visible/enabled/bind/state/global_state/on_init`, e serializa o envelope logo depois do discriminador. Para widgets com `visible`/`enabled` nativos, o campo nativo é **removido** da struct (via `SKIP_FIELDS` novo) e o setter de mesmo nome escreve no envelope (mesma chave JSON, tipo `Reactive<bool>`, `bool` continua aceito). `BindKey` = `Screen(Ident)` | `Global(Ident)` (serializa `chave` / `global.chave`); `Validate` rejeita chave vazia. `StateBlock` = mapa chave->`Data` (chave de tela **sem** `persist/secure`, por tipo); `GlobalStateBlock` = mapa chave->`GlobalEntry` (`Value(Data)` | `Declared{initial, persist: Option<bool>, secure: Option<bool>}`); `OnInit` = uma action ou lista (lista = `multiAction` sync pelo runtime). Widgets `Custom`: o app embute `Envelope` na própria struct (serializa achatado via `serialize_into`).
7. **Envelope de actions.** O gerador acrescenta `condition: Option<Reactive<bool>>` (+ setter) a toda action registrada (as 10 atuais; `NESTED_ONLY_ACTIONS` ficam de fora) e as novas actions à mão a incluem também.
8. **Novas actions** (hand-written em `v1/state/actions.rs`, ligadas ao enum `Action` por um novo gancho do gerador `EXTRA_ACTIONS = [(Variante, "tipo_json", caminho_rust)]`, que gera variante, `Serialize`/`Validate`/`type_name` e `From`): `SetState`, `IncrementState`, `DecrementState`, `ToggleState`, `ResetState`, `AppendState`, `RemoveFromState`, `ToggleItem`, `CancelRequest`, `NoneAction` (necessária para reproduzir o exemplo do contrato; ver decisão provisória D4). `RemoveFromState` só tem os construtores `at_index(key, idx)` e `by_value(key, value)` (sem os dois = irrepresentável; com os dois por `.also_value(..)`, onde `index` prevalece no runtime).
9. **`networkRequest` e `navigate` estendidos.** Via `FIELD_TYPE_OVERRIDES` / `EXTRA_FIELDS` do gerador (novos pseudo-tipos Dart `Data`, `Actions`, `StringList` mapeados para tipos Rust à mão): `NetworkRequest` ganha `on_start/on_success/on_error/on_finally: Option<Actions>` (`Actions` = uma action ou lista, lista mantém a ordem), `request_key: Option<String>`, `body_from_state: Option<Vec<BindKey>>`. `NavigateAction` ganha `params: Option<BTreeMap<String, Data>>`, `result: Option<Data>`, `on_result`, `on_cancel: Option<Box<Action>>`. `multiAction` já tem `sync`; `condition` vem do item 7.
10. **Templates de lista.** `ListView`, `GridView`, `PageView`, `SliverList`, `SliverGrid` ganham (via `EXTRA_FIELDS`) `items: Option<Reactive<Vec<Data>>>`, `item_template: Option<Box<Widget>>`, `item_name` (JSON `as`), `index_name` (JSON `indexAs`) e um setter de alto nível `template(ItemTemplate::new(items).item_as("todo").index_as("i"), |vars| widget)`, onde `vars.item()`/`vars.index()` devolvem `StatePath` já com o **nome efetivo** (default `item`/`index`; texto vazio vira default, como no runtime). Isso mantém a referência tipada coerente com `as`/`indexAs`.
11. **Validação em duas camadas.**
    - **Tipada** (`Validate`, já chamada por `SduiResponse::try_new`): `Ident`, números/índices de `Expr`, `Data::Object` com forma de binding, `BindKey` vazia, `GlobalEntry` com `secure: true` e `persist: Some(false)`, `NavigateAction` (`on_result` e `on_cancel` juntos; `result` com estilo diferente de `Pop`/`PushReplacement`/`PushReplacementNamed`; sem `navigation_style` conta como `Push`), `RemoveFromState` (irrepresentável sem alvo), `body_from_state` vazio de chave vazia.
    - **Sobre o JSON serializado** (`v1/state/check.rs`): tudo que depende de contexto (escopo de variáveis, chaves declaradas, tipo de widget). Percorre o `serde_json::Value` da tela, que é exatamente o que o runtime vê (cobre também `CustomNode`). Novo método **fornecido** em `common::Validate`: `fn validate_serialized(&self, json: &Value, report: &mut ValidationReport) {}` (padrão: nada); `Widget`/`Action` de `v1` o sobrescrevem chamando `check::check_screen`. `ValidationReport { errors: Vec<ValidationIssue>, warnings: Vec<ValidationIssue> }`. `SduiResponse::try_new` serializa para `Value` **uma vez**, roda `validate()` e `validate_serialized()`, falha com 500 se houver `errors`, loga `warnings` com `tracing::warn!`, e só então converte `Value` em `String`.
12. **Regras do verificador de JSON** (cada uma com mensagem que traz o caminho `$.children[2]...` e **nunca valores**, só nomes de chave/propriedade; valores `secure` jamais entram em mensagens ou logs):
    - **Escopo de chaves.** `Scope { keys: Known(set) | Opaque, vars }`. A raiz é `Opaque` (a tela pode ser um pedaço dentro de um escopo desconhecido). Widget com `state`/`globalState`/`onInit` abre escopo `Known(chaves de state)` que **substitui** o externo. Subárvores de destino/overlay (`navigate.widgetJson`, `showDialog.widget`, `showModalBottomSheet.widget`, `showSnackBar.content/action`) viram `Opaque` (decisão provisória D5: evita falso positivo; ganham `Known` de novo se declararem `state`).
    - **Erros (bloqueiam):** chave reservada em `state` (`global params result response event`); `persist`/`secure` em chave de tela (objeto só com `initial/persist/secure` dentro de `state`); `secure:true` com `persist:false` em `globalState`; `bind` vazio, não-texto, em `radio`, ou em widget fora de `{checkBox, switch, slider, radioGroup, textField, textFormField, dropdownMenu}`; `bind` junto da propriedade de valor que ele sobrescreve (`value` / `groupValue` / `initialValue` / `initialSelection`, conforme o widget); escrita (`setState.values`, `increment/decrement/toggle/append/removeFromState`, `toggleItem` — `resetState` não, o runtime ignora) em chave de tela não declarada quando o escopo é `Known`, ou em nome reservado; `bodyFromState` apontando `global.k` com `k` declarado `secure` em qualquer `globalState` da própria resposta; `state`/`globalState` que não sejam objeto; nome `response` fora de `onSuccess/onError/onFinally`, `event` fora de `onChanged/onSelected`, `result` fora de `onResult`, `item`/`index` fora de template (e sem chave de tela homônima) — em escopo `Opaque` esses cinco não são acusados.
    - **Avisos (não bloqueiam, só log):** leitura de chave de tela não declarada em escopo `Known` (decisão provisória D1).
    - **`params`** nunca é acusado (a tela pode ser destino carregado de outra, decisão provisória D5).
    - Detecção de nomes lidos: para `$state`, a raiz do caminho; para `$expr`, um varredor de tokens (ignora literais de texto com escapes, ignora identificador precedido de `.`, ignora identificador seguido de `(`, ignora `true/false/null`). Sem parser completo na biblioteca.
13. **Oráculo de precedência.** Teste Rust com parser de referência **só em teste** (gramática do contrato) que faz `parse(print(expr)) == expr` (estrutura, ignorando `Group`) para um gerador determinístico de árvores aleatórias (LCG com semente fixa, sem dependência nova) mais os casos manuais do plano; mais um dump canônico `Expr::sexp()` (notação S, ex.: `(+ a (* b c))`) usado pelo harness Dart (T9), que compara com o dump da AST do parser **real** do fork para as mesmas strings (`fixtures_reactive/expressions.json`).
14. **Fixtures e harness separados.** `crates/derust/tests/sdui_stac_v1/fixtures_reactive/{envelope,actions,expressions,lists,screens,...}/*.json` e novo harness `tools/stac-compat/v1-reactive/` (pubspec com o fork: `git: {url: https://github.com/deroldo/derust-stac, ref: estado_compartilhado_reativo, path: packages/stac}` e `dependency_overrides` de `stac_core`, `stac_framework`, `stac_logger` do mesmo repositório/ref; **um único lugar** (`ref`) para fixar o pin quando a pessoa definir; sobrescrita local opcional por `pubspec_overrides.yaml` apontando `path:` para o clone local). O harness reativo roda **também** os fixtures oficiais existentes (o fork tem que continuar aceitando tudo) e o teste de dump de expressões. Job de CI `stac-compat-v1-reactive` com `continue-on-error: true` enquanto não houver pin (decisão provisória D3).
15. **Inventário reativo.** `crates/derust/src/sduix/stacx/v1/inventory_state.json` à mão (não gerado), itens `{name, kind, source:"fork", json_key, status, fixture, notes}` para: cada propriedade do envelope, cada action nova, cada campo novo de `networkRequest`/`navigate`/listas, cada operador (unário, aritmético, comparação, igualdade, lógico, ternário, agrupamento, índice, campo) e cada forma de função (`length`, `trim`, `contains` 2 args, `contains` 3 args), cada forma de binding, cada raiz de variável. Teste de completude no mesmo molde de `fixtures_test.rs` (macro `assert_fixture!`/`assert_reactive_fixture!` que registra o fixture usado): falha se algum item sem fixture ou sem teste.
16. **Exemplo.** Novo `examples/sdui_reactive/` (workspace próprio como os demais), sem mexer em `examples/sdui`.
17. **Versão/CHANGELOG.** Entra na **0.7.0** (confirmado pela pessoa); a versão fica fixada em 0.7.0 (CHANGELOG na seção 0.7.0, sem bump adicional).
18. **Quebras de API aceitas** (pré-release, ver decisão provisória D6): campos públicos de structs geradas mudam de `Option<T>` para `Option<Reactive<T>>`; `visible`/`enabled` nativos deixam de ser campos; `NavigateAction.result` e campos `Map/Value` passam a `Data`; `Dyn<T>` ganha variante. Construtores/setters existentes continuam compilando quando possível (impls de compatibilidade do item 3).

## Dependências entre tarefas

```
T1 (expressões/binding) ──> T2 (Reactive, Data, Envelope, blocos) ──> T3 (gerador + spike) ──> GATE G1
                                                                                      │
          G1 ─┬─> T4 (actions de estado) ──┐
              ├─> T5 (networkRequest/navigate) ─┼─> T7 (verificador) ─> T8 (SduiResponse) ─┐
              └─> T6 (templates de lista) ──┘                                              │
T1 ──> T9a (esqueleto do harness reativo, dump de expressões)                              │
T4,T5,T6 ──> T10 (fixtures, inventário, exemplos do contrato) ──> T9b (harness completo) ──┤
                                                                                           ▼
                                                              T11 (exemplo) ─> T12 (docs) ─> T13 (verificação final)
```

- **Paralelo:** T4 ∥ T5 ∥ T6 (após G1; só acrescentam linhas em blocos `// --- T4 ---` etc. de `gen_rust.py`, `v1/state/mod.rs`, `v1/prelude.rs`; integrar em sequência com rebase); T7 pode começar após T2 com JSON escrito à mão e integrar após T4-T6; T9a ∥ T2/T3.
- **Sequencial:** T1 -> T2 -> T3 -> G1 -> (T4,T5,T6) -> T10; T7 -> T8; T11 depende de T8, T9b, T10; T12 depois de T11; T13 por último.
- **Conflito esperado:** `gen_rust.py`, `actions/mod.rs`, `widgets/mod.rs`, `inventory.json` e `prelude.rs` (T3 reescreve o gerador; T4-T6 só acrescentam). Regenerar com `python3 tools/stac-inventory/gen_rust.py <checkout oficial v1.6.0> crates/derust` e conferir que a regeneração é idempotente (segunda execução sem diff).
- **Gates de toda tarefa:** `cargo fmt --all -- --check`; `cargo clippy --all-targets --workspace -- -D warnings` (com e sem `--package derust --features sdui_stac_1`; `--no-default-features --features sdui_stac_1` também); `cargo nextest run`; `cargo nextest run --package derust --features sdui_stac_1`. (No workspace virtual usar `--package derust --features X`.)

---

## T1 — Núcleo de expressões e bindings (dificuldade: complexa)

**Contexto:** decisão 2 (e 13 para o oráculo). Não depende do gerador nem de outros tipos do `v1`. É a parte com maior risco de significado errado (precedência/escape/números) e a que o negócio mais exige provar.

**Arquivos (criar):** `v1/state/{mod,ident,path,expr,binding}.rs`; registrar `pub mod state;` em `v1/mod.rs`; testes unitários nos próprios arquivos e `crates/derust/tests/sdui_stac_v1/expr_test.rs` (declarar em `tests/sdui_stac_v1/main.rs`).

- [x] `Ident` (`new` não falha; `Validate` aplica `IDENT` e rejeita `true/false/null`), `StatePath` (+ `Root`, segmentos), `Display` do caminho (`global.token`, `user.name`, `items[0]`, `params.name`, `response.body.items`).
- [x] `Expr` completo (todas as variantes da decisão 2), açúcar de operadores, `From`s, `Validate` (números, inteiros >= 2^63, índice fracionário/`Div`, `Ident`s embutidos, recursivo com `Path`).
- [x] `Expr::to_source()` / `Display` segundo as regras de impressão (nível, associatividade, `Group`, unário duplo, base de postfix, literal numérico) e `Expr::sexp()` canônico (notação S estável; documentar o formato no doc do método).
- [x] `Binding` (`State(StatePath)` | `Expr(Expr)`), `Serialize` como mapa de uma chave.
- [x] Teste-oráculo da decisão 13: parser de referência **dentro de `expr_test.rs`** (tokenizer + descida recursiva da gramática do contrato) + gerador pseudoaleatório determinístico (mín. 5000 árvores, profundidade até 8) verificando `parse(to_source(e)) == e` e que a segunda impressão é idêntica; teste do varredor de nomes (posteriormente em T7) não é desta tarefa.

- [x] (estado por item) Testes extras de `Expr`/`StatePath`: `item.id`, `todo.name`, `i`, `contains(selected, item.id)`, `contains(selected, item, 'id')`, `length(selected) < 3 || contains(selected, item.id)` (precedência de `||` vs `<`).

**Testes (casos manuais obrigatórios):** `(a + b) * c`; `a + b * c`; `a - (b - c)` vs `a - b - c`; `a / (b * c)`; `-(a + b)`, `- -x`, `!(a && b)`, `!a == b` vs `!(a == b)`; ternário aninhado nos três lugares (`a ? b : c ? d : e` à direita; `(a ? b : c) ? d : e` com parênteses na condição); `(a || b) && c`; `(a + b).x`, `(1).x`, `items[n + 1]`, `items[n / 2]` -> `Validate` erro; funções aninhadas `length(trim(name)) < 3`; `contains(tags, 'none')`, `contains(list, obj, 'id')`; `Expr::num(-1.5)` -> `-1.5`; `2.0` -> `2`; `0.1`; `1e21` (imprime dígitos, mas inteiro >= 2^63 -> erro); `NaN`/`inf` -> erro; texto com `'`, `"`, `\`, `\n`, `\t`, `\r`, vazio, unicode (`'a\'b\\c'`, `''`); caminhos com `global.`; raízes inválidas (`user-name`, `1abc`, `é`, `true`) -> erro com caminho.

**Revisão (obrigatória):** conferir a impressão contra `expression/parser.dart` do fork (níveis em `_levels`, ternário em `parseTernary`, postfix em `_parsePostfix`); nenhum `unwrap`/`panic` na impressão; API não permite construir função desconhecida nem `$state` com operação.

---

## T2 — `Reactive`, `Data`, envelope, blocos de estado (dificuldade: média)

**Contexto:** decisões 3, 4, 5, 6, 7. Depende de T1 (`Binding`, `Ident`). Define os tipos que o gerador (T3) vai referenciar; **escrever e compilar tudo antes do gerador**.

**Arquivos (criar):** `v1/state/{reactive,data,envelope,decl}.rs`; editar `common/dynamic.rs` (variante `Bind` em `Dyn<T>`, `From<Binding>`; atualizar `Serialize/Validate` e testes) e `common/validate.rs` (adicionar `ValidationReport` e o método fornecido `validate_serialized` — a implementação do verificador é T7).

- [x] `Reactive<T>`, `IntoReactive<T>`, trait marcador `Plain` (+ impls para primitivos `bool i64 f64 String`, `serde_json::Value`, e macro para o conjunto de conversões de compatibilidade da decisão 3), `Serialize/Validate/Debug/Clone/PartialEq`.
- [x] `Data` (+ `From<serde_json::Value>`, `From<Binding|Expr|StatePath>`, `From` de primitivos, `From<Vec<_>>`, `FromIterator`, macro auxiliar `data!{...}` opcional) e o `Validate` que rejeita `Object` de uma chave `$state`/`$expr` com texto.
- [x] `BindKey`, `StateBlock`, `GlobalStateBlock`, `GlobalEntry` (+ construtores `GlobalEntry::value(..)`, `::declared(initial).persist(bool).secure(bool)`), `OnInit` (`One(Action)` | `Many(Vec<Action>)`; precisa de `Action` -> depende só do tipo já existente), `Envelope` com `serialize_into<M: SerializeMap>(&self, map: &mut M)` e `Validate`; `ActionEnvelope` não é necessário (a `condition` é campo direto).
- [x] **Spike de coerência (timebox 2h):** num teste de compilação (`crates/derust/tests/sdui_stac_v1/reactive_ergonomics.rs`) provar que `impl<T: Plain> IntoReactive<T> for T` coexiste com `impl<T> IntoReactive<T> for Expr/Binding/StatePath`, e que as chamadas abaixo compilam e inferem: setter `fn spacing(self, v: impl IntoReactive<f64>)` com `8`, `8.0`, `8_f32`, `Expr`, `StatePath`; `fn data(self, v: impl IntoReactive<String>)` com `&str`, `String`, `Expr`. Se o compilador rejeitar, registrar e **parar para decidir** (alternativa a discutir com a pessoa: tipo wrapper `Reactive::bind(..)` explícito, sem `Into`, às custas de ergonomia).

**Testes:** serialização de cada tipo contra JSON esperado (`{"$state":"a.b[0]"}`, `{"$expr":"..."}`, `GlobalEntry` simples e declarada com `initial/persist/secure`, `OnInit` lista); `Data::Object` com forma de binding -> `Validate` erro, com duas chaves -> ok; `result: None` vs `Some(Data::Null)`; `secure+persist(false)` -> erro; `Dyn::Bind` serializa como mapa.

**Revisão:** `Plain` é `pub` mas selado por convenção documentada (não implementar fora); nenhuma dependência nova; sem `unwrap`.

---

## T3 — Gerador: envelope, `Reactive`, `condition`, ganchos (dificuldade: complexa)

**Contexto:** decisões 3, 6, 7, 8 (gancho), 9, 10 (ganchos), 14 (fixtures oficiais intactos). Depende de T2. `tools/stac-inventory/gen_rust.py` (658 linhas) lê a tag oficial `v1.6.0` (clonar em diretório temporário: `git clone --depth 1 --branch v1.6.0 https://github.com/StacDev/stac <tmp>`), gera structs/enums/`dsl.rs`/`prelude.rs`/fixtures/testes. Tudo gerado é marcado `// @generated`.

**Arquivos (editar):** `tools/stac-inventory/{gen_rust.py,fixtures.py,README.md}`; regenerar `crates/derust/src/sduix/stacx/v1/**` e `crates/derust/tests/sdui_stac_v1/{generated_*.rs,fixtures/**}`; `v1/state/mod.rs` (reexports).

- [x] **Medir a linha de base** antes de mexer: `cargo build --package derust --features sdui_stac_1 --timings` e `cargo nextest run --package derust --features sdui_stac_1` (tempo e nº de testes); anotar no PR.
- [x] Envelope: acrescentar a toda struct de widget o campo `envelope: Envelope`, os setters e a serialização logo após o discriminador; `SKIP_FIELDS` novo para remover `visible`/`enabled` nativos (lista da seção "Fatos") sem duplicar a chave; `Validate` propaga ao envelope; `Widget` ganha métodos de conveniência (`.visible(..)` etc. via `Envelope`, opcional).
- [x] `condition` em toda action registrada (fora `NESTED_ONLY_ACTIONS`).
- [x] Conversão de campos-folha para `Reactive<T>` (decisão 3): classificar o tipo (`parse_type`), excluir widget/action; ajustar setters para `impl IntoReactive<T>`; para `Dyn<String>` manter o tipo (agora com `Bind`); `Validate` gerado chama `validate_at` do `Reactive`; emitir `impl Plain for X` para toda struct/enum gerada. Manter o corpo de `Option<Vec<Widget>>`/`Option<Widget>` como está.
- [x] Ganchos novos (apenas a infraestrutura, sem uso real ainda): `EXTRA_ACTIONS` (variante, `actionType`, caminho Rust; gera variante em `Action`, braços de `Serialize/Validate/type_name`, `From`), pseudo-tipos de `FIELD_TYPE_OVERRIDES/EXTRA_FIELDS` `Data`, `Actions`, `StringList`, `ItemTemplate`, `Reactive<...>` explícito, e `SKIP_FIELDS`.
- [x] Fixtures oficiais: `fixtures.py` continua gerando só valores simples (nenhum binding) em `fixtures/` (o harness oficial v1.6.0 continua aceitando); os testes gerados continuam comparando struct -> JSON.
- [x] Regenerar duas vezes (idempotente), `cargo fmt`, resolver erros de compilação em `dsl.rs`, `example_test.rs`, `custom_test.rs`, `core_test.rs`, `examples/sdui` (setters que mudam de tipo).
- [x] Teste de ergonomia no `reactive_ergonomics.rs` da T2 (agora contra tipos reais): `Container::new().padding(EdgeInsets::all(8.0)).visible(Expr::..)`, `Column::new().spacing(StatePath..)`, `Visibility::new(..).visible(true)` (campo nativo removido), `TextField::new().bind(..).enabled(..)`.

**Testes/verificação:** suíte existente (257 tipos) verde; `fixtures_test.rs` (completude) verde; `expr` e novos testes verdes; comparar tempo de build/testes com a linha de base (registrar; se o build com `sdui_stac_1` piorar mais que ~35%, parar e avaliar com a pessoa); serialização de um widget com envelope completo; widgets com `visible/enabled` nativos serializam a chave **uma vez**.

**Revisão (obrigatória, vai ao G1):** revisão independente (`superpowers:requesting-code-review`) do padrão gerado: ergonomia dos setters, inferência de tipos, ausência de ambiguidade em `.into()`, tempo de compilação, quebras de API listadas na decisão 18.

---

## GATE G1 — Validar o padrão antes de replicar

Antes de T4-T6: confirmar (humano ou `superpowers:requesting-code-review`) o resultado da T3 (ergonomia, inferência, tempo de compilação, quebras de API) e que `cargo nextest run` completo está verde. Mudanças de padrão são feitas aqui. Se o padrão exigir mudar uma regra de negócio (por exemplo, "qualquer propriedade aceita binding" ficar inviável), parar e perguntar.

---

## T4 — Actions de estado (dificuldade: média)

**Contexto:** escopo item 4; decisão 8. Contrato: seção "State actions" de `state_reference.mdx`. Todas aceitam `condition`; `key` aceita prefixo `global.`.

**Arquivos:** criar `v1/state/actions.rs` (e fixtures); editar `gen_rust.py` (preencher `EXTRA_ACTIONS` no bloco `// --- T4 ---`), `v1/prelude.rs`/`dsl.rs` atalhos (`set_state(..)`, `increment_state(key)`, ...).

- [x] Structs com builder e `Serialize` manual (chave JSON exata; omitir `None`): `SetState{values: BTreeMap<String,Data>}` (`actionType:"setState"`), `IncrementState/DecrementState{key: BindKey, by, min, max: Option<Reactive<f64>>}`, `ToggleState{key}`, `ResetState{keys: Option<Vec<BindKey>>}`, `AppendState{key, value: Option<Data>}`, `RemoveFromState` (decisão 8; `index: Option<Reactive<i64>>`, `value: Option<Data>`), `ToggleItem{key, value: Option<Data>, by: Option<String>}`, `CancelRequest{request_key: String}`, `NoneAction` (`actionType:"none"`), cada uma com `condition: Option<Reactive<bool>>`.
- [x] `Validate`: chave vazia, `Data` filho, `Reactive` filho. Regras contextuais ficam no verificador (T7).
- [x] Registrar em `EXTRA_ACTIONS`; regenerar; `Action::type_name` retorna os novos tipos.
- [x] Fixtures em `fixtures_reactive/actions/*.json` (um por action, mais variantes com `by/min/max`, `index` binding, `values` com binding aninhado, `condition` por `Expr`).

- [x] (estado por item) Fixtures/testes de `toggleItem` com `value` = `{$state:'item.id'}` e com `by`, e de `condition` com agregado (`length(selected) < 3`); `removeFromState` por `index` = variável do template.

**Testes:** serialização exata de cada action contra o JSON do contrato; `RemoveFromState` não compila sem alvo (doc-test `compile_fail`); `by`, `min`, `max` ausentes não aparecem; nulos explícitos não existem para esses campos; `Action::from`; estar dentro de `MultiAction`/`on_init`.

**Revisão:** nomes JSON conferem com `state_reference.mdx`; nada de `serde_json::Value` cru nas APIs novas.

---

## T5 — `networkRequest` e `navigate` estendidos (dificuldade: média)

**Contexto:** escopo itens 5, 6, 7; decisão 9 e 5. Contrato: seções "networkRequest" e "navigate". Arquivos gerados atuais: `actions/network_request.rs`, `actions/navigate_action.rs` (ver os campos na seção "Fatos"). Depende de G1.

**Arquivos:** `gen_rust.py` (bloco `// --- T5 ---`: `FIELD_TYPE_OVERRIDES`/`EXTRA_FIELDS` para `StacNetworkRequest` e `StacNavigateAction`); tipos à mão `Actions` em `v1/state/actions.rs`; regenerar; fixtures `fixtures_reactive/actions/network_request_*.json`, `navigate_*.json`.

- [x] `NetworkRequest`: `body/headers/query_parameters` -> `Data`/`BTreeMap<String,Data>`; novos `on_start/on_success/on_error/on_finally: Option<Actions>`, `request_key`, `body_from_state: Option<Vec<BindKey>>`; manter `url`/`content_type` como `Dyn<String>`.
- [x] `NavigateAction`: `result: Option<Data>` (substitui o `Option<Map>`), `params`, `arguments` (`Data`), `on_result`, `on_cancel`; `Validate` (decisão 11: `on_result`+`on_cancel`; `result` com estilo inválido).
- [x] `Actions` = `One(Box<Action>)` | `Many(Vec<Action>)`, `From<Action>`, `From<Vec<_>>`; serializa objeto ou lista na ordem dada.
- [x] Atualizar `NetworkResult`/uso de `NetworkRequest` aninhado em `navigate.request` sem quebrar os fixtures oficiais.

**Testes:** JSON dos exemplos "networkRequest" e "navigate" do contrato; `result` ausente vs `result: null` (dois fixtures); lista de callbacks preserva ordem; `onResult`+`onCancel` -> `Validate` erro; `result` com `Push`/sem estilo -> erro, com `Pop`/`PushReplacement`/`PushReplacementNamed` -> ok; `body` com binding aninhado profundo; `body` literal com forma de binding -> erro; `body_from_state` com `global.token`.

**Revisão:** os fixtures oficiais existentes de `networkRequest`/`navigate` continuam idênticos (nenhum campo reativo preenchido por padrão).

---

## T6 — Templates de lista (dificuldade: média)

**Contexto:** escopo item 8; decisão 10. Contrato: seção "List templates". Widgets: `ListView`, `GridView`, `PageView`, `SliverList`, `SliverGrid` (existem gerados). `children` segue funcionando sem `itemTemplate`. Depende de G1.

**Arquivos:** `gen_rust.py` (bloco `// --- T6 ---`: `EXTRA_FIELDS` + setter de alto nível); `v1/state/template.rs` (`ItemTemplate`, `ItemVars`); fixtures `fixtures_reactive/lists/*.json`.

- [ ] Campos `items`, `item_template`, `item_name` (`as`), `index_name` (`indexAs`) nos cinco widgets.
- [ ] `ItemTemplate::new(items: impl IntoReactive<Vec<Data>>)`, `.item_as(..)`, `.index_as(..)`; método `template(self, tpl, |vars: ItemVars| -> Widget)` nos cinco; `ItemVars::item()`/`index()` devolvem `StatePath` com o nome efetivo (vazio -> default); `ItemVars` expõe também `item_field("name")` como atalho.
- [ ] `Validate`: `as`/`indexAs` não vazios têm que ser `Ident` válidos (texto vazio é aceito pelo contrato e vira default; emitir a chave só quando preenchida).

- [ ] (estado por item) `ItemVars`: `item_field("id")`, `in_list(list: StatePath) -> Expr` (`contains(list, item.id)`/`contains_by`), `count(list) -> Expr` (`length(list)`); documentar que estado por item = conjunto de ids em chave de tela (seção "Estado por item de lista").

**Testes:** exemplo "lista" do contrato (com `as: todo`, `indexAs: i`, `removeFromState` por `index: {"$state":"i"}` e `(i + 1) + '. ' + todo.name`); nomes default; renomeação mantém as referências; listas aninhadas com nomes distintos; `itemTemplate` junto de `children` serializa os dois; `items` literal e por binding.

**Revisão:** nenhuma API que permita referenciar `item` com nome diferente do efetivo sem passar por `ItemVars`.

---

## T7 — Verificador de JSON (validação contextual) (dificuldade: complexa)

**Contexto:** decisões 11, 12. Depende de T2 (pode ser desenvolvida sobre JSON escrito à mão) e integra após T4-T6. Arquivo único novo `v1/state/check.rs` + `crates/derust/tests/sdui_stac_v1/check_test.rs`. A tela de entrada é um `serde_json::Value`.

- [ ] Estruturas: `Scope`, `Keys::{Known(HashSet), Opaque}`, conjuntos de variáveis ativas, `Report`.
- [ ] Pré-passada: coletar chaves `secure` de todos os `globalState` da resposta (declaração-objeto com `secure:true`).
- [ ] Passada principal recursiva com `Path`: reconhecer widget (`type` texto) e action (`actionType` texto); abrir escopo em `state/globalState/onInit`; aplicar as regras da decisão 12 (erros e avisos); propagar variáveis por posição (`itemTemplate` dos cinco widgets com `as`/`indexAs`, `onSuccess/onError/onFinally`, `onResult`, `onChanged/onSelected`); `Opaque` nos destinos/overlays.
- [ ] Varredor de nomes de `$expr` (decisão 12) e raiz de `$state`.
- [ ] Tabela de `bind`: widgets suportados e propriedade de valor (`checkBox/switch/slider: value`, `radioGroup: groupValue`, `textField/textFormField: initialValue`, `dropdownMenu: initialSelection`); `radio` com mensagem própria.
- [ ] Expor `pub fn check_screen(json: &Value) -> ValidationReport` e ligar em `Widget`/`Action` de `v1` (`validate_serialized`), via gerador (`gen_rust.py`, bloco `// --- T7 ---`) ou impl à mão se o enum for hand-written.

- [ ] (estado por item) Regras da seção "Estado por item de lista": escrita em chave local (`item/index/as/indexAs`) = erro, template com `state` próprio = aviso, sombreamento entre templates aninhados = aviso, escopo externo herdado pelo template.

**Testes (um por regra, positivo e negativo):** cada item de erro da decisão 12 com a mensagem esperada e o caminho; telas válidas (os 6 exemplos do contrato) -> sem erro e sem aviso; falsos positivos: `state` aninhado que **não** encadeia (escrever em chave do externo dentro do interno -> erro), escrita em `global.x` nunca acusada, `resetState` com chave não declarada ignorada, `params` livre, `item` com chave de tela homônima declarada, destino de `navigate.widgetJson` e `showDialog` opacos, tela raiz sem `state` opaca; `response` em `onStart` -> erro e em `onSuccess` -> ok; `event` em `onChanged` -> ok, em `onPressed` -> erro; `result` em `onCancel` -> erro; leitura de chave não declarada -> só aviso; varredor: `'item' + x`, `a.item`, `item(`, `true`, strings com aspas escapadas; mensagens **não contêm valores** (teste com `secure` cujo `initial` é um texto marcador e assert de que o marcador não aparece em nenhuma mensagem).

**Revisão:** só o que o contrato chama de erro bloqueia; cada regra aponta o trecho do contrato que a justifica (comentário `// contrato: <seção>`); nenhum `panic` em JSON malformado (valores de tipos inesperados viram issue, não pânico).

---

## T8 — Integração em `SduiResponse` e log de avisos (dificuldade: média)

**Contexto:** decisão 11. Depende de T7. Arquivo `crates/derust/src/httpx/response/sdui.rs` (hoje: `validate()` -> `to_string`).

- [ ] `try_new`: `validate()` (erros tipados) -> `serde_json::to_value` -> `validate_serialized` -> se houver erros, `DerustError::without_body(500, "failed to serialize SDUI screen: invalid screen: <issues>")` -> `warnings` via `tracing::warn!` (campo estruturado com `path` e mensagem, sem valores) -> `to_string(&value)`. Nenhum `unwrap_or_default`.
- [ ] Preservar `Content-Type`, `x-trace-id`, tags, `with_headers`, `screen()`.

**Testes:** tela reativa válida -> `Ok` com corpo igual ao fixture; tela com erro de cada camada (tipada e JSON) -> `Err(DerustError)` 500 sem corpo parcial; aviso apenas -> `Ok` e log emitido (capturar com `tracing_subscriber` de teste ou contador de camada, conforme o que já existir nos dev-deps; se nada existir, apenas garantir `Ok`); integração com handler Axum (`tower::ServiceExt::oneshot`, como `tests/sdui_response.rs`) devolvendo `Result<SduiResponse<Widget>, DerustError>`; `DerustTags` preservadas; mensagem de erro não contém valor `secure`.

**Revisão:** custo extra (uma serialização a mais para `Value`) medido com uma tela grande (ex.: 2000 nós) e registrado; sem regressão nos testes anteriores de `SduiResponse`.

---

## T9 — Harness de compatibilidade com o fork (dificuldade: complexa)

**Contexto:** decisão 14, escopo item 10. Dividida em T9a (após T1) e T9b (após T10). Harness atual: `tools/stac-compat/v1` (Flutter, `Stac.initialize()` + `StacRegistry.getParser/getActionParser(...).getModel(json)`, parse estrito; `test/compat_test.dart`, `allowlist.txt`, `test/negative/`). Fork local: `/Users/deroldo/workspace/derust-stac/.worktrees/estado_compartilhado_reativo`.

**Arquivos:** criar `tools/stac-compat/v1-reactive/{pubspec.yaml,analysis_options.yaml,README.md,allowlist.txt,test/compat_reactive_test.dart,test/expression_dump_test.dart,test/negative/*.json}`; editar `.github/workflows/ci.yml`.

- [ ] **T9a (spike curto):** `pubspec.yaml` apontando para o fork (decisão 14), `flutter pub get`; provar com um fixture `{"type":"text","data":{"$expr":"1 + 2"}}` que o parser do fork o aceita; escrever `expression_dump_test.dart`: lê `fixtures_reactive/expressions/expressions.json` (lista de `{source, sexp}`), parseia `source` com `StacExpression.parse` do fork (import `package:stac/src/state/expression/...`, aceito em teste), converte a AST (`StacBinary`, `StacUnary`, `StacTernary`, `StacCall`, `StacFieldAccess`, `StacIndexAccess`, `StacLiteral`, `StacIdentifier`) para o mesmo formato S de `Expr::sexp()` e compara. Se a AST não for acessível, **perguntar** (alternativa: avaliar com escopo fixo e comparar o valor).
- [ ] **T9b:** `compat_reactive_test.dart` percorre `crates/derust/tests/sdui_stac_v1/fixtures_reactive/**` **e** `fixtures/**` (oficiais) com o mesmo `checkTree` do harness atual; inclui a diferença: aceitar chaves de envelope; teste negativo (fixture com `bind` em estrutura inválida ou action desconhecida deve falhar).
- [ ] Teste Rust que gera/atualiza `fixtures_reactive/expressions/expressions.json` a partir de uma lista de `Expr` (comparação com o arquivo versionado; falha se divergir).
- [ ] CI: job `stac-compat-v1-reactive` com `continue-on-error: true` + comentário explicando; matriz registrando `ref` do fork e que não há pin.
- [ ] Se Flutter estiver indisponível localmente, documentar e validar no CI (como na T4 anterior).

**Testes/verificação:** `flutter test` verde localmente ou no CI; simular divergência (alterar um fixture) e ver falhar; alterar a precedência no `to_source` e ver o teste de dump falhar.

**Revisão:** isolamento (nenhum Dart no pacote publicado; `exclude` do `Cargo.toml` cobre `tools/`); mensagens acionáveis; único ponto de pin documentado no README.

---

## T10 — Fixtures, inventário reativo e exemplos do contrato (dificuldade: média)

**Contexto:** escopo itens 10 e critérios de aceite; decisão 15. Depende de T4-T6.

**Arquivos:** `v1/inventory_state.json`, `crates/derust/tests/sdui_stac_v1/{inventory_state.rs,reactive_fixtures_test.rs,contract_examples_test.rs}`, `fixtures_reactive/**`.

- [ ] `inventory_state.json` completo (decisão 15) e teste de schema (campos, nomes únicos, `status` válido).
- [ ] Macro `assert_reactive_fixture!("<categoria>/<nome>", valor)` (mesmo padrão de `assert_fixture!`) e teste de completude que falha se algum item `supported` não tiver fixture existente ou teste que o use.
- [ ] Um fixture e teste por: propriedade do envelope (incluindo `visible`/`enabled` nos widgets com campo nativo, `bind` em cada widget suportado), cada action, cada campo novo, **cada operador e cada função** (um fixture `expressions` por categoria, com a lista `{source, sexp}` consumida pelo T9).
- [ ] `contract_examples_test.rs`: reproduzir em Rust, **literalmente**, os 6 exemplos JSON de `state_reference.mdx` (copiar o JSON como fixture em `fixtures_reactive/screens/contract_*.json`) e comparar `serde_json::to_value(tela_em_rust)` com o JSON documentado (igualdade de `Value`, ordem de chaves irrelevante). Os dois exemplos de `bindings_and_expressions.mdx` (`price * qty`, `'Total: ' + price * qty`) idem.

- [ ] (estado por item) Fixture e exemplo "limite de 3 selecionados": lista com checkbox por item (`value`/`enabled` por expressão sobre `selected`, `onChanged` com `toggleItem`), reproduzido em Rust e aceito pelo harness; teste do verificador para as regras do template.

**Testes/verificação:** remover um fixture e ver o teste de completude falhar; tela de cada exemplo passa também no verificador (T7) sem erros.

**Revisão:** cada item do inventário cita a seção do contrato (`notes`); nada `unsupported` sem justificativa.

---

## T11 — Exemplo executável `examples/sdui_reactive` (dificuldade: média)

**Contexto:** escopo item 11. Depende de T8, T9b e T10. Seguir `examples/sdui` (workspace próprio, `derust = { path = "../../crates/derust", features = ["sdui_stac_1"] }`, handler `async fn(State<AppContext<_>>) -> Result<SduiResponse<Widget>, DerustError>` com `DerustTags`).

- [ ] Telas, só com tipos Rust: **formulário** (`textField` + `checkBox` com `bind`, texto de erro com `visible` por `Expr` tipada `length(trim(name)) < 3`, botão com `enabled` por expressão, `onPressed` com `setState` condicional); **lista** com adição (`appendState`) e remoção (`removeFromState` por `index`) usando `ListView::template`; **`networkRequest`** com `onStart/onSuccess/onError/onFinally`, `requestKey` e `cancelRequest`; **navegação** com `params`, `onResult`/`onCancel` e `navigationStyle: pop` com `result`. Uma rota que devolve erro de validação (500 pelo caminho padrão).
- [ ] A função que monta cada tela fica em `screen.rs` e é compilada também pelo teste `crates/derust/tests/sdui_stac_v1/example_reactive_test.rs` (como `example_test.rs` faz) contra `fixtures_reactive/screens/example_*.json`, aceitos pelo harness (T9b).
- [ ] `curl` manual: 200 com JSON e `content-type`; rota inválida -> 500 com mensagem que localiza o problema.

**Testes/verificação:** `cd examples/sdui_reactive && cargo build`; `cargo clippy` no exemplo; testes da crate verdes; `curl` documentado no README do exemplo (se houver).

**Revisão:** o exemplo não escreve JSON nem expressão em texto; usa só `derust::sduix::stacx::v1::prelude`.

---

## T12 — Documentação, matriz, CHANGELOG (dificuldade: simples)

**Contexto:** escopo item 11. Depende de T11. Arquivos existentes: `crates/derust/src/sduix/stacx/README.md` (README do módulo, incluído como doctest `no_run`), `README.md`, `CLAUDE.md`, `CHANGELOG.md`, `tools/stac-inventory/README.md`, `tools/stac-compat/v1-reactive/README.md`.

- [ ] README do módulo: nova seção "Estado reativo" (blocos de estado, bindings, expressões, envelope, actions, `networkRequest`/`navigate`, listas), quando um erro de validação ocorre (lista de regras em linguagem simples), avisos de runtime não verificáveis no servidor (`'a' < 1`, divisão por zero, `!` em não booleano, tipos que dependem de valor: resultado `null` ou `false`), concatenação (`+` com `null`/lista/objeto -> `null`; número inteiro imprime sem `.0`), índice inteiro (`/` não vale como índice), curto-circuito (derust não reordena), `global.chave` vs `global['chave']`, `enabled` só em lista fechada de widgets, `secure` nunca em log/mensagem.
- [ ] **Avisos das limitações v1 do fork** (só documentar): `params` com rotas nomeadas (`pushNamed*` via `arguments["__stacParams"]`), identidade de item por índice, `state` aninhado que não encadeia, mascaramento `secure` por substituição de texto, `separator` fora do escopo do item, `onChangeStart`/`onChangeEnd` do slider sem `event.value`.
- [ ] Matriz de compatibilidade (README raiz e do módulo): linha nova "vocabulário reativo exige o runtime do fork `deroldo/derust-stac`, branch `estado_compartilhado_reativo`, **sem versão fixada** (contrato lido no commit `92457f25`)"; frontend com Stac oficial ignora ou falha; a matriz e o CHANGELOG são a única proteção.
- [ ] `CHANGELOG.md` (Added/Changed/**Breaking** com a lista da decisão 18), linha em `CLAUDE.md`, README do harness com o ponto único de pin.
- [ ] Atualizar contagens do inventário no README do módulo.

**Testes:** o doctest do README compila (`cargo test --doc --features sdui_stac_1`); links internos válidos.

**Revisão:** nada prometido além do inventário; avisos conferidos contra a seção "v1 limitations" do contrato.

---

## T13 — Verificação final (dificuldade: simples)

**Depende de:** T1-T12.

- [ ] Inventário oficial e reativo: nenhum item `supported` sem fixture e teste; gerador idempotente (rodar duas vezes sem diff).
- [ ] Todos os gates: `cargo fmt --all -- --check`; `cargo clippy --all-targets --workspace -- -D warnings`; `cargo clippy --all-targets --package derust --features sdui_stac_1 -- -D warnings`; `cargo clippy --package derust --no-default-features --features sdui_stac_1 -- -D warnings`; `cargo nextest run`; `cargo nextest run --package derust --features sdui_stac_1`; `cargo nextest run --package derust --all-features`; `task lint`; `task test`; exemplos compilam (`examples/*`).
- [ ] Sem a feature: nada muda no build nem na API pública (`cargo doc -p derust` sem `sdui_stac_1` não lista `sduix`).
- [ ] Critérios de aceite do plano de negócio, um a um, marcando evidência (teste/fixture/arquivo).
- [ ] Revisão final do diff (`superpowers:requesting-code-review`); CI da branch (`rust`, `stac-compat-v1`, `stac-compat-v1-reactive`).
- [ ] Tempo de build e de testes comparado com a linha de base da T3 e registrado.

---

## Estado por item de lista (revisão após rejeição da D7)

**Requisito (pessoa):** bindings, condições e actions de estado dentro de um `itemTemplate` devem poder ler e escrever estado por item. Exemplo ilustrativo (não é regra do contrato): cada item com checkbox; ao selecionar 3 itens, os demais ficam desativados.

**O que o contrato do fork cobre (`state_reference.mdx`, `stac_item_template.dart`, `stac_scope.dart`):**
- `item` e `index` são variáveis locais **somente leitura**; não são chaves de estado. Não existe action que escreva em `item`, em campo de item (`todos[2].done`) ou em posição da lista; `setState` escreve a chave inteira e a linguagem de expressões não tem como montar uma lista modificada (sem map/filter/set).
- Toda action dentro do template enxerga o `state` de tela externo (o template só acrescenta variáveis locais) e pode escrever nele; `toggleItem` (com `by`), `appendState` e `removeFromState` (por `index` ou `value`) editam listas do estado.
- Identidade de item = posição (limitação v1 do fork). `state` declarado dentro do widget do template cria escopo interno **por instância** que substitui o externo (as chaves externas, como o agregado, ficam ilegíveis ali) e fica preso à posição.

**Lacuna:** não há atualização in-place de campo de item (ex.: `todos[i].done = true`). Registrada como lacuna do contrato; **não** inventar action nova.

**Abordagem mais conservadora (adotada):** estado por item = **estado de tela (ou global) indexado por identidade do item**, não escopo por item.
- Endereçamento: uma chave de tela com lista de identificadores/itens (ex.: `selected: []`). O item pertence ao conjunto se `contains(selected, item.id)` (ou `contains(selected, item, 'id')` para objetos). Para itens sem id estável, usar `index`.
- Escrita por item: `toggleItem{key:'selected', value:{$state:'item.id'}}` (ou `value` = item inteiro com `by:'id'`), `appendState`/`removeFromState` (por `index`/`value`). Valor do checkbox: `{$expr: "contains(selected, item.id)"}`; callback `onChanged` com `toggleItem`/`condition` usando `event.value` quando necessário.
- Agregado: leitura de qualquer chave de tela dentro do template, ex.: `length(selected)`. Condição por item que depende do agregado: `enabled: {$expr: "length(selected) < 3 || contains(selected, item.id)"}`; `condition` da action pode usar a mesma expressão (ex.: só adiciona se `length(selected) < 3`). Como o agregado é uma chave comum, todos os itens reagem quando ela muda (reatividade por chave de topo, `selected`).
- `bind` não serve por item (escreve uma chave de tela inteira); documentar.
- Escopos internos por item (`state` no widget do template) **não** devem ser usados para agregados; o verificador emite aviso.

**Pergunta pendente para a pessoa (não decidir sozinho):** o padrão de conjunto-de-ids cobre o caso de seleção/limite, mas **não** edição in-place de campos de item (ex.: contador por item, texto editado por item). Se isso for necessário, é uma extensão do contrato no fork (nova action, ex. escrita em `key[index].campo`) e, por ser mudança de negócio/contrato, precisa de decisão do usuário e de pin do fork. Sem essa resposta, o derust cobre só o padrão acima.

**Impacto nas tarefas:** T1 (testes de caminho `item.campo`/`index` e de `contains(.., item.id)` e `contains(.., item, 'id')`), T4 (`toggleItem` com `by`, `value` por binding de `item`, `condition` com agregado), T6 (`ItemVars` com helpers `in_list(list_path)` -> `Expr contains` e `count(list)` -> `length`, e tipos aceitam `ItemVars::item()` em `Data`), T7 (regras abaixo), T10 (fixtures e exemplo do limite de 3).

**O que o verificador (T7) passa a validar no escopo do template:**
- Escrita (`toggleItem`, `appendState`, `removeFromState`, `setState`, `increment/decrement`, `toggleState`) dentro do template em chave de tela: erro se a chave não estiver declarada no escopo `Known` **externo ao template** (o template herda o escopo de chaves da lista; só `item/index/as/indexAs` são acrescentados).
- Escrita cuja `key` coincide com o nome efetivo de `item`/`index`/`as`/`indexAs`: erro (variável local é somente leitura, não é chave).
- `itemTemplate` cujo widget raiz declara `state`: aviso (v1: o escopo interno esconde as chaves externas, inclusive o agregado) e erro se, nesse caso, alguma action/escrita dentro apontar para chave do escopo externo (já cai na regra de chave não declarada).
- Leitura de `item`/`index` fora do template continua erro; dentro de templates aninhados valem os nomes dos dois níveis (nomes iguais: aviso de sombreamento).
- `toggleItem.by` e `removeFromState.index` por `index` do template são aceitos sem checagem de valor (dependem de runtime).
- Não valida em runtime se a lista `selected` contém ids (comportamento do runtime documentado).

## Decisões provisórias (ambiguidades do plano de negócio; confirmar com a pessoa)

- **D1 — Leitura de chave não declarada:** a pergunta 1 do plano diz "avisos seguem como registro" e, entre parênteses, "inclui leitura de chave não declarada, que o runtime apenas avisa". Interpretado como **não bloqueia** (vira aviso logado). Se a intenção era bloquear, trocar uma linha no verificador (T7). Motivo adicional: `global.x` pode ser declarado em outra tela e `state` externo não encadeia, então bloquear gera falso positivo.
- **D2 — Objeto literal com forma de binding:** o contrato não tem escape, então o "dado literal que não vira binding" é impossível de enviar. Decisão: **rejeitar** no servidor (erro) em vez de emitir. Se a pessoa quiser uma saída de escape, é pedido de negócio novo.
- **D3 — CI contra o fork sem pin:** job `stac-compat-v1-reactive` com `continue-on-error: true` até existir pin; o harness referencia a branch em um único ponto (`ref`).
- **D4 — `NoneAction`** (`{"actionType":"none"}`) incluída porque o exemplo do contrato a usa e o critério de aceite exige reproduzir os exemplos; não está na lista do escopo.
- **D5 — Verificador conservador:** raiz da resposta, destinos de `navigate.widgetJson` e overlays (`showDialog`, `showModalBottomSheet`, `showSnackBar`) são escopo opaco (sem acusar escrita/variável); `params` nunca é acusado. Mais falsos negativos, nenhum falso positivo conhecido.
- **D6 — Quebra de API pública** (pré-release): campos de structs geradas viram `Option<Reactive<T>>`, `visible`/`enabled` nativos deixam de ser campos, `NavigateAction.result` vira `Data`. Construtores e setters seguem compilando quando possível.
- **D7 — REJEITADA pela pessoa** (queria estado atualizável por item de lista). Substituída pela seção "Estado por item de lista" abaixo.
- **D8 — CONFIRMADA:** a mudança entra na 0.7.0.

D1 a D6 seguem provisórias e aguardam confirmação.

## Pontos em aberto (não bloqueiam o início)

1. Viabilidade do spike de coerência de `IntoReactive` (T2/T3): se falhar, escolher entre API explícita (`Reactive::bind(..)`) ou outro desenho, **com a pessoa**.
2. Acesso à AST do parser do fork a partir de teste Dart (T9a); alternativa: comparar por avaliação com escopo fixo.
3. Pin do fork (commit/tag): definido pela pessoa; até lá, matriz diz "sem versão fixada".
