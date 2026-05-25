-- FEATURE: D9
-- Reverse the 0.1.0 -> 0.1.1 canary-upgrade evidence surface.

DROP VIEW IF EXISTS companion_extension_upgrade_events;
DROP FUNCTION IF EXISTS companion_internal.record_extension_upgrade_event(text, text, text, text);
DROP TABLE IF EXISTS companion_internal.extension_upgrade_events;
