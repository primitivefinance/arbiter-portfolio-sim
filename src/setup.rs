use crate::bindings::{
    actor, entrypoint::Entrypoint, exchange, external_normal_strategy_lib,
    i_portfolio_actions::CreatePoolCall, mock_erc20, portfolio, weth::WETH,
};
use crate::common;
use arbiter_core::{
    bindings::liquid_exchange::LiquidExchange, math::float_to_wad, middleware::RevmMiddleware,
};
use std::sync::Arc;
// dynamic imports... generate with build.sh
use ethers::{
    abi::{encode_packed, token, Token, Tokenize},
    prelude::{Address, U128, U256},
    types::{Bytes, Filter, H160},
};
use revm::primitives::B160;

// use super::calls;
// use super::common;
// use crate::calls::DecodedReturns;
// use crate::config::SimConfig;

pub async fn run(
    deployer: Arc<RevmMiddleware>,
    // config: &SimConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // let _ = config; // todo: use config vars for create pool.

    // Deploy weth
    let weth = WETH::deploy(deployer.clone(), ()).unwrap().send().await?;

    // Deploy portfolio
    let portfolio = portfolio::Portfolio::deploy(
        deployer.clone(),
        (
            weth.address(),
            Address::from(B160::zero()),
            Address::from(B160::zero()),
        ),
    )
    .unwrap()
    .send()
    .await?;

    // Deploy Entrypoint
    let entrypoint = Entrypoint::deploy(deployer.clone(), (portfolio.address(), weth.address()))
        .unwrap()
        .send()
        .await?;
    let exchange_address = entrypoint.exchange().call().await?;

    // Start entrypoint deploys the exchange and makes the pool.
    let reciept = entrypoint
        .start(weth.address(), portfolio.address())
        .send()
        .await?
        .await?
        .unwrap();

    let token_0_address = entrypoint.token_0().call().await?;
    let token0 = mock_erc20::MockERC20::new(token_0_address, deployer.clone());

    let token_1_address = entrypoint.token_1().call().await?;
    let token1 = mock_erc20::MockERC20::new(token_1_address, deployer.clone());

    let actor_address = entrypoint.actor().call().await?;

    let actor = actor::Actor::new(actor_address, deployer.clone());

    // Token Approvals
    token0.approve(portfolio.address(), U256::MAX).await?;
    token1.approve(portfolio.address(), U256::MAX).await?;

    // Mint to arbaritrageur
    token0
        .mint(
            B160::from_low_u64_be(common::ARBITRAGEUR_ADDRESS_BASE).into(),
            float_to_wad(50.0),
        )
        .await?;
    token1
        .mint(
            B160::from_low_u64_be(common::ARBITRAGEUR_ADDRESS_BASE).into(),
            float_to_wad(50.0),
        )
        .await?;

    // Mint to exchange
    token0
        .mint(exchange_address, float_to_wad(88888888888888.0))
        .await?;
    token1
        .mint(exchange_address, float_to_wad(88888888888888.0))
        .await?;

    // deploy strategy library
    let library = external_normal_strategy_lib::ExternalNormalStrategyLib::deploy(deployer, ())
        .unwrap()
        .send()
        .await?;

    // exchange_address, "PriceChange"
    let event_filters = Filter::new().event("PriceChange").address(exchange_address);

    Ok(())
}

// pub async fn init_arbitrageur(
//     arbitrageur: &SimpleArbitrageur<arbiter::agent::IsActive>,
//     initial_prices: Vec<f64>,
// ) {
//     // Arbitrageur needs two prices to arb between which are initialized to the initial price in the price path.
//     let mut prices = arbitrageur.prices.lock().await;
//     prices[0] = revm::primitives::U256::from(initial_prices[0]).into();
//     prices[1] = revm::primitives::U256::from(initial_prices[0]).into();
//     drop(prices);
// }

// pub fn init_pool(
//     manager: &SimulationManager,
//     config: &SimConfig,
// ) -> Result<u64, Box<dyn std::error::Error>> {
//     let admin = manager.agents.get("admin").unwrap();
//     let portfolio = manager.deployed_contracts.get("portfolio").unwrap();

//     let create_pool_args: CreatePoolCall = build_create_pool_call(manager, config)?;
//     let result = admin
//         .call(
//             portfolio,
//             "createPool",
//             (
//                 create_pool_args.pair_id,
//                 create_pool_args.reserve_x_per_wad,
//                 create_pool_args.reserve_y_per_wad,
//                 create_pool_args.fee_basis_points,
//                 create_pool_args.priority_fee_basis_points,
//                 create_pool_args.controller,
//                 create_pool_args.strategy,
//                 create_pool_args.strategy_args,
//             )
//                 .into_tokens(),
//         )
//         .unwrap();

//     if !result.is_success() {
//         panic!("createPool failed");
//     }

//     let pool_id: u64 = portfolio
//         .decode_output("createPool", unpack_execution(result).unwrap())
//         .unwrap();

//     Ok(pool_id)
// }

// fn build_create_pool_call(
//     manager: &SimulationManager,
//     config: &SimConfig,
// ) -> Result<CreatePoolCall, anyhow::Error> {
//     let admin = manager.agents.get("admin").unwrap();
//     let actor = manager.deployed_contracts.get("actor").unwrap();
//     let portfolio = manager.deployed_contracts.get("portfolio").unwrap();

//     let mut exec = calls::Caller::new(admin);

//     let config_copy = config.clone();
//     let args = (
//         recast_address(portfolio.address),
//         float_to_wad(config_copy.economic.pool_strike_price_f), // strike price wad
//         (config_copy.economic.pool_volatility_f * common::BASIS_POINT_DIVISOR as f64) as u32, // vol bps
//         (config_copy.economic.pool_time_remaining_years_f * common::SECONDS_PER_YEAR as f64) as u32, // 1 year duration in seconds
//         config_copy.economic.pool_is_perpetual, // is perpetual
//         float_to_wad(config_copy.process.initial_price), // initial price wad
//     )
//         .into_tokens();
//     let create_args: bindings::actor::GetCreatePoolComputedArgsReturn = exec
//         .call(actor, "getCreatePoolComputedArgs", args)?
//         .decoded(actor)?;

//     Ok(CreatePoolCall {
//         pair_id: 1_u32, // pairId todo: fix this if running multiple pairs?
//         reserve_x_per_wad: create_args.initial_x, // reserveXPerWad
//         reserve_y_per_wad: create_args.initial_y, // reserveYPerWad
//         fee_basis_points: config_copy.economic.pool_fee_basis_points, // feeBips
//         priority_fee_basis_points: config_copy.economic.pool_priority_fee_basis_points, // priorityFeeBips
//         controller: H160::zero(),                 // controller,
//         strategy: H160::zero(),                   // address(0) == default strategy
//         strategy_args: create_args.strategy_data, // strategyArgs
//     })
// }

// pub fn allocate_liquidity(manager: &SimulationManager, pool_id: u64) -> Result<(), anyhow::Error> {
//     let admin = manager.agents.get("admin").unwrap();
//     let portfolio = manager.deployed_contracts.get("portfolio").unwrap();

//     let recipient = recast_address(admin.address());
//     let mut exec = calls::Caller::new(admin);

//     // note: this can fail automatically if block.timestamp is 0.
//     // note: this can fail if maxDeltaAsset/maxDeltaQuote is larger than uint128
//     exec.call(
//         portfolio,
//         "allocate",
//         (
//             false, // use max
//             recipient,
//             pool_id,                   // poolId
//             float_to_wad(1.0),         // 100e18 liquidity
//             U128::MAX / U128::from(2), // tries scaling to wad by multiplying beyond word size, div to avoid.
//             U128::MAX / U128::from(2),
//         )
//             .into_tokens(),
//     )?
//     .res()?;

//     Ok(())
// }
