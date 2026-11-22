import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { WORKSPACE_CRATES, bumpCargoLock, classify, insertChangelog, nextVersion } from './release.mjs'

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

test('o Cargo.lock sobe a versão dos crates do workspace e de mais nenhum', () => {
  const lock = [
    'name = "serde"\nversion = "0.2.0"',
    ...WORKSPACE_CRATES.map((name) => `name = "${name}"\nversion = "0.2.0"`),
    'name = "voidgate-tests"\nversion = "0.1.0"',
  ].join('\n\n')
  const bumped = bumpCargoLock(lock, '0.2.0', '0.3.0')
  assert.ok(bumped.includes('name = "serde"\nversion = "0.2.0"'))
  for (const name of WORKSPACE_CRATES) assert.ok(bumped.includes(`name = "${name}"\nversion = "0.3.0"`), name)
  assert.ok(bumped.includes('name = "voidgate-tests"\nversion = "0.1.0"'))
})

test('todo crate que herda a versão do workspace está na lista do release', () => {
  // Um crate novo com version.workspace = true fora da lista ficaria velho no Cargo.lock.
  const version = readFileSync('Cargo.toml', 'utf8').match(/^version = "(.+)"/m)[1]
  const lock = readFileSync('Cargo.lock', 'utf8').replace(/\r\n/g, '\n')
  const atVersion = [...lock.matchAll(/name = "(voidgate[^"]*)"\nversion = "([^"]+)"/g)]
    .filter(([, , v]) => v === version)
    .map(([, name]) => name)
    .sort()
  assert.deepEqual(atVersion, [...WORKSPACE_CRATES].sort())
})
