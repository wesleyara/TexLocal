// Falha se algum arquivo de texto rastreado contém CRLF (TL-002).
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'

const entries = execFileSync('git', ['ls-files', '-z', '--eol'], { encoding: 'utf8' })
  .split('\0')
  .filter(Boolean)
  .map((line) => {
    const [info, path] = line.split('\t')
    return { info, path }
  })
  .filter(({ info }) => !info.includes('-text'))

const bad = entries.filter(({ info, path }) => {
  if (/i\/(crlf|mixed)/.test(info)) return true
  try {
    return readFileSync(path).includes(13)
  } catch {
    return false
  }
})

if (bad.length) {
  console.error('Arquivos com CRLF:\n' + bad.map((f) => ' - ' + f.path).join('\n'))
  process.exit(1)
}
console.log('OK: nenhum arquivo rastreado com CRLF')
