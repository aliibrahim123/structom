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
function typeid(ns, id) {
	[ns, id] = id ? [ns, id] : ['00 00', ns];
	return [u16(ns, 'ns'), u16(id, 'id')];
}
function builtin(name, id, encoding) {
	let first_word = (id % 256).toString(16).padStart(2, '0');
	let second_word = Math.floor(id / 256)
		.toString(16)
		.padStart(2, '0');
	let res = {
		[`${name}_typeid`]: {
			margin: { left: 55 },
			encoding: typeid(`${first_word} ${second_word}`),
			label: { left: 'typeid' },
		},
	};
	if (encoding) {
		res[`${name}_value`] = {
			margin: { left: 55 },
			encoding,
			label: { left: 'value' },
		};
	}
	return res;
}

function object(name, id, encoding) {
	let res = builtin(name, id, [u32('ptr')]);
	res[`${name}_object`] = {
		margin: { left: 55 },
		encoding,
		label: { left: 'object' },
	};
	return res;
}

const rest = u16('...', '');

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
			i32('0a 00 00 00'),
			u32('03 00 00 00'),
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
		u32('segments_len'),
		u64('segment_n', 'u64 * n'),
		u32('root_value', 'typeid'),
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
		u32('01 00 00 00', 'segments_len'),
		...typeid('01 00', '00 00'),
	],
	segments: [
		u32('obj0_len'),
		{ name: 'obj0_fields', bits: 10 },
		pad(2),
		u32('obj1_len'),
		{ name: 'obj1_fields', bits: 7 },
		rest,
	],
	type_id: [u16('ns'), u16('id'), u32('param_n', 'typeid * n')],
	typeid_example: {
		lanes: 2,
		margin: { left: 90 },
		label: { left: ['`u8`', '`Enum<u8>`'] },
		encoding: [
			...typeid('10 00'),
			u32('', ''),
			...typeid('01 00', '01 00'),
			u16('00 00', 'T_ns'),
			u16('10 00', 'T_id'),
		],
	},
	binary_generic_example: {
		lanes: 2,
		compact: false,
		label: { left: ['Generic', 'a'] },
		encoding: [
			u32('0c 00 00 00', 'a'),
			u16('12 34', 'b'),
			pad(2),
			u16('ff 00', 'c'),
			pad(2),

			u32('03 00 00 00', 'len'),
			u8("'a'", ''),
			u8("'b'", ''),
			u8("'c'", ''),
			pad(4),
		],
	},
	...builtin('none', 0),
	...builtin('bool', 2),
	...object('any', 1, [u32('len'), u32('typeid', ''), { bits: 10, name: 'value' }, rest]),
	...builtin('far', 3, [u32('section'), u32('offset')]),
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
