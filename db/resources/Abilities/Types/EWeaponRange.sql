--
-- TOC entry 1133 (class 1247 OID 62672)
-- Name: EWeaponRange; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EWeaponRange" AS ENUM (
    'WEAPON_RANGE_Invalid',
    'WEAPON_RANGE_Pointblank',
    'WEAPON_RANGE_Min',
    'WEAPON_RANGE_Near',
    'WEAPON_RANGE_Mid',
    'WEAPON_RANGE_Far',
    'WEAPON_RANGE_Melee'
);

