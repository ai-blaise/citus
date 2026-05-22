SELECT partman.run_maintenance('public.mb5_parent');
-- Force premake of additional child partitions.
UPDATE partman.part_config SET premake = :row_count
  WHERE parent_table = 'public.mb5_parent';
SELECT partman.run_maintenance('public.mb5_parent');
