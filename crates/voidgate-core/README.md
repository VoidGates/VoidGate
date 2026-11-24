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
| `redact(text, &findings) -> String` | Troca por `[redacted]` todo achado com ação `Mask`. Ignora os outros. |
| `Action` | `Allow < Mask < Block`. `as_str()` dá `"allow"`/`"mask"`/`"block"`. |
| `Finding { code, detector, kind, action, start, end }` | `action` é o que o detector pede: `Mask` esconde o trecho, `Block` barra o texto. Offsets em **bytes** UTF-8. Nunca carrega o texto. |
| `Verdict` | Junta várias mensagens: pior ação + códigos sem repetir. |
| `Detector` | Trait pra detector próprio (`Send + Sync`). Veja abaixo. |

Detectores e códigos: [`src/detectors/`](src/detectors).

## Detector próprio

A decisão de um texto é a pior `action` entre os achados (`Allow` se não houver nenhum). O código do achado não entra na conta: quem decide é o detector, pela `action` que declara. Um detector de fora pode então mascarar, sem tocar no crate:

```rust
use voidgate_core::{inspect_with, redact, Action, Detector, Finding};

struct NomeInterno;

impl Detector for NomeInterno {
    fn scan(&self, texto: &str) -> Vec<Finding> {
        texto
            .match_indices("projeto-aurora")
            .map(|(start, achado)| Finding {
                code: "custom.nome_interno",
                detector: "nome_interno",
                kind: "nome_interno",
                action: Action::Mask,
                start,
                end: start + achado.len(),
            })
            .collect()
    }
}

let detectores: Vec<Box<dyn Detector>> = vec![Box::new(NomeInterno)];
let texto = "Novidade do projeto-aurora na sexta";
let r = inspect_with(&detectores, texto);
assert_eq!(r.action, Action::Mask);
assert_eq!(redact(texto, &r.findings), "Novidade do [redacted] na sexta");
```

Com `Action::Block` o texto é barrado e `redact` não mexe nele. Para juntar com os detectores embutidos, parta de `default_detectors()` e dê `push`.

## Limites

- Sem limite de tamanho: limite a entrada antes (o servidor usa 16 KiB por mensagem).
- Lista de frases fechada, não é classificador.
- Feature `serde`: deriva `Serialize` em `Action` (minúsculo) e `Finding`. É o que o servidor usa no JSON, que não leva `kind` nem `action`.

Usado por: servidor HTTP (`src/`) e pacote Python ([`bindings/python`](../../bindings/python)).
