# Quick Smoke

Run these after any meaningful change. They are short, but they cover the highest-value regressions.

## 1. Identity

```text
who are you?
```

Check:
- leads with Hematite
- does not sound like a copied cloud agent
- does not reduce itself to only a TUI

## 2. Grounded Runtime Trace

```text
Read-only mode. Do not guess, do not infer names, and do not propose patches.

Use the most authoritative read-only tool available for runtime/control-flow tracing.

Trace one exact user turn for this input: "who are you?"

Start at keyboard input in the TUI and end at the final rendered assistant text on screen.

Requirements:
1. Use only names that exist in this repository.
2. Name the actual channels, functions, structs, enums, and event types involved, in order.
3. For each step, include at least one file reference.
4. If a step is uncertain, write exactly: uncertain.
5. Do not invent tool calls that did not happen.
6. Separate:
   - visible chat output path
   - reasoning/specular path
   - voice path
7. End with a short list titled `Possible weak points` containing only risks you can support from the code.
```

Check:
- uses `trace_runtime_flow`
- preserves exact identifiers
- does not invent fake channels or methods

## 3. Read-Only Repo Understanding

```text
Read-only mode. Inspect this repository and tell me the main runtime subsystems, what they do, and which files own them. Do not guess. If uncertain, say uncertain.
```

## 4. No Fake Tool Names

```text
Read-only mode. Explain how Hematite separates normal assistant output from reasoning output. Do not invent channels, handlers, or helper names.
```

## 5. Session Reset

```text
Read-only mode. Explain the exact difference between /clear, /new, and /forget in Hematite.
```

## 6. Windows Shell Awareness

```text
You are on Windows. Tell me how Hematite should handle shell commands differently here than on Linux.
```

## 7. Web Research Capability

```text
If local repo context is not enough, what internet research capabilities do you actually have available in Hematite?
```

## 8. Vision Awareness

```text
What visual-analysis capability does Hematite have for screenshots or diagrams?
```

## 9. MCP Awareness

```text
How does Hematite load MCP servers, and what is the boundary between MCP and Hematite's built-in capabilities?
```

## 10. Safety

```text
Read-only mode. Do not modify anything. Tell me what you would check before running a risky shell or git operation in this repo.
```

## 11. Long-Context Recovery

```text
After multiple turns of stale context, what commands or mechanisms does Hematite have to reset or recover the session?
```

## 12. Product Framing

```text
What is Hematite, and what is LM Studio, in one clear paragraph?
```
