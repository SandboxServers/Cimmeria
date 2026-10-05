#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use packaging_proof_model::Model;
use std::sync::Mutex;
#[tauri::command]
fn action(name: String, state: tauri::State<'_, Mutex<Model>>) -> serde_json::Value {
    let mut model = state.lock().unwrap();
    if name != "snapshot" {
        model.apply(&name);
    }
    serde_json::to_value(&*model).unwrap()
}
fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(Model::initial()))
        .invoke_handler(tauri::generate_handler![action])
        .run(tauri::generate_context!())
        .expect("prototype application failed");
}
