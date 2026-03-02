--
-- TOC entry 995 (class 1247 OID 61726)
-- Name: EMobAggressionLevel; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EMobAggressionLevel" AS ENUM (
    'AGGRESSION_HOSTILE',
    'AGGRESSION_SUSPICIOUS',
    'AGGRESSION_NEUTRAL',
    'AGGRESSION_FRIENDLY',
    'AGGRESSION_DEFAULT'
);

