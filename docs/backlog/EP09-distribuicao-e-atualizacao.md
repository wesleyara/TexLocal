# EP09 — Distribuição e atualização

**Objetivo:** o usuário baixa um instalador por SO e o app se mantém atualizado sozinho (canal único stable). Complementa a pipeline da EP00.

---

### TL-901 — Pacotes por SO
**Como** usuário **quero** um instalador nativo do meu SO **para** instalar como qualquer outro programa.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-008 · **Release:** v0.1.0

**Critérios de aceite**
- [ ] Linux: `.deb` (Debian 12+/Ubuntu 22.04+) com dependências declaradas (`libwebkit2gtk-4.1-0`, `libsecret-1-0`...), `.AppImage`.
- [ ] Windows: `.msi` (WiX) instalação por usuário, sem admin; WebView2 bootstrapper embutido.
- [ ] macOS: `.dmg` com app assinado e notarizado.
- [ ] Ícones, nome, metadados e entradas de menu corretos; desinstalação limpa (dados do usuário mantidos, com opção de remover).
- [ ] Instalador não inclui TeX (baixado pelo wizard — D08).

### TL-902 — Atualização automática (Tauri updater)
**Como** usuário **quero** que o app se atualize **para** ter correções sem esforço.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-008 · **Release:** v0.1.0

**Critérios de aceite**
- [ ] Verifica `latest.json` do GitHub Releases ao iniciar e a cada 24 h; assinatura validada.
- [ ] Diálogo com notas da versão (do CHANGELOG), "atualizar agora", "ao fechar", "pular versão".
- [ ] Atualiza AppImage, `.msi` e `.app`; para `.deb`, delega ao APT (TL-903) e só notifica.
- [ ] Opção de desativar verificação.

### TL-903 — Repositório APT
**Como** usuário Debian/Ubuntu **quero** instalar via APT **para** receber atualizações junto do sistema.

- **Prioridade:** Should · **Estimativa:** 5 · **Depende de:** TL-008 · **Release:** v0.7.0

**Critérios de aceite**
- [ ] Repositório assinado com GPG publicado (GitHub Pages ou bucket) atualizado pela pipeline de release.
- [ ] O `.deb` adiciona o repositório e a chave em `/etc/apt/sources.list.d/` (formato deb822, `signed-by`) na pós-instalação, com opção de não adicionar.
- [ ] Instruções de instalação documentadas.

### TL-904 — Associação de arquivos e "abrir com"
**Como** usuário **quero** abrir `.tex` com duplo clique **para** entrar direto no projeto.

- **Prioridade:** Should · **Estimativa:** 3 · **Depende de:** TL-901, TL-801 · **Release:** v0.7.0

**Critérios de aceite**
- [ ] Associação para `.tex`, `.bib`, `texlocal.toml`; abrir arquivo abre o projeto (raiz detectada) e foca no arquivo.
- [ ] Instância única: novo arquivo abre na janela existente ou nova janela por projeto.

### TL-905 — Deep links
**Como** usuário **quero** links `texlocal://` **para** clonar projetos a partir do navegador.

- **Prioridade:** Could · **Estimativa:** 3 · **Depende de:** TL-704 · **Release:** v0.7.0

**Critérios de aceite**
- [ ] `texlocal://clone?url=...` abre o fluxo de clonagem com confirmação.

### TL-906 — CLI `texlocal`
**Como** power user **quero** abrir projetos e compilar pela linha de comando **para** integrar com meus scripts.

- **Prioridade:** Could · **Estimativa:** 5 · **Depende de:** TL-401 · **Release:** v0.7.0

**Critérios de aceite**
- [ ] `texlocal <pasta|arquivo>` abre; `texlocal build <pasta>` compila headless usando o TeX gerenciado e o manifesto.
- [ ] CLI adicionada ao PATH no Linux (`.deb`) e opcional no Windows/macOS.

### TL-907 — Portabilidade de dados do app
**Como** usuário **quero** exportar/importar minhas configurações **para** migrar de máquina.

- **Prioridade:** Could · **Estimativa:** 3 · **Depende de:** TL-105, TL-502 · **Release:** v0.7.0

**Critérios de aceite**
- [ ] Exporta settings, keybindings, snippets globais, templates do usuário e dicionário pessoal (sem segredos).

### TL-908 — Página de download e instruções
**Como** usuário **quero** uma página clara de download **para** escolher o instalador certo.

- **Prioridade:** Should · **Estimativa:** 5 · **Depende de:** TL-008 · **Release:** v0.7.0

**Critérios de aceite**
- [ ] Site estático (GitHub Pages) detecta o SO e aponta para o último artefato; checksums visíveis; instruções APT.
- [ ] Atualizado automaticamente pela pipeline de release.
