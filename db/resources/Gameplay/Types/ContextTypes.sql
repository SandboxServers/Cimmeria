--
-- TOC entry 770 (class 1247 OID 59914)
-- Name: ContextTypes; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "ContextTypes" AS ENUM (
    'CONTEXT_None',
    'CONTEXT_Talk',
    'CONTEXT_Examine',
    'CONTEXT_Kill',
    'CONTEXT_Interact'
);

