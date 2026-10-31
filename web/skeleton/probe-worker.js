// The wasm-threads probe's two workers (TASK-M8-86, R-627): both instantiate one wasm module on one shared
// WebAssembly.Memory. The second writes a value through the module's atomic store and notifies; the first, waiting on
// it, reads the value back through Atomics and through the module, and says "ok", or why not.

const VALUE = 0x5eed;
const VALUE_AT = 0;
const FLAG_AT = 1;
const WAIT_MS = 3000;

// (module
//   (import "env" "memory" (memory 1 1 shared))
//   (func (export "write") (param i32) i32.const 0 local.get 0 i32.atomic.store)
//   (func (export "read") (result i32) i32.const 0 i32.atomic.load))
const MODULE = new Uint8Array([
  0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
  0x01, 0x09, 0x02, 0x60, 0x01, 0x7f, 0x00, 0x60, 0x00, 0x01, 0x7f,
  0x02, 0x10, 0x01, 0x03, 0x65, 0x6e, 0x76, 0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x03, 0x01, 0x01,
  0x03, 0x03, 0x02, 0x00, 0x01,
  0x07, 0x10, 0x02, 0x05, 0x77, 0x72, 0x69, 0x74, 0x65, 0x00, 0x00, 0x04, 0x72, 0x65, 0x61, 0x64, 0x00, 0x01,
  0x0a, 0x15, 0x02,
  0x0a, 0x00, 0x41, 0x00, 0x20, 0x00, 0xfe, 0x17, 0x02, 0x00, 0x0b,
  0x08, 0x00, 0x41, 0x00, 0xfe, 0x10, 0x02, 0x00, 0x0b,
]);

self.onmessage = async ({ data: { role, memory } }) => {
  try {
    const { instance } = await WebAssembly.instantiate(MODULE, { env: { memory } });
    const shared = new Int32Array(memory.buffer);
    if (role === "second") {
      instance.exports.write(VALUE);
      Atomics.store(shared, FLAG_AT, 1);
      Atomics.notify(shared, FLAG_AT);
      self.postMessage("written");
      return;
    }
    if (Atomics.wait(shared, FLAG_AT, 0, WAIT_MS) === "timed-out") {
      self.postMessage("the second worker's write never arrived");
      return;
    }
    const read = [Atomics.load(shared, VALUE_AT), instance.exports.read()];
    self.postMessage(read.every((v) => v === VALUE) ? "ok" : `read ${read}, not ${VALUE}`);
  } catch (e) {
    self.postMessage(`${e.name}: ${e.message}`);
  }
};
