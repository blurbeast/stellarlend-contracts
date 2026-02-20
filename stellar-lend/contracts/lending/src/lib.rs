#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env};

mod borrow;
use borrow::{
    borrow, get_user_collateral, get_user_debt, initialize_borrow_settings, set_paused,
    BorrowError, CollateralPosition, DebtPosition,
};

mod cross_asset;
use cross_asset::{
    borrow_asset, deposit_collateral_asset, get_cross_position_summary, repay_asset,
    set_asset_params, withdraw_asset, AssetParams, CrossAssetError, PositionSummary,
};

#[cfg(test)]
mod borrow_test;

#[cfg(test)]
mod cross_asset_test;

#[contract]
pub struct LendingContract;

#[contractimpl]
impl LendingContract {
    pub fn borrow(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
        collateral_asset: Address,
        collateral_amount: i128,
    ) -> Result<(), BorrowError> {
        borrow(
            &env,
            user,
            asset,
            amount,
            collateral_asset,
            collateral_amount,
        )
    }

    pub fn initialize_borrow_settings(
        env: Env,
        debt_ceiling: i128,
        min_borrow_amount: i128,
    ) -> Result<(), BorrowError> {
        initialize_borrow_settings(&env, debt_ceiling, min_borrow_amount)
    }

    pub fn set_paused(env: Env, paused: bool) -> Result<(), BorrowError> {
        set_paused(&env, paused)
    }

    pub fn get_user_debt(env: Env, user: Address) -> DebtPosition {
        get_user_debt(&env, &user)
    }

    pub fn get_user_collateral(env: Env, user: Address) -> CollateralPosition {
        get_user_collateral(&env, &user)
    }

    pub fn set_asset_params(
        env: Env,
        asset: Address,
        params: AssetParams,
    ) {
        set_asset_params(&env, asset, params).unwrap();
    }

    pub fn deposit_collateral_asset(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
    ) {
        deposit_collateral_asset(&env, user, asset, amount).unwrap();
    }

    pub fn borrow_asset(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
    ) {
        borrow_asset(&env, user, asset, amount).unwrap();
    }

    pub fn repay_asset(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
    ) {
        repay_asset(&env, user, asset, amount).unwrap();
    }

    pub fn withdraw_asset(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
    ) {
        withdraw_asset(&env, user, asset, amount).unwrap();
    }

    pub fn get_cross_position_summary(
        env: Env,
        user: Address,
    ) -> PositionSummary {
        get_cross_position_summary(&env, user).unwrap()
    }

    pub fn initialize_admin(env: Env, admin: Address) {
        cross_asset::initialize_admin(&env, admin);
    }
}
