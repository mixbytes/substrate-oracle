use codec::{Decode, Encode};
use rstd::cmp::Ord;
use rstd::collections::btree_map::BTreeMap;
use rstd::prelude::Vec;
use sp_arithmetic::traits::SimpleArithmetic;

use crate::external_value::{get_median, ExternalValue, Median};
use crate::period_handler::{Part, PeriodHandler};

type RawString = Vec<u8>;

#[derive(Clone, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum OracleError {
    /// Not enough sources for oracle work
    FewSources(usize, usize),

    /// Not enough pushed values for calculate
    FewPushedValue(usize, usize),

    /// No push in period - can't calculate value
    EmptyPushedValueInPeriod,

    /// The pushed values vector is not the right size.
    WrongValuesCount(usize, usize),

    /// Value id (number in vector) is wrong
    WrongValueId(usize),

    /// Value not calculated
    UncalculatedValue(usize),

    /// Source not in list
    SourcePermissionDenied,

    /// Unknown error in calculate process
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
    /// Name of oracle
    pub name: RawString,

    /// ID in dpos-tablescore
    table: TableId,

    /// Lower limit of the number of sources
    source_limit: u8,

    /// Work with aggregate and calculate periods in oracle
    pub period_handler: PeriodHandler<Moment>,

    /// All pushed by sources data
    sources: BTreeMap<SourceId, Vec<ExternalValue<ValueType, Moment>>>,

    /// Names of external values
    pub names: Vec<RawString>,

    /// Vector of calculated values
    pub values: Vec<ExternalValue<ValueType, Moment>>,

    /// The last period when one of the sources pushed the values
    last_push_period: Option<Moment>,

    /// The `sources` field from previous period for lazy calculating in current period aggregate part
    prev_period_source: BTreeMap<SourceId, Vec<Option<ExternalValue<ValueType, Moment>>>>,
}

impl<
        TableId: Default,
        ValueType: Default + Clone,
        Moment: Default + Clone,
        SourceId: Default + Ord,
    > Oracle<TableId, ValueType, Moment, SourceId>
{
    pub fn get_table(&self) -> &TableId {
        &self.table
    }
}

impl<
        TableId: Default,
        ValueType: Default + Copy + SimpleArithmetic,
        Moment: Default + Copy + SimpleArithmetic,
        SourceId: Default + Ord + Clone,
    > Oracle<TableId, ValueType, Moment, SourceId>
{
    pub fn new(
        name: RawString,
        table: TableId,
        period_handler: PeriodHandler<Moment>,
        source_limit: u8,
        assets_name: Vec<RawString>,
    ) -> Self {
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
            last_push_period: None,
            prev_period_source: BTreeMap::default(),
        }
    }

    /// Count of values inside oracle
    pub fn get_values_count(&self) -> usize {
        self.names.len()
    }

    pub fn is_sources_empty(&self) -> bool {
        self.sources.is_empty()
    }

    pub fn is_value_id_correct(&self, value_id: usize) -> Result<(), OracleError> {
        if value_id < self.get_values_count() {
            Ok(())
        } else {
            Err(OracleError::WrongValueId(value_id))
        }
    }

    /// Is source enough for oracle work
    pub fn is_sources_enough(&self) -> bool {
        (self.sources.len() as u8) >= self.source_limit
    }

    /// Can we allow the calculation of a specific by id value?
    ///
    /// If now the calculation period and the value has not yet been calculated  - yes
    ///
    /// Can return `OracleError::WrongValueId(value_id)`
    pub fn is_allow_calculate(&self, value_id: usize, now: Moment) -> Result<bool, OracleError> {
        self.is_value_id_correct(value_id)?;
        Ok(self
            .period_handler
            .is_allow_calculate(self.values[value_id].last_changed, now))
    }

    pub fn add_assets(&mut self, name: RawString) {
        self.names.push(name);
        self.values.push(ExternalValue::default());
    }

    /// Update sources for oracle
    ///
    /// Return new vector of sources if success
    pub fn update_sources<I>(&mut self, sources: I) -> Result<Vec<&SourceId>, OracleError>
    where
        I: Iterator<Item = SourceId>,
    {
        let default: Vec<ExternalValue<ValueType, Moment>> =
            rstd::iter::repeat_with(ExternalValue::<ValueType, Moment>::default)
                .take(self.get_values_count())
                .collect();

        self.sources = sources
            .map(|account| {
                let external_value = match self.sources.get(&account) {
                    Some(ex_val) => ex_val.clone(),
                    None => default.clone(),
                };
                (account, external_value)
            })
            .collect();

        if self.is_sources_enough() {
            Ok(self.sources.iter().map(|(src, _)| src).collect())
        } else {
            Err(OracleError::FewSources(
                self.source_limit as usize,
                self.sources.len(),
            ))
        }
    }

    /// Store pushed data for previous period for late lazy-calculate
    fn store_pushed_data(&mut self, period_for_store: Moment) {
        // Store only for not calculated in period_for_store values
        let is_need_store_flags: Vec<bool> = self
            .values
            .iter()
            .map(|external| {
                if let Some(moment) = external.last_changed {
                    self.period_handler.get_period_number(moment) != period_for_store
                } else {
                    true
                }
            })
            .collect();

        self.prev_period_source = self
            .sources
            .iter()
            .map(|(source, external_vec)| {
                let data: Vec<Option<ExternalValue<ValueType, Moment>>> = external_vec
                    .iter()
                    .zip(is_need_store_flags.iter())
                    .map(|(val, is_need_store)| {
                        if *is_need_store {
                            Some(val.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                (source.clone(), data)
            })
            .collect();
    }

    fn clear_pushed_data(&mut self) {
        self.sources
            .iter_mut()
            .for_each(|(_source, external_values)| {
                external_values.iter_mut().for_each(|ext| ext.clean())
            });
    }

    pub fn push_values<I>(
        &mut self,
        source: &SourceId,
        now: Moment,
        new_values: I,
    ) -> Result<(), OracleError>
    where
        I: Iterator<Item = ValueType>,
    {
        let current = self.period_handler.get_period_number(now);

        // If this is first push in period - we store and clean previous sources data
        if matches!(self.last_push_period, Some(previous) if previous != current) {
            self.store_pushed_data(self.last_push_period.unwrap());
            self.clear_pushed_data();
        }
        self.last_push_period = Some(current);

        self.sources
            .get_mut(source)
            .map(|external_values| {
                external_values
                    .iter_mut()
                    .zip(new_values)
                    .for_each(|(external_value, new)| external_value.update(new, now));
            })
            .ok_or(OracleError::SourcePermissionDenied)
    }

    fn get_actual_value_variants(
        &self,
        ex_asset_id: usize,
        now: Moment,
    ) -> Result<Vec<&ValueType>, OracleError> {
        self.is_value_id_correct(ex_asset_id)?;

        Ok(match self.period_handler.get_part(now) {
            // Calculate with prev period data
            Part::Aggregate => self
                .prev_period_source
                .iter()
                .filter_map(|(_, assets)| {
                    assets
                        .get(ex_asset_id)
                        .and_then(|ex| ex.as_ref())
                        .and_then(|asset| asset.value.as_ref())
                })
                .collect(),

            // Calculate with current period data
            Part::Calculate => self
                .sources
                .iter()
                .filter_map(|(_, assets)| {
                    assets
                        .get(ex_asset_id)
                        .and_then(|asset| asset.value.as_ref())
                })
                .collect(),
        })
    }

    pub fn pull_value(&mut self, ex_asset_id: usize) -> Result<(ValueType, Moment), OracleError> {
        self.is_value_id_correct(ex_asset_id)?;

        if let (Some(value), Some(moment)) = (
            self.values[ex_asset_id].value,
            self.values[ex_asset_id].last_changed,
        ) {
            Ok((value, moment))
        } else {
            Err(OracleError::UncalculatedValue(ex_asset_id))
        }
    }

    pub fn calculate_value(
        &mut self,
        value_id: usize,
        now: Moment,
    ) -> Result<ValueType, OracleError> {
        if !self.is_sources_enough() {
            return Err(OracleError::FewSources(
                self.source_limit as usize,
                self.get_values_count(),
            ));
        }

        // If in current period nobody pushed (clean) values
        if match self.last_push_period {
            Some(period) => self.period_handler.get_period_number(now) != period,
            None => true,
        } {
            self.clear_pushed_data();
            return Err(OracleError::EmptyPushedValueInPeriod);
        }

        let values: Vec<&ValueType> = self.get_actual_value_variants(value_id, now)?;

        if self.source_limit as usize > values.len() {
            return Err(OracleError::FewPushedValue(
                self.source_limit as usize,
                values.len(),
            ));
        }

        match get_median(values) {
            Some(Median::Value(value)) => Ok(*value),
            Some(Median::Pair(left, right)) => {
                let sum = *left + *right;
                let div = ValueType::one() + ValueType::one();
                Ok(sum / div)
            }
            _ => Err(OracleError::CalculationError),
        }
        .map(|res| {
            self.values[value_id].update(res, now);
            res
        })
    }
}

#[cfg(test)]
mod tests {
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

    const BEGIN: u32 = 100;
    const PERIOD: u32 = 10;
    const AGGREGATE_PART: u32 = 5;
    const CALCULATE_BEGIN: u32 = BEGIN + AGGREGATE_PART + 1;

    fn create_period_handler() -> PeriodHandler {
        super::PeriodHandler::new(BEGIN, PERIOD, AGGREGATE_PART).unwrap()
    }

    fn get_assets_names() -> Vec<&'static str> {
        vec!["f", "s", "t", "f", "f", "s"]
    }

    fn get_assets_value(value: u32) -> Vec<u32> {
        rstd::iter::repeat(value)
            .take(get_assets_names().len())
            .collect()
    }
    fn create_oracle() -> Oracle {
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
    fn create() {
        let oracle = create_oracle();

        assert_eq!(oracle.get_table().clone(), TABLE_ID);
        assert_eq!(oracle.get_values_count(), get_assets_names().len());
        assert!(oracle.values.iter().all(|val| val.is_clean()));
        assert_eq!(oracle.sources.len(), 0);
    }

    #[test]
    fn accounts() {
        let mut oracle = create_oracle();

        let accounts = oracle.update_sources(ACCOUNTS.to_vec().into_iter());

        assert!(accounts.is_ok());
        assert_eq!(accounts.unwrap().len(), ACCOUNTS.len());

        assert_eq!(oracle.sources.len(), ACCOUNTS.len());

        for (_account, values) in oracle.sources.iter() {
            assert!(values.iter().all(|val| val.is_clean()));
            assert_eq!(values.len(), get_assets_names().len());
            assert!(values.iter().all(|ext| ext.is_clean()));
        }
    }

    #[test]
    fn push_simple() {
        let mut oracle = create_oracle();

        oracle
            .update_sources(ALICE..=CAROL)
            .expect("Update accounts error.");

        for account in ALICE..=CAROL {
            assert_eq!(
                oracle.push_values(&account, BEGIN, get_assets_value(10).into_iter()),
                Ok(())
            );
        }

        for i in 0..get_assets_names().len() {
            assert_eq!(oracle.calculate_value(i, CALCULATE_BEGIN), Ok(10));
        }
    }

    macro_rules! assert_ok {
        ($x:expr) => {
            assert_eq!($x, Ok(()));
        };
    }

    #[test]
    fn push() {
        let mut oracle = create_oracle();

        oracle
            .update_sources(ACCOUNTS.to_vec().into_iter())
            .expect("Update accounts error.");

        assert_ok!(oracle.push_values(&BOB, BEGIN + 0, vec![124, 1, 1, 1, 1, 5476346].into_iter()));
        assert_ok!(oracle.push_values(&DAN, BEGIN + 1, vec![128, 1, 1, 1, 1, 5476387].into_iter()));
        assert_ok!(oracle.push_values(&EVE, BEGIN + 2, vec![126, 1, 1, 1, 1, 5476394].into_iter()));

        assert_eq!(
            oracle.calculate_value(0, CALCULATE_BEGIN),
            Err(OE::FewPushedValue(4, 3))
        );
        assert_eq!(oracle.pull_value(0), Err(OE::UncalculatedValue(0)));

        assert_ok!(oracle.push_values(
            &ALICE,
            BEGIN + 3,
            vec![123, 1, 1, 1, 1, 5476378].into_iter()
        ));

        assert_eq!(oracle.calculate_value(0, CALCULATE_BEGIN), Ok(125));
        assert_eq!(oracle.calculate_value(5, CALCULATE_BEGIN), Ok(5476382));
    }
}
