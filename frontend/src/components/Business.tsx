import { useLang } from '../i18n.tsx'
import { GITHUB_URL } from './links.ts'

export function Business() {
  const { text } = useLang()
  return (
    <section className="section" id="planos">
      <div className="wrap">
        <h2 className="section__title">{text.business.title}</h2>
        <p className="lead">{text.business.lead}</p>

        <div className="plans">
          {text.business.plans.map(([name, price, items], index) => (
            <div
              key={name}
              className={`panel plan ${index === 1 ? 'plan--featured' : ''}`}
            >
              <h3>{name}</h3>
              <p className="plan__price">{price}</p>
              <ul>
                {items.map((item) => (
                  <li key={item}>{item}</li>
                ))}
              </ul>
            </div>
          ))}
        </div>

        <h3 className="roadmap__title">{text.business.roadmapTitle}</h3>
        <ol className="roadmap">
          {text.business.roadmap.map(([stage, body]) => (
            <li key={stage}>
              <b>{stage}</b>
              <p>{body}</p>
            </li>
          ))}
        </ol>

        <div className="panel cta">
          <h2 className="oss__title">{text.business.ctaTitle}</h2>
          <p className="lead">{text.business.ctaLead}</p>
          <div className="hero__actions">
            <a className="btn btn--primary" href={GITHUB_URL}>
              {text.business.cta}
            </a>
            <a className="btn btn--ghost" href="#teste">
              {text.business.try}
            </a>
          </div>
        </div>
      </div>
    </section>
  )
}
