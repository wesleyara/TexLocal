# 21. Versionamento

- Status: aceito
- Data: 2026-11-09
- Referência: D21 em [decisoes.md](../backlog/decisoes.md)

## Contexto e problema

Decisão de produto/arquitetura do TexLocal sobre **Versionamento**, tomada na sessão de sabatina inicial.

## Decisão

**SemVer** + **Conventional Commits** (validado no CI) + **release-please** (CHANGELOG, bump sincronizado em `Cargo.toml`/`package.json`/`tauri.conf.json`, tag)

## Alternativas consideradas

- Tag manual
- CalVer

## Consequências

Esta decisão orienta as histórias dos épicos relacionados; mudanças exigem um novo ADR que substitua este.
