const path = require('node:path');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const webpack = require('webpack');
const fs = require('node:fs/promises');
const { createHash } = require('node:crypto');

module.exports = {
  entry: { index: './src/index.ts', loader: './src/loader.ts' },
  target: 'web',
  devtool: 'source-map',
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: '[name].js',
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
    {
      apply(compiler) {
        compiler.hooks.thisCompilation.tap('DecoderArtifact', compilation => {
          compilation.hooks.processAssets.tapPromise({
            name: 'DecoderArtifact',
            stage: webpack.Compilation.PROCESS_ASSETS_STAGE_ADDITIONAL,
          }, async () => {
            const bytes = await fs.readFile(path.resolve(__dirname, 'pkg/seedsong_core_bg.wasm'));
            compilation.emitAsset('decoder.wasm', new webpack.sources.RawSource(bytes));
            compilation.emitAsset('decoder.json', new webpack.sources.RawSource(JSON.stringify({
              abiVersion: 1,
              wasmBytes: bytes.length,
              sha256: createHash('sha256').update(bytes).digest('hex'),
              note: 'Use with the loader from this exact build. Not yet optimized to the M2 60 KB target.',
            }, null, 2) + '\n'));
          });
        });
      },
    },
  ],
  module: {
    rules: [
      { resourceQuery: /source/, type: 'asset/source' },
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
    extensionAlias: { '.js': ['.js', '.ts'] },
    fallback: { buffer: require.resolve('buffer/') },
  },
};
