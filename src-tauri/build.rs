fn main() {
    tauri_build::build();
    // Tauri needs Common Controls 6 on Windows. tauri-build embeds that manifest into the app
    // binary only; without it the unit test binary dies at start-up (STATUS_ENTRYPOINT_NOT_FOUND).
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    if msvc && let Ok(out) = std::env::var("OUT_DIR") {
        let manifest = std::path::Path::new(&out).join("tests.manifest");
        let text = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency><dependentAssembly>
    <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
      processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
  </dependentAssembly></dependency>
</assembly>"#;
        if std::fs::write(&manifest, text).is_ok() {
            println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
            println!("cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}", manifest.display());
        }
    }
}
