use std::{env, fs, path::PathBuf};
fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let lib = root.join(".cache/native/build/lib");
    println!("cargo:rerun-if-changed={}", lib.display());
    if !lib.join("jingyi_native.lib").is_file() {
        panic!("Run scripts/build-native.ps1 before building Jingyi.");
    }
    println!("cargo:rustc-link-search=native={}", lib.display());
    let mut libraries: Vec<_> = fs::read_dir(lib)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "lib"))
        .collect();
    libraries.sort();
    for path in libraries {
        println!(
            "cargo:rustc-link-lib=static={}",
            path.file_stem().unwrap().to_str().unwrap()
        );
    }
    println!("cargo:rustc-link-lib=advapi32");
    println!("cargo:rustc-link-lib=dxcore");
}
