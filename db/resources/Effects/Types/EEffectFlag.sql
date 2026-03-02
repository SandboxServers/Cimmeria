--
-- TOC entry 887 (class 1247 OID 60944)
-- Name: EEffectFlag; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EEffectFlag" AS ENUM (
    'EF_Beneficial_Effect',
    'EF_Offline_Time_Counts',
    'EF_ClearOnDeath',
    'EF_ClearOnDamage',
    'EF_DontUseQR',
    'EF_HasInductionBar',
    'EF_SequenceOnFinish',
    'EF_SequenceOnPulse',
    'EF_SequenceOnFail',
    'EF_SequenceOnStart',
    'EF_SequenceOnRemove',
    'EF_ClearOnRez',
    'EF_HideIconOnClient',
    'EF_OnlySendToGroup',
    'EF_OnlySendToSelf',
    'EF_RemoveOnDisguiseZeroed',
    'EF_RemoveOnBandolierSlotChange',
    'EF_ResolveOnAbilityUser',
    'EF_DisableDisguiseWhenRemoved',
    'EF_AlwaysPersist',
    'EF_Response',
    'EF_RemoveOnStealthZeroed',
    'EF_CalculateQRFromTarget',
    'EF_PromptConfirmationDialog',
    'EF_SequenceOnConfirmation'
);

