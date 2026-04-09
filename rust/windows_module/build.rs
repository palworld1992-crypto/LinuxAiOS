use std::env;

fn main() {
    // When feature `use_libvirt` is enabled, Cargo exposes the env var CARGO_FEATURE_USE_LIBVIRT.
    // Only link libvirt when the feature is enabled to avoid build failures on systems
    // without libvirt development packages.
    if env::var("CARGO_FEATURE_USE_LIBVIRT").is_ok() {
        // Link with libvirt: libvirt.so -> -lvirt
        println!("cargo:rustc-link-lib=dylib=virt");
    }
}
