# 017 launcher sign-in: login screen

The login screen signs you in through the Cimmeria launcher. When you pressed
Play after signing in, the screen shows who you are signed in as and an
**Enter** button instead of the name and password boxes. **Use password**
brings the boxes back for this session so you can type a password instead.
Without a launcher sign-in the screen behaves like the stock screen.

The patch replaces two stock files with whole files:

- `Working/SGWGame/Content/UI/Startup/Login/Login.lua`
- `Working/SGWGame/Content/UI/Startup/Login/Login.layout`

Published patches are append-only: a change after publishing needs a new id.

## Base files

Both overlay files start from the stock QA client files, copied byte for byte
(see the first commit that added them).

| File | Size (bytes) | Date (stock) | SHA-256 of the stock original |
|---|---|---|---|
| `Login.lua` | 7183 | 2026-03-20 18:30:17 | `AFB2CCC0C449550FFDEC3714BAA7E37D6F614C70D11CCB8BC3EB32A8A01CD605` |
| `Login.layout` | 6544 | 2009-06-30 11:57:04 | `9AAEF6DDF133965C47BB724916CE691A827A847C1385BDC352321C3AB82D1B11` |

The overlay `Login.lua` is 307 lines and `Login.layout` is 149 lines. Both are
ASCII with CRLF line endings and are stored `-text`.

## Behaviour

- Ticket mode is on when the launcher wrote `CimmeriaLogin` for this launch
  with `version = 1` and `mode = "ticket"`. Then the screen hides the name,
  password, remember and server boxes and shows `Signed in as <display>
  (<account>)`. The login button reads **Enter**.
- **Enter** sends the launcher's ticket. The status reads `Connecting...`
  first. The account name is never saved.
- With `auto = true` the screen submits the ticket once when it opens.
- A failed sign-in turns the button into **Retry**. The status shows the
  launcher's text for the error when it sent one, else `Login failed (<n>).
  Keep the launcher open, then press Retry.`
- **Use password** leaves ticket mode for this session, shows the stock boxes
  (the server combo only when there is more than one server), clears the boxes
  and resets the button to **Login**.
- The website, account and forums buttons open the launcher's links
  (`links.website`, `links.account`, `links.forums`). A button with no link is
  hidden. Without `CimmeriaLogin` they open the stock URLs.
- Without `CimmeriaLogin`, or with another version or mode, the file does
  nothing beyond the stock screen.

## `CimmeriaLogin` (version 1)

The launcher writes `LoginInternal.lua` for each launch. It is rewritten when
the game exits. The fields this overlay reads:

| Field | Type | Meaning |
|---|---|---|
| `version` | number | Must be `1`. |
| `mode` | string | Must be `"ticket"`. |
| `server` | string | Key into `LoginMod.servers`. |
| `account` | string | Account name shown and sent with the ticket. |
| `ticket` | string | Sign-in ticket sent to the server. |
| `display` | string | Name shown in the status text. |
| `auto` | boolean | Submit the ticket when the screen opens. |
| `links` | table | Optional `website`, `account` and `forums` URLs (`https://`). |
| `errors` | table | Optional text per error number. |

The launcher sanitizes every string: printable ASCII only, no `"` and no `\`.

## Running the UAT

The logic UAT for this overlay is in `test/` (added with the UAT packet). On
Windows, install `lupa` once and run it from the repository root:

```text
python -m pip install lupa
python data/client-patches/017-launcher-sign-in/test/run_lupa.py
```
