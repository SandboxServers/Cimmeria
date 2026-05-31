# Code of Conduct

Cimmeria is a hobby project bringing back a cancelled MMO. It exists because a small group of people cared enough to put in evenings and weekends. We want it to be a place where curious, technical people can collaborate on a fun and unusual problem without friction. This document spells out how we expect contributors and maintainers to treat each other.

## Our standards

**Be helpful.** When somebody asks a question — especially a basic one — answer it generously. Everyone here started knowing nothing about BigWorld, Mercury, or Stargate Worlds. Point to docs, share what you've learned, and remember that the person asking is probably going to be the person answering somebody else's question next month.

**Be precise.** "It doesn't work" is not a bug report; `BASE_EXTERNAL` was unset and the client got a connect timeout is. When you ask for help or open an issue, include the version, the steps, the symptom, and the log lines. When you offer help, ask the questions that get you to a reproducible state rather than guessing.

**Be honest about what you know.** This project has a lot of reverse-engineering surface area. It is fine — encouraged, even — to say "I think but I haven't verified" or "I extrapolated from the BigWorld 2.0.1 reference, not the SGW binary." It is not fine to present speculation as fact. The evidence-tier system in [`docs/reverse-engineering/evidence-standards.md`](docs/reverse-engineering/evidence-standards.md) exists for exactly this reason; use it.

**Be kind in code review.** Review the diff, not the person. A direct "this would deadlock if the channel is closed mid-handshake" is helpful. A condescending "obviously you should have known X" is not. Disagree with technical decisions freely, but keep the disagreement about the technical decision.

**Respect the project's scope.** This is a server for a specific game. We are not a general-purpose MMO engine, a discussion forum for the Stargate franchise, or a platform for unrelated political debate. Keep contributions and conversation aimed at making the emulator better.

## What's not OK

- Personal attacks, harassment, or insults — including ones that are framed as "just being honest."
- Discriminatory language or imagery, including racist, sexist, homophobic, or transphobic content. There is no edge case where this is acceptable.
- Sexual content or imagery in project spaces (issues, PRs, commits, chat).
- Doxxing — sharing private information about another contributor without their consent.
- Sustained disruption of issues, PRs, or chat (off-topic flooding, repeated bad-faith arguments after a maintainer asks you to stop).
- Encouraging or facilitating piracy of the original Stargate Worlds client beyond the limited fair-use territory the project already operates in. If you're not sure where the line is, ask before you post.

## Scope

This applies to all project spaces: GitHub issues and PRs, the project Discord, any official chat channels, the project's tracked email addresses, and any in-person event where you're representing the project.

It also applies when you are publicly representing the project elsewhere — for example, talking about Cimmeria in a YouTube video, on social media, or at a conference.

## Enforcement

Maintainers will enforce this code. If you experience or witness behaviour that violates it, please report it to **<steven.cady@gmail.com>** with a description of the incident, links or screenshots if you have them, and any other context that helps. Reports will be handled confidentially.

Possible responses, in roughly increasing order of severity:

1. A private message asking you to stop a specific behaviour.
2. A public correction in the affected thread.
3. Removing or editing a specific comment, commit, or PR.
4. A temporary ban from project spaces.
5. A permanent ban from project spaces.

Maintainers will try to talk to you first before escalating, except in cases where the behaviour is severe enough that an immediate ban is appropriate (doxxing, targeted harassment, illegal content).

## A note from the maintainers

We don't expect this document to come up often. The Cimmeria community is small, technical, and mostly delightful. We're writing it down because it sets the tone we want, and because new contributors deserve to know what kind of room they're walking into.

If you've read this far: thanks for caring. Now go fix a bug.
