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
    // {"token_code_id": 846},
    // juno
    {"token_code_id": 29},
    `deploy test ${runTs}`,
    contract_owner,
    undefined,  // transferAmount
    // customFees, You add here
  );
  console.log(contract_info);

  // const res = await contract.executeMsg({"TakePool": {
  //   "creator": "juno1evpfprq0mre5n0zysj6cf74xl6psk96gus7dp5",
  //   "counterCreator": "osmo10t3g865e53yhhzvwwr5ldg50yq7vdwwf3qsa06",
  //   "poolId": "poolbdc3b881a89e2aaf231b2a3eefab8bb21161e48213d2373a13c12170b65512b1",
  //   "timeoutHeight": 100,
  //   "timeoutTimestamp": 100
  // }}, contract_owner, undefined, undefined, [{denom: "ujunox", amount: "100"}]);
  // console.log(res);

  // const res1 = await contract.executeMsg({"TakeMultiAssetDeposit": {
  //   "poolId": "poolbdc3b881a89e2aaf231b2a3eefab8bb21161e48213d2373a13c12170b65512b1",
  //   "orderId": 1,
  //   "sender": "juno1evpfprq0mre5n0zysj6cf74xl6psk96gus7dp5",
  //   "timeoutHeight": 100,
  //   "timeoutTimestamp": 100,
  // }}, contract_owner, undefined, undefined, [{denom: "ujunox", amount: "50"}]);
  // console.log(res1);

  // const res = await contract.executeMsg({"SingleAssetDeposit": {
  //   "sender": "juno1evpfprq0mre5n0zysj6cf74xl6psk96gus7dp5",
  //   "poolId": "poolbdc3b881a89e2aaf231b2a3eefab8bb21161e48213d2373a13c12170b65512b1",
  //   "token": {"denom": "ujunox", "amount": "10"},
  //   "timeoutHeight": 100,
  //   "timeoutTimestamp": 100
  // }}, contract_owner, undefined, undefined, [{denom: "ujunox", amount: "10"}]);
  // console.log(res);

  console.log(await contract.poolTokenList({limit: 10, startAfter: null}));

  console.log((await contract.interchainPoolList({limit: 10, startAfter: null})));

  console.log((await contract.interchainPoolList({limit: 10, startAfter: null})).pools[0].assets);
  console.log((await contract.interchainPoolList({limit: 10, startAfter: null})).pools[0].supply);


  console.log(await contract.orderList({limit: 10, startAfter: null}));

  // const res = await contract.executeMsg({"Swap": {
    //   "sender": "juno1evpfprq0mre5n0zysj6cf74xl6psk96gus7dp5",
    //   "swapType": "LEFT",
    //   "poolId": "poolbdc3b881a89e2aaf231b2a3eefab8bb21161e48213d2373a13c12170b65512b1",
    //   "tokenIn": {"denom": "ujunox", "amount": "10"},
    //   "tokenOut": {"denom": "uosmo", "amount": "10"},
    //   "slippage": 1000,
    //   "recipient": "osmo10t3g865e53yhhzvwwr5ldg50yq7vdwwf3qsa06",
    //   "timeoutHeight": 100,
    //   "timeoutTimestamp": 100
    // }}, contract_owner, undefined, undefined, [{denom: "ujunox", amount: "10"}]);
    // console.log(res);

//   const res = await contract.executeMsg({"MakePool": {
//     "sourcePort": "wasm.osmo1usde2wnww8qp5f4gjquyw2nukgz70y3elttfqsvxvs9ur889yn7s8nt68s",
//     "sourceChannel": "channel-612",
//     "creator": "osmo10t3g865e53yhhzvwwr5ldg50yq7vdwwf3qsa06",
//     "counterpartyCreator": "juno1evpfprq0mre5n0zysj6cf74xl6psk96gus7dp5",
//     "liquidity": [{
//         "side": "DESTINATION",
//         "balance": {"denom": "ujunox", "amount": "100"},
//         "weight": 50,
//         "decimal": 6
//       },
//       {
//         "side": "SOURCE",
//         "balance": {"denom": "uosmo", "amount": "100"},
//         "weight": 50,
//         "decimal": 6
//       }
//     ],
//     "swapFee": 10000,
//     "timeoutHeight": 100,
//     "timeoutTimestamp": 100
//   }}, contract_owner, undefined, undefined, [{denom: "uosmo", amount: "100"}]);
//   console.log(res);

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
