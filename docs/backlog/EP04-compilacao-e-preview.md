# EP04 — Compilação e preview

**Objetivo:** compilação automática confiável com latexmk, preview em PDF.js com SyncTeX bidirecional e erros legíveis direto no editor.

---

### TL-401 — Serviço de compilação (latexmk)
**Como** usuário **quero** compilar o documento com um clique **para** ver o resultado.

- **Prioridade:** Must · **Estimativa:** 8 · **Depende de:** TL-201 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Executa `latexmk` com motor do manifesto (`-pdf`, `-xelatex`, `-lualatex`), `-synctex=1`, `-interaction=nonstopmode`, `-file-line-error`, `-outdir`.
- [ ] Saída em `.texlocal/build/` (adicionado ao `.gitignore` automaticamente).
- [ ] Biber/BibTeX e makeindex/glossaries rodados conforme necessário.
- [ ] `shell-escape` desligado por padrão; ligar exige confirmação por projeto (aviso de segurança).
- [ ] Cancelar compilação em andamento; timeout configurável.
- [ ] Processo isolado com PATH apontando para a árvore TeX escolhida.

### TL-402 — Compilação automática
**Como** usuário **quero** que o PDF atualize sozinho **para** manter o fluxo de escrita.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-401 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Gatilhos: salvar e/ou pausa de digitação (debounce configurável, padrão 1,5 s).
- [ ] Nova mudança durante compilação cancela/enfileira (nunca 2 compilações paralelas do mesmo projeto).
- [ ] Liga/desliga pela status bar e `texlocal.toml`.

### TL-403 — Visualizador PDF (PDF.js)
**Como** usuário **quero** ver o PDF dentro do app **para** não alternar janelas.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-101 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Render com PDF.js em worker; zoom (ajustar largura/página, %), rolagem contínua, miniaturas, busca de texto, links internos.
- [ ] Recarrega após compilação **preservando posição e zoom**, sem piscar (double buffering).
- [ ] PDF de 300 páginas navegável sem travar (renderização lazy).

### TL-404 — Parser de log LaTeX
**Como** usuário **quero** erros e avisos organizados **para** entender o que deu errado.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-401 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Parser em Rust para log do LaTeX, BibTeX e Biber: erros, warnings (refs indefinidas, overfull/underfull boxes), arquivo e linha.
- [ ] Mensagens comuns com explicação amigável em pt-BR/en (ex.: "Undefined control sequence").
- [ ] Testes com corpus de logs reais.

### TL-405 — Painel de problemas e erros inline
**Como** usuário **quero** ver os erros no próprio código **para** corrigi-los rápido.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-404, TL-301 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Sublinhado e marcador na gutter; hover mostra mensagem.
- [ ] Painel inferior "Problemas" filtrável por tipo/arquivo; clique navega.
- [ ] Aba "Log bruto" com busca.
- [ ] Filtro para ocultar avisos de box (padrão: ocultos).

### TL-406 — SyncTeX bidirecional
**Como** usuário **quero** pular entre o PDF e o fonte **para** editar onde estou lendo.

- **Prioridade:** Must · **Estimativa:** 8 · **Depende de:** TL-403, TL-401 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Inverso: Ctrl/Cmd+clique (ou duplo clique) no PDF abre o arquivo/linha correspondente.
- [ ] Direto: atalho/botão leva o PDF à posição do cursor, com destaque temporário.
- [ ] Parser de `.synctex.gz` em Rust (sem depender do binário `synctex`).
- [ ] Funciona com arquivos incluídos e `outdir`.
- [ ] Opção de sincronizar automaticamente ao mover o cursor (Could).

### TL-407 — Auto-instalar pacote faltante e recompilar
**Como** estudante **quero** que pacotes faltantes sejam instalados sozinhos **para** não travar por isso.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-208 · **Release:** v0.3.0

**Critérios de aceite**
- [ ] Modos (configuração): perguntar (padrão), instalar automaticamente, nunca.
- [ ] Após instalar, recompila automaticamente; vários pacotes faltantes resolvidos em ciclo (limite de iterações).

### TL-408 — Modo rascunho e compilação rápida
**Como** usuário com documento grande **quero** compilar mais rápido **para** ver mudanças sem esperar.

- **Prioridade:** Could · **Estimativa:** 5 · **Depende de:** TL-401 · **Release:** v0.7.0

**Critérios de aceite**
- [ ] Opção `draft` (imagens como caixas) e compilar só o capítulo atual (`\includeonly`).

### TL-409 — Múltiplos documentos por projeto
**Como** pesquisador **quero** compilar mais de um documento principal (ex.: artigo e slides) **para** manter tudo num projeto.

- **Prioridade:** Should · **Estimativa:** 5 · **Depende de:** TL-401, TL-206 · **Release:** v0.7.0

**Critérios de aceite**
- [ ] Manifesto aceita `[[documents]]` com `main` e motor próprios; seletor do documento ativo na status bar.

### TL-410 — Exportar PDF e fontes
**Como** usuário **quero** exportar o PDF e um zip do projeto **para** entregar ou submeter.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-401 · **Release:** v0.7.0

**Critérios de aceite**
- [ ] "Salvar PDF como…", abrir no visualizador do SO, imprimir.
- [ ] Exportar zip de submissão (fontes + `.bbl`, sem build e sem `.git`), compatível com arXiv.
