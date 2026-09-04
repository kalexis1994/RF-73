//! Package and smoke-test through RackForge's own Rust tools.
use std::{error::Error, fs, path::Path, process::Command};

pub fn build() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("cannot resolve the workspace root")?;
    let host = root
        .parent()
        .ok_or("cannot resolve sibling host")?
        .join("rackforge");
    let tools = host.join("target/release");
    let store = tools.join(format!("rackforge-store{}", std::env::consts::EXE_SUFFIX));
    let core = tools.join(format!("rackforge-core{}", std::env::consts::EXE_SUFFIX));
    let component = root.join("target/wasm32-unknown-unknown/release/rf_rhodes_plugin.wasm");
    let package = root.join("package");
    let dist = root.join("dist");
    let output = dist.join("RF-Rhodes-0.1.0.rfplugin");
    if output.exists() {
        return Err(format!("refusing to overwrite {}", output.display()).into());
    }
    for file in [&store, &core, &component] {
        if !file.is_file() {
            return Err(format!("build the required artifact first: {}", file.display()).into());
        }
    }
    fs::create_dir_all(&dist)?;
    fs::copy(&component, package.join("component.wasm"))?;
    run(Command::new(&core).arg("inspect").arg(&package))?;
    run(Command::new(&core)
        .arg("smoke")
        .arg(&package)
        .arg("--preset")
        .arg("research-direct")
        .arg("--data-root")
        .arg(dist.join("smoke-data")))?;
    run(Command::new(&store)
        .arg("pack-wasm")
        .arg(&package)
        .arg(&component)
        .arg(&output))?;
    println!("Validated package: {}", output.display());
    Ok(())
}

fn run(command: &mut Command) -> Result<(), Box<dyn Error>> {
    let status = command.status()?;
    if !status.success() {
        return Err(format!("RackForge validation failed: {status}").into());
    }
    Ok(())
}
