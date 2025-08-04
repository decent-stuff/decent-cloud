use dcc_common::{PaymentEntries, PaymentEntry, PaymentEntryWithAmount, TokenAmountE9s};
use provider_offering::ServerOffering;
use std::collections::HashMap;

pub fn prompt_for_payment_entries(
    payment_entries_json: &Option<PaymentEntries>,
    offering: &ServerOffering,
    instance_id: &str,
) -> Vec<PaymentEntryWithAmount> {
    let pricing: HashMap<String, HashMap<String, String>> = offering.instance_pricing(instance_id);

    let get_total_price = |model: &str, time_period_unit: &str, quantity: u64| -> TokenAmountE9s {
        pricing
            .get(model)
            .and_then(|units| units.get(time_period_unit))
            .map(|amount| {
                amount
                    .replace("_", "")
                    .parse::<TokenAmountE9s>()
                    .expect("Failed to parse the offering price as TokenAmountE9s")
                    * quantity
            })
            .unwrap()
    };
    let mut payment_entries: Vec<_> = payment_entries_json
        .clone()
        .map(|entries| {
            entries
                .0
                .into_iter()
                .map(|e| PaymentEntryWithAmount {
                    e: e.clone(),
                    amount_e9s: get_total_price(&e.pricing_model, &e.time_period_unit, e.quantity),
                })
                .collect()
        })
        .unwrap_or_default();

    if payment_entries.is_empty() {
        let models = pricing.keys().collect::<Vec<_>>();
        let model = models[dialoguer::Select::new()
            .with_prompt("Please select instance pricing model (ESC to exit)")
            .items(&models)
            .default(0)
            .interact()
            .expect("Failed to read input")];
        let units = pricing[model].keys().collect::<Vec<_>>();
        let time_period_unit = units[dialoguer::Select::new()
            .with_prompt("Please select time period unit")
            .items(&units)
            .report(true)
            .default(0)
            .interact()
            .expect("Failed to read input")];
        let quantity = dialoguer::Input::<u64>::new()
            .with_prompt("Please enter the number of units")
            .default(1)
            .interact()
            .expect("Failed to read input");
        payment_entries.push(PaymentEntryWithAmount {
            e: PaymentEntry::new(model, time_period_unit, quantity),
            amount_e9s: get_total_price(model, time_period_unit, quantity),
        });
    }
    payment_entries
}
