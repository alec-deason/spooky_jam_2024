fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=art/blocks.blend");

    let cargo_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let path = std::path::Path::new(&out_dir).join("consts.rs");
    let mut paths = String::new();
    let mut count = 0;
    for entry in std::fs::read_dir(std::path::Path::new(&cargo_dir).join("assets/levels")).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let filename = path.file_name().unwrap().to_str().unwrap();
        if !filename.starts_with("_") && path.extension() == Some(std::ffi::OsStr::new("glb")) {
            paths.push_str(&format!("\"levels/{}\",", filename));
            count += 1;
        }
    }
    let code = format!("const BLOCKS: [&'static str; {}] = [{paths}];", count);
    std::fs::write(&path, &code).unwrap();
}
