# WARPER-001: hard network amputation

## Summary

Warper must launch and remain useful as a local terminal with no implicit contact with Warp, Oz, Firebase, telemetry, update, session-sharing, or other Warp-hosted infrastructure. The normal state is local-only, logged out, and unauthenticated; terminal use must not depend on account, cloud, billing, sharing, hosted agent, telemetry, or update services.

## Goals / Non-goals

- Goal: make local terminal use work with outbound networking blocked.
- Goal: remove Warp-hosted service entrypoints instead of hiding them behind retrying failures.
- Goal: keep local terminal sessions, panes, local settings, themes, keybindings, launch configs, local logs, and local workflows.
- Non-goal: add offline AI, local LLM support, sync replacement services, or migration to a new hosted backend.
- Non-goal: preserve compatibility with Warp account, Drive, team, sharing, billing, Oz, telemetry, crash upload, or autoupdate flows.
- Non-goal: remove explicitly selected terminal-AI provider integrations that are not Warp/Oz/cloud infrastructure.

## Behavior

1. On a fresh install, opening Warper lands in a usable terminal window without requiring login, signup, SSO, account linking, anonymous cloud user creation, onboarding auth handoff, or any Warp-hosted service setup.
2. Blocking all outbound network traffic before launch does not prevent the app from opening, creating terminal sessions, splitting panes, editing local settings, changing themes, using keybindings, using launch configs, or running local workflows.
3. During normal terminal use, Warper does not attempt to contact `warp.dev`, Oz, Firebase, RudderStack, Sentry, update endpoints, session-sharing endpoints, RTC endpoints, or any equivalent Warp-hosted service.
4. Logs produced during startup and normal terminal use contain no retry loops, malformed URL errors, auth refresh errors, quota fetch errors, cloud sync errors, update check errors, telemetry upload errors, crash upload errors, shared-session connection errors, or agent task polling errors for removed services.
5. Account entrypoints are absent. The user cannot open login, signup, account settings, SSO, paste auth token, auth handoff, reauth, API-key login, anonymous cloud user, or logout UI.
6. Billing and plan entrypoints are absent. The user cannot open pricing, subscription, credits, usage limits, quota modals, upgrade links, pay-as-you-go controls, team plan checks, or billing-related banners.
7. Warp Drive and cloud object entrypoints are absent. The user cannot create, load, share, sync, trash, restore, or browse cloud-backed Drive objects, cloud folders, shared-with-me content, or remote object histories.
8. Team and organization entrypoints are absent. The user cannot create or switch cloud workspaces, manage teams, members, guests, roles, ACLs, enterprise permissions, or audit-style permission flows.
9. Settings are local-only. Preferences, themes, keybindings, launch configs, and other retained user configuration are read from and written to local storage only, with no cloud preference objects or settings sync.
10. Session sharing is absent. The user cannot create shared sessions, join shared sessions, remotely control a session, use presence, assign shared-session roles, or trigger RTC/session-sharing permission flows.
11. Hosted agent entrypoints are absent. Oz/cloud agents, ambient agents, cloud capacity modals, Oz modals, cloud task IDs, hosted child-agent orchestration, and server-polled task state are not reachable from the UI or normal app flows.
12. No telemetry or usage events are uploaded. UGC telemetry, RudderStack sends, hosted analytics queues, and telemetry retry/persist-for-upload behavior are removed. Local diagnostic logging to disk may remain when it never uploads.
13. Crash diagnostics are local-only. The app may write local crash information for the user or developer to inspect, but it does not initialize Sentry, upload crash reports, or send captured errors to a hosted crash service.
14. Warp-hosted autoupdate is absent. The app does not check hosted release/version endpoints, show hosted update menu items, download hosted release bundles, or report update errors to any service.
15. Server-backed experiments and hosted rollout controls are absent. Feature switches that remain are local compile-time or local runtime constants used to keep retained local code configurable, not mechanisms for server-driven rollout.
16. Warp/Oz/cloud service URLs are absent by default. A missing removed service is represented as unavailable, not as an empty string, placeholder URL, malformed URL, production URL, or value that later code tries to parse or dial.
17. Existing persisted account, auth, cloud object, team, billing, sharing, telemetry, and update state from a previous Warp-derived install is ignored or removed locally without contacting a network service.
18. Removed surfaces fail closed. Deep links, commands, keybindings, restored panes, stale persisted state, or feature flags that previously opened removed cloud/account/billing/sharing/agent surfaces do not resurrect those surfaces or start network activity.
19. Retained local data remains usable after the amputation: terminal session restoration where local-only, local command history, local settings, themes, keybindings, launch configs, local logs, and local workflows continue to behave as local features.
20. Developer-facing build and test workflows can verify the local-only invariant without private credentials, private registries, Warp-hosted service access, or best-effort fallbacks.
