---
title: "Client limits: lab"
type: reference
audience: engineers, the lab-chaos campaign
last_updated: 2026-10-11
---

# Client limits: lab

The capacity of the Live Research Lab: clients per daemon, how long a client takes to come up next to others, and the window each runs in. Grades, column rules and the summary are in the [limits index](README.md); the lab itself is [live-research-lab.md](../../guides/live-research-lab.md).

| Bound | Value | Grade | Source | Safe level | Break level | Notes |
|---|---|---|---|---|---|---|
| Clients per daemon | 5 | config | `crates/lab/src/supervisor/instance.rs` `CEILING_MAX_CLIENTS` | n/a | n/a | One per seeded lab account. CL-08 measures machine headroom at five; raising it is decision D-CL7. |
| Bridge up with other clients running | about 25 s | measured | lab, 2026-10-10, single observation, four other clients running (#1312) | n/a | n/a | |
| Watchdog grace before a first bridge answer | 90 s | config | `crates/lab/src/supervisor/stall_grace.rs` `BOOT_GRACE` | n/a | n/a | |
| Window | 1280x720 windowed | config | `crates/lab/src/supervisor/process.rs` `DEFAULT_WINDOW` | n/a | n/a | Raise candidate CL-15: cost per client at 640x360. |
