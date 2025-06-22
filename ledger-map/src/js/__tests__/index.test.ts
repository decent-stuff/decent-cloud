import { LedgerMap } from '../index';

describe('LedgerMap', () => {
    let ledgerMap: LedgerMap;

    beforeEach(() => {
        ledgerMap = new LedgerMap();
    });

    describe('initialization', () => {
        it('should initialize without options', async () => {
            await expect(ledgerMap.initialize()).resolves.toBeUndefined();
        });

        it('should initialize with empty labels array', async () => {
            await expect(ledgerMap.initialize({ labels: [] })).resolves.toBeUndefined();
        });

        it('should initialize with multiple labels', async () => {
            await expect(ledgerMap.initialize({ labels: ['test1', 'test2', 'test3'] })).resolves.toBeUndefined();
        });

        it('should handle multiple initializations', async () => {
            await ledgerMap.initialize();
            await expect(ledgerMap.initialize()).resolves.toBeUndefined();
        });
    });

    describe('error handling', () => {
        const testKey = new Uint8Array([1, 2, 3]);
        const testValue = new Uint8Array([4, 5, 6]);

        describe('uninitialized state', () => {
            const methods = [
                { name: 'upsert', fn: () => ledgerMap.upsert('test', testKey, testValue) },
                { name: 'get', fn: () => ledgerMap.get('test', testKey) },
                { name: 'delete', fn: () => ledgerMap.delete('test', testKey) },
                { name: 'commitBlock', fn: () => ledgerMap.commitBlock() },
                { name: 'getBlocksCount', fn: () => ledgerMap.getBlocksCount() },
                { name: 'getLatestBlockHash', fn: () => ledgerMap.getLatestBlockHash() },
                { name: 'refreshLedger', fn: () => ledgerMap.refreshLedger() }
            ];

            methods.forEach(({ name, fn }) => {
                it(`should throw error on ${name} if not initialized`, () => {
                    expect(fn).toThrow('LedgerMap not initialized');
                });
            });
        });

        describe('invalid inputs', () => {
            beforeEach(async () => {
                await ledgerMap.initialize({ labels: ['test'] });
            });

            it('should handle empty key in upsert', () => {
                const emptyKey = new Uint8Array();
                expect(() => ledgerMap.upsert('test', emptyKey, testValue)).not.toThrow();
            });

            it('should handle empty value in upsert', () => {
                const emptyValue = new Uint8Array();
                expect(() => ledgerMap.upsert('test', testKey, emptyValue)).not.toThrow();
            });

            it('should handle non-existent label', () => {
                expect(() => ledgerMap.upsert('nonexistent', testKey, testValue)).toThrow();
            });

            it('should handle get with non-existent key', () => {
                const nonExistentKey = new Uint8Array([9, 9, 9]);
                expect(() => ledgerMap.get('test', nonExistentKey)).toThrow();
            });
        });
    });

    describe('CRUD operations', () => {
        const testKey = new Uint8Array([1, 2, 3]);
        const testValue = new Uint8Array([4, 5, 6]);
        const testLabel = 'test';

        beforeEach(async () => {
            await ledgerMap.initialize({ labels: [testLabel] });
        });

        it('should handle upsert and get operations', () => {
            ledgerMap.upsert(testLabel, testKey, testValue);
            const result = ledgerMap.get(testLabel, testKey);
            expect(result).toBeInstanceOf(Uint8Array);
            expect(result.length).toBe(testValue.length);
            expect(Array.from(result)).toEqual(Array.from(testValue));
        });

        it('should handle update of existing key', () => {
            const newValue = new Uint8Array([7, 8, 9]);
            ledgerMap.upsert(testLabel, testKey, testValue);
            ledgerMap.upsert(testLabel, testKey, newValue);
            const result = ledgerMap.get(testLabel, testKey);
            expect(Array.from(result)).toEqual(Array.from(newValue));
        });

        it('should handle delete operation', () => {
            ledgerMap.upsert(testLabel, testKey, testValue);
            ledgerMap.delete(testLabel, testKey);
            expect(() => ledgerMap.get(testLabel, testKey)).toThrow();
        });

        it('should handle delete of non-existent key', () => {
            expect(() => ledgerMap.delete(testLabel, testKey)).not.toThrow();
        });
    });

    describe('block operations', () => {
        beforeEach(async () => {
            await ledgerMap.initialize();
        });

        it('should handle multiple block commits', () => {
            expect(() => {
                ledgerMap.commitBlock();
                ledgerMap.commitBlock();
                ledgerMap.commitBlock();
            }).not.toThrow();
        });

        it('should track blocks count correctly', () => {
            const initialCount = ledgerMap.getBlocksCount();
            ledgerMap.commitBlock();
            expect(ledgerMap.getBlocksCount()).toBe(initialCount + 1);
        });

        it('should provide consistent block hashes', () => {
            ledgerMap.commitBlock();
            const hash1 = ledgerMap.getLatestBlockHash();
            const hash2 = ledgerMap.getLatestBlockHash();
            expect(Array.from(hash1)).toEqual(Array.from(hash2));
        });
    });

    describe('refresh operation', () => {
        const testLabel = 'test';
        const testKey = new Uint8Array([1]);
        const testValue = new Uint8Array([2]);

        beforeEach(async () => {
            await ledgerMap.initialize({ labels: [testLabel] });
        });

        it('should handle multiple refresh operations', () => {
            expect(() => {
                ledgerMap.refreshLedger();
                ledgerMap.refreshLedger();
            }).not.toThrow();
        });

        it('should maintain state after refresh', () => {
            ledgerMap.upsert(testLabel, testKey, testValue);
            ledgerMap.refreshLedger();
            const result = ledgerMap.get(testLabel, testKey);
            expect(Array.from(result)).toEqual(Array.from(testValue));
        });
    });
});
