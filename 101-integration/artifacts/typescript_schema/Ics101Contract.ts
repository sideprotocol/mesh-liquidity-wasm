import { Contract, wasmKitTypes } from "@arufa/wasmkit";
export type ExecuteMsg = {
  MakePool: MsgMakePoolRequest;
} | {
  TakePool: MsgTakePoolRequest;
} | {
  SingleAssetDeposit: MsgSingleAssetDepositRequest;
} | {
  MakeMultiAssetDeposit: MsgMakeMultiAssetDepositRequest;
} | {
  TakeMultiAssetDeposit: MsgTakeMultiAssetDepositRequest;
} | {
  MultiAssetWithdraw: MsgMultiAssetWithdrawRequest;
} | {
  Swap: MsgSwapRequest;
};
export type Uint128 = string;
export type PoolSide = "SOURCE" | "DESTINATION";
export type SwapMsgType = "LEFT" | "RIGHT";
export interface MsgMakePoolRequest {
  counterpartyChannel: string;
  counterpartyCreator: string;
  creator: string;
  liquidity: PoolAsset[];
  sourceChainId: string;
  sourceChannel: string;
  sourcePort: string;
  swapFee: number;
  timeoutHeight: number;
  timeoutTimestamp: number;
  [k: string]: unknown;
}
export interface PoolAsset {
  balance: Coin;
  decimal: number;
  side: PoolSide;
  weight: number;
  [k: string]: unknown;
}
export interface Coin {
  amount: Uint128;
  denom: string;
  [k: string]: unknown;
}
export interface MsgTakePoolRequest {
  counterCreator: string;
  creator: string;
  poolId: string;
  timeoutHeight: number;
  timeoutTimestamp: number;
  [k: string]: unknown;
}
export interface MsgSingleAssetDepositRequest {
  poolId: string;
  sender: string;
  timeoutHeight: number;
  timeoutTimestamp: number;
  token: Coin;
  [k: string]: unknown;
}
export interface MsgMakeMultiAssetDepositRequest {
  deposits: DepositAsset[];
  poolId: string;
  timeoutHeight: number;
  timeoutTimestamp: number;
  [k: string]: unknown;
}
export interface DepositAsset {
  balance: Coin;
  sender: string;
  [k: string]: unknown;
}
export interface MsgTakeMultiAssetDepositRequest {
  orderId: number;
  poolId: string;
  sender: string;
  timeoutHeight: number;
  timeoutTimestamp: number;
  [k: string]: unknown;
}
export interface MsgMultiAssetWithdrawRequest {
  counterpartyReceiver: string;
  poolId: string;
  poolToken: Coin;
  receiver: string;
  timeoutHeight: number;
  timeoutTimestamp: number;
  [k: string]: unknown;
}
export interface MsgSwapRequest {
  poolId: string;
  recipient: string;
  sender: string;
  slippage: number;
  swapType: SwapMsgType;
  timeoutHeight: number;
  timeoutTimestamp: number;
  tokenIn: Coin;
  tokenOut: Coin;
  [k: string]: unknown;
}
export interface InstantiateMsg {
  token_code_id: number;
  [k: string]: unknown;
}
export type QueryMsg = {
  OrderList: {
    limit?: number | null;
    start_after?: string | null;
    [k: string]: unknown;
  };
} | {
  Order: {
    order_id: string;
    pool_id: string;
    [k: string]: unknown;
  };
} | {
  Config: {
    [k: string]: unknown;
  };
} | {
  PoolTokenList: {
    limit?: number | null;
    start_after?: string | null;
    [k: string]: unknown;
  };
} | {
  PoolAddressByToken: {
    tokens: Coin[];
    [k: string]: unknown;
  };
} | {
  InterchainPool: {
    tokens: Coin[];
    [k: string]: unknown;
  };
} | {
  InterchainPoolList: {
    limit?: number | null;
    start_after?: string | null;
    [k: string]: unknown;
  };
};
export interface Ics101ReadOnlyInterface {
  orderList: ({
    limit,
    startAfter
  }: {
    limit: number | null;
    startAfter: string | null;
  }) => Promise<any>;
  order: ({
    orderId,
    poolId
  }: {
    orderId: string;
    poolId: string;
  }) => Promise<any>;
  config: () => Promise<any>;
  poolTokenList: ({
    limit,
    startAfter
  }: {
    limit: number | null;
    startAfter: string | null;
  }) => Promise<any>;
  poolAddressByToken: ({
    tokens
  }: {
    tokens: Coin[];
  }) => Promise<any>;
  interchainPool: ({
    tokens
  }: {
    tokens: Coin[];
  }) => Promise<any>;
  interchainPoolList: ({
    limit,
    startAfter
  }: {
    limit: number | null;
    startAfter: string | null;
  }) => Promise<any>;
}
export class Ics101QueryContract extends Contract implements Ics101ReadOnlyInterface {
  constructor(contractName: string, instantiateTag?: string) {
    super(contractName, instantiateTag);
    this.orderList = this.orderList.bind(this);
    this.order = this.order.bind(this);
    this.config = this.config.bind(this);
    this.poolTokenList = this.poolTokenList.bind(this);
    this.poolAddressByToken = this.poolAddressByToken.bind(this);
    this.interchainPool = this.interchainPool.bind(this);
    this.interchainPoolList = this.interchainPoolList.bind(this);
  }

  orderList = async ({
    limit,
    startAfter
  }: {
    limit: number | null;
    startAfter: string | null;
  }): Promise<any> => {
    return this.queryMsg({
      OrderList: {
        limit,
        start_after: startAfter
      }
    });
  };
  order = async ({
    orderId,
    poolId
  }: {
    orderId: string;
    poolId: string;
  }): Promise<any> => {
    return this.queryMsg({
      Order: {
        order_id: orderId,
        pool_id: poolId
      }
    });
  };
  config = async (): Promise<any> => {
    return this.queryMsg({
      Config: {}
    });
  };
  poolTokenList = async ({
    limit,
    startAfter
  }: {
    limit: number | null;
    startAfter: string | null;
  }): Promise<any> => {
    return this.queryMsg({
      PoolTokenList: {
        limit,
        start_after: startAfter
      }
    });
  };
  poolAddressByToken = async ({
    tokens
  }: {
    tokens: Coin[];
  }): Promise<any> => {
    return this.queryMsg({
      PoolAddressByToken: {
        tokens
      }
    });
  };
  interchainPool = async ({
    tokens
  }: {
    tokens: Coin[];
  }): Promise<any> => {
    return this.queryMsg({
      InterchainPool: {
        tokens
      }
    });
  };
  interchainPoolList = async ({
    limit,
    startAfter
  }: {
    limit: number | null;
    startAfter: string | null;
  }): Promise<any> => {
    return this.queryMsg({
      InterchainPoolList: {
        limit,
        start_after: startAfter
      }
    });
  };
}
export interface Ics101Interface extends Ics101ReadOnlyInterface {
  makePool: ({
    account,
    customFees,
    memo,
    transferAmount
  }: {
    account: wasmKitTypes.UserAccount;
    customFees?: wasmKitTypes.TxnStdFee;
    memo?: string;
    transferAmount?: readonly Coin[];
  }) => Promise<any>;
  takePool: ({
    account,
    customFees,
    memo,
    transferAmount
  }: {
    account: wasmKitTypes.UserAccount;
    customFees?: wasmKitTypes.TxnStdFee;
    memo?: string;
    transferAmount?: readonly Coin[];
  }) => Promise<any>;
  singleAssetDeposit: ({
    account,
    customFees,
    memo,
    transferAmount
  }: {
    account: wasmKitTypes.UserAccount;
    customFees?: wasmKitTypes.TxnStdFee;
    memo?: string;
    transferAmount?: readonly Coin[];
  }) => Promise<any>;
  makeMultiAssetDeposit: ({
    account,
    customFees,
    memo,
    transferAmount
  }: {
    account: wasmKitTypes.UserAccount;
    customFees?: wasmKitTypes.TxnStdFee;
    memo?: string;
    transferAmount?: readonly Coin[];
  }) => Promise<any>;
  takeMultiAssetDeposit: ({
    account,
    customFees,
    memo,
    transferAmount
  }: {
    account: wasmKitTypes.UserAccount;
    customFees?: wasmKitTypes.TxnStdFee;
    memo?: string;
    transferAmount?: readonly Coin[];
  }) => Promise<any>;
  multiAssetWithdraw: ({
    account,
    customFees,
    memo,
    transferAmount
  }: {
    account: wasmKitTypes.UserAccount;
    customFees?: wasmKitTypes.TxnStdFee;
    memo?: string;
    transferAmount?: readonly Coin[];
  }) => Promise<any>;
  swap: ({
    account,
    customFees,
    memo,
    transferAmount
  }: {
    account: wasmKitTypes.UserAccount;
    customFees?: wasmKitTypes.TxnStdFee;
    memo?: string;
    transferAmount?: readonly Coin[];
  }) => Promise<any>;
}
export class Ics101Contract extends Ics101QueryContract implements Ics101Interface {
  constructor(instantiateTag?: string) {
    super("ics101", instantiateTag);
    this.makePool = this.makePool.bind(this);
    this.takePool = this.takePool.bind(this);
    this.singleAssetDeposit = this.singleAssetDeposit.bind(this);
    this.makeMultiAssetDeposit = this.makeMultiAssetDeposit.bind(this);
    this.takeMultiAssetDeposit = this.takeMultiAssetDeposit.bind(this);
    this.multiAssetWithdraw = this.multiAssetWithdraw.bind(this);
    this.swap = this.swap.bind(this);
  }

  makePool = async ({
    account,
    customFees,
    memo,
    transferAmount
  }: {
    account: wasmKitTypes.UserAccount;
    customFees?: wasmKitTypes.TxnStdFee;
    memo?: string;
    transferAmount?: readonly Coin[];
  }): Promise<any> => {
    return await this.executeMsg({
      MakePool: {}
    }, account, customFees, memo, transferAmount);
  };
  takePool = async ({
    account,
    customFees,
    memo,
    transferAmount
  }: {
    account: wasmKitTypes.UserAccount;
    customFees?: wasmKitTypes.TxnStdFee;
    memo?: string;
    transferAmount?: readonly Coin[];
  }): Promise<any> => {
    return await this.executeMsg({
      TakePool: {}
    }, account, customFees, memo, transferAmount);
  };
  singleAssetDeposit = async ({
    account,
    customFees,
    memo,
    transferAmount
  }: {
    account: wasmKitTypes.UserAccount;
    customFees?: wasmKitTypes.TxnStdFee;
    memo?: string;
    transferAmount?: readonly Coin[];
  }): Promise<any> => {
    return await this.executeMsg({
      SingleAssetDeposit: {}
    }, account, customFees, memo, transferAmount);
  };
  makeMultiAssetDeposit = async ({
    account,
    customFees,
    memo,
    transferAmount
  }: {
    account: wasmKitTypes.UserAccount;
    customFees?: wasmKitTypes.TxnStdFee;
    memo?: string;
    transferAmount?: readonly Coin[];
  }): Promise<any> => {
    return await this.executeMsg({
      MakeMultiAssetDeposit: {}
    }, account, customFees, memo, transferAmount);
  };
  takeMultiAssetDeposit = async ({
    account,
    customFees,
    memo,
    transferAmount
  }: {
    account: wasmKitTypes.UserAccount;
    customFees?: wasmKitTypes.TxnStdFee;
    memo?: string;
    transferAmount?: readonly Coin[];
  }): Promise<any> => {
    return await this.executeMsg({
      TakeMultiAssetDeposit: {}
    }, account, customFees, memo, transferAmount);
  };
  multiAssetWithdraw = async ({
    account,
    customFees,
    memo,
    transferAmount
  }: {
    account: wasmKitTypes.UserAccount;
    customFees?: wasmKitTypes.TxnStdFee;
    memo?: string;
    transferAmount?: readonly Coin[];
  }): Promise<any> => {
    return await this.executeMsg({
      MultiAssetWithdraw: {}
    }, account, customFees, memo, transferAmount);
  };
  swap = async ({
    account,
    customFees,
    memo,
    transferAmount
  }: {
    account: wasmKitTypes.UserAccount;
    customFees?: wasmKitTypes.TxnStdFee;
    memo?: string;
    transferAmount?: readonly Coin[];
  }): Promise<any> => {
    return await this.executeMsg({
      Swap: {}
    }, account, customFees, memo, transferAmount);
  };
}