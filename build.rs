fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .type_attribute("data_capsule.DataCapsuleBlock", "#[derive(Hash)]")
        .compile(&["proto/data_capsule.proto"], &["proto"])?;
    Ok(())
}