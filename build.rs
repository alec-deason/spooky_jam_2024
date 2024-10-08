fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=art/blocks.blend");

    let cargo_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let path = std::path::Path::new(&out_dir).join("consts.rs");
    let mut block_paths = String::new();
    let mut block_count = 0;
    let mut disaster_paths = String::new();
    let mut disaster_count = 0;
    for entry in std::fs::read_dir(std::path::Path::new(&cargo_dir).join("assets/levels")).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let filename = path.file_name().unwrap().to_str().unwrap();
        if filename.starts_with("block_") && path.extension() == Some(std::ffi::OsStr::new("glb")) {
            block_paths.push_str(&format!("\"levels/{}\",", filename));
            block_count += 1;
        }
        if filename.starts_with("disaster_") && path.extension() == Some(std::ffi::OsStr::new("glb")) {
            disaster_paths.push_str(&format!("\"levels/{}\",", filename));
            disaster_count += 1;
        }
    }
    let mut code = format!("const BLOCKS: [&'static str; {}] = [{block_paths}];", block_count);
    code.push_str(&format!("const DISASTERS: [&'static str; {}] = [{disaster_paths}];", disaster_count));
    std::fs::write(&path, &code).unwrap();
}
