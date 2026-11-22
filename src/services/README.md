# Regras das rotas

Valida a entrada, pergunta ao domínio e monta a resposta. Não conhece axum.

| Arquivo | O que cuida |
| --- | --- |
| `http.rs` | Caminho comum ou isca. |
| `chat.rs` | `allow`, `mask` ou `block`. Bloqueio vence máscara. `Verdict` junta a pior ação e os códigos de várias partes. |
| `proxy.rs` | Recusa local, ou o texto com dado pessoal trocado por `[redacted]`. O conteúdo do chat precisa ser texto. |
| `events.rs` | Grava o evento sem mudar a decisão: se falhar, só conta em `/metrics`. Lê os mais recentes. |
