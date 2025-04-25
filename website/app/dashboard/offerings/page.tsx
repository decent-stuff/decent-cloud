"use client";

import { useState, useEffect } from "react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faPlus, faEdit, faTrash } from "@fortawesome/free-solid-svg-icons";
import HeaderSection from "@/components/ui/header";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { useAuth, AuthenticatedIdentityResult } from "@/lib/auth-context";
import OfferingForm from "@/components/forms/offering-form";

// Offerings page for managing cloud offerings

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
  const [authIdentity, setAuthIdentity] =
    useState<AuthenticatedIdentityResult | null>(null);
  const { isAuthenticated, getAuthenticatedIdentity } = useAuth();

  // Get authenticated identity when needed
  useEffect(() => {
    if (isAuthenticated && showOfferingForm) {
      const fetchIdentity = async () => {
        try {
          const identity = await getAuthenticatedIdentity();
          setAuthIdentity(identity);
        } catch (error) {
          console.error("Failed to get authenticated identity", error);
        }
      };

      // Call the async function and explicitly ignore the promise
      void fetchIdentity();
    }
  }, [isAuthenticated, showOfferingForm, getAuthenticatedIdentity]);

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
          <div className="mb-6">
            <OfferingForm
              onSubmitSuccess={() => {
                setShowOfferingForm(false);
                // We would normally parse the offeringData here and add it to the list
                // but we'll handle that with a data fetching system in the future
              }}
              onCancel={() => setShowOfferingForm(false)}
              authResult={authIdentity}
            />
          </div>
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
              disabled={!isAuthenticated || showOfferingForm}
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
