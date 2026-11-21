import { test } from 'node:test'
import assert from 'node:assert/strict'
import { categoryFor, isMonitored, parseEventsResponse, toPanelEvent } from '../../src/dashboard/api.ts'
import type { ApiEvent } from '../../src/dashboard/api.ts'

const base: ApiEvent = {
  id: 7,
  timestamp_ms: 1_795_000_000_000,
  route: 'proxy_chat',
  direction: 'input',
  action: 'block',
  reason_codes: ['content.prompt_injection'],
}

test('motivos viram categorias do painel', () => {
  assert.equal(categoryFor(['content.prompt_injection']), 'prompt_injection')
  assert.equal(categoryFor(['content.invisible_unicode']), 'prompt_injection')
  assert.equal(categoryFor(['content.personal_data']), 'sensitive_data')
  assert.equal(categoryFor([]), null)
  assert.equal(categoryFor(['content.unknown_future_code']), null)
})

test('com mais de um motivo, vale o mais grave', () => {
  assert.equal(categoryFor(['content.personal_data', 'content.prompt_injection']), 'prompt_injection')
  assert.equal(categoryFor(['content.personal_data', 'content.invisible_unicode']), 'prompt_injection')
})

test('evento da API vira evento do painel só com metadados', () => {
  const event = toPanelEvent(base)
  assert.deepEqual(event, {
    id: 'api_7',
    timestamp: 1_795_000_000_000,
    category: 'prompt_injection',
    action: 'block',
    direction: 'input',
    ruleId: 'content.prompt_injection',
    route: 'proxy_chat',
    reasonCodes: ['content.prompt_injection'],
  })
  for (const field of ['snippet', 'score', 'channel', 'file']) assert.ok(!(field in event), field)
})

test('liberado sem motivo fica sem categoria', () => {
  const event = toPanelEvent({ ...base, action: 'allow', reason_codes: [] })
  assert.deepEqual([event.category, event.ruleId], [null, 'none'])
})

test('resposta: lê a lista e pula eventos malformados', () => {
  const events = parseEventsResponse({
    events: [
      base,
      { ...base, id: 8, action: 'delete' },
      { ...base, id: 9, reason_codes: [1] },
      { ...base, id: 10, timestamp_ms: 'ontem' },
      null,
      { ...base, id: 11, direction: 'output', action: 'mask', reason_codes: ['content.personal_data'] },
    ],
  })
  assert.deepEqual(events.map((e) => e.id), ['api_7', 'api_11'])
})

test('resposta: campos extras são ignorados e nunca chegam à tela', () => {
  const [event] = parseEventsResponse({ events: [{ ...base, content: 'Meu CPF é 52998224725' }] })
  assert.ok(!JSON.stringify(event).includes('52998224725'))
})

test('resposta: corpo sem a lista de eventos é recusado', () => {
  assert.throws(() => parseEventsResponse([base]))
  assert.throws(() => parseEventsResponse({ error: { code: 'unauthorized' } }))
  assert.throws(() => parseEventsResponse(null))
})

test('fraude por arquivo só aparece como monitorada na simulação', () => {
  assert.equal(isMonitored('file_fraud', false), true)
  assert.equal(isMonitored('file_fraud', true), false)
  assert.equal(isMonitored('prompt_injection', true), true)
  assert.equal(isMonitored('sensitive_data', true), true)
})
