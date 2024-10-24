fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=art/blocks.blend");
    println!("cargo:rerun-if-changed=assets/audio");

    let cargo_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let path = std::path::Path::new(&out_dir).join("consts.rs");
    let mut block_paths = String::new();
    let mut block_count = 0;
    let mut decayed_paths = String::new();
    let mut decayed_count = 0;
    for entry in std::fs::read_dir(std::path::Path::new(&cargo_dir).join("assets/levels")).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let filename = path.file_name().unwrap().to_str().unwrap();
        if filename.starts_with("block_") && path.extension() == Some(std::ffi::OsStr::new("glb")) {
            block_paths.push_str(&format!("\"levels/{}\",", filename));
            block_count += 1;
        }
        if filename.starts_with("decayed_") && path.extension() == Some(std::ffi::OsStr::new("glb")) {
            decayed_paths.push_str(&format!("\"levels/{}\",", filename));
            decayed_count += 1;
        }
    }
    let mut code = format!("const BLOCKS: [&'static str; {}] = [{block_paths}];", block_count);
    code.push_str(&format!("const DECAYED: [&'static str; {}] = [{decayed_paths}];", decayed_count));

    for (name, dir) in [("CLANKS", "audio/clank"), ("SQUELCHES", "audio/squelch"), ("SPLASHES", "audio/splash")] {
        let mut paths = String::new();
        let mut count = 0;
        for entry in std::fs::read_dir(std::path::Path::new(&cargo_dir).join("assets").join(dir)).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let filename = path.file_name().unwrap().to_str().unwrap();
            if path.extension() == Some(std::ffi::OsStr::new("ogg")) {
                paths.push_str(&format!("\"{}/{}\",", dir, filename));
                count += 1;
            }
        }
        code.push_str(&format!("const {name}: [&'static str; {count}] = [{paths}];"));
    }
    std::fs::write(&path, &code).unwrap();
}
