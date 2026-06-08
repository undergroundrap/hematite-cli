use serde_json::{json, Value};

pub fn gpu_tools_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["estimate", "batch", "info", "parse", "budget"],
                "description": "estimate: VRAM needed for a model | batch: max batch size in VRAM | info: GPU specs by name | parse: parse nvidia-smi text output | budget: VRAM budget allocation planner"
            },
            "params": {"type": "number", "description": "Model parameter count in billions (e.g. 7.0 for 7B)"},
            "quant": {"type": "string", "description": "Quantization: fp32, fp16, bf16, q8, q6, q5, q4, q3, q2, q1 — or bits per weight like '4bit'"},
            "context": {"type": "integer", "description": "Context length in tokens (default 4096)"},
            "batch_size": {"type": "integer", "description": "Batch size for batch action"},
            "vram_gb": {"type": "number", "description": "Available VRAM in GB"},
            "gpu": {"type": "string", "description": "GPU model name for info action (e.g. 'RTX 4070', '3090', 'A100')"},
            "text": {"type": "string", "description": "nvidia-smi output text to parse"},
            "overhead_gb": {"type": "number", "description": "Reserved VRAM overhead in GB (default 1.0)"},
            "kv_cache": {"type": "boolean", "description": "Include KV cache in estimate (default true)"},
            "head_dim": {"type": "integer", "description": "Attention head dimension (default 128)"},
            "num_heads": {"type": "integer", "description": "Number of attention heads (default 32)"},
            "num_layers": {"type": "integer", "description": "Number of transformer layers (default 32)"}
        },
        "required": []
    })
}

struct GpuSpec {
    name: &'static str,
    vram_gb: f64,
    cuda_cores: u32,
    tensor_cores: u32,
    bandwidth_gbps: f64,
    tflops_fp16: f64,
    architecture: &'static str,
}

static GPU_TABLE: &[GpuSpec] = &[
    GpuSpec {
        name: "RTX 4090",
        vram_gb: 24.0,
        cuda_cores: 16384,
        tensor_cores: 512,
        bandwidth_gbps: 1008.0,
        tflops_fp16: 165.2,
        architecture: "Ada Lovelace",
    },
    GpuSpec {
        name: "RTX 4080 SUPER",
        vram_gb: 16.0,
        cuda_cores: 10240,
        tensor_cores: 320,
        bandwidth_gbps: 736.3,
        tflops_fp16: 104.9,
        architecture: "Ada Lovelace",
    },
    GpuSpec {
        name: "RTX 4080",
        vram_gb: 16.0,
        cuda_cores: 9728,
        tensor_cores: 304,
        bandwidth_gbps: 716.8,
        tflops_fp16: 97.5,
        architecture: "Ada Lovelace",
    },
    GpuSpec {
        name: "RTX 4070 Ti SUPER",
        vram_gb: 16.0,
        cuda_cores: 8448,
        tensor_cores: 264,
        bandwidth_gbps: 672.3,
        tflops_fp16: 88.9,
        architecture: "Ada Lovelace",
    },
    GpuSpec {
        name: "RTX 4070 Ti",
        vram_gb: 12.0,
        cuda_cores: 7680,
        tensor_cores: 240,
        bandwidth_gbps: 504.2,
        tflops_fp16: 82.6,
        architecture: "Ada Lovelace",
    },
    GpuSpec {
        name: "RTX 4070 SUPER",
        vram_gb: 12.0,
        cuda_cores: 7168,
        tensor_cores: 224,
        bandwidth_gbps: 504.2,
        tflops_fp16: 75.8,
        architecture: "Ada Lovelace",
    },
    GpuSpec {
        name: "RTX 4070",
        vram_gb: 12.0,
        cuda_cores: 5888,
        tensor_cores: 184,
        bandwidth_gbps: 504.2,
        tflops_fp16: 58.5,
        architecture: "Ada Lovelace",
    },
    GpuSpec {
        name: "RTX 4060 Ti",
        vram_gb: 8.0,
        cuda_cores: 4352,
        tensor_cores: 136,
        bandwidth_gbps: 288.0,
        tflops_fp16: 44.1,
        architecture: "Ada Lovelace",
    },
    GpuSpec {
        name: "RTX 4060",
        vram_gb: 8.0,
        cuda_cores: 3072,
        tensor_cores: 96,
        bandwidth_gbps: 272.0,
        tflops_fp16: 30.0,
        architecture: "Ada Lovelace",
    },
    GpuSpec {
        name: "RTX 3090 Ti",
        vram_gb: 24.0,
        cuda_cores: 10752,
        tensor_cores: 336,
        bandwidth_gbps: 1008.0,
        tflops_fp16: 80.0,
        architecture: "Ampere",
    },
    GpuSpec {
        name: "RTX 3090",
        vram_gb: 24.0,
        cuda_cores: 10496,
        tensor_cores: 328,
        bandwidth_gbps: 936.2,
        tflops_fp16: 71.0,
        architecture: "Ampere",
    },
    GpuSpec {
        name: "RTX 3080 Ti",
        vram_gb: 12.0,
        cuda_cores: 10240,
        tensor_cores: 320,
        bandwidth_gbps: 912.4,
        tflops_fp16: 68.0,
        architecture: "Ampere",
    },
    GpuSpec {
        name: "RTX 3080",
        vram_gb: 10.0,
        cuda_cores: 8704,
        tensor_cores: 272,
        bandwidth_gbps: 760.3,
        tflops_fp16: 58.0,
        architecture: "Ampere",
    },
    GpuSpec {
        name: "RTX 3070 Ti",
        vram_gb: 8.0,
        cuda_cores: 6144,
        tensor_cores: 192,
        bandwidth_gbps: 608.3,
        tflops_fp16: 42.2,
        architecture: "Ampere",
    },
    GpuSpec {
        name: "RTX 3070",
        vram_gb: 8.0,
        cuda_cores: 5888,
        tensor_cores: 184,
        bandwidth_gbps: 448.0,
        tflops_fp16: 40.0,
        architecture: "Ampere",
    },
    GpuSpec {
        name: "RTX 3060 Ti",
        vram_gb: 8.0,
        cuda_cores: 4864,
        tensor_cores: 152,
        bandwidth_gbps: 448.0,
        tflops_fp16: 32.7,
        architecture: "Ampere",
    },
    GpuSpec {
        name: "RTX 3060",
        vram_gb: 12.0,
        cuda_cores: 3584,
        tensor_cores: 112,
        bandwidth_gbps: 360.0,
        tflops_fp16: 25.3,
        architecture: "Ampere",
    },
    GpuSpec {
        name: "A100 80GB",
        vram_gb: 80.0,
        cuda_cores: 6912,
        tensor_cores: 432,
        bandwidth_gbps: 2000.0,
        tflops_fp16: 312.0,
        architecture: "Ampere",
    },
    GpuSpec {
        name: "A100 40GB",
        vram_gb: 40.0,
        cuda_cores: 6912,
        tensor_cores: 432,
        bandwidth_gbps: 1555.0,
        tflops_fp16: 312.0,
        architecture: "Ampere",
    },
    GpuSpec {
        name: "H100 80GB",
        vram_gb: 80.0,
        cuda_cores: 16896,
        tensor_cores: 528,
        bandwidth_gbps: 3350.0,
        tflops_fp16: 756.0,
        architecture: "Hopper",
    },
    GpuSpec {
        name: "RTX 2080 Ti",
        vram_gb: 11.0,
        cuda_cores: 4352,
        tensor_cores: 544,
        bandwidth_gbps: 616.0,
        tflops_fp16: 26.9,
        architecture: "Turing",
    },
    GpuSpec {
        name: "RTX 2080 SUPER",
        vram_gb: 8.0,
        cuda_cores: 3072,
        tensor_cores: 384,
        bandwidth_gbps: 496.1,
        tflops_fp16: 22.3,
        architecture: "Turing",
    },
];

fn find_gpu(name: &str) -> Option<&'static GpuSpec> {
    let lower = name.to_lowercase();
    GPU_TABLE
        .iter()
        .find(|g| {
            let gname = g.name.to_lowercase();
            gname.contains(&lower) || lower.contains(&gname.replace("rtx ", "").replace(" ", ""))
        })
        .or_else(|| {
            // fuzzy: try matching key tokens
            let tokens: Vec<&str> = lower.split_whitespace().collect();
            GPU_TABLE.iter().find(|g| {
                let gname = g.name.to_lowercase();
                tokens.iter().all(|t| gname.contains(t))
            })
        })
}

fn bits_per_weight(quant: &str) -> f64 {
    let q = quant.to_lowercase();
    match q.as_str() {
        "fp32" | "float32" => 32.0,
        "fp16" | "float16" | "half" => 16.0,
        "bf16" | "bfloat16" => 16.0,
        "q8" | "int8" | "8bit" | "8" => 8.5, // slightly above 8 for overhead
        "q6" | "6bit" | "6" | "q6_k" => 6.5,
        "q5" | "5bit" | "5" | "q5_k_m" | "q5_k_s" => 5.5,
        "q4" | "4bit" | "4" | "q4_k_m" | "q4_k_s" | "q4_0" | "q4_1" => 4.5,
        "q3" | "3bit" | "3" | "q3_k_m" | "q3_k_s" | "q3_k_l" => 3.5,
        "q2" | "2bit" | "2" | "q2_k" => 2.6,
        "q1" | "1bit" | "1" | "iq1_m" | "iq1_s" => 1.7,
        _ => {
            // try parsing a number from e.g. "q4_k" or "4.5bit"
            if let Ok(n) = q
                .chars()
                .filter(|c| c.is_numeric() || *c == '.')
                .collect::<String>()
                .parse::<f64>()
            {
                n + 0.5
            } else {
                4.5 // default q4
            }
        }
    }
}

fn model_vram_gb(
    params_b: f64,
    bpw: f64,
    context: u64,
    include_kv: bool,
    num_layers: u32,
    num_heads: u32,
    head_dim: u32,
) -> (f64, f64, f64) {
    let params = params_b * 1e9;
    // weights
    let weights_gb = (params * bpw / 8.0) / 1e9;
    // KV cache: 2 * num_layers * num_heads * head_dim * context * 2 bytes (fp16)
    let kv_gb = if include_kv {
        (2.0 * num_layers as f64 * num_heads as f64 * head_dim as f64 * context as f64 * 2.0) / 1e9
    } else {
        0.0
    };
    let overhead_gb = weights_gb * 0.05 + 0.3; // 5% overhead + 300 MB runtime
    (weights_gb, kv_gb, overhead_gb)
}

fn action_estimate(args: &Value) -> Result<String, String> {
    let params_b = args
        .get("params")
        .and_then(|v| v.as_f64())
        .ok_or("Provide 'params' (parameter count in billions, e.g. 7.0 for 7B).")?;
    let quant = args.get("quant").and_then(|v| v.as_str()).unwrap_or("q4");
    let context = args.get("context").and_then(|v| v.as_u64()).unwrap_or(4096);
    let include_kv = args
        .get("kv_cache")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let num_layers = args
        .get("num_layers")
        .and_then(|v| v.as_u64())
        .unwrap_or(32) as u32;
    let num_heads = args.get("num_heads").and_then(|v| v.as_u64()).unwrap_or(32) as u32;
    let head_dim = args.get("head_dim").and_then(|v| v.as_u64()).unwrap_or(128) as u32;

    let bpw = bits_per_weight(quant);
    let (weights_gb, kv_gb, overhead_gb) = model_vram_gb(
        params_b, bpw, context, include_kv, num_layers, num_heads, head_dim,
    );
    let total_gb = weights_gb + kv_gb + overhead_gb;

    let mut out = format!(
        "Model VRAM Estimate\n{}\n\nModel:       {:.1}B params @ {} ({:.1} bpw)\nContext:     {} tokens\n\n",
        "=".repeat(40),
        params_b, quant, bpw, context
    );
    out += &format!("Weights:     {:.2} GB\n", weights_gb);
    if include_kv {
        out += &format!(
            "KV cache:    {:.2} GB  ({} layers × {} heads × {} dim × {} ctx × fp16)\n",
            kv_gb, num_layers, num_heads, head_dim, context
        );
    }
    out += &format!("Overhead:    {:.2} GB\n", overhead_gb);
    out += &format!("{}\n", "-".repeat(35));
    out += &format!("TOTAL:       {:.2} GB\n\n", total_gb);

    // GPU fit table
    out += "GPU Fit Check:\n";
    for gpu in GPU_TABLE
        .iter()
        .filter(|g| g.vram_gb >= 8.0 && g.vram_gb <= 24.0)
    {
        let fits = if total_gb <= gpu.vram_gb {
            "✓ fits"
        } else {
            "✗ no  "
        };
        let head = if total_gb <= gpu.vram_gb {
            format!("{:.1} GB free", gpu.vram_gb - total_gb)
        } else {
            format!("{:.1} GB short", total_gb - gpu.vram_gb)
        };
        out += &format!(
            "  {:22} {:>6} GB   {} — {}\n",
            gpu.name, gpu.vram_gb, fits, head
        );
    }
    Ok(out)
}

fn action_batch(args: &Value) -> Result<String, String> {
    let params_b = args
        .get("params")
        .and_then(|v| v.as_f64())
        .ok_or("Provide 'params' (parameter count in billions).")?;
    let quant = args.get("quant").and_then(|v| v.as_str()).unwrap_or("q4");
    let vram_gb = args
        .get("vram_gb")
        .and_then(|v| v.as_f64())
        .ok_or("Provide 'vram_gb' (available VRAM in GB).")?;
    let overhead_gb = args
        .get("overhead_gb")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);
    let context = args.get("context").and_then(|v| v.as_u64()).unwrap_or(4096);
    let num_layers = args
        .get("num_layers")
        .and_then(|v| v.as_u64())
        .unwrap_or(32) as u32;
    let num_heads = args.get("num_heads").and_then(|v| v.as_u64()).unwrap_or(32) as u32;
    let head_dim = args.get("head_dim").and_then(|v| v.as_u64()).unwrap_or(128) as u32;

    let bpw = bits_per_weight(quant);
    let (weights_gb, _, overhead) = model_vram_gb(
        params_b, bpw, context, false, num_layers, num_heads, head_dim,
    );
    let available = vram_gb - overhead_gb - weights_gb - overhead;

    if available <= 0.0 {
        return Ok(format!(
            "Batch Calculation\n{}\n\nModel weights ({:.2} GB) + overhead ({:.2} GB) exceed available VRAM ({:.2} GB).\nCannot fit even batch_size=1.\n",
            "=".repeat(40), weights_gb, overhead_gb + overhead, vram_gb
        ));
    }

    // each activation: roughly 2 * context * hidden_dim * fp16 bytes per layer
    let hidden_dim = (num_heads as f64 * head_dim as f64) as u64;
    let per_batch_gb = (2.0 * context as f64 * hidden_dim as f64 * num_layers as f64 * 2.0) / 1e9;
    let max_batch = if per_batch_gb > 0.0 {
        (available / per_batch_gb).floor() as u64
    } else {
        1
    };

    let mut out = format!(
        "Batch Size Calculation\n{}\n\n{:.1}B params @ {} | {:.1} GB VRAM\n\n",
        "=".repeat(40),
        params_b,
        quant,
        vram_gb
    );
    out += &format!("Model weights:  {:.2} GB\n", weights_gb);
    out += &format!("Runtime OH:     {:.2} GB\n", overhead + overhead_gb);
    out += &format!("Available:      {:.2} GB\n", available);
    out += &format!(
        "Per-batch mem:  {:.3} GB  (approx activations + KV at ctx={})\n\n",
        per_batch_gb, context
    );
    out += &format!("Max batch size: {}\n", max_batch.max(1));

    if max_batch < 1 {
        out += "\nNote: Estimated per-batch memory exceeds available VRAM. Use a shorter context or lower quant.\n";
    }
    Ok(out)
}

fn action_info(gpu_name: &str) -> String {
    match find_gpu(gpu_name) {
        Some(gpu) => {
            format!(
                "GPU Specifications: {}\n{}\n\nVRAM:           {:.0} GB GDDR6X\nArchitecture:   {}\nCUDA Cores:     {}\nTensor Cores:   {}\nMemory BW:      {:.0} GB/s\nFP16 TFLOPs:    {:.1} TFLOPS\n\nLLM Guidance:\n  fp16 model headroom:  {:.0} GB\n  Q4 model headroom:    {:.0} GB\n  Max Q4 7B models:     {}\n  Max Q4 13B models:    {}\n  Recommended quant:    {}\n",
                gpu.name,
                "=".repeat(40),
                gpu.vram_gb,
                gpu.architecture,
                gpu.cuda_cores,
                gpu.tensor_cores,
                gpu.bandwidth_gbps,
                gpu.tflops_fp16,
                gpu.vram_gb - 2.0,
                gpu.vram_gb - 1.0,
                (gpu.vram_gb / 4.5).floor() as u32,
                ((gpu.vram_gb - 1.0) / 7.5).floor() as u32,
                if gpu.vram_gb >= 16.0 { "Q5_K_M or Q6_K for best quality" }
                else if gpu.vram_gb >= 12.0 { "Q4_K_M (best quality/VRAM balance)" }
                else { "Q4_K_M or Q3_K_M for larger models" },
            )
        }
        None => {
            let mut out = format!(
                "GPU '{}' not found in table.\n\nAvailable GPUs:\n",
                gpu_name
            );
            for gpu in GPU_TABLE {
                out += &format!(
                    "  {:30} {:5.0} GB   {:>6.1} TFLOPS fp16\n",
                    gpu.name, gpu.vram_gb, gpu.tflops_fp16
                );
            }
            out
        }
    }
}

fn action_parse(text: &str) -> String {
    // Parse nvidia-smi plain-text output (default format)
    let mut out = format!("nvidia-smi Output Parse\n{}\n\n", "=".repeat(40));
    let mut found_gpu = false;

    for line in text.lines() {
        let line = line.trim();
        // Look for GPU name line: | NVIDIA GeForce RTX 4070 ...
        if line.starts_with('|')
            && (line.contains("NVIDIA")
                || line.contains("Tesla")
                || line.contains("Quadro")
                || line.contains("RTX")
                || line.contains("GTX"))
        {
            if let Some(content) = line.strip_prefix('|') {
                let content = content.trim_end_matches('|').trim();
                // typical: GPU name  Fan  Temp  Perf  Pwr:Usage/Cap
                out += &format!("GPU:  {}\n", content);
                found_gpu = true;
            }
        }
        // VRAM line: | XX MiB / YYYY MiB |
        if line.contains("MiB") && line.contains('/') && line.starts_with('|') {
            if let Some(content) = line.strip_prefix('|') {
                let content = content.trim_end_matches('|').trim();
                // try to extract used/total
                let parts: Vec<&str> = content.split_whitespace().collect();
                for (i, part) in parts.iter().enumerate() {
                    if *part == "MiB" && i > 0 {
                        let used = parts.get(i - 1).copied().unwrap_or("?");
                        if let Some(total_mib) = parts.get(i + 2) {
                            out += &format!("VRAM: {} / {} MiB\n", used, total_mib);
                        }
                    }
                }
            }
        }
        // Temperature and utilization
        if line.contains('%') && line.contains('C') && line.starts_with('|') {
            if let Some(content) = line.strip_prefix('|') {
                let content = content.trim_end_matches('|').trim();
                out += &format!("Load: {}\n", content);
            }
        }
    }

    if !found_gpu {
        out += "(No GPU lines detected. Paste the full output of 'nvidia-smi' as the 'text' argument.)\n\nExpected format:\n  +---...---+\n  | NVIDIA GeForce RTX 4070 ...|\n  | ... % |  ... C  | ...\n";
    }
    out
}

fn action_budget(args: &Value) -> Result<String, String> {
    let vram_gb = args
        .get("vram_gb")
        .and_then(|v| v.as_f64())
        .ok_or("Provide 'vram_gb' (total VRAM in GB).")?;
    let overhead_gb = args
        .get("overhead_gb")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);

    let available = vram_gb - overhead_gb;

    let mut out = format!(
        "VRAM Budget Planner\n{}\n\nTotal VRAM:  {:.1} GB\nOS / Driver: {:.1} GB\nAvailable:   {:.1} GB\n\n",
        "=".repeat(40),
        vram_gb,
        overhead_gb,
        available
    );

    out += "Model fit at common quantizations:\n";
    out += &format!(
        "{:<12} {:>8}  {:>8}  {:>8}  {:>8}  {:>8}\n",
        "Quant", "1B", "3B", "7B", "13B", "34B"
    );
    out += &format!("{}\n", "-".repeat(58));

    for (quant, bpw) in &[
        ("fp16", 16.0_f64),
        ("q8", 8.5),
        ("q6_K", 6.5),
        ("q4_K_M", 4.5),
        ("q3_K_M", 3.5),
        ("q2_K", 2.6),
    ] {
        let row: Vec<String> = [1.0_f64, 3.0, 7.0, 13.0, 34.0]
            .iter()
            .map(|&p| {
                let gb = (p * 1e9 * bpw / 8.0) / 1e9 + 0.5;
                if gb <= available {
                    format!("{:.1}GB✓", gb)
                } else {
                    format!("{:.1}GB✗", gb)
                }
            })
            .collect();
        out += &format!(
            "{:<12} {:>8}  {:>8}  {:>8}  {:>8}  {:>8}\n",
            quant, row[0], row[1], row[2], row[3], row[4]
        );
    }

    out += &format!(
        "\nLargest model that fits at Q4_K_M:  {:.0}B params\n",
        (available / (4.5 / 8.0)) / 1e9
    );
    out += &format!(
        "Largest model that fits at fp16:      {:.0}B params\n",
        (available / (16.0 / 8.0)) / 1e9
    );

    Ok(out)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("estimate");

    Ok(match action {
        "estimate" => action_estimate(args)?,
        "batch" => action_batch(args)?,
        "info" => {
            let gpu = args
                .get("gpu")
                .and_then(|v| v.as_str())
                .ok_or("Provide 'gpu' (GPU model name, e.g. 'RTX 4070').")?;
            action_info(gpu)
        }
        "parse" => {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or("Provide 'text' with the nvidia-smi output to parse.")?;
            action_parse(text)
        }
        "budget" => action_budget(args)?,
        other => format!(
            "Unknown action '{}'. Use: estimate, batch, info, parse, budget",
            other
        ),
    })
}
