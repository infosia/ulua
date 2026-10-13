// corpus: accept/a57-host-shadow
// purpose: A script binding of a host function name shadows the grant in its scope.
// rule: C18
// phase: P3
// grants: hostAdd, hostLog
// tsc: accepts
// js-comparable: yes
function twice(hostAdd: number): number {
  return hostAdd * 2;
}
function local(): number {
  const hostLog = (n: number): number => n + 1;
  return hostLog(41);
}
export function main(): void {
  print(`${twice(hostAdd(1, 2))}`);
  print(`${local()}`);
  hostLog("granted");
}
