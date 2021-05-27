fn main() {
    spirv_builder::SpirvBuilder::new("../shader", "spirv-unknown-vulkan1.2")
        .capability(spirv_builder::Capability::Int8)
        .build()
        .unwrap();
}
