# Contribuir

O trabalho do dia entra na `develop`, com commit e merge direto. A `main` só muda por pull request, com a integração contínua verde.

## Onde a mudança entra

```text
requisição → auth → handlers → services → domain
```

| Mudança | Onde |
| --- | --- |
| Rota nova | `src/handlers/`, `src/services/`, os corpos em `src/models/` e a rota em `src/lib.rs`. Atualize `src/docs/openapi.json`. |
| Regra de decisão | `crates/voidgate-core/`. Um detector novo entra em `default_detectors()` e vale pro servidor e pro pacote Python. Caminho de isca fica em `src/domain/policy.rs`. |
| API do pacote Python | `bindings/python/`. Atualize o `__init__.pyi` junto. |
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

O `cargo test` compila o binding Python, então precisa de um Python 3.10+ instalado.

Pacote Python, num venv:

```text
pip install maturin pytest
maturin develop -m bindings/python/Cargo.toml
pytest bindings/python/tests
python bindings/python/examples/agente_simples.py
```

Na pasta `web/`: `npm test`. Na pasta `tests/`: `powershell -File .\run.ps1`.

O `.env` fica de fora do commit. A chave de exemplo em `.env.example` permanece vazia.

## Publicar no PyPI

O release cria a tag `vX.Y.Z` na `main`. A publicação do pacote Python é manual:

1. Actions → **PyPI** → *Run workflow*, escolha a tag e `target: testpypi`.
2. Confira num venv limpo: `pip install -i https://test.pypi.org/simple/ voidgate==X.Y.Z`.
3. Rode de novo com `target: pypi`.

Não há token: o PyPI confia no workflow `pypi.yml` e no environment de mesmo nome (`testpypi` ou `pypi`), cadastrados como *trusted publisher*. Versão publicada não pode ser republicada; para retirar, use *yank*.
