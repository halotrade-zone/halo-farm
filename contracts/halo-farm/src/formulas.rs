use cosmwasm_std::{Decimal, Uint128};

use crate::state::PhaseInfo;

/// Returns the multiplier over the given _from_ and _to_ range.
/// The multiplier is zero if the _to_ range is before the _end_.
/// The multiplier is the _end_ minus _from_ if the _from_ range is after the _end_.
/// Otherwise, the multiplier is the _to_ minus _from_.
pub fn get_multiplier(from: u64, to: u64, end: u64) -> u64 {
    if to <= end {
        return to - from;
    } else if from >= end {
        return 0;
    }
    // If the phase has ended, the multiplier is the end minus from
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

impl PhaseInfo {
    pub fn get_new_reward_ratio_and_time(
        &self,
        current_time: u64,
        staked_token_balance: Uint128,
    ) -> (Decimal, u64) {
        // If current time is before last reward time, return without updating
        if current_time < self.last_reward_time {
            return (self.accrued_token_per_share, self.last_reward_time);
        }

        // Check if there is any staked token in the farming pool
        if staked_token_balance == Uint128::zero() {
            // No staked token in the farming pool, save last reward time and return
            (Decimal::zero(), self.last_reward_time)
        } else {
            let multiplier = get_multiplier(self.last_reward_time, current_time, self.end_time);
            let reward = Uint128::new(multiplier.into()) * self.reward_balance
                / Uint128::new((self.end_time - self.start_time).into());

            let new_accrued_token_per_share = self.accrued_token_per_share
                + (Decimal::new(reward) / Decimal::new(staked_token_balance));

            (new_accrued_token_per_share, current_time)
        }
    }
}
