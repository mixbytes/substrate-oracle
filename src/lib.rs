#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, Parameter};
use sp_arithmetic::traits::{BaseArithmetic, CheckedAdd, One};
use sp_runtime::traits::Member;
use system::ensure_signed;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod external_value;
mod oracle;
mod period_handler;

use crate::period_handler::PeriodHandler;

pub trait Trait: system::Trait + timestamp::Trait + tablescore::Trait
{
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type OracleId: Default + Parameter + Member + Copy + BaseArithmetic + CheckedAdd + One;
    type ValueType: Default + Parameter + Member + Copy + BaseArithmetic;
}

type Moment<T> = <T as timestamp::Trait>::Moment;
type AssetId<T> = <T as assets::Trait>::AssetId;

type Oracle<T> = crate::oracle::Oracle<
    <T as tablescore::Trait>::TableId,
    <T as Trait>::ValueType,
    Moment<T>,
    <T as system::Trait>::AccountId,
>;

decl_storage! {
    trait Store for Module<T: Trait> as TemplateModule
    {
        pub Oracles get(fn oracles): map hasher(blake2_256) T::OracleId => Oracle<T>;

        OracleIdSequence get(fn next_oracle_id): T::OracleId;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        OracleUpdated(u32, AccountId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        NoneValue,
        OracleIdOverflow,
        WrongPeriods,
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
            assets_name: Vec<Vec<u8>>,
        ) -> dispatch::DispatchResult
        {
            let who = ensure_signed(origin)?;
            let now = timestamp::Module::<T>::get();
            let period = PeriodHandler::new(now, calculate_period, aggregate_period).map_err(|_| Error::<T>::WrongPeriods)?;

            let table = tablescore::Module::<T>::create(who, asset_id, source_limit, Some(name.clone()))?;

            let id = Self::get_next_oracle_id()?;
            Oracles::<T>::insert(id, Oracle::<T>::new(name, table, period, source_limit, assets_name));

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
}
