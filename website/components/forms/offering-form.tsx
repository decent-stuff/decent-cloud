"use client";

import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Card } from "@/components/ui/card";
import { useToast } from "@/components/ui/use-toast";
import { AuthenticatedIdentityResult } from "@/lib/auth-context";
import { updateOffering } from "@/lib/offering-service";
import { isYamlFormat } from "@/lib/yaml-utils";
import dynamic from "next/dynamic";

// Import CodeEditor dynamically to avoid SSR issues with Monaco
const CodeEditor = dynamic(() => import("@/components/code-editor"), {
  ssr: false,
  loading: () => (
    <div className="h-96 bg-black/30 text-white/50 font-mono p-4 rounded-md">
      Loading editor...
    </div>
  ),
});

interface OfferingFormProps {
  onSubmitSuccess: () => void;
  onCancel: () => void;
  authResult: AuthenticatedIdentityResult | null;
}

export default function OfferingForm({
  onSubmitSuccess,
  onCancel,
  authResult,
}: OfferingFormProps) {
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { toast } = useToast();
  const [formatType, setFormatType] = useState<"json" | "yaml">("json");

  const jsonExample = `{
  "api_version": "v0.1.0",
  "kind": "Offering",
  "metadata": {
    "name": "Node Provider Name",
    "version": "1.0"
  },
  "provider": {
    "name": "generic cloud provider",
    "description": "a generic offering specification for a cloud provider"
  },
  "defaults": {
    "compliance": [
      "ISO 27001",
      "SOC 2"
    ],
    "sla": {
      "uptime": "99.95%",
      "measurement_period": "monthly",
      "support": {
        "levels": [
          "standard",
          "premium"
        ],
        "response_time": {
          "critical": "30 minutes",
          "high": "1 hour",
          "medium": "4 hours",
          "low": "8 hours"
        }
      },
      "downtime_compensation": [
        {
          "less_than": "4 minutes",
          "credit_percentage": 5
        },
        {
          "less_than": "15 minutes",
          "credit_percentage": 10
        },
        {
          "more_than": "15 minutes",
          "credit_percentage": 20
        }
      ],
      "maintenance": {
        "window": "Sunday 00:00 - 04:00 UTC",
        "notification_period": "7 days"
      }
    },
    "terms_of_service": [
      {
        "Minimum contract period": "none"
      },
      {
        "Cancellation period": "30 days"
      },
      {
        "Activation period": "Up to 24h"
      }
    ],
    "machine_spec": {
      "instance_types": [
        {
          "id": "gp-small",
          "type": "general-purpose",
          "cpu": "2 vCPUs",
          "memory": "2 GB",
          "storage": {
            "type": "SSD",
            "size": "50 GB"
          },
          "pricing": {
            "on_demand": {
              "hour": 500000000
            },
            "reserved": {
              "year": 10000000000
            }
          },
          "tags": [
            "low-cost",
            "small-instance"
          ],
          "metadata": {
            "optimized_for": "general",
            "availability": "high"
          }
        }
      ]
    },
    "network_spec": {
      "vpc_support": true,
      "public_ip": true,
      "private_ip": true,
      "load_balancers": {
        "type": [
          "application",
          "network"
        ]
      },
      "firewalls": {
        "stateful": true,
        "stateless": false
      }
    },
    "security": {
      "data_encryption": {
        "at_rest": "AES-256",
        "in_transit": "TLS 1.2 or higher"
      },
      "identity_and_access_management": {
        "multi_factor_authentication": true,
        "role_based_access_control": true,
        "single_sign_on": true
      }
    },
    "monitoring": {
      "enabled": true,
      "metrics": {
        "cpu_utilization": true,
        "memory_usage": true,
        "disk_iops": true,
        "network_traffic": true
      },
      "logging": {
        "enabled": true,
        "log_retention": "30 days"
      }
    },
    "backup": {
      "enabled": true,
      "frequency": "daily",
      "retention": "7 days",
      "disaster_recovery": {
        "cross_region_replication": true,
        "failover_time": "1 hour"
      }
    },
    "cost_optimization": {
      "spot_instances_available": true,
      "savings_plans": [
        {
          "type": "compute",
          "discount": "Up to 66%"
        }
      ]
    },
    "service_integrations": {
      "databases": [
        "MySQL",
        "PostgreSQL",
        "MongoDB"
      ],
      "storage_services": [
        "Object Storage",
        "Block Storage"
      ],
      "messaging_services": [
        "Kafka",
        "RabbitMQ"
      ]
    }
  },
  "regions": [
    {
      "name": "eu-central-1",
      "description": "central europe region",
      "geography": {
        "continent": "Europe",
        "country": "Germany",
        "iso_codes": {
          "country_code": "DE",
          "region_code": "EU"
        }
      },
      "compliance": [
        "GDPR"
      ],
      "machine_spec": {
        "instance_types": [
          {
            "id": "mem-medium",
            "type": "memory-optimized",
            "cpu": "4 vCPUs",
            "memory": "16 GB",
            "storage": {
              "type": "SSD",
              "size": "100 GB"
            },
            "pricing": {
              "on_demand": {
                "hour": "0.1"
              },
              "reserved": {
                "year": "20"
              }
            },
            "tags": [
              "high-performance",
              "GDPR-compliant"
            ],
            "metadata": {
              "optimized_for": "high-memory",
              "availability": "high"
            }
          }
        ]
      },
      "availability_zones": [
        {
          "name": "eu-central-1a",
          "description": "primary availability zone"
        },
        {
          "name": "eu-central-1b",
          "description": "secondary availability zone"
        }
      ]
    },
    {
      "name": "us-east-1",
      "description": "united states east coast region",
      "geography": {
        "continent": "North America",
        "country": "United States",
        "iso_codes": {
          "country_code": "US",
          "region_code": "NA"
        }
      },
      "compliance": [
        "SOC 2"
      ],
      "machine_spec": {
        "instance_types": [
          {
            "id": "cpu-large",
            "type": "compute-optimized",
            "cpu": "8 vCPUs",
            "memory": "32 GB",
            "storage": {
              "type": "NVMe",
              "size": "200 GB"
            },
            "pricing": {
              "on_demand": {
                "hour": "0.2"
              },
              "reserved": {
                "year": "50"
              }
            },
            "tags": [
              "high-compute",
              "cost-effective"
            ],
            "metadata": {
              "optimized_for": "high-compute",
              "availability": "high"
            }
          },
          {
            "id": "ai-large",
            "type": "ai-optimized",
            "description": "high-performance instance optimized for AI/ML workloads",
            "cpu": "16 vCPUs",
            "gpu": {
              "count": 4,
              "type": "NVIDIA A100",
              "memory": "80 GB"
            },
            "memory": "256 GB",
            "storage": {
              "type": "NVMe SSD",
              "size": "2 TB",
              "iops": 100000
            },
            "network": {
              "bandwidth": "25 Gbps",
              "latency": "low"
            },
            "pricing": {
              "on_demand": {
                "hour": "0.5"
              },
              "reserved": {
                "year": "100"
              }
            },
            "tags": [
              "ai-ml",
              "gpu-optimized"
            ],
            "metadata": {
              "optimized_for": "ai-ml",
              "availability": "limited"
            },
            "ai_spec": {
              "framework_optimizations": [
                "TensorFlow",
                "PyTorch",
                "RAPIDS AI"
              ],
              "software_stack": {
                "preinstalled": [
                  "CUDA 11.x",
                  "cuDNN 8.x",
                  "NVIDIA Driver 450+"
                ]
              },
              "enhanced_networking": true,
              "distributed_training_support": true
            }
          }
        ]
      },
      "availability_zones": [
        {
          "name": "us-east-1a",
          "description": "primary availability zone"
        },
        {
          "name": "us-east-1b",
          "description": "secondary availability zone"
        }
      ]
    }
  ]
}`;

  const yamlExample = `# Versioning Information
api_version: v0.1.0
kind: Offering
metadata:
  name: "Node Provider Name"
  version: "1.0"

# This template provides an example of a cloud provider offering.
# - Default settings are applied globally and can be overridden in a particular region.
# - All fields can be used to filter and query instances.
# - The schema could be extended to include additional features like IAM and service integrations.

# Provider Information
provider:
  name: generic cloud provider
  description: a generic offering specification for a cloud provider

# Default Specifications (applies globally unless overridden)
defaults:
  compliance:
    - ISO 27001
    - SOC 2
  sla:
    uptime: "99.95%"
    measurement_period: "monthly"
    support:
      levels:
        - standard
        - premium
      response_time:
        critical: "30 minutes"
        high: "1 hour"
        medium: "4 hours"
        low: "8 hours"
    downtime_compensation:
      - less_than: 4 minutes
        credit_percentage: 5
      - less_than: 15 minutes
        credit_percentage: 10
      - more_than: 15 minutes
        credit_percentage: 20
    maintenance:
      window: "Sunday 00:00 - 04:00 UTC"
      notification_period: "7 days"

  terms_of_service:
    - Minimum contract period: none
    - Cancellation period: 30 days
    - Activation period: Up to 24h

  machine_spec:
    instance_types:
      - id: gp-small
        type: general-purpose
        cpu: 2 vCPUs
        memory: 2 GB
        storage:
          type: SSD
          size: 50 GB
        pricing:
          on_demand:
            hour: 500_000_000
          reserved:
            year: 10_000_000_000
        tags:
          - low-cost
          - small-instance
        metadata:
          optimized_for: general
          availability: high

  network_spec:
    vpc_support: true
    public_ip: true
    private_ip: true
    load_balancers:
      type:
        - application
        - network
    firewalls:
      stateful: true
      stateless: false

  security:
    data_encryption:
      at_rest: "AES-256"
      in_transit: "TLS 1.2 or higher"
    identity_and_access_management:
      multi_factor_authentication: true
      role_based_access_control: true
      single_sign_on: true

  monitoring:
    enabled: true
    metrics:
      cpu_utilization: true
      memory_usage: true
      disk_iops: true
      network_traffic: true
    logging:
      enabled: true
      log_retention: "30 days"

  backup:
    enabled: true
    frequency: "daily"
    retention: "7 days"
    disaster_recovery:
      cross_region_replication: true
      failover_time: "1 hour"

  cost_optimization:
    spot_instances_available: true
    savings_plans:
      - type: compute
        discount: "Up to 66%"

  service_integrations:
    databases:
      - MySQL
      - PostgreSQL
      - MongoDB
    storage_services:
      - Object Storage
      - Block Storage
    messaging_services:
      - Kafka
      - RabbitMQ

# Region-Specific Overrides
regions:
  - name: eu-central-1
    description: central europe region
    geography:
      continent: Europe
      country: Germany
      iso_codes: # ISO 3166 country codes
        country_code: DE
        region_code: EU
    compliance:
      - GDPR
    machine_spec:
      instance_types:
        - id: mem-medium
          type: memory-optimized
          cpu: 4 vCPUs
          memory: 16 GB
          storage:
            type: SSD
            size: 100 GB
          pricing:
            on_demand:
              hour: "0.1"
            reserved:
              year: "20"
          tags:
            - high-performance
            - GDPR-compliant
          metadata:
            optimized_for: high-memory
            availability: high
    availability_zones:
      - name: eu-central-1a
        description: primary availability zone
      - name: eu-central-1b
        description: secondary availability zone

  - name: us-east-1
    description: united states east coast region
    geography:
      continent: North America
      country: United States
      iso_codes: # ISO 3166 country codes
        country_code: US
        region_code: NA
    compliance:
      - SOC 2
    machine_spec:
      instance_types:
        - id: cpu-large
          type: compute-optimized
          cpu: 8 vCPUs
          memory: 32 GB
          storage:
            type: NVMe
            size: 200 GB
          pricing:
            on_demand:
              hour: "0.2"
            reserved:
              year: "50"
          tags:
            - high-compute
            - cost-effective
          metadata:
            optimized_for: high-compute
            availability: high
        - id: ai-large
          type: ai-optimized
          description: high-performance instance optimized for AI/ML workloads
          cpu: 16 vCPUs
          gpu:
            count: 4
            type: NVIDIA A100
            memory: 80 GB
          memory: 256 GB
          storage:
            type: NVMe SSD
            size: 2 TB
            iops: 100000
          network:
            bandwidth: 25 Gbps
            latency: low
          pricing:
            on_demand:
              hour: "0.5"
            reserved:
              year: "100"
          tags:
            - ai-ml
            - gpu-optimized
          metadata:
            optimized_for: ai-ml
            availability: limited
          ai_spec:
            framework_optimizations:
              - TensorFlow
              - PyTorch
              - RAPIDS AI
            software_stack:
              preinstalled:
                - CUDA 11.x
                - cuDNN 8.x
                - NVIDIA Driver 450+
            enhanced_networking: true
            distributed_training_support: true

    availability_zones:
      - name: us-east-1a
        description: primary availability zone
      - name: us-east-1b
        description: secondary availability zone
  `;
  const [offeringData, setOfferingData] = useState<string>(jsonExample);

  // Update format type when text changes
  useEffect(() => {
    // Auto-detect format based on content
    if (isYamlFormat(offeringData)) {
      setFormatType("yaml");
    } else {
      setFormatType("json");
    }
  }, [offeringData]);

  const switchToFormat = (format: "json" | "yaml") => {
    if (format === formatType) return;

    // Set the example for the chosen format
    setOfferingData(format === "json" ? jsonExample : yamlExample);
    setFormatType(format);
  };

  // Validation is now handled by the offering service

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!authResult) {
      toast({
        title: "Authentication Required",
        description:
          "Please log in with a seed-phrase based identity to add an offering.",
        variant: "destructive",
      });
      return;
    }

    // Format validation is now handled by the service, which supports both JSON and YAML

    setIsSubmitting(true);

    try {
      const result = await updateOffering(offeringData, authResult);

      if (result.success) {
        toast({
          title: "Offering Added",
          description: result.message,
        });
        onSubmitSuccess();
      } else {
        toast({
          title: "Error Adding Offering",
          description: result.message,
          variant: "destructive",
        });
      }
    } catch (error) {
      toast({
        title: "Error",
        description:
          error instanceof Error ? error.message : "Failed to add offering",
        variant: "destructive",
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Card className="p-6 bg-white/10 backdrop-blur-sm rounded-lg border border-white/20">
      <h2 className="text-xl font-semibold text-white mb-4">
        Add New Offering
      </h2>

      <form onSubmit={handleSubmit}>
        <div className="mb-4">
          <div className="flex justify-between items-center mb-2">
            <Label htmlFor="offering-data" className="text-white block text-lg">
              Offering Data ({formatType.toUpperCase()})
            </Label>
            <div className="flex space-x-2">
              <Button
                type="button"
                size="sm"
                variant={formatType === "json" ? "default" : "outline"}
                onClick={() => switchToFormat("json")}
                className={
                  formatType === "json" ? "bg-blue-600 hover:bg-blue-700" : ""
                }
              >
                JSON
              </Button>
              <Button
                type="button"
                size="sm"
                variant={formatType === "yaml" ? "default" : "outline"}
                onClick={() => switchToFormat("yaml")}
                className={
                  formatType === "yaml" ? "bg-green-600 hover:bg-green-700" : ""
                }
              >
                YAML
              </Button>
            </div>
          </div>
          <div className="rounded-md overflow-hidden border border-gray-700">
            <CodeEditor
              value={offeringData}
              onChange={(value) => setOfferingData(value)}
              language={formatType}
              height="450px"
            />
          </div>
        </div>

        <div className="flex justify-end gap-2 mt-6">
          <Button
            type="button"
            variant="outline"
            onClick={onCancel}
            disabled={isSubmitting}
          >
            Cancel
          </Button>
          <Button
            type="submit"
            className="bg-green-600 hover:bg-green-700 text-white"
            disabled={isSubmitting || !authResult}
          >
            {isSubmitting ? "Submitting..." : "Add Offering"}
          </Button>
        </div>
      </form>
    </Card>
  );
}
