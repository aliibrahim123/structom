#!/usr/bin/env -S deno run

import { render } from 'npm:bit-field';
import { stringify } from 'npm:onml';
import { readdir, rm, writeFile } from 'node:fs/promises';

for (const file of await readdir('./spec/')) {
	if (file.endsWith('.svg')) {
		await rm(`./spec/${file}`);
	}
}

function u8(name) {
	return { bits: 1, name, attr: 'u8' };
}
function u16(name) {
	return { bits: 2, name, attr: 'u16' };
}
function u32(name) {
	return { bits: 4, name, attr: 'u32' };
}
function u64(name) {
	return { bits: 8, name, attr: 'u64' };
}
function i8(name) {
	return { bits: 1, name, attr: 'i8' };
}
function i16(name) {
	return { bits: 2, name, attr: 'i16' };
}
function i32(name) {
	return { bits: 4, name, attr: 'i32' };
}
function i64(name) {
	return { bits: 8, name, attr: 'i64' };
}
function pad(bytes) {
	return { bits: bytes };
}

const encodings = {
	binary_basic: [u16('a'), u8('b'), pad(1), i32('c')],
};

for (const [name, encoding] of Object.entries(encodings)) {
	let diagram = render(encoding, { vflip: true });
	writeFile(`./spec/${name}.svg`, stringify(diagram));
}
