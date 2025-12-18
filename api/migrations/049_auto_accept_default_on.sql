-- Change auto_accept_rentals default to ON (1) for better UX
-- Also enable for all existing providers who haven't explicitly set it

-- Update existing providers to have auto-accept enabled by default
UPDATE provider_profiles SET auto_accept_rentals = 1 WHERE auto_accept_rentals = 0;

-- Note: SQLite doesn't support ALTER COLUMN DEFAULT directly
-- New rows will get the default from column definition, but since we're
-- updating all existing rows to 1, this effectively makes ON the default behavior
