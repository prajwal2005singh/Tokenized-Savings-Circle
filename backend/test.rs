#![cfg(test)]

use super::*;
use soroban_sdk::{testenvironment::TestEnvironment, vec, Address, Env};

// --- Test Setup Helper ---
fn setup_env<'a>() -> (Env, SavingsCircleClient<'a>, Address, Address, Vec<Address>, TokenClient<'a>) {
    let env = Env::default();
    env.ledger().set_timestamp(1_000_000_000); // Set initial time

    let contract_id = env.register_contract(None, SavingsCircle);
    let client = SavingsCircleClient::new(&env, &contract_id);

    let admin = Address::random(&env);
    let token_admin = Address::random(&env);
    let members = vec![&env, Address::random(&env), Address::random(&env), Address::random(&env)];

    // Setup Token Contract
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = TokenClient::new(&env, &token_id);

    // Mint tokens to members for testing deposits
    let deposit_amount: i128 = 100_000_000;
    for member in members.iter() {
        token_client.mint(&token_admin, &member, &deposit_amount.checked_mul(5).unwrap()); // Give each member 5x deposit
        // Since deposits require `token_client.transfer(&depositor, &contract_addr, &amount)`, 
        // the depositor must have an authorization set, which is automatically handled in testutils.
    }
    
    // Also mint a large sum to the contract address's admin in case the contract needs to pay out a penalty refund
    token_client.mint(&token_admin, &client.address, &1_000_000_000_000); 

    (env, client, admin, token_id, members, token_client)
}

// --- Test Cases ---

#[test]
fn test_create_and_join_circle() {
    let (env, client, admin, token_id, initial_members, _) = setup_env();
    let deposit: i128 = 100_000_000;
    let cycle_interval: u64 = 60 * 60 * 24; // 1 day

    // 1. Create the circle
    client.create_circle(
        &admin, 
        &token_id, 
        &deposit, 
        &initial_members, 
        &cycle_interval, 
        &60 * 60
    ).unwrap();

    let state = client.get_circle().unwrap();
    assert_eq!(state.config.owner, admin);
    assert_eq!(state.config.deposit_amount, deposit);
    assert_eq!(state.current_cycle, 1);
    assert!(state.is_open_for_joining);

    // 2. Members join (confirm participation)
    for member in initial_members.iter() {
        client.join_circle(&member).unwrap();
        let m_state = client.get_member_state(&member).unwrap();
        assert_eq!(m_state.reputation_score, 10);
    }
    
    let state = client.get_circle().unwrap();
    assert_eq!(state.members.len(), initial_members.len());
}


#[test]
fn test_deposit_and_payout_happy_path() {
    let (env, client, admin, token_id, members, token_client) = setup_env();
    let deposit: i128 = 100;
    let cycle_interval: u64 = 100;
    
    client.create_circle(&admin, &token_id, &deposit, &members, &cycle_interval, &10).unwrap();
    for member in members.iter() { client.join_circle(&member).unwrap(); }

    let contract_addr = client.address.clone();
    let num_members = members.len() as i128;
    let total_pot = deposit.checked_mul(num_members).unwrap();
    
    // Check contract balance before deposits (should be zero)
    assert_eq!(token_client.balance(&contract_addr), 0);
    let initial_balance_c1 = token_client.balance(&members.get(0).unwrap());

    // --- Cycle 1: All members deposit ---
    for member in members.iter() {
        client.deposit(&member).unwrap();
    }
    assert_eq!(token_client.balance(&contract_addr), total_pot);
    
    // Advance time to allow execution
    env.ledger().set_timestamp(env.ledger().timestamp() + cycle_interval);

    // Execute cycle 1
    let recipient_c1 = members.get(0).unwrap();
    client.execute_cycle().unwrap();
    
    // Contract balance should be 0 after payout
    assert_eq!(token_client.balance(&contract_addr), 0);
    // Recipient C1 should have received the total pot
    assert_eq!(token_client.balance(&recipient_c1), initial_balance_c1 - deposit + total_pot); 
    
    let state = client.get_circle().unwrap();
    assert_eq!(state.current_cycle, 2);
    assert_eq!(state.next_payout_index, 1); // Next recipient is member index 1
}


#[test]
fn test_execute_cycle_with_missing_deposit_and_claim() {
    let (env, client, admin, token_id, members, token_client) = setup_env();
    let deposit: i128 = 10000;
    let cycle_interval: u64 = 100;
    let num_members = members.len() as i128; // 3 members

    client.create_circle(&admin, &token_id, &deposit, &members, &cycle_interval, &10).unwrap();
    for member in members.iter() { client.join_circle(&member).unwrap(); }
    
    let depositor_1 = members.get(0).unwrap();
    let missing_member = members.get(1).unwrap();
    let depositor_2 = members.get(2).unwrap();
    
    // Balances before cycle 1 deposits (excluding contract's admin mint)
    let initial_balance_d1 = token_client.balance(&depositor_1);
    let initial_balance_miss = token_client.balance(&missing_member);
    
    // Deposits made
    client.deposit(&depositor_1).unwrap();
    client.deposit(&depositor_2).unwrap();

    let collected_deposits = deposit.checked_mul(2).unwrap();
    assert_eq!(token_client.balance(&client.address), collected_deposits);
    
    // Advance time to allow execution
    env.ledger().set_timestamp(env.ledger().timestamp() + cycle_interval);
    
    // Execute cycle 1 (Recipient is Member 0)
    let recipient_c1 = members.get(0).unwrap();
    client.execute_cycle().unwrap();
    
    // --- Check Penalty and Reputation ---
    
    // Penalty is 20% of deposit (2000)
    let penalty_value = deposit.checked_div(100).unwrap().checked_mul(20).unwrap(); // 2000
    let penalty_share = penalty_value.checked_div(num_members).unwrap(); // 2000 / 3 = 666

    // 1. Missing Member (Member 1) state check
    let m_state = client.get_member_state(&missing_member).unwrap();
    // Accrued penalties should be negative (fine owed)
    assert_eq!(m_state.penalties_accrued, -penalty_value); // -2000
    assert_eq!(m_state.reputation_score, 9); // Initial 10 - 1 missed deposit
    
    // 2. Depositor 1 (Recipient/Depositor) state check
    let d_state = client.get_member_state(&depositor_1).unwrap();
    // Accrued penalties should be positive (share received)
    assert_eq!(d_state.penalties_accrued, penalty_share); // 666
    assert_eq!(d_state.reputation_score, 11); // Initial 10 + 1 successful deposit
    
    // 3. Depositor 2 state check (non-recipient depositor)
    let d2_state = client.get_member_state(&depositor_2).unwrap();
    assert_eq!(d2_state.penalties_accrued, penalty_share); // 666
    assert_eq!(d2_state.reputation_score, 11);

    // --- Check Claim Refund Logic ---
    
    // Missing Member cannot claim refund (owes fine)
    client.claim_refund(&missing_member).unwrap();
    let m_state_after_claim = client.get_member_state(&missing_member).unwrap();
    assert_eq!(m_state_after_claim.penalties_accrued, -penalty_value); // Still owes -2000

    // Depositor 1 claims refund (is recipient, but can still claim penalty share)
    let balance_d1_before_claim = token_client.balance(&depositor_1);
    client.claim_refund(&depositor_1).unwrap();
    let balance_d1_after_claim = token_client.balance(&depositor_1);
    
    assert_eq!(balance_d1_after_claim, balance_d1_before_claim + penalty_share); // +666
    let d_state_after_claim = client.get_member_state(&depositor_1).unwrap();
    assert_eq!(d_state_after_claim.penalties_accrued, 0); // Claimed
}