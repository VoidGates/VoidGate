import { CATEGORIES } from '../dashboard/events.ts'
import type { MinuteBucket } from '../dashboard/status.ts'
import { useLang } from '../i18n.tsx'

const WIDTH = 1140
const HEIGHT = 150
const BAR_GAP = 8

/** Stacked bars of threats per minute, one color per threat type. */
export function DashboardChart({ buckets }: { buckets: MinuteBucket[] }) {
  const { lang, text } = useLang()
  const copy = text.panel
  const time = new Intl.DateTimeFormat(lang === 'pt' ? 'pt-BR' : 'en-US', { hour: '2-digit', minute: '2-digit' })
  const totals = CATEGORIES.map((category) => buckets.reduce((sum, bucket) => sum + bucket[category], 0))
  const tallest = Math.max(1, ...buckets.map((b) => CATEGORIES.reduce((sum, c) => sum + b[c], 0)))
  const slot = WIDTH / buckets.length
  const unit = (HEIGHT - 6) / tallest
  const summary = CATEGORIES.map((category, index) => `${copy.categories[category]}: ${totals[index]}`).join(', ')
  const labelEvery = Math.ceil(buckets.length / 4)

  return (
    <section className="chart" aria-labelledby="chart-title">
      <div className="chart__head">
        <h2 id="chart-title">{copy.chart.title}</h2>
        <ul className="chart__legend">
          {CATEGORIES.map((category) => (
            <li key={category} className={`chart__key chart__key--${category}`}>
              {copy.categories[category]}
            </li>
          ))}
        </ul>
      </div>
      <svg viewBox={`0 0 ${WIDTH} ${HEIGHT + 24}`} role="img" aria-label={`${copy.chart.label} ${summary}.`}>
        {buckets.map((bucket, index) => {
          let top = HEIGHT
          return (
            <g key={bucket.start}>
              {CATEGORIES.map((category) => {
                const height = bucket[category] * unit
                if (height === 0) return null
                top -= height
                return (
                  <rect
                    key={category}
                    className={`chart__bar chart__bar--${category}`}
                    x={index * slot}
                    y={top}
                    width={slot - BAR_GAP}
                    height={Math.max(2, height - 2)}
                    rx="3"
                  />
                )
              })}
              {index % labelEvery === 0 && (
                <text className="chart__time" x={index * slot} y={HEIGHT + 20}>
                  {time.format(bucket.start)}
                </text>
              )}
            </g>
          )
        })}
        <line className="chart__axis" x1="0" y1={HEIGHT + 0.5} x2={WIDTH} y2={HEIGHT + 0.5} />
      </svg>
    </section>
  )
}
