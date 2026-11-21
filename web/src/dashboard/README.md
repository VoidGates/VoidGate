# Painel de monitoramento

`events.ts` define o formato de cada evento e gera eventos simulados: 30 minutos de histórico ao abrir e uma mensagem nova a cada 2 a 4 segundos, com ondas de ataque. `status.ts` calcula os totais e a situação de cada tipo de ameaça nos últimos 15 minutos (0 a 2 calmo, 3 a 9 atenção, 10 ou mais sob ataque). `status.ts` também agrupa as ameaças por minuto para o gráfico e separa as bloqueadas dos últimos 5 minutos, que se acumulam no portão do rio. Também monta a lista de ameaças recentes (as mensagens liberadas sem mudança ficam de fora) e classifica o risco: 0,75 ou mais é alto, 0,40 ou mais é médio. `useSimulatedEvents.ts` liga a simulação à tela e para tudo quando o painel está pausado.

Os dados são simulados e a tela diz isso. Quando a API registrar as decisões, só `events.ts` e o hook mudam: a tela continua igual. Os trechos guardados já vêm com dados pessoais escondidos.
