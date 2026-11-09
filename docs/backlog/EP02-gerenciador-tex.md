# EP02 — Gerenciador TeX (autossuficiência)

**Objetivo:** o usuário instala só o TexLocal; o app detecta ou instala o TeX Live, baixa pacotes sob demanda e garante que cada projeto tenha o que declara no `texlocal.toml`.

**Diretórios (padrão, via `directories` crate):**
- Linux: `~/.local/share/texlocal/texlive/<ano>`
- Windows: `%LOCALAPPDATA%\TexLocal\texlive\<ano>`
- macOS: `~/Library/Application Support/TexLocal/texlive/<ano>`

Sem necessidade de privilégios de administrador em nenhum SO.

---

### TL-201 — Detectar distribuições TeX existentes
**Como** usuário que já tem TeX **quero** que o app o detecte **para** não baixar tudo de novo.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-001 · **Release:** v0.2.0

**Critérios de aceite**
- [ ] Detecta TeX Live (PATH e locais padrão), MiKTeX (Windows), MacTeX; reporta versão/ano e motores disponíveis.
- [ ] Se houver TeX do sistema, o wizard pergunta qual usar (sistema ou gerenciado); a escolha é alterável nas configurações e sobrescrevível por projeto.
- [ ] Ao usar o TeX do sistema, auto-instalação de pacotes funciona via `tlmgr --usermode` (TeX Live) ou `mpm` (MiKTeX) quando possível; senão, só informa.

### TL-202 — Wizard de primeira execução
**Como** estudante **quero** um assistente de instalação **para** ter o TeX funcionando sem conhecer nada.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-201, TL-109 · **Release:** v0.3.0

**Critérios de aceite**
- [ ] Passos: boas-vindas → detecção → escolha de perfil (Mínimo ~150 MB / Recomendado ~800 MB / Completo ~5 GB, com espaço em disco disponível exibido) → local de instalação → download.
- [ ] Pode ser pulado e reaberto pelas configurações.
- [ ] O usuário pode usar o editor enquanto instala; compilação aguarda com mensagem clara.

### TL-203 — Instalador TeX Live gerenciado
**Como** usuário **quero** que o app instale o TeX Live **para** não precisar fazer isso manualmente.

- **Prioridade:** Must · **Estimativa:** 13 · **Depende de:** TL-202 · **Release:** v0.3.0

**Critérios de aceite**
- [ ] Usa `install-tl` com arquivo de profile gerado (scheme `infraonly`/`basic`/`medium`/`full` mapeado ao perfil + coleções), instalação portable em modo usuário.
- [ ] Verifica checksums/assinatura do `install-tl`.
- [ ] Download retomável; falha de rede não corrompe instalação (instala em diretório temporário e move ao final).
- [ ] Funciona em Linux (x86_64, aarch64), Windows x64, macOS (arm64/x64).
- [ ] Coleção de idioma português (`collection-langportuguese`) e `latexmk` sempre incluídos.

### TL-204 — Seleção de espelho CTAN
**Como** usuário **quero** downloads rápidos **para** não esperar à toa.

- **Prioridade:** Should · **Estimativa:** 3 · **Depende de:** TL-203 · **Release:** v0.3.0

**Critérios de aceite**
- [ ] Usa `mirror.ctan.org` (redirecionamento geográfico) por padrão; opção de fixar espelho; fallback automático se um espelho falhar.
- [ ] Suporte a proxy HTTP(S) do sistema.

### TL-205 — Camada de serviço `tlmgr`
**Como** desenvolvedor **quero** uma API Rust sobre o `tlmgr` **para** instalar, remover, atualizar e buscar pacotes.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-203 · **Release:** v0.3.0

**Critérios de aceite**
- [ ] Operações: `install`, `remove`, `update --all`, `search --file`, `info`, `list --only-installed`.
- [ ] Fila serializada (uma operação por vez), progresso parseado e cancelável.
- [ ] Erros traduzidos para mensagens amigáveis (i18n).

### TL-206 — Manifesto `texlocal.toml`
**Como** pesquisador **quero** que o projeto declare suas dependências **para** compilar igual em qualquer máquina.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-205 · **Release:** v0.3.0

**Critérios de aceite**
- [ ] Schema documentado:
  ```toml
  [project]
  main = "main.tex"
  [tex]
  engine = "pdflatex"   # pdflatex | xelatex | lualatex
  texlive = "2026"      # ano mínimo
  packages = ["abntex2", "siunitx"]
  bibliography = "biber" # bibtex | biber
  [compile]
  auto = true
  shell_escape = false
  outdir = ".texlocal/build"
  ```
- [ ] Criado automaticamente ao abrir/criar projeto (com valores inferidos).
- [ ] Editável pela UI (TL-209) e manualmente, com validação e erro amigável.

### TL-207 — Reconciliar dependências ao abrir projeto
**Como** pesquisador que clonou um projeto **quero** que o app instale o que falta **para** compilar de primeira.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-206 · **Release:** v0.3.0

**Critérios de aceite**
- [ ] Ao abrir, compara `packages` + ano do TeX Live com a instalação global.
- [ ] Mostra diálogo "Este projeto precisa de N pacotes (X MB)" com instalar/ignorar.
- [ ] Se o ano exigido for maior que o instalado, oferece atualizar ou instalar nova árvore lado a lado.

### TL-208 — Detectar pacote faltante na compilação
**Como** usuário **quero** que o app perceba pacote faltante **para** não ter que decifrar o log.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-205, TL-404 · **Release:** v0.3.0

**Critérios de aceite**
- [ ] Reconhece `! LaTeX Error: File 'x.sty' not found`, `.cls`, fontes (`mktextfm`), `.bst`, `.def` etc.
- [ ] Resolve arquivo → pacote com `tlmgr search --global --file` e cache local (índice offline atualizado periodicamente).
- [ ] Registra o pacote no `packages` do manifesto (opção configurável).

### TL-209 — Gerenciador de pacotes na UI
**Como** power user **quero** ver e gerenciar pacotes instalados **para** ter controle sobre a instalação.

- **Prioridade:** Should · **Estimativa:** 5 · **Depende de:** TL-205 · **Release:** v0.3.0

**Critérios de aceite**
- [ ] Lista pacotes instalados (global) e declarados no projeto; busca no CTAN; instalar/remover.
- [ ] Indica pacotes declarados ainda não instalados.

### TL-210 — Atualizar TeX Live
**Como** usuário **quero** atualizar a distribuição **para** ter correções e pacotes novos.

- **Prioridade:** Should · **Estimativa:** 5 · **Depende de:** TL-205 · **Release:** v0.3.0

**Critérios de aceite**
- [ ] Verificação periódica (configurável) de atualizações via `tlmgr update --list`.
- [ ] Migração anual (novo ano do TeX Live): instala nova árvore lado a lado, mantém a anterior até o usuário remover.

### TL-211 — Modo offline
**Como** usuário sem internet **quero** que tudo continue funcionando **para** escrever em qualquer lugar.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-208 · **Release:** v0.3.0

**Critérios de aceite**
- [ ] Sem rede, compilação usa o que está instalado; pacote faltante gera aviso "será instalado quando houver conexão".
- [ ] Fila de instalações pendentes executa ao reconectar.

### TL-212 — Limpeza e uso de disco
**Como** usuário **quero** ver e liberar espaço usado pelo TeX **para** controlar meu disco.

- **Prioridade:** Could · **Estimativa:** 3 · **Depende de:** TL-209 · **Release:** v0.3.0

**Critérios de aceite**
- [ ] Mostra espaço por árvore TeX Live e cache de downloads; remover árvores antigas e limpar caches.
