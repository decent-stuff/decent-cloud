"use client";

import { useState } from "react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faPlus,
  faEdit,
  faTrash,
  faTimes,
} from "@fortawesome/free-solid-svg-icons";
import HeaderSection from "@/components/ui/header";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { useToast } from "@/components/ui/use-toast";
import { useAuth } from "@/lib/auth-context";
import { updateOffering } from "@/lib/offering-service";

// Default offering template
const DEFAULT_OFFERING_TEMPLATE = `{
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
}`;

export default function OfferingsPage() {
  // Define an interface for the offering structure
  interface Offering {
    id: string;
    name: string;
    type: string;
    price: string;
    specs: string;
    location: string;
    status: string;
    created: string;
  }

  const [offerings, setOfferings] = useState<Offering[]>([]);
  const [showOfferingForm, setShowOfferingForm] = useState(false);
  const [offeringData, setOfferingData] = useState(DEFAULT_OFFERING_TEMPLATE);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { toast } = useToast();
  const { isAuthenticated, getAuthenticatedIdentity } = useAuth();

  // Function to handle JSON change in the editor
  const handleJsonChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setOfferingData(e.target.value);
  };

  // Function to validate JSON
  const validateJson = (jsonString: string): boolean => {
    try {
      JSON.parse(jsonString);
      return true;
    } catch {
      return false;
    }
  };

  // Function to submit offering
  const handleSubmitOffering = async () => {
    if (!isAuthenticated) {
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
      // Get authenticated identity for signing from auth context
      const authIdentity = await getAuthenticatedIdentity();
      const result = await updateOffering(offeringData, authIdentity);

      if (result.success) {
        toast({
          title:
            "Offering Added, it will be publicly listed from the next Ledger block, in a few minutes.",
          description: result.message,
        });

        // Parse the offering to display it in the list
        try {
          const offeringJson = JSON.parse(offeringData);
          const newOffering = {
            id: Date.now().toString(),
            name: offeringJson.metadata?.name || "Unnamed Offering",
            type: offeringJson.provider?.name || "Unknown Type",
            price: "Custom", // This would need to be extracted from the offering structure
            specs: `${
              offeringJson.defaults?.machine_spec?.instance_types?.[0]?.cpu ||
              "Custom CPU"
            }, ${
              offeringJson.defaults?.machine_spec?.instance_types?.[0]
                ?.memory || "Custom Memory"
            }`,
            location: offeringJson.regions?.[0]?.geography?.country || "Global",
            status: "Active",
            created: new Date().toISOString().split("T")[0],
          };

          setOfferings([newOffering, ...offerings]);
          setShowOfferingForm(false);
        } catch (e) {
          console.error("Failed to parse offering for display", e);
        }
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

  const handleDeleteOffering = (id: string) => {
    setOfferings(offerings.filter((offering) => offering.id !== id));
  };

  return (
    <div className="container mx-auto px-4 py-8">
      <HeaderSection
        title="My Offerings"
        subtitle="Manage your cloud offerings on the Decent Cloud marketplace"
      />

      <div className="bg-white/10 p-6 rounded-lg backdrop-blur-sm mb-6">
        <div className="mb-6">
          <h3 className="text-xl font-semibold mb-2 text-white">
            Provider Dashboard
          </h3>
          <p className="text-white/90 mb-4">
            Add, edit, and manage your cloud offerings on the Decent Cloud
            marketplace.
          </p>
        </div>

        <div className="flex justify-end mb-6">
          <Button
            className="bg-green-600 hover:bg-green-700 text-white flex items-center gap-2"
            onClick={() => setShowOfferingForm(true)}
            disabled={!isAuthenticated}
          >
            <FontAwesomeIcon icon={faPlus} />
            <span>Add New Offering</span>
          </Button>
        </div>

        {showOfferingForm && (
          <Card className="p-6 bg-black/20 backdrop-blur-sm rounded-lg border border-white/20 mb-6">
            <div className="flex justify-between items-center mb-4">
              <h3 className="text-xl font-semibold text-white">
                Add New Offering
              </h3>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setShowOfferingForm(false)}
                className="text-white/70 hover:text-white hover:bg-white/10"
              >
                <FontAwesomeIcon icon={faTimes} />
              </Button>
            </div>

            <div className="mb-4">
              <Label htmlFor="offering-json" className="text-white mb-2 block">
                ⚠️ Please make sure to adjust the template offering JSON before
                submitting
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
                onClick={() => setShowOfferingForm(false)}
                disabled={isSubmitting}
              >
                Cancel
              </Button>
              <Button
                type="button"
                className="bg-green-600 hover:bg-green-700 text-white"
                disabled={isSubmitting || !isAuthenticated}
                onClick={handleSubmitOffering}
              >
                {isSubmitting ? "Submitting..." : "Add Offering"}
              </Button>
            </div>
          </Card>
        )}
      </div>

      <Card className="p-6 bg-white/10 backdrop-blur-sm rounded-lg border border-white/20">
        <div className="flex justify-between items-center mb-4">
          <h3 className="text-xl font-semibold text-white">
            Your Current Offerings
          </h3>
          <div className="text-xs text-white/70 bg-blue-500/20 px-3 py-1 rounded-full">
            {offerings.length} offerings
          </div>
        </div>

        {offerings.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="w-full text-white">
              <thead>
                <tr className="border-b border-white/20">
                  <th className="py-3 px-4 text-left">Name</th>
                  <th className="py-3 px-4 text-left">Type</th>
                  <th className="py-3 px-4 text-left">Price</th>
                  <th className="py-3 px-4 text-left">Location</th>
                  <th className="py-3 px-4 text-left">Status</th>
                  <th className="py-3 px-4 text-left">Actions</th>
                </tr>
              </thead>
              <tbody>
                {offerings.map((offering) => (
                  <tr
                    key={offering.id}
                    className="border-b border-white/10 hover:bg-white/5"
                  >
                    <td className="py-3 px-4">{offering.name}</td>
                    <td className="py-3 px-4">{offering.type}</td>
                    <td className="py-3 px-4 text-blue-400">
                      {offering.price}
                    </td>
                    <td className="py-3 px-4">{offering.location}</td>
                    <td className="py-3 px-4">
                      <span className="px-2 py-1 rounded-full text-xs bg-green-500/20 text-green-400">
                        {offering.status}
                      </span>
                    </td>
                    <td className="py-3 px-4">
                      <div className="flex space-x-2">
                        <button
                          className="p-1.5 rounded bg-blue-500/20 text-blue-400 hover:bg-blue-500/40 transition-colors"
                          title="Edit offering"
                        >
                          <FontAwesomeIcon icon={faEdit} />
                        </button>
                        <button
                          className="p-1.5 rounded bg-red-500/20 text-red-400 hover:bg-red-500/40 transition-colors"
                          title="Delete offering"
                          onClick={() => handleDeleteOffering(offering.id)}
                        >
                          <FontAwesomeIcon icon={faTrash} />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="text-center py-8 text-white/70">
            <p>You haven't added any offerings yet.</p>
            <Button
              className="mt-4 bg-blue-600 hover:bg-blue-700 text-white"
              onClick={() => setShowOfferingForm(true)}
              disabled={!isAuthenticated}
            >
              Add Your First Offering
            </Button>
            {!isAuthenticated && (
              <p className="mt-3 text-xs text-yellow-400">
                Please log in with a seed-phrase based identity to add
                offerings.
              </p>
            )}
          </div>
        )}
      </Card>
    </div>
  );
}
