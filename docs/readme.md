# Cimmeria Project Documentation

Welcome to the Cimmeria documentation — a server emulator for the Stargate Worlds MMO.

These docs explain what we've figured out about how the game works, what tools are available, and where the project stands. For the deep technical details (memory addresses, wire formats, code analysis), see the [technical/](technical/) folder.

## Getting Started

- [How SGW Works](how-sgw-works.md) — The technology behind Stargate Worlds and how the pieces fit together
- [Client Tools](client-tools.md) — How to use the launcher, editor mode, and debug tools
- [Building the Server](building.md) — How to build and run the Cimmeria server

## Understanding the Game

- [Game Systems](game-systems.md) — Every game feature: combat, abilities, stargates, missions, crafting, and more
- [Game Data](game-data.md) — What game content we have (items, abilities, missions, etc.) and what's missing
- [Commands Reference](commands.md) — Player commands, chat, GM tools, and debug commands

## How It Connects

- [Connection Flow](connection-flow.md) — How a player logs in and enters the game world
- [Network Messages](network-messages.md) — The complete catalog of what the client and server say to each other

## Project Status

- [Project Status](project-status.md) — What works, what's left to build, and the roadmap ahead

## Technical Documentation

The [technical/](technical/) folder contains the detailed reverse engineering analysis with memory addresses, byte-level wire formats, RTTI class hierarchies, and code-level implementation details. These are the primary reference for developers working on the C++ server code.
