use std::io::{self, BufRead};
fn main() {
    let mut model = packaging_proof_model::Model::initial();
    println!("{}", serde_json::to_string(&model).unwrap());
    for line in io::stdin().lock().lines() {
        model.apply(&line.unwrap());
        println!("{}", serde_json::to_string(&model).unwrap());
    }
}
