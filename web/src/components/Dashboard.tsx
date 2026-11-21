import { useState } from 'react'
import { allStatuses, latestThreats, recentlyBlocked, threatsPerMinute, totals } from '../dashboard/status.ts'
import type { EventFilter } from '../dashboard/status.ts'
import type { CategoryStatus } from '../dashboard/status.ts'
import { useEventFeed } from '../dashboard/useEventFeed.ts'
import { useLang } from '../i18n.tsx'
import { DashboardChart } from './DashboardChart.tsx'
import { EventDetail, EventList } from './DashboardEvents.tsx'
import { DashboardRiver } from './DashboardRiver.tsx'
import { Logo } from './Logo.tsx'

function Sparkline({ values }: { values: number[] }) {
  const width = 120
  const height = 36
  const max = Math.max(1, ...values)
  const step = width / Math.max(1, values.length - 1)
  const points = values
    .map((value, index) => `${(index * step).toFixed(1)},${(height - 2 - (value / max) * (height - 4)).toFixed(1)}`)
    .join(' ')
  return (
    <svg className="status__spark" width={width} height={height} aria-hidden="true">
      <polyline points={points} fill="none" strokeWidth="2" />
    </svg>
  )
}

export function Dashboard() {
  const { lang, text, toggle } = useLang()
  const copy = text.panel
  const [paused, setPaused] = useState(false)
  const { events, now, source } = useEventFeed(paused)
  const [filter, setFilter] = useState<EventFilter>('all')
  // null follows the newest threat; a click pins the chosen one while it stays in the window.
  const [pinnedId, setPinnedId] = useState<string | null>(null)
  const listed = latestThreats(events, filter)
  const pinned = pinnedId === null ? undefined : events.find((e) => e.id === pinnedId)
  const selected = pinned ?? listed[0] ?? null
  const sum = totals(events)
  const statuses = allStatuses(events, now)
  const number = new Intl.NumberFormat(lang === 'pt' ? 'pt-BR' : 'en-US')
  const time = new Intl.DateTimeFormat(lang === 'pt' ? 'pt-BR' : 'en-US', { hour: '2-digit', minute: '2-digit' })

  const detail = (status: CategoryStatus) => {
    if (status.count === 0) return copy.none
    if (status.category === 'prompt_injection' && status.peakAt !== null) return `${copy.peak} ${time.format(status.peakAt)}`
    if (status.category === 'sensitive_data' && status.topRule !== null) {
      const rule = copy.rules[status.topRule as keyof typeof copy.rules] ?? status.topRule
      return `${copy.top} ${rule}`
    }
    return status.lastAt !== null ? `${copy.last} ${time.format(status.lastAt)}` : copy.none
  }

  return (
    <div className="dash" id="painel">
      <header className="dash__header">
        <div className="dash__brand">
          <a href="#top" aria-label={text.nav.home}>
            <Logo />
          </a>
          <nav className="dash__nav" aria-label={copy.navLabel}>
            <a href="#top">{copy.site}</a>
            <a href="#painel" aria-current="page">
              {copy.current}
            </a>
          </nav>
        </div>
        <div className="dash__controls">
          <span className={`dash__demo dash__demo--${source}`}>
            <span className="dot" aria-hidden="true" />
            {source === 'api' ? copy.live : source === 'stale' ? copy.stale : copy.demo}
          </span>
          <button type="button" className="dash__button" aria-pressed={paused} onClick={() => setPaused((value) => !value)}>
            {paused ? copy.resume : copy.pause}
          </button>
          <button type="button" className="nav__lang" onClick={toggle}>
            {text.nav.lang}
          </button>
        </div>
      </header>

      <main className="dash__main">
        <h1 className="dash__title">{copy.title}</h1>
        <p className="lead">{copy.lead}</p>

        <DashboardRiver events={events} now={now} paused={paused} blocked={recentlyBlocked(events, now)} />

        <section className="dash__totals" aria-label={copy.window}>
          <p className="dash__window">{copy.window}</p>
          <dl>
            <div>
              <dt>{copy.totals.analyzed}</dt>
              <dd>{number.format(sum.analyzed)}</dd>
            </div>
            <div className="is-blocked">
              <dt>{copy.totals.blocked}</dt>
              <dd>{number.format(sum.blocked)}</dd>
            </div>
            <div className="is-masked">
              <dt>{copy.totals.masked}</dt>
              <dd>{number.format(sum.masked)}</dd>
            </div>
            <div className="is-allowed">
              <dt>{copy.totals.allowed}</dt>
              <dd>{number.format(sum.allowed)}</dd>
            </div>
          </dl>
        </section>

        <section className="status" aria-label={copy.statusLabel}>
          {statuses.map((status) => (
            <div key={status.category} className={`status__item status__item--${status.category} is-${status.level}`}>
              <p className="status__kind">
                <span className="status__swatch" aria-hidden="true" />
                {copy.categories[status.category]}
              </p>
              <h2 className="status__level">{copy.levels[status.level]}</h2>
              <p className="status__count">
                {number.format(status.count)} {copy.inWindow}
              </p>
              <span className="status__detail">{detail(status)}</span>
              <Sparkline values={status.perMinute} />
            </div>
          ))}
        </section>

        <DashboardChart buckets={threatsPerMinute(events, now)} />

        <div className="dash__split">
          <EventList
            events={listed}
            now={now}
            filter={filter}
            onFilter={setFilter}
            selectedId={selected?.id ?? null}
            onSelect={setPinnedId}
          />
          <EventDetail event={selected} />
        </div>
      </main>
    </div>
  )
}
