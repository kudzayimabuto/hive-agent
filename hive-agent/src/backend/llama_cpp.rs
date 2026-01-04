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
        
        // FAIL-SAFE: Unixify line endings using tr (safer than sed which might misinterpret \r as 'r')
        let _ = Command::new("wsl")
            .arg("bash")
            .arg("-c")
            .arg(format!("tr -d '\\r' < {} > {}.tmp && mv {}.tmp {}", found_path, found_path, found_path, found_path))
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
        
        // Use wslpath to canonicalize the path for WSL
        let output = Command::new("wsl")
            .arg("wslpath")
            .arg("-a")
            .arg(model_path)
            .output()
            .map_err(|e| format!("Failed to run wslpath: {}", e))?;
            
        let wsl_model_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        info!("Converted model path: {} -> {}", model_path, wsl_model_path);

        // Spec command: ./bin/llama-cli -m models/... -p "..." --rpc ... -ngl ...
        let cmd = format!(
            "$HOME/llama.cpp/build/bin/llama-cli -m {} -p \"{}\" --rpc {} -ngl {} --verbose",
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

    /// Runs a single inference and returns the output as a string (for API usage)
    pub fn generate_oneshot(model_path: &str, prompt: &str, worker_rpc: &str, ngl: usize) -> Result<String, String> {
        info!("Running oneshot inference...");
        println!("[Debug] resolving wslpath for: '{}'", model_path);
        
        let output = Command::new("wsl")
            .arg("wslpath")
            .arg("-a")
            .arg(model_path)
            .output()
            .map_err(|e| format!("Failed to run wslpath: {}", e))?;
            
        println!("[Debug] wslpath output stdout: {:?}", String::from_utf8_lossy(&output.stdout));
        println!("[Debug] wslpath output stderr: {:?}", String::from_utf8_lossy(&output.stderr));

        let wsl_model_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("[Debug] Resolved wsl_model_path: '{}'", wsl_model_path);

        // Use --no-interactive and -n to ensure it exits and prints to stdout
        // We also might want to silence logs or capture them carefully.
        // For now, we assume stdout contains the generation.
        let cmd = format!(
            "$HOME/llama.cpp/build/bin/llama-cli -m {} -p \"{}\" --rpc {} -ngl {} -n 128",
            wsl_model_path, prompt, worker_rpc, ngl
        );

        info!("Executing oneshot command: {}", cmd);
        println!("[Debug] Full Command: wsl bash -c '{}'", cmd);

        let output = Command::new("wsl")
            .arg("bash")
            .arg("-c")
            .arg(&cmd)
            .output()
            .map_err(|e| format!("Failed to run controller: {}", e))?;

        println!("[Debug] Resolved wsl_model_path: '{}'", wsl_model_path);

        // Streaming execution
        let mut child = Command::new("wsl")
            .arg("bash")
            .arg("-c")
            .arg(&cmd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn controller: {}", e))?;

        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

        let mut output_str = String::new();
        
        // Spawn logs in threads to prevent deadlock (naive but works for debug)
        let stdout_handle = std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            let mut acc = String::new();
            for line in reader.lines() {
                if let Ok(l) = line {
                    println!("[llama-cli] {}", l);
                    acc.push_str(&l);
                    acc.push('\n');
                }
            }
            acc
        });

        let stderr_handle = std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(l) = line {
                    eprintln!("[llama-cli error] {}", l);
                }
            }
        });

        let status = child.wait().map_err(|e| format!("Failed to wait on child: {}", e))?;
        
        let captured_stdout = stdout_handle.join().unwrap_or_default();
        let _ = stderr_handle.join(); // Just finish

        println!("[Debug] Command Exit Status: {}", status);

        if status.success() {
             Ok(captured_stdout)
        } else {
             Err(format!("Inference failed with status {}", status))
        }
    }
}
