---
name: claude-code
description: Coding best practices for PixiCode. Load when writing or changing code, refactoring, fixing bugs, adding tests, or reviewing implementation. Covers small edits, tool use, style guide, and testing.
---

# Claude Code–style guidelines for PixiCode

Use this skill whenever you are editing code, implementing features, fixing bugs, or refactoring. It keeps changes focused and aligned with the repo.

## How to work

- **Prefer small, concrete steps.** One logical change per edit when possible. If a task is large, break it into clear steps and do them in order.
- **Read before editing.** Open the files you will change and understand context. Use search to find usages and related code.
- **Use parallel tools when applicable.** Run multiple independent reads or searches in one turn instead of one-by-one.
- **Follow project rules.** Respect the style guide and any `AGENTS.md` in the repo (e.g. naming, control flow, testing). Run from package dirs when running tests; do not run tests from repo root.
- **Prefer automation.** Execute requested actions without asking for confirmation unless something is missing or the action is unsafe/irreversible.
- **Avoid unnecessary abstractions.** Keep logic in one function unless it is clearly reusable. Prefer single-word names for locals and helpers; only use multi-word names when a single word would be unclear.

## Editing

- Make the minimal change that satisfies the request. Avoid refactors or style tweaks that are not required.
- Preserve existing style and formatting in the file (indentation, quotes, line length).
- After editing, fix any new linter/type errors in the touched code.

## Testing

- Prefer testing real behavior; avoid mocks when possible.
- Do not duplicate implementation logic inside tests.
- Run tests from the relevant package directory (e.g. `packages/pixicode`), not from the repo root.

## When unsure

- If the request is ambiguous or underspecified, ask one short, focused question or propose a single reasonable interpretation and proceed.
- If you lack permission or context (e.g. env vars, API keys), say what is missing and what you would do with it; do not invent secrets or config.

## Summary

Small steps, read first, parallel tools, follow AGENTS.md and style guide, minimal edits, run tests from package dirs, automate when safe.
