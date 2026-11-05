# winget manifest (draft)

Draft manifests for submitting Cove to the Windows Package Manager community
repository ([microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs)) as
`zernivo.Cove`, version **0.9.0**.

**Not ready to submit.** Two things first:

1. **Code signing.** The installer is not code-signed yet. winget accepts
   unsigned installers, but SmartScreen and Defender reputation checks in the
   winget pipeline flag new unsigned publishers often enough that submission
   should wait for a signed build (planned through SignPath for open source).
2. **The real hash.** `InstallerSha256` is a placeholder. After the release is
   published, download the asset and run
   `(Get-FileHash .\Cove_0.9.0_x64-setup.exe).Hash`, or
   `wingetcreate update zernivo.Cove --version 0.9.0 --urls <url>`, which fills it in.

Installer URL pattern for any release:

```
https://github.com/zernivo/Cove/releases/download/v<version>/Cove_<version>_x64-setup.exe
```

The NSIS installer is per-user (no admin) and supports `/S` for a silent install.

To validate locally on Windows: `winget validate --manifest packaging\winget`
and test with `winget install --manifest packaging\winget`.
