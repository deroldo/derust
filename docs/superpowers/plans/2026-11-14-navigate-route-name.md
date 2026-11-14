# Validação e documentação de `routeName` no `NavigateAction` — Refinamento

**Objetivo:** `NavigateAction.route_name` (JSON `routeName`, `Option<Dyn<String>>`) já existe, gerado do modelo oficial v1.6.0. Falta validar o formato, testar, inventariar e documentar. Mesma branch (`sdui_stac`) e mesma versão (0.7.0); sem publicação, tag ou push.

**Contrato (fork `derust-stac`, commit `4cd33635`, `docs/actions/navigate.mdx`):** chave `routeName`, texto, opcional, vale também com `request`. O runtime não impõe formato: o valor vai para `RouteSettings(name:)`. Ao lado de `request`/`widgetJson`/`assetPath` em push, pushReplacement e pushAndRemoveAll (estilo ausente conta como push), é o nome da rota (URL na web) e a tela continua vindo do request. Sem eles, é nome de tela Stac (`detail_screen`) em push-style, ou nome de rota Flutter (`/settings`) nos estilos nomeados. Na web o Flutter faz `Uri.parse(routeName)`.

## Regras de validação (só valor literal; binding `Dyn::Bind` e `{{chave}}` não são checados)

1. Com `request`/`widgetJson`/`assetPath` e estilo push, pushReplacement, pushAndRemoveAll ou ausente: começar com `/screens/`; relativo (sem `//` inicial, sem esquema/host, sem `://`, sem `:` antes da primeira `/`); sem espaço, caractere de controle ou `\`; query (`?a=b`) permitida.
2. Demais casos: só não vazio e sem espaço/controle (`pushStac('detail_screen')`, `pushNamed('/settings')` seguem válidos).
3. Mensagens com o caminho (`$.routeName`), sem o valor. As regras de `result`/`onResult`/`onCancel` não mudam.

## Decisões

- Estilo ligado por binding (`Reactive::Bind`): desconhecido em tempo de geração, aplica-se só a regra básica.
- `://` é rejeitado em qualquer posição (inclusive na query) na regra estrita; é a leitura conservadora de "nada de `://`".
- O valor aparece na barra de endereço e no histórico do navegador: não pode conter segredos/tokens (não verificável; documentado).

## Tarefas

- [x] N1. Regras em `NAVIGATE_CHECKS` (gerador) + helper `state/route_name.rs`; regenerado, idempotente. (`feat(sdui)`)
- [x] N2. Testes de serialização e validação + fixtures `fixtures_reactive/actions/navigate_route_name_*.json`. (`test(sdui)`)
- [x] N3. Inventário `inventory_state.json` (`navigate.routeName`, source `fork`). (`feat(sdui)`)
- [ ] N4. Harness Dart v1-reactive aceita o novo fixture; `flutter test` verde no reativo e no oficial v1. (`test(sdui)`)
- [ ] N5. README do módulo, CHANGELOG 0.7.0 (Added), este refinamento. (`docs(sdui)`)
