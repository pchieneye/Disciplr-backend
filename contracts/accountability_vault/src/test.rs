#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::{Address as _, Events, Ledger}, Address, Env};

#[test]
fn test_dispute() {
    let env = Env::default();
    env.ledger().set_timestamp(1690000000);
    
    let contract_id = env.register_contract(None, AccountabilityVault);
    let client = AccountabilityVaultClient::new(&env, &contract_id);
    let creator = Address::generate(&env);
    env.mock_all_auths();
    
    // Configure window to 1 hour
    client.configure_window(&3600u64);
    
    // Call strictly with creator and index
    client.dispute_milestone(&creator, &1u32);
    assert_eq!(env.events().all().len(), 1);
}
