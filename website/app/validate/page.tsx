import { BlockchainValidator } from '@/components/blockchain-validator';

export default function ValidatePage() {
  return (
    <div className="container mx-auto py-12">
      <h1 className="text-3xl font-bold mb-8 text-center">Blockchain Validation</h1>
      <p className="text-center mb-8 max-w-2xl mx-auto">
        This page allows you to validate the blockchain by checking in as a node provider.
        The validation process consists of getting the latest block hash, signing it, and
        sending it to the canister.
      </p>
      <BlockchainValidator />
    </div>
  );
}
