declare module '*.wasm' {
  const bytes: ArrayBuffer;
  export default bytes;
}

declare module '*?source' {
  const source: string;
  export default source;
}
