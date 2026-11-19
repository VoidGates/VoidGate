# Detectores

Cada um olha o texto e devolve posições em bytes. Nenhum devolve o texto.

| Arquivo | Código | Efeito |
| --- | --- | --- |
| `invisible_unicode.rs` | `content.invisible_unicode` | Bloqueia caractere que a pessoa não vê. |
| `manipulation.rs` | `content.prompt_injection` | Bloqueia frase de manipulação e token de controle de modelo. A lista é fechada, não um classificador. |
| `personal_data.rs` | `content.personal_data` | Mascara e-mail, telefone E.164, cartão, IBAN, CPF, CNPJ e SSN. A lista de países do IBAN é incompleta. |

Um detector novo entra em `default_detectors()`, neste diretório.
