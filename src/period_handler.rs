use codec::{Decode, Encode};
use sp_arithmetic::traits::{BaseArithmetic, One};

#[derive(Encode, Decode, Clone, Eq, Default, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct PeriodHandler<Moment>
{
    begin: Moment,
    calculate_period: Moment,
    aggregate_period: Moment,
    last_sources_update: Option<Moment>,
}

impl<Moment: Default + PartialOrd<Moment>> PeriodHandler<Moment>
{
    pub fn new(
        now: Moment,
        calculate_period: Moment,
        aggregate_period: Moment,
    ) -> Result<PeriodHandler<Moment>, ()>
    {
        if calculate_period >= aggregate_period
        {
            Ok(PeriodHandler {
                calculate_period,
                aggregate_period,
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
    pub fn get_period(&self, now: Moment) -> Moment
    {
        (now - self.begin) / self.calculate_period
    }

    pub fn is_aggregate_time(&self, now: Moment) -> bool
    {
        let next_period = self.get_period(now) + One::one();
        let next_period_begin = self.begin + next_period * self.calculate_period;

        (next_period_begin - now) <= self.aggregate_period
    }

    pub fn is_calculate_time(&self, last_update_time: Option<Moment>, now: Moment) -> bool
    {
        match last_update_time
        {
            Some(last_changed) => self.get_period(now) > self.get_period(last_changed),
            None => true,
        }
    }

    pub fn update_source_time(&mut self, now: Moment)
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
    type PeriodHandler = super::PeriodHandler<u8>;

    #[test]
    fn get_period()
    {
        let handler = PeriodHandler::new(100, 10, 5).unwrap();

        assert_eq!(handler.get_period(100), 0);
        assert_eq!(handler.get_period(109), 0);
        assert_eq!(handler.get_period(110), 1);
        assert_eq!(handler.get_period(121), 2);
    }

    #[test]
    fn is_aggregate_time()
    {
        let handler = PeriodHandler::new(100, 10, 5).unwrap();

        (200..=204).for_each(|now| assert!(!handler.is_aggregate_time(now)));
        (205..=209).for_each(|now| assert!(handler.is_aggregate_time(now)));
    }

    #[test]
    fn is_calculate_time()
    {
        let handler = PeriodHandler::new(100, 10, 5).unwrap();

        assert!(handler.is_calculate_time(None, 100));
        assert!(handler.is_calculate_time(Some(100), 110));
        assert!(!handler.is_calculate_time(Some(100), 101));
    }

    #[test]
    fn is_sources_update_needed()
    {
        let mut handler = PeriodHandler::new(100, 10, 5).unwrap();
        handler.update_source_time(105);

        (106..=114).for_each(|now| assert!(!handler.is_sources_update_needed(now)));
        assert!(handler.is_sources_update_needed(115));
    }
}
