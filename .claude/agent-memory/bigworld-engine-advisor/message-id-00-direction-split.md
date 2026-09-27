---
name: message-id-00-direction-split
description: Msg ID 0x00 means different things per direction in SGW — client->server it is BASEAPP_LOGIN (WORD), server->client it is AUTHENTICATE (DWORD). REPLY_MESSAGE 0xFF borrows the AUTHENTICATE descriptor.
metadata:
  type: project
---

# Msg ID 0x00 is direction-dependent; 0xFF has no descriptor of its own

`deprecated/cpp/src/baseapp/mercury/sgw/messages.cpp` holds two tables:

- **ClientMessageList** (client -> server, 0x00-0x7F): `0x00 = BASEAPP_LOGIN, WORD_LENGTH`
  (line 26); `0x01 = AUTHENTICATE, WORD_LENGTH` (line 33).
- **ServerMessageList** (server -> client, 0x00-0x7F): `0x00 = AUTHENTICATE, DWORD_LENGTH`
  (line 127), commented "Never seen this frame".

So "AUTHENTICATE 0x00 is DWORD_LENGTH" is true only server->client. Inbound 0x00 is a
*different message* and is correctly WORD_LENGTH. Do not conflate them when auditing
inbound length-type tables.

**REPLY_MESSAGE (0xFF) has no table entry** — the tables only cover 0x00-0x7F.
`deprecated/cpp/src/baseapp/mercury/sgw/connect_handler.cpp:93` builds it as
`bundle.beginMessage(BASEMSG_REPLY_MESSAGE, ServerMessageList[BASEMSG_AUTHENTICATE], Bundle::FLUSH)`
— i.e. it *borrows the server AUTHENTICATE descriptor*, which is DWORD_LENGTH. That is
the provenance of the DWORD claim in `docs/protocol/mercury-wire-format.md:294`, and it
is stronger than the uncited `WORD_LENGTH` table row at
`docs/reverse-engineering/findings/space-viewport-wire-formats.md:609,652` that
`docs/drafts/spec/mercury-wire-format.md:739` and its deviation table at :1308 inherited.

Related: [[interface-element-length-escape]]
