#![feature(path_file_prefix)]
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=art/blocks.blend");
    println!("cargo:rerun-if-changed=assets/audio");

    let cargo_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let path = std::path::Path::new(&out_dir).join("consts.rs");
    let mut block_paths = Vec::new();
    let mut cloud_paths = String::new();
    let mut cloud_count = 0;
    let mut prob_sum = 0.0;
    for entry in std::fs::read_dir(std::path::Path::new(&cargo_dir).join("assets/levels")).unwrap()
    {
        let entry = entry.unwrap();
        let path = entry.path();
        let filename = path.file_name().unwrap().to_str().unwrap();
        let prefix = filename.rsplit_once(".").unwrap().0;
        if filename.starts_with("block_") && path.extension() == Some(std::ffi::OsStr::new("glb")) {
            let prob = if let Some((_, n)) = prefix.rsplit_once("--") {
                n.parse::<f32>().unwrap()
            } else {
                1.0
            };
            prob_sum += prob;
            block_paths.push((format!("\"levels/{}\"", filename), prob_sum));
        }
        if filename.contains("cloud") && path.extension() == Some(std::ffi::OsStr::new("glb"))
        {
            cloud_paths.push_str(&format!("\"levels/{}\",", filename));
            cloud_count += 1;
        }
    }

    let mut block_paths_str = String::new();
    let block_count = block_paths.len();
    for (path, prob) in block_paths {
        block_paths_str.push_str(&format!("({path}, {:.3}),", prob / prob_sum));
    }
    let mut code = format!(
        "const BLOCKS: [(&'static str, f32); {}] = [{block_paths_str}];",
        block_count
    );
    code.push_str(&format!(
        "const CLOUDS: [&'static str; {}] = [{cloud_paths}];",
        cloud_count
    ));

    for (name, dir) in [
        ("CLANKS", "audio/clank"),
        ("SQUELCHES", "audio/squelch"),
        ("SPLASHES", "audio/splash"),
    ] {
        let mut paths = String::new();
        let mut count = 0;
        for entry in
            std::fs::read_dir(std::path::Path::new(&cargo_dir).join("assets").join(dir)).unwrap()
        {
            let entry = entry.unwrap();
            let path = entry.path();
            let filename = path.file_name().unwrap().to_str().unwrap();
            if path.extension() == Some(std::ffi::OsStr::new("ogg")) {
                paths.push_str(&format!("\"{}/{}\",", dir, filename));
                count += 1;
            }
        }
        code.push_str(&format!(
            "const {name}: [&'static str; {count}] = [{paths}];"
        ));
    }
    std::fs::write(&path, &code).unwrap();
}
