/**
 * 
 * dev : dev development environment
 * 
 * dev|testnet|main : {
 *  
 *    contract: Contract address;
 *    counterparties:[
 *      {
 *        channelId: counterparty channelId
 *      }
 *    ]
 * 
 * }
 */

const config = {
  sideChain: {
    dev: {
      contract:
        'side1gg6f95cymcfrfzhpek7cf5wl53t5kng52cd2m0krgdlu8k58vd8qzv89wl',
      counterparties: [
        {
          chainID: 'osmo-test-5',
          channelId: 'channel-17',
          name: 'uosmo',
          type: 'wasm',
        },
        {
          chainID: 'injective-888',
          channelId: 'channel-99',
          name: ' injective-888',
          type: 'wasm',
        },
      ],
    },
    testnet: {},
    main: {},
  },
  osmoChain: {
    testnet: {
      contract:
        'osmo1lx8xra29g27tug8jezxvv7xeevv22yc04d5kjufp5gvx9eztmyxq59x3mg',
      counterparties: [
        {
          chainID: 'injective-888',
          channelId: 'channel-1514',
          name: 'injective-888',
          type: 'wasm',
        },
        {
          chainID: 'side-devnet-1',
          channelId: 'channel-1510',
          name: 'SIDE Test',
          type: 'wasm',
        },
      ],
    },
    main: {},
  },
  injective: {
    tsetnet: {
      contract: 'inj13gttmee75m22058kcnsua3yq8uhk9lwkmyurer',
      counterparties: [
        {
          chainID: 'osmo-test-5',
          channelId: 'channel-98',
          name: 'uosmo',
          type: 'wasm',
        },
        {
          chainID: 'side-devnet-1',
          channelId: 'channel-99',
          name: 'SIDE Test',
          type: 'wasm',
        },
      ],
    },
    main: {},
  },
};
