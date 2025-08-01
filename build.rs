const COMMANDS: &[&str] = &["share_text", "share_data", "share_file"];

fn main() {
  tauri_plugin::Builder::new(COMMANDS)
    .global_api_script_path("./api-iife.js")
    .android_path("android")
    .ios_path("ios")
    .build();
}
