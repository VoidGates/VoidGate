# Changelog

A versão do VoidGate é uma só para a API e para o site. A tag `v0.1.0` marca a primeira versão. Daqui em diante, um commit `feat:` na `main` sobe o minor, um `fix:` sobe o patch, e `feat!:`, `fix!:` ou `BREAKING CHANGE` sobe o major. O resto não gera versão.

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
