# Contributing to Hematite

Welcome to the Forge. Hematite is built as an aggressive, local-first alternative to cloud-tethered development environments. Because of the strict 12GB VRAM limits enforced by the `Arc<InferenceEngine>`, adding new tools to the ecosystem requires strict optimization.

## How to Add a New Tool ("The Forger's Guide")

Hematite uses a deterministic `OnceLock` Schema caching layer in `src/tools/tool_schema_cache.rs` to prevent sub-second LLM load latency. If you deploy a new tool, it must be mapped securely across 3 bounds:

### 1. Register the JSON Schema
Open `src/tools/tool_schema_cache.rs` and inject the raw string schema natively inside the cache array:

```json
{
    "name": "YourNewTool",
    "description": "Short, precise description of when the agent should use this tool.",
    "parameters": {
        "arg1": { "type": "string" }
    }
}
```

### 2. Build the Logic implementation
Create `src/tools/your_tool.rs`. The logic must run physically on the OS. **Crucially: All path interactions MUST pass through the Glass Wall.**

If your tool reads/writes paths, you must invoke `guard::path_is_safe(&workspace_root, &target)` before executing local File I/O. 

### 3. Expose the Module
Update `src/tools/mod.rs` to expose your struct natively into the CLI binary limits!

---
**Performance Note**: Never initialize a persistent loop or massive memory map array in a tool. Tools must execute *synchronously* and return lightweight JSON strings natively back to "The Vein" RAG Context in under 3 seconds to keep Swarm iterations flowing.
