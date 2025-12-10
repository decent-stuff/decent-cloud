-- Remove pdf_blob from invoices table - PDFs now stored on disk
-- SQLite doesn't support DROP COLUMN directly, need to recreate table

-- Create new table without pdf_blob
CREATE TABLE invoices_new (
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
    pdf_generated_at_ns INTEGER,
    created_at_ns INTEGER NOT NULL,
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id)
);

-- Copy data (excluding pdf_blob)
INSERT INTO invoices_new (id, contract_id, invoice_number, invoice_date_ns, seller_name, seller_address, seller_vat_id, buyer_name, buyer_address, buyer_vat_id, subtotal_e9s, vat_rate_percent, vat_amount_e9s, total_e9s, currency, pdf_generated_at_ns, created_at_ns)
SELECT id, contract_id, invoice_number, invoice_date_ns, seller_name, seller_address, seller_vat_id, buyer_name, buyer_address, buyer_vat_id, subtotal_e9s, vat_rate_percent, vat_amount_e9s, total_e9s, currency, pdf_generated_at_ns, created_at_ns
FROM invoices;

-- Drop old table and rename new one
DROP TABLE invoices;
ALTER TABLE invoices_new RENAME TO invoices;

-- Recreate indexes
CREATE INDEX idx_invoices_contract_id ON invoices(contract_id);
CREATE INDEX idx_invoices_invoice_number ON invoices(invoice_number);
CREATE INDEX idx_invoices_created_at ON invoices(created_at_ns);
