use std::fs;
use std::path::PathBuf;

fn main() {
    // Dynamically target standard Windows CUDA implementations
    let base_cuda_path = PathBuf::from("C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA");
    
    if base_cuda_path.exists() {
        if let Ok(entries) = fs::read_dir(&base_cuda_path) {
            let mut versions: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.is_dir() && p.file_name().unwrap_or_default().to_string_lossy().starts_with("v12.")
                })
                .collect();
                
            // Sort to ensure highest version (v12.x) is strictly prioritized for RTX architecture
            versions.sort();
            
            if let Some(latest) = versions.last() {
                let lib_path = latest.join("lib").join("x64");
                if lib_path.exists() {
                    // Inject natively into the cargo compiler bounds evading pure CPU fallbacks!
                    println!("cargo:rustc-link-search=native={}", lib_path.display());
                    println!("cargo:rustc-link-lib=dylib=cudart");
                    
                    #[cfg(target_os = "windows")]
                    println!("cargo:rustc-link-lib=dylib=cublas");
                }
            }
        }
    }
    
    // Auto-Rerun on build constraint updates
    println!("cargo:rerun-if-changed=build.rs");
}
