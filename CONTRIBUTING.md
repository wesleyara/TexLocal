# Contribuindo

## Ambiente

- Node (versão em `.nvmrc`) e pnpm (versão em `packageManager`).
- Rust na versão de `rust-toolchain.toml` (instalada automaticamente pelo `rustup`).
- Dependências do Tauri para o seu SO: <https://v2.tauri.app/start/prerequisites/>.

```bash
pnpm install   # também instala os hooks do lefthook
pnpm tauri dev
```

## Fim de linha: sempre LF (Windows)

Todo o repositório usa LF (`.gitattributes`, `.editorconfig`, Prettier, rustfmt). No Windows:

```bash
git config --global core.autocrlf false   # ou "input"
```

No VS Code, o `.vscode/settings.json` do repositório já define `"files.eol": "\n"`. O CI falha se algum arquivo rastreado contiver CRLF (`pnpm check:lf`).

## Commits e PRs

Conventional Commits são obrigatórios (validados pelo hook `commit-msg` e pelo CI, incluindo o título do PR, que vira a mensagem do squash merge).

Formato: `tipo(escopo): descrição`. Tipos: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, `chore`, `revert`.

Escopos permitidos: `editor`, `tex`, `git`, `compile`, `preview`, `snippets`, `history`, `projects`, `templates`, `bib`, `ui`, `i18n`, `settings`, `ci`, `release`, `deps`, `docs`, `core`.

### Versionamento

SemVer, derivado dos commits pelo release-please:

- `fix` incrementa patch; `feat` incrementa minor (enquanto 0.x).
- `BREAKING CHANGE:` no rodapé ou `!` após o tipo (`feat(git)!: ...`) incrementa major (em 0.x, minor).
- O `CHANGELOG.md` é gerado automaticamente: não edite manualmente.

## Qualidade

```bash
pnpm lint && pnpm format:check && pnpm typecheck && pnpm test
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --workspace
```

Veja a Definition of Done em [docs/backlog/convencoes.md](docs/backlog/convencoes.md).
