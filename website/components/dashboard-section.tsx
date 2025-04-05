import { motion } from "framer-motion";
import HeaderSection from "@/components/ui/header";
import React from "react";

interface DashboardData {
  dctPrice: number;
  providerCount: number;
  totalBlocks: number;
  blocksUntilHalving: number;
  validatorCount: number;
  blockReward: number;
}

interface DashboardSectionProps {
  dashboardData: DashboardData;
}

interface DashboardItem {
  title: string;
  key: keyof DashboardData;
  format: (value: number | undefined) => string;
  tooltip: string;
}

const dashboardItems: DashboardItem[] = [
  {
    title: "Latest DCT Price 💎",
    key: "dctPrice",
    format: (value: number | undefined) =>
      value ? `$${value.toFixed(4)}` : "$0.0000",
    tooltip:
      "Our token is like a digital diamond: rare, valuable, and utterly decent! Plus, the price updates live from KongSwap.io!",
  },
  {
    title: "Provider Squad 🤝",
    key: "providerCount",
    format: (value: number | undefined) =>
      value ? `${value} providers` : "0 providers",
    tooltip: "Our awesome providers making the cloud decent again!",
  },
  {
    title: "Block Party 🎉",
    key: "totalBlocks",
    format: (value: number | undefined) =>
      value ? value.toLocaleString() : "0",
    tooltip: "Blocks validated and counting!",
  },
  {
    title: "Blocks Until Next Halving ⏳",
    key: "blocksUntilHalving",
    format: (value: number | undefined) =>
      value ? value.toLocaleString() : "0",
    tooltip: "Blocks until rewards halve!",
  },
  {
    title: "Current Block Validators 🛡️",
    key: "validatorCount",
    format: (value: number | undefined) =>
      value ? `${value} validators` : "0 validators",
    tooltip: "Validators keeping us decent!",
  },
  {
    title: "Accumulated Block Rewards 🎁",
    key: "blockReward",
    format: (value: number | undefined) =>
      value ? `${value.toFixed(2)} DCT` : "0.00 DCT",
    tooltip: "DCT per validated block! With carry-over if unclaimed!",
  },
];

const DashboardSection: React.FC<DashboardSectionProps> = ({
  dashboardData,
}) => {
  return (
    <section id="dashboard">
      <HeaderSection
        title="Dashboard"
        subtitle="Get a quick overview of Decent Cloud's current stats."
      />
      <motion.div
        className="max-w-4xl mx-auto p-6 grid grid-cols-1 sm:grid-cols-2 gap-4"
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true, amount: 0.4 }}
        transition={{ duration: 0.5 }}
      >
        {dashboardItems.map((item, index) => (
          <div
            key={index}
            className="border border-white/10 group relative flex flex-col bg-gradient-to-r from-gray-800/30 to-gray-700/30 rounded-xl p-3 sm:p-4 hover:bg-white/10 hover:shadow-xl transition duration-300 ease-in-out shadow-lg cursor-help"
          >
            <div className="font-semibold text-center text-white/90 text-lg sm:text-xl tracking-wide">
              {item.title}
            </div>
            <div className="text-blue-400 font-bold text-xl sm:text-2xl text-center mt-2">
              {item.format(dashboardData[item.key])}
            </div>

            {/* Tooltip */}
            <div
              className="absolute opacity-0 group-hover:opacity-100 transition-opacity duration-300 bg-gray-900 text-white text-xs rounded-lg p-3 shadow-xl border border-white/10
                 left-1/2 transform -translate-x-1/2 top-full mt-2 w-56 z-50 pointer-events-none"
            >
              {item.tooltip}
            </div>
          </div>
        ))}
      </motion.div>
    </section>
  );
};

export default DashboardSection;
