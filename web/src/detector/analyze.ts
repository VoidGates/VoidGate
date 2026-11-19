import { isValidCpf, maskCpf } from './cpf.ts'

export type Verdict = 'allow' | 'mask' | 'block'

export type FindingKind = 'manipulation' | 'invisible' | 'cpf' | 'email' | 'phone' | 'card' | 'iban'

export type Finding = {
  kind: FindingKind
  /** Start and end offsets in the original text (end is exclusive). */
  start: number
  end: number
  /** Short explanation in plain Portuguese, shown to the visitor. */
  reason: string
}

export type Analysis = {
  verdict: Verdict
  findings: Finding[]
  /** What would reach the model: null when the message is blocked. */
  forwarded: string | null
}

type Rule = { pattern: RegExp; reason: string }

/**
 * Simplified rules for the browser demo. Patterns run on a normalized copy of
 * the text: lowercase, without accents and without invisible characters.
 */
const MANIPULATION_RULES: Rule[] = [
  {
    pattern:
      /\b(ignore|ignora|ignorar|esqueca|esquecer|desconsidere|desconsiderar)\b.{0,30}\b(instrucoes|instrucao|regras|orientacoes|diretrizes)\b/g,
    reason: 'Pede para a IA desobedecer as regras definidas pela empresa.',
  },
  {
    pattern:
      /\b(ignore|disregard|forget)\b.{0,30}\b(instructions|rules|guidelines)\b/g,
    reason: 'Pede para a IA desobedecer as regras definidas pela empresa.',
  },
  {
    pattern:
      /\b(finja|fingir|finge|aja como|agir como|voce agora e|a partir de agora voce e)\b.{0,30}\b(administrador|admin|desenvolvedor|sem regras|sem restricoes)\b/g,
    reason: 'Tenta fazer a IA assumir um papel com mais permissões.',
  },
  {
    pattern:
      /\b(pretend|act as|you are now)\b.{0,30}\b(admin|administrator|developer|unrestricted|dan)\b/g,
    reason: 'Tenta fazer a IA assumir um papel com mais permissões.',
  },
  {
    pattern:
      /\b(mostre|revele|repita|diga|imprima)\b.{0,30}\b(prompt|instrucoes internas|instrucoes do sistema|instrucoes iniciais)\b/g,
    reason: 'Tenta descobrir as instruções secretas da IA.',
  },
  {
    pattern:
      /\b(show|reveal|repeat|print)\b.{0,30}\b(system prompt|your instructions|initial instructions)\b/g,
    reason: 'Tenta descobrir as instruções secretas da IA.',
  },
  {
    pattern: /\b(modo desenvolvedor|developer mode|modo desarrollador|mode developpeur|entwicklermodus|jailbreak)\b/g,
    reason: 'Tenta tirar a IA do modo seguro.',
  },
  {
    pattern: /\b(olvida|olvidar)\b.{0,40}\b(instrucciones|reglas)\b/g,
    reason: 'Pede para a IA desobedecer as regras definidas pela empresa.',
  },
  {
    pattern: /\b(oublie|oublier|ignorez)\b.{0,40}\b(instructions|consignes)\b/g,
    reason: 'Pede para a IA desobedecer as regras definidas pela empresa.',
  },
  {
    pattern: /\b(ignoriere|vergiss)\b.{0,40}\b(anweisungen|regeln)\b/g,
    reason: 'Pede para a IA desobedecer as regras definidas pela empresa.',
  },
  {
    pattern: /\b(actua como|ahora eres)\b.{0,40}\b(administrador|sin restricciones)\b/g,
    reason: 'Tenta fazer a IA assumir um papel com mais permissões.',
  },
  {
    pattern: /忽略之前的指令|忽略以上指令|忽略所有指令|تجاهل التعليمات/g,
    reason: 'Pede para a IA desobedecer as regras definidas pela empresa.',
  },
]

export const INVISIBLE_CHARS = /[\u200B-\u200D\u2060\uFEFF]/
const CPF_PATTERN = /(?<!\d)(\d{3})[.\s]?(\d{3})[.\s]?(\d{3})[-\s]?(\d{2})(?!\d)/g
const EMAIL_PATTERN = /[\w.+-]+@[\w-]+(\.[\w-]+)+/g
const PHONE_PATTERN = /\+[1-9][\d\s().-]{6,20}\d/g
const CARD_PATTERN = /(?<!\d)(?:\d[ .-]?){12,18}\d(?!\d)/g
const IBAN_PATTERN = /\b[A-Z]{2}\d{2}(?:\s?[A-Z0-9]){11,30}\b/gi

type Normalized = {
  text: string
  /** For each character of the normalized text, its offset in the original. */
  origin: number[]
}

/** Lowercases, removes accents and drops invisible characters, keeping offsets. */
function normalize(text: string): Normalized {
  let normalized = ''
  const origin: number[] = []
  for (let i = 0; i < text.length; i++) {
    const char = text[i]
    if (INVISIBLE_CHARS.test(char)) continue
    // Line breaks become spaces so "ignore\nas instruções" still matches.
    if (/\s/.test(char)) {
      normalized += ' '
      origin.push(i)
      continue
    }
    const base = char.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase()
    const out = base.length === 1 ? base : char.toLowerCase()
    for (const piece of out) {
      normalized += piece
      origin.push(i)
    }
  }
  return { text: normalized, origin }
}

function findManipulation(text: string): Finding[] {
  const { text: normalized, origin } = normalize(text)
  const findings: Finding[] = []
  for (const rule of MANIPULATION_RULES) {
    for (const match of normalized.matchAll(rule.pattern)) {
      const first = match.index
      const last = first + match[0].length - 1
      findings.push({
        kind: 'manipulation',
        start: origin[first],
        end: origin[last] + 1,
        reason: rule.reason,
      })
    }
  }
  return findings
}

function findInvisible(text: string): Finding[] {
  const findings: Finding[] = []
  for (let i = 0; i < text.length; i++) {
    if (INVISIBLE_CHARS.test(text[i])) {
      findings.push({
        kind: 'invisible',
        start: i,
        end: i + 1,
        reason: 'Caracteres invisíveis, que podem esconder instruções. Eles seriam removidos.',
      })
    }
  }
  return findings
}

function findPersonalData(text: string): Finding[] {
  const findings: Finding[] = []
  for (const match of text.matchAll(CPF_PATTERN)) {
    const digits = match.slice(1, 5).join('')
    if (isValidCpf(digits)) {
      findings.push({
        kind: 'cpf',
        start: match.index,
        end: match.index + match[0].length,
        reason: 'Um CPF válido. Ele seria escondido antes de chegar à IA.',
      })
    }
  }
  for (const match of text.matchAll(EMAIL_PATTERN)) {
    findings.push({
      kind: 'email',
      start: match.index,
      end: match.index + match[0].length,
      reason: 'Um e-mail. Ele seria escondido antes de chegar à IA.',
    })
  }
  for (const match of text.matchAll(PHONE_PATTERN)) {
    const digits = match[0].replace(/\D/g, '')
    if (digits.length >= 8 && digits.length <= 15) {
      findings.push({
        kind: 'phone',
        start: match.index,
        end: match.index + match[0].length,
        reason: 'Um telefone internacional. Ele seria escondido antes de chegar à IA.',
      })
    }
  }
  for (const match of text.matchAll(CARD_PATTERN)) {
    const digits = match[0].replace(/\D/g, '')
    if (digits.length >= 13 && digits.length <= 19 && luhn(digits) && !/^(\d)\1+$/.test(digits)) {
      findings.push({
        kind: 'card',
        start: match.index,
        end: match.index + match[0].length,
        reason: 'Um cartão. Ele seria escondido antes de chegar à IA.',
      })
    }
  }
  for (const match of text.matchAll(IBAN_PATTERN)) {
    const compact = match[0].replace(/\s/g, '').toUpperCase()
    if (ibanOk(compact)) {
      findings.push({
        kind: 'iban',
        start: match.index,
        end: match.index + match[0].length,
        reason: 'Um IBAN. Ele seria escondido antes de chegar à IA.',
      })
    }
  }
  return findings
}

function luhn(digits: string): boolean {
  let sum = 0
  let double = false
  for (let i = digits.length - 1; i >= 0; i--) {
    let value = Number(digits[i])
    if (double) {
      value *= 2
      if (value > 9) value -= 9
    }
    sum += value
    double = !double
  }
  return sum % 10 === 0
}

const IBAN_LENGTH: Record<string, number> = {
  BR: 29, DE: 22, ES: 24, FR: 27, GB: 22, IT: 27, NL: 18, PT: 25,
}

function ibanOk(compact: string): boolean {
  const expected = IBAN_LENGTH[compact.slice(0, 2)]
  if (!expected || compact.length !== expected) return false
  const rearranged = compact.slice(4) + compact.slice(0, 4)
  let remainder = 0
  for (const char of rearranged) {
    const value = char >= 'A' ? char.charCodeAt(0) - 55 : Number(char)
    if (value >= 10) remainder = (remainder * 10 + Math.floor(value / 10)) % 97
    remainder = (remainder * 10 + (value % 10)) % 97
  }
  return remainder === 1
}

function maskEmail(email: string): string {
  const at = email.indexOf('@')
  return `${email[0]}***${email.slice(at)}`
}

/** Builds the text that would reach the model, with personal data masked. */
function applyMasks(text: string, findings: Finding[]): string {
  const masks = findings
    .filter((f) => f.kind === 'cpf' || f.kind === 'email' || f.kind === 'phone' || f.kind === 'card' || f.kind === 'iban')
    .sort((a, b) => b.start - a.start)
  let result = text
  for (const finding of masks) {
    const original = text.slice(finding.start, finding.end)
    const replacement =
      finding.kind === 'cpf'
        ? maskCpf(original.replace(/\D/g, ''))
        : finding.kind === 'email'
          ? maskEmail(original)
          : '[redacted]'
    result =
      result.slice(0, finding.start) + replacement + result.slice(finding.end)
  }
  return result.replace(new RegExp(INVISIBLE_CHARS.source, 'g'), '')
}

const HIDDEN_NOTE = ' A frase usava caracteres invisíveis para se esconder.'

/**
 * Overlapping findings are merged so the UI can render them in sequence and
 * each piece of text is shown once. On equal starts the longer one comes
 * first, so a CPF inside an e-mail is masked as part of the e-mail.
 */
function mergeOverlaps(findings: Finding[]): Finding[] {
  const sorted = [...findings].sort((a, b) => a.start - b.start || b.end - a.end)
  const merged: Finding[] = []
  for (const finding of sorted) {
    const previous = merged.at(-1)
    if (previous && finding.start < previous.end) {
      previous.end = Math.max(previous.end, finding.end)
      // An attack always keeps its label, whatever it overlaps.
      if (finding.kind === 'manipulation') {
        previous.kind = 'manipulation'
        previous.reason = finding.reason
      } else if (
        finding.kind === 'invisible' &&
        previous.kind === 'manipulation' &&
        !previous.reason.endsWith(HIDDEN_NOTE)
      ) {
        previous.reason += HIDDEN_NOTE
      }
    } else {
      merged.push({ ...finding })
    }
  }
  return merged
}

export function analyze(text: string): Analysis {
  const findings = mergeOverlaps([
    ...findManipulation(text),
    ...findInvisible(text),
    ...findPersonalData(text),
  ])

  // Invisible characters alone are common in copied text, so they are only
  // removed. A block needs an actual manipulation attempt.
  const isBlocked = findings.some((f) => f.kind === 'manipulation')
  if (isBlocked) return { verdict: 'block', findings, forwarded: null }

  return {
    verdict: findings.length > 0 ? 'mask' : 'allow',
    findings,
    forwarded: applyMasks(text, findings),
  }
}
