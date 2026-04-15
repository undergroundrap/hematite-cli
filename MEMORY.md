# Hematite Memory Architecture

Hematite manages state through two specialized subsystems that balance persistent awareness with immediate context window efficiency.

## 1. The Vein (Local RAG)

Managed in `src/memory/vein.rs`. The Vein is a SQLite-backed hybrid retrieval engine that indexes the current project and injects relevant context into each turn.

- **Database:** stored at `.hematite/vein.db` inside the workspace root. Per-project — each folder gets its own index.
- **BM25 (always active):** SQLite FTS5 full-text search with Porter stemming. Works with no embedding model loaded.
- **Semantic (optional):** calls LM Studio's `/v1/embeddings` endpoint using `nomic-embed-text-v2`. Stores vectors in SQLite; reused across sessions.
- **Non-project directories:** source-file indexing is skipped when launched outside a real project (no `Cargo.toml`, `package.json`, `go.mod`, etc.), but The Vein stays active in docs-only mode. `.hematite/docs/`, `.hematite/imports/`, and recent local session reports remain searchable, and the status badge shows `VN:DOC`.
- **Retrieval:** at the start of each turn, changed files are re-indexed and a hybrid BM25+semantic query is run against the user's message. Top results are injected into the system prompt.

Status bar: `VN:SEM` (semantic active) / `VN:FTS` (BM25 only) / `VN:DOC` (docs/session memory only outside a project) / `VN:--` (not yet indexed or after a reset).

## 2. Context Compaction (Short-Term Context)

Managed in `src/agent/conversation.rs` and `src/agent/compaction.rs`.

- **Trigger:** activates when conversation length or token count approaches the context limit.
- **Strategy:** deterministic compaction — preserves key files, recent messages verbatim, and a rolling summary rather than relying on AI-generated summaries that can hallucinate.
- **Alignment:** enforces user-role message ordering required by LM Studio's Jinja templates.

## 3. PageRank Repo Map (Structural Memory)

Managed in `src/memory/repo_map.rs`.

- builds a definition/reference graph across all source files using `tree-sitter` AST parsing
- runs PageRank via `petgraph` to rank files by structural importance
- heat-weighted personalization: actively edited files get score boosts (hottest file 100×, others proportional) so architecturally central and actively edited files float to the top
- injected into the system prompt each turn so the model knows the codebase structure without burning tool calls
- rebuilt at startup and after every file edit

## 4. DeepReflect (Idle Reflection)

Managed in `src/memory/deep_reflect.rs`. A background process that triggers during user idle time to perform deeper summarization and distill session insights into the Vein.
