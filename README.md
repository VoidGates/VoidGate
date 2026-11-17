# VoidGate backend

O VoidGate é uma camada adicional e opcional de segurança para aplicações e fluxos de IA. Este repositório contém um esqueleto de API em Rust. Nesta etapa, ele **decide** se um caminho HTTP configurado deve ser encaminhado para uma isca e se uma mensagem de chat pode seguir; o gateway que integra a API precisa aplicar a decisão. O serviço ainda não recebe tráfego como proxy nem executa um ambiente-isca.

## Requisitos

- Rust estável com Cargo.
- Uma chave de serviço aleatória de 32 a 128 bytes, fornecida em `VOIDGATE_API_KEY`.

As versões das dependências estão fixadas em `Cargo.lock`. Use `--locked` para não atualizar esse arquivo sem perceber.

## Configuração e execução

Em desenvolvimento, copie `.env.example` para `.env`, preencha a chave e rode `cargo run --locked`. O programa lê o `.env` da pasta em que é executado; o arquivo está no `.gitignore` e nunca deve ser versionado. Só linhas `NOME=valor` simples são aceitas (sem aspas).

Uma variável definida no ambiente do processo sempre vence o `.env`. Em produção não há `.env`: configure pelo ambiente.

| Variável | Obrigatória | Uso |
| --- | --- | --- |
| `VOIDGATE_API_KEY` | Sim | Chave enviada no cabeçalho `x-voidgate-key` da rota de decisão. |
| `VOIDGATE_DECOY_PATHS` | Não | Caminhos exatos separados por vírgula; padrão: nenhum. |
| `VOIDGATE_BIND` | Não | Endereço de escuta; padrão: `127.0.0.1:8080`. |
| `VOIDGATE_DEV_DOCS` | Não | `1` liga a tela de testes em `/docs`. Só em desenvolvimento; padrão: desligada. |

Sem `.env`, exemplo para gerar uma chave no PowerShell e rodar só com variáveis de ambiente:

```powershell
$rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
$bytes = New-Object byte[] 32
$rng.GetBytes($bytes)
$env:VOIDGATE_API_KEY = [Convert]::ToBase64String($bytes)
$rng.Dispose()
$env:VOIDGATE_DECOY_PATHS = '/internal/backup'
cargo run --locked
```

Se a chave ou um caminho configurado for inválido, ou se a porta não puder ser aberta, o processo termina com erro. Os valores opcionais ausentes usam os padrões acima.

## Tela de testes (só em desenvolvimento)

Com `VOIDGATE_DEV_DOCS=1`, o serviço publica o Swagger UI em `http://127.0.0.1:8080/docs` e a especificação OpenAPI em `/docs/openapi.json`:

```powershell
$env:VOIDGATE_DEV_DOCS = '1'
cargo run --locked
```

Abra `/docs`, clique em **Authorize**, cole o valor de `VOIDGATE_API_KEY` e use **Try it out** em cada rota. O arquivo `/docs/openapi.json` também pode ser importado no Postman, Insomnia ou Bruno.

A tela descreve a API inteira, então ela não existe fora do desenvolvimento:

- Sem a variável, `/docs` responde como qualquer rota desconhecida.
- Com a variável e um `VOIDGATE_BIND` que não seja loopback, o processo se recusa a iniciar.
- A página carrega o Swagger UI 5.17.14 do cdnjs com hash de integridade e uma política CSP restrita; o navegador precisa de internet na primeira abertura.

A especificação é escrita à mão em `src/docs/openapi.json`. Ao criar ou alterar uma rota, atualize esse arquivo; um teste falha se alguma rota existente sair dele.

## Erros

Todo erro sai no mesmo formato, com código e mensagem fixos:

```json
{ "error": { "code": "invalid_body", "message": "Request body does not match the documented contract." } }
```

| Status | `code` | Quando |
| --- | --- | --- |
| 401 | `unauthorized` | Chave ausente ou inválida. |
| 400 | `invalid_json` | O corpo não é JSON válido. |
| 400 | `invalid_body` | Campo ausente, extra ou com valor fora do contrato. |
| 413 | `payload_too_large` | Corpo ou `content` acima do limite. |
| 415 | `unsupported_media_type` | Sem `Content-Type: application/json`. |
| 404 | `not_found` | Rota desconhecida (só com chave válida). |
| 405 | `method_not_allowed` | Método não aceito pela rota. |

O erro informa a categoria do problema, nunca a regra que falhou: não cita o campo, não repete o que foi enviado e não diz, por exemplo, qual caractere do caminho foi recusado. O contrato completo fica na documentação, que só quem desenvolve acessa.

Sem chave válida, toda rota fora as de saúde devolve o mesmo `401`, exista ela ou não, com qualquer método ou corpo. Quem não tem a chave não consegue mapear as rotas pelo tipo de erro.

As respostas desta API são para o gateway. Ele não deve repassar ao cliente final `reason_code`, `reason_codes`, `findings` nem o corpo de um erro: esses campos dizem qual defesa disparou. Para o cliente final, responda de forma genérica.

O módulo fica em `src/error.rs` (`AppError` e o extrator `ApiJson`); handlers e services novos devolvem `AppError` em vez de um status solto.

## Arquitetura

O projeto segue a **Arquitetura em Camadas** (em inglês, *Layered Architecture*). O código é dividido em camadas, cada uma com uma única responsabilidade, e uma camada só chama a que está logo abaixo dela:

```text
requisição → auth → handlers → services → domain
```

| Camada | Pasta | Responsabilidade | Não faz |
| --- | --- | --- | --- |
| Apresentação | `handlers/` | Recebe a requisição HTTP, chama o service e devolve JSON. | Não contém regra de negócio. |
| Aplicação | `services/` | Valida a entrada, consulta o domínio e monta a resposta. | Não conhece HTTP nem axum. |
| Domínio | `domain/` | Decide: caminho de isca e detectores de conteúdo. | Não conhece JSON, rotas nem configuração. |

Três partes são usadas por todas as camadas:

- `models/`: os corpos de requisição e resposta (DTOs), sem lógica.
- `error.rs`: o `AppError`, único formato de erro. Qualquer camada devolve um `AppError` e ele vira a resposta JSON em um só lugar.
- `state.rs`: o `AppState`, com a configuração lida na inicialização.

Em um sistema comum, a última camada seria a de dados (banco). Este serviço não tem banco: o estado é só leitura e vive em memória, então a camada mais interna é o domínio.

Na prática, isso define onde cada mudança entra:

- Rota nova: um handler em `handlers/`, um service em `services/`, os corpos em `models/` e a rota em `lib.rs`.
- Regra de decisão nova, como um detector: só em `domain/`.
- Erro novo: uma variante em `AppError`.

### Estrutura de pastas

```text
src/
├── main.rs        lê a configuração (.env e ambiente) e sobe o servidor
├── lib.rs         monta o roteador: rotas, autenticação, limites de corpo
├── state.rs       AppState: configuração compartilhada, criada na inicialização
├── auth.rs        middleware da chave x-voidgate-key
├── error.rs       AppError: o único formato de erro da API
├── handlers/      camada HTTP: extrai a requisição, chama o service, devolve JSON
├── services/      regras de negócio: valida a entrada e monta a resposta
├── models/        corpos de requisição e resposta (DTOs)
├── domain/        regras puras de decisão, sem HTTP
│   ├── policy.rs      caminhos de isca
│   └── detectors/     detectores de conteúdo
└── docs/          página do Swagger UI e openapi.json
```
## API atual

- `GET /health`, `GET /live` e `GET /ready` retornam `{"status":"ok"}`. Nesta versão, `/ready` ainda não testa dependências externas.
- `POST /v1/decisions/http` exige `x-voidgate-key` e JSON com `method` e `path`. O corpo é limitado a 4 KiB.

Exemplo no PowerShell, em outro terminal que também tenha `VOIDGATE_API_KEY` definida:

```powershell
Invoke-RestMethod -Uri 'http://127.0.0.1:8080/v1/decisions/http' -Method Post -Headers @{ 'x-voidgate-key' = $env:VOIDGATE_API_KEY } -ContentType 'application/json' -Body '{"method":"GET","path":"/internal/backup"}'
```

Para um caminho configurado, a resposta é `{"action":"decoy","reason_code":"registered_decoy_path"}`. Para outro caminho válido, é `{"action":"allow","reason_code":"no_matching_rule"}`. Caminhos ambíguos ou codificados são rejeitados; a comparação dos caminhos de isca é exata.

### `POST /v1/decisions/chat`

Decide se uma mensagem de chat pode seguir. Exige `x-voidgate-key` e JSON com exatamente dois campos:

| Campo | Valores |
| --- | --- |
| `role` | `"user"`, `"system"` ou `"assistant"` |
| `content` | texto UTF-8 válido de até 16 KiB |

O corpo desta rota é limitado a 32 KiB. O `content` é inspecionado em memória e descartado: ele **nunca** aparece na resposta nem em logs; a resposta só traz posições em bytes do texto original.

Resposta: `{"action":"allow"|"block","reason_codes":[...],"findings":[{"code","detector","start","end"}]}`. `start` e `end` são offsets em bytes (fim exclusivo) do `content` enviado. Sem ocorrências, `action` é `allow` e as duas listas vêm vazias.

Códigos de motivo (`reason_codes`) nesta versão:

| Código | Detector | Dispara quando |
| --- | --- | --- |
| `content.invisible_unicode` | `invisible_unicode` | O texto contém caracteres invisíveis ou de controle bidirecional (Trojan Source): U+200B–U+200F, U+2028–U+202E, U+2060–U+2064, U+2066–U+2069 ou U+FEFF. Qualquer ocorrência gera `block`. |

Respostas de erro, no formato da seção [Erros](#erros): `401` sem chave válida (antes de ler o corpo), `400` para JSON inválido, `role` desconhecida, campo extra ou faltante, `413` para `content` acima de 16 KiB ou corpo acima de 32 KiB, `415` sem `Content-Type: application/json`.

Exemplo no PowerShell. O corpo é enviado como bytes UTF-8 porque o Windows PowerShell 5.1 codifica strings com acento em ANSI, que o serviço rejeita com `400`:

```powershell
$body = [System.Text.Encoding]::UTF8.GetBytes('{"role":"user","content":"Olá, tudo bem?"}')
Invoke-RestMethod -Uri 'http://127.0.0.1:8080/v1/decisions/chat' -Method Post -Headers @{ 'x-voidgate-key' = $env:VOIDGATE_API_KEY } -ContentType 'application/json' -Body $body
```

Resposta: `{"action":"allow","reason_codes":[],"findings":[]}`. Com um U+202E (override da direita para a esquerda) após `Olá `:

```powershell
$body = [System.Text.Encoding]::UTF8.GetBytes('{"role":"user","content":"Olá ' + [char]0x202E + 'mundo"}')
Invoke-RestMethod -Uri 'http://127.0.0.1:8080/v1/decisions/chat' -Method Post -Headers @{ 'x-voidgate-key' = $env:VOIDGATE_API_KEY } -ContentType 'application/json' -Body $body | ConvertTo-Json -Depth 5 -Compress
```

Resposta: `{"action":"block","reason_codes":["content.invisible_unicode"],"findings":[{"code":"content.invisible_unicode","detector":"invisible_unicode","start":5,"end":8}]}` (`Olá ` ocupa 5 bytes e o U+202E ocupa 3).

Os detectores ficam em `src/domain/detectors/`, cada um implementando o trait `Detector`; novos detectores entram na lista `default_detectors()` sem alterar o handler nem o service.

## Verificação

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

## Limites desta etapa

Este serviço não é um firewall de rede nem substitui autenticação da aplicação ou WAF. Não há ainda rate limit distribuído, identidade por tenant, proxy de chat, réplicas, isolamento da isca ou TLS dentro do processo. A decisão de chat só detecta caracteres Unicode invisíveis; não há redação, detecção de dados pessoais nem de manipulação de prompt. Mantenha o bind local ou coloque a API atrás de um gateway confiável com TLS e controles de acesso. A decisão `allow` ou `decoy` só tem efeito quando esse gateway a aplica.
