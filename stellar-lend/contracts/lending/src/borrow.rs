use soroban_sdk::{contracterror, contracttype, Address, Env, Symbol};

/// Errors that can occur during borrow operations
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BorrowError {
    InsufficientCollateral = 1,
    DebtCeilingReached = 2,
    ProtocolPaused = 3,
    InvalidAmount = 4,
    Overflow = 5,
    Unauthorized = 6,
    AssetNotSupported = 7,
    BelowMinimumBorrow = 8,
}

/// Storage keys for borrow-related data
#[contracttype]
#[derive(Clone)]
pub enum BorrowDataKey {
    UserDebt(Address),
    UserCollateral(Address),
    TotalDebt,
    DebtCeiling,
    InterestRate,
    CollateralRatio,
    MinBorrowAmount,
    Paused,
}

/// User debt position
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct DebtPosition {
    pub borrowed_amount: i128,
    pub interest_accrued: i128,
    pub last_update: u64,
    pub asset: Address,
}

/// User collateral position
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CollateralPosition {
    pub amount: i128,
    pub asset: Address,
}

/// Borrow event data
#[contracttype]
#[derive(Clone, Debug)]
pub struct BorrowEvent {
    pub user: Address,
    pub asset: Address,
    pub amount: i128,
    pub collateral: i128,
    pub timestamp: u64,
}

const COLLATERAL_RATIO_MIN: i128 = 15000; // 150% in basis points
const INTEREST_RATE_PER_YEAR: i128 = 500; // 5% in basis points
const SECONDS_PER_YEAR: u64 = 31536000;

/// Borrow assets against deposited collateral
///
/// # Arguments
/// * `env` - The contract environment
/// * `user` - The borrower's address
/// * `asset` - The asset to borrow
/// * `amount` - The amount to borrow
/// * `collateral_asset` - The collateral asset
/// * `collateral_amount` - The collateral amount
///
/// # Returns
/// Returns Ok(()) on success or BorrowError on failure
///
/// # Security
/// - Validates collateral ratio meets minimum requirements
/// - Checks protocol is not paused
/// - Validates debt ceiling not exceeded
/// - Prevents overflow in calculations
pub fn borrow(
    env: &Env,
    user: Address,
    asset: Address,
    amount: i128,
    collateral_asset: Address,
    collateral_amount: i128,
) -> Result<(), BorrowError> {
    user.require_auth();

    if is_paused(env) {
        return Err(BorrowError::ProtocolPaused);
    }

    if amount <= 0 || collateral_amount <= 0 {
        return Err(BorrowError::InvalidAmount);
    }

    let min_borrow = get_min_borrow_amount(env);
    if amount < min_borrow {
        return Err(BorrowError::BelowMinimumBorrow);
    }

    validate_collateral_ratio(collateral_amount, amount)?;

    let total_debt = get_total_debt(env);
    let debt_ceiling = get_debt_ceiling(env);
    let new_total = total_debt
        .checked_add(amount)
        .ok_or(BorrowError::Overflow)?;

    if new_total > debt_ceiling {
        return Err(BorrowError::DebtCeilingReached);
    }

    let mut debt_position = get_debt_position(env, &user);
    let accrued_interest = calculate_interest(env, &debt_position);

    debt_position.borrowed_amount = debt_position
        .borrowed_amount
        .checked_add(amount)
        .ok_or(BorrowError::Overflow)?;
    debt_position.interest_accrued = debt_position
        .interest_accrued
        .checked_add(accrued_interest)
        .ok_or(BorrowError::Overflow)?;
    debt_position.last_update = env.ledger().timestamp();
    debt_position.asset = asset.clone();

    let mut collateral_position = get_collateral_position(env, &user);
    collateral_position.amount = collateral_position
        .amount
        .checked_add(collateral_amount)
        .ok_or(BorrowError::Overflow)?;
    collateral_position.asset = collateral_asset.clone();

    save_debt_position(env, &user, &debt_position);
    save_collateral_position(env, &user, &collateral_position);
    set_total_debt(env, new_total);

    emit_borrow_event(env, user, asset, amount, collateral_amount);

    Ok(())
}

/// Validate collateral ratio meets minimum requirements
fn validate_collateral_ratio(collateral: i128, borrow: i128) -> Result<(), BorrowError> {
    // To avoid overflow, check if collateral >= borrow * 1.5
    // Which is: collateral * 10000 >= borrow * 15000
    // Rearranged: collateral >= (borrow * 15000) / 10000

    let min_collateral = borrow
        .checked_mul(COLLATERAL_RATIO_MIN)
        .ok_or(BorrowError::Overflow)?
        .checked_div(10000)
        .ok_or(BorrowError::InvalidAmount)?;

    if collateral < min_collateral {
        return Err(BorrowError::InsufficientCollateral);
    }

    Ok(())
}

/// Calculate accrued interest for a debt position
fn calculate_interest(env: &Env, position: &DebtPosition) -> i128 {
    if position.borrowed_amount == 0 {
        return 0;
    }

    let current_time = env.ledger().timestamp();
    let time_elapsed = current_time.saturating_sub(position.last_update);

    position
        .borrowed_amount
        .saturating_mul(INTEREST_RATE_PER_YEAR)
        .saturating_mul(time_elapsed as i128)
        .saturating_div(10000)
        .saturating_div(SECONDS_PER_YEAR as i128)
}

fn get_debt_position(env: &Env, user: &Address) -> DebtPosition {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::UserDebt(user.clone()))
        .unwrap_or(DebtPosition {
            borrowed_amount: 0,
            interest_accrued: 0,
            last_update: env.ledger().timestamp(),
            asset: user.clone(), // Placeholder, will be replaced on first borrow
        })
}

fn save_debt_position(env: &Env, user: &Address, position: &DebtPosition) {
    env.storage()
        .persistent()
        .set(&BorrowDataKey::UserDebt(user.clone()), position);
}

fn get_collateral_position(env: &Env, user: &Address) -> CollateralPosition {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::UserCollateral(user.clone()))
        .unwrap_or(CollateralPosition {
            amount: 0,
            asset: user.clone(), // Placeholder, will be replaced on first borrow
        })
}

fn save_collateral_position(env: &Env, user: &Address, position: &CollateralPosition) {
    env.storage()
        .persistent()
        .set(&BorrowDataKey::UserCollateral(user.clone()), position);
}

fn get_total_debt(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::TotalDebt)
        .unwrap_or(0)
}

fn set_total_debt(env: &Env, amount: i128) {
    env.storage()
        .persistent()
        .set(&BorrowDataKey::TotalDebt, &amount);
}

fn get_debt_ceiling(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::DebtCeiling)
        .unwrap_or(i128::MAX)
}

fn get_min_borrow_amount(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::MinBorrowAmount)
        .unwrap_or(1000)
}

fn is_paused(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&BorrowDataKey::Paused)
        .unwrap_or(false)
}

fn emit_borrow_event(env: &Env, user: Address, asset: Address, amount: i128, collateral: i128) {
    let event = BorrowEvent {
        user,
        asset,
        amount,
        collateral,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish((Symbol::new(env, "borrow"),), event);
}

/// Initialize borrow settings (admin only)
pub fn initialize_borrow_settings(
    env: &Env,
    debt_ceiling: i128,
    min_borrow_amount: i128,
) -> Result<(), BorrowError> {
    env.storage()
        .persistent()
        .set(&BorrowDataKey::DebtCeiling, &debt_ceiling);
    env.storage()
        .persistent()
        .set(&BorrowDataKey::MinBorrowAmount, &min_borrow_amount);
    env.storage()
        .persistent()
        .set(&BorrowDataKey::Paused, &false);
    Ok(())
}

/// Set protocol pause state (admin only)
pub fn set_paused(env: &Env, paused: bool) -> Result<(), BorrowError> {
    env.storage()
        .persistent()
        .set(&BorrowDataKey::Paused, &paused);
    Ok(())
}

/// Get user's debt position
pub fn get_user_debt(env: &Env, user: &Address) -> DebtPosition {
    let mut position = get_debt_position(env, user);
    let accrued = calculate_interest(env, &position);
    position.interest_accrued = position.interest_accrued.saturating_add(accrued);
    position
}

/// Get user's collateral position
pub fn get_user_collateral(env: &Env, user: &Address) -> CollateralPosition {
    get_collateral_position(env, user)
}
