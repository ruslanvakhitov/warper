# WARPER-015: remove built-in AI

## Summary

Warper is a terminal with support for third-party CLI agents. Remove Warper's built-in AI agent and every feature, service, setting, command, and dependency that exists to support it.

This spec supersedes the built-in AI requirements in earlier Warper specs, including WARPER-001 and WARPER-005. There is no requirement to preserve built-in agent functionality or compatibility with its saved data.

## Behavior

### Terminal and third-party CLI agents

1. Warper opens terminal sessions and executes the commands the user enters. Terminal input never switches into a Warper-owned agent or sends a prompt to a model provider.
2. Terminal blocks, command history, completions, shell integration, tabs, panes, themes, keybindings, SSH, launch configurations, tab configurations, and Git worktrees remain available.
3. Users can run Codex, Claude Code, Gemini CLI, and other programs directly in the terminal. Existing third-party CLI-agent detection, status, finish notifications, toolbar configuration, plugin integration, and rich input remain available independently of any Warper AI setting or API key.
4. Third-party rich input sends the user's input to the running CLI agent. It does not invoke a Warper-owned model, agent executor, approval system, or orchestration service.
5. Manual file browsing, editing, Git operations, and diff inspection remain available. Their built-in AI generation, review, and agent-context actions are removed.
6. Local workflows remain available as user-invoked terminal commands. Built-in AI prompts, agent workflows, and agent-generated workflow suggestions are removed.

### Removed product surfaces

7. There is no built-in Agent Mode, agent pane, chat interface, conversation list, agent task runner, child-agent orchestration, planning mode, or built-in tool execution.
8. New-session pickers offer terminal sessions and supported third-party CLI agents. They do not offer a Warper Agent session.
9. Menus, command palette entries, shortcuts, context menus, toolbars, input placeholders, empty states, and settings do not advertise or invoke built-in AI.
10. Built-in command generation, command and error explanation, output summarization, AI suggestions, natural-language routing, AI-generated titles and Git text, and automatic AI review are removed.
11. Built-in agent profiles, model and provider selectors, OpenRouter configuration, API-key controls, autonomy and approval settings, agent usage displays, and built-in agent tips are removed.
12. Warper's built-in MCP client, MCP server management, skills catalog, agent rules and memory, repository outlines, and AI codebase indexing are removed. Third-party CLI agents manage their own tools, skills, rules, credentials, and providers.
13. Warper does not scan repositories or watch files to build context for its removed agent. Ordinary terminal, Git, editor, file-search, and third-party CLI-agent integrations retain their required filesystem access.
14. Removed agent, MCP, model, and provider CLI commands are absent from help and fail at argument parsing when invoked. They do not silently succeed or start a terminal as a substitute.
15. Environment variables such as `OPENROUTER_API_KEY` and `OPENROUTER_MODEL` do not enable built-in AI or cause Warper to contact a model provider.

### Runtime and saved data

16. Starting Warper or opening a terminal does not initialize built-in AI services, restore agent conversations, start Warper-managed MCP servers, load model credentials, discover built-in agent skills, or build AI indexes.
17. Built-in agent implementation and dependencies are deleted. A hidden setting, feature flag, disabled menu, no-op service, or permanent compatibility stub does not satisfy removal.
18. Shared functionality needed by the terminal or third-party CLI agents remains under the ownership of those retained features. Its presence must not keep a built-in agent runtime or provider integration alive.
19. There is no requirement to restore, import, migrate, display, export, or recover old built-in agent conversations, agent panes, plans, profiles, memory, MCP installations, indexes, or credentials. They may be discarded. Do not build a compatibility workflow for them.
20. Ordinary terminal state and user-authored terminal configuration remain supported. Removed agent variants in user-authored configuration are unsupported and produce a clear configuration error.
21. Removing built-in AI does not delete third-party CLI-agent configuration, repository instruction files, user projects, terminal command history, or unrelated user settings.
22. Product documentation and packaging describe Warper as a terminal with third-party CLI-agent integration. They do not promise a built-in AI helper or OpenRouter support. Historical upstream research and superseded specs may remain as historical records.
