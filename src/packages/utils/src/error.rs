use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyRatioError, ConversionOverflowError,
    Decimal256RangeExceeded, DecimalRangeExceeded, DivideByZeroError, OverflowError, StdError,
    Uint128,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Not found: {item}")]
    NotFound { item: String },

    #[error("Access control error: {reason}")]
    AccessControl { reason: String },

    // ========== VALIDATION ERRORS ==========
    #[error("Invalid address: {addr}")]
    InvalidAddress { addr: String },

    #[error("Invalid amount: {amount}")]
    InvalidAmount { amount: Uint128 },

    #[error("Insufficient fee: {fee}. Required: {required}{denom}")]
    InsufficientFee {
        fee: Uint128,
        required: Uint128,
        denom: String,
    },

    #[error("Invalid percentage: {value}. Must be between 0 and 1")]
    InvalidPercentage { value: String },

    #[error("Invalid market token: {token}")]
    InvalidMarketToken { token: String },

    #[error("Market token already exists: {token}")]
    MarketTokenExists { token: String },

    // ========== MARKET ERRORS ==========
    #[error("Invalid market id: {market_id}")]
    InvalidMarketId { market_id: String },

    #[error("Market already exists: {market_id}")]
    MarketAlreadyExists { market_id: String },

    #[error("Market not found: {market_id}")]
    MarketNotFound { market_id: String },

    #[error("Market is paused: {market_id}")]
    MarketPaused { market_id: String },

    #[error("Market is not ready: {market_id}")]
    MarketNotReady { market_id: String },

    #[error("Market is terminated: {market_id}")]
    MarketTerminated { market_id: String },

    #[error("Market capacity exceeded. Current: {current}, Limit: {limit}")]
    MarketCapacityExceeded { current: Uint128, limit: Uint128 },

    #[error("Cannot exit market {market_id}: user has outstanding positions")]
    CannotExitMarketWithPositions { market_id: String },

    #[error("User has reached maximum markets limit: {limit}")]
    MaxMarketsReached { limit: u32 },

    // ========== COLLATERAL & LENDING ERRORS ==========
    #[error("Insufficient collateral. Required: {required}, Available: {available}")]
    InsufficientCollateral {
        required: Uint128,
        available: Uint128,
    },

    #[error("Insufficient liquidity. Requested: {requested}, Available: {available}")]
    InsufficientLiquidity {
        requested: Uint128,
        available: Uint128,
    },

    #[error("Health factor too low: {health_factor}. Minimum required: {min_health_factor}")]
    HealthFactorTooLow {
        health_factor: String,
        min_health_factor: String,
    },

    #[error("Cannot borrow: credit line exceeded")]
    CreditLineExceeded {},

    #[error("Position not found for user {user} in market {market_id}")]
    PositionNotFound { user: String, market_id: String },

    #[error("Auto-claim rewards failed: {error}")]
    AutoClaimError { error: String },

    // ========== STAKING ERRORS ==========
    #[error("Market {market_id} does not support collateral staking")]
    StakingNotSupported { market_id: String },

    #[error("Validator not in whitelist: {validator}")]
    ValidatorNotWhitelisted { validator: String },

    #[error("Insufficient staking buffer. Available: {available}, Required: {required}")]
    InsufficientStakingBuffer {
        available: Uint128,
        required: Uint128,
    },

    #[error("Staking delegation failed: {reason}")]
    StakingDelegationFailed { reason: String },

    #[error("Reward harvest failed: {reason}")]
    RewardHarvestFailed { reason: String },

    #[error("Withdrawal not ready. Release time: {release_time}")]
    WithdrawalNotReady { release_time: String },

    #[error("Withdrawal not found: {withdrawal_id}")]
    WithdrawalNotFound { withdrawal_id: u64 },

    #[error("Withdrawal already claimed: {withdrawal_id}")]
    WithdrawalAlreadyClaimed { withdrawal_id: u64 },

    #[error("Cannot harvest rewards yet. Last harvest: {last_harvest}, Interval: {interval}s")]
    HarvestTooEarly { last_harvest: String, interval: u64 },

    #[error("No rewards to claim for user {sender}")]
    InsufficientRewardsToClaim { sender: String },

    // ========== LIQUIDATION ERRORS ==========
    #[error("User {user} is not liquidatable. Health factor: {health_factor}")]
    NotLiquidatable { user: String, health_factor: String },

    #[error("Liquidation amount too high. Max: {max_amount}, Requested: {requested}")]
    LiquidationAmountTooHigh {
        max_amount: Uint128,
        requested: Uint128,
    },

    #[error("Liquidation failed: {reason}")]
    LiquidationFailed { reason: String },

    #[error("Self-liquidation not allowed")]
    SelfLiquidation {},

    #[error("Cannot liquidate: no debt in market {market_id}")]
    NoDebtToLiquidate { market_id: String },

    #[error("Cannot liquidate: no collateral in market {market_id}")]
    NoCollateralToLiquidate { market_id: String },

    // ========== REWARD ERRORS ==========
    #[error("No rewards available for market {market_id}")]
    NoRewards { market_id: String },

    #[error("Auto-swap not enabled for market {market_id}")]
    AutoSwapNotEnabled { market_id: String },

    #[error("Reward distribution failed: {reason}")]
    RewardDistributionFailed { reason: String },

    // ========== PRICING & ORACLE ERRORS ==========
    #[error("Price oracle error: {reason}")]
    PriceOracleError { reason: String },

    #[error("Price not available for pair {from_token} -> {to_token}")]
    PriceNotAvailable {
        from_token: String,
        to_token: String,
    },

    #[error("Price too stale. Last update: {last_update}")]
    PriceStale { last_update: String },

    #[error("Price deviation too high: {deviation}%")]
    PriceDeviationTooHigh { deviation: String },

    // ========== MARKET OPERATION ERRORS ==========
    #[error("Market operation failed: {reason}")]
    MarketOperationFailed { reason: String },

    #[error("Insufficient liquidity for operation")]
    InsufficientMarketLiquidity {},

    // ========== CONFIGURATION ERRORS ==========
    #[error("Invalid configuration: {reason}")]
    InvalidConfig { reason: String },

    #[error("Governance operation required")]
    GovernanceRequired {},

    #[error("Protocol is in emergency stop mode")]
    EmergencyStop {},

    #[error("Feature not implemented: {feature}")]
    NotImplemented { feature: String },

    // ========== FUND ERRORS ==========
    #[error("Incorrect funds sent. Expected {expected}, Got {actual}")]
    IncorrectFunds { expected: String, actual: String },

    #[error("No funds sent")]
    NoFunds {},

    #[error("Wrong denom sent. Expected {expected}, Got {actual}")]
    WrongDenom { expected: String, actual: String },

    #[error("Denom not found: {denom}")]
    DenomNotFound { denom: String },

    // ========== USER ERRORS ==========
    #[error("User not found: {user}")]
    UserNotFound { user: String },

    #[error("User {user} not in market {market_id}")]
    UserNotInMarket { user: String, market_id: String },

    #[error("User {user} already in market {market_id}")]
    UserAlreadyInMarket { user: String, market_id: String },

    // ========== TOKEN OPERATION ERRORS ==========
    #[error("Insufficient balance: has {has}, needs {needs}")]
    InsufficientBalance { has: Uint128, needs: Uint128 },

    #[error("Insufficient allowance: has {has}, needs {needs}")]
    InsufficientAllowance { has: Uint128, needs: Uint128 },

    #[error("Cannot transfer to self")]
    SelfTransfer {},

    #[error("Cannot mint zero tokens")]
    ZeroMint {},

    #[error("Cannot burn zero tokens")]
    ZeroBurn {},

    #[error("Cannot burn more than balance: balance {balance}, burn {burn}")]
    BurnExceedsBalance { balance: Uint128, burn: Uint128 },

    #[error("Not a minter: {address}")]
    NotMinter { address: String },

    #[error("Not the market contract")]
    NotMarketContract {},

    #[error("Not the engine contract")]
    NotEngineContract {},

    // ========== CW2222 DISTRIBUTION ERRORS ==========
    #[error("No funds to withdraw")]
    NoFundsToWithdraw {},

    #[error("Insufficient withdrawable funds: available {available}, requested {requested}")]
    InsufficientWithdrawableFunds {
        available: Uint128,
        requested: Uint128,
    },

    #[error("No funds to distribute")]
    NoFundsToDistribute {},

    #[error("Distribution already pending")]
    DistributionPending {},

    #[error("Invalid distribution source")]
    InvalidDistributionSource {},

    #[error("Funds correction overflow")]
    FundsCorrectionOverflow {},

    // ========== LENDER TOKEN ERRORS ==========
    #[error("Invalid fee share: {value}, must be between 0 and 1")]
    InvalidFeeShare { value: String },

    #[error("Invalid yield share: {value}, must be between 0 and 1")]
    InvalidYieldShare { value: String },

    #[error("No protocol revenue to claim")]
    NoProtocolRevenue {},

    #[error("Foreign rewards not found: {denom}")]
    ForeignRewardsNotFound { denom: String },

    #[error("Swap failed: minimum output not met")]
    SwapSlippageExceeded {},

    #[error("Not the protocol treasury")]
    NotProtocolTreasury {},

    // ========== BORROWER TOKEN ERRORS ==========
    #[error("Total debt update unauthorized")]
    DebtUpdateUnauthorized {},

    #[error("Governance participation not enabled")]
    GovernanceNotEnabled {},

    #[error("Already participating in governance")]
    AlreadyParticipating {},

    #[error("Not participating in governance")]
    NotParticipating {},

    // ========== ADDITIONAL CONFIGURATION ERRORS ==========
    #[error("Invalid decimals: {decimals}, must be <= 18")]
    InvalidDecimals { decimals: u8 },

    #[error("Token already initialized")]
    AlreadyInitialized {},

    #[error("Migration error: {msg}")]
    MigrationError { msg: String },

    #[error("Invalid contract version for migration")]
    InvalidVersion {},

    #[error("Not an operator")]
    NotOperator {},

    #[error("Lending cap exceeded. Cap: {cap}, Would be: {would_be}")]
    LendingCapExceeded { cap: Uint128, would_be: Uint128 },

    #[error("Nothing to withdraw")]
    NothingToWithdraw {},

    #[error("Nothing to repay")]
    NothingToRepay {},

    #[error("Cannot withdraw, would make position unhealthy")]
    WithdrawalWouldUnhealthPosition {},

    #[error("Position is healthy, cannot liquidate")]
    HealthyPosition {
        user: String,
        health_factor: String,
        liquidation_threshold: String,
        market_id: String,
    },

    #[error("Position would be unhealthy. Health factor: {health_factor}")]
    UnhealthyPosition {
        health_factor: String,
        collateral_ratio: String,
    },

    #[error("Oracle price not found or stale")]
    InvalidOraclePrice {},

    #[error("Staking not configured for this market")]
    StakingNotConfigured {},

    #[error("Token operation failed: {reason}")]
    TokenOperationFailed { reason: String },

    // ========== ARITHMETIC ERRORS ==========
    #[error("Overflow error: {0}")]
    Overflow(#[from] OverflowError),

    #[error("Division by zero")]
    DivisionByZero(#[from] DivideByZeroError),

    #[error("Conversion overflow: {0}")]
    ConversionOverflow(#[from] ConversionOverflowError),

    #[error("Decimal out of range")]
    DecimalOutOfRange(#[from] DecimalRangeExceeded),

    #[error("Checked multiply ratio error")]
    CheckedMultiplyRatio(#[from] CheckedMultiplyRatioError),

    #[error("Calculation failed: {reason}")]
    CalculationFailed { reason: String },

    #[error("Decimal256 range exceeded")]
    Decimal256RangeExceeded(#[from] Decimal256RangeExceeded),

    #[error("Decimal256 conversion error")]
    CheckedFromRatio(#[from] CheckedFromRatioError),

    // ========== STORAGE ERRORS ==========
    #[error("Storage error: {reason}")]
    StorageError { reason: String },
}
