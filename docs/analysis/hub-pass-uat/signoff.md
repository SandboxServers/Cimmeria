# Hub pass UAT: owner sign-off sheet

> Type: checklist. Audience: the owner. Source: the results table in the [README](README.md), 2026-10-13. These are the checks only a person can make: the lab reads state, not colour or look.
> The screenshots are not in the repo. They sit in the lab's local screenshots folder (`%LOCALAPPDATA%\cimmeria-lab\screenshots\`) on the workstation that ran the pass. A file name marked "nearest" is the capture closest in time to the server event (the folder clock is UTC-5, the server rows are UTC), not a name the verifier recorded.

Fill the last column with PASS or FAIL (and a line if it fails). The server half of each row already passed or is tracked in the README.

| # | Feature row | What to look at | Screenshot | Looker or verifier note | Owner verdict |
|---|---|---|---|---|---|
| 1 | Mail 1 | New-mail notice: a "New" marker on the inbox entry with the mailbox closed and open | `lab_screenshot-20261013-024508.667.png` (nearest) | Looker: inbox open, mail listed, no New marker visible. Server `mail.notified` fired | |
| 2 | Mail 2 | Cash mail shows a coin icon and 100 naquadah on the entry | `lab_screenshot-20261013-024514.733.png` (nearest) | Looker saw an "Att:" label, not a coin icon | |
| 3 | Mail 3a | Compose window: no item icon in the attachment slot and a red refusal text after Send | `lab_screenshot-20261013-025322.643.png` (nearest) | Looker: compose open, no icon in the slot, no red text. Server refused correctly | |
| 4 | Chat 9 | Welcome line "Welcome to Stargate Worlds..." in sky blue on the Info tab, no popup | `lab_screenshot-20261013-025634.858.png` | Looker saw white and yellow lines, none sky blue; docs say sky blue | |
| 5 | Chat 9a | After the #1678 fix: `/emote` shows once. Today it shows twice (client echo plus server echo) | `lab_screenshot-20261013-025654.372.png` (nearest) | Looker: two "Hubmail HPUwaves" lines in three colours | |
| 6 | Chat 10 | `.announce` and `.location` lines in the colour the docs promise (sky blue Info tab) | `lab_screenshot-20261013-072107.714.png` | Verifier: Server lines red, the `.location` error line white-grey, none sky blue | |
| 7 | Pets U1 | The pet bar opens after a summon (#1674) and the pet model is the Straegis fighter | `lab_screenshot-20261013-025109.415.png` | Looker: no pet panel; the dark spiky mesh in the foreground is the pet | |
| 8 | Pets U6 | Stance buttons highlight on the pet bar (blocked by #1674) | `lab_screenshot-20261013-025532.263.png`, `...025547.607.png` (nearest) | Looker UNSURE x3: no bar or highlight visible | |
| 9 | Pets U14 | Jaffa, Lo'taur, Prime: model, weapon, name plate, Prime visibly different from Jaffa | `lab_screenshot-20261013-034358.958.png`, `...034409.355.png`, `...034419.749.png` (nearest) | Looker: right figures, no name plates, no staff seen, Prime not clearly different | |
| 10 | Deployables DP-U1 | The 2 s warmup cast bar while placing the emitter | none usable | Looker UNSURE: the shot came 6 s after the spawn | |
| 11 | Deployables DP-U2 | A black tripod machine gun on the ground with the label "Deployable: Microwave Emitter" | `lab_screenshot-20261013-031838.590.png` | Verifier: model appears about 8 s after the press; an earlier shot showed bare floor | |
| 12 | Deployables DP-U4 | After 30 s the emitter is gone from the ground | none usable | Looker UNSURE (a pet hid the avatar); server `despawned expired` at 30.0 s | |

Not on this sheet because they need a second player or a non-GM account: duels, the chat flood limit (D4), DND and Ignore.
