use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let zig_out_lib = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("zig/zig-out/lib");
    println!("cargo:rustc-link-search=native={}", zig_out_lib.display());
    println!("cargo:rustc-link-lib=static=linux_zig");
    println!(
        "cargo:rerun-if-changed={}",
        zig_out_lib.join("liblinux_zig.a").display()
    );

    let bindings = bindgen::Builder::default()
        .header("../../zig/src/zig_bindings.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("zig_bindings.rs"))
        .expect("Couldn't write bindings!");
}
