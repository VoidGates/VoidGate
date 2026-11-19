// Scans a unified diff (stdin) for signs of malicious or dangerous changes.
// Prints a Markdown report; exits 1 when any "block" finding is present.
import { readFileSync, appendFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

// Matches any added line: for files whose mere presence in a PR is the problem.
const ANY = /\S/

const RULES = [
  // block: almost never legitimate in this repo
  { level: 'block', name: 'Chave privada', re: /-----BEGIN [A-Z ]*PRIVATE KEY-----/ },
  { level: 'block', name: 'Token AWS', re: /\bAKIA[0-9A-Z]{16}\b/ },
  { level: 'block', name: 'Token GitHub', re: /\b(gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{40,})\b/ },
  { level: 'block', name: 'Token Slack', re: /\bxox[abprs]-[A-Za-z0-9-]{10,}/ },
  { level: 'block', name: 'Segredo hardcoded', re: /(api[_-]?key|secret|passw(or)?d|token)["']?\s*[:=]\s*["'][^"'\s]{12,}["']/i },
  { level: 'block', name: 'Download executado no shell', re: /\b(curl|wget)\b[^\n|]*\|\s*(sudo\s+)?(ba|z)?sh\b/ },
  { level: 'block', name: 'Código decodificado executado', re: /\b(eval|Function)\s*\(\s*(atob|Buffer\.from|unescape|decodeURIComponent)\s*\(/ },
  { level: 'block', name: 'Caractere Unicode invisível (Trojan Source)', re: /[\u202A-\u202E\u2066-\u2069\u200B\u2060]/ },
  { level: 'block', name: 'Hook de instalação npm', file: /(^|\/)package\.json$/, re: /"(pre|post)?install"\s*:/ },
  { level: 'block', name: 'Pacote baixado fora do registry npm', file: /(^|\/)package-lock\.json$/, re: /"resolved":\s*"(?!https:\/\/registry\.npmjs\.org\/)/ },
  { level: 'block', name: 'Troca de registry npm', file: /(^|\/)\.npmrc$/, re: /registry\s*=/ },
  // Rust: build.rs roda código na máquina de quem compila, igual a um postinstall
  { level: 'block', name: 'build.rs do Cargo (executa código ao compilar)', file: /(^|\/)build\.rs$/, re: ANY },
  { level: 'block', name: 'Build script declarado no Cargo.toml', file: /(^|\/)Cargo\.toml$/, re: /^\s*(package\s*\.\s*)?["']?build["']?\s*=/ },
  { level: 'block', name: 'Dependência Rust fora do crates.io', file: /(^|\/)Cargo\.toml$/, re: /(^|[\s{,.])["']?(git|registry)["']?\s*=/ },
  { level: 'block', name: 'Seção [patch]/[replace] no Cargo.toml (troca a origem de um crate)', file: /(^|\/)Cargo\.toml$/, re: /^\s*\[\s*(patch|replace)\b/ },
  { level: 'block', name: 'Crate baixado fora do crates.io', file: /(^|\/)Cargo\.lock$/, re: /^source\s*=\s*"(?!registry\+https:\/\/github\.com\/rust-lang\/crates\.io-index")/ },
  { level: 'block', name: 'Configuração do Cargo alterada (registry/fonte)', file: /(^|\/)\.cargo\/config(\.toml)?$/, re: ANY },
  // warn: legitimate sometimes, needs a human look
  { level: 'warn', name: 'eval / new Function', re: /\beval\s*\(|\bnew Function\s*\(/ },
  { level: 'warn', name: 'child_process', re: /child_process/ },
  { level: 'warn', name: 'Caminho local no Cargo.toml (dependência ou alvo fora do padrão)', file: /(^|\/)Cargo\.toml$/, re: /(^|[\s{,.])["']?path["']?\s*=/ },
  { level: 'warn', name: 'Executa processo externo (Rust)', re: /\bCommand::new\s*\(/ },
  { level: 'warn', name: 'Bloco unsafe (Rust)', re: /\bunsafe\s*(\{|fn\b|impl\b)/ },
  { level: 'warn', name: 'Blob base64/hex longo (possível payload ofuscado)', re: /[A-Za-z0-9+/=]{200,}|(\\x[0-9a-f]{2}){30,}/i },
  { level: 'warn', name: 'Workflow usa pull_request_target', file: /^\.github\/workflows\//, re: /pull_request_target/ },
  { level: 'warn', name: 'Workflow acessa secrets', file: /^\.github\/workflows\//, re: /\$\{\{\s*secrets\./ },
]

const SENSITIVE_PATHS = [
  [/^\.github\//, 'Altera CI/CD (.github/) — revise com atenção redobrada'],
  [/(^|\/)package\.json$/, 'Altera dependências ou scripts npm'],
  [/(^|\/)Cargo\.toml$/, 'Altera dependências Rust'],
  [/(^|\/)vite\.config\.[jt]s$/, 'Altera a configuração do build'],
]

// Git prints paths with special or non-ASCII characters as a quoted C string
// ("b/\303\251/x"); left as is, no file rule would ever match them.
function unquote(path) {
  if (!path.startsWith('"')) return path
  const text = path.slice(1, path.endsWith('"') ? -1 : undefined)
  const bytes = []
  for (let i = 0; i < text.length; i++) {
    const octal = text.slice(i + 1, i + 4)
    if (text[i] !== '\\') bytes.push(...Buffer.from(text[i]))
    else if (/^[0-7]{3}$/.test(octal)) { bytes.push(parseInt(octal, 8)); i += 3 }
    else { i++; bytes.push(...Buffer.from({ n: '\n', t: '\t' }[text[i]] ?? text[i] ?? '')) }
  }
  return Buffer.from(bytes).toString('utf8')
}

export function scan(diff) {
  const findings = []
  const files = new Set()
  let file = null
  let line = 0
  let prev = ''
  let inHunk = false
  for (const raw of diff.split('\n')) {
    if (raw.startsWith('diff --git ')) {
      file = null
      prev = ''
      inHunk = false
      continue
    }
    // Header-like additions inside a hunk are content, even after a removed "-- x" line.
    const isHeader = !inHunk && raw.startsWith('+++ ') && prev.startsWith('--- ')
    prev = raw
    if (isHeader) {
      const path = unquote(raw.slice(4))
      file = path === '/dev/null' ? null : path.slice(2)
      if (file) files.add(file)
      continue
    }
    // A pure rename or copy has no added lines, so content rules never run on
    // it: moving an existing file to build.rs would pass unnoticed.
    const moved = !inHunk && raw.match(/^(?:rename|copy) to (.+)$/)
    if (moved) {
      file = unquote(moved[1])
      files.add(file)
      for (const r of RULES)
        if (r.re === ANY && r.file.test(file)) findings.push({ level: r.level, name: r.name, file, line: 0 })
      continue
    }
    const hunk = raw.match(/^@@ -\d+(?:,\d+)? \+(\d+)/)
    if (hunk) { line = Number(hunk[1]); inHunk = true; continue }
    const bin = raw.match(/^Binary files .* and b\/(.+) differ$/)
    if (bin) { findings.push({ level: 'warn', name: 'Arquivo binário', file: bin[1], line: 0 }); continue }
    if (!file || raw.startsWith('---')) continue
    if (raw.startsWith('+')) {
      const text = raw.slice(1)
      for (const r of RULES) {
        if (r.file && !r.file.test(file)) continue
        if (r.re.test(text)) findings.push({ level: r.level, name: r.name, file, line })
      }
      line++
    } else if (!raw.startsWith('-') && !raw.startsWith('\\')) line++
  }
  for (const f of files)
    for (const [re, why] of SENSITIVE_PATHS)
      if (re.test(f)) findings.push({ level: 'warn', name: why, file: f, line: 0 })
  return findings
}

export function report(findings) {
  const blocks = findings.filter(f => f.level === 'block')
  const head = blocks.length
    ? `## 🚫 PR Guard: ${blocks.length} problema(s) bloqueante(s)\nEste PR não pode ser mergeado até os itens 🚫 serem removidos.`
    : findings.length
      ? '## ⚠️ PR Guard: revisão humana necessária\nNada bloqueante, mas o aprovador deve conferir os pontos abaixo.'
      : '## ✅ PR Guard: nenhum padrão suspeito encontrado\nAinda precisa da aprovação de outro membro.'
  const rows = findings.map(f =>
    `| ${f.level === 'block' ? '🚫' : '⚠️'} | ${f.name} | \`${f.file}${f.line ? `:${f.line}` : ''}\` |`)
  return [head, ...(rows.length ? ['', '| | Regra | Onde |', '|---|---|---|', ...rows] : [])].join('\n')
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const findings = scan(readFileSync(0, 'utf8'))
  const md = report(findings)
  console.log(md)
  if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, md + '\n')
  process.exit(findings.some(f => f.level === 'block') ? 1 : 0)
}
