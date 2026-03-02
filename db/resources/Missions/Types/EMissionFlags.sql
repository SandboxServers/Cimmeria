--
-- TOC entry 986 (class 1247 OID 61662)
-- Name: EMissionFlags; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EMissionFlags" AS ENUM (
    'EMISSION_FLAG_isEnabled',
    'EMISSION_FLAG_isAStory',
    'EMISSION_FLAG_isOverride',
    'EMISSION_FLAG_isHidden',
    'EMISSION_FLAG_canFail',
    'EMISSION_FLAG_canRepeatOnFail',
    'EMISSION_FLAG_canAbandon',
    'EMISSION_FLAG_isShareable',
    'EMISSION_FLAG_isPvp',
    'EMISSION_FLAG_isInstance',
    'EMISSION_FLAG_isFaction',
    'EMISSION_FLAG_awardsXP',
    'EMISSION_FLAG_awardsCash'
);

