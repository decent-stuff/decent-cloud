'use client';

import { useState, useEffect } from 'react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faPlus, faEdit, faTrash } from '@fortawesome/free-solid-svg-icons';
import HeaderSection from '@/components/ui/header';
import { Card } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { useAuth } from '@/lib/auth-context';
import { useRouter } from 'next/navigation';

// Mock data for demonstration purposes
const mockOfferings = [
  {
    id: '1',
    name: 'High Performance Computing Node',
    type: 'Compute',
    price: '0.05 DCT/hour',
    specs: '8 vCPUs, 16GB RAM, 100GB SSD',
    location: 'Europe',
    status: 'Active',
    created: '2025-01-15',
  },
  {
    id: '2',
    name: 'Storage Solution',
    type: 'Storage',
    price: '0.02 DCT/GB/month',
    specs: 'Redundant storage, 99.9% uptime',
    location: 'North America',
    status: 'Active',
    created: '2025-01-20',
  },
];

export default function OfferingsPage() {
  const [offerings, setOfferings] = useState(mockOfferings);
  const { isAuthenticated } = useAuth();
  const router = useRouter();
  const [isLoading, setIsLoading] = useState(true);

  // Check authentication status
  useEffect(() => {
    // Add a small delay to ensure auth state is properly loaded
    const timer = setTimeout(() => {
      setIsLoading(false);
      if (!isAuthenticated) {
        console.log("User not authenticated, redirecting to home page");
        router.push('/');
      } else {
        console.log("User authenticated, staying on offerings page");
      }
    }, 500);

    return () => clearTimeout(timer);
  }, [isAuthenticated, router]);

  const handleDeleteOffering = (id: string) => {
    setOfferings(offerings.filter(offering => offering.id !== id));
  };

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-white text-xl">Loading offerings...</div>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8">
      <HeaderSection
        title="My Offerings"
        subtitle="Manage your cloud offerings on the Decent Cloud marketplace"
      />

        <div className="bg-white/10 p-6 rounded-lg backdrop-blur-sm mb-6">
          <div className="mb-6">
            <h3 className="text-xl font-semibold mb-2 text-white">Provider Dashboard</h3>
            <p className="text-white/90 mb-4">
              Add, edit, and manage your cloud offerings on the Decent Cloud marketplace.
            </p>
            <div className="bg-blue-900/30 p-4 rounded-lg mb-4">
              <p className="text-yellow-300 font-medium mb-2">⚠️ Example Data</p>
              <p className="text-white/90 text-sm">
                This is a demonstration page with mock data. In the future, providers will be able to add real offerings to the marketplace.
              </p>
            </div>
          </div>

          <div className="flex justify-end mb-6">
            <Button className="bg-green-600 hover:bg-green-700 text-white flex items-center gap-2">
              <FontAwesomeIcon icon={faPlus} />
              <span>Add New Offering</span>
            </Button>
          </div>
        </div>

        <Card className="p-6 bg-white/10 backdrop-blur-sm rounded-lg border border-white/20">
          <div className="flex justify-between items-center mb-4">
            <h3 className="text-xl font-semibold text-white">Your Current Offerings</h3>
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
                    <tr key={offering.id} className="border-b border-white/10 hover:bg-white/5">
                      <td className="py-3 px-4">{offering.name}</td>
                      <td className="py-3 px-4">{offering.type}</td>
                      <td className="py-3 px-4 text-blue-400">{offering.price}</td>
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
              <Button className="mt-4 bg-blue-600 hover:bg-blue-700 text-white">
                Add Your First Offering
              </Button>
            </div>
          )}
        </Card>
      </div>
  );
}
