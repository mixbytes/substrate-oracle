// Tests to be written here

use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};

const ALICE: <Test as system::Trait>::AccountId = 123;

#[test]
fn create_oracle()
{
    new_test_ext().execute_with(|| {
        //MockModule::create_oracle(Origin::Signed(ALICE), Some("t".to_owned().as_bytes().to_vec()), 3, 100, 20, 1, vec![]);
    });
}

#[test]
fn correct_error_for_none_value()
{
    new_test_ext().execute_with(|| {
    });
}
