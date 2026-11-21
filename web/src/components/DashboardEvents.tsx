import type { PanelEvent } from '../dashboard/events.ts'
import { riskFor } from '../dashboard/status.ts'
import type { EventFilter } from '../dashboard/status.ts'
import { useLang } from '../i18n.tsx'

/** How long a new event keeps its highlight, in milliseconds. */
const NEW_FOR = 4_000

const FILTERS: EventFilter[] = ['all', 'block', 'mask']

type ListProps = {
  events: PanelEvent[]
  now: number
  filter: EventFilter
  onFilter: (filter: EventFilter) => void
  selectedId: string | null
  onSelect: (id: string) => void
}

export function EventList({ events, now, filter, onFilter, selectedId, onSelect }: ListProps) {
  const { lang, text } = useLang()
  const copy = text.panel
  const time = new Intl.DateTimeFormat(lang === 'pt' ? 'pt-BR' : 'en-US', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })

  return (
    <section className="events" aria-labelledby="events-title">
      <div className="events__head">
        <h2 id="events-title">{copy.events.title}</h2>
        <div className="events__filters" role="group" aria-label={copy.events.filterLabel}>
          {FILTERS.map((option) => (
            <button key={option} type="button" aria-pressed={filter === option} onClick={() => onFilter(option)}>
              {copy.events.filters[option]}
            </button>
          ))}
        </div>
      </div>

      {events.length === 0 ? (
        <p className="events__empty">{copy.events.empty}</p>
      ) : (
        <ol className="events__list">
          {events.map((event) => (
            <li key={event.id}>
              <button
                type="button"
                className={`event ${event.timestamp > now - NEW_FOR ? 'is-new' : ''}`}
                aria-pressed={event.id === selectedId}
                onClick={() => onSelect(event.id)}
              >
                <span className="event__time">{time.format(event.timestamp)}</span>
                <span className={`event__kind event__kind--${event.category}`}>
                  {event.category && copy.categories[event.category]}
                </span>
                <span className="event__snippet">{event.snippet}</span>
                <span className={`event__action event__action--${event.action}`}>{copy.actions[event.action]}</span>
              </button>
            </li>
          ))}
        </ol>
      )}
    </section>
  )
}

export function EventDetail({ event }: { event: PanelEvent | null }) {
  const { lang, text } = useLang()
  const copy = text.panel
  if (event === null || event.category === null) {
    return <aside className="detail detail--empty">{copy.events.empty}</aside>
  }
  const info = copy.detail[event.category]
  const risk = riskFor(event.score)
  const time = new Intl.DateTimeFormat(lang === 'pt' ? 'pt-BR' : 'en-US', {
    day: '2-digit',
    month: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
  const number = new Intl.NumberFormat(lang === 'pt' ? 'pt-BR' : 'en-US', { minimumFractionDigits: 2 })
  const isOutput = event.direction === 'output'
  const received = event.action === 'block' ? copy.detail.receivedNothing : event.snippet

  return (
    <aside className="detail" aria-live="polite">
      <span className={`event__action event__action--${event.action}`}>{copy.actions[event.action]}</span>
      <h2>{isOutput ? copy.detail.outputTitle : info.title}</h2>
      <p className="detail__text">{isOutput ? copy.detail.outputText : info.text}</p>

      {event.file && (
        <>
          <p className="detail__label">{copy.detail.file}</p>
          <p className="detail__box">
            {event.file.name}, {copy.detail.page} {event.file.page}
          </p>
        </>
      )}

      <p className="detail__label">
        {event.file ? copy.detail.hidden : isOutput ? copy.detail.answer : copy.detail.message}
      </p>
      <p className={`detail__box detail__box--${event.category}`}>{event.snippet}</p>

      {!isOutput && (
        <>
          <p className="detail__label">{copy.detail.received}</p>
          <p className="detail__box detail__box--muted">{received}</p>
        </>
      )}

      <dl className="detail__facts">
        <dt>{copy.detail.time}</dt>
        <dd>{time.format(event.timestamp)}</dd>
        <dt>{copy.detail.rule}</dt>
        <dd className="detail__rule">{event.ruleId}</dd>
        <dt>{copy.detail.channel}</dt>
        <dd>{event.channel}</dd>
        <dt>{copy.detail.risk}</dt>
        <dd>
          {copy.detail.risks[risk]} ({number.format(event.score)})
          <span className={`detail__risk detail__risk--${risk}`} aria-hidden="true">
            <i style={{ width: `${Math.round(event.score * 100)}%` }} />
          </span>
        </dd>
      </dl>
    </aside>
  )
}
