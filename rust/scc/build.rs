use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // 1. Xác định thư mục spark (aios/spark)
    let spark_dir = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("spark"))
        .expect("Không thể xác định thư mục spark");

    let lib_dir = spark_dir.join("lib");

    // --- 2. Biên dịch Ada (GPRBuild) ---
    let spark_gpr = spark_dir.join("spark.gpr");

    // Kiểm tra xem thư viện đã tồn tại chưa - nếu có thì bỏ qua build
    let lib_exists = lib_dir.join("libscc.a").exists() || lib_dir.join("libscc.so").exists();

    // Thử sử dụng alr exec để chạy gprbuild với môi trường đúng
    // Nếu thư viện đã tồn tại, sử dụng thư viện hiện có
    let gprbuild_success = if spark_gpr.exists() && !lib_exists {
        // Thử alr exec trước
        let alr_output = Command::new("alr")
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

        match alr_output {
            Ok(out) if out.status.success() => {
                println!("cargo:warning=GPRBuild compiled successfully via alr exec.");
                true
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                eprintln!("GPRBuild (alr) failed: {}", stderr);
                false
            }
            Err(e) => {
                eprintln!("alr exec failed: {}", e);
                false
            }
        }
    } else {
        // Thư viện đã tồn tại hoặc không có file gpr
        // Sử dụng thư viện hiện có một cách im lặng
        lib_exists
    };

    // Kiểm tra lại sau khi thử build
    let final_lib_exists = lib_dir.join("libscc.a").exists() || lib_dir.join("libscc.so").exists();

    if !gprbuild_success && !final_lib_exists {
        panic!(
            "GPRBuild thất bại và không tìm thấy thư viện tại {}",
            lib_dir.display()
        );
    }
    // Nếu thư viện tồn tại, sử dụng im lặng mà không in warning

    // --- 3. Linker Search Paths ---
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // Tìm thư mục adalib (chứa libgnat.a, libgnarl.a)
    let mut found_adalib = false;
    let mut adalib_path: Option<String> = None;

    // Thử tìm qua gprls
    if let Ok(out) = Command::new("gprls").arg("-v").output() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        for line in stderr.lines().map(|l| l.trim()) {
            if line.contains("adalib") && line.starts_with('/') {
                println!("cargo:rustc-link-search=native={}", line);
                found_adalib = true;
                adalib_path = Some(line.to_string());
                break;
            }
        }
    }

    // Nếu gprls không thấy, dùng lệnh find
    if !found_adalib {
        let alire_path = "/root/.local/share/alire/toolchains";
        if let Ok(entries) = Command::new("find")
            .args([alire_path, "-name", "adalib", "-type", "d"])
            .output()
        {
            let paths = String::from_utf8_lossy(&entries.stdout);
            if let Some(first_path) = paths.lines().next() {
                println!("cargo:rustc-link-search=native={}", first_path);
                found_adalib = true;
                adalib_path = Some(first_path.to_string());
            }
        }
    }

    if !found_adalib {
        println!("cargo:warning=KHÔNG tìm thấy adalib. Quá trình link có thể thất bại.");
    }

    // --- 4. Linker flags ---

    // Link tĩnh libscc để chắc chắn có đầy đủ symbol FFI (vd: type_mapper_map_type)
    println!("cargo:rustc-link-lib=static=scc");

    // Thư viện oqs (mật mã) link tĩnh
    println!("cargo:rustc-link-lib=static=oqs");

    // Thư viện Ada Runtime (dùng dynamic vì bản static không PIC, không link được với PIE)
    println!("cargo:rustc-link-lib=gnat");
    println!("cargo:rustc-link-lib=gnarl");

    // --- 5. Symbol khởi tạo + runtime search path ---
    println!("cargo:rustc-link-arg=-Wl,--undefined=sccinit");
    if let Some(path) = &adalib_path {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", path);
    }

    // --- 6. Thư viện hệ thống ---
    println!("cargo:rustc-link-lib=crypto");
    println!("cargo:rustc-link-lib=ssl");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=dl");

    // --- 7. Rebuild Triggers ---
    println!("cargo:rerun-if-changed={}", spark_dir.display());
    println!("cargo:rerun-if-changed=build.rs");
}
