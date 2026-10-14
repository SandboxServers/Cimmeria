# Livewire solver split across lab and server

The server's `server_minigame_state` returns the ticket-free board and the
bounded SmartFox event ring. `server_minigame_tap_read` filters that ring by
sequence without consuming it, so an input flow can compare events after
each cursor move and click.

The 62-library hit atlas is generated from the installed client and loaded
by the lab daemon, not the game server. `lab_livewire_plan` therefore accepts
the server's board object and produces stage-coordinate candidates locally.
It does not act on the board. The planned topmost wire is a geometric
prediction; only an observed `processover` from the client proves the actual
Scaleform hit target before a click. The planned point may need the next
candidate when the movie viewport, animation, or raster hit semantics differ.

The planner treats missing playfield or moving art as unattached, matching
the current unsuffixed library names. It refuses a missing goal library.
The candidate band excludes the static cover and frame, and plan steps
account for earlier cuts before predicting later topmost targets.
