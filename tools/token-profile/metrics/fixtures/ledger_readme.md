# Fixture campaign

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-1 | Decided | Use the packet table | Not a packet. |

## Packets

| ID | Packet | Implementer | Size | Wave | Depends on | Status |
|---|---|---|---|---|---|---|
| FX-01 | Record log types | packet-coder | S | 1 | none | Integrated (#1527) |
| FX-02 | Inbound decoder | rust-gameserver-dev (Haiku ran out of context) | M | 2 | FX-01 | Integrated (#1627) |
| FX-03 | Docs | documentation-writer | S | 3 | FX-02 | Ready |
