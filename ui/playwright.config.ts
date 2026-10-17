import { defineConfig, devices } from '@playwright/test'

// E2E memakai UI di browser biasa: backend diganti browser-ledger in-memory (kosong di setiap page load).
export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? 'github' : 'list',
  use: { baseURL: 'http://localhost:5173', trace: 'retain-on-failure' },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: { command: 'pnpm dev', port: 5173, reuseExistingServer: true },
})
