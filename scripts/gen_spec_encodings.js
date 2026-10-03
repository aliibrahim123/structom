#!/usr/bin/env -S deno run

import { render } from './bit-field.js';
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
function typeid(name) {
	return { bits: 4, name, attr: 'typeid' };
}

const encodings = {
	binary_basic: [
		u16('a'),
		u8('b'),
		{ bits: 1, name: 'pad' },
		i32('c'),
		u32('len'),
		u16('item0'),
		u16('item1'),
		u16('item2'),
	],
	header: [
		{ name: '53', bits: 1 },
		{ name: '54', bits: 1 },
		{ name: '4F', bits: 1 },
		{ name: '4D', bits: 1 },
		u32('imports_len'),
		{ name: 'import_n', bits: 4, attr: 'str * n' },
		u32('sections_len'),
		{ name: 'section_n', bits: 8, attr: 'u64 * n' },
		typeid('root_value'),
	],
	sections: [
		u32('obj0_len'),
		{ name: 'obj0_fields', bits: 10 },
		pad(2),
		u32('obj1_len'),
		{ name: 'obj1_fields', bits: 7 },
		{ name: '...', bits: 2 },
	],
};

for (const [name, encoding] of Object.entries(encodings)) {
	let diagram_raw = render(encoding);

	let rootG = diagram_raw[3];
	for (let laneG of rootG.slice(2)) {
		let cage = laneG[2];
		if (cage && cage[1] && cage[1].stroke === 'black') {
			laneG.splice(2, 0, ['rect', { class: 'bg', width: w, height: h }]);
		}
	}

	let diagram = stringify(diagram_raw);

	writeFile(`./spec/${name}.svg`, diagram);
}
