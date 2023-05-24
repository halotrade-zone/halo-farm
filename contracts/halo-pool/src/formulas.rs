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
