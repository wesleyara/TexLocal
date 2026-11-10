// Gera src/icons/generated.ts com um subconjunto embutido de ícones:
//  - "mdi:*": ícones de interface (Material Design Icons, 24x24, usam currentColor);
//  - "mit:*": ícones de arquivo/pasta do Material Icon Theme (16x16, coloridos).
// Rodar com `node scripts/gen-icons.mjs` após mudar as listas abaixo.
import { createRequire } from 'node:module'
import { writeFileSync } from 'node:fs'

const require = createRequire(import.meta.url)
const mdi = require('@iconify-json/mdi/icons.json')
const mit = require('@iconify-json/material-icon-theme/icons.json')

const UI = [
  'file-tree',
  'magnify',
  'history',
  'source-branch',
  'cog-outline',
  'play',
  'stop',
  'download',
  'bell-outline',
  'bell-badge-outline',
  'close',
  'chevron-right',
  'chevron-down',
  'plus',
  'minus',
  'file-plus-outline',
  'folder-plus-outline',
  'refresh',
  'alert-circle',
  'alert',
  'check-circle',
  'information',
  'sync',
  'timer-outline',
  'cancel',
  'format-list-bulleted',
  'view-split-vertical',
  'page-layout-sidebar-left',
  'text-search',
  'format-section',
  'image-outline',
  'table',
  'checkbox-blank-circle-outline',
  'folder-open-outline',
  'chevron-up',
  'chevron-left',
  'magnify-plus-outline',
  'arrow-expand-horizontal',
  'fit-to-page-outline',
  'link-variant',
  'crosshairs-gps',
  'view-grid-outline',
  'stop-circle-outline',
  'close-circle-outline',
]

const FILES = [
  'document',
  'tex',
  'bibliography',
  'bibtex-style',
  'latexmk',
  'pdf',
  'image',
  'svg',
  'json',
  'markdown',
  'yaml',
  'toml',
  'log',
  'zip',
  'git',
  'settings',
  'tune',
  'typescript',
  'javascript',
  'python',
  'rust',
  'html',
  'css',
  'vue',
  'lock',
  'readme',
  'license',
  'changelog',
  'table',
  'font',
  'audio',
  'video',
  'console',
  'powershell',
  'editorconfig',
  'prettier',
  'eslint',
  'npm',
  'pnpm',
  'vite',
  'tauri',
  'todo',
  'folder-base',
  'folder-base-open',
  'folder-src',
  'folder-src-open',
  'folder-images',
  'folder-images-open',
  'folder-docs',
  'folder-docs-open',
  'folder-dist',
  'folder-dist-open',
  'folder-config',
  'folder-config-open',
  'folder-node',
  'folder-node-open',
  'folder-git',
  'folder-git-open',
  'folder-pdf',
  'folder-pdf-open',
  'folder-public',
  'folder-public-open',
  'folder-scripts',
  'folder-scripts-open',
  'folder-lib',
  'folder-lib-open',
  'folder-test',
  'folder-test-open',
  'folder-utils',
  'folder-utils-open',
  'folder-components',
  'folder-components-open',
  'folder-bibliography',
  'folder-bibliography-open',
]

function pick(set, names, prefix, size) {
  const out = {}
  const missing = []
  for (const n of names) {
    const icon = set.icons[n]
    if (!icon) {
      missing.push(n)
      continue
    }
    out[`${prefix}:${n}`] = { body: icon.body, w: icon.width ?? size, h: icon.height ?? size }
  }
  if (missing.length) throw new Error(`ícones ausentes em ${prefix}: ${missing.join(', ')}`)
  return out
}

const icons = { ...pick(mdi, UI, 'mdi', 24), ...pick(mit, FILES, 'mit', 16) }
const body = `// Gerado por scripts/gen-icons.mjs — não edite à mão.
// Material Design Icons (Apache-2.0) e Material Icon Theme (MIT), via @iconify-json.
export interface IconData {
  body: string
  w: number
  h: number
}

export const ICONS: Record<string, IconData> = ${JSON.stringify(icons, null, 2)}
`
writeFileSync(new URL('../src/icons/generated.ts', import.meta.url), body)
console.log(`${Object.keys(icons).length} ícones gerados`)
