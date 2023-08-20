import codegen from '@cosmwasm/ts-codegen';

codegen({
  contracts: [
    {
      name: 'ics100',
      dir: './contracts/ics100/schema'
    },
    {
      name: 'ics101',
      dir: './contracts/ics101/schema'
    }
  ],
  outPath: './cosmoswasm-codegen',

  // options are completely optional ;)
  options: {
    bundle: {
      bundleFile: 'index.ts',
      scope: 'SideContracts'
    },
    types: {
      enabled: true
    },
    client: {
      enabled: true
    },
    reactQuery: {
      enabled: true,
      optionalClient: true,
      version: 'v4',
      mutations: true,
      queryKeys: true,
      queryFactory: true,
    },
    recoil: {
      enabled: false
    },
    messageComposer: {
      enabled: false
    },
    messageBuilder: {
      enabled: false
    },
    useContractsHooks: {
      enabled: true
    }
  }
}).then(() => {
  console.log('✨ all done!');
});