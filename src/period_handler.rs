use codec::{Decode, Encode};
use rstd::cmp::Ordering;
use sp_arithmetic::traits::BaseArithmetic;

/// Period Handler
/// |---------------------|---------------------|
/// |       period        |       period        |
/// |---------|-----------|---------|-----------|
/// |   agg   |   calc    |   agg   |   calc    |
/// agg - Part of timeline when we aggregate new data from sources
/// calc - Part of timeline when we calculate aggregated values
/// Calculate value we can only once at calc period or at next agg period
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

#[derive(Debug, PartialEq, Eq)]
pub enum Part
{
    Aggregate,
    Calculate,
}

impl<Moment: BaseArithmetic + Copy> PeriodHandler<Moment>
{
    /// Get period number
    pub fn get_period(&self, now: Moment) -> Moment
    {
        (now - self.begin) / self.period }

    fn get_rest_of_period(&self, now: Moment) -> Moment
    {
        let next_period = self.get_period(now) + Moment::one();
        let next_period_begin = self.begin + (next_period * self.period);
        next_period_begin - now
    }

    pub fn get_part(&self, now: Moment) -> Part
    {
        if self.period - self.get_rest_of_period(now) <= self.aggregate_part
        {
            Part::Aggregate
        }
        else
        {
            Part::Calculate
        }
    }

    pub fn is_can_aggregate(&self, now: Moment) -> bool
    {
        self.get_part(now) == Part::Aggregate
    }

    /// Is calculation possible at `now` if the data last changed at `last_update_time`
    ///
    /// If we don't calculate data in the past period - we can calculate it in current aggregate
    /// part
    pub fn is_can_calculate(&self, last_update_time: Option<Moment>, now: Moment) -> bool
    {
        let current_part = self.get_part(now);
        match last_update_time
        {
            Some(last_changed) =>
            {
                let last_part = self.get_part(last_changed);

                let current_period = self.get_period(now);
                let last_period = self.get_period(last_changed);

                match current_period.cmp(&last_period)
                {
                    Ordering::Less => unreachable!(),
                    Ordering::Equal =>
                    {
                        (last_part, current_part) == (Part::Aggregate, Part::Calculate)
                    }
                    Ordering::Greater => match (last_part, current_part)
                    {
                        (_, Part::Calculate) => true,
                        (Part::Aggregate, Part::Aggregate) => true,
                        (Part::Calculate, Part::Aggregate) =>
                        {
                            (current_period - Moment::one()) != last_period
                        }
                    },
                }
            }
            None =>
            {
                if self.get_period(now) == Moment::zero()
                {
                    current_part == Part::Calculate
                }
                else
                {
                    true
                }
            }
        }
    }

    pub fn set_sources_updated(&mut self, now: Moment)
    {
        self.last_sources_update = Some(now);
    }

    pub fn is_sources_update_needed(&self, now: Moment) -> bool
    {
        self.is_can_aggregate(now)
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
    fn is_can_aggregate()
    {
        let handler = PeriodHandler::new(100, 100, 90).expect("Error in create period handler");

        (100..=190).for_each(|now| assert!(handler.is_can_aggregate(now)));
        (191..=199).for_each(|now| assert!(!handler.is_can_aggregate(now)));
    }

    #[test]
    fn is_can_calculate()
    {
        let handler = PeriodHandler::new(100, 100, 90).expect("Error in create period handler");

        (100..=190).for_each(|now| assert!(!handler.is_can_calculate(None, now), "{}", now));
        (191..=199).for_each(|now| assert!(handler.is_can_calculate(None, now), "{}", now));

        (100..=190).for_each(|now| assert!(!handler.is_can_calculate(Some(now), now), "{}", now));
        //Todo add complicated tests
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
