# Testes do detector

`detector.test.ts` cobre pedido comum, manipulação, CPF, e-mail, telefone, cartão, IBAN e os casos em que um achado não pode estragar o outro.

`dashboard.test.ts` cobre a regra de situação por tipo, os totais, as ameaças por minuto, as bloqueadas recentes, a lista de eventos com filtro, a faixa de risco e a simulação do painel: mesma semente, mesma história, e nenhum trecho com CPF, cartão ou e-mail sem máscara.

`api.test.ts` cobre a leitura de `GET /v1/events`: tradução dos motivos, evento só com metadados, eventos malformados pulados, campos extras descartados, resposta inválida recusada e quais tipos de ameaça a API monitora.
