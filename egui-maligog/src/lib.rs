const SHADER: &[u8] = include_bytes!(env!("shader.spv"));

struct UiPass {}

impl UiPass {
    pub fn new() -> Self {
        Self {}
    }
}
