import { useState } from "react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faBars,
  faSearch,
  faServer,
  faTrophy,
  faChartLine,
} from "@fortawesome/free-solid-svg-icons";
import Link from "next/link";
import { AuthButtons } from "@/components/auth-buttons";
import { useAuth } from "@/lib/auth-context";

const Navbar = () => {
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const { isAuthenticated } = useAuth();

  const toggleMenu = () => {
    setIsMenuOpen(!isMenuOpen);
  };

  return (
    <header className="fixed top-0 left-0 w-full bg-transparent md:bg-gradient-to-b md:from-black/80 md:to-transparent md:backdrop-blur-lg z-50 py-4 px-6 sm:px-8 md:px-12 lg:px-24 xl:px-36 flex justify-between items-center transition-all duration-300">
      {/* Logo */}
      <h1 className="hidden md:block whitespace-nowrap text-base sm:text-lg md:text-xl lg:text-2xl xl:text-3xl font-bold text-white tracking-wide">
        Decent Cloud
      </h1>

      {/* Navigation Links */}
      <nav className="hidden text-sm md:flex items-center text-white text-base sm:text-base md:text-base lg:text-lg xl:text-xl font-medium tracking-wide sm:tracking-normal">
        <ul className="flex flex-col md:flex-row space-y-4 md:space-y-0 md:space-x-4 lg:space-x-6 xl:space-x-8">
          {isAuthenticated && (
            <>
              <li>
                <Link
                  href="/dashboard"
                  className="flex items-center gap-1 sm:gap-2 hover:text-blue-400 transition duration-300 whitespace-nowrap"
                >
                  <FontAwesomeIcon icon={faChartLine} className="text-sm" />
                  <span>Dashboard</span>
                </Link>
              </li>
              <li>
                <Link
                  href="/dashboard/marketplace"
                  className="flex items-center gap-1 sm:gap-2 hover:text-blue-400 transition duration-300 whitespace-nowrap"
                >
                  <FontAwesomeIcon icon={faSearch} className="text-sm" />
                  <span>Marketplace</span>
                </Link>
              </li>
              <li>
                <Link
                  href="/dashboard/offerings"
                  className="flex items-center gap-1 sm:gap-2 hover:text-blue-400 transition duration-300 whitespace-nowrap"
                >
                  <FontAwesomeIcon icon={faServer} className="text-sm" />
                  <span>My Offerings</span>
                </Link>
              </li>
              <li>
                <Link
                  href="/dashboard/validators"
                  className="flex items-center gap-1 sm:gap-2 hover:text-blue-400 transition duration-300 whitespace-nowrap"
                >
                  <FontAwesomeIcon icon={faTrophy} className="text-sm" />
                  <span>Validators</span>
                </Link>
              </li>
            </>
          )}
          {!isAuthenticated && (
            <>
              <li>
                <Link
                  href="#features"
                  className="hover:text-blue-400 transition duration-300 whitespace-nowrap"
                >
                  Features
                </Link>
              </li>
              <li>
                <Link
                  href="#info"
                  className="hover:text-blue-400 transition duration-300 whitespace-nowrap"
                >
                  About
                </Link>
              </li>
              <li>
                <Link
                  href="#benefits"
                  className="hover:text-blue-400 transition duration-300 whitespace-nowrap"
                >
                  Benefits
                </Link>
              </li>
              <li>
                <Link
                  href="/dashboard"
                  className="hover:text-blue-400 transition duration-300 whitespace-nowrap"
                >
                  Dashboard
                </Link>
              </li>
            </>
          )}
        </ul>
      </nav>

      {/* Auth Buttons */}
      <div className="hidden md:block">
        <AuthButtons />
      </div>

      {/* Mobile Menu Button */}
      <div className="md:hidden">
        <button className="text-white focus:outline-none" onClick={toggleMenu}>
          <FontAwesomeIcon icon={faBars} size="lg" />
        </button>
      </div>

      {/* Mobile Menu */}
      <div
        className={`fixed top-12 right-0 bg-black/90 backdrop-blur-md w-full max-h-[80vh] overflow-y-auto md:hidden flex flex-col items-center gap-6 py-16 text-white text-xl transition-all duration-300 z-40 ${
          isMenuOpen ? "block" : "hidden"
        }`}
      >
        {!isAuthenticated && (
          <>
            <Link
              href="#features"
              className="hover:text-blue-400 transition duration-300"
              onClick={toggleMenu}
            >
              Features
            </Link>
            <Link
              href="#info"
              className="hover:text-blue-400 transition duration-300"
              onClick={toggleMenu}
            >
              About
            </Link>
            <Link
              href="#benefits"
              className="hover:text-blue-400 transition duration-300"
              onClick={toggleMenu}
            >
              Benefits
            </Link>
            <Link
              href="/dashboard"
              className="hover:text-blue-400 transition duration-300"
              onClick={toggleMenu}
            >
              Dashboard
            </Link>
          </>
        )}

        {isAuthenticated && (
          <>
            <Link
              href="/dashboard"
              className="flex items-center gap-2 hover:text-blue-400 transition duration-300"
              onClick={toggleMenu}
            >
              <FontAwesomeIcon icon={faChartLine} />
              <span>Dashboard</span>
            </Link>
            <Link
              href="/dashboard/marketplace"
              className="flex items-center gap-2 hover:text-blue-400 transition duration-300"
              onClick={toggleMenu}
            >
              <FontAwesomeIcon icon={faSearch} />
              <span>Marketplace</span>
            </Link>
            <Link
              href="/dashboard/offerings"
              className="flex items-center gap-2 hover:text-blue-400 transition duration-300"
              onClick={toggleMenu}
            >
              <FontAwesomeIcon icon={faServer} />
              <span>My Offerings</span>
            </Link>
            <Link
              href="/dashboard/validators"
              className="flex items-center gap-2 hover:text-blue-400 transition duration-300"
              onClick={toggleMenu}
            >
              <FontAwesomeIcon icon={faTrophy} />
              <span>Validators</span>
            </Link>
          </>
        )}
        <AuthButtons />
      </div>
    </header>
  );
};

export default Navbar;
