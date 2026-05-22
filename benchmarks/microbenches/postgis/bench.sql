SELECT count(*)
FROM mb12_points p,
     ST_SetSRID(ST_MakePoint(0, 0), 4326) origin
WHERE ST_DWithin(p.geom, origin, 1.0);
