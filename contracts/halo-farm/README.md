# The farm contract
## Introduction
Each farm is a contract that allows users to deposit, withdraw their LP token to harvest reward. The contract is deployed by the factory owner.

## InstantiateMsg
```javascript
{
    "staked_token": "aura1...",
    "reward_token": "uaura",
    "start_time": 1689148800
    "end_time": 1689192000
    "phases_limit_per_user": 1000000000000000000
    "farm_owner": "aura1..."
    "whitelist": "aura1..."
}
```
Where:
- `staked_token`: The LP token that users will deposit to the farm.
- `reward_token`: The token that users will receive as reward. It can be a native token or a CW-20 token.
- `start_time`: The time when the farm starts.
- `end_time`: The time when the farm ends.
- `phases_limit_per_user`: The maximum amount of phases that a user can deposit to the farm.
- `farm_owner`: The owner of the farm contract.
- `whitelist`: The address of the whitelist. Whitelist is a wallet that can add reward token balance to the farm contract.

## ExecuteMsg
### AddRewardBalance
```javascript
{
    "add_reward_balance": {
        "phase_index": 0,
        "amount": "1000000000000000000"
    }
}
```
It can be called by the whitelist only.

Where:
- `phase_index`: The index of the phase that the reward balance will be added to.
- `amount`: The amount of reward token that will be added to the farm contract.

### Deposit
```javascript
{
    "deposit": {
        "amount": "1000000000000000000"
    }
}
```
Where:
- `amount`: The amount of LP token that will be deposited to the farm contract.

### Withdraw
```javascript
{
    "withdraw": {
        "amount": "1000000000000000000"
    }
}
```
Where:
- `amount`: The amount of LP token that will be withdrawn from the farm contract.

### Harvest
```javascript
{
    "harvest": {}
}
```
Harvest the reward token from the farm contract.

### AddPhase
```javascript
{
    "add_phase": {
        "new_start_time": 1689148801
        "new_end_time": 1689192001
        "whitelist": "aura1..."
    }
}
```
It can be called by the farm owner only and before the new start time.

Where:
- `new_start_time`: The start time of the new phase.
- `new_end_time`: The end time of the new phase.
- `whitelist`: The address of the whitelist. Whitelist is a wallet that can add reward token balance to the farm contract.

### RemovePhase
```javascript
{
    "remove_phase": {
        "phase_index": 0
    }
}
```
Where:
- `phase_index`: The index of the phase that will be removed. It can be called by the farm owner only and before the start time. If the phase has already added reward balance, the balance will be sent to the whitelist.

### ActivatePhase
```javascript
{
    "activate_phase": {}
}
```
Active the latest phase. It can be called by the farm owner only and before the start time.

## QueryMsg
### Farm
```javascript
{
    "farm": {}
}
```
#[returns(FarmInfo)]
Returns the information of the farm contract.

### PendingReward
```javascript
{
    "pending_reward": {
        "address": "aura1..."
    }
}
```
#[returns(PendingRewardResponse)]
Returns the pending reward of the given user address.

### TotalStaked
```javascript
{
    "total_staked": {}
}
```
#[returns(Uint128)]
Returns the total LP staked amount of the farm contract.

### StakerInfo
```javascript
{
    "staker_info": {
        "address": "aura1..."
    }
}
```
#[returns(StakerInfoResponse)]
Returns the staker info of the given user address.
