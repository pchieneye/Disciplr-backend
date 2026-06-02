#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol};

#[contracttype]
pub enum DataKey {
    DisputeWindow,
}

#[contracttype]
pub struct Milestone {
    pub verified: bool,
    pub verified_at: u64,
}

#[contract]
pub struct AccountabilityVault;

#[contractimpl]
impl AccountabilityVault {
    pub fn configure_window(env: Env, window: u64) {
        // Admin or Guardian would call this to configure the window
        env.storage().instance().set(&DataKey::DisputeWindow, &window);
    }

    pub fn dispute_milestone(env: Env, creator: Address, index: u32) {
        creator.require_auth();
        
        // Fetch the configured window, default to 86400 seconds (24 hours) if not set
        let dispute_window: u64 = env.storage().instance().get(&DataKey::DisputeWindow).unwrap_or(86400);
        
        // Mocking the stored milestone retrieval for this patch
        let mut milestone = Milestone { verified: true, verified_at: env.ledger().timestamp() - 100 };
        
        assert!(milestone.verified, "Milestone must be verified to dispute");
        
        let current_time = env.ledger().timestamp();
        assert!(
            current_time <= milestone.verified_at + dispute_window,
            "Dispute window has passed"
        );
        
        milestone.verified = false;
        
        let event_name = Symbol::new(&env, "milestone_disputed");
        env.events().publish((event_name, creator), index);
    }
}
