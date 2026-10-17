// corpus: accept/a70-math-libm
// purpose: Math members computed by the native libm helper, at edge and ordinary inputs.
// rule: C4
// phase: P7
// tsc: accepts
// js-comparable: no C4
export function main(): void {
  print(`acos ${Math.acos(0.5)} ${Math.acos(-1)} ${Math.acos(2)}`);
  print(`acosh ${Math.acosh(1)} ${Math.acosh(2.5)} ${Math.acosh(10000000000)}`);
  print(`asin ${Math.asin(0.5)} ${Math.asin(-0.3)}`);
  print(`asinh ${Math.asinh(1)} ${Math.asinh(-1e-8)} ${Math.asinh(3)}`);
  print(`atan ${Math.atan(0.5)} ${Math.atan(1e+300)} ${Math.atan(-2)}`);
  print(`atanh ${Math.atanh(0.5)} ${Math.atanh(-0.16419020089181302)}`);
  print(`cbrt ${Math.cbrt(2.5)} ${Math.cbrt(-27)} ${Math.cbrt(1e-30)}`);
  print(`cos ${Math.cos(0.1)} ${Math.cos(1)} ${Math.cos(1000000)}`);
  print(`cosh ${Math.cosh(1)} ${Math.cosh(710)}`);
  print(`exp ${Math.exp(1)} ${Math.exp(-745)} ${Math.exp(0.5)}`);
  print(`expm1 ${Math.expm1(1.5)} ${Math.expm1(1e-10)}`);
  print(`log ${Math.log(3)} ${Math.log(10)} ${Math.log(1e-300)}`);
  print(`log1p ${Math.log1p(0.5)} ${Math.log1p(-0.5)}`);
  print(`log10 ${Math.log10(3.141592653589793)} ${Math.log10(1000)}`);
  print(`log2 ${Math.log2(8)} ${Math.log2(3)} ${Math.log2(1e-8)}`);
  print(`sin ${Math.sin(0.5)} ${Math.sin(2.2362173118876765)} ${Math.sin(-3)}`);
  print(`sinh ${Math.sinh(3.141592653589793)} ${Math.sinh(1e-8)}`);
  print(`tan ${Math.tan(1)} ${Math.tan(1.5)}`);
  print(`tanh ${Math.tanh(-0.44094602336741673)} ${Math.tanh(0.5)}`);
  print(`atan2 ${Math.atan2(1, 710)} ${Math.atan2(-0, -1)} ${Math.atan2(1, 0.1)}`);
  print(`hypot ${Math.hypot(1, 1.5)} ${Math.hypot(3, 4)} ${Math.hypot(1e+300, 1e+300)}`);
  print(`pow ${Math.pow(2, 0.5)} ${Math.pow(10, -3)} ${Math.pow(1.1, 100)}`);
}
