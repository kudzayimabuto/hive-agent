use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapability {
    pub device_type: String, // "mobile", "gpu_server"
    pub available_vram: u64,
    pub flops_score: f32,
    pub can_run_docker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPayload {
    pub task_id: String,
    pub model_shard_cid: String,
    // Add more fields as needed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub data: Vec<u8>,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputeTask {
    MatrixMul {
        matrix_a_cid: String,
        matrix_b_cid: String,
        result_cid: String, // Where to store the result
    },
}
