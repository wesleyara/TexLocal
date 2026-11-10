/** Escolha de ícones de arquivo e pasta no estilo do Material Icon Theme. */

const BY_EXTENSION: Record<string, string> = {
  tex: 'tex',
  sty: 'tex',
  cls: 'tex',
  ltx: 'tex',
  dtx: 'tex',
  ins: 'tex',
  bib: 'bibliography',
  bbl: 'bibliography',
  bst: 'bibtex-style',
  pdf: 'pdf',
  png: 'image',
  jpg: 'image',
  jpeg: 'image',
  gif: 'image',
  webp: 'image',
  bmp: 'image',
  ico: 'image',
  eps: 'image',
  svg: 'svg',
  json: 'json',
  md: 'markdown',
  markdown: 'markdown',
  yml: 'yaml',
  yaml: 'yaml',
  toml: 'toml',
  log: 'log',
  blg: 'log',
  zip: 'zip',
  gz: 'zip',
  tar: 'zip',
  '7z': 'zip',
  ts: 'typescript',
  js: 'javascript',
  mjs: 'javascript',
  py: 'python',
  rs: 'rust',
  html: 'html',
  css: 'css',
  vue: 'vue',
  lock: 'lock',
  csv: 'table',
  tsv: 'table',
  ttf: 'font',
  otf: 'font',
  woff: 'font',
  woff2: 'font',
  mp3: 'audio',
  wav: 'audio',
  mp4: 'video',
  sh: 'console',
  bat: 'console',
  ps1: 'powershell',
}

const BY_NAME: Record<string, string> = {
  'texlocal.toml': 'tune',
  latexmkrc: 'latexmk',
  '.latexmkrc': 'latexmk',
  '.gitignore': 'git',
  '.gitattributes': 'git',
  '.editorconfig': 'editorconfig',
  '.prettierrc': 'prettier',
  'package.json': 'npm',
  'pnpm-lock.yaml': 'pnpm',
  'vite.config.ts': 'vite',
  'readme.md': 'readme',
  readme: 'readme',
  license: 'license',
  'license.md': 'license',
  'changelog.md': 'changelog',
  'todo.md': 'todo',
}

const FOLDERS: Record<string, string> = {
  src: 'src',
  images: 'images',
  img: 'images',
  figures: 'images',
  figs: 'images',
  imagens: 'images',
  docs: 'docs',
  chapters: 'docs',
  capitulos: 'docs',
  sections: 'docs',
  dist: 'dist',
  build: 'dist',
  config: 'config',
  '.texlocal': 'config',
  node_modules: 'node',
  '.git': 'git',
  pdf: 'pdf',
  public: 'public',
  assets: 'public',
  scripts: 'scripts',
  lib: 'lib',
  utils: 'utils',
  tests: 'test',
  test: 'test',
  components: 'components',
  bib: 'bibliography',
  bibliography: 'bibliography',
  references: 'bibliography',
}

/** Chave em `ICONS` (ex.: `mit:tex`) para um arquivo. */
export function fileIconKey(name: string): string {
  const lower = name.toLowerCase()
  const byName = BY_NAME[lower]
  if (byName) return `mit:${byName}`
  const dot = lower.lastIndexOf('.')
  const ext = dot > 0 ? lower.slice(dot + 1) : ''
  return `mit:${BY_EXTENSION[ext] ?? 'document'}`
}

/** Chave em `ICONS` para uma pasta, aberta ou fechada. */
export function folderIconKey(name: string, open: boolean): string {
  const kind = FOLDERS[name.toLowerCase()] ?? 'base'
  return `mit:folder-${kind}${open ? '-open' : ''}`
}

/** Todas as chaves que o resolvedor pode devolver (usado nos testes). */
export function allReferencedKeys(): string[] {
  const files = new Set(['document', ...Object.values(BY_EXTENSION), ...Object.values(BY_NAME)])
  const folders = new Set(['base', ...Object.values(FOLDERS)])
  return [
    ...[...files].map((f) => `mit:${f}`),
    ...[...folders].flatMap((f) => [`mit:folder-${f}`, `mit:folder-${f}-open`]),
  ]
}
