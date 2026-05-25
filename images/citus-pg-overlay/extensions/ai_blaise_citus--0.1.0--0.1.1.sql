-- FEATURE: D9
-- Reversible companion-extension canary upgrade evidence for ai-blaise/citus.

CREATE TABLE companion_internal.extension_upgrade_events (
    event_id bigserial PRIMARY KEY,
    release_id text NOT NULL CHECK (btrim(release_id) <> ''),
    previous_version text NOT NULL CHECK (btrim(previous_version) <> ''),
    target_version text NOT NULL CHECK (btrim(target_version) <> ''),
    action text NOT NULL CHECK (action IN ('upgrade', 'rollback', 'canary_observation')),
    recorded_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX extension_upgrade_events_release_recorded_idx
    ON companion_internal.extension_upgrade_events(release_id, recorded_at DESC);

CREATE FUNCTION companion_internal.record_extension_upgrade_event(
    p_release_id text,
    p_previous_version text,
    p_target_version text,
    p_action text
)
RETURNS bigint
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    inserted_event_id bigint;
BEGIN
    IF p_release_id IS NULL OR btrim(p_release_id) = '' THEN
        RAISE EXCEPTION 'release_id must not be empty';
    END IF;
    IF p_previous_version IS NULL OR btrim(p_previous_version) = '' THEN
        RAISE EXCEPTION 'previous_version must not be empty';
    END IF;
    IF p_target_version IS NULL OR btrim(p_target_version) = '' THEN
        RAISE EXCEPTION 'target_version must not be empty';
    END IF;
    IF p_action NOT IN ('upgrade', 'rollback', 'canary_observation') THEN
        RAISE EXCEPTION 'unsupported extension upgrade action: %', p_action;
    END IF;

    INSERT INTO companion_internal.extension_upgrade_events(
        release_id, previous_version, target_version, action
    )
    VALUES (
        btrim(p_release_id),
        btrim(p_previous_version),
        btrim(p_target_version),
        p_action
    )
    RETURNING event_id INTO inserted_event_id;

    RETURN inserted_event_id;
END;
$$;

CREATE VIEW companion_extension_upgrade_events AS
SELECT
    event_id,
    release_id,
    previous_version,
    target_version,
    action,
    recorded_at
FROM companion_internal.extension_upgrade_events;
