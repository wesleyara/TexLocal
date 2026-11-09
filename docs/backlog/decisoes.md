# Decisões

Decisões tomadas na sessão de sabatina (2026-11-09). Cada uma vira um ADR em `docs/adr/` durante a EP00 (TL-009).

| # | Tema | Decisão | Alternativas descartadas |
|---|---|---|---|
| D01 | Referência de UX | Overleaf (histórico, versões nomeadas, diff, restaurar), com git real por baixo | — |
| D02 | Stack | **Tauri 2** + backend **Rust** + frontend **Vue 3 + TypeScript** (Vite, Pinia) | Electron, Qt, UI nativa Rust (Iced/Slint/egui) |
| D03 | Editor | **CodeMirror 6** | Monaco |
| D04 | Preview | **PDF.js** com **SyncTeX** bidirecional | Render nativo via pdfium |
| D05 | Plataformas | **Linux, Windows e macOS no MVP** (.deb + AppImage, .msi, .dmg) | Linux primeiro |
| D06 | Distribuição TeX | **TeX Live gerenciado** pelo app (instalação no diretório de dados do usuário, `tlmgr` para pacotes); usa TeX do sistema se detectado e o usuário preferir | Tectonic, híbrido |
| D07 | Global x projeto | **Instalação TeX global compartilhada** + **manifesto `texlocal.toml` por projeto** (motor, ano do TeX Live, pacotes extras, arquivo principal), versionado no git | Árvore TeX isolada por projeto |
| D08 | Onboarding TeX | **Wizard com perfis**: Mínimo (~150 MB), Recomendado (~800 MB), Completo (~5 GB); download em background, espelho CTAN automático | Sempre mínimo, TeX embutido no instalador |
| D09 | Compilação | **latexmk** automático (salvar/debounce), erros inline, **auto-instalação de pacotes faltantes** detectados no log | — |
| D10 | Git | **Git embutido** (crate `git2`/libgit2, sem exigir git no sistema). Local + remotos + branches básicos | Só local; cliente completo com rebase/stash |
| D11 | Commits | **Auto-checkpoint** após inatividade com mudanças + **versões nomeadas** (tags anotadas) | Só manual; commit por compilação |
| D12 | Histórico | Diff de fonte, **diff visual de PDF**, restaurar arquivo/projeto (sempre como novo commit), exportar versão (zip/PDF) | — |
| D13 | Auth remota | **OAuth device flow (GitHub/GitLab)**, **PAT** e **SSH**; segredos no keyring do SO | — |
| D14 | Conflitos | **Editor de merge 3-way completo no MVP** | Só fast-forward; resolução básica |
| D15 | Snippets | **Global + projeto**, formato **JSON compatível com VS Code**, editor na UI, biblioteca padrão | UltiSnips, só embutidos |
| D16 | Projetos | **Qualquer pasta** é projeto; lista de recentes; `git init` opcional ao abrir | Workspace gerenciado |
| D17 | Recursos de projeto no MVP | Templates (incl. ABNT/abntex2), bibliografia (.bib, `\cite`, BibTeX/biber, DOI), outline + `\ref`/`\label` + multi-arquivo, corretor ortográfico (Hunspell pt-BR/en) | — |
| D18 | Design | **IDE moderna**: escuro por padrão (com tema claro), densa, command palette central. Layout: activity bar · sidebar · editor · PDF | Editorial, minimalista |
| D19 | i18n e licença | Interface **pt-BR e en** desde o início; backlog em pt-BR; licença **MIT** | GPL-3.0 |
| D20 | Telemetria | **Nenhuma**; relatório de crash gerado localmente, enviado pelo usuário se quiser | Opt-in anônima |
| D21 | Versionamento | **SemVer** + **Conventional Commits** (validado no CI) + **release-please** (CHANGELOG, bump sincronizado em `Cargo.toml`/`package.json`/`tauri.conf.json`, tag) | Tag manual, CalVer |
| D22 | Pipeline | **Pipeline de release desde a Sprint 0**: GitHub Actions gera e assina (Windows Authenticode, macOS assinatura + notarização, assinatura do updater) os artefatos das 3 plataformas e publica GitHub Release | CI sem assinatura |
| D23 | Canais | **Somente stable** | stable + beta/nightly |
| D24 | Atualização | **Tauri updater** via GitHub Releases + **repositório APT** para o `.deb` | Só manual |
| D25 | Fora do MVP | Colaboração em tempo real e IA ficam como **épicos futuros** (EP11) | — |
| D26 | Fim de linha | Desenvolvimento no Windows, mas **todo o repositório usa LF** (`.gitattributes` `eol=lf`, `.editorconfig`, Prettier/rustfmt, checagem no CI). No produto, o editor salva novos arquivos em LF e preserva o fim de linha de arquivos existentes | CRLF / autocrlf |

## Premissas e riscos

| Risco | Impacto | Mitigação |
|---|---|---|
| Certificados de assinatura (Apple Developer ~US$99/ano, Authenticode/EV) | Bloqueia releases assinadas | TL-007 trata obtenção; pipeline aceita build não assinado enquanto segredos não existem, marcando a release como "unsigned" |
| Instalação TeX Live no Windows/macOS (caminhos, permissões, antivírus) | Onboarding quebra | Instalação sempre no perfil do usuário, sem admin; testes E2E por SO (TL-104) |
| Diff visual de PDF é caro | Lentidão no histórico | Renderização sob demanda, cache por hash de commit, worker dedicado |
| libgit2 e OAuth/SSH em 3 SOs | Push/pull falha em casos de borda | Matriz de testes com servidores Gitea locais no CI |
| Detecção de pacote faltante a partir do log | Falsos positivos/negativos | Mapa arquivo→pacote via `tlmgr search --file` com cache local + confirmação do usuário |
