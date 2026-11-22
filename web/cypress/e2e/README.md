# Cenários

`site.cy.ts` abre a página e confere o que o visitante vê, em português.

`painel.cy.ts` abre `#painel` com `GET /v1/events` mockado e confere o aviso de dados incompletos: aparece quando há perda ou o painel ficou para trás, e some quando a perda sai da janela de 30 minutos.
