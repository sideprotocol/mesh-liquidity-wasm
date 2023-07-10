import { getAccountByName } from "@arufa/wasmkit";

import { Ics101Contract } from "../artifacts/typescript_schema/Ics101Contract";

export default async function run () {
  const runTs = String(new Date());
  const contract_owner = await getAccountByName("account_0");
  const contract = new Ics101Contract();
  await contract.setupClient();

  const deploy_response = await contract.deploy(
    contract_owner,
  );
  console.log(deploy_response);

  const contract_info = await contract.instantiate(
    // Put cw20 token id here
    // osmosis
    {"token_code_id": 846},
    // juno
    //{"token_code_id": 2940},
    `deploy test ${runTs}`,
    contract_owner,
    undefined,  // transferAmount
    // customFees, You add here
  );
  console.log(contract_info);

  // const res = await contract.executeMsg({"MakePool": {
  //   "sourcePort": "wasm.osmo1mgu8dqrmzd7mewyz4n7sxqhyadkuvqa8ed2knl07d3vtmp4x0v4q85u8c8",
  //   "sourceChannel": "channel-748",
  //   "counterpartyChannel": "channel-510",
  //   "creator": "osmo10t3g865e53yhhzvwwr5ldg50yq7vdwwf3qsa06",
  //   "counterpartyCreator": "juno1evpfprq0mre5n0zysj6cf74xl6psk96gus7dp5",
  //   "liquidity": [
  //     {
  //       "side": "SOURCE",
  //       "balance": {"denom": "uosmo", "amount": "100"},
  //       "weight": 50,
  //       "decimal": 6
  //     },
  //     {
  //       "side": "DESTINATION",
  //       "balance": {"denom": "ujunox", "amount": "100"},
  //       "weight": 50,
  //       "decimal": 6
  //     },
  //   ],
  //   "swapFee": 10000,
  //   "timeoutHeight": 100,
  //   "timeoutTimestamp": 100,
  //   "sourceChainId": "osmo-test-5"
  // }}, contract_owner, undefined, undefined, [{denom: "uosmo", amount: "100"}]);
  // console.log(res);

  // const res = await contract.executeMsg({"MultiAssetWithdraw": {
  //   "poolId": "poolbdc3b881a89e2aaf231b2a3eefab8bb21161e48213d2373a13c12170b65512b1",
  //   "receiver": "osmo10t3g865e53yhhzvwwr5ldg50yq7vdwwf3qsa06",
  //   "counterpartyReceiver": "juno1evpfprq0mre5n0zysj6cf74xl6psk96gus7dp5",
  //   "poolToken": {"denom": "poolbdc3b881a89e2aaf231b2a3eefab8bb21161e48213d2373a13c12170b65512b1", 
  //   "amount": "10000000000000000000"},
  //   "timeoutHeight": 100,
  //   "timeoutTimestamp": 100
  // }}, contract_owner, undefined, undefined, [{denom: "uosmo", amount: "100"}]);
  // console.log(res);

  // const res1 = await contract.executeMsg({"MakeMultiAssetDeposit": {
  //   "poolId": "poolbdc3b881a89e2aaf231b2a3eefab8bb21161e48213d2373a13c12170b65512b1",
  //   "deposits": [
  //     {
  //       "sender": "osmo10t3g865e53yhhzvwwr5ldg50yq7vdwwf3qsa06",
  //       "balance": {"denom": "uosmo", amount: "50"}
  //     },
  //     {
  //       "sender": "juno1evpfprq0mre5n0zysj6cf74xl6psk96gus7dp5",
  //       "balance": {"denom": "ujunox", amount: "50"}
  //     }
  //   ],
  //   "timeoutHeight": 100,
  //   "timeoutTimestamp": 100,
  // }}, contract_owner, undefined, undefined, [{denom: "uosmo", amount: "50"}]);
  // console.log(res1);

  // const res = await contract.executeMsg({"SingleAssetDeposit": {
  //   "sender": "osmo10t3g865e53yhhzvwwr5ldg50yq7vdwwf3qsa06",
  //   "poolId": "poolbdc3b881a89e2aaf231b2a3eefab8bb21161e48213d2373a13c12170b65512b1",
  //   "token": {"denom": "uosmo", "amount": "10"},
  //   "timeoutHeight": 100,
  //   "timeoutTimestamp": 100
  // }}, contract_owner, undefined, undefined, [{denom: "uosmo", amount: "10"}]);
  // console.log(res);

  console.log(await contract.poolTokenList({limit: 10, startAfter: null}));

  console.log(await contract.interchainPoolList({limit: 10, startAfter: null}));

  console.log(await contract.orderList({limit: 10, startAfter: null}));

  console.log(await contract.poolAddressByToken({tokens: [{amount: "100", denom: "ujunox"}, 
  {amount: "100", denom: "uosmo"}]}));

  console.log((await contract.interchainPoolList({limit: 10, startAfter: null})).pools[0].assets);
  console.log((await contract.interchainPoolList({limit: 10, startAfter: null})).pools[0].supply);

  //{"side": {"SOURCE": {} }, "balance": {"denom": "uosmo", "amount": "100"},"weight": 50,"decimal": 6}

  // const inc_response = await contract.increment({account: contract_owner});
  // console.log(inc_response);

  // const response = await contract.getCount();
  // console.log(response);

  // const ex_response = await contract.increment(
  //   {
  //     account: contract_owner,
  //   }
  // );
  // console.log(ex_response);
}
