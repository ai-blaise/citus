-- FEATURE: D9
-- Reversible companion-extension canary upgrade evidence for ai-blaise/citus.

-- FEATURE: O14
CREATE OR REPLACE FUNCTION companion_internal.traceparent_is_valid(p_traceparent text)
RETURNS boolean
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT p_traceparent ~ '^00-[0-9a-f]{32}-[0-9a-f]{16}-[0-9a-f]{2}$'
       AND substring(p_traceparent FROM 4 FOR 32) <> repeat('0', 32)
       AND substring(p_traceparent FROM 37 FOR 16) <> repeat('0', 16)
$$;

CREATE OR REPLACE FUNCTION companion_internal.application_name_field(
    p_application_name text,
    p_key text
)
RETURNS text
LANGUAGE plpgsql
IMMUTABLE
PARALLEL SAFE
AS $$
DECLARE
    part text;
    field_key text;
BEGIN
    IF p_application_name IS NULL OR p_key IS NULL OR btrim(p_key) = '' THEN
        RETURN NULL;
    END IF;

    FOREACH part IN ARRAY string_to_array(p_application_name, ';') LOOP
        IF part LIKE '%=%' THEN
            field_key := split_part(part, '=', 1);
            IF field_key = p_key THEN
                RETURN NULLIF(substr(part, length(field_key) + 2), '');
            END IF;
        END IF;
    END LOOP;

    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION companion.current_traceparent()
RETURNS text
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT CASE
        WHEN companion_internal.traceparent_is_valid(current_setting('trace.parent', true))
            THEN current_setting('trace.parent', true)
        ELSE NULL
    END
$$;

CREATE OR REPLACE FUNCTION companion.current_tracestate()
RETURNS text
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT NULLIF(current_setting('trace.state', true), '')
$$;

CREATE OR REPLACE FUNCTION companion.project_traceparent_from_application_name(
    p_application_name text DEFAULT current_setting('application_name', true)
)
RETURNS jsonb
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    projected_traceparent text;
    projected_tracestate text;
BEGIN
    projected_traceparent := companion_internal.application_name_field(
        p_application_name,
        'traceparent'
    );
    projected_tracestate := companion_internal.application_name_field(
        p_application_name,
        'tracestate'
    );

    IF NOT companion_internal.traceparent_is_valid(projected_traceparent) THEN
        RETURN jsonb_build_object(
            'projected', false,
            'reason', 'missing-or-invalid-traceparent'
        );
    END IF;

    PERFORM set_config('trace.parent', projected_traceparent, true);
    IF projected_tracestate IS NOT NULL THEN
        PERFORM set_config('trace.state', projected_tracestate, true);
    END IF;

    RETURN jsonb_build_object(
        'projected', true,
        'traceparent', projected_traceparent,
        'tracestate', projected_tracestate
    );
END;
$$;

CREATE OR REPLACE FUNCTION companion_current_traceparent()
RETURNS text
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT companion.current_traceparent()
$$;

CREATE OR REPLACE FUNCTION companion_current_tracestate()
RETURNS text
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT companion.current_tracestate()
$$;

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
