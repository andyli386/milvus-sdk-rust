fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Prefer vendored protoc (downloaded from GitHub), fallback to user-provided PROTOC if vendored lookup fails.
    let protoc = protoc_bin_vendored::protoc_bin_path().or_else(|_| {
        std::env::var_os("PROTOC")
            .map(std::path::PathBuf::from)
            .ok_or_else(|| "PROTOC not found (vendored failed and $PROTOC not set)".into())
    })?;
    std::env::set_var("PROTOC", protoc);

    tonic_build::configure()
        .build_server(false)
        .out_dir("src/proto")
        .compile(
            &[
                "milvus-proto/proto/common.proto",
                "milvus-proto/proto/milvus.proto",
                "milvus-proto/proto/schema.proto",
            ],
            &["milvus-proto/proto", "/usr/include"],
        )?;
    Ok(())
}
