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
          sourceChannel: 'channel-17',
        },
        {
          chainID: 'injective-888',
          sourceChannel: 'channel-99',
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
          sourceChannel: 'channel-1514',
        },
        {
          chainID: 'side-devnet-1',
          sourceChannel: 'channel-1510',
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
          sourceChannel: 'channel-98',
        },
        {
          chainID: 'side-devnet-1',
          sourceChannel: 'channel-99',
        },
      ],
    },
    main: {},
  },
};
