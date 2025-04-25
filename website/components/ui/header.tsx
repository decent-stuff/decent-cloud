import React from "react";

interface HeaderSectionProps {
  title: string;
  subtitle?: string;
}

export default function HeaderSection({ title, subtitle }: HeaderSectionProps) {
  return (
    <div className="mb-8">
      <h1 className="text-3xl font-bold text-white mb-2">{title}</h1>
      {subtitle && <p className="text-white/80">{subtitle}</p>}
    </div>
  );
}
