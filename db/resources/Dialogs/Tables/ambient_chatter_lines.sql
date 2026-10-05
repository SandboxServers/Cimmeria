--
-- Ambient chatter lines: what an `ambient_chatter_groups` group says. Lines
-- with the same `exchange_id` form one exchange (a short scene), spoken in
-- `line_index` order; the group plays its exchanges in `exchange_id` order and
-- starts again from the first.
--
-- `speaker_tag` is the `spawnlist.tag` of the NPC who speaks the line, in the
-- group's world; the chat window prefixes the line with that NPC's name.
-- `delay_ms` is the pause before the line: after the previous line, or after
-- the exchange starts for the first one. `text` is shown as written, so it
-- stays non-blank and short enough for one chat line.
--
-- Name: ambient_chatter_lines; Type: TABLE; Schema: resources; Owner: -
--

CREATE TABLE ambient_chatter_lines (
    group_id integer NOT NULL,
    exchange_id integer NOT NULL,
    line_index integer NOT NULL,
    speaker_tag text NOT NULL,
    delay_ms integer DEFAULT 4000 NOT NULL,
    text text NOT NULL,
    CONSTRAINT ambient_chatter_lines_delay_range CHECK (delay_ms >= 0 AND delay_ms <= 60000),
    CONSTRAINT ambient_chatter_lines_text_shape CHECK (length(btrim(text)) > 0 AND length(text) <= 200)
);
