# Execution Plans

Use this directory as the long-lived system of record for larger multi-step work.

- `active/` holds the current execution plan Hematite is driving.
- `completed/` holds archived plans with their final walkthrough notes.
- `tech-debt-tracker.md` captures unfinished cleanup, refactors, and follow-up work that should survive beyond a single session.

`.hematite/PLAN.md` remains the fast local handoff. Hematite mirrors meaningful plans here so intent stays visible across sessions, worktrees, and reviews.
