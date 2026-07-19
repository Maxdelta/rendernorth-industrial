/** Keeps asynchronous UI responses from mutating state after a newer request or unmount. */
export function createRequestGate() {
  let generation = 0;

  return {
    begin() {
      const requestGeneration = ++generation;
      return () => requestGeneration === generation;
    },
    invalidate() {
      generation += 1;
    },
  };
}
