
const juno_testnet_accounts = [
  {
    name: 'account_0',
    address: 'juno1evpfprq0mre5n0zysj6cf74xl6psk96gus7dp5',
    mnemonic: 'omit sphere nurse rib tribe suffer web account catch brain hybrid zero act gold coral shell voyage matter nose stick crucial fog judge text'
  },
  {
    name: 'account_1',
    address: 'juno1njamu5g4n0vahggrxn4ma2s4vws5x4w3u64z8h',
    mnemonic: 'student prison fresh dwarf ecology birth govern river tissue wreck hope autumn basic trust divert dismiss buzz play pistol focus long armed flag bicycle'
  }
];

const osmosis_testnet_accounts = [
  {
    name: 'account_0',
    address: 'osmo10t3g865e53yhhzvwwr5ldg50yq7vdwwf3qsa06',
    mnemonic: 'follow panda reform session awake oval shine author fire dragon retreat steel'
  },
];


// Default list covers most of the supported network
// Networks which are not required can be removed from here
const networks = {
  juno_testnet: {
    endpoint: 'https://juno-testnet-rpc.polkachu.com',
    chainId: 'uni-6',
    accounts: juno_testnet_accounts,
    fees: {
      upload: {
        amount: [{ amount: "750000", denom: "ujunox" }],
        gas: "4000000",
      },
      init: {
        amount: [{ amount: "250000", denom: "ujunox" }],
        gas: "1000000",
      },
      exec: {
        amount: [{ amount: "250000", denom: "ujunox" }],
        gas: "1000000",
      }
    },
  },
  osmosis_testnet: {
    endpoint: 'https://rpc.osmotest5.osmosis.zone',
    chainId: 'osmo-test-5',
    accounts: osmosis_testnet_accounts,
    fees: {
      upload: {
        amount: [{ amount: "100000", denom: "uosmo" }],
        gas: "4000000",
      },
      init: {
        amount: [{ amount: "50000", denom: "uosmo" }],
        gas: "1000000",
      },
      exec: {
        amount: [{ amount: "50000", denom: "uosmo" }],
        gas: "1000000",
      }
    },
  },
};

module.exports = {
  networks: {
    default: networks.osmosis_testnet,
    testnet: networks.osmosis_testnet,
    juno_testnet: networks.juno_testnet,
  },
  mocha: {
    timeout: 60000
  },
  rust: {
    version: "1.68.2",
  },
  commands: {
    compile: "RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown",
    schema: "cargo run --example schema",
  }
};
