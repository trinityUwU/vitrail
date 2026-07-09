fn main() {
    tauri_build::build();

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/ui.proto"], &["proto"])
        .expect("échec de compilation de proto/ui.proto (tonic-build)");
}
