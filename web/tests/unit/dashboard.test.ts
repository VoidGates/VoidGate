import { test } from 'node:test'
import assert from 'node:assert/strict'
import { createSimulator, HISTORY_WINDOW, MINUTE } from '../../src/dashboard/events.ts'
import type { PanelEvent } from '../../src/dashboard/events.ts'
import { categoryStatus, latestThreats, levelFor, recentlyBlocked, riskFor, threatsPerMinute, totals } from '../../src/dashboard/status.ts'

const NOW = Date.UTC(2026, 10, 19, 15, 0, 0)

function event(minutesAgo: number, overrides: Partial<PanelEvent> = {}): PanelEvent {
  return {
    id: `evt_${minutesAgo}`,
    timestamp: NOW - minutesAgo * MINUTE,
    category: 'prompt_injection',
    action: 'block',
    direction: 'input',
    ruleId: 'ignore-instructions',
    score: 0.9,
    snippet: 'x',
    channel: 'Chatbot de atendimento',
    ...overrides,
  }
}

test('nível pela contagem dos últimos 15 min: 0–2 calmo, 3–9 atenção, 10+ sob ataque', () => {
  assert.deepEqual([0, 2, 3, 9, 10, 58].map(levelFor), ['calm', 'calm', 'attention', 'attention', 'attack', 'attack'])
})

test('status conta só o tipo pedido e só dentro da janela de 15 min', () => {
  const events = [
    event(1),
    event(5),
    event(14.5),
    event(16), // fora da janela
    event(2, { category: 'sensitive_data', action: 'mask', ruleId: 'cpf' }),
  ]
  const status = categoryStatus(events, 'prompt_injection', NOW)
  assert.equal(status.count, 3)
  assert.equal(status.level, 'attention')
  assert.equal(status.perMinute.length, 15)
  assert.equal(status.perMinute.reduce((a, b) => a + b, 0), 3)
  assert.equal(status.lastAt, NOW - 1 * MINUTE)
})

test('status traz o pico e a regra mais comum', () => {
  const events = [
    event(8, { category: 'sensitive_data', ruleId: 'cpf' }),
    event(7.9, { category: 'sensitive_data', ruleId: 'cpf' }),
    event(3, { category: 'sensitive_data', ruleId: 'email' }),
  ]
  const status = categoryStatus(events, 'sensitive_data', NOW)
  assert.equal(status.topRule, 'cpf')
  assert.equal(status.peakAt, NOW - 8 * MINUTE)
})

test('status vazio não inventa pico nem último evento', () => {
  const status = categoryStatus([], 'file_fraud', NOW)
  assert.deepEqual([status.count, status.level, status.peakAt, status.lastAt, status.topRule], [0, 'calm', null, null, null])
})

test('totais somam bloqueadas, escondidas e liberadas', () => {
  const events = [event(1), event(2, { action: 'mask' }), event(3, { category: null, action: 'allow' })]
  assert.deepEqual(totals(events), { analyzed: 3, blocked: 1, masked: 1, allowed: 1 })
})

test('simulação: mesma semente, mesma história, ordenada e dentro de 30 min', () => {
  const a = createSimulator(7).history(NOW)
  const b = createSimulator(7).history(NOW)
  assert.deepEqual(a, b)
  assert.ok(a.length > 300)
  for (let i = 0; i < a.length; i++) {
    assert.ok(a[i].timestamp >= NOW - HISTORY_WINDOW && a[i].timestamp < NOW)
    if (i > 0) assert.ok(a[i].timestamp > a[i - 1].timestamp)
  }
})

test('simulação: nenhum trecho traz CPF, cartão ou e-mail sem máscara', () => {
  const simulator = createSimulator(3)
  const events = [...simulator.history(NOW)]
  for (let i = 0; i < 2000; i++) events.push(simulator.next(NOW + i * 1000))
  const clear = [/\d{3}\.\d{3}\.\d{3}-\d{2}/, /\d{4} \d{4} \d{4} \d{4}/, /[a-z0-9._-]{2,}@/i]
  for (const e of events) {
    for (const pattern of clear) assert.doesNotMatch(e.snippet, pattern, e.snippet)
  }
})

test('simulação: os três tipos aparecem e quase tudo é liberado', () => {
  const events = createSimulator(7).history(NOW)
  const kinds = new Set(events.map((e) => e.category))
  for (const kind of ['prompt_injection', 'sensitive_data', 'file_fraud', null]) assert.ok(kinds.has(kind as never), String(kind))
  assert.ok(totals(events).allowed / events.length > 0.7)
})

test('ameaças por minuto: 30 colunas, por tipo, sem contar as liberadas', () => {
  const events = [
    event(0.5),
    event(0.2, { category: 'sensitive_data', action: 'mask' }),
    event(29.5, { category: 'file_fraud' }),
    event(31), // fora da janela
    event(1, { category: null, action: 'allow' }),
  ]
  const buckets = threatsPerMinute(events, NOW)
  assert.equal(buckets.length, 30)
  assert.equal(buckets[0].start, NOW - 30 * MINUTE)
  assert.deepEqual(
    [buckets[29].prompt_injection, buckets[29].sensitive_data, buckets[0].file_fraud],
    [1, 1, 1],
  )
  const total = buckets.reduce((sum, b) => sum + b.prompt_injection + b.sensitive_data + b.file_fraud, 0)
  assert.equal(total, 3)
})

test('bloqueadas nos últimos 5 min ignoram máscaras e eventos antigos', () => {
  const events = [event(1), event(4.9), event(6), event(2, { action: 'mask' })]
  assert.deepEqual(recentlyBlocked(events, NOW).map((e) => e.id), ['evt_1', 'evt_4.9'])
})

test('lista de eventos: só ameaças, mais nova primeiro, com filtro e limite', () => {
  const events = [
    event(5),
    event(4, { category: null, action: 'allow' }),
    event(3, { category: 'sensitive_data', action: 'mask' }),
    event(2),
    event(1, { category: 'sensitive_data', action: 'mask' }),
  ]
  assert.deepEqual(latestThreats(events, 'all').map((e) => e.id), ['evt_1', 'evt_2', 'evt_3', 'evt_5'])
  assert.deepEqual(latestThreats(events, 'block').map((e) => e.id), ['evt_2', 'evt_5'])
  assert.deepEqual(latestThreats(events, 'mask').map((e) => e.id), ['evt_1', 'evt_3'])
  assert.deepEqual(latestThreats(events, 'all', 2).map((e) => e.id), ['evt_1', 'evt_2'])
})

test('risco: 0,75 ou mais alto, 0,40 ou mais médio, abaixo disso baixo', () => {
  assert.deepEqual([0.92, 0.75, 0.74, 0.4, 0.39, 0].map(riskFor), ['high', 'high', 'medium', 'medium', 'low', 'low'])
})
