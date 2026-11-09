# Roadmap

Todas as releases são **stable**, versionadas por SemVer via release-please, publicadas para Linux, Windows e macOS desde a v0.1.0.

```
Sprint 0  ── v0.1.0  Esqueleto + pipeline de release
          ── v0.2.0  Editar e compilar (TeX do sistema)
          ── v0.3.0  TeX autossuficiente
          ── v0.4.0  Histórico local
          ── v0.5.0  Produtividade (snippets, autocomplete, templates, bibliografia)
          ── v0.6.0  Git remoto + merge 3-way
          ── v0.7.0  Diff visual de PDF, ortografia, polimento
          ── v1.0.0  MVP: hardening, docs, acessibilidade
```

## v0.1.0 — Esqueleto + pipeline (Sprint 0)

Objetivo: um app vazio (janela com layout) instalável nas 3 plataformas, com o fluxo completo `commit convencional → release-please → tag → build assinado → GitHub Release → updater`.

- EP00 inteiro (TL-001..TL-010)
- TL-101 (janela e layout), TL-103 (tema escuro), TL-104 (i18n base)
- TL-901 (pacotes por SO), TL-902 (updater)

## v0.2.0 — Editar e compilar

- TL-102, TL-105..TL-108 (palette, configurações, atalhos, árvore de arquivos)
- TL-201 (detecção de TeX do sistema)
- TL-301..TL-303 (editor, linguagem LaTeX, abas)
- TL-401..TL-406 (latexmk, compilação automática, PDF.js, log, erros inline, SyncTeX)
- TL-801, TL-802 (abrir pasta, recentes)
- TL-1001, TL-1006 (estratégia de testes, logs)

## v0.3.0 — TeX autossuficiente

- TL-202..TL-212 (wizard, instalação, espelhos, tlmgr, manifesto, auto-instalação, gestão de pacotes)
- TL-407 (auto-instalar pacote faltante e recompilar)

## v0.4.0 — Histórico local

- TL-601..TL-608, TL-610, TL-611 (git embutido, checkpoints, versões, timeline, diff fonte, restaurar, exportar)

## v0.5.0 — Produtividade

- EP05 inteiro (snippets)
- TL-304..TL-308 (autocomplete, outline, refs, multi-arquivo, busca)
- TL-803..TL-810 (templates, bibliografia)

## v0.6.0 — Git remoto

- EP07 inteiro (auth, clone, push/pull, branches, merge 3-way)

## v0.7.0 — Diff visual e polimento

- TL-609 (diff visual de PDF)
- TL-309, TL-310 (ortografia)
- TL-408..TL-410 (rascunho, múltiplos PDFs, export)
- TL-903..TL-908 (APT, associação de arquivos, integração SO)

## v1.0.0 — MVP

- EP10 inteiro (testes E2E por SO, desempenho, acessibilidade, crash report, documentação)
- Correções e hardening; nenhuma história "Must" aberta

## Pós-v1.0

- EP11 (colaboração em tempo real, assistente IA, etc.)
