fn main() {
    // The build script is executed by Cargo at compile time. It needs to
    // generate the Tauri bindings for the frontend assets.
    tauri_build::build();
}
