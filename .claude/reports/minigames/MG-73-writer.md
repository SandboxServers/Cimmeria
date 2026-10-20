# MG-73 writer report

New docs/analysis/minigames/uat-lab.md (CRLF, lint clean): recipes R1-R8 (cellblock Livewire 1016/1017, five Debug Area terminals, placement check, mission 582/2307, abilities 1327/2089, Prison Boot, shell 5 s refusal and 60 s timeout, D-MG11 on/off). Ask-the-user line first. Linked from docs/content/debug-area.md Minigames row.

Nothing was run; no lab.

Unverified in the recipes (flagged in the file):
- Movement-lock rejection log line (grep `MovementLocked`; exact text unknown).
- Shell `timeout` is logged at DEBUG, so the recipe proves it by the failure result and chain 24057.
- Terminal nameplate, y values, Converse rule validation.
- `.gotolocation DebugArea x y z` coordinates and GM commands (`/gmmissionassign`, `/gmmissionadvance`, `/gmgiveability`, `/gmgiveitem`) taken from docs/commands.md, not tried.
- "Cellblock pair" read as chains 1016/1017 (mission 638); 1041/1042 and 1060/1061 are noted as optional.

Not done: the README.md in docs/analysis/minigames does not list uat-lab.md (not in the packet); the unified-uat.md link target is MG-99's.
