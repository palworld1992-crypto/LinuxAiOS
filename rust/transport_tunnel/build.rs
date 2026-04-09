use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set"));

    let spark_dir = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("spark"))
        .map_or(PathBuf::from("/home/ToHung/LinusAiOS/spark"), |v| v);

    let lib_dir = spark_dir.join("lib");

    if lib_dir.exists() {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    }

    let adalib_path = find_adalib();
    if let Some(ref path) = adalib_path {
        println!("cargo:rustc-link-search=native={}", path);
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", path);
    }

    println!("cargo:rerun-if-changed=build.rs");
}

fn find_adalib() -> Option<String> {
    let known_paths = [
        "/usr/libexec/spark/lib/gcc/x86_64-pc-linux-gnu/15.1.0/adalib",
        "/usr/libexec/gcc/x86_64-pc-linux-gnu/15.1.0/adalib",
        "/usr/lib/gcc/x86_64-pc-linux-gnu/15.1.0/adalib",
        "/usr/lib64/gcc/x86_64-pc-linux-gnu/15.1.0/adalib",
        "/usr/local/lib/gcc/x86_64-pc-linux-gnu/15.1.0/adalib",
    ];

    for p in &known_paths {
        let libgnat = PathBuf::from(p).join("libgnat_pic.a");
        if libgnat.exists() {
            return Some(p.to_string());
        }
    }

    None
}
