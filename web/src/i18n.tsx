import { createContext, useContext, useEffect, useState } from 'react'
import type { ReactNode } from 'react'

export type Lang = 'pt' | 'en'

const copy = {
  pt: {
    nav: {
      home: 'VoidGate, início',
      sections: 'Seções',
      test: 'Teste um ataque',
      how: 'Como funciona',
      who: 'Para quem',
      plans: 'Planos',
      lang: 'EN',
      panel: 'Painel',
    },
    hero: {
      title: 'O firewall que fala a língua da IA.',
      lead: 'O VoidGate fica entre o seu chatbot e qualquer modelo. Ele barra manipulação em vários idiomas e esconde dados pessoais antes que vazem.',
      test: 'Testar um ataque',
      code: 'Ver o código no GitHub',
    },
    why: {
      title: 'Toda empresa está colocando IA no atendimento. Quase nenhuma está protegendo.',
      lead: 'Um chatbot pode ser convencido, com uma frase, a ignorar as regras e entregar dados de clientes. O risco é de quem opera o atendimento.',
      stats: [
        {
          value: '#1',
          text: 'Injeção de prompt é o risco número 1 da lista OWASP Top 10 para aplicações com modelos de linguagem.',
        },
        {
          value: 'GDPR',
          text: 'A mesma camada serve à LGPD, ao GDPR e às leis locais. A multa europeia chega a 4% do faturamento mundial.',
        },
        {
          value: 'A–Z',
          text: 'A frase de ataque muda de idioma. O VoidGate procura a manipulação em português, inglês, espanhol, francês, alemão e chinês, e esconde e-mail, telefone, cartão, IBAN, CPF e SSN.',
        },
      ],
    },
    how: {
      title: 'Como funciona',
      lead: 'Aponte o chatbot para o VoidGate. O endereço do modelo fica na configuração.',
      steps: [
        ['Sua aplicação envia', 'O chatbot manda a mensagem do cliente para o VoidGate.', 'recebendo'],
        ['O VoidGate confere', 'Procura manipulação e dados pessoais, e decide: liberar, esconder ou bloquear.', 'conferindo a mensagem'],
        ['A IA responde', 'Só o que é seguro chega ao modelo.', 'mensagem liberada'],
        ['A resposta é conferida', 'Se a IA tentar devolver um dado sensível, ele é escondido.', 'conferindo a resposta'],
      ],
      note: 'Funciona com qualquer modelo. Troque e veja que nada muda no VoidGate:',
      models: ['Modelo na nuvem', 'Modelo da empresa', 'Modelo local'],
      diagram: 'conectado ao VoidGate',
    },
    audience: {
      title: 'Feito para quem coloca IA no atendimento, em qualquer país.',
      items: [
        ['Empresas', 'Colocam um chatbot no ar sem entregar dados de clientes ao modelo.'],
        ['Setor público', 'Usam IA no atendimento e precisam de registro do que foi bloqueado.'],
        ['Desenvolvedores', 'Trocam o endereço do modelo e ganham a camada, sem reescrever o chatbot.'],
      ],
      ossTitle: 'Aberto para qualquer um conferir.',
      ossLead: 'Em segurança, ninguém deveria confiar numa caixa-preta. O código do VoidGate é público, e qualquer pessoa pode propor novas regras de proteção.',
      ossLink: 'Ver o código no GitHub',
    },
    business: {
      title: 'Código aberto no núcleo. Receita na nuvem.',
      lead: 'Quem quer pode rodar de graça. Quem não quer cuidar de servidor paga pelo serviço gerenciado.',
      plans: [
        ['Comunidade', 'Grátis', ['Código aberto', 'Roda no seu servidor', 'Regras públicas e auditáveis']],
        ['Nuvem', 'Por volume de mensagens', ['VoidGate gerenciado, sem servidor', 'Painel com o que foi bloqueado', 'Relatório para GDPR e LGPD']],
        ['Empresas e governo', 'Sob consulta', ['Instalação no seu ambiente', 'Regras sob medida para o seu país', 'SLA e suporte dedicado']],
      ],
      roadmapTitle: 'Onde estamos',
      roadmap: [
        ['Já funciona', 'API que bloqueia manipulação e texto escondido, mascara e-mail, telefone, cartão, IBAN, CPF e SSN, e um proxy: trocar o endereço do modelo.'],
        ['Próximo', 'Painel, relatório de conformidade e várias empresas na mesma instalação.'],
        ['Depois', 'Ambiente-isca para estudar atacantes.'],
      ],
      ctaTitle: 'Quer proteger o chatbot da sua empresa?',
      ctaLead: 'Estamos procurando empresas para o piloto e parceiros para crescer.',
      cta: 'Falar com o time',
      try: 'Testar um ataque agora',
    },
    panel: {
      navLabel: 'Páginas',
      site: 'Site',
      current: 'Painel',
      demo: 'Modo demonstração: dados simulados',
      pause: 'Pausar',
      resume: 'Retomar',
      title: 'Painel de monitoramento',
      lead: 'Como o VoidGate está protegendo o chatbot agora.',
      window: 'Últimos 30 minutos',
      totals: {
        analyzed: 'mensagens analisadas',
        blocked: 'bloqueadas',
        masked: 'com dados escondidos',
        allowed: 'liberadas sem mudança',
      },
      statusLabel: 'Situação por tipo de ameaça',
      categories: {
        prompt_injection: 'Prompt injection',
        sensitive_data: 'Informação indevida',
        file_fraud: 'Fraude por arquivo',
      },
      levels: { calm: 'Calmo', attention: 'Atenção', attack: 'Sob ataque' },
      inWindow: 'nos últimos 15 min',
      peak: 'Pico às',
      top: 'Mais comum:',
      last: 'Último às',
      none: 'Nada nos últimos 15 min',
      rules: { cpf: 'CPF', email: 'e-mail', card: 'cartão', phone: 'telefone', iban: 'IBAN', ssn: 'SSN' },
      river: {
        label: 'Cada ponto é uma mensagem chegando. As seguras seguem pelo rio de cima, as com dados pessoais seguem escondidas pelo de baixo, e os ataques param no portão.',
        arriving: 'Chegando agora',
        passed: 'Seguiram para a IA',
        blocked: 'bloqueadas em 5 min',
      },
      chart: {
        title: 'Ameaças por minuto',
        label: 'Ameaças por minuto nos últimos 30 minutos.',
      },
      events: {
        title: 'Eventos',
        filterLabel: 'Filtrar eventos',
        filters: { all: 'Todos', block: 'Bloqueados', mask: 'Escondidos' },
        empty: 'Nenhum evento neste filtro nos últimos 30 minutos.',
      },
      actions: { block: 'Bloqueado', mask: 'Escondido', allow: 'Liberado' },
      detail: {
        prompt_injection: {
          title: 'Tentativa de manipular a IA',
          text: 'A mensagem tentava fazer a IA desobedecer as regras da empresa, trocar de papel ou revelar as instruções internas.',
        },
        sensitive_data: {
          title: 'Dado pessoal na mensagem',
          text: 'O cliente mandou um dado pessoal. O VoidGate escondeu o dado antes de a mensagem chegar à IA.',
        },
        file_fraud: {
          title: 'Instrução escondida em arquivo',
          text: 'O arquivo enviado tinha um texto invisível para quem lê, com uma ordem para a IA.',
        },
        outputTitle: 'Dado pessoal na resposta da IA',
        outputText: 'A IA ia devolver um dado pessoal ao cliente. O VoidGate escondeu o dado antes de a resposta sair.',
        file: 'Arquivo',
        page: 'página',
        hidden: 'Trecho escondido encontrado',
        message: 'Mensagem recebida (dados pessoais já escondidos)',
        answer: 'Resposta da IA, como o cliente recebeu',
        received: 'O que a IA recebeu',
        receivedNothing: 'Nada. A mensagem foi barrada antes de chegar ao modelo.',
        time: 'Horário',
        rule: 'Regra',
        channel: 'Canal',
        risk: 'Risco',
        risks: { high: 'Alto', medium: 'Médio', low: 'Baixo' },
      },
    },
    footer: 'Feito no Amazonas para o HackBrasil 2026.',
    try: {
      title: 'Tente enganar a IA.',
      lead: 'Escreva uma mensagem ou escolha um exemplo. Veja na hora o que o VoidGate faria com ela.',
      examples: 'Exemplos',
      message: 'Mensagem para o chatbot',
      placeholder: 'Escreva aqui o que você mandaria para um chatbot.',
      read: 'Como o VoidGate leu',
      empty: 'Nada para analisar ainda.',
      note: 'Simulação no navegador, com as mesmas classes de regra da API. Nada do que você escreve sai do seu computador.',
      waiting: 'Aguardando',
      out: 'O que a IA receberia',
      blocked: 'Nada. A mensagem seria barrada antes de chegar ao modelo.',
      chips: ['Pedir para ignorar as regras', 'Mandar dados pessoais', 'Pergunta comum', 'Texto escondido'],
      verdicts: {
        block: ['Bloqueado', 'Risco alto'],
        mask: ['Liberado com dados escondidos', 'Atenção'],
        allow: ['Liberado', 'Sem risco encontrado'],
      },
      kinds: {
        manipulation: 'Tentativa de manipular a IA',
        invisible: 'Caracteres invisíveis',
        cpf: 'CPF encontrado',
        email: 'E-mail encontrado',
        phone: 'Telefone encontrado',
        card: 'Cartão encontrado',
        iban: 'IBAN encontrado',
      },
    },
  },
  en: {
    nav: {
      home: 'VoidGate, home',
      sections: 'Sections',
      test: 'Try an attack',
      how: 'How it works',
      who: 'Who it is for',
      plans: 'Plans',
      lang: 'PT',
      panel: 'Panel',
    },
    hero: {
      title: 'The firewall between your app and any model.',
      lead: 'VoidGate sits between your chatbot and the model. It blocks manipulation in several languages and hides personal data before it leaks.',
      test: 'Try an attack',
      code: 'See the code on GitHub',
    },
    why: {
      title: 'Every company is putting AI on the front line. Almost none of them are protecting it.',
      lead: 'One sentence can talk a chatbot into ignoring the rules and handing over customer data. The risk belongs to the organization that runs the bot.',
      stats: [
        {
          value: '#1',
          text: 'Prompt injection is risk number 1 on the OWASP Top 10 for applications that use language models.',
        },
        {
          value: 'GDPR',
          text: 'The same layer fits GDPR, Brazil’s LGPD and local privacy law. The European fine reaches 4% of worldwide turnover.',
        },
        {
          value: 'A–Z',
          text: 'The attack changes language. VoidGate looks for manipulation in Portuguese, English, Spanish, French, German and Chinese, and hides email, phone, card, IBAN, CPF and SSN.',
        },
      ],
    },
    how: {
      title: 'How it works',
      lead: 'Point the chatbot at VoidGate. The model address stays in configuration.',
      steps: [
        ['Your app sends', 'The chatbot hands the customer message to VoidGate.', 'receiving'],
        ['VoidGate checks', 'It looks for manipulation and personal data, then allows, hides or blocks.', 'checking the message'],
        ['The model answers', 'Only what is safe reaches the model.', 'message allowed'],
        ['The answer is checked', 'If the model tries to return sensitive data, it is hidden.', 'checking the answer'],
      ],
      note: 'It works with any model. Switch and see that VoidGate does not change:',
      models: ['Cloud model', 'Company model', 'Local model'],
      diagram: 'connected to VoidGate',
    },
    audience: {
      title: 'For teams putting AI in front of customers, in any country.',
      items: [
        ['Companies', 'Ship a chatbot without handing customer data to the model.'],
        ['Public sector', 'Use AI in service and keep a record of what was blocked.'],
        ['Developers', 'Change the model address and get the layer, without rewriting the bot.'],
      ],
      ossTitle: 'Open for anyone to check.',
      ossLead: 'In security, nobody should trust a black box. The VoidGate code is public, and anyone can propose a new protection rule.',
      ossLink: 'See the code on GitHub',
    },
    business: {
      title: 'Open core. Revenue in the cloud.',
      lead: 'Run it yourself for free. Pay for the managed service if you do not want to run a server.',
      plans: [
        ['Community', 'Free', ['Open source', 'Runs on your server', 'Public, auditable rules']],
        ['Cloud', 'Per message volume', ['Managed VoidGate, no server', 'A panel of what was blocked', 'A report for GDPR and LGPD']],
        ['Enterprise and government', 'On request', ['Installed in your environment', 'Rules for your country', 'SLA and dedicated support']],
      ],
      roadmapTitle: 'Where we are',
      roadmap: [
        ['Working now', 'An API that blocks manipulation and hidden text, masks email, phone, card, IBAN, CPF and SSN, and a proxy: change the model address.'],
        ['Next', 'A panel, a compliance report and several companies on one install.'],
        ['Later', 'A decoy environment for studying attackers.'],
      ],
      ctaTitle: 'Want to protect your company’s chatbot?',
      ctaLead: 'We are looking for pilot companies and partners.',
      cta: 'Talk to the team',
      try: 'Try an attack now',
    },
    panel: {
      navLabel: 'Pages',
      site: 'Site',
      current: 'Panel',
      demo: 'Demo mode: simulated data',
      pause: 'Pause',
      resume: 'Resume',
      title: 'Monitoring panel',
      lead: 'How VoidGate is protecting the chatbot right now.',
      window: 'Last 30 minutes',
      totals: {
        analyzed: 'messages analyzed',
        blocked: 'blocked',
        masked: 'with data hidden',
        allowed: 'let through unchanged',
      },
      statusLabel: 'Status by threat type',
      categories: {
        prompt_injection: 'Prompt injection',
        sensitive_data: 'Sensitive data',
        file_fraud: 'File fraud',
      },
      levels: { calm: 'Calm', attention: 'Watch', attack: 'Under attack' },
      inWindow: 'in the last 15 min',
      peak: 'Peak at',
      top: 'Most common:',
      last: 'Last at',
      none: 'Nothing in the last 15 min',
      rules: { cpf: 'CPF', email: 'email', card: 'card', phone: 'phone', iban: 'IBAN', ssn: 'SSN' },
      river: {
        label: 'Each dot is an incoming message. Safe ones follow the upper river, ones with personal data follow the lower one already hidden, and attacks stop at the gate.',
        arriving: 'Arriving now',
        passed: 'Went on to the AI',
        blocked: 'blocked in 5 min',
      },
      chart: {
        title: 'Threats per minute',
        label: 'Threats per minute over the last 30 minutes.',
      },
      events: {
        title: 'Events',
        filterLabel: 'Filter events',
        filters: { all: 'All', block: 'Blocked', mask: 'Hidden' },
        empty: 'No events in this filter in the last 30 minutes.',
      },
      actions: { block: 'Blocked', mask: 'Hidden', allow: 'Allowed' },
      detail: {
        prompt_injection: {
          title: 'Attempt to manipulate the AI',
          text: 'The message tried to make the AI break the company rules, take on another role or reveal its internal instructions.',
        },
        sensitive_data: {
          title: 'Personal data in the message',
          text: 'The customer sent personal data. VoidGate hid it before the message reached the AI.',
        },
        file_fraud: {
          title: 'Instruction hidden in a file',
          text: 'The uploaded file had text invisible to readers, carrying an order for the AI.',
        },
        outputTitle: 'Personal data in the AI answer',
        outputText: 'The AI was about to send personal data to the customer. VoidGate hid it before the answer went out.',
        file: 'File',
        page: 'page',
        hidden: 'Hidden text found',
        message: 'Message received (personal data already hidden)',
        answer: 'AI answer, as the customer received it',
        received: 'What the AI received',
        receivedNothing: 'Nothing. The message was stopped before it reached the model.',
        time: 'Time',
        rule: 'Rule',
        channel: 'Channel',
        risk: 'Risk',
        risks: { high: 'High', medium: 'Medium', low: 'Low' },
      },
    },
    footer: 'Built in Amazonas, Brazil, for HackBrasil 2026.',
    try: {
      title: 'Try to fool the model.',
      lead: 'Write a message or pick an example. See what VoidGate would do with it.',
      examples: 'Examples',
      message: 'Message for the chatbot',
      placeholder: 'Write what you would send to a chatbot.',
      read: 'How VoidGate read it',
      empty: 'Nothing to analyze yet.',
      note: 'A browser simulation, using the same rule classes as the API. Nothing you type leaves this computer.',
      waiting: 'Waiting',
      out: 'What the model would receive',
      blocked: 'Nothing. The message would be stopped before the model.',
      chips: ['Ask it to ignore the rules', 'Send personal data', 'An ordinary question', 'Hidden text'],
      verdicts: {
        block: ['Blocked', 'High risk'],
        mask: ['Allowed with data hidden', 'Caution'],
        allow: ['Allowed', 'No risk found'],
      },
      kinds: {
        manipulation: 'Attempt to manipulate the model',
        invisible: 'Invisible characters',
        cpf: 'CPF found',
        email: 'Email found',
        phone: 'Phone found',
        card: 'Card found',
        iban: 'IBAN found',
      },
    },
  },
} as const

export type Copy = (typeof copy)['pt']

const LangContext = createContext<{ lang: Lang; text: Copy; toggle: () => void } | null>(null)

export function LangProvider({ children }: { children: ReactNode }) {
  const [lang, setLang] = useState<Lang>('pt')
  const text = copy[lang] as Copy
  const toggle = () => setLang((current) => (current === 'pt' ? 'en' : 'pt'))
  useEffect(() => {
    document.documentElement.lang = lang === 'pt' ? 'pt-BR' : 'en'
  }, [lang])
  return <LangContext.Provider value={{ lang, text, toggle }}>{children}</LangContext.Provider>
}

export function useLang() {
  const value = useContext(LangContext)
  if (!value) throw new Error('useLang outside LangProvider')
  return value
}
