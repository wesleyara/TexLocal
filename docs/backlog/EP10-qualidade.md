# EP10 — Qualidade, desempenho e documentação

**Objetivo:** garantir que o MVP seja confiável nas 3 plataformas, rápido, acessível e documentado.

---

### TL-1001 — Estratégia de testes
**Como** mantenedor **quero** uma pirâmide de testes definida **para** evoluir com segurança.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-005 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Unitários Rust (`cargo test`, `insta` para snapshots de parsers), Vue (Vitest + Testing Library).
- [ ] Integração Rust com TeX Live real em container no CI (Linux) e cache entre runs.
- [ ] Cobertura reportada (llvm-cov, v8) com mínimo de 70% nos crates core.

### TL-1002 — Testes E2E por SO
**Como** mantenedor **quero** fluxos ponta a ponta testados em cada SO **para** pegar regressões de plataforma.

- **Prioridade:** Must · **Estimativa:** 8 · **Depende de:** TL-1001 · **Release:** v1.0.0

**Critérios de aceite**
- [ ] WebdriverIO + `tauri-driver` (Linux/Windows); macOS via testes de integração do backend + smoke manual documentado.
- [ ] Fluxos: wizard (perfil mínimo), novo projeto ABNT, compilar, SyncTeX, checkpoint, restaurar, clone/push/pull contra Gitea local, merge 3-way.
- [ ] Teste de fim de linha: arquivo CRLF preserva CRLF; arquivo novo nasce LF (Windows incluído).

### TL-1003 — Orçamento de desempenho
**Como** usuário **quero** um app rápido **para** não perder o foco.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-1001 · **Release:** v1.0.0

**Critérios de aceite**
- [ ] Inicialização a frio < 1,5 s até editor interativo; memória ociosa < 250 MB com projeto médio.
- [ ] Latência de digitação p95 < 16 ms com arquivo de 10k linhas.
- [ ] Benchmarks no CI (criterion para parsers/indexador) com alerta de regressão > 10%.

### TL-1004 — Acessibilidade
**Como** usuário com deficiência **quero** usar o app com teclado e leitor de tela **para** ter acesso igual.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-101 · **Release:** v1.0.0

**Critérios de aceite**
- [ ] Todas as ações acessíveis por teclado; foco visível; ordem lógica.
- [ ] ARIA nos componentes customizados; testado com Orca, NVDA e VoiceOver.
- [ ] Contraste AA; opção de tamanho de fonte da UI; respeita "reduzir movimento".

### TL-1005 — Relatório de crash local
**Como** usuário **quero** gerar um relatório quando algo quebrar **para** reportar o problema se eu quiser.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-109 · **Release:** v1.0.0

**Critérios de aceite**
- [ ] Panic hook Rust + handler global no frontend gravam relatório em `<data>/crash-reports/` (versão, SO, stack, últimos logs), sem conteúdo de documentos.
- [ ] Na próxima abertura: "Ver relatório", "Abrir issue no GitHub (pré-preenchida)", "Descartar". Nada é enviado automaticamente (D20).

### TL-1006 — Logs da aplicação
**Como** mantenedor **quero** logs estruturados **para** diagnosticar problemas reportados.

- **Prioridade:** Should · **Estimativa:** 2 · **Depende de:** TL-001 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] `tracing` com rotação de arquivos em `<data>/logs`; nível configurável; comando "Abrir pasta de logs".
- [ ] Tokens e segredos nunca registrados (teste automatizado).

### TL-1007 — Segurança da aplicação
**Como** usuário **quero** que o app seja seguro **para** abrir projetos de terceiros sem risco.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-401 · **Release:** v1.0.0

**Critérios de aceite**
- [ ] Capabilities do Tauri 2 mínimas por janela; CSP restritiva; sem `eval`.
- [ ] `shell-escape` bloqueado por padrão e por projeto clonado (confirmação explícita).
- [ ] Projetos não confiáveis: aviso ao abrir projeto clonado com `latexmkrc` (executa Perl) — usa `-norc` até o usuário confiar.
- [ ] Revisão de ameaças documentada.

### TL-1008 — Documentação do usuário
**Como** estudante **quero** um guia de uso **para** aprender o app rapidamente.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** — · **Release:** v1.0.0

**Critérios de aceite**
- [ ] Site de docs (VitePress) pt-BR/en: instalação, primeiro projeto, histórico, sincronização, snippets, solução de problemas, FAQ.
- [ ] Publicado pela pipeline junto com a release; link "Ajuda" no app.
