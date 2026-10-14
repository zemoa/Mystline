import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  // Les URL absolues désignent les fichiers de public, servis comme images.
  // Les importer en JavaScript bloque le premier chargement dans WebKitGTK.
  plugins: [vue({ template: { transformAssetUrls: { includeAbsolute: false } } })],
  clearScreen: false,
  server: { host: '127.0.0.1', port: 1420, strictPort: true },
})
