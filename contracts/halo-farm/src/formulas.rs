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

#[cfg(test)]
#[test]
fn test_get_new_reward_ratio_and_time() {
    use cosmwasm_std::Addr;

    let phase_info = PhaseInfo {
        start_time: 100,
        end_time: 200,
        whitelist: Addr::unchecked("addr0000"),
        reward_balance: Uint128::new(1000),
        last_reward_time: 100,
        accrued_token_per_share: Decimal::zero(),
    };

    // No staked token in the farming pool
    let (new_accrued_token_per_share, new_last_reward_time) =
        phase_info.get_new_reward_ratio_and_time(150, Uint128::zero());
    assert_eq!(new_accrued_token_per_share, Decimal::zero());
    assert_eq!(new_last_reward_time, 100);

    // Staked token in the farming pool but current time is before last reward time
    let (new_accrued_token_per_share, new_last_reward_time) =
        phase_info.get_new_reward_ratio_and_time(50, Uint128::new(100));

    assert_eq!(new_accrued_token_per_share, Decimal::zero());
    assert_eq!(new_last_reward_time, 100);

    // Staked token in the farming pool
    let (new_accrued_token_per_share, new_last_reward_time) =
        phase_info.get_new_reward_ratio_and_time(150, Uint128::new(100));
    assert_eq!(new_accrued_token_per_share, Decimal::percent(500));
    assert_eq!(new_last_reward_time, 150);

    // Staked token in the farming pool
    let (new_accrued_token_per_share, new_last_reward_time) =
        phase_info.get_new_reward_ratio_and_time(250, Uint128::new(100));
    assert_eq!(new_accrued_token_per_share, Decimal::percent(1000));
    assert_eq!(new_last_reward_time, 250);
}
