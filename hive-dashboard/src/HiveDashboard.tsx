import { useState, useEffect, useRef } from 'react';
import {
    Activity,
    Server,
    Database,
    Cpu,
    Share2,
    Terminal,
    UploadCloud,
    MessageSquare,
    Send,
    Box,
    Zap,
    FileCode,
    CheckCircle2,
    Lock,
    Network
} from 'lucide-react';

// --- Types ---
interface Peer {
    id: string;
    address: string;
    role: 'Queen' | 'Drone';
    latency: number;
    status: 'active' | 'syncing' | 'computing';
}

interface Metrics {
    cpu_usage: number;
    total_mem: number;
    used_mem: number;
    gpu_usage: number;
}

interface ChatMessage {
    id: string;
    role: 'user' | 'assistant' | 'system';
    content: string;
    timestamp: number;
}

const HiveDashboard = () => {
    // --- State ---
    const [nodeId] = useState<string>('12D3Koo...8jL2');
    const [peers, setPeers] = useState<Peer[]>([]); // Start empty, wait for sync

    // Real state for selected model
    const [selectedModel, setSelectedModel] = useState<string | null>(null);
    const [availableModels, setAvailableModels] = useState<string[]>([]);

    // Derived state for the UI's "activeModel" object
    const activeModel = selectedModel ? {
        name: selectedModel,
        cid: "Qm" + btoa(selectedModel).substring(0, 16), // Mock CID based on filename
        size: "Unknown"
    } : null;

    const [uploadState, setUploadState] = useState<'idle' | 'uploading' | 'hashing' | 'complete'>('idle');
    const [uploadProgress, setUploadProgress] = useState(0);
    const [repoId, setRepoId] = useState('');

    const [promptInput, setPromptInput] = useState('');
    const [isInferencing, setIsInferencing] = useState(false);
    const [inferenceStats, setInferenceStats] = useState({ tps: 0, progress: 0 });

    const [chatHistory, setChatHistory] = useState<ChatMessage[]>([
        { id: '0', role: 'system', content: 'Hive Swarm Ready. Load a model to begin.', timestamp: Date.now() }
    ]);

    const [logs, setLogs] = useState<string[]>([
        '[System] hive-agent v0.5.0 initialized',
        '[Network] mDNS service started',
    ]);

    const [casStats] = useState({
        totalFiles: 12,
        storageUsed: '1.2 GB',
        cacheHits: 45
    });

    const chatEndRef = useRef<HTMLDivElement>(null);
    const fileInputRef = useRef<HTMLInputElement>(null);

    const addLog = (msg: string) => {
        setLogs(prev => [`[${new Date().toLocaleTimeString()}] ${msg}`, ...prev].slice(0, 50));
    };

    // Auto-scroll chat
    useEffect(() => {
        chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [chatHistory]);

    // Fetch models on mount
    useEffect(() => {
        const fetchModels = async () => {
            try {
                const res = await fetch('http://localhost:3000/api/models');
                const data = await res.json();
                if (data.models) {
                    setAvailableModels(data.models);
                }
            } catch (e) {
                console.error("Failed to fetch models", e);
                addLog(`[Error] Failed to fetch models: ${e}`);
            }
        };
        fetchModels();
    }, []);

    const [metrics, setMetrics] = useState<Metrics | null>(null);

    // Poll for Peers & Metrics
    useEffect(() => {
        const fetchPeers = async () => {
            try {
                const res = await fetch('http://localhost:3000/api/peers');
                const data = await res.json();
                if (data.peers) {
                    setPeers(data.peers);
                }
                if (data.metrics) {
                    setMetrics(data.metrics);
                }
            } catch (e) {
                // Silent fail
            }
        };

        fetchPeers(); // Initial call
        const interval = setInterval(fetchPeers, 2000); // Poll every 2s
        return () => clearInterval(interval);
    }, []);


    // --- Actions ---

    const handleFileSelect = async (event: React.ChangeEvent<HTMLInputElement>) => {
        const file = event.target.files?.[0];
        if (!file) return;

        setUploadState('uploading');
        setUploadProgress(10);
        addLog(`[CAS] Initiating upload stream for "${file.name}"...`);

        const formData = new FormData();
        if (repoId.trim()) {
            formData.append('repo_id', repoId.trim());
        }
        formData.append('model', file);

        try {
            // Simulate progress for visual feedback
            const progressInterval = setInterval(() => {
                setUploadProgress(prev => Math.min(prev + 10, 90));
            }, 200);

            const response = await fetch('http://localhost:3000/api/upload', {
                method: 'POST',
                body: formData,
            });

            clearInterval(progressInterval);
            const data = await response.json();

            if (data.status === 'uploaded') {
                setUploadProgress(100);
                setUploadState('hashing'); // Visual step
                setTimeout(() => {
                    setUploadState('complete');
                    setSelectedModel(data.filename);
                    setAvailableModels(prev => [...prev, data.filename]);
                    addLog(`[CAS] Upload Complete. Indexed as ${data.filename}`);
                    setChatHistory(prev => [...prev, {
                        id: Date.now().toString(),
                        role: 'system',
                        content: `Model "${file.name}" loaded successfully.`,
                        timestamp: Date.now()
                    }]);
                    // Reset upload state after a delay
                    setTimeout(() => setUploadState('idle'), 2000);
                }, 800);
            } else {
                setUploadState('idle');
                addLog(`[Error] Upload failed: ${data.error}`);
            }
        } catch (e) {
            setUploadState('idle');
            addLog(`[Error] Network error during upload: ${e}`);
        }
    };

    const handleSendPrompt = async () => {
        if (!promptInput.trim() || !activeModel) return;

        const newPrompt = promptInput;
        setPromptInput('');
        setIsInferencing(true);
        setInferenceStats({ tps: 0, progress: 10 }); // Start progress

        // Add User Message
        setChatHistory(prev => [...prev, {
            id: Date.now().toString(),
            role: 'user',
            content: newPrompt,
            timestamp: Date.now()
        }]);

        addLog(`[RPC] Broadcasting RunRemoteInference...`);

        // Simulate task assignment visual if peers exist
        const workerPeer = peers.length > 0 ? peers[0] : null;
        if (workerPeer) {
            addLog(`[Scheduler] Task assigned to Node ${workerPeer.id} (${workerPeer.address})`);
            setPeers(prev => prev.map(p => p.id === workerPeer.id ? { ...p, status: 'computing' } : p));
        }

        // Fake progress while waiting for real response
        const progressInterval = setInterval(() => {
            setInferenceStats(prev => ({
                tps: prev.tps,
                progress: Math.min(prev.progress + 5, 90)
            }));
        }, 500);

        try {
            const response = await fetch('http://localhost:3000/api/inference', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    model_path: selectedModel,
                    prompt: newPrompt,
                    tokenizer_path: null
                })
            });

            clearInterval(progressInterval);
            const data = await response.json();

            // Reset peer status
            if (workerPeer) {
                setPeers(prev => prev.map(p => p.id === workerPeer.id ? { ...p, status: 'active' } : p));
            }
            setIsInferencing(false);

            if (data.result) {
                setInferenceStats({ tps: 24.5, progress: 100 }); // Mock TPS, real completion
                addLog('[RPC] Result stream closed.');
                setChatHistory(prev => [...prev, {
                    id: Date.now().toString(),
                    role: 'assistant',
                    content: data.result,
                    timestamp: Date.now()
                }]);
            } else {
                addLog(`[Error] Inference failed: ${data.error}`);
                // Show error in chat
                setChatHistory(prev => [...prev, {
                    id: Date.now().toString(),
                    role: 'system',
                    content: `Error: ${data.error}`,
                    timestamp: Date.now()
                }]);
            }
        } catch (e) {
            clearInterval(progressInterval);
            setIsInferencing(false);
            if (workerPeer) {
                setPeers(prev => prev.map(p => p.id === workerPeer.id ? { ...p, status: 'active' } : p));
            }
            addLog(`[Error] Network error: ${e}`);
            setChatHistory(prev => [...prev, {
                id: Date.now().toString(),
                role: 'system',
                content: `Network Error: ${e}`,
                timestamp: Date.now()
            }]);
        }
    };

    return (
        <div className="min-h-screen bg-slate-950 text-slate-200 font-sans p-6 flex flex-col">

            {/* Header */}
            <header className="flex justify-between items-center mb-6 border-b border-slate-800 pb-4">
                <div className="flex items-center gap-3">
                    <div className="w-10 h-10 bg-indigo-600 rounded-lg flex items-center justify-center shadow-lg shadow-indigo-500/20">
                        <Share2 className="text-white w-6 h-6" />
                    </div>
                    <div>
                        <h1 className="text-xl font-bold text-white tracking-tight">Hive Control Plane</h1>
                        <div className="flex items-center gap-2 text-xs text-slate-400 font-mono">
                            <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
                            ID: {nodeId} • Role: QUEEN
                        </div>
                    </div>
                </div>

                <div className="flex items-center gap-4 bg-slate-900 px-4 py-2 rounded-lg border border-slate-800">
                    <Network className="w-4 h-4 text-indigo-400" />
                    <span className="text-xs font-mono text-indigo-300">DHT Network: SYNCED</span>
                </div>

                {/* System Metrics */}
                {metrics && (
                    <div className="flex items-center gap-4 bg-slate-900 px-4 py-2 rounded-lg border border-slate-800 ml-4">
                        <div className="flex items-center gap-2">
                            <Cpu className="w-4 h-4 text-emerald-400" />
                            <span className="text-xs font-mono text-emerald-300">CPU: {metrics.cpu_usage.toFixed(1)}%</span>
                        </div>
                        <div className="w-[1px] h-3 bg-slate-700"></div>
                        <div className="flex items-center gap-2">
                            <Activity className="w-4 h-4 text-purple-400" />
                            <span className="text-xs font-mono text-purple-300">RAM: {(metrics.used_mem / 1024 / 1024 / 1024).toFixed(1)} GB</span>
                        </div>
                    </div>
                )}
            </header>

            <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 flex-1">

                {/* Left Column: Model Management (4 cols) */}
                <div className="lg:col-span-4 space-y-6">

                    {/* Upload Zone */}
                    <div className="bg-slate-900 border border-slate-800 rounded-xl p-6 shadow-sm">
                        <h2 className="text-xs font-bold text-slate-500 uppercase tracking-wider mb-4 flex items-center gap-2">
                            <Database className="w-4 h-4" /> CAS Model Loader
                        </h2>

                        {/* P2P Tokenizer Config */}
                        <div className="mb-4">
                            <label className="text-[10px] uppercase text-slate-500 font-bold mb-1 block">HuggingFace Repo ID (Optional)</label>
                            <input
                                type="text"
                                placeholder="e.g. mistralai/Mistral-7B-v0.1"
                                className="w-full bg-slate-950 border border-slate-800 rounded px-2 py-1 text-xs text-white placeholder-slate-600 focus:outline-none focus:border-indigo-500 mb-1"
                                value={repoId}
                                onChange={(e) => setRepoId(e.target.value)}
                            />
                            <p className="text-[10px] text-slate-500">Auto-downloads tokenizer.json for deployed agents.</p>
                        </div>

                        {!activeModel ? (
                            <div
                                onClick={() => uploadState === 'idle' && fileInputRef.current?.click()}
                                className={`
                  border-2 border-dashed rounded-xl p-8 flex flex-col items-center justify-center text-center transition-all cursor-pointer
                  ${uploadState === 'idle'
                                        ? 'border-slate-700 hover:border-indigo-500 hover:bg-slate-800/50'
                                        : 'border-indigo-500/50 bg-indigo-500/5'}
                `}
                            >
                                {uploadState === 'idle' && (
                                    <>
                                        <UploadCloud className="w-10 h-10 text-indigo-400 mb-3" />
                                        <h3 className="text-sm font-medium text-white">Click to Upload GGUF</h3>
                                        <p className="text-xs text-slate-500 mt-1">Simulates file hashing & DHT announcement</p>
                                    </>
                                )}

                                {(uploadState === 'uploading' || uploadState === 'hashing') && (
                                    <div className="w-full">
                                        <div className="flex justify-between text-xs mb-2">
                                            <span className="text-indigo-300">
                                                {uploadState === 'uploading' ? 'Streaming chunks...' : 'Calculating SHA-256...'}
                                            </span>
                                            <span className="text-white font-mono">{uploadProgress}%</span>
                                        </div>
                                        <div className="h-1.5 bg-slate-800 rounded-full overflow-hidden">
                                            <div
                                                className="h-full bg-indigo-500 transition-all duration-300 ease-out"
                                                style={{ width: `${uploadProgress}%` }}
                                            />
                                        </div>
                                    </div>
                                )}
                            </div>
                        ) : (
                            <div className="bg-emerald-500/10 border border-emerald-500/20 rounded-xl p-4">
                                <div className="flex items-start gap-3">
                                    <div className="p-2 bg-emerald-500/20 rounded-lg">
                                        <Box className="w-5 h-5 text-emerald-400" />
                                    </div>
                                    <div className="flex-1 min-w-0">
                                        <h3 className="text-sm font-bold text-emerald-100 truncate">{activeModel.name}</h3>
                                        <div className="flex items-center gap-2 mt-1">
                                            <span className="text-[10px] bg-slate-800 text-slate-400 px-1.5 py-0.5 rounded font-mono border border-slate-700">
                                                {activeModel.size}
                                            </span>
                                            <span className="text-[10px] text-emerald-400/80 font-mono truncate">
                                                CID: {activeModel.cid}
                                            </span>
                                        </div>
                                    </div>
                                    <CheckCircle2 className="w-4 h-4 text-emerald-500" />
                                </div>
                            </div>
                        )}

                        <div className="mt-4 pt-4 border-t border-slate-800">
                            <div className="flex items-center gap-2 text-xs text-slate-500">
                                <Lock className="w-3 h-3" />
                                <span>Files are encrypted & content-addressed</span>
                            </div>
                        </div>
                    </div>

                    {/* Node Status */}
                    <div className="bg-slate-900 border border-slate-800 rounded-xl p-5 shadow-sm flex-1">
                        <h2 className="text-xs font-bold text-slate-500 uppercase tracking-wider mb-4 flex items-center gap-2">
                            <Server className="w-4 h-4" /> Available Compute
                        </h2>
                        <div className="space-y-3">
                            {peers.length === 0 ? (
                                <div className="p-4 text-center text-slate-600 text-xs italic border border-slate-800 rounded-lg border-dashed">
                                    Waiting for swarm peers...
                                </div>
                            ) : (
                                peers.map(peer => (
                                    <div key={peer.id} className="flex items-center justify-between p-3 bg-slate-950 rounded-lg border border-slate-800">
                                        <div className="flex items-center gap-3">
                                            <div className={`w-2 h-2 rounded-full ${peer.status === 'computing' ? 'bg-indigo-400 animate-pulse' : 'bg-emerald-500'}`} />
                                            <div>
                                                <div className="text-sm font-medium text-slate-300 max-w-[150px] truncate" title={peer.id}>{peer.id}</div>
                                                <div className="text-[10px] text-slate-500 font-mono">{peer.address}</div>
                                            </div>
                                        </div>
                                        {peer.status === 'computing' && (
                                            <span className="text-[10px] bg-indigo-500/20 text-indigo-300 px-2 py-1 rounded animate-pulse">
                                                Running Inference...
                                            </span>
                                        )}
                                    </div>
                                )))}
                        </div>
                    </div>

                </div>

                {/* Center Column: Inference Chat (5 cols) */}
                <div className="lg:col-span-5 flex flex-col bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-lg relative">

                    {/* Top Bar with Model Dropdown and Upload Button */}
                    <div className="p-3 bg-slate-900/80 backdrop-blur border-b border-slate-800 flex justify-between items-center z-10">
                        <div className="flex items-center gap-2">
                            <Zap className="w-4 h-4 text-yellow-500" />
                            <span className="text-sm font-medium text-slate-300">Remote Inference Stream</span>
                        </div>

                        <div className="flex items-center gap-2">
                            <select
                                className="bg-slate-950 border border-slate-800 text-emerald-400 text-[10px] rounded px-2 py-1 focus:outline-none focus:border-indigo-500 max-w-[150px]"
                                value={selectedModel || ""}
                                onChange={(e) => setSelectedModel(e.target.value || null)}
                            >
                                <option value="">Select Model...</option>
                                {availableModels.map(m => (
                                    <option key={m} value={m}>{m}</option>
                                ))}
                            </select>

                            {/* Hidden input for the upload functionality */}
                            <input
                                type="file"
                                ref={fileInputRef}
                                onChange={handleFileSelect}
                                className="hidden"
                                accept=".gguf,.bin"
                            />

                            <button
                                onClick={() => fileInputRef.current?.click()}
                                disabled={uploadState !== 'idle'}
                                className="bg-indigo-600 hover:bg-indigo-500 text-white rounded p-1"
                                title="Upload New Model"
                            >
                                <UploadCloud className="w-3 h-3" />
                            </button>
                        </div>
                    </div>

                    {/* Chat History */}
                    <div className="flex-1 overflow-y-auto p-4 space-y-4 bg-slate-950/50">
                        {chatHistory.map((msg) => (
                            <div key={msg.id} className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
                                <div className={`
                  max-w-[85%] rounded-lg p-3 text-sm leading-relaxed
                  ${msg.role === 'user'
                                        ? 'bg-indigo-600 text-white rounded-br-none'
                                        : msg.role === 'system'
                                            ? 'bg-slate-800/50 text-slate-400 text-xs border border-slate-800 w-full max-w-full text-center py-2'
                                            : 'bg-slate-800 text-slate-200 border border-slate-700 rounded-bl-none'}
                `}>
                                    {msg.role === 'assistant' && (
                                        <div className="text-[10px] text-emerald-400 mb-1 font-bold uppercase tracking-wider flex items-center gap-1">
                                            <FileCode className="w-3 h-3" /> Generated Output
                                        </div>
                                    )}
                                    {msg.content}
                                </div>
                            </div>
                        ))}
                        <div ref={chatEndRef} />
                    </div>

                    {/* Progress Bar Overlay */}
                    {isInferencing && (
                        <div className="h-1 bg-slate-800 w-full">
                            <div className="h-full bg-indigo-500 transition-all duration-200" style={{ width: `${inferenceStats.progress}%` }} />
                        </div>
                    )}

                    {/* Input Area */}
                    <div className="p-4 bg-slate-900 border-t border-slate-800">
                        <div className="flex gap-2">
                            <input
                                type="text"
                                value={promptInput}
                                onChange={(e) => setPromptInput(e.target.value)}
                                onKeyDown={(e) => e.key === 'Enter' && handleSendPrompt()}
                                placeholder={activeModel ? "Send prompt to swarm..." : "Upload or select a model first"}
                                disabled={!activeModel || isInferencing}
                                className="flex-1 bg-slate-950 border border-slate-800 rounded-lg px-4 py-2 text-sm text-white placeholder-slate-600 focus:outline-none focus:border-indigo-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                            />
                            <button
                                onClick={handleSendPrompt}
                                disabled={!activeModel || !promptInput.trim() || isInferencing}
                                className="bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 disabled:hover:bg-indigo-600 text-white rounded-lg px-4 py-2 transition-colors flex items-center justify-center"
                            >
                                <Send className="w-4 h-4" />
                            </button>
                        </div>
                    </div>
                </div>

                {/* Right Column: Logs (3 cols) */}
                <div className="lg:col-span-3 flex flex-col bg-black border border-slate-800 rounded-xl overflow-hidden shadow-sm">
                    <div className="flex items-center gap-2 text-slate-500 p-3 border-b border-slate-900 bg-slate-950">
                        <Terminal className="w-3 h-3" />
                        <span className="text-xs font-bold uppercase">Event Log</span>
                    </div>
                    <div className="flex-1 overflow-y-auto p-3 space-y-2 font-mono text-[10px]">
                        {logs.map((log, i) => (
                            <div key={i} className="text-slate-400 break-all leading-relaxed border-l-2 border-slate-800 pl-2 hover:bg-slate-900/30 transition-colors">
                                <span className="text-indigo-500 opacity-70 mr-2">➜</span>
                                {log}
                            </div>
                        ))}
                    </div>
                </div>

            </div>
        </div>
    );
};

export default HiveDashboard;
