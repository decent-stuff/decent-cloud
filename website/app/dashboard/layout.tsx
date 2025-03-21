"use client";

// import { useAuth } from '@/lib/auth-context';
import Link from "next/link";
import { usePathname } from "next/navigation";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faSearch,
  faServer,
  faTrophy,
  faHome,
  faChartLine,
} from "@fortawesome/free-solid-svg-icons";

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const pathname = usePathname();

  const isActive = (path: string) => {
    return pathname === path;
  };

  return (
    <div className="min-h-screen bg-animated text-white">
      {/* Dashboard Navigation */}
      <div className="bg-black/80 backdrop-blur-lg border-b border-white/10">
        <div className="container mx-auto px-4 py-4">
          <div className="flex items-center justify-between">
            <Link
              href="/"
              className="text-white hover:text-blue-400 transition-colors flex items-center gap-2"
            >
              <FontAwesomeIcon icon={faHome} />
              <span className="font-medium">Back to Home</span>
            </Link>

            <div className="w-24">{/* Spacer for alignment */}</div>
          </div>
        </div>
      </div>

      {/* Dashboard Tabs */}
      <div className="bg-black/60 backdrop-blur-sm border-b border-white/10">
        <div className="container mx-auto px-4">
          <div className="flex overflow-x-auto">
            <Link
              href="/dashboard"
              className={`flex items-center gap-2 px-4 py-3 border-b-2 whitespace-nowrap ${
                isActive("/dashboard") &&
                !isActive("/dashboard/marketplace") &&
                !isActive("/dashboard/offerings") &&
                !isActive("/dashboard/validators")
                  ? "border-blue-400 text-blue-400"
                  : "border-transparent text-white/70 hover:text-white"
              } transition-colors`}
            >
              <FontAwesomeIcon icon={faChartLine} />
              <span>Overview</span>
            </Link>

            <Link
              href="/dashboard/marketplace"
              className={`flex items-center gap-2 px-4 py-3 border-b-2 whitespace-nowrap ${
                isActive("/dashboard/marketplace")
                  ? "border-blue-400 text-blue-400"
                  : "border-transparent text-white/70 hover:text-white"
              } transition-colors`}
            >
              <FontAwesomeIcon icon={faSearch} />
              <span>Marketplace</span>
            </Link>

            <Link
              href="/dashboard/offerings"
              className={`flex items-center gap-2 px-4 py-3 border-b-2 whitespace-nowrap ${
                isActive("/dashboard/offerings")
                  ? "border-blue-400 text-blue-400"
                  : "border-transparent text-white/70 hover:text-white"
              } transition-colors`}
            >
              <FontAwesomeIcon icon={faServer} />
              <span>My Offerings</span>
            </Link>

            <Link
              href="/dashboard/validators"
              className={`flex items-center gap-2 px-4 py-3 border-b-2 whitespace-nowrap ${
                isActive("/dashboard/validators")
                  ? "border-blue-400 text-blue-400"
                  : "border-transparent text-white/70 hover:text-white"
              } transition-colors`}
            >
              <FontAwesomeIcon icon={faTrophy} />
              <span>Validate</span>
            </Link>
          </div>
        </div>
      </div>

      {/* Dashboard Content */}
      <main>{children}</main>
    </div>
  );
}
