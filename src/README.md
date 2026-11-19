# API

Crate `voidgate`. Sobe com `cargo run --locked` na raiz. A configuração exigida está em `.env.example` e na docstring de `main.rs`.

Cuida da decisão e, quando há um modelo configurado, do encaminhamento do chat. Não tem banco, painel nem TLS.

| Arquivo ou pasta | O que cuida |
| --- | --- |
| `main.rs` | Lê o ambiente e sobe o processo. |
| `lib.rs` | Monta as rotas, a autenticação e os limites de corpo. |
| `auth.rs` | Confere `x-voidgate-key`. |
| `server.rs` | Conexões, tempo ocioso e encerramento. |
| `runtime.rs` | No máximo 256 decisões ao mesmo tempo e 5 segundos cada. |
| `rate_limit.rs` | Pedidos por minuto da credencial. |
| `metrics.rs` | Contadores de `GET /metrics`. |
| `state.rs` | Configuração lida uma vez na partida. |
| `error.rs` | O único formato de erro. |
| `handlers/` | Entrada HTTP. |
| `services/` | Regra de cada rota. |
| `domain/` | Decisão pura. |
| `models/` | Corpos JSON. |
| `docs/` | Swagger e OpenAPI, só em desenvolvimento. |
