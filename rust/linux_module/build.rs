use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set"));
    let zig_out_lib = manifest_dir
        .parent()
        .expect("manifest_dir must have parent")
        .parent()
        .expect("manifest_dir must have grandparent")
        .join("zig/zig-out/lib");
    println!("cargo:rustc-link-search=native={}", zig_out_lib.display());

    // Link Zig libraries.
    // NOTE: libebpf_coldpage.a includes all symbols from ebpf/loader.zig plus
    // unique cold page detector functions. We link it INSTEAD of libebpf_loader.a
    // to avoid duplicate symbol errors.
    println!("cargo:rustc-link-lib=static=linux_zig");
    println!("cargo:rustc-link-lib=static=cpu_pinning");
    println!("cargo:rustc-link-lib=static=cgroup_manager");
    println!("cargo:rustc-link-lib=static=ebpf_coldpage");
    println!("cargo:rustc-link-lib=static=iouring_wrapper");
    println!("cargo:rustc-link-lib=static=criu_hibernation");
    println!("cargo:rustc-link-lib=static=preprocess_bloom");
    println!("cargo:rustc-link-lib=static=preprocess_vector");

    println!("cargo:rerun-if-changed={}", zig_out_lib.display());

    let bindings = bindgen::Builder::default()
        .header("../../zig/src/zig_bindings.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set"));
    bindings
        .write_to_file(out_path.join("zig_bindings.rs"))
        .expect("Couldn't write bindings!");
}
