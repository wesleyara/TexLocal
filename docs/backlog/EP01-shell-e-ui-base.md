# EP01 — Shell e UI base

**Objetivo:** a "casca" da IDE: janela, layout redimensionável, temas, i18n, command palette, configurações e atalhos. Referência visual: artefato **TexLocal UI** (direção "IDE moderna").

---

### TL-101 — Janela e layout de painéis
**Como** usuário **quero** um layout com activity bar, sidebar, editor e PDF **para** ver fonte e resultado lado a lado.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-001 · **Release:** v0.1.0

**Critérios de aceite**
- [ ] Regiões: title bar custom (com menus no Windows/Linux, nativa no macOS), activity bar, sidebar, área de editor, painel de PDF, painel inferior (problemas/log), status bar.
- [ ] Divisores arrastáveis; tamanhos persistidos por projeto.
- [ ] Sidebar e painel inferior recolhíveis (`Ctrl/Cmd+B`, `Ctrl/Cmd+J`).
- [ ] Modos: lado a lado, só editor, só PDF, PDF em janela separada (Could).
- [ ] Largura mínima de janela 900 px sem quebra de layout.

### TL-102 — Command palette
**Como** power user **quero** uma command palette **para** executar qualquer ação pelo teclado.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-101 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] `Ctrl/Cmd+Shift+P` abre comandos; `Ctrl/Cmd+P` abre busca de arquivos.
- [ ] Busca fuzzy, mostra atalho de cada comando, histórico de recentes.
- [ ] Registro central de comandos usado por menus, atalhos e palette.

### TL-103 — Temas escuro e claro
**Como** usuário **quero** tema escuro (padrão) e claro **para** escrever confortavelmente.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-101 · **Release:** v0.1.0

**Critérios de aceite**
- [ ] Design tokens (CSS custom properties) para cores, tipografia, espaçamento, raios.
- [ ] Opção "seguir sistema".
- [ ] Tema aplicado também ao CodeMirror e à moldura do PDF (opção de inverter cores do PDF: Could).
- [ ] Contraste AA em ambos.

### TL-104 — Internacionalização pt-BR/en
**Como** usuário **quero** a interface no meu idioma **para** usar o app sem barreiras.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-001 · **Release:** v0.1.0

**Critérios de aceite**
- [ ] `vue-i18n` no frontend e `fluent`/arquivos de mensagem no Rust para textos do backend (erros, notificações, menus nativos).
- [ ] Idioma detectado do SO, alterável nas configurações sem reiniciar.
- [ ] CI falha se houver chave faltando em algum idioma.

### TL-105 — Configurações (global e por projeto)
**Como** usuário **quero** ajustar o app globalmente e por projeto **para** adaptar ao meu fluxo.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-102 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Configurações globais em `settings.json` no diretório de config do usuário; por projeto em `texlocal.toml` (seção `[editor]`).
- [ ] UI com busca e formulário + opção "abrir JSON".
- [ ] Precedência: projeto > global > padrão; validação com schema.

### TL-106 — Atalhos de teclado customizáveis
**Como** power user **quero** remapear atalhos **para** usar o que estou acostumado.

- **Prioridade:** Should · **Estimativa:** 3 · **Depende de:** TL-102 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Mapa padrão por SO (Ctrl no Windows/Linux, Cmd no macOS).
- [ ] Editor de atalhos com detecção de conflitos; `keybindings.json`.

### TL-107 — Árvore de arquivos do projeto
**Como** usuário **quero** navegar e gerenciar os arquivos do projeto **para** organizar meu documento.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-801 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Criar, renomear, mover (drag-and-drop), excluir (para a lixeira do SO) arquivos e pastas.
- [ ] Watcher de sistema de arquivos reflete mudanças externas.
- [ ] Indicadores de status git (modificado, novo) e de arquivo principal.
- [ ] Oculta artefatos de compilação (`.aux`, `.log`, ...) por padrão.
- [ ] Importar arquivos/imagens por arrastar do SO.

### TL-108 — Status bar
**Como** usuário **quero** uma status bar informativa **para** saber o estado do projeto de relance.

- **Prioridade:** Must · **Estimativa:** 2 · **Depende de:** TL-101 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Mostra: branch, estado de sync, estado da compilação (ok/erro/compilando + tempo), motor TeX, contagem de palavras, posição do cursor, fim de linha (LF/CRLF), idioma do corretor.
- [ ] Itens clicáveis abrem a ação correspondente.

### TL-109 — Notificações e tarefas em background
**Como** usuário **quero** ver progresso de downloads, instalações e sync **para** saber o que o app está fazendo.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-101 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Toasts não bloqueantes + central de notificações.
- [ ] Gerenciador de tarefas (download TeX, instalação de pacote, push/pull) com progresso e cancelamento.

### TL-110 — Tela de boas-vindas
**Como** usuário **quero** uma tela inicial **para** abrir recentes, criar projeto ou clonar rapidamente.

- **Prioridade:** Should · **Estimativa:** 3 · **Depende de:** TL-802 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Ações: novo projeto (template), abrir pasta, clonar repositório, recentes.
- [ ] Mostra estado da instalação TeX e atalho para o wizard se não estiver pronta.
