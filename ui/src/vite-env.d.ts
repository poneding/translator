/// <reference types="vite/client" />

// Vite's `?raw` suffix imports a file as a string. We use it for .ftl locale
// bundles so they can be loaded synchronously at module init.
declare module "*.ftl?raw" {
  const content: string;
  export default content;
}
