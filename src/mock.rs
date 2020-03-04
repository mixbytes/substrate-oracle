use crate::{Module, Trait};
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
        pub const MinimumPeriod: u64 = 5;
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
    type OnReapAccount = ();
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

pub type MockModule = Module<Test>;

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities
{
    system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap()
        .into()
}
