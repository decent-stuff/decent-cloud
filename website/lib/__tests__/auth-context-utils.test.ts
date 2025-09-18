import { createHmac } from "crypto";
import { Ed25519KeyIdentity } from "@dfinity/identity";
import { mnemonicToSeedSync } from "bip39";
import {
  generateNewSeedPhrase,
  identityFromSeed,
} from "@/lib/auth-context";

describe("auth-context utilities", () => {
  it("generates mnemonics with expected length", () => {
    const phrase = generateNewSeedPhrase();
    expect(phrase.split(/\s+/)).toHaveLength(12);
  });

  it("derives the same identity as reference HMAC implementation", () => {
    const mnemonic =
      "laundry mom vaccine winter miss poetry busy initial toe cupboard prefer debate";

    const derivedIdentity = identityFromSeed(mnemonic);

    const seed = mnemonicToSeedSync(mnemonic, "");
    const referenceDigest = createHmac("sha512", "ed25519 seed")
      .update(seed)
      .digest();
    const referenceSeed = referenceDigest.subarray(0, 32);
    const referenceIdentity = Ed25519KeyIdentity.fromSecretKey(
      Uint8Array.from(referenceSeed).buffer
    );

    expect(derivedIdentity.getPrincipal().toString()).toEqual(
      referenceIdentity.getPrincipal().toString()
    );
    expect(new Uint8Array(derivedIdentity.getPublicKey().toDer())).toEqual(
      new Uint8Array(referenceIdentity.getPublicKey().toDer())
    );
  });
});
