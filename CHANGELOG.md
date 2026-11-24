# Changelog

A versão do VoidGate é uma só para a API e para o site. A tag `v0.1.0` marca a primeira versão. Daqui em diante, um commit `feat:` na `main` sobe o minor, um `fix:` sobe o patch, e `feat!:`, `fix!:` ou `BREAKING CHANGE` sobe o major. O resto não gera versão.

## [0.4.0] - 2026-11-24

- feat(python): Guard configurável, inspect_messages e voidgate.openai.protect
- fix(core): e-mail antes de ponto final vazava sem máscara
## [0.3.1] - 2026-11-24

- fix(python): pacote leva LICENSE e página do PyPI em inglês
## [0.3.0] - 2026-11-22

- fix(python): offsets em uma passada só
- feat(core): mascara telefone brasileiro
- fix(core): disfarce visual não fura mais a lista de injeção
- fix(ci): mínimo Python 3.10 (3.9 é EOL e some do runner)
- feat: voidgate-core como biblioteca e pacote Python
- fix(dashboard): aviso de dados incompletos expira com a janela (#10)
- fix(deps): bump url 2.5.8 -> 2.5.9 (RUSTSEC-2026-0592)
- fix(dashboard): reject malformed event pages
- fix(events): keep test-only snapshot helpers out of release
- feat(events): add typed actions and incremental event feed
- feat(web): adapt panel screens to metadata-only real events
- feat(web): read real decision events from the API in the panel
- feat(events): add GET /v1/events with metadata-only decision events
- feat(events): record proxy decisions as one input and one output event
- feat(events): record chat decisions and count record failures
- feat(events): add in-memory decision event log
## [0.2.0] - 2026-11-21

- feat(web): add event list, filters and plain-language detail to the panel
- feat(web): add live river and threats-per-minute chart to the panel
- feat(web): add monitoring panel with simulated events and threat status
## [0.1.0] - 2026-11-19

### 2026-11-19

- A API e o site passam a viver neste repositório, em `src/` e `web/`.
- A decisão de chat é `allow`, `mask` ou `block`. Bloqueio vence máscara.
- Detector de manipulação de prompt em várias línguas, com o código `content.prompt_injection`.
- Detector de dado pessoal (`content.personal_data`): e-mail, telefone E.164, cartão, IBAN, CPF, CNPJ e SSN. A resposta da decisão traz só as posições, nunca o valor.
- `POST /v1/proxy/chat` encaminha ao modelo em `VOIDGATE_UPSTREAM_URL`. Sem essa variável a rota responde 503. Bloqueio não chama o modelo. Máscara troca o trecho por `[redacted]` na ida e na volta.
- O site está em português e inglês. A demonstração no navegador cobre telefone, cartão e IBAN, e não chama a API.
- Suíte em `tests/` para a API pública e para a demonstração.
- Documentação do produto, README de cada pasta e o site em `https://voidgates.github.io/VoidGate/`.
- O histórico da API e do site entrou neste repositório para manter as contribuições de Kaell Calacina, Matheus Coutinho e Lucas.

### 2026-11-16

- `POST /v1/decisions/chat` bloqueia caractere invisível (`content.invisible_unicode`) e devolve só offsets.
- Limite de pedidos por credencial, contadores em `GET /metrics` e encerramento que espera o pedido em andamento.
- Contrato único de erro e tela `/docs` só em desenvolvimento, só em loopback.
- O PR Guard lê o diff com o scanner da branch padrão e roda na merge queue.
- Correções apontadas pelo relatório de qualidade da demonstração.

### 2026-11-14

- Esqueleto da API em Rust, com chave `x-voidgate-key`, caminho de isca e `Cargo.lock`.
- Site: abertura, “Tente enganar a IA”, comparação, “Como funciona” e o quiz.
- Primeira versão do PR Guard.
- Publicação do site pelo GitHub Actions.

### 2026-11-10

- Templates iniciais da API e do site.
