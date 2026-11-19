import { defineConfig } from 'cypress'
import { createServer } from 'vite'

export default defineConfig({
  e2e: {
    supportFile: false,
    video: false,
    screenshotOnRunFailure: true,
    // Sobe o Vite junto com o Cypress: `npm run test:e2e` não precisa de outro terminal.
    async setupNodeEvents(_on, config) {
      const server = await createServer({ server: { port: 5173 } })
      await server.listen()
      config.baseUrl = server.resolvedUrls?.local[0] ?? null
      return config
    },
  },
})
