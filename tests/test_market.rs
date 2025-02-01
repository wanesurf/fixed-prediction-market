use cosmwasm_std::Empty;
use cw_multi_test::{App, Contract, ContractWrapper, Executor, IntoAddr};
use truth_markets_contract_fixed::msg::{ExecuteMsg, InstantiateMsg, MarketResponse, QueryMsg};

fn counter_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new_with_empty(
        truth_markets_contract_fixed::contract::execute,
        truth_markets_contract_fixed::contract::instantiate,
        truth_markets_contract_fixed::contract::query,
    ))
}
#[test]
fn test_get_market() {
    let mut app = App::default();
    let code_id = app.store_code(market_contract());
    let owner = "owner".into_addr();

    let contract_addr = app
        .instantiate_contract(
            code_id,
            owner,
            &MarketInitMsg::default(),
            &[],
            "market-label",
            None,
        )
        .unwrap();

    let res: CounterResponse = app
        .wrap()
        .query_wasm_smart(contract_addr, &CounterQueryMsg::Value)
        .unwrap();

    assert_eq!(res.value, 0);
}
