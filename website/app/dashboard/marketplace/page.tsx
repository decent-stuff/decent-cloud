'use client';

import { useState, useEffect } from 'react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faSearch } from '@fortawesome/free-solid-svg-icons';
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
    provider: 'CloudNode Provider',
    price: '0.05 DCT/hour',
    specs: '8 vCPUs, 16GB RAM, 100GB SSD',
    location: 'Europe',
    rating: 4.8,
  },
  {
    id: '2',
    name: 'Storage Solution',
    provider: 'DecentStore',
    price: '0.02 DCT/GB/month',
    specs: 'Redundant storage, 99.9% uptime',
    location: 'North America',
    rating: 4.6,
  },
  {
    id: '3',
    name: 'Web Hosting',
    provider: 'DecentralHost',
    price: '10 DCT/month',
    specs: 'Unlimited bandwidth, 10GB storage',
    location: 'Asia',
    rating: 4.7,
  },
  {
    id: '4',
    name: 'AI Compute Cluster',
    provider: 'NeuralNode',
    price: '0.1 DCT/hour',
    specs: '4 GPUs, 64GB RAM, optimized for ML',
    location: 'Global',
    rating: 4.9,
  },
];

export default function MarketplacePage() {
  const [searchTerm, setSearchTerm] = useState('');
  const [filteredOfferings, setFilteredOfferings] = useState(mockOfferings);
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
        console.log("User authenticated, staying on marketplace page");
      }
    }, 500);

    return () => clearTimeout(timer);
  }, [isAuthenticated, router]);

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    const filtered = mockOfferings.filter(
      (offering) =>
        offering.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        offering.provider.toLowerCase().includes(searchTerm.toLowerCase()) ||
        offering.specs.toLowerCase().includes(searchTerm.toLowerCase())
    );
    setFilteredOfferings(filtered);
  };

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-white text-xl">Loading marketplace...</div>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8">
      <HeaderSection
        title="Marketplace"
        subtitle="Search for available cloud offerings from decentralized providers"
      />

        <div className="bg-white/10 p-6 rounded-lg backdrop-blur-sm mb-6">
          <div className="mb-6">
            <h3 className="text-xl font-semibold mb-2 text-white">Find Cloud Resources</h3>
            <p className="text-white/90 mb-4">
              Browse through available offerings from decentralized providers on the Decent Cloud network.
            </p>
            <div className="bg-blue-900/30 p-4 rounded-lg mb-4">
              <p className="text-yellow-300 font-medium mb-2">⚠️ Example Data</p>
              <p className="text-white/90 text-sm">
                This is a demonstration page with mock data. In the future, this marketplace will contain real offerings from providers on the network.
              </p>
            </div>
          </div>

          <form onSubmit={handleSearch} className="mb-6">
            <div className="flex flex-col md:flex-row gap-4">
              <div className="flex-grow relative">
                <input
                  type="text"
                  placeholder="Search by name, provider, or specifications..."
                  className="w-full p-3 bg-white/20 border border-white/10 rounded-lg text-white placeholder-white/50 focus:outline-none focus:ring-2 focus:ring-blue-500"
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                />
                <FontAwesomeIcon icon={faSearch} className="absolute right-3 top-1/2 transform -translate-y-1/2 text-white/50" />
              </div>
              <Button type="submit" className="bg-blue-600 hover:bg-blue-700 text-white">
                Search
              </Button>
            </div>
          </form>
        </div>

        <Card className="p-6 bg-white/10 backdrop-blur-sm rounded-lg border border-white/20">
          <div className="flex justify-between items-center mb-4">
            <h3 className="text-xl font-semibold text-white">Available Offerings</h3>
            <div className="text-xs text-white/70 bg-blue-500/20 px-3 py-1 rounded-full">
              {filteredOfferings.length} results
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {filteredOfferings.map((offering) => (
              <div key={offering.id} className="border border-white/10 rounded-lg p-4 bg-white/5 hover:bg-white/10 transition-colors">
                <h4 className="text-lg font-medium text-white mb-2">{offering.name}</h4>
                <div className="flex justify-between mb-2">
                  <span className="text-white/70 text-sm">Provider:</span>
                  <span className="text-white font-medium">{offering.provider}</span>
                </div>
                <div className="flex justify-between mb-2">
                  <span className="text-white/70 text-sm">Price:</span>
                  <span className="text-blue-400 font-medium">{offering.price}</span>
                </div>
                <div className="flex justify-between mb-2">
                  <span className="text-white/70 text-sm">Specs:</span>
                  <span className="text-white">{offering.specs}</span>
                </div>
                <div className="flex justify-between mb-2">
                  <span className="text-white/70 text-sm">Location:</span>
                  <span className="text-white">{offering.location}</span>
                </div>
                <div className="flex justify-between mb-4">
                  <span className="text-white/70 text-sm">Rating:</span>
                  <span className="text-yellow-400">{'★'.repeat(Math.floor(offering.rating))} {offering.rating}</span>
                </div>
                <Button className="w-full bg-blue-600 hover:bg-blue-700 text-white">
                  View Details
                </Button>
              </div>
            ))}
          </div>
        </Card>
      </div>
  );
}
