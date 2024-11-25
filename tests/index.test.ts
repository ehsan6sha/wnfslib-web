import init, { init_native } from '../pkg/wnfslib_web'; // Adjust path as needed

class InMemoryDatastore {
  private store: Map<string, Uint8Array> = new Map();
  private totalBytesPut = 0;
  private totalBytesGet = 0;

  async put(cid: Uint8Array, data: Uint8Array): Promise<Uint8Array> {
    const key = this._encodeBase64(cid);
    this.store.set(key, data);
    this.totalBytesPut += data.length;
    return cid;
  }

  async get(cid: Uint8Array): Promise<Uint8Array> {
    const key = this._encodeBase64(cid);
    if (!this.store.has(key)) {
      throw new Error(`No data found for CID: ${key}`);
    }
    const data = this.store.get(key)!;
    this.totalBytesGet += data.length;
    return data;
  }

  private _encodeBase64(bytes: Uint8Array): string {
    return Buffer.from(bytes).toString('base64');
  }
}

describe('init_native with InMemoryDatastore', () => {
  beforeAll(async () => {
    await init(); // Initialize WebAssembly module
  });

  it('should initialize with an in-memory datastore and return a valid CID', async () => {
    const datastore = new InMemoryDatastore();

    const jsClient = {
      get: (cid: Uint8Array) => datastore.get(cid),
      put: (cid: Uint8Array, data: Uint8Array) => datastore.put(cid, data),
    };

    const wnfsKey = new Uint8Array([0x01, 0x02, 0x03, 0x04]);
    const result = await init_native(jsClient, wnfsKey);

    expect(result).toBeDefined();
    expect(result.cid).toBeDefined();
    console.log('Initialization successful. CID:', result.cid);
  });
});