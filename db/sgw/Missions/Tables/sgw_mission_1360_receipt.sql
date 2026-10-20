-- Durable one-shot receipt for the Frost's Letter turn-in.
CREATE TABLE sgw_mission_1360_receipt (
    player_id integer NOT NULL,
    mission_id integer NOT NULL DEFAULT 1360,
    attempt_key text NOT NULL CHECK (length(attempt_key) BETWEEN 1 AND 256),
    session_token text NOT NULL CHECK (session_token ~ '^[0-9a-f]{32}$'),
    item_id integer NOT NULL,
    container_id integer NOT NULL,
    stack_before integer NOT NULL,
    stack_after integer NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    CONSTRAINT sgw_mission_1360_receipt_mission_chk CHECK (mission_id = 1360),
    CONSTRAINT sgw_mission_1360_receipt_stack_chk CHECK (
        stack_before >= 1 AND stack_after >= 0 AND stack_after = stack_before - 1
    )
);
