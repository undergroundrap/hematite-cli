use serde_json::Value;

pub async fn trace_runtime_flow(args: &Value) -> Result<String, String> {
    let topic = args
        .get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("user_turn");

    match topic {
        "user_turn" => Ok(trace_user_turn(args)),
        "session_reset" => Ok(trace_session_reset(args)),
        "reasoning_split" => Ok(trace_reasoning_split()),
        "runtime_subsystems" => Ok(trace_runtime_subsystems()),
        "startup" => Ok(trace_startup()),
        "voice" => Ok(trace_voice()),
        other => Err(format!(
            "Unknown topic '{}'. Use one of: user_turn, session_reset, reasoning_split, runtime_subsystems, startup, voice.",
            other
        )),
    }
}

fn trace_user_turn(args: &Value) -> String {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .unwrap_or("who are you?");

    format!(
        "Verified runtime trace for a normal text turn with input {:?}\n\n\
Visible chat output path\n\
1. Keyboard input is collected inside `run_app` in `src/ui/tui.rs`. When Enter is pressed on a non-slash command, the TUI drains `app.input`, pushes `You`, marks `app.agent_running = true`, and sends the text through `app.user_input_tx`.\n\
   File refs: `src/ui/tui.rs` -> `run_app`, `App::push_message`\n\
2. `app.user_input_tx` is the `user_input_tx` side of `tokio::sync::mpsc::channel::<UserTurn>(32)` assembled inside `build_runtime_bundle` in `src/runtime.rs`. The receiver side is `user_input_rx`.\n\
   File refs: `src/runtime.rs` -> `build_runtime_bundle`; `src/main.rs` -> `main`\n\
3. `main` spawns `run_agent_loop(AgentLoopRuntime {{ user_input_rx, agent_tx, ... }}, AgentLoopConfig {{ ... }})`. That loop waits on `user_input_rx.recv()` and forwards each turn into `ConversationManager::run_turn(&input, agent_tx.clone(), yolo)`.\n\
   File refs: `src/runtime.rs` -> `run_agent_loop`, `AgentLoopRuntime`, `AgentLoopConfig`; `src/agent/conversation.rs` -> `ConversationManager::run_turn`\n\
4. `ConversationManager::run_turn` handles slash-command short circuits first. For a normal prompt like {:?}, it builds the system prompt, updates `self.history`, queries Vein context, and calls `InferenceEngine::call_with_tools(&prompt_msgs, &self.tools, ...)`.\n\
   File refs: `src/agent/conversation.rs` -> `ConversationManager::run_turn`; `src/agent/inference.rs` -> `InferenceEngine::call_with_tools`\n\
5. If the model returns final text and no tool calls, `run_turn` strips think blocks with `strip_think_blocks`, records `ChatMessage::assistant_text(&cleaned)`, then streams the visible reply out as `InferenceEvent::Token(chunk)` values followed by `InferenceEvent::Done`.\n\
   File refs: `src/agent/conversation.rs` -> `ConversationManager::run_turn`; `src/agent/inference.rs` -> `strip_think_blocks`, `InferenceEvent`\n\
6. `run_app` receives those events on `agent_rx`, handles `InferenceEvent::Token` / `InferenceEvent::MutedToken`, ensures the current speaker is `Hematite`, and appends text with `app.update_last_message(token)`. `InferenceEvent::Done` clears busy state and finalizes reasoning state.\n\
   File refs: `src/ui/tui.rs` -> `run_app`, `InferenceEvent::Token`, `InferenceEvent::MutedToken`, `InferenceEvent::Done`\n\n\
Reasoning and specular path\n\
1. Model reasoning emitted inside `<think>` blocks is split out in `ConversationManager::run_turn` with `extract_think_block` and sent to the TUI as `InferenceEvent::Thought(thought)`.\n\
   File refs: `src/agent/conversation.rs` -> `ConversationManager::run_turn`; `src/agent/inference.rs` -> `extract_think_block`, `InferenceEvent::Thought`\n\
2. `run_app` handles `InferenceEvent::Thought` separately from visible chat text by setting `app.thinking = true` and appending the payload into `app.current_thought`.\n\
   File refs: `src/ui/tui.rs` -> `run_app`, `InferenceEvent::Thought`\n\
3. File-watcher diagnostics travel on a different channel. `build_runtime_bundle` creates `specular_tx` / `specular_rx`, `spawn_watcher(specular_tx)` starts the watcher, and `run_app` handles `SpecularEvent::FileChanged` and `SpecularEvent::SyntaxError` in a separate branch from `agent_rx`.\n\
   File refs: `src/runtime.rs` -> `build_runtime_bundle`; `src/agent/specular.rs` -> `spawn_watcher`, `SpecularEvent`; `src/ui/tui.rs` -> `run_app`\n\n\
Voice path\n\
1. `build_runtime_bundle` constructs `VoiceManager::new(agent_tx.clone())`, so the voice subsystem can emit `InferenceEvent::VoiceStatus` messages back into the same `agent_tx` channel used for model events.\n\
   File refs: `src/runtime.rs` -> `build_runtime_bundle`; `src/ui/voice.rs` -> `VoiceManager::new`\n\
2. In `run_app`, only `InferenceEvent::Token` triggers speech. `InferenceEvent::MutedToken` is displayed but not spoken. When speech is allowed, the TUI calls `app.voice_manager.speak(token.clone())`.\n\
   File refs: `src/ui/tui.rs` -> `run_app`, `InferenceEvent::Token`, `InferenceEvent::MutedToken`\n\
3. `VoiceManager::speak` pushes text into its internal sync channel. Background threads in `VoiceManager::new` assemble sentence chunks, synthesize them through `TTSKoko::tts_raw_audio_streaming`, and append PCM chunks into the active `rodio::Sink`.\n\
   File refs: `src/ui/voice.rs` -> `VoiceManager::speak`, `VoiceManager::new`\n\
4. When the turn finishes, `run_app` handles `InferenceEvent::Done` and calls `app.voice_manager.flush()` if voice is enabled.\n\
   File refs: `src/ui/tui.rs` -> `run_app`, `InferenceEvent::Done`\n\n\
Possible weak points\n\
- `ConversationManager::run_turn` in `src/agent/conversation.rs` is a very large control hub. It owns command handling, prompt assembly, tool orchestration, verification, and session persistence in one place.\n\
- `run_agent_loop` in `src/runtime.rs` is the single consumer of `user_input_rx`. If inference or tool work stalls inside one turn, the next user turn waits behind it.\n\
- `spawn_watcher` in `src/agent/specular.rs` runs `cargo check` after `.rs` file modifications and `run_app` can auto-inject a repair prompt from `SpecularEvent::SyntaxError` when the user is idle. That can be noisy or surprising during rapid edits."
        ,
        input,
        input
    )
}

fn trace_session_reset(args: &Value) -> String {
    let command = args
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("all");

    let header = match command {
        "/clear" => "Verified reset trace for /clear",
        "/new" => "Verified reset trace for /new",
        "/forget" => "Verified reset trace for /forget",
        _ => "Verified reset trace for /clear, /new, and /forget",
    };

    let mut out = format!("{header}\n\n");

    if command == "all" || command == "/clear" {
        out.push_str(
            "/clear\n\
1. The slash command is handled entirely inside `run_app` in `src/ui/tui.rs`.\n\
2. It clears TUI-local state: `messages`, `messages_raw`, `last_reasoning`, `current_thought`, `specular_logs`, `active_context`, and resets `current_objective` to `Idle`.\n\
3. It does not send anything through `user_input_tx`, so `ConversationManager::run_turn` is not involved.\n\
4. It pushes the visible system message `Dialogue buffer cleared.` and returns to the event loop.\n\
   File refs: `src/ui/tui.rs` -> `run_app`, `/clear` branch\n\n"
        );
    }

    if command == "all" || command == "/new" {
        out.push_str(
            "/new\n\
1. `run_app` clears the visible TUI state, clears pending attachments, pushes `You: /new`, marks `app.agent_running = true`, and sends `UserTurn::text(\"/new\")` through `app.user_input_tx`.\n\
   File refs: `src/ui/tui.rs` -> `run_app`, `/new` branch\n\
2. `run_agent_loop` receives `/new` from `user_input_rx` and forwards it into `ConversationManager::run_turn(&input, agent_tx.clone(), yolo)`.\n\
   File refs: `src/runtime.rs` -> `run_agent_loop`, `user_input_rx`\n\
3. `ConversationManager::run_turn` matches `user_input.trim() == \"/new\"`, then clears `history`, `reasoning_history`, `session_memory`, `running_summary`, `correction_hints`, and `pinned_files`, resets task files, removes `session.json`, rewrites the empty session file, and streams the fresh-context confirmation before `InferenceEvent::Done`.\n\
   File refs: `src/agent/conversation.rs` -> `ConversationManager::run_turn`, `reset_task_files`, `session_path`\n\n"
        );
    }

    if command == "all" || command == "/forget" {
        out.push_str(
            "/forget\n\
1. `run_app` clears the same visible TUI state as `/new`, clears pending attachments, pushes `You: /forget`, marks `app.agent_running = true`, and sends `UserTurn::text(\"/forget\")` through `app.user_input_tx`.\n\
   File refs: `src/ui/tui.rs` -> `run_app`, `/forget` branch\n\
2. `run_agent_loop` forwards that string into `ConversationManager::run_turn`.\n\
   File refs: `src/runtime.rs` -> `run_agent_loop`\n\
3. `ConversationManager::run_turn` matches `user_input.trim() == \"/forget\"`, clears `history`, `reasoning_history`, `session_memory`, `running_summary`, `correction_hints`, and `pinned_files`, resets task files, purges saved memory artifacts, resets the Vein index, removes and rewrites `session.json`, then streams the hard-forget confirmation before `InferenceEvent::Done`.\n\
   File refs: `src/agent/conversation.rs` -> `ConversationManager::run_turn`, `reset_task_files`, `session_path`\n\n"
        );
    }

    out.push_str(
        "Possible weak points\n\
- Reset behavior is split across `src/ui/tui.rs` and `src/agent/conversation.rs`, so future changes can drift if both sides are not updated together.\n\
- `/clear` is UI-only while `/new` and `/forget` cross the channel boundary into the agent loop. That difference is real and easy to misstate if it is not documented."
    );

    out
}

fn trace_reasoning_split() -> String {
    "Verified reasoning/specular split\n\n\
1. Model reasoning is represented by `InferenceEvent::Thought` in `src/agent/inference.rs`.\n\
2. `ConversationManager::run_turn` sends `InferenceEvent::Thought` when it extracts a `<think>` block or emits internal status updates during tool execution and verification.\n\
   File refs: `src/agent/conversation.rs` -> `ConversationManager::run_turn`; `src/agent/inference.rs` -> `InferenceEvent`, `extract_think_block`\n\
3. `run_app` handles `InferenceEvent::Thought` by appending to `app.current_thought`; it does not append that payload into the main dialogue transcript.\n\
   File refs: `src/ui/tui.rs` -> `run_app`, `InferenceEvent::Thought`\n\
4. Visible assistant text is carried on `InferenceEvent::Token` and `InferenceEvent::MutedToken`, then appended into the `Hematite` chat message via `app.update_last_message(token)`.\n\
   File refs: `src/ui/tui.rs` -> `run_app`, `InferenceEvent::Token`, `InferenceEvent::MutedToken`\n\
5. Watcher-driven specular events are separate from model reasoning. `SpecularEvent::FileChanged` and `SpecularEvent::SyntaxError` come from `spawn_watcher` in `src/agent/specular.rs` over `specular_rx`, not over `agent_rx`.\n\
   File refs: `src/runtime.rs` -> `build_runtime_bundle`; `src/agent/specular.rs` -> `spawn_watcher`, `SpecularEvent`; `src/ui/tui.rs` -> `run_app`\n\n\
Possible weak points\n\
- The SPECULAR panel currently mixes watcher events and model reasoning in one UI area, even though they arrive from different event sources.\n\
- `current_thought` and `last_reasoning` are TUI-managed buffers, so UI reset bugs can make the panel look stale even when the model state is clean."
        .to_string()
}

fn trace_runtime_subsystems() -> String {
    "Verified runtime subsystems\n\n\
- UI and input surface: `src/ui/tui.rs` -> `run_app`, `App`\n\
- Agent turn loop and tool orchestration: `src/agent/conversation.rs` -> `ConversationManager`, `ConversationManager::run_turn`\n\
- Model transport and event schema: `src/agent/inference.rs` -> `InferenceEngine`, `InferenceEvent`\n\
- Runtime bundle assembly and channels: `src/runtime.rs` -> `build_runtime_bundle`, `run_agent_loop`, `user_input_tx`, `user_input_rx`, `agent_tx`, `agent_rx`, `specular_rx`\n\
- Voice subsystem: `src/ui/voice.rs` -> `VoiceManager`\n\
- File watcher / specular subsystem: `src/agent/specular.rs` -> `spawn_watcher`, `SpecularEvent`\n\
- Memory and project indexing: `src/agent/conversation.rs` -> `initialize_vein`, `build_vein_context`; `src/memory/vein.rs`\n\
- External MCP tool bridge: `src/agent/mcp.rs`, `src/agent/mcp_manager.rs`\n\
- LSP bridge: `src/agent/lsp`, `src/tools/lsp_tools.rs`\n\n\
Primary communication paths\n\
- TUI to agent turn loop: `user_input_tx` -> `user_input_rx`\n\
- Agent turn loop to TUI: `agent_tx` -> `agent_rx` carrying `InferenceEvent`\n\
- File watcher to TUI: `specular_rx` carrying `SpecularEvent`\n\
- Swarm worker progress to TUI: `swarm_tx` -> `swarm_rx` carrying `SwarmMessage`\n\n\
Possible weak points\n\
- Runtime flow is distributed across `src/runtime.rs`, `src/main.rs`, `src/ui/tui.rs`, and `src/agent/conversation.rs`, so architectural questions are easy for the model to blur without a grounded helper.\n\
- `ConversationManager::run_turn` is still the highest-complexity subsystem and the main maintenance hotspot."
        .to_string()
}

fn trace_startup() -> String {
    "Verified startup flow\n\n\
1. `main` parses CLI args, then calls `build_runtime_bundle(...)` to assemble the runtime. That function builds `InferenceEngine`, starts GPU and git monitors, runs the LM Studio health check, detects the loaded model and context length, creates channels, starts the watcher, and constructs the voice/swarm services.\n\
   File refs: `src/main.rs` -> `main`; `src/runtime.rs` -> `build_runtime_bundle`; `src/agent/inference.rs` -> `InferenceEngine::new`, `health_check`, `get_loaded_model`, `detect_context_length`\n\
2. `main` spawns `run_agent_loop(...)` for steady-state turn handling and `spawn_runtime_profile_sync(...)` for background LM Studio profile refresh.\n\
   File refs: `src/main.rs` -> `main`; `src/runtime.rs` -> `run_agent_loop`, `spawn_runtime_profile_sync`\n\
3. `main` enters alternate-screen TUI mode and awaits `run_app(...)` with the already-assembled receivers, senders, and runtime services.\n\
   File refs: `src/main.rs` -> `main`; `src/ui/tui.rs` -> `run_app`\n\
4. Inside `run_agent_loop`, Hematite constructs `ConversationManager`, emits `InferenceEvent::RuntimeProfile`, initializes MCP and Vein, emits startup `InferenceEvent::Thought` / `InferenceEvent::Done`, then sends the boot greeting as `InferenceEvent::MutedToken`.\n\
   File refs: `src/runtime.rs` -> `run_agent_loop`; `src/agent/conversation.rs` -> `ConversationManager::new`, `initialize_mcp`, `initialize_vein`\n\n\
Possible weak points\n\
- Startup depends on LM Studio being available before the TUI fully launches.\n\
- `run_agent_loop` still mixes boot diagnostics and steady-state turn handling in one task, even though runtime assembly is cleaner now."
        .to_string()
}

fn trace_voice() -> String {
    "Verified voice synthesis flow\n\n\
Ctrl+T toggle\n\
1. `run_app` in `src/ui/tui.rs` handles `KeyCode::Char('t') if key.modifiers.contains(event::KeyModifiers::CONTROL)`. \
   It calls `app.voice_manager.toggle()` and pushes a System message showing the new state.\n\
   File refs: `src/ui/tui.rs` -> `run_app`, `KeyCode::Char('t')`, `KeyModifiers::CONTROL`\n\
2. `VoiceManager::toggle()` flips the internal `enabled` AtomicBool and returns the new state.\n\
   File refs: `src/ui/voice.rs` -> `VoiceManager::toggle`\n\n\
Speech pipeline\n\
1. `build_runtime_bundle` constructs `VoiceManager::new(agent_tx.clone())` so the voice subsystem \
   can emit `InferenceEvent::VoiceStatus` back on the agent channel.\n\
   File refs: `src/runtime.rs` -> `build_runtime_bundle`; `src/ui/voice.rs` -> `VoiceManager::new`\n\
2. Inside `run_app`, only `InferenceEvent::Token` triggers speech (not `MutedToken`). \
   When enabled and not muted, the TUI calls `app.voice_manager.speak(token.clone())`.\n\
   File refs: `src/ui/tui.rs` -> `run_app`, `InferenceEvent::Token`\n\
3. `VoiceManager::speak` pushes text into an internal sync channel. Background threads assemble \
   sentence chunks, synthesize via `TTSKoko::tts_raw_audio_streaming`, and append PCM into a `rodio::Sink`.\n\
   File refs: `src/ui/voice.rs` -> `VoiceManager::speak`, `TTSKoko::tts_raw_audio_streaming`\n\
4. When the turn ends, `run_app` calls `app.voice_manager.flush()` to drain any remaining audio.\n\
   File refs: `src/ui/tui.rs` -> `InferenceEvent::Done`; `src/ui/voice.rs` -> `VoiceManager::flush`\n\n\
Possible weak points\n\
- The Ctrl+T handler uses crossterm's `KeyCode::Char('t')` + `KeyModifiers::CONTROL` — not a string like 'Ctrl+T'. \
  Searching for 'Ctrl.*T' or 'toggle_voice' will find nothing; search for `KeyCode::Char.*'t'` instead.\n\
- Voice state is an AtomicBool inside `VoiceManager`; there is no `voice_enabled` field on `App`."
        .to_string()
}
