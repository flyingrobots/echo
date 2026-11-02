// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
// Authored programs only. The publication runner never manufactures Core or Target IR.
export const coordinate = 'jedit.text.replace_range@1';

export function pureSource(importLine: string, count: number, packageName = coordinate): string {
  return `package ${packageName};
${importLine}
fn step(xs: List<U8,max=3>) -> List<U8,max=3> {
  let value = if 0u64 < listLen(xs) then listAt(xs,0u64) else 7u8;
  return if listLen(xs) < 3u64 then listAppend(xs,value) else xs;
}
type Input = { left: List<U8,max=2>, right: List<U8,max=1>, index: U64 };
type Output = { items: List<U8,max=3>, length: U64, selected: U8, nested: List<List<U8,max=3>,max=1> };
intent assembleRange(input: Input) returns Output
  profile text.replaceRange basis none budget <= text.replaceRangeBudget
{
  let joined = listConcat(input.left,input.right);
  let items = fold(joined,step,${count}u64);
  let selected = if input.index < listLen(items) then listAt(items,input.index) else 0u8;
  let nested: List<List<U8,max=3>,max=1> = [items];
  let length = listLen(items);
  return { items, length, selected, nested };
}
`;
}

export function aliasSource(importLine: string): string {
  return `package ${coordinate};
${importLine}
type A = { x: U8 }; type B = { x: U8 };
type Input = { xs: List<A,max=2>, ys: List<B,max=2>, flag: Bool };
type Output = { items: List<A,max=4> };
intent assembleRange(input: Input) returns Output
  profile text.replaceRange basis none budget <= text.replaceRangeBudget
{
  let selected = if input.flag then input.xs else input.ys;
  let items = listConcat(selected,selected);
  return { items };
}
`;
}

export function readSource(importLine: string, count: number, nominal = false, bare = false): string {
  const identity = nominal ? `
  let ids: List<text.WitnessId,max=1> = [input.${bare ? 'raw' : 'identity'}];
  let selectedIdentity = if 0u64 < listLen(ids) then listAt(ids,0u64) else input.identity;
  require selectedIdentity == input.identity else text.BasisNotCanonical;` : '';
  return `package ${coordinate};
${importLine}
type Work = { items: List<text.FactBytes,max=3>, payload: text.FactBytes };
fn step(work: Work) -> Work {
  return if listLen(work.items) < 3u64
    then { items: listAppend(work.items,work.payload), payload: work.payload }
    else work;
}
type Input = { address: text.FactAddress, expected: text.FactBytes, fallback: text.FactBytes${nominal ? ', identity: text.WitnessId, raw: Bytes<exact=32>' : ''} };
intent replaceRange(input: Input) returns text.FactBytes
  profile text.replaceRange basis input.address.nodeId budget <= text.replaceRangeBudget
{
  let actual: text.FactBytes = text.readFact(input.address)
    else {
      missing(address) => text.FactMissing,
      typeMismatch(address) => text.FactMalformed,
      notAtom(address) => text.FactMalformed,
      tooLarge(address) => text.FactMalformed,
    };
  require actual == input.expected else text.BasisNotCanonical;
  ${identity}
  let empty: List<text.FactBytes,max=3> = [];
  let initial: Work = { items: empty, payload: actual };
  let built = fold(initial,step,${count}u64);
  let retained = if 2u64 < listLen(built.items) then listAt(built.items,2u64) else input.fallback;
  return retained;
}
`;
}

export const invalidHelpers = [
  { name: 'unguarded-index', kind: 'UnsupportedSourceShape', source: 'fn bad(xs: List<U8,max=2>, i: U64) -> U8 { return listAt(xs,i); }' },
  { name: 'weak-index-guard', kind: 'UnsupportedSourceShape', source: 'fn bad(xs: List<U8,max=2>, i: U64) -> U8 { return if i <= listLen(xs) then listAt(xs,i) else 0u8; }' },
  { name: 'unguarded-append', kind: 'UnsupportedSourceShape', source: 'fn bad(xs: List<U8,max=2>) -> List<U8,max=2> { return listAppend(xs,1u8); }' },
  { name: 'wrong-element', kind: 'TypeMismatch', source: 'fn bad(xs: List<U8,max=2>) -> List<U8,max=2> { return if listLen(xs) < 2u64 then listAppend(xs,1u64) else xs; }' },
  { name: 'concat-overflow', kind: 'InvalidBound', source: 'fn bad(xs: List<U8,max=18446744073709551615>, ys: List<U8,max=1>) -> List<U8,max=1> { return listConcat(xs,ys); }' },
];
