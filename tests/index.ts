import init, {
  init_native,
  mkdir_native,
  ls_native,
  write_file_native,
  load_with_wnfs_key_native,
  read_file_native,
} from '../pkg/wnfslib_web'; // Adjust path to your WebAssembly package

import { CID } from 'multiformats/cid';

class InMemoryDatastore {
  private store: Map<string, Uint8Array> = new Map();
  private totalBytesPut = 0;
  private totalBytesGet = 0;

  // Simulates storing data and returns the CID
  async put(cid: Uint8Array, data: Uint8Array): Promise<Uint8Array> {
    const cidString = this.cidToString(cid); // Convert CID Uint8Array to string
    this.store.set(cidString, data);
    this.totalBytesPut += data.length;

    console.log('put', { cid: cid, data: data });

    return cid; // Return the original CID as confirmation
  }

  // Simulates retrieving data by CID
  async get(cid: Uint8Array): Promise<Uint8Array> {
    const cidString = this.cidToString(cid); // Convert CID Uint8Array to string
    if (!this.store.has(cidString)) {
      console.error(`No data found for CID: ${cidString}`);
      throw new Error(`No data found for CID: ${cidString}`);
    }
    const data = this.store.get(cidString)!; // Use non-null assertion since we checked existence
    this.totalBytesGet += data.length;

    console.log('get', { cid: cid, base64Data: data });

    return data;
  }

  // Helper method to encode Uint8Array to Base64
  private encodeToBase64(bytes: Uint8Array): string {
    return btoa(String.fromCharCode(...bytes));
  }

  // Helper method to convert a Uint8Array (binary CID) to a string representation (CIDv1)
  public cidToString(bytes: Uint8Array): string {
    const cid = CID.decode(bytes); // Decode binary CID into a CID object
    return cid.toString(); // Convert CID object to its string representation (default is CIDv1)
  }

  // Helper method to convert a string representation (CIDv1) back to a Uint8Array
  public stringToCid(str: string): Uint8Array {
    const cid = CID.parse(str); // Parse the string into a CID object
    return cid.bytes; // Get the binary representation of the CID as Uint8Array
  }

  getTotalBytesPut(): number {
    return this.totalBytesPut;
  }

  getTotalBytesGet(): number {
    return this.totalBytesGet;
  }
}

async function main() {
  console.log('Initializing WebAssembly module...');

  // Initialize the WebAssembly module
  await init();

  console.log('WebAssembly module initialized.');

  // Create an instance of InMemoryDatastore
  const datastore = new InMemoryDatastore();

  // Wrap datastore methods to match expected jsClient interface
  const jsClient = {
    get: (cid: Uint8Array): Promise<Uint8Array> => datastore.get(cid),
    put: (cid: Uint8Array, data: Uint8Array): Promise<Uint8Array> => datastore.put(cid, data),
  };

  const partialKey = new Uint8Array([0x01, 0x02, 0x03, 0x04]);

  // Create a new Uint8Array of length 32 and copy the partialKey into it
  const wnfsKey = new Uint8Array(32);
  wnfsKey.set(partialKey);

  console.log('Padded wnfsKey:', wnfsKey);

  console.log('Calling init_native with in-memory datastore...');

  try {
    // Call init_native with the in-memory datastore
    const rootCid = await init_native(jsClient, wnfsKey);

    console.log('Initialization successful.');
    console.log('rootCid:', datastore.cidToString(rootCid));

    console.log('Creating directory...');
    const rootCidMkdir = await mkdir_native(jsClient, rootCid, 'root/test');
    console.log('Directory created with new CID:', datastore.cidToString(rootCidMkdir));

    console.log('Writing file...');
    const helloContent = new TextEncoder().encode('Hello'); // Encode "Hello" as bytes
    const fileCid = await write_file_native(
      jsClient,
      rootCidMkdir,
      'root/hello.txt',
      helloContent,
      BigInt(Math.floor(Date.now() / 1000)) // Convert number to bigint
    );
    console.log('File written with new CID:', datastore.cidToString(fileCid));

    console.log('Listing files after writing...');
    const filesAfterWrite = await ls_native(jsClient, fileCid, 'root');
    console.log('Files in directory after writing:', filesAfterWrite);

    console.log('Reloading WNFS...');
    const reloadedHelper = await load_with_wnfs_key_native(jsClient, fileCid, wnfsKey);
    console.log('WNFS reloaded successfully.', reloadedHelper);

    console.log('Listing files after reload...');
    const filesAfterReload = await ls_native(jsClient, fileCid, 'root');
    console.log('Files in directory after reload:', filesAfterReload);

    console.log('Reading file...');
    const fileContentArray = await read_file_native(jsClient, fileCid, 'root/hello.txt');

    // Extract the actual file content
    console.log({fileContentArray: fileContentArray});

    // Ensure it's an array and convert it to Uint8Array
    if (!Array.isArray(fileContentArray)) {
      throw new TypeError('Expected fileContent to be an array');
    }

    const fileContent = new Uint8Array(fileContentArray);

    console.log(
      'Read file content:',
      new TextDecoder().decode(fileContent) // Decode bytes back into text
    );

    console.log('Total bytes put:', datastore.getTotalBytesPut());
    console.log('Total bytes retrieved:', datastore.getTotalBytesGet());
    
  } catch (error) {
    console.error('Error during initialization or operations:', error);
  }
}

// Run the main function
main();