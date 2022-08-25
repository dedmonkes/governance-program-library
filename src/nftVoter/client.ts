import { Program, Provider } from '@project-serum/anchor';
import { PublicKey } from '@solana/web3.js';
import { NftVoter, IDL } from './nft_voter';

export const NFT_VOTER_ID = new PublicKey(
  '4MJn3vLQyVhwF77iQ2uF2iSLDE3rWT5mmNw58SDtz79Z',
);

export class NftVoterClient {
  constructor(public program: Program<NftVoter>, public devnet?: boolean) {}

  static async connect(
    provider: Provider,
    devnet?: boolean,
  ): Promise<NftVoterClient> {
    // alternatively we could fetch from chain
    // const idl = await Program.fetchIdl(VSR_ID, provider);
    const idl = IDL;

    return new NftVoterClient(
      new Program<NftVoter>(idl as NftVoter, NFT_VOTER_ID, provider),
      devnet,
    );
  }
}
