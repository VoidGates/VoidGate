import type { PanelEvent } from '../dashboard/events.ts'
import { useLang } from '../i18n.tsx'

/** How long a dot takes to cross the river, in milliseconds. Matches the CSS. */
export const RIVER_TRAVEL = 10_000

type Props = { events: PanelEvent[]; now: number; paused: boolean; blocked: PanelEvent[] }

/** Where the blocked messages of the last minutes pile up, just before the gate. */
const PILE = Array.from({ length: 14 }, (_, i) => ({ x: 548 - (i % 4) * 11 - Math.floor(i / 4) * 3, y: 80 + Math.floor(i / 4) * 10 + (i % 2) * 4 }))

/**
 * The live river: every new message is a dot that crosses the gate. Safe
 * messages follow the upper river, messages with personal data follow the
 * lower one already hidden, and attacks stop at the gate.
 */
export function DashboardRiver({ events, now, paused, blocked }: Props) {
  const { text } = useLang()
  const copy = text.panel.river
  const moving = events.filter((e) => e.timestamp > now - RIVER_TRAVEL)
  // A blocked message joins the pile once its own dot has finished the trip.
  const piled = blocked.filter((e) => e.timestamp <= now - RIVER_TRAVEL).slice(-PILE.length)

  return (
    <figure className={`river ${paused ? 'is-paused' : ''}`}>
      <svg viewBox="0 0 1180 190" role="img" aria-label={copy.label}>
        <path className="river__in" d="M0 30 C200 30 300 70 560 75 L560 115 C300 120 200 160 0 160 Z" />
        <path className="river__safe" d="M610 70 C760 70 900 40 1180 40 L1180 92 C900 92 760 98 610 92 Z" />
        <path className="river__masked" d="M610 102 C760 102 900 110 1180 110 L1180 160 C900 160 760 130 610 124 Z" />

        {moving.map((event) => {
          // The id sets the lane, so a dot keeps its height between renders.
          const lane = (Number(event.id.replace(/\D/g, '')) * 37) % 100
          return (
            <circle
              key={event.id}
              className={`river__dot river__dot--${event.action} river__dot--${event.category ?? 'none'}`}
              r={event.category === null ? 4 : 5.5}
              style={{ ['--lane' as string]: `${40 + lane}px` }}
            />
          )
        })}

        {piled.map((event, index) => (
          <circle
            key={event.id}
            className={`river__piled river__dot--${event.category ?? 'none'}`}
            cx={PILE[index].x}
            cy={PILE[index].y}
            r="4.5"
          />
        ))}

        <rect className="river__gate" x="576" y="14" width="14" height="162" rx="7" />
        <rect className="river__lock" x="571" y="72" width="24" height="46" rx="9" />

        {blocked.length > 0 && (
          <g className="river__tag">
            <rect x="356" y="40" width="196" height="26" rx="13" />
            <text x="454" y="58" textAnchor="middle">
              {blocked.length} {copy.blocked}
            </text>
          </g>
        )}

        <text className="river__caption" x="0" y="186">
          {copy.arriving}
        </text>
        <text className="river__caption" x="1180" y="186" textAnchor="end">
          {copy.passed}
        </text>
      </svg>
    </figure>
  )
}
