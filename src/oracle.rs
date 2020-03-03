use codec::{Decode, Encode};
use rstd::cmp::{Ord, Ordering};
use rstd::collections::btree_map::BTreeMap;
use rstd::prelude::Vec;
use sp_arithmetic::traits::BaseArithmetic;

use crate::external_value::{get_median, ExternalValue, Median};
use crate::period_handler::PeriodHandler;

type RawString = Vec<u8>;

#[derive(Clone, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum OracleError
{
    FewSources(usize, usize),
    FewPushedValue(usize, usize),
    WrongAssetsCount(usize, usize),
    WrongAssetId(usize),
    UncalculatedAsset(usize),
    SourcePermissionDenied,
    CalculationError,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Oracle<
    TableId: Default,
    ValueType: Default + Clone,
    Moment: Default + Clone,
    SourceId: Default + Ord,
> {
    pub name: RawString,
    table: TableId,

    source_limit: u8,
    pub period_handler: PeriodHandler<Moment>,

    pub sources: BTreeMap<SourceId, Vec<ExternalValue<ValueType, Moment>>>,
    pub names: Vec<RawString>,
    pub values: Vec<ExternalValue<ValueType, Moment>>,
}

impl<
        TableId: Default,
        ValueType: Default + Clone,
        Moment: Default + Clone,
        SourceId: Default + Ord,
    > Oracle<TableId, ValueType, Moment, SourceId>
{
    fn get_table(&self) -> &TableId
    {
        &self.table
    }
}

impl<
        TableId: Default,
        ValueType: Default + Copy + BaseArithmetic,
        Moment: Default + Copy + BaseArithmetic,
        SourceId: Default + Ord,
    > Oracle<TableId, ValueType, Moment, SourceId>
{
    pub fn new(
        name: RawString,
        table: TableId,
        period_handler: PeriodHandler<Moment>,
        source_limit: u8,
        assets_name: Vec<RawString>,
    ) -> Self
    {
        Oracle {
            name,
            table,
            period_handler,
            source_limit,
            sources: BTreeMap::default(),
            values: rstd::iter::repeat_with(ExternalValue::<ValueType, Moment>::default)
                .take(assets_name.len())
                .collect(),
            names: assets_name,
        }
    }

    pub fn get_assets_count(&self) -> usize
    {
        self.names.len()
    }

    pub fn is_ex_asset_id_correct(&self, ex_asset_id: usize) -> Result<(), OracleError>
    {
        if ex_asset_id < self.get_assets_count()
        {
            Ok(())
        }
        else
        {
            Err(OracleError::WrongAssetId(ex_asset_id))
        }
    }

    pub fn is_sources_enough(&self) -> bool
    {
        (self.sources.len() as u8) >= self.source_limit
    }

    pub fn is_calculate_time(&self, ex_asset_id: usize, now: Moment) -> Result<bool, OracleError>
    {
        self.is_ex_asset_id_correct(ex_asset_id)?;
        Ok(self
            .period_handler
            .is_calculate_time(self.values[ex_asset_id].last_changed, now))
    }

    pub fn add_assets(&mut self, name: RawString)
    {
        self.names.push(name);
        self.values.push(ExternalValue::default());
    }

    pub fn update_accounts<I>(&mut self, sources: I) -> Result<Vec<&SourceId>, OracleError>
    where
        I: Iterator<Item = SourceId>,
    {
        let default: Vec<ExternalValue<ValueType, Moment>> =
            rstd::iter::repeat_with(ExternalValue::<ValueType, Moment>::default)
                .take(self.get_assets_count())
                .collect();

        self.sources = sources
            .map(|account| {
                let external_value = match self.sources.get(&account)
                {
                    Some(ex_val) => ex_val.clone(),
                    None => default.clone(),
                };
                (account, external_value)
            })
            .collect();

        if self.is_sources_enough()
        {
            Ok(self.sources.iter().map(|(src, _)| src).collect())
        }
        else
        {
            Err(OracleError::FewSources(
                self.source_limit as usize,
                self.sources.len(),
            ))
        }
    }

    pub fn push_values<I>(
        &mut self,
        source: &SourceId,
        now: Moment,
        values: I,
    ) -> Result<(), OracleError>
    where
        I: Iterator<Item = ValueType>,
    {
        self.sources
            .get_mut(source)
            .map(|assets| {
                assets
                    .iter_mut()
                    .zip(values)
                    .for_each(|(value, new)| value.update(new, now));
            })
            .ok_or(OracleError::SourcePermissionDenied)
    }

    fn get_actual_values(&self, ex_asset_id: usize) -> Result<Vec<&ValueType>, OracleError>
    {
        self.is_ex_asset_id_correct(ex_asset_id)?;

        Ok(self
            .sources
            .iter()
            .map(|(_, assets)| assets.get(ex_asset_id))
            .filter(|ext| ext.and_then(|val| val.value.as_ref()).is_some())
            .map(|ext| ext.as_ref().map(|val| val.value.as_ref().unwrap()).unwrap())
            .collect())
    }

    pub fn pull_value(&mut self, ex_asset_id: usize) -> Result<(ValueType, Moment), OracleError>
    {
        self.is_ex_asset_id_correct(ex_asset_id)?;

        if let (Some(value), Some(moment)) = (
            self.values[ex_asset_id].value,
            self.values[ex_asset_id].last_changed,
        )
        {
            Ok((value, moment))
        }
        else
        {
            Err(OracleError::UncalculatedAsset(ex_asset_id))
        }
    }

    pub fn calculate_value(
        &mut self,
        ex_asset_id: usize,
        now: Moment,
    ) -> Result<ValueType, OracleError>
    {
        if !self.is_sources_enough()
        {
            return Err(OracleError::FewSources(
                self.source_limit as usize,
                self.get_assets_count(),
            ));
        }

        let assets: Vec<&ValueType> = self.get_actual_values(ex_asset_id)?;

        if self.source_limit as usize > assets.len()
        {
            return Err(OracleError::FewPushedValue(
                self.source_limit as usize,
                assets.len(),
            ));
        }

        match get_median(assets)
        {
            Some(Median::Value(value)) => Ok(value.clone()),
            Some(Median::Pair(left, right)) =>
            {
                let sum = *left + *right;
                let div = ValueType::one() + ValueType::one();
                Ok(sum / div)
            }
            _ => Err(OracleError::CalculationError),
        }
        .map(|res| {
            self.values[ex_asset_id].update(res, now);
            res
        })
    }
}

#[cfg(test)]
mod tests
{
    type Oracle = super::Oracle<u32, u32, u32, u32>;
    type PeriodHandler = super::PeriodHandler<u32>;
    type OE = super::OracleError;

    const ALICE: u32 = 100;
    const BOB: u32 = 132;
    const CHUCK: u32 = 224;
    const CRAIG: u32 = 342;
    const DAN: u32 = 424;
    const EVE: u32 = 235;
    const ERING: u32 = 643;
    const CAROL: u32 = 199;

    const ACCOUNTS: [u32; 8] = [ALICE, BOB, CHUCK, CRAIG, DAN, EVE, ERING, CAROL];

    const TABLE_ID: u32 = 0;
    const SOURCE_LIMIT: u8 = 4;

    fn create_period_handler() -> PeriodHandler
    {
        super::PeriodHandler::new(100, 10, 5).unwrap()
    }

    fn get_assets_names() -> Vec<&'static str>
    {
        vec!["f", "s", "t", "f", "f", "s"]
    }

    fn get_assets_value(value: u32) -> Vec<u32>
    {
        rstd::iter::repeat(value)
            .take(get_assets_names().len())
            .collect()
    }

    fn create_oracle() -> Oracle
    {
        Oracle::new(
            "test".to_owned().as_bytes().to_vec(),
            TABLE_ID,
            create_period_handler(),
            SOURCE_LIMIT,
            get_assets_names()
                .iter()
                .map(|s| s.to_string().as_bytes().to_vec())
                .collect(),
        )
    }

    #[test]
    fn create()
    {
        let oracle = create_oracle();

        assert_eq!(oracle.get_table().clone(), TABLE_ID);
        assert_eq!(oracle.get_assets_count(), get_assets_names().len());
        assert!(oracle.values.iter().all(|val| val.is_clean()));
        assert_eq!(oracle.sources.len(), 0);
    }

    #[test]
    fn accounts()
    {
        let mut oracle = create_oracle();

        let accounts = oracle.update_accounts(ACCOUNTS.to_vec().into_iter());

        assert!(accounts.is_ok());
        assert_eq!(accounts.unwrap().len(), ACCOUNTS.len());

        assert_eq!(oracle.sources.len(), ACCOUNTS.len());

        for (_account, values) in oracle.sources.iter()
        {
            assert!(values.iter().all(|val| val.is_clean()));
            assert_eq!(values.len(), get_assets_names().len());
            assert!(values.iter().all(|ext| ext.is_clean()));
        }
    }

    #[test]
    fn push_simple()
    {
        let mut oracle = create_oracle();

        oracle
            .update_accounts(ALICE..=CAROL)
            .expect("Update accounts error.");

        for account in ALICE..=CAROL
        {
            assert_eq!(
                oracle.push_values(&account, 10, get_assets_value(10).into_iter()),
                Ok(())
            );
        }

        for i in 0..get_assets_names().len()
        {
            assert_eq!(oracle.calculate_value(i, 14), Ok(10));
        }
    }

    macro_rules! assert_ok {
        ($x:expr) => {
            assert_eq!($x, Ok(()));
        };
    }

    #[test]
    fn push()
    {
        let mut oracle = create_oracle();

        oracle
            .update_accounts(ACCOUNTS.to_vec().into_iter())
            .expect("Update accounts error.");

        assert_ok!(oracle.push_values(&BOB, 11, vec![124, 1, 1, 1, 1, 5476346].into_iter()));
        assert_ok!(oracle.push_values(&DAN, 17, vec![128, 1, 1, 1, 1, 5476387].into_iter()));
        assert_ok!(oracle.push_values(&EVE, 19, vec![126, 1, 1, 1, 1, 5476394].into_iter()));

        assert_eq!(
            oracle.calculate_value(0, 20),
            Err(OE::FewPushedValue(4, 3))
        );
        assert_eq!(oracle.pull_value(0), Err(OE::UncalculatedAsset(0)));

        assert_ok!(oracle.push_values(&ALICE, 20, vec![123, 1, 1, 1, 1, 5476378].into_iter()));

        assert_eq!(oracle.calculate_value(0, 20), Ok(125));
        assert_eq!(oracle.calculate_value(5, 20), Ok(5476382));
        assert_eq!(oracle.pull_value(5), Ok((5476382, 20)));
    }
}
