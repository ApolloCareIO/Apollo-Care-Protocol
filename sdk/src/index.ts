/**
 * Apollo Care Protocol SDK
 * 
 * TypeScript SDK for interacting with Apollo Care Protocol on Solana.
 * 
 * @example
 * ```typescript
 * import { ApolloClient, createDevnetClient } from '@apollo-care/sdk';
 * 
 * const client = createDevnetClient();
 * const carStatus = await client.getCarStatus();
 * console.log(`Current CAR: ${carStatus.car}%, Zone: ${carStatus.zone}`);
 * ```
 * 
 * @packageDocumentation
 */

export { ApolloClient, createMainnetClient, createDevnetClient, createLocalClient } from './client';
export type { ApolloClientConfig } from './client';

export * from './types';
export * from './constants';
export * from './utils';

/** SDK Version */
export const VERSION = '1.0.0';

/** Program IDs - Replace with deployed addresses */
export const PROGRAM_IDS = {
  APOLLO_CORE: 'ApLCoRExxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx1',
  APOLLO_GOVERNANCE: 'ApLGoVxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx2',
  APOLLO_STAKING: 'ApLStKxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx3',
  APOLLO_RESERVES: 'ApLRsVxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx4',
  APOLLO_RISK_ENGINE: 'ApLRsKxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx5',
  APOLLO_MEMBERSHIP: 'ApLMbRxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx6',
  APOLLO_CLAIMS: 'ApLClMxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx7',
  APOLLO_REINSURANCE: 'ApLReIxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx8',
} as const;

/** APH Token Configuration */
export const APH_TOKEN = {
  MINT: '6S3T6f1mmhxQWKR1gMiZR1SZLpu396jnnRMGqAZUj3Qj',
  DECIMALS: 9,
  SYMBOL: 'APH',
  NAME: 'Apollo Care',
  PROGRAM: 'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb',
} as const;

/** Network Endpoints */
export const NETWORKS = {
  MAINNET: {
    url: 'https://api.mainnet-beta.solana.com',
    wsUrl: 'wss://api.mainnet-beta.solana.com',
  },
  DEVNET: {
    url: 'https://api.devnet.solana.com',
    wsUrl: 'wss://api.devnet.solana.com',
  },
  LOCALNET: {
    url: 'http://localhost:8899',
    wsUrl: 'ws://localhost:8900',
  },
} as const;
