# Entrada HTTP

Recebe o pedido, chama o service e devolve JSON. Não decide sozinha.

| Arquivo | Rota |
| --- | --- |
| `health.rs` | `GET /health`, `/live` e `/ready`. Públicas. |
| `http.rs` | `POST /v1/decisions/http`. |
| `chat.rs` | `POST /v1/decisions/chat`. Não devolve o texto da mensagem. Registra o evento da decisão. |
| `proxy.rs` | `POST /v1/proxy/chat`. Encaminha ou recusa. Não repassa `x-voidgate-key`. Registra um evento de entrada e, se o modelo respondeu, um de saída. |
| `events.rs` | `GET /v1/events`. Só metadados, com `Cache-Control: no-store`. |
| `docs.rs` | `/docs`. Só entra no binário de desenvolvimento. |
