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
    use std::path::PathBuf;

    let sdk_root = PathBuf::from("motion-detection_inferencing");
    
    // Get ESP-IDF include paths from environment variables set by embuild
    // embuild::espidf::sysenv::output() sets these in the build environment
    let mut esp_idf_includes = Vec::new();
    
    // Method 1: Check DEP_ESP_IDF_SYS_INCLUDE (set by esp-idf-sys crate via cargo)
    // This is the most reliable method as it's set by the esp-idf-sys build script
    if let Ok(include_paths) = std::env::var("DEP_ESP_IDF_SYS_INCLUDE") {
        for path_str in include_paths.split_whitespace() {
            let path = PathBuf::from(path_str);
            if path.exists() {
                esp_idf_includes.push(path);
            }
        }
    }
    
    // Method 2: Check DEP_ESP_IDF_SYS_IDF_PATH and construct common include paths
    if let Ok(idf_path_str) = std::env::var("DEP_ESP_IDF_SYS_IDF_PATH") {
        let idf_path = PathBuf::from(idf_path_str);
        let common_paths = vec![
            idf_path.join("components"),
            idf_path.join("components/esp_timer/include"),
            idf_path.join("components/freertos/FreeRTOS-Kernel/include"),
            idf_path.join("components/freertos/FreeRTOS-Kernel/portable/riscv/include"),
            idf_path.join("components/log/include"),
            idf_path.join("components/esp_common/include"),
        ];
        for path in common_paths {
            if path.exists() && !esp_idf_includes.contains(&path) {
                esp_idf_includes.push(path);
            }
        }
    }
    
    // Method 3: Check IDF_PATH (if set manually in environment)
    if let Ok(idf_path_str) = std::env::var("IDF_PATH") {
        let idf_path = PathBuf::from(idf_path_str);
        let common_paths = vec![
            idf_path.join("components"),
            idf_path.join("components/esp_timer/include"),
            idf_path.join("components/freertos/FreeRTOS-Kernel/include"),
            idf_path.join("components/freertos/FreeRTOS-Kernel/portable/riscv/include"),
            idf_path.join("components/log/include"),
            idf_path.join("components/esp_common/include"),
        ];
        for path in common_paths {
            if path.exists() && !esp_idf_includes.contains(&path) {
                esp_idf_includes.push(path);
            }
        }
    }
    
    // Method 4: Search in .embuild directory (where embuild installs ESP-IDF)
    if esp_idf_includes.is_empty() {
        let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let embuild_dir = manifest_dir.join(".embuild");
        if embuild_dir.exists() {
            // Look for esp-idf in .embuild/espressif/esp-idf/v*
            let esp_idf_base = embuild_dir.join("espressif/esp-idf");
            if esp_idf_base.exists() {
                if let Ok(entries) = std::fs::read_dir(&esp_idf_base) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            // Found version directory (e.g., v5.3.3)
                            let idf_path = path;
                            // Add all component include directories
                            // ESP-IDF components can be found via components/{name}/include
                            let components_dir = idf_path.join("components");
                            
                            // Add common component include paths
                            let mut common_paths = vec![
                                components_dir.clone(), // Base components directory
                                idf_path.join("components/esp_timer/include"),
                                idf_path.join("components/freertos/FreeRTOS-Kernel/include"),
                                idf_path.join("components/freertos/FreeRTOS-Kernel/include/freertos"), // For FreeRTOS.h to find FreeRTOSConfig.h
                                idf_path.join("components/freertos/FreeRTOS-Kernel/portable/riscv/include"),
                                idf_path.join("components/freertos/FreeRTOS-Kernel/portable/riscv/include/freertos"), // For portmacro.h
                                // FreeRTOSConfig.h is in components/freertos/config/include/freertos/
                                idf_path.join("components/freertos/config/include/freertos"), // ESP-IDF FreeRTOS config (freertos subdir)
                                idf_path.join("components/freertos/config/include"), // For freertos/FreeRTOSConfig.h includes
                                // FreeRTOSConfig_arch.h for RISC-V (ESP32-C3) - needs parent directory for freertos/ prefix
                                idf_path.join("components/freertos/config/riscv/include"), // ESP-IDF FreeRTOS RISC-V arch config
                                idf_path.join("components/freertos/include"), // ESP-IDF FreeRTOS wrapper includes
                                idf_path.join("components/log/include"),
                                idf_path.join("components/esp_common/include"),
                                idf_path.join("components/esp_etm/include"), // For esp_etm.h
                                idf_path.join("components/esp_hw_support/include"),
                                idf_path.join("components/esp_system/include"),
                                idf_path.join("components/hal/include"),
                                idf_path.join("components/hal/esp32c3/include"),
                                idf_path.join("components/soc/include"), // Base soc includes
                                idf_path.join("components/soc/esp32c3/include"), // ESP32-C3 specific soc includes
                            ];
                            
                            // Also add all component include directories dynamically
                            // Exclude components that are not needed for embedded (linux, unity, etc.)
                            let excluded_components = vec!["linux", "unity", "idf_test"];
                            if components_dir.exists() {
                                if let Ok(entries) = std::fs::read_dir(&components_dir) {
                                    for entry in entries.flatten() {
                                        let comp_path = entry.path();
                                        if comp_path.is_dir() {
                                            let comp_name = comp_path.file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("");
                                            // Skip excluded components
                                            if excluded_components.contains(&comp_name) {
                                                continue;
                                            }
                                            let include_path = comp_path.join("include");
                                            if include_path.exists() && !common_paths.contains(&include_path) {
                                                common_paths.push(include_path);
                                            }
                                        }
                                    }
                                }
                            }
                            for path in common_paths {
                                if path.exists() && !esp_idf_includes.contains(&path) {
                                    esp_idf_includes.push(path);
                                }
                            }
                            break; // Use first found version
                        }
                    }
                }
            }
        }
    }
    
    // Method 5: Find sdkconfig.h from esp-idf-sys build output
    // sdkconfig.h is generated by esp-idf-sys in target/{profile}/build/esp-idf-sys-*/out/build/config/
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let target_base = manifest_dir.join("target");
    
    // Try common profiles (debug, release) and search for esp-idf-sys build directories
    for profile in &["debug", "release"] {
        let build_dir = target_base.join(format!("riscv32imc-esp-espidf/{}/build", profile));
        if build_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&build_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if dir_name.starts_with("esp-idf-sys-") {
                            let sdkconfig_path = path.join("out/build/config/sdkconfig.h");
                            if sdkconfig_path.exists() {
                                if let Some(sdkconfig_dir) = sdkconfig_path.parent() {
                                    let sdkconfig_dir = sdkconfig_dir.to_path_buf();
                                    if !esp_idf_includes.contains(&sdkconfig_dir) {
                                        esp_idf_includes.push(sdkconfig_dir);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    let mut build = cc::Build::new();
    
    build
        .cpp(true)
        .compiler(compiler_path) // Explicitly set the compiler path
        .flag("-std=c++14")
        .flag("-O3")
        .flag("-g3")
        .define("EI_CLASSIFIER_ENABLE_DETECTION_3D", "0")
        .define("EI_CLASSIFIER_TFLITE_ENABLE_CMSIS_NN", "0")
        .define("EI_NATIVE_ARCH", "1")
        // Enable C function pointers for signal_t (required for C FFI)
        .define("EIDSP_SIGNAL_C_FN_POINTER", "1")
        // Enable ESP-IDF porting layer
        .define("EI_PORTING_ESPRESSIF", "1")
        .define("CONFIG_IDF_TARGET_ESP32C3", "1") // ESP32-C3 target
        // Minimal sdkconfig.h defines (ESP-IDF components need these)
        .define("CONFIG_IDF_TARGET", "esp32c3")
        .define("CONFIG_FREERTOS_HZ", "1000")
        .define("CONFIG_ESP32C3_DEFAULT_CPU_FREQ_MHZ", "160")
        .include(&sdk_root)
        .include(sdk_root.join("src"))
        .include(sdk_root.join("src/edge-impulse-sdk"))
        .include(sdk_root.join("src/model-parameters"))
        .include(sdk_root.join("src/tflite-model"));
    
    // Add ESP-IDF include paths
    for include_path in &esp_idf_includes {
        build.include(include_path);
    }
    
    if esp_idf_includes.is_empty() {
        eprintln!("cargo:warning=WARNING: No ESP-IDF include paths found!");
        eprintln!("cargo:warning=  Checked: DEP_ESP_IDF_SYS_INCLUDE, DEP_ESP_IDF_SYS_IDF_PATH, IDF_PATH");
        eprintln!("cargo:warning=  This will cause compilation errors for ei_porting.cpp");
    } else {
        // Debug: print first few paths
        for (i, path) in esp_idf_includes.iter().take(3).enumerate() {
            println!("cargo:warning=ESP-IDF include[{}]: {:?}", i, path);
        }
    }

    // Recursively add source files
    add_source_files(&mut build, &sdk_root.join("src"));
    
    // Add our C++ wrapper and porting layer
    build.file(sdk_root.join("src/ei_wrapper.cpp"));
    build.file(sdk_root.join("src/ei_porting.cpp"));

    build.compile("edge-impulse-sdk");

    println!("cargo:rerun-if-changed=motion-detection_inferencing");
}

fn add_source_files(build: &mut cc::Build, dir: &std::path::Path) {
    for entry in std::fs::read_dir(dir).expect("Failed to read directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        
        if path.is_dir() {
            // Skip Edge Impulse's ESP-IDF porting directory since we have our own
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if dir_name == "porting" {
                // Check if it's the espressif subdirectory
                let espressif_path = path.join("espressif");
                if espressif_path.exists() {
                    // Skip the espressif porting directory - we use our own ei_porting.cpp
                    continue;
                }
            }
            add_source_files(build, &path);
        } else {
            if let Some(ext) = path.extension() {
                if ext == "c" || ext == "cpp" || ext == "cc" {
                    // Skip Edge Impulse's ESP-IDF porting files
                    let path_str = path.to_string_lossy();
                    if path_str.contains("porting/espressif/ei_classifier_porting.cpp") ||
                       path_str.contains("porting/espressif/debug_log.cpp") {
                        continue;
                    }
                    build.file(&path);
                }
            }
        }
    }
}
