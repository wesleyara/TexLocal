# TexLocal

Editor LaTeX local, multiplataforma e autossuficiente, com histórico de versões estilo Overleaf sustentado por git real, snippets e gerenciamento automático da distribuição TeX.

> Status: pré-1.0, em desenvolvimento. O planejamento está em [docs/backlog](docs/backlog/README.md).

## Stack

Tauri 2 (Rust) · Vue 3 + TypeScript · Vite · Pinia · Vue Router

## Desenvolvimento

Pré-requisitos: Node 24 (`.nvmrc`), pnpm 10, Rust (via `rust-toolchain.toml`) e as [dependências do Tauri](https://v2.tauri.app/start/prerequisites/).

```bash
pnpm install
pnpm tauri dev
```

Verificações: `pnpm lint`, `pnpm typecheck`, `pnpm test`, `cargo test --manifest-path src-tauri/Cargo.toml --workspace`.

Veja [CONTRIBUTING.md](CONTRIBUTING.md) (inclui configuração para Windows) e as [decisões de arquitetura](docs/adr).

## Licença

[MIT](LICENSE)
