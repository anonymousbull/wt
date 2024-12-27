const fs = require('fs');

const bs58 = require('bs58');
const { Keypair } = require('@solana/web3.js');

const privateKey = '4YFq9y5f5hi77Bq8kDCE6VgqoAqKGSQN87yW9YeGybpNfqKUG4WxnwhboHGUeXjY7g8262mhL1kCCM9yy8uGvdj7';

const keypairData = Keypair.fromSeed(Uint8Array.from(bs58.default.decode(privateKey).slice(0, 32)));

const secretKey = `[${keypairData.secretKey.toString()}]`;

fs.writeFileSync('keypair.json', secretKey);

console.log('Public Key:', keypairData.publicKey.toString());
console.log('Secret Key saved to keypair.json');