import { useLang } from '../i18n.tsx'
import { HeroFlow } from './HeroFlow.tsx'
import { GITHUB_URL } from './links.ts'

export function Hero() {
  const { text } = useLang()
  return (
    <section className="hero" id="top">
      <div className="hero__text">
        <h1>{text.hero.title}</h1>
        <p className="lead">{text.hero.lead}</p>
        <div className="hero__actions">
          <a className="btn btn--primary" href="#teste">
            {text.hero.test}
          </a>
          <a className="btn btn--ghost" href={GITHUB_URL}>
            {text.hero.code}
          </a>
        </div>
      </div>
      <HeroFlow />
    </section>
  )
}
