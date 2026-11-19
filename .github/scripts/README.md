# Scripts

`pr-guard.mjs` lê um diff e aponta chave, download executado no shell, pacote fora do registry e script de build. `pr-guard.test.mjs` confere esse leitor.

O workflow chama o script da branch padrão, não o script que veio no pull request.

`release.mjs` lê os commits desde a última tag `vX.Y.Z`. `feat:` sobe o minor, `fix:` sobe o patch, e `feat!:`, `fix!:` ou `BREAKING CHANGE` sobe o major. `release.test.mjs` confere essa conta.
