# voidgate-core

Decide se um texto que vai pro modelo de IA (ou volta dele) passa, tem dado pessoal mascarado ou é bloqueado. Biblioteca pura: sem IO, sem thread, sem rede. Sem a feature `serde`, zero dependências.

```rust
use voidgate_core::{inspect, redact, Action};

let texto = "Meu CPF é 529.982.247-25";
let r = inspect(texto);
assert_eq!(r.action, Action::Mask);
assert_eq!(redact(texto, &r.findings), "Meu CPF é [redacted]");
```

## API

| Item | O quê |
| --- | --- |
| `inspect(text) -> Inspection` | Roda todos os detectores. `Inspection { action, findings }`. |
| `inspect_with(&detectors, text)` | Mesma coisa com uma lista de detectores sua. |
| `redact(text, &findings) -> String` | Troca dado pessoal por `[redacted]`. Ignora os outros achados. |
| `Action` | `Allow < Mask < Block`. `as_str()` dá `"allow"`/`"mask"`/`"block"`. |
| `Finding { code, detector, start, end }` | Offsets em **bytes** UTF-8. Nunca carrega o texto. |
| `Verdict` | Junta várias mensagens: pior ação + códigos sem repetir. |
| `Detector` | Trait pra detector próprio (`Send + Sync`). |

Detectores e códigos: [`src/detectors/`](src/detectors).

## Limites

- Sem limite de tamanho: limite a entrada antes (o servidor usa 16 KiB por mensagem).
- Lista de frases fechada, não é classificador.
- Feature `serde`: deriva `Serialize` em `Action` (minúsculo) e `Finding`. É o que o servidor usa no JSON.

Usado por: servidor HTTP (`src/`) e pacote Python ([`bindings/python`](../../bindings/python)).
