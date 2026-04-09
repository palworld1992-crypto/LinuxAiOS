use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set"));

    // 1. Xác định thư mục spark (aios/spark)
    let spark_dir = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("spark"))
        .map_or(PathBuf::from("/home/ToHung/LinusAiOS/spark"), |v| v);

    let lib_dir = spark_dir.join("lib");

    // --- 2. Biên dịch Ada (GPRBuild) nếu cần ---
    let spark_gpr = spark_dir.join("spark.gpr");
    let lib_exists = lib_dir.join("libscc.a").exists() || lib_dir.join("libscc.so").exists();

    if spark_gpr.exists() && !lib_exists {
        let _ = Command::new("alr")
            .args([
                "exec",
                "--",
                "gprbuild",
                "-P",
                &spark_gpr.display().to_string(),
                "-p",
                "-f",
                "-cargs",
                "-fPIC",
            ])
            .output();
    }

    // --- 3. Linker Search Paths ---
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-search=native=/usr/lib");

    // Thêm rpath để runtime tìm được libscc.so
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());

    // --- 4. Tìm Ada runtime (libgnat, libgnarl) ---
    let adalib_path = find_adalib();
    if let Some(ref path) = adalib_path {
        println!("cargo:rustc-link-search=native={}", path);
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", path);
    } else {
        println!("cargo:warning=KHÔNG tìm thấy adalib. Quá trình link có thể thất bại.");
    }

    // --- 5. Link OQS wrapper FIRST (must be before libscc.so) ---
    // libscc.so depends on liboqs_wrapper.so, so link it first
    if lib_dir.join("liboqs_wrapper.so").exists() {
        println!("cargo:rustc-link-arg=-Wl,--no-as-needed");
        println!("cargo:rustc-link-lib=dylib=oqs_wrapper");
        println!("cargo:rustc-link-arg=-Wl,--as-needed");
    }

    // --- 6. Link SCC library ---
    if lib_dir.join("libscc.so").exists() {
        println!("cargo:rustc-link-lib=dylib=scc");
    } else if lib_dir.join("libscc.a").exists() {
        println!("cargo:rustc-link-lib=static=scc");
    } else {
        println!(
            "cargo:warning=No libscc.a or libscc.so found at {}",
            lib_dir.display()
        );
    }

    // --- 6. Link liboqs (mật mã lượng tử) ---
    if lib_dir.join("liboqs.a").exists() {
        println!("cargo:rustc-link-lib=static=oqs");
    }

    // --- 7. Ada Runtime ---
    println!("cargo:rustc-link-lib=gnat");
    println!("cargo:rustc-link-lib=gnarl");

    // --- 8. Symbol khởi tạo ---
    println!("cargo:rustc-link-arg=-Wl,--undefined=sccinit");

    // --- 9. Thư viện hệ thống ---
    println!("cargo:rustc-link-lib=crypto");
    println!("cargo:rustc-link-lib=ssl");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=dl");

    // --- 10. Rebuild triggers ---
    println!("cargo:rerun-if-changed={}", spark_dir.display());
    println!("cargo:rerun-if-changed=build.rs");
}

/// Tìm đường dẫn adalib chứa libgnat/libgnarl
fn find_adalib() -> Option<String> {
    // 1. Known paths (thử lần lượt)
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

    // 2. Thử từ gnatls -v
    if let Ok(out) = Command::new("gnatls").arg("-v").output() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if line.contains("adalib") && line.starts_with('/') {
                let path = PathBuf::from(line);
                if path.join("libgnat_pic.a").exists() {
                    return Some(line.to_string());
                }
            }
        }
    }

    // 3. Thử từ gprls -v
    if let Ok(out) = Command::new("gprls").arg("-v").output() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        for line in stderr.lines().map(|l| l.trim()) {
            if line.contains("adalib") && line.starts_with('/') {
                let path = PathBuf::from(line);
                if path.join("libgnat_pic.a").exists() {
                    return Some(line.to_string());
                }
            }
        }
    }

    None
}
