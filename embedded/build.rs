use std::fs;
use std::fs::File;
use std::path::Path;
use std::process::Command;
use flate2::write::GzEncoder;
use flate2::Compression;

fn main() {
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
    let target_folder = "../target";
    let output_path = format!("{}/{}", target_folder, "www");
    // Web-Projekt für WebAssembly kompilieren
    Command::new("cargo")
        .current_dir("../web") // Wechsle in das Verzeichnis des `web`-Projekts
        .arg("build")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--release")
        .status()
        .expect("Failed to compile the Web project");
    // // Pfad zur WebAssembly-Zieldatei
    let wasm_file = "../target/wasm32-unknown-unknown/release/web.wasm";

    // wasm-bindgen ausführen
    let wasm_bindgen_status = Command::new("wasm-bindgen")
        .arg("--out-dir")
        .arg(&output_path)
        .arg("--target")
        .arg("web")
        .arg(&wasm_file)
        .status()
        .expect("Failed to execute wasm-bindgen");

    if !wasm_bindgen_status.success() {
        panic!("wasm-bindgen failed");
    }
    // Kopiere die .html-Datei aus dem web-Ordner in den www-Ordner
    let source_html = "../web/index.html";
    let target_html = format!("{}/index.html", output_path);
    fs::copy(source_html, target_html).expect("Failed to copy index.html");

    let web_asset_dir = Path::new("../target/www");

    if let Ok(entries) = fs::read_dir(web_asset_dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() || path.extension().map(|e| e == "gz").unwrap_or(false) {
                continue;
            }

            if let Ok(mut input) = File::open(&path) {
                let mut gz_path = path.clone();
                gz_path.set_extension(format!("{}.gz", path.extension().unwrap().display()));
                if let Ok(output) = File::create(&gz_path) {
                    let mut encoder = GzEncoder::new(output, Compression::new(9));

                    if std::io::copy(&mut input, &mut encoder).is_ok() {
                        if encoder.finish().is_ok() {
                            println!("cargo:rerun-if-changed={}", path.display());
                            println!("cargo:rerun-if-changed={}", gz_path.display());
                        }
                    }
                }
            }
        }
    }

}
