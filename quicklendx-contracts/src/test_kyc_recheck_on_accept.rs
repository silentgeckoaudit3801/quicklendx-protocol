//! Regression tests for investor KYC re-checks at escrow funding time.
//!
//! A bid can be placed while an investor is verified, then accepted later by the
//! business. The funding boundary must re-read investor verification state so a
//! revoked investor cannot fund escrow with a stale, previously valid bid.

use super::*;
use crate::errors::QuickLendXError;
use crate::invoice::{InvoiceCategory, InvoiceStatus};
use crate::payments::EscrowStatus;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, BytesN, Env, String, Vec,
};

fn setup() -> (Env, QuickLendXContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(QuickLendXContract, ());
    let client = QuickLendXContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let _ = client.try_initialize_admin(&admin);
    client.set_admin(&admin);
    (env, client, admin)
}

fn setup_token(env: &Env, addresses: &[&Address], contract_id: &Address, amount: i128) -> Address {
    let token_admin = Address::generate(env);
    let currency = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::Client::new(env, &currency);
    let sac_client = token::StellarAssetClient::new(env, &currency);
    let expiration = env.ledger().sequence() + 100_000;

    for addr in addresses {
        sac_client.mint(addr, &amount);
        token_client.approve(addr, contract_id, &amount, &expiration);
    }

    currency
}

fn verified_business(env: &Env, client: &QuickLendXContractClient, admin: &Address) -> Address {
    let business = Address::generate(env);
    client.submit_kyc_application(&business, &String::from_str(env, "Business KYC"));
    client.verify_business(admin, &business);
    business
}

fn verified_investor(
    env: &Env,
    client: &QuickLendXContractClient,
    investment_limit: i128,
) -> Address {
    let investor = Address::generate(env);
    client.submit_investor_kyc(&investor, &String::from_str(env, "Investor KYC"));
    client.verify_investor(&investor, &investment_limit);
    investor
}

fn verified_invoice(
    env: &Env,
    client: &QuickLendXContractClient,
    business: &Address,
    currency: &Address,
    amount: i128,
) -> BytesN<32> {
    let due_date = env.ledger().timestamp() + 86_400 * 30;
    let invoice_id = client.upload_invoice(
        business,
        &amount,
        currency,
        &due_date,
        &String::from_str(env, "KYC re-check regression invoice"),
        &InvoiceCategory::Services,
        &Vec::new(env),
    );
    client.verify_invoice(&invoice_id);
    invoice_id
}

fn accepted_bid_fixture() -> (
    Env,
    QuickLendXContractClient<'static>,
    Address,
    BytesN<32>,
    BytesN<32>,
    Address,
) {
    let (env, client, admin) = setup();
    let contract_id = env.current_contract_address();
    let investor = verified_investor(&env, &client, 500_000);
    let business = verified_business(&env, &client, &admin);
    let currency = setup_token(&env, &[&investor, &business], &contract_id, 200_000);
    let invoice_id = verified_invoice(&env, &client, &business, &currency, 100_000);
    let bid_id = client.place_bid(&investor, &invoice_id, &100_000, &10_000);

    (env, client, admin, invoice_id, bid_id, investor)
}

#[test]
fn revoked_investor_cannot_fund_with_previously_placed_bid() {
    let (env, client, _admin, invoice_id, bid_id, investor) = accepted_bid_fixture();

    client.revoke_investor_kyc(
        &investor,
        &String::from_str(&env, "KYC revoked before funding"),
    );

    let result = client.try_accept_bid_and_fund(&invoice_id, &bid_id);
    assert_eq!(
        result.unwrap_err().unwrap(),
        QuickLendXError::InvestorNotVerified
    );

    let invoice = client.get_invoice(&invoice_id);
    assert_eq!(invoice.status, InvoiceStatus::Verified);
    assert_eq!(invoice.funded_amount, 0);
    assert!(invoice.funded_at.is_none());
    assert!(invoice.investor.is_none());

    let bid = client.get_bid(&bid_id).expect("bid remains stored");
    assert_eq!(bid.status, BidStatus::Placed);

    let escrow = client.try_get_escrow_details(&invoice_id);
    assert_eq!(
        escrow.unwrap_err().unwrap(),
        QuickLendXError::StorageKeyNotFound
    );
}

#[test]
fn still_verified_investor_accepts_normally() {
    let (_env, client, _admin, invoice_id, bid_id, _investor) = accepted_bid_fixture();

    let result = client.try_accept_bid_and_fund(&invoice_id, &bid_id);
    assert!(result.is_ok(), "verified investor should still fund escrow");

    let invoice = client.get_invoice(&invoice_id);
    assert_eq!(invoice.status, InvoiceStatus::Funded);
    assert_eq!(invoice.funded_amount, 100_000);
    assert!(invoice.funded_at.is_some());
    assert!(invoice.investor.is_some());

    let escrow = client.get_escrow_details(&invoice_id);
    assert_eq!(escrow.status, EscrowStatus::Held);
}

#[test]
fn revoked_then_reverified_investor_accepts_normally() {
    let (env, client, _admin, invoice_id, bid_id, investor) = accepted_bid_fixture();

    client.revoke_investor_kyc(
        &investor,
        &String::from_str(&env, "temporary compliance hold"),
    );
    client.verify_investor(&investor, &500_000);

    let result = client.try_accept_bid_and_fund(&invoice_id, &bid_id);
    assert!(
        result.is_ok(),
        "re-verified investor should be accepted at funding time"
    );

    let invoice = client.get_invoice(&invoice_id);
    assert_eq!(invoice.status, InvoiceStatus::Funded);
}