<p align="center">
  <a href="https://voidgates.github.io/VoidGate/">
    <img src="VoidGate.jpg" alt="VoidGate" width="480">
  </a>
</p>

<p align="center">
  Camada opcional de segurança entre a aplicação e qualquer modelo de IA.
</p>

<p align="center">
  <a href="https://voidgates.github.io/VoidGate/">Página principal</a>
  ·
  <a href="docs/voidgate-documentacao.md">Documentação</a>
  ·
  <a href="docs/referencia.md">Referência da API</a>
  ·
  <a href="CHANGELOG.md">Changelog</a>
  ·
  <a href="CONTRIBUTING.md">Contribuir</a>
</p>

A API decide se uma mensagem de chat pode seguir, mascara dado pessoal ou bloqueia. `POST /v1/proxy/chat` encaminha ao modelo depois dessa decisão. O site em `web/` demonstra o produto no navegador e não chama a API.

## Documentação

| Guia | O que tem |
| --- | --- |
| [Visão do produto](docs/voidgate-documentacao.md) | O problema, o que a API faz e o que o site só simula. |
| [Referência](docs/referencia.md) | Configuração, contrato das rotas, erros e arquitetura. |
| [Arquitetura](docs/referencia.md#arquitetura) | Camadas `handlers`, `services` e `domain`. |
| [Site](web/README.md) | A demonstração e o que ela não faz. |
| [Changelog](CHANGELOG.md) | O que entrou em cada versão. |

Cada pasta do repositório tem um `README.md` com o que ela cuida. O texto de uma pasta aparece quando essa pasta está aberta. A entrada do projeto é este arquivo.

## Começar

Rust estável com Cargo. A chave `VOIDGATE_API_KEY` tem de 32 a 128 bytes, sem espaços nas pontas e sem caracteres de controle.

```powershell
Copy-Item .env.example .env
cargo run --locked
```

Preencha a chave no `.env` antes de subir. O processo escuta em `http://127.0.0.1:8080` e imprime `voidgate listening on http://<endereço>`. A tabela de variáveis, os erros e um exemplo de cada rota estão na [referência](docs/referencia.md).

## O repositório

| Pasta | O que é |
| --- | --- |
| `src/` | API. A entrada HTTP fica em `handlers`, a regra em `services` e a decisão pura em `domain`. |
| `web/` | Site. A demonstração roda no navegador. |
| `tests/` | Suíte da API pública e da demonstração. |
| `docs/` | Documentação do produto e a referência da API. |
| `.github/` | Integração contínua e o corte de versão. |

## Versão

A API e o site usam o mesmo número. Na `main`, um commit `feat:` sobe o minor, `fix:` sobe o patch, e `feat!:`, `fix!:` ou `BREAKING CHANGE` sobe o major. O número fica em `Cargo.toml`, `web/package.json` e `CHANGELOG.md`. A tag é `vX.Y.Z`.

## Contribuir

O caminho de uma mudança, o formato do commit e os comandos de verificação estão em [CONTRIBUTING.md](CONTRIBUTING.md). Um relato de vulnerabilidade fica fora da issue pública: [SECURITY.md](SECURITY.md).
