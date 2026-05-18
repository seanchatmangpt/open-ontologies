// Minimal Deno global declarations for Node TypeScript LSP compatibility.
// These types are provided by the Deno runtime — this file exists only to
// suppress false positives from editors using the Node TS language server.
declare namespace Deno {
  function serve(handler: (req: Request) => Response | Promise<Response>): void;
  const env: {
    get(key: string): string | undefined;
  };
}
