# VoidGate backend

O VoidGate é uma camada adicional e opcional de segurança para aplicações e fluxos de IA. Este repositório contém um esqueleto de API em Rust. Nesta etapa, ele **decide** se um caminho HTTP configurado deve ser encaminhado para uma isca e se uma mensagem de chat pode seguir; o gateway que integra a API precisa aplicar a decisão. O serviço ainda não recebe tráfego como proxy nem executa um ambiente-isca.

## Requisitos

- Rust estável com Cargo.
- Uma chave de serviço aleatória de 32 a 128 bytes, fornecida em `VOIDGATE_API_KEY`.

As versões das dependências estão fixadas em `Cargo.lock`. Use `--locked` para não atualizar esse arquivo sem perceber.

## Configuração e execução

Defina as variáveis no ambiente do processo antes de executar. O arquivo `.env.example` documenta os nomes, mas o programa **não carrega arquivos `.env` automaticamente**.

| Variável | Obrigatória | Uso |
| --- | --- | --- |
| `VOIDGATE_API_KEY` | Sim | Chave enviada no cabeçalho `x-voidgate-key` da rota de decisão. |
| `VOIDGATE_DECOY_PATHS` | Não | Caminhos exatos separados por vírgula; padrão: nenhum. |
| `VOIDGATE_BIND` | Não | Endereço de escuta; padrão: `127.0.0.1:8080`. |
| `VOIDGATE_RATE_LIMIT_PER_MINUTE` | Não | Capacidade local por credencial: 1 a 10.000; padrão: 60. |

Exemplo para gerar uma chave no PowerShell, sem gravá-la no repositório:

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

## API atual

- `GET /health` e `GET /live` indicam que o processo está ativo. `GET /ready` verifica se o estado em memória do limitador está disponível. Não há dependências externas configuradas nesta etapa.
- `POST /v1/decisions/http` exige `x-voidgate-key` e JSON com `method` e `path`. O corpo é limitado a 4 KiB.

Exemplo no PowerShell, em outro terminal que também tenha `VOIDGATE_API_KEY` definida:

```powershell
Invoke-RestMethod -Uri 'http://127.0.0.1:8080/v1/decisions/http' -Method Post -Headers @{ 'x-voidgate-key' = $env:VOIDGATE_API_KEY } -ContentType 'application/json' -Body '{"method":"GET","path":"/internal/backup"}'
```

Para um caminho configurado, a resposta é `{"action":"decoy","reason_code":"registered_decoy_path"}`. Para outro caminho válido, é `{"action":"allow","reason_code":"no_matching_rule"}`. Caminhos ambíguos ou codificados são rejeitados; a comparação dos caminhos de isca é exata.

As rotas `/v1/decisions/*` aplicam um limite por credencial autenticada, depois da autenticação e antes de ler o JSON. O serviço aceita uma única chave nesta versão, e o limitador mantém buckets em memória com capacidade limitada. O algoritmo permite uma rajada de até `VOIDGATE_RATE_LIMIT_PER_MINUTE` pedidos e repõe a capacidade gradualmente ao longo de um minuto. Ao exceder o limite, a API responde `429 Too Many Requests` com `Retry-After` em segundos. Requisições sem chave válida respondem `401` e não consomem essa capacidade. Configure no gateway o limite por IP para tentativas sem chave; esta aplicação não confia em `X-Forwarded-For`. As rotas de saúde permanecem acessíveis. Cada réplica mantém seus próprios buckets, portanto o limite ainda não é global. O serviço também impõe no máximo 256 decisões concorrentes e encerra decisões que excedam 5 segundos com `504`. `GET /metrics` exige a chave da API e expõe apenas contadores agregados, sem conteúdo, caminhos ou credenciais.

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

Respostas de erro: `401` sem chave válida (antes de ler o corpo), `400` para JSON inválido, `role` desconhecida, campo extra ou faltante, `413` para `content` acima de 16 KiB ou corpo acima de 32 KiB, `415` sem `Content-Type: application/json`.

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

Os detectores ficam em `src/detectors/`, cada um implementando o trait `Detector`; novos detectores entram na lista `default_detectors()` sem alterar o handler.

## Verificação

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

## Limites desta etapa

Este serviço não é um firewall de rede nem substitui autenticação da aplicação ou WAF; ele oferece decisões para o gateway consumidor aplicar. O limite continua local a cada réplica e ainda não existe um registro multi-chave/tenant. A decisão de chat nesta versão detecta apenas os caracteres Unicode invisíveis listados acima; não há redação, detecção de dados pessoais nem detecção geral de manipulação de prompt. O endpoint `/ready` verifica apenas a disponibilidade do estado local, pois ainda não há dependências externas. O processo encerra de forma graciosa ao receber Ctrl+C ou SIGTERM. Mantenha o bind local ou coloque a API atrás de um gateway confiável com TLS, limite por IP e controles de acesso. A decisão `allow` ou `decoy` só tem efeito quando esse gateway a aplica.
