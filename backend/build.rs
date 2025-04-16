use std::fs;
use std::process::Command;

fn main() {
    let target_folder = "../target";
    let output_path = format!("{}/{}", target_folder, "www");
    // compile web-projekt
    Command::new("cargo")
        .current_dir("../web") // Wechsle in das Verzeichnis des `web`-Projekts
        .arg("build")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--release")
        .status()
        .expect("Failed to compile the Web project");

    let wasm_file = "../target/wasm32-unknown-unknown/release/web.wasm";

    // run wasm-bindgen
    let status = Command::new("wasm-bindgen")
        .arg("--out-dir")
        .arg(&output_path)
        .arg("--target")
        .arg("web")
        .arg(&wasm_file)
        .status()
        .expect("Failed to execute wasm-bindgen");

    if !status.success() {
        panic!("wasm-bindgen failed");
    }
    let source_html = "../web/index.html";
    let target_html = format!("{}/index.html", output_path);
    fs::copy(source_html, target_html).expect("Failed to copy index.html");
}
