# ICS101 Integration scripts

# Compiling contracts

Use this command to compile your contracts: 
`wasmkit compile`

# Run script

`wasmkit run scripts/sample-script.js`

# Deploying contracts

In `scripts` folder:

First of all you need to create an instance of your contract using contract name.
```js
const contract = new Contract('sample-project', runtimeEnv);

// To deploy your contract
const deploy_response = await contract.deploy(account);

// To initialize your contract
await contract.instantiate({"count": 102}, "deploy test", account);
```

Note: You can check out your contract information in `deploy_response`.

# Interact with contracts

`wasmkit` will load functions using schema, you can call contract functions using `contract.tx`(to execute transactions) and `contract.query`(to query from contract)
```js
// To interact with your contract
// Execute contract function
await contract.tx.increment(account);

// View count in contract
await contract.query.get_count();
```
