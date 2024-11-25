const path = require('path');

module.exports = {
  mode: 'development', // Use 'development' mode for better debugging; switch to 'production' for optimized builds
  entry: './index.ts', // Entry point for your application
  module: {
    rules: [
      {
        test: /\.tsx?$/, // Match .ts and .tsx files
        use: 'ts-loader', // Use ts-loader to transpile TypeScript into JavaScript
        exclude: /node_modules/, // Exclude node_modules from processing
      },
    ],
  },
  resolve: {
    extensions: ['.tsx', '.ts', '.js'], // Automatically resolve these extensions when importing modules
  },
  output: {
    filename: 'bundle.js', // Output filename for the bundled JavaScript file
    path: path.resolve(__dirname, 'dist'), // Output directory for bundled files
    clean: true, // Clean the output directory before each build (optional)
  },
  devServer: {
    static: {
      directory: path.join(__dirname, './'), // Serve static files from the root directory or adjust as needed (e.g., 'public')
    },
    compress: true, // Enable gzip compression for served files
    port: 8080, // Port number for the development server (default is fine)
    hot: true, // Enable hot module replacement (HMR) for faster development (optional)
  },
  experiments: {
    asyncWebAssembly: true, // Enable support for asynchronous WebAssembly imports
  },
};