import { defineConfig } from 'tsup';

export default defineConfig({
  entry: {
    index: 'src/index.ts',
    client: 'src/client.ts',
    types: 'src/types.ts',
    constants: 'src/constants.ts',
    utils: 'src/utils.ts',
  },
  format: ['cjs', 'esm'],
  dts: true,
  sourcemap: true,
  clean: true,
  minify: false,
  splitting: false,
  treeshake: true,
  external: [
    '@solana/web3.js',
    '@coral-xyz/anchor',
    '@solana/spl-token',
    'bn.js',
  ],
  esbuildOptions(options) {
    options.footer = {
      js: 'module.exports = module.exports.default || module.exports;',
    };
  },
});
