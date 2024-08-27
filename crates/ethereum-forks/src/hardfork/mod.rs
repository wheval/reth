mod macros;

mod ethereum;
pub use ethereum::EthereumHardfork;

mod optimism;
pub use optimism::OptimismHardfork;

mod dev;
pub use dev::DEV_HARDFORKS;

use core::{
    any::Any,
    hash::{Hash, Hasher},
};

use alloy_genesis::ChainConfig;
use dyn_clone::DynClone;

use crate::ForkCondition;

/// Generic hardfork trait.
#[auto_impl::auto_impl(&, Box)]
pub trait Hardfork: Any + DynClone + Send + Sync + 'static {
    /// Fork name.
    fn name(&self) -> &'static str;
}

dyn_clone::clone_trait_object!(Hardfork);

impl core::fmt::Debug for dyn Hardfork + 'static {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct(self.name()).finish()
    }
}

impl PartialEq for dyn Hardfork + 'static {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
    }
}

impl Eq for dyn Hardfork + 'static {}

impl Hash for dyn Hardfork + 'static {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name().hash(state)
    }
}

/// Configures hardforks from genesis [`ChainConfig`].
pub trait ConfigureHardforks: Sized {
    /// Returns super chain mainnet hardforks.
    fn super_chain_mainnet_hardforks() -> impl Iterator<Item = (Self, ForkCondition)>;

    /// Initializes block based hardforks from [`ChainConfig`].
    fn init_block_hardforks(
        config: &ChainConfig,
    ) -> impl IntoIterator<Item = (Self, ForkCondition)>;

    /// Initializes TTD based hardfork from [`ChainConfig`].
    fn init_paris(config: &ChainConfig) -> Option<(Self, ForkCondition)>;

    /// Initializes time based hardforks from [`ChainConfig`].
    fn init_time_hardforks(config: &ChainConfig)
        -> impl IntoIterator<Item = (Self, ForkCondition)>;

    /// Initializes hardforks from [`ChainConfig`].
    fn init(config: &ChainConfig) -> impl IntoIterator<Item = (Self, ForkCondition)> {
        Self::init_block_hardforks(config)
            .into_iter()
            .chain(Self::init_paris(config))
            .chain(Self::init_time_hardforks(config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardfork::optimism::OptimismHardfork;
    use std::str::FromStr;

    #[test]
    fn check_hardfork_from_str() {
        let hardfork_str = [
            "frOntier",
            "homEstead",
            "dao",
            "tAngerIne",
            "spurIousdrAgon",
            "byzAntium",
            "constantinople",
            "petersburg",
            "istanbul",
            "muirglacier",
            "bErlin",
            "lonDon",
            "arrowglacier",
            "grayglacier",
            "PARIS",
            "ShAnGhAI",
            "CaNcUn",
            "PrAguE",
        ];
        let expected_hardforks = [
            EthereumHardfork::Frontier,
            EthereumHardfork::Homestead,
            EthereumHardfork::Dao,
            EthereumHardfork::Tangerine,
            EthereumHardfork::SpuriousDragon,
            EthereumHardfork::Byzantium,
            EthereumHardfork::Constantinople,
            EthereumHardfork::Petersburg,
            EthereumHardfork::Istanbul,
            EthereumHardfork::MuirGlacier,
            EthereumHardfork::Berlin,
            EthereumHardfork::London,
            EthereumHardfork::ArrowGlacier,
            EthereumHardfork::GrayGlacier,
            EthereumHardfork::Paris,
            EthereumHardfork::Shanghai,
            EthereumHardfork::Cancun,
            EthereumHardfork::Prague,
        ];

        let hardforks: Vec<EthereumHardfork> =
            hardfork_str.iter().map(|h| EthereumHardfork::from_str(h).unwrap()).collect();

        assert_eq!(hardforks, expected_hardforks);
    }

    #[test]
    fn check_op_hardfork_from_str() {
        let hardfork_str = ["beDrOck", "rEgOlITH", "cAnYoN", "eCoToNe", "FJorD", "GRaNiTe"];
        let expected_hardforks = [
            OptimismHardfork::Bedrock,
            OptimismHardfork::Regolith,
            OptimismHardfork::Canyon,
            OptimismHardfork::Ecotone,
            OptimismHardfork::Fjord,
            OptimismHardfork::Granite,
        ];

        let hardforks: Vec<OptimismHardfork> =
            hardfork_str.iter().map(|h| OptimismHardfork::from_str(h).unwrap()).collect();

        assert_eq!(hardforks, expected_hardforks);
    }

    #[test]
    fn check_nonexistent_hardfork_from_str() {
        assert!(EthereumHardfork::from_str("not a hardfork").is_err());
    }
}
