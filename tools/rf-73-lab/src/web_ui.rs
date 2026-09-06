//! Build the Rust PLAY surface and its generated browser bindings.
use std::{error::Error, fs::OpenOptions, io::Write, process::Command};

pub fn verify_install(
    installed: &std::path::Path,
    source: &std::path::Path,
) -> Result<(), Box<dyn Error>> {
    for asset in [
        "web/play.html",
        "web/style.css",
        "web/app.js",
        "web/app_bg.wasm",
    ] {
        if std::fs::read(installed.join(asset))? != std::fs::read(source.join(asset))? {
            return Err(format!(
                "installed UI asset differs from the freshly built surface: {asset}"
            )
            .into());
        }
    }
    Ok(())
}

pub fn build() -> Result<(), Box<dyn Error>> {
    let root = super::package::workspace_root()?;
    let mut version = Command::new("wasm-bindgen");
    version.arg("--version");
    super::package::hide_console(&mut version);
    let output = version
        .output()
        .map_err(|_| "install wasm-bindgen-cli 0.2.127 to build the Rust UI")?;
    if !output.status.success()
        || String::from_utf8_lossy(&output.stdout).trim() != "wasm-bindgen 0.2.127"
    {
        return Err("the Rust UI requires wasm-bindgen-cli exactly 0.2.127".into());
    }
    super::package::run(Command::new("cargo").current_dir(&root).args([
        "build",
        "--locked",
        "--release",
        "--target",
        "wasm32-unknown-unknown",
        "-p",
        "rf-73-ui",
    ]))?;
    let web = root.join("package/web");
    super::package::run(
        Command::new("wasm-bindgen")
            .arg(root.join("target/wasm32-unknown-unknown/release/rf_73_ui.wasm"))
            .arg("--out-dir")
            .arg(&web)
            .args(["--out-name", "app", "--target", "web", "--no-typescript"]),
    )?;
    // Bootstrap is generated alongside the ABI glue. All interaction logic is Rust.
    let mut script = OpenOptions::new().append(true).open(web.join("app.js"))?;
    script.write_all(b"\n// Generated Rust UI bootstrap.\n__wbg_init();\n")?;
    println!("Built RF-73 Rust PLAY surface.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_or_stale_ui_assets_reject_installation() {
        let root = std::env::temp_dir().join(format!(
            "rf-73-ui-install-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source = root.join("source");
        let installed = root.join("installed");
        std::fs::create_dir_all(source.join("web")).unwrap();
        std::fs::create_dir_all(installed.join("web")).unwrap();
        for asset in ["play.html", "style.css", "app.js", "app_bg.wasm"] {
            std::fs::write(source.join("web").join(asset), asset).unwrap();
            assert!(verify_install(&installed, &source).is_err());
            std::fs::write(installed.join("web").join(asset), asset).unwrap();
        }
        verify_install(&installed, &source).unwrap();
        std::fs::write(installed.join("web/app_bg.wasm"), b"stale").unwrap();
        assert!(verify_install(&installed, &source).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
