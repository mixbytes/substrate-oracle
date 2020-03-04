// Tests to be written here

use crate::{mock::*, Error};
use frame_support::dispatch;
use frame_support::{assert_noop, assert_ok};

fn create_oracle(source_limit: u8) -> dispatch::DispatchResult
{
    OracleModule::create_oracle(
        Origin::signed(ALICE),
        to_raw(ORACLE_NAME),
        source_limit,
        CALCULATION_PERIOD,
        AGGREGATION_PERIOD,
        ASSET_ID,
        get_asset_names(),
    )
}

fn self_votes(table_id: TableId, accounts_votes: Vec<(AccountId, Balance)>)
{
    accounts_votes.into_iter().for_each(|(account, balance)| {
        assert_ok!(TablescoreModule::vote(
            Origin::signed(account),
            table_id,
            balance,
            account
        ));
    });
}

#[test]
fn create()
{
    new_test_ext().execute_with(|| {
        assert_ok!(create_oracle(4));
    });
}

#[test]
fn update_accounts()
{
    new_test_ext().execute_with(|| {
        let oracle_id = OracleModule::next_oracle_id();
        let table_id = TablescoreModule::next_table_id();
        assert_ok!(create_oracle(3));

        TimestampModule::set_timestamp(100);

        self_votes(
            table_id,
            vec![
                (ALICE, 96),
                (OSCAR, 97),
                (JUDY, 98),
                (CAROL, 99),
                (BOB, 100),
                (EVE, 101),
            ],
        );

        let denied = || Error::<Test>::AccountPermissionDenied;
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

        assert_noop!(push(JUDY, 0, 20), denied());
        assert_noop!(push(OSCAR, 0, 20), denied());
        assert_noop!(push(ALICE, 0, 20), denied());
        assert_noop!(push(ERIN, 0, 20), denied());

        self_votes(table_id, vec![(ALICE, 100), (OSCAR, 100), (JUDY, 100)]);

        TimestampModule::set_timestamp(700);

        assert_ok!(push(ALICE, 0, 0));
        assert_ok!(push(OSCAR, 0, 10));
        assert_ok!(push(JUDY, 0, 20));

        assert_noop!(push(EVE, 0, 20), denied());
        assert_noop!(push(BOB, 0, 20), denied());
        assert_noop!(push(CAROL, 0, 20), denied());
        assert_noop!(push(ERIN, 0, 20), denied());
    });
}
