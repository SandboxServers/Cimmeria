--
-- TOC entry 776 (class 1247 OID 59934)
-- Name: EAIState; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EAIState" AS ENUM (
    'AI_STATE_Spawning',
    'AI_STATE_Idle',
    'AI_STATE_Investigating',
    'AI_STATE_Fighting',
    'AI_STATE_Leashing',
    'AI_STATE_Dead',
    'AI_STATE_Despawning',
    'AI_STATE_Follow',
    'AI_STATE_Patrol',
    'AI_STATE_Wander',
    'AI_STATE_Submit',
    'AI_STATE_Error'
);

