fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute(
            "VoteRequest",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "RegisterRequest",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .compile(&["proto/master.proto"], &["proto"])?;
    Ok(())
}
