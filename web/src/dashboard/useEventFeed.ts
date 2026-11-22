import { useEffect, useState } from 'react'
import { fetchEvents } from './api.ts'
import { HISTORY_WINDOW, type PanelEvent } from './events.ts'
import { useSimulatedEvents } from './useSimulatedEvents.ts'

/** simulated: no API reachable; api: live data; stale: the API stopped answering. */
export type FeedSource = 'simulated' | 'api' | 'partial' | 'stale'

type Feed = { events: PanelEvent[]; now: number; source: FeedSource }

const POLL_EVERY = 2_000

/**
 * Real events when the panel runs on the dev server with the API up, simulated
 * ones otherwise. The published site never calls the API: it has no key, and
 * the key must never reach the browser.
 */
export function useEventFeed(paused: boolean): Feed {
  const [live, setLive] = useState<{ events: PanelEvent[]; now: number; stale: boolean; partial: boolean } | null>(null)
  const simulated = useSimulatedEvents(paused || live !== null)

  useEffect(() => {
    if (!import.meta.env.DEV || paused) return
    const controller = new AbortController()
    let timer: number
    let connected = false
    let cursor: string | undefined
    let currentEvents: PanelEvent[] = []
    // Events lost at this moment are older than it, so the warning expires with the window.
    let lastLossAt = -Infinity
    const poll = async () => {
      try {
        let behind = false
        for (let pageNumber = 0; pageNumber < 12; pageNumber += 1) {
          const page = await fetchEvents(cursor, controller.signal)
          if (controller.signal.aborted) return
          if (page.reset || cursor === undefined) currentEvents = []
          if (page.gap || page.reset) lastLossAt = Date.now()
          currentEvents.push(...page.events)
          cursor = page.nextCursor
          if (!page.hasMore) break
          if (pageNumber === 11) behind = true
        }
        const now = Date.now()
        currentEvents = currentEvents.filter((event) => event.timestamp >= now - HISTORY_WINDOW)
        connected = true
        const partial = behind || lastLossAt >= now - HISTORY_WINDOW
        setLive({ events: [...currentEvents], now, stale: false, partial })
      } catch {
        if (controller.signal.aborted) return
        // No answer on the first try: stay on the simulation and stop asking,
        // so the dev server does not log a proxy error every 2 seconds. Reload
        // the page after starting the API. Once connected, keep the last data
        // on screen and say that the API stopped answering.
        if (!connected) return
        setLive((current) => (current === null ? null : { ...current, stale: true }))
      }
      timer = window.setTimeout(poll, POLL_EVERY)
    }
    poll()
    return () => {
      controller.abort()
      window.clearTimeout(timer)
    }
  }, [paused])

  if (live === null) return { ...simulated, source: 'simulated' }
  return { events: live.events, now: live.now, source: live.stale ? 'stale' : live.partial ? 'partial' : 'api' }
}
