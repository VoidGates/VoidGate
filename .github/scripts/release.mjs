// Cuts a version from commits on main.
// shortcut: only feat, fix and a breaking mark; other types never release.
import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const RANK = { patch: 1, minor: 2, major: 3 }

export function classify(message) {
  const head = message.split('\n')[0].trim()
  if (head.startsWith('chore: version ') || head.includes('[skip release]')) return null
  if (/^(feat|fix)(\([^)]+\))?!:/.test(head) || /^BREAKING CHANGE:/m.test(message)) return 'major'
  if (/^feat(\([^)]+\))?:/.test(head)) return 'minor'
  if (/^fix(\([^)]+\))?:/.test(head)) return 'patch'
  return null
}

export function nextVersion(current, messages) {
  let level = null
  const notes = []
  for (const message of messages) {
    const kind = classify(message)
    if (!kind) continue
    notes.push(message.split('\n')[0].trim())
    if (!level || RANK[kind] > RANK[level]) level = kind
  }
  if (!level) return null
  const [major, minor, patch] = current.split('.').map(Number)
  const version = level === 'major'
    ? `${major + 1}.0.0`
    : level === 'minor'
      ? `${major}.${minor + 1}.0`
      : `${major}.${minor}.${patch + 1}`
  return { version, notes }
}

export function insertChangelog(markdown, version, date, notes) {
  const section = [`## [${version}] - ${date}`, '', ...notes.map(note => `- ${note}`), ''].join('\n')
  const at = markdown.indexOf('\n## [')
  if (at === -1) return `${markdown.trimEnd()}\n\n${section}`
  return `${markdown.slice(0, at + 1)}${section}${markdown.slice(at + 1)}`
}

function git(args, env) {
  return execFileSync('git', args, { encoding: 'utf8', env }).trim()
}

function gh(args, env) {
  return execFileSync('gh', args, { encoding: 'utf8', env }).trim()
}

// Crates that take their version from [workspace.package] in Cargo.toml.
export const WORKSPACE_CRATES = ['voidgate', 'voidgate-core', 'voidgate-py']

export function bumpCargoLock(lock, from, version) {
  return WORKSPACE_CRATES.reduce(
    (text, name) => text.replace(`name = "${name}"\nversion = "${from}"`, `name = "${name}"\nversion = "${version}"`),
    lock,
  )
}

function writeVersion(version, from, notes) {
  const cargo = 'Cargo.toml'
  writeFileSync(cargo, readFileSync(cargo, 'utf8').replace(`version = "${from}"`, `version = "${version}"`))
  const cargoLock = 'Cargo.lock'
  writeFileSync(cargoLock, bumpCargoLock(readFileSync(cargoLock, 'utf8'), from, version))
  const packageJson = 'web/package.json'
  writeFileSync(packageJson, readFileSync(packageJson, 'utf8').replace(`"version": "${from}"`, `"version": "${version}"`))
  const lock = 'web/package-lock.json'
  const locked = readFileSync(lock, 'utf8')
  const marker = `"name": "voidgate-web",\n  "version": "${from}"`
  const markerNested = `"name": "voidgate-web",\n      "version": "${from}"`
  writeFileSync(lock, locked
    .replace(marker, `"name": "voidgate-web",\n  "version": "${version}"`)
    .replace(markerNested, `"name": "voidgate-web",\n      "version": "${version}"`))
  const log = 'CHANGELOG.md'
  const date = new Date().toISOString().slice(0, 10)
  writeFileSync(log, insertChangelog(readFileSync(log, 'utf8'), version, date, notes))
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const tag = git(['describe', '--tags', '--abbrev=0', '--match', 'v[0-9]*'])
  const from = tag.slice(1)
  const raw = git(['log', `${tag}..HEAD`, '--format=%B%x1e'])
  const messages = raw.split('\x1e').map(message => message.trim()).filter(Boolean)
  const bump = nextVersion(from, messages)
  if (!bump) {
    console.log('no release')
    process.exit(0)
  }
  writeVersion(bump.version, from, bump.notes)
  const message = `chore: version ${bump.version} [skip release]\n\n${bump.notes.map(note => `- ${note}`).join('\n')}\n`
  const env = {
    ...process.env,
    GIT_AUTHOR_NAME: 'github-actions[bot]',
    GIT_AUTHOR_EMAIL: '41898282+github-actions[bot]@users.noreply.github.com',
    GIT_COMMITTER_NAME: 'github-actions[bot]',
    GIT_COMMITTER_EMAIL: '41898282+github-actions[bot]@users.noreply.github.com',
  }
  const branch = `release/v${bump.version}`
  git(['checkout', '-b', branch])
  git(['add', '--', 'Cargo.toml', 'Cargo.lock', 'web/package.json', 'web/package-lock.json', 'CHANGELOG.md'])
  git(['commit', '-m', message], env)
  const sha = git(['rev-parse', 'HEAD'])
  git(['push', 'origin', `HEAD:refs/heads/${branch}`])
  gh(['pr', 'create', '--base', 'main', '--head', branch, '--title', message.split('\n')[0], '--body', message], env)
  gh(['pr', 'merge', branch, '--merge'], env)
  git(['tag', '-a', `v${bump.version}`, sha, '-m', `VoidGate ${bump.version}`], env)
  git(['push', 'origin', `refs/tags/v${bump.version}`])
  console.log(`released ${bump.version}`)
}
