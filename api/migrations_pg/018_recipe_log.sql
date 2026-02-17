ALTER TABLE cloud_resources ADD COLUMN recipe_log TEXT;
COMMENT ON COLUMN cloud_resources.recipe_log IS 'Combined stdout/stderr from recipe script execution';
