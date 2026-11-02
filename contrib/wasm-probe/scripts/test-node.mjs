import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

const artifact_directory = new URL('../artifacts/', import.meta.url);
const wasm_bytes = await readFile(new URL('ord_core_poc.wasm', artifact_directory));
const fixture_document = JSON.parse(await readFile(new URL('native-vectors.json', artifact_directory), 'utf8'));
const wasm_module = await WebAssembly.compile(wasm_bytes);
const import_array = WebAssembly.Module.imports(wasm_module);
assert.deepEqual(import_array, []);
const instance = await WebAssembly.instantiate(wasm_module, {});
const encoder = new TextEncoder();
const decoder = new TextDecoder();

function call(request) {
	const input_bytes = encoder.encode(JSON.stringify(request));
	const input_pointer = instance.exports.ord_alloc(input_bytes.byteLength);
	assert.notEqual(input_pointer, 0);
	new Uint8Array(instance.exports.memory.buffer, input_pointer, input_bytes.byteLength).set(input_bytes);
	let output_pointer = 0;
	let output_length = 0;
	try {
		const result = instance.exports.ord_process(input_pointer, input_bytes.byteLength);
		output_pointer = Number(result & 0xffffffffn);
		output_length = Number(result >> 32n);
		assert.notEqual(output_pointer, 0);
		return JSON.parse(decoder.decode(new Uint8Array(instance.exports.memory.buffer, output_pointer, output_length)));
	} finally {
		instance.exports.ord_free(input_pointer, input_bytes.byteLength);
		if (output_pointer !== 0) {
			instance.exports.ord_free(output_pointer, output_length);
		}
	}
}

for (const vector of fixture_document.vectors) {
	assert.deepEqual(call(vector.request), vector.response, vector.name);
}

assert.equal(instance.exports.ord_alloc(0), 0);
assert.equal(instance.exports.ord_alloc(65537), 0);
assert.equal(instance.exports.ord_process(0, 1), 0n);
assert.equal(instance.exports.ord_process(1, 65537), 0n);
const invalid_pointer = instance.exports.ord_alloc(1);
new Uint8Array(instance.exports.memory.buffer, invalid_pointer, 1).set([0xff]);
const invalid_result = instance.exports.ord_process(invalid_pointer, 1);
const invalid_output_pointer = Number(invalid_result & 0xffffffffn);
const invalid_output_length = Number(invalid_result >> 32n);
assert.ok(JSON.parse(decoder.decode(new Uint8Array(instance.exports.memory.buffer, invalid_output_pointer, invalid_output_length))).error);
instance.exports.ord_free(invalid_output_pointer, invalid_output_length);
instance.exports.ord_free(invalid_pointer, 1);

const memory_before = instance.exports.memory.buffer.byteLength;
for (let iteration = 0; iteration < 10000; iteration += 1) {
	assert.deepEqual(call(fixture_document.vectors[0].request), fixture_document.vectors[0].response);
}
const memory_after = instance.exports.memory.buffer.byteLength;
assert.equal(memory_after, memory_before);

const result = {
	runtime: process.version,
	wasm_path: 'artifacts/ord_core_poc.wasm',
	wasm_bytes: wasm_bytes.byteLength,
	sha256: createHash('sha256').update(wasm_bytes).digest('hex'),
	vector_count: fixture_document.vectors.length,
	imports: import_array,
	exports: WebAssembly.Module.exports(wasm_module),
	repeated_calls: 10000,
	memory_before,
	memory_after,
	status: 'passed',
};
await writeFile(new URL('node-result.json', artifact_directory), `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
