import { test } from 'node:test'
import assert from 'node:assert/strict'
import { classify, insertChangelog, nextVersion } from './release.mjs'

test('fix sobe o patch e feat sobe o minor', () => {
  assert.equal(classify('fix: fecha o proxy sem upstream'), 'patch')
  assert.equal(classify('feat(chat): aceita outro idioma'), 'minor')
  assert.equal(classify('docs: acerta o readme'), null)
  assert.equal(nextVersion('0.1.0', ['docs: acerta o readme']), null)
  assert.deepEqual(nextVersion('0.1.0', ['fix: fecha o proxy']), { version: '0.1.1', notes: ['fix: fecha o proxy'] })
  assert.equal(nextVersion('0.1.0', ['fix: a', 'feat: b']).version, '0.2.0')
})

test('quebra de contrato sobe o major e o commit de versão não dispara outro', () => {
  assert.equal(classify('feat!: remove o campo reason_code'), 'major')
  assert.equal(classify('fix: algo\n\nBREAKING CHANGE: o corpo mudou'), 'major')
  assert.equal(classify('chore: version 0.1.1 [skip release]'), null)
  assert.equal(nextVersion('0.1.4', ['feat!: remove o campo']).version, '1.0.0')
})

test('a seção nova entra acima da versão anterior', () => {
  const markdown = '# Changelog\n\nTexto.\n\n## [0.1.0] - 2026-11-19\n\n- base\n'
  const next = insertChangelog(markdown, '0.1.1', '2026-11-20', ['fix: fecha o proxy'])
  assert.ok(next.indexOf('## [0.1.1]') < next.indexOf('## [0.1.0]'))
  assert.ok(next.includes('- fix: fecha o proxy'))
  assert.ok(next.includes('- base'))
})
