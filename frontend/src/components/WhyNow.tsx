import { useLang } from '../i18n.tsx'

export function WhyNow() {
  const { text } = useLang()
  return (
    <section className="section" id="por-que">
      <div className="wrap">
        <h2 className="section__title">{text.why.title}</h2>
        <p className="lead">{text.why.lead}</p>
        <div className="stats">
          {text.why.stats.map((stat) => (
            <div key={stat.value}>
              <b className="stats__value">{stat.value}</b>
              <p>{stat.text}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}
