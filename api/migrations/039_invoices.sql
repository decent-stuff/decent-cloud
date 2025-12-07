-- Invoice storage and sequential numbering
-- PDF invoices generated on-demand with Typst

-- Create invoices table
CREATE TABLE invoices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL UNIQUE,
    invoice_number TEXT NOT NULL UNIQUE,
    invoice_date_ns INTEGER NOT NULL,
    seller_name TEXT NOT NULL,
    seller_address TEXT NOT NULL,
    seller_vat_id TEXT,
    buyer_name TEXT,
    buyer_address TEXT,
    buyer_vat_id TEXT,
    subtotal_e9s INTEGER NOT NULL,
    vat_rate_percent INTEGER NOT NULL DEFAULT 0,
    vat_amount_e9s INTEGER NOT NULL DEFAULT 0,
    total_e9s INTEGER NOT NULL,
    currency TEXT NOT NULL,
    pdf_blob BLOB,
    pdf_generated_at_ns INTEGER,
    created_at_ns INTEGER NOT NULL,
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id)
);

-- Create invoice_sequence table for sequential numbering per year (INV-YYYY-NNNNNN)
CREATE TABLE invoice_sequence (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    year INTEGER NOT NULL,
    next_number INTEGER NOT NULL DEFAULT 1
);

-- Initialize invoice sequence for current year
INSERT INTO invoice_sequence (id, year, next_number) VALUES (1, strftime('%Y', 'now'), 1);

-- Create indexes
CREATE INDEX idx_invoices_contract_id ON invoices(contract_id);
CREATE INDEX idx_invoices_invoice_number ON invoices(invoice_number);
CREATE INDEX idx_invoices_created_at ON invoices(created_at_ns);
