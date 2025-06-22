import { render, screen } from "../../test/test-utils";
import { SendFundsDialog } from "../send-funds-dialog";

describe("SendFundsDialog", () => {
  const defaultProps = {
    isOpen: true,
    onClose: jest.fn(),
    onSend: jest.fn(),
    tokenName: "TEST",
    maxAmount: 100,
  };

  beforeEach(() => {
    jest.clearAllMocks();
  });

  it("renders with correct token name and max amount", () => {
    render(<SendFundsDialog {...defaultProps} />);

    expect(screen.getByText("Send TEST")).toBeInTheDocument();
    expect(screen.getByText("Available: 100 TEST")).toBeInTheDocument();
  });

  it("validates required fields", async () => {
    const { user } = render(<SendFundsDialog {...defaultProps} />);

    const sendButton = screen.getByRole("button", { name: /send/i });
    await user.click(sendButton);

    expect(screen.getByText("Recipient is required")).toBeInTheDocument();
  });

  it("validates amount against maximum", async () => {
    const { user } = render(<SendFundsDialog {...defaultProps} />);

    const recipientInput = screen.getByLabelText(/recipient/i);
    const amountInput = screen.getByLabelText(/amount/i);
    const sendButton = screen.getByRole("button", { name: /send/i });

    await user.type(recipientInput, "test-recipient");
    await user.type(amountInput, "150");
    await user.click(sendButton);

    expect(
      screen.getByText("Amount exceeds your balance of 100 TEST")
    ).toBeInTheDocument();
  });

  it("calls onSend with correct values when form is valid", async () => {
    const { user } = render(<SendFundsDialog {...defaultProps} />);

    const recipientInput = screen.getByLabelText(/recipient/i);
    const amountInput = screen.getByLabelText(/amount/i);
    const sendButton = screen.getByRole("button", { name: /send/i });

    await user.type(recipientInput, "test-recipient");
    await user.type(amountInput, "50");
    await user.click(sendButton);

    expect(defaultProps.onSend).toHaveBeenCalledWith("test-recipient", "50");
    expect(defaultProps.onClose).toHaveBeenCalled();
  });

  it("sets maximum amount when max button is clicked", async () => {
    const { user } = render(<SendFundsDialog {...defaultProps} />);

    const maxButton = screen.getByRole("button", { name: /max/i });
    const amountInput = screen.getByLabelText(/amount/i);

    await user.click(maxButton);
    expect(amountInput).toHaveValue("100");
  });

  it("closes dialog when cancel button is clicked", async () => {
    const { user } = render(<SendFundsDialog {...defaultProps} />);

    const cancelButton = screen.getByRole("button", { name: /cancel/i });
    await user.click(cancelButton);

    expect(defaultProps.onClose).toHaveBeenCalled();
  });
});
