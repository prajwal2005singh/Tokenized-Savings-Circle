#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, Map, Symbol, Vec,
    token::Client as TokenClient,
    unwrap::UnwrapInfallible,
};

// --- Custom Errors ---
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    NotOwner = 1,
    CircleExists = 2,
    NotFound = 3,
    NotMember = 4,
    AlreadyJoined = 5,
    JoinDeadlinePassed = 6,
    DepositAlreadyMade = 7,
    NotAllDeposited = 8,
    CycleNotReady = 9,
    CycleNotPassed = 10,
    Paused = 11,
}

// --- Contract Data Keys ---
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    CircleState,    // Global state (CircleState)
    MemberRep(Address), // Member's reputation and state (MemberState)
    LastCycleTime,  // u64 timestamp of the last executed cycle
}

// --- State Structs ---

#[contracttype]
#[derive(Clone, Copy)]
pub struct CircleConfig {
    pub owner: Address,
    pub token_asset: Address,
    pub deposit_amount: i128,
    pub cycle_interval_secs: u64, // Time interval between cycle executions
    pub join_deadline_secs: u64,  // Max time for joining after creation
}

#[contracttype]
#[derive(Clone)]
pub struct CircleState {
    pub config: CircleConfig,
    pub members: Vec<Address>, // The final, confirmed member list
    pub member_deposits: Map<Address, u64>, // Temporary pre-confirmed members
    pub current_cycle: u32,
    pub next_payout_index: u32, // Index in `members` vector for the next payout
    pub deposits_bitmap: u32,  // Bitmap for current cycle deposits (1 = deposited, 0 = missed/late)
    pub is_paused: bool,
    pub is_open_for_joining: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct MemberState {
    pub reputation_score: u32, // +1 for success, -1 for missed
    pub penalties_accrued: i128, // Total value of penalties owed to the member
    pub last_deposit_cycle: u32, // Last cycle member successfully deposited for
}

// --- Events ---
#[contractimpl]
impl CircleState {
    fn emit_deposit_event(env: &Env, member: Address, cycle: u32) {
        env.events().publish((Symbol::new(env, "deposit"), member), cycle);
    }
    
    fn emit_payout_event(env: &Env, recipient: Address, cycle: u32, amount: i128) {
        env.events().publish((Symbol::new(env, "payout"), recipient), (cycle, amount));
    }

    fn emit_penalty_event(env: &Env, member: Address, cycle: u32, amount: i128, is_late: bool) {
        let ty = if is_late { symbol_short!("late") } else { symbol_short!("missed") };
        env.events().publish((Symbol::new(env, "penalty"), member, ty), (cycle, amount));
    }

    fn emit_cycle_executed_event(env: &Env, cycle: u32, recipient: Address) {
        env.events().publish((Symbol::new(env, "cycle_exec"), cycle), recipient);
    }
    
    fn emit_member_joined_event(env: &Env, member: Address) {
        env.events().publish((Symbol::new(env, "joined"), member), ());
    }
}


// --- Utility Functions ---

// Get member index from the vector
fn get_member_index(members: &Vec<Address>, member: &Address) -> Result<u32, Error> {
    for (i, m) in members.iter().enumerate() {
        if m == *member {
            return Ok(i as u32);
        }
    }
    Err(Error::NotMember)
}

// Function to read and write state
fn read_state(env: &Env) -> CircleState {
    env.storage()
        .instance()
        .get(&DataKey::CircleState)
        .unwrap_infallible()
}

fn write_state(env: &Env, state: &CircleState) {
    env.storage().instance().set(&DataKey::CircleState, state);
}

fn read_member_state(env: &Env, member: &Address) -> MemberState {
    env.storage()
        .persistent()
        .get(&DataKey::MemberRep(member.clone()))
        .unwrap_or(MemberState { // Default state for new members
            reputation_score: 10, // Start with a decent score
            penalties_accrued: 0,
            last_deposit_cycle: 0,
        })
}

fn write_member_state(env: &Env, member: &Address, state: &MemberState) {
    env.storage().persistent().set(&DataKey::MemberRep(member.clone()), state);
}

fn get_token_client(env: &Env, token_id: &Address) -> TokenClient {
    TokenClient::new(env, token_id)
}


// --- The Contract ---
#[contract]
pub struct SavingsCircle;

#[contractimpl]
impl SavingsCircle {
    
    // --- Initialization & Membership ---

    /// Creates the savings circle. Only one call allowed per contract instance.
    pub fn create_circle(
        env: Env,
        owner: Address,
        token_asset: Address,
        deposit_amount: i128,
        members: Vec<Address>,
        cycle_interval_secs: u64,
        join_deadline_secs: u64,
    ) -> Result<(), Error> {
        owner.require_auth();

        if env.storage().instance().has(&DataKey::CircleState) {
            return Err(Error::CircleExists);
        }
        
        // Basic validation
        if deposit_amount <= 0 || members.len() == 0 {
            // More robust validation needed in production
        }

        let config = CircleConfig {
            owner: owner.clone(),
            token_asset: token_asset,
            deposit_amount,
            cycle_interval_secs,
            join_deadline_secs,
        };

        let initial_state = CircleState {
            config,
            members: Vec::new(&env), // Members confirm their spot with join_circle
            member_deposits: Map::new(&env),
            current_cycle: 1,
            next_payout_index: 0,
            deposits_bitmap: 0,
            is_paused: false,
            is_open_for_joining: true,
        };

        write_state(&env, &initial_state);
        
        // Pre-confirm initial members for the deadline clock
        for member in members.iter() {
            initial_state.member_deposits.set(member, env.ledger().timestamp());
        }

        Ok(())
    }
    
    /// Confirms participation in the circle. Must be called before deadline.
    pub fn join_circle(env: Env, member: Address) -> Result<(), Error> {
        member.require_auth();
        let mut state = read_state(&env);

        if state.is_paused { return Err(Error::Paused); }
        if !state.is_open_for_joining { return Err(Error::JoinDeadlinePassed); }
        
        let now = env.ledger().timestamp();
        let creation_time = env.storage().instance().get(&DataKey::LastCycleTime).unwrap_or(0); // Use 0 for initial
        
        if now > creation_time + state.config.join_deadline_secs {
             state.is_open_for_joining = false;
             write_state(&env, &state);
             return Err(Error::JoinDeadlinePassed);
        }

        if state.members.contains(&member) {
            return Err(Error::AlreadyJoined);
        }
        
        state.members.push_back(member.clone());
        write_state(&env, &state);
        
        CircleState::emit_member_joined_event(&env, member);

        Ok(())
    }


    // --- Core Operations ---
    
    /// Participant deposits the fixed amount for the current cycle.
    pub fn deposit(env: Env, depositor: Address) -> Result<(), Error> {
        depositor.require_auth();
        let mut state = read_state(&env);
        
        if state.is_paused { return Err(Error::Paused); }

        let token_client = get_token_client(&env, &state.config.token_asset);
        let amount = state.config.deposit_amount;
        
        // 1. Check membership
        let member_index = get_member_index(&state.members, &depositor)?;
        
        // 2. Check if already deposited for this cycle (using bitmap)
        if (state.deposits_bitmap & (1u32 << member_index)) != 0 {
            return Err(Error::DepositAlreadyMade);
        }

        // 3. Transfer token from depositor to contract
        token_client.transfer(&depositor, &env.current_contract_address(), &amount);

        // 4. Update bitmap
        state.deposits_bitmap |= 1u32 << member_index;
        
        // 5. Update reputation (successful deposit)
        let mut m_state = read_member_state(&env, &depositor);
        m_state.reputation_score = m_state.reputation_score.saturating_add(1);
        m_state.last_deposit_cycle = state.current_cycle;
        write_member_state(&env, &depositor, &m_state);

        write_state(&env, &state);
        CircleState::emit_deposit_event(&env, depositor, state.current_cycle);

        Ok(())
    }
    
    /// Executes the next cycle, handles payouts, and applies penalties.
    /// This function is intended to be called by an external relayer/frontend.
    pub fn execute_cycle(env: Env) -> Result<(), Error> {
        // No auth check on the caller, as it's an external trigger
        let mut state = read_state(&env);

        if state.is_paused { return Err(Error::Paused); }
        
        let now = env.ledger().timestamp();
        let last_cycle_time: u64 = env.storage().instance().get(&DataKey::LastCycleTime).unwrap_or(0); // 0 for the very first execution

        // 1. Check Cycle Scheduling
        if last_cycle_time != 0 && now < last_cycle_time + state.config.cycle_interval_secs {
            return Err(Error::CycleNotReady);
        }

        let num_members = state.members.len();
        if num_members == 0 {
            // Cannot execute cycle without members, but this shouldn't happen if join_circle is used correctly
            return Err(Error::NotFound);
        }

        let token_client = get_token_client(&env, &state.config.token_asset);
        let deposit_amount = state.config.deposit_amount;
        let total_pot = deposit_amount.checked_mul(num_members as i128).unwrap_infallible();
        let payout_recipient = state.members.get(state.next_payout_index).unwrap_infallible();

        // --- Penalty & Reputation Logic ---
        
        let penalty_missed_mult: i128 = 20; // 20% penalty
        let base_penalty_amount = deposit_amount.checked_div(100).unwrap_infallible();
        
        let mut pooled_penalties: i128 = 0;

        for i in 0..num_members {
            let member_addr = state.members.get(i as u32).unwrap_infallible();
            let is_deposited = (state.deposits_bitmap & (1u32 << i)) != 0;
            
            if !is_deposited {
                // Member has NOT deposited. This is a MISSED DEPOSIT.
                let mut m_state = read_member_state(&env, &member_addr);
                
                // Penalty value: 20% of deposit
                let penalty_value = base_penalty_amount.checked_mul(penalty_missed_mult).unwrap_infallible();
                
                // NOTE: In the contract, we can't force the transfer from a member here unless they authorized it.
                // For simplicity, the penalty is accrued to the member's account. They are *fined* this amount.
                m_state.penalties_accrued = m_state.penalties_accrued.checked_sub(penalty_value).unwrap_infallible(); // Fined: subtract penalty from their claimable balance
                pooled_penalties = pooled_penalties.checked_add(penalty_value).unwrap_infallible(); // Add penalty value to the pot to be distributed
                
                m_state.reputation_score = m_state.reputation_score.saturating_sub(1); // Decrease score
                

                write_member_state(&env, &member_addr, &m_state);
                CircleState::emit_penalty_event(&env, member_addr, state.current_cycle, penalty_value, false);
            }
        }
        
        // --- Payout Logic ---
        
        // 1. Payout: The recipient receives the total pot of collected deposits
        token_client.transfer(&env.current_contract_address(), &payout_recipient, &total_pot);

        // 2. Penalty Distribution: All collected penalties are distributed equally among ALL members 
        // by increasing their claimable balance.
        if pooled_penalties > 0 {
            let penalty_share = pooled_penalties.checked_div(num_members as i128).unwrap_infallible();

            for member in state.members.iter() {
                let mut m_state = read_member_state(&env, &member);
                m_state.penalties_accrued = m_state.penalties_accrued.checked_add(penalty_share).unwrap_infallible();
                write_member_state(&env, &member, &m_state);
            }
        }

        CircleState::emit_payout_event(&env, payout_recipient, state.current_cycle, total_pot);

        // --- Advance Cycle State ---
        
        state.current_cycle = state.current_cycle.checked_add(1).unwrap_infallible();
        
        // Rotate the payout index
        state.next_payout_index = (state.next_payout_index.checked_add(1).unwrap_infallible()) % num_members;

        // Reset the deposit bitmap for the new cycle
        state.deposits_bitmap = 0;
        
        // Update last execution time
        env.storage().instance().set(&DataKey::LastCycleTime, &now);

        write_state(&env, &state);
        CircleState::emit_cycle_executed_event(&env, state.current_cycle - 1, payout_recipient);

        Ok(())
    }


    // --- Admin & Utility ---
    
    /// Allows a member to claim their accumulated refunds/penalties (positive balance).
    pub fn claim_refund(env: Env, member: Address) -> Result<(), Error> {
        member.require_auth();
        let state = read_state(&env);
        
        let mut m_state = read_member_state(&env, &member);
        
        let amount = m_state.penalties_accrued;
        if amount <= 0 {
            return Ok(()); // Nothing to claim or member owes a fine
        }
        
        let token_client = get_token_client(&env, &state.config.token_asset);
        
        // Transfer collected penalties from contract to member
        token_client.transfer(&env.current_contract_address(), &member, &amount);
        
        // Reset accrued penalties
        m_state.penalties_accrued = 0;
        write_member_state(&env, &member, &m_state);
        
        Ok(())
    }

    /// Emergency pause for the circle.
    pub fn pause(env: Env, owner: Address) -> Result<(), Error> {
        owner.require_auth();
        let mut state = read_state(&env);

        if state.config.owner != owner {
            return Err(Error::NotOwner);
        }
        
        state.is_paused = true;
        write_state(&env, &state);
        Ok(())
    }

    /// Unpauses the circle.
    pub fn unpause(env: Env, owner: Address) -> Result<(), Error> {
        owner.require_auth();
        let mut state = read_state(&env);

        if state.config.owner != owner {
            return Err(Error::NotOwner);
        }

        state.is_paused = false;
        write_state(&env, &state);
        Ok(())
    }
    
    // --- View Functions (Read-Only) ---

    pub fn get_circle(env: Env) -> Result<CircleState, Error> {
        if !env.storage().instance().has(&DataKey::CircleState) {
            return Err(Error::NotFound);
        }
        Ok(read_state(&env))
    }

    pub fn get_member_state(env: Env, member: Address) -> Result<MemberState, Error> {
        let state = read_member_state(&env, &member);
        Ok(state)
    }
}