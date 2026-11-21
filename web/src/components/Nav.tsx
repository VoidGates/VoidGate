import { useLang } from '../i18n.tsx'
import { Logo } from './Logo.tsx'
import { GITHUB_URL } from './links.ts'

export function Nav() {
  const { text, toggle } = useLang()
  return (
    <header className="nav">
      <a href="#top" aria-label={text.nav.home}>
        <Logo />
      </a>
      <nav aria-label={text.nav.sections}>
        <ul className="nav__links">
          <li>
            <a href="#teste">{text.nav.test}</a>
          </li>
          <li>
            <a href="#como-funciona">{text.nav.how}</a>
          </li>
          <li>
            <a href="#para-quem">{text.nav.who}</a>
          </li>
          <li>
            <a href="#planos">{text.nav.plans}</a>
          </li>
          <li>
            <a href={GITHUB_URL}>GitHub</a>
          </li>
          <li>
            <button type="button" className="nav__lang" onClick={toggle}>
              {text.nav.lang}
            </button>
          </li>
        </ul>
      </nav>
      <a className="nav__panel" href="#painel">
        {text.nav.panel}
      </a>
    </header>
  )
}
