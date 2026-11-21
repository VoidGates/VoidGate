# Testes do detector

`detector.test.ts` cobre pedido comum, manipulação, CPF, e-mail, telefone, cartão, IBAN e os casos em que um achado não pode estragar o outro.

`dashboard.test.ts` cobre a regra de situação por tipo, os totais e a simulação do painel: mesma semente, mesma história, e nenhum trecho com CPF, cartão ou e-mail sem máscara.
