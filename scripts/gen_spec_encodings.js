#!/usr/bin/env -S deno run

import { render } from './bit-field.js';
import { stringify } from 'npm:onml';
import { readdir, rm, writeFile } from 'node:fs/promises';

for (const file of await readdir('./spec/')) {
	if (file.endsWith('.svg')) {
		await rm(`./spec/${file}`);
	}
}

function u8(name, type = 'u8') {
	return { bits: 1, name, attr: type };
}
function u16(name, type = 'u16') {
	return { bits: 2, name, attr: type };
}
function u32(name, type = 'u32') {
	return { bits: 4, name, attr: type };
}
function u64(name, type = 'u64') {
	return { bits: 8, name, attr: type };
}
function i8(name, type = 'i8') {
	return { bits: 1, name, attr: type };
}
function i16(name, type = 'i16') {
	return { bits: 2, name, attr: type };
}
function i32(name, type = 'i32') {
	return { bits: 4, name, attr: type };
}
function i64(name, type = 'i64') {
	return { bits: 8, name, attr: type };
}
function pad(bytes) {
	return { bits: bytes };
}
function typeid(name) {
	return { bits: 4, name, attr: 'typeid' };
}

const encodings = {
	binary_example: {
		lanes: 2,
		encoding: [
			u16('a'),
			u8('b'),
			u8('', 'pad'),
			i32('ptr_to_item1'),
			u32('len'),
			u16('item0'),
			u16('item1'),
			u16('item2'),

			u16('01 02'),
			u8('03'),
			u8(''),
			i32('0a'),
			u32('03'),
			u16('04 05'),
			u16('06 07'),
			u16('08 09'),
		],
	},
	header: [
		u8('53', ''),
		u8('54', ''),
		u8('4F', ''),
		u8('4D', ''),
		u32('imports_len'),
		u32('import_n', 'str * n'),
		u32('sections_len'),
		u64('section_n', 'u64 * n'),
		typeid('root_value'),
	],
	header_example: [
		u8('53', ''),
		u8('54', ''),
		u8('4F', ''),
		u8('4D', ''),
		u32('01 00 00 00', 'imports_len'),
		u32('06 00 00 00', 'import1_len'),
		...Array.from('schema').map((v) => u8(v.charCodeAt(0).toString(16), '')),
		pad(2),
		u32('01 00 00 00', 'sections_len'),
		u16('01 00', 'ns'),
		u16('00 00', 'id'),
	],
	sections: [
		u32('obj0_len'),
		{ name: 'obj0_fields', bits: 10 },
		pad(2),
		u32('obj1_len'),
		{ name: 'obj1_fields', bits: 7 },
		u16('...', ''),
	],
	type_id: [u16('ns'), u16('id')],
	typeid_example: {
		lanes: 2,
		margin: { left: 60 },
		label: { left: ['`u8`', '`Enum`'] },
		encoding: [u16('00 00', 'ns'), u16('10 00', 'id'), u16('01 00', 'ns'), u16('01 00', 'id')],
	},
};

for (const [name, encoding] of Object.entries(encodings)) {
	let diagram_raw = Array.isArray(encoding)
		? render(encoding)
		: render(encoding.encoding, encoding);

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
