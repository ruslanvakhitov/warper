# WARPER-001 OpenRouter Manual Verification

This credentialed check is outside the credential-free offline smoke. It proves
that the retained OpenRouter-owned AI flow still works and that the Warper
process does not contact hosted Warp/Oz infrastructure while that flow runs.

Run the helper for the current checklist:

```bash
script/warper_openrouter_manual_verify --print-steps
```

## Required Evidence

- A real OpenRouter API key configured in Warper through Settings > OpenRouter,
  or supplied to a terminal-launched dev build with `OPENROUTER_API_KEY`.
- A clean app log from the same run, normally `~/Library/Logs/warper.log`.
- A process-scoped observed-host list from a host-aware network monitor, one
  hostname per line, filtered to the Warper process during the manual agent
  prompt.

The observed-host list must contain `openrouter.ai` or a subdomain, and must not
contain any nonlocal host outside OpenRouter. Loopback entries are allowed.
Known forbidden endpoints include Warp/Oz, Firebase, RudderStack/Segment,
Sentry, and hosted update/download domains.

Validate the artifacts with:

```bash
script/warper_openrouter_manual_verify --check \
  --observed-hosts /tmp/warper-openrouter-hosts.txt
```

The check requires the app log to contain `OpenRouter request starting` and a
2xx `OpenRouter response received` entry. It fails if the log or observed-host
list contains hosted Warp/Oz endpoints or if the observed-host list includes a
non-OpenRouter host.

## Credential Handling

Do not record placeholder credentials as passing evidence. If a real
OpenRouter key is unavailable, leave the verification pending and keep the
helper output as the encoded path to run later.

For a credential-only preflight before launching Warper:

```bash
OPENROUTER_API_KEY='real key' \
script/warper_openrouter_manual_verify --probe-credential
```

The probe reaches OpenRouter directly with `curl`; it does not validate the
Warper app flow and is not a substitute for the manual app run.
