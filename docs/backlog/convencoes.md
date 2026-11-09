# Convenções do backlog

## Formato de história

```
### TL-XXX — Título curto
**Como** <persona> **quero** <capacidade> **para** <benefício>.

- **Prioridade:** Must | Should | Could | Won't (MoSCoW, relativa ao MVP v1.0)
- **Estimativa:** pontos Fibonacci (1, 2, 3, 5, 8, 13)
- **Depende de:** TL-YYY, ...
- **Release:** v0.x

**Critérios de aceite**
- [ ] Dado ..., quando ..., então ...
```

IDs são sequenciais por épico: EP00 → TL-001..099, EP01 → TL-101..199, ..., EP11 → TL-1101..

## Personas

| Persona | Descrição |
|---|---|
| **Estudante** | Escreve TCC/dissertação em ABNT, conhece pouco de LaTeX e nada de git |
| **Pesquisador** | Escreve artigos, usa bibliografia extensa, colabora via GitHub/GitLab |
| **Power user** | Conhece LaTeX e git, quer atalhos, snippets e controle fino |
| **Mantenedor** | Desenvolve e publica o TexLocal |

## Definition of Ready

- História com critérios de aceite testáveis.
- Dependências concluídas ou planejadas na mesma sprint.
- Design de UI referenciado (artefato TexLocal UI) quando houver tela.

## Definition of Done

- Código revisado em PR com título Conventional Commit.
- Testes unitários (Rust: `cargo test`; Vue: Vitest) e, quando aplicável, E2E (WebdriverIO + tauri-driver).
- CI verde nas 3 plataformas.
- Strings em pt-BR e en.
- Acessível por teclado; contraste AA.
- Documentação do usuário atualizada quando a feature é visível.
- Entrada no CHANGELOG gerada pelo commit (sem edição manual).

## Prioridade

- **Must**: sem isso não há v1.0.
- **Should**: importante, entra no v1.0 salvo imprevisto.
- **Could**: desejável, entra se houver folga.
- **Won't (agora)**: registrado para depois do v1.0.
