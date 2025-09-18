import { ledgerService } from "../ledger-service";
import { decentCloudLedger, LedgerBlock, LedgerEntry } from "@decent-stuff/dc-client";

describe("ledgerService.getValidators", () => {
  const originalGetAllEntries = decentCloudLedger.getAllEntries;
  const originalGetAllBlocks = decentCloudLedger.getAllBlocks;
  const originalGetBlockEntries = decentCloudLedger.getBlockEntries;

  const mockGetAllEntries = jest.fn<Promise<LedgerEntry[]>, []>();
  const mockGetAllBlocks = jest.fn<Promise<LedgerBlock[]>, []>();
  const mockGetBlockEntries = jest.fn<Promise<LedgerEntry[]>, [number]>();

  beforeEach(() => {
    ledgerService.disconnect();
    mockGetAllEntries.mockReset();
    mockGetAllBlocks.mockReset();
    mockGetBlockEntries.mockReset();

    decentCloudLedger.getAllEntries = mockGetAllEntries;
    decentCloudLedger.getAllBlocks = mockGetAllBlocks;
    decentCloudLedger.getBlockEntries = mockGetBlockEntries;
  });

  afterEach(() => {
    decentCloudLedger.getAllEntries = originalGetAllEntries;
    decentCloudLedger.getAllBlocks = originalGetAllBlocks;
    decentCloudLedger.getBlockEntries = originalGetBlockEntries;
  });

  it("includes validators recorded with legacy provider labels", async () => {
    const principal = "legacy-principal";
    const legacyCheckIn: LedgerEntry = {
      label: "NPCheckIn",
      key: `0 1 ${principal}`,
      value: {
        memo: "legacy memo",
      },
      description: "legacy provider check-in",
      blockOffset: 5,
    };

    mockGetAllEntries.mockResolvedValue([legacyCheckIn]);
    mockGetAllBlocks.mockResolvedValue([
      {
        blockOffset: 5,
        timestampNs: 1234,
        blockHash: "hash",
        parentBlockHash: "parent",
        blockSize: 0,
        blockVersion: 1,
        fetchCompareBytes: "",
        fetchOffset: 0,
      },
    ]);
    mockGetBlockEntries.mockResolvedValue([]);

    const validators = await ledgerService.getValidators();

    expect(validators).toHaveLength(1);
    expect(validators[0]).toMatchObject({
      principal,
      blocksValidated: 1,
      memo: "legacy memo",
    });
  });
});
