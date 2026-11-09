# EP08 — Projetos, templates e bibliografia

**Objetivo:** qualquer pasta vira projeto; criar projetos a partir de templates (incluindo ABNT); gerenciar bibliografia com autocomplete de citações e importação por DOI.

---

### TL-801 — Abrir pasta como projeto
**Como** usuário **quero** abrir qualquer pasta **para** trabalhar com projetos já existentes.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-101 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Diálogo nativo de pasta; também via argumento de linha de comando (`texlocal ./tese`) e arrastar pasta para a janela.
- [ ] Detecta arquivo principal; cria `texlocal.toml` se ausente (após confirmação).
- [ ] Múltiplas janelas, uma por projeto.

### TL-802 — Projetos recentes
**Como** usuário **quero** uma lista de recentes **para** voltar rápido ao que estava fazendo.

- **Prioridade:** Must · **Estimativa:** 2 · **Depende de:** TL-801 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Lista com nome, caminho, última abertura, miniatura da 1ª página do PDF; fixar; remover; detecta pasta inexistente.
- [ ] Reabrir último projeto ao iniciar (configurável).

### TL-803 — Novo projeto a partir de template
**Como** estudante **quero** começar de um template **para** não montar o preâmbulo do zero.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-801, TL-206 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Galeria com prévia: artigo, relatório, livro, **ABNT (abntex2: TCC, dissertação, tese)**, beamer, pôster, CV, carta.
- [ ] Wizard: nome, pasta, motor, idioma; gera `texlocal.toml` com pacotes do template e já instala o que falta.
- [ ] Opção "ativar histórico" marcada por padrão.

### TL-804 — Templates do usuário
**Como** pesquisador **quero** salvar meus próprios templates **para** reaproveitar o padrão do meu laboratório.

- **Prioridade:** Should · **Estimativa:** 3 · **Depende de:** TL-803 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] "Salvar projeto como template"; templates do usuário em `<config>/templates/`; importar de URL git.
- [ ] Template com `template.toml` (nome, descrição, variáveis substituídas no wizard).

### TL-805 — Painel de bibliografia
**Como** pesquisador **quero** gerenciar minhas referências no app **para** não depender de outro programa.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-307 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Parser `.bib` em Rust (tolerante a erros) com diagnósticos (chave duplicada, campo obrigatório faltando).
- [ ] Painel lista entradas de todos os `.bib` do projeto com busca, ordenação e filtros por tipo.
- [ ] Formulário de edição de entrada (sincronizado com o arquivo, preservando formatação).

### TL-806 — Autocomplete de citações
**Como** pesquisador **quero** autocomplete em `\cite` **para** citar sem consultar o `.bib`.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-805, TL-304 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Em `\cite`, `\citep`, `\citet`, `\autocite`, `\textcite`, `\parencite`, `\citeonline` (abntex2): busca por chave, autor, título, ano.
- [ ] Hover em chave mostra a referência formatada; ir para entrada; diagnóstico de chave inexistente.

### TL-807 — Importar referência por DOI/ISBN/arXiv
**Como** pesquisador **quero** colar um DOI **para** adicionar a referência automaticamente.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-805 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] DOI via doi.org content negotiation (BibTeX); arXiv via API; ISBN via Open Library.
- [ ] Gera chave no padrão configurável (`autorAno`), detecta duplicatas.
- [ ] Requisição só ocorre por ação explícita do usuário (sem telemetria — D20).

### TL-808 — Importar de arquivos BibTeX/RIS e Zotero
**Como** pesquisador **quero** importar minhas referências existentes **para** migrar sem retrabalho.

- **Prioridade:** Should · **Estimativa:** 5 · **Depende de:** TL-805 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Mesclar `.bib`/`.ris` em um `.bib` do projeto com resolução de duplicatas.
- [ ] Integração com Zotero local via Better BibTeX (se em execução) — Could.

### TL-809 — Gestão de imagens e figuras
**Como** usuário **quero** inserir imagens facilmente **para** montar figuras sem digitar caminhos.

- **Prioridade:** Should · **Estimativa:** 3 · **Depende de:** TL-107, TL-501 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Arrastar/colar imagem no editor copia para `figures/` e insere snippet `figure` com caminho e label sugerido.
- [ ] Prévia de imagem ao passar o mouse em `\includegraphics`.

### TL-810 — Configurações do projeto na UI
**Como** estudante **quero** ajustar motor, arquivo principal e bibliografia por formulário **para** não editar TOML.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-206, TL-105 · **Release:** v0.5.0

**Critérios de aceite**
- [ ] Tela com todas as seções do `texlocal.toml` (projeto, TeX, compilação, editor, histórico) com validação; mudanças gravadas preservando comentários (`toml_edit`).
