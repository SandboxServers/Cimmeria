--
-- TOC entry 824 (class 1247 OID 60172)
-- Name: EChannel; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EChannel" AS ENUM (
    'CHAN_say',
    'CHAN_emote',
    'CHAN_yell',
    'CHAN_team',
    'CHAN_squad',
    'CHAN_command',
    'CHAN_officer',
    'CHAN_server',
    'CHAN_feedback',
    'CHAN_tell',
    'CHAN_splash',
    'CHAN_chat'
);

