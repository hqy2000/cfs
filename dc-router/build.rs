fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute("data_capsule.DataCapsuleBlock", "#[derive(Hash)]")
        .compile(&["../proto/data_capsule.proto", "../proto/router.proto"], &["../proto"])?;
    Ok(())
}