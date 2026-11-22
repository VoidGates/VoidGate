<!--
Conversão para PDF A4
Capa e faixas: fundo #110d0b, painéis #1b1613, linhas #2f2722, texto #efe7da, texto secundário #a89c8c, destaque #d8ae62.
Miolo: fundo #f7f3ec, texto #1b1613. Bloqueio #f0594e. Liberado #7fc4a4.
Títulos: Bricolage Grotesque. Corpo: Instrument Sans ou Source Sans 3. Código e tabelas: JetBrains Mono.
Cabeçalho: VoidGate
Rodapé: Documentação do produto · versão do código 0.1.0 (API) / 0.0.0 (site) · confidencial para uso comercial
Numerar a partir do sumário. Quebrar a página em cada marcação <!-- página -->.
-->

# VoidGate

## A camada de segurança entre a aplicação e o modelo de IA

Feita para o português e para a Lei Geral de Proteção de Dados (LGPD).

Documentação do produto para sócios, clientes de piloto, parceiros e engenheiros que integram a API.

<!-- página -->

## Sumário

1. O problema
2. O que é o VoidGate
3. Estado do produto
4. Como a decisão funciona hoje
5. Segurança operacional da API
6. Arquitetura do backend
7. O site de apresentação
8. Modelo de negócio
9. Como colocar no ar
10. Limites conhecidos
11. Fecho

<!-- página -->

## 1. O problema

Empresas e órgãos públicos estão colocando chatbots de inteligência artificial no atendimento. O modelo recebe a frase do cliente, consulta as instruções da organização e devolve uma resposta. Esse caminho é útil. Também é frágil.

Uma frase pode convencer o modelo a ignorar as regras e devolver dados de clientes. O prejuízo recai sobre quem opera o atendimento. O fornecedor do modelo não absorve a multa, o vazamento nem a perda de confiança.

Três fatos delimitam o espaço em que o VoidGate entra.

**Nº 1 no OWASP.** Injeção de prompt é o risco número 1 da lista OWASP Top 10 para aplicações com modelos de linguagem. A instrução maliciosa chega no mesmo canal da pergunta legítima. O modelo não separa, por conta própria, o pedido do cliente da regra da empresa.

**2% do faturamento.** A LGPD prevê multa de até 2% do faturamento, limitada a R$ 50 milhões por infração, quando dados pessoais vazam. Um chatbot que repete um CPF que não deveria ter visto já é um incidente da organização.

**pt-BR.** A maior parte das proteções de IA foi escrita para o inglês. Golpe, cadastro e documento no Brasil aparecem em português, com CPF e com o vocabulário da LGPD. Uma lista de frases em inglês deixa esse tráfego passar.

O VoidGate fica entre o chatbot e o modelo. A visão do produto é barrar manipulação e esconder dados pessoais antes que vazem, com qualquer modelo: na nuvem, da empresa ou local.

<!-- página -->

## 2. O que é o VoidGate

O VoidGate é uma camada adicional e opcional de segurança para aplicações e fluxos de IA. O núcleo é aberto. A receita prevista está na nuvem e no contrato, não na licença do código.

O trabalho está num repositório, [VoidGate](https://github.com/VoidGates/VoidGate):

| Pasta | Papel | Versão nesta documentação |
| --- | --- | --- |
| `src/` | API de decisão em Rust | 0.1.0 |
| `web/` | Site de apresentação | 0.0.0 |

A visão de produto coloca a camada nos dois sentidos da conversa:

```text
Aplicação  →  VoidGate  →  modelo  →  VoidGate  →  Aplicação
```

Quatro etapas descrevem essa visão:

| Etapa | O que acontece |
| --- | --- |
| 1. A aplicação envia | O chatbot entrega a mensagem do cliente ao VoidGate. |
| 2. O VoidGate confere | Procura manipulação e dados pessoais. Decide liberar, esconder ou bloquear. |
| 3. O modelo responde | Só o que a decisão considerou seguro segue para o modelo. |
| 4. A resposta volta | Se a resposta trouxer dado sensível, ele é escondido antes de chegar ao cliente. |

Três públicos orientam o desenho:

| Público | Uso |
| --- | --- |
| Startups e PMEs | Colocam um chatbot no ar com uma camada na frente dos dados de clientes. |
| Órgãos públicos | Usam IA no atendimento e precisam de registro do que foi bloqueado, como a LGPD pede. |
| Desenvolvedores | Querem proteção que entenda português sem montar um time de segurança. |

`POST /v1/proxy/chat` é esse endereço. Sem `VOIDGATE_UPSTREAM_URL`, a rota responde 503 e a API de decisão continua disponível para quem prefere aplicar `allow`, `mask`, `block` ou `decoy` no próprio gateway.

<!-- página -->

## 3. Estado do produto

A API 0.1.0 decide. O site mostra a visão. As duas coisas convivem e não se confundem.

### Já funciona

Isto está no código do backend 0.1.0:

| Capacidade | Onde aparece |
| --- | --- |
| Bloqueio de caracteres invisíveis e Trojan Source | `POST /v1/decisions/chat` |
| Caminho HTTP marcado para resposta decoy | `POST /v1/decisions/http` e `VOIDGATE_DECOY_PATHS` |
| Limite de uso por credencial | `VOIDGATE_RATE_LIMIT_PER_MINUTE` |
| Limite de decisões simultâneas e tempo máximo por decisão | 256 em paralelo; 5 segundos |
| Métricas agregadas | `GET /metrics`, com a chave da API |
| Eventos de decisão, só metadados | `GET /v1/events`, com a chave da API |
| Saúde do processo | `GET /health`, `GET /live`, `GET /ready` |
| Autenticação por chave | Cabeçalho `x-voidgate-key` |
| Erro em formato único | `error.code` e `error.message` fixos |
| Especificação OpenAPI | Só em build de desenvolvimento, em `/docs` |

### Já funciona, além da lista acima

- Detecção de manipulação de prompt em português, inglês, espanhol, francês, alemão e chinês, mais alguns marcadores de modelo.
- Dados pessoais com ação `mask`: e-mail, telefone E.164, cartão (Luhn), IBAN, CPF, CNPJ e SSN. A resposta da decisão traz offsets. O proxy substitui o trecho por `[redacted]`.
- `POST /v1/proxy/chat`, quando `VOIDGATE_UPSTREAM_URL` está definido. Sem a variável a rota responde 503.

### Próximo

- Painel web.
- Relatórios de conformidade.
- Várias empresas na mesma instalação.

### Depois

- Ambiente-isca para estudar atacantes.

### O que a demonstração faz no navegador

O site classifica manipulação, CPF e e-mail. Essa classificação roda no navegador, com regras simplificadas, e não envia nada a servidor. Não é o motor do backend.

A API de chat detecta caracteres ocultos, manipulação e dados pessoais. A ação `mask` não devolve o texto: só os offsets. Quem mascara o valor é o proxy, ou o gateway, usando esses offsets.

<!-- página -->

## 4. Como a decisão funciona hoje

A API não é proxy. Não executa a isca. Não fala com o modelo. Responde `allow`, `block` ou `decoy`. O gateway aplica.

A integração falha fechada. Qualquer resposta que não seja HTTP 200 — erro, tempo esgotado ou conexão recusada — é bloqueio. Um 400 por caminho malformado também é bloqueio, nunca liberação.

O corpo da decisão fica no gateway. `reason_code`, `reason_codes`, `findings` e o JSON de erro dizem qual defesa disparou. O cliente final recebe uma frase genérica.

### POST /v1/decisions/http

Exige o cabeçalho `x-voidgate-key` e um JSON com `method` e `path`. O corpo cabe em 4 KiB.

| Situação | Resposta |
| --- | --- |
| O caminho está em `VOIDGATE_DECOY_PATHS` | `{"action":"decoy","reason_code":"registered_decoy_path"}` |
| O caminho é válido e não é isca | `{"action":"allow","reason_code":"no_matching_rule"}` |

A comparação é exata. `/internal/backup/`, `/Internal/Backup` e `/internal/backup;x=1` não são a isca `/internal/backup`. O gateway normaliza antes de perguntar: decodifica, remove a barra final e aplica a mesma regra de maiúsculas da aplicação protegida.

Caminho com `%`, `?` ou caractere fora do ASCII recebe 400. Isso é pedido malformado do gateway.

A API marca o caminho. Quem opera o gateway é quem devolve a página isca. O processo VoidGate não hospeda esse ambiente.

### POST /v1/decisions/chat

Exige o cabeçalho `x-voidgate-key` e um JSON com exatamente dois campos:

| Campo | Contrato |
| --- | --- |
| `role` | `user`, `system` ou `assistant` |
| `content` | Texto UTF-8 de até 16 KiB |

O corpo da rota cabe em 100 KiB, para que um `content` de 16 KiB ainda caiba quando cada caractere sai escapado como `\uXXXX`.

O `content` é lido em memória e descartado. Não volta na resposta. Não entra em log. A resposta traz posições em bytes do texto original, com fim exclusivo.

```json
{
  "action": "allow",
  "reason_codes": [],
  "findings": [],
  "truncated": false
}
```

`action` é `allow`, `mask` ou `block`. `block` vence `mask`. Cada item de `findings` traz `code`, `detector`, `start` e `end`. `content.personal_data` pede máscara. `content.prompt_injection` e `content.invisible_unicode` bloqueiam. Caracteres ocultos seguidos formam um único intervalo. A lista traz no máximo 100 itens. `truncated` vem `true` quando havia mais. A decisão conta todas as ocorrências, inclusive as que ficaram de fora da lista.

Nesta versão o único motivo é `content.invisible_unicode`, do detector `invisible_unicode`. Qualquer ocorrência fora das exceções abaixo gera `block`.

| Grupo | Pontos de código |
| --- | --- |
| Largura zero e marcas de direção | U+200B–U+200F, U+2060–U+206F, U+FEFF |
| Controles bidirecionais (Trojan Source) | U+202A–U+202E, U+2066–U+2069, U+061C |
| Separadores de linha e parágrafo | U+2028, U+2029 |
| Caracteres tag (texto ASCII escondido) | U+E0000–U+E0FFF |
| Preenchimentos em branco | U+00AD, U+034F, U+115F, U+1160, U+17B4, U+17B5, U+180B–U+180F, U+3164, U+FFA0 |
| Anotação e formato musical | U+FFF9–U+FFFB, U+1D173–U+1D17A |
| Controles | U+0000–U+001F, exceto tab, LF e CR; U+007F–U+009F |

Exceções, porque entram na escrita normal. Todas exigem vizinhos visíveis do tipo certo. Nenhuma serve para esconder uma frase.

| Caractere | Liberado quando |
| --- | --- |
| U+200D (ZWJ) | Entre dois emoji, ou entre duas letras árabes ou índicas |
| U+200C (ZWNJ) | Entre duas letras árabes ou índicas |
| U+200E e U+200F | Ao lado de texto hebraico ou árabe |
| U+FEFF (BOM) | É o primeiro caractere do texto |

Os seletores de variação U+FE00–U+FE0F ficam de fora. U+FE0F faz parte de emoji comuns. Emoji e escritas são reconhecidos por faixas de código. Se um texto legítimo for bloqueado, a faixa correspondente entra em `src/domain/detectors/invisible_unicode.rs`.

<!-- página -->

## 5. Segurança operacional da API

O processo recusa subir com a defesa desligada por engano. Configuração inválida, nome `VOIDGATE_` desconhecido, arquivo `.env` ilegível ou porta em uso encerram a partida com `error:`. Uma variável do ambiente do processo vence o `.env`. Em produção não há `.env`.

### Configuração

| Variável | Obrigatoriedade | Efeito |
| --- | --- | --- |
| `VOIDGATE_API_KEY` | Obrigatória | 32 a 128 bytes, sem espaço nas pontas e sem caractere de controle. Viaja no cabeçalho `x-voidgate-key`. |
| `VOIDGATE_DECOY_PATHS` | Opcional | Caminhos exatos separados por vírgula. Padrão: nenhum. |
| `VOIDGATE_BIND` | Opcional | Endereço de escuta. Padrão: `127.0.0.1:8080`. |
| `VOIDGATE_RATE_LIMIT_PER_MINUTE` | Opcional | Inteiro de 1 a 10000. Padrão: 60. |
| `VOIDGATE_DEV_DOCS` | Opcional | `0` ou `1`. Só existe em build de desenvolvimento. |

O limite por credencial permite uma rajada até `VOIDGATE_RATE_LIMIT_PER_MINUTE` e repõe a capacidade ao longo de um minuto. Acima disso a resposta é 429, com `Retry-After` em segundos. Pedido sem chave válida responde 401 e não consome essa cota. Há uma única chave nesta versão. Os baldes ficam em memória, um conjunto por réplica. O limite ainda não é global.

`VOIDGATE_DEV_DOCS=1` publica o Swagger UI em `/docs` e a especificação em `/docs/openapi.json`. O bind precisa ser loopback. O cabeçalho `Host` precisa ser `127.0.0.1`, `localhost` ou `[::1]`. O binário de release não contém a página e recusa iniciar com a variável em `1`.

### Limites de conexão e de decisão

| Limite | Valor | Onde vive |
| --- | --- | --- |
| Conexões simultâneas | 1024; as seguintes esperam na fila do sistema | `src/server.rs` |
| Cabeçalhos e conexão ociosa | 30 segundos; depois a conexão fecha | `src/server.rs` |
| Duração de uma decisão, com a chegada do corpo | 5 segundos; depois 504 | `src/runtime.rs` |
| Decisões simultâneas | 256; acima disso 503 | `src/runtime.rs`, `src/state.rs` |
| Encerramento | Ctrl+C ou SIGTERM. Conexões em curso terminam. Corte em 10 segundos | `src/server.rs` |

Esses tetos limitam o processo. Não impedem um único cliente de ocupar todas as conexões. Limite por origem é papel do gateway. A API não confia em `X-Forwarded-For`. O TLS termina no gateway.

Se a requisição chegar com dois cabeçalhos `x-voidgate-key`, vale o primeiro. O gateway apaga qualquer chave vinda do cliente antes de chamar a API.

### Erros

Todo erro sai assim, com código e mensagem fixos. A mensagem não cita o campo, não repete o que foi enviado e não diz qual regra falhou.

```json
{ "error": { "code": "invalid_body", "message": "Request body does not match the documented contract." } }
```

| Status | `code` | Quando |
| --- | --- | --- |
| 401 | `unauthorized` | Chave ausente ou inválida |
| 400 | `invalid_json` | O corpo não é JSON válido |
| 400 | `invalid_body` | Campo ausente, extra ou fora do contrato |
| 413 | `payload_too_large` | Corpo ou `content` acima do limite |
| 415 | `unsupported_media_type` | Sem `Content-Type: application/json` |
| 429 | `rate_limited` | Cota por minuto esgotada; traz `Retry-After` |
| 503 | `unavailable` | 256 decisões em andamento, ou limitador indisponível |
| 504 | `decision_timeout` | A decisão passou de 5 segundos |
| 404 | `not_found` | Rota desconhecida, só com chave válida |
| 405 | `method_not_allowed` | Método recusado pela rota |

Sem chave válida, toda rota que não seja de saúde devolve o mesmo 401, exista a rota ou não, com qualquer método ou corpo. Quem não tem a chave não mapeia o serviço pelo tipo de erro.

### Saúde e métricas

| Rota | O que responde |
| --- | --- |
| `GET /health` | O processo está ativo. |
| `GET /live` | O processo está ativo. |
| `GET /ready` | O estado em memória do limitador está disponível. |
| `GET /metrics` | Exige a chave. Só contadores agregados, sem conteúdo, caminho ou credencial. |
| `GET /v1/events` | Exige a chave. As decisões recentes, só metadados. Veja “Eventos de decisão”. |

Não há dependência externa configurada nesta etapa.

### Eventos de decisão

`GET /v1/events` devolve os eventos dos últimos 30 minutos, no máximo 1000 por página, do mais antigo para o mais novo. A resposta inclui `events`, `next_cursor`, `has_more`, `gap` e `reset`. Envie `next_cursor` como `?cursor=` na leitura seguinte; enquanto `has_more` for verdadeiro, continue lendo. `gap` informa perda de eventos e `reset` informa que o cursor veio de outra instância ou de antes de um reinício. Cada evento tem `id`, `timestamp_ms`, `route` (`decisions_chat` ou `proxy_chat`), `direction` (`input` ou `output`), `action` e `reason_codes`. Não há texto, trecho, posição, pontuação nem regra. A resposta leva `Cache-Control: no-store`. A rota exige a chave, não consome o limite de decisões e não deve ser chamada do navegador.

Em `POST /v1/decisions/chat`, o papel `assistant` gera `output`; `user` e `system` geram `input`. Em `POST /v1/proxy/chat` há um evento `input` por requisição, com a pior ação entre as mensagens, assim que a decisão sai, e um `output` só se o modelo respondeu. Erro de validação não gera evento.

Os eventos ficam na memória do processo, até 10 000, e se perdem ao reiniciar. Cada réplica tem os seus. Se o registro estiver quebrado, a leitura responde 503. A decisão não muda: a falha de gravação é contada em `voidgate_event_record_errors_total`, em `GET /metrics`.

### O que esta API não é

A API 0.1.0 não é firewall de rede. Não substitui o login da aplicação nem um WAF. Não tem limite de taxa distribuído, identidade por empresa, réplicas coordenadas, isolamento da isca nem TLS dentro do processo. A decisão `allow` ou `decoy` só vale quando o gateway a aplica. O bind fica em loopback, ou a API fica atrás de um gateway com TLS e controle de acesso.

<!-- página -->

## 6. Arquitetura da API

A crate `voidgate` está na versão 0.1.0, edition 2021, e não é publicada no registry. O código fica em `src/`. As dependências de runtime são Axum 0.8, Hyper, Tokio, Serde, SHA-2 e subtle. As versões ficam presas em `Cargo.lock`. O comando de build usa `--locked`.

O código segue arquitetura em camadas. Cada camada chama só a de baixo:

```text
requisição → auth → handlers → services → domain
```

| Camada | Pasta | Faz | Fica de fora |
| --- | --- | --- | --- |
| Apresentação | `handlers/` | Lê o HTTP, chama o service, devolve JSON | Regra de negócio |
| Aplicação | `services/` | Valida a entrada e monta a resposta | HTTP e Axum |
| Domínio | `domain/` | Decide caminho de isca e conteúdo | JSON, rotas e configuração |

Três peças atravessam as camadas. `models/` guarda os corpos, sem lógica. `error.rs` define `AppError`, o único formato de erro. `state.rs` guarda o `AppState`, lido na partida e só de leitura. Não há banco. O estado vive em memória.

| Mudança | Onde entra |
| --- | --- |
| Rota nova | Handler, service, modelos e o roteador em `lib.rs` |
| Detector novo | Só em `domain/`. Entra na lista `default_detectors()` |
| Erro novo | Uma variante de `AppError` |

O caminho de isca está em `domain/policy.rs`. Cada detector implementa o trait `Detector`. O detector desta versão está em `domain/detectors/invisible_unicode.rs`.

A especificação OpenAPI é escrita à mão em `src/docs/openapi.json`. Um teste falha se uma rota atual sair do arquivo, ou se os códigos de erro da especificação e do código divergirem. Uma rota nova sem documentação não é detectada por esse teste. Quem cria rota atualiza o arquivo.

Verificação local:

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo audit
```

A integração contínua roda em Ubuntu formatação, Clippy, testes e auditoria de dependências. Em Windows roda os testes.

Mapa das pastas:

```text
src/
├── main.rs          lê .env e ambiente, sobe o servidor
├── lib.rs           rotas, autenticação, limite de corpo
├── server.rs        conexões, tempos e encerramento
├── rate_limit.rs    cota por credencial
├── runtime.rs       decisões simultâneas e tempo máximo
├── metrics.rs       contadores de /metrics
├── state.rs         AppState
├── auth.rs          x-voidgate-key
├── error.rs         AppError
├── handlers/        HTTP
├── services/        validação e resposta
├── models/          DTOs
├── domain/          policy.rs e detectors/
└── docs/            Swagger e openapi.json
```

<!-- página -->

## 7. O site de apresentação

O site explica o VoidGate para um público de negócios. É um aplicativo React 19 com TypeScript e Vite, estilo em CSS puro, sem biblioteca de componentes. As fontes vêm do Fontsource e são servidas pelo próprio site: Bricolage Grotesque nos títulos, Instrument Sans no texto, JetBrains Mono no código. Não há chamada ao Google Fonts. O tema é escuro. Animações param quando o sistema pede para reduzir movimento. A versão do pacote é 0.0.0. O desenvolvimento pede Node 20 ou superior.

A página, de cima para baixo:

| Seção | Função |
| --- | --- |
| Abertura | “O firewall que fala a língua da IA.” |
| Por que agora | OWASP, multa da LGPD e português |
| Tente enganar a IA | Classificação no navegador |
| Comparação | A mesma conversa com e sem a camada |
| Como funciona | As quatro etapas e três tipos de modelo |
| Quiz | “Você consegue ver o ataque?” |
| Para quem | Startups, setor público e desenvolvedores |
| Planos | Comunidade, nuvem, empresas e governo |

“Como funciona” desenha aplicação, VoidGate e modelo. O desenho não muda com o fornecedor. O `POST /v1/proxy/chat` é esse lugar: o chatbot aponta para o VoidGate e o endereço do modelo fica em `VOIDGATE_UPSTREAM_URL`.

### A demonstração no navegador

“Tente enganar a IA” classifica o texto em `src/detector/`, no próprio navegador. O veredito é `allow`, `mask` ou `block`. Nada segue para a API.

As regras simplificadas cobrem quatro classes:

| Classe | O que a demo procura |
| --- | --- |
| Manipulação | Pedido para ignorar instruções, assumir papel privilegiado, revelar o prompt do sistema, ou citar modo desenvolvedor. Português e inglês. |
| Invisíveis | Um conjunto pequeno de caracteres de largura zero |
| CPF | Número com dígito verificador |
| E-mail | Padrão comum de endereço |

O texto é comparado em minúsculas, sem acento e sem esses invisíveis. O resultado mostrado ao visitante é uma vitrine. O motor que bloqueia Trojan Source, tag characters e o restante da tabela da seção 4 está só na API.

### Publicação e testes

O workflow `deploy-pages.yml` publica o site no GitHub Pages a cada push em `main` ou `develop`, ou manualmente pela aba Actions. O endereço fica `https://<dono>.github.io/VoidGate/`.

Em desenvolvimento, o Vite repassa `/api/*` para `http://localhost:8080`, endereço trocável por `VITE_API_URL`. O site ainda não usa esse proxy.

| Comando | Efeito |
| --- | --- |
| `npm run dev` | Servidor local, porta 5173 |
| `npm run build` | Build de produção em `dist/` |
| `npm run lint` | oxlint |
| `npm test` | Testes unitários do detector e do PR Guard, com `node --test` |
| `npm run test:e2e` | Cypress de ponta a ponta; o Vite sobe sozinho |

<!-- página -->

## 8. Modelo de negócio

O núcleo é aberto e gratuito. Quem opera o próprio servidor usa o código. Quem não quer operar servidor paga o serviço. Contrato cobre instalação local e regra sob medida.

| Plano | Preço | O que inclui | Situação nesta versão |
| --- | --- | --- | --- |
| Comunidade | Grátis | Código aberto, execução no servidor do cliente, regras públicas e auditáveis | O código dos dois repositórios está público |
| Nuvem | Por volume de mensagens | Serviço gerenciado, painel do que foi bloqueado, relatório para a LGPD | Ainda não existe no código |
| Empresas e governo | Sob consulta | Instalação no ambiente do cliente, regras do setor, SLA e suporte dedicado | SLA numérico não definido nesta versão |

Não há tabela de preço, franquia de mensagens nem nome de cliente piloto nesta versão.

A chamada comercial atual procura empresas para o piloto e parceiros para crescer. O contato público é o GitHub da organização, [https://github.com/VoidGates](https://github.com/VoidGates).

<!-- página -->

## 9. Como colocar no ar

Esta seção fala com quem integra.

Você precisa de Rust estável, Cargo e uma chave aleatória de 32 a 128 bytes. As dependências saem do `Cargo.lock`. Use `--locked`.

### Desenvolvimento

Copie `.env.example` para `.env` na pasta de onde você vai rodar o processo. Preencha `VOIDGATE_API_KEY`. O arquivo está no `.gitignore`. Não o versione.

O leitor aceita só linhas `NOME=valor`. Sem aspas, sem `export`, sem comentário na mesma linha. Salve em UTF-8. Um `.env` em UTF-16, comum quando o PowerShell grava o arquivo, impede a partida.

```text
cargo run --locked
```

Quando a configuração fecha, o processo imprime `voidgate listening on http://<endereço>`.

Para gerar a chave no PowerShell e subir sem arquivo:

```powershell
$rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
$bytes = New-Object byte[] 32
$rng.GetBytes($bytes)
$env:VOIDGATE_API_KEY = [Convert]::ToBase64String($bytes)
$rng.Dispose()
$env:VOIDGATE_DECOY_PATHS = '/internal/backup'
cargo run --locked
```

No Windows PowerShell 5.1, envie o JSON de chat como bytes UTF-8. String com acento em ANSI volta 400.

### Produção

Configure pelo ambiente do processo. Mantenha `VOIDGATE_BIND` em loopback ou coloque a API atrás de um gateway com TLS. Não ligue `VOIDGATE_DEV_DOCS`.

O que o seu gateway faz:

1. Aplica a decisão e falha fechado. Resposta diferente de 200 é bloqueio.
2. Normaliza o caminho antes de `POST /v1/decisions/http`.
3. Remove `x-voidgate-key` vindo do cliente. Dois cabeçalhos: vale o primeiro.
4. Guarda `reason_code`, `reason_codes`, `findings` e o corpo de erro. O cliente final recebe texto genérico.
5. Limita conexões por origem e termina o TLS.
6. Tolera conexão fechada. Ociosa após 30 segundos. Depois de um 401 o servidor pode fechar sem ler o corpo.

### Documentação interativa

Só na sua máquina, em build de desenvolvimento:

```powershell
$env:VOIDGATE_DEV_DOCS = '1'
cargo run --locked
```

Abra `http://127.0.0.1:8080/docs`, clique em Authorize e cole o valor de `VOIDGATE_API_KEY`. O arquivo `/docs/openapi.json` importa no Postman, no Insomnia e no Bruno. Não publique essa tela.

<!-- página -->

## 10. Limites conhecidos

Três listas. Nenhuma é defeito de redação: cada uma diz o que um leitor apressado pode tomar por pronto.

### Do serviço 0.1.0

- O proxy só aceita `messages[].content` em texto. Conteúdo multimodal é recusado.
- A lista de idiomas e de documentos não cobre todos os países. O código marca onde acrescentar o próximo.
- Não há várias empresas na mesma instalação.
- O limite de taxa não é global entre réplicas.
- Não há TLS dentro do processo.
- O caminho isca é declarado. O ambiente isca não é executado.
- Existe uma chave só.

### Da demonstração no navegador

A vitrine não é o produto. Estes pontos estão no relatório de QA do front-end, de 16/11/2026:

- Quando um achado de manipulação e um caractere invisível se sobrepõem, o destaque pode imprimir o mesmo trecho duas vezes.
- Uma quebra de linha no meio de uma frase de manipulação pode passar como liberada.
- CPF colado a um e-mail pode corromper o texto mascarado.
- CPF escrito com espaços pode não ser reconhecido.
- O conjunto de invisíveis da demo é menor que o da API. Um texto que a API bloqueia pode aparecer liberado no site.

São falhas da vitrine. O motor em Rust que implementa a tabela da seção 4 não passa por esse código.

### Da promessa da página

A seção “Como funciona” descreve esconder dados pessoais e conferir a resposta do modelo. O proxy faz isso quando `VOIDGATE_UPSTREAM_URL` está definido: mascara na ida, encaminha, e mascara a resposta. Sem essa variável, a API só devolve a decisão.

<!-- página -->

## 11. Fecho

O VoidGate é a camada que decide antes do modelo. Essa decisão cobre texto oculto, manipulação e dados pessoais, e o proxy aplica a decisão quando o endereço do modelo aponta para ele. O passo seguinte é o painel e o relatório, para a operação deixar de depender só da API.
