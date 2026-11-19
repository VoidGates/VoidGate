import { useLang } from '../i18n.tsx'
import { GITHUB_URL } from './links.ts'

export function Audience() {
  const { text } = useLang()
  return (
    <section className="section" id="para-quem">
      <div className="wrap">
        <h2 className="section__title">{text.audience.title}</h2>
        <div className="audience">
          {text.audience.items.map(([title, body]) => (
            <div key={title}>
              <h3>{title}</h3>
              <p>{body}</p>
            </div>
          ))}
        </div>

        <div className="oss">
          <div>
            <h2 className="oss__title">{text.audience.ossTitle}</h2>
            <p className="lead">{text.audience.ossLead}</p>
            <a className="btn btn--ghost" href={GITHUB_URL}>
              {text.audience.ossLink}
            </a>
          </div>
          <pre className="oss__rule" aria-label="Exemplo de regra de proteção">
            <span className="k">- id:</span> <span className="s">ignorar-instrucoes</span>
            {'\n'}
            <span className="k">  quando:</span> pedido para ignorar as regras
            {'\n'}
            <span className="k">  acao:</span> <span className="s">bloquear</span>
            {'\n'}
            <span className="k">  exemplos:</span> ataques e textos legítimos
          </pre>
        </div>
      </div>
    </section>
  )
}
