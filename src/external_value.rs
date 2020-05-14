use codec::{Decode, Encode};
use rstd::cmp::{Ord, Ordering};
use rstd::prelude::Vec;

/// Value or pair of value in vector
#[derive(PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum Median<T> {
    Value(T),
    Pair(T, T),
}

/// Get median from ordered values
pub fn get_median<T: Ord + Copy>(mut values: Vec<T>) -> Option<Median<T>> {
    values.sort();

    let middle = values.len() / 2;
    match values.len() {
        0 | 1 => None,
        len if len % 2 == 0 => Some(Median::Pair(values[middle - 1], values[middle])),
        _len => Some(Median::Value(values[middle])),
    }
}

/// External (for blockchain) value
#[derive(Encode, Decode, Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct ExternalValue<ValueType, Moment> {
    pub value: Option<ValueType>,

    /// Moment we last changed the value
    /// - None if value is empty
    pub last_changed: Option<Moment>,
}

impl<ValueType: Default + Eq + Ord + Clone, Moment: Default + Eq + Ord + Clone> Ord
    for ExternalValue<ValueType, Moment>
{
    fn cmp(&self, other: &Self) -> Ordering {
        match self.value.cmp(&other.value) {
            Ordering::Equal => self.last_changed.cmp(&other.last_changed),
            ord => ord,
        }
    }
}

impl<ValueType: Default + Eq + Ord + Clone, Moment: Default + Eq + Ord + Clone> PartialOrd
    for ExternalValue<ValueType, Moment>
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

impl<ValueType: Default + Eq + Ord + Clone, Moment: Default + Eq + Ord + Clone>
    ExternalValue<ValueType, Moment>
{
    pub fn new(value: ValueType, now: Moment) -> Self {
        ExternalValue {
            value: Some(value),
            last_changed: Some(now),
        }
    }

    pub fn clean(&mut self) {
        self.value = None;
        self.last_changed = None;
    }

    pub fn update(&mut self, value: ValueType, now: Moment) {
        self.value = Some(value);
        self.last_changed = Some(now);
    }

    pub fn is_clean(&self) -> bool {
        self.last_changed.is_none() && self.value.is_none()
    }

    /// From pair of option to optional pair of cloned fields
    pub fn get(&self) -> Option<(ValueType, Moment)> {
        match (&self.value, &self.last_changed) {
            (Some(value), Some(last_changed)) => Some((value.clone(), last_changed.clone())),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{get_median, Median};

    #[test]
    fn simple() {
        let array: Vec<u8> = (0..=10).collect();
        let median = array[5];
        assert_eq!(get_median(array), Some(Median::Value(median)));
    }
}
