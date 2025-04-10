export async function connectWallet() {
    if (!window.freighterApi) {
      alert('Freighter wallet not found!');
      return;
    }
    return await window.freighterApi.enable();
  }
  
  export async function getPublicKey() {
    return await window.freighterApi.getPublicKey();
  }
  