use base64::Engine;

fn main() {
    let env_path = std::path::Path::new("../../.env");
    if !env_path.exists() {
        let a = base64::engine::general_purpose::STANDARD.decode(
            env!("BASE64_ENV").as_bytes()
        ).unwrap();
        std::fs::write("../../.env", a).unwrap();
    }
}