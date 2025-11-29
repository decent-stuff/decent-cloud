-- Update example offerings with realistic currencies
-- The example offerings all have 'ICP' but should have mixed currencies for testing

-- Update compute-001 (Basic VPS) to USD
UPDATE provider_offerings
SET currency = 'USD'
WHERE offering_id = 'compute-001'
AND pubkey = x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572';

-- Update compute-002 (Performance VPS) to EUR
UPDATE provider_offerings
SET currency = 'EUR'
WHERE offering_id = 'compute-002'
AND pubkey = x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572';

-- Keep other offerings as ICP (they're cryptocurrency-native services)
