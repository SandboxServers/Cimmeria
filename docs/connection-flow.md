# Connection Flow

How a player gets from the login screen to running around in the game world. This covers the full journey from clicking "Login" to controlling their character.

> **Status: Fully working.** This entire flow has been confirmed end-to-end with a real client. Players can log in, select a shard, enter the game world, and play.

## Overview

The connection happens in three phases:

1. **Authentication** — "Who are you?" (SOAP/HTTP)
2. **Server Assignment** — "Which server should you play on?" (SOAP/HTTP)
3. **World Entry** — "Here's your character in the world" (Mercury/UDP)

## Phase 1: Authentication

The client sends a login request to the **Authentication Server** over HTTP using SOAP (an XML-based web protocol).

```
Player clicks Login
    |
    v
Client sends username + password to /SGWLogin/UserAuth
    |
    v
Auth Server checks credentials against database
    |
    v
Success: Returns shard list + account info
Failure: Returns one of 31 error codes (wrong password, account locked, etc.)
```

If login succeeds, the client shows the server/shard selection screen.

## Phase 2: Server Assignment

The player picks a shard (server) from the list.

```
Player selects a shard
    |
    v
Client sends shard choice to /SGWLogin/ServerSelection
    |
    v
Auth Server asks BaseAppMgr to allocate a BaseApp
    |
    v
Auth Server generates a 64-character encryption key (the "session key")
    |
    v
Auth Server sends back:
  - BaseApp IP address and port
  - Session key (used to encrypt all future traffic)
  - Login ticket (proves you authenticated)
```

At this point, the HTTP phase is done. Everything from here on uses the **Mercury protocol** over UDP.

## Phase 3: World Entry

The client connects directly to the assigned **BaseApp** using the Mercury reliable UDP protocol. All traffic is encrypted with AES-256 using the session key from Phase 2.

### Step 1: BaseApp Login

```
Client sends: baseAppLogin (ticket + session key)
Server sends: Ticket echo (proves server is legitimate)
```

The encryption filter activates on both sides. All subsequent packets are AES-256 encrypted with HMAC-MD5 integrity checking.

### Step 2: Time Sync

The server immediately sends three timing messages to synchronize the client's clock:

- **Update frequency** — "I'll send you updates 10 times per second"
- **Tick sync** — Current game time and tick rate
- **Game time** — Absolute time baseline

These are flushed as a single packet before anything else.

### Step 3: Client Says "I'm Ready"

The client sends `enableEntities` — a signal that it's done initializing and ready to receive game data. The server **must wait** for this before sending any entity information.

### Step 4: Create the Player

The server creates the player entity in two parts:

1. **Create Base Player** — Sets up the persistent part of the player (account, inventory, stats). This is the "base" entity managed by the BaseApp.

2. **Space Viewport Info** — Tells the client which game world/zone the player is in. This is a Stargate Worlds-specific message that must arrive before any map data. (Standard BigWorld doesn't have this — CME added it, probably for Stargate zone transitions.)

3. **Create Cell Player** — Places the player in the world at a specific position and rotation. This is the "cell" entity managed by the CellApp (the spatial simulation).

4. **Forced Position** — Sets the player's authoritative starting position with velocity.

### Step 5: Game Loop

After the cell player is created, the connection switches to "regular" mode:

- Server sends **tick sync** updates 10 times per second
- Server sends **entity updates** for nearby players, NPCs, and objects
- Client sends **movement updates** (position, rotation, velocity)
- Game messages flow in both directions (combat, chat, inventory, etc.)

## The Full Picture

```
Client                     Auth Server          BaseApp
  |                            |                    |
  |-- Login (HTTP) ---------->|                    |
  |<-- Shard list + account --|                    |
  |                            |                    |
  |-- Select shard (HTTP) --->|                    |
  |                            |-- Allocate ------->|
  |<-- BaseApp address + key --|                    |
  |                            |                    |
  |-- baseAppLogin (UDP, encrypted) --------------->|
  |<-- ticket echo ---------------------------------|
  |                                                 |
  |<-- time sync (3 messages, flushed) -------------|
  |                                                 |
  |-- enableEntities ------------------------------->|
  |                                                 |
  |<-- createBasePlayer ----------------------------|
  |<-- spaceViewportInfo (SGW custom) --------------|
  |<-- createCellPlayer ----------------------------|
  |<-- forcedPosition -------------------------------|
  |                                                 |
  |<-- tick updates (10 Hz) ------------------------|
  |-- movement updates ---------------------------->|
  |<-------------- game messages ------------------>|
```

## Error Handling

The system has 31 defined error codes covering everything from "wrong password" to "BaseApp at capacity". Key ones:

- **Connection failures** — DNS lookup failed, TCP/UDP connection failed
- **Auth failures** — Wrong password, user not found, account already in use
- **Server issues** — Database not ready, no BaseApps available, BaseApp overloaded
- **Protocol errors** — Version mismatch, malformed request

## Encryption

All Mercury traffic (Phase 3 onwards) is encrypted:

- **Algorithm**: AES-256-CBC (256-bit Advanced Encryption Standard)
- **Key**: 64-character hex string generated by the Auth Server
- **Integrity**: HMAC-MD5 on every packet (detects tampering/corruption)

This is a custom modification by CME. Standard BigWorld uses Blowfish encryption. CME upgraded to AES-256 for stronger security.

## Ports

| Service | Port | Protocol |
|---------|------|----------|
| Authentication Server | 8080 | HTTP (SOAP/XML) |
| BaseApp | 32832 | UDP (Mercury) |
