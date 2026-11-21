import { CATEGORIES, MINUTE } from './events.ts'
import type { Category, PanelEvent } from './events.ts'

export type Level = 'calm' | 'attention' | 'attack'

export const STATUS_WINDOW = 15 * MINUTE

/** Events of one type in the last 15 minutes: 0–2 calm, 3–9 attention, 10+ under attack. */
export function levelFor(count: number): Level {
  if (count >= 10) return 'attack'
  if (count >= 3) return 'attention'
  return 'calm'
}

export type CategoryStatus = {
  category: Category
  count: number
  level: Level
  /** Events per minute over the window, oldest first, for the sparkline. */
  perMinute: number[]
  /** Start of the busiest minute in the window, or null when there were none. */
  peakAt: number | null
  lastAt: number | null
  /** Most frequent rule in the window, such as `cpf` for sensitive data. */
  topRule: string | null
}

export function categoryStatus(events: PanelEvent[], category: Category, now: number): CategoryStatus {
  const from = now - STATUS_WINDOW
  const recent = events.filter((e) => e.category === category && e.timestamp > from && e.timestamp <= now)
  const minutes = STATUS_WINDOW / MINUTE
  const perMinute = new Array<number>(minutes).fill(0)
  const rules = new Map<string, number>()
  for (const event of recent) {
    const index = Math.min(minutes - 1, Math.floor((event.timestamp - from) / MINUTE))
    perMinute[index] += 1
    rules.set(event.ruleId, (rules.get(event.ruleId) ?? 0) + 1)
  }
  const peak = Math.max(...perMinute)
  const peakIndex = peak > 0 ? perMinute.lastIndexOf(peak) : -1
  let topRule: string | null = null
  for (const [rule, count] of rules) {
    if (topRule === null || count > (rules.get(topRule) ?? 0)) topRule = rule
  }
  return {
    category,
    count: recent.length,
    level: levelFor(recent.length),
    perMinute,
    peakAt: peakIndex >= 0 ? from + peakIndex * MINUTE : null,
    lastAt: recent.length > 0 ? Math.max(...recent.map((e) => e.timestamp)) : null,
    topRule,
  }
}

export function allStatuses(events: PanelEvent[], now: number): CategoryStatus[] {
  return CATEGORIES.map((category) => categoryStatus(events, category, now))
}

export type Totals = { analyzed: number; blocked: number; masked: number; allowed: number }

export function totals(events: PanelEvent[]): Totals {
  const result: Totals = { analyzed: events.length, blocked: 0, masked: 0, allowed: 0 }
  for (const event of events) {
    if (event.action === 'block') result.blocked += 1
    else if (event.action === 'mask') result.masked += 1
    else result.allowed += 1
  }
  return result
}

export type MinuteBucket = { start: number } & Record<Category, number>

/** Threats per minute over the last `minutes`, oldest first, for the stacked chart. */
export function threatsPerMinute(events: PanelEvent[], now: number, minutes = 30): MinuteBucket[] {
  const from = now - minutes * MINUTE
  const buckets: MinuteBucket[] = Array.from({ length: minutes }, (_, index) => ({
    start: from + index * MINUTE,
    prompt_injection: 0,
    sensitive_data: 0,
    file_fraud: 0,
  }))
  for (const event of events) {
    if (event.category === null || event.timestamp <= from || event.timestamp > now) continue
    const index = Math.min(minutes - 1, Math.floor((event.timestamp - from) / MINUTE))
    buckets[index][event.category] += 1
  }
  return buckets
}

/** Messages blocked in the last `minutes`, oldest first. */
export function recentlyBlocked(events: PanelEvent[], now: number, minutes = 5): PanelEvent[] {
  return events.filter((e) => e.action === 'block' && e.timestamp > now - minutes * MINUTE && e.timestamp <= now)
}

export type EventFilter = 'all' | 'block' | 'mask'

/** Threats only (messages that went through untouched are left out), newest first. */
export function latestThreats(events: PanelEvent[], filter: EventFilter, limit = 12): PanelEvent[] {
  const result: PanelEvent[] = []
  for (let i = events.length - 1; i >= 0 && result.length < limit; i--) {
    const event = events[i]
    if (event.category === null) continue
    if (filter !== 'all' && event.action !== filter) continue
    result.push(event)
  }
  return result
}

export type Risk = 'high' | 'medium' | 'low'

export function riskFor(score: number): Risk {
  if (score >= 0.75) return 'high'
  if (score >= 0.4) return 'medium'
  return 'low'
}
