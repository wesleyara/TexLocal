# EP06 — Histórico local (estilo Overleaf sobre git)

**Objetivo:** histórico de versões amigável para quem não conhece git (checkpoints automáticos, versões nomeadas, comparar, restaurar, exportar), usando um repositório git real e padrão por baixo.

**Mapeamento de conceitos**

| UI (Overleaf-like) | Git |
|---|---|
| Checkpoint automático | Commit com mensagem gerada (`chore(checkpoint): 3 arquivos alterados`) |
| Sessão de edição | Grupo de checkpoints consecutivos na timeline |
| Versão nomeada | Tag anotada (`texlocal/<slug>`) |
| Restaurar | Novo commit que reverte arquivos para o estado escolhido (nunca reescreve histórico) |

---

### TL-601 — Camada git embutida
**Como** desenvolvedor **quero** uma API git em Rust **para** não depender do git do sistema.

- **Prioridade:** Must · **Estimativa:** 8 · **Depende de:** TL-001 · **Release:** v0.4.0

**Critérios de aceite**
- [ ] Crate `texlocal-git` sobre `git2` (libgit2 vendored, estático, com HTTPS e SSH).
- [ ] Operações: init, status, add, commit, log paginado, diff (tree/workdir), tag, checkout de arquivo, revert por conteúdo.
- [ ] Repositórios criados com `core.autocrlf=false` e `.gitattributes` padrão (`* text=auto eol=lf`, binários marcados), coerente com D26.
- [ ] Identidade (nome/e-mail) pedida no primeiro uso e salva na config do app (não altera `~/.gitconfig`).
- [ ] Compatível com repositórios criados pelo git CLI (usuário pode usar ambos).

### TL-602 — Inicializar versionamento no projeto
**Como** estudante **quero** ativar o histórico com um clique **para** nunca perder trabalho.

- **Prioridade:** Must · **Estimativa:** 2 · **Depende de:** TL-601, TL-801 · **Release:** v0.4.0

**Critérios de aceite**
- [ ] Ao abrir pasta sem `.git`, oferece "Ativar histórico" (lembrar escolha).
- [ ] Gera `.gitignore` com artefatos LaTeX e `.texlocal/build/`; primeiro commit "Versão inicial".

### TL-603 — Checkpoints automáticos
**Como** usuário **quero** que o app salve versões sozinho **para** voltar no tempo sem pensar nisso.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-602 · **Release:** v0.4.0

**Critérios de aceite**
- [ ] Commit automático após N minutos de inatividade com mudanças (padrão 5), ao fechar o projeto e antes de restaurar/pull.
- [ ] Mensagem gerada com resumo de arquivos; autor = identidade configurada.
- [ ] Não commita se o repositório estiver em estado de merge/conflito.
- [ ] Configurável/desativável por projeto (power users que preferem commits manuais).

### TL-604 — Versões nomeadas
**Como** pesquisador **quero** marcar versões importantes ("Enviado à banca") **para** encontrá-las depois.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-603 · **Release:** v0.4.0

**Critérios de aceite**
- [ ] "Nomear versão atual" (cria checkpoint se houver mudanças + tag anotada com descrição).
- [ ] Nomear qualquer ponto da timeline; renomear e remover rótulo.

### TL-605 — Commit manual
**Como** power user **quero** commits com mensagem própria **para** manter um histórico significativo.

- **Prioridade:** Should · **Estimativa:** 3 · **Depende de:** TL-601 · **Release:** v0.4.0

**Critérios de aceite**
- [ ] Painel de alterações (staged/unstaged, stage por arquivo e por trecho), mensagem, amend do último commit não publicado.

### TL-606 — Timeline do histórico
**Como** usuário **quero** ver a linha do tempo do projeto **para** escolher para onde voltar.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-601 · **Release:** v0.4.0

**Critérios de aceite**
- [ ] Painel lateral com commits agrupados por sessão/dia, versões nomeadas destacadas, autor e arquivos alterados.
- [ ] Filtros: só versões nomeadas, por arquivo, por autor.
- [ ] Paginação/scroll infinito para milhares de commits.

### TL-607 — Diff de fonte entre versões
**Como** usuário **quero** comparar quaisquer duas versões **para** ver o que mudou.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-606 · **Release:** v0.4.0

**Critérios de aceite**
- [ ] Selecionar intervalo na timeline (de/até) ou versão vs atual.
- [ ] Visualização inline e lado a lado (CodeMirror merge view), diff por palavra (bom para prosa).
- [ ] Lista de arquivos alterados com contagem +/−; binários (imagens) com prévia antes/depois.

### TL-608 — Restaurar arquivo ou projeto
**Como** usuário **quero** restaurar uma versão anterior **para** desfazer mudanças ruins com segurança.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-607 · **Release:** v0.4.0

**Critérios de aceite**
- [ ] Restaurar um arquivo, um trecho (hunk) ou o projeto inteiro para o estado da versão escolhida.
- [ ] Sempre cria checkpoint antes e um novo commit "Restaurado de <versão>" depois; nunca reescreve histórico.
- [ ] Confirmação mostrando o que será alterado.

### TL-609 — Diff visual de PDF
**Como** pesquisador **quero** ver as mudanças no PDF renderizado **para** revisar como o leitor verá.

- **Prioridade:** Must · **Estimativa:** 13 · **Depende de:** TL-607, TL-401 · **Release:** v0.7.0

**Critérios de aceite**
- [ ] Compila as duas versões em worktrees temporárias isoladas (reutilizando cache por hash de commit).
- [ ] Modos: lado a lado sincronizado, sobreposição com destaque de pixels alterados, e destaque de texto alterado (comparação do texto extraído por página com mapeamento de posições).
- [ ] Navegação "próxima mudança"; páginas sem mudança podem ser recolhidas.
- [ ] Executa em background com progresso e cancelamento; falha de compilação de uma versão é reportada com clareza.

### TL-610 — Exportar versão
**Como** usuário **quero** baixar uma versão antiga **para** reenviar ou arquivar.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-606 · **Release:** v0.4.0

**Critérios de aceite**
- [ ] Exportar zip dos fontes da versão (`git archive`).
- [ ] Exportar PDF da versão (compila em worktree temporária; usa cache se existir).

### TL-611 — Stash implícito ao trocar de contexto
**Como** usuário **quero** que mudanças não salvas nunca se percam **para** confiar no app.

- **Prioridade:** Must · **Estimativa:** 8 · **Depende de:** TL-603 · **Release:** v0.4.0

**Critérios de aceite**
- [ ] Antes de restaurar, trocar de branch ou pull: checkpoint automático (nunca descarta silenciosamente).
- [ ] Recuperação após crash: buffers não salvos persistidos em `.texlocal/recovery/` e oferecidos ao reabrir.
