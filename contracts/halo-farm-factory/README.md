# The factory contract is used to create new farms.

## Introduction
The factory contract will handle the information related to farms. It will also create new farming pools when factory owner provides the required information.

## InstantiateMsg
We must provide the farm contract code id of `halo_farm` contract. This is the code id of the contract that will be used to create new farms.
```javascript
{
    "farm_code_id": 1
}
```

## ExecuteMsg

### UpdateConfig
```javascript
{
    "update_config": {
        "owner": "aura1...",
        "farm_code_id": 2,
    }
}
```
Where:
- `owner`: The new owner of the factory contract.
- `farm_code_id`: The new farm code id of the contract that will be used to create new farms.

### CreateFarm
```javascript
{
    "create_farm": {
        "create_farm_msg": // a base64 encoded json object
    }
}
```
Where:
- `create_farm_msg`: The message that will be sent to the farm contract to create a new farm. The message must be base64 encoded. For more information about the message, please refer to the [halo-farm](../halo-farm/README.md) contract.

## QueryMsg
### Config
```javascript
{
    "config": {}
}
```
#[returns(ConfigResponse)]
Returns the current configuration of the factory contract.

### Farm
```javascript
{
    "farm": {
        "farm_id": 1
    }
}
```
#[returns(FactoryFarmInfo)]
Returns the information of the farm with the given id.

### Farms
```javascript
{
    "farms": {
        "start_after": 1,
        "limit": 10
    }
}
```
#[returns(Vec<FactoryFarmInfo>)]
Returns the list of farms. The list is paginated and the `start_after` field is used to determine the starting point of the list. The `limit` field is used to determine the number of farms to return.


