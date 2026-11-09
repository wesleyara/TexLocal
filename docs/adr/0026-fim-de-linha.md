# 26. Fim de linha

- Status: aceito
- Data: 2026-11-09
- Referência: D26 em [decisoes.md](../backlog/decisoes.md)

## Contexto e problema

Decisão de produto/arquitetura do TexLocal sobre **Fim de linha**, tomada na sessão de sabatina inicial.

## Decisão

Desenvolvimento no Windows, mas **todo o repositório usa LF** (`.gitattributes` `eol=lf`, `.editorconfig`, Prettier/rustfmt, checagem no CI). No produto, o editor salva novos arquivos em LF e preserva o fim de linha de arquivos existentes

## Alternativas consideradas

- CRLF / autocrlf

## Consequências

Esta decisão orienta as histórias dos épicos relacionados; mudanças exigem um novo ADR que substitua este.
