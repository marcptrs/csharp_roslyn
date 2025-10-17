use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap();
    let host = env::var("HOST").unwrap();

    // Determine the proxy binary name based on host platform
    let binary_name = if host.contains("windows") {
        "roslyn-lsp-proxy.exe"
    } else {
        "roslyn-lsp-proxy"
    };

    if target == "wasm32-wasip1" {
        println!("cargo:warning=Building proxy for host platform before WASM build...");

        let status = Command::new("cargo")
            .args([
                "build",
                "--release",
                "--manifest-path=proxy/Cargo.toml",
                &format!("--target={}", host),
            ])
            .status()
            .expect("Failed to build proxy");

        if !status.success() {
            panic!("Proxy build failed");
        }

        println!("cargo:warning=Proxy build completed successfully");

        let proxy_path = format!("proxy/target/{}/release/{}", host, binary_name);

        // Verify the binary exists
        if !Path::new(&proxy_path).exists() {
            panic!("Proxy binary not found at: {}", proxy_path);
        }

        println!("cargo:rustc-env=PROXY_BINARY_PATH={}", proxy_path);
    } else {
        // For non-WASM builds (tests, etc), set a dummy path
        // This won't be used since PROXY_BINARY is only defined for WASM target
        let dummy_path = format!("proxy/target/{}/release/{}", host, binary_name);
        println!("cargo:rustc-env=PROXY_BINARY_PATH={}", dummy_path);
    }

    println!("cargo:rerun-if-changed=proxy/src");
    println!("cargo:rerun-if-changed=proxy/Cargo.toml");
}
