cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --out-dir wasm/target   --target web target/wasm32-unknown-unknown/release/spooky_jam.wasm
basic-http-server wasm/
