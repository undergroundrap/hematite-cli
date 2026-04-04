# Hematite Memory Architecture: The Sovereign Record

Hematite manages state through two specialized subsystems that balance persistent awareness with immediate context window efficiency.

## 1. The Vein (Long-Term RAG)
Managed in `src/memory/vein.rs`, **The Vein** is a local SQLite-backed RAG engine.
- **Indexing**: Files are automatically indexed into `hematite_memory.db` using a BM25-compatible search strategy (via `rusqlite`).
- **Retrieval**: When the agent is unsure about a file's location, it queries the Vein to retrieve path-relevant snippets without bloating the current context window.

## 2. Smart Sovereign Compaction (Short-Term Context)
Managed in `src/agent/conversation.rs`, this is the deterministic engine that handles the active 32k context window.
- **Trigger**: Activates when the conversation length exceeds 16 messages or total characters hit a threshold.
- **The Pillar**: Instead of AI-generated summaries which can hallucinate, Hematite performs **Deterministic Compaction**.
- **Key Files**: Automatically extracts and preserves the list of files currently being modified.
- **Verbatim Timeline**: Keeps the most recent 15 messages exactly as they occurred.
- **Technical Alignment**: Enforces **Global Sequence Alignment** (User-role must start every turn) and **Orphan Purges** to ensure LM Studio never receives a malformed history.

## 3. DeepReflect (Idle Reflection)
A background process that triggers during user idle time to perform deeper summarization and knowledge distillation, updating the Vein with "learned" insights about the project structure.
