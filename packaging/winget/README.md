# winget manifest (draft)

Draft manifests for submitting Cove to the Windows Package Manager community
repository ([microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs)) as
`zernivo.Cove`, version **0.9.0**.

**Status: not submitted. Waiting on code signing.**

1. **Code signing (blocking).** The v0.9.0 installer is not code-signed.
   winget accepts unsigned installers, but SmartScreen and Defender reputation
   checks in the winget pipeline flag new unsigned publishers often enough
   that submission waits for a signed build. Planned: SignPath Foundation
   (free for open source) for the NSIS setup, or a Microsoft Store MSIX
   package, which Microsoft signs. See [docs/ROADMAP.md](../../docs/ROADMAP.md).
2. **The real hash.** `InstallerSha256` is still a placeholder. Fill it from
   the published asset before submitting: download
   `Cove_0.9.0_x64-setup.exe` from the
   [v0.9.0 release](https://github.com/zernivo/Cove/releases/tag/v0.9.0) and run
   `(Get-FileHash .\Cove_0.9.0_x64-setup.exe).Hash`, or
   `wingetcreate update zernivo.Cove --version 0.9.0 --urls <url>`, which fills it in.
   A signed build will have a different hash, so fill it from the build you submit.

Installer URL pattern for any release:

```
https://github.com/zernivo/Cove/releases/download/v<version>/Cove_<version>_x64-setup.exe
```

The NSIS installer is per-user (no admin) and supports `/S` for a silent install.

To validate locally on Windows: `winget validate --manifest packaging\winget`
and test with `winget install --manifest packaging\winget`.
