use crate::state::PoolInfo;
use cosmwasm_std::Uint128;

pub fn calc_reward(pool_info: &PoolInfo, current_time: u64) -> Uint128 {
    let _reward_per_second = pool_info.reward_per_second;
    let start_time = pool_info.start_time;
    let end_time = pool_info.end_time;

    if current_time < start_time {
        return Uint128::zero();
    }

    if current_time >= end_time {
        return (end_time - start_time).into();
    }
    (current_time - start_time).into()
}

/// Update pool info and return the accrued_token_per_share: Uint128 and the last_reward_time: u64.
pub fn update_pool(
    pool_info: &mut PoolInfo,
    staked_token_supply: Uint128,
    accrued_token_per_share: Uint128,
    current_time: u64,
    last_reward_time: u64,
) -> (Uint128, u64) {
    let reward_per_second = pool_info.reward_per_second;
    let start_time = pool_info.start_time;
    let end_time = pool_info.end_time;

    if current_time < start_time {
        return (Uint128::zero(), last_reward_time);
    }

    if current_time >= end_time {
        return (accrued_token_per_share, end_time);
    }

    let multiplier = get_multiplier(last_reward_time, current_time, end_time);
    let reward = Uint128::new(multiplier.into()) * reward_per_second;
    let new_accrued_token_per_share =
        accrued_token_per_share + reward * Uint128::new(1_000_000) / staked_token_supply;
    (new_accrued_token_per_share, current_time)
}

/// Returns the multiplier over the given _from_ and _to_ range.
/// The multiplier is zero if the _to_ range is before the _end_.
/// The multiplier is the _end_ minus _from_ if the _from_ range is after the _end_.
/// Otherwise, the multiplier is the _to_ minus _from_.
fn get_multiplier(from: u64, to: u64, end: u64) -> u64 {
    if to < end {
        return to - from;
    } else if from >= end {
        return 0;
    }
    end - from
}
