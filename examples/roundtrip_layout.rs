//! Normalize an exported legacy layout using the production Rust validator.
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let text = std::fs::read_to_string(&args[1]).expect("input JSON");
    let layout = simdisplay::model::Layout::parse(&text).expect("valid layout");
    std::fs::write(&args[2], serde_json::to_vec_pretty(&layout).unwrap()).expect("output JSON");
}
