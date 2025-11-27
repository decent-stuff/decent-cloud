-- Add GPU-specific fields for GPU/AI workload offerings
ALTER TABLE provider_offerings ADD COLUMN gpu_count INTEGER;
ALTER TABLE provider_offerings ADD COLUMN gpu_memory_gb INTEGER;
