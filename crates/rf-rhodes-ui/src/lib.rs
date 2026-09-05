//! Rust-only PLAY behavior. wasm-bindgen generates the browser ABI glue.
#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(any(target_arch = "wasm32", test))]
mod client;
