import fs from 'node:fs'
import vue from '@vitejs/plugin-vue'
import { defineConfig, type Plugin } from 'vite'

// While the dev server runs, public/hot holds its URL. The app's `vite` crate
// reads it per request and points asset tags at the dev server.
function hotFile(): Plugin {
  const remove = () => fs.rmSync('public/hot', { force: true })
  return {
    name: 'hot-file',
    configureServer(server) {
      server.httpServer?.once('listening', () => {
        fs.writeFileSync('public/hot', `http://localhost:${server.config.server.port}`)
      })
      process.once('exit', remove)
      for (const signal of ['SIGINT', 'SIGTERM'] as const) {
        process.once(signal, () => process.exit())
      }
    },
  }
}

export default defineConfig(({ command, isSsrBuild }) => ({
  base: command === 'build' ? '/build/' : '/',
  publicDir: false,
  plugins: [vue(), hotFile()],
  // The SSR bundle (`vite build --ssr resources/js/ssr.ts`) runs in Node: `npm run ssr`.
  build: isSsrBuild
    ? { outDir: 'bootstrap/ssr' }
    : { manifest: true, outDir: 'public/build', rollupOptions: { input: 'resources/js/app.ts' } },
  server: { port: 5173, strictPort: true, origin: 'http://localhost:5173' },
}))
