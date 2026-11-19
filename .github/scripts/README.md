# PR Guard

`pr-guard.mjs` lê um diff e aponta chave, download executado no shell, pacote fora do registry e script de build. `pr-guard.test.mjs` confere esse leitor.

O workflow chama o script da branch padrão, não o script que veio no pull request.
