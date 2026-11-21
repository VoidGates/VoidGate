import { useEffect, useState } from 'react'
import { createSimulator, HISTORY_WINDOW } from './events.ts'
import type { PanelEvent } from './events.ts'

type Feed = { events: PanelEvent[]; now: number }

/**
 * Keeps the last 30 minutes of simulated events and adds one every 2 to 4
 * seconds while not paused. When paused, time stops too, so the numbers on
 * screen stay put.
 */
export function useSimulatedEvents(paused: boolean): Feed {
  const [simulator] = useState(createSimulator)
  const [feed, setFeed] = useState<Feed>(() => {
    const now = Date.now()
    return { events: simulator.history(now), now }
  })

  useEffect(() => {
    if (paused) return
    let timer: number
    const schedule = () => {
      timer = window.setTimeout(() => {
        const now = Date.now()
        setFeed((current) => ({
          now,
          events: [...current.events, simulator.next(now)].filter((e) => e.timestamp > now - HISTORY_WINDOW),
        }))
        schedule()
      }, simulator.nextGap())
    }
    schedule()
    return () => window.clearTimeout(timer)
  }, [paused, simulator])

  return feed
}
