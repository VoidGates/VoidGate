import { useLang } from '../i18n.tsx'
import { Logo } from './Logo.tsx'
import { GITHUB_URL } from './links.ts'

export function Footer() {
  const { text } = useLang()
  return (
    <footer className="footer">
      <div>
        <Logo size="footer" />
        <p>{text.footer}</p>
      </div>
      <p className="footer__right">
        Josias, Kael, Matheus e Lucas
        <br />
        <a href={GITHUB_URL}>github.com/VoidGates</a>
      </p>
    </footer>
  )
}
