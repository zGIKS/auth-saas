fn main() {
    println!("cargo:rerun-if-changed=proto/authentication_verification.proto");

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(&["proto/authentication_verification.proto"], &["proto"])
        .expect("failed to compile authentication verification proto");
}
