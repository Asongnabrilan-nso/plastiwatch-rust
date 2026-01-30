fn main() {
    embuild::espidf::sysenv::output();

    if std::env::var("CARGO_FEATURE_EDGE_IMPULSE").is_ok() {
        // Find the C++ compiler in the Embuild toolchain directory
        // Typically: .embuild/espressif/tools/riscv32-esp-elf/esp-<VER>/riscv32-esp-elf/bin/riscv32-esp-elf-g++
        let compiler = find_compiler().unwrap_or_else(|| "riscv32-esp-elf-g++".into());
        std::env::set_var("CXX", &compiler); // Helpful for debugging
        build_ei(&compiler);
    }
}

fn find_compiler() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    // Check local .embuild first, then global ~/.espressif
    let search_dirs = vec![
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(".embuild"),
        dirs::home_dir().map(|h| h.join(".espressif")).unwrap_or_default(),
    ];

    for root in search_dirs {
        let tools_dir = root.join("espressif/tools/riscv32-esp-elf");
        if tools_dir.exists() {
            // Find the versioned directory (e.g., esp-13.2.0_20240530)
            if let Ok(entries) = std::fs::read_dir(&tools_dir) {
                 for entry in entries.flatten() {
                     let path = entry.path();
                     if path.is_dir() {
                         let candidate = path.join("riscv32-esp-elf/bin/riscv32-esp-elf-g++");
                         if candidate.exists() {
                             return Some(candidate);
                         }
                     }
                 }
            }
        }
    }
    None
}

fn build_ei(compiler_path: &std::path::Path) {
    use std::path::{Path, PathBuf};

    let sdk_root = PathBuf::from("motion-detection_inferencing");
    
    let mut build = cc::Build::new();
    
    build
        .cpp(true)
        .compiler(compiler_path) // Explicitly set the compiler path

        .cpp(true)
        .flag("-std=c++14")
        .flag("-O3")
        .flag("-g3")
        .define("EI_CLASSIFIER_ENABLE_DETECTION_3D", "0")
        .define("EI_CLASSIFIER_TFLITE_ENABLE_CMSIS_NN", "0")
        .define("EI_NATIVE_ARCH", "1")
        .include(&sdk_root)
        .include(sdk_root.join("src"))
        .include(sdk_root.join("src/edge-impulse-sdk"))
        .include(sdk_root.join("src/model-parameters"))
        .include(sdk_root.join("src/tflite-model"));

    // Recursively add source files
    add_source_files(&mut build, &sdk_root.join("src"));

    build.compile("edge-impulse-sdk");

    println!("cargo:rerun-if-changed=motion-detection_inferencing");
}

fn add_source_files(build: &mut cc::Build, dir: &std::path::Path) {
    for entry in std::fs::read_dir(dir).expect("Failed to read directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        
        if path.is_dir() {
            // Basic heuristic to skip some non-source dirs if necessary, 
            // but for now we follow the structure.
            // Explicitly skip 'porting' to avoid conflict if not needed, 
            // OR we might need 'porting/espressif' if it exists. 
            // User instruction didn't specify, so compiling all cpp/c files is standard for EI 
            // as long as we define EI_NATIVE_ARCH which usually selects generic implementations.
            add_source_files(build, &path);
        } else {
            if let Some(ext) = path.extension() {
                if ext == "c" || ext == "cpp" || ext == "cc" {
                    build.file(&path);
                }
            }
        }
    }
}
