// Tests to be written here

use crate::mock::*;
use frame_support::dispatch;
use frame_support::{assert_err, assert_ok};

type Error = crate::Error<Test>;

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

        self_votes(table_id, vec![(ALICE, 1)]);

        assert_err!(
            OracleModule::push(Origin::signed(ALICE), oracle_id, get_asset_value(0, 10)),
            Error::NotEnoughSources
        );

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

        let push = |account, moment, offset| {
            OracleModule::push(
                Origin::signed(account),
                oracle_id,
                get_asset_value(moment, offset),
            )
        };

        [EVE, BOB, CAROL].iter().for_each(|&account| {
            assert_ok!(push(account, 0, 0));
        });

        [JUDY, OSCAR, ALICE, ERIN].iter().for_each(|&account| {
            assert_err!(push(account, 0, 20), Error::AccountPermissionDenied);
        });

        self_votes(table_id, vec![(ALICE, 100), (OSCAR, 100), (JUDY, 100)]);

        TimestampModule::set_timestamp(700);

        [ALICE, OSCAR, JUDY].iter().for_each(|&account| {
            assert_ok!(push(account, 0, 0));
        });

        [EVE, BOB, CAROL, ERIN].iter().for_each(|&account| {
            assert_err!(push(account, 0, 20), Error::AccountPermissionDenied);
        });
    });
}

#[test]
fn aggregation()
{
    new_test_ext().execute_with(|| {
        let oracle_id = OracleModule::next_oracle_id();
        let table_id = TablescoreModule::next_table_id();
        assert_ok!(create_oracle(3));

        self_votes(table_id, vec![(CAROL, 99), (BOB, 100), (EVE, 101)]);

        let push = |account, moment, offset| {
            OracleModule::push(
                Origin::signed(account),
                oracle_id,
                get_asset_value(moment, offset),
            )
        };

        let accounts = [EVE, BOB, CAROL];
        accounts.iter().for_each(|&account| {
            assert_ok!(push(account, 0, 20));
        });

        for now in (AGGREGATION_PERIOD + 1)..CALCULATION_PERIOD
        {
            TimestampModule::set_timestamp(now);

            accounts.iter().for_each(|&account| {
                assert_err!(push(account, 0, 20), Error::NotAggregationTime);
            });
        }

        TimestampModule::set_timestamp(CALCULATION_PERIOD);

        accounts.iter().for_each(|&account| {
            assert_ok!(push(account, 0, 20));
        });
    });
}

#[test]
fn calculate()
{
    new_test_ext().execute_with(|| {
        let oracle_id = OracleModule::next_oracle_id();
        let table_id = TablescoreModule::next_table_id();
        assert_ok!(create_oracle(5));

        let votes = vec![
            (EVE, 101),
            (BOB, 100),
            (CAROL, 99),
            (JUDY, 98),
            (OSCAR, 97),
            (ALICE, 96),
        ];
        self_votes(table_id, votes.clone());
        let accounts: Vec<AccountId> = votes.into_iter().map(|(ac, _)| ac).take(5).collect();

        let push = |account, moment, offset| {
            OracleModule::push(
                Origin::signed(account),
                oracle_id,
                get_asset_value(moment, offset),
            )
        };

        let mut now = 0;

        for moment in 0..4
        {
            TimestampModule::set_timestamp(now);

            let offsets: Vec<u128> = accounts
                .iter()
                .enumerate()
                .map(|(index, &acc)| {
                    let offset = 10u128 * (index as u128);
                    assert_ok!(push(acc, moment, offset));
                    offset
                })
                .collect();

            now += AGGREGATION_PERIOD + 1; // Calculation period
            TimestampModule::set_timestamp(now);

            get_median_values(moment, offsets)
                .into_iter()
                .enumerate()
                .for_each(|(asset_id, val)| {
                    assert_ok!(OracleModule::calculate(
                        Origin::signed(ALICE),
                        oracle_id,
                        asset_id as u8
                    ));
                    assert_eq!(
                        OracleModule::oracles(oracle_id)
                            .values
                            .get(asset_id)
                            .and_then(|ex| ex.value),
                        Some(val)
                    );
                });

            assert_err!(
                OracleModule::calculate(
                    Origin::signed(ALICE),
                    oracle_id,
                    1u8 + EXTERNAL_DATA.len() as u8
                ),
                Error::WrongValueId
            );

            now += CALCULATION_PERIOD - 1;
            TimestampModule::set_timestamp(now);
        }
    });
}
