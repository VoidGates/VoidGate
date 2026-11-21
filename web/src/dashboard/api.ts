import type { Action, Category, PanelEvent } from './events.ts'

/** One event as `GET /v1/events` returns it: metadata only, never message text. */
export type ApiEvent = {
  id: number
  timestamp_ms: number
  route: 'decisions_chat' | 'proxy_chat'
  direction: 'input' | 'output'
  action: Action
  reason_codes: string[]
}

const ROUTES = new Set(['decisions_chat', 'proxy_chat'])
const DIRECTIONS = new Set(['input', 'output'])
const ACTIONS = new Set(['allow', 'mask', 'block'])

/**
 * Reason codes to panel categories. Hidden characters count as prompt
 * injection, as agreed with the back-end. File fraud has no code yet.
 * When an event has more than one code, the most serious one wins.
 */
const SEVERITY: [string, Category][] = [
  ['content.prompt_injection', 'prompt_injection'],
  ['content.invisible_unicode', 'prompt_injection'],
  ['content.personal_data', 'sensitive_data'],
]

/** Threat types the API detects today. File fraud is only in the simulation. */
export const MONITORED_BY_API: readonly Category[] = ['prompt_injection', 'sensitive_data']

/** Whether the panel has real data for this threat type in the current mode. */
export function isMonitored(category: Category, live: boolean): boolean {
  return !live || MONITORED_BY_API.includes(category)
}

export function categoryFor(reasonCodes: string[]): Category | null {
  for (const [code, category] of SEVERITY) {
    if (reasonCodes.includes(code)) return category
  }
  return null
}

function isApiEvent(value: unknown): value is ApiEvent {
  if (typeof value !== 'object' || value === null) return false
  const e = value as Record<string, unknown>
  return (
    Number.isInteger(e.id) &&
    Number.isFinite(e.timestamp_ms) &&
    ROUTES.has(e.route as string) &&
    DIRECTIONS.has(e.direction as string) &&
    ACTIONS.has(e.action as string) &&
    Array.isArray(e.reason_codes) &&
    e.reason_codes.every((code) => typeof code === 'string')
  )
}

export function toPanelEvent(event: ApiEvent): PanelEvent {
  return {
    id: `api_${event.id}`,
    timestamp: event.timestamp_ms,
    category: categoryFor(event.reason_codes),
    action: event.action,
    direction: event.direction,
    ruleId: event.reason_codes.join(',') || 'none',
    route: event.route,
    reasonCodes: [...event.reason_codes],
  }
}

/**
 * Reads the body of `GET /v1/events`. Malformed events are skipped instead of
 * breaking the whole panel; a body without an `events` list is rejected.
 */
export function parseEventsResponse(body: unknown): PanelEvent[] {
  if (typeof body !== 'object' || body === null || !Array.isArray((body as { events?: unknown }).events)) {
    throw new Error('Unexpected response from /v1/events')
  }
  return (body as { events: unknown[] }).events.filter(isApiEvent).map(toPanelEvent)
}

/** The dev server proxies `/api` to the API and adds the key there. */
export const EVENTS_URL = '/api/v1/events'

export async function fetchEvents(signal?: AbortSignal): Promise<PanelEvent[]> {
  const response = await fetch(EVENTS_URL, { signal, cache: 'no-store' })
  if (!response.ok) throw new Error(`GET /v1/events answered ${response.status}`)
  return parseEventsResponse(await response.json())
}
