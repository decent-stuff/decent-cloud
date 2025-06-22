export class WasmLedgerMap {
    private storage: Map<string, Map<string, Uint8Array>>;
    private labels: Set<string>;
    private blockCount: number;
    private lastBlockHash: Uint8Array;

    constructor(labels?: string[]) {
        this.storage = new Map();
        this.labels = new Set(labels || []);
        this.blockCount = 0;
        this.lastBlockHash = new Uint8Array([1, 2, 3, 4]);

        // Initialize storage for each label
        if (labels) {
            labels.forEach(label => {
                this.storage.set(label, new Map());
            });
        }
    }

    upsert(label: string, key: Uint8Array, value: Uint8Array): void {
        if (!this.labels.has(label)) {
            throw new Error(`Invalid label: ${label}`);
        }

        const labelStorage = this.storage.get(label)!;
        const keyString = Array.from(key).toString();
        labelStorage.set(keyString, value);
    }

    get(label: string, key: Uint8Array): Uint8Array {
        if (!this.labels.has(label)) {
            throw new Error(`Invalid label: ${label}`);
        }

        const labelStorage = this.storage.get(label)!;
        const keyString = Array.from(key).toString();
        const value = labelStorage.get(keyString);

        if (!value) {
            throw new Error('Key not found');
        }

        return value;
    }

    delete(label: string, key: Uint8Array): void {
        if (!this.labels.has(label)) {
            throw new Error(`Invalid label: ${label}`);
        }

        const labelStorage = this.storage.get(label)!;
        const keyString = Array.from(key).toString();
        labelStorage.delete(keyString);
    }

    commit_block(): void {
        this.blockCount++;
        // Update mock block hash
        const hashArray = Array.from(this.lastBlockHash);
        hashArray[hashArray.length - 1] = this.blockCount;
        this.lastBlockHash = new Uint8Array(hashArray);
    }

    get_blocks_count(): number {
        return this.blockCount;
    }

    get_latest_block_hash(): Uint8Array {
        return this.lastBlockHash;
    }

    refresh(): void {
        // Mock implementation maintains current state
    }
}

export default function init(): Promise<void> {
    return Promise.resolve();
}
