# winget manifest template

Fill in `VERSION` and `SHA256` (from the release's `SHA2-256SUMS`) in the three files, then:

```powershell
winget validate --manifest packaging\winget
winget install --manifest packaging\winget
```

Submit to [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) under
`manifests/f/FlyShip09/lovelyDL/VERSION/`, or let
[wingetcreate](https://github.com/microsoft/winget-create) do it:
`wingetcreate update FlyShip09.lovelyDL --version VERSION --urls <installer URL>`.

The schema version (1.10.0) may need raising to whatever winget-pkgs accepts at the time.
