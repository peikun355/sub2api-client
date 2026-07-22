# Sub2API Client community preview

Community preview builds are the no-cost distribution channel for early testing.
The project signs update payloads with its own Tauri key and publishes SHA-256
checksums, but does not claim an Apple or Microsoft verified publisher identity.

Download only from `https://ai.tavonilo.com` or the official Sub2API Client GitHub
repository and compare the displayed SHA-256 value before opening a package.

## Windows

Prefer the Portable ZIP. Extract it to a new folder and open `sub2api-client.exe`.
Windows SmartScreen may show **Unknown publisher**. After verifying the checksum,
choose **More info** and then **Run anyway**. The MSI has the same publisher warning.

## macOS

Open the DMG, copy **Sub2API Client.app** into Applications, then right-click the
app and choose **Open**. If macOS still blocks it, verify the checksum and run:

```bash
xattr -dr com.apple.quarantine '/Applications/Sub2API Client.app'
open '/Applications/Sub2API Client.app'
```

The app is ad-hoc signed for bundle-integrity checks but is not Apple-notarized.

The website can use the fixed manifest URL below; each manifest entry still points
to immutable, versioned release assets:

`https://github.com/peikun355/sub2api-client/releases/download/community-latest/client-download-manifest.json`

## Linux

AppImage and DEB packages do not require a paid publisher certificate. Verify the
SHA-256 value before installing. For AppImage:

```bash
chmod +x Sub2API-Client-*-Linux-x86_64.AppImage
./Sub2API-Client-*-Linux-x86_64.AppImage
```
