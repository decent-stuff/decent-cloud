import "./globals.css";
import { AuthProvider } from "../lib/auth-context";
import { ToastProvider } from "../components/ui/use-toast";
import { GlobalLedgerComponent as GlobalLedgerComponent } from "@/components/global-ledger-component";

export const metadata = {
  title: "Decent Cloud - Decentralized Cloud Computing",
  description: "Decentralized cloud computing platform powered by blockchain",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="min-h-screen bg-gradient-to-b from-blue-900 to-black text-white">
        <AuthProvider>
          <GlobalLedgerComponent />
          <ToastProvider>{children}</ToastProvider>
        </AuthProvider>
      </body>
    </html>
  );
}
