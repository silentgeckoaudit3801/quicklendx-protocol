#[cfg(test)]
mod tests {
    use crate::invoice::InvoiceCategory;
    use crate::settlement::{get_settlement_summary, is_invoice_finalized};
    use crate::{QuickLendXContract, QuickLendXContractClient, QuickLendXError};
    use soroban_sdk::{testutils::{Address as _, Ledger}, token, Address, BytesN, Env, String, Vec};

    fn setup_funded_invoice(
        env: &Env,
        client: &QuickLendXContractClient,
        contract_id: &Address,
        invoice_amount: i128,
    ) -> BytesN<32> {
        let admin = Address::generate(env);
        let business = Address::generate(env);
        let investor = Address::generate(env);
        let token_admin = Address::generate(env);
        let currency = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();
        let token_client = token::Client::new(env, &currency);
        let sac_client = token::StellarAssetClient::new(env, &currency);
        let initial_balance = 50_000i128;

        sac_client.mint(&business, &initial_balance);
        sac_client.mint(&investor, &initial_balance);
        let expiration = env.ledger().sequence() + 10_000;
        token_client.approve(&business, contract_id, &initial_balance, &expiration);
        token_client.approve(&investor, contract_id, &initial_balance, &expiration);

        client.set_admin(&admin);
        client.submit_kyc_application(&business, &String::from_str(env, "business-kyc"));
        client.verify_business(&admin, &business);
        client.submit_investor_kyc(&investor, &String::from_str(env, "investor-kyc"));
        client.verify_investor(&investor, &initial_balance);

        let due_date = env.ledger().timestamp() + 86_400;
        let invoice_id = client.store_invoice(
            &business,
            &invoice_amount,
            &currency,
            &due_date,
            &String::from_str(env, "Invoice for settlement summary"),
            &InvoiceCategory::Services,
            &Vec::new(env),
        );
        client.verify_invoice(&invoice_id);
        let bid_id = client.place_bid(
            &investor,
            &invoice_id,
            &invoice_amount,
            &(invoice_amount + 100),
            &BytesN::from_array(env, &[0u8; 32]),
        );
        client.accept_bid(&invoice_id, &bid_id);
        invoice_id
    }

    #[test]
    fn settlement_summary_reports_missing_invoice_error() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(QuickLendXContract, ());
        let missing = BytesN::from_array(&env, &[7u8; 32]);

        let err = env
            .as_contract(&contract_id, || get_settlement_summary(&env, &missing))
            .unwrap_err();
        assert_eq!(err, QuickLendXError::InvoiceNotFound);
    }

    #[test]
    fn settlement_summary_tracks_unpaid_partial_overpay_and_finalized() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(QuickLendXContract, ());
        let client = QuickLendXContractClient::new(&env, &contract_id);
        let invoice_amount = 1_000i128;
        let invoice_id = setup_funded_invoice(&env, &client, &contract_id, invoice_amount);

        let unpaid = client.get_settlement_summary(&invoice_id).unwrap();
        assert_eq!(unpaid.total_due, invoice_amount);
        assert_eq!(unpaid.total_paid, 0);
        assert_eq!(unpaid.remaining, invoice_amount);
        assert_eq!(unpaid.percent_complete_bps, 0);
        assert!(!unpaid.finalized);

        client.process_partial_payment(&invoice_id, &250, &String::from_str(&env, "pay-1"));
        let partial = client.get_settlement_summary(&invoice_id).unwrap();
        assert_eq!(partial.total_paid, 250);
        assert_eq!(partial.remaining, 750);
        assert_eq!(partial.percent_complete_bps, 2_500);
        assert!(!partial.finalized);

        client.process_partial_payment(&invoice_id, &2_000, &String::from_str(&env, "pay-2"));
        let capped = client.get_settlement_summary(&invoice_id).unwrap();
        assert_eq!(capped.total_paid, invoice_amount);
        assert_eq!(capped.remaining, 0);
        assert_eq!(capped.percent_complete_bps, 10_000);
        assert!(!capped.finalized);

        let finalize_invoice_id = setup_funded_invoice(&env, &client, &contract_id, invoice_amount);
        client.process_partial_payment(
            &finalize_invoice_id,
            &250,
            &String::from_str(&env, "final-pay-1"),
        );
        client.settle_invoice(&finalize_invoice_id, &750);
        let finalized = client.get_settlement_summary(&finalize_invoice_id).unwrap();
        assert_eq!(finalized.total_due, invoice_amount);
        assert_eq!(finalized.total_paid, invoice_amount);
        assert_eq!(finalized.remaining, 0);
        assert_eq!(finalized.percent_complete_bps, 10_000);
        assert!(finalized.finalized);

        assert!(env
            .as_contract(&contract_id, || is_invoice_finalized(&env, &finalize_invoice_id))
            .unwrap());
    }
}