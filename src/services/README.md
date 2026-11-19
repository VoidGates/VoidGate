# Regras das rotas

Valida a entrada, pergunta ao domínio e monta a resposta. Não conhece axum.

| Arquivo | O que cuida |
| --- | --- |
| `http.rs` | Caminho comum ou isca. |
| `chat.rs` | `allow`, `mask` ou `block`. Bloqueio vence máscara. |
| `proxy.rs` | Recusa local, ou o texto com dado pessoal trocado por `[redacted]`. O conteúdo do chat precisa ser texto. |
