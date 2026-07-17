fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().commands(&[
                "desktop_protocol",
                "desktop_invoke",
                "desktop_emit_tick",
            ]),
        ),
    )
    .expect("failed to run tauri build");
}
