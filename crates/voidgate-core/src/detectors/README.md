# Detectores

Cada um olha o texto e devolve posições em bytes. Nenhum devolve o texto.

| Arquivo | Código | Efeito |
| --- | --- | --- |
| `invisible_unicode.rs` | `content.invisible_unicode` | Bloqueia caractere que a pessoa não vê. |
| `manipulation.rs` | `content.prompt_injection` | Bloqueia frase de manipulação e token de controle de modelo. A lista é fechada, não um classificador. Antes de comparar, normaliza fullwidth, letra matemática, homóglifo cirílico/grego, leetspeak e letra soletrada, então disfarce visual não fura a lista. |
| `personal_data.rs` | `content.personal_data` | Mascara e-mail, telefone E.164 e brasileiro (com parênteses ou hífen), cartão, IBAN, CPF, CNPJ e SSN. A lista de países do IBAN é incompleta. |

Cada achado declara a própria ação: `personal_data` pede `Mask`, os outros dois pedem `Block`. A decisão do texto é a pior ação entre os achados.

Um detector novo entra em `default_detectors()`, neste diretório, e declara a `action` de cada `Finding`.
