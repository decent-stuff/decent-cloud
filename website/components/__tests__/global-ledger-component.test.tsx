import { render, waitFor, act } from "@testing-library/react";
import { GlobalLedgerComponent } from "../global-ledger-component";
import { ledgerService } from "@/lib/ledger-service";

// Mock the ledger service
jest.mock("@/lib/ledger-service", () => ({
  ledgerService: {
    initialize: jest.fn(),
    setPollingInterval: jest.fn(),
    stopPolling: jest.fn(),
  },
}));

describe("GlobalLedgerComponent", () => {
  beforeEach(() => {
    jest.clearAllMocks();
    jest.useFakeTimers({ advanceTimers: true });
    // Setup successful initialization by default
    (ledgerService.initialize as jest.Mock).mockResolvedValue(true);
  });

  afterEach(() => {
    jest.clearAllTimers();
    jest.useRealTimers();
  });

  it("initializes ledger service and starts polling on mount", async () => {
    render(<GlobalLedgerComponent />);

    await waitFor(() => {
      expect(ledgerService.initialize).toHaveBeenCalledTimes(1);
    });

    await waitFor(() => {
      expect(ledgerService.setPollingInterval).toHaveBeenCalledWith(30000);
    });
  });

  it("stops polling on unmount", async () => {
    const { unmount } = render(<GlobalLedgerComponent />);

    await waitFor(() => {
      expect(ledgerService.initialize).toHaveBeenCalled();
    });

    unmount();
    expect(ledgerService.stopPolling).toHaveBeenCalled();
  });

  it("retries initialization on failure with exponential backoff", async () => {
    // Mock first attempt to fail, second to succeed
    (ledgerService.initialize as jest.Mock)
      .mockResolvedValueOnce(false)
      .mockResolvedValueOnce(true);

    render(<GlobalLedgerComponent />);

    // First attempt fails
    await waitFor(() => {
      expect(ledgerService.initialize).toHaveBeenCalledTimes(1);
    });

    // Wait for retry delay
    await act(async () => {
      jest.advanceTimersByTime(1000);
    });

    // Second attempt succeeds
    await waitFor(() => {
      expect(ledgerService.initialize).toHaveBeenCalledTimes(2);
      expect(ledgerService.setPollingInterval).toHaveBeenCalledWith(30000);
    });
  });

  it("handles initialization errors gracefully", async () => {
    const error = new Error("Initialization failed");
    (ledgerService.initialize as jest.Mock).mockRejectedValue(error);

    const consoleSpy = jest.spyOn(console, "error").mockImplementation();

    render(<GlobalLedgerComponent />);

    await waitFor(() => {
      expect(consoleSpy).toHaveBeenCalledWith(
        "Failed to initialize global ledger service:",
        error
      );
    });

    consoleSpy.mockRestore();
  });

  it("stops retrying after max attempts", async () => {
    // Mock all initialization attempts to fail
    (ledgerService.initialize as jest.Mock).mockResolvedValue(false);

    const { unmount } = render(<GlobalLedgerComponent />);

    // Initial attempt
    await waitFor(() => {
      expect(ledgerService.initialize).toHaveBeenCalledTimes(1);
    });

    // Manually advance through each retry delay to ensure all retries complete
    // 1st retry: 1000ms
    await act(async () => {
      jest.advanceTimersByTime(1000);
    });
    await waitFor(() => {
      expect(ledgerService.initialize).toHaveBeenCalledTimes(2);
    });

    // 2nd retry: 2000ms
    await act(async () => {
      jest.advanceTimersByTime(2000);
    });
    await waitFor(() => {
      expect(ledgerService.initialize).toHaveBeenCalledTimes(3);
    });

    // 3rd retry: 4000ms
    await act(async () => {
      jest.advanceTimersByTime(4000);
    });
    await waitFor(() => {
      expect(ledgerService.initialize).toHaveBeenCalledTimes(4);
    });

    // 4th retry: 8000ms
    await act(async () => {
      jest.advanceTimersByTime(8000);
    });
    await waitFor(() => {
      expect(ledgerService.initialize).toHaveBeenCalledTimes(5);
    });

    // 5th retry: 16000ms
    await act(async () => {
      jest.advanceTimersByTime(16000);
    });
    await waitFor(() => {
      expect(ledgerService.initialize).toHaveBeenCalledTimes(6);
    });

    // Advance time significantly to ensure no more retries occur
    await act(async () => {
      jest.advanceTimersByTime(60000);
    });

    // Should still be exactly 6 attempts (1 initial + 5 retries)
    expect(ledgerService.initialize).toHaveBeenCalledTimes(6);

    unmount();
  });
});
