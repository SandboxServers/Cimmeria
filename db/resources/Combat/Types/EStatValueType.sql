--
-- TOC entry 1085 (class 1247 OID 62216)
-- Name: EStatValueType; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EStatValueType" AS ENUM (
    'SVT_BaseMin',
    'SVT_BaseCurrent',
    'SVT_BaseMax',
    'SVT_Min',
    'SVT_Current',
    'SVT_Max'
);

