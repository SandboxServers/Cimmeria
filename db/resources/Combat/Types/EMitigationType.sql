--
-- TOC entry 992 (class 1247 OID 61694)
-- Name: EMitigationType; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EMitigationType" AS ENUM (
    'MITIGATION_Physical',
    'MITIGATION_Physical_Impact',
    'MITIGATION_Physical_Concussive',
    'MITIGATION_Physical_Slashing',
    'MITIGATION_Physical_Piercing',
    'MITIGATION_Energy',
    'MITIGATION_Energy_Plasma',
    'MITIGATION_Energy_Radiation',
    'MITIGATION_Energy_Electrical',
    'MITIGATION_Energy_Particle',
    'MITIGATION_Environmental',
    'MITIGATION_Enviro_Biological',
    'MITIGATION_Enviro_Chemical',
    'MITIGATION_Enviro_Thermal',
    'MITIGATION_Enviro_Cold'
);

