# TexLocal — Backlog

Editor LaTeX local, multiplataforma e autossuficiente, com histórico de versões estilo Overleaf sustentado por git real, snippets e gerenciamento automático da distribuição TeX.

## Visão

> Instalar o `.deb` (ou `.msi`/`.dmg`), abrir uma pasta e escrever. O TexLocal cuida do resto: instala o TeX Live, baixa pacotes que faltam, compila, mostra o PDF, guarda o histórico e sincroniza com GitHub/GitLab — sem o usuário precisar saber o que é `tlmgr` ou `git`.

### Princípios

1. **Zero configuração**: nenhum pré-requisito no sistema (nem TeX, nem git).
2. **Local primeiro**: tudo funciona offline depois da instalação inicial; nada sai da máquina sem ação explícita.
3. **Reprodutível**: o projeto declara o que precisa (`texlocal.toml`) e qualquer máquina com TexLocal consegue compilá-lo.
4. **Git sem fricção**: histórico estilo Overleaf para quem não conhece git; git completo por baixo para quem conhece.
5. **Versionado desde o dia 1**: pipeline de release automatizado existe antes da primeira feature.

## Estrutura

| Arquivo | Conteúdo |
|---|---|
| [decisoes.md](decisoes.md) | Decisões de arquitetura e produto tomadas na sabatina (ADRs resumidos) |
| [roadmap.md](roadmap.md) | Releases, marcos e ordem dos épicos |
| [convencoes.md](convencoes.md) | Formato das histórias, DoR, DoD, prioridade e estimativa |
| [EP00-fundacao-e-release.md](EP00-fundacao-e-release.md) | Repositório, CI, Conventional Commits, release-please, builds assinados |
| [EP01-shell-e-ui-base.md](EP01-shell-e-ui-base.md) | Janela, layout, temas, i18n, command palette, configurações |
| [EP02-gerenciador-tex.md](EP02-gerenciador-tex.md) | Wizard, TeX Live gerenciado, `tlmgr`, manifesto, auto-instalação |
| [EP03-editor.md](EP03-editor.md) | CodeMirror 6, linguagem LaTeX, autocomplete, outline, ortografia |
| [EP04-compilacao-e-preview.md](EP04-compilacao-e-preview.md) | latexmk, PDF.js, SyncTeX, parsing de log, erros inline |
| [EP05-snippets.md](EP05-snippets.md) | Snippets globais/projeto em formato VS Code |
| [EP06-historico-local.md](EP06-historico-local.md) | Checkpoints, versões nomeadas, diff fonte/PDF, restaurar, exportar |
| [EP07-git-remoto.md](EP07-git-remoto.md) | Auth, clone, push/pull, branches, merge 3-way |
| [EP08-projetos-templates-bibliografia.md](EP08-projetos-templates-bibliografia.md) | Projetos, recentes, templates, bibliografia |
| [EP09-distribuicao-e-atualizacao.md](EP09-distribuicao-e-atualizacao.md) | Pacotes, updater, repositório APT, integração com SO |
| [EP10-qualidade.md](EP10-qualidade.md) | Testes, desempenho, acessibilidade, crash report, docs |
| [EP11-futuro.md](EP11-futuro.md) | Pós-MVP: colaboração em tempo real, IA, outros |

## Resumo numérico

| Épico | Histórias | Pontos |
|---|---|---|
| EP00 Fundação e release | 10 | 35 |
| EP01 Shell e UI base | 10 | 37 |
| EP02 Gerenciador TeX | 12 | 60 |
| EP03 Editor | 10 | 43 |
| EP04 Compilação e preview | 10 | 50 |
| EP05 Snippets | 7 | 24 |
| EP06 Histórico local | 11 | 60 |
| EP07 Git remoto | 11 | 58 |
| EP08 Projetos, templates, bibliografia | 10 | 35 |
| EP09 Distribuição e atualização | 8 | 34 |
| EP10 Qualidade | 8 | 34 |
| **Total MVP** | **107** | **470** |

O design de referência da interface está no artefato **TexLocal UI**: https://claude.ai/artifact/1puijeTSSLEfRsFrCVR7Gy (editor + preview, histórico/diff, merge 3-way, wizard TeX, design tokens).
