UPDATE storage_meta SET value = '2' WHERE key = 'schema_logical_version';
INSERT OR REPLACE INTO storage_meta (key, value) VALUES ('upgrade_marker_w2_4_1', 'schema_v2');