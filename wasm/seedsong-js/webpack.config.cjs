const path = require('node:path');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const webpack = require('webpack');

module.exports = {
  entry: './src/index.ts',
  target: 'web',
  devtool: 'source-map',
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'index.js',
    library: { type: 'module' },
    chunkFormat: false,
    clean: true,
  },
  plugins: [
    new WasmPackPlugin({
      crateDirectory: path.resolve(__dirname, '../../crates/seedsong-core'),
      outDir: path.resolve(__dirname, 'pkg'),
      outName: 'seedsong_core',
      extraArgs: '--target web --no-pack --no-opt -- --locked',
    }),
    new webpack.ProvidePlugin({ Buffer: ['buffer', 'Buffer'] }),
  ],
  module: {
    rules: [
      { test: /\.wasm$/, loader: 'arraybuffer-loader', type: 'javascript/auto' },
      {
        test: /\.ts$/,
        use: {
          loader: 'ts-loader',
          options: {
            configFile: path.resolve(__dirname, 'tsconfig.json'),
            // JS is bundled here; tsc emits public declarations after webpack.
            compilerOptions: { emitDeclarationOnly: false, declaration: false, declarationMap: false },
          },
        },
        exclude: /node_modules/,
      },
    ],
  },
  experiments: { outputModule: true },
  resolve: {
    extensions: ['.ts', '.js'],
    fallback: { buffer: require.resolve('buffer/') },
  },
};
