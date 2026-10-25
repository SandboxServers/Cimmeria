# Signing in with the launcher

> **Type:** How-to
> **Audience:** Players
> **Last updated:** 2026-10-25
> **Companions:** [Launcher guide](launcher-guide.md), [How sign-in works](../technical/launcher-sign-in.md), [macOS notes](../guides/macos.md)

Some servers let you sign in through the launcher with an outside account such as Discord instead of a game password. If your server does, the launcher shows a **Sign in** panel. If it does not, nothing changes for you.

This feature is new and has not been tried against the live game yet. The steps below describe what the launcher does; expect rough edges.

## Sign in

1. Press **Sign in with** your provider (for example "Sign in with Discord"). Your browser opens.
2. Finish signing in there, then come back to the launcher. You have 5 minutes. If the browser did not open, the launcher shows an address to copy into one.
3. The panel changes to **Your account**. Press **Play**.

The first time, the launcher may say **Link your account**. That means no game account is tied to this sign-in yet. You can:

- **Create a new account.** Only offered while the server accepts new accounts.
- **Link an existing account.** Type the account name and password and press **Link**. Read the warning: **linking removes that account's password for good.** After that, the account can only be played through this launcher. Press **Remove password and link** to go ahead, or **Keep password** to stop.
- **Link with a code** if you have no password (or lost it). Ask the server staff for a link code. The same warning applies. Type the code and press **Link with a code**, then confirm the same way.

Accounts you link this way never have a password again. The old game login screen will not work for them.

## Add or remove a sign-in method

Under **Your account** you can add another provider ("Add a sign-in method") or remove one. Both ask you to sign in again first if your last sign-in was more than 10 minutes ago. You cannot remove your last method. If you lose access to every method, ask the server staff for a link code.

## Sign out

Press **Sign out**. The launcher forgets your sign-in on this computer.

## While the game runs

The launcher stays open and passes your sign-in to the game. If you close it, it asks **Game still connected**. **Keep running** leaves it open. **Close anyway** closes it, and the game can no longer change server or log in again until you press Play next time.

## When Play stops

- **"Play stopped: could not get a sign-in for this launch."** Check your connection and press Play again. If it keeps failing, sign out and sign in again.
- **"Play stopped: your sign-in has ended."** Sign in again, then press Play.
- **"Your sign-in for this launch is no longer valid"** inside the game: close the game and press Play in the launcher (this also happens if the server restarted).

The launcher never falls back to asking for a password when you are signed in.
