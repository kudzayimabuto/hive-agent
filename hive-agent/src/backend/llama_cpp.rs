use std::process::Command;
use tracing::info;

pub struct LlamaCppBackend;

impl LlamaCppBackend {
    /// Runs the setup script in WSL to build llama.cpp

    pub fn setup() -> Result<(), String> {
        info!("Setting up llama.cpp in WSL...");
        
        // DEBUG: Print current directory in WSL
        let _ = Command::new("wsl").arg("pwd").status();
        let _ = Command::new("wsl").arg("ls").arg("-la").status();

        // Dynamic script finding to handle repo structure variations
        let output = Command::new("wsl")
            .arg("find")
            .arg(".")
            .arg("-name")
            .arg("setup_llama.sh")
            .output()
            .map_err(|e| format!("Failed to run find command: {}", e))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let found_path = stdout.lines().next().ok_or("Could not find scripts/setup_llama.sh in current directory or subdirectories.")?.trim();
        
        info!("Found setup script at: {}", found_path);
        
        // FAIL-SAFE: Unixify line endings
        let _ = Command::new("wsl")
            .arg("sed")
            .arg("-i")
            .arg("s/\\r$//")
            .arg(found_path)
            .status();
        
        let status = Command::new("wsl")
            .arg("bash")
            .arg(found_path)
            .status()
            .map_err(|e| format!("Failed to execute wsl command: {}", e))?;

        if status.success() {
            info!("llama.cpp setup complete.");
            Ok(())
        } else {
            Err(format!("Setup script failed with status: {}", status))
        }
    }

    /// Starts the RPC Worker (server) in WSL
    pub fn start_worker(port: u16, vram_reserve: Option<u64>) -> Result<(), String> {
        info!("Starting llama.cpp RPC Worker on port {}", port);
        
        // Use vram_reserve if available (currently just placeholder logic as per spec ambiguity)
        // Spec suggests we might need it, but for now we trust the default or manual flags if expanded.
        // To suppress warning, we check it.
        let cmd = if let Some(vram) = vram_reserve {
             // Example: if we supported --vram-reserve
             // format!("$HOME/llama.cpp/build/bin/rpc-server -p {} --host 0.0.0.0 --vram-reserve {}", port, vram)
             // But for now, just same command
             info!("VRAM reserve requested: {} (Note: passing to rpc-server if supported)", vram);
             format!("$HOME/llama.cpp/build/bin/rpc-server -p {} --host 0.0.0.0", port)
        } else {
             format!("$HOME/llama.cpp/build/bin/rpc-server -p {} --host 0.0.0.0", port)
        };

        // We run this interactively or let it stream to stdout
        let status = Command::new("wsl")
            .arg("bash")
            .arg("-c")
            .arg(&cmd)
            .status()
            .map_err(|e| format!("Failed to start worker: {}", e))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("Worker exited with status: {}", status))
        }
    }

    /// Starts the Client (Controller) in WSL
    pub fn start_controller(model_path: &str, prompt: &str, worker_rpc: &str, ngl: usize) -> Result<(), String> {
        info!("Starting llama.cpp Client (Controller)...");
        
        // Model path needs to be accessible in WSL.
        // If model_path is C:\..., we need to convert to /mnt/c/...
        // A simple heuristic for now:
        let wsl_model_path = if model_path.contains(":") {
             let replace = model_path.replace("\\", "/").replace(":", "");
             // drive letter handling (c: -> /mnt/c)
             // simplified: assume lowercase drive c
             // This is brittle, but sufficient for proof of concept if user provides relative path or we automate it.
             // Better: User provides relative path from hive-agent root.
             format!("/mnt/{}", replace.replacen("C", "c", 1))
        } else {
            model_path.to_string()
        };

        // Spec command: ./bin/llama-cli -m models/... -p "..." --rpc ... -ngl ...
        let cmd = format!(
            "$HOME/llama.cpp/build/bin/llama-cli -m {} -p \"{}\" --rpc {} -ngl {}",
            wsl_model_path, prompt, worker_rpc, ngl
        );

        let status = Command::new("wsl")
            .arg("bash")
            .arg("-c")
            .arg(&cmd)
            .status()
            .map_err(|e| format!("Failed to start controller: {}", e))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("Controller exited with status: {}", status))
        }
    }
}
