-- FEATURE: MB12 postgis ST_DWithin microbench setup.
CREATE EXTENSION IF NOT EXISTS postgis;
DROP TABLE IF EXISTS mb12_points;
CREATE TABLE mb12_points (
    id BIGINT PRIMARY KEY,
    geom geometry(POINT, 4326) NOT NULL
);
INSERT INTO mb12_points
SELECT s, ST_SetSRID(ST_MakePoint(random() * 360 - 180,
                                  random() * 180 - 90), 4326)
FROM generate_series(1, 100000) s;
CREATE INDEX mb12_points_geom ON mb12_points USING GIST (geom);
ANALYZE mb12_points;
