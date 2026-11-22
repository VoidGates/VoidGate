// Painel em modo dev lendo GET /v1/events (mockado). Ver issue #10.
const page = (gap: boolean, hasMore = false) => ({
  events: [],
  next_cursor: 'abcdef0123456789:0',
  has_more: hasMore,
  gap,
  reset: false,
})
const badge = () => cy.get('.dash__demo')
const HALF_HOUR = 30 * 60 * 1000

describe('painel: aviso de dados incompletos', () => {
  it('sem perda mostra ao vivo', () => {
    cy.intercept('GET', '/api/v1/events*', page(false)).as('events')
    cy.visit('/#painel')
    cy.wait('@events')
    badge().should('have.class', 'dash__demo--api')
  })

  it('o aviso some quando a perda sai da janela de 30 min', () => {
    let gap = true
    cy.intercept('GET', '/api/v1/events*', (req) => req.reply(page(gap))).as('events')
    cy.clock(Date.now(), ['Date', 'setTimeout', 'clearTimeout'])
    cy.visit('/#painel')
    badge().should('have.class', 'dash__demo--partial').then(() => {
      gap = false
    })
    // Ainda dentro da janela: continua avisando.
    cy.tick(2_000)
    cy.wait('@events')
    badge().should('have.class', 'dash__demo--partial')
    cy.tick(HALF_HOUR)
    cy.wait('@events')
    badge().should('have.class', 'dash__demo--api')
  })

  it('12 páginas sem acabar avisa, e volta quando alcança', () => {
    let hasMore = true
    cy.intercept('GET', '/api/v1/events*', (req) => req.reply(page(false, hasMore))).as('events')
    cy.clock(Date.now(), ['Date', 'setTimeout', 'clearTimeout'])
    cy.visit('/#painel')
    badge().should('have.class', 'dash__demo--partial').then(() => {
      hasMore = false
    })
    cy.tick(2_000)
    cy.wait('@events')
    badge().should('have.class', 'dash__demo--api')
  })
})
