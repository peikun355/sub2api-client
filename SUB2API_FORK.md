# Sub2API Client fork policy

Sub2API Client is based on [CC Switch](https://github.com/farion1231/cc-switch) v3.18.0,
fixed at upstream commit `606e7bbe75db7f8285f7a3be006fac22b5d22796`.

The upstream MIT license is preserved in `LICENSE`. The `upstream` Git remote is kept
read-only for synchronization. Product branding, the beginner experience, and the
`sub2api://` setup protocol live in dedicated modules to keep upstream merges small.

- Review upstream changes monthly.
- Merge security fixes as soon as they are available.
- Production builds trust only `https://ai.tavonilo.com`; localhost overrides are debug-only.
- A stable release requires Windows Authenticode credentials, Apple Developer ID and
  notarization credentials, Tauri updater signing credentials, and domestic mirror credentials.

## Local updater signing key

The fork uses its own Tauri updater key instead of the CC Switch upstream key. On the
release Mac, the encrypted private key is stored at:

`~/Library/Application Support/Sub2API Client/signing/sub2api-client.key`

Its password is stored in the macOS Keychain service
`sub2api-client-tauri-signing-password`. The public key is committed in
`src-tauri/tauri.conf.json`. Configure GitHub Actions with the private key contents as
`TAURI_SIGNING_PRIVATE_KEY` and the Keychain password as
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`; never commit either value.
The randomly generated CI keychain password used for `KEYCHAIN_PASSWORD` is stored
in the macOS Keychain service `sub2api-client-ci-keychain-password`.

Windows releases support either a base64-encoded PFX (`WINDOWS_CERTIFICATE` and
`WINDOWS_CERTIFICATE_PASSWORD`) or Azure Artifact Signing. The Azure path uses
GitHub OIDC and requires `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`,
`AZURE_SUBSCRIPTION_ID`, `AZURE_ARTIFACT_SIGNING_ENDPOINT`,
`AZURE_ARTIFACT_SIGNING_ACCOUNT_NAME`, and
`AZURE_ARTIFACT_SIGNING_PROFILE_NAME`. Assign the app registration the
`Artifact Signing Certificate Profile Signer` role and configure a federated
credential for the repository's `release` environment.

## Free community preview channel

Tags matching `community-v*` publish a GitHub prerelease without requiring paid
Apple or Windows publisher certificates or domestic OSS credentials. These builds
still require the private Tauri updater key and include SHA-256 checksums. Windows
artifacts are unsigned at the operating-system publisher layer; macOS artifacts use
an ad-hoc signature and are not notarized. They must never be marked as a stable
release. See `COMMUNITY_RELEASE.md` for user-facing installation instructions.

Tags matching `v*` remain the stable channel and continue to require Windows
Authenticode or Azure Artifact Signing, Apple Developer ID plus notarization, and
the domestic mirror credentials.
