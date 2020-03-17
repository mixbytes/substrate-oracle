use crate::Module;
use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

impl_outer_origin! {
    pub enum Origin for Test {}
}

// For testing the pallet, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of pallets we want to use.
#[derive(Clone, Eq, PartialEq)]
pub struct Test;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
        pub const MinimumPeriod: u64 = 1;
}

impl system::Trait for Test
{
    type Origin = Origin;
    type Call = ();
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type ModuleToIndex = ();
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
}

impl assets::Trait for Test
{
    type Event = ();
    type Balance = u128;
    type AssetId = u32;
}

impl timestamp::Trait for Test
{
    type Moment = u128;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
}

impl tablescore::Trait for Test
{
    type Event = ();
    type TableId = u32;
    type PeriodType = u32;
    type TargetType = <Self as system::Trait>::AccountId;
}

impl crate::Trait for Test
{
    type Event = ();
    type OracleId = u32;
    type ValueType = u128;
}

pub type OracleModule = Module<Test>;
pub type TablescoreModule = tablescore::Module<Test>;
pub type TimestampModule = timestamp::Module<Test>;

pub type AccountId = <Test as system::Trait>::AccountId;
pub type TableId = <Test as tablescore::Trait>::TableId;

pub const ALICE: AccountId = 123;
pub const BOB: AccountId = 225;
pub const CAROL: AccountId = 326;
pub const CHUNK: AccountId = 436;
pub const IVAN: AccountId = 528;
pub const EVE: AccountId = 931;
pub const FRANK: AccountId = 878;
pub const JUDY: AccountId = 839;
pub const OSCAR: AccountId = 754;
pub const ERIN: AccountId = 635;

pub type Balance = <Test as assets::Trait>::Balance;
pub const ASSET_ID: <Test as assets::Trait>::AssetId = 0;
pub const TOTAL_BALANCE: Balance = 10000;

pub const ORACLE_NAME: &str = "test";

pub const BTC_USD: &str = "BTC/USD";
pub const AUD_USD: &str = "AUD/USD";
pub const EUR_USD: &str = "EUR/USD";
pub const GBP_USD: &str = "GBP/USD";
pub const USD_CAD: &str = "USD/CAD";
pub const USD_CHF: &str = "USD/CHF";
pub const USD_JPY: &str = "USD/JPY";

pub const EXCHANGES: [&str; 7] = [
    BTC_USD, AUD_USD, EUR_USD, GBP_USD, USD_CAD, USD_CHF, USD_JPY,
];

pub const BTC_USD_DATA: [Balance; 4] = [878779, 886967, 886967, 886967];
pub const AUD_USD_DATA: [Balance; 4] = [878779, 886967, 886967, 886967];
pub const EUR_USD_DATA: [Balance; 4] = [878779, 886967, 886967, 886967];
pub const GBP_USD_DATA: [Balance; 4] = [878779, 886967, 886967, 886967];
pub const USD_CAD_DATA: [Balance; 4] = [878779, 886967, 886967, 886967];
pub const USD_CHF_DATA: [Balance; 4] = [878779, 886967, 886967, 886967];
pub const USD_JPY_DATA: [Balance; 4] = [878779, 886967, 886967, 886967];

pub const EXTERNAL_DATA: [[Balance; 4]; 7] = [
    BTC_USD_DATA,
    AUD_USD_DATA,
    EUR_USD_DATA,
    GBP_USD_DATA,
    USD_CAD_DATA,
    USD_CHF_DATA,
    USD_JPY_DATA,
];

pub const AGGREGATION_PERIOD: <Test as timestamp::Trait>::Moment = 60 * 9;
pub const CALCULATION_PERIOD: <Test as timestamp::Trait>::Moment = 60 * 10;

pub fn to_raw(input: &'static str) -> Vec<u8>
{
    input.to_owned().as_bytes().to_vec()
}

pub fn get_asset_names() -> Vec<Vec<u8>>
{
    EXCHANGES.iter().map(|pair| to_raw(pair)).collect()
}

pub fn get_asset_value(moment: usize, offset: Balance) -> Vec<Balance>
{
    EXTERNAL_DATA
        .iter()
        .map(|data| data[moment])
        .map(|asset_value| asset_value + offset)
        .collect()
}

pub fn get_median_value(moment: usize, asset_id: usize, offsets: Vec<Balance>) -> Balance
{
    let data: Balance = EXTERNAL_DATA
        .iter()
        .map(|data| data[moment])
        .nth(asset_id)
        .unwrap();

    let mut offsets: Vec<Balance> = offsets.into_iter().map(|offset| offset + data).collect();
    offsets.sort();

    let middle = offsets.len() / 2;
    match offsets.len()
    {
        0 | 1 => 0,
        len if len % 2 == 0 => (offsets[middle - 1] + offsets[middle]) / 2,
        _len => offsets[middle],
    }
}

pub fn get_median_values(moment: usize, offsets: Vec<Balance>) -> Vec<Balance>
{
    (1..EXTERNAL_DATA.len())
        .map(|asset_id| get_median_value(moment, asset_id, offsets.clone()))
        .collect()
}

pub fn new_test_ext() -> sp_io::TestExternalities
{
    let mut t = system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    assets::GenesisConfig::<Test> {
        assets: vec![ASSET_ID],
        initial_balance: TOTAL_BALANCE,
        endowed_accounts: vec![
            ALICE, BOB, CAROL, CHUNK, IVAN, EVE, FRANK, JUDY, OSCAR, ERIN,
        ],
        next_asset_id: 0,
        spending_asset_id: ASSET_ID,
        staking_asset_id: ASSET_ID,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}
