import { Ed25519KeyIdentity } from '@dfinity/identity'
import { generateMnemonic, mnemonicToSeedSync } from 'bip39'
import { createHmac } from 'crypto'

export function generateNewSeedPhrase(): string {
    return generateMnemonic()
}

export function identityFromSeed(seedPhrase: string): Ed25519KeyIdentity {
    // 1. Generate seed from mnemonic with empty password (matching backend)
    const seed = mnemonicToSeedSync(seedPhrase, '')

    // 2 & 3. Create HMAC-SHA512 with key "ed25519 seed" and feed seed
    const hmac = createHmac('sha512', 'ed25519 seed')
    hmac.update(seed)

    // 4. Get first 32 bytes of HMAC output
    const keyMaterial = hmac.digest()
    const seedBytes = keyMaterial.slice(0, 32)

    // Convert Buffer to ArrayBuffer for DFinity identity
    const privateKeyArrayBuffer = new Uint8Array(seedBytes).buffer

    // Create DFinity identity from private key
    return Ed25519KeyIdentity.fromSecretKey(privateKeyArrayBuffer)
}
