use cosmwasm_std::{Uint128, Decimal};

/// Returns the multiplier over the given _from_ and _to_ range.
/// The multiplier is zero if the _to_ range is before the _end_.
/// The multiplier is the _end_ minus _from_ if the _from_ range is after the _end_.
/// Otherwise, the multiplier is the _to_ minus _from_.
pub fn get_multiplier(from: u64, to: u64, end: u64) -> u64 {
    if to < end {
        return to - from;
    } else if from >= end {
        return 0;
    }
    end - from
}

/// Calculates the reward amount
pub fn calc_reward_amount(
    staked_amount: Uint128,
    accrued_token_per_share: Decimal,
    reward_debt: Uint128,
) -> Uint128 {
        (staked_amount * accrued_token_per_share)
        .checked_sub(reward_debt)
        .unwrap_or(Uint128::zero())
}

pub fn update_pool(
    end_time: u64,
    reward_per_second: Decimal,
    staked_token_supply: Uint128,
    accrued_token_per_share: Decimal,
    current_time: u64,
    last_reward_time: u64,
) -> (Decimal, u64) {

    // If current time is before start time or after end time or before last reward time, return without update pool
    if current_time < last_reward_time {
        return (accrued_token_per_share, last_reward_time);
    }

    // Check if there is any reward token in the pool
    if staked_token_supply == Uint128::zero() {
        // No reward token in the pool, save last reward time and return
        (Decimal::zero(), last_reward_time)
    } else {
        let multiplier = get_multiplier(
            last_reward_time,
            current_time,
            end_time,
        );

        let reward = Decimal::new(multiplier.into()) * reward_per_second;
        let new_accrued_token_per_share
            = accrued_token_per_share
            + (reward / Decimal::new(staked_token_supply.into()));

        (new_accrued_token_per_share, current_time)
    }
}
