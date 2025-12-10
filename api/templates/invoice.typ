// Invoice template - renders invoice data from JSON input
#let data = json(sys.inputs.data_file)

// Translations dictionary
#let translations = (
  en: (
    invoice: "INVOICE",
    issue-date: "ISSUE DATE",
    service-date: "SERVICE DATE",
    payment-status: "PAYMENT STATUS",
    from: "FROM",
    bill-to: "BILL TO",
    vat: "VAT",
    description: "DESCRIPTION",
    qty: "QTY",
    price: "PRICE",
    total: "TOTAL",
    subtotal: "Subtotal",
    vat-of: "VAT",
    bank-details: "BANK DETAILS",
    iban: "IBAN",
  ),
  de: (
    invoice: "RECHNUNG",
    issue-date: "RECHNUNGSDATUM",
    service-date: "LEISTUNGSDATUM",
    payment-status: "ZAHLUNGSSTATUS",
    from: "VON",
    bill-to: "RECHNUNG AN",
    vat: "USt-IdNr",
    description: "BESCHREIBUNG",
    qty: "MENGE",
    price: "PREIS",
    total: "GESAMT",
    subtotal: "Zwischensumme",
    vat-of: "MwSt",
    bank-details: "BANKVERBINDUNG",
    iban: "IBAN",
  ),
  fr: (
    invoice: "FACTURE",
    issue-date: "DATE D'ÉMISSION",
    service-date: "DATE DE SERVICE",
    payment-status: "STATUT DE PAIEMENT",
    from: "DE",
    bill-to: "FACTURER À",
    vat: "TVA",
    description: "DESCRIPTION",
    qty: "QTÉ",
    price: "PRIX",
    total: "TOTAL",
    subtotal: "Sous-total",
    vat-of: "TVA",
    bank-details: "COORDONNÉES BANCAIRES",
    iban: "IBAN",
  ),
  es: (
    invoice: "FACTURA",
    issue-date: "FECHA DE EMISIÓN",
    service-date: "FECHA DE SERVICIO",
    payment-status: "ESTADO DE PAGO",
    from: "DE",
    bill-to: "FACTURAR A",
    vat: "NIF/CIF",
    description: "DESCRIPCIÓN",
    qty: "CANT",
    price: "PRECIO",
    total: "TOTAL",
    subtotal: "Subtotal",
    vat-of: "IVA",
    bank-details: "DATOS BANCARIOS",
    iban: "IBAN",
  ),
)

// Get language from data or default to English
#let lang = data.at("language", default: "en")
#let t = translations.at(lang, default: translations.en)

// Currency symbol helper
#let currency-sym = {
  if data.currency == "EUR" { "€" }
  else if data.currency == "USD" { "$" }
  else if data.currency == "GBP" { "£" }
  else { data.currency + " " }
}

#set document(title: t.invoice + " " + data.invoice-id, date: auto)
#set page(margin: (top: 2.5cm, bottom: 2cm, left: 2.5cm, right: 2.5cm), paper: "a4")
#set text(font: "Liberation Sans", size: 10pt, fill: luma(30), lang: lang)

// Header with title
#align(center)[
  #block(width: 100%, fill: rgb("#1a365d"), inset: 1em, radius: 3pt)[
    #text(size: 24pt, weight: "bold", fill: white)[#t.invoice]
  ]
]

#v(1.5em)

// Invoice ID and dates
#align(center)[
  #text(size: 14pt, weight: "bold", fill: rgb("#1a365d"))[#data.invoice-id]
]

#v(1em)

#grid(
  columns: (1fr, 1fr, 1fr),
  gutter: 1em,
  align: center,
  [
    #text(size: 8pt, fill: luma(100))[#t.issue-date]
    #linebreak()
    #text(weight: "medium")[#data.issuing-date]
  ],
  [
    #text(size: 8pt, fill: luma(100))[#t.service-date]
    #linebreak()
    #text(weight: "medium")[#data.delivery-date]
  ],
  [
    #text(size: 8pt, fill: luma(100))[#t.payment-status]
    #linebreak()
    #text(weight: "medium")[#data.due-date]
  ],
)

#v(1.5em)
#line(length: 100%, stroke: 1pt + rgb("#e2e8f0"))
#v(1.5em)

// From and To sections
#grid(
  columns: (1fr, 1fr),
  gutter: 3em,
  [
    #text(size: 8pt, fill: luma(100), weight: "bold")[#t.from]
    #v(0.5em)
    #text(size: 11pt, weight: "bold", fill: rgb("#1a365d"))[#data.biller.name]
    #v(0.3em)
    #text(size: 9pt)[
      #data.biller.address.street
      #linebreak()
      #data.biller.address.postal-code #data.biller.address.city
      #linebreak()
      #data.biller.address.country
    ]
    #if data.biller.at("vat-id", default: none) != none [
      #v(0.3em)
      #text(size: 9pt, fill: luma(80))[#t.vat: #data.biller.vat-id]
    ]
  ],
  [
    #text(size: 8pt, fill: luma(100), weight: "bold")[#t.bill-to]
    #v(0.5em)
    #text(size: 11pt, weight: "bold", fill: rgb("#1a365d"))[#data.recipient.name]
    #v(0.3em)
    #text(size: 9pt)[
      #if data.recipient.address.street != "" [#data.recipient.address.street #linebreak()]
      #if data.recipient.address.postal-code != "" or data.recipient.address.city != "" [
        #data.recipient.address.postal-code #data.recipient.address.city #linebreak()
      ]
      #if data.recipient.address.country != "" [#data.recipient.address.country]
    ]
    #if data.recipient.at("vat-id", default: none) != none [
      #v(0.3em)
      #text(size: 9pt, fill: luma(80))[#t.vat: #data.recipient.vat-id]
    ]
  ],
)

#v(2em)

// Items table
#block(width: 100%, stroke: 1pt + rgb("#e2e8f0"), radius: 5pt, clip: true)[
  #table(
    columns: (auto, 1fr, auto, auto, auto),
    stroke: none,
    inset: (x: 12pt, y: 10pt),
    fill: (_, row) => if row == 0 { rgb("#f7fafc") } else if calc.rem(row, 2) == 0 { rgb("#fafafa") } else { white },
    table.header(
      [#text(size: 8pt, weight: "bold", fill: luma(80))[\#]],
      [#text(size: 8pt, weight: "bold", fill: luma(80))[#t.description]],
      [#text(size: 8pt, weight: "bold", fill: luma(80))[#t.qty]],
      [#text(size: 8pt, weight: "bold", fill: luma(80))[#t.price]],
      [#text(size: 8pt, weight: "bold", fill: luma(80))[#t.total]],
    ),
    ..data.items.enumerate().map(((i, item)) => (
      text(weight: "medium")[#str(i + 1)],
      text(size: 9pt)[#item.description],
      align(center)[#str(item.quantity)],
      align(right)[#currency-sym#calc.round(item.price, digits: 2)],
      align(right)[#text(weight: "medium")[#currency-sym#calc.round(item.price * item.quantity, digits: 2)]],
    )).flatten()
  )
]

#v(1em)

// Totals
#let subtotal = data.items.map(item => item.price * item.quantity).sum()
#let vat-amount = subtotal * data.vat / 100
#let total = subtotal + vat-amount

#align(right)[
  #block(width: 45%, stroke: 1pt + rgb("#e2e8f0"), radius: 5pt, inset: 1em)[
    #grid(
      columns: (1fr, auto),
      row-gutter: 8pt,
      [#text(fill: luma(80))[#t.subtotal]], [#text(weight: "medium")[#currency-sym#calc.round(subtotal, digits: 2)]],
      [#text(fill: luma(80))[#t.vat-of (#data.vat%)]], [#text(weight: "medium")[#currency-sym#calc.round(vat-amount, digits: 2)]],
    )
    #v(0.5em)
    #line(length: 100%, stroke: 1pt + rgb("#e2e8f0"))
    #v(0.5em)
    #grid(
      columns: (1fr, auto),
      [#text(size: 12pt, weight: "bold", fill: rgb("#1a365d"))[#t.total]],
      [#text(size: 12pt, weight: "bold", fill: rgb("#1a365d"))[#currency-sym#calc.round(total, digits: 2)]],
    )
  ]
]

// Bank details
#v(2em)
#if data.biller.at("iban", default: "") != "" [
  #block(fill: rgb("#f7fafc"), inset: 1em, radius: 5pt, width: 100%)[
    #text(size: 8pt, weight: "bold", fill: luma(80))[#t.bank-details]
    #v(0.3em)
    #text(size: 9pt)[#t.iban: #data.biller.iban]
  ]
]

// Note (passed through as-is, should be pre-translated by backend)
#if "note" in data and data.note != none [
  #v(1em)
  #block(fill: rgb("#fffbeb"), stroke: 1pt + rgb("#fbbf24"), inset: 1em, radius: 5pt, width: 100%)[
    #text(size: 9pt, fill: rgb("#92400e"))[#data.note]
  ]
]
