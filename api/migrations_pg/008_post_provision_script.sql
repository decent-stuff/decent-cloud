-- Phase 3: Templated Deployments
-- Add post_provision_script field to provider_offerings
-- This script will be executed via SSH after VM provisioning completes
--
-- The script should include a shebang line to specify the interpreter:
--   #!/bin/bash, #!/usr/bin/env python3, #!/usr/bin/perl, etc.
-- If no shebang is present, /bin/sh is used as default.

ALTER TABLE provider_offerings ADD COLUMN IF NOT EXISTS post_provision_script TEXT;

COMMENT ON COLUMN provider_offerings.post_provision_script IS 'Script to execute via SSH after VM provisioning. Use shebang (#!/bin/bash, #!/usr/bin/env python3) to specify interpreter.';
