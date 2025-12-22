-- Fix example offerings to use ISO country codes instead of full names
-- This ensures they can be matched to agent pools via country_to_region mapping

UPDATE provider_offerings
SET datacenter_country = 'US'
WHERE datacenter_country = 'USA'
  AND pubkey = x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572';

UPDATE provider_offerings
SET datacenter_country = 'DE'
WHERE datacenter_country = 'Germany'
  AND pubkey = x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572';

UPDATE provider_offerings
SET datacenter_country = 'SG'
WHERE datacenter_country = 'Singapore'
  AND pubkey = x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572';

UPDATE provider_offerings
SET datacenter_country = 'NL'
WHERE datacenter_country = 'Netherlands'
  AND pubkey = x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572';
