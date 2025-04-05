#![allow(non_snake_case)]
#![no_std]
use soroban_sdk::{contract, contracttype, contractimpl, log, Env, Symbol, String, Address, Vec, symbol_short, BytesN};

// Asset status structure to track leasing metrics
#[contracttype]
#[derive(Clone)]
pub struct AssetStats {
    pub active_leases: u64,     // Count of currently active leases
    pub completed_leases: u64,  // Count of completed leases
    pub overdue_leases: u64,    // Count of leases with overdue payments
    pub total_leases: u64,      // Total count of all leases created
    pub total_xlm_processed: u64, // Total XLM processed through the system
}

// For referencing the AssetStats struct - shortened to fit 9 character limit
const ALL_ASSET: Symbol = symbol_short!("ALL_ASSET");

// Mapping unique_id of asset to its LeaseStatus
#[contracttype] 
pub enum LeaseStatusBook { 
    LeaseStatus(u64)
}

// Structure to track the status of a lease
#[contracttype]
#[derive(Clone)] 
pub struct LeaseStatus {
    pub lease_id: u64,          // Unique lease identifier
    pub asset_id: u64,          // Associated asset identifier
    pub lessee: Address,        // Address of the person leasing the asset
    pub start_time: u64,        // Lease start timestamp
    pub end_time: u64,          // Lease end timestamp
    pub period_payment: u64,    // Amount of XLM per payment period
    pub payment_frequency: u64, // How often payments occur (in seconds)
    pub last_payment_time: u64, // When the last payment was made
    pub next_payment_time: u64, // When the next payment is due
    pub is_active: bool,        // Whether the lease is currently active
    pub is_overdue: bool,       // Whether payments are overdue
    pub total_paid: u64,        // Total XLM paid so far
    pub security_deposit: u64,  // Security deposit amount in XLM
}

// Mapping asset_id to Asset
#[contracttype] 
pub enum AssetBook { 
    Asset(u64)
}

// For creating unique asset IDs
const COUNT_ASSETS: Symbol = symbol_short!("C_ASSETS"); 

// Structure defining an asset available for lease
#[contracttype]
#[derive(Clone)] 
pub struct Asset {
    pub asset_id: u64,          // Unique asset identifier
    pub owner: Address,         // Address of the asset owner
    pub title: String,          // Title/name of the asset
    pub description: String,    // Description of the asset
    pub price_per_period: u64,  // Price in XLM per period
    pub period_duration: u64,   // Duration of a payment period in seconds
    pub min_lease_duration: u64, // Minimum lease duration in seconds
    pub max_lease_duration: u64, // Maximum lease duration in seconds
    pub deposit_required: u64,   // Required security deposit in XLM
    pub is_available: bool,      // Whether the asset is available for lease
    pub current_lease_id: u64,   // ID of current active lease (0 if none)
}

// For tracking the next available lease ID
const COUNT_LEASES: Symbol = symbol_short!("C_LEASES");

#[contract]
pub struct RentPaymentContract;

#[contractimpl]
impl RentPaymentContract {
    
    // Register a new asset for leasing
    pub fn register_asset(
        env: Env, 
        owner: Address,  // Pass the owner address as a parameter instead of using invoker
        title: String, 
        description: String,
        price_per_period: u64,
        period_duration: u64,
        min_lease_duration: u64,
        max_lease_duration: u64,
        deposit_required: u64
    ) -> u64 {
        // Create a new unique asset ID
        let mut count_assets: u64 = env.storage().instance().get(&COUNT_ASSETS).unwrap_or(0);
        count_assets += 1;
        
        // Create a new asset
        let asset = Asset {
            asset_id: count_assets,
            owner: owner,
            title: title,
            description: description,
            price_per_period: price_per_period,
            period_duration: period_duration,
            min_lease_duration: min_lease_duration,
            max_lease_duration: max_lease_duration,
            deposit_required: deposit_required,
            is_available: true,
            current_lease_id: 0,
        };
        
        // Update the asset count
        env.storage().instance().set(&COUNT_ASSETS, &count_assets);
        
        // Store the asset data
        env.storage().instance().set(&AssetBook::Asset(count_assets), &asset);
        
        // Update global stats
        let mut stats = Self::view_asset_stats(env.clone());
        stats.total_leases = stats.total_leases;  // No change to total leases
        env.storage().instance().set(&ALL_ASSET, &stats);
        
        env.storage().instance().extend_ttl(10000, 10000);
        
        log!(&env, "Asset registered with ID: {}", count_assets);
        
        return count_assets;
    }
    
    // Create a new lease for an asset
    pub fn create_lease(
        env: Env,
        asset_id: u64,
        lessee: Address,  // Pass the lessee address as a parameter
        lease_duration: u64  // Duration in seconds
    ) -> u64 {
        // Get the asset
        let mut asset = Self::view_asset(env.clone(), asset_id);
        
        // Check if asset exists and is available
        if asset.asset_id == 0 || !asset.is_available {
            log!(&env, "Asset is not available for lease");
            panic!("Asset is not available for lease");
        }
        
        // Validate lease duration
        if lease_duration < asset.min_lease_duration || lease_duration > asset.max_lease_duration {
            log!(&env, "Lease duration outside allowed range");
            panic!("Lease duration outside allowed range");
        }
        
        // Create a new unique lease ID
        let mut count_leases: u64 = env.storage().instance().get(&COUNT_LEASES).unwrap_or(0);
        count_leases += 1;
        
        // Get current time
        let now = env.ledger().timestamp();
        
        // Create new lease
        let lease_status = LeaseStatus {
            lease_id: count_leases,
            asset_id: asset_id,
            lessee: lessee,
            start_time: now,
            end_time: now + lease_duration,
            period_payment: asset.price_per_period,
            payment_frequency: asset.period_duration,
            last_payment_time: now,  // Initial payment happens at lease creation
            next_payment_time: now + asset.period_duration,
            is_active: true,
            is_overdue: false,
            total_paid: asset.deposit_required + asset.price_per_period,  // Initial payment + deposit
            security_deposit: asset.deposit_required,
        };
        
        // Update the lease count
        env.storage().instance().set(&COUNT_LEASES, &count_leases);
        
        // Store the lease data
        env.storage().instance().set(&LeaseStatusBook::LeaseStatus(count_leases), &lease_status);
        
        // Update asset availability
        asset.is_available = false;
        asset.current_lease_id = count_leases;
        env.storage().instance().set(&AssetBook::Asset(asset_id), &asset);
        
        // Update global stats
        let mut stats = Self::view_asset_stats(env.clone());
        stats.active_leases += 1;
        stats.total_leases += 1;
        stats.total_xlm_processed += asset.deposit_required + asset.price_per_period;
        env.storage().instance().set(&ALL_ASSET, &stats);
        
        env.storage().instance().extend_ttl(10000, 10000);
        
        log!(&env, "Lease created with ID: {}", count_leases);
        
        return count_leases;
    }
    
    // Process a payment for a lease
    pub fn process_payment(env: Env, lease_id: u64, caller: Address) {
        // Get the lease
        let mut lease = Self::view_lease(env.clone(), lease_id);
        
        // Check if lease exists and is active
        if lease.lease_id == 0 || !lease.is_active {
            log!(&env, "Lease is not active");
            panic!("Lease is not active");
        }
        
        // Verify caller is the lessee
        if caller != lease.lessee {
            log!(&env, "Only the lessee can make payments");
            panic!("Only the lessee can make payments");
        }
        
        // Get current time
        let now = env.ledger().timestamp();
        
        // Update lease payment info
        lease.last_payment_time = now;
        lease.next_payment_time = now + lease.payment_frequency;
        lease.total_paid += lease.period_payment;
        lease.is_overdue = false;
        
        // Store updated lease data
        env.storage().instance().set(&LeaseStatusBook::LeaseStatus(lease_id), &lease);
        
        // Update global stats
        let mut stats = Self::view_asset_stats(env.clone());
        stats.total_xlm_processed += lease.period_payment;
        if lease.is_overdue {
            stats.overdue_leases -= 1;
        }
        env.storage().instance().set(&ALL_ASSET, &stats);
        
        log!(&env, "Payment processed for lease ID: {}", lease_id);
    }
    
    // End a lease (can be called by lessee or automatically when lease expires)
    pub fn end_lease(env: Env, lease_id: u64, caller: Address) {
        // Get the lease
        let mut lease = Self::view_lease(env.clone(), lease_id);
        
        // Check if lease exists and is active
        if lease.lease_id == 0 || !lease.is_active {
            log!(&env, "Lease is not active");
            panic!("Lease is not active");
        }
        
        // Get the asset
        let mut asset = Self::view_asset(env.clone(), lease.asset_id);
        
        // Verify caller is either the lessee or asset owner
        if caller != lease.lessee && caller != asset.owner {
            log!(&env, "Only the lessee or asset owner can end the lease");
            panic!("Only the lessee or asset owner can end the lease");
        }
        
        // Update lease status
        lease.is_active = false;
        env.storage().instance().set(&LeaseStatusBook::LeaseStatus(lease_id), &lease);
        
        // Update asset availability
        asset.is_available = true;
        asset.current_lease_id = 0;
        env.storage().instance().set(&AssetBook::Asset(lease.asset_id), &asset);
        
        // Update global stats
        let mut stats = Self::view_asset_stats(env.clone());
        stats.active_leases -= 1;
        stats.completed_leases += 1;
        if lease.is_overdue {
            stats.overdue_leases -= 1;
        }
        env.storage().instance().set(&ALL_ASSET, &stats);
        
        log!(&env, "Lease ended for lease ID: {}", lease_id);
    }
    
    // Mark a lease as overdue (called by a scheduled job or manually by owner)
    pub fn mark_lease_overdue(env: Env, lease_id: u64, caller: Address) {
        // Get the lease
        let mut lease = Self::view_lease(env.clone(), lease_id);
        
        // Check if lease exists and is active
        if lease.lease_id == 0 || !lease.is_active {
            log!(&env, "Lease is not active");
            panic!("Lease is not active");
        }
        
        // Get the asset
        let asset = Self::view_asset(env.clone(), lease.asset_id);
        
        // Verify caller is the asset owner
        if caller != asset.owner {
            log!(&env, "Only the asset owner can mark a lease as overdue");
            panic!("Only the asset owner can mark a lease as overdue");
        }
        
        // Get current time
        let now = env.ledger().timestamp();
        
        // Check if payment is actually overdue
        if now < lease.next_payment_time {
            log!(&env, "Payment is not yet due");
            panic!("Payment is not yet due");
        }
        
        // Update lease status
        lease.is_overdue = true;
        env.storage().instance().set(&LeaseStatusBook::LeaseStatus(lease_id), &lease);
        
        // Update global stats
        let mut stats = Self::view_asset_stats(env.clone());
        stats.overdue_leases += 1;
        env.storage().instance().set(&ALL_ASSET, &stats);
        
        log!(&env, "Lease marked as overdue for lease ID: {}", lease_id);
    }
    
    // View asset stats
    pub fn view_asset_stats(env: Env) -> AssetStats {
        env.storage().instance().get(&ALL_ASSET).unwrap_or(AssetStats {
            active_leases: 0,
            completed_leases: 0,
            overdue_leases: 0,
            total_leases: 0,
            total_xlm_processed: 0,
        })
    }
    
    // View asset details
    pub fn view_asset(env: Env, asset_id: u64) -> Asset {
        let key = AssetBook::Asset(asset_id);
        
        env.storage().instance().get(&key).unwrap_or(Asset {
            asset_id: 0,
            owner: Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
            title: String::from_str(&env, "Not_Found"),
            description: String::from_str(&env, "Not_Found"),
            price_per_period: 0,
            period_duration: 0,
            min_lease_duration: 0,
            max_lease_duration: 0,
            deposit_required: 0,
            is_available: false,
            current_lease_id: 0,
        })
    }
    
    // View lease details
    pub fn view_lease(env: Env, lease_id: u64) -> LeaseStatus {
        let key = LeaseStatusBook::LeaseStatus(lease_id);
        
        env.storage().instance().get(&key).unwrap_or(LeaseStatus {
            lease_id: 0,
            asset_id: 0,
            lessee: Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
            start_time: 0,
            end_time: 0,
            period_payment: 0,
            payment_frequency: 0,
            last_payment_time: 0,
            next_payment_time: 0,
            is_active: false,
            is_overdue: false,
            total_paid: 0,
            security_deposit: 0,
        })
    }
    
    // Get all assets owned by a specific address
    pub fn get_owner_assets(env: Env, owner: Address) -> Vec<u64> {
        let count_assets: u64 = env.storage().instance().get(&COUNT_ASSETS).unwrap_or(0);
        let mut owner_assets = Vec::new(&env);
        
        for i in 1..=count_assets {
            let asset = Self::view_asset(env.clone(), i);
            if asset.owner == owner {
                owner_assets.push_back(asset.asset_id);
            }
        }
        
        return owner_assets;
    }
    
    // Get all active leases for a specific lessee
    pub fn get_lessee_leases(env: Env, lessee: Address) -> Vec<u64> {
        let count_leases: u64 = env.storage().instance().get(&COUNT_LEASES).unwrap_or(0);
        let mut lessee_leases = Vec::new(&env);
        
        for i in 1..=count_leases {
            let lease = Self::view_lease(env.clone(), i);
            if lease.lessee == lessee && lease.is_active {
                lessee_leases.push_back(lease.lease_id);
            }
        }
        
        return lessee_leases;
    }
}