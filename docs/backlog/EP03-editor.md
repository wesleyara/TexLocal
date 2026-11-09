# EP03 — Editor

**Objetivo:** edição de LaTeX produtiva com CodeMirror 6: realce, autocomplete, outline, referências cruzadas, multi-arquivo e corretor ortográfico.

---

### TL-301 — Integração CodeMirror 6
**Como** usuário **quero** um editor de texto rápido e robusto **para** escrever documentos grandes sem travar.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-101 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Componente Vue encapsulando CodeMirror 6 com estado por arquivo.
- [ ] Desfazer/refazer, multi-cursor, busca e substituição (regex), dobra de código, quebra de linha suave, números de linha, minimapa (Could).
- [ ] Arquivo de 10.000 linhas abre em < 300 ms e digita sem lag perceptível.
- [ ] Salvamento automático configurável (padrão: ao perder foco + 1 s após digitar).
- [ ] **Fim de linha:** novos arquivos em LF; arquivos existentes preservam o fim de linha detectado (LF/CRLF), exibido na status bar e alterável.
- [ ] Codificação UTF-8 (detecta e avisa sobre Latin-1).

### TL-302 — Linguagem LaTeX
**Como** usuário **quero** realce e indentação de LaTeX **para** ler o código facilmente.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-301 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Gramática Lezer para LaTeX (comandos, ambientes, math inline/display, comentários, verbatim, argumentos).
- [ ] Fechamento automático de `{}`, `[]`, `$$`; `\begin{x}` gera `\end{x}`; renomear `\begin` atualiza `\end`.
- [ ] Realce de BibTeX para `.bib`; texto puro para `.txt`, `.md` com realce básico.

### TL-303 — Abas e grupos de editor
**Como** usuário **quero** vários arquivos abertos em abas **para** alternar rapidamente.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-301 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Abas com indicador de não salvo, reordenáveis, fechar outras/à direita.
- [ ] Split do editor (Should).
- [ ] Abas reabertas ao reabrir o projeto.

### TL-304 — Autocomplete de comandos e ambientes
**Como** usuário **quero** sugestões de comandos **para** escrever mais rápido e sem erros.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-302 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Base de comandos/ambientes de LaTeX core + pacotes populares (amsmath, graphicx, hyperref, siunitx, abntex2...), com assinatura e documentação curta.
- [ ] Sugestões filtradas pelos pacotes efetivamente carregados no projeto.
- [ ] Comandos definidos pelo usuário (`\newcommand`, `\DeclareMathOperator`, `\newenvironment`) indexados.
- [ ] Completar caminhos em `\includegraphics`, `\input`, `\include`.

### TL-305 — Outline do documento
**Como** usuário **quero** ver a estrutura do documento **para** navegar entre seções.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-307 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Árvore de `\part`..`\subparagraph`, figuras e tabelas (com `\caption`), atravessando arquivos incluídos.
- [ ] Clique navega; seção atual destacada conforme cursor; arrastar para reordenar seções (Could).

### TL-306 — Referências cruzadas `\label`/`\ref`
**Como** usuário **quero** autocomplete e validação de referências **para** não deixar `??` no PDF.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-307 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Autocomplete em `\ref`, `\eqref`, `\autoref`, `\cref`, `\pageref` com prévia do contexto do label.
- [ ] Ir para definição (`F12`/Ctrl+clique) e localizar usos.
- [ ] Diagnóstico de label duplicado e referência inexistente.
- [ ] Renomear label atualiza todas as referências.

### TL-307 — Indexação de projeto multi-arquivo
**Como** desenvolvedor **quero** um índice do projeto **para** alimentar outline, refs, citações e comandos.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-302 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Indexador em Rust (parser tolerante) resolve árvore a partir do arquivo principal (`\input`, `\include`, `\subfile`, `\import`).
- [ ] Atualização incremental ao salvar/editar; índice exposto ao frontend via comandos Tauri.
- [ ] Detecção do arquivo principal (`\documentclass`) com magic comment `% !TEX root =` respeitado.

### TL-308 — Busca no projeto
**Como** usuário **quero** buscar e substituir em todos os arquivos **para** fazer mudanças globais.

- **Prioridade:** Should · **Estimativa:** 3 · **Depende de:** TL-301 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Busca com regex, case, palavra inteira, filtro por glob; substituir com prévia.
- [ ] Backend em Rust (`grep`/`ignore` crates), respeita `.gitignore`.

### TL-309 — Corretor ortográfico
**Como** estudante **quero** correção ortográfica em pt-BR e en **para** entregar textos sem erros.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-302 · **Release:** v0.7.0

**Critérios de aceite**
- [ ] Hunspell (via crate `spellbook` ou binding) com dicionários pt-BR e en-US embutidos.
- [ ] Ignora comandos, argumentos de `\ref`/`\cite`/`\label`/`\usepackage`, math e verbatim.
- [ ] Sublinhado, sugestões no menu de contexto, "adicionar ao dicionário" (global ou projeto, salvo em `.texlocal/dictionary.txt`).
- [ ] Idioma por projeto (manifesto) e detecção de `\usepackage[brazil]{babel}`.

### TL-310 — Contagem de palavras
**Como** estudante **quero** saber quantas palavras tenho **para** cumprir limites.

- **Prioridade:** Should · **Estimativa:** 2 · **Depende de:** TL-307 · **Release:** v0.7.0

**Critérios de aceite**
- [ ] Contagem do documento e da seleção, ignorando comandos (estilo `texcount`), mostrada na status bar com detalhamento por seção.
