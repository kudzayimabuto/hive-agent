use axum::{
    extract::{State, Json, Multipart, DefaultBodyLimit},
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use crate::inference::InferenceEngine;
use crate::scheduler::Scheduler;
use crate::message::Message;
use crate::backend::llama_cpp::LlamaCppBackend;
use std::io::Write;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};


#[derive(Clone)]
pub struct AppState {
    pub inference_engine: Arc<Mutex<Option<InferenceEngine>>>,
    pub scheduler: Arc<Mutex<Scheduler>>,
    pub p2p_sender: mpsc::Sender<Message>,
    pub pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<Result<String, String>>>>>,
}

pub async fn start_server(
    inference_engine: Arc<Mutex<Option<InferenceEngine>>>,
    scheduler: Arc<Mutex<Scheduler>>,
    p2p_sender: mpsc::Sender<Message>,
    pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<Result<String, String>>>>>,
) {
    let state = AppState { 
        inference_engine, 
        scheduler, 
        p2p_sender, 
        pending_requests
    };

    // Create models directory if it doesn't exist
    let _ = std::fs::create_dir_all("models");

    let app = Router::new()
        .route("/api/status", get(get_status))
        .route("/api/models", get(list_models))
        .route("/api/peers", get(list_peers))
        .route("/api/inference", post(run_inference))
        .route("/api/upload", post(upload_model))
        .nest_service("/models", tower_http::services::ServeDir::new("models"))
        .layer(DefaultBodyLimit::disable())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Dashboard API listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn get_status(State(state): State<AppState>) -> Json<Value> {
    let peers = state.scheduler.lock().unwrap().peers.len();
    Json(json!({
        "node_id": "local-node",
        "role": "Queen",
        "peers": peers,
        "status": "active"
    }))
}

async fn list_peers(State(state): State<AppState>) -> Json<Value> {
    let scheduler = state.scheduler.lock().unwrap();
    let peers: Vec<Value> = scheduler.peers.values().map(|p| {
        json!({
            "id": p.id.to_string(),
            "address": p.address.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", "), // Join multiple addrs
            "role": "Drone",
            "latency": 5, // Mock latency
            "status": p.status
        })
    }).collect();

    
    // Add local system stats (placeholder for now to fix build)
    let cpu_usage = 0.0;
    let total_mem = 0;
    let used_mem = 0;
    
    Json(json!({ 
        "peers": peers,
        "metrics": {
            "cpu_usage": cpu_usage,
            "total_mem": total_mem,
            "used_mem": used_mem,
            "gpu_usage": 0 // Placeholder
        }
    }))
}

async fn list_models() -> Json<Value> {
    let mut models = Vec::new();
    if let Ok(entries) = std::fs::read_dir("models") {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Ok(file_name) = entry.file_name().into_string() {
                    if file_name.ends_with(".gguf") || file_name.ends_with(".bin") {
                        models.push(file_name);
                    }
                }
            }
        }
    }
    Json(json!({ "models": models }))
}

async fn upload_model(mut multipart: Multipart) -> Json<Value> {
    let mut repo_id: Option<String> = None;
    let mut file_name: Option<String> = None;
    
    // We need to process fields. Note: We expect 'repo_id' before 'model' or handle order carefully.
    // Axum multipart streams fields, so we might need to buffer.
    // However, for simplicity, let's just handle them as they come. 
    // If 'model' comes first, we save it. If 'repo_id' comes later, we trigger download.
    // But 'repo_id' is needed to name the tokenizer correctly if we key it off model name.
    
    // Better strategy: Process all fields. Save model file. If repo_id was seen (or is seen), use it.
    
    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(name) = field.name() {
            match name {
                "repo_id" => {
                    if let Ok(val) = field.text().await {
                        if !val.is_empty() {
                            println!("Received HF Repo ID: {}", val);
                            repo_id = Some(val);
                        }
                    }
                }
                "model" => {
                    if let Some(fname) = field.file_name() {
                        let fname = fname.to_string();
                        let path = format!("models/{}", fname);
                        file_name = Some(fname.clone());
                        
                        // Read bytes
                         let data = match field.bytes().await {
                            Ok(bytes) => bytes,
                            Err(e) => return Json(json!({ "error": format!("Failed to read bytes: {}", e) })),
                        };

                        if let Ok(mut file) = std::fs::File::create(&path) {
                            if file.write_all(&data).is_err() {
                                 return Json(json!({ "error": "Failed to write model file" }));
                            }
                            println!("Uploaded model: {}", path);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // After processing fields, if we have both model name and repo_id, fetch tokenizer
    if let (Some(model_name), Some(repo)) = (file_name.clone(), repo_id) {
        println!("Attempting to auto-download tokenizer from {}", repo);
        tokio::spawn(async move {
            let url = format!("https://huggingface.co/{}/resolve/main/tokenizer.json", repo);
            let target_path = format!("models/{}.tokenizer.json", model_name);
            
            match reqwest::get(&url).await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        if let Ok(bytes) = resp.bytes().await {
                            if let Ok(mut file) = std::fs::File::create(&target_path) {
                                let _ = file.write_all(&bytes);
                                println!("Downloaded tokenizer to {}", target_path);
                            }
                        }
                    } else {
                        println!("Failed to find tokenizer.json at {}", url);
                        // Try tokenizer_config.json?
                    }
                }
                Err(e) => println!("Error downloading tokenizer: {}", e),
            }
        });
    }

    if let Some(fname) = file_name {
        Json(json!({ "status": "uploaded", "filename": fname }))
    } else {
        Json(json!({ "error": "No model file found in request" }))
    }
}

#[derive(serde::Deserialize)]
struct InferenceRequest {
    model_path: Option<String>,
    tokenizer_path: Option<String>,
    prompt: String,
}

async fn run_inference(
    State(state): State<AppState>,
    Json(payload): Json<InferenceRequest>,
) -> Json<Value> {
    let model_path_raw = payload.model_path.unwrap_or_else(|| "models/tinyllama-1.1b-chat-v1.0.Q4_K_S.gguf".to_string());
    let model_path = if std::path::Path::new(&model_path_raw).exists() {
        model_path_raw.clone()
    } else {
        format!("models/{}", model_path_raw)
    };
    // Check for specific tokenizer
    let specific_tokenizer = format!("{}.tokenizer.json", model_path);
    let tokenizer_path = if std::path::Path::new(&specific_tokenizer).exists() {
        specific_tokenizer
    } else {
        payload.tokenizer_path.unwrap_or_else(|| "tokenizer.json".to_string())
    };
    
    let prompt_raw = payload.prompt;
    
    // Simple Llama 2 Chat Template
    let prompt = prompt_raw;

    println!("Received inference request: {}", prompt);
    println!("Using tokenizer: {}", tokenizer_path);



    // Dynamic Discovery: Check for peers in the swarm
    let peers = {
        let scheduler = state.scheduler.lock().unwrap();
        scheduler.peers.clone()
    };
    
    // Find the first peer with a valid IP4 address
    let mut worker_rpc_url = None;
    
    for (peer_id, info) in peers {
        for addr in info.address {
            // Extract IP from Multiaddr (e.g., /ip4/192.168.1.10/tcp/1234)
            // We need to parse it string-wise or use Multiaddr methods
            let addr_str = addr.to_string();
            if addr_str.contains("/ip4/") && !addr_str.contains("127.0.0.1") {
                // Parse out the IP. Hacky string parsing for now.
                // Format is usually /ip4/<ip>/tcp/<port>
                let parts: Vec<&str> = addr_str.split('/').collect();
                if parts.len() >= 3 && parts[1] == "ip4" {
                    let ip = parts[2];
                    // Assume default worker port 50052
                    worker_rpc_url = Some(format!("{}:50052", ip));
                    println!("Discovered Peer {} at {}. Using RPC: {}", peer_id, ip, worker_rpc_url.as_ref().unwrap());
                    break;
                }
            }
        }
        if worker_rpc_url.is_some() {
            break;
        }
    }

    if let Some(rpc_url) = worker_rpc_url {
        println!("Offloading inference to Worker: {}", rpc_url);
        
        let result = tokio::task::spawn_blocking({
            let prompt = prompt.clone();
            let model = model_path.clone(); // Use the requested model path
            let rpc = rpc_url;
            let ngl = 99; // Default to full offload for discovered peers
            move || {
                LlamaCppBackend::generate_oneshot(&model, &prompt, &rpc, ngl)
            }
        }).await;

         match result {
             Ok(Ok(output)) => return Json(json!({ "result": output })),
             Ok(Err(e)) => return Json(json!({ "error": e })),
             Err(_) => return Json(json!({ "error": "Internal server error" })),
        }
    }

    // Fallback to Local Inference if no peers found
    println!("No suitable peers found. Running locally.");
    
    let peer_count = state.scheduler.lock().unwrap().peers.len(); // Re-check for other logic if needed, but we already tried.
    
    if false { // Disable the old "Broadcasting task" block since we handled it above via RPC

        // Distributed Inference
        println!("Broadcasting task to {} peers...", peer_count);
        let task_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        
        {
            state.pending_requests.lock().unwrap().insert(task_id.clone(), tx);
        }
        
        let my_local_ip = local_ip_address::local_ip().map(|ip| ip.to_string()).unwrap_or("127.0.0.1".to_string());
        let model_filename = std::path::Path::new(&model_path).file_name().unwrap_or_default().to_string_lossy().to_string();
        let download_url = format!("http://{}:3000/models/{}", my_local_ip, model_filename);

        let msg = Message::TaskRequest {
            task_id: task_id.clone(),
            prompt: prompt,
            model_name: model_filename, 
            download_url: Some(download_url),
            layer_range: None, // Default to full load for now (Replication)
        };
        
        if let Err(e) = state.p2p_sender.send(msg).await {
            return Json(json!({ "error": format!("Failed to send to P2P loop: {}", e) }));
        }
        
        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_secs(1200), rx).await {
            Ok(Ok(Ok(result))) => Json(json!({ "result": result })),
            Ok(Ok(Err(e))) => Json(json!({ "error": format!("Remote Error: {}", e) })),
            Ok(Err(_)) => Json(json!({ "error": "Internal channel closed" })),
            Err(_) => {
                // Remove from pending on timeout
                state.pending_requests.lock().unwrap().remove(&task_id);
                println!("Task {} timed out after 1200s", task_id);
                Json(json!({ "error": "Distributed inference timed out (1200s limit exceeded)" }))
            }
        }

    } else {
        // Local Inference (Fallback)
        println!("No peers found. Running locally.");
        
        let inference_result = tokio::time::timeout(std::time::Duration::from_secs(300), tokio::task::spawn_blocking(move || {
            let mut engine_lock = state.inference_engine.lock().unwrap();
            
            let should_reload = if let Some(engine) = engine_lock.as_ref() {
                engine.model_path != model_path
            } else {
                true
            };

            if should_reload {
                println!("Loading model: {}", model_path);
                match InferenceEngine::load(&model_path, &tokenizer_path, None) {
                    Ok(new_engine) => {
                        *engine_lock = Some(new_engine);
                    },
                    Err(e) => {
                        return Err(format!("Failed to load model: {}", e));
                    }
                }
            } else {
                 println!("Using cached model: {}", model_path);
            }

            if let Some(engine) = engine_lock.as_mut() {
                match engine.generate(&prompt, 20) { // Reduced to 20 tokens for speed
                    Ok(result) => Ok(result),
                    Err(e) => Err(format!("Inference failed: {}", e)),
                }
            } else {
                Err("Engine not initialized".to_string())
            }
        })).await;

        match inference_result {
            Ok(Ok(Ok(result))) => Json(json!({ "result": result })),
            Ok(Ok(Err(e))) => Json(json!({ "error": e })),
            Ok(Err(_join_err)) => Json(json!({ "error": "Internal server error (task panic)" })),
            Err(_elapsed) => Json(json!({ "error": "Inference timed out (engine too slow or stuck)" })),
        }
    }
}
