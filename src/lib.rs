#![feature(rustc_private)] // decl_storage extra genesis bug
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, Parameter};
use sp_arithmetic::traits::{BaseArithmetic, CheckedAdd, One};
use sp_runtime::traits::{MaybeSerializeDeserialize, Member};
use system::ensure_signed;

use crate::oracle::OracleError as InternalError;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod external_value;
mod oracle;
mod period_handler;

use crate::period_handler::PeriodHandler;

type AccountId<T> = <T as system::Trait>::AccountId;

pub trait Trait:
    system::Trait + timestamp::Trait + tablescore::Trait<TargetType = AccountId<Self>>
{
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type OracleId: Default + Parameter + Member + Copy + BaseArithmetic + MaybeSerializeDeserialize;
    type ValueType: Default + Parameter + Member + Copy + BaseArithmetic;
}

type Moment<T> = <T as timestamp::Trait>::Moment;
type AssetId<T> = <T as assets::Trait>::AssetId;

type Oracle<T> = crate::oracle::Oracle<
    <T as tablescore::Trait>::TableId,
    <T as Trait>::ValueType,
    Moment<T>,
    AccountId<T>,
>;

decl_storage! {
    trait Store for Module<T: Trait> as OracleModule
    {
        pub Oracles get(fn oracles): map hasher(blake2_256) T::OracleId => Oracle<T>;
        OracleIdSequence get(fn next_oracle_id): T::OracleId;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        OracleId = <T as Trait>::OracleId,
        ValueType = <T as Trait>::ValueType,
        ValueId = u8,
    {
        OracleCreated(OracleId, AccountId),
        OracleUpdated(OracleId, ValueId, ValueType),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        NoneValue,
        OracleIdOverflow,
        WrongPeriods,
        WrongValuesCount,
        WrongValueId,
        NotAggregationTime,
        NotCalculateTime,
        NotEnoughSources,
        NotEnoughValues,
        NotCalculatedValue,
        AccountPermissionDenied,
    }
}

impl<T: Trait> From<InternalError> for Error<T>
{
    fn from(error: InternalError) -> Self
    {
        match error
        {
            InternalError::FewSources(_exp, _act) => Error::<T>::NotEnoughSources,
            InternalError::FewPushedValue(_exp, _act) => Error::<T>::NotEnoughValues,
            InternalError::WrongValuesCount(_exp, _act) => Error::<T>::WrongValuesCount,
            InternalError::WrongValueId(_asset) => Error::<T>::WrongValueId,
            InternalError::UncalculatedValue(_asset) => Error::<T>::NotCalculatedValue,
            InternalError::SourcePermissionDenied => Error::<T>::AccountPermissionDenied,
            InternalError::CalculationError => Error::<T>::NoneValue,
        }
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        pub fn create_oracle(origin,
            name: Vec<u8>,
            source_limit: u8,
            calculate_period: Moment<T>,
            aggregate_period: Moment<T>,
            asset_id: AssetId<T>,
            values_names: Vec<Vec<u8>>,
        ) -> dispatch::DispatchResult
        {
            let who = ensure_signed(origin)?;
            let now = timestamp::Module::<T>::get();
            let period = PeriodHandler::new(now, calculate_period, aggregate_period)
                .map_err(|_| Error::<T>::WrongPeriods)?;

            let table = tablescore::Module::<T>::create(who.clone(), asset_id, source_limit, Some(name.clone()))?;

            let id = Self::get_next_oracle_id()?;
            Oracles::<T>::insert(id, Oracle::<T>::new(name, table, period, source_limit, values_names));

            Self::deposit_event(RawEvent::OracleCreated(id, who));

            Ok(())
        }

        pub fn push(origin,
            oracle_id: T::OracleId,
            values: Vec<T::ValueType>) -> dispatch::DispatchResult
        {
            let who = ensure_signed(origin)?;
            let now = timestamp::Module::<T>::get();

            let oracle = Oracles::<T>::get(oracle_id);

            if oracle.sources.is_empty()
                || oracle.period_handler.is_sources_update_needed(now)
            {
                Self::update_accounts(oracle_id)
                    .map_err(Error::<T>::from)?;
            }

            if !oracle.period_handler.is_can_aggregate(now)
            {
                Err(Error::<T>::NotAggregationTime)?;
            }

            Oracles::<T>::mutate(oracle_id, |oracle| {
                oracle.push_values(
                    &who,
                    now,
                    values.into_iter(),
                )
            })
            .map_err(Error::<T>::from)?;

            Ok(())
        }

        pub fn calculate(origin,
            oracle_id: T::OracleId,
            value_id: u8) -> dispatch::DispatchResult
        {
            ensure_signed(origin)?;
            let now = timestamp::Module::<T>::get();
            let oracle = Oracles::<T>::get(oracle_id);

            if oracle.period_handler.is_sources_update_needed(now)
            {
                Self::update_accounts(oracle_id).map_err(Error::<T>::from)?;
            }

            if !oracle.is_calculate_time(value_id as usize, now).map_err(Error::<T>::from)?
            {
                Err(Error::<T>::NotCalculateTime)?;
            }

            let new_value = Oracles::<T>::mutate(oracle_id, |oracle| {
                oracle.calculate_value(value_id as usize, now)
            }).map_err(Error::<T>::from)?;

            Self::deposit_event(RawEvent::OracleUpdated(oracle_id, value_id, new_value));

            Ok(())
        }
    }
}

impl<T: Trait> Module<T>
{
    fn get_next_oracle_id() -> Result<T::OracleId, Error<T>>
    {
        OracleIdSequence::<T>::mutate(|id| match id.checked_add(&One::one())
        {
            Some(res) =>
            {
                let result = *id;
                *id = res;
                Ok(result)
            }
            None => Err(Error::<T>::OracleIdOverflow),
        })
    }

    fn update_accounts(oracle_id: T::OracleId) -> Result<Vec<AccountId<T>>, InternalError>
    {
        Oracles::<T>::mutate(oracle_id, |oracle| {
            let table = tablescore::Module::<T>::tables(oracle.get_table());
            let accounts = oracle.update_accounts(table.get_head().into_iter().cloned())?;

            Ok(accounts.into_iter().cloned().collect())
        })
    }

    fn get_external_value(
        oracle_id: T::OracleId,
        value_id: usize,
    ) -> Result<(T::ValueType, Moment<T>), Error<T>>
    {
        Oracles::<T>::get(oracle_id)
            .values
            .get(value_id)
            .ok_or(Error::<T>::WrongValueId)?
            .get()
            .ok_or(Error::<T>::NotCalculatedValue)
    }
}
