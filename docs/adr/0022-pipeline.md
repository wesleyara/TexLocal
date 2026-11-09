# 22. Pipeline

- Status: aceito
- Data: 2026-11-09
- Referência: D22 em [decisoes.md](../backlog/decisoes.md)

## Contexto e problema

Decisão de produto/arquitetura do TexLocal sobre **Pipeline**, tomada na sessão de sabatina inicial.

## Decisão

**Pipeline de release desde a Sprint 0**: GitHub Actions gera e assina (Windows Authenticode, macOS assinatura + notarização, assinatura do updater) os artefatos das 3 plataformas e publica GitHub Release

## Alternativas consideradas

- CI sem assinatura

## Consequências

Esta decisão orienta as histórias dos épicos relacionados; mudanças exigem um novo ADR que substitua este.
