# Sign in with ChatGPT

A fifth AI provider, "ChatGPT (tu suscripción)", that uses the user's ChatGPT plan through OpenAI's official "Sign in with ChatGPT" flow for open-source, locally run apps (developers.openai.com/siwc, token-sharing-open-source). No API key.
Branch `feat/chatgpt-signin`. Runners: `cargo test --workspace`, `pnpm test`, `pnpm check`, `pnpm build`. High risk: credentials (Task Size item 2) → independent verification and native review.

## Specs

- **S1 — Official flow only.** Authorization with OAuth 2.0 + PKCE (S256) in the system browser at `https://auth.openai.com/api/accounts/authorize` with dynamic client registration as documented (first sign-in `client_id=dynamic_agent_client`, `agent_name_hint=PlexusTyping`, a persisted per-installation `ext_agent_host_id` (`urn:uuid:…`), `response_type=code`, `resource=https://api.openai.com/v1`, `state`, `nonce`, scopes `openid profile email offline_access resource.invoke chatgpt.tokens.use.direct`); redirect `http://127.0.0.1:<port>/auth/callback` (port 1455 by default, never `localhost`); the issued `client_id` (`oaiapp_…`) is kept and reused with `id_token_hint` on later sign-ins. Token exchange at `https://auth.openai.com/api/accounts/oauth/token` without client secret; the ID token is validated against the published JWKS (issuer, audience, nonce, expiry) and the grant must include `chatgpt.tokens.use.direct`. Never the Codex CLI client id, never `chatgpt.com/backend-api`. Exact parameters are taken from the current official docs; where they differ from this spec, the docs win and the Log records it.
- **S2 — Secrets.** Access token, refresh token, issued client id and host id live only in the system keychain (existing `secrets` module, same service), never in the SQLite store, logs, frontend or errors. Refresh tokens rotate: refreshes are serialized and the new pair replaces the old atomically. Sign-out revokes at the revocation endpoint (best effort) and deletes them. The callback listener binds only to 127.0.0.1, accepts one request with the matching `state`, and stops after it or after 5 minutes.
- **S3 — Inference.** Requests go to `POST https://api.openai.com/v1/responses` with `Authorization: Bearer <access_token>`, `store: false`, `stream: true` (server-sent events are read until completion), the prompt as `instructions`/user input as the docs require, no `temperature`, `max_output_tokens`, `metadata`, `user` or `previous_response_id`, and no system-role items. Models come from `GET https://api.openai.com/v1/models` filtered as the docs say. Usage-limit failures (`subscription_sharing_usage_limit_exceeded`, `subscription_sharing_usage_unavailable`) map to a Spanish notice "Has alcanzado el límite de tu plan de ChatGPT". An expired access token is refreshed once and the request retried.
- **S4 — Provider in the chain.** `ProviderId::Chatgpt` ("ChatGPT (tu suscripción)") joins the fallback chain like the other providers (validation, retry, logging, timeouts). It needs a signed-in account instead of a key; without one the chain skips it with the notice "Inicia sesión en ChatGPT".
- **S5 — UI.** In Ajustes → IA, the ChatGPT provider shows "Iniciar sesión con ChatGPT" (opens the browser, waits, then shows the account email) or "Cerrar sesión", and its model selector; errors are shown in Spanish. Endpoint allowlist (`Endpoints::production`) gains only `auth.openai.com` and keeps `api.openai.com`.
- **S6 — Docs.** README: how to sign in, that usage counts against the ChatGPT plan, that it is OpenAI's preview programme for open-source apps, and how to sign out. THIRD_PARTY/privacy note updated.

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1, S2 | OAuth flow + PKCE + callback listener + token exchange/refresh/revoke + JWKS validation + keychain storage (Rust, mocked endpoints in tests) | worker | pending | |
| T2 | S3, S4 | ChatGPT provider: Responses streaming request/parse, model list, refresh-and-retry, chain integration | worker (after T1) | pending | |
| T3 | S5, S6 | Ajustes → IA sign-in/out UI, commands, README | worker (after T2) | pending | |
| T4 | S1–S6 | Independent verification + native review | gentle-ai-verify + review | pending | |

## Log

- **L1 (user, verbatim, 2026-11-17):** "tambien con openai podemos añadir login con oauth, que creo que esta permitido, para cosas como esta de codigo y demas, como en opencode o pi".
- **L2 (research, 2026-11-17):** OpenAI's official route since 2026-09-29: "These docs explain ChatGPT plan usage for open-source and locally hosted apps" (developers.openai.com/siwc/token-sharing-open-source); Pi and OpenCode are listed partners; the old Codex client id + `chatgpt.com/backend-api` is legacy and not approved. Anthropic and Google do not allow subscription use from third-party apps.
- **L3 (user, verbatim, 2026-11-18):** "continua" then "haz lo que consideres mas optimo" — the agent picks this block next.
