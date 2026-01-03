use anyhow::Result;
use ndarray::Array2;
use rand::Rng;

pub struct ComputeEngine;

impl ComputeEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_matrix(rows: usize, cols: usize) -> Array2<f32> {
        let mut rng = rand::thread_rng();
        Array2::from_shape_fn((rows, cols), |_| rng.gen::<f32>())
    }

    pub fn multiply(a: &Array2<f32>, b: &Array2<f32>) -> Result<Array2<f32>> {
        Ok(a.dot(b))
    }
    
    // Helper to serialize matrix to bytes (for storage)
    pub fn serialize_matrix(matrix: &Array2<f32>) -> Result<Vec<u8>> {
        let shape = matrix.shape();
        let data = matrix.as_slice().unwrap(); // Standard layout
        
        // Simple binary format: [rows: u64][cols: u64][data: f32...]
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(shape[0] as u64).to_le_bytes());
        bytes.extend_from_slice(&(shape[1] as u64).to_le_bytes());
        
        for &val in data {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        
        Ok(bytes)
    }

    pub fn deserialize_matrix(bytes: &[u8]) -> Result<Array2<f32>> {
        let rows = u64::from_le_bytes(bytes[0..8].try_into()?) as usize;
        let cols = u64::from_le_bytes(bytes[8..16].try_into()?) as usize;
        
        let mut data = Vec::with_capacity(rows * cols);
        let mut offset = 16;
        
        while offset < bytes.len() {
            let val = f32::from_le_bytes(bytes[offset..offset+4].try_into()?);
            data.push(val);
            offset += 4;
        }
        
        Ok(Array2::from_shape_vec((rows, cols), data)?)
    }
}
