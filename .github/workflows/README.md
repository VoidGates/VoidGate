# Workflows

| Arquivo | O que cuida |
| --- | --- |
| `rust.yml` | Formato, clippy, testes Rust e o teste do PR Guard. |
| `web.yml` | Lint, testes e build do site. Não publica. |
| `deploy-pages.yml` | Publica `web/` no GitHub Pages. |
| `pr-guard.yml` | Lê o diff do pull request com o script de `scripts/`. O código do pull request não é executado. |
