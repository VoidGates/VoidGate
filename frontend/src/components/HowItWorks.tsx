import { useState } from 'react'

import { useLang } from '../i18n.tsx'

/** Where the moving dot sits in the diagram for each step. */
const DOT_POSITION = [
  { x: 235, y: 100 },
  { x: 380, y: 166 },
  { x: 525, y: 100 },
  { x: 235, y: 140 },
]

export function HowItWorks() {
  const { text } = useLang()
  const steps = text.how.steps.map(([title, body, status]) => ({
    title,
    text: body,
    status,
  }))
  const models = text.how.models
  const [step, setStep] = useState(1)
  const [modelIndex, setModelIndex] = useState(0)
  const model = models[modelIndex]
  const dot = DOT_POSITION[step]

  return (
    <section className="section" id="como-funciona">
      <div className="wrap">
        <h2 className="section__title">{text.how.title}</h2>
        <p className="lead">{text.how.lead}</p>

        <div className="how">
          <ol className="how__steps">
            {steps.map((item, index) => (
              <li key={item.title}>
                <button
                  type="button"
                  className="how__step"
                  aria-pressed={step === index}
                  onClick={() => setStep(index)}
                >
                  <span className="how__number">{index + 1}</span>
                  <b>{item.title}</b>
                  <span className="how__text">{item.text}</span>
                </button>
              </li>
            ))}
          </ol>

          <div className="panel how__panel">
            <svg
              className="how__diagram"
              viewBox="0 0 760 220"
              role="img"
              aria-label={`Etapa ${step + 1}: ${steps[step].title}. ${model} ${text.how.diagram}.`}
            >
              <line
                className={`how__line ${step === 0 ? 'is-active' : ''}`}
                x1="180"
                y1="100"
                x2="290"
                y2="100"
              />
              <line
                className={`how__line ${step === 2 ? 'is-active' : ''}`}
                x1="470"
                y1="100"
                x2="580"
                y2="100"
              />
              <line
                className={`how__line how__line--return ${step === 3 ? 'is-active' : ''}`}
                x1="180"
                y1="140"
                x2="580"
                y2="140"
              />

              <rect className="how__node" x="0" y="70" width="180" height="90" rx="18" />
              <text className="how__label" x="90" y="121" textAnchor="middle">
                Sua aplicação
              </text>

              <rect
                className={`how__gate ${step === 1 || step === 3 ? 'is-busy' : ''}`}
                x="290"
                y="45"
                width="180"
                height="140"
                rx="22"
              />
              <text className="how__gate-title" x="380" y="108" textAnchor="middle">
                VoidGate
              </text>
              <text className="how__gate-status" x="380" y="134" textAnchor="middle">
                {steps[step].status}
              </text>

              <rect className="how__node" x="580" y="70" width="180" height="90" rx="18" />
              <text className="how__label" x="670" y="121" textAnchor="middle">
                {model}
              </text>

              <circle
                className="how__dot"
                r="8"
                style={{ transform: `translate(${dot.x}px, ${dot.y}px)` }}
              />
            </svg>

            <p className="note">{text.how.note}</p>
            <div className="chips" role="group" aria-label="Modelo de IA">
              {models.map((name, index) => (
                <button
                  key={name}
                  type="button"
                  className="chip"
                  aria-pressed={modelIndex === index}
                  onClick={() => setModelIndex(index)}
                >
                  {name}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
