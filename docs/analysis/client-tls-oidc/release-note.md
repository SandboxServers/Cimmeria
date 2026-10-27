# client-tls-oidc: release note (draft)

> **Type:** Reference
> **Audience:** Whoever writes the launcher release text and the colo announcement
> **Last updated:** 2026-10-26
> **Companions:** [Ledger](README.md), [Contract](contract.md), [Launcher sign-in](../../technical/launcher-sign-in.md), [Player guide](../../client/launcher-sign-in-guide.md), [Auth over HTTPS](../../operations/auth-https.md)

## Text to ship

**Sign in with Discord or another provider from the launcher.** On a server that offers it, the launcher shows a **Sign in** panel. You sign in in your browser, link or create a game account once, and press Play. The launcher hands the game a one-time ticket, so the game never sees a password.

- Signing in is optional. If you do not sign in, or the server does not offer it, Play works as before with your game account and password.
- Linking an existing account removes its password for good. The launcher asks you to confirm first.
- Once you are signed in, Play never falls back to a password. If it cannot get a ticket, it stops and says why.
- Server operators: provider apps and the proxy gateway need setting before the first deploy ([Auth over HTTPS](../../operations/auth-https.md)).

## What this note does not say, and why

The request for this note asked it to say that "OIDC is the default sign-in method and Basic remains selectable". The code and the spec do not support that wording, so it is not in the text above:

- **Sign-in is opt-in, not the default.** `plan()` in `storage/launch/sign_in_play.rs` gives a player with no sign-in the relay-password path, and gives the unchanged server list when the server offers no relay. No setting makes provider sign-in the default for a player who has not signed in.
- **The launcher has no "Basic" choice.** The only "basic" in the design is the OIDC `token_endpoint_auth_method`: `client_secret_basic` is the default and `client_secret_post` is selectable per provider by the operator ([contract C18.3](contract.md)). That is a server setting, not something a player picks.

If the owner meant "OIDC providers send the client secret as `client_secret_basic` by default, and `client_secret_post` stays selectable", that is true and is documented in [Launcher sign-in](../../technical/launcher-sign-in.md). Confirm the intended wording before publishing.
