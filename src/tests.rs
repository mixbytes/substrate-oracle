// Tests to be written here

use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};

#[test]
fn create_oracle()
{
    new_test_ext().execute_with(|| {
        assert_ok!(OracleModule::create_oracle(
            Origin::signed(ALICE),
            to_raw(ORACLE_NAME),
            3,
            CALCULATION_PERIOD,
            AGGREGATION_PERIOD,
            ASSET_ID,
            get_asset_names()
        ));
    });
}

#[test]
fn update_accounts()
{
    new_test_ext().execute_with(|| {
        let oracle_id = OracleModule::next_oracle_id();
        let table_id = TablescoreModule::next_table_id();

        assert_ok!(OracleModule::create_oracle(
            Origin::signed(ALICE),
            to_raw(ORACLE_NAME),
            3,
            CALCULATION_PERIOD,
            AGGREGATION_PERIOD,
            ASSET_ID,
            get_asset_names()
        ));

        let self_vote = |account, balance| {
            assert_ok!(TablescoreModule::vote(
                Origin::signed(account),
                table_id,
                balance,
                account
            ));
        };

        self_vote(ALICE, 96);
        self_vote(OSCAR, 97);
        self_vote(JUDY, 98);
        self_vote(CAROL, 99);
        self_vote(BOB, 100);
        self_vote(EVE, 101);

        TimestampModule::set_timestamp(100);

        let push = |account, moment, offset| {
            OracleModule::push(
                Origin::signed(account),
                oracle_id,
                get_asset_value(moment, offset),
            )
        };

        assert_ok!(push(EVE, 0, 0));
        assert_ok!(push(BOB, 0, 10));
        assert_ok!(push(CAROL, 0, 20));

        assert_noop!(push(JUDY, 0, 20), Error::<Test>::AccountPermissionDenied);
        assert_noop!(push(OSCAR, 0, 20), Error::<Test>::AccountPermissionDenied);
        assert_noop!(push(ALICE, 0, 20), Error::<Test>::AccountPermissionDenied);
        assert_noop!(push(ERIN, 0, 20), Error::<Test>::AccountPermissionDenied);
    });
}
