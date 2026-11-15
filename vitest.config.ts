import { existsSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { defineConfig } from 'vitest/config'
import { playwright } from '@vitest/browser-playwright'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// Motor de WebView2 (EdgeCore, versión más reciente) si existe; si no, el Edge estándar.
function edgeLaunchOptions() {
  const core = join(process.env['ProgramFiles(x86)'] ?? '', 'Microsoft', 'EdgeCore')
  const version = existsSync(core)
    ? readdirSync(core)
        .filter((d) => existsSync(join(core, d, 'msedge.exe')))
        .sort((a, b) => a.localeCompare(b, undefined, { numeric: true }))
        .pop()
    : undefined
  return version ? { executablePath: join(core, version, 'msedge.exe') } : { channel: 'msedge' }
}

export default defineConfig({
  test: {
    projects: [
      { plugins: [svelte()], test: { name: 'unit', environment: 'node', include: ['src/**/*.test.ts'] } },
      { plugins: [svelte()], test: { name: 'layout', include: ['tests/layout/**/*.test.ts'], browser: { enabled: true, headless: true, provider: playwright({ launchOptions: edgeLaunchOptions() }), instances: [{ browser: 'chromium' }] } } },
    ],
  },
})
