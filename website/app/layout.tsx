import "./globals.css";
import { Inter } from "next/font/google";
import { AuthProvider } from "@/lib/auth-context";
import { GlobalLedgerService } from "@/components/global-ledger-service";

const inter = Inter({ subsets: ["latin"] });

export const metadata = {
  title: "Decent Cloud - Decentralized Cloud Platform",
  description:
    "A novel decentralized cloud platform bridging the gap between traditional centralized cloud services and fully decentralized systems.",
  icons: {
    icon: [{ url: "/favicon.svg", type: "image/svg+xml" }],
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className={inter.className}>
        <AuthProvider>
          <GlobalLedgerService />
          {children}
        </AuthProvider>
      </body>
    </html>
  );
}
