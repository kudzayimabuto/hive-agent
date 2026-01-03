use candle_transformers::models::quantized_llama::ModelWeights;

fn main() {
    println!("Checking ModelWeights structure...");
    // This won't run, but the compiler error will tell me if 'layers' is private.
    // let m: ModelWeights = ...;
    // let _ = m.layers; 
}
