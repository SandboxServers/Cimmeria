--
-- TOC entry 773 (class 1247 OID 59926)
-- Name: EAIErrorStateAction; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EAIErrorStateAction" AS ENUM (
    'EAI_ERROR_ACTION_Nothing',
    'EAI_ERROR_ACTION_Despawn',
    'EAI_ERROR_ACTION_Invisible'
);

