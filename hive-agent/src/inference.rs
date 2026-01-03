use anyhow::{Error, Result};
use crate::model::sharded_llama as model;
use candle_core::{Tensor, Device};
use candle_transformers::generation::LogitsProcessor;
use model::ModelWeights;
use tokenizers::Tokenizer;

pub struct InferenceEngine {
    model: ModelWeights,
    tokenizer: Tokenizer,
    device: Device,
    pub model_path: String,
}

impl InferenceEngine {
    pub fn load(model_path: &str, tokenizer_path: &str, layer_range: Option<(usize, usize)>) -> Result<Self> {
        println!("Loading model from {}", model_path);
        let device = {
            #[cfg(feature = "cuda")]
            {
                println!("Attempting to use CUDA Backend...");
                match Device::new_cuda(0) {
                    Ok(d) => {
                        println!("🚀 Using CUDA Backend (NVIDIA)");
                        d
                    },
                    Err(e) => {
                        println!("⚠️ CUDA failed: {}. Falling back to CPU.", e);
                        Device::Cpu
                    }
                }
            }
            #[cfg(not(feature = "cuda"))]
            {
                println!("💻 Using CPU Backend");
                Device::Cpu
            }
        };
        
        let mut file = std::fs::File::open(model_path)?;
        println!("File opened");
        let content = candle_core::quantized::gguf_file::Content::read(&mut file)?;
        println!("Content read");
        let model = ModelWeights::from_gguf(content, &mut file, &device, layer_range)?;
        println!("Model loaded (Range: {:?})", layer_range);
        
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(Error::msg)?;
        println!("Tokenizer loaded");
        
        Ok(Self {
            model,
            tokenizer,
            device,
            model_path: model_path.to_string(),
        })
    }

    pub fn generate(&mut self, prompt: &str, sample_len: usize) -> Result<String> {
        println!("Encoding prompt...");
        let mut tokens = self.tokenizer
            .encode(prompt, true)
            .map_err(Error::msg)?
            .get_ids()
            .to_vec();
        println!("Prompt encoded. Tokens: {}", tokens.len());
            
        let mut logits_processor = LogitsProcessor::new(299792458, Some(0.8), Some(0.95));
        let mut new_tokens = vec![];

        println!("Starting generation loop...");
        for index in 0..sample_len {
            let context_size = if index > 0 { 1 } else { tokens.len() };
            let start_pos = tokens.len().saturating_sub(context_size);
            let input = Tensor::new(&tokens[start_pos..], &self.device)?.unsqueeze(0)?;
            
            let logits = self.model.forward(&input, start_pos)?;
            let logits = logits.squeeze(0)?.squeeze(0)?.to_dtype(candle_core::DType::F32)?;
            
            let next_token = logits_processor.sample(&logits)?;
            tokens.push(next_token);
            new_tokens.push(next_token);
            
            // Log progress
            use std::io::Write;
            print!(".");
            std::io::stdout().flush().ok();

            if let Some(t) = self.tokenizer.id_to_token(next_token) {
                if t == "</s>" {
                    break;
                }
            }
        }
        println!(); // Newline after generation
        
        let output = self.tokenizer.decode(&new_tokens, true).map_err(Error::msg)?;
        Ok(output)
    }
}
