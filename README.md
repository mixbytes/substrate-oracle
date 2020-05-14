# oracle-pallet for Substrate

## Overview
Pallet for work with oracles

## Description
You can create an oracle and use a tablescore-pallet to maintain source pool to provide an average final value.

In pallet public API we have methods:
```rust
/// Create oracle in runtime
///
///  * `name` - A raw string for identify oracle
///  * `source_limit` - Lower limit of the number of sources
///  * `period` - Defines oracle work cycle. Period have aggregate and calculate part.
///  * `aggregate_period` - Part of period when sources can push values. The rest part of
///  period - `calculate_part` when we can calculate from pushed values.
///  * `asset_id` - Asset with the help of which voting is carried out in tablescore
///  * `values_names` - Names of all external values for oracle
///
pub fn create_oracle(origin,
    name: Vec<u8>,
    source_limit: u8,
    period: Moment<T>,
    aggregate_period: Moment<T>,
    asset_id: AssetId<T>,
    values_names: Vec<Vec<u8>>,
) -> dispatch::DispatchResult;

/// Push values to oracle
///
/// In order to push, you need some conditions:
/// - You must be the winner from tablescore
/// - `values` must be the right size
/// - There must be an aggregation period
pub fn push(origin,
    oracle_id: T::OracleId,
    values: Vec<T::ValueType>) -> dispatch::DispatchResult;

/// Calculate value in oracle
///
/// In order to calculate, you need some conditions:
/// - There must be a calculate period part or in the previous
/// calculate period part the value was not calculated
/// - There are enough pushed values in oracle
pub fn calculate(origin,
    oracle_id: T::OracleId,
    value_id: u8) -> dispatch::DispatchResult;
```

## Build

```console
# Build
cargo build

# Build as wasm
cargo wbuild

# Test pallet
cargo test
```

## Example
Example of selecting a subset of accounts by tablescore

```rust
pub trait Trait: oracle::Trait<ValueType=u128> {
    ...
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        pub fn work_with_oracle(origin, oracle_id: <T as oracle::Trait>::OracleId) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            let external_value = oracle::Module::<T>::get_external_value(oracle_id, value_id)?;

            /// Work with external value

            Ok(())
        }
}
```
