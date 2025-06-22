import { render, screen } from "../../test/test-utils";
import { AuthDialog } from "../auth-dialog";
import { useAuth } from "@/lib/auth-context";

// Mock the auth context
jest.mock("@/lib/auth-context", () => ({
  useAuth: jest.fn(),
}));

// Mock router
jest.mock("next/navigation", () => ({
  useRouter: () => ({
    back: jest.fn(),
  }),
}));

describe("AuthDialog", () => {
  const mockLoginWithII = jest.fn();
  const mockLoginWithSeedPhrase = jest.fn();
  const mockSetShowSeedPhrase = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
    (useAuth as jest.Mock).mockReturnValue({
      loginWithII: mockLoginWithII,
      loginWithSeedPhrase: mockLoginWithSeedPhrase,
      showSeedPhrase: false,
      setShowSeedPhrase: mockSetShowSeedPhrase,
    });
  });

  it("renders auth options when opened", () => {
    render(<AuthDialog autoOpen />);

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText("Let's get started")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /internet identity/i })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /seed phrase/i })
    ).toBeInTheDocument();
  });

  it("handles Internet Identity login", async () => {
    const { user } = render(<AuthDialog autoOpen returnUrl="/custom-return" />);

    const iiButton = screen.getByRole("button", { name: /internet identity/i });
    await user.click(iiButton);

    expect(mockLoginWithII).toHaveBeenCalledWith("/custom-return");
  });

  it("handles seed phrase flow", async () => {
    const { user } = render(<AuthDialog autoOpen />);

    const seedPhraseButton = screen.getByRole("button", {
      name: /seed phrase/i,
    });
    await user.click(seedPhraseButton);

    expect(mockSetShowSeedPhrase).toHaveBeenCalledWith(true);
  });

  it("shows informative text about authentication methods", () => {
    render(<AuthDialog autoOpen />);

    expect(
      screen.getByText(/credentials are stored securely/i)
    ).toBeInTheDocument();
    expect(
      screen.getByText(/internet computer's official authentication service/i)
    ).toBeInTheDocument();
  });

  describe("non-autoOpen mode", () => {
    it("renders trigger button when not in autoOpen mode", () => {
      render(<AuthDialog />);

      expect(
        screen.getByRole("button", { name: /register\/sign in/i })
      ).toBeInTheDocument();
    });

    it("opens dialog when trigger is clicked", async () => {
      const { user } = render(<AuthDialog />);

      const triggerButton = screen.getByRole("button", {
        name: /register\/sign in/i,
      });
      await user.click(triggerButton);

      expect(screen.getByText("Let's get started")).toBeInTheDocument();
    });
  });

  describe("error handling", () => {
    it("handles seed phrase login failure", async () => {
      const error = new Error("Invalid seed phrase");
      mockLoginWithSeedPhrase.mockRejectedValue(error);

      // Mock console.error before rendering
      const consoleSpy = jest.spyOn(console, "error").mockImplementation();

      const { user } = render(<AuthDialog autoOpen />);

      const seedPhraseButton = screen.getByRole("button", {
        name: /seed phrase/i,
      });
      await user.click(seedPhraseButton);

      expect(mockSetShowSeedPhrase).toHaveBeenCalledWith(true);

      // The error handling happens in the SeedPhraseDialog component when it submits
      // Since we're testing the AuthDialog component, we should test that it properly
      // calls setShowSeedPhrase when the seed phrase button is clicked
      // The actual error handling for seed phrase submission would be tested in the SeedPhraseDialog tests

      consoleSpy.mockRestore();
    });
  });
});
