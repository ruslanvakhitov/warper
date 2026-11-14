# FAQ

## Do I need a Warp account?

No. Warper has no account system. There is no sign-in, no sign-up, no anonymous cloud user.

## Where does my data live?

Locally. See the **Local data** section of [README.md](README.md) for paths.

## Does Warper have a built-in AI agent?

No. Run a third-party CLI agent such as Codex, Claude Code, or Gemini CLI in the terminal. Configure models, credentials, tools, and skills in that agent.

## Is this an official Warp.dev build?

No. Warper is an independent hard fork. It uses Warp's open-source code under its existing licenses (AGPL v3 for most of the repo, MIT for `warpui_core` and `warpui`). It is not affiliated with, endorsed by, or sponsored by Warp.dev.

## Where do I report bugs?

[GitHub issues](https://github.com/ruslanvakhitov/warper/issues) on this repository.

If the bug also reproduces in upstream Warp, please file it there too — they have engineering resources we don't.

## Does Warper send anything over the network?

Warper has no model-provider integration, telemetry upload, hosted crash reporting, remote config fetch, Drive sync, or autoupdate ping. Programs you run in the terminal, including third-party CLI agents, make their own network requests.

## Can I use Codex, Claude Code, or Gemini CLI inside Warper?

Yes — just run them. Type `codex` (or `claude`, `gemini`, etc.) and Warper notices, adding a status indicator, optional rich input, and a finish notification. It doesn't wrap the agent CLI in its own UI; you keep using the agent the way its authors intended.

## Why is the binary named `warp-oss`?

Inherited from upstream. The bundle ships as `Warper.app` with bundle id `dev.warper.Warper`; the executable inside still uses the upstream name.

## Will the server, account system, or Drive ever come back?

No. They were removed deliberately. If you want those features, run upstream [Warp](https://www.warp.dev) — it has them, with the cloud product attached.

## Can I run Warper on Linux or Windows?

The bundling scripts target macOS, Linux, and Windows. macOS is the most exercised path; the other two compile but receive less attention. Issues against either are welcome.
