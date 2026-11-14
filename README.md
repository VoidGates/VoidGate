# VoidGate backend

O VoidGate é uma camada adicional e opcional de segurança para aplicações e fluxos de IA. Este repositório contém um esqueleto de API em Rust. Nesta etapa, ele **decide** se um caminho HTTP configurado deve ser encaminhado para uma isca; o gateway que integra a API precisa aplicar a decisão. O serviço ainda não recebe tráfego como proxy nem executa um ambiente-isca.

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

- `GET /health`, `GET /live` e `GET /ready` retornam `{"status":"ok"}`. Nesta versão, `/ready` ainda não testa dependências externas.
- `POST /v1/decisions/http` exige `x-voidgate-key` e JSON com `method` e `path`. O corpo é limitado a 4 KiB.

Exemplo no PowerShell, em outro terminal que também tenha `VOIDGATE_API_KEY` definida:

```powershell
Invoke-RestMethod -Uri 'http://127.0.0.1:8080/v1/decisions/http' -Method Post -Headers @{ 'x-voidgate-key' = $env:VOIDGATE_API_KEY } -ContentType 'application/json' -Body '{"method":"GET","path":"/internal/backup"}'
```

Para um caminho configurado, a resposta é `{"action":"decoy","reason_code":"registered_decoy_path"}`. Para outro caminho válido, é `{"action":"allow","reason_code":"no_matching_rule"}`. Caminhos ambíguos ou codificados são rejeitados; a comparação dos caminhos de isca é exata.

## Verificação

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

## Limites desta etapa

Este serviço não é um firewall de rede nem substitui autenticação da aplicação ou WAF. Não há ainda rate limit distribuído, identidade por tenant, proxy de chat, réplicas, isolamento da isca ou TLS dentro do processo. Mantenha o bind local ou coloque a API atrás de um gateway confiável com TLS e controles de acesso. A decisão `allow` ou `decoy` só tem efeito quando esse gateway a aplica.
