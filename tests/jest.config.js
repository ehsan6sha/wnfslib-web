module.exports = {
    preset: 'ts-jest', // Use ts-jest preset to handle TypeScript files
    testEnvironment: 'node', // Use Node.js environment for testing
    transform: {
      '^.+\\.tsx?$': 'ts-jest', // Transform .ts and .tsx files using ts-jest
    },
    extensionsToTreatAsEsm: ['.ts'], // Treat TypeScript files as ES Modules
    globals: {
      'ts-jest': {
        useESM: true, // Enable ESM support in ts-jest
      },
    },
    moduleNameMapper: {
      // Map WebAssembly imports to the correct path
      '\\.wasm$': '<rootDir>/__mocks__/wasmMock.js',
    },
  };