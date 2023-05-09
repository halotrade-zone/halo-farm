use cosmwasm_std::Uint128;
use crate::state::PoolInfo;

pub fn calc_reward(pool_info: &PoolInfo, current_time: u64) -> Uint128 {
    let reward_per_second = pool_info.reward_per_second;
    let start_time = pool_info.start_time;
    let end_time = pool_info.end_time;

    if current_time < start_time {
        return Uint128::zero();
    }

    if current_time >= end_time {
        return reward_per_second.multiply_ratio(end_time - start_time, 1u64);
    }

    reward_per_second.multiply_ratio(current_time - start_time, 1u64)
}