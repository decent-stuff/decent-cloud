#import "@preview/invoice-maker:1.1.0": *

// Parse JSON data from file path passed via --input data_file=path
#let invoice_data = json(sys.inputs.data_file)

// Build invoice using invoice-maker package
#invoice(
  language: invoice_data.language,
  invoice-id: invoice_data.invoice-id,
  issuing-date: invoice_data.issuing-date,
  delivery-date: invoice_data.delivery-date,
  due-date: invoice_data.due-date,

  biller: (
    name: invoice_data.biller.name,
    address: (
      street: invoice_data.biller.address.street,
      city: invoice_data.biller.address.city,
      postal-code: invoice_data.biller.address.postal-code,
      country: invoice_data.biller.address.country,
    ),
    vat-id: invoice_data.biller.at("vat-id", default: none),
  ),

  recipient: (
    name: invoice_data.recipient.name,
    address: (
      street: invoice_data.recipient.address.at("street", default: ""),
      city: invoice_data.recipient.address.at("city", default: ""),
      postal-code: invoice_data.recipient.address.at("postal-code", default: ""),
      country: invoice_data.recipient.address.at("country", default: ""),
    ),
    vat-id: invoice_data.recipient.at("vat-id", default: none),
  ),

  items: invoice_data.items.map(item => (
    number: item.number,
    description: item.description,
    quantity: item.quantity,
    price: item.price,
  )),

  vat: invoice_data.vat,
  currency: invoice_data.currency,
)

// Add note if provided (e.g., VAT exemption notice)
#if "note" in invoice_data and invoice_data.note != none {
  v(1em)
  line(length: 100%, stroke: 0.5pt + gray)
  v(0.5em)
  text(size: 9pt, fill: rgb("#555"))[#invoice_data.note]
}
