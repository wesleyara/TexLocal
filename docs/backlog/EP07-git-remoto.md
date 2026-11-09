# EP07 — Git remoto

**Objetivo:** sincronizar projetos com GitHub, GitLab ou qualquer servidor git (HTTPS/SSH), com autenticação simples, branches básicos e editor de merge 3-way completo.

Segredos sempre no keyring do SO (crate `keyring`: Secret Service/libsecret no Linux, Credential Manager no Windows, Keychain no macOS).

---

### TL-701 — Login OAuth GitHub/GitLab (device flow)
**Como** estudante **quero** entrar com minha conta GitHub/GitLab **para** sincronizar sem configurar chaves.

- **Prioridade:** Must · **Estimativa:** 8 · **Depende de:** TL-601 · **Release:** v0.6.0

**Critérios de aceite**
- [ ] OAuth device flow (GitHub OAuth App; GitLab.com e instâncias self-hosted configuráveis).
- [ ] Abre navegador com o código; token salvo no keyring; mostra conta conectada.
- [ ] Renovação de token (GitLab) e logout que remove o segredo.
- [ ] Múltiplas contas; conta escolhida por remoto.

### TL-702 — Token pessoal (PAT)
**Como** pesquisador em servidor próprio **quero** usar um token pessoal **para** autenticar onde não há OAuth.

- **Prioridade:** Must · **Estimativa:** 2 · **Depende de:** TL-701 · **Release:** v0.6.0

**Critérios de aceite**
- [ ] Cadastro de host + usuário + token; validação de conectividade; armazenado no keyring.

### TL-703 — Chaves SSH
**Como** power user **quero** usar SSH **para** reaproveitar minha configuração existente.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-601 · **Release:** v0.6.0

**Critérios de aceite**
- [ ] Usa ssh-agent (Linux/macOS) e Pageant/OpenSSH agent (Windows); fallback para chaves em `~/.ssh` com passphrase pedida e opcionalmente salva no keyring.
- [ ] Gerar par ed25519 e copiar chave pública (com link para a página de chaves do GitHub/GitLab).
- [ ] Verificação de host conhecido (`known_hosts`) com confirmação de fingerprint.

### TL-704 — Clonar repositório
**Como** pesquisador **quero** clonar um projeto existente **para** trabalhar com coautores.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-701 · **Release:** v0.6.0

**Critérios de aceite**
- [ ] Por URL ou listando repositórios da conta conectada (busca).
- [ ] Progresso e cancelamento; após clonar, abre o projeto e executa reconciliação de dependências (TL-207).

### TL-705 — Publicar projeto em remoto
**Como** usuário **quero** publicar um projeto local no GitHub/GitLab **para** ter backup e compartilhar.

- **Prioridade:** Must · **Estimativa:** 3 · **Depende de:** TL-701 · **Release:** v0.6.0

**Critérios de aceite**
- [ ] Criar repositório (privado por padrão) via API do provedor ou adicionar remoto existente; primeiro push.

### TL-706 — Push, pull e sincronizar
**Como** usuário **quero** sincronizar com um clique **para** manter coautores atualizados.

- **Prioridade:** Must · **Estimativa:** 8 · **Depende de:** TL-703, TL-702 · **Release:** v0.6.0

**Critérios de aceite**
- [ ] Botão "Sincronizar" = checkpoint + fetch + merge + push; indicadores ahead/behind na status bar.
- [ ] Fetch periódico em background (configurável); notifica novidades remotas.
- [ ] Merge (sem rebase no MVP); fast-forward quando possível.
- [ ] Push rejeitado → fetch + merge + nova tentativa, com feedback claro.
- [ ] Versões nomeadas (tags) enviadas/recebidas.

### TL-707 — Branches básicos
**Como** pesquisador **quero** criar e alternar branches **para** experimentar sem afetar a versão principal.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-706 · **Release:** v0.6.0

**Critérios de aceite**
- [ ] Criar, alternar (com checkpoint automático), renomear, excluir branch local; publicar branch; acompanhar branch remota.
- [ ] Mesclar branch na atual (usa TL-708/TL-709 em conflito).
- [ ] Seletor de branch na status bar.

### TL-708 — Detecção e estado de conflitos
**Como** usuário **quero** ser avisado claramente de conflitos **para** saber o que resolver.

- **Prioridade:** Must · **Estimativa:** 5 · **Depende de:** TL-706 · **Release:** v0.6.0

**Critérios de aceite**
- [ ] Lista de arquivos em conflito; compilação automática e checkpoints pausados durante o merge.
- [ ] Ações: abrir editor 3-way, abortar merge (volta ao estado anterior), concluir merge.

### TL-709 — Editor de merge 3-way
**Como** pesquisador **quero** um editor visual de 3 vias **para** resolver conflitos com segurança.

- **Prioridade:** Must · **Estimativa:** 13 · **Depende de:** TL-708 · **Release:** v0.6.0

**Critérios de aceite**
- [ ] Layout: "Minha" (ours) | "Base" (opcional) | "Deles" (theirs) em cima; "Resultado" editável embaixo.
- [ ] Por conflito: aceitar minha, aceitar deles, aceitar ambas (ordem escolhível), editar manualmente.
- [ ] Navegação próximo/anterior conflito; contador de conflitos restantes.
- [ ] Diff por palavra realçado; resolução automática de mudanças não conflitantes.
- [ ] Conflitos em binários: escolher versão inteira com prévia (imagens).
- [ ] Marcar arquivo como resolvido só quando não há conflitos pendentes; "compilar resultado" antes de concluir.

### TL-710 — Conflitos de exclusão/renomeação
**Como** usuário **quero** resolver conflitos de arquivo excluído ou renomeado **para** não ficar preso.

- **Prioridade:** Should · **Estimativa:** 3 · **Depende de:** TL-709 · **Release:** v0.6.0

**Critérios de aceite**
- [ ] Casos modify/delete e rename/rename apresentados com opções manter/excluir/escolher nome.

### TL-711 — Gerenciar remotos e credenciais
**Como** power user **quero** gerenciar remotos e contas **para** controlar onde meu trabalho vai.

- **Prioridade:** Should · **Estimativa:** 3 · **Depende de:** TL-706 · **Release:** v0.6.0

**Critérios de aceite**
- [ ] Adicionar/remover/renomear remoto, alterar URL, associar conta; testar conexão.
- [ ] Tela de contas conectadas (OAuth/PAT/SSH) com revogação.
