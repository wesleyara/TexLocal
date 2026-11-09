# Certificados e segredos de assinatura (TL-007)

Passos **manuais** do mantenedor: dependem de contas e custos externos. Enquanto os segredos não existirem, o workflow `release.yml` continua e marca a release como _unsigned_.

## Segredos do repositório (GitHub → Settings → Secrets)

| Segredo                                                              | Uso                                                         | Como obter                                                                                                                                 |
| -------------------------------------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`    | Assinatura dos artefatos do updater (`.sig`)                | `pnpm tauri signer generate -w ~/.tauri/texlocal.key`; coloque a **chave pública** em `plugins.updater.pubkey` do `tauri.conf.json` (EP09) |
| `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`                    | Developer ID Application (.p12 em base64)                   | Apple Developer Program (~US$99/ano)                                                                                                       |
| `APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`            | Notarização via App Store Connect API key                   | App Store Connect → Users and Access → Keys                                                                                                |
| `WINDOWS_CERTIFICATE` (+ senha) ou credenciais Azure Trusted Signing | Authenticode do `.msi`                                      | Azure Trusted Signing (preferido) ou certificado EV                                                                                        |
| `GPG_PRIVATE_KEY`, `GPG_PASSPHRASE`                                  | Assinar o repositório APT e o `.deb` (EP09)                 | `gpg --full-generate-key`                                                                                                                  |
| `RELEASE_PLEASE_TOKEN`                                               | PAT/GitHub App para o release-please disparar `release.yml` | Fine-grained PAT com `contents` e `pull-requests` write                                                                                    |

Com a chave do updater definida, o `release.yml` já aplica `src-tauri/tauri.updater.conf.json` quando `TAURI_SIGNING_PRIVATE_KEY` existe.

## Branch protection (manual)

Em `main`: exigir CI verde (jobs `check`, `lf`, `commits`) e 1 revisão; permitir apenas squash merge.

## Rotação e expiração

| Item                           | Validade típica        | Ação                                                                                          |
| ------------------------------ | ---------------------- | --------------------------------------------------------------------------------------------- |
| Developer ID / notarização     | 5 anos / anual (conta) | Renovar a conta anualmente; recriar o certificado antes de expirar                            |
| Authenticode / Trusted Signing | 1–3 anos               | Renovar 30 dias antes; atualizar segredo                                                      |
| Chave do updater               | Não expira             | Rotacionar só em comprometimento (exige release com nova `pubkey` assinada pela chave antiga) |
| Chave GPG                      | 2 anos (sugerido)      | Estender validade ou publicar nova chave no repositório APT                                   |

Registre aqui a data de expiração de cada item ao obtê-lo.
