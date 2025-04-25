"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { Card } from "@/components/ui/card";
import { useToast } from "@/components/ui/use-toast";
import { AuthenticatedIdentityResult } from "@/lib/auth-context";
import { updateOffering } from "@/lib/offering-service";

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
  const [offeringData, setOfferingData] = useState<string>(`{
  "api_version": "v0.1.0",
  "kind": "Offering",
  "metadata": {
    "name": "My Cloud Offering",
    "version": "1.0"
  },
  "provider": {
    "name": "My Provider Name",
    "description": "Description of my cloud offering"
  },
  "defaults": {
    "machine_spec": {
      "instance_types": [
        {
          "id": "small",
          "type": "general-purpose",
          "cpu": "2 vCPUs",
          "memory": "4 GB",
          "storage": {
            "type": "SSD",
            "size": "50 GB"
          },
          "pricing": {
            "on_demand": {
              "hour": 50000000
            }
          },
          "metadata": {
            "optimized_for": "general",
            "availability": "high"
          }
        }
      ]
    },
    "terms_of_service": [
      "Minimum contract period: none",
      "Cancellation period: 1 day"
    ],
    "network_spec": {
      "vpc_support": true,
      "public_ip": true,
      "private_ip": true,
      "load_balancers": {
        "type": [
          "network"
        ]
      }
    }
  },
  "regions": [
    {
      "name": "eu-central-1",
      "description": "Central Europe region",
      "geography": {
        "continent": "Europe",
        "country": "Germany",
        "iso_codes": {
          "country_code": "DE",
          "region_code": "EU"
        }
      },
      "availability_zones": [
        {
          "name": "eu-central-1a",
          "description": "Primary availability zone"
        }
      ]
    }
  ]
}`);

  const handleJsonChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setOfferingData(e.target.value);
  };

  const validateJson = (jsonString: string): boolean => {
    try {
      JSON.parse(jsonString);
      return true;
    } catch {
      return false;
    }
  };

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

    if (!validateJson(offeringData)) {
      toast({
        title: "Invalid JSON",
        description: "Please enter a valid JSON offering definition.",
        variant: "destructive",
      });
      return;
    }

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
          <Label htmlFor="offering-json" className="text-white mb-2 block">
            Offering JSON
          </Label>
          <Textarea
            id="offering-json"
            className="h-96 font-mono text-sm bg-black/30 text-white"
            placeholder="Enter your offering JSON..."
            value={offeringData}
            onChange={handleJsonChange}
          />
          <p className="text-white/70 text-xs mt-2">
            Enter a valid JSON offering definition. This will be signed with
            your identity and submitted to the network.
          </p>
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
