import { Server, TransactionBuilder, Networks, Contract } from 'soroban-client';

const server = new Server('https://horizon-testnet.stellar.org');
const contractId = 'CCDP5IJT5AZGLCRNBKYXNQ24E6XQYOWAGB2JQR52OQXE7IIP2HM6PIIV';

export async function createLease(assetId, duration, price) {
  const pubKey = await window.freighterApi.getPublicKey();
  const account = await server.getAccount(pubKey);

  const tx = new TransactionBuilder(account, {
    fee: '100',
    networkPassphrase: Networks.TESTNET,
  })
    .addOperation(
      Contract.call(contractId, 'create_lease', [assetId, duration, price])
    )
    .setTimeout(30)
    .build();

  const signedTx = await window.freighterApi.signTransaction(tx.toXDR(), {
    network: 'TESTNET'
  });

  const txResult = await server.submitTransaction(TransactionBuilder.fromXDR(signedTx, Networks.TESTNET));
  console.log('Lease created!', txResult);
}
