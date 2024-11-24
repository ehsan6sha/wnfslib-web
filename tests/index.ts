import init, { 
    init_rust_logger,
    init_native,
    write_file_native,
    read_file_native,
    mkdir_native,
    ls_native,
    load_with_wnfs_key_native
} from '../pkg/wnfslib_web';

// Add this before your WebDatastore class
declare global {
    function getFromStorage(key: string): Promise<Uint8Array>;
    function putToStorage(key: string, value: Uint8Array): Promise<void>;
}

// Implement the global functions using your WebDatastore instance
let globalStore: WebDatastore;

window.getFromStorage = async (key: string): Promise<Uint8Array> => {
    return globalStore.getFromStorage(key);
};

window.putToStorage = async (key: string, value: Uint8Array): Promise<void> => {
    return globalStore.putToStorage(key, value);
};

class WebDatastore {
    private store: Map<string, Uint8Array>;
    private prefix: string;

    constructor(prefix: string) {
        this.store = new Map();
        this.prefix = prefix;
    }

    async getFromStorage(key: string): Promise<Uint8Array> {
        const value = this.store.get(key);
        if (!value) {
            throw new Error(`Key not found: ${key}`);
        }
        return value;
    }

    async putToStorage(key: string, value: Uint8Array): Promise<void> {
        this.store.set(key, value);
    }
}

async function runTests() {
    const log = (msg: string, isError = false) => {
        const div = document.getElementById('logs')!;
        const p = document.createElement('p');
        p.textContent = msg;
        p.className = isError ? 'error' : 'success';
        div.appendChild(p);
        console.log(msg);
    };

    try {
        // Initialize WASM module
        await init();
        init_rust_logger();

        // Create datastore
        globalStore = new WebDatastore('test-');
        const prefix = 'test-store';

        // Generate WNFS key
        const encoder = new TextEncoder();
        const keyPhrase = encoder.encode('test');
        const wnfsKey = await crypto.subtle.digest('SHA-256', keyPhrase);
        const wnfsKeyArray = new Uint8Array(wnfsKey);

        log('Initializing WNFS...');
        const config = await init_native(prefix, wnfsKeyArray);
        log(`Root CID: ${config}`);

        // Test mkdir
        log('Creating directory...');
        const mkdirCid = await mkdir_native(prefix, config, 'opt');
        log(`Directory created with CID: ${mkdirCid}`);

        // Test write file
        log('Writing file...');
        const content = encoder.encode('Hello, World!');
        const writeFileCid = await write_file_native(prefix, mkdirCid, 'root/test.txt', content);
        log(`File written with CID: ${writeFileCid}`);

        // Test read file
        log('Reading file...');
        const readContent = await read_file_native(prefix, writeFileCid, 'root/test.txt');
        const decoder = new TextDecoder();
        const readText = decoder.decode(readContent);
        log(`File content: ${readText}`);

        // Test ls
        log('Listing directory...');
        const listing = await ls_native(prefix, writeFileCid, 'root');
        log(`Directory listing: ${decoder.decode(listing)}`);

        // Test reload
        log('Testing reload...');
        const configReloaded = await load_with_wnfs_key_native(prefix, wnfsKeyArray, writeFileCid);
        log(`Reloaded Root CID: ${configReloaded}`);
        const reloadedListing = await ls_native(prefix, writeFileCid, 'root');
        log(`Reloaded directory listing: ${decoder.decode(reloadedListing)}`);

        log('All tests completed successfully!');
    } catch (error) {
        log(`Test failed: ${error}`, true);
    }
}

runTests();