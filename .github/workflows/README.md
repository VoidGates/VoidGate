# Workflows

| Arquivo | O que cuida |
| --- | --- |
| `rust.yml` | Formato, clippy, testes Rust, o teste do PR Guard e o teste do corte de versão. O job `python` gera o wheel e roda o pytest e o exemplo no Linux e no Windows, no Python 3.9 e 3.13. |
| `web.yml` | Lint, testes e build do site. Não publica. |
| `deploy-pages.yml` | Publica `web/` no GitHub Pages. |
| `pr-guard.yml` | Lê o diff do pull request com o script de `scripts/`. O código do pull request não é executado. |
| `release.yml` | Na `main`, sobe a versão quando o commit começa com `feat:` ou `fix:`. |
