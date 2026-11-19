# Contribuir

O VoidGate aceita correção e função nova pelo fluxo normal do GitHub: um branch, um pull request para `main`, e a integração contínua verde.

## Onde a mudança entra

```text
requisição → auth → handlers → services → domain
```

| Mudança | Onde |
| --- | --- |
| Rota nova | `src/handlers/`, `src/services/`, os corpos em `src/models/` e a rota em `src/lib.rs`. Atualize `src/docs/openapi.json`. |
| Regra de decisão | `src/domain/`. Um detector novo entra em `default_detectors()`. |
| Erro novo | Uma variante em `src/error.rs`. |
| Texto ou layout do site | `web/`. |
| Comportamento que a API e o site prometem juntos | `tests/`. |

Uma camada só chama a que está logo abaixo. `handlers` não decide. `domain` não conhece HTTP.

## Commit

O título do commit decide a versão publicada na `main`:

| Título | Versão |
| --- | --- |
| `fix: ...` | patch, de `0.1.0` para `0.1.1` |
| `feat: ...` | minor, para `0.2.0` |
| `feat!: ...`, `fix!: ...` ou um corpo com `BREAKING CHANGE:` | major, para `1.0.0` |
| `docs:`, `chore:`, `ci:`, `test:`, `refactor:` | nenhuma |

O título começa com o tipo, em minúsculas. Um `feat:` no meio da frase não conta.

## Verificar

```text
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
```

Na pasta `web/`: `npm test`. Na pasta `tests/`: `powershell -File .\run.ps1`.

O `.env` fica de fora do commit. A chave de exemplo em `.env.example` permanece vazia.
