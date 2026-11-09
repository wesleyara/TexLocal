# EP05 — Snippets

**Objetivo:** snippets globais e por projeto, no formato JSON compatível com VS Code, com biblioteca padrão e editor na interface.

**Locais:**
- Global: `<config>/snippets/*.json`
- Projeto: `.texlocal/snippets/*.json` (versionado no git)

---

### TL-501 — Motor de snippets (formato VS Code)
**Como** power user **quero** snippets com placeholders **para** inserir estruturas rapidamente.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-301 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Parser da sintaxe VS Code: `$1`, `${1:padrão}`, `${1|a,b,c|}`, `$0`, placeholders espelhados, variáveis (`$TM_SELECTED_TEXT`, `$CLIPBOARD`, `$CURRENT_YEAR`, `$TM_FILENAME_BASE`...), transformações regex.
- [ ] Navegação Tab/Shift+Tab, Esc sai.
- [ ] Campo `scope` (`latex`, `bibtex`) e `prefix` (string ou lista).
- [ ] Arquivos de snippets do VS Code importados sem conversão.

### TL-502 — Snippets globais e de projeto
**Como** pesquisador **quero** snippets do projeto versionados **para** compartilhar padrões com coautores.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-501 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Carrega de padrão embutido → global → projeto; mesmo prefixo: projeto sobrescreve.
- [ ] Recarrega ao alterar arquivos (watcher).
- [ ] Erros de JSON exibidos no painel de problemas.

### TL-503 — Biblioteca padrão
**Como** estudante **quero** snippets prontos **para** produzir sem decorar comandos.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-501 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Inclui: figure, table (com booktabs), equation/align, itemize/enumerate, section*, frame (beamer), theorem, código (listings/minted), citações ABNT, matrizes, frações, `\SI`.
- [ ] Pode ser desativada por categoria.

### TL-504 — Integração com autocomplete
**Como** usuário **quero** ver snippets nas sugestões **para** descobri-los sem decorar prefixos.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-501, TL-304 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Snippets aparecem no autocomplete com ícone e prévia expandida.
- [ ] Ordem configurável (snippets antes/depois de comandos).

### TL-505 — Editor de snippets na UI
**Como** usuário **quero** criar e editar snippets visualmente **para** não editar JSON na mão.

- **Prioridade:** Should · **Estimativa:** 5 · **Depende de:** TL-502 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Lista com busca por escopo (padrão/global/projeto), formulário (nome, prefixo, corpo, descrição) com prévia ao vivo.
- [ ] "Criar snippet a partir da seleção".
- [ ] Botão "abrir JSON".

### TL-506 — Inserir snippet pela palette
**Como** power user **quero** inserir um snippet pela command palette **para** acessar qualquer um rapidamente.

- **Prioridade:** Should · **Estimativa:** 2 · **Depende de:** TL-501, TL-102 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Comando "Inserir snippet…" com busca fuzzy; snippets podem ter atalho de teclado próprio.

### TL-507 — Importar/exportar snippets
**Como** usuário **quero** importar e exportar snippets **para** migrar de outros editores.

- **Prioridade:** Could · **Estimativa:** 3 · **Depende de:** TL-505 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Importar `.json` / `.code-snippets` do VS Code; exportar coleção selecionada.
