// Tiny safe calculator: + - * / % ^ and parentheses. No eval.
export function calculate(input: string): number | null {
  const src = input.replace(/\s+/g, "").replace(/×/g, "*").replace(/÷/g, "/");
  if (!/^[\d.+\-*/%^()]+$/.test(src) || !/[+\-*/%^]/.test(src.slice(1))) return null;
  let i = 0;
  const peek = () => src[i];
  function num(): number {
    if (peek() === "(") { i++; const v = expr(); if (src[i++] !== ")") throw 0; return v; }
    if (peek() === "-") { i++; return -num(); }
    const m = /^\d*\.?\d+/.exec(src.slice(i));
    if (!m) throw 0;
    i += m[0].length;
    return parseFloat(m[0]);
  }
  function pow(): number { const b = num(); if (peek() === "^") { i++; return Math.pow(b, pow()); } return b; }
  function term(): number {
    let v = pow();
    while (peek() === "*" || peek() === "/" || peek() === "%") {
      const op = src[i++]; const r = pow();
      v = op === "*" ? v * r : op === "/" ? v / r : v % r;
    }
    return v;
  }
  function expr(): number {
    let v = term();
    while (peek() === "+" || peek() === "-") { const op = src[i++]; const r = term(); v = op === "+" ? v + r : v - r; }
    return v;
  }
  try {
    const v = expr();
    return i === src.length && Number.isFinite(v) ? v : null;
  } catch {
    return null;
  }
}
