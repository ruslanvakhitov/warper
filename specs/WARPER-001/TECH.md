# WARPER-001: hard network amputation - Tech spec

Companion to `PRODUCT.md` in this directory; refer there for user-visible behavior.

## Context

- The roadmap source is `/Users/p2p/GitClones/Notes/Current/Hobbies&Crafts/Pet Projects/warper/2026-04-30-warper-airgapped-roadmap.md`, section `## 0: hard network amputation`.
- `crates/warp_core/src/channel/config.rs:7` defines `ChannelConfig` with server, Oz, telemetry, autoupdate, crash reporting, and MCP static config. `WarpServerConfig::production()` hard-codes `https://app.warp.dev`, `wss://rtc.app.warp.dev/graphql/v2`, `wss://sessions.app.warp.dev`, and a Firebase auth API key at `crates/warp_core/src/channel/config.rs:43`. `OzConfig::production()` hard-codes `https://oz.warp.dev` at `crates/warp_core/src/channel/config.rs:65`.
- `crates/warp_core/src/channel/state.rs:37` initializes the OSS Warper channel but still installs `WarpServerConfig::production()` and `OzConfig::production()`. URL override methods at `crates/warp_core/src/channel/state.rs:85` assume present, parseable URLs.
- `app/src/lib.rs:1010` extracts API keys from launch options, `app/src/lib.rs:1022` initializes `AuthState`, `app/src/lib.rs:1033` registers `ServerApiProvider`, and `app/src/lib.rs:1042` registers `AuthManager`. This makes auth and server clients part of the core startup graph.
- `app/src/lib.rs:1155` initializes cached server experiments, `app/src/lib.rs:1160` registers `AIRequestUsageModel`, and `app/src/lib.rs:1162` registers `UserWorkspaces` from server team/workspace clients.
- `app/src/lib.rs:1183` initializes crash reporting when the feature is compiled, `app/src/lib.rs:1197` checks and reports autoupdate errors, and `app/src/lib.rs:1205` performs autoupdate cleanup when `FeatureFlag::Autoupdate` is enabled.
- `app/src/lib.rs:1526` registers `CloudModel`, `app/src/lib.rs:1549` registers a cloud-object `SyncQueue` backed by `ServerApiProvider`, `app/src/lib.rs:1593` registers `TeamUpdateManager`, `app/src/lib.rs:1601` registers `UpdateManager`, and `app/src/lib.rs:1610` registers the cloud preferences syncer.
- `app/src/server/server_api.rs:1181` contains server calls such as `GET /current_time`; `app/src/server/server_api.rs:1213` fetches hosted client versions; `app/src/server/server_api.rs:1271` constructs the provider and HTTP clients used by many subsystems.
- `app/src/auth/auth_state.rs:76` initializes auth from test state, launch API key, `WARP_USER_SECRET`, or persisted secure storage. `app/src/auth/auth_state.rs:140` persists Firebase-backed users and removes logged-out persisted state.
- `app/src/server/cloud_objects/update_manager.rs:198` describes `UpdateManager` as the bridge between SQLite, `CloudModel`, and `SyncQueue`; `app/src/server/cloud_objects/update_manager.rs:688` polls the object client for changed objects when online and logged in.
- `app/src/terminal/shared_session/sharer/network.rs:580` creates shared-session WebSockets through `connect_endpoint("/sessions/create")`, obtains auth through `ServerApiProvider`, and sends telemetry context. `app/src/terminal/shared_session/sharer/network.rs:673` reconnects to session-sharing endpoints with retries.
- `app/src/server/telemetry/mod.rs:91` flushes queued telemetry to RudderStack for release bundles or sandbox telemetry, `app/src/server/telemetry/mod.rs:116` flushes persisted telemetry to RudderStack, and `app/src/server/telemetry/mod.rs:143` persists queued events for later upload.
- `app/src/ai/agent/api/impl.rs:26` contains the OSS third-party provider path for multi-agent output, while non-OSS paths build server API requests with cloud task metadata. `app/src/ai/request_usage_model.rs:226` fetches AI request usage from the server and `app/src/ai/request_usage_model.rs:374` encodes billing, overage, bonus credit, PAYG, and BYOK availability.

## Proposed changes

### 1. Make hosted services unrepresentable by default

- Replace mandatory `WarpServerConfig` and `OzConfig` fields in `ChannelConfig` with optional, typed service configs. The Warper OSS channel should initialize with no server config, no Oz config, no telemetry upload config, no autoupdate config, no crash upload config, and no Firebase key.
- Remove `WarpServerConfig::production()` and `OzConfig::production()` from the Warper default path. Production Warp endpoint constructors can remain only behind code that is not used by Warper, or be deleted if no retained build target needs them.
- Change URL accessors so callers must handle `None`. Do not use empty strings, localhost placeholders, dummy domains, or malformed URLs to represent removed services.
- Delete launch and environment auth entrypoints that only exist to authenticate against Warp services: API-key launch login, `WARP_USER_SECRET`, Firebase persisted-user restoration, auth refresh, and reauth prompts.

### 2. Remove server-shaped startup dependencies

- Split `initialize_app` so local startup registers only local models needed for terminal operation before any optional network subsystem. The retained base graph should include local persistence, settings, terminal, pane groups, editor support, themes, keybindings, command history, launch configs, local logs, and local workflows.
- Remove `ServerApiProvider`, `AuthManager`, hosted `AuthStateProvider` behavior, `ServerExperiments`, server-backed `AIRequestUsageModel`, `UserWorkspaces` team/workspace clients, cloud `SyncQueue`, `TeamUpdateManager`, `UpdateManager`, and cloud preferences syncer from the Warper startup path.
- Prefer deleting callers that only consume cloud/server state. Where a retained local feature currently depends on a server-shaped type, introduce a local service with a local name and narrow API instead of a broad `NoopServerApi`.
- Treat logged-out/no-auth as the only normal state. Any retained code that checks login should either be deleted with its cloud feature or rewritten so terminal-local functionality does not depend on auth.

### 3. Delete cloud, account, billing, team, and sharing surfaces

- Remove auth UI modules and commands: login, signup, SSO, paste auth token, auth handoff, account settings, reauth, anonymous cloud user, and logout surfaces.
- Remove billing and quota UI paths: pricing, subscriptions, credits, usage limit modals, upgrade links, PAYG checks, team plan checks, and quota banners.
- Remove Warp Drive/cloud object creation, loading, sharing, shared-with-me, trash sync, cloud folders, ACLs, and cloud object polling. Retain only local data models that are necessary for local terminal workflows, renamed away from cloud concepts where practical.
- Remove team/workspace-as-organization surfaces: members, guests, roles, enterprise controls, permission flows, and audit-style flows.
- Remove shared-session and RTC entrypoints and background reconnect logic, including deep links, keybindings, commands, restore paths, and permission UI that can initiate them.

### 4. Remove hosted Warp/Oz agent and server AI paths

- Delete Oz/cloud agents, ambient agents, OpenWarp launch, cloud capacity, Codex/Oz modal, server task ID, hosted child-agent orchestration, hosted task polling, and hosted task status sync paths from Warper.
- Leave explicitly selected terminal-AI provider integrations outside WARPER-001 when they are not Warp/Oz/cloud infrastructure. They should not be contacted during baseline startup or normal terminal use unless the user invokes/configures the AI feature that owns that provider.
- Delete server-backed request usage, billing credit, bonus grant, overage, quota, refund, and hosted feedback paths.

### 5. Remove upload and update clients

- Delete RudderStack upload and UGC telemetry upload paths. Local diagnostic logs may remain, but queued telemetry should not be persisted for later network upload.
- Delete Sentry initialization and crash upload. Keep only local crash diagnostics that never upload.
- Delete hosted autoupdate checks, hosted version fetches, hosted release bundle download paths, update error reporting, and hosted update menu items.
- Remove server-backed experiment fetches and flags whose only purpose is hosted rollout control. Retain feature constants only when they are local compile-time or local runtime switches for retained local code.

### 6. Add guardrails

- Add static denylist checks for removed endpoint literals and service names in app/runtime crates: `app.warp.dev`, `rtc.app.warp.dev`, `sessions.app.warp.dev`, `oz.warp.dev`, Firebase auth API keys/domains, RudderStack hosts, Sentry DSNs, and hosted update URLs.
- Keep static denylist scopes focused on runtime surfaces and removed user entrypoints while excluding specs, scripts, test files, generated schema/generated files, and OpenRouter implementation files. OpenRouter remains out of scope for WARPER-001.
- Include stale restore/deeplink guardrails for pane-group and URI roots that could resurrect anonymous signup, Warp Drive object panes, shared-session state, billing/team/platform/settings deeplinks, or hosted agent panes.
- Add startup tests that run with hosted configs absent and fail on parse attempts, retries, or background tasks for removed services.
- Add an offline smoke path that launches the app or a minimal app harness with outbound networking blocked or intercepted and verifies local terminal creation still succeeds.
- Add UI absence tests for account, billing, Drive, sharing, team, Oz/cloud agent, hosted update, and telemetry upload entrypoints.

## Testing and validation

- Product invariants 1, 2, 19: run the app from a fresh local profile with outbound networking blocked; create terminal sessions, split panes, modify local settings/themes/keybindings, use launch configs, and run local workflows.
- Product invariants 3, 4, 16: add unit/integration tests for absent service configs and a startup log/network-intercept smoke test that fails on DNS/connect attempts or retry logs for removed services.
- Product invariants 5-11, 14, 18: add UI/integration coverage or command registry tests proving removed commands, keybindings, panes, deep links, settings pages, menus, modals, and restore paths do not expose account, billing, Drive, team, sharing, Oz/cloud agent, session-sharing, or hosted update surfaces.
- Product invariants 12, 13: add static and runtime checks proving telemetry upload and Sentry initialization are absent while local logs/crash files, if retained, do not contain upload destinations or background send tasks.
- Product invariant 15: add tests around feature flag and experiment initialization to prove server-backed experiment state is not loaded or fetched, and retained flags are local-only.
- Product invariant 17: create migration tests with persisted auth/cloud/team/billing/sharing/update state from the old schema and verify app startup ignores or removes it locally without network access.
- Product invariant 20: include the denylist and offline smoke in the presubmit/toolchain path so regressions are caught without private credentials or hosted service availability. The local smoke command is `./script/warper_offline_local_smoke`; it runs `./script/check_warper_static_denylist` for `startup`, `entrypoints`, `telemetry-crash-update`, and `all-runtime`, focused `warp_core` channel tests, and `cargo check -p warp --bin warp-oss`. The local smoke must print the exact validations it performs and the explicit gaps it does not cover: no macOS `.app` launch, no OS-level outbound network block, no live UI terminal/pane/settings exercise, no private credentials, no hosted Warp/Oz availability, and no OpenRouter behavior validation.
- Before implementation PR: run `./script/presubmit` plus `./script/warper_offline_local_smoke`. For a faster static-only check while iterating, run `./script/check_warper_static_denylist all-runtime` and `./script/check_warper_static_denylist entrypoints`.

## Risks and mitigations

- Cloud concepts are deeply embedded in startup and persistence. Mitigate by deleting feature clusters in vertical slices and keeping each slice buildable: config/auth, cloud objects, UI surfaces, telemetry/update, hosted agents, then guardrails.
- Some local features may currently depend on cloud-named models for storage or menu state. Mitigate by extracting narrow local services only for retained behavior, with names that do not imply server availability.
- Persisted user data may contain old account/cloud state. Mitigate with explicit migration or ignore paths that never call network services.
- Third-party crates can initiate network indirectly through update, telemetry, auth, or hosted Warp/Oz AI paths. Mitigate with runtime network interception during smoke tests and static endpoint denylist checks for removed services.

## Parallelization

- Config/auth/server graph: channel config, auth initialization, `ServerApiProvider`, server experiments, URL accessors.
- Cloud and sync graph: `CloudModel`, `SyncQueue`, `UpdateManager`, `TeamUpdateManager`, Drive/cloud object surfaces, cloud preferences sync.
- UI amputation: account, billing, team, sharing, Drive, hosted update, telemetry, and Oz/cloud agent entrypoints.
- Telemetry/crash/update: RudderStack, Sentry, telemetry persistence-for-upload, autoupdate/version fetches.
- Tests and guardrails: endpoint denylist, absent-config tests, offline smoke, UI absence coverage, migration fixtures.
