# EP00 — Fundação e pipeline de release

**Objetivo:** antes da primeira feature, ter repositório, CI e pipeline de release completos, para que toda mudança já saia versionada (SemVer), com CHANGELOG e instaladores assinados das 3 plataformas.

**Release alvo:** v0.1.0 (Sprint 0)

---

### TL-001 — Monorepo Tauri 2 + Vue 3 + TypeScript
**Como** mantenedor **quero** o esqueleto do projeto criado **para** começar a desenvolver sobre uma base padronizada.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** — · **Release:** v0.1.0

**Critérios de aceite**
- [ ] Estrutura: `src-tauri/` (Rust, workspace com crates `texlocal-core`, `texlocal-git`, `texlocal-tex`), `src/` (Vue 3 + TS + Vite + Pinia + Vue Router), `docs/`.
- [ ] `pnpm tauri dev` abre uma janela em Linux, Windows e macOS.
- [ ] Toolchains fixadas: `rust-toolchain.toml`, `packageManager` no `package.json`, `.nvmrc`.
- [ ] Licença MIT, `README.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`.

### TL-002 — Fim de linha LF em todo o repositório (dev no Windows)
**Como** mantenedor que desenvolve no Windows **quero** que todos os arquivos usem fim de linha LF **para** evitar diffs ruidosos, scripts quebrados e builds inconsistentes entre SOs.

- **Prioridade:** Must · **Estimativa:** 2 · **Depende de:** TL-001 · **Release:** v0.1.0

**Critérios de aceite**
- [ ] `.gitattributes` com `* text=auto eol=lf` e binários explícitos (`*.png binary`, `*.ico binary`, `*.icns binary`, `*.pdf binary`, `*.woff2 binary`).
- [ ] `.editorconfig` com `end_of_line = lf`, `charset = utf-8`, `insert_final_newline = true`.
- [ ] Prettier com `endOfLine: "lf"`; `rustfmt.toml` com `newline_style = "Unix"`.
- [ ] CONTRIBUTING documenta para Windows: `git config core.autocrlf false` (ou `input`) e configuração do VS Code (`"files.eol": "\n"`).
- [ ] Job de CI falha se algum arquivo de texto rastreado contém CRLF (`git ls-files --eol` / `git grep -I --files-with-matches $'\r'`).
- [ ] Clonar no Windows e rodar `pnpm tauri dev` não gera alterações no `git status`.

### TL-003 — Lint, formatação e hooks locais
**Como** mantenedor **quero** padronização automática de código **para** manter a qualidade sem revisão manual de estilo.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-001 · **Release:** v0.1.0

**Critérios de aceite**
- [ ] Rust: `rustfmt` + `clippy -D warnings`.
- [ ] Frontend: ESLint (vue + ts) + Prettier + `vue-tsc --noEmit`.
- [ ] Hooks com `lefthook` (multiplataforma, funciona no Windows): pre-commit formata/linta arquivos staged; commit-msg valida Conventional Commits.

### TL-004 — Conventional Commits obrigatórios
**Como** mantenedor **quero** commits padronizados **para** que versão e CHANGELOG sejam derivados automaticamente.

- **Prioridade:** Must · **Estimativa:** 2 · **Depende de:** TL-003 · **Release:** v0.1.0

**Critérios de aceite**
- [ ] `commitlint` com `@commitlint/config-conventional`, escopos permitidos documentados (`editor`, `tex`, `git`, `compile`, `snippets`, `ui`, `ci`, `release`, ...).
- [ ] CI valida título do PR (squash merge) e commits.
- [ ] `BREAKING CHANGE:` / `!` documentado como gatilho de major.

### TL-005 — CI de verificação (3 SOs)
**Como** mantenedor **quero** CI em cada PR **para** garantir que nada quebra em nenhuma plataforma.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-003 · **Release:** v0.1.0

**Critérios de aceite**
- [ ] GitHub Actions com matriz `ubuntu-22.04`, `windows-latest`, `macos-latest` (arm64) + `macos-13` (x64) ou build universal.
- [ ] Jobs: lint, typecheck, `cargo test`, `vitest`, build Tauri de verificação, checagem LF (TL-002).
- [ ] Cache de cargo e pnpm; tempo total < 15 min.
- [ ] Branch `main` protegida: CI verde + 1 revisão obrigatórios.

### TL-006 — release-please com versão sincronizada
**Como** mantenedor **quero** que o release-please mantenha um PR de release **para** versionar com SemVer sem esforço manual.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-004 · **Release:** v0.1.0

**Critérios de aceite**
- [ ] `release-please-config.json` + `.release-please-manifest.json`, versão inicial `0.1.0`.
- [ ] Bump sincronizado em `package.json`, `src-tauri/Cargo.toml` (workspace) e `src-tauri/tauri.conf.json` (extra-files).
- [ ] `CHANGELOG.md` gerado com seções Features/Bug Fixes/BREAKING.
- [ ] Ao mergear o PR de release: tag `vX.Y.Z` criada e GitHub Release em rascunho.
- [ ] Pré-1.0: `feat` incrementa minor, `fix` incrementa patch (`bump-minor-pre-major`).

### TL-007 — Certificados e segredos de assinatura
**Como** mantenedor **quero** os certificados configurados como segredos **para** publicar instaladores confiáveis.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** — · **Release:** v0.1.0

**Critérios de aceite**
- [ ] Par de chaves do Tauri updater gerado; chave pública no `tauri.conf.json`, privada em segredo.
- [ ] Apple Developer ID Application + credenciais de notarização (API key) em segredos.
- [ ] Certificado Windows (Authenticode, preferencialmente Azure Trusted Signing ou EV) em segredos.
- [ ] Chave GPG para assinar o repositório APT e o `.deb`.
- [ ] Documento interno de rotação e expiração dos certificados.

### TL-008 — Workflow de build e publicação da release
**Como** mantenedor **quero** que a tag dispare build assinado das 3 plataformas **para** publicar a release automaticamente.

- **Prioridade:** Must · **Estimativa:** 8 · **Depende de:** TL-005, TL-006, TL-007 · **Release:** v0.1.0

**Critérios de aceite**
- [ ] Gatilho: tag `v*` criada pelo release-please.
- [ ] `tauri-action` gera: `.deb` e `.AppImage` (x86_64; arm64 como Could), `.msi` (x64), `.dmg` (universal ou arm64 + x64).
- [ ] macOS assinado e notarizado; Windows assinado; artefatos do updater assinados (`.sig`).
- [ ] `latest.json` do updater publicado junto à release.
- [ ] SHA-256 de todos os artefatos publicado em `SHA256SUMS`.
- [ ] Release sai de rascunho para publicada somente se todos os jobs passarem.
- [ ] Se segredos de assinatura de SO estiverem ausentes, build continua e a release é marcada "unsigned" nas notas.

### TL-009 — ADRs das decisões iniciais
**Como** mantenedor **quero** as decisões registradas como ADR **para** que futuros contribuidores entendam o porquê.

- **Prioridade:** Should · **Estimativa:** 2 · **Depende de:** TL-001 · **Release:** v0.1.0

**Critérios de aceite**
- [ ] `docs/adr/0001..` cobrindo D01–D26 de [decisoes.md](decisoes.md), formato MADR.

### TL-010 — Templates de issue/PR e Dependabot
**Como** mantenedor **quero** templates e atualização de dependências **para** manter o projeto organizado e seguro.

- **Prioridade:** Should · **Estimativa:** 2 · **Depende de:** TL-005 · **Release:** v0.1.0

**Critérios de aceite**
- [ ] Templates de bug (com SO, versão, log) e feature.
- [ ] Template de PR com checklist da DoD.
- [ ] Dependabot (cargo, npm, github-actions) com commits `chore(deps):`.
- [ ] `cargo audit` e `pnpm audit` no CI semanal.
