--
-- TOC entry 950 (class 1247 OID 61442)
-- Name: EKismetViewType; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EKismetViewType" AS ENUM (
    'KISMET_VIEW_Witness',
    'KISMET_VIEW_NonWitness',
    'KISMET_VIEW_Finish',
    'KISMET_VIEW_EventInvoker',
    'KISMET_VIEW_EventWitness'
);

