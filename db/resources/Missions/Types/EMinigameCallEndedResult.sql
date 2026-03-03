--
-- TOC entry 980 (class 1247 OID 61616)
-- Name: EMinigameCallEndedResult; Type: TYPE; Schema: resources; Owner: -
--

CREATE TYPE "EMinigameCallEndedResult" AS ENUM (
    'MCER_SuccessPayTip',
    'MCER_CallerClosedWindow',
    'MCER_CallerMovedTooFar',
    'MCER_CallerInterrupted',
    'MCER_CallerDied',
    'MCER_CallerDisconnected',
    'MCER_RemoteClosedWindow',
    'MCER_RemoteMovedTooFar',
    'MCER_RemoteInterrupted',
    'MCER_RemoteDied',
    'MCER_RemoteDisconnected'
);

