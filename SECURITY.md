# Política de segurança

O VoidGate é uma camada de segurança. Uma falha nele vira falha em quem confia nele, então relatos são tratados com prioridade sobre trabalho novo.

## Versões com correção

O projeto está antes da 1.0. Só a linha mais recente recebe correção de segurança.

| Versão | Recebe correção |
| --- | --- |
| 0.2.x | ✅ |
| < 0.2 | ❌ Atualize para a 0.2.x. |

Vale para os três artefatos, que têm o mesmo número de versão: o servidor (`voidgate`), a biblioteca Rust (`voidgate-core`) e o pacote Python (`voidgate`).

## Como relatar

**Não abra issue pública, PR nem discussão para vulnerabilidade.**

Relate em privado pelo GitHub: [**Report a vulnerability**](https://github.com/VoidGates/VoidGate/security/advisories/new).

Inclua:

- versão ou commit afetado, e qual artefato (servidor, `voidgate-core` ou pacote Python);
- passo a passo para reproduzir, com a menor entrada possível;
- o impacto: o que um atacante consegue e em que condição;
- se souber, uma sugestão de correção.

Não mande dado real: nem chave de API, nem mensagem de cliente, nem CPF de verdade. Use valores sintéticos, como o CPF de teste `529.982.247-25`.

## O que acontece depois

| Etapa | Meta |
| --- | --- |
| Confirmar o recebimento | até 3 dias úteis |
| Primeira avaliação (procede ou não, gravidade) | até 10 dias úteis |
| Correção de gravidade alta ou crítica | o quanto antes, com release `fix:` dedicado |

As metas são de esforço, não SLA contratual. Quem relatou acompanha tudo pelo próprio advisory. A divulgação é coordenada: o advisory se torna público junto com a versão corrigida, com crédito a quem relatou, se a pessoa quiser.

## Escopo

### É vulnerabilidade

| Categoria | Exemplo |
| --- | --- |
| Dado pessoal vazando apesar de `mask` | `redact`/proxy devolve o CPF que deveria ter virado `[redacted]` |
| Eco de conteúdo | Resposta de decisão, erro, log, `/metrics` ou `/v1/events` contendo o texto enviado |
| Burla de autenticação | Rota protegida respondendo sem `x-voidgate-key` válida, ou a chave vazando |
| Falha aberta | Erro, timeout ou entrada malformada resultando em `allow`, ou o proxy encaminhando mensagem bloqueada |
| Negação de serviço por entrada | Texto dentro dos limites que derruba o processo, trava a decisão ou estoura memória |
| Crash na biblioteca | Entrada que faz `voidgate-core` entrar em pânico ou o pacote Python derrubar o interpretador |
| Cadeia de suprimentos | Dependência comprometida, build que executa código inesperado, artefato publicado diferente do código |

### Não é vulnerabilidade (abra issue normal)

- **Frase de manipulação que passa.** A detecção é uma lista fechada de padrões, não um classificador. Um texto novo que escapa da lista é melhoria de cobertura, não falha de segurança. Abra issue com o texto. Se a forma de escapar for genérica (uma normalização que fura qualquer padrão), aí é vulnerabilidade.
- Tipo de documento ou país que ainda não é detectado.
- Ataque que depende de o integrador ignorar a [configuração recomendada](#como-usar-com-segurança): bind público sem gateway, tratar erro como `allow` etc.
- Divergência entre a demonstração do site (`web/`) e o motor. A demo roda regras simplificadas no navegador e não protege nada.
- Relatório de scanner automático sem prova de impacto.

## Como usar com segurança

Servidor:

- **Falhe fechado.** Qualquer resposta diferente de `200` (erro, timeout, conexão recusada) é bloqueio, nunca liberação.
- Mantenha `VOIDGATE_BIND` em loopback, ou coloque a API atrás de um gateway com TLS e controle de acesso. Não há TLS dentro do processo.
- Remova `x-voidgate-key` vindo do cliente final antes de repassar a requisição.
- Não exponha `reason_codes`, `findings` nem corpos de erro ao cliente final. Eles são para o seu gateway e os seus logs.
- Não ligue `VOIDGATE_DEV_DOCS` em produção.

Biblioteca (`voidgate-core` e pacote Python):

- Confira tudo que entra no modelo, não só a mensagem do usuário: resultado de ferramenta, página web, documento de RAG.
- Limite o tamanho da entrada antes de chamar. A biblioteca não impõe limite; o servidor usa 16 KiB por mensagem.
- Trate exceção ou erro na checagem como bloqueio.
- A regra só muda quando você atualiza o pacote. Acompanhe as versões.

## O que o projeto já faz

- A chave é comparada por hash SHA-256 em tempo constante (`subtle`).
- Findings carregam só offsets, nunca o texto. Há testes contra eco no chat, no proxy e nos eventos.
- Dependências travadas em `Cargo.lock` e `package-lock.json`. Build com `--locked` e `cargo audit` no CI.
- O PR Guard lê o diff de todo pull request e bloqueia segredo hardcoded, caractere invisível (Trojan Source), `build.rs`, dependência fora do registry oficial e script de instalação npm.

## Fora do alcance desta versão

Não há limite de taxa global entre réplicas, identidade por empresa nem TLS no processo. Esses limites estão documentados em [docs/referencia.md](docs/referencia.md#limites-desta-etapa) e não são tratados como vulnerabilidade enquanto continuarem documentados.
