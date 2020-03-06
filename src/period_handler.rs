use codec::{Decode, Encode};
use sp_arithmetic::traits::{BaseArithmetic, One};

/// Period Handler
/// |---------------------|---------------------|
/// |       period        |       period        |
/// |---------|-----------|---------|-----------|
/// |   agg   |   calc    |   agg   |   calc    |
/// agg - Part of timeline when we aggregate new data from sources
/// calc - Part of timeline when we calculate aggregated values
/// Calculate value we can once at calc period
#[derive(Encode, Decode, Clone, Eq, Default, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct PeriodHandler<Moment>
{
    /// Begin of period handle
    begin: Moment,

    /// One period delta
    period: Moment,

    /// Aggregate part of period
    aggregate_part: Moment,

    /// Moment when we last update sources
    last_sources_update: Option<Moment>,
}

impl<Moment: Default + PartialOrd<Moment>> PeriodHandler<Moment>
{
    pub fn new(
        now: Moment,
        period: Moment,
        aggregate_part: Moment,
    ) -> Result<PeriodHandler<Moment>, ()>
    {
        if period > aggregate_part
        {
            Ok(PeriodHandler {
                period,
                aggregate_part,
                begin: now,
                last_sources_update: None,
            })
        }
        else
        {
            Err(())
        }
    }
}

impl<Moment: BaseArithmetic + Copy> PeriodHandler<Moment>
{
    /// Get period number
    pub fn get_period(&self, now: Moment) -> Moment
    {
        (now - self.begin) / self.period
    }

    /// Get part of calculate period
    fn get_calculate_part(&self) -> Moment
    {
        self.period - self.aggregate_part
    }

    /// Get rest of time of the current period
    fn get_rest_of_period(&self, now: Moment) -> Moment
    {
        let next_period = self.get_period(now) + One::one();
        let next_period_begin = self.begin + (next_period * self.period);
        next_period_begin - now
    }

    pub fn is_aggregate_time(&self, now: Moment) -> bool
    {
        self.get_rest_of_period(now) >= self.get_calculate_part()
    }

    pub fn is_calculate_time(&self, last_update_time: Option<Moment>, now: Moment) -> bool
    {
        if self.is_aggregate_time(now)
        {
            return false;
        }

        match last_update_time
        {
            Some(last_changed) => self.get_period(now) > self.get_period(last_changed),
            None => true,
        }
    }

    pub fn set_sources_updated(&mut self, now: Moment)
    {
        self.last_sources_update = Some(now);
    }

    pub fn is_sources_update_needed(&self, now: Moment) -> bool
    {
        self.is_aggregate_time(now)
            && match self.last_sources_update
            {
                None => true,
                Some(last_sources_update) =>
                {
                    self.get_period(last_sources_update) < self.get_period(now)
                }
            }
    }
}

#[cfg(test)]
mod tests
{
    type PeriodHandler = super::PeriodHandler<u32>;

    #[test]
    fn create()
    {
        assert_eq!(PeriodHandler::new(0, 1, 10), Err(()));

        let handler = PeriodHandler::new(0, 100, 90);
        assert!(handler.is_ok());
    }

    #[test]
    fn get_period()
    {
        let handler = PeriodHandler::new(100, 100, 90).expect("Error in create period handler");

        (100..=199).for_each(|now| assert_eq!(handler.get_period(now), 0));
        (200..=299).for_each(|now| assert_eq!(handler.get_period(now), 1));
    }

    #[test]
    fn is_aggregate_time()
    {
        let handler = PeriodHandler::new(100, 100, 90).expect("Error in create period handler");

        (100..=190).for_each(|now| assert!(handler.is_aggregate_time(now)));
        (191..=199).for_each(|now| assert!(!handler.is_aggregate_time(now)));
    }

    #[test]
    fn is_calculate_time()
    {
        let handler = PeriodHandler::new(100, 100, 90).expect("Error in create period handler");

        (100..=190).for_each(|now| assert!(!handler.is_calculate_time(None, now)));
        (191..=199).for_each(|now| assert!(handler.is_calculate_time(None, now)));

        (100..=190).for_each(|now| assert!(!handler.is_calculate_time(Some(now), now)));
        (191..=199).for_each(|now| assert!(!handler.is_calculate_time(Some(190), now)));

        (291..=299).for_each(|now| assert!(handler.is_calculate_time(Some(190), now)));
    }

    #[test]
    fn is_sources_update_needed()
    {
        let mut handler = PeriodHandler::new(100, 100, 90).expect("Error in create period handler");

        (100..=190).for_each(|now| assert!(handler.is_sources_update_needed(now)));
        handler.set_sources_updated(100);
        (100..=190).for_each(|now| assert!(!handler.is_sources_update_needed(now)));
        (200..=290).for_each(|now| assert!(handler.is_sources_update_needed(now)));
    }
}
