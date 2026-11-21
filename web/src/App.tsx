import { useEffect, useState } from 'react'
import { Audience } from './components/Audience.tsx'
import { BeforeAfter } from './components/BeforeAfter.tsx'
import { Business } from './components/Business.tsx'
import { Dashboard } from './components/Dashboard.tsx'
import { Footer } from './components/Footer.tsx'
import { Hero } from './components/Hero.tsx'
import { HowItWorks } from './components/HowItWorks.tsx'
import { Nav } from './components/Nav.tsx'
import { Quiz } from './components/Quiz.tsx'
import { TryAttack } from './components/TryAttack.tsx'
import { WhyNow } from './components/WhyNow.tsx'

const PANEL_HASH = '#painel'

function useIsPanel() {
  const [isPanel, setIsPanel] = useState(() => window.location.hash === PANEL_HASH)
  useEffect(() => {
    const update = () => setIsPanel(window.location.hash === PANEL_HASH)
    window.addEventListener('hashchange', update)
    return () => window.removeEventListener('hashchange', update)
  }, [])
  return isPanel
}

export default function App() {
  const isPanel = useIsPanel()

  useEffect(() => {
    window.scrollTo(0, 0)
  }, [isPanel])

  if (isPanel) return <Dashboard />

  return (
    <>
      <Nav />
      <main>
        <Hero />
        <WhyNow />
        <TryAttack />
        <BeforeAfter />
        <HowItWorks />
        <Quiz />
        <Audience />
        <Business />
      </main>
      <Footer />
    </>
  )
}
