# Relatório de QA do front-end

Avaliação do site de apresentação do VoidGate feita em 16/11/2026, na branch
`feat/pr-guard`. Cada item diz onde o problema aparece na tela, como reproduzir,
a causa no código e a melhoria sugerida.

## Resumo

| # | Problema | Tela | Gravidade | Teste que cobre |
|---|---|---|---|---|
| 1 | Texto duplicado no destaque | Tente enganar a IA | Alta | `cypress/e2e/site.cy.ts:97` (`skip`) |
| 2 | Quebra de linha fura o bloqueio | Tente enganar a IA | Alta | `tests/unit/detector.test.ts:113` (`todo`) |
| 3 | CPF dentro de e-mail corrompe a saída | Tente enganar a IA | Média | `tests/unit/detector.test.ts:117` (`todo`) |
| 4 | CPF com espaços não é detectado | Tente enganar a IA | Média | `tests/unit/detector.test.ts:130` (`todo`) |
| 5 | Caractere invisível: a demo libera, o backend bloqueia | Tente enganar a IA | Média | sem teste |
| 6 | A demo conhece poucos caracteres invisíveis | Tente enganar a IA | Média | sem teste |
| 7 | A página promete defesas que o backend ainda não tem | Tente enganar a IA, Como funciona | Média | sem teste |
| 8 | Proxy `/api` sem uso | Configuração | Baixa | sem teste |
| 9 | README pede Node 20+, os testes precisam de 22.18+ | README | Baixa | sem teste |

Gravidade alta: o visitante vê o erro, ou um ataque simples passa na demo.

Os itens 1 a 4 já têm teste escrito, marcado como bug conhecido. Ao corrigir,
tire o `todo` (unitário) ou o `.skip` (Cypress) e o teste passa a proteger a
correção.

## Como rodar os testes

```bash
npm test                # unitários do detector e do PR Guard
npm run test:e2e        # Cypress, sobe o Vite sozinho
npm run test:e2e:open   # Cypress em modo interativo
```

---

## 1. Texto duplicado no destaque

**Tela:** seção "Tente enganar a IA" (`#teste`), bloco "Como o VoidGate leu".

**Como reproduzir:** clique no exemplo "Texto escondido".

**Resultado atual:** o trecho "e as instruções" aparece duas vezes, uma dentro
do destaque vermelho e outra logo depois dos pontos que marcam os caracteres
invisíveis.

**Esperado:** o texto aparece uma vez só, igual ao que foi digitado.

**Causa:** o detector devolve achados que se sobrepõem. A frase de manipulação é
um achado que vai do "i" até "instruções", e cada caractere invisível dentro
dela é outro achado. `mergeOverlaps` (`src/detector/analyze.ts:174`) só junta
achados do mesmo tipo. O componente `Highlighted`
(`src/components/TryAttack.tsx:43`) assume que os achados vêm em sequência: na
linha 55 ele faz `cursor = finding.end`, o cursor volta para trás e o texto
seguinte é impresso de novo.

**Melhoria:** fazer o `Highlighted` ignorar o achado que começa antes do cursor.
O painel da direita continua explicando os dois motivos (manipulação e
caracteres invisíveis), que é o objetivo desse exemplo.

```tsx
findings.forEach((finding, index) => {
  if (finding.start < cursor) return // já coberto por um achado anterior
  ...
})
```

Com essa escolha, sobreposição entre tipos diferentes passa a ser permitida no
detector. O teste `tests/unit/detector.test.ts:122` afirma o contrário e deve
ser removido; o teste do Cypress passa a cobrir o comportamento.

## 2. Quebra de linha fura o bloqueio

**Tela:** "Tente enganar a IA", campo de mensagem.

**Como reproduzir:** digite `ignore`, aperte Enter e digite `as instruções`.

**Resultado atual:** "Liberado", sem nenhum achado.

**Esperado:** "Bloqueado", como acontece com a mesma frase em uma linha só.

**Causa:** as regras de manipulação (`src/detector/analyze.ts:32` a `57`) usam
`.{0,30}` entre o verbo e o alvo. Em expressão regular, `.` não casa com quebra
de linha.

**Melhoria:** acrescentar a flag `s` às seis regras (`/gs` no lugar de `/g`).
Com ela, `.` passa a casar também com quebra de linha.

## 3. CPF dentro de e-mail corrompe a saída

**Tela:** "Tente enganar a IA", bloco "O que a IA receberia".

**Como reproduzir:** digite `52998224725@email.com`.

**Resultado atual:** `5***@email.comcom`.

**Esperado:** `5***@email.com`.

**Causa:** `findPersonalData` (`src/detector/analyze.ts:126`) encontra um CPF e
um e-mail no mesmo trecho. `applyMasks` (linha 156) aplica as duas máscaras
usando as posições do texto original, e a segunda troca escreve em cima da
primeira.

**Melhoria:** em `findPersonalData`, procurar os e-mails primeiro e descartar o
CPF que estiver dentro de um e-mail já encontrado.

## 4. CPF com espaços não é detectado

**Tela:** "Tente enganar a IA".

**Como reproduzir:** digite `meu cpf 529 982 247 25`.

**Resultado atual:** "Liberado"; o CPF chega inteiro em "O que a IA receberia".

**Esperado:** "Liberado com dados escondidos" e `***.***.***-25`.

**Causa:** `CPF_PATTERN` (`src/detector/analyze.ts:67`) só aceita ponto e hífen
como separadores.

**Melhoria:** aceitar espaço como separador. O dígito verificador continua
evitando falso alarme com números comuns.

```ts
const CPF_PATTERN = /(?<!\d)(\d{3})[.\s]?(\d{3})[.\s]?(\d{3})[-\s]?(\d{2})(?!\d)/g
```

## 5. Caractere invisível: a demo libera, o backend bloqueia

**Tela:** "Tente enganar a IA", painel do veredito.

**Como reproduzir:** cole um texto comum com um caractere de largura zero
(U+200B) no meio.

**Resultado atual:** "Liberado com dados escondidos", com a explicação
"Eles seriam removidos" (`src/detector/analyze.ts:119`).

**No backend:** `POST /v1/decisions/chat` responde `block` para qualquer
caractere oculto fora das exceções documentadas. Não existe ação de remover.

**Causa:** decisão proposital na demo (`src/detector/analyze.ts:201`), tomada
antes de o backend definir a regra.

**Melhoria:** o time precisa escolher uma regra só. Se vale a do backend, a demo
deve bloquear e o texto da linha 119 muda. Se vale a da demo, o backend precisa
de uma ação de limpeza além de `allow` e `block`.

## 6. A demo conhece poucos caracteres invisíveis

**Tela:** "Tente enganar a IA".

**Como reproduzir:** cole um texto com U+202E (inversão de direção, usado no
ataque Trojan Source).

**Resultado atual:** "Liberado", sem achado.

**No backend:** bloqueado. A lista dele cobre largura zero, controles
bidirecionais, caracteres tag, preenchimentos em branco e caracteres de
controle (README do backend, seção `POST /v1/decisions/chat`).

**Causa:** `INVISIBLE_CHARS` (`src/detector/analyze.ts:66`) lista só U+200B a
U+200D, U+2060 e U+FEFF.

**Melhoria:** ampliar a expressão para os mesmos grupos do backend. As exceções
dele (emoji com U+200D, escrita árabe e persa) podem ficar de fora da demo, mas
então um emoji de família seria marcado; vale testar esse caso antes de
publicar.

## 7. A página promete defesas que o backend ainda não tem

**Tela:** "Tente enganar a IA" e "Como funciona".

| Defesa mostrada na página | Demo no navegador | Backend hoje |
|---|---|---|
| Bloquear manipulação de prompt | Sim | Não detecta |
| Esconder CPF | Sim | Não detecta nem redige |
| Esconder e-mail | Sim | Não detecta nem redige |
| Caracteres invisíveis | Remove | Bloqueia |

O aviso "Simulação no navegador, com regras simplificadas"
(`src/components/TryAttack.tsx:122`) já deixa claro que não é o motor real. O
risco é a apresentação mostrar como pronto o que o próprio README do backend
lista em "Limites desta etapa".

**Melhoria no front:** dizer na página o que já existe no produto e o que está
previsto.

**Melhoria no backend:** para cumprir o que a página mostra, faltam um detector
de manipulação de prompt, um detector de dados pessoais (CPF e e-mail) e uma
ação de mascarar na resposta de `/v1/decisions/chat`.

## 8. Proxy `/api` sem uso

**Ponto:** `vite.config.ts:11` a `17`.

**Situação:** o front não chama o backend em nenhum lugar; não há `fetch` no
código. O proxy apontava para a porta 8000 e o backend sobe na 8080. A porta já
foi corrigida.

**Por que a demo não pode chamar a API direto:** as rotas de decisão exigem a
chave `x-voidgate-key`. Uma chave colocada no código do navegador fica visível
para qualquer visitante. O backend também foi feito para ser chamado por um
gateway, não por uma página.

**Melhoria:** remover o proxy enquanto não houver uso. Para uma demo ao vivo, o
backend precisa de uma rota pública sem chave, com limite por IP, ou de um
gateway na frente. Essa é uma decisão do time.

## 9. README pede Node 20+

**Ponto:** `README.md:27`.

**Situação:** `npm test` roda arquivos TypeScript direto no Node, o que só
funciona a partir do Node 22.18. O CI já usa Node 22.

**Melhoria:** trocar o texto para "Requer Node 22.18+".

---

## O que foi verificado e está correto

- A página carrega sem erro no console.
- Os links do menu levam a seções que existem.
- Não há rolagem horizontal em tela de 375 px.
- HTML digitado no campo aparece como texto e não é executado.
- O quiz conta o placar certo, trava os botões depois da resposta e zera em
  "Jogar de novo".
- CPF com dígito verificador errado ou com dígitos repetidos não é mascarado.

## O que não foi testado

- Firefox e Safari (o Cypress rodou no Electron).
- Comparação visual de telas e leitor de tela.
- O backend em execução. A comparação dos itens 5 a 8 vem do README dele.
