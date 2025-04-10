import { connectWallet, getPublicKey } from './utils/stellar.js';
import { createLease, getLeaseStatus } from './utils/leaseManager.js';

document.getElementById('connect-wallet').addEventListener('click', async () => {
  await connectWallet();
  const pubKey = await getPublicKey();
  document.getElementById('wallet-address').textContent = `Connected: ${pubKey}`;
});
