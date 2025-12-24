export interface UUID {
	type: 'uuid',
	value: Uint8Array
};
export interface Dur {
	type: 'dur',
	value: bigint
}
export type Value = 
	boolean | number | string | bigint | Date | UUID | Dur | Array<Value> | Map<Value, Value>;