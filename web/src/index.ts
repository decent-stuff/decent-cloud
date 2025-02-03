import { Actor, ActorMethod, HttpAgent, ActorSubclass } from '@dfinity/agent';
import { Principal } from '@dfinity/principal';
import { IDL } from '@dfinity/candid';
// import { LedgerMapWrapper } from "@decent-stuff/ledger-map";
import init, { WasmLedgerMapWrapper } from '../../../ledger-map/dist/wasm';

// Initialize
const ledger = new LedgerMapWrapper();
await ledger.initialize();

// Store data
const key = new TextEncoder().encode("alice");
const value = new TextEncoder().encode("data");

ledger.beginBlock();
ledger.upsert("users", key, value);
ledger.commitBlock();

// Retrieve data
const retrieved = ledger.get("users", key);

// Constants from dcc-common
const DATA_PULL_BYTES_BEFORE_LEN = 1024;

interface LedgerEntry {
    key: Uint8Array;
    value: Uint8Array;
    label: number;
}

interface LedgerBlock {
    header: {
        version: number;
        blockIndex: number;
    };
    entries: LedgerEntry[];
}

interface LedgerCanisterService {
    data_fetch: ActorMethod<[string | undefined, Uint8Array | undefined], [string, Uint8Array]>;
}

const createLedgerCanisterActor = (agent: HttpAgent, canisterId: Principal): ActorSubclass<LedgerCanisterService> => {
    return Actor.createActor<LedgerCanisterService>(() => {
        const Service = IDL.Service({
            'data_fetch': IDL.Func(
                [IDL.Opt(IDL.Text), IDL.Opt(IDL.Vec(IDL.Nat8))],
                [IDL.Text, IDL.Vec(IDL.Nat8)],
                ['query']
            ),
        });
        return Service;
    }, {
        agent,
        canisterId,
    });
};

export class DecentCloudWebClient {
    private agent: HttpAgent;
    private actor: ActorSubclass<LedgerCanisterService>;
    private storage: Storage;

    constructor(networkUrl: string, canisterId: string) {
        this.agent = new HttpAgent({ host: networkUrl });
        this.actor = createLedgerCanisterActor(
            this.agent,
            Principal.fromText(canisterId)
        );
        this.storage = window.localStorage;
    }

    /**
     * Fetches the latest data from the ledger canister
     */
    async fetchLedgerData(): Promise<void> {
        const currentPosition = BigInt(this.storage.getItem('ledgerPosition') || '0');

        // Get bytes before current position if needed
        const bytesBefore = currentPosition > DATA_PULL_BYTES_BEFORE_LEN ?
            this.readFromStorage(currentPosition - BigInt(DATA_PULL_BYTES_BEFORE_LEN), DATA_PULL_BYTES_BEFORE_LEN) :
            null;

        // Fetch data from canister
        const [cursorRemote, data] = await this.actor.data_fetch(
            currentPosition.toString(),
            bytesBefore || undefined
        );

        const remotePosition = BigInt(cursorRemote.split(',')[1]);

        // Validate remote data
        if (remotePosition < currentPosition) {
            throw new Error('Remote ledger has less data than local');
        }

        // Store the fetched data
        if (data.length > 0) {
            await this.writeToStorage(remotePosition, data);
            this.storage.setItem('ledgerPosition', remotePosition.toString());
        }
    }

    private readFromStorage(position: bigint, length: number): Uint8Array {
        const key = `ledger_${position}`;
        const data = this.storage.getItem(key);
        if (!data) {
            return new Uint8Array(0);
        }
        return new Uint8Array(JSON.parse(data));
    }

    private async writeToStorage(position: bigint, data: Uint8Array): Promise<void> {
        const key = `ledger_${position}`;
        this.storage.setItem(key, JSON.stringify(Array.from(data)));
    }

    /**
     * Gets the current local ledger position
     */
    getCurrentPosition(): bigint {
        return BigInt(this.storage.getItem('ledgerPosition') || '0');
    }

    /**
     * Clears all stored ledger data
     */
    clearStorage(): void {
        for (let i = 0; i < this.storage.length; i++) {
            const key = this.storage.key(i);
            if (key?.startsWith('ledger_')) {
                this.storage.removeItem(key);
            }
        }
        this.storage.removeItem('ledgerPosition');
    }
}
