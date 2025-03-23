"use client";

import Navbar from "@/components/navbar";
import HeroSection from "@/components/hero-section";
import Footer from "@/components/footer";
import DashboardSection from "@/components/dashboard-section";
import FeaturesSection from "@/components/features-section";
import InfoSection from "@/components/info-section";
import BenefitsSection from "@/components/benefits-section";

interface DashboardData {
  dctPrice: number;
  providerCount: number;
  totalBlocks: number;
  blocksUntilHalving: number;
  validatorCount: number;
  blockReward: number;
  userIcpBalance?: number;
  userCkUsdcBalance?: number;
  userCkUsdtBalance?: number;
  userDctBalance?: number;
}

interface PageProps {
  dashboardData: DashboardData | null;
}

export function Page({ dashboardData }: PageProps) {
  return (
    <>
      <div className="min-h-screen bg-animated text-white  px-6 sm:px-8 md:px-12 lg:px-22 xl:px-30">
        <Navbar />
        <HeroSection />

        <main className="container mx-auto py-10">
          {dashboardData && <DashboardSection dashboardData={dashboardData} />}

          <FeaturesSection />

          <InfoSection />

          <BenefitsSection />
        </main>
      </div>
      <Footer />
    </>
  );
}
